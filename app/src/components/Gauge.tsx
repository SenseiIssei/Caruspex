// Round gauge with a smoothly trailing needle (GaugeWidget.cpp): a 240
// degree scale from bottom left to bottom right, ticks inside the arc, the
// value arc in the accent gradient, amber to red outside the warn range.
import { useEffect, useId, useRef, useState } from "react";

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

export function Gauge({
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
  const [shown, setShown] = useState(target);
  const shownRef = useRef(target);

  // The needle trails the value: 450 ms, ease out cubic.
  useEffect(() => {
    const from = shownRef.current;
    const start = performance.now();
    let frame = 0;
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / 450);
      const v = from + (target - from) * (1 - (1 - t) ** 3);
      shownRef.current = v;
      setShown(v);
      if (t < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [target]);

  const titleSpace = 26;
  const side = size;
  const thickness = Math.max(7, side * 0.07);
  const radius = side / 2 - thickness * 1.5;
  const cx = side / 2;
  const cy = side / 2 + thickness * 0.5;
  const fraction = Math.max(0, Math.min(1, (shown - lo) / (hi - lo)));
  const hasWarn = warnFrom !== undefined && warnTo !== undefined && (warnFrom !== 0 || warnTo !== 0);
  const warning = active && hasWarn && (target < (warnFrom ?? -Infinity) || target > (warnTo ?? Infinity));

  const ticks = Array.from({ length: 41 }, (_, i) => {
    const deg = START + (SWEEP * i) / 40;
    const major = i % 10 === 0;
    const a = point(cx, cy, radius - thickness * (major ? 1.55 : 1.25), deg);
    const b = point(cx, cy, radius - thickness * 0.95, deg);
    return { a, b, major };
  });
  const tip = point(cx, cy, radius, START + SWEEP * fraction);
  const text = active ? shown.toFixed(decimals) : "--";

  return (
    <div className="gauge" style={{ width: size }}>
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
        {ticks.map((t, i) => (
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
        <path d={arcPath(cx, cy, radius, START, SWEEP)} fill="none" stroke="var(--track)" strokeWidth={thickness} strokeLinecap="round" />
        {active && fraction > 0.001 ? (
          <>
            <path
              d={arcPath(cx, cy, radius, START, SWEEP * fraction)}
              fill="none"
              stroke={`url(#${id}-${warning ? "warn" : "ok"})`}
              strokeWidth={thickness}
              strokeLinecap="round"
            />
            <circle cx={tip.x} cy={tip.y} r={thickness * 1.05} fill={warning ? "var(--warn)" : "var(--sky)"} opacity={0.3} />
            <circle cx={tip.x} cy={tip.y} r={thickness * 0.3} fill="#fff" opacity={0.92} />
          </>
        ) : null}
        <text
          x={cx}
          y={cy + side * 0.04}
          textAnchor="middle"
          className="gauge-value"
          style={{ fontSize: Math.max(16, side * 0.17), fill: active ? "var(--text)" : "var(--text-faint)" }}
        >
          {text}
        </text>
        <text x={cx} y={cy + side * 0.15} textAnchor="middle" className="gauge-unit" style={{ fontSize: Math.max(11, side * 0.07) }}>
          {unit}
        </text>
      </svg>
      <div className="gauge-title" style={{ height: titleSpace }}>
        {title}
      </div>
    </div>
  );
}
