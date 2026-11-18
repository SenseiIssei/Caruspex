//! SAE J1939 end to end: the demo truck behind the demo ELM327 in protocol
//! A, through the real Elm327 driver and the J1939 session.

use std::sync::{Arc, Mutex};

use caruspex_demo::{DemoAdapter, DemoClock, DemoOptions, DemoTransport, DemoVehicle};
use caruspex_link::{Elm327, FrameLink, LogFn};
use caruspex_protocols::j1939::params::{J1939_KEY_BASE, Reading};
use caruspex_protocols::j1939::{AckKind, J1939Session, SpnConversion};

struct Truck {
    clock: DemoClock,
    elm: Elm327,
    session: J1939Session,
    log: Arc<Mutex<Vec<String>>>,
}

fn truck_with(adapter: DemoAdapter, rates: &[u32]) -> Result<Truck, String> {
    let clock = DemoClock::new(600.0);
    let options = DemoOptions {
        adapter,
        clock: Some(clock.source()),
        latency_scale: 0.0,
        vehicle: DemoVehicle::Truck,
        ..DemoOptions::default()
    };
    let log = Arc::new(Mutex::new(Vec::new()));
    let sink = log.clone();
    let make_log = move || -> LogFn {
        let sink = sink.clone();
        Box::new(move |line: &str| sink.lock().unwrap().push(line.to_string()))
    };
    let mut elm = Elm327::new();
    elm.set_log(make_log());
    let kbit = elm.connect_j1939(Box::new(DemoTransport::new(options)), rates)?;
    assert_eq!(kbit, 250);
    let mut session = J1939Session::new();
    session.set_waits(300, 700);
    session.set_log(make_log());
    Ok(Truck {
        clock,
        elm,
        session,
        log,
    })
}

fn truck() -> Truck {
    truck_with(DemoAdapter::CloneV15, &[250]).expect("connects")
}

#[test]
fn connects_by_listening_and_finds_the_rate() {
    let t = truck();
    assert_eq!(t.elm.j1939_kbit(), 250);
    assert!(t.elm.protocol_text().contains("J1939"));
    // 500 first: nothing to hear there, then 250.
    let t = truck_with(DemoAdapter::GenuineV14b, &[500, 250]).expect("connects");
    assert_eq!(t.elm.j1939_kbit(), 250);
    assert_eq!(t.elm.connect_trace().protocols_tried, "BA");
    // Only 500: the truck is not heard.
    let error = truck_with(DemoAdapter::CloneV15, &[500])
        .err()
        .expect("fails");
    assert!(error.contains("J1939"), "{error}");
}

#[test]
fn an_obdlink_takes_500_kbit_with_its_own_command() {
    // STPBR on an STN chip; the demo truck stays at 250, so nothing is heard
    // at 500, but the command must be accepted on the way.
    let error = truck_with(DemoAdapter::ObdLinkEx, &[500])
        .err()
        .expect("fails");
    assert!(error.contains("9"), "{error}");
}

#[test]
fn units_values_and_active_faults_by_listening() {
    let mut t = truck();
    let frames = t.session.listen(&mut t.elm, 400).unwrap();
    assert!(frames > 20, "{frames}");
    let units: Vec<u8> = t.session.units().iter().map(|u| u.address).collect();
    assert_eq!(units, vec![0x00, 0x03, 0x0B, 0x17]);

    let values = t.session.values();
    let get = |spn: u32| {
        values
            .iter()
            .find(|v| v.spn == spn)
            .unwrap_or_else(|| panic!("SPN {spn}"))
            .reading
    };
    assert!(matches!(get(190), Reading::Value(rpm) if (500.0..2000.0).contains(&rpm)));
    assert_eq!(get(110), Reading::Value(85.0));
    assert!(matches!(get(96), Reading::Value(v) if (v - 62.0).abs() < 0.5));
    assert!(matches!(get(168), Reading::Value(v) if (v - 27.8).abs() < 0.01));
    assert!(matches!(get(171), Reading::Value(v) if (v - 12.5).abs() < 0.05));
    // The sensor of the active fault says "error".
    assert_eq!(get(105), Reading::Error);

    let active = t.session.active_faults(&mut t.elm).unwrap();
    let engine = active
        .iter()
        .find(|u| u.address == 0x00)
        .expect("engine DM1");
    assert!(engine.lamps.mil.on() && engine.lamps.amber_warning.on());
    assert!(!engine.lamps.red_stop.on());
    let codes: Vec<(u32, u8, Option<u8>)> = engine
        .dtcs
        .iter()
        .map(|d| (d.spn, d.fmi, d.occurrences))
        .collect();
    assert_eq!(codes, vec![(105, 3, Some(7)), (111, 17, Some(2))]);
    assert!(
        engine
            .dtcs
            .iter()
            .all(|d| d.conversion == SpnConversion::V4)
    );
    // The other units send a DM1 without faults.
    assert_eq!(active.len(), 4);
    assert!(
        active
            .iter()
            .filter(|u| u.address != 0)
            .all(|u| u.dtcs.is_empty())
    );
}

#[test]
fn requests_previous_faults_identification_and_counters() {
    let mut t = truck();
    t.session.listen(&mut t.elm, 300).unwrap();
    let previous = t.session.previous_faults(&mut t.elm).unwrap();
    let find = |a: u8| previous.iter().find(|u| u.address == a).expect("DM2");
    assert_eq!(find(0x00).dtcs[0].spn, 1761);
    assert_eq!(find(0x00).dtcs[0].fmi, 18);
    assert_eq!(find(0x0B).dtcs[0].spn, 904);
    assert!(find(0x03).dtcs.is_empty());
    // The tester claimed F9 before asking.
    assert_eq!(t.session.address(), (0xF9, true));

    // VIN straight from the engine: RTS, CTS, three packets, end of message.
    let vin = t.session.read_vin(&mut t.elm).unwrap();
    assert_eq!(vin.as_deref(), Some(DemoTransport::truck_vin()));

    let components = t.session.component_ids(&mut t.elm).unwrap();
    assert_eq!(components.len(), 4);
    let engine = &components.iter().find(|(a, _)| *a == 0).unwrap().1;
    assert_eq!(engine.make, "CARUSPEX");
    assert_eq!(engine.model, "CX-D13");
    let software = t.session.software_ids(&mut t.elm).unwrap();
    let engine_sw = &software.iter().find(|(a, _)| *a == 0).unwrap().1;
    assert_eq!(
        engine_sw,
        &vec!["ECM 4.2.1".to_string(), "BOOT 1.07".to_string()]
    );

    t.session.read_counters(&mut t.elm).unwrap();
    let hours = t
        .session
        .values()
        .into_iter()
        .find(|v| v.spn == 247)
        .expect("engine hours");
    assert!(matches!(hours.reading, Reading::Value(h) if (h - 12_345.77).abs() < 0.1));
    assert_eq!(
        caruspex_protocols::j1939::find_param(247).unwrap().key(),
        J1939_KEY_BASE + 247
    );
}

#[test]
fn clearing_active_then_previous_faults() {
    let mut t = truck();
    let cleared = t.session.clear_active(&mut t.elm).unwrap();
    assert_eq!(cleared.acks.len(), 4);
    assert!(cleared.acks.iter().all(|(_, k)| *k == AckKind::Positive));
    assert!(cleared.confirmed());
    t.clock.advance(2.0);
    let active = t.session.active_faults(&mut t.elm).unwrap();
    assert!(
        active
            .iter()
            .all(|u| u.dtcs.is_empty() && !u.lamps.any_on())
    );
    let previous = t.session.previous_faults(&mut t.elm).unwrap();
    let engine: Vec<u32> = previous
        .iter()
        .find(|u| u.address == 0)
        .unwrap()
        .dtcs
        .iter()
        .map(|d| d.spn)
        .collect();
    assert_eq!(engine, vec![1761, 105, 111]);
    assert!(t.session.clear_previous(&mut t.elm).unwrap().confirmed());
    let previous = t.session.previous_faults(&mut t.elm).unwrap();
    assert!(previous.iter().all(|u| u.dtcs.is_empty()));
    assert!(t.log.lock().unwrap().iter().any(|l| l.contains("DM11")));
}

#[test]
fn the_link_names_itself() {
    let t = truck();
    assert!(FrameLink::name(&t.elm).contains("J1939"));
}
