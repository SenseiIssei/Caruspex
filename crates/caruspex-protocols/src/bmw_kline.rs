//! BMW's E series on the K-line: DS2 and KWP2000 in BMW's own style.
//!
//! BMW cars from about 1995 to 2007 (E36 late, E38, E39, E46, E52, E53,
//! E83, E85, then E60, E61, E63, E64, E65, E87 and E90 up to 03/2007) are
//! diagnosed on the K-line at pin 7 of the OBD socket. Many older models
//! put a second line on pin 8, so the K+DCAN cables bridge pins 7 and 8 in
//! their K-line position. Three ways of talking share that wire:
//!
//! ```text
//!   DS2            9600 baud 8E1, no wake-up
//!                  [address] [length of the whole frame] [data ...] [XOR]
//!                  answer: [address] [length] [status] [data ...] [XOR]
//!                  status A0 = done, A1 busy, A2 rejected, B0 parameter
//!                  wrong, B1 function unknown, FF not acknowledged
//!
//!   KWP2000 (BMW)  10400 baud 8N1, ISO 14230 fast init (25 ms low, 25 ms
//!                  high, StartCommunication 81), then
//!                  [80+length] [target] [F1] [data ...] [sum]
//!
//!   BMW-FAST       the same frames at 115200 baud 8N1 without any init,
//!                  through the central gateway of the E60 generation
//! ```
//!
//! The cable hears everything it sends, so every request comes back as an
//! echo before the answer. No echo at all means the cable is not on the
//! K-line: switched to D-CAN, or not plugged in.
//!
//! Only reading and clearing the fault memory are possible here: every
//! request goes through [`request_allowed`], which knows identification,
//! reading and clearing the fault memory and the session frames, nothing
//! else. No coding, no adaptation, no actuators, no memory writes.
//!
//! Sources, read as protocol knowledge only (no code or data copied): the
//! frame layouts, line settings and status bytes as described in public
//! write-ups of DS2 and in EdiabasLib by Ulrich Holeschak (GPL, file
//! EdInterfaceObd: concept list with parity per protocol, fast init
//! timing, echo handling); ISO 14230-2 and -3 for the KWP2000 timing,
//! frames and DTC status byte. The module addresses are the ones of the
//! I-bus and K-bus device lists that circulate in the BMW community; the
//! E60 generation brought the address map the F series still uses
//! (data/ecus/bmw.csv). None of this has met a real car yet.

use caruspex_i18n::l;
use caruspex_link::timing::sleep_ms;
use caruspex_link::{ByteStream, LineParity};

use crate::kwp1281::KLineLog;
use crate::kwp2000::{kwp_frame, negative_response_text, read_kwp_message, read_kwp_rest};

/// The tester's address in KWP2000 frames.
pub const TESTER: u8 = 0xF1;

/// DS2 commands and status bytes.
pub mod ds2 {
    pub const IDENT: u8 = 0x00;
    pub const READ_FAULTS: u8 = 0x04;
    pub const CLEAR_FAULTS: u8 = 0x05;

    pub const OK: u8 = 0xA0;
    pub const BUSY: u8 = 0xA1;
    pub const REJECTED: u8 = 0xA2;
    pub const PARAMETER: u8 = 0xB0;
    pub const FUNCTION: u8 = 0xB1;
    pub const NOT_ACKNOWLEDGED: u8 = 0xFF;
}

/// KWP2000 services in the BMW variant (ISO 14230-3 numbers).
pub mod kwp {
    pub const START_COMMUNICATION: u8 = 0x81;
    pub const STOP_COMMUNICATION: u8 = 0x82;
    pub const CLEAR_DIAGNOSTIC_INFORMATION: u8 = 0x14;
    pub const READ_DTC_BY_STATUS: u8 = 0x18;
    pub const READ_ECU_IDENTIFICATION: u8 = 0x1A;
    pub const TESTER_PRESENT: u8 = 0x3E;
    pub const NEGATIVE: u8 = 0x7F;
    /// 7F xx 78: the answer takes longer.
    pub const RESPONSE_PENDING: u8 = 0x78;
}

/// How long the first answer byte may take, normally and in the scan,
/// where most addresses stay silent. DS2 and KWP2000 control units answer
/// within about 50 ms (P2); the rest is margin for the USB latency of the
/// serial chip, which is 16 ms per read with FTDI defaults.
const ANSWER_MS: u64 = 1000;
const SCAN_ANSWER_MS: u64 = 300;
/// A busy unit (DS2 A1, KWP2000 7F xx 78) gets this much per round.
const PENDING_MS: u64 = 5000;
const INTER_BYTE_MS: u64 = 100;
const ECHO_MS: u64 = 150;
/// Pause between two requests (DS2 regeneration time, KWP2000 P3min).
const REQUEST_GAP_MS: u64 = 60;

/// The three ways of talking on BMW's K-line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BmwProtocol {
    Ds2,
    Kwp2000,
    BmwFast,
}

impl BmwProtocol {
    pub const ALL: [BmwProtocol; 3] = [Self::Ds2, Self::Kwp2000, Self::BmwFast];

    /// Baud rate and parity on the line.
    pub fn line(self) -> (u32, LineParity) {
        match self {
            Self::Ds2 => (9600, LineParity::Even),
            Self::Kwp2000 => (10400, LineParity::None),
            Self::BmwFast => (115200, LineParity::None),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ds2 => "DS2",
            Self::Kwp2000 => "KWP2000 (BMW)",
            Self::BmwFast => "BMW-FAST",
        }
    }

    /// Stable name for the interface and the jobs.
    pub fn key(self) -> &'static str {
        match self {
            Self::Ds2 => "ds2",
            Self::Kwp2000 => "kwp2000",
            Self::BmwFast => "bmw-fast",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key() == key)
    }
}

/// What a module does, for its name. The English texts are the ones of
/// the other control unit lists, so their translations apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Engine,
    Gearbox,
    Brakes,
    Airbag,
    Instruments,
    Immobilizer,
    Body,
    Lighting,
    Climate,
    Parking,
    Seats,
    Infotainment,
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Engine => l("Motor", "Engine"),
        Kind::Gearbox => l("Getriebe", "Gearbox"),
        Kind::Brakes => l("Bremsen und ABS", "Brakes and ABS"),
        Kind::Airbag => l("Airbag", "Airbag"),
        Kind::Instruments => l("Kombiinstrument", "Instrument cluster"),
        Kind::Immobilizer => l("Wegfahrsperre", "Immobilizer"),
        Kind::Body => l("Zentralelektronik", "Body electronics"),
        Kind::Lighting => l("Licht", "Lighting"),
        Kind::Climate => l("Klimaanlage", "Climate control"),
        Kind::Parking => l("Einparkhilfe", "Parking aid"),
        Kind::Seats => l("Sitze", "Seats"),
        Kind::Infotainment => l("Radio und Navigation", "Radio and navigation"),
    }
}

/// A known module address with BMW's short name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BmwModule {
    pub address: u8,
    /// BMW's abbreviation, "DME/DDE", "EGS".
    pub short: &'static str,
    kind: Kind,
}

impl BmwModule {
    const fn new(address: u8, short: &'static str, kind: Kind) -> Self {
        Self {
            address,
            short,
            kind,
        }
    }

    /// "Motor (DME/DDE)" in the active language.
    pub fn name(&self) -> String {
        format!("{} ({})", kind_name(self.kind), self.short)
    }
}

/// The DS2 modules of the older E series (E36 late, E38, E39, E46, E53,
/// E83, E85), engine first. The numbers are the device ids of the I-bus
/// and K-bus, which DS2 shares.
pub const DS2_MODULES: &[BmwModule] = &[
    BmwModule::new(0x12, "DME/DDE", Kind::Engine),
    BmwModule::new(0x32, "EGS", Kind::Gearbox),
    BmwModule::new(0x56, "ABS/ASC/DSC", Kind::Brakes),
    BmwModule::new(0xA4, "MRS", Kind::Airbag),
    BmwModule::new(0x80, "IKE/KOMBI", Kind::Instruments),
    BmwModule::new(0x44, "EWS", Kind::Immobilizer),
    BmwModule::new(0x00, "GM/ZKE", Kind::Body),
    BmwModule::new(0xD0, "LCM/LSZ", Kind::Lighting),
    BmwModule::new(0x5B, "IHKA", Kind::Climate),
    BmwModule::new(0x60, "PDC", Kind::Parking),
    BmwModule::new(0x72, "SM", Kind::Seats),
];

/// The KWP2000 modules of the E60 generation (E60 to E65, E87, E90 up to
/// 03/2007); the F series kept most of these addresses.
pub const KWP_MODULES: &[BmwModule] = &[
    BmwModule::new(0x12, "DME/DDE", Kind::Engine),
    BmwModule::new(0x18, "EGS", Kind::Gearbox),
    BmwModule::new(0x29, "DSC", Kind::Brakes),
    BmwModule::new(0x01, "ACSM/MRS", Kind::Airbag),
    BmwModule::new(0x60, "KOMBI", Kind::Instruments),
    BmwModule::new(0x40, "CAS", Kind::Immobilizer),
    BmwModule::new(0x00, "JBE", Kind::Body),
    BmwModule::new(0x72, "FRM", Kind::Lighting),
    BmwModule::new(0x78, "IHKA", Kind::Climate),
    BmwModule::new(0x64, "PDC", Kind::Parking),
    BmwModule::new(0x63, "CCC/M-ASK", Kind::Infotainment),
];

/// The name of the unit at `address` as reached with `protocol`. The same
/// number means different units in the two address maps (0x60 is the park
/// distance control on DS2 and the instrument cluster on KWP2000).
pub fn module_name(address: u8, protocol: BmwProtocol) -> String {
    let lists: [&[BmwModule]; 2] = if protocol == BmwProtocol::Ds2 {
        [DS2_MODULES, KWP_MODULES]
    } else {
        [KWP_MODULES, DS2_MODULES]
    };
    lists
        .iter()
        .flat_map(|list| list.iter())
        .find(|m| m.address == address)
        .map(BmwModule::name)
        .unwrap_or_else(|| format!("{}0x{address:02X}", l("Steuergerät ", "Control unit ")))
}

/// The order of the scan: every DS2 address at 9600 8E1, then KWP2000 with
/// fast init at 10400 on the KWP2000 addresses and the DS2 ones (the late
/// engine units of the E46 and E39 already speak it), then BMW-FAST on the
/// KWP2000 addresses. Grouped by line setting, so the line is switched
/// twice and not per address.
pub fn scan_plan() -> Vec<(BmwProtocol, u8)> {
    let mut plan: Vec<(BmwProtocol, u8)> = DS2_MODULES
        .iter()
        .map(|m| (BmwProtocol::Ds2, m.address))
        .collect();
    let mut kwp: Vec<u8> = KWP_MODULES.iter().map(|m| m.address).collect();
    for m in DS2_MODULES {
        if !kwp.contains(&m.address) {
            kwp.push(m.address);
        }
    }
    plan.extend(kwp.into_iter().map(|a| (BmwProtocol::Kwp2000, a)));
    plan.extend(
        KWP_MODULES
            .iter()
            .map(|m| (BmwProtocol::BmwFast, m.address)),
    );
    plan
}

/// Every request that may leave the tester: identification, reading and
/// clearing the fault memory, and the KWP2000 session frames. Coding,
/// adaptation, actuator tests and memory access are not on the list, so
/// they cannot be sent by mistake.
pub fn request_allowed(protocol: BmwProtocol, data: &[u8]) -> bool {
    match protocol {
        BmwProtocol::Ds2 => matches!(data, [0x00] | [0x04] | [0x04, 0x01] | [0x05]),
        BmwProtocol::Kwp2000 | BmwProtocol::BmwFast => matches!(
            data,
            [0x81]
                | [0x82]
                | [0x3E]
                | [0x3E, 0x01]
                | [0x1A, 0x80]
                | [0x18, 0x02, 0xFF, 0xFF]
                | [0x14, 0xFF, 0xFF]
        ),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

// --- DS2 framing --------------------------------------------------------------

/// DS2 checksum: XOR over every byte before it.
pub fn ds2_checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |x, &b| x ^ b)
}

/// A DS2 request: address, length of the whole frame (address, length,
/// data and checksum), data, XOR checksum. `None` when the data does not
/// fit the length byte.
pub fn ds2_frame(address: u8, data: &[u8]) -> Option<Vec<u8>> {
    let length = u8::try_from(data.len() + 3).ok()?;
    let mut frame = Vec::with_capacity(data.len() + 3);
    frame.push(address);
    frame.push(length);
    frame.extend_from_slice(data);
    frame.push(ds2_checksum(&frame));
    Some(frame)
}

/// A DS2 answer taken apart: who sent it, the status byte and the data
/// after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ds2Reply {
    pub address: u8,
    pub status: u8,
    pub data: Vec<u8>,
}

/// Checks length and checksum of a whole DS2 answer.
pub fn parse_ds2_frame(frame: &[u8]) -> Result<Ds2Reply, String> {
    if frame.len() < 4 || usize::from(frame[1]) != frame.len() {
        return Err(format!(
            "{}{}",
            l(
                "DS2-Antwort mit falscher Länge: ",
                "DS2 answer with a wrong length: "
            ),
            hex(frame)
        ));
    }
    let (body, sum) = frame.split_at(frame.len() - 1);
    let expected = ds2_checksum(body);
    if sum[0] != expected {
        return Err(format!(
            "{}{:02X}{}{:02X}",
            l(
                "Prüfsumme stimmt nicht: erwartet ",
                "Checksum mismatch: expected "
            ),
            expected,
            l(", bekommen ", ", got "),
            sum[0]
        ));
    }
    Ok(Ds2Reply {
        address: frame[0],
        status: frame[2],
        data: frame[3..frame.len() - 1].to_vec(),
    })
}

/// Plain text for a DS2 status byte other than A0.
pub fn ds2_status_text(status: u8) -> String {
    match status {
        ds2::BUSY => l(
            "Steuergerät beschäftigt, später erneut versuchen",
            "Control unit busy, try again later",
        )
        .to_string(),
        ds2::REJECTED | ds2::NOT_ACKNOWLEDGED => {
            l("Dienst abgelehnt", "Service rejected").to_string()
        }
        ds2::PARAMETER => l(
            "Angefragter Wert liegt außerhalb des Bereichs",
            "Requested value is out of range",
        )
        .to_string(),
        ds2::FUNCTION => l("Dienst wird nicht unterstützt", "Service not supported").to_string(),
        _ => format!(
            "{}{status:02X}",
            l("Ablehnungsgrund 0x", "Rejection reason 0x")
        ),
    }
}

// --- What comes back --------------------------------------------------------------

/// One entry of a BMW fault memory. BMW publishes no text table for its
/// codes, so the code stays a hex number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BmwFault {
    /// "0x2A" for DS2 (one byte, the fault location), "0x2A82" for
    /// KWP2000.
    pub code: String,
    /// Present right now (KWP2000 status), `None` when the unit does not
    /// say it in a known way (DS2).
    pub present: Option<bool>,
    /// The other bytes of the entry as hex (status, environment).
    pub detail: String,
    pub text: String,
}

/// The honest text for a BMW code.
pub fn bmw_fault_text() -> String {
    l(
        "BMW-spezifisch, Text nur mit Herstellerdaten",
        "BMW specific, text only with manufacturer data",
    )
    .to_string()
}

/// The fault memory from a DS2 answer to 04. Its layout belongs to each
/// unit and is only described in BMW's own data; most units put the count
/// first and then entries of equal size that begin with the fault
/// location. That common case is read, anything else is kept as raw bytes.
pub fn ds2_faults(data: &[u8]) -> Vec<BmwFault> {
    let Some((&count, rest)) = data.split_first() else {
        return Vec::new();
    };
    if count == 0 {
        return Vec::new();
    }
    let count = usize::from(count);
    if rest.len() >= count && rest.len().is_multiple_of(count) && rest.len() / count <= 16 {
        return rest
            .chunks(rest.len() / count)
            .map(|entry| BmwFault {
                code: format!("0x{:02X}", entry[0]),
                present: None,
                detail: hex(&entry[1..]),
                text: bmw_fault_text(),
            })
            .collect();
    }
    vec![BmwFault {
        code: "?".to_string(),
        present: None,
        detail: hex(data),
        text: format!(
            "{} {}",
            l(
                "Aufbau dieses Fehlerspeichers unbekannt, die Rohdaten stehen daneben.",
                "Layout of this fault memory unknown, the raw data is next to it."
            ),
            bmw_fault_text()
        ),
    }]
}

/// The fault memory from a KWP2000 answer to 18 02 FF FF: 58, count, then
/// two bytes of code and one status byte per entry. Bits 6 and 5 of the
/// status are the storage state of ISO 14230-3; both set means present at
/// the time of the request.
pub fn kwp_faults(reply: &[u8]) -> Vec<BmwFault> {
    let Some(&count) = reply.get(1) else {
        return Vec::new();
    };
    reply
        .get(2..)
        .unwrap_or_default()
        .as_chunks::<3>()
        .0
        .iter()
        .take(usize::from(count))
        .filter(|e| !(e[0] == 0 && e[1] == 0))
        .map(|e| BmwFault {
            code: format!("0x{:02X}{:02X}", e[0], e[1]),
            present: Some(e[2] & 0x60 == 0x60),
            detail: format!("{:02X}", e[2]),
            text: bmw_fault_text(),
        })
        .collect()
}

/// The readable part of an identification answer: every run of at least
/// three printable characters (part number, supplier, dates), or the bytes
/// as hex when there is none.
pub fn identification_text(data: &[u8]) -> String {
    let mut runs: Vec<String> = Vec::new();
    let mut current = String::new();
    for &b in data {
        if (0x20..0x7F).contains(&b) {
            current.push(char::from(b));
            continue;
        }
        if current.trim().len() >= 3 {
            runs.push(current.trim().to_string());
        }
        current.clear();
    }
    if current.trim().len() >= 3 {
        runs.push(current.trim().to_string());
    }
    if runs.is_empty() {
        return hex(data);
    }
    runs.join(" ")
}

// --- The line -------------------------------------------------------------------

/// Why a request did not work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BmwError {
    /// Nothing came back: the unit is not fitted or not on this protocol.
    NoAnswer,
    /// The cable did not hear itself: not on the K-line.
    NoEcho,
    /// The unit answered, but not with what was asked; or the line failed.
    Failed(String),
}

impl BmwError {
    pub fn message(&self) -> String {
        match self {
            Self::NoAnswer => l(
                "Steuergerät antwortet nicht",
                "Control unit does not respond",
            )
            .to_string(),
            Self::NoEcho => no_echo_text(),
            Self::Failed(text) => text.clone(),
        }
    }
}

/// The cable stays silent about its own bytes.
pub fn no_echo_text() -> String {
    l(
        "Das Kabel hört sich selbst nicht. Steht der Schalter des K+DCAN-Kabels auf K-Line, und steckt es fest in der Buchse? Ein KKL-Kabel braucht Strom von der Buchse, also Zündung an.",
        "The cable does not hear itself. Is the K+DCAN cable switched to K-line and firmly in the socket? A KKL cable needs power from the socket, so ignition on.",
    )
    .to_string()
}

/// One unit the scan found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BmwUnit {
    pub address: u8,
    pub protocol: BmwProtocol,
    pub name: String,
    pub identification: String,
    pub faults: Vec<BmwFault>,
    /// The fault memory could be read (it may still be empty).
    pub faults_read: bool,
    /// Why something is missing.
    pub note: String,
}

/// What the scan reports along the way. Returning false from the callback
/// stops the scan after the current unit.
pub enum ScanEvent<'a> {
    Trying {
        index: usize,
        total: usize,
        protocol: BmwProtocol,
        address: u8,
        name: &'a str,
    },
    Found(&'a BmwUnit),
}

/// BMW's K-line through a serial cable or a simulation. Owns the stream;
/// get it back with [`BmwKLine::into_stream`].
pub struct BmwKLine<S: ByteStream> {
    stream: S,
    log: Option<KLineLog>,
    /// The line setting in use, `None` before the first request.
    line: Option<BmwProtocol>,
    /// The address with an open KWP2000 session after fast init.
    session: Option<u8>,
    answer_ms: u64,
}

impl<S: ByteStream> BmwKLine<S> {
    pub fn new(stream: S, log: Option<KLineLog>) -> Self {
        Self {
            stream,
            log,
            line: None,
            session: None,
            answer_ms: ANSWER_MS,
        }
    }

    pub fn stream(&self) -> &S {
        &self.stream
    }

    pub fn stream_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    /// Ends an open session and hands the stream back.
    pub fn into_stream(mut self) -> S {
        self.stop_session();
        self.stream
    }

    fn log(&self, line: &str) {
        if let Some(log) = &self.log {
            log(line);
        }
    }

    /// Switches the line to the settings of `protocol`; an open KWP2000
    /// session ends first.
    fn select(&mut self, protocol: BmwProtocol) -> Result<(), BmwError> {
        if self.line == Some(protocol) {
            return Ok(());
        }
        self.stop_session();
        let (baud, parity) = protocol.line();
        if !self.stream.set_line(baud, parity) {
            return Err(BmwError::Failed(format!(
                "{}{baud}{}",
                l(
                    "Die Leitung lässt sich nicht auf ",
                    "The line cannot be switched to "
                ),
                l(" Baud umstellen", " baud")
            )));
        }
        self.log(&format!("   {} ({baud} Bd)", protocol.label()));
        self.line = Some(protocol);
        Ok(())
    }

    /// Sends a whole frame and takes its echo off the line.
    fn send(&mut self, frame: &[u8]) -> Result<(), BmwError> {
        self.log(&format!("   -> {}", hex(frame)));
        for &byte in frame {
            if !self.stream.write_byte(byte) {
                return Err(BmwError::Failed(
                    l("Senden fehlgeschlagen", "Send failed").to_string(),
                ));
            }
        }
        if !self.stream.echoes_own_bytes() {
            return Ok(());
        }
        for &byte in frame {
            match self.stream.read_byte(ECHO_MS) {
                None => return Err(BmwError::NoEcho),
                Some(echo) if echo != byte => {
                    return Err(BmwError::Failed(format!(
                        "{}{byte:02X}{}{echo:02X}",
                        l("Echo passt nicht: erwartet ", "Echo mismatch: expected "),
                        l(", bekommen ", ", got ")
                    )));
                }
                Some(_) => {}
            }
        }
        Ok(())
    }

    // --- DS2 ------------------------------------------------------------------

    fn ds2_receive(&mut self, wait_ms: u64) -> Result<Ds2Reply, BmwError> {
        let Some(address) = self.stream.read_byte(wait_ms) else {
            return Err(BmwError::NoAnswer);
        };
        let Some(length) = self.stream.read_byte(INTER_BYTE_MS) else {
            return Err(BmwError::Failed(
                l("Längenbyte fehlt", "Length byte missing").to_string(),
            ));
        };
        let mut frame = vec![address, length];
        while frame.len() < usize::from(length) {
            match self.stream.read_byte(INTER_BYTE_MS) {
                Some(b) => frame.push(b),
                None => {
                    return Err(BmwError::Failed(format!(
                        "{}{}{}{}{}",
                        l("Antwort bricht nach ", "Response breaks off after "),
                        frame.len(),
                        l(" von ", " of "),
                        length,
                        l(" Bytes ab", " bytes")
                    )));
                }
            }
        }
        self.log(&format!("   <- {}", hex(&frame)));
        parse_ds2_frame(&frame).map_err(BmwError::Failed)
    }

    /// One DS2 request; the data after the status byte A0. A busy unit is
    /// asked again a few times.
    pub fn ds2_request(&mut self, address: u8, data: &[u8]) -> Result<Vec<u8>, BmwError> {
        if !request_allowed(BmwProtocol::Ds2, data) {
            return Err(BmwError::Failed(not_allowed()));
        }
        self.select(BmwProtocol::Ds2)?;
        let frame = ds2_frame(address, data).ok_or_else(|| BmwError::Failed(not_allowed()))?;
        for _ in 0..4 {
            sleep_ms(REQUEST_GAP_MS);
            self.send(&frame)?;
            let reply = self.ds2_receive(self.answer_ms)?;
            if reply.address != address {
                return Err(BmwError::Failed(format!(
                    "{}0x{:02X}",
                    l(
                        "Antwort von einer anderen Adresse: ",
                        "Answer from another address: "
                    ),
                    reply.address
                )));
            }
            match reply.status {
                ds2::OK => return Ok(reply.data),
                ds2::BUSY => sleep_ms(200),
                other => return Err(BmwError::Failed(ds2_status_text(other))),
            }
        }
        Err(BmwError::Failed(ds2_status_text(ds2::BUSY)))
    }

    // --- KWP2000 and BMW-FAST ---------------------------------------------------

    /// Opens a KWP2000 session with fast init at 10400 baud.
    fn start_session(&mut self, address: u8) -> Result<(), BmwError> {
        self.select(BmwProtocol::Kwp2000)?;
        if self.session == Some(address) {
            return Ok(());
        }
        self.stop_session();
        let start = kwp_frame(address, TESTER, &[kwp::START_COMMUNICATION]);
        self.log(&format!("   fast init -> {}", hex(&start)));
        let Some(answer) = self.stream.fast_init(&start) else {
            return Err(BmwError::NoAnswer);
        };
        self.log(&format!("   <- {}", hex(&answer)));
        // The header may or may not be part of the answer (a J2534 device
        // strips it); the positive response C1 is in there either way.
        let body = answer.get(1..).unwrap_or_default();
        if let Some(at) = body.iter().position(|&b| b == kwp::NEGATIVE) {
            let code = body.get(at + 2).copied().unwrap_or(0);
            return Err(BmwError::Failed(negative_response_text(code)));
        }
        if !body.contains(&(kwp::START_COMMUNICATION + 0x40)) {
            return Err(BmwError::Failed(format!(
                "{}{}",
                l("Unerwartete Antwort: ", "Unexpected answer: "),
                hex(&answer)
            )));
        }
        self.session = Some(address);
        // P3min before the first request.
        sleep_ms(REQUEST_GAP_MS);
        Ok(())
    }

    /// Ends an open KWP2000 session; harmless when none is open.
    pub fn stop_session(&mut self) {
        let Some(address) = self.session.take() else {
            return;
        };
        let frame = kwp_frame(address, TESTER, &[kwp::STOP_COMMUNICATION]);
        if self.send(&frame).is_ok() {
            let _ = read_kwp_message(&mut self.stream, SCAN_ANSWER_MS, INTER_BYTE_MS);
        }
    }

    /// One KWP2000 or BMW-FAST request; the positive answer, service byte
    /// first. A unit that asks for more time (7F xx 78) gets it.
    pub fn kwp_request(
        &mut self,
        protocol: BmwProtocol,
        address: u8,
        data: &[u8],
    ) -> Result<Vec<u8>, BmwError> {
        if protocol == BmwProtocol::Ds2 || !request_allowed(protocol, data) || data.is_empty() {
            return Err(BmwError::Failed(not_allowed()));
        }
        if protocol == BmwProtocol::Kwp2000 {
            self.start_session(address)?;
        } else {
            self.select(protocol)?;
        }
        sleep_ms(REQUEST_GAP_MS);
        self.send(&kwp_frame(address, TESTER, data))?;
        let mut wait = self.answer_ms;
        let mut pending = false;
        for _ in 0..10 {
            let Some(format) = self.stream.read_byte(wait) else {
                // Silence right after the request is a missing unit.
                return Err(if pending {
                    BmwError::Failed(BmwError::NoAnswer.message())
                } else {
                    BmwError::NoAnswer
                });
            };
            let (target, source, reply) =
                read_kwp_rest(&mut self.stream, format, INTER_BYTE_MS).map_err(BmwError::Failed)?;
            self.log(&format!("   <- {}", hex(&reply)));
            if target != TESTER || source != address {
                continue; // somebody else's conversation
            }
            if reply.len() >= 3 && reply[0] == kwp::NEGATIVE {
                if reply[2] == kwp::RESPONSE_PENDING {
                    wait = PENDING_MS;
                    pending = true;
                    continue;
                }
                return Err(BmwError::Failed(negative_response_text(reply[2])));
            }
            if reply.first() == Some(&(data[0] + 0x40)) {
                return Ok(reply);
            }
        }
        Err(BmwError::Failed(
            l(
                "Steuergerät antwortet nicht sinnvoll",
                "Control unit does not respond meaningfully",
            )
            .to_string(),
        ))
    }

    // --- Services -------------------------------------------------------------

    /// Is anyone at `address` speaking `protocol`? The identification when
    /// so. `scan` uses the short timeouts for missing units.
    pub fn identify(
        &mut self,
        protocol: BmwProtocol,
        address: u8,
        scan: bool,
    ) -> Result<String, BmwError> {
        self.answer_ms = if scan { SCAN_ANSWER_MS } else { ANSWER_MS };
        let result = match protocol {
            BmwProtocol::Ds2 => self
                .ds2_request(address, &[ds2::IDENT])
                .map(|data| identification_text(&data)),
            _ => self
                .kwp_request(protocol, address, &[kwp::READ_ECU_IDENTIFICATION, 0x80])
                .map(|reply| identification_text(reply.get(2..).unwrap_or_default())),
        };
        self.answer_ms = ANSWER_MS;
        result
    }

    pub fn read_faults(
        &mut self,
        protocol: BmwProtocol,
        address: u8,
    ) -> Result<Vec<BmwFault>, String> {
        match protocol {
            BmwProtocol::Ds2 => {
                // Most units take 04 alone; some want a block number and
                // say so with B0.
                let data = match self.ds2_request(address, &[ds2::READ_FAULTS]) {
                    Err(BmwError::Failed(_)) => {
                        self.ds2_request(address, &[ds2::READ_FAULTS, 0x01])
                    }
                    other => other,
                };
                data.map(|d| ds2_faults(&d)).map_err(|e| e.message())
            }
            _ => self
                .kwp_request(
                    protocol,
                    address,
                    &[kwp::READ_DTC_BY_STATUS, 0x02, 0xFF, 0xFF],
                )
                .map(|reply| kwp_faults(&reply))
                .map_err(|e| e.message()),
        }
    }

    pub fn clear_faults(&mut self, protocol: BmwProtocol, address: u8) -> Result<(), String> {
        match protocol {
            BmwProtocol::Ds2 => self
                .ds2_request(address, &[ds2::CLEAR_FAULTS])
                .map(|_| ())
                .map_err(|e| e.message()),
            _ => self
                .kwp_request(
                    protocol,
                    address,
                    &[kwp::CLEAR_DIAGNOSTIC_INFORMATION, 0xFF, 0xFF],
                )
                .map(|_| ())
                .map_err(|e| e.message()),
        }
    }

    /// Asks one address in one protocol and, when someone answers, reads
    /// the fault memory. `Ok(None)` when nobody is there; an error only when
    /// the line itself does not work (no echo), which ends a scan.
    pub fn probe(&mut self, protocol: BmwProtocol, address: u8) -> Result<Option<BmwUnit>, String> {
        let mut unit = BmwUnit {
            address,
            protocol,
            name: module_name(address, protocol),
            identification: String::new(),
            faults: Vec::new(),
            faults_read: false,
            note: String::new(),
        };
        match self.identify(protocol, address, true) {
            Ok(text) => unit.identification = text,
            Err(BmwError::NoAnswer) => {
                self.stop_session();
                return Ok(None);
            }
            Err(BmwError::NoEcho) => return Err(no_echo_text()),
            // A refused identification still means a unit is there.
            Err(BmwError::Failed(error)) => unit.note = error,
        }
        match self.read_faults(protocol, address) {
            Ok(faults) => {
                unit.faults = faults;
                unit.faults_read = true;
            }
            Err(error) => unit.note = error,
        }
        self.stop_session();
        Ok(Some(unit))
    }

    /// Clears one unit and reads it again. The unit as it is afterwards.
    pub fn clear_and_check(
        &mut self,
        protocol: BmwProtocol,
        address: u8,
    ) -> Result<BmwUnit, String> {
        let cleared = self.clear_faults(protocol, address);
        sleep_ms(300);
        let after = self.probe(protocol, address)?;
        self.stop_session();
        cleared?;
        after.ok_or_else(|| BmwError::NoAnswer.message())
    }

    /// Runs the whole [`scan_plan`]; an address found once is not asked
    /// again in another protocol. Stops early when `on_event` returns false;
    /// an error means the line does not work at all.
    pub fn scan(
        &mut self,
        mut on_event: impl FnMut(ScanEvent<'_>) -> bool,
    ) -> Result<Vec<BmwUnit>, String> {
        let plan = scan_plan();
        let total = plan.len();
        let mut found: Vec<BmwUnit> = Vec::new();
        for (index, &(protocol, address)) in plan.iter().enumerate() {
            if found.iter().any(|u| u.address == address) {
                continue;
            }
            let name = module_name(address, protocol);
            let go_on = on_event(ScanEvent::Trying {
                index,
                total,
                protocol,
                address,
                name: &name,
            });
            if !go_on {
                break;
            }
            if let Some(unit) = self.probe(protocol, address)? {
                let go_on = on_event(ScanEvent::Found(&unit));
                found.push(unit);
                if !go_on {
                    break;
                }
            }
        }
        self.stop_session();
        Ok(found)
    }
}

fn not_allowed() -> String {
    l(
        "Diese Anfrage schickt Caruspex nicht, es liest nur und löscht Fehler.",
        "Caruspex does not send this request, it only reads and clears faults.",
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ds2_frames_carry_their_whole_length_and_an_xor() {
        // Identification of the engine: 12 04 00 16.
        assert_eq!(ds2_frame(0x12, &[0x00]).unwrap(), [0x12, 0x04, 0x00, 0x16]);
        // Reading the instrument cluster: 80 04 04 80.
        assert_eq!(ds2_frame(0x80, &[0x04]).unwrap(), [0x80, 0x04, 0x04, 0x80]);
        assert_eq!(ds2_checksum(&[0x12, 0x05, 0x04, 0x01]), 0x12);
        assert!(ds2_frame(0x12, &[0; 253]).is_none());
    }

    #[test]
    fn ds2_answers_are_checked() {
        let good = [0x12, 0x05, 0xA0, 0x42, 0x12 ^ 0x05 ^ 0xA0 ^ 0x42];
        let reply = parse_ds2_frame(&good).unwrap();
        assert_eq!(reply.status, ds2::OK);
        assert_eq!(reply.data, [0x42]);
        let mut bad = good;
        bad[4] ^= 1;
        assert!(parse_ds2_frame(&bad).is_err());
        assert!(parse_ds2_frame(&good[..4]).is_err());
        assert!(!ds2_status_text(ds2::REJECTED).is_empty());
    }

    #[test]
    fn kwp_frames_have_header_and_sum() {
        let frame = kwp_frame(0x12, TESTER, &[0x1A, 0x80]);
        assert_eq!(frame, [0x82, 0x12, 0xF1, 0x1A, 0x80, 0x1F]);
        // Long data moves the length into its own byte.
        let long = kwp_frame(0x12, TESTER, &[0u8; 70]);
        assert_eq!(&long[..4], &[0x80, 0x12, 0xF1, 70]);
        assert_eq!(long.len(), 4 + 70 + 1);
    }

    #[test]
    fn faults_keep_bmw_codes_as_hex() {
        let ds2 = ds2_faults(&[2, 0x2A, 0x01, 0x10, 0x05]);
        assert_eq!(ds2.len(), 2);
        assert_eq!(ds2[0].code, "0x2A");
        assert_eq!(ds2[0].detail, "01");
        assert_eq!(ds2[0].text, bmw_fault_text());
        assert!(ds2_faults(&[0]).is_empty());
        // Three bytes for two entries does not split evenly: raw data.
        let raw = ds2_faults(&[2, 1, 2, 3]);
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0].code, "?");

        let kwp = kwp_faults(&[0x58, 2, 0x2A, 0x82, 0x60, 0x5E, 0x20, 0x20]);
        assert_eq!(kwp.len(), 2);
        assert_eq!(kwp[0].code, "0x2A82");
        assert_eq!(kwp[0].present, Some(true));
        assert_eq!(kwp[1].present, Some(false));
    }

    #[test]
    fn identification_keeps_the_readable_parts() {
        let mut data = b"7519308".to_vec();
        data.extend_from_slice(&[0x0B, 0x06]);
        data.extend_from_slice(b"SIEMENS");
        assert_eq!(identification_text(&data), "7519308 SIEMENS");
        assert_eq!(identification_text(&[1, 2]), "01 02");
    }

    #[test]
    fn only_reading_and_clearing_may_leave_the_tester() {
        assert!(request_allowed(BmwProtocol::Ds2, &[0x00]));
        assert!(request_allowed(BmwProtocol::Ds2, &[0x05]));
        // Status reads, actuators, memory and coding stay out.
        for data in [&[0x0B][..], &[0x0C, 0x01], &[0x06, 0, 0], &[0x08], &[0x1B]] {
            assert!(!request_allowed(BmwProtocol::Ds2, data));
        }
        assert!(request_allowed(BmwProtocol::BmwFast, &[0x14, 0xFF, 0xFF]));
        for data in [
            &[0x2E, 0xF1, 0x90][..],
            &[0x30, 0x01],
            &[0x31, 0x01],
            &[0x3B, 0x01],
            &[0x27, 0x01],
            &[0x10, 0x86],
            &[0x3D, 0, 0, 0],
        ] {
            assert!(!request_allowed(BmwProtocol::Kwp2000, data));
        }
    }

    #[test]
    fn the_scan_plan_groups_by_line_setting() {
        let plan = scan_plan();
        let first_kwp = plan
            .iter()
            .position(|p| p.0 == BmwProtocol::Kwp2000)
            .unwrap();
        let first_fast = plan
            .iter()
            .position(|p| p.0 == BmwProtocol::BmwFast)
            .unwrap();
        assert!(plan[..first_kwp].iter().all(|p| p.0 == BmwProtocol::Ds2));
        assert!(
            plan[first_kwp..first_fast]
                .iter()
                .all(|p| p.0 == BmwProtocol::Kwp2000)
        );
        assert_eq!(plan[0], (BmwProtocol::Ds2, 0x12));
        // Names follow the address map of the protocol.
        assert!(module_name(0x60, BmwProtocol::Ds2).contains("PDC"));
        assert!(module_name(0x60, BmwProtocol::BmwFast).contains("KOMBI"));
        assert!(module_name(0x7E, BmwProtocol::Ds2).contains("0x7E"));
        assert_eq!(
            BmwProtocol::from_key("bmw-fast"),
            Some(BmwProtocol::BmwFast)
        );
    }
}
