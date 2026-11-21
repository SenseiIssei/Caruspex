"""Turns the OBDb signal sets into Caruspex PID packs.

OBDb (github.com/OBDb, CC BY-SA 4.0) keeps one repository per car model
("Toyota-Prius", "Hyundai-IONIQ-5") with the manufacturer values the car
answers: which control unit to ask (header, receive address), what to ask
(service 01, 21 or 22 with an identifier) and where each value sits in the
answer (bit index, length, byte order, sign, scaling), each command limited
to model years where needed.

    py -I tools/gen_obdb.py --fetch      download or refresh the signal sets into .cache/obdb/
    py -I tools/gen_obdb.py              convert the cache into data/obdb/
    py -I tools/gen_obdb.py --fixtures   convert the test fixtures only (offline, used by the CI)

The download needs network and a GitHub token for the API rate limit (GH_TOKEN,
GITHUB_TOKEN or `gh auth token`); a repository whose last push did not change
is not downloaded again. The cache (about 50 MB) is not committed, so the CI
cannot rebuild the packs; it only checks the conversion on the fixtures. To
refresh the packs: --fetch, then convert, run the tests, commit data/obdb/.
CARUSPEX_CACHE moves the cache from .cache/ to another folder (it gets an
obdb/ inside). The commit of every repository is recorded in the cache and
named in ATTRIBUTION.md.

Make fallback. OBDb keeps one repository per make too ("BYD", "Skoda"),
merged by OBDb from that make's model repositories, and its tools use it
for a model whose own signal sets are empty. Such a model may have recorded
answers of a real car in tests/test_cases/<year>/commands/. Those answers
show that the car answers a command and that its bytes decode, but the
meaning still comes from other models of the make, and merged sets of
large makes read the same identifier differently for different models.
So the fallback is used only for the models in FALLBACK, whose recorded
values were checked by hand for being consistent with each other and with
the car (the reason is written next to each), and only as far as the
answers confirm it: a command of the make set is taken when the model has
answers for it and the make set reads its bytes in one way only, a signal
of it when OBDb's test cases give its value and the port of OBDb's decoder
below gives the same value on every recorded answer. Everything else of
the make set stays out ("fallback unconfirmed", "fallback ambiguous").

Conversion. Every OBDb signal becomes a Torque equation over the data bytes
A, B, ... of the answer (A is the first byte after the echoed service and
identifier, which is where OBDb's bit index 0 is too). The bits are taken
exactly as OBDb's reference decoder does (.schemas/python/can/signals.py):
bit 0 is the top bit of A, blsb reverses the bytes of a value longer than 8
bits, sign is two's complement over the length, the value is
raw * mul / div + add, and it is clamped to [min, max] when max > min (the
equation clamps only where the field can leave that range). Every converted
signal is checked against a port of that decoder on eight answers; the
conversion stops on the first difference. A signal that cannot be written
exactly is skipped and counted, never approximated:

    enumeration   the value is a text from a list (fmt.map)
    ascii         the value is text
    null bound    OBDb shows no value on some answers (nullmin, nullmax)
    too long      more than 40 bits, or a byte beyond ZZ
    extended      the command needs extended addressing (eax)
    session       the command needs a diagnostic session (din, dout), except
                  the extended session (din 03) of a Volkswagen group engine
                  at 7E0: those commands are taken, their C line names the
                  session "03" and the app switches the engine into it only
                  while such a value is read (SESSION_MAKES below)
    k-line        the command is for ISO 9141 or 14230
    functional    asked at 7DF or DB33 but answered by one control unit (rax)

Model years: each command may hold a year filter, and a repository may hold
signal sets for single years ("2017-2017.json"). The years a repository
covers (its generations.yaml and the years of its test cases) are cut into
ranges in which the same commands apply, and every range becomes a pack
"obdb-<repo>-<from>-<to>" (x = open). Of commands with the same address and
request that apply in the same year, the last one in the file wins, as in
OBDb.

Matching: the OBDb make and model are mapped to the brand and model of the
vehicle catalog (data/vehicles/): by name, by a search alias of the catalog,
or without trim words at the end, and through data/obdb_aliases.csv where
the names differ. The app offers the packs whose catalog brand, model and
years fit the detected car.

Output, all under CC BY-SA 4.0 and kept apart from the MIT code:
data/obdb/obdb-packs.z, a zlib-compressed text file read by
crates/caruspex-record/src/obdb.rs, and data/obdb/ATTRIBUTION.md. Lines of
the text, tab separated:

    O <vehicles> <repositories> <signals> <skipped> <packs>     once, the totals
    R <repo> <make> <model> <from-to,...> <brand|model;...> <ev>  one car, its packs
    C <pack mask> <request> <header> <response> [<session>]      a command
    V <name> <ev short name> <equation> <min> <max> <unit>        a value of it

C lines belong to the R line above them, V lines to the C line above them;
bit i of the mask says whether the command is in the i-th pack of the car.
"""

from __future__ import annotations

import collections
import concurrent.futures
import csv
import decimal
import io
import json
import math
import os
import random
import re
import subprocess
import unicodedata
import sys
import urllib.parse
import urllib.request
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CACHE = os.path.join(os.environ.get("CARUSPEX_CACHE") or os.path.join(ROOT, ".cache"), "obdb")
DATA = os.path.join(ROOT, "data", "obdb")
TARGET = os.path.join(DATA, "obdb-packs.z")
ALIASES = os.path.join(ROOT, "data", "obdb_aliases.csv")
ATTRIBUTION = os.path.join(DATA, "ATTRIBUTION.md")
VEHICLES = os.path.join(ROOT, "data", "vehicles")

API = "https://api.github.com"
RAW = "https://raw.githubusercontent.com"
ORG = "OBDb"


# ---------------------------------------------------------------------------
# Download
# ---------------------------------------------------------------------------

def token() -> str:
    for name in ("GH_TOKEN", "GITHUB_TOKEN"):
        if os.environ.get(name):
            return os.environ[name]
    try:
        out = subprocess.run(["gh", "auth", "token"], capture_output=True, text=True, timeout=30)
        return out.stdout.strip() if out.returncode == 0 else ""
    except (OSError, subprocess.SubprocessError):
        return ""


def fetch(url: str, auth: str = "") -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "Caruspex OBDb import"})
    if auth and url.startswith(API):
        request.add_header("Authorization", f"Bearer {auth}")
    for attempt in range(4):
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                return response.read()
        except urllib.error.HTTPError as e:
            if e.code == 404 or attempt == 3:
                raise
        except (urllib.error.URLError, TimeoutError):
            if attempt == 3:
                raise
    raise RuntimeError("unreachable")


def list_repos(auth: str) -> list[dict]:
    repos = []
    page = 1
    while True:
        batch = json.loads(fetch(f"{API}/orgs/{ORG}/repos?per_page=100&type=public&page={page}", auth))
        if not batch:
            break
        repos += [{"name": r["name"], "branch": r["default_branch"], "pushed_at": r["pushed_at"],
                   "archived": r.get("archived", False)} for r in batch]
        page += 1
    return sorted(repos, key=lambda r: r["name"].lower())


def is_vehicle_repo(name: str) -> bool:
    # Make-Model; the make repositories ("Toyota"), SAEJ1979 and the tools
    # have no model part.
    return bool(re.match(r"^[A-Za-z0-9]+(-[A-Za-z0-9.]+)+$", name)) and name not in ("vscode-obdb",)


def is_make_repo(name: str) -> bool:
    # "BYD", "Skoda", "Land-Rover": the merged make sets used as fallback.
    return (bool(re.match(r"^[A-Za-z]+$", name)) or name in MULTI_WORD_MAKES) and name != "SAEJ1979"


def safe_name(path: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]", "_", path)


def has_commands(folder: str, signalsets: list[str]) -> bool:
    for path in signalsets:
        data = json.load(io.open(os.path.join(folder, safe_name(path)), encoding="utf-8"))
        if data.get("commands"):
            return True
    return False


# Models that take the make fallback, with the check that allows it.
FALLBACK = {
    # The BYD set is merged from the Sealion 6 alone. On the recorded
    # answers of the Atto 3 (model years 2022 and 2025) the state of charge
    # of 22 0005 (23 to 92 %) matches the separate nominal state of charge
    # of 22 1FFC (22.9 to 92.2 %), the pack voltage (352 to 431 V) and the
    # cell voltages (2.74 to 3.35 V) fit its LFP pack, the factory capacity
    # (about 150 Ah) fits 60 kWh, and the four wheel speeds agree. OVMS reads
    # the Atto 3 at the same address with the same layout for 22 0005
    # (state of charge), 0008 (voltage), 0009 (current), 002F, 0031 and 0032
    # (cell and pack temperatures) (vehicle_byd_atto3.cpp, MIT, commit
    # 7470869a2db5).
    "BYD-Atto-3": "BYD",
}

TEST_FILE = re.compile(r"^tests/test_cases/\d{4}/commands/[^/]+\.yaml$")


def fetch_repo(repo: dict, auth: str) -> tuple[str, str]:
    """Brings one repository's cache up to date. Returns (name, what happened)."""
    name = repo["name"]
    folder = os.path.join(CACHE, name)
    meta_path = os.path.join(folder, "meta.json")
    old = json.load(open(meta_path, encoding="utf-8")) if os.path.exists(meta_path) else {}
    if old.get("pushed_at") == repo["pushed_at"] and old.get("complete") and old.get("commit"):
        return name, "unchanged"
    # The commit first and the tree of exactly that commit, so the files and
    # the commit named in ATTRIBUTION.md belong together.
    commit = json.loads(fetch(f"{API}/repos/{ORG}/{name}/commits/{repo['branch']}", auth))["sha"]
    tree = json.loads(fetch(f"{API}/repos/{ORG}/{name}/git/trees/{commit}?recursive=1", auth))
    paths = {t["path"]: t["sha"] for t in tree.get("tree", []) if t["type"] == "blob"}
    signalsets = sorted(p for p in paths if re.match(r"^signalsets/v3/[^/]+\.json$", p))
    years = sorted({int(m.group(1)) for p in paths
                    for m in [re.match(r"^tests/test_cases/(\d{4})/", p)] if m})
    wanted = signalsets + [p for p in ("generations.yaml",) if p in paths]
    os.makedirs(folder, exist_ok=True)
    blobs = {}

    def get(path: str) -> None:
        local = os.path.join(folder, safe_name(path))
        if old.get("blobs", {}).get(path) != paths[path] or not os.path.exists(local):
            data = fetch(f"{RAW}/{ORG}/{name}/{commit}/{urllib.parse.quote(path)}")
            with open(local, "wb") as f:
                f.write(data)
        blobs[path] = paths[path]

    for path in wanted:
        get(path)
    # The recorded answers are only needed where the make fallback may apply;
    # they are kept for every model without own signals, so a model added to
    # FALLBACK later has them without a new download.
    tests = []
    if is_vehicle_repo(name) and not has_commands(folder, signalsets):
        tests = sorted(p for p in paths if TEST_FILE.match(p))
        for path in tests:
            get(path)
    meta = {"name": name, "branch": repo["branch"], "pushed_at": repo["pushed_at"], "commit": commit,
            "signalsets": signalsets, "tests": tests, "blobs": blobs, "test_years": years,
            "complete": True}
    with open(meta_path, "w", encoding="utf-8", newline="\n") as f:
        json.dump(meta, f, indent=1)
    return name, "downloaded"


def do_fetch() -> int:
    auth = token()
    if not auth:
        print("warning: no GitHub token (GH_TOKEN or gh auth token), the API allows 60 calls an hour",
              file=sys.stderr)
    os.makedirs(CACHE, exist_ok=True)
    repos = [r for r in list_repos(auth) if is_vehicle_repo(r["name"]) or is_make_repo(r["name"])]
    with open(os.path.join(CACHE, "repos.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(repos, f, indent=1)
    def one(repo: dict) -> tuple[str, str]:
        try:
            return fetch_repo(repo, auth)
        except Exception as e:  # one bad repository must not stop the rest
            return repo["name"], f"failed ({type(e).__name__}: {e})"

    counts: collections.Counter = collections.Counter()
    with concurrent.futures.ThreadPoolExecutor(max_workers=12) as pool:
        for name, what in pool.map(one, repos):
            if what.startswith("failed"):
                print(f"{name}: {what}", file=sys.stderr)
                what = "failed"
            counts[what] += 1
    print(f"{len(repos)} vehicle repositories: " + ", ".join(f"{n} {w}" for w, n in sorted(counts.items())))
    return 0


# ---------------------------------------------------------------------------
# Signals to Torque equations
# ---------------------------------------------------------------------------

MAX_BYTES = 26 + 26 * 26  # A .. ZZ

# OBDb unit -> unit shown. "ascii" (text) has no number and is skipped.
UNITS = {
    "ampereHours": "Ah", "amps": "A", "bars": "bar", "celsius": "°C", "centimeters": "cm",
    "coulombs": "C", "degrees": "°", "fahrenheit": "°F", "feet": "ft", "framesPerSecond": "fps",
    "gallons": "gal", "gallonsPerHour": "gal/h", "gigahertz": "GHz", "gramsPerLiter": "g/l",
    "gramsPerSecond": "g/s", "gravity": "g", "hertz": "Hz", "hex": "", "hours": "h",
    "inches": "in", "inchPound": "in·lb", "joules": "J", "kelvin": "K", "kiloampereHours": "kAh",
    "kiloamps": "kA", "kilogramsPerHour": "kg/h", "kilohertz": "kHz", "kilojoules": "kJ",
    "kilometers": "km", "kilometersPerHour": "km/h", "kiloohms": "kΩ", "kilopascal": "kPa",
    "kilovolts": "kV", "kilowattHours": "kWh", "kilowattHoursPer100Kilometers": "kWh/100 km",
    "kilowattHoursPer100Miles": "kWh/100 mi", "kilowatts": "kW", "liters": "l",
    "litersPerHour": "l/h", "megahertz": "MHz", "megaohms": "MΩ", "meters": "m",
    "metersPerSecond": "m/s", "metersPerSecondSquared": "m/s²", "microhertz": "µHz",
    "microohms": "µΩ", "miles": "mi", "milesPerKilowattHour": "mi/kWh", "milesPerHour": "mph",
    "milliampereHours": "mAh", "milliamps": "mA", "milligramsPerDeciliter": "mg/dl",
    "milligramsPerStroke": "mg/stroke", "millihertz": "mHz", "millimeters": "mm",
    "milliohms": "mΩ", "milliseconds": "ms", "millivolts": "mV", "milliwatts": "mW",
    "minutes": "min", "nanohertz": "nHz", "newtonMeters": "Nm", "normal": "", "noyes": "",
    "offon": "", "ohms": "Ω", "onoff": "", "percent": "%", "poundFoot": "lb·ft", "psi": "psi",
    "radians": "rad", "rpm": "1/min", "scalar": "", "seconds": "s", "terahertz": "THz",
    "unknown": "", "volts": "V", "watts": "W", "yards": "yd", "yesno": "",
}

# OBDb suggested metric -> the short name the EV battery page looks for, with
# the unit it expects.
EV_METRICS = {
    "stateOfCharge": ("SOC", "percent"),
    "stateOfHealth": ("SOH", "percent"),
    "tractionBatteryVoltage": ("Batt V", "volts"),
    "tractionBatteryCurrent": ("Batt A", "amps"),
    "starterBatteryVoltage": ("12V", "volts"),
}
EV_PACK_METRICS = {"stateOfCharge", "stateOfHealth", "tractionBatteryVoltage",
                   "tractionBatteryCurrent", "tractionBatteryCapacity", "isCharging", "pluggedIn"}
# OBDb signals without a suggestedMetric whose name and unit say plainly
# which one it is ("High voltage battery state of charge", %; "Battery total
# voltage", V). The current of the BYD set counts as discharging when
# positive, as the EV page wants it: OVMS reads the same field of the Atto 3
# into its battery current, which is positive on discharge
# (vehicle/OVMS.V3/components/vehicle_byd_atto3/src/vehicle_byd_atto3.cpp,
# MIT, commit 7470869a2db5). OBDb's own Sealion 6 set says nothing about the
# sign, so its current stays without a short name.
METRIC_BY_ID = {
    "BYD_HVBAT_ST_CHG": "stateOfCharge", "SEALION_HVBAT_ST_CHG": "stateOfCharge",
    "BYD_BAT_TOT_V": "tractionBatteryVoltage", "SEALION_BAT_TOT_V": "tractionBatteryVoltage",
    "BYD_BAT_TOT_CURRENT": "tractionBatteryCurrent",
}

EPSILON = sys.float_info.epsilon


class Skip(Exception):
    def __init__(self, reason: str):
        super().__init__(reason)
        self.reason = reason


def byte_name(index: int) -> str:
    if index < 26:
        return chr(65 + index)
    index -= 26
    return chr(65 + index // 26) + chr(65 + index % 26)


def number_text(value) -> str:
    """A number as the equation parser reads it: digits and a point, no exponent.
    Written from the shortest text that gives back the same double, so the
    parser (correctly rounded) gets exactly OBDb's value."""
    if isinstance(value, bool):
        raise Skip("number")
    if isinstance(value, int):
        return str(value)
    text = format(decimal.Decimal(repr(float(value))), "f")
    if "." in text:
        text = text.rstrip("0").rstrip(".")
    return text or "0"


def signed_text(value) -> str:
    text = number_text(value)
    return f"({text})" if text.startswith("-") else text


def raw_expression(bix: int, length: int, blsb: bool) -> str:
    """The unsigned field as an equation, the way OBDb's _extract_bits reads it.
    Bit 0 is the top bit of A; with blsb the bytes holding a value longer than
    8 bits are reversed first."""
    if length < 1 or bix < 0:
        raise Skip("format")
    first, last = bix // 8, (bix + length - 1) // 8
    start, count = bix // 8, (length + 7) // 8

    def source(i: int) -> int:
        if blsb and length > 8 and start <= i < start + count:
            return start + count - 1 - (i - start)
        return i

    indexes = [source(i) for i in range(first, last + 1)]
    if max(indexes) >= MAX_BYTES:
        raise Skip("too long")
    names = [byte_name(i) for i in indexes]
    keep = 8 - bix % 8  # bits of the first byte that belong to the value
    drop = 7 - (bix + length - 1) % 8  # bits of the last byte below it
    head = names[0]
    if len(names) == 1:
        if length == 1:
            return "{%s:%d}" % (head, drop)
        value = head if keep == 8 else f"({head}-(({head}>{keep})<{keep}))"
        return value if drop == 0 else f"({value}>{drop})"
    if len(names) > 5:
        raise Skip("too long")
    if keep == 8 and len(names) <= 4:
        joined = f"INT{8 * len(names)}({':'.join(names)})"
    else:
        if keep != 8:
            head = f"({head}-(({head}>{keep})<{keep}))"
        rest = names[1:]
        tail = rest[0] if len(rest) == 1 else f"INT{8 * len(rest)}({':'.join(rest)})"
        joined = f"(({head}<{8 * len(rest)})+{tail})"
    return joined if drop == 0 else f"({joined}>{drop})"


def scaled(raw: float, mul, div, add) -> float:
    return raw * mul / div + add


def convert_signal(signal: dict) -> dict:
    """One OBDb signal as {name, equation, min, max, unit, metric}, or Skip."""
    fmt = signal.get("fmt", {})
    if "map" in fmt:
        raise Skip("enumeration")
    unit = fmt.get("unit", "")
    if unit == "ascii":
        raise Skip("ascii")
    if unit not in UNITS:
        raise Skip("unit")
    length = int(fmt["len"])
    bix = int(fmt.get("bix", 0))
    blsb = bool(fmt.get("blsb", False))
    sign = bool(fmt.get("sign", False))
    mul, div, add = fmt.get("mul", 1), fmt.get("div", 1), fmt.get("add", 0)
    low, high = fmt.get("min", 0), fmt["max"]
    if div == 0:
        raise Skip("format")
    raw = raw_expression(bix, length, blsb)
    if sign:
        if length in (8, 16, 24, 32):
            value = f"SIGNED{length if length != 8 else ''}({raw})"
        else:
            value = f"({raw}-BIT({raw}:{length - 1})*{2 ** length})"
        raw_low, raw_high = -(1 << (length - 1)), (1 << (length - 1)) - 1
    else:
        value = raw
        raw_low, raw_high = 0, (1 << length) - 1
    if mul != 1:
        value += "*" + signed_text(mul)
    if div != 1:
        value += "/" + signed_text(div)
    if add != 0:
        text = number_text(add)
        value += text if text.startswith("-") else "+" + text
    # OBDb clamps to [min, max] when max > min. Written only where the
    # field can leave that range.
    ends = sorted((scaled(raw_low, mul, div, add), scaled(raw_high, mul, div, add)))
    if high > low:
        if ends[1] > high:
            value = f"MIN({value}:{number_text(high)})"
        if ends[0] < low:
            value = f"MAX({value}:{number_text(low)})"
        ends = [max(low, min(e, high)) for e in ends]
    # A value on a null bound is "no value" in OBDb; there is no way to say
    # that in an equation.
    if "nullmin" in fmt and ends[0] <= fmt["nullmin"] + EPSILON:
        raise Skip("null bound")
    if "nullmax" in fmt and ends[1] >= fmt["nullmax"] - EPSILON:
        raise Skip("null bound")
    metric = ""
    suggested = signal.get("suggestedMetric") or METRIC_BY_ID.get(signal["id"], "")
    wanted = EV_METRICS.get(suggested)
    if wanted and wanted[1] == unit:
        metric = wanted[0]
    name = clean_text(signal.get("name") or signal["id"])
    return {"name": name, "equation": value, "min": number_text(low), "max": number_text(high),
            "unit": UNITS[unit], "metric": metric, "ev": suggested in EV_PACK_METRICS}


def clean_text(text: str) -> str:
    text = text.replace(chr(0x2014), "-").replace(chr(0x2013), "-")
    return " ".join(text.replace("\t", " ").split())


# ---------------------------------------------------------------------------
# Self-check: OBDb's decoder against the equation, on many answers
# ---------------------------------------------------------------------------

def obdb_decode(fmt: dict, data: bytes):
    """OBDb's Scaling.decode_value (.schemas/python/can/signals.py), None for "no value"."""
    start, length = fmt.get("bix", 0), fmt["len"]
    end = start + length
    if end > len(data) * 8:
        raise ValueError("not enough data")
    array = bytearray(data)
    if fmt.get("blsb", False) and length > 8:
        first = start // 8
        last = min(first + (length + 7) // 8, len(array))
        array[first:last] = bytearray(reversed(array[first:last]))
    raw = 0
    for i in range(start, end):
        if array[i // 8] & (1 << (7 - i % 8)):
            raw |= 1 << (end - i - 1)
    if fmt.get("sign", False) and raw & (1 << (length - 1)):
        raw -= 1 << length
    value = raw * fmt.get("mul", 1) / fmt.get("div", 1) + fmt.get("add", 0)
    low, high = fmt.get("min", 0), fmt["max"]
    if high > low:
        value = max(low, min(value, high))
    if fmt.get("nullmin") is not None and value <= fmt["nullmin"] + EPSILON:
        return None
    if fmt.get("nullmax") is not None and value >= fmt["nullmax"] - EPSILON:
        return None
    return value


class Equation:
    """The subset of crates/caruspex-record/src/custom_pids.rs this generator
    writes, evaluated the same way (same operations in the same order)."""

    FUNCTIONS = {"SIGNED": 1, "SIGNED16": 1, "SIGNED24": 1, "SIGNED32": 1, "INT16": 2,
                 "INT24": 3, "INT32": 4, "MIN": 2, "MAX": 2, "BIT": 2}

    def __init__(self, text: str):
        self.s, self.pos = text, 0
        self.tree = self.shift()
        if self.pos != len(self.s):
            raise ValueError(f"trailing text in {text}")

    def peek(self) -> str:
        return self.s[self.pos] if self.pos < len(self.s) else ""

    def shift(self):
        left = self.sum()
        while self.peek() in ("<", ">"):
            op = self.s[self.pos]
            self.pos += 1
            left = (op, left, self.sum())
        return left

    def sum(self):
        left = self.product()
        while self.peek() in ("+", "-"):
            op = self.s[self.pos]
            self.pos += 1
            left = (op, left, self.product())
        return left

    def product(self):
        left = self.unary()
        while self.peek() in ("*", "/"):
            op = self.s[self.pos]
            self.pos += 1
            left = (op, left, self.unary())
        return left

    def unary(self):
        if self.peek() == "-":
            self.pos += 1
            return ("neg", self.unary())
        return self.primary()

    def primary(self):
        m = re.compile(r"[0-9.]+|[A-Z]+[0-9]*|[(){}:]").match(self.s, self.pos)
        if not m:
            raise ValueError(f"bad equation {self.s}")
        token = m.group(0)
        self.pos = m.end()
        if token == "(":
            inner = self.shift()
            self.expect(")")
            return inner
        if token == "{":
            byte = self.primary()
            self.expect(":")
            bit = self.primary()
            self.expect("}")
            return ("BIT", byte, bit)
        if token[0].isdigit() or token[0] == ".":
            return ("num", float(token))
        if token in self.FUNCTIONS:
            self.expect("(")
            args = [self.shift()]
            while self.peek() == ":":
                self.pos += 1
                args.append(self.shift())
            self.expect(")")
            if len(args) != self.FUNCTIONS[token]:
                raise ValueError(f"{token} takes {self.FUNCTIONS[token]} arguments")
            return (token, *args)
        index = ord(token[0]) - 65 if len(token) == 1 else 26 + (ord(token[0]) - 65) * 26 + ord(token[1]) - 65
        return ("byte", index)

    def expect(self, char: str) -> None:
        if self.peek() != char:
            raise ValueError(f"expected {char} in {self.s}")
        self.pos += 1

    def evaluate(self, data: bytes) -> float:
        return self.eval(self.tree, data)

    def eval(self, node, data: bytes) -> float:
        op = node[0]
        if op == "num":
            return node[1]
        if op == "byte":
            return float(data[node[1]])
        if op == "neg":
            return -self.eval(node[1], data)
        if op.startswith("SIGNED"):
            bits = int(op[6:] or 8)
            raw = int(self.eval(node[1], data)) & ((1 << bits) - 1)
            return float(raw) - 2.0 ** bits if raw >= 1 << (bits - 1) else float(raw)
        if op.startswith("INT"):
            total = 0.0
            for arg in node[1:]:
                total = total * 256.0 + float(int(self.eval(arg, data)) & 0xFF)
            return total
        a, b = self.eval(node[1], data), self.eval(node[2], data)
        if op == "+":
            return a + b
        if op == "-":
            return a - b
        if op == "*":
            return a * b
        if op == "/":
            return a / b
        if op == "<":
            return float(int(a)) * 2.0 ** int(b)
        if op == ">":
            return math.floor(float(int(a)) * 2.0 ** -int(b))
        if op == "MIN":
            return min(a, b)
        if op == "MAX":
            return max(a, b)
        if op == "BIT":
            return float((int(a) >> int(b)) & 1)
        raise ValueError(op)


def check_signal(fmt: dict, equation: str, seed: int) -> None:
    """Raises if the equation and OBDb's decoder disagree on some answer."""
    eq = Equation(equation)
    size = (fmt.get("bix", 0) + fmt["len"] + 7) // 8 + 8
    rng = random.Random(seed)
    answers = [bytes(size), bytes([0xFF] * size)]
    answers += [bytes(rng.randrange(256) for _ in range(size)) for _ in range(6)]
    for data in answers:
        expected, actual = obdb_decode(fmt, data), eq.evaluate(data)
        if expected is None or expected != actual:
            raise SystemExit(f"error: {equation} gives {actual}, OBDb {expected} for {data.hex()} ({fmt})")


# ---------------------------------------------------------------------------
# Commands: addresses and requests
# ---------------------------------------------------------------------------

# Makes whose engine values behind the extended diagnostic session are
# taken: the Volkswagen group diesels (soot mass measured, distance and time
# since the last regeneration). The app reads them in that session only from
# the engine control unit and only with the services its allow list knows
# (caruspex_protocols::session_request_allowed).
SESSION_MAKES = ("Audi", "Volkswagen", "VW", "Seat", "SEAT", "Skoda", "Cupra", "Porsche", "Bentley",
                 "Lamborghini")


def command_session(repo: str, cmd: dict) -> str:
    """The diagnostic session a command needs: "" for the default one,
    "03" for the extended session of a Volkswagen group engine, or Skip."""
    if "din" not in cmd and "dout" not in cmd:
        return ""
    make, _ = split_repo(repo)
    service = next(iter(cmd["cmd"]), "")
    if (make in SESSION_MAKES and str(cmd.get("din", "")).upper() == "03" and "dout" not in cmd
            and cmd["hdr"].upper() == "7E0" and cmd.get("rax", "7E8").upper() == "7E8"
            and service == "22" and "eax" not in cmd):
        return "03"
    raise Skip("session")


def command_address(cmd: dict) -> tuple[str, str, str]:
    """(request, header, response) in Caruspex form, or Skip. The session
    (din, dout) is checked by command_session."""
    if "eax" in cmd:
        raise Skip("extended")
    if cmd.get("proto") in ("9141-2", "14230") or "baud" in cmd or "init" in cmd:
        raise Skip("k-line")
    service, = cmd["cmd"].keys()
    pid = cmd["cmd"][service].upper()
    if service not in ("01", "21", "22") or not re.match(r"^[0-9A-F]+$", pid):
        raise Skip("format")
    request = service + pid
    hdr = cmd["hdr"].upper()
    rax = cmd.get("rax", "").upper()
    if not re.match(r"^[0-9A-F]+$", hdr + rax):
        raise Skip("format")
    if len(hdr) == 3:
        if hdr == "7DF":
            if rax:
                raise Skip("functional")
            return request, "", ""
        if rax and len(rax) != 3:
            raise Skip("format")
        return request, hdr, rax
    if len(hdr) != 4:
        raise Skip("format")
    priority = int(cmd.get("pri", "18"), 16)
    header = f"{priority:02X}{hdr}{cmd.get('tst', 'F1').upper()}"
    if hdr == "DB33":
        if rax:
            raise Skip("functional")
        return request, "", ""
    if not rax:
        return request, header, ""
    value = int(rax, 16)
    if len(rax) > 6:
        response = value & 0x1FFFFFFF
    elif value <= 0xFF:
        response = ((priority & 0x1F) << 24) | 0x00DAF100 | value
    else:
        response = (priority << 24) | (value & 0xFFFFFF)
    return request, header, f"{response:08X}"


# ---------------------------------------------------------------------------
# Model years
# ---------------------------------------------------------------------------

def filter_matches(f: dict | None, year: int) -> bool:
    """OBDb's Filter.matches, including its quirks (from >= to wraps)."""
    if f is None:
        return True
    lo, hi, years = f.get("from"), f.get("to"), f.get("years") or None
    if lo is not None and hi is not None:
        if lo < hi:
            if lo <= year <= hi:
                return True
        elif year >= lo or year <= hi:
            return True
    elif hi is not None:
        if year <= hi:
            return True
    elif lo is not None:
        if year >= lo:
            return True
    return bool(years) and year in years


def filter_years(f: dict | None) -> list[int]:
    if not f:
        return []
    out = [y for y in (f.get("from"), f.get("to")) if y is not None]
    return out + list(f.get("years") or [])


def generations(text: str) -> list[tuple[int | None, int | None]]:
    """(start, end) of each entry under "generations:", end None while built."""
    out: list[list] = []
    inside = False
    for line in text.splitlines():
        if line and not line[0].isspace():
            inside = line.startswith("generations:")
            continue
        stripped = line.strip()
        if not inside:
            continue
        if stripped.startswith("- "):
            out.append([None, None])
            stripped = stripped[2:].strip()
        m = re.match(r"^(start_year|end_year):\s*(\d{4})\s*$", stripped)
        if m and out:
            out[-1][0 if m.group(1) == "start_year" else 1] = int(m.group(2))
    return [(a, b) for a, b in out]


def signalset_years(path: str) -> tuple[int, int] | None:
    m = re.match(r"^signalsets/v3/(\d{4})-(\d{4})\.json$", path)
    return (int(m.group(1)), int(m.group(2))) if m else None


# ---------------------------------------------------------------------------
# Makes, models and the vehicle catalog
# ---------------------------------------------------------------------------

# Makes written with a hyphen in OBDb repository names.
MULTI_WORD_MAKES = ("Alfa-Romeo", "Aston-Martin", "Land-Rover", "Mercedes-Benz", "Rolls-Royce")
# OBDb make -> brands of the catalog (first one for display).
MAKE_BRANDS = {
    "VW": ["Volkswagen"], "FIAT": ["Fiat"], "INFINITI": ["Infiniti"], "MINI": ["Mini"],
    "smart": ["Smart"], "Citroen": ["Citroën"], "Seat": ["SEAT"], "Skoda": ["Škoda"],
    "VauxhallOpel": ["Opel", "Vauxhall"], "Vauxhall": ["Vauxhall", "Opel"],
    "Alfa-Romeo": ["Alfa Romeo"], "Aston-Martin": ["Aston Martin"], "Land-Rover": ["Land Rover"],
    "DongFeng": ["Dongfeng"],
}
# Words dropped from a model name when nothing else matched.
MODEL_SUFFIXES = ("plug in hybrid", "plugin hybrid", "hybrid", "phev", "hev", "electric", "ev",
                  "e", "class", "prime", "recharge", "energi", "limited", "cargo", "wagon",
                  "convertible", "tourer", "gli")


def fold(text: str) -> str:
    text = unicodedata.normalize("NFKD", text)
    return "".join(c for c in text.lower() if c.isalnum() and not unicodedata.combining(c))


def split_repo(name: str) -> tuple[str, str]:
    for make in MULTI_WORD_MAKES:
        if name.startswith(make + "-"):
            return make, name[len(make) + 1:]
    make, _, model = name.partition("-")
    return make, model


def display_model(model: str) -> str:
    parts = model.split("-")
    out = parts[0]
    for prev, part in zip(parts, parts[1:]):
        keep = (len(prev) <= 2 and prev.isalpha()) or (len(part) == 1 and part.isalpha())
        out += ("-" if keep else " ") + part
    return out


def load_catalog() -> dict[str, list[tuple[str, list[str]]]]:
    """Brand -> [(model, [aliases of its generations])]."""
    catalog: dict[str, dict[str, list[str]]] = collections.defaultdict(dict)
    for path in sorted(os.listdir(VEHICLES)):
        if not path.endswith(".csv") or path == "brands.csv":
            continue
        text = io.open(os.path.join(VEHICLES, path), encoding="utf-8-sig").read()
        lines = [line for line in text.splitlines() if line.strip() and not line.startswith("#")]
        for row in csv.DictReader(lines):
            aliases = catalog[row["brand"]].setdefault(row["model"], [])
            aliases += [a for a in row.get("aliases", "").split("|") if a.strip()]
    return {b: sorted(m.items()) for b, m in catalog.items()}


def load_aliases() -> dict[str, list[tuple[str, str]]]:
    out: dict[str, list[tuple[str, str]]] = collections.defaultdict(list)
    if not os.path.exists(ALIASES):
        return out
    text = io.open(ALIASES, encoding="utf-8-sig").read()
    lines = [line for line in text.splitlines() if line.strip() and not line.startswith("#")]
    for row in csv.DictReader(lines):
        out[row["repo"]].append((row["brand"], row["model"]))
    return out


def match_catalog(repo: str, make: str, model: str, catalog: dict,
                  aliases: dict) -> list[tuple[str, str]]:
    """(catalog brand, catalog model) pairs the repository stands for."""
    if repo in aliases:
        pairs = aliases[repo]
        for brand, cat_model in pairs:
            if cat_model != "-" and cat_model not in dict(catalog.get(brand, [])):
                raise SystemExit(f"error: {ALIASES}: {brand} {cat_model} is not in the catalog")
        return [(b, m) for b, m in pairs if m != "-"]
    brands = MAKE_BRANDS.get(make) or [b for b in catalog if fold(b) == fold(make)]
    # The name as it is, then without trim and drive words at the end
    # ("Camry Hybrid", "Sierra 2500HD" -> "Camry", "Sierra").
    words = model.lower().replace("-", " ").split()
    candidates = [" ".join(words)]
    while len(words) > 1:
        if words[-1] in MODEL_SUFFIXES or " ".join(words[-2:]) in MODEL_SUFFIXES:
            words = words[:-1]
        elif any(ch.isdigit() for ch in words[-1]) and len("".join(words[:-1])) >= 2:
            words = words[:-1]
        else:
            break
        candidates.append(" ".join(words))
    out = []
    for brand in brands:
        models = catalog.get(brand, [])
        for want in candidates:
            hits = [m for m, _ in models if m.lower().replace("-", " ") == want]
            if not hits:
                hits = [m for m, _ in models if fold(m) == fold(want)]
            if not hits:
                hits = sorted({m for m, al in models if any(fold(a) == fold(want) for a in al)})
            if len(hits) == 1:
                out.append((brand, hits[0]))
                break
    return out


# ---------------------------------------------------------------------------
# Conversion
# ---------------------------------------------------------------------------

def load_repo(name: str) -> dict | None:
    folder = os.path.join(CACHE, name)
    meta_path = os.path.join(folder, "meta.json")
    if not os.path.exists(meta_path):
        return None
    meta = json.load(open(meta_path, encoding="utf-8"))
    sets = {}
    for path in meta["signalsets"]:
        sets[path] = json.load(io.open(os.path.join(folder, safe_name(path)), encoding="utf-8"))
    gens_path = os.path.join(folder, "generations.yaml")
    gens = generations(io.open(gens_path, encoding="utf-8").read()) if os.path.exists(gens_path) else []
    return {"name": name, "sets": sets, "generations": gens, "test_years": meta["test_years"],
            "commit": meta.get("commit", ""),
            # Read only where they are used, so an odd file elsewhere cannot stop the run.
            "tests": load_tests(folder, meta.get("tests", [])) if name in FALLBACK else {}}


# ---------------------------------------------------------------------------
# Make fallback: recorded answers of the model confirm the make set
# ---------------------------------------------------------------------------

def yaml_scalar(text: str):
    text = text.strip()
    if text in ("[]", "{}"):
        return [] if text == "[]" else {}
    if len(text) >= 2 and text[0] == text[-1] and text[0] in "'\"":
        return text[1:-1]
    return text


def yaml_block(lines: list[tuple[int, str]], i: int, indent: int):
    """The block style subset OBDb writes its test cases in: mappings,
    sequences and plain or quoted scalars. Returns (value, next line)."""
    if i < len(lines) and lines[i][0] == indent and (lines[i][1] == "-" or lines[i][1].startswith("- ")):
        items = []
        while i < len(lines) and lines[i][0] == indent and (lines[i][1] == "-" or lines[i][1].startswith("- ")):
            rest = lines[i][1][1:].strip()
            if not rest:
                value, i = yaml_block(lines, i + 1, lines[i + 1][0]) if i + 1 < len(lines) else (None, i + 1)
            elif re.match(r"^[^'\"\s][^:]*:(\s|$)", rest):
                # "- key: value" opens a mapping two columns further in.
                lines[i] = (indent + 2, rest)
                value, i = yaml_block(lines, i, indent + 2)
            else:
                value, i = yaml_scalar(rest), i + 1
            items.append(value)
        return items, i
    out: dict = {}
    while i < len(lines) and lines[i][0] == indent:
        m = re.match(r"^(\"[^\"]*\"|'[^']*'|[^:]+):(?:\s+(.*))?$", lines[i][1])
        if not m:
            raise ValueError(f"unexpected line {lines[i][1]!r}")
        key, rest = yaml_scalar(m.group(1)), (m.group(2) or "").strip()
        i += 1
        if re.match(r"^[|>][+-]?$", rest):
            # A block scalar: the CAN frames of an answer, one per line.
            block = []
            while i < len(lines) and lines[i][0] > indent:
                block.append(lines[i][1])
                i += 1
            out[key] = ("\n" if rest[0] == "|" else " ").join(block)
        elif rest:
            out[key] = yaml_scalar(rest)
        elif i < len(lines) and (lines[i][0] > indent or (lines[i][0] == indent and lines[i][1].startswith("- "))):
            out[key], i = yaml_block(lines, i, lines[i][0])
        else:
            out[key] = None
    return out, i


def parse_yaml(text: str):
    lines = []
    for raw in text.splitlines():
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        lines.append((len(raw) - len(raw.lstrip(" ")), raw.strip()))
    if not lines:
        return {}
    value, end = yaml_block(lines, 0, lines[0][0])
    if end != len(lines):
        raise ValueError(f"unexpected line {lines[end][1]!r}")
    return value


def load_tests(folder: str, paths: list[str]) -> dict[str, list[tuple[int, list[str], dict]]]:
    """Command id -> [(model year, CAN frames, expected values)] of the
    recorded answers in tests/test_cases/<year>/commands/."""
    out: dict[str, list] = collections.defaultdict(list)
    for path in paths:
        year = int(path.split("/")[2])
        try:
            data = parse_yaml(io.open(os.path.join(folder, safe_name(path)), encoding="utf-8").read())
        except (ValueError, UnicodeDecodeError) as e:
            raise SystemExit(f"error: {folder} {path}: {e}")
        if not isinstance(data, dict):
            continue
        ident = str(data.get("command_id") or os.path.basename(path)[:-5])
        for case in data.get("test_cases") or []:
            if not isinstance(case, dict) or not isinstance(case.get("expected_values"), dict):
                continue
            response = case.get("response")
            frames = response if isinstance(response, list) else str(response or "").split()
            out[ident].append((year, [str(f) for f in frames], case["expected_values"]))
    return out


def obdb_command_id(cmd: dict) -> str:
    """The id OBDb's Command.from_json gives a command (.schemas/python/can/signals.py)."""
    service, = cmd["cmd"].keys()
    value = int(cmd["cmd"][service], 16)
    ident = cmd["hdr"] + ("." + cmd["rax"] if cmd.get("rax") else "")
    ident += "." + service + (f"{value:04X}" if service == "22" else f"{value:02X}")
    parts = []
    for key, label in (("tmo", "t"), ("eax", "e"), ("tst", "ta")):
        if cmd.get(key) and int(cmd[key], 16):
            parts.append(f"{label}={int(cmd[key], 16):02X}")
    if cmd.get("fcm1"):
        parts.append("fc=1")
    if cmd.get("proto") == "iso9141_2":
        parts.append("p=9141-2")
    if cmd.get("pri") and int(cmd["pri"], 16):
        parts.append(f"c={int(cmd['pri'], 16):02X}")
    f = cmd.get("filter")
    if f:
        text = []
        lo, hi = f.get("from"), f.get("to")
        if lo is not None and hi is not None and lo < hi:
            text.append(f"{lo}-{hi}")
        else:
            if lo is not None:
                text.append(f"{lo}-")
            if hi is not None:
                text.append(f"-{hi}")
        text += [str(y) for y in sorted(f.get("years") or [])]
        parts.append("f=" + ";".join(text))
    for key, label in (("din", "din"), ("dout", "dout")):
        if key in cmd:
            parts.append(f"{label}={int(cmd[key], 16):02X}")
    if "baud" in cmd:
        parts.append(f"b={cmd['baud']}")
    return ident + ("|" + ",".join(parts) if parts else "")


def field_key(signal: dict) -> tuple:
    fmt = signal.get("fmt", {})
    return (fmt.get("bix", 0), fmt.get("len"), bool(fmt.get("blsb")), bool(fmt.get("sign")),
            fmt.get("mul", 1), fmt.get("div", 1), fmt.get("add", 0), fmt.get("unit", ""),
            json.dumps(fmt.get("map"), sort_keys=True))


def fallback_ambiguous(signals: list[dict]) -> bool:
    """True when the merged make set reads the same bits of an answer in two
    ways (another scaling, unit or position), because the models it was
    merged from disagree about this command. Recorded answers cannot say
    which reading is right for the model, so the command stays out."""
    for i, a in enumerate(signals):
        for b in signals[i + 1:]:
            fa, fb = a.get("fmt", {}), b.get("fmt", {})
            a0, b0 = fa.get("bix", 0), fb.get("bix", 0)
            if a0 < b0 + fb.get("len", 0) and b0 < a0 + fa.get("len", 0) and field_key(a) != field_key(b):
                return True
    return False


def confirmed_by_tests(signal: dict, request: str, cases: list) -> bool:
    """True when the recorded answers give this signal's value and OBDb's
    decoder gives that value on every one of them."""
    seen = False
    for _, frames, expected in cases:
        if signal["id"] not in expected:
            continue
        try:
            want = float(expected[signal["id"]])
            data = bytes.fromhex(reassemble(frames, request))
            got = obdb_decode(signal["fmt"], data)
        except (ValueError, TypeError, IndexError):
            return False
        if got is None or abs(got - want) >= 1e-5:
            return False
        seen = True
    return seen


def command_key(cmd: dict) -> tuple:
    return (cmd["hdr"].upper(), cmd.get("rax", "").upper(), cmd.get("eax", ""), cmd.get("tst", ""),
            cmd.get("pri", ""), json.dumps(cmd["cmd"], sort_keys=True).upper())


def year_segments(repo: dict) -> tuple[list[tuple[int, int, tuple]], set]:
    """[(from, to, commands)], 0 = open end; commands are (path, index) in file order."""
    sets = repo["sets"]
    default = "signalsets/v3/default.json"
    special = {p: signalset_years(p) for p in sets if p != default}
    starts = [a for a, _ in repo["generations"] if a]
    ends = [b for _, b in repo["generations"] if b]
    open_high = not repo["generations"] or any(b is None for _, b in repo["generations"])
    known = starts + ends + list(repo["test_years"])
    bounds = []
    for path, data in sets.items():
        for cmd in data.get("commands", []):
            bounds += filter_years(cmd.get("filter"))
        if special.get(path):
            bounds += [y for y in special[path] if y]
    if known:
        lo, hi = min(known), max(known)
        open_low = False
    elif bounds:
        lo, hi = min(bounds) - 1, max(bounds)
        open_low = True
    else:
        lo = hi = 0
        open_low = True
    top = max([hi] + [b + 1 for b in bounds]) if open_high else hi

    def commands_for(year: int) -> tuple:
        path = default if default in sets else sorted(sets)[0]
        for p, span in special.items():
            if span and span[0] <= year <= span[1]:
                path = p
                break
        chosen: dict[tuple, tuple] = {}
        for index, cmd in enumerate(sets[path].get("commands", [])):
            if filter_matches(cmd.get("filter"), year):
                chosen.pop(command_key(cmd), None)
                chosen[command_key(cmd)] = (path, index)
        return tuple(sorted(chosen.values(), key=lambda pi: pi[1]))

    segments: list[list] = []
    for year in range(lo, top + 1):
        commands = commands_for(year)
        if segments and segments[-1][2] == commands:
            segments[-1][1] = year
        else:
            segments.append([year, year, commands])
    if segments:
        if open_low:
            segments[0][0] = 0
        if open_high:
            segments[-1][1] = 0
    if lo == 0 and hi == 0:
        segments = [[0, 0, commands_for(0)]]
    used = {c for s in segments for c in s[2]}
    return [(a, b, c) for a, b, c in segments if c], used


def slug(text: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", fold_keep(text)).strip("-")


def fold_keep(text: str) -> str:
    text = unicodedata.normalize("NFKD", text)
    return "".join(c for c in text.lower() if not unicodedata.combining(c))


def pack_id(repo: str, start: int, end: int, single: bool) -> str:
    base = "obdb-" + slug(repo)
    if single and not start and not end:
        return base
    return f"{base}-{start or 'x'}-{end or 'x'}"


def tsv(*cells) -> str:
    for c in cells:
        if "\t" in str(c) or "\n" in str(c):
            raise SystemExit(f"error: tab or line break in {c!r}")
    return "\t".join(str(c) for c in cells)


def convert(stats: collections.Counter, unmatched: list, check: bool = True,
            sources: dict | None = None) -> list[str]:
    repos = json.load(open(os.path.join(CACHE, "repos.json"), encoding="utf-8"))
    catalog = load_catalog()
    aliases = load_aliases()
    makes = {entry["name"].lower(): entry["name"] for entry in repos if is_make_repo(entry["name"])}
    lines: list[str] = []
    for entry in repos:
        name = entry["name"]
        if name in MULTI_WORD_MAKES or not is_vehicle_repo(name):
            continue
        repo = load_repo(name)
        if repo is None or not repo["sets"]:
            continue
        make, model = split_repo(name)
        if not model:
            continue
        stats["repos"] += 1
        fallback = None
        if not any(d.get("commands") for d in repo["sets"].values()) and repo["tests"] and name in FALLBACK:
            make_repo = load_repo(makes[FALLBACK[name].lower()]) if FALLBACK[name].lower() in makes else None
            if make_repo and any(d.get("commands") for d in make_repo["sets"].values()):
                repo["sets"] = make_repo["sets"]
                fallback = (make_repo["name"], make_repo["commit"], repo["tests"])
        segments, used = year_segments(repo)
        # Every signal once: converted, or the reason it is not.
        converted: dict[tuple, tuple] = {}
        for path, data in repo["sets"].items():
            for index, cmd in enumerate(data.get("commands", [])):
                signals = cmd.get("signals", [])
                cases = None
                if fallback:
                    # Of the make set only what this model answered counts
                    # as its own signals.
                    cases = fallback[2].get(obdb_command_id(cmd))
                    if not cases:
                        continue
                stats["signals"] += len(signals)
                if (path, index) not in used:
                    stats["skip: no model year or replaced"] += len(signals)
                    continue
                try:
                    session = command_session(name, cmd)
                    address = command_address(cmd)
                except Skip as s:
                    stats["skip: " + s.reason] += len(signals)
                    continue
                if cases is not None and fallback_ambiguous(signals):
                    stats["skip: fallback ambiguous"] += len(signals)
                    continue
                values = []
                fields = set()
                for signal in signals:
                    if cases is not None:
                        # The merged set repeats one field under the names
                        # of several models; it is shown once.
                        if field_key(signal) in fields:
                            stats["skip: fallback duplicate"] += 1
                            continue
                        fields.add(field_key(signal))
                    try:
                        value = convert_signal(signal)
                    except Skip as s:
                        stats["skip: " + s.reason] += 1
                        continue
                    if cases is not None and not confirmed_by_tests(signal, address[0], cases):
                        stats["skip: fallback unconfirmed"] += 1
                        continue
                    if check:
                        check_signal(signal["fmt"], value["equation"],
                                     zlib.crc32(f"{name} {signal['id']}".encode()))
                    values.append(value)
                if values:
                    converted[(path, index)] = (address, values, session)
                    if session:
                        stats["in the extended session"] += len(values)
                    stats["imported"] += len(values)
        packs = []
        for start, end, commands in segments:
            present = [c for c in commands if c in converted]
            if present:
                packs.append((start, end, present))
        if not packs:
            stats["repos without values"] += 1
            continue
        matches = match_catalog(name, make, model, catalog, aliases)
        if not matches:
            unmatched.append(name)
        brands = MAKE_BRANDS.get(make)
        shown_make = matches[0][0] if matches else (brands[0] if brands else make.replace("-", " "))
        ev = any(v["ev"] for c in converted.values() for v in c[1])
        segment_text = ",".join(f"{a}-{b}" for a, b, _ in packs)
        match_text = ";".join(f"{b}|{m}" for b, m in matches)
        lines.append(tsv("R", name, shown_make, display_model(model), segment_text, match_text,
                         1 if ev else 0))
        stats["packs"] += len(packs)
        if fallback:
            stats["fallback repos"] += 1
        if sources is not None:
            sources[name] = (repo["commit"], fallback[0] if fallback else "", fallback[1] if fallback else "")
        # Commands in file order, each with the packs it belongs to.
        order = sorted({c for _, _, cs in packs for c in cs},
                       key=lambda pi: (list(repo["sets"]).index(pi[0]), pi[1]))
        for c in order:
            mask = sum(1 << i for i, (_, _, cs) in enumerate(packs) if c in cs)
            (request, header, response), values, session = converted[c]
            cells = ["C", f"{mask:x}", request, header, response] + ([session] if session else [])
            lines.append(tsv(*cells))
            for v in values:
                lines.append(tsv("V", v["name"], v["metric"], v["equation"], v["min"], v["max"],
                                 v["unit"]))
    return lines


def do_convert() -> int:
    if not os.path.exists(os.path.join(CACHE, "repos.json")):
        print(f"error: no cache in {CACHE}; run with --fetch first", file=sys.stderr)
        return 1
    stats: collections.Counter = collections.Counter()
    unmatched: list[str] = []
    sources: dict = {}
    lines = convert(stats, unmatched, sources=sources)
    skipped = sum(n for k, n in stats.items() if k.startswith("skip: "))
    vehicles = stats["repos"] - stats["repos without values"]
    head = tsv("O", vehicles, stats["repos"], stats["imported"], skipped, stats["packs"])
    text = "\n".join([head] + lines) + "\n"
    data = zlib.compress(text.encode("utf-8"), 9)
    os.makedirs(DATA, exist_ok=True)
    with open(TARGET, "wb") as f:
        f.write(data)
    print(f"{TARGET}: {len(data)} bytes ({len(text.encode('utf-8'))} unpacked)")
    print(f"{stats['repos']} repositories ({vehicles} with values), {stats['packs']} packs, "
          f"{stats['signals']} signals: {stats['imported']} imported, {skipped} skipped")
    for key in sorted(k for k in stats if k.startswith("skip: ")):
        print(f"  {key[6:]}: {stats[key]}")
    print(f"{stats['in the extended session']} imported signals are read in the extended session")
    print(f"{len(unmatched)} vehicles without a catalog model"
          + (": " + ", ".join(unmatched) if "--verbose" in sys.argv else " (--verbose lists them)"))
    repos = [line.split("\t")[1] for line in lines if line.startswith("R\t")]
    write_attribution(repos, sources, stats, vehicles, skipped)
    return 0


def write_attribution(repos: list[str], sources: dict, stats: collections.Counter, vehicles: int,
                      skipped: int) -> None:
    pushed = json.load(open(os.path.join(CACHE, "repos.json"), encoding="utf-8"))
    state = max(r["pushed_at"] for r in pushed)[:10]
    reasons = ", ".join(f"{key[6:]} {stats[key]}" for key in sorted(stats) if key.startswith("skip: "))
    text = f"""# OBDb data in Caruspex

The file `obdb-packs.z` in this folder is an adaptation of the signal sets of
[OBDb](https://github.com/OBDb), the open vehicle signal database, by the OBDb
contributors. OBDb and this adaptation are licensed under the
[Creative Commons Attribution-ShareAlike 4.0 International](https://creativecommons.org/licenses/by-sa/4.0/)
licence, see `LICENSE` next to this file. This folder is not covered by the
MIT licence of the Caruspex code.

Changes made: `tools/gen_obdb.py` converted the signal sets as of {state}
into packs of custom values for Caruspex. Every signal became a Torque
equation that gives the same value as OBDb's own decoder; commands were split
into packs by model year. Signals that cannot be expressed that way were left
out ({reasons}).

Models whose own signal sets are empty ({stats['fallback repos']} here) take OBDb's merged
set of the make, as OBDb's tools do, but only where that was checked (FALLBACK
in tools/gen_obdb.py) and only the commands and signals that the recorded
answers of that model in its repository (tests/test_cases) confirm; such a
line names the make repository and its commit after the model's own.

Totals: {stats['repos']} repositories read, {vehicles} with values, {stats['packs']} packs,
{stats['imported']} signals imported, {skipped} skipped. Of the imported signals
{stats['in the extended session']} are engine values of Volkswagen group cars that OBDb reads in
the extended diagnostic session (din 03); their commands carry that session.

Repositories used, each at github.com/OBDb/<name>, with the commit read:

"""

    def line(name: str) -> str:
        commit, make, make_commit = sources.get(name, ("", "", ""))
        text = f"- {name}" + (f" {commit}" if commit else "")
        return text + (f" (signals from {make} {make_commit})" if make else "")

    text += "\n".join(line(name) for name in repos) + "\n"
    with open(ATTRIBUTION, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


# ---------------------------------------------------------------------------
# Test fixtures: real OBDb commands with recorded answers
# ---------------------------------------------------------------------------

FIXTURES = os.path.join(ROOT, "crates", "caruspex-record", "tests", "fixtures")
FIXTURE_IN = os.path.join(FIXTURES, "obdb_signals.json")
FIXTURE_OUT = os.path.join(FIXTURES, "obdb_cases.json")


def is_echo(line: str, request: str) -> bool:
    """The request the adapter echoes before the answer ("2121" or "21211")."""
    line = line.strip().upper()
    return line.startswith(request) and len(line) <= len(request) + 2


def reassemble(lines: list[str], request: str) -> str:
    """The data bytes after the echo, from the CAN frames of an OBDb test case."""
    frames = [line.strip().upper() for line in lines if line.strip()]
    frames = [f for f in frames if not is_echo(f, request)]
    if not frames:
        return ""
    width = 8 if len(frames[0]) % 2 == 0 else 3
    payload = b""
    length = 0
    for frame in frames:
        data = bytes.fromhex(frame[width:])
        kind = data[0] >> 4
        if kind == 0:
            payload, length = data[1:1 + (data[0] & 0x0F)], data[0] & 0x0F
        elif kind == 1:
            length = ((data[0] & 0x0F) << 8) | data[1]
            payload = data[2:]
        elif kind == 2:
            payload += data[1:]
    payload = payload[:length]
    echo = 1 + (len(request) - 2) // 2
    return payload[echo:].hex().upper()


def do_fixtures() -> int:
    """Converts the fixture commands and checks them against the values OBDb
    recorded for the same answers. Runs offline; the CI compares the result."""
    source = json.load(io.open(FIXTURE_IN, encoding="utf-8"))
    out = []
    for entry in source["entries"]:
        cmd = entry["command"]
        item = {"repo": entry["repo"], "year": entry["year"]}
        try:
            session = command_session(entry["repo"], cmd)
            request, header, response = command_address(cmd)
            item.update({"request": request, "header": header, "response": response})
            if session:
                item["session"] = session
        except Skip as s:
            item["skip"] = s.reason
            out.append(item)
            continue
        item["frames"] = [f for f in entry["response"] if not is_echo(f, request)]
        data = bytes.fromhex(reassemble(entry["response"], request))
        item["data"] = data.hex().upper()
        signals = []
        for signal in cmd["signals"]:
            expected = entry["expected"].get(signal["id"])
            row = {"id": signal["id"]}
            try:
                value = convert_signal(signal)
                reference = obdb_decode(signal["fmt"], data)
                if not isinstance(expected, float) or reference is None or \
                        abs(reference - expected) > 1e-6 * max(1.0, abs(expected)):
                    raise SystemExit(f"error: {entry['repo']} {signal['id']}: OBDb decodes "
                                     f"{reference}, its test case says {expected}")
                check_signal(signal["fmt"], value["equation"], zlib.crc32(signal["id"].encode()))
                row.update({"name": value["name"], "equation": value["equation"],
                            "unit": value["unit"], "expected": expected})
            except Skip as s:
                row["skip"] = s.reason
            signals.append(row)
        item["signals"] = signals
        out.append(item)
    with open(FIXTURE_OUT, "w", encoding="utf-8", newline="\n") as f:
        json.dump({"_note": source["_note"] + " Generated by tools/gen_obdb.py --fixtures.",
                   "entries": out}, f, ensure_ascii=False, indent=1)
        f.write("\n")
    converted = sum(1 for e in out for s in e.get("signals", []) if "equation" in s)
    print(f"{FIXTURE_OUT}: {len(out)} commands, {converted} signals converted")
    return 0


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    if "--fetch" in sys.argv:
        return do_fetch()
    if "--fixtures" in sys.argv:
        return do_fixtures()
    return do_convert()


if __name__ == "__main__":
    sys.exit(main())
