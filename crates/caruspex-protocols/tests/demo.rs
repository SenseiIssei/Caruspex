//! The built-in demo vehicle, end to end through the real Elm327 driver and
//! the OBD services (the Elm327/Obd2 cases of tests/test_demo.cpp; the
//! wire-level cases live in caruspex-demo).

mod common;

use caruspex_demo::{DemoAdapter, DemoClock, DemoOptions, DemoTransport};
use caruspex_i18n::l;
use caruspex_link::elm327::{
    DEFAULT_REQUEST_TIMEOUT_MS, DEFAULT_SEND_TIMEOUT_MS, PROTOCOL_ORDER_CAN_FIRST,
};
use caruspex_link::{Elm327, hex_to_bytes};
use caruspex_protocols::obd2::{Fault, Obd2, VehicleStatus};
use common::*;

fn live(obd: &mut Obd2<'_>, pid: i32) -> f64 {
    obd.read_live(pid).expect("value").value
}

/// Data bytes of the first answer to a mode 01 request, without the PID echo.
fn pid_bytes(elm: &mut Elm327, command: &str) -> Vec<u8> {
    let result = elm.request(command, 0x01, DEFAULT_REQUEST_TIMEOUT_MS);
    assert!(result.ok(), "{command}");
    result.messages[0][1..].to_vec()
}

/// The North American check digit, computed independently of the demo.
fn vin_check_digit(vin: &str) -> char {
    let value = |c: char| -> u32 {
        if let Some(d) = c.to_digit(10) {
            return d;
        }
        let letters = "ABCDEFGHJKLMNPRSTUVWXYZ";
        let values = [
            1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 7, 9, 2, 3, 4, 5, 6, 7, 8, 9,
        ];
        values[letters.find(c).expect("valid letter")]
    };
    let weights = [8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2];
    let sum: u32 = vin.chars().zip(weights).map(|(c, w)| value(c) * w).sum();
    match sum % 11 {
        10 => 'X',
        rest => char::from_digit(rest, 10).unwrap(),
    }
}

fn has_code(faults: &[Fault], code: &str, kind: &str) -> bool {
    faults.iter().any(|f| f.code == code && f.kind == kind)
}

fn not_ready(status: &VehicleStatus) -> usize {
    status.readiness.iter().filter(|r| !r.ready).count()
}

#[test]
fn elm327_and_obd2_connect_to_the_demo_car() {
    let _lang = german();
    let mut rig = DemoRig::new(300.0);
    assert!(rig.connected, "{}", rig.error);
    assert_eq!(rig.elm.version(), "ELM327 v1.5");
    assert_eq!(rig.elm.protocol_text(), "ISO 15765-4 (CAN 11/500)");
    let mut obd = Obd2::new(&mut rig.elm);

    let vin = obd.read_vin();
    assert_eq!(vin, DemoTransport::vin());
    assert_eq!(vin.len(), 17);
    assert!(!vin.contains(['I', 'O', 'Q']));
    assert_eq!(vin.chars().nth(8).unwrap(), vin_check_digit(&vin));

    let pids = obd.supported_pids();
    for pid in [
        0x01, 0x05, 0x0C, 0x0D, 0x10, 0x14, 0x15, 0x2F, 0x42, 0x5C, 0x5E,
    ] {
        assert!(pids.contains(&pid), "PID {pid:02X}");
    }
    assert!(!pids.contains(&0x0A));
    // Every supported PID really answers, and nothing else does.
    for pid in 0x01..=0x7F {
        if pid % 0x20 == 0 {
            continue;
        }
        let result = obd
            .elm()
            .request(&format!("01{pid:02X}"), 0x01, DEFAULT_REQUEST_TIMEOUT_MS);
        assert_eq!(result.ok(), pids.contains(&pid), "PID {pid:02X}");
    }

    let identity = obd.read_identity();
    assert_eq!(identity.vin, DemoTransport::vin());
    assert!(identity.calibration_id.starts_with("CX5EM21A0417"));
    assert!(identity.ecu_name.starts_with("ECM-EngineControl"));
    assert!(!identity.calibration_verification.is_empty());
}

#[test]
fn without_the_gearbox_the_identification_is_unambiguous() {
    let _lang = german();
    let mut rig = DemoRig::with(300.0, DemoAdapter::CloneV15, false, 0.0);
    assert!(rig.connected);
    let identity = Obd2::new(&mut rig.elm).read_identity();
    assert_eq!(identity.calibration_id, "CX5EM21A0417");
    assert_eq!(identity.ecu_name, "ECM-EngineControl");
}

#[test]
fn automatic_protocol_search_finds_the_demo_car() {
    let _lang = german();
    let clock = DemoClock::new(100.0);
    let options = DemoOptions::with_clock(&clock);
    let mut elm = Elm327::new();
    elm.connect(Box::new(DemoTransport::new(options.clone())), "0")
        .expect("connects");
    assert_eq!(elm.protocol_text(), "AUTO, ISO 15765-4 (CAN 11/500)");
    elm.connect(
        Box::new(DemoTransport::new(options)),
        PROTOCOL_ORDER_CAN_FIRST,
    )
    .expect("connects");
}

#[test]
fn demo_faults_read_clear_and_come_back() {
    let _lang = german();
    let mut rig = DemoRig::new(300.0);
    assert!(rig.connected);
    let clock = rig.clock.clone();
    let mut obd = Obd2::new(&mut rig.elm);
    let stored = l("gespeichert", "stored");
    let intermittent = l("sporadisch", "intermittent");
    let permanent = l("dauerhaft", "permanent");

    let faults = obd.read_faults().expect("faults");
    assert!(has_code(&faults, "P0171", stored));
    assert!(has_code(&faults, "P0420", stored));
    assert!(has_code(&faults, "P0300", intermittent));
    assert_eq!(faults.len(), 3);

    let status = obd.read_status();
    assert!(status.valid);
    assert!(status.mil_on);
    assert_eq!(status.fault_count, 2);
    assert_eq!(not_ready(&status), 2); // EVAP and EGR/VVT
    assert!(live(&mut obd, 0x31) > 1800.0);

    let (rows, trigger) = obd.read_freeze_frame();
    assert!(!rows.is_empty());
    assert_eq!(trigger, "P0171");

    obd.clear_faults().expect("cleared");

    // Stored and pending are gone; permanent codes stay until the monitors
    // pass again, exactly what mode 0A is for.
    let faults = obd.read_faults().expect("faults");
    assert_eq!(faults.len(), 2);
    assert!(has_code(&faults, "P0171", permanent));
    assert!(has_code(&faults, "P0420", permanent));
    let status = obd.read_status();
    assert!(!status.mil_on);
    assert_eq!(status.fault_count, 0);
    assert_eq!(not_ready(&status), 5);
    assert_eq!(live(&mut obd, 0x31), 0.0);
    assert_eq!(live(&mut obd, 0x4E), 0.0);
    assert_eq!(live(&mut obd, 0x30), 0.0);
    let (rows, trigger) = obd.read_freeze_frame();
    assert!(rows.is_empty());
    assert!(trigger.is_empty());

    // After a while of driving the causes are still there.
    clock.advance(700.0);
    let faults = obd.read_faults().expect("faults");
    assert!(has_code(&faults, "P0171", stored));
    assert!(has_code(&faults, "P0300", intermittent));
    assert!(has_code(&faults, "P0420", intermittent));
    let status = obd.read_status();
    assert!(status.mil_on);
    assert_eq!(status.fault_count, 1);
    assert_eq!(not_ready(&status), 1); // EVAP never completes in the demo
    assert!(live(&mut obd, 0x31) > 5.0);
    assert!(live(&mut obd, 0x4E) >= 11.0);
    let (rows, trigger) = obd.read_freeze_frame();
    assert!(!rows.is_empty());
    assert_eq!(trigger, "P0300");
}

#[test]
fn demo_live_values_follow_the_injected_clock() {
    let _lang = german();
    let mut rig = DemoRig::new(1.0); // ignition on, engine not yet started
    assert!(rig.connected);
    let clock = rig.clock.clone();
    let mut obd = Obd2::new(&mut rig.elm);

    assert_eq!(live(&mut obd, 0x0C), 0.0);
    assert_eq!(live(&mut obd, 0x0D), 0.0);
    assert!(near(live(&mut obd, 0x42), 12.37, 0.1));
    assert_eq!(
        obd.elm().send("ATRV", DEFAULT_SEND_TIMEOUT_MS).lines[0],
        "12.6V"
    );
    assert!(near(live(&mut obd, 0x05), 12.0, 1.0));

    clock.set(20.0); // idling, cold
    let cold_coolant = live(&mut obd, 0x05);
    assert!(live(&mut obd, 0x0C) > 900.0);
    assert_eq!(live(&mut obd, 0x0D), 0.0);
    assert!(live(&mut obd, 0x42) > 13.0);
    assert_eq!(pid_bytes(obd.elm(), "0103")[0], 0x01); // open loop, cold

    // Driving starts 50 s after ignition on, the cycle lasts 120 s.
    // 70 s into a cycle the car cruises at 100 km/h in sixth gear.
    clock.set(50.0 + 3.0 * 120.0 + 70.0);
    assert!(live(&mut obd, 0x05) > cold_coolant + 30.0);
    let speed = live(&mut obd, 0x0D);
    let rpm = live(&mut obd, 0x0C);
    assert!(near(speed, 100.0, 2.0));
    let gear = pid_bytes(obd.elm(), "01A4");
    assert_eq!(gear.len(), 4);
    assert_eq!(gear[1] >> 4, 6);
    let ratio = f64::from((u16::from(gear[2]) << 8) | u16::from(gear[3])) / 1000.0;
    assert!(approx(ratio, 0.82));
    // rpm from speed, gear ratio, final drive 3.2 and 1.94 m tyre circumference.
    assert!(near(rpm, speed / 3.6 / 1.94 * 60.0 * ratio * 3.2, 40.0));
    assert_eq!(pid_bytes(obd.elm(), "0103")[0], 0x02); // closed loop
    assert!(near(live(&mut obd, 0x07), 17.2, 0.8));
    assert!(live(&mut obd, 0x10) > 5.0);
    assert!(live(&mut obd, 0x5E) > 2.0);

    // Several PIDs in one request, as CAN allows.
    let multi = obd
        .elm()
        .request("010C0D05", 0x01, DEFAULT_REQUEST_TIMEOUT_MS);
    assert!(multi.ok());
    // 0C a b 0D a 05 a: eight bytes with the service byte, so two frames.
    // Without headers the padding of the last frame is dropped.
    assert_eq!(multi.messages[0].len(), 7);
    assert_eq!(multi.messages[0][0], 0x0C);
    assert_eq!(multi.messages[0][3], 0x0D);
    assert_eq!(multi.messages[0][5], 0x05);

    // The O2 sensor switches.
    let (mut low, mut high) = (1.0f64, 0.0f64);
    for _ in 0..20 {
        clock.advance(0.05);
        let v = live(&mut obd, 0x14);
        low = low.min(v);
        high = high.max(v);
    }
    assert!(low < 0.2);
    assert!(high > 0.7);

    // Braking: fuel cut.
    clock.set(50.0 + 3.0 * 120.0 + 90.0);
    assert_eq!(pid_bytes(obd.elm(), "0103")[0], 0x04);
    assert_eq!(live(&mut obd, 0x5E), 0.0);

    // Odometer and fuel level over a longer drive.
    // PID A6 is read raw: the odometer is newer than the PID table.
    let odometer = |obd: &mut Obd2<'_>| {
        let d = pid_bytes(obd.elm(), "01A6");
        assert_eq!(d.len(), 4);
        ((f64::from(d[0]) * 256.0 + f64::from(d[1])) * 256.0 + f64::from(d[2])) * 256.0 / 10.0
            + f64::from(d[3]) / 10.0
    };
    let odometer_before = odometer(&mut obd);
    let fuel_before = live(&mut obd, 0x2F);
    let run_before = live(&mut obd, 0x1F);
    assert!(odometer_before > 48213.0);
    clock.advance(1200.0);
    let odometer_after = odometer(&mut obd);
    assert!(near(odometer_after - odometer_before, 18.3, 0.5)); // 54.8 km/h
    assert!(live(&mut obd, 0x2F) < fuel_before);
    assert!(near(live(&mut obd, 0x1F), run_before + 1200.0, 1.0));
}

#[test]
fn demo_answers_are_deterministic_with_an_injected_clock() {
    let _lang = german();
    let mut a = DemoRig::new(777.7);
    let mut b = DemoRig::new(777.7);
    assert!(a.connected);
    assert!(b.connected);
    for command in ["010C", "010D", "0114", "0110", "015E", "01A6", "06A4"] {
        assert_eq!(
            a.elm.send(command, DEFAULT_SEND_TIMEOUT_MS).lines,
            b.elm.send(command, DEFAULT_SEND_TIMEOUT_MS).lines,
            "{command}"
        );
    }
}

#[test]
fn demo_multi_frame_answers_with_and_without_headers() {
    let _lang = german();
    let mut rig = DemoRig::new(300.0);
    assert!(rig.connected);
    let elm = &mut rig.elm;

    // Headers off: total length, then numbered lines.
    let plain = elm.send("0902", DEFAULT_SEND_TIMEOUT_MS);
    assert_eq!(plain.lines.len(), 4);
    assert_eq!(plain.lines[0], "014");
    assert!(plain.lines[1].starts_with("0:490201"));
    assert!(plain.lines[2].starts_with("1:"));

    // Headers on: identifier, PCI bytes and the reassembly is up to us.
    assert_eq!(elm.send("ATH1", DEFAULT_SEND_TIMEOUT_MS).lines[0], "OK");
    let headed = elm.send("0902", DEFAULT_SEND_TIMEOUT_MS);
    assert_eq!(headed.lines.len(), 3);
    let mut payload: Vec<u8> = Vec::new();
    let mut length = 0usize;
    for (i, line) in headed.lines.iter().enumerate() {
        assert!(line.starts_with("7E8"));
        let frame = hex_to_bytes(&line[3..]);
        assert_eq!(frame.len(), 8);
        if i == 0 {
            assert_eq!(frame[0], 0x10); // first frame
            length = usize::from(frame[1]);
            payload.extend_from_slice(&frame[2..]);
        } else {
            assert_eq!(usize::from(frame[0]), 0x20 + i); // consecutive frames
            payload.extend_from_slice(&frame[1..]);
        }
    }
    assert_eq!(length, 20);
    payload.truncate(length);
    assert_eq!(payload[0], 0x49);
    assert_eq!(payload[1], 0x02);
    assert_eq!(
        String::from_utf8(payload[3..].to_vec()).unwrap(),
        DemoTransport::vin()
    );

    // Both control units answer the calibration request, three frames each.
    let both = elm.send("0904", DEFAULT_SEND_TIMEOUT_MS);
    assert_eq!(both.lines.len(), 6);
    assert_eq!(
        both.lines.iter().filter(|l| l.starts_with("7E9")).count(),
        3
    );
    assert_eq!(elm.send("ATH0", DEFAULT_SEND_TIMEOUT_MS).lines[0], "OK");
}

#[test]
fn demo_mode_06_reports_misfire_counters_per_cylinder() {
    let _lang = german();
    let mut rig = DemoRig::new(600.0);
    assert!(rig.connected);
    let elm = &mut rig.elm;

    let mask = elm.request("0600", 0x06, DEFAULT_REQUEST_TIMEOUT_MS);
    assert!(mask.ok());
    assert_eq!(mask.messages[0], [0x00, 0xC0, 0x00, 0x00, 0x01]);
    let misfire_mask = elm.request("06A0", 0x06, DEFAULT_REQUEST_TIMEOUT_MS);
    assert!(misfire_mask.ok());
    assert_eq!(misfire_mask.messages[0], [0xA0, 0xF8, 0x00, 0x00, 0x00]);

    let mut counters = |command: &str| {
        let r = elm.request(command, 0x06, DEFAULT_REQUEST_TIMEOUT_MS);
        assert!(r.ok());
        let m = &r.messages[0];
        assert!(m.len() >= 18);
        assert_eq!(m[1], 0x0B); // EWMA over the last ten driving cycles
        assert_eq!(m[2], 0x24); // counts
        assert_eq!(m[10], 0x0C); // this driving cycle
        let word = |at: usize| (u32::from(m[at]) << 8) | u32::from(m[at + 1]);
        (word(3), word(12))
    };
    let cylinder1 = counters("06A2");
    let cylinder3 = counters("06A4");
    assert!(cylinder3.0 > cylinder1.0);
    assert!(cylinder3.1 > cylinder1.1);

    // The catalyst test fails, which is what P0420 is about.
    let catalyst = elm.request("0621", 0x06, DEFAULT_REQUEST_TIMEOUT_MS);
    assert!(catalyst.ok());
    let c = &catalyst.messages[0];
    let value = (u32::from(c[3]) << 8) | u32::from(c[4]);
    let maximum = (u32::from(c[7]) << 8) | u32::from(c[8]);
    assert!(value > maximum);

    // Through Obd2 the same counters come out as misfire counts.
    let results = Obd2::new(elm).read_monitor_results().expect("results");
    let misfires = caruspex_protocols::misfire_counts(&results);
    assert!(misfires.iter().any(|m| m.cylinder == 3 && m.has_current));
}
