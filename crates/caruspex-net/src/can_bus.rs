//! Interfaces that hand over raw CAN frames instead of speaking ELM327:
//! SAE J2534 PassThru devices, SLCAN/Lawicel sticks (CANable, USBtin,
//! CANUSB) and SocketCAN on Linux. [`CanElm`] puts an ELM327 front end on top
//! of them, with the ISO-TP framing (ISO 15765-2) done here, so OBD, UDS, the
//! BMW addressing and listening to broadcast frames work through every one
//! of them exactly as through an ELM327.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use caruspex_i18n::l;
use caruspex_link::timing::now_ms;
use caruspex_link::{ByteStream, Transport};

use crate::text::{atoi, hex, is_hex, normalize, parse_hex, strtol_hex};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CanFrame {
    pub id: u32,
    /// 29-bit id.
    pub extended: bool,
    /// 0 to 8 bytes.
    pub data: Vec<u8>,
}

/// One CAN interface at a fixed bit rate.
pub trait CanLink: Send {
    fn send(&mut self, frame: &CanFrame) -> bool;
    /// The next frame within `timeout_ms`; `None` if none came.
    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame>;
    fn name(&self) -> String;
    /// Battery voltage at the socket if the interface can measure it.
    fn battery_voltage(&mut self) -> Option<f64> {
        None
    }
}

/// A [`CanLink`] that several users share, one call at a time: the ELM
/// front end and VAG TP 2.0 ([`crate::tp20::Tp20`]) on the same interface.
/// The interface closes when the last clone is dropped.
#[derive(Clone)]
pub struct SharedCanLink(Arc<Mutex<Box<dyn CanLink>>>);

impl SharedCanLink {
    pub fn new(link: Box<dyn CanLink>) -> Self {
        Self(Arc::new(Mutex::new(link)))
    }

    fn lock(&self) -> MutexGuard<'_, Box<dyn CanLink>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl CanLink for SharedCanLink {
    fn send(&mut self, frame: &CanFrame) -> bool {
        self.lock().send(frame)
    }
    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame> {
        self.lock().receive(timeout_ms)
    }
    fn name(&self) -> String {
        self.lock().name()
    }
    fn battery_voltage(&mut self) -> Option<f64> {
        self.lock().battery_voltage()
    }
}

/// ELM327 default answer time (ATST).
const FIRST_ANSWER_MS: u64 = 1000;
/// More units after the first one.
const MORE_ANSWERS_MS: u64 = 60;
/// After 7F xx 78.
const PENDING_MS: u64 = 5000;

// ---------------------------------------------------------------------------
// ELM327 front end
// ---------------------------------------------------------------------------

/// An ELM327 as far as Caruspex uses one, on top of a [`CanLink`]: AT commands,
/// OBD and UDS requests with ISO-TP (11 and 29 bit, functional and
/// physical), raw frames with auto formatting off (BMW 6F1 addressing) and
/// the bus monitor (ATMA) with the receive filter.
pub struct CanElm {
    link: SharedCanLink,
    name: String,
    bitrate: u32,
    pending: Vec<u8>,
    echo: bool,
    linefeeds: bool,
    spaces: bool,
    headers: bool,
    auto_format: bool,
    /// ATSP7/9: 29-bit ids.
    extended: bool,
    /// ATSH (with ATCP on top for 29 bit).
    header: u32,
    priority: u32,
    /// ATCRA.
    filter: Option<u32>,
    /// ATFCSH.
    flow_header: Option<u32>,
    monitoring: bool,
}

#[derive(Default)]
struct Answer {
    frames: Vec<CanFrame>,
    total: usize,
    have: usize,
    multi: bool,
    done: bool,
}

impl CanElm {
    pub fn new(link: Box<dyn CanLink>, name: impl Into<String>, bitrate: u32) -> Self {
        Self {
            link: SharedCanLink::new(link),
            name: name.into(),
            bitrate,
            pending: Vec::new(),
            echo: true,
            linefeeds: false,
            spaces: true,
            headers: false,
            auto_format: true,
            extended: false,
            header: 0x7DF,
            priority: 0x18,
            filter: None,
            flow_header: None,
            monitoring: false,
        }
    }

    /// The CAN interface underneath, for VAG TP 2.0 next to the ELM front end.
    pub fn shared_link(&self) -> SharedCanLink {
        self.link.clone()
    }

    fn reset(&mut self) {
        self.echo = true;
        self.linefeeds = false;
        self.spaces = true;
        self.headers = false;
        self.auto_format = true;
        self.extended = false;
        self.header = 0x7DF;
        self.priority = 0x18;
        self.filter = None;
        self.flow_header = None;
        self.monitoring = false;
    }

    fn end(&self) -> &'static str {
        if self.linefeeds { "\r\n" } else { "\r" }
    }

    fn hex(&self, bytes: &[u8]) -> String {
        hex(bytes, self.spaces)
    }

    fn line(&self, frame: &CanFrame, with_id: bool) -> String {
        let mut out = String::new();
        if with_id {
            out = if frame.extended {
                format!("{:08X}", frame.id)
            } else {
                format!("{:03X}", frame.id)
            };
            if self.spaces {
                out.push(' ');
            }
        }
        out + &self.hex(&frame.data)
    }

    /// Frames that arrived between two requests belong to nobody.
    fn drain(&mut self) {
        self.pending.clear();
        while !self.monitoring && self.link.receive(0).is_some() {}
    }

    fn handle(&mut self, command: &str) -> Vec<String> {
        if command.is_empty() {
            return Vec::new();
        }
        if let Some(rest) = command.strip_prefix("AT") {
            return self.at(rest);
        }
        if command.starts_with("ST") {
            return vec!["?".into()];
        }
        let mut digits = command.to_string();
        if digits.len() % 2 == 1 {
            digits.pop(); // "expected answers" digit
        }
        if !is_hex(&digits) {
            return vec!["?".into()];
        }
        if self.auto_format {
            self.request(&parse_hex(&digits))
        } else {
            self.raw(&parse_hex(&digits))
        }
    }

    fn at(&mut self, c: &str) -> Vec<String> {
        let ok = || vec!["OK".to_string()];
        let question = || vec!["?".to_string()];
        if c == "Z" || c == "WS" {
            self.reset();
            return vec!["ELM327 v2.1".into()];
        }
        if c == "D" {
            self.reset();
            return ok();
        }
        if c == "I" {
            return vec!["ELM327 v2.1".into()];
        }
        if c == "@1" {
            return vec![self.name.clone()];
        }
        if c == "E0" || c == "E1" {
            self.echo = c == "E1";
            return ok();
        }
        if c == "L0" || c == "L1" {
            self.linefeeds = c == "L1";
            return ok();
        }
        if c == "S0" || c == "S1" {
            self.spaces = c == "S1";
            return ok();
        }
        if c == "H0" || c == "H1" {
            self.headers = c == "H1";
            return ok();
        }
        if c == "CAF0" || c == "CAF1" {
            self.auto_format = c == "CAF1";
            return ok();
        }
        if c.starts_with("SP") || c.starts_with("TP") {
            // 6/8 are 11 bit, 7/9 are 29 bit; the bit rate is the interface's.
            let digit = c.as_bytes()[c.len() - 1];
            if digit == b'6' || digit == b'8' {
                self.extended = false;
            }
            if digit == b'7' || digit == b'9' {
                self.extended = true;
            }
            if (b'1'..=b'5').contains(&digit) {
                return question(); // no K-line or J1850 on a CAN interface
            }
            self.header = if self.extended { 0xDB33F1 } else { 0x7DF };
            return ok();
        }
        if c == "DP" || c == "DPN" {
            let slow = self.bitrate == 250000;
            if c == "DPN" {
                let digit = if self.extended {
                    if slow { '9' } else { '7' }
                } else if slow {
                    '8'
                } else {
                    '6'
                };
                return vec![digit.to_string()];
            }
            return vec![format!(
                "ISO 15765-4 (CAN {}/{})",
                if self.extended { 29 } else { 11 },
                self.bitrate as i32 / 1000
            )];
        }
        if let Some(id) = c.strip_prefix("SH") {
            if !is_hex(id) || (id.len() != 3 && id.len() != 6 && id.len() != 8) {
                return question();
            }
            let value = u32::from_str_radix(id, 16).unwrap_or(0);
            if id.len() == 8 {
                self.priority = value >> 24;
                self.header = value & 0xFFFFFF;
            } else {
                self.header = value;
            }
            return ok();
        }
        if c.starts_with("CP") && c.len() == 4 && is_hex(&c[2..]) {
            self.priority = u32::from_str_radix(&c[2..], 16).unwrap_or(0) & 0x1F;
            return ok();
        }
        if c == "CRA" || c == "AR" {
            self.filter = None;
            return ok();
        }
        if let Some(id) = c.strip_prefix("CRA") {
            if !is_hex(id) || (id.len() != 3 && id.len() != 8) {
                return question();
            }
            self.filter = u32::from_str_radix(id, 16).ok();
            return ok();
        }
        if let Some(id) = c.strip_prefix("FCSH") {
            if !is_hex(id) {
                return question();
            }
            // strtoul clamps what does not fit.
            self.flow_header = Some(u32::from_str_radix(id, 16).unwrap_or(u32::MAX));
            return ok();
        }
        if c == "FCSM0" {
            self.flow_header = None;
            return ok();
        }
        if c == "MA" {
            self.monitoring = true;
            return Vec::new();
        }
        if c == "RV" {
            return match self.link.battery_voltage() {
                Some(v) if v < 0.0 => question(),
                Some(v) => vec![format!("{v:.1}V")],
                None => question(),
            };
        }
        const ACCEPTED: [&str; 16] = [
            "AT", "ST", "FCSD", "FCSM", "PC", "M0", "R1", "AL", "NL", "V0", "V1", "D0", "D1",
            "CFC1", "CM", "CF",
        ];
        if ACCEPTED.iter().any(|a| c.starts_with(a)) {
            return ok();
        }
        question()
    }

    fn accepts(&self, f: &CanFrame, functional: bool) -> bool {
        if f.extended != self.extended || f.data.is_empty() {
            return false;
        }
        if let Some(filter) = self.filter {
            return f.id == filter;
        }
        if !self.extended {
            return if functional {
                (0x7E8..=0x7EF).contains(&f.id)
            } else {
                f.id == self.header.wrapping_add(8)
            };
        }
        if functional {
            return (f.id & 0x1FFF_0000) == 0x18DA_0000 && ((f.id >> 8) & 0xFF) == 0xF1;
        }
        let request = (self.priority << 24) | self.header;
        let swapped = (request & 0x1FFF_0000) | ((request & 0xFF) << 8) | ((request >> 8) & 0xFF);
        f.id == swapped
    }

    fn flow_control_id(&self, r: &CanFrame) -> u32 {
        if let Some(flow) = self.flow_header {
            if r.extended && flow <= 0xFFFFFF {
                return (self.priority << 24) | flow;
            }
            return flow;
        }
        if !r.extended {
            return r.id.wrapping_sub(8);
        }
        (r.id & 0x1FFF_0000) | ((r.id & 0xFF) << 8) | ((r.id >> 8) & 0xFF)
    }

    fn request_id(&self) -> u32 {
        if self.extended {
            (self.priority << 24) | self.header
        } else {
            self.header
        }
    }

    fn request(&mut self, bytes: &[u8]) -> Vec<String> {
        if bytes.is_empty() || bytes.len() > 7 {
            return vec!["?".into()];
        }
        let functional = if self.extended {
            (self.header & 0xFFFF00) == 0xDB3300
        } else {
            self.header == 0x7DF
        };
        let mut data = vec![bytes.len() as u8];
        data.extend_from_slice(bytes);
        data.resize(8, 0x00);
        let out = CanFrame {
            id: self.request_id(),
            extended: self.extended,
            data,
        };
        self.drain();
        if !self.link.send(&out) {
            return vec!["CAN ERROR".into()];
        }

        let mut order: Vec<u32> = Vec::new();
        let mut answers: HashMap<u32, Answer> = HashMap::new();
        let mut deadline = now_ms() + FIRST_ANSWER_MS;
        while now_ms() < deadline {
            let Some(f) = self.link.receive(deadline.saturating_sub(now_ms())) else {
                break;
            };
            if !self.accepts(&f, functional) {
                continue;
            }
            if !answers.contains_key(&f.id) {
                order.push(f.id);
            }
            let a = answers.entry(f.id).or_default();
            if a.done {
                continue;
            }
            let kind = f.data[0] >> 4;
            if kind == 0 {
                let len = usize::from(f.data[0] & 0x0F);
                let pending =
                    len >= 3 && f.data.len() > 3 && f.data[1] == 0x7F && f.data[3] == 0x78;
                a.frames.push(f);
                if pending {
                    deadline = deadline.max(now_ms() + PENDING_MS);
                    continue;
                }
                a.done = true;
            } else if kind == 1 && f.data.len() >= 2 {
                a.multi = true;
                a.total = (usize::from(f.data[0] & 0x0F) << 8) | usize::from(f.data[1]);
                a.have = f.data.len() - 2;
                // Flow control: send everything, no pause between frames.
                let fc = CanFrame {
                    id: self.flow_control_id(&f),
                    extended: f.extended,
                    data: vec![0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                };
                a.frames.push(f);
                self.link.send(&fc);
                deadline = deadline.max(now_ms() + FIRST_ANSWER_MS);
                continue;
            } else if kind == 2 && a.multi {
                a.have += f.data.len() - 1;
                a.frames.push(f);
                if a.have >= a.total {
                    a.done = true;
                } else {
                    continue;
                }
            } else {
                continue;
            }
            // A complete answer: a physical request is done, a functional one
            // waits a moment for further units.
            if !functional {
                break;
            }
            deadline = deadline.min(now_ms() + MORE_ANSWERS_MS);
        }

        let mut lines = Vec::new();
        for id in &order {
            let a = &answers[id];
            if self.headers {
                lines.extend(a.frames.iter().map(|fr| self.line(fr, true)));
                continue;
            }
            if !a.multi {
                for fr in &a.frames {
                    let len = usize::from(fr.data[0] & 0x0F).min(fr.data.len() - 1);
                    lines.push(self.hex(&fr.data[1..1 + len]));
                }
                continue;
            }
            lines.push(format!("{:03X}", a.total));
            for (index, fr) in a.frames.iter().enumerate() {
                let skip = if fr.data[0] >> 4 == 1 { 2 } else { 1 };
                let skip = skip.min(fr.data.len());
                lines.push(format!(
                    "{:X}:{}{}",
                    index % 16,
                    if self.spaces { " " } else { "" },
                    self.hex(&fr.data[skip..])
                ));
            }
        }
        if lines.is_empty() {
            return vec!["NO DATA".into()];
        }
        lines
    }

    fn raw(&mut self, bytes: &[u8]) -> Vec<String> {
        if bytes.is_empty() || bytes.len() > 8 {
            return vec!["?".into()];
        }
        let out = CanFrame {
            id: self.request_id(),
            extended: self.extended,
            data: bytes.to_vec(),
        };
        self.drain();
        if !self.link.send(&out) {
            return vec!["CAN ERROR".into()];
        }
        // Whatever answers, as it comes: the caller does the framing.
        let mut lines = Vec::new();
        let mut deadline = now_ms() + FIRST_ANSWER_MS;
        while now_ms() < deadline {
            let Some(f) = self.link.receive(deadline.saturating_sub(now_ms())) else {
                break;
            };
            if f.extended != self.extended {
                continue;
            }
            if self.filter.is_some_and(|filter| f.id != filter) {
                continue;
            }
            lines.push(self.line(&f, self.headers));
            deadline = deadline.min(now_ms() + MORE_ANSWERS_MS);
        }
        if lines.is_empty() {
            return vec!["NO DATA".into()];
        }
        lines
    }
}

impl Transport for CanElm {
    fn write(&mut self, data: &[u8]) -> bool {
        if self.monitoring {
            // Any character ends the monitor; the prompt follows.
            self.monitoring = false;
            let end = self.end();
            self.pending.extend_from_slice(end.as_bytes());
            self.pending.push(b'>');
            return true;
        }
        let command = normalize(data);
        let mut out = Vec::new();
        if self.echo {
            let cut = data.iter().position(|&c| c == b'\r').unwrap_or(data.len());
            out.extend_from_slice(&data[..cut]);
            out.push(b'\r');
        }
        let end = self.end();
        for line in self.handle(&command) {
            out.extend_from_slice(line.as_bytes());
            out.extend_from_slice(end.as_bytes());
        }
        if !self.monitoring {
            out.extend_from_slice(end.as_bytes());
            out.push(b'>');
        }
        self.pending.extend_from_slice(&out);
        true
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        if self.monitoring && self.pending.is_empty() {
            // Frames as they come, through the receive filter.
            let deadline = now_ms() + timeout_ms.max(1);
            while now_ms() < deadline {
                let Some(f) = self.link.receive(deadline.saturating_sub(now_ms())) else {
                    break;
                };
                if self.filter.is_some_and(|filter| f.id != filter) {
                    continue;
                }
                let line = self.line(&f, self.headers) + self.end();
                self.pending.extend_from_slice(line.as_bytes());
                if self.pending.len() > 200 {
                    break;
                }
            }
        }
        std::mem::take(&mut self.pending)
    }

    fn flush_input(&mut self) {
        self.drain();
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

// ---------------------------------------------------------------------------
// SLCAN / Lawicel
// ---------------------------------------------------------------------------

/// SLCAN / Lawicel ASCII protocol over a serial port: "S6\r" (500 kbit),
/// "O\r" (open), "t7DF8020100..." (11-bit frame), "T18DB33F1..." (29-bit).
pub struct SlcanLink {
    serial: Box<dyn Transport>,
    buffer: Vec<u8>,
    version: String,
    open: bool,
}

fn find(haystack: &[u8], needle: u8) -> Option<usize> {
    haystack.iter().position(|&c| c == needle)
}

/// Whether a serial port speaks SLCAN (answers "V" with a version).
/// Returns the version ("V1013").
pub fn probe_slcan(serial: &mut dyn Transport) -> Option<String> {
    serial.write(b"\r\r\r"); // ends whatever half command is pending
    serial.read(50);
    serial.flush_input();
    serial.write(b"V\r");
    let mut got = Vec::new();
    let deadline = now_ms() + 300;
    while now_ms() < deadline && find(&got, b'\r').is_none() {
        got.extend_from_slice(&serial.read(30));
    }
    let v = find(&got, b'V')?;
    if got.len() < v + 5 {
        return None;
    }
    let version = &got[v..v + 5];
    version[1..]
        .iter()
        .all(u8::is_ascii_hexdigit)
        .then(|| version.iter().map(|&b| b as char).collect())
}

impl SlcanLink {
    pub fn new(serial: Box<dyn Transport>) -> Self {
        Self {
            serial,
            buffer: Vec::new(),
            version: String::new(),
            open: false,
        }
    }

    /// Sends a command; false if the adapter answered BEL.
    fn command(&mut self, text: &str) -> bool {
        self.serial.write(format!("{text}\r").as_bytes());
        let mut got = Vec::new();
        let deadline = now_ms() + 300;
        while now_ms() < deadline {
            got.extend_from_slice(&self.serial.read(30));
            if find(&got, b'\r').is_some() || find(&got, 0x07).is_some() {
                break;
            }
        }
        find(&got, 0x07).is_none()
    }

    /// Sets the bit rate and opens the channel. `Err` with the reason.
    pub fn open(&mut self, bitrate: u32) -> Result<(), String> {
        const RATES: [(u32, char); 9] = [
            (10000, '0'),
            (20000, '1'),
            (50000, '2'),
            (100000, '3'),
            (125000, '4'),
            (250000, '5'),
            (500000, '6'),
            (800000, '7'),
            (1000000, '8'),
        ];
        let Some(code) = RATES
            .iter()
            .find(|(rate, _)| *rate == bitrate)
            .map(|(_, c)| *c)
        else {
            return Err(l(
                "Diese Bitrate kennt SLCAN nicht.",
                "SLCAN does not know this bit rate.",
            )
            .into());
        };
        let Some(version) = probe_slcan(self.serial.as_mut()) else {
            return Err(l(
                "Der Anschluss antwortet nicht wie ein SLCAN-Adapter (CANable, USBtin, CANUSB).",
                "The port does not answer like an SLCAN adapter (CANable, USBtin, CANUSB).",
            )
            .into());
        };
        self.version = version;
        self.command("C"); // closed first, so the bit rate can be set
        if !self.command(&format!("S{code}")) {
            return Err(l(
                "Der Adapter nimmt die Bitrate nicht an.",
                "The adapter refuses the bit rate.",
            )
            .into());
        }
        self.command("Z0"); // no time stamps
        if !self.command("O") {
            return Err(l(
                "Der CAN-Kanal lässt sich nicht öffnen.",
                "The CAN channel cannot be opened.",
            )
            .into());
        }
        self.open = true;
        Ok(())
    }

    fn parse_line(b: &[u8]) -> Option<CanFrame> {
        if b.is_empty() || (b[0] != b't' && b[0] != b'T') {
            return None; // "z"/"Z" send acks, BEL
        }
        let ext = b[0] == b'T';
        let id_len = if ext { 8 } else { 3 };
        if b.len() < 1 + id_len + 1 {
            return None;
        }
        let id = &b[1..1 + id_len];
        let dlc = i32::from(b[1 + id_len]) - i32::from(b'0');
        if !id.iter().all(u8::is_ascii_hexdigit) || !(0..=8).contains(&dlc) {
            return None;
        }
        let dlc = dlc as usize;
        if b.len() < 2 + id_len + dlc * 2 {
            return None;
        }
        let data = &b[2 + id_len..2 + id_len + dlc * 2];
        Some(CanFrame {
            extended: ext,
            id: id.iter().fold(0u32, |v, &c| {
                (v << 4) | (c as char).to_digit(16).unwrap_or(0)
            }),
            // strtol per byte, as the C++ version reads them
            data: data.chunks(2).map(|pair| strtol_hex(pair) as u8).collect(),
        })
    }
}

impl Drop for SlcanLink {
    fn drop(&mut self) {
        if self.open {
            self.serial.write(b"C\r");
        }
    }
}

impl CanLink for SlcanLink {
    fn send(&mut self, frame: &CanFrame) -> bool {
        let mut text = if frame.extended {
            format!("T{:08X}{}", frame.id, frame.data.len())
        } else {
            format!("t{:03X}{}", frame.id & 0x7FF, frame.data.len())
        };
        for byte in &frame.data {
            text += &format!("{byte:02X}");
        }
        text.push('\r');
        self.serial.write(text.as_bytes())
    }

    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame> {
        let deadline = now_ms() + timeout_ms;
        loop {
            while let Some(end) = find(&self.buffer, b'\r') {
                let line: Vec<u8> = self.buffer.drain(..=end).take(end).collect();
                if let Some(frame) = Self::parse_line(&line) {
                    return Some(frame);
                }
            }
            let now = now_ms();
            if now >= deadline {
                return None;
            }
            let chunk = self.serial.read((deadline - now).min(50));
            self.buffer.extend_from_slice(&chunk);
            if timeout_ms == 0 && find(&self.buffer, b'\r').is_none() {
                return None;
            }
        }
    }

    fn name(&self) -> String {
        let mut name = format!("SLCAN {}", self.serial.name());
        if !self.version.is_empty() {
            name += &format!(" {}", self.version);
        }
        name
    }
}

// ---------------------------------------------------------------------------
// SocketCAN (Linux)
// ---------------------------------------------------------------------------

/// CAN network interfaces of the system ("can0", "vcan0"), empty elsewhere.
pub fn socket_can_interfaces() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        crate::socketcan::interfaces()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

/// The interface must already be up at the right bit rate
/// (ip link set can0 up type can bitrate 500000).
pub fn open_socket_can(interface: &str) -> Result<Box<dyn CanLink>, String> {
    #[cfg(target_os = "linux")]
    {
        crate::socketcan::open(interface)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = interface;
        Err(l(
            "SocketCAN gibt es nur unter Linux.",
            "SocketCAN exists on Linux only.",
        )
        .into())
    }
}

// ---------------------------------------------------------------------------
// SAE J2534 PassThru (Windows)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct J2534Device {
    /// As registered, "OpenPort 2.0 J2534 ISO/CAN/VPW/PWM".
    pub name: String,
    pub vendor: String,
    /// Path of the driver DLL.
    pub library: String,
    /// False: a 32-bit driver, which a 64-bit program cannot load.
    pub usable: bool,
}

/// The registered PassThru drivers (HKLM\SOFTWARE\PassThruSupport.04.04).
pub fn j2534_devices() -> Vec<J2534Device> {
    #[cfg(windows)]
    {
        crate::j2534::devices()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Opens the device through its driver DLL and connects CAN at the bit rate.
pub fn open_j2534(library: &str, bitrate: u32) -> Result<Box<dyn CanLink>, String> {
    #[cfg(windows)]
    {
        if library.is_empty() {
            return Err(l(
                "Kein J2534-Gerät eingerichtet. Ist der Treiber des Herstellers installiert?",
                "No J2534 device set up. Is the maker's driver installed?",
            )
            .into());
        }
        crate::j2534::open(library, bitrate)
    }
    #[cfg(not(windows))]
    {
        let _ = (library, bitrate);
        Err(l(
            "J2534-Geräte gibt es nur unter Windows.",
            "J2534 devices exist on Windows only.",
        )
        .into())
    }
}

/// The K-line protocols of a PassThru device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KLineKind {
    /// ProtocolID 3.
    Iso9141,
    /// ProtocolID 4 (KWP2000 on the K-line).
    Iso14230,
}

/// The driver DLL for a registered device name, a DLL path as it is, or the
/// first usable driver when `place` is empty.
pub fn j2534_library(place: &str) -> String {
    let all = j2534_devices();
    let mut library = place.to_string();
    for d in &all {
        if d.name == place {
            library = d.library.clone();
        }
    }
    if library.is_empty()
        && let Some(usable) = all.iter().find(|d| d.usable)
    {
        library = usable.library.clone();
    }
    library
}

/// The K-line of a PassThru device as a byte stream for KWP1281 and
/// KWP2000, with the 5-baud and the fast init done by the device. `device`
/// is a registered name or a DLL path (empty: the first usable driver).
/// Returns the stream and its name for the log.
pub fn open_j2534_kline(
    device: &str,
    kind: KLineKind,
    baud: u32,
) -> Result<(Box<dyn ByteStream>, String), String> {
    #[cfg(windows)]
    {
        let library = j2534_library(device);
        if library.is_empty() {
            return Err(l(
                "Kein J2534-Gerät eingerichtet. Ist der Treiber des Herstellers installiert?",
                "No J2534 device set up. Is the maker's driver installed?",
            )
            .into());
        }
        let stream = crate::j2534::open_kline(&library, kind, baud)?;
        let name = stream.name();
        Ok((Box::new(stream), name))
    }
    #[cfg(not(windows))]
    {
        let _ = (device, kind, baud);
        Err(l(
            "J2534-Geräte gibt es nur unter Windows.",
            "J2534 devices exist on Windows only.",
        )
        .into())
    }
}

// ---------------------------------------------------------------------------
// Choosing an interface
// ---------------------------------------------------------------------------

/// Opens a serial port at a baud rate: the serial transport of caruspex-link,
/// or a fake in the tests.
pub type SerialOpener<'a> = &'a dyn Fn(&str, u32) -> Result<Box<dyn Transport>, String>;

/// "slcan:COM5", "slcan:/dev/ttyACM0", "socketcan:can0", "j2534:<dll path or
/// device name>"; an optional "@250000" sets the bit rate (500000 if left out).
/// Returns the ELM front end, or the reason. SLCAN opens its port at
/// 115200 baud through `open_serial`.
pub fn open_can_interface(spec: &str, open_serial: SerialOpener<'_>) -> Result<CanElm, String> {
    let mut target = spec;
    let mut bitrate: i64 = 500000;
    if let Some(at) = spec.rfind('@') {
        bitrate = i64::from(atoi(&spec.as_bytes()[at + 1..]) as i32);
        target = &spec[..at];
    }
    let (kind, place) = match target.find(':') {
        Some(colon) => (&target[..colon], &target[colon + 1..]),
        None => (target, ""),
    };
    let rate = format!(" ({} kbit/s)", bitrate / 1000);
    // As the C++ version passes it on: a negative rate wraps around.
    let bitrate = bitrate as u32;

    let link: Box<dyn CanLink> = match kind {
        "slcan" => {
            let serial = open_serial(place, 115200)?;
            let mut slcan = SlcanLink::new(serial);
            slcan.open(bitrate)?;
            Box::new(slcan)
        }
        "socketcan" => open_socket_can(if place.is_empty() { "can0" } else { place })?,
        "j2534" => open_j2534(&j2534_library(place), bitrate)?,
        _ => {
            return Err(format!(
                "{}{}",
                l("Unbekannte CAN-Schnittstelle: ", "Unknown CAN interface: "),
                spec
            ));
        }
    };
    let name = link.name() + &rate;
    Ok(CanElm::new(link, name, bitrate))
}
