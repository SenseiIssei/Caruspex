// Demo answers for the knowledge pages: the vehicle catalog with its search
// (VehicleCatalog.cpp), the VIN decoder (Vin.cpp) and the manufacturer fault
// code tables (DtcBrands.gen.cpp). The tables are a JSON copy of the C++ ones
// (scripts/extract-catalog.py) and load on first use, so the preview does
// not carry them until a knowledge page asks.
import type { BrandFamily, CatalogMatch, DtcLexiconEntry, Severity, VehicleDiagnosisInfo, VehicleGenerationInfo, VinInfo } from "../../types";
import { L } from "../core";
import { genericCount, lexicon, severityLabel, systemName } from "../dtc";
import type { PartialHandlers } from "../runtime";

// [brand, country de, country en, group]
type BrandRecord = [string, string, string, string];
// [brand, model, generation, yearFrom, yearTo, body, powertrains, connector,
//  lateConnector, connectorChange, bus, protocolOrder, location, note de,
//  note en, vinModelCodes, aliases, usMarket]
type GenRecord = [string, string, string, number, number, number, number, number, number, number, number, string, number, string, string, string, string, boolean];
// [wmi, manufacturer, brands, yearRule, codeRule, country override]
type WmiRecord = [string, string, string, number, number, [string, string] | null];
// [first, from, to, de, en]
type CountryRecord = [string, string, string, string, string];
// [code, de, en, severity, system, detail, throttle]
type BrandCode = [string, string, string, number, number, number, boolean];
// [key, label de, label en, brands, records]
type FamilyRecord = [string, string, string, string, BrandCode[]];

// [length, width, height, wheelbase, track, mass, Wikidata item, maker's
// document ("" for Wikidata alone)]; 0 = none.
export type MeasuredRecord = [number, number, number, number, number, number, string, string];

interface CatalogData {
  brands: BrandRecord[];
  generations: GenRecord[];
  wmi: WmiRecord[];
  countries: CountryRecord[];
  // Dimensions from the makers and Wikidata by generation index
  // (generated/dimensions.rs).
  measured: Record<string, MeasuredRecord>;
}

// --- Enums of VehicleCatalog.h and Vehicles.h ------------------------------------------

const Connector = { None: 0, Obd2: 1, Vag2x2: 2, Bmw20: 3, Mb38: 4, Other: 5 } as const;
const Location = { Unknown: 0, EngineBay: 8 } as const;
const Bus = { KLine: 0, Can: 1, Pwm: 2, Vpw: 3, Mixed: 4 } as const;
const Support = { None: 0, Manufacturer: 1, Kkl: 2, Partial: 3, Full: 4 } as const;
const PtPetrol = 1;
const PtDiesel = 2;
const PtHybrid = 4;
const PtPlugIn = 8;
const PtElectric = 16;
const PtGas = 32;

// Vin.h VinYearRule and VinModelCodeRule.
const YearRule = { Standard: 0, Unknown: 1, None: 2 } as const;
const CodeRule = { None: 0, Vag78: 1, Mercedes456: 2, Porsche78: 3, Tesla4: 4 } as const;

// --- Loading -------------------------------------------------------------------------------

let catalogPromise: Promise<CatalogData> | null = null;
let familiesPromise: Promise<FamilyRecord[]> | null = null;

function loadCatalog(): Promise<CatalogData> {
  catalogPromise ??= import("../data/catalog.json").then((m) => m.default as unknown as CatalogData);
  return catalogPromise;
}

// Brand, model, body and engines of one generation, for the X-ray demo.
export async function catalogGeneration(id: number): Promise<{
  brand: string;
  model: string;
  generation: string;
  body: number;
  powertrains: number;
  yearFrom: number;
  measured: MeasuredRecord | null;
} | null> {
  const data = await loadCatalog();
  const g = data.generations[id];
  return g
    ? { brand: g[0], model: g[1], generation: g[2], body: g[5], powertrains: g[6], yearFrom: g[3], measured: data.measured[String(id)] ?? null }
    : null;
}

function loadFamilies(): Promise<FamilyRecord[]> {
  familiesPromise ??= import("../data/dtc-brands.json").then((m) => (m.default as unknown as { families: FamilyRecord[] }).families);
  return familiesPromise;
}

// --- Text helpers of VehicleCatalog.cpp ------------------------------------------------

// The newest model year a "still built" generation can have.
const OPEN_END = 2027;

const endYear = (g: GenRecord) => (g[4] ? g[4] : OPEN_END);
const covers = (g: GenRecord, year: number) => year >= g[3] && year <= endYear(g);

// Lower case without accents, so "Škoda", "skoda" and "SKODA" are equal and
// "Citroen" finds "Citroën".
function fold(text: string): string {
  return text
    .toLowerCase()
    .replace(/ß/g, "ss")
    .replace(/ø/g, "o")
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "");
}

const same = (a: string, b: string) => fold(a) === fold(b);
const isAlnum = (c: string) => (c >= "a" && c <= "z") || (c >= "0" && c <= "9");

// Splits folded text into words at everything that is not a letter or digit.
function words(folded: string): string[] {
  const out: string[] = [];
  let cur = "";
  for (const c of folded) {
    if (isAlnum(c)) cur += c;
    else if (cur) {
      out.push(cur);
      cur = "";
    }
  }
  if (cur) out.push(cur);
  return out;
}

const joined = (folded: string) => [...folded].filter(isAlnum).join("");
const isDigits = (s: string) => /^[0-9]+$/.test(s);

const ROMAN = ["i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x", "xi", "xii", "xiii", "xiv"];

// Ordinal of a generation name: "IV" -> 4, "Mk4" -> 4, otherwise 0.
function ordinal(g: string): number {
  const r = ROMAN.indexOf(g);
  if (r >= 0) return r + 1;
  if (g.length > 2 && g.startsWith("mk") && isDigits(g.slice(2))) return Number(g.slice(2));
  return 0;
}

interface SearchKeys {
  brand: string[];
  model: string[];
  modelWords: string[];
  generation: string[];
  alias: string[];
}

function addUnique(v: string[], s: string): void {
  if (s && !v.includes(s)) v.push(s);
}

function buildKeys(g: GenRecord): SearchKeys {
  const k: SearchKeys = { brand: [], model: [], modelWords: [], generation: [], alias: [] };
  const brand = fold(g[0]);
  for (const w of words(brand)) addUnique(k.brand, w);
  addUnique(k.brand, joined(brand));
  const model = fold(g[1]);
  k.modelWords = words(model);
  for (const w of k.modelWords) addUnique(k.model, w);
  addUnique(k.model, joined(model));
  const gen = fold(g[2]);
  for (const w of words(gen)) addUnique(k.generation, w);
  addUnique(k.generation, joined(gen));
  const n = ordinal(joined(gen));
  if (n) {
    addUnique(k.generation, String(n));
    addUnique(k.generation, ROMAN[n - 1]!);
    addUnique(k.generation, `mk${n}`);
  }
  for (const w of words(fold(g[15]))) addUnique(k.generation, w);
  for (const phrase of fold(g[16]).split("|")) {
    for (const w of words(phrase)) addUnique(k.alias, w);
    addUnique(k.alias, joined(phrase));
  }
  return k;
}

function prefixOf(v: string[], s: string): boolean {
  if (s.length < 3) return false;
  return v.some((w) => w.length > s.length && w.startsWith(s));
}

// Points for one query word; 0 = no hit.
function scoreWord(k: SearchKeys, w: string): number {
  if (k.generation.includes(w)) return 6;
  if (k.model.includes(w)) return 5;
  if (k.alias.includes(w)) return 4;
  if (k.brand.includes(w)) return 3;
  if (prefixOf(k.model, w) || prefixOf(k.brand, w) || prefixOf(k.alias, w)) return 2;
  return 0;
}

// "golf4" -> "golf", "4"; used when a word as a whole finds nothing.
function splitLettersDigits(w: string): string[] {
  const out: string[] = [];
  let cur = "";
  const digit = (c: string) => c >= "0" && c <= "9";
  for (const c of w) {
    if (cur && digit(c) !== digit(cur[cur.length - 1]!)) {
      out.push(cur);
      cur = "";
    }
    cur += c;
  }
  if (cur) out.push(cur);
  return out;
}

let keysCache: SearchKeys[] | null = null;
function allKeys(data: CatalogData): SearchKeys[] {
  keysCache ??= data.generations.map(buildKeys);
  return keysCache;
}

function brandGroup(data: CatalogData, brand: string): string {
  return data.brands.find((b) => same(b[0], brand))?.[3] ?? "";
}

const onlyElectric = (pt: number) => pt !== 0 && (pt & ~PtElectric) === 0;

export function bodyName(body: number): string {
  switch (body) {
    case 0: return L("Schrägheck", "Hatchback");
    case 1: return L("Limousine", "Sedan");
    case 2: return L("Kombi", "Estate");
    case 3: return L("SUV", "SUV");
    case 4: return L("Transporter", "Van");
    case 5: return L("Coupé", "Coupe");
    case 6: return L("Cabrio", "Convertible");
    case 7: return L("Pick-up", "Pickup");
    case 8: return L("Van (MPV)", "MPV");
    case 9: return L("Kleinstwagen", "Microcar");
    case 10: return L("Roadster", "Roadster");
  }
  return "";
}

function connectorName(c: number): string {
  switch (c) {
    case Connector.None: return L("kein Diagnosestecker", "no diagnostic connector");
    case Connector.Obd2: return L("16-polige OBD-II-Buchse", "16-pin OBD-II socket");
    case Connector.Vag2x2: return L("VAG-Rundstecker (2x2), KKL-Kabel", "VAG round plugs (2x2), KKL cable");
    case Connector.Bmw20: return L("BMW-Rundstecker, 20 Pole", "BMW round plug, 20 pins");
    case Connector.Mb38: return L("Mercedes-Rundstecker, 38 Pole", "Mercedes round plug, 38 pins");
    case Connector.Other: return L("herstellereigener Stecker", "manufacturer plug");
  }
  return "";
}

function socketText(location: number): string {
  switch (location) {
    case 0:
      return L("Meist unter dem Armaturenbrett auf der Fahrerseite, manchmal hinter einer Abdeckung oder in der Mittelkonsole.",
        "Usually under the dashboard on the driver side, sometimes behind a cover or in the centre console.");
    case 1:
      return L("Unter dem Armaturenbrett auf der Fahrerseite, nahe der Lenksäule.",
        "Under the dashboard on the driver side, near the steering column.");
    case 2: return L("Direkt unter der Lenksäule.", "Directly below the steering column.");
    case 3: return L("Im Fahrerfußraum hinter einer Abdeckung.", "In the driver's footwell behind a cover.");
    case 4:
      return L("Am oder im Sicherungskasten auf der Fahrerseite, hinter dessen Deckel.",
        "At or in the fuse box on the driver side, behind its lid.");
    case 5:
      return L("In der Mittelkonsole hinter einer Abdeckung vor dem Schalthebel.",
        "In the centre console behind a cover in front of the gear lever.");
    case 6:
      return L("Hinter oder unter dem Aschenbecher in der Mittelkonsole.",
        "Behind or below the ashtray in the centre console.");
    case 7: return L("Unter dem Armaturenbrett auf der Beifahrerseite.", "Under the dashboard on the passenger side.");
    case 8: return L("Im Motorraum, meist unter einer runden Kappe.", "In the engine bay, usually under a round cap.");
    case 9:
      return L("Hinten an der Mittelkonsole beziehungsweise unter den Rücksitzen.",
        "At the rear of the centre console or under the rear seats.");
  }
  return "";
}

export function powertrainText(pt: number): string {
  const names: [number, string, string][] = [
    [PtPetrol, "Benzin", "Petrol"],
    [PtDiesel, "Diesel", "Diesel"],
    [PtHybrid, "Hybrid", "Hybrid"],
    [PtPlugIn, "Plug-in-Hybrid", "Plug-in hybrid"],
    [PtElectric, "Elektro", "Electric"],
    [PtGas, "Gas (LPG/CNG)", "Gas (LPG/CNG)"],
  ];
  return names.filter(([flag]) => pt & flag).map(([, de, en]) => L(de, en)).join(", ");
}

function supportName(s: number): string {
  switch (s) {
    case Support.None: return L("keine Diagnose", "no diagnosis");
    case Support.Manufacturer: return L("nur Herstellerdiagnose", "manufacturer diagnosis only");
    case Support.Kkl: return L("KKL-Kabel (KWP1281)", "KKL cable (KWP1281)");
    case Support.Partial: return L("OBD-II teilweise", "OBD-II in part");
    case Support.Full: return L("OBD-II vollständig", "full OBD-II");
  }
  return "";
}

function yearsText(g: GenRecord): string {
  if (g[4] === 0) return L("seit ", "since ") + g[3];
  if (g[4] === g[3]) return String(g[3]);
  return `${g[3]}${L(" bis ", " to ")}${g[4]}`;
}

function displayName(g: GenRecord): string {
  let out = `${g[0]} ${g[1]}`;
  if (g[2] && fold(g[2]) !== fold(g[1])) out += ` ${g[2]}`;
  return `${out} (${yearsText(g)})`;
}

function connectorFor(g: GenRecord, year: number): number {
  const y = year <= 0 ? g[3] : year;
  return g[9] && y >= g[9] ? g[8] : g[7];
}

function kklPossible(data: CatalogData, g: GenRecord, year: number): boolean {
  const y = year <= 0 ? g[3] : year;
  if (brandGroup(data, g[0]) !== "vag" || y < 1990) return false;
  const c = connectorFor(g, y);
  if (c === Connector.Vag2x2) return true;
  return c === Connector.Obd2 && (g[10] === Bus.KLine || g[10] === Bus.Mixed) && y <= 2008;
}

function supportFor(g: GenRecord, year: number, powertrain = 0): number {
  const y = year <= 0 ? g[3] : year;
  const pt = powertrain || g[6];
  switch (connectorFor(g, y)) {
    case Connector.None: return Support.None;
    case Connector.Vag2x2: return Support.Kkl;
    case Connector.Bmw20:
    case Connector.Mb38:
    case Connector.Other: return Support.Manufacturer;
  }
  if (onlyElectric(pt)) return Support.Partial;
  if (g[17]) return y >= 1996 ? Support.Full : Support.Partial;
  // EU: petrol cars (and hybrids, gas cars) from 2001, diesels from 2004.
  const petrolLike = (pt & (PtPetrol | PtHybrid | PtPlugIn | PtGas)) !== 0;
  return y >= (petrolLike ? 2001 : 2004) ? Support.Full : Support.Partial;
}

// Location with the connector taken into account (an unknown spot of a BMW
// round plug is still the engine bay).
function locationText(g: GenRecord, year: number): string {
  if (g[12] !== Location.Unknown) return socketText(g[12]);
  switch (connectorFor(g, year)) {
    case Connector.None: return L("Kein Diagnosestecker.", "No diagnostic connector.");
    case Connector.Bmw20:
    case Connector.Mb38: return socketText(Location.EngineBay);
    case Connector.Vag2x2:
      return L("Je nach Modell im Sicherungskasten unter dem Armaturenbrett, unter der Schaltkulisse oder im Motorraum.",
        "Depending on the model in the fuse box under the dashboard, under the gear lever gaiter or in the engine bay.");
    case Connector.Other:
      return L("Herstellerstecker, meist im Motorraum oder unter dem Armaturenbrett.",
        "Manufacturer plug, usually in the engine bay or under the dashboard.");
  }
  return socketText(Location.Unknown);
}

function diagnosisText(data: CatalogData, g: GenRecord, year: number): string {
  const y = year <= 0 ? g[3] : year;
  const c = connectorFor(g, y);
  const kkl = kklPossible(data, g, y);
  let out = "";
  switch (supportFor(g, y)) {
    case Support.None:
      out = L("Dieses Auto hat noch keine Diagnose, die ein Adapter auslesen kann. Fehler zeigen sich höchstens als Blinkcode.",
        "This car has no diagnosis yet that an adapter could read. Faults show up as blink codes at most.");
      break;
    case Support.Kkl:
      out = L("VAG-Rundstecker (2x2 Pole). Mit einem KKL-Kabel liest Caruspex Fehlerspeicher und Messwerte über KWP1281.",
        "VAG round plugs (2x2 pins). With a KKL cable Caruspex reads fault memory and live values over KWP1281.");
      break;
    case Support.Manufacturer:
      if (c === Connector.Bmw20) {
        out = L("BMW-Rundstecker mit 20 Polen im Motorraum. Ein ELM327 versteht ihn nicht, dafür braucht es ein BMW-Diagnosegerät.",
          "BMW round 20-pin plug in the engine bay. An ELM327 does not understand it, that takes a BMW diagnostic tool.");
      } else if (c === Connector.Mb38) {
        out = L("Mercedes-Rundstecker mit 38 Polen im Motorraum. Ein ELM327 versteht ihn nicht, dafür braucht es ein Mercedes-Diagnosegerät.",
          "Mercedes round 38-pin plug in the engine bay. An ELM327 does not understand it, that takes a Mercedes diagnostic tool.");
      } else if (y >= 2008) {
        out = L("Herstellereigener Diagnoseanschluss statt der genormten OBD-II-Buchse. Ein ELM327 kann damit nicht ohne Weiteres sprechen.",
          "Manufacturer diagnostic connector instead of the standard OBD-II socket. An ELM327 cannot simply talk to it.");
      } else {
        out = L("Herstellereigener Diagnosestecker, falls vorhanden. Ein ELM327 kann damit nicht sprechen, oft lassen sich aber Blinkcodes auslesen.",
          "Manufacturer diagnostic plug, if fitted. An ELM327 cannot talk to it, but blink codes can often be read.");
      }
      break;
    case Support.Partial:
      if (onlyElectric(g[6])) {
        out = L("Reines Elektroauto: Abgaswerte gibt es nicht, daher antworten oft nur wenige OBD-Messwerte und kaum Fehlercodes.",
          "Pure electric car: there are no emissions to report, so often only a few OBD values and hardly any trouble codes answer.");
      } else {
        out = L("16-polige OBD-Buchse vorhanden, aber OBD war für dieses Baujahr noch keine Pflicht (EU: Benziner ab 2001, Diesel ab 2004; USA ab 1996). Viele Motorsteuergeräte antworten trotzdem, ein Versuch lohnt sich.",
          "16-pin OBD socket present, but OBD was not yet mandatory for this model year (EU: petrol from 2001, diesel from 2004; USA from 1996). Many engine control units answer anyway, it is worth a try.");
      }
      if (kkl) out += L(" Sicher klappt es mit einem KKL-Kabel über KWP1281.", " A KKL cable with KWP1281 works for sure.");
      break;
    case Support.Full:
      out = L("Volle OBD-II-Diagnose über die 16-polige Buchse: Fehlercodes, Messwerte und Bereitschaftstests.",
        "Full OBD-II diagnosis through the 16-pin socket: trouble codes, live values and readiness tests.");
      if (!g[17] && g[6] & PtDiesel && y < 2004) {
        out += L(" Bei Dieselmotoren dieses Baujahrs war OBD noch keine Pflicht.", " For diesel engines of this model year OBD was not yet mandatory.");
      }
      if (kkl) out += L(" Die übrigen Steuergeräte erreicht Caruspex mit einem KKL-Kabel.", " Caruspex reaches the other control units with a KKL cable.");
      break;
  }
  if (g[9] && y < g[9] && g[8] !== g[7]) {
    out += ` ${L("Ab etwa %d: ", "From about %d: ").replace("%d", String(g[9]))}${connectorName(g[8])}.`;
  }
  const note = L(g[13], g[14]);
  if (note) out += ` ${note}`;
  return out;
}

function info(data: CatalogData, id: number): VehicleGenerationInfo {
  const g = data.generations[id]!;
  return {
    id,
    brand: g[0],
    model: g[1],
    generation: g[2],
    yearFrom: g[3],
    yearTo: g[4],
    body: g[5],
    bodyName: bodyName(g[5]),
    powertrains: g[6],
    powertrainText: powertrainText(g[6]),
    displayName: displayName(g),
    yearsText: yearsText(g),
    protocolOrder: g[11],
    note: g[13] || g[14] ? L(g[13], g[14]) : "",
    usMarket: g[17],
  };
}

const byFold = (a: string, b: string) => (fold(a) < fold(b) ? -1 : fold(a) > fold(b) ? 1 : 0);

function brands(data: CatalogData): string[] {
  return data.brands.map((b) => b[0]).sort(byFold);
}

function models(data: CatalogData, brand: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const g of data.generations) {
    if (same(g[0], brand) && !seen.has(fold(g[1]))) {
      seen.add(fold(g[1]));
      out.push(g[1]);
    }
  }
  return out.sort(byFold);
}

function generations(data: CatalogData, brand: string, model: string): number[] {
  const out: number[] = [];
  data.generations.forEach((g, i) => {
    if (same(g[0], brand) && same(g[1], model)) out.push(i);
  });
  // Array.prototype.sort is stable, like std::stable_sort.
  return out.sort((a, b) => data.generations[a]![3] - data.generations[b]![3]);
}

// Tolerant search: "golf 4", "a2", "e46", "w203", "octavia 2008", "model 3".
function search(data: CatalogData, query: string, limit: number): number[] {
  const tokens = words(fold(query));
  if (tokens.length === 0) return [];
  const t = data.generations;
  const keys = allKeys(data);
  let yearQuery = false;
  let queryYear = 0;
  const hits: { index: number; score: number }[] = [];

  for (let i = 0; i < t.length; i++) {
    const k = keys[i]!;
    let score = 0;
    let ok = true;
    const matched: string[] = [];
    for (const w of tokens) {
      const s = scoreWord(k, w);
      if (s > 0) {
        score += s;
        matched.push(w);
        continue;
      }
      // A year: filters the generations instead of matching text.
      if (w.length === 4 && isDigits(w)) {
        const year = Number(w);
        if (year >= 1950 && year <= 2049) {
          yearQuery = true;
          queryYear = year;
          if (covers(t[i]!, year)) {
            score += 2;
            continue;
          }
          ok = false;
          break;
        }
      }
      // "golf4" or "e46i": try the parts on their own.
      const parts = splitLettersDigits(w);
      if (parts.length > 1) {
        let partScore = 0;
        let allParts = true;
        for (const p of parts) {
          const ps = scoreWord(k, p);
          if (ps === 0) {
            allParts = false;
            break;
          }
          partScore += ps;
          matched.push(p);
        }
        if (allParts) {
          score += partScore;
          continue;
        }
      }
      ok = false;
      break;
    }
    if (!ok) continue;
    // Bonus when the query names the whole model, so "golf 4" prefers the
    // Golf over the Golf Cabriolet.
    if (k.modelWords.length > 0 && k.modelWords.every((mw) => matched.includes(mw))) score += 3;
    hits.push({ index: i, score });
  }

  hits.sort((a, b) => {
    if (a.score !== b.score) return b.score - a.score;
    if (yearQuery) {
      // Among equal hits the generation that had just started that year is
      // more likely than one at the end of its life.
      const ca = covers(t[a.index]!, queryYear);
      const cb = covers(t[b.index]!, queryYear);
      if (ca !== cb) return ca ? -1 : 1;
      return t[b.index]![3] - t[a.index]![3];
    }
    return 0;
  });
  return hits.slice(0, limit).map((h) => h.index);
}

// --- VIN decoder (Vin.cpp) ---------------------------------------------------------------

// ISO 3780 sorts the second character in this order; ranges like "AA-AH" are
// meant along it.
const RANGE_ORDER = "ABCDEFGHJKLMNPRSTUVWXYZ1234567890";
const rangeIndex = (c: string) => (c ? RANGE_ORDER.indexOf(c) : -1);

function findWmi(data: CatalogData, vin: string): WmiRecord | null {
  if (vin.length < 2) return null;
  const key = vin.slice(0, 3).toUpperCase();
  let prefix: WmiRecord | null = null;
  for (const e of data.wmi) {
    if (e[0].length === 3 && key.length === 3 && e[0] === key) return e;
    if (e[0].length === 2 && e[0] === key.slice(0, 2)) prefix = e;
  }
  return prefix;
}

function vinCountry(data: CatalogData, vin: string): string {
  if (vin.length < 2) return "";
  const first = vin[0]!.toUpperCase();
  const second = rangeIndex(vin[1]!.toUpperCase());
  if (second < 0) return "";
  for (const r of data.countries) {
    if (r[0] !== first) continue;
    if (second >= rangeIndex(r[1]) && second <= rangeIndex(r[2])) return L(r[3], r[4]);
  }
  return L("nicht zugeordnet", "not assigned");
}

function vinRegion(vin: string): string {
  if (!vin) return "";
  const c = vin[0]!.toUpperCase();
  if (c >= "A" && c <= "H") return L("Afrika", "Africa");
  if (c >= "J" && c <= "R") return L("Asien", "Asia");
  if (c >= "S" && c <= "Z") return L("Europa", "Europe");
  if (c >= "1" && c <= "5") return L("Nordamerika", "North America");
  if (c === "6") return L("Ozeanien", "Oceania");
  if (c === "7") {
    // 7A to 7E is New Zealand, the rest of the range went to the USA.
    const s = vin.length > 1 ? vin[1]!.toUpperCase() : "";
    return s >= "A" && s <= "E" ? L("Ozeanien", "Oceania") : L("Nordamerika", "North America");
  }
  if (c === "8" || c === "9") return L("Südamerika", "South America");
  return L("unbekannt", "unknown");
}

const YEAR_CODES = "ABCDEFGHJKLMNPRSTVWXY123456789";
function vinBaseYear(code: string): number {
  const p = code ? YEAR_CODES.indexOf(code) : -1;
  return p >= 0 ? 1980 + p : 0;
}

// Values of the characters for the check digit (49 CFR 565.15).
function transliterate(c: string): number {
  if (c >= "0" && c <= "9") return c.charCodeAt(0) - 48;
  const table: Record<string, number> = {
    A: 1, J: 1, B: 2, K: 2, S: 2, C: 3, L: 3, T: 3, D: 4, M: 4, U: 4, E: 5, N: 5, V: 5,
    F: 6, W: 6, G: 7, P: 7, X: 7, H: 8, Y: 8, R: 9, Z: 9,
  };
  return table[c] ?? -1;
}

const WEIGHTS = [8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2];

function vinCheckDigit(vin: string): string {
  if (vin.length !== 17) return "";
  let sum = 0;
  for (let i = 0; i < 17; i++) {
    const v = transliterate(vin[i]!.toUpperCase());
    if (v < 0) return "";
    sum += v * WEIGHTS[i]!;
  }
  const rest = sum % 11;
  return rest === 10 ? "X" : String(rest);
}

const isDigit = (c: string | undefined) => !!c && c >= "0" && c <= "9";

function decodeVin(data: CatalogData, input: string, currentYearArg = 0): VinInfo {
  const info: VinInfo = {
    valid: false, vin: "", wmi: "", manufacturer: "", brand: "", brandCandidates: [], country: "", region: "",
    northAmerica: false, modelYears: [], modelYear: 0, modelYearCertain: false, yearCode: "", checkDigitApplies: false,
    checkDigitOk: false, checkDigit: "", expectedCheckDigit: "", plant: "", serial: "", modelCode: "", problems: [], notes: [],
  };
  const currentYear = currentYearArg > 0 ? currentYearArg : new Date().getFullYear();

  // Normalise: upper case, drop the separators people type or copy along.
  let badChars = false;
  let ioq = false;
  for (const raw of input) {
    const c = raw.toUpperCase();
    if (" -.\t*\r\n".includes(c)) continue;
    if (c === "I" || c === "O" || c === "Q") ioq = true;
    else if (!/^[A-Z0-9]$/.test(c)) badChars = true;
    info.vin += c;
  }
  if (!info.vin) {
    info.problems.push(L("Keine Fahrgestellnummer angegeben.", "No VIN given."));
    return info;
  }
  const vin = info.vin;
  if (vin.length !== 17) {
    info.problems.push(
      vin.length < 17
        ? L("Zu kurz: eine Fahrgestellnummer hat 17 Zeichen. Autos vor 1981 hatten oft kürzere Nummern, die sich nicht nach dieser Norm deuten lassen.",
          "Too short: a VIN has 17 characters. Cars built before 1981 often had shorter numbers that cannot be decoded after this standard.")
        : L("Zu lang: eine Fahrgestellnummer hat 17 Zeichen.", "Too long: a VIN has 17 characters."),
    );
  }
  if (ioq) {
    info.problems.push(L("Enthält I, O oder Q. Diese Buchstaben kommen in Fahrgestellnummern nicht vor, gemeint ist wohl 1 oder 0.",
      "Contains I, O or Q. These letters never appear in a VIN, probably 1 or 0 is meant."));
  }
  if (badChars) {
    info.problems.push(L("Enthält Zeichen, die keine Buchstaben oder Ziffern sind.", "Contains characters that are neither letters nor digits."));
  }
  info.valid = vin.length === 17 && !ioq && !badChars;

  // Manufacturer and origin work with the first three characters alone.
  if (vin.length >= 3) info.wmi = vin.slice(0, 3);
  info.country = vinCountry(data, vin);
  info.region = vinRegion(vin);
  const first = vin[0]!;
  const second = vin.length > 1 ? vin[1]! : "";
  const chinese = first === "L" || first === "H";
  info.northAmerica = (first >= "1" && first <= "5") || (first === "7" && rangeIndex(second) >= rangeIndex("F"));

  const wmi = vin.length >= 2 ? findWmi(data, vin) : null;
  if (wmi) {
    info.manufacturer = wmi[1];
    info.brandCandidates = wmi[2].split("|").filter(Boolean);
    if (info.brandCandidates.length === 1) info.brand = info.brandCandidates[0]!;
    if (wmi[5]) info.country = L(wmi[5][0], wmi[5][1]);
  } else if (vin.length >= 3) {
    info.notes.push(L("Der Hersteller ist in der Tabelle nicht verzeichnet.", "The manufacturer is not in the table."));
  }

  if (vin.length !== 17) return info;

  // Check digit. Mandatory in North America and China, optional elsewhere.
  info.checkDigit = vin[8]!;
  info.expectedCheckDigit = vinCheckDigit(vin);
  info.checkDigitOk = info.expectedCheckDigit !== "" && info.checkDigit === info.expectedCheckDigit;
  info.checkDigitApplies = info.northAmerica || chinese;
  if (info.checkDigitApplies && !info.checkDigitOk && info.valid) {
    info.problems.push(L("Die Prüfziffer an Stelle 9 stimmt nicht. Wahrscheinlich ist ein Zeichen falsch abgeschrieben.",
      "The check digit at position 9 does not match. Probably a character was copied wrongly."));
  } else if (!info.checkDigitApplies && info.valid) {
    info.notes.push(
      info.checkDigitOk
        ? L("Die Prüfziffer an Stelle 9 stimmt. Das spricht für ein Fahrzeug, das für Nordamerika gebaut wurde.",
          "The check digit at position 9 matches. That suggests a vehicle built for North America.")
        : L("Die Prüfziffer passt nicht, ist bei Fahrzeugen für Europa aber nicht vorgeschrieben. Das ist kein Fehler.",
          "The check digit does not match, but it is not required for vehicles built for Europe. That is not an error."),
    );
  }
  // A matching check digit outside North America almost always means a car
  // built to US rules, including the position 7 rule below.
  const usRules = info.northAmerica || (!chinese && info.checkDigitOk);

  info.plant = vin[10]!;
  info.serial = vin.slice(11);

  // Maker specific model codes.
  if (wmi) {
    switch (wmi[4]) {
      case CodeRule.Vag78:
      case CodeRule.Porsche78:
        if (vin.slice(6, 8) !== "ZZ") info.modelCode = vin.slice(6, 8);
        break;
      case CodeRule.Tesla4:
        info.modelCode = vin.slice(3, 4);
        break;
      case CodeRule.Mercedes456:
        // European layout: three digits of the model series. North American
        // VINs use letter codes there, which we leave alone.
        if (isDigit(vin[3]) && isDigit(vin[4]) && isDigit(vin[5])) info.modelCode = vin.slice(3, 6);
        break;
    }
  }

  // Model year from position 10.
  info.yearCode = vin[9]!;
  let rule: number = wmi ? wmi[3] : info.northAmerica || chinese ? YearRule.Standard : YearRule.Unknown;
  if (usRules || chinese) rule = YearRule.Standard;
  if (wmi && wmi[4] === CodeRule.Mercedes456 && info.modelCode && !usRules) {
    // Mercedes-Benz writes no model year into European VINs.
    rule = YearRule.None;
  }

  const base = vinBaseYear(info.yearCode);
  if (rule === YearRule.None) {
    info.notes.push(L("Dieser Hersteller schreibt bei Fahrzeugen für Europa kein Baujahr in die Fahrgestellnummer. Es steht im Fahrzeugschein.",
      "This manufacturer writes no model year into VINs for Europe. It is in the registration papers."));
    return info;
  }
  if (base === 0) {
    info.notes.push(L("Stelle 10 enthält kein Baujahr. In Europa ist das erlaubt, das Baujahr steht dann nur im Fahrzeugschein.",
      "Position 10 holds no model year. That is allowed in Europe, the year is then only in the registration papers."));
    return info;
  }

  const maxYear = currentYear + 1;
  if (usRules) {
    // US rule since model year 2010: a letter at position 7 marks the second
    // cycle (2010-2039), a digit the first one (1980-2009).
    const secondCycle = !isDigit(vin[6]);
    info.modelYears.push(secondCycle ? base + 30 : base);
    info.modelYearCertain = true;
  } else {
    if (base + 30 <= maxYear) info.modelYears.push(base + 30);
    info.modelYears.push(base);
    info.modelYearCertain = rule === YearRule.Standard && info.modelYears.length === 1;
    if (rule === YearRule.Unknown) {
      info.notes.push(L("Ob dieser Hersteller an Stelle 10 das Baujahr schreibt, ist nicht sicher. Das Baujahr ist daher nur eine Vermutung.",
        "It is not certain that this manufacturer puts the model year at position 10. The year is therefore only a guess."));
    }
  }
  info.modelYear = info.modelYears[0]!;
  return info;
}

function hasCode(g: GenRecord, code: string): boolean {
  return !!code && ` ${g[15]} `.includes(` ${code} `);
}

// Best catalog guess for a decoded VIN (catalog::matchVin).
function matchVin(data: CatalogData, vin: VinInfo): CatalogMatch {
  const m: CatalogMatch = { generation: null, brand: "", year: vin.modelYear, confidence: 0, alternatives: [], reason: "" };
  const brandList = vin.brandCandidates.length > 0 ? vin.brandCandidates : vin.brand ? [vin.brand] : [];
  if (brandList.length === 0) {
    m.reason = L("Der Hersteller ist unbekannt, daher gibt es keinen Treffer im Katalog.",
      "The manufacturer is unknown, so there is no match in the catalog.");
    return m;
  }
  if (brandList.length === 1) m.brand = brandList[0]!;
  const t = data.generations;
  const brandFits = (g: GenRecord) => brandList.some((b) => same(g[0], b));
  const out = (ids: number[]) => ids.map((i) => info(data, i));

  // 1. The model code from the VIN, checked against the year candidates.
  if (vin.modelCode) {
    const byCode: number[] = [];
    t.forEach((g, i) => {
      if (brandFits(g) && hasCode(g, vin.modelCode)) byCode.push(i);
    });
    if (byCode.length > 0) {
      // Try the most likely year first, then the alternatives.
      for (let yi = 0; yi < vin.modelYears.length; yi++) {
        const year = vin.modelYears[yi]!;
        const fit = byCode.filter((i) => covers(t[i]!, year));
        if (fit.length === 0) continue;
        // Several models can share a code (Golf IV and Bora are both 1J);
        // they keep the catalog order. Within one model the newer generation
        // wins in the change-over years.
        const ordered: number[] = [];
        for (const i of fit) {
          const g = t[i]!;
          const at = ordered.findIndex((o) => t[o]![1] === g[1] && t[o]![0] === g[0]);
          if (at < 0) ordered.push(i);
          else if (g[3] > t[ordered[at]!]![3]) ordered.splice(at, 0, i);
          else ordered.splice(at + 1, 0, i);
        }
        const best = ordered[0]!;
        m.generation = info(data, best);
        m.alternatives = out(ordered.slice(1));
        m.year = year;
        m.brand = t[best]![0];
        m.confidence = yi === 0 && vin.modelYearCertain ? 95 : 85;
        if (ordered.length > 1) m.confidence -= 15;
        m.reason =
          yi === 0
            ? L("Modellcode und Baujahr aus der Fahrgestellnummer passen zu dieser Generation.",
              "Model code and model year from the VIN match this generation.")
            : L("Der Modellcode passt; das Baujahr ergibt sich erst über die Bauzeit dieser Generation.",
              "The model code matches; the model year follows from the build period of this generation.");
        return m;
      }
      // No usable year: the code alone.
      const best = byCode[0]!;
      m.generation = info(data, best);
      m.alternatives = out(byCode.slice(1));
      m.brand = t[best]![0];
      m.confidence = byCode.length === 1 ? 75 : 55;
      if (vin.modelYears.length === 0) {
        m.year = 0;
        m.reason = L("Der Modellcode passt, ein Baujahr steht nicht in der Fahrgestellnummer.",
          "The model code matches, the VIN carries no model year.");
      } else {
        m.confidence -= 20;
        m.reason = L("Der Modellcode passt, aber das Baujahr liegt außerhalb der bekannten Bauzeit.",
          "The model code matches, but the model year lies outside the known build period.");
      }
      return m;
    }
  }

  // 2. Brand and year only: everything of that brand built in that year.
  const alternatives: number[] = [];
  for (const year of vin.modelYears) {
    t.forEach((g, i) => {
      if (brandFits(g) && covers(g, year)) alternatives.push(i);
    });
    if (alternatives.length > 0) {
      m.year = year;
      break;
    }
  }
  if (alternatives.length === 1) {
    const best = alternatives[0]!;
    m.generation = info(data, best);
    m.brand = t[best]![0];
    m.confidence = vin.modelYearCertain ? 70 : 55;
    m.reason = L("Von diesem Hersteller gab es in diesem Baujahr nur dieses Modell.", "This manufacturer only built this model in that year.");
    return m;
  }
  m.alternatives = out(alternatives);
  m.confidence = m.brand ? 30 : 15;
  m.reason = L("Der Hersteller ist klar, das Modell steht nicht in der Fahrgestellnummer. Bitte das Modell aus der Liste wählen.",
    "The manufacturer is clear, the model is not in the VIN. Please pick the model from the list.");
  return m;
}

// --- Manufacturer fault code tables ------------------------------------------------------

const GENERIC_LABEL = () => L("SAE/ISO (alle Marken)", "SAE/ISO (all brands)");

export const knowledgeHandlers: PartialHandlers = {
  catalog_size: async () => (await loadCatalog()).generations.length,
  catalog_brands: async () => brands(await loadCatalog()),
  catalog_models: async ({ brand }) => models(await loadCatalog(), brand),
  catalog_generations: async ({ brand, model }) => {
    const data = await loadCatalog();
    return generations(data, brand, model).map((i) => info(data, i));
  },
  catalog_search: async ({ query, limit }) => {
    const data = await loadCatalog();
    return search(data, query, limit).map((i) => info(data, i));
  },
  catalog_diagnosis: async ({ generationId, year }): Promise<VehicleDiagnosisInfo> => {
    const data = await loadCatalog();
    const g = data.generations[generationId];
    if (!g) throw new Error(`no generation ${generationId}`);
    const y = year <= 0 ? g[3] : year;
    const support = supportFor(g, y);
    const connector = connectorFor(g, y);
    return {
      generation: info(data, generationId),
      year: y,
      support,
      supportName: supportName(support),
      connector,
      connectorName: connectorName(connector),
      locationText: locationText(g, y),
      diagnosisText: diagnosisText(data, g, y),
      kklPossible: kklPossible(data, g, y),
      protocolOrder: g[11],
    };
  },
  decode_vin: async ({ vin }) => decodeVin(await loadCatalog(), vin),
  match_vin: async ({ vin }) => {
    const data = await loadCatalog();
    return matchVin(data, decodeVin(data, vin));
  },

  // The generic tables of mock/dtc.ts plus the brand tables, the way
  // dtcLexicon() lists them: generic codes first, then family by family.
  brand_families: async (): Promise<BrandFamily[]> => {
    const families = await loadFamilies();
    return [
      { key: "", label: GENERIC_LABEL(), count: genericCount() },
      ...families.map((f) => ({ key: f[0], label: L(f[1], f[2]), count: f[4].length })),
    ];
  },
  dtc_lexicon: async (): Promise<DtcLexiconEntry[]> => {
    const families = await loadFamilies();
    const out = lexicon();
    for (const f of families) {
      const label = L(f[1], f[2]);
      for (const r of f[4]) {
        out.push({ code: r[0], family: f[0], familyLabel: label, text: L(r[1], r[2]), severity: r[3] as Severity, system: r[4] });
      }
    }
    const systems = new Map<number, string>();
    const severities = new Map<number, string>();
    for (const e of out) {
      if (!systems.has(e.system)) systems.set(e.system, systemName(e.system));
      if (!severities.has(e.severity)) severities.set(e.severity, severityLabel(e.severity));
      e.systemName = systems.get(e.system)!;
      e.severityLabel = severities.get(e.severity)!;
    }
    return out;
  },
  // The settings page opens the folder of the settings file; the browser
  // preview keeps them in local storage, so there is nothing to show.
  open_config_location: () => undefined,
};
