// Live values read back out of a recording without replaying it: the same
// numbers the live view decoded while the recording ran.

mod common;

use caruspex_data::pids::default_live_pids;
use caruspex_demo::{DemoAdapter, DemoClock, DemoOptions, DemoTransport};
use caruspex_i18n::Language;
use caruspex_link::Elm327;
use caruspex_protocols::Obd2;
use caruspex_record::{
    RecordingMeta, RecordingTransport, live_frames, load_recording, parse_recording,
};

use common::language;

fn temp_file(name: &str) -> String {
    std::env::temp_dir()
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn record_live(name: &str, headers: bool) -> (Vec<(i32, f64)>, caruspex_record::Recording) {
    let _lang = language(Language::English);
    let path = temp_file(name);
    let clock = DemoClock::new(600.0);
    let read;
    {
        let mut elm = Elm327::new();
        let demo = Box::new(DemoTransport::new(DemoOptions {
            adapter: DemoAdapter::CloneV15,
            clock: Some(clock.source()),
            latency_scale: 0.0,
            ..DemoOptions::default()
        }));
        let recorder = RecordingTransport::new(demo, &path, RecordingMeta::default());
        elm.connect(recorder, "6").expect("connect");
        assert!(elm.set_headers(headers));
        read = Obd2::new(&mut elm)
            .read_live_values(default_live_pids())
            .iter()
            .map(|v| (v.key, v.value))
            .collect::<Vec<_>>();
        elm.disconnect();
    }
    let recording = load_recording(&path).expect("loads");
    let _ = std::fs::remove_file(&path);
    (read, recording)
}

fn check_same(read: &[(i32, f64)], recording: &caruspex_record::Recording) {
    assert!(
        read.iter().any(|(key, _)| *key == 0x0C),
        "the demo car reports rpm"
    );
    let frames = live_frames(recording);
    let last = frames.last().expect("at least one frame");
    for (key, value) in read {
        let decoded = last.values.get(key).copied();
        assert_eq!(decoded, Some(*value), "value {key:#x}");
    }
}

#[test]
fn values_from_a_recording_match_the_live_view_with_headers_off() {
    let (read, recording) = record_live("caruspex_rs_test_live_log_plain.jsonl", false);
    check_same(&read, &recording);
}

#[test]
fn values_from_a_recording_match_the_live_view_with_headers_on() {
    let (read, recording) = record_live("caruspex_rs_test_live_log_headers.jsonl", true);
    check_same(&read, &recording);
}

#[test]
fn echo_spaces_noise_and_multi_frame_answers_are_read() {
    let text = concat!(
        "{\"format\":\"caruspex-recording\",\"version\":1,\"created\":\"2026-11-20T10:00:00Z\",\"transport\":\"test\"}\n",
        // Echo on, spaces on, the search message first.
        "{\"t\":1.000,\"dir\":\"tx\",\"data\":\"01 0C\\r\"}\n",
        "{\"t\":1.010,\"dir\":\"rx\",\"data\":\"010C\\rSEARCHING...\\r41 0C 1A F8 \\r\\r>\"}\n",
        // A combined request answered in three frames (headers off).
        "{\"t\":1.100,\"dir\":\"tx\",\"data\":\"010B0F0E0C\\r\"}\n",
        "{\"t\":1.130,\"dir\":\"rx\",\"data\":\"00B\\r0:410BB40F470E\\r1:940C2EE0\\r\\r>\"}\n",
        // Not a mode 01 request: ignored.
        "{\"t\":1.200,\"dir\":\"tx\",\"data\":\"ATRV\\r\"}\n",
        "{\"t\":1.210,\"dir\":\"rx\",\"data\":\"12.6V\\r\\r>\"}\n",
        // No answer.
        "{\"t\":1.300,\"dir\":\"tx\",\"data\":\"0110\\r\"}\n",
        "{\"t\":1.330,\"dir\":\"rx\",\"data\":\"NO DATA\\r\\r>\"}\n",
    );
    let recording = parse_recording(text).expect("parses");
    let frames = live_frames(&recording);
    assert_eq!(frames.len(), 2);
    // 0x1AF8 / 4 = 1726 rpm.
    assert_eq!(frames[0].values.get(&0x0C), Some(&1726.0));
    let second = &frames[1];
    // 0x2EE0 / 4 = 3000 rpm, 0xB4 = 180 kPa, 0x47 - 40 = 31 °C, 0x94 / 2 - 64 = 10 degrees.
    assert_eq!(second.values.get(&0x0C), Some(&3000.0));
    assert_eq!(second.values.get(&0x0B), Some(&180.0));
    assert_eq!(second.values.get(&0x0F), Some(&31.0));
    assert_eq!(second.values.get(&0x0E), Some(&10.0));
    assert!(!second.values.contains_key(&0x10));
}

#[test]
fn old_values_do_not_travel_into_later_frames() {
    let text = concat!(
        "{\"format\":\"caruspex-recording\",\"version\":1,\"created\":\"2026-11-20T10:00:00Z\",\"transport\":\"test\"}\n",
        "{\"t\":1.000,\"dir\":\"tx\",\"data\":\"010B0C\\r\"}\n",
        "{\"t\":1.020,\"dir\":\"rx\",\"data\":\"410BB40C2EE0\\r\\r>\"}\n",
        "{\"t\":9.000,\"dir\":\"tx\",\"data\":\"010C\\r\"}\n",
        "{\"t\":9.020,\"dir\":\"rx\",\"data\":\"410C2EE0\\r\\r>\"}\n",
    );
    let frames = live_frames(&parse_recording(text).expect("parses"));
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].values.get(&0x0B), Some(&180.0));
    assert!(!frames[1].values.contains_key(&0x0B));
}
