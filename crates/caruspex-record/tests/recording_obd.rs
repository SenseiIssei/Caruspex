// The recording tests of tests/test_recording.cpp in their original form, with
// Obd2: faults, VIN and live values read through the protocol layer. (The
// tests of recording.rs run on raw requests, because the protocols crate
// came later.)

mod common;

use caruspex_data::pids::default_live_pids;
use caruspex_demo::{DemoAdapter, DemoClock, DemoOptions, DemoTransport};
use caruspex_i18n::Language;
use caruspex_link::elm327::PROTOCOL_ORDER_KLINE_FIRST;
use caruspex_link::{AdaptiveTiming, Elm327, Transport};
use caruspex_protocols::Obd2;
use caruspex_record::{
    RecordingMeta, RecordingTransport, ReplayPacing, ReplayTransport, active_recording,
    load_recording, recording_summary, start_recording, stop_recording,
};

use common::{InterceptTransport, capture_log, language, sent_commands};

fn temp_file(name: &str) -> String {
    std::env::temp_dir()
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn warm_demo(clock: &DemoClock, adapter: DemoAdapter) -> Box<DemoTransport> {
    Box::new(DemoTransport::new(DemoOptions {
        adapter,
        clock: Some(clock.source()),
        latency_scale: 0.0,
        ..DemoOptions::default()
    }))
}

// What a session reads, in a form that can be compared.
#[derive(Debug, PartialEq)]
struct Readout {
    faults: Vec<String>,
    vin: String,
    live: Vec<String>,
    protocol: String,
}

fn read_everything(elm: &mut Elm327) -> Readout {
    let protocol = elm.protocol_text().to_string();
    let mut obd = Obd2::new(elm);
    let faults = obd
        .read_faults()
        .iter()
        .map(|f| format!("{}|{}|{}", f.code, f.kind, f.ecu))
        .collect();
    let vin = obd.read_vin();
    let live = obd
        .read_live_values(default_live_pids())
        .iter()
        .map(|v| format!("{}={:.6}{}", v.key, v.value, v.text))
        .collect();
    Readout {
        faults,
        vin,
        live,
        protocol,
    }
}

#[test]
fn recording_and_replay_give_the_same_faults_vin_and_live_values() {
    let _lang = language(Language::English);
    let path = temp_file("caruspex_rs_test_roundtrip_obd.jsonl");
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
    assert!(live.faults.len() >= 2);
    assert_eq!(live.vin, DemoTransport::vin());
    assert!(live.live.len() >= 5);

    // The demo car moves on; the replay must not care.
    clock.set(2000.0);
    let replay = ReplayTransport::new(&path, ReplayPacing::Fast);
    assert!(replay.is_open());
    assert_eq!(
        replay.name(),
        "Recording: caruspex_rs_test_roundtrip_obd.jsonl"
    );

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
    assert_eq!(again.faults, live.faults);
    assert_eq!(again.vin, live.vin);
    assert_eq!(again.live, live.live);
    assert_eq!(again.protocol, live.protocol);
    elm.disconnect();

    // A different order goes straight to the recorded protocol.
    let second = ReplayTransport::new(&path, ReplayPacing::Fast);
    assert_eq!(second.protocol_order(PROTOCOL_ORDER_KLINE_FIRST), "6");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_recording_started_in_a_running_session_replays_on_its_own_with_obd() {
    let _lang = language(Language::English);
    let path = temp_file("caruspex_rs_test_midsession_obd.jsonl");
    let clock = DemoClock::new(600.0);
    let faults: Vec<String>;
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
        faults = Obd2::new(&mut elm)
            .read_faults()
            .iter()
            .map(|f| format!("{}|{}", f.code, f.ecu))
            .collect();
        assert!(stop_recording(&mut elm));
        assert!(!stop_recording(&mut elm));
        assert_eq!(elm.transport_name(), "Demo vehicle (built-in)");
        // The session goes on unrecorded.
        assert_eq!(Obd2::new(&mut elm).read_vin(), DemoTransport::vin());
    }
    assert!(faults.len() >= 2);

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
        let again: Vec<String> = Obd2::new(&mut elm)
            .read_faults()
            .iter()
            .map(|f| format!("{}|{}", f.code, f.ecu))
            .collect();
        assert_eq!(again, faults);
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn adaptive_timing_refused_atat2_falls_back_to_atat1_and_the_session_works() {
    let _lang = language(Language::English);
    let clock = DemoClock::new(600.0);
    let mut elm = Elm327::new();
    elm.set_verbose(true);
    let log = capture_log(&mut elm);
    elm.set_adaptive_timing(AdaptiveTiming::Aggressive);
    let inner: Box<dyn Transport> = warm_demo(&clock, DemoAdapter::CloneV15);
    let intercepting =
        InterceptTransport::new(inner, |c: &str| (c == "ATAT2").then(|| b"?\r\r>".to_vec()));
    elm.connect(Box::new(intercepting), "6").expect("connect");
    assert_eq!(elm.adaptive_timing_level(), 1);
    let sent = sent_commands(&log.lock().unwrap());
    assert_eq!(sent.last().map(String::as_str), Some("ATAT1"));
    let told = log
        .lock()
        .unwrap()
        .iter()
        .any(|l| l.contains("ATAT2 refused"));
    assert!(told);
    // The session works as before.
    assert_eq!(Obd2::new(&mut elm).read_vin(), DemoTransport::vin());
}
