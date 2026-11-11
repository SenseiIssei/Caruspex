//! What an STN chip (OBDLink) adds, end to end through the real Elm327
//! driver, the OBD services and UDS against the demo car: live values with
//! the batched commands, and the body control units on the medium speed and
//! the single wire CAN. The demo counts its simulated adapter time
//! (`DemoTransport::adapter_ms`), so the gain is measured exactly and the
//! same on every machine.

use caruspex_data::ecu_addresses::ecu_from_ids;
use caruspex_demo::{DemoAdapter, DemoAdapterClock, DemoClock, DemoOptions, DemoTransport};
use caruspex_link::elm327::PROTOCOL_ORDER_CAN_FIRST;
use caruspex_link::{Elm327, StnBus};
use caruspex_protocols::obd2::{LiveValue, Obd2};
use caruspex_protocols::uds::{Uds, scan_ecus};

/// The twelve values of the live page plus twelve more engine PIDs: four
/// groups of six on CAN, as the app asks them on an STN chip per tick.
const PIDS: [i32; 24] = [
    0x0C, 0x0D, 0x05, 0x0F, 0x04, 0x11, 0x10, 0x0B, 0x06, 0x07, 0x42, 0x03, 0x13, 0x14, 0x15, 0x1C,
    0x1F, 0x21, 0x23, 0x2F, 0x30, 0x31, 0x33, 0x3C,
];

struct Rig {
    elm: Elm327,
    adapter: DemoAdapterClock,
}

fn rig(clock: &DemoClock, adapter: DemoAdapter, batching: bool, round_trip_ms: f64) -> Rig {
    let mut options = DemoOptions::with_clock(clock);
    options.adapter = adapter;
    options.round_trip_ms = round_trip_ms;
    let demo = DemoTransport::new(options);
    let adapter_clock = demo.adapter_clock();
    let mut elm = Elm327::new();
    elm.set_stn_batching(batching);
    elm.connect(Box::new(demo), PROTOCOL_ORDER_CAN_FIRST)
        .expect("connects");
    // As the app does on CAN: every value keeps the unit it came from.
    assert!(elm.set_headers(true));
    Rig {
        elm,
        adapter: adapter_clock,
    }
}

/// One round of live values and the simulated adapter time it took.
fn live_round(rig: &mut Rig, pids: &[i32]) -> (f64, Vec<LiveValue>) {
    let before = rig.adapter.ms();
    let values = Obd2::new(&mut rig.elm).read_live_batch(pids, false);
    (rig.adapter.ms() - before, values)
}

#[test]
fn batched_commands_need_one_round_trip_per_round_of_live_values() {
    let clock = DemoClock::new(300.0);
    for (link, round_trip) in [("USB", 2.0), ("Bluetooth", 30.0)] {
        let mut single = rig(&clock, DemoAdapter::ObdLinkEx, false, round_trip);
        let mut batched = rig(&clock, DemoAdapter::ObdLinkEx, true, round_trip);
        assert!(!single.elm.stn_batch());
        assert!(batched.elm.stn_batch());

        let (slow, one_by_one) = live_round(&mut single, &PIDS);
        let (fast, together) = live_round(&mut batched, &PIDS);
        // Same car, same moment: the same values either way.
        assert_eq!(one_by_one.len(), together.len());
        assert!(one_by_one.len() >= PIDS.len());
        for (a, b) in one_by_one.iter().zip(&together) {
            assert_eq!((a.key, a.value), (b.key, b.value));
        }
        println!(
            "{link}: 24 PIDs one request per group {slow:.0} ms, batched {fast:.0} ms ({:.0} % less)",
            100.0 * (1.0 - fast / slow)
        );
        // Four groups were four round trips; batched it is one.
        assert!(
            (slow - fast - 3.0 * round_trip).abs() < 0.5,
            "{link}: {slow} vs {fast}"
        );
    }
}

#[test]
fn a_chip_without_batched_commands_reads_the_same_way_as_an_elm327() {
    let clock = DemoClock::new(300.0);
    let mut sx = rig(&clock, DemoAdapter::ObdLinkStn, true, 2.0);
    assert!(sx.elm.is_stn());
    assert!(!sx.elm.stn_batch());
    let mut clone = rig(&clock, DemoAdapter::CloneV15, true, 2.0);
    let (_, a) = live_round(&mut sx, &PIDS);
    let (_, b) = live_round(&mut clone, &PIDS);
    assert_eq!(a.len(), b.len());
}

#[test]
fn the_body_units_on_ms_can_and_sw_can_are_reached_after_switching() {
    let clock = DemoClock::new(300.0);
    let mut rig = rig(&clock, DemoAdapter::ObdLinkMxPlus, true, 0.0);
    let elm = &mut rig.elm;
    assert_eq!(
        elm.stn_extra_buses(),
        [StnBus::MediumSpeed, StnBus::SingleWire]
    );
    let high_speed_text = elm.protocol_text().to_string();
    let body_ms = ecu_from_ids(0x726, 0x72E, false, "BCM");
    let body_sw = ecu_from_ids(0x241, 0x641, false, "BCM");

    // On the high speed CAN neither answers.
    let found = scan_ecus(elm, &[body_ms.clone(), body_sw.clone()], None, None);
    assert!(found.iter().all(|e| !e.present));

    elm.switch_stn_bus(StnBus::MediumSpeed).expect("MS-CAN");
    assert_eq!(elm.stn_bus(), StnBus::MediumSpeed);
    assert!(elm.protocol_text().contains("STP 53"));
    let found = scan_ecus(elm, std::slice::from_ref(&body_ms), None, None);
    assert!(found[0].present);
    assert_eq!(found[0].info.system_name.trim(), "BODY CTRL MS");
    assert_eq!(found[0].dtcs[0].code, "B1342");
    // OBD to the functional address goes nowhere on this bus.
    assert!(!elm.request("0100", 0x01, 2000).ok());

    elm.switch_stn_bus(StnBus::SingleWire).expect("SW-CAN");
    {
        let mut uds = Uds::new(elm);
        uds.select(&body_sw).expect("select");
        assert!(uds.tester_present());
        assert_eq!(uds.read_info().system_name.trim(), "BODY CTRL SW");
        uds.restore_default();
    }

    elm.switch_stn_bus(StnBus::HighSpeed).expect("back");
    assert_eq!(elm.protocol_text(), high_speed_text);
    assert!(elm.request("0100", 0x01, 2000).ok());
}

#[test]
fn the_obdlink_ex_has_no_single_wire_can() {
    let clock = DemoClock::new(300.0);
    let mut rig = rig(&clock, DemoAdapter::ObdLinkEx, true, 0.0);
    assert_eq!(rig.elm.stn_extra_buses(), [StnBus::MediumSpeed]);
    assert!(rig.elm.switch_stn_bus(StnBus::SingleWire).is_err());
    assert_eq!(rig.elm.stn_bus(), StnBus::HighSpeed);
    assert!(rig.elm.request("0100", 0x01, 2000).ok());
}
