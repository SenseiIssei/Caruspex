// The health check (HealthPage.cpp): one button, about half a minute of
// reading, then a score with every finding explained in plain words, plus
// what is still missing for the emissions test and how to drive to finish it.
import { useEffect, useState } from "react";
import { call, type HealthRow } from "../api";
import type { IconName } from "../components/Icon";
import { ScoreRing } from "../components/ScoreRing";
import { FindingRow } from "../components/Tiles";
import { Button, Card, Page, PageHeader, Progress, Select } from "../components/ui";
import { t, translator } from "../i18n";
import { useAppEvent, useBackendEvent } from "../lib/hooks";
import { navigate } from "../shell/nav";
import { useSession } from "../state/session";
import "./HealthPage.css";

const tr = translator("caruspex::HealthPage");
const MW = "caruspex::MainWindow";

// Icon and colour for an analysis level (0 good .. 3 critical). The start
// page uses the same three for its findings.
export function levelIcon(level: number): IconName {
  switch (level) {
    case 0: return "check-circle";
    case 1: return "info";
    case 2: return "alert";
    default: return "x-circle";
  }
}

export function levelColour(level: number): string {
  switch (level) {
    case 0: return "var(--good)";
    case 1: return "var(--info)";
    case 2: return "var(--warn)";
    default: return "var(--bad)";
  }
}

export function levelLabel(level: number): string {
  switch (level) {
    case 0: return tr("In Ordnung");
    case 1: return tr("Hinweis");
    case 2: return tr("Bald ansehen");
    default: return tr("Dringend");
  }
}

const caption = (grade: number) => (grade === 0 ? tr("Gut in Schuss") : grade === 1 ? tr("Braucht Pflege") : tr("Bitte ansehen"));

export default function HealthPage() {
  const connected = useSession((s) => s.connected);
  const [running, setRunningState] = useState(false);
  const [duration, setDuration] = useState(30);
  const [percent, setPercent] = useState(0);
  const [step, setStep] = useState("");
  const [headline, setHeadline] = useState(() => tr("Bereit für den Check"));
  const [result, setResult] = useState<HealthRow | null>(null);
  // Bumped with every result, so the new rows fade in again.
  const [generation, setGeneration] = useState(0);

  const setRunning = (value: boolean) => {
    setRunningState(value);
    if (value) {
      setPercent(0);
      setHeadline(tr("Prüfe ..."));
    }
  };

  // setConnected(false) ends a running check.
  useEffect(() => {
    if (!connected) setRunningState(false);
  }, [connected]);

  useBackendEvent("health_progress", ({ step: text, percent: value }) => {
    if (!running) setRunning(true);
    setPercent(value);
    setStep(text);
  });

  useBackendEvent("health_result", ({ row }) => {
    setRunningState(false);
    setStep(tr.n("%n Prüfungen ausgeführt.", row.checksRun));
    setHeadline(caption(row.grade));
    setResult(row);
    setGeneration((g) => g + 1);
  });

  // Developer aid for screenshots: open the page and start a short check.
  useAppEvent<string>("dev:page", (name) => {
    if (name !== "health-run") return;
    navigate("health");
    setRunning(true);
    setStep(t(MW, "Starte ..."));
    void call("run_health_check", { seconds: 12 });
  });

  const start = () => {
    setRunning(true);
    void call("run_health_check", { seconds: duration });
  };

  const durations = [
    { value: 15, label: tr("Kurz, 15 Sekunden") },
    { value: 30, label: tr("Normal, 30 Sekunden") },
    { value: 90, label: tr("Gründlich, 90 Sekunden") },
  ];

  const monitors = result?.monitors ?? [];
  const open = monitors.filter((m) => !m.complete).length;
  let monitorSummary = tr(
    "Die Bereitschaftstests zeigen, ob das Auto seine " + "Selbstprüfungen seit dem letzten Löschen abgeschlossen hat.",
  );
  if (result) {
    if (monitors.length === 0) monitorSummary = tr("Das Auto hat keine Bereitschaftstests gemeldet.");
    else if (open === 0) monitorSummary = tr("Alle Tests abgeschlossen. Von dieser Seite steht der AU " + "nichts im Weg.");
    else
      monitorSummary = result.diesel
        ? tr.n(
            "%n Test(s) noch offen. Beim Diesel sollten für die AU möglichst " + "alle abgeschlossen sein. Unten steht, wie man sie fährt.",
            open,
          )
        : tr.n(
            "%n Test(s) noch offen. Beim Benziner wird ein offener Test oft " +
              "akzeptiert, sicherer ist es, vorher die Prüfstelle zu fragen. " +
              "Unten steht, wie man sie fährt.",
            open,
          );
  }
  // Open ones first, with their drive pattern.
  const orderedMonitors = [...monitors.filter((m) => !m.complete), ...monitors.filter((m) => m.complete)];

  return (
    <Page className="health-page">
      <PageHeader
        eyebrow={tr("Diagnose")}
        title={tr("Gesundheitscheck")}
        subtitle={tr(
          "Liest Fehlerspeicher, Bereitschaftstests und die " +
            "Bauteilprüfung, beobachtet dann eine halbe Minute lang " +
            "Gemisch, Temperatur, Lambdasonden und Ladespannung. " +
            "Heraus kommt eine Punktzahl, und jeder Abzug wird erklärt.",
        )}
      />

      <Card className="health-hero">
        <div className="health-hero-row">
          <ScoreRing
            size={230}
            score={result ? result.score : -1}
            caption={result ? caption(result.grade) : tr("Noch offen")}
            subcaption={tr("Gesundheit")}
            busy={running}
          />
          <div className="health-hero-texts">
            <h2 className="headline health-headline">{headline}</h2>
            <p className="subtle health-summary">
              {result
                ? result.summary
                : tr(
                    "Am besten mit warmem Motor im Stand. Während der Messung einmal " +
                      "für ein paar Sekunden 2500 1/min halten, dann sieht Caruspex auch, " +
                      "wie das Gemisch unter Last geregelt wird.",
                  )}
            </p>
            <div className="health-controls">
              <Button
                variant="primary"
                icon="sparkles"
                className="health-start"
                disabled={running || !connected}
                title={connected ? undefined : tr("Erst verbinden.")}
                onClick={start}
              >
                {tr("Check starten")}
              </Button>
              <Select className="health-duration" value={duration} options={durations} onChange={setDuration} disabled={running} />
              {running ? (
                <Button variant="ghost" onClick={() => void call("cancel_scan")}>
                  {tr("Abbrechen")}
                </Button>
              ) : null}
            </div>
            {running ? <Progress value={percent} /> : null}
            {step ? <div className="hint">{step}</div> : null}
          </div>
        </div>
      </Card>

      <div className="health-columns">
        <Card className="health-findings" title={tr("Befunde")}>
          <div className="subtle">{tr("Das Wichtigste zuerst. Jeder Punkt sagt, was er " + "bedeutet und was jetzt zu tun ist.")}</div>
          <div className="health-list" key={generation}>
            {!result ? (
              <FindingRow icon="info" tint="var(--info)" title={tr("Noch kein Ergebnis")} text={tr("Nach dem Check steht hier jeder Befund.")} />
            ) : result.findings.length === 0 ? (
              <FindingRow
                icon="check-circle"
                tint="var(--good)"
                title={tr("Nichts gefunden")}
                text={tr("Alle Prüfungen, die dieses Auto zulässt, sind " + "unauffällig.")}
              />
            ) : (
              result.findings.map((f, i) => (
                <div key={i} className="fade-in" style={{ animationDelay: `${i * 50}ms` }}>
                  <FindingRow
                    icon={levelIcon(f.level)}
                    tint={levelColour(f.level)}
                    title={f.title}
                    text={<span className="wrap">{f.advice ? `${f.explanation}\n${tr("Was tun: %1", f.advice)}` : f.explanation}</span>}
                    meta={levelLabel(f.level)}
                  />
                </div>
              ))
            )}
          </div>
        </Card>

        <div className="health-side">
          <Card title={tr("Bereit für die AU?")}>
            <div className="subtle">{monitorSummary}</div>
            {orderedMonitors.length > 0 ? (
              <div className="health-list" key={generation}>
                {orderedMonitors.map((m, i) => {
                  if (m.complete) {
                    return <FindingRow key={i} icon="check-circle" tint="var(--good)" title={m.name} meta={tr("fertig")} />;
                  }
                  let text = `${m.explanation}\n${tr("So fahren: %1", m.drivePattern)}`;
                  if (m.typicalTime) text += ` (${m.typicalTime})`;
                  if (m.coldStart) text += `\n${tr("Braucht einen Kaltstart, also " + "erst nach einigen Stunden " + "Standzeit beginnen.")}`;
                  return (
                    <FindingRow key={i} icon="clock" tint="var(--warn)" title={m.name} text={<span className="wrap">{text}</span>} meta={tr("offen")} />
                  );
                })}
              </div>
            ) : null}
          </Card>

          {result && result.notChecked.length > 0 ? (
            <Card className="fade-in" title={tr("Nicht geprüft")}>
              <div className="subtle">{tr("Was dieses Auto nicht meldet oder was die Messung " + "nicht hergab.")}</div>
              <div className="health-skipped">
                {result.notChecked.map((s, i) => (
                  <div key={i} className="hint">
                    {`·  ${s}`}
                  </div>
                ))}
              </div>
            </Card>
          ) : null}
        </div>
      </div>
    </Page>
  );
}
