// Live values read back out of a recording without replaying it: the same
// numbers the live view decoded while the recording ran.

mod common;

use std::time::Instant;

use caruspex_data::pids::default_live_pids;
use caruspex_demo::{DemoAdapter, DemoClock, DemoOptions, DemoTransport};
use caruspex_i18n::Language;
use caruspex_link::{Elm327, Transport, hex_to_bytes};
use caruspex_protocols::Obd2;
use caruspex_record::{
    Recording, RecordingMeta, RecordingTransport, live_frames, load_recording, parse_recording,
};

use common::language;

fn temp_file(name: &str) -> String {
    std::env::temp_dir()
        .join(name)
        .to_string_lossy()
        .into_owned()
}

// ---------------------------------------------------------------------------
// The demo car only speaks CAN 11/500. OtherBus sits between the driver and
// the demo and prints the demo's answers the way an ELM327 prints them on
// another bus with headers on: with 29-bit identifiers (ISO 15765-4, engine
// 18DAF110, transmission 18DAF118), or with the header bytes and the
// checksum of ISO 9141-2 (48 6B) or ISO 14230 (8x F1) on the K-line.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum Bus {
    Can29,
    Iso9141,
    Kwp,
}

impl Bus {
    fn protocol(self) -> &'static str {
        match self {
            Bus::Can29 => "7",
            Bus::Iso9141 => "3",
            Bus::Kwp => "5",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Bus::Can29 => "ISO 15765-4 (CAN 29/500)",
            Bus::Iso9141 => "ISO 9141-2",
            Bus::Kwp => "ISO 14230-4 (KWP FAST)",
        }
    }
}

struct OtherBus {
    demo: DemoTransport,
    bus: Bus,
    headers: bool,
    pending: Vec<u8>,
}

/// Sum of the bytes modulo 256, the K-line checksum.
fn sum_of(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |sum, &b| sum.wrapping_add(b))
}

impl OtherBus {
    fn ask_demo(&mut self, line: &[u8]) -> String {
        self.demo.write(line);
        let mut reply = Vec::new();
        for _ in 0..100 {
            reply.extend(self.demo.read(50));
            if reply.contains(&b'>') {
                break;
            }
        }
        String::from_utf8_lossy(&reply).into_owned()
    }

    /// One line of the demo's answer as it would look on the other bus.
    fn convert(&self, line: &str) -> String {
        let digits: String = line.chars().filter(|c| *c != ' ').collect();
        let frame_line = self.headers
            && digits.len() % 2 == 1
            && digits.len() >= 5
            && digits.starts_with("7E")
            && digits.chars().all(|c| c.is_ascii_hexdigit());
        if !frame_line {
            return line.to_string();
        }
        let (id, rest) = digits.split_at(3);
        let frame = hex_to_bytes(rest);
        let source = if id == "7E9" { 0x18 } else { 0x10 };
        let bytes = match self.bus {
            Bus::Can29 => {
                let mut out = vec![0x18, 0xDA, 0xF1, source];
                out.extend_from_slice(&frame);
                out
            }
            Bus::Iso9141 | Bus::Kwp => {
                // The K-line gets one PID per request, which always fits
                // into a single frame.
                assert_eq!(frame[0] >> 4, 0, "a single frame: {line}");
                let length = usize::from(frame[0] & 0x0F);
                let mut out = if self.bus == Bus::Iso9141 {
                    vec![0x48, 0x6B, source]
                } else {
                    vec![0x80 | length as u8, 0xF1, source]
                };
                out.extend_from_slice(&frame[1..1 + length]);
                out.push(sum_of(&out));
                out
            }
        };
        let gap = if line.contains(' ') { " " } else { "" };
        bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(gap)
    }
}

impl Transport for OtherBus {
    fn write(&mut self, data: &[u8]) -> bool {
        let command: String = data
            .iter()
            .filter(|c| !matches!(c, b'\r' | b'\n' | b' '))
            .map(|&c| (c as char).to_ascii_uppercase())
            .collect();
        let reply = if command.starts_with("ATSP") || command.starts_with("ATTP") {
            // The demo car only answers on protocol 6.
            self.ask_demo(b"ATSP6\r");
            "OK\r\r>".to_string()
        } else if command == "ATDP" {
            format!("{}\r\r>", self.bus.description())
        } else if command == "ATDPN" {
            format!("{}\r\r>", self.bus.protocol())
        } else {
            match command.as_str() {
                "ATH1" => self.headers = true,
                "ATH0" | "ATZ" | "ATD" | "ATWS" => self.headers = false,
                _ => {}
            }
            let reply = self.ask_demo(data);
            reply
                .split('\r')
                .map(|line| self.convert(line))
                .collect::<Vec<_>>()
                .join("\r")
        };
        self.pending.extend(reply.into_bytes());
        true
    }
    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
    fn flush_input(&mut self) {
        self.pending.clear();
        self.demo.flush_input();
    }
    fn name(&self) -> String {
        "demo car on another bus".into()
    }
}

/// How a recording is made: which adapter, which bus, headers on or off,
/// which PIDs the live view reads.
struct Setup<'a> {
    adapter: DemoAdapter,
    bus: Option<Bus>,
    headers: bool,
    pids: &'a [i32],
}

impl Default for Setup<'_> {
    fn default() -> Self {
        Setup {
            adapter: DemoAdapter::CloneV15,
            bus: None,
            headers: false,
            pids: default_live_pids(),
        }
    }
}

/// Two rounds of the live view on the demo car, recorded. Returns what the
/// live view read in the last round.
fn record_live_with(name: &str, setup: Setup) -> (Vec<(i32, f64)>, Recording) {
    let _lang = language(Language::English);
    let path = temp_file(name);
    let clock = DemoClock::new(600.0);
    let mut read = Vec::new();
    {
        let mut elm = Elm327::new();
        let demo = DemoTransport::new(DemoOptions {
            adapter: setup.adapter,
            clock: Some(clock.source()),
            latency_scale: 0.0,
            transmission_ecu: true,
            ..DemoOptions::default()
        });
        let (transport, protocol): (Box<dyn Transport>, &str) = match setup.bus {
            None => (Box::new(demo), "6"),
            Some(bus) => (
                Box::new(OtherBus {
                    demo,
                    bus,
                    headers: false,
                    pending: Vec::new(),
                }),
                bus.protocol(),
            ),
        };
        let recorder = RecordingTransport::new(transport, &path, RecordingMeta::default());
        elm.connect(recorder, protocol).expect("connect");
        assert!(elm.set_headers(setup.headers));
        let mut obd = Obd2::new(&mut elm);
        for _ in 0..2 {
            read = obd
                .read_live_values(setup.pids)
                .iter()
                .map(|v| (v.key, v.value))
                .collect::<Vec<_>>();
            clock.advance(1.0);
        }
        elm.disconnect();
    }
    let recording = load_recording(&path).expect("loads");
    let _ = std::fs::remove_file(&path);
    (read, recording)
}

fn record_live(name: &str, headers: bool) -> (Vec<(i32, f64)>, Recording) {
    record_live_with(
        name,
        Setup {
            headers,
            ..Setup::default()
        },
    )
}

fn check_same(read: &[(i32, f64)], recording: &Recording) {
    assert!(
        read.iter().any(|(key, _)| *key == 0x0C),
        "the demo car reports rpm"
    );
    let frames = live_frames(recording);
    assert_eq!(frames.len(), 2, "one frame per round");
    let last = frames.last().expect("at least one frame");
    for (key, value) in read {
        let decoded = last.values.get(key).copied();
        assert_eq!(decoded, Some(*value), "value {key:#x}");
    }
}

/// Everything the adapter answered, as one text.
fn answers(recording: &Recording) -> String {
    recording
        .events
        .iter()
        .map(|e| String::from_utf8_lossy(&e.data).into_owned())
        .collect::<Vec<_>>()
        .join("")
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

// The transmission answers PID A4 in the same combined request as the engine
// answers the rest, so two senders take part in one answer.
const PIDS_OF_TWO_UNITS: [i32; 7] = [0x0C, 0x0D, 0xA4, 0x05, 0x0F, 0x04, 0x11];

#[test]
fn values_from_a_recording_match_the_live_view_on_can_29_bit() {
    let (read, recording) = record_live_with(
        "caruspex_rs_test_live_log_can29.jsonl",
        Setup {
            bus: Some(Bus::Can29),
            headers: true,
            pids: &PIDS_OF_TWO_UNITS,
            ..Setup::default()
        },
    );
    let text = answers(&recording);
    assert!(text.contains("ATSP7"), "{text}");
    // A multi-frame answer of the engine and the transmission's answer to
    // the same request.
    let (_, after) = text
        .split_once("010C0DA4050F04")
        .expect("the combined request");
    let (answer, _) = after.split_once('>').expect("its answer");
    assert!(answer.contains("18DAF11010"), "{answer}");
    assert!(answer.contains("18DAF118"), "{answer}");
    assert!(read.iter().any(|(key, _)| *key & 0xFF == 0xA4));
    check_same(&read, &recording);

    let (read, recording) = record_live_with(
        "caruspex_rs_test_live_log_can29_default.jsonl",
        Setup {
            bus: Some(Bus::Can29),
            headers: true,
            ..Setup::default()
        },
    );
    check_same(&read, &recording);
}

#[test]
fn values_from_a_recording_match_the_live_view_on_the_k_line_with_headers() {
    for (bus, engine, transmission) in [
        (Bus::Iso9141, "486B10", "486B18"),
        (Bus::Kwp, "F110", "F118"),
    ] {
        let (read, recording) = record_live_with(
            &format!("caruspex_rs_test_live_log_kline_{bus:?}.jsonl"),
            Setup {
                bus: Some(bus),
                headers: true,
                pids: &PIDS_OF_TWO_UNITS,
                ..Setup::default()
            },
        );
        let text = answers(&recording);
        assert!(text.contains(&format!("ATSP{}", bus.protocol())), "{text}");
        assert!(text.contains(engine), "{bus:?}: {text}");
        assert!(text.contains(transmission), "{bus:?}: {text}");
        check_same(&read, &recording);
    }
}

#[test]
fn values_from_a_recording_match_the_live_view_with_batched_stn_commands() {
    let (read, recording) = record_live_with(
        "caruspex_rs_test_live_log_stn.jsonl",
        Setup {
            adapter: DemoAdapter::ObdLinkMxPlus,
            headers: true,
            ..Setup::default()
        },
    );
    let text = answers(&recording);
    assert!(text.contains("STBC1"), "{text}");
    assert!(text.contains('|'), "{text}");
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

// ---------------------------------------------------------------------------
// Recordings written by hand, for what the demo car cannot do.
// ---------------------------------------------------------------------------

const HEADER: &str = "{\"format\":\"caruspex-recording\",\"version\":1,\"created\":\"2026-11-20T10:00:00Z\",\"transport\":\"test\"}\n";

/// A command at `t` and its answer 10 ms later, as recording lines.
fn exchange(t: f64, command: &str, reply: &[&str]) -> String {
    let mut answer = String::new();
    for line in reply {
        answer.push_str(line);
        answer.push_str("\\r");
    }
    format!(
        "{{\"t\":{t:.3},\"dir\":\"tx\",\"data\":\"{command}\\r\"}}\n{{\"t\":{:.3},\"dir\":\"rx\",\"data\":\"{answer}\\r>\"}}\n",
        t + 0.01
    )
}

fn spaced(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// ISO 9141-2 with headers on: 48 6B, then the sender, the data and the
/// checksum.
fn iso9141(sender_and_data: &[u8]) -> String {
    let mut bytes = vec![0x48, 0x6B];
    bytes.extend_from_slice(sender_and_data);
    bytes.push(sum_of(&bytes));
    spaced(&bytes)
}

/// The same with a checksum one off.
fn iso9141_broken(sender_and_data: &[u8]) -> String {
    let mut bytes = vec![0x48, 0x6B];
    bytes.extend_from_slice(sender_and_data);
    bytes.push(sum_of(&bytes).wrapping_add(1));
    spaced(&bytes)
}

/// ISO 14230 with headers on: format byte with the length, tester F1, the
/// sender, the data and the checksum.
fn kwp(sender: u8, data: &[u8]) -> String {
    let mut bytes = vec![0x80 | data.len() as u8, 0xF1, sender];
    bytes.extend_from_slice(data);
    bytes.push(sum_of(&bytes));
    spaced(&bytes)
}

#[test]
fn a_k_line_answer_with_a_wrong_checksum_loses_its_value() {
    for with_dialogue in [true, false] {
        let mut text = String::from(HEADER);
        if with_dialogue {
            text += &exchange(0.5, "ATZ", &["ELM327 v1.5"]);
            text += &exchange(0.6, "ATSP3", &["OK"]);
            text += &exchange(0.7, "ATH1", &["OK"]);
        }
        // 0x1AF8 / 4 = 1726 rpm, 0x50 = 80 km/h.
        text += &exchange(1.0, "010C", &[&iso9141(&[0x10, 0x41, 0x0C, 0x1A, 0xF8])]);
        text += &exchange(1.1, "010D", &[&iso9141(&[0x10, 0x41, 0x0D, 0x50])]);
        // Broken: 4000 rpm and the engine's 90 km/h; the transmission's
        // 79 km/h after it is fine.
        text += &exchange(
            2.0,
            "010C",
            &[&iso9141_broken(&[0x10, 0x41, 0x0C, 0x3E, 0x80])],
        );
        text += &exchange(
            2.1,
            "010D",
            &[
                &iso9141_broken(&[0x10, 0x41, 0x0D, 0x5A]),
                &iso9141(&[0x18, 0x41, 0x0D, 0x4F]),
            ],
        );
        // 0x2EE0 / 4 = 3000 rpm.
        text += &exchange(3.0, "010C", &[&iso9141(&[0x10, 0x41, 0x0C, 0x2E, 0xE0])]);

        let frames = live_frames(&parse_recording(&text).expect("parses"));
        let rpm: Vec<f64> = frames.iter().map(|f| f.values[&0x0C]).collect();
        assert_eq!(rpm, vec![1726.0, 3000.0], "with dialogue: {with_dialogue}");
        assert_eq!(frames[0].values.get(&0x0D), Some(&79.0));
    }
}

#[test]
fn kwp_answers_of_a_recording_started_in_a_running_session_are_read() {
    // Started after connecting: the header knows the protocol, the dialogue
    // never shows ATH1.
    let mut text = String::from(
        "{\"format\":\"caruspex-recording\",\"version\":1,\"created\":\"2026-11-20T10:00:00Z\",\"transport\":\"test\",\"protocolId\":\"5\"}\n",
    );
    text += &exchange(1.0, "010C", &[&kwp(0x10, &[0x41, 0x0C, 0x1A, 0xF8])]);
    text += &exchange(1.1, "0105", &[&kwp(0x10, &[0x41, 0x05, 0x5A])]);
    text += &exchange(2.0, "010C", &[&kwp(0x10, &[0x41, 0x0C, 0x2E, 0xE0])]);
    let frames = live_frames(&parse_recording(&text).expect("parses"));
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].values.get(&0x0C), Some(&1726.0));
    // 0x5A - 40 = 50 °C.
    assert_eq!(frames[0].values.get(&0x05), Some(&50.0));
    assert_eq!(frames[1].values.get(&0x0C), Some(&3000.0));
}

#[test]
fn two_control_units_on_can_29_bit_keep_the_engine_value_and_their_own() {
    let mut text = String::from(HEADER);
    text += &exchange(0.5, "ATZ", &["ELM327 v1.5"]);
    text += &exchange(0.6, "ATSP7", &["OK"]);
    text += &exchange(0.7, "ATH1", &["OK"]);
    // Both answer the speed; the engine comes first, as on a real bus.
    text += &exchange(
        1.0,
        "010C0D",
        &[
            "18 DA F1 10 06 41 0C 1A F8 0D 50 AA",
            "18 DA F1 18 03 41 0D 4F AA AA AA AA",
        ],
    );
    // The engine's answer in three frames, the transmission's single frame
    // in between: each sender is put together on its own.
    text += &exchange(
        2.0,
        "010C0D0B0F1011",
        &[
            "18DAF110100F410C2EE00D51",
            "18DAF11803410D4FAAAAAAAA",
            "18DAF110210B630F37100113",
            "18DAF11022111F5555555555",
        ],
    );
    text += &exchange(3.0, "010C", &["18DAF11004410C1AF8AAAAAA"]);

    let frames = live_frames(&parse_recording(&text).expect("parses"));
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].values.get(&0x0C), Some(&1726.0));
    assert_eq!(frames[0].values.get(&0x0D), Some(&80.0));
    let second = &frames[1];
    assert_eq!(second.values.get(&0x0C), Some(&3000.0));
    // 0x63 = 99 kPa, 0x37 - 40 = 15 °C, 0x0113 / 100 = 2.75 g/s, 0x1F = 12.2 %.
    assert_eq!(second.values.get(&0x0B), Some(&99.0));
    assert_eq!(second.values.get(&0x0F), Some(&15.0));
    assert_eq!(second.values.get(&0x10), Some(&2.75));
    assert!((second.values[&0x11] - 31.0 * 100.0 / 255.0).abs() < 1e-6);
    assert_eq!(frames[2].values.get(&0x0C), Some(&1726.0));
}

#[test]
fn a_thirty_minute_recording_is_read_quickly() {
    // 29-bit CAN with headers on, 20 rounds a second for 30 minutes: a
    // combined request for six PIDs answered in three frames and one single
    // PID, about what the live view sends to an engine.
    const ROUNDS: usize = 30 * 60 * 20;
    let mut text = String::with_capacity(ROUNDS * 420);
    text += HEADER;
    text += &exchange(0.5, "ATZ", &["ELM327 v1.5"]);
    text += &exchange(0.6, "ATSP7", &["OK"]);
    text += &exchange(0.7, "ATH1", &["OK"]);
    for round in 0..ROUNDS {
        let t = 1.0 + round as f64 * 0.05;
        let rpm = 800 + (round % 5000) as u32;
        let raw = rpm * 4;
        let first = format!(
            "18 DA F1 10 10 0F 41 0C {:02X} {:02X} 0D 32",
            raw >> 8,
            raw & 0xFF
        );
        text += &exchange(
            t,
            "010C0D0B0F1011",
            &[
                &first,
                "18 DA F1 10 21 0B 63 0F 37 10 01 13",
                "18 DA F1 10 22 11 1F 55 55 55 55 55",
            ],
        );
        text += &exchange(t + 0.025, "0105", &["18 DA F1 10 03 41 05 5A AA AA AA AA"]);
    }
    let recording = parse_recording(&text).expect("parses");
    assert_eq!(recording.events.len(), 6 + ROUNDS * 4);

    let start = Instant::now();
    let frames = live_frames(&recording);
    let took = start.elapsed().as_secs_f64();
    eprintln!(
        "live_frames: {ROUNDS} rounds, {} events, {} MB of text, {:.0} ms",
        recording.events.len(),
        text.len() / 1_000_000,
        took * 1000.0
    );
    assert_eq!(frames.len(), ROUNDS);
    assert_eq!(frames[0].values.get(&0x0C), Some(&800.0));
    assert_eq!(frames[0].values.get(&0x05), Some(&50.0));
    assert_eq!(frames[ROUNDS - 1].values.len(), 7);
    // Generous for a debug build on a busy runner; a release build needs a
    // fraction of it.
    assert!(took < 20.0, "{took:.1} s for 30 minutes");
}
