// Fitting the owner's own model onto the car: turned along the car with
// its front forward, scaled to the real length, wheels on the road and over
// the axles of the layout.
import { describe, expect, it } from "vitest";
import { applyFit, contactAxles, externalFiles, fitShell, looksLikeGlass, orientation, turnY, type Bounds } from "./shell";
import type { Vec3 } from "./layout";

// A crude model the way glTF wants it: 4.2 units long along +Z (its front),
// 1.8 wide, 1.45 high, tyres touching the ground at z = +1.3 and -1.3.
function model(): { points: Vec3[]; size: Vec3 } {
  const points: Vec3[] = [];
  for (const x of [-0.9, 0.9]) for (const y of [0.3, 1.45]) for (const z of [-2.1, 2.1]) points.push([x, y, z]);
  for (const z of [1.3, -1.3]) for (const x of [-0.75, 0.75]) for (const dz of [-0.05, 0, 0.05]) points.push([x, 0, z + dz]);
  return { points, size: [1.8, 1.45, 4.2] };
}

function boundsOf(points: Vec3[]): Bounds {
  const min: Vec3 = [Infinity, Infinity, Infinity];
  const max: Vec3 = [-Infinity, -Infinity, -Infinity];
  for (const p of points) {
    for (let i = 0; i < 3; i++) {
      min[i] = Math.min(min[i]!, p[i]!);
      max[i] = Math.max(max[i]!, p[i]!);
    }
  }
  return { min, max };
}

// The Golf VII of the layout: 4.255 m long, wheelbase 2.637 m.
const car = { length: 4.255, wheelbase: 2.637, frontAxle: 1.264, rearAxle: 1.264 - 2.637 };

describe("own model", () => {
  it("turns a glTF model so its front (+Z) looks along the car (+x)", () => {
    const { size } = model();
    const angle = orientation(size, false);
    expect(angle).toBeCloseTo(Math.PI / 2, 9);
    const front = turnY([0, 0, 2.1], angle);
    expect(front[0]).toBeCloseTo(2.1, 9);
    expect(front[2]).toBeCloseTo(0, 9);
    // Turned around by the user: the front goes to -x.
    expect(turnY([0, 0, 2.1], orientation(size, true))[0]).toBeCloseTo(-2.1, 9);
    // A model already long in x keeps its direction.
    expect(orientation([4, 1.4, 1.8], false)).toBe(0);
  });

  it("finds the axles from the tyres' lowest points", () => {
    const { points, size } = model();
    const turned = points.map((p) => turnY(p, orientation(size, false)));
    const axles = contactAxles(turned, boundsOf(turned));
    expect(axles).not.toBeNull();
    expect(axles!.front).toBeCloseTo(1.3, 6);
    expect(axles!.rear).toBeCloseTo(-1.3, 6);
    // A box without wheels: no axles.
    const box: Vec3[] = [[-2, 0, -1], [2, 0, 1], [-2, 1.4, 1], [2, 1.4, -1], [0, 0, 0]];
    expect(contactAxles(box, boundsOf(box))).toBeNull();
  });

  it("scales to the real length, puts the wheels on the road and over the axles", () => {
    const { points, size } = model();
    const angle = orientation(size, false);
    const turned = points.map((p) => turnY(p, angle));
    const bounds = boundsOf(turned);
    const fit = fitShell(bounds, contactAxles(turned, bounds), car, angle);
    expect(fit.scale).toBeCloseTo(4.255 / 4.2, 9);
    // 2.6 model units of wheelbase are 2.634 m, close to 2.637: a tiny stretch.
    expect(fit.modelWheelbase).toBeCloseTo(2.6 * fit.scale, 9);
    expect(fit.stretchX).toBeCloseTo(2.637 / (2.6 * fit.scale), 9);
    const front = applyFit(fit, [1.3, 0, 0.75]);
    const rear = applyFit(fit, [-1.3, 0, 0.75]);
    expect(front[0]).toBeCloseTo(car.frontAxle, 6);
    expect(rear[0]).toBeCloseTo(car.rearAxle, 6);
    expect(front[1]).toBeCloseTo(0, 9);
    // Centred across the car, the length close to the real one.
    expect(applyFit(fit, [0, 0, 0])[2]).toBeCloseTo(0, 9);
    const ends = [applyFit(fit, [bounds.max[0], 0, 0])[0], applyFit(fit, [bounds.min[0], 0, 0])[0]];
    expect(ends[0]! - ends[1]!).toBeCloseTo(car.length * fit.stretchX, 6);
    // The roof is as high as the model says, scaled.
    expect(applyFit(fit, [0, 1.45, 0])[1]).toBeCloseTo(1.45 * fit.scale, 9);
  });

  it("keeps the proportions of a model of another car", () => {
    const { points, size } = model();
    const angle = orientation(size, false);
    const turned = points.map((p) => turnY(p, angle));
    const bounds = boundsOf(turned);
    // A car with a much longer wheelbase: no stretching, axles centred.
    const fit = fitShell(bounds, contactAxles(turned, bounds), { ...car, wheelbase: 3.2, rearAxle: car.frontAxle - 3.2 }, angle);
    expect(fit.stretchX).toBe(1);
    const middle = applyFit(fit, [0, 0, 0])[0];
    expect(middle).toBeCloseTo((car.frontAxle + car.frontAxle - 3.2) / 2, 9);
    // Without wheels: centred on its length.
    const plain = fitShell(bounds, null, car, angle);
    expect(plain.modelWheelbase).toBeNull();
    expect(applyFit(plain, [(bounds.min[0] + bounds.max[0]) / 2, bounds.min[1], 0])).toEqual([0, 0, 0]);
  });

  it("knows glass and the files a .gltf needs", () => {
    expect(looksLikeGlass("Windscreen_mat", false, 1)).toBe(true);
    expect(looksLikeGlass("body paint", true, 0.4)).toBe(true);
    expect(looksLikeGlass("body paint", false, 1)).toBe(false);
    expect(externalFiles({ buffers: [{ uri: "car.bin" }, { uri: "data:application/octet-stream;base64,AAAA" }], images: [{ uri: "tex%20a.png" }, {}] })).toEqual([
      "car.bin",
      "tex a.png",
    ]);
  });
});
