// The crate caruspex-advice for the demo backend: "Was soll ich jetzt tun?"
// (next_steps.rs) and the checklist of the connection assistant
// (connect_help.rs). Same rules and the same texts as the Rust side, so the
// core catalogs translate both.
import type { AdviceCost, AdviceInput, AdviceItem, AdviceReport, ConnectCheck, ConnectHelp, Urgency } from "../types";
import { CarZone } from "../types";
import { L } from "./core";
import { locateDtc, lookupDtc } from "./dtc";

// --- Small helpers -------------------------------------------------------------------

function args(template: string, ...values: string[]): string {
  let out = template;
  for (let i = values.length - 1; i >= 0; i--) out = out.split(`%${i + 1}`).join(values[i]!);
  return out;
}

function number(value: number, decimals: number): string {
  const text = value.toFixed(decimals);
  return L(text.replace(".", ","), text);
}

// Health check limits (analysis::health_check::limits).
const COOLANT_HOT = 108;
const COOLANT_OVERHEAT = 115;
const CHARGING_NONE = 12.6;
const CHARGING_TOO_HIGH = 15.5;

// DtcSystem in the order of the enum, as far as the rules need it.
const SYSTEM = { unknown: 0, fuelAir: 1, airIntake: 2, ignition: 4, cooling: 8, lubrication: 9, exhaust: 10, charging: 14 };
// Severity: 0 info, 1 low, 2 medium, 3 high, 4 critical.
const CRITICAL = 4;
const STOP: Urgency = 0;
const SOON: Urgency = 1;
const WAIT: Urgency = 2;
const SOURCE_ORDER: AdviceItem["source"][] = ["observation", "fault", "finding", "live", "lamp", "readiness"];

interface Draft extends AdviceItem {
  severity: number;
  system: number;
  parts: string[];
}

export function costText(cost: AdviceCost): string {
  switch (cost) {
    case 1: return L("Kostet nichts", "Costs nothing");
    case 2: return L("Reparatur in der Werkstatt grob unter 200 Euro", "Workshop repair roughly below 200 euros");
    case 3: return L("Reparatur in der Werkstatt grob 200 bis 800 Euro", "Workshop repair roughly 200 to 800 euros");
    case 4: return L("Reparatur in der Werkstatt grob über 800 Euro", "Workshop repair roughly above 800 euros");
    default: return L("Kosten erst nach der Fehlersuche abschätzbar", "Cost can only be estimated once the cause is found");
  }
}

function flashingSentence(): string {
  return L(
    "Eine blinkende Lampe heißt starke Zündaussetzer: unverbrannter Kraftstoff kann den Katalysator in wenigen Minuten zerstören.",
    "A flashing light means severe misfiring: unburnt fuel can destroy the catalytic converter within minutes.",
  );
}

function isMisfireCode(code: string): boolean {
  if (code.length !== 5 || !code.startsWith("P03") || !/^\d{4}$/.test(code.slice(1))) return false;
  const n = Number(code.slice(1));
  return n >= 300 && n <= 312;
}

function isPending(f: AdviceInput["faults"][number]): boolean {
  return !f.stored && (f.kind === L("sporadisch", "intermittent") || f.kind === "sporadisch" || f.kind === "intermittent");
}

function faultUrgency(severity: number, confirmed: boolean, misfire: boolean, flashing: boolean): Urgency {
  if (misfire && flashing) return STOP;
  if (severity >= CRITICAL) return STOP;
  if (severity === 3) return SOON;
  if (severity === 2 && confirmed) return SOON;
  return WAIT;
}

function draft(urgency: Urgency, source: AdviceItem["source"], title: string, why: string, cost: AdviceCost): Draft {
  return { urgency, source, code: "", title, why, cost, costText: "", workshop: "", evidence: [], page: "", severity: 2, system: 0, parts: [] };
}

function faultItem(f: AdviceInput["faults"][number], input: AdviceInput): Draft {
  const info = lookupDtc(f.code);
  const pending = isPending(f);
  const confirmed = !pending;
  const misfire = isMisfireCode(info.code);
  const flashing = misfire && input.milFlashing;
  const urgency = faultUrgency(info.severity, confirmed, misfire, input.milFlashing);
  const title = info.known || !f.text ? info.text : f.text;
  const why = flashing
    ? flashingSentence()
    : pending && urgency === WAIT
      ? L(
          "Noch nicht bestätigt: das Steuergerät hat den Fehler einmal bemerkt und prüft bei den nächsten Fahrten weiter.",
          "Not confirmed yet: the control unit noticed the fault once and keeps checking on the next drives.",
        )
      : info.advice;
  const parts = [args(L("Fehlercode %1, %2: %3.", "Trouble code %1, %2: %3."), info.code, f.kind, title)];
  if (f.ecu) parts.push(args(L("Gemeldet von: %1.", "Reported by: %1."), f.ecu));
  if (flashing) parts.push(L("Die Motorkontrollleuchte hat geblinkt.", "The check engine light was flashing."));
  else if (confirmed && input.milOn === true) parts.push(L("Die Motorkontrollleuchte ist an.", "The check engine light is on."));
  const zone = locateDtc(info.code);
  if (zone.zone !== CarZone.Unknown) parts.push(args(L("Bereich: %1.", "Area: %1."), zone.name));
  if (info.causes.length > 0) parts.push(args(L("Häufige Ursachen: %1.", "Common causes: %1."), info.causes.slice(0, 2).join("; ")));
  const cost = (info.costClass === 0 ? 0 : info.costClass + 1) as AdviceCost;
  const out = draft(urgency, "fault", title, why, cost);
  out.code = info.code;
  out.page = "faults";
  out.severity = info.severity;
  out.system = info.system;
  out.parts = parts;
  return out;
}

function findingTopic(id: string): [number, AdviceCost, boolean] | null {
  switch (id) {
    case "faults":
    case "mil":
    case "readiness":
    case "cleared_recently":
    case "mil_distance":
      return null;
    case "misfire": return [SYSTEM.ignition, 3, true];
    case "overheating": return [SYSTEM.cooling, 0, true];
    case "oil_temp": return [SYSTEM.lubrication, 0, true];
    case "thermostat": return [SYSTEM.cooling, 3, false];
    case "sensor_coolant": return [SYSTEM.cooling, 2, false];
    case "sensor_intake": return [SYSTEM.airIntake, 2, false];
    case "charging": return [SYSTEM.charging, 3, false];
    case "battery_rest": return [SYSTEM.charging, 2, false];
    case "catalyst_temp": return [SYSTEM.exhaust, 0, false];
  }
  if (id.startsWith("fuel_trim")) return [SYSTEM.fuelAir, 0, false];
  if (id.startsWith("catalyst")) return [SYSTEM.exhaust, 4, false];
  if (id.startsWith("o2_")) return [SYSTEM.exhaust, 3, false];
  return [SYSTEM.unknown, 0, false];
}

function findingItem(f: AdviceInput["findings"][number]): Draft | null {
  const topic = findingTopic(f.id);
  if (!topic) return null;
  const [system, cost, stopWhenCritical] = topic;
  let urgency: Urgency;
  if (f.level === 0) return null;
  else if (f.level === 1) urgency = WAIT;
  else if (f.level === 2) urgency = SOON;
  else urgency = stopWhenCritical ? STOP : SOON;
  const out = draft(urgency, "finding", f.title, f.explanation, cost);
  out.page = "health";
  out.system = system;
  out.parts = [args(L("Der Gesundheitscheck meldet: %1.", "The health check reports: %1."), f.title)];
  return out;
}

function liveItems(input: AdviceInput): Draft[] {
  const value = (pid: number) => {
    for (let i = input.live.length - 1; i >= 0; i--) {
      const v = input.live[i]!;
      if (v.pid === pid && Number.isFinite(v.value)) return v.value;
    }
    return undefined;
  };
  const measured = (it: Draft, system: number, page: AdviceItem["page"]) => {
    it.system = system;
    it.page = page;
    it.parts = [args(L("Gemessen bei laufendem Motor: %1.", "Measured with the engine running: %1."), it.title)];
    return it;
  };
  const out: Draft[] = [];
  const coolant = value(0x05);
  if (coolant !== undefined && coolant >= COOLANT_HOT) {
    const stop = coolant >= COOLANT_OVERHEAT;
    const why = stop
      ? L(
          "Über 115 °C kann die Zylinderkopfdichtung Schaden nehmen; Motor abstellen und abkühlen lassen.",
          "Above 115 °C the head gasket can be damaged; switch off the engine and let it cool down.",
        )
      : L(
          "Die Kühlmitteltemperatur liegt über dem normalen Bereich; Kühlmittelstand und Lüfter bald prüfen lassen.",
          "The coolant temperature is above the normal range; have the coolant level and the fan checked soon.",
        );
    const title = args(L("Kühlmittel sehr heiß: %1 °C", "Coolant very hot: %1 °C"), number(coolant, 0));
    out.push(measured(draft(stop ? STOP : SOON, "live", title, why, 0), SYSTEM.cooling, "health"));
  }
  const rpm = value(0x0c);
  const volts = value(0x42);
  if (volts !== undefined && rpm !== undefined && rpm >= 500) {
    const shown = number(volts, 1);
    let pair: [string, string] | null = null;
    if (volts < CHARGING_NONE) {
      pair = [
        args(L("Lichtmaschine lädt nicht: %1 V", "Alternator not charging: %1 V"), shown),
        L(
          "Bei laufendem Motor sollten mindestens 13,2 V anliegen, sonst wird die Batterie leer und das Auto bleibt stehen.",
          "With the engine running there should be at least 13.2 V, otherwise the battery goes flat and the car stops.",
        ),
      ];
    } else if (volts > CHARGING_TOO_HIGH) {
      pair = [
        args(L("Ladespannung zu hoch: %1 V", "Charging voltage too high: %1 V"), shown),
        L(
          "Über 15,5 V leiden Batterie und Elektronik; meist ist der Regler der Lichtmaschine defekt.",
          "Above 15.5 V the battery and electronics suffer; usually the alternator's regulator is faulty.",
        ),
      ];
    }
    if (pair) out.push(measured(draft(SOON, "live", pair[0], pair[1], 3), SYSTEM.charging, "battery"));
  }
  return out;
}

function absorb(into: Draft, other: Draft): void {
  if (other.urgency < into.urgency) {
    into.urgency = other.urgency;
    into.why = other.why;
  }
  if (into.cost === 0) into.cost = other.cost;
  into.evidence.push(other.title, ...other.evidence);
}

export function whatToDo(input: AdviceInput): AdviceReport {
  const items: Draft[] = input.faults.filter((f) => f.code.trim()).map((f) => faultItem(f, input));

  const measured: Draft[] = [];
  const candidates = [...input.findings.map(findingItem).filter((d): d is Draft => d !== null), ...liveItems(input)];
  for (const it of candidates) {
    const i = it.system !== SYSTEM.unknown ? measured.findIndex((m) => m.system === it.system) : -1;
    if (i < 0) measured.push(it);
    else if (it.urgency < measured[i]!.urgency) {
      const weaker = measured[i]!;
      measured[i] = it;
      absorb(it, weaker);
    } else absorb(measured[i]!, it);
  }
  for (const m of measured) {
    let best = -1;
    if (m.system !== SYSTEM.unknown) {
      items.forEach((f, i) => {
        if (f.source === "fault" && f.system === m.system && (best < 0 || f.urgency < items[best]!.urgency)) best = i;
      });
    }
    if (best >= 0) absorb(items[best]!, m);
    else items.push(m);
  }

  const confirmedFault = input.faults.some((f) => !isPending(f));
  if (input.milOn === true && !confirmedFault) {
    const it = draft(
      SOON,
      "lamp",
      L("Motorkontrollleuchte an, aber kein Fehlercode", "Check engine light on, but no trouble code"),
      L(
        "Die Lampe meldet einen abgasrelevanten Fehler; liefert das Motorsteuergerät keinen Code, steht er oft in einem anderen Steuergerät.",
        "The light reports an emissions fault; if the engine control unit gives no code, it is often stored in another control unit.",
      ),
      0,
    );
    it.page = "faults";
    it.parts = [
      L(
        "Die Motorkontrollleuchte leuchtet, über OBD liefert das Motorsteuergerät aber keinen Fehlercode. Bitte alle Steuergeräte auslesen.",
        "The check engine light is on, but the engine control unit gives no trouble code over OBD. Please read all control units.",
      ),
    ];
    items.push(it);
  }

  const misfireListed = items.some((i) => i.source === "fault" && isMisfireCode(i.code));
  if (input.milFlashing && !misfireListed) {
    const it = draft(STOP, "observation", L("Motorkontrollleuchte blinkt", "Check engine light is flashing"), flashingSentence(), 0);
    it.page = "faults";
    it.severity = CRITICAL;
    it.system = SYSTEM.ignition;
    it.parts = [
      L(
        "Die Motorkontrollleuchte hat während der Fahrt geblinkt, Verdacht auf starke Zündaussetzer. Das Auto wurde abgestellt.",
        "The check engine light was flashing while driving, suspected severe misfiring. The car has been parked.",
      ),
    ];
    items.push(it);
  }
  if (input.redWarningLamp) {
    const it = draft(
      STOP,
      "observation",
      L("Rote Warnlampe leuchtet", "A red warning light is on"),
      L(
        "Rot heißt anhalten: meist geht es um Öldruck, Kühlmittel, Bremse oder Ladestrom, und Weiterfahren kann Motor oder Bremsen kosten.",
        "Red means stop: it is usually oil pressure, coolant, brakes or charging, and driving on can cost the engine or the brakes.",
      ),
      0,
    );
    it.severity = CRITICAL;
    it.parts = [
      L(
        "Eine rote Warnlampe leuchtet (Symbol fotografieren und zeigen). Das Auto wurde abgestellt.",
        "A red warning light is on (take a photo of the symbol and show it). The car has been parked.",
      ),
    ];
    items.push(it);
  }

  const open = input.readiness.filter((r) => !r.ready).map((r) => r.name);
  if (open.length > 0) {
    const names = open.join(", ");
    const it = draft(
      WAIT,
      "readiness",
      args(L("Bereitschaftstests offen: %1", "Readiness monitors incomplete: %1"), names),
      L(
        "Das Auto hat seine Selbsttests noch nicht beendet; nach dem Löschen oder einem Batteriewechsel ist das normal und erledigt sich durch ein paar Tage normales Fahren.",
        "The car has not finished its self-tests yet; after clearing codes or a battery change that is normal and sorts itself out after a few days of normal driving.",
      ),
      1,
    );
    it.page = "health";
    it.severity = 0;
    it.parts = [
      args(
        L(
          "Bereitschaftstests noch offen: %1. Vor der Abgasuntersuchung erst fertig fahren lassen.",
          "Readiness monitors still incomplete: %1. Let them complete before the emissions test.",
        ),
        names,
      ),
    ];
    items.push(it);
  }

  for (const it of items) {
    it.costText = costText(it.cost);
    const parts = [...it.parts];
    if (it.evidence.length > 0) parts.push(args(L("Messwerte dazu: %1.", "Measured values: %1."), it.evidence.join("; ")));
    it.workshop = parts.join(" ");
  }
  const sorted = items
    .map((it, index) => ({ it, index }))
    .sort(
      (a, b) =>
        a.it.urgency - b.it.urgency ||
        SOURCE_ORDER.indexOf(a.it.source) - SOURCE_ORDER.indexOf(b.it.source) ||
        b.it.severity - a.it.severity ||
        a.index - b.index,
    )
    .map(({ it }) => {
      const { severity: _s, system: _y, parts: _p, ...item } = it;
      return item as AdviceItem;
    });

  const nothingRead =
    !input.faultsRead &&
    input.faults.length === 0 &&
    input.milOn === null &&
    input.readiness.length === 0 &&
    input.findings.length === 0 &&
    input.live.length === 0 &&
    !input.milFlashing &&
    !input.redWarningLamp;
  const count = (u: Urgency) => sorted.filter((i) => i.urgency === u).length;
  let headline: string;
  let summary: string;
  if (nothingRead) {
    headline = L("Noch nichts gelesen", "Nothing read yet");
    summary = L("Verbinden und das Auto prüfen, dann steht hier, was zu tun ist.", "Connect and check the car, then this page tells you what to do.");
  } else if (count(STOP) > 0) {
    headline = L("Bitte anhalten", "Please stop");
    summary = L(
      "Mindestens ein Punkt verträgt keine Weiterfahrt. Anhalten, sobald es sicher geht, und Hilfe holen.",
      "At least one point does not allow driving on. Stop as soon as it is safe and get help.",
    );
  } else if (count(SOON) > 0) {
    headline = L("Bald in die Werkstatt", "Visit a workshop soon");
    summary = L(
      "Weiterfahren geht, aber lass die Punkte unten in den nächsten Wochen ansehen.",
      "You can keep driving, but have the points below looked at in the next few weeks.",
    );
  } else if (count(WAIT) > 0) {
    headline = L("Nichts Dringendes", "Nothing urgent");
    summary = L("Was unten steht, kann bis zur nächsten Inspektion warten.", "What is listed below can wait until the next service.");
  } else {
    headline = L("Alles in Ordnung", "All good");
    summary = L("Kein Fehler gespeichert und nichts Auffälliges gefunden.", "No fault stored and nothing unusual found.");
  }
  return { headline, summary, nothingRead, items: sorted };
}

// --- The connection assistant ----------------------------------------------------------

export interface ConnectFacts {
  kind: "usb" | "bluetooth" | "wifi" | "network" | "can" | "replay";
  portsFound?: number;
  deviceFound?: boolean;
  portOpened?: boolean;
  portBusy?: boolean;
  adapterAnswered?: boolean;
  adapterId?: string;
  protocolsTried?: string;
  busError?: string;
  baudsTried?: number[];
  canBitRate?: number;
}

function protocolFamilies(numbers: string): string {
  const out: string[] = [];
  for (const c of numbers) {
    const name =
      c === "1" || c === "2" ? "SAE J1850" : c === "3" ? "ISO 9141 (K-Line)" : c === "4" || c === "5" ? "KWP2000 (K-Line)" : c >= "6" && c <= "9" ? "CAN" : c >= "A" && c <= "C" ? "CAN (J1939)" : "";
    if (name && !out.includes(name)) out.push(name);
  }
  return out.join(", ");
}

function looksLikeClone(id: string): boolean {
  const up = id.toUpperCase().trim();
  if (!up || up === "?") return false;
  return up.includes("V1.5") || !(up.includes("ELM") || up.includes("OBD") || up.includes("STN"));
}

function check(id: string, state: ConnectCheck["state"], title: string, text: string): ConnectCheck {
  return { id, state, title, text: state === "fine" ? "" : text };
}

function ignitionText(): string {
  return L(
    "Ohne Zündung antwortet kein Steuergerät. Zündung einschalten, der Motor darf aus sein.",
    "Without the ignition no control unit answers. Switch the ignition on, the engine may stay off.",
  );
}

function busText(error: string): string {
  return args(
    L(
      "Der Adapter meldet %1. Das heißt meist: Zündung aus, falsches Protokoll oder ein Problem an der Leitung zum Auto.",
      "The adapter reports %1. That usually means: ignition off, wrong protocol or a problem on the line to the car.",
    ),
    error,
  );
}

function foundLines(f: ConnectFacts): string[] {
  const out: string[] = [];
  if (f.portsFound === 0) out.push(L("Am Rechner ist kein Anschluss zu sehen.", "No port shows up on this computer."));
  else if (f.portsFound !== undefined) out.push(args(L("Anschlüsse am Rechner: %1", "Ports on this computer: %1"), String(f.portsFound)));
  if (f.deviceFound === false) out.push(L("Kein Bluetooth-Adapter in der Nähe gefunden.", "No Bluetooth adapter found nearby."));
  if (f.portOpened === false && f.portBusy) out.push(L("Der Anschluss ist von einem anderen Programm belegt.", "Another program is using the port."));
  else if (f.portOpened === false) out.push(L("Die Verbindung zum Adapter ließ sich nicht öffnen.", "The connection to the adapter could not be opened."));
  else if (f.portOpened === true) out.push(L("Die Verbindung zum Adapter ist offen.", "The connection to the adapter is open."));
  if (f.adapterAnswered === true) out.push(args(L("Der Adapter antwortet: %1", "The adapter answers: %1"), f.adapterId ?? ""));
  else if (f.adapterAnswered === false) out.push(L("Der Adapter antwortet nicht.", "The adapter does not answer."));
  const families = protocolFamilies(f.protocolsTried ?? "");
  if (families) out.push(args(L("Probierte Protokolle: %1. Kein Steuergerät hat geantwortet.", "Protocols tried: %1. No control unit answered."), families));
  if (f.busError) out.push(args(L("Der Adapter meldet: %1", "The adapter reports: %1"), f.busError));
  return out;
}

function adapterAndCar(f: ConnectFacts, out: ConnectCheck[]): void {
  const answered = f.adapterAnswered === true;
  if (f.portOpened === true) {
    out.push(
      check(
        "power",
        answered ? "fine" : "problem",
        L("Adapter hat Strom", "Adapter has power"),
        L(
          "Die meisten Adapter leuchten, sobald sie in der Buchse stecken. Kein Licht: die Sicherung der Diagnosebuchse prüfen, oft ist es die des Zigarettenanzünders.",
          "Most adapters light up as soon as they sit in the socket. No light: check the fuse of the diagnostic socket, often the one of the cigarette lighter.",
        ),
      ),
    );
  }
  if (answered && looksLikeClone(f.adapterId ?? "")) {
    out.push(
      check(
        "clone",
        "check",
        L("Nachbau-Adapter", "Clone adapter"),
        args(
          L(
            "Der Adapter nennt sich %1. Das steht fast nur auf billigen Nachbauten, die manche Protokolle nicht beherrschen; ein anderer Adapter hilft oft.",
            "The adapter calls itself %1. That is found almost only on cheap clones that cannot handle some protocols; another adapter often helps.",
          ),
          f.adapterId ?? "",
        ),
      ),
    );
  }
  if (f.busError) out.push(check("bus", "problem", L("Meldung vom Fahrzeugbus", "Message from the vehicle bus"), busText(f.busError)));
  out.push(check("ignition", answered ? "problem" : "check", L("Zündung an", "Ignition on"), ignitionText()));
  out.push(
    check(
      "plug",
      "check",
      L("Stecker sitzt fest", "Plug firmly seated"),
      L(
        "Den Adapter abziehen, 10 Sekunden warten und fest in die OBD-Buchse stecken.",
        "Unplug the adapter, wait 10 seconds and push it firmly into the OBD socket.",
      ),
    ),
  );
  const families = protocolFamilies(f.protocolsTried ?? "");
  if (answered && families) {
    out.push(
      check(
        "protocol",
        "check",
        L("Protokoll", "Protocol"),
        args(
          L(
            "Probiert: %1. Auf dieser Seite das Modell auswählen, dann probiert Caruspex zuerst das passende Protokoll; Autos vor etwa 2005 sprechen oft K-Line.",
            "Tried: %1. Pick the model on this page, then Caruspex tries the matching protocol first; cars from before about 2005 often speak K-Line.",
          ),
          families,
        ),
      ),
    );
  }
}

export function connectHelp(f: ConnectFacts): ConnectHelp {
  const items: ConnectCheck[] = [];
  const opened = f.portOpened === true;
  const answered = f.adapterAnswered === true;
  switch (f.kind) {
    case "replay":
      return { found: [], items: [] };
    case "usb": {
      const noPort = f.portsFound === 0;
      items.push(
        check(
          "driver",
          noPort ? "problem" : opened || f.portsFound !== undefined ? "fine" : "check",
          L("Treiber", "Driver"),
          L(
            "Ohne Treiber zeigt der Rechner keinen Anschluss. Meist fehlt der Treiber für den Chip im Kabel (CH340, FTDI oder CP210x), oder das USB-Kabel ist nur ein Ladekabel.",
            "Without a driver the computer shows no port. Usually the driver for the chip in the cable is missing (CH340, FTDI or CP210x), or the USB cable is a charging cable only.",
          ),
        ),
      );
      if (!noPort) {
        items.push(
          check(
            "port",
            f.portBusy ? "problem" : opened ? "fine" : "check",
            L("Anschluss frei", "Port free"),
            f.portBusy
              ? L(
                  "Ein anderes Programm hält den Anschluss, etwa eine andere Diagnose-App oder ein zweites Caruspex. Schließen und noch einmal verbinden.",
                  "Another program holds the port, such as another diagnostic app or a second Caruspex. Close it and connect again.",
                )
              : L(
                  "Steckt der Adapter noch, und ist es der richtige Anschluss? Im Geräte-Manager unter Anschlüsse steht, welcher zum Adapter gehört.",
                  "Is the adapter still plugged in, and is it the right port? Device Manager lists under Ports which one belongs to the adapter.",
                ),
          ),
        );
      }
      if (opened) {
        items.push(
          check(
            "baud",
            answered ? "fine" : "check",
            L("Baudrate", "Baud rate"),
            args(
              L(
                "Probiert: %1 Baud. Manche Adapter brauchen eine feste Rate, einstellbar in den Einstellungen unter Verbindung.",
                "Tried: %1 baud. Some adapters need a fixed rate, which can be set in Settings under Connection.",
              ),
              (f.baudsTried ?? []).join(", "),
            ),
          ),
        );
      }
      if (opened || f.portOpened === undefined) adapterAndCar(f, items);
      break;
    }
    case "bluetooth":
      items.push(
        check(
          "bluetooth",
          f.deviceFound === true || opened ? "fine" : "check",
          L("Bluetooth am Rechner", "Bluetooth on this computer"),
          L("Bluetooth muss am Rechner eingeschaltet sein.", "Bluetooth must be switched on on this computer."),
        ),
      );
      items.push(
        check(
          "pairing",
          opened ? "fine" : "problem",
          L("Kopplung", "Pairing"),
          f.deviceFound === false
            ? L(
                "Kein Adapter in der Nähe. Bluetooth-LE-Adapter brauchen keine Kopplung; klassische einmal in den Systemeinstellungen koppeln, die PIN ist meist 1234 oder 0000. Manche Adapter haben nur bei Zündung an Strom.",
                "No adapter nearby. Bluetooth LE adapters need no pairing; pair classic ones once in the system settings, the PIN is usually 1234 or 0000. Some adapters only have power with the ignition on.",
              )
            : L(
                "Der Adapter ließ keine Verbindung zu. Ist er noch mit einem Handy verbunden? Ein Adapter spricht nur mit einem Gerät.",
                "The adapter refused the connection. Is it still connected to a phone? An adapter talks to one device only.",
              ),
        ),
      );
      if (opened) adapterAndCar(f, items);
      else items.push(check("ignition", "check", L("Zündung an", "Ignition on"), ignitionText()));
      break;
    case "wifi":
      items.push(
        check(
          "wifi",
          opened ? "fine" : "problem",
          L("WLAN des Adapters", "The adapter's Wi-Fi"),
          L(
            "Der Rechner muss mit dem WLAN des Adapters verbunden sein (oft WiFi_OBDII), nicht mit dem Heimnetz.",
            "The computer must be connected to the adapter's Wi-Fi (often WiFi_OBDII), not to the home network.",
          ),
        ),
      );
      if (!opened) {
        items.push(
          check(
            "address",
            "check",
            L("Adresse und Port", "Address and port"),
            L(
              "Die meisten Adapter nutzen 192.168.0.10 und Port 35000, manche 192.168.4.1. Das steht in der Anleitung des Adapters.",
              "Most adapters use 192.168.0.10 and port 35000, some 192.168.4.1. The adapter's manual says which.",
            ),
          ),
        );
      }
      adapterAndCar(f, items);
      break;
    case "network":
      items.push(
        check(
          "cable",
          "check",
          L("Netzwerkkabel", "Network cable"),
          L(
            "Ein ENET-Kabel (Ethernet zu OBD) braucht die Brücke für die Aktivierung; ein einfaches Netzwerkkabel ohne sie geht nicht.",
            "An ENET cable (Ethernet to OBD) needs the activation bridge; a plain network cable without it does not work.",
          ),
        ),
        check(
          "address",
          "check",
          L("Netzwerkadresse", "Network address"),
          L(
            "Der Rechner sollte seine Adresse automatisch beziehen. Antwortet das Auto nicht darauf, vergibt der Rechner nach etwa einer Minute selbst eine; dann noch einmal verbinden.",
            "The computer should obtain its address automatically. If the car does not answer, the computer assigns one itself after about a minute; then connect again.",
          ),
        ),
        check(
          "firewall",
          "check",
          L("Firewall", "Firewall"),
          L(
            "Die Firewall muss die Suche nach dem Auto zulassen (UDP 6811 für ENET, 13400 für DoIP).",
            "The firewall must allow the search for the car (UDP 6811 for ENET, 13400 for DoIP).",
          ),
        ),
        check("ignition", "check", L("Zündung an", "Ignition on"), ignitionText()),
      );
      break;
    case "can":
      items.push(
        check(
          "driver",
          opened ? "fine" : "problem",
          L("Treiber", "Driver"),
          L(
            "Das CAN-Interface ließ sich nicht öffnen. Ist es eingesteckt und der Treiber installiert? Viele J2534-Treiber gibt es nur in 32 Bit, Caruspex braucht 64 Bit.",
            "The CAN interface could not be opened. Is it plugged in and the driver installed? Many J2534 drivers exist in 32 bit only, Caruspex needs 64 bit.",
          ),
        ),
      );
      if (opened) {
        if (f.busError) items.push(check("bus", "problem", L("Meldung vom Fahrzeugbus", "Message from the vehicle bus"), busText(f.busError)));
        items.push(
          check(
            "bitrate",
            "check",
            L("Bitrate", "Bit rate"),
            args(
              L(
                "Probiert: %1 kbit/s. Fast alle Autos ab 2008 nutzen 500 kbit/s an der OBD-Buchse, manche ältere 250 kbit/s.",
                "Tried: %1 kbit/s. Almost all cars from 2008 use 500 kbit/s at the OBD socket, some older ones 250 kbit/s.",
              ),
              String(Math.floor((f.canBitRate ?? 0) / 1000)),
            ),
          ),
          check("ignition", "problem", L("Zündung an", "Ignition on"), ignitionText()),
        );
      }
      break;
  }
  const rank = { problem: 0, check: 1, fine: 2 } as const;
  const sorted = items.map((it, i) => ({ it, i })).sort((a, b) => rank[a.it.state] - rank[b.it.state] || a.i - b.i);
  return { found: foundLines(f), items: sorted.map(({ it }) => it) };
}
