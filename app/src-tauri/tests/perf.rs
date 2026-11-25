//! Performance of the worker as the app runs it: the real loop with its
//! timers, the live ticks, the events into a sink that serializes every
//! payload to JSON like Tauri does before it goes to the window. The adapter
//! is the demo car, answering at once or behind a simulated serial line
//! (`caruspex_demo::SimulatedSerial`). docs/performance.md explains the
//! numbers and the budgets.
//!
//! The guards run with `cargo test`; their budgets are generous so a slow CI
//! machine passes. The report and the soak are ignored tests:
//!
//!     cargo test -p caruspex-app --release --test perf -- --ignored --nocapture --test-threads 1
//!
//! `CARUSPEX_SOAK_SECONDS=600` makes the soak last ten minutes.

#[path = "../../../crates/caruspex-protocols/tests/perf_support/alloc.rs"]
mod alloc;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use alloc::{Counting, Snapshot, Uncounted, paused, snapshot};
use caruspex_app_lib::bench::{
    AppSettings, Job, LiveEvent, LiveRow, Sink, Worker, init_english_names,
};
use caruspex_data::pids::default_live_pids;
use caruspex_demo::{DemoOptions, DemoTransport, LineStats, LineTiming, SimulatedSerial};
use caruspex_link::Transport;
use serde_json::Value;

#[global_allocator]
static COUNTING: Counting = Counting;

// One measurement at a time: they compare timings.
static SERIAL: Mutex<()> = Mutex::new(());

/// Live values before the measured window starts: the batch memory and
/// the lazily built tables settle first.
const WARM_UP_VALUES: u64 = 60;

#[derive(Default)]
struct State {
    connected: Option<Instant>,
    ready: Option<Instant>,
    /// Live values, and the events they came in.
    live: u64,
    live_events: u64,
    logs: u64,
    other: u64,
    json_bytes: u64,
    window_start: Option<Mark>,
    window_end: Option<Mark>,
    last_prompt: Option<Instant>,
    latencies: Vec<Duration>,
}

#[derive(Clone, Copy)]
struct Mark {
    at: Instant,
    memory: Snapshot,
    live: u64,
    live_events: u64,
    logs: u64,
    other: u64,
    json_bytes: u64,
}

struct Probe {
    started: Instant,
    window: Duration,
    stats: Option<LineStats>,
    state: Mutex<State>,
    done: AtomicBool,
}

impl Sink for Probe {
    fn emit(&self, event: &str, payload: Value) {
        // What Tauri does with every event before it reaches the window.
        let text = serde_json::to_string(&payload).unwrap_or_default();
        self.record(event, text, 0);
    }

    // The window's sink (TauriSink) hands the rows to Tauri as they are,
    // which serializes them once.
    fn live_values(&self, rows: &[LiveRow]) {
        let text = serde_json::to_string(&LiveEvent { rows }).unwrap_or_default();
        self.record("live_values", text, rows.len() as u64);
    }
}

impl Probe {
    fn record(&self, event: &str, text: String, values: u64) {
        let now = Instant::now();
        let prompt = self.stats.as_ref().and_then(|s| s.get().last_prompt);
        paused(|| {
            let mut s = self.state.lock().unwrap();
            s.json_bytes += text.len() as u64;
            match event {
                "live_values" => {
                    s.live += values;
                    s.other += 1;
                    s.live_events += 1;
                    // The first value after a new answer: how long from the
                    // adapter's prompt to the event.
                    if let Some(p) = prompt
                        && s.last_prompt != Some(p)
                    {
                        s.last_prompt = Some(p);
                        if s.window_start.is_some() {
                            s.latencies.push(now.saturating_duration_since(p));
                        }
                    }
                }
                "log_line" => s.logs += 1,
                "connected" => {
                    s.other += 1;
                    s.connected = Some(now);
                }
                // The last event of the readout after connecting: status,
                // faults, then the supported PIDs.
                "supported_pids" => {
                    s.other += 1;
                    s.ready.get_or_insert(now);
                }
                _ => s.other += 1,
            }
            let mark = Mark {
                at: now,
                memory: snapshot(),
                live: s.live,
                live_events: s.live_events,
                logs: s.logs,
                other: s.other,
                json_bytes: s.json_bytes,
            };
            if s.window_start.is_none() && s.live >= WARM_UP_VALUES {
                s.window_start = Some(mark);
            } else if let Some(start) = s.window_start {
                s.window_end = Some(mark);
                if now.duration_since(start.at) >= self.window {
                    self.done.store(true, Ordering::SeqCst);
                }
            }
        });
        drop(text);
    }
}

struct Report {
    connect: Duration,
    ready: Duration,
    seconds: f64,
    values_per_s: f64,
    logs_per_s: f64,
    live_events_per_s: f64,
    events_per_s: f64,
    json_kib_per_s: f64,
    allocs_per_value: f64,
    bytes_per_value: f64,
    heap_growth: i64,
    latency_median: Duration,
    latency_p95: Duration,
    latency_max: Duration,
}

fn settings() -> AppSettings {
    AppSettings::default()
}

/// Connects the worker to `transport`, starts the live values and lets the
/// loop run until `window` of live values is measured (or `limit` passed).
fn run(
    transport: Box<dyn Transport>,
    stats: Option<LineStats>,
    pids: Vec<i32>,
    window: Duration,
    values: AppSettings,
) -> Report {
    let probe = Arc::new(Probe {
        started: Instant::now(),
        window,
        stats: stats.clone(),
        state: Mutex::new(State::default()),
        done: AtomicBool::new(false),
    });
    let (tx, rx) = channel();
    tx.send(Job::OpenTransport {
        transport,
        protocol_order: "6".into(),
    })
    .unwrap();
    tx.send(Job::StartLive(pids)).unwrap();
    let watch = probe.clone();
    let limit = window + Duration::from_secs(30);
    let stopper = std::thread::spawn(move || {
        let t0 = Instant::now();
        while !watch.done.load(Ordering::SeqCst) && t0.elapsed() < limit {
            std::thread::sleep(Duration::from_millis(5));
        }
        let _ = tx.send(Job::Shutdown);
    });
    let mut worker = Worker::new(probe.clone(), Arc::new(AtomicBool::new(false)));
    worker.apply_settings(values);
    // The real loop of the worker thread, here on the test thread so the
    // allocation counter sees it.
    worker.run(rx);
    stopper.join().unwrap();

    let s = probe.state.lock().unwrap();
    let start = s.window_start.expect("live values arrived");
    let end = s.window_end.expect("live values went on");
    let seconds = end.at.duration_since(start.at).as_secs_f64();
    let used = end.memory.since(start.memory);
    let values = (end.live - start.live).max(1) as f64;
    let logs = end.logs - start.logs;
    let mut lat = s.latencies.clone();
    lat.sort();
    let pick = |q: f64| {
        lat.get(((lat.len() as f64 - 1.0) * q).round() as usize)
            .copied()
            .unwrap_or_default()
    };
    Report {
        connect: s.connected.unwrap().duration_since(probe.started),
        ready: s.ready.unwrap_or(start.at).duration_since(probe.started),
        seconds,
        values_per_s: values / seconds,
        logs_per_s: logs as f64 / seconds,
        live_events_per_s: (end.live_events - start.live_events) as f64 / seconds,
        events_per_s: (logs + (end.other - start.other)) as f64 / seconds,
        json_kib_per_s: (end.json_bytes - start.json_bytes) as f64 / 1024.0 / seconds,
        allocs_per_value: used.allocs as f64 / values,
        bytes_per_value: used.bytes as f64 / values,
        heap_growth: used.live,
        latency_median: pick(0.5),
        latency_p95: pick(0.95),
        latency_max: lat.last().copied().unwrap_or_default(),
    }
}

fn instant_demo() -> Box<dyn Transport> {
    Box::new(Uncounted(DemoTransport::new(DemoOptions {
        latency_scale: 0.0,
        engine_start_seconds: 0.0,
        ..DemoOptions::default()
    })))
}

fn serial(timing: LineTiming) -> (Box<dyn Transport>, LineStats) {
    let line = SimulatedSerial::demo(
        DemoOptions {
            engine_start_seconds: 0.0,
            ..DemoOptions::default()
        },
        timing,
    );
    let stats = line.stats();
    (Box::new(Uncounted(line)), stats)
}

fn six() -> Vec<i32> {
    default_live_pids()[..6].to_vec()
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn print(label: &str, r: &Report) {
    println!(
        "| {label}: connected / readout done | {:.0} ms / {:.0} ms |",
        ms(r.connect),
        ms(r.ready)
    );
    println!(
        "| {label}: live values per second | {:.0} (over {:.1} s) |",
        r.values_per_s, r.seconds
    );
    println!(
        "| {label}: events per second (live values / log lines) | {:.0} ({:.0} / {:.0}) |",
        r.events_per_s, r.live_events_per_s, r.logs_per_s
    );
    println!(
        "| {label}: JSON to the window | {:.1} KiB/s |",
        r.json_kib_per_s
    );
    println!(
        "| {label}: answer to event, median / p95 / max | {:.2} / {:.2} / {:.2} ms |",
        ms(r.latency_median),
        ms(r.latency_p95),
        ms(r.latency_max)
    );
    println!(
        "| {label}: allocations per live value | {:.1} ({:.0} bytes) |",
        r.allocs_per_value, r.bytes_per_value
    );
    println!(
        "| {label}: heap growth over the window | {:+} bytes |",
        r.heap_growth
    );
}

// --- Guards -----------------------------------------------------------------

#[test]
fn the_live_loop_keeps_to_the_poll_interval_and_allocates_little() {
    let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let r = run(
        instant_demo(),
        None,
        six(),
        Duration::from_secs(2),
        settings(),
    );
    // 20 ms between ticks and six values per tick: at most 300 values a
    // second, whatever the adapter can do.
    assert!(r.values_per_s < 330.0, "{} values/s", r.values_per_s);
    assert!(r.values_per_s > 30.0, "{} values/s", r.values_per_s);
    // All values of a tick in one event: a sixth of one event per value.
    assert!(
        r.live_events_per_s * 5.0 < r.values_per_s,
        "{} events/s for {} values/s",
        r.live_events_per_s,
        r.values_per_s
    );
    // About 23 now, 37 while live_value went through a serde_json::Value,
    // 85 before the copies in the hot path went (see docs/performance.md):
    // allocation counts do not depend on the machine.
    assert!(
        r.allocs_per_value < 45.0,
        "{:.1} allocations per live value",
        r.allocs_per_value
    );
    assert!(
        r.heap_growth < 256 * 1024,
        "heap grew by {} bytes",
        r.heap_growth
    );
}

#[test]
fn an_answer_reaches_the_sink_quickly() {
    let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    // Under 1 ms on a desktop. The shared macOS runners of the CI wake
    // threads much later when a neighbour is busy (15 ms and once 50 ms
    // seen), so the best of three runs counts: a real regression such as a
    // sleep in the path is slow in every run, a noisy neighbour is not.
    let mut medians = Vec::new();
    for _ in 0..3 {
        let (transport, stats) = serial(LineTiming::elm327_115200());
        let r = run(
            transport,
            Some(stats),
            six(),
            Duration::from_secs(2),
            settings(),
        );
        assert!(r.values_per_s > 20.0, "{} values/s", r.values_per_s);
        medians.push(r.latency_median);
        if r.latency_median < Duration::from_millis(50) {
            return;
        }
    }
    panic!("median latency in every run at least 50 ms: {medians:?}");
}

// --- Report and soak ----------------------------------------------------------

#[test]
#[ignore = "report, run with --ignored --nocapture --test-threads 1"]
fn report() {
    let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    println!();
    println!("| measurement | value |");
    println!("|---|---|");
    // What the app builds at start before the window opens: the English
    // names of the PID table and the built-in packs.
    let before = snapshot();
    let t0 = Instant::now();
    init_english_names();
    let took = t0.elapsed();
    let used = snapshot().since(before);
    println!(
        "| start: English names of PIDs and packs | {:.2} ms, {} KiB heap |",
        ms(took),
        used.live / 1024
    );
    let window = Duration::from_secs(5);
    let r = run(instant_demo(), None, six(), window, settings());
    print("demo at once, 6 PIDs", &r);
    let r = run(
        instant_demo(),
        None,
        six(),
        window,
        AppSettings {
            log_raw: false,
            ..settings()
        },
    );
    print("demo at once, 6 PIDs, raw log off", &r);
    let (t, s) = serial(LineTiming::elm327_38400());
    let r = run(t, Some(s), six(), window, settings());
    print("ELM327 38400, 6 PIDs", &r);
    let (t, s) = serial(LineTiming::elm327_38400());
    let r = run(t, Some(s), default_live_pids().to_vec(), window, settings());
    print("ELM327 38400, 12 PIDs", &r);
    // An OBDLink: four requests per round trip, 24 values per tick.
    let stn = Box::new(Uncounted(DemoTransport::new(DemoOptions {
        latency_scale: 0.0,
        engine_start_seconds: 0.0,
        adapter: caruspex_demo::DemoAdapter::ObdLinkMxPlus,
        ..DemoOptions::default()
    })));
    let r = run(stn, None, default_live_pids().to_vec(), window, settings());
    print("OBDLink at once, 12 PIDs", &r);
    let demo = Box::new(Uncounted(DemoTransport::new(DemoOptions {
        engine_start_seconds: 0.0,
        ..DemoOptions::default()
    })));
    let r = run(demo, None, six(), window, settings());
    print("demo with its delays, 6 PIDs", &r);
}

#[test]
#[ignore = "soak, run with --ignored --nocapture; CARUSPEX_SOAK_SECONDS sets the length"]
fn soak() {
    let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let seconds: u64 = std::env::var("CARUSPEX_SOAK_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let r = run(
        instant_demo(),
        None,
        default_live_pids().to_vec(),
        Duration::from_secs(seconds),
        settings(),
    );
    print("soak, demo at once, 12 PIDs", &r);
    assert!(r.heap_growth < 1024 * 1024, "{}", r.heap_growth);
}
