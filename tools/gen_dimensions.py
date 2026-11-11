"""Real outer dimensions per model generation from Wikidata (CC0).

Wikidata keeps many car models and their generations as items of the class
"car model" (Q3231690) or "car model series" (Q59773381), most of them with
length (P2043), width (P2049), height (P2048), wheelbase (P3039) and mass
(P2067). This tool brings those values to the generations of the vehicle
catalog (data/vehicles/), for the 3D X-ray view.

    py -I tools/gen_dimensions.py --fetch      query Wikidata, keep the answers in .cache/wikidata/
    py -I tools/gen_dimensions.py --match      match the cache to the catalog, write data/wikidata/dimensions.csv
    py -I tools/gen_dimensions.py              write the Rust table from data/wikidata/dimensions.csv
    py -I tools/gen_dimensions.py --fixtures   check the conversion and matching on the test fixtures

--fetch needs the network and asks the SPARQL endpoint
https://query.wikidata.org/sparql with a User-Agent that names the project, a
few queries in all. The cache is not committed. --match and the default run
work offline; the CI runs the default (the Rust table follows the committed
CSV) and --fixtures. To refresh the values: --fetch, --match, look at the
report and the diff of data/wikidata/dimensions.csv, run the tests, commit.

Conversion. Quantities come with their unit; millimetre, centimetre, metre,
inch and foot become millimetres, kilogram, tonne and pound become
kilograms, rounded to whole numbers. Other units are skipped and counted.
Values outside a sane range for a road car (a length of 40 m, a width of
12 mm: unit mistakes in Wikidata) are skipped and counted, never repaired.
Deprecated statements are left out; of several values the one whose "applies
to part" (P518) names the body of the catalog row wins, otherwise the lower
middle value of those without a qualifier (a real value, never an average).

Matching. The name of an item ("BMW 3 Series (F30)", "Volkswagen Golf Mk7",
"Opel Corsa D", "Audi A2") is split into brand, model and generation: the
brand is the catalog brand the name starts with (or the item's brand P1716),
the model one of the names of the catalog model (its German name or a search
alias such as "3 Series"), and what is left must name the generation: its
code ("F30", "8V", "W213"), its number ("Mk7", "VII", "7", "seventh
generation") or its first year ("(2019)", checked against the catalog years
and the item's start time P580). A name without generation part fits only a
model with a single generation in the catalog whose years overlap the item's.
Of several fitting items the one named by its label wins over one that only
lists the name as an alias, then the one with more values.
data/wikidata/aliases.csv sets the item by hand where names differ (several
items joined by "|": each value from the first that has it), blocks a wrong
match ("-"), or leaves out a value that is not the body's (a width measured
over the mirrors). Every line there says why.

Wikidata has no property for the track of a car, so that column stays empty;
the X-ray view takes the typical track for the width then.

Values from the makers. Wikidata lacks the dimensions of some popular
generations (F30, F36, Focus, Octavia) or has only the values of the whole
series. data/dimensions_manual.csv lists them by hand from the technical data
the maker published (press kit or data sheet, named with its year in every
row). Those values come first; Wikidata fills the fields the row leaves
empty. The Rust table keeps the document, so the app can name it.

Output: data/wikidata/dimensions.csv (brand, model, generation, the values in
mm and kg, the item ids and the English name), committed, and from it and
data/dimensions_manual.csv crates/caruspex-vehicles/src/generated/dimensions.rs.
"""

from __future__ import annotations

import collections
import csv
import functools
import io
import json
import os
import re
import sys
import time
import unicodedata
import urllib.error
import urllib.parse
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CACHE = os.path.join(ROOT, ".cache", "wikidata")
VEHICLES = os.path.join(ROOT, "data", "vehicles")
DATA = os.path.join(ROOT, "data", "wikidata")
TABLE = os.path.join(DATA, "dimensions.csv")
ALIASES = os.path.join(DATA, "aliases.csv")
MANUAL = os.path.join(ROOT, "data", "dimensions_manual.csv")
TARGET_RS = os.path.join(ROOT, "crates", "caruspex-vehicles", "src", "generated", "dimensions.rs")
FIXTURES = os.path.join(ROOT, "crates", "caruspex-vehicles", "tests", "fixtures")

ENDPOINT = "https://query.wikidata.org/sparql"
USER_AGENT = "Caruspex data generator (github.com/SenseiIssei/Caruspex)"
ENTITY = "http://www.wikidata.org/entity/"

CLASSES = "wd:Q3231690 wd:Q59773381"
PROPERTIES = {
    "P2043": "length",
    "P2049": "width",
    "P2048": "height",
    "P3039": "wheelbase",
    "P2067": "mass",
}
FIELDS = ["length", "width", "height", "wheelbase", "track", "mass"]
MAIN = ["length", "width", "height", "wheelbase"]

# Unit item -> factor to millimetres or kilograms.
LENGTH_UNITS = {
    "Q174789": 1.0,  # millimetre
    "Q174728": 10.0,  # centimetre
    "Q11573": 1000.0,  # metre
    "Q218593": 25.4,  # inch
    "Q3710": 304.8,  # foot
}
MASS_UNITS = {
    "Q11570": 1.0,  # kilogram
    "Q191118": 1000.0,  # tonne
    "Q100995": 0.45359237,  # pound
}
# Sane range per field for a road car, in mm or kg.
RANGES = {
    "length": (2000, 7000),
    "width": (1200, 2700),
    "height": (1000, 2800),
    "wheelbase": (1500, 4600),
    "mass": (400, 4500),
}

# Body of a catalog row -> Wikidata items its "applies to part" may name.
BODY_PARTS = {
    "hatch": {"Q188317", "Q5276285"},  # hatchback, liftback
    "sedan": {"Q190578", "Q28872924"},  # sedan, fastback sedan
    "estate": {"Q193800", "Q1501101"},  # station wagon, shooting brake
    "suv": {"Q192152"},
    "van": {"Q193468", "Q1186137"},
    "coupe": {"Q190591"},
    "cabrio": {"Q189488"},
    "pickup": {"Q215601"},
    "mpv": {"Q1186137"},
    "micro": set(),
    "roadster": {"Q1130543"},
}

YEAR_PROPS = ["P580", "P582", "P571", "P2669", "P729", "P730"]
LINK_PROPS = ["P1716", "P176", "P179"]


# ---------------------------------------------------------------------------
# Download
# ---------------------------------------------------------------------------

def sparql(query: str) -> dict:
    data = urllib.parse.urlencode({"query": query, "format": "json"}).encode()
    request = urllib.request.Request(ENDPOINT, data=data, headers={
        "User-Agent": USER_AGENT,
        "Accept": "application/sparql-results+json",
        "Content-Type": "application/x-www-form-urlencoded",
    })
    for attempt in range(6):
        try:
            with urllib.request.urlopen(request, timeout=300) as response:
                return json.load(response)
        except urllib.error.HTTPError as e:
            # 429: too many requests, 5xx: the query service is busy.
            if attempt == 5 or (e.code != 429 and e.code < 500):
                raise
            wait = int(e.headers.get("Retry-After") or 0) or 20 * (attempt + 1)
            print(f"  HTTP {e.code}, waiting {wait} s", file=sys.stderr)
            time.sleep(min(wait, 300))
        except (urllib.error.URLError, TimeoutError):
            if attempt == 5:
                raise
            time.sleep(20 * (attempt + 1))
    raise RuntimeError("unreachable")


HAS_DIMENSION = "FILTER EXISTS { ?item wdt:P2043|wdt:P3039|wdt:P2049|wdt:P2048 ?any }"


def queries() -> dict[str, str]:
    out = {}
    for pid in PROPERTIES:
        out[f"quantity_{pid}"] = f"""
SELECT ?item ?amount ?unit ?applies ?rank WHERE {{
  VALUES ?cls {{ {CLASSES} }}
  ?item wdt:P31 ?cls ; p:{pid} ?st .
  ?st psv:{pid} ?node ; wikibase:rank ?rank .
  ?node wikibase:quantityAmount ?amount ; wikibase:quantityUnit ?unit .
  OPTIONAL {{ ?st pq:P518 ?applies }}
}}"""
    out["names"] = f"""
SELECT ?item ?name ?kind WHERE {{
  VALUES ?cls {{ {CLASSES} }}
  ?item wdt:P31 ?cls .
  {HAS_DIMENSION}
  {{ ?item rdfs:label ?name BIND("label" AS ?kind) }} UNION {{ ?item skos:altLabel ?name BIND("alias" AS ?kind) }}
  FILTER(LANG(?name) IN ("en", "de"))
}}"""
    values = " ".join(f"wdt:{p}" for p in LINK_PROPS)
    out["links"] = f"""
SELECT ?item ?pred ?value ?valueLabel WHERE {{
  VALUES ?cls {{ {CLASSES} }}
  VALUES ?pred {{ {values} }}
  ?item wdt:P31 ?cls ; ?pred ?value .
  {HAS_DIMENSION}
  OPTIONAL {{ ?value rdfs:label ?valueLabel FILTER(LANG(?valueLabel) = "en") }}
}}"""
    values = " ".join(f"wdt:{p}" for p in YEAR_PROPS)
    out["years"] = f"""
SELECT ?item ?pred ?time WHERE {{
  VALUES ?cls {{ {CLASSES} }}
  VALUES ?pred {{ {values} }}
  ?item wdt:P31 ?cls ; ?pred ?time .
  {HAS_DIMENSION}
}}"""
    return out


def fetch_answers(folder: str, log=print) -> None:
    """Asks the query service and keeps the answers in folder (the cache, or
    a scratch folder for the monthly data check)."""
    os.makedirs(folder, exist_ok=True)
    for name, query in queries().items():
        log(f"query {name} ...")
        result = sparql(query)
        rows = result["results"]["bindings"]
        with open(os.path.join(folder, name + ".json"), "w", encoding="utf-8", newline="\n") as f:
            json.dump(result, f, ensure_ascii=False)
        log(f"  {len(rows)} rows")
        time.sleep(3)


def current_rows(folder: str) -> list[dict]:
    """The Wikidata rows (as in dimensions.csv) the answers in folder give
    for today's catalog and aliases."""
    stats: collections.Counter = collections.Counter()
    items = load_items(load_answers(folder), stats)
    catalog = load_catalog()
    return rows_for(catalog, match(catalog, items, load_aliases()))


def do_fetch() -> int:
    fetch_answers(CACHE)
    return 0


# ---------------------------------------------------------------------------
# Reading the answers
# ---------------------------------------------------------------------------

def qid(uri: str) -> str:
    return uri[len(ENTITY):] if uri.startswith(ENTITY) else uri.rsplit("/", 1)[-1]


class Item:
    def __init__(self, ident: str):
        self.id = ident
        self.labels: dict[str, str] = {}
        self.names: list[str] = []
        self.links: dict[str, list[str]] = collections.defaultdict(list)
        self.years: dict[str, list[int]] = collections.defaultdict(list)
        # field -> [(value, applies-to items)]
        self.values: dict[str, list[tuple[int, set[str]]]] = collections.defaultdict(list)

    @property
    def label(self) -> str:
        return self.labels.get("en") or self.labels.get("de") or self.id

    def start(self) -> int | None:
        for p in ("P580", "P571", "P729"):
            if self.years.get(p):
                return min(self.years[p])
        return None

    def end(self) -> int | None:
        for p in ("P582", "P2669", "P730"):
            if self.years.get(p):
                return max(self.years[p])
        return None


def convert(field: str, amount: str, unit: str) -> int | None:
    """A quantity in the unit Wikidata states, in mm or kg; None for an
    unknown unit or a value outside the range of a road car."""
    try:
        value = float(amount)
    except ValueError:
        return None
    factors = MASS_UNITS if field == "mass" else LENGTH_UNITS
    factor = factors.get(qid(unit))
    if factor is None:
        return None
    result = int(round(value * factor))
    low, high = RANGES[field]
    return result if low <= result <= high else None


def load_items(answers: dict[str, dict], stats: collections.Counter) -> dict[str, Item]:
    items: dict[str, Item] = {}

    def item(uri: str) -> Item:
        ident = qid(uri)
        if ident not in items:
            items[ident] = Item(ident)
        return items[ident]

    for pid, field in PROPERTIES.items():
        grouped: dict[tuple[str, str, str], set[str]] = collections.defaultdict(set)
        for row in answers.get(f"quantity_{pid}", {}).get("results", {}).get("bindings", []):
            if row["rank"]["value"].endswith("DeprecatedRank"):
                stats["deprecated"] += 1
                continue
            key = (row["item"]["value"], row["amount"]["value"], row["unit"]["value"])
            applies = grouped[key]
            if "applies" in row:
                applies.add(qid(row["applies"]["value"]))
        for (uri, amount, unit), applies in sorted(grouped.items()):
            value = convert(field, amount, unit)
            if value is None:
                stats[f"skipped {field}"] += 1
                continue
            item(uri).values[field].append((value, applies))
    for row in answers.get("names", {}).get("results", {}).get("bindings", []):
        it = items.get(qid(row["item"]["value"]))
        if it is None:
            continue
        name = row["name"]["value"]
        lang = row["name"].get("xml:lang", "")
        if row["kind"]["value"] == "label":
            it.labels[lang] = name
        if name not in it.names:
            it.names.append(name)
    for row in answers.get("links", {}).get("results", {}).get("bindings", []):
        it = items.get(qid(row["item"]["value"]))
        if it is None:
            continue
        label = row.get("valueLabel", {}).get("value", "")
        it.links[qid(row["pred"]["value"])].append(label)
    for row in answers.get("years", {}).get("results", {}).get("bindings", []):
        it = items.get(qid(row["item"]["value"]))
        m = re.match(r"^[+-]?(\d{4})", row["time"]["value"])
        if it is None or not m:
            continue
        it.years[qid(row["pred"]["value"])].append(int(m.group(1)))
    for it in items.values():
        it.names.sort(key=lambda n: (n != it.labels.get("en"), n != it.labels.get("de"), n))
    return items


def pick(values: list[tuple[int, set[str]]], body: str) -> int:
    """One real value for a body: the one whose qualifier names the body, else
    the lower middle one of those without a qualifier (or of all)."""
    if not values:
        return 0
    wanted = BODY_PARTS.get(body, set())
    fitting = sorted(v for v, applies in values if applies & wanted)
    if not fitting:
        fitting = sorted(v for v, applies in values if not applies) or sorted(v for v, _ in values)
    return fitting[(len(fitting) - 1) // 2]


# ---------------------------------------------------------------------------
# The catalog
# ---------------------------------------------------------------------------

class Generation:
    def __init__(self, row: dict):
        self.brand = row["brand"]
        self.model = row["model"]
        self.generation = row["generation"]
        self.year_from = int(row["from"] or 0)
        self.year_to = int(row["to"] or 0)
        self.body = row["body"]
        self.aliases = [a.strip() for a in (row.get("aliases") or "").split("|") if a.strip()]

    @property
    def key(self) -> tuple[str, str, str]:
        return (self.brand, self.model, self.generation)


def load_catalog(folder: str = VEHICLES) -> list[Generation]:
    out = []
    for path in sorted(os.listdir(folder)):
        if not path.endswith(".csv") or path == "brands.csv":
            continue
        text = io.open(os.path.join(folder, path), encoding="utf-8-sig").read()
        lines = [line for line in text.splitlines() if line.strip() and not line.startswith("#")]
        out += [Generation(row) for row in csv.DictReader(lines)]
    return out


class Alias:
    """A line of aliases.csv: the items for a generation ("-" for none, several
    separated by "|", the first one with a value wins) and the values to leave
    out (a width with the mirrors, say)."""

    def __init__(self, row: dict):
        item = row["item"].strip()
        self.items = [] if item == "-" else [i.strip() for i in item.split("|") if i.strip()]
        self.skip = {s.strip() for s in (row.get("skip") or "").split("|") if s.strip()}
        for s in self.skip:
            if s not in FIELDS:
                raise SystemExit(f"error: {ALIASES}: unknown value {s!r} in skip")


def load_aliases(path: str = ALIASES) -> dict[tuple[str, str, str], Alias]:
    out = {}
    if not os.path.exists(path):
        return out
    text = io.open(path, encoding="utf-8-sig").read()
    lines = [line for line in text.splitlines() if line.strip() and not line.startswith("#")]
    for row in csv.DictReader(lines):
        out[(row["brand"], row["model"], row["generation"])] = Alias(row)
    return out


# ---------------------------------------------------------------------------
# Matching names
# ---------------------------------------------------------------------------

@functools.lru_cache(maxsize=None)
def fold(text: str) -> str:
    """Lower case without accents; every run of other characters one space."""
    text = unicodedata.normalize("NFKD", text.replace("ß", "ss"))
    text = "".join(c for c in text.lower() if not unicodedata.combining(c))
    return " ".join(re.sub(r"[^a-z0-9]+", " ", text).split())


ROMAN = {"i": 1, "ii": 2, "iii": 3, "iv": 4, "v": 5, "vi": 6, "vii": 7, "viii": 8, "ix": 9, "x": 10,
         "xi": 11, "xii": 12}
ORDINALS = {"first": 1, "second": 2, "third": 3, "fourth": 4, "fifth": 5, "sixth": 6, "seventh": 7,
            "eighth": 8, "ninth": 9, "tenth": 10, "eleventh": 11, "twelfth": 12,
            "erste": 1, "zweite": 2, "dritte": 3, "vierte": 4, "funfte": 5, "sechste": 6,
            "siebte": 7, "achte": 8, "neunte": 9, "zehnte": 10}
# Brand names in Wikidata that differ from the catalog.
BRAND_NAMES = {
    "Mercedes-Benz": ["Mercedes", "Mercedes Benz", "Daimler"],
    "Volkswagen": ["VW"],
    "Škoda": ["Skoda", "Skoda Auto"],
    "SEAT": ["Seat"],
    "Citroën": ["Citroen"],
    "Mini": ["MINI", "BMW Mini"],
    "Land Rover": ["Range Rover"],
    "Chevrolet": ["Chevy"],
    "Alfa Romeo": ["Alfa"],
    "Lada": ["VAZ", "AvtoVAZ"],
}
GENERATION_WORDS = {"generation", "gen", "generacion", "generazione", "baureihe", "series", "serie", "typ",
                    "type", "facelift", "phase", "mk", "mark", "der", "the", "model", "year"}


def generation_tokens(text: str) -> set[str]:
    return set(_generation_tokens(text))


@functools.lru_cache(maxsize=None)
def _generation_tokens(text: str) -> frozenset[str]:
    """Canonical tokens of a generation name: "#7" for a number in any
    spelling, codes in lower case ("f30", "8v", "w213"), "@2019" for a year."""
    out: set[str] = set()
    words = fold(text).split()
    i = 0
    while i < len(words):
        w = words[i]
        nxt = words[i + 1] if i + 1 < len(words) else ""
        m = re.fullmatch(r"(?:mk|mark)(\d{1,2})", w)
        if m:
            out.add("#" + m.group(1))
        elif w in ("mk", "mark") and nxt.isdigit():
            out.add("#" + nxt)
            i += 1
        elif w in ROMAN:
            out.add(f"#{ROMAN[w]}")
        elif w in ORDINALS:
            out.add(f"#{ORDINALS[w]}")
        elif re.fullmatch(r"(\d{1,2})(st|nd|rd|th)", w):
            out.add("#" + re.match(r"\d+", w).group(0))
        elif re.fullmatch(r"(19|20)\d\d", w):
            out.add("@" + w)
        elif w.isdigit() and len(w) <= 2:
            out.add("#" + str(int(w)))
        elif w not in GENERATION_WORDS:
            out.add(w)
        i += 1
    return frozenset(out)


def model_names(g: Generation) -> list[str]:
    """Folded names the model goes by: the catalog name and those aliases that
    are names rather than codes or engines."""
    names = [g.model]
    for alias in g.aliases:
        if re.fullmatch(r"[A-Za-z]{0,3}\d{2,4}[a-z]{0,3}", alias):
            continue  # a chassis code (F31, S213) or an engine (430i)
        names.append(alias)
    out = []
    for n in names:
        f = fold(n)
        if f and f not in out:
            out.append(f)
    # "3er" is "3 Series" in English, "E-Klasse" is "E-Class".
    extra = []
    for f in out:
        extra.append(re.sub(r"^(\d)er\b", r"\1 series", f))
        extra.append(re.sub(r"\bklasse\b", "class", f))
    return out + [e for e in extra if e not in out]


@functools.lru_cache(maxsize=None)
def brand_prefixes(brand: str) -> tuple[str, ...]:
    return tuple(sorted({fold(b) for b in [brand] + BRAND_NAMES.get(brand, [])}, key=len, reverse=True))


def split_name(name: str, brand: str, models: list[str]) -> str | None:
    """What is left of an item name after the brand and the model; None when
    the name is not this brand and model."""
    f = fold(name)
    for prefix in brand_prefixes(brand):
        if f == prefix or f.startswith(prefix + " "):
            rest = f[len(prefix):].strip()
            break
    else:
        return None
    for model in sorted(models, key=len, reverse=True):
        if not model:
            return rest
        if rest == model:
            return ""
        if rest.startswith(model + " "):
            return rest[len(model):].strip()
    return None


def code_tokens(g: Generation) -> tuple[set[str], set[str]]:
    """Tokens of the generation (its code or number) and of further body codes
    in the aliases (F31 next to F30)."""
    main = generation_tokens(g.generation)
    extra: set[str] = set()
    for alias in g.aliases:
        if re.fullmatch(r"[A-Za-z]{1,3}\d{2,3}", alias):
            extra |= generation_tokens(alias)
    # "Golf 4", "Octavia 3": a number after the model name.
    for alias in g.aliases:
        m = re.fullmatch(r".*\D\s(\d{1,2})", alias)
        if m and fold(alias).startswith(fold(g.model)):
            main.add("#" + m.group(1))
    return main, extra - main


def years_overlap(g: Generation, it: Item) -> bool:
    start = it.start()
    if start is None:
        return True
    end = it.end() or 9999
    to = g.year_to or 9999
    return start <= to + 1 and end >= g.year_from - 1


NUMBERS = {"#" + str(n) for n in range(1, 13)}
# A chassis or type code: letters and two to four digits (f30, w213, xp130), or 8v.
CODE = re.compile(r"[a-z]{1,3}\d{2,4}|\d[a-z]")


def score(g: Generation, it: Item, siblings: list[Generation], brand_ok: bool) -> float:
    """How well an item fits a generation; 0 for not at all. Higher is better:
    3 the generation code, 2 a number or year that fits, 1 the model itself
    with a single generation in the catalog."""
    models = model_names(g)
    main, extra = code_tokens(g)
    best = 0
    for name in it.names:
        rest = split_name(name, g.brand, models)
        if rest is None and brand_ok:
            # The brand is only in P1716/P176, the name starts with the model.
            rest = split_name(g.brand + " " + name, g.brand, models)
        if rest is None:
            # "BMW F20", "Mercedes-Benz W205": the brand and the chassis code.
            rest = split_name(name, g.brand, [""]) if any(CODE.fullmatch(t) for t in main) else None
            if not rest or generation_tokens(rest) != main:
                continue
        tokens = generation_tokens(rest)
        years = {int(t[1:]) for t in tokens if t.startswith("@")}
        codes = {t for t in tokens if not t.startswith("@")}
        # Further chassis codes next to the generation's own ("E170/E180").
        others = {t for t in codes - main - extra if CODE.fullmatch(t)}
        if codes and codes & main and not (codes - main - extra - NUMBERS - others):
            s = 3 if codes <= main | extra else 2
        elif not codes and years:
            s = 2 if any(abs(y - g.year_from) <= 1 for y in years) else 0
        elif not tokens:
            s = 1 if len(siblings) == 1 and years_overlap(g, it) else 0
        else:
            s = 0
        if s and not years_overlap(g, it):
            s = 0
        # The item's own name counts a little more than one of its aliases
        # ("Toyota Verso" over the Verso-S that lists it as an alias).
        if s and name in it.labels.values():
            s += 0.5
        best = max(best, s)
    return best


def match(catalog: list[Generation], items: dict[str, Item],
          aliases: dict) -> dict[tuple, tuple[list[Item], set[str]]]:
    """Generation key -> (its items, the values to leave out)."""
    by_model: dict[tuple[str, str], list[Generation]] = collections.defaultdict(list)
    for g in catalog:
        by_model[(g.brand, g.model)].append(g)
    # The items of each brand: by the start of a name, or by P1716/P176.
    by_brand: dict[str, list[tuple[Item, bool]]] = {}
    for brand in {g.brand for g in catalog}:
        prefixes = brand_prefixes(brand)
        found = []
        for it in items.values():
            linked = any(fold(b) in prefixes for b in it.links.get("P1716", []) + it.links.get("P176", []))
            named = any(fold(n) == p or fold(n).startswith(p + " ") for n in it.names for p in prefixes)
            if linked or named:
                found.append((it, linked))
        by_brand[brand] = found
    out: dict[tuple, tuple[list[Item], set[str]]] = {}
    for g in catalog:
        alias = aliases.get(g.key)
        if alias is not None and alias.items:
            for ident in alias.items:
                if ident not in items:
                    raise SystemExit(f"error: {ALIASES}: {ident} for {' '.join(g.key)} has no dimensions in Wikidata")
            out[g.key] = ([items[i] for i in alias.items], alias.skip)
            continue
        if alias is not None and not alias.skip:
            continue  # blocked
        skip = alias.skip if alias is not None else set()
        siblings = by_model[(g.brand, g.model)]
        scored = [(score(g, it, siblings, linked), it) for it, linked in by_brand[g.brand]]
        scored = [(s, it) for s, it in scored if s > 0]
        if not scored:
            continue
        top = max(s for s, _ in scored)
        best = [it for s, it in scored if s == top]
        if len(best) > 1:
            # Several items for one generation (the saloon and the estate as
            # their own items): the one whose name has nothing but the code.
            plain = [it for it in best if any(generation_tokens(split_name(n, g.brand, model_names(g)) or "x")
                                              <= code_tokens(g)[0] for n in it.names)]
            best = plain if len(plain) >= 1 else best
            best.sort(key=lambda it: (-sum(1 for f in MAIN if it.values.get(f)), int(it.id[1:])))
        out[g.key] = ([best[0]], skip)
    return out


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

CSV_HEADER = ["brand", "model", "generation"] + FIELDS + ["item", "label"]


def rows_for(catalog: list[Generation], matched: dict) -> list[dict]:
    rows = []
    for g in catalog:
        if g.key not in matched:
            continue
        found, skip = matched[g.key]
        values = {f: 0 for f in FIELDS}
        used = []
        for it in found:
            for f in FIELDS:
                if not values[f] and f not in skip:
                    values[f] = pick(it.values.get(f, []), g.body)
                    if values[f] and it.id not in used:
                        used.append(it.id)
        if not any(values[f] for f in MAIN):
            continue
        rows.append({"brand": g.brand, "model": g.model, "generation": g.generation, **values,
                     "item": "|".join(used), "label": found[0].label})
    rows.sort(key=lambda r: (r["brand"], r["model"], r["generation"]))
    return rows


def write_csv(path: str, rows: list[dict]) -> None:
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("# Outer dimensions per catalog generation from Wikidata (CC0), by tools/gen_dimensions.py --match.\n")
        f.write("# Millimetres and kilograms; 0 where Wikidata has no value. Do not edit by hand:\n")
        f.write("# set or block an item in aliases.csv and run --match again.\n")
        w = csv.DictWriter(f, fieldnames=CSV_HEADER, lineterminator="\n")
        w.writeheader()
        for r in rows:
            w.writerow({k: (v if v != 0 else "") if k in FIELDS else v for k, v in r.items()})


def read_csv(path: str) -> list[dict]:
    text = io.open(path, encoding="utf-8-sig").read()
    lines = [line for line in text.splitlines() if line.strip() and not line.startswith("#")]
    out = []
    for row in csv.DictReader(lines):
        for f in FIELDS:
            row[f] = int(row[f] or 0)
        out.append(row)
    return out


def rust_str(text: str) -> str:
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def read_manual(path: str, catalog: set[tuple[str, str, str]]) -> dict[tuple[str, str, str], dict]:
    """The rows of data/dimensions_manual.csv by generation: the values (0 for
    an empty field) and the document they come from. Every row needs a
    catalog generation, a source and sane values."""
    out: dict[tuple[str, str, str], dict] = {}
    if not os.path.exists(path):
        return out
    text = io.open(path, encoding="utf-8-sig").read()
    lines = [line for line in text.splitlines() if line.strip() and not line.startswith("#")]
    for row in csv.DictReader(lines):
        key = (row["brand"], row["model"], row["generation"])
        where = f"{path}: {' '.join(key)}"
        if key not in catalog:
            raise SystemExit(f"error: {where} is not in the catalog")
        if key in out:
            raise SystemExit(f"error: {where} is listed twice")
        source = (row.get("source") or "").strip()
        if not source or not re.search(r"(19|20)\d\d", source):
            raise SystemExit(f"error: {where}: the source needs the document and its year")
        if "\u2014" in source or "\u2013" in source:
            raise SystemExit(f"error: {where}: no long dashes in the source")
        values = {}
        for f in MAIN:
            v = int(row.get(f) or 0)
            lo, hi = RANGES[f]
            if v and not lo <= v <= hi:
                raise SystemExit(f"error: {where}: {f} {v} is outside {lo} to {hi}")
            values[f] = v
        if not any(values.values()):
            raise SystemExit(f"error: {where}: no values")
        if values["length"] and values["wheelbase"] and values["wheelbase"] >= values["length"]:
            raise SystemExit(f"error: {where}: the wheelbase is not shorter than the car")
        out[key] = {"values": values, "source": source}
    return out


def merge_manual(rows: list[dict], manual: dict[tuple[str, str, str], dict]) -> list[dict]:
    """The Wikidata rows with the makers' values on top: a value of the manual
    table wins, Wikidata fills what it leaves empty. "doc" names the
    document when it gave at least one value."""
    out = {}
    for r in rows:
        out[(r["brand"], r["model"], r["generation"])] = {**r, "doc": ""}
    for key, m in manual.items():
        r = out.get(key) or {"brand": key[0], "model": key[1], "generation": key[2],
                             **{f: 0 for f in FIELDS}, "item": "", "label": "", "doc": ""}
        # The Wikidata item stays named only while one of its values is still used.
        if not any(r[f] and not m["values"].get(f) for f in FIELDS):
            r["item"] = ""
        for f, v in m["values"].items():
            if v:
                r[f] = v
        r["doc"] = m["source"]
        out[key] = r
    return sorted(out.values(), key=lambda r: (r["brand"], r["model"], r["generation"]))


def write_rust(rows: list[dict], total: int) -> None:
    complete = sum(1 for r in rows if all(r[f] for f in ("length", "width", "height", "wheelbase")))
    manual = sum(1 for r in rows if r.get("doc"))
    lines = [
        "// Generated by tools/gen_dimensions.py from data/wikidata/dimensions.csv and",
        "// data/dimensions_manual.csv, do not edit by hand.",
        f"// Dimensions: {len(rows)} of {total} generations, {complete} with length, width, height and wheelbase;",
        f"// {manual} with values from the makers' technical data, the rest from Wikidata (CC0).",
        "",
        "use crate::anatomy::Measured as M;",
        "",
        "/// Sorted by brand, model and generation (byte order) for a binary search.",
        "#[rustfmt::skip]",
        "pub static MEASURED: &[M] = &[",
    ]
    for r in sorted(rows, key=lambda r: (r["brand"].encode(), r["model"].encode(), r["generation"].encode())):
        values = ", ".join(f"{f}: {r[f]}" for f in FIELDS)
        lines.append(f"    M {{ brand: {rust_str(r['brand'])}, model: {rust_str(r['model'])}, "
                     f"generation: {rust_str(r['generation'])}, {values}, item: {rust_str(r['item'])}, "
                     f"doc: {rust_str(r.get('doc', ''))} }},")
    lines.append("];")
    os.makedirs(os.path.dirname(TARGET_RS), exist_ok=True)
    with open(TARGET_RS, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")


def coverage(rows: list[dict], total: int) -> str:
    complete = sum(1 for r in rows if all(r[f] for f in ("length", "width", "height", "wheelbase")))
    per = {f: sum(1 for r in rows if r[f]) for f in FIELDS}
    manual = sum(1 for r in rows if r.get("doc"))
    return (f"{len(rows)} of {total} generations have dimensions ({100 * len(rows) / max(1, total):.1f} %), "
            f"{manual} of them from the makers' data, "
            f"{complete} complete (length, width, height, wheelbase); "
            + ", ".join(f"{f} {n}" for f, n in per.items()))


def load_answers(folder: str) -> dict[str, dict]:
    answers = {}
    for name in list(queries()):
        path = os.path.join(folder, name + ".json")
        if os.path.exists(path):
            answers[name] = json.load(io.open(path, encoding="utf-8"))
    return answers


def do_match() -> int:
    answers = load_answers(CACHE)
    if not answers:
        print(f"error: no answers in {CACHE}, run --fetch first", file=sys.stderr)
        return 1
    stats: collections.Counter = collections.Counter()
    items = load_items(answers, stats)
    catalog = load_catalog()
    matched = match(catalog, items, load_aliases())
    rows = rows_for(catalog, matched)
    os.makedirs(DATA, exist_ok=True)
    write_csv(TABLE, rows)
    merged = merge_manual(rows, read_manual(MANUAL, {g.key for g in catalog}))
    write_rust(merged, len(catalog))
    print(f"{len(items)} Wikidata items with dimensions; skipped: "
          + ", ".join(f"{k} {v}" for k, v in sorted(stats.items())))
    print(coverage(merged, len(catalog)))
    if "--verbose" in sys.argv:
        for r in rows:
            print(f"  {r['brand']} {r['model']} {r['generation']}: {r['item']} {r['label']}")
    return 0


def do_table() -> int:
    rows = read_csv(TABLE)
    catalog = {g.key for g in load_catalog()}
    for r in rows:
        if (r["brand"], r["model"], r["generation"]) not in catalog:
            print(f"error: {TABLE}: {r['brand']} {r['model']} {r['generation']} is not in the catalog; "
                  "run --match again", file=sys.stderr)
            return 1
    merged = merge_manual(rows, read_manual(MANUAL, catalog))
    write_rust(merged, len(catalog))
    print(coverage(merged, len(catalog)))
    return 0


# ---------------------------------------------------------------------------
# Test fixtures
# ---------------------------------------------------------------------------

def do_fixtures() -> int:
    """Converts and matches the small recorded answers in the fixtures folder
    against the small catalog there, writes the result next to them (the CI
    compares it with the committed file) and checks the expectations."""
    folder = os.path.join(FIXTURES, "wikidata")
    answers = load_answers(folder)
    stats: collections.Counter = collections.Counter()
    items = load_items(answers, stats)
    catalog = load_catalog(os.path.join(folder, "catalog"))
    aliases = load_aliases(os.path.join(folder, "aliases.csv"))
    rows = rows_for(catalog, match(catalog, items, aliases))
    out = os.path.join(FIXTURES, "wikidata_dimensions.csv")
    write_csv(out, rows)
    expect = json.load(io.open(os.path.join(folder, "expect.json"), encoding="utf-8"))
    got = {f"{r['brand']}|{r['model']}|{r['generation']}": r for r in rows}
    failed = 0
    for key, want in expect["rows"].items():
        row = got.get(key)
        if want is None:
            if row is not None:
                print(f"error: {key} should have no match, got {row['item']}", file=sys.stderr)
                failed += 1
            continue
        for field, value in want.items():
            if row is None or row[field] != value:
                print(f"error: {key} {field}: want {value}, got {row and row[field]}", file=sys.stderr)
                failed += 1
    for key, value in expect.get("skipped", {}).items():
        if stats[key] != value:
            print(f"error: skipped {key}: want {value}, got {stats[key]}", file=sys.stderr)
            failed += 1
    for text, tokens in expect.get("tokens", {}).items():
        if sorted(generation_tokens(text)) != sorted(tokens):
            print(f"error: tokens of {text!r}: want {tokens}, got {sorted(generation_tokens(text))}", file=sys.stderr)
            failed += 1
    print(f"{out}: {len(rows)} generations matched, {len(expect['rows'])} expectations, {failed} failed")
    return 1 if failed else 0


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    if "--fetch" in sys.argv:
        return do_fetch()
    if "--match" in sys.argv:
        return do_match()
    if "--fixtures" in sys.argv:
        return do_fixtures()
    return do_table()


if __name__ == "__main__":
    sys.exit(main())
