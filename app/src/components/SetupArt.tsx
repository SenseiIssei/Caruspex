// The simple line drawings of the first-start assistant: the usual adapter
// kinds, where the OBD socket sits in a car (seen from above, left-hand
// drive) and the ignition. Drawn with the theme colours, so they work in
// dark, light and high contrast; no photos.
import type { ReactNode } from "react";

export type AdapterKind = "usb" | "bluetooth" | "wifi" | "enet" | "kkl";

const LINE = { fill: "none", stroke: "currentColor", strokeWidth: 2.2, strokeLinecap: "round", strokeLinejoin: "round" } as const;

// The 16-pin OBD plug seen from the front: the trapezoid with two rows of
// pins, at (x, y) with width w.
function ObdPlug({ x, y, w = 44 }: { x: number; y: number; w?: number }) {
  const h = w * 0.5;
  const inset = w * 0.12;
  const pins: ReactNode[] = [];
  for (let row = 0; row < 2; row++) {
    for (let i = 0; i < 8; i++) {
      const px = x + inset + 4 + ((w - 2 * inset - 8) * i) / 7 + (row === 1 ? 0 : 0);
      const py = y + h * (row === 0 ? 0.36 : 0.68);
      pins.push(<circle key={`${row}-${i}`} cx={px} cy={py} r={1.4} fill="currentColor" />);
    }
  }
  return (
    <g>
      <path d={`M${x} ${y} H${x + w} L${x + w - inset} ${y + h} H${x + inset} Z`} {...LINE} fill="var(--surface-high)" />
      {pins}
    </g>
  );
}

function UsbPlug({ x, y }: { x: number; y: number }) {
  return (
    <g>
      <rect x={x} y={y} width={18} height={12} rx={2} {...LINE} fill="var(--surface-high)" />
      <rect x={x + 18} y={y + 2} width={10} height={8} {...LINE} />
      <path d={`M${x + 21} ${y + 5}h4`} {...LINE} strokeWidth={1.6} />
    </g>
  );
}

export function AdapterArt({ kind }: { kind: AdapterKind }) {
  return (
    <svg className="setup-art" viewBox="0 0 140 90" aria-hidden>
      {kind === "usb" ? (
        <>
          <ObdPlug x={8} y={30} />
          <path d="M52 41 C75 41 72 66 96 62" {...LINE} />
          <UsbPlug x={96} y={56} />
        </>
      ) : null}
      {kind === "bluetooth" ? (
        <>
          <ObdPlug x={38} y={56} w={50} />
          <rect x={44} y={14} width={38} height={42} rx={6} {...LINE} fill="var(--surface-high)" />
          <path d="M58 26l12 10-6 5V20l6 5-12 10" {...LINE} stroke="var(--sky)" strokeWidth={2} />
          <circle cx={74} cy={50} r={2} fill="var(--good)" />
        </>
      ) : null}
      {kind === "wifi" ? (
        <>
          <ObdPlug x={38} y={58} w={50} />
          <rect x={44} y={26} width={38} height={32} rx={6} {...LINE} fill="var(--surface-high)" />
          <path d="M51 20a18 18 0 0 1 24 0" {...LINE} stroke="var(--sky)" />
          <path d="M45 13a27 27 0 0 1 36 0" {...LINE} stroke="var(--sky)" />
          <circle cx={63} cy={42} r={2.2} fill="var(--sky)" />
        </>
      ) : null}
      {kind === "enet" ? (
        <>
          <ObdPlug x={8} y={30} />
          <path d="M52 41 C75 41 72 66 94 62" {...LINE} />
          <rect x={94} y={52} width={30} height={22} rx={2} {...LINE} fill="var(--surface-high)" />
          <path d="M100 74v-6h18v6" {...LINE} strokeWidth={1.6} />
          <path d="M101 58h16M101 62h16" {...LINE} strokeWidth={1.2} />
        </>
      ) : null}
      {kind === "kkl" ? (
        <>
          <ObdPlug x={8} y={30} />
          <path d="M52 41 C62 41 64 50 70 50" {...LINE} />
          <rect x={70} y={43} width={20} height={14} rx={4} {...LINE} fill="var(--surface-high)" />
          <path d="M90 50 C96 50 92 63 98 63" {...LINE} />
          <UsbPlug x={98} y={57} />
        </>
      ) : null}
    </svg>
  );
}

// Socket spots in the plan view (viewBox 320 x 280), index = SocketLocation.
const SPOTS: Record<number, [number, number]> = {
  1: [92, 128],
  2: [112, 140],
  3: [72, 172],
  4: [42, 118],
  5: [160, 136],
  6: [160, 154],
  7: [228, 128],
  8: [160, 40],
  9: [160, 236],
};

// The car seen from above with the socket marked. 0 (unknown) marks the
// usual spots: under the dashboard on the driver side first.
export function SocketSketch({ location, label }: { location: number; label: string }) {
  const known = location in SPOTS;
  const marks: [number, number][] = known ? [SPOTS[location]!] : [SPOTS[1]!, SPOTS[2]!, SPOTS[5]!];
  return (
    <svg className="setup-sketch" viewBox="0 0 320 280" role="img" aria-label={label}>
      {/* Body, bonnet line, windscreen */}
      <rect x={20} y={6} width={280} height={268} rx={56} {...LINE} fill="var(--surface)" />
      <path d="M40 76 H280" {...LINE} strokeOpacity={0.5} />
      <path d="M44 92 Q160 70 276 92" {...LINE} />
      {/* Dashboard */}
      <path d="M36 100 H284 V116 Q160 126 36 116 Z" {...LINE} fill="var(--surface-high)" />
      {/* Steering wheel */}
      <circle cx={112} cy={118} r={24} {...LINE} />
      <path d="M90 116 H134 M112 116 V140" {...LINE} strokeWidth={1.6} />
      {/* Centre console with gear lever */}
      <rect x={146} y={118} width={28} height={92} rx={8} {...LINE} fill="var(--surface-high)" />
      <circle cx={160} cy={178} r={5} {...LINE} />
      {/* Front seats and rear bench */}
      <rect x={62} y={160} width={70} height={56} rx={14} {...LINE} />
      <rect x={188} y={160} width={70} height={56} rx={14} {...LINE} />
      <rect x={52} y={226} width={216} height={34} rx={14} {...LINE} />
      {/* Engine bay hint */}
      <path d="M120 30 H200 M130 44 H190" {...LINE} strokeOpacity={0.45} />
      {marks.map(([x, y], i) => (
        <g key={i} className={i === 0 ? "setup-spot main" : "setup-spot"}>
          <circle cx={x} cy={y} r={16} fill="var(--accent)" opacity={0.18} className="setup-spot-pulse" />
          <circle cx={x} cy={y} r={7} fill="var(--accent)" stroke="var(--surface)" strokeWidth={2} />
        </g>
      ))}
    </svg>
  );
}

// The ignition lock turned to II, with the warning lamps that light up.
export function IgnitionArt() {
  const marks = ["0", "I", "II", "III"];
  return (
    <svg className="setup-art wide" viewBox="0 0 240 110" aria-hidden>
      <circle cx={60} cy={58} r={36} {...LINE} fill="var(--surface-high)" />
      {marks.map((m, i) => {
        const a = ((-150 + i * 40) * Math.PI) / 180;
        return (
          <text
            key={m}
            x={60 + Math.cos(a) * 46}
            y={58 + Math.sin(a) * 46 + 4}
            textAnchor="middle"
            fontSize={11}
            fontWeight={i === 2 ? 800 : 500}
            fill={i === 2 ? "var(--accent)" : "currentColor"}
          >
            {m}
          </text>
        );
      })}
      {/* The key, turned to II */}
      <g transform="rotate(20 60 58)">
        <rect x={54} y={30} width={12} height={56} rx={5} {...LINE} fill="var(--surface)" />
        <path d="M60 36v40" {...LINE} strokeWidth={1.4} />
      </g>
      {/* Lamps in the instrument cluster */}
      <rect x={120} y={26} width={108} height={60} rx={12} {...LINE} />
      <g transform="translate(130 44)">
        <path d="M0 6h4V2h5V0h6v2h4l3 3v8h-3l-3 3H4l-4-3z" fill="none" stroke="var(--warn)" strokeWidth={1.8} strokeLinejoin="round" />
      </g>
      <g transform="translate(166 44)">
        <rect x={0} y={3} width={22} height={13} rx={2} fill="none" stroke="var(--bad)" strokeWidth={1.8} />
        <path d="M4 3V0M18 3V0M5 9h4M14 9h4M16 7v4" fill="none" stroke="var(--bad)" strokeWidth={1.6} strokeLinecap="round" />
      </g>
      <g transform="translate(200 44)">
        <path d="M2 14c0-6 4-10 10-10h4l4 4-6 6z" fill="none" stroke="var(--bad)" strokeWidth={1.8} strokeLinejoin="round" />
        <circle cx={4} cy={18} r={1.5} fill="var(--bad)" />
      </g>
    </svg>
  );
}
