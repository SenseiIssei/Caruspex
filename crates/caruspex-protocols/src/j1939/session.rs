//! A diagnosis session on a J1939 bus: listen, ask, read and clear faults.
//!
//! The session keeps what it heard (the newest frame of every parameter
//! group per sender, the DM1 of every unit, the address claims) and works
//! over any [`FrameLink`], which is handed in with every call, so the same
//! session can stay alive while the worker keeps the link elsewhere.
//!
//! The tester only claims an address once it has to send something
//! (a request, a clear): listening needs none. It takes F9, the first
//! address for an off-board diagnostic tool, or FA when F9 is in use, and
//! gives way to any unit with a lower NAME, as the address claim wants.

use std::collections::{BTreeMap, BTreeSet};

use caruspex_i18n::l;
use caruspex_link::{BusFrame, FrameLink, LogFn};

use super::dtc::{J1939Dtc, Lamps, SpnConversion, decode_dm};
use super::params::{J1939Value, decode_params};
use super::tp::{TpCm, TpCollector, TpEvent};
use super::{
    GLOBAL, J1939Id, NULL_ADDRESS, PGN_ACK, PGN_ADDRESS_CLAIM, PGN_COMPONENT_ID, PGN_DM1, PGN_DM2,
    PGN_DM3, PGN_DM11, PGN_DM21, PGN_ENGINE_HOURS, PGN_FUEL_CONSUMPTION, PGN_IDLE_OPERATION,
    PGN_REQUEST, PGN_SOFTWARE_ID, PGN_TP_CM, PGN_VEHICLE_DISTANCE, PGN_VEHICLE_DISTANCE_HR,
    PGN_VIN, TESTER_ADDRESSES, address_label,
};

/// How long a request waits for answers (the standard gives a unit 200 ms;
/// adapters and gateways add their own delay).
const REQUEST_WAIT_MS: u64 = 1250;
/// How long the sender of an RTS waits for the next CTS.
const CTS_WAIT_MS: u64 = 1250;
/// How long an address claim may be contested.
const CLAIM_WAIT_MS: u64 = 250;
/// DM1 comes once a second; a long one in packets takes a little longer.
const DM1_WAIT_MS: u64 = 1600;

/// The NAME of Caruspex as a tester: able to take any address, industry
/// group "global", function 129 (off-board service tool), a fixed identity.
pub(crate) const TESTER_NAME: u64 = (1u64 << 63) | (129u64 << 40) | 0x0C_A5;

/// One control unit heard on the bus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct J1939Unit {
    pub address: u8,
    /// "Motor (00)".
    pub label: String,
    /// The NAME from its address claim, if one was heard.
    pub name: Option<u64>,
    /// It sent a DM1 (active faults) while we listened.
    pub sends_dm1: bool,
}

/// The faults of one unit (DM1 or DM2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitFaults {
    pub address: u8,
    pub lamps: Lamps,
    pub dtcs: Vec<J1939Dtc>,
}

/// The answer of a unit to a clear request or another request it does not
/// answer with data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AckKind {
    Positive,
    Negative,
    AccessDenied,
    CannotRespond,
}

impl AckKind {
    fn from_control(control: u8) -> Option<AckKind> {
        Some(match control {
            0 => AckKind::Positive,
            1 => AckKind::Negative,
            2 => AckKind::AccessDenied,
            3 => AckKind::CannotRespond,
            _ => return None,
        })
    }

    pub fn text(self) -> &'static str {
        match self {
            AckKind::Positive => l("erledigt", "done"),
            AckKind::Negative => l("abgelehnt", "refused"),
            AckKind::AccessDenied => l("kein Zugriff", "access denied"),
            AckKind::CannotRespond => l("gerade nicht möglich", "not possible right now"),
        }
    }
}

/// What came back on a clear request (DM11 or DM3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClearResult {
    pub acks: Vec<(u8, AckKind)>,
}

impl ClearResult {
    /// Nobody refused and at least one unit confirmed.
    pub fn confirmed(&self) -> bool {
        !self.acks.is_empty() && self.acks.iter().all(|(_, a)| *a == AckKind::Positive)
    }
}

/// Make, model, serial number and unit number of one control unit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComponentId {
    pub make: String,
    pub model: String,
    pub serial: String,
    pub unit: String,
}

/// Whatever came in for one request.
#[derive(Default)]
struct Answers {
    messages: Vec<(u8, Vec<u8>)>,
    acks: Vec<(u8, AckKind)>,
}

enum Event {
    Message {
        source: u8,
        pgn: u32,
        data: Vec<u8>,
    },
    RequestToSend {
        source: u8,
        size: u16,
        packets: u8,
        max_per_cts: u8,
        pgn: u32,
    },
    Ack {
        source: u8,
        kind: AckKind,
        pgn: u32,
    },
    Aborted {
        source: u8,
    },
}

pub struct J1939Session {
    address: u8,
    claimed: bool,
    /// Addresses claimed by others, with their NAME.
    claims: BTreeMap<u8, u64>,
    /// Addresses a unit with a lower NAME took from us.
    lost: BTreeSet<u8>,
    /// Newest data per sender and parameter group.
    latest: BTreeMap<(u8, u32), Vec<u8>>,
    /// Newest DM1 per sender.
    dm1: BTreeMap<u8, Vec<u8>>,
    tp: TpCollector,
    older: Option<SpnConversion>,
    frames: u64,
    log: Option<LogFn>,
    request_wait_ms: u64,
    dm1_wait_ms: u64,
}

impl Default for J1939Session {
    fn default() -> Self {
        Self::new()
    }
}

fn little_endian_pgn(data: &[u8]) -> u32 {
    u32::from(data[0]) | (u32::from(data[1]) << 8) | (u32::from(data[2]) << 16)
}

/// Text fields of the identification messages: up to the '*' that ends
/// each, without padding or control characters.
fn text_fields(data: &[u8]) -> Vec<String> {
    let text: String = data
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '\u{0}'
            }
        })
        .collect();
    let mut fields: Vec<String> = text
        .split('*')
        .map(|f| f.replace('\u{0}', "").trim().to_string())
        .collect();
    // After the last '*' there is only padding.
    if fields.len() > 1 {
        fields.pop();
    }
    fields
}

impl J1939Session {
    pub fn new() -> Self {
        J1939Session {
            address: TESTER_ADDRESSES[0],
            claimed: false,
            claims: BTreeMap::new(),
            lost: BTreeSet::new(),
            latest: BTreeMap::new(),
            dm1: BTreeMap::new(),
            tp: TpCollector::new(),
            older: None,
            frames: 0,
            log: None,
            request_wait_ms: REQUEST_WAIT_MS,
            dm1_wait_ms: DM1_WAIT_MS,
        }
    }

    pub fn set_log(&mut self, log: LogFn) {
        self.log = Some(log);
    }

    fn log(&self, line: &str) {
        if let Some(log) = &self.log {
            log(line);
        }
    }

    /// How long a request waits for answers and how long reading the
    /// active faults listens; shorter in tests against a simulation that
    /// answers at once.
    pub fn set_waits(&mut self, request_ms: u64, dm1_ms: u64) {
        self.request_wait_ms = request_ms;
        self.dm1_wait_ms = dm1_ms;
    }

    /// The tester's address, and whether it has been claimed yet.
    pub fn address(&self) -> (u8, bool) {
        (self.address, self.claimed)
    }

    /// Everything one frame tells.
    fn take(&mut self, frame: &BusFrame) -> Option<Event> {
        self.frames += 1;
        let id = J1939Id::parse(frame.id);
        let data = &frame.data;
        let us = self.claimed.then_some(self.address);
        if id.pgn == PGN_ADDRESS_CLAIM && data.len() >= 8 {
            let name = u64::from_le_bytes(data[..8].try_into().ok()?);
            if name == TESTER_NAME {
                return None; // our own claim, echoed
            }
            if id.source == NULL_ADDRESS {
                return None; // "cannot claim"
            }
            self.claims.insert(id.source, name);
            if id.source == self.address && name < TESTER_NAME {
                self.lost.insert(self.address);
                self.claimed = false;
            }
            return None;
        }
        if self.claimed && id.source == self.address {
            return None;
        }
        if id.destination != GLOBAL && Some(id.destination) != us {
            return None; // a PDU1 message between two others
        }
        match id.pgn {
            PGN_TP_CM | super::PGN_TP_DT => match self.tp.feed(id, data, us)? {
                TpEvent::Complete {
                    source, pgn, data, ..
                } => {
                    self.store(source, pgn, &data);
                    Some(Event::Message { source, pgn, data })
                }
                TpEvent::RequestToSend {
                    source,
                    size,
                    packets,
                    max_per_cts,
                    pgn,
                } => Some(Event::RequestToSend {
                    source,
                    size,
                    packets,
                    max_per_cts,
                    pgn,
                }),
                TpEvent::Aborted { source, .. } => Some(Event::Aborted { source }),
            },
            PGN_ACK if data.len() >= 8 => Some(Event::Ack {
                source: id.source,
                kind: AckKind::from_control(data[0])?,
                pgn: little_endian_pgn(&data[5..8]),
            }),
            PGN_REQUEST => None,
            pgn => {
                self.store(id.source, pgn, data);
                Some(Event::Message {
                    source: id.source,
                    pgn,
                    data: data.clone(),
                })
            }
        }
    }

    fn store(&mut self, source: u8, pgn: u32, data: &[u8]) {
        if pgn == PGN_DM1 {
            self.dm1.insert(source, data.to_vec());
        }
        self.latest.insert((source, pgn), data.to_vec());
    }

    /// Listens for `ms` milliseconds and keeps everything heard. Returns the
    /// number of frames.
    pub fn listen(&mut self, link: &mut dyn FrameLink, ms: u64) -> Result<u64, String> {
        let before = self.frames;
        link.exchange(None, ms, &mut |f| {
            self.take(f);
            false
        })?;
        Ok(self.frames - before)
    }

    /// Every unit heard so far (sent anything or claimed an address),
    /// without the tester itself.
    pub fn units(&self) -> Vec<J1939Unit> {
        let mut addresses: BTreeSet<u8> = self.latest.keys().map(|(a, _)| *a).collect();
        addresses.extend(self.claims.keys());
        if self.claimed {
            addresses.remove(&self.address);
        }
        addresses.remove(&NULL_ADDRESS);
        addresses
            .into_iter()
            .map(|address| J1939Unit {
                address,
                label: address_label(address),
                name: self.claims.get(&address).copied(),
                sends_dm1: self.dm1.contains_key(&address),
            })
            .collect()
    }

    /// The values of the table, decoded from the newest frame of each group
    /// and sender, sorted by SPN and sender.
    pub fn values(&self) -> Vec<J1939Value> {
        let mut out: Vec<J1939Value> = self
            .latest
            .iter()
            .flat_map(|((source, pgn), data)| decode_params(*pgn, *source, data))
            .collect();
        out.sort_by_key(|v| (v.spn, v.source));
        out
    }

    /// The newest data of one parameter group from one sender.
    pub fn latest(&self, source: u8, pgn: u32) -> Option<&[u8]> {
        self.latest.get(&(source, pgn)).map(Vec::as_slice)
    }

    fn send(&mut self, link: &mut dyn FrameLink, frame: &BusFrame) -> Result<(), String> {
        link.exchange(Some(frame), 0, &mut |_| true)
    }

    /// Claims the tester address if that has not happened yet.
    fn ensure_address(&mut self, link: &mut dyn FrameLink) -> Result<(), String> {
        for _ in 0..6 {
            if self.claimed {
                return Ok(());
            }
            let taken = |a: &u8| self.claims.contains_key(a) || self.lost.contains(a);
            let Some(address) = TESTER_ADDRESSES
                .iter()
                .chain((0x80..=0xF7).collect::<Vec<u8>>().iter().rev())
                .find(|a| !taken(a))
                .copied()
            else {
                return Err(l(
                    "Auf dem Bus ist keine Adresse für das Diagnosegerät mehr frei.",
                    "No address is left free for the diagnostic tool on the bus.",
                )
                .to_string());
            };
            self.address = address;
            self.claimed = true;
            let claim = BusFrame {
                id: J1939Id::raw(6, PGN_ADDRESS_CLAIM, GLOBAL, address),
                data: TESTER_NAME.to_le_bytes().to_vec(),
            };
            link.exchange(Some(&claim), CLAIM_WAIT_MS, &mut |f| {
                self.take(f);
                false
            })?;
            if self.claimed {
                self.log(&format!(
                    "{}{address:02X}",
                    l(
                        "  J1939: Diagnosegerät meldet sich mit Adresse ",
                        "  J1939: diagnostic tool claims address "
                    )
                ));
            }
        }
        if self.claimed {
            Ok(())
        } else {
            Err(l(
                "Das Diagnosegerät bekommt auf dem Bus keine Adresse.",
                "The diagnostic tool gets no address on the bus.",
            )
            .to_string())
        }
    }

    /// Asks for `pgn` at `destination` (or everybody with [`GLOBAL`]) and
    /// collects the answers: the messages in that group (over the transport
    /// protocol where needed) and acknowledgements. A unit asked directly
    /// is waited for until it answered; a global request runs the whole
    /// wait.
    fn request(
        &mut self,
        link: &mut dyn FrameLink,
        pgn: u32,
        destination: u8,
    ) -> Result<Answers, String> {
        self.ensure_address(link)?;
        let request = BusFrame {
            id: J1939Id::raw(6, PGN_REQUEST, destination, self.address),
            data: vec![pgn as u8, (pgn >> 8) as u8, (pgn >> 16) as u8],
        };
        let mut answers = Answers::default();
        let mut pending: Vec<(u8, u16, u8, u8)> = Vec::new();
        let direct = destination != GLOBAL;
        let handle = |session: &mut Self,
                      answers: &mut Answers,
                      pending: &mut Vec<(u8, u16, u8, u8)>,
                      f: &BusFrame|
         -> bool {
            match session.take(f) {
                Some(Event::Message {
                    source,
                    pgn: got,
                    data,
                }) if got == pgn && (!direct || source == destination) => {
                    answers.messages.push((source, data));
                    direct
                }
                Some(Event::RequestToSend {
                    source,
                    size,
                    packets,
                    max_per_cts,
                    pgn: got,
                }) if got == pgn => {
                    pending.push((source, size, packets, max_per_cts));
                    true
                }
                Some(Event::Ack {
                    source,
                    kind,
                    pgn: got,
                }) if got == pgn && (!direct || source == destination) => {
                    answers.acks.push((source, kind));
                    direct
                }
                _ => false,
            }
        };
        link.exchange(Some(&request), self.request_wait_ms, &mut |f| {
            handle(self, &mut answers, &mut pending, f)
        })?;
        // Units that want to send their answer in packets get a CTS each.
        let mut rounds = 0;
        while let Some((source, size, packets, max_per_cts)) = pending.pop() {
            rounds += 1;
            if rounds > 8 {
                break;
            }
            if let Some(data) = self.receive_rts(link, source, size, packets, max_per_cts, pgn)? {
                answers.messages.push((source, data));
            }
            if !direct && pending.is_empty() {
                // Others may still be on their way.
                link.exchange(None, 200, &mut |f| {
                    handle(self, &mut answers, &mut pending, f)
                })?;
            }
        }
        Ok(answers)
    }

    /// The receiving side of an RTS/CTS transfer: CTS for the missing
    /// packets until the message is complete, then the end of message
    /// acknowledgement. `None` if the sender went quiet or gave up.
    fn receive_rts(
        &mut self,
        link: &mut dyn FrameLink,
        source: u8,
        size: u16,
        packets: u8,
        max_per_cts: u8,
        pgn: u32,
    ) -> Result<Option<Vec<u8>>, String> {
        let mut result: Option<Vec<u8>> = None;
        for _ in 0..usize::from(packets) + 3 {
            let Some(next) = self.tp.missing(source, self.address) else {
                break;
            };
            let count = (packets - next + 1).min(max_per_cts.max(1));
            let cts = BusFrame {
                id: J1939Id::raw(7, PGN_TP_CM, source, self.address),
                data: TpCm::Cts {
                    packets: count,
                    next,
                    pgn,
                }
                .encode(),
            };
            let mut aborted = false;
            link.exchange(Some(&cts), CTS_WAIT_MS, &mut |f| match self.take(f) {
                Some(Event::Message {
                    source: s,
                    pgn: got,
                    data,
                }) if s == source && got == pgn => {
                    result = Some(data);
                    true
                }
                Some(Event::Aborted { source: s }) if s == source => {
                    aborted = true;
                    true
                }
                _ => false,
            })?;
            if result.is_some() || aborted {
                break;
            }
        }
        if result.is_some() {
            let ack = BusFrame {
                id: J1939Id::raw(7, PGN_TP_CM, source, self.address),
                data: TpCm::EndOfMessage { size, packets, pgn }.encode(),
            };
            self.send(link, &ack)?;
        } else {
            self.tp.drop_transfer(source, self.address);
            let abort = BusFrame {
                id: J1939Id::raw(7, PGN_TP_CM, source, self.address),
                data: TpCm::Abort { reason: 3, pgn }.encode(),
            };
            self.send(link, &abort)?;
        }
        Ok(result)
    }

    /// The active faults (DM1) of every unit. DM1 is broadcast once a
    /// second, so this listens first and asks only if nothing came.
    pub fn active_faults(&mut self, link: &mut dyn FrameLink) -> Result<Vec<UnitFaults>, String> {
        self.dm1.clear();
        self.listen(link, self.dm1_wait_ms)?;
        if self.dm1.is_empty() {
            let answers = self.request(link, PGN_DM1, GLOBAL)?;
            for (source, data) in answers.messages {
                self.dm1.insert(source, data);
            }
        }
        Ok(self
            .dm1
            .iter()
            .map(|(&address, data)| {
                let dm = decode_dm(data, self.older);
                UnitFaults {
                    address,
                    lamps: dm.lamps,
                    dtcs: dm.dtcs,
                }
            })
            .collect())
    }

    /// The previously active faults (DM2) of every unit that answers.
    pub fn previous_faults(&mut self, link: &mut dyn FrameLink) -> Result<Vec<UnitFaults>, String> {
        let answers = self.request(link, PGN_DM2, GLOBAL)?;
        let mut by_unit: BTreeMap<u8, Vec<u8>> = BTreeMap::new();
        for (source, data) in answers.messages {
            by_unit.insert(source, data);
        }
        Ok(by_unit
            .into_iter()
            .map(|(address, data)| {
                let dm = decode_dm(&data, self.older);
                UnitFaults {
                    address,
                    lamps: dm.lamps,
                    dtcs: dm.dtcs,
                }
            })
            .collect())
    }

    /// The VIN: asked of the engine (0x00) directly when it is on the bus,
    /// else of everybody.
    pub fn read_vin(&mut self, link: &mut dyn FrameLink) -> Result<Option<String>, String> {
        let engine_heard = self.latest.keys().any(|(a, _)| *a == 0x00);
        let mut answers = if engine_heard {
            self.request(link, PGN_VIN, 0x00)?.messages
        } else {
            Vec::new()
        };
        if answers.is_empty() {
            answers = self.request(link, PGN_VIN, GLOBAL)?.messages;
        }
        Ok(answers
            .iter()
            .filter_map(|(_, data)| text_fields(data).into_iter().next())
            .find(|vin| !vin.is_empty()))
    }

    /// The VIN of every unit that answers a request to everybody. Engine,
    /// instrument cluster and tachograph often each keep their own copy, and
    /// a copy that differs points to a unit from another vehicle.
    pub fn read_vins(&mut self, link: &mut dyn FrameLink) -> Result<Vec<(u8, String)>, String> {
        let answers = self.request(link, PGN_VIN, GLOBAL)?;
        let mut out: Vec<(u8, String)> = answers
            .messages
            .iter()
            .filter_map(|(source, data)| {
                let vin = text_fields(data).into_iter().next()?;
                (!vin.is_empty()).then_some((*source, vin))
            })
            .collect();
        out.sort_by_key(|(source, _)| *source);
        out.dedup_by_key(|(source, _)| *source);
        Ok(out)
    }

    /// Asks for what a used vehicle check compares: idle hours and idle
    /// fuel, both odometers (some units send them only on request) and DM21
    /// (distance and time with the lamp on and since the last clearing).
    /// Everything lands in [`J1939Session::values`].
    pub fn read_usage(&mut self, link: &mut dyn FrameLink) -> Result<(), String> {
        for pgn in [
            PGN_IDLE_OPERATION,
            PGN_VEHICLE_DISTANCE,
            PGN_VEHICLE_DISTANCE_HR,
            PGN_DM21,
        ] {
            self.request(link, pgn, GLOBAL)?;
        }
        Ok(())
    }

    /// Make, model, serial and unit number of every unit that answers.
    pub fn component_ids(
        &mut self,
        link: &mut dyn FrameLink,
    ) -> Result<Vec<(u8, ComponentId)>, String> {
        let answers = self.request(link, PGN_COMPONENT_ID, GLOBAL)?;
        Ok(answers
            .messages
            .into_iter()
            .map(|(source, data)| {
                let mut fields = text_fields(&data).into_iter();
                let mut next = || fields.next().unwrap_or_default();
                (
                    source,
                    ComponentId {
                        make: next(),
                        model: next(),
                        serial: next(),
                        unit: next(),
                    },
                )
            })
            .collect())
    }

    /// The software identification of every unit that answers: one or more
    /// version texts each.
    pub fn software_ids(
        &mut self,
        link: &mut dyn FrameLink,
    ) -> Result<Vec<(u8, Vec<String>)>, String> {
        let answers = self.request(link, PGN_SOFTWARE_ID, GLOBAL)?;
        Ok(answers
            .messages
            .into_iter()
            .filter(|(_, data)| !data.is_empty())
            .map(|(source, data)| {
                let count = usize::from(data[0]);
                let mut fields = text_fields(&data[1..]);
                fields.retain(|f| !f.is_empty());
                fields.truncate(count.max(1));
                (source, fields)
            })
            .collect())
    }

    /// Asks for the counters most trucks send only on request: engine hours
    /// and fuel used. They land in [`J1939Session::values`].
    pub fn read_counters(&mut self, link: &mut dyn FrameLink) -> Result<(), String> {
        for pgn in [PGN_ENGINE_HOURS, PGN_FUEL_CONSUMPTION] {
            self.request(link, pgn, GLOBAL)?;
        }
        Ok(())
    }

    /// Clears the active faults (DM11) of every unit. Only call this after
    /// the user confirmed it: on a truck it can also reset counters the
    /// workshop needs.
    pub fn clear_active(&mut self, link: &mut dyn FrameLink) -> Result<ClearResult, String> {
        self.clear(link, PGN_DM11)
    }

    /// Clears the previously active faults (DM3) of every unit.
    pub fn clear_previous(&mut self, link: &mut dyn FrameLink) -> Result<ClearResult, String> {
        self.clear(link, PGN_DM3)
    }

    fn clear(&mut self, link: &mut dyn FrameLink, pgn: u32) -> Result<ClearResult, String> {
        let answers = self.request(link, pgn, GLOBAL)?;
        if pgn == PGN_DM11 {
            self.dm1.clear();
        }
        for (source, kind) in &answers.acks {
            self.log(&format!(
                "  J1939 {}: {} {}",
                if pgn == PGN_DM11 { "DM11" } else { "DM3" },
                address_label(*source),
                kind.text()
            ));
        }
        Ok(ClearResult { acks: answers.acks })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_fields_end_at_the_star() {
        assert_eq!(
            text_fields(b"1M2AX09C5JM000001*"),
            vec!["1M2AX09C5JM000001"]
        );
        assert_eq!(
            text_fields(b"MAKE*MODEL*SN1*U7*\xFF\xFF"),
            vec!["MAKE", "MODEL", "SN1", "U7"]
        );
        assert_eq!(text_fields(b"NOSTAR"), vec!["NOSTAR"]);
    }

    #[test]
    fn tester_name_gives_way() {
        // Arbitrary address capable puts the top bit: any unit with a fixed
        // address has a lower NAME and keeps its address.
        const { assert!(TESTER_NAME > 0x0000_FFFF_FFFF_FFFF) };
        assert_eq!(AckKind::from_control(0), Some(AckKind::Positive));
        assert_eq!(AckKind::from_control(9), None);
    }
}
