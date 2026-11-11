"""Generates crates/caruspex-data/src/generated/ecu_rows.rs from data/ecus/*.csv.

Every CSV holds the control units of one or more brand families, one per row:

    family,kind,request,response,bits,name_de,name_en,note

    family     brand family key as in DtcBrands (VAG, FORD, TOYOTA, ...)
    kind       engine gearbox brakes airbag steering instruments gateway body
               climate parking lighting doors seats infotainment battery
               charger hybrid other
    request    CAN id the tester sends to, hex without 0x ("7E0", "18DA10F1")
    response   CAN id the control unit answers on, hex
    bits       11, 29, or ext for BMW style extended addressing: 11-bit ids,
               request 6F1 for every unit, response 6xx, and xx (the unit's
               diagnostic address) as the first data byte of every frame
    name_de    display name in German
    name_en    display name in English
    note       free text for maintainers (VAG address, model range, ...),
               copied into the generated file as a comment only

Lines starting with '#' are comments. Only addresses with a reliable source
go in; say where they come from in the comment at the top of the file.
The scan order is decided in crates/caruspex-data/src/ecu_addresses.rs from the kind, so the row
order inside a file only matters within one kind.

    py -I tools/gen_ecus.py
"""

from __future__ import annotations

import csv
import io
import os
import re
import sys

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(HERE, "data", "ecus")
RUST_TARGET = os.path.join(HERE, "crates", "caruspex-data", "src", "generated", "ecu_rows.rs")

COLUMNS = ["family", "kind", "request", "response", "bits", "name_de", "name_en", "note"]

# Same keys as FAMILIES in crates/caruspex-data/src/generated/dtc_brands.rs. The test
# checks that every family of the ECU lists is known there.
FAMILIES = [
    "VAG", "BMW", "MERCEDES", "FORD", "MAZDA", "GM", "OPEL", "TOYOTA", "HONDA",
    "NISSAN", "HYUNDAI", "RENAULT", "PSA", "FIAT", "VOLVO", "SUBARU", "MITSUBISHI",
]

KINDS = {
    "engine": "Engine", "gearbox": "Gearbox", "brakes": "Brakes", "airbag": "Airbag",
    "steering": "Steering", "instruments": "Instruments", "gateway": "Gateway",
    "body": "Body", "climate": "Climate", "parking": "Parking", "lighting": "Lighting",
    "doors": "Doors", "seats": "Seats", "infotainment": "Infotainment",
    "battery": "Battery", "charger": "Charger", "hybrid": "Hybrid", "other": "Other",
}

HEX = re.compile(r"^[0-9A-F]{1,8}$")


class DataError(Exception):
    pass


def check_text(text: str, where: str) -> None:
    for bad in (chr(0x2014), chr(0x2013)):  # em dash, en dash
        if bad in text:
            raise DataError(f"{where}: no long dashes in user-visible text")
    if '"' in text or "\\" in text:
        raise DataError(f"{where}: quotes and backslashes are not allowed")


def parse_id(text: str, bits: int, what: str) -> int:
    value = text.upper()
    if not HEX.match(value):
        raise DataError(f"{what} '{text}' is not a hex id like 7E0")
    number = int(value, 16)
    limit = 0x7FF if bits == 11 else 0x1FFFFFFF
    if number > limit:
        raise DataError(f"{what} {value} does not fit into {bits} bits")
    return number


def build_row(r: dict) -> dict:
    if r["family"] not in FAMILIES:
        raise DataError(f"unknown family '{r['family']}'")
    if r["kind"] not in KINDS:
        raise DataError(f"unknown kind '{r['kind']}'")
    if r["bits"] not in ("11", "29", "ext"):
        raise DataError("bits must be 11, 29 or ext")
    extended_address = -1
    bits = 11 if r["bits"] == "ext" else int(r["bits"])
    request = parse_id(r["request"], bits, "request")
    response = parse_id(r["response"], bits, "response")
    if request == response:
        raise DataError("request and response must differ")
    if bits == 11 and request == 0x7DF:
        raise DataError("7DF is the OBD broadcast address, not a control unit")
    if r["bits"] == "ext":
        if request != 0x6F1 or (response & 0xF00) != 0x600:
            raise DataError("extended addressing needs request 6F1 and a response 6xx")
        extended_address = response & 0xFF
    if not r["name_de"] or not r["name_en"]:
        raise DataError("name needs German and English")
    for col in ("name_de", "name_en", "note"):
        check_text(r[col], col)
    return {
        "family": r["family"], "kind": KINDS[r["kind"]], "request": request,
        "response": response, "ext": bits == 29, "xaddr": extended_address, "de": r["name_de"],
        "en": r["name_en"], "note": r["note"],
    }


def load_rows() -> list[dict]:
    out: list[dict] = []
    seen: set[tuple] = set()
    files = sorted(n for n in os.listdir(DATA) if n.endswith(".csv"))
    if not files:
        raise DataError(f"no CSV files in {DATA}")
    for name in files:
        header_seen = False
        with io.open(os.path.join(DATA, name), encoding="utf-8", newline="") as f:
            for lineno, raw in enumerate(csv.reader(f), start=1):
                if not raw or not "".join(raw).strip():
                    continue
                if raw[0].strip().startswith("#"):
                    continue
                cells = [c.strip() for c in raw]
                if not header_seen:
                    if cells != COLUMNS:
                        raise DataError(f"{name}: header must be {','.join(COLUMNS)}")
                    header_seen = True
                    continue
                where = f"{name}:{lineno}"
                if len(cells) != len(COLUMNS):
                    raise DataError(f"{where}: expected {len(COLUMNS)} columns, got {len(cells)}")
                try:
                    row = build_row(dict(zip(COLUMNS, cells)))
                except DataError as e:
                    raise DataError(f"{where}: {e}") from None
                keys = [("response", row["response"], row["ext"])]
                # With extended addressing every unit shares the request id.
                if row["xaddr"] < 0:
                    keys.insert(0, ("request", row["request"], row["ext"]))
                for key in keys:
                    if (row["family"],) + key in seen:
                        raise DataError(f"{where}: {key[0]} {key[1]:X} twice in {row['family']}")
                    seen.add((row["family"],) + key)
                out.append(row)
    return out


def rust(text: str) -> str:
    return '"' + text + '"'


def write_rust(rows: list[dict], families: list[str]) -> None:
    lines = [
        "// Generated by tools/gen_ecus.py from data/ecus/*.csv, do not edit by hand.",
        f"// {len(families)} families, {len(rows)} control units.",
        "",
        "use crate::ecu_addresses::{EcuKind as K, EcuRow};",
        "use caruspex_i18n::Text;",
        "",
        "pub static ECU_ROWS: &[EcuRow] = &[",
    ]
    for r in rows:
        xaddr = r["xaddr"] if r["xaddr"] < 0 else hex(r["xaddr"])
        line = (f"    EcuRow {{ family: {rust(r['family'])}, request: 0x{r['request']:X}, "
                f"response: 0x{r['response']:X}, extended29: {'true' if r['ext'] else 'false'}, "
                f"extended_address: {xaddr}, kind: K::{r['kind']}, "
                f"name: Text::new({rust(r['de'])}, {rust(r['en'])}) }},")
        if r["note"]:
            line += f"  // {r['note']}"
        lines.append(line)
    lines += ["];", ""]
    os.makedirs(os.path.dirname(RUST_TARGET), exist_ok=True)
    io.open(RUST_TARGET, "w", encoding="utf-8", newline="\n").write("\n".join(lines))
    print(RUST_TARGET)


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    try:
        rows = load_rows()
    except DataError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1

    # Keep the families in the order of the DTC database, which is also the
    # order the ecu_addresses module reports them in.
    rows.sort(key=lambda r: FAMILIES.index(r["family"]))
    families = [f for f in FAMILIES if any(r["family"] == f for r in rows)]

    write_rust(rows, families)
    counts = ", ".join(f"{f} {sum(1 for r in rows if r['family'] == f)}" for f in families)
    print(f"{len(families)} families, {len(rows)} control units ({counts})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
