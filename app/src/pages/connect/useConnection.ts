// Connecting: the attempt for the chosen way, and the state of the
// connection from the backend events.
import { useState, type RefObject } from "react";
import { call, type ConnectHelp, type ConnectRoute, type VehicleGenerationInfo } from "../../api";
import { t, translator } from "../../i18n";
import { useBackendEvent } from "../../lib/hooks";
import { navigate } from "../../shell/nav";
import { garageStore, rememberedRoute } from "../../state/garage";
import { applySettings, markConnecting, setProMode, settingsStore } from "../../state/session";
import { ORDER_CAN_FIRST, ORDER_KLINE_FIRST, autoBle, canFdPossible, type ConnectLatest, type Mode } from "./shared";

const tr = translator("caruspex::ConnectPage");
// Texts of the truck option (SAE J1939).
const tk = translator("caruspex::Truck");
const MW = "caruspex::MainWindow";

export function useConnection(latest: RefObject<ConnectLatest>) {
  // Connection state
  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [stateTone, setStateTone] = useState<"muted" | "warn" | "good" | "bad">("muted");
  const [details, setDetails] = useState("");
  const [error, setError] = useState<string | null>(null);
  // The checklist of the connection assistant for the last failed attempt.
  const [help, setHelp] = useState<ConnectHelp | null>(null);

  // The protocol order to try first, from the vehicle choice or the settings.
  const protocolOrder = (g: VehicleGenerationInfo | null): string => {
    if (g?.protocolOrder) return g.protocolOrder;
    return settingsStore.get().values.protocolOrder === "can" ? ORDER_CAN_FIRST : ORDER_KLINE_FIRST;
  };

  // --- Connecting ----------------------------------------------------------------------------

  const showError = (text: string, checklist: ConnectHelp | null = null) => {
    setError(text);
    setHelp(checklist && checklist.items.length > 0 ? checklist : null);
  };

  const onConnecting = () => {
    setConnecting(true);
    setStateTone("warn");
  };

  const startConnect = (override?: { mode?: Mode; port?: string }) => {
    const s = latest.current;
    if (s.connected) {
      void call("close_adapter");
      return;
    }
    if (s.connecting) return;
    setError(null);
    const order = protocolOrder(s.generation);
    const values = settingsStore.get().values;
    // The J1939 bit rate when connecting to a truck, else left out.
    const truckRateArg = s.truck ? s.truckRate : undefined;
    // The way that worked last time, when it fits what is chosen now: the
    // backend tries it first and searches as usual only if it fails.
    const remembered = (kind: string, fits: (r: ConnectRoute) => boolean): ConnectRoute | undefined => {
      const route = rememberedRoute(garageStore.get().cars, kind);
      return route && fits(route) ? route : undefined;
    };
    switch (override?.mode ?? s.mode) {
      case "bmw":
        // The K+DCAN and KKL cable talk to the units on the BMW page.
        setProMode(true);
        navigate("bmw");
        return;
      case "cable": {
        const device = override?.port ?? s.port;
        markConnecting(device ? t(MW, "Verbinde über %1 ...", device) : t(MW, "Suche den Adapter an allen Anschlüssen ..."));
        // Remember the port so auto-connect knows where to go next time.
        if (device && values.lastPort !== device) void applySettings({ ...values, lastPort: device });
        const route = remembered("serial", (r) => !device || r.device === device);
        void call("open_serial", { device, baud: values.baudOverride, protocolOrder: order, remembered: route, truck: truckRateArg });
        break;
      }
      case "wifi": {
        const text = s.address.trim();
        const colon = text.indexOf(":");
        const host = colon < 0 ? text : text.slice(0, colon);
        // QString::toInt gives 0 for anything that is not a number.
        const parsed = colon < 0 ? 35000 : Number.parseInt(text.slice(colon + 1), 10);
        const portNumber = Number.isFinite(parsed) ? parsed : 0;
        markConnecting(t(MW, "Verbinde mit %1:%2 ...", host, portNumber));
        const route = remembered("tcp", (r) => r.device === `${host}:${portNumber}`);
        void call("open_tcp", { host, port: portNumber, protocolOrder: order, remembered: route, truck: truckRateArg });
        break;
      }
      case "demo":
        markConnecting(s.truck ? tk("Starte den Demo-Lkw ...") : t(MW, "Starte das Demo-Auto ..."));
        void call("open_demo", { protocolOrder: order, truck: truckRateArg });
        break;
      case "can": {
        const device = s.canDevices[s.canIndex]?.spec ?? "";
        if (!device) {
          showError(tr("Kein CAN-Interface gefunden. Treiber installiert und Gerät eingesteckt?"));
          return;
        }
        // "+fd" right before the bit rate switches CAN FD on.
        const fd = s.canFd && canFdPossible(device) && !s.truck ? "+fd" : "";
        // A truck: the bit rate of the J1939 choice (the backend tries both
        // with automatic).
        const rate = s.truck ? (s.truckRate || 250) * 1000 : s.canRate;
        const spec = `${device}${fd}@${rate}`;
        markConnecting(t(MW, "Verbinde über das CAN-Interface %1 ...", spec));
        const route = remembered("can", (r) => r.device.split("@")[0] === device);
        void call("open_can", { spec, protocolOrder: order, remembered: route, truck: truckRateArg });
        break;
      }
      case "enet": {
        const host = s.enetHost.trim();
        markConnecting(host ? t(MW, "Verbinde über ENET mit %1 ...", host) : t(MW, "Suche den BMW über ENET ..."));
        const route = remembered("enet", (r) => !host || r.device === host);
        void call("open_enet", { host, protocolOrder: order, remembered: route });
        break;
      }
      case "ble": {
        const item = s.ble.items.find((i) => i.id === s.ble.selected) ?? autoBle();
        markConnecting(
          item.id ? t(MW, "Verbinde über Bluetooth mit %1 ...", item.name || item.id) : t(MW, "Suche einen Bluetooth-Adapter ..."),
        );
        const route = remembered("ble", (r) => !item.id || r.device === item.id);
        void call("open_ble", { id: item.id, name: item.name, kinds: item.kinds, protocolOrder: order, remembered: route, truck: truckRateArg });
        break;
      }
      case "replay": {
        const path = s.replayPath.trim();
        if (!path) {
          showError(tr("Erst eine Aufnahme wählen."));
          return;
        }
        markConnecting(t(MW, "Spiele die Aufnahme %1 ab ...", path));
        void call("open_replay", { path, realTime: s.replayRealTime, protocolOrder: order });
        break;
      }
    }
    onConnecting();
  };

  // --- Backend events -------------------------------------------------------------------------

  useBackendEvent("connected", ({ adapter, transport, protocol }) => {
    setConnected(true);
    setConnecting(false);
    setStateTone("good");
    setDetails(tr("Adapter: %1\nWeg: %2\nProtokoll: %3", adapter, transport, protocol));
    setError(null);
  });

  useBackendEvent("failed", ({ error: text, help: checklist }) => {
    setConnecting(false);
    if (!latest.current.connected) setStateTone("bad");
    showError(text, checklist ?? null);
  });

  useBackendEvent("closed", () => {
    setConnected(false);
    setConnecting(false);
    setStateTone("muted");
    setDetails("");
  });

  return { connected, connecting, stateTone, details, error, help, startConnect };
}
