//! Checks the KWP1281 state machine against a simulated control unit
//! (tests/test_kwp1281.cpp). This tests the path to the throttle body basic
//! settings without a car.

mod common;

use caruspex_protocols::kwp1281::{Ecu, Kwp1281, decode_measurement, ecu_name, known_ecus};
use common::fake_ecu::FakeEcu;
use common::{approx, german};

fn connected(ecu: FakeEcu) -> Kwp1281<FakeEcu> {
    let mut protocol = Kwp1281::new(ecu, None);
    protocol.connect(Ecu::Engine).expect("connects");
    protocol
}

#[test]
fn connecting_wakes_the_right_control_unit() {
    let _lang = german();
    let mut protocol = Kwp1281::new(FakeEcu::default(), None);
    assert_eq!(protocol.connect(Ecu::Engine), Ok(()));
    assert_eq!(protocol.stream().woken_address(), 0x01);
    assert!(protocol.connected());
}

#[test]
fn the_control_unit_identification_is_read() {
    let _lang = german();
    let protocol = connected(FakeEcu::with_identification(&[
        "1C0906032AA ",
        "SIMOS 1.0 ",
        "Code 12345 ",
    ]));
    assert_eq!(protocol.identification().len(), 3);
    assert_eq!(protocol.identification()[0], "1C0906032AA ");
    assert_eq!(protocol.identification()[1], "SIMOS 1.0 ");
}

#[test]
fn other_control_units_can_be_addressed() {
    let _lang = german();
    let mut protocol = Kwp1281::new(FakeEcu::default(), None);
    protocol.connect(Ecu::Abs).expect("connects");
    assert_eq!(protocol.stream().woken_address(), 0x03);
}

#[test]
fn read_and_clear_the_fault_memory() {
    let _lang = german();
    let mut ecu = FakeEcu::default();
    // Two faults: 0x111B (4379) and 0x111C, the second one intermittent.
    ecu.fault_payload = vec![0x11, 0x1B, 0x24, 0x11, 0x1C, 0xA4];
    let mut protocol = connected(ecu);

    let faults = protocol.read_faults().expect("faults");
    assert_eq!(faults.len(), 2);
    assert_eq!(faults[0].code, 0x111B);
    assert_eq!(faults[1].code, 0x111C);
    // Bit 7 in the status byte means intermittent.
    assert!(faults[1].sporadic);
    assert!(!faults[0].sporadic);

    protocol.clear_faults().expect("cleared");
    assert!(protocol.stream().clear_was_called);

    assert!(protocol.read_faults().expect("faults").is_empty());
}

#[test]
fn read_a_measuring_block() {
    let _lang = german();
    let mut protocol = connected(FakeEcu::default());
    let values = protocol.read_group(1).expect("values");
    assert_eq!(protocol.stream().last_group_requested, 1);
    assert_eq!(values.len(), 4);
    // Formula 0x01 is engine speed: 0.2 times A times B.
    assert_eq!(values[0].unit, "1/min");
    assert!(approx(values[0].value, 0.2 * 64.0 * 100.0));
}

#[test]
fn basic_settings_is_a_separate_service() {
    let _lang = german();
    let mut protocol = connected(FakeEcu::default());
    let values = protocol.basic_setting(60).expect("values");
    assert_eq!(protocol.stream().last_basic_setting_group, 60);
    assert_eq!(protocol.stream().last_group_requested, -1); // not confused with reading
    assert!(!values.is_empty());
}

#[test]
fn control_units_without_basic_settings_report_it_cleanly() {
    let _lang = german();
    let mut ecu = FakeEcu::default();
    ecu.supports_basic_setting = false;
    let mut protocol = connected(ecu);
    let error = protocol.basic_setting(98).expect_err("refused");
    assert!(!error.is_empty());
}

#[test]
fn several_requests_in_a_row_keep_the_rhythm() {
    let _lang = german();
    let mut protocol = connected(FakeEcu::default());
    for round in 0..5u8 {
        protocol.read_group(round + 1).expect("values");
        assert_eq!(protocol.stream().last_group_requested, i32::from(round) + 1);
    }
}

#[test]
fn disconnecting_signs_off_from_the_control_unit() {
    let _lang = german();
    let mut protocol = connected(FakeEcu::default());
    protocol.disconnect();
    assert!(!protocol.connected());
    assert_eq!(protocol.stream().end_output_count, 1);
}

#[test]
fn measuring_value_formulas_compute_correctly() {
    let _lang = german();
    // Engine speed
    let rpm = decode_measurement(0x01, 0x40, 0x64);
    assert_eq!(rpm.unit, "1/min");
    assert!(approx(rpm.value, 1280.0));

    // Temperature: A times (B minus 100) times 0.1
    let temp = decode_measurement(0x05, 0x01, 0xE2);
    assert_eq!(temp.unit, "Grad C");
    assert!(approx(temp.value, 0.1 * 1.0 * (226.0 - 100.0)));

    // Voltage
    let volt = decode_measurement(0x06, 0x0A, 0x8C);
    assert_eq!(volt.unit, "V");
    assert!(approx(volt.value, 0.001 * 10.0 * 140.0));

    // Empty field
    assert!(!decode_measurement(0x00, 0x00, 0x00).valid);

    // An unknown formula does not fall over but returns the raw number.
    let unknown = decode_measurement(0xEE, 0x12, 0x34);
    assert!(unknown.valid);
    assert!(approx(unknown.value, f64::from(0x12 * 256 + 0x34)));
}

#[test]
fn control_unit_names_are_known() {
    let _lang = german();
    assert_eq!(ecu_name(Ecu::Engine), "01 Motor");
    assert_eq!(ecu_name(Ecu::Abs), "03 Bremsenelektronik (ABS)");
    assert!(known_ecus().len() >= 10);
    assert_eq!(Ecu::from_address(0x17), Some(Ecu::InstrumentCluster));
    assert_eq!(Ecu::from_address(0x99), None);
}

#[test]
fn read_an_adaptation_channel() {
    let _lang = german();
    let mut ecu = FakeEcu::default();
    ecu.adaptation_value = 132;
    let mut protocol = connected(ecu);
    let (value, shown) = protocol.read_adaptation(1).expect("channel");
    assert_eq!(protocol.stream().last_adaptation_channel, 1);
    assert_eq!(value, 132);
    assert_eq!(shown.len(), 4);
}

#[test]
fn control_units_without_adaptation_report_it_cleanly() {
    let _lang = german();
    let mut ecu = FakeEcu::default();
    ecu.supports_adaptation = false;
    let mut protocol = connected(ecu);
    let error = protocol.read_adaptation(99).expect_err("refused");
    assert!(!error.is_empty());
}

#[test]
fn output_test_names_the_active_component() {
    let _lang = german();
    let mut ecu = FakeEcu::default();
    ecu.actuator_name = "Tankentlueftungsventil N80".into();
    let mut protocol = connected(ecu);
    let actuator = protocol.actuator_test().expect("actuator");
    assert_eq!(actuator, "Tankentlueftungsventil N80");
    assert_eq!(protocol.stream().actuator_test_count, 1);
}

#[test]
fn vag_fault_numbers_get_plain_text() {
    let _lang = german();
    let mut ecu = FakeEcu::default();
    // 16486 corresponds to P0102, 561 is an old VAG-specific number.
    ecu.fault_payload = vec![0x40, 0x66, 0x24, 0x02, 0x31, 0x24];
    let mut protocol = connected(ecu);
    let faults = protocol.read_faults().expect("faults");
    assert_eq!(faults.len(), 2);
    assert_eq!(faults[0].code, 16486);
    assert!(faults[0].text.contains("P0102"));
    assert!(faults[0].text.contains("Luftmassenmesser"));
    assert_eq!(faults[1].code, 561);
    assert!(faults[1].text.contains("Gemischadaption"));
}
