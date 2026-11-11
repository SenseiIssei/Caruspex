//! Broadcast values: DBC signals (Intel and Motorola order, signed, with a
//! multiplexer) and the Tesla frames decoded with the signals of the Tesla
//! pack. Frame bytes are worked out by hand from the DBC definitions.
//!
//! The listen-only connection (Elm327) and the pack and CSV import
//! (CustomPids, PidPacks) are tested in the crates that own them.

use caruspex_net::can_signals::{decode_can_signal, parse_can_signal, parse_monitor_request};

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn dbc_signals_are_parsed() {
    let s = parse_can_signal("DBC:0|16@1+(0.01,0)").unwrap();
    assert_eq!(s.start, 0);
    assert_eq!(s.length, 16);
    assert!(s.intel);
    assert!(!s.is_signed);
    assert!(near(s.factor, 0.01));
    assert_eq!(s.mux_value, -1);

    let s = parse_can_signal("DBC:16|16@1-(-0.1,0)").unwrap();
    assert!(s.is_signed);
    assert!(near(s.factor, -0.1));

    let s = parse_can_signal("DBC:2|12@1+(0.002,0) m1@0|2").unwrap();
    assert_eq!(s.mux_value, 1);
    assert_eq!(s.mux_start, 0);
    assert_eq!(s.mux_length, 2);
    assert!(s.mux_intel);

    let s = parse_can_signal("DBC:7|8@0+(0.5,-40)").unwrap();
    assert!(!s.intel);
    assert!(near(s.offset, -40.0));

    assert!(parse_can_signal("0|16@1+(1,0)").is_err());
    assert!(parse_can_signal("DBC:0|16@2+(1,0)").is_err());
    let error = parse_can_signal("DBC:0|99@1+(1,0)").unwrap_err();
    assert!(!error.is_empty());
    assert!(error.ends_with("DBC:0|99@1+(1,0)"));
    assert!(parse_can_signal("DBC:nonsense").is_err());

    assert_eq!(parse_monitor_request("CAN132"), Some(0x132));
    assert_eq!(parse_monitor_request("can3b6"), Some(0x3B6));
    assert_eq!(parse_monitor_request("CAN800"), None);
    assert_eq!(parse_monitor_request("220101"), None);
}

#[test]
fn dbc_signals_are_decoded_in_both_byte_orders() {
    let s = parse_can_signal("DBC:0|16@1+(0.01,0)").unwrap();
    assert!(near(decode_can_signal(&s, &[0x10, 0x96]).unwrap(), 384.16));
    assert!(decode_can_signal(&s, &[0x10]).is_none()); // frame too short

    let s = parse_can_signal("DBC:16|16@1-(-0.1,0)").unwrap();
    assert!(near(
        decode_can_signal(&s, &[0, 0, 0x38, 0xFF]).unwrap(),
        20.0
    ));

    // Motorola: 7|16 is byte 0 then byte 1; 4|8 runs from bit 4 of byte 0
    // down and on into bits 7 to 5 of byte 1.
    let s = parse_can_signal("DBC:7|16@0+(1,0)").unwrap();
    assert!(near(
        decode_can_signal(&s, &[0x12, 0x34]).unwrap(),
        f64::from(0x1234)
    ));
    let s = parse_can_signal("DBC:4|8@0+(1,0)").unwrap();
    assert!(near(
        decode_can_signal(&s, &[0x16, 0xA0]).unwrap(),
        f64::from(0xB5)
    ));

    // Multiplexed: only in frames with the right multiplexer value.
    let s = parse_can_signal("DBC:2|12@1+(0.002,0) m1@0|2").unwrap();
    assert!(near(
        decode_can_signal(&s, &[0x09, 0x20, 0xF8, 0x07, 0x00, 0x00]).unwrap(),
        4.1
    ));
    assert!(decode_can_signal(&s, &[0x00, 0x00, 0x82, 0x78, 0x00, 0x00]).is_none());
    let s = parse_can_signal("DBC:16|8@1+(0.5,-40) m0@0|2").unwrap();
    assert!(near(
        decode_can_signal(&s, &[0x00, 0x00, 0x82, 0x78, 0x00, 0x00]).unwrap(),
        25.0
    ));
}

fn frame_of(value: u64) -> Vec<u8> {
    value.to_le_bytes().to_vec()
}

/// The values of the C++ listen-only test, from the frames of a Model 3 and
/// the signals of data/pids/tesla-model3-y.csv.
#[test]
fn tesla_frames_decode_with_the_pack_signals() {
    let f132 = [0x10, 0x96, 0x38, 0xFF, 0x00, 0x00, 0x00, 0x00];
    // SOCmin 80.0, SOCUI 80.5, SOCmax 81.0, SOCave 80.7, BOL 82.0 kWh.
    let f292 = frame_of(800 | (805 << 10) | (810 << 20) | (807 << 30) | (820 << 40));
    // Full pack 74.6 kWh (746), remaining 60.1 kWh (601 at bit 11).
    let f352 = frame_of(746 | (601 << 11));
    // Multiplexer 1 with cells 4.100 V / 4.080 V, multiplexer 0 with
    // temperatures 25 and 20 degrees.
    let f332 = [
        vec![0x09, 0x20, 0xF8, 0x07, 0x00, 0x00],
        vec![0x00, 0x00, 0x82, 0x78, 0x00, 0x00],
    ];

    let value = |signal: &str, frames: &[&[u8]]| -> Option<f64> {
        let s = parse_can_signal(signal).unwrap();
        frames.iter().find_map(|f| decode_can_signal(&s, f))
    };
    let cells: Vec<&[u8]> = f332.iter().map(Vec::as_slice).collect();
    assert!(near(
        value("DBC:0|16@1+(0.01,0)", &[&f132]).unwrap(),
        384.16
    ));
    assert!(near(value("DBC:16|16@1-(-0.1,0)", &[&f132]).unwrap(), 20.0));
    assert!(near(value("DBC:10|10@1+(0.1,0)", &[&f292]).unwrap(), 80.5));
    assert!(near(value("DBC:30|10@1+(0.1,0)", &[&f292]).unwrap(), 80.7));
    assert!(near(value("DBC:40|10@1+(0.1,0)", &[&f292]).unwrap(), 82.0));
    assert!(near(value("DBC:0|11@1+(0.1,0)", &[&f352]).unwrap(), 74.6));
    assert!(near(value("DBC:11|11@1+(0.1,0)", &[&f352]).unwrap(), 60.1));
    assert!(near(
        value("DBC:2|12@1+(0.002,0) m1@0|2", &cells).unwrap(),
        4.1
    ));
    assert!(near(
        value("DBC:16|12@1+(0.002,0) m1@0|2", &cells).unwrap(),
        4.08
    ));
    assert!(near(
        value("DBC:16|8@1+(0.5,-40) m0@0|2", &cells).unwrap(),
        25.0
    ));
    assert!(near(
        value("DBC:24|8@1+(0.5,-40) m0@0|2", &cells).unwrap(),
        20.0
    ));
}
