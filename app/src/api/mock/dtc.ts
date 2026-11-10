// Fault code knowledge for the demo backend: the generic SAE tables, the
// plain-language help and the zone ranges of the C++ data (data/dtc.json is
// extracted from src/data/DtcGeneric*.gen.cpp, DtcDetails.gen.cpp and
// DtcLocation.cpp). Manufacturer tables are left out; the real backend has
// them.
import type { CostClass, DtcInfo, DtcLexiconEntry, Severity, ZoneInfo } from "../types";
import { CarZone } from "../types";
import { L, Llist } from "./core";
import raw from "./data/dtc.json";

type CodeRecord = [string, string, string, number, number, number, boolean];
type DetailRecord = [string, string, string[], string[], string[], string[], number];

const data = raw as unknown as {
  codes: CodeRecord[];
  details: DetailRecord[];
  systemNames: [string, string][];
  zoneRanges: [number, number, number, string, string][];
};

const byCode = new Map<string, CodeRecord>(data.codes.map((c) => [c[0], c]));

export function severityLabel(s: Severity): string {
  switch (s) {
    case 0: return L("Hinweis", "Note");
    case 1: return L("Gering", "Low");
    case 2: return L("Mittel", "Medium");
    case 3: return L("Hoch", "High");
    case 4: return L("Kritisch", "Critical");
  }
}

function severityAdvice(s: Severity): string {
  switch (s) {
    case 0:
      return L("Kein Fehler im eigentlichen Sinn, Weiterfahren ist in Ordnung.",
        "Not a fault as such, it is fine to keep driving.");
    case 1:
      return L("Weiterfahren ist in Ordnung. Bei Gelegenheit prüfen lassen.",
        "It is fine to keep driving. Have it checked when convenient.");
    case 2:
      return L("Weiterfahren ist möglich, aber bald in die Werkstatt: Verbrauch, Abgaswerte oder Komfort können leiden.",
        "You can keep driving, but go to a workshop soon: consumption, emissions or comfort may suffer.");
    case 3:
      return L("Nur noch behutsam und auf kurzen Strecken fahren und in den nächsten Tagen prüfen lassen, sonst können Folgeschäden entstehen.",
        "Drive gently and only short distances, and have it checked within the next few days, otherwise further damage can follow.");
    case 4:
      return L("Nicht weiterfahren. Anhalten, sobald es sicher geht, Motor abstellen und Hilfe holen.",
        "Do not drive on. Stop as soon as it is safe, switch off the engine and get help.");
  }
}

function costLabel(c: CostClass): string {
  switch (c) {
    case 1: return L("günstig", "cheap");
    case 2: return L("mittel", "moderate");
    case 3: return L("teuer", "expensive");
    default: return "";
  }
}

function costHint(c: CostClass): string {
  switch (c) {
    case 1: return L("Reparatur in der Werkstatt grob unter 200 Euro", "Workshop repair roughly below 200 euros");
    case 2: return L("Reparatur in der Werkstatt grob 200 bis 800 Euro", "Workshop repair roughly 200 to 800 euros");
    case 3: return L("Reparatur in der Werkstatt grob über 800 Euro", "Workshop repair roughly above 800 euros");
    default: return "";
  }
}

export function systemName(system: number): string {
  const n = data.systemNames[system] ?? data.systemNames[0]!;
  return L(n[0], n[1]);
}

export function describeDtc(code: string): string {
  const r = byCode.get(code.toUpperCase());
  if (r) return L(r[1], r[2]);
  return L("Unbekannter Fehlercode", "Unknown trouble code");
}

export function lookupDtc(codeText: string): DtcInfo {
  const code = codeText.trim().toUpperCase();
  const r = byCode.get(code);
  const severity = (r ? r[3] : 2) as Severity;
  const detail = r && r[5] >= 0 ? data.details[r[5]] : undefined;
  const cost = (detail ? detail[6] : 0) as CostClass;
  const system = r ? r[4] : 0;
  const manufacturer = /^[PCBU][13]/.test(code);
  return {
    code,
    text: r ? L(r[1], r[2]) : L("Unbekannter Fehlercode", "Unknown trouble code"),
    severity,
    severityLabel: severityLabel(severity),
    system,
    systemName: systemName(system),
    advice: detail ? L(detail[0], detail[1]) : severityAdvice(severity),
    causes: detail ? Llist(detail[2], detail[3]) : [],
    checks: detail ? Llist(detail[4], detail[5]) : [],
    costClass: cost,
    costLabel: costLabel(cost),
    costHint: costHint(cost),
    manufacturerSpecific: manufacturer,
    known: !!r,
    hasDetails: !!detail,
    throttleRelated: r ? r[6] : false,
    family: "",
    familyLabel: "",
  };
}

const ZONE_NAMES: [string, string][] = [
  ["nicht zugeordnet", "not assigned"],
  ["Ansaugtrakt", "Intake system"],
  ["Drosselklappe", "Throttle body"],
  ["Einspritzung", "Fuel injection"],
  ["Zündanlage", "Ignition system"],
  ["Motor und Steuertrieb", "Engine and timing drive"],
  ["Kühlung", "Cooling system"],
  ["Turbolader", "Turbocharger"],
  ["Abgasanlage", "Exhaust system"],
  ["Kraftstoffanlage", "Fuel system"],
  ["Getriebe", "Transmission"],
  ["Steuergerät", "Control unit"],
  ["Bordnetz", "Electrical system"],
  ["Datenbus", "Data bus"],
  ["Räder und Bremsen", "Wheels and brakes"],
  ["Innenraum", "Cabin"],
];

export function zoneName(zone: CarZone): string {
  const n = ZONE_NAMES[zone] ?? ZONE_NAMES[0]!;
  return L(n[0], n[1]);
}

export function locateDtc(codeText: string): ZoneInfo {
  const code = codeText.trim().toUpperCase();
  const letter = code[0] ?? "P";
  const number = Number.parseInt(code.slice(1), 16);
  let zone = CarZone.Unknown;
  let hint = L("keine Zuordnung hinterlegt", "no location on file");
  if (letter === "C") {
    zone = CarZone.Wheels;
    hint = L("Raddrehzahlfühler, ABS-Hydraulik und die Bremsen", "Wheel speed sensors, ABS hydraulics and the brakes");
  } else if (letter === "B") {
    zone = CarZone.Cabin;
    hint = L("Innenraum: Airbag, Gurtstraffer, Komfortelektronik", "Cabin: airbags, seat belt pretensioners, comfort electronics");
  } else if (letter === "U") {
    zone = CarZone.Network;
    hint = L("Datenbus zwischen den Steuergeräten, oft nur ein Steckerproblem", "Data bus between the control units, often just a connector problem");
  } else if (!Number.isNaN(number)) {
    for (const [lo, hi, z, de, en] of data.zoneRanges) {
      if (number >= lo && number <= hi) {
        zone = z as CarZone;
        hint = L(de, en);
        break;
      }
    }
  }
  return { zone, name: zoneName(zone), hint };
}

export function lexicon(): DtcLexiconEntry[] {
  const label = L("SAE/ISO (alle Marken)", "SAE/ISO (all brands)");
  return data.codes.map((r) => ({
    code: r[0],
    family: "",
    familyLabel: label,
    text: L(r[1], r[2]),
    severity: r[3] as Severity,
    system: r[4],
  }));
}

export function genericCount(): number {
  return data.codes.length;
}
