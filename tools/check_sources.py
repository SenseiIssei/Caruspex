"""Compares the data sources Caruspex is built from with what is recorded.

    python tools/check_sources.py            report as Markdown on stdout
    python tools/check_sources.py --update   also write the current state to data/sources.json

Checked:
  - the WMI list on Wikibooks: codes there that crates/caruspex-vehicles/src/vin.rs does not know
  - OBDb: number of vehicle repositories (github.com/OBDb, CC BY-SA 4.0)
  - opendbc (comma.ai, MIT) and model3dbc (MIT): newest commit

Exit code 0 when nothing changed, 10 when there is something to look at, so
the monthly workflow knows whether to open an issue. Needs network; a
GitHub token in GH_TOKEN avoids the API rate limit.
"""

from __future__ import annotations

import json
import os
import re
import sys
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STATE = os.path.join(ROOT, "data", "sources.json")
WMI_RAW = ("https://en.wikibooks.org/w/index.php?title=Vehicle_Identification_Numbers_(VIN_codes)/"
           "World_Manufacturer_Identifier_(WMI)&action=raw")


def fetch(url: str, api: bool = False) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "Caruspex source check"})
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    if api and token:
        request.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()


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


def newest_commit(repo: str) -> str:
    data = json.loads(fetch(f"https://api.github.com/repos/{repo}/commits/HEAD", api=True))
    return data["sha"]


def obdb_count() -> int:
    data = json.loads(fetch("https://api.github.com/search/repositories?q=org:OBDb", api=True))
    return int(data["total_count"])


def main() -> int:
    state = json.load(open(STATE, encoding="utf-8")) if os.path.exists(STATE) else {}
    current = dict(state)
    lines = ["## Datencheck der Quellen", ""]
    changed = False

    known = known_wmis()
    listed = wikibooks_wmis()
    # Codes covered by a two-character range entry count as known.
    # Only codes that appeared since the last check: the list has many
    # trailers, buses and special vehicles nobody diagnoses over OBD.
    seen_path = os.path.join(ROOT, "data", "wmi_seen.txt")
    seen = set(open(seen_path, encoding="utf-8").read().split()) if os.path.exists(seen_path) else set()
    missing = {c: m for c, m in listed.items() if c not in known and c[:2] not in known and c not in seen}
    current["wikibooks_wmi_codes"] = len(listed)
    if "--update" in sys.argv:
        with open(seen_path, "w", encoding="utf-8", newline="\n") as f:
            f.write("\n".join(sorted(seen | set(listed))) + "\n")
    if missing:
        changed = True
        lines.append(f"**Neue VIN-Kennungen auf Wikibooks seit dem letzten Check: {len(missing)}**")
        lines.append("")
        for code, make in sorted(missing.items())[:200]:
            lines.append(f"- `{code}` {make}")
        lines.append("")
    else:
        lines.append("VIN-Kennungen: nichts Neues.")

    count = obdb_count()
    current["obdb_repositories"] = count
    if count != state.get("obdb_repositories"):
        changed = True
        lines.append(f"**OBDb: {count} Fahrzeuge (vorher {state.get('obdb_repositories', '?')})**")
    for key, repo in (("opendbc_commit", "commaai/opendbc"), ("model3dbc_commit", "joshwardell/model3dbc")):
        sha = newest_commit(repo)
        current[key] = sha
        if sha != state.get(key):
            changed = True
            lines.append(f"**{repo}: neuer Stand {sha[:10]} (vorher {str(state.get(key, '?'))[:10]})**, "
                         f"https://github.com/{repo}/compare/{state.get(key, sha)}...{sha}")
    print("\n".join(lines))
    if "--update" in sys.argv:
        with open(STATE, "w", encoding="utf-8", newline="\n") as f:
            json.dump(current, f, indent=2, sort_keys=True)
            f.write("\n")
    return 10 if changed else 0


if __name__ == "__main__":
    sys.exit(main())
