//! KWP1281, VAG's own protocol from roughly 1995 to 2005.
//!
//! Unlike EOBD it reaches every control unit and the services that are not
//! emissions-related: measuring blocks, adaptation channels, output tests and
//! the basic settings used for throttle body adaptation.
//!
//! Why an ELM327 cannot do this: KWP1281 is a block protocol on a single-wire
//! line. The other side acknowledges every single byte with its one's
//! complement before the next one is sent. The ELM327 firmware only knows
//! complete requests and responses and cannot reproduce this byte rhythm.
//! It takes a KKL cable that passes the K-line through raw.

use std::sync::Arc;

use caruspex_data::vag_faults::describe_vag_fault;
use caruspex_i18n::l;
use caruspex_link::ByteStream;
use caruspex_link::timing::sleep_ms;

/// Log callback of the K-line protocols, one finished line per call. Shared,
/// because `VagKLine` hands the same one to whichever protocol it starts.
pub type KLineLog = Arc<dyn Fn(&str) + Send + Sync>;

/// A control unit address on the K-line, the same numbers VCDS uses. Any
/// address can be woken; the known units have names below.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ecu(pub u8);

// The names of the known units keep the spelling of the former enum variants.
#[allow(non_upper_case_globals)]
impl Ecu {
    pub const Engine: Ecu = Ecu(0x01);
    pub const Gearbox: Ecu = Ecu(0x02);
    pub const Abs: Ecu = Ecu(0x03);
    pub const Airbag: Ecu = Ecu(0x15);
    pub const InstrumentCluster: Ecu = Ecu(0x17);
    pub const CentralElectrics: Ecu = Ecu(0x09);
    pub const ComfortSystem: Ecu = Ecu(0x46);
    pub const Immobilizer: Ecu = Ecu(0x25);
    pub const AirConditioning: Ecu = Ecu(0x08);
    pub const SteeringWheel: Ecu = Ecu(0x16);
    pub const Radio: Ecu = Ecu(0x56);
}

impl Ecu {
    /// The address sent as the 5-baud wake-up.
    pub fn address(self) -> u8 {
        self.0
    }

    /// Any address, known or not.
    pub fn from_address(address: u8) -> Ecu {
        Ecu(address)
    }
}

pub fn ecu_name(ecu: Ecu) -> &'static str {
    match ecu {
        Ecu::Engine => l("01 Motor", "01 Engine"),
        Ecu::Gearbox => l("02 Getriebe", "02 Transmission"),
        Ecu::Abs => l("03 Bremsenelektronik (ABS)", "03 Brake electronics (ABS)"),
        Ecu::AirConditioning => l("08 Klimaanlage", "08 Air conditioning"),
        Ecu::CentralElectrics => l("09 Zentralelektrik", "09 Central electrics"),
        Ecu::Airbag => l("15 Airbag", "15 Airbags"),
        Ecu::SteeringWheel => l("16 Lenkradelektronik", "16 Steering wheel electronics"),
        Ecu::InstrumentCluster => l("17 Kombiinstrument", "17 Instrument cluster"),
        Ecu::Immobilizer => l("25 Wegfahrsperre", "25 Immobilizer"),
        Ecu::ComfortSystem => l("46 Komfortsystem", "46 Comfort system"),
        Ecu::Radio => "56 Radio",
        _ => l("unbekannt", "unknown"),
    }
}

pub fn known_ecus() -> &'static [Ecu] {
    const LIST: [Ecu; 11] = [
        Ecu::Engine,
        Ecu::Gearbox,
        Ecu::Abs,
        Ecu::AirConditioning,
        Ecu::CentralElectrics,
        Ecu::Airbag,
        Ecu::SteeringWheel,
        Ecu::InstrumentCluster,
        Ecu::Immobilizer,
        Ecu::ComfortSystem,
        Ecu::Radio,
    ];
    &LIST
}

/// Block titles of the protocol.
pub mod block {
    pub const CLEAR_FAULTS: u8 = 0x05;
    pub const END_OUTPUT: u8 = 0x06;
    pub const READ_FAULTS: u8 = 0x07;
    pub const ACK: u8 = 0x09;
    pub const ACTUATOR_TEST: u8 = 0x04;
    /// Basic settings.
    pub const BASIC_SETTING: u8 = 0x28;
    /// Read measuring block.
    pub const READ_GROUP: u8 = 0x29;
    pub const READ_ADAPTATION: u8 = 0x2A;
    pub const TEST_ADAPTATION: u8 = 0x2B;
    pub const SAVE_ADAPTATION: u8 = 0x2C;
    pub const LOGIN: u8 = 0x2B;
    pub const ASCII_DATA: u8 = 0xF6;
    pub const FAULT_LIST: u8 = 0xFC;
    pub const GROUP_READING: u8 = 0xE7;
    pub const ADAPTATION_VALUE: u8 = 0xE6;
    pub const NOT_SUPPORTED: u8 = 0x0A;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Kwp1281Fault {
    /// VAG's own five-digit number.
    pub code: i32,
    pub status: u8,
    /// Plain text, if known.
    pub text: String,
    pub sporadic: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeasuringValue {
    /// "Engine speed"
    pub label: String,
    pub value: f64,
    pub unit: String,
    pub valid: bool,
}

const BYTE_TIMEOUT_MS: u64 = 1200;
const ACK_TIMEOUT_MS: u64 = 500;
const BLOCK_END: u8 = 0x03;

fn hex(value: u8) -> String {
    format!("{value:02X}")
}

/// The two key bytes after the sync byte; the second is only waited for if
/// the first came.
pub(crate) fn read_key_bytes<S: ByteStream + ?Sized>(stream: &mut S) -> Option<(u8, u8)> {
    let key1 = stream.read_byte(1000)?;
    let key2 = stream.read_byte(1000)?;
    Some((key1, key2))
}

fn not_connected() -> String {
    l("Nicht verbunden", "Not connected").to_string()
}

/// The KWP1281 state machine on a K-line. Owns the byte stream; get it back
/// with [`Kwp1281::into_stream`].
pub struct Kwp1281<S: ByteStream> {
    stream: S,
    log: Option<KLineLog>,
    identification: Vec<String>,
    ecu: Ecu,
    counter: u8,
    connected: bool,
}

impl<S: ByteStream> Kwp1281<S> {
    pub fn new(stream: S, log: Option<KLineLog>) -> Self {
        Self {
            stream,
            log,
            identification: Vec::new(),
            ecu: Ecu::Engine,
            counter: 0,
            connected: false,
        }
    }

    pub fn stream(&self) -> &S {
        &self.stream
    }

    pub fn stream_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    /// The stream back, without signing off (see [`Kwp1281::disconnect`]).
    pub fn into_stream(self) -> S {
        self.stream
    }

    fn log(&self, line: &str) {
        if let Some(log) = &self.log {
            log(line);
        }
    }

    /// Address of the control unit, so fault numbers are interpreted
    /// correctly. With `attach` the caller has to set it.
    pub fn set_ecu(&mut self, ecu: Ecu) {
        self.ecu = ecu;
    }

    pub fn connected(&self) -> bool {
        self.connected
    }

    pub fn identification(&self) -> &[String] {
        &self.identification
    }

    pub fn block_counter(&self) -> u8 {
        self.counter
    }

    // --- Byte-by-byte rhythm ------------------------------------------------

    /// On the single-wire line you hear yourself. The echo has to be removed,
    /// otherwise it would be mistaken for the other side's answer.
    fn read_echo(&mut self, expected: u8) -> Result<(), String> {
        if !self.stream.echoes_own_bytes() {
            return Ok(());
        }
        let Some(echo) = self.stream.read_byte(ACK_TIMEOUT_MS) else {
            return Err(format!(
                "{}{}{}",
                l("Kein Echo auf ", "No echo for "),
                hex(expected),
                l(
                    ", hängt das Kabel richtig?",
                    ", is the cable connected properly?"
                )
            ));
        };
        if echo != expected {
            return Err(format!(
                "{}{}{}{}",
                l("Echo passt nicht: erwartet ", "Echo mismatch: expected "),
                hex(expected),
                l(", bekommen ", ", got "),
                hex(echo)
            ));
        }
        Ok(())
    }

    /// Send a byte, discard the echo, then wait for the other side's
    /// complement.
    fn send_byte_acked(&mut self, value: u8) -> Result<(), String> {
        sleep_ms(2); // the control unit needs a short pause between bytes
        if !self.stream.write_byte(value) {
            return Err(l("Senden fehlgeschlagen", "Send failed").to_string());
        }
        self.read_echo(value)?;

        let Some(ack) = self.stream.read_byte(BYTE_TIMEOUT_MS) else {
            return Err(format!(
                "{}{}{}",
                l(
                    "Steuergerät quittiert ",
                    "Control unit does not acknowledge "
                ),
                hex(value),
                l(" nicht", "")
            ));
        };
        let expected = !value;
        if ack != expected {
            return Err(format!(
                "{}{}{}{}",
                l(
                    "Falsche Quittung: erwartet ",
                    "Wrong acknowledgement: expected "
                ),
                hex(expected),
                l(", bekommen ", ", got "),
                hex(ack)
            ));
        }
        Ok(())
    }

    /// Receive a byte and acknowledge it with its complement.
    fn receive_byte_acked(&mut self, send_ack: bool) -> Result<u8, String> {
        let Some(value) = self.stream.read_byte(BYTE_TIMEOUT_MS) else {
            return Err(l(
                "Steuergerät antwortet nicht mehr",
                "Control unit stopped responding",
            )
            .to_string());
        };
        if !send_ack {
            return Ok(value);
        }

        let ack = !value;
        sleep_ms(2);
        if !self.stream.write_byte(ack) {
            return Err(l(
                "Quittung lässt sich nicht senden",
                "Cannot send acknowledgement",
            )
            .to_string());
        }
        self.read_echo(ack)?;
        Ok(value)
    }

    // --- Blocks -------------------------------------------------------------

    /// Layout: length, counter, title, payload, 0x03. The length counts
    /// everything except itself. The final 0x03 is not acknowledged.
    fn send_block(&mut self, title: u8, payload: &[u8]) -> Result<(), String> {
        let length = (payload.len() + 3) as u8;
        self.counter = self.counter.wrapping_add(1);

        self.log(&format!(
            "   -> Block {}{}{})",
            hex(title),
            l(" (Zähler ", " (counter "),
            hex(self.counter)
        ));

        self.send_byte_acked(length)?;
        self.send_byte_acked(self.counter)?;
        self.send_byte_acked(title)?;
        for &byte in payload {
            self.send_byte_acked(byte)?;
        }

        sleep_ms(2);
        if !self.stream.write_byte(BLOCK_END) {
            return Err(
                l("Blockende lässt sich nicht senden", "Cannot send block end").to_string(),
            );
        }
        self.read_echo(BLOCK_END)
    }

    fn receive_block(&mut self) -> Result<(u8, Vec<u8>), String> {
        let length = self.receive_byte_acked(true)?;
        if length < 3 {
            return Err(format!(
                "{}{}",
                l("Unsinnige Blocklänge ", "Implausible block length "),
                hex(length)
            ));
        }

        self.counter = self.receive_byte_acked(true)?;
        let title = self.receive_byte_acked(true)?;

        let data_bytes = usize::from(length - 3);
        let mut payload = Vec::with_capacity(data_bytes);
        for _ in 0..data_bytes {
            payload.push(self.receive_byte_acked(true)?);
        }

        let end = self.receive_byte_acked(false)?;
        if end != BLOCK_END {
            return Err(format!(
                "{}{}",
                l("Blockende fehlt, stattdessen ", "Block end missing, got "),
                hex(end)
            ));
        }
        self.log(&format!(
            "   <- Block {}, {}{}",
            hex(title),
            data_bytes,
            l(" Bytes", " bytes")
        ));
        Ok((title, payload))
    }

    // --- Connection ---------------------------------------------------------

    /// Wakes the control unit and reads its identification.
    pub fn connect(&mut self, ecu: Ecu) -> Result<(), String> {
        self.ecu = ecu;
        self.identification.clear();
        self.counter = 0;
        self.connected = false;

        if !self.stream.slow_init(ecu.address()) {
            return Err(format!(
                "{}{}{}",
                l("Steuergerät ", "Control unit "),
                ecu_name(ecu),
                l(
                    " meldet sich nicht.\n  - Zündung an?\n  - Hat das Fahrzeug dieses Steuergerät überhaupt?\n  - Bei neueren Fahrzeugen läuft die Diagnose über CAN, nicht über K-Line.",
                    " does not respond.\n  - Ignition on?\n  - Does the vehicle have this control unit at all?\n  - Newer vehicles are diagnosed over CAN, not over K-Line."
                )
            ));
        }

        // After the wake-up address come the sync byte and two key bytes.
        let sync = self.stream.read_byte(2000);
        if sync != Some(0x55) {
            return Err(format!(
                "{}{})",
                l(
                    "Kein Synchronbyte 0x55 vom Steuergerät (bekommen ",
                    "No sync byte 0x55 from the control unit (got "
                ),
                hex(sync.unwrap_or(0))
            ));
        }
        let Some((key1, key2)) = read_key_bytes(&mut self.stream) else {
            return Err(l("Schlüsselbytes fehlen", "Key bytes missing").to_string());
        };
        self.log(&format!(
            "{}{} {}",
            l("   Schlüsselbytes ", "   Key bytes "),
            hex(key1),
            hex(key2)
        ));

        // The second key byte is sent back inverted.
        sleep_ms(30);
        if !self.stream.write_byte(!key2) {
            return Err(l(
                "Antwort auf die Schlüsselbytes lässt sich nicht senden",
                "Cannot send the reply to the key bytes",
            )
            .to_string());
        }
        self.read_echo(!key2)?;

        self.connected = true;
        if let Err(error) = self.drain_identification() {
            self.connected = false;
            return Err(error);
        }
        Ok(())
    }

    /// Like `connect`, but the caller has already done the wake-up. Needed
    /// when only the key bytes reveal which protocol the control unit speaks.
    pub fn attach(&mut self) -> Result<(), String> {
        self.identification.clear();
        self.counter = 0;
        self.connected = true;
        if let Err(error) = self.drain_identification() {
            self.connected = false;
            return Err(error);
        }
        Ok(())
    }

    /// Right after connecting, the control unit sends its identification as
    /// ASCII blocks until it signals completion with an acknowledgement.
    fn drain_identification(&mut self) -> Result<(), String> {
        for _ in 0..20 {
            let (title, payload) = self.receive_block()?;

            if title == block::ASCII_DATA {
                let text: String = payload.iter().map(|&b| (b & 0x7F) as char).collect();
                self.identification.push(text);
                self.send_block(block::ACK, &[])?;
                continue;
            }
            if title == block::ACK {
                return Ok(());
            }
            // Something unexpected: acknowledge anyway and carry on.
            self.send_block(block::ACK, &[])?;
        }
        Err(l(
            "Das Steuergerät hört nicht auf, sich vorzustellen",
            "The control unit keeps sending identification blocks",
        )
        .to_string())
    }

    pub fn disconnect(&mut self) {
        if !self.connected {
            return;
        }
        let _ = self.send_block(block::END_OUTPUT, &[]);
        self.connected = false;
        self.counter = 0;
    }

    // --- Services -----------------------------------------------------------

    pub fn read_faults(&mut self) -> Result<Vec<Kwp1281Fault>, String> {
        if !self.connected {
            return Err(not_connected());
        }
        self.send_block(block::READ_FAULTS, &[])?;

        let mut out = Vec::new();
        for _ in 0..20 {
            let (title, payload) = self.receive_block()?;

            if title == block::FAULT_LIST {
                // Three bytes per fault: two for the number, one for the
                // status.
                for entry in payload.as_chunks::<3>().0 {
                    let code = (i32::from(entry[0]) << 8) | i32::from(entry[1]);
                    if code == 0xFFFF || code == 0 {
                        continue;
                    }
                    out.push(Kwp1281Fault {
                        code,
                        status: entry[2],
                        sporadic: (entry[2] & 0x80) != 0,
                        text: describe_vag_fault(code, i32::from(self.ecu.address())),
                    });
                }
                self.send_block(block::ACK, &[])?;
                continue;
            }
            if title == block::ACK {
                return Ok(out);
            }
            if title == block::NOT_SUPPORTED {
                return Err(l(
                    "Dieses Steuergerät kennt den Dienst nicht",
                    "This control unit does not support the service",
                )
                .to_string());
            }
            self.send_block(block::ACK, &[])?;
        }
        Err(l(
            "Zu viele Blöcke beim Lesen des Fehlerspeichers",
            "Too many blocks while reading the fault memory",
        )
        .to_string())
    }

    pub fn clear_faults(&mut self) -> Result<(), String> {
        if !self.connected {
            return Err(not_connected());
        }
        self.send_block(block::CLEAR_FAULTS, &[])?;

        let (title, _) = self.receive_block()?;
        if title == block::ACK {
            return Ok(());
        }
        if title == block::FAULT_LIST {
            // Some control units send back the emptied list.
            return self.send_block(block::ACK, &[]);
        }
        Err(format!(
            "{}{}{}",
            l("Unerwartete Antwort ", "Unexpected response "),
            hex(title),
            l(" auf das Löschen", " to the clear request")
        ))
    }

    /// Read a measuring block, returns up to four values.
    pub fn read_group(&mut self, group: u8) -> Result<Vec<MeasuringValue>, String> {
        if !self.connected {
            return Err(not_connected());
        }
        self.send_block(block::READ_GROUP, &[group])?;

        let (title, payload) = self.receive_block()?;

        if title == block::NOT_SUPPORTED {
            return Err(format!(
                "{}{}{}",
                l("Messwertblock ", "Measuring block "),
                group,
                l(" gibt es hier nicht", " does not exist here")
            ));
        }
        if title != block::GROUP_READING {
            return Err(format!(
                "{}{}{}",
                l("Unerwartete Antwort ", "Unexpected response "),
                hex(title),
                l(" auf den Messwertblock", " to the measuring block")
            ));
        }
        // Three bytes per display field: formula, value A, value B.
        Ok(payload
            .as_chunks::<3>()
            .0
            .iter()
            .map(|f| decode_measurement(f[0], f[1], f[2]))
            .collect())
    }

    /// Basic settings: the same block as a measuring block, but the control
    /// unit actively regulates: with block 060 or 098 it drives the throttle
    /// body to its end stops and stores the values.
    pub fn basic_setting(&mut self, group: u8) -> Result<Vec<MeasuringValue>, String> {
        if !self.connected {
            return Err(not_connected());
        }
        self.send_block(block::BASIC_SETTING, &[group])?;

        let (title, payload) = self.receive_block()?;

        if title == block::NOT_SUPPORTED {
            return Err(format!(
                "{}{}{}",
                l("Grundeinstellung ", "Basic settings "),
                group,
                l(
                    " kennt dieses Steuergerät nicht",
                    " are not supported by this control unit"
                )
            ));
        }
        if title != block::GROUP_READING {
            return Err(format!(
                "{}{}{}",
                l("Unerwartete Antwort ", "Unexpected response "),
                hex(title),
                l(" auf die Grundeinstellung", " to the basic settings")
            ));
        }
        Ok(payload
            .as_chunks::<3>()
            .0
            .iter()
            .map(|f| decode_measurement(f[0], f[1], f[2]))
            .collect())
    }

    /// Read an adaptation channel: the stored value and the live values the
    /// control unit displays alongside it.
    pub fn read_adaptation(&mut self, channel: u8) -> Result<(i32, Vec<MeasuringValue>), String> {
        if !self.connected {
            return Err(not_connected());
        }
        self.send_block(block::READ_ADAPTATION, &[channel])?;

        let (title, payload) = self.receive_block()?;

        if title == block::NOT_SUPPORTED {
            return Err(format!(
                "{}{}{}",
                l("Anpassungskanal ", "Adaptation channel "),
                channel,
                l(" gibt es hier nicht", " does not exist here")
            ));
        }
        if title != block::ADAPTATION_VALUE && title != block::GROUP_READING {
            return Err(format!(
                "{}{}{}",
                l("Unerwartete Antwort ", "Unexpected response "),
                hex(title),
                l(" auf den Anpassungskanal", " to the adaptation channel")
            ));
        }
        // First the four display fields, then two bytes with the channel value.
        let mut shown = Vec::new();
        let mut i = 0;
        while i + 2 < payload.len() && shown.len() < 4 {
            shown.push(decode_measurement(
                payload[i],
                payload[i + 1],
                payload[i + 2],
            ));
            i += 3;
        }
        let mut value = 0;
        if i + 1 < payload.len() {
            value = (i32::from(payload[i]) << 8) | i32::from(payload[i + 1]);
        }
        Ok((value, shown))
    }

    /// Output test: the control unit cycles through its actuators one by one
    /// and reports the one that is currently active.
    pub fn actuator_test(&mut self) -> Result<String, String> {
        if !self.connected {
            return Err(not_connected());
        }
        self.send_block(block::ACTUATOR_TEST, &[])?;

        let (title, payload) = self.receive_block()?;

        if title == block::NOT_SUPPORTED {
            return Err(l(
                "Stellglieddiagnose kennt dieses Steuergerät nicht",
                "This control unit does not support the output test",
            )
            .to_string());
        }
        if title == block::ACK {
            return Ok(l("Ende der Stellglieddiagnose", "End of output test").to_string());
        }
        // The control unit names the active actuator either as text or as a
        // two-byte number.
        if title == block::ASCII_DATA {
            return Ok(payload.iter().map(|&b| (b & 0x7F) as char).collect());
        }
        if payload.len() >= 2 {
            let number = (i32::from(payload[0]) << 8) | i32::from(payload[1]);
            return Ok(format!("{}{}", l("Stellglied ", "Actuator "), number));
        }
        Ok(l("unbekanntes Stellglied", "unknown actuator").to_string())
    }

    /// Keeps the connection alive while nothing is being requested. Without
    /// regular traffic the connection drops after about one second. `Err`
    /// with an empty text when not connected.
    pub fn keep_alive(&mut self) -> Result<(), String> {
        if !self.connected {
            return Err(String::new());
        }
        self.send_block(block::ACK, &[])?;
        self.receive_block().map(|_| ())
    }
}

// --- Measuring value formulas -------------------------------------------------

/// Translate the raw bytes of a measuring block into a number and unit.
///
/// VAG encodes every value with a formula number and two bytes. These are the
/// formulas that actually occur on engine control units.
pub fn decode_measurement(formula: u8, a: u8, b: u8) -> MeasuringValue {
    let da = f64::from(a);
    let db = f64::from(b);
    let (value, unit, label): (f64, &str, String) = match formula {
        0x00 => {
            return MeasuringValue {
                label: l("leer", "empty").to_string(),
                value: 0.0,
                unit: String::new(),
                valid: false,
            };
        }
        0x01 => (0.2 * da * db, "1/min", l("Drehzahl", "Engine speed").into()),
        0x02 => (da * 0.002 * db, "%", l("Anteil", "Ratio").into()),
        0x03 => (
            0.002 * da * db,
            l("Grad", "deg"),
            l("Winkel", "Angle").into(),
        ),
        0x04 => (
            (db - 127.0).abs() * 0.01 * da,
            l("Grad v.OT", "deg BTDC"),
            l("Zündwinkel", "Ignition timing").into(),
        ),
        0x05 => (
            da * (db - 100.0) * 0.1,
            l("Grad C", "deg C"),
            l("Temperatur", "Temperature").into(),
        ),
        0x06 => (0.001 * da * db, "V", l("Spannung", "Voltage").into()),
        0x07 => (0.01 * da * db, "km/h", l("Geschwindigkeit", "Speed").into()),
        0x08 => (0.1 * da * db, "", l("Wert", "Value").into()),
        0x09 => (
            (db - 127.0) * 0.02 * da,
            l("Grad", "deg"),
            l("Winkel", "Angle").into(),
        ),
        0x0A => (
            db,
            "",
            if b == 0 {
                l("kalt", "cold")
            } else {
                l("warm", "warm")
            }
            .into(),
        ),
        0x0B => (
            (db - 128.0) * 0.0001 * da + 1.0,
            "",
            l("Faktor", "Factor").into(),
        ),
        0x0C => (0.001 * da * db, "Ohm", l("Widerstand", "Resistance").into()),
        0x0D => ((db - 127.0) * 0.001 * da, "mm", l("Weg", "Travel").into()),
        0x0E => (0.005 * da * db, "bar", l("Druck", "Pressure").into()),
        0x0F => (0.01 * da * db, "ms", l("Zeit", "Time").into()),
        0x12 => (0.04 * da * db, "mbar", l("Druck", "Pressure").into()),
        0x13 => (0.01 * da * db, "l", l("Menge", "Quantity").into()),
        0x14 => (
            da * (db - 128.0) / 128.0,
            "%",
            l("Anpassung", "Adaptation").into(),
        ),
        0x15 => (0.001 * da * db, "V", l("Spannung", "Voltage").into()),
        0x16 => (0.001 * da * db, "ms", l("Zeit", "Time").into()),
        0x17 => (db / 256.0 * da, "%", l("Anteil", "Ratio").into()),
        0x19 => (
            da * 1.421 + db / 182.0,
            "g/s",
            l("Luftmasse", "Air mass").into(),
        ),
        0x1A => (
            db - da,
            l("Grad C", "deg C"),
            l("Temperaturdifferenz", "Temperature difference").into(),
        ),
        0x1B => (
            (db - 128.0).abs() * 0.01 * da,
            l("Grad v.OT", "deg BTDC"),
            l("Winkel", "Angle").into(),
        ),
        0x1C => (db - da, "", l("Differenz", "Difference").into()),
        0x1E => (da * db / 12.0, "km/h", l("Geschwindigkeit", "Speed").into()),
        0x1F => (da * db / 2560.0, "", l("Wert", "Value").into()),
        0x21 => (
            if a == 0 {
                100.0 * db / 255.0
            } else {
                100.0 * db / da
            },
            "%",
            l("Anteil", "Ratio").into(),
        ),
        0x22 => (
            (da * db) / 100.0 - 400.0,
            "kW",
            l("Leistung", "Power").into(),
        ),
        0x23 => (da * db / 100.0, "l/h", l("Verbrauch", "Consumption").into()),
        0x24 => (
            da * 2560.0 + db * 10.0,
            "km",
            l("Strecke", "Distance").into(),
        ),
        0x25 => (db, "", l("Bitmuster", "Bit pattern").into()),
        0x27 => (
            db / da,
            "mg/H",
            l("Einspritzmenge", "Injection quantity").into(),
        ),
        0x31 => (db / 40.0 * da, "mg/h", l("Menge", "Quantity").into()),
        0x33 => (
            (db - 128.0) * da / 255.0,
            "mg/H",
            l("Menge", "Quantity").into(),
        ),
        0x36 => (da * 256.0 + db, "", l("Zähler", "Counter").into()),
        0x37 => (da * db / 200.0, "s", l("Zeit", "Time").into()),
        _ => (
            da * 256.0 + db,
            "",
            format!("{}{}", l("Formel ", "Formula "), hex(formula)),
        ),
    };
    MeasuringValue {
        label,
        value,
        unit: unit.to_string(),
        valid: true,
    }
}
