//! Performance of the live loop below the app: the ELM327 driver and the
//! OBD-II layer against the demo car, once answering at once and once behind
//! a simulated serial line with the delays of a real adapter
//! (`caruspex_demo::SimulatedSerial`). docs/performance.md explains the
//! numbers and the budgets.
//!
//! The guards run with `cargo test`; their budgets are generous so a slow CI
//! machine passes. The full report and the long soak are ignored tests:
//!
//!     cargo test -p caruspex-protocols --release --test perf -- --ignored --nocapture
//!
//! `CARUSPEX_SOAK_SECONDS=600` makes the soak last ten minutes.

#[path = "perf_support/alloc.rs"]
mod alloc;

use std::time::{Duration, Instant};

use alloc::{Counting, Uncounted, snapshot};
use caruspex_data::pids::default_live_pids;
use caruspex_demo::{DemoAdapter, DemoOptions, DemoTransport, LineTiming, SimulatedSerial};
use caruspex_link::{Elm327, Transport};
use caruspex_protocols::obd2::Obd2;

#[global_allocator]
static COUNTING: Counting = Counting;

fn options(adapter: DemoAdapter) -> DemoOptions {
    DemoOptions {
        adapter,
        latency_scale: 0.0,
        engine_start_seconds: 0.0,
        ..DemoOptions::default()
    }
}

/// Connected like the worker does it: CAN 11/500, headers on.
fn connect(transport: Box<dyn Transport>) -> (Elm327, Duration) {
    let mut elm = Elm327::new();
    // The worker logs every line by default (Settings, "log raw"); the log
    // function is part of the cost.
    elm.set_log(Box::new(|line: &str| {
        std::hint::black_box(line);
    }));
    let t0 = Instant::now();
    elm.connect(transport, "6").expect("demo car answers");
    elm.set_headers(true);
    (elm, t0.elapsed())
}

fn instant(adapter: DemoAdapter) -> Box<dyn Transport> {
    Box::new(Uncounted(DemoTransport::new(options(adapter))))
}

fn serial(adapter: DemoAdapter, timing: LineTiming) -> Box<dyn Transport> {
    Box::new(Uncounted(SimulatedSerial::demo(options(adapter), timing)))
}

fn pids(n: usize) -> Vec<i32> {
    default_live_pids()[..n.min(default_live_pids().len())].to_vec()
}

/// One round of `pids` as the worker's live tick reads it.
fn round(elm: &mut Elm327, pids: &[i32]) -> usize {
    Obd2::new(elm).read_live_batch(pids, false).len()
}

struct RoundStats {
    per_round: Duration,
    values_per_round: f64,
    allocs_per_round: f64,
    bytes_per_round: f64,
}

fn measure_rounds(elm: &mut Elm327, pids: &[i32], rounds: usize) -> RoundStats {
    // Warm up: the batch memory and the lazily built tables settle.
    for _ in 0..5 {
        round(elm, pids);
    }
    let before = snapshot();
    let t0 = Instant::now();
    let mut values = 0;
    for _ in 0..rounds {
        values += round(elm, pids);
    }
    let elapsed = t0.elapsed();
    let used = snapshot().since(before);
    RoundStats {
        per_round: elapsed / rounds as u32,
        values_per_round: values as f64 / rounds as f64,
        allocs_per_round: used.allocs as f64 / rounds as f64,
        bytes_per_round: used.bytes as f64 / rounds as f64,
    }
}

// --- Guards -----------------------------------------------------------------

#[test]
fn a_live_round_allocates_little() {
    let (mut elm, _) = connect(instant(DemoAdapter::CloneV15));
    let six = pids(6);
    let stats = measure_rounds(&mut elm, &six, 200);
    assert!(stats.values_per_round >= 5.0, "{}", stats.values_per_round);
    // About 110 now, 314 before (docs/performance.md); allocation counts do
    // not depend on the machine. The budget leaves room for a changed PID
    // table, not for a return of the per-line copies.
    assert!(
        stats.allocs_per_round < 220.0,
        "allocations per round of 6 PIDs: {:.0}",
        stats.allocs_per_round
    );
}

#[test]
fn the_live_loop_does_not_grow() {
    let (mut elm, _) = connect(instant(DemoAdapter::CloneV15));
    let six = pids(6);
    for _ in 0..200 {
        round(&mut elm, &six);
    }
    let before = snapshot();
    for _ in 0..3000 {
        round(&mut elm, &six);
    }
    let grown = snapshot().since(before).live;
    assert!(
        grown < 64 * 1024,
        "live heap grew by {grown} bytes over 3000 rounds"
    );
}

#[test]
fn a_round_on_a_slow_line_costs_one_request_per_six_pids() {
    let line = SimulatedSerial::demo(options(DemoAdapter::CloneV15), LineTiming::elm327_38400());
    let counts = line.stats();
    let (mut elm, _) = connect(Box::new(Uncounted(line)));
    let six = pids(6);
    let before = counts.get();
    let stats = measure_rounds(&mut elm, &six, 10);
    let after = counts.get();
    // 15 rounds (with the warm-up), one request each. The time depends on
    // the machine, the counts do not.
    assert_eq!(after.requests - before.requests, 15);
    // All but the first carry the answer count, except the plain one every
    // 32nd request.
    assert!(after.hinted - before.hinted >= 13, "{before:?} {after:?}");
    // About 35 ms with the answer count, 65 without, 150 means requests
    // went one by one.
    assert!(
        stats.per_round < Duration::from_millis(150),
        "{:?}",
        stats.per_round
    );
}

// --- Report -----------------------------------------------------------------

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

#[test]
#[ignore = "report, run with --ignored --nocapture"]
fn report() {
    println!();
    println!("| measurement | value |");
    println!("|---|---|");
    for (name, timing) in [
        ("38400", LineTiming::elm327_38400()),
        ("115200", LineTiming::elm327_115200()),
    ] {
        let (_, connect_time) = connect(serial(DemoAdapter::CloneV15, timing));
        println!(
            "| connect, simulated ELM327 at {name} baud | {:.0} ms |",
            ms(connect_time)
        );
    }
    let (_, connect_time) = connect(Box::new(Uncounted(DemoTransport::new(DemoOptions {
        engine_start_seconds: 0.0,
        ..DemoOptions::default()
    }))));
    println!(
        "| connect, demo car with its own delays | {:.0} ms |",
        ms(connect_time)
    );

    for (adapter, label) in [
        (DemoAdapter::CloneV15, "ELM327 clone"),
        (DemoAdapter::ObdLinkEx, "OBDLink EX (STN batch)"),
    ] {
        // 12 is the whole default list of the live page.
        for n in [1usize, 6, 12] {
            let (mut elm, _) = connect(serial(adapter, LineTiming::elm327_38400()));
            let list = pids(n);
            let s = measure_rounds(&mut elm, &list, 20);
            println!(
                "| {label} 38400, round of {n} PIDs | {:.1} ms, {:.0} values/s |",
                ms(s.per_round),
                s.values_per_round / s.per_round.as_secs_f64()
            );
        }
    }

    let (mut elm, _) = connect(instant(DemoAdapter::CloneV15));
    let s = measure_rounds(&mut elm, &pids(6), 2000);
    println!(
        "| CPU per round of 6 PIDs, adapter answering at once | {:.1} us |",
        s.per_round.as_secs_f64() * 1e6
    );
    println!(
        "| allocations per round of 6 PIDs | {:.0} ({:.0} bytes) |",
        s.allocs_per_round, s.bytes_per_round
    );
}

#[test]
#[ignore = "soak, run with --ignored --nocapture; CARUSPEX_SOAK_SECONDS sets the length"]
fn soak() {
    let seconds: u64 = std::env::var("CARUSPEX_SOAK_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let (mut elm, _) = connect(instant(DemoAdapter::CloneV15));
    let six = pids(6);
    for _ in 0..200 {
        round(&mut elm, &six);
    }
    let start = snapshot();
    let t0 = Instant::now();
    let mut rounds = 0u64;
    let mut next_print = Duration::from_secs(10);
    while t0.elapsed() < Duration::from_secs(seconds) {
        round(&mut elm, &six);
        rounds += 1;
        if t0.elapsed() >= next_print {
            println!(
                "{:>5} s  {rounds:>9} rounds  live heap {:+} bytes",
                next_print.as_secs(),
                snapshot().since(start).live
            );
            next_print += Duration::from_secs(10);
        }
    }
    let grown = snapshot().since(start).live;
    println!("soak: {rounds} rounds in {seconds} s, live heap {grown:+} bytes");
    assert!(grown < 256 * 1024, "{grown}");
}
