//! Fake adapters for the tests, ported from the C++ tests. None of them
//! touches real hardware.
#![allow(dead_code)]

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use caruspex_i18n::{Language, set_language};
use caruspex_link::{Elm327, LogFn, Transport};

/// The command as the fakes compare it: no CR, LF or spaces, upper case.
pub fn normalize(data: &[u8]) -> String {
    data.iter()
        .filter(|&&c| c != b'\r' && c != b'\n' && c != b' ')
        .map(|&c| (c as char).to_ascii_uppercase())
        .collect()
}

/// Lines plus a blank line and the prompt, as an ELM327 answers.
pub fn answer(lines: &[&str]) -> Vec<u8> {
    let mut out = String::new();
    for l in lines {
        out.push_str(l);
        out.push('\r');
    }
    out.push_str("\r>");
    out.into_bytes()
}

// ---------------------------------------------------------------------------
// Language: the active language is global, the tests of one file run in
// parallel. Tests that look at texts hold this guard; it switches back to
// German at the end, even when a check fails.
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

/// A log that collects every line Elm327 writes.
pub fn capture_log(elm: &mut Elm327) -> Arc<Mutex<Vec<String>>> {
    let lines = Arc::new(Mutex::new(Vec::new()));
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
// SettingsTransport (test_settings.cpp): answers like an ELM327 on the
// simulator. With silent the adapter can be muted to run into a timeout.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct SettingsTransport {
    pub silent: Arc<AtomicBool>,
    pending: Vec<u8>,
}

impl Transport for SettingsTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        if self.silent.load(Ordering::SeqCst) {
            return true;
        }
        let cmd = normalize(data);
        self.pending = match cmd.as_str() {
            "ATZ" => answer(&["ELM327 v1.5"]),
            c if c.starts_with("AT") => answer(&["OK"]),
            "0100" => answer(&["41 00 BE 1F A8 13"]),
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
// ListeningBus (test_can_signals.cpp): an ELM327 on a bus where nobody
// answers OBD, but frames go by.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct ListeningBus {
    /// By id, sent in turn.
    pub frames: BTreeMap<u32, Vec<Vec<u8>>>,
    /// Behaves like an STN chip (OBDLink EX): STI, STBC, STMA n, STM n and
    /// the pass filter STFPA, as the OBDLink manual describes them.
    pub stn: bool,
    /// Every command line as received, for the tests to check.
    pub sent: Arc<Mutex<Vec<String>>>,
    pending: Vec<u8>,
    headers: bool,
    monitoring: bool,
    filter: Option<u32>,
    pass: Option<u32>,
}

impl ListeningBus {
    /// Three rounds of every frame the hardware filter (and, for STM, the
    /// pass filter) lets through; with a count the monitor ends by itself.
    fn monitor(&mut self, all: bool, use_pass: bool, count: Option<usize>) {
        let mut printed = 0;
        for _ in 0..3 {
            for (id, list) in &self.frames {
                if !all && self.filter.is_some_and(|f| f != *id) {
                    continue;
                }
                if use_pass && self.pass.is_some_and(|p| p != *id) {
                    continue;
                }
                for f in list {
                    if count.is_some_and(|n| printed >= n) {
                        continue;
                    }
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
                    printed += 1;
                }
            }
        }
        if count.is_some_and(|n| printed >= n) {
            self.pending.extend_from_slice(b"\r>");
        } else {
            self.monitoring = true;
        }
    }
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
        self.sent.lock().unwrap().push(cmd.clone());
        if self.stn {
            let count = |rest: &str| rest.parse::<usize>().ok();
            if let Some(rest) = cmd.strip_prefix("STMA") {
                self.monitor(true, false, count(rest));
                return true;
            }
            if let Some(rest) = cmd.strip_prefix("STM") {
                self.monitor(false, true, count(rest));
                return true;
            }
            let lines: Vec<&str> = if cmd == "STI" {
                vec!["STN2232 v5.10.3"]
            } else if cmd == "STBC1" {
                vec!["OK"]
            } else if let Some(args) = cmd.strip_prefix("STFPA") {
                let pattern = args.split(',').next().unwrap_or("");
                self.pass = u32::from_str_radix(pattern, 16).ok();
                vec!["OK"]
            } else if cmd == "ATAR" {
                self.filter = None;
                self.pass = None;
                vec!["OK"]
            } else {
                Vec::new()
            };
            if !lines.is_empty() {
                self.pending.extend(answer(&lines));
                return true;
            }
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
            self.monitor(false, false, None);
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
// UdsOnlyCar (test_obd_on_uds.cpp): refuses the classic services and answers
// the emission data only over UDS.
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Reply {
    pub plain: Vec<&'static str>,
    pub headers: Vec<&'static str>,
}

#[derive(Default)]
pub struct UdsOnlyCar {
    /// Given out in turn, the last one stays.
    pub replies: BTreeMap<String, Vec<Reply>>,
    pub sent: Arc<Mutex<Vec<String>>>,
    pending: Vec<u8>,
    headers: bool,
}

impl UdsOnlyCar {
    pub fn count(sent: &Arc<Mutex<Vec<String>>>, cmd: &str) -> usize {
        sent.lock().unwrap().iter().filter(|s| *s == cmd).count()
    }
}

impl Transport for UdsOnlyCar {
    fn write(&mut self, data: &[u8]) -> bool {
        let cmd: String = data
            .iter()
            .filter(|&&c| c != b'\r' && c != b' ')
            .map(|&c| (c as char).to_ascii_uppercase())
            .collect();
        self.sent.lock().unwrap().push(cmd.clone());
        let refusal;
        let lines: Vec<&str> = if cmd == "ATZ" {
            self.headers = false;
            vec!["ELM327 v2.1"]
        } else if cmd == "ATH1" || cmd == "ATH0" {
            self.headers = cmd == "ATH1";
            vec!["OK"]
        } else if cmd == "ATDP" {
            vec!["ISO 15765-4 (CAN 11/500)"]
        } else if cmd.starts_with("AT") {
            vec!["OK"]
        } else if cmd.starts_with("ST") {
            vec!["?"]
        } else if cmd.len() >= 2 && cmd.starts_with('0') {
            // Every classic service is refused.
            refusal = if self.headers {
                format!("7E8 03 7F {} 11", &cmd[..2])
            } else {
                format!("7F {} 11", &cmd[..2])
            };
            vec![refusal.as_str()]
        } else {
            match self.replies.get_mut(&cmd) {
                Some(list) if !list.is_empty() => {
                    let r = list[0].clone();
                    if list.len() > 1 {
                        list.remove(0);
                    }
                    if self.headers { r.headers } else { r.plain }
                }
                _ => vec!["NO DATA"],
            }
        };
        let bytes = answer(&lines);
        self.pending.extend(bytes);
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

fn reply(plain: &[&'static str], headers: &[&'static str]) -> Reply {
    Reply {
        plain: plain.to_vec(),
        headers: headers.to_vec(),
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
// DemoAdapter: the adapter half of the C++ DemoTransport (core/DemoTransport),
// enough of it for the adapter probe and the connect tests. The car behind it
// answers 0100 from the engine (7E8) and, optionally, the gearbox (7E9).
// ---------------------------------------------------------------------------

/// Which adapter the demo pretends to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoAdapter {
    /// "ELM327 v1.5", command set of v1.4
    CloneV15,
    /// behaves like a real ELM327 v1.4b
    GenuineV14b,
    /// claims v2.1, command set of v1.4
    CloneV21,
    /// claims v2.1 and answers "OK" even to commands it does not know
    CloneOkToAll,
    /// STN chip (OBDLink), ELM327 v1.4b compatible plus ST commands
    ObdLinkStn,
}

const V10: i32 = 100;
const V11: i32 = 110;
const V12: i32 = 120;
const V13: i32 = 130;
const V13A: i32 = 131;
const V14: i32 = 140;
const V14B: i32 = 142;
const V20: i32 = 200;

struct Profile {
    version: &'static str,
    description: &'static str,
    level: i32,
    stn_id: Option<&'static str>,
    stn_device: Option<&'static str>,
    ok_to_all: bool,
}

fn profile_for(adapter: DemoAdapter) -> Profile {
    let p = |version, description, level, stn_id, stn_device, ok_to_all| Profile {
        version,
        description,
        level,
        stn_id,
        stn_device,
        ok_to_all,
    };
    match adapter {
        DemoAdapter::GenuineV14b => p(
            "ELM327 v1.4b",
            "OBDII to RS232 Interpreter",
            V14B,
            None,
            None,
            false,
        ),
        DemoAdapter::CloneV21 => p(
            "ELM327 v2.1",
            "OBDII to RS232 Interpreter",
            V14,
            None,
            None,
            false,
        ),
        DemoAdapter::CloneOkToAll => p(
            "ELM327 v2.1",
            "OBDII to RS232 Interpreter",
            V14,
            None,
            None,
            true,
        ),
        DemoAdapter::ObdLinkStn => p(
            "ELM327 v1.4b",
            "SCANTOOL.NET LLC",
            V14B,
            Some("STN1130 v5.6.19"),
            Some("OBDLink SX r4.2"),
            false,
        ),
        DemoAdapter::CloneV15 => p(
            "ELM327 v1.5",
            "OBDII to RS232 Interpreter",
            V14,
            None,
            None,
            false,
        ),
    }
}

struct Response {
    lines: Vec<String>,
    delay_ms: f64,
    prompt: bool,
}

fn resp(lines: &[&str], delay_ms: f64) -> Response {
    Response {
        lines: lines.iter().map(|s| s.to_string()).collect(),
        delay_ms,
        prompt: true,
    }
}

pub struct FakeAdapter {
    profile: Profile,
    latency: f64,
    gearbox: bool,
    output: VecDeque<(Instant, Vec<u8>)>,
    input_line: String,
    last_command: String,
    discard_line: bool,
    echo: bool,
    linefeeds: bool,
    spaces: bool,
    headers: bool,
    protocol: u32,
    auto_search: bool,
    active: bool,
    monitoring: bool,
}

impl FakeAdapter {
    pub fn new(adapter: DemoAdapter, gearbox: bool, latency: f64) -> Self {
        let mut a = Self {
            profile: profile_for(adapter),
            latency,
            gearbox,
            output: VecDeque::new(),
            input_line: String::new(),
            last_command: String::new(),
            discard_line: false,
            echo: true,
            linefeeds: false,
            spaces: true,
            headers: false,
            protocol: 0,
            auto_search: true,
            active: false,
            monitoring: false,
        };
        // Power-on greeting; Elm327::connect clears it away like a real one.
        let greeting = format!("{}\r\r>", a.profile.version);
        a.emit(greeting.into_bytes(), 0.0);
        a
    }

    fn line_end(&self) -> &'static str {
        if self.linefeeds { "\r\n" } else { "\r" }
    }

    fn supports(&self, level: i32) -> bool {
        self.profile.level >= level
    }

    fn emit(&mut self, text: Vec<u8>, delay_ms: f64) {
        let mut ready =
            Instant::now() + Duration::from_micros((delay_ms * self.latency * 1000.0) as u64);
        if let Some((last, _)) = self.output.back() {
            ready = ready.max(*last);
        }
        self.output.push_back((ready, text));
    }

    fn reset_settings(&mut self) {
        self.echo = true;
        self.linefeeds = false;
        self.spaces = true;
        self.headers = false;
        self.protocol = 0;
        self.auto_search = true;
        self.active = false;
    }

    fn unknown(&self) -> Response {
        resp(&[if self.profile.ok_to_all { "OK" } else { "?" }], 2.0)
    }

    fn ok(&self, level: i32) -> Response {
        if self.supports(level) {
            resp(&["OK"], 2.0)
        } else {
            self.unknown()
        }
    }

    fn bytes_text(&self, bytes: &[u8]) -> String {
        let parts: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
        parts.join(if self.spaces { " " } else { "" })
    }

    fn obd_request(&mut self, hex: &str) -> Response {
        let mut hex = hex.to_string();
        let mut count_hint = 0usize;
        if hex.len() % 2 == 1 {
            // A trailing single digit tells the adapter how many answers to
            // wait for (v1.3 and later).
            if !self.supports(V13) {
                return self.unknown();
            }
            count_hint = usize::from_str_radix(&hex[hex.len() - 1..], 16).unwrap_or(0);
            if count_hint == 0 {
                return self.unknown();
            }
            hex.pop();
        }
        let data: Vec<u8> = (0..hex.len() / 2)
            .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap_or(0))
            .collect();
        if data.is_empty() || data.len() > 7 {
            return resp(&["?"], 2.0);
        }
        let mut response = resp(&[], 2.0);
        if !self.active {
            if self.auto_search {
                response.lines.push("SEARCHING...".into());
                response.delay_ms += 80.0;
            } else if self.protocol != 6 {
                return match self.protocol {
                    3 | 4 => resp(&["BUS INIT: ...ERROR"], 300.0),
                    5 => resp(&["BUS INIT: ERROR"], 200.0),
                    8 | 9 | 11 | 12 => resp(&["CAN ERROR"], 150.0),
                    _ => resp(&["NO DATA"], 120.0),
                };
            }
            self.active = true;
        }
        let mut replies: Vec<(u32, Vec<u8>)> = Vec::new();
        if data == [0x01, 0x00] {
            replies.push((0x7E8, vec![0x41, 0x00, 0xBE, 0x1F, 0xA8, 0x13]));
            if self.gearbox {
                replies.push((0x7E9, vec![0x41, 0x00, 0x98, 0x18, 0x00, 0x01]));
            }
        }
        if count_hint > 0 && replies.len() > count_hint {
            replies.truncate(count_hint);
        }
        if replies.is_empty() {
            response.lines.push("NO DATA".into());
            response.delay_ms += 40.0;
            return response;
        }
        let frames = replies.len();
        for (id, payload) in &replies {
            let mut frame = vec![payload.len() as u8];
            frame.extend_from_slice(payload);
            let line = if self.headers {
                format!(
                    "{id:03X}{}{}",
                    if self.spaces { " " } else { "" },
                    self.bytes_text(&frame)
                )
            } else {
                self.bytes_text(payload)
            };
            response.lines.push(line);
        }
        // Without a count hint the adapter waits a little for further answers.
        response.delay_ms += 6.0 + frames as f64 + if count_hint > 0 { 0.0 } else { 12.0 };
        response
    }

    fn at_command(&mut self, c: &str) -> Response {
        let flag = |s: &mut Self, set: fn(&mut Self), level: i32| {
            if !s.supports(level) {
                return s.unknown();
            }
            set(s);
            resp(&["OK"], 2.0)
        };
        let is_hex_arg = |prefix: &str, digits: usize| {
            c.strip_prefix(prefix).is_some_and(|rest| {
                rest.len() == digits && rest.chars().all(|ch| ch.is_ascii_hexdigit())
            })
        };
        match c {
            "Z" | "WS" => {
                self.reset_settings();
                let version = self.profile.version;
                resp(&["", "", version], if c == "Z" { 40.0 } else { 20.0 })
            }
            "D" => {
                let keep = self.echo;
                self.reset_settings();
                self.echo = keep;
                resp(&["OK"], 2.0)
            }
            "I" => resp(&[self.profile.version], 2.0),
            "@1" => resp(&[self.profile.description], 2.0),
            "E0" => flag(self, |s| s.echo = false, V10),
            "E1" => flag(self, |s| s.echo = true, V10),
            "L0" => flag(self, |s| s.linefeeds = false, V10),
            "L1" => flag(self, |s| s.linefeeds = true, V10),
            "S0" => flag(self, |s| s.spaces = false, V13),
            "S1" => flag(self, |s| s.spaces = true, V13),
            "H0" => flag(self, |s| s.headers = false, V10),
            "H1" => flag(self, |s| s.headers = true, V10),
            "D0" | "D1" => self.ok(V13),
            "CAF0" | "CAF1" | "M0" | "M1" | "R0" | "R1" | "AL" | "NL" => self.ok(V10),
            "AT0" | "AT1" | "AT2" | "KW0" | "KW1" | "AR" => self.ok(V12),
            "FE" => self.ok(V13A),
            "CSM0" | "CSM1" => self.ok(V14),
            "JHF0" | "JHF1" | "CRA" => self.ok(V14B),
            "DP" => {
                let text = if self.auto_search {
                    if self.active {
                        "AUTO, ISO 15765-4 (CAN 11/500)"
                    } else {
                        "AUTO"
                    }
                } else {
                    "ISO 15765-4 (CAN 11/500)"
                };
                resp(&[text], 2.0)
            }
            "PC" => {
                self.active = false;
                resp(&["OK"], 2.0)
            }
            // Measured at pin 16, a little below the battery itself.
            "RV" => resp(&["14.1V"], 3.0),
            "IGN" => {
                if self.supports(V14) {
                    resp(&["ON"], 2.0)
                } else {
                    self.unknown()
                }
            }
            "AMC" => {
                if self.supports(V20) {
                    resp(&["00"], 2.0)
                } else {
                    self.unknown()
                }
            }
            "PPS" => {
                if self.supports(V11) {
                    resp(
                        &[
                            "00:FF F  01:FF F  02:FF F  03:32 F",
                            "04:01 F  05:FF F  06:F1 F  07:09 F",
                        ],
                        8.0,
                    )
                } else {
                    self.unknown()
                }
            }
            "MA" => {
                self.monitoring = true;
                Response {
                    lines: Vec::new(),
                    delay_ms: 0.0,
                    prompt: false,
                }
            }
            _ if is_hex_arg("ST", 2) || is_hex_arg("SH", 3) => resp(&["OK"], 2.0),
            _ if is_hex_arg("CRA", 3) => self.ok(V13),
            _ if c.starts_with("SP") || c.starts_with("TP") => {
                let mut rest = &c[2..];
                let mut automatic = false;
                if rest.len() == 2 && rest.starts_with('A') {
                    automatic = true;
                    rest = &rest[1..];
                }
                match u32::from_str_radix(rest, 16) {
                    Ok(p) if rest.len() == 1 && p <= 12 => {
                        self.protocol = p;
                        self.auto_search = automatic || p == 0;
                        self.active = false;
                        resp(&["OK"], 2.0)
                    }
                    _ => self.unknown(),
                }
            }
            _ => self.unknown(),
        }
    }

    fn execute(&mut self, command: &str) -> Response {
        if let Some(rest) = command.strip_prefix("AT") {
            return self.at_command(rest);
        }
        if self.profile.stn_id.is_some() && command.starts_with("ST") {
            return match command {
                "STI" => resp(&[self.profile.stn_id.unwrap_or("?")], 2.0),
                "STDI" => resp(&[self.profile.stn_device.unwrap_or("?")], 2.0),
                _ => resp(&["?"], 2.0),
            };
        }
        if !command.is_empty() && command.chars().all(|c| c.is_ascii_hexdigit()) {
            return self.obd_request(command);
        }
        resp(&["?"], 2.0)
    }

    fn handle_line(&mut self, raw: &str) {
        let mut command: String = raw
            .chars()
            .filter(|&c| c != ' ' && c != '\t')
            .map(|c| c.to_ascii_uppercase())
            .collect();
        let end = self.line_end();
        // The echo goes out before the command changes anything.
        if self.echo {
            self.emit(format!("{raw}{end}").into_bytes(), 0.0);
        }
        if command.is_empty() {
            // A bare carriage return repeats the last command.
            if self.last_command.is_empty() {
                self.emit(b">".to_vec(), 1.0);
                return;
            }
            command = self.last_command.clone();
        }
        self.last_command = command.clone();
        let r = self.execute(&command);
        if !r.prompt {
            return;
        }
        let end = self.line_end();
        let mut text = String::new();
        for line in &r.lines {
            text.push_str(line);
            text.push_str(end);
        }
        text.push_str(end);
        text.push('>');
        self.emit(text.into_bytes(), r.delay_ms);
    }
}

impl Transport for FakeAdapter {
    fn write(&mut self, data: &[u8]) -> bool {
        for &ch in data {
            if self.monitoring {
                // Any character ends monitoring; the rest of that line is lost.
                self.monitoring = false;
                let end = self.line_end();
                self.emit(format!("STOPPED{end}{end}>").into_bytes(), 1.0);
                self.discard_line = ch != b'\r';
                continue;
            }
            if self.discard_line {
                if ch == b'\r' {
                    self.discard_line = false;
                }
                continue;
            }
            if ch == b'\r' {
                let line = std::mem::take(&mut self.input_line);
                self.handle_line(&line);
            } else if ch != b'\n' && ch != 0 && self.input_line.len() < 64 {
                self.input_line.push(ch as char);
            }
        }
        true
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            let now = Instant::now();
            let mut result = Vec::new();
            while self.output.front().is_some_and(|(at, _)| *at <= now) {
                if let Some((_, text)) = self.output.pop_front() {
                    result.extend(text);
                }
            }
            if !result.is_empty() {
                return result;
            }
            if now >= deadline {
                return result;
            }
            let mut wake = deadline;
            if let Some((at, _)) = self.output.front() {
                wake = wake.min(*at);
            }
            std::thread::sleep(wake.saturating_duration_since(now));
        }
    }

    fn flush_input(&mut self) {
        // Only what the adapter has already sent is thrown away; answers still
        // on their way arrive later.
        let now = Instant::now();
        while self.output.front().is_some_and(|(at, _)| *at <= now) {
            self.output.pop_front();
        }
    }

    fn name(&self) -> String {
        "Demo".into()
    }
}

/// An Elm327 connected to the fake demo adapter on protocol 6. Every line
/// Elm327 logs (command and answer) ends up in `sent`.
pub struct DemoRig {
    pub elm: Elm327,
    pub sent: Arc<Mutex<Vec<String>>>,
    pub connected: bool,
    pub error: String,
}

impl DemoRig {
    pub fn new(adapter: DemoAdapter, gearbox: bool, latency: f64) -> Self {
        let mut elm = Elm327::new();
        elm.set_verbose(true);
        let sent = capture_log(&mut elm);
        let result = elm.connect(Box::new(FakeAdapter::new(adapter, gearbox, latency)), "6");
        Self {
            elm,
            sent,
            connected: result.is_ok(),
            error: result.err().unwrap_or_default(),
        }
    }
}

/// Answers a command itself (`Some`) or lets it through (`None`).
pub type Rule = Box<dyn FnMut(&str) -> Option<Vec<u8>> + Send>;

/// Wraps a transport and lets a test answer chosen commands itself.
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
