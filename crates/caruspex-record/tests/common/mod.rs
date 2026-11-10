//! Fakes for the tests, ported from the C++ tests: a rig around the built-in
//! demo car (tests/DemoRig.h), the scripted ELM327 with recorded OBDb answers
//! (test_custom_pids.cpp), the listening bus of test_can_signals.cpp and a
//! transport that answers chosen commands itself. None of them touches real
//! hardware.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use caruspex_demo::{DemoAdapter, DemoClock, DemoOptions, DemoTransport};
use caruspex_i18n::{Language, set_language};
use caruspex_link::{Elm327, LogFn, Transport};

pub type Log = Arc<Mutex<Vec<String>>>;

// The active language is global and the tests of one file run in parallel:
// every test holds this guard, which switches back to German at the end,
// even when a check fails. Inside a test `set_language` may switch.
pub struct LanguageGuard {
    _lock: MutexGuard<'static, ()>,
}

pub fn language(language: Language) -> LanguageGuard {
    static LOCK: Mutex<()> = Mutex::new(());
    let lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    set_language(language);
    LanguageGuard { _lock: lock }
}

pub fn german() -> LanguageGuard {
    language(Language::German)
}

impl Drop for LanguageGuard {
    fn drop(&mut self) {
        set_language(Language::German);
    }
}

pub fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

pub fn approx(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1e-5 * (1.0 + expected.abs().max(actual.abs()))
}

/// Lines plus a blank line and the prompt, as an ELM327 answers.
pub fn answer<S: AsRef<str>>(lines: &[S]) -> Vec<u8> {
    let mut out = String::new();
    for l in lines {
        out.push_str(l.as_ref());
        out.push('\r');
    }
    out.push_str("\r>");
    out.into_bytes()
}

pub fn capture_log(elm: &mut Elm327) -> Log {
    let lines: Log = Arc::new(Mutex::new(Vec::new()));
    let sink = lines.clone();
    let log: LogFn = Box::new(move |line: &str| sink.lock().unwrap().push(line.to_string()));
    elm.set_log(log);
    lines
}

/// The commands out of the verbose log lines ("  > ATZ   ->   ...").
pub fn sent_commands(log: &[String]) -> Vec<String> {
    log.iter()
        .filter_map(|line| {
            let a = line.find("> ")?;
            let b = line.find("   ->")?;
            (b >= a + 2).then(|| line[a + 2..b].to_string())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// DemoRig (tests/DemoRig.h): an Elm327 connected to the built-in demo car on
// protocol 6 without delays, with a clock the test moves by hand. Every line
// Elm327 logs (command and answer) ends up in `sent`.
// ---------------------------------------------------------------------------

pub struct DemoRig {
    pub clock: DemoClock,
    pub elm: Elm327,
    pub sent: Log,
    pub connected: bool,
}

impl DemoRig {
    pub fn new(start: f64) -> Self {
        let clock = DemoClock::new(start);
        let options = DemoOptions {
            adapter: DemoAdapter::CloneV15,
            clock: Some(clock.source()),
            latency_scale: 0.0,
            transmission_ecu: true,
            ..DemoOptions::default()
        };
        let mut elm = Elm327::new();
        elm.set_verbose(true);
        let sent = capture_log(&mut elm);
        let connected = elm
            .connect(Box::new(DemoTransport::new(options)), "6")
            .is_ok();
        DemoRig {
            clock,
            elm,
            sent,
            connected,
        }
    }

    pub fn clear_sent(&self) {
        self.sent.lock().unwrap().clear();
    }

    pub fn sent(&self) -> Vec<String> {
        self.sent.lock().unwrap().clone()
    }

    /// How many logged commands are exactly `what`.
    pub fn count(&self, what: &str) -> usize {
        let needle = format!("> {what} ");
        self.sent()
            .iter()
            .filter(|line| line.contains(&needle))
            .count()
    }
}

// ---------------------------------------------------------------------------
// EcuScript: a scripted ELM327 on CAN 11 bit that knows which control unit is
// addressed. Answers are recorded CAN frames as OBDb writes them:
// "7EC103E620101FFF7E7" is id 7EC with the eight data bytes after it. They
// come out with or without headers, the way the adapter prints them with CAN
// auto formatting.
// ---------------------------------------------------------------------------

type Frames = Arc<Mutex<BTreeMap<String, Vec<String>>>>;

pub struct EcuScript {
    /// "7E4|220101" -> frames
    frames: Frames,
    sent: Log,
    pending: Vec<u8>,
    header: String,
    headers: bool,
}

fn spaced(hex: &str) -> String {
    hex.as_bytes()
        .chunks(2)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

impl EcuScript {
    fn format(&self, raw: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        for f in raw {
            let id = &f[..3];
            let data = &f[3..];
            if self.headers {
                out.push(format!("{id} {}", spaced(data)));
                continue;
            }
            let pci = usize::from_str_radix(&data[..2], 16).unwrap();
            match pci >> 4 {
                0 => out.push(spaced(&data[2..2 + pci * 2])),
                1 => {
                    let length =
                        ((pci & 0x0F) << 8) | usize::from_str_radix(&data[2..4], 16).unwrap();
                    out.push(format!("{length:03X}"));
                    out.push(format!("0: {}", spaced(&data[4..])));
                }
                _ => out.push(format!("{}: {}", pci & 0x0F, spaced(&data[2..]))),
            }
        }
        out
    }
}

impl Transport for EcuScript {
    fn write(&mut self, data: &[u8]) -> bool {
        let cmd: String = data
            .iter()
            .filter(|&&c| c != b'\r' && c != b'\n' && c != b' ')
            .map(|&c| (c as char).to_ascii_uppercase())
            .collect();
        self.sent.lock().unwrap().push(cmd.clone());
        if cmd == "ATZ" {
            self.headers = false;
            self.header = "7DF".into();
            self.pending = b"ELM327 v1.5\r\r>".to_vec();
        } else if cmd == "ATH1" || cmd == "ATH0" {
            self.headers = cmd == "ATH1";
            self.pending = b"OK\r\r>".to_vec();
        } else if cmd == "ATDP" {
            self.pending = b"ISO 15765-4 (CAN 11/500)\r\r>".to_vec();
        } else if let Some(h) = cmd.strip_prefix("ATSH") {
            self.header = h.to_string();
            self.pending = b"OK\r\r>".to_vec();
        } else if cmd.starts_with("AT") {
            self.pending = b"OK\r\r>".to_vec();
        } else if cmd == "0100" {
            self.pending = if self.headers {
                b"7E8 06 41 00 BE 1F A8 13 AA\r\r>".to_vec()
            } else {
                b"41 00 BE 1F A8 13\r\r>".to_vec()
            };
        } else {
            let found = self
                .frames
                .lock()
                .unwrap()
                .get(&format!("{}|{cmd}", self.header))
                .cloned();
            self.pending = match found {
                None => b"NO DATA\r\r>".to_vec(),
                Some(frames) => {
                    let mut out = String::new();
                    for line in self.format(&frames) {
                        out.push_str(&line);
                        out.push('\r');
                    }
                    out.push_str("\r>");
                    out.into_bytes()
                }
            };
        }
        true
    }
    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
    fn flush_input(&mut self) {
        self.pending.clear();
    }
    fn name(&self) -> String {
        "EcuScript".into()
    }
}

pub struct ScriptCar {
    pub elm: Elm327,
    frames: Frames,
    sent: Log,
    pub connected: bool,
}

impl ScriptCar {
    pub fn new() -> Self {
        let frames: Frames = Arc::new(Mutex::new(BTreeMap::new()));
        let sent: Log = Arc::new(Mutex::new(Vec::new()));
        let script = EcuScript {
            frames: frames.clone(),
            sent: sent.clone(),
            pending: Vec::new(),
            header: "7DF".into(),
            headers: false,
        };
        let mut elm = Elm327::new();
        let connected = elm.connect(Box::new(script), "6").is_ok();
        ScriptCar {
            elm,
            frames,
            sent,
            connected,
        }
    }

    pub fn set_frames<S: AsRef<str>>(&self, key: &str, frames: &[S]) {
        self.frames.lock().unwrap().insert(
            key.to_string(),
            frames.iter().map(|s| s.as_ref().to_string()).collect(),
        );
    }

    pub fn count(&self, cmd: &str) -> usize {
        self.sent
            .lock()
            .unwrap()
            .iter()
            .filter(|c| *c == cmd)
            .count()
    }

    pub fn clear_sent(&self) {
        self.sent.lock().unwrap().clear();
    }
}

// ---------------------------------------------------------------------------
// ListeningBus (test_can_signals.cpp): an ELM327 on a bus where nobody
// answers OBD, but frames go by.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct ListeningBus {
    /// By id, sent in turn.
    pub frames: BTreeMap<u32, Vec<Vec<u8>>>,
    pending: Vec<u8>,
    headers: bool,
    monitoring: bool,
    filter: Option<u32>,
}

impl Transport for ListeningBus {
    fn write(&mut self, data: &[u8]) -> bool {
        let cmd: String = data
            .iter()
            .filter(|&&c| c != b'\r' && c != b' ')
            .map(|&c| (c as char).to_ascii_uppercase())
            .collect();
        if self.monitoring {
            // Any character stops the monitor.
            self.monitoring = false;
            self.pending.extend_from_slice(b"STOPPED\r\r>");
            return true;
        }
        let lines: Vec<&str> = if cmd == "ATZ" {
            vec!["ELM327 v1.5"]
        } else if cmd == "ATH1" || cmd == "ATH0" {
            self.headers = cmd == "ATH1";
            vec!["OK"]
        } else if cmd == "ATCRA" || cmd == "ATAR" {
            self.filter = None;
            vec!["OK"]
        } else if let Some(id) = cmd.strip_prefix("ATCRA") {
            self.filter = u32::from_str_radix(id, 16).ok();
            vec!["OK"]
        } else if cmd == "ATMA" {
            self.monitoring = true;
            for _ in 0..3 {
                for (id, list) in &self.frames {
                    if self.filter.is_some_and(|f| f != *id) {
                        continue;
                    }
                    for f in list {
                        let mut line = if self.headers {
                            format!("{id:03X} ")
                        } else {
                            String::new()
                        };
                        for b in f {
                            line.push_str(&format!("{b:02X} "));
                        }
                        self.pending.extend_from_slice(line.as_bytes());
                        self.pending.push(b'\r');
                    }
                }
            }
            return true;
        } else if cmd.starts_with("AT") {
            vec!["OK"]
        } else if cmd.starts_with("ST") {
            vec!["?"]
        } else {
            vec!["NO DATA"]
        };
        self.pending.extend(answer(&lines));
        true
    }
    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
    fn flush_input(&mut self) {
        self.pending.clear();
    }
    fn name(&self) -> String {
        "Listening bus".into()
    }
}

fn le_frame(v: u64) -> Vec<u8> {
    (0..8).map(|i| ((v >> (8 * i)) & 0xFF) as u8).collect()
}

fn ascii(mux: u8, text: &str) -> Vec<u8> {
    let mut f = vec![mux];
    f.extend(text.bytes().map(|c| if c == b'_' { 0 } else { c }));
    f
}

/// A Model 3 as the DBC describes its frames.
pub fn tesla() -> ListeningBus {
    let mut bus = ListeningBus::default();
    // 132: 384.16 V (0x9610), smoothed current raw -200 -> 20.0 A.
    bus.frames.insert(
        0x132,
        vec![vec![0x10, 0x96, 0x38, 0xFF, 0x00, 0x00, 0x00, 0x00]],
    );
    // 292: SOCmin 80.0, SOCUI 80.5, SOCmax 81.0, SOCave 80.7, BOL 82.0 kWh.
    bus.frames.insert(
        0x292,
        vec![le_frame(
            800 | (805 << 10) | (810 << 20) | (807 << 30) | (820 << 40),
        )],
    );
    // 352: full pack 74.6 kWh (746), remaining 60.1 kWh (601 at bit 11).
    bus.frames.insert(0x352, vec![le_frame(746 | (601 << 11))]);
    // 332: multiplexer 1 with cells 4.100 V / 4.080 V, multiplexer 0 with
    // temperatures 25 and 20 degrees.
    bus.frames.insert(
        0x332,
        vec![
            vec![0x09, 0x20, 0xF8, 0x07, 0x00, 0x00],
            vec![0x00, 0x00, 0x82, 0x78, 0x00, 0x00],
        ],
    );
    // 405: the VIN in three parts.
    bus.frames.insert(
        0x405,
        vec![
            ascii(0x10, "____5YJ"),
            ascii(0x11, "3E1EA7K"),
            ascii(0x12, "F123456"),
        ],
    );
    bus
}

// ---------------------------------------------------------------------------
// InterceptTransport: wraps a transport and lets a test answer chosen
// commands itself (`Some`) or lets them through (`None`).
// ---------------------------------------------------------------------------

pub type Rule = Box<dyn FnMut(&str) -> Option<Vec<u8>> + Send>;

pub struct InterceptTransport {
    inner: Box<dyn Transport>,
    rule: Rule,
    queued: Vec<u8>,
}

impl InterceptTransport {
    pub fn new(
        inner: Box<dyn Transport>,
        rule: impl FnMut(&str) -> Option<Vec<u8>> + Send + 'static,
    ) -> Self {
        Self {
            inner,
            rule: Box::new(rule),
            queued: Vec::new(),
        }
    }
}

fn normalize(data: &[u8]) -> String {
    data.iter()
        .filter(|&&c| c != b'\r' && c != b'\n' && c != b' ')
        .map(|&c| (c as char).to_ascii_uppercase())
        .collect()
}

impl Transport for InterceptTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        if let Some(answer) = (self.rule)(&normalize(data)) {
            self.queued.extend(answer);
            return true;
        }
        self.inner.write(data)
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        if !self.queued.is_empty() {
            return std::mem::take(&mut self.queued);
        }
        self.inner.read(timeout_ms)
    }
    fn flush_input(&mut self) {
        self.queued.clear();
        self.inner.flush_input();
    }
    fn name(&self) -> String {
        self.inner.name()
    }
}
