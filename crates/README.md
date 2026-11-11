# The Caruspex crates

Caruspex is one Cargo workspace: a set of small crates for the diagnosis
core, a command-line tool, and the Tauri app in `app/`. Up to version 0.8 it
was written in C++ with Qt; the Rust version behaves the same (same bytes on
the wire, same timeouts, same texts) and the old code stays in the git
history and in the v0.8.0 release.

## Crates

| Crate | What it holds | Builds on |
|---|---|---|
| caruspex-i18n | the 18 languages: `l(de, en)`, `Text`, the catalog built from `translations/` at compile time | |
| caruspex-vehicles | VIN decoding, the WMI table, the vehicle catalog (generated from `data/vehicles/`), the shape of a car for the X-ray view with real dimensions per generation from Wikidata (generated from `data/wikidata/`) | i18n |
| caruspex-data | trouble codes, PIDs, monitor tests, control unit addresses (generated from `data/ecus/`), VAG fault numbers, fault locations | i18n |
| caruspex-link | the `Transport` and `ByteStream` traits, serial with USB detection, TCP, K-line, Bluetooth LE and classic Bluetooth, the ELM327 driver, the adapter check | i18n |
| caruspex-protocols | OBD-II with batched requests, UDS with BMW addressing and the control unit scan, KWP1281, KWP2000, the VAG K-line | link, data |
| caruspex-net | BMW ENET (HSFZ), DoIP, raw CAN interfaces (SLCAN, SocketCAN, J2534) with their own ISO-TP and an ELM327 front end, CAN signals | link |
| caruspex-record | recordings and replay, custom PIDs with Torque equations, the built-in PID packs (generated from `data/pids/`), JSON output | link, protocols, net, data |
| caruspex-analysis | health check, battery test, readiness, drive cycles, trip computer, performance timer | i18n |
| caruspex-advice | plain advice: "what should I do now?" in three steps from faults, readiness, health check and live values, and the checklist of the connection assistant | i18n, data, analysis |
| caruspex-demo | the built-in demo car: an ELM327 with simulated control units and an injectable clock | link |
| caruspex-cli | `caruspex_cli`, the command-line tool | all of the above |
| app/src-tauri | the desktop app: the worker thread, settings, garage, report, the signed updater | all of the above |

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
- `unsafe` only where the operating system needs it (J2534, SocketCAN,
  classic Bluetooth), kept inside those modules.
- No long dashes (U+2014, U+2013) in code, comments or texts.

## Checks

    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    python tools/i18n_rust.py --strict
    I18N_STRICT=1 npm --prefix app run build
