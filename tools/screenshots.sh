#!/usr/bin/env bash
# Takes the README screenshots with the built-in demo car.
#
#   tools/screenshots.sh de      German, into docs/de/
#   tools/screenshots.sh en      English, into docs/
#
# Uses a throwaway garage so your own one stays untouched.
set -euo pipefail

lang="${1:-de}"
here="$(cd "$(dirname "$0")/.." && pwd)"
app="$here/build/bin/Release/caruspex"
[ -x "$app" ] || app="$here/build/bin/Release/caruspex.exe"
[ -x "$app" ] || app="$here/build/bin/caruspex"
out="$here/docs"
[ "$lang" = "de" ] && out="$here/docs/de"
mkdir -p "$out"

export CARUSPEX_GARAGE="$(mktemp -d)/garage.json"
size="1440x920"

shot() {   # name page delay
    "$app" --language "$lang" --demo --page "$2" --shot "$out/screenshot-$1.png" \
        --shot-delay "$3" --size "$size"
}

shot overview start 7000 &
shot faults faults 7000 &
shot live live-run 16000 &
wait
shot health health-run 22000 &
shot adapter adapter-run 20000 &
"$app" --language "$lang" --page vehicles:WVWZZZ1JZ3W386752 --shot "$out/screenshot-vehicles.png" \
    --shot-delay 2500 --size "$size" &
wait
echo "Screenshots in $out"
