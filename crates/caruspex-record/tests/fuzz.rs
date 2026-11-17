//! Fuzz-style property tests of the file parsers: recordings, Torque CSV
//! with its equations, and the replay on top of a recording. Random and
//! half-broken input must never panic, and what can be written must read
//! back. Normal `cargo test`, stable toolchain; PROPTEST_CASES raises the
//! number of cases.

use caruspex_link::Transport;
use caruspex_record::{
    CustomPid, RecordedDirection, RecordedEvent, Recording, ReplayPacing, ReplayTransport,
    TorqueEquation, describe_recording, parse_recording, parse_request_hex, parse_torque_csv,
    recording_summary, share_recording, write_recording, write_torque_csv,
};
use proptest::prelude::*;

/// A line that looks a bit like a recording line, or not at all.
fn recording_line() -> impl Strategy<Value = String> {
    prop_oneof![
        r#"\{"t":[0-9.e-]{1,8},"dir":"(tx|rx|note)","data":"[ -~]{0,20}"\}"#,
        r#"\{"t":[0-9]{1,3}\.[0-9]{3},"dir":"rx","data":"(\\u00[0-9a-f]{2}|\\r|\\n|\\"|[0-9A-F ]){0,20}"\}"#,
        r#"\{"[a-z]{1,5}":(true|false|null|-?[0-9]{1,4}|"[^"\\]{0,6}")(,"[a-z]{1,5}":[0-9]{1,3})?\}"#,
        r#"\{"t":1,"data":"\\ud8[0-9a-f]{2}\\u[0-9a-f]{4}","dir":"rx"\}"#,
        ".{0,40}",
    ]
}

fn header() -> &'static str {
    r#"{"format":"caruspex-recording","version":1,"created":"2026-11-09T18:02:11Z","transport":"COM5"}"#
}

fn csv_line() -> impl Strategy<Value = String> {
    prop_oneof![
        "[A-Za-z ]{0,8},[A-Za-z]{0,4},(01|22|21)[0-9A-F]{2,4},[A-Z()*/+0-9. -]{0,20},-?[0-9]{1,4},[0-9]{1,5},[a-z%°]{0,3}(,7E[0-9A-F])?",
        "\"[^\"]{0,10}\",[^,\n]{0,10},[^,\n]{0,10},[^,\n]{0,24}",
        ".{0,50}",
    ]
}

fn equation() -> impl Strategy<Value = String> {
    prop_oneof![
        "[A-H()*/+\\-0-9.<>{}:,^ ]{0,24}",
        "(A|B|C|AA|AB)(\\*|/|\\+|-)(256|0|1|-1|B|0.5)",
        "\\{[A-D]:[0-9]{1,2}\\}",
        "SIGNED\\([A-D]\\)\\*[0-9]{1,3}",
        "[A-Z]{1,3}<<[0-9]{1,3}",
        ".{0,20}",
    ]
}

proptest! {
    #[test]
    fn any_text_reads_or_fails_cleanly(lines in prop::collection::vec(recording_line(), 0..12), with_header in any::<bool>()) {
        let mut text = String::new();
        if with_header {
            text.push_str(header());
            text.push('\n');
        }
        text.push_str(&lines.join("\n"));
        if let Ok(recording) = parse_recording(&text) {
            let info = describe_recording(&recording);
            prop_assert!(info.duration_seconds >= 0.0 || info.duration_seconds.is_nan() || recording.events.iter().any(|e| e.t < 0.0));
            let _ = recording_summary(&info);
        }
    }

    #[test]
    fn any_bytes_read_or_fail_cleanly(bytes in prop::collection::vec(any::<u8>(), 0..400)) {
        let mut text = header().as_bytes().to_vec();
        text.push(b'\n');
        text.extend(bytes);
        let recording = parse_recording(&text).expect("the header is there");
        let _ = describe_recording(&recording);
    }

    #[test]
    fn a_replay_answers_any_command(
        lines in prop::collection::vec(recording_line(), 0..12),
        commands in prop::collection::vec("[A-Z0-9 ]{0,10}", 0..10),
    ) {
        let text = format!("{}\n{}", header(), lines.join("\n"));
        let Ok(recording) = parse_recording(&text) else { return Ok(()); };
        let mut replay = ReplayTransport::from_recording(recording, "fuzz", ReplayPacing::Fast);
        for c in commands {
            replay.write(format!("{c}\r").as_bytes());
            let _ = replay.read(5);
        }
    }

    #[test]
    fn sharing_any_recording_keeps_it_readable(
        lines in prop::collection::vec(recording_line(), 0..12),
        vin in "[A-HJ-NPR-Z0-9]{17}",
    ) {
        let text = format!("{}
{}", header(), lines.join("
"));
        let Ok(recording) = parse_recording(&text) else { return Ok(()); };
        let shared = share_recording(&recording, &[&vin]);
        let again = parse_recording(&shared.file).unwrap();
        prop_assert_eq!(again.events.len(), recording.events.len());
        prop_assert!(again.transport.is_empty());
        // Written and read again, the original keeps its traffic (times
        // to the millisecond, never below zero).
        let round = parse_recording(write_recording(&recording)).unwrap();
        prop_assert_eq!(round.events.len(), recording.events.len());
        for (a, b) in round.events.iter().zip(&recording.events) {
            prop_assert_eq!(a.direction, b.direction);
            prop_assert_eq!(&a.data, &b.data);
            if (0.0..1e9).contains(&b.t) {
                prop_assert!((a.t - b.t).abs() < 0.001);
            }
        }
    }

    #[test]
    fn torque_csv_never_panics(lines in prop::collection::vec(csv_line(), 0..10)) {
        let (pids, _errors) = parse_torque_csv(&lines.join("\n"), 9000);
        // What was read can be written and read again.
        let again = parse_torque_csv(&write_torque_csv(&pids), 9000).0;
        prop_assert_eq!(again.len(), pids.len());
        for (a, b) in again.iter().zip(&pids) {
            prop_assert_eq!(&a.request, &b.request);
            prop_assert_eq!(&a.equation, &b.equation);
        }
    }

    #[test]
    fn equations_never_panic(text in equation(), data in prop::collection::vec(any::<u8>(), 0..12)) {
        let eq = TorqueEquation::new(&text);
        if eq.ok()
            && let Ok(value) = eq.evaluate(&data)
        {
            prop_assert!(!value.is_nan() || data.len() < eq.bytes_needed());
        }
        let _ = parse_request_hex(&text);
    }

    #[test]
    fn a_written_recording_reads_back(events in prop::collection::vec((0u32..100_000, 0usize..3, prop::collection::vec(any::<u8>(), 0..30)), 0..20)) {
        // The writer is the recording transport; the same escaping by hand
        // here keeps the test independent of a file.
        let mut text = String::from(header());
        text.push('\n');
        let mut expected = Vec::new();
        for (ms, dir, data) in &events {
            let dir_name = ["tx", "rx", "note"][*dir];
            let mut escaped = String::new();
            for &c in data {
                match c {
                    b'"' => escaped.push_str("\\\""),
                    b'\\' => escaped.push_str("\\\\"),
                    0x20..=0x7E => escaped.push(c as char),
                    _ => escaped.push_str(&format!("\\u00{c:02x}")),
                }
            }
            text.push_str(&format!("{{\"t\":{}.{:03},\"dir\":\"{dir_name}\",\"data\":\"{escaped}\"}}\n", ms / 1000, ms % 1000));
            expected.push(RecordedEvent {
                t: f64::from(*ms) / 1000.0,
                direction: [RecordedDirection::Tx, RecordedDirection::Rx, RecordedDirection::Note][*dir],
                data: data.clone(),
            });
        }
        let recording: Recording = parse_recording(&text).unwrap();
        prop_assert_eq!(recording.skipped_lines, 0);
        prop_assert_eq!(recording.events.len(), expected.len());
        for (got, want) in recording.events.iter().zip(&expected) {
            prop_assert_eq!(got.direction, want.direction);
            prop_assert_eq!(&got.data, &want.data);
            prop_assert!((got.t - want.t).abs() < 1e-6);
        }
    }
}

#[test]
fn csv_of_the_default_pid_reads_back() {
    let pid = CustomPid {
        name: "Boost".into(),
        request: "220101".into(),
        equation: "A*256+B".into(),
        ..CustomPid::default()
    };
    let (again, errors) = parse_torque_csv(&write_torque_csv(&[pid]), 9000);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(again[0].equation, "A*256+B");
}
