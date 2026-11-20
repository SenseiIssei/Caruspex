// The three.js scene of the X-ray view: loaded on the first visit, then fed
// with the car, the network and the packets.
import { useEffect, useRef, useState, type RefObject } from "react";
import type { CarLayout } from "../../xray/layout";
import type { FrameStats, SectionAxis, XrayScene } from "../../xray/scene";
import { readPalette } from "./support";

export function useXrayScene({
  active,
  reduced,
  layout,
  carKey,
  devView,
  pendingPacket,
}: {
  active: boolean;
  reduced: boolean;
  layout: CarLayout | null;
  carKey: string;
  devView: "side" | "rear" | "front" | null;
  pendingPacket: RefObject<string | null>;
}) {
  const host = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<XrayScene | null>(null);
  const [mode, setMode] = useState<"idle" | "loading" | "3d" | "2d">("idle");
  const [hovered, setHovered] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const lastCar = useRef("");
  const pickRef = useRef<(id: string | null) => void>(() => undefined);

  // The section plane (dragged in the view or with the slider) and the
  // exploded drivetrain.
  const [section, setSection] = useState<{ axis: SectionAxis; t: number } | null>(null);
  // The frame counter of the screenshot option ("stats").
  const [stats, setStats] = useState<FrameStats | null>(null);

  const started = useRef(false);
  const unmounted = useRef(false);
  useEffect(() => {
    if (!active || started.current) return;
    started.current = true;
    setMode("loading");
    void import("../../xray/scene").then(
      (mod) => {
        if (unmounted.current || !host.current || sceneRef.current) return;
        if (!mod.webglAvailable()) {
          setMode("2d");
          return;
        }
        try {
          sceneRef.current = new mod.XrayScene(host.current, readPalette(), reduced, {
            onHover: (id) => setHovered(id),
            onPick: (id) => pickRef.current(id),
            onSection: (t) => setSection((s) => (s ? { ...s, t } : s)),
            onStats: (s) => setStats(s),
          });
          setMode("3d");
          // The development server lends the scene to the measurements in
          // docs/performance.md (renderer counts); a release has no such hook.
          if (import.meta.env.DEV) (window as { __caruspexXray?: XrayScene }).__caruspexXray = sceneRef.current;
        } catch {
          setMode("2d");
        }
      },
      () => !unmounted.current && setMode("2d"),
    );
    // The scene is made once; reduced motion is read at that moment.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  useEffect(() => {
    unmounted.current = false;
    return () => {
      unmounted.current = true;
      started.current = false;
      lastCar.current = "";
      sceneRef.current?.dispose();
      sceneRef.current = null;
    };
  }, []);
  useEffect(() => sceneRef.current?.setActive(active), [active, mode]);

  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene || !layout || mode !== "3d") return;
    if (carKey !== lastCar.current) {
      const sameCar = lastCar.current.split("|")[0] === carKey.split("|")[0];
      if (sameCar) scene.setPalette(readPalette());
      scene.setCar(layout, sameCar);
      lastCar.current = carKey;
    } else {
      scene.setNetwork(layout);
    }
    if (pendingPacket.current) {
      scene.sendPacket(pendingPacket.current);
      pendingPacket.current = null;
    }
  }, [layout, carKey, mode]);

  // The straight view of the screenshot option, once the car is drawn.
  useEffect(() => {
    if (!devView || mode !== "3d" || !carKey) return;
    // Again a little later: the car may be redrawn once more while the demo
    // connection settles.
    const timers = [300, 2000, 5000].map((ms) => setTimeout(() => sceneRef.current?.viewFrom(devView), ms));
    return () => timers.forEach(clearTimeout);
  }, [devView, carKey, mode]);

  return { host, sceneRef, mode, hovered, selected, setSelected, pickRef, section, setSection, stats };
}
