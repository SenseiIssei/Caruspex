// The history tab: every check with its score trend.
import type { ReactNode } from "react";
import type { GarageCar } from "../../api";
import { FindingRow } from "../../components/Tiles";
import { localeNumber, translator } from "../../i18n";
import { odometerJumps } from "../../state/garage";
import { shortDate, shortDateTime } from "./shared";

const tr = translator("caruspex::GaragePage");

// --- Score trend ------------------------------------------------------------------------------

// The points of every check over time, so a car getting worse shows at a
// glance.
function ScoreTrend({ car }: { car: GarageCar }) {
  const points = car.checks.filter((c) => c.score >= 0);
  if (points.length < 2) return null;
  const w = 600;
  const h = 70;
  const pad = 8;
  const x = (i: number) => pad + (i * (w - 2 * pad)) / (points.length - 1);
  const y = (score: number) => pad + ((100 - Math.max(0, Math.min(100, score))) * (h - 2 * pad)) / 100;
  const line = points.map((p, i) => `${i === 0 ? "M" : "L"}${x(i).toFixed(1)},${y(p.score).toFixed(1)}`).join(" ");
  const area = `${line} L${x(points.length - 1).toFixed(1)},${h - pad} L${x(0).toFixed(1)},${h - pad} Z`;
  const last = points[points.length - 1]!;
  const tone = (s: number) => (s >= 80 ? "var(--good)" : s >= 50 ? "var(--warn)" : "var(--bad)");
  return (
    <div className="garage-trend fade-in">
      <div className="row">
        <span className="hint grow">{tr("Punkte der Prüfungen")}</span>
        <span className="strong num" style={{ color: tone(last.score) }}>
          {tr("%1 Punkte", last.score)}
        </span>
      </div>
      <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" className="garage-trend-chart" aria-hidden>
        <defs>
          <linearGradient id="garage-trend-fill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="var(--accent)" stopOpacity="0.28" />
            <stop offset="1" stopColor="var(--accent)" stopOpacity="0" />
          </linearGradient>
        </defs>
        <line x1={pad} x2={w - pad} y1={y(50)} y2={y(50)} stroke="var(--track)" strokeDasharray="4 6" vectorEffect="non-scaling-stroke" />
        <path d={area} fill="url(#garage-trend-fill)" />
        <path d={line} fill="none" stroke="var(--accent)" strokeWidth={2} strokeLinejoin="round" vectorEffect="non-scaling-stroke" />
      </svg>
      <div className="garage-trend-dots">
        {points.map((p, i) => (
          <span
            key={`${p.when}-${i}`}
            className="garage-trend-dot"
            title={`${shortDateTime(p.when)}: ${tr("%1 Punkte", p.score)}`}
            style={{ insetInlineStart: `${(x(i) / w) * 100}%`, top: `${(y(p.score) / h) * 100}%`, background: tone(p.score) }}
          />
        ))}
      </div>
    </div>
  );
}

export function HistoryTab({ car }: { car: GarageCar }) {
  const rows: ReactNode[] = [];
  // A falling odometer between two checks is worth a word, especially when
  // buying a used car. It is a hint: a swapped instrument cluster or a
  // mistake in reading can explain it too.
  odometerJumps(car).forEach((j, i) =>
    rows.push(
      <FindingRow
        key={`jump-${i}`}
        icon="alert"
        tint="var(--bad)"
        title={tr("Kilometerstand ist gesunken")}
        text={tr(
          "Am %1 waren es %2 km, am %3 nur noch %4 km. Das kann ein getauschtes Kombiinstrument sein, aber auch ein zurückgedrehter Tacho. Beim Gebrauchtwagenkauf nachfragen.",
          shortDate(j.earlier),
          localeNumber(j.earlierKm, 0),
          shortDate(j.later),
          localeNumber(j.laterKm, 0),
        )}
      />,
    ),
  );
  if (car.checks.length === 0) {
    rows.push(
      <FindingRow
        key="none"
        icon="info"
        tint="var(--info)"
        title={tr("Noch keine Prüfung gespeichert")}
        text={tr("Auto prüfen auf der Startseite legt einen Eintrag an.")}
      />,
    );
  }
  for (let i = car.checks.length - 1; i >= 0; i--) {
    const c = car.checks[i]!;
    let title = shortDateTime(c.when);
    if (c.odometerKm >= 0) title += `  ·  ${localeNumber(c.odometerKm, 0)} km`;
    const parts = c.codes.map((code, k) => (k < c.texts.length ? `${code} ${c.texts[k]}` : code));
    let text = parts.length === 0 ? tr("Kein Fehler gespeichert.") : parts.join("\n");
    if (c.mil) text += `\n${tr("Motorkontrollleuchte an.")}`;
    if (c.openMonitors > 0) text += `\n${tr.n("%n Bereitschaftstest(s) offen.", c.openMonitors)}`;
    rows.push(
      <FindingRow
        key={`${c.when}-${i}`}
        icon={c.codes.length === 0 ? "check-circle" : "alert"}
        tint={c.codes.length === 0 ? "var(--good)" : "var(--bad)"}
        title={title}
        text={<span className="wrap">{text}</span>}
        meta={c.score >= 0 ? tr("%1 Punkte", c.score) : undefined}
      />,
    );
  }
  return (
    <div className="garage-tab">
      <ScoreTrend car={car} />
      <div className="garage-rows">{rows}</div>
    </div>
  );
}
