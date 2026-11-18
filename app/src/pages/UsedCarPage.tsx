// The used car check: for someone standing next to a car they might buy.
// One button reads the VIN, the odometer of every control unit with a
// documented identifier, when the fault memory was last cleared, readiness,
// faults, a short health check and the battery of electric cars; the result
// is a traffic light, every finding in plain words, the questions for the
// seller and a PDF for the purchase. On a truck over SAE J1939 the same page
// compares the VIN and identification of every unit, the odometers of
// dashboard, engine and tachograph, engine hours, idling and fuel.
import { useEffect, useRef, useState, type CSSProperties } from "react";
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
const tc = translator("caruspex::TruckCheck");

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

// What the truck variant reads, in the same shape.
const TRUCK_STEPS: { icon: IconName; title: () => string; text: () => string }[] = [
  {
    icon: "key",
    title: () => t(R, "Fahrgestellnummer"),
    text: () => tc("Aus jedem Steuergerät, das eine speichert: Motor, Kombiinstrument, Tachograph. Weichen sie ab, stammt ein Teil aus einem anderen Fahrzeug."),
  },
  {
    icon: "cpu",
    title: () => tc("Steuergeräte"),
    text: () => tc("Hersteller, Modell, Seriennummer und Software jedes Steuergeräts, zum Abgleich mit Rechnungen."),
  },
  {
    icon: "gauge",
    title: () => t(R, "Kilometerstand"),
    text: () => tc("Kombiinstrument, Motorsteuergerät und Tachograph zählen getrennt. Weichen sie ab, wurde vielleicht etwas verstellt oder getauscht."),
  },
  {
    icon: "clock",
    title: () => tc("Betriebsstunden und Verbrauch"),
    text: () => tc("Stunden gegen Kilometer, Leerlaufanteil und der Verbrauch über das ganze Motorleben."),
  },
  {
    icon: "alert",
    title: () => tc("Fehler und Lampen"),
    text: () => tc("Aktive und frühere Fehler jedes Steuergeräts, die vier Lampen und wann zuletzt gelöscht wurde."),
  },
  {
    icon: "heart",
    title: () => t(MW, "Gesundheitscheck"),
    text: () => tc("Zehn Sekunden Mithören: Kühlmittel, Öldruck, Bordspannung, AdBlue und Partikelfilter."),
  },
];

const num = (v: number) => localeNumber(v, 0);

export default function UsedCarPage() {
  const connected = useSession((s) => s.connected);
  const truck = useSession((s) => s.j1939);
  const result = useStore(usedCarStore, (s) => s.result);
  const [papers, setPapers] = useState("");
  const [running, setRunning] = useState(false);
  const [percent, setPercent] = useState(0);
  const [step, setStep] = useState("");
  const [generation, setGeneration] = useState(0);

  // A run asked for before the connection stood (?page=used-car-run in the
  // browser) starts once it does.
  const pendingRun = useRef(false);

  useEffect(() => {
    if (!connected) setRunning(false);
    if (connected && pendingRun.current) {
      pendingRun.current = false;
      navigate("used-car");
      start(DEMO_VIN);
    }
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

  useBackendEvent("failed", () => {
    setRunning(false);
    setStep("");
  });

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
    // The app asks twice (start page, then --page after connecting): one run.
    if (running || pendingRun.current || usedCarStore.get().result) return;
    setPapers(DEMO_VIN);
    if (connected) start(DEMO_VIN);
    else pendingRun.current = true;
  });

  const headline = running ? t(HP, "Prüfe ...") : result ? result.headline : tr("Bereit für die Besichtigung");
  const summary = result
    ? result.summary
    : truck
      ? tc("Zündung an, am besten bei laufendem Motor im Stand. Die Prüfung dauert etwa eine halbe Minute und verändert nichts am Fahrzeug.")
      : tr(
          "Zündung an, am besten bei laufendem Motor im Stand. Die Prüfung dauert etwa eine halbe Minute und verändert nichts am Auto.",
        );

  return (
    <Page className="used-car-page">
      <PageHeader
        eyebrow={truck ? tc("Lkw") : t(MW, "Diagnose")}
        title={truck ? tc("Gebrauchten Lkw prüfen") : tr("Gebrauchtwagen prüfen")}
        subtitle={
          truck
            ? tc(
                "Für die Besichtigung vor dem Kauf eines Lkw oder Busses: Passen Fahrgestellnummern und Kilometerstände aller Steuergeräte zusammen, passen Betriebsstunden, Leerlauf und Verbrauch dazu? Heraus kommt eine Ampel, die Fragen an den Verkäufer und ein PDF für den Kauf.",
              )
            : tr(
                "Für die Besichtigung vor dem Kauf: Passt die Fahrgestellnummer, stimmt der Kilometerstand, wurde kurz vorher gelöscht? Heraus kommt eine Ampel, die Fragen an den Verkäufer und ein PDF für den Kauf.",
              )
        }
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

      {result ? (
        result.truck ? (
          <TruckFacts row={result} key={`facts${generation}`} />
        ) : (
          <Facts row={result} key={`facts${generation}`} />
        )
      ) : null}

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
              {(truck ? TRUCK_STEPS : STEPS).map((s, i) => (
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
              {truck
                ? tc(
                    "Die Prüfung liest nur. Sie löscht keine Fehler, setzt keine Zähler zurück und ändert nichts am Fahrzeug, der Verkäufer kann also beruhigt zustimmen.",
                  )
                : tr(
                    "Die Prüfung liest nur. Sie löscht keine Fehler und ändert nichts am Auto, der Verkäufer kann also beruhigt zustimmen.",
                  )}
            </Note>
            {truck ? (
              <Note icon="alert" title={tc("Was die Prüfung nicht sieht")}>
                {tc(
                  "Ein Kombiinstrument lässt sich tauschen oder umprogrammieren und ein Fehlerspeicher löschen. Den Speicher des Tachographen, herstellereigene Zähler und den Zustand von Rahmen, Bremsen und Reifen sieht kein Steuergerät.",
                )}
              </Note>
            ) : null}
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

// The key numbers of a truck: odometer, hours with the idle share, fuel,
// faults and the short health check.
function TruckFacts({ row }: { row: UsedCarRow }) {
  const truck = row.truck;
  if (!truck) return null;
  const km = row.mileage.length > 0 ? Math.max(...row.mileage.map((m) => m.km)) : -1;
  const has = (id: string) => row.items.some((i) => i.id === id);
  const tone = (good: boolean) => (good ? "var(--good)" : "var(--warn)");
  return (
    <div className="uc-facts fade-in">
      <StatTile
        icon="gauge"
        caption={t(R, "Kilometerstand")}
        value={km >= 0 ? num(km) : undefined}
        unit="km"
        state={has("mileage_mismatch") ? "var(--bad)" : km >= 0 ? "var(--good)" : ""}
        note={row.mileage.length > 1 ? tr("aus %1 Steuergeräten", row.mileage.length) : undefined}
      />
      <StatTile
        icon="clock"
        caption={tc("Motorbetriebsstunden")}
        value={truck.engineHours >= 0 ? num(truck.engineHours) : undefined}
        unit="h"
        state={truck.engineHours >= 0 ? tone(!has("hours_too_few") && !has("km_too_few")) : ""}
        note={truck.idleShare >= 0 ? tc("%1 % im Leerlauf", num(truck.idleShare)) : undefined}
      />
      <StatTile
        icon="fuel"
        caption={tc("Verbrauch")}
        value={truck.fuelPer100km >= 0 ? localeNumber(truck.fuelPer100km, 1) : undefined}
        unit="l/100 km"
        state={truck.fuelPer100km >= 0 ? tone(!row.items.some((i) => i.id === "fuel_average" && i.level >= 2)) : ""}
        note={truck.averageSpeed >= 0 ? tc("%1 km je Betriebsstunde", localeNumber(truck.averageSpeed, 1)) : undefined}
      />
      <StatTile
        icon="alert"
        caption={tr("Fehler")}
        value={truck.activeFaults >= 0 ? String(truck.activeFaults) : undefined}
        state={truck.activeFaults >= 0 ? tone(truck.activeFaults === 0) : ""}
        note={truck.previousFaults > 0 ? tc("dazu %1 früher aktiv", truck.previousFaults) : undefined}
      />
      <StatTile
        icon="heart"
        caption={t(MW, "Gesundheitscheck")}
        value={row.healthScore >= 0 ? String(row.healthScore) : undefined}
        unit={row.healthScore >= 0 ? tr("von 100") : undefined}
        state={row.healthScore >= 0 ? (row.healthScore >= 85 ? "var(--good)" : row.healthScore >= 60 ? "var(--warn)" : "var(--bad)") : ""}
      />
    </div>
  );
}
