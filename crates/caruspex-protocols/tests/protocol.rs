//! The Obd2 and K-line parts of tests/test_protocol.cpp and the Obd2 part of
//! tests/test_obd_on_uds.cpp. The parser and table cases of those files are
//! ported in caruspex-link and caruspex-data.

mod common;

use caruspex_data::dtc::describe_dtc;
use caruspex_data::pids::find_pid;
use caruspex_data::vag_faults::describe_vag_fault;
use caruspex_i18n::{Language, language as active_language, set_language};
use caruspex_link::{Elm327, HeaderFormat, decode_messages};
use caruspex_protocols::kwp1281::{Ecu, decode_measurement, ecu_name};
use caruspex_protocols::kwp2000::negative_response_text;
use caruspex_protocols::obd2::{Fault, Obd2};
use common::*;

fn fault_elm() -> Elm327 {
    let mut elm = Elm327::new();
    elm.connect(Box::new(FaultTransport::default()), "6")
        .expect("connects");
    elm
}

fn read_test_faults() -> Vec<Fault> {
    let mut elm = fault_elm();
    Obd2::new(&mut elm).read_faults()
}

#[test]
fn oxygen_sensor_test_and_k_line_monitoring_are_decoded() {
    let _lang = german();
    let mut elm = fault_elm();
    let mut obd = Obd2::new(&mut elm);

    // Mode 05: one byte per value, 0.005 V per bit.
    let o2 = obd.read_oxygen_sensor_tests().expect("results");
    assert_eq!(o2.len(), 1);
    assert_eq!(o2[0].test_id, 0x01);
    assert_eq!(o2[0].sensor, 1);
    assert!(approx(o2[0].value, 0.700));
    assert_eq!(o2[0].unit, "V");

    // Mode 06 without CAN: ID, component, value, lower and upper limit.
    let monitor = obd.read_monitor_results().expect("results");
    assert_eq!(monitor.len(), 1);
    assert!(approx(monitor[0].value, 0.750));
    assert!(approx(monitor[0].minimum, 0.400));
    assert!(approx(monitor[0].maximum, 1.000));
    assert!(monitor[0].passed);
}

#[test]
fn core_texts_follow_the_active_language() {
    let _lang = german();
    assert_eq!(active_language(), Language::German);

    // German is the default.
    let faults = read_test_faults();
    assert_eq!(faults.len(), 2);
    assert_eq!(faults[0].kind, "gespeichert");
    assert_eq!(faults[1].kind, "sporadisch");

    set_language(Language::English);
    // Protocol and adapter messages from the core itself.
    let faults = read_test_faults();
    assert_eq!(faults.len(), 2);
    assert_eq!(faults[0].code, "P0507");
    assert_eq!(faults[0].kind, "stored");
    assert_eq!(faults[1].kind, "intermittent");
    assert_eq!(ecu_name(Ecu::Engine), "01 Engine");
    assert!(negative_response_text(0x22).contains("Conditions not correct"));
    assert!(
        decode_messages(&["NO DATA"], 0x01, HeaderFormat::None)
            .message
            .contains("does not support")
    );
    assert_eq!(decode_measurement(0x01, 0x40, 0x64).label, "Engine speed");

    // Data tables, resolved through the public lookups.
    assert!(find_pid(0x0C).unwrap().name.contains("Engine"));
    assert!(describe_dtc("P0171").contains("lean"));
    let vag = describe_vag_fault(16486, 0x01);
    assert!(vag.contains("P0102"));
    assert!(!vag.contains("Luftmassenmesser"));

    set_language(Language::German);
    assert_eq!(ecu_name(Ecu::Engine), "01 Motor");
    assert!(describe_dtc("P0171").contains("mager"));
}

#[test]
fn obd_on_uds_detected_at_connect_classic_results_from_uds_answers() {
    let _lang = language(Language::English);
    for headers in [false, true] {
        let car = scripted_uds_car();
        let sent = car.sent.clone();
        let mut elm = Elm327::new();
        elm.connect(Box::new(car), "6").expect("connects");
        assert!(elm.obd_on_uds(), "headers {headers}");
        if headers {
            assert!(elm.set_headers(true));
        }

        let mut obd = Obd2::new(&mut elm);
        let pids = obd.supported_pids();
        assert!(pids.contains(&0x0C));
        assert!(pids.contains(&0x05));

        let rpm = obd.read_live(0x0C).expect("rpm");
        assert_eq!(rpm.value, 800.0); // 0x0C80 / 4
        let coolant = obd.read_live(0x05).expect("coolant"); // after one "busy" answer
        assert_eq!(coolant.value, 83.0);
        assert_eq!(count_sent(&sent, "22F405"), 2);

        assert_eq!(obd.read_vin(), "WAUZZZ8V5LA012345");

        let codes: Vec<String> = obd.read_faults().into_iter().map(|f| f.code).collect();
        assert!(codes.iter().any(|c| c == "P0171"));
        assert!(codes.iter().any(|c| c == "P0420"));
        assert!(codes.iter().any(|c| c == "P0300")); // permanent

        obd.clear_faults().expect("cleared");
        assert_eq!(count_sent(&sent, "14FFFF33"), 1);
        // No classic service was sent after the connection was made.
        assert_eq!(count_sent(&sent, "0300"), 0);
        assert_eq!(count_sent(&sent, "03"), 0);
    }
}
