// Demo answers for the diagnose pages: the ports and interfaces the connect
// page lists, the Bluetooth scan, mode 06 with the identity (Worker::
// readMonitors) and the health check (Worker::runHealthCheck with
// analysis::runHealthCheck of core/analysis/HealthCheck.cpp). The numbers
// come from the demo car the same way DemoTransport.cpp builds its answers,
// so the pages show what the Qt version shows on the demo car.
import { t } from "../../../i18n";
import type {
  BleDeviceRow,
  HealthFindingRow,
  HealthRow,
  J2534DeviceRow,
  Level,
  MonitorGuideRow,
  MonitorRow,
  SerialPortInfo,
} from "../../types";
import { DEMO_VIN, ENGINE_CALIBRATION, ENGINE_CVN, simulate } from "../car";
import { L } from "../core";
import { lookupDtc } from "../dtc";
import { liveRow, SUPPORTED_PIDS } from "../pids";
import { appSeconds, car, emit, link, log, wait, type PartialHandlers } from "../runtime";
import { faultRows, statusRow } from "../session";

const W = "caruspex::Worker";

// --- Ports and interfaces -------------------------------------------------------------

// A browser cannot see the serial ports; two plausible ones let every part of
// the connect page be seen: a CH340 adapter and the mainboard port.
const DEMO_PORTS: SerialPortInfo[] = [
  { device: "COM3", description: "USB-SERIAL CH340 (COM3)", likelyAdapter: true, chipGuess: "CH340" },
  { device: "COM1", description: "Kommunikationsanschluss (COM1)", likelyAdapter: false, chipGuess: "" },
];

// One usable PassThru device and one with only a 32 bit driver, so the note
// for the latter shows as well.
const DEMO_J2534: J2534DeviceRow[] = [
  {
    name: "OpenPort 2.0 J2534 ISO/CAN/VPW/PWM",
    vendor: "Tactrix Inc.",
    library: "C:\\Windows\\System32\\op20pt64.dll",
    usable: true,
  },
  {
    name: "Scanmatik 2 PRO",
    vendor: "Scanmatik",
    library: "C:\\Program Files (x86)\\Scanmatik\\sm2j2534.dll",
    usable: false,
  },
];

// What a scan near a car typically finds: a BLE adapter, a dual mode one and
// a speaker that is no adapter at all (Worker::bleScanFinished scores).
const DEMO_BLE: BleDeviceRow[] = [
  { id: "D2:5C:41:7A:09:3E", name: "IOS-Vlink", score: 5, kinds: 1 },
  { id: "00:1D:A5:68:98:8B", name: "OBDII", score: 4, kinds: 3 },
  { id: "F4:4E:FD:12:6B:70", name: "JBL Flip 5", score: 0, kinds: 2 },
];

// --- Two-language phrases (core/analysis/Common.h) ------------------------------------

interface Phrase {
  de: string;
  en: string;
}

const same = (s: string): Phrase => ({ de: s, en: s });
const text = (p: Phrase): string => L(p.de, p.en);

function fill(pattern: string, args: string[]): string {
  let i = 0;
  return pattern.replace(/\{\}/g, () => args[i++] ?? "");
}

// format() of Common.cpp: each "{} " takes the next argument in its language.
function format(de: string, en: string, args: Phrase[] = []): Phrase {
  return { de: fill(de, args.map((a) => a.de)), en: fill(en, args.map((a) => a.en)) };
}

// operator+ of Common.cpp: joins with a space, skipping empty parts.
function plus(...parts: Phrase[]): Phrase {
  const join = (x: string, y: string) => (!x ? y : !y ? x : `${x} ${y}`);
  return parts.reduce((a, b) => ({ de: join(a.de, b.de), en: join(a.en, b.en) }), { de: "", en: "" });
}

function formatNumber(value: number, decimals: number, german: boolean): string {
  let out = value.toFixed(Math.max(0, Math.min(6, decimals)));
  // "-0" or "-0.0" looks odd next to a measured value.
  if (/^-[0.]+$/.test(out)) out = out.slice(1);
  return german ? out.replace(".", ",") : out;
}

function number(value: number, decimals: number): Phrase {
  return { de: formatNumber(value, decimals, true), en: formatNumber(value, decimals, false) };
}

function signedNumber(value: number, decimals: number): Phrase {
  const p = number(value, decimals);
  if (Math.round(value * 10 ** decimals) > 0) return { de: `+${p.de}`, en: `+${p.en}` };
  return p;
}

// --- Mode 06 (DemoTransport::monitorRecords, decoded like Obd2.cpp) ----------------

interface MonitorRecord {
  componentId: number;
  testId: number;
  name: string;
  component: string;
  value: number;
  minimum: number;
  maximum: number;
  unit: string;
  passed: boolean;
}

const ENGINE_MIDS = [0x01, 0x02, 0x21, 0x3c, 0x41, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5];

// data/MonitorTests.cpp: monitorScale for the unit and scaling IDs the demo uses.
function monitorScale(uas: number, raw: number): { value: number; unit: string } {
  switch (uas) {
    case 0x0b: return { value: raw * 0.001, unit: "V" };
    case 0x10: return { value: raw, unit: "ms" };
    case 0x20: return { value: raw * 0.0039062, unit: "" };
    default: return { value: raw, unit: "" };
  }
}

function oxygenTestName(testId: number): string {
  switch (testId) {
    case 0x01: return L("Spannungsschwelle mager nach fett", "Lean-to-rich threshold voltage");
    case 0x02: return L("Spannungsschwelle fett nach mager", "Rich-to-lean threshold voltage");
    case 0x03: return L("tiefste Spannung im Regelbetrieb", "Minimum voltage in closed loop");
    case 0x04: return L("höchste Spannung im Regelbetrieb", "Maximum voltage in closed loop");
    case 0x05: return L("Umschaltzeit mager nach fett", "Lean-to-rich switch time");
    case 0x06: return L("Umschaltzeit fett nach mager", "Rich-to-lean switch time");
    case 0x07: return L("tiefste Sondenspannung", "Minimum sensor voltage");
    case 0x08: return L("höchste Sondenspannung", "Maximum sensor voltage");
    case 0x09: return L("Zeit zwischen den Umschaltungen", "Time between transitions");
    case 0x0a: return L("Periodendauer der Regelung", "Control period");
    default: return `Test ${testId}`;
  }
}

const hex2 = (v: number) => (v & 0xff).toString(16).toUpperCase().padStart(2, "0");

function monitorTestNameCan(mid: number, tid: number): string {
  if (mid >= 0xa1 && mid <= 0xad) {
    if (tid === 0x0b) return L("Aussetzer im Schnitt der letzten 10 Fahrten", "Misfires, average of the last 10 trips");
    if (tid === 0x0c) return L("Aussetzer in der aktuellen Fahrt", "Misfires in the current trip");
  }
  if (tid >= 0x01 && tid <= 0x0a && mid >= 0x01 && mid <= 0x10) return oxygenTestName(tid);
  return `Test 0x${hex2(tid)}${L(" (herstellerspezifisch)", " (manufacturer-specific)")}`;
}

function monitorComponentName(id: number): string {
  const bankSensor = (index: number) =>
    `${L("Bank ", "bank ")}${Math.floor(index / 4) + 1}${L(" Sonde ", " sensor ")}${(index % 4) + 1}`;
  if (id >= 0x01 && id <= 0x10) return L("Lambdasonde ", "O2 sensor ") + bankSensor(id - 0x01);
  if (id >= 0x21 && id <= 0x24) return L("Katalysator", "Catalyst") + L(" Bank ", " bank ") + (id - 0x20);
  if (id === 0x3c) return L("Tankentlüftung, Leck 0,020 Zoll", "Evaporative system, 0.020 inch leak");
  if (id >= 0x41 && id <= 0x50) return L("Lambdasondenheizung ", "O2 sensor heater ") + bankSensor(id - 0x41);
  if (id === 0xa1) return L("Verbrennungsaussetzer, ganzer Motor", "Misfire, whole engine");
  if (id >= 0xa2 && id <= 0xad) return L("Verbrennungsaussetzer Zylinder ", "Misfire cylinder ") + (id - 0xa1);
  return L("Baugruppe 0x", "Component 0x") + hex2(id);
}

function monitorRecords(time = car.now()): MonitorRecord[] {
  const out: MonitorRecord[] = [];
  const s = simulate(time);
  const r = car.runSinceClear(time);
  // Misfire counters: cylinder 3 misfires now and then (P0300).
  const cycleRun = Math.min(s.runTime, r);
  const current = [Math.trunc(cycleRun * 0.002), 0, Math.trunc(cycleRun * 0.06), Math.trunc(cycleRun * 0.004)];
  const average = [
    car.cleared ? Math.min(1, Math.trunc(r / 300)) : 1,
    0,
    car.cleared ? Math.min(41, Math.trunc(r / 15)) : 41,
    car.cleared ? Math.min(2, Math.trunc(r / 200)) : 2,
  ];
  for (const mid of ENGINE_MIDS) {
    const done = car.monitorComplete(mid, time);
    const record = (tid: number, uas: number, value: number, low: number, high: number) => {
      // A test that has not run since clearing reports zeros.
      if (!done) value = low = high = 0;
      const v = monitorScale(uas, value);
      const minimum = monitorScale(uas, low).value;
      const maximum = monitorScale(uas, high).value;
      out.push({
        componentId: mid,
        testId: tid,
        name: monitorTestNameCan(mid, tid),
        component: monitorComponentName(mid),
        value: v.value,
        minimum,
        maximum,
        unit: v.unit,
        passed: v.value >= minimum && v.value <= maximum,
      });
    };
    switch (mid) {
      case 0x01: // O2 sensor bank 1 sensor 1: switch times in ms
        record(0x05, 0x10, 68, 0, 150);
        record(0x06, 0x10, 74, 0, 150);
        break;
      case 0x02: // O2 sensor bank 1 sensor 2: min and max voltage in mV
        record(0x07, 0x0b, 84, 0, 400);
        record(0x08, 0x0b, 812, 600, 1100);
        break;
      case 0x21: // catalyst bank 1: oxygen storage ratio, fails (P0420)
        record(0x80, 0x20, 210, 0, 154);
        break;
      case 0x3c: // EVAP 0.020 inch leak: never completed
        record(0x80, 0x01, 0, 0, 0);
        break;
      case 0x41: // O2 heater bank 1 sensor 1: warm-up time in ms
        record(0x80, 0x10, 14200, 0, 40000);
        break;
      case 0xa1:
        record(0x0b, 0x24, average.reduce((a, b) => a + b, 0), 0, 0xffff);
        record(0x0c, 0x24, current.reduce((a, b) => a + b, 0), 0, 0xffff);
        break;
      default: {
        const cylinder = mid - 0xa2;
        record(0x0b, 0x24, average[cylinder]!, 0, 0xffff);
        record(0x0c, 0x24, current[cylinder]!, 0, 0xffff);
      }
    }
  }
  return out;
}

function toMonitorRow(r: MonitorRecord): MonitorRow {
  return {
    name: r.name,
    component: r.component,
    value: r.value,
    minimum: r.minimum,
    maximum: r.maximum,
    unit: r.unit,
    hasMinimum: true,
    hasMaximum: true,
    passed: r.passed,
  };
}

// The demo talks CAN, which has no mode 05 any more (Obd2::readOxygenSensorTests).
function oxygenNote(): string {
  return L(
    "Das Steuergerät liefert keine Lambdasondenprüfung. Bei Fahrzeugen mit CAN " +
      "ist dieser Dienst abgeschafft, die Werte stecken dort in der " +
      "Bauteilüberwachung.",
    "The control unit provides no oxygen sensor test results. Vehicles with CAN " +
      "no longer have this service; the values are part of component " +
      "monitoring there.",
  );
}

// --- Health check (core/analysis/HealthCheck.cpp) --------------------------------------

// analysis::Monitor, the ones the demo engine supports.
enum Monitor {
  Misfire,
  FuelSystem,
  Components,
  Catalyst,
  HeatedCatalyst,
  Evap,
  SecondaryAir,
  AcRefrigerant,
  OxygenSensor,
  OxygenSensorHeater,
  Egr,
}

interface MonitorState {
  monitor: Monitor;
  supported: boolean;
  complete: boolean;
}

// monitorName() of Readiness.cpp.
const MONITOR_NAMES: Record<Monitor, Phrase> = {
  [Monitor.Misfire]: { de: "Verbrennungsaussetzer", en: "Misfire" },
  [Monitor.FuelSystem]: { de: "Kraftstoffsystem", en: "Fuel system" },
  [Monitor.Components]: { de: "Bauteilüberwachung", en: "Comprehensive components" },
  [Monitor.Catalyst]: { de: "Katalysator", en: "Catalyst" },
  [Monitor.HeatedCatalyst]: { de: "Katalysatorheizung", en: "Heated catalyst" },
  [Monitor.Evap]: { de: "Tankentlüftung", en: "Evaporative system" },
  [Monitor.SecondaryAir]: { de: "Sekundärluft", en: "Secondary air system" },
  [Monitor.AcRefrigerant]: { de: "Klimaanlage", en: "A/C refrigerant" },
  [Monitor.OxygenSensor]: { de: "Lambdasonde", en: "Oxygen sensor" },
  [Monitor.OxygenSensorHeater]: { de: "Lambdasondenheizung", en: "Oxygen sensor heater" },
  [Monitor.Egr]: { de: "Abgasrückführung / Nockenwellenverstellung", en: "EGR / variable valve timing" },
};

interface Guide {
  explanation: Phrase;
  drivePattern: Phrase;
  typicalTime: Phrase;
  coldStart: boolean;
}

// allDriveCycleGuides() of DriveCycleGuide.cpp for the petrol monitors.
const GUIDES: Partial<Record<Monitor, Guide>> = {
  [Monitor.Misfire]: {
    explanation: {
      de: "Prüft, ob in jedem Zylinder das Gemisch sauber zündet. Ein Aussetzer schüttelt den Motor und schickt unverbranntes Benzin in den Katalysator, der davon überhitzen kann.",
      en: "Checks that the mixture fires cleanly in every cylinder. A misfire shakes the engine and sends unburnt fuel into the catalytic converter, which can overheat from it.",
    },
    drivePattern: {
      de: "Läuft ständig mit. Ein paar Minuten normale Fahrt mit wechselnder Drehzahl reichen meistens.",
      en: "Runs all the time. A few minutes of normal driving at varying engine speeds is usually enough.",
    },
    typicalTime: { de: "wenige Minuten", en: "a few minutes" },
    coldStart: false,
  },
  [Monitor.FuelSystem]: {
    explanation: {
      de: "Prüft, ob die Gemischregelung das Verhältnis von Luft und Kraftstoff im erlaubten Bereich halten kann. Muss sie zu stark nachregeln, stimmt etwas nicht (Falschluft, Einspritzung, Kraftstoffdruck).",
      en: "Checks that the mixture control can keep the ratio of air and fuel within its allowed range. If it has to correct too much, something is off (air leak, injection, fuel pressure).",
    },
    drivePattern: {
      de: "Läuft ständig mit, sobald der Motor warm ist. Einige Minuten Leerlauf und gleichmäßige Fahrt.",
      en: "Runs all the time once the engine is warm. A few minutes of idling and steady driving.",
    },
    typicalTime: { de: "5 bis 10 Minuten", en: "5 to 10 minutes" },
    coldStart: false,
  },
  [Monitor.Components]: {
    explanation: {
      de: "Prüft die Sensoren und Stellglieder des Motors auf Kurzschluss, Unterbrechung und unplausible Werte, zum Beispiel Temperaturfühler und Drosselklappe.",
      en: "Checks the engine's sensors and actuators for short circuits, open circuits and implausible values, for example temperature sensors and the throttle.",
    },
    drivePattern: {
      de: "Läuft ständig mit und ist meist schon kurz nach dem Start fertig.",
      en: "Runs all the time and is usually complete shortly after starting.",
    },
    typicalTime: { de: "unter einer Minute bis wenige Minuten", en: "under a minute to a few minutes" },
    coldStart: false,
  },
  [Monitor.Catalyst]: {
    explanation: {
      de: "Prüft, ob der Katalysator die Abgase noch richtig reinigt. Dafür vergleicht das Steuergerät die Lambdasonde vor dem Katalysator mit der dahinter.",
      en: "Checks that the catalytic converter still cleans the exhaust properly. To do so, the control unit compares the oxygen sensor in front of the converter with the one behind it.",
    },
    drivePattern: {
      de: "Motor ganz warm fahren, dann 5 bis 10 Minuten gleichmäßig zwischen 70 und 100 km/h auf ebener Strecke, mit ruhigem Gasfuß. Danach ein paar Mal vom Gas gehen und ausrollen lassen. Manche Autos brauchen dafür zwei oder drei Fahrten.",
      en: "Drive until the engine is fully warm, then 5 to 10 minutes at a steady 70 to 100 km/h (45 to 60 mph) on a level road with a light foot. Afterwards lift off the throttle a few times and let the car coast. Some cars need two or three trips.",
    },
    typicalTime: { de: "15 bis 30 Minuten, oft über mehrere Fahrten", en: "15 to 30 minutes, often over several trips" },
    coldStart: false,
  },
  [Monitor.Evap]: {
    explanation: {
      de: "Prüft, ob das Tanksystem dicht ist, damit keine Benzindämpfe entweichen. Schon ein schlecht geschlossener Tankdeckel kann den Test scheitern lassen.",
      en: "Checks that the fuel tank system is sealed so no fuel vapour escapes. Even a fuel cap that is not closed properly can make the test fail.",
    },
    drivePattern: {
      de: "Der schwierigste Test. Das Auto mindestens 8 Stunden stehen lassen (am besten über Nacht), Tank zwischen einem Viertel und drei Vierteln voll, Tankdeckel fest zu. Kaltstart, dann ruhig fahren mit Stadt- und Landstraßenanteil und einer Minute Leerlauf am Ende. Viele Autos testen erst nach dem Abstellen, also danach wieder einige Stunden stehen lassen. Bei sehr kaltem oder sehr heißem Wetter läuft der Test oft gar nicht.",
      en: "The hardest one. Let the car sit for at least 8 hours (overnight is best), tank between a quarter and three quarters full, fuel cap tightly closed. Cold start, then drive calmly with some town and some country road and a minute of idling at the end. Many cars run the test only after being switched off, so let it sit for a few hours again afterwards. In very cold or very hot weather the test often does not run at all.",
    },
    typicalTime: { de: "oft ein bis mehrere Tage normaler Nutzung", en: "often one to several days of normal use" },
    coldStart: true,
  },
  [Monitor.OxygenSensor]: {
    explanation: {
      de: "Prüft, ob die Lambdasonden schnell und richtig auf den Sauerstoff im Abgas reagieren. Sie sind die Augen der Gemischregelung.",
      en: "Checks that the oxygen sensors react quickly and correctly to the oxygen in the exhaust. They are the eyes of the mixture control.",
    },
    drivePattern: {
      de: "Motor warm fahren, dann einige Minuten gleichmäßig zwischen 50 und 90 km/h. Danach mehrmals vom Gas gehen und von etwa 90 auf 50 km/h ausrollen lassen, ohne zu bremsen. Zum Schluss eine Minute Leerlauf.",
      en: "Drive until warm, then a few minutes at a steady 50 to 90 km/h (30 to 55 mph). Afterwards lift off the throttle several times and coast from about 90 down to 50 km/h without braking. Finish with a minute of idling.",
    },
    typicalTime: { de: "10 bis 20 Minuten", en: "10 to 20 minutes" },
    coldStart: false,
  },
  [Monitor.OxygenSensorHeater]: {
    explanation: {
      de: "Prüft die Heizung in den Lambdasonden. Die Sonden messen erst ab etwa 300 Grad richtig, die Heizung bringt sie nach dem Start schnell dahin.",
      en: "Checks the heaters inside the oxygen sensors. The sensors only measure properly from about 300 degrees Celsius, and the heater gets them there quickly after starting.",
    },
    drivePattern: {
      de: "Kaltstart, danach ein paar Minuten Leerlauf oder ruhige Fahrt.",
      en: "Cold start, then a few minutes of idling or gentle driving.",
    },
    typicalTime: { de: "wenige Minuten nach einem Kaltstart", en: "a few minutes after a cold start" },
    coldStart: true,
  },
  [Monitor.Egr]: {
    explanation: {
      de: "Prüft die Abgasrückführung, die einen Teil des Abgases zurück in den Motor leitet und so Stickoxide senkt. Bei manchen Autos steckt hier auch die Nockenwellenverstellung mit drin.",
      en: "Checks the exhaust gas recirculation, which leads part of the exhaust back into the engine to lower nitrogen oxides. On some cars the variable valve timing is checked here as well.",
    },
    drivePattern: {
      de: "Motor warm fahren, dann gleichmäßig um 70 bis 90 km/h fahren und mehrmals vom Gas gehen und lange ausrollen lassen, ohne zu bremsen. Beim Diesel helfen zusätzlich längere Abschnitte mit gleichmäßiger Teillast.",
      en: "Drive until warm, then cruise at about 70 to 90 km/h (45 to 55 mph) and lift off the throttle several times, letting the car coast for a long stretch without braking. On diesels, longer stretches at steady part load help as well.",
    },
    typicalTime: { de: "10 bis 20 Minuten", en: "10 to 20 minutes" },
    coldStart: false,
  },
};

// Readiness of the demo engine from PID 01: bytes B and C say what is
// supported, byte D what is still open (DemoTransport::enginePid 0x01).
function readinessStates(): MonitorState[] {
  const byteB = 0x07;
  const byteC = 0xe5;
  const byteD = car.readinessD();
  const out: MonitorState[] = [];
  const continuous: [number, Monitor][] = [
    [0x01, Monitor.Misfire],
    [0x02, Monitor.FuelSystem],
    [0x04, Monitor.Components],
  ];
  for (const [mask, monitor] of continuous) {
    out.push({ monitor, supported: (byteB & mask) !== 0, complete: (byteB & (mask << 4)) === 0 });
  }
  const spark: [number, Monitor][] = [
    [0x01, Monitor.Catalyst],
    [0x02, Monitor.HeatedCatalyst],
    [0x04, Monitor.Evap],
    [0x08, Monitor.SecondaryAir],
    [0x10, Monitor.AcRefrigerant],
    [0x20, Monitor.OxygenSensor],
    [0x40, Monitor.OxygenSensorHeater],
    [0x80, Monitor.Egr],
  ];
  for (const [mask, monitor] of spark) {
    out.push({ monitor, supported: (byteC & mask) !== 0, complete: (byteD & mask) === 0 });
  }
  return out;
}

type FaultKind = "stored" | "pending" | "permanent";

interface HealthFault {
  code: string;
  text: string;
  kind: FaultKind;
}

interface TimedValue {
  t: number;
  value: number;
}

interface HealthInput {
  milOn: boolean;
  readiness: MonitorState[];
  faults: HealthFault[];
  monitorResults: MonitorRecord[];
  diesel: boolean;
  live: Map<number, TimedValue[]>;
}

interface Finding {
  id: string;
  level: Level;
  title: Phrase;
  explanation: Phrase;
  advice: Phrase;
}

interface Skipped {
  id: string;
  title: Phrase;
  reason: Phrase;
}

// healthLivePids() of HealthCheck.cpp.
const HEALTH_LIVE_PIDS = [
  0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0c, 0x0d, 0x0f, 0x10, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1f,
  0x21, 0x30, 0x31, 0x3c, 0x42, 0x5c,
];

const PID_FUEL_STATUS = 0x03;
const PID_COOLANT = 0x05;
const PID_RPM = 0x0c;
const PID_INTAKE = 0x0f;
const PID_RUN_TIME = 0x1f;
const PID_MIL_DISTANCE = 0x21;
const PID_WARMUPS = 0x30;
const PID_CLEAR_DISTANCE = 0x31;
const PID_CATALYST = 0x3c;
const PID_VOLTAGE = 0x42;
const PID_OIL = 0x5c;

const RUNNING_RPM = 400;
const STOPPED_RPM = 50;
const MATCH_WINDOW = 3;
const WARM_FOR_TRIMS = 70;

// analysis::limits of HealthCheck.h.
const limits = {
  trimWarn: 10,
  trimBad: 20,
  chargingLow: 13.2,
  chargingNone: 12.6,
  chargingHigh: 15.0,
  chargingTooHigh: 15.5,
  batteryWeak: 12.2,
  batteryFlat: 11.8,
  warmCoolant: 80,
  coldThermostat: 75,
  longRunSeconds: 15 * 60,
  coolantHot: 108,
  coolantOverheat: 115,
  oilHot: 135,
  oilOverheat: 150,
  catalystHot: 900,
  nearLimitShare: 0.1,
  fewWarmups: 5,
  shortDistanceKm: 50,
};

const GOOD: Level = 0;
const INFO: Level = 1;
const WARNING: Level = 2;
const CRITICAL: Level = 3;

const mean = (v: number[]) => (v.length === 0 ? 0 : v.reduce((a, b) => a + b, 0) / v.length);
function median(v: number[]): number {
  if (v.length === 0) return 0;
  const s = [...v].sort((a, b) => a - b);
  const n = s.length;
  return n % 2 ? s[(n - 1) / 2]! : (s[n / 2 - 1]! + s[n / 2]!) / 2;
}

class Ctx {
  findings: Finding[] = [];
  notChecked: Skipped[] = [];
  checksRun = 0;
  constructor(readonly input: HealthInput) {}

  add(id: string, level: Level, title: Phrase, explanation: Phrase, advice: Phrase = { de: "", en: "" }): void {
    this.findings.push({ id, level, title, explanation, advice });
  }
  skip(id: string, title: Phrase, reason: Phrase): void {
    this.notChecked.push({ id, title, reason });
  }
  ran(): void {
    this.checksRun++;
  }
  get(pid: number): TimedValue[] | null {
    const s = this.input.live.get(pid);
    return s && s.length > 0 ? s : null;
  }
  // The reading closest in time to t, if one lies within the window.
  nearest(series: TimedValue[] | null, time: number): number | null {
    if (!series) return null;
    let best: TimedValue | null = null;
    for (const v of series) if (!best || Math.abs(v.t - time) < Math.abs(best.t - time)) best = v;
    if (!best || Math.abs(best.t - time) > MATCH_WINDOW) return null;
    return best.value;
  }
  // True if the sample at time t was taken with a warm engine in closed loop.
  closedLoopWarm(time: number): boolean {
    const status = this.nearest(this.get(PID_FUEL_STATUS), time);
    if (status !== null && Math.trunc(status) !== 2) return false;
    const coolant = this.nearest(this.get(PID_COOLANT), time);
    if (coolant !== null && coolant < WARM_FOR_TRIMS) return false;
    const rpm = this.nearest(this.get(PID_RPM), time);
    if (rpm !== null && rpm < RUNNING_RPM) return false;
    return true;
  }
  hasIncompleteReadiness(): boolean {
    return this.input.readiness.some((m) => m.supported && !m.complete);
  }
}

const latest = (s: TimedValue[]) => s.reduce((best, v) => (v.t >= best.t ? v : best), s[0]!).value;
const maxOf = (s: TimedValue[]) => Math.max(...s.map((v) => v.value));

function checkFaults(c: Ctx): void {
  c.ran();
  if (c.input.faults.length === 0) {
    c.add(
      "faults",
      GOOD,
      format("Keine Fehler gespeichert", "No fault codes stored"),
      format(
        "Das Motorsteuergerät meldet keinen gespeicherten, vorgemerkten oder dauerhaften Fehler.",
        "The engine control unit reports no stored, pending or permanent fault.",
      ),
    );
    return;
  }
  for (const f of c.input.faults) {
    // The database knows how urgent a code is; pending codes count one step
    // lower, they may still disappear on their own (Worker::runHealthCheck).
    const severity = lookupDtc(f.code).severity;
    let level: Level = INFO;
    if (severity === 4) level = CRITICAL;
    else if (severity === 3 || severity === 2) level = WARNING;
    if (f.kind !== "stored" && level === WARNING) level = INFO;
    const meaning =
      f.kind === "permanent"
        ? format(
            "Dieser Fehler ist dauerhaft gespeichert. Er lässt sich nicht löschen und verschwindet erst, wenn das Steuergerät selbst festgestellt hat, dass alles wieder in Ordnung ist.",
            "This fault is stored as permanent. It cannot be cleared and only goes away once the control unit has confirmed by itself that everything is fine again.",
          )
        : f.kind === "pending"
          ? format(
              "Dieser Fehler ist erst vorgemerkt: Das Steuergerät hat ihn einmal bemerkt, aber noch nicht bestätigt. Taucht er bei den nächsten Fahrten wieder auf, wird er fest gespeichert.",
              "This fault is only pending: the control unit noticed it once but has not confirmed it yet. If it shows up again on the next trips, it gets stored for good.",
            )
          : format("Dieser Fehler ist fest gespeichert.", "This fault is stored.");
    const advice =
      level === CRITICAL
        ? format(
            "Bald in die Werkstatt, und bis dahin möglichst wenig und schonend fahren.",
            "Have it seen by a garage soon, and until then drive as little and as gently as possible.",
          )
        : level === WARNING
          ? format(
              "Bei Gelegenheit prüfen lassen. In der Fehlerliste stehen typische Ursachen.",
              "Have it checked when convenient. The fault list shows typical causes.",
            )
          : format(
              "Im Auge behalten. Kommt er nach dem Löschen wieder, genauer nachsehen.",
              "Keep an eye on it. If it comes back after clearing, look into it more closely.",
            );
    c.add(`fault:${f.code}`, level, same(f.text ? `${f.code} ${f.text}` : f.code), meaning, advice);
  }
}

function checkMil(c: Ctx): void {
  c.ran();
  if (c.input.milOn) {
    c.add(
      "mil",
      WARNING,
      format("Motorkontrollleuchte an", "Check engine lamp on"),
      format(
        "Das Steuergerät hat die gelbe Motorkontrollleuchte eingeschaltet. Es hat also einen Fehler erkannt, der die Abgaswerte verschlechtern kann. Mit leuchtender Lampe fällt ein Auto in vielen Ländern durch die Abgasuntersuchung.",
        "The control unit has switched on the yellow check engine lamp. It has found a fault that can make the emissions worse. With the lamp on, a car fails the emissions test in many countries.",
      ),
      format(
        "Die Fehlerliste zeigt den Grund. Blinkt die Lampe während der Fahrt, sofort Gas wegnehmen und bald anhalten: Das bedeutet starke Aussetzer, die den Katalysator zerstören können.",
        "The fault list shows the reason. If the lamp flashes while driving, ease off at once and stop soon: that means heavy misfires that can destroy the catalytic converter.",
      ),
    );
  } else {
    c.add(
      "mil",
      GOOD,
      format("Motorkontrollleuchte aus", "Check engine lamp off"),
      format("Das Steuergerät meldet die Warnlampe als aus.", "The control unit reports the warning lamp as off."),
    );
  }
}

function checkReadiness(c: Ctx): void {
  c.ran();
  const supported = c.input.readiness.filter((m) => m.supported);
  const open = supported.filter((m) => !m.complete).map((m) => m.monitor);
  const diesel = c.input.diesel;
  if (open.length === 0) {
    c.add(
      "readiness",
      GOOD,
      format("Alle Bereitschaftstests fertig", "All readiness monitors complete"),
      format(
        "{} von {} Selbsttests sind abgeschlossen. Das ist die beste Voraussetzung für die Abgasuntersuchung.",
        "{} of {} self-tests are complete. That is the best starting point for the emissions test.",
        [number(supported.length, 0), number(supported.length, 0)],
      ),
    );
    return;
  }
  const list: Phrase = {
    de: open.map((m) => MONITOR_NAMES[m].de).join(", "),
    en: open.map((m) => MONITOR_NAMES[m].en).join(", "),
  };
  const level: Level = diesel ? WARNING : open.length === 1 ? INFO : WARNING;
  const title =
    open.length === 1
      ? format("1 Bereitschaftstest noch offen", "1 readiness monitor not complete")
      : format("{} Bereitschaftstests noch offen", "{} readiness monitors not complete", [number(open.length, 0)]);
  const explanation = format(
    "Noch nicht fertig: {}. Das Auto hat diese Selbsttests seit dem letzten Löschen des Fehlerspeichers (oder seit die Batterie abgeklemmt war) noch nicht zu Ende gebracht. Das ist an sich kein Defekt, solange keine Fehler gespeichert sind.",
    "Not complete yet: {}. The car has not finished these self-tests since the fault memory was last cleared (or since the battery was disconnected). That alone is not a defect, as long as no faults are stored.",
    [list],
  );
  const rule = diesel
    ? format(
        "Für die Abgasuntersuchung sollten beim Diesel möglichst alle Tests fertig sein; die genauen Regeln hängen vom Land und der Prüfstelle ab.",
        "For the emissions test, a diesel should have all monitors complete if possible; the exact rules depend on the country and the testing station.",
      )
    : format(
        "Als Faustregel für die Abgasuntersuchung: Beim Benziner wird oft ein offener Test hingenommen (so regelt es zum Beispiel die US-Umweltbehörde EPA für Autos ab Baujahr 2001), mehrere offene Tests meist nicht. Die genauen Regeln hängen vom Land und der Prüfstelle ab.",
        "As a rule of thumb for the emissions test: a petrol car is often accepted with one incomplete monitor (that is the US EPA rule for model year 2001 and later), with several usually not. The exact rules depend on the country and the testing station.",
      );
  const advice = plus(
    rule,
    format(
      "Am sichersten ist es, vorher die passenden Fahrten zu machen (siehe Fahrzyklus-Hilfe) und danach erneut zu prüfen.",
      "The safest way is to do the matching drives first (see the drive cycle help) and check again afterwards.",
    ),
  );
  c.add("readiness", level, title, explanation, advice);
}

function checkHistory(c: Ctx): void {
  const warmups = c.get(PID_WARMUPS);
  const distance = c.get(PID_CLEAR_DISTANCE);
  if (!warmups && !distance) {
    c.skip(
      "cleared_recently",
      format("Zuletzt gelöscht", "Last cleared"),
      format(
        "Warmläufe und Strecke seit dem Löschen (PID 30 und 31) fehlen.",
        "Warm-ups and distance since clearing (PID 30 and 31) are missing.",
      ),
    );
  } else {
    c.ran();
    const w = warmups ? latest(warmups) : null;
    const d = distance ? latest(distance) : null;
    const recent = (w !== null && w < limits.fewWarmups) || (d !== null && d < limits.shortDistanceKm);
    const facts =
      w !== null && d !== null
        ? format("Seit dem letzten Löschen: {} Warmläufe und {} km.", "Since the last clearing: {} warm-ups and {} km.", [
            number(w, 0),
            number(d, 0),
          ])
        : w !== null
          ? format("Seit dem letzten Löschen: {} Warmläufe.", "Since the last clearing: {} warm-ups.", [number(w, 0)])
          : format("Seit dem letzten Löschen: {} km.", "Since the last clearing: {} km.", [number(d ?? 0, 0)]);
    if (recent) {
      let explanation = plus(
        facts,
        format(
          "Das ist sehr wenig. Mit dem Löschen werden auch die Bereitschaftstests zurückgesetzt, und Fehler, die erst nach einiger Zeit wiederkommen, sind noch nicht zu sehen. Beim Gebrauchtwagenkauf kann das ein Hinweis sein, dass jemand Fehler kurz vor der Besichtigung gelöscht hat. Es gibt aber auch harmlose Gründe, etwa eine Reparatur oder eine abgeklemmte Batterie.",
          "That is very little. Clearing also resets the readiness monitors, and faults that only come back after a while are not visible yet. When buying a used car, this can be a hint that someone cleared faults shortly before the viewing. There are harmless reasons too, such as a repair or a disconnected battery.",
        ),
      );
      if (c.hasIncompleteReadiness()) {
        explanation = plus(
          explanation,
          format("Dazu passen die noch offenen Bereitschaftstests.", "The incomplete readiness monitors fit that picture."),
        );
      }
      c.add(
        "cleared_recently",
        WARNING,
        format("Fehlerspeicher vor Kurzem gelöscht", "Fault memory cleared recently"),
        explanation,
        format(
          "Beim Kauf nachfragen, warum gelöscht wurde. Am besten nach einer Probefahrt von 20 bis 30 Minuten oder nach ein paar Tagen Fahrt noch einmal prüfen.",
          "When buying, ask why it was cleared. Ideally check again after a test drive of 20 to 30 minutes or after a few days of driving.",
        ),
      );
    } else {
      c.add(
        "cleared_recently",
        GOOD,
        format("Nicht frisch gelöscht", "Not freshly cleared"),
        plus(facts, format("Kein Hinweis auf ein Löschen kurz vor der Prüfung.", "No sign of clearing shortly before the check.")),
      );
    }
  }
  const mil = c.get(PID_MIL_DISTANCE);
  if (mil) {
    c.ran();
    const km = latest(mil);
    if (km > 0) {
      c.add(
        "mil_distance",
        INFO,
        format("Warnlampe war {} km lang an", "Warning lamp was on for {} km", [number(km, 0)]),
        format(
          "Seit dem letzten Löschen ist das Auto {} km mit leuchtender Motorkontrollleuchte gefahren.",
          "Since the last clearing the car has been driven {} km with the check engine lamp on.",
          [number(km, 0)],
        ),
        format(
          "Je länger ein Fehler ignoriert wird, desto eher entstehen Folgeschäden. Den Grund in der Fehlerliste nachsehen.",
          "The longer a fault is ignored, the more likely it causes further damage. Look up the reason in the fault list.",
        ),
      );
    }
  }
}

// Bank 1 only: the demo engine is an inline four, bank 2 does not exist.
function checkTrims(c: Ctx): void {
  const st = c.get(0x06);
  const lt = c.get(0x07);
  const id = "fuel_trim_bank1";
  const bankName = format("Bank {}", "bank {}", [number(1, 0)]);
  const checkTitle = format("Gemischregelung ({})", "Mixture control ({})", [bankName]);
  if (!st && !lt) {
    c.skip(
      id,
      checkTitle,
      c.input.diesel
        ? format("Diesel melden meist keine Gemischkorrektur (PID 06 bis 09).", "Diesels usually report no fuel trims (PID 06 to 09).")
        : format("Die Gemischkorrektur (PID 06 und 07) wurde nicht gelesen.", "The fuel trims (PID 06 and 07) were not read."),
    );
    return;
  }
  const base = (st ?? lt)!;
  const other = st ? lt : null;
  const rpm = c.get(PID_RPM);
  const all: number[] = [];
  const idle: number[] = [];
  const cruise: number[] = [];
  for (const s of base) {
    if (!c.closedLoopWarm(s.t)) continue;
    let total = s.value;
    if (other) {
      const o = c.nearest(other, s.t);
      if (o === null) continue;
      total += o;
    }
    all.push(total);
    const r = c.nearest(rpm, s.t);
    if (r !== null) {
      if (r < 1100) idle.push(total);
      else if (r >= 1800) cruise.push(total);
    }
  }
  if (all.length === 0) {
    c.skip(
      id,
      checkTitle,
      format(
        "Keine Werte bei warmem Motor im geschlossenen Regelkreis. Die Korrektur sagt erst etwas aus, wenn der Motor warm ist und die Lambdaregelung arbeitet.",
        "No readings with a warm engine in closed loop. The trims only mean something once the engine is warm and the oxygen control is active.",
      ),
    );
    return;
  }
  c.ran();
  const avg = mean(all);
  const magnitude = Math.abs(avg);
  const value = signedNumber(avg, 1);
  const partial =
    st && lt
      ? { de: "", en: "" }
      : format(
          "(Nur ein Teil der Korrektur wurde gelesen, das Ergebnis ist deshalb ungenauer.)",
          "(Only part of the correction was read, so the result is less precise.)",
        );
  if (magnitude < limits.trimWarn) {
    c.add(
      id,
      GOOD,
      format("Gemischregelung in Ordnung ({})", "Mixture control fine ({})", [bankName]),
      plus(
        format(
          "Die Korrektur liegt im Schnitt bei {} %, also im normalen Bereich von plus/minus 10 %.",
          "The correction averages {}%, well within the normal range of plus/minus 10%.",
          [value],
        ),
        partial,
      ),
    );
    return;
  }
  const level: Level = magnitude >= limits.trimBad ? CRITICAL : WARNING;
  // Comparing idle and higher engine speed narrows the cause down: an air
  // leak matters most at idle, where little air flows anyway.
  let pattern: Phrase = { de: "", en: "" };
  if (avg > 0 && idle.length >= 3 && cruise.length >= 3) {
    const atIdle = mean(idle);
    const atCruise = mean(cruise);
    if (atIdle >= limits.trimWarn && atCruise < 5.0) {
      pattern = format(
        "Im Leerlauf ist die Korrektur hoch ({} %), bei höherer Drehzahl fast normal ({} %). Das spricht stark für Falschluft.",
        "At idle the correction is high ({}%), at higher engine speed almost normal ({}%). That points strongly to an air leak.",
        [signedNumber(atIdle, 1), signedNumber(atCruise, 1)],
      );
    } else if (atIdle >= limits.trimWarn && atCruise >= limits.trimWarn) {
      pattern = format(
        "Die Korrektur ist im Leerlauf und bei höherer Drehzahl hoch. Das spricht eher für den Luftmassenmesser oder den Kraftstoffdruck als für Falschluft.",
        "The correction is high both at idle and at higher engine speed. That points more to the mass air flow sensor or the fuel pressure than to an air leak.",
      );
    }
  }
  if (avg > 0) {
    c.add(
      id,
      level,
      format("Gemisch zu mager ({})", "Mixture too lean ({})", [bankName]),
      plus(
        format(
          "Das Steuergerät gibt im Schnitt {} % mehr Kraftstoff dazu, als es berechnet hat. Es kommt also mehr Luft in den Motor, als gemessen wird, oder zu wenig Kraftstoff an. Häufige Gründe: Falschluft durch einen undichten Schlauch oder eine Dichtung hinter dem Luftmassenmesser, ein verschmutzter Luftmassenmesser oder zu wenig Kraftstoffdruck.",
          "The control unit adds on average {}% more fuel than it calculated. So either more air gets into the engine than is measured, or too little fuel arrives. Common reasons: an air leak through a cracked hose or gasket after the mass air flow sensor, a dirty mass air flow sensor or low fuel pressure.",
          [value],
        ),
        pattern,
        partial,
      ),
      format(
        "Im Leerlauf auf Zischen achten und Unterdruckschläuche ansehen. Eine Werkstatt findet Falschluft schnell mit einem Rauchtest.",
        "Listen for hissing at idle and look at the vacuum hoses. A garage finds air leaks quickly with a smoke test.",
      ),
    );
  } else {
    c.add(
      id,
      level,
      format("Gemisch zu fett ({})", "Mixture too rich ({})", [bankName]),
      plus(
        format(
          "Das Steuergerät nimmt im Schnitt {} % Kraftstoff weg. Es kommt also zu viel Kraftstoff oder zu wenig Luft an. Typische Gründe: ein undichtes Einspritzventil, zu hoher Kraftstoffdruck, ein hängendes Tankentlüftungsventil, ein falsch messender Luftmassenmesser oder Kraftstoff im Motoröl.",
          "The control unit takes away on average {}% fuel. So too much fuel or too little air arrives. Typical reasons: a leaking injector, too high fuel pressure, a stuck purge valve, a mass air flow sensor that reads wrong, or fuel in the engine oil.",
          [number(magnitude, 1)],
        ),
        partial,
      ),
      format(
        "Bald prüfen lassen, bevor der Katalysator Schaden nimmt. Ölstand ansehen: Riecht das Öl nach Kraftstoff oder steigt der Stand, ist Kraftstoff im Öl.",
        "Have it checked soon, before the catalytic converter suffers. Look at the oil: if it smells of fuel or the level rises, there is fuel in the oil.",
      ),
    );
  }
}

interface Switching {
  samples: number;
  duration: number;
  switches: number;
  low: number;
  high: number;
}

// Counts rich/lean changes of a narrowband sensor. A hysteresis around the
// 0.45 V midpoint keeps noise from counting as switching.
function analyseSwitching(c: Ctx, s: TimedValue[]): Switching {
  const out: Switching = { samples: 0, duration: 0, switches: 0, low: 0, high: 0 };
  let state = 0;
  let first = 0;
  let last = 0;
  for (const v of s) {
    if (!c.closedLoopWarm(v.t)) continue;
    if (out.samples === 0) {
      first = last = v.t;
      out.low = out.high = v.value;
    }
    out.samples++;
    first = Math.min(first, v.t);
    last = Math.max(last, v.t);
    out.low = Math.min(out.low, v.value);
    out.high = Math.max(out.high, v.value);
    const now = v.value < 0.3 ? -1 : v.value > 0.6 ? 1 : 0;
    if (now !== 0) {
      if (state !== 0 && now !== state) out.switches++;
      state = now;
    }
  }
  out.duration = last - first;
  return out;
}

function checkOxygenSensors(c: Ctx): void {
  if (c.input.diesel) return;
  const up = c.get(0x14);
  if (!up) {
    c.skip(
      "o2_switching",
      format("Lambdasonden", "Oxygen sensors"),
      format(
        "Keine Spannungen der Lambdasonden (PID 14 bis 1B). Autos mit Breitbandsonden melden sie anders, dann ist diese Prüfung hier nicht möglich.",
        "No oxygen sensor voltages (PID 14 to 1B). Cars with wideband sensors report them differently, and then this check is not possible here.",
      ),
    );
    return;
  }
  const bankName = format("Bank {}", "bank {}", [number(1, 0)]);
  const sw = analyseSwitching(c, up);
  if (sw.samples < 10 || sw.duration < 10) {
    c.skip(
      "o2_switching_bank1",
      format("Lambdasonde vorn ({})", "Front oxygen sensor ({})", [bankName]),
      format(
        "Zu wenige Messwerte bei warmem Motor im geschlossenen Regelkreis. Nötig sind mindestens 10 Werte über 10 Sekunden.",
        "Too few readings with a warm engine in closed loop. At least 10 readings over 10 seconds are needed.",
      ),
    );
    return;
  }
  c.ran();
  const range = format("{} und {} V", "{} and {} V", [number(sw.low, 2), number(sw.high, 2)]);
  if (sw.switches === 0) {
    const cause =
      sw.high < 0.3
        ? format(
            "Sie zeigt dauernd mager. Das kann die Sonde selbst sein, aber auch Falschluft oder ein Abgasleck vor der Sonde.",
            "It reads lean all the time. That can be the sensor itself, but also an air leak or an exhaust leak in front of the sensor.",
          )
        : sw.low > 0.6
          ? format(
              "Sie zeigt dauernd fett. Das kann die Sonde selbst sein oder wirklich zu viel Kraftstoff.",
              "It reads rich all the time. That can be the sensor itself or genuinely too much fuel.",
            )
          : format(
              "Die Spannung steht fast still. Typisch für eine gealterte oder verrußte Sonde.",
              "The voltage barely moves. Typical of an aged or sooted sensor.",
            );
    c.add(
      "o2_switching_bank1",
      WARNING,
      format("Lambdasonde vorn regelt nicht ({})", "Front oxygen sensor not switching ({})", [bankName]),
      plus(
        format(
          "Bei warmem Motor sollte die Spannung der Sonde vor dem Katalysator ständig zwischen etwa 0,1 und 0,9 V hin und her springen. Hier lag sie {} Sekunden lang nur zwischen {}.",
          "With a warm engine the voltage of the sensor in front of the catalytic converter should keep jumping between about 0.1 and 0.9 V. Here, for {} seconds, it only stayed between {}.",
          [number(sw.duration, 0), range],
        ),
        cause,
      ),
      format("Sonde, Stecker und Kabel prüfen lassen.", "Have the sensor, its plug and wire checked."),
    );
    return;
  }
  const rate = sw.switches / sw.duration;
  if (rate < 0.1 && sw.duration >= 30) {
    c.add(
      "o2_switching_bank1",
      INFO,
      format("Lambdasonde vorn eher träge ({})", "Front oxygen sensor rather slow ({})", [bankName]),
      format(
        "Die Sonde hat in {} Sekunden nur {}-mal gewechselt. Eine frische Sonde wechselt bei warmem Motor etwa einmal pro Sekunde. Allerdings fragt der Adapter jeden Wert nur ein paar Mal pro Sekunde ab und kann schnelle Wechsel verpassen.",
        "In {} seconds the sensor switched only {} times. A fresh sensor switches about once per second with a warm engine. However, the adapter polls each value only a few times per second and can miss fast changes.",
        [number(sw.duration, 0), number(sw.switches, 0)],
      ),
      format(
        "Bei 2000 bis 2500/min noch einmal messen. Bleibt sie träge, ist sie wahrscheinlich gealtert.",
        "Measure again at 2000 to 2500 rpm. If it stays slow, it has probably aged.",
      ),
    );
  } else {
    c.add(
      "o2_switching_bank1",
      GOOD,
      format("Lambdasonde vorn regelt ({})", "Front oxygen sensor switching ({})", [bankName]),
      format("{} Wechsel in {} Sekunden, Spannung zwischen {}.", "{} switches in {} seconds, voltage between {}.", [
        number(sw.switches, 0),
        number(sw.duration, 0),
        range,
      ]),
    );
  }
  // Catalyst: a good converter stores oxygen and smooths the signal behind
  // it. If the rear sensor copies the front one, it no longer does that.
  const down = c.get(0x15);
  if (!down || sw.switches < 6) return;
  const rear = analyseSwitching(c, down);
  if (rear.samples < 10) return;
  c.ran();
  const ratio = rear.switches / sw.switches;
  const caveat = format(
    "Das ist eine Schätzung aus den Live-Werten. Die genaue Prüfung macht das Steuergerät selbst (Bereitschaftstest Katalysator und Modus 06).",
    "This is an estimate from live values. The control unit does the exact check itself (catalyst readiness monitor and mode 06).",
  );
  if (ratio >= 0.7) {
    c.add(
      "catalyst_bank1",
      WARNING,
      format("Katalysator speichert wenig Sauerstoff ({})", "Catalytic converter stores little oxygen ({})", [bankName]),
      plus(
        format(
          "Die Sonde hinter dem Katalysator wechselt fast so oft wie die davor ({} zu {} Wechsel). Ein guter Katalysator glättet das Signal dahinter. Das deutet auf einen gealterten Katalysator hin.",
          "The sensor behind the converter switches almost as often as the one in front ({} to {} switches). A good converter smooths the signal behind it. That points to an aged converter.",
          [number(rear.switches, 0), number(sw.switches, 0)],
        ),
        caveat,
      ),
      format(
        "Auf den Fehler P0420/P0430 achten und vor einem Tausch die hintere Sonde und Abgaslecks prüfen lassen.",
        "Watch for fault P0420/P0430, and before replacing anything have the rear sensor and exhaust leaks checked.",
      ),
    );
  } else {
    c.add(
      "catalyst_bank1",
      GOOD,
      format("Katalysator arbeitet ({})", "Catalytic converter working ({})", [bankName]),
      plus(
        format(
          "Die Sonde hinter dem Katalysator bleibt ruhig ({} Wechsel gegen {} vorn). So soll es sein.",
          "The sensor behind the converter stays calm ({} switches against {} in front). That is how it should be.",
          [number(rear.switches, 0), number(sw.switches, 0)],
        ),
        caveat,
      ),
    );
  }
}

// The demo battery and alternator are healthy, so only the reachable
// verdicts of checkVoltage are here: charging fine or low, resting voltage.
function checkVoltage(c: Ctx): void {
  const chargeTitle = format("Ladespannung", "Charging voltage");
  const restTitle = format("Batterie bei stehendem Motor", "Battery with engine off");
  const volts = c.get(PID_VOLTAGE);
  const rpm = c.get(PID_RPM);
  if (!volts || !rpm) {
    const why = !volts
      ? format("Die Spannung (PID 42) wurde nicht gelesen.", "The voltage (PID 42) was not read.")
      : format("Ohne Drehzahl ist nicht klar, ob der Motor lief.", "Without engine speed it is unclear whether the engine ran.");
    c.skip("charging", chargeTitle, why);
    c.skip("battery_rest", restTitle, why);
    return;
  }
  const running: number[] = [];
  const off: number[] = [];
  for (const v of volts) {
    const r = c.nearest(rpm, v.t);
    if (r === null) continue;
    if (r > RUNNING_RPM) running.push(v.value);
    else if (r < STOPPED_RPM) off.push(v.value);
  }
  const sourceNote = format(
    "Gemessen am Diagnosestecker; dort liegt die Spannung oft 0,1 bis 0,3 V unter der an den Batteriepolen.",
    "Measured at the diagnostic socket, where the voltage is often 0.1 to 0.3 V below the battery terminals.",
  );
  if (running.length === 0) {
    c.skip("charging", chargeTitle, format("Der Motor lief während der Messung nicht.", "The engine was not running during the measurement."));
  } else {
    c.ran();
    const v = median(running);
    const value = number(v, 2);
    const smart = format(
      "Autos mit Start-Stopp und intelligentem Lademanagement (oft mit AGM- oder EFB-Batterie) senken die Ladespannung bei voller Batterie bewusst ab; dort kann ein Wert um 12,5 bis 13 V normal sein.",
      "Cars with start-stop and smart charging (often with an AGM or EFB battery) lower the charging voltage on purpose when the battery is full; there a value of about 12.5 to 13 V can be normal.",
    );
    if (v < limits.chargingLow) {
      c.add(
        "charging",
        WARNING,
        format("Ladespannung niedrig", "Charging voltage low"),
        plus(
          format(
            "Bei laufendem Motor liegen {} V an. Normal sind 13,8 bis 14,8 V; unter 13,2 V lädt die Lichtmaschine meist zu schwach.",
            "With the engine running there are {} V. Normal is 13.8 to 14.8 V; below 13.2 V the alternator usually charges too weakly.",
            [value],
          ),
          smart,
          sourceNote,
        ),
        format(
          "Mit dem geführten Batterietest nachmessen, auch mit eingeschaltetem Licht und Gebläse.",
          "Measure again with the guided battery test, also with the lights and blower switched on.",
        ),
      );
    } else {
      c.add(
        "charging",
        GOOD,
        format("Lichtmaschine lädt", "Alternator is charging"),
        format("Bei laufendem Motor liegen {} V an, das ist im normalen Bereich.", "With the engine running there are {} V, which is normal.", [value]),
      );
    }
  }
  if (off.length === 0) {
    c.skip(
      "battery_rest",
      restTitle,
      format(
        "Der Motor lief die ganze Zeit. Die Ruhespannung misst der geführte Batterietest.",
        "The engine ran the whole time. The guided battery test measures the resting voltage.",
      ),
    );
  } else {
    c.ran();
    const loadNote = format(
      "Bei eingeschalteter Zündung ziehen Steuergeräte und Pumpen Strom, der Wert liegt deshalb etwas unter der echten Ruhespannung.",
      "With the ignition on, control units and pumps draw current, so the value is a little below the true resting voltage.",
    );
    c.add(
      "battery_rest",
      GOOD,
      format("Batteriespannung in Ordnung", "Battery voltage fine"),
      plus(format("Bei stehendem Motor liegen {} V an.", "With the engine off there are {} V.", [number(median(off), 2)]), loadNote),
    );
  }
}

function engineRunTime(c: Ctx): number | null {
  const rt = c.get(PID_RUN_TIME);
  if (rt) return latest(rt);
  const rpm = c.get(PID_RPM);
  if (!rpm) return null;
  const running = rpm.filter((v) => v.value > RUNNING_RPM).map((v) => v.t);
  return running.length > 0 ? Math.max(...running) - Math.min(...running) : null;
}

// The demo engine warms up normally and never overheats, so the thermostat
// verdicts and the "not long enough" note are what it can reach.
function checkCooling(c: Ctx): void {
  const thermoTitle = format("Thermostat und Betriebstemperatur", "Thermostat and operating temperature");
  const coolant = c.get(PID_COOLANT);
  if (!coolant) {
    c.skip("thermostat", thermoTitle, format("Die Kühlmitteltemperatur (PID 05) wurde nicht gelesen.", "The coolant temperature (PID 05) was not read."));
    return;
  }
  c.ran();
  const hottest = maxOf(coolant);
  const runTime = engineRunTime(c);
  const hottestText = number(hottest, 0);
  const reached = () =>
    c.add(
      "thermostat",
      GOOD,
      format("Betriebstemperatur erreicht", "Operating temperature reached"),
      format("Das Kühlmittel hat {} °C erreicht. Das Thermostat arbeitet.", "The coolant reached {} °C. The thermostat works.", [hottestText]),
    );
  if (runTime !== null && runTime >= limits.longRunSeconds) {
    const minutes = number(runTime / 60, 0);
    if (hottest < limits.warmCoolant) {
      c.add(
        "thermostat",
        INFO,
        format("Motor bleibt eher kühl", "Engine stays rather cool"),
        format(
          "Nach {} Minuten Laufzeit hat das Kühlmittel {} °C erreicht. Normal sind 80 bis 105 °C. Bei kaltem Wetter und ruhiger Fahrt kann das noch in Ordnung sein.",
          "After {} minutes of running the coolant reached {} °C. Normal is 80 to 105 °C. In cold weather and with gentle driving this can still be fine.",
          [minutes, hottestText],
        ),
        format("Bei der nächsten längeren Fahrt noch einmal ansehen.", "Look at it again on the next longer drive."),
      );
    } else {
      reached();
    }
  } else if (hottest >= limits.warmCoolant) {
    reached();
  } else {
    c.skip(
      "thermostat",
      thermoTitle,
      runTime !== null
        ? format(
            "Der Motor lief erst {} Minuten. Für eine Aussage braucht es etwa 15 Minuten Fahrt.",
            "The engine had only run for {} minutes. A verdict needs about 15 minutes of driving.",
            [number(runTime / 60, 0)],
          )
        : format(
            "Die Laufzeit des Motors ist unbekannt (PID 1F oder Drehzahl fehlt), und der Motor war noch nicht warm.",
            "The engine run time is unknown (PID 1F or engine speed missing), and the engine was not warm yet.",
          ),
    );
  }
}

// Intake, oil and catalyst temperatures. A sensor stuck at -40 °C (a broken
// wire) cannot happen on the demo car, so that branch is left out.
function checkOtherTemperatures(c: Ctx): void {
  if (c.get(PID_INTAKE)) c.ran();
  const oil = c.get(PID_OIL);
  if (oil) {
    c.ran();
    if (maxOf(oil) >= limits.oilHot) {
      const hottest = maxOf(oil);
      c.add(
        "oil_temp",
        hottest >= limits.oilOverheat ? CRITICAL : WARNING,
        format("Öl sehr heiß", "Oil very hot"),
        format(
          "Das Motoröl hat {} °C erreicht. Normal sind etwa 90 bis 120 °C, bei sportlicher Fahrt auch kurz mehr. Ab etwa 150 °C altert das Öl sehr schnell und schmiert schlechter.",
          "The engine oil reached {} °C. Normal is about 90 to 120 °C, briefly more when driven hard. From about 150 °C the oil ages very quickly and lubricates less well.",
          [number(hottest, 0)],
        ),
        format(
          "Ölstand prüfen und den Motor schonen. Bleibt es so, Kühlung prüfen lassen.",
          "Check the oil level and go easy on the engine. If it stays like this, have the cooling checked.",
        ),
      );
    }
  }
  const cat = c.get(PID_CATALYST);
  if (cat) {
    c.ran();
    if (maxOf(cat) >= limits.catalystHot) {
      c.add(
        "catalyst_temp",
        WARNING,
        format("Katalysator sehr heiß", "Catalytic converter very hot"),
        format(
          "Das Steuergerät meldet {} °C am Katalysator. Ab etwa 900 °C kann er auf Dauer Schaden nehmen. Häufigster Grund sind Zündaussetzer, bei denen unverbrannter Kraftstoff im Katalysator nachbrennt.",
          "The control unit reports {} °C at the catalytic converter. From about 900 °C it can be damaged over time. The most common reason is misfiring, where unburnt fuel burns in the converter.",
          [number(maxOf(cat), 0)],
        ),
        format(
          "Auf Aussetzer und Fehlercodes achten und hohe Last vermeiden, bis die Ursache klar ist.",
          "Watch for misfires and fault codes and avoid heavy load until the cause is clear.",
        ),
      );
    }
  }
}

function checkMode06(c: Ctx): void {
  c.ran();
  let evaluated = 0;
  let problems = 0;
  for (const r of c.input.monitorResults) {
    const misfire = r.componentId >= 0xa1 && r.componentId <= 0xad;
    const notRunYet = r.value === 0 && r.minimum === 0 && r.maximum === 0;
    if (misfire || notRunYet) continue;
    evaluated++;
    const id = `mode06:${hex2(r.componentId)}:${hex2(r.testId)}`;
    const name = same(r.component ? (r.name ? `${r.component}: ${r.name}` : r.component) : r.name);
    const measured = format("Gemessen {} {}, erlaubt {} bis {}.", "Measured {} {}, allowed {} to {}.", [
      number(r.value, 3),
      same(r.unit),
      number(r.minimum, 3),
      number(r.maximum, 3),
    ]);
    if (!r.passed) {
      problems++;
      c.add(
        id,
        WARNING,
        format("Prüfung nicht bestanden: {}", "Test failed: {}", [name]),
        plus(
          format(
            "Das Steuergerät hat diesen Wert außerhalb seiner Grenzen gemessen. Meist folgt bald ein Fehlercode.",
            "The control unit measured this value outside its limits. A fault code usually follows soon.",
          ),
          measured,
        ),
        format(
          "Das betroffene Bauteil prüfen lassen, bevor die Warnlampe angeht.",
          "Have the component checked before the warning lamp comes on.",
        ),
      );
      continue;
    }
    // Zero limits are usually placeholders ("no lower limit"), so only real
    // limits count for the "close to the limit" check.
    const near =
      (r.maximum !== 0 && r.maximum - r.value <= limits.nearLimitShare * Math.abs(r.maximum)) ||
      (r.minimum !== 0 && r.value - r.minimum <= limits.nearLimitShare * Math.abs(r.minimum));
    if (near) {
      problems++;
      c.add(
        id,
        INFO,
        format("Knapp an der Grenze: {}", "Close to the limit: {}", [name]),
        plus(
          format(
            "Die Prüfung ist bestanden, aber der Wert liegt weniger als 10 % von seiner Grenze entfernt. So kündigen sich Fehler oft lange vor der Warnlampe an.",
            "The test passed, but the value is less than 10% away from its limit. That is how faults often show up long before the warning lamp.",
          ),
          measured,
        ),
        format(
          "Bei der nächsten Prüfung wieder ansehen. Wandert der Wert weiter zur Grenze, das Bauteil prüfen lassen.",
          "Look again at the next check. If the value keeps moving towards the limit, have the component checked.",
        ),
      );
    }
  }
  if (problems === 0) {
    c.add(
      "mode06",
      GOOD,
      format("Bauteilprüfungen im grünen Bereich", "Component tests well within limits"),
      evaluated > 0
        ? format("Alle {} Prüfungen liegen mit Abstand innerhalb ihrer Grenzen.", "All {} tests are comfortably within their limits.", [
            number(evaluated, 0),
          ])
        : format("Das Steuergerät hat noch keine Prüfung abgeschlossen.", "The control unit has not completed any test yet."),
    );
  }
}

function checkMisfire(c: Ctx): void {
  // Component A2..AD is cylinder 1..12, test 0B the average over the last ten
  // drives, test 0C the count of the current or last drive.
  const perCylinder = new Map<number, number>();
  let failed = false;
  for (const r of c.input.monitorResults) {
    if (r.componentId < 0xa2 || r.componentId > 0xad) continue;
    if (r.testId !== 0x0b && r.testId !== 0x0c) continue;
    const cyl = r.componentId - 0xa1;
    perCylinder.set(cyl, Math.max(perCylinder.get(cyl) ?? 0, r.value));
    if (!r.passed) failed = true;
  }
  if (perCylinder.size === 0) {
    c.skip(
      "misfire",
      format("Zündaussetzer je Zylinder", "Misfires per cylinder"),
      format(
        "Das Auto meldet keine Aussetzerzähler in Modus 06 (gibt es meist nur bei CAN-Fahrzeugen).",
        "The car reports no misfire counters in mode 06 (usually only on CAN vehicles).",
      ),
    );
    return;
  }
  c.ran();
  let total = 0;
  let worst = 0;
  let worstCyl = 0;
  const items: Phrase[] = [];
  for (const [cyl, count] of [...perCylinder.entries()].sort((a, b) => a[0] - b[0])) {
    total += count;
    if (count > worst) {
      worst = count;
      worstCyl = cyl;
    }
    if (count > 0) items.push(format("Zylinder {}: {}", "cylinder {}: {}", [number(cyl, 0), number(count, 0)]));
  }
  if (total <= 0 && !failed) {
    c.add(
      "misfire",
      GOOD,
      format("Keine Zündaussetzer", "No misfires"),
      format("Alle {} Zylinder zünden sauber, die Zähler stehen auf null.", "All {} cylinders fire cleanly, the counters are at zero.", [
        number(perCylinder.size, 0),
      ]),
    );
    return;
  }
  const list: Phrase = { de: items.map((i) => i.de).join(", "), en: items.map((i) => i.en).join(", ") };
  const level: Level = failed || worst >= 20 ? WARNING : INFO;
  const focus =
    total >= 10 && worst >= 0.6 * total
      ? format(
          "Fast alle Aussetzer kommen von Zylinder {}. Das spricht für ein Teil genau dieses Zylinders: Zündkerze, Zündspule oder Einspritzventil. Ein einfacher Test: Zündspule mit einem Nachbarzylinder tauschen und sehen, ob der Fehler mitwandert.",
          "Almost all misfires come from cylinder {}. That points to a part of exactly this cylinder: spark plug, ignition coil or injector. A simple test: swap the coil with a neighbouring cylinder and see whether the fault moves with it.",
          [number(worstCyl, 0)],
        )
      : { de: "", en: "" };
  c.add(
    "misfire",
    level,
    level === WARNING ? format("Zündaussetzer erkannt", "Misfires detected") : format("Vereinzelte Zündaussetzer", "Occasional misfires"),
    plus(
      format(
        "Gezählte Aussetzer: {}. Einzelne Aussetzer kommen vor, etwa beim Kaltstart oder bei schlechtem Kraftstoff. Viele davon schütteln den Motor und schaden dem Katalysator. Ab wann das Steuergerät einen Fehler setzt, legt jeder Hersteller selbst fest.",
        "Misfires counted: {}. A few misfires happen, for example on a cold start or with poor fuel. Many of them shake the engine and harm the catalytic converter. Each manufacturer decides itself when a fault is set.",
        [list],
      ),
      focus,
    ),
    level === WARNING
      ? format("Zündkerzen und Zündspulen prüfen lassen.", "Have the spark plugs and ignition coils checked.")
      : format("Im Auge behalten und nach einigen Fahrten wieder prüfen.", "Keep an eye on it and check again after a few drives."),
  );
}

function runAnalysis(input: HealthInput): HealthRow {
  const c = new Ctx(input);
  checkFaults(c);
  checkMil(c);
  checkReadiness(c);
  checkHistory(c);
  checkTrims(c);
  checkOxygenSensors(c);
  checkVoltage(c);
  checkCooling(c);
  checkOtherTemperatures(c);
  checkMode06(c);
  checkMisfire(c);
  // Array.prototype.sort is stable, like std::stable_sort.
  const findings = [...c.findings].sort((a, b) => b.level - a.level);

  // score() of HealthCheck.cpp.
  let penalty = 0;
  let worst: Level = GOOD;
  let warnings = 0;
  let criticals = 0;
  for (const f of findings) {
    if (f.level > worst) worst = f.level;
    if (f.level === INFO) penalty += 3;
    else if (f.level === WARNING) {
      penalty += 12;
      warnings++;
    } else if (f.level === CRITICAL) {
      penalty += 30;
      criticals++;
    }
  }
  let score = Math.max(0, Math.min(100, 100 - penalty));
  // A single serious finding must not hide behind many good ones.
  if (worst === CRITICAL) score = Math.min(score, 49);
  else if (worst === WARNING) score = Math.min(score, 84);
  const grade: 0 | 1 | 2 = worst === CRITICAL ? 2 : worst === WARNING ? 1 : 0;
  let summary: Phrase;
  if (grade === 0) {
    summary = format("Alles im grünen Bereich. {} Prüfungen gemacht, nichts, was Sorgen machen müsste.", "All good. {} checks done, nothing to worry about.", [
      number(c.checksRun, 0),
    ]);
  } else if (grade === 1) {
    summary =
      warnings === 1
        ? format("Ein Punkt sollte bald angeschaut werden. Er ist nicht akut gefährlich.", "One point should be looked at soon. It is not an immediate danger.")
        : format("{} Punkte sollten bald angeschaut werden. Nichts davon ist akut gefährlich.", "{} points should be looked at soon. None of them is an immediate danger.", [
            number(warnings, 0),
          ]);
  } else {
    summary =
      criticals === 1
        ? format("Ein Punkt braucht bald Aufmerksamkeit, bitte zuerst ansehen.", "One point needs attention soon, please look at it first.")
        : format("{} Punkte brauchen bald Aufmerksamkeit, bitte zuerst ansehen.", "{} points need attention soon, please look at them first.", [
            number(criticals, 0),
          ]);
  }
  if (c.notChecked.length > 0) {
    summary = plus(
      summary,
      format("{} Prüfungen waren mangels Daten nicht möglich.", "{} checks were not possible for lack of data.", [number(c.notChecked.length, 0)]),
    );
  }

  const monitors: MonitorGuideRow[] = [];
  for (const m of input.readiness) {
    if (!m.supported) continue;
    const guide = GUIDES[m.monitor];
    monitors.push({
      name: text(MONITOR_NAMES[m.monitor]),
      complete: m.complete,
      explanation: guide ? text(guide.explanation) : "",
      drivePattern: guide ? text(guide.drivePattern) : "",
      typicalTime: guide ? text(guide.typicalTime) : "",
      coldStart: guide?.coldStart ?? false,
    });
  }

  const rows: HealthFindingRow[] = findings.map((f) => ({
    id: f.id,
    level: f.level,
    area: "",
    title: text(f.title),
    explanation: text(f.explanation),
    advice: text(f.advice),
  }));
  return {
    score,
    grade,
    summary: text(summary),
    findings: rows,
    notChecked: c.notChecked.map((s) => `${text(s.title)}: ${text(s.reason)}`),
    monitors,
    checksRun: c.checksRun,
    diesel: input.diesel,
  };
}

async function healthCheck(seconds: number): Promise<void> {
  if (!link.connected) {
    emit("failed", { error: t(W, "Für den Gesundheitscheck erst verbinden.") });
    return;
  }
  // stopLive(): the check needs the line for itself.
  if (link.liveTimer) {
    clearInterval(link.liveTimer);
    link.liveTimer = 0;
  }
  link.cancel = false;
  emit("busy", { active: true });

  emit("health_progress", { step: t(W, "Bereitschaft und Fehlerlampe"), percent: 4 });
  await wait(350);
  const status = statusRow();
  emit("status", { row: status });

  emit("health_progress", { step: t(W, "Fehlerspeicher"), percent: 10 });
  await wait(450);
  const rows = faultRows();
  emit("faults", { rows });
  const pending = car.pendingCodes();
  const faults: HealthFault[] = rows.map((r) => ({
    code: r.code,
    text: r.text,
    kind: r.stored ? "stored" : pending.includes(r.code) ? "pending" : "permanent",
  }));

  emit("health_progress", { step: t(W, "Bauteilprüfung (Modus 06)"), percent: 18 });
  await wait(600);
  const monitorResults = monitorRecords();

  // Live series: only what the car offers.
  const pids = HEALTH_LIVE_PIDS.filter((p) => SUPPORTED_PIDS.includes(p));
  const live = new Map<number, TimedValue[]>();
  const start = appSeconds();
  const total = Math.max(1, seconds);
  while (appSeconds() - start < seconds && !link.cancel) {
    const now = car.now();
    const sample = simulate(now);
    const elapsedAtRead = appSeconds() - start;
    for (const pid of pids) {
      const row = liveRow(pid, sample, car, now, appSeconds());
      if (!row) continue;
      emit("live_value", { row });
      const series = live.get(pid) ?? [];
      series.push({ t: elapsedAtRead, value: row.value });
      live.set(pid, series);
    }
    await wait(220);
    const elapsed = appSeconds() - start;
    const left = Math.max(0, seconds - Math.trunc(elapsed));
    const percent = 25 + Math.trunc((70 * elapsed) / total);
    emit("health_progress", { step: t(W, "Messwerte sammeln, noch %1 s", left), percent: Math.min(95, percent) });
  }

  const row = runAnalysis({
    milOn: status.mil,
    readiness: readinessStates(),
    faults,
    monitorResults,
    // PID 51 of the demo car says petrol.
    diesel: false,
    live,
  });
  emit("health_progress", { step: t(W, "Fertig"), percent: 100 });
  emit("health_result", { row });
  emit("busy", { active: false });
}

async function readMonitors(): Promise<void> {
  if (!link.connected) {
    emit("monitor_results", { rows: [], note: t(W, "Nicht verbunden.") });
    return;
  }
  emit("busy", { active: true });
  await wait(500);
  // Mode 09: the ECU name is "ECM", a zero byte and "-EngineControl";
  // infoText drops the zero byte.
  emit("identity", { vin: DEMO_VIN, calibration: ENGINE_CALIBRATION, checksum: ENGINE_CVN, ecuName: "ECM-EngineControl" });
  await wait(700);
  const rows = monitorRecords().map(toMonitorRow);
  emit("monitor_results", { rows, note: "" });
  await wait(400);
  emit("oxygen_tests", { rows: [], note: oxygenNote() });
  emit("busy", { active: false });
  log(t(W, "Bauteilüberwachung gelesen: %1 Tests, %2 Sondenprüfungen.", rows.length, 0));
}

export const diagnoseHandlers: PartialHandlers = {
  list_serial_ports: () => DEMO_PORTS.map((p) => ({ ...p })),
  list_j2534_devices: () => DEMO_J2534.map((d) => ({ ...d })),
  // SocketCAN only exists on Linux; the demo behaves like a Windows PC.
  list_socketcan_interfaces: () => [],
  scan_ble: async ({ milliseconds }) => {
    emit("busy", { active: true });
    await wait(Math.min(milliseconds, 2500));
    emit("busy", { active: false });
    emit("ble_scan_finished", { devices: DEMO_BLE.map((d) => ({ ...d })), error: "" });
  },
  run_health_check: ({ seconds }) => {
    void healthCheck(seconds);
  },
  read_monitors: () => {
    void readMonitors();
  },
};
