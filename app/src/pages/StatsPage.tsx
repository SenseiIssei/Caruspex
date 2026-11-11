// Statistics (StatsPage.cpp): minimum, mean and maximum per quantity of the
// running measurement, plus derived figures such as the fuel used and the
// distance. On top of the Qt page each row carries a small trend line, and
// a click on a row shows how its values are spread.
import { useEffect, useRef, useState } from "react";
import { call } from "../api";
import type { LiveRow } from "../api";
import { Histogram, RangeBar, Sparkline } from "../components/charts/Mini";
import { toast } from "../components/Toast";
import { Button, Card, Page, PageHeader, cx } from "../components/ui";
import { t, translator } from "../i18n";
import { useBackendEvent, usePageActive } from "../lib/hooks";
import { useSession } from "../state/session";
import { qtFilters, stamp } from "./measure";
import "./StatsPage.css";

const tr = translator("caruspex::StatsPage");

// Petrol: air-fuel ratio 14.7 to 1, density about 745 grams per litre.
const AIR_FUEL_RATIO = 14.7;
const FUEL_DENSITY_GRAM_PER_LITRE = 745;
// Values kept per quantity for the trend line and the spread.
const KEEP = 600;

interface Series {
  pid: number;
  name: string;
  unit: string;
  minimum: number;
  maximum: number;
  sum: number;
  last: number;
  count: number;
  displayMin: number;
  displayMax: number;
  recent: number[];
}

function formatDuration(milliseconds: number): string {
  const seconds = Math.floor(milliseconds / 1000);
  return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

export default function StatsPage() {
  const connected = useSession((s) => s.connected);
  const active = usePageActive();
  const series = useRef(new Map<number, Series>());
  const clockStart = useRef<number | null>(null);
  // Fuel consumption and distance are integrated over time.
  const totals = useRef({ fuelLitres: 0, distanceKm: 0, lastMaf: 0, lastSpeed: 0, lastMafMs: -1, lastSpeedMs: -1, sawMaf: false });
  const dirty = useRef(false);
  const [, setVersion] = useState(0);
  const [selected, setSelected] = useState<number | null>(null);

  const reset = () => {
    series.current.clear();
    clockStart.current = null;
    totals.current = { fuelLitres: 0, distanceKm: 0, lastMaf: 0, lastSpeed: 0, lastMafMs: -1, lastSpeedMs: -1, sawMaf: false };
    setSelected(null);
    setVersion((v) => v + 1);
  };

  useEffect(() => {
    if (!connected) reset();
  }, [connected]);

  useBackendEvent("live_value", ({ row }: { row: LiveRow }) => {
    if (row.text) return; // status values cannot be averaged
    if (clockStart.current === null) clockStart.current = performance.now();

    let s = series.current.get(row.pid);
    if (!s) {
      s = {
        pid: row.pid,
        name: row.name,
        unit: row.unit,
        minimum: row.value,
        maximum: row.value,
        sum: 0,
        last: row.value,
        count: 0,
        displayMin: row.displayMin,
        displayMax: row.displayMax,
        recent: [],
      };
      series.current.set(row.pid, s);
    }
    s.minimum = Math.min(s.minimum, row.value);
    s.maximum = Math.max(s.maximum, row.value);
    s.sum += row.value;
    s.last = row.value;
    s.count++;
    s.recent.push(row.value);
    if (s.recent.length > KEEP) s.recent.splice(0, s.recent.length - KEEP);

    const now = performance.now() - clockStart.current;
    const tt = totals.current;
    // Integrating the air mass gives the amount of fuel burned.
    if (row.pid === 0x10) {
      tt.sawMaf = true;
      if (tt.lastMafMs >= 0) {
        const seconds = (now - tt.lastMafMs) / 1000;
        const airGrams = 0.5 * (tt.lastMaf + row.value) * seconds;
        tt.fuelLitres += airGrams / AIR_FUEL_RATIO / FUEL_DENSITY_GRAM_PER_LITRE;
      }
      tt.lastMaf = row.value;
      tt.lastMafMs = now;
    }
    // Integrating the speed gives the distance.
    if (row.pid === 0x0d) {
      if (tt.lastSpeedMs >= 0) {
        const hours = (now - tt.lastSpeedMs) / 3600000;
        tt.distanceKm += 0.5 * (tt.lastSpeed + row.value) * hours;
      }
      tt.lastSpeed = row.value;
      tt.lastSpeedMs = now;
    }
    dirty.current = true;
  });

  // The figures follow a few times a second while the page is on screen.
  useEffect(() => {
    if (!active) return;
    setVersion((v) => v + 1);
    const id = setInterval(() => {
      if (!dirty.current) return;
      dirty.current = false;
      setVersion((v) => v + 1);
    }, 250);
    return () => clearInterval(id);
  }, [active]);

  const list = [...series.current.values()];
  const elapsed = clockStart.current === null ? 0 : performance.now() - clockStart.current;
  const tt = totals.current;
  let samples = 0;
  for (const s of list) samples += s.count;
  const rpm = series.current.get(0x0c);
  const speed = series.current.get(0x0d);
  let consumption = "--";
  if (tt.sawMaf && tt.fuelLitres > 0) {
    if (tt.distanceKm > 0.1) {
      consumption = `${((tt.fuelLitres / tt.distanceKm) * 100).toFixed(1)} l/100km`;
    } else {
      // When stationary there is no distance, so the flow rate is what matters.
      const hours = elapsed / 3600000;
      consumption = hours > 0 ? `${(tt.fuelLitres / hours).toFixed(2)} l/h` : "--";
    }
  }
  const empty = list.length === 0;
  const cards: [string, string][] = [
    [tr("Messdauer"), empty ? "--" : formatDuration(elapsed)],
    [tr("Messpunkte"), empty ? "--" : String(samples)],
    [tr("Höchste Drehzahl"), rpm ? rpm.maximum.toFixed(0) : "--"],
    [tr("Höchste Geschwindigkeit"), speed ? speed.maximum.toFixed(0) : "--"],
    [tr("Verbrauch geschätzt"), consumption],
    [tr("Strecke seit Start"), tt.distanceKm > 0 ? tt.distanceKm.toFixed(2) : "--"],
  ];

  const exportCsv = async () => {
    if (series.current.size === 0) return;
    const lines = [tr("Messwert;Kleinster;Mittel;Größter;Aktuell;Einheit;Messpunkte")];
    for (const s of series.current.values()) {
      lines.push(
        `${s.name};${s.minimum.toFixed(3)};${(s.sum / s.count).toFixed(3)};${s.maximum.toFixed(3)};${s.last.toFixed(3)};${s.unit};${s.count}`,
      );
    }
    try {
      await call("save_text_file", {
        title: tr("Speichern"),
        suggestedName: tr("statistik_%1.csv", stamp()),
        filters: qtFilters(tr("CSV (*.csv)")),
        content: `${lines.join("\n")}\n`,
      });
    } catch {
      toast(`${tr("Fehler")}: ${tr("Datei lässt sich nicht schreiben.")}`, "error", 6000);
    }
  };

  const chosen = selected !== null ? series.current.get(selected) : undefined;
  const fmt = (v: number) => v.toFixed(2);

  return (
    <Page className="stats-page">
      <PageHeader eyebrow={tr("MESSEN")} title={tr("Statistik")} />

      <div className="row wrap-row">
        <Button icon="refresh" onClick={reset}>
          {tr("Zurücksetzen")}
        </Button>
        <Button icon="download" disabled={empty} onClick={() => void exportCsv()}>
          {tr("Als CSV speichern")}
        </Button>
      </div>

      <div className="stats-cards">
        {cards.map(([caption, value]) => (
          <div className="card flat stats-card" key={caption}>
            <div className="stats-card-value num ltr">{value}</div>
            <div className="subtle stats-card-caption">{caption}</div>
          </div>
        ))}
      </div>

      <div className="table-wrap stats-table">
        <table className="table">
          <thead>
            <tr>
              <th>{tr("Messwert")}</th>
              <th className="stats-trend-head" aria-hidden />
              <th className="num">{tr("Kleinster")}</th>
              <th className="num">{tr("Mittel")}</th>
              <th className="num">{tr("Größter")}</th>
              <th className="num">{tr("Aktuell")}</th>
              <th>{tr("Einheit")}</th>
            </tr>
          </thead>
          <tbody>
            {list.map((s, i) => {
              const mean = s.sum / s.count;
              const low = s.displayMax > s.displayMin ? Math.min(s.displayMin, s.minimum) : s.minimum;
              const high = s.displayMax > s.displayMin ? Math.max(s.displayMax, s.maximum) : s.maximum;
              return (
                <tr
                  key={s.pid}
                  className={cx("fade-in", "stats-row", selected === s.pid && "selected")}
                  onClick={() => setSelected(selected === s.pid ? null : s.pid)}
                >
                  <td>{s.name}</td>
                  <td className="stats-trend">
                    <Sparkline values={s.recent.slice(-80)} width={110} height={22} colour={`var(--series-${(i % 8) + 1})`} />
                    <RangeBar low={low} high={high} min={s.minimum} max={s.maximum} mean={mean} current={s.last} width={110} height={10} />
                  </td>
                  <td className="num">{s.minimum.toFixed(2)}</td>
                  <td className="num">{mean.toFixed(2)}</td>
                  <td className="num">{s.maximum.toFixed(2)}</td>
                  <td className="num strong">{s.last.toFixed(2)}</td>
                  <td className="muted">{s.unit}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {chosen ? (
        <Card
          className="fade-in"
          title={`${chosen.name}${chosen.unit ? ` (${chosen.unit})` : ""}`}
          icon="chart"
          actions={<Button variant="ghost" small icon="x" title={t("caruspex::TitleBar", "Schließen")} onClick={() => setSelected(null)} />}
        >
          <Histogram
            label={chosen.name}
            values={chosen.recent}
            height={150}
            format={fmt}
            markers={[
              { value: chosen.minimum, label: tr("Kleinster"), colour: "var(--info)" },
              { value: chosen.sum / chosen.count, label: tr("Mittel"), colour: "var(--text-muted)" },
              { value: chosen.maximum, label: tr("Größter"), colour: "var(--warn)" },
            ]}
          />
        </Card>
      ) : null}

      <div className="hint">
        {tr(
          "Der Verbrauch wird aus der Luftmasse gerechnet (Luftverhältnis 14,7 zu 1, Dichte 745 Gramm je Liter) und gilt so nur für Benziner. Die Strecke ergibt sich aus dem Geschwindigkeitssignal. Beides sind Schätzungen aus den Messwerten, keine Werte vom Bordcomputer.",
        )}
      </div>
    </Page>
  );
}
