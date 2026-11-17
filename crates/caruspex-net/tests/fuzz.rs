//! Fuzz-style property tests of the network and CAN parsers: HSFZ and DoIP
//! frames cut from any byte stream, the identification answers, the DBC
//! signal syntax, SLCAN lines and the ELM front ends on top of a car that
//! answers rubbish. Normal `cargo test`, stable toolchain; PROPTEST_CASES
//! raises the number of cases.

use caruspex_link::Transport;
use caruspex_net::enet::{DiagAnswer, DiagLink, EnetElm, Flavor, doip, hsfz};
use caruspex_net::{
    CanElm, CanFrame, CanLink, SlcanLink, decode_can_signal, parse_can_signal,
    parse_monitor_request,
};
use proptest::prelude::*;

/// Cuts `bytes` at the given points into chunks, the way TCP hands them out.
fn chunks(bytes: &[u8], cuts: &[usize]) -> Vec<Vec<u8>> {
    let mut points: Vec<usize> = cuts.iter().map(|c| c % (bytes.len() + 1)).collect();
    points.sort_unstable();
    let mut out = Vec::new();
    let mut from = 0;
    for p in points {
        out.push(bytes[from..p].to_vec());
        from = p;
    }
    out.push(bytes[from..].to_vec());
    out
}

fn payloads() -> impl Strategy<Value = Vec<(u8, Vec<u8>)>> {
    prop::collection::vec(
        (any::<u8>(), prop::collection::vec(any::<u8>(), 0..40)),
        0..6,
    )
}

/// An ELM327 command line as the driver or a user at the raw console
/// sends it.
fn command() -> impl Strategy<Value = String> {
    prop_oneof![
        "AT[A-Z0-9 ]{0,10}",
        "ST[A-Z0-9 ]{0,10}",
        "[0-9A-F ]{0,16}",
        "AT ?SH ?[0-9A-F]{0,8}",
        "AT ?CRA ?[0-9A-FX]{0,8}",
        ".{0,16}",
    ]
}

struct RubbishCar {
    answers: Vec<Vec<u8>>,
    next: usize,
}

impl DiagLink for RubbishCar {
    fn request(
        &mut self,
        target: u16,
        _data: &[u8],
        _timeout_ms: u64,
    ) -> Result<Vec<DiagAnswer>, String> {
        self.next += 1;
        if self.answers.is_empty() || self.next.is_multiple_of(5) {
            return Err("gone".into());
        }
        let data = self.answers[self.next % self.answers.len()].clone();
        Ok(vec![DiagAnswer {
            source: target,
            data,
        }])
    }
}

struct RubbishBus {
    frames: Vec<CanFrame>,
    next: usize,
}

impl CanLink for RubbishBus {
    fn send(&mut self, _frame: &CanFrame) -> bool {
        true
    }
    fn receive(&mut self, _timeout_ms: u64) -> Option<CanFrame> {
        // A few frames, then silence, so every request ends quickly.
        let frame = self.frames.get(self.next).cloned();
        self.next += 1;
        frame
    }
    fn name(&self) -> String {
        "fuzz".into()
    }
}

struct RubbishSerial {
    bytes: Vec<u8>,
}

impl Transport for RubbishSerial {
    fn write(&mut self, _data: &[u8]) -> bool {
        true
    }
    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        let take = self.bytes.len().min(7);
        self.bytes.drain(..take).collect()
    }
    fn flush_input(&mut self) {}
    fn name(&self) -> String {
        "fuzz".into()
    }
}

fn can_frame() -> impl Strategy<Value = CanFrame> {
    (
        prop_oneof![0x7E8u32..0x7F0, 0u32..0x800, any::<u32>()],
        any::<bool>(),
        prop::collection::vec(any::<u8>(), 0..9),
    )
        .prop_map(|(id, extended, data)| CanFrame { id, extended, data })
}

fn talk(elm: &mut dyn Transport, commands: &[String]) {
    for c in commands {
        elm.write(format!("{c}\r").as_bytes());
        let _ = elm.read(10);
    }
}

proptest! {
    #[test]
    fn hsfz_frames_come_back_whole_in_any_chunks(frames in payloads(), cuts in prop::collection::vec(any::<usize>(), 0..8)) {
        let mut stream = Vec::new();
        for (control, payload) in &frames {
            stream.extend(hsfz::encode(*control, payload));
        }
        let mut parser = hsfz::Parser::new();
        let mut got = Vec::new();
        for piece in chunks(&stream, &cuts) {
            parser.feed(&piece);
            got.extend(parser.by_ref());
        }
        prop_assert!(!parser.broken());
        prop_assert_eq!(got.len(), frames.len());
        for (frame, (control, payload)) in got.iter().zip(&frames) {
            prop_assert_eq!(frame.control, *control);
            prop_assert_eq!(&frame.payload, payload);
        }
    }

    #[test]
    fn doip_messages_come_back_whole_in_any_chunks(messages in payloads(), cuts in prop::collection::vec(any::<usize>(), 0..8)) {
        let mut stream = Vec::new();
        for (kind, payload) in &messages {
            stream.extend(doip::encode(0x8000 | u16::from(*kind), payload));
        }
        let mut parser = doip::Parser::new();
        let mut got = Vec::new();
        for piece in chunks(&stream, &cuts) {
            parser.feed(&piece);
            got.extend(parser.by_ref());
        }
        prop_assert!(!parser.broken());
        prop_assert_eq!(got.len(), messages.len());
        for (message, (kind, payload)) in got.iter().zip(&messages) {
            prop_assert_eq!(message.payload_type, 0x8000 | u16::from(*kind));
            prop_assert_eq!(&message.payload, payload);
        }
    }

    #[test]
    fn any_bytes_on_the_wire_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..300)) {
        let mut h = hsfz::Parser::new();
        h.feed(&bytes);
        let _ = h.by_ref().count();
        let mut d = doip::Parser::new();
        d.feed(&bytes);
        let _ = d.by_ref().count();
        if let Some(ident) = hsfz::parse_ident(&bytes) {
            prop_assert!(ident.vin.is_empty() || ident.vin.len() == 17);
        }
        if let Some(entity) = doip::parse_announcement(&bytes) {
            prop_assert!(entity.vin.is_empty() || entity.vin.len() == 17);
        }
    }

    #[test]
    fn identification_answers_with_a_header(rest in prop::collection::vec(any::<u8>(), 0..80)) {
        let mut answer = vec![0, 0, 0, 0x32, 0, 0x11];
        answer.extend_from_slice(b"DIAGADR");
        answer.extend(rest);
        let _ = hsfz::parse_ident(&answer);
    }

    #[test]
    fn dbc_signals_never_panic(text in prop_oneof![
        "DBC:[0-9|@+()m,.e-]{0,30}",
        "DBC:[0-9]{1,4}\\|[0-9]{1,3}@[01][+-]\\([0-9.e-]{0,8},[0-9.e-]{0,8}\\)(m[0-9]{1,3}@[0-9]{1,3}\\|[0-9]{1,2}(@[01])?)?",
        ".{0,30}",
    ], frame in prop::collection::vec(any::<u8>(), 0..9)) {
        if let Ok(signal) = parse_can_signal(&text) {
            prop_assert!((1..=64).contains(&signal.length));
            if let Some(value) = decode_can_signal(&signal, &frame) {
                prop_assert!(value.is_finite());
            }
        }
        let _ = parse_monitor_request(&text);
    }

    #[test]
    fn slcan_lines_never_panic(bytes in prop::collection::vec(prop_oneof![
        Just(b't'), Just(b'T'), Just(b'r'), Just(b'\r'), Just(0x07u8),
        0x30u8..0x47, any::<u8>(),
    ], 0..200)) {
        let mut link = SlcanLink::new(Box::new(RubbishSerial { bytes }));
        for _ in 0..40 {
            if let Some(frame) = link.receive(1) {
                prop_assert!(frame.data.len() <= 8);
            }
        }
    }
}

proptest! {
    // The front ends wait out their timeouts on silence: fewer cases.
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(48)
    ))]

    #[test]
    fn the_enet_front_end_survives_any_car(
        answers in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..24), 0..6),
        commands in prop::collection::vec(command(), 0..12),
        doip_flavor in any::<bool>(),
    ) {
        let flavor = if doip_flavor { Flavor::Doip } else { Flavor::Bmw };
        let mut elm = EnetElm::new(Box::new(RubbishCar { answers, next: 0 }), "fuzz", flavor);
        let mut all = vec!["ATZ".to_string(), "ATH1".to_string(), "0100".to_string(), "0902".to_string()];
        all.extend(commands);
        talk(&mut elm, &all);
    }

    #[test]
    fn the_can_front_end_survives_any_bus(
        frames in prop::collection::vec(can_frame(), 0..10),
        commands in prop::collection::vec(command(), 0..8),
    ) {
        let mut elm = CanElm::new(Box::new(RubbishBus { frames, next: 0 }), "fuzz", 500_000);
        let mut all = vec!["ATZ".to_string(), "ATST10".to_string(), "ATH1".to_string(), "0100".to_string(), "0902".to_string()];
        all.extend(commands);
        talk(&mut elm, &all);
    }
}
