#!/usr/bin/env bash
# Prepares a release on its own branch, without pushing anything:
#
#   bash tools/release.sh 0.9.0
#
# 1. branch release-<version> from the current main
# 2. version in CMakeLists.txt
# 3. changelog/<version>.md with a German and an English part (opened for
#    writing if it does not exist yet; the script stops until it is filled)
# 4. translation check, build, tests, packages and smoke test of this system
#
# Afterwards, by hand and only after an explicit go: push the branch, open
# the pull request, merge when the CI is green, tag v<version> and push the
# tag. The CI builds the packages and publishes the release.
set -euo pipefail

cd "$(dirname "$0")/.."
version=${1:?usage: tools/release.sh <version, e.g. 0.9.0>}
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "not a version: $version" >&2; exit 1; }

if [ -n "$(git status --porcelain)" ]; then
    echo "The working tree is not clean; commit or stash first." >&2
    exit 1
fi
git fetch -q origin
git checkout -q -B "release-$version" origin/main

sed -i "s/^project(Caruspex VERSION [0-9.]* LANGUAGES CXX)/project(Caruspex VERSION $version LANGUAGES CXX)/" CMakeLists.txt
grep -q "VERSION $version " CMakeLists.txt || { echo "version not set in CMakeLists.txt" >&2; exit 1; }

notes="changelog/$version.md"
if [ ! -f "$notes" ]; then
    cat > "$notes" <<EOF
## Deutsch

(Was ist neu, in ganzen Sätzen für Halter, keine Namen, keine langen Gedankenstriche.)

## English

(What is new, in plain sentences for car owners, no names, no long dashes.)
EOF
    echo "Wrote $notes. Fill in both parts, then run this script again." >&2
    exit 2
fi
if grep -q "^(Was ist neu\|^(What is new" "$notes"; then
    echo "$notes still has the placeholders." >&2
    exit 2
fi
if LC_ALL=C grep -q $'\xe2\x80\x94\|\xe2\x80\x93' "$notes"; then   # U+2014, U+2013
    echo "$notes contains a long dash." >&2
    exit 2
fi

cmake_bin=$(command -v cmake || echo "/c/Program Files/Microsoft Visual Studio/18/Community/Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin/cmake.exe")
python_bin=$(command -v python3 || command -v python || echo "py -I")

$python_bin tools/gen_i18n.py --check --strict
"$cmake_bin" build >/dev/null
"$cmake_bin" --build build --config Release
if [ -x build/bin/Release/caruspex_tests.exe ]; then build/bin/Release/caruspex_tests.exe; else build/bin/caruspex_tests; fi
REQUIRE_INSTALLER=1 bash tools/package.sh build
bash tools/smoke_test.sh smoke
rm -rf package dist smoke

# Fresh README images, so they show this version (taken here with the real
# fonts, not on a headless CI machine).
[ -d /c/Qt/6.8.3/msvc2022_64/bin ] && export PATH="/c/Qt/6.8.3/msvc2022_64/bin:$PATH"
bash tools/screenshots.sh de
bash tools/screenshots.sh en

git add CMakeLists.txt "$notes" docs/*.png docs/de/*.png
git commit -q -m "Version $version mit Aenderungsliste auf Deutsch und Englisch"
echo
echo "Release $version is ready on branch release-$version. Next, after a go:"
echo "  git push -u origin release-$version   and open the pull request"
echo "  after merging: git tag -a v$version origin/main -m \"Caruspex $version\" && git push origin v$version"
