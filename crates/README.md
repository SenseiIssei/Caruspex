# The Caruspex crates

Caruspex is one Cargo workspace: a set of small crates for the diagnosis
core, a command-line tool, and the Tauri app in `app/`. Up to version 0.8 it
was written in C++ with Qt; the Rust version behaves the same (same bytes on
the wire, same timeouts, same texts) and the old code stays in the git
history and in the v0.8.0 release.

## Crates

| Crate | What it holds | Builds on |
|---|---|---|
| caruspex-i18n | the 18 languages: `l(de, en)`, `Text`, the catalogs built from `translations/` at compile time, packed per language and unpacked when first needed | |
| caruspex-vehicles | VIN decoding, the WMI table, the vehicle catalog (generated from `data/vehicles/`, with motorcycles and their diagnostic plugs, trucks with SAE J1939 and their 9-pin socket, the exact socket spot from `data/obd_sockets.csv` and the engine codes from Wikipedia in `data/engines/`), the shape of a car for the X-ray view with real dimensions per generation from the makers' technical data and Wikidata (generated from `data/dimensions_manual.csv` and `data/wikidata/`) | i18n |
| caruspex-data | trouble codes, PIDs, monitor tests, control unit addresses (generated from `data/ecus/`), VAG fault numbers, fault locations | i18n |
| caruspex-link | the `Transport`, `ByteStream` and `FrameLink` (29-bit frames for SAE J1939) traits, serial with USB detection, TCP, K-line, Bluetooth LE and classic Bluetooth, the ELM327 driver with the extras of STN chips (batched commands, monitoring with a count, MS-CAN and SW-CAN) and SAE J1939 in protocol A, the adapter check | i18n |
| caruspex-j2534 | SAE J2534 PassThru drivers: the driver DLL in the program, or a 32-bit one in the helper program `caruspex-j2534-bridge.exe` (built for i686-pc-windows-msvc, shipped next to caruspex.exe), which Caruspex talks to over a named pipe | |
| caruspex-protocols | OBD-II with batched requests, UDS with BMW addressing and the control unit scan, KWP1281, KWP2000, the VAG K-line, KWP2000 over VAG TP 2.0, SAE J1939 for trucks (faults DM1/DM2, transport protocol, address claim, broadcast values) | link, data |
| caruspex-net | BMW ENET (HSFZ), DoIP, raw CAN interfaces (SLCAN, SocketCAN, J2534) with their own ISO-TP and an ELM327 front end shared by all of them, SAE J1939 on them (`J1939Can`), VAG TP 2.0, the K-line of J2534 devices, CAN signals | link, j2534 |
| caruspex-record | recordings and replay, custom PIDs with Torque equations, the built-in PID packs (generated from `data/pids/` and, for listening to broadcast values, from opendbc in `data/opendbc/`), JSON output | link, protocols, net, data |
| caruspex-analysis | health check, battery test, readiness, drive cycles, trip computer, performance timer | i18n |
| caruspex-advice | plain advice: "what should I do now?" in three steps from faults, readiness, health check and live values, and the checklist of the connection assistant | i18n, data, analysis |
| caruspex-demo | the built-in demo car: an ELM327 or an OBDLink (STN commands, MS-CAN and SW-CAN) with simulated control units, and a demo truck on SAE J1939, an injectable clock and a count of the simulated adapter time | link |
| caruspex-pdf | a small PDF 1.4 writer without dependencies: the standard fonts (Helvetica, Courier) with WinAnsiEncoding, explicit stand-ins for characters it lacks ("?" and a note for anything else), lines, filled boxes, pages, and a flow layout with wrapped text, tables whose rows stay on one page and page numbers | |
| caruspex-tune | the tuning simulator: a transparent full-load model of the BMW N55 (air path, turbo and intercooler, knock limit, friction, exhaust temperature, fuel system, gearbox) with plain warnings, and the comparison of recorded live values with it. Computes only: no link, no protocol, nothing that reaches a car, no map writing | i18n |
| caruspex-cli | `caruspex_cli`, the command-line tool, with the PDF report (`report`) | all of the above |
| app/src-tauri | the app: the worker thread, settings, garage, report, the signed updater; a library (`lib.rs`) that `main.rs` starts on the desktop and the Android project in `gen/android` loads, with USB and classic Bluetooth adapters through `android.rs` and its Kotlin plugin (see `docs/android.md`) | all of the above |

## Conventions

- Errors are `Result<T, String>`; the text is the user-facing message.
- Texts: `caruspex_i18n::l("de", "en")` or `Text::new("de", "en")`. The other
  16 languages are looked up by the English text, so a changed English text
  loses its translations until `tools/i18n_rust.py` lists it again and it is
  translated (see `translations/TRANSLATING.md`).
- Bytes on the wire are `Vec<u8>` / `&[u8]`.
- Everything that talks to a car goes through `caruspex_link::Transport`
  (the ELM327 dialogue; ENET, CAN and the demo car emulate an ELM327) or
  `ByteStream` (K-line bytes). Timing helpers are in `caruspex_link::timing`.
- Generated tables live in `crates/<crate>/src/generated/`, written by the
  Python generators in `tools/` and never edited by hand; the CI checks that
  they are up to date. The generated modules are excluded from rustfmt.
- Tests run against simulated control units, fake gateways, a fake J2534
  driver and the demo car; the CLI is compared with golden files captured
  from the 0.8 version. No test needs hardware.
- `unsafe` only where the operating system needs it (J2534 in
  caruspex-j2534, SocketCAN, classic Bluetooth), kept inside those modules.
- No long dashes (U+2014, U+2013) in code, comments or texts.

## Checks

`bash tools/check_all.sh` runs all of them and the rest of the list in
CONTRIBUTING.md. The Rust part alone:

    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    python tools/i18n_rust.py --strict
    I18N_STRICT=1 npm --prefix app run build
