// Demo answer for the used car check: Worker::runUsedCarCheck with
// analysis::used_car of crates/caruspex-analysis ported to TypeScript, on the
// demo car. The texts are the ones of the Rust module, so the core catalog
// translates them the same way.
import { t } from "../../../i18n";
import type { Level, UsedCarItemRow, UsedCarMileageRow, UsedCarRow, UsedCarSection } from "../../types";
import { DEMO_VIN, simulate } from "../car";
import { L } from "../core";
import { lookupDtc } from "../dtc";
import { car, emit, link, wait, type PartialHandlers } from "../runtime";
import { faultRows, readLive, statusRow } from "../session";
import { demoMonitors, quietHealthCheck } from "./diagnose";

const W = "caruspex::Worker";

// analysis::used_car::limits
const RECENT_WARM_UPS = 5;
const RECENT_DISTANCE_KM = 50;
const RECENT_MINUTES = 180;
const MILEAGE_TOLERANCE_KM = 100;
const MILEAGE_TOLERANCE_SHARE = 0.01;
const LOW_KM_PER_YEAR = 3000;
const HIGH_KM_PER_YEAR = 40000;
const LIVE_SECONDS = 10;

// A pattern with %1..%9 in the active language, filled in one pass.
function m(de: string, en: string, ...args: (string | number)[]): string {
  return L(de, en).replace(/%([1-9])/g, (whole, d: string) => {
    const value = args[Number(d) - 1];
    if (value === undefined) return whole;
    return typeof value === "number" ? String(Math.round(value)) : value;
  });
}

type Kind = "stored" | "pending" | "permanent";
interface Fault {
  code: string;
  text: string;
  kind: Kind;
  level: Level;
}

const normalise = (vin: string) => vin.replace(/[\s\-.*]/g, "").toUpperCase();
const wellFormed = (vin: string) => /^[A-HJ-NPR-Z0-9]{17}$/.test(vin);

// dtc_level of the worker: pending codes count one step lower.
function dtcLevel(code: string, stored: boolean): Level {
  const severity = lookupDtc(code).severity;
  let level: Level = 1;
  if (severity === 4) level = 3;
  else if (severity === 3 || severity === 2) level = 2;
  if (!stored && level === 2) level = 1;
  return level;
}

interface Input {
  vin: string;
  papers: string;
  odometer: number | null;
  other: UsedCarMileageRow[];
  warmUps: number | null;
  distance: number | null;
  minutes: number | null;
  milDistance: number | null;
  mil: boolean;
  monitors: { name: string; complete: boolean }[];
  faults: Fault[];
  health: { score: number; grade: 0 | 1 | 2; summary: string } | null;
  year: number;
}

function evaluate(input: Input): UsedCarRow {
  const items: UsedCarItemRow[] = [];
  const notChecked: string[] = [];
  const add = (id: string, section: UsedCarSection, level: Level, title: string, explanation: string, ask = "") =>
    items.push({ id, section, level, title, explanation, ask });
  const skip = (title: string, reason: string) => notChecked.push(`${title}: ${reason}`);

  // Identity: the demo car is always the made-up Caruspex demo car.
  const vehicle = L("Caruspex Demo-Auto", "Caruspex demo car");
  const askPlate = m(
    "Stimmt die Fahrgestellnummer an der Frontscheibe und am Typschild mit den Papieren überein?",
    "Does the VIN on the windscreen and on the type plate match the papers?",
  );
  add(
    "vehicle",
    "identity",
    0,
    m("Erkannt: %1", "Identified: %1", vehicle),
    m(
      "Laut Fahrgestellnummer ein %1, Modelljahr %2 (die Baureihe wurde %3 gebaut). Passt das zu Anzeige und Papieren?",
      "According to the VIN a %1, model year %2 (the series was built %3). Does that match the advert and the papers?",
      vehicle,
      2021,
      m("seit %1", "since %1", 2021),
    ),
  );
  const papers = normalise(input.papers);
  if (!papers) {
    skip(
      m("Fahrgestellnummer der Papiere", "VIN from the papers"),
      m(
        "Nicht eingegeben. Für den Vergleich die Nummer aus der Zulassungsbescheinigung Teil I (Feld E) eintragen.",
        "Not entered. For the comparison, type in the number from the registration certificate.",
      ),
    );
  } else if (!wellFormed(papers)) {
    add(
      "papers_vin_invalid",
      "identity",
      1,
      m("Eingegebene Nummer bitte prüfen", "Please check the number entered"),
      m(
        "\"%1\" ist keine gültige Fahrgestellnummer (17 Zeichen, ohne I, O und Q). Vermutlich ein Tippfehler; bitte noch einmal von den Papieren abschreiben.",
        "\"%1\" is not a valid VIN (17 characters, without I, O and Q). Probably a typing error; please copy it from the papers again.",
        papers,
      ),
    );
  } else if (papers === input.vin) {
    add(
      "vin_matches_papers",
      "identity",
      0,
      m("Fahrgestellnummer passt zu den Papieren", "VIN matches the papers"),
      m("Im Motorsteuergerät und in den Papieren steht dieselbe Nummer: %1.", "The engine control unit and the papers carry the same number: %1.", papers),
      askPlate,
    );
  } else {
    add(
      "vin_differs_from_papers",
      "identity",
      3,
      m("Fahrgestellnummer weicht von den Papieren ab", "VIN differs from the papers"),
      m(
        "Im Motorsteuergerät steht %1, in den Papieren %2. Entweder stammt das Steuergerät aus einem anderen Auto, oder das Auto ist nicht das aus den Papieren. Das vor dem Kauf klären, im Zweifel bei der Zulassungsstelle.",
        "The engine control unit says %1, the papers say %2. Either the control unit comes from another car, or the car is not the one in the papers. Clear this up before buying, if in doubt with the registration office.",
        input.vin,
        papers,
      ),
      m("Warum steht im Motorsteuergerät eine andere Fahrgestellnummer als in den Papieren?", "Why does the engine control unit carry a different VIN than the papers?"),
    );
  }

  // Mileage
  const readings: UsedCarMileageRow[] = [];
  if (input.odometer !== null) readings.push({ source: L("Motorsteuergerät (OBD)", "Engine control unit (OBD)"), km: input.odometer });
  readings.push(...input.other);
  const askRecords = m(
    "Gibt es Serviceheft, Rechnungen und Prüfberichte, die den Kilometerstand belegen?",
    "Are there a service book, invoices and inspection reports that back up the mileage?",
  );
  const tolerance = (km: number) => Math.max(km * MILEAGE_TOLERANCE_SHARE, MILEAGE_TOLERANCE_KM);
  if (readings.length > 0) {
    const highest = Math.max(...readings.map((r) => r.km));
    const lowest = Math.min(...readings.map((r) => r.km));
    const list = readings.map((r) => m("%1: %2 km", "%1: %2 km", r.source, r.km)).join(", ");
    if (readings.length === 1) {
      add(
        "mileage_single",
        "mileage",
        1,
        m("Kilometerstand: %1 km", "Mileage: %1 km", highest),
        m(
          "Nur eine Quelle: %1. Für dieses Modell sind keine weiteren Steuergeräte mit belegter Kennung bekannt, ein Vergleich ist also nicht möglich. Der Wert muss zum Tacho passen.",
          "Only one source: %1. No further control units with a documented identifier are known for this model, so no comparison is possible. The value must match the dashboard.",
          list,
        ),
        askRecords,
      );
    } else if (highest - lowest > tolerance(highest)) {
      add(
        "mileage_mismatch",
        "mileage",
        3,
        m("Kilometerstände passen nicht zusammen", "Mileage readings do not match"),
        m(
          "Die Steuergeräte speichern verschiedene Werte: %1. Der Unterschied beträgt %2 km. Ein wenig ist normal, weil nicht jedes Gerät gleich oft speichert. So viel deutet aber darauf hin, dass der Tacho verstellt oder ein Steuergerät aus einem anderen Auto eingebaut wurde.",
          "The control units store different values: %1. The difference is %2 km. A little is normal, because not every unit stores equally often. This much points to an altered odometer or a control unit from another car.",
          list,
          highest - lowest,
        ),
        m(
          "Warum zeigen die Steuergeräte verschiedene Kilometerstände? Wurde der Tacho oder ein Steuergerät getauscht, und gibt es dafür einen Beleg?",
          "Why do the control units show different mileages? Was the instrument cluster or a control unit replaced, and is there proof of it?",
        ),
      );
    } else {
      add(
        "mileage_consistent",
        "mileage",
        0,
        m("Kilometerstände passen zusammen", "Mileage readings agree"),
        m(
          "Alle Quellen stimmen überein: %1. Kleine Unterschiede sind normal, weil nicht jedes Gerät gleich oft speichert.",
          "All sources agree: %1. Small differences are normal, because not every unit stores equally often.",
          list,
        ),
        askRecords,
      );
    }
    const age = input.year - 2021 + 0.5;
    const perYear = Math.round(highest / age / 100) * 100;
    if (age >= 3 && highest / age < LOW_KM_PER_YEAR) {
      add(
        "mileage_low_for_age",
        "mileage",
        1,
        m("Auffallend wenig gefahren", "Remarkably low mileage"),
        m(
          "Im Schnitt etwa %1 km pro Jahr. Das gibt es, etwa bei Zweitwagen, ist aber auch das, was ein zurückgedrehter Tacho zeigt.",
          "On average about %1 km per year. That happens, for a second car for instance, but it is also what a turned back odometer shows.",
          perYear,
        ),
        askRecords,
      );
    } else if (highest / age > HIGH_KM_PER_YEAR) {
      add(
        "mileage_high_for_age",
        "mileage",
        1,
        m("Viel gefahren", "High mileage"),
        m(
          "Im Schnitt etwa %1 km pro Jahr, meist Langstrecke. Das schont den Motor, aber Verschleißteile wie Kupplung, Bremsen und Fahrwerk sind entsprechend beansprucht.",
          "On average about %1 km per year, mostly long distances. That is easy on the engine, but wear parts such as clutch, brakes and suspension have worked accordingly.",
          perYear,
        ),
      );
    } else {
      add(
        "mileage_fits_age",
        "mileage",
        0,
        m("Kilometerstand passt zum Alter", "Mileage fits the age"),
        m("Im Schnitt etwa %1 km pro Jahr, ein üblicher Wert.", "On average about %1 km per year, a usual figure.", perYear),
      );
    }
  }

  // Clearing
  const facts: string[] = [];
  if (input.distance !== null) facts.push(m("%1 km", "%1 km", input.distance));
  if (input.warmUps !== null) facts.push(m("%1 Warmläufe", "%1 warm-ups", input.warmUps));
  if (input.minutes !== null) facts.push(m("%1 Stunden Betrieb", "%1 hours of operation", input.minutes / 60));
  const permanent = input.faults.filter((f) => f.kind === "permanent");
  const faultList = (list: Fault[]) => list.map((f) => (f.text ? m("%1 (%2)", "%1 (%2)", f.code, f.text) : f.code)).join(", ");
  const codes = (list: Fault[]) => list.map((f) => f.code).join(", ");
  const recent =
    (input.warmUps !== null && input.warmUps < RECENT_WARM_UPS) ||
    (input.distance !== null && input.distance < RECENT_DISTANCE_KM) ||
    (input.minutes !== null && input.minutes < RECENT_MINUTES);
  if (recent) {
    add(
      "cleared_recently",
      "clearing",
      2,
      m("Fehlerspeicher vor Kurzem gelöscht", "Fault memory cleared recently"),
      m(
        "Seit dem letzten Löschen: %1. Wer den Fehlerspeicher löscht, schaltet auch die Warnlampe aus und setzt die Selbsttests des Autos zurück. Fehler, die erst nach einigen Fahrten wiederkommen, sind bei der Besichtigung dann noch nicht zu sehen. Kurz vor einem Verkauf kann das heißen, dass ein Fehler versteckt werden soll. Es gibt auch harmlose Gründe, etwa eine Reparatur, eine Inspektion oder eine abgeklemmte Batterie.",
        "Since the last clearing: %1. Clearing the fault memory also switches off the warning lamp and resets the car's self-tests. Faults that only come back after a few drives are then not visible at the viewing. Shortly before a sale this can mean that a fault is being hidden. There are harmless reasons too, such as a repair, a service or a disconnected battery.",
        facts.join(", "),
      ),
      m("Wann und warum wurde der Fehlerspeicher zuletzt gelöscht? Gibt es dazu eine Rechnung?", "When and why was the fault memory last cleared? Is there an invoice for it?"),
    );
    if (permanent.length > 0) {
      add(
        "cleared_but_permanent",
        "clearing",
        3,
        m("Gelöscht, aber nicht repariert", "Cleared, but not repaired"),
        m(
          "Trotz des Löschens meldet das Auto dauerhafte Fehler: %1. Diese Codes lassen sich nicht löschen; sie verschwinden erst, wenn das Auto selbst festgestellt hat, dass der Fehler behoben ist. Der Fehler war also vor dem Löschen da und ist es vermutlich noch.",
          "Despite the clearing the car reports permanent faults: %1. These codes cannot be cleared; they only go away once the car itself has seen that the fault is fixed. So the fault was there before the clearing and probably still is.",
          faultList(permanent),
        ),
        m("Warum wurde %1 gelöscht, ohne den Fehler zu reparieren?", "Why was %1 cleared without repairing the fault?", codes(permanent)),
      );
    }
  } else {
    add(
      "not_cleared_recently",
      "clearing",
      0,
      m("Nicht frisch gelöscht", "Not freshly cleared"),
      m(
        "Seit dem letzten Löschen: %1. Kein Hinweis darauf, dass der Fehlerspeicher kurz vor der Besichtigung gelöscht wurde.",
        "Since the last clearing: %1. No sign that the fault memory was cleared shortly before the viewing.",
        facts.join(", "),
      ),
    );
  }
  if (input.milDistance !== null && input.milDistance > 0) {
    add(
      "mil_distance",
      "clearing",
      1,
      m("Mit Warnlampe gefahren: %1 km", "Driven with the warning lamp on: %1 km", input.milDistance),
      m(
        "Seit dem letzten Löschen ist das Auto %1 km mit leuchtender Motorkontrollleuchte gefahren. Je länger ein Fehler ignoriert wird, desto eher entstehen Folgeschäden.",
        "Since the last clearing the car has been driven %1 km with the check engine light on. The longer a fault is ignored, the more likely it causes further damage.",
        input.milDistance,
      ),
      m("Warum wurde mit leuchtender Motorkontrollleuchte weitergefahren?", "Why was the car driven on with the check engine light on?"),
    );
  }

  // Readiness
  const open = input.monitors.filter((x) => !x.complete);
  if (open.length === 0) {
    add(
      "readiness_complete",
      "readiness",
      0,
      m("Alle Bereitschaftstests abgeschlossen", "All readiness monitors complete"),
      m(
        "Das Auto hat alle %1 Selbsttests seit dem letzten Löschen beendet. Das spricht dagegen, dass kurz vor der Besichtigung gelöscht wurde.",
        "The car has finished all %1 self-tests since the last clearing. That speaks against a clearing shortly before the viewing.",
        input.monitors.length,
      ),
    );
  } else {
    add(
      "readiness_open",
      "readiness",
      open.length === 1 ? 1 : 2,
      m("Offene Bereitschaftstests: %1 von %2", "Readiness monitors not complete: %1 of %2", open.length, input.monitors.length),
      m(
        "Noch nicht fertig: %1. Nach dem Löschen des Fehlerspeichers oder dem Abklemmen der Batterie braucht das Auto einige Fahrten, bis diese Selbsttests wieder durch sind. Einzelne offene Tests sind normal, viele passen zu einem frischen Löschen.",
        "Not complete yet: %1. After the fault memory was cleared or the battery disconnected, the car needs a few drives until these self-tests are done again. Single open tests are normal, many fit a fresh clearing.",
        open.map((x) => x.name).join(", "),
      ),
      open.length === 1 ? "" : m("Wurde in letzter Zeit die Batterie abgeklemmt oder der Fehlerspeicher gelöscht?", "Was the battery disconnected or the fault memory cleared recently?"),
    );
  }

  // Faults
  if (input.mil) {
    add(
      "mil_on",
      "faults",
      2,
      m("Motorkontrollleuchte an", "Check engine light on"),
      m(
        "Das Steuergerät meldet die Motorkontrollleuchte als eingeschaltet. Bei laufendem Motor muss sie aus sein; leuchtet sie, liegt ein bestätigter Fehler an der Abgasreinigung oder am Motor vor.",
        "The control unit reports the check engine light as on. With the engine running it must be off; if it is lit, there is a confirmed fault in the emission control or the engine.",
      ),
      m("Seit wann leuchtet die Motorkontrollleuchte, und was hat die Werkstatt dazu gesagt?", "Since when has the check engine light been on, and what did the workshop say about it?"),
    );
  }
  const of = (kind: Kind) => input.faults.filter((f) => f.kind === kind);
  const stored = of("stored");
  const pending = of("pending");
  if (input.faults.length === 0) {
    add(
      "no_faults",
      "faults",
      0,
      m("Kein Fehler gespeichert", "No faults stored"),
      m("Weder gespeicherte noch vorläufige noch dauerhafte Fehler im Motorsteuergerät.", "No stored, pending or permanent faults in the engine control unit."),
    );
  }
  if (stored.length > 0) {
    add(
      "faults_stored",
      "faults",
      Math.max(1, ...stored.map((f) => f.level)) as Level,
      m("Gespeicherte Fehler: %1", "Stored faults: %1", stored.length),
      m(
        "%1. Diese Fehler hat das Steuergerät bestätigt. Was sie bedeuten und was eine Reparatur ungefähr kostet, steht im Fehlerspeicher bei jedem Code.",
        "%1. The control unit has confirmed these faults. What they mean and roughly what a repair costs is shown with each code in the fault memory.",
        faultList(stored),
      ),
      m("Was ist die Ursache für %1, und was würde die Reparatur kosten?", "What is the cause of %1, and what would the repair cost?", codes(stored)),
    );
  }
  if (pending.length > 0) {
    add(
      "faults_pending",
      "faults",
      1,
      m("Vorläufige Fehler: %1", "Pending faults: %1", pending.length),
      m(
        "%1. Diese Fehler hat das Steuergerät einmal gesehen, aber noch nicht bestätigt. Sie können verschwinden oder zu gespeicherten Fehlern werden.",
        "%1. The control unit has seen these faults once but not confirmed them yet. They can disappear or become stored faults.",
        faultList(pending),
      ),
    );
  }
  if (permanent.length > 0) {
    add(
      "faults_permanent",
      "faults",
      2,
      m("Dauerhafte Fehler: %1", "Permanent faults: %1", permanent.length),
      m(
        "%1. Dauerhafte Fehler lassen sich nicht löschen. Sie verschwinden erst, wenn das Auto selbst festgestellt hat, dass der Fehler behoben ist.",
        "%1. Permanent faults cannot be cleared. They only go away once the car itself has seen that the fault is fixed.",
        faultList(permanent),
      ),
      m("Wurde der Fehler hinter %1 repariert, und gibt es dafür eine Rechnung?", "Was the fault behind %1 repaired, and is there an invoice for it?", codes(permanent)),
    );
  }

  // Health
  if (input.health) {
    add(
      "health",
      "health",
      input.health.grade === 0 ? 0 : input.health.grade === 1 ? 1 : 2,
      m("Gesundheitscheck: %1 von 100 Punkten", "Health check: %1 of 100 points", input.health.score),
      input.health.summary,
    );
  }

  const order: UsedCarSection[] = ["identity", "mileage", "clearing", "readiness", "faults", "health", "battery"];
  items.sort((a, b) => order.indexOf(a.section) - order.indexOf(b.section));
  const critical = items.filter((i) => i.level === 3).length;
  const warnings = items.filter((i) => i.level === 2).length;
  let light: 0 | 1 | 2 = 0;
  let headline = m("Unauffällig", "Nothing unusual");
  let summary = m(
    "Die Steuergeräte zeigen nichts, was gegen das Auto spricht. Eine Probefahrt und ein Blick in die Wartungsnachweise ersetzt das nicht.",
    "The control units show nothing that speaks against the car. That does not replace a test drive and a look at the service records.",
  );
  if (critical > 0) {
    light = 2;
    headline = m("Vorsicht: vor dem Kauf klären", "Careful: clear up before buying");
    summary = m(
      "Punkte, die gegen einen Kauf ohne weitere Prüfung sprechen: %1. Die Fragen unten zuerst mit dem Verkäufer klären und das Auto in einer Werkstatt ansehen lassen.",
      "Points that speak against buying without a closer look: %1. Clear up the questions below with the seller first and have the car looked at by a workshop.",
      critical,
    );
  } else if (warnings > 0) {
    light = 1;
    headline = m("Mit Fragen kaufen", "Buy with questions");
    summary = m(
      "Punkte, die Sie mit dem Verkäufer besprechen sollten: %1. Nichts davon schließt einen Kauf aus, aber es gehört in den Preis oder in den Kaufvertrag.",
      "Points to talk through with the seller: %1. None of them rules out a purchase, but they belong in the price or in the contract.",
      warnings,
    );
  }
  const questions = items
    .filter((i) => i.ask)
    .map((i, n) => ({ i, n }))
    .sort((a, b) => b.i.level - a.i.level || a.n - b.n)
    .map(({ i }) => i.ask);

  return {
    light,
    headline,
    summary,
    vin: input.vin,
    papersVin: papers,
    vehicle,
    modelYear: 2021,
    mileage: readings,
    distanceSinceClearKm: input.distance ?? -1,
    minutesSinceClear: input.minutes ?? -1,
    warmUps: input.warmUps ?? -1,
    readinessComplete: input.monitors.filter((x) => x.complete).length,
    readinessTotal: input.monitors.length,
    storedFaults: stored.length,
    pendingFaults: pending.length,
    permanentFaults: permanent.length,
    healthScore: input.health ? input.health.score : -1,
    electric: false,
    batteryPack: "",
    batterySoh: -1,
    batterySoc: -1,
    items,
    questions,
    notChecked,
  };
}

async function usedCarCheck(papersVin: string): Promise<void> {
  if (!link.connected) {
    emit("failed", { error: L("Für die Gebrauchtwagen-Prüfung erst verbinden.", "Connect first to check a used car.") });
    return;
  }
  if (link.liveTimer) {
    clearInterval(link.liveTimer);
    link.liveTimer = 0;
  }
  link.cancel = false;
  emit("busy", { active: true });
  const progress = (step: string, percent: number) => emit("used_car_progress", { step, percent });

  progress(L("Fahrgestellnummer", "Vehicle identification number"), 4);
  await wait(400);
  progress(t(W, "Bereitschaft und Fehlerlampe"), 10);
  await wait(300);
  const status = statusRow();
  emit("status", { row: status });
  progress(t(W, "Fehlerspeicher"), 16);
  await wait(400);
  const rows = faultRows();
  emit("faults", { rows });
  const pendingCodes = car.pendingCodes();
  const faults: Fault[] = rows.map((r) => ({
    code: r.code,
    text: r.text,
    kind: r.stored ? "stored" : pendingCodes.includes(r.code) ? "pending" : "permanent",
    level: dtcLevel(r.code, r.stored),
  }));

  progress(L("Kilometerstand und letztes Löschen", "Mileage and last clearing"), 22);
  await wait(400);
  const value = (pid: number): number | null => {
    const row = readLive(pid);
    if (!row) return null;
    emit("live_value", { row });
    return row.value;
  };
  const odometer = value(0xa6);
  const distance = value(0x31);
  const warmUps = value(0x30);
  const minutes = value(0x4e);
  const milDistance = value(0x21);

  progress(L("Kilometerstand in weiteren Steuergeräten", "Mileage in other control units"), 30);
  await wait(500);
  const km = simulate(car.now()).odometer;
  const other: UsedCarMileageRow[] = [
    { source: L("Kilometerstand (Kombiinstrument)", "Odometer (instrument cluster)"), km: Math.floor(km) },
    { source: L("Kilometerstand (ABS)", "Odometer (ABS)"), km: Math.floor(km / 10) * 10 },
  ];

  progress(t(W, "Bauteilprüfung (Modus 06)"), 42);
  await wait(400);
  const health = await quietHealthCheck(LIVE_SECONDS, (elapsed) => {
    const left = Math.max(0, LIVE_SECONDS - Math.trunc(elapsed));
    progress(t(W, "Messwerte sammeln, noch %1 s", left), Math.min(95, 48 + Math.trunc((47 * elapsed) / LIVE_SECONDS)));
  });

  const row = evaluate({
    vin: DEMO_VIN,
    papers: papersVin,
    odometer,
    other,
    warmUps,
    distance,
    minutes,
    milDistance,
    mil: status.mil,
    monitors: demoMonitors(),
    faults,
    health: health ? { score: health.score, grade: health.grade, summary: health.summary } : null,
    year: new Date().getFullYear(),
  });
  progress(t(W, "Fertig"), 100);
  emit("used_car_result", { row });
  emit("busy", { active: false });
}

export const usedCarHandlers: PartialHandlers = {
  run_used_car_check: ({ papersVin }) => {
    void usedCarCheck(papersVin);
  },
};
