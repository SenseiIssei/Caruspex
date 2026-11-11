// The live chart: many series over a sliding time window, drawn on a canvas.
//
// Every curve is drawn as a share of its own range (like the Qt chart), so a
// value in the thousands does not flatten everything else; the y axis reads
// "% of range". The range comes from the PID table when it has one,
// otherwise it starts around the first value and grows with a little
// headroom. The real values are kept for the hover readout.
//
// The data lives in a SeriesStore outside React: values arrive many times a
// second and only the canvas needs them. The canvas redraws in an animation
// frame, only while the page is on screen and only when something changed.
import { useCallback, useEffect, useRef, useState, useSyncExternalStore, type PointerEvent } from "react";
import { usePageActive } from "../../lib/hooks";
import { cssColor, toCss, useThemeVersion } from "../../lib/theme";
import "./charts.css";

export const SERIES_COLOURS = 8;

export function seriesColour(index: number): string {
  return `var(--series-${(index % SERIES_COLOURS) + 1})`;
}

export interface SeriesInfo {
  name: string;
  unit: string;
  // Range from the table, 0/0 when there is none.
  displayMin: number;
  displayMax: number;
}

export interface Track {
  key: number;
  name: string;
  unit: string;
  colour: number;
  low: number;
  high: number;
  // Range from the PID table, it never shrinks or grows from there unless a
  // reading leaves it.
  fixed: boolean;
  times: number[];
  values: number[];
  hidden: boolean;
}

export interface LegendEntry {
  key: number;
  name: string;
  unit: string;
  colour: number;
  low: number;
  high: number;
  hidden: boolean;
}

export class SeriesStore {
  readonly tracks = new Map<number, Track>();
  windowSeconds = 60;
  // Newest time seen, the right edge of the window.
  now = 0;
  dirty = true;
  private legend: LegendEntry[] = [];
  private listeners = new Set<() => void>();

  subscribe = (fn: () => void): (() => void) => {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  };

  legendSnapshot = (): LegendEntry[] => this.legend;

  private legendChanged(): void {
    this.legend = [...this.tracks.values()].map((t) => ({
      key: t.key,
      name: t.name,
      unit: t.unit,
      colour: t.colour,
      low: t.low,
      high: t.high,
      hidden: t.hidden,
    }));
    for (const fn of [...this.listeners]) fn();
  }

  clear(): void {
    this.tracks.clear();
    this.now = 0;
    this.dirty = true;
    this.legendChanged();
  }

  setWindow(seconds: number): void {
    this.windowSeconds = Math.max(5, seconds);
    this.dirty = true;
  }

  toggle(key: number): void {
    const track = this.tracks.get(key);
    if (!track) return;
    track.hidden = !track.hidden;
    this.dirty = true;
    this.legendChanged();
  }

  // Adds one reading at time t (seconds since the measurement started).
  add(key: number, t: number, value: number, info: SeriesInfo): void {
    let track = this.tracks.get(key);
    let rangeChanged = false;
    if (!track) {
      let low: number;
      let high: number;
      let fixed = false;
      if (info.displayMax > info.displayMin) {
        low = info.displayMin;
        high = info.displayMax;
        fixed = true;
      } else {
        // No range in the table: start around the first value and grow.
        const pad = Math.max(1, Math.abs(value) * 0.1);
        low = value - pad;
        high = value + pad;
      }
      track = { key, name: info.name, unit: info.unit, colour: this.tracks.size, low, high, fixed, times: [], values: [], hidden: false };
      this.tracks.set(key, track);
      rangeChanged = true;
    } else if (value < track.low || value > track.high) {
      // A reading outside the range: widen it with a little headroom so it
      // does not happen on every sample.
      const span = Math.max(1, track.high - track.low);
      if (value < track.low) track.low = value - span * 0.1;
      if (value > track.high) track.high = value + span * 0.1;
      rangeChanged = true;
    }
    track.times.push(t);
    track.values.push(value);
    // Drop points outside the time window.
    let drop = 0;
    while (track.times.length - drop > 2 && track.times[drop]! < t - this.windowSeconds) drop++;
    if (drop > 0) {
      track.times.splice(0, drop);
      track.values.splice(0, drop);
    }
    if (t > this.now) this.now = t;
    this.dirty = true;
    if (rangeChanged) this.legendChanged();
  }
}

// Share of the track's range in percent, -4 to 104 like the Qt axis.
export function share(track: Track, value: number): number {
  const span = track.high - track.low;
  if (span <= 0) return 50;
  return Math.min(104, Math.max(-4, ((value - track.low) / span) * 100));
}

// Range numbers like QString::arg(v, 0, 'g', 4).
function short(v: number): string {
  if (!Number.isFinite(v)) return "";
  const text = Number(v.toPrecision(4));
  return String(text);
}

// The index of the sample nearest to t (times are sorted).
function nearest(times: number[], t: number): number {
  let lo = 0;
  let hi = times.length - 1;
  if (hi < 0) return -1;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (times[mid]! < t) lo = mid + 1;
    else hi = mid;
  }
  if (lo > 0 && Math.abs(times[lo - 1]! - t) < Math.abs(times[lo]! - t)) return lo - 1;
  return lo;
}

function niceStep(span: number, target: number): number {
  const raw = span / Math.max(1, target);
  const mag = 10 ** Math.floor(Math.log10(raw));
  const norm = raw / mag;
  const step = norm < 1.5 ? 1 : norm < 3 ? 2 : norm < 7 ? 5 : 10;
  return step * mag;
}

interface Hover {
  x: number;
  y: number;
}

export function TimeSeriesChart({
  store,
  xTitle,
  yTitle,
  emptyText,
  height = 300,
  formatValue = (v: number) => v.toFixed(2),
}: {
  store: SeriesStore;
  xTitle: string;
  yTitle: string;
  // Shown in the plot while there is no series yet.
  emptyText?: string;
  height?: number;
  formatValue?: (v: number) => string;
}) {
  const active = usePageActive();
  const theme = useThemeVersion();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const plotRef = useRef<HTMLDivElement>(null);
  const hoverRef = useRef<Hover | null>(null);
  const [tip, setTip] = useState<{ x: number; y: number; time: number; rows: { colour: number; name: string; text: string }[] } | null>(null);
  const legend = useSyncExternalStore(store.subscribe, store.legendSnapshot);
  const sizeRef = useRef({ w: 0, h: 0, dpr: 1 });
  const paletteRef = useRef<{ series: string[]; grid: string; axis: string; text: string; faint: string; font: string } | null>(null);

  // Colours of the active theme, read back from the CSS tokens.
  useEffect(() => {
    const css = getComputedStyle(document.documentElement);
    paletteRef.current = {
      series: Array.from({ length: SERIES_COLOURS }, (_, i) => toCss(cssColor(`--series-${i + 1}`))),
      grid: toCss(cssColor("--border")),
      axis: toCss(cssColor("--border-bright")),
      text: toCss(cssColor("--text-muted")),
      faint: toCss(cssColor("--text-faint")),
      font: css.getPropertyValue("--font-sans").trim() || "sans-serif",
    };
    store.dirty = true;
  }, [theme, store]);

  // Size of the canvas in device pixels, so lines stay crisp on high DPI.
  useEffect(() => {
    const plot = plotRef.current;
    const canvas = canvasRef.current;
    if (!plot || !canvas) return;
    const apply = () => {
      const rect = plot.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      const w = Math.max(1, Math.round(rect.width));
      const h = Math.max(1, Math.round(rect.height));
      if (w === sizeRef.current.w && h === sizeRef.current.h && dpr === sizeRef.current.dpr) return;
      sizeRef.current = { w, h, dpr };
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
      store.dirty = true;
    };
    apply();
    const observer = new ResizeObserver(apply);
    observer.observe(plot);
    // A move to a screen with another scale changes devicePixelRatio.
    const media = window.matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`);
    media.addEventListener("change", apply);
    return () => {
      observer.disconnect();
      media.removeEventListener("change", apply);
    };
  }, [store]);

  const layout = useCallback(() => {
    const { w, h } = sizeRef.current;
    const left = 46;
    const right = 12;
    const top = 10;
    const bottom = 38;
    return { left, top, width: Math.max(10, w - left - right), height: Math.max(10, h - top - bottom), w, h };
  }, []);

  const range = useCallback(() => {
    const win = store.windowSeconds;
    const now = store.now;
    return now > win ? { from: now - win, to: now } : { from: 0, to: win };
  }, [store]);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const palette = paletteRef.current;
    if (!canvas || !palette) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const { dpr } = sizeRef.current;
    const L = layout();
    const { from, to } = range();
    const span = Math.max(0.001, to - from);
    const xOf = (t: number) => L.left + ((t - from) / span) * L.width;
    const yOf = (p: number) => L.top + (1 - (p + 4) / 108) * L.height;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, L.w, L.h);

    // Grid and axes
    ctx.lineWidth = 1;
    ctx.font = `11px ${palette.font}`;
    ctx.fillStyle = palette.faint;
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";
    for (const p of [0, 25, 50, 75, 100]) {
      const y = Math.round(yOf(p)) + 0.5;
      ctx.strokeStyle = palette.grid;
      ctx.beginPath();
      ctx.moveTo(L.left, y);
      ctx.lineTo(L.left + L.width, y);
      ctx.stroke();
      ctx.fillText(String(p), L.left - 8, y);
    }
    const step = niceStep(span, Math.max(2, Math.floor(L.width / 110)));
    ctx.textAlign = "center";
    ctx.textBaseline = "top";
    for (let t = Math.ceil(from / step) * step; t <= to + 1e-9; t += step) {
      const x = Math.round(xOf(t)) + 0.5;
      ctx.strokeStyle = palette.grid;
      ctx.beginPath();
      ctx.moveTo(x, L.top);
      ctx.lineTo(x, L.top + L.height);
      ctx.stroke();
      ctx.fillText(step < 1 ? t.toFixed(1) : String(Math.round(t)), x, L.top + L.height + 6);
    }
    ctx.strokeStyle = palette.axis;
    ctx.strokeRect(L.left + 0.5, L.top + 0.5, L.width, L.height);

    // Axis titles
    ctx.fillStyle = palette.text;
    ctx.font = `600 11px ${palette.font}`;
    ctx.textAlign = "center";
    ctx.textBaseline = "bottom";
    ctx.fillText(xTitle, L.left + L.width / 2, L.h - 2);
    ctx.save();
    ctx.translate(11, L.top + L.height / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.textBaseline = "middle";
    ctx.fillText(yTitle, 0, 0);
    ctx.restore();

    // Curves, clipped to the plot area
    ctx.save();
    ctx.beginPath();
    ctx.rect(L.left, L.top - 2, L.width, L.height + 4);
    ctx.clip();
    ctx.lineJoin = "round";
    ctx.lineCap = "round";
    ctx.lineWidth = 2;
    for (const track of store.tracks.values()) {
      if (track.hidden || track.times.length === 0) continue;
      ctx.strokeStyle = palette.series[track.colour % SERIES_COLOURS]!;
      ctx.beginPath();
      for (let i = 0; i < track.times.length; i++) {
        const x = xOf(track.times[i]!);
        const y = yOf(share(track, track.values[i]!));
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      if (track.times.length === 1) {
        const x = xOf(track.times[0]!);
        const y = yOf(share(track, track.values[0]!));
        ctx.lineTo(x + 0.1, y);
      }
      ctx.stroke();
    }
    ctx.restore();

    // Hover: a line at the pointer and the nearest reading of each curve
    const hover = hoverRef.current;
    if (hover && hover.x >= L.left && hover.x <= L.left + L.width && store.tracks.size > 0) {
      const time = from + ((hover.x - L.left) / L.width) * span;
      ctx.strokeStyle = palette.axis;
      ctx.setLineDash([4, 4]);
      ctx.beginPath();
      ctx.moveTo(Math.round(hover.x) + 0.5, L.top);
      ctx.lineTo(Math.round(hover.x) + 0.5, L.top + L.height);
      ctx.stroke();
      ctx.setLineDash([]);
      const rows: { colour: number; name: string; text: string }[] = [];
      for (const track of store.tracks.values()) {
        if (track.hidden) continue;
        const i = nearest(track.times, time);
        if (i < 0) continue;
        const v = track.values[i]!;
        const colour = palette.series[track.colour % SERIES_COLOURS]!;
        const x = xOf(track.times[i]!);
        const y = yOf(share(track, v));
        ctx.fillStyle = colour;
        ctx.beginPath();
        ctx.arc(x, y, 3.5, 0, Math.PI * 2);
        ctx.fill();
        rows.push({ colour: track.colour, name: track.name, text: `${formatValue(v)} ${track.unit}`.trim() });
      }
      setTip({ x: hover.x, y: hover.y, time, rows });
    } else if (hover) {
      // Pointer over the axis labels: no reading to show.
      setTip(null);
    }
  }, [layout, range, store, xTitle, yTitle, formatValue]);

  // Redraw while on screen and only when something changed.
  useEffect(() => {
    if (!active) return;
    store.dirty = true;
    let frame = 0;
    const loop = () => {
      if (store.dirty) {
        store.dirty = false;
        draw();
      }
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  }, [active, draw, store]);

  const onMove = (e: PointerEvent) => {
    const rect = plotRef.current?.getBoundingClientRect();
    if (!rect) return;
    hoverRef.current = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    store.dirty = true;
  };
  const onLeave = () => {
    hoverRef.current = null;
    setTip(null);
    store.dirty = true;
  };

  const plotWidth = sizeRef.current.w;
  const tipLeft = tip ? (tip.x > plotWidth - 240 ? Math.max(4, tip.x - 236) : tip.x + 14) : 0;

  return (
    <div className="ts-chart">
      <div className="ts-chart-plot" ref={plotRef} style={{ height }} onPointerMove={onMove} onPointerLeave={onLeave}>
        <canvas ref={canvasRef} aria-label={yTitle} role="img" />
        {legend.length === 0 && emptyText ? <div className="ts-chart-empty">{emptyText}</div> : null}
        {tip && tip.rows.length > 0 ? (
          <div className="ts-chart-tip" style={{ left: tipLeft, top: Math.max(4, Math.min(tip.y - 10, height - 40 - tip.rows.length * 18)) }}>
            <div className="ts-chart-tip-time ltr">{tip.time.toFixed(1)} s</div>
            {tip.rows.map((r, i) => (
              <div className="ts-chart-tip-row" key={i}>
                <span className="dot" style={{ background: seriesColour(r.colour) }} />
                <span className="name">{r.name}</span>
                <span className="value ltr">{r.text}</span>
              </div>
            ))}
          </div>
        ) : null}
      </div>
      {legend.length > 0 ? (
        <div className="ts-legend">
          {legend.map((entry) => (
            <button
              type="button"
              key={entry.key}
              className={`ts-legend-item${entry.hidden ? " off" : ""}`}
              onClick={() => store.toggle(entry.key)}
              aria-pressed={!entry.hidden}
              title={`${entry.name} [${short(entry.low)}, ${short(entry.high)}] ${entry.unit}`.trim()}
            >
              <span className="swatch" style={{ background: seriesColour(entry.colour) }} />
              <span className="label">{entry.name}</span>
              <span className="range ltr">
                [{short(entry.low)}, {short(entry.high)}]{entry.unit ? ` ${entry.unit}` : ""}
              </span>
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
