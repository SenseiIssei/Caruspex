//! Parsing of adapter lines: tests/test_protocol.cpp ([parser]) and the
//! multi-sender cases of tests/test_obd_extended.cpp ([parser][multi]).

mod common;

use caruspex_i18n::Language;
use caruspex_link::{ElmStatus, HeaderFormat, decode_messages, header_format_for, hex_to_bytes};

/// The VIN out of a mode 09 PID 02 message (PID, count, 17 characters).
fn vin_of(message: &[u8]) -> String {
    String::from_utf8(message[2..].to_vec()).unwrap()
}

#[test]
fn hex_lines_become_bytes() {
    assert_eq!(
        hex_to_bytes("4100BE1FA813"),
        [0x41, 0x00, 0xBE, 0x1F, 0xA8, 0x13]
    );
    assert_eq!(hex_to_bytes("41 00 BE"), [0x41, 0x00, 0xBE]);
    assert!(hex_to_bytes("").is_empty());
    // Odd number of digits: the last half byte is dropped.
    assert_eq!(hex_to_bytes("4100B"), [0x41, 0x00]);
}

#[test]
fn responses_to_service_03_are_recognized() {
    let result = decode_messages(&["430507155900 00"], 0x03, HeaderFormat::None);
    assert!(result.ok());
    assert_eq!(result.messages.len(), 1);
    assert_eq!(result.messages[0][0], 0x05);
    assert_eq!(result.messages[0][1], 0x07);
}

#[test]
fn multi_line_can_responses_are_assembled() {
    let result = decode_messages(
        &["014", "0:4902014741", "1:554E45"],
        0x09,
        HeaderFormat::None,
    );
    assert!(result.ok());
}

#[test]
fn adapter_messages_are_reported_as_errors() {
    let status =
        |lines: &[&str], service| decode_messages(lines, service, HeaderFormat::None).status;
    assert_eq!(status(&["NO DATA"], 0x01), ElmStatus::NoData);
    assert_eq!(status(&["UNABLE TO CONNECT"], 0x01), ElmStatus::BusError);
    assert_eq!(status(&["BUS INIT: ERROR"], 0x01), ElmStatus::BusError);
    assert_eq!(status(&["7F0312"], 0x03), ElmStatus::Negative);
}

#[test]
fn adapter_messages_follow_the_active_language() {
    {
        let _english = common::language(Language::English);
        let r = decode_messages(&["NO DATA"], 0x01, HeaderFormat::None);
        assert!(r.message.contains("does not support"));
        let r = decode_messages(&["7F0312"], 0x03, HeaderFormat::None);
        assert_eq!(r.message, "Control unit rejects the service (code 0x12)");
    }
    let _german = common::language(Language::German);
    let r = decode_messages(&["CAN ERROR"], 0x01, HeaderFormat::None);
    assert_eq!(r.message, "Adapter meldet: CAN ERROR");
}

#[test]
fn headers_off_several_answers_stay_apart() {
    // Two single-frame answers to 0100.
    let single = decode_messages(&["4100BE3FA813", "410098180001"], 0x01, HeaderFormat::None);
    assert_eq!(single.messages.len(), 2);
    assert_eq!(single.sources, ["", ""]);
    assert_eq!(single.messages[1][1], 0x98);

    // Two multi-frame VINs one after the other, each with its length line.
    let vins = decode_messages(
        &[
            "014",
            "0:490201575657",
            "1:5A5A5A314B5A41",
            "2:57303030303031",
            "014",
            "0:490201575657",
            "1:5A5A5A314B5A41",
            "2:57303030303032",
        ],
        0x09,
        HeaderFormat::None,
    );
    assert_eq!(vins.messages.len(), 2);
    assert_eq!(vin_of(&vins.messages[0]), "WVWZZZ1KZAW000001");
    assert_eq!(vin_of(&vins.messages[1]), "WVWZZZ1KZAW000002");

    // A one-byte answer (clear codes) is not mistaken for a length line.
    assert!(decode_messages(&["44"], 0x04, HeaderFormat::None).ok());
}

#[test]
fn can_11_bit_with_headers_single_and_multi_frame() {
    // Padding after the single frame length is cut off.
    let r = decode_messages(
        &["7E8 06 41 00 BE 3F A8 13 00", "7E9064100981800 01AA"],
        0x01,
        HeaderFormat::Can11,
    );
    assert!(r.ok());
    assert_eq!(r.messages.len(), 2);
    assert_eq!(r.sources, ["7E8", "7E9"]);
    assert_eq!(r.messages[0], [0x00, 0xBE, 0x3F, 0xA8, 0x13]);
    assert_eq!(r.messages[1], [0x00, 0x98, 0x18, 0x00, 0x01]);

    // Two multi-frame VINs, interleaved frame by frame as on a real bus.
    let vin = decode_messages(
        &[
            "7E8 10 14 49 02 01 57 56 57",
            "7E9 10 14 49 02 01 57 56 57",
            "7E8 21 5A 5A 5A 31 4B 5A 41",
            "7E9 21 5A 5A 5A 31 4B 5A 41",
            "7E9 22 57 30 30 30 30 30 32",
            "7E8 22 57 30 30 30 30 30 31",
        ],
        0x09,
        HeaderFormat::Can11,
    );
    assert_eq!(vin.messages.len(), 2);
    assert_eq!(vin.sources, ["7E9", "7E8"]);
    assert_eq!(vin_of(&vin.messages[0]), "WVWZZZ1KZAW000002");
    assert_eq!(vin_of(&vin.messages[1]), "WVWZZZ1KZAW000001");

    // Without spaces (ATS0) the same.
    let compact = decode_messages(
        &[
            "7E8101449020157565 7",
            "7E8215A5A5A314B5A41",
            "7E82257303030303031",
        ],
        0x09,
        HeaderFormat::Can11,
    );
    assert_eq!(compact.messages.len(), 1);
    assert_eq!(vin_of(&compact.messages[0]), "WVWZZZ1KZAW000001");
}

#[test]
fn can_29_bit_with_headers() {
    // Engine answers, transmission refuses: the answer counts.
    let r = decode_messages(
        &["18DAF11004410C1AF8AAAA", "18DAF118037F0112"],
        0x01,
        HeaderFormat::Can29,
    );
    assert!(r.ok());
    assert_eq!(r.messages.len(), 1);
    assert_eq!(r.sources[0], "18DAF110");
    assert_eq!(r.messages[0], [0x0C, 0x1A, 0xF8]);

    // Only a refusal: that is the result.
    assert_eq!(
        decode_messages(&["18DAF118037F0112"], 0x01, HeaderFormat::Can29).status,
        ElmStatus::Negative
    );

    // Four codes in two frames, padding at the end is cut off.
    let dtc = decode_messages(
        &[
            "18DAF110 10 0A 43 04 01 33 03 01",
            "18DAF110 21 04 20 07 00 AA AA AA",
        ],
        0x03,
        HeaderFormat::Can29,
    );
    assert_eq!(dtc.messages.len(), 1);
    assert_eq!(
        dtc.messages[0],
        [0x04, 0x01, 0x33, 0x03, 0x01, 0x04, 0x20, 0x07, 0x00]
    );
}

#[test]
fn k_line_and_j1850_with_headers() {
    // ISO 9141-2: 48 6B source, data, checksum.
    let iso = decode_messages(
        &["48 6B 10 41 0C 1A F8 C2", "48 6B 18 41 0C 00 00 00"],
        0x01,
        HeaderFormat::KLine,
    );
    assert_eq!(iso.messages.len(), 2);
    assert_eq!(iso.sources, ["10", "18"]);
    assert_eq!(iso.messages[0], [0x0C, 0x1A, 0xF8]);

    // ISO 14230 with the length in the format byte, and with a length byte.
    let kwp = decode_messages(
        &["83F110410D32E6", "80F11803410D32F0"],
        0x01,
        HeaderFormat::KLine,
    );
    assert_eq!(kwp.messages.len(), 2);
    assert_eq!(kwp.messages[0], [0x0D, 0x32]);
    assert_eq!(kwp.messages[1], [0x0D, 0x32]);
    assert_eq!(kwp.sources[1], "18");

    let j1850 = decode_messages(
        &["48 6B 10 43 01 33 00 00 00 00 5A"],
        0x03,
        HeaderFormat::J1850,
    );
    assert_eq!(j1850.messages[0], [0x01, 0x33, 0, 0, 0, 0]);
    assert_eq!(header_format_for('6'), HeaderFormat::Can11);
    assert_eq!(header_format_for('9'), HeaderFormat::Can29);
    assert_eq!(header_format_for('5'), HeaderFormat::KLine);
}
