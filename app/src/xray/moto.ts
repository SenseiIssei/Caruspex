// The motorcycle of the X-ray view: a parametric bike from the catalog's
// dimensions and a handful of numbers per kind (naked, sport with fairing,
// touring, enduro, cruiser, scooter) and per engine (single, parallel twin,
// triple, four, V-twin, L-twin, V4, boxer). Frame, tank, seat, engine with
// its cylinders, exhaust, fork, swingarm, both wheels with their brakes,
// handlebar, lights, the bodywork as glass, the battery and the maker's
// diagnostic plug under the seat with the adapter cable, and the control
// units the scan found.
//
// Coordinates as in layout.ts: metres, x along the bike (front +x, 0 the
// middle), y up from the road, z across (+z the left side). Like the cars,
// this is a bike that fits the model (its proportions, its kind, its
// engine), not a drawing of it.
//
// layout.ts and this module import each other; neither may use the other's
// values at load time, only inside functions.
import { BodyType, CarZone } from "../api/types";
import {
  box,
  buildNetwork,
  clampInside,
  cylinder,
  dedupe,
  EcuKind,
  smoothstep,
  socketSpotOf,
  SocketSpot,
  type BusKind,
  type CarLayout,
  type EcuPlace,
  type Hull,
  type Layer,
  type Material,
  type Part,
  type Tube,
  type Vec3,
  type Wheel,
  type XrayInput,
} from "./layout";
import { bodyGeometry, shapeFor, type Profile } from "./shape";

export type MotoStyle = "naked" | "sport" | "touring" | "enduro" | "cruiser" | "scooter";
export const MOTO_STYLES: MotoStyle[] = ["naked", "sport", "touring", "enduro", "cruiser", "scooter"];

export type MotoEngine = "single" | "twin" | "triple" | "four" | "vtwin" | "ltwin" | "v4" | "boxer";
export type FinalDrive = "chain" | "belt" | "shaft";

export interface MotoRow {
  brand: string;
  model: string;
  style: MotoStyle;
  engine: MotoEngine;
  drive?: FinalDrive;
  // Air- and oil-cooled: an oil cooler instead of a radiator.
  air?: boolean;
}

// The motorcycles of the catalog (data/vehicles/moto.csv): kind, engine and
// final drive as the makers name them in their model ranges.
export const MOTO_ROWS: MotoRow[] = [
  { brand: "Honda", model: "CB500F", style: "naked", engine: "twin" },
  { brand: "Honda", model: "CB650R", style: "naked", engine: "four" },
  { brand: "Honda", model: "CRF1100L Africa Twin", style: "enduro", engine: "twin" },
  { brand: "Honda", model: "NC750X", style: "touring", engine: "twin" },
  { brand: "Yamaha", model: "MT-07", style: "naked", engine: "twin" },
  { brand: "Yamaha", model: "MT-09", style: "naked", engine: "triple" },
  { brand: "Yamaha", model: "Tracer 9", style: "touring", engine: "triple" },
  { brand: "Yamaha", model: "Ténéré 700", style: "enduro", engine: "twin" },
  { brand: "Kawasaki", model: "Z650", style: "naked", engine: "twin" },
  { brand: "Kawasaki", model: "Z900", style: "naked", engine: "four" },
  { brand: "Kawasaki", model: "Ninja 650", style: "sport", engine: "twin" },
  { brand: "Kawasaki", model: "Versys 650", style: "touring", engine: "twin" },
  { brand: "Suzuki", model: "SV650", style: "naked", engine: "vtwin" },
  { brand: "Suzuki", model: "V-Strom 650", style: "enduro", engine: "vtwin" },
  { brand: "Suzuki", model: "GSX-S1000", style: "naked", engine: "four" },
  { brand: "KTM", model: "390 Duke", style: "naked", engine: "single" },
  { brand: "KTM", model: "790 Duke", style: "naked", engine: "twin" },
  { brand: "KTM", model: "890 Duke", style: "naked", engine: "twin" },
  { brand: "KTM", model: "1290 Super Adventure", style: "enduro", engine: "vtwin" },
  { brand: "BMW", model: "R 1250 GS", style: "enduro", engine: "boxer", drive: "shaft" },
  { brand: "BMW", model: "R 1300 GS", style: "enduro", engine: "boxer", drive: "shaft" },
  { brand: "BMW", model: "F 900 R", style: "naked", engine: "twin" },
  { brand: "BMW", model: "S 1000 RR", style: "sport", engine: "four" },
  { brand: "Triumph", model: "Street Triple 765", style: "naked", engine: "triple" },
  { brand: "Triumph", model: "Tiger 900", style: "enduro", engine: "triple" },
  { brand: "Triumph", model: "Trident 660", style: "naked", engine: "triple" },
  { brand: "Ducati", model: "Monster", style: "naked", engine: "ltwin" },
  { brand: "Ducati", model: "Multistrada V4", style: "touring", engine: "v4" },
  { brand: "Ducati", model: "Scrambler", style: "naked", engine: "ltwin", air: true },
  { brand: "Harley-Davidson", model: "Sportster S", style: "cruiser", engine: "vtwin", drive: "belt" },
  { brand: "Harley-Davidson", model: "Pan America", style: "enduro", engine: "vtwin" },
];

// Bikes outside the table: the kind from words of the name, the engine
// from the kind.
const STYLE_WORDS: [RegExp, MotoStyle][] = [
  [/SCOOTER|ROLLER|VESPA|\bPCX|FORZA|T-?MAX|X-?MAX|N-?MAX|BURGMAN|ADV ?\d|SH ?\d/, "scooter"],
  [/SPORTSTER|SOFTAIL|GLIDE|REBEL|VULCAN|BOBBER|CRUISER|INTRUDER|SHADOW|FAT BOY|DRAG ?STAR|ELIMINATOR/, "cruiser"],
  [/\bGS\b|ADVENTURE|AFRICA|TIGER|T[EÉ]N[EÉ]R|STROM|PAN AMERICA|KLR|ENDURO|\bDR\b|\bEXC\b|DESERT ?X|TRANSALP/, "enduro"],
  [/TRACER|MULTISTRADA|VERSYS|\bNC\d|\bRT\b|\bGT\b|FJR|PAN-?EUROPEAN|TOURER|DEAUVILLE/, "touring"],
  [/NINJA|\bRR\b|\bR1\b|\bR6\b|\bR7\b|CBR|GSX-?R|PANIGALE|\bRSV|\bRC ?\d|DAYTONA|SUPERSPORT/, "sport"],
];

const STYLE_ENGINE: Record<MotoStyle, MotoEngine> = {
  naked: "twin",
  sport: "four",
  touring: "twin",
  enduro: "twin",
  cruiser: "vtwin",
  scooter: "single",
};

export interface MotoSpec {
  style: MotoStyle;
  engine: MotoEngine;
  drive: FinalDrive;
  liquid: boolean;
  // "table": the row of MOTO_ROWS; "name": guessed from the name; "chosen":
  // the kind picked in the view.
  source: "table" | "name" | "chosen";
}

export function motoRow(brand: string, model: string): MotoRow | null {
  return MOTO_ROWS.find((r) => r.brand === brand && r.model === model) ?? null;
}

// The kind, engine and drive of a bike; a kind picked in the view wins.
export function motoSpec(brand: string, model: string, chosen?: MotoStyle): MotoSpec {
  const row = motoRow(brand, model);
  const upper = model.toUpperCase();
  const named = STYLE_WORDS.find(([re]) => re.test(upper))?.[1] ?? "naked";
  const style = chosen ?? row?.style ?? named;
  const sameKind = row !== null && row.style === style;
  const engine = sameKind ? row.engine : chosen && row?.engine && style !== "scooter" ? row.engine : STYLE_ENGINE[style];
  const drive: FinalDrive = sameKind && row.drive ? row.drive : style === "scooter" ? "belt" : style === "cruiser" ? "belt" : "chain";
  return {
    style,
    engine,
    drive,
    liquid: !(sameKind && row.air),
    source: chosen && chosen !== row?.style ? "chosen" : row ? "table" : "name",
  };
}

// --- The numbers per kind --------------------------------------------------------

interface StyleSpec {
  // Outer diameter and width of the tyres in metres.
  frontWheel: number;
  rearWheel: number;
  frontTyre: number;
  rearTyre: number;
  // Rake of the fork from the vertical in degrees.
  rake: number;
  // Heights of the seat, the top of the steering head and the grips.
  seat: number;
  head: number;
  bar: number;
  // Half the handlebar's width and whether it is two clip-ons.
  barHalf: number;
  clipOns: boolean;
  fairing: "none" | "full" | "touring" | "beak" | "scooter";
  // Top of the windscreen, 0 for none.
  screen: number;
  cases: boolean;
  spokes: "cast" | "wire";
  frontDiscs: 1 | 2;
  exhaust: "side" | "high" | "twin" | "scooter";
  // Swingarm length as a share of the wheelbase.
  pivot: number;
  // Height of the crankcase centre.
  engineY: number;
  // Length, height and width of the tank.
  tank: Vec3;
}

const STYLES: Record<MotoStyle, StyleSpec> = {
  naked: {
    frontWheel: 0.6, rearWheel: 0.63, frontTyre: 0.12, rearTyre: 0.18, rake: 25, seat: 0.8, head: 0.92, bar: 1.04, barHalf: 0.39,
    clipOns: false, fairing: "none", screen: 0, cases: false, spokes: "cast", frontDiscs: 2, exhaust: "side", pivot: 0.4, engineY: 0.37,
    tank: [0.5, 0.25, 0.34],
  },
  sport: {
    frontWheel: 0.6, rearWheel: 0.64, frontTyre: 0.12, rearTyre: 0.19, rake: 23.5, seat: 0.83, head: 0.88, bar: 0.9, barHalf: 0.33,
    clipOns: true, fairing: "full", screen: 1.12, cases: false, spokes: "cast", frontDiscs: 2, exhaust: "side", pivot: 0.42, engineY: 0.4,
    tank: [0.48, 0.24, 0.34],
  },
  touring: {
    frontWheel: 0.62, rearWheel: 0.64, frontTyre: 0.12, rearTyre: 0.18, rake: 25.5, seat: 0.83, head: 0.98, bar: 1.08, barHalf: 0.4,
    clipOns: false, fairing: "touring", screen: 1.34, cases: true, spokes: "cast", frontDiscs: 2, exhaust: "side", pivot: 0.4, engineY: 0.38,
    tank: [0.52, 0.27, 0.4],
  },
  enduro: {
    frontWheel: 0.72, rearWheel: 0.69, frontTyre: 0.09, rearTyre: 0.15, rake: 27, seat: 0.87, head: 1.06, bar: 1.18, barHalf: 0.43,
    clipOns: false, fairing: "beak", screen: 1.4, cases: false, spokes: "wire", frontDiscs: 2, exhaust: "high", pivot: 0.41, engineY: 0.42,
    tank: [0.52, 0.27, 0.42],
  },
  cruiser: {
    frontWheel: 0.66, rearWheel: 0.64, frontTyre: 0.13, rearTyre: 0.19, rake: 30, seat: 0.7, head: 0.92, bar: 1.0, barHalf: 0.43,
    clipOns: false, fairing: "none", screen: 0, cases: false, spokes: "cast", frontDiscs: 1, exhaust: "twin", pivot: 0.33, engineY: 0.36,
    tank: [0.46, 0.22, 0.3],
  },
  scooter: {
    frontWheel: 0.52, rearWheel: 0.5, frontTyre: 0.11, rearTyre: 0.13, rake: 27, seat: 0.78, head: 0.98, bar: 1.06, barHalf: 0.37,
    clipOns: false, fairing: "scooter", screen: 1.22, cases: false, spokes: "cast", frontDiscs: 1, exhaust: "scooter", pivot: 0.32, engineY: 0.34,
    tank: [0.3, 0.12, 0.3],
  },
};

// The cylinders of each engine: banks with their lean (radians about z,
// negative leans forward) and the positions of the cylinders across the bike.
interface EngineSpec {
  width: number;
  bore: number;
  banks: { tilt: number; z: number[] }[];
}

const ENGINES: Record<MotoEngine, EngineSpec> = {
  single: { width: 0.24, bore: 0.1, banks: [{ tilt: -0.25, z: [0] }] },
  twin: { width: 0.3, bore: 0.09, banks: [{ tilt: -0.35, z: [-0.055, 0.055] }] },
  triple: { width: 0.36, bore: 0.08, banks: [{ tilt: -0.4, z: [-0.08, 0, 0.08] }] },
  four: { width: 0.42, bore: 0.07, banks: [{ tilt: -0.5, z: [-0.105, -0.035, 0.035, 0.105] }] },
  vtwin: { width: 0.26, bore: 0.1, banks: [{ tilt: -0.72, z: [0.02] }, { tilt: 0.55, z: [-0.02] }] },
  ltwin: { width: 0.26, bore: 0.1, banks: [{ tilt: -1.2, z: [0.02] }, { tilt: 0.36, z: [-0.02] }] },
  v4: { width: 0.34, bore: 0.08, banks: [{ tilt: -1.05, z: [-0.05, 0.05] }, { tilt: 0.5, z: [-0.05, 0.05] }] },
  boxer: { width: 0.28, bore: 0.12, banks: [] },
};

// --- The envelope -------------------------------------------------------------------

// A bike has no closed body: the envelope the parts are kept in is a box
// over the length, width and height, a little narrower at the ends.
function envelope(L: number, W: number, H: number, profile: Profile): Hull {
  const front = L / 2;
  const rear = -L / 2;
  const floor = 0.02;
  const planHalf = (u: number) => (W / 2) * (0.35 + 0.65 * smoothstep(0, 0.12, Math.min(u, 1 - u)));
  return {
    length: L,
    width: W,
    height: H,
    profile,
    belt: H * 0.6,
    front,
    rear,
    station: (x) => (x - rear) / L,
    xAt: (u) => rear + u * L,
    top: () => H,
    bottom: () => floor,
    beltAt: () => H * 0.6,
    planHalf,
    halfWidth(u, y) {
      if (u < 0 || u > 1 || y < floor || y > H) return 0;
      return planHalf(u);
    },
    section(u, theta) {
      const c = Math.cos(theta);
      const s = Math.sin(theta);
      return [rear + u * L, floor + ((H - floor) * (1 + Math.sign(s) * Math.pow(Math.abs(s), 0.4))) / 2, planHalf(u) * Math.sign(c) * Math.pow(Math.abs(c), 0.4)];
    },
  };
}

// --- Building blocks ------------------------------------------------------------------

const add = (p: Vec3, q: Vec3): Vec3 => [p[0] + q[0], p[1] + q[1], p[2] + q[2]];

// A rod between two points at the same height across (its z the middle of
// both), as a cylinder along x turned into the line.
function rod(id: string, layer: Layer, material: Material, from: Vec3, to: Vec3, diameter: number, zones: CarZone[] = []): Part {
  const dx = to[0] - from[0];
  const dy = to[1] - from[1];
  const length = Math.hypot(dx, dy);
  const tilt = Math.atan2(dy, dx);
  return {
    ...cylinder(id, layer, material, "x", [(from[0] + to[0]) / 2, (from[1] + to[1]) / 2, (from[2] + to[2]) / 2], length, diameter, zones),
    tilt: Math.abs(tilt) > Math.PI / 2 ? tilt - Math.sign(tilt) * Math.PI : tilt,
  };
}

const shell = (id: string, center: Vec3, size: Vec3, tilt = 0, shape: Part["shape"] = "ellipsoid", round = 0.45): Part => ({
  ...box(id, "body", "shell", center, size, [CarZone.Cabin]),
  shape,
  tilt,
  round,
});

// A chain or belt loop round two sprockets in a plane across z.
function loop(a: Vec3, ra: number, b: Vec3, rb: number): Vec3[] {
  const pts: Vec3[] = [];
  const arc = (c: Vec3, r: number, from: number, to: number) => {
    for (let i = 0; i <= 8; i++) {
      const t = from + ((to - from) * i) / 8;
      pts.push([c[0] + r * Math.cos(t), c[1] + r * Math.sin(t), c[2]]);
    }
  };
  // a is in front of b: over the top backwards, round b, along the bottom
  // forwards, round a.
  arc(a, ra, Math.PI / 2, Math.PI / 2);
  arc(b, rb, Math.PI / 2, (3 * Math.PI) / 2);
  arc(a, ra, -Math.PI / 2, Math.PI / 2);
  return dedupe(pts);
}

// --- The bike -----------------------------------------------------------------------

export function buildMotoLayout(input: XrayInput): CarLayout {
  const a = input.anatomy;
  const spec = motoSpec(a.brand, a.model, input.motoStyle);
  const st = STYLES[spec.style];
  const eng = ENGINES[spec.engine];
  const rf = st.frontWheel / 2;
  const rr = st.rearWheel / 2;
  const ft = st.frontTyre;
  const rt = spec.style === "scooter" || spec.style === "enduro" ? st.rearTyre : Math.min(0.24, Math.max(0.14, a.trackMm / 1000));
  const WB = Math.min(1.85, Math.max(1.2, a.wheelbaseMm / 1000));
  const L = Math.max(a.lengthMm / 1000, WB + rf + rr + 0.16);
  const frontAxle = L / 2 - rf - 0.04;
  const rearAxle = frontAxle - WB;
  const tail = -L / 2 + 0.05;
  const theta = (st.rake * Math.PI) / 180;
  const forkDir: [number, number] = [-Math.sin(theta), Math.cos(theta)];
  const HT: Vec3 = [frontAxle - (st.head - rf) * Math.tan(theta), st.head, 0];
  const HB: Vec3 = [HT[0] - forkDir[0] * 0.17, HT[1] - forkDir[1] * 0.17, 0];
  const scooter = spec.style === "scooter";
  const px = rearAxle + st.pivot * WB;
  const py = rr + 0.1;
  const ex = scooter ? px + 0.06 : px + 0.27;
  const ey = st.engineY;
  const seat = st.seat;
  const boxer = spec.engine === "boxer";
  const engineWidth = boxer ? 0.28 : eng.width;

  // The envelope: wide enough for the handlebar, the cases and a boxer's
  // cylinders, high enough for the windscreen.
  const W = Math.max(a.widthMm / 1000, st.barHalf * 2 + 0.06, boxer ? 0.98 : 0, st.cases ? rt + 0.88 : 0);
  const H = Math.max(a.heightMm / 1000, st.screen + 0.05, st.bar + 0.06, HT[1] + 0.22);
  const shape = shapeFor(a, BodyType.Motorcycle);
  const profile = bodyGeometry(L, H, WB, rf, shape).profile;
  const hull = envelope(L, W, H, profile);

  const parts: Part[] = [];
  const tubes: Tube[] = [];
  const anchors: Record<string, Vec3> = {};

  // --- Wheels with their brake discs.
  const discR = scooter ? 0.11 : spec.style === "enduro" ? 0.14 : 0.155;
  const driveSide = spec.drive === "belt" && !scooter ? -1 : 1;
  const wheels: Wheel[] = [
    {
      center: [frontAxle, rf, 0],
      radius: rf,
      width: ft,
      front: true,
      driven: false,
      moto: { discs: st.frontDiscs === 2 ? [{ side: 1, radius: discR }, { side: -1, radius: discR }] : [{ side: -1, radius: discR }], spokes: st.spokes },
    },
    {
      center: [rearAxle, rr, 0],
      radius: rr,
      width: rt,
      front: false,
      driven: true,
      moto: { discs: [{ side: -driveSide, radius: scooter ? 0.1 : 0.11 }], spokes: st.spokes },
    },
  ];

  // --- Fork, steering head, triple clamps.
  const forkZ = ft / 2 + 0.045;
  const axle: Vec3 = [frontAxle, rf, 0];
  for (const side of [1, -1]) {
    parts.push(rod(`fork-${side}`, "body", "metal", [axle[0], axle[1], side * forkZ], [HT[0] - forkDir[0] * 0.02, HT[1], side * forkZ], 0.05));
    const slider: Vec3 = [axle[0] - forkDir[0] * 0.34, axle[1] + forkDir[1] * 0.34, side * forkZ];
    parts.push(rod(`fork-slider-${side}`, "body", "metal", [axle[0], axle[1] - 0.02, side * forkZ], slider, 0.066));
  }
  for (const [id, at] of [["clamp-top", HT], ["clamp-bottom", HB]] as const) {
    parts.push({ ...box(id, "body", "metal", at, [0.09, 0.035, 2 * forkZ + 0.08]), tilt: theta });
  }
  parts.push(rod("steering-head", "body", "metal", add(HB, [forkDir[0] * -0.03, -forkDir[1] * 0.03, 0]), add(HT, [forkDir[0] * 0.03, forkDir[1] * 0.03, 0]), 0.07));

  // --- Frame (one mesh), swingarm, shocks.
  const strands: Vec3[][] = [];
  for (const s of [1, -1]) {
    if (spec.style === "cruiser") {
      strands.push([HB, [HB[0] + 0.02, ey - 0.12, s * 0.1], [px + 0.05, ey - 0.14, s * 0.11], [px, py + 0.06, s * 0.11]]);
      strands.push([[px, seat - 0.06, s * 0.12], [tail + 0.08, seat + 0.02, s * 0.09]]);
      strands.push([[px - 0.02, py + 0.06, s * 0.11], [px, seat - 0.06, s * 0.12]]);
    } else if (scooter) {
      strands.push([HB, [HB[0] - 0.1, 0.3, s * 0.1], [px + 0.2, 0.28, s * 0.12], [px - 0.05, seat - 0.14, s * 0.12], [tail + 0.1, seat - 0.06, s * 0.1]]);
    } else {
      // Twin spars or a trellis from the steering head to the swingarm
      // pivot, the down tubes to the engine, the subframe under the seat.
      strands.push([[HT[0] - 0.02, HT[1] - 0.03, s * 0.05], [HT[0] - 0.2, HT[1] - 0.08, s * 0.13], [px + 0.06, py + 0.2, s * 0.13], [px, py + 0.03, s * 0.12]]);
      strands.push([[HB[0] - 0.01, HB[1], s * 0.05], [HB[0] - 0.1, ey + 0.14, s * 0.11], [ex + 0.14, ey + 0.05, s * 0.1]]);
      if (spec.style === "enduro") strands.push([[ex + 0.14, ey + 0.05, s * 0.1], [ex + 0.16, ey - 0.12, s * 0.09], [ex - 0.12, ey - 0.17, s * 0.09], [px, py - 0.04, s * 0.11]]);
      strands.push([[px + 0.08, py + 0.24, s * 0.12], [tail + 0.1, seat - 0.02, s * 0.08]]);
      strands.push([[px - 0.02, py + 0.08, s * 0.11], [tail + 0.26, seat - 0.07, s * 0.09]]);
    }
  }
  if (spec.style === "cruiser") strands.push([HT, [HT[0] - 0.3, seat + 0.04, 0], [px + 0.05, seat - 0.06, 0]]);
  tubes.push({ id: "frame", layer: "body", points: strands[0]!, branches: strands.slice(1), radius: spec.style === "cruiser" ? 0.026 : 0.02, material: "metal", zones: [] });

  const armZ = rt / 2 + 0.045;
  if (!scooter) {
    for (const s of [1, -1]) parts.push(rod(`swingarm-${s}`, "body", "metal", [px, py, s * armZ], [rearAxle, rr, s * armZ], 0.055));
  }
  if (spec.style === "cruiser" || scooter) {
    for (const s of scooter ? [-1] : [1, -1]) {
      parts.push(rod(`shock-${s}`, "body", "metal", [rearAxle + 0.03, rr + 0.04, s * (armZ + 0.03)], [rearAxle + 0.16, seat - 0.08, s * (armZ + 0.03)], 0.05));
    }
  } else {
    const low: Vec3 = [px + (rearAxle - px) * 0.3, py + (rr - py) * 0.3 + 0.03, 0];
    parts.push(rod("shock", "body", "metal", low, [px + 0.02, seat - 0.13, 0], 0.06));
  }

  // --- Engine: crankcase, gearbox behind it, the cylinders with their heads,
  // coils and throttle bodies, the air box under the tank.
  const crank: Vec3 = [ex, ey, 0];
  parts.push({ ...box("engine", "drivetrain", "engine", crank, boxer ? [0.38, 0.24, 0.28] : [0.34, 0.26, engineWidth], [CarZone.Cylinders]), explode: [0, 0, -0.5] });
  // A scooter's belt case runs beside the rear tyre, not through it.
  const gearbox: Vec3 = scooter ? [(px + rearAxle) / 2, rr + 0.04, Math.max(0.1, rt / 2 + 0.065)] : [ex - 0.25, ey - 0.01, 0];
  parts.push({
    ...box("gearbox", "drivetrain", "gearbox", gearbox, scooter ? [px - rearAxle + 0.05, 0.15, 0.11] : [0.18, 0.21, engineWidth * 0.86], [CarZone.Gearbox]),
    explode: scooter ? [0, 0, 0.4] : [-0.25, 0, -0.5],
  });
  anchors.gearbox = [gearbox[0], gearbox[1] + 0.13, gearbox[2]];
  const ports: Vec3[] = [];
  const coils: Vec3[] = [];
  let coilIndex = 0;
  if (boxer) {
    for (const s of [1, -1]) {
      parts.push({ ...cylinder(`cylinder-${s}`, "drivetrain", "engine", "z", [ex + 0.06, ey + 0.04, s * 0.27], 0.24, eng.bore, [CarZone.Cylinders]), explode: [0, 0.15, -0.5 + s * 0.12] });
      const head: Vec3 = [ex + 0.06, ey + 0.05, s * 0.41];
      parts.push({ ...box(`cylinder-head-${s}`, "drivetrain", "metal", head, [0.18, 0.16, 0.08], [CarZone.Cylinders]), explode: [0, 0.28, -0.5 + s * 0.2] });
      const coil: Vec3 = [head[0], head[1] + 0.1, s * 0.37];
      parts.push({ ...cylinder(`coil-${coilIndex++}`, "drivetrain", "electric", "y", coil, 0.07, 0.04, [CarZone.Ignition]), explode: [0, 0.4, -0.5 + s * 0.2] });
      coils.push(coil);
      ports.push([head[0] + 0.08, head[1] - 0.05, s * 0.38]);
    }
  } else {
    const base: Vec3 = scooter ? [ex + 0.06, ey + 0.02, 0] : [ex + 0.03, ey + 0.12, 0];
    const banks = scooter ? [{ tilt: -1.35, z: [0] }] : eng.banks;
    banks.forEach((bank, bi) => {
      const d: [number, number] = [-Math.sin(bank.tilt), Math.cos(bank.tilt)];
      const at = (dist: number, z: number): Vec3 => [base[0] + d[0] * dist, base[1] + d[1] * dist, z];
      const span = bank.z.length;
      const z0 = (bank.z[0]! + bank.z[span - 1]!) / 2;
      // One cylinder alone, or the block of a row of them.
      const row = bank.z[span - 1]! - bank.z[0]!;
      const block =
        span === 1
          ? cylinder(`cylinder-${bi}`, "drivetrain", "engine", "y", at(0.1, z0), 0.2, eng.bore + 0.02, [CarZone.Cylinders])
          : box(`cylinder-${bi}`, "drivetrain", "engine", at(0.1, z0), [eng.bore + 0.05, 0.2, row + eng.bore + 0.02], [CarZone.Cylinders]);
      parts.push({ ...block, tilt: bank.tilt, explode: [0, 0.15, -0.5] });
      const head = at(0.24, z0);
      parts.push({ ...box(`cylinder-head-${bi}`, "drivetrain", "metal", head, [0.15, 0.09, Math.max(0.16, (bank.z[span - 1]! - bank.z[0]!) + eng.bore + 0.05)], [CarZone.Cylinders]), tilt: bank.tilt, explode: [0, 0.28, -0.5] });
      for (const z of bank.z) {
        const coil = at(0.31, z);
        parts.push({ ...cylinder(`coil-${coilIndex++}`, "drivetrain", "electric", "y", coil, 0.07, 0.035, [CarZone.Ignition]), tilt: bank.tilt, explode: [0, 0.4, -0.5] });
        coils.push(coil);
        // The exhaust leaves the head on the side away from the other bank:
        // in front for a forward leaning bank, behind for the rear one.
        const out = bank.tilt <= 0 ? 1 : -1;
        ports.push([head[0] + out * 0.08 * Math.cos(bank.tilt), head[1] - 0.03, z]);
      }
    });
  }
  // Throttle bodies and the fuel rail behind the heads, the air box above.
  const throttle: Vec3 = scooter ? [ex + 0.05, ey + 0.16, -0.05] : [ex - 0.04, Math.min(seat - 0.03, ey + 0.31), 0];
  parts.push({ ...cylinder("throttle", "drivetrain", "metal", "z", throttle, Math.max(0.08, engineWidth * 0.7), 0.05, [CarZone.Throttle]), explode: [0, 0.5, -0.5] });
  const rail: Vec3 = [throttle[0] + 0.03, throttle[1] + 0.04, throttle[2]];
  parts.push({ ...cylinder("fuel-rail", "drivetrain", "fuel", "z", rail, Math.max(0.06, engineWidth * 0.55), 0.02, [CarZone.Injection]), explode: [0, 0.55, -0.5] });
  const airbox: Vec3 = scooter ? [px + 0.1, seat - 0.2, 0.12] : [ex - 0.14, Math.min(seat + 0.02, ey + 0.4), 0];
  parts.push({ ...box("airbox", "drivetrain", "metal", airbox, [0.22, 0.1, 0.22], [CarZone.Intake]), explode: [0, 0.65, -0.5] });
  tubes.push({ id: "intake-pipe", layer: "drivetrain", points: [airbox, [throttle[0], airbox[1] - 0.02, throttle[2]], throttle], radius: 0.03, material: "metal", zones: [CarZone.Intake], explode: [0, 0.55, -0.5] });
  anchors.engine = [ex, ey + 0.2, 0];
  anchors.intake = airbox;
  anchors.throttle = throttle;
  anchors.injection = rail;
  anchors.ignition = coils[0] ?? anchors.engine;

  // --- Final drive: chain or belt round the sprockets, or the shaft.
  const sprocket: Vec3 = scooter ? [px + 0.02, rr + 0.04, 0.12] : [ex - 0.3, ey - 0.03, driveSide * armZ];
  const rearHub: Vec3 = [rearAxle, rr, scooter ? 0.12 : driveSide * armZ];
  if (spec.drive === "shaft") {
    parts.push({ ...rod("shaft", "drivetrain", "metal", sprocket, rearHub, 0.05, [CarZone.Gearbox]), explode: [0, 0, 0.35] });
    parts.push({ ...box("final-drive", "drivetrain", "gearbox", rearHub, [0.13, 0.13, 0.09], [CarZone.Gearbox]), explode: [0, 0, 0.35] });
  } else if (!scooter) {
    const belt = spec.drive === "belt";
    const small = belt ? 0.06 : 0.045;
    const big = belt ? 0.14 : 0.1;
    parts.push({ ...cylinder("sprocket-front", "drivetrain", "metal", "z", sprocket, belt ? 0.04 : 0.012, small * 2, [CarZone.Gearbox]), explode: [0, 0, driveSide * 0.35] });
    parts.push({ ...cylinder("sprocket-rear", "drivetrain", "metal", "z", rearHub, belt ? 0.04 : 0.012, big * 2, [CarZone.Gearbox]), explode: [0, 0, driveSide * 0.35] });
    tubes.push({ id: belt ? "belt" : "chain", layer: "drivetrain", points: loop(sprocket, small + 0.008, rearHub, big + 0.008), radius: belt ? 0.012 : 0.008, material: belt ? "cabin" : "metal", zones: [CarZone.Gearbox], explode: [0, 0, driveSide * 0.35] });
  }

  // --- Exhaust: headers from every port to the collector under the engine,
  // the catalytic converter, the silencer.
  const collector: Vec3 = scooter ? [ex - 0.02, 0.2, -0.08] : [ex + 0.06, ey - 0.16, -0.04];
  const catalyst: Vec3 = scooter ? [ex - 0.16, 0.22, -0.12] : [ex - 0.1, ey - 0.17, -0.05];
  const headers = ports.map((port): Vec3[] => [port, [port[0] + (port[0] > ex ? 0.07 : -0.05), Math.max(0.22, port[1] - 0.2), port[2] * 0.7 - 0.04], collector]);
  let silencer: [Vec3, Vec3];
  let second: [Vec3, Vec3] | null = null;
  switch (st.exhaust) {
    case "high":
      silencer = [[px - 0.12, seat - 0.24, -0.17], [Math.max(tail + 0.22, px - 0.52), seat - 0.08, -0.17]];
      break;
    case "twin":
      silencer = [[px - 0.05, 0.27, -0.19], [rearAxle - 0.12, 0.3, -0.19]];
      second = [[px - 0.05, 0.38, -0.2], [rearAxle - 0.08, 0.41, -0.2]];
      break;
    case "scooter":
      silencer = [[rearAxle + 0.12, 0.3, -0.15], [rearAxle - 0.26, 0.38, -0.15]];
      break;
    default:
      silencer = [[px - 0.04, ey - 0.06, -0.16], [px - 0.42, seat - 0.3, -0.18]];
  }
  const exhaustOut: Vec3 = [0, 0, -0.85];
  tubes.push({
    id: "exhaust",
    layer: "drivetrain",
    points: dedupe([...(headers[0] ?? [collector]), catalyst, [silencer[0][0] + 0.12, Math.min(silencer[0][1], catalyst[1] + 0.05), silencer[0][2]], silencer[0]]),
    branches: [...headers.slice(1), ...(second ? [[catalyst, [second[0][0] + 0.1, second[0][1], second[0][2]], second[0]] as Vec3[]] : [])],
    radius: 0.022,
    material: "exhaust",
    zones: [CarZone.Exhaust],
    explode: exhaustOut,
  });
  parts.push({ ...rod("catalyst", "drivetrain", "exhaust", [catalyst[0] + 0.11, catalyst[1], catalyst[2]], [catalyst[0] - 0.11, catalyst[1], catalyst[2]], 0.08, [CarZone.Exhaust]), explode: exhaustOut });
  parts.push({ ...rod("silencer", "drivetrain", "exhaust", silencer[0], silencer[1], spec.style === "cruiser" ? 0.085 : 0.11, [CarZone.Exhaust]), explode: exhaustOut });
  if (second) parts.push({ ...rod("silencer-2", "drivetrain", "exhaust", second[0], second[1], 0.085, [CarZone.Exhaust]), explode: exhaustOut });
  anchors.exhaust = catalyst;

  // --- Cooling: the radiator in front of the engine (an oil cooler on an
  // air-cooled one), the hoses.
  const radiator: Vec3 = scooter ? [HT[0] - 0.12, 0.42, 0] : [Math.min(ex + 0.31, frontAxle - rf * 0.55), ey + 0.27, 0];
  const radSize: Vec3 = spec.liquid ? [0.05, scooter ? 0.2 : 0.3, scooter ? 0.26 : Math.max(0.3, engineWidth + 0.04)] : [0.04, 0.1, 0.22];
  parts.push({ ...box("radiator", "cooling", "cooling", radiator, radSize, [CarZone.Cooling]), tilt: 0.25 });
  anchors.cooling = [radiator[0], radiator[1] + radSize[1] / 2, 0];
  if (spec.liquid) {
    tubes.push({ id: "hose-upper", layer: "cooling", points: [[radiator[0] - 0.03, radiator[1] + 0.1, 0.08], [radiator[0] - 0.1, radiator[1] + 0.06, 0.1], [ex + 0.1, ey + 0.2, 0.08]], radius: 0.014, material: "cooling", zones: [CarZone.Cooling] });
    tubes.push({ id: "hose-lower", layer: "cooling", points: [[radiator[0] - 0.02, radiator[1] - 0.12, -0.08], [ex + 0.18, ey - 0.02, -0.1], [ex + 0.12, ey - 0.06, -0.08]], radius: 0.014, material: "cooling", zones: [CarZone.Cooling] });
  }

  // --- Tank (in front of the seat, under the seat on a scooter) and the
  // seat; the bodywork in glass.
  const [tankLen, tankH, tankW] = st.tank;
  const tankFront = HT[0] - 0.05;
  const tank: Vec3 = scooter ? [px + 0.1, seat - 0.27, 0] : [tankFront - tankLen / 2, seat + (spec.style === "cruiser" ? 0.14 : 0.09), 0];
  parts.push({ ...box("fuel-tank", "energy", "fuel", tank, [tankLen, tankH, tankW], [CarZone.FuelTank]), shape: scooter ? "box" : "ellipsoid", tilt: scooter ? 0 : -0.06 });
  anchors.fuel = [tank[0], tank[1] + tankH / 2, 0];
  if (!scooter) parts.push(shell("tank-cover", tank, [tankLen + 0.06, tankH + 0.05, tankW + 0.05], -0.06));
  const seatFront = scooter ? HT[0] - 0.5 : tankFront - tankLen + 0.05;
  const seatRear = spec.style === "cruiser" ? px - 0.2 : tail + 0.12;
  parts.push({ ...box("seat", "body", "seat", [(seatFront + seatRear) / 2, seat - 0.03, 0], [seatFront - seatRear, 0.09, spec.style === "sport" ? 0.24 : 0.3], [CarZone.Cabin]), round: 0.45 });
  if (spec.style === "cruiser") parts.push({ ...box("pillion", "body", "seat", [px - 0.42, seat + 0.03, 0], [0.26, 0.06, 0.22], [CarZone.Cabin]), round: 0.45 });
  anchors.cabin = [(seatFront + seatRear) / 2, seat + 0.03, 0];
  if (!scooter) {
    // From ahead of the pivot back to the rear tyre, no further: under the
    // low seat of a cruiser they reached into the wheel.
    const coverFront = px + 0.07;
    const coverRear = Math.min(coverFront - 0.2, Math.max(px - 0.35, rearAxle + rr * 0.8));
    parts.push(shell("side-covers", [(coverFront + coverRear) / 2, seat - 0.15, 0], [coverFront - coverRear, 0.17, 0.3], 0, "box", 0.35));
    parts.push(shell("tail", [tail + 0.2, seat - 0.01, 0], [0.38, 0.12, 0.2], 0.08));
  }
  parts.push({ ...box("number-plate", "body", "cabin", [tail + 0.04, seat - 0.24, 0], [0.015, 0.14, 0.2]), tilt: 0.3 });
  const fender: Vec3 = spec.style === "enduro" ? [HB[0] + 0.06, HB[1] - 0.06, 0] : [frontAxle - 0.03, 2 * rf + 0.035, 0];
  parts.push(shell("front-fender", fender, [0.46, 0.04, ft + 0.07], spec.style === "enduro" ? 0.15 : 0.12, "box", 0.5));

  // The fairings and the windscreen of each kind.
  const screen = (h: number, lean: number): void => {
    if (st.screen <= 0) return;
    const bottom: Vec3 = [HT[0] + 0.16, st.screen - h * Math.cos(lean), 0];
    const top: Vec3 = [bottom[0] - h * Math.sin(lean), st.screen, 0];
    parts.push({ ...box("windscreen", "body", "screen", [(bottom[0] + top[0]) / 2, (bottom[1] + top[1]) / 2, 0], [0.012, h, spec.style === "touring" ? 0.42 : 0.32]), tilt: lean, round: 0.4 });
  };
  switch (st.fairing) {
    case "full":
      parts.push(shell("fairing", [HT[0] + 0.15, HT[1] - 0.07, 0], [0.42, 0.34, 0.46], -0.3));
      parts.push(shell("fairing-lower", [ex + 0.1, ey + 0.13, 0], [0.86, 0.46, 0.5], 0.08));
      screen(0.24, 0.95);
      break;
    case "touring":
      parts.push(shell("fairing", [HT[0] + 0.12, HT[1] - 0.03, 0], [0.45, 0.48, 0.62], -0.15));
      parts.push(shell("fairing-lower", [ex + 0.16, ey + 0.22, 0], [0.56, 0.3, 0.48], 0.1));
      screen(0.42, 0.55);
      for (const s of [1, -1]) {
        parts.push({ ...shell(`case-${s}`, [rearAxle + 0.04, seat - 0.3, s * (rt / 2 + 0.24)], [0.46, 0.36, 0.2], 0, "box", 0.3) });
      }
      break;
    case "beak":
      parts.push(shell("fairing", [HT[0] + 0.1, HT[1] - 0.05, 0], [0.32, 0.3, 0.4], -0.2));
      parts.push(shell("tank-shrouds", [HT[0] - 0.2, seat + 0.02, 0], [0.5, 0.34, tankW + 0.1], 0.1));
      parts.push({ ...box("bash-plate", "body", "metal", [ex + 0.02, ey - 0.17, 0], [0.42, 0.02, 0.24]), round: 0.3 });
      screen(0.34, 0.45);
      break;
    case "scooter": {
      parts.push(shell("leg-shield", [HT[0] - 0.06, (0.3 + HT[1]) / 2, 0], [0.1, HT[1] - 0.26, 0.52], theta, "box", 0.45));
      const floorFront = HT[0] - 0.16;
      const floorRear = px + 0.14;
      parts.push(shell("floor", [(floorFront + floorRear) / 2, 0.31, 0], [Math.max(0.1, floorFront - floorRear), 0.04, 0.38], 0, "box", 0.5));
      parts.push(shell("rear-body", [rearAxle + 0.24, seat - 0.17, 0], [0.96, 0.42, 0.42], 0.04));
      parts.push(shell("handlebar-cover", [HT[0] - 0.02, HT[1] + 0.07, 0], [0.28, 0.14, 0.5], 0.1));
      screen(0.24, 0.4);
      break;
    }
  }

  // --- Lights, instruments, handlebar, footrests.
  const lamp: Vec3 = spec.style === "sport" ? [HT[0] + 0.32, HT[1] - 0.16, 0] : scooter ? [HT[0] + 0.12, HT[1] + 0.04, 0] : [HT[0] + 0.13, HT[1] - 0.12, 0];
  parts.push(cylinder("headlight", "body", "lamp", "x", lamp, 0.08, spec.style === "sport" ? 0.1 : 0.17, [CarZone.Electrics]));
  const tailLamp: Vec3 = [tail + 0.02, seat + 0.01, 0];
  parts.push(box("tail-light", "body", "rearlamp", tailLamp, [0.04, 0.05, 0.14], [CarZone.Electrics]));
  const dash: Vec3 = [HT[0] - 0.02, HT[1] + 0.11, 0];
  parts.push({ ...box("dash", "body", "cabin", dash, [0.05, 0.1, 0.18], [CarZone.Cabin]), tilt: 0.6 });
  const bar = st.bar;
  const half = Math.min(st.barHalf, W / 2 - 0.04);
  if (st.clipOns) {
    tubes.push({
      id: "handlebar",
      layer: "body",
      points: [[HT[0] + 0.01, HT[1] - 0.02, forkZ], [HT[0] - 0.12, bar, half]],
      branches: [[[HT[0] + 0.01, HT[1] - 0.02, -forkZ], [HT[0] - 0.12, bar, -half]]],
      radius: 0.012,
      material: "metal",
      zones: [],
    });
  } else {
    tubes.push({
      id: "handlebar",
      layer: "body",
      points: [[HT[0] - 0.12, bar, half], [HT[0] - 0.04, bar - 0.04, 0.15], [HT[0], HT[1] + 0.05, 0], [HT[0] - 0.04, bar - 0.04, -0.15], [HT[0] - 0.12, bar, -half]],
      radius: 0.012,
      material: "metal",
      zones: [],
    });
  }
  for (const s of [1, -1]) parts.push(cylinder(`grip-${s}`, "body", "seat", "z", [HT[0] - 0.12, bar, s * (half - 0.05)], 0.11, 0.035));
  if (!scooter) {
    const peg: Vec3 = spec.style === "cruiser" ? [ex + 0.3, ey - 0.02, 0.24] : [px + 0.04, py - 0.06, 0.17];
    for (const s of [1, -1]) parts.push(cylinder(`footrest-${s}`, "body", "metal", "z", [peg[0], peg[1], s * peg[2]], 0.09, 0.025));
  }

  // --- Electrics: the battery under the seat with the main fuse and the
  // starter relay, the ground strap to the engine, alternator and starter,
  // the main harness along the frame to the head (lamps, switches,
  // instruments) and to the tail light, the engine harness.
  const battery: Vec3 = scooter ? [px + 0.36, 0.38, 0] : [px - 0.1, seat - 0.21, 0];
  parts.push(box("battery12", "electrics", "electric", battery, [0.15, 0.13, 0.09], [CarZone.Electrics]));
  anchors.battery = [battery[0], battery[1] + 0.09, 0];
  const mainFuse: Vec3 = [battery[0] + 0.1, battery[1] + 0.03, 0.05];
  parts.push(box("main-fuses", "electrics", "electric", mainFuse, [0.05, 0.04, 0.04], [CarZone.Electrics]));
  // Ahead of the rear tyre: under the low seat of a cruiser the spot behind
  // the pivot was inside the wheel.
  const fuseBox: Vec3 = scooter ? [px - 0.05, seat - 0.12, 0.08] : [Math.max(px - 0.32, rearAxle + rr + 0.09), seat - 0.13, 0.04];
  parts.push(box("fuse-box", "electrics", "electric", fuseBox, [0.11, 0.03, 0.07], [CarZone.Electrics]));
  const alternator: Vec3 = boxer ? [ex + 0.22, ey + 0.06, 0] : [ex + 0.02, ey - 0.01, engineWidth / 2 + 0.03];
  parts.push(cylinder("alternator", "electrics", "electric", boxer ? "x" : "z", alternator, 0.06, 0.18, [CarZone.Electrics]));
  const starter: Vec3 = [ex - 0.1, ey + 0.14, -0.06];
  parts.push(cylinder("starter", "electrics", "electric", "z", starter, 0.13, 0.065, [CarZone.Electrics]));
  tubes.push({ id: "ground", layer: "electrics", points: [[battery[0] - 0.05, battery[1] + 0.07, -0.03], [battery[0] + 0.05, ey + 0.2, -0.08], [ex - 0.12, ey + 0.1, -engineWidth / 2]], radius: 0.008, material: "metal", zones: [CarZone.Electrics] });
  tubes.push({ id: "starter-cable", layer: "electrics", points: [mainFuse, [mainFuse[0] + 0.05, starter[1] + 0.05, -0.02], starter], radius: 0.009, material: "electric", zones: [CarZone.Electrics] });
  const spineY = Math.min(seat - 0.09, HT[1] - 0.12);
  const front: Vec3 = [HT[0] - 0.22, Math.max(spineY, HT[1] - 0.14), 0.05];
  tubes.push({
    id: "harness",
    layer: "electrics",
    points: [fuseBox, [battery[0], seat - 0.11, 0.05], [px + 0.08, spineY, 0.05], front, [HT[0] + 0.02, HT[1] - 0.06, 0.04], lamp],
    branches: [[front, [HT[0] - 0.05, HT[1] + 0.03, 0.06], dash], [fuseBox, [tail + 0.18, seat - 0.07, 0.03], tailLamp], [fuseBox, mainFuse]],
    radius: 0.011,
    material: "electric",
    zones: [CarZone.Electrics],
  });

  // --- The maker's diagnostic plug under the seat (data/obd_sockets.csv),
  // the adapter cable out to the 16-pin socket and the adapter in it.
  const spot = socketSpotOf(a.socketLocation ?? 0, a.socketSpot ?? SocketSpot.UnderSeat) === SocketSpot.EngineBay ? SocketSpot.EngineBay : SocketSpot.UnderSeat;
  const obd: Vec3 = spot === SocketSpot.EngineBay ? [ex + 0.15, ey + 0.32, -0.12] : [px - 0.22, seat - 0.13, -0.09];
  // The cable comes out under the seat on the right; the socket with the
  // adapter hangs beside the side cover.
  const side = -Math.min(W / 2 - 0.07, 0.24);
  const socket: Vec3 = [px - 0.36, seat - 0.2, side];
  const cable: Vec3[] = [obd, [obd[0] - 0.05, seat - 0.1, side * 0.7], [socket[0] + 0.04, seat - 0.1, side], [socket[0], socket[1] + 0.05, side], socket];
  anchors.obd = obd;

  // --- Control units on the frame: the engine control unit under the seat,
  // the ABS unit behind the engine, the instruments at the head.
  const placeAt = (place: EcuPlace): Vec3 => {
    switch (place) {
      case "engineBay": return [px + 0.08, seat - 0.12, 0];
      case "gearbox": return [ex - 0.25, ey + 0.17, 0];
      case "bulkhead": return [px + 0.15, py + 0.17, -0.11];
      case "steeringColumn":
      case "cluster":
      case "dashCentre":
      case "windscreen": return [HT[0] - 0.03, HT[1] + 0.2, 0];
      case "headlight": return [HT[0] + 0.07, HT[1] - 0.06, 0];
      case "battery": return [battery[0], battery[1] + 0.1, 0];
      case "fuelPump": return [tank[0], tank[1] - tankH / 2 + 0.02, 0];
      case "rearAxle": return [rearAxle + 0.18, rr + 0.12, 0];
      case "boot":
      case "rearBumper": return [tail + 0.25, seat - 0.07, 0];
      default: return [px - 0.3, seat - 0.11, 0];
    }
  };
  const lane: Record<BusKind, number> = { powertrain: -0.03, comfort: 0.03, infotainment: 0.06, diagnostic: 0, kline: 0.045, mscan: -0.06, swcan: -0.075 };
  const network = buildNetwork(input, {
    placeAt,
    obd,
    step: 0.07,
    clamp: (q) => clampInside(hull, q, 0.05),
    route: (from, to, bus) => {
      const z = lane[bus];
      return dedupe([from, [from[0], spineY - 0.03, z], [to[0], spineY - 0.03, z], to]);
    },
  });
  anchors.network = network.hub;
  const engineEcu = network.ecus.find((e) => e.kind === EcuKind.Engine);
  anchors.ecu = engineEcu ? engineEcu.position : placeAt("engineBay");
  tubes.push({
    id: "engine-harness",
    layer: "electrics",
    points: [anchors.ecu, [ex - 0.08, Math.max(ey + 0.36, throttle[1] + 0.06), 0.07], ...coils.slice(0, 1)],
    branches: [
      ...coils.slice(1).map((c): Vec3[] => [[ex - 0.08, Math.max(ey + 0.36, throttle[1] + 0.06), 0.07], c]),
      [[ex - 0.08, Math.max(ey + 0.36, throttle[1] + 0.06), 0.07], [alternator[0] - 0.05, ey + 0.16, alternator[2]], alternator],
    ],
    radius: 0.008,
    material: "electric",
    zones: [CarZone.Electrics, CarZone.Ignition],
  });

  // Everything inside the envelope.
  for (const part of parts) part.center = clampInside(hull, part.center, 0.03);
  for (const tube of tubes) {
    tube.points = tube.points.map((q) => clampInside(hull, q, tube.radius + 0.01));
    if (tube.branches) tube.branches = tube.branches.map((b) => b.map((q) => clampInside(hull, q, tube.radius + 0.01)));
  }
  for (const r of network.routes) r.points = r.points.map((q, i) => (i === r.points.length - 1 ? q : clampInside(hull, q, 0.02)));
  for (const key of Object.keys(anchors)) anchors[key] = clampInside(hull, anchors[key]!, 0.03);

  return {
    vehicle: "moto",
    hull,
    shape,
    frontAxle,
    rearAxle,
    wheels,
    parts,
    tubes,
    ecus: network.ecus,
    routes: network.routes,
    busLabels: network.busLabels,
    obd: clampInside(hull, obd, 0.03),
    socketSpot: spot,
    adapter: { socket: clampInside(hull, socket, 0.05), dir: [0, -1, 0], cable: cable.map((q) => clampInside(hull, q, 0.03)) },
    hub: network.hub,
    anchors,
    moto: spec,
    heavy: null,
    size: { lengthMm: a.lengthMm, widthMm: a.widthMm, heightMm: a.heightMm, wheelbaseMm: a.wheelbaseMm, trackMm: a.trackMm, wheelMm: a.wheelMm },
  };
}
