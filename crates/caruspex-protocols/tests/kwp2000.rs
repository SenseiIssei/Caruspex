//! Checks KWP2000 and the switch that decides based on the key bytes
//! (tests/test_kwp2000.cpp). This is the path an Audi A2 1.6 FSI with Bosch
//! MED takes.

mod common;

use caruspex_protocols::kwp1281::Ecu;
use caruspex_protocols::kwp2000::{
    KLineProtocol, negative_response_text, protocol_from_key_bytes, session,
};
use caruspex_protocols::vag_kline::VagKLine;
use common::fake_ecu::FakeEcu;
use common::fake_ecu2000::FakeEcu2000;
use common::{approx, german};

fn line2000(ecu: FakeEcu2000) -> VagKLine<FakeEcu2000> {
    let mut line = VagKLine::new(ecu, None);
    line.connect(Ecu::Engine).expect("connects");
    line
}

#[test]
fn key_bytes_reveal_the_protocol() {
    // The old block protocol announces itself with 0x01 0x8A.
    assert_eq!(protocol_from_key_bytes(0x01, 0x8A), KLineProtocol::Kwp1281);
    // KWP2000 announces itself with 0x6B or 0x6F and relatives.
    assert_eq!(protocol_from_key_bytes(0xEF, 0x8F), KLineProtocol::Kwp2000);
    assert_eq!(protocol_from_key_bytes(0x6B, 0x8F), KLineProtocol::Kwp2000);
    // Nonsense stays unknown instead of being guessed wrong.
    assert_eq!(protocol_from_key_bytes(0x00, 0x00), KLineProtocol::Unknown);
}

#[test]
fn the_switch_picks_kwp1281_for_an_old_control_unit() {
    let _lang = german();
    let mut line = VagKLine::new(FakeEcu::default(), None);
    line.connect(Ecu::Engine).expect("connects");
    assert_eq!(line.protocol(), KLineProtocol::Kwp1281);
    assert!(line.connected());
    assert!(!line.identification().is_empty());
}

#[test]
fn the_switch_picks_kwp2000_for_the_fsi_control_unit() {
    let _lang = german();
    let line = line2000(FakeEcu2000::default());
    assert_eq!(line.protocol(), KLineProtocol::Kwp2000);
    assert!(line.connected());
    assert_eq!(line.stream().woken_address(), 0x01);
    assert_eq!(line.stream().last_session, i32::from(session::DIAGNOSTICS));
    assert_eq!(line.identification().len(), 1);
    assert!(line.identification()[0].contains("MED"));
}

#[test]
fn read_and_clear_faults_over_kwp2000() {
    let _lang = german();
    let mut ecu = FakeEcu2000::default();
    // 0x4066 is 16486 and therefore P0102, 0x0231 is the old VAG number 561.
    ecu.faults = vec![0x40, 0x66, 0xE0, 0x02, 0x31, 0x60];
    let mut line = line2000(ecu);

    let faults = line.read_faults().expect("faults");
    assert_eq!(faults.len(), 2);
    assert_eq!(faults[0].code, 16486);
    assert!(faults[0].text.contains("Luftmassenmesser"));
    assert_eq!(faults[1].code, 561);
    assert!(faults[1].text.contains("Gemischadaption"));

    line.clear_faults().expect("cleared");
    assert!(line.stream().clear_was_called);

    assert!(line.read_faults().expect("faults").is_empty());
}

#[test]
fn measuring_block_over_kwp2000() {
    let _lang = german();
    let mut line = line2000(FakeEcu2000::default());
    let values = line.read_group(3).expect("values");
    assert_eq!(line.stream().last_group, 3);
    assert_eq!(values.len(), 4);
    assert_eq!(values[0].unit, "1/min");
    assert!(approx(values[0].value, 0.2 * 64.0 * 100.0));
}

#[test]
fn basic_settings_over_kwp2000_open_a_separate_session() {
    let _lang = german();
    let mut line = line2000(FakeEcu2000::default());
    let sessions_after_connect = line.stream().session_count;

    let values = line.basic_setting(98).expect("values");
    assert_eq!(line.stream().last_session, i32::from(session::ADAPTATION));
    assert_eq!(line.stream().session_count, sessions_after_connect + 1);
    assert_eq!(line.stream().last_group, 98);
    assert!(!values.is_empty());
}

#[test]
fn a_rejected_session_type_is_reported_cleanly() {
    let _lang = german();
    let mut ecu = FakeEcu2000::default();
    ecu.refuse_session = i32::from(session::ADAPTATION);
    let mut line = line2000(ecu);

    let error = line.basic_setting(98).expect_err("refused");
    assert!(!error.is_empty());
    // The reason should be in plain text, not a bare number.
    assert!(error.contains("Bedingungen"), "{error}");
}

#[test]
fn a_different_session_type_can_be_specified() {
    let _lang = german();
    let mut line = line2000(FakeEcu2000::default());
    line.set_basic_setting_session(0x86);
    line.basic_setting(60).expect("values");
    assert_eq!(line.stream().last_session, 0x86);
}

#[test]
fn keep_the_connection_alive_over_kwp2000() {
    let _lang = german();
    let mut line = line2000(FakeEcu2000::default());
    assert_eq!(line.keep_alive(), Ok(()));
}

#[test]
fn disconnecting_signs_off_from_the_kwp2000_control_unit() {
    let _lang = german();
    let mut line = line2000(FakeEcu2000::default());
    line.disconnect();
    assert!(!line.connected());
    assert!(line.stream().stopped);
    // Handing the stream back signs off as well.
    let mut again = line2000(line.into_stream());
    assert!(again.connected());
    again.keep_alive().expect("alive");
    assert!(again.into_stream().stopped);
}

#[test]
fn what_kwp2000_cannot_do_is_reported_instead_of_failing_silently() {
    let _lang = german();
    let mut line = line2000(FakeEcu2000::default());

    let error = line.read_adaptation(1).expect_err("not for KWP2000");
    assert!(error.contains("KWP2000"));

    let error = line.actuator_test().expect_err("not for KWP2000");
    assert!(error.contains("KWP2000"));
}

#[test]
fn rejected_services_come_back_in_plain_text() {
    let _lang = german();
    assert!(negative_response_text(0x11).contains("nicht unterstützt"));
    assert!(negative_response_text(0x22).contains("Bedingungen"));
    assert!(negative_response_text(0x33).contains("Freischaltcode"));
    assert!(!negative_response_text(0xAB).is_empty());
}
