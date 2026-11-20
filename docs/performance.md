# Speed and memory

Caruspex should feel immediate on a slow adapter and stay small during an hour of live values. This page
describes how that is measured, what the numbers were before and after the work in 0.16, and which tests
fail when it gets clearly worse.

## The harness

Everything runs against the demo car; no test needs an adapter or a car.

- **`caruspex_demo::SimulatedSerial`** (`crates/caruspex-demo/src/serial_sim.rs`) puts the demo car behind a
  line with the delays of a real adapter. `DemoTransport` alone answers within a few milliseconds and hides
  what an ELM327 costs. `LineTiming` sets them: ten bits per byte at the baud rate in both directions, the
  adapter's own time per AT command (3 ms) and per reset (500 ms), the control units' time per request
  (12 ms), 1 ms between answer frames, the adaptive wait after the last answer (30 ms) unless the request
  named the number of answer lines, and 100 ms before NO DATA. `LineTiming::elm327_38400()` and
  `elm327_115200()` are a clone over USB or Bluetooth SPP; `scaled(f)` stretches or removes all delays.
  `LineStats` counts commands, requests and requests with an answer count, and knows when the last prompt
  became readable, which is where the latency measurement starts.
- **A counting allocator** (`crates/caruspex-protocols/tests/perf_support/alloc.rs`), only in the two test
  binaries below. It counts per thread, so parallel tests do not see each other. The simulated adapter and
  car are wrapped in `Uncounted`, so the counts are those of Caruspex: driver, protocols, worker, and the
  JSON that Tauri would send to the window. The bytes a transport returns from `read` are allocated inside
  the uncounted part; a real serial port allocates them once per read as well.
- **`crates/caruspex-protocols/tests/perf.rs`**: the ELM327 driver and `Obd2::read_live_batch` as the live
  tick uses them, connected like the worker connects (CAN 11/500, headers on, a log function as with the
  raw log on).
- **`app/src-tauri/tests/perf.rs`**: the real worker loop (`Worker::run` with its timers, on the test thread so
  the allocator sees it), started with `OpenTransport` and `StartLive` like the Connect and Live pages do. The
  sink serializes every payload to a JSON string, which is what Tauri does with each event. It measures the
  time to "connected" and to the end of the readout after it (status, faults, supported PIDs), live values,
  events and JSON bytes per second, the time from the adapter's prompt to the live value leaving the
  worker, allocations per live value, and the heap over the measured window. The worker is reached through
  `caruspex_app_lib::bench`, a hidden module for these tests only.

## Commands

The guards run with every `cargo test --workspace`. The report and the soak are ignored tests:

```bash
# Driver and protocols: connect, round times, CPU and allocations per round
cargo test -p caruspex-protocols --release --test perf -- --ignored --nocapture report

# The worker as the app runs it
cargo test -p caruspex-app --release --test perf -- --ignored --nocapture --test-threads 1 report

# Ten minute soaks of the live loop (default 30 s), heap printed every 10 s
CARUSPEX_SOAK_SECONDS=600 cargo test -p caruspex-protocols --release --test perf -- --ignored --nocapture soak
CARUSPEX_SOAK_SECONDS=600 cargo test -p caruspex-app --release --test perf -- --ignored --nocapture soak
```

Timings vary by a few percent between runs, more when the machine is busy. Compare runs on the same
machine, before and after a change.

## The guards

| Test | Budget | Measured after |
|---|---|---|
| `caruspex-protocols` `a_live_round_allocates_little` | under 220 allocations per round of 6 PIDs (314 before) | about 110 |
| `caruspex-protocols` `the_live_loop_does_not_grow` | heap grows less than 64 KiB over 3000 rounds | 0 |
| `caruspex-protocols` `a_round_on_a_slow_line_costs_one_request_per_six_pids` | one request per round, the answer count on all but the first, under 150 ms per round at 38400 baud | 15 requests, 14 with the count, about 35 ms |
| `caruspex-app` `the_live_loop_keeps_to_the_poll_interval_and_allocates_little` | 30 to 330 live values per second at the default 20 ms interval, under 70 allocations per live value (85 before), heap growth under 256 KiB in 2 s | about 192 values per second, 37 allocations per value |
| `caruspex-app` `an_answer_reaches_the_sink_quickly` | median under 15 ms from the prompt to the event at 115200 baud | under 1 ms |
| `caruspex-link` `count_hint.rs` | the answer count digit: learned, used, dropped when refused or when it loses answers, relearned every 32nd request | |

The budgets are generous on purpose. They catch a return of per-line copies, a request per PID instead of
per six, a leak or a loop that floods the window, not a few percent.

## Before and after

Windows 11, release build, measured with the commands above. "Before" is 0.16.0 as it was, with only the
harness added.

| Measurement | Before | After |
|---|---|---|
| Round of 6 PIDs, ELM327 clone at 38400 baud | 64.4 ms | 34.3 ms |
| Round of 1 PID, same line | 49.4 ms | 19.3 ms |
| Round of 12 PIDs (two requests), same line | 129.0 ms | 69.2 ms |
| Live values per second in a loop of 6 PIDs, same line | 93 | 175 |
| Connect, simulated ELM327 at 38400 baud | 1122 ms | 630 ms |
| Connect, demo car with its own delays | 589 ms | 89 ms |
| CPU per round of 6 PIDs, adapter answering at once | 31.7 us | 9.0 us |
| Allocations per round of 6 PIDs (driver and protocols) | 314 (16.1 KB) | 110 (12.9 KB) |
| Worker, ELM327 at 38400, live values per second (6 or 12 PIDs) | 53 to 64 | 170 |
| Worker, ELM327 at 38400, connected (readout of status, faults and PIDs done) | 1122 ms | 630 ms (1181 ms) |
| Worker, demo car with its delays, connected (readout done) | 588 to 699 ms | 88 ms (311 ms) |
| Worker, demo car with its delays, live values per second | 99 to 128 | 196 |
| Worker, demo answering at once, live values per second (the cap of the 20 ms interval) | 192 | 192 |
| Worker, allocations per live value, JSON for the window included | 85 (5.9 KB) | 37 (4.0 KB) |
| Worker, adapter prompt to event, median / p95 | 0.4 / 0.6 ms | 0.3 / 0.6 ms |
| Worker, heap growth over 5 s of live values | none (about -2 KB) | none (under 2 KB, the latencies the probe keeps) |
| Start: English names of the PID table and the built-in packs | | 0.7 ms, 302 KiB heap |

The OBDLink rows of the protocols report change less: with the batched commands of the STN chip a round of
12 PIDs is already one line, and the answer count digit is not put into batched lines (see below).

## What changed and why

- **The answer count digit** (`Elm327::request`, ELM327 v1.3 and later). After the last answer an ELM327
  waits whether another control unit answers, up to its adaptive timeout; that wait was about half of every
  live request. A single hex digit after the request tells the adapter how many answer lines to expect, and
  it returns as soon as they are there. The ELM327 data sheet asks programs to count first and then use the
  count, so a mode 01 request on CAN goes out plain once, its answer frames are counted, and the next
  identical request carries that number. It is never smaller than what came last time, so nothing is cut
  off; fewer answers than the digit says only cost the usual timeout. Any AT or ST command forgets the
  counts (headers, filters or the target may have changed), and every 32nd request goes out plain again so
  a control unit that starts to answer is heard. An adapter that answers "?" to the digit (older than v1.3,
  or a clone that only claims a version) or loses answers with it twice gets plain requests for the rest of
  the session, and the request that went wrong is repeated plain at once, so no value is lost. A replay
  never gets the digit (`Transport::count_hint_allowed`), and a recording made with it still answers the
  plain request. `Elm327::set_answer_count_hint(false)` switches it off.
- **The live tick counts its interval from its start** (`Timer::rearm_from` in `worker.rs`). Before, the
  20 ms of the poll interval came after every round, so a slow adapter idled 20 ms (31 ms with the Windows
  timer) between two rounds. Now a round that takes longer than the interval is followed by the next at
  once, and a fast adapter still gets no more ticks than the interval allows. Jobs from the interface still
  get through between two ticks.
- **The greeting is drained until its prompt** (`Elm327::drain_greeting`). Connecting waited a fixed 300 ms
  for an unprompted greeting and 200 ms for the answer to a lone carriage return. Both end with the prompt,
  and nothing follows it until the next command, so the waits end there now. Without a prompt they last as
  long as before.
- **Fewer copies in the hot path.** The error words of the adapter are matched without building two strings
  per word and line, the answer lines move into the result instead of being copied, echo and status lines
  are compared without building upper-case copies, hex is decoded in one pass, the service byte is cut off
  in place, the live values move out of their map instead of being cloned, the batch memory moves back to
  the worker instead of being cloned every tick, mode 01 requests are built without `format!` per PID, and
  the worker hands its `json!` payloads for log lines, busy and live values straight to the sink instead of
  copying the finished `Value` once more through `serde_json::to_value`.

## Looked at and left as it is

- **Events to the window.** Every live value is its own `live_value` event, and many pages listen to it.
  A batched event (one per tick with all rows) would save the per-event cost in Tauri and the webview, but
  only once the interface listens to it; the payload of `live_value` is unchanged. The rate is capped by the
  poll interval: at most six values per 20 ms tick on a plain ELM327, 24 with STN batching.
- **The raw log** (on by default) sends one `log_line` per request, which is about one event per live tick.
  It is part of what users and test reports rely on, so it is not thinned; with it off the worker sends
  only the live values.
- **The answer count in STN batch lines.** The FRPM does not say how the digit behaves inside a `STBC`
  batch, so batched lines go without it.
- **Large tables.** Fault codes, the vehicle table, dimensions, control unit addresses and engine codes are
  static data in the program and cost no heap until used; the vehicle catalog and the OBDb packs are
  unpacked on first use, the translations per language. At start the app builds only the English names of
  the PID table and the built-in packs (the row above).
- **The Windows timer.** Waiting 20 ms between jobs becomes about 31 ms on Windows without a raised timer
  resolution, which caps the demo at about 190 values per second. Raising the system timer resolution costs
  battery for every program, so it stays.
- **Resident memory of the whole app** (the webview included) is not measured by the tests. By hand: start
  `cargo run -p caruspex-app --release -- --demo --page live` and watch the working set, for example with
  `Get-Process caruspex | Select-Object WorkingSet64` in PowerShell, over ten minutes.
