// The demo car of crates/caruspex-demo, as a pure function of the time
// since "ignition on": the engine starts after a few seconds, warms up at idle
// and then repeats a drive cycle (accelerate, cruise, brake, stand). Faults,
// freeze frames, readiness and the monitors follow the same rules as the C++
// demo, including clearing with mode 04.

export const DEMO_VIN = "YCXDE15C7MC048217";
export const ENGINE_CALIBRATION = "CX5EM21A0417";
export const GEARBOX_CALIBRATION = "CX6AT21B0203";
export const ENGINE_CVN = "5E710C93";
export const GEARBOX_CVN = "2B9047D1";
export const ADAPTER_VERSION = "ELM327 v1.5";
export const ADAPTER_DESCRIPTION = "OBDII to RS232 Interpreter";
export const PROTOCOL_TEXT = "ISO 15765-4 (CAN 11/500)";

const AMBIENT = 12.0;
const BARO = 99.0;
const ODOMETER_START = 48213.4;
const FUEL_START = 64.3;
const TANK_LITRES = 50.0;
const LONG_TERM_TRIM = 17.2;
const CRANK_SECONDS = 1.0;
const WARM_IDLE_SECONDS = 45.0;
export const ENGINE_START_SECONDS = 4.0;

const RATIOS = [4.2, 2.5, 1.65, 1.25, 1.0, 0.82];
const FINAL_DRIVE = 3.2;
const TYRE = 1.94;

export type Phase = "off" | "crank" | "idle" | "accelerate" | "cruise" | "brake";

interface Segment {
  duration: number;
  v0: number;
  v1: number;
  phase: Phase;
}

const CYCLE: Segment[] = [
  { duration: 12, v0: 0, v1: 50, phase: "accelerate" },
  { duration: 28, v0: 50, v1: 50, phase: "cruise" },
  { duration: 15, v0: 50, v1: 100, phase: "accelerate" },
  { duration: 30, v0: 100, v1: 100, phase: "cruise" },
  { duration: 15, v0: 100, v1: 0, phase: "brake" },
  { duration: 20, v0: 0, v1: 0, phase: "idle" },
];
const WOBBLE = 1.2;

function segmentPoint(seg: Segment, s: number): { speed: number; km: number } {
  if (seg.phase === "cruise") {
    const period = seg.duration / 2;
    const w = (2 * Math.PI) / period;
    return {
      speed: seg.v0 + WOBBLE * Math.sin(w * s),
      km: (seg.v0 * s + (WOBBLE / w) * (1 - Math.cos(w * s))) / 3600,
    };
  }
  const slope = (seg.v1 - seg.v0) / seg.duration;
  return { speed: seg.v0 + slope * s, km: (seg.v0 * s + 0.5 * slope * s * s) / 3600 };
}

const CYCLE_SECONDS = CYCLE.reduce((a, s) => a + s.duration, 0);
const CYCLE_KM = CYCLE.reduce((a, s) => a + segmentPoint(s, s.duration).km, 0);

export interface Sample {
  phase: Phase;
  runTime: number;
  speed: number;
  rpm: number;
  distanceKm: number;
  gear: number;
  gearRatio: number;
  load: number;
  throttleRel: number;
  throttleAbs: number;
  throttleCmd: number;
  pedalD: number;
  pedalE: number;
  map: number;
  maf: number;
  fuelRate: number;
  timing: number;
  iat: number;
  coolant: number;
  oil: number;
  catTemp: number;
  rail: number;
  stft: number;
  ltft: number;
  o2up: number;
  o2down: number;
  lambda: number;
  fuelStatus: number;
  battery: number;
  fuelLevel: number;
  odometer: number;
  torqueDemand: number;
  torqueActual: number;
}

function blank(): Sample {
  return {
    phase: "off", runTime: 0, speed: 0, rpm: 0, distanceKm: 0, gear: 0, gearRatio: 0,
    load: 0, throttleRel: 0, throttleAbs: 14.5, throttleCmd: 0, pedalD: 14.9, pedalE: 7.5,
    map: BARO, maf: 0, fuelRate: 0, timing: 0, iat: AMBIENT + 2, coolant: AMBIENT, oil: AMBIENT,
    catTemp: AMBIENT, rail: 0, stft: 0, ltft: LONG_TERM_TRIM, o2up: 0.45, o2down: 0.45, lambda: 1,
    fuelStatus: 0, battery: 12.6, fuelLevel: FUEL_START, odometer: ODOMETER_START,
    torqueDemand: 0, torqueActual: 0,
  };
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

export function engineRunTime(t: number, start = ENGINE_START_SECONDS): number {
  return Math.max(0, t - start - CRANK_SECONDS);
}

// The whole car at one moment. No state, so any moment can be looked at
// again (the freeze frame does exactly that).
export function simulate(t: number, start = ENGINE_START_SECONDS): Sample {
  const s = blank();
  if (t < start) {
    s.phase = "off";
    s.battery = Math.max(12.45, 12.62 - 0.0004 * t);
    return s;
  }
  if (t < start + CRANK_SECONDS) {
    s.phase = "crank";
    s.rpm = 240 + 30 * Math.sin(t * 40);
    s.battery = 10.4 + 0.2 * Math.sin(t * 25);
    s.fuelStatus = 0x01;
    s.lambda = 0.9;
    s.rail = 3000;
    return s;
  }
  const run = t - start - CRANK_SECONDS;
  s.runTime = run;
  const warm = 1 - Math.exp(-run / 200);
  s.coolant = AMBIENT + (90 - AMBIENT) * warm + 1.5 * Math.sin(run / 45) * warm ** 4;
  s.oil = AMBIENT + (96 - AMBIENT) * (1 - Math.exp(-run / 380));
  const cold = clamp((70 - s.coolant) / 58, 0, 1);

  s.phase = "idle";
  let segTime = 0;
  const driveTime = run - WARM_IDLE_SECONDS;
  if (driveTime > 0) {
    const cycles = Math.floor(driveTime / CYCLE_SECONDS);
    let tau = driveTime - cycles * CYCLE_SECONDS;
    let km = cycles * CYCLE_KM;
    for (const seg of CYCLE) {
      if (tau <= seg.duration) {
        const p = segmentPoint(seg, tau);
        s.speed = Math.max(0, p.speed);
        km += p.km;
        s.phase = seg.phase;
        segTime = tau;
        break;
      }
      km += segmentPoint(seg, seg.duration).km;
      tau -= seg.duration;
    }
    s.distanceKm = km;
  }

  const rpmAt = (kmh: number, gear: number) =>
    (kmh / 3.6 / TYRE) * 60 * (RATIOS[gear - 1] ?? 1) * FINAL_DRIVE;
  let gear = 1;
  if (s.speed >= 3) {
    if (s.phase === "accelerate") {
      gear = 6;
      for (let g = 1; g <= 6; g++) {
        if (rpmAt(s.speed, g) <= 2700) {
          gear = g;
          break;
        }
      }
    } else {
      const lowest = s.phase === "brake" ? 1050 : 1250;
      for (let g = 6; g >= 1; g--) {
        if (rpmAt(s.speed, g) >= lowest) {
          gear = g;
          break;
        }
      }
    }
  }
  s.gear = gear;
  s.gearRatio = RATIOS[gear - 1] ?? 1;

  const idle = 760 + 340 * cold;
  let rpm = s.speed < 3 ? idle : rpmAt(s.speed, gear);
  if (s.phase === "accelerate" && s.speed < 25) rpm = Math.max(rpm, 1500 + 40 * s.speed);
  s.rpm = Math.max(rpm, idle) + 8 * Math.sin(t * 7.3);

  switch (s.phase) {
    case "accelerate":
      s.throttleRel = 36 + 6 * Math.sin(segTime * 0.9);
      s.load = 68 + 8 * Math.sin(segTime * 0.7);
      s.timing = 16 + 2 * Math.sin(t * 1.3);
      s.torqueDemand = 58;
      s.rail = 14000;
      break;
    case "cruise":
      s.throttleRel = 9 + 0.09 * s.speed + 1.5 * Math.sin(t * 0.8);
      s.load = 20 + 0.14 * s.speed + 2 * Math.sin(t * 0.8);
      s.timing = 32 - 0.25 * s.load;
      s.torqueDemand = s.load * 0.85;
      s.rail = 7000 + 30 * s.speed;
      break;
    case "brake":
      s.throttleRel = 0;
      s.load = 13 + 2 * Math.sin(t);
      s.timing = 22;
      s.torqueDemand = 0;
      s.rail = 4000;
      break;
    default:
      s.throttleRel = 0;
      s.load = 20 + 6 * cold + Math.sin(t * 1.7);
      s.timing = 7 + 3 * Math.sin(t * 2.1) - 4 * cold;
      s.torqueDemand = 8;
      s.rail = 4500;
  }
  const fuelCut = s.phase === "brake" && s.rpm > 1150 && cold < 0.5;
  s.torqueActual = fuelCut ? 0 : s.load * 0.92;
  s.throttleAbs = 14.5 + s.throttleRel * 0.82;
  s.throttleCmd = s.throttleRel * 0.95 + (s.speed < 3 ? 3.1 + 2 * cold : 0);
  s.pedalD = 14.9 + s.throttleRel * 0.75;
  s.pedalE = s.pedalD / 2;
  s.map = Math.min(BARO, 22 + s.load * 0.78);
  s.maf = (s.rpm / 120) * 1.4 * 1.18 * (s.load / 100) * 1.05;
  s.iat = AMBIENT + 3 + 7 * Math.exp(-s.speed / 40) + 3 * (1 - Math.exp(-run / 300));

  if (run <= 25 || s.coolant <= 35) {
    s.fuelStatus = 0x01;
    s.lambda = 0.94;
    s.o2up = run < 15 ? 0.45 : 0.62;
    s.o2down = 0.45;
    s.stft = 0;
  } else if (fuelCut) {
    s.fuelStatus = 0x04;
    s.lambda = 1.99;
    s.o2up = 0.04;
    s.o2down = 0.09;
    s.stft = 0;
  } else {
    s.fuelStatus = 0x02;
    s.lambda = 1;
    const phase = 2 * Math.PI * 1.1 * t;
    s.o2up = 0.45 + 0.38 * Math.tanh(3 * Math.sin(phase));
    s.stft = 2 - 5.5 * Math.tanh(3 * Math.sin(phase - 0.5));
    // A worn catalyst stores little oxygen, so the downstream sensor follows
    // the upstream one instead of staying calm (P0420).
    s.o2down = 0.47 + 0.24 * Math.tanh(2 * Math.sin(phase - 1.1));
  }
  s.fuelRate = fuelCut ? 0 : (s.maf / (14.7 * s.lambda)) * 3.6 / 0.745;

  let catTarget = 430;
  if (s.phase === "cruise") catTarget = 560 + 1.2 * s.speed;
  if (s.phase === "accelerate") catTarget = 700;
  if (s.phase === "brake") catTarget = 520;
  s.catTemp = AMBIENT + (catTarget - AMBIENT) * (1 - Math.exp(-run / 170));
  s.battery = 14.2 + 0.06 * Math.sin(t * 0.9) - 0.5 * Math.exp(-run / 25);

  const fuelUsed = (0.74 * run) / 3600 + 0.068 * s.distanceKm;
  s.fuelLevel = FUEL_START - (fuelUsed / TANK_LITRES) * 100;
  s.odometer = ODOMETER_START + s.distanceKm;
  return s;
}

export function warmUpRunTime(): number {
  return 200 * Math.log((90 - AMBIENT) / (90 - 70));
}

// Run time since clearing at which a monitor completes again (s).
const HEATER_DONE = 90;
const OXYGEN_DONE = 300;
const CATALYST_DONE = 420;
const EGR_DONE = 540;
const MISFIRE_BACK = 180;
const LEAN_PENDING = 240;
const LEAN_CONFIRMED = 600;
const CATALYST_CONFIRMED = 900;
const NEVER = 1e18;

// Fault memory and monitors over time.
export class DemoCar {
  private origin = performance.now();
  cleared = false;
  clearRun = 0;
  clearKm = 0;

  // Seconds since ignition on.
  now(): number {
    return (performance.now() - this.origin) / 1000;
  }

  restart(): void {
    this.origin = performance.now();
  }

  sample(t = this.now()): Sample {
    return simulate(t);
  }

  runSinceClear(t = this.now()): number {
    if (!this.cleared) return NEVER;
    return Math.max(0, engineRunTime(t) - this.clearRun);
  }

  storedCodes(t = this.now()): string[] {
    if (!this.cleared) return ["P0171", "P0420"];
    const r = this.runSinceClear(t);
    const codes: string[] = [];
    if (r >= LEAN_CONFIRMED) codes.push("P0171");
    if (r >= CATALYST_CONFIRMED) codes.push("P0420");
    return codes;
  }

  pendingCodes(t = this.now()): string[] {
    if (!this.cleared) return ["P0300"];
    const r = this.runSinceClear(t);
    const codes: string[] = [];
    if (r >= MISFIRE_BACK) codes.push("P0300");
    if (r >= LEAN_PENDING) codes.push("P0171");
    if (r >= CATALYST_DONE) codes.push("P0420");
    return codes;
  }

  // Permanent codes survive mode 04 on purpose: only a passed monitor removes
  // them.
  permanentCodes(): string[] {
    return ["P0171", "P0420"];
  }

  clear(t = this.now()): void {
    this.cleared = true;
    this.clearRun = engineRunTime(t);
    this.clearKm = simulate(t).distanceKm;
  }

  // Freeze frame 0 belongs to the first fault after clearing; before the demo
  // is cleared it shows an older moment from a warm cruise.
  freezeFrame(frame: number, t = this.now()): { sample: Sample; code: string } | null {
    const start = ENGINE_START_SECONDS;
    if (frame < 0 || frame > 1) return null;
    if (!this.cleared) {
      if (frame === 0) {
        const sample = simulate(start + CRANK_SECONDS + 1300);
        sample.stft = 7;
        return { sample, code: "P0171" };
      }
      return { sample: simulate(start + CRANK_SECONDS + 1520), code: "P0300" };
    }
    const r = this.runSinceClear(t);
    if (frame === 0) {
      if (r < MISFIRE_BACK) return null;
      return { sample: simulate(start + CRANK_SECONDS + this.clearRun + MISFIRE_BACK), code: "P0300" };
    }
    if (r < LEAN_PENDING) return null;
    const sample = simulate(start + CRANK_SECONDS + this.clearRun + LEAN_PENDING);
    sample.stft = 7;
    return { sample, code: "P0171" };
  }

  // Byte D of PID 01: incomplete monitors.
  incompleteMonitors(r: number): number {
    let d = 0x04; // evaporative system: needs a cold soak, never done here
    if (r < CATALYST_DONE) d |= 0x01;
    if (r < OXYGEN_DONE) d |= 0x20;
    if (r < HEATER_DONE) d |= 0x40;
    if (r < EGR_DONE) d |= 0x80;
    return d;
  }

  readinessD(t = this.now()): number {
    if (!this.cleared) return 0x84;
    return this.incompleteMonitors(this.runSinceClear(t));
  }

  monitorComplete(mid: number, t = this.now()): boolean {
    const r = this.runSinceClear(t);
    switch (mid) {
      case 0x01:
      case 0x02:
        return r >= OXYGEN_DONE;
      case 0x21:
        return r >= CATALYST_DONE;
      case 0x3c:
        return false;
      case 0x41:
        return r >= HEATER_DONE;
      default:
        return true;
    }
  }

  // Distance with the MIL on (PID 21) and the counters since clearing.
  milDistance(t = this.now()): number {
    const s = simulate(t);
    if (!this.cleared) return 312 + s.distanceKm;
    if (this.runSinceClear(t) >= LEAN_CONFIRMED) {
      const on = simulate(ENGINE_START_SECONDS + CRANK_SECONDS + this.clearRun + LEAN_CONFIRMED);
      return s.distanceKm - on.distanceKm;
    }
    return 0;
  }

  warmUps(t = this.now()): number {
    const run = simulate(t).runTime;
    const warmAt = warmUpRunTime();
    let count = this.cleared ? 0 : 36;
    if (run >= warmAt && (!this.cleared || this.clearRun < warmAt)) count++;
    return count;
  }

  distanceSinceClear(t = this.now()): number {
    const s = simulate(t);
    return this.cleared ? s.distanceKm - this.clearKm : 1842 + s.distanceKm;
  }

  milMinutes(t = this.now()): number {
    const run = simulate(t).runTime;
    const r = this.runSinceClear(t);
    if (!this.cleared) return Math.floor(1270 + run / 60);
    if (r >= LEAN_CONFIRMED) return Math.floor((r - LEAN_CONFIRMED) / 60);
    return 0;
  }

  minutesSinceClear(t = this.now()): number {
    const run = simulate(t).runTime;
    return Math.floor(this.cleared ? this.runSinceClear(t) / 60 : 2580 + run / 60);
  }
}
