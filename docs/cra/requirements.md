# Essential requirements: how Caruspex meets them

Status: 21 November 2026, Caruspex 0.17. Mapping of Annex I of the Cyber Resilience Act (Regulation (EU)
2024/2847) to the code and the processes of Caruspex. Caruspex is currently outside the scope of the CRA
([scope.md](scope.md)); the table shows what is in place and what a manufacturer would still have to add.
Legend: **met**, **partly**, **open**, **n/a** (not applicable, with the reason in
[risk-assessment.md](risk-assessment.md)).

## Annex I part I: properties of the product

| Point | Requirement (short) | Status | How, where |
|---|---|---|---|
| 1 | Appropriate level of cybersecurity based on the risks | partly | Risk assessment in [risk-assessment.md](risk-assessment.md); its gaps are listed there. |
| 2(a) | No known exploitable vulnerabilities when made available | met | `tools/audit.py` (RustSec, npm audit) in `.github/workflows/audit.yml` on dependency changes and weekly; a release is cut from a green main. Accepted findings need a reason and an expiry (`tools/audit_exceptions.toml`). |
| 2(b) | Secure by default configuration, reset to the original state | met | Read only by default, every action on the car behind a confirmation (`CONTRIBUTING.md`, safety rules); no network service is opened; update check on, signature check always. Reset: delete the settings file (the button **Open storage location** in the settings shows it). |
| 2(c) | Security updates, automatic by default with opt-out, notification, postponing | partly | Signed updater (`app/src-tauri/src/update.rs`): the check at start is on by default and shows the new version; installing is one click, "Install updates automatically" is **off** by default (`app/src-tauri/src/settings.rs`); both can be switched off; the user can postpone by not installing. A manufacturer would have to switch automatic installation of security updates on by default. No updater on Android and for .deb installs (they get the release page). |
| 2(d) | Protection from unauthorised access | n/a | No users, no remote access, no listening service; runs in the user's account. |
| 2(e) | Confidentiality of stored and transmitted data | partly | Internet traffic (update) only over HTTPS; personal data minimised and removed from shared files (`crates/caruspex-record/src/share.rs`, `app/src-tauri/src/crash.rs`). Data at rest relies on the operating system. |
| 2(f) | Integrity of data, commands, programs, configuration; report corruption | partly | Updates checked (signature, HTTPS, fixed download prefix). Settings and garage are written atomically (`write_atomically` in `app/src-tauri/src/settings.rs`); a garage file that cannot be read is kept as `garage.json.damaged` before the next save could overwrite it (`app/src-tauri/src/garage.rs`), but the user is not told yet, and a damaged settings file silently falls back to the defaults. The update manifest itself is not signed (rollback gap, see risk assessment). |
| 2(g) | Data minimisation | met | No telemetry; the update request carries only the version in the user agent; recordings only when started by the user ([privacy page](../site/content/en/privacy.md)). |
| 2(h) | Availability of essential functions, resilience | met | Parsers must not panic (property tests in `crates/*/tests/fuzz.rs`); one worker thread owns the connection and runs the jobs one after another, so a stop request always gets through (`docs/ARCHITECTURE.md`); crash reports on panic (`app/src-tauri/src/crash.rs`). |
| 2(i) | Minimise impact on other devices and networks | met | Timeouts and pauses that real control units need (`caruspex_link::timing`); no bus flooding; the ENET discovery is one broadcast. |
| 2(j) | Limit attack surfaces | met | No open ports, no remote code, strict content security policy, minimal Tauri capabilities (`app/src-tauri/capabilities/`), the bridge pipe takes one local client. |
| 2(k) | Exploitation mitigation | met | Rust (memory safe), `unsafe` only in the modules that need the operating system and tested under AddressSanitizer and Rust's undefined behaviour checks (`sanitizers` job in `.github/workflows/build.yml`); release builds with the platform defaults (ASLR, DEP/NX). |
| 2(l) | Recording of security-relevant activity, with opt-out | n/a | No security-relevant internal activity apart from updates, which are reported in the interface. Optional log file of the adapter traffic. |
| 2(m) | Secure removal of all data and settings, secure transfer | partly | All data is in the settings folder and the Caruspex data folder (privacy page); uninstalling and deleting that folder removes everything ([user-information.md](user-information.md)). There is no "delete all my data" button. Garage export and import are plain files. |

## Annex I part II: vulnerability handling

| Point | Requirement (short) | Status | How, where |
|---|---|---|---|
| 1 | Identify components; SBOM in a common machine-readable format, at least top-level dependencies | met | CycloneDX 1.6 JSON per platform, every crate and npm package with version, purl, licence and hash, plus fonts and data sets (`tools/sbom.py`), attached to every release as `Caruspex-<version>-sbom-<platform>.cdx.json`. |
| 2 | Remediate without delay, security updates separate from functionality updates where feasible | partly | Fixes go into a new release, delivered by the updater. There is no separate security-only update channel; a security fix can be released as a patch version on its own ([SECURITY.md](../../SECURITY.md)). |
| 3 | Effective and regular security tests and reviews | partly | Tests, clippy with `-D warnings`, property tests of the parsers, sanitizers, audit job. No external review or penetration test. |
| 4 | Publish fixed vulnerabilities with description, affected versions, impact, severity, remediation | met | GitHub security advisories with CVE ids, published with the fix ([SECURITY.md](../../SECURITY.md)). |
| 5 | Coordinated vulnerability disclosure policy | met | [SECURITY.md](../../SECURITY.md): private reporting, response targets, disclosure timeline. |
| 6 | Facilitate sharing of vulnerability information, contact address | partly | GitHub private vulnerability reporting (needs to be switched on in the repository settings); `security.txt` on the documentation website. A manufacturer would also need a contact that is not only an automated form (Article 13(17)). |
| 7 | Secure distribution of updates, automatic for security updates where applicable | met | Signed packages, HTTPS, fixed download prefix, SHA256SUMS and build provenance for manual downloads. Automatic installation is opt-in (see part I 2(c)). |
| 8 | Disseminate security updates without delay, free of charge, with advisory messages | met | Free releases; the update dialog shows the change list; the advisory says what to do. |

## Supply chain in the release workflow

Every release on GitHub carries, next to the packages, `latest.json` and the `.sig` files the updater uses:

| File | What it is | Made by |
|---|---|---|
| `Caruspex-<version>-sbom-windows-x64.cdx.json`, `-linux-x86_64`, `-macos` | SBOM of the packages of that platform | `sbom` job, `tools/sbom.py` |
| `Caruspex-<version>-sbom-android-arm64.cdx.json` | SBOM of the APK (only when the APK is attached) | `sbom` job, attached by `android-release` |
| `SHA256SUMS` | SHA-256 of every file of the release (the APK lines are added when it is attached) | `release` job, `sha256sum` |
| `Caruspex-<version>-provenance.sigstore.json` | Sigstore bundle of the GitHub build provenance for every file in SHA256SUMS | `actions/attest-build-provenance` (pinned by commit SHA) |

The new steps add files with new names only, run after the signing and after `latest.json` is written, and run
before `gh release create`, so a failure leaves no release instead of one without `latest.json`. How users check
a download is in [SECURITY.md](../../SECURITY.md#checking-a-download).

**What the SBOMs cover**: the Rust crates of the workspace and of `Cargo.lock` that are compiled for that
platform (crates only needed at build time, such as proc macros and build scripts, with scope `excluded`;
test-only crates are left out), the npm packages of the interface bundle without the dev tools, the bundled fonts
and data sets. **Not covered**: libraries of the operating system (WebView2, WebKitGTK, WKWebView), the libraries
linuxdeploy copies into the AppImage, the Inno Setup installer stub, and the Gradle and AndroidX dependencies of
the APK.

**Checks**: `python tools/sbom.py --check` (in the `sbom` job of every push and pull request) builds every SBOM
twice, fails on any difference, and checks the CycloneDX rules the generator could break (component types,
purls, hash lengths, licence entries, unique references, dependency references). Validated against the official
CycloneDX 1.6 JSON schema with `cyclonedx-python-lib` 11.12 (`JsonStrictValidator`) on 21 November 2026; to
repeat it:

```bash
python -m venv .venv && .venv/bin/pip install "cyclonedx-python-lib[json-validation]==11.12.0"
python tools/sbom.py --out target/sbom
.venv/bin/python -c "import glob; from cyclonedx.schema import SchemaVersion; from cyclonedx.validation.json import JsonStrictValidator as V; v = V(SchemaVersion.V1_6); [print(p, v.validate_str(open(p).read()) or 'ok') for p in glob.glob('target/sbom/*.json')]"
```

## Annex VII technical documentation: what exists

| Annex VII point | Where |
|---|---|
| 1 General description, intended purpose, versions, user information | [risk-assessment.md](risk-assessment.md), [user-information.md](user-information.md), `changelog/` |
| 2(a) Design and architecture | [docs/ARCHITECTURE.md](../ARCHITECTURE.md), `crates/README.md` |
| 2(b) Vulnerability handling, SBOM, CVD policy, contact, update distribution | [SECURITY.md](../../SECURITY.md), this file, `app/src-tauri/src/update.rs` |
| 2(c) Production and monitoring processes | `.github/workflows/build.yml`, `.github/workflows/audit.yml`, `tools/release.sh` |
| 3 Cybersecurity risk assessment | [risk-assessment.md](risk-assessment.md) |
| 4 Support period | [support.md](support.md) |
| 5 Standards applied | none (no harmonised standards for the CRA have been published yet); the solutions are described here |
| 6 Test reports | CI runs of each release (kept by GitHub for a limited time only) |
| 7 EU declaration of conformity | not drawn up: Caruspex is not placed on the market |

## Open points, by importance

1. **Turn on GitHub private vulnerability reporting and Dependabot security updates** (repository settings, see
   [SECURITY.md](../../SECURITY.md)). Without the first, the reporting link leads nowhere.
2. **Rollback protection in the updater**: bind the version and file name in the signed trusted comment
   (risk assessment, "Malicious update").
3. **Pin all third-party actions by commit SHA** and pin `cryptography` in the release job.
4. **Support period statement**: decided, there is no support period. Caruspex is maintained on a best effort basis by
   one person in spare time, without any promise; if maintenance stops, the repository will be archived and say so
   ([support.md](support.md)).
5. **Replace or follow up the unmaintained crates** reported by RustSec (`chrono` and others).
6. If Caruspex is ever monetised: automatic security updates on by default, a non-automated contact, the
   manufacturer's name and address, EU declaration of conformity and CE marking on the download page, an
   end-of-support notice in the app (Article 13(19)), keeping security updates available for 10 years
   (Article 13(9)).
