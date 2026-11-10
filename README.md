# Caruspex

[![Build](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml/badge.svg)](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Open-source vehicle diagnostics for Windows, Linux and macOS, written in Rust with a Tauri 2 interface.
One cable or dongle, one application: read and clear trouble codes, watch live sensor data,
check emission readiness, and talk to VW/Audi control units at the byte level over a K-Line cable.

The name comes from the *haruspex*, the Roman seer who read omens from the entrails. Caruspex reads the car's.

The application speaks 18 languages, the interface and every fault code text alike: English, German, Spanish,
French, Portuguese, Italian, Dutch, Polish, Czech, Russian, Ukrainian, Turkish, Arabic (right to left), Chinese,
Japanese, Korean, Hindi and Indonesian. It follows the system language, can be switched in the settings, or
forced per run with `--language fr` and so on. How translations are made and checked is in
[translations/TRANSLATING.md](translations/TRANSLATING.md). [Deutsche Fassung dieser Seite](README.de.md)

![Overview with live gauges](docs/screenshot-overview.png)

## Highlights

- **Two complete diagnostic stacks**: standard OBD-II/EOBD through any ELM327 adapter, and VW/Audi
  manufacturer diagnostics (KWP1281 and KWP2000) over a raw KKL K-Line cable.
- **Protocols implemented from scratch**: KWP1281 acknowledges every single byte with its one's complement,
  something ELM327 firmware cannot do. Caruspex drives the K-Line itself, including the 5-baud wake-up
  address that is bit-banged by toggling the serial break signal with deadline-based timing, so OS
  scheduling jitter does not accumulate across the ten bits.
- **Automatic protocol detection**: after the wake-up, the ECU's two key bytes tell whether it speaks
  KWP1281 or KWP2000 (ISO 14230); Caruspex switches accordingly.
- **Modular Rust workspace**: one crate per area (adapter links, protocols, network and CAN, vehicles, data,
  recordings, analysis, demo car, command-line tool), and the Tauri application on top. The core knows nothing
  about the interface, so the app and the command-line tool run on exactly the same code.
- **Tested without a car**: simulated ECUs (`crates/caruspex-protocols/tests/common/`) reproduce the
  byte-by-byte handshake, fake gateways stand in for ENET and DoIP, a fake J2534 driver covers the PassThru path,
  and a Python ELM327 simulator allows end-to-end runs of the app.
- **One set of translations**: the core and the interface share the same translation files in
  `translations/<code>/`, in 18 languages. The core looks its texts up by the English text, the interface builds
  its catalogs from the same files.
- **Recognises every car**: a VIN decoder with 365 manufacturer codes and a catalog of more than 2,100
  model generations since 1980 tell which car is connected and what works with it.
- **Built-in demo car**: an in-process ELM327 with an engine and a gearbox on CAN, so everything can be tried
  without hardware.
- **Generated data, not hand-typed**: about 1,700 trouble codes, many of them computed from regular code families
  (per cylinder, per bank, per oxygen sensor) by `tools/gen_dtc.py`.

## Features

### Knows the car
- After connecting, Caruspex reads the VIN and tells you which car it is: make, model, generation, model year.
- A catalog of more than 2,100 model generations from 94 brands, 1980 to today, including brands that no longer exist.
- For every generation: which diagnostic connector, where it sits, which protocol, and whether OBD-II, a KKL cable or only the maker's own tools will work.
- A VIN checker for any VIN: manufacturer, country, model year, check digit, model code.
- A garage that recognises every car by its VIN and keeps the history of its checks and odometer readings.

![Vehicles](docs/screenshot-vehicles.png)

### One-button health check
- Reads the fault memory, readiness monitors and component tests, then watches mixture, temperatures, oxygen sensors and charging voltage for half a minute.
- A score from 0 to 100 where every point lost is explained in plain words, with the next step.
- Ready for the emissions test? Which monitor is still open and how to drive to finish it.
- An inspection report as PDF, for printing or for the workshop.

![Health check](docs/screenshot-health.png)

### Fault memory
- Stored, pending and permanent codes; with several control units, each code shows who reported it (engine, gearbox).
- About 1,400 standard codes (P, C, B, U) plus manufacturer codes for VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai/Kia, Subaru and more. A P1 code is read the way the detected brand means it.
- A traffic light per code and the question everyone asks first: may I keep driving?
- Usual causes, things you can check yourself, and a rough cost class.
- The car in see-through view shows where the affected part sits.
- Clearing with a clear warning and an automatic re-read; CSV export.

![Fault memory](docs/screenshot-faults.png)

### Live data
- Every mode 01 value of SAE J1979 up to PID C4: 145 PIDs with more than 300 readings, grouped and searchable.
- Odometer, gear, oil temperature, fuel rate, torque, boost, exhaust temperatures, particulate filter, AdBlue, NOx and hybrid battery, as far as the car reports them.
- Rolling chart and CSV recording.

![Live data](docs/screenshot-live.png)

### Measuring
- Trip computer: consumption live and on average, cost, CO2, distance and a gentle driving style score, even for cars without a trip computer of their own.
- Performance: 0 to 100 km/h, 80 to 120, quarter mile and braking distance from the speed signal (closed roads only).
- Battery and alternator: resting voltage, cranking dip, charging voltage with and without load, guided in four steps.

### Inspection
- Readiness monitors in plain words, freeze frame, VIN.
- Mode 06 component monitoring with value and limits, misfire counts per cylinder.
- ECU names, calibration IDs and checksums from mode 09, per control unit.
- Raw ELM327 terminal.

### Adapter check
- Genuine ELM327 or clone? It asks for the commands of each version, measures the response time and says honestly what follows from that.
- Recognises OBDLink and other STN adapters.
- Never writes anything to the adapter.

![Adapter check](docs/screenshot-adapter.png)

### Every cable, every way into the car
Caruspex finds out by itself what is plugged in and what the car speaks.

| Interface | Examples | How |
|---|---|---|
| ELM327 over USB, Bluetooth, Bluetooth LE, Wi-Fi | cheap ELM327, Vgate, vLinker, Veepeak | serial port, paired or direct BLE, TCP |
| STN chips | OBDLink SX, EX, MX+, CX | as ELM327, with faster timing |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik, Godiag, VAS 5054A in J2534 mode | the maker's 64-bit driver, CAN with ISO-TP done by Caruspex |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | serial port, found by the automatic port search |
| SocketCAN (Linux) | PEAK PCAN-USB, Kvaser, CANable with candleLight, Raspberry Pi CAN hats | can0 and friends |
| KKL cable (VAG 409.1) | FTDI or CP2102 cables | K-Line, KWP1281 and KWP2000 |
| ENET cable | BMW and Mini from the F series | HSFZ over Ethernet |
| DoIP cable | many cars from about 2016 | ISO 13400 over Ethernet |

On every one of them the same functions work: OBD-II and OBD on UDS, every control unit over UDS, BMW
addressing, listening to broadcast values (Tesla), recording and replay.

![Connect](docs/screenshot-connect.png)

### All control units over CAN (UDS)
- With a plain ELM327, Caruspex asks every control unit one by one: engine, gearbox, ABS, airbag, instrument cluster, climate and more, on most cars from about 2008.
- Fault memory per unit with the failure type (short circuit, open circuit, implausible ...), part number, software and hardware version.
- Address lists for VAG, BMW and Mini, Ford, Mazda, GM, Toyota, Nissan, Hyundai/Kia, Renault and PSA; for everything else a sweep over the usual addresses.
- BMW and Mini from the F series on with BMW's own extended addressing over 6F1, plus a sweep over all 128 diagnostic addresses; fault codes in BMW's format as ISTA shows them (e.g. 480A10).
- Clearing per unit, with a warning.

![All control units](docs/screenshot-ecus.png)

### VW and Audi over a KKL cable (KWP1281 and KWP2000)
- Throttle body adaptation: the real basic setting in measuring block 060 or 098.
- Scan every control unit, including ABS, airbag, instrument cluster and comfort, with a printable report.
- Adaptation channels and output tests, deliberately read-only.

### Manufacturer and custom values
- Electric car batteries: state of health, charge, cell voltages with spread, temperatures, for Hyundai Kona and Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf and Tesla Model 3 and Y (by listening on the CAN bus); every value with a documented source.
- Load your own values in the widely used Torque CSV format, with a complete evaluator for Torque equations.

![EV battery](docs/screenshot-battery.png)

### Record and replay
- Record a session with the car into a file and play it back later without the car, handy for troubleshooting.

### Without a car or adapter
- A built-in demo car: it drives, warms up, has a few faults stored and talks like a real ELM327 over CAN.
- An offline lexicon of all codes, filterable by brand.

### Look and feel
- A new interface in the style of Odysync: calm, dark, with smooth transitions.
- Step-by-step connecting with an automatic search over all ports.
- 18 languages; settings apply immediately.
- Tells you when a new version is out: at start it asks GitHub for the newest release and nothing else (no data
  about you or the car); can be switched off in the settings.

## Supported vehicles

Any vehicle with OBD-II or EOBD: in Europe petrol cars from 2001 and diesels from 2004, in the US from 1996. Older VW, Audi, Seat and Skoda from about 1990 work over the KKL cable. The Vehicles page tells you which way works for a given car.

| Era | Bus | Examples |
|---|---|---|
| until about 1995 | manufacturer specific | VAG with KKL through the 2x2 plugs, otherwise workshop tools only |
| 1996 to about 2003 | K-Line (ISO 9141-2, KWP2000) | Audi A2, VW Golf IV, Toyota, Renault |
| from about 2004 | CAN (ISO 15765-4) | Audi A4 B7, VW Golf V, modern Japanese cars |
| US until about 2005 | SAE J1850 PWM and VPW | Ford, Chevrolet |
| newest cars | OBD on UDS (SAE J1979-2) | required in the US from model year 2027, already in some newer cars; detected and translated automatically |
| BMW and Mini from the F series | ENET (HSFZ over Ethernet) or D-CAN with 6F1 addressing | every control unit |
| many cars from about 2016 | DoIP (ISO 13400) with a DoIP cable | OBD over Ethernet, found automatically |
| Tesla Model 3 and Y | no OBD: broadcast CAN frames | battery, cells, temperatures, speed, odometer by listening (adapter cable to the diagnostic connector) |

Pure electric cars do not have to offer OBD-II emissions data. Some answer partly anyway; for cars without any answer but with traffic on the bus, Caruspex connects for listening and reads what the built-in packs know (Tesla Model 3 and Y so far).

## Supported adapters

- Any **ELM327**-compatible adapter: USB, classic Bluetooth (on Windows through the COM port of a paired adapter), **Bluetooth LE**
  (Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE and other adapters with the usual GATT services; found by a
  scan, no pairing) or Wi-Fi via TCP
- An **ENET cable** (OBD to RJ45) for BMW and Mini from the F series on: Caruspex finds the car on the network
  (HSFZ, ports 6811 and 6801) and reads every control unit over Ethernet, no adapter chip in between. The computer
  needs an automatic address (169.254.x.x), which Windows, macOS and Linux assign by themselves when no DHCP answers.
  Caruspex only reads here as well; coding and flashing stay with E-Sys, ISTA or the tuning tools.
- Any **KKL cable (VAG 409.1)**. FTDI or CP2102 based cables are far more reliable on the K-Line than the
  cheapest clones; for KWP1281 a genuine FTDI FT232RL is recommended.

Proprietary interfaces such as Delphi DS150E, Autocom or VAS 5054A are not supported, they do not speak ELM327.

| Function | ELM327 | KKL 409.1 |
|---|---|---|
| Read and clear engine faults | yes | yes |
| Live data, freeze frame, readiness | yes | yes |
| ABS, airbag, instrument cluster, comfort | **no** | yes |
| Manufacturer measuring blocks | **no** | yes |
| **Throttle body adaptation** | **no** | yes |

This is a hardware limit, not a software one: KWP1281 is a block protocol in which every byte is
acknowledged by the other side. The ELM327 firmware only handles whole request/response pairs.
A KKL cable passes the K-Line through raw, so Caruspex can speak the protocol itself.

### Two VAG protocols, detected automatically

| Protocol | Typical for | Basic settings |
|---|---|---|
| KWP1281 (block protocol) | 1.4 MPI engines | dedicated command (block 0x28) |
| KWP2000 (ISO 14230) | 1.6 FSI with Bosch MED | dedicated diagnostic session, then read block |

Adaptation channels and output tests are currently implemented on the KWP1281 path; on KWP2000 the
application says so instead of failing silently.

Not yet implemented: manufacturer diagnostics over CAN with VW TP 2.0 (vehicles from about 2004).

## Tech stack

| Area | Technology |
|---|---|
| Language | Rust (edition 2024) |
| Interface | Tauri 2 with React 19 and TypeScript, built with Vite |
| Libraries | `serialport` (serial ports), `btleplug` (Bluetooth LE), `libloading` (J2534 drivers) |
| Localization | One set of JSON files in `translations/` for core and interface, 18 languages |
| Build | Cargo workspace, npm for the interface |
| Tests | `cargo test` against simulated ECUs, fake gateways and a fake J2534 driver |
| Tooling | Python 3 (code and table generators, translation checks, ELM327 simulator) |
| Platforms | Windows 10 and 11, Linux, macOS |

## Download

Ready-made packages are on the [releases page](https://github.com/SenseiIssei/Caruspex/releases):

| System | File | Notes |
|---|---|---|
| Windows 10 and 11, 64-bit | `Caruspex-<version>-windows-x64-setup.exe` | Installs without admin rights and can put `caruspex_cli` on the search path. It updates an installed 0.8 in place and installs the WebView2 runtime only if it is missing |
| Windows 10 and 11, 64-bit | `Caruspex-<version>-windows-x64.zip` | Portable, unzip and start. Needs the WebView2 runtime, which Windows 10 and 11 already have |
| macOS 13 and newer, Apple Silicon | `Caruspex-<version>-macos.dmg` | Drag the app to Applications |
| Linux, x86_64 | `Caruspex-<version>-linux-x86_64.AppImage` | `chmod +x` and start it |
| Debian and Ubuntu, 64-bit | `Caruspex-<version>-linux-amd64.deb` | Install with `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |
| Command-line tool only | `caruspex_cli-<version>-windows-x64.zip`, `caruspex_cli-<version>-linux-x86_64.tar.gz`, `caruspex_cli-<version>-macos.tar.gz` | For scripts and servers, no interface |

The packages are not signed yet. Windows SmartScreen asks on the first start ("More info", "Run anyway"); on macOS
open the app the first time with right click and "Open".

## Building

Requirements: the stable Rust toolchain (edition 2024, Rust 1.85 or newer), Node 24 with npm, and Python 3 for
the generators and translation checks. On Linux the system packages the interface and the adapter links need
(Debian and Ubuntu names):

```bash
sudo apt-get install libwebkit2gtk-4.1-dev librsvg2-dev libdbus-1-dev libudev-dev pkg-config
```

Windows needs the WebView2 runtime (part of Windows 10 and 11), macOS needs nothing extra.

Install the interface dependencies once, then build the command-line tool and the application:

```bash
npm --prefix app ci
```

```bash
cargo build --release -p caruspex-cli
```

```bash
npm --prefix app run tauri -- build
```

`npm --prefix app run tauri -- dev` starts the application with live reload while you work on it. The
interface alone, without the Tauri window, runs in a normal browser with the built-in demo car:

```bash
npm --prefix app run dev
```

Open `http://localhost:5287/?demo` to connect to the demo car at once. `?lang=de` picks a language and
`?theme=light` the light theme, for example `?demo&lang=de&theme=light`. Without the Tauri shell there is no
adapter access, so the browser version only knows the demo car.

### Serial port permissions on Linux

To access the adapter without `sudo`:

```bash
sudo usermod -aG dialout $USER
```

Then log out and back in.

## Build outputs

| Output | Description |
|---|---|
| `target/release/caruspex` / `caruspex.exe` | The application with the Tauri interface (`tauri build` also writes the installers and bundles to `target/release/bundle/`) |
| `target/release/caruspex_cli` / `caruspex_cli.exe` | Scriptable command-line tool on the same core, see [Command-line tool](#command-line-tool) |

### Packages and releases

`bash tools/package.sh` builds the packages of the current system into `dist/`: the installer and two zips on
Windows (the installer needs [Inno Setup 6](https://jrsoftware.org/isinfo.php)), the dmg and the command-line archive
on macOS, the AppImage, the `.deb` and the command-line archive on Linux. `bash tools/smoke_test.sh` then installs or
opens them like a user would, starts both programs and removes them again. The CI does both on every run.

A release comes from a tag. `bash tools/release.sh <version>` prepares it on its own branch without pushing anything:
it sets the version in `Cargo.toml` and `app/package.json`, creates `changelog/<version>.md` (German and English),
runs the checks, tests, packages and smoke test, and refreshes the README screenshots (`tools/screenshots.sh de` and
`tools/screenshots.sh en`). After the pull request is merged, tag `v<version>` and push the tag. The CI builds all
three systems and publishes the release with the packages and the change list.

The CI (`.github/workflows/build.yml`) first runs formatting, the generated-table check, the translation check, the
interface build and clippy on Linux, then tests, packages and smoke-tests Windows, Linux and macOS.
A second workflow (`.github/workflows/data-check.yml`) looks once a month for new VIN codes, vehicles and DBC files
upstream and opens an issue with what is new.

## Tests

```bash
cargo test --workspace
```

About 370 tests run in a few seconds, all without a car. They cover the ELM327 response parser, trouble-code
decoding, PID conversion, vehicle profiles and the VIN decoder, the OBD-II and UDS services against the demo car,
full KWP1281 and KWP2000 sessions against simulated control units, ENET (HSFZ) and DoIP against fake gateways,
SLCAN and the J2534 path against a fake driver, recordings and replay, and the analysis (health check, battery test,
trip computer, performance timer). The command-line tests run the built `caruspex_cli` and compare its output with
the golden files in `crates/caruspex-cli/tests/golden/`, which hold what the earlier C++ version printed for the same
commands, so any difference to the old tool shows up.

The checks the CI runs besides the tests:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
python tools/i18n_rust.py --strict
```

```bash
npm --prefix app run build
```

The first two are formatting and lints. `tools/i18n_rust.py --strict` fails when a text in the core is missing from
the translation files or a translation is missing in one of the 18 languages; the interface build checks its own
texts the same way when `I18N_STRICT=1` is set. Run `python tools/gen_dtc.py`, `gen_vehicles.py`, `gen_ecus.py` and
`gen_pids.py` to refresh the generated tables; the CI fails if they differ from what is checked in.

## Try it without a car

The quickest way is the built-in demo car: pick "Demo car" on the Connect page, or start with

```bash
target/release/caruspex --demo --language en
```

`caruspex --help` lists every option of the application: `--tcp`, `--port`, `--bluetooth`, `--enet`, `--can`,
`--replay`, `--report`, `--language`, `--theme`, `--page`, `--shot` and `--size`.

The Python simulator plays either the Audi A2 engine on the K-Line or, with `--can`, a CAN car with engine
and gearbox (headers, ISO-TP, modes 06 and 09 per control unit):

```bash
python tools/mock_elm327.py --can
```

Then connect the application (on Windows the binary is `target/release/caruspex.exe`):

```bash
target/release/caruspex --tcp 127.0.0.1:35000 --language en
```

The command-line tool works against the simulator as well:

```bash
target/release/caruspex_cli read --tcp 127.0.0.1:35000
```

### Command-line tool

`caruspex_cli <command>` with `--demo`, `--tcp HOST[:PORT]`, `--port NAME [--baud N]`, `--enet HOST|auto`,
`--can SPEC` or `--replay FILE`:

| Command | What it does |
|---|---|
| `read` | Warning lamp, readiness and fault codes, with the meaning for the make when the VIN gives it away |
| `vin` | VIN, what it says (maker, country, model year, check digit) and the matching car from the catalog |
| `freeze` | Every freeze frame, one per stored fault |
| `health` | Health check: faults, readiness, mode 06 and a live series (`--seconds N`, 15 by default) |
| `probe` | Adapter test: what the ELM327 really is, which commands it knows, round trip times |
| `uds` | UDS on CAN: identification and fault memory of every control unit (`--ids A,B,...` for your own addresses) |
| `custom` | Maker and custom values of a built-in pack (`--pack NAME`) or a Torque CSV (`--csv FILE`), once |
| `live`, `info`, `clear`, `ecus`, `ports`, `kklcheck` | Live values, VIN and first freeze frame, clearing, all per control unit, serial ports, KKL cable self-test |
| `enet`, `interfaces` | Looks for BMW and Mini cars and DoIP cars on the network; lists the CAN interfaces of this computer (J2534 drivers, SocketCAN) |
| `version`, `help` | Version of the program, the full list of commands and options |

`--json` prints `read`, `vin`, `health`, `probe`, `freeze`, `uds` and `custom` as JSON for scripts, `--record FILE`
saves the whole session with the adapter for `--replay`, `--fast-timing` switches the adapter to aggressive adaptive
timing, `-v` logs every adapter command (to stderr together with `--json`). `caruspex_cli help` lists everything.

```bash
target/release/caruspex_cli health --demo --json > health.json
```

The screenshots on this page come from the demo car, taken by the app itself:
`--demo --page faults --shot shot.png --size 1440x920` opens a page, saves a picture of the window after a
few seconds and quits (`tools/screenshots.sh` takes the whole set).

## Project structure

```
crates/caruspex-i18n/       Languages: German and English in the code, the other 16 from translations/
crates/caruspex-vehicles/   VIN decoder and the vehicle catalog (brands, models, generations, sockets, buses)
crates/caruspex-data/       Trouble codes, PIDs, control unit addresses, code-to-component mapping
crates/caruspex-link/       Ways to the adapter (serial, TCP, Bluetooth LE), ELM327 driver, adapter check
crates/caruspex-protocols/  OBD-II, UDS, KWP2000 and KWP1281 (K-Line, byte by byte)
crates/caruspex-net/        BMW ENET (HSFZ), DoIP, SLCAN, SocketCAN, J2534 PassThru
crates/caruspex-record/     Recordings and replay, custom values (Torque CSV, PID packs), JSON writer
crates/caruspex-analysis/   Health check, drive cycle guide, trip computer, performance timer, battery test
crates/caruspex-demo/       The built-in demo car: an ELM327 with simulated control units
crates/caruspex-cli/        The command-line tool, with its golden-file tests
app/src/                    The interface: React 19 and TypeScript (pages, shell, state, report, i18n)
app/src-tauri/              The Tauri application: window, commands, update check, command-line options
translations/               Shared translation files for core and interface, 18 languages
data/                       The vehicle catalog as hand-maintained CSV, control unit and PID sources
tools/                      Generators, translation checks, packaging, release and screenshot scripts, ELM327 simulator
docs/                       Screenshots of the README
packaging/                  Windows installer script, Linux desktop file, macOS Info.plist
changelog/                  Release notes per version, German and English
```

The trouble-code database is generated: `tools/gen_dtc.py` expands the regular EOBD code families and the
curated lists in `tools/dtc/` into `crates/caruspex-data/src/generated/*.rs`. Re-run it after changing the data:

```bash
python tools/gen_dtc.py
```

The vehicle catalog works the same way: edit the CSV files in `data/vehicles/`, then run

```bash
python tools/gen_vehicles.py
```

Control unit addresses and PID packs have `tools/gen_ecus.py` and `tools/gen_pids.py`. Translations are described
in [translations/TRANSLATING.md](translations/TRANSLATING.md); after adding a text in the code,
`python tools/i18n_rust.py` lists what still needs translating.

## A note on clearing codes

OBD mode 04 clears the fault memory, the freeze frame and the readiness monitors. Right afterwards the
car will fail an emissions test until it has completed several full drive cycles, and a code whose cause
is still present will come back after a short drive. Clearing is not a repair.

## Disclaimer

Caruspex is a hobby project and is provided without warranty. Use it at your own risk, only on vehicles
you are allowed to work on, and never while driving. All product and brand names are trademarks of their
respective owners; Caruspex is not affiliated with or endorsed by any vehicle manufacturer.

## License

MIT, see [LICENSE](LICENSE). The bundled fonts are licensed under the SIL Open Font License 1.1:
JetBrains Mono ([OFL.txt](resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](resources/fonts/OFL-Inter.txt)) and
Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](resources/fonts/OFL-PlusJakartaSans.txt)).
