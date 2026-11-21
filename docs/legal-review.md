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
   **Generic expansion done (2026-11-20).** The generic table grew from 1381 to 3368 codes: 1989 new codes in
   `tools/dtc/extended.csv` (P0A00 to P0FFF 10 to 517, P2A00 to P2FFF 3 to 131, U0 86 to 219, U3 0 to 18,
   C0 7 to 143, B0 6 to 23, P0000 to P09FF 887 to 1404, P2000 to P29FF 316 to 847). A code is listed only
   when two independent sources agree on its meaning: the obdex data set (https://github.com/foerbsnavi/OBDex,
   data CC0-1.0, data of 2026-11-07, checked 2026-11-20) and a manufacturer document, either the GM OBD
   descriptions (https://gsi.ext.gm.com/gmspo/mode6/pdf/, 117 documents, published by GM for the emission
   rules) or a bulletin or investigation document in the public NHTSA archive (https://static.nhtsa.gov/odi/,
   2171 documents). From the manufacturer documents only the fact (code and meaning) is taken, no wording.
   The texts are Caruspex's own, composed from 810 phrases (`tools/dtc/phrases.py`, German and English, the
   other 16 languages in `translations/*/core_dtc38.json`). `tools/dtc/extended.py` holds the source
   statement, which `tools/gen_dtc.py` writes into the head of the generated tables. Not used: SAE J2012 and
   ISO 15031-6 themselves, GPL lists, unlicensed lists; Ford's OBD summaries refused the download. 6160 obdex
   codes without a second source, and the disputed C0040 and C0045 (removed) and C0035 (kept, owner to
   decide), are listed in `data/dtc/uncertain.md`; they show the group meaning of their range. Remaining
   risk: obdex names GM and NHTSA documents among its own sources, so for some codes the two sources are not
   fully independent; the C01xx to C04xx codes in the table follow GM's use.
   **Second pass (2026-11-20): two tiers.** The generic table now holds 9415 codes: 1379 written out, 2027
   "confirmed" (the 1989 above plus 38 found by a second, more careful check that reads abbreviations and
   synonyms) and 6009 "single source", which only obdex defines; the app (fault page and lexicon) and both
   reports (PDF/HTML of the app and of the command line) show "Meaning from one source" next to them, in all 18
   languages. Codes whose obdex description itself says the title is absent from the public J2012 lists or
   unverified (102) are left out, as are 13 whose composed text would read wrong; `data/dtc/uncertain.md` lists
   the tiers per block and every code left out. obdex is CC0, so the single tier carries no licence risk; the
   texts are still Caruspex's own phrases. Manufacturer codes: obdex holds none (it leaves P1, B1, C1 and U1 to
   the makers), and no other freely licensed list was found; the GM OBD documents name GM's own codes, but only
   in fragments of table cells, so no brand table was extended. Ford's OBD summaries refuse automated
   downloads and were not fetched again.
   **Manufacturer codes from NHTSA bulletins (2026-11-21).** 424 manufacturer codes in 12 families, in
   `tools/dtc/brand_codes.csv` (loader and source statement in `tools/dtc/brand_ext.py`, written into the head
   of `dtc_brands.rs`). Source: the manufacturer communications (technical service bulletins) that NHTSA
   publishes as a data set, https://static.nhtsa.gov/odi/ffdd/tsbs/ (TSBS_RECEIVED_1995-1999 to
   TSBS_RECEIVED_2020-2024, retrieved 2026-11-21; data.gov labels the data set as public domain, US government
   data). From the summary of a bulletin only the fact is taken: which code it names and which part and fault
   the code stands for. The texts are composed from Caruspex's own phrases (293 new parts and 6 fault types,
   German and English in `tools/dtc/phrases.py`, the other 16 languages in `translations/*/core_dtcmfr.json`).
   Every row names up to four NHTSA ids of its bulletins. Tier "confirmed" (274 codes): at least two bulletins
   with their own number, issued in different months, give the same meaning; the same text filed once per
   brand or model line counts once. Tier "single" (150): one bulletin; the app and the reports say "Meaning
   from one source". Per family: Chrysler/Dodge/Jeep/Ram 239 (a new family), Fiat/Alfa Romeo 47, Nissan 37,
   Honda 30, Ford 29, Hyundai/Kia 10, Toyota 10, Subaru 8, VAG 7, Mazda 3, Mercedes 2, Mitsubishi 2.
   Remaining risks: the bulletins are written by the makers for the US market, so a code may mean something
   else on a car for another market (said in the source statement); the bulletin summaries themselves are
   the makers' text, of which nothing is copied. Still open, because no acceptable source exists: BMW and
   Mini (their bulletins name BMW's own fault memory numbers such as 20A508, not P, B, C or U codes), Volvo
   cars (the Volvo entries with such codes are Volvo trucks; the car bulletins use Volvo's module codes such
   as ECM-130A), Opel (only the Ampera, whose GM Voltec codes do not hold for other Opel models), Renault and
   PSA (not sold in the USA, no bulletins). Searched without a usable result: obdex (CC0, generic codes only),
   OBDb (no fault codes in its signal sets), Wikidata (no property for fault codes), the EU type approval
   rules (Regulation (EU) 2017/1151, Annex XI, refers to ISO 15031-6 and SAE J2012 and lists no manufacturer
   codes), the full bulletin PDFs (the data set has no links to them and the API addresses tried did not answer, so
   only the summaries were used).
   Not used: GPL lists, unlicensed lists, forums, paid databases.
   **OBD socket spots (2026-11-21).** The 209 car rows of `data/obd_sockets.csv` from before stay an own
   compilation; no free source was found to check them (Wikidata has no property for it, OBDb no location
   data). Eight Ford and Lincoln generations were added with a source: Ford's special service messages SSM
   46022, 46118, 47070, 47489 and 47635 in the same NHTSA data set say the data link connector sits on the
   driver side under the steering column. The file names the messages above these rows. Mondeo Mk5 and
   Transit Mk8 keep their old spot (left of the column) although the messages say under the column for the
   US Fusion and Transit; to be checked at a real car.
5. **Default-on update check** (low). Resolved: asks first since 0.18. On every installation Caruspex asks once
   "Look for updates?"; only after a yes do official builds ask GitHub (a US service) for `latest.json`, at start and
   every six hours, which shows the IP address. No request goes out without a yes. It is disclosed on the privacy
   page and can be changed in the settings.
6. **Documentation website** (low). It runs on GitHub Pages. Whether it needs a provider identification
   (Impressum, section 5 DDG) and a privacy note for the hosting depends on whether it counts as a business-like
   service; not decided here.
   **Owner's decision:** no imprint. The site stays purely private and non-commercial (no ads, no paid offers).
7. **Personal data in the git history** (low, owner's own data). 9 early commits carry a second personal e-mail
   address as author and committer; 37 merge commits made in the GitHub web interface carry a first name as author
   name. Nothing else personal was found. Rewriting public history breaks the existing fork and clones; leaving it
   is an option.
   **Owner's decision:** remove the extra e-mail address and the first name from the history (being done
   separately).
8. **Release secrets** (low). The release workflow passes the literal `none` as the updater key password, so the
   key is effectively unprotected by a password; it only exists as a repository secret. No secret is in the
   repository or its history.
9. **Not reviewed**: the licence terms of the WebView2 bootstrapper that the installer carries (Microsoft documents it
   for distribution with apps); a trademark search for the name "Caruspex"; the built packages themselves
   (`tools/package.sh` was not run, so the files in the packages were checked in the script only).
10. **Workflow note**: dependency update pull requests (Dependabot) will fail the new notices check until
    `python tools/gen_notices.py` is run on their branch.
11. **Own signals for Tesla Model S and X, BYD, Xpeng, NIO and VAG diesels (2026-11-21).** Searched: every
    repository of OBDb (github.com/OBDb, 897 on 2026-11-21, CC BY-SA 4.0), opendbc (commaai/opendbc, MIT, at the
    pinned commit c81a2155976b and on master 79ddcc7bb72d), the vehicle modules of OVMS 3 (MIT, commit
    7470869a2db5) and the car modules of evDash (MIT). Taken:
    - **BYD Atto 3**: 68 values (battery state of charge, pack and cell voltages, current, temperatures, wheel
      speeds, 12 V battery). OBDb's own Atto 3 set is empty; the values come from OBDb's merged BYD set (commit
      c154169b7527, made from the Sealion 6), and only for the commands and signals that the Atto 3's recorded
      answers in its OBDb repository (commit 2019ba775df9) confirm with the same value. Such a make fallback is
      used only for models listed in `FALLBACK` in `tools/gen_obdb.py` after a check by hand (the merged sets of
      large makes read one identifier in different ways; for the Skoda Octavia it would have shown a high
      voltage battery). For the Atto 3 the recorded values agree with each other and with the car, and OVMS's
      `vehicle_byd_atto3.cpp` (MIT, (C) 2023 Alan Harper, added to `data/pids/LICENSE-OVMS`) reads the same
      addresses with the same layout; only the sign of its current was taken from there, as a fact. One signal
      without a matching recorded value was left out. Listening (opendbc `byd_atto3.dbc`, MIT, pinned commit): 7
      values, new pack `opendbc-byd-atto3`.
    - **BYD Seal U**: OBDb's Sealion 6 set (174 values, already imported) now reaches the Seal U through
      `data/obdb_aliases.csv`; OBDb's own `generations.yaml` of the Sealion 6 says it is sold in Europe as the
      Seal U.
    - **Tesla Model X with HW4 (2024)**: 6 listening values from opendbc `tesla_model3_party.dbc` (its
      `values.py` puts this car on that bus), new pack `opendbc-tesla-model-x-hw4`. **Model S and X up to 2020**:
      one more value (outside temperature, `tesla_can.dbc` BODY_R1) in `opendbc-tesla-s-x`, now 8.
    - **VAG diesels**: no new source was needed. The OBDb packs already hold, from each model's own set, soot
      mass (27 VAG models), regeneration values (45) and the oil level (62), 1672 values on 63 VAG models in
      all; recorded answers of the Audi Q7, VW Transporter and VW Passat are now test fixtures.
    - OBDb's attribution (`data/obdb/ATTRIBUTION.md`) now names the commit of every repository read, as
      `tools/gen_notices.py` already claimed.

    Left open, and why:
    - **Tesla Model S and X battery values**: OBDb's Tesla-Model-S and Tesla-Model-X repositories and its Tesla
      make set are empty, opendbc has no battery messages for these cars. OVMS's `vehicle_teslamodels` (MIT) reads
      voltage (0x102) and state of charge (0x302) on the powertrain bus, but its documentation calls that support
      experimental, and no second source confirms it. Not taken; it can come with a recording of a real car.
    - **Xpeng and NIO**: no repository in OBDb, no DBC or platform in opendbc, no module in OVMS or evDash.
      Nothing freely licensed was found; forum posts and scan tool lists are not acceptable.
    - **Other BYD models (Dolphin, Seal, Han, Dolphin Surf)**: OBDb has only an empty BYD-Dolphin-Mini without
      recorded answers; nothing else.
    - **VAG diesel: measured soot mass, distance and time since the last regeneration, regeneration duration**:
      OBDb has them for many models, but only behind the extended diagnostic session (10 03, `din` in OBDb):
      340 such signals on 50 VAG models. Caruspex does not switch sessions for custom values, so the converter
      leaves them out ("session"). Whether the live values may change the control unit's session is a decision
      for the owner, not a data question.
    - The make fallback for the 16 other OBDb models with empty sets and recorded answers (among them Skoda
      Octavia and Enyaq, Mitsubishi Lancer, Mazda CX-7, Volvo V70) stays off until each is checked by hand.
