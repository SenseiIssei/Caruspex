//! A small stand-in for the Elm327 driver of caruspex-link, so the ELM327
//! front ends of this crate can be driven the way the driver drives them:
//! the same command sequence on connect, the same line cleaning, ISO-TP
//! lines put back together, and the BMW extended addressing of Uds.
//! The full Elm327/Obd2/Uds stack is tested in the crates that own it.

#![allow(dead_code)]

use caruspex_link::Transport;
use caruspex_link::timing::now_ms;

fn strip(s: &str) -> String {
    s.chars()
        .filter(|c| *c != ' ')
        .collect::<String>()
        .to_ascii_uppercase()
}

fn hex_bytes(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(
        (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
            .collect(),
    )
}

/// One answer: the sending unit as the adapter shows it ("7E8", empty
/// without headers) and its bytes, service byte first.
#[derive(Debug, Clone)]
pub struct Message {
    pub source: String,
    pub data: Vec<u8>,
}

/// The frames of one answer with extended addressing: tester address,
/// PCI, data.
#[derive(Default)]
struct Reassembly {
    payload: Vec<u8>,
    expected: usize,
    next_index: u8,
    first_frame: bool,
    answered: bool,
}

impl Reassembly {
    fn take(&mut self, frames: impl Iterator<Item = Vec<u8>>, tester: u8) {
        for b in frames {
            if b.len() < 2 || b[0] != tester {
                continue;
            }
            let pci = b[1];
            match pci >> 4 {
                0 if !self.first_frame => {
                    let length = usize::from(pci & 0x0F);
                    if length == 0 || b.len() < 2 + length {
                        continue;
                    }
                    // A pending answer may come first; the last single frame wins.
                    self.payload = b[2..2 + length].to_vec();
                    self.expected = length;
                    self.answered = true;
                }
                1 if b.len() >= 3 => {
                    self.expected = (usize::from(pci & 0x0F) << 8) | usize::from(b[2]);
                    self.payload = b[3..].to_vec();
                    self.first_frame = true;
                    self.next_index = 1;
                    self.answered = true;
                }
                2 if self.first_frame => {
                    if pci & 0x0F != self.next_index & 0x0F {
                        continue; // repeated or out of order
                    }
                    self.next_index = self.next_index.wrapping_add(1);
                    self.payload.extend_from_slice(&b[2..]);
                }
                _ => {}
            }
        }
    }
}

pub struct MiniElm<T: Transport> {
    pub transport: T,
    pub headers: bool,
    pub obd_on_uds: bool,
    pub bits29: bool,
    pub protocol_text: String,
}

impl<T: Transport> MiniElm<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            headers: false,
            obd_on_uds: false,
            bits29: false,
            protocol_text: String::new(),
        }
    }

    /// As Elm327::send: the cleaned lines, empty if nothing came.
    pub fn send(&mut self, command: &str, timeout_ms: u64) -> Vec<String> {
        self.transport.flush_input();
        self.transport.write(format!("{command}\r").as_bytes());
        let mut buffer = String::new();
        let deadline = now_ms() + timeout_ms;
        while now_ms() < deadline {
            let chunk = self.transport.read(120);
            if !chunk.is_empty() {
                buffer += &String::from_utf8_lossy(&chunk);
                if buffer.contains('>') {
                    break;
                }
            }
        }
        let echo = strip(command);
        let mut lines = Vec::new();
        for line in buffer.replace('>', "").split(['\r', '\n']) {
            let line = line.trim();
            if line.is_empty()
                || strip(line) == echo
                || line.to_ascii_uppercase().starts_with("SEARCHING")
            {
                continue;
            }
            lines.push(line.to_string());
        }
        let sent = strip(command);
        if sent == "ATH1" {
            self.headers = true;
        }
        if ["ATH0", "ATZ", "ATD", "ATWS"].contains(&sent.as_str()) {
            self.headers = false;
        }
        lines
    }

    pub fn at_ok(&mut self, command: &str) -> bool {
        self.send(command, 2000)
            .iter()
            .any(|l| l.to_ascii_uppercase().contains("OK"))
    }

    pub fn set_headers(&mut self, on: bool) -> bool {
        let ok = self.at_ok(if on { "ATH1" } else { "ATH0" });
        if ok {
            self.headers = on;
        }
        ok
    }

    fn id_width(&self) -> usize {
        if self.bits29 { 8 } else { 3 }
    }

    /// The lines of an answer as messages, ISO-TP put back together.
    pub fn decode(&self, lines: &[String]) -> Vec<Message> {
        let mut out = Vec::new();
        // Multi frame answers in progress: source, expected length, bytes.
        let mut open: Vec<(String, usize, Vec<u8>)> = Vec::new();
        let finish =
            |open: &mut Vec<(String, usize, Vec<u8>)>, index: usize, out: &mut Vec<Message>| {
                if open[index].2.len() >= open[index].1 {
                    let (source, total, mut data) = open.remove(index);
                    data.truncate(total);
                    out.push(Message { source, data });
                }
            };
        for line in lines {
            let compact = strip(line);
            if self.headers {
                let width = self.id_width();
                if compact.len() < width {
                    continue;
                }
                let Some(bytes) = hex_bytes(&compact[width..]) else {
                    continue;
                };
                let source = compact[..width].to_string();
                let Some(&pci) = bytes.first() else {
                    continue;
                };
                match pci >> 4 {
                    0 => {
                        let len = usize::from(pci & 0x0F).min(bytes.len() - 1);
                        out.push(Message {
                            source,
                            data: bytes[1..1 + len].to_vec(),
                        });
                    }
                    1 if bytes.len() >= 2 => {
                        let total = (usize::from(pci & 0x0F) << 8) | usize::from(bytes[1]);
                        open.retain(|(s, _, _)| *s != source);
                        open.push((source, total, bytes[2..].to_vec()));
                    }
                    2 => {
                        if let Some(index) = open.iter().position(|(s, _, _)| *s == source) {
                            open[index].2.extend_from_slice(&bytes[1..]);
                            finish(&mut open, index, &mut out);
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if compact.len() == 3 && compact.bytes().all(|c| c.is_ascii_hexdigit()) {
                let total = usize::from_str_radix(&compact, 16).unwrap();
                open.retain(|(s, _, _)| !s.is_empty());
                open.push((String::new(), total, Vec::new()));
                continue;
            }
            if let Some((_, rest)) = compact.split_once(':') {
                if let (Some(bytes), Some(index)) = (
                    hex_bytes(rest),
                    open.iter().position(|(s, _, _)| s.is_empty()),
                ) {
                    open[index].2.extend_from_slice(&bytes);
                    finish(&mut open, index, &mut out);
                }
                continue;
            }
            if let Some(bytes) = hex_bytes(&compact) {
                out.push(Message {
                    source: String::new(),
                    data: bytes,
                });
            }
        }
        out
    }

    pub fn request(&mut self, command: &str, timeout_ms: u64) -> Vec<Message> {
        let lines = self.send(command, timeout_ms);
        self.decode(&lines)
    }

    /// Elm327::connect: reset, settings, then each protocol of the order with
    /// 0100 (and 22F400 for OBD on UDS). Returns the protocol text.
    pub fn connect(&mut self, order: &str) -> Result<String, String> {
        self.obd_on_uds = false;
        let banner = self.send("ATZ", 6000);
        if banner.is_empty() {
            return Err("no answer to ATZ".into());
        }
        for command in ["ATE0", "ATL0", "ATS0", "ATH0"] {
            self.send(command, 2000);
        }
        self.send("ATAT1", 2000);
        self.send("ATST64", 2000);
        self.send("STI", 1500);
        for protocol in order.chars() {
            self.send(&format!("ATSP{protocol}"), 2000);
            for _attempt in 0..2 {
                let probe = self.request("0100", 8000);
                let mut ok = probe.iter().any(|m| m.data.starts_with(&[0x41, 0x00]));
                if !ok && ('6'..='9').contains(&protocol) {
                    let uds = self.request("22F400", 8000);
                    if uds
                        .first()
                        .is_some_and(|m| m.data.starts_with(&[0x62, 0xF4, 0x00]))
                    {
                        self.obd_on_uds = true;
                        ok = true;
                    }
                }
                if ok {
                    let dp = self.send("ATDP", 2000);
                    self.protocol_text = dp.first().cloned().unwrap_or_default();
                    self.bits29 = self.protocol_text.contains("CAN 29");
                    return Ok(self.protocol_text.clone());
                }
            }
        }
        Err("No contact with the control unit.".into())
    }

    pub fn read_vin(&mut self) -> String {
        let (command, head): (&str, &[u8]) = if self.obd_on_uds {
            ("22F802", &[0x62, 0xF8, 0x02])
        } else {
            ("0902", &[0x49, 0x02, 0x01])
        };
        for m in self.request(command, 5000) {
            if m.data.starts_with(head) {
                return m.data[head.len()..]
                    .iter()
                    .filter(|b| b.is_ascii_alphanumeric())
                    .map(|&b| b as char)
                    .collect();
            }
        }
        String::new()
    }

    /// Service 03 on CAN: count byte, then two bytes per code.
    pub fn read_faults(&mut self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for m in self.request("03", 5000) {
            if m.data.first() != Some(&0x43) || m.data.len() < 2 {
                continue;
            }
            for pair in m.data[2..].as_chunks::<2>().0 {
                if *pair == [0, 0] {
                    continue;
                }
                let letter = ['P', 'C', 'B', 'U'][usize::from(pair[0] >> 6)];
                out.push((
                    m.source.clone(),
                    format!(
                        "{letter}{}{:X}{:02X}",
                        (pair[0] >> 4) & 3,
                        pair[0] & 0x0F,
                        pair[1]
                    ),
                ));
            }
        }
        out
    }

    /// Uds::setup for a unit with extended address (BMW): raw frames.
    pub fn address_extended(&mut self, request: u32, response: u32) -> bool {
        self.at_ok(&format!("ATSH{:03X}", request & 0x7FF))
            && self.at_ok(&format!("ATCRA{:03X}", response & 0x7FF))
            && self.at_ok("ATCAF0")
    }

    pub fn restore_default(&mut self) {
        self.at_ok("ATCAF1");
        self.at_ok("ATFCSM0");
        self.at_ok("ATSH7DF");
        if !self.at_ok("ATCRA") {
            self.at_ok("ATAR");
        }
    }

    fn raw_line_bytes(&self, line: &str) -> Option<Vec<u8>> {
        if line.chars().any(|c| !c.is_ascii_hexdigit() && c != ' ') {
            return None;
        }
        let mut hex = strip(line);
        if self.headers {
            if hex.len() < 3 {
                return None;
            }
            hex = hex[3..].to_string();
        }
        hex_bytes(&hex)
    }

    fn raw_frame(bytes: &[u8]) -> String {
        (0..8)
            .map(|i| format!("{:02X}", bytes.get(i).copied().unwrap_or(0)))
            .collect()
    }

    /// uds::extendedRequest: single frame out, the answer with flow control
    /// from the tester. `None` if the unit did not answer.
    pub fn extended_request(
        &mut self,
        target: u8,
        tester: u8,
        bytes: &[u8],
        timeout_ms: u64,
    ) -> Option<Vec<u8>> {
        let mut frame = vec![target, bytes.len() as u8];
        frame.extend_from_slice(bytes);
        let lines = self.send(&Self::raw_frame(&frame), timeout_ms);
        let mut r = Reassembly::default();
        r.take(lines.iter().filter_map(|l| self.raw_line_bytes(l)), tester);
        if r.first_frame {
            for _round in 0..4 {
                if r.payload.len() >= r.expected {
                    break;
                }
                let more = self.send(&Self::raw_frame(&[target, 0x30, 0x00, 0x00]), timeout_ms);
                let before = r.payload.len();
                r.take(more.iter().filter_map(|l| self.raw_line_bytes(l)), tester);
                if r.payload.len() == before {
                    break;
                }
            }
            if r.payload.len() < r.expected {
                return None;
            }
        }
        r.payload.truncate(r.expected);
        r.answered.then_some(r.payload)
    }

    /// Elm327::monitorLines and captureFrames: raw frames of one id.
    pub fn capture_frames(
        &mut self,
        can_id: u32,
        duration_ms: u64,
        max_frames: usize,
    ) -> Vec<Vec<u8>> {
        self.send("ATCAF0", 2000);
        self.send(&format!("ATCRA{:03X}", can_id & 0x7FF), 2000);
        self.transport.flush_input();
        self.transport.write(b"ATMA\r");
        let mut buffer = String::new();
        let deadline = now_ms() + duration_ms;
        while now_ms() < deadline && buffer.matches('\r').count() <= max_frames {
            buffer += &String::from_utf8_lossy(&self.transport.read(50));
            if buffer.contains('>') {
                break;
            }
        }
        if !buffer.contains('>') {
            // Any character ends the monitor; the prompt follows.
            self.transport.write(b"\r");
            let stop = now_ms() + 1500;
            while now_ms() < stop {
                buffer += &String::from_utf8_lossy(&self.transport.read(50));
                if buffer.contains('>') {
                    break;
                }
            }
        }
        let skip = [
            "ATMA",
            "STOPPED",
            "BUFFER FULL",
            "?",
            "SEARCHING...",
            "NO DATA",
            "CAN ERROR",
            "OK",
        ];
        let mut frames = Vec::new();
        for line in buffer.split(['\r', '\n', '>']) {
            let line = line.trim();
            if line.is_empty() || skip.contains(&line.to_ascii_uppercase().as_str()) {
                continue;
            }
            if frames.len() >= max_frames {
                break;
            }
            let mut hex = strip(line);
            if self.headers {
                if hex.len() < 3 {
                    continue;
                }
                hex = hex[3..].to_string();
            }
            if hex.is_empty() || hex.len() > 16 {
                continue;
            }
            if let Some(bytes) = hex_bytes(&hex) {
                frames.push(bytes);
            }
        }
        self.send("ATCAF1", 2000);
        if !self.at_ok("ATCRA") {
            self.at_ok("ATAR");
        }
        frames
    }
}
