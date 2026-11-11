//! KWP2000 over VAG TP 2.0 on CAN: the control units of VW, Audi, Seat and
//! Skoda from about 2004 to 2012. The services are the ones of the K-line
//! ([`crate::kwp2000::Kwp2000`]): identification, fault memory, measuring
//! blocks and basic settings; only the way the messages travel differs. The
//! transport (`caruspex_net::Tp20` on a raw CAN interface) comes in as a
//! [`MessageLink`], so the tests can hand in a simulated one.

use caruspex_i18n::l;
use caruspex_link::MessageLink;

use crate::kwp1281::{Ecu, KLineLog, Kwp1281Fault, MeasuringValue, ecu_name};
use crate::kwp2000::{
    KLineProtocol, faults_from_reply, identification_text, negative_response_text, service,
    session, values_from_reply,
};
use crate::vag_kline::VagDiagnosis;

/// The first answer of a request.
const ANSWER_MS: u64 = 1000;
/// After 7F xx 78 (response pending).
const PENDING_MS: u64 = 5000;

fn hex2(value: u8) -> String {
    format!("{value:02X}")
}

fn not_connected() -> String {
    l("Nicht verbunden", "Not connected").to_string()
}

/// The protocol name shown in the interface.
pub fn tp20_label() -> &'static str {
    "KWP2000 (TP 2.0)"
}

/// KWP2000 over TP 2.0. Owns the transport; dropping it closes the channel.
pub struct VagTp20<L: MessageLink> {
    link: L,
    log: Option<KLineLog>,
    ecu: Ecu,
    identification: Vec<String>,
    started: bool,
    basic_setting_session: u8,
}

impl<L: MessageLink> VagTp20<L> {
    pub fn new(link: L, log: Option<KLineLog>) -> Self {
        Self {
            link,
            log,
            ecu: Ecu::Engine,
            identification: Vec::new(),
            started: false,
            basic_setting_session: session::ADAPTATION,
        }
    }

    pub fn link(&self) -> &L {
        &self.link
    }

    pub fn link_mut(&mut self) -> &mut L {
        &mut self.link
    }

    fn log(&self, line: &str) {
        if let Some(log) = &self.log {
            log(line);
        }
    }

    /// Raw service for everything not covered here. The positive response,
    /// service byte first.
    pub fn request(&mut self, payload: &[u8]) -> Result<Vec<u8>, String> {
        if payload.is_empty() {
            return Err(l(
                "Nachricht hat eine unmögliche Länge",
                "Message has an impossible length",
            )
            .to_string());
        }
        if self.log.is_some() {
            let text: String = payload.iter().map(|&b| hex2(b) + " ").collect();
            self.log(&format!("   -> {text}"));
        }
        self.link.send_message(payload)?;
        let mut timeout = ANSWER_MS;
        for _ in 0..10 {
            let reply = self.link.receive_message(timeout)?;
            if self.log.is_some() {
                let text: String = reply.iter().map(|&b| hex2(b) + " ").collect();
                self.log(&format!("   <- {text}"));
            }
            if reply.len() >= 3 && reply[0] == service::NEGATIVE {
                if reply[2] == 0x78 {
                    timeout = PENDING_MS; // control unit still working
                    continue;
                }
                return Err(negative_response_text(reply[2]));
            }
            if reply.first().map(|&b| u32::from(b)) == Some(u32::from(payload[0]) + 0x40) {
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

    fn need_session(&self) -> Result<(), String> {
        if self.started && self.link.channel_open() {
            Ok(())
        } else {
            Err(not_connected())
        }
    }
}

impl<L: MessageLink> VagDiagnosis for VagTp20<L> {
    fn connect(&mut self, ecu: Ecu) -> Result<(), String> {
        self.disconnect();
        self.ecu = ecu;
        self.identification.clear();
        self.log(&format!(
            "{}{}",
            l(
                "TP 2.0: öffne den Kanal zu ",
                "TP 2.0: opening the channel to "
            ),
            ecu_name(ecu)
        ));
        self.link.open_channel(ecu.address())?;
        if let Err(error) = self.request(&[service::START_DIAGNOSTIC_SESSION, session::DIAGNOSTICS])
        {
            self.link.close_channel();
            return Err(error);
        }
        self.started = true;
        // 0x9B returns part number and description as readable text.
        if let Ok(reply) = self.request(&[service::READ_ECU_IDENTIFICATION, 0x9B]) {
            let text = identification_text(&reply);
            if !text.is_empty() {
                self.identification.push(text);
            }
        }
        Ok(())
    }

    fn disconnect(&mut self) {
        self.started = false;
        self.link.close_channel();
    }

    fn connected(&self) -> bool {
        self.started && self.link.channel_open()
    }

    fn protocol(&self) -> KLineProtocol {
        if self.connected() {
            KLineProtocol::Kwp2000
        } else {
            KLineProtocol::Unknown
        }
    }

    fn protocol_name(&self) -> &'static str {
        tp20_label()
    }

    fn identification(&self) -> Vec<String> {
        self.identification.clone()
    }

    fn read_faults(&mut self) -> Result<Vec<Kwp1281Fault>, String> {
        self.need_session()?;
        // 0xFF 0x00 means: all faults, regardless of status.
        let reply = self.request(&[service::READ_DTC_BY_STATUS, 0x02, 0xFF, 0x00])?;
        Ok(faults_from_reply(&reply, self.ecu.address()))
    }

    fn clear_faults(&mut self) -> Result<(), String> {
        self.need_session()?;
        self.request(&[service::CLEAR_DIAGNOSTIC_INFORMATION, 0xFF, 0x00])
            .map(|_| ())
    }

    fn read_group(&mut self, group: u8) -> Result<Vec<MeasuringValue>, String> {
        self.need_session()?;
        let reply = self.request(&[service::READ_DATA_BY_LOCAL_ID, group])?;
        Ok(values_from_reply(&reply))
    }

    fn basic_setting(&mut self, group: u8) -> Result<Vec<MeasuringValue>, String> {
        self.need_session()?;
        // As on the K-line: another session type, then the same measuring
        // block, and the control unit regulates actively.
        let mode = self.basic_setting_session;
        if let Err(error) = self.request(&[service::START_DIAGNOSTIC_SESSION, mode]) {
            return Err(format!(
                "{}{}{}{}",
                l("Sitzungsart 0x", "Session type 0x"),
                hex2(mode),
                l(" abgelehnt: ", " rejected: "),
                error
            ));
        }
        self.read_group(group)
    }

    fn keep_alive(&mut self) -> Result<(), String> {
        self.need_session()?;
        // The channel test keeps TP 2.0 open, TesterPresent the session.
        self.link.keep_alive()?;
        self.request(&[service::TESTER_PRESENT, 0x01]).map(|_| ())
    }

    fn read_adaptation(&mut self, _channel: u8) -> Result<(i32, Vec<MeasuringValue>), String> {
        Err(l(
            "Anpassungskanäle lesen ist bisher nur für KWP1281 umgesetzt. Dieses Steuergerät spricht KWP2000.",
            "Reading adaptation channels is only implemented for KWP1281 so far. This control unit speaks KWP2000.",
        )
        .to_string())
    }

    fn actuator_test(&mut self) -> Result<String, String> {
        Err(l(
            "Die Stellglieddiagnose ist bisher nur für KWP1281 umgesetzt. Dieses Steuergerät spricht KWP2000.",
            "The output test is only implemented for KWP1281 so far. This control unit speaks KWP2000.",
        )
        .to_string())
    }

    fn set_basic_setting_session(&mut self, mode: u8) {
        self.basic_setting_session = mode;
    }
}

impl<L: MessageLink> Drop for VagTp20<L> {
    fn drop(&mut self) {
        self.disconnect();
    }
}
