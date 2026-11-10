// Small SVG charts for tables and tiles: a sparkline of recent values, a bar
// that places min, mean and current inside a range, and a histogram of how
// the values are spread. Crisp at every size and themed through CSS tokens.
import "./charts.css";

// A line of the recent values, scaled to their own minimum and maximum.
export function Sparkline({
  values,
  width = 120,
  height = 28,
  colour = "var(--accent)",
}: {
  values: readonly number[];
  width?: number;
  height?: number;
  colour?: string;
}) {
  if (values.length < 2) return <svg className="sparkline" width={width} height={height} aria-hidden />;
  let lo = Infinity;
  let hi = -Infinity;
  for (const v of values) {
    if (v < lo) lo = v;
    if (v > hi) hi = v;
  }
  const span = hi - lo || 1;
  const pad = 2;
  const step = (width - pad * 2) / (values.length - 1);
  const y = (v: number) => (hi === lo ? height / 2 : pad + (1 - (v - lo) / span) * (height - pad * 2));
  let d = "";
  values.forEach((v, i) => {
    d += `${i === 0 ? "M" : "L"}${(pad + i * step).toFixed(1)} ${y(v).toFixed(1)}`;
  });
  const last = values[values.length - 1]!;
  const area = `${d}L${(pad + (values.length - 1) * step).toFixed(1)} ${height}L${pad} ${height}Z`;
  return (
    <svg className="sparkline" width={width} height={height} viewBox={`0 0 ${width} ${height}`} aria-hidden>
      <path d={area} fill={colour} opacity={0.12} />
      <path d={d} fill="none" stroke={colour} strokeWidth={1.6} strokeLinejoin="round" strokeLinecap="round" />
      <circle cx={pad + (values.length - 1) * step} cy={y(last)} r={2.4} fill={colour} />
    </svg>
  );
}

// Minimum to maximum as a bar, with ticks for the mean and the current value.
export function RangeBar({
  low,
  high,
  min,
  max,
  mean,
  current,
  width = 140,
  height = 14,
}: {
  // The scale (the table range or the seen range).
  low: number;
  high: number;
  min: number;
  max: number;
  mean: number;
  current: number;
  width?: number;
  height?: number;
}) {
  const span = high - low || 1;
  const x = (v: number) => Math.max(0, Math.min(width, ((v - low) / span) * width));
  const mid = height / 2;
  return (
    <svg className="range-bar" width={width} height={height} viewBox={`0 0 ${width} ${height}`} aria-hidden>
      <rect x={0} y={mid - 2} width={width} height={4} rx={2} fill="var(--track)" />
      <rect x={x(min)} y={mid - 3} width={Math.max(2, x(max) - x(min))} height={6} rx={3} fill="var(--accent)" opacity={0.55} />
      <line x1={x(mean)} x2={x(mean)} y1={1} y2={height - 1} stroke="var(--text-muted)" strokeWidth={1.5} />
      <circle cx={x(current)} cy={mid} r={3.6} fill="var(--sky)" stroke="var(--surface)" strokeWidth={1.2} />
    </svg>
  );
}

export interface HistogramMarker {
  value: number;
  label: string;
  colour: string;
}

// How the values spread between their minimum and maximum, in bins, with
// optional markers (min, mean, max) below the bars.
export function Histogram({
  values,
  bins = 24,
  height = 120,
  markers = [],
  format = (v: number) => v.toFixed(1),
}: {
  values: readonly number[];
  bins?: number;
  height?: number;
  markers?: HistogramMarker[];
  format?: (v: number) => string;
}) {
  const width = 600;
  const axis = 30;
  const plot = height - axis;
  if (values.length === 0) return <svg className="histogram" viewBox={`0 0 ${width} ${height}`} aria-hidden />;
  let lo = Infinity;
  let hi = -Infinity;
  for (const v of values) {
    if (v < lo) lo = v;
    if (v > hi) hi = v;
  }
  if (hi === lo) {
    lo -= 0.5;
    hi += 0.5;
  }
  const counts = new Array<number>(bins).fill(0);
  for (const v of values) {
    const i = Math.min(bins - 1, Math.floor(((v - lo) / (hi - lo)) * bins));
    counts[i]!++;
  }
  const peak = Math.max(...counts);
  const barWidth = width / bins;
  const xOf = (v: number) => ((v - lo) / (hi - lo)) * width;
  return (
    <svg className="histogram" viewBox={`0 0 ${width} ${height}`} role="img">
      <defs>
        <linearGradient id="histogram-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="var(--sky)" />
          <stop offset="1" stopColor="var(--accent)" />
        </linearGradient>
      </defs>
      {counts.map((c, i) => {
        const h = peak > 0 ? (c / peak) * (plot - 6) : 0;
        return (
          <rect
            key={i}
            className="bin"
            x={i * barWidth + 1}
            y={plot - h}
            width={Math.max(1, barWidth - 2)}
            height={h}
            rx={2}
            fill="url(#histogram-fill)"
            opacity={0.85}
          />
        );
      })}
      <line x1={0} x2={width} y1={plot + 0.5} y2={plot + 0.5} stroke="var(--border-bright)" />
      {markers.map((m, i) => {
        const x = Math.max(0, Math.min(width, xOf(m.value)));
        const anchor = x < 60 ? "start" : x > width - 60 ? "end" : "middle";
        return (
          <g key={i}>
            <line x1={x} x2={x} y1={0} y2={plot} stroke={m.colour} strokeWidth={1.5} strokeDasharray="4 3" vectorEffect="non-scaling-stroke" />
            <text className="histogram-axis" x={x} y={plot + 13 + (i % 2) * 12} textAnchor={anchor}>
              {m.label} {format(m.value)}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
