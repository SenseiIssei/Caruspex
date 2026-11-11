// Connecting, the fault memory, vehicle details, the live loop, raw
// commands and recording, as the C++ Worker does them with the demo car.
// Real hardware is not reachable from a browser, so every other way of
// connecting answers the way the Worker does when nothing is found.
import { t } from "../../i18n";
import type { FaultRow, FreezeFrameRow, LiveRow, StatusRow } from "../types";
import {
  ADAPTER_VERSION,
  DEMO_VIN,
  PROTOCOL_TEXT,
  simulate,
  type Sample,
} from "./car";
import { connectHelp, type ConnectFacts } from "./advice";
import { L } from "./core";
import { describeDtc } from "./dtc";
import { keysOfPid, liveRow, SUPPORTED_PIDS } from "./pids";
import { appSeconds, busy, car, emit, link, log, wait, type PartialHandlers } from "./runtime";

const W = "caruspex::Worker";

function engineEcu(): string {
  return L("Motor (7E8)", "Engine (7E8)");
}

function readiness(): { names: string[]; ready: boolean[] } {
  const b = [
    { mask: 0x01, de: "Verbrennungsaussetzer", en: "Misfire" },
    { mask: 0x02, de: "Kraftstoffsystem", en: "Fuel system" },
    { mask: 0x04, de: "Bauteilüberwachung", en: "Comprehensive components" },
  ];
  const c = [
    { mask: 0x01, de: "Katalysator", en: "Catalyst" },
    { mask: 0x02, de: "Katalysatorheizung", en: "Heated catalyst" },
    { mask: 0x04, de: "Tankentlüftung", en: "Evaporative system" },
    { mask: 0x08, de: "Sekundärluft", en: "Secondary air system" },
    { mask: 0x10, de: "Klimaanlage", en: "A/C refrigerant" },
    { mask: 0x20, de: "Lambdasonde", en: "Oxygen sensor" },
    { mask: 0x40, de: "Lambdasondenheizung", en: "Oxygen sensor heater" },
    { mask: 0x80, de: "Abgasrückführung", en: "EGR system" },
  ];
  const byteB = 0x07;
  const byteC = 0xe5;
  const byteD = car.readinessD();
  const names: string[] = [];
  const ready: boolean[] = [];
  for (const m of b) {
    if (byteB & m.mask) {
      names.push(L(m.de, m.en));
      ready.push((byteB & (m.mask << 4)) === 0);
    }
  }
  for (const m of c) {
    if (byteC & m.mask) {
      names.push(L(m.de, m.en));
      ready.push((byteD & m.mask) === 0);
    }
  }
  return { names, ready };
}

export function statusRow(): StatusRow {
  const stored = car.storedCodes();
  const r = readiness();
  return {
    valid: true,
    mil: stored.length > 0,
    faultCount: stored.length,
    monitorNames: r.names,
    monitorReady: r.ready,
  };
}

export function faultRows(): FaultRow[] {
  const rows: FaultRow[] = [];
  const add = (code: string, de: string, en: string, stored: boolean) => {
    if (rows.some((r) => r.code === code)) return;
    rows.push({
      code,
      kind: L(de, en),
      text: describeDtc(code),
      throttleRelated: false,
      stored,
      permanent: de === "dauerhaft",
      ecu: engineEcu(),
    });
  };
  for (const code of car.storedCodes()) add(code, "gespeichert", "stored", true);
  for (const code of car.pendingCodes()) add(code, "sporadisch", "intermittent", false);
  for (const code of car.permanentCodes()) add(code, "dauerhaft", "permanent", false);
  return rows;
}

function sampleRows(sample: Sample, keys: number[], t0: number): LiveRow[] {
  const out: LiveRow[] = [];
  for (const key of keys) {
    const row = liveRow(key, sample, car, t0, appSeconds());
    if (row) out.push(row);
  }
  return out;
}

const FREEZE_KEYS = [0x03, 0x04, 0x05, 0x06, 0x07, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11];

export function freezeFrames(): FreezeFrameRow[] {
  const frames: FreezeFrameRow[] = [];
  for (const frame of [0, 1]) {
    const f = car.freezeFrame(frame);
    if (!f) continue;
    frames.push({ frame, dtc: f.code, ecu: engineEcu(), values: sampleRows(f.sample, FREEZE_KEYS, car.now()) });
  }
  return frames;
}

export function readLive(key: number): LiveRow | null {
  const t0 = car.now();
  return liveRow(key, simulate(t0), car, t0, appSeconds());
}

export function refreshStatusAndFaults(): void {
  emit("busy", { active: true });
  emit("status", { row: statusRow() });
  const rows = faultRows();
  emit("faults", { rows });
  log(rows.length === 0 ? t(W, "Fehlerspeicher gelesen: kein Eintrag.") : t(W, "Fehlerspeicher gelesen: %1 Einträge.", rows.length));
  emit("supported_pids", { pids: SUPPORTED_PIDS });
  emit("busy", { active: false });
}

async function connectDemo(): Promise<void> {
  if (link.connected) closeLink(false);
  emit("busy", { active: true });
  log("ATZ");
  await wait(250);
  log(ADAPTER_VERSION);
  log("ATE0 ATL0 ATS1 ATH0 ATSP6");
  await wait(350);
  log(`0100: ${PROTOCOL_TEXT}`);
  await wait(250);
  car.restart();
  link.connected = true;
  emit("connected", { adapter: ADAPTER_VERSION, transport: L("Demo-Fahrzeug (eingebaut)", "Demo vehicle (built-in)"), protocol: PROTOCOL_TEXT });
  emit("busy", { active: false });
  refreshStatusAndFaults();
}

// A failed attempt, with the checklist of the connection assistant built
// from what the attempt would have found out in a browser.
async function noHardware(error: string, facts?: ConnectFacts): Promise<void> {
  await busy(() => wait(900));
  emit("failed", facts ? { error, help: connectHelp(facts) } : { error });
}

function stopLiveLoop(silent = false): void {
  link.customKeys = [];
  if (link.liveTimer) {
    clearInterval(link.liveTimer);
    link.liveTimer = 0;
    if (!silent) log(t(W, "Messung gestoppt."));
  }
}

function closeLink(notify = true): void {
  stopLiveLoop(true);
  if (link.recording) {
    emit("recording_state", { active: false, path: link.recording, exchanges: link.exchanges, error: "" });
    link.recording = "";
  }
  link.connected = false;
  if (notify) emit("closed", {});
}

// Hook for the manufacturer values: the live loop gives them every other
// tick, like Worker::pollTick. Set by the module that implements read_custom.
export let customTick: ((keys: number[]) => void) | null = null;
export function setCustomTick(fn: (keys: number[]) => void): void {
  customTick = fn;
}

function pollTick(): void {
  if (!link.connected) return;
  if (link.customKeys.length > 0) {
    link.customTurn = !link.customTurn;
    if (link.customTurn || link.livePids.length === 0) {
      customTick?.(link.customKeys);
      return;
    }
  }
  if (link.livePids.length === 0) return;
  const t0 = car.now();
  const sample = simulate(t0);
  for (let i = 0; i < 6 && i < link.livePids.length; i++) {
    const pid = link.livePids[link.liveIndex % link.livePids.length]!;
    link.liveIndex = (link.liveIndex + 1) % link.livePids.length;
    // A PID answers with all its values (0x01 lamp and count, 0x14 voltage and trim).
    const keys = pid > 0xff ? [pid] : keysOfPid(pid);
    for (const key of keys) {
      const row = liveRow(key, sample, car, t0, appSeconds());
      if (row) emit("live_value", { row });
    }
    if (link.recording) link.exchanges++;
  }
}

export function startLoop(): void {
  if (link.liveTimer) clearInterval(link.liveTimer);
  // The browser cannot keep up with 50 events a second for long; 40 ms is
  // close to what a quick adapter delivers.
  link.liveTimer = setInterval(pollTick, Math.max(40, link.pollIntervalMs));
}

function rawReply(command: string): string {
  const c = command.replace(/\s+/g, "").toUpperCase();
  const s = simulate(car.now());
  const hex = (v: number, digits = 2) => Math.max(0, Math.round(v)).toString(16).toUpperCase().padStart(digits, "0");
  const pair = (v: number) => {
    const h = hex(v, 4);
    return `${h.slice(0, 2)} ${h.slice(2)}`;
  };
  if (c === "ATZ") return ADAPTER_VERSION;
  if (c === "ATI") return ADAPTER_VERSION;
  if (c === "AT@1") return "OBDII to RS232 Interpreter";
  if (c === "ATRV") return `${s.battery.toFixed(1)}V`;
  if (c === "ATDP") return PROTOCOL_TEXT;
  if (c === "ATDPN") return "A6";
  if (c.startsWith("AT")) return "OK";
  if (c === "0100") return "41 00 BE 3F A8 13";
  if (c === "0105") return `41 05 ${hex(s.coolant + 40)}`;
  if (c === "010C") return `41 0C ${pair(s.rpm * 4)}`;
  if (c === "010D") return `41 0D ${hex(s.speed)}`;
  if (c === "0142") return `41 42 ${pair((s.battery - 0.25) * 1000)}`;
  if (c === "03") return car.storedCodes().length === 2 ? "43 02 01 71 04 20" : "43 00";
  if (c === "07") return car.pendingCodes().length > 0 ? "47 01 03 00" : "47 00";
  if (c === "0902") return "49 02 01 59 43 58 44 45 31 35 43 37 4D 43 30 34 38 32 31 37";
  if (/^[0-9A-F]+$/.test(c)) return "NO DATA";
  return "?";
}

export const sessionHandlers: PartialHandlers = {
  list_serial_ports: () => [],
  list_j2534_devices: () => [],
  open_demo: () => {
    void connectDemo();
  },
  open_serial: ({ device }) => {
    void noHardware(
      device === ""
        ? t(W, "Kein Anschluss gefunden. Steckt der Adapter am Rechner, und ist ein Bluetooth-Adapter gekoppelt?")
        : t(W, "An keinem Anschluss hat ein ELM327 geantwortet. Zündung an und Adapter richtig eingesteckt?"),
      device === ""
        ? { kind: "usb", portsFound: 0 }
        : { kind: "usb", portOpened: true, adapterAnswered: false, baudsTried: [38400, 9600, 115200, 500000] },
    );
  },
  open_tcp: ({ host, port }) => {
    void noHardware(L(`Keine Verbindung zu ${host}:${port}`, `Cannot connect to ${host}:${port}`), { kind: "wifi", portOpened: false });
  },
  open_enet: ({ host }) => {
    if (host === "") log(t(W, "Suche das Auto im Netzwerk (ENET und DoIP) ..."));
    void noHardware(L("Kein Fahrzeug im Netzwerk gefunden.", "No vehicle found on the network."), { kind: "network", portOpened: false });
  },
  open_can: ({ spec }) => {
    const rate = Number(spec.split("@").pop()) || 500000;
    void noHardware(L(`CAN-Schnittstelle ${spec} lässt sich nicht öffnen.`, `Cannot open the CAN interface ${spec}.`), {
      kind: "can",
      portOpened: false,
      canBitRate: rate,
    });
  },
  scan_ble: async ({ milliseconds }) => {
    await busy(() => wait(Math.min(milliseconds, 2500)));
    emit("ble_scan_finished", { devices: [], error: L("Bluetooth ist im Browser nicht verfügbar.", "Bluetooth is not available in the browser.") });
  },
  open_ble: () => {
    log(t(W, "Suche Bluetooth-Adapter ..."));
    void noHardware(L("Kein Bluetooth-Adapter gefunden.", "No Bluetooth adapter found."), { kind: "bluetooth", deviceFound: false });
  },
  open_replay: ({ path }) => {
    void noHardware(t(W, "Aufnahme lässt sich nicht öffnen: %1", path));
  },
  close_adapter: () => closeLink(true),
  start_recording: ({ path }) => {
    if (!link.connected) {
      emit("recording_state", { active: false, path, exchanges: 0, error: t(W, "Erst verbinden, dann aufzeichnen.") });
      return;
    }
    link.recording = path;
    link.exchanges = 0;
    emit("recording_state", { active: true, path, exchanges: 0, error: "" });
  },
  stop_recording: () => {
    const path = link.recording;
    link.recording = "";
    emit("recording_state", { active: false, path, exchanges: link.exchanges, error: "" });
  },
  read_faults: () => {
    if (link.connected) refreshStatusAndFaults();
  },
  clear_faults: async () => {
    if (!link.connected) return;
    await busy(async () => {
      await wait(600);
      car.clear();
      log(t(W, "Fehlerspeicher gelöscht."));
    });
    refreshStatusAndFaults();
  },
  read_details: async () => {
    if (!link.connected) return;
    await busy(async () => {
      await wait(300);
      const frames = freezeFrames();
      emit("vehicle_info", {
        vin: DEMO_VIN,
        freezeCode: frames[0]?.dtc ?? "",
        freezeRows: frames[0]?.values ?? [],
      });
      emit("freeze_frames", { frames });
      for (const pid of [0x05, 0x2f, 0x42, 0xa6]) {
        const row = readLive(pid);
        if (row) emit("live_value", { row });
      }
    });
  },
  start_live: ({ pids }) => {
    link.livePids = pids;
    if (!link.connected || pids.length === 0) return;
    link.liveIndex = 0;
    startLoop();
    log(t(W, "Messung gestartet, %1 Werte im Umlauf.", pids.length));
  },
  stop_live: () => stopLiveLoop(),
  send_raw: async ({ command }) => {
    await wait(120);
    emit("raw_reply", { reply: link.connected ? rawReply(command) : t(W, "(keine Antwort)") });
  },
  bruteforce_clear: async ({ attempts }) => {
    if (!link.connected) {
      emit("clear_progress", { line: t(W, "Nicht verbunden.") });
      emit("clear_finished", { everythingGone: false, remaining: 0 });
      return;
    }
    await busy(async () => {
      for (let attempt = 1; attempt <= attempts; attempt++) {
        emit("clear_progress", { line: t(W, "Versuch %1 von %2 ...", attempt, attempts) });
        await wait(700);
        car.clear();
        // Permanent codes stay until their monitor passes, so the demo
        // memory is never fully empty right after clearing.
        const remaining = faultRows().length;
        if (remaining === 0) {
          emit("clear_progress", { line: t(W, " Speicher ist leer.") });
          emit("clear_finished", { everythingGone: true, remaining: 0 });
          return;
        }
        emit("clear_progress", { line: t(W, " noch %1 Einträge", remaining) });
        if (attempt === attempts) {
          emit("clear_finished", { everythingGone: false, remaining });
        }
      }
    });
    refreshStatusAndFaults();
  },
  cancel_scan: () => {
    link.cancel = true;
  },
};

// Used by the settings handler: the poll interval takes effect at once.
export function applyPollInterval(ms: number): void {
  link.pollIntervalMs = ms;
  if (link.liveTimer) startLoop();
}

