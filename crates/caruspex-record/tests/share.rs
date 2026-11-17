// "Mitschnitt teilen": the serial number of the VIN goes in every form the
// traffic carries it, the port name and the time of day go, and the shared
// file still replays.

mod common;

use caruspex_demo::{DemoClock, DemoOptions, DemoTransport};
use caruspex_i18n::Language;
use caruspex_link::Elm327;
use caruspex_protocols::Obd2;
use caruspex_record::{
    RecordedDirection, RecordedEvent, Recording, RecordingMeta, RecordingTransport, ReplayPacing,
    ReplayTransport, VIN_HIDDEN, find_vins, load_recording, parse_recording, share_recording,
};

use common::language;

const VIN: &str = "WVWZZZ1KZ6W123456";
const MASKED: &str = "WVWZZZ1KZ6W000000";

fn event(t: f64, direction: RecordedDirection, data: &str) -> RecordedEvent {
    RecordedEvent {
        t,
        direction,
        data: data.as_bytes().to_vec(),
    }
}

/// One exchange per form a VIN answer takes.
fn every_form() -> Recording {
    use RecordedDirection::{Note, Rx, Tx};
    Recording {
        version: 1,
        created: "2026-11-17T08:15:42Z".into(),
        transport: "Bluetooth OBDLink CX (00:04:3E:12:34:56)".into(),
        adapter: "ELM327 v1.5".into(),
        events: vec![
            event(
                0.1,
                Note,
                "recording started in a running session, VIN WVWZZZ1KZ6W123456",
            ),
            // CAN, headers off, spaces off, the answer split over two reads
            // in the middle of a byte.
            event(0.2, Tx, "0902\r"),
            event(0.3, Rx, "014\r0:49020157565"),
            event(0.31, Rx, "7\r1:5A5A5A314B5A36\r2:57313233343536\r\r>"),
            // CAN, headers on, with spaces.
            event(0.4, Tx, "0902\r"),
            event(
                0.5,
                Rx,
                "7E8 10 14 49 02 01 57 56 57\r7E8 21 5A 5A 5A 31 4B 5A 36\r7E8 22 57 31 32 33 34 35 36\r\r>",
            ),
            // K-line, four bytes per line.
            event(0.6, Tx, "0902\r"),
            event(
                0.7,
                Rx,
                "49 02 01 00 00 00 57\r49 02 02 56 57 5A 5A\r49 02 03 5A 31 4B 5A\r49 02 04 36 57 31 32\r49 02 05 33 34 35 36\r\r>",
            ),
            // K-line with headers and a checksum byte at the end of each line.
            event(0.8, Tx, "0902\r"),
            event(
                0.9,
                Rx,
                "48 6B 10 49 02 01 00 00 00 57 E1\r48 6B 10 49 02 02 56 57 5A 5A E2\r48 6B 10 49 02 03 5A 31 4B 5A E3\r48 6B 10 49 02 04 36 57 31 32 E4\r48 6B 10 49 02 05 33 34 35 36 E5\r\r>",
            ),
            // UDS 22 F1 90, 29 bit headers without spaces.
            event(1.0, Tx, "22F190\r"),
            event(
                1.1,
                Rx,
                "18DAF1101014 62F190575657\r18DAF110215A5A5A314B5A36\r18DAF1102257313233343536\r\r>",
            ),
            event(1.2, Tx, "010C\r"),
            event(1.3, Rx, "410C1AF8\r\r>"),
        ],
        ..Recording::default()
    }
}

#[test]
fn the_vin_is_found_in_every_form() {
    assert_eq!(find_vins(&every_form()), vec![VIN.to_string()]);
}

#[test]
fn every_form_loses_the_serial_number() {
    let _lang = language(Language::English);
    let shared = share_recording(&every_form(), &[]);
    let text = String::from_utf8(shared.file.clone()).unwrap();
    for serial in ["313233343536", "31 32 33 34 35 36", "123456"] {
        assert!(!text.contains(serial), "{serial} in\n{text}");
    }
    // Maker, model and year stay readable.
    assert!(text.contains("5A5A5A314B5A36"));
    let again = parse_recording(&shared.file).unwrap();
    assert_eq!(again.events.len(), every_form().events.len());
    assert_eq!(find_vins(&again), vec![MASKED.to_string()]);
    // Five hex answers and the note.
    assert_eq!(shared.preview.vin_places, 6);
    assert_eq!(shared.preview.vins, vec![MASKED.to_string()]);
    // The Bluetooth address and the time of day are gone.
    assert!(!text.contains("00:04:3E"), "{text}");
    assert!(again.transport.is_empty());
    assert_eq!(again.created, "2026-11-17");
    assert!(shared.preview.transport_removed);
    // The other answers are untouched.
    assert!(text.contains("410C1AF8"));
    let kinds: Vec<&str> = shared
        .preview
        .requests
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    assert!(
        kinds.contains(&"Vehicle data (VIN, software) (09)"),
        "{kinds:?}"
    );
    assert!(!shared.preview.sample.is_empty());
    assert_eq!(shared.preview.bytes, shared.file.len());
}

#[test]
fn a_known_vin_is_hidden_even_where_the_traffic_hides_it_well() {
    use RecordedDirection::{Rx, Tx};
    // A first frame alone (the rest was lost): not a whole VIN in the
    // traffic, but the program knows it from the session.
    let recording = Recording {
        version: 1,
        events: vec![
            event(0.1, Tx, "ATI\r"),
            event(0.2, Rx, "ELM327 v1.5 WVWZZZ1KZ6W123456\r\r>"),
        ],
        ..Recording::default()
    };
    let shared = share_recording(&recording, &[VIN]);
    let text = String::from_utf8(shared.file).unwrap();
    assert!(text.contains(MASKED), "{text}");
    assert!(!text.contains("123456"));
    assert_eq!(VIN_HIDDEN, 6);
}

#[test]
fn a_shared_demo_session_still_replays() {
    let _lang = language(Language::English);
    let path = std::env::temp_dir()
        .join("caruspex_rs_test_share_demo.jsonl")
        .to_string_lossy()
        .into_owned();
    let clock = DemoClock::new(600.0);
    {
        let mut elm = Elm327::new();
        let recorder = RecordingTransport::new(
            Box::new(DemoTransport::new(DemoOptions {
                clock: Some(clock.source()),
                latency_scale: 0.0,
                ..DemoOptions::default()
            })),
            &path,
            RecordingMeta::default(),
        );
        elm.connect(recorder, "6").expect("connect");
        assert!(elm.set_headers(true));
        assert_eq!(Obd2::new(&mut elm).read_vin(), DemoTransport::vin());
        elm.disconnect();
    }
    let recording = load_recording(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let shared = share_recording(&recording, &[]);
    let vin = DemoTransport::vin();
    let masked = format!("{}{}", &vin[..17 - VIN_HIDDEN], "0".repeat(VIN_HIDDEN));
    assert_eq!(shared.preview.vins, vec![masked.clone()]);

    let replay = ReplayTransport::from_recording(
        parse_recording(&shared.file).unwrap(),
        "shared",
        ReplayPacing::Fast,
    );
    let mut elm = Elm327::new();
    elm.connect(Box::new(replay), "6")
        .expect("the shared file replays");
    assert!(elm.set_headers(true));
    assert_eq!(Obd2::new(&mut elm).read_vin(), masked);
}
