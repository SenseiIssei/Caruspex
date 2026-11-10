"""Generates src/data/PidPacks.gen.cpp from data/pids/*.csv.

Every CSV is one pack of custom values (manufacturer values, mode 22 and
mode 21) in the Torque Pro format, extended by three columns:

    Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header,Response,Name DE,ShortName DE

    Name, ShortName   English display name and short name
    ModeAndPID        request in hex, e.g. 220101 or 2101 (0x prefix allowed),
                      or CAN plus an 11-bit id (CAN132) for a value the car
                      broadcasts by itself; Equation is then a DBC signal
                      like DBC:0|16@1+(0.01,0) (see src/core/CanSignals.h)
    Equation          Torque equation over the data bytes A, B, ... (see
                      src/core/CustomPids.h for what is supported)
    Min/Max Value     sensible gauge range
    Units             unit, the same in both languages
    Header            request id in hex (7E4, 79B), empty = functional OBD
    Response          response id in hex, empty = header + 8
    Name DE, ShortName DE   the German names

The pack itself is described by comment lines at the top:

    #@ id: hyundai-kona-niro-ev
    #@ title_de: Hyundai Kona Elektro und Kia e-Niro (2018 bis 2022)
    #@ title_en: Hyundai Kona Electric and Kia e-Niro (2018 to 2022)
    #@ category: hybrid        (hybrid or custom)

Every value needs a source: a comment line "# Source: ..." applies to all
values below it until the next one. Values without a reliable public source
do not go in; a wrong formula for a battery value is worse than none.
Other lines starting with '#' are free comments.

The demo pack (id caruspex-demo) comes first, the others in file name order.

    py -I tools/gen_pids.py
"""

from __future__ import annotations

import csv
import io
import os
import re
import sys

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(HERE, "data", "pids")
TARGET = os.path.join(HERE, "src", "data", "PidPacks.gen.cpp")

COLUMNS = ["Name", "ShortName", "ModeAndPID", "Equation", "Min Value", "Max Value", "Units",
           "Header", "Response", "Name DE", "ShortName DE"]

CATEGORIES = {"hybrid": "HybridEv", "custom": "Custom"}
META_KEYS = ("id", "title_de", "title_en", "category")
DEMO = "caruspex-demo"

HEX = re.compile(r"^[0-9A-F]+$")
PACK_ID = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")


class DataError(Exception):
    pass


def check_text(text: str, where: str) -> None:
    for bad in (chr(0x2014), chr(0x2013)):  # em dash, en dash
        if bad in text:
            raise DataError(f"{where}: no long dashes in user-visible text")


MONITOR = re.compile(r"^CAN[0-9A-F]{3}$")
DBC = re.compile(r"^DBC:\d+\|\d+@[01][+-]\(-?[0-9.]+(e-?\d+)?,-?[0-9.]+(e-?\d+)?\)"
                 r"( ?m\d+@\d+\|\d+(@[01])?)?$", re.IGNORECASE)


def parse_request(text: str) -> str:
    value = text.upper()
    if MONITOR.match(value):
        if int(value[3:], 16) > 0x7FF:
            raise DataError(f"ModeAndPID {value}: the id does not fit into 11 bits")
        return value
    if value.startswith("0X"):
        value = value[2:]
    if not HEX.match(value) or len(value) % 2 or not 4 <= len(value) <= 14:
        raise DataError(f"ModeAndPID '{text}' is not a hex request like 220101")
    return value


def parse_id(text: str, what: str) -> str:
    if not text:
        return ""
    value = text.upper()
    if not HEX.match(value) or len(value) not in (3, 8):
        raise DataError(f"{what} '{text}' is not a CAN id like 7E4 or 18DA44F1")
    if len(value) == 3 and int(value, 16) > 0x7FF:
        raise DataError(f"{what} {value} does not fit into 11 bits")
    return value


def parse_float(text: str, what: str) -> float:
    try:
        return float(text)
    except ValueError:
        raise DataError(f"{what} '{text}' is not a number") from None


def cpp(text: str) -> str:
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def number(value: float) -> str:
    text = repr(float(value))
    return text if ("." in text or "e" in text) else text + ".0"


def load_pack(path: str) -> dict:
    name = os.path.basename(path)
    meta: dict[str, str] = {}
    rows: list[dict] = []
    source = ""
    header_seen = False
    with io.open(path, encoding="utf-8-sig", newline="") as f:
        text = f.read()
    # Comment lines are taken as they are, the rest goes through the CSV
    # reader one line at a time (pack values never span lines).
    for lineno, line in enumerate(text.splitlines(), start=1):
        line = line.strip()
        if not line:
            continue
        if line.startswith("#"):
            if line.startswith("#@"):
                key, _, value = line[2:].partition(":")
                key, value = key.strip(), value.strip()
                if key not in META_KEYS:
                    raise DataError(f"{name}:{lineno}: unknown pack key '{key}'")
                meta[key] = value
            elif line[1:].strip().lower().startswith("source:"):
                source = line[1:].strip()[len("source:"):].strip()
            continue
        cells = [c.strip() for c in next(csv.reader([line]))]
        where = f"{name}:{lineno}"
        if not header_seen:
            if cells != COLUMNS:
                raise DataError(f"{where}: header must be {','.join(COLUMNS)}")
            header_seen = True
            continue
        if len(cells) != len(COLUMNS):
            raise DataError(f"{where}: expected {len(COLUMNS)} columns, got {len(cells)}")
        r = dict(zip(COLUMNS, cells))
        try:
            if not source:
                raise DataError("no '# Source:' line above this value")
            for col in ("Name", "ShortName", "Name DE", "ShortName DE", "Equation"):
                if not r[col]:
                    raise DataError(f"{col} is empty")
            for col in ("Name", "ShortName", "Name DE", "ShortName DE", "Units"):
                check_text(r[col], col)
            check_text(source, "source")
            header = parse_id(r["Header"], "Header")
            response = parse_id(r["Response"], "Response")
            if response and not header:
                raise DataError("Response without Header")
            if header and response and len(header) != len(response):
                raise DataError("Header and Response differ in width")
            if header == "7DF":
                raise DataError("7DF is the functional address; leave Header empty instead")
            if MONITOR.match(r["ModeAndPID"].upper()):
                if header or response:
                    raise DataError("a broadcast value (CANxxx) has no Header or Response")
                if not DBC.match(r["Equation"]):
                    raise DataError(f"Equation '{r['Equation']}' is not a DBC signal like DBC:0|16@1+(0.01,0)")
            rows.append({
                "en": r["Name"], "short_en": r["ShortName"], "de": r["Name DE"],
                "short_de": r["ShortName DE"], "request": parse_request(r["ModeAndPID"]),
                "equation": r["Equation"], "min": parse_float(r["Min Value"], "Min Value"),
                "max": parse_float(r["Max Value"], "Max Value"), "unit": r["Units"],
                "header": header, "response": response, "source": source,
            })
        except DataError as e:
            raise DataError(f"{where}: {e}") from None
    for key in META_KEYS:
        if not meta.get(key):
            raise DataError(f"{name}: missing '#@ {key}:' line")
    if not PACK_ID.match(meta["id"]):
        raise DataError(f"{name}: pack id '{meta['id']}' must be lower case words joined by '-'")
    if meta["category"] not in CATEGORIES:
        raise DataError(f"{name}: category must be one of {', '.join(CATEGORIES)}")
    check_text(meta["title_de"], f"{name}: title_de")
    check_text(meta["title_en"], f"{name}: title_en")
    if not rows:
        raise DataError(f"{name}: no values")
    for lang in ("en", "de"):
        names = [r[lang] for r in rows]
        dupes = sorted({n for n in names if names.count(n) > 1})
        if dupes:
            raise DataError(f"{name}: names used twice: {', '.join(dupes)}")
    return {"id": meta["id"], "title_de": meta["title_de"], "title_en": meta["title_en"],
            "category": CATEGORIES[meta["category"]], "rows": rows, "file": name}


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    try:
        files = sorted(n for n in os.listdir(DATA) if n.endswith(".csv"))
        if not files:
            raise DataError(f"no CSV files in {DATA}")
        packs = [load_pack(os.path.join(DATA, n)) for n in files]
        ids = [p["id"] for p in packs]
        if len(set(ids)) != len(ids):
            raise DataError("a pack id is used twice")
    except DataError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    packs.sort(key=lambda p: (p["id"] != DEMO, p["file"]))
    total = sum(len(p["rows"]) for p in packs)

    lines = [
        "// Generated by tools/gen_pids.py from data/pids/*.csv. Do not edit by hand.",
        f"// {len(packs)} packs, {total} values.",
        "",
        "#include <cstddef>",
        "",
        '#include "data/PidPacksTable.h"',
        "",
        "namespace caruspex::pidpacks::detail {",
        "namespace {",
        "",
        "using C = PidCategory;",
        "",
        "const PackRow kPacks[] = {",
    ]
    for p in packs:
        lines.append(f"    {{{cpp(p['id'])}, {{{cpp(p['title_de'])}, {cpp(p['title_en'])}}}, "
                     f"C::{p['category']}}},  // {p['file']}")
    lines += ["};", "", "const ValueRow kValues[] = {"]
    for index, p in enumerate(packs):
        lines.append(f"    // {p['id']}")
        for r in p["rows"]:
            lines.append(
                f"    {{{index}, {{{cpp(r['de'])}, {cpp(r['en'])}}}, "
                f"{{{cpp(r['short_de'])}, {cpp(r['short_en'])}}}, {cpp(r['request'])}, "
                f"{cpp(r['header'])}, {cpp(r['response'])}, {cpp(r['equation'])}, "
                f"{number(r['min'])}, {number(r['max'])}, {cpp(r['unit'])},\n"
                f"     {cpp(r['source'])}}},")
    lines += [
        "};",
        "",
        "}  // namespace",
        "",
        "const PackRow* packTable(std::size_t& count) {",
        "    count = sizeof(kPacks) / sizeof(kPacks[0]);",
        "    return kPacks;",
        "}",
        "",
        "const ValueRow* valueTable(std::size_t& count) {",
        "    count = sizeof(kValues) / sizeof(kValues[0]);",
        "    return kValues;",
        "}",
        "",
        "}  // namespace caruspex::pidpacks::detail",
        "",
    ]
    io.open(TARGET, "w", encoding="utf-8", newline="\n").write("\n".join(lines))
    counts = ", ".join(f"{p['id']} {len(p['rows'])}" for p in packs)
    print(f"{TARGET}\n{len(packs)} packs, {total} values ({counts})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
