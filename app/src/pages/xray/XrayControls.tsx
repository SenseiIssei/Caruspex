// The controls over the 3D view: layers, cuts, the bus legend, the frame
// counter and the help line.
import type { CSSProperties, Dispatch, RefObject, SetStateAction } from "react";
import { Button, Chip } from "../../components/ui";
import { localeNumber, translator } from "../../i18n";
import { LAYERS, type BusKind, type Layer } from "../../xray/layout";
import type { FrameStats, SectionAxis, XrayScene } from "../../xray/scene";
import { busName, layerName } from "./names";

const tr = translator("caruspex::XrayPage");
// A phone or tablet: the help names fingers instead of the mouse.
const touchScreen = typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia("(pointer: coarse)").matches;

export function XrayControls({
  layers,
  setLayers,
  energyName,
  isMoto,
  sceneRef,
  exploded,
  setExploded,
  section,
  setSection,
  busesShown,
  stats,
}: {
  layers: Record<Layer, boolean>;
  setLayers: Dispatch<SetStateAction<Record<Layer, boolean>>>;
  energyName: string;
  isMoto: boolean;
  sceneRef: RefObject<XrayScene | null>;
  exploded: boolean;
  setExploded: Dispatch<SetStateAction<boolean>>;
  section: { axis: SectionAxis; t: number } | null;
  setSection: Dispatch<SetStateAction<{ axis: SectionAxis; t: number } | null>>;
  busesShown: BusKind[];
  stats: FrameStats | null;
}) {
  return (
    <>
      <div className="xray-layers" role="group" aria-label={tr("Ebenen")}>
        {LAYERS.map((layer) => (
          <Chip key={layer} on={layers[layer]} onClick={() => setLayers((l) => ({ ...l, [layer]: !l[layer] }))}>
            {layerName(layer, energyName, isMoto)}
          </Chip>
        ))}
      </div>
      <div className="xray-view-buttons">
        <Button small icon="refresh" title={tr("Ansicht zurücksetzen")} onClick={() => sceneRef.current?.resetView()} />
      </div>
      <div className="xray-tools" role="group" aria-label={tr("Schnitte")}>
        <Chip on={exploded} onClick={() => setExploded((e) => !e)} title={tr("Motor, Getriebe und Auspuff auseinanderziehen")}>
          {tr("Antrieb zerlegen")}
        </Chip>
        <Chip on={section !== null} onClick={() => setSection((s) => (s ? null : { axis: "x", t: 0.55 }))} title={tr("Eine Ebene durch das Fahrzeug ziehen, die alles davor ausblendet")}>
          {tr("Schnitt")}
        </Chip>
        {section ? (
          <>
            {(["x", "z", "y"] as SectionAxis[]).map((axis) => (
              <Chip key={axis} on={section.axis === axis} onClick={() => setSection({ axis, t: section.axis === axis ? section.t : 0.5 })}>
                {axis === "x" ? tr("Quer") : axis === "z" ? tr("Längs") : tr("Von oben")}
              </Chip>
            ))}
            <input
              className="xray-section-slider"
              type="range"
              min={2}
              max={98}
              value={Math.round(section.t * 100)}
              aria-label={tr("Schnittebene verschieben")}
              title={tr("Schnittebene verschieben")}
              onChange={(e) => setSection({ axis: section.axis, t: Number(e.target.value) / 100 })}
            />
          </>
        ) : null}
      </div>
      {(layers.network && busesShown.length > 0) || layers.electrics ? (
        <div className="xray-legend">
          {layers.network
            ? busesShown.map((bus) => (
                <span key={bus} className="xray-legend-item">
                  <span className="xray-legend-line" style={{ "--bus": `var(--bus-${bus})` } as CSSProperties} />
                  {busName(bus)}
                </span>
              ))
            : null}
          {layers.electrics ? <span className="xray-legend-note">{tr("Kabelbaum vereinfacht: er zeigt, wo die Stränge laufen, kein Stromlaufplan.")}</span> : null}
        </div>
      ) : null}
      {stats ? (
        <div className="xray-stats hint">
          {tr("%1 Bilder pro Sekunde, %2 ms je Bild (%3 ms Zeichnen), %4 Zeichenaufrufe", localeNumber(stats.fps, 0), localeNumber(stats.frameMs, 1), localeNumber(stats.renderMs, 1), stats.calls)}
        </div>
      ) : null}
      <div className="xray-help hint">
        {touchScreen
          ? tr("Ziehen dreht, zwei Finger zoomen und verschieben, Tippen zeigt ein Teil.")
          : tr("Ziehen dreht, Mausrad zoomt, rechte Maustaste verschiebt.")}
      </div>
    </>
  );
}
