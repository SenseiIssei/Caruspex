// The live values the demo car answers (kEnginePids in DemoTransport.cpp),
// with names, units and ranges from data/Pids.cpp.
import type { LiveRow, PidInfo } from "../types";
import { L } from "./core";
import type { DemoCar, Sample } from "./car";

// PidCategory of data/Pids.h.
const CATEGORY_NAMES: [string, string][] = [
  ["Motor", "Engine"],
  ["Kraftstoff", "Fuel"],
  ["Luft und Aufladung", "Air and boost"],
  ["Abgas", "Exhaust"],
  ["Abgasreinigung", "Emission control"],
  ["Temperaturen", "Temperatures"],
  ["Elektrik", "Electrical"],
  ["Fahrzeug", "Vehicle"],
  ["Hybrid und Elektro", "Hybrid and EV"],
  ["Diagnose", "Diagnostics"],
  ["Hersteller und eigene Werte", "Manufacturer and custom values"],
];
const ENGINE = 0, FUEL = 1, AIR = 2, EXHAUST = 3, TEMPS = 5, ELECTRICAL = 6, VEHICLE = 7, DIAG = 9;

export function categoryName(category: number): string {
  const n = CATEGORY_NAMES[category] ?? CATEGORY_NAMES[0]!;
  return L(n[0], n[1]);
}

interface Spec {
  key: number;
  de: string;
  en: string;
  unit: string;
  unitEn?: string;
  category: number;
  min: number;
  max: number;
  warnLow?: number;
  warnHigh?: number;
  value: (s: Sample, car: DemoCar, t: number) => number;
  text?: (v: number) => string;
}

const fuelStatusText = (v: number): string => {
  switch (v) {
    case 0: return L("Motor aus oder nicht vorhanden", "engine off or not present");
    case 1: return L("offener Regelkreis (Motor noch kalt)", "open loop (engine still cold)");
    case 2: return L("geschlossener Regelkreis (normal)", "closed loop (normal)");
    case 4: return L("offener Regelkreis (Volllast oder Schub)", "open loop (full load or overrun)");
    case 8: return L("offener Regelkreis (Systemfehler)", "open loop (system fault)");
    default: return String(v);
  }
};
const onOff = (v: number) => (v !== 0 ? L("an", "on") : L("aus", "off"));
const o2 = (de: string, en: string) => ({
  de: `Lambdasonde ${de}`,
  en: `O2 sensor ${en}`,
});
const b1s1 = o2("Bank 1 Sonde 1 (vor Kat)", "bank 1 sensor 1 (before catalyst)");
const b1s2 = o2("Bank 1 Sonde 2 (hinter Kat)", "bank 1 sensor 2 (after catalyst)");

// Rounds like the byte encoding of the real answer does.
const q = (v: number, step: number) => Math.round(v / step) * step;

const SPECS: Spec[] = [
  { key: 0x01, de: "Motorkontrollleuchte", en: "Check engine light (MIL)", unit: "", category: DIAG, min: 0, max: 1,
    value: (_s, car, t) => (car.storedCodes(t).length > 0 ? 1 : 0), text: onOff },
  { key: 0x101, de: "Anzahl gespeicherter Fehlercodes", en: "Number of stored trouble codes", unit: "", category: DIAG, min: 0, max: 20,
    value: (_s, car, t) => car.storedCodes(t).length },
  { key: 0x03, de: "Gemischregelung", en: "Fuel system status", unit: "", category: FUEL, min: 0, max: 0,
    value: (s) => s.fuelStatus, text: fuelStatusText },
  { key: 0x04, de: "Motorlast berechnet", en: "Calculated engine load", unit: "%", category: ENGINE, min: 0, max: 100,
    value: (s) => q(s.load, 100 / 255) },
  { key: 0x05, de: "Kühlmitteltemperatur", en: "Coolant temperature", unit: "°C", category: TEMPS, min: -40, max: 130, warnLow: -10, warnHigh: 110,
    value: (s) => Math.round(s.coolant) },
  { key: 0x06, de: "Kurzzeit-Gemischkorrektur Bank 1", en: "Short-term fuel trim bank 1", unit: "%", category: FUEL, min: -30, max: 30, warnLow: -20, warnHigh: 20,
    value: (s) => q(s.stft, 100 / 128) },
  { key: 0x07, de: "Langzeit-Gemischkorrektur Bank 1", en: "Long-term fuel trim bank 1", unit: "%", category: FUEL, min: -30, max: 30, warnLow: -20, warnHigh: 20,
    value: (s) => q(s.ltft, 100 / 128) },
  { key: 0x0b, de: "Saugrohrdruck", en: "Intake manifold pressure", unit: "kPa", category: AIR, min: 0, max: 255,
    value: (s) => Math.round(s.map) },
  { key: 0x0c, de: "Motordrehzahl", en: "Engine speed", unit: "1/min", category: ENGINE, min: 0, max: 8000, warnLow: 0, warnHigh: 6500,
    value: (s) => q(s.rpm, 0.25) },
  { key: 0x0d, de: "Geschwindigkeit", en: "Vehicle speed", unit: "km/h", category: VEHICLE, min: 0, max: 250,
    value: (s) => Math.round(s.speed) },
  { key: 0x0e, de: "Zündzeitpunkt", en: "Ignition timing advance", unit: "Grad v.OT", unitEn: "deg BTDC", category: ENGINE, min: -20, max: 60,
    value: (s) => q(s.timing, 0.5) },
  { key: 0x0f, de: "Ansauglufttemperatur", en: "Intake air temperature", unit: "°C", category: TEMPS, min: -40, max: 100,
    value: (s) => Math.round(s.iat) },
  { key: 0x10, de: "Luftmasse (LMM)", en: "Mass air flow (MAF)", unit: "g/s", category: AIR, min: 0, max: 300,
    value: (s) => q(s.maf, 0.01) },
  { key: 0x11, de: "Drosselklappenstellung", en: "Throttle position", unit: "%", category: AIR, min: 0, max: 100,
    value: (s) => q(s.throttleAbs, 100 / 255) },
  { key: 0x14, de: `${b1s1.de}, Spannung`, en: `${b1s1.en}, voltage`, unit: "V", category: EXHAUST, min: 0, max: 1.275,
    value: (s) => q(s.o2up, 0.005) },
  { key: 0x114, de: `${b1s1.de}, Gemischkorrektur`, en: `${b1s1.en}, fuel trim`, unit: "%", category: EXHAUST, min: -30, max: 30,
    value: (s) => q(s.stft, 100 / 128) },
  { key: 0x15, de: `${b1s2.de}, Spannung`, en: `${b1s2.en}, voltage`, unit: "V", category: EXHAUST, min: 0, max: 1.275,
    value: (s) => q(s.o2down, 0.005) },
  { key: 0x1f, de: "Laufzeit seit Motorstart", en: "Run time since engine start", unit: "s", category: VEHICLE, min: 0, max: 7200,
    value: (s) => Math.floor(s.runTime) },
  { key: 0x21, de: "Strecke mit Fehlerlampe an", en: "Distance with MIL on", unit: "km", category: DIAG, min: 0, max: 65535,
    value: (_s, car, t) => Math.round(car.milDistance(t)) },
  { key: 0x23, de: "Raildruck", en: "Fuel rail pressure", unit: "kPa", category: FUEL, min: 0, max: 250000,
    value: (s) => q(s.rail, 10) },
  { key: 0x2f, de: "Tankfüllstand", en: "Fuel tank level", unit: "%", category: FUEL, min: 0, max: 100, warnLow: 10, warnHigh: 101,
    value: (s) => q(s.fuelLevel, 100 / 255) },
  { key: 0x30, de: "Warmläufe seit Löschen", en: "Warm-ups since codes cleared", unit: "", category: DIAG, min: 0, max: 255,
    value: (_s, car, t) => car.warmUps(t) },
  { key: 0x31, de: "Strecke seit Löschen", en: "Distance since codes cleared", unit: "km", category: DIAG, min: 0, max: 65535,
    value: (_s, car, t) => Math.round(car.distanceSinceClear(t)) },
  { key: 0x33, de: "Luftdruck (barometrisch)", en: "Barometric pressure", unit: "kPa", category: AIR, min: 50, max: 110,
    value: () => 99 },
  { key: 0x3c, de: "Katalysatortemperatur Bank 1 Sonde 1", en: "Catalyst temperature bank 1 sensor 1", unit: "°C", category: EXHAUST, min: 0, max: 1000, warnLow: -50, warnHigh: 900,
    value: (s) => q(s.catTemp, 0.1) },
  { key: 0x42, de: "Spannung Steuergerät", en: "Control unit voltage", unit: "V", category: ELECTRICAL, min: 8, max: 16, warnLow: 11.5, warnHigh: 15,
    value: (s) => q(s.battery - 0.25, 0.001) },
  { key: 0x43, de: "Absolute Last", en: "Absolute load", unit: "%", category: ENGINE, min: 0, max: 200,
    value: (s) => q(s.load * 0.95, 100 / 255) },
  { key: 0x44, de: "Lambda angefordert", en: "Commanded lambda", unit: "", category: FUEL, min: 0, max: 2,
    value: (s) => q(s.lambda, 1 / 32768) },
  { key: 0x45, de: "Relative Drosselklappenstellung", en: "Relative throttle position", unit: "%", category: AIR, min: 0, max: 100,
    value: (s) => q(s.throttleRel, 100 / 255) },
  { key: 0x46, de: "Umgebungstemperatur", en: "Ambient air temperature", unit: "°C", category: TEMPS, min: -40, max: 60,
    value: () => 12 },
  { key: 0x47, de: "Drosselklappe absolut B", en: "Absolute throttle position B", unit: "%", category: AIR, min: 0, max: 100,
    value: (s) => q(s.throttleAbs + 1.2, 100 / 255) },
  { key: 0x49, de: "Fahrpedalstellung D", en: "Accelerator pedal position D", unit: "%", category: ENGINE, min: 0, max: 100,
    value: (s) => q(s.pedalD, 100 / 255) },
  { key: 0x4a, de: "Fahrpedalstellung E", en: "Accelerator pedal position E", unit: "%", category: ENGINE, min: 0, max: 100,
    value: (s) => q(s.pedalE, 100 / 255) },
  { key: 0x4c, de: "Drosselklappen-Ansteuerung", en: "Commanded throttle actuator", unit: "%", category: AIR, min: 0, max: 100,
    value: (s) => q(s.throttleCmd, 100 / 255) },
  { key: 0x4d, de: "Laufzeit mit Fehlerlampe an", en: "Run time with MIL on", unit: "min", category: DIAG, min: 0, max: 65535,
    value: (_s, car, t) => car.milMinutes(t) },
  { key: 0x4e, de: "Zeit seit dem Löschen", en: "Time since codes cleared", unit: "min", category: DIAG, min: 0, max: 65535,
    value: (_s, car, t) => car.minutesSinceClear(t) },
  { key: 0x51, de: "Kraftstoffart", en: "Fuel type", unit: "", category: FUEL, min: 0, max: 0,
    value: () => 1, text: () => L("Benzin", "Gasoline") },
  { key: 0x5c, de: "Öltemperatur", en: "Engine oil temperature", unit: "°C", category: TEMPS, min: -40, max: 150, warnLow: -10, warnHigh: 130,
    value: (s) => Math.round(s.oil) },
  { key: 0x5e, de: "Kraftstoffverbrauch", en: "Engine fuel rate", unit: "l/h", category: FUEL, min: 0, max: 30,
    value: (s) => q(s.fuelRate, 0.05) },
  { key: 0x61, de: "Drehmoment angefordert", en: "Driver's demand engine torque", unit: "%", category: ENGINE, min: -25, max: 100,
    value: (s) => Math.round(s.torqueDemand) },
  { key: 0x62, de: "Drehmoment tatsächlich", en: "Actual engine torque", unit: "%", category: ENGINE, min: -25, max: 100,
    value: (s) => Math.round(s.torqueActual) },
  { key: 0x63, de: "Bezugsdrehmoment des Motors", en: "Engine reference torque", unit: "Nm", category: ENGINE, min: 0, max: 1000,
    value: () => 240 },
  { key: 0xa6, de: "Kilometerstand", en: "Odometer", unit: "km", category: VEHICLE, min: 0, max: 1,
    value: (s) => q(s.odometer, 0.1) },
];

const BY_KEY = new Map(SPECS.map((s) => [s.key, s]));

// The PIDs the engine says it supports (mode 01 support masks).
export const SUPPORTED_PIDS = [
  0x01, 0x03, 0x04, 0x05, 0x06, 0x07, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x13, 0x14, 0x15, 0x1c,
  0x1f, 0x21, 0x23, 0x2f, 0x30, 0x31, 0x33, 0x3c, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x49, 0x4a,
  0x4c, 0x4d, 0x4e, 0x51, 0x5c, 0x5e, 0x61, 0x62, 0x63, 0xa6,
];

// defaultLivePids() of data/Pids.cpp.
export const DEFAULT_LIVE_PIDS = [0x0c, 0x0d, 0x05, 0x0f, 0x04, 0x11, 0x10, 0x0b, 0x06, 0x07, 0x42, 0x03];

export function pidTable(): PidInfo[] {
  return SPECS.map((s) => ({
    key: s.key,
    pid: s.key & 0xff,
    sub: s.key >> 8,
    name: L(s.de, s.en),
    unit: s.unitEn ? L(s.unit, s.unitEn) : s.unit,
    category: s.category,
    categoryName: categoryName(s.category),
    displayMin: s.min,
    displayMax: s.max,
    warnLow: s.warnLow ?? 0,
    warnHigh: s.warnHigh ?? 0,
    isText: !!s.text,
  }));
}

// All values one PID answers with (a PID can carry several).
export function keysOfPid(pid: number): number[] {
  return SPECS.filter((s) => (s.key & 0xff) === pid).map((s) => s.key);
}

export function liveRow(key: number, sample: Sample, car: DemoCar, t: number, time: number): LiveRow | null {
  const spec = BY_KEY.get(key);
  if (!spec) return null;
  const value = spec.value(sample, car, t);
  const warn =
    (spec.warnLow !== undefined || spec.warnHigh !== undefined) &&
    (value < (spec.warnLow ?? -Infinity) || value > (spec.warnHigh ?? Infinity));
  return {
    pid: key,
    name: L(spec.de, spec.en),
    unit: spec.unitEn ? L(spec.unit, spec.unitEn) : spec.unit,
    text: spec.text ? spec.text(value) : "",
    value,
    warn,
    time,
    displayMin: spec.min,
    displayMax: spec.max,
  };
}
