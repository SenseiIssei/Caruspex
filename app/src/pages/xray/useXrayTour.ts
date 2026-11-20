// The findings of the X-ray view and the guided check that flies from one
// to the next.
import { useEffect, useMemo, useRef, useState, type Dispatch, type RefObject, type SetStateAction } from "react";
import { call } from "../../api";
import type { LiveRow } from "../../api";
import { CarZone } from "../../api";
import { localeNumber, translator } from "../../i18n";
import { useBackendEvent, useInterval } from "../../lib/hooks";
import { zoneAnchor, type CarLayout } from "../../xray/layout";
import type { XrayScene } from "../../xray/scene";
import { LIVE_KEYS, zoneLabel } from "./names";
import type { Finding, ZoneFinding } from "./support";

const tr = translator("caruspex::XrayPage");

export function useXrayTour({
  layout,
  zoneFindings,
  liveRows,
  connected,
  active,
  reduced,
  sceneRef,
  setSelected,
}: {
  layout: CarLayout | null;
  zoneFindings: ZoneFinding[];
  liveRows: LiveRow[];
  connected: boolean;
  active: boolean;
  reduced: boolean;
  sceneRef: RefObject<XrayScene | null>;
  setSelected: Dispatch<SetStateAction<string | null>>;
}) {
  const findings = useMemo<Finding[]>(() => {
    if (!layout) return [];
    const out: Finding[] = [];
    for (const z of zoneFindings) {
      out.push({ id: `zone-${z.zone}`, anchor: zoneAnchor(layout, z.zone), title: z.info.name || zoneLabel(z.zone), text: `${z.codes.join(", ")}. ${z.info.hint}`.trim(), zone: z.zone, ecu: null, tone: "bad" });
    }
    for (const e of layout.ecus) {
      if (!e.faulty || e.asking) continue;
      out.push({ id: `ecu-${e.key}`, anchor: e.position, title: e.name, text: tr("Dieses Steuergerät hat Fehler gespeichert."), zone: null, ecu: e.key, tone: "bad" });
    }
    for (const row of liveRows) {
      if (!row.warn) continue;
      const key = LIVE_KEYS.find((k) => k.pid === row.pid)!;
      const anchor = key.anchor === "wheel" ? zoneAnchor(layout, CarZone.Wheels) : layout.anchors[key.anchor];
      if (anchor) out.push({ id: `live-${row.pid}`, anchor, title: row.name, text: `${localeNumber(row.value, key.decimals)} ${row.unit}`, zone: null, ecu: null, tone: "warn" });
    }
    return out;
  }, [layout, zoneFindings, liveRows]);

  const [tour, setTour] = useState<number | null>(null);
  const waitingForFaults = useRef(false);
  const startCheck = () => {
    setSelected(null);
    if (connected) {
      waitingForFaults.current = true;
      void call("read_faults");
      // Start anyway if the car does not answer in time.
      setTimeout(() => {
        if (!waitingForFaults.current) return;
        waitingForFaults.current = false;
        setTour(0);
      }, 4000);
    } else {
      setTour(0);
    }
  };
  useBackendEvent("faults", () => {
    if (!waitingForFaults.current) return;
    waitingForFaults.current = false;
    // The zones are looked up right after the codes arrive.
    setTimeout(() => setTour(0), 700);
  });
  const steps = findings.length > 0 ? findings : null;
  const step = tour !== null && steps ? steps[Math.min(tour, steps.length - 1)]! : null;
  useEffect(() => {
    const scene = sceneRef.current;
    if (tour === null || !scene) return;
    if (step) scene.flyTo(step.anchor);
    else scene.resetView();
  }, [tour, step?.id]);
  useInterval(
    () => setTour((i) => (i === null ? null : steps && i + 1 < steps.length ? i + 1 : i)),
    tour !== null && steps && tour + 1 < steps.length && active ? (reduced ? 7000 : 4800) : null,
  );
  const endTour = () => {
    setTour(null);
    sceneRef.current?.resetView();
  };

  return { tour, setTour, steps, step, startCheck, endTour };
}
