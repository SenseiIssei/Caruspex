// Light or dark, and the palette as numbers for drawings that mix colours
// (the health ring, the gauge, the car diagram). The values live in
// styles/theme.css; this reads them back so both stay in one place.
import { useSyncExternalStore } from "react";

export type Appearance = "auto" | "dark" | "light";

const media = typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: light)") : null;
let current: Appearance = "auto";
const subscribers = new Set<() => void>();
let version = 0;

export function effectiveLight(appearance: Appearance = current): boolean {
  if (appearance === "light") return true;
  if (appearance === "dark") return false;
  return media?.matches ?? false;
}

function apply(): void {
  document.documentElement.dataset.theme = effectiveLight() ? "light" : "dark";
  version++;
  for (const fn of subscribers) fn();
}

export function setAppearance(appearance: Appearance): void {
  current = appearance;
  apply();
}

media?.addEventListener("change", () => {
  if (current === "auto") apply();
});

export function isLight(): boolean {
  return document.documentElement.dataset.theme === "light";
}

// Re-renders the caller when the theme switches.
export function useThemeVersion(): number {
  return useSyncExternalStore(
    (fn) => {
      subscribers.add(fn);
      return () => subscribers.delete(fn);
    },
    () => version,
  );
}

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

export function parseColor(text: string): Rgb {
  const s = text.trim();
  if (s.startsWith("#")) {
    const hex = s.length === 4 ? s.slice(1).replace(/./g, (c) => c + c) : s.slice(1, 7);
    const n = Number.parseInt(hex, 16);
    return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
  }
  const m = s.match(/rgba?\(([^)]+)\)/);
  if (m) {
    const [r = 0, g = 0, b = 0] = m[1]!.split(",").map((x) => Number.parseFloat(x));
    return { r, g, b };
  }
  return { r: 128, g: 128, b: 128 };
}

export function toCss({ r, g, b }: Rgb, alpha = 1): string {
  return alpha >= 1
    ? `rgb(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)})`
    : `rgba(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)}, ${alpha})`;
}

// Mixes two colours, t = 0 gives a, t = 1 gives b.
export function mix(a: Rgb, b: Rgb, t: number): Rgb {
  const k = Math.max(0, Math.min(1, t));
  return { r: a.r + (b.r - a.r) * k, g: a.g + (b.g - a.g) * k, b: a.b + (b.b - a.b) * k };
}

export function lighter(c: Rgb, factor: number): Rgb {
  return mix(c, { r: 255, g: 255, b: 255 }, Math.max(0, factor - 1));
}

// A colour of the active palette, e.g. cssColor("--good").
export function cssColor(name: string): Rgb {
  if (typeof document === "undefined") return { r: 128, g: 128, b: 128 };
  return parseColor(getComputedStyle(document.documentElement).getPropertyValue(name) || "#808080");
}

// The colour for a 0..1 health value: red, amber, green (theme::healthColor).
export function healthColor(fraction: number): Rgb {
  const f = Math.max(0, Math.min(1, fraction));
  const bad = cssColor("--bad");
  const warn = cssColor("--warn");
  const good = cssColor("--good");
  return f < 0.5 ? mix(bad, warn, f * 2) : mix(warn, good, (f - 0.5) * 2);
}
