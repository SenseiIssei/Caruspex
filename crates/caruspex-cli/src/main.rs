//! Console tool for the core. The actual application is the Tauri app.
//!
//!   caruspex_cli ports
//!   caruspex_cli kklcheck --port COM3
//!   caruspex_cli read --tcp 127.0.0.1:35000
//!   caruspex_cli read --port COM5 --baud 38400
//!   caruspex_cli health --demo --json
//!   caruspex_cli ecus --tcp 127.0.0.1:35000   (per control unit, all live values)
//!   caruspex_cli uds --demo --ids 760,737      (UDS: identification and faults per ECU)
//!   caruspex_cli read --port COM5 --record car.jsonl   (record the session)
//!   caruspex_cli read --replay car.jsonl               (play it back without a car)
//!   caruspex_cli custom --port COM5 --pack hyundai-kona-niro-ev   (maker values)
//!
//! `caruspex_cli help` lists every command and option.

#![forbid(unsafe_code)]

mod args;
mod commands;
mod out;

use std::sync::Arc;
use std::time::Instant;

use caruspex_i18n::{Language, set_language};
use caruspex_link::elm327::{PROTOCOL_ORDER_CAN_FIRST, PROTOCOL_ORDER_KLINE_FIRST};
use caruspex_link::{AdaptiveTiming, Elm327, SerialTransport, TcpTransport, Transport};
use caruspex_net::{
    discover_doip, discover_enet, j2534_devices, open_can_interface, open_network_car,
    socket_can_interfaces,
};
use caruspex_protocols::Obd2;
use caruspex_record::{
    RecordingMeta, RecordingTransport, ReplayPacing, ReplayTransport, recording_summary,
};

use args::{Options, USAGE, stoi};
use out::{ljust, say, say_err};

fn print_ports() {
    let ports = caruspex_link::enumerate_ports();
    if ports.is_empty() {
        say("No COM port found.\n");
        return;
    }
    for p in &ports {
        say(&format!("  {} {}\n", ljust(&p.device, 8), p.description));
        if p.vid != 0 {
            say(&format!(
                "           USB VID_{:04X} PID_{:04X}\n",
                p.vid, p.pid
            ));
        }
        if !p.chip_guess.is_empty() {
            say(&format!("           Chip: {}\n", p.chip_guess));
        }
    }
}

fn make_transport(tcp: &str, port: &str, baud: i32) -> Result<Box<dyn Transport>, String> {
    if !tcp.is_empty() {
        let colon = tcp.find(':');
        let host = &tcp[..colon.unwrap_or(tcp.len())];
        let p = match colon {
            None => 35000,
            Some(c) => stoi(&tcp[c + 1..]).ok_or_else(|| format!("Not a number after {host}:"))?,
        };
        let p = u16::try_from(p).map_err(|_| format!("Invalid port: {p}"))?;
        let mut t = TcpTransport::new();
        t.open(host, p)?;
        return Ok(Box::new(t));
    }
    let mut t = SerialTransport::new();
    t.open(port, baud.max(0) as u32)?;
    Ok(Box::new(t))
}

fn open_serial(device: &str, baud: u32) -> Result<Box<dyn Transport>, String> {
    let mut t = SerialTransport::new();
    t.open(device, baud)?;
    Ok(Box::new(t))
}

fn command_interfaces() {
    let devices = j2534_devices();
    for d in &devices {
        say(&format!(
            "j2534:{}   {}{}\n",
            d.library,
            d.name,
            if d.usable {
                ""
            } else {
                "  (32-bit driver, not usable)"
            }
        ));
    }
    let sockets = socket_can_interfaces();
    for n in &sockets {
        say(&format!("socketcan:{n}\n"));
    }
    if devices.is_empty() && sockets.is_empty() {
        say("No J2534 driver and no SocketCAN interface found. SLCAN sticks: slcan:<port>.\n");
    }
}

fn command_enet() -> i32 {
    let mut error = String::new();
    let cars = match discover_enet(3000) {
        Ok(c) => c,
        Err(e) => {
            error = e;
            Vec::new()
        }
    };
    let entities = match discover_doip(3000) {
        Ok(c) => c,
        Err(e) => {
            error = e;
            Vec::new()
        }
    };
    if cars.is_empty() && entities.is_empty() {
        say(&format!(
            "No car answered on the network{}{}.\n",
            if error.is_empty() { "" } else { ": " },
            error
        ));
        return 2;
    }
    for car in &cars {
        say(&format!(
            "ENET  {} VIN {} MAC {}\n",
            ljust(&car.address, 16),
            ljust(if car.vin.is_empty() { "?" } else { &car.vin }, 17),
            car.mac
        ));
    }
    for e in &entities {
        say(&format!(
            "DoIP  {} VIN {} address {:04X}\n",
            ljust(&e.address, 16),
            ljust(if e.vin.is_empty() { "?" } else { &e.vin }, 17),
            e.logical_address.map_or(0xFFFF, u32::from)
        ));
    }
    0
}

fn run(argv: &[String]) -> i32 {
    // The console tool speaks English, including the texts that come from the core.
    set_language(Language::English);

    let command = argv.get(1).cloned().unwrap_or_else(|| "ports".to_string());
    let opt = match Options::parse(argv) {
        Ok(o) => o,
        Err(text) => {
            say(&text);
            return 1;
        }
    };
    let as_json = opt.json;

    match command.as_str() {
        "help" | "--help" | "-h" => {
            say(USAGE);
            return 0;
        }
        "version" | "--version" => {
            say(&format!("caruspex_cli {}\n", env!("CARGO_PKG_VERSION")));
            return 0;
        }
        "ports" => {
            print_ports();
            return 0;
        }
        "interfaces" => {
            command_interfaces();
            return 0;
        }
        "enet" => return command_enet(),
        "kklcheck" => return commands::kkl_check(&opt.port),
        _ => {}
    }
    const KNOWN: [&str; 11] = [
        "read", "clear", "live", "info", "vin", "freeze", "health", "probe", "ecus", "uds",
        "custom",
    ];
    if !KNOWN.contains(&command.as_str()) {
        say(&format!("Unknown command: {command}\n\n{USAGE}"));
        return 1;
    }

    let mut uds_list = Vec::new();
    if command == "uds" && !opt.uds_ids.is_empty() {
        // Without --ids the list is chosen after connecting, by the VIN.
        match commands::parse_uds_ids(&opt.uds_ids) {
            Ok(list) => uds_list = list,
            Err(error) => return commands::fail(as_json, &error, 1),
        }
    }
    let mut custom = None;
    if command == "custom"
        && opt.csv_path.is_empty()
        && let Some(query) = commands::obdb_query(&opt.pack)
    {
        // Only a list, no adapter needed.
        return commands::list_obdb_packs(query, as_json);
    }
    if command == "custom" {
        match commands::load_custom_pids(&opt.pack, &opt.csv_path, opt.demo) {
            Ok(c) => custom = Some(c),
            Err((error, warnings)) => {
                for w in &warnings {
                    say_err(&format!("{w}\n"));
                }
                return commands::fail(as_json, &error, 1);
            }
        }
    }

    let mut protocol_order = if opt.demo || !opt.enet.is_empty() {
        PROTOCOL_ORDER_CAN_FIRST.to_string()
    } else {
        PROTOCOL_ORDER_KLINE_FIRST.to_string()
    };
    if !opt.can_spec.is_empty() {
        protocol_order = "67".to_string();
    }
    let transport: Result<Box<dyn Transport>, String> = if !opt.replay_path.is_empty() {
        let replay = ReplayTransport::new(&opt.replay_path, ReplayPacing::Fast);
        if !replay.is_open() {
            return commands::fail(as_json, replay.error(), 2);
        }
        if !as_json {
            say(&format!("{}\n", recording_summary(&replay.info())));
        }
        // The recorded protocol first, so the replay does not have to step
        // through the ones the car never answered on.
        protocol_order = replay.protocol_order(&protocol_order);
        Ok(Box::new(replay))
    } else if opt.demo {
        // Ten minutes after ignition on: the engine is warm and the demo car
        // is out on its drive cycle, so the live values say something.
        let origin = Instant::now();
        let options = caruspex_demo::DemoOptions {
            clock: Some(Arc::new(move || 600.0 + origin.elapsed().as_secs_f64())),
            ..Default::default()
        };
        Ok(Box::new(caruspex_demo::DemoTransport::new(options)))
    } else if !opt.can_spec.is_empty() {
        open_can_interface(&opt.can_spec, &open_serial).map(|t| Box::new(t) as Box<dyn Transport>)
    } else if !opt.enet.is_empty() {
        open_network_car(&opt.enet).map(|car| Box::new(car.transport) as Box<dyn Transport>)
    } else {
        make_transport(&opt.tcp, &opt.port, opt.baud)
    };
    let mut transport = match transport {
        Ok(t) => t,
        Err(error) => return commands::fail(as_json, &format!("Connection failed: {error}"), 2),
    };

    let mut recording_path = None;
    if !opt.record_path.is_empty() {
        let wrapped =
            RecordingTransport::new(transport, &opt.record_path, RecordingMeta::default());
        if !wrapped.is_recording() {
            return commands::fail(as_json, wrapped.error(), 2);
        }
        recording_path = Some(wrapped.path().to_string());
        transport = wrapped;
    }

    let mut elm = Elm327::new();
    if opt.fast_timing {
        elm.set_adaptive_timing(AdaptiveTiming::Aggressive);
    }
    if opt.verbose {
        // With --json the log goes to stderr so stdout stays parseable.
        elm.set_log(Box::new(move |m: &str| {
            if as_json {
                say_err(&format!("{m}\n"));
            } else {
                say(&format!("{m}\n"));
            }
        }));
    }
    if let Err(error) = elm.connect(transport, &protocol_order) {
        return commands::fail(as_json, &error, 3);
    }
    if !as_json {
        say(&format!("Adapter    : {}\n", elm.version()));
        if elm.is_stn() {
            say(&format!("STN chip   : {}\n", elm.stn_id()));
        }
        say(&format!("Connection : {}\n", elm.transport_name()));
        say(&format!("Protocol   : {}\n", elm.protocol_text()));
        if elm.adaptive_timing_level() > 0 {
            say(&format!(
                "Timing     : ATAT{}\n",
                elm.adaptive_timing_level()
            ));
        }
        if let Some(path) = &recording_path {
            say(&format!("Recording  : {path}\n"));
        }
        say("\n");
    }

    let result = match command.as_str() {
        "read" => commands::read(&mut elm, as_json),
        "vin" => commands::vin(&mut elm, as_json),
        "freeze" => commands::freeze(&mut elm, as_json),
        "health" => commands::health(&mut elm, as_json, opt.seconds),
        "probe" => commands::probe(&mut elm, as_json),
        "clear" => {
            let mut obd = Obd2::new(&mut elm);
            match obd.clear_faults() {
                Ok(()) => say("Fault memory cleared.\n"),
                Err(err) => {
                    say("Clearing failed.\n");
                    if !err.is_empty() {
                        say(&format!("{err}\n"));
                    }
                }
            }
            0
        }
        "live" => {
            let mut obd = Obd2::new(&mut elm);
            for v in obd.read_live_values(caruspex_data::pids::default_live_pids()) {
                commands::print_value(&v);
            }
            0
        }
        "info" => commands::info(&mut elm),
        "uds" => commands::uds(&mut elm, uds_list, as_json),
        "custom" => {
            let (pids, title, warnings) = custom.unwrap_or_default();
            commands::custom(&mut elm, &pids, &title, &warnings, as_json)
        }
        "ecus" => commands::ecus(&mut elm),
        _ => 0,
    };

    elm.disconnect();
    result
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let code = run(&argv);
    out::flush();
    std::process::exit(code);
}
