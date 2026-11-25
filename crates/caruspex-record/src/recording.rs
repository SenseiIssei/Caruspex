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

use caruspex_i18n::l;

// The JSON Lines format, writing a recording and playing one back.
mod json;
mod recorder;
mod replay;

use json::{FlatJsonParser, JsonValue, json_string, string_field, text_field};

pub use recorder::{
    RecordingHandle, RecordingTransport, active_recording, start_recording, stop_recording,
};
pub use replay::{ReplayPacing, ReplayTransport};

pub(crate) const RECORDING_FORMAT: &str = "caruspex-recording";
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

/// Whether a recorded command answers the one the replay is asked. Besides
/// the same command: the answer count digit after a request (ELM327 v1.3 and
/// later, "010C0D2"). Caruspex adds it on a real adapter once it knows how
/// many answers come, never on a replay (`count_hint_allowed`), so a
/// recording may hold the request with the digit where the replay asks
/// without it; the answer is the same.
fn same_request(recorded: &[u8], asked: &[u8]) -> bool {
    if recorded == asked {
        return true;
    }
    recorded.len() == asked.len() + 1
        && asked.len() >= 4
        && asked.len().is_multiple_of(2)
        && recorded.starts_with(asked)
        && recorded.iter().all(u8::is_ascii_hexdigit)
        && recorded.last() != Some(&b'0')
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
pub(crate) fn normalize_command(command: &[u8]) -> Vec<u8> {
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

/// A recording as the file the recording transport writes, header first.
/// `parse_recording` reads it back to the same events.
pub fn write_recording(recording: &Recording) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"{\"format\":");
    out.extend(json_string(RECORDING_FORMAT.as_bytes(), false));
    out.extend_from_slice(format!(",\"version\":{}", recording.version.max(1)).as_bytes());
    for (key, value) in [
        ("created", &recording.created),
        ("transport", &recording.transport),
        ("adapter", &recording.adapter),
        ("protocol", &recording.protocol),
        ("protocolId", &recording.protocol_id),
    ] {
        if !value.is_empty() {
            out.extend_from_slice(format!(",\"{key}\":").as_bytes());
            out.extend(json_string(value.as_bytes(), false));
        }
    }
    out.extend_from_slice(b"}\n");
    for e in &recording.events {
        let direction = match e.direction {
            RecordedDirection::Tx => "tx",
            RecordedDirection::Rx => "rx",
            RecordedDirection::Note => "note",
        };
        let ms = (e.t.max(0.0) * 1000.0).round() as u64;
        out.extend_from_slice(
            format!(
                "{{\"t\":{},\"dir\":\"{direction}\",\"data\":",
                format_seconds(ms)
            )
            .as_bytes(),
        );
        out.extend(json_string(&e.data, e.direction != RecordedDirection::Note));
        out.extend_from_slice(b"}\n");
    }
    out
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
