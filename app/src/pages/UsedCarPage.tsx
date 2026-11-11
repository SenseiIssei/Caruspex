// The used car check: for someone standing next to a car they might buy.
// One button reads the VIN, the odometer of every control unit with a
// documented identifier, when the fault memory was last cleared, readiness,
// faults, a short health check and the battery of electric cars; the result
// is a traffic light, every finding in plain words, the questions for the
// seller and a PDF for the purchase.
import { useEffect, useState, type CSSProperties } from "react";
import { call, type UsedCarRow } from "../api";
import type { IconName } from "../components/Icon";
import { Icon } from "../components/Icon";
import { FindingRow, StatTile } from "../components/Tiles";
import { Button, Card, Note, Page, PageHeader, Progress, TextInput } from "../components/ui";
import { localeNumber, t, translator } from "../i18n";
import { useAppEvent, useBackendEvent } from "../lib/hooks";
import { useStore } from "../lib/store";
import { DEMO_VIN } from "../lib/vehicleIdentity";
import { USED_CAR_SECTIONS, usedCarLevelLabel, usedCarSectionTitle } from "../report/report";
import { navigate, openUsedCarReport } from "../shell/nav";
import { useSession } from "../state/session";
import { usedCarStore } from "../state/usedCar";
import { levelColour, levelIcon } from "./HealthPage";
import "./UsedCarPage.css";

const tr = translator("caruspex::UsedCarPage");
const MW = "caruspex::MainWindow";
const HP = "caruspex::HealthPage";
const R = "caruspex::Report";

// The three lamps, the lit one glowing; none lit before the first result.
function TrafficLight({ light, busy }: { light: number | null; busy: boolean }) {
  const lamps = [
    { tone: "var(--bad)", on: light === 2 },
    { tone: "var(--warn)", on: light === 1 },
    { tone: "var(--good)", on: light === 0 },
  ];
  return (
    <div className={`uc-light${busy ? " busy" : ""}`} aria-hidden>
      {lamps.map((l, i) => (
        <span key={i} className={`uc-lamp${l.on ? " on" : ""}`} style={{ "--lamp": l.tone } as CSSProperties} />
      ))}
    </div>
  );
}

const STEPS: { icon: IconName; title: () => string; text: () => string }[] = [
  {
    icon: "key",
    title: () => t(R, "Fahrgestellnummer"),
    text: () => tr("Aus dem Motorsteuergerät, verglichen mit dem Fahrzeugkatalog und mit den Papieren."),
  },
  {
    icon: "gauge",
    title: () => t(R, "Kilometerstand"),
    text: () => tr("Aus jedem Steuergerät, für das eine Kennung belegt ist. Weichen sie ab, wurde vielleicht der Tacho verstellt."),
  },
  {
    icon: "clock",
    title: () => tr("Zuletzt gelöscht"),
    text: () => tr("Strecke, Warmläufe und Betriebszeit seit dem letzten Löschen. Frisch gelöscht kann heißen: Fehler versteckt."),
  },
  {
    icon: "alert",
    title: () => tr("Bereitschaft und Fehler"),
    text: () => tr("Offene Selbsttests sowie gespeicherte, vorläufige und dauerhafte Fehler."),
  },
  {
    icon: "heart",
    title: () => t(MW, "Gesundheitscheck"),
    text: () => tr("Zehn Sekunden Messwerte im Stand: Gemisch, Temperatur, Ladespannung."),
  },
  {
    icon: "bolt-circle",
    title: () => tr("Akku"),
    text: () => tr("Bei E-Autos und Hybriden, wenn für das Modell Akkuwerte belegt sind."),
  },
];

const num = (v: number) => localeNumber(v, 0);

export default function UsedCarPage() {
  const connected = useSession((s) => s.connected);
  const result = useStore(usedCarStore, (s) => s.result);
  const [papers, setPapers] = useState("");
  const [running, setRunning] = useState(false);
  const [percent, setPercent] = useState(0);
  const [step, setStep] = useState("");
  const [generation, setGeneration] = useState(0);

  useEffect(() => {
    if (!connected) setRunning(false);
  }, [connected]);

  useBackendEvent("used_car_progress", ({ step: text, percent: value }) => {
    setRunning(value < 100);
    setPercent(value);
    setStep(text);
  });

  useBackendEvent("used_car_result", ({ row }) => {
    setRunning(false);
    setStep("");
    usedCarStore.set({ result: row, when: Date.now() });
    setGeneration((g) => g + 1);
  });

  useBackendEvent("failed", () => setRunning(false));

  const start = (vin = papers) => {
    setRunning(true);
    setPercent(0);
    setStep(t(MW, "Starte ..."));
    void call("run_used_car_check", { papersVin: vin });
  };

  // Developer aid for screenshots and --report: open the page and run the
  // check with the VIN of the papers filled in.
  useAppEvent<string>("dev:page", (name) => {
    if (name !== "used-car-run") return;
    navigate("used-car");
    setPapers(DEMO_VIN);
    start(DEMO_VIN);
  });

  const headline = running ? t(HP, "Prüfe ...") : result ? result.headline : tr("Bereit für die Besichtigung");
  const summary = result
    ? result.summary
    : tr(
        "Zündung an, am besten bei laufendem Motor im Stand. Die Prüfung dauert etwa eine halbe Minute und verändert nichts am Auto.",
      );

  return (
    <Page className="used-car-page">
      <PageHeader
        eyebrow={t(MW, "Diagnose")}
        title={tr("Gebrauchtwagen prüfen")}
        subtitle={tr(
          "Für die Besichtigung vor dem Kauf: Passt die Fahrgestellnummer, stimmt der Kilometerstand, wurde kurz vorher gelöscht? Heraus kommt eine Ampel, die Fragen an den Verkäufer und ein PDF für den Kauf.",
        )}
      />

      <Card className="uc-hero">
        <div className="uc-hero-row">
          <TrafficLight light={result && !running ? result.light : null} busy={running} />
          <div className="uc-hero-texts">
            <h2 className="headline">{headline}</h2>
            <p className="subtle uc-summary">{summary}</p>
            <label className="uc-label" htmlFor="uc-papers">
              {tr("Fahrgestellnummer aus den Papieren (Zulassungsbescheinigung Teil I, Feld E), freiwillig")}
            </label>
            <div className="uc-controls">
              <TextInput
                id="uc-papers"
                className="uc-vin mono"
                value={papers}
                maxLength={24}
                spellCheck={false}
                placeholder="WVWZZZ1JZ3W000001"
                onChange={(v) => setPapers(v.toUpperCase())}
                disabled={running}
              />
              <Button
                variant="primary"
                icon="tag"
                className="uc-start"
                disabled={running || !connected}
                title={connected ? undefined : t(HP, "Erst verbinden.")}
                onClick={() => start()}
              >
                {tr("Prüfung starten")}
              </Button>
              {running ? (
                <Button variant="ghost" onClick={() => void call("cancel_scan")}>
                  {t(HP, "Abbrechen")}
                </Button>
              ) : null}
              {result && !running ? (
                <Button icon="file" onClick={openUsedCarReport}>
                  {tr("PDF für den Kauf")}
                </Button>
              ) : null}
            </div>
            {running ? <Progress value={percent} /> : null}
            {step ? <div className="hint">{step}</div> : null}
          </div>
        </div>
      </Card>

      {result ? <Facts row={result} key={`facts${generation}`} /> : null}

      {result ? (
        <div className="uc-columns" key={generation}>
          <div className="uc-findings">
            {USED_CAR_SECTIONS.map((section) => {
              const items = result.items.filter((i) => i.section === section);
              if (items.length === 0) return null;
              return (
                <Card key={section} className="fade-in" title={usedCarSectionTitle(section)}>
                  <div className="uc-list">
                    {items.map((item) => (
                      <FindingRow
                        key={item.id}
                        icon={levelIcon(item.level)}
                        tint={levelColour(item.level)}
                        title={item.title}
                        text={<span className="wrap">{item.explanation}</span>}
                        meta={usedCarLevelLabel(item.level)}
                      />
                    ))}
                  </div>
                </Card>
              );
            })}
          </div>
          <div className="uc-side">
            <Card className="fade-in uc-questions" icon="list" title={tr("Das sollten Sie den Verkäufer fragen")}>
              {result.questions.length === 0 ? (
                <div className="subtle">{tr("Keine offenen Fragen aus der Prüfung.")}</div>
              ) : (
                <ol className="uc-question-list">
                  {result.questions.map((q, i) => (
                    <li key={i}>{q}</li>
                  ))}
                </ol>
              )}
            </Card>
            {result.mileage.length > 0 ? (
              <Card className="fade-in" icon="gauge" title={tr("Kilometerstand je Steuergerät")}>
                <table className="table uc-mileage">
                  <tbody>
                    {result.mileage.map((m, i) => (
                      <tr key={i}>
                        <td>{m.source}</td>
                        <td className="num">{`${num(m.km)} km`}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </Card>
            ) : null}
            {result.notChecked.length > 0 ? (
              <Card className="fade-in" title={t(HP, "Nicht geprüft")}>
                <div className="uc-skipped">
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
      ) : (
        <div className="uc-columns">
          <Card className="uc-findings" title={tr("Was geprüft wird")}>
            <div className="uc-steps">
              {STEPS.map((s, i) => (
                <div key={i} className="uc-step">
                  <div className="uc-step-icon">
                    <Icon name={s.icon} size={18} />
                  </div>
                  <div>
                    <div className="uc-step-title">{s.title()}</div>
                    <div className="subtle">{s.text()}</div>
                  </div>
                </div>
              ))}
            </div>
          </Card>
          <div className="uc-side">
            <Note icon="lightbulb" title={tr("Tipp")}>
              {tr(
                "Vor der Probefahrt prüfen und danach noch einmal. Fehler, die erst unterwegs auftreten, zeigen sich dann im zweiten Durchgang.",
              )}
            </Note>
            <Note icon="info" title={tr("Nur lesend")}>
              {tr(
                "Die Prüfung liest nur. Sie löscht keine Fehler und ändert nichts am Auto, der Verkäufer kann also beruhigt zustimmen.",
              )}
            </Note>
          </div>
        </div>
      )}
    </Page>
  );
}

// The key numbers as tiles above the findings.
function Facts({ row }: { row: UsedCarRow }) {
  const km = row.mileage.length > 0 ? Math.max(...row.mileage.map((m) => m.km)) : -1;
  const sinceClear = row.distanceSinceClearKm;
  const faults = row.storedFaults < 0 ? -1 : row.storedFaults + row.permanentFaults;
  const tone = (good: boolean) => (good ? "var(--good)" : "var(--warn)");
  return (
    <div className="uc-facts fade-in">
      <StatTile
        icon="gauge"
        caption={t(R, "Kilometerstand")}
        value={km >= 0 ? num(km) : undefined}
        unit="km"
        state={row.items.some((i) => i.id === "mileage_mismatch") ? "var(--bad)" : km >= 0 ? "var(--good)" : ""}
        note={row.mileage.length > 1 ? tr("aus %1 Steuergeräten", row.mileage.length) : undefined}
      />
      <StatTile
        icon="clock"
        caption={tr("Seit dem letzten Löschen")}
        value={sinceClear >= 0 ? num(sinceClear) : undefined}
        unit="km"
        state={sinceClear >= 0 ? tone(!row.items.some((i) => i.id === "cleared_recently")) : ""}
        note={row.warmUps >= 0 ? tr("%1 Warmläufe", num(row.warmUps)) : undefined}
      />
      <StatTile
        icon="check-circle"
        caption={t(R, "Bereitschaftstests")}
        value={row.readinessTotal > 0 ? `${row.readinessComplete}/${row.readinessTotal}` : undefined}
        state={row.readinessTotal > 0 ? tone(row.readinessComplete === row.readinessTotal) : ""}
        note={row.readinessTotal > 0 ? t(R, "abgeschlossen") : undefined}
      />
      <StatTile
        icon="alert"
        caption={tr("Fehler")}
        value={faults >= 0 ? String(faults) : undefined}
        state={faults >= 0 ? tone(faults === 0) : ""}
        note={row.pendingFaults > 0 ? tr("dazu %1 vorläufig", row.pendingFaults) : undefined}
      />
      {row.electric ? (
        <StatTile
          icon="bolt-circle"
          caption={tr("Akku")}
          value={row.batterySoh >= 0 ? num(row.batterySoh) : undefined}
          unit="%"
          state={row.batterySoh >= 0 ? (row.batterySoh >= 80 ? "var(--good)" : row.batterySoh >= 70 ? "var(--warn)" : "var(--bad)") : ""}
          note={row.batteryPack || undefined}
        />
      ) : (
        <StatTile
          icon="heart"
          caption={t(MW, "Gesundheitscheck")}
          value={row.healthScore >= 0 ? String(row.healthScore) : undefined}
          unit={row.healthScore >= 0 ? tr("von 100") : undefined}
          state={row.healthScore >= 0 ? (row.healthScore >= 85 ? "var(--good)" : row.healthScore >= 60 ? "var(--warn)" : "var(--bad)") : ""}
        />
      )}
    </div>
  );
}
