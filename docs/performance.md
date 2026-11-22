# Performance

How quick and how lean Caruspex is, how that was measured and what keeps it that way. Every number here can
be measured again with the commands next to it.

## Backend: Speed and memory

Caruspex should feel immediate on a slow adapter and stay small during an hour of live values. This page
describes how that is measured, what the numbers were before and after the work in 0.16, and which tests
fail when it gets clearly worse.

### The harness

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

### Commands

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

### The guards

| Test | Budget | Measured after |
|---|---|---|
| `caruspex-protocols` `a_live_round_allocates_little` | under 220 allocations per round of 6 PIDs (314 before) | about 110 |
| `caruspex-protocols` `the_live_loop_does_not_grow` | heap grows less than 64 KiB over 3000 rounds | 0 |
| `caruspex-protocols` `a_round_on_a_slow_line_costs_one_request_per_six_pids` | one request per round, the answer count on all but the first, under 150 ms per round at 38400 baud | 15 requests, 14 with the count, about 35 ms |
| `caruspex-app` `the_live_loop_keeps_to_the_poll_interval_and_allocates_little` | 30 to 330 live values per second at the default 20 ms interval, under 45 allocations per live value (85 in 0.16, 37 with the event through a `Value`), heap growth under 256 KiB in 2 s | about 192 values per second, 23 allocations per value |
| `caruspex-app` `an_answer_reaches_the_sink_quickly` | median under 50 ms from the prompt to the event at 115200 baud, best of three runs (shared CI macOS runners reach 15 ms, once 50 ms) | under 1 ms |
| `caruspex-record` `a_thirty_minute_recording_is_read_quickly` | `live_frames` reads 30 minutes of CAN 29 bit with headers (36 000 rounds, 144 006 events, 11 MB) in under 20 s in a debug build | about 1.3 s debug, 0.2 s release |
| `caruspex-link` `count_hint.rs` | the answer count digit: learned, used, dropped when refused or when it loses answers, relearned every 32nd request | |

The budgets are generous on purpose. They catch a return of per-line copies, a request per PID instead of
per six, a leak or a loop that floods the window, not a few percent.

### Before and after

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
| Soak of the driver and protocols, 60 s (6.2 million rounds of 6 PIDs, adapter at once) | | heap +0 bytes |

The OBDLink rows of the protocols report change less: with the batched commands of the STN chip a round of
12 PIDs is already one line, and the answer count digit is not put into batched lines (see below).

### What changed and why

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

### Hangs, dead links and the window thread (November 2026)

The worker runs one job after the other, so anything that waits long inside a job also holds every job
behind it, Disconnect included. Measured with the tests named in brackets, Windows 11, debug builds unless
release is said.

| What | Before | After |
|---|---|---|
| Disconnect during a live round on an adapter that fell silent (`disconnect_on_a_silent_adapter_does_not_wait_for_the_live_round`) | over 20 s (the request in flight, six single requests of 4 s each, ATPC) | 2.1 s, of which 2 s are the ATPC |
| Disconnect during a control unit scan, health check or used car check | until the job ended | the job is cancelled, the request in flight ends within 120 ms |
| Live round of 6 PIDs on a silent adapter, CAN / K-line (`a_silent_adapter_costs_one_timeout_per_round_and_keeps_the_batch_memory`) | 7 / 6 timeouts of 4 s, and on CAN six requests per round for the rest of the session | 1 timeout, the PIDs stay batched |
| Reads of a closed TCP connection in 300 ms (`a_connection_the_adapter_closed_waits_instead_of_spinning`) | 138 289 (a core at 100 %) | 7 |
| Connecting to a Wi-Fi address nobody answers | 21 s (Windows), up to 2 min (Linux) | 5 s |
| A Bluetooth LE call that never finishes | waits forever | fails after its limit (scan + 20 s, connect 60 s) |
| Program end | the process ended while the worker might still close the adapter | waits up to 3 s for ATPC and the port |
| Share recording of 30 minutes (11 MB, release) | 0.35 to 0.7 s on the window's thread | on a thread of its own |
| Worker, allocations per live value, JSON for the window included (release report) | 37.3 (4047 bytes) | 23.3 (2711 bytes) |

- **Interrupting the driver.** `Elm327::set_interrupt` takes a flag another thread may raise: the command in
  flight ends within one read (at most 120 ms), further commands return at once without being sent. The
  app's Disconnect (`WorkerHandle::close`) cancels a running scan or check and raises the flag; the worker
  lowers both when it reaches the close, so the ATPC goes out as before. `disconnect` leaves the ATPC out when
  the adapter said nothing at all to the last command. Ending the program does the same and waits up to 3 s.
- **Silence is not a refusal.** A request that got no byte at all (not even NO DATA or a prompt) means the
  adapter is gone or hung: `request_many` hands the same answer to the requests after it without sending them,
  and `read_live_batch` keeps the PIDs batched and skips the single requests of that round.
- **Dead links wait like quiet ones.** A pulled USB cable, a closed Wi-Fi connection or a lost Bluetooth
  adapter answered every `read` at once with nothing; the driver and the K-line code read in a loop until their
  deadline and spun a core during every request. Serial, TCP, RFCOMM and Bluetooth LE now wait out the time
  asked (`timing::wait_out`). A healthy port waits that long anyway, so no timing the control units see
  changes.
- **The window's thread.** A Tauri command without `async` runs on the thread of the window. The lists of
  J2534 devices and of PCAN and Kvaser channels (driver DLLs of other makers) and Share recording run with
  `spawn_blocking` now. The lookups (catalog, PID table, fault codes) stay: under 3 ms on their first call
  (catalog search 2.9 ms, PID table 0.9 ms, release); the fault code lexicon (19 ms) was async already.
  Saving settings and the garage stays a plain command on purpose: as async commands two saves could overtake
  each other and the older state would be written last.
- **Live values straight to JSON.** `Sink::live_value` hands the row to Tauri, which serializes it once; the
  `serde_json::Value` built for every live value before is gone. The payload is the same.

### Looked at and left as it is

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

## UI

The interface is React 19 in a WebView (WebView2 on Windows). Two facts shape everything below:

- **Every page stays mounted.** Pages keep listening to the backend while another page is on screen (the trip
  computer, the statistics and the X-ray view collect what happens meanwhile). So whatever a hidden page does
  per event, it does all the time. `usePageActive()` (in `app/src/lib/hooks.ts`) tells a component whether its
  page is the one on screen.
- **Live values arrive many times a second**, one `live_value` event per reading. The demo backend of the
  browser preview sends up to 150 a second, a real adapter 10 to 50.

### Rules for UI code

- **Show readings with `useLiveValues` / `useLiveRows`** (`app/src/state/liveView.ts`), not with a
  `useBackendEvent("live_value")` that sets state. They render at most once per animation frame, only when a
  shown value changed, and not at all while the page is hidden; the page catches up when it is shown.
  Keep `useBackendEvent("live_value")` for code that needs every reading (the chart, the CSV recording, the
  performance timer, the trip statistics), and let it write to refs, not state.
- **Animate outside React.** The needle of `Gauge` moves by setting four SVG attributes in an animation
  frame; React renders the gauge only when its value changes.
- **Big lists are memoised and split.** The live page builds its list of values once per tick or search, the
  table of readings is its own component with one memoised row per value. `StatTile` is memoised.
- **Hidden pages do not poll.** Timers that refresh what is on screen run only while `usePageActive()` is
  true (the "What should I do now?" advice, the live table, the X-ray labels).
- **The first picture comes first.** `PageHost` (`app/src/shell/App.tsx`) mounts the page on screen, and the
  other pages in a transition right after it. The start-up in `App` waits for `pagesMounted()`
  (`app/src/shell/nav.ts`) before it asks for `app_info`, which starts the tasks of the command line in the
  backend, so every page listens before anything is connected. Whatever has to be heard before that belongs
  in a store in `app/src/state/` or in `App`.
- **A page shown once keeps its layout while hidden.** Its slot gets the class `kept`: hidden with
  `content-visibility: hidden` and `inert` instead of `display: none`, so showing it again does not lay it out
  anew. Pages never shown stay `display: none`. Reading a size (`clientWidth`, `getBoundingClientRect`,
  `scrollHeight`) inside a hidden page forces the layout of that page; do it in an event handler, a
  `ResizeObserver` callback or while the page is active.
- **No measuring in mount effects.** A `useLayoutEffect` that reads a size when it mounts forces a layout of
  the whole window in the middle of a commit; `VehicleDiagram` takes its size from its `ResizeObserver`.
- **Growing text is appended.** The log panel (`app/src/shell/LogDrawer.tsx`) adds one block per batch of
  lines and drops whole blocks at the top; while it is closed it does not update at all.
- **Intl formatters are kept.** `localeNumber`, `localeDate` and `localeDateTime` (`app/src/i18n/index.ts`)
  keep one formatter per language and form; building one costs about two hundred times as much as using it.
- **Large data loads on the first visit.** The fault code lexicon fetches and draws its table when it is first
  opened (it was ten thousand elements at start).
- **One backend subscription per event name.** `app/src/api/hub.ts` shares one Tauri `listen()` between all
  handlers of an event, so a hundred `useBackendEvent` calls at start are about fifty round trips, not a
  hundred. A handler that throws does not stop the others.
- **three.js resources are freed or shared.** Anything removed from the X-ray scene goes through
  `disposeTree` (`app/src/xray/dispose.ts`); the packets share one geometry and one pair of materials per bus.
  The render loop stops while the page is hidden and while the window is minimised (`visibilitychange`).
- **The demo backend is not part of the app.** `app/src/api/index.ts` loads it with a dynamic import only in a
  plain browser, and only the development server and `npm --prefix app run build:demo` contain it; the app
  build (`npm --prefix app run build`, which Tauri builds into the program file) leaves it out together with
  the core catalogs it translates its fault codes with. `tools/bundle_budget.py` fails the build if a demo
  chunk ends up in the app build, or three.js or a catalog in the entry chunk.
- **Core texts come from the backend.** Fault code descriptions, value names and messages are translated by
  the Rust core (`caruspex-i18n`); the interface catalogs hold only the interface texts. They are keyed by a
  short hash of context and German source (`textKey()` in `app/src/i18n/index.ts`), since the interface
  writes both in the code anyway, and German loads no catalog at all.

### Budget in the CI

`python tools/bundle_budget.py` runs after `npm --prefix app run build` in the Checks job. The budgets leave
room on purpose (entry chunk 900 KB / 300 KB gzip, its style sheet 140 KB, a language catalog 250 KB / 70 KB
gzip, any other later chunk 900 KB / 260 KB gzip, the whole build 7 MB, no chunk of the demo backend at all)
and catch a large dependency or data file that slipped into the start or into the app, not a few kilobytes.
`python tools/bundle_budget.py --demo --dist app/dist-demo` checks the demo build (`npm --prefix app run
build:demo`, whole build 20 MB, a chunk 1200 KB). If a change needs more, raise the number in the script and say why in
the pull request.

To see what a chunk contains, build with source maps into a scratch folder
(`npx --prefix app vite build --sourcemap --outDir <scratch>`) and attribute the bytes of
`assets/index-*.js` to its sources with any source map explorer.

### How it was measured

`tools/ui_measure.py` drives a headless Chromium with Python Playwright (installed by whoever measures, not a
dependency of the project) against the browser preview with the demo car, 1440x920:

```bash
npm --prefix app run build:demo                      # the release build with the demo backend, in app/dist-demo
npx --prefix app vite preview --outDir dist-demo --port 5392
npm --prefix app run dev                             # the development server, port 5287
py -I tools/ui_measure.py startup http://localhost:5392 4   # median of 5 starts, CPU four times slower
py -I tools/ui_measure.py live http://localhost:5392 live-run
py -I tools/ui_measure.py live http://localhost:5392 start-live
py -I tools/ui_measure.py long http://localhost:5392 10
py -I tools/ui_measure.py xray http://localhost:5287
py -I tools/ui_measure.py xraylive http://localhost:5287
```

Renders are counted through a stand-in for the React DevTools hook (a component counts when it did work in a
commit), so the release build is measured as shipped; against the development server the counts come with
component names and the page they belong to. CPU time is Chromium's `ScriptDuration` and `TaskDuration`,
memory the JS heap after a forced garbage collection plus the backing stores of typed arrays. The X-ray counts
come from wrapped WebGL calls (buffers, textures, programs alive and created) and, on the development server,
`renderer.info` through `window.__caruspexXray`, which only the development build sets.

### Numbers (November 2026)

Before and after the work of November 2026 (branch `perf-ui`). Browser numbers are from the release build in
headless Chromium with the demo backend (about 150 readings a second); "app" numbers from the release app on
Windows 11 with WebView2, the demo car and `--page live-run` or `--page start-live`, measured through
WebView2's DevTools port.

| What | Before | After |
|---|---|---|
| Entry chunk (`vite build`) | 1217 kB, 364 kB gzip | 732 kB, 234 kB gzip |
| Demo backend | inside the entry chunk | own chunk (488 kB), never loaded by the app |
| Start, browser, CPU 4x slower: DOM content loaded | 169 to 245 ms | 129 to 142 ms |
| Start, browser, CPU 4x slower: last long task ends | 967 to 1169 ms | 783 to 950 ms |
| Start, browser, CPU 4x slower: total blocking time | 50 to 104 ms | 6 to 40 ms |
| Start, app: DOM content loaded / first paint | 49 / 220 ms | 38 / 216 ms |
| DOM nodes after start | 25 256 | 6 751 (the lexicon alone was 10 113 elements) |
| JS event listeners after start | 2 350 | 671 |
| JS heap after start (after GC) | 11.6 MB | 8.9 MB |
| Live page, browser: React commits / component renders per s | 76 to 88 / 1166 to 1198 | 28 / 369 to 375 |
| Live page, browser: script time per s | 33 to 37 ms | 15 to 16 ms |
| Start page with gauges, browser: commits / renders per s | 72 to 74 / 1531 to 1549 | 49 / 724 to 726 |
| Live page, app: script / all main thread work per s | 69 / 230 ms | 17 / 191 ms |
| Start page with gauges, app: script / all main thread work per s | 75 / 505 ms | 26 / 392 ms |
| Live page, app: JS heap after GC / WebView renderer private memory | 23.7 / 227 MB | 10.5 / 180 MB |
| 10 min live session with recording, browser: heap plus typed arrays | +4.6 MB (17.3 to 21.9) | +1.5 MB (12.1 to 13.6) |
| 10 min live session: DOM nodes, listeners | flat | flat |
| X-ray page opened 20 times, rebuilt 20 times | no leak (WebGL buffers 267, textures 8, programs 11 before and after) | same, `renderer.info`: 76 geometries, 4 textures, 11 programs throughout |
| X-ray page with live values: WebGL objects created per s | 3.6 buffers, 0.6 textures, a shader program every 10 s | 0 |

Of the renders left, the hidden throttle page (`ThrottlePage.tsx`, which sets state on every reading) makes
about 300 a second on the live page and 600 on the start page; without it the live page is at about 70
renders a second and the start page at about 120. The main thread work that is left on the start page is
mostly layout and paint of the six needles, which move sixty times a second as before.

Screenshots of the live page, the start page with its gauges and the X-ray view (with packets in flight) look
the same before and after.

### Texts and the size of the build (20 November 2026)

Version 0.19 added about 9400 generic fault codes in 18 languages. In the app they are translated by the Rust
core (`translations/<code>/core_dtc38.json`, built into `caruspex-i18n`); the interface build carried a second
copy of every core text, fault codes included, as one core catalog per language for the demo backend of the
browser preview, keyed by the full English text. The app never loaded those catalogs, but Tauri builds the
whole interface build into the program file. The fault code texts (`dtc38`) were 27 to 30 % of each of these
catalogs, the English keys about 300 kB of each. The interface catalogs themselves held no fault code text,
but half of each was its keys ("context|German source").

Now the app build leaves the demo backend out (`npm --prefix app run build:demo` builds it for a plain
browser), the interface catalogs are keyed by a short hash, and German, the source language, loads no catalog.
The demo backend also puts generic fault codes together in the chosen language now, the way the Rust core
does (before, the 8000 codes made of phrases showed in English in every language but German).

| What | Before | After |
|---|---|---|
| Interface build (`app/dist`, what the program file carries) | 19041 KB | 5082 KB |
| Demo build (`build:demo`, browser only) | (the same build) | 17747 KB |
| Largest chunks of the app build | `core-hi` 990 KB, `mock` 927 KB, `core-ru`/`core-uk` 919 KB, ... | `index` (entry) 729 KB, `scene` 651 KB, `hi` 174 KB |
| Entry chunk | 730 KB (232 KB gzip) | 729 KB (232 KB gzip) |
| Interface catalog per language | 166 KB (`en`) to 250 KB (`hi`) | 90 KB (`en`) to 174 KB (`hi`) |
| Loaded at start for texts, German / French | 166 KB / 345 KB | none / 192 KB |
| Release program file `caruspex.exe` (Windows, `cargo build --release -p caruspex-app`) | 28 570 624 bytes | 25 944 576 bytes (-2.5 MB) |
| App, JS heap after GC 8 s after start with the demo car (de / fr / hi) | 7.6 / 7.8 / 7.9 MB | 7.4 / 7.9 / 8.0 MB |
| Browser preview, JS heap after GC after start (de / fr / hi) | 12.2 / 13.2 / 13.4 MB | 11.0 / 12.3 / 12.5 MB |

The heap of the app barely changes: it never held the core catalogs, and a catalog of 1600 texts is small
once parsed. What changed for the app is what it carries and what it parses at start and on a language switch.
The browser preview still loads the demo backend (927 KB) and the core catalog of its language (580 to 990 KB).

### Second round (22 November 2026)

Branch `opt-ui`. The demo backend of the browser preview now writes one raw log line per live tick, like the
worker does with "Log raw adapter traffic" on (the default), so the browser numbers include the log panel.
"App" is a debug build of `caruspex-app` on Windows 11 with WebView2, the demo car, `--page live-run`, measured
through WebView2's DevTools port; the interface in it is the same release bundle as in a release build. The
machine was busy with other builds during these runs, so times vary by about a third between runs; the
counts (renders, commits, layouts) do not.

| What | Before | After |
|---|---|---|
| Live page, app, 60 s into a session, log closed: main thread / layout per s | 264 to 316 / 206 to 241 ms | 46 to 63 / 3 to 4 ms |
| Live page, app, same with the log open | 277 to 351 / 215 to 253 ms | 119 / 39 ms |
| Live page, browser, after 60 s (about 1700 log lines): main thread per s | 233 to 248 ms | 64 to 69 ms |
| Live page, browser, 18 s after start: commits / renders / main thread per s | 10 / 115 / 74 ms | 4 / 60 / 21 ms |
| Start page with gauges, browser: commits / renders per s | 31 / 173 | 25 / 121 |
| Start, app: first contentful paint (median of 4) | 232 ms | 178 ms |
| Start, browser, CPU 4x slower: frame after the first React commit (from navigation, demo backend included) | 710 to 796 ms | 271 to 310 ms |
| Start, browser, CPU 4x slower: long task holding the first commit / components in it | 481 to 544 ms / 699 | 61 to 65 ms / 81 |
| Start, browser, CPU 4x slower: all pages mounted | 660 ms | 808 ms |
| Fault code lexicon shown again, browser, CPU 4x slower | 619 to 1295 ms | 101 to 357 ms |
| Glossary shown again, same | 112 to 130 ms | 67 to 70 ms |
| `localeNumber`, Node 24 | 148 us per call | 0.67 us per call |
| X-ray page opened 20 times, rebuilt 20 times | no leak | no leak (WebGL buffers 267, textures 8, programs 11 throughout) |

What was behind it:

- **The log panel** joined up to 5000 lines into one text and laid all of it out again every 150 ms while
  values streamed, also when it was closed. That was most of the main thread on every page during a live
  session in the app. It now appends one block per batch and does nothing while closed (`LogDrawer.tsx`).
- **The start** read the size of the car picture of the start page in a layout effect, which forced a layout
  of the whole window in the middle of the first commit and a second render; and it built all 29 pages before
  the first picture. Now the picture measures with its `ResizeObserver` and `PageHost` mounts the visible page
  first. The other pages are mounted a little later than before (about 150 ms with the CPU 4x slower), and
  the start-up waits for them before it talks to the backend.
- **Showing a page again** laid it out from scratch (`display: none`); the lexicon table has 3000 rows.
  Pages shown once keep their layout now (`content-visibility: hidden`), the lexicon rows are memoised and
  its search uses `useDeferredValue`, the glossary sorts once per language.

Run again with `py -I tools/ui_measure.py live <base> live-run 60` (and `... 60 open`), `firstframe <base> 4`
and `switch <base> 4`.

### Still open

- `PageSlot` renders the whole page again on every switch, because `<Component />` is a new element each
  time. A memoised element would leave pages that do not call `usePageActive()` alone, but a page that reads
  changing state outside a hook during render would then show old values until its next render; that needs a
  pass over every page first.
- The X-ray view draws every frame while a fault pulses (the demo car has faults), and the chips over the 3D
  view blur what is behind them (`backdrop-filter`), which the GPU redoes for every frame. Both are part of
  the look; a slower pulse or chips without blur would change it.
- With the log panel open during a live session the app still spends about 40 ms a second on layout, mostly
  for following the end of the text.
- The core catalogs of the demo build repeat every English text as the key; a shared list of the English
  texts and one array per language would cut about 300 KB from each, for the browser preview only.
- `XrayPage` keeps the layout code of the 3D view (about 110 kB) in the entry chunk, because the page
  collects scan results while hidden; moving the layout into the lazy chunk needs the page split in two.
- Every Tauri event reaches the WebView as a script of its own. A batched live event from the backend would
  cut that per reading cost; the interface keeps handling `live_value` either way.
