// Demo answers for the measure pages: the trip computer, the performance
// timer and the battery test of core/analysis ported to TypeScript, the
// adapter voltage, and the manufacturer values (the packs of data/pids and
// the made-up identifiers 1001 to 1010 of the demo engine).
import { t } from "../../../i18n";
import type {
  BatteryInput,
  BatteryReport,
  BatteryStepInfo,
  BatteryStepResult,
  CustomRow,
  Level,
  PerformanceReport,
  PidPackInfo,
  SpeedSample,
  TimedRun,
  TripConfig,
  TripSummary,
} from "../../types";
import { ENGINE_START_SECONDS, simulate } from "../car";
import { L } from "../core";
import packData from "../data/pid-packs.json";
import { readLive, sessionHandlers, setCustomTick, startLoop } from "../session";
import { appSeconds, busy, car, emit, link, subscribe, wait, type PartialHandlers } from "../runtime";

const W = "caruspex::Worker";

// --- Phrases (analysis/Common.h) ------------------------------------------------------

// A text the core builds from a German and an English pattern; numbers get a
// decimal comma in German. Other languages look the English pattern up.
interface Phrase {
  de: string;
  en: string;
}

function formatNumber(value: number, decimals: number, german: boolean): string {
  let out = value.toFixed(Math.max(0, Math.min(6, decimals)));
  // "-0" or "-0.0" looks odd next to a measured value.
  if (/^-[0.]*$/.test(out)) out = out.slice(1);
  return german ? out.replace(".", ",") : out;
}

function number(value: number, decimals: number): Phrase {
  return { de: formatNumber(value, decimals, true), en: formatNumber(value, decimals, false) };
}

function format(de: string, en: string, args: Phrase[] = []): Phrase {
  const fill = (pattern: string, german: boolean) => {
    let i = 0;
    return pattern.replace(/\{\}/g, () => {
      const arg = args[i++];
      return arg ? (german ? arg.de : arg.en) : "";
    });
  };
  return { de: fill(de, true), en: fill(en, false) };
}

const EMPTY: Phrase = { de: "", en: "" };

function plus(...parts: Phrase[]): Phrase {
  const join = (a: string, b: string) => (!a ? b : !b ? a : `${a} ${b}`);
  return parts.reduce((acc, p) => ({ de: join(acc.de, p.de), en: join(acc.en, p.en) }), EMPTY);
}

// The text in the active language. Filled patterns are not in the core
// catalogs, so languages other than German and English read the English.
function text(p: Phrase): string {
  return L(p.de, p.en);
}

// --- Trip computer (analysis/TripComputer.cpp) ---------------------------------------

const PID_RPM = 0x0c;
const PID_SPEED = 0x0d;
const PID_MAP = 0x0b;
const PID_INTAKE_TEMP = 0x0f;
const PID_MAF = 0x10;
const PID_FUEL_RATE = 0x5e;

const ENGINE_ON_RPM = 300;
const MOVING_KMH = 1;
const AIR_GAS_CONSTANT = 287.05; // J/(kg K), dry air
const MAX_PLAUSIBLE_ACCEL = 15; // m/s^2; faster changes are glitches
const ACCEL_WINDOW = 1; // s over which acceleration is judged
const HARSH_ACCEL = 2.5; // m/s^2, about 9 km/h per second
const HARSH_BRAKE = -3;
const HIGH_RPM_PETROL = 3000;
const HIGH_RPM_DIESEL = 2500;

// FuelSource
const NONE = 0, FUEL_RATE = 1, MASS_AIR_FLOW = 2, SPEED_DENSITY = 3;
type Source = 0 | 1 | 2 | 3;

interface Held {
  value: number;
  t: number;
  valid: boolean;
}

class TripComputer {
  private readonly config: TripConfig;
  private held = new Map<number, Held>();
  private lastTick = 0;
  private started = false;
  private distanceKm = 0;
  private fuelLitres = 0;
  private drivingSeconds = 0;
  private movingSeconds = 0;
  private idleSeconds = 0;
  private idleFuel = 0;
  private gapSeconds = 0;
  private maxSpeed = 0;
  private sourceSeconds = new Map<Source, number>();
  private window: { t: number; fuel: number; distance: number }[] = [];
  private liveRate = 0;
  private highRpmSeconds = 0;
  private rpmKnownDrivingSeconds = 0;
  private harshAccel = 0;
  private harshBrake = 0;
  private inHarshAccel = false;
  private inHarshBrake = false;
  private anchorT = 0;
  private anchorV = 0;
  private anchorValid = false;
  private lastSpeed = 0;
  private lastSpeedT = 0;
  private lastSpeedValid = false;
  // A suspicious speed jump, kept until confirmed.
  private pendingJump = 0;
  private pendingJumpValid = false;

  constructor(config: TripConfig) {
    this.config = { ...config };
    if (this.config.maxGapSeconds <= 0) this.config.maxGapSeconds = 5;
    if (this.config.liveWindowSeconds <= 0) this.config.liveWindowSeconds = 5;
    if (this.config.volumetricEfficiency <= 0) this.config.volumetricEfficiency = 0.85;
  }

  private diesel(): boolean {
    return this.config.fuel === 2;
  }

  private stoichiometricRatio(): number {
    return this.diesel() ? 14.5 : 14.7;
  }

  private densityKgPerLitre(): number {
    return this.diesel() ? 0.832 : 0.745;
  }

  private co2KgPerLitre(): number {
    return this.diesel() ? 2.65 : 2.37;
  }

  // Fuel flow in l/h from mass air flow (g/s): g air/s -> g fuel/s -> l/h.
  private fuelRateFromMaf(gramsPerSecond: number): number {
    const fuelGramsPerSecond = gramsPerSecond / this.stoichiometricRatio();
    return (fuelGramsPerSecond / (this.densityKgPerLitre() * 1000)) * 3600;
  }

  // Ideal gas: the air in one full engine displacement is p V / (R T). A
  // four-stroke engine fills its displacement once every two turns.
  private airFlowFromSpeedDensity(mapKpa: number, intakeC: number, rpm: number): number {
    const pressurePa = mapKpa * 1000;
    const volumeM3 = this.config.displacementLitres / 1000;
    const temperatureK = intakeC + 273.15;
    if (temperatureK <= 0) return 0;
    const kgPerFill = (pressurePa * volumeM3 * this.config.volumetricEfficiency) / (AIR_GAS_CONSTANT * temperatureK);
    return kgPerFill * 1000 * (rpm / 120);
  }

  private fresh(pid: number, t: number): Held | null {
    const h = this.held.get(pid);
    if (!h || !h.valid) return null;
    if (t - h.t > this.config.maxGapSeconds) return null;
    return h;
  }

  private currentFuelRate(t: number): { rate: number; source: Source } {
    const rpm = this.fresh(PID_RPM, t);
    if (rpm && rpm.value < ENGINE_ON_RPM) return { rate: 0, source: NONE }; // engine off burns nothing
    const rate = this.fresh(PID_FUEL_RATE, t);
    if (rate) return { rate: Math.max(0, rate.value), source: FUEL_RATE };
    const maf = this.fresh(PID_MAF, t);
    if (maf) return { rate: this.fuelRateFromMaf(Math.max(0, maf.value)), source: MASS_AIR_FLOW };
    const map = this.fresh(PID_MAP, t);
    const iat = this.fresh(PID_INTAKE_TEMP, t);
    if (map && iat && rpm && this.config.displacementLitres > 0) {
      return { rate: this.fuelRateFromMaf(this.airFlowFromSpeedDensity(map.value, iat.value, rpm.value)), source: SPEED_DENSITY };
    }
    return { rate: 0, source: NONE };
  }

  private integrate(t0: number, t1: number): void {
    const dt = t1 - t0;
    if (dt <= 0) return;
    const speed = this.fresh(PID_SPEED, t1);
    const rpm = this.fresh(PID_RPM, t1);
    const { rate, source } = this.currentFuelRate(t1);
    if (!speed && !rpm && source === NONE) return; // nothing known

    const engineOn = rpm ? rpm.value >= ENGINE_ON_RPM : source !== NONE || speed !== null;
    const v = speed ? speed.value : 0;
    const dDistance = (v * dt) / 3600;
    const dFuel = engineOn ? (rate * dt) / 3600 : 0;

    this.distanceKm += dDistance;
    this.fuelLitres += dFuel;
    if (engineOn) {
      this.drivingSeconds += dt;
      if (source !== NONE) this.sourceSeconds.set(source, (this.sourceSeconds.get(source) ?? 0) + dt);
    }
    if (v >= MOVING_KMH) this.movingSeconds += dt;
    if (engineOn && speed && v < MOVING_KMH) {
      this.idleSeconds += dt;
      this.idleFuel += dFuel;
    }
    this.liveRate = engineOn ? rate : 0;

    this.window.push({ t: t1, fuel: dFuel, distance: dDistance });
    while (this.window.length > 0 && this.window[0]!.t < t1 - this.config.liveWindowSeconds) this.window.shift();

    if (engineOn && rpm && v >= 5) {
      this.rpmKnownDrivingSeconds += dt;
      const limit = this.diesel() ? HIGH_RPM_DIESEL : HIGH_RPM_PETROL;
      if (rpm.value > limit) this.highRpmSeconds += dt;
    }
  }

  private trackSpeed(kmh: number, t: number): void {
    this.maxSpeed = Math.max(this.maxSpeed, kmh);
    if (!this.anchorValid || t - this.anchorT > this.config.maxGapSeconds || t < this.anchorT) {
      this.anchorT = t;
      this.anchorV = kmh;
      this.anchorValid = true;
      return;
    }
    // Speed comes in steps of 1 km/h. Over a fraction of a second that step
    // alone looks like a hard acceleration, so acceleration is judged over
    // at least a second.
    if (t - this.anchorT < ACCEL_WINDOW) return;
    const a = (kmh - this.anchorV) / 3.6 / (t - this.anchorT);
    if (a > HARSH_ACCEL) {
      if (!this.inHarshAccel) ++this.harshAccel;
      this.inHarshAccel = true;
    } else {
      this.inHarshAccel = false;
    }
    if (a < HARSH_BRAKE) {
      if (!this.inHarshBrake) ++this.harshBrake;
      this.inHarshBrake = true;
    } else {
      this.inHarshBrake = false;
    }
    this.anchorT = t;
    this.anchorV = kmh;
  }

  addSample(pid: number, value: number, time: number): void {
    let t = time;
    if (!Number.isFinite(value) || !Number.isFinite(t)) return;

    if (pid === PID_SPEED) {
      if (value < 0 || value > 400) return;
      // A single reading that would need an impossible acceleration is a
      // transmission glitch. It is dropped, unless the new value repeats,
      // which means the speed really changed.
      if (this.lastSpeedValid && t >= this.lastSpeedT && t - this.lastSpeedT <= this.config.maxGapSeconds) {
        const dt = Math.max(t - this.lastSpeedT, 0.1);
        const a = Math.abs(value - this.lastSpeed) / 3.6 / dt;
        if (a > MAX_PLAUSIBLE_ACCEL) {
          if (!this.pendingJumpValid || Math.abs(value - this.pendingJump) > 5) {
            this.pendingJump = value;
            this.pendingJumpValid = true;
            return;
          }
        }
      }
      this.pendingJumpValid = false;
    }

    if (!this.started) {
      this.started = true;
      this.lastTick = t;
    } else if (t < this.lastTick) {
      if (this.lastTick - t > this.config.maxGapSeconds) {
        // The clock jumped back: start a new time base, keep the totals.
        for (const h of this.held.values()) h.valid = false;
        this.anchorValid = false;
        this.lastTick = t;
      } else {
        // Slightly out of order: use the value, but do not integrate
        // backwards in time.
        t = this.lastTick;
      }
    } else {
      const dt = t - this.lastTick;
      if (dt > this.config.maxGapSeconds) {
        this.gapSeconds += dt;
        this.anchorValid = false;
      } else {
        this.integrate(this.lastTick, t);
      }
      this.lastTick = t;
    }

    this.held.set(pid, { value, t, valid: true });
    if (pid === PID_SPEED) {
      this.lastSpeed = value;
      this.lastSpeedT = t;
      this.lastSpeedValid = true;
      this.trackSpeed(value, t);
    }
  }

  summary(): TripSummary {
    const s: TripSummary = {
      distanceKm: this.distanceKm,
      fuelLitres: this.fuelLitres,
      drivingSeconds: this.drivingSeconds,
      movingSeconds: this.movingSeconds,
      idleSeconds: this.idleSeconds,
      gapSeconds: this.gapSeconds,
      averageLitresPer100Km: 0,
      liveLitresPer100Km: 0,
      liveLitresPerHour: 0,
      idleLitresPerHour: 0,
      co2GramsPerKm: 0,
      co2Kg: 0,
      cost: 0,
      averageSpeedKmh: 0,
      averageMovingSpeedKmh: 0,
      maxSpeedKmh: this.maxSpeed,
      idleShare: 0,
      fuelSource: NONE,
      fuelIsRough: false,
      fuelNote: "",
      eco: { total: 0, smoothness: 0, engineSpeed: -1, idling: 0, harshAccelerations: 0, harshBrakings: 0, explanation: "" },
    };
    if (this.distanceKm >= 0.1) s.averageLitresPer100Km = (this.fuelLitres / this.distanceKm) * 100;
    let wFuel = 0;
    let wDist = 0;
    for (const w of this.window) {
      wFuel += w.fuel;
      wDist += w.distance;
    }
    const speed = this.fresh(PID_SPEED, this.lastTick);
    if (speed && speed.value >= 5 && wDist >= 0.005) s.liveLitresPer100Km = (wFuel / wDist) * 100;
    s.liveLitresPerHour = this.liveRate;
    if (this.idleSeconds >= 5) s.idleLitresPerHour = (this.idleFuel / this.idleSeconds) * 3600;

    const co2 = this.co2KgPerLitre();
    s.co2Kg = this.fuelLitres * co2;
    s.co2GramsPerKm = s.averageLitresPer100Km * co2 * 10;
    s.cost = this.fuelLitres * this.config.fuelPricePerLitre;

    if (this.drivingSeconds > 0) {
      s.averageSpeedKmh = this.distanceKm / (this.drivingSeconds / 3600);
      s.idleShare = this.idleSeconds / this.drivingSeconds;
    }
    if (this.movingSeconds > 0) s.averageMovingSpeedKmh = this.distanceKm / (this.movingSeconds / 3600);

    let best = 0;
    for (const source of [FUEL_RATE, MASS_AIR_FLOW, SPEED_DENSITY] as Source[]) {
      const seconds = this.sourceSeconds.get(source) ?? 0;
      if (seconds > best) {
        best = seconds;
        s.fuelSource = source;
      }
    }
    const diesel = this.diesel();
    let note: Phrase;
    switch (s.fuelSource) {
      case FUEL_RATE:
        note = format(
          "Verbrauch aus dem Kraftstoffdurchsatz, den das Steuergerät selbst meldet. Das ist die genaueste Quelle.",
          "Fuel use from the fuel rate the control unit reports itself. That is the most accurate source.",
        );
        break;
      case MASS_AIR_FLOW:
        s.fuelIsRough = diesel;
        note = diesel
          ? format(
              "Verbrauch aus dem Luftmassenmesser geschätzt. Ein Diesel läuft mit viel Luftüberschuss, deshalb ist das nur eine grobe Schätzung, die deutlich zu hoch liegen kann.",
              "Fuel use estimated from the mass air flow sensor. A diesel runs with a lot of excess air, so this is only a rough estimate that can be well too high.",
            )
          : format(
              "Verbrauch aus dem Luftmassenmesser berechnet (Luft geteilt durch 14,7). Meist auf etwa 5 bis 10 % genau; beim Ausrollen mit Schubabschaltung liegt der Wert etwas zu hoch.",
              "Fuel use calculated from the mass air flow sensor (air divided by 14.7). Usually accurate to about 5 to 10%; when coasting with fuel cut-off the value reads a little high.",
            );
        break;
      case SPEED_DENSITY:
        s.fuelIsRough = true;
        note = plus(
          format(
            "Verbrauch aus Saugrohrdruck, Ansauglufttemperatur und Drehzahl geschätzt, mit angenommenem Füllungsgrad von {} %. Das ist eine grobe Schätzung; sie lässt sich mit einer vollen Tankfüllung nachjustieren.",
            "Fuel use estimated from manifold pressure, intake temperature and engine speed with an assumed volumetric efficiency of {}%. That is a rough estimate; it can be calibrated against a full tank.",
            [number(this.config.volumetricEfficiency * 100, 0)],
          ),
          diesel ? format("Beim Diesel liegt sie zusätzlich deutlich zu hoch.", "On a diesel it is also well too high.") : EMPTY,
        );
        break;
      default:
        note = format(
          "Kein Verbrauch messbar: Es fehlen Kraftstoffdurchsatz (PID 5E), Luftmasse (PID 10) oder Saugrohrdruck mit Hubraum.",
          "No fuel use available: fuel rate (PID 5E), mass air flow (PID 10) or manifold pressure with displacement are missing.",
        );
    }
    s.fuelNote = text(note);

    // Eco score
    const e = s.eco;
    e.harshAccelerations = this.harshAccel;
    e.harshBrakings = this.harshBrake;
    if (this.drivingSeconds < 60) {
      e.explanation = text(
        format("Noch zu wenig Fahrt für eine Bewertung des Fahrstils.", "Not enough driving yet to rate the driving style."),
      );
      return s;
    }
    // Smoothness: harsh events per ten minutes of moving, short trips count
    // as ten minutes so one event does not ruin them.
    const tenMinutes = Math.max(this.movingSeconds / 600, 1);
    const eventRate = (this.harshAccel + this.harshBrake) / tenMinutes;
    const clamp = (v: number) => Math.min(100, Math.max(0, v));
    e.smoothness = Math.round(clamp(100 - 15 * eventRate));
    let highShare = 0;
    if (this.rpmKnownDrivingSeconds >= 30) {
      highShare = this.highRpmSeconds / this.rpmKnownDrivingSeconds;
      e.engineSpeed = Math.round(clamp(100 - 200 * highShare));
    } else {
      e.engineSpeed = -1;
    }
    const idlePercent = s.idleShare * 100;
    e.idling = Math.round(idlePercent <= 10 ? 100 : clamp(100 - 2 * (idlePercent - 10)));

    let weighted = 0.4 * e.smoothness + 0.3 * e.idling;
    let weights = 0.7;
    if (e.engineSpeed >= 0) {
      weighted += 0.3 * e.engineSpeed;
      weights += 0.3;
    }
    e.total = Math.round(weighted / weights);

    let explanation = format(
      "{} starke Beschleunigungen und {} harte Bremsungen, {} % der Zeit im Stand mit laufendem Motor.",
      "{} hard accelerations and {} hard brakings, {}% of the time standing with the engine running.",
      [number(this.harshAccel, 0), number(this.harshBrake, 0), number(idlePercent, 0)],
    );
    if (e.engineSpeed >= 0) {
      explanation = plus(
        explanation,
        format("{} % der Fahrzeit über {}/min.", "{}% of the driving time above {} rpm.", [
          number(highShare * 100, 0),
          number(diesel ? HIGH_RPM_DIESEL : HIGH_RPM_PETROL, 0),
        ]),
      );
    }
    // One tip for the weakest part, so the advice stays short.
    let weakest = e.smoothness;
    let tip = format(
      "Tipp: Vorausschauend fahren, früh vom Gas gehen und sanft beschleunigen spart am meisten.",
      "Tip: looking ahead, lifting off early and accelerating gently saves the most.",
    );
    if (e.engineSpeed >= 0 && e.engineSpeed < weakest) {
      weakest = e.engineSpeed;
      tip = format(
        "Tipp: Früher hochschalten. Die meisten Motoren arbeiten zwischen 1500 und 2500/min am sparsamsten.",
        "Tip: shift up earlier. Most engines are most efficient between 1500 and 2500 rpm.",
      );
    }
    if (e.idling < weakest) {
      tip = format(
        "Tipp: Bei längeren Wartezeiten den Motor ausschalten. Im Leerlauf verbraucht das Auto Kraftstoff, ohne voranzukommen.",
        "Tip: switch the engine off when waiting longer. At idle the car burns fuel without getting anywhere.",
      );
    }
    if (e.total >= 85) tip = format("Sehr ruhiger, sparsamer Fahrstil.", "Very calm, economical driving style.");
    e.explanation = text(plus(explanation, tip));
    return s;
  }
}

const DEFAULT_TRIP: TripConfig = {
  fuel: 1,
  displacementLitres: 0,
  volumetricEfficiency: 0.85,
  fuelPricePerLitre: 0,
  maxGapSeconds: 5,
  liveWindowSeconds: 5,
};

const trip = { computer: new TripComputer(DEFAULT_TRIP), running: false };

// The backend feeds the trip computer from its live loop; here that loop is
// the live_value event of the demo.
subscribe("live_value", ({ row }) => {
  if (trip.running) trip.computer.addSample(row.pid, row.value, row.time);
});
subscribe("closed", () => {
  trip.running = false;
  stopVoltagePoll();
});

// --- Performance timer (analysis/PerformanceTimer.cpp) ---------------------------------

const STANDSTILL = 0.5; // km/h; below this the car stands
const SPIKE = 20; // km/h jump that is undone by the next sample
const RELIABLE_GAP = 1; // s; larger sample gaps make a run unreliable
const DROP_TOLERANCE = 2; // km/h a rolling run may sag below its start
const BRAKE_REBOUND = 3; // km/h of speed gain that ends a braking run
const QUARTER_MILE_METERS = 402.336;
const SIXTY_MPH_KMH = 96.56064;

function emptyRun(): TimedRun {
  return { found: false, seconds: 0, startTime: 0, endTime: 0, distanceMeters: 0, trapSpeedKmh: 0, reliable: true };
}

function interpolateTime(a: SpeedSample, b: SpeedSample, kmh: number): number {
  if (b.kmh === a.kmh) return b.t;
  return a.t + ((kmh - a.kmh) / (b.kmh - a.kmh)) * (b.t - a.t);
}

// Better run: reliable before unreliable, then faster (or shorter).
function better(candidate: TimedRun, best: TimedRun, byDistance: boolean): boolean {
  if (!candidate.found) return false;
  if (!best.found) return true;
  if (candidate.reliable !== best.reliable) return candidate.reliable;
  return byDistance ? candidate.distanceMeters < best.distanceMeters : candidate.seconds < best.seconds;
}

// The moment the car set off. i is the last standing sample, i + 1 the first
// moving one; a straight line through the first two moving samples,
// followed back to zero, gives a good guess.
function launchTime(s: SpeedSample[], i: number): number {
  const stand = s[i]!;
  const m1 = s[i + 1]!;
  if (i + 2 < s.length) {
    const m2 = s[i + 2]!;
    if (m2.kmh > m1.kmh && m2.t > m1.t) {
      const slope = (m2.kmh - m1.kmh) / (m2.t - m1.t);
      const t0 = m1.t - m1.kmh / slope;
      return Math.min(m1.t, Math.max(stand.t, t0));
    }
  }
  return stand.t;
}

function cleanSpeedSamples(input: SpeedSample[]): { samples: SpeedSample[]; dropped: number } {
  const before = input.length;
  const valid = input
    .filter((x) => Number.isFinite(x.t) && Number.isFinite(x.kmh) && x.kmh >= 0 && x.kmh <= 400)
    .map((x, i) => ({ x, i }))
    .sort((a, b) => a.x.t - b.x.t || a.i - b.i)
    .map((e) => e.x);
  // Equal timestamps: keep the later reading.
  const unique: SpeedSample[] = [];
  for (const x of valid) {
    if (unique.length > 0 && unique[unique.length - 1]!.t === x.t) unique[unique.length - 1] = x;
    else unique.push(x);
  }
  // A single reading far off both neighbours, in opposite directions, is a
  // transmission glitch.
  const out: SpeedSample[] = [];
  for (let i = 0; i < unique.length; i++) {
    if (i > 0 && i + 1 < unique.length) {
      const up = unique[i]!.kmh - unique[i - 1]!.kmh;
      const down = unique[i + 1]!.kmh - unique[i]!.kmh;
      if (Math.abs(up) > SPIKE && Math.abs(down) > SPIKE && up * down < 0) continue;
    }
    out.push(unique[i]!);
  }
  return { samples: out, dropped: before - out.length };
}

function measureFromStandstill(s: SpeedSample[], targetKmh: number): TimedRun {
  let best = emptyRun();
  for (let i = 0; i + 1 < s.length; i++) {
    if (s[i]!.kmh >= STANDSTILL || s[i + 1]!.kmh < STANDSTILL) continue;
    const run = emptyRun();
    run.startTime = launchTime(s, i);
    let maxGap = s[i + 1]!.t - s[i]!.t;
    for (let j = i + 1; j < s.length; j++) {
      maxGap = Math.max(maxGap, s[j]!.t - s[j - 1]!.t);
      if (s[j]!.kmh < STANDSTILL) break; // stopped again before the target
      if (s[j]!.kmh >= targetKmh) {
        run.endTime = interpolateTime(s[j - 1]!, s[j]!, targetKmh);
        run.seconds = run.endTime - run.startTime;
        run.found = run.seconds > 0;
        run.reliable = maxGap <= RELIABLE_GAP;
        break;
      }
    }
    if (better(run, best, false)) best = run;
  }
  return best;
}

function measureRolling(s: SpeedSample[], fromKmh: number, toKmh: number): TimedRun {
  let best = emptyRun();
  for (let i = 1; i < s.length; i++) {
    if (!(s[i - 1]!.kmh < fromKmh && s[i]!.kmh >= fromKmh)) continue;
    const run = emptyRun();
    run.startTime = interpolateTime(s[i - 1]!, s[i]!, fromKmh);
    let maxGap = s[i]!.t - s[i - 1]!.t;
    for (let j = i; j < s.length; j++) {
      if (j > i) maxGap = Math.max(maxGap, s[j]!.t - s[j - 1]!.t);
      if (s[j]!.kmh < fromKmh - DROP_TOLERANCE) break; // lifted off, no run
      if (s[j]!.kmh >= toKmh) {
        run.endTime = j === i ? run.startTime : interpolateTime(s[j - 1]!, s[j]!, toKmh);
        run.seconds = run.endTime - run.startTime;
        run.found = run.seconds > 0;
        run.reliable = maxGap <= RELIABLE_GAP;
        break;
      }
    }
    if (better(run, best, false)) best = run;
  }
  return best;
}

function measureQuarterMile(s: SpeedSample[]): TimedRun {
  let best = emptyRun();
  for (let i = 0; i + 1 < s.length; i++) {
    if (s[i]!.kmh >= STANDSTILL || s[i + 1]!.kmh < STANDSTILL) continue;
    const run = emptyRun();
    run.startTime = launchTime(s, i);
    // From the launch moment the speed rises linearly to the first moving
    // sample; afterwards trapezoids between samples.
    let prev: SpeedSample = { t: run.startTime, kmh: 0 };
    let distance = 0;
    let maxGap = s[i + 1]!.t - s[i]!.t;
    for (let j = i + 1; j < s.length; j++) {
      const cur = s[j]!;
      if (j > i + 1) maxGap = Math.max(maxGap, cur.t - s[j - 1]!.t);
      if (cur.kmh < STANDSTILL) break;
      const dt = cur.t - prev.t;
      if (dt <= 0) continue;
      const va = prev.kmh / 3.6;
      const vb = cur.kmh / 3.6;
      const segment = ((va + vb) / 2) * dt;
      if (distance + segment >= QUARTER_MILE_METERS) {
        // Solve for the moment inside the segment where the distance is
        // reached, with the speed rising linearly.
        const remaining = QUARTER_MILE_METERS - distance;
        const a = (vb - va) / dt;
        const tau = Math.abs(a) < 1e-9 ? remaining / va : (-va + Math.sqrt(Math.max(0, va * va + 2 * a * remaining))) / a;
        run.endTime = prev.t + tau;
        run.seconds = run.endTime - run.startTime;
        run.trapSpeedKmh = (va + a * tau) * 3.6;
        run.distanceMeters = QUARTER_MILE_METERS;
        run.found = true;
        run.reliable = maxGap <= RELIABLE_GAP;
        break;
      }
      distance += segment;
      prev = cur;
    }
    if (better(run, best, false)) best = run;
  }
  return best;
}

function measureBraking(s: SpeedSample[], fromKmh: number): TimedRun {
  let best = emptyRun();
  for (let i = 1; i < s.length; i++) {
    if (!(s[i - 1]!.kmh >= fromKmh && s[i]!.kmh < fromKmh)) continue;
    const run = emptyRun();
    run.startTime = interpolateTime(s[i - 1]!, s[i]!, fromKmh);
    let prev: SpeedSample = { t: run.startTime, kmh: fromKmh };
    let distance = 0;
    let lowest = fromKmh;
    let maxGap = s[i]!.t - s[i - 1]!.t;
    for (let j = i; j < s.length; j++) {
      const cur = s[j]!;
      if (j > i) maxGap = Math.max(maxGap, cur.t - s[j - 1]!.t);
      if (cur.kmh > lowest + BRAKE_REBOUND) break; // braking was interrupted
      lowest = Math.min(lowest, cur.kmh);
      const dt = cur.t - prev.t;
      if (dt > 0) distance += ((prev.kmh + cur.kmh) / 2 / 3.6) * dt;
      if (cur.kmh < STANDSTILL) {
        run.endTime = cur.t;
        run.seconds = run.endTime - run.startTime;
        run.distanceMeters = distance;
        run.found = run.seconds > 0;
        run.reliable = maxGap <= RELIABLE_GAP;
        break;
      }
      prev = cur;
    }
    if (better(run, best, true)) best = run;
  }
  return best;
}

function analyzePerformance(samples: SpeedSample[]): PerformanceReport {
  const { samples: s, dropped } = cleanSpeedSamples(samples);
  let sampleRateHz = 0;
  let accuracySeconds = 0;
  if (s.length >= 2) {
    const gaps: number[] = [];
    for (let i = 1; i < s.length; i++) gaps.push(s[i]!.t - s[i - 1]!.t);
    gaps.sort((a, b) => a - b);
    const medianGap = gaps[Math.floor(gaps.length / 2)]!;
    if (medianGap > 0) sampleRateHz = 1 / medianGap;
    // Interpolation keeps the error at each end below half a sample
    // interval in theory; whole-km/h steps and adapter jitter add to it.
    accuracySeconds = Math.max(0.2, medianGap);
  }
  let hint = format(
    "{} Messwerte pro Sekunde, die Zeiten sind damit auf etwa plus/minus {} s genau. Die Geschwindigkeit über OBD stammt von den Raddrehzahlfühlern und ist in ganzen km/h gestuft; durchdrehende oder blockierende Räder verfälschen sie. Bitte nur dort messen, wo es erlaubt und sicher ist, etwa auf einer abgesperrten Strecke.",
    "{} samples per second, so the times are good to about plus/minus {} s. The speed via OBD comes from the wheel speed sensors and is stepped in whole km/h; spinning or locked wheels distort it. Please only measure where it is legal and safe, such as on a closed track.",
    [number(sampleRateHz, 1), number(accuracySeconds, 1)],
  );
  if (sampleRateHz > 0 && sampleRateHz < 4) {
    hint = plus(
      hint,
      format(
        "Für brauchbare Zeiten sollten es mindestens 5 Messwerte pro Sekunde sein. Dafür während der Messung nur die Geschwindigkeit abfragen.",
        "Usable times need at least 5 samples per second. For that, poll only the speed during the measurement.",
      ),
    );
  }
  return {
    zeroTo100: measureFromStandstill(s, 100),
    eightyTo120: measureRolling(s, 80, 120),
    zeroTo60mph: measureFromStandstill(s, SIXTY_MPH_KMH),
    quarterMile: measureQuarterMile(s),
    brake100To0: measureBraking(s, 100),
    sampleRateHz,
    accuracySeconds,
    samplesUsed: s.length,
    samplesDropped: dropped,
    accuracyHint: text(hint),
  };
}

// --- Battery test (analysis/BatteryTest.cpp) ------------------------------------------

const B = {
  full: 12.6,
  threeQuarters: 12.4,
  weak: 12.2,
  flat: 11.9,
  crankGood: 10.0,
  crankWeak: 9.6,
  crankBad: 8.5,
  chargeLow: 13.8,
  chargeHigh: 14.8,
  chargeNone: 13.2,
  chargeTooHigh: 15.0,
  chargeDangerous: 15.5,
};

const GOOD: Level = 0, INFO: Level = 1, WARNING: Level = 2, CRITICAL: Level = 3;

interface StepDraft {
  id: string;
  level: Level;
  volts: number;
  title: Phrase;
  explanation: Phrase;
  advice: Phrase;
}

interface Draft {
  steps: StepDraft[];
  missing: string[];
  soc: number | null;
  crankingRate: number;
}

function smartCharging(input: BatteryInput): boolean {
  return input.startStop || input.type === 3 || input.type === 2;
}

// AGM batteries rest about 0.1 V higher than flooded ones.
function restingOffset(type: number): number {
  return type === 3 ? 0.1 : 0;
}

const volts = (v: number) => format("{} V", "{} V", [number(v, 2)]);

function addStep(r: Draft, id: string, level: Level, v: number, title: Phrase, explanation: Phrase, advice: Phrase = EMPTY): void {
  r.steps.push({ id, level, volts: v, title, explanation, advice });
}

// Rough state of charge from the resting voltage (lead-acid, about 20 C).
function estimateStateOfCharge(restingVolts: number, type: number): number {
  const table = [
    [11.8, 0],
    [12.0, 25],
    [12.2, 50],
    [12.4, 75],
    [12.6, 100],
  ] as const;
  const v = restingVolts - restingOffset(type);
  if (v <= table[0][0]) return 0;
  if (v >= table[4][0]) return 100;
  for (let i = 1; i < 5; i++) {
    if (v <= table[i]![0]) {
      const f = (v - table[i - 1]![0]) / (table[i]![0] - table[i - 1]![0]);
      const soc = table[i - 1]![1] + f * (table[i]![1] - table[i - 1]![1]);
      return Math.round(soc / 5) * 5; // steps of 5 %
    }
  }
  return 100;
}

function evaluateResting(input: BatteryInput, r: Draft): void {
  if (input.restingVolts === null) {
    r.missing.push("resting");
    return;
  }
  const v = input.restingVolts;
  const off = restingOffset(input.type);
  const soc = estimateStateOfCharge(v, input.type);
  r.soc = soc;

  let notes = EMPTY;
  const recentlyDriven = input.minutesSinceEngineOff !== null && input.minutesSinceEngineOff < 30;
  if (recentlyDriven) {
    notes = format(
      "Der Motor ist erst {} Minuten aus. Kurz nach der Fahrt zeigt die Batterie eine Oberflächenladung und liest sich zu gut. Am aussagekräftigsten ist der Wert nach einigen Stunden Standzeit.",
      "The engine has only been off for {} minutes. Shortly after driving the battery carries a surface charge and reads too good. The value means most after a few hours parked.",
      [number(input.minutesSinceEngineOff!, 0)],
    );
  } else if (v > 12.9 + off) {
    notes = format(
      "Über 12,9 V deutet auf eine Oberflächenladung kurz nach der Fahrt oder dem Laden hin. Nach einigen Stunden Standzeit noch einmal messen.",
      "Above 12.9 V points to a surface charge shortly after driving or charging. Measure again after a few hours parked.",
    );
  }
  if (input.ambientCelsius !== null && input.ambientCelsius < 5) {
    notes = plus(
      notes,
      format(
        "Bei Kälte liegt die Ruhespannung etwas niedriger, ohne dass die Batterie schlechter ist.",
        "In the cold the resting voltage reads a little lower without the battery being worse.",
      ),
    );
  }
  const socText = format("Das entspricht grob {} % Ladung.", "That is roughly {}% charge.", [number(soc, 0)]);

  if (v >= B.full + off) {
    addStep(r, "resting", GOOD, v, format("Batterie voll", "Battery full"),
      plus(format("Ruhespannung {}. Eine volle Batterie hat etwa 12,6 V (AGM etwa 12,7 V).",
        "Resting voltage {}. A full battery has about 12.6 V (AGM about 12.7 V).", [volts(v)]), socText, notes));
  } else if (v >= B.threeQuarters + off) {
    addStep(r, "resting", GOOD, v, format("Batterie gut geladen", "Battery well charged"),
      plus(format("Ruhespannung {}. Nicht ganz voll, aber im grünen Bereich.", "Resting voltage {}. Not quite full, but fine.", [volts(v)]), socText, notes));
  } else if (v >= B.weak + off) {
    addStep(r, "resting", INFO, v, format("Batterie halb geladen", "Battery half charged"),
      plus(format("Ruhespannung {}. Die Batterie ist nur etwa halb voll. Auf Dauer schadet das, weil sich Sulfat an den Platten festsetzt.",
        "Resting voltage {}. The battery is only about half full. In the long run that harms it, because sulphate builds up on the plates.", [volts(v)]), socText, notes),
      format("Eine längere Fahrt machen oder mit einem Ladegerät nachladen.", "Take a longer drive or top it up with a charger."));
  } else if (v >= B.flat + off) {
    addStep(r, "resting", WARNING, v, format("Batterie schwach", "Battery weak"),
      plus(format("Ruhespannung {}. Unter 12,2 V ist die Batterie stark entladen oder gealtert. Bei Kälte kann das Auto damit schon schlecht anspringen.",
        "Resting voltage {}. Below 12.2 V the battery is heavily discharged or aged. In the cold the car may already struggle to start.", [volts(v)]), socText, notes),
      format("Mit einem Ladegerät voll laden und nach einem Tag erneut messen. Fällt die Spannung wieder ab, ist die Batterie verbraucht oder etwas zieht im Stand Strom.",
        "Charge it fully with a charger and measure again after a day. If the voltage drops again, the battery is worn out or something drains it while parked."));
  } else {
    addStep(r, "resting", CRITICAL, v, format("Batterie tiefentladen", "Battery deeply discharged"),
      plus(format("Ruhespannung {}. Die Batterie ist so gut wie leer. Tiefentladung schadet Bleibatterien dauerhaft.",
        "Resting voltage {}. The battery is practically empty. Deep discharge permanently harms lead-acid batteries.", [volts(v)]), notes),
      format("Sofort laden. Danach prüfen lassen, ob sie die Ladung noch hält, und nach einem Stromverbraucher im Stand suchen.",
        "Charge it right away. Afterwards have it checked whether it still holds a charge, and look for something draining it while parked."));
  }
}

function evaluateCranking(input: BatteryInput, r: Draft): void {
  let minimum = input.crankingMinimumVolts;
  if (input.cranking.length > 0) {
    let lowest = input.cranking[0]!.volts;
    let first = input.cranking[0]!.t;
    let last = first;
    for (const s of input.cranking) {
      lowest = Math.min(lowest, s.volts);
      first = Math.min(first, s.t);
      last = Math.max(last, s.t);
    }
    minimum = minimum !== null ? Math.min(minimum, lowest) : lowest;
    if (input.cranking.length >= 2 && last > first) r.crankingRate = (input.cranking.length - 1) / (last - first);
  }
  if (minimum === null) {
    r.missing.push("cranking");
    return;
  }
  const v = minimum;

  let notes = EMPTY;
  if (r.crankingRate > 0 && r.crankingRate < 10) {
    notes = format(
      "Gemessen wurde mit {} Werten pro Sekunde. Der Einbruch beim Anlassen dauert oft nur Sekundenbruchteile, der echte Tiefstwert kann also noch etwas niedriger liegen.",
      "Measured at {} readings per second. The dip while cranking often lasts only a fraction of a second, so the real minimum may be a little lower.",
      [number(r.crankingRate, 0)],
    );
  }
  if (input.ambientCelsius !== null && input.ambientCelsius < 0) {
    notes = plus(notes, format(
      "Bei Frost braucht der Anlasser mehr Strom, ein etwas tieferer Einbruch ist dann normal.",
      "In freezing weather the starter needs more current, so a slightly deeper dip is normal then.",
    ));
  }

  // A "dip" that is hardly below the resting voltage means the start was
  // not caught at all.
  if (input.restingVolts !== null && v > input.restingVolts - 0.3) {
    addStep(r, "cranking", INFO, v, format("Kein Einbruch beim Start gemessen", "No dip measured while cranking"),
      plus(format("Der tiefste Wert ({}) liegt kaum unter der Ruhespannung. Vermutlich wurde der Startvorgang nicht erfasst.",
        "The lowest value ({}) is hardly below the resting voltage. The start was probably not captured.", [volts(v)]), notes),
      format("Den Schritt wiederholen und erst nach dem Start der Messung den Schlüssel drehen.",
        "Repeat the step and only turn the key after the measurement has started."));
    return;
  }

  if (v >= B.crankGood) {
    addStep(r, "cranking", GOOD, v, format("Startspannung gut", "Cranking voltage good"),
      plus(format("Beim Anlassen fiel die Spannung nur auf {}. Über 10 V hat die Batterie genug Kraft für den Anlasser.",
        "While cranking the voltage only dropped to {}. Above 10 V the battery has enough strength for the starter.", [volts(v)]), notes));
  } else if (v >= B.crankWeak) {
    addStep(r, "cranking", INFO, v, format("Startspannung knapp", "Cranking voltage borderline"),
      plus(format("Beim Anlassen fiel die Spannung auf {}. Das reicht noch, aber unter 9,6 V wird es kritisch, vor allem im Winter.",
        "While cranking the voltage dropped to {}. That is still enough, but below 9.6 V it gets critical, especially in winter.", [volts(v)]), notes),
      format("Vor dem Winter die Batterie prüfen lassen.", "Have the battery tested before winter."));
  } else if (v >= B.crankBad) {
    addStep(r, "cranking", WARNING, v, format("Startspannung zu niedrig", "Cranking voltage too low"),
      plus(format("Beim Anlassen fiel die Spannung auf {}. Unter 9,6 V ist meist die Batterie schwach. Ist sie voll geladen und trotzdem so tief, kann auch der Anlasser zu viel Strom ziehen oder ein Kabel schlecht Kontakt haben.",
        "While cranking the voltage dropped to {}. Below 9.6 V the battery is usually weak. If it is fully charged and still drops this low, the starter may draw too much current or a cable may have a poor contact.", [volts(v)]), notes),
      format("Batterie laden und einen Belastungstest machen lassen. Polklemmen und Massekabel auf festen Sitz und Korrosion prüfen.",
        "Charge the battery and have a load test done. Check the terminals and ground cable for tightness and corrosion."));
  } else {
    addStep(r, "cranking", CRITICAL, v, format("Batterie bricht beim Start ein", "Battery collapses while cranking"),
      plus(format("Beim Anlassen fiel die Spannung auf {}. So tief schalten sich manche Steuergeräte schon ab, und der nächste Start kann scheitern.",
        "While cranking the voltage dropped to {}. That low, some control units already switch off, and the next start may fail.", [volts(v)]), notes),
      format("Batterie bald ersetzen oder prüfen lassen, Starthilfekabel mitnehmen.", "Replace or test the battery soon, and carry jump leads."));
  }
}

const SMART_NOTE = format(
  "Dieses Auto hat ein intelligentes Lademanagement (Start-Stopp, AGM- oder EFB-Batterie). Es lädt nur, wenn es nötig ist, und nutzt gern die Bremsphasen. Werte von etwa 12,5 bis 15 V sind dabei normal.",
  "This car has smart charging (start-stop, AGM or EFB battery). It only charges when needed and likes to use braking phases. Values of about 12.5 to 15 V are normal then.",
);

function evaluateCharging(input: BatteryInput, r: Draft): void {
  const smart = smartCharging(input);
  if (input.idleVolts === null) {
    r.missing.push("charging_idle");
  } else {
    const v = input.idleVolts;
    const base = format(
      "Im Leerlauf ohne zusätzliche Verbraucher liegen {} an. Normal sind 13,8 bis 14,8 V.",
      "At idle without extra consumers there are {}. Normal is 13.8 to 14.8 V.",
      [volts(v)],
    );
    if (v > B.chargeDangerous) {
      addStep(r, "charging_idle", CRITICAL, v, format("Ladespannung viel zu hoch", "Charging voltage far too high"),
        plus(base, format("Über 15,5 V kocht die Batterie und Elektronik kann Schaden nehmen. Meist ist der Laderegler defekt.",
          "Above 15.5 V the battery boils and electronics can be damaged. Usually the voltage regulator has failed.")),
        format("Bald in die Werkstatt.", "Have it seen by a garage soon."));
    } else if (v > B.chargeTooHigh) {
      addStep(r, "charging_idle", smart ? INFO : WARNING, v, format("Ladespannung hoch", "Charging voltage high"),
        plus(base, smart ? SMART_NOTE : format("Dauerhaft über 15 V wird die Batterie überladen.", "Permanently above 15 V the battery gets overcharged.")),
        format("Nach ein paar Minuten noch einmal messen. Bleibt es so, Regler prüfen lassen.",
          "Measure again after a few minutes. If it stays like this, have the regulator checked."));
    } else if (v > B.chargeHigh) {
      addStep(r, "charging_idle", smart ? GOOD : INFO, v, format("Ladespannung am oberen Rand", "Charging voltage at the upper edge"),
        plus(base, smart ? SMART_NOTE : format("Kurz nach dem Start oder bei Kälte ist das normal.", "Shortly after starting or in the cold that is normal.")));
    } else if (v >= B.chargeLow) {
      addStep(r, "charging_idle", GOOD, v, format("Lichtmaschine lädt richtig", "Alternator charges properly"), base);
    } else if (v >= B.chargeNone) {
      addStep(r, "charging_idle", smart ? GOOD : INFO, v, format("Ladespannung eher niedrig", "Charging voltage rather low"),
        plus(base, smart ? SMART_NOTE : format("Etwas wenig; der Test mit eingeschalteten Verbrauchern zeigt, ob die Lichtmaschine mithält.",
          "A little low; the test with consumers switched on shows whether the alternator keeps up.")));
    } else if (smart && v >= 12.4) {
      addStep(r, "charging_idle", INFO, v, format("Lademanagement pausiert vermutlich", "Smart charging probably paused"),
        plus(base, SMART_NOTE),
        format("Der nächste Schritt mit eingeschalteten Verbrauchern zeigt, ob die Lichtmaschine liefert.",
          "The next step with consumers switched on shows whether the alternator delivers."));
    } else if (v >= 12.7) {
      addStep(r, "charging_idle", WARNING, v, format("Ladespannung zu niedrig", "Charging voltage too low"),
        plus(base, format("Unter 13,2 V lädt die Batterie kaum.", "Below 13.2 V the battery barely charges.")),
        format("Keilrippenriemen, Kabel und Lichtmaschine prüfen lassen.", "Have the drive belt, cables and alternator checked."));
    } else {
      addStep(r, "charging_idle", CRITICAL, v, format("Lichtmaschine lädt nicht", "Alternator is not charging"),
        plus(base, format("Die Spannung liegt auf Batterieniveau, die Batterie wird also leer gefahren.",
          "The voltage is at battery level, so the battery is being drained.")),
        format("Bald prüfen lassen, sonst bleibt das Auto liegen.", "Have it checked soon, or the car will stop on the road."));
    }
  }

  if (input.loadedVolts === null) {
    r.missing.push("charging_load");
    return;
  }
  const v = input.loadedVolts;
  const base = format(
    "Mit Licht, Gebläse und Heckscheibenheizung liegen {} an.",
    "With lights, blower and rear window heating on there are {}.",
    [volts(v)],
  );
  let drop = EMPTY;
  if (input.idleVolts !== null) {
    drop = format("Das sind {} V weniger als ohne Verbraucher.", "That is {} V less than without consumers.", [number(input.idleVolts - v, 2)]);
    if (input.idleVolts - v <= 0.05) drop = EMPTY;
  }
  const floorBad = smart ? 12.4 : 12.7;
  const floorWeak = smart ? 12.8 : B.chargeNone;
  if (v < floorBad) {
    addStep(r, "charging_load", CRITICAL, v, format("Lichtmaschine schafft die Last nicht", "Alternator cannot handle the load"),
      plus(base, drop, format("Die Batterie muss mitliefern und wird dabei leer.", "The battery has to help out and drains in the process.")),
      format("Lichtmaschine und Keilrippenriemen bald prüfen lassen.", "Have the alternator and drive belt checked soon."));
  } else if (v < floorWeak) {
    addStep(r, "charging_load", WARNING, v, format("Ladespannung bricht unter Last ein", "Charging voltage sags under load"),
      plus(base, drop, format("Eine gesunde Lichtmaschine hält auch unter Last über 13,2 V.", "A healthy alternator stays above 13.2 V even under load."),
        smart ? SMART_NOTE : EMPTY),
      format("Riemenspannung prüfen und die Lichtmaschine testen lassen.", "Check the belt tension and have the alternator tested."));
  } else if (input.idleVolts !== null && input.idleVolts - v > 0.5 && !smart) {
    addStep(r, "charging_load", INFO, v, format("Ladespannung sinkt unter Last spürbar", "Charging voltage drops noticeably under load"),
      plus(base, drop, format("Noch ausreichend, aber mehr als die üblichen 0,5 V Abfall.", "Still enough, but more than the usual 0.5 V drop.")),
      format("Bei Gelegenheit Riemen und Lichtmaschine ansehen lassen.", "Have the belt and alternator looked at when convenient."));
  } else {
    addStep(r, "charging_load", GOOD, v, format("Lichtmaschine hält unter Last", "Alternator copes with the load"), plus(base, drop));
  }
}

function evaluateBattery(input: BatteryInput): BatteryReport {
  const r: Draft = { steps: [], missing: [], soc: null, crankingRate: 0 };
  evaluateResting(input, r);
  evaluateCranking(input, r);
  evaluateCharging(input, r);
  let overall: Level = GOOD;
  for (const s of r.steps) overall = Math.max(overall, s.level) as Level;

  let summary: Phrase;
  if (r.steps.length === 0) {
    summary = format("Noch keine Messung. Bitte die Schritte des Tests durchgehen.", "No measurement yet. Please go through the steps of the test.");
  } else {
    summary =
      overall <= INFO
        ? format("Batterie und Lichtmaschine sind in Ordnung.", "Battery and alternator are fine.")
        : overall === WARNING
          ? format("Ein Teil des Bordnetzes schwächelt. Details stehen bei den einzelnen Schritten.",
              "Part of the electrical system is weak. The details are with the individual steps.")
          : format("Das Bordnetz hat ein ernstes Problem. Bitte bald darum kümmern, sonst springt das Auto womöglich nicht mehr an.",
              "The electrical system has a serious problem. Please deal with it soon, or the car may not start any more.");
    if (r.missing.length > 0) {
      summary = plus(summary, format("{} von 4 Schritten fehlen noch.", "{} of 4 steps are still missing.", [number(r.missing.length, 0)]));
    }
  }
  const steps: BatteryStepResult[] = r.steps.map((s) => ({
    id: s.id,
    level: s.level,
    volts: s.volts,
    title: text(s.title),
    explanation: text(s.explanation),
    advice: text(s.advice),
  }));
  return {
    overall,
    stateOfChargePercent: r.soc,
    steps,
    missingSteps: r.missing,
    summary: text(summary),
    crankingSampleRateHz: r.crankingRate,
  };
}

function batteryTestSteps(): BatteryStepInfo[] {
  return [
    {
      id: "resting",
      instruction: L(
        "Schritt 1, Ruhespannung: Das Auto sollte mindestens 30 Minuten stehen, besser einige Stunden. Alle Verbraucher aus, Türen zu. Zündung nur kurz einschalten (nicht starten), damit der Adapter messen kann.",
        "Step 1, resting voltage: the car should have been parked for at least 30 minutes, better a few hours. All consumers off, doors closed. Switch the ignition on only briefly (do not start) so the adapter can measure.",
      ),
    },
    {
      id: "cranking",
      instruction: L(
        "Schritt 2, Startspannung: Messung starten und dann den Motor anlassen. Caruspex fragt währenddessen nur die Spannung ab, so schnell es geht, und merkt sich den tiefsten Wert.",
        "Step 2, cranking voltage: start the measurement and then start the engine. During this Caruspex polls only the voltage, as fast as it can, and keeps the lowest value.",
      ),
    },
    {
      id: "charging_idle",
      instruction: L(
        "Schritt 3, Ladespannung im Leerlauf: Motor im Leerlauf laufen lassen, alle Verbraucher aus. Etwa eine Minute warten.",
        "Step 3, charging voltage at idle: let the engine idle with all consumers off. Wait about a minute.",
      ),
    },
    {
      id: "charging_load",
      instruction: L(
        "Schritt 4, Ladespannung unter Last: Abblendlicht, Gebläse auf höchster Stufe und Heckscheibenheizung einschalten. Etwa eine Minute warten.",
        "Step 4, charging voltage under load: switch on the low beams, the blower at full speed and the rear window heating. Wait about a minute.",
      ),
    },
  ];
}

// --- Adapter voltage (AT RV at pin 16) -----------------------------------------------------

let voltageTimer: ReturnType<typeof setInterval> | 0 = 0;
// While the battery page records the start (fast polling), the demo car is
// switched off and started again: the start comes two seconds into the
// recording, so the dip is there to catch.
let crankFrom = -1;

function stopVoltagePoll(): void {
  if (voltageTimer) clearInterval(voltageTimer);
  voltageTimer = 0;
  crankFrom = -1;
}

function voltageTick(): void {
  if (!link.connected) {
    stopVoltagePoll();
    return;
  }
  const sample = crankFrom >= 0 ? simulate(appSeconds() - crankFrom + ENGINE_START_SECONDS - 2) : simulate(car.now());
  // ATRV answers with one decimal, a little below the battery itself.
  const volts = Math.round((sample.battery - 0.05) * 10) / 10;
  emit("adapter_voltage", { volts, time: appSeconds() });
}

// --- Manufacturer and user values (core/CustomPids) ----------------------------------------

interface PackValue {
  en: string;
  de: string;
  shortEn: string;
  shortDe: string;
  request: string;
  unit: string;
  min: number;
  max: number;
  header: string;
  source: string;
}

interface PackEntry {
  id: string;
  titleDe: string;
  titleEn: string;
  category: number;
  values: PackValue[];
}

const PACKS = packData as PackEntry[];
const PACK_KEY_BASE = 0x100000;
const USER_KEY_BASE = 0x200000;

interface Known {
  value: PackValue;
  pack: string;
}

// Every custom value by key: the packs, then what was imported.
const known = new Map<number, Known>();
{
  let index = 0;
  for (const pack of PACKS) for (const value of pack.values) known.set(PACK_KEY_BASE + index++, { value, pack: pack.id });
}
let userCount = 0;

function rowOf(key: number, k: Known): CustomRow {
  return {
    key,
    name: L(k.value.de, k.value.en),
    shortName: L(k.value.shortDe, k.value.shortEn),
    shortNameEn: k.value.shortEn,
    unit: k.value.unit,
    pack: k.pack,
    source: k.value.source,
    request: k.value.request,
    header: k.value.header,
  };
}

function packs(): PidPackInfo[] {
  let index = 0;
  return PACKS.map((p) => {
    const values = p.values.map((v) => rowOf(PACK_KEY_BASE + index++, { value: v, pack: p.id }));
    return {
      id: p.id,
      title: L(p.titleDe, p.titleEn),
      brand: p.category === 8 ? (p.titleEn.split(" ")[0] ?? "") : "",
      description: "",
      values,
      category: p.category,
    };
  });
}

// What the demo engine (7E0) answers to service 22 and plain mode 01; every
// other control unit stays silent.
function answer(v: PackValue): { value: number } | { error: string } {
  if (v.request.startsWith("CAN")) {
    return {
      error:
        L("Diese Nachricht ist auf dem Bus nicht zu hören (", "This message is not heard on the bus (") +
        v.request +
        L("). Steckt das Kabel am richtigen Bus, und ist das Auto wach?", "). Is the cable on the right bus, and is the car awake?"),
    };
  }
  const now = car.now();
  const s = simulate(now);
  const running = s.runTime > 0;
  const engine = v.header === "7E0" || v.header === "";
  if (engine && v.request.startsWith("01") && v.request.length === 4) {
    const row = readLive(Number.parseInt(v.request.slice(2), 16));
    if (row) return { value: row.value };
  }
  if (v.header === "7E0") {
    switch (v.request) {
      case "221001":
        // The oil drains back into the sump when the engine stops, so the
        // level reads higher at rest.
        return { value: Math.round(running ? 66 + 2 * Math.sin(now * 0.3) : 74) };
      case "221002":
        return { value: Math.round(s.oil * 10) / 10 };
      case "221003":
        return { value: 1834 + (now >= ENGINE_START_SECONDS ? 1 : 0) };
      case "221004":
        return { value: running ? 1 : 0 };
      case "221010":
        return v.unit === "V"
          ? { value: Math.round(s.battery * 1000) / 1000 }
          : { value: (2456000 + Math.floor(s.runTime)) / 3600 };
    }
  }
  return { error: L("Keine Antwort vom Steuergerät (NO DATA).", "No answer from the control unit (NO DATA).") };
}

// readCustomNow of the Worker: reads each key once, sends the good ones as
// live values. Returns the counts and the first error.
function readCustomNow(keys: number[]): { ok: number; failed: number; firstError: string } {
  let ok = 0;
  let failed = 0;
  let firstError = "";
  for (const key of keys) {
    const k = known.get(key);
    if (!k) continue;
    const a = answer(k.value);
    if ("value" in a) {
      ok++;
      emit("live_value", {
        row: {
          pid: key,
          name: L(k.value.de, k.value.en),
          unit: k.value.unit,
          text: "",
          value: a.value,
          warn: false,
          time: appSeconds(),
          displayMin: k.value.min,
          displayMax: k.value.max,
        },
      });
    } else {
      failed++;
      if (!firstError) firstError = a.error;
    }
  }
  return { ok, failed, firstError };
}

setCustomTick((keys) => {
  readCustomNow(keys);
});

// --- Handlers ------------------------------------------------------------------------------

export const measureHandlers: PartialHandlers = {
  pid_packs: () => packs(),

  read_custom: async ({ keys }) => {
    if (!link.connected) {
      emit("custom_read", { ok: 0, failed: keys.length, firstError: t(W, "Erst verbinden.") });
      return;
    }
    await busy(async () => {
      // Every value is one request; the silent ones wait for the timeout.
      await wait(Math.min(2500, 60 + keys.length * 45));
      emit("custom_read", readCustomNow(keys));
    });
  },

  start_custom_poll: ({ keys }) => {
    if (!link.connected) return;
    link.customKeys = keys;
    if (keys.length > 0 && !link.liveTimer) startLoop();
  },

  import_custom_csv: async ({ path }) => {
    await wait(150);
    // The browser hands out the file name only, not the file; the demo knows
    // its own pack file and treats it as the imported one.
    if (!/caruspex-demo/i.test(path)) {
      emit("custom_imported", {
        rows: [],
        errors: [
          t(W, "Datei lässt sich nicht öffnen: %1",
            L("Im Browser lässt sich nur caruspex-demo.csv laden.", "In the browser only caruspex-demo.csv can be loaded.")),
        ],
      });
      return;
    }
    const demo = PACKS[0]!;
    const rows: CustomRow[] = [];
    for (const value of demo.values) {
      const key = USER_KEY_BASE + userCount++;
      const entry: Known = { value, pack: "" };
      known.set(key, entry);
      rows.push(rowOf(key, entry));
    }
    emit("custom_imported", { rows, errors: [] });
  },

  start_voltage_poll: ({ intervalMs }) => {
    if (!link.connected) return;
    // The Worker stops the live loop first: the voltage needs the adapter alone.
    sessionHandlers.stop_live?.({});
    if (voltageTimer) clearInterval(voltageTimer);
    const fast = intervalMs <= 60;
    if (fast && crankFrom < 0) crankFrom = appSeconds();
    if (!fast) crankFrom = -1;
    // The browser timer does not go below about 20 ms anyway.
    voltageTimer = setInterval(voltageTick, Math.max(20, intervalMs));
  },

  stop_voltage_poll: () => stopVoltagePoll(),

  trip_reset: ({ config }) => {
    trip.computer = new TripComputer(config);
  },
  trip_start: () => {
    trip.running = true;
  },
  trip_stop: () => {
    trip.running = false;
    return trip.computer.summary();
  },
  trip_summary: () => trip.computer.summary(),

  analyze_performance: ({ samples }) => analyzePerformance(samples),

  evaluate_battery: ({ input }) => evaluateBattery(input),
  battery_test_steps: () => batteryTestSteps(),
};
