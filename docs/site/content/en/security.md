# Security and updates

This page is the security information for Caruspex: what the program is for, which risks to know about, how
updates work and how to remove it. It follows the user information that the EU Cyber Resilience Act asks for
(Annex II), as far as it applies to a free, non-commercial program.

## The program

- **Name**: Caruspex, the app `caruspex` and the command-line tool `caruspex_cli`, for Windows, Linux and macOS.
  The version is in the name of every download and in the change list of each release.
- **Published by**: the Caruspex project on [GitHub](https://github.com/SenseiIssei/Caruspex), as free software
  under the MIT licence. Questions and bugs go to the [issues](https://github.com/SenseiIssei/Caruspex/issues).
- **Purpose**: reading and explaining the diagnostic data of cars, motorcycles and trucks through an adapter you
  own, on your own computer. There is no account, no cloud and no tracking ([Privacy](privacy.md)).
- **Security properties**: reading never changes anything in the car, and anything that makes the car do
  something asks first. Updates are installed only when their signature matches the key built into the app.
  Caruspex opens no network port and runs no code from files or from the internet.

## Reporting a vulnerability {#report}

Please report security problems privately with the
[form for a new advisory](https://github.com/SenseiIssei/Caruspex/security/advisories/new), not in a public issue.
The [security policy](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md) says what happens next.
Fixed vulnerabilities are published as [security advisories](https://github.com/SenseiIssei/Caruspex/security/advisories).

## Risks to know about {#risks}

- Download Caruspex only from the [releases page](https://github.com/SenseiIssei/Caruspex/releases). Copies from
  elsewhere may be changed.
- Open recordings, PID packs and 3D models only from people you trust. They cannot run code, but a broken file
  can make Caruspex slow or close it.
- Clear fault codes, start output tests and basic settings only with the car standing still and in a safe place.
- Many Bluetooth and Wi-Fi adapters have no PIN or an open network: anyone nearby can connect to them. Unplug the
  adapter when you are done.
- ENET cables, DoIP and Wi-Fi adapters have no encryption: use them directly or in your own network only.
- J2534 drivers are programs of their makers and run with your rights. Install them only from the maker.

## Updates and support {#support}

- At start Caruspex asks GitHub whether there is a newer version and shows it. Installing takes one click, and the
  app only installs a package whose signature matches. The AppImage and the macOS app update themselves; the .deb
  package and the Android app get the download page.
- Both the check and the automatic installation are switches in the settings, under **Window and language**.
  Without the check, look at the releases page now and then.
- Security fixes go into the newest release only. Older versions get no separate fixes, so keep Caruspex up to date.
- Caruspex is maintained in spare time and has no fixed end date. If maintenance stops, the repository and a last
  release will say so in advance.

## Checking a download {#verify}

Every release has a file `SHA256SUMS` with the checksum of every file, and GitHub's signed record of how the files
were built. With the files in one folder:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

Without these tools, `Get-FileHash <file>` in PowerShell (Windows) or `shasum -a 256 <file>` (macOS) shows the
checksum to compare with the line in `SHA256SUMS`.

## Removing Caruspex and your data {#remove}

1. Uninstall the program: on Windows under Settings, Apps; on macOS drag Caruspex out of Applications; on Linux
   delete the AppImage or run `sudo apt remove caruspex`.
2. Delete your data: the button **Open storage location** in the settings shows the folders with the settings,
   the garage and the crash reports. Delete them. Recordings and reports you saved yourself are where you saved them.

## For developers and companies {#integrators}

- Every release has a software bill of materials per platform, `Caruspex-<version>-sbom-<platform>.cdx.json`
  (CycloneDX), with every library and its version.
- The risk assessment, the list of requirements and the scope of the Cyber Resilience Act for Caruspex are in
  [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra).
- Caruspex is not placed on the market as a product in the sense of the Cyber Resilience Act, so there is no EU
  declaration of conformity and no CE marking.
