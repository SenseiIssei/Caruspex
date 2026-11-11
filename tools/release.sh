#!/usr/bin/env bash
# Prepares a release on its own branch, without pushing anything:
#
#   bash tools/release.sh 0.9.1
#
# 1. branch release-<version> from the current main
# 2. version in Cargo.toml (workspace) and app/package.json
# 3. changelog/<version>.md with a German and an English part (written as a
#    skeleton if it does not exist yet; the script stops until it is filled)
# 4. checks, tests, packages and smoke test of this system
# 5. fresh README screenshots in both languages
#
# Afterwards, by hand and only after an explicit go: push the branch, open
# the pull request, merge when the CI is green, tag v<version> and push the
# tag. The CI builds the packages and publishes the release.
set -euo pipefail

cd "$(dirname "$0")/.."
version=${1:?usage: tools/release.sh <version, e.g. 0.9.1>}
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "not a version: $version" >&2; exit 1; }

# Only the change list of this version may already be there (written first,
# then this script run again).
if [ -n "$(git status --porcelain | grep -v " changelog/$version.md\$")" ]; then
    echo "The working tree is not clean; commit or stash first." >&2
    exit 1
fi
# On the release branch already (second run after writing the change list,
# or a fix to this script on the branch): keep it, otherwise start it fresh.
if [ "$(git branch --show-current)" != "release-$version" ]; then
    git fetch -q origin
    git checkout -q -B "release-$version" origin/main
fi

sed -i '/^\[workspace.package\]/,/^\[/s/^version = "[0-9.]*"/version = "'"$version"'"/' Cargo.toml
sed -i '0,/"version": "[0-9.]*"/s//"version": "'"$version"'"/' app/package.json
grep -q "^version = \"$version\"" Cargo.toml || { echo "version not set in Cargo.toml" >&2; exit 1; }

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

# The first Python that really runs (on Windows "python3" can be a stub that
# only points to the Microsoft Store).
python_bin=""
for candidate in python3 python "py -I"; do
    if $candidate -c "pass" >/dev/null 2>&1; then python_bin=$candidate; break; fi
done
[ -n "$python_bin" ] || { echo "no working Python found" >&2; exit 1; }
$python_bin tools/i18n_rust.py --strict
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo update -w   # the new version into Cargo.lock
REQUIRE_INSTALLER=1 bash tools/package.sh
# On Windows the smoke test window goes next to the main screen.
SMOKE_WINDOW_POS=${SMOKE_WINDOW_POS:--1880,40} bash tools/smoke_test.sh smoke
rm -rf package dist smoke

# Fresh README images, so they show this version (taken here with the real
# fonts, not on a headless CI machine).
bash tools/screenshots.sh de
bash tools/screenshots.sh en

git add Cargo.toml Cargo.lock app/package.json app/package-lock.json "$notes" docs/*.png docs/de/*.png
git commit -q -m "Version $version mit Aenderungsliste auf Deutsch und Englisch"
echo
echo "Release $version is ready on branch release-$version. Next, after a go:"
echo "  git push -u origin release-$version   and open the pull request"
echo "  after merging: git tag -a v$version origin/main -m \"Caruspex $version\" && git push origin v$version"
