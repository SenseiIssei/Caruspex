//! KWP2000 on the K-line, VAG's successor to KWP1281.
//!
//! Control units switched over from about 2001. The Audi A2 sits right on the
//! boundary: the 1.4 with port injection mostly still speaks KWP1281, the
//! 1.6 FSI with its Bosch MED mostly already KWP2000.
//!
//! Unlike KWP1281 this is not a block protocol with an acknowledgement per
//! byte; whole messages with header and checksum are sent instead:
//!
//! ```text
//!     [Format] [Target] [Source] [Data ...] [Checksum]
//! ```
//!
//! The format byte carries the data length in its lower six bits.

use caruspex_data::vag_faults::describe_vag_fault;
use caruspex_i18n::l;
use caruspex_link::ByteStream;
use caruspex_link::timing::sleep_ms;

use crate::kwp1281::{
    Ecu, KLineLog, Kwp1281Fault, MeasuringValue, decode_measurement, ecu_name, read_key_bytes,
};

/// The two key bytes after the wake-up address tell which protocol the
/// control unit speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum KLineProtocol {
    #[default]
    Unknown,
    Kwp1281,
    Kwp2000,
}

/// Services that VAG control units offer over KWP2000.
pub mod service {
    pub const START_DIAGNOSTIC_SESSION: u8 = 0x10;
    pub const CLEAR_DIAGNOSTIC_INFORMATION: u8 = 0x14;
    pub const READ_DTC_BY_STATUS: u8 = 0x18;
    pub const READ_ECU_IDENTIFICATION: u8 = 0x1A;
    pub const READ_DATA_BY_LOCAL_ID: u8 = 0x21;
    pub const SECURITY_ACCESS: u8 = 0x27;
    pub const START_ROUTINE_BY_LOCAL_ID: u8 = 0x31;
    pub const STOP_ROUTINE_BY_LOCAL_ID: u8 = 0x32;
    pub const TESTER_PRESENT: u8 = 0x3E;
    pub const STOP_COMMUNICATION: u8 = 0x82;
    pub const NEGATIVE: u8 = 0x7F;
}

/// Session types. 0x89 is the usual diagnostic session at VAG.
pub mod session {
    pub const DEFAULT: u8 = 0x81;
    pub const DIAGNOSTICS: u8 = 0x89;
    pub const ADAPTATION: u8 = 0x85;
}

const BYTE_TIMEOUT_MS: u64 = 1200;
const ECHO_TIMEOUT_MS: u64 = 600;
const INTER_BYTE_GAP_MS: u64 = 5;

fn hex2(value: u8) -> String {
    format!("{value:02X}")
}

fn no_session() -> String {
    l("Keine Sitzung offen", "No session open").to_string()
}

pub fn protocol_from_key_bytes(key1: u8, key2: u8) -> KLineProtocol {
    // The seventh bit is parity in each case and gets in the way of comparing.
    let k1 = key1 & 0x7F;
    let k2 = key2 & 0x7F;

    // 0x01 0x0A identifies the old block-protocol control units.
    if k1 == 0x01 && k2 == 0x0A {
        return KLineProtocol::Kwp1281;
    }
    // 0x6B 0x0F and relatives stand for KWP2000.
    if k1 == 0x6B || k1 == 0x0F || k1 == 0x6D || k1 == 0x6E {
        return KLineProtocol::Kwp2000;
    }
    // Anything with the upper bit set in the second key byte also speaks
    // KWP2000, in our experience.
    if k2 == 0x0F || k2 == 0x8F {
        return KLineProtocol::Kwp2000;
    }
    KLineProtocol::Unknown
}

pub fn protocol_label(protocol: KLineProtocol) -> &'static str {
    match protocol {
        KLineProtocol::Kwp1281 => l("KWP1281 (Blockprotokoll)", "KWP1281 (block protocol)"),
        KLineProtocol::Kwp2000 => "KWP2000 (ISO 14230)",
        KLineProtocol::Unknown => l("unbekannt", "unknown"),
    }
}

/// Wakes a control unit with the 5-baud address and reads the key bytes.
/// Afterwards it is clear which protocol is used: the key bytes and the
/// protocol come back. Both protocols share this start, which is why it
/// lives here in one place.
///
/// The start is the same for both protocols: wake-up address at 5 baud, then
/// the control unit responds with the sync byte 0x55 and two key bytes. The
/// tester answers with the inverted second key byte. Only after that do the
/// paths diverge.
pub fn wake_up_kline<S: ByteStream + ?Sized>(
    stream: &mut S,
    ecu: Ecu,
    log: Option<&KLineLog>,
) -> Result<(u8, u8, KLineProtocol), String> {
    if !stream.slow_init(ecu.address()) {
        return Err(format!(
            "{}{}{}",
            l("Steuergerät ", "Control unit "),
            ecu_name(ecu),
            l(
                " meldet sich nicht.\n  - Zündung an?\n  - Hat das Fahrzeug dieses Steuergerät überhaupt?\n  - Bei neueren Fahrzeugen läuft die Diagnose über CAN, nicht K-Line.",
                " does not respond.\n  - Ignition on?\n  - Does the vehicle have this control unit at all?\n  - Newer vehicles are diagnosed over CAN, not K-Line."
            )
        ));
    }

    let sync = stream.read_byte(2000);
    if sync != Some(0x55) {
        return Err(format!(
            "{}{})",
            l(
                "Kein Synchronbyte 0x55 vom Steuergerät (bekommen ",
                "No sync byte 0x55 from the control unit (got "
            ),
            hex2(sync.unwrap_or(0))
        ));
    }
    let Some((key1, key2)) = read_key_bytes(stream) else {
        return Err(l("Schlüsselbytes fehlen", "Key bytes missing").to_string());
    };

    let protocol = protocol_from_key_bytes(key1, key2);
    if let Some(log) = log {
        log(&format!(
            "{}{} {} -> {}",
            l("   Schlüsselbytes ", "   Key bytes "),
            hex2(key1),
            hex2(key2),
            protocol_label(protocol)
        ));
    }

    sleep_ms(30);
    let answer = !key2;
    if !stream.write_byte(answer) {
        return Err(l(
            "Antwort auf die Schlüsselbytes lässt sich nicht senden",
            "Cannot send the reply to the key bytes",
        )
        .to_string());
    }
    if stream.echoes_own_bytes() && stream.read_byte(600) != Some(answer) {
        return Err(l(
            "Kein sauberes Echo auf die Antwort",
            "No clean echo of the reply",
        )
        .to_string());
    }

    // KWP2000 control units then also send the inverted address. If it is
    // missing that is no big deal, so only listen briefly.
    if protocol == KLineProtocol::Kwp2000 {
        let _inverted_address = stream.read_byte(400);
    }

    if protocol == KLineProtocol::Unknown {
        return Err(format!(
            "{}{} {}{}",
            l("Unbekannte Schlüsselbytes ", "Unknown key bytes "),
            hex2(key1),
            hex2(key2),
            l(
                ". Das Steuergerät spricht weder KWP1281 noch KWP2000 so, wie erwartet.",
                ". The control unit speaks neither KWP1281 nor KWP2000 as expected."
            )
        ));
    }
    Ok((key1, key2, protocol))
}

/// Plain-text reason for a rejected request.
pub fn negative_response_text(code: u8) -> String {
    match code {
        0x10 => l("Dienst abgelehnt", "Service rejected").to_string(),
        0x11 => l("Dienst wird nicht unterstützt", "Service not supported").to_string(),
        0x12 => l(
            "Unterfunktion wird nicht unterstützt",
            "Sub-function not supported",
        )
        .to_string(),
        0x21 => l(
            "Steuergerät beschäftigt, später erneut versuchen",
            "Control unit busy, try again later",
        )
        .to_string(),
        0x22 => l(
            "Bedingungen stimmen nicht (Motor läuft? Zündung aus?)",
            "Conditions not correct (engine running? ignition off?)",
        )
        .to_string(),
        0x31 => l(
            "Angefragter Wert liegt außerhalb des Bereichs",
            "Requested value is out of range",
        )
        .to_string(),
        0x33 => l(
            "Zugriff verweigert, Freischaltcode nötig",
            "Access denied, security code required",
        )
        .to_string(),
        0x35 => l("Ungültiger Schlüssel", "Invalid key").to_string(),
        0x78 => l(
            "Antwort dauert länger, Steuergerät arbeitet noch",
            "Response pending, control unit still working",
        )
        .to_string(),
        _ => format!(
            "{}{}",
            l("Ablehnungsgrund 0x", "Rejection reason 0x"),
            hex2(code)
        ),
    }
}

/// A KWP2000 fault entry takes three bytes: two for the number, one for the
/// status. `ecu_address` is the address of the reporting control unit; it
/// decides what a VAG number means. -1 if unknown.
pub fn decode_kwp2000_fault(high: u8, low: u8, status: u8, ecu_address: i32) -> Kwp1281Fault {
    let code = (i32::from(high) << 8) | i32::from(low);
    Kwp1281Fault {
        code,
        status,
        // Bit 7 set means the fault is currently present. Otherwise it is
        // intermittent.
        sporadic: (status & 0x80) == 0,
        text: describe_vag_fault(code, ecu_address),
    }
}

/// KWP2000 on a K-line after the wake-up. Owns the byte stream; get it back
/// with [`Kwp2000::into_stream`].
pub struct Kwp2000<S: ByteStream> {
    stream: S,
    log: Option<KLineLog>,
    identification: Vec<String>,
    target: u8,
    source: u8,
    started: bool,
}

impl<S: ByteStream> Kwp2000<S> {
    pub fn new(stream: S, log: Option<KLineLog>) -> Self {
        Self {
            stream,
            log,
            identification: Vec::new(),
            target: 0x01,
            source: 0xF1,
            started: false,
        }
    }

    pub fn stream(&self) -> &S {
        &self.stream
    }

    pub fn stream_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    /// The stream back, without signing off (see [`Kwp2000::stop`]).
    pub fn into_stream(self) -> S {
        self.stream
    }

    fn log(&self, line: &str) {
        if let Some(log) = &self.log {
            log(line);
        }
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn identification(&self) -> &[String] {
        &self.identification
    }

    // --- Messages -----------------------------------------------------------

    fn send_message(&mut self, payload: &[u8]) -> Result<(), String> {
        if payload.is_empty() || payload.len() > 63 {
            return Err(l(
                "Nachricht hat eine unmögliche Länge",
                "Message has an impossible length",
            )
            .to_string());
        }

        let mut frame = vec![0x80 | payload.len() as u8, self.target, self.source];
        frame.extend_from_slice(payload);
        let checksum = frame.iter().fold(0u8, |sum, &b| sum.wrapping_add(b));
        frame.push(checksum);

        if self.log.is_some() {
            let text: String = frame.iter().map(|&b| hex2(b) + " ").collect();
            self.log(&format!("   -> {text}"));
        }

        for &byte in &frame {
            if !self.stream.write_byte(byte) {
                return Err(l("Senden fehlgeschlagen", "Send failed").to_string());
            }
            // Single-wire line: our own byte comes back as an echo and must be
            // removed.
            if self.stream.echoes_own_bytes() {
                let Some(echo) = self.stream.read_byte(ECHO_TIMEOUT_MS) else {
                    return Err(format!(
                        "{}{}{}",
                        l("Kein Echo auf ", "No echo for "),
                        hex2(byte),
                        l(
                            ", hängt das Kabel richtig?",
                            ", is the cable connected properly?"
                        )
                    ));
                };
                if echo != byte {
                    return Err(format!(
                        "{}{}{}{}",
                        l("Echo passt nicht: erwartet ", "Echo mismatch: expected "),
                        hex2(byte),
                        l(", bekommen ", ", got "),
                        hex2(echo)
                    ));
                }
            }
            sleep_ms(INTER_BYTE_GAP_MS);
        }
        Ok(())
    }

    fn receive_message(&mut self) -> Result<Vec<u8>, String> {
        let Some(format) = self.stream.read_byte(BYTE_TIMEOUT_MS) else {
            return Err(l(
                "Steuergerät antwortet nicht",
                "Control unit does not respond",
            )
            .to_string());
        };
        if (format & 0xC0) != 0x80 {
            return Err(format!(
                "{}{}",
                l("Unerwartetes Formatbyte ", "Unexpected format byte "),
                hex2(format)
            ));
        }

        let addresses = self.stream.read_byte(BYTE_TIMEOUT_MS).and_then(|target| {
            self.stream
                .read_byte(BYTE_TIMEOUT_MS)
                .map(|source| (target, source))
        });
        let Some((target, source)) = addresses else {
            return Err(l(
                "Adressbytes fehlen in der Antwort",
                "Address bytes missing in the response",
            )
            .to_string());
        };

        let mut length = usize::from(format & 0x3F);
        if length == 0 {
            // The length is in a separate byte after the addresses.
            let Some(extra) = self.stream.read_byte(BYTE_TIMEOUT_MS) else {
                return Err(l("Längenbyte fehlt", "Length byte missing").to_string());
            };
            length = usize::from(extra);
        }

        let mut checksum = format.wrapping_add(target).wrapping_add(source);
        let mut payload = Vec::with_capacity(length);
        for i in 0..length {
            let Some(byte) = self.stream.read_byte(BYTE_TIMEOUT_MS) else {
                return Err(format!(
                    "{}{}{}{}{}",
                    l("Antwort bricht nach ", "Response breaks off after "),
                    i,
                    l(" von ", " of "),
                    length,
                    l(" Bytes ab", " bytes")
                ));
            };
            payload.push(byte);
            checksum = checksum.wrapping_add(byte);
        }

        let Some(received) = self.stream.read_byte(BYTE_TIMEOUT_MS) else {
            return Err(l("Prüfsumme fehlt", "Checksum missing").to_string());
        };
        if received != checksum {
            return Err(format!(
                "{}{}{}{}",
                l(
                    "Prüfsumme stimmt nicht: erwartet ",
                    "Checksum mismatch: expected "
                ),
                hex2(checksum),
                l(", bekommen ", ", got "),
                hex2(received)
            ));
        }

        if self.log.is_some() {
            let text: String = payload.iter().map(|&b| hex2(b) + " ").collect();
            self.log(&format!("   <- {text}"));
        }
        Ok(payload)
    }

    /// Raw service for everything not covered here. The positive response,
    /// service byte first.
    pub fn request(&mut self, payload: &[u8]) -> Result<Vec<u8>, String> {
        self.send_message(payload)?;

        // The control unit may ask for a delay with 0x7F ... 0x78. The actual
        // response then follows shortly afterwards.
        for _ in 0..6 {
            let reply = self.receive_message()?;
            if reply.len() >= 3 && reply[0] == service::NEGATIVE {
                if reply[2] == 0x78 {
                    sleep_ms(120);
                    continue; // control unit still working
                }
                return Err(negative_response_text(reply[2]));
            }
            if !reply.is_empty() && u32::from(reply[0]) == u32::from(payload[0]) + 0x40 {
                return Ok(reply);
            }
            // Something unexpected: listen once more.
        }
        Err(l(
            "Steuergerät antwortet nicht sinnvoll",
            "Control unit does not respond meaningfully",
        )
        .to_string())
    }

    // --- Session ------------------------------------------------------------

    /// The caller has already handled the wake-up address and key bytes;
    /// this only opens the session.
    pub fn start(&mut self, ecu: Ecu, key1: u8, key2: u8) -> Result<(), String> {
        self.identification.clear();
        self.target = ecu.address();
        self.started = false;

        if protocol_from_key_bytes(key1, key2) != KLineProtocol::Kwp2000 {
            return Err(format!(
                "{}{} {})",
                l(
                    "Dieses Steuergerät spricht kein KWP2000 (Schlüsselbytes ",
                    "This control unit does not speak KWP2000 (key bytes "
                ),
                hex2(key1),
                hex2(key2)
            ));
        }

        self.request(&[service::START_DIAGNOSTIC_SESSION, session::DIAGNOSTICS])?;
        self.started = true;
        self.read_ecu_identification();
        Ok(())
    }

    pub fn stop(&mut self) {
        if !self.started {
            return;
        }
        let _ = self.send_message(&[service::STOP_COMMUNICATION]);
        let _ = self.receive_message();
        self.started = false;
    }

    fn read_ecu_identification(&mut self) -> bool {
        // 0x9B returns part number and description as readable text.
        let Ok(reply) = self.request(&[service::READ_ECU_IDENTIFICATION, 0x9B]) else {
            return false;
        };
        let text: String = reply
            .iter()
            .skip(1)
            .map(|&b| b & 0x7F)
            .filter(|&c| (0x20..0x7F).contains(&c))
            .map(char::from)
            .collect();
        if !text.is_empty() {
            self.identification.push(text);
        }
        true
    }

    /// Without traffic the connection drops after about five seconds.
    pub fn tester_present(&mut self) -> Result<(), String> {
        if !self.started {
            return Err(no_session());
        }
        self.request(&[service::TESTER_PRESENT, 0x01]).map(|_| ())
    }

    // --- Services -----------------------------------------------------------

    pub fn read_faults(&mut self) -> Result<Vec<Kwp1281Fault>, String> {
        if !self.started {
            return Err(no_session());
        }

        // 0xFF 0x00 means: all faults, regardless of status.
        let reply = self.request(&[service::READ_DTC_BY_STATUS, 0x02, 0xFF, 0x00])?;
        let mut out = Vec::new();
        if reply.len() < 2 {
            return Ok(out); // response without entries
        }

        let count = usize::from(reply[1]);
        for i in 0..count {
            let at = 2 + i * 3;
            if at + 2 >= reply.len() {
                break;
            }
            let fault = decode_kwp2000_fault(
                reply[at],
                reply[at + 1],
                reply[at + 2],
                i32::from(self.target),
            );
            if fault.code != 0 && fault.code != 0xFFFF {
                out.push(fault);
            }
        }
        Ok(out)
    }

    pub fn clear_faults(&mut self) -> Result<(), String> {
        if !self.started {
            return Err(no_session());
        }
        self.request(&[service::CLEAR_DIAGNOSTIC_INFORMATION, 0xFF, 0x00])
            .map(|_| ())
    }

    pub fn read_group(&mut self, group: u8) -> Result<Vec<MeasuringValue>, String> {
        if !self.started {
            return Err(no_session());
        }

        let reply = self.request(&[service::READ_DATA_BY_LOCAL_ID, group])?;
        // Response: 0x61, group number, then three bytes per display field.
        Ok(reply
            .get(2..)
            .unwrap_or_default()
            .as_chunks::<3>()
            .0
            .iter()
            .map(|f| decode_measurement(f[0], f[1], f[2]))
            .collect())
    }

    /// Basic settings over KWP2000.
    ///
    /// Unlike KWP1281 there is no dedicated block title for this. The usual
    /// way is to open a different session type first and then read the same
    /// measuring block: the control unit then actively regulates. Which
    /// session type a given control unit requires is not uniform, so it can
    /// be specified ([`session::ADAPTATION`] is the usual one).
    pub fn basic_setting(
        &mut self,
        group: u8,
        session_mode: u8,
    ) -> Result<Vec<MeasuringValue>, String> {
        if !self.started {
            return Err(no_session());
        }

        if let Err(error) = self.request(&[service::START_DIAGNOSTIC_SESSION, session_mode]) {
            return Err(format!(
                "{}{}{}{}",
                l("Sitzungsart 0x", "Session type 0x"),
                hex2(session_mode),
                l(" abgelehnt: ", " rejected: "),
                error
            ));
        }
        self.read_group(group)
    }
}
