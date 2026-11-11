#!/usr/bin/env bash
# Builds the release packages of this system into dist/:
#
#   Windows  dist/Caruspex-<v>-windows-x64-setup.exe  (Inno Setup, same AppId as before,
#                                                      so it updates an installed 0.8 in place)
#            dist/Caruspex-<v>-windows-x64.zip        (portable, needs the WebView2 runtime)
#            dist/caruspex_cli-<v>-windows-x64.zip
#   Linux    dist/Caruspex-<v>-linux-x86_64.AppImage
#            dist/Caruspex-<v>-linux-amd64.deb
#            dist/caruspex_cli-<v>-linux-x86_64.tar.gz
#   macOS    dist/Caruspex-<v>-macos.dmg
#            dist/Caruspex-<v>-macos.app.tar.gz      (the app bundle for the updater)
#            dist/caruspex_cli-<v>-macos.tar.gz
#
# The release job signs the update packages (setup.exe, AppImage,
# .app.tar.gz) and writes latest.json, see tools/release_assets.py.
#
# Usage: bash tools/package.sh
# REQUIRE_INSTALLER=1 fails instead of skipping the Windows installer when
# Inno Setup is missing.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)
VERSION=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\([0-9.]*\)"/\1/p' Cargo.toml)
[ -n "$VERSION" ] || { echo "no version in Cargo.toml" >&2; exit 1; }
echo "Caruspex $VERSION"

export CARGO_TARGET_DIR="$ROOT/target"
STAGE="$ROOT/package"
DIST="$ROOT/dist"
RELEASE="$CARGO_TARGET_DIR/release"
rm -rf "$STAGE" "$DIST"
mkdir -p "$STAGE" "$DIST"

[ -d app/node_modules ] || npm --prefix app ci
cargo build --release -p caruspex-cli

case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*)
    npm --prefix app run tauri -- build --no-bundle
    cp "$RELEASE/caruspex.exe" "$RELEASE/caruspex_cli.exe" LICENSE "$STAGE/"

    (cd "$STAGE" && powershell -NoProfile -Command \
        "Compress-Archive -Path caruspex.exe,caruspex_cli.exe,LICENSE -DestinationPath '$(cygpath -w "$DIST")\\Caruspex-$VERSION-windows-x64.zip'")
    (cd "$STAGE" && powershell -NoProfile -Command \
        "Compress-Archive -Path caruspex_cli.exe,LICENSE -DestinationPath '$(cygpath -w "$DIST")\\caruspex_cli-$VERSION-windows-x64.zip'")

    # Microsoft's small WebView2 bootstrapper: the installer runs it only on
    # the rare Windows without the runtime.
    BOOT="$ROOT/target/MicrosoftEdgeWebview2Setup.exe"
    [ -s "$BOOT" ] || curl -fsSL -o "$BOOT" "https://go.microsoft.com/fwlink/p/?LinkId=2124703"
    cp "$BOOT" "$STAGE/"

    ISCC=$(command -v iscc || true)
    for candidate in "/c/Program Files (x86)/Inno Setup 6/ISCC.exe" \
                     "${LOCALAPPDATA:-}/Programs/Inno Setup 6/ISCC.exe"; do
        [ -z "$ISCC" ] && [ -f "$candidate" ] && ISCC=$candidate
    done
    if [ -n "$ISCC" ]; then
        "$ISCC" -Q "-DAppVersion=$VERSION" "-DSourceDir=$(cygpath -w "$STAGE")" \
            "-DOutputDir=$(cygpath -w "$DIST")" packaging/windows/caruspex.iss
    elif [ "${REQUIRE_INSTALLER:-0}" = 1 ]; then
        echo "Inno Setup 6 (ISCC.exe) not found" >&2
        exit 1
    else
        echo "Inno Setup 6 not found, skipping the installer"
    fi
    ;;

Linux)
    export APPIMAGE_EXTRACT_AND_RUN=1
    npm --prefix app run tauri -- build --bundles appimage,deb
    cp "$RELEASE"/bundle/appimage/*.AppImage "$DIST/Caruspex-$VERSION-linux-x86_64.AppImage"
    cp "$RELEASE"/bundle/deb/*.deb "$DIST/Caruspex-$VERSION-linux-amd64.deb"
    tar -czf "$DIST/caruspex_cli-$VERSION-linux-x86_64.tar.gz" \
        -C "$RELEASE" caruspex_cli -C "$ROOT" LICENSE
    ;;

Darwin)
    npm --prefix app run tauri -- build --bundles app,dmg
    cp "$RELEASE"/bundle/dmg/*.dmg "$DIST/Caruspex-$VERSION-macos.dmg"
    # The bundle itself for the updater, which replaces the installed one
    # with it. COPYFILE_DISABLE keeps the ._ files of extended attributes out.
    BUNDLE=$(ls -d "$RELEASE"/bundle/macos/*.app | head -n 1)
    COPYFILE_DISABLE=1 tar -czf "$DIST/Caruspex-$VERSION-macos.app.tar.gz" \
        -C "$(dirname "$BUNDLE")" "$(basename "$BUNDLE")"
    tar -czf "$DIST/caruspex_cli-$VERSION-macos.tar.gz" \
        -C "$RELEASE" caruspex_cli -C "$ROOT" LICENSE
    ;;

*)
    echo "unsupported system $(uname -s)" >&2
    exit 1
    ;;
esac

ls -l "$DIST"
