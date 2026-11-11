"""Generates crates/caruspex-vehicles/src/generated/catalog.rs from data/vehicles/*.csv.

brands.csv lists every brand once:

    name,country_de,country_en,group,market

group is one of vag, bmw, mb, ford, gm, chrysler or empty. It decides which
manufacturer plug older cars have and which OBD-II protocol a US car of the
1990s speaks. market (eu or na) is the default for the rows of that brand.

Every other CSV holds one model generation per row:

    brand,model,generation,from,to,body,power,vin,connector,location,
    aliases,note_de,note_en,bus,protocols

Only the first seven columns are required. Empty columns are filled in from
the brand group, the market and the years, the same rules a person would
apply. Fill them in by hand only where a model differs from the rule.

    from, to     production years, to = 0 or empty while still built
    body         hatch sedan estate suv van coupe cabrio pickup mpv micro roadster
    power        letters: P petrol, D diesel, H hybrid, X plug-in, E electric, G gas
    vin          model codes from the VIN, space separated (VAG "1J", Mercedes "203")
    connector    none obd2 vag2x2 bmw20 mb38 other, optionally with a later
                 connector: "vag2x2>obd2@1996" means round plugs, from about
                 1996 the 16-pin socket
    location     dashleft (driver side under the dashboard, near the steering
                 column), column, footwell, fusebox, console, ashtray,
                 passenger, engine, rear; empty if not known
    aliases      extra search words separated by '|'
    bus          kline can pwm vpw mixed
    protocols    ELM327 protocol digits in the order to try

A comment line "#@ market=na" switches the market for the following rows of
that file, "#@ market=" goes back to the brand default.

    py -I tools/gen_vehicles.py
"""

from __future__ import annotations

import csv
import io
import os
import sys

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(HERE, "data", "vehicles")
TARGET_RS = os.path.join(HERE, "crates", "caruspex-vehicles", "src", "generated", "catalog.rs")

COLUMNS = ["brand", "model", "generation", "from", "to", "body", "power", "vin",
           "connector", "location", "aliases", "note_de", "note_en", "bus", "protocols"]

BODY = {
    "hatch": "Hatchback", "sedan": "Sedan", "estate": "Estate", "suv": "Suv",
    "van": "Van", "coupe": "Coupe", "cabrio": "Convertible", "pickup": "Pickup",
    "mpv": "Mpv", "micro": "Microcar", "roadster": "Roadster",
}
POWER = {"P": "PtPetrol", "D": "PtDiesel", "H": "PtHybrid", "X": "PtPlugIn",
         "E": "PtElectric", "G": "PtGas"}
CONNECTOR = {"none": "None", "obd2": "Obd2", "vag2x2": "Vag2x2", "bmw20": "Bmw20",
             "mb38": "Mb38", "other": "Other"}
LOCATION = {
    "": "Unknown", "dashleft": "UnderDashLeft", "column": "UnderSteeringColumn",
    "footwell": "FootwellCover", "fusebox": "FuseBox", "console": "CentreConsole",
    "ashtray": "BehindAshtray", "passenger": "PassengerSide", "engine": "EngineBay",
    "rear": "RearConsole",
}
BUS = {"kline": "KLine", "can": "Can", "pwm": "Pwm", "vpw": "Vpw", "mixed": "Mixed"}

# ELM327 protocol numbers: 1 J1850 PWM, 2 J1850 VPW, 3 ISO 9141-2,
# 4 KWP 5-baud, 5 KWP fast, 6-9 CAN. Same orders as crates/caruspex-vehicles/src/vehicles.rs.
ORDER = {
    "kline": "5436789",
    "can": "6789543",
    "pwm": "1265439",
    "vpw": "2165439",
    "mixed": "6543789",
    "mixed-ford": "615437892",
    "mixed-gm": "625437891",
    "mixed-chrysler": "623547891",
    "kline-chrysler": "325467891",
}

CURRENT_YEAR = 2026


class DataError(Exception):
    pass


def default_connector(group: str, market: str, y0: int, y1: int) -> str:
    """Connector after the usual rules, as "early" or "early>late@year"."""
    if y0 >= 1996:
        return "obd2"
    if market == "na":
        early, switch = "other", 1996
    elif group == "vag":
        if y0 < 1990:
            return "none>vag2x2@1990" if y1 >= 1991 else "none"
        early, switch = "vag2x2", 1996
    elif group == "bmw":
        if y0 < 1987:
            return "none>bmw20@1987" if y1 >= 1988 else "none"
        early, switch = "bmw20", 2001
    elif group == "mb":
        early = "mb38" if y0 >= 1993 else "other"
        switch = 2001
    else:
        if y0 < 1987:
            # Long runners got a socket with EOBD, the others at least a
            # manufacturer plug once fuel injection with fault memory came.
            if y1 >= 2002:
                return "none>obd2@2001"
            if y1 >= 1991:
                return "none>other@1990"
            return "none"
        early, switch = "other", 1996
    if y1 >= switch + 1:
        return f"{early}>obd2@{switch}"
    return early


def default_bus(group: str, market: str, power: str, obd_from: int, y1: int) -> str:
    if power == "E" or obd_from >= 2008:
        return "can"
    if group == "vag" and market != "na" and obd_from <= 2001:
        # VAG stayed with K-Line for the whole life of these generations.
        return "kline"
    if market == "na":
        if y1 <= 2003:
            return {"ford": "pwm", "gm": "vpw", "chrysler": "kline"}.get(group, "kline")
        return "mixed"
    if y1 <= 2003:
        return "kline"
    return "mixed"


def default_order(group: str, market: str, bus: str) -> str:
    if market == "na" and group in ("ford", "gm", "chrysler"):
        key = f"{bus}-{group}"
        if key in ORDER:
            return ORDER[key]
    return ORDER[bus]


def parse_connector(text: str) -> tuple[str, str, int]:
    if ">" not in text:
        if text not in CONNECTOR:
            raise DataError(f"unknown connector '{text}'")
        return text, text, 0
    early, rest = text.split(">", 1)
    late, _, year = rest.partition("@")
    if early not in CONNECTOR or late not in CONNECTOR or not year.isdigit():
        raise DataError(f"bad connector '{text}', expected like vag2x2>obd2@1996")
    return early, late, int(year)


def check_text(text: str, where: str) -> None:
    for bad in ("\u2014", "\u2013"):  # em dash, en dash
        if bad in text:
            raise DataError(f"{where}: no long dashes in user-visible text")
    if '"' in text or "\\" in text:
        raise DataError(f"{where}: quotes and backslashes are not allowed")


def rust(text: str) -> str:
    return '"' + text + '"'


def rust_power(power: str) -> str:
    """Powertrain flags as Rust constants, from the table names (PtPetrol -> PT_PETROL)."""
    out = []
    for name in power.split(" | "):
        out.append("PT_" + "".join("_" + c if c.isupper() else c.upper() for c in name[2:]).lstrip("_"))
    return " | ".join(out)


def write_rust(brands: dict, rows: list[dict]) -> None:
    lines = [
        "// Generated by tools/gen_vehicles.py from data/vehicles/*.csv, do not edit by hand.",
        f"// {len(brands)} brands, {len(rows)} model generations.",
        "",
        "use crate::catalog::{",
        "    BodyType as B, BrandInfo, Connector as C, PT_DIESEL, PT_ELECTRIC, PT_GAS, PT_HYBRID,",
        "    PT_PETROL, PT_PLUG_IN, SocketLocation as L, VehicleGeneration,",
        "};",
        "use crate::vehicles::BusKind as K;",
        "use caruspex_i18n::Text;",
        "",
        "#[rustfmt::skip]",
        "pub static BRANDS: &[BrandInfo] = &[",
    ]
    for name in sorted(brands, key=str.casefold):
        b = brands[name]
        lines.append(
            f"    BrandInfo {{ name: {rust(name)}, country: Text::new({rust(b['de'])}, {rust(b['en'])}), "
            f"group: {rust(b['group'])} }},")
    lines += ["];", "", "#[rustfmt::skip]", "pub static GENERATIONS: &[VehicleGeneration] = &["]
    for r in rows:
        lines.append(
            f"    VehicleGeneration {{ brand: {rust(r['brand'])}, model: {rust(r['model'])}, "
            f"generation: {rust(r['generation'])}, year_from: {r['from']}, year_to: {r['to']}, "
            f"body: B::{r['body']}, powertrains: {rust_power(r['power'])}, "
            f"connector: C::{r['early']}, late_connector: C::{r['late']}, connector_change: {r['switch']}, "
            f"bus: K::{r['bus']}, protocol_order: {rust(r['order'])}, location: L::{r['location']}, "
            f"note: Text::new({rust(r['note_de'])}, {rust(r['note_en'])}), "
            f"vin_model_codes: {rust(r['vin'])}, aliases: {rust(r['aliases'])}, us_market: {r['us']} }},")
    lines += ["];", ""]
    os.makedirs(os.path.dirname(TARGET_RS), exist_ok=True)
    io.open(TARGET_RS, "w", encoding="utf-8", newline="\n").write("\n".join(lines))


def load_brands() -> dict[str, dict[str, str]]:
    brands: dict[str, dict[str, str]] = {}
    with io.open(os.path.join(DATA, "brands.csv"), encoding="utf-8", newline="") as f:
        rows = [r for r in csv.reader(f) if r and not r[0].startswith("#")]
    header = rows[0]
    if header != ["name", "country_de", "country_en", "group", "market"]:
        raise DataError("brands.csv: unexpected header")
    for r in rows[1:]:
        r = (r + [""] * 5)[:5]
        name, de, en, group, market = (c.strip() for c in r)
        if name in brands:
            raise DataError(f"brands.csv: {name} twice")
        if group not in ("", "vag", "bmw", "mb", "ford", "gm", "chrysler"):
            raise DataError(f"brands.csv: {name}: unknown group {group}")
        if market not in ("eu", "na"):
            raise DataError(f"brands.csv: {name}: market must be eu or na")
        check_text(de + en, f"brands.csv {name}")
        brands[name] = {"de": de, "en": en, "group": group, "market": market}
    return brands


def load_rows(brands: dict[str, dict[str, str]]) -> list[dict]:
    out: list[dict] = []
    seen: set[tuple] = set()
    files = sorted(n for n in os.listdir(DATA) if n.endswith(".csv") and n != "brands.csv")
    for name in files:
        path = os.path.join(DATA, name)
        market_override = ""
        header_seen = False
        with io.open(path, encoding="utf-8", newline="") as f:
            for lineno, raw in enumerate(csv.reader(f), start=1):
                if not raw or not "".join(raw).strip():
                    continue
                first = raw[0].strip()
                if first.startswith("#@"):
                    key, _, value = first[2:].strip().partition("=")
                    if key.strip() != "market" or value.strip() not in ("", "eu", "na"):
                        raise DataError(f"{name}:{lineno}: unknown directive {first}")
                    market_override = value.strip()
                    continue
                if first.startswith("#"):
                    continue
                if not header_seen:
                    if [c.strip() for c in raw] != COLUMNS[:len(raw)] or len(raw) < 7:
                        raise DataError(f"{name}: header must be {','.join(COLUMNS)}")
                    header_seen = True
                    continue
                where = f"{name}:{lineno}"
                if len(raw) > len(COLUMNS):
                    raise DataError(f"{where}: too many columns")
                r = dict(zip(COLUMNS, [c.strip() for c in raw] + [""] * (len(COLUMNS) - len(raw))))
                try:
                    out.append(build_row(r, brands, market_override, where))
                except DataError as e:
                    raise DataError(f"{where}: {e}") from None
                key = (r["brand"], r["model"], r["generation"], r["from"])
                if key in seen:
                    raise DataError(f"{where}: duplicate row {key}")
                seen.add(key)
    return out


def build_row(r: dict, brands: dict, market_override: str, where: str) -> dict:
    brand = r["brand"]
    if brand not in brands:
        raise DataError(f"brand '{brand}' missing in brands.csv")
    info = brands[brand]
    group = info["group"]
    market = market_override or info["market"]
    if not r["model"]:
        raise DataError("model is empty")
    try:
        y0 = int(r["from"])
        y1 = int(r["to"] or "0")
    except ValueError:
        raise DataError("years must be numbers") from None
    if not 1930 <= y0 <= CURRENT_YEAR + 1:
        raise DataError(f"implausible start year {y0}")
    if y1 and (y1 < y0 or y1 > CURRENT_YEAR + 1):
        raise DataError(f"implausible end year {y1}")
    y1eff = y1 or CURRENT_YEAR
    if r["body"] not in BODY:
        raise DataError(f"unknown body '{r['body']}'")
    power = r["power"]
    if not power or any(c not in POWER for c in power):
        raise DataError(f"bad power '{power}'")

    connector = r["connector"] or default_connector(group, market, y0, y1eff)
    early, late, switch = parse_connector(connector)
    if switch and not (y0 < switch <= y1eff):
        raise DataError(f"connector change {switch} outside {y0}-{y1eff}")

    if "obd2" in (early, late):
        obd_from = y0 if early == "obd2" else switch
        bus = r["bus"] or default_bus(group, market, power, obd_from, y1eff)
        if bus not in BUS:
            raise DataError(f"unknown bus '{bus}'")
        order = r["protocols"] or default_order(group, market, bus)
    else:
        if r["bus"] or r["protocols"]:
            raise DataError("bus and protocols make no sense without an OBD-II socket")
        bus = "kline"
        order = ""
    if any(c not in "123456789" for c in order) or len(set(order)) != len(order):
        raise DataError(f"bad protocol order '{order}'")

    location = r["location"]
    if location not in LOCATION:
        raise DataError(f"unknown location '{location}'")
    if not location and early in ("bmw20", "mb38") and late == early:
        location = "engine"

    if bool(r["note_de"]) != bool(r["note_en"]):
        raise DataError("note needs German and English")
    for col in ("model", "generation", "note_de", "note_en", "aliases", "vin"):
        check_text(r[col], where)

    return {
        "brand": brand, "model": r["model"], "generation": r["generation"],
        "from": y0, "to": y1, "body": BODY[r["body"]],
        "power": " | ".join(POWER[c] for c in "PDHXEG" if c in power),
        "early": CONNECTOR[early], "late": CONNECTOR[late], "switch": switch,
        "bus": BUS[bus], "order": order, "location": LOCATION[location],
        "note_de": r["note_de"], "note_en": r["note_en"],
        "vin": " ".join(r["vin"].split()), "aliases": r["aliases"],
        "us": "true" if market == "na" else "false",
    }


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    try:
        brands = load_brands()
        rows = load_rows(brands)
    except DataError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1

    used = {r["brand"] for r in rows}
    unused = sorted(set(brands) - used)
    if unused:
        print(f"note: brands without rows: {', '.join(unused)}")

    write_rust(brands, rows)
    print(f"{TARGET_RS}\n{len(brands)} brands, {len(rows)} generations")
    return 0


if __name__ == "__main__":
    sys.exit(main())
