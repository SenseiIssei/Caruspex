// The big health ring of Widgets.cpp: a gradient arc that fills up to the
// score, the number counting up in the middle, a slowly turning dotted halo
// around it. While something is read the arc turns into a spinner.
import { useEffect, useId, useRef, useState } from "react";
import { healthColor, lighter, toCss, useThemeVersion } from "../lib/theme";

export function ScoreRing({
  score,
  caption,
  subcaption,
  busy,
  size = 240,
}: {
  // 0..100, or a negative number for "not known yet".
  score: number;
  caption: string;
  subcaption?: string;
  busy?: boolean;
  size?: number;
}) {
  useThemeVersion();
  const id = useId();
  const [shown, setShown] = useState(score >= 0 ? 0 : 0);
  const shownRef = useRef(0);

  // Count up from the previous value, 1.4 s, ease out cubic.
  useEffect(() => {
    if (score < 0) {
      shownRef.current = 0;
      setShown(0);
      return;
    }
    const from = shownRef.current;
    const start = performance.now();
    let frame = 0;
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / 1400);
      const eased = 1 - (1 - t) ** 3;
      const v = from + (score - from) * eased;
      shownRef.current = v;
      setShown(v);
      if (t < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [score]);

  const known = score >= 0;
  const fraction = known ? Math.max(0, Math.min(1, shown / 100)) : 0;
  const tone = known ? healthColor(fraction) : null;
  const toneCss = tone ? toCss(tone) : "var(--accent)";
  const toneLight = tone ? toCss(lighter(tone, 1.25)) : "var(--accent-hover)";

  const c = size / 2;
  const outer = size / 2 - 4;
  const radius = outer - 22;
  const thickness = Math.max(10, size * 0.055);
  const circumference = 2 * Math.PI * radius;
  const dots = Array.from({ length: 72 }, (_, i) => {
    const a = (i * 5 * Math.PI) / 180;
    return { x: c + Math.cos(a) * outer, y: c + Math.sin(a) * outer, major: i % 6 === 0 };
  });
  const tipAngle = ((90 - 360 * fraction) * Math.PI) / 180;
  const tip = { x: c + Math.cos(tipAngle) * radius, y: c - Math.sin(tipAngle) * radius };

  return (
    <div
      className="score-ring"
      style={{ width: size, height: size }}
      role="img"
      aria-label={[known ? `${Math.round(score)} / 100` : "--", caption, subcaption].filter(Boolean).join(", ")}
    >
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden>
        <defs>
          <radialGradient id={`${id}-halo`}>
            <stop offset="0.55" stopColor={toneCss} stopOpacity="0.12" />
            <stop offset="1" stopColor={toneCss} stopOpacity="0" />
          </radialGradient>
          <radialGradient id={`${id}-disc`}>
            <stop offset="0" stopColor="var(--surface-high)" />
            <stop offset="1" stopColor="var(--surface)" />
          </radialGradient>
          <linearGradient id={`${id}-arc`} x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor={toneLight} />
            <stop offset="1" stopColor={toneCss} />
          </linearGradient>
          <linearGradient id={`${id}-spin`} x1="0" y1="0" x2="1" y2="0">
            <stop offset="0" stopColor="var(--sky)" />
            <stop offset="1" stopColor="var(--accent)" />
          </linearGradient>
        </defs>
        <circle className="score-halo" cx={c} cy={c} r={outer} fill={`url(#${id}-halo)`} />
        <circle cx={c} cy={c} r={radius - thickness / 2} fill={`url(#${id}-disc)`} />
        <g className="score-dots" style={{ transformOrigin: `${c}px ${c}px` }}>
          {dots.map((d, i) => (
            <circle key={i} cx={d.x} cy={d.y} r={1.1} fill="var(--border-bright)" opacity={d.major ? 0.9 : 0.45} />
          ))}
        </g>
        <circle cx={c} cy={c} r={radius} fill="none" stroke="var(--track)" strokeWidth={thickness} />
        {busy ? (
          <g className="score-spin" style={{ transformOrigin: `${c}px ${c}px` }}>
            <circle
              cx={c}
              cy={c}
              r={radius}
              fill="none"
              stroke={`url(#${id}-spin)`}
              strokeWidth={thickness}
              strokeLinecap="round"
              strokeDasharray={`${circumference * 0.25} ${circumference}`}
            />
          </g>
        ) : known && fraction > 0 ? (
          <>
            <circle
              cx={c}
              cy={c}
              r={radius}
              fill="none"
              stroke={`url(#${id}-arc)`}
              strokeWidth={thickness}
              strokeLinecap="round"
              strokeDasharray={`${circumference * fraction} ${circumference}`}
              transform={`rotate(-90 ${c} ${c})`}
            />
            <circle cx={tip.x} cy={tip.y} r={thickness * 0.9} fill={toneCss} opacity={0.35} />
            <circle cx={tip.x} cy={tip.y} r={thickness * 0.22} fill="#fff" opacity={0.9} />
          </>
        ) : null}
      </svg>
      <div className="score-text">
        <div className="score-number" style={{ fontSize: Math.max(30, size * 0.22), backgroundImage: `linear-gradient(180deg, var(--number-top), ${known ? toneLight : "var(--text-muted)"})` }}>
          {known ? Math.round(shown) : "--"}
        </div>
        <div className="score-caption" style={{ fontSize: Math.max(13, size * 0.058) }}>
          {caption}
        </div>
        {subcaption ? (
          <div className="score-subcaption" style={{ fontSize: Math.max(11, size * 0.045) }}>
            {subcaption}
          </div>
        ) : null}
      </div>
    </div>
  );
}
