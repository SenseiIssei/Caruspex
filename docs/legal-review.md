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

1. **Hand-written PID packs: resolved (2026-11-20).** Every value now rests on OBDb (CC BY-SA 4.0), the Open
   Vehicle Monitoring System or evDash (both MIT), named per value with the file and commit in each pack.
   CanZE, EVNotify and JejuSoul are no longer sources; `tools/gen_pids.py` refuses source lines that name them
   (and DDT4all, pyren, python-OBD). The four packs that adapt OBDb (`hyundai-ioniq-28`, `hyundai-kona-niro-ev`,
   `nissan-leaf-ze0-aze0`, `renault-zoe-ph1`) carry `#@ licence: CC BY-SA 4.0`, with `data/pids/ATTRIBUTION.md`,
   the licence text and the MIT notices of OVMS and evDash next to them; the generator checks that an OBDb
   source has that line. Kept: all 29 Ioniq and 33 Kona/e-Niro values, all 7 Leaf values, 22 of 24 Zoe values.
   Dropped (no permissive source): Zoe state of health of the EVC (22 3206) and average battery temperature
   (21 04 byte BV). The Zoe cell voltage maximum and minimum are now taken as MAX and MIN of the two fields,
   because evDash and the older definition disagree on which field is which. Remaining risk: OVMS and evDash
   name CanZE as where their Renault knowledge came from; only their own code (not the CanZE field lines OVMS
   quotes in comments or in `docs/PIDs_Renault_Zoe.txt`) is used. The OVMS BMW i3 address files are
   GPL-3.0 (derived from EdiabasLib) and were not used.
2. **Control unit address lists: resolved (2026-11-20).** Every row of `data/ecus/*.csv` names its sources in
   the note column: the ISO 15765-4 convention for 7E0/7E8 and 7E1/7E9, opendbc, OVMS and evDash (MIT), and
   OBDb (CC BY-SA 4.0, used only to confirm single addresses, which are facts). The GPL lists (bmw-f, DDT4all,
   CanZE, arduino-psa-diag) and the scan logs of commercial tools are no longer sources. Kept 72 of 105 rows.
   Dropped 33 rows that no permissive or public source confirms: BMW 19 (01 ACSM, 10 gateway, 67 ZBE, 61 ATM,
   37 AMP, 3F ASD, 72 REM, 17 EKPM, 1C ICM, 76 VDC, 0F GHAS, 5D KAFAS, 06 TRSVC, 41 to 44 lights, 6D and 6E
   seats), Ford 8 (737 airbag, 716 gateway, 724 SCCM, 733 HVAC, 736 PAM, 740 and 741 doors, 727 audio), Hyundai
   7D2 airbag, Nissan 745 BCM, PSA 6AD brakes and 772 airbag, VAG 70C steering column and 70A parking aid. The
   BMW address sweep and the generic 700 to 7F7 sweep still find these units, by address instead of by name.
   No own observation was available, so no row is marked as observed.
3. **WMI codes: resolved (2026-11-20).** The table in `vin.rs` now holds only codes that NHTSA vPIC (public
   domain) or Wikidata (P6793, CC0) confirm: 285 checked codes plus 66 three character codes from vPIC that
   replace the two character ranges 1G, 2G, 3G, KL, 1F, 2F, 3F, 1L, 5L and JN (vPIC shows other makers inside
   those ranges); JT stays a range because every JT code in vPIC is Toyota. 124 codes that only the Wikibooks
   list has (mostly makers that do not sell in the USA, such as TMB, VSS, VF7, VR3, XTA, UU1) moved to
   `data/wmi/wmi.csv` under CC BY-SA 4.0 with `LICENSE` and `ATTRIBUTION.md` (revision 4682957); `tools/gen_wmi.py`
   writes them into a generated file that `vin.rs` includes. Dropped: TMC (vPIC assigns it to another maker),
   JHG and MRX (in no source). The Wikibooks attribution stays, now pointing at `data/wmi`.
4. **Manufacturer fault code texts: resolved as own compilation (2026-11-20).** No freely licensed data set of
   manufacturer codes exists (OBDb and Wikidata hold none; the open source scan tools with code lists are GPL).
   All 298 codes are kept as Caruspex's own compilation of facts (code number and meaning) in its own words;
   `tools/dtc/brands.py` says so in `SOURCES`, which `tools/gen_dtc.py` writes into the head of
   `dtc_brands.rs`. The generic tables (`tools/dtc/generic.py`) state the SAE J2012 / ISO 15031-6 numbering
   and that nothing is taken from GPL scan tools. Owner to confirm: that the brand texts were indeed written
   without copying a list (this review could not check the history before 2026-11-09), and whether the English
   generic texts, which use the standard's short fault names, are acceptable as they are.
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
