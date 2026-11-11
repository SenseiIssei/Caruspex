//! Fakes for the tests, ported from the C++ tests: scripted ELM327 adapters,
//! the simulated KWP1281 and KWP2000 control units (tests/FakeEcu*, ported
//! in their own files) and a rig around the built-in demo car. None of them
//! touches real hardware.
#![allow(dead_code)]

pub mod fake_ecu;
pub mod fake_ecu2000;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

use caruspex_demo::{DemoAdapter, DemoClock, DemoOptions, DemoTransport};
use caruspex_i18n::{Language, set_language};
use caruspex_link::{Elm327, LogFn, Transport};

pub fn approx(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1e-5 * (1.0 + expected.abs().max(actual.abs()))
}

pub fn near(actual: f64, expected: f64, margin: f64) -> bool {
    (actual - expected).abs() <= margin
}

/// The command as the fakes compare it: no CR, LF or spaces, upper case.
pub fn normalize(data: &[u8]) -> String {
    data.iter()
        .filter(|&&c| c != b'\r' && c != b'\n' && c != b' ')
        .map(|&c| (c as char).to_ascii_uppercase())
        .collect()
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

// ---------------------------------------------------------------------------
// Language: the active language is global and the tests of one file run in
// parallel. Every test holds this guard; it switches back to German at the
// end, even when a check fails. Inside a test `set_language` may switch.
// ---------------------------------------------------------------------------

pub struct LanguageGuard {
    _lock: MutexGuard<'static, ()>,
}

pub fn language(language: Language) -> LanguageGuard {
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

pub fn german() -> LanguageGuard {
    language(Language::German)
}

// ---------------------------------------------------------------------------
// ScriptCar: the scripted ELM327 of test_obd_batch.cpp (RecordingTransport),
// test_obd_extended.cpp and test_uds.cpp (ScriptTransport) in one. Every
// request has an answer for headers off and one for headers on; ATH0/ATH1
// switch between them like a real adapter. Several answers for the same
// request are given out one after the other (the last one stays).
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct Reply {
    pub plain: Vec<String>,
    pub headers: Vec<String>,
}

pub fn reply(plain: &[&str], headers: &[&str]) -> Reply {
    Reply {
        plain: plain.iter().map(|s| s.to_string()).collect(),
        headers: headers.iter().map(|s| s.to_string()).collect(),
    }
}

pub struct ScriptState {
    pub replies: BTreeMap<String, VecDeque<Reply>>,
    /// AT commands answered with "?".
    pub refused: BTreeSet<String>,
    /// Every command, in order.
    pub sent: Vec<String>,
    /// Every non-AT command except the plain 0100 probe, in order.
    pub obd_sent: Vec<String>,
    /// The answer to 0100 when no reply is scripted for it.
    pub protocol_reply: String,
    headers: bool,
}

impl ScriptState {
    /// One answer for `cmd`, replacing what was scripted before.
    pub fn set(&mut self, cmd: &str, r: Reply) {
        self.replies.insert(cmd.to_string(), VecDeque::from([r]));
    }

    /// Several answers for `cmd`, given out in turn.
    pub fn set_all(&mut self, cmd: &str, list: Vec<Reply>) {
        self.replies.insert(cmd.to_string(), list.into());
    }

    pub fn count(&self, cmd: &str) -> usize {
        self.sent.iter().filter(|s| *s == cmd).count()
    }
}

pub type Script = Arc<Mutex<ScriptState>>;

pub struct ScriptCar {
    state: Script,
    pending: Vec<u8>,
}

impl ScriptCar {
    pub fn new() -> (ScriptCar, Script) {
        let state = Arc::new(Mutex::new(ScriptState {
            replies: BTreeMap::new(),
            refused: BTreeSet::new(),
            sent: Vec::new(),
            obd_sent: Vec::new(),
            protocol_reply: "41 00 BE 1F A8 13".to_string(),
            headers: false,
        }));
        (
            ScriptCar {
                state: state.clone(),
                pending: Vec::new(),
            },
            state,
        )
    }
}

impl Transport for ScriptCar {
    fn write(&mut self, data: &[u8]) -> bool {
        let cmd = normalize(data);
        let mut s = self.state.lock().unwrap();
        s.sent.push(cmd.clone());
        let lines: Vec<String> = if s.refused.contains(&cmd) {
            vec!["?".into()]
        } else if cmd == "ATZ" {
            s.headers = false;
            vec!["ELM327 v1.5".into()]
        } else if cmd == "ATH1" || cmd == "ATH0" {
            s.headers = cmd == "ATH1";
            vec!["OK".into()]
        } else if cmd.starts_with("AT") {
            vec!["OK".into()]
        } else if s.replies.get(&cmd).is_some_and(|q| !q.is_empty()) {
            if cmd != "0100" {
                s.obd_sent.push(cmd.clone());
            }
            let headers = s.headers;
            let queue = s.replies.get_mut(&cmd).unwrap();
            let r = if queue.len() > 1 {
                queue.pop_front().unwrap()
            } else {
                queue[0].clone()
            };
            if headers { r.headers } else { r.plain }
        } else if cmd == "0100" {
            vec![s.protocol_reply.clone()]
        } else {
            s.obd_sent.push(cmd.clone());
            vec!["NO DATA".into()]
        };
        self.pending = answer(&lines);
        true
    }
    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
    fn flush_input(&mut self) {
        self.pending.clear();
    }
    fn name(&self) -> String {
        "Script".into()
    }
}

/// An Elm327 connected to a fresh ScriptCar with the given protocol order.
pub fn script_rig(order: &str) -> (Elm327, Script) {
    script_rig_with(order, |_| {})
}

/// Like `script_rig`, with answers scripted before connecting.
pub fn script_rig_with(order: &str, setup: impl FnOnce(&mut ScriptState)) -> (Elm327, Script) {
    let (car, script) = ScriptCar::new();
    setup(&mut script.lock().unwrap());
    let mut elm = Elm327::new();
    elm.connect(Box::new(car), order).expect("connects");
    (elm, script)
}

// ---------------------------------------------------------------------------
// FaultTransport (test_protocol.cpp): just enough of an ELM327 to connect and
// read faults: P0507 stored (mode 03) and P0121 pending (mode 07).
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FaultTransport {
    pending: Vec<u8>,
}

impl Transport for FaultTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        let cmd = normalize(data);
        self.pending = match cmd.as_str() {
            "ATZ" => answer(&["ELM327 v1.5"]),
            c if c.starts_with("AT") => answer(&["OK"]),
            "0100" => answer(&["41 00 BE 1F A8 13"]),
            "03" => answer(&["43 05 07 00 00 00 00"]),
            "07" => answer(&["47 01 21 00 00 00 00"]),
            "050101" => answer(&["45 01 01 8C"]),
            "0601" => answer(&["46 01 01 00 96 00 50 00 C8"]),
            _ => answer(&["NO DATA"]),
        };
        true
    }
    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
    fn flush_input(&mut self) {
        self.pending.clear();
    }
    fn name(&self) -> String {
        "Test".into()
    }
}

// ---------------------------------------------------------------------------
// UdsOnlyCar (test_obd_on_uds.cpp): refuses the classic services and answers
// the emission data only over UDS.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct UdsOnlyCar {
    /// Given out in turn, the last one stays.
    pub replies: BTreeMap<String, Vec<Reply>>,
    pub sent: Arc<Mutex<Vec<String>>>,
    pending: Vec<u8>,
    headers: bool,
}

pub fn count_sent(sent: &Arc<Mutex<Vec<String>>>, cmd: &str) -> usize {
    sent.lock().unwrap().iter().filter(|s| *s == cmd).count()
}

impl Transport for UdsOnlyCar {
    fn write(&mut self, data: &[u8]) -> bool {
        let cmd = normalize(data);
        self.sent.lock().unwrap().push(cmd.clone());
        let lines: Vec<String> = if cmd == "ATZ" {
            self.headers = false;
            vec!["ELM327 v2.1".into()]
        } else if cmd == "ATH1" || cmd == "ATH0" {
            self.headers = cmd == "ATH1";
            vec!["OK".into()]
        } else if cmd == "ATDP" {
            vec!["ISO 15765-4 (CAN 11/500)".into()]
        } else if cmd.starts_with("AT") {
            vec!["OK".into()]
        } else if cmd.starts_with("ST") {
            vec!["?".into()]
        } else if cmd.len() >= 2 && cmd.starts_with('0') {
            // Every classic service is refused.
            vec![if self.headers {
                format!("7E8 03 7F {} 11", &cmd[..2])
            } else {
                format!("7F {} 11", &cmd[..2])
            }]
        } else {
            match self.replies.get_mut(&cmd) {
                Some(list) if !list.is_empty() => {
                    let r = list[0].clone();
                    if list.len() > 1 {
                        list.remove(0);
                    }
                    if self.headers { r.headers } else { r.plain }
                }
                _ => vec!["NO DATA".into()],
            }
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
        "UDS only".into()
    }
}

pub fn scripted_uds_car() -> UdsOnlyCar {
    let mut car = UdsOnlyCar::default();
    let r = &mut car.replies;
    r.insert(
        "22F400".into(),
        vec![reply(
            &["62 F4 00 BE 3F A8 13"],
            &["7E8 07 62 F4 00 BE 3F A8 13"],
        )],
    );
    r.insert(
        "22F401".into(),
        vec![reply(
            &["62 F4 01 81 07 65 04"],
            &["7E8 07 62 F4 01 81 07 65 04"],
        )],
    );
    r.insert(
        "22F40C".into(),
        vec![reply(&["62 F4 0C 0C 80"], &["7E8 05 62 F4 0C 0C 80 AA AA"])],
    );
    // The first answer is "busy", the second the value.
    r.insert(
        "22F405".into(),
        vec![
            reply(&["7F 22 78"], &["7E8 03 7F 22 78 AA AA AA AA"]),
            reply(&["62 F4 05 7B"], &["7E8 04 62 F4 05 7B AA AA AA"]),
        ],
    );
    // VIN in F802 without the count byte of service 09: 20 bytes.
    r.insert(
        "22F802".into(),
        vec![reply(
            &[
                "014",
                "0: 62 F8 02 57 41 55",
                "1: 5A 5A 5A 38 56 35 4C",
                "2: 41 30 31 32 33 34 35",
            ],
            &[
                "7E8 10 14 62 F8 02 57 41 55",
                "7E8 21 5A 5A 5A 38 56 35 4C",
                "7E8 22 41 30 31 32 33 34 35",
            ],
        )],
    );
    // Two confirmed emission DTCs, WWH-OBD records: severity, DTC (3), status.
    // 59 42 33 | avail FF | sev avail E0 | format 04 | 20 01 71 00 08 | 40 04 20 00 08
    r.insert(
        "19423308FF".into(),
        vec![reply(
            &[
                "010",
                "0: 59 42 33 FF E0 04",
                "1: 20 01 71 00 08 40 04",
                "2: 20 00 08",
            ],
            &[
                "7E8 10 10 59 42 33 FF E0 04",
                "7E8 21 20 01 71 00 08 40 04",
                "7E8 22 20 00 08 AA AA AA AA",
            ],
        )],
    );
    r.insert(
        "19423304FF".into(),
        vec![reply(
            &["59 42 33 FF E0 04"],
            &["7E8 06 59 42 33 FF E0 04 AA"],
        )],
    );
    // One permanent DTC: 59 55 33 | avail FF | format 04 | 03 00 00 08
    r.insert(
        "195533".into(),
        vec![reply(
            &["59 55 33 FF 04 03 00 00 08"],
            &["7E8 10 09 59 55 33 FF 04 03", "7E8 21 00 00 08 AA AA AA AA"],
        )],
    );
    r.insert(
        "14FFFF33".into(),
        vec![reply(&["54"], &["7E8 01 54 AA AA AA AA AA AA"])],
    );
    car
}

// ---------------------------------------------------------------------------
// DemoRig (tests/DemoRig.h): an Elm327 connected to the built-in demo car on
// protocol 6 without delays, with a clock the test moves by hand. Every line
// Elm327 logs (command and answer) ends up in `sent`.
// ---------------------------------------------------------------------------

pub struct DemoRig {
    pub clock: DemoClock,
    pub elm: Elm327,
    pub sent: Arc<Mutex<Vec<String>>>,
    pub connected: bool,
    pub error: String,
}

impl DemoRig {
    pub fn new(start: f64) -> Self {
        Self::with(start, DemoAdapter::CloneV15, true, 0.0)
    }

    pub fn with(start: f64, adapter: DemoAdapter, gearbox: bool, latency: f64) -> Self {
        let clock = DemoClock::new(start);
        let options = DemoOptions {
            adapter,
            clock: Some(clock.source()),
            latency_scale: latency,
            transmission_ecu: gearbox,
            ..DemoOptions::default()
        };
        let mut elm = Elm327::new();
        elm.set_verbose(true);
        let sent = Arc::new(Mutex::new(Vec::new()));
        let sink = sent.clone();
        let log: LogFn = Box::new(move |line: &str| sink.lock().unwrap().push(line.to_string()));
        elm.set_log(log);
        let result = elm.connect(Box::new(DemoTransport::new(options)), "6");
        DemoRig {
            clock,
            elm,
            sent,
            connected: result.is_ok(),
            error: result.err().unwrap_or_default(),
        }
    }

    pub fn now(&self) -> f64 {
        self.clock.get()
    }

    pub fn set_now(&self, seconds: f64) {
        self.clock.set(seconds);
    }

    pub fn advance(&self, seconds: f64) {
        self.clock.advance(seconds);
    }

    pub fn clear_sent(&self) {
        self.sent.lock().unwrap().clear();
    }

    pub fn sent(&self) -> Vec<String> {
        self.sent.lock().unwrap().clone()
    }

    /// How many logged commands start with `prefix`.
    pub fn count_sent(&self, prefix: &str) -> usize {
        let needle = format!("  > {prefix}");
        self.sent
            .lock()
            .unwrap()
            .iter()
            .filter(|line| line.starts_with(&needle))
            .count()
    }
}
