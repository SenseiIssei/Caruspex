#!/usr/bin/env bash
# Builds the downloadable packages from an existing Release build:
#
#   Windows  dist/Caruspex-<v>-windows-x64-setup.exe  (Inno Setup)
#            dist/Caruspex-<v>-windows-x64.zip        (portable)
#   Linux    dist/Caruspex-<v>-linux-x86_64.AppImage
#            dist/caruspex_cli-<v>-linux-x86_64.tar.gz
#   macOS    dist/Caruspex-<v>-macos.dmg
#
# Usage: bash tools/package.sh [build-dir]
# QT_ROOT_DIR points at the Qt installation (the CI sets it). On Windows the
# installer needs Inno Setup 6; without it only the zip is made, unless
# REQUIRE_INSTALLER=1 (CI) turns that into an error.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)
BUILD=${1:-build}
VERSION=$(sed -n 's/^project(Caruspex VERSION \([0-9.]*\).*/\1/p' CMakeLists.txt)
[ -n "$VERSION" ] || { echo "no version in CMakeLists.txt" >&2; exit 1; }
echo "Caruspex $VERSION"

STAGE="$ROOT/package"
DIST="$ROOT/dist"
rm -rf "$STAGE" "$DIST"
mkdir -p "$STAGE" "$DIST"

case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*)
    QT_ROOT_DIR=${QT_ROOT_DIR:-C:/Qt/6.8.3/msvc2022_64}
    BIN="$BUILD/bin/Release"
    cp "$BIN/caruspex.exe" "$BIN/caruspex_cli.exe" LICENSE "$STAGE/"
    "$QT_ROOT_DIR/bin/windeployqt.exe" --release --no-translations \
        --no-system-d3d-compiler --no-opengl-sw "$STAGE/caruspex.exe"

    # The Visual C++ runtime goes next to the programs (app-local), so
    # nothing has to be installed system wide.
    VSWHERE="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
    VS=$("$VSWHERE" -latest -products '*' -property installationPath | tr -d '\r')
    CRT=$(ls -d "$(cygpath -u "$VS")"/VC/Redist/MSVC/*/x64/Microsoft.VC14*.CRT 2>/dev/null | sort -V | tail -1)
    [ -n "$CRT" ] || { echo "Visual C++ runtime not found under $VS" >&2; exit 1; }
    cp "$CRT"/*.dll "$STAGE/"
    echo "runtime from $CRT"

    (cd "$STAGE" && powershell -NoProfile -Command \
        "Compress-Archive -Path * -DestinationPath '$(cygpath -w "$DIST")\\Caruspex-$VERSION-windows-x64.zip'")

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
    QT_ROOT_DIR=${QT_ROOT_DIR:?set QT_ROOT_DIR to the Qt installation}
    APPDIR="$STAGE/AppDir"
    cmake --install "$BUILD" --prefix "$APPDIR/usr"

    TOOLS="$BUILD/linuxdeploy"
    mkdir -p "$TOOLS"
    for tool in linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage \
                linuxdeploy-plugin-qt/releases/download/continuous/linuxdeploy-plugin-qt-x86_64.AppImage; do
        file="$TOOLS/$(basename "$tool")"
        [ -f "$file" ] || curl -fsSL -o "$file" "https://github.com/linuxdeploy/$tool"
        chmod +x "$file"
    done

    # No FUSE on build machines; the offscreen plugin lets the package be
    # started without a display for the smoke test.
    export APPIMAGE_EXTRACT_AND_RUN=1
    export QMAKE="$QT_ROOT_DIR/bin/qmake"
    export EXTRA_PLATFORM_PLUGINS="libqoffscreen.so"
    export LDAI_OUTPUT="$DIST/Caruspex-$VERSION-linux-x86_64.AppImage"
    export OUTPUT="$LDAI_OUTPUT"
    PATH="$TOOLS:$PATH" "$TOOLS/linuxdeploy-x86_64.AppImage" \
        --appdir "$APPDIR" \
        --executable "$APPDIR/usr/bin/caruspex" \
        --executable "$APPDIR/usr/bin/caruspex_cli" \
        --desktop-file "$APPDIR/usr/share/applications/caruspex.desktop" \
        --icon-file resources/icons/caruspex.png \
        --plugin qt --output appimage

    tar -czf "$DIST/caruspex_cli-$VERSION-linux-x86_64.tar.gz" \
        -C "$BUILD/bin" caruspex_cli -C "$ROOT" LICENSE
    ;;

Darwin)
    QT_ROOT_DIR=${QT_ROOT_DIR:?set QT_ROOT_DIR to the Qt installation}
    cp -R "$BUILD/bin/caruspex.app" "$STAGE/Caruspex.app"
    "$QT_ROOT_DIR/bin/macdeployqt" "$STAGE/Caruspex.app"
    # Not notarised: an ad-hoc signature keeps Apple Silicon starting it,
    # Gatekeeper still asks once (right click, Open).
    codesign --force --deep --sign - "$STAGE/Caruspex.app"
    cp "$BUILD/bin/caruspex_cli" LICENSE "$STAGE/"
    ln -s /Applications "$STAGE/Applications"
    hdiutil create -volname "Caruspex $VERSION" -srcfolder "$STAGE" -ov -format UDZO \
        "$DIST/Caruspex-$VERSION-macos.dmg"
    ;;

*)
    echo "unknown system $(uname -s)" >&2
    exit 1
    ;;
esac

echo
ls -l "$DIST"
