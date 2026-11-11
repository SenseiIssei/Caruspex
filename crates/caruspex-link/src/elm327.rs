//! ELM327 driver: initialisation, protocol selection, commands, response parsing.

use caruspex_i18n::l;

use crate::timing::{now_ms, sleep_ms};
use crate::transport::{LogFn, Transport};

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

/// True if the adapter confirmed a setting with a line that says "OK".
fn answered_ok(result: &ElmResult) -> bool {
    if result.status == ElmStatus::AdapterError {
        return false;
    }
    result.lines.iter().any(|line| to_upper(line) == "OK")
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
        _ => l("unbekannt", "unknown"),
    }
}

/// Every hex digit in the text, two at a time. Anything else is skipped; an
/// odd last digit is dropped.
pub fn hex_to_bytes(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text
        .chars()
        .filter_map(|c| c.to_digit(16))
        .map(|d| d as u8)
        .collect();
    digits
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[high, low]| (high << 4) | low)
        .collect()
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
        let digits = strip_spaces(&to_upper(line));

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
    let mut result = ElmResult {
        lines: lines.iter().map(|s| s.as_ref().to_string()).collect(),
        ..ElmResult::default()
    };

    for line in lines {
        let line = line.as_ref();
        let up = to_upper(line);
        if up.starts_with("NO DATA") {
            result.status = ElmStatus::NoData;
            result.message = l(
                "Steuergerät unterstützt diese Anfrage nicht (NO DATA).",
                "Control unit does not support this request (NO DATA).",
            )
            .to_string();
            return result;
        }
        for err in ERRORS {
            if up == err || up.starts_with(&format!("{err} ")) || up.starts_with(&format!("{err}:"))
            {
                result.status = ElmStatus::BusError;
                result.message = format!("{}{}", l("Adapter meldet: ", "Adapter reports: "), line);
                return result;
            }
        }
    }

    let raw = split_messages(lines, headers);

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
            result.messages.push(bytes[1..].to_vec());
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
        let up = to_upper(line.trim());
        ERRORS
            .iter()
            .filter(|err| **err != "?")
            .any(|err| {
                up == *err
                    || up.starts_with(&format!("{err} "))
                    || up.starts_with(&format!("{err}:"))
            })
            .then(|| line.trim().to_string())
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
    obd_on_uds: bool,
    listen_only: bool,
    stn_id: String,
    version: String,
    protocol_text: String,
    protocol_id: char,
    headers: bool,
    connected: bool,
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
            obd_on_uds: false,
            listen_only: false,
            stn_id: String::new(),
            version: "?".to_string(),
            protocol_text: l("unbekannt", "unknown").to_string(),
            protocol_id: '0',
            headers: false,
            connected: false,
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

    fn drain_greeting(&mut self) {
        // Wi-Fi and Bluetooth adapters announce themselves unprompted. If this
        // greeting is left in the buffer, it answers the first command and all
        // following responses are shifted by one position.
        let Some(t) = self.transport.as_mut() else {
            return;
        };
        sleep_ms(300);
        t.flush_input();
        t.write(b"\r");
        sleep_ms(200);
        t.flush_input();
    }

    /// Sends one line and collects the response up to the prompt.
    pub fn send(&mut self, command: &str, timeout_ms: u64) -> ElmResult {
        let mut result = ElmResult::default();
        let scale = self.timeout_scale;
        let Some(t) = self.transport.as_mut() else {
            result.status = ElmStatus::AdapterError;
            result.message = l("Kein Adapter verbunden.", "No adapter connected.").to_string();
            return result;
        };
        t.flush_input();
        let mut line = command.as_bytes().to_vec();
        line.push(b'\r');
        t.write(&line);

        // The configured factor applies here, at the only place where we
        // actually wait.
        let wait_ms = (timeout_ms as f64 * scale) as u64;

        let mut buffer: Vec<u8> = Vec::new();
        let deadline = now_ms() + wait_ms;
        while now_ms() < deadline {
            let chunk = t.read(120);
            if !chunk.is_empty() {
                buffer.extend_from_slice(&chunk);
                if buffer.contains(&b'>') {
                    break;
                }
            }
        }
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

        let cleaned: Vec<u8> = buffer.into_iter().filter(|&c| c != b'>').collect();
        let text = String::from_utf8_lossy(&cleaned);
        let echo = to_upper(&strip_spaces(command));
        for raw in text.split(['\r', '\n']) {
            let line = trim(raw);
            if line.is_empty() {
                continue;
            }
            if to_upper(&strip_spaces(line)) == echo {
                continue;
            }
            if to_upper(line).starts_with("SEARCHING") {
                continue;
            }
            result.lines.push(line.to_string());
        }
        // Keep track of the header setting even when someone sends the AT
        // commands by hand (the raw console, the KKL reset). A reset switches
        // headers off again.
        let sent = to_upper(&strip_spaces(command));
        if sent == "ATH1" {
            self.headers = true;
        }
        if sent == "ATH0" || sent == "ATZ" || sent == "ATD" || sent == "ATWS" {
            self.headers = false;
        }

        if self.verbose && self.log.is_some() {
            let mut joined = String::new();
            for l in &result.lines {
                joined.push_str(l);
                joined.push_str(" | ");
            }
            self.log(&format!("  > {command}   ->   {joined}"));
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
        let raw = self.send(command, timeout_ms);
        if raw.status == ElmStatus::AdapterError {
            return raw;
        }
        decode_messages(&raw.lines, expected_service, self.header_format())
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
    pub fn monitor_lines(&mut self, duration_ms: u64, max_lines: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let Some(t) = self.transport.as_mut() else {
            return lines;
        };
        t.flush_input();
        t.write(b"ATMA\r");
        let mut buffer: Vec<u8> = Vec::new();
        let deadline = now_ms() + duration_ms;
        let complete = |b: &[u8]| b.iter().filter(|&&c| c == b'\r').count();
        while now_ms() < deadline && complete(&buffer) <= max_lines {
            buffer.extend(t.read(50));
            if buffer.contains(&b'>') {
                break; // stopped by itself (BUFFER FULL)
            }
        }
        if !buffer.contains(&b'>') {
            // Any character ends the monitor; the prompt follows.
            t.write(b"\r");
            let stop = now_ms() + 1500;
            while now_ms() < stop {
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
            if SKIP.contains(&up.as_str()) {
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
        for line in self.monitor_lines(duration_ms, max_frames) {
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
        let cra = self.at("CRA", DEFAULT_SEND_TIMEOUT_MS);
        if !answered_ok(&cra) {
            self.at("AR", DEFAULT_SEND_TIMEOUT_MS);
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
        self.stn = false;
        self.stn_id.clear();
        self.timing_level = 0;
        self.obd_on_uds = false;
        self.listen_only = false;
        self.trace = ConnectTrace::default();
        self.drain_greeting();

        let banner = self.at("Z", 6000);
        if banner.status == ElmStatus::AdapterError {
            return Err(banner.message);
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
        if self.transport.is_some() && self.connected {
            self.at("PC", 2000);
        }
        self.transport = None;
        self.connected = false;
        self.headers = false;
        self.stn = false;
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
