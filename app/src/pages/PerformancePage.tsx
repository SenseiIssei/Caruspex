// Performance measurement (PerformancePage.cpp): a stopwatch for 0 to 100,
// 80 to 120, 0 to 60 mph, the quarter mile and braking, from the speed
// signal alone. While it runs only the speed is polled, as fast as the
// adapter allows; analysis::analyzePerformance in the backend turns the
// samples into times.
import { useEffect, useRef, useState } from "react";
import { call } from "../api";
import type { PerformanceReport, SpeedSample, TimedRun } from "../api";
import { Icon } from "../components/Icon";
import { StatTile } from "../components/Tiles";
import { Button, Card, Page, PageHeader } from "../components/ui";
import { localeNumber, translator } from "../i18n";
import { useBackendEvent, useInterval } from "../lib/hooks";
import { useSession } from "../state/session";
import "./PerformancePage.css";

const tr = translator("caruspex::PerformancePage");

interface Shown {
  value: string;
  unit: string;
  state: string;
  note?: string;
}

function showRun(run: TimedRun, distance: boolean): Shown | null {
  if (!run.found) return null;
  const state = run.reliable ? "var(--good)" : "var(--warn)";
  if (distance && run.trapSpeedKmh <= 0) {
    return {
      value: localeNumber(run.distanceMeters, 1),
      unit: "m",
      state,
      note: tr("in %1 s", localeNumber(run.seconds, 2)),
    };
  }
  return {
    value: localeNumber(run.seconds, 2),
    unit: "s",
    state,
    note:
      run.trapSpeedKmh > 0
        ? tr("Ziel mit %1 km/h", localeNumber(run.trapSpeedKmh, 0))
        : !run.reliable
          ? tr("Lücken in der Messung, ungenau")
          : undefined,
  };
}

type RunKey = "zeroTo100" | "eightyTo120" | "zeroTo60mph" | "quarterMile" | "brake100To0";

export default function PerformancePage() {
  const connected = useSession((s) => s.connected);
  const [running, setRunning] = useState(false);
  const [speed, setSpeed] = useState<string>("--");
  const [state, setState] = useState(tr("Bereit machen: anhalten, dann auf Messen drücken."));
  const [accuracy, setAccuracy] = useState("");
  const [runs, setRuns] = useState<Partial<Record<RunKey, Shown>>>({});
  const samples = useRef<SpeedSample[]>([]);
  const runningRef = useRef(false);

  const evaluate = async () => {
    if (samples.current.length < 3) return;
    const report: PerformanceReport = await call("analyze_performance", { samples: samples.current });
    // A run that was found stays on its tile until the next measurement.
    setRuns((prev) => {
      const next = { ...prev };
      const put = (key: RunKey, distance: boolean) => {
        const shown = showRun(report[key], distance);
        if (shown) next[key] = shown;
      };
      put("zeroTo100", false);
      put("eightyTo120", false);
      put("zeroTo60mph", false);
      put("quarterMile", false);
      put("brake100To0", true);
      return next;
    });
    setAccuracy(
      tr(
        "%1 Messungen pro Sekunde, Genauigkeit etwa ±%2 s. %3",
        localeNumber(report.sampleRateHz, 1),
        localeNumber(report.accuracySeconds, 2),
        report.accuracyHint,
      ),
    );
  };

  const toggle = async () => {
    const next = !runningRef.current;
    runningRef.current = next;
    setRunning(next);
    if (next) {
      samples.current = [];
      setRuns({});
      setState(tr("Misst. Aus dem Stand losfahren oder bei 80 km/h durchbeschleunigen."));
      await call("start_live", { pids: [0x0d] });
    } else {
      setState(tr("Messung beendet. Die Zeiten bleiben stehen, bis neu gemessen wird."));
      await call("stop_live");
      await evaluate();
    }
  };

  // setConnected(false) ends a running measurement.
  useEffect(() => {
    if (!connected && runningRef.current) void toggle();
  }, [connected]);

  useInterval(() => void evaluate(), running ? 700 : null);

  useBackendEvent("live_value", ({ row }) => {
    if (row.pid !== 0x0d) return;
    setSpeed(row.value.toFixed(0));
    if (runningRef.current) samples.current.push({ t: row.time, kmh: row.value });
  });

  const tile = (key: RunKey, icon: "timer" | "route" | "stop", caption: string) => {
    const shown = runs[key];
    return <StatTile icon={icon} caption={caption} value={shown?.value} unit={shown?.unit} state={shown?.state} note={shown?.note} />;
  };

  return (
    <Page className="performance-page">
      <PageHeader
        eyebrow={tr("Messen")}
        title={tr("Leistungsmessung")}
        subtitle={tr(
          "Stoppuhr aus dem Tachosignal: 0 bis 100, 80 bis 120, Viertelmeile und Bremsweg. Während der Messung fragt Caruspex nur das Tempo ab, so oft der Adapter kann.",
        )}
      />

      {/* Safety first, and plainly. */}
      <div className="performance-warning">
        <Icon name="alert" size={20} />
        <span>
          {tr("Nur auf abgesperrter Strecke oder Rennstrecke messen, nie im Straßenverkehr. Den Laptop bedient der Beifahrer, nicht wer fährt.")}
        </span>
      </div>

      <Card className="performance-hero">
        <div className="eyebrow">{tr("TEMPO")}</div>
        <div className="performance-speed-row">
          <span className={`performance-speed num ltr${running ? " live" : ""}`}>{speed}</span>
          <span className="performance-unit ltr">km/h</span>
        </div>
        <div className="subtle">{state}</div>
        {accuracy ? <div className="hint performance-accuracy">{accuracy}</div> : null}
        <Button
          variant="primary"
          className="performance-start"
          icon={running ? "stop" : "timer"}
          disabled={!connected}
          onClick={() => void toggle()}
        >
          {running ? tr("Messung beenden") : tr("Messen")}
        </Button>
      </Card>

      <div className="performance-tiles">
        {tile("zeroTo100", "timer", tr("0 bis 100 km/h"))}
        {tile("eightyTo120", "timer", tr("80 bis 120 km/h"))}
        {tile("zeroTo60mph", "timer", tr("0 bis 60 mph"))}
        {tile("quarterMile", "route", tr("Viertelmeile (402 m)"))}
        {tile("brake100To0", "stop", tr("Bremsen 100 bis 0"))}
      </div>
    </Page>
  );
}
