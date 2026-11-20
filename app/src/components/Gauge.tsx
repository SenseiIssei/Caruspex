// Round gauge with a smoothly trailing needle (GaugeWidget.cpp): a 240
// degree scale from bottom left to bottom right, ticks inside the arc, the
// value arc in the accent gradient, amber to red outside the warn range.
//
// React draws the gauge when its value changes; the needle in between moves
// sixty times a second by setting four attributes directly, so six gauges on
// the start page do not mean three hundred renders a second.
import { memo, useEffect, useId, useLayoutEffect, useMemo, useRef } from "react";

const START = 210; // degrees, counter-clockwise from the x axis
const SWEEP = -240;

function point(cx: number, cy: number, r: number, deg: number) {
  const a = (deg * Math.PI) / 180;
  return { x: cx + Math.cos(a) * r, y: cy - Math.sin(a) * r };
}

function arcPath(cx: number, cy: number, r: number, from: number, sweep: number): string {
  const a = point(cx, cy, r, from);
  const b = point(cx, cy, r, from + sweep);
  const large = Math.abs(sweep) > 180 ? 1 : 0;
  const clockwise = sweep < 0 ? 1 : 0;
  return `M ${a.x} ${a.y} A ${r} ${r} 0 ${large} ${clockwise} ${b.x} ${b.y}`;
}

// Everything that only depends on the size.
function geometry(size: number) {
  const side = size;
  const thickness = Math.max(7, side * 0.07);
  const radius = side / 2 - thickness * 1.5;
  const cx = side / 2;
  const cy = side / 2 + thickness * 0.5;
  const ticks = Array.from({ length: 41 }, (_, i) => {
    const deg = START + (SWEEP * i) / 40;
    const major = i % 10 === 0;
    const a = point(cx, cy, radius - thickness * (major ? 1.55 : 1.25), deg);
    const b = point(cx, cy, radius - thickness * 0.95, deg);
    return { a, b, major };
  });
  return { side, thickness, radius, cx, cy, ticks, track: arcPath(cx, cy, radius, START, SWEEP) };
}

export const Gauge = memo(function Gauge({
  title,
  unit,
  min,
  max,
  value,
  decimals = 0,
  warnFrom,
  warnTo,
  size = 180,
}: {
  title: string;
  unit: string;
  min: number;
  max: number;
  // null while nothing is measured: grey, "--".
  value: number | null;
  decimals?: number;
  warnFrom?: number;
  warnTo?: number;
  size?: number;
}) {
  const id = useId();
  const lo = min;
  const hi = max > min ? max : min + 1;
  const active = value !== null && Number.isFinite(value);
  const target = active ? Math.min(hi, Math.max(lo, value)) : lo;
  const shownRef = useRef(target);
  const geo = useMemo(() => geometry(size), [size]);
  const { side, thickness, radius, cx, cy } = geo;

  const groupRef = useRef<SVGGElement>(null);
  const arcRef = useRef<SVGPathElement>(null);
  const haloRef = useRef<SVGCircleElement>(null);
  const dotRef = useRef<SVGCircleElement>(null);
  const textRef = useRef<SVGTextElement>(null);

  // Puts the needle at v: the value arc, the tip and the number.
  const paint = (v: number) => {
    const fraction = Math.max(0, Math.min(1, (v - lo) / (hi - lo)));
    const group = groupRef.current;
    const show = active && fraction > 0.001;
    if (group) group.style.display = show ? "" : "none";
    if (show) {
      const tip = point(cx, cy, radius, START + SWEEP * fraction);
      arcRef.current?.setAttribute("d", arcPath(cx, cy, radius, START, SWEEP * fraction));
      for (const dot of [haloRef.current, dotRef.current]) {
        dot?.setAttribute("cx", String(tip.x));
        dot?.setAttribute("cy", String(tip.y));
      }
    }
    if (textRef.current) textRef.current.textContent = active ? v.toFixed(decimals) : "--";
  };
  const paintRef = useRef(paint);
  paintRef.current = paint;

  // Every render draws the needle where it stands, before the screen shows it.
  useLayoutEffect(() => paint(shownRef.current));

  // The needle trails the value: 450 ms, ease out cubic.
  useEffect(() => {
    const from = shownRef.current;
    const start = performance.now();
    let frame = 0;
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / 450);
      const v = from + (target - from) * (1 - (1 - t) ** 3);
      shownRef.current = v;
      paintRef.current(v);
      if (t < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [target]);

  const titleSpace = 26;
  const hasWarn = warnFrom !== undefined && warnTo !== undefined && (warnFrom !== 0 || warnTo !== 0);
  const warning = active && hasWarn && (target < (warnFrom ?? -Infinity) || target > (warnTo ?? Infinity));

  return (
    <div className="gauge" style={{ width: size }} role="img" aria-label={`${title}: ${active ? `${value.toFixed(decimals)} ${unit}` : "--"}`}>
      <svg width={size} height={side} viewBox={`0 0 ${side} ${side}`} aria-hidden>
        <defs>
          <linearGradient id={`${id}-ok`} x1="0" y1="1" x2="1" y2="1">
            <stop offset="0" stopColor="var(--accent)" />
            <stop offset="1" stopColor="var(--sky)" />
          </linearGradient>
          <linearGradient id={`${id}-warn`} x1="0" y1="1" x2="1" y2="1">
            <stop offset="0" stopColor="var(--warn)" />
            <stop offset="1" stopColor="var(--bad)" />
          </linearGradient>
        </defs>
        {geo.ticks.map((t, i) => (
          <line
            key={i}
            x1={t.a.x}
            y1={t.a.y}
            x2={t.b.x}
            y2={t.b.y}
            stroke="var(--border-bright)"
            strokeOpacity={t.major ? 0.9 : 0.5}
            strokeWidth={t.major ? 1.4 : 1}
          />
        ))}
        <path d={geo.track} fill="none" stroke="var(--track)" strokeWidth={thickness} strokeLinecap="round" />
        <g ref={groupRef}>
          <path ref={arcRef} fill="none" stroke={`url(#${id}-${warning ? "warn" : "ok"})`} strokeWidth={thickness} strokeLinecap="round" />
          <circle ref={haloRef} r={thickness * 1.05} fill={warning ? "var(--warn)" : "var(--sky)"} opacity={0.3} />
          <circle ref={dotRef} r={thickness * 0.3} fill="#fff" opacity={0.92} />
        </g>
        <text
          ref={textRef}
          x={cx}
          y={cy + side * 0.04}
          textAnchor="middle"
          className="gauge-value"
          style={{ fontSize: Math.max(16, side * 0.17), fill: active ? "var(--text)" : "var(--text-faint)" }}
        />
        <text x={cx} y={cy + side * 0.15} textAnchor="middle" className="gauge-unit" style={{ fontSize: Math.max(11, side * 0.07) }}>
          {unit}
        </text>
      </svg>
      <div className="gauge-title" style={{ height: titleSpace }}>
        {title}
      </div>
    </div>
  );
});
