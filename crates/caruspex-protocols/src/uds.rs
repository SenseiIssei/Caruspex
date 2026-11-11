//! Unified Diagnostic Services (ISO 14229) over an ELM327, read-only plus
//! clearing fault codes.
//!
//! Almost every car since about 2008 speaks UDS on CAN, not only the engine
//! but also ABS, airbag, gearbox, instrument cluster, climate and the body
//! units. Standard OBD-II (modes 01 to 0A) only reaches the emission related
//! units; UDS reaches all of them, as long as the gateway lets the diagnostic
//! socket talk to them. Caruspex only reads: fault memory, identification.
//! The one writing service is clearing faults, with the same warnings as
//! everywhere.
//!
//! The ELM327 is told whom to talk to with ATSH (request id) and ATCRA
//! (response id); for 29-bit ids ATCP/ATSH and ATCRA work the same way.
//! BMW style extended addressing (one target byte in front of every frame)
//! is done in raw mode (ATCAF0) with the ISO-TP framing in Caruspex itself,
//! because many ELM327 clones handle ATCEA badly or not at all.
//! Afterwards the default OBD addressing is restored.

use caruspex_data::dtc::{decode_dtc, lookup_dtc};
use caruspex_data::ecu_addresses::EcuDefinition;
use caruspex_data::failure_types::failure_type_text;
use caruspex_i18n::l;
use caruspex_link::timing::{now_ms, sleep_ms};
use caruspex_link::{Elm327, ElmStatus, HeaderFormat, decode_messages, split_messages};

/// How long an ECU may keep answering 7F xx 78 ("still working") before we
/// give up. ISO 14229 allows 5 s per pending answer (P2* server); clearing a
/// big fault memory can need a few of them.
const PENDING_LIMIT_MS: u64 = 10000;
const MAX_ROUNDS: usize = 25;

// Timeouts on the PC side. The adapter itself gives up after its own ATST
// timer and prints "NO DATA", which normally comes much earlier.
const AT_TIMEOUT_MS: u64 = 2000;
const SHORT_REQUEST_MS: u64 = 1000;
const CLEAR_REQUEST_MS: u64 = 5000;

/// The usual wait of [`Uds::request`].
pub const DEFAULT_UDS_TIMEOUT_MS: u64 = 2000;

/// One fault from service 19: the two-byte SAE code plus the failure type
/// byte (what kind of fault) and the status byte (how current it is).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UdsDtc {
    /// "C0035", "B1000", "U0100"
    pub code: String,
    /// Third byte, SAE J2012-DA failure type.
    pub failure_type: u8,
    /// "Kurzschluss nach Masse" / "circuit short to ground"
    pub failure_text: String,
    /// ISO 14229 status byte.
    pub status: u8,
    /// Bit 0: failing right now.
    pub test_failed: bool,
    /// Bit 2.
    pub pending: bool,
    /// Bit 3: stored.
    pub confirmed: bool,
    /// Bit 7.
    pub warning_lamp: bool,
    /// Description from the DTC database.
    pub text: String,
}

/// What an ECU says about itself (service 22), empty where it does not answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UdsEcuInfo {
    /// F187 spare part number
    pub part_number: String,
    /// F189 or F195
    pub software_version: String,
    /// F191 or F193
    pub hardware_version: String,
    /// F197
    pub system_name: String,
    /// F18C
    pub serial: String,
    /// F190
    pub vin: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UdsEcu {
    pub definition: EcuDefinition,
    /// Answered at all.
    pub present: bool,
    /// Service 19 worked.
    pub dtcs_read: bool,
    pub info: UdsEcuInfo,
    pub dtcs: Vec<UdsDtc>,
    /// Plain text if something went wrong, active language.
    pub note: String,
}

fn hex_id(value: u32, digits: usize) -> String {
    format!("{value:0digits$X}")
}

/// True if the adapter answered the AT command with OK.
fn at_ok(elm: &mut Elm327, command: &str) -> bool {
    let r = elm.send(command, AT_TIMEOUT_MS);
    if r.status == ElmStatus::AdapterError {
        return false;
    }
    r.lines
        .iter()
        .any(|line| line.to_ascii_uppercase().contains("OK"))
}

fn bus29(elm: &Elm327) -> bool {
    matches!(elm.protocol_id(), '7' | '9')
}

/// Plain text for a negative response code (ISO 14229-1 annex A).
fn nrc_text(nrc: u8) -> String {
    let text: Option<&str> = match nrc {
        0x10 => Some(l("allgemeine Ablehnung", "general reject")),
        0x11 => Some(l("Dienst wird nicht unterstützt", "service not supported")),
        0x12 => Some(l(
            "Unterfunktion wird nicht unterstützt",
            "sub-function not supported",
        )),
        0x13 => Some(l(
            "falsche Länge oder falsches Format",
            "incorrect message length or format",
        )),
        0x14 => Some(l("Antwort wäre zu lang", "response too long")),
        0x21 => Some(l("beschäftigt, bitte wiederholen", "busy, repeat request")),
        0x22 => Some(l(
            "Bedingungen nicht erfüllt (Zündung, Motorzustand, Geschwindigkeit?)",
            "conditions not correct (ignition, engine state, speed?)",
        )),
        0x24 => Some(l(
            "falsche Reihenfolge der Anfragen",
            "request sequence error",
        )),
        0x31 => Some(l(
            "Kennung oder Wert außerhalb des Bereichs",
            "request out of range",
        )),
        0x33 => Some(l(
            "Zugriff gesperrt, braucht eine Freischaltung (Security Access)",
            "security access denied",
        )),
        0x35 => Some(l("falscher Schlüssel", "invalid key")),
        0x36 => Some(l("zu viele Versuche", "exceeded number of attempts")),
        0x37 => Some(l(
            "Wartezeit noch nicht abgelaufen",
            "required time delay not expired",
        )),
        0x72 => Some(l(
            "allgemeiner Programmierfehler",
            "general programming failure",
        )),
        0x78 => Some(l("Antwort noch in Arbeit", "response pending")),
        0x7E => Some(l(
            "Unterfunktion in der aktiven Sitzung nicht unterstützt",
            "sub-function not supported in active session",
        )),
        0x7F => Some(l(
            "Dienst in der aktiven Sitzung nicht unterstützt",
            "service not supported in active session",
        )),
        _ => None,
    };
    let code = format!("0x{nrc:02X}");
    let out = l("Steuergerät lehnt ab: ", "Control unit refuses: ");
    match text {
        Some(text) => format!("{out}{text} ({code})"),
        None => format!("{out}{}{code}", l("Code ", "code ")),
    }
}

fn still_busy() -> String {
    l(
        "Steuergerät bleibt beschäftigt und liefert keine Antwort.",
        "Control unit stays busy and never delivers the answer.",
    )
    .to_string()
}

fn no_data() -> String {
    l(
        "Keine Antwort vom Steuergerät (NO DATA).",
        "No answer from the control unit (NO DATA).",
    )
    .to_string()
}

/// What came back for one request.
#[derive(Default)]
struct Exchange {
    /// Any answer at all, negative ones included.
    answered: bool,
    positive: bool,
    /// Positive response without the service id.
    data: Vec<u8>,
    error: String,
}

/// Hex bytes of one raw adapter line, the 11-bit id dropped when headers
/// are on. Spaces may be on or off.
fn raw_line_bytes(line: &str, headers: bool) -> Vec<u8> {
    let mut hex = String::new();
    for c in line.chars() {
        if c.is_ascii_hexdigit() {
            hex.push(c);
        } else if c != ' ' {
            return Vec::new(); // "NO DATA", "CAN ERROR", "0:" ...
        }
    }
    if headers {
        if hex.len() < 3 {
            return Vec::new();
        }
        hex.drain(..3);
    }
    if !hex.len().is_multiple_of(2) {
        return Vec::new();
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0))
        .collect()
}

fn raw_frame(bytes: &[u8]) -> String {
    (0..8)
        .map(|i| format!("{:02X}", bytes.get(i).copied().unwrap_or(0x00)))
        .collect()
}

/// One request with BMW style extended addressing: the reassembled answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExtendedReply {
    pub answered: bool,
    /// Reassembled ISO-TP payload, service byte first.
    pub payload: Vec<u8>,
    pub error: String,
}

/// ISO-TP reassembly of raw frames that start with the tester address.
struct Reassembly {
    tester: u8,
    headers: bool,
    payload: Vec<u8>,
    expected: usize,
    next_index: u32,
    first_frame_seen: bool,
    answered: bool,
}

impl Reassembly {
    // Every frame of the unit starts with the tester address, then the PCI.
    fn take(&mut self, lines: &[String]) {
        for line in lines {
            let b = raw_line_bytes(line, self.headers);
            if b.len() < 2 || b[0] != self.tester {
                continue;
            }
            let pci = b[1];
            let kind = pci >> 4;
            if kind == 0 && !self.first_frame_seen {
                let length = usize::from(pci & 0x0F);
                if length == 0 || b.len() < 2 + length {
                    continue;
                }
                // A pending answer (7F xx 78) may come before the real one;
                // the last single frame wins.
                self.payload = b[2..2 + length].to_vec();
                self.expected = length;
                self.answered = true;
            } else if kind == 1 && b.len() >= 3 {
                self.expected = (usize::from(pci & 0x0F) << 8) | usize::from(b[2]);
                self.payload = b[3..].to_vec();
                self.first_frame_seen = true;
                self.next_index = 1;
                self.answered = true;
            } else if kind == 2 && self.first_frame_seen {
                if u32::from(pci & 0x0F) != (self.next_index & 0x0F) {
                    continue; // repeated or out of order
                }
                self.next_index += 1;
                self.payload.extend_from_slice(&b[2..]);
            }
        }
    }
}

/// One request with BMW style extended addressing, the framing done here:
/// the ISO-TP frames as the raw lines of the adapter (CAN auto formatting
/// off). `target` is the unit's address, the answer has to start with
/// `tester`. Requests up to six bytes (one single frame).
pub fn extended_request(
    elm: &mut Elm327,
    target: u8,
    tester: u8,
    bytes: &[u8],
    timeout_ms: u64,
) -> ExtendedReply {
    let mut reply = ExtendedReply::default();
    if bytes.is_empty() || bytes.len() > 6 {
        reply.error = l(
            "Anfrage zu lang (höchstens 6 Bytes mit erweiterter Adresse).",
            "Request too long (at most 6 bytes with extended addressing).",
        )
        .to_string();
        return reply;
    }
    let headers = elm.header_format() == HeaderFormat::Can11;
    let mut frame = vec![target, bytes.len() as u8];
    frame.extend_from_slice(bytes);
    let raw = elm.send(&raw_frame(&frame), timeout_ms);
    if raw.status == ElmStatus::AdapterError {
        reply.error = raw.message;
        return reply;
    }

    let mut r = Reassembly {
        tester,
        headers,
        payload: Vec::new(),
        expected: 0,
        next_index: 1,
        first_frame_seen: false,
        answered: false,
    };
    r.take(&raw.lines);

    if r.first_frame_seen {
        // Flow control from the tester: send everything, no pause. The
        // consecutive frames come back as the answer to this frame.
        let mut round = 0;
        while round < 4 && r.payload.len() < r.expected {
            let more = elm.send(&raw_frame(&[target, 0x30, 0x00, 0x00]), timeout_ms);
            if more.status == ElmStatus::AdapterError {
                break;
            }
            let before = r.payload.len();
            r.take(&more.lines);
            if r.payload.len() == before {
                break;
            }
            round += 1;
        }
        if r.payload.len() < r.expected {
            reply.answered = r.answered;
            reply.error = l(
                "Lange Antwort kam nicht vollständig an.",
                "A long answer did not arrive completely.",
            )
            .to_string();
            return reply;
        }
    }
    r.payload.truncate(r.expected);
    reply.answered = r.answered;
    reply.payload = r.payload;
    if !reply.answered {
        reply.error = no_data();
    }
    reply
}

/// The same rounds as `exchange`, for BMW style extended addressing.
fn exchange_extended(elm: &mut Elm327, target: u8, bytes: &[u8], timeout_ms: u64) -> Exchange {
    let mut ex = Exchange::default();
    let sid = bytes.first().copied().unwrap_or(0);
    let start = now_ms();
    for _ in 0..MAX_ROUNDS {
        let r = extended_request(elm, target, 0xF1, bytes, timeout_ms);
        if !r.answered || r.payload.is_empty() {
            ex.error = r.error;
            return ex;
        }
        let b = &r.payload;
        ex.answered = true;
        if u32::from(b[0]) == u32::from(sid) + 0x40 {
            ex.positive = true;
            ex.data = b[1..].to_vec();
            return ex;
        }
        if b[0] == 0x7F && b.len() >= 3 && b[1] == sid {
            if b[2] == 0x78 || b[2] == 0x21 {
                if now_ms() - start > PENDING_LIMIT_MS {
                    ex.error = still_busy();
                    return ex;
                }
                sleep_ms(if b[2] == 0x21 { 100 } else { 20 });
                continue;
            }
            ex.error = nrc_text(b[2]);
            return ex;
        }
        ex.error = l(
            "Keine verwertbare Antwort erhalten.",
            "No usable response received.",
        )
        .to_string();
        return ex;
    }
    ex.error = still_busy();
    ex
}

/// One request with all its rounds (7F xx 78 / 21 asks again). `extended`
/// is the BMW target byte, -1 for normal addressing.
fn exchange(elm: &mut Elm327, bytes: &[u8], timeout_ms: u64, extended: i32) -> Exchange {
    if extended >= 0 {
        return exchange_extended(elm, (extended & 0xFF) as u8, bytes, timeout_ms);
    }
    let mut ex = Exchange::default();
    if bytes.is_empty() || bytes.len() > 7 {
        // The ELM327 only sends single frames: at most seven data bytes.
        ex.error = l(
            "Anfrage zu lang für den Adapter (höchstens 7 Bytes).",
            "Request too long for the adapter (at most 7 bytes).",
        )
        .to_string();
        return ex;
    }
    let command: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
    let sid = bytes[0];

    let start = now_ms();
    for _ in 0..MAX_ROUNDS {
        let raw = elm.send(&command, timeout_ms);
        if raw.status == ElmStatus::AdapterError {
            ex.error = raw.message;
            return ex;
        }
        // decode_messages knows the adapter's status lines (NO DATA, CAN
        // ERROR, ...). The service filter does not matter for that part.
        let status = decode_messages(&raw.lines, sid, elm.header_format());
        if status.status == ElmStatus::BusError {
            ex.error = status.message;
            return ex;
        }
        let mut pending = false;
        let mut busy = false;
        let mut refusal: Option<u8> = None;
        for msg in split_messages(&raw.lines, elm.header_format()) {
            let b = &msg.bytes;
            if b.is_empty() {
                continue;
            }
            if u32::from(b[0]) == u32::from(sid) + 0x40 {
                ex.answered = true;
                ex.positive = true;
                ex.data = b[1..].to_vec();
                return ex; // a positive answer wins over a pending one before it
            }
            if b[0] == 0x7F && b.len() >= 3 && b[1] == sid {
                ex.answered = true;
                if b[2] == 0x78 {
                    pending = true;
                } else if b[2] == 0x21 {
                    busy = true;
                } else if refusal.is_none() {
                    refusal = Some(b[2]);
                }
            }
        }
        if let Some(nrc) = refusal {
            ex.error = nrc_text(nrc);
            return ex;
        }
        if pending || busy {
            // The ECU is still working. The adapter stopped listening at its
            // own timeout, so the late answer can only be had by asking
            // again. Every service used here may be repeated safely.
            if now_ms() - start > PENDING_LIMIT_MS {
                ex.error = still_busy();
                return ex;
            }
            sleep_ms(if busy && !pending { 100 } else { 20 });
            continue;
        }
        if status.status == ElmStatus::NoData
            && raw
                .lines
                .first()
                .is_some_and(|line| line.to_ascii_uppercase().starts_with("NO DATA"))
        {
            ex.error = no_data();
            return ex;
        }
        ex.error = l(
            "Keine verwertbare Antwort erhalten.",
            "No usable response received.",
        )
        .to_string();
        return ex;
    }
    ex.error = still_busy();
    ex
}

/// Printable ASCII only, trimmed. Identification strings are often padded
/// with spaces, zero bytes or 0xFF.
fn clean_ascii(bytes: &[u8]) -> String {
    let out: String = bytes
        .iter()
        .filter(|&&b| (0x20..=0x7E).contains(&b))
        .map(|&b| b as char)
        .collect();
    out.trim_matches(' ').to_string()
}

fn status_bits(dtc: &mut UdsDtc, status: u8) {
    dtc.status = status;
    dtc.test_failed = (status & 0x01) != 0;
    dtc.pending = (status & 0x04) != 0;
    dtc.confirmed = (status & 0x08) != 0;
    dtc.warning_lamp = (status & 0x80) != 0;
}

/// Decodes the three-byte DTC of service 19 ("C0035" plus failure type).
pub fn decode_uds_dtc(b1: u8, b2: u8, b3: u8, status: u8) -> UdsDtc {
    let mut dtc = UdsDtc {
        code: decode_dtc(i32::from(b1), i32::from(b2)),
        ..UdsDtc::default()
    };
    if dtc.code.is_empty() {
        dtc.code = "P0000".to_string();
    }
    dtc.failure_type = b3;
    dtc.failure_text = failure_type_text(i32::from(b3));
    status_bits(&mut dtc, status);
    dtc.text = lookup_dtc(&dtc.code, "").text;
    dtc
}

/// BMW stores its own codes in the three bytes, shown as six hex digits the
/// way ISTA does ("480A10"), without an SAE meaning or failure type.
pub fn decode_bmw_dtc(b1: u8, b2: u8, b3: u8, status: u8) -> UdsDtc {
    let mut dtc = UdsDtc {
        code: format!("{b1:02X}{b2:02X}{b3:02X}"),
        ..UdsDtc::default()
    };
    status_bits(&mut dtc, status);
    dtc.text = l(
        "BMW-Fehlercode. Die genaue Beschreibung steht in der Werkstattsoftware (ISTA); mit dem Code findet man sie auch in Foren und Fehlerlisten.",
        "BMW fault code. The exact description is in the workshop software (ISTA); forums and fault lists find it by the code as well.",
    )
    .to_string();
    dtc
}

/// UDS on one control unit at a time. Dropping it restores the default OBD
/// addressing.
pub struct Uds<'a> {
    elm: &'a mut Elm327,
    addressed: bool,
    /// Target byte with BMW style addressing, else -1.
    extended: i32,
}

impl<'a> Uds<'a> {
    pub fn new(elm: &'a mut Elm327) -> Self {
        Self {
            elm,
            addressed: false,
            extended: -1,
        }
    }

    /// The adapter underneath.
    pub fn elm(&mut self) -> &mut Elm327 {
        self.elm
    }

    /// Points the adapter at one ECU. `Err` if the adapter refused the setup.
    pub fn select(&mut self, ecu: &EcuDefinition) -> Result<(), String> {
        if !self.elm.connected() {
            return Err(l("Kein Adapter verbunden.", "No adapter connected.").to_string());
        }
        if !self.elm.is_can() {
            return Err(l(
                "UDS geht über den ELM327 nur auf CAN; dieses Auto spricht ein anderes Protokoll.",
                "UDS through the ELM327 needs CAN; this car speaks another protocol.",
            )
            .to_string());
        }
        if ecu.extended29 != bus29(self.elm) {
            return Err(if ecu.extended29 {
                l(
                    "Steuergerät mit 29-Bit-Adresse, das Auto ist aber mit 11 Bit verbunden.",
                    "Control unit with a 29-bit address, but the car is connected with 11 bit.",
                )
            } else {
                l(
                    "Steuergerät mit 11-Bit-Adresse, das Auto ist aber mit 29 Bit verbunden.",
                    "Control unit with an 11-bit address, but the car is connected with 29 bit.",
                )
            }
            .to_string());
        }

        // From here on the adapter is changed, so dropping puts it back even
        // if a step fails halfway.
        self.addressed = true;
        self.extended = -1;
        if ecu.extended_address >= 0 {
            if ecu.extended29 {
                self.restore_default();
                return Err(l(
                    "Erweiterte Adressierung gibt es hier nur mit 11-Bit-Adressen.",
                    "Extended addressing is only supported with 11-bit ids.",
                )
                .to_string());
            }
            // Raw frames: the adapter neither adds the PCI byte nor sends flow
            // control, Caruspex does both (see extended_request).
            let ok = at_ok(self.elm, &format!("ATSH{}", hex_id(ecu.request & 0x7FF, 3)))
                && at_ok(
                    self.elm,
                    &format!("ATCRA{}", hex_id(ecu.response & 0x7FF, 3)),
                )
                && at_ok(self.elm, "ATCAF0");
            if !ok {
                self.restore_default();
                return Err(l(
                    "Der Adapter nimmt die Adresse nicht an (ATSH/ATCRA/ATCAF0).",
                    "The adapter does not accept the address (ATSH/ATCRA/ATCAF0).",
                )
                .to_string());
            }
            self.extended = ecu.extended_address;
            return Ok(());
        }
        let ok = if ecu.extended29 {
            // ATSH takes the lower three bytes, ATCP the priority byte on top.
            at_ok(
                self.elm,
                &format!("ATCP{}", hex_id((ecu.request >> 24) & 0x1F, 2)),
            ) && at_ok(
                self.elm,
                &format!("ATSH{}", hex_id(ecu.request & 0xFF_FFFF, 6)),
            ) && at_ok(
                self.elm,
                &format!("ATCRA{}", hex_id(ecu.response & 0x1FFF_FFFF, 8)),
            )
        } else {
            at_ok(self.elm, &format!("ATSH{}", hex_id(ecu.request & 0x7FF, 3)))
                && at_ok(
                    self.elm,
                    &format!("ATCRA{}", hex_id(ecu.response & 0x7FF, 3)),
                )
        };
        if !ok {
            self.restore_default();
            return Err(l(
                "Der Adapter nimmt die Adresse nicht an (ATSH/ATCRA).",
                "The adapter does not accept the address (ATSH/ATCRA).",
            )
            .to_string());
        }
        // Flow control for multi-frame answers goes to the request id. In its
        // automatic mode the adapter guesses the id from the "+8" convention,
        // which not every maker follows, so it is set explicitly. Older clones
        // without these commands still work for ECUs that follow the
        // convention, which is why a refusal is not an error.
        let digits = if ecu.extended29 { 8 } else { 3 };
        if at_ok(self.elm, &format!("ATFCSH{}", hex_id(ecu.request, digits)))
            && at_ok(self.elm, "ATFCSD300000")
        {
            at_ok(self.elm, "ATFCSM1");
        }
        // CAN auto formatting (PCI bytes handled by the adapter) is the
        // default, and the ISO-TP parsing relies on it.
        at_ok(self.elm, "ATCAF1");
        Ok(())
    }

    /// Back to the functional OBD address (7DF/7E8 or 18DB33F1).
    pub fn restore_default(&mut self) {
        self.addressed = false;
        if !self.elm.connected() || !self.elm.is_can() {
            return;
        }
        if self.extended >= 0 {
            at_ok(self.elm, "ATCAF1");
        }
        self.extended = -1;
        at_ok(self.elm, "ATFCSM0");
        if bus29(self.elm) {
            at_ok(self.elm, "ATCP18");
            at_ok(self.elm, "ATSHDB33F1");
        } else {
            at_ok(self.elm, "ATSH7DF");
        }
        // ATCRA without an address came with v1.4b; clones with an older
        // command set reset the receive filter with ATAR.
        if !at_ok(self.elm, "ATCRA") {
            at_ok(self.elm, "ATAR");
        }
    }

    /// Raw request/response for one service, handles 7F xx 78 (still busy).
    /// Returns the positive response without the service id byte.
    /// [`DEFAULT_UDS_TIMEOUT_MS`] is the usual `timeout_ms`.
    pub fn request(&mut self, bytes: &[u8], timeout_ms: u64) -> Result<Vec<u8>, String> {
        let ex = exchange(self.elm, bytes, timeout_ms, self.extended);
        if !ex.positive {
            return Err(ex.error);
        }
        Ok(ex.data)
    }

    /// 3E 00. True if the selected ECU answered.
    pub fn tester_present(&mut self) -> bool {
        // Any answer counts, a refusal too: then the ECU is there but does not
        // like the service in its current session.
        exchange(self.elm, &[0x3E, 0x00], SHORT_REQUEST_MS, self.extended).answered
    }

    /// 19 02 with the given status mask (0xFF: everything the ECU reports).
    pub fn read_dtcs(&mut self, status_mask: u8) -> Result<Vec<UdsDtc>, String> {
        let response = self.request(&[0x19, 0x02, status_mask], DEFAULT_UDS_TIMEOUT_MS)?;
        // 59 02 <availability mask> then four bytes per fault.
        if response.len() < 2 || response[0] != 0x02 {
            return Err(l(
                "Unerwartete Antwort auf 19 02.",
                "Unexpected answer to 19 02.",
            )
            .to_string());
        }
        let mut out = Vec::new();
        let mut i = 2;
        while i + 3 < response.len() {
            let r = &response[i..i + 4];
            i += 4;
            if r[0] == 0 && r[1] == 0 && r[2] == 0 {
                continue;
            }
            if self.extended >= 0 {
                out.push(decode_bmw_dtc(r[0], r[1], r[2], r[3]));
            } else {
                out.push(decode_uds_dtc(r[0], r[1], r[2], r[3]));
            }
        }
        Ok(out)
    }

    /// One identifier, empty if the ECU does not have it.
    fn did(&mut self, id: u16) -> String {
        let [high, low] = id.to_be_bytes();
        let Ok(response) = self.request(&[0x22, high, low], SHORT_REQUEST_MS) else {
            return String::new();
        };
        if response.len() < 2 || response[0] != high || response[1] != low {
            return String::new();
        }
        clean_ascii(&response[2..])
    }

    /// 22 F187, F189, F191, F197, F18C, F190, each optional.
    pub fn read_info(&mut self) -> UdsEcuInfo {
        let mut info = UdsEcuInfo {
            part_number: self.did(0xF187),
            software_version: self.did(0xF189),
            ..UdsEcuInfo::default()
        };
        if info.software_version.is_empty() {
            info.software_version = self.did(0xF195);
        }
        info.hardware_version = self.did(0xF191);
        if info.hardware_version.is_empty() {
            info.hardware_version = self.did(0xF193);
        }
        info.system_name = self.did(0xF197);
        info.serial = self.did(0xF18C);
        info.vin = self.did(0xF190);
        info
    }

    /// 14 FF FF FF: clears all groups of the selected ECU.
    pub fn clear_dtcs(&mut self) -> Result<(), String> {
        self.request(&[0x14, 0xFF, 0xFF, 0xFF], CLEAR_REQUEST_MS)
            .map(|_| ())
    }
}

impl Drop for Uds<'_> {
    fn drop(&mut self) {
        if self.addressed {
            self.restore_default();
        }
    }
}

/// Progress callback of [`scan_ecus`]: index, total, name of the ECU.
pub type ScanProgress<'a> = dyn FnMut(usize, usize, &str) + 'a;

/// Asks every ECU of the list once: is it there, what is it, what does it
/// have stored. `progress(index, total, name)` is called before each ECU;
/// `cancelled()` is checked between ECUs. The adapter ends up on the default
/// OBD addressing again.
pub fn scan_ecus(
    elm: &mut Elm327,
    ecus: &[EcuDefinition],
    mut progress: Option<&mut ScanProgress<'_>>,
    cancelled: Option<&dyn Fn() -> bool>,
) -> Vec<UdsEcu> {
    let mut out = Vec::with_capacity(ecus.len());
    let can = elm.connected() && elm.is_can();

    // A missing ECU costs the adapter's full answer timeout ("NO DATA"),
    // twice per address. 200 ms instead of the 400 ms set at connect is
    // still four times what UDS gives an ECU for its first answer (P2 =
    // 50 ms), and a slow ECU says 7F xx 78 well within it.
    let shortened = can && at_ok(elm, "ATST32");

    {
        let mut uds = Uds::new(elm);
        let total = ecus.len();
        for (i, def) in ecus.iter().enumerate() {
            if cancelled.is_some_and(|c| c()) {
                break;
            }
            if let Some(p) = progress.as_mut() {
                p(i, total, &def.name);
            }

            let mut ecu = UdsEcu {
                definition: def.clone(),
                ..UdsEcu::default()
            };
            if let Err(error) = uds.select(def) {
                ecu.note = error;
                out.push(ecu);
                continue;
            }
            ecu.present = uds.tester_present();
            if !ecu.present {
                // Some ECUs ignore 3E outside an extended session but do
                // answer the identification.
                ecu.present = exchange(
                    uds.elm(),
                    &[0x22, 0xF1, 0x87],
                    SHORT_REQUEST_MS,
                    def.extended_address,
                )
                .answered;
            }
            if !ecu.present {
                ecu.note = l(
                    "Keine Antwort: nicht verbaut, oder das Gateway lässt die Anfrage nicht durch.",
                    "No answer: not fitted, or the gateway does not pass the request on.",
                )
                .to_string();
                out.push(ecu);
                continue;
            }
            ecu.info = uds.read_info();
            match uds.read_dtcs(0xFF) {
                Ok(dtcs) => {
                    ecu.dtcs = dtcs;
                    ecu.dtcs_read = true;
                }
                Err(error) => {
                    ecu.dtcs_read = false;
                    ecu.note = format!(
                        "{}{}",
                        l(
                            "Fehlerspeicher nicht lesbar. ",
                            "Fault memory not readable. "
                        ),
                        error
                    );
                }
            }
            out.push(ecu);
        }
        uds.restore_default();
    }

    if shortened {
        at_ok(elm, "ATST64"); // the value Elm327::connect sets
    }
    out
}
