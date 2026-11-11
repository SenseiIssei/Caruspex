// The shape of the car's body for the X-ray view: a handful of numbers that
// make a silhouette recognisable (where the windscreen starts and how steep
// it is, how the roof runs out at the back, how high the window line sits,
// how the side windows are split), and the profile the hull is built from.
//
// The numbers come in three steps:
//
// 1. Defaults per body type, adjusted for the era (older cars have steeper
//    windscreens and lower window lines) and the drivetrain (an engine along
//    the car means a long bonnet, an electric car a short one).
// 2. A hand-tuned row of SHAPES for common generations, so a Golf, a BMW Gran
//    Coupe or a Fiat 500 reads as itself. The rows hold only shape, never
//    dimensions (those come from Wikidata or stay typical, see anatomy.rs),
//    and each says where its values come from: the published dimensions
//    (overhangs from length minus wheelbase), the standard tyre size, and
//    well known traits of the body visible on any press photo. They are
//    estimates of proportions, not measurements of a drawing.
// 3. The profile: the roof line from end to end, the stations of the
//    windscreen, roof and rear window, the side window opening with its
//    pillars and the door seams, all in metres for one car.
//
// Coordinates as in layout.ts: x along the car (front is +x, 0 the middle),
// y up from the road, stations u from 0 (rear end) to 1 (front end). No
// logos, badges or brand graphics are drawn, only the body.
import { BodyType, DrivenAxles, EngineLayout, type VehicleAnatomy } from "../api/types";

export type RearStyle =
  | "notchback"
  | "fastback"
  | "hatchback"
  | "estate"
  | "suv"
  | "van"
  | "pickup"
  | "roadster"
  | "convertible";

export interface ShapeSpec {
  style: RearStyle;
  // Side doors (a tailgate does not count).
  doors: 2 | 4;
  // Share of the overhangs (length minus wheelbase) in front of the front axle.
  frontShare: number;
  // Metres from the front axle back to the base of the windscreen; negative
  // when the windscreen starts ahead of the axle (vans).
  cowl: number;
  // Rake of the windscreen and of the rear window (or tailgate glass) in
  // degrees from the horizontal.
  windscreen: number;
  rearWindow: number;
  // Metres the roof bulges above the line between its ends, and where along
  // the roof the highest point is (0 its rear end, 1 its front end).
  roofCrown: number;
  roofPeak: number;
  // Metres the rear end of the roof sits below its front end (a
  // hatchback's roof falling towards the spoiler).
  roofDrop: number;
  // Boot lid (notchback, fastback), rear deck (roadster) or load bed
  // (pickup) in metres.
  deck: number;
  // Share of the height at the base of the side windows at the windscreen,
  // and how many metres the line rises towards the back.
  belt: number;
  beltRise: number;
  // Ground to sill in metres.
  rideHeight: number;
  // Share of the height of the bonnet's front edge and of the boot lid.
  nose: number;
  tail: number;
  // How much narrower the glass house is than the body at the window line.
  tumblehome: number;
  // Roundness of the nose and the tail in plan view (exponent, small = round).
  noseRound: number;
  tailRound: number;
  // Width of the last pillar at the window line in metres (a Golf's broad
  // C-pillar, a thin D-pillar of an estate).
  lastPillar: number;
  // Doors without window frames: the pillars between the windows are thin.
  frameless: boolean;
  // A fixed window behind the last door.
  quarterWindow: boolean;
  // The forward kink at the rear end of the side windows (BMW).
  kink: boolean;
}

// Outer wheel diameter in metres, when a row knows the standard tyre.
export interface ShapeRow {
  brand: string;
  model: string;
  generations: string[];
  // The body this row describes; omitted: the catalog's own. Several rows
  // for one generation are its body variants (saloon, estate, coupe).
  body?: BodyType;
  shape: Partial<ShapeSpec> & { wheel?: number };
  // Where the values come from.
  note: string;
}

export interface Shape extends ShapeSpec {
  body: BodyType;
  // Outer wheel diameter in metres.
  wheel: number;
  source: "typical" | "profile";
  note: string;
}

// --- Defaults ----------------------------------------------------------------------

const BASE: Record<BodyType, ShapeSpec> = {
  [BodyType.Hatchback]: {
    style: "hatchback", doors: 4, frontShare: 0.55, cowl: 0.42, windscreen: 28, rearWindow: 58, roofCrown: 0.035, roofPeak: 0.5, roofDrop: 0.05,
    deck: 0, belt: 0.65, beltRise: 0.04, rideHeight: 0.14, nose: 0.5, tail: 0.66, tumblehome: 0.17, noseRound: 4.5, tailRound: 7,
    lastPillar: 0.4, frameless: false, quarterWindow: false, kink: false,
  },
  [BodyType.Sedan]: {
    style: "notchback", doors: 4, frontShare: 0.47, cowl: 0.5, windscreen: 27, rearWindow: 27, roofCrown: 0.04, roofPeak: 0.55, roofDrop: 0,
    deck: 0.5, belt: 0.66, beltRise: 0.04, rideHeight: 0.14, nose: 0.48, tail: 0.68, tumblehome: 0.18, noseRound: 4.5, tailRound: 7,
    lastPillar: 0.22, frameless: false, quarterWindow: false, kink: false,
  },
  [BodyType.Estate]: {
    style: "estate", doors: 4, frontShare: 0.47, cowl: 0.5, windscreen: 28, rearWindow: 70, roofCrown: 0.025, roofPeak: 0.6, roofDrop: 0.015,
    deck: 0, belt: 0.65, beltRise: 0.04, rideHeight: 0.14, nose: 0.48, tail: 0.66, tumblehome: 0.16, noseRound: 4.5, tailRound: 8,
    lastPillar: 0.12, frameless: false, quarterWindow: true, kink: false,
  },
  [BodyType.Suv]: {
    style: "suv", doors: 4, frontShare: 0.5, cowl: 0.45, windscreen: 30, rearWindow: 64, roofCrown: 0.025, roofPeak: 0.6, roofDrop: 0.02,
    deck: 0, belt: 0.63, beltRise: 0.04, rideHeight: 0.19, nose: 0.53, tail: 0.66, tumblehome: 0.15, noseRound: 5, tailRound: 8,
    lastPillar: 0.16, frameless: false, quarterWindow: true, kink: false,
  },
  [BodyType.Van]: {
    style: "van", doors: 4, frontShare: 0.47, cowl: -0.05, windscreen: 33, rearWindow: 84, roofCrown: 0.02, roofPeak: 0.5, roofDrop: 0,
    deck: 0, belt: 0.56, beltRise: 0, rideHeight: 0.17, nose: 0.5, tail: 0.6, tumblehome: 0.08, noseRound: 5, tailRound: 10,
    lastPillar: 0.1, frameless: false, quarterWindow: true, kink: false,
  },
  [BodyType.Coupe]: {
    style: "fastback", doors: 2, frontShare: 0.47, cowl: 0.55, windscreen: 25, rearWindow: 22, roofCrown: 0.04, roofPeak: 0.55, roofDrop: 0,
    deck: 0.32, belt: 0.67, beltRise: 0.05, rideHeight: 0.13, nose: 0.47, tail: 0.69, tumblehome: 0.2, noseRound: 4.5, tailRound: 7,
    lastPillar: 0.2, frameless: true, quarterWindow: true, kink: false,
  },
  [BodyType.Convertible]: {
    style: "convertible", doors: 2, frontShare: 0.48, cowl: 0.5, windscreen: 27, rearWindow: 30, roofCrown: 0.02, roofPeak: 0.5, roofDrop: 0,
    deck: 0.55, belt: 0.69, beltRise: 0.03, rideHeight: 0.13, nose: 0.48, tail: 0.71, tumblehome: 0.2, noseRound: 4.5, tailRound: 7,
    lastPillar: 0.3, frameless: true, quarterWindow: true, kink: false,
  },
  [BodyType.Pickup]: {
    style: "pickup", doors: 4, frontShare: 0.42, cowl: 0.42, windscreen: 31, rearWindow: 85, roofCrown: 0.02, roofPeak: 0.5, roofDrop: 0,
    deck: 1.55, belt: 0.64, beltRise: 0.02, rideHeight: 0.22, nose: 0.56, tail: 0.62, tumblehome: 0.1, noseRound: 6, tailRound: 12,
    lastPillar: 0.14, frameless: false, quarterWindow: false, kink: false,
  },
  [BodyType.Mpv]: {
    style: "estate", doors: 4, frontShare: 0.53, cowl: 0.22, windscreen: 29, rearWindow: 72, roofCrown: 0.025, roofPeak: 0.55, roofDrop: 0.02,
    deck: 0, belt: 0.6, beltRise: 0.03, rideHeight: 0.15, nose: 0.47, tail: 0.6, tumblehome: 0.12, noseRound: 4.5, tailRound: 8,
    lastPillar: 0.14, frameless: false, quarterWindow: true, kink: false,
  },
  [BodyType.Microcar]: {
    style: "hatchback", doors: 2, frontShare: 0.5, cowl: 0.12, windscreen: 34, rearWindow: 78, roofCrown: 0.03, roofPeak: 0.5, roofDrop: 0.02,
    deck: 0, belt: 0.6, beltRise: 0.02, rideHeight: 0.14, nose: 0.48, tail: 0.6, tumblehome: 0.12, noseRound: 3.5, tailRound: 5,
    lastPillar: 0.25, frameless: false, quarterWindow: false, kink: false,
  },
  [BodyType.Roadster]: {
    style: "roadster", doors: 2, frontShare: 0.47, cowl: 0.6, windscreen: 27, rearWindow: 30, roofCrown: 0, roofPeak: 0.5, roofDrop: 0,
    deck: 0.85, belt: 0.74, beltRise: 0.02, rideHeight: 0.12, nose: 0.5, tail: 0.76, tumblehome: 0.2, noseRound: 4.5, tailRound: 7,
    lastPillar: 0.2, frameless: true, quarterWindow: false, kink: false,
  },
};

// The defaults for a body, year and drivetrain.
export function defaultShape(body: BodyType, year: number, engine: EngineLayout, driven: DrivenAxles): ShapeSpec {
  const s: ShapeSpec = { ...(BASE[body] ?? BASE[BodyType.Hatchback]) };
  // Older cars: steeper windscreens, lower window lines, flatter roofs,
  // longer boots; newer ones the other way round.
  if (year > 0 && year < 1990) {
    s.windscreen += 7;
    s.belt -= 0.07;
    s.roofCrown *= 0.5;
    s.deck *= 1.15;
    s.tumblehome *= 0.7;
    s.rearWindow = Math.min(88, s.rearWindow + 4);
  } else if (year > 0 && year < 2002) {
    s.windscreen += 3;
    s.belt -= 0.035;
    s.tumblehome *= 0.85;
  } else if (year >= 2015) {
    s.windscreen -= 1;
    s.belt += 0.01;
  }
  // The engine decides the bonnet: along the car a long one with the front
  // axle far forward, at the back or none a short and low one.
  switch (engine) {
    case EngineLayout.FrontLongitudinal:
      s.frontShare = Math.min(s.frontShare, driven === DrivenAxles.Front ? 0.5 : 0.43);
      s.cowl = Math.max(s.cowl, driven === DrivenAxles.Front ? 0.55 : 0.75);
      break;
    case EngineLayout.Mid:
    case EngineLayout.Rear:
      s.frontShare = 0.44;
      s.cowl = Math.min(s.cowl, 0.3);
      s.nose -= 0.04;
      break;
    case EngineLayout.None:
      s.frontShare = Math.min(s.frontShare, 0.48);
      s.cowl = Math.min(s.cowl, 0.3);
      s.nose -= 0.03;
      s.windscreen -= 2;
      break;
    default:
      break;
  }
  return s;
}

// --- Hand-tuned rows ------------------------------------------------------------------

// Overhang shares are front / (length - wheelbase) from the published
// dimensions of the generation; wheels the outer diameter of the standard
// tyre (225/45 R18 = 0.66 m); angles and the rest read off side views. The
// note names the traits the row is about.
const B = BodyType;
export const SHAPES: ShapeRow[] = [
  // --- BMW: engine along the car, long bonnet, short front overhang, the kink
  // at the rear of the side windows.
  {
    brand: "BMW", model: "4er Gran Coupé", generations: ["F36"],
    shape: { style: "fastback", doors: 4, frameless: true, quarterWindow: true, kink: true, frontShare: 0.43, cowl: 0.78, windscreen: 25, rearWindow: 18, deck: 0.2, roofCrown: 0.045, roofPeak: 0.62, belt: 0.67, beltRise: 0.05, nose: 0.46, tail: 0.69, tumblehome: 0.21, lastPillar: 0.16, rideHeight: 0.13, wheel: 0.66 },
    note: "Four-door coupe with a tailgate: long sloping rear window down to a short boot lid, frameless door windows, small window behind the rear door, BMW kink. Overhangs 790/1038 mm (4638 mm long, 2810 mm wheelbase), 225/45 R18.",
  },
  {
    brand: "BMW", model: "4er", generations: ["F32"],
    shape: { style: "fastback", doors: 2, frameless: true, quarterWindow: true, kink: true, frontShare: 0.43, cowl: 0.8, windscreen: 25, rearWindow: 21, deck: 0.38, roofCrown: 0.04, belt: 0.68, beltRise: 0.05, nose: 0.46, tail: 0.7, tumblehome: 0.21, wheel: 0.66 },
    note: "Coupe: long doors without frames, short glass house, BMW kink. Overhangs 790/1038 mm, 225/45 R18.",
  },
  {
    brand: "BMW", model: "3er", generations: ["F30"],
    shape: { style: "notchback", kink: true, frontShare: 0.41, cowl: 0.76, windscreen: 26, rearWindow: 26, deck: 0.5, belt: 0.66, beltRise: 0.04, nose: 0.47, tail: 0.69, wheel: 0.65 },
    note: "Saloon with a classic three-box profile, long bonnet, BMW kink. Overhangs 745/1069 mm (4624, 2810), 225/50 R17.",
  },
  {
    brand: "BMW", model: "3er", generations: ["F30"], body: B.Estate,
    shape: { style: "estate", kink: true, frontShare: 0.41, cowl: 0.76, windscreen: 26, rearWindow: 66, lastPillar: 0.14, wheel: 0.65 },
    note: "Touring (F31): the estate of the F30, sloping D-pillar.",
  },
  {
    brand: "BMW", model: "3er", generations: ["G20"],
    shape: { style: "notchback", kink: true, frontShare: 0.43, cowl: 0.76, windscreen: 25, rearWindow: 25, deck: 0.48, belt: 0.67, beltRise: 0.05, nose: 0.46, tail: 0.7, wheel: 0.67 },
    note: "Saloon, a little longer at the front than the F30, BMW kink. Overhangs 798/1060 mm (4709, 2851), 225/45 R18.",
  },
  {
    brand: "BMW", model: "3er", generations: ["G20"], body: B.Estate,
    shape: { style: "estate", kink: true, frontShare: 0.43, cowl: 0.76, windscreen: 25, rearWindow: 64, lastPillar: 0.14, wheel: 0.67 },
    note: "Touring (G21).",
  },
  {
    brand: "BMW", model: "1er", generations: ["F20"],
    shape: { style: "hatchback", kink: true, frontShare: 0.44, cowl: 0.7, windscreen: 27, rearWindow: 58, belt: 0.66, beltRise: 0.05, nose: 0.47, lastPillar: 0.3, wheel: 0.63 },
    note: "The rear-wheel drive hatchback: long bonnet for its size, short front overhang, BMW kink. Overhangs 724/910 mm (4324, 2690), 205/55 R16.",
  },
  {
    brand: "BMW", model: "5er", generations: ["F10"],
    shape: { style: "notchback", kink: true, frontShare: 0.43, cowl: 0.82, windscreen: 26, rearWindow: 25, deck: 0.55, belt: 0.67, beltRise: 0.04, nose: 0.48, tail: 0.69, wheel: 0.68 },
    note: "Large saloon with a long bonnet, BMW kink. Overhangs 853/1133 mm (4899, 2968), 225/55 R17.",
  },
  {
    brand: "BMW", model: "5er", generations: ["F10"], body: B.Estate,
    shape: { style: "estate", kink: true, frontShare: 0.43, cowl: 0.82, windscreen: 26, rearWindow: 64, lastPillar: 0.14, wheel: 0.68 },
    note: "Touring (F11).",
  },
  {
    brand: "BMW", model: "5er", generations: ["G30"],
    shape: { style: "notchback", kink: true, frontShare: 0.45, cowl: 0.82, windscreen: 25, rearWindow: 24, deck: 0.52, belt: 0.68, beltRise: 0.04, nose: 0.47, tail: 0.7, wheel: 0.69 },
    note: "Saloon, rear window a little flatter than the F10, BMW kink. Overhangs 880/1093 mm (4936, 2975), 225/55 R17.",
  },
  {
    brand: "BMW", model: "5er", generations: ["G30"], body: B.Estate,
    shape: { style: "estate", kink: true, frontShare: 0.45, cowl: 0.82, windscreen: 25, rearWindow: 63, lastPillar: 0.14, wheel: 0.69 },
    note: "Touring (G31).",
  },
  // --- Volkswagen and the group's compact cars: engine across, broad C-pillar.
  {
    brand: "Volkswagen", model: "Golf", generations: ["IV"],
    shape: { roofDrop: 0.05, frontShare: 0.53, cowl: 0.4, windscreen: 31, rearWindow: 62, roofCrown: 0.03, belt: 0.6, beltRise: 0.03, nose: 0.47, lastPillar: 0.5, wheel: 0.62 },
    note: "Upright hatchback with the broad Golf C-pillar and a low window line. 195/65 R15.",
  },
  {
    brand: "Volkswagen", model: "Golf", generations: ["V", "VI"],
    shape: { roofDrop: 0.05, frontShare: 0.54, cowl: 0.42, windscreen: 29, rearWindow: 60, belt: 0.63, beltRise: 0.035, nose: 0.49, lastPillar: 0.5, wheel: 0.63 },
    note: "Hatchback, taller and rounder than the IV, broad C-pillar. 195/65 R15.",
  },
  {
    brand: "Volkswagen", model: "Golf", generations: ["VII"],
    shape: { roofDrop: 0.06, frontShare: 0.53, cowl: 0.46, windscreen: 27, rearWindow: 60, roofCrown: 0.03, belt: 0.66, beltRise: 0.04, nose: 0.5, lastPillar: 0.48, wheel: 0.63 },
    note: "Hatchback with a long low bonnet line, the windscreen moved back, broad C-pillar. Overhangs 863/755 mm (4255, 2637), 205/55 R16.",
  },
  {
    brand: "Volkswagen", model: "Golf", generations: ["VIII"],
    shape: { roofDrop: 0.06, frontShare: 0.55, cowl: 0.46, windscreen: 26, rearWindow: 58, roofCrown: 0.03, belt: 0.67, beltRise: 0.04, nose: 0.49, lastPillar: 0.46, wheel: 0.64 },
    note: "Hatchback, flatter windscreen than the VII, broad C-pillar. 205/55 R16.",
  },
  {
    brand: "Volkswagen", model: "Polo", generations: ["V", "VI"],
    shape: { frontShare: 0.54, cowl: 0.4, windscreen: 28, rearWindow: 62, belt: 0.65, nose: 0.5, lastPillar: 0.42, wheel: 0.6 },
    note: "Small hatchback in the Golf's style, broad C-pillar. 185/65 R15.",
  },
  {
    brand: "Volkswagen", model: "Passat", generations: ["B6", "B7"],
    shape: { style: "notchback", frontShare: 0.48, cowl: 0.45, windscreen: 27, rearWindow: 27, deck: 0.5, belt: 0.66, nose: 0.48, wheel: 0.65 },
    note: "Saloon with the engine across, long front overhang. 215/55 R16.",
  },
  {
    brand: "Volkswagen", model: "Passat", generations: ["B6", "B7"], body: B.Estate,
    shape: { style: "estate", frontShare: 0.48, cowl: 0.45, windscreen: 27, rearWindow: 70, lastPillar: 0.12, wheel: 0.65 },
    note: "Variant: estate with an upright tailgate.",
  },
  {
    brand: "Volkswagen", model: "Passat", generations: ["B8"],
    shape: { style: "estate", frontShare: 0.47, cowl: 0.5, windscreen: 26, rearWindow: 68, belt: 0.67, beltRise: 0.03, lastPillar: 0.12, wheel: 0.66 },
    note: "Variant, the body most B8 have: long flat roof, upright tailgate. 215/55 R17.",
  },
  {
    brand: "Volkswagen", model: "Passat", generations: ["B8"], body: B.Sedan,
    shape: { style: "notchback", frontShare: 0.47, cowl: 0.5, windscreen: 26, rearWindow: 26, deck: 0.5, wheel: 0.66 },
    note: "Saloon of the B8.",
  },
  {
    brand: "Volkswagen", model: "Tiguan", generations: ["I"],
    shape: { windscreen: 31, rearWindow: 66, belt: 0.62, rideHeight: 0.19, wheel: 0.7 },
    note: "Compact SUV with an upright tailgate. 235/55 R17.",
  },
  {
    brand: "Volkswagen", model: "Tiguan", generations: ["II"],
    shape: { windscreen: 28, rearWindow: 62, belt: 0.64, beltRise: 0.04, rideHeight: 0.19, wheel: 0.71 },
    note: "Compact SUV, longer and lower than the first. 215/65 R17.",
  },
  {
    brand: "Audi", model: "A2", generations: ["8Z"],
    shape: { style: "hatchback", frontShare: 0.53, cowl: 0.08, windscreen: 30, rearWindow: 52, roofCrown: 0.09, roofPeak: 0.62, belt: 0.56, beltRise: 0.03, nose: 0.46, tumblehome: 0.22, noseRound: 3.2, tailRound: 5, lastPillar: 0.32, wheel: 0.59 },
    note: "Aluminium one-box: the windscreen starts far forward over a short bonnet, the roof is one high arch falling to a short tail with a spoiler, low window line. Overhangs 755/666 mm (3826, 2405), 175/60 R15.",
  },
  {
    brand: "Audi", model: "A3", generations: ["8P"],
    shape: { roofDrop: 0.06, frontShare: 0.55, cowl: 0.45, windscreen: 28, rearWindow: 54, belt: 0.64, nose: 0.49, lastPillar: 0.36, wheel: 0.63 },
    note: "Compact hatchback, more sloping tailgate than the Golf it shares parts with. 205/55 R16.",
  },
  {
    brand: "Audi", model: "A3", generations: ["8V"],
    shape: { roofDrop: 0.06, frontShare: 0.55, cowl: 0.48, windscreen: 27, rearWindow: 54, belt: 0.66, nose: 0.48, lastPillar: 0.34, wheel: 0.64 },
    note: "Hatchback (Sportback), sharp shoulder line. 205/55 R16.",
  },
  {
    brand: "Audi", model: "A4", generations: ["B7", "B8", "B9"],
    shape: { style: "notchback", frontShare: 0.5, cowl: 0.58, windscreen: 26, rearWindow: 26, deck: 0.5, belt: 0.66, nose: 0.47, wheel: 0.66 },
    note: "Saloon with the engine along the car ahead of the front axle: long front overhang, short rear one. Overhangs 948/945 mm (B8: 4701, 2808), 225/50 R17.",
  },
  {
    brand: "Audi", model: "A4", generations: ["B7", "B8", "B9"], body: B.Estate,
    shape: { style: "estate", frontShare: 0.5, cowl: 0.58, windscreen: 26, rearWindow: 62, lastPillar: 0.13, wheel: 0.66 },
    note: "Avant: estate with a sloping D-pillar.",
  },
  {
    brand: "Audi", model: "A6", generations: ["C7", "C8"],
    shape: { style: "notchback", frontShare: 0.48, cowl: 0.62, windscreen: 25, rearWindow: 25, deck: 0.52, belt: 0.67, nose: 0.47, wheel: 0.69 },
    note: "Large saloon, engine along the car ahead of the front axle, coupe-like roof. 225/55 R18.",
  },
  {
    brand: "Audi", model: "A6", generations: ["C7", "C8"], body: B.Estate,
    shape: { style: "estate", frontShare: 0.48, cowl: 0.62, windscreen: 25, rearWindow: 58, lastPillar: 0.14, wheel: 0.69 },
    note: "Avant: estate with a strongly sloping tailgate.",
  },
  // --- Mercedes-Benz: engine along the car, cabin set back.
  {
    brand: "Mercedes-Benz", model: "C-Klasse", generations: ["W204"],
    shape: { style: "notchback", frontShare: 0.43, cowl: 0.78, windscreen: 27, rearWindow: 27, deck: 0.5, belt: 0.65, nose: 0.48, wheel: 0.65 },
    note: "Saloon with a long bonnet and short front overhang. 205/55 R16.",
  },
  {
    brand: "Mercedes-Benz", model: "C-Klasse", generations: ["W205"],
    shape: { style: "notchback", frontShare: 0.43, cowl: 0.8, windscreen: 25, rearWindow: 25, deck: 0.47, roofCrown: 0.05, belt: 0.66, beltRise: 0.03, nose: 0.47, wheel: 0.66 },
    note: "Saloon, rounder roof and flatter windscreen than the W204. Overhangs 789/1057 mm (4686, 2840), 205/60 R16.",
  },
  {
    brand: "Mercedes-Benz", model: "C-Klasse", generations: ["W204", "W205"], body: B.Estate,
    shape: { style: "estate", frontShare: 0.43, cowl: 0.8, windscreen: 26, rearWindow: 62, lastPillar: 0.14, wheel: 0.66 },
    note: "T-Modell: estate (S204, S205).",
  },
  {
    brand: "Mercedes-Benz", model: "E-Klasse", generations: ["W212"],
    shape: { style: "notchback", frontShare: 0.44, cowl: 0.82, windscreen: 26, rearWindow: 26, deck: 0.55, belt: 0.66, nose: 0.48, wheel: 0.68 },
    note: "Large saloon, long bonnet. 225/55 R16.",
  },
  {
    brand: "Mercedes-Benz", model: "E-Klasse", generations: ["W213"],
    shape: { style: "notchback", frontShare: 0.43, cowl: 0.85, windscreen: 25, rearWindow: 24, deck: 0.52, roofCrown: 0.05, belt: 0.67, beltRise: 0.03, nose: 0.47, wheel: 0.69 },
    note: "Large saloon, long bonnet, flowing roof. Overhangs 850/1134 mm (4923, 2939), 225/55 R17.",
  },
  {
    brand: "Mercedes-Benz", model: "E-Klasse", generations: ["W212", "W213"], body: B.Estate,
    shape: { style: "estate", frontShare: 0.43, cowl: 0.85, windscreen: 25, rearWindow: 66, roofCrown: 0.03, lastPillar: 0.12, belt: 0.66, wheel: 0.69 },
    note: "T-Modell: estate (S212, S213) with a long flat roof and a sloping tailgate.",
  },
  {
    brand: "Mercedes-Benz", model: "E-Klasse", generations: ["W212", "W213"], body: B.Coupe,
    shape: { style: "fastback", doors: 2, frameless: true, quarterWindow: false, frontShare: 0.43, cowl: 0.85, windscreen: 24, rearWindow: 20, deck: 0.4, belt: 0.68, wheel: 0.69 },
    note: "Coupe (C207, C238): no B-pillar, one sweep of glass.",
  },
  {
    brand: "Mercedes-Benz", model: "E-Klasse", generations: ["W212", "W213"], body: B.Convertible,
    shape: { style: "convertible", doors: 2, frontShare: 0.43, cowl: 0.85, windscreen: 26, wheel: 0.69 },
    note: "Cabriolet (A207, A238) with the soft top closed.",
  },
  {
    brand: "Mercedes-Benz", model: "A-Klasse", generations: ["W176", "W177"],
    shape: { roofDrop: 0.08, frontShare: 0.55, cowl: 0.45, windscreen: 26, rearWindow: 50, roofCrown: 0.04, belt: 0.66, beltRise: 0.07, nose: 0.47, lastPillar: 0.35, wheel: 0.65 },
    note: "Low hatchback with a strongly rising window line and a sloping tailgate. 205/55 R16.",
  },
  // --- Other compact and small cars.
  {
    brand: "Opel", model: "Corsa", generations: ["D", "E"],
    shape: { windscreen: 29, rearWindow: 60, belt: 0.64, beltRise: 0.05, lastPillar: 0.38, wheel: 0.6 },
    note: "Small hatchback with a rising window line. 185/65 R15.",
  },
  {
    brand: "Opel", model: "Corsa", generations: ["F"],
    shape: { windscreen: 27, rearWindow: 58, belt: 0.65, beltRise: 0.04, lastPillar: 0.36, wheel: 0.61 },
    note: "Small hatchback, lower than the E. 195/55 R16.",
  },
  {
    brand: "Opel", model: "Astra", generations: ["H", "J", "K"],
    shape: { windscreen: 27, rearWindow: 56, belt: 0.65, beltRise: 0.05, lastPillar: 0.38, wheel: 0.63 },
    note: "Compact hatchback. 205/55 R16.",
  },
  {
    brand: "Ford", model: "Focus", generations: ["Mk2", "Mk3", "Mk4"],
    shape: { windscreen: 27, rearWindow: 57, belt: 0.65, beltRise: 0.05, lastPillar: 0.36, wheel: 0.63 },
    note: "Compact hatchback with a long rising window line. 205/55 R16.",
  },
  {
    brand: "Ford", model: "Fiesta", generations: ["Mk6", "Mk7"],
    shape: { windscreen: 27, rearWindow: 58, belt: 0.64, beltRise: 0.06, lastPillar: 0.38, wheel: 0.6 },
    note: "Small hatchback with a strongly rising window line. 195/60 R15.",
  },
  {
    brand: "Toyota", model: "Corolla", generations: ["E210"],
    shape: { windscreen: 26, rearWindow: 50, belt: 0.66, beltRise: 0.04, lastPillar: 0.36, wheel: 0.64 },
    note: "Hatchback with a sloping tailgate. 205/55 R16.",
  },
  {
    brand: "Toyota", model: "Corolla", generations: ["E210"], body: B.Estate,
    shape: { style: "estate", windscreen: 26, rearWindow: 60, lastPillar: 0.14, wheel: 0.64 },
    note: "Touring Sports: the estate.",
  },
  {
    brand: "Toyota", model: "Yaris", generations: ["XP130", "XP210"],
    shape: { windscreen: 28, rearWindow: 60, belt: 0.64, beltRise: 0.04, lastPillar: 0.36, wheel: 0.6 },
    note: "Small hatchback. 185/60 R15.",
  },
  {
    brand: "Toyota", model: "Prius", generations: ["XW30", "XW50"],
    shape: { style: "fastback", frontShare: 0.51, cowl: 0.18, windscreen: 23, rearWindow: 22, deck: 0.06, roofCrown: 0.06, roofPeak: 0.68, belt: 0.64, beltRise: 0.05, nose: 0.43, tail: 0.72, lastPillar: 0.25, wheel: 0.63 },
    note: "The hybrid wedge: very flat windscreen starting far forward, one roof arch falling to a cut-off tail (Kamm tail) with a split rear window. 195/65 R15.",
  },
  {
    brand: "Toyota", model: "Prius", generations: ["XW60"],
    shape: { style: "fastback", frontShare: 0.5, cowl: 0.2, windscreen: 21, rearWindow: 20, deck: 0.08, roofCrown: 0.05, roofPeak: 0.6, belt: 0.66, beltRise: 0.06, nose: 0.42, tail: 0.73, lastPillar: 0.22, wheel: 0.67 },
    note: "Low coupe-like hybrid with an extremely flat windscreen. 195/50 R19.",
  },
  {
    brand: "Škoda", model: "Octavia", generations: ["II", "III", "IV"],
    shape: { style: "notchback", windscreen: 27, rearWindow: 28, deck: 0.38, belt: 0.65, beltRise: 0.03, tail: 0.7, wheel: 0.63 },
    note: "Liftback: looks like a saloon, the boot lid lifts with the rear window. 205/55 R16.",
  },
  {
    brand: "Škoda", model: "Octavia", generations: ["II", "III", "IV"], body: B.Estate,
    shape: { style: "estate", windscreen: 27, rearWindow: 70, lastPillar: 0.12, wheel: 0.63 },
    note: "Combi: the estate.",
  },
  {
    brand: "Škoda", model: "Fabia", generations: ["II"],
    shape: { windscreen: 31, rearWindow: 68, roofCrown: 0.02, belt: 0.62, lastPillar: 0.3, wheel: 0.6 },
    note: "Small hatchback with a tall, square roof. 185/60 R15.",
  },
  {
    brand: "Škoda", model: "Fabia", generations: ["III"],
    shape: { windscreen: 28, rearWindow: 62, belt: 0.64, lastPillar: 0.34, wheel: 0.6 },
    note: "Small hatchback, lower and wider than the II. 185/60 R15.",
  },
  {
    brand: "SEAT", model: "Leon", generations: ["II"],
    shape: { roofDrop: 0.08, windscreen: 26, rearWindow: 48, roofCrown: 0.05, belt: 0.65, beltRise: 0.05, lastPillar: 0.28, wheel: 0.63 },
    note: "Hatchback shaped like a coupe: rear door handles hidden in the window frame, roof falling far back. 205/55 R16.",
  },
  {
    brand: "SEAT", model: "Leon", generations: ["III"],
    shape: { windscreen: 27, rearWindow: 56, belt: 0.66, beltRise: 0.04, lastPillar: 0.34, wheel: 0.63 },
    note: "Compact hatchback with sharp lines. 205/55 R16.",
  },
  {
    brand: "Renault", model: "Clio", generations: ["IV", "V"],
    shape: { roofDrop: 0.07, windscreen: 27, rearWindow: 52, roofCrown: 0.045, belt: 0.65, beltRise: 0.05, lastPillar: 0.32, wheel: 0.61 },
    note: "Small hatchback with a coupe-like roof, rear door handles in the window frame. 195/55 R16.",
  },
  {
    brand: "Renault", model: "Zoe", generations: ["I"],
    shape: { windscreen: 26, rearWindow: 52, roofCrown: 0.05, belt: 0.64, beltRise: 0.04, nose: 0.45, lastPillar: 0.32, wheel: 0.62 },
    note: "Small electric hatchback, rounded with a short bonnet. 195/55 R16.",
  },
  {
    brand: "Peugeot", model: "208", generations: ["I", "II"],
    shape: { windscreen: 27, rearWindow: 58, belt: 0.65, beltRise: 0.04, lastPillar: 0.36, wheel: 0.6 },
    note: "Small hatchback. 185/65 R15.",
  },
  {
    brand: "Peugeot", model: "308", generations: ["I", "II"],
    shape: { windscreen: 26, rearWindow: 56, belt: 0.65, beltRise: 0.04, lastPillar: 0.36, wheel: 0.63 },
    note: "Compact hatchback with a long, flat windscreen. 205/55 R16.",
  },
  {
    brand: "Hyundai", model: "i30", generations: ["GD", "PD"],
    shape: { windscreen: 27, rearWindow: 56, belt: 0.65, beltRise: 0.04, lastPillar: 0.36, wheel: 0.63 },
    note: "Compact hatchback. 205/55 R16.",
  },
  {
    brand: "Hyundai", model: "Kona", generations: ["OS"],
    shape: { windscreen: 28, rearWindow: 58, belt: 0.62, rideHeight: 0.17, lastPillar: 0.2, wheel: 0.66 },
    note: "Small SUV, low for its class, with a sloping tailgate. 215/55 R17.",
  },
  {
    brand: "Kia", model: "Ceed", generations: ["CD"],
    shape: { windscreen: 27, rearWindow: 56, belt: 0.65, beltRise: 0.04, lastPillar: 0.36, wheel: 0.63 },
    note: "Compact hatchback. 205/55 R16.",
  },
  // --- Electric cars.
  {
    brand: "Tesla", model: "Model 3", generations: ["I"],
    shape: { style: "fastback", frontShare: 0.46, cowl: 0.25, windscreen: 22, rearWindow: 20, deck: 0.28, roofCrown: 0.06, roofPeak: 0.55, belt: 0.66, beltRise: 0.04, nose: 0.42, tail: 0.7, frameless: true, quarterWindow: true, lastPillar: 0.14, wheel: 0.67 },
    note: "Saloon with a short boot lid behind one long sweep of glass from the windscreen over the roof, short low bonnet, frameless door windows. Overhangs 838/981 mm (4694, 2875), 235/45 R18.",
  },
  {
    brand: "Tesla", model: "Model Y", generations: ["I"],
    shape: { roofDrop: 0.08, style: "suv", frontShare: 0.46, cowl: 0.25, windscreen: 23, rearWindow: 30, roofCrown: 0.05, roofPeak: 0.6, belt: 0.64, beltRise: 0.04, nose: 0.43, frameless: true, quarterWindow: true, lastPillar: 0.18, rideHeight: 0.17, wheel: 0.7 },
    note: "Crossover with the Model 3's front and a sloping fastback tailgate. 255/45 R19.",
  },
  {
    brand: "Nissan", model: "Leaf", generations: ["ZE0"],
    shape: { windscreen: 25, rearWindow: 55, roofCrown: 0.06, belt: 0.62, nose: 0.47, lastPillar: 0.34, wheel: 0.63 },
    note: "Tall electric hatchback with a high roof arch and a raked windscreen. 205/55 R16.",
  },
  {
    brand: "Nissan", model: "Leaf", generations: ["ZE1"],
    shape: { windscreen: 26, rearWindow: 54, roofCrown: 0.045, belt: 0.64, beltRise: 0.04, nose: 0.46, lastPillar: 0.32, wheel: 0.64 },
    note: "Electric hatchback, lower and sharper than the first. 205/55 R16.",
  },
  {
    brand: "Dacia", model: "Sandero", generations: ["II", "III"],
    shape: { windscreen: 29, rearWindow: 62, belt: 0.63, beltRise: 0.03, lastPillar: 0.34, wheel: 0.61 },
    note: "Small hatchback with plain upright lines. 185/65 R15.",
  },
  {
    brand: "Dacia", model: "Duster", generations: ["I", "II"],
    shape: { windscreen: 33, rearWindow: 70, roofCrown: 0.02, belt: 0.6, rideHeight: 0.21, lastPillar: 0.18, wheel: 0.69 },
    note: "Boxy small SUV with an upright windscreen and tailgate. 215/65 R16.",
  },
  {
    brand: "Fiat", model: "500", generations: ["I"],
    shape: { roofDrop: 0.04, style: "hatchback", doors: 2, frontShare: 0.53, cowl: 0.18, windscreen: 33, rearWindow: 72, roofCrown: 0.06, roofPeak: 0.45, belt: 0.6, beltRise: 0.02, nose: 0.5, tumblehome: 0.18, noseRound: 2.6, tailRound: 3.5, lastPillar: 0.3, wheel: 0.58 },
    note: "Three-door city car after the 1957 original: round nose, short bonnet, rounded roof, upright rear. Overhangs 660/586 mm (3546, 2300), 185/55 R15.",
  },
  {
    brand: "Mini", model: "Mini", generations: ["R50", "R56", "F56"],
    shape: { roofDrop: 0.0, style: "hatchback", doors: 2, frontShare: 0.53, cowl: 0.32, windscreen: 34, rearWindow: 76, roofCrown: 0.008, roofPeak: 0.5, belt: 0.66, beltRise: 0, nose: 0.53, tumblehome: 0.1, noseRound: 3.4, tailRound: 5, lastPillar: 0.18, wheel: 0.6 },
    note: "Three-door: upright windscreen, flat 'floating' roof over a band of glass, high window line, short overhangs, wheels at the corners. 175/65 R15 to 195/55 R16.",
  },
];

// --- Choosing ------------------------------------------------------------------------

function rowsFor(brand: string, model: string, generation: string): ShapeRow[] {
  if (!brand || !generation) return [];
  return SHAPES.filter((r) => r.brand === brand && r.model === model && r.generations.includes(generation));
}

// The bodies a generation is drawn in: the catalog's own first, then the
// variants the table knows.
export function bodyVariants(anatomy: Pick<VehicleAnatomy, "brand" | "model" | "generation">, catalogBody: BodyType): BodyType[] {
  const out = [catalogBody];
  for (const r of rowsFor(anatomy.brand, anatomy.model, anatomy.generation)) {
    const body = r.body ?? catalogBody;
    if (!out.includes(body)) out.push(body);
  }
  return out;
}

// The shape of a car: the defaults for its body, year and drivetrain, with
// the hand-tuned row of its generation and body on top.
export function shapeFor(anatomy: VehicleAnatomy, catalogBody: BodyType, body: BodyType = catalogBody): Shape {
  const rows = rowsFor(anatomy.brand, anatomy.model, anatomy.generation);
  const row = rows.find((r) => (r.body ?? catalogBody) === body);
  const base = defaultShape(body, anatomy.year, anatomy.engine, anatomy.driven);
  const { wheel, ...spec } = row?.shape ?? {};
  return {
    ...base,
    ...spec,
    body,
    wheel: wheel ?? anatomy.wheelMm / 1000,
    source: row ? "profile" : "typical",
    note: row?.note ?? "",
  };
}

// --- The profile ----------------------------------------------------------------------

export interface Seam {
  x: number;
  top: number;
  bottom: number;
}

export interface Profile {
  // Roof line: (station from the rear 0 to the front 1, share of the height).
  top: [number, number][];
  // Shares of the height of the window line at the windscreen and at the
  // rear end of the side windows.
  belt: number;
  beltRear: number;
  // Stations of the base of the windscreen, the front and rear end of the
  // roof and the base of the rear window (or the top of the bed).
  cowl: number;
  roofFront: number;
  roofRear: number;
  tail: number;
  // The side window opening: stations of its front edge at the window line
  // and at the roof, of its rear edge at the window line and at the roof.
  dlo: [number, number, number, number];
  // Pillars and dividers inside the opening (stations) and their widths.
  pillars: number[];
  pillarWidths: number[];
  // Depth of the kink at the rear lower corner, as a station difference.
  kink: number;
  // Vertical door seams below the window line (metres).
  seams: Seam[];
  clearance: number;
  rearSeats: boolean;
  tumblehome: number;
  noseRound: number;
  tailRound: number;
  // Stations where the underbody starts to rise towards the ends.
  rearLift: number;
  frontLift: number;
}

export interface BodyGeometry {
  profile: Profile;
  frontAxle: number;
  rearAxle: number;
  wheelRadius: number;
}

const rad = (deg: number) => (deg * Math.PI) / 180;
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

// The profile of one car in metres: length, height, wheelbase and the shape.
export function bodyGeometry(length: number, height: number, wheelbase: number, wheelRadius: number, s: Shape): BodyGeometry {
  const L = length;
  const H = height;
  const xF = L / 2;
  const xR = -L / 2;
  const station = (x: number) => (x - xR) / L;
  const wb = Math.min(wheelbase, L * 0.8);
  const r = Math.min(wheelRadius, (H - 0.2) / 2.6);
  // Overhangs: each at least a wheel radius and a bit.
  const over = L - wb;
  const minOver = r + 0.08;
  const fo = clamp(over * s.frontShare, Math.min(minOver, over / 2), Math.max(over / 2, over - minOver));
  const frontAxle = xF - fo;
  const rearAxle = frontAxle - wb;

  const belt = clamp(s.belt, 0.45, 0.82) * H;
  const beltRear = Math.min(belt + s.beltRise, H - 0.25);
  const crown = s.roofCrown;
  // The ends of the roof: the front one a crown below the top, the rear one
  // lower by the drop (a hatchback's roof falls towards its spoiler).
  const roofY = H - crown;
  const roofRearY = roofY - clamp(s.roofDrop, 0, 0.2);
  const noseY = clamp(s.nose * H, s.rideHeight + 0.3, belt - 0.05);
  const tan = (deg: number) => Math.tan(rad(clamp(deg, 8, 89)));

  // The base of the windscreen: behind the front axle by the bonnet length,
  // never past the middle of the wheelbase or into the nose.
  const cowlX = clamp(frontAxle - s.cowl, rearAxle + wb * 0.55, xF - 0.3);
  const cowlY = belt + 0.02;
  // A roadster's height is the top of its windscreen frame.
  const roofTop = s.style === "roadster" ? H : roofY;
  let roofFrontX = cowlX - (roofTop - cowlY) / tan(s.windscreen);

  // The rear: where the roof ends and what follows.
  let roofRearX: number;
  let tailX: number;
  const rear: [number, number][] = [];
  const glassBottom = beltRear + 0.03;
  const tailY = clamp(s.tail * H, belt - 0.04, H - 0.2);
  const bumperY = Math.max(s.rideHeight + 0.35, belt - 0.45);
  switch (s.style) {
    case "notchback":
    case "fastback":
    case "convertible": {
      const deck = clamp(s.deck, 0.04, L * 0.2);
      tailX = xR + 0.06 + deck;
      roofRearX = tailX + (roofRearY - tailY) / tan(s.rearWindow);
      rear.push([xR, bumperY], [xR + 0.025, tailY - 0.05], [xR + 0.07, tailY - 0.005], [tailX, tailY + 0.01]);
      break;
    }
    case "pickup": {
      const bedY = beltRear - 0.02;
      tailX = xR + 0.05 + clamp(s.deck, 0.8, L * 0.4);
      roofRearX = tailX + 0.1;
      rear.push([xR, bedY - 0.12], [xR + 0.03, bedY], [tailX - 0.06, bedY], [tailX, roofY - 0.2]);
      break;
    }
    case "roadster": {
      const deck = clamp(s.deck, 0.3, L * 0.3);
      tailX = xR + 0.06 + deck;
      roofRearX = roofFrontX - 0.08;
      rear.push([xR, bumperY], [xR + 0.03, tailY - 0.04], [xR + 0.08, tailY], [tailX, tailY + 0.02]);
      break;
    }
    default: {
      // Hatchback, estate, SUV, van: a tailgate whose glass starts at the
      // window line.
      const set = s.style === "hatchback" ? 0.15 : s.style === "van" ? 0.04 : 0.09;
      tailX = xR + set;
      roofRearX = tailX + (roofRearY - glassBottom) / tan(s.rearWindow);
      rear.push([xR, bumperY], [xR + 0.025, glassBottom - 0.16], [tailX, glassBottom]);
      break;
    }
  }
  // The roof needs some length: a too flat rear window or windscreen for
  // this car gets steeper.
  const minRoof = s.style === "roadster" ? 0.08 : Math.min(0.9, L * 0.18);
  if (roofFrontX - roofRearX < minRoof) {
    const mid = (roofFrontX + roofRearX) / 2;
    roofFrontX = Math.min(cowlX - 0.15, mid + minRoof / 2);
    roofRearX = Math.max(tailX + 0.05, roofFrontX - minRoof);
  }

  // The roof line from the rear to the front end.
  const pts: [number, number][] = [...rear];
  if (s.style === "roadster") {
    // The open cockpit behind the windscreen frame, at the height of the doors.
    const rim = beltRear + 0.06;
    pts.push([tailX + 0.25, rim], [roofRearX - 0.12, rim], [roofRearX, roofTop - 0.06], [roofFrontX, roofTop]);
  } else {
    const n = 8;
    const p = clamp(s.roofPeak, 0.2, 0.8);
    for (let i = 0; i <= n; i++) {
      const t = i / n;
      const k = t < p ? (t - p) / p : (t - p) / (1 - p);
      const end = t < p ? roofRearY : roofY;
      pts.push([roofRearX + (roofFrontX - roofRearX) * t, end + (H - end) * (1 - k * k)]);
    }
  }
  // Windscreen, bonnet and nose.
  pts.push([(roofFrontX + cowlX) / 2, (roofTop + cowlY) / 2], [cowlX, cowlY]);
  const noseX = xF - 0.12;
  pts.push([(cowlX + noseX) / 2, noseY + (cowlY - noseY) * 0.62], [noseX, noseY], [xF - 0.03, noseY - 0.05], [xF, noseY - 0.17]);
  const top: [number, number][] = [];
  for (const [x, y] of pts.sort((a, b) => a[0] - b[0])) {
    const u = clamp(station(x), 0, 1);
    const last = top[top.length - 1];
    if (last && u - last[0] < 0.004) continue;
    top.push([u, clamp(y, s.rideHeight + 0.2, H) / H]);
  }
  top[0]![0] = 0;
  top[top.length - 1]![0] = 1;

  // The side window opening.
  const aPillar = 0.07;
  const dloFrontBottom = cowlX - 0.05;
  const dloFrontTop = roofFrontX - aPillar;
  let dloRearBottom: number;
  let dloRearTop: number;
  if (s.style === "pickup") {
    dloRearBottom = tailX + 0.14;
    dloRearTop = tailX + 0.16;
  } else if (s.style === "roadster") {
    dloRearBottom = Math.max(cowlX - 1.25, tailX + 0.3);
    dloRearTop = dloRearBottom;
  } else {
    // Where the rear glass crosses the window line, then the last pillar.
    const glassAtBelt = s.style === "notchback" || s.style === "fastback" || s.style === "convertible"
      ? roofRearX - (roofRearY - beltRear) / tan(s.rearWindow)
      : tailX + (beltRear + 0.03 - glassBottom) / tan(s.rearWindow);
    dloRearBottom = glassAtBelt + s.lastPillar;
    dloRearTop = roofRearX + Math.max(0.08, s.lastPillar * 0.45);
  }
  dloRearBottom = clamp(dloRearBottom, xR + 0.15, dloFrontBottom - 0.8);
  dloRearTop = clamp(dloRearTop, dloRearBottom - 0.6, dloFrontTop - 0.4);

  // Pillars and dividers between the windows, and the door seams below.
  const pillars: number[] = [];
  const widths: number[] = [];
  const thick = s.frameless ? 0.03 : 0.075;
  const seams: Seam[] = [];
  const archTop = r + 0.07;
  const sill = s.rideHeight + 0.08;
  const seamBottom = (x: number) =>
    Math.abs(x - frontAxle) < r + 0.06 || Math.abs(x - rearAxle) < r + 0.06 ? archTop + 0.02 : sill;
  const addSeam = (x: number) => seams.push({ x, top: belt + (beltRear - belt) * (1 - (x - dloRearBottom) / Math.max(0.1, dloFrontBottom - dloRearBottom)), bottom: seamBottom(x) });
  addSeam(cowlX - 0.02);
  if (s.style === "van") {
    const b = cowlX - 0.95;
    pillars.push(b, rearAxle + 0.1);
    widths.push(thick, thick);
    addSeam(b);
  } else if (s.doors === 4) {
    const b = cowlX - (s.style === "pickup" ? 1.0 : 1.02);
    pillars.push(b);
    widths.push(thick);
    addSeam(b);
    // The rear door ends over the rear wheel arch.
    const rearDoor = clamp(rearAxle + r * 0.55, b - 1.15, b - 0.7);
    addSeam(rearDoor);
    if (s.style === "estate" || s.style === "suv") {
      pillars.push(rearDoor + 0.02);
      widths.push(s.frameless ? 0.035 : 0.06);
    } else if (s.quarterWindow) {
      pillars.push(Math.max(rearDoor + 0.02, dloRearBottom + 0.18));
      widths.push(0.025);
    }
  } else if (s.style !== "roadster") {
    const door = cowlX - 1.28;
    addSeam(door);
    if (s.quarterWindow) {
      pillars.push(door + 0.02);
      widths.push(s.frameless ? 0.025 : 0.05);
    }
  } else {
    addSeam(cowlX - 1.15);
  }
  const keep = pillars.map((p, i) => [p, widths[i]!] as const).filter(([p]) => p > dloRearBottom + 0.1 && p < dloFrontBottom - 0.2);

  const profile: Profile = {
    top,
    belt: belt / H,
    beltRear: beltRear / H,
    cowl: station(cowlX),
    roofFront: station(roofFrontX),
    roofRear: station(roofRearX),
    tail: station(tailX),
    dlo: [station(dloFrontBottom), station(dloFrontTop), station(dloRearBottom), station(dloRearTop)],
    pillars: keep.map(([p]) => station(p)),
    pillarWidths: keep.map(([, w]) => w / L),
    kink: s.kink ? 0.09 / L : 0,
    seams,
    clearance: s.rideHeight,
    rearSeats: s.style !== "roadster" && L > 3.1,
    tumblehome: s.tumblehome,
    noseRound: s.noseRound,
    tailRound: s.tailRound,
    rearLift: station(rearAxle - r * 0.9),
    frontLift: station(frontAxle + r * 0.9),
  };
  return { profile, frontAxle, rearAxle, wheelRadius: r };
}
