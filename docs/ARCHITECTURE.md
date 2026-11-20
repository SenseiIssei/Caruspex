# How Caruspex is built

This is the map for people who want to change Caruspex. How to build it and what a pull request needs is in
[CONTRIBUTING.md](../CONTRIBUTING.md); the conventions of the Rust code are in [crates/README.md](../crates/README.md).

```
page (app/src)  --call-->  Tauri command  --Job-->  worker thread  -->  protocol  -->  transport  -->  adapter  -->  car
page (app/src)  <--event-- app.emit       <--Sink-- worker/*.rs    <--  answers   <--  transport  <--  adapter  <--  car
```

## The crates

One Cargo workspace (`Cargo.toml` at the root). Each crate has a short module comment at the top of `src/lib.rs`
that says more.

| Crate | What it does |
|---|---|
| `caruspex-i18n` | The 18 languages: `l(de, en)` and `Text`, the catalogs built from `translations/` at compile time, packed per language and unpacked on first use. |
| `caruspex-vehicles` | VIN decoding, the WMI table, the vehicle catalog with plugs, buses and socket locations, motorcycles and trucks, engine codes, real dimensions and the drivetrain layout for the X-ray view. |
| `caruspex-data` | Fault codes with plain explanations, PIDs, monitor tests, control unit addresses, VAG fault numbers, fault locations. |
| `caruspex-link` | The ways to an adapter: the `Transport`, `ByteStream`, `FrameLink` and `MessageLink` traits, serial with USB detection, TCP, Bluetooth LE and classic Bluetooth, the K-line, the ELM327 driver with the STN extras, the adapter check. |
| `caruspex-j2534` | SAE J2534 PassThru drivers, and the 32-bit helper program `caruspex-j2534-bridge` for 32-bit drivers on Windows. |
| `caruspex-protocols` | The diagnosis protocols: OBD-II, UDS with the control unit scan, KWP1281, KWP2000, the VAG K-line, KWP2000 over VAG TP 2.0, BMW on the K-line, SAE J1939 for trucks. |
| `caruspex-net` | BMW ENET (HSFZ), DoIP and raw CAN interfaces (SLCAN, SocketCAN, J2534, PCAN, Kvaser) with their own ISO-TP, each ending in something that answers like an ELM327. Also VAG TP 2.0, J1939 on raw CAN and CAN signals for listening. |
| `caruspex-record` | Recordings and replay, "Share recording", custom PIDs with Torque equations, the built-in PID packs, JSON output. |
| `caruspex-analysis` | Health check, battery test, readiness, drive cycles, trip computer, performance timer. |
| `caruspex-advice` | "What should I do now?" from faults, readiness, health check and live values, and the checklist of the connection assistant. |
| `caruspex-demo` | The built-in demo car and demo truck behind a simulated ELM327 or OBDLink. |
| `caruspex-cli` | `caruspex_cli`, the command-line tool on the same core, with golden-file tests. |
| `app/src-tauri` (`caruspex-app`) | The app: window, Tauri commands, the worker thread, settings, garage, report, crash reports, the signed updater. A library (`lib.rs`) that `main.rs` starts on the desktop and the Android project loads. |

The interface is React 19 and TypeScript in `app/src/`: `pages/` (one file per sidebar page), `shell/` (window
frame, sidebar, dialogs, the page list in `shell/pages.tsx`), `state/` (stores fed by backend events), `api/`
(the contract with the backend and the mock backend), `xray/` (the 3D view), `report/`, `i18n/`.

## How a connection flows

Take "read the fault memory" on the faults page:

1. **Page.** `app/src/pages/FaultsPage.tsx` calls `call("read_faults")` from `app/src/api`. Every command and
   event, with its argument and payload types, is declared in `app/src/api/contract.ts`.
2. **Backend choice.** `app/src/api/index.ts` uses the Tauri backend (`api/tauri.ts`, `invoke` and `listen`) inside
   the app window, and the mock backend (`api/mock/`, a simulated worker with the demo car) in a plain browser.
   Only the development server and `npm --prefix app run build:demo` contain the mock backend; the app build
   leaves it out, together with the core catalogs it translates its texts with.
3. **Tauri command.** `read_faults` in `app/src-tauri/src/commands.rs` puts `Job::ReadFaults` into the worker's
   channel and returns at once. Commands that only look something up (fault code texts, the catalog) answer
   directly instead. All commands are registered in `generate_handler!` in `app/src-tauri/src/lib.rs`.
4. **Worker.** One thread (`app/src-tauri/src/worker.rs`) owns the connection and runs the jobs one after another.
   The handling is split by area in `app/src-tauri/src/worker/`: `connect.rs` (every way to connect, the demo car,
   replay, recording), `faults.rs`, `live.rs`, `health.rs`, `uds.rs`, `kkl.rs`, `bmw.rs`, `truck.rs`,
   `used_car.rs`, `adapter.rs`. Timers such as the live loop are deadlines checked between two jobs, so a stop
   request always gets through.
5. **Protocol.** The worker asks `caruspex_protocols::Obd2` (or `Uds`, `Kwp1281`, `Kwp2000`, `J1939Session`, ...),
   which builds the requests and decodes the answers, with texts from `caruspex-data`.
6. **Transport.** The protocol talks through `caruspex_link::Elm327` to a `Transport`: a serial port, TCP,
   Bluetooth, the demo car, a replay, or a front end in `caruspex-net` that turns ENET, DoIP or a CAN interface into
   ELM327 answers. K-line protocols use a `ByteStream` instead.
7. **Events back.** The worker emits events through its `Sink` (`TauriSink` in `lib.rs`), for example
   `faults` with the rows; `app.emit` sends them to the window. Pages and the stores in `app/src/state/` subscribe
   with `on(...)` or `useBackendEvent(...)`. The log lines take the same way (`log_line`).

The tests of the worker use a `Sink` that collects events in a list (`app/src-tauri/src/worker/tests.rs`).

## Where the data comes from

| Data | Edited in | Generator | Generated into |
|---|---|---|---|
| Fault codes, causes, checks | `tools/dtc/*.py` | `tools/gen_dtc.py` | `crates/caruspex-data/src/generated/` |
| Vehicle catalog, OBD socket spots | `data/vehicles/*.csv`, `data/obd_sockets.csv` | `tools/gen_vehicles.py` | `crates/caruspex-vehicles/src/generated/` |
| Control unit addresses | `data/ecus/*.csv` | `tools/gen_ecus.py` | `crates/caruspex-data/src/generated/` |
| PID packs | `data/pids/*.csv`, `data/obdb/`, `data/opendbc/` | `tools/gen_pids.py` | `crates/caruspex-record/src/generated/` |
| Engine codes (Wikipedia, CC BY-SA) | `data/engines/` | `tools/gen_engines.py` | `crates/caruspex-vehicles/src/generated/` |
| VIN maker codes only Wikibooks has (CC BY-SA); the rest is in `vin.rs` (NHTSA vPIC, Wikidata) | `data/wmi/` | `tools/gen_wmi.py` | `crates/caruspex-vehicles/src/generated/` |
| Dimensions (Wikidata CC0, makers' data sheets) | `data/wikidata/`, `data/dimensions_manual.csv` | `tools/gen_dimensions.py` | `crates/caruspex-vehicles/src/generated/` |
| Catalog copy for the browser preview and UI tests | (from the Rust catalog) | `app/scripts/extract-catalog.py` | `app/src/api/mock/data/` |
| Interface text catalogs | `translations/`, `app/src/i18n/new-texts*.json` | `app/scripts/gen-i18n.mjs` (on every UI build) | `app/src/i18n/generated/` (not in git) |
| Core text catalogs | `translations/` | `crates/caruspex-i18n/build.rs` (at compile time) | the build directory |
| Licences of everything shipped | `Cargo.lock`, `app/package-lock.json`, the Android build file, the data table in the script | `tools/gen_notices.py` | `THIRD_PARTY_NOTICES.md`, copied into `app/public/legal/` on every UI build |

Files in `generated/` folders and `app/src/api/mock/data/` are never edited by hand. The CI runs the generators
and fails when the result differs from what is committed.

Some sources need the network: `tools/gen_obdb.py --fetch`, `tools/gen_opendbc.py --fetch` and
`tools/gen_dimensions.py --fetch` fill a download cache in `.cache/` (not committed). The CI only checks the
conversions offline on fixtures. Once a month `.github/workflows/data-check.yml` runs `tools/check_sources.py`
and opens an issue when an upstream source has something new.

## Demo car, recordings and replay

- **Demo car** (`crates/caruspex-demo`): `DemoTransport` is a `Transport` that plays an ELM327 character by
  character, with a simulated car behind it whose state is a function of the time since "ignition on". It has
  OBD-II and UDS control units, units on the extra buses of an OBDLink, and a demo truck on J1939. The app
  connects to it with `--demo` or the Connect page, the CLI with `--demo`, and the tests use it everywhere.
- **Recordings** (`crates/caruspex-record/src/recording.rs`): `RecordingTransport` sits between the ELM327 driver
  and the real transport and writes every write and read with a timestamp as JSON Lines.
- **Replay**: `ReplayTransport` reads such a file and answers each command with what was recorded, so a session on
  somebody's car can be reproduced at the desk (`--replay <file.jsonl>` in the app and the CLI).
- **Share recording** (`crates/caruspex-record/src/share.rs`): a copy of a recording that is fit for a public
  issue, with the last digits of every VIN, the port or address and the time of day removed. The user sees a
  preview of what stays before the file is saved.
- **Browser preview**: the mock backend in `app/src/api/mock/` simulates the worker with the same demo car data,
  so the whole interface runs with `npm --prefix app run dev` and no Rust at all.

## The X-ray view

The X-ray page (`app/src/pages/XrayPage.tsx`) shows the connected car, or any generation of the catalog, as a
3D glass model with three.js, loaded only when the page opens.

- `caruspex-vehicles` provides the facts: body type, dimensions, drivetrain layout, the socket spot.
- `app/src/xray/shape.ts` turns them into the shape of the body (defaults per body type and era, a hand-tuned table
  for common generations), `moto.ts` and `heavy.ts` do the same for motorcycles, trucks and buses.
- `app/src/xray/layout.ts` places the parts, the control units the scan found (by kind and name) and the bus lines
  between them; `scene.ts` draws what the layout computed. Faults color their zone, live values hang on their part.
- Without WebGL the page falls back to the 2D side view (`components/VehicleDiagram.tsx`).
- `app/src/xray/layout.test.ts` builds a model of every generation of the catalog and checks it (wheels on the
  ground, parts inside the body). `tools/xray_shots.py` takes pictures of many generations with the built app for a
  contact sheet.

## Release and updater

- A release starts with `bash tools/release.sh <version>`, which prepares a branch locally (version in
  `Cargo.toml` and `app/package.json`, `changelog/<version>.md` in German and English, checks, packages, smoke
  test, screenshots) and pushes nothing.
- After the pull request is merged, the maintainer pushes the tag `v<version>`. The CI builds and smoke-tests the
  packages on Windows, Linux and macOS; the `release` job checks that the tag matches the version, signs the update
  packages with the updater key held as a repository secret, runs `tools/release_assets.py` (checks every signature
  against the public key built into the app, writes `latest.json`) and publishes the GitHub release. The Android APK
  is signed and attached only when its signing secrets exist.
- The updater in the app (`app/src-tauri/src/update.rs`) asks the newest release of this repository for
  `latest.json`, downloads only from this repository's releases and installs a package only when its signature
  matches the public key in the app. Both the check and the installation can be switched off in the settings.
- Next to the packages every release carries `SHA256SUMS`, a Sigstore bundle of GitHub's build provenance for
  every file (`actions/attest-build-provenance`) and one SBOM per platform (`tools/sbom.py`, CycloneDX). These
  files have their own names and are added before the release is created, so `latest.json` and the update
  packages stay exactly as the updater expects them. How to check a download is in
  [SECURITY.md](../SECURITY.md#checking-a-download).
- None of the release steps run for pull requests or in forks.

## Security and the Cyber Resilience Act

[docs/cra/](cra/scope.md) holds the scope assessment for the EU Cyber Resilience Act, the risk assessment of
Caruspex, the mapping of the essential requirements to the code ([requirements.md](cra/requirements.md)), the user
information and the support statement. Keep [risk-assessment.md](cra/risk-assessment.md) current when a change
adds a new way to an adapter, a new file format, a network request or a step in the release workflow.
