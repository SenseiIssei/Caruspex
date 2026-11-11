// The X-ray view (Phase 20): the connected car as a turnable 3D model with a
// glass body. Engine, drivetrain, tank or battery, the control units the
// scan found and the bus lines between them sit where they sit in a real
// car; faults light up their zone, live values hang on their part and
// packets run along the lines to the unit that is being read.
//
// three.js loads only when this page is opened for the first time. Without
// WebGL (some remote desktops, a blocked GPU) the page shows the 2D side
// view with its see-through mode instead.
import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { call } from "../api";
import type { EcuScanRow, LiveRow, VehicleAnatomy, ZoneInfo } from "../api";
import { BodyType, CarZone, Powertrain } from "../api";
import { Icon } from "../components/Icon";
import { Button, Card, Chip, Page, PageHeader, Select, type Option } from "../components/ui";
import { VehicleDiagram } from "../components/VehicleDiagram";
import { localeNumber, t, translator } from "../i18n";
import { useAppEvent, useBackendEvent, useInterval, usePageActive } from "../lib/hooks";
import { onApp } from "../lib/store";
import { cssColor, toCss, useThemeVersion } from "../lib/theme";
import { DEMO_VIN, bodyStyleFor } from "../lib/vehicleIdentity";
import { navigate } from "../shell/nav";
import { live, useSession } from "../state/session";
import {
  EcuKind,
  LAYERS,
  buildLayout,
  zoneAnchor,
  type BusKind,
  type CarLayout,
  type EcuInput,
  type Layer,
  type Vec3,
} from "../xray/layout";
import type { XrayLabel, XrayPalette, XrayScene } from "../xray/scene";
import "./XrayPage.css";

const tr = translator("caruspex::XrayPage");
const VD = "caruspex::VehicleDiagram";

// The live values that get a label, and where.
const LIVE_KEYS: { pid: number; anchor: string; decimals: number }[] = [
  { pid: 0x05, anchor: "cooling", decimals: 0 },
  { pid: 0x0c, anchor: "engine", decimals: 0 },
  { pid: 0x42, anchor: "battery", decimals: 1 },
  { pid: 0x0d, anchor: "wheel", decimals: 0 },
  { pid: 0x2f, anchor: "fuel", decimals: 0 },
  { pid: 0x0f, anchor: "intake", decimals: 0 },
  { pid: 0x5b, anchor: "hv", decimals: 0 },
];

// VAG K-line addresses and the kind of unit behind them.
const KKL_KINDS: Record<number, number> = {
  0x01: EcuKind.Engine, 0x02: EcuKind.Gearbox, 0x03: EcuKind.Brakes, 0x08: EcuKind.Climate,
  0x09: EcuKind.Body, 0x15: EcuKind.Airbag, 0x16: EcuKind.Steering, 0x17: EcuKind.Instruments,
  0x19: EcuKind.Gateway, 0x44: EcuKind.Steering, 0x46: EcuKind.Body, 0x56: EcuKind.Infotainment,
  0x76: EcuKind.Parking, 0x36: EcuKind.Seats, 0x42: EcuKind.Doors, 0x52: EcuKind.Doors,
};

const udsKey = (request: number) => `uds-${request.toString(16)}`;

function layerName(layer: Layer, energy: string): string {
  switch (layer) {
    case "body": return tr("Karosserie");
    case "drivetrain": return tr("Antrieb");
    case "cooling": return t(VD, "Kühlung");
    case "energy": return energy;
    case "electrics": return tr("Elektrik");
    case "network": return tr("Netz");
  }
}

function busName(bus: BusKind): string {
  switch (bus) {
    case "powertrain": return tr("Antriebs-CAN");
    case "comfort": return tr("Komfort-CAN");
    case "diagnostic": return tr("Diagnose-CAN");
    case "kline": return tr("K-Leitung");
  }
}

// What a part of the model is called (the zone names of the side view
// where they fit).
function partName(id: string): string {
  const is = (...prefixes: string[]) => prefixes.some((p) => id === p || id.startsWith(`${p}-`));
  if (is("engine", "cylinder")) return t(VD, "Motor");
  if (is("coil")) return t(VD, "Zündung");
  if (is("intake", "airbox")) return t(VD, "Ansaugung");
  if (is("fuel-rail")) return t(VD, "Einspritzung");
  if (is("throttle")) return t(VD, "Drosselklappe");
  if (is("gearbox", "front-diff", "rear-diff", "shaft", "prop")) return t(VD, "Getriebe");
  if (is("catalyst")) return tr("Katalysator");
  if (is("exhaust", "middle-silencer", "rear-silencer")) return t(VD, "Abgas");
  if (is("turbo")) return t(VD, "Turbo");
  if (is("radiator", "fan", "hose", "expansion")) return t(VD, "Kühlung");
  if (is("fuel-tank", "filler", "fuel-line")) return t(VD, "Tank");
  if (is("hv-battery")) return tr("Hochvoltakku");
  if (is("hv-cable", "inverter")) return tr("Hochvolt");
  if (is("motor")) return tr("Elektromotor");
  if (is("battery12", "alternator", "fuse-box", "harness")) return t(VD, "Bordnetz");
  if (is("obd")) return tr("OBD-Buchse");
  return t(VD, "Innenraum");
}

function zoneLabel(zone: CarZone): string {
  switch (zone) {
    case CarZone.Intake: return t(VD, "Ansaugung");
    case CarZone.Throttle: return t(VD, "Drosselklappe");
    case CarZone.Injection: return t(VD, "Einspritzung");
    case CarZone.Ignition: return t(VD, "Zündung");
    case CarZone.Cylinders: return t(VD, "Motor");
    case CarZone.Cooling: return t(VD, "Kühlung");
    case CarZone.Turbo: return t(VD, "Turbo");
    case CarZone.Exhaust: return t(VD, "Abgas");
    case CarZone.FuelTank: return t(VD, "Tank");
    case CarZone.Gearbox: return t(VD, "Getriebe");
    case CarZone.Ecu: return t(VD, "Steuergerät");
    case CarZone.Electrics: return t(VD, "Bordnetz");
    case CarZone.Network: return t(VD, "Datenbus");
    case CarZone.Wheels: return t(VD, "Räder und ABS");
    case CarZone.Cabin: return t(VD, "Innenraum");
  }
  return "";
}

function readPalette(): XrayPalette {
  const css = (name: string) => toCss(cssColor(name));
  return {
    light: document.documentElement.dataset.theme === "light",
    accent: css("--accent"),
    sky: css("--sky"),
    good: css("--good"),
    warn: css("--warn"),
    bad: css("--bad"),
    info: css("--info"),
    text: css("--text"),
    muted: css("--text-muted"),
    bus: {
      powertrain: css("--bus-powertrain"),
      comfort: css("--bus-comfort"),
      diagnostic: css("--bus-diagnostic"),
      kline: css("--bus-kline"),
    },
  };
}

function useReducedMotion(): boolean {
  const query = typeof window !== "undefined" ? window.matchMedia("(prefers-reduced-motion: reduce)") : null;
  const [reduced, setReduced] = useState(query?.matches ?? false);
  useEffect(() => {
    if (!query) return;
    const on = () => setReduced(query.matches);
    query.addEventListener("change", on);
    return () => query.removeEventListener("change", on);
  }, [query]);
  return reduced;
}

// "Motor (7E8)" -> 0x7E8; 0 when the text has no address.
function responseOf(ecu: string): number {
  const m = ecu.match(/\(([0-9A-Fa-f]{3,8})\)/);
  return m ? Number.parseInt(m[1]!, 16) : 0;
}

interface ZoneFinding {
  zone: CarZone;
  info: ZoneInfo;
  codes: string[];
}

interface Finding {
  id: string;
  anchor: Vec3;
  title: string;
  text: string;
  zone: CarZone | null;
  ecu: string | null;
  tone: "bad" | "warn" | "good";
}

export default function XrayPage() {
  const active = usePageActive();
  const connected = useSession((s) => s.connected);
  const identity = useSession((s) => s.identity);
  const vin = useSession((s) => s.vin);
  const faults = useSession((s) => s.faults);
  const udsEcus = useSession((s) => s.udsEcus);
  const protocol = useSession((s) => s.protocol);
  const supported = useSession((s) => s.supportedPids);
  const themeVersion = useThemeVersion();
  const reduced = useReducedMotion();

  // --- Which car -------------------------------------------------------------------
  const generation = identity?.generation ?? null;
  const demo = vin === DEMO_VIN;
  const body = generation ? generation.body : demo ? BodyType.Estate : BodyType.Hatchback;
  const year = identity?.year || generation?.yearFrom || (connected ? 2012 : 2018);
  const [powertrain, setPowertrain] = useState(0);
  const [anatomy, setAnatomy] = useState<VehicleAnatomy | null>(null);
  useEffect(() => setPowertrain(0), [generation?.id]);
  useEffect(() => {
    let stale = false;
    void call("vehicle_anatomy", {
      generationId: generation?.id ?? -1,
      year,
      body,
      brand: generation?.brand ?? "",
      model: generation?.model ?? "",
      powertrain: generation ? powertrain : Powertrain.Petrol,
    }).then(
      (a) => !stale && setAnatomy(a),
      () => undefined,
    );
    return () => {
      stale = true;
    };
  }, [generation, year, body, powertrain]);

  // --- Control units ------------------------------------------------------------------
  const [asking, setAsking] = useState<{ key: string; kind: number; name: string; source: "uds" | "kline" } | null>(null);
  const [scanning, setScanning] = useState<{ index: number; total: number; name: string } | null>(null);
  const [kklRows, setKklRows] = useState<EcuScanRow[]>([]);
  const [kklOpen, setKklOpen] = useState(false);
  const kline = /9141|14230|KWP/i.test(protocol) || kklOpen;

  useBackendEvent("uds_scan_started", ({ total }) => setScanning({ index: 0, total, name: "" }));
  useBackendEvent("uds_scan_progress", ({ name, index, total, request, kind }) => {
    setScanning({ index, total, name });
    setAsking({ key: udsKey(request), kind, name, source: "uds" });
    pendingPacket.current = udsKey(request);
  });
  useBackendEvent("uds_scan_finished", () => {
    setScanning(null);
    setAsking(null);
  });
  useBackendEvent("scan_started", ({ total }) => {
    setKklRows([]);
    setScanning({ index: 0, total, name: "" });
  });
  useBackendEvent("scan_progress", ({ ecuName, index, total, address }) => {
    setScanning({ index, total, name: ecuName });
    setAsking({ key: `kkl-${address}`, kind: KKL_KINDS[address] ?? EcuKind.Other, name: ecuName, source: "kline" });
    pendingPacket.current = `kkl-${address}`;
  });
  useBackendEvent("scan_ecu_result", ({ row }) => setKklRows((rows) => [...rows.filter((r) => r.address !== row.address), row]));
  useBackendEvent("scan_finished", () => {
    setScanning(null);
    setAsking(null);
  });
  useBackendEvent("kkl_connected", ({ connected: on }) => setKklOpen(on));
  useEffect(() => {
    if (connected) return;
    setKklRows([]);
    setScanning(null);
    setAsking(null);
  }, [connected]);

  const ecuInputs = useMemo<EcuInput[]>(() => {
    const list: EcuInput[] = [];
    for (const r of udsEcus) {
      if (!r.present) continue;
      list.push({ key: udsKey(r.request), kind: r.kind, name: r.kind === EcuKind.Other && r.systemName ? r.systemName : r.name, faulty: r.dtcs.length > 0, source: "uds" });
    }
    for (const r of kklRows) {
      if (!r.present) continue;
      list.push({ key: `kkl-${r.address}`, kind: KKL_KINDS[r.address] ?? EcuKind.Other, name: r.name, faulty: r.faults.length > 0, source: "kline" });
    }
    if (connected) {
      // Plain OBD always reaches the engine, and the gearbox when it reports.
      const gearboxFaults = faults.filter((f) => responseOf(f.ecu) === 0x7e9);
      const engineFaults = faults.filter((f) => responseOf(f.ecu) !== 0x7e9);
      const engine = list.find((e) => e.kind === EcuKind.Engine);
      if (engine) engine.faulty ||= engineFaults.length > 0;
      else list.push({ key: "obd-engine", kind: EcuKind.Engine, name: t(VD, "Motor"), faulty: engineFaults.length > 0, source: kline ? "kline" : "obd" });
      const gearbox = list.find((e) => e.kind === EcuKind.Gearbox);
      if (gearbox) gearbox.faulty ||= gearboxFaults.length > 0;
      else if (gearboxFaults.length > 0) list.push({ key: "obd-gearbox", kind: EcuKind.Gearbox, name: t(VD, "Getriebe"), faulty: true, source: kline ? "kline" : "obd" });
    }
    if (asking && !list.some((e) => e.key === asking.key)) {
      list.push({ key: asking.key, kind: asking.kind, name: asking.name, faulty: false, source: asking.source, asking: true });
    }
    return list;
  }, [udsEcus, kklRows, faults, connected, kline, asking]);

  // --- Faults and their zones -----------------------------------------------------------
  const [zoneFindings, setZoneFindings] = useState<ZoneFinding[]>([]);
  const codes = useMemo(() => {
    const set = new Set<string>(faults.map((f) => f.code));
    for (const r of udsEcus) for (const d of r.dtcs) set.add(d.code);
    for (const r of kklRows) for (const f of r.faults) set.add(f.code);
    return [...set];
  }, [faults, udsEcus, kklRows]);
  useEffect(() => {
    let stale = false;
    void Promise.all(codes.map((code) => call("locate_dtc", { code }).catch(() => null))).then((found) => {
      if (stale) return;
      const byZone = new Map<CarZone, ZoneFinding>();
      found.forEach((info, i) => {
        if (!info || info.zone === CarZone.Unknown) return;
        const entry = byZone.get(info.zone) ?? { zone: info.zone, info, codes: [] };
        entry.codes.push(codes[i]!);
        byZone.set(info.zone, entry);
      });
      setZoneFindings([...byZone.values()]);
    });
    return () => {
      stale = true;
    };
  }, [codes]);
  const zones = useMemo(() => zoneFindings.map((z) => z.zone), [zoneFindings]);

  // --- Layout ----------------------------------------------------------------------------
  const turbo = (anatomy?.powertrain ?? 0) === Powertrain.Diesel || zones.includes(CarZone.Turbo);
  const layout = useMemo<CarLayout | null>(
    () => (anatomy ? buildLayout({ anatomy, body, ecus: ecuInputs, kline, turbo }) : null),
    [anatomy, body, ecuInputs, kline, turbo],
  );
  const carKey = anatomy ? `${JSON.stringify(anatomy)}|${body}|${turbo}|${themeVersion}` : "";

  // --- The scene ---------------------------------------------------------------------------
  const host = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<XrayScene | null>(null);
  const [mode, setMode] = useState<"idle" | "loading" | "3d" | "2d">("idle");
  const [hovered, setHovered] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const lastCar = useRef("");
  const pendingPacket = useRef<string | null>(null);
  const pickRef = useRef<(id: string | null) => void>(() => undefined);

  const started = useRef(false);
  const unmounted = useRef(false);
  useEffect(() => {
    if (!active || started.current) return;
    started.current = true;
    setMode("loading");
    void import("../xray/scene").then(
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
          });
          setMode("3d");
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

  // --- Layers ---------------------------------------------------------------------------------
  const [layers, setLayers] = useState<Record<Layer, boolean>>({ body: true, drivetrain: true, cooling: true, energy: true, electrics: true, network: true });
  useEffect(() => sceneRef.current?.setLayers(layers), [layers, mode]);
  const energyName =
    anatomy && anatomy.battery !== 0 ? (anatomy.fuelTank ? tr("Kraftstoff und Hochvolt") : tr("Hochvolt")) : tr("Kraftstoff");

  // --- Live values on the parts ---------------------------------------------------------------
  const [liveRows, setLiveRows] = useState<LiveRow[]>([]);
  useInterval(
    () => {
      const rows = LIVE_KEYS.map((k) => live.get(k.pid)).filter((r): r is LiveRow => !!r);
      setLiveRows((old) => (old.length === rows.length && old.every((r, i) => r === rows[i]) ? old : rows));
    },
    active && connected ? 400 : null,
  );
  useEffect(() => {
    if (!connected) setLiveRows([]);
  }, [connected]);
  const [liveOn, setLiveOn] = useState(false);
  const startedLive = useRef(false);
  const toggleLive = () => {
    if (liveOn) {
      void call("stop_live");
      startedLive.current = false;
      setLiveOn(false);
      return;
    }
    const pids = LIVE_KEYS.map((k) => k.pid).filter((p) => supported.length === 0 || supported.includes(p));
    void call("start_live", { pids });
    startedLive.current = true;
    setLiveOn(true);
  };
  useEffect(() => {
    if ((active && connected) || !startedLive.current) return;
    startedLive.current = false;
    setLiveOn(false);
    if (connected) void call("stop_live");
  }, [active, connected]);

  // --- Findings and the guided check ----------------------------------------------------------
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
    const at = part?.center ?? tube?.points[Math.floor((tube.points.length - 1) / 2)] ?? (id === "obd" ? layout.obd : null);
    if (at) scene.flyTo(at, 2.8);
  };

  // --- Highlight and labels ---------------------------------------------------------------------
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
    const pointed = hovered ?? selected;
    if (pointed) {
      if (pointed.startsWith("ecu:")) {
        const e = layout.ecus.find((n) => n.key === pointed.slice(4));
        if (e) labels.push({ id: "pointed", anchor: [e.position[0], e.position[1] + 0.06, e.position[2]], title: e.name, value: busName(e.bus), tone: e.faulty ? "bad" : "accent" });
      } else {
        const part = layout.parts.find((p) => p.id === pointed);
        const tube = layout.tubes.find((p) => p.id === pointed);
        const at = part ? ([part.center[0], part.center[1] + part.size[1] / 2 + 0.03, part.center[2]] as Vec3) : tube ? tube.points[Math.floor((tube.points.length - 1) / 2)]! : pointed === "obd" ? layout.obd : null;
        if (at && !zoneFindings.some((z) => (part?.zones ?? tube?.zones ?? []).includes(z.zone))) {
          labels.push({ id: "pointed", anchor: at, title: partName(pointed), tone: "accent" });
        }
      }
    }
    scene.setLabels(labels);
  }, [layout, zoneFindings, liveRows, hovered, selected, mode]);

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

  // --- Scan ---------------------------------------------------------------------------------------
  const brand = identity?.brand ?? "";
  const startScan = useCallback(async () => {
    if (!connected) return;
    const family = await call("brand_family", { brand }).catch(() => "");
    void call("scan_uds", { family });
  }, [connected, brand]);

  // "xray-scan", "xray-check" and "xray-live" of --page, for pictures.
  useAppEvent<string>("dev:page", (name) => {
    if (!name.startsWith("xray")) return;
    navigate("xray");
    if (name === "xray-scan") setTimeout(() => void startScan(), 600);
    if (name === "xray-check") setTimeout(startCheck, 1200);
    if (name === "xray-live") setTimeout(() => !liveOn && toggleLive(), 600);
  });

  // --- Text for the side panel -----------------------------------------------------------------------
  const title = connected ? identity?.title || t("caruspex::DashboardPage", "Verbundenes Fahrzeug") : tr("Beispielauto");
  const facts = anatomy
    ? [anatomy.engine !== 0 ? anatomy.engineName : "", anatomy.drivenName].filter(Boolean).join(" · ")
    : "";
  const dims = anatomy
    ? tr(
        "%1 m lang, %2 m breit, %3 m hoch, Radstand %4 m",
        localeNumber(anatomy.lengthMm / 1000, 2),
        localeNumber(anatomy.widthMm / 1000, 2),
        localeNumber(anatomy.heightMm / 1000, 2),
        localeNumber(anatomy.wheelbaseMm / 1000, 2),
      )
    : "";
  const powertrainOptions: Option<number>[] = (anatomy?.powertrainChoices ?? []).map((c) => ({ value: c.flag, label: c.name }));
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
        <div className="xray-stage card" ref={host} tabIndex={0} aria-label={tr("3D-Modell des Autos. Pfeiltasten drehen, Umschalt mit Pfeiltasten verschiebt, Plus und Minus zoomen.")}>
          {mode === "2d" ? (
            <div className="xray-fallback">
              <VehicleDiagram body={bodyStyleFor(body)} anatomy affected={zones} />
              <div className="hint">{tr("Dieser Rechner kann kein WebGL zeigen, darum hier die einfache Seitenansicht.")}</div>
            </div>
          ) : null}
          {mode === "loading" || mode === "idle" ? <div className="xray-loading hint">{tr("Das 3D-Modell wird geladen ...")}</div> : null}

          {mode === "3d" ? (
            <>
              <div className="xray-layers" role="group" aria-label={tr("Ebenen")}>
                {LAYERS.map((layer) => (
                  <Chip key={layer} on={layers[layer]} onClick={() => setLayers((l) => ({ ...l, [layer]: !l[layer] }))}>
                    {layerName(layer, energyName)}
                  </Chip>
                ))}
              </div>
              <div className="xray-view-buttons">
                <Button small icon="refresh" title={tr("Ansicht zurücksetzen")} onClick={() => sceneRef.current?.resetView()} />
              </div>
              {layers.network && busesShown.length > 0 ? (
                <div className="xray-legend">
                  {busesShown.map((bus) => (
                    <span key={bus} className="xray-legend-item">
                      <span className="xray-legend-line" style={{ "--bus": `var(--bus-${bus})` } as CSSProperties} />
                      {busName(bus)}
                    </span>
                  ))}
                </div>
              ) : null}
              <div className="xray-help hint">{tr("Ziehen dreht, Mausrad zoomt, rechte Maustaste verschiebt.")}</div>
            </>
          ) : null}

          {tour !== null ? (
            <div className="xray-tour fade-in" key={step?.id ?? "none"} data-tone={step?.tone ?? "good"}>
              <div className="xray-tour-head">
                <Icon name={step ? "alert" : "check-circle"} size={18} />
                <span className="strong">{step ? step.title : tr("Nichts gefunden")}</span>
                {steps ? <span className="hint">{tr("%1 von %2", (tour ?? 0) + 1, steps.length)}</span> : null}
              </div>
              <div className="subtle">
                {step ? step.text : connected ? tr("Kein Fehler gespeichert und jedes gefundene Steuergerät ist in Ordnung.") : tr("Erst verbinden, dann zeigt die Prüfung, wo am Auto etwas ist.")}
              </div>
              <div className="xray-tour-buttons">
                {steps && steps.length > 1 ? (
                  <>
                    <Button small icon="chevron-left" disabled={!tour} onClick={() => setTour((i) => Math.max(0, (i ?? 0) - 1))} />
                    <Button small icon="chevron-right" disabled={(tour ?? 0) + 1 >= steps.length} onClick={() => setTour((i) => Math.min(steps.length - 1, (i ?? 0) + 1))} />
                  </>
                ) : null}
                <Button small onClick={endTour}>
                  {t("caruspex::TitleBar", "Schließen")}
                </Button>
              </div>
            </div>
          ) : null}
        </div>

        <div className="xray-side">
          <Card className="xray-car">
            <div className="eyebrow">{connected ? t("caruspex::DashboardPage", "DEIN FAHRZEUG") : tr("NOCH NICHT VERBUNDEN")}</div>
            <h2 className="section-title">{title}</h2>
            {anatomy ? (
              <>
                <div className="subtle">{[generation?.bodyName, facts].filter(Boolean).join(" · ")}</div>
                <div className="xray-reason">{anatomy.reason}</div>
                <div className="hint">{dims}</div>
                <div className="hint">{anatomy.dimensionsText}</div>
                {powertrainOptions.length > 1 ? (
                  <label className="xray-select">
                    <span className="hint">{tr("Antrieb")}</span>
                    <Select value={anatomy.powertrain} options={powertrainOptions} onChange={(v) => setPowertrain(Number(v))} />
                  </label>
                ) : null}
              </>
            ) : null}
            {!connected ? <div className="hint">{tr("Verbinde dich mit deinem Auto oder dem Demo-Auto, dann zeigt das Modell seine Form, seine Steuergeräte und seine Fehler.")}</div> : null}
          </Card>

          <Card className="xray-ecus" title={t("caruspex::MainWindow", "Alle Steuergeräte")} icon="cpu">
            {scanning ? (
              <div className="hint">{t("caruspex::EcuPage", "Frage %1 (%2 von %3)", scanning.name, scanning.index + 1, scanning.total)}</div>
            ) : null}
            {layout && layout.ecus.length > 0 ? (
              <div className="xray-ecu-list">
                {layout.ecus.map((e) => (
                  <button
                    key={e.key}
                    type="button"
                    className={`xray-ecu${selectedEcu === e.key ? " selected" : ""}${e.asking ? " asking" : ""}`}
                    style={{ "--bus": `var(--bus-${e.bus})` } as CSSProperties}
                    onClick={() => pickRef.current(`ecu:${e.key}`)}
                  >
                    <span className="xray-ecu-dot" data-faulty={e.faulty || undefined} />
                    <span className="xray-ecu-name">{e.name}</span>
                    <span className="hint">{e.faulty ? tr("Fehler") : e.asking ? tr("fragt ...") : busName(e.bus)}</span>
                  </button>
                ))}
              </div>
            ) : (
              <div className="hint">{connected ? tr("Noch nicht gesucht. Steuergeräte suchen zeigt jedes Gerät an seinem Platz.") : tr("Nach dem Verbinden erscheinen hier die Steuergeräte.")}</div>
            )}
            {udsEcus.length > 0 ? (
              <Button small icon="chevron-right" onClick={() => navigate("ecus")}>
                {t("caruspex::MainWindow", "Alle Steuergeräte")}
              </Button>
            ) : null}
          </Card>
        </div>
      </div>
    </Page>
  );
}
