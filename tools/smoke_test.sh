#!/usr/bin/env bash
# Installs or opens the packages from tools/package.sh the way a user would
# and checks that both programs start: the command line tool prints its
# version, the app opens the demo car and writes a screenshot.
#
# Usage: bash tools/smoke_test.sh [shot-dir]
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)
DIST="$ROOT/dist"
SHOTS=${1:-$ROOT/smoke}
mkdir -p "$SHOTS"
VERSION=$(sed -n 's/^project(Caruspex VERSION \([0-9.]*\).*/\1/p' CMakeLists.txt)
WORK=$(mktemp -d)

check_cli() {
    local out
    out=$("$1" version | tr -d '\r')
    echo "$out"
    [ "$out" = "caruspex_cli $VERSION" ] || { echo "unexpected version: $out" >&2; exit 1; }
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
    "$TARGET/caruspex.exe" --demo --page faults --shot-delay 3000 \
        --shot "$(cygpath -w "$SHOTS")\\windows.png"
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
    check_cli "$WORK/squashfs-root/usr/bin/caruspex_cli"
    APPIMAGE_EXTRACT_AND_RUN=1 QT_QPA_PLATFORM=offscreen \
        "$IMAGE" --demo --page faults --shot-delay 3000 --shot "$SHOTS/linux.png"
    check_shot "$SHOTS/linux.png"
    tar -xzf "$DIST/caruspex_cli-$VERSION-linux-x86_64.tar.gz" -C "$WORK"
    check_cli "$WORK/caruspex_cli"
    ;;

Darwin)
    MOUNT="$WORK/mnt"
    hdiutil attach "$DIST/Caruspex-$VERSION-macos.dmg" -mountpoint "$MOUNT" -nobrowse -quiet
    check_cli "$MOUNT/caruspex_cli"
    "$MOUNT/Caruspex.app/Contents/MacOS/caruspex" --demo --page faults --shot-delay 3000 \
        --shot "$SHOTS/macos.png"
    check_shot "$SHOTS/macos.png"
    hdiutil detach "$MOUNT" -quiet
    ;;
esac
echo "smoke test passed"
