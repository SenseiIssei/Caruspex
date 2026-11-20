// The zone labels of the see-through mode: chips above the car, joined to
// their part by a thin line, drawn in real pixels.
import { CarZone } from "../../api";
import { t } from "../../i18n";
import type { Part } from "./geometry";
import { n } from "./path";

const CTX = "caruspex::VehicleDiagram";

// The zone labels have to be visible to the translation scanner as the texts
// they are (QT_TR_NOOP in the C++ code).
function zoneLabel(label: string): string {
  switch (label) {
    case "Kühlung": return t(CTX, "Kühlung");
    case "Ansaugung": return t(CTX, "Ansaugung");
    case "Drosselklappe": return t(CTX, "Drosselklappe");
    case "Motor": return t(CTX, "Motor");
    case "Einspritzung": return t(CTX, "Einspritzung");
    case "Zündung": return t(CTX, "Zündung");
    case "Turbo": return t(CTX, "Turbo");
    case "Getriebe": return t(CTX, "Getriebe");
    case "Bordnetz": return t(CTX, "Bordnetz");
    case "Steuergerät": return t(CTX, "Steuergerät");
    case "Abgas": return t(CTX, "Abgas");
    case "Tank": return t(CTX, "Tank");
    case "Innenraum": return t(CTX, "Innenraum");
    case "Datenbus": return t(CTX, "Datenbus");
    case "Räder und ABS": return t(CTX, "Räder und ABS");
  }
  return label;
}

// --- Labels -----------------------------------------------------------------------------

let measureCanvas: HTMLCanvasElement | null = null;

function textWidth(text: string, font: string): number {
  if (typeof document === "undefined") return text.length * 7.5;
  measureCanvas ??= document.createElement("canvas");
  const ctx = measureCanvas.getContext("2d");
  if (!ctx) return text.length * 7.5;
  ctx.font = font;
  return ctx.measureText(text).width;
}

interface Chip {
  part: Part;
  text: string;
  x: number;
  w: number;
  hit: boolean;
}

// One chip per affected zone above the car, plus the hovered one, joined to
// the part by a thin line. Chips are spread so they never overlap.
export function layoutChips(parts: Part[], affected: Set<CarZone>, hover: CarZone, width: number, map: (x: number) => number): Chip[] {
  const family = typeof document !== "undefined" ? getComputedStyle(document.documentElement).getPropertyValue("--font-sans") : "";
  const font = `600 13px ${family || "sans-serif"}`;
  const chips: Chip[] = [];
  for (const part of parts) {
    const hit = affected.has(part.zone);
    if (!hit && part.zone !== hover) continue;
    const text = zoneLabel(part.label);
    chips.push({ part, text, x: map(part.anchor.x), w: textWidth(text, font) + 26, hit });
  }
  chips.sort((a, b) => a.x - b.x);
  // Spread left to right, then pull back inside the field on the right.
  let cursor = 8;
  for (const c of chips) {
    c.x = Math.max(c.x - c.w / 2, cursor);
    cursor = c.x + c.w + 8;
  }
  const right = width - 8;
  let overflow = cursor - 8 - right;
  for (let i = chips.length - 1; i >= 0 && overflow > 0; i--) {
    const room = i > 0 ? chips[i]!.x - (chips[i - 1]!.x + chips[i - 1]!.w + 8) : chips[i]!.x - 8;
    const shift = Math.min(overflow, Math.max(0, room));
    for (let j = i; j < chips.length; j++) chips[j]!.x -= shift;
    overflow -= shift;
  }
  return chips;
}

// The label layer over the car; its text stays the same size at every width.
export function DiagramLabels({
  chips,
  focus,
  size,
  mapX,
  mapY,
}: {
  chips: Chip[];
  focus: CarZone;
  size: { w: number; h: number };
  mapX: (x: number) => number;
  mapY: (y: number) => number;
}) {
  return (
    <svg className="vd-labels" width={size.w} height={size.h} aria-hidden>
      {chips.map((c) => {
        const focused = c.part.zone === focus;
        const tone = c.hit ? "var(--bad)" : "var(--accent)";
        const chipY = 6;
        const chipH = 26;
        const fx = c.x + c.w / 2;
        const fy = chipY + chipH;
        const tx = mapX(c.part.anchor.x);
        const ty = mapY(c.part.anchor.y);
        return (
          <g key={c.part.zone} className="vd-chip fade-in">
            <path
              d={`M${n(fx)} ${n(fy)}C${n(fx)} ${n(fy + 40)} ${n(tx)} ${n(ty - 60)} ${n(tx)} ${n(ty)}`}
              style={{ fill: "none", stroke: tone, strokeOpacity: focused ? 0.9 : 0.55, strokeWidth: 1.1 }}
            />
            <circle cx={tx} cy={ty} r="3.2" style={{ fill: tone }} />
            <rect
              x={c.x + 0.5}
              y={chipY + 0.5}
              width={c.w - 1}
              height={chipH - 1}
              rx="13"
              style={{
                fill: "var(--surface)",
                fillOpacity: 240 / 255,
                stroke: tone,
                strokeOpacity: focused ? 1 : 0.6,
                strokeWidth: focused ? 1.6 : 1,
              }}
            />
            <circle cx={c.x + 12} cy={chipY + chipH / 2} r="3.5" style={{ fill: tone }} />
            <text x={c.x + 20} y={chipY + chipH / 2} dominantBaseline="central" className="vd-chip-text">
              {c.text}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
