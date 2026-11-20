# Legal review

Date: 2026-11-21. Scope: the public repository, its full git history and the published programs (Windows, Linux,
macOS, Android). This is a technical review against the licence texts and sources named below, not legal advice.

## What was checked

| Area | How | Result |
|---|---|---|
| Rust dependencies | `cargo metadata` for Cargo.lock; 495 crates reach the shipped programs (caruspex-app, caruspex-cli, caruspex-j2534) as normal dependencies | All permissive. 8 under MPL-2.0 (serialport, cssparser, cssparser-macros, selectors, dtoa-short, option-ext): file based copyleft, used unchanged, source on crates.io. Choices with GPL/LGPL (unescaper `MIT OR GPL-3.0-only`, r-efi `... OR LGPL-2.1-or-later`) are used under MIT. No crate without a licence field. No GPL, AGPL, SSPL or non-commercial licence. |
| npm packages | `app/package-lock.json` and `app/node_modules/*/package.json` | Shipped: @tauri-apps/api, react, react-dom, scheduler, three (MIT or Apache-2.0), plus helper code of vite and rolldown (MIT). Dev tools include MPL-2.0 packages, not shipped. |
| Android libraries | `app/src-tauri/gen/android/app/build.gradle.kts`, POM files in the Gradle cache | AndroidX, Material Components, Kotlin and kotlinx: Apache-2.0. usb-serial-for-android: MIT. |
| Native libraries | `-sys` crates | Linked against the system's GTK, WebKitGTK, D-Bus, udev (Linux), WebView2 (Windows), WebKit (macOS); none of them is shipped in the packages. The Windows installer carries Microsoft's WebView2 bootstrapper. |
| Data in `data/` | Header comments, `tools/gen_*.py`, upstream licences (checked with the GitHub API on 2026-11-20) | See the findings below. OBDb and engine codes (CC BY-SA 4.0) have their licence files; opendbc and model3dbc (MIT) too; Wikidata is CC0. |
| Fonts, icons, images | `resources/`, `app/public`, `docs/*.png`, git history of image files | Fonts: JetBrains Mono, Inter, Plus Jakarta Sans under OFL 1.1, licence files present and shipped next to the fonts in the app. Icons: own line drawings. Images: screenshots of the app and renders of its own procedural 3D view. No car photos, brand logos, map tiles or third party 3D models or textures. |
| Code provenance | Search of the tree and the full history for "ported from", "taken from", "copied", "based on", GPL project names | No copied code found. EdiabasLib (GPL) is named as protocol knowledge in `bmw_kline.rs` and `enet.rs`, with no tables taken over; "ported from" refers to the project's own earlier C++ version. |
| Trademarks | README, docs, app, packaging | Makes, adapters and programs are named only to say what works with what. No logos. No wording that suggests endorsement. |
| Safety | LICENSE, README, docs, app, CONTRIBUTING | MIT warranty disclaimer present. Writing actions sit behind a confirmation (CONTRIBUTING rules). Nothing found that encourages odometer, emissions, immobiliser or tachograph tampering. |
| Privacy | `update.rs`, `crash.rs`, `commands.rs`, AndroidManifest, CSP in `tauri.conf.json`, docs privacy page | No telemetry. Network: the update check and update download (GitHub, official builds only, can be switched off), adapters on the local network, and browser links the user clicks. Crash reports are local and opt-in. The interface loads nothing from outside (CSP). |
| Secrets and personal data | `git log -p --all` (428 commits) searched for keys, tokens, passwords, private keys, e-mail addresses, IP addresses, host names, home paths, VIN-like strings | No secret found. VINs are test or demo values. Personal data only in commit metadata, see below. |
| Packaging | `packaging/` templates | Licence fields were MIT only; descriptions claimed "only reads, sends nothing". Fixed. |

## What was fixed

- `THIRD_PARTY_NOTICES.md` (new, 478 KiB): every shipped Rust crate, npm package and Android library with licence and
  source, the full licence texts (deduplicated), the data and fonts with their licences. Written by
  `tools/gen_notices.py` (standard library only, reads `cargo metadata`, the lock file and `node_modules`); the CI job
  Checks fails when it is out of date or a dependency has a licence outside the allowed list.
- The notices and LICENSE ship with every package: next to LICENSE in the Windows zip, the installer and the CLI
  archives (`tools/package.sh`), and inside the app interface (`app/public/legal`), where Settings shows them
  with the font licences under "Show the licences of the components". This covers the deb, AppImage, dmg and APK too.
- README in all 18 languages: one managed block (`tools/readme_blocks.json`, `tools/readme_languages.py`) with
  safety and liability, a trademark notice, and licence and data credits (OBDb, opendbc, model3dbc, Wikipedia,
  Wikibooks, Wikidata, fonts). The same text is a new page of the documentation website ("legal", 18 languages).
- App: Settings has the data credits, a trademark note, the licence viewer and a "Safety and liability" group; the
  setup assistant, which opens on the first start, says never to operate the app while driving.
- CONTRIBUTING: an explicit rule against odometer, emissions control, immobiliser and tachograph tampering; data from
  GPL, non-commercial or unlicensed projects is not accepted; new dependencies need an allowed licence and a run of
  `tools/gen_notices.py`.
- Privacy page (18 languages): the KKL test reports and where they are stored, that only official builds check for
  updates, that GitHub sees the IP address and version on update requests, the Android location permission (only
  up to Android 11, only for the Bluetooth LE search).
- Packaging: `project_license` and the snap licence are now `MIT AND CC-BY-SA-4.0`, winget names both; the
  descriptions say what the app writes to the car instead of "only reads, sends nothing"; a short
  non-affiliation and safety note was added.
- Attribution notes in `crates/caruspex-vehicles/src/vin.rs` (Wikibooks WMI list, CC BY-SA 4.0) and
  `crates/caruspex-record/tests/custom_pids.rs` (OBDb test cases, CC BY-SA 4.0).
- A first name in PLAN.md and in two test strings was replaced.

## Open points for the owner, by risk

1. **Hand-written PID packs built from sources with incompatible or missing licences** (medium).
   `data/pids/renault-zoe-ph1.csv` takes its field definitions from CanZE (`_Fields.csv`, GPL-3.0).
   `data/pids/hyundai-ioniq-28.csv` names EVNotify, whose licence is CC BY-NC 4.0 (non-commercial), and the
   JejuSoul PID repository, which has no licence at all; `hyundai-kona-niro-ev.csv` names JejuSoul too. The other
   sources (OVMS: MIT for most of its code; OBDb: CC BY-SA 4.0) are compatible, OBDb with share-alike. The values are
   byte positions and scale factors; whether such facts are protected is a legal question this review does not
   answer. It also contradicts CONTRIBUTING and the note in PLAN.md that CanZE was to be used as protocol
   knowledge only. Options: re-derive each value only from OVMS and OBDb (and mark packs that rely on OBDb as
   CC BY-SA 4.0 like `data/obdb`), dropping values that only CanZE, EVNotify or JejuSoul back; or ask those
   projects for permission.
2. **Control unit address lists citing GPL and proprietary tools** (low to medium). `data/ecus/bmw.csv` cites a GPL-3.0
   community list, `renault.csv` the DDT4all address tables and CanZE (both GPL-3.0), `psa.csv` arduino-psa-diag
   (GPL-3.0); Ford, Mazda, VAG, Toyota and BMW rows cite scan logs of FORScan, VCDS, OBDeleven, Techstream, E-Sys
   and ISTA. About 105 rows of single addresses. Decide whether to keep them as facts, or reword the sources as own
   observations after cross-checking against permissive sources.
3. **WMI codes partly after Wikibooks** (low to medium). Part of the table in `vin.rs` follows the Wikibooks WMI list
   (CC BY-SA 4.0). Attribution is now given. Share-alike would apply to adapted material; decide whether to move
   those entries into a CC BY-SA data file like `data/engines`, or to verify them against public-domain sources
   (NHTSA vPIC) and drop the dependency.
4. **Manufacturer fault code texts without named sources** (low to medium). `tools/dtc/brands.py` (298 codes in 17
   families) names no sources, unlike `data/`. Generic codes follow SAE J2012 / ISO 15031-6 numbering in the
   project's own words. Add the sources, or confirm they are an own compilation.
5. **Default-on update check** (low). On start, official builds ask GitHub (a US service) for `latest.json`, which
   shows the IP address. It is disclosed on the privacy page and can be switched off. Decide whether to keep it on
   by default or ask on the first start.
6. **Documentation website** (low). It runs on GitHub Pages. Whether it needs a provider identification
   (Impressum, section 5 DDG) and a privacy note for the hosting depends on whether it counts as a business-like
   service; not decided here.
7. **Personal data in the git history** (low, owner's own data). 9 early commits carry a second personal e-mail
   address as author and committer; 37 merge commits made in the GitHub web interface carry a first name as author
   name. Nothing else personal was found. Rewriting public history breaks the existing fork and clones; leaving it
   is an option.
8. **Release secrets** (low). The release workflow passes the literal `none` as the updater key password, so the
   key is effectively unprotected by a password; it only exists as a repository secret. No secret is in the
   repository or its history.
9. **Not reviewed**: the licence terms of the WebView2 bootstrapper that the installer carries (Microsoft documents it
   for distribution with apps); a trademark search for the name "Caruspex"; the built packages themselves
   (`tools/package.sh` was not run, so the files in the packages were checked in the script only).
10. **Workflow note**: dependency update pull requests (Dependabot) will fail the new notices check until
    `python tools/gen_notices.py` is run on their branch.
