//! A K-line with a stopwatch, for the first test of a KKL cable on a car.
//!
//! [`TimedKLine`] does what [`crate::KLineStream`] does (the 5-baud wake-up
//! address clocked by hand with the break line, then single bytes at 9600
//! baud) and notes the time of every step on the way: how long each break
//! call took and when the edge really came, when the sync byte 0x55 and the
//! key bytes arrived, how long the reply to the key bytes took and whether
//! its echo came back. [`analyze`] and [`judge`] turn that into a verdict a
//! car owner can act on.
//!
//! The line itself sits behind [`RawKLine`], so the simulated car of
//! caruspex-demo can take the place of the serial port, with its own clock.
//!
//! Timing limits from ISO 9141-2: 5 baud with 5 % tolerance (190 to 210 ms
//! per bit), W1 = 20 to 300 ms from the end of the address to the sync byte,
//! W4 = 25 to 50 ms from the second key byte to the tester's inverted reply.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use caruspex_i18n::l;

use crate::kline_stream::wake_up_bits;
use crate::serial::Serial;
use crate::transport::{ByteStream, LineParity};

/// 5 baud: 200 ms per bit.
pub(crate) const BIT_US: u64 = 200_000;
/// ISO 9141-2 allows 5 % on the bit time.
pub const BIT_TOLERANCE_US: u64 = 10_000;
/// Beyond this many control units no longer recognise the address.
pub const BIT_LIMIT_US: u64 = 20_000;
/// W4 max: the inverted second key byte must be on the line within 50 ms.
pub const REPLY_MAX_US: u64 = 50_000;
/// The idle time before the address, as in KLineStream.
const IDLE_BEFORE_US: u64 = 350_000;
/// How many line events after the address are kept: enough for sync, key
/// bytes, reply and the start of the first block.
const TRACE_EVENTS: usize = 64;
/// How long the echo probe waits for each byte to come back.
const ECHO_WAIT_US: u64 = 150_000;

/// A K-line at the lowest level: the break line, raw bytes and a clock.
/// Times are microseconds on a monotonic clock of the line's choosing.
pub trait RawKLine: Send {
    fn now_us(&self) -> u64;
    /// Waits until the clock shows `deadline_us`.
    fn sleep_until_us(&mut self, deadline_us: u64);
    /// Pulls the transmit line low (`true`) or releases it.
    fn set_break(&mut self, active: bool) -> bool;
    fn flush_input(&mut self);
    /// What arrives within `timeout_ms`, returns as soon as something came.
    fn read(&mut self, timeout_ms: u64) -> Vec<u8>;
    fn write(&mut self, data: &[u8]) -> bool;
    fn set_baud(&mut self, baud: u32) -> bool;
    fn baud(&self) -> u32;
}

impl<T: RawKLine + ?Sized> RawKLine for Box<T> {
    fn now_us(&self) -> u64 {
        (**self).now_us()
    }
    fn sleep_until_us(&mut self, deadline_us: u64) {
        (**self).sleep_until_us(deadline_us)
    }
    fn set_break(&mut self, active: bool) -> bool {
        (**self).set_break(active)
    }
    fn flush_input(&mut self) {
        (**self).flush_input()
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        (**self).read(timeout_ms)
    }
    fn write(&mut self, data: &[u8]) -> bool {
        (**self).write(data)
    }
    fn set_baud(&mut self, baud: u32) -> bool {
        (**self).set_baud(baud)
    }
    fn baud(&self) -> u32 {
        (**self).baud()
    }
}

/// A serial port as a raw K-line, on the real clock.
pub struct SerialLine {
    serial: Serial,
    epoch: Instant,
}

impl SerialLine {
    /// Opens the port the user picked, 8N1 at `baud`.
    pub fn open(device: &str, baud: u32) -> Result<Self, String> {
        let mut serial = Serial::new();
        serial.open(device, baud)?;
        Ok(Self {
            serial,
            epoch: Instant::now(),
        })
    }
}

impl RawKLine for SerialLine {
    fn now_us(&self) -> u64 {
        self.epoch.elapsed().as_micros() as u64
    }
    fn sleep_until_us(&mut self, deadline_us: u64) {
        // Sleep most of the way, then yield for the rest: the operating
        // system tends to oversleep by a millisecond or two.
        loop {
            let now = self.now_us();
            if now >= deadline_us {
                return;
            }
            let remaining = deadline_us - now;
            if remaining > 3_000 {
                std::thread::sleep(Duration::from_micros(remaining - 2_000));
            } else {
                std::thread::yield_now();
            }
        }
    }
    fn set_break(&mut self, active: bool) -> bool {
        self.serial.set_break(active)
    }
    fn flush_input(&mut self) {
        self.serial.flush_input()
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        self.serial.read(timeout_ms)
    }
    fn write(&mut self, data: &[u8]) -> bool {
        self.serial.write(data)
    }
    fn set_baud(&mut self, baud: u32) -> bool {
        self.serial.change_baud(baud)
    }
    fn baud(&self) -> u32 {
        self.serial.baud()
    }
}

/// How the address bits are clocked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WakeStrategy {
    /// Every edge at its target time, like KLineStream.
    #[default]
    Standard,
    /// Every break call starts earlier by the time such a call took on the
    /// last attempt, so a line that only switches when a slow call returns
    /// still switches on time. Assumes the line changes when the call ends.
    Compensated { on_lead_us: u64, off_lead_us: u64 },
}

impl WakeStrategy {
    pub fn key(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Compensated { .. } => "compensated",
        }
    }

    fn lead_us(self, one: bool) -> u64 {
        match self {
            Self::Standard => 0,
            Self::Compensated {
                on_lead_us,
                off_lead_us,
            } => {
                if one {
                    off_lead_us
                } else {
                    on_lead_us
                }
            }
        }
    }

    fn max_lead_us(self) -> u64 {
        self.lead_us(true).max(self.lead_us(false))
    }
}

/// One bit of the wake-up address as it went out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitEdge {
    /// A one (line released) or a zero (break on).
    pub one: bool,
    /// Where the bit should start, from the start of the address.
    pub target_us: u64,
    /// When the break call that switched to this bit returned, from the
    /// start of the address. `None` when the bit has the level of the one
    /// before, so nothing had to switch.
    pub edge_us: Option<u64>,
    /// How long that break call took.
    pub call_us: u64,
}

/// Something on the line after the address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEvent {
    /// A byte came in; the time is when the read returned it.
    Received { at_us: u64, byte: u8 },
    /// A byte went out; the time is when the write call started.
    Sent { at_us: u64, byte: u8, call_us: u64 },
}

/// Everything measured during one wake-up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WakeTrace {
    pub address: u8,
    /// The baud rate after the address.
    pub baud: u32,
    pub strategy: WakeStrategy,
    pub bits: Vec<BitEdge>,
    /// The break line could not be switched.
    pub break_failed: bool,
    /// The baud rate could not be set after the address.
    pub baud_failed: bool,
    /// From the start of the address to the end of the stop bit.
    pub address_us: u64,
    /// Bytes heard while the address went out: the break comes back as
    /// 0x00 when the K-line is powered.
    pub address_echo: usize,
    /// What happened after the address, times from its end.
    pub events: Vec<LineEvent>,
}

/// Sends a few bytes and listens for them: a KKL cable on a powered K-line
/// hears everything it sends.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EchoProbe {
    pub sent: Vec<u8>,
    pub received: Vec<u8>,
    /// Per byte that came back: from the start of the write to the read.
    pub round_trip_us: Vec<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EchoState {
    /// Nothing came back.
    None,
    /// Exactly what was sent.
    Clean,
    /// Something else, or only a part.
    Broken,
}

impl EchoProbe {
    pub fn state(&self) -> EchoState {
        if self.received.is_empty() {
            EchoState::None
        } else if self.received == self.sent {
            EchoState::Clean
        } else {
            EchoState::Broken
        }
    }

    pub fn worst_round_trip_us(&self) -> Option<u64> {
        self.round_trip_us.iter().copied().max()
    }
}

/// [`crate::KLineStream`] with a stopwatch: a byte stream for the K-line
/// protocols that records the timing of every wake-up.
pub struct TimedKLine<L: RawKLine> {
    line: L,
    baud_after_init: u32,
    strategy: WakeStrategy,
    buffer: VecDeque<(u64, u8)>,
    trace: Option<WakeTrace>,
    address_end_us: u64,
    /// Until the first byte after the address that is not the 0x00 of the
    /// break: the sync byte is due.
    awaiting_sync: bool,
}

impl<L: RawKLine> TimedKLine<L> {
    pub fn new(line: L, baud_after_init: u32) -> Self {
        Self {
            line,
            baud_after_init,
            strategy: WakeStrategy::Standard,
            buffer: VecDeque::new(),
            trace: None,
            address_end_us: 0,
            awaiting_sync: false,
        }
    }

    pub fn set_strategy(&mut self, strategy: WakeStrategy) {
        self.strategy = strategy;
    }

    pub fn strategy(&self) -> WakeStrategy {
        self.strategy
    }

    pub fn set_baud_after_init(&mut self, baud: u32) {
        self.baud_after_init = baud;
    }

    /// The measurements of the last wake-up, taken out.
    pub fn take_trace(&mut self) -> Option<WakeTrace> {
        self.trace.take()
    }

    pub fn line(&self) -> &L {
        &self.line
    }

    pub fn line_mut(&mut self) -> &mut L {
        &mut self.line
    }

    /// Waits on the line's clock, with the line idle.
    pub fn pause_ms(&mut self, ms: u64) {
        let deadline = self.line.now_us() + ms * 1000;
        self.line.sleep_until_us(deadline);
    }

    fn record(&mut self, event: LineEvent) {
        if let Some(trace) = self.trace.as_mut()
            && trace.events.len() < TRACE_EVENTS
        {
            trace.events.push(event);
        }
    }

    fn since_address(&self, at: u64) -> u64 {
        at.saturating_sub(self.address_end_us)
    }

    /// Sends `bytes` one by one at the current baud rate and waits for each
    /// to come back. Harmless for a sleeping control unit, which only
    /// listens for the 5-baud address; the line should rest a while
    /// afterwards before a wake-up.
    pub fn echo_probe(&mut self, bytes: &[u8]) -> EchoProbe {
        let mut probe = EchoProbe {
            sent: bytes.to_vec(),
            ..EchoProbe::default()
        };
        self.line.flush_input();
        self.buffer.clear();
        for &byte in bytes {
            let start = self.line.now_us();
            if !self.line.write(&[byte]) {
                break;
            }
            let deadline = start + ECHO_WAIT_US;
            loop {
                let now = self.line.now_us();
                if now >= deadline {
                    break;
                }
                let wait = ((deadline - now) / 1000).clamp(1, 20);
                let chunk = self.line.read(wait);
                if let Some(&first) = chunk.first() {
                    probe.received.push(first);
                    probe
                        .round_trip_us
                        .push(self.line.now_us().saturating_sub(start));
                    // More than one byte for one sent: keep them, they make
                    // the echo count as broken.
                    probe.received.extend_from_slice(&chunk[1..]);
                    break;
                }
            }
        }
        probe
    }

    /// Clocks the bits of `address` out on the break line and measures them,
    /// without listening for an answer. The cable test runs this with the
    /// cable on USB only; on a car it would wake a control unit.
    pub fn clock_address(&mut self, address: u8) -> WakeTrace {
        let mut trace = WakeTrace {
            address,
            baud: self.baud_after_init,
            strategy: self.strategy,
            ..WakeTrace::default()
        };
        if !self.line.set_break(false) {
            trace.break_failed = true;
            return trace;
        }
        self.pause_ms(IDLE_BEFORE_US / 1000);

        // Every edge gets a fixed target counted from the start, so the
        // operating system's overshoot does not carry over from bit to bit.
        // The start lies ahead by the largest lead, so even the first edge
        // can be started early.
        let start = self.line.now_us() + self.strategy.max_lead_us();
        let mut level = true;
        for (index, one) in wake_up_bits(address).into_iter().enumerate() {
            let target_us = index as u64 * BIT_US;
            if one == level {
                trace.bits.push(BitEdge {
                    one,
                    target_us,
                    edge_us: None,
                    call_us: 0,
                });
                continue;
            }
            let lead = self.strategy.lead_us(one);
            self.line
                .sleep_until_us((start + target_us).saturating_sub(lead));
            let before = self.line.now_us();
            let ok = self.line.set_break(!one);
            let after = self.line.now_us();
            trace.bits.push(BitEdge {
                one,
                target_us,
                edge_us: Some(after.saturating_sub(start)),
                call_us: after - before,
            });
            if !ok {
                trace.break_failed = true;
                self.line.set_break(false);
                return trace;
            }
            level = one;
        }
        self.line.sleep_until_us(start + 10 * BIT_US);
        // The end of the stop bit, taken before the last release: the line
        // is high already, a slow call would only push the end back.
        let end = self.line.now_us();
        self.line.set_break(false);
        trace.address_us = end.saturating_sub(start);
        self.address_end_us = end;
        trace
    }
}

impl<L: RawKLine> ByteStream for TimedKLine<L> {
    fn write_byte(&mut self, value: u8) -> bool {
        let before = self.line.now_us();
        let ok = self.line.write(&[value]);
        let after = self.line.now_us();
        let at_us = self.since_address(before);
        self.record(LineEvent::Sent {
            at_us,
            byte: value,
            call_us: after - before,
        });
        ok
    }

    fn read_byte(&mut self, timeout_ms: u64) -> Option<u8> {
        let deadline = self.line.now_us() + timeout_ms * 1000;
        loop {
            while let Some((at, byte)) = self.buffer.pop_front() {
                let at_us = self.since_address(at);
                self.record(LineEvent::Received { at_us, byte });
                // The break of the address can still trickle in as 0x00
                // after the address; the sync byte is 0x55, never 0x00.
                if self.awaiting_sync && byte == 0x00 {
                    if let Some(trace) = self.trace.as_mut() {
                        trace.address_echo += 1;
                    }
                    continue;
                }
                self.awaiting_sync = false;
                return Some(byte);
            }
            let now = self.line.now_us();
            if now >= deadline {
                return None;
            }
            let wait = ((deadline - now) / 1000).clamp(1, 20);
            let chunk = self.line.read(wait);
            let at = self.line.now_us();
            self.buffer.extend(chunk.into_iter().map(|b| (at, b)));
        }
    }

    fn slow_init(&mut self, address: u8) -> bool {
        self.line.flush_input();
        self.buffer.clear();
        self.awaiting_sync = false;
        let mut trace = self.clock_address(address);
        if trace.break_failed {
            self.trace = Some(trace);
            return false;
        }
        // What came in during the address is the echo of its breaks. Read it
        // instead of flushing blindly: a quick control unit may already send
        // the sync byte 20 ms after the stop bit.
        for _ in 0..4 {
            let chunk = self.line.read(1);
            if chunk.is_empty() {
                break;
            }
            trace.address_echo += chunk.len();
        }
        if !self.line.set_baud(self.baud_after_init) {
            trace.baud_failed = true;
            self.trace = Some(trace);
            return false;
        }
        self.awaiting_sync = true;
        self.trace = Some(trace);
        true
    }

    fn echoes_own_bytes(&self) -> bool {
        true
    }

    fn set_line(&mut self, baud: u32, parity: LineParity) -> bool {
        if parity != LineParity::None {
            return false;
        }
        self.buffer.clear();
        if self.line.baud() != baud && !self.line.set_baud(baud) {
            return false;
        }
        self.baud_after_init = baud;
        self.line.flush_input();
        true
    }
}

// --- What the measurements say ----------------------------------------------------

/// A stretch of the address with one level: what the line really did,
/// compared with the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinePhase {
    pub one: bool,
    pub bits: u8,
    pub measured_us: u64,
    pub target_us: u64,
    /// The break call that started this phase.
    pub call_us: u64,
}

/// The numbers of one wake-up, worked out of its trace.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WakeFindings {
    pub phases: Vec<LinePhase>,
    /// The largest deviation of an edge from its place, counted from the
    /// first edge, the way the control unit measures.
    pub worst_edge_error_us: u64,
    /// The whole address against 2000 ms.
    pub address_error_us: i64,
    /// Average time of a break-on and of a break-off call.
    pub mean_on_call_us: u64,
    pub mean_off_call_us: u64,
    pub max_call_us: u64,
    pub address_echo: usize,
    /// The first byte after the address that is not the break's 0x00.
    pub first_byte: Option<u8>,
    pub first_byte_after_us: Option<u64>,
    pub key_bytes: Option<(u8, u8)>,
    /// From the second key byte (as read) to the start of the reply.
    pub reply_after_us: Option<u64>,
    /// The echo of the reply: Some(true) when it matched.
    pub reply_echo: Option<bool>,
    /// From the start of the reply to its echo.
    pub reply_round_trip_us: Option<u64>,
    /// The control unit said something after the reply.
    pub heard_after_reply: bool,
}

fn mean(values: &[u64]) -> u64 {
    if values.is_empty() {
        0
    } else {
        values.iter().sum::<u64>() / values.len() as u64
    }
}

/// Works the numbers out of a trace.
pub fn analyze(trace: &WakeTrace) -> WakeFindings {
    let mut findings = WakeFindings {
        address_echo: trace.address_echo,
        ..WakeFindings::default()
    };

    // Edges: only the bits where the level changed.
    let edges: Vec<(&BitEdge, u64)> = trace
        .bits
        .iter()
        .filter_map(|b| b.edge_us.map(|e| (b, e)))
        .collect();
    if let Some(&(first, first_edge)) = edges.first() {
        for &(bit, edge) in &edges {
            let measured = edge.saturating_sub(first_edge) as i64;
            let target = bit.target_us.saturating_sub(first.target_us) as i64;
            findings.worst_edge_error_us =
                findings.worst_edge_error_us.max(measured.abs_diff(target));
        }
        let on: Vec<u64> = edges
            .iter()
            .filter(|(b, _)| !b.one)
            .map(|(b, _)| b.call_us)
            .collect();
        let off: Vec<u64> = edges
            .iter()
            .filter(|(b, _)| b.one)
            .map(|(b, _)| b.call_us)
            .collect();
        findings.mean_on_call_us = mean(&on);
        findings.mean_off_call_us = mean(&off);
        findings.max_call_us = edges.iter().map(|(b, _)| b.call_us).max().unwrap_or(0);

        // Phases: from one edge to the next, the last one to the end.
        for (i, &(bit, edge)) in edges.iter().enumerate() {
            let (next_edge, next_target) = match edges.get(i + 1) {
                Some(&(next, e)) => (e, next.target_us),
                None => (trace.address_us, 10 * BIT_US),
            };
            let target_us = next_target.saturating_sub(bit.target_us);
            findings.phases.push(LinePhase {
                one: bit.one,
                bits: (target_us / BIT_US) as u8,
                measured_us: next_edge.saturating_sub(edge),
                target_us,
                call_us: bit.call_us,
            });
        }
        let length = trace.address_us.saturating_sub(first_edge) as i64;
        let wanted = (10 * BIT_US - first.target_us) as i64;
        findings.address_error_us = length - wanted;
    }

    // The bytes after the address.
    let mut received = trace.events.iter().filter_map(|e| match *e {
        LineEvent::Received { at_us, byte } => Some((at_us, byte)),
        LineEvent::Sent { .. } => None,
    });
    let mut first = received.by_ref().skip_while(|&(_, b)| b == 0x00);
    if let Some((at, byte)) = first.next() {
        findings.first_byte = Some(byte);
        findings.first_byte_after_us = Some(at);
        if byte == 0x55
            && let (Some((_, k1)), Some((k2_at, k2))) = (first.next(), first.next())
        {
            findings.key_bytes = Some((k1, k2));
            reply_findings(trace, &mut findings, k2_at, k2);
        }
    }
    findings
}

/// The tester's reply to the key bytes: when it went out, its echo and
/// whether the control unit went on.
fn reply_findings(trace: &WakeTrace, findings: &mut WakeFindings, key2_at: u64, key2: u8) {
    let reply = !key2;
    let Some(sent_index) = trace.events.iter().position(
        |e| matches!(*e, LineEvent::Sent { byte, at_us, .. } if byte == reply && at_us >= key2_at),
    ) else {
        return;
    };
    let LineEvent::Sent { at_us: sent_at, .. } = trace.events[sent_index] else {
        return;
    };
    findings.reply_after_us = Some(sent_at.saturating_sub(key2_at));
    let mut after = trace.events[sent_index + 1..]
        .iter()
        .filter_map(|e| match *e {
            LineEvent::Received { at_us, byte } => Some((at_us, byte)),
            LineEvent::Sent { .. } => None,
        });
    if let Some((echo_at, echo)) = after.next() {
        findings.reply_echo = Some(echo == reply);
        findings.reply_round_trip_us = Some(echo_at.saturating_sub(sent_at));
        findings.heard_after_reply = after.next().is_some();
    }
}

/// What the first test or the cable test found, in a word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeVerdict {
    /// The control unit answered and the session runs.
    Connected,
    /// Cable test: no echo with the cable on USB only, as it should be, and
    /// the break line switches in time.
    CableReady,
    /// Cable test: the cable hears itself, so it sits on a powered K-line.
    CableOnCar,
    /// The break line or the baud rate cannot be set.
    PortProblem,
    /// The cable does not hear itself.
    NoEcho,
    /// The cable hears something else than it sent.
    EchoBroken,
    /// The address bits are too far off 200 ms.
    TimingUnstable,
    /// Everything on our side is fine, the control unit stays silent.
    NoAnswer,
    /// A byte came, but not the sync byte 0x55.
    WrongSync,
    /// The sync byte came, the key bytes did not.
    KeyBytesMissing,
    /// The reply to the key bytes was later than the control unit waits.
    ReplyTooLate,
    /// The reply came in time, the control unit did not go on anyway.
    ReplyRejected,
    /// Woken, but the blocks afterwards broke off.
    ProtocolError,
}

impl WakeVerdict {
    pub fn key(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::CableReady => "cable-ready",
            Self::CableOnCar => "cable-on-car",
            Self::PortProblem => "port-problem",
            Self::NoEcho => "no-echo",
            Self::EchoBroken => "echo-broken",
            Self::TimingUnstable => "timing-unstable",
            Self::NoAnswer => "no-answer",
            Self::WrongSync => "wrong-sync",
            Self::KeyBytesMissing => "key-bytes-missing",
            Self::ReplyTooLate => "reply-too-late",
            Self::ReplyRejected => "reply-rejected",
            Self::ProtocolError => "protocol-error",
        }
    }

    /// A good result: nothing to fix.
    pub fn is_ok(self) -> bool {
        matches!(self, Self::Connected | Self::CableReady | Self::CableOnCar)
    }

    /// The cable or the computer is at fault: every other control unit
    /// would fail the same way, so the test stops.
    pub fn stops_the_test(self) -> bool {
        matches!(
            self,
            Self::PortProblem
                | Self::NoEcho
                | Self::EchoBroken
                | Self::TimingUnstable
                | Self::ReplyTooLate
        )
    }

    pub fn headline(self) -> &'static str {
        match self {
            Self::Connected => l(
                "Das Steuergerät hat sich gemeldet",
                "The control unit answered",
            ),
            Self::CableReady => l(
                "Kabel in Ordnung, bereit für das Auto",
                "Cable OK, ready for the car",
            ),
            Self::CableOnCar => l(
                "Das Kabel hört sich selbst: es steckt an einer K-Line mit Strom",
                "The cable hears itself: it is on a powered K-line",
            ),
            Self::PortProblem => l(
                "Das Kabel lässt sich nicht so schalten, wie die K-Line es braucht",
                "The cable cannot be switched the way the K-line needs",
            ),
            Self::NoEcho => l(
                "Kein Echo: das Kabel hört sich selbst nicht",
                "No echo: the cable does not hear itself",
            ),
            Self::EchoBroken => l("Das Echo ist verfälscht", "The echo is garbled"),
            Self::TimingUnstable => l(
                "Die Weckadresse ist zu ungenau getaktet",
                "The wake-up address is clocked too unevenly",
            ),
            Self::NoAnswer => l(
                "Kabel in Ordnung, aber keine Antwort vom Auto",
                "Cable OK, but no answer from the car",
            ),
            Self::WrongSync => l(
                "Es kam eine Antwort, aber nicht das Synchronbyte 0x55",
                "An answer came, but not the sync byte 0x55",
            ),
            Self::KeyBytesMissing => l(
                "Das Synchronbyte kam, die Schlüsselbytes nicht",
                "The sync byte came, the key bytes did not",
            ),
            Self::ReplyTooLate => l(
                "Die Antwort auf die Schlüsselbytes kam zu spät",
                "The reply to the key bytes came too late",
            ),
            Self::ReplyRejected => l(
                "Das Steuergerät hat die Antwort auf die Schlüsselbytes nicht angenommen",
                "The control unit did not accept the reply to the key bytes",
            ),
            Self::ProtocolError => l(
                "Geweckt, aber danach brach der Austausch ab",
                "Woken, but the exchange broke off afterwards",
            ),
        }
    }

    pub fn advice(self) -> &'static str {
        match self {
            Self::Connected => "",
            Self::CableReady => l(
                "Ohne Auto kommt kein Echo zurück, das ist richtig so. Die Break-Leitung schaltet genau genug für die 5-Baud-Weckadresse. Als Nächstes das Kabel ins Auto stecken und den ersten Test starten.",
                "Without the car no echo comes back, which is how it should be. The break line switches accurately enough for the 5-baud wake-up address. Next, plug the cable into the car and start the first test.",
            ),
            Self::CableOnCar => l(
                "Ein KKL-Kabel hört alles, was es sendet, sobald die K-Line Strom hat, also steckt es schon im Auto. Die Weckadresse misst deshalb erst der erste Test, damit kein Steuergerät ungefragt geweckt wird.",
                "A KKL cable hears everything it sends once the K-line has power, so it is already in the car. That is why only the first test measures the wake-up address, so no control unit is woken unasked.",
            ),
            Self::PortProblem => l(
                "Die Break-Leitung oder die Baudrate ließen sich nicht setzen. Ein anderes Programm am Port schließen, den Treiber des Kabels neu installieren oder ein anderes Kabel probieren: manche Nachbau-Chips können keine Break-Leitung.",
                "The break line or the baud rate could not be set. Close any other program on the port, reinstall the cable's driver or try another cable: some cloned chips cannot drive the break line.",
            ),
            Self::NoEcho => l(
                "Am Auto hört ein KKL-Kabel alles, was es sendet. Ohne Echo steckt der Stecker nicht richtig in der OBD-Buchse, oder an Pin 16 fehlen die 12 V (Sicherung der Diagnosebuchse prüfen). Ohne Auto ist kein Echo normal.",
                "On the car a KKL cable hears everything it sends. Without an echo the plug is not seated in the OBD socket, or pin 16 has no 12 V (check the fuse of the diagnostic socket). Without the car, no echo is normal.",
            ),
            Self::EchoBroken => l(
                "Zurück kommt nicht das, was gesendet wurde. Steckt noch ein anderes Gerät an der Buchse (Tracker, zweiter Tester)? Hat das Kabel einen Umschalter (K+DCAN), muss er auf K-Line stehen. Sonst Kabel und Stecker prüfen.",
                "What comes back is not what was sent. Is another device on the socket (tracker, second tester)? If the cable has a switch (K+DCAN), it must be on K-line. Otherwise check the cable and the plug.",
            ),
            Self::TimingUnstable => l(
                "Die Bits der 5-Baud-Adresse weichen um mehr als 20 ms von 200 ms ab, so erkennt das Steuergerät seine Adresse nicht. Das Kabel direkt am Rechner statt an einem USB-Hub anstecken, andere Programme schließen, den Energiesparmodus ausschalten, beim FTDI-Kabel den Latenz-Timer auf 1 ms stellen.",
                "The bits of the 5-baud address are more than 20 ms off 200 ms, so the control unit does not recognise its address. Plug the cable into the computer directly instead of a USB hub, close other programs, turn power saving off, set the latency timer of an FTDI cable to 1 ms.",
            ),
            Self::NoAnswer => l(
                "Echo und Zeiten passen, nur das Steuergerät schweigt. Ist die Zündung an (Motor aus)? Adresse 01 ist das Motorsteuergerät. Das Auto muss die K-Line an Pin 7 haben; Autos ab etwa 2005 werden über CAN diagnostiziert.",
                "Echo and timing are fine, only the control unit stays silent. Is the ignition on (engine off)? Address 01 is the engine control unit. The car needs the K-line on pin 7; cars from about 2005 are diagnosed over CAN.",
            ),
            Self::WrongSync => l(
                "Vielleicht spricht das Steuergerät eine andere Baudrate; Caruspex hat 10400 Baud mitprobiert. Möglich sind auch Störungen auf der Leitung: Stecker und Masse prüfen.",
                "The control unit may use another baud rate; Caruspex also tried 10400 baud. Interference on the line is possible too: check the plug and the ground.",
            ),
            Self::KeyBytesMissing => l(
                "Das Steuergerät hat angefangen und dann aufgehört. Oft hilft ein zweiter Versuch; kommt es jedes Mal, gehen im Kabel Bytes verloren.",
                "The control unit started and then stopped. A second attempt often helps; if it happens every time, the cable loses bytes.",
            ),
            Self::ReplyTooLate => l(
                "Das Steuergerät wartet höchstens 50 ms auf das umgedrehte zweite Schlüsselbyte. Die Bytes bleiben zu lange im USB-Chip: beim FTDI-Kabel den Latenz-Timer auf 1 ms stellen und das Kabel nicht über einen USB-Hub anschließen.",
                "The control unit waits at most 50 ms for the inverted second key byte. The bytes stay too long in the USB chip: set the latency timer of an FTDI cable to 1 ms and do not connect the cable through a USB hub.",
            ),
            Self::ReplyRejected => l(
                "Die Antwort kam rechtzeitig und ihr Echo stimmte, trotzdem kam danach nichts mehr. Das kann ein anderes Gerät an der K-Line oder ein Wackelkontakt sein. Die Zündung einmal aus- und wieder einschalten und neu versuchen.",
                "The reply came in time and its echo matched, yet nothing followed. Another device on the K-line or a loose contact can cause this. Switch the ignition off and on once and try again.",
            ),
            Self::ProtocolError => l(
                "Die Weckadresse hat geklappt, danach riss der Austausch der Blöcke ab. Die genaue Stelle steht im Protokoll darunter.",
                "The wake-up address worked, then the exchange of blocks broke off. The exact place is in the log below.",
            ),
        }
    }
}

/// The verdict of one wake-up attempt. `latency_ms` is the FTDI latency
/// timer when known: the reply to the key bytes is read that much later than
/// the byte really came.
pub fn judge(
    trace: &WakeTrace,
    findings: &WakeFindings,
    connected: bool,
    latency_ms: Option<u32>,
) -> WakeVerdict {
    if trace.break_failed || trace.baud_failed {
        return WakeVerdict::PortProblem;
    }
    if connected {
        return WakeVerdict::Connected;
    }
    let Some(first) = findings.first_byte else {
        if findings.worst_edge_error_us > BIT_LIMIT_US
            || findings.address_error_us.unsigned_abs() > 2 * BIT_LIMIT_US
        {
            return WakeVerdict::TimingUnstable;
        }
        return WakeVerdict::NoAnswer;
    };
    if first != 0x55 {
        return WakeVerdict::WrongSync;
    }
    if findings.key_bytes.is_none() {
        return WakeVerdict::KeyBytesMissing;
    }
    let Some(reply_after) = findings.reply_after_us else {
        return WakeVerdict::ProtocolError;
    };
    match findings.reply_echo {
        None => return WakeVerdict::NoEcho,
        Some(false) => return WakeVerdict::EchoBroken,
        Some(true) => {}
    }
    if findings.heard_after_reply {
        return WakeVerdict::ProtocolError;
    }
    let hidden = u64::from(latency_ms.unwrap_or(0)) * 1000;
    if reply_after > REPLY_MAX_US || reply_after + hidden > REPLY_MAX_US {
        WakeVerdict::ReplyTooLate
    } else {
        WakeVerdict::ReplyRejected
    }
}

/// Things that worked but were close to a limit, as plain sentences.
pub fn warnings(findings: &WakeFindings, latency_ms: Option<u32>) -> Vec<String> {
    let ms = |us: u64| format!("{:.1}", us as f64 / 1000.0);
    let mut out = Vec::new();
    if findings.worst_edge_error_us > BIT_TOLERANCE_US
        && findings.worst_edge_error_us <= BIT_LIMIT_US
    {
        out.push(
            l(
                "Die Bits der Weckadresse weichen bis zu %1 ms ab (erlaubt sind 10 ms). Es hat gereicht, aber knapp.",
                "The bits of the wake-up address are up to %1 ms off (10 ms are allowed). It was enough, but only just.",
            )
            .replace("%1", &ms(findings.worst_edge_error_us)),
        );
    }
    if let Some(reply) = findings.reply_after_us {
        let hidden = u64::from(latency_ms.unwrap_or(0)) * 1000;
        if reply + hidden > 40_000 && reply + hidden <= REPLY_MAX_US {
            out.push(
                l(
                    "Die Antwort auf die Schlüsselbytes ging nach %1 ms raus, das Steuergerät wartet höchstens 50 ms.",
                    "The reply to the key bytes went out after %1 ms, the control unit waits at most 50 ms.",
                )
                .replace("%1", &ms(reply + hidden)),
            );
        }
    }
    out.extend(findings.reply_round_trip_us.and_then(slow_echo_warning));
    out
}

/// A hint when an echo took much longer than the byte itself (about 1 ms
/// at 9600 baud): the USB chip holds received bytes back.
pub fn slow_echo_warning(round_trip_us: u64) -> Option<String> {
    (round_trip_us > 10_000).then(|| {
        l(
            "Ein Byte braucht %1 ms bis zum Echo, eigentlich reicht 1 ms. Der USB-Chip hält die Bytes zurück (Latenz-Timer).",
            "A byte takes %1 ms until its echo, 1 ms would do. The USB chip holds the bytes back (latency timer).",
        )
        .replace("%1", &format!("{:.1}", round_trip_us as f64 / 1000.0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line that answers with a script and keeps its own clock.
    struct ScriptLine {
        now: u64,
        breaks: Vec<(u64, bool)>,
        rx: VecDeque<(u64, u8)>,
        written: Vec<u8>,
        break_delay_us: u64,
        baud: u32,
    }

    impl ScriptLine {
        fn new() -> Self {
            Self {
                now: 0,
                breaks: Vec::new(),
                rx: VecDeque::new(),
                written: Vec::new(),
                break_delay_us: 1_000,
                baud: 9600,
            }
        }
    }

    impl RawKLine for ScriptLine {
        fn now_us(&self) -> u64 {
            self.now
        }
        fn sleep_until_us(&mut self, deadline_us: u64) {
            self.now = self.now.max(deadline_us);
        }
        fn set_break(&mut self, active: bool) -> bool {
            self.now += self.break_delay_us;
            self.breaks.push((self.now, active));
            true
        }
        fn flush_input(&mut self) {}
        fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
            match self.rx.front() {
                Some(&(at, _)) if at <= self.now + timeout_ms * 1000 => {
                    self.now = self.now.max(at);
                    vec![self.rx.pop_front().unwrap().1]
                }
                _ => {
                    self.now += timeout_ms * 1000;
                    Vec::new()
                }
            }
        }
        fn write(&mut self, data: &[u8]) -> bool {
            self.written.extend_from_slice(data);
            for &b in data {
                self.rx.push_front((self.now + 1_000, b));
            }
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

    #[test]
    fn the_address_is_clocked_on_its_edges_only() {
        let mut k = TimedKLine::new(ScriptLine::new(), 9600);
        let trace = k.clock_address(0x01);
        // 0x01: start 0, d0 1, d1..d6 0, parity 0, stop 1: four edges.
        let edges = trace.bits.iter().filter(|b| b.edge_us.is_some()).count();
        assert_eq!(edges, 4);
        assert_eq!(trace.bits.len(), 10);
        let findings = analyze(&trace);
        assert_eq!(findings.phases.len(), 4);
        assert_eq!(findings.phases[2].bits, 7);
        assert!(findings.worst_edge_error_us <= 1, "{findings:?}");
        assert_eq!(findings.mean_on_call_us, 1_000);
    }

    #[test]
    fn a_slow_break_call_shows_as_edge_error_and_compensation_removes_it() {
        let mut line = ScriptLine::new();
        line.break_delay_us = 30_000;
        let mut k = TimedKLine::new(line, 9600);
        let findings = analyze(&k.clock_address(0x01));
        // Every call is equally late, so the edges keep their distance.
        assert!(findings.worst_edge_error_us <= 1);
        assert_eq!(findings.max_call_us, 30_000);

        k.set_strategy(WakeStrategy::Compensated {
            on_lead_us: 30_000,
            off_lead_us: 30_000,
        });
        let trace = k.clock_address(0x01);
        let findings = analyze(&trace);
        assert!(findings.worst_edge_error_us <= 1);
        // With the lead the first edge lands on its target.
        assert_eq!(trace.bits[0].edge_us, Some(0));
    }

    #[test]
    fn the_echo_probe_tells_a_powered_line_from_none() {
        let mut k = TimedKLine::new(ScriptLine::new(), 9600);
        let probe = k.echo_probe(&[0x55, 0xAA]);
        assert_eq!(probe.state(), EchoState::Clean);
        assert_eq!(probe.round_trip_us.len(), 2);
        let empty = EchoProbe {
            sent: vec![0x55],
            ..EchoProbe::default()
        };
        assert_eq!(empty.state(), EchoState::None);
        let broken = EchoProbe {
            sent: vec![0x55],
            received: vec![0x45],
            round_trip_us: vec![1_000],
        };
        assert_eq!(broken.state(), EchoState::Broken);
    }

    fn trace_with(events: Vec<LineEvent>) -> WakeTrace {
        let mut k = TimedKLine::new(ScriptLine::new(), 9600);
        let mut trace = k.clock_address(0x01);
        trace.events = events;
        trace
    }

    fn rx(at_ms: u64, byte: u8) -> LineEvent {
        LineEvent::Received {
            at_us: at_ms * 1000,
            byte,
        }
    }

    fn tx(at_ms: u64, byte: u8) -> LineEvent {
        LineEvent::Sent {
            at_us: at_ms * 1000,
            byte,
            call_us: 100,
        }
    }

    #[test]
    fn the_verdicts_follow_the_bytes_after_the_address() {
        let silent = trace_with(vec![]);
        assert_eq!(
            judge(&silent, &analyze(&silent), false, None),
            WakeVerdict::NoAnswer
        );

        let garbage = trace_with(vec![rx(40, 0x00), rx(45, 0x5D)]);
        let f = analyze(&garbage);
        assert_eq!(f.first_byte, Some(0x5D));
        assert_eq!(judge(&garbage, &f, false, None), WakeVerdict::WrongSync);

        let no_keys = trace_with(vec![rx(40, 0x55)]);
        assert_eq!(
            judge(&no_keys, &analyze(&no_keys), false, None),
            WakeVerdict::KeyBytesMissing
        );

        let base = vec![rx(40, 0x55), rx(50, 0x01), rx(55, 0x8A)];
        let mut late = base.clone();
        late.extend([tx(110, 0x75), rx(112, 0x75)]);
        let late = trace_with(late);
        let f = analyze(&late);
        assert_eq!(f.reply_after_us, Some(55_000));
        assert_eq!(judge(&late, &f, false, None), WakeVerdict::ReplyTooLate);

        let mut rejected = base.clone();
        rejected.extend([tx(86, 0x75), rx(88, 0x75)]);
        let rejected = trace_with(rejected);
        let f = analyze(&rejected);
        assert_eq!(
            judge(&rejected, &f, false, Some(1)),
            WakeVerdict::ReplyRejected
        );
        // The same 31 ms with a 40 ms latency timer: the byte really came
        // up to 40 ms earlier than it was read, so the reply was too late.
        assert_eq!(
            judge(&rejected, &f, false, Some(40)),
            WakeVerdict::ReplyTooLate
        );

        let mut broken = base.clone();
        broken.extend([tx(86, 0x75), rx(88, 0x65)]);
        let broken = trace_with(broken);
        assert_eq!(
            judge(&broken, &analyze(&broken), false, None),
            WakeVerdict::EchoBroken
        );

        let mut mute = base.clone();
        mute.push(tx(86, 0x75));
        let mute = trace_with(mute);
        assert_eq!(
            judge(&mute, &analyze(&mute), false, None),
            WakeVerdict::NoEcho
        );

        let mut cut = base;
        cut.extend([tx(86, 0x75), rx(88, 0x75), rx(110, 0x0F)]);
        let cut = trace_with(cut);
        assert_eq!(
            judge(&cut, &analyze(&cut), false, None),
            WakeVerdict::ProtocolError
        );
        assert_eq!(
            judge(&cut, &analyze(&cut), true, None),
            WakeVerdict::Connected
        );
    }

    #[test]
    fn uneven_bits_without_an_answer_are_a_timing_problem() {
        let mut trace = trace_with(vec![]);
        // Move the second edge 35 ms late.
        if let Some(edge) = trace.bits[1].edge_us.as_mut() {
            *edge += 35_000;
        }
        let f = analyze(&trace);
        assert!(f.worst_edge_error_us >= 34_000);
        assert_eq!(judge(&trace, &f, false, None), WakeVerdict::TimingUnstable);
    }

    #[test]
    fn leading_zero_bytes_are_the_echo_of_the_break() {
        let mut line = ScriptLine::new();
        line.rx.push_back((2_400_000, 0x00));
        line.rx.push_back((2_420_000, 0x55));
        let mut k = TimedKLine::new(line, 9600);
        assert!(k.slow_init(0x01));
        assert_eq!(k.read_byte(2000), Some(0x55));
        let trace = k.take_trace().unwrap();
        assert_eq!(trace.address_echo, 1);
        assert_eq!(analyze(&trace).first_byte, Some(0x55));
    }
}
