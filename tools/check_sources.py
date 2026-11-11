"""Compares the data sources Caruspex is built from with what is recorded.

    python tools/check_sources.py            report as Markdown on stdout
    python tools/check_sources.py --update   also write the current state to data/sources.json,
                                             data/wmi_seen.txt and data/obdb_seen.txt

Checked:
  - the WMI list on Wikibooks: codes there that crates/caruspex-vehicles/src/vin.rs does not know
  - OBDb (github.com/OBDb, CC BY-SA 4.0): vehicle repositories that are new since the last
    check, by name, and those changed since the packs in data/obdb were made
    (tools/gen_obdb.py), by name
  - Wikidata (CC0): the dimensions per generation, asked again with the queries of
    tools/gen_dimensions.py and matched to today's catalog, against
    data/wikidata/dimensions.csv: generations with new values, changed values, values
    gone, and the rows of data/dimensions_manual.csv that Wikidata now has as well
  - opendbc (comma.ai, MIT) and model3dbc (MIT): newest commit

Exit code 0 when nothing changed, 10 when there is something to look at, so
the monthly workflow knows whether to open an issue. Needs network; a
GitHub token in GH_TOKEN avoids the API rate limit. The Wikidata queries take
a few minutes; their progress goes to stderr.
"""

from __future__ import annotations

import importlib.util
import json
import os
import re
import sys
import tempfile
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STATE = os.path.join(ROOT, "data", "sources.json")
OBDB_SEEN = os.path.join(ROOT, "data", "obdb_seen.txt")
WMI_RAW = ("https://en.wikibooks.org/w/index.php?title=Vehicle_Identification_Numbers_(VIN_codes)/"
           "World_Manufacturer_Identifier_(WMI)&action=raw")
# Longest list per finding in the issue; the rest is counted.
LIST_MAX = 150
ISSUE_MAX = 60000


def fetch(url: str, api: bool = False) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "Caruspex source check"})
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    if api and token:
        request.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()


def tool(name: str):
    """One of the generators next to this file, loaded by its path."""
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), name + ".py")
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def listed(lines: list[str], items: list[str]) -> None:
    for item in items[:LIST_MAX]:
        lines.append(f"- {item}")
    if len(items) > LIST_MAX:
        lines.append(f"- ... und {len(items) - LIST_MAX} weitere")
    lines.append("")


# --- VIN codes ---------------------------------------------------------------

def known_wmis() -> set[str]:
    # One table entry per line: w("WVW", "Volkswagen AG", ...
    path = os.path.join(ROOT, "crates", "caruspex-vehicles", "src", "vin.rs")
    text = open(path, encoding="utf-8").read()
    return set(re.findall(r'^\s*w\("([A-Z0-9]{2,3})",', text, re.MULTILINE))


def wikibooks_wmis() -> dict[str, str]:
    raw = fetch(WMI_RAW).decode("utf-8", errors="replace")
    out: dict[str, str] = {}
    for line in raw.splitlines():
        m = re.match(r"^\|\s*([A-HJ-NPR-Z0-9]{3})\s*\|\|\s*(.+)$", line)
        if m:
            make = re.sub(r"<ref.*?(</ref>|/>)|\[\[(?:[^|\]]*\|)?([^\]]*)\]\]|'''?", lambda g: g.group(2) or "", m.group(2))
            out.setdefault(m.group(1), make.strip()[:90])
    return out


def check_wmi(current: dict, lines: list[str]) -> bool:
    known = known_wmis()
    listed_codes = wikibooks_wmis()
    # Codes covered by a two-character range entry count as known.
    # Only codes that appeared since the last check: the list has many
    # trailers, buses and special vehicles nobody diagnoses over OBD.
    seen_path = os.path.join(ROOT, "data", "wmi_seen.txt")
    seen = set(open(seen_path, encoding="utf-8").read().split()) if os.path.exists(seen_path) else set()
    missing = {c: m for c, m in listed_codes.items() if c not in known and c[:2] not in known and c not in seen}
    current["wikibooks_wmi_codes"] = len(listed_codes)
    if "--update" in sys.argv:
        with open(seen_path, "w", encoding="utf-8", newline="\n") as f:
            f.write("\n".join(sorted(seen | set(listed_codes))) + "\n")
    if missing:
        lines.append(f"**Neue VIN-Kennungen auf Wikibooks seit dem letzten Check: {len(missing)}**")
        lines.append("")
        for code, make in sorted(missing.items())[:200]:
            lines.append(f"- `{code}` {make}")
        lines.append("")
        return True
    lines.append("VIN-Kennungen: nichts Neues.")
    lines.append("")
    return False


# --- OBDb --------------------------------------------------------------------

def obdb_import_date() -> str:
    """The date of the OBDb data in data/obdb (written by tools/gen_obdb.py)."""
    path = os.path.join(ROOT, "data", "obdb", "ATTRIBUTION.md")
    if not os.path.exists(path):
        return ""
    m = re.search(r"as of (\d{4}-\d{2}-\d{2})", open(path, encoding="utf-8").read())
    return m.group(1) if m else ""


def obdb_used() -> set[str]:
    """The repositories the packs in data/obdb were made from (ATTRIBUTION.md)."""
    path = os.path.join(ROOT, "data", "obdb", "ATTRIBUTION.md")
    if not os.path.exists(path):
        return set()
    text = open(path, encoding="utf-8").read()
    return set(re.findall(r"^- ([A-Za-z0-9][A-Za-z0-9.-]*)$", text, re.MULTILINE))


def check_obdb(state: dict, current: dict, lines: list[str]) -> bool:
    """New and changed OBDb vehicle repositories, by name."""
    obdb = tool("gen_obdb")
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN") or ""
    repos = [r for r in obdb.list_repos(token) if obdb.is_vehicle_repo(r["name"]) and not r.get("archived")]
    names = {r["name"] for r in repos}
    # Without a list from an earlier check, the repositories of the import count as seen.
    seen = set(open(OBDB_SEEN, encoding="utf-8").read().split()) if os.path.exists(OBDB_SEEN) else obdb_used()
    if "--update" in sys.argv:
        with open(OBDB_SEEN, "w", encoding="utf-8", newline="\n") as f:
            f.write("\n".join(sorted(seen | names, key=str.lower)) + "\n")
    link = lambda n: f"[{n}](https://github.com/OBDb/{n})"
    changed = False
    current["obdb_repositories"] = len(names)
    new = sorted(names - seen, key=str.lower)
    if new:
        changed = True
        lines.append(f"**OBDb: {len(new)} neue Fahrzeuge seit dem letzten Check** "
                     f"({len(names)} insgesamt, vorher {state.get('obdb_repositories', '?')})")
        lines.append("")
        listed(lines, [link(n) for n in new])
    else:
        lines.append(f"OBDb: keine neuen Fahrzeuge ({len(names)}).")
        lines.append("")
    imported = obdb_import_date()
    if imported:
        newer = sorted((r["name"] for r in repos if r["pushed_at"][:10] > imported and r["name"] not in new),
                       key=str.lower)
        current["obdb_changed_since_import"] = len(newer)
        if newer and len(newer) != state.get("obdb_changed_since_import"):
            changed = True
            lines.append(f"**OBDb: {len(newer)} Fahrzeuge seit dem Import ({imported}) geändert.** "
                         "Pakete neu erzeugen: `py -I tools/gen_obdb.py --fetch`, dann "
                         "`py -I tools/gen_obdb.py`, Tests laufen lassen, `data/obdb` committen.")
            lines.append("")
            listed(lines, [link(n) for n in newer])
    return changed


# --- Wikidata dimensions -----------------------------------------------------

FIELDS_DE = {"length": "Länge", "width": "Breite", "height": "Höhe", "wheelbase": "Radstand", "mass": "Gewicht"}


def compare_dimensions(now: dict, before: dict, manual: dict) -> tuple[list[str], list[str], list[str], list[str]]:
    """New, changed and gone generations of the Wikidata rows (keyed by brand,
    model, generation), and the hand-listed ones Wikidata now has too."""
    fields = list(FIELDS_DE)
    unit = lambda f, v: f"{v} kg" if f == "mass" else f"{v} mm"
    name = lambda k: " ".join(k)
    new, changes, also = [], [], []
    for k, r in sorted(now.items()):
        old = before.get(k)
        if old is None:
            item = r["item"].split("|")[0]
            values = ", ".join(f"{FIELDS_DE[f]} {unit(f, r[f])}" for f in fields if r[f])
            new.append(f"{name(k)}: {values} ([{item}](https://www.wikidata.org/wiki/{item}))")
        else:
            diff = [f"{FIELDS_DE[f]} {old[f] or '-'} -> {r[f] or '-'}" for f in fields if old[f] != r[f]]
            if diff:
                changes.append(f"{name(k)}: " + ", ".join(diff))
        hand = manual.get(k)
        if hand:
            covered = [f for f in hand["values"] if hand["values"][f] and r.get(f) and not (old and old.get(f))]
            if covered:
                also.append(f"{name(k)}: " + ", ".join(
                    f"{FIELDS_DE[f]} {r[f]} (Hersteller {hand['values'][f]})" for f in covered))
    gone = [name(k) for k in sorted(before) if k not in now]
    return new, changes, gone, also


def check_wikidata(current: dict, lines: list[str]) -> bool:
    """The Wikidata dimensions of today against data/wikidata/dimensions.csv."""
    dims = tool("gen_dimensions")
    with tempfile.TemporaryDirectory() as folder:
        dims.fetch_answers(folder, log=lambda text: print(text, file=sys.stderr))
        rows = dims.current_rows(folder)
    key = lambda r: (r["brand"], r["model"], r["generation"])
    now = {key(r): r for r in rows}
    before = {key(r): r for r in dims.read_csv(dims.TABLE)}
    manual = dims.read_manual(dims.MANUAL, {g.key for g in dims.load_catalog()})
    new, changes, gone, also = compare_dimensions(now, before, manual)
    current["wikidata_dimension_rows"] = len(now)
    if new or changes or gone:
        lines.append(f"**Wikidata-Maße: {len(new)} Generationen neu, {len(changes)} geändert, {len(gone)} ohne Werte** "
                     f"({len(now)} mit Werten, in `data/wikidata/dimensions.csv` {len(before)}). Übernehmen: "
                     "`py -I tools/gen_dimensions.py --fetch`, dann `--match`, den Diff von "
                     "`data/wikidata/dimensions.csv` ansehen, Tests laufen lassen, committen.")
        lines.append("")
        if new:
            lines.append("Neu:")
            listed(lines, new)
        if changes:
            lines.append("Geändert:")
            listed(lines, changes)
        if gone:
            lines.append("Nicht mehr zugeordnet (Eintrag in Wikidata oder `data/wikidata/aliases.csv` prüfen):")
            listed(lines, gone)
    else:
        lines.append(f"Wikidata-Maße: nichts Neues ({len(now)} Generationen mit Werten).")
        lines.append("")
    if also:
        lines.append("Aus `data/dimensions_manual.csv` hat Wikidata jetzt auch Werte (die Herstellerwerte "
                     "gehen weiter vor; Abweichungen prüfen):")
        lines.append("")
        listed(lines, also)
    return bool(new or changes or gone)


# --- Commits of the CAN databases --------------------------------------------

def newest_commit(repo: str) -> str:
    data = json.loads(fetch(f"https://api.github.com/repos/{repo}/commits/HEAD", api=True))
    return data["sha"]


def check_dbc(state: dict, current: dict, lines: list[str]) -> bool:
    changed = False
    for key, repo in (("opendbc_commit", "commaai/opendbc"), ("model3dbc_commit", "joshwardell/model3dbc")):
        sha = newest_commit(repo)
        current[key] = sha
        if sha != state.get(key):
            changed = True
            lines.append(f"**{repo}: neuer Stand {sha[:10]} (vorher {str(state.get(key, '?'))[:10]})**, "
                         f"https://github.com/{repo}/compare/{state.get(key, sha)}...{sha}")
            lines.append("")
    return changed


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    state = json.load(open(STATE, encoding="utf-8")) if os.path.exists(STATE) else {}
    current = dict(state)
    lines = ["## Datencheck der Quellen", ""]
    changed = check_wmi(current, lines)
    changed = check_obdb(state, current, lines) or changed
    try:
        changed = check_wikidata(current, lines) or changed
    except Exception as e:  # the query service is busy at times; the other checks still count
        lines.append(f"Wikidata-Maße: Abfrage fehlgeschlagen ({type(e).__name__}: {e}), beim nächsten Lauf wieder.")
        lines.append("")
    changed = check_dbc(state, current, lines) or changed
    report = "\n".join(lines).rstrip()
    # An issue holds 65536 characters; the workflow adds a line.
    if len(report) > ISSUE_MAX:
        print(report, file=sys.stderr)
        report = report[:ISSUE_MAX].rsplit("\n", 1)[0] + "\n\n(gekürzt, der ganze Bericht steht im Log des Laufs)"
    print(report)
    if "--update" in sys.argv:
        with open(STATE, "w", encoding="utf-8", newline="\n") as f:
            json.dump(current, f, indent=2, sort_keys=True)
            f.write("\n")
    return 10 if changed else 0


if __name__ == "__main__":
    sys.exit(main())
