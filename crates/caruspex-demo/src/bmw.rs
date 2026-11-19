//! Simulated BMWs of the E series on the K-line, for the tests and the
//! dev page: an E46 whose units speak DS2 (9600 baud 8E1) and an E60
//! whose units speak KWP2000 in BMW style (10400 baud 8N1 after fast
//! init), or BMW-FAST (115200 baud, no init) with [`BmwCar::E60Fast`].
//!
//! [`BmwSimLine`] plays the cable and the car: everything written comes
//! back as an echo, as on the real single-wire line, and a whole request
//! frame gets its answer queued right behind the echo. Nothing waits, so a
//! unit that is not fitted costs no time in the tests. The frames are built
//! here on their own, independent of caruspex-protocols, so the tests check
//! one implementation against the other.
//!
//! The car keeps its state (fault memory, what was sent) across
//! connections: [`BmwSimLine::connect`] hands out a new line to the same
//! car. Part numbers and codes are made up.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

use caruspex_link::{ByteStream, LineParity};

const TESTER: u8 = 0xF1;

/// Which car sits behind the simulated cable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BmwCar {
    /// 3 Series E46: DS2 units.
    E46,
    /// 5 Series E60: KWP2000 with fast init at 10400 baud.
    E60,
    /// The same E60 units on BMW-FAST at 115200 baud.
    E60Fast,
}

impl BmwCar {
    /// The name of the device in the port lists ("sim:e46").
    pub fn device(self) -> &'static str {
        match self {
            Self::E46 => "sim:e46",
            Self::E60 => "sim:e60",
            Self::E60Fast => "sim:e60-fast",
        }
    }

    pub fn from_device(device: &str) -> Option<Self> {
        [Self::E46, Self::E60, Self::E60Fast]
            .into_iter()
            .find(|c| c.device() == device)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Speech {
    Ds2,
    Kwp,
    Fast,
}

#[derive(Clone, Debug)]
struct Unit {
    address: u8,
    speech: Speech,
    ident: Vec<u8>,
    /// DS2: whole entries; KWP2000: code high, code low, status.
    faults: Vec<Vec<u8>>,
    /// Comes back right after clearing: the cause is still there.
    stubborn: Option<Vec<u8>>,
    /// DS2: 04 alone is answered with B0, the unit wants 04 01.
    wants_block: bool,
    /// DS2: the first identification is answered with A1 (busy).
    busy_once: bool,
    /// KWP2000: the first fault read is answered with 7F 18 78.
    pending_once: bool,
    clears: u32,
}

impl Unit {
    fn new(address: u8, speech: Speech, ident: &str) -> Self {
        Self {
            address,
            speech,
            ident: ident.as_bytes().to_vec(),
            faults: Vec::new(),
            stubborn: None,
            wants_block: false,
            busy_once: false,
            pending_once: false,
            clears: 0,
        }
    }
    fn faults(mut self, list: &[&[u8]]) -> Self {
        self.faults = list.iter().map(|f| f.to_vec()).collect();
        self
    }
    fn stubborn(mut self, fault: &[u8]) -> Self {
        self.faults.push(fault.to_vec());
        self.stubborn = Some(fault.to_vec());
        self
    }
}

fn e46() -> Vec<Unit> {
    let ds2 = Speech::Ds2;
    let mut dsc = Unit::new(0x56, ds2, "6750920 DSC MK60 SIM").faults(&[&[0x5D, 0x20, 0x01]]);
    dsc.wants_block = true;
    let mut ike = Unit::new(0x80, ds2, "6914880 IKE SIM");
    ike.busy_once = true;
    vec![
        Unit::new(0x12, ds2, "7519308 DME MS43 SIM")
            .faults(&[&[0x2A, 0x01, 0x10, 0x05]])
            .stubborn(&[0x51, 0x21, 0x00, 0x00]),
        Unit::new(0x32, ds2, "7514822 EGS GS20 SIM"),
        dsc,
        Unit::new(0xA4, ds2, "6962580 MRS3 SIM").faults(&[&[0x1F, 0x80]]),
        ike,
        Unit::new(0x44, ds2, "6905668 EWS3 SIM"),
        Unit::new(0x00, ds2, "6904330 GM5 SIM"),
        Unit::new(0xD0, ds2, "6928730 LCM SIM").faults(&[&[0x09, 0x00]]),
        Unit::new(0x5B, ds2, "6911540 IHKA SIM"),
    ]
}

fn e60(speech: Speech) -> Vec<Unit> {
    // Longer than 63 bytes, so the answer needs the extra length byte.
    let long_ident = format!(
        "7560301 DME MSV70 SIM {}",
        "HW 07 SW 33 CODING 0A DIAG 4C BUS 05 WEEK 12 YEAR 05 "
    );
    let mut dsc = Unit::new(0x29, speech, "6769390 DSC SIM").faults(&[&[0x5E, 0x20, 0x20]]);
    dsc.pending_once = true;
    vec![
        Unit::new(0x12, speech, &long_ident)
            .faults(&[&[0x2A, 0x82, 0x60]])
            .stubborn(&[0x29, 0xCC, 0x20]),
        Unit::new(0x18, speech, "7577850 EGS GA6HP SIM"),
        dsc,
        Unit::new(0x01, speech, "6961066 ACSM SIM").faults(&[&[0x93, 0x4B, 0x60]]),
        Unit::new(0x60, speech, "6974990 KOMBI SIM"),
        Unit::new(0x40, speech, "6954720 CAS2 SIM"),
        Unit::new(0x00, speech, "6966355 JBE SIM").faults(&[&[0xA0, 0xB8, 0x20]]),
        Unit::new(0x78, speech, "6983710 IHKA SIM"),
    ]
}

struct Car {
    units: Vec<Unit>,
    /// Every whole frame the car received, StartCommunication included.
    sent: Vec<Vec<u8>>,
    on_kline: bool,
}

/// A simulated cable with a BMW behind it. See the module documentation.
pub struct BmwSimLine {
    car: Arc<Mutex<Car>>,
    baud: u32,
    parity: LineParity,
    to_tester: VecDeque<u8>,
    incoming: Vec<u8>,
    /// The unit with a KWP2000 session after fast init.
    session: Option<u8>,
}

fn ds2_answer(address: u8, status: u8, data: &[u8]) -> Vec<u8> {
    let mut frame = vec![address, (data.len() + 4) as u8, status];
    frame.extend_from_slice(data);
    frame.push(frame.iter().fold(0, |x, &b| x ^ b));
    frame
}

fn kwp_answer(source: u8, data: &[u8]) -> Vec<u8> {
    let mut frame = if data.len() <= 0x3F {
        vec![0x80 | data.len() as u8, TESTER, source]
    } else {
        vec![0x80, TESTER, source, data.len() as u8]
    };
    frame.extend_from_slice(data);
    frame.push(frame.iter().fold(0u8, |s, &b| s.wrapping_add(b)));
    frame
}

impl BmwSimLine {
    pub fn new(car: BmwCar) -> Self {
        let units = match car {
            BmwCar::E46 => e46(),
            BmwCar::E60 => e60(Speech::Kwp),
            BmwCar::E60Fast => e60(Speech::Fast),
        };
        Self::with_car(Arc::new(Mutex::new(Car {
            units,
            sent: Vec::new(),
            on_kline: true,
        })))
    }

    fn with_car(car: Arc<Mutex<Car>>) -> Self {
        Self {
            car,
            baud: 9600,
            parity: LineParity::None,
            to_tester: VecDeque::new(),
            incoming: Vec::new(),
            session: None,
        }
    }

    /// A new line to the same car, as when the cable is opened again.
    pub fn connect(&self) -> Self {
        Self::with_car(self.car.clone())
    }

    fn car(&self) -> MutexGuard<'_, Car> {
        self.car.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Off: the cable is switched to D-CAN. Nothing reaches the K-line,
    /// so there is neither an echo nor an answer.
    pub fn set_on_kline(&self, on: bool) {
        self.car().on_kline = on;
    }

    /// Every whole frame the car received so far.
    pub fn sent_frames(&self) -> Vec<Vec<u8>> {
        self.car().sent.clone()
    }

    pub fn fault_count(&self, address: u8) -> usize {
        self.car()
            .units
            .iter()
            .find(|u| u.address == address)
            .map_or(0, |u| u.faults.len())
    }

    /// How often a unit was asked to clear.
    pub fn clear_count(&self, address: u8) -> u32 {
        self.car()
            .units
            .iter()
            .find(|u| u.address == address)
            .map_or(0, |u| u.clears)
    }

    fn line_speech(&self) -> Option<Speech> {
        match (self.baud, self.parity) {
            (9600, LineParity::Even) => Some(Speech::Ds2),
            (10400, LineParity::None) => Some(Speech::Kwp),
            (115200, LineParity::None) => Some(Speech::Fast),
            _ => None,
        }
    }

    /// Looks whether `incoming` holds a whole frame and answers it.
    fn take_frame(&mut self) {
        let Some(speech) = self.line_speech() else {
            // Wrong speed or parity: the units hear garbage.
            self.incoming.clear();
            return;
        };
        let complete = if speech == Speech::Ds2 {
            match self.incoming.get(1) {
                Some(&length) if length < 3 => {
                    self.incoming.clear();
                    return;
                }
                Some(&length) => self.incoming.len() == usize::from(length),
                None => false,
            }
        } else {
            let format = self.incoming[0];
            let length = usize::from(format & 0x3F);
            let header = if length == 0 { 4 } else { 3 };
            match (length, self.incoming.get(3)) {
                (0, None) => false,
                (0, Some(&extra)) => self.incoming.len() == header + usize::from(extra) + 1,
                _ => self.incoming.len() == header + length + 1,
            }
        };
        if !complete {
            return;
        }
        let frame = std::mem::take(&mut self.incoming);
        self.car().sent.push(frame.clone());
        let answer = if speech == Speech::Ds2 {
            self.answer_ds2(&frame)
        } else {
            self.answer_kwp(speech, &frame)
        };
        self.to_tester.extend(answer);
    }

    fn answer_ds2(&mut self, frame: &[u8]) -> Vec<u8> {
        let (body, sum) = frame.split_at(frame.len() - 1);
        if body.iter().fold(0, |x, &b| x ^ b) != sum[0] {
            return Vec::new();
        }
        let address = frame[0];
        let data = &frame[2..frame.len() - 1];
        let mut car = self.car();
        let Some(unit) = car
            .units
            .iter_mut()
            .find(|u| u.address == address && u.speech == Speech::Ds2)
        else {
            return Vec::new();
        };
        match data {
            [0x00] if unit.busy_once => {
                unit.busy_once = false;
                ds2_answer(address, 0xA1, &[])
            }
            [0x00] => ds2_answer(address, 0xA0, &unit.ident),
            [0x04] if unit.wants_block => ds2_answer(address, 0xB0, &[]),
            [0x04] | [0x04, 0x01] => {
                let mut out = vec![unit.faults.len() as u8];
                for f in &unit.faults {
                    out.extend_from_slice(f);
                }
                ds2_answer(address, 0xA0, &out)
            }
            [0x05] => {
                unit.clears += 1;
                unit.faults = unit.stubborn.iter().cloned().collect();
                ds2_answer(address, 0xA0, &[])
            }
            _ => ds2_answer(address, 0xA2, &[]),
        }
    }

    fn answer_kwp(&mut self, speech: Speech, frame: &[u8]) -> Vec<u8> {
        let (body, sum) = frame.split_at(frame.len() - 1);
        if body.iter().fold(0u8, |s, &b| s.wrapping_add(b)) != sum[0] || frame[2] != TESTER {
            return Vec::new();
        }
        let target = frame[1];
        // KWP2000 at 10400 needs the session of the fast init first.
        if speech == Speech::Kwp && self.session != Some(target) {
            return Vec::new();
        }
        let header = if frame[0] & 0x3F == 0 { 4 } else { 3 };
        let data = frame[header..frame.len() - 1].to_vec();
        let mut car = self.car();
        let Some(unit) = car
            .units
            .iter_mut()
            .find(|u| u.address == target && u.speech == speech)
        else {
            return Vec::new();
        };
        let reply = match data.as_slice() {
            [0x1A, 0x80] => {
                let mut out = vec![0x5A, 0x80];
                out.extend_from_slice(&unit.ident);
                out
            }
            [0x18, 0x02, 0xFF, 0xFF] if unit.pending_once => {
                unit.pending_once = false;
                let mut both = kwp_answer(target, &[0x7F, 0x18, 0x78]);
                let mut out = vec![0x58, unit.faults.len() as u8];
                for f in &unit.faults {
                    out.extend_from_slice(f);
                }
                both.extend(kwp_answer(target, &out));
                return both;
            }
            [0x18, 0x02, 0xFF, 0xFF] => {
                let mut out = vec![0x58, unit.faults.len() as u8];
                for f in &unit.faults {
                    out.extend_from_slice(f);
                }
                out
            }
            [0x14, 0xFF, 0xFF] => {
                unit.clears += 1;
                unit.faults = unit.stubborn.iter().cloned().collect();
                vec![0x54, 0xFF, 0xFF]
            }
            [0x3E, ..] => vec![0x7E],
            [0x82] => {
                drop(car);
                self.session = None;
                return kwp_answer(target, &[0xC2]);
            }
            [sid, ..] => vec![0x7F, *sid, 0x11],
            [] => return Vec::new(),
        };
        kwp_answer(target, &reply)
    }
}

impl ByteStream for BmwSimLine {
    fn write_byte(&mut self, value: u8) -> bool {
        if !self.car().on_kline {
            return true;
        }
        self.to_tester.push_back(value);
        self.incoming.push(value);
        self.take_frame();
        true
    }

    fn read_byte(&mut self, _timeout_ms: u64) -> Option<u8> {
        self.to_tester.pop_front()
    }

    // BMW units do not answer the 5-baud address of VAG.
    fn slow_init(&mut self, _address: u8) -> bool {
        false
    }

    fn fast_init(&mut self, start: &[u8]) -> Option<Vec<u8>> {
        if self.line_speech() != Some(Speech::Kwp) || !self.car().on_kline {
            return None;
        }
        self.car().sent.push(start.to_vec());
        let [0x81, target, TESTER, 0x81, sum] = *start else {
            return None;
        };
        if sum
            != 0x81u8
                .wrapping_add(target)
                .wrapping_add(TESTER)
                .wrapping_add(0x81)
        {
            return None;
        }
        let fitted = self
            .car()
            .units
            .iter()
            .any(|u| u.address == target && u.speech == Speech::Kwp);
        if !fitted {
            return None;
        }
        self.session = Some(target);
        self.incoming.clear();
        self.to_tester.clear();
        // Key bytes EF 8F: KWP2000 with headers and addresses.
        Some(kwp_answer(target, &[0xC1, 0xEF, 0x8F]))
    }

    fn set_line(&mut self, baud: u32, parity: LineParity) -> bool {
        self.baud = baud;
        self.parity = parity;
        self.incoming.clear();
        self.to_tester.clear();
        self.session = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ds2_unit_answers_behind_the_echo() {
        let mut line = BmwSimLine::new(BmwCar::E46);
        assert!(line.set_line(9600, LineParity::Even));
        for b in [0x12, 0x04, 0x00, 0x16] {
            line.write_byte(b);
        }
        let mut heard = Vec::new();
        while let Some(b) = line.read_byte(0) {
            heard.push(b);
        }
        assert_eq!(&heard[..4], &[0x12, 0x04, 0x00, 0x16]);
        assert_eq!(heard[4], 0x12);
        assert_eq!(usize::from(heard[5]), heard.len() - 4);
        assert_eq!(heard[6], 0xA0);
        // At the wrong parity nobody answers.
        line.set_line(9600, LineParity::None);
        for b in [0x12, 0x04, 0x00, 0x16] {
            line.write_byte(b);
        }
        let mut count = 0;
        while line.read_byte(0).is_some() {
            count += 1;
        }
        assert_eq!(count, 4, "only the echo");
    }

    #[test]
    fn the_e60_wants_a_fast_init_first() {
        let mut line = BmwSimLine::new(BmwCar::E60);
        line.set_line(10400, LineParity::None);
        assert_eq!(line.fast_init(&[0x81, 0x55, 0xF1, 0x81, 0x48]), None);
        let answer = line.fast_init(&[0x81, 0x12, 0xF1, 0x81, 0x05]).unwrap();
        assert_eq!(&answer[..4], &[0x83, 0xF1, 0x12, 0xC1]);
        assert_eq!(BmwCar::from_device("sim:e60"), Some(BmwCar::E60));
    }
}
