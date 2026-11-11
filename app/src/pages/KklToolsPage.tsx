// The two manufacturer specific services no ELM327 can do (KklToolsPage.cpp):
// reading adaptation channels and the output test, over the KKL cable.
import { useEffect, useRef, useState } from "react";
import { call } from "../api";
import type { KklEcuInfo, LiveRow } from "../api";
import { Button, Card, NumberInput, Page, PageHeader, Select } from "../components/ui";
import { fixed, translator } from "../i18n";
import { useBackendEvent } from "../lib/hooks";
import { KklMessage, KklState, PortChoice, useMessageBox, usePorts } from "./WorkshopShared";

const tr = translator("caruspex::KklToolsPage");

const pad = (n: number) => String(n).padStart(2, "0");

// updateAvailability(): without the cable the page says why nothing works.
const unavailable = () =>
  tr("Beides sind herstellereigene Dienste. Über einen ELM327-Adapter sind sie nicht erreichbar, dafür wird das KKL-Kabel gebraucht.");

function ValueTable({ rows }: { rows: LiveRow[] }) {
  return (
    <div className="table-wrap" style={{ minHeight: 160 }}>
      <table className="table ws-values">
        <thead>
          <tr>
            <th>{tr("Feld")}</th>
            <th className="value">{tr("Wert")}</th>
            <th>{tr("Einheit")}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((v, i) => (
            <tr key={`${i}-${v.name}`}>
              <td>{v.name}</td>
              <td className="value ltr">{fixed(v.value, 2)}</td>
              <td className="ltr">{v.unit}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export default function KklToolsPage() {
  const ports = usePorts(tr, (device, description) => (description ? `${device}  [${description}]` : device));
  const [messageBox, showMessage] = useMessageBox();
  const [ecus, setEcus] = useState<KklEcuInfo[]>([]);
  const [ecu, setEcu] = useState(0x01);
  const [connected, setConnected] = useState(false);
  const [identification, setIdentification] = useState("");
  const [channel, setChannel] = useState(1);
  const [channelValue, setChannelValue] = useState(() => tr("Wert: --"));
  const [shown, setShown] = useState<LiveRow[]>([]);
  const [actuator, setActuator] = useState("--");
  const [actuatorLog, setActuatorLog] = useState<string[]>([]);
  const [message, setMessage] = useState(() => ({ text: unavailable(), error: false }));
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void call("kkl_ecus").then(
      (list) => {
        setEcus(list);
        if (list.length > 0) setEcu(list[0]!.address);
      },
      () => setEcus([]),
    );
  }, []);

  useEffect(() => {
    const el = logRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [actuatorLog]);

  useBackendEvent("kkl_connected", ({ connected: on, identification: id }) => {
    setConnected(on);
    setIdentification(id);
    if (!on) setMessage({ text: unavailable(), error: false });
  });

  useBackendEvent("kkl_message", ({ message: text, isError }) => setMessage({ text, error: isError }));

  useBackendEvent("adaptation_result", ({ channel: ch, value, shown: values }) => {
    setChannelValue(tr("Kanal %1: Wert %2", ch, value));
    setShown(values);
  });

  useBackendEvent("actuator_result", ({ actuator: name }) => {
    setActuator(name);
    const now = new Date();
    const stamp = `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`;
    setActuatorLog((log) => [...log, `${stamp}  ${name}`]);
  });

  const connectOrClose = () => {
    if (connected) {
      void call("close_kkl");
      return;
    }
    if (!ports.port) {
      showMessage(tr("Kein Anschluss"), tr("Erst ein KKL-Kabel anstecken und auf Suchen drücken."));
      return;
    }
    void call("open_kkl", { device: ports.port, ecuAddress: ecu });
  };

  return (
    <Page className="kkl-page">
      <PageHeader eyebrow={tr("WERKSTATT")} title={tr("Anpassung und Stellglieder")} />

      <Card title={tr("Steuergerät verbinden (KKL-Kabel)")} icon="cable">
        <div className="ws-bar">
          <label className="ws-field ws-ecu">
            <span className="ws-caption">{tr("Steuergerät:")}</span>
            <Select
              className="grow"
              value={ecu}
              options={ecus.map((e) => ({ value: e.address, label: e.name }))}
              onChange={setEcu}
              disabled={connected}
            />
          </label>
          <PortChoice caption={tr("Anschluss:")} options={ports.options} value={ports.port} onChange={ports.setPort} />
          <Button icon="refresh" onClick={() => void ports.refresh()}>
            {tr("Suchen")}
          </Button>
          <Button variant={connected ? "default" : "primary"} icon={connected ? "unplug" : "plug"} onClick={connectOrClose}>
            {connected ? tr("Trennen") : tr("Verbinden")}
          </Button>
        </div>
        <KklState connected={connected} text={connected ? tr("verbunden") : tr("nicht verbunden")} />
        <div className="ws-ident">{identification}</div>
      </Card>

      <Card title={tr("Anpassungskanal lesen")} icon="sliders">
        <div className="ws-explain">
          {tr(
            "Anpassungskanäle sind die Stellschrauben des Steuergeräts, etwa der Leerlauf oder die Startmenge. Caruspex liest sie hier aus. Verstellt wird bewusst nichts: ein falscher Wert im falschen Kanal legt den Motor lahm, und manche Kanäle brauchen zusätzlich einen Zugriffscode.",
          )}
        </div>
        <div className="kkl-row">
          <label className="ws-field">
            <span className="ws-caption">{tr("Kanal:")}</span>
            <NumberInput value={channel} onChange={(v) => setChannel(Math.round(v))} min={0} max={255} />
          </label>
          <Button disabled={!connected} onClick={() => void call("read_adaptation_channel", { channel })}>
            {tr("Kanal lesen")}
          </Button>
          <span className="ws-big num fade-in" key={channelValue}>
            {channelValue}
          </span>
        </div>
        <ValueTable rows={shown} />
      </Card>

      <Card title={tr("Stellglieddiagnose")} icon="zap">
        <div className="ws-explain">
          {tr(
            "Das Steuergerät klappert seine Aktoren der Reihe nach durch und nennt jeweils das laufende Bauteil. Man hört und sieht dann, ob es wirklich anspricht. Jeder Druck auf den Knopf schaltet zum nächsten Stellglied weiter.\n\nZündung an, Motor aus. Beim laufenden Motor verweigern die meisten Steuergeräte den Dienst.",
          )}
        </div>
        <div className="kkl-row">
          <Button icon="play" disabled={!connected} onClick={() => void call("run_actuator_test")}>
            {tr("Nächstes Stellglied")}
          </Button>
          <span className="ws-big accent grow fade-in" key={actuator}>
            {actuator}
          </span>
        </div>
        <div className="ws-log" ref={logRef} style={{ minHeight: 130, maxHeight: 220 }}>
          {actuatorLog.join("\n")}
        </div>
      </Card>

      <KklMessage text={message.text} error={message.error} />
      {messageBox}
    </Page>
  );
}
