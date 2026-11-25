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
use caruspex_link::{ByteStream, IsoTpBatch, IsoTpRequest, Transport};

use crate::elm_front::{self, ElmBackend, ElmFront};
use crate::text::{atoi, is_hex, strtol_hex};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CanFrame {
    pub id: u32,
    /// 29-bit id.
    pub extended: bool,
    /// 0 to 8 bytes; up to 64 on a CAN FD link.
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
    /// A CAN FD link: every frame goes out as an FD frame and frames of up
    /// to 64 bytes come in. Off unless asked for when opening.
    fn fd(&self) -> bool {
        false
    }
    /// The interface is known to be gone (unplugged, taken down), see
    /// [`Transport::lost`].
    fn lost(&mut self) -> bool {
        false
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
    fn fd(&self) -> bool {
        self.lock().fd()
    }
    fn lost(&mut self) -> bool {
        self.lock().lost()
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
    /// Display settings, ATSH (with ATCP on top for 29 bit) and ATCRA.
    front: ElmFront,
    /// ATSP7/9: 29-bit ids.
    extended: bool,
    priority: u32,
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
    /// Came in CAN FD frames (longer than 8 bytes, or with the escape
    /// lengths of ISO 15765-2:2016).
    fd: bool,
}

/// Where the data of a single or first frame begins and how long the
/// message is, with the escapes of CAN FD: a single frame longer than 8
/// bytes has length 0 in its PCI byte and the length in the next byte, a
/// first frame of more than 4095 bytes has length 0 and four length bytes.
fn pci_length(data: &[u8]) -> (usize, usize, bool) {
    match data[0] >> 4 {
        0 if data[0] & 0x0F == 0 && data.len() > 8 => (2, usize::from(data[1]), true),
        0 => (1, usize::from(data[0] & 0x0F), false),
        _ => {
            let short = (usize::from(data[0] & 0x0F) << 8) | usize::from(data[1]);
            if short == 0 && data.len() >= 6 {
                let long = u32::from_be_bytes([data[2], data[3], data[4], data[5]]);
                (6, long as usize, true)
            } else {
                (2, short, data.len() > 8)
            }
        }
    }
}

/// An answer that came in CAN FD frames as classic frames of 8 bytes, as an
/// ELM327 would have shown it: whoever reads the lines needs to know only
/// classic ISO-TP. Longer than 4095 bytes does not fit in classic frames
/// and stays as it came.
fn as_classic(a: &mut Answer) {
    let Some(first) = a.frames.first() else {
        return;
    };
    let (id, extended) = (first.id, first.extended);
    let mut payload = Vec::new();
    let mut total = 0;
    for f in &a.frames {
        match f.data[0] >> 4 {
            0 => {
                let (start, length, _) = pci_length(&f.data);
                let end = (start + length).min(f.data.len());
                payload = f.data[start.min(end)..end].to_vec();
                total = payload.len();
            }
            1 => {
                let (start, length, _) = pci_length(&f.data);
                total = length;
                payload = f.data[start.min(f.data.len())..].to_vec();
            }
            _ => payload.extend_from_slice(&f.data[1..]),
        }
    }
    if total > 0xFFF {
        return;
    }
    payload.truncate(total);
    let frame = |data: Vec<u8>| CanFrame { id, extended, data };
    let mut frames = Vec::new();
    if payload.len() <= 7 {
        let mut d = vec![payload.len() as u8];
        d.extend_from_slice(&payload);
        d.resize(8, 0x00);
        frames.push(frame(d));
    } else {
        let mut d = vec![
            0x10 | (payload.len() >> 8) as u8,
            (payload.len() & 0xFF) as u8,
        ];
        d.extend_from_slice(&payload[..6]);
        frames.push(frame(d));
        for (n, chunk) in payload[6..].chunks(7).enumerate() {
            let mut d = vec![0x20 | ((n + 1) & 0x0F) as u8];
            d.extend_from_slice(chunk);
            d.resize(8, 0x00);
            frames.push(frame(d));
        }
    }
    a.multi = payload.len() > 7;
    a.total = payload.len();
    a.frames = frames;
}

impl CanElm {
    pub fn new(link: Box<dyn CanLink>, name: impl Into<String>, bitrate: u32) -> Self {
        Self {
            link: SharedCanLink::new(link),
            name: name.into(),
            bitrate,
            front: ElmFront::new(),
            extended: false,
            priority: 0x18,
            flow_header: None,
            monitoring: false,
        }
    }

    /// The CAN interface underneath, for VAG TP 2.0 next to the ELM front end.
    pub fn shared_link(&self) -> SharedCanLink {
        self.link.clone()
    }

    fn reset(&mut self) {
        self.front.reset();
        self.extended = false;
        self.priority = 0x18;
        self.flow_header = None;
        self.monitoring = false;
    }

    fn end(&self) -> &'static str {
        if self.front.linefeeds { "\r\n" } else { "\r" }
    }

    fn hex(&self, bytes: &[u8]) -> String {
        self.front.hex(bytes)
    }

    fn line(&self, frame: &CanFrame, with_id: bool) -> String {
        let mut out = String::new();
        if with_id {
            out = if frame.extended {
                format!("{:08X}", frame.id)
            } else {
                format!("{:03X}", frame.id)
            };
            if self.front.spaces {
                out.push(' ');
            }
        }
        out + &self.hex(&frame.data)
    }

    /// Frames that arrived between two requests belong to nobody.
    fn drain(&mut self) {
        self.front.pending.clear();
        while !self.monitoring && self.link.receive(0).is_some() {}
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
            self.front.header = if self.extended { 0xDB33F1 } else { 0x7DF };
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
                self.front.header = value & 0xFFFFFF;
            } else {
                self.front.header = value;
            }
            return ok();
        }
        if c.starts_with("CP") && c.len() == 4 && is_hex(&c[2..]) {
            self.priority = u32::from_str_radix(&c[2..], 16).unwrap_or(0) & 0x1F;
            return ok();
        }
        if c == "CRA" || c == "AR" {
            self.front.filter = None;
            return ok();
        }
        if let Some(id) = c.strip_prefix("CRA") {
            if !is_hex(id) || (id.len() != 3 && id.len() != 8) {
                return question();
            }
            self.front.filter = u32::from_str_radix(id, 16).ok();
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
        if let Some(filter) = self.front.filter {
            return f.id == filter;
        }
        if !self.extended {
            return if functional {
                (0x7E8..=0x7EF).contains(&f.id)
            } else {
                f.id == self.front.header.wrapping_add(8)
            };
        }
        if functional {
            return (f.id & 0x1FFF_0000) == 0x18DA_0000 && ((f.id >> 8) & 0xFF) == 0xF1;
        }
        let request = (self.priority << 24) | self.front.header;
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
            (self.priority << 24) | self.front.header
        } else {
            self.front.header
        }
    }

    fn request(&mut self, bytes: &[u8]) -> Vec<String> {
        if bytes.is_empty() || bytes.len() > 7 {
            return vec!["?".into()];
        }
        let functional = if self.extended {
            (self.front.header & 0xFFFF00) == 0xDB3300
        } else {
            self.front.header == 0x7DF
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
            if f.data.len() > 8 {
                a.fd = true;
            }
            if kind == 0 {
                let (start, len, escaped) = pci_length(&f.data);
                a.fd |= escaped;
                let pending = len >= 3
                    && f.data.len() > start + 2
                    && f.data[start] == 0x7F
                    && f.data[start + 2] == 0x78;
                a.frames.push(f);
                if pending {
                    deadline = deadline.max(now_ms() + PENDING_MS);
                    continue;
                }
                a.done = true;
            } else if kind == 1 && f.data.len() >= 2 {
                a.multi = true;
                let (start, total, escaped) = pci_length(&f.data);
                a.fd |= escaped;
                a.total = total;
                a.have = f.data.len().saturating_sub(start);
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

        for a in answers.values_mut().filter(|a| a.fd) {
            as_classic(a);
        }
        let mut lines = Vec::new();
        for id in &order {
            let a = &answers[id];
            if self.front.headers {
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
                    if self.front.spaces { " " } else { "" },
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
            if self.front.filter.is_some_and(|filter| f.id != filter) {
                continue;
            }
            lines.push(self.line(&f, self.front.headers));
            deadline = deadline.min(now_ms() + MORE_ANSWERS_MS);
        }
        if lines.is_empty() {
            return vec!["NO DATA".into()];
        }
        lines
    }
}

impl ElmBackend for CanElm {
    fn front(&mut self) -> &mut ElmFront {
        &mut self.front
    }
    fn at(&mut self, command: &str) -> Vec<String> {
        CanElm::at(self, command)
    }
    fn formatted(&mut self, bytes: &[u8]) -> Vec<String> {
        self.request(bytes)
    }
    fn raw(&mut self, bytes: &[u8]) -> Vec<String> {
        CanElm::raw(self, bytes)
    }
    fn prompt(&self) -> bool {
        !self.monitoring
    }
}

impl Transport for CanElm {
    fn write(&mut self, data: &[u8]) -> bool {
        if self.monitoring {
            // Any character ends the monitor; the prompt follows.
            self.monitoring = false;
            let end = self.end();
            self.front.pending.extend_from_slice(end.as_bytes());
            self.front.pending.push(b'>');
            return true;
        }
        elm_front::respond(self, data);
        true
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        if self.monitoring && self.front.pending.is_empty() {
            // Frames as they come, through the receive filter.
            let deadline = now_ms() + timeout_ms.max(1);
            while now_ms() < deadline {
                let Some(f) = self.link.receive(deadline.saturating_sub(now_ms())) else {
                    break;
                };
                if self.front.filter.is_some_and(|filter| f.id != filter) {
                    continue;
                }
                let line = self.line(&f, self.front.headers) + self.end();
                self.front.pending.extend_from_slice(line.as_bytes());
                if self.front.pending.len() > 200 {
                    break;
                }
            }
        }
        std::mem::take(&mut self.front.pending)
    }

    fn flush_input(&mut self) {
        self.drain();
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn lost(&mut self) -> bool {
        self.link.lost()
    }

    fn iso_tp_batch(&mut self) -> Option<&mut dyn IsoTpBatch> {
        if self.monitoring { None } else { Some(self) }
    }
}

// ---------------------------------------------------------------------------
// Several ISO-TP sessions at once
// ---------------------------------------------------------------------------

/// One control unit of a batch: its request and the answer coming in.
struct Session {
    index: usize,
    deadline: u64,
    done: bool,
    /// The message being put together, and its length.
    payload: Vec<u8>,
    total: usize,
    collecting: bool,
}

impl Session {
    /// Takes one answer frame (PCI first); a complete message when there is one.
    fn take(&mut self, d: &[u8]) -> Option<Vec<u8>> {
        if d.is_empty() {
            return None;
        }
        match d[0] >> 4 {
            0 => {
                let (start, length, _) = pci_length(d);
                if length == 0 || d.len() < start + length {
                    return None;
                }
                self.collecting = false;
                Some(d[start..start + length].to_vec())
            }
            1 if d.len() >= 2 => {
                let (start, total, _) = pci_length(d);
                self.total = total;
                self.payload = d[start.min(d.len())..].to_vec();
                self.collecting = true;
                None
            }
            2 if self.collecting => {
                self.payload.extend_from_slice(&d[1..]);
                if self.payload.len() < self.total {
                    return None;
                }
                self.collecting = false;
                let mut done = std::mem::take(&mut self.payload);
                done.truncate(self.total);
                Some(done)
            }
            _ => None,
        }
    }
}

fn session_key(r: &IsoTpRequest) -> (u32, bool, bool) {
    (r.response, r.extended, r.target.is_some())
}

/// The tester's address in the answers of BMW style extended addressing.
const TESTER: u8 = 0xF1;

impl CanElm {
    /// One round: every unit of `round` asked at once, the answers collected
    /// as they come. No two units of a round answer on the same id.
    fn batch_round(
        &mut self,
        requests: &[IsoTpRequest],
        round: &[usize],
        timeout_ms: u64,
        out: &mut [Vec<Vec<u8>>],
    ) {
        self.drain();
        let start = now_ms();
        let mut sessions: Vec<Session> = Vec::new();
        for &i in round {
            let r = &requests[i];
            let room = if r.target.is_some() { 6 } else { 7 };
            if r.data.is_empty() || r.data.len() > room {
                continue; // only single frames
            }
            let mut data: Vec<u8> = r.target.into_iter().collect();
            data.push(r.data.len() as u8);
            data.extend_from_slice(&r.data);
            data.resize(8, 0x00);
            let frame = CanFrame {
                id: r.request,
                extended: r.extended,
                data,
            };
            if !self.link.send(&frame) {
                continue;
            }
            sessions.push(Session {
                index: i,
                deadline: start + timeout_ms,
                done: false,
                payload: Vec::new(),
                total: 0,
                collecting: false,
            });
        }
        loop {
            let open: Vec<&Session> = sessions.iter().filter(|s| !s.done).collect();
            let Some(until) = open.iter().map(|s| s.deadline).max() else {
                return;
            };
            let now = now_ms();
            if now >= until {
                return;
            }
            let Some(f) = self.link.receive(until - now) else {
                return;
            };
            let Some(s) = sessions.iter_mut().find(|s| {
                let r = &requests[s.index];
                !s.done
                    && f.id == r.response
                    && f.extended == r.extended
                    && (r.target.is_none() || f.data.first() == Some(&TESTER))
            }) else {
                continue;
            };
            let r = &requests[s.index];
            let d = if r.target.is_some() {
                &f.data[1..]
            } else {
                &f.data[..]
            };
            if d.first().is_some_and(|pci| pci >> 4 == 1) {
                // Flow control: everything, no pause between frames.
                let mut fc: Vec<u8> = r.target.into_iter().collect();
                fc.extend_from_slice(&[0x30, 0x00, 0x00]);
                fc.resize(8, 0x00);
                let flow = CanFrame {
                    id: r.request,
                    extended: r.extended,
                    data: fc,
                };
                s.deadline = s.deadline.max(now_ms() + FIRST_ANSWER_MS);
                s.take(d);
                self.link.send(&flow);
                continue;
            }
            let Some(message) = s.take(d) else {
                continue;
            };
            let pending = message.len() >= 3 && message[0] == 0x7F && message[2] == 0x78;
            out[s.index].push(message);
            if pending {
                s.deadline = s.deadline.max(now_ms() + PENDING_MS);
            } else {
                s.done = true;
            }
        }
    }
}

impl IsoTpBatch for CanElm {
    fn exchange_all(&mut self, requests: &[IsoTpRequest], timeout_ms: u64) -> Vec<Vec<Vec<u8>>> {
        let mut out = vec![Vec::new(); requests.len()];
        let mut left: Vec<usize> = (0..requests.len()).collect();
        while !left.is_empty() {
            // Units that answer on the same id go into the next round.
            let mut round: Vec<usize> = Vec::new();
            let mut later = Vec::new();
            for i in left {
                let key = session_key(&requests[i]);
                if round.iter().any(|&j| session_key(&requests[j]) == key) {
                    later.push(i);
                } else {
                    round.push(i);
                }
            }
            self.batch_round(requests, &round, timeout_ms, &mut out);
            left = later;
        }
        out
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

    fn lost(&mut self) -> bool {
        self.serial.lost()
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

fn open_socket_can_with(interface: &str, fd: bool) -> Result<Box<dyn CanLink>, String> {
    #[cfg(target_os = "linux")]
    {
        crate::socketcan::open(interface, fd)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (interface, fd);
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
    /// False: a 32-bit driver and no helper program to load it with.
    pub usable: bool,
    /// A 32-bit driver, loaded by the helper program caruspex-j2534-bridge.
    pub bridged: bool,
}

/// The helper program for 32-bit J2534 drivers to use instead of the one
/// next to the program; `None` goes back to that one. For the tests and
/// development builds.
pub fn set_j2534_bridge(path: Option<std::path::PathBuf>) {
    #[cfg(windows)]
    crate::j2534::set_bridge(path);
    #[cfg(not(windows))]
    let _ = path;
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
    open_j2534_with(library, bitrate, false)
}

/// Like [`open_j2534`] with CAN FD: the channel is connected with the
/// FD_CAN protocol of J2534-2 (newer devices), the data phase runs at the
/// device's own rate.
pub fn open_j2534_fd(library: &str, bitrate: u32) -> Result<Box<dyn CanLink>, String> {
    open_j2534_with(library, bitrate, true)
}

fn open_j2534_with(library: &str, bitrate: u32, fd: bool) -> Result<Box<dyn CanLink>, String> {
    #[cfg(windows)]
    {
        if library.is_empty() {
            return Err(l(
                "Kein J2534-Gerät eingerichtet. Ist der Treiber des Herstellers installiert?",
                "No J2534 device set up. Is the maker's driver installed?",
            )
            .into());
        }
        crate::j2534::open(library, bitrate, fd)
    }
    #[cfg(not(windows))]
    {
        let _ = (library, bitrate, fd);
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
pub(crate) fn j2534_library(place: &str) -> String {
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
// PEAK PCAN-Basic and Kvaser CANlib (Windows)
// ---------------------------------------------------------------------------

/// A channel of a maker's CAN library, as the interface list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VendorChannel {
    /// "PCAN · USB1", "Kvaser · Kvaser Leaf Light v2 channel 1".
    pub label: String,
    /// For [`open_can_interface`]: "pcan:USB1", "kvaser:0".
    pub spec: String,
}

/// The CAN library DLLs of the makers: PCAN-Basic and Kvaser CANlib.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanVendor {
    Pcan,
    Kvaser,
}

impl CanVendor {
    /// The DLL as the maker's installer puts it into the system folder.
    pub fn library(self) -> &'static str {
        match self {
            CanVendor::Pcan => "PCANBasic.dll",
            CanVendor::Kvaser => "canlib32.dll",
        }
    }
}

/// The channels of PCAN-Basic and Kvaser CANlib that have hardware behind
/// them; empty when the libraries are not installed or not on Windows.
pub fn can_vendor_channels() -> Vec<VendorChannel> {
    let mut out = vendor_channels_in(CanVendor::Pcan, CanVendor::Pcan.library());
    out.extend(vendor_channels_in(
        CanVendor::Kvaser,
        CanVendor::Kvaser.library(),
    ));
    out
}

/// The channels of one library loaded from `library` (a path, for the tests).
pub fn vendor_channels_in(vendor: CanVendor, library: &str) -> Vec<VendorChannel> {
    #[cfg(windows)]
    {
        match vendor {
            CanVendor::Pcan => crate::vendor_can::pcan_channels(library),
            CanVendor::Kvaser => crate::vendor_can::kvaser_channels(library),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (vendor, library);
        Vec::new()
    }
}

/// Opens a channel ("USB1" for PCAN, "0" for Kvaser; empty: the first) of
/// the library loaded from `library` at the bit rate.
pub fn open_vendor_can(
    vendor: CanVendor,
    library: &str,
    channel: &str,
    bitrate: u32,
) -> Result<Box<dyn CanLink>, String> {
    #[cfg(windows)]
    {
        match vendor {
            CanVendor::Pcan => crate::vendor_can::open_pcan(library, channel, bitrate),
            CanVendor::Kvaser => crate::vendor_can::open_kvaser(library, channel, bitrate),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (vendor, library, channel, bitrate);
        Err(l(
            "PCAN-Basic und Kvaser CANlib gibt es hier nur unter Windows; unter Linux gehen beide über SocketCAN.",
            "PCAN-Basic and Kvaser CANlib are used on Windows only here; on Linux both work through SocketCAN.",
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
/// device name>", "pcan:USB1", "kvaser:0"; an optional "@250000" sets the bit rate (500000 if left out).
/// "+fd" right before the bit rate ("socketcan:can0+fd@500000") switches CAN
/// FD on, for SocketCAN and J2534 devices that have it.
/// Returns the ELM front end, or the reason. SLCAN opens its port at
/// 115200 baud through `open_serial`.
pub fn open_can_interface(spec: &str, open_serial: SerialOpener<'_>) -> Result<CanElm, String> {
    let mut target = spec;
    let mut bitrate: i64 = 500000;
    if let Some(at) = spec.rfind('@') {
        bitrate = i64::from(atoi(&spec.as_bytes()[at + 1..]) as i32);
        target = &spec[..at];
    }
    let (target, fd) = match target.strip_suffix("+fd") {
        Some(rest) => (rest, true),
        None => (target, false),
    };
    let (kind, place) = match target.find(':') {
        Some(colon) => (&target[..colon], &target[colon + 1..]),
        None => (target, ""),
    };
    // The link says "CAN FD" in its name itself.
    let rate = format!(" ({} kbit/s)", bitrate / 1000);
    if fd && !matches!(kind, "socketcan" | "j2534") {
        return Err(l(
            "CAN FD geht hier nur über SocketCAN und J2534-Geräte.",
            "CAN FD works only through SocketCAN and J2534 devices here.",
        )
        .into());
    }
    // As the C++ version passes it on: a negative rate wraps around.
    let bitrate = bitrate as u32;

    let link: Box<dyn CanLink> = match kind {
        "slcan" => {
            let serial = open_serial(place, 115200)?;
            let mut slcan = SlcanLink::new(serial);
            slcan.open(bitrate)?;
            Box::new(slcan)
        }
        "socketcan" => open_socket_can_with(if place.is_empty() { "can0" } else { place }, fd)?,
        "j2534" => open_j2534_with(&j2534_library(place), bitrate, fd)?,
        "pcan" => open_vendor_can(CanVendor::Pcan, CanVendor::Pcan.library(), place, bitrate)?,
        "kvaser" => open_vendor_can(
            CanVendor::Kvaser,
            CanVendor::Kvaser.library(),
            place,
            bitrate,
        )?,
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
