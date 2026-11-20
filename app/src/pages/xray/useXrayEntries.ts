// Entries into the X-ray view: a zone from another page, and the switches
// of the screenshot option and of --page (xray-generation, xray-scan, ...),
// which the command line and the picture tools use.
import { useEffect, useRef, type Dispatch, type RefObject, type SetStateAction } from "react";
import { CarZone } from "../../api";
import { useAppEvent } from "../../lib/hooks";
import { onApp } from "../../lib/store";
import { navigate } from "../../shell/nav";
import { zoneAnchor, type CarLayout } from "../../xray/layout";
import type { XrayScene } from "../../xray/scene";

export function useXrayEntries({
  layout,
  setTour,
  sceneRef,
  devFlags,
  setDevFlags,
  connected,
  scanQueue,
  startScan,
  showGeneration,
  setSessionModel,
  setDevView,
  startCheck,
  liveOn,
  toggleLive,
}: {
  layout: CarLayout | null;
  setTour: Dispatch<SetStateAction<number | null>>;
  sceneRef: RefObject<XrayScene | null>;
  devFlags: string[];
  setDevFlags: Dispatch<SetStateAction<string[]>>;
  connected: boolean;
  scanQueue: RefObject<string[]>;
  startScan: () => Promise<void>;
  showGeneration: (arg: string) => void;
  setSessionModel: (model: { path: string; turned: boolean }) => void;
  setDevView: (view: "side" | "rear" | "front") => void;
  startCheck: () => void;
  liveOn: boolean;
  toggleLive: () => void;
}) {
  // --- Entries from other pages ------------------------------------------------------------------
  useEffect(
    () =>
      onApp<CarZone>("xray:zone", (zone) => {
        if (!layout) return;
        setTour(null);
        setTimeout(() => sceneRef.current?.flyTo(zoneAnchor(layout, zone)), 350);
      }),
    [layout],
  );

  // The switches "scan" and "scan-all" of xray-generation: the units of the
  // connected (demo) car on the generation shown, once connected.
  const devScanned = useRef(false);
  useEffect(() => {
    const all = devFlags.includes("scan-all");
    if (devScanned.current || !connected || !(all || devFlags.includes("scan"))) return;
    devScanned.current = true;
    if (all) scanQueue.current = ["FORD@MS", "GM@SW"];
    setTimeout(() => void startScan(), 1500);
    // startScan only needs to exist; the scan runs once.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [devFlags, connected]);

  // "xray-scan", "xray-check" and "xray-live" of --page, for pictures.
  useAppEvent<string>("dev:page", (name) => {
    if (!name.startsWith("xray")) return;
    navigate("xray");
    if (name.startsWith("xray-generation:")) showGeneration(name.slice("xray-generation:".length));
    // A model file for pictures of the own-model view, without the dialog.
    if (name.startsWith("xray-model:")) {
      const [path, view] = name.slice("xray-model:".length).split("|");
      setSessionModel({ path: path ?? "", turned: false });
      if (view === "side" || view === "rear" || view === "front") setDevView(view);
    }
    if (name === "xray-scan") setTimeout(() => void startScan(), 600);
    // Every bus one after the other: the CAN of the socket, then the MS-CAN
    // and the SW-CAN (with CARUSPEX_DEMO_ADAPTER=obdlink-mx for the demo).
    if (name === "xray-scan-all") {
      scanQueue.current = ["FORD@MS", "GM@SW"];
      setTimeout(() => void startScan(), 600);
    }
    if (name === "xray-stats") setDevFlags((f) => [...f, "stats"]);
    if (name === "xray-check") setTimeout(startCheck, 1200);
    if (name === "xray-live") setTimeout(() => !liveOn && toggleLive(), 600);
  });
}
