// Polls every known control unit over the KKL cable in turn (ScanPage.cpp)
// and shows which ones the vehicle has and where something is stored in
// fault memory. Can also clear everywhere, with retries and read-back.
import { Fragment, useEffect, useRef, useState } from "react";
import { call } from "../api";
import type { EcuScanRow } from "../api";
import { confirm } from "../components/Modal";
import { Button, Page, PageHeader, Progress } from "../components/ui";
import { translator } from "../i18n";
import { useBackendEvent } from "../lib/hooks";
import { useSettings } from "../state/session";
import { PortChoice, useMessageBox, usePorts } from "./WorkshopShared";

const tr = translator("caruspex::ScanPage");

const pad = (n: number) => String(n).padStart(2, "0");

export default function ScanPage() {
  const settings = useSettings();
  // setClearAttempts(): at least one.
  const clearAttempts = settings.clearAttempts > 0 ? settings.clearAttempts : 1;
  const ports = usePorts(tr, (device, description) => (description ? `${device}  [${description}]` : device));
  const [messageBox, showMessage] = useMessageBox();

  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<{ value: number; total: number } | null>(null);
  const [status, setStatus] = useState(() => tr("Noch nicht durchsucht."));
  const [clearLog, setClearLog] = useState<string[] | null>(null);
  const [results, setResults] = useState<EcuScanRow[]>([]);
  const logRef = useRef<HTMLDivElement>(null);
  // Units answered in this run, for the "cancelled" line.
  const checked = useRef(0);

  useEffect(() => {
    const el = logRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [clearLog]);

  useBackendEvent("scan_started", ({ total }) => {
    setRunning(true);
    checked.current = 0;
    setResults([]);
    setProgress({ value: 0, total });
  });

  useBackendEvent("clear_progress", ({ line }) => {
    // The Worker reports the EOBD clear of the Faults page on the same
    // signal, and the Qt page shows those lines here too.
    setClearLog((log) => (log === null ? [line] : [...log, line]));
  });

  useBackendEvent("scan_progress", ({ ecuName, index, total }) => {
    setProgress((p) => ({ value: index - 1, total: p?.total ?? total }));
    setStatus(tr("Frage %1 an (%2 von %3) ...", ecuName, index, total));
  });

  useBackendEvent("scan_ecu_result", ({ row }) => {
    checked.current += 1;
    setResults((list) => [...list, row]);
    setProgress((p) => (p ? { ...p, value: p.value + 1 } : p));
  });

  useBackendEvent("scan_finished", ({ present, withFaults, cancelled }) => {
    setRunning(false);
    setProgress(null);
    if (cancelled) {
      setStatus(tr("Abgebrochen. %1 Steuergeräte geprüft.", checked.current));
      return;
    }
    setStatus(
      withFaults === 0
        ? tr("Fertig. %1 Steuergeräte gefunden, keines meldet einen Fehler.", present)
        : tr("Fertig. %1 Steuergeräte gefunden, %2 davon mit Einträgen im Speicher.", present, withFaults),
    );
  });

  const noPort = () => showMessage(tr("Kein Anschluss"), tr("Erst ein KKL-Kabel anstecken und auf Suchen drücken."));

  const startOrCancel = () => {
    if (running) {
      void call("cancel_scan");
      setStatus(tr("Abbruch angefordert, das laufende Steuergerät wird noch zu Ende gefragt ..."));
      return;
    }
    if (!ports.port) {
      noPort();
      return;
    }
    void call("scan_all_ecus", { device: ports.port, clearAttempts: 0 });
  };

  const clearEverywhere = async () => {
    if (!ports.port) {
      noPort();
      return;
    }
    const ok = await confirm({
      title: tr("Überall löschen"),
      icon: "alert",
      danger: true,
      text: (
        <>
          <div className="strong">{tr("In jedem gefundenen Steuergerät den Fehlerspeicher löschen?")}</div>
          <div className="subtle" style={{ marginTop: 8 }}>
            {tr(
              "Jedes Steuergerät wird angefunkt, ausgelesen und gelöscht. Klappt das Löschen nicht, wird es bis zu %1-mal wiederholt und jedes Mal nachgeprüft.\n\nWas das nicht kann: einen Fehler beseitigen, dessen Ursache noch besteht. Der steht sofort wieder drin. Löschen ersetzt keine Reparatur.\n\nZündung an, Motor aus. Der Durchlauf dauert einige Minuten.",
              clearAttempts,
            )}
          </div>
        </>
      ),
      confirm: tr("Ja, überall löschen"),
      cancel: tr("Abbrechen"),
    });
    if (!ok) return;
    setClearLog([]);
    void call("scan_all_ecus", { device: ports.port, clearAttempts });
  };

  const exportReport = async () => {
    if (results.length === 0) return;
    const now = new Date();
    const stamp = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}_${pad(now.getHours())}-${pad(now.getMinutes())}`;
    // tr("dd.MM.yyyy HH:mm") is a Qt date pattern, translated per language.
    const pattern = tr("dd.MM.yyyy HH:mm");
    const created = pattern
      .replace("dd", pad(now.getDate()))
      .replace("MM", pad(now.getMonth() + 1))
      .replace("yyyy", String(now.getFullYear()))
      .replace("HH", pad(now.getHours()))
      .replace("mm", pad(now.getMinutes()));
    const lines: string[] = [];
    lines.push(tr("Prüfbericht Caruspex"));
    lines.push(tr("Erstellt am %1", created));
    lines.push("=".repeat(60), "");
    let totalFaults = 0;
    for (const row of results) {
      lines.push(row.name);
      if (!row.present) {
        lines.push(`   ${tr("nicht verbaut oder meldet sich nicht")}`, "");
        continue;
      }
      if (row.identification) lines.push(`   ${tr("Kennung: %1", row.identification)}`);
      if (row.faults.length === 0) {
        lines.push(`   ${tr("kein Eintrag im Fehlerspeicher")}`);
      } else {
        for (const fault of row.faults) {
          lines.push(`   ${fault.code}  [${fault.kind}]  ${fault.text}`);
          ++totalFaults;
        }
      }
      lines.push("");
    }
    lines.push("-".repeat(60));
    lines.push(tr("Insgesamt %1 Einträge.", totalFaults));
    const filter = tr("Textdatei (*.txt)");
    try {
      await call("save_text_file", {
        title: tr("Bericht speichern"),
        suggestedName: tr("prüfbericht_%1.txt", stamp),
        filters: [{ name: filter.replace(/\s*\([^)]*\)\s*$/, ""), extensions: ["txt"] }],
        content: `${lines.join("\n")}\n`,
      });
    } catch {
      showMessage(tr("Fehler"), tr("Datei lässt sich nicht schreiben."));
    }
  };

  return (
    <Page className="scan-page">
      <PageHeader
        eyebrow={tr("WERKSTATT")}
        title={tr("Alle Steuergeräte durchsuchen")}
        subtitle={tr(
          "Klappert der Reihe nach jedes bekannte Steuergerät ab und zeigt, welche das Fahrzeug hat und wo etwas im Speicher steht. Jeder Versuch braucht wegen der 5-Baud-Weckadresse ein paar Sekunden, der ganze Durchlauf also rund eine Minute. Dafür wird ein KKL-Kabel gebraucht, ein ELM327 kommt an die Steuergeräte außerhalb des Motors nicht heran.",
        )}
      />

      <div className="ws-bar">
        <PortChoice caption={tr("Anschluss:")} options={ports.options} value={ports.port} onChange={ports.setPort} />
        <Button icon="refresh" onClick={() => void ports.refresh()}>
          {tr("Suchen")}
        </Button>
        <Button variant={running ? "default" : "primary"} icon={running ? "stop" : "search"} onClick={startOrCancel}>
          {running ? tr("Abbrechen") : tr("Durchsuchen")}
        </Button>
        <Button variant="danger" icon="trash" onClick={() => void clearEverywhere()}>
          {tr("Überall löschen")}
        </Button>
        <Button icon="download" disabled={running || results.length === 0} onClick={() => void exportReport()}>
          {tr("Bericht speichern")}
        </Button>
      </div>

      {progress ? <Progress value={progress.total > 0 ? (100 * Math.max(0, progress.value)) / progress.total : 0} /> : null}
      <div className="subtle wrap">{status}</div>

      {clearLog !== null ? (
        <div className="ws-log fade-in" ref={logRef} style={{ maxHeight: 130, minHeight: 60 }}>
          {clearLog.join("\n")}
        </div>
      ) : null}

      <div className="table-wrap">
        <table className="table scan-table">
          <thead>
            <tr>
              <th>{tr("Steuergerät")}</th>
              <th>{tr("Zustand")}</th>
              <th>{tr("Kennung")}</th>
            </tr>
          </thead>
          <tbody>
            {results.map((row) => {
              if (!row.present) {
                return (
                  <tr key={row.address} className="absent fade-in">
                    <td>{row.name}</td>
                    <td className="state">{tr("nicht verbaut")}</td>
                    <td className="ident">{row.identification}</td>
                  </tr>
                );
              }
              const count = row.faults.length;
              return (
                <Fragment key={row.address}>
                  <tr className="unit fade-in">
                    <td>{row.name}</td>
                    <td className={count === 0 ? "state ok" : "state bad"}>
                      {count === 0 ? tr("in Ordnung") : count === 1 ? tr("1 Fehler") : tr("%1 Fehler", count)}
                    </td>
                    <td className="ident ltr">{row.identification}</td>
                  </tr>
                  {row.faults.map((fault, i) => (
                    <tr key={`${row.address}-${fault.code}-${i}`} className="fault fade-in">
                      <td className="ltr">{fault.code}</td>
                      <td className="state">{fault.kind}</td>
                      <td>{fault.text}</td>
                    </tr>
                  ))}
                </Fragment>
              );
            })}
          </tbody>
        </table>
      </div>
      {messageBox}
    </Page>
  );
}
