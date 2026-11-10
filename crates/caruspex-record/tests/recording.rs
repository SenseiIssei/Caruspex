// Recording and replay of an adapter session (tests/test_recording.cpp).
//
// The tests that read faults, the VIN and live values through Obd2 come with
// the protocols crate; here the same checks run on raw requests.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use caruspex_demo::{DemoAdapter, DemoClock, DemoOptions, DemoTransport};
use caruspex_i18n::{Language, set_language};
use caruspex_link::elm327::PROTOCOL_ORDER_KLINE_FIRST;
use caruspex_link::{AdaptiveTiming, Elm327, ElmResult, Transport};
use caruspex_record::{
    RECORDING_VERSION, RecordedDirection, Recording, RecordingMeta, RecordingTransport,
    ReplayPacing, ReplayTransport, active_recording, describe_recording, load_recording,
    parse_recording, recording_summary, start_recording, stop_recording,
};

// The active language is global and the tests of one file run in parallel:
// tests that look at texts hold this guard, which switches back to German.
struct LanguageGuard {
    _lock: MutexGuard<'static, ()>,
}

fn language(language: Language) -> LanguageGuard {
    static LOCK: Mutex<()> = Mutex::new(());
    let lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    set_language(language);
    LanguageGuard { _lock: lock }
}

impl Drop for LanguageGuard {
    fn drop(&mut self) {
        set_language(Language::German);
    }
}

fn temp_file(name: &str) -> String {
    let path: PathBuf = std::env::temp_dir().join(name);
    path.to_string_lossy().into_owned()
}

fn warm_demo(clock: &DemoClock, adapter: DemoAdapter) -> Box<DemoTransport> {
    Box::new(DemoTransport::new(DemoOptions {
        adapter,
        clock: Some(clock.source()),
        latency_scale: 0.0,
        ..DemoOptions::default()
    }))
}

// What a session reads, in a form that can be compared: the raw answers to
// a few requests.
fn read_everything(elm: &mut Elm327) -> Vec<ElmResult> {
    ["0100", "03", "07", "0902", "010C", "010D", "0105"]
        .iter()
        .map(|c| elm.request(c, u8::from_str_radix(&c[..2], 16).unwrap(), 6000))
        .collect()
}

// A transport that hands out fixed answers, for the byte-exact file test.
#[derive(Default)]
struct ScriptedTransport {
    answers: VecDeque<Vec<u8>>,
}

impl Transport for ScriptedTransport {
    fn write(&mut self, _data: &[u8]) -> bool {
        true
    }
    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        self.answers.pop_front().unwrap_or_default()
    }
    fn flush_input(&mut self) {}
    fn name(&self) -> String {
        "Scripted \u{e4}".to_string()
    }
}

fn replay_answer(replay: &mut ReplayTransport, command: &str) -> Vec<u8> {
    replay.flush_input();
    replay.write(format!("{command}\r").as_bytes());
    replay.read(50)
}

fn small_recording(body: &str) -> Recording {
    let text = format!(
        "{{\"format\":\"caruspex-recording\",\"version\":1,\"adapter\":\"ELM327 v1.4b\"}}\n{body}"
    );
    parse_recording(text).expect("small recording parses")
}

fn text_of(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn recording_and_replay_give_the_same_answers() {
    let _lang = language(Language::English);
    let path = temp_file("caruspex_rs_test_roundtrip.jsonl");
    let clock = DemoClock::new(600.0);

    let live;
    {
        let mut elm = Elm327::new();
        let recorder = RecordingTransport::new(
            warm_demo(&clock, DemoAdapter::CloneV15),
            &path,
            RecordingMeta::default(),
        );
        assert!(recorder.is_recording());
        // Protocol 5 first: the demo car refuses it, so the recording also
        // holds a failed attempt.
        elm.connect(recorder, "56").expect("connect");
        // Like the application: headers on, so every fault keeps its sender.
        assert!(elm.set_headers(true));
        live = read_everything(&mut elm);
        let active = active_recording(&elm).expect("recording is active");
        assert!(active.exchanges() > 20);
        elm.disconnect();
    }
    assert!(live[0].ok());
    assert!(live[3].ok());
    assert!(live[4].ok());

    // The demo car moves on; the replay must not care.
    clock.set(2000.0);
    let replay = ReplayTransport::new(&path, ReplayPacing::Fast);
    assert!(replay.is_open());
    assert_eq!(replay.name(), "Recording: caruspex_rs_test_roundtrip.jsonl");

    let info = replay.info();
    assert_eq!(info.adapter, "ELM327 v1.5");
    assert!(info.protocol.contains("15765-4"));
    assert_eq!(info.protocol_id, "6");
    assert!(info.exchanges > 20);
    assert_eq!(info.skipped_lines, 0);
    assert!(!info.truncated);
    assert_eq!(info.transport, "Demo vehicle (built-in)");
    assert!(recording_summary(&info).contains("Adapter: ELM327 v1.5"));

    let mut elm = Elm327::new();
    // The same protocol order as when recording: every command matches.
    elm.connect(Box::new(replay), "56").expect("connect replay");
    assert_eq!(elm.version(), "ELM327 v1.5");
    assert!(elm.is_can());
    assert!(elm.set_headers(true));
    let again = read_everything(&mut elm);
    assert_eq!(again, live);
    elm.disconnect();

    // A different order goes straight to the recorded protocol.
    let second = ReplayTransport::new(&path, ReplayPacing::Fast);
    assert_eq!(second.protocol_order(PROTOCOL_ORDER_KLINE_FIRST), "6");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_recording_started_in_a_running_session_replays_on_its_own() {
    let _lang = language(Language::English);
    let path = temp_file("caruspex_rs_test_midsession.jsonl");
    let clock = DemoClock::new(600.0);
    let live;
    {
        let mut elm = Elm327::new();
        elm.connect(warm_demo(&clock, DemoAdapter::CloneV15), "6")
            .expect("connect");
        assert!(elm.set_headers(true));
        let handle = start_recording(&mut elm, &path).expect("recording starts");
        assert!(handle.is_recording());
        // Only one at a time.
        assert!(start_recording(&mut elm, &path).is_err());
        // Restored after the preamble.
        assert!(elm.headers_on());
        live = read_everything(&mut elm);
        assert!(stop_recording(&mut elm));
        assert!(!stop_recording(&mut elm));
        assert!(!handle.is_recording());
        assert_eq!(elm.transport_name(), "Demo vehicle (built-in)");
        // The session goes on unrecorded.
        assert!(elm.request("0902", 0x09, 6000).ok());
    }
    assert!(live[0].ok());

    let rec = load_recording(&path).expect("loads");
    assert_eq!(rec.protocol_id, "6");
    assert_eq!(rec.adapter, "ELM327 v1.5");

    // Connect walks the K-line protocols first: the replay refuses them the
    // way a CAN car would, until protocol 6 comes up.
    for order in ["6", "56"] {
        let replay = ReplayTransport::new(&path, ReplayPacing::Fast);
        let mut elm = Elm327::new();
        elm.connect(Box::new(replay), order)
            .expect("connect replay");
        assert_eq!(elm.protocol_id(), '6');
        assert_eq!(elm.version(), "ELM327 v1.5");
        assert!(elm.set_headers(true));
        assert_eq!(read_everything(&mut elm), live);
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn recording_keeps_every_byte_and_the_header_text() {
    let path = temp_file("caruspex_rs_test_bytes.jsonl");
    let all_bytes: Vec<u8> = (0..=255u8).collect();
    {
        let scripted = ScriptedTransport {
            answers: VecDeque::from([all_bytes.clone(), b"\"quoted\" \\ back\r\n>".to_vec()]),
        };
        let meta = RecordingMeta {
            adapter: "Adapter \u{e4}\u{f6}\u{fc}".to_string(),
            ..RecordingMeta::default()
        };
        let mut rec = RecordingTransport::new(Box::new(scripted), &path, meta);
        assert!(rec.is_recording());
        assert_eq!(rec.name(), "Scripted \u{e4}");
        rec.write(b"01 0c\r");
        assert_eq!(rec.read(10), all_bytes);
        assert_eq!(rec.read(10).len(), 18);
        // Empty reads are not written down.
        assert!(rec.read(10).is_empty());
        rec.note("a note");
    }
    let loaded = load_recording(&path).expect("loads");
    assert_eq!(loaded.version, RECORDING_VERSION);
    assert_eq!(loaded.adapter, "Adapter \u{e4}\u{f6}\u{fc}");
    assert_eq!(loaded.transport, "Scripted \u{e4}");
    assert_eq!(loaded.created.len(), 20);
    assert_eq!(loaded.events.len(), 4);
    assert_eq!(loaded.events[0].direction, RecordedDirection::Tx);
    assert_eq!(loaded.events[0].data, b"01 0c\r");
    assert_eq!(loaded.events[1].data, all_bytes);
    assert_eq!(loaded.events[2].data, b"\"quoted\" \\ back\r\n>");
    assert_eq!(loaded.events[3].direction, RecordedDirection::Note);

    // The file itself: header field order and the raw-byte escapes of the
    // C++ version.
    let text = String::from_utf8(std::fs::read(&path).unwrap()).expect("file is UTF-8");
    let mut lines = text.lines();
    let header = lines.next().unwrap();
    assert!(header.starts_with("{\"format\":\"caruspex-recording\",\"version\":1,\"created\":\""));
    assert!(header.ends_with(
        "\",\"transport\":\"Scripted \u{e4}\",\"adapter\":\"Adapter \u{e4}\u{f6}\u{fc}\"}"
    ));
    let first = lines.next().unwrap();
    assert!(first.starts_with("{\"t\":0."));
    assert!(first.ends_with(",\"dir\":\"tx\",\"data\":\"01 0c\\r\"}"));
    let second = lines.next().unwrap();
    assert!(second.contains(",\"dir\":\"rx\",\"data\":\"\\u0000\\u0001"));
    assert!(second.contains("\\u007f\\u0080\\u0081"));
    assert!(second.ends_with("\\u00fe\\u00ff\"}"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn malformed_and_cut_off_lines_are_skipped() {
    let text = "{\"format\":\"caruspex-recording\",\"version\":1,\"created\":\"2026-11-09T10:00:00Z\"}\r\n\
        {\"t\":0.100,\"dir\":\"tx\",\"data\":\"010C\\r\"}\r\n\
        this is not json\n\
        {\"t\":0.110,\"dir\":\"rx\"}\n\
        {\"t\":0.120,\"dir\":\"sideways\",\"data\":\"x\"}\n\
        {\"t\":0.130,\"dir\":\"rx\",\"data\":[1,2]}\n\
        \n\
        {\"t\":0.140,\"dir\":\"rx\",\"data\":\"410C1AF8\\r\\r>\"}\n\
        {\"t\":1.5,\"dir\":\"rx\",\"data\":\"41 0C";
    let rec = parse_recording(text).expect("header is fine");
    assert_eq!(rec.events.len(), 2);
    assert_eq!(rec.skipped_lines, 4);
    assert!(rec.truncated);
    assert_eq!(rec.created, "2026-11-09T10:00:00Z");

    let info = describe_recording(&rec);
    assert_eq!(info.exchanges, 1);
    assert_eq!(info.skipped_lines, 4);
    assert!(info.truncated);
    assert!(info.duration_seconds > 0.13);

    let mut replay = ReplayTransport::from_recording(rec, "test", ReplayPacing::Fast);
    assert_eq!(
        text_of(&replay_answer(&mut replay, "010C")),
        "410C1AF8\r\r>"
    );

    // Not a recording at all.
    let error = parse_recording("{\"t\":0.1,\"dir\":\"tx\",\"data\":\"ATZ\"}\n").unwrap_err();
    assert!(!error.is_empty());
    assert!(parse_recording("").is_err());
    // No version.
    assert!(parse_recording("{\"format\":\"caruspex-recording\"}\n").is_err());
    assert!(load_recording(&temp_file("caruspex_rs_test_missing_file.jsonl")).is_err());
}

#[test]
fn commands_that_were_never_recorded_get_elm327_answers() {
    let _lang = language(Language::German);
    let mut replay = ReplayTransport::from_recording(
        small_recording(
            "{\"t\":0.1,\"dir\":\"tx\",\"data\":\"0100\\r\"}\n\
             {\"t\":0.2,\"dir\":\"rx\",\"data\":\"4100BE3FA813\\r\\r>\"}\n",
        ),
        "small.jsonl",
        ReplayPacing::Fast,
    );
    assert_eq!(replay.name(), "Aufnahme: small.jsonl");
    assert_eq!(replay_answer(&mut replay, "ATXYZ"), b"?\r\r>");
    assert_eq!(replay_answer(&mut replay, "STI"), b"?\r\r>");
    assert_eq!(replay_answer(&mut replay, "0902"), b"NO DATA\r\r>");
    assert_eq!(replay_answer(&mut replay, "ATZ"), b"\r\rELM327 v1.4b\r\r>");
    assert_eq!(replay_answer(&mut replay, "ATI"), b"ELM327 v1.4b\r\r>");
    assert_eq!(replay_answer(&mut replay, "ATSP6"), b"OK\r\r>");
    assert_eq!(replay_answer(&mut replay, "01 00"), b"4100BE3FA813\r\r>");
    assert_eq!(replay.matched(), 1);
    assert_eq!(replay.unmatched(), 6);
}

#[test]
fn repeating_live_polls_wrap_around_to_the_start() {
    let mut replay = ReplayTransport::from_recording(
        small_recording(
            "{\"t\":0.0,\"dir\":\"tx\",\"data\":\"ATZ\\r\"}\n\
             {\"t\":0.1,\"dir\":\"rx\",\"data\":\"ELM327 v1.4b\\r\\r>\"}\n\
             {\"t\":1.0,\"dir\":\"tx\",\"data\":\"010C\\r\"}\n\
             {\"t\":1.1,\"dir\":\"rx\",\"data\":\"410C0FA0\\r\\r>\"}\n\
             {\"t\":1.2,\"dir\":\"tx\",\"data\":\"010D\\r\"}\n\
             {\"t\":1.3,\"dir\":\"rx\",\"data\":\"410D10\\r\\r>\"}\n\
             {\"t\":2.0,\"dir\":\"tx\",\"data\":\"010C\\r\"}\n\
             {\"t\":2.1,\"dir\":\"rx\",\"data\":\"410C\"}\n\
             {\"t\":2.15,\"dir\":\"rx\",\"data\":\"1F40\\r\\r>\"}\n\
             {\"t\":2.2,\"dir\":\"tx\",\"data\":\"010D\\r\"}\n\
             {\"t\":2.3,\"dir\":\"rx\",\"data\":\"410D20\\r\\r>\"}\n",
        ),
        "polls",
        ReplayPacing::Fast,
    );
    let mut rpm = Vec::new();
    let mut speed = Vec::new();
    for _ in 0..5 {
        rpm.push(text_of(&replay_answer(&mut replay, "01 0c")));
        speed.push(text_of(&replay_answer(&mut replay, "010D")));
    }
    assert_eq!(
        rpm,
        [
            "410C0FA0\r\r>",
            "410C1F40\r\r>",
            "410C0FA0\r\r>",
            "410C1F40\r\r>",
            "410C0FA0\r\r>"
        ]
    );
    assert_eq!(
        speed,
        [
            "410D10\r\r>",
            "410D20\r\r>",
            "410D10\r\r>",
            "410D20\r\r>",
            "410D10\r\r>"
        ]
    );
    assert_eq!(replay.wrapped(), 2);
    assert_eq!(replay.unmatched(), 0);
}

#[test]
fn real_time_replay_honours_the_recorded_delay() {
    // A whole second between question and answer: a short read must come
    // back empty even when a busy build machine oversleeps a lot.
    let mut replay = ReplayTransport::from_recording(
        small_recording(
            "{\"t\":1.000,\"dir\":\"tx\",\"data\":\"0100\\r\"}\n\
             {\"t\":2.000,\"dir\":\"rx\",\"data\":\"4100BE3FA813\\r\\r>\"}\n",
        ),
        "paced",
        ReplayPacing::RealTime,
    );
    replay.write(b"0100\r");
    // Not there yet.
    assert!(replay.read(20).is_empty());
    // Arrives later.
    assert_eq!(replay.read(3000), b"4100BE3FA813\r\r>");
}

#[test]
fn replay_of_a_session_recorded_with_atat1_answers_atat2_with_a_question_mark() {
    // A recording made with standard timing, replayed by a driver that asks
    // for ATAT2: the replay answers "?" like an adapter without it, and the
    // driver stays with ATAT1.
    let path = temp_file("caruspex_rs_test_timing.jsonl");
    let clock = DemoClock::new(600.0);
    {
        let mut elm = Elm327::new();
        let recorder = RecordingTransport::new(
            warm_demo(&clock, DemoAdapter::CloneV15),
            &path,
            RecordingMeta::default(),
        );
        elm.connect(recorder, "6").expect("connect");
        elm.disconnect();
    }
    let mut elm = Elm327::new();
    elm.set_adaptive_timing(AdaptiveTiming::Aggressive);
    let replay = ReplayTransport::new(&path, ReplayPacing::Fast);
    elm.connect(Box::new(replay), "6").expect("connect replay");
    assert_eq!(elm.adaptive_timing_level(), 1);
    elm.disconnect();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn an_unreadable_path_leaves_a_passing_through_wrapper() {
    let clock = DemoClock::new(600.0);
    let path = std::env::temp_dir()
        .join("caruspex_rs_no_such_dir")
        .join("x.jsonl");
    let mut rec = RecordingTransport::new(
        warm_demo(&clock, DemoAdapter::CloneV15),
        &path.to_string_lossy(),
        RecordingMeta::default(),
    );
    assert!(!rec.is_recording());
    assert!(!rec.error().is_empty());
    assert!(rec.write(b"ATZ\r"));
    let inner = rec.release().expect("transport is handed back");
    assert_eq!(inner.name(), "Demo vehicle (built-in)");
}
