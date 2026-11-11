// Connecting, step by step: where the socket is, which adapter, then one
// button (ConnectPage.cpp). The page keeps its own picture of the
// connection from the backend events, like the Qt page did with the
// Worker signals MainWindow connected to it, and writes the log lines
// MainWindow wrote before each attempt.
import { useCallback, useEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import { call, type BleDeviceRow, type ConnectHelp, type SerialPortInfo, type VehicleGenerationInfo } from "../api";
import { Icon, type IconName } from "../components/Icon";
import { TermStrip } from "../components/Term";
import { Button, Card, Check, Chip, Page, PageHeader, Select, Spinner, TextInput, cx } from "../components/ui";
import { t, translator } from "../i18n";
import { ACCESS } from "../lib/access";
import { useAppEvent, useBackendEvent } from "../lib/hooks";
import { navigate } from "../shell/nav";
import { appendLog, applySettings, markConnecting, settingsStore } from "../state/session";
import { openSetup } from "../state/setup";
import type { SetupChoice } from "../shell/SetupAssistant";
import "./ConnectPage.css";

const tr = translator("caruspex::ConnectPage");
// Texts that are new in the connection assistant.
const tc = translator("caruspex::ConnectHelp");
const MW = "caruspex::MainWindow";

// The values of core/Elm327.cpp: kProtocolOrderKLineFirst, kProtocolOrderCanFirst.
const ORDER_KLINE_FIRST = "54312678";
const ORDER_CAN_FIRST = "67891543";

type Mode = "cable" | "wifi" | "demo" | "replay" | "ble" | "enet" | "can";

interface BleChoice {
  id: string;
  name: string;
  kinds: number;
  label: string;
}

interface CanChoice {
  label: string;
  // Empty for a J2534 device that only has a 32 bit driver.
  spec: string;
}

const CAN_RATES = [500000, 250000, 125000, 1000000];

// The last Bluetooth adapter is offered again without a new scan
// (QSettings connect/bleId, connect/bleName, connect/bleKinds).
const BLE_KEYS = { id: "caruspex.connect.bleId", name: "caruspex.connect.bleName", kinds: "caruspex.connect.bleKinds" };

function readStored(key: string): string {
  try {
    return localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

function writeStored(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Private window or blocked storage: the adapter is simply not remembered.
  }
}

function autoBle(): BleChoice {
  return { id: "", name: "", kinds: 0, label: tr("Automatisch suchen") };
}

function initialBle(): { items: BleChoice[]; selected: string } {
  const items = [autoBle()];
  const lastId = readStored(BLE_KEYS.id);
  if (!lastId) return { items, selected: "" };
  const lastName = readStored(BLE_KEYS.name);
  const kinds = Number(readStored(BLE_KEYS.kinds) || "1") || 1;
  items.push({ id: lastId, name: lastName, kinds, label: lastName || lastId });
  return { items, selected: lastId };
}

function currentYear(): number {
  return new Date().getFullYear();
}

function portLabel(p: SerialPortInfo): string {
  let label = p.device;
  if (p.description) label += `  ·  ${p.description}`;
  if (p.chipGuess) label += `  [${p.chipGuess}]`;
  return label;
}

const defaultSocketHint = () =>
  tr(
    "Die OBD-Buchse sitzt fast immer im Fußraum auf der Fahrerseite, " +
      "unter dem Lenkrad oder hinter einer kleinen Klappe.",
  );

// One numbered step of the "how to" card, with the gradient circle.
function Step({ n, icon, title, text }: { n: number; icon: IconName; title: string; text: string }) {
  return (
    <div className="connect-step">
      <div className="connect-step-number">{n}</div>
      <div className="connect-step-texts">
        <div className="connect-step-head">
          <Icon name={icon} size={16} className="muted" />
          <span>{title}</span>
        </div>
        <div className="subtle">{text}</div>
      </div>
    </div>
  );
}

// The connection assistant: what the attempt found out, then the points to
// check, the likely reason first.
const CHECK_LOOK: Record<string, { icon: IconName; tone: string }> = {
  problem: { icon: "x-circle", tone: "var(--bad)" },
  check: { icon: "info", tone: "var(--warn)" },
  fine: { icon: "check-circle", tone: "var(--good)" },
};

function ConnectChecklist({ help }: { help: ConnectHelp }) {
  return (
    <div className="connect-help fade-in">
      {help.found.length > 0 ? (
        <div className="connect-help-found">
          <div className="connect-help-head">{tc("Was Caruspex herausgefunden hat")}</div>
          <ul>
            {help.found.map((line) => (
              <li key={line} className="selectable">
                {line}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      <div className="connect-help-head">{tc("Das kannst du prüfen")}</div>
      <div className="connect-help-list">
        {help.items.map((item) => {
          const look = CHECK_LOOK[item.state] ?? CHECK_LOOK.check!;
          return (
            <div key={item.id} className={`connect-check ${item.state}`} style={{ "--tone": look.tone } as CSSProperties}>
              <Icon name={look.icon} size={18} className="connect-check-icon" />
              <div className="connect-check-texts">
                <div className="connect-check-title">{item.title}</div>
                {item.text ? <div className="subtle">{item.text}</div> : null}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ModeChip({ mode, current, icon, label, onPick }: { mode: Mode; current: Mode; icon: IconName; label: string; onPick: (m: Mode) => void }) {
  return (
    <Chip on={mode === current} onClick={() => onPick(mode)}>
      <Icon name={icon} size={16} />
      {label}
    </Chip>
  );
}

// The square refresh button next to a list.
function RefreshButton({ title, onClick, disabled }: { title?: string; onClick: () => void; disabled?: boolean }) {
  return <Button icon="refresh" className="connect-refresh" title={title} aria-label={title} onClick={onClick} disabled={disabled} />;
}

export default function ConnectPage() {
  const [mode, setModeState] = useState<Mode>("cable");
  const [hasBle, setHasBle] = useState(true);

  // Cable
  const [ports, setPorts] = useState<SerialPortInfo[]>([]);
  const [port, setPort] = useState("");
  // Wi-Fi
  const [address, setAddress] = useState("192.168.0.10:35000");
  // Replay
  const [replayPath, setReplayPath] = useState("");
  const [replayRealTime, setReplayRealTime] = useState(false);
  // Bluetooth
  const [ble, setBle] = useState(initialBle);
  const [bleScanning, setBleScanning] = useState(false);
  const [bleNote, setBleNote] = useState(() =>
    tr(
      "Adapter mit Bluetooth LE (Vgate iCar Pro BLE, vLinker, OBDLink CX, " +
        "Veepeak BLE) findet Caruspex direkt, ohne Koppeln. Klassische " +
        "Bluetooth-Adapter einmal in den Systemeinstellungen koppeln, die PIN " +
        "ist meist 1234 oder 0000, danach stehen sie hier in der Liste.",
    ),
  );
  // ENET
  const [enetHost, setEnetHost] = useState("");
  // CAN
  const [canDevices, setCanDevices] = useState<CanChoice[]>([]);
  const [canIndex, setCanIndex] = useState(0);
  const [canRate, setCanRate] = useState(500000);

  // Vehicle
  const [vehicleText, setVehicleText] = useState("");
  const [suggestions, setSuggestions] = useState<VehicleGenerationInfo[]>([]);
  const [suggestOpen, setSuggestOpen] = useState(false);
  const [suggestIndex, setSuggestIndex] = useState(-1);
  const [generation, setGenerationState] = useState<VehicleGenerationInfo | null>(null);
  const [year, setYear] = useState(2010);
  const [yearRange, setYearRange] = useState<[number, number]>([1980, currentYear() + 1]);
  const [profileNote, setProfileNote] = useState("");
  const [socketHint, setSocketHint] = useState(defaultSocketHint);

  // Connection state
  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [stateTone, setStateTone] = useState<"muted" | "warn" | "good" | "bad">("muted");
  const [details, setDetails] = useState("");
  const [error, setError] = useState<string | null>(null);
  // The checklist of the connection assistant for the last failed attempt.
  const [help, setHelp] = useState<ConnectHelp | null>(null);

  // The latest values for the handlers that run outside a render (app events
  // that connect right after changing the mode or the port).
  const latest = useRef({ mode, port, address, replayPath, replayRealTime, ble, enetHost, canDevices, canIndex, canRate, generation, connected, connecting });
  latest.current = { mode, port, address, replayPath, replayRealTime, ble, enetHost, canDevices, canIndex, canRate, generation, connected, connecting };
  const searchSeq = useRef(0);

  // --- Lists -----------------------------------------------------------------------------

  // Fills the port list and keeps the previous choice when it is still there.
  const refreshPorts = useCallback(async (): Promise<SerialPortInfo[]> => {
    let list: SerialPortInfo[] = [];
    try {
      list = await call("list_serial_ports");
    } catch {
      list = [];
    }
    setPorts(list);
    setPort((previous) => (previous && list.some((p) => p.device === previous) ? previous : ""));
    return list;
  }, []);

  // Lists the CAN interfaces of this computer again: J2534, SocketCAN, then
  // every serial port as a possible SLCAN stick.
  const refreshCanDevices = useCallback(async () => {
    const [j2534, socketCan, serial] = await Promise.all([
      call("list_j2534_devices").catch(() => []),
      call("list_socketcan_interfaces").catch(() => [] as string[]),
      call("list_serial_ports").catch(() => [] as SerialPortInfo[]),
    ]);
    const items: CanChoice[] = [];
    for (const d of j2534 ?? []) {
      let label = `J2534 · ${d.name}`;
      if (!d.usable) label += ` · ${tr("nur 32-Bit-Treiber")}`;
      items.push({ label, spec: d.usable ? `j2534:${d.library}` : "" });
    }
    for (const name of socketCan ?? []) items.push({ label: `SocketCAN · ${name}`, spec: `socketcan:${name}` });
    for (const p of serial ?? []) items.push({ label: `SLCAN · ${p.device}`, spec: `slcan:${p.device}` });
    const { canDevices: before, canIndex: index } = latest.current;
    const previous = before[index]?.spec ?? "";
    const keep = items.findIndex((i) => i.spec === previous);
    setCanDevices(items);
    setCanIndex(keep >= 0 ? keep : 0);
  }, []);

  useEffect(() => {
    void refreshPorts();
    void refreshCanDevices();
    call("app_info")
      .then((info) => setHasBle(info?.hasBle ?? true))
      .catch(() => undefined);
  }, [refreshPorts, refreshCanDevices]);

  // --- Vehicle -----------------------------------------------------------------------------

  // Preselects a car from the catalog (or none for automatic).
  const setGeneration = useCallback((g: VehicleGenerationInfo | null, wanted: number) => {
    setGenerationState(g);
    if (g) {
      const last = g.yearTo > 0 ? g.yearTo : currentYear();
      setYearRange([g.yearFrom, last]);
      setYear(Math.min(last, Math.max(g.yearFrom, wanted)));
      setVehicleText(g.displayName);
    } else {
      setYearRange([1980, currentYear() + 1]);
    }
  }, []);

  // The profile note and the socket hint follow the vehicle and the year
  // (updateProfileNote).
  useEffect(() => {
    if (!generation) {
      setProfileNote(
        tr(
          "Ohne Auswahl probiert Caruspex die Protokolle der Reihe nach " +
            "durch und erkennt das Auto danach an der Fahrgestellnummer.",
        ),
      );
      setSocketHint(defaultSocketHint());
      return;
    }
    let cancelled = false;
    call("catalog_diagnosis", { generationId: generation.id, year })
      .then((info) => {
        if (cancelled || !info) return;
        setProfileNote(info.diagnosisText);
        setSocketHint(tr("Bei diesem Auto: %1", info.locationText));
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [generation, year]);

  const onVehicleEdited = (value: string) => {
    setVehicleText(value);
    const query = value.trim();
    const seq = ++searchSeq.current;
    if (query.length >= 2) {
      call("catalog_search", { query, limit: 15 })
        .then((found) => {
          if (seq !== searchSeq.current) return;
          setSuggestions(found ?? []);
          setSuggestIndex(-1);
          setSuggestOpen((found ?? []).length > 0);
        })
        .catch(() => undefined);
    } else {
      setSuggestions([]);
      setSuggestOpen(false);
    }
    if (query === "") setGeneration(null, year);
  };

  const activateSuggestion = (g: VehicleGenerationInfo) => {
    const last = g.yearTo > 0 ? g.yearTo : currentYear();
    setSuggestOpen(false);
    setGeneration(g, Math.min(last, Math.max(g.yearFrom, year)));
  };

  const onVehicleKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (!suggestOpen || suggestions.length === 0) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSuggestIndex((i) => (i + 1) % suggestions.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSuggestIndex((i) => (i <= 0 ? suggestions.length - 1 : i - 1));
    } else if (e.key === "Enter" && suggestIndex >= 0) {
      e.preventDefault();
      activateSuggestion(suggestions[suggestIndex]!);
    } else if (e.key === "Escape") {
      setSuggestOpen(false);
    }
  };

  const clearVehicle = () => {
    searchSeq.current++;
    setVehicleText("");
    setSuggestions([]);
    setSuggestOpen(false);
    setGeneration(null, year);
  };

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
    switch (override?.mode ?? s.mode) {
      case "cable": {
        const device = override?.port ?? s.port;
        markConnecting(device ? t(MW, "Verbinde über %1 ...", device) : t(MW, "Suche den Adapter an allen Anschlüssen ..."));
        // Remember the port so auto-connect knows where to go next time.
        if (device && values.lastPort !== device) void applySettings({ ...values, lastPort: device });
        void call("open_serial", { device, baud: values.baudOverride, protocolOrder: order });
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
        void call("open_tcp", { host, port: portNumber, protocolOrder: order });
        break;
      }
      case "demo":
        markConnecting(t(MW, "Starte das Demo-Auto ..."));
        void call("open_demo", { protocolOrder: order });
        break;
      case "can": {
        const device = s.canDevices[s.canIndex]?.spec ?? "";
        if (!device) {
          showError(tr("Kein CAN-Interface gefunden. Treiber installiert und Gerät eingesteckt?"));
          return;
        }
        const spec = `${device}@${s.canRate}`;
        markConnecting(t(MW, "Verbinde über das CAN-Interface %1 ...", spec));
        void call("open_can", { spec, protocolOrder: order });
        break;
      }
      case "enet": {
        const host = s.enetHost.trim();
        markConnecting(host ? t(MW, "Verbinde über ENET mit %1 ...", host) : t(MW, "Suche den BMW über ENET ..."));
        void call("open_enet", { host, protocolOrder: order });
        break;
      }
      case "ble": {
        const item = s.ble.items.find((i) => i.id === s.ble.selected) ?? autoBle();
        markConnecting(
          item.id ? t(MW, "Verbinde über Bluetooth mit %1 ...", item.name || item.id) : t(MW, "Suche einen Bluetooth-Adapter ..."),
        );
        void call("open_ble", { id: item.id, name: item.name, kinds: item.kinds, protocolOrder: order });
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

  const setMode = (m: Mode) => setModeState(m);

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

  // Result of a Bluetooth scan (Worker::bleScanFinished).
  useBackendEvent("ble_scan_finished", ({ devices, error: scanError }) => {
    setBleScanning(false);
    const items: BleChoice[] = [autoBle()];
    let adapters = 0;
    for (const d of devices as BleDeviceRow[]) {
      const adapter = d.score >= 3;
      if (adapter) adapters++;
      const kind = d.kinds || 1;
      let label = d.name ? d.name : tr("Unbenanntes Gerät (%1)", d.id);
      label += `  ·  ${kind === 2 ? tr("klassisch") : kind === 3 ? tr("LE und klassisch") : tr("LE")}`;
      if (adapter) label += `  ·  ${tr("OBD-Adapter")}`;
      items.push({ id: d.id, name: d.name, kinds: kind, label });
    }
    setBle((before) => {
      const keep = before.selected && items.some((i) => i.id === before.selected) ? before.selected : null;
      return { items, selected: keep ?? (adapters > 0 ? items[1]!.id : "") };
    });
    if (scanError) {
      setBleNote(scanError);
    } else if (devices.length === 0) {
      setBleNote(
        tr(
          "Kein Bluetooth-Gerät gefunden. Steckt der Adapter im Auto, und " +
            "ist die Zündung an? Manche Adapter schlafen ein und wachen erst nach " +
            "einem Druck auf ihren Knopf wieder auf.",
        ),
      );
    } else {
      const count = devices.length === 1 ? tr("Ein Gerät gefunden") : tr("%1 Geräte gefunden", devices.length);
      setBleNote(
        adapters === 0
          ? tr("%1, keins davon sieht nach einem OBD-Adapter aus. Unbekannte " + "Adapter lassen sich trotzdem wählen.", count)
          : tr("%1, davon %2 als OBD-Adapter erkannt.", count, adapters),
      );
    }
  });

  // Remembers the device the Bluetooth connection went to.
  useBackendEvent("ble_adapter_found", ({ id, name, kinds }) => {
    writeStored(BLE_KEYS.id, id);
    writeStored(BLE_KEYS.name, name);
    writeStored(BLE_KEYS.kinds, String(kinds));
    setBle((before) => {
      const items = before.items.some((i) => i.id === id) ? before.items : [...before.items, { id, name, kinds, label: name || id }];
      return { items, selected: id };
    });
  });

  // --- App events (the hand-overs MainWindow did) ------------------------------------------------

  // Auto-connect from the settings: select the saved port and connect.
  useAppEvent<string>("connect:auto", (lastPort) => {
    void (async () => {
      const list = await refreshPorts();
      if (list.some((p) => p.device === lastPort)) {
        setPort(lastPort);
        appendLog(t(MW, "Verbinde selbsttätig mit %1, so eingestellt.", lastPort));
        setTimeout(() => startConnect({ port: lastPort }), 0);
      } else {
        appendLog(t(MW, "Selbsttätiges Verbinden: Anschluss %1 ist nicht da.", lastPort));
      }
    })();
  });

  // ?demo: straight to the built-in demo car (autoConnectDemo).
  useAppEvent("connect:demo", () => {
    setMode("demo");
    startConnect({ mode: "demo" });
  });

  // The automatic search found the adapter on this port.
  useAppEvent<string>("connect:port-found", (device) => {
    void refreshPorts().then((list) => {
      if (list.some((p) => p.device === device)) setPort(device);
    });
  });

  // A car chosen on the vehicles page.
  useAppEvent<{ generation: VehicleGenerationInfo; year: number }>("connect:generation", (choice) => {
    if (choice?.generation) setGeneration(choice.generation, choice.year);
  });

  // The first-start assistant hands over the adapter, the car and maybe
  // the wish to connect right away.
  useAppEvent<SetupChoice>("connect:setup", (choice) => {
    setMode(choice.mode);
    if (choice.generation) setGeneration(choice.generation, choice.year);
    // After the render, so the connection uses the new choice.
    if (choice.connect) setTimeout(() => startConnect({ mode: choice.mode }), 60);
  });

  // --page connect-help: a Wi-Fi attempt to a closed port of this computer,
  // to see the connection assistant without any adapter.
  useAppEvent<string>("dev:page", (name) => {
    if (name !== "connect-help") return;
    navigate("connect");
    setMode("wifi");
    setAddress("127.0.0.1:1");
    latest.current.address = "127.0.0.1:1";
    startConnect({ mode: "wifi" });
  });

  // Opens the Bluetooth choice and scans, without connecting.
  useAppEvent<string>("dev:page", (name) => {
    if (name !== "connect-ble") return;
    navigate("connect");
    setMode("ble");
    void call("scan_ble", { milliseconds: 5000 });
  });

  // --- Rendering ---------------------------------------------------------------------------------

  const pickReplay = async () => {
    const [recording, all] = tr("Caruspex-Aufnahme (*.jsonl);;Alle Dateien (*)").split(";;");
    const path = await call("pick_open_path", {
      title: tr("Aufnahme wählen"),
      filters: [
        { name: recording ?? "", extensions: ["jsonl"] },
        { name: all ?? "", extensions: ["*"] },
      ],
    }).catch(() => null);
    if (path) setReplayPath(path);
  };

  const scanBle = () => {
    setBleScanning(true);
    setBleNote(tr("Suche Bluetooth-Geräte, das dauert bis zu einer halben Minute ..."));
    void call("scan_ble", { milliseconds: 8000 });
  };

  const portOptions = [{ value: "", label: tr("Automatisch suchen") }, ...ports.map((p) => ({ value: p.device, label: portLabel(p) }))];
  const bleOptions = ble.items.map((i) => ({ value: i.id, label: i.label }));
  const canOptions = canDevices.map((d, i) => ({ value: i, label: d.label }));
  const rateOptions = CAN_RATES.map((r) => ({ value: r, label: `${r / 1000} kbit/s` }));
  const stateText = connecting ? tr("Verbinde ...") : connected ? tr("Verbunden") : tr("Nicht verbunden");

  return (
    <Page className="connect-page">
      <PageHeader
        eyebrow={tr("Verbindung")}
        title={tr("Mit dem Auto verbinden")}
        subtitle={tr(
          "Ein Adapter in der OBD-Buchse, die Zündung an, ein Klick. " +
            "Welches Protokoll das Auto spricht, findet Caruspex " +
            "selbst heraus.",
        )}
        actions={
          <Button icon="sparkles" onClick={() => openSetup()}>
            {t("caruspex::Usability", "Einrichtungs-Assistent")}
          </Button>
        }
      />
      <TermStrip ids={["obd", "elm327", "kkl", "can", "vin"]} />

      <div className="connect-columns">
        <Card className="connect-steps" title={tr("So geht's")}>
          <Step n={1} icon="map-pin" title={tr("Buchse finden")} text={socketHint} />
          <Step
            n={2}
            icon="cable"
            title={tr("Adapter einstecken")}
            text={tr(
              "ELM327 per USB, Bluetooth oder WLAN. Für Airbag, ABS und " +
                "Grundeinstellungen bei VAG bis etwa 2004 ein KKL-Kabel.",
            )}
          />
          <Step
            n={3}
            icon="key"
            title={tr("Zündung an")}
            text={tr(
              "Zündung einschalten. Der Motor darf aus sein, für Live-Werte " + "darf er laufen, aber nur im Freien.",
            )}
          />
          <Step
            n={4}
            icon="plug"
            title={tr("Verbinden")}
            text={tr(
              "Rechts auf Verbinden drücken. Beim ersten Mal dauert es ein " +
                "paar Sekunden, bis das Protokoll gefunden ist.",
            )}
          />
        </Card>

        <Card className="connect-adapter" title={tr("Adapter")}>
          {/* Two rows, so the card keeps its width in every language: the
              cables first, then the ways without one. */}
          <div className="connect-chips">
            <ModeChip mode="cable" current={mode} icon="usb" label={tr("USB oder COM-Port")} onPick={setMode} />
            {hasBle ? <ModeChip mode="ble" current={mode} icon="bluetooth" label={tr("Bluetooth")} onPick={setMode} /> : null}
            <ModeChip mode="wifi" current={mode} icon="wifi" label={tr("WLAN")} onPick={setMode} />
            <ModeChip mode="enet" current={mode} icon="ethernet" label={tr("Netzwerkkabel")} onPick={setMode} />
          </div>
          <div className="connect-chips">
            <ModeChip mode="can" current={mode} icon="cpu" label={tr("CAN-Interface")} onPick={setMode} />
            <ModeChip mode="demo" current={mode} icon="sparkles" label={tr("Demo-Auto")} onPick={setMode} />
            <ModeChip mode="replay" current={mode} icon="play" label={tr("Aufnahme")} onPick={setMode} />
          </div>

          <div className="connect-mode fade-in" key={mode}>
            {mode === "cable" ? (
              <>
                <div className="connect-line">
                  <Select className="grow" value={port} options={portOptions} onChange={setPort} />
                  <RefreshButton title={tr("Anschlüsse neu suchen")} onClick={() => void refreshPorts()} />
                </div>
                <div className="hint">
                  {tr(
                    "Bluetooth-Adapter erscheinen nach dem Koppeln in Windows als " +
                      "eigener COM-Anschluss. Bei Automatisch probiert Caruspex alle " +
                      "Anschlüsse der Reihe nach.",
                  )}
                </div>
              </>
            ) : null}

            {mode === "wifi" ? (
              <>
                <TextInput className="connect-field ltr" value={address} onChange={setAddress} onKeyDown={(e) => e.key === "Enter" && startConnect()} />
                <div className="hint">
                  {tr("Erst mit dem WLAN des Adapters verbinden. Die meisten nutzen " + "192.168.0.10, Port 35000.")}
                </div>
              </>
            ) : null}

            {mode === "demo" ? (
              <div className="subtle connect-demo-text">
                {tr(
                  "Ein nachgebautes Auto im Programm: es fährt, wärmt sich auf, " +
                    "hat ein paar Fehler gespeichert. Ideal zum Ausprobieren, ganz " +
                    "ohne Adapter.",
                )}
              </div>
            ) : null}

            {mode === "replay" ? (
              <>
                <div className="connect-line">
                  <TextInput
                    className="grow connect-field"
                    value={replayPath}
                    onChange={setReplayPath}
                    placeholder={tr("Aufnahme wählen (.jsonl)")}
                  />
                  <Button className="connect-tall" onClick={() => void pickReplay()}>
                    {tr("Wählen")}
                  </Button>
                </div>
                <Check checked={replayRealTime} onChange={setReplayRealTime}>
                  {tr("Im ursprünglichen Tempo abspielen")}
                </Check>
                <div className="hint">
                  {tr(
                    "Spielt eine aufgezeichnete Sitzung ab, als wäre das Auto da. Gut für " +
                      "die Fehlersuche, oder wenn jemand eine Aufnahme schickt.",
                  )}
                </div>
              </>
            ) : null}

            {mode === "ble" ? (
              <>
                <div className="connect-line">
                  <Select
                    className="grow"
                    value={ble.selected}
                    options={bleOptions}
                    onChange={(id) => setBle((b) => ({ ...b, selected: id }))}
                  />
                  <RefreshButton title={tr("Nach Bluetooth-Geräten suchen")} onClick={scanBle} disabled={bleScanning} />
                </div>
                <div className="hint">{bleNote}</div>
              </>
            ) : null}

            {mode === "enet" ? (
              <>
                <TextInput
                  className="connect-field"
                  value={enetHost}
                  onChange={setEnetHost}
                  placeholder={tr("Automatisch suchen, oder die Adresse des Autos (169.254.x.x)")}
                  onKeyDown={(e) => e.key === "Enter" && startConnect()}
                />
                <div className="hint">
                  {tr(
                    "Für ein ENET-Kabel (BMW und Mini ab der F-Baureihe) oder ein DoIP-Kabel " +
                      "(viele neuere Autos) zwischen OBD-Buchse und Netzwerkanschluss. " +
                      "Caruspex erkennt selbst, was das Auto spricht. Zündung an; nach dem " +
                      "Einstecken braucht der Rechner bis zu einer Minute für seine " +
                      "automatische Adresse. Caruspex liest nur, Codieren und Flashen gehören " +
                      "in die Werkstattsoftware.",
                  )}
                </div>
              </>
            ) : null}

            {mode === "can" ? (
              <>
                <div className="connect-line">
                  <Select className="grow" value={canIndex} options={canOptions} onChange={setCanIndex} disabled={canOptions.length === 0} />
                  <Select className="connect-rate" value={canRate} options={rateOptions} onChange={setCanRate} />
                  <RefreshButton title={tr("Anschlüsse neu suchen")} onClick={() => void refreshCanDevices()} />
                </div>
                <div className="hint">
                  {tr(
                    "Für Geräte, die rohe CAN-Nachrichten liefern: J2534-PassThru (Tactrix OpenPort, " +
                      "Mongoose, VCX Nano, Scanmatik und viele mehr, mit dem 64-Bit-Treiber des " +
                      "Herstellers), SLCAN-Sticks (CANable, USBtin, CANUSB) und unter Linux SocketCAN. " +
                      "Autos mit OBD über CAN fahren mit 500 kbit/s.",
                  )}
                </div>
              </>
            ) : null}
          </div>

          <hr className="divider" />

          <div className="connect-vehicle-title">
            <h2 className="section-title">{tr("Fahrzeug")}</h2>
            <span className="hint">{tr("freiwillig")}</span>
          </div>
          <div className="connect-line connect-vehicle-row">
            <div className="connect-search grow">
              <input
                className="input connect-field"
                value={vehicleText}
                placeholder={tr("Automatisch erkennen, oder suchen: Golf 4, A2, E46 ...")}
                onChange={(e) => onVehicleEdited(e.target.value)}
                onKeyDown={onVehicleKey}
                onFocus={() => suggestions.length > 0 && setSuggestOpen(true)}
                onBlur={() => setTimeout(() => setSuggestOpen(false), 150)}
                role="combobox"
                aria-expanded={suggestOpen}
                aria-autocomplete="list"
              />
              {vehicleText ? (
                <button type="button" className="connect-clear" onClick={clearVehicle} tabIndex={-1} aria-label={t(ACCESS, "Eingabe leeren")}>
                  <Icon name="x" size={14} />
                </button>
              ) : null}
              {suggestOpen && suggestions.length > 0 ? (
                <ul className="connect-suggestions fade-in" role="listbox">
                  {suggestions.slice(0, 15).map((g, i) => (
                    <li
                      key={g.id}
                      role="option"
                      aria-selected={i === suggestIndex}
                      className={cx(i === suggestIndex && "on")}
                      onMouseDown={(e) => {
                        e.preventDefault();
                        activateSuggestion(g);
                      }}
                      onMouseEnter={() => setSuggestIndex(i)}
                    >
                      <span>{g.displayName}</span>
                      <span className="faint">({g.yearsText})</span>
                    </li>
                  ))}
                </ul>
              ) : null}
            </div>
            <label className="connect-year input">
              <span className="faint">{tr("Baujahr ")}</span>
              <input
                type="number"
                className="num"
                value={year}
                min={yearRange[0]}
                max={yearRange[1]}
                onChange={(e) => {
                  const v = Number(e.target.value);
                  if (e.target.value !== "" && Number.isFinite(v)) setYear(v);
                }}
                onBlur={() => setYear((v) => Math.min(yearRange[1], Math.max(yearRange[0], v)))}
              />
            </label>
          </div>
          <div className="hint">{profileNote}</div>

          <hr className="divider" />

          <Button
            variant={connected ? "danger" : "primary"}
            icon={connected ? "unplug" : "plug"}
            className={cx("connect-button", !connected && !connecting && "pulse")}
            disabled={connecting}
            onClick={() => startConnect()}
          >
            {connected ? tr("Trennen") : tr("Verbinden")}
          </Button>

          <div className="connect-state">
            {connecting ? <Spinner size={16} /> : null}
            <span className={cx("connect-state-label", stateTone)}>{stateText}</span>
          </div>
          {details ? <div className="hint wrap selectable connect-details">{details}</div> : null}

          {error !== null ? (
            <div className="connect-error fade-in" role="alert">
              <div className="connect-error-head">{tr("Das hat nicht geklappt")}</div>
              <div className="wrap selectable">{error}</div>
              {help ? null : (
                <div className="subtle">
                  {tr(
                    "Häufige Gründe: Zündung aus, Adapter nicht ganz eingesteckt, falscher " +
                      "Anschluss gewählt, oder ein anderes Programm hält den Anschluss " +
                      "offen. Bei Bluetooth hilft oft, den Adapter neu zu koppeln.",
                  )}
                </div>
              )}
            </div>
          ) : null}
          {error !== null && help ? <ConnectChecklist help={help} /> : null}
        </Card>
      </div>
    </Page>
  );
}
