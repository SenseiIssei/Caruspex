// Trucks over SAE J1939: the lamps and identification of every control
// unit (fault page) and the live values the units broadcast (live page).
import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { call, type J1939ValueRow } from "../api";
import { Button, Card, Page, PageHeader, Pill } from "./ui";
import { localeNumber, translator } from "../i18n";
import { useBackendEvent, usePageActive } from "../lib/hooks";
import { useSession } from "../state/session";
import "./J1939.css";

const tr = translator("caruspex::Truck");

// The four lamps of a J1939 unit, in the order of the standard.
function Lamp({ on, tone, label }: { on: boolean; tone: string; label: string }) {
  return (
    <span className={`j1939-lamp${on ? " on" : ""}`} style={{ "--tone": tone } as CSSProperties} title={label}>
      <span className="j1939-lamp-dot" />
      {label}
    </span>
  );
}

// Every control unit of the truck with its lamps, how many faults it has
// and what it calls itself.
export function J1939UnitsCard() {
  const units = useSession((s) => s.j1939Units);
  const ids = useSession((s) => s.j1939Ids);
  if (units.length === 0) return null;
  return (
    <Card className="j1939-units fade-in" title={tr("Steuergeräte am J1939-Bus")} icon="cpu">
      <div className="j1939-unit-list">
        {units.map((u) => {
          const id = ids.find((i) => i.address === u.address);
          return (
            <div key={u.address} className="j1939-unit">
              <div className="j1939-unit-head">
                <span className="strong">{u.label}</span>
                <span className="spacer" />
                {u.active > 0 ? <Pill tone="var(--bad)">{tr("%1 aktiv", u.active)}</Pill> : null}
                {u.previous > 0 ? <Pill tone="var(--warn)">{tr("%1 früher", u.previous)}</Pill> : null}
                {u.active === 0 && u.previous === 0 ? <Pill tone="var(--good)">{tr("ohne Fehler")}</Pill> : null}
              </div>
              <div className="j1939-lamps">
                <Lamp on={u.mil} tone="var(--warn)" label={tr("Abgaslampe (MIL)")} />
                <Lamp on={u.redStop} tone="var(--bad)" label={tr("Rote Stopplampe")} />
                <Lamp on={u.amberWarning} tone="var(--warn)" label={tr("Gelbe Warnlampe")} />
                <Lamp on={u.protect} tone="var(--info)" label={tr("Schutzlampe")} />
              </div>
              {id && (id.make || id.model || id.software.length > 0) ? (
                <div className="subtle j1939-id selectable">
                  {[id.make, id.model, id.serial].filter(Boolean).join(" · ")}
                  {id.software.length > 0 ? `  ·  ${tr("Software %1", id.software.join(", "))}` : ""}
                </div>
              ) : null}
            </div>
          );
        })}
      </div>
      <div className="hint">
        {tr(
          "Rote Stopplampe: anhalten, sobald es sicher geht. Gelbe Warnlampe: bald in die Werkstatt. Die Abgaslampe meldet Fehler, die die Abgase verschlechtern, die Schutzlampe Probleme außerhalb des Motors.",
        )}
      </div>
    </Card>
  );
}

// The live page of a truck: the units broadcast their values all the time,
// so Caruspex only listens. Grouped by sender.
export function J1939LivePage() {
  const connected = useSession((s) => s.connected);
  const active = usePageActive();
  const [running, setRunning] = useState(false);
  const [rows, setRows] = useState<J1939ValueRow[]>([]);
  const runningRef = useRef(false);

  useBackendEvent("j1939_values", ({ rows: next }) => setRows(next));

  const start = () => {
    runningRef.current = true;
    setRunning(true);
    void call("start_live", { pids: [] });
  };
  const stop = () => {
    runningRef.current = false;
    setRunning(false);
    void call("stop_live");
  };

  // Leaving the page or losing the connection stops listening.
  useEffect(() => {
    if ((!active || !connected) && runningRef.current) stop();
  }, [active, connected]);
  useEffect(() => () => {
    if (runningRef.current) void call("stop_live");
  }, []);

  const groups = useMemo(() => {
    const by = new Map<number, { name: string; rows: J1939ValueRow[] }>();
    for (const r of rows) {
      let g = by.get(r.source);
      if (!g) {
        g = { name: r.sourceName, rows: [] };
        by.set(r.source, g);
      }
      g.rows.push(r);
    }
    return [...by.entries()].sort((a, b) => a[0] - b[0]).map(([, g]) => g);
  }, [rows]);

  const shown = (r: J1939ValueRow) => {
    if (r.state === "error") return tr("Sensor meldet Fehler");
    const digits = Math.abs(r.value) >= 1000 || r.unit === "" ? 0 : Math.abs(r.value) >= 100 ? 1 : 2;
    return `${localeNumber(r.value, digits)} ${r.unit}`.trim();
  };

  return (
    <Page className="j1939-live-page">
      <PageHeader
        eyebrow={tr("Lkw")}
        title={tr("Live-Werte (SAE J1939)")}
        subtitle={tr(
          "Die Steuergeräte eines Lkw senden ihre Werte ständig. Caruspex hört mit und zeigt die bekannten, nach Absender sortiert.",
        )}
        actions={
          running ? (
            <Button icon="stop" onClick={stop}>
              {tr("Anhalten")}
            </Button>
          ) : (
            <Button variant="primary" icon="play" disabled={!connected} onClick={start}>
              {tr("Mithören")}
            </Button>
          )
        }
      />
      {rows.length === 0 ? (
        <Card>
          <div className="subtle">
            {connected
              ? tr("Auf Mithören drücken, dann kommen die Werte im Sekundentakt.")
              : tr("Erst mit dem Lkw verbinden.")}
          </div>
        </Card>
      ) : null}
      <div className="j1939-groups">
        {groups.map((g) => (
          <Card key={g.name} title={g.name} icon="cpu">
            <table className="j1939-table">
              <tbody>
                {g.rows.map((r) => (
                  <tr key={`${r.source}-${r.spn}`} title={r.about}>
                    <td className="j1939-name">{r.name}</td>
                    <td className={`j1939-value num${r.state === "error" ? " bad" : ""}`}>{shown(r)}</td>
                    <td className="faint mono ltr">SPN {r.spn}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Card>
        ))}
      </div>
    </Page>
  );
}
