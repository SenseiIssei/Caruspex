// Everything known about the car that is connected right now, collected from
// the backend events in one place (VehicleState.cpp), plus the connection
// state of the title bar, the log and the settings. The wiring MainWindow did
// between the worker and the garage lives here too.
import { call, on } from "../api";
import type {
  AppSettings,
  ConnectRoute,
  FaultRow,
  FreezeFrameRow,
  HealthFindingRow,
  J1939IdRow,
  J1939UnitRow,
  LiveRow,
  MonitorRow,
  ReportFinding,
  StatusRow,
  UdsEcuRow,
} from "../api";
import { DEFAULT_SETTINGS } from "../api";
import { toast } from "../components/Toast";
import { t, tn } from "../i18n";
import { FrameNotifier } from "../lib/frames";
import { createStore, emitApp, useStore } from "../lib/store";
import { identifyVin, type IdentifiedVehicle } from "../lib/vehicleIdentity";
import { addCheck, findCar, garageStore, isoNow, latestOdometer, rememberConnection, serviceStatus, touchCar } from "./garage";

const MW = "caruspex::MainWindow";

export type Link = "offline" | "connecting" | "online" | "error";

export interface ReadinessRow {
  name: string;
  ready: boolean;
}

// What the start page found, for the report (DashboardPage::findings()).
export interface DashboardFinding {
  icon: string;
  // "bad" | "warn" | "good" | "info": the colour of the finding.
  tone: "bad" | "warn" | "good" | "info";
  title: string;
  text: string;
  meta: string;
}

export interface SessionState {
  link: Link;
  linkText: string;
  connected: boolean;
  busy: boolean;
  adapter: string;
  transport: string;
  protocol: string;
  // Connected through a raw CAN interface: TP 2.0 for older VAG cars works.
  rawCan: boolean;
  // Connected to a truck over SAE J1939.
  j1939: boolean;
  // J1939: the control units with their lamps, and their identification.
  j1939Units: J1939UnitRow[];
  j1939Ids: J1939IdRow[];
  // The extra CAN buses the OBDLink of this connection can switch to:
  // "ms" (Ford MS-CAN), "sw" (GM single wire CAN). Empty for an ELM327.
  stnBuses: string[];
  // The way the connection went, remembered in the garage with the VIN.
  route: ConnectRoute | null;
  connectedAt: string;
  vin: string;
  identity: IdentifiedVehicle | null;
  haveFaults: boolean;
  faults: FaultRow[];
  status: StatusRow | null;
  freezeCode: string;
  freezeRows: LiveRow[];
  freezeFrames: FreezeFrameRow[];
  monitors: MonitorRow[];
  calibration: string;
  ecuName: string;
  supportedPids: number[];
  // Score of the last health evaluation, -1 if none yet, with what the start
  // page shows for it.
  score: number;
  scoreCaption: string;
  findings: DashboardFinding[];
  // The findings of the last full health check, for "Was soll ich jetzt tun?".
  healthFindings: HealthFindingRow[];
  recording: boolean;
  // The file of the last recording of this run, for "Mitschnitt teilen".
  lastRecording: string;
  // Due and nearly due service items of the connected car (badge).
  servicesDue: number;
  // Control units of the last scan of all units (EcuPage::results()), for
  // the report.
  udsEcus: UdsEcuRow[];
  // Units an earlier scan found on another bus of the same connection (the
  // MS-CAN or SW-CAN of an OBDLink), for the X-ray view.
  udsOtherBuses: UdsEcuRow[];
}

const initial: SessionState = {
  link: "offline",
  linkText: "",
  connected: false,
  busy: false,
  adapter: "",
  transport: "",
  protocol: "",
  rawCan: false,
  j1939: false,
  j1939Units: [],
  j1939Ids: [],
  stnBuses: [],
  route: null,
  connectedAt: "",
  vin: "",
  identity: null,
  haveFaults: false,
  faults: [],
  status: null,
  freezeCode: "",
  freezeRows: [],
  freezeFrames: [],
  monitors: [],
  calibration: "",
  ecuName: "",
  supportedPids: [],
  score: -1,
  scoreCaption: "",
  findings: [],
  healthFindings: [],
  recording: false,
  lastRecording: "",
  servicesDue: 0,
  udsEcus: [],
  udsOtherBuses: [],
};

export const session = createStore<SessionState>(initial);

export function useSession<S>(selector: (s: SessionState) => S): S {
  return useStore(session, selector);
}

// Latest reading per live key, kept outside the store: it changes many times
// a second and only a few places read it directly.
export const live = new Map<number, LiveRow>();

// Which keys of `live` changed, for the pages that show them (useLiveRows in
// state/liveView.ts): told once per animation frame, not once per reading.
export const liveFrames = new FrameNotifier<number>();

export function liveValue(pid: number, fallback = 0): number {
  return live.get(pid)?.value ?? fallback;
}

export function readiness(status: StatusRow | null = session.get().status): ReadinessRow[] {
  if (!status) return [];
  const rows: ReadinessRow[] = [];
  for (let i = 0; i < status.monitorNames.length && i < status.monitorReady.length; i++) {
    rows.push({ name: status.monitorNames[i]!, ready: status.monitorReady[i]! });
  }
  return rows;
}

// --- Settings -------------------------------------------------------------------------

// modeOverride: the mode of --mode or ?mode= for this run only, null for the
// setting.
export const settingsStore = createStore<{ values: AppSettings; loaded: boolean; modeOverride: boolean | null }>({
  values: DEFAULT_SETTINGS,
  loaded: false,
  modeOverride: null,
});

export function useSettings(): AppSettings {
  return useStore(settingsStore, (s) => s.values);
}

// Profi mode: every page. Beginners see the everyday ones.
export function useProMode(): boolean {
  return useStore(settingsStore, (s) => s.modeOverride ?? s.values.proMode);
}

// Switches the mode for good (the settings switch and the hints on the pages).
export function setProMode(on: boolean): void {
  settingsStore.set({ modeOverride: null });
  void applySettings({ ...settingsStore.get().values, proMode: on });
}

// Saves, passes them on to the backend and to the pages (applySettings).
export async function applySettings(values: AppSettings): Promise<void> {
  const before = settingsStore.get().values;
  settingsStore.set({ values });
  await call("save_settings", { values });
  if (values.logToFile && values.logPath && (!before.logToFile || before.logPath !== values.logPath)) {
    appendLog(t(MW, "Protokoll wird zusätzlich nach %1 geschrieben.", values.logPath));
  }
}

// --- Log --------------------------------------------------------------------------------

// total: lines added so far, clears: how often the log was emptied; with both
// the log panel can tell which of `lines` are new and append only those.
export const logStore = createStore<{ lines: string[]; total: number; clears: number }>({ lines: [], total: 0, clears: 0 });

// A line the interface logs itself: shown and, like everything in
// MainWindow::appendLog, written to the log file when one is open.
export function appendLog(line: string): void {
  showLog(line);
  void call("write_log", { line }).catch(() => undefined);
}

// Only shows the line. The worker's own lines (log_line) are already in the
// log file when they arrive. A scan sends hundreds of lines a second: they
// reach the log panel together every 150 ms, so the page (and the X-ray
// view) is not drawn again for every single line.
let pendingLines: string[] = [];
let flushTimer: ReturnType<typeof setTimeout> | null = null;

function flushLog(): void {
  flushTimer = null;
  const add = pendingLines;
  pendingLines = [];
  if (!add.length) return;
  logStore.set((s) => {
    const lines = s.lines.concat(add);
    return { lines: lines.length > 5000 ? lines.slice(-5000) : lines, total: s.total + add.length };
  });
}

function showLog(line: string): void {
  const now = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  pendingLines.push(`${p(now.getHours())}:${p(now.getMinutes())}:${p(now.getSeconds())}  ${line}`);
  flushTimer ??= setTimeout(flushLog, 150);
}

export function clearLog(): void {
  pendingLines = [];
  logStore.set((s) => ({ lines: [], clears: s.clears + 1 }));
}

// --- Title bar ---------------------------------------------------------------------------

export function setLink(link: Link, text: string): void {
  session.set({ link, linkText: text });
}

// --- Garage hand-over ------------------------------------------------------------------

// Stores the current check in the garage, once VIN and faults are known.
export function recordCheck(): void {
  const s = session.get();
  if (!s.vin || !s.haveFaults) return;
  addCheck(s.vin, {
    when: isoNow(),
    score: s.score,
    mil: s.status?.mil ?? false,
    openMonitors: readiness(s.status).filter((r) => !r.ready).length,
    odometerKm: live.has(0xa6) ? liveValue(0xa6) : -1,
    codes: s.faults.map((f) => f.code),
    texts: s.faults.map((f) => f.text),
  });
}

// Due and nearly due service items of the connected car: badge and toast.
export function remindServices(withToast: boolean): void {
  const vin = session.get().vin;
  const car = vin ? findCar(vin) : undefined;
  if (!car) {
    session.set({ servicesDue: 0 });
    return;
  }
  const km = live.has(0xa6) ? liveValue(0xa6) : latestOdometer(vin);
  const lines: string[] = [];
  for (const st of serviceStatus(car, km)) {
    if (st.state === "due" || st.state === "soon") lines.push(`${st.service.title}: ${st.text}`);
  }
  session.set({ servicesDue: lines.length });
  if (withToast && lines.length > 0) toast(t(MW, "Wartung: %1", lines.join(", ")), "warning", 8000);
}

// --- Report -------------------------------------------------------------------------------

// The findings of the start page with their level (collectReportExtras).
export function reportFindings(): ReportFinding[] {
  return session.get().findings.map((f) => ({
    severity:
      f.tone === "bad" ? t(MW, "Hoch") : f.tone === "warn" ? t(MW, "Mittel") : f.tone === "good" ? t(MW, "In Ordnung") : t(MW, "Hinweis"),
    title: f.title,
    text: f.text,
  }));
}

// --- Wiring ---------------------------------------------------------------------------------

function resetVehicle(): Partial<SessionState> {
  live.clear();
  liveFrames.markAll();
  return {
    connected: false,
    adapter: "",
    transport: "",
    protocol: "",
    rawCan: false,
    j1939: false,
    j1939Units: [],
    j1939Ids: [],
    stnBuses: [],
    route: null,
    connectedAt: "",
    vin: "",
    identity: null,
    haveFaults: false,
    faults: [],
    status: null,
    freezeCode: "",
    freezeRows: [],
    freezeFrames: [],
    monitors: [],
    calibration: "",
    ecuName: "",
    supportedPids: [],
    score: -1,
    healthFindings: [],
    udsEcus: [],
    udsOtherBuses: [],
  };
}

async function onVin(vin: string): Promise<void> {
  // Recognise the car: name, picture and the brand for maker codes.
  const car = await identifyVin(vin);
  if (session.get().vin !== vin) return;
  session.set({ identity: car });
  if (car.known) toast(t(MW, "Erkannt: %1", car.title), "success");
  touchCar(vin, car.title, car.subtitle, car.body);
  rememberConnection(vin, session.get().route, session.get().adapter);
  setTimeout(() => remindServices(true), 1500);
  recordCheck();
}

function setVin(raw: string): void {
  const clean = raw.trim().toUpperCase();
  if (!clean || clean === session.get().vin) return;
  session.set({ vin: clean });
  void onVin(clean);
}

let started = false;

export function startSession(): void {
  if (started) return;
  started = true;

  on("log_line", ({ line }) => showLog(line));
  on("busy", ({ active }) => session.set({ busy: active }));

  on("connected", ({ adapter, transport, protocol, rawCan, stnBuses, route, j1939 }) => {
    session.set({
      ...resetVehicle(),
      connected: true,
      adapter,
      transport,
      protocol,
      rawCan: rawCan === true,
      j1939: j1939 === true,
      stnBuses: stnBuses ?? [],
      route: route ?? null,
      connectedAt: isoNow(),
      link: "online",
      linkText: t(MW, "Verbunden · %1", protocol),
    });
    toast(t(MW, "Verbunden über %1. Protokoll: %2", transport, protocol), "success");
    // After connecting the start page is the natural place to go. The VIN
    // comes right after, so the garage recognises the car.
    emitApp("navigate", "start");
    setTimeout(() => void call("read_details"), 0);
  });

  on("failed", ({ error }) => {
    if (!session.get().connected) setLink("error", t(MW, "Keine Verbindung"));
    appendLog(error);
    toast(error, "error", 6000);
  });

  on("closed", ({ lost }) => {
    session.set({
      ...resetVehicle(),
      recording: false,
      link: lost ? "error" : "offline",
      linkText: lost ? t(MW, "Verbindung verloren") : t(MW, "Nicht verbunden"),
      servicesDue: 0,
    });
    if (lost) toast(lost, "error", 10000);
    appendLog(t(MW, "Verbindung getrennt."));
  });

  on("j1939_units", ({ units }) => session.set({ j1939Units: units }));
  on("j1939_ids", ({ units }) => session.set({ j1939Ids: units }));

  on("faults", ({ rows }) => {
    session.set({ faults: rows, haveFaults: true });
    recordCheck();
  });

  on("status", ({ row }) => {
    if (row.valid) session.set({ status: row });
  });

  on("vehicle_info", ({ vin, freezeCode, freezeRows }) => {
    session.set({ freezeCode, freezeRows });
    setVin(vin);
  });

  on("freeze_frames", ({ frames }) => session.set({ freezeFrames: frames }));

  on("identity", ({ vin, calibration, ecuName }) => {
    session.set({ calibration, ecuName });
    setVin(vin);
  });

  on("monitor_results", ({ rows }) => session.set({ monitors: rows }));
  // A scan of another bus (MS-CAN, SW-CAN of an OBDLink) keeps the units
  // found on the others for the X-ray view.
  on("uds_scan_started", ({ bus }) =>
    session.set((s) => {
      const scanned = bus ?? "hs";
      const kept = [...s.udsOtherBuses, ...s.udsEcus].filter((e) => (e.bus ?? "hs") !== scanned);
      return { udsEcus: [], udsOtherBuses: kept };
    }),
  );
  on("uds_ecu_result", ({ row }) =>
    session.set((s) => {
      const list = s.udsEcus.filter((e) => !(e.request === row.request && e.response === row.response));
      return { udsEcus: [...list, row] };
    }),
  );
  on("supported_pids", ({ pids }) => session.set({ supportedPids: pids }));

  on("live_values", ({ rows }) => {
    for (const row of rows) {
      live.set(row.pid, row);
      liveFrames.mark(row.pid);
    }
    if (!rows.some((row) => row.pid === 0xa6)) return;
    // The odometer often arrives after the faults: store it with the check
    // of this session (it replaces the one from a minute ago).
    const vin = session.get().vin;
    const car = vin ? findCar(vin) : undefined;
    const last = car?.checks[car.checks.length - 1];
    if (last && last.odometerKm < 0 && Date.now() - new Date(last.when).getTime() < 15 * 60 * 1000) recordCheck();
  });

  on("kkl_message", ({ message }) => appendLog(message));

  on("adapter_found", ({ device }) => {
    const values = { ...settingsStore.get().values, lastPort: device };
    void applySettings(values);
    emitApp("connect:port-found", device);
    toast(t(MW, "Adapter an %1 gefunden.", device), "success");
  });

  on("recording_state", ({ active, path, exchanges, error }) => {
    session.set({ recording: active });
    if (!active && path && !error) session.set({ lastRecording: path });
    if (error) toast(error, "error");
    else if (active) toast(t(MW, "Zeichne auf nach %1", path), "info");
    else if (path) toast(tn(MW, "Aufzeichnung gespeichert: %1 (%n Anfragen)", exchanges, path), "success");
  });

  on("health_result", ({ row }) => {
    session.set({ healthFindings: row.findings });
    const caption = row.grade === 0 ? t(MW, "Gut in Schuss") : row.grade === 1 ? t(MW, "Braucht Pflege") : t(MW, "Bitte ansehen");
    toast(t(MW, "Gesundheitscheck fertig: %1 Punkte.", row.score), row.grade === 0 ? "success" : "warning");
    emitApp("dashboard:health", { score: row.score, caption, findings: row.findings });
  });

  garageStore.subscribe(() => remindServices(false));
}

// Called by the connect page before every attempt (the log line and the
// capsule MainWindow set).
export function markConnecting(logLine: string): void {
  appendLog(logLine);
  setLink("connecting", t(MW, "Verbinde ..."));
}
