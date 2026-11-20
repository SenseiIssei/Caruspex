"""Known vulnerabilities in the dependencies (the Audit workflow).

    py -I tools/audit.py                  RustSec for Cargo.lock and npm for app/
    py -I tools/audit.py --rust-json F --npm-json F   read saved reports instead

Runs `cargo audit --json` (RustSec advisory database, every crate of
Cargo.lock) and `npm audit --omit=dev --package-lock-only --json` (only the
packages that end up in the interface) and fails when either reports a
vulnerability that is not accepted in tools/audit_exceptions.toml.

RustSec warnings (unmaintained, unsound, yanked) are printed but do not fail
the run: they need a decision, not an emergency.

An exception is for a finding that does not affect Caruspex (code path not
used, platform not built) and always has a reason and an expiry date. An
expired exception fails the run, so it is looked at again; one that matches
nothing any more is reported so it can be removed.
"""

from __future__ import annotations

import datetime
import json
import os
import re
import shutil
import subprocess
import sys
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXCEPTIONS = os.path.join(ROOT, "tools", "audit_exceptions.toml")
GITHUB = os.environ.get("GITHUB_ACTIONS") == "true"


def note(kind: str, text: str) -> None:
    # In the CI the lines also become annotations on the run page.
    print(f"::{kind}::{text}" if GITHUB else f"{kind.upper()}: {text}")


def load_exceptions(today: datetime.date) -> tuple[list[dict], list[str]]:
    if not os.path.isfile(EXCEPTIONS):
        return [], []
    data = tomllib.load(open(EXCEPTIONS, "rb"))
    entries, problems = [], []
    for n, entry in enumerate(data.get("exception", []), 1):
        where = f"tools/audit_exceptions.toml, exception {n}"
        missing = [k for k in ("id", "package", "reason", "expires") if not entry.get(k)]
        if missing:
            problems.append(f"{where}: {', '.join(missing)} missing")
            continue
        if not isinstance(entry["expires"], datetime.date):
            problems.append(f"{where}: expires must be a date like 2027-03-31")
            continue
        if entry["expires"] < today:
            problems.append(f"{where} ({entry['id']}, {entry['package']}) expired on {entry['expires']}: check it again")
            continue
        if entry["expires"] > today + datetime.timedelta(days=366):
            problems.append(f"{where} ({entry['id']}): at most one year ahead")
            continue
        entries.append(entry)
    return entries, problems


def run_json(command: list[str], cwd: str) -> dict:
    executable = shutil.which(command[0])
    if executable is None:
        raise SystemExit(f"{command[0]} is not installed")
    result = subprocess.run([executable, *command[1:]], cwd=cwd, capture_output=True, text=True, encoding="utf-8")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        raise SystemExit(f"{' '.join(command)} gave no report:\n{result.stderr}")


def rust_findings(report: dict) -> tuple[list[dict], list[str]]:
    findings = []
    for item in report.get("vulnerabilities", {}).get("list", []):
        advisory = item["advisory"]
        findings.append({
            "ids": {advisory["id"], *advisory.get("aliases", [])},
            "package": item["package"]["name"],
            "text": f"{advisory['id']} in {item['package']['name']} {item['package']['version']}: {advisory['title']}"
                    f" (fixed in {', '.join(item.get('versions', {}).get('patched', [])) or 'no version yet'}) {advisory.get('url') or ''}".rstrip(),
        })
    warnings = []
    for kind, items in sorted(report.get("warnings", {}).items()):
        for item in items:
            advisory = item.get("advisory") or {}
            warnings.append(f"{kind}: {item['package']['name']} {item['package']['version']}"
                            f" {advisory.get('id', '')} {advisory.get('title', '')}".rstrip())
    return findings, warnings


def npm_findings(report: dict) -> list[dict]:
    findings = []
    for name, item in sorted(report.get("vulnerabilities", {}).items()):
        for via in item.get("via", []):
            if not isinstance(via, dict):
                continue  # only points at another package of the list
            ghsa = re.search(r"GHSA(-[23456789cfghjmpqrvwx]{4}){3}", via.get("url", ""))
            ident = ghsa.group(0) if ghsa else str(via.get("source"))
            findings.append({
                "ids": {ident, str(via.get("source"))},
                "package": name,
                "text": f"{ident} in {name} {item.get('range', '')} ({via.get('severity')}): {via.get('title')} {via.get('url', '')}".rstrip(),
            })
    return findings


def main() -> int:
    arguments = sys.argv[1:]
    today = datetime.date.today()
    exceptions, problems = load_exceptions(today)

    if "--rust-json" in arguments:
        rust = json.load(open(arguments[arguments.index("--rust-json") + 1], encoding="utf-8"))
    else:
        rust = run_json(["cargo", "audit", "--json"], ROOT)
    if "--npm-json" in arguments:
        npm = json.load(open(arguments[arguments.index("--npm-json") + 1], encoding="utf-8"))
    else:
        npm = run_json(["npm", "audit", "--omit=dev", "--package-lock-only", "--json"], os.path.join(ROOT, "app"))
    if "vulnerabilities" not in rust or "vulnerabilities" not in npm:
        print("an audit report has no 'vulnerabilities' part (no network?)", file=sys.stderr)
        return 1

    found, warnings = rust_findings(rust)
    found += npm_findings(npm)
    used: set[int] = set()
    failing = []
    for finding in found:
        match = next((i for i, e in enumerate(exceptions)
                      if e["id"] in finding["ids"] and e["package"] == finding["package"]), None)
        if match is None:
            failing.append(finding["text"])
        else:
            used.add(match)
            print(f"accepted until {exceptions[match]['expires']}: {finding['text']} ({exceptions[match]['reason']})")
    for i, entry in enumerate(exceptions):
        if i not in used:
            note("notice", f"exception {entry['id']} ({entry['package']}) matches nothing any more, remove it")
    for warning in warnings:
        note("warning", f"RustSec {warning}")

    db = rust.get("database", {})
    print(f"RustSec database {db.get('last-commit', '?')[:12]} ({db.get('advisory-count', '?')} advisories), "
          f"{rust.get('lockfile', {}).get('dependency-count', '?')} crates; "
          f"npm production packages: {npm.get('metadata', {}).get('dependencies', {}).get('prod', '?')}")
    for text in problems + failing:
        note("error", text)
    if failing:
        print(f"\n{len(failing)} vulnerability finding(s). Update the dependency (cargo update -p NAME, npm update NAME),"
              " or, if Caruspex is not affected, add an exception with reason and expiry to tools/audit_exceptions.toml"
              " (see SECURITY.md).", file=sys.stderr)
    if problems or failing:
        return 1
    print("no open vulnerabilities")
    return 0


if __name__ == "__main__":
    sys.exit(main())
