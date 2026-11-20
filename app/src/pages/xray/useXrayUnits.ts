// The control units of the X-ray view (from the UDS and K-line scans, the
// J1939 network and plain OBD) and the zones of the stored faults.
import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { call } from "../../api";
import type { EcuScanRow } from "../../api";
import { CarZone } from "../../api";
import { t } from "../../i18n";
import { useBackendEvent } from "../../lib/hooks";
import { useSession, type SessionState } from "../../state/session";
import { EcuKind, type EcuInput } from "../../xray/layout";
import { J1939_KINDS, KKL_KINDS, responseOf, udsKey } from "./names";
import type { ZoneFinding } from "./support";

const VD = "caruspex::VehicleDiagram";

export function useXrayUnits({
  connected,
  protocol,
  faults,
  udsEcus,
  j1939Units,
  pendingPacket,
}: {
  connected: boolean;
  protocol: string;
  faults: SessionState["faults"];
  udsEcus: SessionState["udsEcus"];
  j1939Units: SessionState["j1939Units"];
  // The unit the next packet goes to, sent once the scene has it.
  pendingPacket: RefObject<string | null>;
}) {
  // --- Control units ------------------------------------------------------------------
  const [asking, setAsking] = useState<{ key: string; kind: number; name: string; source: "uds" | "kline"; bus?: string } | null>(null);
  const [scanning, setScanning] = useState<{ index: number; total: number; name: string } | null>(null);
  const [kklRows, setKklRows] = useState<EcuScanRow[]>([]);
  const [kklOpen, setKklOpen] = useState(false);
  const kline = /9141|14230|KWP/i.test(protocol) || kklOpen;

  // The bus the running scan asks on ("hs", "ms", "sw").
  const scanBus = useRef("hs");
  // Scans still to run one after the other (the dev option "xray-scan-all").
  const scanQueue = useRef<string[]>([]);
  useBackendEvent("uds_scan_started", ({ total, bus }) => {
    scanBus.current = bus ?? "hs";
    setScanning({ index: 0, total, name: "" });
  });
  // A sweep asks hundreds of addresses in a few seconds: the unit being
  // asked is shown at most every 150 ms, so the scene is not rebuilt for each.
  const nextAsk = useRef<{ scanning: { index: number; total: number; name: string }; asking: { key: string; kind: number; name: string; source: "uds"; bus: string } } | null>(null);
  const askTimer = useRef<number | null>(null);
  const flushAsk = () => {
    askTimer.current = null;
    const next = nextAsk.current;
    if (!next) return;
    setScanning(next.scanning);
    setAsking(next.asking);
    pendingPacket.current = next.asking.key;
  };
  useBackendEvent("uds_scan_progress", ({ name, index, total, request, kind }) => {
    const key = scanBus.current !== "hs" ? `${udsKey(request)}-${scanBus.current}` : udsKey(request);
    nextAsk.current = { scanning: { index, total, name }, asking: { key, kind, name, source: "uds", bus: scanBus.current } };
    askTimer.current ??= window.setTimeout(flushAsk, 150);
  });
  useBackendEvent("uds_scan_finished", () => {
    if (askTimer.current !== null) window.clearTimeout(askTimer.current);
    askTimer.current = null;
    nextAsk.current = null;
    setScanning(null);
    setAsking(null);
    const next = scanQueue.current.shift();
    if (next !== undefined) setTimeout(() => void call("scan_uds", { family: next }), 400);
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

  const otherBuses = useSession((s) => s.udsOtherBuses);
  const ecuInputs = useMemo<EcuInput[]>(() => {
    const list: EcuInput[] = [];
    // The units of this scan and those an earlier scan found on another bus
    // (the MS-CAN or SW-CAN of an OBDLink).
    for (const r of [...otherBuses, ...udsEcus]) {
      if (!r.present) continue;
      const key = r.bus && r.bus !== "hs" ? `${udsKey(r.request)}-${r.bus}` : udsKey(r.request);
      if (list.some((e) => e.key === key)) continue;
      list.push({ key, kind: r.kind, name: r.kind === EcuKind.Other && r.systemName ? r.systemName : r.name, faulty: r.dtcs.length > 0, source: "uds", bus: r.bus });
    }
    for (const r of kklRows) {
      if (!r.present) continue;
      list.push({ key: `kkl-${r.address}`, kind: KKL_KINDS[r.address] ?? EcuKind.Other, name: r.name, faulty: r.faults.length > 0, source: "kline" });
    }
    // The units of a truck's J1939 network, with their active faults (DM1).
    for (const u of j1939Units) {
      const name = u.address === 0xee ? `${u.label} (TCO)` : u.label;
      list.push({ key: `j1939-${u.address}`, kind: J1939_KINDS[u.address] ?? EcuKind.Other, name, faulty: u.active > 0, source: "uds" });
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
      list.push({ key: asking.key, kind: asking.kind, name: asking.name, faulty: false, source: asking.source, asking: true, bus: asking.bus });
    }
    return list;
  }, [udsEcus, otherBuses, kklRows, j1939Units, faults, connected, kline, asking]);

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

  return { scanning, kline, scanQueue, ecuInputs, zoneFindings, zones };
}
