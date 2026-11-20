// What the connection page and its parts share: the kinds of adapter, the
// remembered Bluetooth adapter and a few small helpers.
import type { SerialPortInfo, VehicleGenerationInfo } from "../../api";
import { translator } from "../../i18n";

const tr = translator("caruspex::ConnectPage");

// CAN FD goes through SocketCAN and J2534 devices only.
export const canFdPossible = (spec: string) => spec.startsWith("socketcan:") || spec.startsWith("j2534:");

// The values of core/Elm327.cpp: kProtocolOrderKLineFirst, kProtocolOrderCanFirst.
export const ORDER_KLINE_FIRST = "54312678";
export const ORDER_CAN_FIRST = "67891543";

// "bmw": BMW's E series on the K-line, which goes to its own page.
export type Mode = "cable" | "wifi" | "demo" | "replay" | "ble" | "enet" | "can" | "bmw";

export interface BleChoice {
  id: string;
  name: string;
  kinds: number;
  label: string;
}

export interface CanChoice {
  label: string;
  // Empty for a J2534 device that only has a 32 bit driver.
  spec: string;
}

export const CAN_RATES = [500000, 250000, 125000, 1000000];

// The last Bluetooth adapter is offered again without a new scan
// (QSettings connect/bleId, connect/bleName, connect/bleKinds).
export const BLE_KEYS = { id: "caruspex.connect.bleId", name: "caruspex.connect.bleName", kinds: "caruspex.connect.bleKinds" };

function readStored(key: string): string {
  try {
    return localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

export function writeStored(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Private window or blocked storage: the adapter is simply not remembered.
  }
}

export function autoBle(): BleChoice {
  return { id: "", name: "", kinds: 0, label: tr("Automatisch suchen") };
}

export function initialBle(): { items: BleChoice[]; selected: string } {
  const items = [autoBle()];
  const lastId = readStored(BLE_KEYS.id);
  if (!lastId) return { items, selected: "" };
  const lastName = readStored(BLE_KEYS.name);
  const kinds = Number(readStored(BLE_KEYS.kinds) || "1") || 1;
  items.push({ id: lastId, name: lastName, kinds, label: lastName || lastId });
  return { items, selected: lastId };
}

export function currentYear(): number {
  return new Date().getFullYear();
}

export function portLabel(p: SerialPortInfo): string {
  let label = p.device;
  if (p.description) label += `  ·  ${p.description}`;
  if (p.chipGuess) label += `  [${p.chipGuess}]`;
  return label;
}

export const defaultSocketHint = () =>
  tr(
    "Die OBD-Buchse sitzt fast immer im Fußraum auf der Fahrerseite, " +
      "unter dem Lenkrad oder hinter einer kleinen Klappe.",
  );

// startConnect of the page: connects with the current choice, or with
// another mode or port that was just set.
export type StartConnect = (override?: { mode?: Mode; port?: string }) => void;

// The values the handlers read when they run outside a render (app events
// that connect right after changing the mode or the port).
export interface ConnectLatest {
  mode: Mode;
  port: string;
  address: string;
  replayPath: string;
  replayRealTime: boolean;
  ble: { items: BleChoice[]; selected: string };
  enetHost: string;
  canDevices: CanChoice[];
  canIndex: number;
  canRate: number;
  canFd: boolean;
  truck: boolean;
  truckRate: number;
  generation: VehicleGenerationInfo | null;
  connected: boolean;
  connecting: boolean;
}
