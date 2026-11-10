#!/usr/bin/env bash
# Takes the README screenshots with the built-in demo car.
#
#   tools/screenshots.sh de      German, into docs/de/
#   tools/screenshots.sh en      English, into docs/
#
# Needs the release build of the app (cargo build --release -p caruspex-app
# or tools/package.sh). Uses a throwaway garage so your own one stays
# untouched. SHOT_WINDOW_POS places the windows (default: on a monitor left
# of the main one, so the main screen stays free); they never take the focus.
set -euo pipefail

lang="${1:-de}"
here="$(cd "$(dirname "$0")/.." && pwd)"
app="$here/target/release/caruspex"
[ -x "$app" ] || app="$here/target/release/caruspex.exe"
[ -x "$app" ] || { echo "build the app first: cargo build --release -p caruspex-app" >&2; exit 1; }
out="$here/docs"
[ "$lang" = "de" ] && out="$here/docs/de"
mkdir -p "$out"

garage="$(mktemp -d)/garage.json"
pos="${SHOT_WINDOW_POS:--1880,40}"
size="1440x920"

shot() {   # name page delay
    local file="$out/screenshot-$1.png"
    command -v cygpath >/dev/null && file="$(cygpath -w "$file")"
    CARUSPEX_GARAGE="$garage" "$app" --theme dark --language "$lang" --demo --page "$2" \
        --window-pos "$pos" --size "$size" --shot "$file" --shot-delay "$3"
}

# One after the other: every window sits at the same spot.
shot overview start 7000
shot faults faults 7000
shot live live-run 16000
shot health health-run 16000
shot adapter adapter-run 9000
shot ecus ecus-run 16000
shot vehicles vehicles 5000
shot connect connect 5000
shot battery battery 7000
echo "screenshots in $out"
