# Extracts the vehicle catalog, the WMI table and the manufacturer fault code
# tables from the generated C++ sources into JSON for the browser preview
# (src/api/mock/data/catalog.json and dtc-brands.json). The real backend reads
# the same tables in Rust; the preview only needs a faithful copy.
#
# Run from anywhere:  py -I app/scripts/extract-catalog.py
# Re-run it whenever tools/gen_vehicles.py, tools/gen_dtc.py or Vin.cpp change.
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
DATA = ROOT / "src" / "data"
OUT = HERE.parent / "src" / "api" / "mock" / "data"

# Enum orders of VehicleCatalog.h, Vehicles.h, Vin.h, Severity.h, DtcDatabase.h.
BODY = ["Hatchback", "Sedan", "Estate", "Suv", "Van", "Coupe", "Convertible", "Pickup", "Mpv", "Microcar", "Roadster"]
CONNECTOR = ["None", "Obd2", "Vag2x2", "Bmw20", "Mb38", "Other"]
LOCATION = ["Unknown", "UnderDashLeft", "UnderSteeringColumn", "FootwellCover", "FuseBox", "CentreConsole",
            "BehindAshtray", "PassengerSide", "EngineBay", "RearConsole"]
BUS = ["KLine", "Can", "Pwm", "Vpw", "Mixed"]
POWERTRAIN = {"PtPetrol": 1, "PtDiesel": 2, "PtHybrid": 4, "PtPlugIn": 8, "PtElectric": 16, "PtGas": 32}
YEAR_RULE = {"kStd": 0, "kUnk": 1, "kNoYear": 2}
CODE_RULE = {"kNone": 0, "kVag": 1, "kMb": 2, "kPor": 3, "kTesla": 4}
SEVERITY = ["Info", "Low", "Medium", "High", "Critical"]
SYSTEM = ["Unknown", "FuelAir", "AirIntake", "Throttle", "Ignition", "ValveTiming", "Injection", "Turbo", "Cooling",
          "Lubrication", "Exhaust", "Egr", "Evap", "DieselAftertreatment", "Charging", "EngineControl",
          "Transmission", "Hybrid", "Network", "Brakes", "Steering", "Chassis", "Airbag", "Body", "Immobilizer",
          "Climate"]

STR = r'"((?:[^"\\]|\\.)*)"'


def unescape(s):
    return s.encode("utf-8").decode("unicode_escape").encode("latin-1").decode("utf-8") if "\\" in s else s


def catalog():
    text = (DATA / "VehicleCatalog.gen.cpp").read_text(encoding="utf-8")
    brand_re = re.compile(r"^\s*\{" + STR + r", \{" + STR + r", " + STR + r"\}, " + STR + r"\},\s*$")
    gen_re = re.compile(
        r"^\s*\{" + STR + r", " + STR + r", " + STR + r", (\d+), (\d+), B::(\w+), ([\w |]+), C::(\w+), C::(\w+), (\d+), "
        r"K::(\w+), " + STR + r", L::(\w+), \{" + STR + r", " + STR + r"\}, " + STR + r", " + STR + r", (true|false)\},\s*$")
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
    text = (DATA / "Vin.cpp").read_text(encoding="utf-8")
    texts = {}
    for m in re.finditer(r"constexpr Text (\w+) = \{" + STR + r", " + STR + r"\};", text):
        texts[m.group(1)] = [m.group(2), m.group(3)]
    wmi_re = re.compile(r"^\s*\{" + STR + r", " + STR + r", " + STR + r", (\w+), (\w+), (\{\}|\w+)\},")
    country_re = re.compile(r"^\s*\{'(.)', '(.)', '(.)', \{" + STR + r", " + STR + r"\}\},")
    wmi, countries = [], []
    for line in text.splitlines():
        m = wmi_re.match(line)
        if m:
            g = m.groups()
            country = texts[g[5]] if g[5] != "{}" else None
            wmi.append([g[0], g[1], g[2], YEAR_RULE[g[3]], CODE_RULE[g[4]], country])
            continue
        m = country_re.match(line)
        if m:
            countries.append(list(m.groups()))
    if len(wmi) < 300 or len(countries) < 70:
        sys.exit(f"vin parse looks short: {len(wmi)} WMI entries, {len(countries)} ranges")
    return wmi, countries


def brand_dtcs():
    text = (DATA / "DtcBrands.gen.cpp").read_text(encoding="utf-8")
    rec_re = re.compile(r"^\s*\{" + STR + r", \{" + STR + r", " + STR + r"\}, Severity::(\w+), DtcSystem::(\w+), (\w+), (true|false)\},")
    fam_re = re.compile(r"^\s*\{" + STR + r", \{" + STR + r", " + STR + r"\}, " + STR + r", (?:std::span<const Record>\((\w+)\)|\{\})\},")
    tables, current = {}, None
    families = []
    for line in text.splitlines():
        m = re.match(r"^constexpr Record (\w+)\[\] = \{", line)
        if m:
            current = m.group(1)
            tables[current] = []
            continue
        m = rec_re.match(line)
        if m and current:
            g = m.groups()
            detail = -1 if g[5] == "kNoDetail" else int(g[5])
            tables[current].append([g[0], unescape(g[1]), unescape(g[2]), SEVERITY.index(g[3]), SYSTEM.index(g[4]), detail, g[6] == "true"])
            continue
        m = fam_re.match(line)
        if m:
            g = m.groups()
            families.append([g[0], unescape(g[1]), unescape(g[2]), g[3], tables.get(g[4], []) if g[4] else []])
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
