//! A simulated Audi A2 (8Z, 1.4 petrol) behind a KKL cable with an FTDI
//! chip, at the level of the bare line: break edges, single bytes and their
//! times. It is what the first-test page and its tests run against.
//!
//! [`A2Line`] implements [`RawKLine`] with a clock of its own: waits of the
//! tester (the 200 ms bits, read timeouts) are skipped instead of slept, and
//! the short pauses of the protocol code (`timing::sleep_ms`) move the same
//! simulated clock. Real time does not count at all, so a slow machine gives
//! the same result as a fast one and the times the tester measures are the
//! ones the simulated car sees.
//!
//! The control units listen for their 5-baud address and check its edges
//! against a 200 ms grid, send the sync byte 0x55 and the key bytes 01 8A,
//! wait at most 50 ms for the inverted second key byte, then speak KWP1281
//! byte by byte with complements, built here on their own, independent of
//! caruspex-protocols. [`A2Fault`] breaks one thing at a time, for every
//! verdict of the test. Part numbers, the VIN and the fault entries are made
//! up.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use caruspex_link::RawKLine;

/// The port name of the simulated car in the lists.
pub const A2_SIM_DEVICE: &str = "sim:a2";

/// One byte at 9600 baud (start, 8 data, stop).
const BYTE_US: u64 = 1_042;
/// The control units accept an edge this far off its place.
const EDGE_TOLERANCE_US: u64 = 25_000;
/// W1: the sync byte comes this long after the stop bit.
const SYNC_DELAY_US: u64 = 40_000;
/// W2 and W3 between sync and key bytes.
const KEY_GAP_US: u64 = 8_000;
/// W4 max: the inverted second key byte must come within 50 ms.
const REPLY_MAX_US: u64 = 50_000;
/// A control unit answers a byte this long after it came.
const TURN_US: u64 = 1_500;
/// After the reply to the key bytes the first block starts this much later.
const FIRST_BLOCK_US: u64 = 15_000;

const READ_FAULTS: u8 = 0x07;
const READ_GROUP: u8 = 0x29;
const END_OUTPUT: u8 = 0x06;
const ACK: u8 = 0x09;
const NOT_SUPPORTED: u8 = 0x0A;
const ASCII_DATA: u8 = 0xF6;
const FAULT_LIST: u8 = 0xFC;
const GROUP_READING: u8 = 0xE7;

/// One thing that goes wrong, for the verdicts of the test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum A2Fault {
    #[default]
    None,
    /// Releasing the break takes 38 ms, setting it 3 ms (a slow USB chip):
    /// the edges of the address land off their places.
    SlowBreak,
    /// Every received byte waits 40 ms in the chip (latency timer 40 ms):
    /// the reply to the key bytes comes too late.
    SlowRead,
    /// Ignition off: the line has power, no control unit answers.
    NoAnswer,
    /// The control units do not take the inverted key byte.
    KeyInversion,
    /// Something else on the line: the tester's own bytes come back changed.
    BrokenEcho,
    /// The cable is on USB only, or pin 16 has no 12 V: no echo, no car.
    NoPower,
}

impl A2Fault {
    pub const ALL: [A2Fault; 7] = [
        Self::None,
        Self::SlowBreak,
        Self::SlowRead,
        Self::NoAnswer,
        Self::KeyInversion,
        Self::BrokenEcho,
        Self::NoPower,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::None => "",
            Self::SlowBreak => "slow-break",
            Self::SlowRead => "slow-read",
            Self::NoAnswer => "no-answer",
            Self::KeyInversion => "key-inversion",
            Self::BrokenEcho => "broken-echo",
            Self::NoPower => "no-power",
        }
    }

    /// "sim:a2", or "sim:a2:slow-read" and so on.
    pub fn device(self) -> String {
        match self {
            Self::None => A2_SIM_DEVICE.to_string(),
            other => format!("{A2_SIM_DEVICE}:{}", other.key()),
        }
    }

    pub fn from_device(device: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.device() == device)
    }

    /// The latency timer the simulated FTDI driver reports.
    pub fn latency_ms(self) -> u32 {
        if self == Self::SlowRead { 40 } else { 1 }
    }

    fn powered(self) -> bool {
        self != Self::NoPower
    }
}

/// A control unit of the simulated car.
struct Unit {
    address: u8,
    identification: &'static [&'static str],
    /// Three bytes per entry: number high, low, status.
    faults: &'static [u8],
    groups: &'static [(u8, [u8; 12])],
}

// Engine at idle: 800 rpm, 90 degrees C, lambda +1 %, conditions; load,
// injection time, air mass; throttle angle and ignition timing.
const ENGINE_GROUPS: [(u8, [u8; 12]); 3] = [
    (
        1,
        [
            0x01, 0x14, 0xC8, 0x05, 0x0A, 0xBE, 0x14, 0x19, 0x85, 0x25, 0x00, 0x6F,
        ],
    ),
    (
        2,
        [
            0x01, 0x14, 0xC8, 0x21, 0x00, 0x33, 0x0F, 0x19, 0x0A, 0x19, 0x02, 0x80,
        ],
    ),
    (
        3,
        [
            0x01, 0x14, 0xC8, 0x19, 0x02, 0x80, 0x17, 0x64, 0x08, 0x04, 0x64, 0x8B,
        ],
    ),
];

/// "No fault": the list a KWP1281 unit sends when its memory is empty.
const NO_FAULTS: [u8; 3] = [0xFF, 0xFF, 0x88];

static UNITS: [Unit; 5] = [
    Unit {
        address: 0x01,
        identification: &["036906034AM ", "MARELLI 4LV    ", "1,4l R4/4V      3170"],
        // 16684, misfire, sporadic.
        faults: &[0x41, 0x2C, 0xA3],
        groups: &ENGINE_GROUPS,
    },
    Unit {
        address: 0x17,
        identification: &[
            "8Z0920900F  KOMBI+WEGFAHRSP VDO V05",
            "WAUZZZ8Z01N000417",
            "AUZ7Z0A0000417",
        ],
        faults: &NO_FAULTS,
        groups: &[],
    },
    Unit {
        address: 0x03,
        identification: &["8Z0614517C  ABS/ESP ITT 20 IE  ", "0001"],
        faults: &NO_FAULTS,
        groups: &[],
    },
    Unit {
        address: 0x15,
        identification: &["8Z0959655B  AIRBAG VW61   ", "0105"],
        // 00588, driver's igniter, resistance too high.
        faults: &[0x02, 0x4C, 0x1B],
        groups: &[],
    },
    Unit {
        address: 0x46,
        identification: &["8Z0959433C  ZENTRALE KOMFORT", "0002"],
        faults: &NO_FAULTS,
        groups: &[],
    },
];
// 09 central electrics is not fitted in this car, so the test also meets a
// unit that stays silent.

enum Phase {
    /// Sync and key bytes are out, the reply is due.
    Handshake {
        key2: u8,
        key2_end_us: u64,
    },
    Blocks,
}

/// The control unit that is awake.
struct Session {
    unit: usize,
    phase: Phase,
    counter: u8,
    identification_index: usize,
    /// The rest of the block being sent.
    pending: VecDeque<u8>,
    awaiting_ack: bool,
    last_sent: u8,
    incoming: Vec<u8>,
    /// The last tester block asked for the fault memory: an ACK ends it.
    after_faults: bool,
}

/// The simulated A2 behind its KKL cable.
pub struct A2Line {
    fault: A2Fault,
    skipped_us: u64,
    baud: u32,
    /// Break edges since the last address: time and the new level (true =
    /// released).
    edges: Vec<(u64, bool)>,
    /// What the tester can read: when it shows up, the byte, and whether
    /// the control unit sent it (garbled at another baud rate).
    rx: VecDeque<(u64, u8, bool)>,
    session: Option<Session>,
    /// Requests that would change something in the car (clearing, output
    /// test, basic settings, adaptation): the first test must send none.
    changing_requests: Arc<AtomicUsize>,
}

impl A2Line {
    pub fn new(fault: A2Fault) -> Self {
        Self {
            fault,
            skipped_us: {
                // The simulated clock of this thread starts with the line.
                caruspex_link::timing::use_virtual_clock(true);
                0
            },
            baud: 9600,
            edges: Vec::new(),
            rx: VecDeque::new(),
            session: None,
            changing_requests: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn fault(&self) -> A2Fault {
        self.fault
    }

    /// Counts the requests that would change something in the car; keep a
    /// clone before handing the line away.
    pub fn changing_requests(&self) -> Arc<AtomicUsize> {
        self.changing_requests.clone()
    }

    fn latency_us(&self) -> u64 {
        u64::from(self.fault.latency_ms()) * 1000
    }

    fn skip_to(&mut self, at: u64) {
        let now = self.now_us();
        if at > now {
            self.skipped_us += at - now;
        }
    }

    /// The control unit puts a byte on the line at `at`.
    fn unit_sends(&mut self, at: u64, byte: u8) {
        let visible = at + BYTE_US + self.latency_us();
        self.rx.push_back((visible, byte, true));
    }

    // --- The 5-baud address ---------------------------------------------------

    /// Looks at the edges once the address must be over: decodes it and
    /// wakes the unit when everything lines up.
    fn decode_address(&mut self) {
        let now = self.now_us();
        let Some(&(start, level)) = self.edges.first() else {
            return;
        };
        if level {
            // A release without a start bit before it.
            self.edges.remove(0);
            return;
        }
        let end = start + 10 * 200_000;
        if now < end {
            return;
        }
        let edges = std::mem::take(&mut self.edges);
        // Every edge on the 200 ms grid of the start bit.
        let on_grid = edges.iter().all(|&(t, _)| {
            let offset = t - start;
            let nearest = (offset + 100_000) / 200_000 * 200_000;
            offset.abs_diff(nearest) <= EDGE_TOLERANCE_US
        });
        if !on_grid || !self.fault.powered() {
            return;
        }
        let level_at = |t: u64| {
            edges
                .iter()
                .take_while(|&&(e, _)| e <= t)
                .last()
                .map(|&(_, l)| l)
                .unwrap_or(true)
        };
        let bits: Vec<bool> = (0..10)
            .map(|i| level_at(start + i * 200_000 + 100_000))
            .collect();
        if bits[0] || !bits[9] {
            return;
        }
        let mut address = 0u8;
        let mut ones = 0;
        for i in 0..7 {
            if bits[1 + i] {
                address |= 1 << i;
                ones += 1;
            }
        }
        if (ones + usize::from(bits[8])) % 2 == 0 {
            return; // odd parity broken
        }
        let Some(unit) = UNITS.iter().position(|u| u.address == address) else {
            return;
        };
        if self.fault == A2Fault::NoAnswer {
            return;
        }
        let sync = end + SYNC_DELAY_US;
        let key1 = sync + BYTE_US + KEY_GAP_US;
        let key2 = key1 + BYTE_US + KEY_GAP_US;
        self.unit_sends(sync, 0x55);
        self.unit_sends(key1, 0x01);
        self.unit_sends(key2, 0x8A);
        self.session = Some(Session {
            unit,
            phase: Phase::Handshake {
                key2: 0x8A,
                key2_end_us: key2 + BYTE_US,
            },
            counter: 0,
            identification_index: 0,
            pending: VecDeque::new(),
            awaiting_ack: false,
            last_sent: 0,
            incoming: Vec::new(),
            after_faults: false,
        });
    }

    // --- KWP1281 of the control unit ---------------------------------------------

    fn begin_block(&mut self, at: u64, title: u8, payload: &[u8]) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        s.counter = s.counter.wrapping_add(1);
        s.pending.clear();
        s.pending.push_back((payload.len() + 3) as u8);
        s.pending.push_back(s.counter);
        s.pending.push_back(title);
        s.pending.extend(payload.iter().copied());
        s.pending.push_back(0x03);
        self.send_next(at);
    }

    fn send_next(&mut self, at: u64) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let Some(byte) = s.pending.pop_front() else {
            return;
        };
        s.last_sent = byte;
        // The block end gets no complement.
        s.awaiting_ack = !s.pending.is_empty();
        self.unit_sends(at, byte);
    }

    fn next_identification(&mut self, at: u64) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        let unit = &UNITS[s.unit];
        if let Some(text) = unit.identification.get(s.identification_index) {
            s.identification_index += 1;
            self.begin_block(at, ASCII_DATA, text.as_bytes());
        } else {
            self.begin_block(at, ACK, &[]);
        }
    }

    fn handle_block(&mut self, at: u64, block: &[u8]) {
        let title = block[2];
        let payload = block[3..block.len() - 1].to_vec();
        let Some(s) = self.session.as_mut() else {
            return;
        };
        s.counter = block[1];
        let unit = &UNITS[s.unit];
        match title {
            ACK => {
                if std::mem::take(&mut s.after_faults) {
                    self.begin_block(at, ACK, &[]);
                } else {
                    self.next_identification(at);
                }
            }
            READ_FAULTS => {
                s.after_faults = true;
                let faults = unit.faults;
                self.begin_block(at, FAULT_LIST, faults);
            }
            READ_GROUP => {
                let group = payload.first().copied().unwrap_or(0);
                match unit.groups.iter().find(|(g, _)| *g == group) {
                    Some((_, values)) => {
                        let values = *values;
                        self.begin_block(at, GROUP_READING, &values);
                    }
                    None => self.begin_block(at, NOT_SUPPORTED, &[]),
                }
            }
            END_OUTPUT => self.session = None,
            _ => {
                // Clearing, output test, basic settings, adaptation: none of
                // it belongs to a read-only test.
                self.changing_requests.fetch_add(1, Ordering::SeqCst);
                self.begin_block(at, NOT_SUPPORTED, &[]);
            }
        }
    }

    /// A byte of the tester reaches the control unit at `at`.
    fn unit_receives(&mut self, at: u64, byte: u8) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        match s.phase {
            Phase::Handshake { key2, key2_end_us } => {
                let in_time = at.saturating_sub(key2_end_us) <= REPLY_MAX_US;
                if byte != !key2 || !in_time || self.fault == A2Fault::KeyInversion {
                    self.session = None;
                    return;
                }
                s.phase = Phase::Blocks;
                self.next_identification(at + FIRST_BLOCK_US);
            }
            Phase::Blocks => {
                if s.awaiting_ack {
                    if byte != !s.last_sent {
                        self.session = None;
                        return;
                    }
                    s.awaiting_ack = false;
                    self.send_next(at + TURN_US);
                    return;
                }
                s.incoming.push(byte);
                let length = usize::from(s.incoming[0]);
                if s.incoming.len() == length + 1 {
                    let block = std::mem::take(&mut s.incoming);
                    if block.len() >= 4 {
                        self.handle_block(at + TURN_US, &block);
                    }
                    return;
                }
                self.unit_sends(at + TURN_US, !byte);
            }
        }
    }
}

impl Drop for A2Line {
    // The worker thread lives on after a simulated run and must be back in
    // real time for a real cable.
    fn drop(&mut self) {
        caruspex_link::timing::use_virtual_clock(false);
    }
}

impl RawKLine for A2Line {
    fn now_us(&self) -> u64 {
        caruspex_link::timing::virtual_now_us().unwrap_or(0) + self.skipped_us
    }

    fn sleep_until_us(&mut self, deadline_us: u64) {
        self.skip_to(deadline_us);
        self.decode_address();
    }

    fn set_break(&mut self, active: bool) -> bool {
        if self.fault == A2Fault::SlowBreak {
            let delay = if active { 3_000 } else { 38_000 };
            let at = self.now_us() + delay;
            self.skip_to(at);
        }
        self.decode_address();
        let now = self.now_us();
        let level = !active;
        if self.edges.last().map(|&(_, l)| l) != Some(level) && !(self.edges.is_empty() && level) {
            self.edges.push((now, level));
        }
        // On a powered line the break comes back as a 0x00 with a framing
        // error.
        if active && self.fault.powered() {
            let visible = now + BYTE_US + self.latency_us();
            self.rx.push_back((visible, 0x00, false));
        }
        true
    }

    fn flush_input(&mut self) {
        let now = self.now_us();
        self.rx.retain(|&(at, _, _)| at > now);
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        self.decode_address();
        let now = self.now_us();
        let Some(first) = self.rx.iter().map(|&(at, _, _)| at).min() else {
            self.skip_to(now + timeout_ms * 1000);
            self.decode_address();
            return Vec::new();
        };
        if first > now + timeout_ms * 1000 {
            self.skip_to(now + timeout_ms * 1000);
            self.decode_address();
            return Vec::new();
        }
        self.skip_to(first);
        let now = self.now_us();
        let (mut due, rest): (Vec<_>, Vec<_>) =
            self.rx.drain(..).partition(|&(at, _, _)| at <= now);
        self.rx = rest.into();
        due.sort_by_key(|&(at, _, _)| at);
        // The units speak 9600 baud; at another rate the tester reads
        // garbage.
        due.into_iter()
            .map(|(_, byte, from_unit)| {
                if from_unit && self.baud != 9600 {
                    byte ^ 0x5A
                } else {
                    byte
                }
            })
            .collect()
    }

    fn write(&mut self, data: &[u8]) -> bool {
        self.decode_address();
        let start = self.now_us();
        for (i, &byte) in data.iter().enumerate() {
            let at = start + i as u64 * BYTE_US;
            if !self.fault.powered() {
                continue;
            }
            let echo = if self.fault == A2Fault::BrokenEcho {
                byte ^ 0x10
            } else {
                byte
            };
            let visible = at + BYTE_US + self.latency_us();
            self.rx.push_back((visible, echo, false));
            self.unit_receives(at + BYTE_US, byte);
        }
        // The write returns once the bytes are on the line.
        self.skip_to(start + data.len() as u64 * BYTE_US);
        true
    }

    fn set_baud(&mut self, baud: u32) -> bool {
        self.baud = baud;
        true
    }

    fn baud(&self) -> u32 {
        self.baud
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use caruspex_link::kline_timed::{EchoState, analyze, judge};
    use caruspex_link::{ByteStream, TimedKLine, WakeStrategy, WakeVerdict};

    fn wake(fault: A2Fault, strategy: WakeStrategy) -> (TimedKLine<A2Line>, bool) {
        let mut k = TimedKLine::new(A2Line::new(fault), 9600);
        k.set_strategy(strategy);
        assert!(k.slow_init(0x01));
        let ok = k.read_byte(2000) == Some(0x55)
            && k.read_byte(1000) == Some(0x01)
            && k.read_byte(1000) == Some(0x8A);
        (k, ok)
    }

    #[test]
    fn the_engine_wakes_and_sends_its_identification() {
        let (mut k, ok) = wake(A2Fault::None, WakeStrategy::Standard);
        assert!(ok);
        k.pause_ms(30);
        assert!(k.write_byte(0x75));
        assert_eq!(k.read_byte(500), Some(0x75), "echo");
        // The first identification block: length, counter, ASCII title.
        assert_eq!(k.read_byte(1000), Some(15));
        let trace = k.take_trace().unwrap();
        let findings = analyze(&trace);
        assert!(findings.address_echo >= 2, "{findings:?}");
        assert!(findings.worst_edge_error_us < 5_000, "{findings:?}");
        assert_eq!(findings.key_bytes, Some((0x01, 0x8A)));
    }

    #[test]
    fn a_slow_break_misses_the_grid_and_compensation_fixes_it() {
        let (mut k, ok) = wake(A2Fault::SlowBreak, WakeStrategy::Standard);
        assert!(!ok);
        let trace = k.take_trace().unwrap();
        let findings = analyze(&trace);
        assert!(findings.worst_edge_error_us > 30_000, "{findings:?}");
        assert_eq!(
            judge(&trace, &findings, false, Some(1)),
            WakeVerdict::TimingUnstable
        );
        let lead = WakeStrategy::Compensated {
            on_lead_us: findings.mean_on_call_us,
            off_lead_us: findings.mean_off_call_us,
        };
        let (_, ok) = wake(A2Fault::SlowBreak, lead);
        assert!(ok);
    }

    #[test]
    fn without_power_nothing_comes_back() {
        let mut k = TimedKLine::new(A2Line::new(A2Fault::NoPower), 9600);
        assert_eq!(k.echo_probe(&[0x55, 0xAA]).state(), EchoState::None);
        let mut k = TimedKLine::new(A2Line::new(A2Fault::BrokenEcho), 9600);
        assert_eq!(k.echo_probe(&[0x55, 0xAA]).state(), EchoState::Broken);
        let mut k = TimedKLine::new(A2Line::new(A2Fault::None), 9600);
        let probe = k.echo_probe(&[0x55, 0xAA]);
        assert_eq!(probe.state(), EchoState::Clean);
        assert!(probe.worst_round_trip_us().unwrap() < 5_000);
    }

    #[test]
    fn at_10400_baud_the_sync_byte_is_garbage() {
        let mut k = TimedKLine::new(A2Line::new(A2Fault::None), 10400);
        assert!(k.slow_init(0x01));
        let first = k.read_byte(2000);
        assert!(first.is_some() && first != Some(0x55));
    }

    #[test]
    fn devices_name_their_fault() {
        for fault in A2Fault::ALL {
            assert_eq!(A2Fault::from_device(&fault.device()), Some(fault));
        }
        assert_eq!(A2Fault::from_device("sim:e46"), None);
        assert_eq!(A2Fault::SlowRead.latency_ms(), 40);
    }
}
