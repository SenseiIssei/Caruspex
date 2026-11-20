"""Generates the fault code tables in crates/caruspex-data/src/generated/*.rs.

The data lives in tools/dtc/:

    generic.py  SAE J2012 / ISO 15031-6 codes, same meaning on every car
    extended.py the long tail of the standard (extended.csv), composed from
                the shared phrases in phrases.py
    brands.py   manufacturer-specific codes per brand family
    details.py  plain-language advice, causes, checks and a cost class
    ranges.py   group meanings for codes that are in no table

Many codes follow fixed families (per cylinder, per bank, per sensor). They
are computed instead of typed out, which avoids typos. Every text exists in
German and English; the app picks the active language at lookup time.

The output is plain static slices, sorted by code, one file per area. That
keeps compile times low and needs no runtime initialisation.

    py -I tools/gen_dtc.py
"""

from __future__ import annotations

import io
import json
import os
import re
import sys
import unicodedata

# -I does not put the script directory on sys.path, so add it by hand.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from dtc.brands import FAMILIES, SOURCES  # noqa: E402
from dtc.details import DETAILS  # noqa: E402
from dtc.extended import EXT, SOURCES as EXT_SOURCES  # noqa: E402
from dtc.generic import T as GENERIC  # noqa: E402
from dtc.model import COSTS, SEVERITIES, SYSTEMS, Table, is_manufacturer  # noqa: E402
from dtc.phrases import FAULTS, PARTS  # noqa: E402
from dtc.ranges import RANGES, fine_rules  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RUST_OUT = os.path.join(ROOT, "crates", "caruspex-data", "src", "generated")
MOCK_OUT = os.path.join(ROOT, "app", "src", "api", "mock", "data", "dtc.json")
DETAIL_INDEX = {key: i for i, key in enumerate(DETAILS)}


def detail_ref(key: str) -> str:
    if not key:
        return ""
    assert key in DETAIL_INDEX, f"unknown detail block {key}"
    return str(DETAIL_INDEX[key])


def quote(text: str) -> str:
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def normalize_brand(name: str) -> str:
    """Must match normalize_brand() in crates/caruspex-data/src/dtc/mod.rs."""
    folded = unicodedata.normalize("NFKD", name)
    folded = "".join(c for c in folded if not unicodedata.combining(c))
    return re.sub(r"[^a-z0-9]", "", folded.lower())


def split_generic() -> dict[str, Table]:
    parts = {letter: Table(f"generic {letter}") for letter in "PCBU"}
    for code, rec in GENERIC.codes.items():
        parts[code[0]].codes[code] = rec
    return parts


def split_extended() -> dict[str, dict]:
    parts: dict[str, dict] = {letter: {} for letter in "PCBU"}
    for code, ext in EXT.items():
        parts[code[0]][code] = ext
    return parts


def phrase_list() -> list[tuple[str, str]]:
    """(de, en) of every phrase that a record uses, in a fixed order."""
    used_parts = sorted({e.part for e in EXT.values()})
    used_faults = sorted({e.fault for e in EXT.values() if e.fault})
    return [(PARTS[en], en) for en in used_parts] + [(FAULTS[en], en) for en in used_faults]


PHRASE_INDEX = {en: i for i, (_, en) in enumerate(phrase_list())}


def rtext(de: str, en: str) -> str:
    return f"Text::new({quote(de)}, {quote(en)})"


def rust_detail(key: str) -> str:
    return "NO_DETAIL" if not key else str(DETAIL_INDEX[key])


def rust_record_lines(table: Table, extended: dict | None = None) -> list[str]:
    extended = extended or {}
    lines = []
    for code in sorted([*table.codes, *extended]):
        if code in extended:
            e = extended[code]
            how = str(PHRASE_INDEX[e.fault]) if e.fault else "NO_PHRASE"
            lines.append(f"    q({quote(code)}, {PHRASE_INDEX[e.part]}, {quote(e.args)}, {how}, "
                         f"{'true' if e.lead else 'false'}, {'true' if e.single else 'false'}, "
                         f"Severity::{SEVERITIES[e.sev]}, "
                         f"DtcSystem::{SYSTEMS[e.sys]}, {rust_detail(e.detail)}),")
            continue
        r = table.codes[code]
        lines.append(f"    r({quote(code)}, {quote(r.de)}, {quote(r.en)}, Severity::{SEVERITIES[r.sev]}, "
                     f"DtcSystem::{SYSTEMS[r.sys]}, {rust_detail(r.detail)}, "
                     f"{'true' if r.throttle else 'false'}),")
    return lines


def record_uses(lines: list[str]) -> list[str]:
    """Imports for a table file, only what it needs (clippy denies unused ones)."""
    body = "\n".join(lines)
    names = ["NO_DETAIL"]
    if "NO_PHRASE" in body:
        names.append("NO_PHRASE")
    names.append("Record")
    if "    q(" in body:
        names.append("q")
    if "    r(" in body:
        names.append("r")
    return ["use crate::dtc::DtcSystem;",
            f"use crate::dtc::tables::{{{', '.join(names)}}};",
            "use crate::severity::Severity;"]


def rust_header(what: str, uses: list[str]) -> list[str]:
    return [
        "// Generated by tools/gen_dtc.py, do not edit by hand.",
        f"// {what}",
        "",
        *uses,
        "",
    ]


def write_rust(name: str, lines: list[str]) -> None:
    os.makedirs(RUST_OUT, exist_ok=True)
    path = os.path.join(RUST_OUT, name)
    with io.open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")
    print(f"wrote {path}")


def rust_family_const(key: str) -> str:
    ident = "FAMILY_" + key.upper()
    assert ident.isidentifier(), key
    return ident


def write_all_rust(parts: dict[str, Table]) -> None:
    names = {"P": "POWERTRAIN", "C": "CHASSIS", "B": "BODY", "U": "NETWORK"}
    ext = split_extended()
    sources = "\n//\n// " + "\n// ".join(EXT_SOURCES)

    body = ["pub static POWERTRAIN: &[Record] = &["] + rust_record_lines(parts["P"], ext["P"]) + ["];"]
    lines = rust_header(f"{len(parts['P'])} written out and {len(ext['P'])} composed generic powertrain codes "
                        f"(SAE J2012).{sources}", record_uses(body))
    write_rust("dtc_generic_p.rs", lines + body)

    count = sum(len(parts[x]) for x in "CBU")
    count_ext = sum(len(ext[x]) for x in "CBU")
    body = []
    for letter in "CBU":
        body += [f"pub static {names[letter]}: &[Record] = &["]
        body += rust_record_lines(parts[letter], ext[letter])
        body += ["];", ""]
    lines = rust_header(f"{count} written out and {count_ext} composed generic chassis, body and network codes "
                        f"(SAE J2012).{sources}", record_uses(body))
    write_rust("dtc_generic_cbu.rs", lines + body[:-1])

    lines = rust_header(f"{len(PHRASE_INDEX)} phrases the composed fault code texts are built from.",
                        ["use caruspex_i18n::Text;"])
    lines += ["pub static PHRASES: &[Text] = &["]
    for i, (de, en) in enumerate(phrase_list()):
        lines.append(f"    // {i}")
        lines.append(f"    {rtext(de, en)},")
    lines += ["];"]
    write_rust("dtc_phrases.rs", lines)

    brand_total = sum(len(f.table) for f in FAMILIES)
    # The source statement of brands.py goes into the head of the file.
    lines = rust_header(f"{brand_total} manufacturer-specific codes in {len(FAMILIES)} brand families.\n// "
                        + "\n// ".join(SOURCES),
                        ["use crate::dtc::DtcSystem;",
                         "use crate::dtc::tables::{Family, NO_DETAIL, Record, r};",
                         "use crate::severity::Severity;",
                         "use caruspex_i18n::Text;"])
    for fam in FAMILIES:
        if not fam.table.codes:
            continue
        lines += [f"static {rust_family_const(fam.key)}: &[Record] = &["]
        lines += rust_record_lines(fam.table)
        lines += ["];", ""]
    lines += ["pub static FAMILIES: &[Family] = &["]
    for fam in FAMILIES:
        brands = "|".join(dict.fromkeys(normalize_brand(b) for b in fam.brands))
        records = rust_family_const(fam.key) if fam.table.codes else "&[]"
        lines.append(f"    Family {{ key: {quote(fam.key)}, label: {rtext(*fam.label)}, "
                     f"brands: {quote(brands)}, records: {records} }},")
    lines += ["];"]
    write_rust("dtc_brands.rs", lines)

    lines = rust_header(f"{len(DETAILS)} plain-language help blocks and {len(all_rules())} group rules.",
                        ["use crate::dtc::tables::{Detail, NO_DETAIL, RangeRule};",
                         "use crate::dtc::{CostClass, DtcSystem};",
                         "use crate::severity::Severity;",
                         "use caruspex_i18n::Text;"])
    lines += ["pub static DETAILS: &[Detail] = &["]
    for key, (cost, advice, causes, checks) in DETAILS.items():
        causes_de = "\n".join(c[0] for c in causes)
        causes_en = "\n".join(c[1] for c in causes)
        checks_de = "\n".join(c[0] for c in checks)
        checks_en = "\n".join(c[1] for c in checks)
        lines += [f"    // {DETAIL_INDEX[key]}: {key}",
                  "    Detail {",
                  f"        advice: {rtext(*advice)},",
                  f"        causes: {rtext(causes_de, causes_en)},",
                  f"        checks: {rtext(checks_de, checks_en)},",
                  f"        cost: CostClass::{COSTS[cost]},",
                  "    },"]
    lines += ["];", "", "pub static RANGE_RULES: &[RangeRule] = &["]
    for letter, lo, hi, de, en, sev, sys, det in all_rules():
        lines.append(f"    RangeRule {{ letter: '{letter}', lo: 0x{lo:04X}, hi: 0x{hi:04X}, text: {rtext(de, en)}, "
                     f"severity: Severity::{SEVERITIES[sev]}, system: DtcSystem::{SYSTEMS[sys]}, "
                     f"detail: {rust_detail(det)} }},")
    lines += ["];"]
    write_rust("dtc_details.rs", lines)


def write_mock() -> None:
    """The generic tables for the browser preview (app/src/api/mock/dtc.ts).

    Written-out codes as before, composed codes as phrase references like in
    Rust, so the lazy chunk stays small. System names and zone ranges come
    from Rust code (dtc/mod.rs, dtc_location.rs) and are carried over from
    the existing file unchanged.
    """
    old = json.load(io.open(MOCK_OUT, encoding="utf-8"))
    sev = list(SEVERITIES)
    sys_ = list(SYSTEMS)
    cost = list(COSTS)
    det = lambda key: DETAIL_INDEX[key] if key else -1  # noqa: E731
    codes = [[code, r.de, r.en, sev.index(r.sev), sys_.index(r.sys), det(r.detail), r.throttle]
             for code, r in sorted(GENERIC.codes.items())]
    composed = [[code, PHRASE_INDEX[e.part], e.args, PHRASE_INDEX[e.fault] if e.fault else -1,
                 1 if e.lead else 0, 1 if e.single else 0, sev.index(e.sev), sys_.index(e.sys), det(e.detail)]
                for code, e in sorted(EXT.items())]
    details = [[advice[0], advice[1], [c[0] for c in causes], [c[1] for c in causes],
                [c[0] for c in checks], [c[1] for c in checks], cost.index(c_)]
               for c_, advice, causes, checks in DETAILS.values()]
    data = {"codes": codes, "composed": composed, "phrases": [list(p) for p in phrase_list()],
            "details": details, "systemNames": old["systemNames"], "zoneRanges": old["zoneRanges"]}
    with io.open(MOCK_OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(data, ensure_ascii=False, separators=(",", ":")))
    print(f"wrote {MOCK_OUT}")


def known_generic() -> dict[str, tuple[str, str]]:
    """Severity and system of every generic code, written out or composed."""
    out = {code: (r.sev, r.sys) for code, r in GENERIC.codes.items()}
    out.update({code: (e.sev, e.sys) for code, e in EXT.items()})
    return out


_RULES: list[tuple] = []


def all_rules() -> list[tuple]:
    """The finer rules derived from the tables first, then the hand-written ones."""
    if not _RULES:
        _RULES.extend(fine_rules(known_generic()) + RANGES)
    return _RULES


def check() -> None:
    for code in GENERIC.codes:
        assert not is_manufacturer(code), f"{code} lies in a manufacturer range but is listed as generic"
    for code in EXT:
        assert code not in GENERIC.codes, f"{code} is in generic.py and in extended.csv"
    for fam in FAMILIES:
        for code in fam.table.codes:
            # A brand table may define codes in an SAE range only where SAE has
            # not given them a meaning we list, otherwise it would hide it.
            assert is_manufacturer(code) or code not in GENERIC.codes, \
                f"{fam.key} {code} would shadow the generic meaning"
    for rec in [*GENERIC.codes.values(), *EXT.values(), *(r for f in FAMILIES for r in f.table.codes.values())]:
        detail_ref(rec.detail)
    for r in all_rules():
        detail_ref(r[7])


def main() -> None:
    check()
    parts = split_generic()
    write_all_rust(parts)
    write_mock()

    brand_total = sum(len(f.table) for f in FAMILIES)
    with_detail = sum(1 for r in GENERIC.codes.values() if r.detail)
    brand_detail = sum(1 for f in FAMILIES for r in f.table.codes.values() if r.detail)
    ext = split_extended()
    print(f"generic: {len(GENERIC)} codes ({', '.join(f'{x} {len(parts[x])}' for x in 'PCBU')}), "
          f"{with_detail} with plain-language details")
    print(f"extended: {len(EXT)} codes ({', '.join(f'{x} {len(ext[x])}' for x in 'PCBU')}), "
          f"{sum(1 for e in EXT.values() if not e.single)} confirmed, "
          f"{sum(1 for e in EXT.values() if e.single)} from one source, "
          f"{sum(1 for e in EXT.values() if e.detail)} with details, {len(PHRASE_INDEX)} phrases")
    print("brands: " + ", ".join(f"{f.key} {len(f.table)}" for f in FAMILIES)
          + f" = {brand_total}, {brand_detail} with details")
    print(f"details: {len(DETAILS)} blocks, ranges: {len(RANGES)} rules and {len(all_rules()) - len(RANGES)} finer ones")


if __name__ == "__main__":
    main()
