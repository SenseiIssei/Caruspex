#!/usr/bin/env bash
# Installs or opens the packages from tools/package.sh the way a user would
# and checks that both programs start: the command line tool reads the demo
# car, the app reports its version, opens the demo car and writes a
# screenshot (on macOS and Linux also a PDF report).
#
# Usage: bash tools/smoke_test.sh [shot-dir]
# SMOKE_WINDOW_POS=-1880,40 places the window (for example on a second
# monitor) and keeps it from taking the focus. On Linux the app runs on a
# virtual screen through xvfb-run when that is installed.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)
DIST="$ROOT/dist"
SHOTS=${1:-$ROOT/smoke}
mkdir -p "$SHOTS"
SHOTS=$(cd "$SHOTS" && pwd)
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

check_pdf() {
    [ -s "$1" ] || { echo "no PDF report at $1" >&2; exit 1; }
    [ "$(head -c 5 "$1")" = "%PDF-" ] || { echo "$1 is no PDF" >&2; exit 1; }
    echo "PDF report $1 ($(wc -c < "$1") bytes)"
}

# Opens the demo car with the app given as arguments: once for a picture of
# the fault page, once for a PDF report. Both runs end the app themselves.
check_demo() {
    local name=$1
    shift
    local place=()
    [ -n "${SMOKE_WINDOW_POS:-}" ] && place=(--window-pos "$SMOKE_WINDOW_POS")
    CARUSPEX_GARAGE="$WORK/garage.json" "$@" --demo --page faults \
        ${place[@]+"${place[@]}"} --size 1440x920 --shot-delay 5000 --shot "$SHOTS/$name.png"
    check_shot "$SHOTS/$name.png"
    CARUSPEX_GARAGE="$WORK/garage.json" "$@" --demo \
        ${place[@]+"${place[@]}"} --size 1440x920 --report "$SHOTS/$name.pdf"
    check_pdf "$SHOTS/$name.pdf"
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
    # The program from the .deb runs against the system's WebKitGTK, like
    # after an installation. Without a screen it gets a virtual one; the
    # DMA-BUF renderer of newer WebKitGTK needs a GPU that xvfb lacks.
    dpkg-deb -x "$DIST/Caruspex-$VERSION-linux-amd64.deb" "$WORK/deb"
    screen=()
    command -v xvfb-run >/dev/null && screen=(xvfb-run -a -s "-screen 0 1600x1000x24")
    export WEBKIT_DISABLE_DMABUF_RENDERER=1
    check_demo linux ${screen[@]+"${screen[@]}"} "$WORK/deb/usr/bin/caruspex"
    ;;

Darwin)
    MOUNT="$WORK/mnt"
    hdiutil attach "$DIST/Caruspex-$VERSION-macos.dmg" -mountpoint "$MOUNT" -nobrowse -quiet
    check_app "$MOUNT/Caruspex.app/Contents/MacOS/caruspex"
    check_demo macos "$MOUNT/Caruspex.app/Contents/MacOS/caruspex"
    hdiutil detach "$MOUNT" -quiet
    tar -xzf "$DIST/caruspex_cli-$VERSION-macos.tar.gz" -C "$WORK"
    check_cli "$WORK/caruspex_cli"
    ;;
esac
echo "smoke test passed"
