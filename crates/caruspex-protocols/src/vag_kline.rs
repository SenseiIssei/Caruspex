//! A single entry point to both VAG protocols on the K-line.
//!
//! After the wake-up address, the key bytes reveal whether the control unit
//! speaks the old block protocol KWP1281 or the newer KWP2000. The Audi A2
//! sits right on this boundary: the 1.4 with port injection mostly still
//! KWP1281, the 1.6 FSI with its Bosch MED mostly already KWP2000. Whoever
//! operates the software should not have to know this.

use caruspex_i18n::l;
use caruspex_link::ByteStream;

use crate::kwp1281::{Ecu, KLineLog, Kwp1281, Kwp1281Fault, MeasuringValue};
use crate::kwp2000::{KLineProtocol, Kwp2000, protocol_label, session, wake_up_kline};

/// Who holds the stream right now.
enum Link<S: ByteStream> {
    Idle(S),
    Old(Kwp1281<S>),
    Modern(Kwp2000<S>),
}

fn not_connected() -> String {
    l("Nicht verbunden", "Not connected").to_string()
}

/// Both VAG K-line protocols behind one interface. Owns the byte stream;
/// dropping it signs off from the control unit, [`VagKLine::into_stream`]
/// does the same and hands the stream back.
pub struct VagKLine<S: ByteStream> {
    // Always Some, except while a protocol is being swapped and after
    // into_stream.
    link: Option<Link<S>>,
    log: Option<KLineLog>,
    protocol: KLineProtocol,
    basic_setting_session: u8,
}

impl<S: ByteStream> VagKLine<S> {
    pub fn new(stream: S, log: Option<KLineLog>) -> Self {
        Self {
            link: Some(Link::Idle(stream)),
            log,
            protocol: KLineProtocol::Unknown,
            basic_setting_session: session::ADAPTATION,
        }
    }

    pub fn stream(&self) -> &S {
        match self.link.as_ref().expect("stream present") {
            Link::Idle(s) => s,
            Link::Old(k) => k.stream(),
            Link::Modern(k) => k.stream(),
        }
    }

    pub fn stream_mut(&mut self) -> &mut S {
        match self.link.as_mut().expect("stream present") {
            Link::Idle(s) => s,
            Link::Old(k) => k.stream_mut(),
            Link::Modern(k) => k.stream_mut(),
        }
    }

    /// Signs off and hands the stream back.
    pub fn into_stream(mut self) -> S {
        self.disconnect();
        match self.link.take().expect("stream present") {
            Link::Idle(s) => s,
            Link::Old(k) => k.into_stream(),
            Link::Modern(k) => k.into_stream(),
        }
    }

    fn take_stream(&mut self) -> S {
        match self.link.take().expect("stream present") {
            Link::Idle(s) => s,
            Link::Old(k) => k.into_stream(),
            Link::Modern(k) => k.into_stream(),
        }
    }

    pub fn connect(&mut self, ecu: Ecu) -> Result<(), String> {
        self.disconnect();

        let mut stream = self.take_stream();
        let (key1, key2, protocol) = match wake_up_kline(&mut stream, ecu, self.log.as_ref()) {
            Ok(found) => found,
            Err(error) => {
                self.link = Some(Link::Idle(stream));
                self.protocol = KLineProtocol::Unknown;
                return Err(error);
            }
        };
        self.protocol = protocol;

        if protocol == KLineProtocol::Kwp1281 {
            let mut old = Kwp1281::new(stream, self.log.clone());
            old.set_ecu(ecu);
            if let Err(error) = old.attach() {
                self.link = Some(Link::Idle(old.into_stream()));
                return Err(error);
            }
            self.link = Some(Link::Old(old));
            return Ok(());
        }

        let mut modern = Kwp2000::new(stream, self.log.clone());
        if let Err(error) = modern.start(ecu, key1, key2) {
            self.link = Some(Link::Idle(modern.into_stream()));
            return Err(error);
        }
        self.link = Some(Link::Modern(modern));
        Ok(())
    }

    pub fn disconnect(&mut self) {
        match self.link.take() {
            Some(Link::Old(mut old)) => {
                old.disconnect();
                self.link = Some(Link::Idle(old.into_stream()));
            }
            Some(Link::Modern(mut modern)) => {
                modern.stop();
                self.link = Some(Link::Idle(modern.into_stream()));
            }
            other => self.link = other,
        }
        self.protocol = KLineProtocol::Unknown;
    }

    pub fn connected(&self) -> bool {
        match &self.link {
            Some(Link::Old(old)) => old.connected(),
            Some(Link::Modern(modern)) => modern.started(),
            _ => false,
        }
    }

    pub fn protocol(&self) -> KLineProtocol {
        self.protocol
    }

    pub fn protocol_name(&self) -> &'static str {
        protocol_label(self.protocol)
    }

    pub fn identification(&self) -> Vec<String> {
        match &self.link {
            Some(Link::Old(old)) => old.identification().to_vec(),
            Some(Link::Modern(modern)) => modern.identification().to_vec(),
            _ => Vec::new(),
        }
    }

    // --- Supported by both protocols ----------------------------------------

    pub fn read_faults(&mut self) -> Result<Vec<Kwp1281Fault>, String> {
        match &mut self.link {
            Some(Link::Old(old)) => old.read_faults(),
            Some(Link::Modern(modern)) => modern.read_faults(),
            _ => Err(not_connected()),
        }
    }

    pub fn clear_faults(&mut self) -> Result<(), String> {
        match &mut self.link {
            Some(Link::Old(old)) => old.clear_faults(),
            Some(Link::Modern(modern)) => modern.clear_faults(),
            _ => Err(not_connected()),
        }
    }

    pub fn read_group(&mut self, group: u8) -> Result<Vec<MeasuringValue>, String> {
        match &mut self.link {
            Some(Link::Old(old)) => old.read_group(group),
            Some(Link::Modern(modern)) => modern.read_group(group),
            _ => Err(not_connected()),
        }
    }

    pub fn basic_setting(&mut self, group: u8) -> Result<Vec<MeasuringValue>, String> {
        let session = self.basic_setting_session;
        match &mut self.link {
            Some(Link::Old(old)) => old.basic_setting(group),
            Some(Link::Modern(modern)) => modern.basic_setting(group, session),
            _ => Err(not_connected()),
        }
    }

    pub fn keep_alive(&mut self) -> Result<(), String> {
        match &mut self.link {
            Some(Link::Old(old)) => old.keep_alive(),
            Some(Link::Modern(modern)) => modern.tester_present(),
            _ => Err(not_connected()),
        }
    }

    // --- Only KWP1281 knows these two. With KWP2000 they report that
    // cleanly. ---------------------------------------------------------------

    pub fn read_adaptation(&mut self, channel: u8) -> Result<(i32, Vec<MeasuringValue>), String> {
        match &mut self.link {
            Some(Link::Old(old)) => old.read_adaptation(channel),
            Some(Link::Modern(_)) => Err(l(
                "Anpassungskanäle lesen ist bisher nur für KWP1281 umgesetzt. Dieses Steuergerät spricht KWP2000.",
                "Reading adaptation channels is only implemented for KWP1281 so far. This control unit speaks KWP2000.",
            )
            .to_string()),
            _ => Err(not_connected()),
        }
    }

    pub fn actuator_test(&mut self) -> Result<String, String> {
        match &mut self.link {
            Some(Link::Old(old)) => old.actuator_test(),
            Some(Link::Modern(_)) => Err(l(
                "Die Stellglieddiagnose ist bisher nur für KWP1281 umgesetzt. Dieses Steuergerät spricht KWP2000.",
                "The output test is only implemented for KWP1281 so far. This control unit speaks KWP2000.",
            )
            .to_string()),
            _ => Err(not_connected()),
        }
    }

    /// KWP2000 only: session type for basic settings. Which one a given
    /// control unit requires is not uniform.
    pub fn set_basic_setting_session(&mut self, mode: u8) {
        self.basic_setting_session = mode;
    }

    pub fn basic_setting_session(&self) -> u8 {
        self.basic_setting_session
    }
}

impl<S: ByteStream> Drop for VagKLine<S> {
    fn drop(&mut self) {
        self.disconnect();
    }
}
