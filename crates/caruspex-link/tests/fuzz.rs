//! Fuzz-style property tests of the ELM327 reply parsers: random and
//! half-broken adapter replies must never panic, and what comes out keeps
//! its own rules. They run with the normal `cargo test` on the stable
//! toolchain; PROPTEST_CASES raises the number of cases for a longer hunt.

use caruspex_link::{
    Elm327, HeaderFormat, Transport, decode_messages, hex_to_bytes, split_batch_answer,
    split_messages,
};
use proptest::prelude::*;

const FORMATS: [HeaderFormat; 5] = [
    HeaderFormat::None,
    HeaderFormat::Can11,
    HeaderFormat::Can29,
    HeaderFormat::KLine,
    HeaderFormat::J1850,
];

/// Lines that look like adapter output: hex with and without spaces,
/// numbered multi-frame lines, CAN ids, length lines, status words, and
/// plain noise.
fn adapter_line() -> impl Strategy<Value = String> {
    prop_oneof![
        "[0-9A-F ]{0,40}",
        "[0-9A-F]:[0-9A-F ]{0,30}",
        "7E[0-9A-F][0-9A-F ]{0,24}",
        "18DA[0-9A-F]{4}[0-9A-F ]{0,24}",
        "[0-9A-F]{3}",
        Just("NO DATA".to_string()),
        Just("CAN ERROR".to_string()),
        Just("BUS INIT: ...ERROR".to_string()),
        Just("SEARCHING...".to_string()),
        Just("?".to_string()),
        ".{0,20}",
    ]
}

/// Replies the fake adapter sends: one of the lines above or raw bytes,
/// always with the prompt at the end so `send` returns at once.
#[derive(Clone, Debug)]
enum Reply {
    Lines(Vec<String>),
    Bytes(Vec<u8>),
}

fn reply() -> impl Strategy<Value = Reply> {
    prop_oneof![
        prop::collection::vec(adapter_line(), 0..6).prop_map(Reply::Lines),
        prop::collection::vec(any::<u8>(), 0..64).prop_map(Reply::Bytes),
    ]
}

struct Scripted {
    replies: Vec<Reply>,
    next: usize,
    pending: Vec<u8>,
}

impl Transport for Scripted {
    fn write(&mut self, _data: &[u8]) -> bool {
        let reply = if self.replies.is_empty() {
            Reply::Lines(Vec::new())
        } else {
            self.replies[self.next % self.replies.len()].clone()
        };
        self.next += 1;
        self.pending = match reply {
            Reply::Lines(lines) => lines.join("\r").into_bytes(),
            Reply::Bytes(bytes) => bytes,
        };
        self.pending.extend_from_slice(b"\r\r>");
        true
    }
    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
    fn flush_input(&mut self) {}
    fn name(&self) -> String {
        "fuzz".into()
    }
}

proptest! {
    #[test]
    fn decoding_lines_never_panics(
        lines in prop::collection::vec(adapter_line(), 0..12),
        service in any::<u8>(),
        format in 0usize..5,
    ) {
        let format = FORMATS[format];
        let result = decode_messages(&lines, service, format);
        prop_assert_eq!(result.messages.len(), result.sources.len());
        let raw = split_messages(&lines, format);
        // Every decoded message is one of the split ones without its
        // service byte.
        for message in &result.messages {
            prop_assert!(raw.iter().any(|m| m.bytes.len() == message.len() + 1
                && &m.bytes[1..] == message.as_slice()));
        }
    }

    #[test]
    fn hex_to_bytes_takes_every_digit_pair(text in ".{0,80}") {
        let digits = text.chars().filter(char::is_ascii_hexdigit).count();
        prop_assert_eq!(hex_to_bytes(&text).len(), digits / 2);
    }

    #[test]
    fn batch_answers_never_panic(lines in prop::collection::vec(adapter_line(), 0..10)) {
        let _ = split_batch_answer(&lines);
    }

}

proptest! {
    // Every connect waits half a second for a greeting of the adapter, so
    // fewer cases here; PROPTEST_CASES still raises them.
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(16)
    ))]

    #[test]
    fn the_driver_survives_any_adapter(replies in prop::collection::vec(reply(), 1..12)) {
        let mut elm = Elm327::new();
        // No protocol to try: the reset, the settings and the STN check
        // read the random replies, nothing sleeps between protocols.
        let _ = elm.connect(
            Box::new(Scripted { replies, next: 0, pending: Vec::new() }),
            "",
        );
        // The transport stays after a failed connect, so the request paths
        // can read more of them.
        let _ = elm.request("0100", 0x01, 50);
        let _ = elm.request("03", 0x03, 50);
        let _ = elm.request("0902", 0x09, 50);
        let _ = elm.set_headers(true);
        let _ = elm.request("0902", 0x09, 50);
        let _ = elm.request_many(&["010C".to_string(), "010D".to_string()], 0x01, 50);
        let _ = elm.monitor_lines(20, 4);
        let _ = elm.version();
    }
}
