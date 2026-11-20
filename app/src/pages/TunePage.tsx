// Tuning simulator: a virtual BMW N55 (435i) on the desk. Offsets per rpm
// band for boost, ignition and lambda, the curves stock against changed, the
// limits the change touches, and the comparison with a recording of the real
// car. Simulation only: nothing here reaches the worker or the car, and
// Caruspex never writes engine maps (crates/caruspex-tune).
import { useEffect, useMemo, useState } from "react";
import { call } from "../api";
import type { TuneBand, TuneComparison, TuneCurve, TuneInput, TunePair, TuneResult, TuneWarning } from "../api";
import { Icon } from "../components/Icon";
import { Button, Card, NumberInput, Page, PageHeader, Pill, Segmented } from "../components/ui";
import { localeNumber, translator } from "../i18n";
import { useAppEvent, usePageActive } from "../lib/hooks";
import { navigate } from "../shell/nav";
import { settingsStore } from "../state/session";
import "./TunePage.css";

const tr = translator("caruspex::TunePage");

// The rpm bands of the offset table; the last one includes the redline.
const BANDS: readonly [number, number][] = [
  [1000, 2000],
  [2000, 3500],
  [3500, 5000],
  [5000, 6000],
  [6000, 7000],
];

type Offsets = { boostBar: number; ignitionDeg: number; lambda: number }[];

const zeroOffsets = (): Offsets => BANDS.map(() => ({ boostBar: 0, ignitionDeg: 0, lambda: 0 }));

interface Conditions {
  ambientC: number;
  ambientHpa: number;
  octane: number;
  gearbox: "automatic" | "manual";
}

const DEFAULT_CONDITIONS: Conditions = { ambientC: 20, ambientHpa: 1013, octane: 98, gearbox: "automatic" };

function toInput(offsets: Offsets, conditions: Conditions): TuneInput {
  const bands: TuneBand[] = BANDS.map(([fromRpm, toRpm], i) => ({ fromRpm, toRpm, ...offsets[i]! }));
  return { bands, ...conditions };
}

// With a plus for gains; a value that rounds to zero gets no sign at all.
const signed = (value: number, decimals: number) => {
  const rounded = Number(value.toFixed(decimals));
  if (rounded === 0) return localeNumber(0, decimals);
  return (rounded > 0 ? "+" : "") + localeNumber(rounded, decimals);
};

// --- The chart ------------------------------------------------------------------------

const W = 760;
const H = 300;
const PAD = { left: 52, right: 52, top: 34, bottom: 34 };

function Curves({ stock, changed }: { stock: TuneCurve; changed: TuneCurve }) {
  const all = [...stock.points, ...changed.points];
  // Round tops, so the five steps of the grid are round numbers too.
  const maxNm = Math.max(500, Math.ceil(Math.max(...all.map((p) => p.torqueNm)) / 100) * 100);
  const maxPs = Math.max(400, Math.ceil(Math.max(...all.map((p) => p.powerPs)) / 100) * 100);
  const x = (rpm: number) => PAD.left + ((rpm - 1000) / 6000) * (W - PAD.left - PAD.right);
  const yNm = (nm: number) => H - PAD.bottom - (nm / maxNm) * (H - PAD.top - PAD.bottom);
  const yPs = (ps: number) => H - PAD.bottom - (ps / maxPs) * (H - PAD.top - PAD.bottom);
  const path = (curve: TuneCurve, y: (v: number) => number, value: (p: TuneCurve["points"][number]) => number) =>
    curve.points.map((p, i) => `${i ? "L" : "M"}${x(p.rpm).toFixed(1)},${y(value(p)).toFixed(1)}`).join(" ");
  const ticks = [0, 0.2, 0.4, 0.6, 0.8, 1];
  return (
    <svg className="tune-chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label={tr("Drehmoment und Leistung über der Drehzahl")}>
      {ticks.map((f) => {
        const y = H - PAD.bottom - f * (H - PAD.top - PAD.bottom);
        return (
          <g key={f}>
            <line className="tune-grid" x1={PAD.left} x2={W - PAD.right} y1={y} y2={y} />
            <text className="tune-axis torque" x={PAD.left - 8} y={y + 4} textAnchor="end">
              {Math.round(f * maxNm)}
            </text>
            <text className="tune-axis power" x={W - PAD.right + 8} y={y + 4}>
              {Math.round(f * maxPs)}
            </text>
          </g>
        );
      })}
      {[1000, 2000, 3000, 4000, 5000, 6000, 7000].map((rpm) => (
        <text key={rpm} className="tune-axis" x={x(rpm)} y={H - PAD.bottom + 18} textAnchor="middle">
          {rpm}
        </text>
      ))}
      <text className="tune-axis torque" x={PAD.left - 8} y={PAD.top - 16} textAnchor="end">
        Nm
      </text>
      <text className="tune-axis power" x={W - PAD.right + 8} y={PAD.top - 16}>
        PS
      </text>
      <text className="tune-axis" x={W - PAD.right} y={H - 2} textAnchor="end">
        {tr("1/min")}
      </text>
      <path className="tune-line torque stock" d={path(stock, yNm, (p) => p.torqueNm)} />
      <path className="tune-line power stock" d={path(stock, yPs, (p) => p.powerPs)} />
      <path className="tune-line torque" d={path(changed, yNm, (p) => p.torqueNm)} />
      <path className="tune-line power" d={path(changed, yPs, (p) => p.powerPs)} />
    </svg>
  );
}

// --- Warnings ------------------------------------------------------------------------

function severityTone(w: TuneWarning): string {
  return w.severity === "note" ? "var(--info)" : w.severity === "limit" ? "var(--bad)" : "var(--warn)";
}

function severityText(w: TuneWarning): string {
  return w.severity === "note" ? tr("Hinweis") : w.severity === "limit" ? tr("Grenze") : tr("Warnung");
}

function rpmRange(from: number, to: number): string {
  return from === to ? tr("bei %1 1/min", from) : tr("%1 bis %2 1/min", from, to);
}

// --- Comparison table ----------------------------------------------------------------

function cell(pair: TunePair, decimals: number, percent: boolean) {
  if (pair.measured === null || pair.simulated === null) return <td className="faint">-</td>;
  const off = percent ? pair.deviationPct : pair.difference;
  return (
    <td className="num ltr">
      {localeNumber(pair.measured, decimals)} / {localeNumber(pair.simulated, decimals)}
      {off !== null ? <span className="tune-dev"> ({percent ? `${signed(off, 1)} %` : signed(off, decimals)})</span> : null}
    </td>
  );
}

function Comparison({ result, example }: { result: TuneComparison; example: boolean }) {
  return (
    <div className="tune-compare fade-in">
      {example ? (
        <div className="tune-example">
          <Icon name="info" size={16} />
          {tr("Beispiel mit erfundenen Messwerten, kein echtes Auto.")}
        </div>
      ) : null}
      <div className="hint wrap">
        {tr(
          "%1 von %2 Messpunkten bei Volllast verglichen, %3 in Teillast und %4 außerhalb von 1000 bis 7000 1/min weggelassen.",
          result.used,
          result.total,
          result.partLoad,
          result.outOfRange,
        )}
      </div>
      {result.bands.length === 0 ? (
        <div className="subtle">{tr("Keine Volllast im Mitschnitt: der Motor muss dafür eine Weile voll beschleunigen.")}</div>
      ) : (
        <div className="table-wrap">
          <table className="table tune-compare-table">
            <thead>
              <tr>
                <th>{tr("Drehzahl")}</th>
                <th className="num">{tr("Punkte")}</th>
                <th>{tr("Saugrohrdruck kPa")}</th>
                <th>{tr("Ladeluft °C")}</th>
                <th>{tr("Luftmasse g/s")}</th>
                <th>{tr("Zündung °")}</th>
                <th>{tr("Lambda")}</th>
              </tr>
            </thead>
            <tbody>
              {result.bands.map((b) => (
                <tr key={b.fromRpm}>
                  <td className="ltr">
                    {b.fromRpm} - {b.toRpm}
                  </td>
                  <td className="num">{b.samples}</td>
                  {cell(b.manifoldKpa, 0, true)}
                  {cell(b.chargeAirC, 0, false)}
                  {cell(b.airGs, 0, true)}
                  {cell(b.timingDeg, 1, false)}
                  {cell(b.lambda, 2, false)}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <div className="hint wrap">
        {tr(
          "Je Zelle: gemessen / simuliert (Abweichung). Saugrohrdruck und Ladeluft rechnet das Modell mit den eingestellten Kennfeldern; Luftmasse und Zündung mit dem gemessenen Druck und der gemessenen Temperatur. Das zeigt, ob das Modell den Zustand des Autos erklärt.",
        )}
      </div>
    </div>
  );
}

// --- The page ------------------------------------------------------------------------

export default function TunePage() {
  const active = usePageActive();
  const [offsets, setOffsets] = useState<Offsets>(zeroOffsets);
  const [conditions, setConditions] = useState<Conditions>(DEFAULT_CONDITIONS);
  const [result, setResult] = useState<TuneResult | null>(null);
  const [comparison, setComparison] = useState<{ result: TuneComparison; example: boolean } | null>(null);
  const [compareError, setCompareError] = useState("");
  const [logPath, setLogPath] = useState<string | null>(null);
  const input = useMemo(() => toInput(offsets, conditions), [offsets, conditions]);

  // Recomputed on every change; a whole curve takes far less than a millisecond.
  useEffect(() => {
    if (!active) return;
    let stale = false;
    void call("tune_simulate", { input }).then((r) => {
      if (!stale) setResult(r);
    });
    return () => {
      stale = true;
    };
  }, [active, input]);

  // A comparison follows the maps it is compared with.
  const compare = async (path: string | null, example: boolean) => {
    setCompareError("");
    try {
      const r = example ? await call("tune_compare_example", { input }) : await call("tune_compare_log", { path: path!, input });
      setComparison({ result: r, example });
    } catch (e) {
      setComparison(null);
      setCompareError(String(e));
    }
  };

  useEffect(() => {
    if (!active || !comparison) return;
    void compare(logPath, comparison.example);
    // Only a change of the maps or conditions repeats the comparison.
  }, [input]);

  const openLog = async () => {
    const path = await call("pick_open_path", {
      title: tr("Mitschnitt öffnen"),
      filters: [{ name: tr("Caruspex-Mitschnitt"), extensions: ["jsonl"] }],
    });
    if (!path) return;
    setLogPath(path);
    await compare(path, false);
  };

  // --page tune-demo: a changed map with its warnings and the example
  // comparison, for screenshots; tune-demo:compare scrolls further down.
  // Pro mode only for this run.
  useAppEvent<string>("dev:page", (name) => {
    if (name !== "tune-demo" && name !== "tune-demo:compare") return;
    settingsStore.set({ modeOverride: true });
    navigate("tune");
    const demo = zeroOffsets();
    demo[1] = { boostBar: 0.2, ignitionDeg: 0, lambda: 0 };
    demo[2] = { boostBar: 0.2, ignitionDeg: 1, lambda: -0.03 };
    demo[3] = { boostBar: 0.15, ignitionDeg: 1, lambda: -0.03 };
    setOffsets(demo);
    setComparison({ result: { file: "", total: 0, used: 0, partLoad: 0, outOfRange: 0, bands: [] }, example: true });
    void call("tune_compare_example", { input: toInput(demo, DEFAULT_CONDITIONS) }).then((r) => {
      setComparison({ result: r, example: true });
      // The chart (or, with :compare, the warnings) at the top of the window.
      const target = name === "tune-demo:compare" ? ".tune-page .tune-warnings" : ".tune-page .tune-peaks";
      setTimeout(() => document.querySelector(target)?.closest(".card")?.scrollIntoView(), 300);
    });
  });

  const setOffset = (band: number, key: keyof Offsets[number], value: number) =>
    setOffsets((list) => list.map((o, i) => (i === band ? { ...o, [key]: value } : o)));

  const changedAny = offsets.some((o) => o.boostBar !== 0 || o.ignitionDeg !== 0 || o.lambda !== 0);
  const stock = result?.stock;
  const changed = result?.changed;
  const torqueShare = result ? Math.round(result.uncertainty.torqueShare * 100) : 8;

  const peak = (curve: TuneCurve | undefined, which: "torque" | "power") =>
    curve ? (which === "torque" ? localeNumber(curve.peakTorque.nm, 0) : localeNumber(curve.peakPower.ps, 0)) : "--";
  const delta = (which: "torque" | "power") => {
    if (!stock || !changed) return "";
    const d = which === "torque" ? changed.peakTorque.nm - stock.peakTorque.nm : changed.peakPower.ps - stock.peakPower.ps;
    return Math.abs(d) < 0.5 ? "" : signed(d, 0);
  };

  // Inner values at every 1000 rpm from 1500.
  const shown = [1500, 2500, 3500, 4500, 5500, 6500];
  const at = (curve: TuneCurve | undefined, rpm: number) => curve?.points.find((p) => p.rpm === rpm);
  const rows: { label: string; value: (p: TuneCurve["points"][number]) => string }[] = [
    { label: tr("Ladedruck bar"), value: (p) => localeNumber(p.boostBar, 2) },
    { label: tr("Ladeluft °C"), value: (p) => localeNumber(p.chargeAirC, 0) },
    { label: tr("Zündung °"), value: (p) => localeNumber(p.timingDeg, 1) },
    { label: tr("Klopfreserve °"), value: (p) => localeNumber(p.knockMarginDeg, 1) },
    { label: tr("Abgas °C"), value: (p) => localeNumber(p.egtC, 0) },
    { label: tr("Turbo %"), value: (p) => localeNumber(p.turboLoad * 100, 0) },
    { label: tr("Hochdruckpumpe %"), value: (p) => localeNumber(p.pumpLoad * 100, 0) },
    { label: tr("Mitteldruck bar"), value: (p) => localeNumber(p.bmepBar, 1) },
    { label: tr("Lambda"), value: (p) => localeNumber(p.lambda, 2) },
  ];

  return (
    <Page className="tune-page">
      <PageHeader
        eyebrow={tr("WERKSTATT")}
        title={tr("Tuning-Simulator")}
        subtitle={tr(
          "Ein virtueller BMW N55 (435i, 306 PS, 400 Nm) mit veröffentlichten Daten. Verändere Ladedruck, Zündung und Lambda je Drehzahlband und sieh, was der Motor dazu sagt, bevor irgendetwas das Auto berührt.",
        )}
      />

      <div className="tune-banner">
        <Icon name="shield" size={22} />
        <div>
          <strong>{tr("Nur Simulation, schreibt nichts ins Auto")}</strong>
          <div>
            {tr(
              "Caruspex rechnet hier nur am Schreibtisch und schreibt nie Kennfelder ins Steuergerät. Echtes Tuning gehört zu etablierten Werkzeugen und in eine Werkstatt, mit Abnahme der Änderung (in Deutschland durch den TÜV).",
            )}
          </div>
        </div>
      </div>

      <Card title={tr("Bedingungen")} icon="thermometer">
        <div className="tune-conditions">
          <label className="tune-field">
            <span className="ws-caption">{tr("Getriebe")}</span>
            <Segmented
              value={conditions.gearbox}
              onChange={(gearbox) => setConditions((c) => ({ ...c, gearbox }))}
              options={[
                { value: "automatic", label: tr("Automatik 8HP45") },
                { value: "manual", label: tr("Schaltgetriebe 6-Gang") },
              ]}
            />
          </label>
          <label className="tune-field">
            <span className="ws-caption">{tr("Kraftstoff (ROZ)")}</span>
            <Segmented
              value={conditions.octane}
              onChange={(octane) => setConditions((c) => ({ ...c, octane }))}
              options={[95, 98, 100, 102].map((v) => ({ value: v, label: String(v) }))}
            />
          </label>
          <label className="tune-field">
            <span className="ws-caption">{tr("Außentemperatur")}</span>
            <NumberInput value={conditions.ambientC} min={-30} max={50} suffix="°C" onChange={(ambientC) => setConditions((c) => ({ ...c, ambientC }))} />
          </label>
          <label className="tune-field">
            <span className="ws-caption">{tr("Luftdruck")}</span>
            <NumberInput value={conditions.ambientHpa} min={600} max={1100} step={5} suffix="hPa" onChange={(ambientHpa) => setConditions((c) => ({ ...c, ambientHpa }))} />
          </label>
        </div>
      </Card>

      <Card
        title={tr("Änderungen je Drehzahlband")}
        icon="sliders"
        actions={
          <div className="tune-actions">
            <Button icon="car" onClick={() => setOffsets(zeroOffsets())} disabled={!changedAny}>
              {tr("Serie")}
            </Button>
            <Button
              icon="refresh"
              onClick={() => {
                setOffsets(zeroOffsets());
                setConditions(DEFAULT_CONDITIONS);
              }}
            >
              {tr("Zurücksetzen")}
            </Button>
          </div>
        }
      >
        <div className="hint wrap">
          {tr("Zusätzlich zur Serie: Ladedruck in bar, Zündung in Grad früher (+) oder später (-), Lambda kleiner heißt fetter.")}
        </div>
        <div className="table-wrap">
          <table className="table tune-offsets">
            <thead>
              <tr>
                <th>{tr("Drehzahl")}</th>
                <th>{tr("Ladedruck bar")}</th>
                <th>{tr("Zündung °")}</th>
                <th>{tr("Lambda")}</th>
              </tr>
            </thead>
            <tbody>
              {BANDS.map(([from, to], i) => (
                <tr key={from}>
                  <td className="ltr">
                    {from} - {to}
                  </td>
                  <td>
                    <NumberInput value={offsets[i]!.boostBar} min={-0.5} max={0.8} step={0.05} decimals={2} onChange={(v) => setOffset(i, "boostBar", v)} />
                  </td>
                  <td>
                    <NumberInput value={offsets[i]!.ignitionDeg} min={-10} max={6} step={0.5} decimals={1} onChange={(v) => setOffset(i, "ignitionDeg", v)} />
                  </td>
                  <td>
                    <NumberInput value={offsets[i]!.lambda} min={-0.2} max={0.1} step={0.01} decimals={2} onChange={(v) => setOffset(i, "lambda", v)} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      <Card title={tr("Drehmoment und Leistung")} icon="chart">
        <div className="tune-peaks">
          <div className="tune-peak">
            <div className="ws-caption">{tr("Drehmoment")}</div>
            <div className="tune-peak-value num ltr">
              {peak(changed, "torque")} <span className="tune-unit">Nm</span> <span className="tune-delta">{delta("torque")}</span>
            </div>
            <div className="faint">{tr("Serie %1 Nm", peak(stock, "torque"))}</div>
          </div>
          <div className="tune-peak">
            <div className="ws-caption">{tr("Leistung")}</div>
            <div className="tune-peak-value num ltr">
              {peak(changed, "power")} <span className="tune-unit">PS</span> <span className="tune-delta">{delta("power")}</span>
            </div>
            <div className="faint">{tr("Serie %1 PS", peak(stock, "power"))}</div>
          </div>
          <div className="tune-legend">
            <span>
              <i className="swatch torque" /> {tr("Drehmoment")}
            </span>
            <span>
              <i className="swatch power" /> {tr("Leistung")}
            </span>
            <span>
              <i className="swatch dashed" /> {tr("Serie")}
            </span>
          </div>
        </div>
        {stock && changed ? <Curves stock={stock} changed={changed} /> : <div className="tune-chart" />}
        <div className="hint wrap">
          {tr(
            "Alles sind Schätzungen eines Modells, etwa ±%1 % bei Drehmoment und Leistung. Das Modell ist noch an keinem echten Auto geprüft. Rechenzeit für beide Kurven: %2 ms.",
            torqueShare,
            result ? localeNumber(result.micros / 1000, 3) : "--",
          )}
        </div>
      </Card>

      <Card title={tr("Grenzen und Warnungen")} icon="alert">
        {changed && changed.warnings.length === 0 ? (
          <div className="tune-ok">
            <Icon name="check-circle" size={18} />
            {tr("Keine Grenze berührt.")}
          </div>
        ) : null}
        <ul className="tune-warnings">
          {(changed?.warnings ?? []).map((w, i) => (
            <li key={`${w.kind}-${w.fromRpm}-${i}`} className={`tune-warning ${w.severity}`}>
              <div className="tune-warning-head">
                <Pill tone={severityTone(w)}>{severityText(w)}</Pill>
                <strong>{w.title}</strong>
                <span className="faint ltr">{rpmRange(w.fromRpm, w.toRpm)}</span>
              </div>
              <div>{w.reason}</div>
            </li>
          ))}
        </ul>
      </Card>

      <Card title={tr("Innenwerte")} icon="engine">
        <div className="table-wrap">
          <table className="table tune-inner">
            <thead>
              <tr>
                <th>{tr("Drehzahl")}</th>
                {shown.map((rpm) => (
                  <th key={rpm} className="num ltr">
                    {rpm}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.label}>
                  <td>{row.label}</td>
                  {shown.map((rpm) => {
                    const c = at(changed, rpm);
                    const s = at(stock, rpm);
                    const now = c ? row.value(c) : "--";
                    const before = s ? row.value(s) : "";
                    return (
                      <td key={rpm} className="num ltr">
                        {now}
                        {changedAny && before !== now ? <span className="tune-was"> ({before})</span> : null}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="hint wrap">{tr("In Klammern der Wert der Serie, wo er sich unterscheidet.")}</div>
      </Card>

      <Card title={tr("Vergleich mit einem Mitschnitt")} icon="activity">
        <div className="ws-explain">
          {tr(
            "Nimm im Auto eine Fahrt mit den Live-Werten auf (Drehzahl, Last, Saugrohrdruck, Ansauglufttemperatur, Zündzeitpunkt, Lambda, Luftmasse) und beschleunige dabei einmal voll durch, nur wo das erlaubt und sicher ist. Caruspex vergleicht dann die Volllast-Punkte mit dem Modell.",
          )}
        </div>
        <div className="tune-actions">
          <Button variant="primary" icon="file" onClick={() => void openLog()}>
            {tr("Mitschnitt öffnen")}
          </Button>
          <Button icon="sparkles" onClick={() => void compare(null, true)}>
            {tr("Beispiel ansehen")}
          </Button>
        </div>
        {compareError ? <div className="tune-error">{compareError}</div> : null}
        {comparison ? (
          <>
            {comparison.result.file ? <div className="faint">{comparison.result.file}</div> : null}
            <Comparison result={comparison.result} example={comparison.example} />
          </>
        ) : null}
      </Card>
    </Page>
  );
}
