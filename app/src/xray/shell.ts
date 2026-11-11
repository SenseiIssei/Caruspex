// Fitting a 3D model the user owns (glTF or GLB) onto the car of the X-ray
// view: turned so its length runs along the car, scaled to the car's real
// length, moved so its wheels sit on the road and its axles over the axles
// of the layout, so the drivetrain, the control units and the lines stay
// inside. Pure numbers without three.js, so the tests can check them.
//
// glTF models face +Z (the specification's "front"), the car's front is +x
// here: a model longer in z than in x is turned a quarter round. A model
// that faces the other way is turned around by the user (one button), the
// choice is kept with the file.
import type { Vec3 } from "./layout";

export interface Bounds {
  min: Vec3;
  max: Vec3;
}

export interface CarFrame {
  // Metres, x of the axles in the layout (front is +x).
  length: number;
  wheelbase: number;
  frontAxle: number;
  rearAxle: number;
}

export interface ShellFit {
  // Turn about the vertical axis before anything else (radians).
  rotationY: number;
  // Uniform scale, model units to metres.
  scale: number;
  // Extra scale along the car so the wheelbase fits too (1 when the model's
  // wheelbase is far off: then it is another car, keep its proportions).
  stretchX: number;
  // Where the model's reference point lands, and that point in the turned
  // model: x the middle between its axles (or of its length), y its lowest
  // point, z its middle.
  pivot: Vec3;
  position: Vec3;
  // The model's own wheelbase in metres after scaling, null without wheels.
  modelWheelbase: number | null;
}

// The largest correction of the wheelbase by stretching along the car.
const MAX_STRETCH = 0.08;

// The turn that brings a model's length along x: a quarter turn when it is
// longer in z (glTF's forward axis), plus half a turn when the user turned
// it around.
export function orientation(size: Vec3, turned: boolean): number {
  const quarter = size[2] > size[0] ? Math.PI / 2 : 0;
  return quarter + (turned ? Math.PI : 0);
}

// A point turned about y by an angle (three.js' rotation.y).
export function turnY(p: Vec3, angle: number): Vec3 {
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  return [p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c];
}

// The axles of a turned model from its lowest points: the tyres touch the
// road, the body does not. Points within a few per cent of the height above
// the lowest one are split at the middle of the length; each half gives the
// x of an axle. null when one half has no such point.
export function contactAxles(points: Iterable<Vec3>, bounds: Bounds): { front: number; rear: number } | null {
  const height = bounds.max[1] - bounds.min[1];
  const limit = bounds.min[1] + Math.max(1e-6, height * 0.035);
  const middle = (bounds.min[0] + bounds.max[0]) / 2;
  const front: number[] = [];
  const rear: number[] = [];
  for (const p of points) {
    if (p[1] > limit) continue;
    (p[0] >= middle ? front : rear).push(p[0]);
  }
  if (front.length === 0 || rear.length === 0) return null;
  const length = bounds.max[0] - bounds.min[0];
  const mean = (xs: number[]) => xs.reduce((a, b) => a + b, 0) / xs.length;
  const spread = (xs: number[], m: number) => Math.sqrt(xs.reduce((a, x) => a + (x - m) * (x - m), 0) / xs.length);
  const f = mean(front);
  const r = mean(rear);
  // Tyres touch the road in small patches; a flat floor touches it all along.
  if (spread(front, f) > length * 0.06 || spread(rear, r) > length * 0.06) return null;
  // Both axles well apart and inside the length.
  if (f - r < length * 0.35) return null;
  return { front: f, rear: r };
}

// The fit of a turned model (bounds and axles in its turned units) onto the car.
export function fitShell(bounds: Bounds, axles: { front: number; rear: number } | null, car: CarFrame, rotationY = 0): ShellFit {
  const length = Math.max(1e-6, bounds.max[0] - bounds.min[0]);
  const scale = car.length / length;
  let stretchX = 1;
  let modelWheelbase: number | null = null;
  let pivotX = (bounds.min[0] + bounds.max[0]) / 2;
  let targetX = 0;
  if (axles) {
    modelWheelbase = (axles.front - axles.rear) * scale;
    const ratio = car.wheelbase / modelWheelbase;
    if (Math.abs(ratio - 1) <= MAX_STRETCH) stretchX = ratio;
    pivotX = (axles.front + axles.rear) / 2;
    targetX = (car.frontAxle + car.rearAxle) / 2;
  }
  const pivot: Vec3 = [pivotX, bounds.min[1], (bounds.min[2] + bounds.max[2]) / 2];
  return { rotationY, scale, stretchX, pivot, position: [targetX, 0, 0], modelWheelbase };
}

// Where a point of the turned model ends up in the car.
export function applyFit(fit: ShellFit, p: Vec3): Vec3 {
  return [
    fit.position[0] + (p[0] - fit.pivot[0]) * fit.scale * fit.stretchX,
    fit.position[1] + (p[1] - fit.pivot[1]) * fit.scale,
    fit.position[2] + (p[2] - fit.pivot[2]) * fit.scale,
  ];
}

// Names of meshes and materials that are glass in most models.
const GLASS = /glass|window|windscreen|windshield|scheibe|glas|vitre|vetro|cristal|okno|lamp|light|lens/i;

export function looksLikeGlass(name: string, transparent: boolean, opacity: number): boolean {
  return GLASS.test(name) || (transparent && opacity < 0.9);
}

// Wheels and tyres stay solid, as the generated ones do.
const WHEEL = /wheel|tyre|tire|rim|reifen|felge|\brad\b|rueda|roue|pneu|ruota/i;

export function looksLikeWheel(name: string): boolean {
  return WHEEL.test(name);
}

// The files a .gltf names next to itself (buffers and images that are not
// embedded), relative to its folder.
export function externalFiles(gltf: { buffers?: { uri?: string }[]; images?: { uri?: string }[] }): string[] {
  const out: string[] = [];
  for (const entry of [...(gltf.buffers ?? []), ...(gltf.images ?? [])]) {
    const uri = entry.uri;
    if (!uri || uri.startsWith("data:")) continue;
    const name = decodeURIComponent(uri);
    if (!out.includes(name)) out.push(name);
  }
  return out;
}
