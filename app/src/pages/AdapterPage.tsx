// What the adapter really is (AdapterPage.cpp): version, chip, speed, the
// commands it truly understands, and an honest guess whether it is a clone.
import { useEffect, useState } from "react";
import { call } from "../api";
import type { AdapterRow } from "../api";
import { FindingRow, StatTile } from "../components/Tiles";
import { Button, Card, Page, PageHeader, Spinner } from "../components/ui";
import { localeNumber, translator } from "../i18n";
import { useAppEvent, useBackendEvent } from "../lib/hooks";
import { navigate } from "../shell/nav";
import { useSession } from "../state/session";
import "./AdapterPage.css";

const tr = translator("caruspex::AdapterPage");

export default function AdapterPage() {
  const connected = useSession((s) => s.connected);
  const [testing, setTesting] = useState(false);
  const [verdict, setVerdict] = useState(() => tr("Erst verbinden, dann testen. Dauert etwa zehn Sekunden."));
  const [report, setReport] = useState<AdapterRow | null>(null);

  // setConnected(false) stops the spinner; the last report stays.
  useEffect(() => {
    if (!connected) setTesting(false);
  }, [connected]);

  useBackendEvent("adapter_report", ({ row }) => {
    setTesting(false);
    setVerdict(row.verdict);
    setReport(row);
  });

  // A refused test (not connected) must not leave the spinner turning.
  useBackendEvent("failed", () => setTesting(false));

  const start = () => {
    setTesting(true);
    setVerdict(tr("Teste ..."));
    void call("probe_adapter");
  };

  // "adapter-run" of MainWindow::showPageByName.
  useAppEvent<string>("dev:page", (name) => {
    if (name !== "adapter-run") return;
    navigate("adapter");
    if (connected) start();
    else void call("probe_adapter");
  });

  const speedTone = report
    ? report.roundTripMs < 60
      ? "var(--good)"
      : report.roundTripMs < 150
        ? "var(--warn)"
        : "var(--bad)"
    : undefined;

  return (
    <Page className="adapter-page">
      <PageHeader
        eyebrow={tr("Werkzeug")}
        title={tr("Adaptertest")}
        subtitle={tr(
          "Viele ELM327 sind Nachbauten, die eine höhere Version behaupten, als sie können. Der Test fragt nach, misst die Antwortzeit und sagt ehrlich, was er daraus schließt. Er schreibt nichts in den Adapter.",
        )}
      />

      <Card className="adapter-hero">
        <div className="row">
          <Button variant="primary" icon="cpu" className="adapter-start" disabled={!connected || testing} onClick={start}>
            {tr("Adapter testen")}
          </Button>
          {testing ? <Spinner size={20} /> : null}
        </div>
        <div className="adapter-verdict fade-in" key={verdict}>
          {verdict}
        </div>
      </Card>

      <div className="adapter-tiles">
        <StatTile
          icon="info"
          caption={tr("Meldet sich als")}
          value={report ? report.version || "?" : undefined}
          note={report?.description || undefined}
        />
        <StatTile
          icon="cpu"
          caption={tr("Verhält sich wie")}
          value={report ? (report.isStn ? "STN" : report.behavesLike || "?") : undefined}
          note={report?.isStn ? report.stnId : undefined}
        />
        <StatTile
          icon="timer"
          caption={tr("Antwortzeit")}
          value={report ? localeNumber(report.roundTripMs, 0) : undefined}
          unit={report ? "ms" : undefined}
          state={speedTone}
          note={
            report && report.roundTripWithHintMs > 0
              ? tr("mit Antwortzahl %1 ms", localeNumber(report.roundTripWithHintMs, 0))
              : undefined
          }
        />
        <StatTile
          icon="battery"
          caption={tr("Spannung an der Buchse")}
          value={report?.hasVoltage ? localeNumber(report.voltage, 1) : undefined}
          unit={report?.hasVoltage ? "V" : undefined}
        />
      </div>

      <div className="adapter-columns">
        <Card title={tr("Was der Test ergeben hat")}>
          <div className="adapter-facts">
            {report?.facts.map((f, i) => (
              <div className="fade-in" key={`f${i}`} style={{ animationDelay: `${i * 40}ms` }}>
                <FindingRow icon="info" tint="var(--info)" title={f} />
              </div>
            ))}
            {report?.speedTips.map((tip, i) => (
              <div className="fade-in" key={`s${i}`} style={{ animationDelay: `${(report.facts.length + i) * 40}ms` }}>
                <FindingRow icon="zap" tint="var(--accent)" title={tip} />
              </div>
            ))}
          </div>
        </Card>

        <Card title={tr("Befehle im Einzelnen")}>
          <div className="table-wrap adapter-checks">
            <table className="table">
              <thead>
                <tr>
                  <th>{tr("Befehl")}</th>
                  <th>{tr("Seit")}</th>
                  <th>{tr("Kann er")}</th>
                  <th className="grow">{tr("Wofür")}</th>
                </tr>
              </thead>
              <tbody>
                {report?.checks.map((c, i) => (
                  <tr key={c.command} className="fade-in" style={{ animationDelay: `${i * 30}ms` }}>
                    <td className="mono adapter-command ltr">{c.command}</td>
                    <td className="ltr">{c.version}</td>
                    <td className={c.supported ? "good" : "bad"}>{c.supported ? tr("ja") : tr("nein")}</td>
                    <td>{c.purpose}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      </div>
    </Page>
  );
}
