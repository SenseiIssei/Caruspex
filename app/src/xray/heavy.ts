// Trucks and buses of the X-ray view: a European cab-over tractor or rigid
// truck with a box body, a US conventional with its long bonnet, a low-floor
// city bus and a coach. The cab (or the bus body) is the glass hull; behind
// a truck's cab the hull is only the envelope the parts are kept in and the
// frame lies open: rails and cross members, the fifth wheel, fuel and AdBlue
// tanks, the battery box and the exhaust aftertreatment on the frame sides,
// two or three axles with twin tyres at the rear. The control units of a
// J1939 network (engine, gearbox, EBS, instrument cluster, tachograph) and
// the round 9-pin socket sit where one looks for them.
//
// Coordinates as in layout.ts: metres, x along the vehicle (front +x, 0 the
// middle), y up from the road, z across (+z the left side, the driver's in
// Europe and America). Like the cars, a vehicle of this kind in typical
// proportions, not a drawing of the model.
//
// layout.ts and this module import each other; neither may use the other's
// values at load time, only inside functions.
import { CarZone, EngineLayout, type VehicleAnatomy } from "../api/types";
import {
  box,
  buildNetwork,
  clampInside,
  cylinder,
  dedupe,
  EcuKind,
  makeHull,
  socketSpotOf,
  type BusKind,
  type CarLayout,
  type EcuPlace,
  type Part,
  type Tube,
  type Vec3,
  type Wheel,
  type XrayInput,
} from "./layout";
import { shapeFor, type Grille, type Lamp, type Profile, type Seam, type Size } from "./shape";

export type HeavyKind = "tractor" | "rigid" | "citybus" | "coach";
export const HEAVY_KINDS: HeavyKind[] = ["tractor", "rigid", "citybus", "coach"];

// A European truck has its cab over the engine, an American one sits
// behind a bonnet.
export type CabStyle = "cabover" | "conventional";

export interface HeavyRow {
  brand: string;
  model: string;
  kind: HeavyKind;
  axles?: 2 | 3;
}

// The trucks of the catalog (data/vehicles/, body type Truck): what the
// makers sell them as first. The heavy ranges are tractors for long haul;
// the TGS is MAN's construction and distribution range, mostly rigid with
// three axles. The American tractors run on a tandem (6x4).
export const HEAVY_ROWS: HeavyRow[] = [
  { brand: "MAN", model: "TGX", kind: "tractor" },
  { brand: "MAN", model: "TGS", kind: "rigid", axles: 3 },
  { brand: "Scania", model: "R", kind: "tractor" },
  { brand: "Scania", model: "S", kind: "tractor" },
  { brand: "Volvo Trucks", model: "FH", kind: "tractor" },
  { brand: "DAF", model: "XF", kind: "tractor" },
  { brand: "Mercedes-Benz", model: "Actros", kind: "tractor" },
  { brand: "Iveco", model: "Stralis", kind: "tractor" },
  { brand: "Iveco", model: "S-Way", kind: "tractor" },
  { brand: "Renault Trucks", model: "T", kind: "tractor" },
  { brand: "Freightliner", model: "Cascadia", kind: "tractor", axles: 3 },
  { brand: "Kenworth", model: "T680", kind: "tractor", axles: 3 },
  { brand: "Peterbilt", model: "579", kind: "tractor", axles: 3 },
  { brand: "International", model: "LT", kind: "tractor", axles: 3 },
  { brand: "Mack", model: "Anthem", kind: "tractor", axles: 3 },
];

const US_MAKERS = ["Peterbilt", "Kenworth", "Freightliner", "Mack", "International", "Western Star"];

// Buses by maker and name: coach makers, then city and coach ranges.
const COACH_MAKERS = /^(SETRA|NEOPLAN|VAN HOOL|IRIZAR|TEMSA|VDL)/;
const CITY_MAKERS = /^(SOLARIS|HEULIEZ|EBUSCO|KARSAN)/;
const CITY_WORDS = /CITARO|CONECTO|URBINO|URBANWAY|CITEA|LION'?S CITY|CITY ?BUS|STADTBUS|LINIENBUS|7900|EXQUICITY|CITELIS|ROUTEMASTER/;
const COACH_WORDS = /TOURISMO|TRAVEGO|INTOURO|CROSSWAY|LION'?S COACH|TOURLINER|SKYLINER|CITYLINER|COMFORTCLASS|TOPCLASS|MULTICLASS|9700|9900|EVADYS|MAGELYS|COACH|REISEBUS/;

export interface HeavySpec {
  kind: HeavyKind;
  cab: CabStyle;
  axles: 2 | 3;
  // "table": the row of HEAVY_ROWS; "name": guessed from maker and name;
  // "chosen": the kind picked in the view.
  source: "table" | "name" | "chosen";
}

export function heavyRow(brand: string, model: string): HeavyRow | null {
  return HEAVY_ROWS.find((r) => r.brand === brand && r.model === model) ?? null;
}

// The kind, cab and axles of a truck or bus; a kind picked in the view wins.
export function heavySpec(brand: string, model: string, chosen?: HeavyKind): HeavySpec {
  const row = heavyRow(brand, model);
  const name = model.toUpperCase();
  const maker = brand.toUpperCase();
  const named: HeavyKind = COACH_WORDS.test(name) ? "coach" : CITY_WORDS.test(name) ? "citybus" : COACH_MAKERS.test(maker) ? "coach" : CITY_MAKERS.test(maker) ? "citybus" : "tractor";
  const kind = chosen ?? row?.kind ?? named;
  const us = US_MAKERS.includes(brand);
  const bus = kind === "citybus" || kind === "coach";
  const axles: 2 | 3 = row && row.kind === kind ? (row.axles ?? 2) : kind === "rigid" || (us && kind === "tractor") ? 3 : 2;
  return { kind, cab: bus || !us ? "cabover" : "conventional", axles, source: chosen ? "chosen" : row ? "table" : "name" };
}

// --- Dimensions ---------------------------------------------------------------------

interface Dims {
  L: number;
  W: number;
  H: number;
  // Front axle to the first rear axle, and from there to the second.
  wb: number;
  tandem: number;
  wheel: number;
  frontTrack: number;
  rearTrack: number;
}

// Typical values of each kind (manufacturers' data sheets of common
// configurations): a 4x2 tractor with a high sleeper cab, a 6x4 American
// tractor with a 72-inch sleeper, a 6x4 rigid with a box body, a 12 m
// low-floor city bus and a 12 m coach. 315/80 R22.5 tyres on trucks and
// coaches, 275/70 R22.5 on city buses.
function typicalDims(spec: HeavySpec): Dims {
  const tandem = spec.axles === 3 ? 1.35 : 0;
  switch (spec.kind) {
    case "citybus":
      return { L: 12.0, W: 2.55, H: 3.05, wb: 5.9, tandem: 0, wheel: 0.96, frontTrack: 2.1, rearTrack: 1.84 };
    case "coach":
      return spec.axles === 3
        ? { L: 13.9, W: 2.55, H: 3.85, wb: 6.6, tandem: 1.45, wheel: 1.05, frontTrack: 2.1, rearTrack: 1.84 }
        : { L: 12.2, W: 2.55, H: 3.75, wb: 6.2, tandem: 0, wheel: 1.05, frontTrack: 2.1, rearTrack: 1.84 };
    case "rigid":
      return spec.cab === "conventional"
        ? { L: 9.6, W: 2.59, H: 3.9, wb: 5.2, tandem, wheel: 1.03, frontTrack: 2.08, rearTrack: 1.83 }
        : { L: spec.axles === 3 ? 9.0 : 8.5, W: 2.55, H: 3.6, wb: spec.axles === 3 ? 4.5 : 4.8, tandem, wheel: 1.07, frontTrack: 2.05, rearTrack: 1.83 };
    default:
      return spec.cab === "conventional"
        ? { L: spec.axles === 3 ? 8.4 : 7.4, W: 2.59, H: 4.1, wb: spec.axles === 3 ? 5.19 : 5.0, tandem, wheel: 1.03, frontTrack: 2.08, rearTrack: 1.83 }
        : { L: spec.axles === 3 ? 6.9 : 6.05, W: 2.5, H: 3.9, wb: spec.axles === 3 ? 3.6 : 3.7, tandem, wheel: 1.07, frontTrack: 2.05, rearTrack: 1.83 };
  }
}

// The catalog's values where it has them (measuredFields), the typical ones
// of the kind for the rest: the catalog's typical truck is a van.
function heavyDims(a: VehicleAnatomy, spec: HeavySpec): Dims {
  const t = typicalDims(spec);
  if (a.dimensionSource === "typical") return t;
  const bit = (b: number) => (a.measuredFields & b) !== 0;
  const d = { ...t };
  if (bit(1)) d.L = Math.max(5, a.lengthMm / 1000);
  if (bit(2)) d.W = Math.min(2.6, Math.max(2.2, a.widthMm / 1000));
  if (bit(4)) d.H = Math.max(2.6, a.heightMm / 1000);
  if (bit(8)) d.wb = a.wheelbaseMm / 1000;
  return d;
}

// --- The vehicle in numbers ---------------------------------------------------------------

interface Plan {
  spec: HeavySpec;
  L: number;
  W: number;
  H: number;
  xF: number;
  xR: number;
  frontAxle: number;
  rearAxles: number[];
  r: number;
  // Bottom of the envelope.
  clearance: number;
  // Truck: the back wall of the cab, its height and floor, the top of the
  // frame rails and of the envelope over the open frame.
  cabBack: number;
  cabHeight: number;
  floor: number;
  frameTop: number;
  chassisTop: number;
  // Base of the windscreen, its top, and where the bonnet of a conventional
  // starts (the cab's front for a cab-over).
  cowlX: number;
  shieldTop: number;
  glassTop: number;
  belt: number;
  beltRear: number;
  // Bus doors: from x to x.
  doors: [number, number][];
  driverZ: number;
}

function plan(a: VehicleAnatomy, spec: HeavySpec): { p: Plan; d: Dims } {
  const d = heavyDims(a, spec);
  const { L, W, H } = d;
  const xF = L / 2;
  const xR = -L / 2;
  const r = d.wheel / 2;
  const bus = spec.kind === "citybus" || spec.kind === "coach";
  const conventional = spec.cab === "conventional" && !bus;
  // Front overhang of the kind, kept so the last axle clears the rear end.
  const wanted = spec.kind === "citybus" ? 2.7 : spec.kind === "coach" ? 2.65 : conventional ? 0.95 : 1.45;
  const span = d.wb + d.tandem;
  const fo = Math.max(r + 0.2, Math.min(wanted, L - span - r - 0.15));
  const frontAxle = xF - fo;
  const rearAxles = [frontAxle - d.wb];
  if (d.tandem > 0) rearAxles.push(frontAxle - d.wb - d.tandem);
  const base = {
    spec, L, W, H, xF, xR, frontAxle, rearAxles, r, driverZ: 0.6, doors: [] as [number, number][],
  };
  if (spec.kind === "citybus") {
    const rearAxle = rearAxles[0]!;
    const mid = (frontAxle + rearAxle) / 2 + 0.3;
    return {
      d,
      p: {
        ...base,
        clearance: 0.22, cabBack: xR, cabHeight: H, floor: 0.34, frameTop: 0.34, chassisTop: H,
        cowlX: xF + 0.04, shieldTop: H - 0.3, glassTop: H - 0.3, belt: 0.95, beltRear: 1.15,
        doors: [[xF - 1.55, xF - 0.32], [mid - 0.62, mid + 0.62], [rearAxle - r - 1.45, rearAxle - r - 0.2]],
      },
    };
  }
  if (spec.kind === "coach") {
    const rearAxle = rearAxles[0]!;
    return {
      d,
      p: {
        ...base,
        clearance: 0.3, cabBack: xR, cabHeight: H, floor: 1.25, frameTop: 1.15, chassisTop: H,
        cowlX: xF + 0.1, shieldTop: H - 0.42, glassTop: H - 0.42, belt: 1.3, beltRear: 1.9,
        doors: [[xF - 1.45, xF - 0.55], [rearAxle + r + 0.25, rearAxle + r + 1.2]],
      },
    };
  }
  const rigid = spec.kind === "rigid";
  const frameTop = 1.05;
  if (conventional) {
    // A long bonnet, the cab behind it, a sleeper with a raised roof on a
    // tractor, a day cab on a rigid.
    const cowlX = xF - 2.15;
    return {
      d,
      p: {
        ...base,
        clearance: 0.3, cabBack: cowlX - (rigid ? 1.45 : 2.85), cabHeight: rigid ? Math.min(H - 0.3, 3.05) : H, floor: 1.3, frameTop,
        chassisTop: rigid ? H : frameTop + 0.36, cowlX, shieldTop: 2.95, glassTop: 2.92, belt: 2.0, beltRear: 2.05,
      },
    };
  }
  // A cab-over: the cab sits over the front axle and the engine; a high
  // sleeper on a tractor, a day cab on a rigid.
  const cabLength = rigid ? 1.75 : 2.3;
  return {
    d,
    p: {
      ...base,
      clearance: 0.3, cabBack: xF - cabLength, cabHeight: rigid ? Math.min(H - 0.45, 3.15) : H, floor: 1.35, frameTop,
      chassisTop: rigid ? H : frameTop + 0.36, cowlX: xF + 0.1, shieldTop: 2.95, glassTop: 2.92, belt: 2.0, beltRear: 2.0,
    },
  };
}

const lamp = (width: number, wrap: number, drop: number, height: number, round: number): Lamp => ({ width, wrap, drop, height, round, rise: 0, twin: false, bar: false });
const grille = (width: number, height: number, drop: number, openings: 0 | 1, intakeWidth = 0, intakeHeight = 0): Grille => ({ width, height, drop, round: 8, openings, intakeWidth, intakeHeight });

// The profile of the cab or the bus body for the hull and its glass.
function heavyProfile(p: Plan): Profile {
  const { L, H, xF, xR, frontAxle, r } = p;
  const station = (x: number) => (x - xR) / L;
  const rear = p.rearAxles[p.rearAxles.length - 1]!;
  const pts: [number, number][] = [];
  const seams: Seam[] = [];
  const pillars: number[] = [];
  let roofFront: number;
  let roofRear: number;
  let dlo: [number, number, number, number];
  let headlight: Lamp;
  let taillight: Lamp;
  let front: Grille;
  let noseTop: number;
  let tailTop: number;
  let beltRun: number | undefined;
  let open = 0;
  let rearFace = 0;
  const kind = p.spec.kind;
  if (kind === "citybus" || kind === "coach") {
    const coach = kind === "coach";
    // A box with rounded roof edges; the coach's windscreen leans back to
    // its base over the front panel, the city bus's stands upright.
    pts.push([xR, H - 0.3], [xR + 0.1, H - 0.1], [xR + 0.4, H]);
    if (coach) {
      pts.push([xF - 0.95, H], [xF - 0.68, H - 0.12], [xF - 0.48, p.shieldTop + 0.04], [xF - 0.06, p.belt + 0.02], [xF, p.belt - 0.1]);
      roofFront = station(xF - 0.6);
      beltRun = station(xF - 1.6);
      noseTop = p.belt - 0.12;
      headlight = lamp(0.45, 0.08, 0.42, 0.15, 6);
      front = grille(0.5, 0.06, 0.6, 0, 0.55, 0.08);
    } else {
      pts.push([xF - 0.35, H], [xF - 0.12, H - 0.07], [xF - 0.02, H - 0.24], [xF, H - 0.34]);
      roofFront = station(xF - 0.32);
      beltRun = station(xF - 0.7);
      noseTop = p.belt - 0.03;
      headlight = lamp(0.36, 0.08, 0.28, 0.16, 6);
      front = grille(0.5, 0.06, 0.6, 0, 0.6, 0.07);
    }
    roofRear = station(xR + 0.3);
    // Tall tail light strips at the rear corners.
    tailTop = coach ? 2.0 : 1.7;
    taillight = lamp(0.13, 0.07, 0.1, coach ? 1.0 : 0.85, 8);
    dlo = [station(xF - (coach ? 0.5 : 0.28)), station(xF - (coach ? 0.55 : 0.3)), station(xR + (coach ? 0.7 : 0.95)), station(xR + (coach ? 0.75 : 0.95))];
    // Pillars at the door edges, then between the windows every 1.5 m or so.
    const edges = p.doors.flatMap(([a, b]) => [a, b]);
    pillars.push(...edges);
    const stops = [xR + (coach ? 0.7 : 0.95), ...edges, xF - 0.3].sort((x, y) => x - y);
    for (let i = 0; i + 1 < stops.length; i++) {
      const a = stops[i]!;
      const b = stops[i + 1]!;
      if (p.doors.some(([d0, d1]) => Math.abs(d0 - a) < 1e-6 && Math.abs(d1 - b) < 1e-6)) continue;
      const n = Math.floor((b - a) / 1.45);
      for (let k = 1; k <= n && pillars.length < MAX_PILLARS; k++) pillars.push(a + ((b - a) * k) / (n + 1));
    }
    for (const [a, b] of p.doors) {
      for (const x of [a, b, (a + b) / 2]) seams.push({ x, top: p.glassTop - 0.02, bottom: p.clearance + 0.1 });
    }
    if (coach) {
      // The flaps of the luggage holds between the axles.
      const from = frontAxle - r - 0.25;
      const to = Math.max(p.doors[1]![1] + 0.1, rear + r + 0.25);
      const n = Math.max(1, Math.round((from - to) / 1.6));
      for (let k = 0; k <= n; k++) seams.push({ x: from - ((from - to) * k) / n, top: p.floor - 0.12, bottom: p.clearance + 0.12 });
    }
  } else {
    const conventional = p.spec.cab === "conventional";
    // Behind the cab the open frame: the envelope at the height of the
    // fifth wheel and the mudguards, or of a rigid's box body.
    pts.push([xR, p.chassisTop - 0.02], [p.cabBack - 0.12, p.chassisTop], [p.cabBack, p.cabHeight - 0.15]);
    if (conventional) {
      // Sleeper roof down to the cab roof, the windscreen, the bonnet that
      // falls a little to the grille.
      const cab = Math.min(3.05, p.cabHeight);
      const fairing = p.cabHeight - cab > 0.2;
      pts.push([p.cabBack + 0.12, p.cabHeight]);
      if (fairing) pts.push([p.cowlX - 1.75, p.cabHeight - 0.02], [p.cowlX - 1.15, cab + 0.5], [p.cowlX - 0.66, cab + 0.08]);
      pts.push([p.cowlX - 0.52, cab], [p.cowlX - 0.42, p.shieldTop], [p.cowlX, p.belt], [p.cowlX + 0.5, p.belt - 0.12], [xF - 0.15, 1.72], [xF - 0.03, 1.62], [xF, 1.5]);
      roofFront = station(p.cowlX - 0.55);
      noseTop = 1.6;
      headlight = lamp(0.32, 0.12, 0.2, 0.16, 5);
      front = grille(0.42, 0.85, 0.06, 1);
      dlo = [station(p.cowlX - 0.06), station(p.cowlX - 0.48), station(p.cowlX - 1.2), station(p.cowlX - 1.2)];
      seams.push({ x: p.cowlX - 0.04, top: p.belt, bottom: 1.05 }, { x: p.cowlX - 1.22, top: p.glassTop + 0.03, bottom: 1.05 });
      // The bonnet's hinge line over the front wheel.
      seams.push({ x: p.cowlX + 0.05, top: p.belt - 0.03, bottom: 1.1 });
    } else {
      // The front of the cab: the roof cap over the windscreen, the windscreen
      // leaning back a little, the flat face below with grille and lamps.
      pts.push([p.cabBack + 0.12, p.cabHeight], [xF - 0.75, p.cabHeight - 0.01], [xF - 0.5, p.cabHeight - 0.1]);
      if (p.cabHeight > p.shieldTop + 0.35) pts.push([xF - 0.4, p.shieldTop + 0.16]);
      pts.push([xF - 0.3, p.shieldTop], [xF - 0.04, p.belt], [xF, p.belt - 0.12]);
      roofFront = station(xF - 0.42);
      noseTop = p.belt - 0.14;
      headlight = lamp(0.34, 0.06, 0.88, 0.2, 6);
      front = grille(0.62, 0.55, 0.06, 1);
      dlo = [station(xF - 0.12), station(xF - 0.34), station(xF - 1.24), station(xF - 1.24)];
      seams.push({ x: xF - 0.1, top: p.belt - 0.02, bottom: 1.05 }, { x: xF - 1.26, top: p.glassTop + 0.03, bottom: 1.05 });
    }
    roofRear = station(p.cabBack);
    tailTop = p.frameTop;
    taillight = lamp(0.2, 0.02, 0.1, 0.12, 8);
    open = station(p.cabBack);
    // The lamps of a truck are on its frame, not on the hull.
    rearFace = -1;
  }
  const top: [number, number][] = [];
  for (const [x, y] of pts.sort((m, n) => m[0] - n[0])) {
    const u = Math.min(1, Math.max(0, station(x)));
    const last = top[top.length - 1];
    if (last && u - last[0] < 0.003) continue;
    top.push([u, Math.min(1, y / H)]);
  }
  top[0]![0] = 0;
  top[top.length - 1]![0] = 1;
  const kept = pillars.filter((x) => station(x) > dlo[2] + 0.004 && station(x) < dlo[0] - 0.004).sort((m, n) => m - n).slice(0, MAX_PILLARS);
  return {
    top,
    belt: p.belt / H,
    beltRear: p.beltRear / H,
    // The windscreen ends at the window line below: a conventional's cowl
    // station a little over the bonnet keeps its base sharp.
    cowl: station(p.cowlX + (p.spec.cab === "conventional" && kind !== "citybus" && kind !== "coach" ? 0.1 : 0)),
    roofFront,
    roofRear,
    // No rear window: a sleeper's back wall, a bus's engine bay.
    tail: roofRear,
    dlo,
    pillars: kept.map(station),
    pillarWidths: kept.map((x) => (p.doors.some(([a, b]) => Math.abs(x - a) < 1e-6 || Math.abs(x - b) < 1e-6) ? 0.09 : 0.07) / L),
    kink: 0,
    seams,
    clearance: p.clearance,
    rearSeats: false,
    tumblehome: kind === "coach" ? 0.04 : 0.025,
    noseRound: p.spec.cab === "conventional" && kind !== "citybus" && kind !== "coach" ? 6 : 12,
    tailRound: 20,
    rearLift: station(rear - r * 0.9),
    frontLift: station(frontAxle + r * 0.9),
    headlight,
    taillight,
    grille: front,
    noseTop,
    tailTop,
    open,
    bed: null,
    spare: null,
    rearFace,
    glassTop: p.glassTop,
    rail: null,
    beltRun,
    // Flat sides and a flat roof with tight edges.
    section: kind === "coach" ? 9 : 11,
  };
}

// The glass shader takes this many pillars.
export const MAX_PILLARS = 16;

// --- The vehicle -------------------------------------------------------------------------

export function buildHeavyLayout(input: XrayInput): CarLayout {
  const a = input.anatomy;
  const spec = heavySpec(a.brand, a.model, input.heavyKind);
  const { p, d } = plan(a, spec);
  const profile = heavyProfile(p);
  const hull = makeHull(p.L, p.W, p.H, profile);
  const { xF, xR, W, frontAxle, rearAxles, r } = p;
  const bus = spec.kind === "citybus" || spec.kind === "coach";
  const truck = !bus;
  const conventional = truck && spec.cab === "conventional";
  const combustion = a.engine !== EngineLayout.None;
  const lhd = 1;
  const parts: Part[] = [];
  const tubes: Tube[] = [];
  const anchors: Record<string, Vec3> = {};
  const rear0 = rearAxles[0]!;
  const rearLast = rearAxles[rearAxles.length - 1]!;
  const frameHalf = 0.43;

  // --- Wheels: single tyres in front, twins on the driven rear axles (a
  // coach's trailing axle runs on single tyres).
  const frontTyre = bus ? 0.3 : 0.32;
  const twin = bus ? 0.6 : 0.64;
  const frontTrack = Math.min(d.frontTrack, W - frontTyre - 0.06);
  const rearTrack = Math.min(d.rearTrack, W - twin - 0.06);
  const wheels: Wheel[] = [];
  for (const side of [1, -1]) wheels.push({ center: [frontAxle, r, (side * frontTrack) / 2], radius: r, width: frontTyre, front: true, driven: false });
  rearAxles.forEach((x, i) => {
    const tag = spec.kind === "coach" && i > 0;
    const driven = i === 0 || (truck && spec.axles === 3 && (spec.kind === "rigid" || conventional));
    for (const side of [1, -1]) {
      wheels.push(tag
        ? { center: [x, r, (side * frontTrack) / 2], radius: r, width: frontTyre, front: false, driven: false }
        : { center: [x, r, (side * rearTrack) / 2], radius: r, width: twin, front: false, driven, dual: true });
    }
  });

  // --- Frame and body.
  const frameFrom = truck ? xR + 0.12 : xR + 0.4;
  const frameTo = truck ? xF - 0.35 : xF - 0.5;
  if (truck) {
    for (const side of [1, -1]) parts.push(box(`frame-rail-${side}`, "body", "metal", [(frameFrom + frameTo) / 2, p.frameTop - 0.14, side * frameHalf], [frameTo - frameFrom, 0.28, 0.07]));
    const cross = [frameTo - 0.05, frontAxle - r - 0.3, (frontAxle + rear0) / 2, ...rearAxles.map((x) => x + r + 0.12), frameFrom + 0.05];
    cross.forEach((x, i) => parts.push(box(`frame-cross-${i}`, "body", "metal", [x, p.frameTop - 0.16, 0], [0.08, 0.18, 2 * frameHalf])));
    // Mudguards over the rear wheels, flat plates on their brackets.
    const guardFrom = rearLast - r - 0.1;
    const guardTo = rear0 + r + 0.1;
    for (const side of [1, -1]) {
      parts.push({ ...box(`mudguard-${side}`, "body", "shell", [(guardFrom + guardTo) / 2, 2 * r + 0.07, (side * rearTrack) / 2], [guardTo - guardFrom, 0.04, twin + 0.1]), round: 0.3 });
    }
    // The tail lights and the underrun guard at the end of the frame.
    for (const side of [1, -1]) parts.push(box(`tail-light-${side}`, "body", "rearlamp", [xR + 0.08, p.frameTop - 0.24, side * (W / 2 - 0.3)], [0.06, 0.14, 0.42]));
    parts.push(box("underrun-guard", "body", "metal", [xR + 0.14, 0.58, 0], [0.1, 0.1, W - 0.3]));
    if (spec.kind === "tractor") {
      // The fifth wheel a little ahead of the rear axle (of the tandem's middle).
      const fx = (rear0 + rearLast) / 2 + 0.3;
      parts.push(box("fifth-wheel-mount", "body", "metal", [fx, p.frameTop + 0.06, 0], [0.9, 0.12, 0.95]));
      parts.push({ ...cylinder("fifth-wheel", "body", "metal", "y", [fx, p.frameTop + 0.17, 0], 0.08, 0.95), round: 0.1 });
    } else {
      // The box body on its subframe behind the cab.
      const from = p.cabBack - 0.12;
      const to = xR + 0.04;
      const bottom = p.frameTop + 0.1;
      parts.push({ ...box("cargo-box", "body", "shell", [(from + to) / 2, (bottom + p.H - 0.02) / 2, 0], [from - to, p.H - 0.02 - bottom, W - 0.04]), round: 0.02 });
      parts.push(box("subframe", "body", "metal", [(from + to) / 2, p.frameTop + 0.05, 0], [from - to, 0.1, 2 * frameHalf + 0.08]));
    }
  }

  // --- Cab or bus interior: dashboard, driver's seat with the steering
  // wheel, the passenger seat or the bus's rows, a truck's bunk.
  const cabFront = conventional ? p.cowlX : xF;
  const dashX = cabFront - (conventional ? 0.3 : bus ? 0.45 : 0.42);
  const driverFloor = spec.kind === "coach" ? 1.0 : p.floor;
  parts.push(box("dashboard", "body", "cabin", [dashX, driverFloor + 0.62, bus ? 0.55 : 0], [0.36, 0.42, bus ? 1.0 : W * 0.8], [CarZone.Cabin]));
  const seatX = dashX - 0.62;
  for (const side of truck ? [1, -1] : [1]) {
    parts.push(box(`seat-front-${side}`, "body", "cabin", [seatX, driverFloor + 0.45, side * p.driverZ], [0.5, 0.14, 0.5], [CarZone.Cabin]));
    parts.push(box(`back-front-${side}`, "body", "cabin", [seatX - 0.28, driverFloor + 0.8, side * p.driverZ], [0.12, 0.62, 0.48], [CarZone.Cabin]));
  }
  parts.push({ ...cylinder("steering-wheel", "body", "cabin", "y", [dashX - 0.24, driverFloor + 0.85, lhd * p.driverZ], 0.04, 0.48, [CarZone.Cabin]), tilt: -0.5 });
  if (spec.kind === "tractor") parts.push(box("bunk", "body", "cabin", [p.cabBack + 0.42, p.floor + 0.5, 0], [0.7, 0.14, W - 0.4], [CarZone.Cabin]));
  if (bus) {
    // Double seats on both sides from behind the front door to the rear,
    // leaving the doors free; a city bus's rear rows stand on the podium.
    const seatFloor = (x: number) => (spec.kind === "coach" ? p.floor : x < rear0 + r ? 0.78 : p.floor);
    const first = spec.kind === "coach" ? xF - 1.75 : frontAxle - r - 0.35;
    let n = 0;
    for (let x = first; x > xR + 0.9 && n < 14; x -= 0.85) {
      if (p.doors.some(([a, b]) => x > a - 0.35 && x < b + 0.35)) continue;
      if (spec.kind === "citybus" && Math.abs(x - frontAxle) < r + 0.3) continue;
      for (const side of [1, -1]) {
        if (spec.kind === "citybus" && side === -1 && x > rear0 + r + 0.3 && x < frontAxle - r - 0.3 && n % 3 === 1) continue;
        parts.push(box(`row-${n}-${side}`, "body", "cabin", [x, seatFloor(x) + 0.45, side * 0.68], [0.45, 0.12, 0.95], [CarZone.Cabin]));
        parts.push(box(`row-back-${n}-${side}`, "body", "cabin", [x - 0.24, seatFloor(x) + 0.78, side * 0.68], [0.1, 0.56, 0.95], [CarZone.Cabin]));
      }
      n++;
    }
  }
  anchors.cabin = [dashX, driverFloor + 0.9, 0];

  // --- Drivetrain: an inline six along the vehicle, under a cab-over's cab,
  // under a conventional's bonnet, at the rear of a bus; the gearbox
  // behind it, the prop shaft to the driven axles.
  let engine: Vec3;
  const block: Vec3 = [1.3, 0.62, 0.56];
  if (bus) engine = spec.kind === "coach" ? [xR + 1.3, 0.95, 0] : [xR + 0.95, 0.8, 0.3];
  else engine = conventional ? [frontAxle - 0.2, 1.08, 0] : [frontAxle - 0.25, 0.98, 0];
  const engineFront = bus ? -1 : 1;
  const gearbox: Vec3 = [engine[0] - engineFront * (block[0] / 2 + 0.45), engine[1] - 0.12, engine[2]];
  const exhaustSide = -1;
  if (combustion) {
    parts.push(box("engine", "drivetrain", "engine", engine, block, [CarZone.Cylinders]));
    const headY = engine[1] + block[1] / 2 + 0.08;
    parts.push(box("cylinder-head", "drivetrain", "metal", [engine[0], headY, engine[2]], [block[0] * 0.9, 0.16, block[2] * 0.85], [CarZone.Cylinders]));
    // Six injectors along the head.
    for (let i = 0; i < 6; i++) {
      parts.push(cylinder(`coil-${i}`, "drivetrain", "electric", "y", [engine[0] + (i - 2.5) * block[0] * 0.15, headY + 0.11, engine[2]], 0.1, 0.045, [CarZone.Ignition]));
    }
    const manifold: Vec3 = [engine[0], headY - 0.06, engine[2] + block[2] / 2 + 0.07];
    parts.push(box("intake-manifold", "drivetrain", "metal", manifold, [block[0] * 0.85, 0.16, 0.1], [CarZone.Intake]));
    const rail: Vec3 = [manifold[0], manifold[1] - 0.12, manifold[2] - 0.03];
    parts.push(cylinder("fuel-rail", "drivetrain", "fuel", "x", rail, block[0] * 0.75, 0.035, [CarZone.Injection]));
    const turbo: Vec3 = [engine[0] + engineFront * 0.15, engine[1] + 0.18, engine[2] + exhaustSide * (block[2] / 2 + 0.14)];
    parts.push(cylinder("turbo", "drivetrain", "metal", "z", turbo, 0.16, 0.26, [CarZone.Turbo]));
    // The air filter: behind the cab on the frame, under a conventional's
    // bonnet, in a bus's engine bay.
    const airbox: Vec3 = bus ? [engine[0] + 0.1, engine[1] + 0.7, -0.5] : conventional ? [p.cowlX + 0.45, 1.6, 0.6] : [p.cabBack - 0.35, p.frameTop + 0.2, 0.25];
    parts.push(cylinder("airbox", "drivetrain", "metal", bus || conventional ? "x" : "y", airbox, bus || conventional ? 0.6 : 0.32, 0.36, [CarZone.Intake]));
    tubes.push({ id: "intake-pipe", layer: "drivetrain", points: [airbox, [airbox[0], Math.max(airbox[1], turbo[1]) + 0.05, (airbox[2] + turbo[2]) / 2], turbo], radius: 0.05, material: "metal", zones: [CarZone.Intake] });
    anchors.intake = airbox;
    anchors.injection = rail;
    anchors.ignition = [engine[0], headY + 0.18, engine[2]];
    anchors.engine = [engine[0], headY + 0.12, engine[2]];
    anchors.turbo = turbo;
    parts.push(cylinder("gearbox", "drivetrain", "gearbox", "x", gearbox, 0.85, 0.48, [CarZone.Gearbox]));
    anchors.gearbox = [gearbox[0], gearbox[1] + 0.3, gearbox[2]];
  } else {
    // An electric truck or bus: the motor drives the rear axle.
    const motor: Vec3 = [rear0 + (bus ? -0.75 : 0.75), r + 0.1, bus ? 0.3 : 0];
    parts.push(cylinder("motor-0", "drivetrain", "motor", "x", motor, 0.7, 0.42, []));
    parts.push(box("inverter-0", "energy", "hv", [motor[0], motor[1] + 0.38, motor[2]], [0.5, 0.14, 0.42], []));
    anchors.motor = [motor[0], motor[1] + 0.25, motor[2]];
    anchors.engine = anchors.motor;
    anchors.gearbox = anchors.motor;
  }
  // Axles: the front beam, the rear axle housings with their differentials.
  tubes.push({ id: "front-axle", layer: "drivetrain", points: [[frontAxle, r - 0.05, frontTrack / 2 - 0.2], [frontAxle, r - 0.05, -frontTrack / 2 + 0.2]], radius: 0.06, material: "metal", zones: [CarZone.Wheels] });
  rearAxles.forEach((x, i) => {
    const driven = wheels.some((w) => !w.front && w.center[0] === x && w.driven);
    tubes.push({ id: `rear-axle-${i}`, layer: "drivetrain", points: [[x, r, rearTrack / 2 - 0.25], [x, r, -rearTrack / 2 + 0.25]], radius: 0.075, material: "metal", zones: [CarZone.Gearbox], explode: [0, 0.05, 0] });
    if (driven) parts.push(box(`rear-diff-${i}`, "drivetrain", "gearbox", [x, r, bus ? 0.3 : 0], [0.4, 0.4, 0.42], [CarZone.Gearbox]));
  });
  const drivenAxles = rearAxles.filter((x) => wheels.some((w) => !w.front && w.center[0] === x && w.driven));
  if (combustion) {
    const start: Vec3 = [gearbox[0] - engineFront * 0.45, gearbox[1], gearbox[2]];
    const prop = [start, ...drivenAxles.map((x): Vec3 => [x + (bus ? -0.2 : 0.2), r + 0.02, bus ? 0.3 : 0])];
    tubes.push({ id: "prop-shaft", layer: "drivetrain", points: dedupe(prop), radius: 0.05, material: "metal", zones: [CarZone.Gearbox] });
  }

  // --- Exhaust: from the turbo down to the aftertreatment box (particle
  // filter and SCR catalyst in one) on the right frame side, the tail
  // pipe out of it.
  const tankY = truck ? p.frameTop - 0.32 : bus ? 0.62 : 0.6;
  const besideRail = frameHalf + 0.38;
  const behindFront = frontAxle - r - 0.12;
  let catalyst: Vec3;
  if (bus) catalyst = [xR + 0.9, engine[1] + 0.85, -0.65];
  else if (conventional) catalyst = [behindFront - 1.95, tankY + 0.02, -besideRail];
  else catalyst = [behindFront - 0.55, tankY + 0.04, -besideRail];
  if (combustion) {
    const turbo = anchors.turbo!;
    const down: Vec3 = bus ? [turbo[0], turbo[1] + 0.3, turbo[2] - 0.1] : [turbo[0] - 0.1, tankY + 0.3, turbo[2] - 0.1];
    const tail: Vec3 = bus ? [xR + 0.25, catalyst[1] + 0.25, -0.95] : [catalyst[0] - 0.65, tankY - 0.25, -besideRail - 0.1];
    tubes.push({ id: "exhaust", layer: "drivetrain", points: [turbo, down, [catalyst[0] + 0.55, catalyst[1], catalyst[2] + 0.2], catalyst, [catalyst[0] - 0.5, catalyst[1] - 0.05, catalyst[2]], tail], radius: 0.05, material: "exhaust", zones: [CarZone.Exhaust] });
    parts.push(box("catalyst", "drivetrain", "exhaust", catalyst, [0.95, 0.55, 0.55], [CarZone.Exhaust]));
    anchors.exhaust = catalyst;
  }

  // --- Cooling: the big radiator behind the grille, at the rear side wall
  // of a bus.
  const radiator: Vec3 = bus ? [xR + 1.35, 1.55, W / 2 - 0.22] : conventional ? [xF - 0.35, 1.15, 0] : [xF - 0.3, 1.35, 0];
  const radSize: Vec3 = bus ? [1.0, 0.8, 0.07] : [0.08, 0.95, 1.05];
  parts.push(box("radiator", "cooling", "cooling", radiator, radSize, [CarZone.Cooling]));
  parts.push(cylinder("fan", "cooling", "cooling", bus ? "z" : "x", bus ? [radiator[0], radiator[1], radiator[2] - 0.12] : [radiator[0] - 0.12, radiator[1], 0], 0.07, 0.75, [CarZone.Cooling]));
  anchors.cooling = [radiator[0], radiator[1] + 0.4, radiator[2]];
  const hoseTo: Vec3 = combustion ? [engine[0] + engineFront * block[0] * 0.45, engine[1] + 0.15, engine[2]] : anchors.motor!;
  tubes.push({ id: "hose-upper", layer: "cooling", points: [[radiator[0] - (bus ? 0 : 0.05), radiator[1] + 0.3, radiator[2] - (bus ? 0.1 : 0)], [hoseTo[0], radiator[1] + 0.3, (radiator[2] + hoseTo[2]) / 2], hoseTo], radius: 0.035, material: "cooling", zones: [CarZone.Cooling] });
  tubes.push({ id: "hose-lower", layer: "cooling", points: [[radiator[0] - (bus ? 0 : 0.05), radiator[1] - 0.35, radiator[2] - (bus ? 0.1 : 0)], [hoseTo[0], hoseTo[1] - 0.3, (radiator[2] + hoseTo[2]) / 2], [hoseTo[0], hoseTo[1] - 0.25, hoseTo[2]]], radius: 0.035, material: "cooling", zones: [CarZone.Cooling] });
  const expansion: Vec3 = bus ? [radiator[0], radiator[1] + 0.6, radiator[2] - 0.2] : conventional ? [p.cowlX + 0.25, 1.75, -0.6] : [p.cabBack + 0.25, p.floor - 0.12, -0.55];
  parts.push(box("expansion-tank", "cooling", "cooling", expansion, [0.22, 0.16, 0.16], [CarZone.Cooling]));

  // --- Energy: fuel tank on the left frame side (both sides on an American
  // tractor), the AdBlue tank behind it; a bus's tanks behind the front
  // axle; an electric truck's packs along the frame.
  let fuel: Vec3 | null = null;
  if (a.fuelTank) {
    if (bus) fuel = spec.kind === "coach" ? [frontAxle - r - 0.55, 0.75, 0] : [frontAxle - r - 0.45, 0.62, -0.85];
    else fuel = [behindFront - (conventional ? 0.8 : 0.66), tankY, besideRail];
    const tankSize: [number, number, number] = bus ? (spec.kind === "coach" ? [0.6, 0.55, 1.2] : [0.6, 0.55, 0.45]) : [conventional ? 1.55 : 1.3, 0.62, 0.62];
    if (bus) parts.push(box("fuel-tank", "energy", "fuel", fuel, tankSize, [CarZone.FuelTank]));
    else parts.push({ ...cylinder("fuel-tank", "energy", "fuel", "x", fuel, tankSize[0], tankSize[1], [CarZone.FuelTank]), round: 0.3 });
    if (conventional) parts.push(cylinder("fuel-tank-2", "energy", "fuel", "x", [behindFront - 0.6, tankY, -besideRail], 1.15, 0.62, [CarZone.FuelTank]));
    anchors.fuel = [fuel[0], fuel[1] + 0.35, fuel[2]];
    if (combustion) {
      tubes.push({ id: "fuel-line", layer: "energy", points: [fuel, [fuel[0], tankY - 0.1, frameHalf - 0.05], [engine[0], tankY - 0.1, frameHalf - 0.05], [engine[0] - engineFront * 0.3, engine[1], block[2] / 2 + 0.05]], radius: 0.012, material: "fuel", zones: [CarZone.FuelTank, CarZone.Injection] });
      const adblue: Vec3 = bus ? [xR + 2.0, 0.7, -0.85] : [fuel[0] - tankSize[0] / 2 - 0.35, tankY + 0.02, besideRail - 0.02];
      parts.push(box("adblue-tank", "energy", "fuel", adblue, [0.45, 0.5, 0.42], [CarZone.Exhaust]));
      tubes.push({ id: "adblue-line", layer: "energy", points: [adblue, [adblue[0], adblue[1] - 0.2, 0], [catalyst[0], catalyst[1] - 0.2, 0], catalyst], radius: 0.01, material: "fuel", zones: [CarZone.Exhaust] });
    }
  }
  if (!combustion) {
    const packs: Vec3[] = bus ? [[0, p.H - 0.2, 0.5], [0, p.H - 0.2, -0.5]] : [[behindFront - 0.8, tankY, besideRail], [behindFront - 0.8, tankY, -besideRail]];
    packs.forEach((q, i) => parts.push(box(i ? `hv-battery-${i}` : "hv-battery", "energy", "battery", q, bus ? [5, 0.3, 0.8] : [1.5, 0.62, 0.6], [CarZone.Electrics])));
    anchors.hv = [packs[0]![0], packs[0]![1] + 0.35, packs[0]![2]];
    tubes.push({ id: "hv-cable-0", layer: "energy", points: [packs[0]!, [packs[0]![0], tankY, 0.2], [anchors.motor![0], tankY, 0.2], anchors.motor!], radius: 0.02, material: "hv", zones: [CarZone.Electrics] });
  }

  // --- Electrics: two 12 V batteries in their box (24 V), the main fuses
  // and the battery switch on it, the strand to the central electrics in
  // the cab, the socket's feed, and along the frame to the rear lamps.
  const battery: Vec3 = bus ? [frontAxle - r - 0.45, 0.55, 0.85] : conventional ? [behindFront - 1.55, tankY, -besideRail] : [behindFront - 1.55, tankY + 0.02, -besideRail];
  parts.push(box("battery12", "electrics", "electric", battery, [0.55, 0.5, 0.6], [CarZone.Electrics]));
  anchors.battery = [battery[0], battery[1] + 0.32, battery[2]];
  const mainFuses: Vec3 = [battery[0], battery[1] + 0.3, battery[2] + Math.sign(battery[2]) * -0.15];
  parts.push(box("main-fuses", "electrics", "electric", mainFuses, [0.16, 0.08, 0.12], [CarZone.Electrics]));
  const groundPoint: Vec3 = truck ? [battery[0] + 0.3, p.frameTop - 0.14, -frameHalf - 0.06] : [battery[0] + 0.35, 0.5, battery[2] - 0.2];
  parts.push(box("ground-point", "electrics", "metal", groundPoint, [0.05, 0.05, 0.05], [CarZone.Electrics]));
  tubes.push({ id: "ground", layer: "electrics", points: [[battery[0] + 0.15, battery[1] + 0.27, battery[2]], [battery[0] + 0.25, battery[1] + 0.2, (battery[2] + groundPoint[2]) / 2], groundPoint], radius: 0.014, material: "metal", zones: [CarZone.Electrics] });
  // The central electrics: in the cab on the passenger side, behind a
  // bus driver.
  const fuseBox: Vec3 = bus ? [dashX - 0.9, 1.7, W / 2 - 0.25] : [dashX - 0.1, p.floor + 0.42, -lhd * 0.75];
  parts.push(box("fuse-box", "electrics", "electric", fuseBox, bus ? [0.5, 0.7, 0.2] : [0.32, 0.3, 0.12], [CarZone.Electrics]));
  if (combustion) {
    const alternator: Vec3 = [engine[0] + engineFront * (block[0] / 2 - 0.1), engine[1] + 0.12, engine[2] + block[2] / 2 + 0.16];
    parts.push(cylinder("alternator", "electrics", "electric", "x", alternator, 0.24, 0.2, [CarZone.Electrics]));
    const starter: Vec3 = [engine[0] - engineFront * (block[0] / 2 + 0.08), engine[1] - 0.18, engine[2] - block[2] / 2 - 0.06];
    parts.push(cylinder("starter", "electrics", "electric", "x", starter, 0.32, 0.14, [CarZone.Electrics]));
    tubes.push({ id: "starter-cable", layer: "electrics", points: [mainFuses, [mainFuses[0], tankY - 0.15, -0.3], [starter[0], tankY - 0.15, -0.3], starter], radius: 0.016, material: "electric", zones: [CarZone.Electrics] });
  }
  const strandY = truck ? p.frameTop - 0.1 : bus ? 0.3 : 0.5;
  const cabEntry: Vec3 = truck ? [Math.min(fuseBox[0], p.cabBack + 0.4), p.floor - 0.1, -lhd * 0.6] : [fuseBox[0], strandY, fuseBox[2] - 0.1];
  tubes.push({
    id: "harness",
    layer: "electrics",
    points: dedupe([mainFuses, [mainFuses[0], strandY, -lhd * (frameHalf - 0.08)], [cabEntry[0], strandY, -lhd * (frameHalf - 0.08)], cabEntry, fuseBox]),
    radius: 0.02,
    material: "electric",
    zones: [CarZone.Electrics],
  });
  const lampY = truck ? p.frameTop - 0.24 : 0.75;
  const rearLamp = (side: number): Vec3 => [xR + 0.12, lampY, side * (W / 2 - 0.35)];
  tubes.push({ id: "harness-rear", layer: "electrics", points: [mainFuses, [mainFuses[0], strandY, lhd * (frameHalf - 0.08)], [xR + 0.3, strandY, lhd * (frameHalf - 0.08)], rearLamp(1)], branches: [[[xR + 0.3, strandY, lhd * (frameHalf - 0.08)], [xR + 0.2, lampY, 0], rearLamp(-1)]], radius: 0.014, material: "electric", zones: [CarZone.Electrics] });
  const headY = profile.noseTop - profile.headlight.drop - profile.headlight.height / 2;
  const frontLamp = (side: number): Vec3 => [xF - 0.25, headY, side * (W / 2 - 0.4)];
  const frontSplit: Vec3 = [xF - 0.4, headY + 0.1, 0];
  tubes.push({ id: "harness-front", layer: "electrics", points: [fuseBox, [fuseBox[0], headY + 0.1, fuseBox[2]], frontSplit, frontLamp(1)], branches: [[frontSplit, frontLamp(-1)]], radius: 0.012, material: "electric", zones: [CarZone.Electrics] });

  // --- The round 9-pin socket (SAE J1939-13) under the dashboard on the
  // driver's side, the place data/obd_sockets.csv gives for a car aside.
  const spot = socketSpotOf(a.socketLocation ?? 0, a.socketSpot ?? 0);
  const passenger = spot === 2 || spot === 9;
  const obd: Vec3 = [dashX - 0.12, driverFloor + 0.4, (passenger ? -1 : 1) * lhd * (bus ? 0.95 : W / 2 - 0.35)];
  anchors.obd = obd;
  tubes.push({ id: "obd-feed", layer: "electrics", points: [fuseBox, [fuseBox[0], obd[1] + 0.05, (fuseBox[2] + obd[2]) / 2], obd], radius: 0.007, material: "electric", zones: [CarZone.Electrics, CarZone.Network] });

  // --- Control units: the engine's on its left side, the gearbox's on the
  // gearbox, the EBS central unit on the frame ahead of the rear axle (in a
  // bus in the front electrics), cluster and tachograph in the cab.
  const roofConsole: Vec3 = bus ? [xF - 0.55, p.glassTop - 0.05, lhd * 0.7] : conventional ? [p.cowlX - 0.75, Math.min(3.0, p.cabHeight) - 0.25, 0] : [xF - 0.62, p.shieldTop + 0.05, 0];
  const placeAt = (place: EcuPlace): Vec3 => {
    switch (place) {
      case "engineBay":
        return combustion ? [engine[0] - engineFront * 0.25, engine[1] + 0.05, engine[2] + block[2] / 2 + 0.12] : [anchors.motor![0], anchors.motor![1] + 0.3, 0.3];
      case "gearbox":
        return [gearbox[0], gearbox[1] + 0.32, gearbox[2]];
      case "bulkhead":
        return truck ? [rear0 + r + 0.45, p.frameTop - 0.2, frameHalf - 0.16] : [fuseBox[0], fuseBox[1] - 0.45, fuseBox[2]];
      case "tunnel":
        return [dashX, driverFloor + 0.3, 0];
      case "steeringColumn":
        return [dashX - 0.12, driverFloor + 0.55, lhd * p.driverZ];
      case "cluster":
        return [dashX - 0.05, driverFloor + 0.88, lhd * p.driverZ];
      case "glovebox":
      case "footwell":
        return [fuseBox[0], fuseBox[1] + 0.12, fuseBox[2]];
      case "dashCentre":
        return [dashX, driverFloor + 0.75, bus ? 0.2 : 0];
      case "headlight":
        return frontLamp(1);
      case "driverDoor":
        return [seatX + 0.2, driverFloor + 0.65, lhd * (W / 2 - 0.12)];
      case "passengerDoor":
        return [seatX + 0.2, driverFloor + 0.65, -lhd * (W / 2 - 0.12)];
      case "underSeat":
        return [seatX, driverFloor + 0.15, lhd * p.driverZ];
      case "passengerSeat":
        return [seatX, driverFloor + 0.15, -lhd * p.driverZ];
      case "boot":
        return truck ? [p.cabBack + 0.25, p.floor + 0.9, 0.5] : [xR + 0.6, 2.1, 0.3];
      case "battery":
        return anchors.hv ?? [battery[0], battery[1] + 0.35, battery[2]];
      case "rearBumper":
        return [xR + 0.25, lampY, 0];
      case "inverter":
        return anchors.motor ?? anchors.engine!;
      case "roof":
        return [roofConsole[0], roofConsole[1], 0];
      case "windscreen":
        return [cabFront - 0.25, (conventional ? p.shieldTop : p.glassTop) - 0.08, 0];
      case "fuelPump":
        return fuel ? [fuel[0], fuel[1] + 0.3, fuel[2]] : [battery[0], battery[1] + 0.35, battery[2]];
      case "rearAxle":
        return [rear0 + 0.35, truck ? p.frameTop - 0.2 : r + 0.35, 0];
      case "tachograph":
        // The DIN slot in the console over the windscreen, on the driver's side.
        return [roofConsole[0], roofConsole[1], lhd * 0.35];
    }
  };
  const harnessY = truck ? p.frameTop - 0.18 : bus ? 0.3 : 0.5;
  const cabY = p.floor + 0.08;
  const lane: Record<BusKind, number> = { powertrain: -0.08, comfort: 0.08, infotainment: 0.18, diagnostic: 0, kline: 0.13, mscan: -0.16, swcan: -0.24 };
  const inCab = (q: Vec3) => truck && q[0] > p.cabBack + 0.05 && q[1] > p.floor - 0.2;
  const network = buildNetwork(input, {
    placeAt,
    obd,
    step: 0.16,
    clamp: (q) => clampInside(hull, q, 0.09),
    route: (from, to, kind) => {
      const z = lane[kind];
      // Inside the cab along its floor; to the frame down the back of the
      // cab and along the rails.
      if (inCab(from) && inCab(to)) return dedupe([from, [from[0], cabY, from[2]], [from[0], cabY, z], [to[0], cabY, z], [to[0], cabY, to[2]], to]);
      const y = Math.max(p.clearance + 0.12, Math.min(harnessY, from[1] - 0.05, to[1] - 0.05));
      const down = truck ? Math.min(from[0], p.cabBack + 0.15) : from[0];
      return dedupe([from, [down, from[1], from[2]], [down, y, z], [to[0], y, z], [to[0], y, to[2]], to]);
    },
  });
  anchors.network = network.hub;
  const engineEcu = network.ecus.find((e) => e.kind === EcuKind.Engine);
  anchors.ecu = engineEcu ? engineEcu.position : placeAt("engineBay");
  if (combustion) {
    const coils = [0, 5].map((i): Vec3 => [engine[0] + (i - 2.5) * block[0] * 0.15, engine[1] + block[1] / 2 + 0.3, engine[2]]);
    const at = placeAt("engineBay");
    tubes.push({ id: "engine-harness", layer: "electrics", points: [at, [at[0], coils[0]![1], at[2] * 0.5], coils[0]!, coils[1]!], radius: 0.014, material: "electric", zones: [CarZone.Electrics, CarZone.Ignition] });
  }

  // The drivetrain taken apart: the engine lifts, its head and injectors
  // further, the gearbox moves back, the exhaust out to the right.
  const explodeOf = (id: string): Vec3 | undefined => {
    const is = (...prefixes: string[]) => prefixes.some((q) => id === q || id.startsWith(`${q}-`));
    if (id === "engine") return [0, 0.5, 0];
    if (is("cylinder-head")) return [0, 0.8, 0];
    if (is("coil")) return [0, 1.05, 0];
    if (is("intake-manifold", "fuel-rail")) return [0, 0.5, 0.45];
    if (is("airbox", "intake-pipe")) return [0, 0.9, 0.4];
    if (is("turbo")) return [0, 0.4, -0.5];
    if (is("gearbox")) return [-engineFront * 0.6, 0.25, 0];
    if (is("rear-diff")) return [0, 0.3, 0];
    if (is("prop-shaft")) return [0, 0.18, 0];
    if (is("exhaust", "catalyst")) return [0, 0, -0.85];
    if (is("motor")) return [0, 0.45, 0];
    return undefined;
  };
  for (const part of parts) if (part.layer === "drivetrain") part.explode ??= explodeOf(part.id);
  for (const tube of tubes) if (tube.layer === "drivetrain") tube.explode ??= explodeOf(tube.id);

  // Everything inside the envelope.
  for (const part of parts) part.center = clampInside(hull, part.center, 0.03);
  for (const tube of tubes) {
    tube.points = tube.points.map((q) => clampInside(hull, q, tube.radius + 0.01));
    if (tube.branches) tube.branches = tube.branches.map((b) => b.map((q) => clampInside(hull, q, tube.radius + 0.01)));
  }
  const routes = network.routes;
  for (const rt of routes) rt.points = rt.points.map((q, i) => (i === rt.points.length - 1 ? q : clampInside(hull, q, 0.02)));
  for (const key of Object.keys(anchors)) anchors[key] = clampInside(hull, anchors[key]!, 0.03);
  const socket = clampInside(hull, obd, 0.07);
  const size: Size = {
    lengthMm: Math.round(p.L * 1000),
    widthMm: Math.round(p.W * 1000),
    heightMm: Math.round(p.H * 1000),
    wheelbaseMm: Math.round(d.wb * 1000),
    trackMm: Math.round(frontTrack * 1000),
    wheelMm: Math.round(d.wheel * 1000),
  };
  return {
    vehicle: "car",
    hull,
    shape: shapeFor(a, input.body),
    frontAxle,
    rearAxle: rear0,
    wheels,
    parts,
    tubes,
    ecus: network.ecus,
    routes,
    busLabels: network.busLabels,
    obd: socket,
    socketSpot: spot,
    adapter: { socket, dir: [0, -0.5, (obd[2] > 0 ? -1 : 1) * 0.87], cable: null, round: true },
    hub: network.hub,
    anchors,
    moto: null,
    heavy: spec,
    size,
  };
}
