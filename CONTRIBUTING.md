# Contributing to Caruspex

Thank you for wanting to help. Caruspex is a free diagnostics program for cars, motorcycles and trucks, and
most of it was built against simulated control units. Every test on a real car, every fixed text and every
careful pull request makes it more useful for people who own a car and an adapter.

Kurz auf Deutsch steht [am Ende](#kurz-auf-deutsch). Reports and pull requests in German are welcome too.

- [Ways to help without code](#ways-to-help-without-code)
- [Build and run](#build-and-run)
- [Developer switches](#developer-switches)
- [What a pull request must pass](#what-a-pull-request-must-pass)
- [How texts and translations work](#how-texts-and-translations-work)
- [House rules](#house-rules)
- [Safety rules for anything that talks to a car](#safety-rules-for-anything-that-talks-to-a-car)
- [Common changes, step by step](#common-changes-step-by-step)
- [Pull requests](#pull-requests)

How the pieces fit together is in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), the crates and their
conventions in [crates/README.md](crates/README.md).

## Ways to help without code

- **Test on a real car.** Fill in the [test report form](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml)
  (`.github/ISSUE_TEMPLATE/test-report.yml`). A recording made with **Share recording** in the app helps most:
  it has the last digits of the VIN, the port and adapter addresses and the time of day removed, and the app
  shows what stays before you save it. Never post a full VIN, a number plate or your name.
- **Report a bug** or **suggest a feature** with the forms under [New issue](https://github.com/SenseiIssei/Caruspex/issues/new/choose).
- **Improve a translation**: see [translations/TRANSLATING.md](translations/TRANSLATING.md).
- **Correct the documentation**: the website is built from `docs/site/content/<language>/*.md`.
- **Report a security problem** privately, as described in [SECURITY.md](SECURITY.md), not in a public issue.
  What the EU Cyber Resilience Act means for Caruspex (scope, risk assessment, requirements) is in
  [docs/cra/](docs/cra/scope.md).

## Build and run

### What you need

- **Rust**: the stable toolchain, at least the `rust-version` in the root `Cargo.toml` (1.90 at the moment),
  with `rustfmt` and `clippy` (`rustup component add rustfmt clippy`).
- **Node**: Node 24 (the LTS line the CI uses) with npm.
- **Python 3** (the CI uses 3.12) for the generators and checks in `tools/`. Only the standard library is
  needed for the checks. The examples below say `python`; on Windows use `py -I`.
- **Linux**: the system packages of the interface and the adapter links (Debian and Ubuntu names):

  ```bash
  sudo apt-get install libwebkit2gtk-4.1-dev librsvg2-dev libdbus-1-dev libudev-dev pkg-config
  ```

- **Windows**: the WebView2 runtime (part of Windows 10 and 11). Optional: `rustup target add i686-pc-windows-msvc`
  for the test of the 32-bit J2534 bridge; without the target that test is skipped.
- **macOS**: nothing extra.

### First build

```bash
npm --prefix app ci
npm --prefix app run build
cargo run -p caruspex-app -- --demo
```

`npm --prefix app run build` writes the interface to `app/dist/`. The app has the interface built in (the
default feature `custom-protocol`), so `cargo build`, `cargo run`, `cargo test` and `cargo clippy` of the app
need `app/dist/` to exist. Build the interface first after a fresh clone and after changes in `app/src/`.

The command-line tool on the same core: `cargo run -p caruspex-cli -- read --demo`
(`cargo run -p caruspex-cli -- help` lists every command).

### Working on it

| Command | What it gives you |
|---|---|
| `npm --prefix app run dev:app` | The app with live reload: the Tauri window loads the Vite dev server (`localhost:5287`) instead of the built-in interface. Pick the demo car on the Connect page. |
| `npm --prefix app run dev` | The interface alone in a normal browser, with a simulated backend (`app/src/api/mock/`) instead of Rust. Open `http://localhost:5287/?demo`. |
| `npm --prefix app run build:demo` | The same as a release build in `app/dist-demo/` (`npx --prefix app vite preview --outDir dist-demo`). `npm --prefix app run build` leaves the simulated backend out, so its `app/dist/` only works inside the app. |
| `cargo run -p caruspex-app -- --demo` | The app as users get it, connected to the built-in demo car. |
| `python tools/mock_elm327.py --can` | A simulated ELM327 on `127.0.0.1:35000` (without `--can`: an Audi A2 engine on the K-line). Connect with `--tcp 127.0.0.1:35000`. |

The browser preview reads its options from the address: `?demo`, `?lang=de`, `?theme=light`, `?page=faults`,
`?mode=beginner`, `?contrast=high`, for example `http://localhost:5287/?demo&lang=de&theme=light`. It has no
adapter access, so it only knows the demo car.

### Nobody needs a car

`--demo` connects to the built-in demo car (`crates/caruspex-demo`): a simulated ELM327 with an engine,
a gearbox and several control units that only answer UDS, a consistent drive cycle, fault codes, freeze
frames, readiness and clearing. `CARUSPEX_DEMO_ADAPTER=obdlink-mx` (or `obdlink-ex`) makes it answer like an
OBDLink with its extra buses. `--replay <file.jsonl>` plays back a recorded session, for example one that
somebody attached to a test report. All tests run against simulated control units, fake gateways, a fake J2534
driver and the demo car; no test needs hardware.

### Speed and memory

Caruspex should answer quickly and stay small. [docs/performance.md](docs/performance.md) has the benchmark
harness (the demo car behind a simulated serial line with the delays of a real ELM327), the numbers it gives,
the guard tests that fail when the live loop gets clearly slower or hungrier, and the commands for the full
report and the ten minute soak. Run the report before and after a change to the driver, the protocols or the
worker.

## Developer switches

The app's options are parsed in `app/src-tauri/src/cli.rs`; `caruspex --help` lists the ones meant for users.
These help most while developing:

| Switch | What it does |
|---|---|
| `--demo` | Connect to the demo car at start. |
| `--replay <file.jsonl>` | Play back a recording instead of talking to an adapter. |
| `--page <name>` | Start on this page. The names are the ids in `app/src/shell/pages.tsx` (`faults`, `live`, `xray`, `used-car`, `settings`, ...). A few extra views exist for screenshots and tests: `setup`, `setup-socket`, `setup-ignition`, `setup-connect`, `term-hint`, `contrast` (`app/src/shell/App.tsx`), `connect-bmw` (`ConnectPage.tsx`), `kkl-firsttest` and `kkl-cabletest` with an optional fault of the simulated A2 (`kkl-firsttest:slow-read`; the faults are in `crates/caruspex-demo/src/a2.rs`, `KklFirstTestPage.tsx`) `xray-generation:<id or search>` with more parts behind it (`XrayPage.tsx`, see `tools/xray_shots.py`) and `tune-demo` or `tune-demo:compare`, the tuning simulator with a changed map and the example comparison (`TunePage.tsx`). |
| `--mode <beginner\|pro>` | Beginner or pro mode for this run, without changing the setting. Most workshop pages are pro only. |
| `--language <code>` | Language for this run: `de`, `en`, `fr`, `ar` and the other codes of `translations/`. (It is `--language`, not `--lang`.) |
| `--theme <dark\|light>` | Appearance for this run. |
| `--shot <file.png>` | Save a picture of the window after a few seconds and quit; `--shot-delay <ms>` (default 5200) and `--size <WxH>` (for example `1440x920`) go with it. No update check runs. |
| `--window-pos <x,y>` | Where the window opens, in logical pixels, without taking the focus, so test runs stay off the screen you work on. |
| `--report <file>` | After connecting, write a diagnostic report (PDF, or HTML for `.html`) and quit; `--report-wait <ms>` (default 8000). |
| `--tcp`, `--port`, `--bluetooth`, `--enet`, `--can` | Connect to a real adapter at start, see `caruspex --help`. |

Environment variables for tests: `CARUSPEX_GARAGE=<file.json>` uses a throwaway garage instead of yours (the
screenshot and smoke test scripts do that), `CARUSPEX_CRASH_DIR` moves the crash reports.

Example: `cargo run -p caruspex-app -- --demo --mode pro --language en --page faults --shot faults.png --size 1440x920`.

## What a pull request must pass

The CI (`.github/workflows/build.yml`) runs these on every pull request. Run them before you push; all of them
work offline and without secrets. One command runs all of them in the order of the CI, keeps going after a
failure and ends with a list of every step with its time:

```bash
bash tools/check_all.sh              # everything below
bash tools/check_all.sh --no-npm-ci  # keep app/node_modules, faster on a second run
bash tools/check_all.sh --android    # plus clippy of the app for Android (needs ANDROID_HOME with an NDK)
```

The single steps, on Linux and macOS:

```bash
cargo fmt --all --check

# Generated tables are up to date (the CI fails on any difference)
python tools/gen_dtc.py
python tools/gen_vehicles.py
python tools/gen_ecus.py
python tools/gen_pids.py
python tools/gen_engines.py
python tools/gen_wmi.py
python tools/gen_obdb.py --fixtures
python tools/gen_dimensions.py
python tools/gen_dimensions.py --fixtures
python app/scripts/extract-catalog.py
git status    # nothing may have changed under crates/*/src/generated, the test fixtures or app/src/api/mock/data

python tools/i18n_rust.py --strict
python tools/readme_languages.py --check
python docs/site/build.py --check
python tools/dist_manifests.py --check-templates

npm --prefix app ci
python tools/gen_notices.py --check
I18N_STRICT=1 npm --prefix app run build
I18N_STRICT=1 npm --prefix app test
python tools/bundle_budget.py
python tools/sbom.py --check

cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

On Windows run the Python lines as `py -I tools/...` and set `I18N_STRICT` in PowerShell with
`$env:I18N_STRICT = "1"` before the npm lines.

What else runs in the CI:

- **Build** on Windows, Linux and macOS: the interface, `cargo test --workspace`, the packages
  (`bash tools/package.sh`) and a smoke test that installs the package, starts both programs and removes them
  again (`bash tools/smoke_test.sh smoke`). You rarely need these locally.
- **Android**: an unsigned APK for arm64 (see [docs/android.md](docs/android.md)).
- **Sanitizers**: AddressSanitizer and Rust's undefined behaviour checks on the modules with `unsafe` code,
  on nightly. It may fail without blocking a pull request.
- **SBOM**: `python tools/sbom.py --check` builds the software bills of materials (CycloneDX) of every platform
  from `Cargo.lock` and `app/package-lock.json` and checks them; the release attaches them.
- **Audit** (`.github/workflows/audit.yml`): `python tools/audit.py` looks up every crate and the npm packages of
  the interface in the RustSec and npm advisory databases, on dependency changes and once a week. It needs
  `cargo install cargo-audit --locked`. A finding that does not affect Caruspex goes into
  `tools/audit_exceptions.toml` with a reason and an expiry date ([SECURITY.md](SECURITY.md#dependencies)).

Pull requests from forks run all of this without any secrets. Signing and the release only run for version
tags in the main repository, and only the main repository publishes the documentation website. GitHub may ask
the maintainer to approve the workflow runs of a first-time contributor, so the checks of your first pull
request can wait a little.

### Dependency updates

Dependabot (`.github/dependabot.yml`) opens a pull request each month for the crates, the npm packages, the
GitHub Actions and the Python packages of the release job. Any change of `Cargo.lock`, `app/package-lock.json`
or the Android libraries also changes `THIRD_PARTY_NOTICES.md`, so the step **Third party notices are current**
fails on such a pull request until the file is regenerated. The step writes the new file and attaches it to the
run as the artifact `third-party-notices`; the error message names the run. On the branch of the pull request:

```bash
# Either regenerate it
npm --prefix app ci
python tools/gen_notices.py
# or take the file the run made
gh run download <run id> -n third-party-notices --repo SenseiIssei/Caruspex

git diff THIRD_PARTY_NOTICES.md    # only new or changed packages and licences
git commit -m "Third party notices" THIRD_PARTY_NOTICES.md
```

This stays a manual step on purpose. A workflow that commits to the pull request by itself would need a token
with write access in a run that installs and executes what the pull request brings (npm install scripts, Cargo
build scripts); that is the kind of run that can leak the token. The CI only hands the file over, with the
read-only token every pull request gets. The SBOMs need nothing: they are built from the lock files in every
run and never committed.

Actions are pinned by commit SHA with the version in a comment (`uses: actions/checkout@<sha> # v7.0.1`), and
the Python packages of the release job by version and hash (`tools/requirements-release.txt`); Dependabot updates
both. Pin a new action the same way: `gh api repos/<owner>/<repo>/git/ref/tags/<tag>` gives the SHA (for an
annotated tag follow `object.url` to the commit).

## How texts and translations work

Caruspex speaks 18 languages. German and English are written in the code, the other 16 come from JSON files.
The full guide is [translations/TRANSLATING.md](translations/TRANSLATING.md); the short version:

- **Rust (core texts)**: write `caruspex_i18n::l("German", "English")` or `Text::new("German", "English")`. The
  German text is the source, the English one is the key the other languages are looked up by, so changing an
  English text drops its translations until it is translated again.
  1. `python tools/i18n_rust.py` lists the texts that are not in `translations/source/core_*.json` yet.
  2. `python tools/i18n_rust.py --write --area NAME` puts them into their own file
     `translations/source/core_NAME.json` with ids `c-NAME-1`, `c-NAME-2`, ... Choose a short NAME for your piece
     of work (`--area used-car`); separate files keep parallel branches from handing out the same id.
  3. Add the translations as `translations/<code>/core_NAME.json`, one JSON object from id to text, for all 16
     other languages.
- **Interface (TypeScript)**: pages call `translator("caruspex::SomePage")` and write `tr("German text")`. A new
  text goes into `app/src/i18n/new-texts/<area>.json` with `context`, `de`, `en` and a `tr` object with the text
  for every other language code. `app/scripts/gen-i18n.mjs` builds the catalogs in `app/src/i18n/generated/`
  (not in git) on every `npm --prefix app run build`.
- `python tools/i18n_rust.py --strict` and `I18N_STRICT=1 npm --prefix app run build` fail while any language
  lacks a text. If you cannot translate into every language, say so in the pull request.
- Keep placeholders (`%1` to `%9`, `%n`), line breaks and technical names (OBD, ELM327, UDS, fault codes,
  units) exactly as they are.

## House rules

- **No long dashes.** The characters U+2014 and U+2013 do not appear in code, comments, texts, translations or
  documentation. Use a comma, a colon or a plain hyphen. Several checks fail on them.
- **Comments explain why**, not what the next line does: the timeout a real adapter needs, the byte order of a
  certain control unit, the source of a value.
- **Small, focused pull requests.** One fix or one feature per pull request, without unrelated reformatting.
  Large changes are easier when you open an issue first and describe the plan.
- **Plain words** in the interface: the texts talk to a car owner, not to an engineer. Say honestly what was
  tested and what was not.
- **No copied code from GPL projects** or from anything with an unclear licence. Caruspex is under the MIT
  licence ([LICENSE](LICENSE)). Projects such as EdiabasLib (GPL) may be read as protocol knowledge, the way
  `crates/caruspex-protocols/src/bmw_kline.rs` describes it, but no code, tables or data are taken over.
- **Only freely licensed data, with its source.** Every data file says where its rows come from (look at the
  comment lines at the top of `data/ecus/*.csv` or `data/pids/*.csv`). Data under another licence keeps its
  licence file next to it (`data/engines/LICENSE`, `data/obdb/LICENSE`, `data/opendbc/LICENSE`). Data from
  manufacturer software, paid databases or leaked documents is not accepted, and neither are tables taken
  from projects under the GPL, under a non-commercial licence (such as CC BY-NC) or without any licence.
  New data that ends up in the programs also needs a line in the data table of `tools/gen_notices.py`.
- **A permissive project only counts for what it wrote itself.** Name the file and commit. Lines a project
  quotes from a GPL source (OVMS comments that repeat CanZE field lines, OVMS's BMW i3 files derived from
  EdiabasLib) are not a source. A PID pack that takes anything from OBDb is CC BY-SA 4.0 and says
  `#@ licence: CC BY-SA 4.0`; `tools/gen_pids.py` checks this. VIN maker codes go into `vin.rs` only when
  NHTSA vPIC or Wikidata lists them, otherwise into `data/wmi/wmi.csv` (CC BY-SA, after Wikibooks). A
  control unit address needs a source in the note column of its row; a value nobody freely licensed
  confirms stays out.
- **New dependencies** must have a licence that allows shipping them in an MIT program (MIT, BSD, ISC,
  Apache-2.0, Zlib, MPL-2.0 and similar; no GPL, AGPL or non-commercial licences). After changing
  `Cargo.lock`, `app/package-lock.json` or the Android libraries run `python tools/gen_notices.py`, which
  rewrites `THIRD_PARTY_NOTICES.md` and fails on a licence it does not know.
- **Generated files are never edited by hand.** `crates/*/src/generated/`, `app/src/api/mock/data/` and
  `app/src/i18n/generated/` are written by the generators; change their input and run the generator.
- **Errors are `Result<T, String>`** with a message the user can read; `unsafe` only where the operating system
  needs it, inside the modules that already have it (see [crates/README.md](crates/README.md)).
- Keep the existing style: `cargo fmt`, and the formatting of the files around your change.

## Safety rules for anything that talks to a car

A diagnostics program can do damage. These rules are not up for discussion in a pull request:

- **Read only by default.** Connecting, scanning and reading never change anything in the car.
- **Anything that makes the car do something only behind a confirmation.** Clearing fault codes, starting an
  output test or a basic setting is started by the user, never by itself, and a new action of that kind asks
  first in plain words what will happen.
- **Never coding, writing adaptation values, flashing or memory writes**, and never anything about the
  immobiliser, keys or the mileage. Pull requests that add them are closed.
- **Nothing that helps to break the law.** No odometer changes, no switching off or bypassing emissions
  control (EGR, particulate filter, AdBlue/SCR, lambda or catalyst monitoring), no immobiliser or key bypass,
  no tachograph manipulation, also not "for testing" or behind a hidden switch. Such changes are illegal in
  many countries; issues and pull requests that ask for them are closed. Where a protocol module can send
  arbitrary requests, keep them behind an allow list like `request_allowed` in `bmw_kline.rs`.
- **Respect the timing.** Keep the timeouts and pauses that real control units need; `caruspex_link::timing`
  has the helpers.
- **Private data stays private.** No VIN, number plate, location or name leaves the computer without the
  user's action. The update check is the only network request the app makes by itself.

## Common changes, step by step

### Add or correct a vehicle

1. Edit the right file in `data/vehicles/` (one model generation per row; columns and rules are in the
   docstring of `tools/gen_vehicles.py`). A new brand goes into `data/vehicles/brands.csv` first.
2. Optional: the exact spot of the OBD socket in `data/obd_sockets.csv`, outer dimensions from the maker's data
   sheet in `data/dimensions_manual.csv` (with the document and its year), engine codes in
   `data/engines/engines.csv` (`tools/gen_engines.py`).
3. Run `python tools/gen_vehicles.py` and `python app/scripts/extract-catalog.py`, then the tests:
   `cargo test -p caruspex-vehicles` and `npm --prefix app test` (the X-ray test builds a model of every
   generation).
4. Look at it: `cargo run -p caruspex-app -- --page "xray-generation:Brand Model Generation"` or the
   **Vehicles** page in pro mode.

### Add an adapter or a transport

1. A new way to an ELM327 (a cable, a radio) implements `caruspex_link::Transport` in `crates/caruspex-link`.
   A K-line cable implements `ByteStream`. A network or raw CAN interface lives in `crates/caruspex-net` and ends
   in something that answers like an ELM327 (`CanElm`, `EnetElm`), so the rest of Caruspex works unchanged.
2. Test it against a fake of the device in the crate's `tests/` folder (look at `tests/common/`).
3. Connect it in the app: a `Job` variant in `app/src-tauri/src/worker.rs`, its handling in
   `app/src-tauri/src/worker/connect.rs`, a command in `app/src-tauri/src/commands.rs` registered in
   `lib.rs`, the command in `app/src/api/contract.ts` with an answer in `app/src/api/mock/`, and the choice on
   the Connect page (`app/src/pages/ConnectPage.tsx`).
4. Document it: a page in `docs/site/content/<language>/` for every language (the page names are in `PAGES` in
   `docs/site/build.py`), and an entry in the dropdown of `.github/ISSUE_TEMPLATE/test-report.yml`.

### Add a protocol

1. A module in `crates/caruspex-protocols/src/`. OBD-II and UDS borrow a connected `Elm327`, the K-line
   protocols own a `ByteStream`, KWP2000 over TP 2.0 owns a `MessageLink`.
2. Tests against a simulated control unit in `crates/caruspex-protocols/tests/` (the existing files show how),
   including the error paths: negative responses, timeouts, cut off answers. `tests/fuzz.rs` feeds random
   answers; a parser must never panic.
3. Name your sources in the module comment: the standard, public write-ups, recordings. Follow the safety
   rules above.

### Add a PID pack (manufacturer values)

1. A CSV in `data/pids/` in the Torque Pro format with three extra columns and `#@` lines for id, titles and
   category; the format is in the docstring of `tools/gen_pids.py`, the supported equations in
   `crates/caruspex-record/src/custom_pids.rs`.
2. Name at least one source for every value in the comment at the top, like `data/pids/hyundai-kona-niro-ev.csv`.
3. Run `python tools/gen_pids.py` and `cargo test -p caruspex-record`. Try your pack with
   `cargo run -p caruspex-cli -- custom --replay <recording.jsonl> --pack <id>` on a recording of the real car;
   `cargo run -p caruspex-cli -- custom --demo --pack caruspex-demo` shows how a pack runs against the demo car.

The packs in `data/obdb/` and `data/opendbc/` are converted from upstream projects by `tools/gen_obdb.py` and
`tools/gen_opendbc.py`; fix them upstream or in the converter, not in the CSV.

### Add or improve a translation

Follow [translations/TRANSLATING.md](translations/TRANSLATING.md). A new language touches more places (the
language list in `app/src/i18n/index.ts`, `tools/readme_languages.py`, `docs/site/content/`, a README); please
open an issue first.

### Report a test on a real car

Use the [test report form](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml). Say
which adapter and which car, tick what worked, and attach a shared recording if you can. Every report goes into
the "Tested with" list of the README.

## Pull requests

- Fork the repository, make a branch, commit, open the pull request against `main`. The template lists the
  checks; tick what you ran.
- Write in the description what changed and why, and how you tested it (demo car, recording, real car with
  which adapter).
- If the interface changed, a screenshot helps (`--shot`, see above).
- By opening a pull request you agree that your contribution is published under the MIT licence of the project.
- Everyone taking part follows the [code of conduct](CODE_OF_CONDUCT.md).

## Kurz auf Deutsch

Danke fürs Mitmachen. Am meisten hilft ein [Testbericht vom echten Auto](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml),
am besten mit einem Mitschnitt aus **Mitschnitt teilen**. Fehler und Wünsche gehen über die Formulare unter
[New issue](https://github.com/SenseiIssei/Caruspex/issues/new/choose), Sicherheitslücken nur privat nach
[SECURITY.md](SECURITY.md).

Bauen: `npm --prefix app ci`, `npm --prefix app run build`, dann `cargo run -p caruspex-app -- --demo`
(eingebautes Demo-Auto, kein Auto nötig). Live-Neuladen: `npm --prefix app run dev:app`. Vor dem Pull Request
die Prüfungen aus [What a pull request must pass](#what-a-pull-request-must-pass) laufen lassen.

Die Regeln in Kürze: Texte auf Deutsch und Englisch im Code, die anderen 16 Sprachen in JSON
(`tools/i18n_rust.py --write --area NAME`); keine langen Gedankenstriche; Kommentare erklären das Warum; kleine
Pull Requests; kein Code aus GPL-Projekten; nur frei lizenzierte Daten mit Quelle; nur lesen, alles, was das
Auto etwas tun lässt, nur nach Rückfrage, und nie Codierung, Anpassung, Flashen, Wegfahrsperre oder
Kilometerstand. Nichts, was beim Gesetzesbruch hilft: kein Verstellen des Kilometerstands, kein Abschalten
der Abgasreinigung, kein Umgehen der Wegfahrsperre, keine Eingriffe in den Tachographen.
