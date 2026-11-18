//! `caruspex_cli j1939`: trucks and buses over SAE J1939. Listens to the
//! bus, then asks for what is not broadcast: the control units with their
//! identification, the VIN, the active (DM1) and previous (DM2) faults with
//! the lamps, and the common values. `--clear` then clears both fault
//! lists (DM11 and DM3).

use std::sync::Arc;
use std::time::Instant;

use caruspex_link::{Elm327, FrameLink, Transport};
use caruspex_net::{J1939Can, open_can_interface};
use caruspex_protocols::j1939::params::Reading;
use caruspex_protocols::j1939::{
    AckKind, ClearResult, J1939Session, UnitFaults, address_label, describe,
};
use caruspex_record::JsonWriter;

use crate::args::Options;
use crate::commands::fail;
use crate::out::{ljust, say, say_err};

/// How long the first listen lasts.
const LISTEN_MS: u64 = 1500;

/// The rates to try: the one from --rate or the @ of --can, else both.
fn rates(opt: &Options) -> Vec<u32> {
    match opt.rate {
        250 | 500 => vec![opt.rate as u32],
        _ => vec![250, 500],
    }
}

/// Opens the way to the bus: a raw CAN interface, or an ELM327 (cable,
/// Wi-Fi, the demo truck) switched to protocol A. Returns the link and
/// the lines for the header.
/// The lines of the header: label and value.
type Header = Vec<(String, String)>;

fn open(opt: &Options) -> Result<(Box<dyn FrameLink>, Header), String> {
    if !opt.can_spec.is_empty() {
        let (base, given) = match opt.can_spec.rsplit_once('@') {
            Some((b, r)) => (b.to_string(), r.trim().parse::<u32>().ok()),
            None => (opt.can_spec.clone(), None),
        };
        let list = match given {
            Some(bits) => vec![bits / 1000],
            None => rates(opt),
        };
        let mut last = String::new();
        for kbit in list {
            let spec = format!("{base}@{}", kbit * 1000);
            let front = open_can_interface(&spec, &crate::open_serial)?;
            let name = Transport::name(&front);
            let mut can = J1939Can::new(front.shared_link());
            drop(front);
            let mut probe = J1939Session::new();
            if probe.listen(&mut can, LISTEN_MS)? >= 2 {
                let header = vec![
                    ("Interface ".to_string(), name),
                    (
                        "Protocol  ".to_string(),
                        format!("SAE J1939 (29 bit, {kbit} kbit/s)"),
                    ),
                ];
                return Ok((Box::new(can), header));
            }
            last = format!("Nothing heard on {spec}.");
        }
        return Err(last);
    }
    let transport: Box<dyn Transport> = if opt.demo {
        let origin = Instant::now();
        Box::new(caruspex_demo::DemoTransport::new(
            caruspex_demo::DemoOptions {
                clock: Some(Arc::new(move || 600.0 + origin.elapsed().as_secs_f64())),
                vehicle: caruspex_demo::DemoVehicle::Truck,
                ..Default::default()
            },
        ))
    } else {
        let baud = if opt.tcp.is_empty() && !opt.baud_given {
            crate::find_baud(&opt.port).unwrap_or(opt.baud)
        } else {
            opt.baud
        };
        crate::make_transport(&opt.tcp, &opt.port, baud)?
    };
    let mut elm = Elm327::new();
    if opt.verbose {
        let as_json = opt.json;
        elm.set_log(Box::new(move |m: &str| {
            if as_json {
                say_err(&format!("{m}\n"));
            } else {
                say(&format!("{m}\n"));
            }
        }));
    }
    elm.connect_j1939(transport, &rates(opt))?;
    let header = vec![
        ("Adapter   ".to_string(), elm.version().to_string()),
        ("Connection".to_string(), elm.transport_name()),
        ("Protocol  ".to_string(), elm.protocol_text().to_string()),
    ];
    Ok((Box::new(elm), header))
}

fn lamps_text(unit: &UnitFaults) -> String {
    let l = &unit.lamps;
    let mut on = Vec::new();
    for (lamp, name) in [
        (l.mil, "MIL"),
        (l.red_stop, "red stop"),
        (l.amber_warning, "amber warning"),
        (l.protect, "protect"),
    ] {
        if lamp.on() {
            on.push(name);
        }
    }
    if on.is_empty() {
        "off".to_string()
    } else {
        on.join(", ")
    }
}

fn print_faults(title: &str, units: &[UnitFaults], with_lamps: bool) {
    let count: usize = units.iter().map(|u| u.dtcs.len()).sum();
    say(&format!("{title}: {count}\n"));
    for unit in units
        .iter()
        .filter(|u| !u.dtcs.is_empty() || u.lamps.any_on())
    {
        if with_lamps {
            say(&format!(
                "  {}   lamps: {}\n",
                address_label(unit.address),
                lamps_text(unit)
            ));
        } else {
            say(&format!("  {}\n", address_label(unit.address)));
        }
        for dtc in &unit.dtcs {
            let seen = dtc
                .occurrences
                .map(|n| format!(" (occurred {n} times)"))
                .unwrap_or_default();
            say(&format!(
                "    {} {}{seen}\n",
                ljust(&dtc.code(), 18),
                describe(dtc.spn, dtc.fmi)
            ));
        }
    }
}

fn json_faults(json: &mut JsonWriter, name: &str, units: &[UnitFaults]) {
    json.key(name).begin_array();
    for unit in units {
        json.begin_object()
            .field("address", &i32::from(unit.address))
            .field("unit", &address_label(unit.address))
            .field("mil", &unit.lamps.mil.on())
            .field("redStop", &unit.lamps.red_stop.on())
            .field("amberWarning", &unit.lamps.amber_warning.on())
            .field("protect", &unit.lamps.protect.on())
            .key("faults")
            .begin_array();
        for dtc in &unit.dtcs {
            json.begin_object()
                .field("spn", &(dtc.spn as i32))
                .field("fmi", &i32::from(dtc.fmi))
                .key("occurrences");
            match dtc.occurrences {
                Some(n) => json.value(&i32::from(n)),
                None => json.null(),
            };
            json.field("text", &describe(dtc.spn, dtc.fmi)).end_object();
        }
        json.end_array().end_object();
    }
    json.end_array();
}

fn clear_line(name: &str, result: &ClearResult) -> String {
    if result.acks.is_empty() {
        return format!("{name}: no control unit confirmed.\n");
    }
    let parts: Vec<String> = result
        .acks
        .iter()
        .map(|(a, k)| format!("{} {}", address_label(*a), k.text()))
        .collect();
    format!("{name}: {}\n", parts.join(", "))
}

pub fn command(opt: &Options) -> i32 {
    let as_json = opt.json;
    let (mut link, header) = match open(opt) {
        Ok(found) => found,
        Err(error) => return fail(as_json, &format!("Connection failed: {error}"), 2),
    };
    let link = link.as_mut();
    let mut session = J1939Session::new();
    let read = (|| -> Result<_, String> {
        session.listen(link, LISTEN_MS)?;
        let active = session.active_faults(link)?;
        let previous = session.previous_faults(link)?;
        let vin = session.read_vin(link)?;
        let components = session.component_ids(link)?;
        let software = session.software_ids(link)?;
        session.read_counters(link)?;
        Ok((active, previous, vin, components, software))
    })();
    let (active, previous, vin, components, software) = match read {
        Ok(found) => found,
        Err(error) => return fail(as_json, &error, 3),
    };
    let units = session.units();
    let values = session.values();
    let cleared = if opt.clear {
        match (session.clear_active(link), session.clear_previous(link)) {
            (Ok(a), Ok(p)) => Some((a, p)),
            (Err(e), _) | (_, Err(e)) => return fail(as_json, &e, 3),
        }
    } else {
        None
    };

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object();
        for (name, value) in &header {
            json.field(&name.trim().to_ascii_lowercase(), value);
        }
        json.key("vin");
        match &vin {
            Some(v) => json.value(v),
            None => json.null(),
        };
        json.key("units").begin_array();
        for u in &units {
            let c = components.iter().find(|(a, _)| *a == u.address);
            let sw = software.iter().find(|(a, _)| *a == u.address);
            json.begin_object()
                .field("address", &i32::from(u.address))
                .field("name", &u.label)
                .field("make", &c.map(|(_, c)| c.make.clone()).unwrap_or_default())
                .field(
                    "model",
                    &c.map(|(_, c)| c.model.clone()).unwrap_or_default(),
                )
                .field(
                    "serial",
                    &c.map(|(_, c)| c.serial.clone()).unwrap_or_default(),
                )
                .string_list("software", &sw.map(|(_, s)| s.clone()).unwrap_or_default())
                .end_object();
        }
        json.end_array();
        json_faults(&mut json, "active", &active);
        json_faults(&mut json, "previous", &previous);
        json.key("values").begin_array();
        for v in &values {
            json.begin_object()
                .field("spn", &(v.spn as i32))
                .field("name", &v.name)
                .field("unit", &v.unit)
                .field("source", &i32::from(v.source))
                .key("value");
            match v.reading {
                Reading::Value(x) => json.value(&x),
                _ => json.null(),
            };
            json.field(
                "state",
                match v.reading {
                    Reading::Value(_) => "value",
                    Reading::Error => "error",
                    Reading::NotAvailable => "not available",
                },
            )
            .end_object();
        }
        json.end_array();
        if let Some((a, p)) = &cleared {
            json.field("clearedActive", &a.confirmed())
                .field("clearedPrevious", &p.confirmed());
        }
        json.end_object();
        say(&json.text());
        return 0;
    }

    for (name, value) in &header {
        say(&format!("{name} : {value}\n"));
    }
    say(&format!(
        "VIN        : {}\n\n",
        vin.as_deref().unwrap_or("(not reported)")
    ));
    say(&format!("Control units: {}\n", units.len()));
    for u in &units {
        let mut line = format!("  {}", ljust(&u.label, 34));
        if let Some((_, c)) = components.iter().find(|(a, _)| *a == u.address) {
            line.push_str(&format!(" {} {} {}", c.make, c.model, c.serial));
        }
        if let Some((_, s)) = software.iter().find(|(a, _)| *a == u.address) {
            line.push_str(&format!("  software {}", s.join(", ")));
        }
        say(&format!("{}\n", line.trim_end()));
    }
    say("\n");
    print_faults("Active faults (DM1)", &active, true);
    print_faults("Previously active faults (DM2)", &previous, false);
    say("\nValues:\n");
    for v in &values {
        let reading = match v.reading {
            Reading::Value(x) => format!("{x:.2} {}", v.unit),
            Reading::Error => "error".to_string(),
            Reading::NotAvailable => continue,
        };
        say(&format!(
            "  {} {} {} {}\n",
            ljust(&format!("SPN {}", v.spn), 9),
            ljust(&v.name, 36),
            ljust(&reading, 18),
            address_label(v.source)
        ));
    }
    if let Some((a, p)) = &cleared {
        say("\n");
        say(&clear_line("Cleared active faults (DM11)", a));
        say(&clear_line("Cleared previous faults (DM3)", p));
        if a.acks.iter().any(|(_, k)| *k != AckKind::Positive) {
            return 4;
        }
    }
    0
}
