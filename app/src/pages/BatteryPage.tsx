// Battery and alternator (BatteryPage.cpp): four short guided measurements.
// The voltage comes from the adapter itself (pin 16 of the OBD socket), so
// it works on every car, old or new. analysis::evaluateBattery in the
// backend turns the readings into verdicts with plain explanations.
import { useEffect, useRef, useState } from "react";
import { call } from "../api";
import type { BatteryInput, BatteryReport, BatteryStepInfo, BatteryType, Level, VoltageSample } from "../api";
import { Sparkline } from "../components/charts/Mini";
import { Gauge } from "../components/Gauge";
import type { IconName } from "../components/Icon";
import { FindingRow } from "../components/Tiles";
import { Button, Card, Check, Page, PageHeader, Progress, Select } from "../components/ui";
import { localeNumber, translator } from "../i18n";
import { useBackendEvent, useInterval } from "../lib/hooks";
import { useSession } from "../state/session";
import "./BatteryPage.css";

const tr = translator("caruspex::BatteryPage");

// Recording: 1 resting, 2 cranking, 3 idle, 4 loaded (0 = none).
type Recording = 0 | 1 | 2 | 3 | 4;
const DURATIONS = [4, 12, 6, 6];

// levelIcon and levelColour of HealthPage.cpp.
function levelIcon(level: Level): IconName {
  return level === 0 ? "check-circle" : level === 1 ? "info" : level === 2 ? "alert" : "x-circle";
}
function levelColour(level: Level): string {
  return level === 0 ? "var(--good)" : level === 1 ? "var(--info)" : level === 2 ? "var(--warn)" : "var(--bad)";
}

const EMPTY_INPUT: BatteryInput = {
  type: 0,
  startStop: false,
  restingVolts: null,
  minutesSinceEngineOff: null,
  cranking: [],
  crankingMinimumVolts: null,
  idleVolts: null,
  loadedVolts: null,
  ambientCelsius: null,
};

export default function BatteryPage() {
  const connected = useSession((s) => s.connected);
  const [steps, setSteps] = useState<BatteryStepInfo[]>([]);
  const [type, setType] = useState<BatteryType>(0);
  const [startStop, setStartStop] = useState(false);
  const [volts, setVolts] = useState<number | null>(null);
  const [history, setHistory] = useState<number[]>([]);
  const [recording, setRecording] = useState<Recording>(0);
  const [stepValues, setStepValues] = useState<string[]>(["", "", "", ""]);
  const [report, setReport] = useState<BatteryReport | null>(null);
  const [progress, setProgress] = useState(0);

  const recordingRef = useRef<Recording>(0);
  const recordStart = useRef(0);
  const recordUntil = useRef(0);
  const buffer = useRef<VoltageSample[]>([]);
  const input = useRef<BatteryInput>({ ...EMPTY_INPUT });
  const options = useRef({ type, startStop });
  options.current = { type, startStop };

  useEffect(() => {
    void call("battery_test_steps").then(setSteps);
  }, []);

  const setStepValue = (index: number, text: string) =>
    setStepValues((prev) => {
      const next = prev.slice();
      next[index] = text;
      return next;
    });

  const evaluate = async () => {
    const i = input.current;
    if (i.restingVolts === null && i.crankingMinimumVolts === null && i.idleVolts === null && i.loadedVolts === null) return;
    i.type = options.current.type;
    i.startStop = options.current.startStop;
    setReport(await call("evaluate_battery", { input: i }));
  };

  // The battery kind and start-stop change the verdict at once.
  useEffect(() => {
    void evaluate();
  }, [type, startStop]);

  // setConnected(false): stop recording, the gauge goes grey.
  useEffect(() => {
    if (connected) return;
    recordingRef.current = 0;
    setRecording(0);
    setVolts(null);
    setHistory([]);
  }, [connected]);

  const record = (what: Recording, seconds: number) => {
    recordingRef.current = what;
    setRecording(what);
    buffer.current = [];
    recordStart.current = performance.now() / 1000;
    recordUntil.current = recordStart.current + seconds;
    setProgress(0);
    setStepValue(what - 1, what === 2 ? tr("Nimmt %1 s auf. Jetzt den Motor starten.", seconds) : tr("Misst %1 s ...", seconds));
    // Fast polling while cranking catches the lowest point.
    void call("start_voltage_poll", { intervalMs: what === 2 ? 40 : 200 });
  };

  const finishRecording = () => {
    const what = recordingRef.current;
    recordingRef.current = 0;
    setRecording(0);
    // Keep watching slowly, so the gauge stays alive.
    void call("start_voltage_poll", { intervalMs: 400 });
    const samples = buffer.current;
    if (samples.length === 0) return;
    let sum = 0;
    let lowest = samples[0]!.volts;
    for (const s of samples) {
      sum += s.volts;
      lowest = Math.min(lowest, s.volts);
    }
    const mean = sum / samples.length;
    const i = input.current;
    switch (what) {
      case 1:
        i.restingVolts = mean;
        setStepValue(0, tr("gemessen: %1 V", localeNumber(mean, 2)));
        break;
      case 2:
        i.cranking = samples.slice();
        i.crankingMinimumVolts = lowest;
        setStepValue(1, tr("tiefster Wert: %1 V", localeNumber(lowest, 2)));
        break;
      case 3:
        i.idleVolts = mean;
        setStepValue(2, tr("gemessen: %1 V", localeNumber(mean, 2)));
        break;
      case 4:
        i.loadedVolts = mean;
        setStepValue(3, tr("gemessen: %1 V", localeNumber(mean, 2)));
        break;
      default:
        break;
    }
    void evaluate();
  };

  useBackendEvent("adapter_voltage", ({ volts: v, time }) => {
    setVolts(v);
    setHistory((prev) => (prev.length >= 150 ? [...prev.slice(-149), v] : [...prev, v]));
    if (recordingRef.current === 0) return;
    buffer.current.push({ t: time, volts: v });
    if (performance.now() / 1000 >= recordUntil.current) finishRecording();
  });

  useInterval(
    () => {
      const total = recordUntil.current - recordStart.current;
      setProgress(total > 0 ? Math.min(100, ((performance.now() / 1000 - recordStart.current) / total) * 100) : 0);
    },
    recording !== 0 ? 100 : null,
  );

  const titles = [tr("Ruhespannung"), tr("Beim Anlassen"), tr("Laden im Leerlauf"), tr("Laden unter Last")];

  let summary = tr("Nach der ersten Messung steht hier, wie es um Batterie und Lichtmaschine steht.");
  if (report) {
    summary = report.summary;
    if (report.stateOfChargePercent !== null) summary += ` ${tr("Ladezustand etwa %1 %.", report.stateOfChargePercent)}`;
  }

  return (
    <Page className="battery-page">
      <PageHeader
        eyebrow={tr("Messen")}
        title={tr("Batterie und Lichtmaschine")}
        subtitle={tr("Vier kurze Messungen, Schritt für Schritt. Die Spannung misst der Adapter direkt an der Buchse, das klappt bei jedem Auto.")}
      />

      <div className="battery-columns">
        <Card className="battery-gauge-card" title={tr("Spannung jetzt")}>
          <div className="battery-gauge">
            <Gauge title={tr("Bordnetz")} unit="V" min={6} max={16} decimals={2} warnFrom={12.2} warnTo={14.9} value={volts} size={240} />
          </div>
          {history.length > 1 && volts !== null ? (
            <div className="battery-history">
              <Sparkline values={history} width={260} height={34} colour="var(--sky)" />
            </div>
          ) : null}
          <Select<BatteryType>
            value={type}
            onChange={setType}
            options={[
              { value: 0, label: tr("Batterieart unbekannt") },
              { value: 1, label: tr("Nassbatterie (Standard)") },
              { value: 2, label: tr("EFB (Start-Stopp)") },
              { value: 3, label: tr("AGM (Start-Stopp, Vlies)") },
            ]}
          />
          <Check checked={startStop} onChange={setStartStop}>
            {tr("Auto hat Start-Stopp")}
          </Check>
        </Card>

        <Card className="battery-steps-card" title={tr("So geht's")}>
          {steps.slice(0, 4).map((step, i) => {
            const what = (i + 1) as Recording;
            const active = recording === what;
            return (
              <div key={step.id} className={`battery-step${active ? " active" : ""}`}>
                <div className="battery-step-row">
                  <div className="battery-step-texts">
                    <div className="battery-step-title">{`${i + 1}. ${titles[i]}`}</div>
                    <div className="subtle">{step.instruction}</div>
                    {stepValues[i] ? <div className={`hint battery-step-value${active ? " live" : ""}`}>{stepValues[i]}</div> : null}
                    {active ? <Progress value={progress} /> : null}
                  </div>
                  <Button
                    icon="bolt-circle"
                    busy={active}
                    disabled={!connected || recording !== 0}
                    onClick={() => record(what, DURATIONS[i]!)}
                  >
                    {i === 1 ? tr("Aufnehmen, dann starten") : tr("Messen")}
                  </Button>
                </div>
                {i < 3 ? <hr className="divider" /> : null}
              </div>
            );
          })}
        </Card>
      </div>

      <Card title={tr("Ergebnis")}>
        <div className="subtle">{summary}</div>
        {report && report.steps.length > 0 ? (
          <div className="battery-results">
            {report.steps.map((step) => (
              <FindingRow
                  key={step.id}
                  icon={levelIcon(step.level)}
                  tint={levelColour(step.level)}
                  title={step.title}
                  text={<span className="wrap">{step.advice ? `${step.explanation}\n${tr("Was tun: %1", step.advice)}` : step.explanation}</span>}
                  meta={<span className="ltr">{`${localeNumber(step.volts, 2)} V`}</span>}
                />
            ))}
          </div>
        ) : null}
      </Card>
    </Page>
  );
}
