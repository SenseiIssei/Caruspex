// The geometry of the side view: body outline, windows, door lines and the
// parts behind the faults, laid out in a fixed 1000 by 400 field.
import type { BodyStyle } from "../../api";
import { CarZone } from "../../api";
import { PathBuilder, pointAtPercent, xAtY, type Cubic } from "./path";

export const FIELD_W = 1000;
export const FIELD_H = 400;

// app/VehicleDiagram.h BodyStyle.
const Hatchback = 0;
const Sedan = 1;
const Estate = 2;
const Suv = 3;
const Van = 4;
const Coupe = 5;
const Pickup = 6;
const Microcar = 7;

// --- Geometry ---------------------------------------------------------------------

export interface Shape {
  front: number;
  rear: number; // x extents of the body
  wheelF: number;
  wheelR: number;
  wheelRad: number;
  ground: number; // y of the road
  sill: number; // lower edge of the doors
  bumper: number; // lower edge of the bumpers
  nose: number; // top of the front face
  hood: number; // hood at the windscreen
  belt: number; // lower edge of the side windows
  roof: number;
  aBase: number;
  roofFront: number;
  roofRear: number;
  cBase: number;
  deck: number; // boot lid or load bed height
  tail: number; // where the rear face starts
  notch: boolean; // separate boot (saloon, pick-up)
  pillars: number; // side window splits
}

// Real proportions in metres of a typical car of each kind, measured from
// the front bumper and from the road. Heights become y values below.
// length, wheelbase, frontOverhang, wheelRadius, sill, bumper, nose, hood,
// belt, roof, aBase, roofFront, roofRear, cBase, deck, tail, notch, pillars
type Metres = [number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, boolean, number];

const METRES: Record<number, Metres> = {
  [Hatchback]: [4.26, 2.64, 0.86, 0.32, 0.22, 0.17, 0.74, 0.96, 0.99, 1.45, 1.24, 2.02, 3.42, 4.18, 1.02, 0.86, false, 1], // Golf class
  [Sedan]: [4.7, 2.81, 0.93, 0.32, 0.22, 0.17, 0.72, 0.95, 0.99, 1.43, 1.38, 2.2, 3.5, 4.05, 1.02, 0.86, true, 1], // A4 class
  [Estate]: [4.72, 2.81, 0.93, 0.32, 0.22, 0.17, 0.72, 0.95, 0.99, 1.45, 1.38, 2.2, 4.38, 4.62, 1.22, 1.0, false, 2],
  [Suv]: [4.5, 2.68, 0.92, 0.36, 0.38, 0.3, 0.92, 1.12, 1.16, 1.67, 1.4, 2.15, 4.08, 4.4, 1.16, 1.0, false, 2], // Tiguan class
  [Van]: [4.9, 3.0, 0.98, 0.34, 0.38, 0.3, 0.95, 1.1, 1.18, 1.97, 0.78, 1.3, 4.82, 4.87, 1.86, 1.76, false, 3], // Transporter class
  [Coupe]: [4.18, 2.5, 0.85, 0.33, 0.2, 0.15, 0.7, 0.92, 0.97, 1.35, 1.5, 2.22, 2.8, 3.86, 0.98, 0.85, true, 1], // TT class
  [Pickup]: [5.32, 3.09, 0.92, 0.38, 0.42, 0.36, 1.05, 1.25, 1.3, 1.8, 1.55, 2.2, 3.28, 3.38, 1.3, 1.22, true, 1], // Hilux class
  [Microcar]: [2.7, 1.87, 0.45, 0.3, 0.24, 0.18, 0.72, 0.95, 1.0, 1.55, 0.75, 1.25, 2.38, 2.62, 1.1, 0.95, false, 0], // fortwo class
};

function shapeFor(style: BodyStyle): Shape {
  const [length, wheelbase, frontOverhang, wheelRadius, sill, bumper, nose, hood, belt, roof, aBase, roofFront, roofRear, cBase, deck, tail, notch, pillars] =
    METRES[style] ?? METRES[Hatchback]!;
  // As large as fits: at most 900 long and 330 high, never above 200 per metre.
  const u = Math.min(900 / length, 330 / roof, 200);
  const ground = 380;
  const front = (FIELD_W - length * u) / 2;
  const x = (m: number) => front + m * u;
  const y = (m: number) => ground - m * u;
  return {
    front,
    rear: x(length),
    wheelF: x(frontOverhang),
    wheelR: x(frontOverhang + wheelbase),
    wheelRad: wheelRadius * u,
    ground,
    sill: y(sill),
    bumper: y(bumper),
    nose: y(nose),
    hood: y(hood),
    belt: y(belt),
    roof: y(roof),
    aBase: x(aBase),
    roofFront: x(roofFront),
    roofRear: x(roofRear),
    cBase: x(cBase),
    deck: y(deck),
    tail: y(tail),
    notch,
    pillars,
  };
}

export interface Part {
  zone: CarZone;
  d: string;
  anchor: { x: number; y: number }; // where the label line ends
  label: string; // German source, translated when drawn
  small: boolean; // small parts win the hit test over big ones behind them
  // Set for parts that are a thick line (the data bus): drawn as a stroke of
  // this width instead of a filled area.
  line?: number;
}

export interface Geometry {
  shape: Shape;
  body: string;
  glass: string;
  cuts: string;
  doorLines: Cubic[];
  handles: number[];
  parts: Part[];
}

export function build(style: BodyStyle): Geometry {
  const s = shapeFor(style);
  const wheelY = s.ground - s.wheelRad;
  const archR = s.wheelRad + 9;

  // --- Body outline, clockwise from the bottom of the front bumper ---------
  const b = new PathBuilder();
  b.moveTo(s.front + 22, s.bumper);
  b.cubicTo(s.front + 4, s.bumper - 4, s.front, s.nose + 52, s.front, s.nose + 26);
  b.cubicTo(s.front, s.nose + 6, s.front + 12, s.nose - 6, s.front + 46, s.nose - 10);
  // Hood, slightly domed, up to the base of the windscreen.
  b.cubicTo(s.front + (s.aBase - s.front) * 0.45, s.hood - 6, s.aBase - 40, s.hood - 2, s.aBase, s.hood);
  // Windscreen
  b.cubicTo(
    s.aBase + (s.roofFront - s.aBase) * 0.55,
    s.hood - (s.hood - s.roof) * 0.62,
    s.roofFront - 30,
    s.roof + 2,
    s.roofFront,
    s.roof,
  );
  // Roof
  b.cubicTo(
    s.roofFront + (s.roofRear - s.roofFront) * 0.35,
    s.roof - 7,
    s.roofRear - (s.roofRear - s.roofFront) * 0.25,
    s.roof - 4,
    s.roofRear,
    s.roof + 4,
  );
  if (s.notch) {
    // Rear window down to the boot lid, then the boot.
    b.cubicTo(s.roofRear + (s.cBase - s.roofRear) * 0.45, s.roof + 18, s.cBase - 22, s.deck - 8, s.cBase, s.deck);
    b.lineTo(s.rear - 28, s.deck + 3);
    b.cubicTo(s.rear - 8, s.deck + 5, s.rear, s.deck + 14, s.rear, s.tail);
  } else {
    // Hatch or estate: one sweep from the roof to the tail.
    const run = s.cBase - s.roofRear;
    b.cubicTo(
      s.roofRear + run * 0.45,
      s.roof + 4 + (s.deck - s.roof) * 0.1,
      s.cBase + 6 - run * 0.08,
      s.deck - (s.deck - s.roof) * 0.3,
      s.cBase + 6,
      s.deck,
    );
    b.cubicTo(s.cBase + 8, s.deck + 6, s.rear, s.deck + 2, s.rear, s.tail);
  }
  b.lineTo(s.rear, s.bumper - 28);
  b.cubicTo(s.rear, s.bumper - 6, s.rear - 6, s.bumper, s.rear - 24, s.bumper);

  // Lower edge with the two wheel arches: from the right end of the arch
  // over the top to its left end, counter-clockwise on screen.
  const arch = (cx: number) => {
    const dy = Math.max(-archR + 1, Math.min(s.sill - wheelY, archR - 1));
    const a = (Math.asin(dy / archR) * 180) / Math.PI;
    const dx = Math.sqrt(archR * archR - dy * dy);
    b.lineTo(cx + dx, s.sill);
    if (Math.abs(wheelY + dy - s.sill) > 0.01) b.lineTo(cx + dx, wheelY + dy);
    b.arcTo(archR, 180 + 2 * a > 180, false, cx - dx, wheelY + dy);
  };
  b.lineTo(s.rear - 40, s.sill);
  arch(s.wheelR);
  arch(s.wheelF);
  b.lineTo(s.front + 40, s.sill);
  b.cubicTo(s.front + 30, s.sill, s.front + 26, s.bumper, s.front + 22, s.bumper);
  b.close();

  // --- Side windows -----------------------------------------------------------
  const glassBottom = s.belt - 3;
  const glassTop = s.roof + 13;
  const xFrontBottom = xAtY(s.aBase, s.hood, s.roofFront, s.roof, glassBottom) + 16;
  const xFrontTop = xAtY(s.aBase, s.hood, s.roofFront, s.roof, glassTop) + 14;
  const rwTopX = s.roofRear;
  const rwTopY = s.roof + 4;
  const rwBottomX = s.notch ? s.cBase : s.cBase + 6;
  const rwBottomY = s.deck;
  let xRearBottom = xAtY(rwTopX, rwTopY, rwBottomX, rwBottomY, glassBottom) - 18;
  const xRearTop = xAtY(rwTopX, rwTopY, rwBottomX, rwBottomY, glassTop) - 12;
  if (s.pillars >= 2) {
    // estate, SUV, van: the glass runs almost to the back
    xRearBottom = Math.min(xRearBottom, s.cBase - 18);
  }
  const g = new PathBuilder();
  g.moveTo(xFrontBottom, glassBottom);
  g.lineTo(xFrontTop, glassTop);
  g.cubicTo(xFrontTop + 40, glassTop - 4, xRearTop - 40, glassTop - 2, xRearTop, glassTop + 2);
  g.lineTo(xRearBottom, glassBottom);
  g.close();

  // Pillars are cut out of the glass (a mask does what subtracted() did).
  const cuts = new PathBuilder();
  const span = xRearBottom - xFrontBottom;
  const pillarX = (i: number) => xFrontBottom + span * (i / (s.pillars + 1)) * (s.pillars === 1 ? 0.95 : 1.0);
  for (let i = 1; i <= s.pillars; i++) {
    const x = pillarX(i);
    cuts.rect(x - 7, glassTop - 10, 14, glassBottom - glassTop + 20);
  }

  // --- Door shut lines ----------------------------------------------------------
  const doorLines: Cubic[] = [];
  const vertical = (xTop: number, xBottom: number) => {
    doorLines.push({
      x0: xTop,
      y0: glassTop - 6,
      x1: xTop,
      y1: s.belt + 20,
      x2: xBottom,
      y2: s.belt + 40,
      x3: xBottom,
      y3: s.sill - 2,
    });
  };
  const frontDoor = s.wheelF + archR + 14;
  vertical(xFrontTop - 6, frontDoor);
  if (s.pillars >= 1) {
    const xb = pillarX(1);
    vertical(xb, xb + 4);
    if (style !== Coupe && style !== Pickup) {
      const rearDoor = Math.min(xRearBottom - 4, s.wheelR - archR - 8);
      vertical(rearDoor + 6, rearDoor);
    }
  }
  const handles: number[] = [];
  for (let i = 0; i < doorLines.length - 1 && i < 2; i++) {
    handles.push(pointAtPercent(doorLines[i + 1]!, 0.4).x - 34);
  }

  // --- Parts behind the faults -----------------------------------------------------
  const parts: Part[] = [];
  const bayLeft = s.front + 38;
  const bayRight = s.aBase - 18;
  const bayW = bayRight - bayLeft;
  const bayTop = (s.nose + s.hood) / 2 + 10;
  const floorY = s.sill + 2;

  const eX = bayLeft + bayW * 0.3;
  const eY = bayTop + 26;
  const eW = bayW * 0.46;
  const eH = s.sill - 14 - (bayTop + 26);
  const engine = { left: eX, top: eY, right: eX + eW, bottom: eY + eH, width: eW, cx: eX + eW / 2, cy: eY + eH / 2 };

  const add = (zone: CarZone, path: PathBuilder, anchor: { x: number; y: number }, label: string, line?: number) => {
    const width = path.width + (line ?? 0);
    parts.push({ zone, d: path.d, anchor, label, small: width < 60, line });
  };
  const at = (x: number, y: number) => ({ x, y });

  // Radiator right behind the grille.
  add(CarZone.Cooling, new PathBuilder().roundedRect(s.front + 16, s.nose + 6, 13, s.sill - s.nose - 22, 3), at(s.front + 22, s.nose + 30), "Kühlung");
  // Air box at the front top of the bay.
  add(CarZone.Intake, new PathBuilder().roundedRect(bayLeft + 4, bayTop, bayW * 0.3, 24, 7), at(bayLeft + 4 + bayW * 0.15, bayTop + 12), "Ansaugung");
  // Throttle body where the intake meets the engine.
  add(CarZone.Throttle, new PathBuilder().ellipse(engine.left - 4, bayTop + 34, 10, 10), at(engine.left - 4, bayTop + 34), "Drosselklappe");
  // Engine block with its head.
  add(CarZone.Cylinders, new PathBuilder().roundedRect(engine.left, engine.top, engine.width, eH, 8), at(engine.cx, engine.cy), "Motor");
  // Fuel rail across the top of the head.
  add(
    CarZone.Injection,
    new PathBuilder().roundedRect(engine.left + 6, engine.top - 9, engine.width - 12, 6, 3),
    at(engine.cx, engine.top - 6),
    "Einspritzung",
  );
  // Ignition coils sit on the head.
  {
    const coils = new PathBuilder();
    for (let i = 0; i < 4; i++) {
      const x = engine.left + 10 + (i * (engine.width - 20)) / 3 - 4;
      coils.roundedRect(x, engine.top + 6, 8, 14, 2);
    }
    add(CarZone.Ignition, coils, at(engine.right - 14, engine.top + 12), "Zündung");
  }
  // Turbo at the lower front of the engine.
  add(CarZone.Turbo, new PathBuilder().ellipse(engine.left - 14, engine.bottom - 18, 13, 13), at(engine.left - 14, engine.bottom - 18), "Turbo");
  // Gearbox behind the engine.
  {
    const gb = new PathBuilder();
    gb.moveTo(engine.right, engine.top + 22);
    gb.lineTo(engine.right + 58, engine.top + 34);
    gb.lineTo(engine.right + 58, engine.bottom - 4);
    gb.lineTo(engine.right, engine.bottom);
    gb.close();
    add(CarZone.Gearbox, gb, at(engine.right + 30, engine.cy + 8), "Getriebe");
  }
  // Battery at the back of the bay.
  add(CarZone.Electrics, new PathBuilder().roundedRect(bayRight - 52, bayTop - 2, 40, 26, 4), at(bayRight - 32, bayTop + 11), "Bordnetz");
  // Engine control unit behind the dashboard.
  add(CarZone.Ecu, new PathBuilder().roundedRect(s.aBase + 14, s.belt + 14, 34, 20, 4), at(s.aBase + 31, s.belt + 24), "Steuergerät");
  // Exhaust: from the engine down, under the floor, to the back.
  {
    const ex = new PathBuilder();
    const pipe = 5;
    const catX = engine.right + 78;
    const mufflerX = s.wheelR + 30;
    ex.roundedRect(engine.cx - 4, engine.bottom - 2, 8, floorY - engine.bottom + 4, 3);
    ex.roundedRect(engine.cx, floorY - pipe / 2, s.rear - 30 - engine.cx, pipe, 2);
    ex.roundedRect(catX, floorY - 10, 58, 20, 8);
    ex.roundedRect(mufflerX, floorY - 12, Math.max(30, s.rear - 60 - mufflerX), 22, 9);
    add(CarZone.Exhaust, ex, at(catX + 29, floorY), "Abgas");
  }
  // Fuel tank under the rear seat.
  add(CarZone.FuelTank, new PathBuilder().roundedRect(s.wheelR - archR - 150, s.sill - 34, 128, 30, 9), at(s.wheelR - archR - 86, s.sill - 19), "Tank");
  // Cabin: dashboard and the two seats.
  {
    const cabin = new PathBuilder();
    const seatY = s.belt + 18;
    const seat = (x: number) => {
      cabin.moveTo(x, s.sill - 22);
      cabin.cubicTo(x - 6, seatY + 30, x - 12, seatY, x - 4, seatY - 22);
      cabin.lineTo(x + 10, seatY - 20);
      cabin.cubicTo(x + 6, seatY + 10, x + 12, seatY + 34, x + 16, s.sill - 40);
      cabin.lineTo(x + 70, s.sill - 40);
      cabin.lineTo(x + 70, s.sill - 22);
      cabin.close();
    };
    const frontSeat = s.roofFront + (s.roofRear - s.roofFront) * 0.3;
    seat(frontSeat);
    if (style !== Coupe && style !== Microcar && style !== Pickup) {
      seat(Math.min(s.wheelR - archR - 70, frontSeat + 210));
    }
    cabin.ellipse(s.aBase + 70, s.belt + 2, 6, 16);
    add(CarZone.Cabin, cabin, at(frontSeat + 20, s.belt + 30), "Innenraum");
  }
  // Data bus: a line from the front to the back through the cabin.
  {
    const bus = new PathBuilder();
    bus.moveTo(bayRight - 30, bayTop + 26);
    bus.cubicTo(s.aBase + 40, s.belt + 60, s.roofRear - 60, s.sill - 14, s.rear - 60, s.tail + 20);
    add(CarZone.Network, bus, at((s.aBase + s.roofRear) / 2, s.sill - 24), "Datenbus", 4);
  }
  // Wheels and brakes
  {
    const w = new PathBuilder();
    w.ellipse(s.wheelF, wheelY, s.wheelRad * 0.45, s.wheelRad * 0.45);
    w.ellipse(s.wheelR, wheelY, s.wheelRad * 0.45, s.wheelRad * 0.45);
    add(CarZone.Wheels, w, at(s.wheelF, wheelY), "Räder und ABS");
  }

  return { shape: s, body: b.d, glass: g.d, cuts: cuts.d, doorLines, handles, parts };
}
