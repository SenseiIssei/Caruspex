"""More SAE J2012 / ISO 15031-6 codes, composed from shared phrases.

generic.py writes every text out. The codes here are the long tail of the
standard: thousands of codes that differ only in the part (sensor A, B, C,
cylinder 1 to 12) and the fault type (circuit low, circuit high). They are
listed in extended.csv, one row per code, and their texts are put together
from two phrases at runtime: the part with its letters or numbers filled in,
and the fault type. phrases.py holds both in German and English; the other
16 languages translate each phrase once (area dtc38).

extended.csv, separated by ';', lines starting with '#' are comments:

    code;part;args;fault;form;severity;system;detail;tier

    part      English phrase of the part, %1 to %9 stand for args
    args      letters or numbers for %1, %2, ..., separated by '|'
    fault     English fault type, empty when the part says it all
    form      '-' gives "part - fault", ':' gives "fault: part"
    detail    key of a plain-language block in details.py, may be empty
    tier      'confirmed' when two independent sources agree on the meaning,
              'single' when one credible, cleanly licensed source defines it;
              the app and the reports say "Meaning from one source" next to it

Which codes are in the table and in which tier: see SOURCES. Codes left out
are listed with the reason in data/dtc/uncertain.md; they still get the
group meaning of their range.
"""

from __future__ import annotations

import csv
import io
import os
import re
from dataclasses import dataclass

from .model import SEVERITIES, SYSTEMS, check_text, is_manufacturer
from .phrases import FAULTS, PARTS

HERE = os.path.dirname(os.path.abspath(__file__))
CSV_PATH = os.path.join(HERE, "extended.csv")

# Written into the head of the generated tables. Per block: where the code
# numbers and their meaning were checked. The texts are Caruspex's own.
SOURCES = [
    "Extended generic codes (tools/dtc/extended.csv): code numbers and meanings as facts, texts in Caruspex's",
    "own words. Tier 'confirmed': two independent sources agree on the meaning (1 and 2, or 1 and 3).",
    "Tier 'single': only source 1 defines it, and obdex does not itself call the title unverified or",
    "absent from the public J2012 lists; shown with the note \"Meaning from one source\".",
    "1. obdex, https://github.com/foerbsnavi/OBDex (data CC0-1.0, LICENSE-DATA), commit 9ba1ffa, data of",
    "   2026-11-07, retrieved 2026-11-20: title of every generic code (P0, P2, P3, U0, U3, B0, C0).",
    "2. GM OBD descriptions (OBDG, 'mode6' documents published by General Motors for the emission",
    "   regulations), https://gsi.ext.gm.com/gmspo/mode6/pdf/, 117 documents 1997 to 2025, retrieved",
    "   2026-11-20: the code with the same part and fault in the monitor tables. Facts only, no wording.",
    "3. Manufacturer bulletins in the public NHTSA archive (US government site),",
    "   https://static.nhtsa.gov/odi/tsbs/ and /odi/inv/, retrieved 2026-11-20: the code with the same",
    "   part and fault in a bulletin or investigation document. Facts only, no wording.",
    "Not used: SAE J2012 and ISO 15031-6 themselves (paid), GPL code lists (python-OBD and other scan",
    "tools), code lists without a licence.",
]

_ARG = re.compile(r"^[A-Z0-9]{1,3}(?:/[A-Z0-9]{1,3})*$")
_PLACEHOLDER = re.compile(r"%([1-9])")


@dataclass(frozen=True)
class Ext:
    part: str
    args: str
    fault: str
    lead: bool
    single: bool
    sev: str
    sys: str
    detail: str


def placeholders(text: str) -> set[int]:
    return {int(n) for n in _PLACEHOLDER.findall(text)}


def _load() -> dict[str, Ext]:
    out: dict[str, Ext] = {}
    with io.open(CSV_PATH, encoding="utf-8", newline="") as f:
        rows = [line for line in f if line.strip() and not line.startswith("#")]
    for row in csv.reader(rows, delimiter=";"):
        assert len(row) == 9, f"extended.csv: wrong number of fields in {row}"
        code, part, args, fault, form, sev, sys_, detail, tier = row
        assert tier in ("confirmed", "single"), f"extended.csv {code}: tier {tier}"
        assert re.match(r"^[PCBU][0-3][0-9A-F]{3}$", code), f"extended.csv: malformed code {code}"
        assert code not in out, f"extended.csv: {code} listed twice"
        assert not is_manufacturer(code), f"extended.csv: {code} lies in a manufacturer range"
        assert part in PARTS, f"extended.csv {code}: part {part!r} missing in phrases.py"
        assert not fault or fault in FAULTS, f"extended.csv {code}: fault {fault!r} missing in phrases.py"
        assert form in ("-", ":"), f"extended.csv {code}: form must be '-' or ':'"
        assert form == "-" or fault, f"extended.csv {code}: ':' needs a fault"
        assert sev in SEVERITIES, f"extended.csv {code}: severity {sev}"
        assert sys_ in SYSTEMS, f"extended.csv {code}: system {sys_}"
        values = args.split("|") if args else []
        assert all(_ARG.match(v) for v in values), f"extended.csv {code}: bad args {args!r}"
        assert placeholders(part) == set(range(1, len(values) + 1)), \
            f"extended.csv {code}: {part!r} does not take {len(values)} args"
        out[code] = Ext(part, args, fault, form == ":", tier == "single", sev, sys_, detail)
    return out


def _check_phrases() -> None:
    for table in (PARTS, FAULTS):
        for en, de in table.items():
            check_text(en, en)
            check_text(en, de)
            assert placeholders(en) == placeholders(de), f"placeholders differ: {en!r} / {de!r}"
    for en in FAULTS:
        assert not placeholders(en), f"a fault type takes no args: {en!r}"


_check_phrases()
EXT: dict[str, Ext] = _load()
