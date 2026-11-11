// Inspection report as a printable document (Report.cpp): what the car is,
// what it said, what that means. Written as HTML, shown in the preview,
// saved as PDF by the backend or printed. The garage file of one car is the
// second document of this kind.
import { call } from "../api";
import type { FaultRow, GarageCar, LiveRow, MonitorRow, ReportExtras, UsedCarRow, UsedCarSection } from "../api";
import { localeNumber, locale, t } from "../i18n";
import { fuelLogConsumption, odometerJumps, parseIso, serviceStatus } from "../state/garage";
import { live, readiness, reportFindings, session, type ReadinessRow } from "../state/session";

const R = "caruspex::Report";
const tr = (de: string, ...args: unknown[]) => t(R, de, ...args);

// What reportHtml needs to know about the connected car (VehicleState).
export interface ReportState {
  displayName: string;
  description: string;
  vin: string;
  ecuName: string;
  calibration: string;
  // Odometer from the live values, null if never read.
  odometerKm: number | null;
  adapter: string;
  transport: string;
  protocol: string;
  haveFaults: boolean;
  faults: FaultRow[];
  // Where on the car each code points (locateDtc), by code.
  faultZones: Record<string, string>;
  haveStatus: boolean;
  mil: boolean;
  readiness: ReadinessRow[];
  freezeCode: string;
  freezeFrame: LiveRow[];
  monitors: MonitorRow[];
}

// A snapshot of the session for the report. Async because the zone of each
// code comes from the backend.
export async function collectReportState(): Promise<ReportState> {
  const s = session.get();
  const zones: Record<string, string> = {};
  await Promise.all(
    s.faults.map(async (f) => {
      try {
        zones[f.code] = (await call("locate_dtc", { code: f.code })).name;
      } catch {
        zones[f.code] = "";
      }
    }),
  );
  return {
    displayName: s.identity?.title ?? "",
    description: s.identity?.subtitle ?? "",
    vin: s.vin,
    ecuName: s.ecuName,
    calibration: s.calibration,
    odometerKm: live.has(0xa6) ? live.get(0xa6)!.value : null,
    adapter: s.adapter,
    transport: s.transport,
    protocol: s.protocol,
    haveFaults: s.haveFaults,
    faults: s.faults,
    faultZones: zones,
    haveStatus: s.status !== null,
    mil: s.status?.mil ?? false,
    readiness: readiness(s.status),
    freezeCode: s.freezeCode,
    freezeFrame: s.freezeRows,
    monitors: s.monitors,
  };
}

// collectReportExtras() of MainWindow: the start page's rating and the scan
// of all control units.
export function collectReportExtras(): ReportExtras {
  const s = session.get();
  return { score: s.score, scoreCaption: s.scoreCaption, findings: reportFindings(), ecus: s.udsEcus };
}

// QString::toHtmlEscaped
function esc(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function scoreColour(score: number): string {
  if (score < 0) return "#6b7280";
  if (score >= 85) return "#12875a";
  if (score >= 60) return "#b97a06";
  return "#c93636";
}

// QString::arg(value, 0, 'g', 5)
function g5(v: number): string {
  if (!Number.isFinite(v)) return String(v);
  return String(Number(v.toPrecision(5)));
}

const pad = (n: number) => String(n).padStart(2, "0");

// The page looks like paper in the preview; print and PDF keep the margins
// of the page setup.
const STYLE =
  "body { font-family: 'Inter', 'Segoe UI', sans-serif; color: #141821; font-size: 10pt; }" +
  "@media screen { html { background: #ffffff; } body { margin: 28px 34px; } }";

export function reportHtml(state: ReportState, extras: ReportExtras): string {
  let h = "";
  h +=
    "<html><head><meta charset='utf-8'><style>" +
    STYLE +
    "h1 { font-size: 20pt; margin: 0; color: #141821; }" +
    "h2 { font-size: 12.5pt; margin-top: 18px; margin-bottom: 6px; color: #3b3fb8; }" +
    ".muted { color: #515b6b; }" +
    ".code { font-family: 'JetBrains Mono', Consolas, monospace; font-weight: bold; }" +
    "table { border-collapse: collapse; width: 100%; }" +
    "th { text-align: left; background: #eef0f6; color: #515b6b; padding: 5px 6px; " +
    "font-size: 9pt; }" +
    "td { padding: 5px 6px; border-bottom: 1px solid #e2e5ee; vertical-align: top; }" +
    "</style></head><body>";

  // Header
  const now = new Date();
  h += "<table><tr><td style='border:none'>";
  h += "<div style='color:#5a54f0; font-weight:bold; letter-spacing:2px; font-size:9pt'>CARUSPEX</div>";
  h += `<h1>${esc(tr("Prüfbericht"))}</h1>`;
  h += `<div class='muted'>${esc(`${pad(now.getDate())}.${pad(now.getMonth() + 1)}.${now.getFullYear()}, ${pad(now.getHours())}:${pad(now.getMinutes())}`)}</div>`;
  h += "</td><td style='border:none; text-align:right; width:30%'>";
  if (extras.score >= 0) {
    h +=
      `<div style='font-size:30pt; font-weight:bold; color:${scoreColour(extras.score)}'>${extras.score}</div>` +
      `<div class='muted'>${esc(extras.scoreCaption)}</div>`;
  }
  h += "</td></tr></table>";

  // Vehicle
  h += `<h2>${esc(tr("Fahrzeug"))}</h2><table>`;
  const row = (k: string, v: string) => {
    if (!v) return;
    h += `<tr><td style='width:32%' class='muted'>${esc(k)}</td><td>${esc(v)}</td></tr>`;
  };
  row(tr("Bezeichnung"), state.displayName);
  row(tr("Beschreibung"), state.description);
  row(tr("Fahrgestellnummer"), state.vin);
  row(tr("Steuergerät"), state.ecuName);
  row(tr("Kalibrierung"), state.calibration);
  if (state.odometerKm !== null) row(tr("Kilometerstand"), `${state.odometerKm.toFixed(0)} km`);
  row(tr("Adapter"), state.adapter);
  row(tr("Verbindung"), state.transport);
  row(tr("Protokoll"), state.protocol);
  h += "</table>";

  // Findings
  if (extras.findings.length > 0) {
    h += `<h2>${esc(tr("Befunde"))}</h2><table><tr><th>${esc(tr("Stufe"))}</th><th>${esc(tr("Befund"))}</th></tr>`;
    for (const f of extras.findings) {
      h += `<tr><td style='width:16%'>${esc(f.severity)}</td><td><b>${esc(f.title)}</b><br/><span class='muted'>${esc(f.text)}</span></td></tr>`;
    }
    h += "</table>";
  }

  // Faults
  h += `<h2>${esc(tr("Fehlerspeicher"))}</h2>`;
  if (!state.haveFaults) {
    h += `<p class='muted'>${esc(tr("Nicht ausgelesen."))}</p>`;
  } else if (state.faults.length === 0) {
    h += `<p>${esc(tr("Kein Fehler gespeichert."))}</p>`;
  } else {
    h += `<table><tr><th>${esc(tr("Code"))}</th><th>${esc(tr("Art"))}</th><th>${esc(tr("Bedeutung"))}</th><th>${esc(tr("Wo"))}</th></tr>`;
    for (const f of state.faults) {
      h +=
        `<tr><td class='code'>${esc(f.code)}</td><td>${esc(f.kind)}</td><td>${esc(f.text)}</td>` +
        `<td class='muted'>${esc(state.faultZones[f.code] ?? "")}</td></tr>`;
    }
    h += "</table>";
  }
  if (state.haveStatus) {
    h += `<p>${esc(tr("Motorkontrollleuchte"))}: <b>${esc(state.mil ? tr("an") : tr("aus"))}</b></p>`;
  }

  // All control units over UDS
  if (extras.ecus.length > 0) {
    h += `<h2>${esc(tr("Alle Steuergeräte"))}</h2><table><tr><th>${esc(tr("Steuergerät"))}</th><th>${esc(tr("Teilenummer"))}</th><th>${esc(tr("Fehler"))}</th></tr>`;
    for (const e of extras.ecus) {
      const codes = e.dtcs.map((d) => `<span class='code'>${esc(d.code)}</span> ${esc(d.text)}`);
      h += `<tr><td>${esc(e.name)}</td><td class='muted'>${esc(e.partNumber)}</td><td>${codes.length === 0 ? esc(tr("keine")) : codes.join("<br/>")}</td></tr>`;
    }
    h += "</table>";
  }

  // Readiness
  if (state.readiness.length > 0) {
    h += `<h2>${esc(tr("Bereitschaftstests"))}</h2><table><tr><th>${esc(tr("Test"))}</th><th>${esc(tr("Stand"))}</th></tr>`;
    for (const r of state.readiness) {
      h += `<tr><td>${esc(r.name)}</td><td style='color:${r.ready ? "#12875a" : "#b97a06"}'>${esc(r.ready ? tr("abgeschlossen") : tr("offen"))}</td></tr>`;
    }
    h += "</table>";
  }

  // Freeze frame
  if (state.freezeFrame.length > 0) {
    h += `<h2>${esc(tr("Umgebungsdaten zum Fehler"))}</h2>`;
    if (state.freezeCode) {
      h += `<p class='muted'>${esc(tr("Ausgelöst durch"))} <span class='code'>${esc(state.freezeCode)}</span></p>`;
    }
    h += "<table>";
    for (const r of state.freezeFrame) {
      const value = r.text ? r.text : `${r.value.toFixed(1)} ${r.unit}`;
      h += `<tr><td style='width:50%'>${esc(r.name)}</td><td>${esc(value)}</td></tr>`;
    }
    h += "</table>";
  }

  // Component monitoring
  if (state.monitors.length > 0) {
    h +=
      `<h2>${esc(tr("Bauteilprüfung"))}</h2><table><tr><th>${esc(tr("Prüfung"))}</th><th>${esc(tr("Wert"))}</th>` +
      `<th>${esc(tr("Grenzen"))}</th><th>${esc(tr("Ergebnis"))}</th></tr>`;
    for (const m of state.monitors) {
      let limits = "";
      if (m.hasMinimum) limits += `≥ ${g5(m.minimum)}`;
      if (m.hasMaximum) {
        if (limits) limits += "  ";
        limits += `≤ ${g5(m.maximum)}`;
      }
      h +=
        `<tr><td>${esc(m.name)}<br/><span class='muted'>${esc(m.component)}</span></td><td>${g5(m.value)} ${esc(m.unit)}</td>` +
        `<td>${esc(limits)}</td><td style='color:${m.passed ? "#12875a" : "#c93636"}'>${esc(m.passed ? tr("bestanden") : tr("nicht bestanden"))}</td></tr>`;
    }
    h += "</table>";
  }

  h += `<p class='muted' style='margin-top:22px; font-size:8.5pt'>${esc(
    tr("Erstellt mit Caruspex, freie Fahrzeugdiagnose. Der Bericht gibt wieder, was die Steuergeräte gemeldet haben. Er ersetzt keine Prüfung in der Werkstatt."),
  )}</p>`;
  h += "</body></html>";
  return h;
}

// QLocale::ShortFormat for a date, and for a date with time.
function shortDate(d: Date): string {
  return new Intl.DateTimeFormat(locale(), { dateStyle: "short" }).format(d);
}
function shortDateTime(d: Date): string {
  return new Intl.DateTimeFormat(locale(), { dateStyle: "short", timeStyle: "short" }).format(d);
}

// The file of one garage car: checks, service plan, trips and fuel log.
// Works without a connection.
export function garageHtml(car: GarageCar): string {
  let h =
    "<html><head><meta charset='utf-8'><style>" +
    STYLE +
    "h1 { font-size: 20pt; margin: 0; } h2 { font-size: 12.5pt; margin-top: 18px; color: #3b3fb8; }" +
    ".muted { color: #515b6b; } .code { font-family: 'JetBrains Mono', Consolas, monospace; font-weight: bold; }" +
    "table { border-collapse: collapse; width: 100%; }" +
    "th { text-align: left; background: #eef0f6; color: #515b6b; padding: 5px 6px; font-size: 9pt; }" +
    "td { padding: 5px 6px; border-bottom: 1px solid #e2e5ee; vertical-align: top; }" +
    "</style></head><body>";
  h += "<div style='color:#5a54f0; font-weight:bold; letter-spacing:2px; font-size:9pt'>CARUSPEX</div>";
  h += `<h1>${esc(tr("Fahrzeugakte"))}</h1>`;
  h += `<p><b>${esc(car.name)}</b><br/><span class='muted'>${esc(car.description)}<br/>VIN ${esc(car.vin)}</span></p>`;

  let km = -1;
  for (const c of car.checks) km = Math.max(km, c.odometerKm);
  for (const f of car.fuel) km = Math.max(km, f.odometerKm);
  const services = serviceStatus(car, km);
  if (services.length > 0) {
    h += `<h2>${esc(tr("Wartung"))}</h2><table><tr><th>${esc(tr("Was"))}</th><th>${esc(tr("Zuletzt"))}</th><th>${esc(tr("Stand"))}</th></tr>`;
    for (const st of services) {
      const last: string[] = [];
      const lastDate = parseIso(st.service.lastDate);
      if (lastDate) last.push(shortDate(lastDate));
      if (st.service.lastKm >= 0) last.push(`${localeNumber(st.service.lastKm, 0)} km`);
      h += `<tr><td>${esc(st.service.title)}</td><td class='muted'>${esc(last.join(", "))}</td><td>${esc(st.text)}</td></tr>`;
    }
    h += "</table>";
  }

  for (const j of odometerJumps(car)) {
    const earlier = parseIso(j.earlier);
    const later = parseIso(j.later);
    const detail = `${localeNumber(j.earlierKm, 0)} km (${earlier ? shortDate(earlier) : ""}), ${localeNumber(j.laterKm, 0)} km (${later ? shortDate(later) : ""})`;
    h += `<p style='color:#c93636'><b>${esc(tr("Kilometerstand gesunken:"))}</b> ${esc(detail)}</p>`;
  }

  h += `<h2>${esc(tr("Prüfungen"))}</h2>`;
  if (car.checks.length === 0) {
    h += `<p class='muted'>${esc(tr("Noch keine."))}</p>`;
  } else {
    h += `<table><tr><th>${esc(tr("Datum"))}</th><th>${esc(tr("Kilometer"))}</th><th>${esc(tr("Punkte"))}</th><th>${esc(tr("Fehler"))}</th></tr>`;
    for (let i = car.checks.length - 1; i >= 0; i--) {
      const c = car.checks[i]!;
      const codes = c.codes.map((code, k) => `<span class='code'>${esc(code)}</span> ${k < c.texts.length ? esc(c.texts[k]!) : ""}`);
      const when = parseIso(c.when);
      h +=
        `<tr><td>${esc(when ? shortDateTime(when) : "")}</td>` +
        `<td>${c.odometerKm >= 0 ? localeNumber(c.odometerKm, 0) : "-"}</td>` +
        `<td>${c.score >= 0 ? String(c.score) : "-"}</td>` +
        `<td>${codes.length === 0 ? esc(tr("keine")) : codes.join("<br/>")}</td></tr>`;
    }
    h += "</table>";
  }

  if (car.trips.length > 0) {
    let distance = 0;
    let fuel = 0;
    let cost = 0;
    for (const trip of car.trips) {
      distance += trip.distanceKm;
      fuel += trip.fuelLitres;
      cost += trip.cost;
    }
    const summary = tr(
      "%1 Fahrten, %2 km, im Schnitt %3 l/100 km, %4 €",
      car.trips.length,
      localeNumber(distance, 0),
      distance > 1 ? localeNumber((fuel / distance) * 100, 1) : "-",
      localeNumber(cost, 2),
    );
    h += `<h2>${esc(tr("Fahrten"))}</h2><p>${esc(summary)}</p>`;
  }
  if (car.fuel.length > 0) {
    const logged = fuelLogConsumption(car);
    const summary =
      logged > 0
        ? tr("%1 Tankfüllungen, Verbrauch %2 l/100 km", car.fuel.length, localeNumber(logged, 1))
        : tr("%1 Tankfüllungen", car.fuel.length);
    h += `<h2>${esc(tr("Tankbuch"))}</h2><p>${esc(summary)}</p>`;
  }
  if (car.note) {
    const note = esc(car.note).replace(/\n/g, "<br/>");
    h += `<h2>${esc(tr("Notiz"))}</h2><p>${note}</p>`;
  }
  h += `<p class='muted' style='margin-top:22px; font-size:8.5pt'>${esc(tr("Erstellt mit Caruspex aus der Garage auf diesem Rechner."))}</p>`;
  h += "</body></html>";
  return h;
}

// --- The used car check ---------------------------------------------------------------------

const UC = "caruspex::UsedCarPage";
const uc = (de: string, ...args: unknown[]) => t(UC, de, ...args);
const HP = "caruspex::HealthPage";

export const USED_CAR_SECTIONS: UsedCarSection[] = ["identity", "mileage", "clearing", "readiness", "faults", "health", "battery"];

// Section headings of the check, shared with the page.
export function usedCarSectionTitle(section: UsedCarSection): string {
  switch (section) {
    case "identity": return uc("Fahrzeug und Fahrgestellnummer");
    case "mileage": return tr("Kilometerstand");
    case "clearing": return uc("Zuletzt gelöscht");
    case "readiness": return tr("Bereitschaftstests");
    case "faults": return tr("Fehlerspeicher");
    case "health": return t("caruspex::MainWindow", "Gesundheitscheck");
    case "battery": return uc("Akku");
  }
}

export function usedCarLevelLabel(level: number): string {
  switch (level) {
    case 0: return t(HP, "In Ordnung");
    case 1: return t(HP, "Hinweis");
    case 2: return t(HP, "Bald ansehen");
    default: return t(HP, "Dringend");
  }
}

const LIGHT_COLOURS = ["#12875a", "#b97a06", "#c93636"];
const LEVEL_COLOURS = ["#12875a", "#3b6fd8", "#b97a06", "#c93636"];

// The document for the purchase: what the car said at the viewing, the
// questions for the seller and room for both signatures.
export function usedCarHtml(row: UsedCarRow, when: Date): string {
  let h =
    "<html><head><meta charset='utf-8'><style>" +
    STYLE +
    "h1 { font-size: 20pt; margin: 0; } h2 { font-size: 12.5pt; margin-top: 18px; margin-bottom: 6px; color: #3b3fb8; }" +
    ".muted { color: #515b6b; }" +
    "table { border-collapse: collapse; width: 100%; }" +
    "th { text-align: left; background: #eef0f6; color: #515b6b; padding: 5px 6px; font-size: 9pt; }" +
    "td { padding: 5px 6px; border-bottom: 1px solid #e2e5ee; vertical-align: top; }" +
    "ol { margin: 4px 0 0 0; padding-left: 22px; } li { margin: 3px 0; }" +
    "tr { page-break-inside: avoid; }" +
    ".sign td { border: none; padding-top: 38px; width: 50%; } .line { border-top: 1px solid #141821; padding-top: 4px; }" +
    "</style></head><body>";

  const colour = LIGHT_COLOURS[row.light] ?? "#141821";
  h += "<table><tr><td style='border:none'>";
  h += "<div style='color:#5a54f0; font-weight:bold; letter-spacing:2px; font-size:9pt'>CARUSPEX</div>";
  h += `<h1>${esc(uc("Gebrauchtwagen-Check"))}</h1>`;
  h += `<div class='muted'>${esc(`${pad(when.getDate())}.${pad(when.getMonth() + 1)}.${when.getFullYear()}, ${pad(when.getHours())}:${pad(when.getMinutes())}`)}</div>`;
  h += "</td><td style='border:none; text-align:right; width:42%'>";
  h += `<div style='display:inline-block; padding:8px 14px; border-radius:8px; border:2px solid ${colour}; color:${colour}; font-weight:bold; font-size:12pt'>${esc(row.headline)}</div>`;
  h += "</td></tr></table>";
  h += `<p>${esc(row.summary)}</p>`;

  // The facts in one table.
  h += `<h2>${esc(tr("Fahrzeug"))}</h2><table>`;
  const fact = (k: string, v: string) => {
    if (!v) return;
    h += `<tr><td style='width:38%' class='muted'>${esc(k)}</td><td>${esc(v)}</td></tr>`;
  };
  fact(tr("Bezeichnung"), row.vehicle);
  if (row.modelYear > 0) fact(uc("Modelljahr"), String(row.modelYear));
  fact(uc("Fahrgestellnummer im Steuergerät"), row.vin || uc("nicht gemeldet"));
  fact(uc("Fahrgestellnummer in den Papieren"), row.papersVin || uc("nicht eingegeben"));
  // The highest reading first, then what each control unit stores.
  if (row.mileage.length > 0) fact(tr("Kilometerstand"), `${localeNumber(Math.max(...row.mileage.map((m) => m.km)), 0)} km`);
  if (row.mileage.length > 1) {
    for (const m of row.mileage) fact(`   ${m.source}`, `${localeNumber(m.km, 0)} km`);
  }
  const since: string[] = [];
  if (row.distanceSinceClearKm >= 0) since.push(`${localeNumber(row.distanceSinceClearKm, 0)} km`);
  if (row.warmUps >= 0) since.push(uc("%1 Warmläufe", localeNumber(row.warmUps, 0)));
  if (row.minutesSinceClear >= 0) since.push(uc("%1 h Betrieb", localeNumber(row.minutesSinceClear / 60, 0)));
  fact(uc("Seit dem letzten Löschen"), since.join(", "));
  if (row.readinessTotal >= 0) fact(tr("Bereitschaftstests"), uc("%1 von %2 abgeschlossen", row.readinessComplete, row.readinessTotal));
  if (row.storedFaults >= 0) {
    fact(tr("Fehlerspeicher"), uc("%1 gespeichert, %2 vorläufig, %3 dauerhaft", row.storedFaults, row.pendingFaults, row.permanentFaults));
  }
  if (row.healthScore >= 0) fact(t("caruspex::MainWindow", "Gesundheitscheck"), uc("%1 von 100 Punkten", row.healthScore));
  if (row.batterySoh >= 0) fact(t("caruspex::EvPage", "Akkugesundheit"), `${localeNumber(row.batterySoh, 0)} %`);
  h += "</table>";

  if (row.questions.length > 0) {
    h += `<h2>${esc(uc("Fragen an den Verkäufer"))}</h2><ol>`;
    for (const q of row.questions) h += `<li>${esc(q)}</li>`;
    h += "</ol>";
  }

  h += `<h2>${esc(tr("Befunde"))}</h2><table><tr><th>${esc(tr("Stufe"))}</th><th>${esc(tr("Befund"))}</th></tr>`;
  for (const section of USED_CAR_SECTIONS) {
    const items = row.items.filter((i) => i.section === section);
    if (items.length === 0) continue;
    h += `<tr><td colspan='2' style='background:#f6f7fb; font-weight:bold'>${esc(usedCarSectionTitle(section))}</td></tr>`;
    for (const i of items) {
      h +=
        `<tr><td style='width:16%; color:${LEVEL_COLOURS[i.level] ?? "#141821"}'>${esc(usedCarLevelLabel(i.level))}</td>` +
        `<td><b>${esc(i.title)}</b><br/><span class='muted'>${esc(i.explanation)}</span></td></tr>`;
    }
  }
  h += "</table>";

  if (row.notChecked.length > 0) {
    h += `<h2>${esc(t(HP, "Nicht geprüft"))}</h2>`;
    for (const n of row.notChecked) h += `<p class='muted' style='margin:3px 0'>${esc(n)}</p>`;
  }

  h += `<p style='margin-top:20px'>${esc(uc("Ausgelesen bei der Besichtigung, im Beisein von Käufer und Verkäufer."))}</p>`;
  h +=
    "<table class='sign'><tr>" +
    `<td><div class='line'>${esc(uc("Ort, Datum, Käufer"))}</div></td>` +
    `<td><div class='line'>${esc(uc("Ort, Datum, Verkäufer"))}</div></td>` +
    "</tr></table>";
  h += `<p class='muted' style='margin-top:22px; font-size:8.5pt'>${esc(
    tr("Erstellt mit Caruspex, freie Fahrzeugdiagnose. Der Bericht gibt wieder, was die Steuergeräte gemeldet haben. Er ersetzt keine Prüfung in der Werkstatt."),
  )}</p>`;
  h += "</body></html>";
  return h;
}
