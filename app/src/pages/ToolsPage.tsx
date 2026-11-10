// "Prüfen und AU" (ToolsPage.cpp): readiness monitors for the emissions
// test, the VIN with the freeze frame data, and raw commands to the adapter.
import { useEffect, useRef, useState } from "react";
import { call, type LiveRow, type StatusRow } from "../api";
import { Button, Card, Page, PageHeader } from "../components/ui";
import { fixed, translator } from "../i18n";
import { useBackendEvent } from "../lib/hooks";
import { useSession } from "../state/session";
import "./ToolsPage.css";

const tr = translator("caruspex::ToolsPage");

interface Readiness {
  rows: { name: string; ready: boolean }[];
  summary: string;
}

interface VehicleInfo {
  vin: string;
  freezeCode: string;
  freezeRows: LiveRow[];
}

function readinessFrom(row: StatusRow): Readiness {
  const rows = row.monitorNames.map((name, i) => ({ name, ready: row.monitorReady[i] ?? false }));
  if (rows.length === 0) return { rows, summary: tr("Das Steuergerät meldet keine Bereitschaftstests.") };
  const total = rows.length;
  const ready = rows.filter((r) => r.ready).length;
  return {
    rows,
    summary:
      ready === total
        ? tr("Alle %1 Tests sind bereit. Für die Abgasuntersuchung passt das.", total)
        : tr(
            "%1 von %2 Tests sind bereit. Für die Abgasuntersuchung müssen alle " +
              "durchlaufen sein, dafür braucht das Auto ein paar vollständige " +
              "Fahrzyklen.",
            ready,
            total,
          ),
  };
}

// Puts the VIN into the translated sentence as its own left-to-right span,
// wherever the language places %1.
function withVin(template: string, vin: string) {
  const [before, after] = template.split("%1");
  return (
    <>
      {before}
      <span className="ltr mono">{vin}</span>
      {after}
    </>
  );
}

export default function ToolsPage() {
  const connected = useSession((s) => s.connected);
  const [readiness, setReadiness] = useState<Readiness | null>(null);
  const [info, setInfo] = useState<VehicleInfo | null>(null);
  const [command, setCommand] = useState("");
  const [replies, setReplies] = useState<string[]>([]);
  const repliesRef = useRef<HTMLTextAreaElement>(null);

  // setConnected(false) empties what belonged to the last car.
  useEffect(() => {
    if (!connected) {
      setReadiness(null);
      setInfo(null);
    }
  }, [connected]);

  useBackendEvent("status", ({ row }) => setReadiness(readinessFrom(row)));
  useBackendEvent("vehicle_info", (payload) => setInfo(payload));
  useBackendEvent("raw_reply", ({ reply }) => setReplies((r) => [...r, reply]));

  // Keep the newest reply in view, like appendPlainText does.
  useEffect(() => {
    const el = repliesRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [replies]);

  const sendRaw = () => {
    const text = command.trim();
    if (!text) return;
    setReplies((r) => [...r, `> ${text}`]);
    void call("send_raw", { command: text });
    setCommand("");
  };

  const vinText = !info
    ? tr("Fahrgestellnummer: noch nicht gelesen")
    : info.vin
      ? tr("Fahrgestellnummer: %1", info.vin)
      : tr("Fahrgestellnummer: liefert das Steuergerät nicht " + "(bei Fahrzeugen vor etwa 2003 normal)");
  const freezeTitle = !info ? "" : info.freezeCode ? tr("Aufgezeichnet beim Auftreten von %1:", info.freezeCode) : tr("Kein Umgebungsdatensatz gespeichert.");

  return (
    <Page className="tools-page">
      <PageHeader
        eyebrow={tr("DIAGNOSE")}
        title={tr("Prüfen und Werkzeuge")}
        actions={
          <Button variant="primary" icon="download" className="tools-load" disabled={!connected} onClick={() => void call("read_details")}>
            {tr("Daten vom Fahrzeug holen")}
          </Button>
        }
      />

      <Card title={tr("Bereitschaftstests (Abgasuntersuchung)")} icon="shield">
        <div className="subtle">{readiness ? readiness.summary : tr("Noch nichts gelesen.")}</div>
        <div className="table-wrap tools-table">
          <table className="table">
            <thead>
              <tr>
                <th>{tr("Test")}</th>
                <th>{tr("Zustand")}</th>
              </tr>
            </thead>
            <tbody>
              {(readiness?.rows ?? []).map((r, i) => (
                <tr key={`${r.name}-${i}`} className="fade-in">
                  <td>{r.name}</td>
                  <td className={r.ready ? "good" : "warn"}>
                    <span className="tools-state">
                      <span className="dot" style={{ background: r.ready ? "var(--good)" : "var(--warn)" }} />
                      {r.ready ? tr("bereit") : tr("noch nicht bereit")}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      <Card title={tr("Fahrzeug und Umgebungsdaten beim Fehler")} icon="car">
        <div className="selectable">
          {info?.vin ? withVin(tr("Fahrgestellnummer: %1"), info.vin) : vinText}
        </div>
        {freezeTitle ? <div className="subtle">{freezeTitle}</div> : null}
        <div className="table-wrap tools-table">
          <table className="table">
            <thead>
              <tr>
                <th>{tr("Messwert")}</th>
                <th className="num">{tr("Wert")}</th>
                <th>{tr("Einheit")}</th>
              </tr>
            </thead>
            <tbody>
              {(info?.freezeRows ?? []).map((v, i) => (
                <tr key={`${v.pid}-${i}`} className="fade-in">
                  <td>{v.name}</td>
                  <td className="num">{v.text ? v.text : <span className="ltr">{fixed(v.value, 2)}</span>}</td>
                  <td>{v.unit}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      <Card title={tr("Adapter direkt ansprechen")} icon="terminal">
        <div className="hint">{tr("Für Fortgeschrittene: ELM327-Befehle wie ATI, ATDP oder 0100 direkt senden.")}</div>
        <div className="tools-raw-line">
          <input
            className="input mono ltr grow"
            value={command}
            placeholder="ATI"
            onChange={(e) => setCommand(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") sendRaw();
            }}
          />
          <Button disabled={!connected} onClick={sendRaw}>
            {tr("Senden")}
          </Button>
        </div>
        <textarea ref={repliesRef} className="textarea tools-replies ltr" readOnly value={replies.join("\n")} />
      </Card>
    </Page>
  );
}
