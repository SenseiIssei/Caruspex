//! Reading custom values from the car: sessions, headers and the answers of
//! service 21 and 22, and the values that are listened for.

use std::collections::{BTreeMap, HashMap};

use super::csv::parse_request_hex;
use super::equation::TorqueEquation;
use caruspex_i18n::l;
use caruspex_link::Elm327;
use caruspex_net::can_signals::{
    CanSignal, decode_can_signal, parse_can_signal, parse_monitor_request,
};
use caruspex_protocols::{LiveValue, Uds, UdsRefusal, ecu_display_name};

use super::CustomPid;
use super::csv::default_response;
use crate::session::ExtendedSessions;

const REQUEST_TIMEOUT_MS: u64 = 2000;

/// Broadcast values are listened for, not asked ("CAN132").
fn is_monitored(pid: &CustomPid) -> bool {
    parse_monitor_request(&pid.request).is_some()
}

/// Where to ask, as the adapter sees it. Empty for the default addressing.
fn address_key(pid: &CustomPid) -> String {
    if is_monitored(pid) {
        return "listen".to_string();
    }
    if pid.header.is_empty() {
        return String::new();
    }
    let response = if pid.response.is_empty() {
        default_response(&pid.header)
    } else {
        pid.response.clone()
    };
    format!("{}/{response}", pid.header)
}

/// One answer: the data bytes after the echo, or why there are none.
type Answer = Result<Vec<u8>, String>;

/// Result of one value in [`CustomPidReader::read_all`]. The value is filled
/// as far as the definition goes, also when reading failed.
#[derive(Clone, Debug, Default)]
pub struct CustomReading {
    pub ok: bool,
    pub value: LiveValue,
    pub error: String,
    /// Read with the control unit in the extended diagnostic session.
    pub session: bool,
}

/// Why a value that needs the extended session was not read, when the
/// caller keeps no sessions (the command line).
fn no_session_reason() -> String {
    l(
        "Dieser Wert braucht die erweiterte Diagnosesitzung des Steuergeräts. Die liest nur die Live-Ansicht der App.",
        "This value needs the extended diagnostic session of the control unit. Only the live view of the app reads it.",
    )
    .to_string()
}

/// The control unit a value is asked at.
fn ecu_definition(pid: &CustomPid) -> Result<caruspex_data::ecu_addresses::EcuDefinition, String> {
    let invalid = || l("Ungültige Adresse.", "Invalid address.").to_string();
    let response = if pid.response.is_empty() {
        default_response(&pid.header)
    } else {
        pid.response.clone()
    };
    Ok(caruspex_data::ecu_addresses::EcuDefinition {
        request: u32::from_str_radix(&pid.header, 16).map_err(|_| invalid())?,
        response: u32::from_str_radix(&response, 16).map_err(|_| invalid())?,
        extended29: pid.header.len() > 3,
        name: format!("ECU {}", pid.header),
        ..Default::default()
    })
}

/// Reads custom values over and over, as the live view does. It remembers
/// which control unit the adapter is pointed at, so consecutive values of the
/// same header cost no extra AT commands, and it parses every equation only
/// once. Dropping it puts the adapter back on the default OBD addressing.
pub struct CustomPidReader<'a> {
    uds: Uds<'a>,
    addressed: bool,
    /// Address key the adapter points at.
    current: String,
    equations: HashMap<String, TorqueEquation>,
    signals: HashMap<String, CanSignal>,
}

impl<'a> CustomPidReader<'a> {
    pub fn new(elm: &'a mut Elm327) -> Self {
        Self {
            uds: Uds::new(elm),
            addressed: false,
            current: String::new(),
            equations: HashMap::new(),
            signals: HashMap::new(),
        }
    }

    /// Back to the default OBD addressing now (also done when dropped).
    pub fn restore_default(&mut self) {
        if self.addressed {
            self.uds.restore_default();
        }
        self.addressed = false;
        self.current.clear();
    }

    fn address(&mut self, pid: &CustomPid) -> Result<(), String> {
        let key = address_key(pid);
        if key.is_empty() {
            self.restore_default();
            return Ok(());
        }
        if self.addressed && key == self.current {
            return Ok(());
        }
        let def = ecu_definition(pid)?;
        self.addressed = false;
        self.current.clear();
        self.uds.select(&def)?; // select puts the default back itself
        self.addressed = true;
        self.current = key;
        Ok(())
    }

    fn ask(&mut self, pid: &CustomPid) -> Answer {
        let request = match parse_request_hex(&pid.request) {
            Some(r) if r.len() >= 2 => r,
            _ => return Err(l("Ungültige Anfrage.", "Invalid request.").to_string()),
        };
        self.address(pid)?;
        let response = self.uds.request(&request, REQUEST_TIMEOUT_MS)?;
        // The positive answer repeats everything after the service byte
        // (the identifier), then come the data bytes.
        let echo = request.len() - 1;
        if response.len() < echo || response[..echo] != request[1..] {
            return Err(l(
                "Die Antwort passt nicht zur Anfrage.",
                "The answer does not match the request.",
            )
            .to_string());
        }
        Ok(response[echo..].to_vec())
    }

    /// The same in the extended session: only the requests the allow list of
    /// [`Uds::session_request`] knows (22 here) go out.
    fn ask_session(&mut self, pid: &CustomPid) -> Result<Vec<u8>, UdsRefusal> {
        let refused = |message: String| UdsRefusal { nrc: None, message };
        let request = match parse_request_hex(&pid.request) {
            Some(r) if r.len() >= 2 => r,
            _ => {
                return Err(refused(
                    l("Ungültige Anfrage.", "Invalid request.").to_string(),
                ));
            }
        };
        self.address(pid).map_err(refused)?;
        let response = self.uds.session_request(&request, REQUEST_TIMEOUT_MS)?;
        let echo = request.len() - 1;
        if response.len() < echo || response[..echo] != request[1..] {
            return Err(refused(
                l(
                    "Die Antwort passt nicht zur Anfrage.",
                    "The answer does not match the request.",
                )
                .to_string(),
            ));
        }
        Ok(response[echo..].to_vec())
    }

    /// The frames of one id, for all values that need it. Multiplexed values
    /// need frames with every multiplexer value, so it listens a little
    /// longer for those.
    fn listen(&mut self, can_id: u32, multiplexed: bool) -> Vec<Vec<u8>> {
        self.restore_default();
        self.uds.elm().capture_frames(
            can_id,
            if multiplexed { 900 } else { 400 },
            if multiplexed { 40 } else { 6 },
        )
    }

    fn base_value(pid: &CustomPid) -> LiveValue {
        LiveValue {
            key: pid.key,
            name: pid.name.clone(),
            unit: pid.unit.clone(),
            category: pid.category,
            display_min: pid.min_value,
            display_max: pid.max_value,
            ..LiveValue::default()
        }
    }

    fn decode_monitored(
        &mut self,
        pid: &CustomPid,
        frames: &[Vec<u8>],
        out: &mut LiveValue,
    ) -> Result<(), String> {
        *out = Self::base_value(pid);
        let signal = match self.signals.get(&pid.equation) {
            Some(s) => *s,
            None => {
                let s = parse_can_signal(&pid.equation)?;
                self.signals.insert(pid.equation.clone(), s);
                s
            }
        };
        if frames.is_empty() {
            return Err(format!(
                "{}{}{}",
                l(
                    "Diese Nachricht ist auf dem Bus nicht zu hören (",
                    "This message is not heard on the bus ("
                ),
                pid.request,
                l(
                    "). Steckt das Kabel am richtigen Bus, und ist das Auto wach?",
                    "). Is the cable on the right bus, and is the car awake?"
                )
            ));
        }
        // The newest frame that carries the value.
        for frame in frames.iter().rev() {
            if let Some(v) = decode_can_signal(&signal, frame) {
                out.value = v;
                return Ok(());
            }
        }
        Err(l(
            "Der Wert kam in den gehörten Nachrichten nicht vor.",
            "The value was not in the frames heard.",
        )
        .to_string())
    }

    fn equation(&mut self, text: &str) -> TorqueEquation {
        self.equations
            .entry(text.to_string())
            .or_insert_with(|| TorqueEquation::new(text))
            .clone()
    }

    fn decode(
        &mut self,
        pid: &CustomPid,
        answer: &Answer,
        out: &mut LiveValue,
    ) -> Result<(), String> {
        *out = Self::base_value(pid);
        if !pid.header.is_empty() {
            out.ecu_source = if pid.response.is_empty() {
                default_response(&pid.header)
            } else {
                pid.response.clone()
            };
            out.ecu = ecu_display_name(&out.ecu_source);
        }
        let data = answer.as_ref().map_err(Clone::clone)?;
        out.value = self.equation(&pid.equation).evaluate(data)?;
        Ok(())
    }

    /// One value, the simple way. A value that needs the extended session
    /// is not read here, see [`Self::read_all_with`].
    pub fn read(&mut self, pid: &CustomPid) -> Result<LiveValue, String> {
        if pid.session != 0 {
            return Err(no_session_reason());
        }
        let mut out = LiveValue::default();
        if let Some(id) = parse_monitor_request(&pid.request) {
            let multiplexed = pid.equation.contains(" m") || pid.equation.contains(")m");
            let frames = self.listen(id, multiplexed);
            self.decode_monitored(pid, &frames, &mut out)?;
            return Ok(out);
        }
        let eq = self.equation(&pid.equation);
        if !eq.ok() {
            return Err(eq.error().to_string());
        }
        let answer = self.ask(pid);
        self.decode(pid, &answer, &mut out)?;
        Ok(out)
    }

    /// All values, in the order given. Values sharing a header and a request
    /// are served from one answer (the battery management of an EV puts some
    /// thirty values into one), and values of the same control unit are
    /// asked one after another to save re-addressing. Values that need the
    /// extended session are not read, see [`Self::read_all_with`].
    pub fn read_all(&mut self, pids: &[CustomPid]) -> Vec<CustomReading> {
        self.read_all_with(pids, None)
    }

    /// [`Self::read_all`], and the values that need the extended diagnostic
    /// session through `sessions`, which enters and keeps it for their
    /// control unit. While a unit is in that session every request to it
    /// goes through the allow list of [`Uds::session_request`].
    pub fn read_all_with(
        &mut self,
        pids: &[CustomPid],
        mut sessions: Option<&mut ExtendedSessions>,
    ) -> Vec<CustomReading> {
        let mut out: Vec<CustomReading> = vec![CustomReading::default(); pids.len()];

        // Group by control unit, in the order the units first appear; the
        // control unit the adapter already points at goes first.
        let mut order: Vec<String> = Vec::new();
        for pid in pids {
            let key = address_key(pid);
            if !order.contains(&key) {
                order.push(key);
            }
        }
        let current = if self.addressed {
            self.current.clone()
        } else {
            String::new()
        };
        if let Some(here) = order.iter().position(|k| *k == current) {
            order[..=here].rotate_right(1);
        }

        // Broadcast values: listen once per id for all values of that id.
        let mut heard: BTreeMap<u32, Vec<Vec<u8>>> = BTreeMap::new();
        for pid in pids {
            let Some(id) = parse_monitor_request(&pid.request) else {
                continue;
            };
            if heard.contains_key(&id) {
                continue;
            }
            let multiplexed = pids.iter().any(|other| {
                parse_monitor_request(&other.request) == Some(id)
                    && other
                        .equation
                        .as_bytes()
                        .get(4..)
                        .is_some_and(|rest| rest.contains(&b'm'))
            });
            let frames = self.listen(id, multiplexed);
            heard.insert(id, frames);
        }
        for (i, pid) in pids.iter().enumerate() {
            let Some(id) = parse_monitor_request(&pid.request) else {
                continue;
            };
            let frames = heard.get(&id).cloned().unwrap_or_default();
            let reading = &mut out[i];
            match self.decode_monitored(pid, &frames, &mut reading.value) {
                Ok(()) => reading.ok = true,
                Err(e) => reading.error = e,
            }
        }

        for unit in &order {
            if unit == "listen" {
                continue;
            }
            let mut answers: BTreeMap<String, Answer> = BTreeMap::new(); // by request
            // The extended session, when a value of this unit needs it:
            // Ok when the unit is in it, Err with the reason when not.
            let mut session: Option<Result<(), String>> = None;
            if let Some(first) = pids
                .iter()
                .find(|p| address_key(p) == *unit && p.session != 0)
            {
                session = Some(match sessions.as_deref_mut() {
                    None => Err(no_session_reason()),
                    Some(s) if !s.enabled() => Err(s.off_reason()),
                    Some(s) => match self.address(first).and_then(|_| ecu_definition(first)) {
                        Ok(def) => s.prepare(&mut self.uds, unit, &def),
                        Err(e) => Err(e),
                    },
                });
            }
            let mut retried = false;
            for (i, pid) in pids.iter().enumerate() {
                if address_key(pid) != *unit {
                    continue;
                }
                if pid.session != 0
                    && let Some(Err(reason)) = &session
                {
                    let reading = &mut out[i];
                    reading.value = Self::base_value(pid);
                    reading.error = reason.clone();
                    continue;
                }
                let eq = self.equation(&pid.equation);
                if !eq.ok() {
                    let reading = &mut out[i];
                    reading.value.key = pid.key;
                    reading.value.name = pid.name.clone();
                    reading.error = eq.error().to_string();
                    continue;
                }
                let in_session = matches!(session, Some(Ok(())));
                if !answers.contains_key(&pid.request) {
                    let answer = if in_session {
                        self.ask_in_session(pid, unit, &mut sessions, &mut session, &mut retried)
                    } else {
                        self.ask(pid)
                    };
                    answers.insert(pid.request.clone(), answer);
                }
                let answer = &answers[&pid.request];
                let reading = &mut out[i];
                match self.decode(pid, answer, &mut reading.value) {
                    Ok(()) => {
                        reading.ok = true;
                        reading.session = in_session && pid.session != 0;
                    }
                    Err(e) => reading.error = e,
                }
            }
        }
        out
    }
}

impl CustomPidReader<'_> {
    /// One request to a unit in the extended session. A refusal that looks
    /// like the unit left the session by itself (its S3 timer ran out)
    /// enters it once more and asks again; no answer at all leaves the
    /// session at once.
    fn ask_in_session(
        &mut self,
        pid: &CustomPid,
        unit: &str,
        sessions: &mut Option<&mut ExtendedSessions>,
        session: &mut Option<Result<(), String>>,
        retried: &mut bool,
    ) -> Answer {
        let Some(keeper) = sessions.as_deref_mut() else {
            return Err(no_session_reason());
        };
        let mut result = self.ask_session(pid);
        if let Err(refusal) = &result
            && matches!(refusal.nrc, Some(0x31 | 0x7E | 0x7F))
            && pid.session != 0
            && !*retried
        {
            *retried = true;
            keeper.mark_left(unit);
            match ecu_definition(pid).and_then(|def| keeper.prepare(&mut self.uds, unit, &def)) {
                Ok(()) => result = self.ask_session(pid),
                Err(reason) => {
                    *session = Some(Err(reason.clone()));
                    return Err(reason);
                }
            }
        }
        match result {
            Ok(data) => {
                keeper.touch(unit);
                Ok(data)
            }
            Err(refusal) if refusal.nrc.is_none() => {
                keeper.give_up(&mut self.uds, unit);
                *session = Some(Err(refusal.message.clone()));
                Err(refusal.message)
            }
            Err(refusal) => {
                // The unit answered, so it still counts as kept.
                keeper.touch(unit);
                Err(refusal.message)
            }
        }
    }
}

impl Drop for CustomPidReader<'_> {
    fn drop(&mut self) {
        self.restore_default();
    }
}

/// One value read once, the simple way. Addresses the header (if any),
/// restores the default addressing afterwards. For more than one value use
/// [`CustomPidReader`], which is much faster.
pub fn read_custom(elm: &mut Elm327, pid: &CustomPid) -> Result<LiveValue, String> {
    let mut reader = CustomPidReader::new(elm);
    reader.read(pid)
}
