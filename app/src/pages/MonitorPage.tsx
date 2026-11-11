// Modes 06 and 05 (MonitorPage.cpp): what the control unit continuously
// checks by itself. Instead of just pass or fail it reports the measured
// value with its limits, so a component close to its limit shows long
// before the warning lamp comes on.
import { useEffect, useRef, useState } from "react";
import { call, type MonitorRow, type OxygenRow } from "../api";
import { Icon } from "../components/Icon";
import { toast } from "../components/Toast";
import { Button, Page, PageHeader, cx } from "../components/ui";
import { fixed, translator } from "../i18n";
import { useBackendEvent } from "../lib/hooks";
import { useSession } from "../state/session";
import "./MonitorPage.css";

const tr = translator("caruspex::MonitorPage");

interface Identity {
  vin: string;
  calibration: string;
  checksum: string;
  ecuName: string;
}

const pad = (n: number) => String(n).padStart(2, "0");

// QDateTime::toString with the tokens the translations use (dd, MM, yyyy,
// HH, mm, ss and their short forms).
function formatDate(pattern: string, d: Date): string {
  const tokens: Record<string, string> = {
    yyyy: String(d.getFullYear()),
    yy: pad(d.getFullYear() % 100),
    MM: pad(d.getMonth() + 1),
    M: String(d.getMonth() + 1),
    dd: pad(d.getDate()),
    d: String(d.getDate()),
    HH: pad(d.getHours()),
    H: String(d.getHours()),
    hh: pad(d.getHours()),
    h: String(d.getHours()),
    mm: pad(d.getMinutes()),
    m: String(d.getMinutes()),
    ss: pad(d.getSeconds()),
    s: String(d.getSeconds()),
  };
  return pattern.replace(/yyyy|yy|MM|M|dd|d|HH|H|hh|h|mm|m|ss|s/g, (k) => tokens[k] ?? k);
}

function identityParts(id: Identity | null): string[] | null {
  if (!id) return null;
  const parts: string[] = [];
  if (id.vin) parts.push(tr("Fahrgestellnummer: %1", id.vin));
  if (id.calibration) parts.push(tr("Kalibrierstand: %1", id.calibration));
  if (id.checksum) parts.push(tr("Prüfsumme: %1", id.checksum));
  if (id.ecuName) parts.push(tr("Steuergerät: %1", id.ecuName));
  return parts;
}

const valueText = (value: number, unit: string) => `${fixed(value, 3)} ${unit}`.trim();

export default function MonitorPage() {
  const connected = useSession((s) => s.connected);
  const [status, setStatus] = useState(() => tr("Noch nichts ausgelesen."));
  const [identity, setIdentity] = useState<Identity | null>(null);
  // What the export writes: the last results of each kind.
  const [monitors, setMonitors] = useState<MonitorRow[]>([]);
  const [oxygen, setOxygen] = useState<OxygenRow[]>([]);
  // What the tree shows: a new mode 06 answer clears it, mode 05 adds its group.
  const [treeMonitors, setTreeMonitors] = useState<MonitorRow[] | null>(null);
  const [treeOxygen, setTreeOxygen] = useState<OxygenRow[] | null>(null);
  const [exportEnabled, setExportEnabled] = useState(false);
  // The same two lists for the event handlers: mode 06 and mode 05 answers
  // can arrive back to back, before the next render.
  const lists = useRef<{ monitors: MonitorRow[]; oxygen: OxygenRow[] }>({ monitors: [], oxygen: [] });
  const [collapsed, setCollapsed] = useState<{ monitors: boolean; oxygen: boolean }>({ monitors: false, oxygen: false });

  useEffect(() => {
    if (connected) return;
    setTreeMonitors(null);
    setTreeOxygen(null);
    setMonitors([]);
    setOxygen([]);
    lists.current = { monitors: [], oxygen: [] };
    setIdentity(null);
    setExportEnabled(false);
    setStatus(tr("Noch nichts ausgelesen."));
  }, [connected]);

  useBackendEvent("identity", (payload) => setIdentity(payload));

  useBackendEvent("monitor_results", ({ rows, note }) => {
    lists.current.monitors = rows;
    setMonitors(rows);
    if (rows.length === 0) {
      setStatus(note ? note : tr("Keine Tests gemeldet."));
    } else {
      const failed = rows.filter((r) => !r.passed).length;
      setStatus(
        failed === 0 ? tr("%1 Tests, alle im Rahmen.", rows.length) : tr("%1 Tests, %2 außerhalb der Grenzen.", rows.length, failed),
      );
    }
    setTreeMonitors(rows);
    setTreeOxygen(null);
    setCollapsed({ monitors: false, oxygen: false });
    setExportEnabled(rows.length > 0 || lists.current.oxygen.length > 0);
  });

  useBackendEvent("oxygen_tests", ({ rows, note }) => {
    lists.current.oxygen = rows;
    setOxygen(rows);
    if (rows.length === 0) {
      if (lists.current.monitors.length === 0 && note) setStatus(note);
      return;
    }
    setTreeOxygen(rows);
    setExportEnabled(true);
  });

  const parts = identityParts(identity);
  const identityText = parts === null ? "" : parts.length === 0 ? tr("Das Steuergerät nennt keine Kennungen. Bei Fahrzeugen " + "vor etwa 2003 ist das normal.") : parts.join("     ");

  const exportReport = async () => {
    if (monitors.length === 0 && oxygen.length === 0) return;
    const now = new Date();
    const stamp = formatDate("yyyy-MM-dd_HH-mm", now);
    let out = `${tr("Bauteilüberwachung, ausgelesen mit Caruspex")}\n`;
    out += `${formatDate(tr("dd.MM.yyyy HH:mm"), now)}\n`;
    out += `${identityText}\n`;
    out += `${"=".repeat(78)}\n\n`;
    if (monitors.length > 0) {
      out += `${tr("Bauteilüberwachung (Modus 06)")}\n\n`;
      for (const r of monitors) {
        out += `${r.passed ? "  [ok]   " : "  [!!]   "}${r.name}\n`;
        out += `          ${r.component}\n`;
        out += `          ${tr("Messwert %1 %2", fixed(r.value, 3), r.unit)}`;
        if (r.hasMinimum) out += tr(", Untergrenze %1", fixed(r.minimum, 3));
        if (r.hasMaximum) out += tr(", Obergrenze %1", fixed(r.maximum, 3));
        out += "\n\n";
      }
    }
    if (oxygen.length > 0) {
      out += `${tr("Lambdasondenprüfung (Modus 05)")}\n\n`;
      for (const r of oxygen) {
        out += `  ${tr("Sonde %1", r.sensor)}  ${r.name}: ${fixed(r.value, 3)} ${r.unit}\n`;
      }
    }
    try {
      await call("save_text_file", {
        title: tr("Bericht speichern"),
        suggestedName: tr("bauteilprüfung_%1.txt", stamp),
        filters: [{ name: tr("Textdatei (*.txt)"), extensions: ["txt"] }],
        content: out,
      });
    } catch {
      toast(`${tr("Fehler")}: ${tr("Datei lässt sich nicht schreiben.")}`, "error", 6000);
    }
  };

  const toggle = (group: "monitors" | "oxygen") => setCollapsed((c) => ({ ...c, [group]: !c[group] }));

  const groupRow = (group: "monitors" | "oxygen", title: string) => (
    <tr
      className="monitor-group"
      onClick={() => toggle(group)}
      tabIndex={0}
      aria-expanded={!collapsed[group]}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          toggle(group);
        }
      }}
    >
      <td colSpan={6}>
        <span className="monitor-group-title">
          <Icon name={collapsed[group] ? "chevron-right" : "chevron-down"} size={14} className="monitor-chevron" />
          {title}
        </span>
      </td>
    </tr>
  );

  const empty = treeMonitors === null && treeOxygen === null;

  return (
    <Page className="monitor-page">
      <PageHeader eyebrow={tr("DIAGNOSE")} title={tr("Bauteilüberwachung")} />
      <p className="hint wrap monitor-hint">
        {tr(
          "Das Steuergerät prüft im Betrieb laufend seine eigenen Bauteile. Hier stehen " +
            "die Ergebnisse: nicht nur bestanden oder nicht, sondern der Messwert samt " +
            "erlaubter Unter- und Obergrenze. Damit sieht man, wie knapp ein Bauteil noch " +
            "im Rahmen liegt, lange bevor die Fehlerlampe angeht.\n\n" +
            "Ein Katalysator, dessen Wirkungsgrad hart an der Grenze kratzt, fällt hier " +
            "auf. Im Fehlerspeicher steht dazu noch nichts.",
        )}
      </p>

      <div className="monitor-bar">
        <Button variant="primary" icon="stethoscope" className="monitor-button" disabled={!connected} onClick={() => void call("read_monitors")}>
          {tr("Auslesen")}
        </Button>
        <Button icon="file" className="monitor-button" disabled={!exportEnabled} onClick={() => void exportReport()}>
          {tr("Als Bericht speichern")}
        </Button>
        <span className="spacer" />
        <span className="subtle monitor-status">{status}</span>
      </div>

      {identityText ? (
        <div className="subtle monitor-identity selectable fade-in">
          {parts && parts.length > 0 ? parts.map((p, i) => <span key={i}>{p}</span>) : identityText}
        </div>
      ) : null}

      <div className="table-wrap monitor-table">
        <table className="table">
          <thead>
            <tr>
              <th>{tr("Test")}</th>
              <th>{tr("Baugruppe")}</th>
              <th className="num">{tr("Messwert")}</th>
              <th className="num">{tr("Untergrenze")}</th>
              <th className="num">{tr("Obergrenze")}</th>
              <th>{tr("Bewertung")}</th>
            </tr>
          </thead>
          <tbody>
            {treeMonitors !== null ? groupRow("monitors", tr("Bauteilüberwachung (Modus 06)")) : null}
            {treeMonitors !== null && !collapsed.monitors
              ? treeMonitors.map((r, i) => (
                  <tr key={`m${i}`} className="fade-in monitor-child">
                    <td>{r.name}</td>
                    <td>{r.component}</td>
                    <td className="num ltr">{valueText(r.value, r.unit)}</td>
                    <td className="num ltr">{r.hasMinimum ? fixed(r.minimum, 3) : "-"}</td>
                    <td className="num ltr">{r.hasMaximum ? fixed(r.maximum, 3) : "-"}</td>
                    <td className={cx("monitor-verdict", r.passed ? "good" : "bad")}>{r.passed ? tr("im Rahmen") : tr("ausserhalb")}</td>
                  </tr>
                ))
              : null}
            {treeOxygen !== null ? groupRow("oxygen", tr("Lambdasondenprüfung (Modus 05)")) : null}
            {treeOxygen !== null && !collapsed.oxygen
              ? treeOxygen.map((r, i) => (
                  <tr key={`o${i}`} className="fade-in monitor-child">
                    <td>{r.name}</td>
                    <td>{tr("Sonde %1", r.sensor)}</td>
                    <td className="num ltr">{valueText(r.value, r.unit)}</td>
                    <td />
                    <td />
                    <td />
                  </tr>
                ))
              : null}
          </tbody>
        </table>
        {empty ? <div className="monitor-empty" /> : null}
      </div>
    </Page>
  );
}
