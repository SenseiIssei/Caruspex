// Side view of a car, drawn in code (VehicleDiagram.cpp). Two uses:
//
//  - On the start page as a clean picture of the connected car, in the body
//    style the catalog knows for it.
//  - On the fault page in "see-through" mode: the body turns into glass and
//    the parts behind a fault (engine, intake, exhaust, tank, gearbox, ...)
//    light up where they sit in a real car.
//
// The car is laid out in a fixed 1000 by 400 field and scaled to the element
// without distortion (the SVG viewBox does what fieldToWidget() did), so it
// looks right at every size. The labels above the car are drawn in a second
// layer in real pixels, so their text stays the same size at every width.
import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import type { BodyStyle } from "../api";
import { CarZone } from "../api";
import { t } from "../i18n";
import { usePageActive } from "../lib/hooks";
import { parseColor, type Rgb } from "../lib/theme";
import "./VehicleDiagram.css";

export interface VehicleDiagramProps {
  body: BodyStyle;
  // Paint colour of the body (CSS colour). Default "#4b5878".
  paint?: string;
  // Glass body with the parts inside. Off for the plain picture.
  anatomy?: boolean;
  // A light sweep across the car while something is being read.
  scanning?: boolean;
  // All zones in which something was found.
  affected?: readonly CarZone[];
  // The currently selected zone is highlighted and pulses.
  focus?: CarZone;
  onZoneClick?: (zone: CarZone) => void;
  className?: string;
}

const CTX = "caruspex::VehicleDiagram";
const FIELD_W = 1000;
const FIELD_H = 400;

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

interface Shape {
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

// Point on a straight line between a and b at height y.
function xAtY(ax: number, ay: number, bx: number, by: number, y: number): number {
  if (Math.abs(ay - by) < 1e-9) return ax;
  return ax + ((bx - ax) * (y - ay)) / (by - ay);
}

const n = (v: number) => (Math.round(v * 100) / 100).toString();

// Collects an SVG path and its horizontal extent (for the "small part"
// rule of the hit test).
class PathBuilder {
  private out: string[] = [];
  minX = Infinity;
  maxX = -Infinity;

  private see(...xs: number[]): void {
    for (const x of xs) {
      if (x < this.minX) this.minX = x;
      if (x > this.maxX) this.maxX = x;
    }
  }
  moveTo(x: number, y: number): this {
    this.see(x);
    this.out.push(`M${n(x)} ${n(y)}`);
    return this;
  }
  lineTo(x: number, y: number): this {
    this.see(x);
    this.out.push(`L${n(x)} ${n(y)}`);
    return this;
  }
  cubicTo(c1x: number, c1y: number, c2x: number, c2y: number, x: number, y: number): this {
    this.see(x);
    this.out.push(`C${n(c1x)} ${n(c1y)} ${n(c2x)} ${n(c2y)} ${n(x)} ${n(y)}`);
    return this;
  }
  arcTo(r: number, large: boolean, sweep: boolean, x: number, y: number): this {
    this.see(x);
    this.out.push(`A${n(r)} ${n(r)} 0 ${large ? 1 : 0} ${sweep ? 1 : 0} ${n(x)} ${n(y)}`);
    return this;
  }
  close(): this {
    this.out.push("Z");
    return this;
  }
  rect(x: number, y: number, w: number, h: number): this {
    return this.moveTo(x, y).lineTo(x + w, y).lineTo(x + w, y + h).lineTo(x, y + h).close();
  }
  roundedRect(x: number, y: number, w: number, h: number, radius: number): this {
    const r = Math.max(0, Math.min(radius, w / 2, h / 2));
    this.see(x, x + w);
    this.out.push(
      `M${n(x + r)} ${n(y)}H${n(x + w - r)}A${n(r)} ${n(r)} 0 0 1 ${n(x + w)} ${n(y + r)}V${n(y + h - r)}` +
        `A${n(r)} ${n(r)} 0 0 1 ${n(x + w - r)} ${n(y + h)}H${n(x + r)}A${n(r)} ${n(r)} 0 0 1 ${n(x)} ${n(y + h - r)}` +
        `V${n(y + r)}A${n(r)} ${n(r)} 0 0 1 ${n(x + r)} ${n(y)}Z`,
    );
    return this;
  }
  ellipse(cx: number, cy: number, rx: number, ry: number): this {
    this.see(cx - rx, cx + rx);
    this.out.push(
      `M${n(cx + rx)} ${n(cy)}A${n(rx)} ${n(ry)} 0 1 1 ${n(cx - rx)} ${n(cy)}A${n(rx)} ${n(ry)} 0 1 1 ${n(cx + rx)} ${n(cy)}Z`,
    );
    return this;
  }
  get d(): string {
    return this.out.join("");
  }
  get width(): number {
    return this.maxX - this.minX;
  }
}

interface Cubic {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  x3: number;
  y3: number;
}

function cubicAt(c: Cubic, t: number): { x: number; y: number } {
  const u = 1 - t;
  return {
    x: u * u * u * c.x0 + 3 * u * u * t * c.x1 + 3 * u * t * t * c.x2 + t * t * t * c.x3,
    y: u * u * u * c.y0 + 3 * u * u * t * c.y1 + 3 * u * t * t * c.y2 + t * t * t * c.y3,
  };
}

// QPainterPath::pointAtPercent measures along the length of the curve, not
// along its parameter; sampled finely enough for a door handle.
function pointAtPercent(c: Cubic, percent: number): { x: number; y: number } {
  const steps = 64;
  const lengths = [0];
  let prev = cubicAt(c, 0);
  for (let i = 1; i <= steps; i++) {
    const p = cubicAt(c, i / steps);
    lengths.push(lengths[i - 1]! + Math.hypot(p.x - prev.x, p.y - prev.y));
    prev = p;
  }
  const target = lengths[steps]! * percent;
  for (let i = 1; i <= steps; i++) {
    if (lengths[i]! >= target) {
      const span = lengths[i]! - lengths[i - 1]!;
      const f = span > 0 ? (target - lengths[i - 1]!) / span : 0;
      return cubicAt(c, (i - 1 + f) / steps);
    }
  }
  return cubicAt(c, 1);
}

interface Part {
  zone: CarZone;
  d: string;
  anchor: { x: number; y: number }; // where the label line ends
  label: string; // German source, translated when drawn
  small: boolean; // small parts win the hit test over big ones behind them
  // Set for parts that are a thick line (the data bus): drawn as a stroke of
  // this width instead of a filled area.
  line?: number;
}

interface Geometry {
  shape: Shape;
  body: string;
  glass: string;
  cuts: string;
  doorLines: Cubic[];
  handles: number[];
  parts: Part[];
}

function build(style: BodyStyle): Geometry {
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

// The zone labels have to be visible to the translation scanner as the texts
// they are (QT_TR_NOOP in the C++ code).
function zoneLabel(label: string): string {
  switch (label) {
    case "Kühlung": return t(CTX, "Kühlung");
    case "Ansaugung": return t(CTX, "Ansaugung");
    case "Drosselklappe": return t(CTX, "Drosselklappe");
    case "Motor": return t(CTX, "Motor");
    case "Einspritzung": return t(CTX, "Einspritzung");
    case "Zündung": return t(CTX, "Zündung");
    case "Turbo": return t(CTX, "Turbo");
    case "Getriebe": return t(CTX, "Getriebe");
    case "Bordnetz": return t(CTX, "Bordnetz");
    case "Steuergerät": return t(CTX, "Steuergerät");
    case "Abgas": return t(CTX, "Abgas");
    case "Tank": return t(CTX, "Tank");
    case "Innenraum": return t(CTX, "Innenraum");
    case "Datenbus": return t(CTX, "Datenbus");
    case "Räder und ABS": return t(CTX, "Räder und ABS");
  }
  return label;
}

// --- Colour helpers ------------------------------------------------------------------

// QColor::lighter and darker work on the HSV value; lighter takes the
// overflow above full brightness out of the saturation.
function toHsv({ r, g, b }: Rgb): { h: number; s: number; v: number } {
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d > 0) {
    if (max === r) h = ((g - b) / d) % 6;
    else if (max === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
    h *= 60;
    if (h < 0) h += 360;
  }
  return { h, s: max === 0 ? 0 : (d / max) * 255, v: max };
}

function fromHsv(h: number, s: number, v: number): Rgb {
  const sat = s / 255;
  const c = v * sat;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = v - c;
  const [r, g, b] =
    h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x];
  return { r: r + m, g: g + m, b: b + m };
}

function lighter(c: Rgb, factor: number): Rgb {
  const hsv = toHsv(c);
  let v = (hsv.v * factor) / 100;
  let s = hsv.s;
  if (v > 255) {
    s = Math.max(0, s - (v - 255));
    v = 255;
  }
  return fromHsv(hsv.h, s, v);
}

function darker(c: Rgb, factor: number): Rgb {
  const hsv = toHsv(c);
  return fromHsv(hsv.h, hsv.s, (hsv.v * 100) / factor);
}

const rgb = ({ r, g, b }: Rgb) => `rgb(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)})`;

// --- Labels -----------------------------------------------------------------------------

let measureCanvas: HTMLCanvasElement | null = null;

function textWidth(text: string, font: string): number {
  if (typeof document === "undefined") return text.length * 7.5;
  measureCanvas ??= document.createElement("canvas");
  const ctx = measureCanvas.getContext("2d");
  if (!ctx) return text.length * 7.5;
  ctx.font = font;
  return ctx.measureText(text).width;
}

interface Chip {
  part: Part;
  text: string;
  x: number;
  w: number;
  hit: boolean;
}

// One chip per affected zone above the car, plus the hovered one, joined to
// the part by a thin line. Chips are spread so they never overlap.
function layoutChips(parts: Part[], affected: Set<CarZone>, hover: CarZone, width: number, map: (x: number) => number): Chip[] {
  const family = typeof document !== "undefined" ? getComputedStyle(document.documentElement).getPropertyValue("--font-sans") : "";
  const font = `600 13px ${family || "sans-serif"}`;
  const chips: Chip[] = [];
  for (const part of parts) {
    const hit = affected.has(part.zone);
    if (!hit && part.zone !== hover) continue;
    const text = zoneLabel(part.label);
    chips.push({ part, text, x: map(part.anchor.x), w: textWidth(text, font) + 26, hit });
  }
  chips.sort((a, b) => a.x - b.x);
  // Spread left to right, then pull back inside the field on the right.
  let cursor = 8;
  for (const c of chips) {
    c.x = Math.max(c.x - c.w / 2, cursor);
    cursor = c.x + c.w + 8;
  }
  const right = width - 8;
  let overflow = cursor - 8 - right;
  for (let i = chips.length - 1; i >= 0 && overflow > 0; i--) {
    const room = i > 0 ? chips[i]!.x - (chips[i - 1]!.x + chips[i - 1]!.w + 8) : chips[i]!.x - 8;
    const shift = Math.min(overflow, Math.max(0, room));
    for (let j = i; j < chips.length; j++) chips[j]!.x -= shift;
    overflow -= shift;
  }
  return chips;
}

// --- Component ----------------------------------------------------------------------------

export function VehicleDiagram({
  body,
  paint = "#4b5878",
  anatomy = false,
  scanning = false,
  affected,
  focus = CarZone.Unknown,
  onZoneClick,
  className,
}: VehicleDiagramProps) {
  const rawId = useId();
  const id = rawId.replace(/[^a-zA-Z0-9_-]/g, "");
  const active = usePageActive();
  const geo = useMemo(() => build(body), [body]);
  const s = geo.shape;
  const [hover, setHover] = useState<CarZone>(CarZone.Unknown);
  const box = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });

  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const measure = () => setSize({ w: el.clientWidth, h: el.clientHeight });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // Without the see-through mode nothing can be hovered.
  useEffect(() => {
    if (!anatomy) setHover(CarZone.Unknown);
  }, [anatomy]);

  const hit = useMemo(() => new Set(affected ?? []), [affected]);
  const alpha = anatomy ? 0.32 : 1.0;
  const base = useMemo(() => parseColor(paint), [paint]);
  const wheelY = s.ground - s.wheelRad;
  const rim = s.wheelRad * 0.68;

  // fieldToWidget(): the same fit the viewBox does, for the label layer.
  const scale = Math.min(size.w / FIELD_W, size.h / FIELD_H) || 0;
  const offX = (size.w - FIELD_W * scale) / 2;
  const offY = (size.h - FIELD_H * scale) / 2;
  const mapX = (x: number) => offX + x * scale;
  const mapY = (y: number) => offY + y * scale;
  const chips = anatomy && scale > 0 ? layoutChips(geo.parts, hit, hover, size.w, mapX) : [];

  const len = s.rear - s.front;
  const sweepStyle = {
    "--sweep-from": `${s.front + len * -0.4}px`,
    "--sweep-to": `${s.front + len * 1.4}px`,
  } as CSSProperties;

  const lac = (c: Rgb, a = 1) => ({ stopColor: rgb(c), stopOpacity: alpha * a });
  const lensAlpha = Math.max(alpha, 0.6);
  const ref = (name: string) => `url(#${id}-${name})`;

  // Small parts are hit-tested first: they go on top in the hit layer.
  const hitOrder = [...geo.parts.filter((p) => !p.small), ...geo.parts.filter((p) => p.small)];

  return (
    <div
      ref={box}
      className={`vehicle-diagram${active ? "" : " paused"}${scanning ? " scanning" : ""}${className ? ` ${className}` : ""}`}
    >
      <svg className="vd-car" viewBox={`0 0 ${FIELD_W} ${FIELD_H}`} preserveAspectRatio="xMidYMid meet" aria-hidden>
        <defs>
          <radialGradient id={`${id}-shadow`}>
            <stop offset="0" stopColor="#000" stopOpacity={170 / 255} />
            <stop offset="0.6" stopColor="#000" stopOpacity={60 / 255} />
            <stop offset="1" stopColor="#000" stopOpacity={0} />
          </radialGradient>
          <linearGradient id={`${id}-floor`} x1="0" y1="0" x2="1" y2="0">
            <stop offset="0" style={{ stopColor: "var(--text)", stopOpacity: 0 }} />
            <stop offset="0.5" style={{ stopColor: "var(--text)", stopOpacity: 0.12 }} />
            <stop offset="1" style={{ stopColor: "var(--text)", stopOpacity: 0 }} />
          </linearGradient>
          {/* Wheel gradients in wheel-local coordinates, shared by both wheels. */}
          <radialGradient id={`${id}-tyre`} gradientUnits="userSpaceOnUse" cx="0" cy="0" r={s.wheelRad}>
            <stop offset="0.7" stopColor="rgb(30,33,40)" />
            <stop offset="0.86" stopColor="rgb(17,19,24)" />
            <stop offset="1" stopColor="rgb(10,11,14)" />
          </radialGradient>
          <radialGradient id={`${id}-metal`} gradientUnits="userSpaceOnUse" cx={-rim * 0.3} cy={-rim * 0.4} r={rim * 1.4}>
            <stop offset="0" stopColor="rgb(214,220,232)" />
            <stop offset="0.5" stopColor="rgb(132,140,158)" />
            <stop offset="1" stopColor="rgb(62,68,82)" />
          </radialGradient>
          <linearGradient id={`${id}-calliper`} gradientUnits="userSpaceOnUse" x1="0" y1={-rim * 0.8} x2="0" y2={-rim * 0.3}>
            <stop offset="0" style={{ stopColor: "var(--accent-hover)" }} />
            <stop offset="1" style={{ stopColor: "var(--accent-dim)" }} />
          </linearGradient>
          <linearGradient id={`${id}-spoke`} gradientUnits="userSpaceOnUse" x1={-rim * 0.2} y1="0" x2={rim * 0.2} y2="0">
            <stop offset="0" stopColor="rgb(170,178,194)" />
            <stop offset="1" stopColor="rgb(96,104,120)" />
          </linearGradient>
          <radialGradient id={`${id}-hub`} gradientUnits="userSpaceOnUse" cx="0" cy="0" r={rim * 0.22}>
            <stop offset="0" stopColor="rgb(220,226,236)" />
            <stop offset="1" stopColor="rgb(80,88,104)" />
          </radialGradient>
          {/* Paint: light from above, a bright shoulder line, dark towards the sills. */}
          <linearGradient id={`${id}-lacquer`} gradientUnits="userSpaceOnUse" x1="0" y1={s.roof} x2="0" y2={s.bumper}>
            <stop offset="0" style={lac(lighter(base, 165))} />
            <stop offset="0.3" style={lac(lighter(base, 125))} />
            <stop offset="0.44" style={lac(lighter(base, 150))} />
            <stop offset="0.5" style={lac(base)} />
            <stop offset="0.8" style={lac(darker(base, 160))} />
            <stop offset="1" style={lac(darker(base, 260))} />
          </linearGradient>
          <linearGradient id={`${id}-shine`} gradientUnits="userSpaceOnUse" x1="0" y1={s.belt + 6} x2="0" y2={s.belt + 22}>
            <stop offset="0" stopColor="#fff" stopOpacity={0} />
            <stop offset="0.5" stopColor="#fff" stopOpacity={alpha * 0.16} />
            <stop offset="1" stopColor="#fff" stopOpacity={0} />
          </linearGradient>
          <linearGradient id={`${id}-tint`} gradientUnits="userSpaceOnUse" x1="0" y1={s.roof} x2="0" y2={s.belt}>
            <stop offset="0" stopColor="rgb(54,64,88)" stopOpacity={alpha * (anatomy ? 0.6 : 1)} />
            <stop offset="1" stopColor="rgb(14,17,26)" stopOpacity={alpha * (anatomy ? 0.6 : 1)} />
          </linearGradient>
          <linearGradient id={`${id}-refl`} gradientUnits="userSpaceOnUse" x1={s.roofFront} y1={s.roof} x2={s.roofFront + 140} y2={s.belt}>
            <stop offset="0" stopColor="#fff" stopOpacity={0} />
            <stop offset="0.45" stopColor="#fff" stopOpacity={34 / 255} />
            <stop offset="0.55" stopColor="#fff" stopOpacity={0} />
          </linearGradient>
          <radialGradient id={`${id}-headglow`} gradientUnits="userSpaceOnUse" cx={s.front + 20} cy={s.nose + 2} r="70">
            <stop offset="0" stopColor="rgb(150,200,255)" stopOpacity={(90 * alpha) / 255} />
            <stop offset="1" stopColor="rgb(150,200,255)" stopOpacity={0} />
          </radialGradient>
          <linearGradient id={`${id}-headlens`} gradientUnits="userSpaceOnUse" x1={s.front + 10} y1="0" x2={s.front + 72} y2="0">
            <stop offset="0" stopColor="rgb(235,245,255)" stopOpacity={lensAlpha} />
            <stop offset="1" stopColor="rgb(120,170,255)" stopOpacity={(200 / 255) * lensAlpha} />
          </linearGradient>
          <radialGradient id={`${id}-tailglow`} gradientUnits="userSpaceOnUse" cx={s.rear - 6} cy={s.tail + 14} r="50">
            <stop offset="0" stopColor="rgb(255,70,80)" stopOpacity={(110 * alpha) / 255} />
            <stop offset="1" stopColor="rgb(255,70,80)" stopOpacity={0} />
          </radialGradient>
          <linearGradient id={`${id}-taillens`} gradientUnits="userSpaceOnUse" x1="0" y1={s.tail + 6} x2="0" y2={s.tail + 22}>
            <stop offset="0" stopColor="rgb(255,120,120)" stopOpacity={lensAlpha} />
            <stop offset="1" stopColor="rgb(200,30,50)" stopOpacity={lensAlpha} />
          </linearGradient>
          <linearGradient id={`${id}-rimlight`} gradientUnits="userSpaceOnUse" x1={s.front} y1="0" x2={s.rear} y2="0">
            <stop offset="0" stopColor="#fff" stopOpacity={10 / 255} />
            <stop offset="0.45" stopColor="#fff" stopOpacity={(anatomy ? 70 : 60) / 255} />
            <stop offset="1" stopColor="#fff" stopOpacity={10 / 255} />
          </linearGradient>
          <linearGradient id={`${id}-sweep`} gradientUnits="userSpaceOnUse" x1="-60" y1="0" x2="60" y2="0">
            <stop offset="0" stopColor="#fff" stopOpacity={0} />
            {scanning ? (
              <stop offset="0.5" style={{ stopColor: "var(--accent)", stopOpacity: 70 / 255 }} />
            ) : (
              <stop offset="0.5" stopColor="#fff" stopOpacity={26 / 255} />
            )}
            <stop offset="1" stopColor="#fff" stopOpacity={0} />
          </linearGradient>
          <clipPath id={`${id}-body`}>
            <path d={geo.body} />
          </clipPath>
          <clipPath id={`${id}-glassclip`}>
            <path d={geo.glass} />
          </clipPath>
          <mask id={`${id}-pillars`} maskUnits="userSpaceOnUse" x="0" y="0" width={FIELD_W} height={FIELD_H}>
            <rect x="0" y="0" width={FIELD_W} height={FIELD_H} fill="#fff" />
            {geo.cuts ? <path d={geo.cuts} fill="#000" /> : null}
          </mask>
          {geo.parts
            .filter((p) => p.line)
            .map((p) => (
              // The outline of a thick line: a wide stroke minus a narrow one.
              <mask key={p.zone} id={`${id}-tube-${p.zone}`} maskUnits="userSpaceOnUse" x="0" y="0" width={FIELD_W} height={FIELD_H}>
                <path d={p.d} fill="none" stroke="#fff" strokeWidth={(p.line ?? 0) + 1.2} />
                <path d={p.d} fill="none" stroke="#000" strokeWidth={Math.max(0.5, (p.line ?? 0) - 1.2)} />
              </mask>
            ))}
        </defs>

        {/* Ground: a soft shadow and a faint floor line that fades out to the sides. */}
        <ellipse
          cx={(s.front + s.rear) / 2}
          cy={s.ground}
          rx={len * 0.62}
          ry={len * 0.62 * 0.07}
          fill={ref("shadow")}
        />
        <line x1="10" y1={s.ground + 1} x2={FIELD_W - 10} y2={s.ground + 1} stroke={ref("floor")} strokeWidth="1.2" />

        {/* Wheels */}
        {[s.wheelF, s.wheelR].map((cx) => (
          <g key={cx} transform={`translate(${n(cx)} ${n(wheelY)})`}>
            <circle r={s.wheelRad + 6} fill="rgb(6,7,10)" />
            <circle r={s.wheelRad} fill={ref("tyre")} />
            <circle r={rim} fill={ref("metal")} />
            {/* Brake disc and calliper visible between the spokes */}
            <circle r={rim * 0.86} fill="rgb(40,44,54)" />
            <rect
              transform="rotate(-35)"
              x={-rim * 0.22}
              y={-rim * 0.82}
              width={rim * 0.44}
              height={rim * 0.38}
              rx="4"
              fill={ref("calliper")}
            />
            {/* Five spokes */}
            {[0, 1, 2, 3, 4].map((i) => (
              <path
                key={i}
                transform={`rotate(${i * 72 + 18})`}
                d={`M${n(-rim * 0.1)} ${n(-rim * 0.18)}L${n(-rim * 0.2)} ${n(-rim * 0.95)}L${n(rim * 0.2)} ${n(-rim * 0.95)}L${n(rim * 0.1)} ${n(-rim * 0.18)}Z`}
                fill={ref("spoke")}
              />
            ))}
            {/* Rim lip and hub */}
            <circle r={rim} fill="none" stroke="rgb(190,198,214)" strokeOpacity={160 / 255} strokeWidth="1.6" />
            <circle r={rim * 0.2} fill={ref("hub")} />
          </g>
        ))}

        {/* Body */}
        <g className="vd-body">
          <path d={geo.body} fill={ref("lacquer")} />
          <g clipPath={ref("body")}>
            {/* Dark lower band (sills, bumpers) and the shoulder highlight */}
            <rect x="0" y={s.sill - 12} width={FIELD_W} height="80" fill="rgb(14,16,22)" fillOpacity={alpha * 0.85} />
            <rect x="0" y={s.belt + 6} width={FIELD_W} height="16" fill={ref("shine")} />
          </g>
          {/* Windows, with a diagonal reflection on the glass */}
          <g mask={ref("pillars")}>
            <path d={geo.glass} fill={ref("tint")} />
            {!anatomy ? (
              <rect x="0" y="0" width={FIELD_W} height={FIELD_H} fill={ref("refl")} clipPath={ref("glassclip")} />
            ) : null}
          </g>
          {/* Door lines and handles */}
          {geo.doorLines.map((c, i) => (
            <path
              key={i}
              d={`M${n(c.x0)} ${n(c.y0)}C${n(c.x1)} ${n(c.y1)} ${n(c.x2)} ${n(c.y2)} ${n(c.x3)} ${n(c.y3)}`}
              fill="none"
              stroke="#000"
              strokeOpacity={alpha * 0.55}
              strokeWidth="1.4"
            />
          ))}
          {geo.handles.map((x, i) => (
            <rect key={i} x={x} y={s.belt + 18} width="22" height="5" rx="2.5" fill="rgb(10,12,16)" fillOpacity={alpha * 0.7} />
          ))}
          {/* Head light with a glow */}
          <ellipse cx={s.front + 20} cy={s.nose + 2} rx="70" ry="40" fill={ref("headglow")} />
          <path
            d={`M${n(s.front + 10)} ${n(s.nose + 6)}C${n(s.front + 26)} ${n(s.nose - 6)} ${n(s.front + 60)} ${n(s.nose - 8)} ${n(s.front + 72)} ${n(s.nose - 6)}L${n(s.front + 64)} ${n(s.nose + 8)}C${n(s.front + 44)} ${n(s.nose + 12)} ${n(s.front + 22)} ${n(s.nose + 14)} ${n(s.front + 10)} ${n(s.nose + 6)}Z`}
            fill={ref("headlens")}
          />
          {/* Tail light */}
          <ellipse cx={s.rear - 4} cy={s.tail + 14} rx="50" ry="26" fill={ref("tailglow")} />
          <rect x={s.rear - 34} y={s.tail + 6} width="32" height="15" rx="5" fill={ref("taillens")} />
          {/* Rim light along the top edge */}
          <path d={geo.body} fill="none" stroke={ref("rimlight")} strokeWidth={anatomy ? 1.6 : 1.2} />
        </g>

        {/* The parts behind the faults */}
        {anatomy ? (
          <g className="vd-parts">
            {geo.parts.map((part) => {
              const isHit = hit.has(part.zone);
              const focused = part.zone === focus;
              const hovered = part.zone === hover;
              const tone = isHit ? "var(--bad)" : "var(--accent)";
              const fillAlpha = isHit ? 0.28 : hovered ? 0.22 : 0.08;
              const lineAlpha = isHit ? 1 : hovered ? 0.95 : 0.45;
              const lineWidth = isHit ? 1.8 : 1.2;
              const pulsing = isHit && focused;
              return (
                <g key={part.zone} className={`vd-part${pulsing ? " pulsing" : ""}`}>
                  {pulsing ? (
                    // A soft glow around the part in focus.
                    <path
                      className="vd-halo"
                      d={part.d}
                      style={{ fill: "none", stroke: "var(--bad)", "--halo-extra": `${part.line ?? 0}px` } as CSSProperties}
                      strokeLinejoin="round"
                      strokeLinecap="round"
                    />
                  ) : null}
                  {part.line ? (
                    <>
                      <path
                        className="vd-tubefill"
                        d={part.d}
                        style={{ fill: "none", stroke: tone, strokeOpacity: fillAlpha, strokeWidth: part.line }}
                      />
                      <path
                        className="vd-line"
                        d={part.d}
                        mask={ref(`tube-${part.zone}`)}
                        style={{ fill: "none", stroke: tone, strokeOpacity: lineAlpha, strokeWidth: part.line + lineWidth }}
                      />
                    </>
                  ) : (
                    <path
                      className="vd-shape"
                      d={part.d}
                      style={{ fill: tone, fillOpacity: fillAlpha, stroke: tone, strokeOpacity: lineAlpha, strokeWidth: lineWidth }}
                    />
                  )}
                </g>
              );
            })}
          </g>
        ) : null}

        {/* A light band that runs over the car; bright while reading. */}
        <g clipPath={ref("body")}>
          <rect key={scanning ? "scan" : "idle"} className="vd-sweep" style={sweepStyle} x="-60" y="0" width="120" height={FIELD_H} fill={ref("sweep")} />
        </g>

        {/* Hit areas: the part itself or 5 units around it, small parts on top. */}
        {anatomy ? (
          <g className="vd-hits">
            {hitOrder.map((part) => (
              <path
                key={part.zone}
                d={part.d}
                className="vd-hit"
                fill={part.line ? "none" : "transparent"}
                stroke="transparent"
                strokeWidth={(part.line ?? 0) + 10}
                strokeLinejoin="round"
                onPointerEnter={() => setHover(part.zone)}
                onPointerLeave={() => setHover((h) => (h === part.zone ? CarZone.Unknown : h))}
                onClick={() => onZoneClick?.(part.zone)}
              />
            ))}
          </g>
        ) : null}
      </svg>

      {chips.length > 0 ? (
        <svg className="vd-labels" width={size.w} height={size.h} aria-hidden>
          {chips.map((c) => {
            const focused = c.part.zone === focus;
            const tone = c.hit ? "var(--bad)" : "var(--accent)";
            const chipY = 6;
            const chipH = 26;
            const fx = c.x + c.w / 2;
            const fy = chipY + chipH;
            const tx = mapX(c.part.anchor.x);
            const ty = mapY(c.part.anchor.y);
            return (
              <g key={c.part.zone} className="vd-chip fade-in">
                <path
                  d={`M${n(fx)} ${n(fy)}C${n(fx)} ${n(fy + 40)} ${n(tx)} ${n(ty - 60)} ${n(tx)} ${n(ty)}`}
                  style={{ fill: "none", stroke: tone, strokeOpacity: focused ? 0.9 : 0.55, strokeWidth: 1.1 }}
                />
                <circle cx={tx} cy={ty} r="3.2" style={{ fill: tone }} />
                <rect
                  x={c.x + 0.5}
                  y={chipY + 0.5}
                  width={c.w - 1}
                  height={chipH - 1}
                  rx="13"
                  style={{
                    fill: "var(--surface)",
                    fillOpacity: 240 / 255,
                    stroke: tone,
                    strokeOpacity: focused ? 1 : 0.6,
                    strokeWidth: focused ? 1.6 : 1,
                  }}
                />
                <circle cx={c.x + 12} cy={chipY + chipH / 2} r="3.5" style={{ fill: tone }} />
                <text x={c.x + 20} y={chipY + chipH / 2} dominantBaseline="central" className="vd-chip-text">
                  {c.text}
                </text>
              </g>
            );
          })}
        </svg>
      ) : null}
    </div>
  );
}
