//! Recording a session with the adapter and playing it back.
//!
//! [`RecordingTransport`] sits between `Elm327` and the real transport and
//! writes down every write and every read with a timestamp.
//! [`ReplayTransport`] reads such a file and plays the adapter's part: `Elm327`
//! talks to it like to a serial port, and every command gets the answer that
//! was recorded for it. That makes a session with a real car reproducible on
//! the desk, which helps when hunting bugs and when someone sends in a
//! recording of their car.
//!
//! File format: JSON Lines, one object per line, UTF-8. The C++ version
//! writes and reads the same files.
//!
//! ```text
//! {"format":"caruspex-recording","version":1,"created":"2026-11-09T18:02:11Z",
//!  "transport":"COM5","adapter":"ELM327 v1.5","protocol":"...","protocolId":"6"}
//! {"t":0.512,"dir":"tx","data":"ATZ\r"}
//! {"t":0.538,"dir":"rx","data":"ATZ\r\r\rELM327 v1.5\r\r>"}
//! {"t":4.100,"dir":"note","data":"free text, ignored on replay"}
//! ```
//!
//! The first line is the header; "adapter", "protocol" and "protocolId" are
//! only present when they were known when the recording started (that is,
//! when it was started in a running session). "t" counts seconds since the
//! start of the recording. "data" carries the bytes exactly as they went over
//! the wire: bytes outside printable ASCII are written as \u00XX and stand
//! for that single byte, not for a Unicode character. A reader skips lines it
//! cannot parse, so a file cut off in the middle of its last line (the
//! program crashed, the disk was full) still plays back.

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use caruspex_i18n::l;
use caruspex_link::timing::{now_ms, sleep_ms};
use caruspex_link::{Elm327, Transport};

pub const RECORDING_FORMAT: &str = "caruspex-recording";
pub const RECORDING_VERSION: i32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordedDirection {
    Tx,
    Rx,
    Note,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordedEvent {
    /// Seconds since the start.
    pub t: f64,
    pub direction: RecordedDirection,
    /// Raw bytes.
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recording {
    pub version: i32,
    /// UTC, ISO 8601.
    pub created: String,
    /// Name of the recorded transport.
    pub transport: String,
    /// Adapter identification, if known.
    pub adapter: String,
    /// Protocol text, if known.
    pub protocol: String,
    /// "6" and so on, if known.
    pub protocol_id: String,
    pub events: Vec<RecordedEvent>,
    /// Lines that could not be read.
    pub skipped_lines: i32,
    /// The last line was cut off.
    pub truncated: bool,
}

/// What the header of a new recording says besides format and date.
#[derive(Clone, Debug, Default)]
pub struct RecordingMeta {
    pub adapter: String,
    pub protocol: String,
    /// Protocol number as the adapter counts, "6" for CAN 11 bit 500k.
    pub protocol_id: String,
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn file_name_of(path: &str) -> String {
    match path.rfind(['/', '\\']) {
        Some(slash) => path[slash + 1..].to_string(),
        None => path.to_string(),
    }
}

/// Seconds with three decimals, always with a decimal point.
fn format_seconds(ms: u64) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000)
}

/// Current time in UTC as "2026-11-09T18:02:11Z". The calendar conversion is
/// done by hand (days to civil date), the same way as in the C++ version.
fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut days = secs / 86400;
    let mut rest = secs % 86400;
    if rest < 0 {
        rest += 86400;
        days -= 1;
    }
    days += 719468;
    let era = (if days >= 0 { days } else { days - 146096 }) / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year += 1;
    }
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        (rest / 60) % 60,
        rest % 60
    )
}

// ---------------------------------------------------------------------------
// JSON, just as much as this format needs
// ---------------------------------------------------------------------------

/// Escapes a string for JSON. With `bytes` every byte outside printable ASCII
/// becomes \u00XX and means exactly that byte; this is how the raw adapter
/// traffic is kept, which need not be valid UTF-8. Without `bytes` (header
/// texts) UTF-8 passes through and only control characters are escaped.
fn json_string(s: &[u8], bytes: bool) -> Vec<u8> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = vec![b'"'];
    for &c in s {
        match c {
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\t' => out.extend_from_slice(b"\\t"),
            _ if c < 0x20 || c == 0x7F || (bytes && c >= 0x80) => {
                out.extend_from_slice(b"\\u00");
                out.push(DIGITS[(c >> 4) as usize]);
                out.push(DIGITS[(c & 0xF) as usize]);
            }
            _ => out.push(c),
        }
    }
    out.push(b'"');
    out
}

enum JsonValue {
    Text(Vec<u8>),
    Number(f64),
    Other,
}

struct FlatJsonParser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> FlatJsonParser<'a> {
    fn new(s: &'a [u8]) -> Self {
        Self { s, i: 0 }
    }

    /// Parses one object whose values are strings, numbers, booleans or null.
    /// Nested objects and arrays are not part of the format and fail.
    fn parse(&mut self) -> Option<HashMap<Vec<u8>, JsonValue>> {
        let mut out = HashMap::new();
        self.skip_space();
        if !self.take(b'{') {
            return None;
        }
        self.skip_space();
        if self.take(b'}') {
            return self.at_end().then_some(out);
        }
        loop {
            self.skip_space();
            let key = self.parse_string()?;
            self.skip_space();
            if !self.take(b':') {
                return None;
            }
            self.skip_space();
            let value = if self.peek() == b'"' {
                JsonValue::Text(self.parse_string()?)
            } else if self.peek() == b'-' || self.peek().is_ascii_digit() {
                JsonValue::Number(self.parse_number()?)
            } else if self.parse_literal() {
                JsonValue::Other
            } else {
                return None;
            };
            out.insert(key, value);
            self.skip_space();
            if self.take(b',') {
                continue;
            }
            if self.take(b'}') {
                return self.at_end().then_some(out);
            }
            return None;
        }
    }

    fn peek(&self) -> u8 {
        self.s.get(self.i).copied().unwrap_or(0)
    }

    fn take(&mut self, c: u8) -> bool {
        if self.i >= self.s.len() || self.s[self.i] != c {
            return false;
        }
        self.i += 1;
        true
    }

    fn skip_space(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\r' | b'\n') {
            self.i += 1;
        }
    }

    fn at_end(&mut self) -> bool {
        self.skip_space();
        self.i == self.s.len()
    }

    fn parse_hex4(&mut self) -> Option<u32> {
        if self.i + 4 > self.s.len() {
            return None;
        }
        let mut value = 0u32;
        for k in 0..4 {
            let v = (self.s[self.i + k] as char).to_digit(16)?;
            value = value * 16 + v;
        }
        self.i += 4;
        Some(value)
    }

    fn append_utf8(out: &mut Vec<u8>, cp: u32) {
        if cp < 0x80 {
            out.push(cp as u8);
        } else if cp < 0x800 {
            out.push((0xC0 | (cp >> 6)) as u8);
            out.push((0x80 | (cp & 0x3F)) as u8);
        } else if cp < 0x10000 {
            out.push((0xE0 | (cp >> 12)) as u8);
            out.push((0x80 | ((cp >> 6) & 0x3F)) as u8);
            out.push((0x80 | (cp & 0x3F)) as u8);
        } else {
            out.push((0xF0 | (cp >> 18)) as u8);
            out.push((0x80 | ((cp >> 12) & 0x3F)) as u8);
            out.push((0x80 | ((cp >> 6) & 0x3F)) as u8);
            out.push((0x80 | (cp & 0x3F)) as u8);
        }
    }

    fn parse_string(&mut self) -> Option<Vec<u8>> {
        if !self.take(b'"') {
            return None;
        }
        let mut out = Vec::new();
        while self.i < self.s.len() {
            let c = self.s[self.i];
            self.i += 1;
            if c == b'"' {
                return Some(out);
            }
            if c < 0x20 {
                return None;
            }
            if c != b'\\' {
                out.push(c);
                continue;
            }
            if self.i >= self.s.len() {
                return None;
            }
            let e = self.s[self.i];
            self.i += 1;
            match e {
                b'"' => out.push(b'"'),
                b'\\' => out.push(b'\\'),
                b'/' => out.push(b'/'),
                b'b' => out.push(0x08),
                b'f' => out.push(0x0C),
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'u' => {
                    let cp = self.parse_hex4()?;
                    if cp < 0x100 {
                        // One raw byte, see json_string.
                        out.push(cp as u8);
                    } else if (0xD800..0xDC00).contains(&cp) {
                        if !self.take(b'\\') || !self.take(b'u') {
                            return None;
                        }
                        let low = self.parse_hex4()?;
                        if !(0xDC00..=0xDFFF).contains(&low) {
                            return None;
                        }
                        Self::append_utf8(
                            &mut out,
                            0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00),
                        );
                    } else {
                        Self::append_utf8(&mut out, cp);
                    }
                }
                _ => return None,
            }
        }
        None // the closing quote is missing: a cut-off line
    }

    /// Decimal number with optional fraction and exponent, parsed by hand
    /// exactly like the C++ version so both read the same time stamps.
    fn parse_number(&mut self) -> Option<f64> {
        let negative = self.take(b'-');
        if !self.peek().is_ascii_digit() {
            return None;
        }
        let mut value = 0.0f64;
        while self.peek().is_ascii_digit() {
            value = value * 10.0 + f64::from(self.s[self.i] - b'0');
            self.i += 1;
        }
        if self.take(b'.') {
            if !self.peek().is_ascii_digit() {
                return None;
            }
            let mut scale = 0.1f64;
            while self.peek().is_ascii_digit() {
                value += f64::from(self.s[self.i] - b'0') * scale;
                self.i += 1;
                scale /= 10.0;
            }
        }
        if self.peek() == b'e' || self.peek() == b'E' {
            self.i += 1;
            let mut neg_exp = false;
            if self.peek() == b'+' || self.peek() == b'-' {
                neg_exp = self.s[self.i] == b'-';
                self.i += 1;
            }
            if !self.peek().is_ascii_digit() {
                return None;
            }
            let mut exponent = 0;
            while self.peek().is_ascii_digit() && exponent < 400 {
                exponent = exponent * 10 + i32::from(self.s[self.i] - b'0');
                self.i += 1;
            }
            while self.peek().is_ascii_digit() {
                self.i += 1;
            }
            for _ in 0..exponent {
                value = if neg_exp { value / 10.0 } else { value * 10.0 };
            }
        }
        Some(if negative { -value } else { value })
    }

    fn parse_literal(&mut self) -> bool {
        for word in [&b"true"[..], b"false", b"null"] {
            if self.s[self.i.min(self.s.len())..].starts_with(word) {
                self.i += word.len();
                return true;
            }
        }
        false
    }
}

fn text_field(fields: &HashMap<Vec<u8>, JsonValue>, key: &str) -> Vec<u8> {
    match fields.get(key.as_bytes()) {
        Some(JsonValue::Text(text)) => text.clone(),
        _ => Vec::new(),
    }
}

fn string_field(fields: &HashMap<Vec<u8>, JsonValue>, key: &str) -> String {
    String::from_utf8_lossy(&text_field(fields, key)).into_owned()
}

fn upper_ascii(s: &[u8]) -> Vec<u8> {
    s.to_ascii_uppercase()
}

/// The answer lines of one exchange as Elm327 would see them: without the
/// prompt, the echo of the command and "SEARCHING...".
fn answer_lines(command: &[u8], answer: &[u8]) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    let mut flush = |current: &mut Vec<u8>| {
        let line = std::mem::take(current);
        let blank = |c: &u8| *c == b' ' || *c == b'\t';
        let start = line.iter().position(|c| !blank(c));
        let line: &[u8] = match start {
            None => &[],
            Some(a) => {
                let mut end = line.len();
                while end > a && blank(&line[end - 1]) {
                    end -= 1;
                }
                &line[a..end]
            }
        };
        if line.is_empty() {
            return;
        }
        if normalize_command(line) == command {
            return;
        }
        if upper_ascii(line).starts_with(b"SEARCHING") {
            return;
        }
        lines.push(line.to_vec());
    };
    for &c in answer {
        if c == b'>' {
            continue;
        }
        if c == b'\r' || c == b'\n' {
            flush(&mut current);
        } else {
            current.push(c);
        }
    }
    flush(&mut current);
    lines
}

fn is_at_or_st(command: &[u8]) -> bool {
    command.starts_with(b"AT") || command.starts_with(b"ST")
}

/// "ATSP6", "ATSPA6", "ATTP6": the protocol digit, or 0 if it is not one.
fn protocol_digit(command: &[u8]) -> u8 {
    if !command.starts_with(b"ATSP") && !command.starts_with(b"ATTP") {
        return 0;
    }
    let arg = &command[4..];
    if arg.len() == 2 && arg[0] == b'A' {
        return b'A'; // automatic, starting at a protocol
    }
    if arg.len() != 1 || !arg[0].is_ascii_hexdigit() {
        return 0;
    }
    arg[0]
}

/// "12,3" in German, "12.3" in English.
fn one_decimal(value: f64) -> String {
    let tenths = (value * 10.0 + 0.5) as i64;
    format!("{}{}{}", tenths / 10, l(",", "."), tenths % 10)
}

// ---------------------------------------------------------------------------
// Reading and describing
// ---------------------------------------------------------------------------

/// Normalises a command for comparison: upper case, without spaces and line
/// ends. "01 0c\r" and "010C" are the same command.
pub fn normalize_command(command: &[u8]) -> Vec<u8> {
    command
        .iter()
        .filter(|c| !matches!(c, b' ' | b'\t' | b'\r' | b'\n'))
        .map(u8::to_ascii_uppercase)
        .collect()
}

/// Reads a recording from text. Fails with an error text if the content is
/// not a Caruspex recording at all; single unreadable lines are only counted
/// in `skipped_lines`.
pub fn parse_recording(text: impl AsRef<[u8]>) -> Result<Recording, String> {
    let text = text.as_ref();
    let mut out = Recording::default();
    let mut have_header = false;
    let mut start = 0;
    while start < text.len() {
        let (end, last_without_newline) = match text[start..].iter().position(|&c| c == b'\n') {
            Some(p) => (start + p, false),
            None => (text.len(), true),
        };
        let mut line = &text[start..end];
        start = end + 1;
        if line.last() == Some(&b'\r') {
            line = &line[..line.len() - 1];
        }
        if line.iter().all(|&c| c == b' ' || c == b'\t') {
            continue;
        }

        let fields = FlatJsonParser::new(line).parse();

        if !have_header {
            let Some(fields) =
                fields.filter(|f| text_field(f, "format") == RECORDING_FORMAT.as_bytes())
            else {
                return Err(l(
                    "Die Datei ist keine Caruspex-Aufnahme.",
                    "The file is not a Caruspex recording.",
                )
                .to_string());
            };
            out.version = match fields.get(&b"version"[..]) {
                Some(JsonValue::Number(n)) => *n as i32,
                _ => 0,
            };
            if out.version < 1 {
                return Err(l(
                    "Die Aufnahme nennt keine gültige Formatversion.",
                    "The recording does not state a valid format version.",
                )
                .to_string());
            }
            out.created = string_field(&fields, "created");
            out.transport = string_field(&fields, "transport");
            out.adapter = string_field(&fields, "adapter");
            out.protocol = string_field(&fields, "protocol");
            out.protocol_id = string_field(&fields, "protocolId");
            have_header = true;
            continue;
        }

        let event = fields.and_then(|fields| {
            let t = match fields.get(&b"t"[..]) {
                Some(JsonValue::Number(n)) => *n,
                _ => return None,
            };
            let data = match fields.get(&b"data"[..]) {
                Some(JsonValue::Text(d)) => d.clone(),
                _ => return None,
            };
            let direction = match text_field(&fields, "dir").as_slice() {
                b"tx" => RecordedDirection::Tx,
                b"rx" => RecordedDirection::Rx,
                b"note" => RecordedDirection::Note,
                _ => return None,
            };
            Some(RecordedEvent { t, direction, data })
        });
        match event {
            Some(event) => out.events.push(event),
            // A broken last line without a line end is what a crash while
            // writing leaves behind. Anything else broken is counted.
            None if last_without_newline => out.truncated = true,
            None => out.skipped_lines += 1,
        }
    }
    if !have_header {
        return Err(l("Die Aufnahme ist leer.", "The recording is empty.").to_string());
    }
    Ok(out)
}

/// Reads a recording from a file.
pub fn load_recording(path: &str) -> Result<Recording, String> {
    match std::fs::read(path) {
        Ok(content) => parse_recording(content),
        Err(_) => Err(format!(
            "{}{}",
            l(
                "Die Aufnahme lässt sich nicht öffnen: ",
                "Cannot open the recording: "
            ),
            path
        )),
    }
}

/// A short overview of a recording.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RecordingInfo {
    pub duration_seconds: f64,
    /// Commands sent (bare carriage returns not counted).
    pub exchanges: i32,
    pub created: String,
    pub transport: String,
    /// From the header, else from the answer to ATZ or ATI.
    pub adapter: String,
    /// From the header, else from the answer to ATDP.
    pub protocol: String,
    /// Protocol number ("6"): from the header, else from the ATSP that
    /// preceded the answered ATDP of the connect. Empty if unknown.
    pub protocol_id: String,
    pub skipped_lines: i32,
    pub truncated: bool,
}

pub fn describe_recording(recording: &Recording) -> RecordingInfo {
    let mut info = RecordingInfo {
        created: recording.created.clone(),
        transport: recording.transport.clone(),
        adapter: recording.adapter.clone(),
        protocol: recording.protocol.clone(),
        protocol_id: recording.protocol_id.clone(),
        skipped_lines: recording.skipped_lines,
        truncated: recording.truncated,
        ..RecordingInfo::default()
    };

    let mut command: Vec<u8> = Vec::new();
    let mut answer: Vec<u8> = Vec::new();
    let mut last_protocol = 0u8; // digit of the last ATSP / ATTP
    let finish =
        |info: &mut RecordingInfo, command: &[u8], answer: &[u8], last_protocol: &mut u8| {
            if command.is_empty() {
                return;
            }
            let digit = protocol_digit(command);
            if digit != 0 {
                *last_protocol = digit;
            }
            let lines = answer_lines(command, answer);
            let Some(first) = lines.first() else {
                return;
            };
            let real = first.as_slice() != b"?" && upper_ascii(first) != b"OK";
            let first_text = String::from_utf8_lossy(first).into_owned();
            if info.adapter.is_empty() && real && matches!(command, b"ATZ" | b"ATI" | b"ATWS") {
                info.adapter = first_text.clone();
            }
            if real && command == b"ATDP" {
                if recording.protocol.is_empty() {
                    info.protocol = first_text;
                }
                // Elm327::connect asks ATDP right after the protocol it set
                // with ATSP has answered, so that ATSP names the protocol.
                if recording.protocol_id.is_empty()
                    && *last_protocol != 0
                    && *last_protocol != b'A'
                    && *last_protocol != b'0'
                {
                    info.protocol_id = (*last_protocol as char).to_string();
                }
            }
        };
    for e in &recording.events {
        info.duration_seconds = info.duration_seconds.max(e.t);
        match e.direction {
            RecordedDirection::Tx => {
                finish(&mut info, &command, &answer, &mut last_protocol);
                command = normalize_command(&e.data);
                answer.clear();
                if !command.is_empty() {
                    info.exchanges += 1;
                }
            }
            RecordedDirection::Rx => answer.extend_from_slice(&e.data),
            RecordedDirection::Note => {}
        }
    }
    finish(&mut info, &command, &answer, &mut last_protocol);
    info
}

/// The same as a few lines of text in the active language.
pub fn recording_summary(info: &RecordingInfo) -> String {
    let unknown = l("unbekannt", "unknown");
    let or_unknown = |s: &str| {
        if s.is_empty() {
            unknown.to_string()
        } else {
            s.to_string()
        }
    };
    let mut text = String::new();
    text += &format!(
        "{}{} s\n",
        l("Dauer: ", "Duration: "),
        one_decimal(info.duration_seconds)
    );
    text += &format!("{}{}\n", l("Befehle: ", "Commands: "), info.exchanges);
    text += &format!(
        "{}{}\n",
        l("Adapter: ", "Adapter: "),
        or_unknown(&info.adapter)
    );
    text += &format!(
        "{}{}\n",
        l("Protokoll: ", "Protocol: "),
        or_unknown(&info.protocol)
    );
    if !info.transport.is_empty() {
        text += &format!("{}{}\n", l("Verbindung: ", "Connection: "), info.transport);
    }
    if !info.created.is_empty() {
        text += &format!("{}{}\n", l("Aufgenommen: ", "Recorded: "), info.created);
    }
    if info.skipped_lines > 0 {
        text += &format!(
            "{}{}\n",
            l(
                "Nicht lesbare Zeilen übersprungen: ",
                "Unreadable lines skipped: "
            ),
            info.skipped_lines
        );
    }
    if info.truncated {
        text += l(
            "Die letzte Zeile ist abgeschnitten und wurde ausgelassen.\n",
            "The last line is cut off and was left out.\n",
        );
    }
    text
}

// ---------------------------------------------------------------------------
// Recording
// ---------------------------------------------------------------------------

struct Shared {
    inner: Mutex<Option<Box<dyn Transport>>>,
    file: Mutex<Option<File>>,
    path: String,
    error: String,
    start_ms: u64,
    exchanges: AtomicUsize,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Shared {
    fn write_line(&self, line: &[u8]) {
        if let Some(file) = lock(&self.file).as_mut() {
            // Written in one piece, line by line: if the program dies,
            // everything up to the last complete line is still in the file.
            let _ = file.write_all(line);
            let _ = file.flush();
        }
    }

    fn write_event(&self, direction: &str, data: &[u8], bytes: bool) {
        let mut line = format!(
            "{{\"t\":{},\"dir\":\"{direction}\",\"data\":",
            format_seconds(now_ms().saturating_sub(self.start_ms))
        )
        .into_bytes();
        line.extend(json_string(data, bytes));
        line.extend_from_slice(b"}\n");
        self.write_line(&line);
    }

    fn release(&self) -> Option<Box<dyn Transport>> {
        *lock(&self.file) = None;
        lock(&self.inner).take()
    }
}

// Transports living inside an Elm327 are only reachable as `dyn Transport`.
// To find out whether one of them is a recording (and to take the recording
// out again) without downcasting, every RecordingTransport registers itself
// under the address of its heap allocation.
static REGISTRY: Mutex<Option<HashMap<usize, Weak<Shared>>>> = Mutex::new(None);

fn registry_insert(address: usize, shared: &Arc<Shared>) {
    lock(&REGISTRY)
        .get_or_insert_with(HashMap::new)
        .insert(address, Arc::downgrade(shared));
}

fn registry_remove(address: usize) {
    if let Some(map) = lock(&REGISTRY).as_mut() {
        map.remove(&address);
    }
}

fn registry_find(transport: &dyn Transport) -> Option<Arc<Shared>> {
    let address = transport as *const dyn Transport as *const () as usize;
    lock(&REGISTRY)
        .as_ref()
        .and_then(|map| map.get(&address))
        .and_then(Weak::upgrade)
}

/// A cheap reference to a running recording, for the status line and for
/// notes. Stays valid after the recording has ended.
#[derive(Clone)]
pub struct RecordingHandle {
    shared: Arc<Shared>,
}

impl RecordingHandle {
    pub fn is_recording(&self) -> bool {
        lock(&self.shared.file).is_some()
    }
    pub fn error(&self) -> &str {
        &self.shared.error
    }
    pub fn path(&self) -> &str {
        &self.shared.path
    }
    /// Number of commands written so far.
    pub fn exchanges(&self) -> i32 {
        self.shared.exchanges.load(Ordering::Relaxed) as i32
    }
    /// Adds a free-text line to the recording (replay ignores it).
    pub fn note(&self, text: &str) {
        self.shared.write_event("note", text.as_bytes(), false);
    }
}

/// Wraps another transport and writes every write and every non-empty read
/// into a file. Everything is passed through unchanged; if the file cannot
/// be opened, the wrapper simply passes through without recording (check
/// `is_recording()`).
///
/// `new` returns a `Box` on purpose: [`active_recording`] and
/// [`RecordingTransport::unwrap`] find the recording again by the address of
/// the allocation, so do not move it out of the box.
pub struct RecordingTransport {
    shared: Arc<Shared>,
}

impl RecordingTransport {
    pub fn new(
        inner: Box<dyn Transport>,
        path: &str,
        meta: RecordingMeta,
    ) -> Box<RecordingTransport> {
        let start_ms = now_ms();
        let transport_name = inner.name();
        let mut error = String::new();
        let file = match File::create(path) {
            Ok(file) => Some(file),
            Err(_) => {
                error = format!(
                    "{}{}",
                    l(
                        "Die Aufnahme lässt sich nicht anlegen: ",
                        "Cannot create the recording: "
                    ),
                    path
                );
                None
            }
        };
        let shared = Arc::new(Shared {
            inner: Mutex::new(Some(inner)),
            file: Mutex::new(file),
            path: path.to_string(),
            error,
            start_ms,
            exchanges: AtomicUsize::new(0),
        });
        if lock(&shared.file).is_some() {
            let mut header = Vec::new();
            header.extend_from_slice(b"{\"format\":");
            header.extend(json_string(RECORDING_FORMAT.as_bytes(), false));
            header.extend_from_slice(format!(",\"version\":{RECORDING_VERSION}").as_bytes());
            header.extend_from_slice(b",\"created\":");
            header.extend(json_string(utc_now().as_bytes(), false));
            header.extend_from_slice(b",\"transport\":");
            header.extend(json_string(transport_name.as_bytes(), false));
            for (key, value) in [
                ("adapter", &meta.adapter),
                ("protocol", &meta.protocol),
                ("protocolId", &meta.protocol_id),
            ] {
                if !value.is_empty() {
                    header.extend_from_slice(format!(",\"{key}\":").as_bytes());
                    header.extend(json_string(value.as_bytes(), false));
                }
            }
            header.extend_from_slice(b"}\n");
            shared.write_line(&header);
        }
        let boxed = Box::new(RecordingTransport { shared });
        registry_insert(&*boxed as *const RecordingTransport as usize, &boxed.shared);
        boxed
    }

    pub fn handle(&self) -> RecordingHandle {
        RecordingHandle {
            shared: self.shared.clone(),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.handle().is_recording()
    }
    pub fn error(&self) -> &str {
        &self.shared.error
    }
    pub fn path(&self) -> &str {
        &self.shared.path
    }
    /// Number of commands written so far.
    pub fn exchanges(&self) -> i32 {
        self.handle().exchanges()
    }
    /// Adds a free-text line to the recording (replay ignores it).
    pub fn note(&self, text: &str) {
        self.handle().note(text);
    }

    /// Ends the recording and hands back the wrapped transport.
    pub fn release(&mut self) -> Option<Box<dyn Transport>> {
        self.shared.release()
    }

    /// If `transport` is a RecordingTransport, ends the recording and returns
    /// the transport inside it; otherwise returns `transport` unchanged.
    pub fn unwrap(transport: Box<dyn Transport>) -> Box<dyn Transport> {
        if let Some(shared) = registry_find(&*transport)
            && let Some(inner) = shared.release()
        {
            return inner;
        }
        transport
    }
}

impl Drop for RecordingTransport {
    fn drop(&mut self) {
        registry_remove(self as *const RecordingTransport as usize);
    }
}

impl Transport for RecordingTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        self.shared.write_event("tx", data, true);
        if !normalize_command(data).is_empty() {
            self.shared.exchanges.fetch_add(1, Ordering::Relaxed);
        }
        match lock(&self.shared.inner).as_mut() {
            Some(inner) => inner.write(data),
            None => false,
        }
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let chunk = match lock(&self.shared.inner).as_mut() {
            Some(inner) => inner.read(timeout_ms),
            None => return Vec::new(),
        };
        if !chunk.is_empty() {
            self.shared.write_event("rx", &chunk, true);
        }
        chunk
    }

    fn flush_input(&mut self) {
        if let Some(inner) = lock(&self.shared.inner).as_mut() {
            inner.flush_input();
        }
    }

    fn name(&self) -> String {
        lock(&self.shared.inner)
            .as_ref()
            .map(|t| t.name())
            .unwrap_or_default()
    }
}

/// The running recording of this session, if any.
pub fn active_recording(elm: &Elm327) -> Option<RecordingHandle> {
    elm.transport()
        .and_then(registry_find)
        .map(|shared| RecordingHandle { shared })
}

/// Starts a recording on an Elm327 that is already connected, without
/// reconnecting. A recording started in the middle of a session lacks the
/// connect sequence, so a few harmless commands follow at once (ATI, the
/// format settings connect already made, ATDP and 0100) and land in the
/// file. That way the recording can be replayed on its own. Must be called
/// from the thread that uses the Elm327, between two requests.
pub fn start_recording(elm: &mut Elm327, path: &str) -> Result<RecordingHandle, String> {
    if !elm.connected() || elm.transport().is_none() {
        return Err(l(
            "Keine Verbindung zum Adapter.",
            "No connection to the adapter.",
        )
        .to_string());
    }
    if active_recording(elm).is_some() {
        return Err(l(
            "Es läuft schon eine Aufnahme.",
            "A recording is already running.",
        )
        .to_string());
    }
    let meta = RecordingMeta {
        adapter: elm.version().to_string(),
        protocol: elm.protocol_text().to_string(),
        protocol_id: if elm.protocol_id() != '0' {
            elm.protocol_id().to_string()
        } else {
            String::new()
        },
    };

    let mut error = String::new();
    let mut started: Option<RecordingHandle> = None;
    elm.wrap_transport(|inner| {
        let mut recording = RecordingTransport::new(inner, path, meta);
        if !recording.is_recording() {
            error = recording.error().to_string();
            return recording.release();
        }
        started = Some(recording.handle());
        Some(recording as Box<dyn Transport>)
    });
    let Some(started) = started else {
        return Err(error);
    };

    // The connect sequence is missing from this recording. These commands
    // stand in for it, in the order Elm327::connect sends them: each one
    // only reads or sets what connect has set already (the timing to the
    // level in effect), and the header setting of the session is restored
    // at the end. With them a replay finds the identification, the format
    // settings, the timing, an answer to 0100 and the protocol, which is all
    // Elm327::connect needs.
    started.note("recording started in a running session");
    let headers = elm.headers_on();
    elm.send("ATI", 1500);
    elm.send("ATE0", 1500);
    elm.send("ATL0", 1500);
    elm.send("ATS0", 1500);
    elm.send("ATH0", 1500);
    if elm.adaptive_timing_level() > 0 {
        elm.send(&format!("ATAT{}", elm.adaptive_timing_level()), 1500);
    }
    if elm.is_stn() {
        elm.send("STI", 1500);
    }
    elm.request("0100", 0x01, 6000);
    elm.send("ATDP", 1500);
    if headers {
        elm.set_headers(true);
    }
    started.note("session continues");
    Ok(started)
}

/// Ends a recording started with `start_recording` (or by wrapping the
/// transport before connect). Returns false if none was running.
pub fn stop_recording(elm: &mut Elm327) -> bool {
    if active_recording(elm).is_none() {
        return false;
    }
    elm.wrap_transport(|transport| Some(RecordingTransport::unwrap(transport)));
    true
}

// ---------------------------------------------------------------------------
// Replay
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplayPacing {
    /// Every answer is there at once.
    #[default]
    Fast,
    /// Answers arrive with the recorded delays.
    RealTime,
}

struct Chunk {
    /// Seconds after the command.
    offset: f64,
    data: Vec<u8>,
}

struct Exchange {
    /// Normalised.
    command: Vec<u8>,
    answer: Vec<Chunk>,
}

struct Pending {
    ready_at_ms: u64,
    data: Vec<u8>,
}

/// Plays a recording back in place of a real adapter. On every write it looks
/// for the next recorded command that matches (see [`normalize_command`]),
/// first from the current position a bounded number of steps forward, then
/// from the start of the recording. The second search is what lets a live
/// display poll longer than it did while recording: the recorded polls simply
/// repeat. A command that was never recorded gets the answer an ELM327 would
/// give to something it cannot serve: "?" for AT and ST commands and "NO
/// DATA" for requests. Three AT commands are answered the way any ELM327
/// would, since a recording started in a running session does not contain
/// them: ATZ, ATWS and ATI give the recorded adapter identification, and ATSP
/// / ATTP give "OK". If the recorded protocol is known, a request after an
/// unrecorded ATSP for a different protocol gets "UNABLE TO CONNECT", so
/// `Elm327::connect` steps on to the recorded protocol as it would on the car.
pub struct ReplayTransport {
    recording: Recording,
    display_name: String,
    pacing: ReplayPacing,
    open: bool,
    error: String,
    adapter_id: String,
    protocol_id: String,
    exchanges: Vec<Exchange>,
    position: usize,
    pending: VecDeque<Pending>,
    wrong_protocol: bool,
    matched: i32,
    unmatched: i32,
    wrapped: i32,
}

impl ReplayTransport {
    /// How far forward from the current position a command is looked for
    /// before the search starts over at the beginning.
    pub const SEARCH_WINDOW: usize = 64;

    fn empty(recording: Recording, display_name: String, pacing: ReplayPacing) -> Self {
        Self {
            recording,
            display_name,
            pacing,
            open: false,
            error: String::new(),
            adapter_id: String::new(),
            protocol_id: String::new(),
            exchanges: Vec::new(),
            position: 0,
            pending: VecDeque::new(),
            wrong_protocol: false,
            matched: 0,
            unmatched: 0,
            wrapped: 0,
        }
    }

    /// Loads the file; check `is_open()` and `error()` afterwards.
    pub fn new(path: &str, pacing: ReplayPacing) -> Self {
        let mut replay = Self::empty(Recording::default(), file_name_of(path), pacing);
        match load_recording(path) {
            Ok(recording) => {
                replay.recording = recording;
                replay.open = true;
                replay.build();
            }
            Err(error) => replay.error = error,
        }
        replay
    }

    /// Plays a recording already in memory. `display_name` shows up in `name()`.
    pub fn from_recording(
        recording: Recording,
        display_name: impl Into<String>,
        pacing: ReplayPacing,
    ) -> Self {
        let mut replay = Self::empty(recording, display_name.into(), pacing);
        replay.open = true;
        replay.build();
        replay
    }

    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn error(&self) -> &str {
        &self.error
    }
    pub fn recording(&self) -> &Recording {
        &self.recording
    }
    pub fn info(&self) -> RecordingInfo {
        describe_recording(&self.recording)
    }
    /// The protocol order to hand to `Elm327::connect` for this recording:
    /// the recorded protocol if known, otherwise `fallback`.
    pub fn protocol_order(&self, fallback: &str) -> String {
        if self.protocol_id.is_empty() {
            fallback.to_string()
        } else {
            self.protocol_id.clone()
        }
    }

    /// Statistics, mainly for tests and the log.
    pub fn matched(&self) -> i32 {
        self.matched
    }
    pub fn unmatched(&self) -> i32 {
        self.unmatched
    }
    pub fn wrapped(&self) -> i32 {
        self.wrapped
    }

    fn build(&mut self) {
        self.exchanges.clear();
        let mut sent_at = 0.0f64;
        for e in &self.recording.events {
            match e.direction {
                RecordedDirection::Tx => {
                    self.exchanges.push(Exchange {
                        command: normalize_command(&e.data),
                        answer: Vec::new(),
                    });
                    sent_at = e.t;
                }
                RecordedDirection::Rx => {
                    // Whatever arrived before the first command (a greeting)
                    // has no command to belong to and is left out.
                    if let Some(last) = self.exchanges.last_mut() {
                        last.answer.push(Chunk {
                            offset: (e.t - sent_at).max(0.0),
                            data: e.data.clone(),
                        });
                    }
                }
                RecordedDirection::Note => {}
            }
        }
        let info = describe_recording(&self.recording);
        self.adapter_id = info.adapter;
        self.protocol_id = info.protocol_id;
        self.position = 0;
    }

    fn find(&mut self, command: &[u8]) -> Option<usize> {
        let count = self.exchanges.len();
        let window_end = count.min(self.position + Self::SEARCH_WINDOW);
        // Forward from where the replay stands, a bounded distance.
        for i in self.position..window_end {
            if self.exchanges[i].command == command {
                self.position = i + 1;
                return Some(i);
            }
        }
        // From the start: polls that run longer than the recording repeat.
        for i in 0..self.position.min(count) {
            if self.exchanges[i].command == command {
                self.wrapped += 1;
                self.position = i + 1;
                return Some(i);
            }
        }
        // Last resort: further ahead than the window reaches.
        for i in window_end..count {
            if self.exchanges[i].command == command {
                self.position = i + 1;
                return Some(i);
            }
        }
        None
    }

    fn fallback_answer(&self, command: &[u8]) -> Vec<u8> {
        let end: &[u8] = b"\r\r>";
        let with_end = |head: &[u8]| [head, end].concat();
        if command.is_empty() {
            return b">".to_vec();
        }
        if command == b"ATZ" || command == b"ATWS" {
            return if self.adapter_id.is_empty() {
                with_end(b"?")
            } else {
                [b"\r\r", self.adapter_id.as_bytes(), end].concat()
            };
        }
        if command == b"ATI" {
            return if self.adapter_id.is_empty() {
                with_end(b"?")
            } else {
                with_end(self.adapter_id.as_bytes())
            };
        }
        if protocol_digit(command) != 0 {
            return with_end(b"OK");
        }
        if is_at_or_st(command) {
            return with_end(b"?");
        }
        if self.wrong_protocol {
            return with_end(b"UNABLE TO CONNECT");
        }
        with_end(b"NO DATA")
    }
}

impl Transport for ReplayTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        if !self.open {
            return false;
        }
        let command = normalize_command(data);
        let now = now_ms();
        // While a wrong protocol is selected, requests do not reach any
        // recorded answer, so they do not move the position either.
        let refused = self.wrong_protocol && !command.is_empty() && !is_at_or_st(&command);
        let index = if refused { None } else { self.find(&command) };

        let digit = protocol_digit(&command);
        if digit != 0 {
            // Only an unrecorded protocol switch needs the bookkeeping: a
            // recorded one comes with its recorded consequences.
            self.wrong_protocol = index.is_none()
                && !self.protocol_id.is_empty()
                && digit != b'A'
                && digit != b'0'
                && self.protocol_id.as_bytes() != [digit];
        }

        let Some(index) = index else {
            self.unmatched += 1;
            let answer = self.fallback_answer(&command);
            self.pending.push_back(Pending {
                ready_at_ms: now,
                data: answer,
            });
            return true;
        };
        self.matched += 1;
        for chunk in &self.exchanges[index].answer {
            let delay = if self.pacing == ReplayPacing::RealTime {
                (chunk.offset * 1000.0) as u64
            } else {
                0
            };
            self.pending.push_back(Pending {
                ready_at_ms: now + delay,
                data: chunk.data.clone(),
            });
        }
        true
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let deadline = now_ms() + timeout_ms;
        loop {
            let mut result = Vec::new();
            let now = now_ms();
            while self.pending.front().is_some_and(|p| p.ready_at_ms <= now) {
                if let Some(p) = self.pending.pop_front() {
                    result.extend(p.data);
                }
            }
            if !result.is_empty() {
                return result;
            }
            if now >= deadline {
                return Vec::new();
            }
            // Like a quiet serial port: wait until something is due or the
            // timeout is over.
            let mut wake = deadline;
            if let Some(p) = self.pending.front()
                && p.ready_at_ms < wake
            {
                wake = p.ready_at_ms;
            }
            sleep_ms(wake - now);
        }
    }

    fn flush_input(&mut self) {
        // Elm327 flushes right before every command; whatever is still queued
        // belongs to the previous one.
        self.pending.clear();
    }

    fn name(&self) -> String {
        format!("{}{}", l("Aufnahme: ", "Recording: "), self.display_name)
    }
}
