# Packaging

| Folder | What it holds |
|---|---|
| `windows/` | The Inno Setup script of the Windows installer, built by `tools/package.sh` |
| `winget/` | The four winget manifests (templates) for `SenseiIssei.Caruspex` |
| `homebrew/` | The Homebrew cask (template) for a tap repository |
| `linux/` | Desktop file and AppStream metainfo with the app ID `dev.senseiissei.caruspex`, used by Flathub and Snap |
| `flathub/` | The Flatpak manifest (template) for Flathub |
| `snap/` | `snapcraft.yaml` (template) for the Snap Store |

The templates contain placeholders such as `@VERSION@` and `@SHA256_LINUX_DEB@`. They
are never submitted as they are: after a release has been published, fill them in with

```bash
py -I tools/dist_manifests.py 0.14.0
```

The script downloads `Caruspex-<version>-windows-x64-setup.exe`, `Caruspex-<version>-macos.dmg` and
`Caruspex-<version>-linux-amd64.deb` of the GitHub release `v<version>` (or takes them from a folder with
`--from dist`), computes their SHA-256, takes the date from the release's `latest.json` and the release
notes from `changelog/<version>.md`, and writes everything to `dist/manifests/<version>/`. The CI only
checks the templates offline (`--check-templates`).

All four packages install the same files as the release: the Inno Setup installer, the dmg, and
for Flathub and Snap the `.deb` built on Ubuntu 22.04. Nothing here is built from source on the
stores' machines.

Nothing in this folder has been submitted anywhere yet. Each step below is done by the owner by hand.

## winget

1. Fork [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) once.
2. Copy `dist/manifests/<version>/winget/manifests/s/SenseiIssei/Caruspex/<version>/` into the same path in the
   fork, on a new branch.
3. Check locally: `winget validate --manifest <that folder>`, and if wanted install it in a Windows
   Sandbox: `winget install --manifest <that folder>`.
4. Open a pull request against `microsoft/winget-pkgs`. The bots there run the installer in a sandbox; the
   first submission is reviewed by a moderator.

For later versions the same, or `wingetcreate update SenseiIssei.Caruspex --version <version> --urls <setup.exe url>`.

Notes: the installer installs per user without admin rights (`Scope: user`, `/CURRENTUSER`) and for all
users with `/ALLUSERS`. The silent switches are the Inno Setup ones (`/VERYSILENT /SUPPRESSMSGBOXES
/NORESTART /SP-`). The in-app updater keeps working for winget users, it runs the same installer.

## Homebrew

The cask is meant for an own tap first (`homebrew/cask` itself wants notarised apps, see Phase 18 of PLAN.md).

1. Create a public repository `SenseiIssei/homebrew-caruspex` on GitHub.
2. Copy `dist/manifests/<version>/homebrew/Casks/caruspex.rb` to `Casks/caruspex.rb` there and push.
3. Users install with `brew install --cask senseiissei/caruspex/caruspex`.
4. Before publishing: `brew audit --cask --new senseiissei/caruspex/caruspex` and `brew style` on the file.

The app is not notarised yet, so macOS asks on the first start (right click, "Open"). `auto_updates true`
tells Homebrew that the app updates itself.

## Flathub

1. Read the [submission guide](https://docs.flathub.org/docs/for-app-authors/submission) and fork
   [flathub/flathub](https://github.com/flathub/flathub).
2. On a branch from `new-pr`, add the three files from `dist/manifests/<version>/flathub/`.
3. Build and test locally on Linux:
   `flatpak run org.flatpak.Builder --force-clean --user --install --install-deps-from=flathub builddir dev.senseiissei.caruspex.yml`,
   then `flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest dev.senseiissei.caruspex.yml`
   and `flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo repo`.
4. Open the pull request against `new-pr`. Flathub checks that the app ID belongs to the submitter: the
   domain written backwards in `dev.senseiissei.caruspex` has to be verified during the review. The ID
   stays as it is, the app already uses it for its settings and updates.
5. After acceptance Flathub creates the repository `flathub/dev.senseiissei.caruspex`; later versions are
   pull requests there (the `x-checker-data` entry lets Flathub's checker open them by itself).

Before submitting, check that the chosen `runtime-version` of `org.gnome.Platform` is still supported and
still contains WebKitGTK 4.1 (`libwebkit2gtk-4.1.so.0`). The permissions are explained in the manifest:
serial adapters need `--device=all`, Bluetooth `--allow=bluetooth` and BlueZ, SocketCAN `--allow=canbus`.
Inside Flatpak the app does not install updates itself; Flathub does that.

## Snap

1. Install snapcraft on Linux and log in: `snapcraft login`.
2. Reserve the name once: `snapcraft register caruspex`.
3. In `dist/manifests/<version>/snap/` run `snapcraft` (it builds in an LXD or Multipass machine), then
   test with `sudo snap install --dangerous caruspex_<version>_amd64.snap`.
4. Upload: `snapcraft upload --release=stable caruspex_<version>_amd64.snap`.

The interfaces `raw-usb` (serial adapters), `bluez` and `can-bus` are not connected automatically. Users
connect them once with `sudo snap connect caruspex:raw-usb` and so on, or the owner asks for automatic
connection in the [Snapcraft forum](https://forum.snapcraft.io/c/store-requests/19). Inside the snap the
app does not install updates itself; the Snap Store does that.

## What was checked locally

- `py -I tools/dist_manifests.py 0.13.0` against the published 0.13.0 release: hashes, dates and notes
  filled in, and `winget validate` passes on the result.
- The desktop file and the metainfo are checked by the script for their required keys; `appstreamcli`,
  `desktop-file-validate`, `flatpak-builder` and `snapcraft` were not available on the machine where
  these files were written, so the Flathub and Snap builds have not run yet.
