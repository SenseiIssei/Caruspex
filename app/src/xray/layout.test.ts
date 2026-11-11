// The procedural car of the X-ray view: every catalog generation gives a car
// whose wheels stand on the road, whose parts and control units sit inside
// the body, and every kind of control unit has a place.
import { describe, expect, it } from "vitest";
import catalog from "../api/mock/data/catalog.json";
import { anatomyOf } from "../api/mock/areas/xray";
import { BodyType, CarZone, Powertrain } from "../api/types";
import {
  ECU_KIND_COUNT,
  ECU_PLACES,
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

  it("gives every catalog generation a valid car", () => {
    const generations = (catalog as unknown as { generations: [string, string, string, number, number, number, number][] }).generations;
    expect(generations.length).toBeGreaterThan(500);
    for (const g of generations) {
      const [brand, model, generation, from, , body, powertrains] = g;
      const anatomy = anatomyOf(body as BodyType, brand, model, from, powertrains, 0);
      const layout = buildLayout({ anatomy, body: body as BodyType, ecus: ALL_KINDS.slice(0, 8), kline: false, turbo: false });
      checkCar(layout, `${brand} ${model} ${generation}`);
    }
  });
});
