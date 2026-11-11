// The procedural car of the X-ray view: every catalog generation (with its
// Wikidata dimensions and every body variant the shape table knows) gives a
// car whose wheels stand on the road under positive overhangs, whose cabin
// sits above the floor, whose parts and control units sit inside the body,
// and every kind of control unit has a place.
import { describe, expect, it } from "vitest";
import catalog from "../api/mock/data/catalog.json";
import { anatomyOf } from "../api/mock/areas/xray";
import type { MeasuredRecord } from "../api/mock/areas/knowledge";
import { BodyType, CarZone, Powertrain, type VehicleAnatomy } from "../api/types";
import { SHAPES, bodyVariants, shapeFor } from "./shape";
import {
  ECU_KIND_COUNT,
  ECU_PLACES,
  EcuKind,
  guessKind,
  buildLayout,
  insideHull,
  zoneAnchor,
  type CarLayout,
  type EcuInput,
  type Vec3,
} from "./layout";

const ALL_KINDS: EcuInput[] = Array.from({ length: ECU_KIND_COUNT }, (_, kind) => ({
  key: `k${kind}`,
  kind,
  name: `unit ${kind}`,
  faulty: kind === 2,
  source: "uds" as const,
}));

function checkCar(layout: CarLayout, label: string): void {
  const { hull } = layout;
  const p = hull.profile;
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
    for (const q of t.points) expect(insideHull(hull, q, -0.02), `${label}: tube ${t.id}`).toBe(true);
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
            checkCar(layout, `${BodyType[body]} ${brand} ${model} ${year} ${v}`);
          }
        }
      }
    }
  });

  it("gives every catalog generation a valid car, in every body variant", () => {
    const data = catalog as unknown as {
      generations: [string, string, string, number, number, number, number][];
      measured: Record<string, MeasuredRecord>;
    };
    expect(data.generations.length).toBeGreaterThan(500);
    let measured = 0;
    data.generations.forEach((g, i) => {
      const [brand, model, generation, from, to, body, powertrains] = g;
      for (const year of [from, to || 2026]) {
        const anatomy = anatomyOf(body as BodyType, brand, model, year, powertrains, 0, generation, data.measured[String(i)] ?? null);
        if (anatomy.dimensionSource !== "typical") measured++;
        for (const variant of bodyVariants(anatomy, body as BodyType)) {
          const layout = buildLayout({ anatomy, body: body as BodyType, variant, ecus: ALL_KINDS.slice(0, 8), kline: false, turbo: false });
          checkCar(layout, `${brand} ${model} ${generation} ${year} ${BodyType[variant]}`);
        }
      }
    });
    // About half of the catalog has its dimensions from Wikidata.
    expect(measured / 2).toBeGreaterThan(900);
    // Over 5000 cars: more than the default five seconds on a busy runner.
  }, 60_000);

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
