# Caruspex

[![Build](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml/badge.svg)](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml)
![C++20](https://img.shields.io/badge/C%2B%2B-20-blue)
![Qt 6](https://img.shields.io/badge/Qt-6-41cd52)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Open-source vehicle diagnostics for Windows, Linux and macOS, written in C++20 with Qt 6.
One cable or dongle, one application: read and clear trouble codes, watch live sensor data,
check emission readiness, and talk to VW/Audi control units at the byte level over a K-Line cable.

The name comes from the *haruspex*, the Roman seer who read omens from the entrails. Caruspex reads the car's.

The application is available in English and German: it follows the system language, can be switched in
the settings, or forced per run with `--language en` / `--language de`. [Deutsche Fassung dieser Seite](README.de.md)

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
- **Portable, Qt-free core**: transport, adapter drivers, protocols and data live in a plain C++20 library
  using `std::chrono`. Only the serial port is OS-specific (`Serial_win.cpp` / `Serial_posix.cpp`).
- **Tested without a car**: a simulated ECU (`tests/FakeEcu.cpp`, `tests/FakeEcu2000.cpp`) reproduces the
  byte-by-byte handshake, plus a Python ELM327 simulator for end-to-end runs of the GUI.
- **Bilingual without Qt in the core**: the GUI uses Qt Linguist (`translations/caruspex_en.ts`), while the
  Qt-free core resolves its texts (trouble codes, PID names, protocol messages) through a tiny `L(de, en)` helper.
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

### All control units over CAN (UDS)
- With a plain ELM327, Caruspex asks every control unit one by one: engine, gearbox, ABS, airbag, instrument cluster, climate and more, on most cars from about 2008.
- Fault memory per unit with the failure type (short circuit, open circuit, implausible ...), part number, software and hardware version.
- Address lists for VAG, Ford, Mazda, GM, Toyota, Nissan, Hyundai/Kia, Renault and PSA; for everything else a sweep over the usual addresses.
- Clearing per unit, with a warning.

### VW and Audi over a KKL cable (KWP1281 and KWP2000)
- Throttle body adaptation: the real basic setting in measuring block 060 or 098.
- Scan every control unit, including ABS, airbag, instrument cluster and comfort, with a printable report.
- Adaptation channels and output tests, deliberately read-only.

### Manufacturer and custom values
- Electric car batteries: state of health, charge, cell voltages with spread, temperatures, for Hyundai Kona and Ioniq, Kia e-Niro, Renault Zoe and Nissan Leaf; every value with a documented source.
- Load your own values in the widely used Torque CSV format, with a complete evaluator for Torque equations.

### Record and replay
- Record a session with the car into a file and play it back later without the car, handy for troubleshooting.

### Without a car or adapter
- A built-in demo car: it drives, warms up, has a few faults stored and talks like a real ELM327 over CAN.
- An offline lexicon of all codes, filterable by brand.

### Look and feel
- A new interface in the style of Odysync: calm, dark, with smooth transitions.
- Step-by-step connecting with an automatic search over all ports.
- English and German; settings apply immediately.

## Supported vehicles

Any vehicle with OBD-II or EOBD: in Europe petrol cars from 2001 and diesels from 2004, in the US from 1996. Older VW, Audi, Seat and Skoda from about 1990 work over the KKL cable. The Vehicles page tells you which way works for a given car.

| Era | Bus | Examples |
|---|---|---|
| until about 1995 | manufacturer specific | VAG with KKL through the 2x2 plugs, otherwise workshop tools only |
| 1996 to about 2003 | K-Line (ISO 9141-2, KWP2000) | Audi A2, VW Golf IV, Toyota, Renault |
| from about 2004 | CAN (ISO 15765-4) | Audi A4 B7, VW Golf V, modern Japanese cars |
| US until about 2005 | SAE J1850 PWM and VPW | Ford, Chevrolet |

Pure electric cars do not have to offer OBD-II emissions data. Some answer partly anyway, Tesla not at all through the standard route. Caruspex says so instead of trying.

## Supported adapters

- Any **ELM327**-compatible adapter: USB, classic Bluetooth (paired, appears as a serial port), **Bluetooth LE**
  (Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE and other adapters with the usual GATT services; found by a
  scan, no pairing) or Wi-Fi via TCP
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
| Language | C++20 |
| GUI | Qt 6 Widgets and Qt Charts, frameless window with custom title bar |
| Localization | Qt Linguist for the GUI, `L(de, en)` text pairs in the core |
| Build | CMake 3.21+, Conan 2 |
| Tests | Catch2 3, simulated ECUs, CTest |
| Tooling | Python 3 (code generator, ELM327 simulator) |
| Platforms | Windows (MSVC 2022), Linux (GCC 12+), macOS (Clang 15+) |

## Download

Ready-made packages are on the [releases page](https://github.com/SenseiIssei/Caruspex/releases):

| System | File | Notes |
|---|---|---|
| Windows 10 and 11, 64-bit | `Caruspex-<version>-windows-x64-setup.exe` | Installs without admin rights and can put `caruspex_cli` on the search path. A portable `.zip` is there as well |
| macOS 13 and newer, Apple Silicon | `Caruspex-<version>-macos.dmg` | Drag the app to Applications, `caruspex_cli` is next to it |
| Linux, x86_64 | `Caruspex-<version>-linux-x86_64.AppImage` | `chmod +x` and start it. The command-line tool alone is in `caruspex_cli-<version>-linux-x86_64.tar.gz` |

The packages are not signed yet. Windows SmartScreen asks on the first start ("More info", "Run anyway"); on macOS
open the app the first time with right click and "Open".

## Building

Requirements: a C++20 compiler (MSVC 2022, GCC 12 or Clang 15 and newer), CMake 3.21+, Python 3,
and Qt 6.5+ with the Charts module.

The quickest way to get Qt is `aqtinstall` (building Qt through Conan compiles it from source and takes hours):

```bash
pip install aqtinstall conan
```

### 1. Install Qt

Windows:

```bash
python -m aqt install-qt windows desktop 6.8.3 win64_msvc2022_64 -m qtcharts -O C:/Qt
```

Linux:

```bash
python -m aqt install-qt linux desktop 6.8.3 linux_gcc_64 -m qtcharts -O ~/Qt
```

macOS:

```bash
python -m aqt install-qt mac desktop 6.8.3 clang_64 -m qtcharts -O ~/Qt
```

On Linux, Qt also needs the usual OpenGL and XCB system packages, for example on Ubuntu:

```bash
sudo apt-get install libgl1-mesa-dev libxkbcommon-x11-0 libxcb-cursor0 libxcb-icccm4 libxcb-image0 libxcb-keysyms1 libxcb-randr0 libxcb-render-util0 libxcb-shape0 libxcb-xinerama0 libxcb-xkb1
```

### 2. Dependencies and build (same on every platform)

```bash
conan profile detect --force
```

```bash
conan install . --output-folder=build/conan --build=missing -s compiler.cppstd=20
```

Point `CMAKE_PREFIX_PATH` at your Qt installation:
`C:/Qt/6.8.3/msvc2022_64` on Windows, `~/Qt/6.8.3/gcc_64` on Linux, `~/Qt/6.8.3/macos` on macOS.

```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE=build/conan/conan_toolchain.cmake -DCMAKE_PREFIX_PATH=C:/Qt/6.8.3/msvc2022_64
```

```bash
cmake --build build --config Release
```

`cmake` must be on the `PATH`, otherwise Conan cannot build its packages.

To run the application outside the build tree, deploy the Qt libraries next to it with `windeployqt`
(Windows) or `macdeployqt` (macOS).

CMake options:

| Option | Default | Meaning |
|---|---|---|
| `CARUSPEX_GUI` | `ON` | Build the Qt application (`OFF` builds only the core and the CLI, no Qt needed) |
| `CARUSPEX_TESTS` | `OFF` | Build the Catch2 test suite |

### Serial port permissions on Linux

To access the adapter without `sudo`:

```bash
sudo usermod -aG dialout $USER
```

Then log out and back in.

## Build outputs

| Output | Description |
|---|---|
| `caruspex` / `caruspex.exe` / `caruspex.app` | GUI application |
| `caruspex_cli` / `caruspex_cli.exe` | Scriptable command-line tool on the same core, see [Command-line tool](#command-line-tool) |

### Packages and releases

After a Release build, `bash tools/package.sh` writes the packages of the current system into `dist/`: the installer
and a zip on Windows (needs [Inno Setup 6](https://jrsoftware.org/isinfo.php) for the installer), the dmg on macOS,
the AppImage on Linux. `bash tools/smoke_test.sh` then installs them like a user would, starts both programs and
removes them again. The CI does both on every run.

A release comes from a tag: set the version in `CMakeLists.txt`, write `changelog/<version>.md` (German and
English), tag `v<version>` and push the tag. The CI builds all three systems and publishes the release with the
packages and the change list.

The app icon is drawn in code (`src/app/AppIcon.cpp`). After changing it, `caruspex --render-icon build/icon` and
`python tools/gen_icons.py build/icon` refresh the files in `resources/icons/`.

## Tests

```bash
cmake -S . -B build -DCARUSPEX_TESTS=ON -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE=build/conan/conan_toolchain.cmake -DCMAKE_PREFIX_PATH=C:/Qt/6.8.3/msvc2022_64
```

```bash
cmake --build build --config Release
```

```bash
ctest --test-dir build -C Release --output-on-failure
```

The suite covers the ELM327 response parser, trouble-code decoding, PID conversion, vehicle profiles,
settings, and full KWP1281/KWP2000 sessions against simulated control units. No car required.

## Try it without a car

The quickest way is the built-in demo car: pick "Demo car" on the Connect page, or start with

```bash
build/bin/Release/caruspex.exe --demo --language en
```

The Python simulator plays either the Audi A2 engine on the K-Line or, with `--can`, a CAN car with engine
and gearbox (headers, ISO-TP, modes 06 and 09 per control unit):

```bash
python tools/mock_elm327.py --can
```

Then connect the GUI (Windows path shown; on Linux and macOS the binary is in `build/bin/`):

```bash
build/bin/Release/caruspex.exe --tcp 127.0.0.1:35000 --language en
```

The command-line tool works against the simulator as well:

```bash
build/bin/Release/caruspex_cli.exe read --tcp 127.0.0.1:35000
```

### Command-line tool

`caruspex_cli <command>` with `--demo`, `--tcp HOST[:PORT]` or `--port NAME [--baud N]`:

| Command | What it does |
|---|---|
| `read` | Warning lamp, readiness and fault codes, with the meaning for the make when the VIN gives it away |
| `vin` | VIN, what it says (maker, country, model year, check digit) and the matching car from the catalog |
| `freeze` | Every freeze frame, one per stored fault |
| `health` | Health check: faults, readiness, mode 06 and a live series (`--seconds N`, 15 by default) |
| `probe` | Adapter test: what the ELM327 really is, which commands it knows, round trip times |
| `live`, `info`, `clear`, `ecus`, `ports`, `kklcheck` | Live values, VIN and first freeze frame, clearing, all per control unit, serial ports, KKL cable self-test |

`--json` prints `read`, `vin`, `health`, `probe` and `freeze` as JSON for scripts, `-v` logs every adapter
command (to stderr together with `--json`). `caruspex_cli help` lists everything.

```bash
build/bin/Release/caruspex_cli.exe health --demo --json > health.json
```

The screenshots on this page come from the demo car, taken by the app itself:
`--demo --page faults --shot shot.png --size 1440x920` opens a page, saves a picture of the window after a
few seconds and quits.

## Project structure

```
src/core/           Transport, ELM327 driver, OBD services, KWP1281, KWP2000, demo car, adapter check. No Qt.
src/core/analysis/  Health check, drive cycle guide, trip computer, performance timer, battery test
src/data/           Trouble codes, PIDs, VIN decoder, vehicle catalog, code-to-component mapping
src/app/            Qt 6 user interface (Widgets, Charts, Svg, PrintSupport)
src/cli_main.cpp    Command-line tool on the same core
data/vehicles/      The vehicle catalog as hand-maintained CSV
translations/       Qt Linguist translation of the GUI (German source, English target)
tools/              Generators for trouble codes and the catalog, ELM327 simulator
tests/              Catch2 tests and simulated ECUs
```

The trouble-code database is generated: `tools/gen_dtc.py` expands the regular EOBD code families and the
curated lists in `tools/dtc/` into `src/data/*.gen.cpp`. Re-run it after changing the data:

```bash
python tools/gen_dtc.py
```

The vehicle catalog works the same way: edit the CSV files in `data/vehicles/`, then run

```bash
python tools/gen_vehicles.py
```

After changing GUI strings, refresh the translation file and fill in the new entries (e.g. with Qt Linguist):

```bash
cmake --build build --target update_translations
```

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
