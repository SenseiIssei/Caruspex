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
import { buildMotoLayout, type MotoSpec, type MotoStyle } from "./moto";
import { bodyGeometry, shapeFor, type Profile, type Shape } from "./shape";

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
  | "motor"
  // Bodywork of a motorcycle, drawn in the glass of the car body; "screen"
  // is a windscreen (darker glass).
  | "shell"
  | "screen"
  | "seat"
  // Headlight and tail light.
  | "lamp"
  | "rearlamp";

export interface Part {
  id: string;
  layer: Layer;
  // "box" with rounded edges, a cylinder along an axis, or an ellipsoid
  // filling the box.
  shape: "box" | "cylinder" | "ellipsoid";
  axis: "x" | "y" | "z";
  center: Vec3;
  // Box and ellipsoid: the full extents. Cylinder: [length, diameter,
  // diameter] along its axis.
  size: Vec3;
  material: Material;
  zones: CarZone[];
  // Turned about the z axis (across the car) in radians, positive lifts the
  // front end: a raked fork, a leaning cylinder, a windscreen.
  tilt?: number;
  // Corner radius of a box as a share of its smallest side (0.22 default,
  // 0.5 for a pill).
  round?: number;
  // Where the part moves in the exploded view of the drivetrain (metres).
  explode?: Vec3;
}

export interface Tube {
  id: string;
  layer: Layer;
  points: Vec3[];
  // More strands drawn as one mesh with the first (the tubes of a frame).
  branches?: Vec3[][];
  radius: number;
  material: Material;
  zones: CarZone[];
  explode?: Vec3;
}

export interface Wheel {
  center: Vec3;
  radius: number;
  width: number;
  front: boolean;
  driven: boolean;
  // A motorcycle wheel: its brake discs (side +1 left, -1 right, and the
  // radius) and its rim.
  moto?: { discs: { side: number; radius: number }[]; spokes: "cast" | "wire" };
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
  | "passengerDoor"
  | "underSeat"
  | "passengerSeat"
  | "boot"
  | "battery"
  | "rearBumper"
  | "inverter"
  | "roof"
  | "windscreen"
  | "fuelPump"
  | "rearAxle";

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

// Places the name of a unit gives away, before the place of its kind. The
// abbreviations in brackets come with the names of data/ecus/*.csv in every
// language (REM is BMW's rear electronics, FZD its roof function centre,
// KAFAS the camera behind the mirror, EKPM the fuel pump control, GHAS the
// rear differential lock, DDM and PDM Ford's door modules); the words cover
// German and English names a unit reports itself.
const PLACE_WORDS: [RegExp, EcuPlace, number[] | null][] = [
  [/\bREM\b|HECKELEKTRONIK|REAR ELECTRONIC|\bSAM-?H\b/, "boot", null],
  [/\bFZD\b|\bDACH|\bROOF/, "roof", null],
  [/\bKAFAS\b|FRONTKAMERA|FRONT CAMERA|\bKAMERA FAHRERASSISTENZ|CAMERA DRIVER ASSIST/, "windscreen", null],
  [/\bEKPM\b|KRAFTSTOFFPUMPE|FUEL PUMP/, "fuelPump", null],
  [/\bGHAS\b|DIFFERENTIAL/, "rearAxle", null],
  [/\bAMP\b|VERSTÄRKER|AMPLIFIER/, "boot", null],
  [/\bSCCM\b|LENKSÄULE|STEERING COLUMN/, "steeringColumn", null],
  [/\bPDM\b|BEIFAHRERTÜR|PASSENGER DOOR/, "passengerDoor", null],
  [/BEIFAHRER|PASSENGER/, "passengerSeat", [EcuKind.Seats]],
];

// Where a maker puts a unit differently, only where the data says so:
// BMW's gateway is part of the front electronics module (FEM/ZGM in
// data/ecus/bmw.csv, the same module as the body electronics FEM/BDC), so
// it sits with the body unit in the footwell rather than behind the
// glovebox.
const BRAND_PLACES: { brands: string[]; kind: number; place: EcuPlace }[] = [
  { brands: ["BMW", "MINI", "Mini", "Alpina"], kind: EcuKind.Gateway, place: "footwell" },
];

// The place of one unit: by its name, by the maker, by its kind. mirror:
// the second of a pair (the second light module) on the passenger side.
export function placeFor(kind: number, name: string, brand = ""): { place: EcuPlace; mirror: boolean } {
  const upper = name.toUpperCase();
  const mirror = kind === EcuKind.Lighting && /\b2\b/.test(upper);
  for (const [re, place, kinds] of PLACE_WORDS) {
    if ((kinds === null || kinds.includes(kind)) && re.test(upper)) return { place, mirror };
  }
  const row = BRAND_PLACES.find((r) => r.kind === kind && r.brands.includes(brand));
  if (row) return { place: row.place, mirror };
  return { place: ECU_PLACES[kind] ?? "boot", mirror };
}

// The buses a scan can report: the CANs behind the gateway (drivetrain,
// comfort, infotainment), the diagnosis CAN from the socket to the gateway,
// the K-line, and the second CANs on the socket that an OBDLink reaches:
// Ford's MS-CAN (pins 3 and 11) and GM's single wire CAN (pin 1).
export type BusKind = "powertrain" | "comfort" | "infotainment" | "diagnostic" | "kline" | "mscan" | "swcan";
export const BUSES: BusKind[] = ["powertrain", "comfort", "infotainment", "diagnostic", "kline", "mscan", "swcan"];

// The generic address sweep finds units it has no name for; the name they
// report themselves (DID F197, "INSTRUMENTS", "AIRBAG 8CH") usually says
// what they are.
const KIND_WORDS: [RegExp, number][] = [
  [/AIRBAG|SRS|RESTRAINT/, EcuKind.Airbag],
  [/INSTRUMENT|KOMBI|CLUSTER|IPC\b/, EcuKind.Instruments],
  [/CLIMA|KLIMA|HVAC|AIR ?CON/, EcuKind.Climate],
  [/\bABS\b|\bESP\b|\bESC\b|BRAKE|BREMS/, EcuKind.Brakes],
  [/GATEWAY|\bGW\b|\bZGW\b/, EcuKind.Gateway],
  [/STEER|LENK|\bEPS\b/, EcuKind.Steering],
  [/RADIO|NAVI|INFOTAIN|\bMMI\b|HEAD ?UNIT|AUDIO/, EcuKind.Infotainment],
  [/\bBCM\b|BODY|ZENTRAL|\bBSI\b|\bSAM\b/, EcuKind.Body],
  [/PARK|\bPDC\b|\bPAS\b/, EcuKind.Parking],
  [/DOOR|TUER|TÜR/, EcuKind.Doors],
  [/SEAT|SITZ/, EcuKind.Seats],
  [/LIGHT|LICHT|LAMP/, EcuKind.Lighting],
  [/TRANS|GEARBOX|GETRIEBE|\bTCM\b|\bTCU\b|\bDSG\b/, EcuKind.Gearbox],
  [/ENGINE|MOTOR|\bECM\b|\bPCM\b|\bDME\b|\bDDE\b/, EcuKind.Engine],
  [/\bBMS\b|BATTER/, EcuKind.Battery],
  [/CHARG|\bOBC\b|LADE/, EcuKind.Charger],
  [/HYBRID|INVERTER/, EcuKind.Hybrid],
];

export function guessKind(kind: number, name: string): number {
  if (kind !== EcuKind.Other) return kind;
  const upper = name.toUpperCase();
  return KIND_WORDS.find(([re]) => re.test(upper))?.[1] ?? kind;
}

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
    case EcuKind.Infotainment:
      return "infotainment";
  }
  return "comfort";
}

// The bus of one unit: how it was read first (K-line, the MS-CAN or SW-CAN
// of an OBDLink), then by its kind.
export function busFor(e: EcuInput, kline: boolean): BusKind {
  if (e.source === "kline" || (kline && e.source === "obd")) return "kline";
  if (e.bus === "ms") return "mscan";
  if (e.bus === "sw") return "swcan";
  return busForKind(e.kind);
}

// Buses that start at the socket itself instead of behind the gateway.
export const SOCKET_BUSES: BusKind[] = ["kline", "mscan", "swcan"];

export interface EcuInput {
  key: string;
  kind: number;
  name: string;
  faulty: boolean;
  // How it was read: a UDS scan, plain OBD, or the K-line.
  source: "uds" | "obd" | "kline";
  // Being asked by a running scan, not answered yet.
  asking?: boolean;
  // The CAN a UDS scan found it on: "hs", "ms" (Ford MS-CAN), "sw" (GM SW-CAN).
  bus?: string;
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

// Where the name of a bus stands in the view: on its trunk.
export interface BusLabel {
  bus: BusKind;
  anchor: Vec3;
}

// --- The diagnostic socket -------------------------------------------------------

// SocketSpot of the catalog (data/obd_sockets.csv) in declaration order.
export const SocketSpot = {
  Unknown: 0, DashLeftOfColumn: 1, DashRightOfColumn: 2, AbovePedals: 3, BelowColumn: 4, FootwellLeft: 5,
  FuseBoxLower: 6, ConsoleFront: 7, ConsoleSide: 8, Passenger: 9, RearConsole: 10, EngineBay: 11, UnderSeat: 12,
} as const;

// The exact spot of the socket: the one of data/obd_sockets.csv, else the
// coarse SocketLocation turned into its most likely spot, else the most
// common one (under the dashboard left of the steering column).
export function socketSpotOf(location: number, spot: number): number {
  if (spot > 0 && spot <= SocketSpot.UnderSeat) return spot;
  switch (location) {
    case 1: return SocketSpot.DashLeftOfColumn;
    case 2: return SocketSpot.BelowColumn;
    case 3: return SocketSpot.FootwellLeft;
    case 4: return SocketSpot.FuseBoxLower;
    case 5:
    case 6: return SocketSpot.ConsoleFront;
    case 7: return SocketSpot.Passenger;
    case 8: return SocketSpot.EngineBay;
    case 9: return SocketSpot.RearConsole;
    case 10: return SocketSpot.UnderSeat;
  }
  return SocketSpot.DashLeftOfColumn;
}

// The socket with the adapter in it. For a motorcycle the maker's plug sits
// under the seat and an adapter cable leads to the 16-pin socket the
// adapter is plugged into.
export interface Adapter {
  // The 16-pin socket and the direction the adapter sticks out of it.
  socket: Vec3;
  dir: Vec3;
  // The adapter cable from the maker's plug to that socket, null in a car.
  cable: Vec3[] | null;
}

// The profile of the body comes from shape.ts (defaults per body type and
// era, hand-tuned rows for common generations).
export type { Profile } from "./shape";

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
  // Height of the window line at the windscreen.
  belt: number;
  // x of the front and rear end.
  front: number;
  rear: number;
  // Station 0..1 (rear to front) of an x, and back.
  station(x: number): number;
  xAt(u: number): number;
  top(u: number): number;
  bottom(u: number): number;
  // Height of the window line at a station (it may rise towards the back).
  beltAt(u: number): number;
  // Half the width in plan view at a station.
  planHalf(u: number): number;
  // Half the width at a station and height; 0 outside the hull.
  halfWidth(u: number, y: number): number;
  // One point of the cross-section, theta from -pi/2 (bottom) round to 3pi/2.
  section(u: number, theta: number): Vec3;
}

const SECTION_EXPONENT = 5;

export function makeHull(lengthM: number, widthM: number, heightM: number, profile: Profile): Hull {
  const topCurve = monotone(profile.top);
  const height = heightM;
  const front = lengthM / 2;
  const rear = -lengthM / 2;
  const belt = profile.belt * height;
  const beltRear = profile.beltRear * height;
  const clearance = profile.clearance;
  const dloRear = profile.dlo[2];
  const beltAt = (u: number) => beltRear + (belt - beltRear) * smoothstep(dloRear, profile.cowl, u);
  // The underbody rises from behind the rear wheels and ahead of the front ones.
  const bottom = (u: number) =>
    clearance +
    0.17 * (1 - smoothstep(0, Math.max(0.02, profile.rearLift), u)) +
    0.15 * smoothstep(Math.min(0.98, profile.frontLift), 1, u) +
    0.02 * smoothstep(0.75, 0.95, u);
  const top = (u: number) => topCurve(u) * height;
  const planHalf = (u: number) => {
    const s = Math.abs(2 * u - 1);
    const p = u > 0.5 ? profile.noseRound : profile.tailRound;
    return (widthM / 2) * Math.pow(Math.max(0, 1 - Math.pow(s, p)), 1 / p);
  };
  // The glasshouse leans in above the window line. Over a bonnet or boot
  // lower than that line there is no glass: smoothstep with its edges the
  // wrong way round gave 1 there and pulled the whole nose in by the
  // tumblehome, so the front wheels were drawn up to a quarter too close.
  const tumble = (y: number, u: number) => {
    const from = beltAt(u) - 0.05;
    const to = top(u);
    return to <= from ? 1 : 1 - profile.tumblehome * smoothstep(from, to, y);
  };
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
    beltAt,
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

// --- Lamps and grille -------------------------------------------------------------

// The front or rear face at one height in plan view: from the centre line
// (z = 0) round the corner and back along the side, with the distance along
// that line and where the corner is (where the line turns more along the car
// than across it).
interface FaceArc {
  x: number[];
  z: number[];
  s: number[];
  corner: number;
  end: "front" | "rear";
}

function faceArc(hull: Hull, y: number, end: "front" | "rear"): FaceArc | null {
  const n = 160;
  const x: number[] = [];
  const z: number[] = [];
  // Never back past the wheel arches: a lamp on a car without overhangs
  // stays on its corner.
  const stop = end === "front" ? hull.profile.frontLift : hull.profile.rearLift;
  for (let i = 0; i <= n; i++) {
    const u = end === "front" ? 1 - (0.35 * i) / n : (0.35 * i) / n;
    if (x.length > 2 && (end === "front" ? u < stop : u > stop)) break;
    const half = hull.halfWidth(u, y);
    if (half <= 0) {
      if (x.length) break;
      continue;
    }
    if (!x.length) {
      x.push(hull.xAt(u));
      z.push(0);
    }
    x.push(hull.xAt(u));
    z.push(half);
  }
  if (x.length < 3) return null;
  const s = [0];
  let corner = -1;
  for (let i = 1; i < x.length; i++) {
    const dx = Math.abs(x[i]! - x[i - 1]!);
    const dz = z[i]! - z[i - 1]!;
    s.push(s[i - 1]! + Math.hypot(dx, dz));
    if (corner < 0 && i > 1 && dx > Math.abs(dz)) corner = s[i - 1]!;
  }
  return { x, z, s, corner: corner < 0 ? s[s.length - 1]! : corner, end };
}

// The point at a distance along the face, a few millimetres out from the
// surface so the line stays visible on the glass.
function arcPoint(arc: FaceArc, dist: number, y: number, side: number): Vec3 {
  const { x, z, s } = arc;
  const d = Math.min(s[s.length - 1]!, Math.max(0, dist));
  let i = 0;
  while (i < s.length - 2 && s[i + 1]! < d) i++;
  const seg = Math.max(1e-6, s[i + 1]! - s[i]!);
  const t = (d - s[i]!) / seg;
  const tx = x[i + 1]! - x[i]!;
  const tz = z[i + 1]! - z[i]!;
  const len = Math.max(1e-6, Math.hypot(tx, tz));
  const [nx, nz] = arc.end === "front" ? [tz / len, -tx / len] : [-tz / len, tx / len];
  const out = 0.006;
  return [x[i]! + tx * t + nx * out, y, side * (z[i]! + tz * t + nz * out)];
}

export interface LampOutlines {
  // Closed polylines on the body: headlights, tail lights and the openings
  // in the front (grille and intake).
  head: Vec3[][];
  tail: Vec3[][];
  grille: Vec3[][];
}

// The lamps and the grille of the profile drawn on the hull: each an
// outline measured along the face from the centre line, so a lamp that
// wraps round the corner follows the body on to the side.
export function lampOutlines(hull: Hull): LampOutlines {
  const p = hull.profile;
  const cache = new Map<string, FaceArc | null>();
  const arcAt = (y: number, end: "front" | "rear") => {
    const key = `${end}${Math.round(y * 200)}`;
    if (!cache.has(key)) cache.set(key, faceArc(hull, Math.round(y * 200) / 200, end));
    return cache.get(key)!;
  };
  // A rounded rectangle (superellipse) in (distance along the face, height),
  // centred at sc with half sizes rs and ry; dist < 0 is the other side.
  const loop = (end: "front" | "rear", sc: number, rs: number, yc: number, ry: number, round: number, rise: number, side: number): Vec3[] => {
    const pts: Vec3[] = [];
    const e = 2 / Math.max(1.5, round);
    for (let i = 0; i <= 48; i++) {
      const a = (2 * Math.PI * i) / 48;
      const c = Math.cos(a);
      const sn = Math.sin(a);
      const d = sc + rs * Math.sign(c) * Math.pow(Math.abs(c), e);
      const q = rs > 0 ? (d - (sc - rs)) / (2 * rs) : 0.5;
      const y = yc + ry * Math.sign(sn) * Math.pow(Math.abs(sn), e) + rise * (q - 0.5);
      const arc = arcAt(y, end);
      if (!arc) return [];
      pts.push(arcPoint(arc, Math.abs(d), y, d < 0 ? -side : side));
    }
    return pts;
  };
  const faceBottom = (end: "front" | "rear") => hull.bottom(end === "front" ? 0.985 : 0.015);
  const lamps = (end: "front" | "rear") => {
    const lamp = end === "front" ? p.headlight : p.taillight;
    // A low face (a wedge nose) gets a lamp in proportion.
    const faceTop = end === "front" ? p.noseTop : p.tailTop;
    const room = faceTop - (faceBottom(end) + 0.06);
    const k = lamp.drop + lamp.height > room && lamp.drop > 0 ? Math.max(0.2, room / (lamp.drop + lamp.height)) : 1;
    const top = faceTop - lamp.drop * k;
    const bottom = Math.max(faceBottom(end) + 0.06, top - lamp.height * k);
    const mid = arcAt((top + bottom) / 2, end);
    if (!mid || top - bottom < 0.02) return [];
    const s1 = Math.min(mid.s[mid.s.length - 1]!, mid.corner + lamp.wrap);
    const s0 = Math.max(0.05, s1 - lamp.width);
    if (s1 - s0 < 0.03) return [];
    const yc = (top + bottom) / 2;
    const ry = (top - bottom) / 2;
    const out: Vec3[][] = [];
    for (const side of [1, -1]) {
      if (lamp.twin) {
        const m = (s0 + s1) / 2;
        const r = Math.min((s1 - s0) / 4 - 0.01, ry);
        out.push(loop(end, (s0 + m) / 2, r, yc, ry, lamp.round, lamp.rise / 2, side));
        out.push(loop(end, (m + s1) / 2, r, yc, ry, lamp.round, lamp.rise / 2, side));
      } else {
        out.push(loop(end, (s0 + s1) / 2, (s1 - s0) / 2, yc, ry, lamp.round, lamp.rise, side));
      }
    }
    if (lamp.bar) {
      const bar: Vec3[] = [];
      for (let i = 0; i <= 24; i++) {
        const d = -s0 + (2 * s0 * i) / 24;
        bar.push(arcPoint(mid, Math.abs(d), yc - lamp.rise / 2, d < 0 ? -1 : 1));
      }
      out.push(bar);
    }
    return out.filter((l) => l.length > 1);
  };
  const grille = (): Vec3[][] => {
    const g = p.grille;
    const out: Vec3[][] = [];
    const top = p.noseTop - g.drop;
    const bottom = Math.max(faceBottom("front") + 0.1, top - g.height);
    if (g.openings > 0 && top - bottom > 0.02 && g.width > 0.02) {
      const yc = (top + bottom) / 2;
      const ry = (top - bottom) / 2;
      if (g.openings === 2) {
        const half = g.width / 2;
        out.push(loop("front", half + 0.012, half - 0.018, yc, ry, g.round, 0, 1));
        out.push(loop("front", -(half + 0.012), half - 0.018, yc, ry, g.round, 0, 1));
      } else {
        out.push(loop("front", 0, g.width, yc, ry, g.round, 0, 1));
      }
    }
    if (g.intakeWidth > 0.02 && g.intakeHeight > 0.02) {
      const low = faceBottom("front") + 0.05;
      const high = Math.min(low + g.intakeHeight, g.openings > 0 ? bottom - 0.04 : p.noseTop - 0.12);
      if (high - low > 0.025) out.push(loop("front", 0, g.intakeWidth, (low + high) / 2, (high - low) / 2, 6, 0, 1));
    }
    return out.filter((l) => l.length > 1);
  };
  return { head: lamps("front"), tail: lamps("rear"), grille: grille() };
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
  // The body variant to draw (an estate of a saloon generation); omitted:
  // the catalog's body.
  variant?: BodyType;
  // The kind of motorcycle to draw (naked, sport, touring ...); omitted: the
  // one moto.ts knows for the model.
  motoStyle?: MotoStyle;
}

export interface CarLayout {
  vehicle: "car" | "moto";
  hull: Hull;
  shape: Shape;
  frontAxle: number;
  rearAxle: number;
  wheels: Wheel[];
  parts: Part[];
  tubes: Tube[];
  ecus: EcuNode[];
  routes: Route[];
  busLabels: BusLabel[];
  // The diagnostic socket (on a motorcycle the maker's plug), the spot it
  // was put at (SocketSpot) and the adapter in it.
  obd: Vec3;
  socketSpot: number;
  adapter: Adapter;
  hub: Vec3;
  // Positions for the zones of fault codes and for the live labels.
  anchors: Record<string, Vec3>;
  // The motorcycle drawn (kind, engine, final drive), null for a car.
  moto: MotoSpec | null;
}

export const box = (id: string, layer: Layer, material: Material, center: Vec3, size: Vec3, zones: CarZone[] = []): Part => ({
  id,
  layer,
  shape: "box",
  axis: "x",
  center,
  size,
  material,
  zones,
});

export const cylinder = (
  id: string,
  layer: Layer,
  material: Material,
  axis: "x" | "y" | "z",
  center: Vec3,
  length: number,
  diameter: number,
  zones: CarZone[] = [],
): Part => ({ id, layer, shape: "cylinder", axis, center, size: [length, diameter, diameter], material, zones });

// --- Control units and their lines, for cars and motorcycles --------------------------

export interface NetworkFrame {
  // The base position of a place in this vehicle.
  placeAt: (place: EcuPlace) => Vec3;
  // The line between two points along the harness of this vehicle.
  route: (from: Vec3, to: Vec3, bus: BusKind) => Vec3[];
  // Pulls a point inside the vehicle.
  clamp: (p: Vec3) => Vec3;
  obd: Vec3;
  // Spacing of several units at one place.
  step: number;
}

// Every unit at its place with its bus, the gateway as the hub, and one
// line per unit: from the gateway, or from the socket for the K-line, the
// MS-CAN and the SW-CAN, which do not pass the gateway.
export function buildNetwork(input: XrayInput, frame: NetworkFrame): { ecus: EcuNode[]; routes: Route[]; hub: Vec3; busLabels: BusLabel[] } {
  const used = new Map<string, number>();
  const brand = input.anatomy.brand;
  const ecus: EcuNode[] = input.ecus.map((raw) => {
    const e = { ...raw, kind: guessKind(raw.kind, raw.name) };
    const { place, mirror } = placeFor(e.kind, e.name, brand);
    const key = mirror ? `${place}-2` : place;
    const n = used.get(key) ?? 0;
    used.set(key, n + 1);
    const base = frame.placeAt(place);
    const b: Vec3 = mirror ? [base[0], base[1], -base[2]] : base;
    // Several units at one place stand next to each other.
    const s = frame.step;
    const shifted: Vec3 = [b[0] - Math.floor(n / 2) * s * 1.2, b[1] + (n % 2) * s, b[2] - (n > 0 ? Math.sign(b[2] || 1) * s * 0.92 * Math.ceil(n / 2) : 0)];
    return { ...e, place, position: frame.clamp(shifted), bus: busFor(e, input.kline) };
  });
  const gateway = ecus.find((e) => e.kind === EcuKind.Gateway);
  const hub = gateway ? gateway.position : frame.obd;
  const routes: Route[] = [];
  if (gateway) routes.push({ bus: input.kline ? "kline" : "diagnostic", ecu: "", points: frame.route(frame.obd, hub, "diagnostic") });
  for (const e of ecus) {
    if (e === gateway) continue;
    const start = SOCKET_BUSES.includes(e.bus) ? frame.obd : hub;
    routes.push({ bus: e.bus, ecu: e.key, points: frame.route(start, e.position, e.bus) });
  }
  return { ecus, routes, hub, busLabels: labelBuses(routes) };
}

// One name per bus, in the middle of the longest run of its first line.
function labelBuses(routes: Route[]): BusLabel[] {
  const out: BusLabel[] = [];
  for (const r of routes) {
    if (out.some((l) => l.bus === r.bus) || r.points.length < 2) continue;
    let best = 0;
    let at: Vec3 = r.points[0]!;
    for (let i = 0; i + 1 < r.points.length; i++) {
      const p = r.points[i]!;
      const q = r.points[i + 1]!;
      const d = Math.hypot(q[0] - p[0], q[1] - p[1], q[2] - p[2]);
      if (d > best) {
        best = d;
        at = [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2 + 0.015, (p[2] + q[2]) / 2];
      }
    }
    out.push({ bus: r.bus, anchor: at });
  }
  return out;
}

// --- The car ----------------------------------------------------------------------------

export function buildLayout(input: XrayInput): CarLayout {
  if (input.body === BodyType.Motorcycle) return buildMotoLayout(input);
  const a = input.anatomy;
  const L = a.lengthMm / 1000;
  const W = a.widthMm / 1000;
  const H = a.heightMm / 1000;
  const wheelbase = Math.min(a.wheelbaseMm / 1000, L * 0.8);
  const track = Math.min(a.trackMm / 1000, W - 0.2);
  const body = input.variant ?? input.body;
  const shape = shapeFor(a, input.body, body);
  const geometry = bodyGeometry(L, H, wheelbase, shape.wheel / 2, shape);
  const hull = makeHull(L, W, H, geometry.profile);
  const p = hull.profile;
  const { frontAxle, rearAxle } = geometry;
  const radius = geometry.wheelRadius;
  const tyre = body === BodyType.Suv || body === BodyType.Pickup ? 0.24 : body === BodyType.Microcar ? 0.17 : 0.21;
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

  // Moves a box (center, half extents) that sits beside a wheel and below
  // its top in towards the middle until it clears the tyre. The fixed
  // offsets of the engine bay suit a car of average width; a narrow kei car
  // or an old Jimny had its battery and alternator inside the front wheel.
  const inboard = (c: Vec3, half: Vec3): Vec3 => {
    for (const w of wheels) {
      if (Math.abs(c[0] - w.center[0]) >= w.radius + half[0] || c[1] - half[1] >= 2 * w.radius) continue;
      if (c[2] * w.center[2] <= 0) continue;
      const limit = Math.max(0, Math.abs(w.center[2]) - w.width / 2 - 0.02 - half[2]);
      if (Math.abs(c[2]) > limit) c = [c[0], c[1], Math.sign(c[2]) * limit];
    }
    return c;
  };
  // Inner face of the rear tyres: the rear bench sits between the arches.
  const rearArch = Math.abs(wheels[2]!.center[2]) - tyre / 2 - 0.02;

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
    const bench = Math.min(W * 0.72, 2 * rearArch);
    parts.push(box("seat-rear", "body", "cabin", [rearSeatX, seatY + 0.02, 0], [0.48, 0.12, bench], [CarZone.Cabin]));
    parts.push(box("back-rear", "body", "cabin", [rearSeatX - 0.28, seatY + 0.33, 0], [0.11, 0.56, bench], [CarZone.Cabin]));
  }
  parts.push(cylinder("steering-wheel", "body", "cabin", "x", [dashX - 0.32, clear + 0.78, lhd * 0.37], 0.04, 0.38, [CarZone.Cabin]));
  anchors.cabin = [dashX, clear + 0.75, 0];

  // --- Drivetrain.
  const combustion = a.engine !== EngineLayout.None;
  let engineCenter: Vec3 = [frontAxle, clear + 0.45, 0];
  let gearboxCenter: Vec3 = [frontAxle, clear + 0.3, 0];
  let blockSize: Vec3 = [0.4, 0.42, 0.5];
  let coilLine: [Vec3, Vec3] | null = null;
  const across = a.engine === EngineLayout.FrontTransverse || a.engine === EngineLayout.Mid || a.engine === EngineLayout.Rear;
  const intakeDir = across ? (a.engine === EngineLayout.FrontTransverse ? -1 : 1) : 1;
  const exhaustSide = across ? -intakeDir : -1;
  const engineTop = (c: Vec3, h: number) => c[1] + h / 2;
  if (combustion) {
    const blockH = 0.42;
    if (a.engine === EngineLayout.FrontTransverse) {
      engineCenter = [frontAxle + 0.12, clear + 0.38, -0.1];
      blockSize = [0.36, blockH, 0.52];
      gearboxCenter = inboard([frontAxle - 0.02, clear + 0.3, 0.3], [0.17, 0.17, 0.18]);
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
    const wide = blockSize[2] > blockSize[0];
    for (let i = 0; i < 4; i++) {
      const t = (i - 1.5) * (wide ? blockSize[2] : blockSize[0]) * 0.2;
      const c: Vec3 = wide ? [head[0], headY + 0.1, head[2] + t] : [head[0] + t, headY + 0.1, head[2]];
      parts.push(cylinder(`coil-${i}`, "drivetrain", "electric", "y", c, 0.1, 0.045, [CarZone.Ignition]));
    }
    const coilSpan = (wide ? blockSize[2] : blockSize[0]) * 0.36;
    coilLine = wide
      ? [[head[0] - 0.09, headY + 0.13, head[2] + coilSpan], [head[0] - 0.09, headY + 0.13, head[2] - coilSpan]]
      : [[head[0] + coilSpan, headY + 0.13, head[2] + 0.09], [head[0] - coilSpan, headY + 0.13, head[2] + 0.09]];
    // Intake side (towards the cabin for a transverse engine) with the
    // manifold, the throttle body and the air box.
    const intakeOffset = wide ? blockSize[0] / 2 + 0.07 : blockSize[2] / 2 + 0.07;
    const manifold: Vec3 = wide
      ? [engineCenter[0] + intakeDir * intakeOffset, headY - 0.04, engineCenter[2]]
      : [engineCenter[0], headY - 0.04, engineCenter[2] + intakeDir * intakeOffset];
    parts.push(box("intake-manifold", "drivetrain", "metal", manifold, wide ? [0.1, 0.16, blockSize[2] * 0.8] : [blockSize[0] * 0.8, 0.16, 0.1], [CarZone.Intake]));
    const rail: Vec3 = wide ? [manifold[0] - intakeDir * 0.04, manifold[1] - 0.09, manifold[2]] : [manifold[0], manifold[1] - 0.09, manifold[2] - intakeDir * 0.04];
    parts.push(cylinder("fuel-rail", "drivetrain", "fuel", wide ? "z" : "x", rail, (wide ? blockSize[2] : blockSize[0]) * 0.75, 0.03, [CarZone.Injection]));
    const throttle: Vec3 = wide ? [manifold[0], manifold[1] + 0.02, manifold[2] + blockSize[2] * 0.45] : [manifold[0] + blockSize[0] * 0.45, manifold[1] + 0.02, manifold[2]];
    parts.push(cylinder("throttle", "drivetrain", "metal", wide ? "z" : "x", throttle, 0.09, 0.09, [CarZone.Throttle]));
    let airbox: Vec3 = a.engine === EngineLayout.FrontTransverse ? [engineCenter[0] + 0.05, headY + 0.08, 0.38] : [engineCenter[0] + 0.1, headY + 0.08, -0.38];
    airbox = inboard(clampInside(hull, airbox, 0.12), [0.15, 0.07, 0.1]);
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
    const exManifold: Vec3 = wide
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
      const turbo: Vec3 = [exManifold[0] + (wide ? exhaustSide * 0.06 : 0), exManifold[1] - 0.06, exManifold[2] + (wide ? 0.12 : -0.08)];
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
        explode: [0, 0.05, side * 0.3],
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
  const radH = Math.min(0.32, (hull.top(radiatorU) - hull.bottom(radiatorU)) * 0.45);
  const radY = (hull.top(radiatorU) + hull.bottom(radiatorU)) / 2 - 0.02;
  const radW = Math.min(W * 0.44, 2 * (Math.min(hull.halfWidth(radiatorU, radY - radH / 2), hull.halfWidth(radiatorU, radY + radH / 2)) - 0.06));
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

  // --- Electrics: the main harness as strands. The 12 V battery with the
  // main fuses on its plus pole and the ground straps to the body and the
  // engine, alternator and starter, the engine harness to the coils, the
  // strand through the bulkhead to the fuse box and along the sill to the
  // rear lamps, the lamps in front. Where the strands run and where to look,
  // not a wiring diagram.
  const batteryAtBack = a.engine === EngineLayout.FrontLongitudinal && body !== BodyType.Pickup && body !== BodyType.Suv;
  const battery12 = inboard(
    clampInside(
      hull,
      batteryAtBack ? [rearAxle - 0.45, clear + 0.32, -0.45] : combustion && a.engine !== EngineLayout.FrontTransverse ? [frontAxle + 0.15, clear + 0.5, 0.42] : [frontAxle + 0.05, clear + 0.5, 0.44],
      0.12,
    ),
    [0.13, 0.095, 0.085],
  );
  parts.push(box("battery12", "electrics", "electric", battery12, [0.26, 0.19, 0.17], [CarZone.Electrics]));
  anchors.battery = [battery12[0], battery12[1] + 0.12, battery12[2]];
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
  const minusPole: Vec3 = [battery12[0] - 0.08, battery12[1] + 0.1, battery12[2] - 0.04];
  const mainFuses = clampInside(hull, [battery12[0] + 0.06, battery12[1] + 0.13, battery12[2] - 0.05], 0.05);
  parts.push(box("main-fuses", "electrics", "electric", mainFuses, [0.1, 0.05, 0.09], [CarZone.Electrics]));
  const bodyGround = inboard(clampInside(hull, [battery12[0] - 0.2, battery12[1] + 0.05, battery12[2] + Math.sign(battery12[2] || 1) * 0.12], 0.05), [0.02, 0.02, 0.02]);
  parts.push(box("ground-point", "electrics", "metal", bodyGround, [0.04, 0.04, 0.04], [CarZone.Electrics]));
  const ground: Tube = { id: "ground", layer: "electrics", points: [minusPole, [minusPole[0] - 0.06, minusPole[1] + 0.03, minusPole[2]], bodyGround], radius: 0.01, material: "metal", zones: [CarZone.Electrics] };
  tubes.push(ground);
  const fuseBox = clampInside(hull, [cowlX - 0.3, clear + 0.5, lhd * 0.52], 0.08);
  parts.push(box("fuse-box", "electrics", "electric", fuseBox, [0.16, 0.14, 0.06], [CarZone.Electrics]));
  if (combustion) {
    const alternator = inboard(
      clampInside(hull, [engineCenter[0] + (a.engine === EngineLayout.FrontLongitudinal ? 0.36 : 0), engineCenter[1] + 0.02, engineCenter[2] + (a.engine === EngineLayout.FrontLongitudinal ? -0.22 : -0.32)], 0.08),
      [0.08, 0.065, 0.08],
    );
    parts.push(cylinder("alternator", "electrics", "electric", across3(a.engine), alternator, 0.16, 0.13, [CarZone.Electrics]));
    // The starter at the flange between engine and gearbox, low down.
    const starter = clampInside(
      hull,
      a.engine === EngineLayout.FrontLongitudinal
        ? [engineCenter[0] - blockSize[0] / 2 - 0.06, clear + 0.22, -0.2]
        : [(engineCenter[0] + gearboxCenter[0]) / 2 + 0.12, clear + 0.22, (engineCenter[2] + gearboxCenter[2]) / 2],
      0.07,
    );
    parts.push(cylinder("starter", "electrics", "electric", a.engine === EngineLayout.FrontLongitudinal ? "x" : "z", starter, 0.2, 0.09, [CarZone.Electrics]));
    tubes.push({ id: "starter-cable", layer: "electrics", points: [mainFuses, [mainFuses[0], clear + 0.3, mainFuses[2]], [starter[0], clear + 0.3, starter[2]], starter], radius: 0.012, material: "electric", zones: [CarZone.Electrics] });
    ground.branches = [[minusPole, [minusPole[0] - 0.05, engineCenter[1], (minusPole[2] + engineCenter[2]) / 2], [engineCenter[0], engineCenter[1] + 0.05, engineCenter[2] + blockSize[2] / 2 * Math.sign(minusPole[2] || 1) * 0.9]]];
    // The engine harness: from the engine control unit along the coils and
    // injectors, to the alternator and the starter.
    const ecu = placeAt("engineBay");
    const strand: Vec3[] = [ecu, [ecu[0], ecu[1] + 0.02, (ecu[2] + coilLine![0][2]) / 2], coilLine![0], coilLine![1]];
    tubes.push({
      id: "engine-harness",
      layer: "electrics",
      points: strand,
      branches: [[coilLine![0], [alternator[0], coilLine![0][1] - 0.1, alternator[2] * 0.6], alternator], [ecu, [ecu[0], clear + 0.32, ecu[2] * 0.7], starter]],
      radius: 0.012,
      material: "electric",
      zones: [CarZone.Electrics, CarZone.Ignition],
    });
  }
  // The main strand: from the main fuses through the bulkhead to the fuse
  // box, down to the sill on the driver's side and along it to the rear.
  const sillZ = (x: number, y: number) => lhd * (hull.halfWidth(hull.station(x), y) - 0.12);
  const rearEnd = clampInside(hull, [hull.rear + 0.3, clear + 0.55, lhd * 0.5], 0.1);
  tubes.push({
    id: "harness",
    layer: "electrics",
    points: [mainFuses, [mainFuses[0], clear + 0.62, mainFuses[2] * 0.8], [cowlX - 0.05, clear + 0.62, sillZ(cowlX - 0.05, clear + 0.62)], fuseBox, [cowlX - 0.35, floor + 0.04, sillZ(cowlX - 0.35, floor + 0.04)], [rearAxle + 0.1, floor + 0.04, sillZ(rearAxle + 0.1, floor + 0.04)], rearEnd],
    radius: 0.016,
    material: "electric",
    zones: [CarZone.Electrics],
  });
  // The lamps: a strand to each headlight and each tail light.
  const lampAt = (end: "front" | "rear", side: number): Vec3 => {
    const x = end === "front" ? hull.front - 0.22 : hull.rear + 0.16;
    const y = (end === "front" ? p.noseTop : p.tailTop) - 0.08;
    return clampInside(hull, [x, y, side * (hull.halfWidth(hull.station(x), y) - 0.15)], 0.06);
  };
  const frontSplit = clampInside(hull, [Math.min(hull.front - 0.45, mainFuses[0] + 0.3), clear + 0.55, 0], 0.08);
  tubes.push({ id: "harness-front", layer: "electrics", points: [mainFuses, frontSplit, lampAt("front", 1)], branches: [[frontSplit, lampAt("front", -1)]], radius: 0.01, material: "electric", zones: [CarZone.Electrics] });
  const rearSplit = clampInside(hull, [hull.rear + 0.2, rearEnd[1], 0], 0.08);
  tubes.push({ id: "harness-rear", layer: "electrics", points: [rearEnd, lampAt("rear", 1)], branches: [[rearEnd, rearSplit, lampAt("rear", -1)]], radius: 0.01, material: "electric", zones: [CarZone.Electrics] });

  // --- The OBD socket at its spot (data/obd_sockets.csv or the catalog's
  // location), the adapter in it and its feed from the fuse box (pin 16).
  const spot = socketSpotOf(a.socketLocation ?? 0, a.socketSpot ?? 0);
  const socket = carSocket(hull, spot, { cowlX, clear, floor, frontSeatX, lhd });
  const obd = socket.at;
  anchors.obd = obd;
  tubes.push({ id: "obd-feed", layer: "electrics", points: [fuseBox, [fuseBox[0], obd[1] + 0.02, (fuseBox[2] + obd[2]) / 2], obd], radius: 0.006, material: "electric", zones: [CarZone.Electrics, CarZone.Network] });

  // --- Network: the control units and their buses.
  const harnessY = floor + 0.08;
  const lane: Record<BusKind, number> = { powertrain: -0.07, comfort: 0.07, infotainment: 0.17, diagnostic: 0, kline: 0.13, mscan: -0.15, swcan: -0.22 };
  const network = buildNetwork(input, {
    placeAt,
    obd,
    step: 0.13,
    clamp: (q) => clampInside(hull, q, 0.09),
    route: (from, to, bus) => {
      const z = lane[bus];
      const y = Math.max(harnessY, Math.min(from[1], to[1]) - 0.4);
      return dedupe([from, [from[0], y, from[2]], [from[0], y, z], [to[0], y, z], [to[0], y, to[2]], to]);
    },
  });
  anchors.network = network.hub;
  const engineEcu = network.ecus.find((e) => e.kind === EcuKind.Engine);
  anchors.ecu = engineEcu ? engineEcu.position : placeAt("engineBay");

  // The drivetrain taken apart: the engine lifts, its head and coils
  // further, the gearbox moves out, the exhaust out to the right.
  const gearboxOut: Vec3 =
    a.engine === EngineLayout.FrontLongitudinal ? [-0.45, 0.18, 0] : a.engine === EngineLayout.FrontTransverse ? [0, 0.2, 0.55] : [-0.4, 0.2, 0];
  const intakeOut: Vec3 = across ? [intakeDir * 0.3, 0.55, 0] : [0, 0.55, intakeDir * 0.3];
  const explodeOf = (id: string): Vec3 | undefined => {
    const is = (...prefixes: string[]) => prefixes.some((q) => id === q || id.startsWith(`${q}-`));
    if (id === "engine") return [0, 0.42, 0];
    if (is("cylinder-head")) return [0, 0.68, 0];
    if (is("coil")) return [0, 0.92, 0];
    if (is("intake-manifold", "fuel-rail", "throttle")) return intakeOut;
    if (is("airbox", "intake-pipe")) return [intakeOut[0], 0.85, intakeOut[2]];
    if (is("turbo")) return across ? [exhaustSide * 0.35, 0.3, 0] : [0, 0.3, -0.35];
    if (is("gearbox")) return gearboxOut;
    if (is("front-diff")) return [0.3, 0.2, 0];
    if (is("rear-diff")) return [-0.3, 0.2, 0];
    if (is("prop-shaft")) return [0, 0.14, 0];
    if (is("exhaust", "catalyst", "middle-silencer", "rear-silencer")) return [0, 0, -0.75];
    if (is("motor")) return [0, 0.35, 0];
    return undefined;
  };
  for (const part of parts) if (part.layer === "drivetrain") part.explode ??= explodeOf(part.id);
  for (const tube of tubes) if (tube.layer === "drivetrain") tube.explode ??= explodeOf(tube.id);

  // Small or unusual bodies (a Smart, a roadster) leave less room than the
  // offsets above assume: whatever ended up outside is pulled back in.
  for (const part of parts) part.center = clampInside(hull, part.center, 0.03);
  for (const tube of tubes) {
    tube.points = tube.points.map((q) => clampInside(hull, q, tube.radius + 0.01));
    if (tube.branches) tube.branches = tube.branches.map((b) => b.map((q) => clampInside(hull, q, tube.radius + 0.01)));
  }
  const routes = network.routes;
  for (const r of routes) r.points = r.points.map((q, i) => (i === r.points.length - 1 ? q : clampInside(hull, q, 0.02)));
  for (const key of Object.keys(anchors)) anchors[key] = clampInside(hull, anchors[key]!, 0.03);

  return {
    vehicle: "car",
    hull,
    shape,
    frontAxle,
    rearAxle,
    wheels,
    parts,
    tubes,
    ecus: network.ecus,
    routes,
    busLabels: network.busLabels,
    obd,
    socketSpot: spot,
    adapter: { socket: obd, dir: socket.dir, cable: null },
    hub: network.hub,
    anchors,
    moto: null,
  };
}

// The socket of a car at its spot, inside the cabin (in the engine bay for
// the BMW and Mercedes round plugs), and the direction the plugged adapter
// points: down into the footwell under the dashboard, up out of the
// console, sideways out of a side panel.
function carSocket(
  hull: Hull,
  spot: number,
  c: { cowlX: number; clear: number; floor: number; frontSeatX: number; lhd: number },
): { at: Vec3; dir: Vec3 } {
  const side = (x: number, y: number, inset: number) => c.lhd * (hull.halfWidth(hull.station(x), y) - inset);
  const dash = c.clear + 0.47;
  const down: Vec3 = [-0.55, -0.83, 0];
  let at: Vec3;
  let dir: Vec3 = down;
  switch (spot) {
    case SocketSpot.DashRightOfColumn:
      at = [c.cowlX - 0.42, dash, c.lhd * 0.17];
      break;
    case SocketSpot.AbovePedals:
      at = [c.cowlX - 0.3, c.clear + 0.44, c.lhd * 0.4];
      break;
    case SocketSpot.BelowColumn:
      at = [c.cowlX - 0.45, c.clear + 0.45, c.lhd * 0.37];
      break;
    case SocketSpot.FootwellLeft:
      at = [c.cowlX - 0.25, c.clear + 0.3, side(c.cowlX - 0.25, c.clear + 0.3, 0.08)];
      dir = [0, 0, -c.lhd];
      break;
    case SocketSpot.FuseBoxLower:
      at = [c.cowlX - 0.44, c.clear + 0.42, side(c.cowlX - 0.44, c.clear + 0.42, 0.12)];
      dir = [-0.4, -0.3, -c.lhd * 0.87];
      break;
    case SocketSpot.ConsoleFront:
      at = [c.cowlX - 0.55, c.clear + 0.38, 0];
      dir = [-0.6, 0.8, 0];
      break;
    case SocketSpot.ConsoleSide:
      at = [c.cowlX - 0.7, c.floor + 0.13, c.lhd * 0.12];
      dir = [0, 0, c.lhd];
      break;
    case SocketSpot.Passenger:
      at = [c.cowlX - 0.42, dash, -c.lhd * 0.4];
      break;
    case SocketSpot.RearConsole:
      at = [c.frontSeatX - 0.34, c.floor + 0.2, 0];
      dir = [-1, 0, 0];
      break;
    case SocketSpot.EngineBay:
      at = [c.cowlX + 0.2, c.clear + 0.58, c.lhd * 0.42];
      dir = [0, 1, 0];
      break;
    case SocketSpot.UnderSeat:
      at = [c.frontSeatX + 0.2, c.floor + 0.06, c.lhd * 0.37];
      dir = [1, 0, 0];
      break;
    default:
      at = [c.cowlX - 0.42, dash, c.lhd * 0.58];
  }
  // The engine bay is in front of the bulkhead, everything else behind it.
  const clamped = clampInside(hull, at, 0.07);
  if (spot !== SocketSpot.EngineBay) clamped[0] = Math.min(clamped[0], c.cowlX - 0.12);
  return { at: clampInside(hull, clamped, 0.07), dir };
}

function across3(engine: EngineLayout): "x" | "z" {
  return engine === EngineLayout.FrontLongitudinal ? "x" : "z";
}

export function dedupe(points: Vec3[]): Vec3[] {
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
      case "passengerDoor":
        p = [c.frontSeatX + 0.1, c.clear + 0.55, -side(c.frontSeatX + 0.1, c.clear + 0.55, 0.07)];
        break;
      case "underSeat":
        p = [c.frontSeatX, c.floor + 0.06, 0.37];
        break;
      case "passengerSeat":
        p = [c.frontSeatX, c.floor + 0.06, -0.37];
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
      case "roof": {
        // Behind the windscreen header, between the sun visors.
        const x = hull.xAt(hull.profile.roofFront) - 0.15;
        p = [x, hull.top(hull.station(x)) - 0.1, 0];
        break;
      }
      case "windscreen": {
        // The camera behind the mirror, at the top of the windscreen.
        const x = hull.xAt(hull.profile.roofFront) + 0.05;
        p = [x, hull.top(hull.station(x)) - 0.12, 0];
        break;
      }
      case "fuelPump":
        p = [c.rearAxle + 0.45, c.floor + 0.12, -0.3];
        break;
      case "rearAxle":
        p = [c.rearAxle + 0.05, c.clear + 0.38, 0];
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
