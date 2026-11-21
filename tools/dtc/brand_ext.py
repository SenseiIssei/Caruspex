"""Manufacturer codes taken from public NHTSA documents, composed from phrases.

brands.py writes the older manufacturer codes out in full. The codes here
come from the manufacturer communications (technical service bulletins)
that NHTSA publishes as a data set, and their texts are put together from
the shared phrases in phrases.py, like the long tail of the generic codes
(extended.py). Only the fact is taken from a bulletin: which code the maker
names and what part and fault it stands for. The wording is Caruspex's own.

brand_codes.csv, separated by ';', lines starting with '#' are comments:

    family;code;part;args;fault;form;severity;system;detail;tier;nhtsa

    family    key of a family in brands.py
    part, args, fault, form, severity, system, detail
              as in extended.csv
    tier      'confirmed' when at least two bulletins with their own number,
              issued in different months, give the same meaning; 'single'
              when one bulletin does (the same text filed once per brand or
              model line counts once). The app says "Meaning from one source"
              next to a single code.
    nhtsa     up to four NHTSA ids of the bulletins, comma separated, so a row
              can be checked again

A code that brands.py already lists for the family stays there; this table
may not repeat it.
"""

from __future__ import annotations

import csv
import io
import os
import re

from .brands import BY_KEY
from .extended import Ext, placeholders
from .model import SEVERITIES, SYSTEMS, is_manufacturer
from .phrases import FAULTS, PARTS

HERE = os.path.dirname(os.path.abspath(__file__))
CSV_PATH = os.path.join(HERE, "brand_codes.csv")

# Written into the head of the generated dtc_brands.rs.
SOURCES = [
    "Composed manufacturer codes (tools/dtc/brand_codes.csv): code numbers and meanings as facts,",
    "texts in Caruspex's own words from the phrase table. Source: the manufacturer communications",
    "(technical service bulletins) in NHTSA's public data set, https://static.nhtsa.gov/odi/ffdd/tsbs/",
    "(TSBS_RECEIVED_1995-1999 to TSBS_RECEIVED_2020-2024, US government data, public domain),",
    "retrieved 2026-11-21: the summary of a bulletin names the code with its part and fault. The",
    "bulletins are for the US market; on cars for other markets a maker may use a code differently.",
    "Tier 'confirmed': two bulletins with their own number from different months give the meaning;",
    "tier 'single': one bulletin does, shown with the note \"Meaning from one source\".",
    "Not used: GPL code lists, unlicensed lists, forums, paid code databases.",
]

_ARG = re.compile(r"^[A-Z0-9]{1,3}$")


def _load() -> dict[str, dict[str, Ext]]:
    out: dict[str, dict[str, Ext]] = {}
    with io.open(CSV_PATH, encoding="utf-8", newline="") as f:
        rows = [line for line in f if line.strip() and not line.startswith("#")]
    for row in csv.reader(rows, delimiter=";"):
        assert len(row) == 11, f"brand_codes.csv: wrong number of fields in {row}"
        fam, code, part, args, fault, form, sev, sys_, detail, tier, nhtsa = row
        where = f"brand_codes.csv {fam} {code}"
        assert fam in BY_KEY, f"{where}: unknown family"
        assert re.match(r"^[PCBU][0-3][0-9A-F]{3}$", code), f"{where}: malformed code"
        assert is_manufacturer(code), f"{where}: not a manufacturer range"
        table = out.setdefault(fam, {})
        assert code not in table, f"{where}: listed twice"
        assert code not in BY_KEY[fam].table.codes, f"{where}: already written out in brands.py"
        assert part in PARTS, f"{where}: part {part!r} missing in phrases.py"
        assert not fault or fault in FAULTS, f"{where}: fault {fault!r} missing in phrases.py"
        assert form in ("-", ":"), f"{where}: form must be '-' or ':'"
        assert form == "-" or fault, f"{where}: ':' needs a fault"
        assert sev in SEVERITIES, f"{where}: severity {sev}"
        assert sys_ in SYSTEMS, f"{where}: system {sys_}"
        assert tier in ("confirmed", "single"), f"{where}: tier {tier}"
        assert re.match(r"^\d+(,\d+){0,3}$", nhtsa), f"{where}: NHTSA ids {nhtsa!r}"
        assert tier == "single" or "," in nhtsa, f"{where}: a confirmed code needs two bulletins"
        values = args.split("|") if args else []
        assert all(_ARG.match(v) for v in values), f"{where}: bad args {args!r}"
        assert placeholders(part) == set(range(1, len(values) + 1)), \
            f"{where}: {part!r} does not take {len(values)} args"
        table[code] = Ext(part, args, fault, form == ":", tier == "single", sev, sys_, detail)
    return out


BRAND_EXT: dict[str, dict[str, Ext]] = _load()
