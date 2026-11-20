//! The demo car behind a slow serial line, for measurements.
//!
//! [`DemoTransport`] answers within a few milliseconds, which hides what a
//! real adapter costs. [`SimulatedSerial`] puts the time back that a real
//! ELM327 spends: every byte takes ten bits at the baud rate in both
//! directions, the adapter needs a moment per command, the control units a
//! moment per request, and after the last answer the adapter keeps waiting
//! for further answers (its adaptive timeout) unless the request said how
//! many answer lines to expect (the digit after the request, ELM327 v1.3
//! and later). The defaults are in the range of a clone at 38400 baud on a
//! CAN car: 40 to 60 ms per request.
//!
//! Nothing here is used by the app; the benchmarks and the performance tests
//! of the workspace connect through it (see docs/performance.md).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use caruspex_link::Transport;

use crate::{DemoOptions, DemoTransport};

/// How a simulated adapter spends its time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineTiming {
    /// Bits per second of the line; every byte costs ten bits.
    pub baud: u32,
    /// The adapter's own work for an AT or ST command, in milliseconds.
    pub command_ms: f64,
    /// The reset (ATZ, ATWS): a real ELM327 restarts.
    pub reset_ms: f64,
    /// From the end of an OBD request to the first answer frame.
    pub ecu_ms: f64,
    /// Between two answer frames on the bus.
    pub frame_gap_ms: f64,
    /// After the last answer: the adapter waits whether more control units
    /// answer, unless the request named the number of answer lines.
    pub trailing_ms: f64,
    /// The whole wait before NO DATA.
    pub no_data_ms: f64,
}

impl LineTiming {
    /// An ELM327 clone over USB or Bluetooth SPP at 38400 baud.
    pub fn elm327_38400() -> Self {
        LineTiming {
            baud: 38_400,
            command_ms: 3.0,
            reset_ms: 500.0,
            ecu_ms: 12.0,
            frame_gap_ms: 1.0,
            trailing_ms: 30.0,
            no_data_ms: 100.0,
        }
    }

    /// The same adapter at 115200 baud.
    pub fn elm327_115200() -> Self {
        LineTiming {
            baud: 115_200,
            ..Self::elm327_38400()
        }
    }

    /// Times scaled by `factor` (0 answers at once, the bytes still in order).
    pub fn scaled(self, factor: f64) -> Self {
        LineTiming {
            baud: if factor <= 0.0 {
                u32::MAX
            } else {
                (f64::from(self.baud) / factor) as u32
            },
            command_ms: self.command_ms * factor,
            reset_ms: self.reset_ms * factor,
            ecu_ms: self.ecu_ms * factor,
            frame_gap_ms: self.frame_gap_ms * factor,
            trailing_ms: self.trailing_ms * factor,
            no_data_ms: self.no_data_ms * factor,
        }
    }

    fn byte_ms(&self, count: usize) -> f64 {
        count as f64 * 10_000.0 / f64::from(self.baud.max(1))
    }
}

impl Default for LineTiming {
    fn default() -> Self {
        Self::elm327_38400()
    }
}

/// What the simulated line counted so far, readable from outside while the
/// transport sits in a driver.
#[derive(Clone, Debug, Default)]
pub struct LineStats(Arc<Mutex<LineCounts>>);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineCounts {
    /// Command lines received.
    pub commands: u64,
    /// OBD requests among them (hex lines).
    pub requests: u64,
    /// Requests that named the number of answer lines.
    pub hinted: u64,
    /// When the last prompt was ready to be read: the end of the adapter's
    /// answer.
    pub last_prompt: Option<Instant>,
}

impl LineStats {
    pub fn get(&self) -> LineCounts {
        self.0.lock().map(|c| *c).unwrap_or_default()
    }
    fn update(&self, f: impl FnOnce(&mut LineCounts)) {
        if let Ok(mut c) = self.0.lock() {
            f(&mut c);
        }
    }
}

struct Chunk {
    ready_at: Instant,
    bytes: Vec<u8>,
}

/// A transport that answers like the demo car behind a real serial line.
pub struct SimulatedSerial<T: Transport = DemoTransport> {
    inner: T,
    timing: LineTiming,
    line: Vec<u8>,
    pending: VecDeque<Chunk>,
    busy_until: Instant,
    stats: LineStats,
}

impl SimulatedSerial<DemoTransport> {
    /// The demo car (answering at once) behind a line with this timing.
    pub fn demo(options: DemoOptions, timing: LineTiming) -> Self {
        let options = DemoOptions {
            latency_scale: 0.0,
            ..options
        };
        Self::new(DemoTransport::new(options), timing)
    }
}

impl<T: Transport> SimulatedSerial<T> {
    /// `inner` should answer at once; the delays come from `timing`.
    pub fn new(inner: T, timing: LineTiming) -> Self {
        SimulatedSerial {
            inner,
            timing,
            line: Vec::new(),
            pending: VecDeque::new(),
            busy_until: Instant::now(),
            stats: LineStats::default(),
        }
    }

    pub fn stats(&self) -> LineStats {
        self.stats.clone()
    }

    fn at(base: Instant, ms: f64) -> Instant {
        base + Duration::from_micros((ms.max(0.0) * 1000.0) as u64)
    }

    /// Queues `bytes` from `start` on, in pieces of 16 bytes like a USB
    /// serial chip hands them over; returns when the last byte is there.
    fn queue(&mut self, start: Instant, bytes: &[u8]) -> Instant {
        let mut t = start;
        for piece in bytes.chunks(16) {
            t = Self::at(t, self.timing.byte_ms(piece.len()));
            self.pending.push_back(Chunk {
                ready_at: t,
                bytes: piece.to_vec(),
            });
        }
        t
    }

    /// Everything the inner transport says at once (it has no delays).
    fn drain_inner(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let chunk = self.inner.read(0);
            if chunk.is_empty() {
                break;
            }
            out.extend_from_slice(&chunk);
        }
        out
    }

    fn command(&mut self, command: Vec<u8>) {
        let now = Instant::now();
        let tx = Self::at(now, self.timing.byte_ms(command.len() + 1));
        let start = tx.max(self.busy_until);
        let mut line = command.clone();
        line.push(b'\r');
        self.inner.write(&line);
        let answer = self.drain_inner();

        let text: String = command
            .iter()
            .filter(|c| !c.is_ascii_whitespace())
            .map(|&c| (c as char).to_ascii_uppercase())
            .collect();
        let parts: Vec<&str> = text.split('|').collect();
        let is_at = text.starts_with("AT") || text.starts_with("ST");
        let requests = if is_at {
            0
        } else {
            parts
                .iter()
                .filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_hexdigit()))
                .count()
        };
        // Requests that named how many answer lines come (the odd last digit).
        let hinted: Vec<u32> = parts
            .iter()
            .filter(|p| !p.is_empty() && p.len() % 2 == 1)
            .filter_map(|p| p.chars().last().and_then(|c| c.to_digit(16)))
            .collect();

        // The answer lines without echo and prompt: data or status.
        let body: Vec<&[u8]> = answer
            .split(|&b| b == b'\r' || b == b'\n' || b == b'>')
            .filter(|l| !l.is_empty())
            .collect();
        let echo = command.as_slice();
        let data_lines = body.iter().filter(|l| **l != echo).count();
        // Answer frames: the length line in front of a multi-frame answer
        // (three hex digits, headers off) is the adapter's, not a frame.
        let frames = body
            .iter()
            .filter(|l| **l != echo)
            .filter(|l| !(l.len() == 3 && l.iter().all(u8::is_ascii_hexdigit)))
            .count() as u32;
        let hint = if hinted.len() == requests && frames >= hinted.iter().sum::<u32>() {
            requests
        } else {
            0
        };
        let no_data = body.iter().any(|l| l.starts_with(b"NO DATA"));
        let prompt_at_end = answer.last() == Some(&b'>');
        let content = if prompt_at_end {
            &answer[..answer.len() - 1]
        } else {
            &answer[..]
        };

        let mut t = start;
        if requests > 0 {
            if no_data {
                t = Self::at(t, self.timing.no_data_ms);
            } else {
                t = Self::at(
                    t,
                    self.timing.ecu_ms * requests as f64
                        + self.timing.frame_gap_ms * data_lines.saturating_sub(1) as f64,
                );
            }
        } else if text == "ATZ" || text == "ATWS" {
            t = Self::at(t, self.timing.reset_ms);
        } else {
            t = Self::at(t, self.timing.command_ms);
        }
        let end = self.queue(t, content);
        let mut prompt = end;
        if requests > 0 && !no_data && hint < requests {
            // The adaptive wait for answers that may still come.
            prompt = Self::at(end, self.timing.trailing_ms * (requests - hint) as f64);
        }
        if prompt_at_end {
            self.pending.push_back(Chunk {
                ready_at: prompt,
                bytes: vec![b'>'],
            });
        }
        self.busy_until = prompt;
        self.stats.update(|c| {
            c.commands += 1;
            c.requests += requests as u64;
            c.hinted += hinted.len() as u64;
            if prompt_at_end {
                c.last_prompt = Some(prompt);
            }
        });
    }
}

impl<T: Transport> Transport for SimulatedSerial<T> {
    fn write(&mut self, data: &[u8]) -> bool {
        for &b in data {
            if b == b'\r' {
                let command = std::mem::take(&mut self.line);
                self.command(command);
            } else {
                self.line.push(b);
            }
        }
        true
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            // Whatever the inner adapter says on its own (monitoring).
            let extra = self.drain_inner();
            if !extra.is_empty() {
                let now = Instant::now();
                self.queue(now.max(self.busy_until), &extra);
            }
            let now = Instant::now();
            let mut out = Vec::new();
            while self.pending.front().is_some_and(|c| c.ready_at <= now) {
                if let Some(c) = self.pending.pop_front() {
                    out.extend_from_slice(&c.bytes);
                }
            }
            if !out.is_empty() {
                return out;
            }
            if now >= deadline {
                return out;
            }
            let wake = self
                .pending
                .front()
                .map_or(deadline, |c| c.ready_at.min(deadline))
                .min(now + Duration::from_millis(20));
            std::thread::sleep(wake.saturating_duration_since(now));
        }
    }

    fn flush_input(&mut self) {
        let now = Instant::now();
        while self.pending.front().is_some_and(|c| c.ready_at <= now) {
            self.pending.pop_front();
        }
        self.inner.flush_input();
    }

    fn name(&self) -> String {
        format!("simulated serial @ {} baud", self.timing.baud)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_costs_the_line_and_the_adaptive_wait() {
        let mut line = SimulatedSerial::demo(DemoOptions::default(), LineTiming::default());
        // The power-on greeting, as the driver clears it.
        line.flush_input();
        let start = Instant::now();
        line.write(b"ATE0\r");
        let mut answer = Vec::new();
        while !answer.contains(&b'>') && start.elapsed() < Duration::from_secs(2) {
            answer.extend(line.read(100));
        }
        assert!(answer.contains(&b'>'));
        // An unhinted OBD request waits for the trailing timeout.
        let t0 = Instant::now();
        line.write(b"0100\r");
        let mut answer = Vec::new();
        while !answer.contains(&b'>') && t0.elapsed() < Duration::from_secs(2) {
            answer.extend(line.read(100));
        }
        let plain = t0.elapsed();
        assert!(plain >= Duration::from_millis(40), "{plain:?}");
        let stats = line.stats().get();
        assert_eq!(stats.requests, 1);
        assert_eq!(stats.hinted, 0);
    }

    #[test]
    fn scaled_to_zero_it_answers_at_once() {
        let timing = LineTiming::default().scaled(0.0);
        let mut line = SimulatedSerial::demo(DemoOptions::default(), timing);
        line.write(b"ATZ\r");
        let t0 = Instant::now();
        let mut answer = Vec::new();
        while !answer.contains(&b'>') && t0.elapsed() < Duration::from_secs(1) {
            answer.extend(line.read(50));
        }
        assert!(String::from_utf8_lossy(&answer).contains("ELM327"));
        assert!(t0.elapsed() < Duration::from_millis(100));
    }
}
