# Extracts the vehicle catalog, the WMI table and the manufacturer fault code
# tables from the Rust sources into JSON for the browser preview
# (src/api/mock/data/catalog.json and dtc-brands.json). The real backend reads
# the same tables; the preview only needs a faithful copy. Sources:
# crates/caruspex-vehicles/src/generated/catalog.rs, crates/caruspex-vehicles/src/vin.rs
# and crates/caruspex-data/src/generated/dtc_brands.rs.
#
# Run from anywhere:  py -I app/scripts/extract-catalog.py
# Re-run it whenever tools/gen_vehicles.py, tools/gen_dtc.py or vin.rs change.
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
VEHICLES = ROOT / "crates" / "caruspex-vehicles" / "src"
DTC_BRANDS = ROOT / "crates" / "caruspex-data" / "src" / "generated" / "dtc_brands.rs"
OUT = HERE.parent / "src" / "api" / "mock" / "data"

# Enum orders of catalog.rs, vin.rs, severity.rs and dtc/mod.rs.
BODY = ["Hatchback", "Sedan", "Estate", "Suv", "Van", "Coupe", "Convertible", "Pickup", "Mpv", "Microcar", "Roadster"]
CONNECTOR = ["None", "Obd2", "Vag2x2", "Bmw20", "Mb38", "Other"]
LOCATION = ["Unknown", "UnderDashLeft", "UnderSteeringColumn", "FootwellCover", "FuseBox", "CentreConsole",
            "BehindAshtray", "PassengerSide", "EngineBay", "RearConsole"]
BUS = ["KLine", "Can", "Pwm", "Vpw", "Mixed"]
POWERTRAIN = {"PT_PETROL": 1, "PT_DIESEL": 2, "PT_HYBRID": 4, "PT_PLUG_IN": 8, "PT_ELECTRIC": 16, "PT_GAS": 32}
YEAR_RULE = {"STD": 0, "UNK": 1, "NO_YEAR": 2}
CODE_RULE = {"NO_CODE": 0, "VAG": 1, "MB": 2, "POR": 3, "TESLA": 4}
SEVERITY = ["Info", "Low", "Medium", "High", "Critical"]
SYSTEM = ["Unknown", "FuelAir", "AirIntake", "Throttle", "Ignition", "ValveTiming", "Injection", "Turbo", "Cooling",
          "Lubrication", "Exhaust", "Egr", "Evap", "DieselAftertreatment", "Charging", "EngineControl",
          "Transmission", "Hybrid", "Network", "Brakes", "Steering", "Chassis", "Airbag", "Body", "Immobilizer",
          "Climate"]

STR = r'"((?:[^"\\]|\\.)*)"'


def unescape(s):
    return s.encode("utf-8").decode("unicode_escape").encode("latin-1").decode("utf-8") if "\\" in s else s


def catalog():
    text = (VEHICLES / "generated" / "catalog.rs").read_text(encoding="utf-8")
    brand_re = re.compile(r"^\s*BrandInfo \{ name: " + STR + r", country: Text::new\(" + STR + r", " + STR
                          + r"\), group: " + STR + r" \},\s*$")
    gen_re = re.compile(
        r"^\s*VehicleGeneration \{ brand: " + STR + r", model: " + STR + r", generation: " + STR
        + r", year_from: (\d+), year_to: (\d+), body: B::(\w+), powertrains: ([\w |]+), connector: C::(\w+), "
        r"late_connector: C::(\w+), connector_change: (\d+), bus: K::(\w+), protocol_order: " + STR
        + r", location: L::(\w+), note: Text::new\(" + STR + r", " + STR + r"\), vin_model_codes: " + STR
        + r", aliases: " + STR + r", us_market: (true|false) \},\s*$")
    brands, gens = [], []
    for line in text.splitlines():
        m = gen_re.match(line)
        if m:
            g = m.groups()
            pt = 0
            for flag in g[6].split("|"):
                pt |= POWERTRAIN[flag.strip()]
            gens.append([unescape(g[0]), unescape(g[1]), unescape(g[2]), int(g[3]), int(g[4]), BODY.index(g[5]), pt,
                         CONNECTOR.index(g[7]), CONNECTOR.index(g[8]), int(g[9]), BUS.index(g[10]), g[11],
                         LOCATION.index(g[12]), unescape(g[13]), unescape(g[14]), unescape(g[15]), unescape(g[16]),
                         g[17] == "true"])
            continue
        m = brand_re.match(line)
        if m:
            brands.append([unescape(x) for x in m.groups()])
    header = re.search(r"(\d+) brands, (\d+) model generations", text)
    if not header or int(header.group(1)) != len(brands) or int(header.group(2)) != len(gens):
        sys.exit(f"catalog parse mismatch: {len(brands)} brands, {len(gens)} generations, header {header and header.groups()}")
    return brands, gens


def vin_tables():
    text = (VEHICLES / "vin.rs").read_text(encoding="utf-8")
    texts = {}
    for m in re.finditer(r"^const (\w+): Text = Text::new\(" + STR + r", " + STR + r"\);", text, re.M):
        texts[m.group(1)] = [m.group(2), m.group(3)]
    wmi_re = re.compile(r"^\s*w\(" + STR + r", " + STR + r", " + STR + r", (\w+), (\w+), (\w+)\),")
    country_re = re.compile(r"^\s*cr\('(.)', '(.)', '(.)', " + STR + r", " + STR + r"\),")
    wmi, countries = [], []
    for line in text.splitlines():
        m = wmi_re.match(line)
        if m:
            g = m.groups()
            country = texts[g[5]] if g[5] != "NO_COUNTRY" else None
            wmi.append([g[0], g[1], g[2], YEAR_RULE[g[3]], CODE_RULE[g[4]], country])
            continue
        m = country_re.match(line)
        if m:
            countries.append(list(m.groups()))
    if len(wmi) < 300 or len(countries) < 70:
        sys.exit(f"vin parse looks short: {len(wmi)} WMI entries, {len(countries)} ranges")
    return wmi, countries


def brand_dtcs():
    text = (DTC_BRANDS).read_text(encoding="utf-8")
    rec_re = re.compile(r"^\s*r\(" + STR + r", " + STR + r", " + STR + r", Severity::(\w+), DtcSystem::(\w+), (\w+), (true|false)\),")
    fam_re = re.compile(r"^\s*Family \{ key: " + STR + r", label: Text::new\(" + STR + r", " + STR + r"\), brands: " + STR + r", records: (\w+|&\[\]) \},")
    tables, current = {}, None
    families = []
    for line in text.splitlines():
        m = re.match(r"^static (\w+): &\[Record\] = &\[", line)
        if m:
            current = m.group(1)
            tables[current] = []
            continue
        if line.startswith("pub static FAMILIES"):
            current = None
            continue
        m = rec_re.match(line)
        if m and current:
            g = m.groups()
            detail = -1 if g[5] == "NO_DETAIL" else int(g[5])
            tables[current].append([g[0], unescape(g[1]), unescape(g[2]), SEVERITY.index(g[3]), SYSTEM.index(g[4]), detail, g[6] == "true"])
            continue
        m = fam_re.match(line)
        if m:
            g = m.groups()
            families.append([g[0], unescape(g[1]), unescape(g[2]), g[3], tables.get(g[4], []) if g[4] != "&[]" else []])
    total = sum(len(f[4]) for f in families)
    header = re.search(r"(\d+) manufacturer-specific codes in (\d+) brand families", text)
    if not header or int(header.group(1)) != total or int(header.group(2)) != len(families):
        sys.exit(f"dtc parse mismatch: {total} codes in {len(families)} families")
    return families


def main():
    brands, gens = catalog()
    wmi, countries = vin_tables()
    OUT.mkdir(parents=True, exist_ok=True)
    with open(OUT / "catalog.json", "w", encoding="utf-8") as f:
        json.dump({"brands": brands, "generations": gens, "wmi": wmi, "countries": countries}, f,
                  ensure_ascii=False, separators=(",", ":"))
    families = brand_dtcs()
    with open(OUT / "dtc-brands.json", "w", encoding="utf-8") as f:
        json.dump({"families": families}, f, ensure_ascii=False, separators=(",", ":"))
    print(f"{len(brands)} brands, {len(gens)} generations, {len(wmi)} WMI entries, {len(countries)} country ranges, "
          f"{sum(len(x[4]) for x in families)} brand codes in {len(families)} families")


if __name__ == "__main__":
    main()
