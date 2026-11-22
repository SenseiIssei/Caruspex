//! ELM327 driver: initialisation, protocol selection, commands, response parsing.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use caruspex_i18n::l;

use crate::timing::{now_ms, sleep_ms};
use crate::transport::{IsoTpBatch, LogFn, Transport};

mod j1939;
mod stn;
pub use j1939::parse_j1939_line;
pub use stn::{StnBus, split_batch_answer, stn_buses};

/// Default wait of [`Elm327::send`] in milliseconds.
pub const DEFAULT_SEND_TIMEOUT_MS: u64 = 5000;
/// Default wait of [`Elm327::request`] in milliseconds.
pub const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 6000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ElmStatus {
    #[default]
    Ok,
    /// Control unit does not know the service.
    NoData,
    /// No contact with the vehicle.
    BusError,
    /// Adapter does not answer at all / not an ELM327.
    AdapterError,
    /// Control unit refuses (7F ...).
    Negative,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ElmResult {
    pub status: ElmStatus,
    /// Raw lines.
    pub lines: Vec<String>,
    /// Decoded payload without the service byte.
    pub messages: Vec<Vec<u8>>,
    /// Parallel to messages: which control unit sent each one. Only filled
    /// while headers are switched on (`Elm327::set_headers`), otherwise every
    /// entry is empty. Format: "7E8" on CAN 11 bit, "18DAF110" on CAN 29 bit,
    /// the source byte as "10" on K-line and J1850.
    pub sources: Vec<String>,
    /// Plain-text message on errors.
    pub message: String,
}

impl ElmResult {
    pub fn ok(&self) -> bool {
        self.status == ElmStatus::Ok
    }
}

/// How the adapter prints a response line while headers are on (ATH1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HeaderFormat {
    /// Headers off: payload only, multi-frame CAN as "0:", "1:" lines.
    #[default]
    None,
    /// "7E8 06 41 00 BE 3F A8 13": 11-bit ID, then the ISO-TP frame.
    Can11,
    /// "18DAF110 06 41 00 ...": 29-bit ID, then the ISO-TP frame.
    Can29,
    /// ISO 9141-2 and ISO 14230 (KWP): 3 or 4 header bytes, data, checksum.
    KLine,
    /// SAE J1850 PWM/VPW: 3 header bytes, data, CRC.
    J1850,
}

/// Order suited to VAG: K-Line first (A2), then CAN (A4 B7).
pub const PROTOCOL_ORDER_KLINE_FIRST: &str = "54312678";
pub const PROTOCOL_ORDER_CAN_FIRST: &str = "67891543";

/// Adaptive timing of the adapter (ATAT). The adapter measures how fast the
/// control units answer and shortens its own wait accordingly. ATAT1 is the
/// power-on default and careful; ATAT2 waits less and so gets more requests
/// through per second, at the risk of missing a slow control unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AdaptiveTiming {
    /// ATAT1
    #[default]
    Standard,
    /// ATAT2, back to ATAT1 if refused or if answers go missing
    Aggressive,
    /// ATAT2 on STN chips (OBDLink and others), ATAT1 on everything else
    Auto,
}

/// One complete message as it came off the bus, service byte first, and who
/// sent it (empty while headers are off).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ElmMessage {
    pub source: String,
    pub bytes: Vec<u8>,
}

const ERRORS: [&str; 12] = [
    "UNABLE TO CONNECT",
    "BUS INIT: ERROR",
    "BUS INIT: ...ERROR",
    "BUS ERROR",
    "CAN ERROR",
    "DATA ERROR",
    "BUS BUSY",
    "FB ERROR",
    "LV RESET",
    "STOPPED",
    "ERR",
    "?",
];

/// True when a reply is mostly bytes no ELM327 sends: the replacement
/// character of invalid UTF-8 and control characters. That is what a wrong
/// baud rate looks like.
fn reply_is_garbled(text: &str) -> bool {
    let total = text.chars().count();
    if total == 0 {
        return false;
    }
    let bad = text
        .chars()
        .filter(|c| {
            *c == '\u{FFFD}'
                || (c.is_control() && *c != '\r' && *c != '\n' && *c != '\t')
                || !c.is_ascii()
        })
        .count();
    bad * 3 >= total
}

/// True when the reply to ATZ holds no identification, only STOPPED or
/// nothing: the adapter was busy and dropped the reset.
fn reset_was_interrupted(lines: &[String]) -> bool {
    let useful: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    !useful.is_empty()
        && useful
            .iter()
            .all(|l| l.trim().eq_ignore_ascii_case("STOPPED"))
}

// ---------------------------------------------------------------------------
// Small string helpers, ASCII like the C locale
// ---------------------------------------------------------------------------

/// `isspace` of the C locale.
fn is_c_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0B' | '\x0C' | '\r')
}

/// Two upper-case hex digits, for negative response codes.
fn hex_byte(value: u8) -> String {
    format!("{value:02X}")
}

fn trim(s: &str) -> &str {
    s.trim_matches(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
}

fn to_upper(s: &str) -> String {
    s.to_ascii_uppercase()
}

fn strip_spaces(s: &str) -> String {
    s.chars().filter(|c| !is_c_space(*c)).collect()
}

/// `strip_spaces(to_upper(s))` in one go: one allocation instead of two.
fn compact_upper(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    out.extend(
        s.chars()
            .filter(|c| !is_c_space(*c))
            .map(|c| c.to_ascii_uppercase()),
    );
    out
}

/// `compact_upper(a) == compact_upper(b)` without building either. Every
/// answer line is compared with the echo, so this runs once per line.
fn same_compact(a: &str, b: &str) -> bool {
    fn squeeze(s: &str) -> impl Iterator<Item = u8> + '_ {
        s.bytes()
            .filter(|c| !is_c_space(char::from(*c)))
            .map(|c| c.to_ascii_uppercase())
    }
    squeeze(a).eq(squeeze(b))
}

/// `to_upper(s).starts_with(prefix)` for an upper-case ASCII prefix.
fn starts_with_upper(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

/// The line (upper case) is the error `err` alone or followed by a space or
/// a colon ("BUS INIT: ...ERROR", "CAN ERROR 1").
fn is_error_line(line: &str, err: &str) -> bool {
    starts_with_upper(line, err)
        && matches!(
            line.as_bytes().get(err.len()),
            None | Some(b' ') | Some(b':')
        )
}

/// True if the adapter confirmed a setting with a line that says "OK".
fn answered_ok(result: &ElmResult) -> bool {
    if result.status == ElmStatus::AdapterError {
        return false;
    }
    result
        .lines
        .iter()
        .any(|line| line.eq_ignore_ascii_case("OK"))
}

fn is_hex_line(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit() || is_c_space(c))
}

fn all_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn protocol_name(id: char) -> &'static str {
    match id {
        '1' => "SAE J1850 PWM",
        '2' => "SAE J1850 VPW",
        '3' => l(
            "ISO 9141-2 (K-Line, 5-Baud-Init)",
            "ISO 9141-2 (K-Line, 5-baud init)",
        ),
        '4' => l(
            "ISO 14230-4 KWP (5-Baud-Init)",
            "ISO 14230-4 KWP (5-baud init)",
        ),
        '5' => l("ISO 14230-4 KWP (Fast-Init)", "ISO 14230-4 KWP (fast init)"),
        '6' => l(
            "ISO 15765-4 CAN (11 Bit, 500 kBaud)",
            "ISO 15765-4 CAN (11 bit, 500 kbaud)",
        ),
        '7' => l(
            "ISO 15765-4 CAN (29 Bit, 500 kBaud)",
            "ISO 15765-4 CAN (29 bit, 500 kbaud)",
        ),
        '8' => l(
            "ISO 15765-4 CAN (11 Bit, 250 kBaud)",
            "ISO 15765-4 CAN (11 bit, 250 kbaud)",
        ),
        '9' => l(
            "ISO 15765-4 CAN (29 Bit, 250 kBaud)",
            "ISO 15765-4 CAN (29 bit, 250 kbaud)",
        ),
        'A' => l(
            "SAE J1939 (29 Bit, 250 kBaud)",
            "SAE J1939 (29 bit, 250 kbaud)",
        ),
        _ => l("unbekannt", "unknown"),
    }
}

/// Every hex digit in the text, two at a time. Anything else is skipped; an
/// odd last digit is dropped.
pub fn hex_to_bytes(text: &str) -> Vec<u8> {
    // One pass and one allocation: this runs for every answer line.
    let mut out = Vec::with_capacity(text.len() / 2);
    let mut high: Option<u8> = None;
    for d in text.chars().filter_map(|c| c.to_digit(16)) {
        match high.take() {
            Some(h) => out.push((h << 4) | d as u8),
            None => high = Some(d as u8),
        }
    }
    out
}

/// Header format the adapter uses for a protocol number ('1' to '9').
pub fn header_format_for(protocol_id: char) -> HeaderFormat {
    match protocol_id {
        '1' | '2' => HeaderFormat::J1850,
        '3' | '4' | '5' => HeaderFormat::KLine,
        '6' | '8' => HeaderFormat::Can11,
        '7' | '9' => HeaderFormat::Can29,
        _ => HeaderFormat::None,
    }
}

fn is_indexed_line(line: &str) -> bool {
    let b = line.as_bytes();
    b.len() > 2 && b[1] == b':' && b[0].is_ascii_hexdigit()
}

// Headers off. Single-frame responses are one line each, so several control
// units simply give several lines. Multi-frame CAN responses arrive numbered
// ("0:", "1:" ...), preceded by a line with the total length in three hex
// digits. The numbered lines start directly with the service byte, so
// nothing may be cut off. A new length line, or a "0:" once the announced
// length is complete, starts the next message: that is how the responses of
// two control units are kept apart. The index wraps from F to 0 on long
// messages, which is why "0:" alone is not enough.
fn split_without_headers<S: AsRef<str>>(lines: &[S]) -> Vec<ElmMessage> {
    struct State {
        out: Vec<ElmMessage>,
        collected: Vec<u8>,
        announced: i64,
        collecting: bool,
    }
    impl State {
        fn flush(&mut self) {
            if self.collecting && !self.collected.is_empty() {
                if self.announced > 0 && self.collected.len() as i64 > self.announced {
                    self.collected.truncate(self.announced as usize);
                }
                self.out.push(ElmMessage {
                    source: String::new(),
                    bytes: std::mem::take(&mut self.collected),
                });
            }
            self.collected.clear();
            self.collecting = false;
            self.announced = -1;
        }
    }
    let mut st = State {
        out: Vec::new(),
        collected: Vec::new(),
        announced: -1,
        collecting: false,
    };

    for line in lines {
        let line = line.as_ref();
        if is_indexed_line(line) {
            let first = line.as_bytes()[0] == b'0';
            let complete = st.announced > 0 && st.collected.len() as i64 >= st.announced;
            if first && st.collecting && !st.collected.is_empty() && (st.announced < 0 || complete)
            {
                st.flush();
            }
            st.collecting = true;
            st.collected.extend(hex_to_bytes(&line[2..]));
        } else if is_hex_line(line) {
            let digits = strip_spaces(line);
            // Exactly three digits: a real message always has whole bytes.
            if digits.len() == 3 {
                // Length line of the next multi-frame message.
                st.flush();
                st.announced = i64::from_str_radix(&digits, 16).unwrap_or(0);
                st.collecting = true;
                continue;
            }
            st.flush();
            let bytes = hex_to_bytes(line);
            if !bytes.is_empty() {
                st.out.push(ElmMessage {
                    source: String::new(),
                    bytes,
                });
            }
        }
    }
    st.flush();
    st.out
}

// Headers on. Every line is one frame with the sender in front.
fn split_with_headers<S: AsRef<str>>(lines: &[S], format: HeaderFormat) -> Vec<ElmMessage> {
    // ISO-TP reassembly per sender for CAN.
    struct Pending {
        source: String,
        total: usize,
        data: Vec<u8>,
    }
    let mut out: Vec<ElmMessage> = Vec::new();
    let mut pending: Vec<Pending> = Vec::new();

    fn finish(pending: &mut Vec<Pending>, out: &mut Vec<ElmMessage>, source: &str) {
        if let Some(i) = pending.iter().position(|p| p.source == source) {
            let mut done = pending.remove(i);
            if done.data.len() > done.total {
                done.data.truncate(done.total);
            }
            out.push(ElmMessage {
                source: done.source,
                bytes: done.data,
            });
        }
    }

    for line in lines {
        let line = line.as_ref();
        if !is_hex_line(line) {
            continue;
        }
        let digits = compact_upper(line);

        if format == HeaderFormat::Can11 || format == HeaderFormat::Can29 {
            let id_length = if format == HeaderFormat::Can11 { 3 } else { 8 };
            if digits.len() < id_length + 2 {
                continue;
            }
            let source = digits[..id_length].to_string();
            let frame = hex_to_bytes(&digits[id_length..]);
            if frame.is_empty() {
                continue;
            }
            let kind = frame[0] >> 4;
            if kind == 0 {
                // Single frame: the low nibble is the length. Anything after
                // it is padding.
                let mut length = usize::from(frame[0] & 0x0F);
                let mut start = 1;
                if length == 0 && frame.len() > 1 {
                    // CAN FD escape
                    length = usize::from(frame[1]);
                    start = 2;
                }
                let end = frame.len().min(start + length);
                let bytes = if start <= end {
                    frame[start..end].to_vec()
                } else {
                    Vec::new()
                };
                out.push(ElmMessage { source, bytes });
            } else if kind == 1 && frame.len() >= 2 {
                // First frame: twelve bits of total length, then data.
                finish(&mut pending, &mut out, &source); // an unfinished older message from the same sender
                let total = (usize::from(frame[0] & 0x0F) << 8) | usize::from(frame[1]);
                pending.push(Pending {
                    source: source.clone(),
                    total,
                    data: frame[2..].to_vec(),
                });
                if pending.last().is_some_and(|p| p.data.len() >= p.total) {
                    finish(&mut pending, &mut out, &source);
                }
            } else if kind == 2 {
                // Consecutive frame.
                let Some(p) = pending.iter_mut().find(|p| p.source == source) else {
                    continue;
                };
                p.data.extend_from_slice(&frame[1..]);
                if p.data.len() >= p.total {
                    finish(&mut pending, &mut out, &source);
                }
            }
            // Type 3 is flow control, sent by the adapter itself.
            continue;
        }

        // K-line and J1850: header bytes, data, then one checksum byte.
        let bytes = hex_to_bytes(&digits);
        let mut header = 3;
        if format == HeaderFormat::KLine
            && !bytes.is_empty()
            && (bytes[0] & 0xC0) != 0
            && (bytes[0] & 0x3F) == 0
        {
            header = 4; // ISO 14230 with a separate length byte
        }
        if bytes.len() < header + 2 {
            continue;
        }
        out.push(ElmMessage {
            source: format!("{:02X}", bytes[2]),
            bytes: bytes[header..bytes.len() - 1].to_vec(),
        });
    }

    // Whatever is still incomplete is handed out as far as it got.
    while let Some(first) = pending.first() {
        let source = first.source.clone();
        finish(&mut pending, &mut out, &source);
    }
    out
}

/// The lines of one adapter reply to `command`, as [`Elm327::send`] keeps
/// them: without the prompt, empty lines, the echo and "SEARCHING...".
/// A recording holds the raw reply, so reading one back goes through here
/// too.
pub fn reply_lines(reply: &[u8], command: &str) -> Vec<String> {
    let text = String::from_utf8_lossy(reply);
    let mut lines = Vec::new();
    for raw in text.split(['\r', '\n']) {
        // The prompt may sit anywhere in a chunk; `send` drops it before.
        let without_prompt;
        let raw = if raw.contains('>') {
            without_prompt = raw.replace('>', "");
            without_prompt.as_str()
        } else {
            raw
        };
        let line = trim(raw);
        if line.is_empty() {
            continue;
        }
        // The echo, in case echo is still on.
        if same_compact(line, command) {
            continue;
        }
        if starts_with_upper(line, "SEARCHING") {
            continue;
        }
        lines.push(line.to_string());
    }
    lines
}

/// The first half of [`decode_messages`]: the adapter lines put back together
/// into whole messages (ISO-TP reassembly included), without filtering by
/// service. UDS needs this to see negative responses such as 7F 22 78 next
/// to the positive one. Status lines like "NO DATA" are skipped.
pub fn split_messages<S: AsRef<str>>(lines: &[S], headers: HeaderFormat) -> Vec<ElmMessage> {
    if headers == HeaderFormat::None {
        split_without_headers(lines)
    } else {
        split_with_headers(lines, headers)
    }
}

/// Splits the adapter lines into messages. With headers the sender of each
/// message is kept in [`ElmResult::sources`] and multi-frame CAN responses
/// (ISO-TP first and consecutive frames) are put back together per sender.
pub fn decode_messages<S: AsRef<str>>(
    lines: &[S],
    expected_service: u8,
    headers: HeaderFormat,
) -> ElmResult {
    decode_lines(
        lines.iter().map(|s| s.as_ref().to_string()).collect(),
        expected_service,
        headers,
    )
}

/// [`decode_messages`] on lines the caller hands over, so they are not
/// copied once more for [`ElmResult::lines`].
fn decode_lines(lines: Vec<String>, expected_service: u8, headers: HeaderFormat) -> ElmResult {
    let mut result = ElmResult {
        lines,
        ..ElmResult::default()
    };

    for line in &result.lines {
        if starts_with_upper(line, "NO DATA") {
            result.status = ElmStatus::NoData;
            result.message = l(
                "Steuergerät unterstützt diese Anfrage nicht (NO DATA).",
                "Control unit does not support this request (NO DATA).",
            )
            .to_string();
            return result;
        }
        if ERRORS.iter().any(|err| is_error_line(line, err)) {
            result.status = ElmStatus::BusError;
            result.message = format!("{}{}", l("Adapter meldet: ", "Adapter reports: "), line);
            return result;
        }
    }

    let raw = split_messages(&result.lines, headers);

    // With several control units on the bus one of them may refuse while
    // another answers. Only if nobody answers is the refusal the result.
    let mut refusal: Option<u8> = None;
    let expected = u32::from(expected_service);
    for msg in raw {
        let bytes = &msg.bytes;
        if bytes.is_empty() {
            continue;
        }
        if bytes[0] == 0x7F && bytes.len() >= 3 {
            refusal.get_or_insert(bytes[2]);
            continue;
        }
        let first = u32::from(bytes[0]);
        if first == expected + 0x40 || first == expected {
            // The service byte goes, the rest stays where it is.
            let mut payload = msg.bytes;
            payload.remove(0);
            result.messages.push(payload);
            result.sources.push(msg.source);
        }
    }
    if result.messages.is_empty() {
        if let Some(code) = refusal {
            result.status = ElmStatus::Negative;
            result.message = format!(
                "{}{})",
                l(
                    "Steuergerät lehnt den Dienst ab (Code 0x",
                    "Control unit rejects the service (code 0x"
                ),
                hex_byte(code)
            );
            return result;
        }
        result.status = ElmStatus::NoData;
        result.message = l(
            "Keine verwertbare Antwort erhalten.",
            "No usable response received.",
        )
        .to_string();
    }
    result
}

/// The ELM327 driver. Owns the transport it talks through.
/// What the last [`Elm327::connect`] found out, for the connection
/// assistant. It stays after a failed attempt and after `disconnect`, and is
/// reset by the next `connect`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConnectTrace {
    /// The adapter answered the reset (ATZ).
    pub adapter_answered: bool,
    /// What it called itself, e.g. "ELM327 v1.5".
    pub adapter_id: String,
    /// The protocol numbers tried, in order.
    pub protocols_tried: String,
    /// The last error line the adapter reported from the bus, e.g.
    /// "UNABLE TO CONNECT" or "CAN ERROR".
    pub bus_error: String,
}

/// The first line of a reply that is an error report of the adapter about
/// the bus. A lone "?" (unknown command) is not one.
fn bus_error_line(result: &ElmResult) -> Option<String> {
    result.lines.iter().find_map(|line| {
        let line = line.trim();
        ERRORS
            .iter()
            .filter(|err| **err != "?")
            .any(|err| is_error_line(line, err))
            .then(|| line.to_string())
    })
}

pub struct Elm327 {
    trace: ConnectTrace,
    transport: Option<Box<dyn Transport>>,
    log: Option<LogFn>,
    timeout_scale: f64,
    verbose: bool,
    adaptive_timing: AdaptiveTiming,
    timing_level: u8,
    stn: bool,
    stn_batching: bool,
    stn_batch: bool,
    stn_bus: StnBus,
    hs_protocol_text: String,
    obd_on_uds: bool,
    listen_only: bool,
    stn_id: String,
    version: String,
    protocol_text: String,
    protocol_id: char,
    headers: bool,
    connected: bool,
    /// SAE J1939 bit rate in kbit/s while in protocol A or B, else 0.
    j1939_kbit: u32,
    /// The 29-bit id the adapter sends with right now (ATCP and ATSH).
    j1939_header: Option<u32>,
    /// Requests may carry the answer count digit.
    count_hint_enabled: bool,
    /// Times the digit cost answers in this session.
    count_hint_misses: u8,
    /// Answer frames per request, learned from plain requests.
    answer_counts: Vec<AnswerCount>,
    /// Raised by another thread to end the exchange in flight
    /// ([`Elm327::set_interrupt`]).
    interrupt: Option<Arc<AtomicBool>>,
    /// The last command got nothing back at all, not even a prompt.
    silent: bool,
}

/// Reads until a prompt came or `ms` passed; true if the prompt came.
fn wait_for_prompt(t: &mut dyn Transport, ms: u64, interrupt: Option<&AtomicBool>) -> bool {
    let deadline = now_ms() + ms;
    loop {
        let now = now_ms();
        if now >= deadline || raised(interrupt) {
            return false;
        }
        if t.read((deadline - now).min(50)).contains(&b'>') {
            return true;
        }
    }
}

/// Whether the interrupt flag of [`Elm327::set_interrupt`] is up.
pub(crate) fn raised(interrupt: Option<&AtomicBool>) -> bool {
    interrupt.is_some_and(|flag| flag.load(Ordering::SeqCst))
}

/// How many answer lines a request got last time.
#[derive(Clone, Debug)]
struct AnswerCount {
    command: String,
    frames: u8,
    uses: u16,
}

/// Misses of the answer count digit before a session stops using it.
const COUNT_HINT_MISSES: u8 = 2;
/// Every this many uses a request goes out plain once.
const COUNT_HINT_RELEARN: u16 = 32;
/// Distinct requests remembered (the live page rotates through a few).
const COUNT_HINT_MEMORY: usize = 64;

/// The CAN frames among the answer lines of a request: with headers every
/// hex line is one; without, the length line in front of a multi-frame
/// answer (three hex digits) is the adapter's and not a frame.
fn answer_frames(lines: &[String], headers: bool) -> usize {
    lines
        .iter()
        .filter(|line| is_hex_line(line))
        .filter(|line| headers || line.chars().filter(|c| !is_c_space(*c)).count() != 3)
        .count()
}

impl Default for Elm327 {
    fn default() -> Self {
        Self::new()
    }
}

impl Elm327 {
    pub fn new() -> Self {
        Self {
            trace: ConnectTrace::default(),
            transport: None,
            log: None,
            timeout_scale: 1.0,
            verbose: true,
            adaptive_timing: AdaptiveTiming::Standard,
            timing_level: 0,
            stn: false,
            stn_batching: true,
            stn_batch: false,
            stn_bus: StnBus::HighSpeed,
            hs_protocol_text: String::new(),
            obd_on_uds: false,
            listen_only: false,
            stn_id: String::new(),
            version: "?".to_string(),
            protocol_text: l("unbekannt", "unknown").to_string(),
            protocol_id: '0',
            headers: false,
            connected: false,
            j1939_kbit: 0,
            j1939_header: None,
            count_hint_enabled: true,
            count_hint_misses: 0,
            answer_counts: Vec::new(),
            interrupt: None,
            silent: false,
        }
    }

    /// A flag another thread may raise to end what the driver is doing: the
    /// command in flight returns within one read of the transport (at most
    /// 120 ms) as if the adapter had not answered, and every further command
    /// returns at once without being sent, until the flag is lowered again.
    /// The app's Disconnect raises it, so it does not wait for a search or a
    /// live round on a silent adapter, where every request costs a whole
    /// timeout.
    pub fn set_interrupt(&mut self, flag: Arc<AtomicBool>) {
        self.interrupt = Some(flag);
    }

    /// Whether the flag of [`Elm327::set_interrupt`] is up.
    pub fn interrupted(&self) -> bool {
        raised(self.interrupt.as_deref())
    }

    /// The answer to a command that was cut short or not sent because the
    /// interrupt flag is up.
    fn interrupted_result() -> ElmResult {
        ElmResult {
            status: ElmStatus::AdapterError,
            message: l("Kein Adapter verbunden.", "No adapter connected.").to_string(),
            ..ElmResult::default()
        }
    }

    pub fn set_log(&mut self, log: LogFn) {
        self.log = Some(log);
    }
    pub fn clear_log(&mut self) {
        self.log = None;
    }

    fn log(&self, line: &str) {
        if let Some(log) = &self.log {
            log(line);
        }
    }

    /// Every timeout is multiplied by this factor. Sluggish clones need more
    /// time; the gradation between short and long waits is preserved because
    /// not everything is set to the same value.
    pub fn set_timeout_scale(&mut self, scale: f64) {
        self.timeout_scale = if scale > 0.0 { scale } else { 1.0 };
    }
    pub fn timeout_scale(&self) -> f64 {
        self.timeout_scale
    }
    /// Log every sent line together with its response.
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }
    pub fn verbose(&self) -> bool {
        self.verbose
    }
    /// Which adaptive timing connect sets up. Takes effect on the next connect.
    pub fn set_adaptive_timing(&mut self, mode: AdaptiveTiming) {
        self.adaptive_timing = mode;
    }
    pub fn adaptive_timing(&self) -> AdaptiveTiming {
        self.adaptive_timing
    }

    pub fn connected(&self) -> bool {
        self.connected
    }
    /// What the last connect attempt found out.
    pub fn connect_trace(&self) -> &ConnectTrace {
        &self.trace
    }
    /// Connected only for listening: the car does not answer OBD, but there
    /// is traffic on CAN 500 kbit (Tesla). Values come from broadcast frames.
    pub fn listen_only(&self) -> bool {
        self.listen_only
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn protocol_text(&self) -> &str {
        &self.protocol_text
    }
    pub fn transport_name(&self) -> String {
        self.transport
            .as_ref()
            .map(|t| t.name())
            .unwrap_or_default()
    }
    /// The transport in use, if any. Elm327 keeps ownership.
    pub fn transport(&self) -> Option<&dyn Transport> {
        self.transport.as_deref()
    }
    pub fn transport_mut(&mut self) -> Option<&mut (dyn Transport + 'static)> {
        self.transport.as_deref_mut()
    }
    /// Several ISO-TP sessions at once, when connected through a raw CAN
    /// interface; `None` for a real ELM327 (and while recording, so the
    /// recording holds every request).
    pub fn iso_tp_batch(&mut self) -> Option<&mut dyn IsoTpBatch> {
        if !self.connected {
            return None;
        }
        self.transport.as_deref_mut()?.iso_tp_batch()
    }

    /// Replaces the transport by whatever `wrap` builds around it, without
    /// reconnecting: this is how a recording is switched on and off in a
    /// running session. `wrap` receives the current transport and returns the
    /// one to use from now on (`None` leaves the driver without one). Call it
    /// between two requests, from the thread that uses this Elm327.
    pub fn wrap_transport(
        &mut self,
        wrap: impl FnOnce(Box<dyn Transport>) -> Option<Box<dyn Transport>>,
    ) {
        let Some(current) = self.transport.take() else {
            return;
        };
        self.transport = wrap(current);
        if self.transport.is_none() {
            self.connected = false;
        }
    }

    /// The adaptive timing actually in effect after connect: 1 (ATAT1) or 2
    /// (ATAT2). 0 before connect, or if the adapter refused both.
    pub fn adaptive_timing_level(&self) -> u8 {
        self.timing_level
    }
    /// An STN chip (OBDLink and others) instead of an ELM327 or a clone of
    /// it. Found out during connect with the read-only STI, which an STN
    /// answers with its firmware ("STN1130 v5.6.19") and anything else with
    /// "?".
    pub fn is_stn(&self) -> bool {
        self.stn
    }
    /// The car answers the emission services only over UDS (SAE J1979-2,
    /// "OBD on UDS", required in the US from model year 2027 and already in
    /// some newer cars). `request` then translates the classic services
    /// (01, 03, 04, 07, 09, 0A) to UDS and the answers back, so the rest of
    /// the program does not notice. Freeze frames (02) and test results (06)
    /// are not available in this mode.
    pub fn obd_on_uds(&self) -> bool {
        self.obd_on_uds
    }
    pub fn stn_id(&self) -> &str {
        &self.stn_id
    }
    /// Protocol number as the adapter counts ('1' to '9'), '0' if unknown.
    pub fn protocol_id(&self) -> char {
        self.protocol_id
    }
    pub fn is_can(&self) -> bool {
        ('6'..='9').contains(&self.protocol_id)
    }
    pub fn headers_on(&self) -> bool {
        self.headers
    }
    pub fn header_format(&self) -> HeaderFormat {
        if self.headers {
            header_format_for(self.protocol_id)
        } else {
            HeaderFormat::None
        }
    }

    #[cfg(test)]
    pub(crate) fn reply_is_garbled_for_test(text: &str) -> bool {
        reply_is_garbled(text)
    }

    #[cfg(test)]
    pub(crate) fn reset_was_interrupted_for_test(lines: &[String]) -> bool {
        reset_was_interrupted(lines)
    }

    fn drain_greeting(&mut self) {
        // Wi-Fi and Bluetooth adapters announce themselves unprompted. If this
        // greeting is left in the buffer, it answers the first command and all
        // following responses are shifted by one position.
        let Some(t) = self.transport.as_mut() else {
            return;
        };
        // A greeting ends with the prompt, and nothing follows it until the
        // next command: once the prompt is there, waiting longer gains
        // nothing. The same for the answer to the lone carriage return.
        // Without a prompt this waits as long as it always did.
        let interrupt = self.interrupt.as_deref();
        wait_for_prompt(t.as_mut(), 300, interrupt);
        t.flush_input();
        t.write(b"\r");
        wait_for_prompt(t.as_mut(), 200, interrupt);
        t.flush_input();
    }

    /// Sends one line and collects the response up to the prompt.
    pub fn send(&mut self, command: &str, timeout_ms: u64) -> ElmResult {
        let mut result = ElmResult::default();
        let scale = self.timeout_scale;
        let interrupt = self.interrupt.as_deref();
        let Some(t) = self.transport.as_mut() else {
            result.status = ElmStatus::AdapterError;
            result.message = l("Kein Adapter verbunden.", "No adapter connected.").to_string();
            return result;
        };
        if raised(interrupt) {
            return Self::interrupted_result();
        }
        t.flush_input();
        let mut line = command.as_bytes().to_vec();
        line.push(b'\r');
        t.write(&line);

        // The configured factor applies here, at the only place where we
        // actually wait.
        let wait_ms = (timeout_ms as f64 * scale) as u64;

        let mut buffer: Vec<u8> = Vec::new();
        let deadline = now_ms() + wait_ms;
        let mut prompt = false;
        while now_ms() < deadline {
            let chunk = t.read(120);
            if !chunk.is_empty() {
                buffer.extend_from_slice(&chunk);
                // Earlier chunks had no prompt, or the loop would have ended.
                if chunk.contains(&b'>') {
                    prompt = true;
                    break;
                }
            }
            if raised(interrupt) {
                break;
            }
        }
        // Cut short: what came so far is no whole answer.
        if !prompt && raised(interrupt) {
            return Self::interrupted_result();
        }
        self.silent = buffer.is_empty();
        if buffer.is_empty() {
            result.status = ElmStatus::AdapterError;
            result.message = format!(
                "{}{}{}",
                l(
                    "Keine Antwort des Adapters auf '",
                    "No response from the adapter to '"
                ),
                command,
                l(
                    "'. Falsche Baudrate, oder Kabel/Bluetooth weg?",
                    "'. Wrong baud rate, or cable/Bluetooth disconnected?"
                )
            );
            return result;
        }

        buffer.retain(|&c| c != b'>');
        result.lines = reply_lines(&buffer, command);
        // Keep track of the header setting even when someone sends the AT
        // commands by hand (the raw console, the KKL reset). A reset switches
        // headers off again.
        let sent = compact_upper(command);
        if sent == "ATH1" {
            self.headers = true;
        }
        if sent == "ATH0" || sent == "ATZ" || sent == "ATD" || sent == "ATWS" {
            self.headers = false;
        }
        // Any setting may change who answers (headers, filters, the target
        // address, the protocol): the answer counts learned so far no longer
        // hold.
        if sent.starts_with("AT") || sent.starts_with("ST") {
            self.answer_counts.clear();
        }

        if self.verbose && self.log.is_some() {
            let size: usize = result.lines.iter().map(|l| l.len() + 3).sum();
            let mut line = String::with_capacity(command.len() + size + 12);
            line.push_str("  > ");
            line.push_str(command);
            line.push_str("   ->   ");
            for l in &result.lines {
                line.push_str(l);
                line.push_str(" | ");
            }
            self.log(&line);
        }
        result
    }

    fn at(&mut self, command: &str, timeout_ms: u64) -> ElmResult {
        self.send(&format!("AT{command}"), timeout_ms)
    }

    /// Sends and decodes the response to the expected service (e.g. 0x03 -> 0x43).
    pub fn request(&mut self, command: &str, expected_service: u8, timeout_ms: u64) -> ElmResult {
        if self.obd_on_uds {
            return self.request_on_uds(command, expected_service, timeout_ms);
        }
        if let Some(frames) = self.count_hint_for(command) {
            let hinted = format!("{command}{frames:X}");
            let raw = self.send(&hinted, timeout_ms);
            if raw.status == ElmStatus::AdapterError {
                return raw;
            }
            let refused = raw.lines.len() == 1 && raw.lines[0] == "?";
            let seen = answer_frames(&raw.lines, self.headers);
            let result = decode_lines(raw.lines, expected_service, self.header_format());
            if result.ok() && !refused {
                // Fewer lines than the digit cost the full wait already;
                // the count only goes down after a plain request.
                self.learn_answer_count(command, seen.max(usize::from(frames)));
                return result;
            }
            // The digit was refused, or nothing usable came back with it:
            // ask the plain way, as without the digit, and so find out whose
            // fault it was.
            let plain = self.plain_request(command, expected_service, timeout_ms);
            if refused || plain.ok() {
                self.count_hint_missed(refused);
            }
            return plain;
        }
        self.plain_request(command, expected_service, timeout_ms)
    }

    fn plain_request(&mut self, command: &str, expected_service: u8, timeout_ms: u64) -> ElmResult {
        let raw = self.send(command, timeout_ms);
        if raw.status == ElmStatus::AdapterError {
            return raw;
        }
        let seen = answer_frames(&raw.lines, self.headers);
        let result = decode_lines(raw.lines, expected_service, self.header_format());
        if result.ok() {
            self.learn_answer_count(command, seen);
        } else {
            self.forget_answer_count(command);
        }
        result
    }

    // -----------------------------------------------------------------------
    // The answer count digit (ELM327 v1.3 and later)
    //
    // After the last answer the adapter waits whether another control unit
    // answers, up to its (adaptive) timeout, and that wait is a large part
    // of every request. A single hex digit after the request tells it how
    // many answer lines to expect, and it returns as soon as they are there
    // (ELM327 data sheet, "Talking to the Vehicle" and "Setting Timeouts").
    // The data sheet asks programs to find the count out first and only then
    // use it: a request goes out plain, its answer frames are counted, and
    // the next identical request carries that count. Fewer answers than the
    // digit says only cost the usual timeout, and the count is never smaller
    // than what came last time, so nothing is cut off. Only mode 01 on CAN,
    // where the live values repeat the same requests and every answer line
    // is one CAN frame. An adapter that answers "?" to the digit (older than
    // v1.3, or a clone that only claims a version) or loses answers with it
    // gets plain requests for the rest of the session.
    // -----------------------------------------------------------------------

    /// Whether requests may carry the answer count digit (on by default).
    /// Takes effect at once; the counts are learned again.
    pub fn set_answer_count_hint(&mut self, on: bool) {
        self.count_hint_enabled = on;
        self.answer_counts.clear();
    }

    /// The answer count digit is in use: allowed, and the adapter has not
    /// turned it down in this session.
    pub fn answer_count_hint(&self) -> bool {
        self.count_hint_enabled && self.count_hint_misses < COUNT_HINT_MISSES
    }

    fn count_hint_for(&mut self, command: &str) -> Option<u8> {
        if !self.answer_count_hint() || !self.connected || !self.is_can() || self.listen_only {
            return None;
        }
        if !self
            .transport
            .as_ref()
            .is_some_and(|t| t.count_hint_allowed())
        {
            return None;
        }
        let entry = self
            .answer_counts
            .iter_mut()
            .find(|e| e.command == command)?;
        entry.uses = entry.uses.wrapping_add(1);
        // Now and then the plain way, so a control unit that starts to
        // answer as well is not left out for long.
        if entry.uses.is_multiple_of(COUNT_HINT_RELEARN) {
            return None;
        }
        Some(entry.frames)
    }

    fn learn_answer_count(&mut self, command: &str, frames: usize) {
        if !self.answer_count_hint() || !self.is_can() {
            return;
        }
        let b = command.as_bytes();
        let mode01 = b.len() >= 4
            && b.len().is_multiple_of(2)
            && b.starts_with(b"01")
            && b.iter()
                .all(|c| c.is_ascii_digit() || (b'A'..=b'F').contains(c));
        if !mode01 || !(1..=15).contains(&frames) {
            self.forget_answer_count(command);
            return;
        }
        let frames = frames as u8;
        if let Some(entry) = self.answer_counts.iter_mut().find(|e| e.command == command) {
            entry.frames = frames;
            return;
        }
        if self.answer_counts.len() >= COUNT_HINT_MEMORY {
            self.answer_counts.clear();
        }
        self.answer_counts.push(AnswerCount {
            command: command.to_string(),
            frames,
            uses: 0,
        });
    }

    fn forget_answer_count(&mut self, command: &str) {
        if let Some(i) = self.answer_counts.iter().position(|e| e.command == command) {
            self.answer_counts.swap_remove(i);
        }
    }

    fn count_hint_missed(&mut self, refused: bool) {
        self.answer_counts.clear();
        self.count_hint_misses = if refused {
            COUNT_HINT_MISSES
        } else {
            self.count_hint_misses.saturating_add(1)
        };
        if self.count_hint_misses >= COUNT_HINT_MISSES {
            self.log(l(
                "  Der Adapter kommt mit der Antwortzahl hinter der Anfrage nicht zurecht, Anfragen gehen ohne sie.",
                "  The adapter does not handle the answer count after a request, requests go without it.",
            ));
        }
    }

    // -----------------------------------------------------------------------
    // OBD on UDS (SAE J1979-2)
    //
    //   01 pp      ->  22 F4 pp       PIDs of service 01 as data identifiers
    //   09 ii      ->  22 F8 ii       vehicle information
    //   03         ->  19 42 33 08 FF confirmed emission DTCs (WWH-OBD format)
    //   07         ->  19 42 33 04 FF pending
    //   0A         ->  19 55 33       permanent
    //   04         ->  14 FF FF 33    clear the emission group
    // -----------------------------------------------------------------------

    fn uds_request(&mut self, command: &str, timeout_ms: u64) -> ElmResult {
        let sid = command
            .get(..2)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .unwrap_or(0);
        let needle = format!("7F{sid:02X}78");
        let mut result = ElmResult::default();
        // A unit that is still busy says 7F xx 78; ask again, as UDS allows.
        for _ in 0..10 {
            let raw = self.send(command, timeout_ms);
            if raw.status == ElmStatus::AdapterError {
                return raw;
            }
            let pending = raw
                .lines
                .iter()
                .any(|line| to_upper(&strip_spaces(line)).contains(&needle));
            result = decode_messages(&raw.lines, sid, self.header_format());
            if result.ok() || !pending {
                return result;
            }
            sleep_ms(50);
        }
        result
    }

    fn request_on_uds(
        &mut self,
        command: &str,
        _expected_service: u8,
        timeout_ms: u64,
    ) -> ElmResult {
        let mut hex = to_upper(&strip_spaces(command));
        if !hex.len().is_multiple_of(2) {
            hex.pop(); // "expected answers" digit
        }
        let none = ElmResult {
            status: ElmStatus::NoData,
            message: l(
                "Im Modus OBD über UDS nicht verfügbar.",
                "Not available in OBD on UDS mode.",
            )
            .to_string(),
            ..ElmResult::default()
        };
        if !all_hex(&hex) {
            return none;
        }
        let bytes = hex_to_bytes(&hex);
        if bytes.is_empty() {
            return none;
        }

        // Collects the translated payloads per sending unit, in order.
        let mut order: Vec<String> = Vec::new();
        let mut payloads: Vec<Vec<u8>> = Vec::new();
        let mut add = |source: String, data: Vec<u8>| {
            if let Some(i) = order.iter().position(|o| *o == source) {
                payloads[i].extend(data);
            } else {
                order.push(source);
                payloads.push(data);
            }
        };
        let source_of =
            |last: &ElmResult, m: usize| last.sources.get(m).cloned().unwrap_or_default();

        match bytes[0] {
            0x01 | 0x09 => {
                // One identifier per request; several PIDs are joined back into
                // one multi-PID answer per unit.
                let high: u8 = if bytes[0] == 0x01 { 0xF4 } else { 0xF8 };
                let mut last = ElmResult::default();
                for &item in &bytes[1..] {
                    last = self.uds_request(&format!("22{high:02X}{item:02X}"), timeout_ms);
                    if !last.ok() {
                        continue;
                    }
                    for (m, d) in last.messages.iter().enumerate() {
                        if d.len() < 2 || d[0] != high || d[1] != item {
                            continue;
                        }
                        let mut classic = d[1..].to_vec();
                        // Service 09 had a count byte in front of these items.
                        if bytes[0] == 0x09 && (item == 0x02 || item == 0x0A) {
                            classic.insert(1, 1);
                        }
                        add(source_of(&last, m), classic);
                    }
                }
                finish_uds(order, payloads, last, none)
            }
            0x03 | 0x07 | 0x0A => {
                let permanent = bytes[0] == 0x0A;
                let cmd = if permanent {
                    "195533"
                } else if bytes[0] == 0x03 {
                    "19423308FF"
                } else {
                    "19423304FF"
                };
                let last = self.uds_request(cmd, timeout_ms);
                if !last.ok() {
                    return last;
                }
                for (m, d) in last.messages.iter().enumerate() {
                    // 42: sub, group, status mask, severity mask, format, then
                    // severity + DTC (3) + status. 55: sub, group, status mask,
                    // format, then DTC (3) + status.
                    let start = if permanent { 4 } else { 5 };
                    let step = if permanent { 4 } else { 5 };
                    let dtc_at = if permanent { 0 } else { 1 };
                    if d.len() < start {
                        continue;
                    }
                    let mut classic: Vec<u8> = vec![0]; // count first, as on CAN
                    let mut i = start;
                    while i + step <= d.len() {
                        classic.push(d[i + dtc_at]);
                        classic.push(d[i + dtc_at + 1]);
                        i += step;
                    }
                    classic[0] = ((classic.len() - 1) / 2) as u8;
                    add(source_of(&last, m), classic);
                }
                finish_uds(order, payloads, last, none)
            }
            0x04 => {
                let last = self.uds_request("14FFFF33", timeout_ms);
                if !last.ok() {
                    return last;
                }
                ElmResult {
                    messages: vec![Vec::new(); last.messages.len()],
                    sources: last.sources,
                    ..ElmResult::default()
                }
            }
            _ => none, // 02, 05, 06, 08 have no simple counterpart
        }
    }

    // -----------------------------------------------------------------------
    // Listening to the bus
    // -----------------------------------------------------------------------

    /// Listens on the bus (ATMA) for up to `duration_ms` or `max_lines` lines
    /// and stops the adapter again. The lines as printed, status lines removed.
    /// An STN chip gets STMA with the number of lines instead: it stops by
    /// itself once that many messages came in, which saves the stop and its
    /// wait.
    pub fn monitor_lines(&mut self, duration_ms: u64, max_lines: usize) -> Vec<String> {
        let command = if self.stn {
            format!("STMA {}", max_lines.clamp(1, 32767))
        } else {
            "ATMA".to_string()
        };
        self.monitor_with(&command, duration_ms, max_lines)
    }

    /// The monitor loop behind [`Elm327::monitor_lines`], for any of the
    /// monitor commands (ATMA, STMA n, STM n).
    fn monitor_with(&mut self, command: &str, duration_ms: u64, max_lines: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let Some(t) = self.transport.as_mut() else {
            return lines;
        };
        t.flush_input();
        let mut line = command.as_bytes().to_vec();
        line.push(b'\r');
        t.write(&line);
        let interrupt = self.interrupt.as_deref();
        let mut buffer: Vec<u8> = Vec::new();
        let deadline = now_ms() + duration_ms;
        let complete = |b: &[u8]| b.iter().filter(|&&c| c == b'\r').count();
        while now_ms() < deadline && complete(&buffer) <= max_lines && !raised(interrupt) {
            buffer.extend(t.read(50));
            if buffer.contains(&b'>') {
                break; // stopped by itself (BUFFER FULL)
            }
        }
        if !buffer.contains(&b'>') {
            // Any character ends the monitor; the prompt follows.
            t.write(b"\r");
            let stop = now_ms() + 1500;
            while now_ms() < stop && !raised(interrupt) {
                buffer.extend(t.read(50));
                if buffer.contains(&b'>') {
                    break;
                }
            }
        }
        const SKIP: [&str; 8] = [
            "ATMA",
            "STOPPED",
            "BUFFER FULL",
            "?",
            "SEARCHING...",
            "NO DATA",
            "CAN ERROR",
            "OK",
        ];
        let text = String::from_utf8_lossy(&buffer);
        for raw in text.split(['\r', '\n', '>']) {
            let t = trim(raw);
            if t.is_empty() {
                continue;
            }
            let up = to_upper(t);
            // The echo of an ST monitor command; data lines are hex only.
            if SKIP.contains(&up.as_str()) || up.starts_with("STM") {
                continue;
            }
            lines.push(t.to_string());
        }
        lines.truncate(max_lines);
        lines
    }

    /// Frames with this 11-bit id that the car sends by itself, raw data
    /// bytes, at most `max_frames`. Restores CAN auto formatting and the
    /// receive filter afterwards.
    pub fn capture_frames(
        &mut self,
        can_id: u32,
        duration_ms: u64,
        max_frames: usize,
    ) -> Vec<Vec<u8>> {
        let mut frames = Vec::new();
        if self.transport.is_none() {
            return frames;
        }
        let filter = format!("CRA{:03X}", can_id & 0x7FF);
        // Raw data bytes: without auto formatting the first byte is not taken
        // for an ISO-TP PCI.
        self.at("CAF0", DEFAULT_SEND_TIMEOUT_MS);
        self.at(&filter, DEFAULT_SEND_TIMEOUT_MS);
        let lines = if self.stn {
            self.stn_monitor_id(can_id, duration_ms, max_frames)
        } else {
            self.monitor_lines(duration_ms, max_frames)
        };
        for line in lines {
            let mut hex = strip_spaces(&to_upper(&line));
            if self.headers {
                if hex.len() < 3 {
                    continue;
                }
                hex = hex[3..].to_string();
            }
            if hex.is_empty() || !hex.len().is_multiple_of(2) || hex.len() > 16 || !all_hex(&hex) {
                continue;
            }
            frames.push(hex_to_bytes(&hex));
        }
        self.at("CAF1", DEFAULT_SEND_TIMEOUT_MS);
        if self.stn {
            // ATAR clears the pass filter of stn_monitor_id together with the
            // hardware filter and turns the automatic filtering back on.
            self.at("AR", DEFAULT_SEND_TIMEOUT_MS);
        } else {
            let cra = self.at("CRA", DEFAULT_SEND_TIMEOUT_MS);
            if !answered_ok(&cra) {
                self.at("AR", DEFAULT_SEND_TIMEOUT_MS);
            }
        }
        frames
    }

    /// Headers on (ATH1) keep the sender address of every response, so the
    /// results can say which control unit replied when several answer at
    /// once (engine 7E8, transmission 7E9, ...). Off by default. Returns
    /// false if the adapter refused the command.
    pub fn set_headers(&mut self, on: bool) -> bool {
        let r = self.at(if on { "H1" } else { "H0" }, 3000);
        if r.status == ElmStatus::AdapterError {
            return false;
        }
        if r.lines.iter().any(|line| to_upper(line).contains("OK")) {
            self.headers = on;
            return true;
        }
        if on {
            self.headers = false;
        }
        false
    }

    fn detect_stn(&mut self) {
        // STI only reads the firmware identification. An ELM327 or a clone
        // answers "?" (an ST command is not an AT command, so even clones that
        // say "OK" to every AT command give themselves away here).
        self.stn = false;
        self.stn_id.clear();
        let sti = self.send("STI", 1500);
        if sti.status == ElmStatus::AdapterError || sti.lines.is_empty() {
            return;
        }
        let first = sti.lines[0].clone();
        if to_upper(&first).starts_with("STN") {
            self.stn = true;
            self.log(&format!(
                "{}{}",
                l("  STN-Chip erkannt: ", "  STN chip detected: "),
                first
            ));
            self.stn_id = first;
        }
    }

    fn apply_aggressive_timing(&mut self) {
        let wanted = self.adaptive_timing == AdaptiveTiming::Aggressive
            || (self.adaptive_timing == AdaptiveTiming::Auto && self.stn);
        if !wanted {
            return;
        }
        if self.timing_level == 0 {
            self.log(l(
                "  Der Adapter kennt keine adaptive Zeitsteuerung (ATAT), ATAT2 entfällt.",
                "  The adapter has no adaptive timing (ATAT), so ATAT2 is skipped.",
            ));
            return;
        }
        // Only now, with a protocol found: the search itself runs with the
        // careful ATAT1, so a slow control unit is not missed while looking.
        let at2 = self.at("AT2", 2000);
        if !answered_ok(&at2) {
            self.at("AT1", 2000);
            self.timing_level = 1;
            self.log(l(
                "  ATAT2 abgelehnt, es bleibt bei ATAT1.",
                "  ATAT2 refused, staying with ATAT1.",
            ));
            return;
        }
        // One request as a check that answers still come through with the
        // shorter wait. If not, back to the careful setting.
        let check = self.request("0100", 0x01, DEFAULT_REQUEST_TIMEOUT_MS);
        if !check.ok() {
            self.at("AT1", 2000);
            self.timing_level = 1;
            self.log(l(
                "  Mit ATAT2 fehlen Antworten, zurück auf ATAT1.",
                "  Answers go missing with ATAT2, back to ATAT1.",
            ));
            return;
        }
        self.timing_level = 2;
        self.log(l(
            "  Schnelle adaptive Zeitsteuerung (ATAT2) aktiv.",
            "  Aggressive adaptive timing (ATAT2) active.",
        ));
    }

    /// Takes ownership of the transport, resets the adapter and searches the
    /// protocols in the given order ("6", "54312678", ...).
    pub fn connect(
        &mut self,
        transport: Box<dyn Transport>,
        protocol_order: &str,
    ) -> Result<(), String> {
        self.transport = Some(transport);
        self.connected = false;
        self.j1939_kbit = 0;
        self.j1939_header = None;
        self.stn = false;
        self.stn_id.clear();
        self.stn_batch = false;
        self.stn_bus = StnBus::HighSpeed;
        self.timing_level = 0;
        self.obd_on_uds = false;
        self.listen_only = false;
        self.count_hint_misses = 0;
        self.answer_counts.clear();
        self.trace = ConnectTrace::default();
        self.drain_greeting();

        let mut banner = self.at("Z", 6000);
        if banner.status == ElmStatus::AdapterError {
            return Err(banner.message);
        }
        // An STN chip that was still busy (a protocol search started by stray
        // bytes, for example after another baud rate was tried first) answers
        // the reset with STOPPED and drops it. Once more, then it is idle.
        if reset_was_interrupted(&banner.lines) {
            self.drain_greeting();
            banner = self.at("Z", 6000);
            if banner.status == ElmStatus::AdapterError {
                return Err(banner.message);
            }
        }
        let mut version = String::new();
        for line in &banner.lines {
            version.push_str(line);
            version.push(' ');
        }
        self.version = trim(&version).to_string();
        if self.version.is_empty() {
            self.version = "?".to_string();
        }
        // At the wrong baud rate the adapter still sends something, but only
        // scrambled bytes. That is not an answer: the caller tries the next
        // rate instead of blaming the car.
        if reply_is_garbled(&self.version) {
            self.log(&format!(
                "{}{}'",
                l(
                    "  Unleserliche Antwort, vermutlich passt die Baudrate nicht: '",
                    "  Garbled answer, probably the baud rate does not match: '"
                ),
                self.version.escape_default()
            ));
            return Err(l(
                "Der Adapter antwortet unleserlich, vermutlich passt die Baudrate nicht.",
                "The adapter answers garbled, probably the baud rate does not match.",
            )
            .to_string());
        }
        self.trace.adapter_answered = true;
        self.trace.adapter_id = self.version.clone();
        let up = to_upper(&self.version);
        if !up.contains("ELM") && !up.contains("OBD") {
            self.log(&format!(
                "{}{}'",
                l(
                    "  Hinweis: unerwartete Adapterkennung '",
                    "  Note: unexpected adapter ID '"
                ),
                self.version
            ));
        }

        for cmd in ["E0", "L0", "S0", "H0"] {
            self.at(cmd, DEFAULT_SEND_TIMEOUT_MS);
        }
        // ATAT1 is the power-on default; set it anyway, since the adapter may
        // come out of an earlier session with different timing. Adapters older
        // than v1.2 know no adaptive timing at all.
        let at1 = self.at("AT1", DEFAULT_SEND_TIMEOUT_MS);
        self.timing_level = if answered_ok(&at1) { 1 } else { 0 };
        self.at("ST64", DEFAULT_SEND_TIMEOUT_MS);
        self.detect_stn();

        for proto in protocol_order.chars() {
            self.trace.protocols_tried.push(proto);
            self.at(&format!("SP{proto}"), DEFAULT_SEND_TIMEOUT_MS);
            self.log(&format!(
                "{}{} ({}) ...",
                l("  Versuche Protokoll ", "  Trying protocol "),
                proto,
                protocol_name(proto)
            ));
            let timeout = if ('3'..='5').contains(&proto) {
                12000
            } else {
                8000
            };
            let can = ('6'..='9').contains(&proto);
            for _ in 0..2 {
                let mut probe = self.request("0100", 0x01, timeout);
                if !probe.ok() && probe.status != ElmStatus::AdapterError && can {
                    // Newer cars may know the emission services only over UDS.
                    let uds = self.uds_request("22F400", timeout);
                    if uds.ok()
                        && uds
                            .messages
                            .first()
                            .is_some_and(|m| m.len() >= 2 && m[0] == 0xF4 && m[1] == 0x00)
                    {
                        self.obd_on_uds = true;
                        probe = uds;
                        self.log(l(
                            "  Das Auto spricht OBD über UDS (SAE J1979-2).",
                            "  The car speaks OBD on UDS (SAE J1979-2).",
                        ));
                    }
                }
                if probe.ok() {
                    let dp = self.at("DP", DEFAULT_SEND_TIMEOUT_MS);
                    self.protocol_text = match dp.lines.first() {
                        Some(first) => first.clone(),
                        None => protocol_name(proto).to_string(),
                    };
                    self.protocol_id = proto;
                    self.connected = true;
                    self.apply_aggressive_timing();
                    self.setup_stn();
                    return Ok(());
                }
                if probe.status == ElmStatus::AdapterError {
                    return Err(probe.message);
                }
                if let Some(line) = bus_error_line(&probe) {
                    self.trace.bus_error = line;
                }
                sleep_ms(400);
            }
        }

        // No answer to OBD at all. If there is traffic on CAN 500 kbit, the car
        // may still broadcast its values (Tesla has no OBD responder at all).
        if protocol_order.contains('6') {
            self.at("SP6", DEFAULT_SEND_TIMEOUT_MS);
            self.at("CRA", DEFAULT_SEND_TIMEOUT_MS);
            let frames = self
                .monitor_lines(800, 12)
                .iter()
                .map(|line| strip_spaces(line))
                .filter(|hex| hex.len() >= 4 && all_hex(hex))
                .count();
            if frames >= 3 {
                self.listen_only = true;
                self.connected = true;
                self.protocol_id = '6';
                self.protocol_text = format!(
                    "ISO 15765-4 (CAN 11/500), {}",
                    l("nur Mithören", "listen only")
                );
                self.log(l(
                    "  Kein OBD, aber Verkehr auf dem CAN-Bus: verbunden zum Mithören.",
                    "  No OBD, but traffic on the CAN bus: connected for listening.",
                ));
                self.setup_stn();
                return Ok(());
            }
        }

        Err(l(
            "Kein Kontakt zum Steuergerät.\n  - Zündung an? (Motor darf aus sein)\n  - Stecker fest im OBD-Port?\n  - Manche Adapter brauchen einen zweiten Anlauf: abziehen, 10 s warten, neu stecken.",
            "No contact with the control unit.\n  - Ignition on? (The engine may be off)\n  - Plug firmly seated in the OBD port?\n  - Some adapters need a second attempt: unplug, wait 10 s, plug back in.",
        )
        .to_string())
    }

    /// Closes the protocol (ATPC) and drops the transport.
    pub fn disconnect(&mut self) {
        // An adapter that said nothing at all to the last command will not
        // answer this one either; asking would only cost the whole wait.
        if self.transport.is_some() && self.connected && !self.silent {
            self.at("PC", 2000);
        }
        self.silent = false;
        self.transport = None;
        self.connected = false;
        self.headers = false;
        self.j1939_kbit = 0;
        self.j1939_header = None;
        self.stn = false;
        self.stn_batch = false;
        self.stn_bus = StnBus::HighSpeed;
        self.obd_on_uds = false;
        self.listen_only = false;
        self.stn_id.clear();
        self.timing_level = 0;
        self.protocol_id = '0';
        self.protocol_text = l("unbekannt", "unknown").to_string();
    }
}

fn finish_uds(
    order: Vec<String>,
    payloads: Vec<Vec<u8>>,
    last: ElmResult,
    none: ElmResult,
) -> ElmResult {
    if payloads.is_empty() {
        return if last.ok() { none } else { last };
    }
    ElmResult {
        messages: payloads,
        sources: order,
        ..ElmResult::default()
    }
}

#[cfg(test)]
mod reset_tests {
    use super::Elm327;

    #[test]
    fn a_reset_answered_with_stopped_counts_as_interrupted() {
        let lines = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(Elm327::reset_was_interrupted_for_test(&lines(&["STOPPED"])));
        assert!(Elm327::reset_was_interrupted_for_test(&lines(&[
            "", "STOPPED"
        ])));
        assert!(!Elm327::reset_was_interrupted_for_test(&lines(&[
            "ELM327 v1.4b"
        ])));
        assert!(!Elm327::reset_was_interrupted_for_test(&lines(&[
            "STOPPED",
            "ELM327 v1.4b"
        ])));
        assert!(!Elm327::reset_was_interrupted_for_test(&lines(&[])));
    }

    #[test]
    fn scrambled_bytes_of_a_wrong_baud_rate_are_no_answer() {
        assert!(Elm327::reply_is_garbled_for_test("^\u{FFFD}\u{FFFD}"));
        assert!(Elm327::reply_is_garbled_for_test("~\u{1}\u{FFFD}F"));
        assert!(!Elm327::reply_is_garbled_for_test("ELM327 v1.4b"));
        assert!(!Elm327::reply_is_garbled_for_test(
            "OBDII to RS232 Interpreter"
        ));
        assert!(!Elm327::reply_is_garbled_for_test("?"));
        assert!(!Elm327::reply_is_garbled_for_test(""));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_bytes_skips_garbage_and_odd_digit() {
        // Every hex digit counts, the index too: "04902" -> 04 90.
        assert_eq!(hex_to_bytes("0: 49 02"), vec![0x04, 0x90]);
        assert_eq!(hex_to_bytes("4100B"), vec![0x41, 0x00]);
    }

    #[test]
    fn the_driver_can_move_to_a_worker_thread() {
        fn assert_send<T: Send>() {}
        assert_send::<Elm327>();
    }

    #[test]
    fn protocol_names_and_formats() {
        assert_eq!(protocol_name('1'), "SAE J1850 PWM");
        assert_eq!(header_format_for('0'), HeaderFormat::None);
        assert_eq!(header_format_for('2'), HeaderFormat::J1850);
    }
}
