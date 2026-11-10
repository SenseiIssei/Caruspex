#!/usr/bin/env bash
# Installs or opens the packages from tools/package.sh the way a user would
# and checks that both programs start: the command line tool reads the demo
# car, the app reports its version (and on Windows also opens the demo car
# and writes a screenshot).
#
# Usage: bash tools/smoke_test.sh [shot-dir]
# Windows only: SMOKE_WINDOW_POS=-1880,40 places the window (for example on a
# second monitor) and keeps it from taking the focus.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)
DIST="$ROOT/dist"
SHOTS=${1:-$ROOT/smoke}
mkdir -p "$SHOTS"
VERSION=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\([0-9.]*\)"/\1/p' Cargo.toml)
WORK=$(mktemp -d)

check_cli() {
    local out
    out=$("$1" version | tr -d '\r')
    echo "$out"
    [ "$out" = "caruspex_cli $VERSION" ] || { echo "unexpected version: $out" >&2; exit 1; }
    "$1" read --demo >/dev/null || { echo "caruspex_cli read --demo failed" >&2; exit 1; }
    echo "caruspex_cli reads the demo car"
}

check_app() {
    local out
    out=$("$1" --version | tr -d '\r')
    echo "$out"
    [ "$out" = "Caruspex $VERSION" ] || { echo "unexpected app version: $out" >&2; exit 1; }
}

check_shot() {
    [ -s "$1" ] || { echo "no screenshot at $1" >&2; exit 1; }
    echo "screenshot $1 ($(wc -c < "$1") bytes)"
}

case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*)
    SETUP="$DIST/Caruspex-$VERSION-windows-x64-setup.exe"
    TARGET="$WORK/Caruspex"
    "$SETUP" //VERYSILENT //SUPPRESSMSGBOXES //CURRENTUSER //NORESTART \
        "//DIR=$(cygpath -w "$TARGET")"
    check_cli "$TARGET/caruspex_cli.exe"
    check_app "$TARGET/caruspex.exe"
    place=()
    [ -n "${SMOKE_WINDOW_POS:-}" ] && place=(--window-pos "$SMOKE_WINDOW_POS")
    CARUSPEX_GARAGE="$WORK/garage.json" "$TARGET/caruspex.exe" --demo --page faults \
        "${place[@]}" --size 1440x920 --shot-delay 5000 --shot "$(cygpath -w "$SHOTS")\\windows.png"
    check_shot "$SHOTS/windows.png"
    "$TARGET/unins000.exe" //VERYSILENT //SUPPRESSMSGBOXES
    sleep 3
    [ ! -f "$TARGET/caruspex.exe" ] || { echo "uninstall left caruspex.exe" >&2; exit 1; }
    echo "uninstalled"
    ;;

Linux)
    IMAGE="$DIST/Caruspex-$VERSION-linux-x86_64.AppImage"
    chmod +x "$IMAGE"
    (cd "$WORK" && "$IMAGE" --appimage-extract >/dev/null)
    check_app "$WORK/squashfs-root/usr/bin/caruspex"
    tar -xzf "$DIST/caruspex_cli-$VERSION-linux-x86_64.tar.gz" -C "$WORK"
    check_cli "$WORK/caruspex_cli"
    dpkg-deb --info "$DIST/Caruspex-$VERSION-linux-amd64.deb" >/dev/null
    echo "deb package is readable"
    ;;

Darwin)
    MOUNT="$WORK/mnt"
    hdiutil attach "$DIST/Caruspex-$VERSION-macos.dmg" -mountpoint "$MOUNT" -nobrowse -quiet
    check_app "$MOUNT/Caruspex.app/Contents/MacOS/caruspex"
    hdiutil detach "$MOUNT" -quiet
    tar -xzf "$DIST/caruspex_cli-$VERSION-macos.tar.gz" -C "$WORK"
    check_cli "$WORK/caruspex_cli"
    ;;
esac
echo "smoke test passed"
