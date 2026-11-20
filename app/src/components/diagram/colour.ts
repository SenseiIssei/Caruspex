// Colour helpers of the side view, the way QColor does them.
import type { Rgb } from "../../lib/theme";

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

export function lighter(c: Rgb, factor: number): Rgb {
  const hsv = toHsv(c);
  let v = (hsv.v * factor) / 100;
  let s = hsv.s;
  if (v > 255) {
    s = Math.max(0, s - (v - 255));
    v = 255;
  }
  return fromHsv(hsv.h, s, v);
}

export function darker(c: Rgb, factor: number): Rgb {
  const hsv = toHsv(c);
  return fromHsv(hsv.h, hsv.s, (hsv.v * 100) / factor);
}

export const rgb = ({ r, g, b }: Rgb) => `rgb(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)})`;
