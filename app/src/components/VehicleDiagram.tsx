// Side view of a car, drawn in code (VehicleDiagram.cpp). Two uses:
//
//  - On the start page as a clean picture of the connected car, in the body
//    style the catalog knows for it.
//  - On the fault page in "see-through" mode: the body turns into glass and
//    the parts behind a fault (engine, intake, exhaust, tank, gearbox, ...)
//    light up where they sit in a real car.
//
// The car is laid out in a fixed 1000 by 400 field and scaled to the element
// without distortion (the SVG viewBox does what fieldToWidget() did), so it
// looks right at every size. The labels above the car are drawn in a second
// layer in real pixels, so their text stays the same size at every width.
import { useEffect, useId, useMemo, useRef, useState, type CSSProperties } from "react";
import type { BodyStyle } from "../api";
import { CarZone } from "../api";
import { usePageActive } from "../lib/hooks";
import { parseColor, type Rgb } from "../lib/theme";
import { darker, lighter, rgb } from "./diagram/colour";
import { FIELD_H, FIELD_W, build } from "./diagram/geometry";
import { DiagramLabels, layoutChips } from "./diagram/labels";
import { n } from "./diagram/path";
import "./VehicleDiagram.css";

export interface VehicleDiagramProps {
  body: BodyStyle;
  // Paint colour of the body (CSS colour). Default "#4b5878".
  paint?: string;
  // Glass body with the parts inside. Off for the plain picture.
  anatomy?: boolean;
  // A light sweep across the car while something is being read.
  scanning?: boolean;
  // All zones in which something was found.
  affected?: readonly CarZone[];
  // The currently selected zone is highlighted and pulses.
  focus?: CarZone;
  onZoneClick?: (zone: CarZone) => void;
  className?: string;
}

export function VehicleDiagram({
  body,
  paint = "#4b5878",
  anatomy = false,
  scanning = false,
  affected,
  focus = CarZone.Unknown,
  onZoneClick,
  className,
}: VehicleDiagramProps) {
  const rawId = useId();
  const id = rawId.replace(/[^a-zA-Z0-9_-]/g, "");
  const active = usePageActive();
  const geo = useMemo(() => build(body), [body]);
  const s = geo.shape;
  const [hover, setHover] = useState<CarZone>(CarZone.Unknown);
  const box = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });

  // Only the observer: it reports the first size after the browser's own
  // layout, before the picture is painted. Reading clientWidth here forced a
  // layout of the whole window in the middle of the start (a third of the
  // first long task with the CPU four times slower), and a second render.
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const observer = new ResizeObserver(() => {
      const w = el.clientWidth;
      const h = el.clientHeight;
      setSize((s) => (s.w === w && s.h === h ? s : { w, h }));
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // Without the see-through mode nothing can be hovered.
  useEffect(() => {
    if (!anatomy) setHover(CarZone.Unknown);
  }, [anatomy]);

  const hit = useMemo(() => new Set(affected ?? []), [affected]);
  const alpha = anatomy ? 0.32 : 1.0;
  const base = useMemo(() => parseColor(paint), [paint]);
  const wheelY = s.ground - s.wheelRad;
  const rim = s.wheelRad * 0.68;

  // fieldToWidget(): the same fit the viewBox does, for the label layer.
  const scale = Math.min(size.w / FIELD_W, size.h / FIELD_H) || 0;
  const offX = (size.w - FIELD_W * scale) / 2;
  const offY = (size.h - FIELD_H * scale) / 2;
  const mapX = (x: number) => offX + x * scale;
  const mapY = (y: number) => offY + y * scale;
  const chips = anatomy && scale > 0 ? layoutChips(geo.parts, hit, hover, size.w, mapX) : [];

  const len = s.rear - s.front;
  const sweepStyle = {
    "--sweep-from": `${s.front + len * -0.4}px`,
    "--sweep-to": `${s.front + len * 1.4}px`,
  } as CSSProperties;

  const lac = (c: Rgb, a = 1) => ({ stopColor: rgb(c), stopOpacity: alpha * a });
  const lensAlpha = Math.max(alpha, 0.6);
  const ref = (name: string) => `url(#${id}-${name})`;

  // Small parts are hit-tested first: they go on top in the hit layer.
  const hitOrder = [...geo.parts.filter((p) => !p.small), ...geo.parts.filter((p) => p.small)];

  return (
    <div
      ref={box}
      className={`vehicle-diagram${active ? "" : " paused"}${scanning ? " scanning" : ""}${className ? ` ${className}` : ""}`}
    >
      <svg className="vd-car" viewBox={`0 0 ${FIELD_W} ${FIELD_H}`} preserveAspectRatio="xMidYMid meet" aria-hidden>
        <defs>
          <radialGradient id={`${id}-shadow`}>
            <stop offset="0" stopColor="#000" stopOpacity={170 / 255} />
            <stop offset="0.6" stopColor="#000" stopOpacity={60 / 255} />
            <stop offset="1" stopColor="#000" stopOpacity={0} />
          </radialGradient>
          <linearGradient id={`${id}-floor`} x1="0" y1="0" x2="1" y2="0">
            <stop offset="0" style={{ stopColor: "var(--text)", stopOpacity: 0 }} />
            <stop offset="0.5" style={{ stopColor: "var(--text)", stopOpacity: 0.12 }} />
            <stop offset="1" style={{ stopColor: "var(--text)", stopOpacity: 0 }} />
          </linearGradient>
          {/* Wheel gradients in wheel-local coordinates, shared by both wheels. */}
          <radialGradient id={`${id}-tyre`} gradientUnits="userSpaceOnUse" cx="0" cy="0" r={s.wheelRad}>
            <stop offset="0.7" stopColor="rgb(30,33,40)" />
            <stop offset="0.86" stopColor="rgb(17,19,24)" />
            <stop offset="1" stopColor="rgb(10,11,14)" />
          </radialGradient>
          <radialGradient id={`${id}-metal`} gradientUnits="userSpaceOnUse" cx={-rim * 0.3} cy={-rim * 0.4} r={rim * 1.4}>
            <stop offset="0" stopColor="rgb(214,220,232)" />
            <stop offset="0.5" stopColor="rgb(132,140,158)" />
            <stop offset="1" stopColor="rgb(62,68,82)" />
          </radialGradient>
          <linearGradient id={`${id}-calliper`} gradientUnits="userSpaceOnUse" x1="0" y1={-rim * 0.8} x2="0" y2={-rim * 0.3}>
            <stop offset="0" style={{ stopColor: "var(--accent-hover)" }} />
            <stop offset="1" style={{ stopColor: "var(--accent-dim)" }} />
          </linearGradient>
          <linearGradient id={`${id}-spoke`} gradientUnits="userSpaceOnUse" x1={-rim * 0.2} y1="0" x2={rim * 0.2} y2="0">
            <stop offset="0" stopColor="rgb(170,178,194)" />
            <stop offset="1" stopColor="rgb(96,104,120)" />
          </linearGradient>
          <radialGradient id={`${id}-hub`} gradientUnits="userSpaceOnUse" cx="0" cy="0" r={rim * 0.22}>
            <stop offset="0" stopColor="rgb(220,226,236)" />
            <stop offset="1" stopColor="rgb(80,88,104)" />
          </radialGradient>
          {/* Paint: light from above, a bright shoulder line, dark towards the sills. */}
          <linearGradient id={`${id}-lacquer`} gradientUnits="userSpaceOnUse" x1="0" y1={s.roof} x2="0" y2={s.bumper}>
            <stop offset="0" style={lac(lighter(base, 165))} />
            <stop offset="0.3" style={lac(lighter(base, 125))} />
            <stop offset="0.44" style={lac(lighter(base, 150))} />
            <stop offset="0.5" style={lac(base)} />
            <stop offset="0.8" style={lac(darker(base, 160))} />
            <stop offset="1" style={lac(darker(base, 260))} />
          </linearGradient>
          <linearGradient id={`${id}-shine`} gradientUnits="userSpaceOnUse" x1="0" y1={s.belt + 6} x2="0" y2={s.belt + 22}>
            <stop offset="0" stopColor="#fff" stopOpacity={0} />
            <stop offset="0.5" stopColor="#fff" stopOpacity={alpha * 0.16} />
            <stop offset="1" stopColor="#fff" stopOpacity={0} />
          </linearGradient>
          <linearGradient id={`${id}-tint`} gradientUnits="userSpaceOnUse" x1="0" y1={s.roof} x2="0" y2={s.belt}>
            <stop offset="0" stopColor="rgb(54,64,88)" stopOpacity={alpha * (anatomy ? 0.6 : 1)} />
            <stop offset="1" stopColor="rgb(14,17,26)" stopOpacity={alpha * (anatomy ? 0.6 : 1)} />
          </linearGradient>
          <linearGradient id={`${id}-refl`} gradientUnits="userSpaceOnUse" x1={s.roofFront} y1={s.roof} x2={s.roofFront + 140} y2={s.belt}>
            <stop offset="0" stopColor="#fff" stopOpacity={0} />
            <stop offset="0.45" stopColor="#fff" stopOpacity={34 / 255} />
            <stop offset="0.55" stopColor="#fff" stopOpacity={0} />
          </linearGradient>
          <radialGradient id={`${id}-headglow`} gradientUnits="userSpaceOnUse" cx={s.front + 20} cy={s.nose + 2} r="70">
            <stop offset="0" stopColor="rgb(150,200,255)" stopOpacity={(90 * alpha) / 255} />
            <stop offset="1" stopColor="rgb(150,200,255)" stopOpacity={0} />
          </radialGradient>
          <linearGradient id={`${id}-headlens`} gradientUnits="userSpaceOnUse" x1={s.front + 10} y1="0" x2={s.front + 72} y2="0">
            <stop offset="0" stopColor="rgb(235,245,255)" stopOpacity={lensAlpha} />
            <stop offset="1" stopColor="rgb(120,170,255)" stopOpacity={(200 / 255) * lensAlpha} />
          </linearGradient>
          <radialGradient id={`${id}-tailglow`} gradientUnits="userSpaceOnUse" cx={s.rear - 6} cy={s.tail + 14} r="50">
            <stop offset="0" stopColor="rgb(255,70,80)" stopOpacity={(110 * alpha) / 255} />
            <stop offset="1" stopColor="rgb(255,70,80)" stopOpacity={0} />
          </radialGradient>
          <linearGradient id={`${id}-taillens`} gradientUnits="userSpaceOnUse" x1="0" y1={s.tail + 6} x2="0" y2={s.tail + 22}>
            <stop offset="0" stopColor="rgb(255,120,120)" stopOpacity={lensAlpha} />
            <stop offset="1" stopColor="rgb(200,30,50)" stopOpacity={lensAlpha} />
          </linearGradient>
          <linearGradient id={`${id}-rimlight`} gradientUnits="userSpaceOnUse" x1={s.front} y1="0" x2={s.rear} y2="0">
            <stop offset="0" stopColor="#fff" stopOpacity={10 / 255} />
            <stop offset="0.45" stopColor="#fff" stopOpacity={(anatomy ? 70 : 60) / 255} />
            <stop offset="1" stopColor="#fff" stopOpacity={10 / 255} />
          </linearGradient>
          <linearGradient id={`${id}-sweep`} gradientUnits="userSpaceOnUse" x1="-60" y1="0" x2="60" y2="0">
            <stop offset="0" stopColor="#fff" stopOpacity={0} />
            {scanning ? (
              <stop offset="0.5" style={{ stopColor: "var(--accent)", stopOpacity: 70 / 255 }} />
            ) : (
              <stop offset="0.5" stopColor="#fff" stopOpacity={26 / 255} />
            )}
            <stop offset="1" stopColor="#fff" stopOpacity={0} />
          </linearGradient>
          <clipPath id={`${id}-body`}>
            <path d={geo.body} />
          </clipPath>
          <clipPath id={`${id}-glassclip`}>
            <path d={geo.glass} />
          </clipPath>
          <mask id={`${id}-pillars`} maskUnits="userSpaceOnUse" x="0" y="0" width={FIELD_W} height={FIELD_H}>
            <rect x="0" y="0" width={FIELD_W} height={FIELD_H} fill="#fff" />
            {geo.cuts ? <path d={geo.cuts} fill="#000" /> : null}
          </mask>
          {geo.parts
            .filter((p) => p.line)
            .map((p) => (
              // The outline of a thick line: a wide stroke minus a narrow one.
              <mask key={p.zone} id={`${id}-tube-${p.zone}`} maskUnits="userSpaceOnUse" x="0" y="0" width={FIELD_W} height={FIELD_H}>
                <path d={p.d} fill="none" stroke="#fff" strokeWidth={(p.line ?? 0) + 1.2} />
                <path d={p.d} fill="none" stroke="#000" strokeWidth={Math.max(0.5, (p.line ?? 0) - 1.2)} />
              </mask>
            ))}
        </defs>

        {/* Ground: a soft shadow and a faint floor line that fades out to the sides. */}
        <ellipse
          cx={(s.front + s.rear) / 2}
          cy={s.ground}
          rx={len * 0.62}
          ry={len * 0.62 * 0.07}
          fill={ref("shadow")}
        />
        <line x1="10" y1={s.ground + 1} x2={FIELD_W - 10} y2={s.ground + 1} stroke={ref("floor")} strokeWidth="1.2" />

        {/* Wheels */}
        {[s.wheelF, s.wheelR].map((cx) => (
          <g key={cx} transform={`translate(${n(cx)} ${n(wheelY)})`}>
            <circle r={s.wheelRad + 6} fill="rgb(6,7,10)" />
            <circle r={s.wheelRad} fill={ref("tyre")} />
            <circle r={rim} fill={ref("metal")} />
            {/* Brake disc and calliper visible between the spokes */}
            <circle r={rim * 0.86} fill="rgb(40,44,54)" />
            <rect
              transform="rotate(-35)"
              x={-rim * 0.22}
              y={-rim * 0.82}
              width={rim * 0.44}
              height={rim * 0.38}
              rx="4"
              fill={ref("calliper")}
            />
            {/* Five spokes */}
            {[0, 1, 2, 3, 4].map((i) => (
              <path
                key={i}
                transform={`rotate(${i * 72 + 18})`}
                d={`M${n(-rim * 0.1)} ${n(-rim * 0.18)}L${n(-rim * 0.2)} ${n(-rim * 0.95)}L${n(rim * 0.2)} ${n(-rim * 0.95)}L${n(rim * 0.1)} ${n(-rim * 0.18)}Z`}
                fill={ref("spoke")}
              />
            ))}
            {/* Rim lip and hub */}
            <circle r={rim} fill="none" stroke="rgb(190,198,214)" strokeOpacity={160 / 255} strokeWidth="1.6" />
            <circle r={rim * 0.2} fill={ref("hub")} />
          </g>
        ))}

        {/* Body */}
        <g className="vd-body">
          <path d={geo.body} fill={ref("lacquer")} />
          <g clipPath={ref("body")}>
            {/* Dark lower band (sills, bumpers) and the shoulder highlight */}
            <rect x="0" y={s.sill - 12} width={FIELD_W} height="80" fill="rgb(14,16,22)" fillOpacity={alpha * 0.85} />
            <rect x="0" y={s.belt + 6} width={FIELD_W} height="16" fill={ref("shine")} />
          </g>
          {/* Windows, with a diagonal reflection on the glass */}
          <g mask={ref("pillars")}>
            <path d={geo.glass} fill={ref("tint")} />
            {!anatomy ? (
              <rect x="0" y="0" width={FIELD_W} height={FIELD_H} fill={ref("refl")} clipPath={ref("glassclip")} />
            ) : null}
          </g>
          {/* Door lines and handles */}
          {geo.doorLines.map((c, i) => (
            <path
              key={i}
              d={`M${n(c.x0)} ${n(c.y0)}C${n(c.x1)} ${n(c.y1)} ${n(c.x2)} ${n(c.y2)} ${n(c.x3)} ${n(c.y3)}`}
              fill="none"
              stroke="#000"
              strokeOpacity={alpha * 0.55}
              strokeWidth="1.4"
            />
          ))}
          {geo.handles.map((x, i) => (
            <rect key={i} x={x} y={s.belt + 18} width="22" height="5" rx="2.5" fill="rgb(10,12,16)" fillOpacity={alpha * 0.7} />
          ))}
          {/* Head light with a glow */}
          <ellipse cx={s.front + 20} cy={s.nose + 2} rx="70" ry="40" fill={ref("headglow")} />
          <path
            d={`M${n(s.front + 10)} ${n(s.nose + 6)}C${n(s.front + 26)} ${n(s.nose - 6)} ${n(s.front + 60)} ${n(s.nose - 8)} ${n(s.front + 72)} ${n(s.nose - 6)}L${n(s.front + 64)} ${n(s.nose + 8)}C${n(s.front + 44)} ${n(s.nose + 12)} ${n(s.front + 22)} ${n(s.nose + 14)} ${n(s.front + 10)} ${n(s.nose + 6)}Z`}
            fill={ref("headlens")}
          />
          {/* Tail light */}
          <ellipse cx={s.rear - 4} cy={s.tail + 14} rx="50" ry="26" fill={ref("tailglow")} />
          <rect x={s.rear - 34} y={s.tail + 6} width="32" height="15" rx="5" fill={ref("taillens")} />
          {/* Rim light along the top edge */}
          <path d={geo.body} fill="none" stroke={ref("rimlight")} strokeWidth={anatomy ? 1.6 : 1.2} />
        </g>

        {/* The parts behind the faults */}
        {anatomy ? (
          <g className="vd-parts">
            {geo.parts.map((part) => {
              const isHit = hit.has(part.zone);
              const focused = part.zone === focus;
              const hovered = part.zone === hover;
              const tone = isHit ? "var(--bad)" : "var(--accent)";
              const fillAlpha = isHit ? 0.28 : hovered ? 0.22 : 0.08;
              const lineAlpha = isHit ? 1 : hovered ? 0.95 : 0.45;
              const lineWidth = isHit ? 1.8 : 1.2;
              const pulsing = isHit && focused;
              return (
                <g key={part.zone} className={`vd-part${pulsing ? " pulsing" : ""}`}>
                  {pulsing ? (
                    // A soft glow around the part in focus.
                    <path
                      className="vd-halo"
                      d={part.d}
                      style={{ fill: "none", stroke: "var(--bad)", "--halo-extra": `${part.line ?? 0}px` } as CSSProperties}
                      strokeLinejoin="round"
                      strokeLinecap="round"
                    />
                  ) : null}
                  {part.line ? (
                    <>
                      <path
                        className="vd-tubefill"
                        d={part.d}
                        style={{ fill: "none", stroke: tone, strokeOpacity: fillAlpha, strokeWidth: part.line }}
                      />
                      <path
                        className="vd-line"
                        d={part.d}
                        mask={ref(`tube-${part.zone}`)}
                        style={{ fill: "none", stroke: tone, strokeOpacity: lineAlpha, strokeWidth: part.line + lineWidth }}
                      />
                    </>
                  ) : (
                    <path
                      className="vd-shape"
                      d={part.d}
                      style={{ fill: tone, fillOpacity: fillAlpha, stroke: tone, strokeOpacity: lineAlpha, strokeWidth: lineWidth }}
                    />
                  )}
                </g>
              );
            })}
          </g>
        ) : null}

        {/* A light band that runs over the car; bright while reading. */}
        <g clipPath={ref("body")}>
          <rect key={scanning ? "scan" : "idle"} className="vd-sweep" style={sweepStyle} x="-60" y="0" width="120" height={FIELD_H} fill={ref("sweep")} />
        </g>

        {/* Hit areas: the part itself or 5 units around it, small parts on top. */}
        {anatomy ? (
          <g className="vd-hits">
            {hitOrder.map((part) => (
              <path
                key={part.zone}
                d={part.d}
                className="vd-hit"
                fill={part.line ? "none" : "transparent"}
                stroke="transparent"
                strokeWidth={(part.line ?? 0) + 10}
                strokeLinejoin="round"
                onPointerEnter={() => setHover(part.zone)}
                onPointerLeave={() => setHover((h) => (h === part.zone ? CarZone.Unknown : h))}
                onClick={() => onZoneClick?.(part.zone)}
              />
            ))}
          </g>
        ) : null}
      </svg>

      {chips.length > 0 ? <DiagramLabels chips={chips} focus={focus} size={size} mapX={mapX} mapY={mapY} /> : null}
    </div>
  );
}
