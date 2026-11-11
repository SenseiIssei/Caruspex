//! BMW ENET: diagnosis over Ethernet with HSFZ, the protocol of the ENET cable
//! for BMW and Mini from the F series on (about 2007 to today).
//!
//! The OBD socket of these cars carries Ethernet on pins 3/11 and 12/13; the
//! cable bridges pin 8 to switch the gateway's Ethernet on. The car gives
//! itself an automatic address (169.254.x.x), the computer does the same when
//! no DHCP answers. Port 6811 answers a broadcast with the VIN, port 6801
//! takes the diagnosis.
//!
//! HSFZ frame: 4-byte length (big endian, counts the payload), 0x00, control
//! byte, payload. Diagnosis (control 0x01) carries source address, target
//! address and the UDS bytes; the car first acknowledges every request
//! (0x02, the request echoed), then answers. It also asks "still there?"
//! (0x12), which the tester answers with 0x13.
//! Protocol facts after the open source EdiabasLib (EdInterfaceEnet).
//!
//! So that everything else in Caruspex works unchanged, [`EnetElm`] puts an
//! ELM327 front end on top: OBD requests go to the engine (DME, address 12),
//! BMW style extended addressing (ATSH 6F1, raw frames) goes to the unit in
//! the first byte. To the rest of the program the car looks like an ELM327 on
//! CAN 11 bit 500 kbit.
//!
//! DoIP (ISO 13400-2), the open standard for diagnosis over Ethernet in newer
//! cars, lives here as well (see [`doip`]) and uses the same front end.

use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Duration;

use caruspex_i18n::l;
use caruspex_link::Transport;
use caruspex_link::timing::now_ms;

use crate::text::{hex, is_hex, normalize, parse_hex};

pub mod hsfz {
    //! HSFZ frames and the identification answer.

    pub const DIAGNOSTIC_PORT: u16 = 6801;
    pub const IDENT_PORT: u16 = 6811;
    /// ENET testers use F4, D-CAN uses F1.
    pub const TESTER: u8 = 0xF4;
    /// All units.
    pub const FUNCTIONAL: u8 = 0xDF;
    /// DME.
    pub const ENGINE: u8 = 0x12;
    /// EGS.
    pub const GEARBOX: u8 = 0x18;

    // Control bytes.
    pub const DIAGNOSTIC: u8 = 0x01;
    pub const ACK: u8 = 0x02;
    pub const IDENT: u8 = 0x11;
    pub const ALIVE_CHECK: u8 = 0x12;
    pub const ALIVE_REPLY: u8 = 0x13;
    pub const NACK: u8 = 0xFF;

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Frame {
        pub control: u8,
        pub payload: Vec<u8>,
    }

    pub fn encode(control: u8, payload: &[u8]) -> Vec<u8> {
        let size = payload.len() as u32;
        let mut out = Vec::with_capacity(6 + payload.len());
        out.extend_from_slice(&size.to_be_bytes());
        out.push(0x00);
        out.push(control);
        out.extend_from_slice(payload);
        out
    }

    /// Cuts a byte stream into frames.
    #[derive(Debug, Default)]
    pub struct Parser {
        buffer: Vec<u8>,
        broken: bool,
    }

    impl Parser {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn feed(&mut self, data: &[u8]) {
            self.buffer.extend_from_slice(data);
        }

        /// A length no car would send.
        pub fn broken(&self) -> bool {
            self.broken
        }
    }

    /// The frames cut so far; `None` until a whole frame is there.
    impl Iterator for Parser {
        type Item = Frame;

        fn next(&mut self) -> Option<Frame> {
            if self.broken || self.buffer.len() < 6 {
                return None;
            }
            let b = &self.buffer;
            let length = u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize;
            // A UDS answer is at most 4095 bytes plus the two addresses.
            if length > 0x10000 {
                self.broken = true;
                return None;
            }
            if self.buffer.len() < 6 + length {
                return None;
            }
            let frame = Frame {
                control: self.buffer[5],
                payload: self.buffer[6..6 + length].to_vec(),
            };
            self.buffer.drain(..6 + length);
            Some(frame)
        }
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct VehicleIdent {
        /// IP of the gateway.
        pub address: String,
        pub vin: String,
        pub mac: String,
        /// The gateway's diagnostic address, usually 0x10.
        pub diag_address: Option<u8>,
    }

    /// The answer to the identification broadcast:
    /// header, "DIAGADR10BMWMAC001A37xxxxxxBMWVINWBA...". `None` if it is none.
    pub fn parse_ident(d: &[u8]) -> Option<VehicleIdent> {
        let text = |at: usize, count: usize| -> String {
            d.iter().skip(at).take(count).map(|&b| b as char).collect()
        };
        if d.len() < 44 || text(6, 7) != "DIAGADR" {
            return None;
        }
        let address = &d[13..15];
        let mut out = VehicleIdent {
            diag_address: if address.iter().all(u8::is_ascii_hexdigit) {
                u8::from_str_radix(&text(13, 2), 16).ok()
            } else {
                None
            },
            mac: text(21, 12),
            ..VehicleIdent::default()
        };
        if d.len() >= 56 {
            out.vin = text(39, 17);
            let printable = out.vin.chars().all(|c| c.is_ascii_alphanumeric());
            if !printable {
                out.vin.clear();
            }
        }
        Some(out)
    }
}

pub mod doip {
    //! DoIP (ISO 13400-2), the open standard for diagnosis over Ethernet in
    //! newer cars (many makers from about 2016, with a DoIP cable that
    //! bridges the activation pin 8). Port 13400 for discovery (UDP) and
    //! diagnosis (TCP).
    //!
    //! Header: protocol version, its inverse, payload type (2 bytes), payload
    //! length (4 bytes), all big endian. The tester first asks for routing
    //! (0x0005, answer 0x0006 with code 0x10 = granted), then sends diagnostic
    //! messages (0x8001: source, target, UDS), which the entity acknowledges
    //! (0x8002, or 0x8003 if not) before the unit answers with a 0x8001 of its
    //! own. Alive checks (0x0007) are answered with 0x0008.
    //! Logical addresses are 16 bit; testers use 0x0E00 to 0x0FFF, and 0xE400 is
    //! the functional address of the legislated OBD units.

    pub const PORT: u16 = 13400;
    pub const TESTER: u16 = 0x0E80;
    pub const OBD_FUNCTIONAL: u16 = 0xE400;
    /// ISO 13400-2:2012.
    pub const VERSION: u8 = 0x02;

    // Payload types.
    pub const GENERIC_NACK: u16 = 0x0000;
    pub const VEHICLE_IDENT_REQUEST: u16 = 0x0001;
    pub const VEHICLE_ANNOUNCEMENT: u16 = 0x0004;
    pub const ROUTING_ACTIVATION_REQUEST: u16 = 0x0005;
    pub const ROUTING_ACTIVATION_RESPONSE: u16 = 0x0006;
    pub const ALIVE_CHECK_REQUEST: u16 = 0x0007;
    pub const ALIVE_CHECK_RESPONSE: u16 = 0x0008;
    pub const DIAGNOSTIC_MESSAGE: u16 = 0x8001;
    pub const DIAGNOSTIC_ACK: u16 = 0x8002;
    pub const DIAGNOSTIC_NACK: u16 = 0x8003;

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Message {
        pub payload_type: u16,
        pub payload: Vec<u8>,
    }

    /// A message with the protocol version of ISO 13400-2:2012.
    pub fn encode(payload_type: u16, payload: &[u8]) -> Vec<u8> {
        encode_with_version(payload_type, payload, VERSION)
    }

    pub fn encode_with_version(payload_type: u16, payload: &[u8], version: u8) -> Vec<u8> {
        let size = payload.len() as u32;
        let mut out = Vec::with_capacity(8 + payload.len());
        out.push(version);
        out.push(!version);
        out.extend_from_slice(&payload_type.to_be_bytes());
        out.extend_from_slice(&size.to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    #[derive(Debug, Default)]
    pub struct Parser {
        buffer: Vec<u8>,
        broken: bool,
    }

    impl Parser {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn feed(&mut self, data: &[u8]) {
            self.buffer.extend_from_slice(data);
        }

        pub fn broken(&self) -> bool {
            self.broken
        }
    }

    /// The messages cut so far; `None` until a whole one is there.
    impl Iterator for Parser {
        type Item = Message;

        fn next(&mut self) -> Option<Message> {
            if self.broken || self.buffer.len() < 8 {
                return None;
            }
            let b = &self.buffer;
            if b[0] ^ b[1] != 0xFF {
                self.broken = true; // the inverse version does not match: not DoIP
                return None;
            }
            let length = u32::from_be_bytes([b[4], b[5], b[6], b[7]]) as usize;
            if length > 0x10000 {
                self.broken = true;
                return None;
            }
            if self.buffer.len() < 8 + length {
                return None;
            }
            let message = Message {
                payload_type: u16::from_be_bytes([self.buffer[2], self.buffer[3]]),
                payload: self.buffer[8..8 + length].to_vec(),
            };
            self.buffer.drain(..8 + length);
            Some(message)
        }
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Entity {
        /// IP.
        pub address: String,
        pub vin: String,
        pub logical_address: Option<u16>,
    }

    /// The vehicle announcement (0x0004): VIN, logical address, EID, GID, ...
    pub fn parse_announcement(p: &[u8]) -> Option<Entity> {
        if p.len() < 32 {
            return None; // VIN 17, address 2, EID 6, GID 6, further action 1
        }
        let mut vin: String = p[..17]
            .iter()
            .filter(|b| b.is_ascii_alphanumeric())
            .map(|&b| b as char)
            .collect();
        if vin.len() != 17 {
            vin.clear();
        }
        Some(Entity {
            address: String::new(),
            vin,
            logical_address: Some(u16::from_be_bytes([p[17], p[18]])),
        })
    }
}

/// The bytes under the session: TCP in the car, a fake in the tests.
/// (The C++ version calls it ByteStream; that name is the K-line stream in
/// caruspex-link here.)
pub trait NetStream: Send {
    fn send(&mut self, data: &[u8]) -> bool;
    /// `Some(n)` bytes read, `Some(0)` after the timeout, `None` when the
    /// connection is gone.
    fn receive(&mut self, buffer: &mut [u8], timeout_ms: u64) -> Option<usize>;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagAnswer {
    pub source: u16,
    /// UDS bytes, service byte first.
    pub data: Vec<u8>,
}

/// One UDS request to one unit (or the functional address) and its final
/// answers; "response pending" (7F xx 78) is waited out inside.
pub trait DiagLink: Send {
    fn request(
        &mut self,
        target: u16,
        data: &[u8],
        timeout_ms: u64,
    ) -> Result<Vec<DiagAnswer>, String>;
}

/// After every 7F xx 78.
const PENDING_EXTRA_MS: u64 = 5000;
/// Overall, for very slow units.
const PENDING_LIMIT_MS: u64 = 30000;
/// More answers after the first one.
const FUNCTIONAL_QUIET_MS: u64 = 250;

// ---------------------------------------------------------------------------
// Session: acknowledgement, alive check, response pending
// ---------------------------------------------------------------------------

pub struct HsfzSession {
    stream: Box<dyn NetStream>,
    parser: hsfz::Parser,
    tester: u8,
    closed: bool,
}

impl HsfzSession {
    pub fn new(stream: Box<dyn NetStream>) -> Self {
        Self::with_tester(stream, hsfz::TESTER)
    }

    pub fn with_tester(stream: Box<dyn NetStream>, tester: u8) -> Self {
        Self {
            stream,
            parser: hsfz::Parser::new(),
            tester,
            closed: false,
        }
    }

    pub fn closed(&self) -> bool {
        self.closed
    }

    fn read_frame(&mut self, deadline: u64) -> Option<hsfz::Frame> {
        let mut buffer = [0u8; 4096];
        loop {
            if let Some(frame) = self.parser.next() {
                return Some(frame);
            }
            if self.parser.broken() {
                self.closed = true;
                return None;
            }
            let now = now_ms();
            if now >= deadline {
                return None;
            }
            match self.stream.receive(&mut buffer, deadline - now) {
                None => {
                    self.closed = true;
                    return None;
                }
                Some(got) if got > 0 => self.parser.feed(&buffer[..got]),
                Some(_) => {}
            }
        }
    }

    fn answer_alive_check(&mut self) {
        self.stream
            .send(&hsfz::encode(hsfz::ALIVE_REPLY, &[0x00, self.tester]));
    }
}

impl DiagLink for HsfzSession {
    fn request(
        &mut self,
        target: u16,
        data: &[u8],
        timeout_ms: u64,
    ) -> Result<Vec<DiagAnswer>, String> {
        let mut answers = Vec::new();
        if self.closed {
            return Err(l(
                "Die ENET-Verbindung zum Auto ist getrennt.",
                "The ENET connection to the car is gone.",
            )
            .into());
        }
        let mut payload = vec![self.tester, target as u8];
        payload.extend_from_slice(data);
        let frame = hsfz::encode(hsfz::DIAGNOSTIC, &payload);
        if !self.stream.send(&frame) {
            self.closed = true;
            return Err(l("Senden über ENET ging nicht.", "Sending over ENET failed.").into());
        }

        let functional = target == u16::from(hsfz::FUNCTIONAL);
        let start = now_ms();
        let mut deadline = start + timeout_ms.max(300);
        let mut acknowledged = false;
        let mut resent = 0;
        while let Some(input) = self.read_frame(deadline) {
            match input.control {
                hsfz::ACK => acknowledged = true,
                hsfz::NACK => {
                    // The gateway did not take it; one more try, as EdiabasLib does.
                    let first = resent == 0;
                    resent += 1;
                    if first && self.stream.send(&frame) {
                        continue;
                    }
                    return Err(l(
                        "Das Gateway nimmt die Anfrage nicht an.",
                        "The gateway refuses the request.",
                    )
                    .into());
                }
                hsfz::ALIVE_CHECK => self.answer_alive_check(),
                hsfz::DIAGNOSTIC => {
                    if input.payload.len() < 3 {
                        continue;
                    }
                    let source = u16::from(input.payload[0]);
                    let dest = input.payload[1];
                    if dest != self.tester {
                        continue;
                    }
                    if !functional && source != target {
                        continue;
                    }
                    let answer = DiagAnswer {
                        source,
                        data: input.payload[2..].to_vec(),
                    };
                    let d = &answer.data;
                    if d.len() >= 3 && d[0] == 0x7F && d[2] == 0x78 {
                        // Still working: wait longer, up to an overall limit.
                        deadline = (now_ms() + PENDING_EXTRA_MS).min(start + PENDING_LIMIT_MS);
                        continue;
                    }
                    answers.push(answer);
                    if !functional {
                        return Ok(answers);
                    }
                    deadline = deadline.min(now_ms() + FUNCTIONAL_QUIET_MS);
                }
                control => {
                    // 0x40 to 0x4F are the gateway's error telegrams (unknown
                    // target, wrong format, out of memory): no answer will come.
                    if (0x40..=0x4F).contains(&control) {
                        return Err(format!(
                            "{}0x{:02X}{}",
                            l(
                                "Das Gateway meldet einen Fehler ",
                                "The gateway reports error "
                            ),
                            control,
                            l(
                                " (zum Beispiel ein unbekanntes Steuergerät).",
                                " (for example an unknown control unit)."
                            )
                        ));
                    }
                    // other telegrams are ignored
                }
            }
        }
        if !answers.is_empty() {
            return Ok(answers);
        }
        Err(if self.closed {
            l(
                "Das Auto hat die ENET-Verbindung getrennt.",
                "The car closed the ENET connection.",
            )
        } else if !acknowledged {
            l(
                "Das Gateway bestätigt die Anfrage nicht.",
                "The gateway does not acknowledge the request.",
            )
        } else {
            l(
                "Keine Antwort vom Steuergerät.",
                "No answer from the control unit.",
            )
        }
        .into())
    }
}

// ---------------------------------------------------------------------------
// DoIP session
// ---------------------------------------------------------------------------

pub struct DoipSession {
    stream: Box<dyn NetStream>,
    parser: doip::Parser,
    tester: u16,
    closed: bool,
}

impl DoipSession {
    pub fn new(stream: Box<dyn NetStream>) -> Self {
        Self::with_tester(stream, doip::TESTER)
    }

    pub fn with_tester(stream: Box<dyn NetStream>, tester: u16) -> Self {
        Self {
            stream,
            parser: doip::Parser::new(),
            tester,
            closed: false,
        }
    }

    fn read_message(&mut self, deadline: u64) -> Option<doip::Message> {
        let mut buffer = [0u8; 4096];
        let message = loop {
            if let Some(message) = self.parser.next() {
                break message;
            }
            if self.parser.broken() {
                self.closed = true;
                return None;
            }
            let now = now_ms();
            if now >= deadline {
                return None;
            }
            match self.stream.receive(&mut buffer, deadline - now) {
                None => {
                    self.closed = true;
                    return None;
                }
                Some(got) if got > 0 => self.parser.feed(&buffer[..got]),
                Some(_) => {}
            }
            // The entity may ask whether we are still there at any time.
        };
        if message.payload_type == doip::ALIVE_CHECK_REQUEST {
            self.stream.send(&doip::encode(
                doip::ALIVE_CHECK_RESPONSE,
                &self.tester.to_be_bytes(),
            ));
        }
        Some(message)
    }

    /// Asks the entity for routing. `Err` with the reason (code).
    pub fn activate(&mut self) -> Result<(), String> {
        let [high, low] = self.tester.to_be_bytes();
        let request = [high, low, 0x00, 0x00, 0x00, 0x00, 0x00];
        if !self
            .stream
            .send(&doip::encode(doip::ROUTING_ACTIVATION_REQUEST, &request))
        {
            return Err(l("Senden über DoIP ging nicht.", "Sending over DoIP failed.").into());
        }
        let deadline = now_ms() + 3000;
        while let Some(input) = self.read_message(deadline) {
            if input.payload_type == doip::GENERIC_NACK {
                return Err(l(
                    "Das Fahrzeug lehnt die DoIP-Nachricht ab.",
                    "The vehicle rejects the DoIP message.",
                )
                .into());
            }
            if input.payload_type != doip::ROUTING_ACTIVATION_RESPONSE || input.payload.len() < 5 {
                continue;
            }
            let code = input.payload[4];
            if code == 0x10 {
                return Ok(());
            }
            return Err(format!(
                "{}0x{:02X}{}",
                l(
                    "Das Fahrzeug gibt die Diagnose nicht frei (Code ",
                    "The vehicle does not grant routing (code "
                ),
                code,
                l(
                    "). Manche Hersteller verlangen dafür eine Anmeldung in ihrer Software.",
                    "). Some makers require a login in their own software for it."
                )
            ));
        }
        Err(l(
            "Keine Antwort auf die DoIP-Freigabe.",
            "No answer to the DoIP routing activation.",
        )
        .into())
    }
}

impl DiagLink for DoipSession {
    fn request(
        &mut self,
        target: u16,
        data: &[u8],
        timeout_ms: u64,
    ) -> Result<Vec<DiagAnswer>, String> {
        let mut answers = Vec::new();
        if self.closed {
            return Err(l(
                "Die DoIP-Verbindung zum Auto ist getrennt.",
                "The DoIP connection to the car is gone.",
            )
            .into());
        }
        let mut payload = Vec::with_capacity(4 + data.len());
        payload.extend_from_slice(&self.tester.to_be_bytes());
        payload.extend_from_slice(&target.to_be_bytes());
        payload.extend_from_slice(data);
        if !self
            .stream
            .send(&doip::encode(doip::DIAGNOSTIC_MESSAGE, &payload))
        {
            self.closed = true;
            return Err(l("Senden über DoIP ging nicht.", "Sending over DoIP failed.").into());
        }
        let functional = target >= 0xE000;
        let start = now_ms();
        let mut deadline = start + timeout_ms.max(300);
        while let Some(input) = self.read_message(deadline) {
            if input.payload_type == doip::DIAGNOSTIC_NACK {
                let code = if input.payload.len() > 4 {
                    input.payload[4]
                } else {
                    0
                };
                return Err(format!(
                    "{}0x{:02X}).",
                    l(
                        "Das Fahrzeug nimmt die Anfrage nicht an (DoIP ",
                        "The vehicle refuses the request (DoIP "
                    ),
                    code
                ));
            }
            if input.payload_type != doip::DIAGNOSTIC_MESSAGE || input.payload.len() < 5 {
                continue;
            }
            let p = &input.payload;
            let source = u16::from_be_bytes([p[0], p[1]]);
            let dest = u16::from_be_bytes([p[2], p[3]]);
            if dest != self.tester {
                continue;
            }
            if !functional && source != target {
                continue;
            }
            let answer = DiagAnswer {
                source,
                data: p[4..].to_vec(),
            };
            let d = &answer.data;
            if d.len() >= 3 && d[0] == 0x7F && d[2] == 0x78 {
                deadline = (now_ms() + PENDING_EXTRA_MS).min(start + PENDING_LIMIT_MS);
                continue;
            }
            answers.push(answer);
            if !functional {
                return Ok(answers);
            }
            deadline = deadline.min(now_ms() + FUNCTIONAL_QUIET_MS);
        }
        if !answers.is_empty() {
            return Ok(answers);
        }
        Err(if self.closed {
            l(
                "Das Auto hat die DoIP-Verbindung getrennt.",
                "The car closed the DoIP connection.",
            )
        } else {
            l(
                "Keine Antwort vom Steuergerät.",
                "No answer from the control unit.",
            )
        }
        .into())
    }
}

// ---------------------------------------------------------------------------
// ELM327 front end
// ---------------------------------------------------------------------------

const BANNER: &str = "ELM327 v2.1";
// UDS gives a unit 50 ms for its first answer (P2), a busy one says
// 7F xx 78 and then gets more time in the session. So the waits can be
// short, which matters when a scan asks 128 addresses: "are you there" (3E)
// shortest, everything else a little longer.
const REQUEST_TIMEOUT_MS: u64 = 1500;

fn timeout_for(request: &[u8]) -> u64 {
    if request.first() == Some(&0x3E) {
        600
    } else {
        REQUEST_TIMEOUT_MS
    }
}

/// Bmw: HSFZ with the BMW addresses (DME 12, EGS 18) and the 6F1 raw
/// frames. Doip: OBD to the functional OBD address, answers numbered
/// 7E8, 7E9, ... in the order the units first answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Flavor {
    #[default]
    Bmw,
    Doip,
}

/// An ELM327 as far as Caruspex uses one, on top of a [`DiagLink`].
pub struct EnetElm {
    link: Box<dyn DiagLink>,
    name: String,
    flavor: Flavor,
    /// DoIP: logical address per 7E8 + n.
    learned: Vec<u16>,
    pending: Vec<u8>,
    echo: bool,
    linefeeds: bool,
    spaces: bool,
    headers: bool,
    auto_format: bool,
    header: u32,
    receive_filter: Option<u32>,
    /// Unit whose long answer waits for flow control.
    raw_target: Option<u8>,
    /// Its consecutive frames.
    raw_pending: Vec<Vec<u8>>,
}

impl EnetElm {
    pub fn new(link: Box<dyn DiagLink>, name: impl Into<String>, flavor: Flavor) -> Self {
        Self {
            link,
            name: name.into(),
            flavor,
            learned: Vec::new(),
            pending: Vec::new(),
            echo: true,
            linefeeds: false,
            spaces: true,
            headers: false,
            auto_format: true,
            header: 0x7DF,
            receive_filter: None,
            raw_target: None,
            raw_pending: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.echo = true;
        self.linefeeds = false;
        self.spaces = true;
        self.headers = false;
        self.auto_format = true;
        self.header = 0x7DF;
        self.receive_filter = None;
        self.raw_target = None;
        self.raw_pending.clear();
    }

    fn hex(&self, bytes: &[u8]) -> String {
        hex(bytes, self.spaces)
    }

    fn id_text(&self, id: u32) -> String {
        format!("{:03X}{}", id & 0x7FF, if self.spaces { " " } else { "" })
    }

    fn handle(&mut self, command: &str) -> Vec<String> {
        if command.is_empty() {
            return Vec::new();
        }
        if let Some(rest) = command.strip_prefix("AT") {
            return self.at(rest);
        }
        if command.starts_with("ST") {
            return vec!["?".into()]; // no STN commands here
        }
        let mut digits = command.to_string();
        // A trailing single digit is the ELM327's "expected answers" hint.
        if digits.len() % 2 == 1 {
            digits.pop();
        }
        if !is_hex(&digits) {
            return vec!["?".into()];
        }
        if self.auto_format {
            self.obd(&parse_hex(&digits))
        } else {
            self.raw(&parse_hex(&digits))
        }
    }

    fn at(&mut self, c: &str) -> Vec<String> {
        let ok = || vec!["OK".to_string()];
        if c == "Z" || c == "WS" || c == "D" {
            self.reset();
            return if c == "D" { ok() } else { vec![BANNER.into()] };
        }
        if c == "I" {
            return vec![BANNER.into()];
        }
        if c == "@1" {
            return vec!["Caruspex ENET (BMW HSFZ)".into()];
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
        if c == "DP" {
            return vec!["ISO 15765-4 (CAN 11/500)".into()];
        }
        if c == "DPN" {
            return vec!["6".into()];
        }
        if let Some(id) = c.strip_prefix("SH") {
            if id.len() != 3 || !is_hex(id) {
                return vec!["?".into()]; // only 11-bit ids
            }
            self.header = u32::from_str_radix(id, 16).unwrap_or(0);
            return ok();
        }
        if c == "CRA" || c == "AR" {
            self.receive_filter = None;
            return ok();
        }
        if let Some(id) = c.strip_prefix("CRA") {
            if id.len() != 3 || !is_hex(id) {
                return vec!["?".into()];
            }
            self.receive_filter = u32::from_str_radix(id, 16).ok();
            return ok();
        }
        if c == "RV" {
            // No voltage pin in the cable: the engine unit's own reading (PID 42).
            let engine = if self.flavor == Flavor::Doip {
                doip::OBD_FUNCTIONAL
            } else {
                u16::from(hsfz::ENGINE)
            };
            if let Ok(answers) = self.link.request(engine, &[0x01, 0x42], REQUEST_TIMEOUT_MS)
                && let Some(first) = answers.first()
                && first.data.len() >= 4
                && first.data[0] == 0x41
            {
                let d = &first.data;
                let volts = f64::from((u32::from(d[2]) << 8) | u32::from(d[3])) / 1000.0;
                return vec![format!("{volts:.1}V")];
            }
            return vec!["?".into()];
        }
        // Settings that change nothing over Ethernet.
        const ACCEPTED: [&str; 18] = [
            "SP", "TP", "AT", "ST", "FCSH", "FCSD", "FCSM", "CP", "PC", "M0", "R1", "AL", "NL",
            "V0", "V1", "D0", "D1", "CFC1",
        ];
        if ACCEPTED.iter().any(|a| c.starts_with(a)) {
            return ok();
        }
        vec!["?".into()]
    }

    /// Normal addressing: OBD (7DF, 7E0, 7E1) to the engine and the gearbox,
    /// framed by "the adapter" as an ELM327 with CAN auto formatting would.
    fn obd(&mut self, bytes: &[u8]) -> Vec<String> {
        let no_data = || vec!["NO DATA".to_string()];
        if bytes.is_empty() || bytes.len() > 7 {
            return vec!["?".into()];
        }
        let functional = self.header == 0x7DF;
        let target = if self.flavor == Flavor::Doip {
            // Functional to all OBD units; 7E0 + n to the unit that answered as
            // 7E8 + n before.
            if functional {
                Some(doip::OBD_FUNCTIONAL)
            } else if (0x7E0..=0x7E7).contains(&self.header)
                && ((self.header - 0x7E0) as usize) < self.learned.len()
            {
                Some(self.learned[(self.header - 0x7E0) as usize])
            } else {
                None
            }
        } else if functional || self.header == 0x7E0 {
            Some(u16::from(hsfz::ENGINE))
        } else if self.header == 0x7E1 {
            Some(u16::from(hsfz::GEARBOX))
        } else {
            None
        };
        let Some(target) = target else {
            return no_data();
        };

        let Ok(answers) = self.link.request(target, bytes, timeout_for(bytes)) else {
            return no_data();
        };

        let mut lines = Vec::new();
        for a in &answers {
            // Functional requests get no refusals, as on the bus.
            if functional && a.data.first() == Some(&0x7F) {
                continue;
            }
            let mut id: u32 = if a.source == u16::from(hsfz::GEARBOX) {
                0x7E9
            } else {
                0x7E8
            };
            if self.flavor == Flavor::Doip {
                let index = match self.learned.iter().position(|&s| s == a.source) {
                    Some(index) => index,
                    None => {
                        if self.learned.len() >= 8 {
                            continue;
                        }
                        self.learned.push(a.source);
                        self.learned.len() - 1
                    }
                };
                id = 0x7E8 + index as u32;
            }
            if self.receive_filter.is_some_and(|f| f != id) {
                continue;
            }
            let d = &a.data;
            if d.len() <= 7 {
                if self.headers {
                    let mut frame = vec![d.len() as u8];
                    frame.extend_from_slice(d);
                    frame.resize(8, 0xAA);
                    lines.push(self.id_text(id) + &self.hex(&frame));
                } else {
                    lines.push(self.hex(d));
                }
                continue;
            }
            // Multi frame: first frame with six bytes, then seven each.
            if self.headers {
                let mut first = vec![0x10 | ((d.len() >> 8) & 0x0F) as u8, (d.len() & 0xFF) as u8];
                first.extend_from_slice(&d[..6]);
                lines.push(self.id_text(id) + &self.hex(&first));
                let mut index = 1usize;
                let mut at = 6;
                while at < d.len() {
                    let mut cf = vec![0x20 | (index & 0x0F) as u8];
                    cf.extend_from_slice(&d[at..(at + 7).min(d.len())]);
                    cf.resize(8, 0xAA);
                    lines.push(self.id_text(id) + &self.hex(&cf));
                    at += 7;
                    index += 1;
                }
            } else {
                lines.push(format!("{:03X}", d.len()));
                let mut index = 0usize;
                let mut at = 0;
                while at < d.len() {
                    let take = if at == 0 { 6 } else { 7 };
                    let part = &d[at..(at + take).min(d.len())];
                    lines.push(format!(
                        "{}{}{}",
                        index % 10,
                        if self.spaces { ": " } else { ":" },
                        self.hex(part)
                    ));
                    at += take;
                    index += 1;
                }
            }
        }
        if lines.is_empty() {
            return no_data();
        }
        lines
    }

    /// CAN auto formatting off with ATSH 6F1: BMW style extended addressing,
    /// the ISO-TP frames come from the tester as they would go on the bus.
    fn raw(&mut self, bytes: &[u8]) -> Vec<String> {
        let no_data = || vec!["NO DATA".to_string()];
        if self.flavor != Flavor::Bmw || self.header != 0x6F1 || bytes.len() < 2 {
            return no_data();
        }
        let target = bytes[0];
        let pci = bytes[1];
        let response_id = 0x600 | u32::from(target);
        if self.receive_filter.is_some_and(|f| f != response_id) {
            return no_data();
        }
        let line = |me: &Self, frame: &[u8]| {
            let mut padded = frame.to_vec();
            padded.resize(8, 0x00);
            (if me.headers {
                me.id_text(response_id)
            } else {
                String::new()
            }) + &me.hex(&padded)
        };

        if pci >> 4 == 3 {
            // Flow control from the tester: out with the rest of the long answer.
            if Some(target) != self.raw_target || self.raw_pending.is_empty() {
                return no_data();
            }
            let pending = std::mem::take(&mut self.raw_pending);
            let lines = pending.iter().map(|cf| line(self, cf)).collect();
            self.raw_target = None;
            return lines;
        }
        if pci >> 4 != 0 {
            return no_data(); // only single frame requests
        }
        let length = usize::from(pci & 0x0F);
        if length == 0 || bytes.len() < 2 + length {
            return no_data();
        }
        let request = &bytes[2..2 + length];

        let Ok(answers) = self
            .link
            .request(u16::from(target), request, timeout_for(request))
        else {
            return no_data();
        };
        let Some(answer) = answers.first() else {
            return no_data();
        };
        let d = &answer.data;
        let tester = 0xF1u8; // what a D-CAN tester would see
        if d.len() <= 6 {
            let mut frame = vec![tester, d.len() as u8];
            frame.extend_from_slice(d);
            return vec![line(self, &frame)];
        }
        // First frame now, the consecutive frames after the flow control.
        let mut first = vec![
            tester,
            0x10 | ((d.len() >> 8) & 0x0F) as u8,
            (d.len() & 0xFF) as u8,
        ];
        first.extend_from_slice(&d[..5]);
        self.raw_pending.clear();
        let mut index = 1usize;
        let mut at = 5;
        while at < d.len() {
            let mut cf = vec![tester, 0x20 | (index & 0x0F) as u8];
            cf.extend_from_slice(&d[at..(at + 6).min(d.len())]);
            self.raw_pending.push(cf);
            at += 6;
            index += 1;
        }
        self.raw_target = Some(target);
        vec![line(self, &first)]
    }
}

impl Transport for EnetElm {
    fn write(&mut self, data: &[u8]) -> bool {
        let command = normalize(data);
        let mut out = Vec::new();
        if self.echo {
            let cut = data.iter().position(|&c| c == b'\r').unwrap_or(data.len());
            out.extend_from_slice(&data[..cut]);
            out.push(b'\r');
        }
        let end: &[u8] = if self.linefeeds { b"\r\n" } else { b"\r" };
        for line in self.handle(&command) {
            out.extend_from_slice(line.as_bytes());
            out.extend_from_slice(end);
        }
        out.extend_from_slice(end);
        out.push(b'>');
        self.pending.extend_from_slice(&out);
        true
    }

    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        // The answer is complete when write returns.
        std::mem::take(&mut self.pending)
    }

    fn flush_input(&mut self) {
        self.pending.clear();
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

struct TcpStream(std::net::TcpStream);

impl NetStream for TcpStream {
    fn send(&mut self, data: &[u8]) -> bool {
        self.0.write_all(data).is_ok()
    }

    fn receive(&mut self, buffer: &mut [u8], timeout_ms: u64) -> Option<usize> {
        // Waits until the socket can be read, as select() would.
        let result = if timeout_ms == 0 {
            if self.0.set_nonblocking(true).is_err() {
                return None;
            }
            let r = self.0.read(buffer);
            let _ = self.0.set_nonblocking(false);
            r
        } else {
            if self
                .0
                .set_read_timeout(Some(Duration::from_millis(timeout_ms)))
                .is_err()
            {
                return None;
            }
            self.0.read(buffer)
        };
        match result {
            Ok(0) => None,
            Ok(got) => Some(got),
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) =>
            {
                Some(0)
            }
            Err(_) => None,
        }
    }
}

fn connect_tcp(host: &str, port: u16, timeout_ms: u64) -> Option<std::net::TcpStream> {
    let ip: Ipv4Addr = host.parse().ok()?;
    let address = SocketAddr::V4(SocketAddrV4::new(ip, port));
    let stream =
        std::net::TcpStream::connect_timeout(&address, Duration::from_millis(timeout_ms)).ok()?;
    let _ = stream.set_nodelay(true);
    Some(stream)
}

/// Sends `request` to both broadcast addresses up to three times and hands
/// every datagram to `take` until something was found.
fn broadcast_search(
    socket: &UdpSocket,
    port: u16,
    request: &[u8],
    timeout_ms: u64,
    mut take: impl FnMut(&[u8], String) -> bool,
) {
    let _ = socket.set_broadcast(true);
    let deadline = now_ms() + timeout_ms;
    let mut found = false;
    for _attempt in 0..3 {
        if found {
            break;
        }
        // The plain broadcast leaves on the default interface only; the
        // link-local one reaches the ENET cable on a second network port.
        for target in [Ipv4Addr::BROADCAST, Ipv4Addr::new(169, 254, 255, 255)] {
            let _ = socket.send_to(request, SocketAddrV4::new(target, port));
        }
        let round = deadline.min(now_ms() + timeout_ms / 3 + 1);
        while now_ms() < round {
            let wait = round.saturating_sub(now_ms()).max(1);
            if socket
                .set_read_timeout(Some(Duration::from_millis(wait)))
                .is_err()
            {
                break;
            }
            let mut buffer = [0u8; 512];
            match socket.recv_from(&mut buffer) {
                Ok((0, _)) => continue,
                Ok((got, from)) => {
                    if take(&buffer[..got], from.ip().to_string()) {
                        found = true;
                    }
                }
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => break,
                Err(_) => continue,
            }
        }
    }
}

/// Looks for cars on the local network (identification broadcast on 6811,
/// to 255.255.255.255 and the link-local 169.254.255.255).
pub fn discover_enet(timeout_ms: u64) -> Result<Vec<hsfz::VehicleIdent>, String> {
    let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) else {
        return Err(l(
            "Kein UDP-Socket für die Fahrzeugsuche.",
            "No UDP socket for the vehicle search.",
        )
        .into());
    };
    let mut found: Vec<hsfz::VehicleIdent> = Vec::new();
    let request = hsfz::encode(hsfz::IDENT, &[]);
    broadcast_search(
        &socket,
        hsfz::IDENT_PORT,
        &request,
        timeout_ms,
        |datagram, from| {
            let Some(mut ident) = hsfz::parse_ident(datagram) else {
                return false;
            };
            ident.address = from;
            if !found.iter().any(|v| v.address == ident.address) {
                found.push(ident);
            }
            true
        },
    );
    Ok(found)
}

/// DoIP entities on the local network (vehicle identification on 13400).
pub fn discover_doip(timeout_ms: u64) -> Result<Vec<doip::Entity>, String> {
    let mut found: Vec<doip::Entity> = Vec::new();
    let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) else {
        return Ok(found);
    };
    // Version 0xFF is the "any version" the standard asks for here.
    let request = doip::encode_with_version(doip::VEHICLE_IDENT_REQUEST, &[], 0xFF);
    broadcast_search(
        &socket,
        doip::PORT,
        &request,
        timeout_ms,
        |datagram, from| {
            let mut parser = doip::Parser::new();
            parser.feed(datagram);
            let Some(message) = parser.next() else {
                return false;
            };
            if message.payload_type != doip::VEHICLE_ANNOUNCEMENT {
                return false;
            }
            let Some(mut entity) = doip::parse_announcement(&message.payload) else {
                return false;
            };
            entity.address = from;
            if !found.iter().any(|e| e.address == entity.address) {
                found.push(entity);
            }
            true
        },
    );
    Ok(found)
}

/// Connects to the car: host empty or "auto" looks for it first. Returns the
/// ELM front end and what the car said about itself.
pub fn open_enet(host: &str) -> Result<(EnetElm, hsfz::VehicleIdent), String> {
    let mut ident = hsfz::VehicleIdent::default();
    let address;
    if host.is_empty() || host == "auto" {
        let cars = discover_enet(3000)?;
        let Some(first) = cars.into_iter().next() else {
            return Err(l(
                "Kein BMW über ENET gefunden. Steckt das ENET-Kabel im Auto und am Netzwerkanschluss, \
                 und ist die Zündung an? Nach dem Einstecken braucht der Rechner bis zu einer Minute \
                 für seine automatische Adresse (169.254.x.x), dann noch einmal versuchen.",
                "No BMW found over ENET. Is the ENET cable plugged into the car and the network port, \
                 and is the ignition on? After plugging in, the computer needs up to a minute for its \
                 automatic address (169.254.x.x); then try again.",
            )
            .into());
        };
        ident = first;
        address = ident.address.clone();
    } else {
        address = host.to_string();
        ident.address = address.clone();
    }

    let Some(socket) = connect_tcp(&address, hsfz::DIAGNOSTIC_PORT, 5000) else {
        return Err(format!(
            "{}{}:{}{}",
            l("Keine Verbindung zu ", "No connection to "),
            address,
            hsfz::DIAGNOSTIC_PORT,
            l(
                ". Ist das die Adresse des Autos, und ist die Zündung an?",
                ". Is this the car's address, and is the ignition on?"
            )
        ));
    };
    let mut name = format!("ENET {address}");
    if !ident.vin.is_empty() {
        name += &format!(" ({})", ident.vin);
    }
    let session = HsfzSession::new(Box::new(TcpStream(socket)));
    Ok((EnetElm::new(Box::new(session), name, Flavor::Bmw), ident))
}

pub fn open_doip(host: &str) -> Result<(EnetElm, doip::Entity), String> {
    let mut entity = doip::Entity::default();
    let address;
    if host.is_empty() || host == "auto" {
        let cars = discover_doip(2500)?;
        let Some(first) = cars.into_iter().next() else {
            return Err(l(
                "Kein Fahrzeug mit DoIP im Netzwerk gefunden.",
                "No vehicle with DoIP found on the network.",
            )
            .into());
        };
        entity = first;
        address = entity.address.clone();
    } else {
        address = host.to_string();
        entity.address = address.clone();
    }
    let Some(socket) = connect_tcp(&address, doip::PORT, 5000) else {
        return Err(format!(
            "{}{}:{}",
            l("Keine DoIP-Verbindung zu ", "No DoIP connection to "),
            address,
            doip::PORT
        ));
    };
    let mut session = DoipSession::new(Box::new(TcpStream(socket)));
    session.activate()?;
    let mut name = format!("DoIP {address}");
    if !entity.vin.is_empty() {
        name += &format!(" ({})", entity.vin);
    }
    Ok((EnetElm::new(Box::new(session), name, Flavor::Doip), entity))
}

/// What [`open_network_car`] found.
pub struct NetworkCar {
    pub transport: EnetElm,
    /// "ENET" or "DoIP".
    pub kind: &'static str,
    pub vin: String,
}

/// Whatever the network cable leads to: BMW ENET (HSFZ) or DoIP, found
/// automatically when host is empty or "auto", otherwise tried in that order.
pub fn open_network_car(host: &str) -> Result<NetworkCar, String> {
    let enet_error = match open_enet(host) {
        Ok((transport, ident)) => {
            return Ok(NetworkCar {
                transport,
                kind: "ENET",
                vin: ident.vin,
            });
        }
        Err(e) => e,
    };
    let doip_error = match open_doip(host) {
        Ok((transport, entity)) => {
            return Ok(NetworkCar {
                transport,
                kind: "DoIP",
                vin: entity.vin,
            });
        }
        Err(e) => e,
    };
    Err(format!("{enet_error}\n{doip_error}"))
}
