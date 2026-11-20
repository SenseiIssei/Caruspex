// The X-ray view (Phase 20): the connected car as a turnable 3D model with a
// glass body. Engine, drivetrain, tank or battery, the control units the
// scan found and the bus lines between them sit where they sit in a real
// car; faults light up their zone, live values hang on their part and
// packets run along the lines to the unit that is being read.
//
// three.js loads only when this page is opened for the first time. Without
// WebGL (some remote desktops, a blocked GPU) the page shows the 2D side
// view with its see-through mode instead.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { call } from "../api";
import { BodyType, CarZone, Powertrain } from "../api";
import { Button, Page, PageHeader } from "../components/ui";
import { VehicleDiagram } from "../components/VehicleDiagram";
import { t, translator } from "../i18n";
import { useBackendEvent, usePageActive } from "../lib/hooks";
import { useThemeVersion } from "../lib/theme";
import { bodyStyleFor } from "../lib/vehicleIdentity";
import { useSession } from "../state/session";
import { EcuKind, LAYERS, buildLayout, type BusKind, type CarLayout, type Layer } from "../xray/layout";
import type { SectionAxis } from "../xray/scene";
import { bodyVariants } from "../xray/shape";
import { useReducedMotion } from "./xray/support";
import { useXrayCar } from "./xray/useXrayCar";
import { useXrayUnits } from "./xray/useXrayUnits";
import { useXrayScene } from "./xray/useXrayScene";
import { useOwnModel } from "./xray/useOwnModel";
import { useXrayLive } from "./xray/useXrayLive";
import { useXrayTour } from "./xray/useXrayTour";
import { useXrayLabels } from "./xray/useXrayLabels";
import { useXrayEntries } from "./xray/useXrayEntries";
import { XrayControls } from "./xray/XrayControls";
import { XrayTour } from "./xray/XrayTour";
import { XrayCarCard, XrayEcuCard } from "./xray/XraySide";
import "./XrayPage.css";

const tr = translator("caruspex::XrayPage");

export default function XrayPage() {
  const active = usePageActive();
  const connected = useSession((s) => s.connected);
  const identity = useSession((s) => s.identity);
  const vin = useSession((s) => s.vin);
  const faults = useSession((s) => s.faults);
  const udsEcus = useSession((s) => s.udsEcus);
  const j1939Units = useSession((s) => s.j1939Units);
  const protocol = useSession((s) => s.protocol);
  const supported = useSession((s) => s.supportedPids);
  const themeVersion = useThemeVersion();
  const reduced = useReducedMotion();

  // The unit the next packet goes to: set by a scan, sent once the scene has
  // the unit drawn.
  const pendingPacket = useRef<string | null>(null);

  // --- Which car -------------------------------------------------------------------
  const { shown, generation, body, variant, setVariant, motoStyle, setMotoStyle, heavyKind, setHeavyKind, setPowertrain, anatomy, devView, setDevView, devFlags, setDevFlags, showGeneration } =
    useXrayCar({ connected, identity, vin });

  // --- Control units and the zones of the faults ---------------------------------------
  const { scanning, kline, scanQueue, ecuInputs, zoneFindings, zones } = useXrayUnits({ connected, protocol, faults, udsEcus, j1939Units, pendingPacket });

  // --- Layout ----------------------------------------------------------------------------
  const turbo = (anatomy?.powertrain ?? 0) === Powertrain.Diesel || zones.includes(CarZone.Turbo);
  const variants = useMemo(() => (anatomy ? bodyVariants(anatomy, body) : [body]), [anatomy, body]);
  const drawn = variant !== null && variants.includes(variant) ? variant : body;
  const isMoto = body === BodyType.Motorcycle;
  const isHeavy = body === BodyType.Truck;
  const layout = useMemo<CarLayout | null>(
    () => (anatomy ? buildLayout({ anatomy, body, variant: drawn, ecus: ecuInputs, kline, turbo, motoStyle: motoStyle ?? undefined, heavyKind: heavyKind ?? undefined }) : null),
    [anatomy, body, drawn, ecuInputs, kline, turbo, motoStyle, heavyKind],
  );
  const carKey = anatomy ? `${JSON.stringify(anatomy)}|${body}|${drawn}|${motoStyle}|${heavyKind}|${turbo}|${themeVersion}` : "";

  // --- The scene -----------------------------------------------------------------------
  const { host, sceneRef, mode, hovered, selected, setSelected, pickRef, section, setSection, stats } = useXrayScene({
    active,
    reduced,
    layout,
    carKey,
    devView,
    pendingPacket,
  });

  // --- The owner's own 3D model --------------------------------------------------------
  const { garageCar, setSessionModel, ownModel, modelState, keepModel, pickModel } = useOwnModel({ vin, shown, sceneRef, mode, layout });

  // Packets for everything the OBD side reads.
  const engineKey = useMemo(() => ecuInputs.find((e) => e.kind === EcuKind.Engine)?.key ?? "obd-engine", [ecuInputs]);
  const lastPacket = useRef(0);
  const obdPacket = useCallback(() => {
    const now = performance.now();
    if (now - lastPacket.current < 160) return;
    lastPacket.current = now;
    sceneRef.current?.sendPacket(engineKey);
  }, [engineKey]);
  useBackendEvent("live_value", obdPacket);
  useBackendEvent("faults", obdPacket);
  useBackendEvent("vehicle_info", obdPacket);
  useBackendEvent("status", obdPacket);
  useBackendEvent("freeze_frames", obdPacket);

  // --- Layers and cuts --------------------------------------------------------------------------
  const [layers, setLayers] = useState<Record<Layer, boolean>>({ body: true, drivetrain: true, cooling: true, energy: true, electrics: true, network: true });
  useEffect(() => sceneRef.current?.setLayers(layers), [layers, mode]);
  useEffect(() => sceneRef.current?.setSection(section), [section, mode, carKey]);
  const [exploded, setExploded] = useState(false);
  useEffect(() => sceneRef.current?.setExplode(exploded), [exploded, mode, carKey]);
  const statsOn = devFlags.includes("stats");
  // "bare": the car alone over the whole window, without the page and the
  // controls, for the pictures of tools/xray_shots.py.
  const bare = devFlags.includes("bare");
  useEffect(() => sceneRef.current?.setStats(statsOn), [statsOn, mode]);
  useEffect(() => {
    for (const flag of devFlags) {
      if (flag === "explode") setExploded(true);
      const cut = flag.match(/^section=([xyz]):([\d.]+)$/);
      if (cut) setSection({ axis: cut[1] as SectionAxis, t: Number(cut[2]) });
      const hide = flag.match(/^hide=([a-z,]+)$/);
      if (hide) setLayers((l) => ({ ...l, ...Object.fromEntries(hide[1]!.split(",").filter((x) => (LAYERS as string[]).includes(x)).map((x) => [x, false])) }));
    }
  }, [devFlags]);
  const energyName =
    anatomy && anatomy.battery !== 0 ? (anatomy.fuelTank ? tr("Kraftstoff und Hochvolt") : tr("Hochvolt")) : tr("Kraftstoff");

  // --- Live values on the parts --------------------------------------------------------
  const { liveRows, liveOn, toggleLive } = useXrayLive({ active, connected, supported });

  // --- Findings and the guided check ---------------------------------------------------
  const { tour, setTour, steps, step, startCheck, endTour } = useXrayTour({
    layout,
    zoneFindings,
    liveRows,
    connected,
    active,
    reduced,
    sceneRef,
    setSelected,
  });

  // --- Picking ----------------------------------------------------------------------------------
  pickRef.current = (id) => {
    const scene = sceneRef.current;
    if (!id || !scene || !layout) {
      setSelected(null);
      return;
    }
    setTour(null);
    setSelected(id);
    if (id.startsWith("ecu:")) {
      const e = layout.ecus.find((n) => n.key === id.slice(4));
      if (e) scene.flyTo(e.position, 2.4);
      return;
    }
    const part = layout.parts.find((p) => p.id === id);
    const tube = layout.tubes.find((p) => p.id === id);
    const at = part?.center ?? tube?.points[Math.floor((tube.points.length - 1) / 2)] ?? (id.startsWith("obd") ? layout.adapter.socket : null);
    if (at) scene.flyTo(at, 2.8);
  };

  // --- Highlight and labels ------------------------------------------------------------
  const selectedEcu = useXrayLabels({ sceneRef, layout, zones, zoneFindings, liveRows, step, selected, hovered, mode, layers });

  // --- Scan ---------------------------------------------------------------------------------------
  const brand = identity?.brand ?? "";
  const startScan = useCallback(async () => {
    if (!connected) return;
    const family = await call("brand_family", { brand }).catch(() => "");
    void call("scan_uds", { family });
  }, [connected, brand]);

  // Entries from other pages and the switches of the command line.
  useXrayEntries({
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
  });

  const busesShown = useMemo(() => {
    const set = new Set<BusKind>();
    for (const r of layout?.routes ?? []) set.add(r.bus);
    return [...set];
  }, [layout]);

  const subtitle = tr("Durch die Karosserie sehen: wo Motor, Tank oder Akku und die Steuergeräte sitzen, und was Caruspex gerade liest.");

  return (
    <Page className="xray-page" wide>
      <PageHeader
        title={tr("Röntgenansicht")}
        subtitle={subtitle}
        actions={
          <>
            <Button icon="cpu" disabled={!connected || !!scanning} onClick={() => void startScan()}>
              {t("caruspex::EcuPage", "Steuergeräte suchen")}
            </Button>
            <Button icon="activity" disabled={!connected} className={liveOn ? "on" : undefined} onClick={toggleLive}>
              {liveOn ? tr("Live-Werte aus") : t("caruspex::DashboardPage", "Live-Werte")}
            </Button>
            <Button variant="primary" icon="sparkles" onClick={startCheck}>
              {t("caruspex::DashboardPage", "Auto prüfen")}
            </Button>
          </>
        }
      />

      <div className="xray-grid">
        <div className={bare ? "xray-stage card xray-bare" : "xray-stage card"} ref={host} tabIndex={0} aria-label={tr("3D-Modell des Autos. Pfeiltasten drehen, Umschalt mit Pfeiltasten verschiebt, Plus und Minus zoomen.")}>
          {mode === "2d" ? (
            <div className="xray-fallback">
              <VehicleDiagram body={bodyStyleFor(body)} anatomy affected={zones} />
              <div className="hint">{tr("Dieser Rechner kann kein WebGL zeigen, darum hier die einfache Seitenansicht.")}</div>
            </div>
          ) : null}
          {mode === "loading" || mode === "idle" ? <div className="xray-loading hint">{tr("Das 3D-Modell wird geladen ...")}</div> : null}

          {mode === "3d" && !bare ? (
            <XrayControls
              layers={layers}
              setLayers={setLayers}
              energyName={energyName}
              isMoto={isMoto}
              sceneRef={sceneRef}
              exploded={exploded}
              setExploded={setExploded}
              section={section}
              setSection={setSection}
              busesShown={busesShown}
              stats={stats}
            />
          ) : null}

          {tour !== null ? <XrayTour tour={tour} step={step} steps={steps} connected={connected} setTour={setTour} endTour={endTour} /> : null}
        </div>

        <div className="xray-side">
          <XrayCarCard
            shown={shown}
            connected={connected}
            identity={identity}
            generation={generation}
            anatomy={anatomy}
            layout={layout}
            variants={variants}
            drawn={drawn}
            setVariant={setVariant}
            setPowertrain={setPowertrain}
            isMoto={isMoto}
            setMotoStyle={setMotoStyle}
            isHeavy={isHeavy}
            setHeavyKind={setHeavyKind}
            mode={mode}
            ownModel={ownModel}
            modelState={modelState}
            garageCar={garageCar}
            keepModel={keepModel}
            pickModel={pickModel}
          />

          <XrayEcuCard connected={connected} scanning={scanning} layout={layout} selectedEcu={selectedEcu} pickRef={pickRef} udsEcus={udsEcus} />
        </div>
      </div>
    </Page>
  );
}
