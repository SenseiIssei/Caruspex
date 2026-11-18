// The procedural car of the X-ray view: every catalog generation (with its
// Wikidata dimensions and every body variant the shape table knows) gives a
// car whose wheels stand on the road under positive overhangs at the
// wheelbase and track of the data, whose cabin sits above the floor, whose
// parts and control units sit inside the body and clear of the tyres, and
// every kind of control unit has a place.
import { describe, expect, it } from "vitest";
import catalog from "../api/mock/data/catalog.json";
import catalogRs from "../../../crates/caruspex-vehicles/src/generated/catalog.rs?raw";
import { anatomyOf } from "../api/mock/areas/xray";
import type { MeasuredRecord } from "../api/mock/areas/knowledge";
import { BodyType, CarZone, DrivenAxles, EngineLayout, Powertrain, type VehicleAnatomy } from "../api/types";
import { MOTO_ROWS, MOTO_STYLES, motoSpec } from "./moto";
import { SHAPES, bodyVariants, defaultShape, shapeFor } from "./shape";
import {
  ECU_KIND_COUNT,
  ECU_PLACES,
  EcuKind,
  guessKind,
  buildLayout,
  insideHull,
  lampOutlines,
  placeFor,
  SocketSpot,
  zoneAnchor,
  type CarLayout,
  type EcuInput,
  type Part,
  type Vec3,
  type Wheel,
} from "./layout";

// [brand, model, generation, from, to, body, powertrains, ..., location (12),
// ..., spot (18)] of the catalog copy.
type GenRow = [string, string, string, number, number, number, number, number, number, number, number, string, number, string, string, string, string, boolean, number, boolean];

const ALL_KINDS: EcuInput[] = Array.from({ length: ECU_KIND_COUNT }, (_, kind) => ({
  key: `k${kind}`,
  kind,
  name: `unit ${kind}`,
  faulty: kind === 2,
  source: "uds" as const,
}));

// The share of a part (sampled on a grid over its box, tilt ignored) that
// lies inside the tyre of a wheel.
function shareInWheel(p: Part, w: Wheel): number {
  let [sx, sy, sz] = p.size;
  if (p.shape === "cylinder" && p.axis === "y") [sx, sy] = [p.size[1], p.size[0]];
  if (p.shape === "cylinder" && p.axis === "z") [sx, sz] = [p.size[1], p.size[0]];
  const [x, y, z] = p.center;
  // Boxes that do not even touch the box round the wheel need no grid.
  if (Math.abs(x - w.center[0]) > sx / 2 + w.radius || Math.abs(z - w.center[2]) > sz / 2 + w.width / 2 || y - sy / 2 > 2 * w.radius) return 0;
  const n = 4;
  let inside = 0;
  for (let i = 0; i <= n; i++) {
    for (let j = 0; j <= n; j++) {
      for (let k = 0; k <= n; k++) {
        const qx = x + sx * (i / n - 0.5);
        const qy = y + sy * (j / n - 0.5);
        const qz = z + sz * (k / n - 0.5);
        if (Math.hypot(qx - w.center[0], qy - w.center[1]) < w.radius && Math.abs(qz - w.center[2]) < w.width / 2) inside++;
      }
    }
  }
  return inside / (n + 1) ** 3;
}

// Wheelbase and track as the data give them (the wheelbase capped at four
// fifths of the length, the track at most a fifth narrower where the body
// is rounder than the data's box), the pairs mirrored, and no part more
// than grazing a tyre.
function checkWheels(layout: CarLayout, label: string, anatomy?: VehicleAnatomy): void {
  const wheelbase = layout.frontAxle - layout.rearAxle;
  const moto = layout.vehicle === "moto";
  expect(wheelbase, `${label}: wheelbase`).toBeGreaterThan(moto ? 1.1 : 1.4);
  expect(wheelbase, `${label}: wheelbase`).toBeLessThan(moto ? 1.9 : 4.2);
  if (anatomy && !moto) expect(wheelbase, `${label}: wheelbase of the data`).toBeCloseTo(Math.min(anatomy.wheelbaseMm, anatomy.lengthMm * 0.8) / 1000, 3);
  if (!moto) {
    expect(layout.wheels.length, label).toBe(4);
    for (const [left, right] of [[0, 1], [2, 3]] as const) {
      const l = layout.wheels[left]!;
      const r = layout.wheels[right]!;
      expect(r.center[0], label).toBe(l.center[0]);
      expect(r.center[2], label).toBeCloseTo(-l.center[2], 9);
      const track = l.center[2] - r.center[2];
      expect(track, `${label}: track`).toBeGreaterThan(0.9);
      expect(track + l.width, `${label}: track`).toBeLessThan(layout.hull.width + 0.06);
      if (anatomy) expect(track, `${label}: track of the data`).toBeGreaterThan((anatomy.trackMm / 1000) * 0.8);
    }
  }
  const wrong: string[] = [];
  for (const w of layout.wheels) {
    for (const p of layout.parts) {
      const share = shareInWheel(p, w);
      if (share > 0.1) wrong.push(`${p.id} ${Math.round(share * 100)} % in the ${w.front ? "front" : "rear"} wheel`);
    }
  }
  expect(wrong.slice(0, 3), label).toEqual([]);
}

function checkCar(layout: CarLayout, label: string, anatomy?: VehicleAnatomy): void {
  const { hull } = layout;
  const p = hull.profile;
  checkWheels(layout, label, anatomy);
  // Overhangs: the wheels stay inside the length with room to spare.
  expect(layout.frontAxle, label).toBeLessThan(hull.front);
  expect(layout.rearAxle, label).toBeGreaterThan(hull.rear);
  expect(layout.frontAxle - layout.rearAxle, label).toBeGreaterThan(1.2);
  // A profile that runs from the rear to the front without gaps.
  for (let i = 1; i < p.top.length; i++) expect(p.top[i]![0], label).toBeGreaterThan(p.top[i - 1]![0]);
  expect(p.top[0]![0]).toBe(0);
  expect(p.top[p.top.length - 1]![0]).toBe(1);
  for (const [, y] of p.top) expect(y, label).toBeLessThanOrEqual(1 + 1e-9);
  // Rear window base, roof, windscreen, bonnet in this order.
  expect(p.tail, label).toBeLessThan(p.roofRear + 1e-9);
  expect(p.roofRear, label).toBeLessThan(p.roofFront);
  expect(p.roofFront, label).toBeLessThan(p.cowl);
  // The side window opening and its pillars.
  const [frontBottom, frontTop, rearBottom, rearTop] = p.dlo;
  expect(rearBottom, label).toBeLessThan(frontBottom);
  expect(rearTop, label).toBeLessThan(frontTop + 1e-9);
  for (const pillar of p.pillars) {
    expect(pillar, label).toBeGreaterThan(rearBottom);
    expect(pillar, label).toBeLessThan(frontBottom);
  }
  // The highest point is the height of the car.
  let highest = 0;
  for (let u = 0; u <= 1; u += 0.01) highest = Math.max(highest, hull.top(u));
  expect(highest, label).toBeCloseTo(hull.height, 1);
  // The cabin above the floor and under the roof.
  for (const part of layout.parts.filter((q) => q.zones.includes(CarZone.Cabin))) {
    const u = hull.station(part.center[0]);
    expect(part.center[1] - part.size[1] / 2, `${label}: ${part.id} above the floor`).toBeGreaterThan(hull.bottom(u) - 0.02);
    expect(part.center[1], `${label}: ${part.id} under the roof`).toBeLessThan(hull.top(u));
  }
  for (const w of layout.wheels) {
    // On the road, under the body, not sticking out at the ends.
    expect(w.center[1] - w.radius, label).toBeCloseTo(0, 6);
    expect(w.center[0] + w.radius, label).toBeLessThan(hull.front);
    expect(w.center[0] - w.radius, label).toBeGreaterThan(hull.rear);
    expect(Math.abs(w.center[2]) + w.width / 2, label).toBeLessThan(hull.width / 2 + 0.03);
  }
  for (const p of layout.parts) {
    expect(insideHull(hull, p.center, 0.005), `${label}: ${p.id} at ${p.center.map((v) => v.toFixed(2))}`).toBe(true);
  }
  for (const t of layout.tubes) {
    for (const s of [t.points, ...(t.branches ?? [])]) for (const q of s) expect(insideHull(hull, q, -0.02), `${label}: tube ${t.id}`).toBe(true);
  }
  for (const e of layout.ecus) {
    expect(insideHull(hull, e.position, 0.03), `${label}: ecu ${e.kind} (${e.place})`).toBe(true);
  }
  for (const r of layout.routes) {
    expect(r.points.length, label).toBeGreaterThan(1);
    for (const q of r.points) expect(insideHull(hull, q, -0.02), `${label}: route to ${r.ecu}`).toBe(true);
    if (r.ecu) {
      const ecu = layout.ecus.find((e) => e.key === r.ecu)!;
      expect(r.points[r.points.length - 1]).toEqual(ecu.position);
    }
  }
  for (let zone = CarZone.Intake; zone <= CarZone.Cabin; zone++) {
    const a: Vec3 = zoneAnchor(layout, zone);
    expect(a.every(Number.isFinite), `${label}: zone ${zone}`).toBe(true);
  }
  checkNumbers(layout, label);
  checkSocket(layout, label);
  checkLamps(layout, label);
}

// No NaN or infinity anywhere in the model.
function checkNumbers(layout: CarLayout, label: string): void {
  const bad: string[] = [];
  const finite = (what: string, ...values: number[]) => {
    if (!values.every(Number.isFinite)) bad.push(what);
  };
  for (const p of layout.parts) finite(p.id, ...p.center, ...p.size, p.tilt ?? 0, ...(p.explode ?? []));
  for (const t of layout.tubes) for (const s of [t.points, ...(t.branches ?? [])]) for (const q of s) finite(t.id, ...q, t.radius);
  for (const w of layout.wheels) finite("wheel", ...w.center, w.radius, w.width);
  for (const e of layout.ecus) finite(e.key, ...e.position);
  for (const r of layout.routes) for (const q of r.points) finite(`route ${r.ecu}`, ...q);
  for (const [k, a] of Object.entries(layout.anchors)) finite(k, ...a);
  for (const b of layout.busLabels) finite(b.bus, ...b.anchor);
  finite("socket", ...layout.obd, ...layout.adapter.socket, ...layout.adapter.dir);
  for (const q of layout.adapter.cable ?? []) finite("adapter cable", ...q);
  expect(bad.slice(0, 3), label).toEqual([]);
}

// The diagnostic socket where the data says: in the engine bay in front of
// the bulkhead for the BMW and Mercedes round plugs, otherwise in the cabin
// behind it, under the seat of a motorcycle; the adapter in it.
function checkSocket(layout: CarLayout, label: string): void {
  const { hull } = layout;
  expect(insideHull(hull, layout.obd, 0.01), `${label}: socket inside`).toBe(true);
  expect(Math.hypot(...layout.adapter.dir), label).toBeGreaterThan(0.5);
  if (layout.vehicle === "moto") {
    const seat = layout.parts.find((p) => p.id === "seat")!;
    expect(layout.socketSpot, label).toBe(SocketSpot.UnderSeat);
    expect(layout.obd[1], `${label}: plug under the seat`).toBeLessThan(seat.center[1]);
    expect(Math.abs(layout.obd[0] - seat.center[0]), `${label}: plug under the seat`).toBeLessThan(seat.size[0] / 2 + 0.1);
    expect(layout.adapter.cable!.length, label).toBeGreaterThan(1);
    return;
  }
  const cowl = hull.xAt(hull.profile.cowl);
  if (layout.socketSpot === SocketSpot.EngineBay) expect(layout.obd[0], `${label}: socket in the engine bay`).toBeGreaterThan(cowl);
  else {
    expect(layout.obd[0], `${label}: socket in the cabin`).toBeLessThan(cowl);
    expect(layout.obd[1], `${label}: socket under the window line`).toBeLessThan(hull.beltAt(hull.station(layout.obd[0])));
  }
  expect(layout.adapter.socket).toEqual(layout.obd);
}

// A motorcycle: both wheels on the road in line, the tyres inside the
// length, the engine between the wheels, the parts inside the envelope,
// the fork from the front axle up to the steering head.
function checkMoto(layout: CarLayout, label: string): void {
  const { hull } = layout;
  expect(layout.vehicle, label).toBe("moto");
  checkWheels(layout, label);
  expect(layout.wheels.length, label).toBe(2);
  for (const w of layout.wheels) {
    expect(w.center[1] - w.radius, label).toBeCloseTo(0, 6);
    expect(w.center[2], label).toBe(0);
    expect(w.center[0] + w.radius, label).toBeLessThan(hull.front);
    expect(w.center[0] - w.radius, label).toBeGreaterThan(hull.rear);
    expect(w.moto?.discs.length, label).toBeGreaterThan(0);
  }
  const [front, rear] = layout.wheels;
  expect(front!.center[0] - rear!.center[0], label).toBeGreaterThan(1.1);
  // The tyres do not touch.
  expect(front!.center[0] - front!.radius, label).toBeGreaterThan(rear!.center[0] + rear!.radius);
  const engine = layout.parts.find((p) => p.id === "engine")!;
  expect(engine.center[0], label).toBeGreaterThan(rear!.center[0]);
  expect(engine.center[0], label).toBeLessThan(front!.center[0]);
  expect(engine.center[1] - engine.size[1] / 2, `${label}: ground clearance`).toBeGreaterThan(0.1);
  for (const id of ["frame", "handlebar", "exhaust"]) expect(layout.tubes.some((t) => t.id === id), `${label}: ${id}`).toBe(true);
  for (const id of ["fuel-tank", "seat", "headlight", "tail-light", "battery12", "fork-1", "fork--1"]) expect(layout.parts.some((p) => p.id === id), `${label}: ${id}`).toBe(true);
  for (const p of layout.parts) {
    expect(insideHull(hull, p.center, 0.005), `${label}: ${p.id} at ${p.center.map((v) => v.toFixed(2))}`).toBe(true);
  }
  for (const t of layout.tubes) {
    for (const s of [t.points, ...(t.branches ?? [])]) for (const q of s) expect(insideHull(hull, q, -0.02), `${label}: tube ${t.id}`).toBe(true);
  }
  for (const e of layout.ecus) expect(insideHull(hull, e.position, 0.02), `${label}: ecu ${e.kind}`).toBe(true);
  for (const r of layout.routes) {
    const ecu = layout.ecus.find((e) => e.key === r.ecu);
    if (ecu) expect(r.points[r.points.length - 1]).toEqual(ecu.position);
  }
  // The exploded view moves the engine out of the frame.
  expect(Math.hypot(...(engine.explode ?? [0, 0, 0])), label).toBeGreaterThan(0.2);
  for (let zone = CarZone.Intake; zone <= CarZone.Cabin; zone++) expect(zoneAnchor(layout, zone).every(Number.isFinite), `${label}: zone ${zone}`).toBe(true);
  checkNumbers(layout, label);
  checkSocket(layout, label);
}

// Headlights on the front face ahead of the front wheels, tail lights on the
// rear face behind the rear wheels, both on the surface of the body, one on
// each side; the grille between the headlights.
function checkLamps(layout: CarLayout, label: string): void {
  const { hull } = layout;
  const lamps = lampOutlines(hull);
  // Thousands of points per car: collect what is wrong, one expect at the end.
  const wrong: string[] = [];
  const onBody = (q: Vec3, what: string) => {
    const u = hull.station(q[0]);
    if (!q.every(Number.isFinite)) wrong.push(`${what} not finite`);
    else if (u < -0.01 || u > 1.01) wrong.push(`${what} off the ends at ${u.toFixed(3)}`);
    else if (q[1] < hull.bottom(Math.min(1, Math.max(0, u))) - 0.01) wrong.push(`${what} under the body`);
    else if (Math.abs(q[2]) > hull.width / 2 + 0.02) wrong.push(`${what} wider than the car`);
  };
  for (const l of lamps.head) {
    for (const q of l) {
      onBody(q, "headlight");
      if (q[0] <= layout.frontAxle) wrong.push("headlight behind the front axle");
    }
  }
  for (const l of lamps.tail) {
    for (const q of l) {
      onBody(q, "tail light");
      if (q[0] >= layout.rearAxle) wrong.push("tail light ahead of the rear axle");
    }
  }
  for (const l of lamps.grille) for (const q of l) onBody(q, "grille");
  const sides = (loops: Vec3[][]) => new Set(loops.map((l) => Math.sign(l[0]![2]))).size;
  if (sides(lamps.head) !== 2) wrong.push("headlights not on both sides");
  if (sides(lamps.tail) !== 2) wrong.push("tail lights not on both sides");
  expect(wrong.slice(0, 3), label).toEqual([]);
}

describe("x-ray layout", () => {
  it("has a place for every kind of control unit", () => {
    for (let kind = 0; kind < ECU_KIND_COUNT; kind++) expect(ECU_PLACES[kind], `kind ${kind}`).toBeTruthy();
  });

  it("recognises units of the generic sweep by their own name", () => {
    expect(guessKind(EcuKind.Other, "INSTRUMENTS")).toBe(EcuKind.Instruments);
    expect(guessKind(EcuKind.Other, "AIRBAG 8CH")).toBe(EcuKind.Airbag);
    expect(guessKind(EcuKind.Other, "ABS/ESP 9.3")).toBe(EcuKind.Brakes);
    expect(guessKind(EcuKind.Other, "CLIMATE 2Z")).toBe(EcuKind.Climate);
    expect(guessKind(EcuKind.Other, "XYZ")).toBe(EcuKind.Other);
    expect(guessKind(EcuKind.Engine, "AIRBAG")).toBe(EcuKind.Engine);
  });

  it("builds the demo car: an estate with a 1.4 petrol engine", () => {
    const anatomy = anatomyOf(BodyType.Estate, "", "", 2021, Powertrain.Petrol, Powertrain.Petrol);
    const layout = buildLayout({ anatomy, body: BodyType.Estate, ecus: ALL_KINDS, kline: false, turbo: false });
    checkCar(layout, "demo");
    expect(layout.parts.some((p) => p.id === "engine")).toBe(true);
    expect(layout.parts.some((p) => p.id === "fuel-tank")).toBe(true);
    expect(layout.parts.some((p) => p.id === "hv-battery")).toBe(false);
    // Engine in front of the cabin, tank in front of the rear axle.
    const engine = layout.parts.find((p) => p.id === "engine")!;
    const tank = layout.parts.find((p) => p.id === "fuel-tank")!;
    expect(engine.center[0]).toBeGreaterThan(layout.frontAxle - 0.2);
    expect(tank.center[0]).toBeGreaterThan(layout.rearAxle);
    // The gateway is the hub; everything else hangs off it.
    expect(layout.routes.length).toBe(ALL_KINDS.length);
  });

  it("builds every body type with every drivetrain", () => {
    const variants = [Powertrain.Petrol, Powertrain.Diesel, Powertrain.Hybrid, Powertrain.PlugIn, Powertrain.Electric];
    const brands: [string, string][] = [["", ""], ["BMW", "3er"], ["Porsche", "911"], ["Porsche", "Boxster"], ["Tesla", "Model S"]];
    for (let body = BodyType.Hatchback; body <= BodyType.Roadster; body++) {
      for (const year of [1988, 2004, 2022]) {
        for (const [brand, model] of brands) {
          for (const v of variants) {
            const anatomy = anatomyOf(body, brand, model, year, v, v);
            const layout = buildLayout({ anatomy, body, ecus: ALL_KINDS, kline: year < 2000, turbo: v === Powertrain.Diesel });
            checkCar(layout, `${BodyType[body]} ${brand} ${model} ${year} ${v}`, anatomy);
          }
        }
      }
    }
    // Over 800 cars: more than five seconds on a busy machine.
  }, 30_000);

  it("gives every catalog generation a valid car, in every body variant", () => {
    const data = catalog as unknown as {
      generations: GenRow[];
      measured: Record<string, MeasuredRecord>;
    };
    // The copy (app/scripts/extract-catalog.py) holds every generation of
    // the Rust table: its ids are the indices. It missed the 23 trucks once,
    // and every id after the first truck pointed at the wrong car.
    expect(data.generations.length).toBe(Number(/(\d+) model generations/.exec(catalogRs)![1]));
    let measured = 0;
    let motorcycles = 0;
    data.generations.forEach((g, i) => {
      const [brand, model, generation, from, to, body, powertrains] = g;
      const socket = { location: g[12], spot: g[18] };
      for (const year of [from, to || 2026]) {
        const anatomy = anatomyOf(body as BodyType, brand, model, year, powertrains, 0, generation, data.measured[String(i)] ?? null, socket);
        if (anatomy.dimensionSource !== "typical") measured++;
        if (body === BodyType.Motorcycle) {
          // The kind the table gives, and every other kind the view offers.
          motorcycles++;
          checkMoto(buildLayout({ anatomy, body, ecus: ALL_KINDS.slice(0, 8), kline: false, turbo: false }), `${brand} ${model} ${generation} ${year}`);
          for (const motoStyle of MOTO_STYLES) {
            const layout = buildLayout({ anatomy, body, ecus: ALL_KINDS.slice(0, 6), kline: false, turbo: false, motoStyle });
            checkMoto(layout, `${brand} ${model} ${generation} ${year} ${motoStyle}`);
          }
          continue;
        }
        for (const variant of bodyVariants(anatomy, body as BodyType)) {
          const layout = buildLayout({ anatomy, body: body as BodyType, variant, ecus: ALL_KINDS.slice(0, 8), kline: false, turbo: false });
          checkCar(layout, `${brand} ${model} ${generation} ${year} ${BodyType[variant]}`, anatomy);
        }
      }
    });
    // About half of the catalog has its dimensions from Wikidata.
    expect(measured / 2).toBeGreaterThan(900);
    expect(motorcycles / 2).toBeGreaterThanOrEqual(35);
    // Over 5000 cars: more than the default five seconds on a busy runner.
  }, 60_000);

  it("knows the kind and engine of every motorcycle of the catalog", () => {
    const data = catalog as unknown as { generations: GenRow[] };
    const bikes = data.generations.filter((g) => g[5] === BodyType.Motorcycle);
    expect(bikes.length).toBeGreaterThanOrEqual(35);
    for (const [brand, model] of bikes) {
      expect(motoSpec(brand, model).source, `${brand} ${model}`).toBe("table");
    }
    for (const row of MOTO_ROWS) expect(bikes.some(([b, m]) => b === row.brand && m === row.model), `${row.brand} ${row.model}`).toBe(true);
    // The kinds follow the bikes: a GS with its boxer and shaft, a Ninja
    // with a fairing, a Sportster with its belt.
    expect(motoSpec("BMW", "R 1250 GS")).toMatchObject({ style: "enduro", engine: "boxer", drive: "shaft" });
    expect(motoSpec("Kawasaki", "Ninja 650").style).toBe("sport");
    expect(motoSpec("Harley-Davidson", "Sportster S")).toMatchObject({ style: "cruiser", drive: "belt" });
    expect(motoSpec("Ducati", "Scrambler").liquid).toBe(false);
    // Bikes outside the table by their name, a kind picked in the view.
    expect(motoSpec("Honda", "Forza 350").style).toBe("scooter");
    expect(motoSpec("Yamaha", "MT-07", "touring")).toMatchObject({ style: "touring", engine: "twin", source: "chosen" });
  });

  it("draws each kind of motorcycle with its own parts", () => {
    const anatomy = anatomyOf(BodyType.Motorcycle, "Yamaha", "MT-07", 2021, Powertrain.Petrol, 0, "II", null, { location: 10, spot: 12 });
    const ids = (motoStyle: (typeof MOTO_STYLES)[number]) => {
      const layout = buildLayout({ anatomy, body: BodyType.Motorcycle, ecus: ALL_KINDS, kline: false, turbo: false, motoStyle });
      checkMoto(layout, motoStyle);
      return new Set([...layout.parts.map((p) => p.id), ...layout.tubes.map((t) => t.id)]);
    };
    expect(ids("naked").has("fairing")).toBe(false);
    expect(ids("sport").has("windscreen")).toBe(true);
    expect(ids("touring").has("case-1")).toBe(true);
    expect(ids("enduro").has("bash-plate")).toBe(true);
    expect(ids("cruiser").has("pillion")).toBe(true);
    expect(ids("scooter").has("leg-shield")).toBe(true);
    expect(ids("naked").has("chain")).toBe(true);
    // A boxer's cylinders stand out to both sides.
    const gs = anatomyOf(BodyType.Motorcycle, "BMW", "R 1250 GS", 2020, Powertrain.Petrol, 0, "I");
    const layout = buildLayout({ anatomy: gs, body: BodyType.Motorcycle, ecus: [], kline: false, turbo: false });
    const heads = layout.parts.filter((p) => p.id.startsWith("cylinder-head"));
    expect(heads.map((h) => Math.sign(h.center[2])).sort()).toEqual([-1, 1]);
    expect(layout.parts.some((p) => p.id === "shaft")).toBe(true);
    // The wire-spoked front wheel of an enduro is larger than the rear one.
    expect(layout.wheels[0]!.radius).toBeGreaterThan(layout.wheels[1]!.radius);
  });

  it("puts the socket at the spot of the data", () => {
    const at = (location: number, spot: number) => {
      const anatomy = anatomyOf(BodyType.Hatchback, "Volkswagen", "Golf", 2015, Powertrain.Petrol, 0, "", null, { location, spot });
      const layout = buildLayout({ anatomy, body: BodyType.Hatchback, ecus: [], kline: false, turbo: false });
      checkSocket(layout, `${location}/${spot}`);
      return layout;
    };
    const left = at(1, SocketSpot.DashLeftOfColumn);
    const right = at(1, SocketSpot.DashRightOfColumn);
    const console = at(5, SocketSpot.ConsoleFront);
    const passenger = at(7, SocketSpot.Passenger);
    const bay = at(8, 0);
    // Left of the column further out than right of it; the console in the
    // middle; the passenger side across; the round plug in the engine bay.
    expect(left.obd[2]).toBeGreaterThan(right.obd[2]);
    expect(Math.abs(console.obd[2])).toBeLessThan(0.05);
    expect(passenger.obd[2]).toBeLessThan(0);
    expect(bay.socketSpot).toBe(SocketSpot.EngineBay);
    // Without any data: the most common spot.
    expect(at(0, 0).socketSpot).toBe(SocketSpot.DashLeftOfColumn);
    // The feed from the fuse box ends at the socket.
    const feed = left.tubes.find((t) => t.id === "obd-feed")!;
    expect(feed.points[feed.points.length - 1]).toEqual(left.obd);
  });

  it("draws the main harness: battery, fuses, ground, engine harness, strand to the rear", () => {
    const anatomy = anatomyOf(BodyType.Sedan, "", "", 2015, Powertrain.Diesel, Powertrain.Diesel);
    const layout = buildLayout({ anatomy, body: BodyType.Sedan, ecus: ALL_KINDS, kline: false, turbo: true });
    const ids = new Set([...layout.parts.map((p) => p.id), ...layout.tubes.map((t) => t.id)]);
    for (const id of ["battery12", "main-fuses", "ground", "ground-point", "fuse-box", "starter", "alternator", "engine-harness", "harness", "harness-front", "harness-rear", "obd-feed"]) {
      expect(ids.has(id), id).toBe(true);
    }
    const harness = layout.tubes.find((t) => t.id === "harness")!;
    // From the engine bay to the rear of the car.
    expect(harness.points[0]![0]).toBeGreaterThan(layout.hull.xAt(layout.hull.profile.cowl));
    expect(harness.points[harness.points.length - 1]![0]).toBeLessThan(layout.rearAxle);
    // An electric car has no engine harness, but its 12 V net all the same.
    const ev = anatomyOf(BodyType.Hatchback, "Volkswagen", "ID.3", 2021, Powertrain.Electric, Powertrain.Electric);
    const evLayout = buildLayout({ anatomy: ev, body: BodyType.Hatchback, ecus: [], kline: false, turbo: false });
    expect(evLayout.tubes.some((t) => t.id === "engine-harness")).toBe(false);
    expect(evLayout.parts.some((p) => p.id === "battery12")).toBe(true);
  });

  it("takes the drivetrain apart in the exploded view", () => {
    const anatomy = anatomyOf(BodyType.Estate, "", "", 2021, Powertrain.Petrol, Powertrain.Petrol);
    const layout = buildLayout({ anatomy, body: BodyType.Estate, ecus: [], kline: false, turbo: false });
    const off = (id: string) => layout.parts.find((p) => p.id === id)?.explode ?? layout.tubes.find((t) => t.id === id)?.explode;
    // The head lifts further than the block, the gearbox moves out, the
    // exhaust to the side; nothing outside the drivetrain moves.
    expect(off("cylinder-head")![1]).toBeGreaterThan(off("engine")![1]);
    expect(Math.hypot(...off("gearbox")!)).toBeGreaterThan(0.3);
    expect(off("exhaust")![2]).toBeLessThan(-0.3);
    expect(layout.parts.filter((p) => p.layer !== "drivetrain").every((p) => !p.explode)).toBe(true);
    expect(layout.parts.filter((p) => p.layer === "drivetrain").every((p) => p.explode)).toBe(true);
  });

  it("puts every bus the scan reports on its own line", () => {
    const anatomy = anatomyOf(BodyType.Sedan, "Ford", "Mondeo", 2012, Powertrain.Petrol, Powertrain.Petrol);
    const ecus: EcuInput[] = [
      { key: "gw", kind: EcuKind.Gateway, name: "Gateway", faulty: false, source: "uds" },
      { key: "pcm", kind: EcuKind.Engine, name: "PCM", faulty: false, source: "uds", bus: "hs" },
      { key: "radio", kind: EcuKind.Infotainment, name: "SYNC (APIM)", faulty: false, source: "uds" },
      { key: "ipc", kind: EcuKind.Instruments, name: "IPC", faulty: false, source: "uds" },
      { key: "bcm", kind: EcuKind.Body, name: "BCM", faulty: false, source: "uds", bus: "ms" },
      { key: "gm", kind: EcuKind.Body, name: "BCM", faulty: false, source: "uds", bus: "sw" },
    ];
    const layout = buildLayout({ anatomy, body: BodyType.Sedan, ecus, kline: false, turbo: false });
    const bus = (key: string) => layout.ecus.find((e) => e.key === key)!.bus;
    expect([bus("gw"), bus("pcm"), bus("radio"), bus("ipc"), bus("bcm"), bus("gm")]).toEqual(["diagnostic", "powertrain", "infotainment", "comfort", "mscan", "swcan"]);
    // The MS-CAN and the SW-CAN start at the socket, the others behind the gateway.
    const from = (key: string) => layout.routes.find((r) => r.ecu === key)!.points[0];
    expect(from("bcm")).toEqual(layout.obd);
    expect(from("gm")).toEqual(layout.obd);
    expect(from("radio")).toEqual(layout.hub);
    // One name per bus on its line.
    expect(new Set(layout.busLabels.map((b) => b.bus))).toEqual(new Set(["diagnostic", "powertrain", "infotainment", "comfort", "mscan", "swcan"]));
  });

  it("places units by their name and by the maker where the data says so", () => {
    expect(placeFor(EcuKind.Body, "Heckelektronik (REM)").place).toBe("boot");
    expect(placeFor(EcuKind.Body, "Roof function centre (FZD)").place).toBe("roof");
    expect(placeFor(EcuKind.Other, "Camera driver assistance (KAFAS)").place).toBe("windscreen");
    expect(placeFor(EcuKind.Doors, "Passenger door (PDM)").place).toBe("passengerDoor");
    expect(placeFor(EcuKind.Seats, "Seat module passenger (SM)").place).toBe("passengerSeat");
    expect(placeFor(EcuKind.Lighting, "Light module 2 (LHM)").mirror).toBe(true);
    // BMW's gateway is part of the front electronics module.
    expect(placeFor(EcuKind.Gateway, "Gateway (FEM/ZGM)", "BMW").place).toBe("footwell");
    expect(placeFor(EcuKind.Gateway, "Gateway", "Volkswagen").place).toBe("glovebox");
  });

  it("names a catalog generation for every hand-tuned shape", () => {
    const data = catalog as unknown as { generations: [string, string, string][] };
    for (const row of SHAPES) {
      for (const code of row.generations) {
        expect(data.generations.some(([b, m, g]) => b === row.brand && m === row.model && g === code), `${row.brand} ${row.model} ${code}`).toBe(true);
      }
      expect(row.note.length, `${row.brand} ${row.model}`).toBeGreaterThan(10);
    }
  });
});

describe("hand-tuned shapes", () => {
  const data = catalog as unknown as {
    generations: [string, string, string, number, number, number, number][];
    measured: Record<string, MeasuredRecord>;
  };
  const anatomyFor = (brand: string, model: string, generation: string): [VehicleAnatomy, BodyType] => {
    const i = data.generations.findIndex(([b, m, g]) => b === brand && m === model && g === generation);
    const [, , , from, , body, powertrains] = data.generations[i]!;
    return [anatomyOf(body as BodyType, brand, model, from, powertrains, 0, generation, data.measured[String(i)] ?? null), body as BodyType];
  };

  it("draws the BMW 4 Series Gran Coupe (F36) as a four-door fastback", () => {
    const [anatomy, body] = anatomyFor("BMW", "4er Gran Coupé", "F36");
    expect(anatomy.dimensionSource).toBe("manufacturer");
    expect(anatomy.lengthMm).toBe(4638);
    // 1.39 m high after the press kit (Wikidata only has the series values).
    expect(anatomy.heightMm).toBe(1389);
    expect(anatomy.dimensionDocument).toContain("2014");
    const shape = shapeFor(anatomy, body);
    expect(shape.source).toBe("profile");
    expect(shape.style).toBe("fastback");
    expect(shape.doors).toBe(4);
    expect(shape.frameless).toBe(true);
    const layout = buildLayout({ anatomy, body, ecus: [], kline: false, turbo: false });
    const p = layout.hull.profile;
    // One long rear window: from the roof far down to a short boot lid.
    expect((p.roofRear - p.tail) * layout.hull.length).toBeGreaterThan(0.9);
    expect((p.tail * layout.hull.length)).toBeLessThan(0.4);
    // A B-pillar between the two doors, thin without frames, and the small
    // window behind the rear door.
    expect(p.pillars.length).toBe(2);
    expect(Math.min(...p.pillarWidths) * layout.hull.length).toBeLessThan(0.04);
    expect(p.kink).toBeGreaterThan(0);
    // Engine along the car: long bonnet, short front overhang.
    expect(layout.hull.front - layout.frontAxle).toBeLessThan(0.85);
    expect(layout.frontAxle - layout.hull.xAt(p.cowl)).toBeGreaterThan(0.6);
    // A notchback F30 has its rear window ending well before the tail.
    const [f30, f30Body] = anatomyFor("BMW", "3er", "F30");
    const saloon = buildLayout({ anatomy: f30, body: f30Body, ecus: [], kline: false, turbo: false }).hull.profile;
    expect(saloon.tail * f30.lengthMm / 1000).toBeGreaterThan(0.4);
  });

  it("knows the body variants of a generation", () => {
    const [e, body] = anatomyFor("Mercedes-Benz", "E-Klasse", "W213");
    expect(bodyVariants(e, body)).toEqual([BodyType.Sedan, BodyType.Estate, BodyType.Coupe, BodyType.Convertible]);
    const estate = shapeFor(e, body, BodyType.Estate);
    expect(estate.style).toBe("estate");
    expect(shapeFor(e, body, BodyType.Coupe).doors).toBe(2);
    const [golf, golfBody] = anatomyFor("Volkswagen", "Golf", "VII");
    expect(bodyVariants(golf, golfBody)).toEqual([BodyType.Hatchback]);
    expect(shapeFor(golf, golfBody).style).toBe("hatchback");
    const [fiat, fiatBody] = anatomyFor("Fiat", "500", "I");
    expect(shapeFor(fiat, fiatBody).doors).toBe(2);
  });

  it("falls back to the defaults of the body without a row", () => {
    const [a, body] = anatomyFor("Lada", "Granta", "I");
    const shape = shapeFor(a, body);
    expect(shape.source).toBe("typical");
    expect(shape.style).toBe("notchback");
  });
});

describe("lamps and grille", () => {
  const data = catalog as unknown as {
    generations: [string, string, string, number, number, number, number][];
    measured: Record<string, MeasuredRecord>;
  };
  const carFor = (brand: string, model: string, generation: string) => {
    const i = data.generations.findIndex(([b, m, g]) => b === brand && m === model && g === generation);
    const [, , , from, , body, powertrains] = data.generations[i]!;
    const anatomy = anatomyOf(body as BodyType, brand, model, from, powertrains, 0, generation, data.measured[String(i)] ?? null);
    const layout = buildLayout({ anatomy, body: body as BodyType, ecus: [], kline: false, turbo: false });
    return { anatomy, body: body as BodyType, shape: shapeFor(anatomy, body as BodyType), layout };
  };

  it("follow the era: round lamps on old cars, slim ones that wrap round on new ones", () => {
    const old = defaultShape(BodyType.Sedan, 1972, EngineLayout.FrontLongitudinal, DrivenAxles.Rear);
    const now = defaultShape(BodyType.Sedan, 2022, EngineLayout.FrontTransverse, DrivenAxles.Front);
    expect(old.headlight.round).toBeLessThan(3);
    expect(old.headlight.wrap).toBeLessThan(0);
    expect(now.headlight.height).toBeLessThan(old.headlight.height);
    expect(now.headlight.wrap).toBeGreaterThan(0.1);
    expect(now.taillight.wrap).toBeGreaterThan(old.taillight.wrap);
    // An electric car has no radiator to feed: the grille is closed.
    expect(defaultShape(BodyType.Hatchback, 2022, EngineLayout.None, DrivenAxles.Rear).grille.openings).toBe(0);
    expect(now.grille.openings).toBe(1);
  });

  it("follow the hand-tuned rows", () => {
    // Two grille openings side by side on the BMW (and the intake below),
    // none on the Tesla, round headlights on the Fiat 500.
    const bmw = carFor("BMW", "3er", "F30");
    expect(bmw.shape.grille.openings).toBe(2);
    expect(lampOutlines(bmw.layout.hull).grille.length).toBe(3);
    const tesla = carFor("Tesla", "Model 3", "I");
    expect(tesla.shape.grille.openings).toBe(0);
    expect(lampOutlines(tesla.layout.hull).grille.length).toBe(1);
    expect(carFor("Fiat", "500", "I").shape.headlight.round).toBe(2);
    // The estate variant keeps the front of its saloon.
    expect(shapeFor(bmw.anatomy, bmw.body, BodyType.Estate).grille.openings).toBe(2);
  });

  it("wrap modern lamps round the corner on to the side", () => {
    const { layout } = carFor("Volkswagen", "Golf", "VIII");
    const lamps = lampOutlines(layout.hull);
    const half = layout.hull.width / 2;
    // The outer end of a headlight sweeps back round the corner, the inner
    // end stays clear of the middle.
    for (const lamps_ of [lamps.head, lamps.tail]) {
      const across = lamps_.flat().map((q) => Math.abs(q[2]));
      const along = lamps_.flat().map((q) => q[0]);
      expect(Math.max(...across)).toBeGreaterThan(half * 0.6);
      expect(Math.min(...across)).toBeGreaterThan(0.04);
      expect(Math.max(...along) - Math.min(...along)).toBeGreaterThan(0.12);
    }
  });
});
