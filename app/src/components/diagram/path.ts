// Path helpers of the side view: an SVG path builder that also tracks the
// horizontal extent, and points on cubic curves (QPainterPath stand-ins).

// Point on a straight line between a and b at height y.
export function xAtY(ax: number, ay: number, bx: number, by: number, y: number): number {
  if (Math.abs(ay - by) < 1e-9) return ax;
  return ax + ((bx - ax) * (y - ay)) / (by - ay);
}

export const n = (v: number) => (Math.round(v * 100) / 100).toString();

// Collects an SVG path and its horizontal extent (for the "small part"
// rule of the hit test).
export class PathBuilder {
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

export interface Cubic {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  x3: number;
  y3: number;
}

export function cubicAt(c: Cubic, t: number): { x: number; y: number } {
  const u = 1 - t;
  return {
    x: u * u * u * c.x0 + 3 * u * u * t * c.x1 + 3 * u * t * t * c.x2 + t * t * t * c.x3,
    y: u * u * u * c.y0 + 3 * u * u * t * c.y1 + 3 * u * t * t * c.y2 + t * t * t * c.y3,
  };
}

// QPainterPath::pointAtPercent measures along the length of the curve, not
// along its parameter; sampled finely enough for a door handle.
export function pointAtPercent(c: Cubic, percent: number): { x: number; y: number } {
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
