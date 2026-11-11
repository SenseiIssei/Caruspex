// Pieces the K-line pages share (ScanPage, ThrottlePage, KklToolsPage): the
// port choice that each Qt page filled from enumeratePorts(), and the
// one-button message box (QMessageBox::information / warning).
import { useCallback, useEffect, useState, type ReactNode } from "react";
import { call } from "../api";
import { Modal } from "../components/Modal";
import { Button, Select, type Option } from "../components/ui";
import { t, translator, type Translator } from "../i18n";
import { useSession } from "../state/session";
import "./Workshop.css";

const ways = translator("caruspex::KklWays");

// The port choice for VAG TP 2.0 on the CAN interface of the connection
// (worker.rs TP20_DEVICE), and the prefix for the K-line of a J2534 device
// (J2534_DEVICE_PREFIX).
export const TP20_DEVICE = "tp20";
export const J2534_DEVICE_PREFIX = "j2534:";

// Why the list is empty and what helps.
export const noWay = () =>
  ways(
    "Erst ein KKL-Kabel anstecken und auf Suchen drücken. VW, Audi, Seat und Skoda von etwa 2004 bis 2012 gehen auch ohne: über eine CAN-Schnittstelle (J2534, SLCAN, SocketCAN) mit dem Auto verbinden, dann steht TP 2.0 in der Liste.",
  );

// The serial ports as choices, with the ways that need no KKL cable in front:
// TP 2.0 while the car is connected through a raw CAN interface (VW, Audi,
// Seat, Skoda of about 2004 to 2012), then the K-line of every usable J2534
// device. With nothing at all the list holds one entry with an empty device,
// which the pages answer with the "Kein Anschluss" message.
// label: how a port is shown; the Qt pages differ a little in this.
export function usePorts(tr: Translator, label: (device: string, description: string) => string) {
  const rawCan = useSession((s) => s.connected && s.rawCan);
  const [options, setOptions] = useState<Option<string>[]>([{ value: "", label: tr("kein Anschluss gefunden") }]);
  const [port, setPort] = useState("");
  const refresh = useCallback(async () => {
    let list: Option<string>[] = [];
    try {
      const ports = await call("list_serial_ports");
      list = ports.map((p) => ({ value: p.device, label: label(p.device, p.description) }));
    } catch {
      // No ports is the same as no answer.
    }
    try {
      const devices = await call("list_j2534_devices");
      for (const d of devices.filter((x) => x.usable)) {
        list.push({ value: J2534_DEVICE_PREFIX + d.library, label: ways("K-Line über J2534: %1", d.name) });
      }
    } catch {
      // No PassThru drivers, or not on Windows.
    }
    if (rawCan) list.unshift({ value: TP20_DEVICE, label: ways("TP 2.0 über die CAN-Schnittstelle (VAG etwa 2004 bis 2012)") });
    if (list.length === 0) list = [{ value: "", label: tr("kein Anschluss gefunden") }];
    setOptions(list);
    // A rebuilt QComboBox starts at its first entry.
    setPort(list[0]!.value);
    // The label function and the translator are fixed per page; the list is
    // built again when the connection changes.
  }, [rawCan]);
  useEffect(() => {
    void refresh();
  }, [refresh]);
  return { options, port, setPort, refresh };
}

// "Anschluss:" and the port choice, as the Qt rows have them.
export function PortChoice({
  caption,
  options,
  value,
  onChange,
  disabled,
}: {
  caption: string;
  options: readonly Option<string>[];
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
}) {
  return (
    <label className="ws-field ws-port">
      <span className="ws-caption">{caption}</span>
      <Select className="grow" value={value} options={options} onChange={onChange} disabled={disabled} />
    </label>
  );
}

// A message with a single OK button. show() opens it; the element goes
// somewhere into the page.
export function useMessageBox(): [ReactNode, (title: string, text: ReactNode) => void] {
  const [box, setBox] = useState<{ title: string; text: ReactNode } | null>(null);
  const close = useCallback(() => setBox(null), []);
  const element = box ? (
    <Modal
      small
      title={box.title}
      icon="info"
      onClose={close}
      footer={
        <Button variant="primary" onClick={close} autoFocus>
          {t("QPlatformTheme", "OK")}
        </Button>
      }
    >
      <div className="wrap">{box.text}</div>
    </Modal>
  ) : null;
  const show = useCallback((title: string, text: ReactNode) => setBox({ title, text }), []);
  return [element, show];
}

// Where a K-line page stands: muted when idle, green when connected.
export function KklState({ connected, text }: { connected: boolean; text: string }) {
  return (
    <div className={connected ? "ws-state on" : "ws-state"}>
      <span className="ws-state-dot" />
      {text}
    </div>
  );
}

// The last message of the K-line Worker under a card, red for errors.
export function KklMessage({ text, error }: { text: string; error: boolean }) {
  if (!text) return null;
  return (
    <div className={error ? "ws-message error fade-in" : "ws-message fade-in"} key={text}>
      {text}
    </div>
  );
}
