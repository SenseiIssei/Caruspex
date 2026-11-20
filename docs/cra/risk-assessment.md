# Cybersecurity risk assessment

Status: 21 November 2026, Caruspex 0.17. Structure after Article 13(2) and (3) of the Cyber Resilience Act
(Regulation (EU) 2024/2847): intended purpose, reasonably foreseeable use and misuse, operational environment,
assets, threats, measures, open gaps. Caruspex is currently outside the scope of the CRA ([scope.md](scope.md));
this document is kept anyway and updated with every release that changes an interface, the updater or the
release chain.

## Product and intended purpose

- **What**: a diagnostics program for cars, motorcycles and trucks. It reads fault codes, live values, readiness
  and control unit data through a diagnostic adapter, and explains them in plain words.
- **Parts**: the desktop app `caruspex` (Windows x64, Linux x86_64, macOS arm64), the command-line tool
  `caruspex_cli`, the 32-bit helper `caruspex-j2534-bridge` on Windows, an Android build (no release yet).
- **Users**: car owners and hobby mechanics on their own computer or phone; workshops in pro mode.
- **Operating environment**: an ordinary user account on a desktop system or a phone, without administrator
  rights at run time. No server, no account, no cloud. The car is connected through an adapter the user owns.
- **Expected time in use**: years; cars and adapters are kept a long time.
- **Reasonably foreseeable misuse**: clearing fault codes to hide defects before a sale, running output tests
  or basic settings while the car is moving, opening files from strangers (recordings, PID packs, 3D models),
  installing builds from unofficial sources.

## Assets

1. The integrity of the user's computer (the program and its updater run with the user's rights).
2. The safety of the car: Caruspex can send requests that make a control unit do something (clearing codes,
   output tests, basic settings).
3. Personal data: VINs, odometer readings and the garage history, recordings, crash reports.
4. The release chain: the updater signing key, the Android signing key, the GitHub repository and its releases.
5. Availability of the diagnosis (a crash in the middle of a test should not leave the car in a test state).

## Threats, measures, gaps

| Threat | Existing measures | Gaps and next steps |
|---|---|---|
| **Malicious update.** Someone replaces a package or latest.json, or intercepts the download. | Packages are installed only when the minisign signature matches the public key compiled into the app (`app/src-tauri/src/update.rs`, `verify_file`); the private key is only a repository secret used by the release job. Downloads only over HTTPS and only from `https://github.com/SenseiIssei/Caruspex/releases/download/` (`allowed_download`); a file with a wrong signature is deleted. `tools/release_assets.py` checks every signature against the same key before publishing. Only official builds look for updates (`official_build`). | latest.json itself is not signed and the signature does not bind the version: with control of the GitHub releases (but not of the key) an attacker could offer an older, validly signed package as an "update" (rollback). Fix: check that the `file:` name in the signed trusted comment equals the downloaded file name and contains the version of the manifest; enforce the same in `tools/release_assets.py`. |
| **Compromised release chain.** A pull request, a third-party action or a dependency reaches the secrets or the published files. | Workflows default to `contents: read`; pull requests from forks run without secrets; signing and publishing only run for version tags in the main repository (`.github/workflows/build.yml`); checkout without persisted credentials. SHA256SUMS and GitHub build provenance (Sigstore) for every release file, SBOMs per platform. Dependabot for actions, Cargo, npm, pip and Gradle; RustSec and npm audit weekly (`.github/workflows/audit.yml`). Since 0.18 every action is pinned by commit SHA (Dependabot raises SHA and version comment together) and the release job installs `cryptography` and its dependencies in exact versions with hashes (`tools/requirements-release.txt`, `--require-hashes --only-binary :all:`). Dependency update pull requests get the regenerated `THIRD_PARTY_NOTICES.md` as an artifact; nothing is pushed by a workflow (CONTRIBUTING.md, "Dependency updates"). | What the packaging downloads at build time is not pinned by this repository: the AppImage tools the tauri bundler fetches and the WebView2 bootstrapper in `tools/package.sh` (from Microsoft over HTTPS). The system packages of the runners come from the signed Ubuntu archive in whatever version it has. Branch protection, required reviews and tag protection are repository settings that the code cannot enforce. The updater key has no documented rotation or revocation procedure. |
| **Malicious file opened by the user** (replay recording, custom PID CSV, PID pack, 3D model, garage file). | No file format can contain code that is run. The parsers are covered by property tests with random input (`crates/caruspex-record/tests/fuzz.rs`, `crates/caruspex-protocols/tests/fuzz.rs`, `crates/caruspex-link/tests/fuzz.rs`, `crates/caruspex-net/tests/fuzz.rs`) and must not panic. Custom PID equations are evaluated by an own small interpreter (`crates/caruspex-record/src/custom_pids.rs`), not by a script engine. 3D models are limited to glTF and 512 MB (`app/src-tauri/src/commands.rs`). The interface has a strict content security policy without remote scripts (`app/src-tauri/tauri.conf.json`). | Recordings and CSV files are read completely into memory without a size limit: a huge file can exhaust memory (only the user's own session, no code execution). The garage and settings JSON are not covered by the property tests. |
| **The adapter and the car link.** A manipulated adapter or a control unit sends malformed answers; a request reaches the car that the user did not want. | Answers are parsed defensively and fuzzed (see above). Read-only by default: anything that makes the car do something needs a confirmation, and coding, adaptation writes, flashing, immobiliser and odometer functions are refused by design (`CONTRIBUTING.md`, safety rules; allow lists such as `request_allowed` in `crates/caruspex-protocols/src/bmw_kline.rs`). Timeouts from `caruspex_link::timing`. | The confirmation dialogs cannot know whether the car is moving. The user information warns against tests while driving ([user-information.md](user-information.md)). |
| **Bluetooth and Wi-Fi adapters.** Someone nearby connects to the adapter or listens in. | Caruspex only connects to the adapter the user chose; the traffic is diagnostic data, no credentials. | Cheap ELM327 adapters often have no pairing PIN or an open Wi-Fi network; Caruspex cannot fix that. The user information says to unplug the adapter after use. |
| **Network interfaces (ENET, DoIP, Wi-Fi adapters).** Traffic on a shared network. | Only local connections the user starts; ENET discovery is a UDP broadcast on the local network (`crates/caruspex-net/src/enet.rs`). Nothing is routed to the internet. | The protocols themselves (HSFZ, DoIP, ELM327 over TCP) have no authentication or encryption; they are meant for a direct cable or a private network. |
| **J2534 drivers.** A PassThru driver is a third-party DLL loaded into Caruspex or into the 32-bit bridge. | The Connect page offers the drivers their installers registered under `HKLM\SOFTWARE\PassThruSupport.04.04` (`crates/caruspex-net/src/j2534.rs`); any other DLL only when the user names it (`--can j2534:<driver.dll>`). The bridge pipe accepts one local client only (`FILE_FLAG_FIRST_PIPE_INSTANCE`, `PIPE_REJECT_REMOTE_CLIENTS` in `crates/caruspex-j2534/src/bridge.rs`); the unsafe code is limited to that module and runs under AddressSanitizer in the CI. | A driver DLL runs with the user's rights; Caruspex cannot verify it. The pipe uses the default access rights of Windows. |
| **Personal data leaves the computer.** | No telemetry, no account; the only automatic request is the update check, which sends nothing but the program version in the user agent and can be switched off. Crash reports stay local and are only posted by the user; VINs, traffic bytes and the home folder are cut out (`app/src-tauri/src/crash.rs`). "Share recording" removes the last VIN digits, addresses and the time of day (`crates/caruspex-record/src/share.rs`). | Data at rest (garage, recordings) is not encrypted; it relies on the protection of the user account, like other desktop programs. |
| **Android app.** | Not released. Permissions: Bluetooth, network, location only up to Android 11 for Bluetooth scanning (`app/src-tauri/gen/android/app/src/main/AndroidManifest.xml`). The APK is only published when signed with the release key from repository secrets. | No in-app updater on Android; updates would come from the store or a new APK. The Gradle dependencies are not in the SBOM. |
| **Known vulnerabilities in dependencies.** | `tools/audit.py` (RustSec and npm audit) on dependency changes and weekly; Dependabot; SBOMs list every crate and npm package with its version and hash. | RustSec warnings, no vulnerabilities (checked 20 November 2026). `chrono` (RUSTSEC-2026-0543, unmaintained) is replaced by `jiff` since 0.18 and no longer compiled; it stays in Cargo.lock only as an unused optional dependency of `serde_with`. The others come with tauri's Linux stack and cannot be replaced yet: `glib` 0.18 is unsound (RUSTSEC-2024-0429, fixed in 0.20, but gtk 0.18 of tauri, wry and tao needs 0.18; the affected `VariantStrIter` is not called by Caruspex or any of its Linux dependencies), `proc-macro-error` (RUSTSEC-2024-0370) is a build-time macro of glib 0.18, and `zvariant`, `zvariant_derive`, `zbus_names` (RUSTSEC-2026-0370 to 0372) come with zbus 5 of tauri-plugin-opener and tauri-plugin-single-instance (D-Bus on the user's session bus; their 3.0 alphas still use zbus 5). Each has an entry with its reason and an expiry of 20 May 2027 in `tools/audit_exceptions.toml`, so the decision is checked again. |

## Requirements of Annex I part I point 2 that do not apply

- **(d) access control, authentication**: Caruspex has no users, roles or remote access; it runs inside the user's
  account and uses the protection of the operating system.
- **(e) encryption at rest**: the stored data is the user's own diagnostic history on their own device; encryption
  is left to the operating system (disk encryption). The only data in transit over the internet (update check
  and download) uses HTTPS.
- **(l) recording of internal activity**: there is no security-relevant internal activity beyond the update; the
  optional log file records the adapter traffic when the user switches it on.

The full mapping is in [requirements.md](requirements.md).
