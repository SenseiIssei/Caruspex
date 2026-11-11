// The car of the X-ray view, laid out in numbers: the hull from the
// dimensions and the body type, the wheels, every part of the drivetrain,
// the control units at their usual places and the bus lines between them.
// Pure functions without three.js, so the tests can check the result (every
// part inside the hull, the wheels on the ground, a place for every kind of
// control unit) and the scene only turns it into meshes.
//
// Coordinates are metres: x along the car (front is +x, 0 is the middle of
// the car), y up (the road is 0), z across (+z is the left side, the
// driver's side in a left-hand drive car).
import { BodyType, CarZone, DrivenAxles, EngineLayout, TractionBattery, type VehicleAnatomy } from "../api/types";

export type Vec3 = [number, number, number];

export type Layer = "body" | "drivetrain" | "cooling" | "energy" | "electrics" | "network";
export const LAYERS: Layer[] = ["body", "drivetrain", "cooling", "energy", "electrics", "network"];

// What a part is made of, for its colour in the scene.
export type Material =
  | "engine"
  | "metal"
  | "gearbox"
  | "exhaust"
  | "cooling"
  | "fuel"
  | "hv"
  | "battery"
  | "electric"
  | "cabin"
  | "motor";

export interface Part {
  id: string;
  layer: Layer;
  // "box" with rounded edges, or a cylinder along an axis.
  shape: "box" | "cylinder";
  axis: "x" | "y" | "z";
  center: Vec3;
  // Box: the full extents. Cylinder: [length, diameter, diameter] along its axis.
  size: Vec3;
  material: Material;
  zones: CarZone[];
}

export interface Tube {
  id: string;
  layer: Layer;
  points: Vec3[];
  radius: number;
  material: Material;
  zones: CarZone[];
}

export interface Wheel {
  center: Vec3;
  radius: number;
  width: number;
  front: boolean;
  driven: boolean;
}

// EcuKind of data/EcuAddresses.h.
export const EcuKind = {
  Engine: 0, Gearbox: 1, Brakes: 2, Airbag: 3, Steering: 4, Instruments: 5, Gateway: 6,
  Body: 7, Climate: 8, Parking: 9, Lighting: 10, Doors: 11, Seats: 12, Infotainment: 13,
  Battery: 14, Charger: 15, Hybrid: 16, Other: 17,
} as const;
export const ECU_KIND_COUNT = 18;

export type EcuPlace =
  | "engineBay"
  | "gearbox"
  | "bulkhead"
  | "tunnel"
  | "steeringColumn"
  | "cluster"
  | "glovebox"
  | "footwell"
  | "dashCentre"
  | "headlight"
  | "driverDoor"
  | "underSeat"
  | "boot"
  | "battery"
  | "rearBumper"
  | "inverter";

// The usual mounting place per kind. Makers differ (and a few put the
// gateway in the instrument cluster or the radio in the boot), but this is
// where one looks first.
export const ECU_PLACES: Record<number, EcuPlace> = {
  [EcuKind.Engine]: "engineBay",
  [EcuKind.Gearbox]: "gearbox",
  [EcuKind.Brakes]: "bulkhead",
  [EcuKind.Airbag]: "tunnel",
  [EcuKind.Steering]: "steeringColumn",
  [EcuKind.Instruments]: "cluster",
  [EcuKind.Gateway]: "glovebox",
  [EcuKind.Body]: "footwell",
  [EcuKind.Climate]: "dashCentre",
  [EcuKind.Parking]: "rearBumper",
  [EcuKind.Lighting]: "headlight",
  [EcuKind.Doors]: "driverDoor",
  [EcuKind.Seats]: "underSeat",
  [EcuKind.Infotainment]: "dashCentre",
  [EcuKind.Battery]: "battery",
  [EcuKind.Charger]: "boot",
  [EcuKind.Hybrid]: "inverter",
  [EcuKind.Other]: "boot",
};

export type BusKind = "powertrain" | "comfort" | "diagnostic" | "kline";

// Which CAN a kind usually hangs on behind the gateway.
export function busForKind(kind: number): BusKind {
  switch (kind) {
    case EcuKind.Engine:
    case EcuKind.Gearbox:
    case EcuKind.Brakes:
    case EcuKind.Airbag:
    case EcuKind.Steering:
    case EcuKind.Battery:
    case EcuKind.Charger:
    case EcuKind.Hybrid:
      return "powertrain";
    case EcuKind.Gateway:
      return "diagnostic";
  }
  return "comfort";
}

export interface EcuInput {
  key: string;
  kind: number;
  name: string;
  faulty: boolean;
  // How it was read: a UDS scan, plain OBD, or the K-line.
  source: "uds" | "obd" | "kline";
  // Being asked by a running scan, not answered yet.
  asking?: boolean;
}

export interface EcuNode extends EcuInput {
  place: EcuPlace;
  position: Vec3;
  bus: BusKind;
}

export interface Route {
  bus: BusKind;
  // The unit at the far end ("" for the line from the socket to the gateway).
  ecu: string;
  // From the hub (gateway or socket) to the unit.
  points: Vec3[];
}

export interface Profile {
  // Roof line: (station from the rear 0 to the front 1, share of the height).
  top: [number, number][];
  belt: number;
  // Stations of the base of the windscreen, the front and rear end of the
  // roof, the base of the rear window and the pillars between the doors.
  cowl: number;
  roofFront: number;
  roofRear: number;
  tail: number;
  pillars: number[];
  clearance: number;
  rearSeats: boolean;
}

const PROFILES: Record<BodyType, Profile> = {
  [BodyType.Hatchback]: {
    top: [[0, 0.5], [0.015, 0.66], [0.05, 0.86], [0.14, 0.985], [0.33, 1], [0.54, 0.99], [0.71, 0.68], [0.84, 0.63], [0.96, 0.56], [1, 0.4]],
    belt: 0.66, cowl: 0.71, roofFront: 0.54, roofRear: 0.14, tail: 0.03, pillars: [0.36], clearance: 0.14, rearSeats: true,
  },
  [BodyType.Estate]: {
    top: [[0, 0.5], [0.012, 0.68], [0.03, 0.92], [0.07, 0.978], [0.3, 1], [0.53, 0.995], [0.71, 0.68], [0.84, 0.63], [0.96, 0.56], [1, 0.4]],
    belt: 0.67, cowl: 0.71, roofFront: 0.53, roofRear: 0.06, tail: 0.016, pillars: [0.36, 0.15], clearance: 0.14, rearSeats: true,
  },
  [BodyType.Sedan]: {
    top: [[0, 0.55], [0.02, 0.69], [0.1, 0.72], [0.14, 0.735], [0.27, 0.97], [0.36, 1], [0.52, 0.995], [0.71, 0.68], [0.84, 0.63], [0.96, 0.56], [1, 0.41]],
    belt: 0.67, cowl: 0.71, roofFront: 0.52, roofRear: 0.3, tail: 0.14, pillars: [0.4], clearance: 0.14, rearSeats: true,
  },
  [BodyType.Suv]: {
    top: [[0, 0.5], [0.012, 0.7], [0.03, 0.93], [0.08, 0.985], [0.3, 1], [0.55, 0.99], [0.69, 0.74], [0.8, 0.71], [0.95, 0.67], [1, 0.52]],
    belt: 0.7, cowl: 0.69, roofFront: 0.55, roofRear: 0.07, tail: 0.02, pillars: [0.37, 0.15], clearance: 0.2, rearSeats: true,
  },
  [BodyType.Van]: {
    top: [[0, 0.46], [0.008, 0.75], [0.02, 0.97], [0.05, 1], [0.7, 1], [0.79, 0.96], [0.88, 0.56], [0.97, 0.5], [1, 0.4]],
    belt: 0.53, cowl: 0.88, roofFront: 0.77, roofRear: 0.04, tail: 0.01, pillars: [0.68, 0.42, 0.18], clearance: 0.17, rearSeats: true,
  },
  [BodyType.Coupe]: {
    top: [[0, 0.55], [0.02, 0.7], [0.1, 0.735], [0.15, 0.76], [0.32, 0.97], [0.43, 1], [0.53, 0.99], [0.69, 0.72], [0.82, 0.68], [0.95, 0.63], [1, 0.48]],
    belt: 0.7, cowl: 0.69, roofFront: 0.53, roofRear: 0.36, tail: 0.15, pillars: [], clearance: 0.13, rearSeats: true,
  },
  [BodyType.Convertible]: {
    top: [[0, 0.56], [0.02, 0.71], [0.12, 0.75], [0.18, 0.78], [0.32, 0.95], [0.4, 1], [0.5, 0.99], [0.65, 0.75], [0.8, 0.69], [0.95, 0.64], [1, 0.49]],
    belt: 0.72, cowl: 0.65, roofFront: 0.5, roofRear: 0.33, tail: 0.18, pillars: [], clearance: 0.13, rearSeats: true,
  },
  [BodyType.Pickup]: {
    top: [[0, 0.55], [0.01, 0.64], [0.37, 0.645], [0.385, 0.95], [0.41, 1], [0.58, 0.995], [0.68, 0.74], [0.8, 0.72], [0.95, 0.69], [1, 0.55]],
    belt: 0.64, cowl: 0.68, roofFront: 0.58, roofRear: 0.41, tail: 0.385, pillars: [0.49], clearance: 0.22, rearSeats: true,
  },
  [BodyType.Mpv]: {
    top: [[0, 0.48], [0.012, 0.72], [0.03, 0.93], [0.08, 0.99], [0.4, 1], [0.6, 0.98], [0.76, 0.64], [0.85, 0.61], [0.96, 0.57], [1, 0.44]],
    belt: 0.62, cowl: 0.76, roofFront: 0.6, roofRear: 0.07, tail: 0.02, pillars: [0.42, 0.18], clearance: 0.15, rearSeats: true,
  },
  [BodyType.Microcar]: {
    top: [[0, 0.5], [0.01, 0.72], [0.04, 0.94], [0.12, 1], [0.52, 0.99], [0.74, 0.66], [0.86, 0.62], [0.97, 0.58], [1, 0.46]],
    belt: 0.62, cowl: 0.74, roofFront: 0.52, roofRear: 0.1, tail: 0.02, pillars: [], clearance: 0.14, rearSeats: false,
  },
  [BodyType.Roadster]: {
    top: [[0, 0.6], [0.02, 0.75], [0.2, 0.79], [0.33, 0.82], [0.37, 0.98], [0.44, 1], [0.53, 0.97], [0.6, 0.8], [0.8, 0.72], [0.95, 0.66], [1, 0.52]],
    belt: 0.78, cowl: 0.6, roofFront: 0.53, roofRear: 0.4, tail: 0.33, pillars: [], clearance: 0.12, rearSeats: false,
  },
};

export function profileFor(body: BodyType): Profile {
  return PROFILES[body] ?? PROFILES[BodyType.Hatchback];
}

// --- Curves ----------------------------------------------------------------------

// Monotone cubic interpolation (Fritsch and Carlson): smooth through the
// points without overshooting, so a flat roof stays flat.
export function monotone(points: [number, number][]): (x: number) => number {
  const n = points.length;
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const d: number[] = [];
  const m: number[] = [];
  for (let i = 0; i < n - 1; i++) d.push((ys[i + 1]! - ys[i]!) / (xs[i + 1]! - xs[i]!));
  m.push(d[0]!);
  for (let i = 1; i < n - 1; i++) m.push(d[i - 1]! * d[i]! <= 0 ? 0 : (d[i - 1]! + d[i]!) / 2);
  m.push(d[n - 2]!);
  for (let i = 0; i < n - 1; i++) {
    if (d[i] === 0) {
      m[i] = 0;
      m[i + 1] = 0;
      continue;
    }
    const a = m[i]! / d[i]!;
    const b = m[i + 1]! / d[i]!;
    const s = a * a + b * b;
    if (s > 9) {
      const t = 3 / Math.sqrt(s);
      m[i] = t * a * d[i]!;
      m[i + 1] = t * b * d[i]!;
    }
  }
  return (x: number) => {
    if (x <= xs[0]!) return ys[0]!;
    if (x >= xs[n - 1]!) return ys[n - 1]!;
    let i = 0;
    while (i < n - 2 && x > xs[i + 1]!) i++;
    const h = xs[i + 1]! - xs[i]!;
    const t = (x - xs[i]!) / h;
    const t2 = t * t;
    const t3 = t2 * t;
    return (
      (2 * t3 - 3 * t2 + 1) * ys[i]! + (t3 - 2 * t2 + t) * h * m[i]! + (-2 * t3 + 3 * t2) * ys[i + 1]! + (t3 - t2) * h * m[i + 1]!
    );
  };
}

export function smoothstep(a: number, b: number, x: number): number {
  const t = Math.min(1, Math.max(0, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
}

// --- The hull --------------------------------------------------------------------

export interface Hull {
  length: number;
  width: number;
  height: number;
  profile: Profile;
  belt: number;
  // x of the front and rear end.
  front: number;
  rear: number;
  // Station 0..1 (rear to front) of an x, and back.
  station(x: number): number;
  xAt(u: number): number;
  top(u: number): number;
  bottom(u: number): number;
  // Half the width in plan view at a station.
  planHalf(u: number): number;
  // Half the width at a station and height; 0 outside the hull.
  halfWidth(u: number, y: number): number;
  // One point of the cross-section, theta from -pi/2 (bottom) round to 3pi/2.
  section(u: number, theta: number): Vec3;
}

const SECTION_EXPONENT = 5;
// How much narrower the roof is than the body at the beltline.
const TUMBLEHOME = 0.17;

export function makeHull(lengthM: number, widthM: number, heightM: number, body: BodyType): Hull {
  const profile = profileFor(body);
  const topCurve = monotone(profile.top);
  const height = heightM;
  const front = lengthM / 2;
  const rear = -lengthM / 2;
  const belt = profile.belt * height;
  const clearance = profile.clearance;
  const bottom = (u: number) =>
    clearance + 0.17 * (1 - smoothstep(0, 0.09, u)) + 0.15 * smoothstep(0.91, 1, u) + 0.03 * smoothstep(0.75, 0.95, u);
  const top = (u: number) => topCurve(u) * height;
  const planHalf = (u: number) => {
    const s = Math.abs(2 * u - 1);
    const p = u > 0.5 ? 4.5 : 7;
    return (widthM / 2) * Math.pow(Math.max(0, 1 - Math.pow(s, p)), 1 / p);
  };
  const tumble = (y: number, u: number) => 1 - TUMBLEHOME * smoothstep(belt - 0.05, top(u), y);
  const n = SECTION_EXPONENT;
  const hull: Hull = {
    length: lengthM,
    width: widthM,
    height,
    profile,
    belt,
    front,
    rear,
    station: (x) => (x - rear) / lengthM,
    xAt: (u) => rear + u * lengthM,
    top,
    bottom,
    planHalf,
    halfWidth(u, y) {
      if (u < 0 || u > 1) return 0;
      const yb = bottom(u);
      const yt = top(u);
      if (y < yb || y > yt) return 0;
      const yc = (yb + yt) / 2;
      const hh = (yt - yb) / 2;
      const r = Math.abs((y - yc) / hh);
      return planHalf(u) * Math.pow(Math.max(0, 1 - Math.pow(r, n)), 1 / n) * tumble(y, u);
    },
    section(u, theta) {
      const yb = bottom(u);
      const yt = top(u);
      const yc = (yb + yt) / 2;
      const hh = (yt - yb) / 2;
      const c = Math.cos(theta);
      const s = Math.sin(theta);
      const y = yc + hh * Math.sign(s) * Math.pow(Math.abs(s), 2 / n);
      const z = planHalf(u) * Math.sign(c) * Math.pow(Math.abs(c), 2 / n) * tumble(y, u);
      return [rear + u * lengthM, y, z];
    },
  };
  return hull;
}

// Whether a point lies inside the hull with some room to spare.
export function insideHull(hull: Hull, p: Vec3, margin = 0.02): boolean {
  const u = hull.station(p[0]);
  if (u <= 0 || u >= 1) return false;
  if (p[1] < hull.bottom(u) + margin || p[1] > hull.top(u) - margin) return false;
  return Math.abs(p[2]) <= hull.halfWidth(u, p[1]) - margin;
}

// Pulls a point inside the hull: first along the car, then up or down, then
// towards the middle.
export function clampInside(hull: Hull, p: Vec3, margin = 0.06): Vec3 {
  const minX = hull.rear + hull.length * 0.03;
  const maxX = hull.front - hull.length * 0.03;
  const x = Math.min(maxX, Math.max(minX, p[0]));
  const u = hull.station(x);
  const yb = hull.bottom(u) + margin;
  const yt = hull.top(u) - margin;
  const y = yt < yb ? (yb + yt) / 2 : Math.min(yt, Math.max(yb, p[1]));
  const half = Math.max(0, hull.halfWidth(u, y) - margin);
  const z = Math.max(-half, Math.min(half, p[2]));
  return [x, y, z];
}

// --- The whole car ------------------------------------------------------------------

export interface XrayInput {
  anatomy: VehicleAnatomy;
  body: BodyType;
  ecus: EcuInput[];
  // The diagnosis line of the OBD socket is a K-line (ISO 9141, KWP2000 or a
  // KKL cable) instead of CAN.
  kline: boolean;
  // A turbocharger is drawn for diesels and when a fault points at one.
  turbo: boolean;
}

export interface CarLayout {
  hull: Hull;
  frontAxle: number;
  rearAxle: number;
  wheels: Wheel[];
  parts: Part[];
  tubes: Tube[];
  ecus: EcuNode[];
  routes: Route[];
  obd: Vec3;
  hub: Vec3;
  // Positions for the zones of fault codes and for the live labels.
  anchors: Record<string, Vec3>;
}

const box = (id: string, layer: Layer, material: Material, center: Vec3, size: Vec3, zones: CarZone[] = []): Part => ({
  id,
  layer,
  shape: "box",
  axis: "x",
  center,
  size,
  material,
  zones,
});

const cylinder = (
  id: string,
  layer: Layer,
  material: Material,
  axis: "x" | "y" | "z",
  center: Vec3,
  length: number,
  diameter: number,
  zones: CarZone[] = [],
): Part => ({ id, layer, shape: "cylinder", axis, center, size: [length, diameter, diameter], material, zones });

function frontShare(engine: EngineLayout, body: BodyType): number {
  if (body === BodyType.Van) return 0.47;
  switch (engine) {
    case EngineLayout.FrontTransverse: return 0.54;
    case EngineLayout.FrontLongitudinal: return 0.47;
    case EngineLayout.Mid: return 0.5;
    case EngineLayout.Rear: return 0.4;
  }
  return 0.5;
}

export function buildLayout(input: XrayInput): CarLayout {
  const a = input.anatomy;
  const L = a.lengthMm / 1000;
  const W = a.widthMm / 1000;
  const H = a.heightMm / 1000;
  const wheelbase = Math.min(a.wheelbaseMm / 1000, L * 0.8);
  const track = Math.min(a.trackMm / 1000, W - 0.2);
  const hull = makeHull(L, W, H, input.body);
  const p = hull.profile;
  const overhang = L - wheelbase;
  const frontAxle = hull.front - overhang * frontShare(a.engine, input.body);
  const rearAxle = frontAxle - wheelbase;
  const radius = Math.min(a.wheelMm / 2000, (H - 0.2) / 2.6);
  const tyre = input.body === BodyType.Suv || input.body === BodyType.Pickup ? 0.24 : input.body === BodyType.Microcar ? 0.17 : 0.21;
  const clear = p.clearance;
  const cowlX = hull.xAt(p.cowl);
  const floor = clear + 0.12;
  const lhd = 1; // +z is the driver's side

  const wheels: Wheel[] = [];
  for (const front of [true, false]) {
    // The tyre stays under the body: the track shrinks where the body is
    // narrower than the typical values assume.
    const axleU = hull.station(front ? frontAxle : rearAxle);
    const room = Math.min(hull.halfWidth(axleU, radius), hull.halfWidth(axleU, radius * 1.4)) - tyre / 2 - 0.015;
    const axleTrack = Math.min(track, room * 2);
    for (const side of [1, -1]) {
      const driven =
        a.driven === DrivenAxles.All || (front ? a.driven === DrivenAxles.Front : a.driven === DrivenAxles.Rear);
      wheels.push({ center: [front ? frontAxle : rearAxle, radius, (side * axleTrack) / 2], radius, width: tyre, front, driven });
    }
  }

  const parts: Part[] = [];
  const tubes: Tube[] = [];
  const anchors: Record<string, Vec3> = {};

  // --- Cabin (body layer): dashboard, seats, steering wheel.
  const dashX = cowlX - 0.22;
  parts.push(box("dashboard", "body", "cabin", [dashX, clear + 0.62, 0], [0.32, 0.2, W * 0.78], [CarZone.Cabin]));
  const frontSeatX = Math.max(cowlX - 1.0, rearAxle + 0.5);
  const seatY = Math.max(floor + 0.14, clear + 0.28);
  for (const side of [1, -1]) {
    parts.push(box(`seat-front-${side}`, "body", "cabin", [frontSeatX, seatY, side * 0.37], [0.5, 0.12, 0.5], [CarZone.Cabin]));
    parts.push(box(`back-front-${side}`, "body", "cabin", [frontSeatX - 0.27, seatY + 0.33, side * 0.37], [0.11, 0.6, 0.48], [CarZone.Cabin]));
  }
  const rearSeatX = frontSeatX - 0.86;
  const rearRoom = p.rearSeats && rearSeatX - 0.3 > rearAxle - 0.15;
  if (rearRoom) {
    parts.push(box("seat-rear", "body", "cabin", [rearSeatX, seatY + 0.02, 0], [0.48, 0.12, W * 0.72], [CarZone.Cabin]));
    parts.push(box("back-rear", "body", "cabin", [rearSeatX - 0.28, seatY + 0.33, 0], [0.11, 0.56, W * 0.72], [CarZone.Cabin]));
  }
  parts.push(cylinder("steering-wheel", "body", "cabin", "x", [dashX - 0.32, clear + 0.78, lhd * 0.37], 0.04, 0.38, [CarZone.Cabin]));
  anchors.cabin = [dashX, clear + 0.75, 0];

  // --- Drivetrain.
  const combustion = a.engine !== EngineLayout.None;
  let engineCenter: Vec3 = [frontAxle, clear + 0.45, 0];
  let gearboxCenter: Vec3 = [frontAxle, clear + 0.3, 0];
  const engineTop = (c: Vec3, h: number) => c[1] + h / 2;
  if (combustion) {
    const blockH = 0.42;
    let blockSize: Vec3;
    if (a.engine === EngineLayout.FrontTransverse) {
      engineCenter = [frontAxle + 0.12, clear + 0.38, -0.1];
      blockSize = [0.36, blockH, 0.52];
      gearboxCenter = [frontAxle - 0.02, clear + 0.3, 0.3];
    } else if (a.engine === EngineLayout.FrontLongitudinal) {
      engineCenter = [frontAxle - 0.18, clear + 0.38, 0];
      blockSize = [0.62, blockH, 0.4];
      gearboxCenter = [engineCenter[0] - 0.6, clear + 0.25, 0];
    } else if (a.engine === EngineLayout.Mid) {
      engineCenter = [rearAxle + 0.5, clear + 0.36, 0];
      blockSize = [0.38, blockH, 0.56];
      gearboxCenter = [rearAxle + 0.1, clear + 0.28, 0];
    } else {
      engineCenter = [rearAxle - 0.42, clear + 0.36, 0];
      blockSize = [0.42, 0.36, 0.62];
      gearboxCenter = [rearAxle + 0.08, clear + 0.26, 0];
    }
    engineCenter = clampInside(hull, engineCenter, blockSize[1] / 2 + 0.03);
    const block = box("engine", "drivetrain", "engine", engineCenter, blockSize, [CarZone.Cylinders]);
    parts.push(block);
    // Cylinder head and cam cover on top.
    const headY = engineTop(engineCenter, blockSize[1]) + 0.07;
    const head: Vec3 = [engineCenter[0], headY, engineCenter[2]];
    parts.push(box("cylinder-head", "drivetrain", "metal", head, [blockSize[0] * 0.8, 0.14, blockSize[2] * 0.92], [CarZone.Cylinders]));
    // Ignition coils (or glow plugs) along the head.
    const across = blockSize[2] > blockSize[0];
    for (let i = 0; i < 4; i++) {
      const t = (i - 1.5) * (across ? blockSize[2] : blockSize[0]) * 0.2;
      const c: Vec3 = across ? [head[0], headY + 0.1, head[2] + t] : [head[0] + t, headY + 0.1, head[2]];
      parts.push(cylinder(`coil-${i}`, "drivetrain", "electric", "y", c, 0.1, 0.045, [CarZone.Ignition]));
    }
    // Intake side (towards the cabin for a transverse engine) with the
    // manifold, the throttle body and the air box.
    const intakeDir = across ? (a.engine === EngineLayout.FrontTransverse ? -1 : 1) : 1;
    const intakeOffset = across ? blockSize[0] / 2 + 0.07 : blockSize[2] / 2 + 0.07;
    const manifold: Vec3 = across
      ? [engineCenter[0] + intakeDir * intakeOffset, headY - 0.04, engineCenter[2]]
      : [engineCenter[0], headY - 0.04, engineCenter[2] + intakeDir * intakeOffset];
    parts.push(box("intake-manifold", "drivetrain", "metal", manifold, across ? [0.1, 0.16, blockSize[2] * 0.8] : [blockSize[0] * 0.8, 0.16, 0.1], [CarZone.Intake]));
    const rail: Vec3 = across ? [manifold[0] - intakeDir * 0.04, manifold[1] - 0.09, manifold[2]] : [manifold[0], manifold[1] - 0.09, manifold[2] - intakeDir * 0.04];
    parts.push(cylinder("fuel-rail", "drivetrain", "fuel", across ? "z" : "x", rail, (across ? blockSize[2] : blockSize[0]) * 0.75, 0.03, [CarZone.Injection]));
    const throttle: Vec3 = across ? [manifold[0], manifold[1] + 0.02, manifold[2] + blockSize[2] * 0.45] : [manifold[0] + blockSize[0] * 0.45, manifold[1] + 0.02, manifold[2]];
    parts.push(cylinder("throttle", "drivetrain", "metal", across ? "z" : "x", throttle, 0.09, 0.09, [CarZone.Throttle]));
    let airbox: Vec3 = a.engine === EngineLayout.FrontTransverse ? [engineCenter[0] + 0.05, headY + 0.08, 0.38] : [engineCenter[0] + 0.1, headY + 0.08, -0.38];
    airbox = clampInside(hull, airbox, 0.12);
    parts.push(box("airbox", "drivetrain", "metal", airbox, [0.3, 0.14, 0.2], [CarZone.Intake]));
    tubes.push({ id: "intake-pipe", layer: "drivetrain", points: [airbox, [throttle[0], airbox[1], airbox[2] * 0.6], throttle], radius: 0.035, material: "metal", zones: [CarZone.Intake] });
    anchors.intake = airbox;
    anchors.throttle = throttle;
    anchors.injection = rail;
    anchors.ignition = [head[0], headY + 0.15, head[2]];
    anchors.engine = [engineCenter[0], headY + 0.1, engineCenter[2]];

    // Gearbox.
    if (a.engine === EngineLayout.FrontLongitudinal) {
      parts.push(cylinder("gearbox", "drivetrain", "gearbox", "x", gearboxCenter, 0.62, 0.3, [CarZone.Gearbox]));
    } else {
      parts.push(cylinder("gearbox", "drivetrain", "gearbox", "z", gearboxCenter, 0.36, 0.34, [CarZone.Gearbox]));
    }
    anchors.gearbox = [gearboxCenter[0], gearboxCenter[1] + 0.2, gearboxCenter[2]];

    // Exhaust: manifold, catalytic converter, silencers, tail pipe.
    const exhaustSide = across ? -intakeDir : -1;
    const exManifold: Vec3 = across
      ? [engineCenter[0] + exhaustSide * (blockSize[0] / 2 + 0.05), engineCenter[1] + 0.08, engineCenter[2]]
      : [engineCenter[0], engineCenter[1] + 0.08, engineCenter[2] - (blockSize[2] / 2 + 0.05)];
    const underY = clear + 0.06;
    const rearMuffler: Vec3 = [hull.rear + 0.42, clear + 0.13, -0.1];
    const tail: Vec3 = [hull.rear + 0.08, clear + 0.12, -W * 0.3];
    let exhaust: Vec3[];
    let catalyst: Vec3;
    let middle: Vec3;
    if (a.engine === EngineLayout.FrontTransverse || a.engine === EngineLayout.FrontLongitudinal) {
      const down: Vec3 = [exManifold[0] - 0.05, underY + 0.12, exManifold[2] * 0.5];
      catalyst = [frontAxle - 0.45, underY + 0.03, 0.02];
      middle = [rearAxle + 0.7, underY + 0.02, 0.02];
      exhaust = [exManifold, down, [catalyst[0] + 0.25, underY + 0.03, 0.02], catalyst, middle, [rearAxle + 0.25, underY + 0.02, -0.05], [rearAxle - 0.1, clear + 0.2, -0.1], rearMuffler, tail];
    } else {
      catalyst = [engineCenter[0] - 0.3, underY + 0.08, -0.15];
      middle = rearMuffler;
      exhaust = [exManifold, [exManifold[0] - 0.1, underY + 0.1, exManifold[2]], catalyst, rearMuffler, tail];
    }
    tubes.push({ id: "exhaust", layer: "drivetrain", points: exhaust, radius: 0.028, material: "exhaust", zones: [CarZone.Exhaust] });
    parts.push(cylinder("catalyst", "drivetrain", "exhaust", "x", catalyst, 0.32, 0.13, [CarZone.Exhaust]));
    if (middle !== rearMuffler) parts.push(cylinder("middle-silencer", "drivetrain", "exhaust", "x", middle, 0.36, 0.11, [CarZone.Exhaust]));
    parts.push(cylinder("rear-silencer", "drivetrain", "exhaust", "z", rearMuffler, 0.6, 0.2, [CarZone.Exhaust]));
    anchors.exhaust = catalyst;
    if (input.turbo) {
      const turbo: Vec3 = [exManifold[0] + (across ? exhaustSide * 0.06 : 0), exManifold[1] - 0.06, exManifold[2] + (across ? 0.12 : -0.08)];
      parts.push(cylinder("turbo", "drivetrain", "metal", "z", turbo, 0.12, 0.17, [CarZone.Turbo]));
      anchors.turbo = turbo;
    }
  }

  // Shafts to the driven wheels, a prop shaft for rear and all-wheel drive
  // with the engine in front, and the electric motors.
  const diffAt = (x: number): Vec3 => [x, radius, 0];
  const halfShafts = (x: number, from: Vec3) => {
    for (const side of [1, -1]) {
      tubes.push({
        id: `shaft-${x > 0 ? "front" : "rear"}-${side}`,
        layer: "drivetrain",
        points: [[from[0], radius, from[2] * 0.5 + side * 0.08], [x, radius, side * (track / 2 - tyre / 2 - 0.02)]],
        radius: 0.022,
        material: "metal",
        zones: [CarZone.Gearbox],
      });
    }
  };
  const frontDriven = a.driven !== DrivenAxles.Rear;
  const rearDriven = a.driven !== DrivenAxles.Front;
  const motors: Vec3[] = [];
  if (a.frontMotor) motors.push([frontAxle + (combustion ? 0 : 0.02), radius + 0.02, combustion ? gearboxCenter[2] - 0.2 : 0]);
  if (a.rearMotor) motors.push([rearAxle + (combustion && a.engine !== EngineLayout.Rear && a.engine !== EngineLayout.Mid ? 0 : 0.25), radius + 0.02, 0]);
  if (frontDriven) halfShafts(frontAxle, combustion && a.engine === EngineLayout.FrontTransverse ? [frontAxle, radius, gearboxCenter[2] * 0.6] : diffAt(frontAxle));
  if (rearDriven) halfShafts(rearAxle, diffAt(rearAxle));
  if (combustion && rearDriven && (a.engine === EngineLayout.FrontLongitudinal || a.engine === EngineLayout.FrontTransverse)) {
    const start: Vec3 = a.engine === EngineLayout.FrontLongitudinal ? [gearboxCenter[0] - 0.3, gearboxCenter[1], 0] : [gearboxCenter[0], radius, 0];
    tubes.push({ id: "prop-shaft", layer: "drivetrain", points: [start, [rearAxle + 0.15, radius, 0]], radius: 0.035, material: "metal", zones: [CarZone.Gearbox] });
    parts.push(box("rear-diff", "drivetrain", "gearbox", [rearAxle, radius, 0], [0.24, 0.22, 0.26], [CarZone.Gearbox]));
  }
  if (combustion && a.engine === EngineLayout.FrontLongitudinal && frontDriven) {
    parts.push(box("front-diff", "drivetrain", "gearbox", [frontAxle, radius, 0], [0.2, 0.2, 0.24], [CarZone.Gearbox]));
  }
  motors.forEach((m, i) => {
    parts.push(cylinder(`motor-${i}`, "drivetrain", "motor", "z", m, 0.4, 0.28, []));
    parts.push(box(`inverter-${i}`, "energy", "hv", [m[0] + (m[0] > 0 ? 0.05 : -0.05), m[1] + 0.24, m[2]], [0.3, 0.1, 0.36], []));
  });
  if (motors.length > 0) anchors.motor = [motors[0]![0], motors[0]![1] + 0.2, motors[0]![2]];
  if (!combustion) anchors.engine = anchors.motor ?? [frontAxle, radius + 0.3, 0];

  // --- Cooling: radiator in the nose, fan, hoses, expansion tank.
  // In the nose, just ahead of an engine that sits in front.
  const frontEngine = a.engine === EngineLayout.FrontTransverse || a.engine === EngineLayout.FrontLongitudinal;
  const radiatorX = Math.min(hull.front - 0.3, frontEngine ? engineCenter[0] + 0.48 : hull.front - 0.3);
  const radiatorU = hull.station(radiatorX);
  const radH = Math.min(0.42, (hull.top(radiatorU) - hull.bottom(radiatorU)) * 0.55);
  const radY = (hull.top(radiatorU) + hull.bottom(radiatorU)) / 2 - 0.02;
  const radW = Math.min(W * 0.62, 2 * (Math.min(hull.halfWidth(radiatorU, radY - radH / 2), hull.halfWidth(radiatorU, radY + radH / 2)) - 0.06));
  const radiator: Vec3 = [radiatorX, radY, 0];
  parts.push(box("radiator", "cooling", "cooling", radiator, [0.04, radH, radW], [CarZone.Cooling]));
  parts.push(cylinder("fan", "cooling", "cooling", "x", [radiatorX - 0.07, radY, 0], 0.05, Math.min(radH * 0.9, 0.36), [CarZone.Cooling]));
  anchors.cooling = [radiatorX, radY + radH / 2, 0];
  const engineFront: Vec3 = combustion ? [engineCenter[0] + 0.15, engineCenter[1] + 0.1, engineCenter[2]] : anchors.motor ?? [frontAxle, radius, 0];
  tubes.push({ id: "hose-upper", layer: "cooling", points: [[radiatorX - 0.02, radY + radH * 0.35, radW * 0.35], [radiatorX - 0.2, radY + radH * 0.4, radW * 0.25], engineFront], radius: 0.022, material: "cooling", zones: [CarZone.Cooling] });
  tubes.push({ id: "hose-lower", layer: "cooling", points: [[radiatorX - 0.02, radY - radH * 0.35, -radW * 0.35], [radiatorX - 0.25, radY - radH * 0.4, -radW * 0.3], [engineFront[0] - 0.05, engineFront[1] - 0.22, engineFront[2] - 0.1]], radius: 0.022, material: "cooling", zones: [CarZone.Cooling] });
  const expansion = clampInside(hull, [cowlX + 0.2, clear + 0.62, -0.5], 0.1);
  parts.push(box("expansion-tank", "cooling", "cooling", expansion, [0.14, 0.12, 0.1], [CarZone.Cooling]));

  // --- Energy: fuel tank and lines, or the high-voltage battery.
  if (a.fuelTank) {
    const tankX = a.engine === EngineLayout.Mid || a.engine === EngineLayout.Rear ? frontAxle - 0.35 : rearAxle + 0.38;
    const tank = clampInside(hull, [tankX, clear + 0.14, 0], 0.1);
    parts.push(box("fuel-tank", "energy", "fuel", tank, [0.5, 0.22, Math.min(0.92, track * 0.62)], [CarZone.FuelTank]));
    const filler: Vec3 = [rearAxle - 0.12, clear + 0.62, -(hull.halfWidth(hull.station(rearAxle - 0.12), clear + 0.62) - 0.06)];
    tubes.push({ id: "filler-neck", layer: "energy", points: [[tank[0] - 0.1, tank[1] + 0.05, -0.3], [rearAxle + 0.05, clear + 0.4, filler[2] * 0.8], filler], radius: 0.025, material: "fuel", zones: [CarZone.FuelTank] });
    if (combustion) {
      const toEngine: Vec3 = anchors.injection ?? engineCenter;
      tubes.push({ id: "fuel-line", layer: "energy", points: [[tank[0] + 0.25, tank[1], 0.18], [frontAxle - 0.2, clear + 0.06, 0.18], [toEngine[0] - 0.1, clear + 0.2, 0.18], toEngine], radius: 0.009, material: "fuel", zones: [CarZone.FuelTank, CarZone.Injection] });
    }
    anchors.fuel = [tank[0], tank[1] + 0.13, tank[2]];
  }
  if (a.battery === TractionBattery.Floor) {
    const len = wheelbase - 0.55;
    const pack: Vec3 = [(frontAxle + rearAxle) / 2 - 0.05, clear + 0.08, 0];
    parts.push(box("hv-battery", "energy", "battery", pack, [len, 0.12, Math.min(track * 0.92, W - 0.3)], [CarZone.Electrics]));
    anchors.hv = [pack[0], pack[1] + 0.08, pack[2]];
  } else if (a.battery === TractionBattery.Hybrid || a.battery === TractionBattery.PlugIn) {
    const big = a.battery === TractionBattery.PlugIn;
    const pack = clampInside(hull, [rearAxle + (big ? 0.2 : 0.55), clear + 0.18, 0], 0.08);
    parts.push(box("hv-battery", "energy", "battery", pack, [big ? 0.5 : 0.32, big ? 0.24 : 0.2, Math.min(1.05, track * 0.7)], [CarZone.Electrics]));
    anchors.hv = [pack[0], pack[1] + 0.12, pack[2]];
  }
  if (anchors.hv) {
    for (const [i, m] of motors.entries()) {
      tubes.push({ id: `hv-cable-${i}`, layer: "energy", points: [anchors.hv, [anchors.hv[0], clear + 0.1, 0.25], [m[0], clear + 0.1, 0.25], [m[0], m[1] + 0.2, m[2] + 0.1]], radius: 0.014, material: "hv", zones: [CarZone.Electrics] });
    }
  }

  // --- Electrics: 12 V battery, alternator, the main harness to the back.
  const batteryAtBack = a.engine === EngineLayout.FrontLongitudinal && input.body !== BodyType.Pickup && input.body !== BodyType.Suv;
  const battery12 = clampInside(
    hull,
    batteryAtBack ? [rearAxle - 0.45, clear + 0.32, -0.45] : combustion && a.engine !== EngineLayout.FrontTransverse ? [frontAxle + 0.15, clear + 0.5, 0.42] : [frontAxle + 0.05, clear + 0.5, 0.44],
    0.12,
  );
  parts.push(box("battery12", "electrics", "electric", battery12, [0.26, 0.19, 0.17], [CarZone.Electrics]));
  anchors.battery = [battery12[0], battery12[1] + 0.12, battery12[2]];
  if (combustion) {
    const alternator: Vec3 = [engineCenter[0] + (a.engine === EngineLayout.FrontLongitudinal ? 0.36 : 0), engineCenter[1] + 0.02, engineCenter[2] + (a.engine === EngineLayout.FrontLongitudinal ? -0.22 : -0.32)];
    parts.push(cylinder("alternator", "electrics", "electric", across3(a.engine), clampInside(hull, alternator, 0.08), 0.16, 0.13, [CarZone.Electrics]));
  }
  const fuseBox = clampInside(hull, [cowlX - 0.3, clear + 0.5, lhd * 0.52], 0.08);
  parts.push(box("fuse-box", "electrics", "electric", fuseBox, [0.16, 0.14, 0.06], [CarZone.Electrics]));
  const sillZ = (x: number, y: number) => lhd * (hull.halfWidth(hull.station(x), y) - 0.12);
  tubes.push({
    id: "harness",
    layer: "electrics",
    points: [battery12, [battery12[0], clear + 0.62, battery12[2] * 0.8], [cowlX - 0.05, clear + 0.62, sillZ(cowlX - 0.05, clear + 0.62)], fuseBox, [cowlX - 0.35, floor + 0.04, sillZ(cowlX - 0.35, floor + 0.04)], [rearAxle + 0.1, floor + 0.04, sillZ(rearAxle + 0.1, floor + 0.04)], clampInside(hull, [hull.rear + 0.3, clear + 0.55, lhd * 0.5], 0.1)],
    radius: 0.016,
    material: "electric",
    zones: [CarZone.Electrics],
  });

  // --- Network: the OBD socket, the control units and their buses.
  const obd = clampInside(hull, [cowlX - 0.38, clear + 0.42, lhd * 0.3], 0.08);
  anchors.obd = obd;
  const placeAt = placeFinder(hull, {
    frontAxle,
    rearAxle,
    cowlX,
    clear,
    floor,
    engineCenter,
    gearboxCenter,
    combustion,
    battery12,
    hv: anchors.hv ?? null,
    motor: motors[0] ?? null,
    frontSeatX,
  });
  const used = new Map<EcuPlace, number>();
  const ecus: EcuNode[] = input.ecus.map((e) => {
    const place = ECU_PLACES[e.kind] ?? "boot";
    const n = used.get(place) ?? 0;
    used.set(place, n + 1);
    const base = placeAt(place);
    // Several units at one place stand next to each other.
    const shifted: Vec3 = [base[0] - Math.floor(n / 2) * 0.16, base[1] + (n % 2) * 0.13, base[2] - (n > 0 ? Math.sign(base[2] || 1) * 0.12 * Math.ceil(n / 2) : 0)];
    const position = clampInside(hull, shifted, 0.09);
    const bus: BusKind = e.source === "kline" || (input.kline && e.source === "obd") ? "kline" : busForKind(e.kind);
    return { ...e, place, position, bus };
  });
  const gateway = ecus.find((e) => e.kind === EcuKind.Gateway);
  const hub = gateway ? gateway.position : obd;
  anchors.network = hub;
  const engineEcu = ecus.find((e) => e.kind === EcuKind.Engine);
  anchors.ecu = engineEcu ? engineEcu.position : placeAt("engineBay");

  const harnessY = floor + 0.08;
  const lane: Record<BusKind, number> = { powertrain: -0.07, comfort: 0.07, diagnostic: 0, kline: 0.13 };
  const route = (from: Vec3, to: Vec3, bus: BusKind): Vec3[] => {
    const z = lane[bus];
    const y = Math.max(harnessY, Math.min(from[1], to[1]) - 0.4);
    return dedupe([from, [from[0], y, from[2]], [from[0], y, z], [to[0], y, z], [to[0], y, to[2]], to]);
  };
  const routes: Route[] = [];
  if (gateway) routes.push({ bus: input.kline ? "kline" : "diagnostic", ecu: "", points: route(obd, hub, "diagnostic") });
  for (const e of ecus) {
    if (e === gateway) continue;
    // A K-line goes straight from the socket to the unit, past the gateway.
    const start = e.bus === "kline" ? obd : hub;
    routes.push({ bus: e.bus, ecu: e.key, points: route(start, e.position, e.bus) });
  }

  // Small or unusual bodies (a Smart, a roadster) leave less room than the
  // offsets above assume: whatever ended up outside is pulled back in.
  for (const part of parts) part.center = clampInside(hull, part.center, 0.03);
  for (const tube of tubes) tube.points = tube.points.map((q) => clampInside(hull, q, tube.radius + 0.01));
  for (const r of routes) r.points = r.points.map((q, i) => (i === r.points.length - 1 ? q : clampInside(hull, q, 0.02)));
  for (const key of Object.keys(anchors)) anchors[key] = clampInside(hull, anchors[key]!, 0.03);

  return { hull, frontAxle, rearAxle, wheels, parts, tubes, ecus, routes, obd, hub, anchors };
}

function across3(engine: EngineLayout): "x" | "z" {
  return engine === EngineLayout.FrontLongitudinal ? "x" : "z";
}

function dedupe(points: Vec3[]): Vec3[] {
  const out: Vec3[] = [];
  for (const p of points) {
    const last = out[out.length - 1];
    if (last && Math.abs(last[0] - p[0]) + Math.abs(last[1] - p[1]) + Math.abs(last[2] - p[2]) < 1e-3) continue;
    out.push(p);
  }
  return out;
}

interface PlaceContext {
  frontAxle: number;
  rearAxle: number;
  cowlX: number;
  clear: number;
  floor: number;
  engineCenter: Vec3;
  gearboxCenter: Vec3;
  combustion: boolean;
  battery12: Vec3;
  hv: Vec3 | null;
  motor: Vec3 | null;
  frontSeatX: number;
}

// Where each place is in this car, always inside the hull.
export function placeFinder(hull: Hull, c: PlaceContext): (place: EcuPlace) => Vec3 {
  const side = (x: number, y: number, inset: number) => hull.halfWidth(hull.station(x), y) - inset;
  return (place) => {
    let p: Vec3;
    switch (place) {
      case "engineBay":
        p = c.combustion ? [c.engineCenter[0] - 0.05, c.engineCenter[1] + 0.32, c.engineCenter[2] + 0.45] : [c.frontAxle + 0.1, c.clear + 0.55, 0.3];
        break;
      case "gearbox":
        p = c.combustion ? [c.gearboxCenter[0], c.gearboxCenter[1] + 0.26, c.gearboxCenter[2]] : c.motor ? [c.motor[0], c.motor[1] + 0.25, c.motor[2] + 0.2] : [c.frontAxle, c.clear + 0.4, 0.2];
        break;
      case "bulkhead":
        p = [c.cowlX + 0.16, c.clear + 0.5, -0.45];
        break;
      case "tunnel":
        p = [c.frontSeatX + 0.15, c.floor + 0.1, 0];
        break;
      case "steeringColumn":
        p = [c.cowlX - 0.32, c.clear + 0.6, 0.37];
        break;
      case "cluster":
        p = [c.cowlX - 0.2, c.clear + 0.76, 0.37];
        break;
      case "glovebox":
        p = [c.cowlX - 0.3, c.clear + 0.58, -0.4];
        break;
      case "footwell":
        p = [c.cowlX - 0.32, c.clear + 0.4, side(c.cowlX - 0.32, c.clear + 0.4, 0.12)];
        break;
      case "dashCentre":
        p = [c.cowlX - 0.2, c.clear + 0.62, 0];
        break;
      case "headlight":
        p = [hull.front - 0.3, c.clear + 0.5, side(hull.front - 0.3, c.clear + 0.5, 0.18)];
        break;
      case "driverDoor":
        p = [c.frontSeatX + 0.1, c.clear + 0.55, side(c.frontSeatX + 0.1, c.clear + 0.55, 0.07)];
        break;
      case "underSeat":
        p = [c.frontSeatX, c.floor + 0.06, 0.37];
        break;
      case "boot":
        p = [c.rearAxle - 0.35, c.clear + 0.42, 0.35];
        break;
      case "battery":
        p = c.hv ? [c.hv[0] + 0.3, c.hv[1] + 0.02, 0] : [c.battery12[0], c.battery12[1] + 0.16, c.battery12[2]];
        break;
      case "rearBumper":
        p = [hull.rear + 0.22, c.clear + 0.32, 0];
        break;
      case "inverter":
        p = c.motor ? [c.motor[0], c.motor[1] + 0.4, c.motor[2] - 0.25] : [c.frontAxle + 0.25, c.clear + 0.6, -0.3];
        break;
    }
    return clampInside(hull, p, 0.09);
  };
}

// --- Zones of fault codes -------------------------------------------------------------

// The anchor of a zone and the ids of the parts and tubes it lights up.
export function zoneAnchor(layout: CarLayout, zone: CarZone): Vec3 {
  const a = layout.anchors;
  switch (zone) {
    case CarZone.Intake: return a.intake ?? a.engine!;
    case CarZone.Throttle: return a.throttle ?? a.engine!;
    case CarZone.Injection: return a.injection ?? a.engine!;
    case CarZone.Ignition: return a.ignition ?? a.engine!;
    case CarZone.Cylinders: return a.engine!;
    case CarZone.Cooling: return a.cooling!;
    case CarZone.Turbo: return a.turbo ?? a.exhaust ?? a.engine!;
    case CarZone.Exhaust: return a.exhaust ?? a.engine!;
    case CarZone.FuelTank: return a.fuel ?? a.hv ?? a.engine!;
    case CarZone.Gearbox: return a.gearbox ?? a.motor ?? a.engine!;
    case CarZone.Ecu: return a.ecu!;
    case CarZone.Electrics: return a.battery!;
    case CarZone.Network: return a.network!;
    case CarZone.Wheels: {
      const w = layout.wheels[0]!;
      return [w.center[0], w.center[1] + w.radius, w.center[2]];
    }
    case CarZone.Cabin: return a.cabin!;
  }
  return [0, layout.hull.height * 0.5, 0];
}

// The ids of everything that belongs to a zone.
export function zoneMembers(layout: CarLayout, zone: CarZone): string[] {
  const out: string[] = [];
  for (const p of layout.parts) if (p.zones.includes(zone)) out.push(p.id);
  for (const t of layout.tubes) if (t.zones.includes(zone)) out.push(t.id);
  return out;
}
