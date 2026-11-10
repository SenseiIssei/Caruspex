//! Built-in demo vehicle: a simulated ELM327 with a simulated car behind it.
//!
//! [`DemoTransport`] plugs into the ELM327 driver like a serial port or a
//! Wi-Fi dongle, but everything happens in-process. It plays the adapter
//! character by character the way a real one talks: echo until ATE0, the '>'
//! prompt, "SEARCHING..." on automatic protocol search, "NO DATA", "?" for
//! unknown commands, and CAN frames with ISO-TP multi-frame responses, with
//! or without headers.
//!
//! Behind the adapter sits a fictional modern petrol car on CAN (11 bit,
//! 500 kbaud) with two control units: engine (answers on 7E8) and automatic
//! transmission (7E9). Its state is a pure function of the elapsed time since
//! "ignition on": the engine starts after a few seconds, warms up at idle and
//! then repeats a drive cycle (accelerate, cruise, brake, stand). Fault codes,
//! freeze frame, readiness, on-board monitoring and identification behave
//! consistently, including clearing with mode 04.
//!
//! Addressed physically (ATSH, ATCRA) the car also speaks UDS (ISO 14229):
//! tester present, identification (22 F187 ...), fault memory (19 02) and
//! clearing (14 FF FF FF). Besides engine (7E0) and gearbox (7E1) four units
//! answer only this way: ABS 760, airbag 737, instrument cluster 720 and
//! climate 744, each on request + 8. The engine also has a few maker values
//! under service 22 (oil level, oil temperature, starts, a multi-frame block),
//! read by the demo pack of the custom values (data/pids/caruspex-demo.csv).
//!
//! For tests the clock can be injected (see [`DemoClock`]) and the answer
//! delay switched off, so every response is deterministic.

#![forbid(unsafe_code)]

mod adapter;
mod codec;
mod sim;

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use caruspex_i18n::l;
use caruspex_link::Transport;

use adapter::Impl;

/// Which adapter the demo pretends to be. The default is the most common one
/// in the wild: a clone that reports "v1.5", a version the original
/// manufacturer never released. The others exist mainly so the adapter probe
/// can be tested against every verdict it knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DemoAdapter {
    /// "ELM327 v1.5", command set of v1.4
    #[default]
    CloneV15,
    /// behaves like a real ELM327 v1.4b
    GenuineV14b,
    /// claims v2.1, command set of v1.4
    CloneV21,
    /// claims v2.1 and answers "OK" even to commands it does not know
    CloneOkToAll,
    /// STN chip (OBDLink), ELM327 v1.4b compatible plus ST commands
    ObdLinkStn,
}

/// A source of "seconds since ignition on".
pub type ClockFn = Arc<dyn Fn() -> f64 + Send + Sync>;

/// A clock the test (or the UI) moves by hand. Clones share the same time.
#[derive(Clone, Debug)]
pub struct DemoClock(Arc<AtomicU64>);

impl DemoClock {
    pub fn new(start: f64) -> Self {
        DemoClock(Arc::new(AtomicU64::new(start.to_bits())))
    }

    pub fn get(&self) -> f64 {
        f64::from_bits(self.0.load(Ordering::SeqCst))
    }

    pub fn set(&self, seconds: f64) {
        self.0.store(seconds.to_bits(), Ordering::SeqCst);
    }

    pub fn advance(&self, seconds: f64) {
        self.set(self.get() + seconds);
    }

    /// The function to put into [`DemoOptions::clock`].
    pub fn source(&self) -> ClockFn {
        let clock = self.clone();
        Arc::new(move || clock.get())
    }
}

#[derive(Clone)]
pub struct DemoOptions {
    pub adapter: DemoAdapter,

    /// Seconds since "ignition on". `None`: real time since construction.
    /// Tests pass a [`DemoClock::source`] they advance themselves.
    pub clock: Option<ClockFn>,

    /// Scales the simulated answer delays (a few milliseconds per request).
    /// 0 answers at once, which keeps tests fast and deterministic.
    pub latency_scale: f64,

    /// Seconds after ignition on at which the engine is started.
    pub engine_start_seconds: f64,

    /// Whether the transmission control unit (7E9) is present.
    pub transmission_ecu: bool,
}

impl Default for DemoOptions {
    fn default() -> Self {
        DemoOptions {
            adapter: DemoAdapter::CloneV15,
            clock: None,
            latency_scale: 1.0,
            engine_start_seconds: 4.0,
            transmission_ecu: true,
        }
    }
}

impl DemoOptions {
    /// Options driven by a hand-moved clock, without answer delays.
    pub fn with_clock(clock: &DemoClock) -> Self {
        DemoOptions {
            clock: Some(clock.source()),
            latency_scale: 0.0,
            ..DemoOptions::default()
        }
    }
}

pub(crate) struct Chunk {
    pub ready_at: Instant,
    pub text: Vec<u8>,
}

pub struct DemoTransport {
    d: Impl,
}

impl DemoTransport {
    pub fn new(options: DemoOptions) -> Self {
        let mut d = Impl::new(options);
        // Power-on greeting; the ELM327 driver clears it away like a real one.
        let greeting = format!("{}\r\r>", d.profile.version);
        d.emit(greeting.into_bytes(), 0.0);
        DemoTransport { d }
    }

    /// The vehicle identification number of the demo car (17 characters,
    /// valid check digit, fictional manufacturer code).
    pub fn vin() -> &'static str {
        adapter::VIN
    }
}

impl Default for DemoTransport {
    fn default() -> Self {
        DemoTransport::new(DemoOptions::default())
    }
}

impl Transport for DemoTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        let d = &mut self.d;
        for &ch in data {
            if d.monitoring {
                // Any character ends monitoring; the rest of that line is
                // lost, which is why programs send a lone carriage return
                // first.
                d.monitoring = false;
                let end = d.line_end();
                let text = format!("STOPPED{end}{end}>");
                d.emit(text.into_bytes(), 1.0);
                d.discard_line = ch != b'\r';
                continue;
            }
            if d.discard_line {
                if ch == b'\r' {
                    d.discard_line = false;
                }
                continue;
            }
            if ch == b'\r' {
                let line = std::mem::take(&mut d.input_line);
                d.handle_line(&line);
            } else if ch != b'\n' && ch != 0 && d.input_line.len() < 64 {
                d.input_line.push(ch);
            }
        }
        true
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let d = &mut self.d;
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            let mut result = Vec::new();
            let now = Instant::now();
            while d.output.front().is_some_and(|c| c.ready_at <= now) {
                if let Some(chunk) = d.output.pop_front() {
                    result.extend_from_slice(&chunk.text);
                }
            }
            if !result.is_empty() {
                return result;
            }
            if now >= deadline {
                return Vec::new();
            }
            let mut wake = deadline;
            if let Some(front) = d.output.front()
                && front.ready_at < wake
            {
                wake = front.ready_at;
            }
            std::thread::sleep(wake.saturating_duration_since(Instant::now()));
        }
    }

    fn flush_input(&mut self) {
        // Only what the adapter has already sent is thrown away, like a
        // serial port buffer; answers still on their way arrive later.
        let d = &mut self.d;
        let now = Instant::now();
        while d.output.front().is_some_and(|c| c.ready_at <= now) {
            d.output.pop_front();
        }
    }

    fn name(&self) -> String {
        l("Demo-Fahrzeug (eingebaut)", "Demo vehicle (built-in)").to_string()
    }
}

pub(crate) type Output = VecDeque<Chunk>;
