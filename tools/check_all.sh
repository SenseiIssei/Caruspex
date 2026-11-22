#!/usr/bin/env bash
# Every check a pull request must pass (CONTRIBUTING.md, "What a pull
# request must pass"), in one command, offline and without secrets:
#
#   bash tools/check_all.sh              all checks, then a summary
#   bash tools/check_all.sh --android    plus clippy of the app for Android
#   bash tools/check_all.sh --no-npm-ci  reuse app/node_modules (faster)
#   bash tools/check_all.sh --list       print the steps and quit
#
# It runs every step even after one failed and lists all failures at the
# end, so one run shows everything that is left to fix. The exit code is 0
# only when every step passed.
#
# The generators must not change anything: their output is compared with
# what was in the working tree before the step, so a generated file that is
# already regenerated but not committed yet does not count as a failure (the
# CI compares with the commit).
#
# CARGO_TARGET_DIR is respected. The Android step needs ANDROID_HOME with an
# NDK and `rustup target add aarch64-linux-android`; it builds into
# ANDROID_TARGET_DIR (default: <target dir>/android) so the host build is not
# rebuilt with the Android compiler settings.
set -uo pipefail

cd "$(dirname "$0")/.."

android=0
npm_ci=1
list=0
for arg in "$@"; do
    case "$arg" in
        --android) android=1 ;;
        --no-npm-ci) npm_ci=0 ;;
        --list) list=1 ;;
        -h|--help) sed -n '2,21p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown option: $arg (see --help)" >&2; exit 2 ;;
    esac
done

# The first Python that really runs (on Windows "python3" can be a stub that
# only points to the Microsoft Store), as in tools/release.sh.
python_bin=""
for candidate in python3 python "py -I"; do
    if $candidate -c "pass" >/dev/null 2>&1; then python_bin=$candidate; break; fi
done

# What the generators write; the CI compares the same paths.
generated=(
    'crates/*/src/generated/*'
    crates/caruspex-record/tests/fixtures
    crates/caruspex-vehicles/tests/fixtures
    app/src/api/mock/data
)

# One line per file with the hash of its content as git would store it, so a
# generator that writes LF over a CRLF checkout (core.autocrlf) changes
# nothing here, unlike in `git status`.
generated_state() {
    local f
    git ls-files -z --cached --others --exclude-standard -- "${generated[@]}" |
        while IFS= read -r -d '' f; do
            if [ -f "$f" ]; then
                printf '%s %s
' "$(git hash-object -- "$f")" "$f"
            else
                printf 'missing %s
' "$f"
            fi
        done
}

step_generators() {
    local before after
    before=$(generated_state)
    $python_bin tools/gen_dtc.py &&
        $python_bin tools/gen_vehicles.py &&
        $python_bin tools/gen_ecus.py &&
        $python_bin tools/gen_pids.py &&
        $python_bin tools/gen_engines.py &&
        $python_bin tools/gen_wmi.py &&
        $python_bin tools/gen_obdb.py --fixtures &&
        $python_bin tools/gen_dimensions.py &&
        $python_bin tools/gen_dimensions.py --fixtures &&
        $python_bin app/scripts/extract-catalog.py || return 1
    after=$(generated_state)
    if [ "$before" != "$after" ]; then
        echo "The generators changed these files; check and commit them:" >&2
        diff <(echo "$before") <(echo "$after") | sed -n 's/^> [^ ]* /  /p' >&2
        return 1
    fi
}

step_npm_ci() {
    if [ "$npm_ci" = 0 ] && [ -d app/node_modules ]; then
        echo "app/node_modules kept (--no-npm-ci)"
        return 0
    fi
    npm --prefix app ci --no-audit --no-fund
}

step_android() {
    local home=${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}
    if [ -z "$home" ] || [ ! -d "$home/ndk" ]; then
        echo "ANDROID_HOME with an NDK is needed for this step" >&2
        return 1
    fi
    if ! rustup target list --installed 2>/dev/null | grep -qx aarch64-linux-android; then
        echo "rustup target add aarch64-linux-android is needed for this step" >&2
        return 1
    fi
    local ndk host bin ext=""
    ndk=$(ls -d "$home"/ndk/*/ | sort -V | tail -n 1)
    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*) host=windows-x86_64; ext=.cmd ;;
        Darwin) host=darwin-x86_64 ;;
        *) host=linux-x86_64 ;;
    esac
    bin="${ndk%/}/toolchains/llvm/prebuilt/$host/bin"
    local ar="$bin/llvm-ar"
    [ -f "$ar.exe" ] && ar="$ar.exe"
    CC_aarch64_linux_android="$bin/aarch64-linux-android24-clang$ext" \
    CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$bin/aarch64-linux-android24-clang$ext" \
    AR_aarch64_linux_android="$ar" \
    CARGO_TARGET_DIR="${ANDROID_TARGET_DIR:-${CARGO_TARGET_DIR:-target}/android}" \
        cargo clippy -p caruspex-app --lib --target aarch64-linux-android -- -D warnings
}

# name | command. Same order as the Checks job of build.yml; the interface is
# built before clippy and the tests because the app has it built in.
steps=(
    "Formatting|cargo fmt --all --check"
    "Generated tables are up to date|step_generators"
    "Translations are complete|$python_bin tools/i18n_rust.py --strict"
    "README language bar is current|$python_bin tools/readme_languages.py --check"
    "Documentation website builds|$python_bin docs/site/build.py --check --out ${CARGO_TARGET_DIR:-target}/docs-site"
    "Package manager templates|$python_bin tools/dist_manifests.py --check-templates"
    "Interface packages|step_npm_ci"
    "Third party notices are current|$python_bin tools/gen_notices.py --check"
    "Interface build (strict texts)|env I18N_STRICT=1 npm --prefix app run build"
    "Interface tests|env I18N_STRICT=1 npm --prefix app test"
    "Interface size budget|$python_bin tools/bundle_budget.py"
    "SBOM|$python_bin tools/sbom.py --check"
    "Lints|cargo clippy --workspace --all-targets -- -D warnings"
    "Tests|cargo test --workspace"
)
[ "$android" = 1 ] && steps+=("Lints for Android|step_android")

if [ "$list" = 1 ]; then
    for s in "${steps[@]}"; do echo "${s%%|*}: ${s#*|}"; done
    exit 0
fi
[ -n "$python_bin" ] || { echo "no working Python found" >&2; exit 1; }

results=()
failed=0
total_start=$(date +%s)
for s in "${steps[@]}"; do
    name=${s%%|*}
    command=${s#*|}
    echo
    echo "==> $name"
    start=$(date +%s)
    if eval "$command"; then status=ok; else status=FAILED; failed=$((failed + 1)); fi
    took=$(( $(date +%s) - start ))
    results+=("$(printf '%-6s %5ss  %s' "$status" "$took" "$name")")
done

echo
echo "Summary ($(( $(date +%s) - total_start )) s):"
printf '  %s\n' "${results[@]}"
if [ "$failed" -gt 0 ]; then
    echo "$failed step(s) failed." >&2
    exit 1
fi
echo "All checks passed."
