# Security policy

How Caruspex handles vulnerabilities, how to report one and how to check a download. What the EU Cyber Resilience
Act means for Caruspex is in [docs/cra/](docs/cra/scope.md): the scope, the risk assessment, the essential
requirements, the user information and the support period. For users the short version is the
[security page of the documentation](https://senseiissei.github.io/Caruspex/en/security.html).

## Reporting a vulnerability

Please do not report security problems in a public issue, a pull request or a discussion.

Report them privately with GitHub's private vulnerability reporting: open the
[Security tab of the repository](https://github.com/SenseiIssei/Caruspex/security) and choose
**Report a vulnerability**, or go straight to
[the form for a new advisory](https://github.com/SenseiIssei/Caruspex/security/advisories/new).
Only the maintainers of Caruspex see the report. Reports in English or German are welcome.

Please include what is affected (version, system, which part of Caruspex), how to reproduce it, and what an
attacker could do with it. You will get an answer in the advisory. Once a fix is released, the advisory is
published, with credit to you if you want it.

## What counts

Caruspex runs on the user's computer or phone and talks to a car through an adapter. Examples of what we want to
hear about:

- the updater installing something that was not signed with the release key, or downloading from anywhere but the
  releases of this repository (`app/src-tauri/src/update.rs`)
- a file the user opens (a recording for replay, a custom PID CSV, a PID pack, a 3D model, a garage file) that
  makes Caruspex run code, read or write files it should not, or crash in a way that can be abused
- personal data leaving the computer without the user's action, or "Share recording" and the crash reports
  keeping a full VIN, an address or the user's home folder that they promise to remove
- any way for Caruspex to write to a car without the user's confirmation, or to send requests the safety rules in
  [CONTRIBUTING.md](CONTRIBUTING.md) rule out
- a problem in the CI that could let a pull request reach the release secrets

Weaknesses of cars, control units or adapters themselves are not problems of Caruspex; please report those to their
makers.

## Supported versions

Fixes go into the newest release, which the built-in updater offers to every installation. Older versions do not
get separate fixes.

| Version | Security fixes |
|---|---|
| the newest release | yes |
| every older release | no, update to the newest |

**Support period**: Caruspex is a free, non-commercial project maintained in spare time. There is no fixed end
date and no contract. Vulnerabilities are handled as described here for as long as the project is maintained; if
maintenance stops, the repository, a last release and the documentation website will say so in advance, at least
90 days ahead where possible. Old releases stay on the releases page but are unsupported and may contain known
vulnerabilities. The full statement is in [docs/cra/support.md](docs/cra/support.md).

## How a report is handled

Caruspex is maintained in spare time, so these are targets, not guarantees:

| Step | Target |
|---|---|
| First answer in the advisory | within 7 days |
| Assessment (confirmed or not, severity with CVSS) | within 14 days |
| Fix released, critical or high severity | within 30 days of the confirmation |
| Fix released, medium or low severity | with the next release, at the latest within 90 days |

**Coordinated disclosure**: please keep the problem private until a fixed release is out. The advisory is
published together with the fixed release, at the latest 90 days after the report unless we agree on another
date with you (for example when a fix depends on another project). If the problem is already being exploited or
publicly known, the fix and the advisory come as fast as possible.

**Publication**: fixed vulnerabilities are published as
[GitHub security advisories](https://github.com/SenseiIssei/Caruspex/security/advisories) with the affected and
the fixed versions, the impact, the severity and what users should do. GitHub assigns a CVE id on request from
the advisory, so the advisory also reaches the CVE list and the GitHub Advisory Database. The release notes of the
fixed release point to the advisory, and the built-in updater offers the release to every installation.

A vulnerability in a library that Caruspex uses is also reported to that library's maintainers, with the fix
where we have one.

## Checking a download

Every release has, next to the packages:

- `SHA256SUMS`: the SHA-256 checksum of every file of the release,
- `Caruspex-<version>-provenance.sigstore.json`: GitHub's signed build provenance (Sigstore) for those files,
- `Caruspex-<version>-sbom-<platform>.cdx.json`: the software bill of materials (CycloneDX) of each platform,
- `*.sig`: the minisign signatures that the built-in updater checks.

With the downloaded files in one folder:

```bash
# The checksums (Linux; on macOS: shasum -a 256 <file>; on Windows: Get-FileHash <file>)
sha256sum --ignore-missing -c SHA256SUMS

# That the file was built by this repository's release workflow from a tagged commit (needs the GitHub CLI)
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
# The same offline, with the bundle of the release
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex \
  --bundle Caruspex-<version>-provenance.sigstore.json
```

Updates installed by the app need none of this: the updater only downloads from this repository's releases over
HTTPS and installs a package only when its minisign signature matches the public key built into the app
(`app/src-tauri/src/update.rs`). To check such a signature by hand with [minisign](https://jedisct1.github.io/minisign/):

```bash
base64 -d < <file>.sig > <file>.minisig
# PUBLIC_KEY is the string of the same name in app/src-tauri/src/update.rs
echo "$PUBLIC_KEY" | base64 -d > caruspex-updater.pub
minisign -V -p caruspex-updater.pub -m <file> -x <file>.minisig
```

## Dependencies

- `.github/workflows/audit.yml` runs `tools/audit.py` on every change of the dependencies and once a week: the
  RustSec advisory database for every crate in `Cargo.lock`, and `npm audit` for the packages of the interface.
  It fails on any known vulnerability.
- A finding that does not affect Caruspex (the vulnerable function is not used, the platform is not built) is
  accepted in `tools/audit_exceptions.toml` with the advisory id, the package, the reason and an expiry date at
  most one year ahead. An expired entry fails the audit until it is checked again. Unmaintained and unsound
  crates are shown as warnings.
- Dependabot (`.github/dependabot.yml`) proposes updates for Cargo, npm and the GitHub Actions every month, and
  security updates for these and for the Gradle files of the Android app.

## For maintainers

**Repository settings** that this policy relies on (Settings, Advanced Security): private vulnerability
reporting on, Dependabot alerts and Dependabot security updates on.

**Checklist for an actively exploited vulnerability or a severe incident.** Caruspex is free software that is not
monetised and therefore not placed on the market in the sense of the EU Cyber Resilience Act (Regulation (EU)
2024/2847); the reporting duty of its Article 14 does not apply to it today ([docs/cra/scope.md](docs/cra/scope.md)).
Voluntary reports are possible (Article 15). If Caruspex is ever monetised, or a severe incident hits the release
chain, follow these steps:

1. Write down when you became aware: the deadlines run from that moment (Article 14(2) and (4)).
   - An **actively exploited vulnerability** is one with reliable evidence that a malicious actor used it
     (Article 3(42)); a report from a researcher without such evidence is not one.
   - A **severe incident** affects the protection of important data or functions, or led or could lead to
     malicious code in Caruspex or on users' systems (Article 14(5)), for example a leaked updater key, a hijacked
     GitHub account or a manipulated release.
2. **Within 24 hours**: early warning through ENISA's
   [single reporting platform](https://www.enisa.europa.eu/topics/product-security/single-reporting-platform-srp)
   to the CSIRT designated as coordinator of the Member State of the main establishment (for Germany CERT-Bund at
   the BSI, see ENISA's list) and to ENISA, with the Member States where Caruspex is used, if known (Article
   14(1), (2)(a), (7)). For an incident, say whether it is suspected to be malicious (Article 14(4)(a)).
3. **Within 72 hours**: the notification with the affected versions, the nature of the exploit or incident, what
   was done and what users can do, and how sensitive the information is (Article 14(2)(b), (4)(b)).
4. **Final report**: for a vulnerability no later than 14 days after a fix or mitigation is available; for an
   incident within one month after the 72-hour notification (Article 14(2)(c), (4)(c)).
5. **Inform the users** without waiting for the final report: a security advisory, a note in the release, an
   update through the updater (Article 14(8)). If the updater key may be compromised, say so and do not publish
   updates signed with it.
6. Keep the times, reports and decisions with the advisory (Article 13(7)).
