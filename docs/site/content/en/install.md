# Install

Every release is on the [releases page on GitHub](https://github.com/SenseiIssei/Caruspex/releases/latest). The packages are not signed yet, so Windows and macOS ask once before the first start.

## Windows 10 and 11

| File | What it is |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | The installer. It needs no admin rights, can put the command-line tool `caruspex_cli` on the search path and installs the WebView2 runtime only if it is missing. |
| `Caruspex-<version>-windows-x64.zip` | Portable: unzip it and start `caruspex.exe`. |

On the first start Windows SmartScreen may say that the app is unknown: click "More info", then "Run anyway".

USB adapters need a driver for their USB chip. Windows 10 and 11 usually find it by themselves; if not, see [ELM327 over USB](elm327-usb.md).

## macOS 13 and newer (Apple Silicon)

Download `Caruspex-<version>-macos.dmg`, open it and drag Caruspex to Applications. Because the app is not notarised yet, open it the first time with a right click and "Open".

## Linux (x86_64)

| File | What it is |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | Runs on almost every distribution: make it executable with `chmod +x` and start it. It also updates itself. |
| `Caruspex-<version>-linux-amd64.deb` | For Debian and Ubuntu: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

To use serial adapters without `sudo`, add yourself to the `dialout` group once, then log out and back in:

```
sudo usermod -aG dialout $USER
```

## Command-line tool only

For scripts and servers there is `caruspex_cli` without the interface, for Windows, Linux and macOS. `caruspex_cli help` lists the commands.

`caruspex_cli report` writes the diagnostic report as a PDF file, without the app and over every connection the other commands know (`--demo`, `--port`, `--tcp`, `--enet`, `--can`, `--replay`):

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

The report can be written in German, English, Spanish, French, Portuguese, Italian, Dutch and Indonesian (`--lang`). For the other languages it comes out in English, because the fonts every PDF viewer has built in lack their letters.

## Updates

At start Caruspex asks GitHub whether there is a newer version, and nothing else. A new version is only installed if its signature matches the key built into the app. Both the check and the automatic installation can be switched off in the settings.

## Package managers

Packages for winget, Homebrew, Flathub and Snap are being prepared. Until they are published, the files above are the way to install Caruspex.
