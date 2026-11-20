// What the scene highlights and the labels it shows: fault zones, live
// values, bus names and the part under the pointer.
import { useEffect, type RefObject } from "react";
import type { LiveRow } from "../../api";
import { CarZone } from "../../api";
import { localeNumber } from "../../i18n";
import { zoneAnchor, type CarLayout, type Layer, type Vec3 } from "../../xray/layout";
import type { XrayLabel, XrayScene } from "../../xray/scene";
import { LIVE_KEYS, busName, partName, zoneLabel } from "./names";
import type { Finding, ZoneFinding } from "./support";

export function useXrayLabels({
  sceneRef,
  layout,
  zones,
  zoneFindings,
  liveRows,
  step,
  selected,
  hovered,
  mode,
  layers,
}: {
  sceneRef: RefObject<XrayScene | null>;
  layout: CarLayout | null;
  zones: CarZone[];
  zoneFindings: ZoneFinding[];
  liveRows: LiveRow[];
  step: Finding | null;
  selected: string | null;
  hovered: string | null;
  mode: "idle" | "loading" | "3d" | "2d";
  layers: Record<Layer, boolean>;
}): string | null {
  const selectedEcu = selected?.startsWith("ecu:") ? selected.slice(4) : null;
  useEffect(() => {
    sceneRef.current?.setHighlight({
      zones,
      focusZone: step?.zone ?? null,
      focusEcu: step?.ecu ?? null,
      selectedEcu,
    });
  }, [zones, step?.zone, step?.ecu, selectedEcu, mode, layout]);

  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene || !layout) return;
    const labels: XrayLabel[] = [];
    for (const z of zoneFindings) {
      labels.push({ id: `zone-${z.zone}`, anchor: zoneAnchor(layout, z.zone), title: zoneLabel(z.zone), value: z.codes.join(" · "), tone: "bad" });
    }
    for (const row of liveRows) {
      const key = LIVE_KEYS.find((k) => k.pid === row.pid)!;
      const anchor = key.anchor === "wheel" ? zoneAnchor(layout, CarZone.Wheels) : layout.anchors[key.anchor];
      if (!anchor) continue;
      const value = row.text ? row.text : `${localeNumber(row.value, key.decimals)} ${row.unit}`.trim();
      labels.push({ id: `live-${row.pid}`, anchor: [anchor[0], anchor[1] + 0.05, anchor[2]], title: row.name, value, tone: row.warn ? "warn" : "info" });
    }
    // The name of every bus on its line.
    if (layers.network) {
      for (const b of layout.busLabels) labels.push({ id: `bus-${b.bus}`, anchor: b.anchor, title: busName(b.bus), tone: "bus", bus: b.bus });
    }
    const pointed = hovered ?? selected;
    if (pointed) {
      if (pointed.startsWith("ecu:")) {
        const e = layout.ecus.find((n) => n.key === pointed.slice(4));
        if (e) labels.push({ id: "pointed", anchor: [e.position[0], e.position[1] + 0.06, e.position[2]], title: e.name, value: busName(e.bus), tone: e.faulty ? "bad" : "accent" });
      } else {
        const part = layout.parts.find((p) => p.id === pointed);
        const tube = layout.tubes.find((p) => p.id === pointed);
        const at = part
          ? ([part.center[0], part.center[1] + part.size[1] / 2 + 0.03, part.center[2]] as Vec3)
          : tube
            ? tube.points[Math.floor((tube.points.length - 1) / 2)]!
            : pointed.startsWith("obd")
              ? layout.adapter.socket
              : null;
        if (at && !zoneFindings.some((z) => (part?.zones ?? tube?.zones ?? []).includes(z.zone))) {
          labels.push({ id: "pointed", anchor: at, title: partName(pointed), tone: "accent" });
        }
      }
    }
    scene.setLabels(labels);
  }, [layout, zoneFindings, liveRows, hovered, selected, mode, layers.network]);

  return selectedEcu;
}
