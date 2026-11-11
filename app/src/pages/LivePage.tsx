// Live values (LivePage.cpp): everything the control unit measures, grouped
// by area, with the manufacturer values of the built-in packs and the user's
// own Torque CSV values below. Tick values on the left, start at the top;
// the chart shows every curve as a share of its own range, the table the
// current readings, and the recording can be saved as CSV.
import { useEffect, useMemo, useRef, useState } from "react";
import { call } from "../api";
import type { CustomRow, LiveRow, PidInfo, PidPackInfo } from "../api";
import { SeriesStore, TimeSeriesChart } from "../components/charts/TimeSeriesChart";
import { Modal } from "../components/Modal";
import { toast } from "../components/Toast";
import { Button, Card, Check, Page, PageHeader, TextInput, cx } from "../components/ui";
import { translator } from "../i18n";
import { useAppEvent, useBackendEvent, usePageActive } from "../lib/hooks";
import { navigate } from "../shell/nav";
import { useSession, useSettings } from "../state/session";
import { qtFilters, stamp } from "./measure";
import "./LivePage.css";

const tr = translator("caruspex::LivePage");

interface ListItem {
  kind: "head" | "pid" | "custom";
  key: number;
  label: string;
  tooltip?: string;
}

interface Sample {
  seconds: number;
  pid: number;
  name: string;
  value: number;
  unit: string;
}

export default function LivePage() {
  const connected = useSession((s) => s.connected);
  const settings = useSettings();
  const active = usePageActive();

  const [table, setTable] = useState<PidInfo[]>([]);
  const [packs, setPacks] = useState<PidPackInfo[]>([]);
  const [imports, setImports] = useState<CustomRow[][]>([]);
  const [checked, setChecked] = useState<Set<number>>(new Set());
  const [supported, setSupported] = useState<Set<number>>(new Set());
  const [search, setSearch] = useState("");
  const [running, setRunningState] = useState(false);
  const [record, setRecord] = useState(true);
  const [canExport, setCanExport] = useState(false);
  const [nothingChosen, setNothingChosen] = useState(false);

  const store = useRef(new SeriesStore()).current;
  const rows = useRef(new Map<number, LiveRow>());
  const [rowsVersion, setRowsVersion] = useState(0);
  const rowsDirty = useRef(false);
  const recorded = useRef<Sample[]>([]);
  const clockStart = useRef<number | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const runningRef = useRef(false);
  const connectedRef = useRef(connected);
  connectedRef.current = connected;
  const recordRef = useRef(record);
  recordRef.current = record;

  // The table and the packs come from the core, the defaults are ticked.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const [pids, defaults, packList] = await Promise.all([call("pid_table"), call("default_live_pids"), call("pid_packs")]);
      if (cancelled) return;
      setTable(pids);
      setPacks(packList);
      setChecked(new Set(defaults));
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    store.setWindow(settings.chartWindowSeconds);
  }, [settings.chartWindowSeconds, store]);

  const setRunning = (value: boolean) => {
    runningRef.current = value;
    setRunningState(value);
  };

  // setConnected(false): stop, empty the table.
  useEffect(() => {
    if (connected) return;
    setRunning(false);
    rows.current.clear();
    setRowsVersion((v) => v + 1);
  }, [connected]);

  // One entry per PID, grouped by category; then one section per pack and
  // one per imported file.
  const items = useMemo<ListItem[]>(() => {
    const out: ListItem[] = [];
    const byCategory = new Map<number, { name: string; pids: Map<number, PidInfo[]> }>();
    for (const info of table) {
      if (info.pid === 0x00 || info.key >= 0x100000) continue;
      let cat = byCategory.get(info.category);
      if (!cat) {
        cat = { name: info.categoryName, pids: new Map() };
        byCategory.set(info.category, cat);
      }
      const list = cat.pids.get(info.pid) ?? [];
      list.push(info);
      cat.pids.set(info.pid, list);
    }
    for (const category of [...byCategory.keys()].sort((a, b) => a - b)) {
      const cat = byCategory.get(category)!;
      out.push({ kind: "head", key: -1, label: cat.name.toUpperCase() });
      for (const [pid, values] of cat.pids) {
        const first = values.find((v) => v.sub === 0) ?? values[0]!;
        const label = values.find((v) => v.title)?.title ?? first.name;
        out.push({ kind: "pid", key: pid, label, tooltip: values.length > 1 ? tr.n("%n Werte", values.length) : undefined });
      }
    }
    for (const pack of packs) {
      out.push({ kind: "head", key: -1, label: pack.title.toUpperCase() });
      for (const v of pack.values) {
        out.push({ kind: "custom", key: v.key, label: v.name, tooltip: v.source ? tr("Quelle: %1", v.source) : undefined });
      }
    }
    for (const batch of imports) {
      out.push({ kind: "head", key: -1, label: tr("Eigene Werte").toUpperCase() });
      for (const r of batch) {
        out.push({ kind: "custom", key: r.key, label: r.name, tooltip: `${r.header}  ${r.request}`.trim() || undefined });
      }
    }
    return out;
  }, [table, packs, imports]);

  const unsupported = (item: ListItem) => item.kind === "pid" && supported.size > 0 && !supported.has(item.key);

  // Whatever the vehicle does not support is greyed out and deselected.
  useBackendEvent("supported_pids", ({ pids }) => {
    if (pids.length === 0) return;
    const set = new Set(pids);
    setSupported(set);
    setChecked((prev) => {
      const next = new Set(prev);
      for (const item of items) if (item.kind === "pid" && !set.has(item.key)) next.delete(item.key);
      return next;
    });
  });

  useBackendEvent("custom_imported", ({ rows: imported, errors }) => {
    if (imported.length > 0) {
      setImports((prev) => [...prev, imported]);
      setChecked((prev) => {
        const next = new Set(prev);
        for (const r of imported) next.add(r.key);
        return next;
      });
      requestAnimationFrame(() => {
        const list = listRef.current;
        if (list) list.scrollTop = list.scrollHeight;
      });
    }
    let text = tr.n("%n Wert(e) geladen.", imported.length);
    if (errors.length > 0) text += ` ${tr.n("%n Zeile(n) übersprungen: %1", errors.length, errors.slice(0, 3).join("; "))}`;
    toast(text, errors.length === 0 ? "success" : "warning", 7000);
  });

  useBackendEvent("live_value", ({ row }) => {
    rows.current.set(row.pid, row);
    rowsDirty.current = true;
    if (row.text) return; // status values do not belong in the chart
    if (clockStart.current === null) return;
    const seconds = (performance.now() - clockStart.current) / 1000;
    if (recordRef.current) {
      recorded.current.push({ seconds, pid: row.pid, name: row.name, value: row.value, unit: row.unit });
      if (recorded.current.length === 1) setCanExport(true);
    }
    store.add(row.pid, seconds, row.value, row);
  });

  // The table follows the values a few times a second while on screen.
  useEffect(() => {
    if (!active) return;
    setRowsVersion((v) => v + 1);
    const id = setInterval(() => {
      if (!rowsDirty.current) return;
      rowsDirty.current = false;
      setRowsVersion((v) => v + 1);
    }, 250);
    return () => clearInterval(id);
  }, [active]);

  const selected = (kind: "pid" | "custom") =>
    items.filter((i) => i.kind === kind && checked.has(i.key) && !unsupported(i)).map((i) => i.key);

  const toggleRun = () => {
    if (runningRef.current) {
      void call("stop_live");
      setRunning(false);
      return;
    }
    const pids = selected("pid");
    const custom = selected("custom");
    if (pids.length === 0 && custom.length === 0) {
      setNothingChosen(true);
      return;
    }
    store.clear();
    store.setWindow(settings.chartWindowSeconds);
    clockStart.current = performance.now();
    recorded.current = [];
    setCanExport(false);
    void (async () => {
      await call("start_live", { pids });
      if (custom.length > 0) await call("start_custom_poll", { keys: custom });
    })();
    setRunning(true);
  };

  const toggleFromOutside = () => {
    navigate("live");
    if (connectedRef.current) toggleRun();
  };
  useAppEvent("live:toggle", toggleFromOutside);
  useAppEvent<string>("dev:page", (name) => {
    if (name === "live-run") toggleFromOutside();
  });

  const importValues = async () => {
    const path = await call("pick_open_path", {
      title: tr("Eigene Werte laden"),
      filters: qtFilters(tr("Torque-CSV (*.csv);;Alle Dateien (*)")),
    });
    if (path) await call("import_custom_csv", { path });
  };

  const exportCsv = async () => {
    if (recorded.current.length === 0) return;
    const lines = [tr("Sekunde;PID;Messwert;Wert;Einheit")];
    for (const s of recorded.current) {
      const pid = `0x${s.pid.toString(16).padStart(2, "0")}`.toUpperCase();
      lines.push(`${s.seconds.toFixed(2)};${pid};${s.name};${s.value.toFixed(3)};${s.unit}`);
    }
    try {
      await call("save_text_file", {
        title: tr("Aufzeichnung speichern"),
        suggestedName: tr("messung_%1.csv", stamp()),
        filters: qtFilters(tr("CSV (*.csv)")),
        content: `${lines.join("\n")}\n`,
      });
    } catch {
      toast(`${tr("Fehler")}: ${tr("Datei lässt sich nicht schreiben.")}`, "error", 6000);
    }
  };

  const needle = search.trim().toLowerCase();
  // rowsVersion changes when the readings did; the map keeps arrival order.
  void rowsVersion;
  const tableRows = [...rows.current.values()];

  return (
    <Page className="live-page">
      <PageHeader
        eyebrow={tr("Messen")}
        title={tr("Live-Werte")}
        subtitle={tr("Alles, was das Steuergerät laufend misst, nach Bereichen sortiert. Links ankreuzen, oben starten.")}
      />

      <div className="live-bar">
        <Button variant={running ? "danger" : "primary"} icon={running ? "stop" : "play"} disabled={!connected} onClick={toggleRun}>
          {running ? tr("Messung stoppen") : tr("Messung starten")}
        </Button>
        <Button icon="file" title={tr("Messwerte aus einer Torque-CSV-Datei hinzufügen")} onClick={() => void importValues()}>
          {tr("Eigene Werte laden")}
        </Button>
        <Button icon="download" disabled={!canExport} onClick={() => void exportCsv()}>
          {tr("Aufzeichnung speichern")}
        </Button>
        <Check checked={record} onChange={setRecord}>
          {tr("mitschreiben")}
        </Check>
        <span className="spacer" />
        <span className="hint live-hint">{tr("Die Werte laufen einzeln über den Bus, daher aktualisiert es auf der K-Line gemächlich.")}</span>
      </div>

      <Card className="live-chart-card">
        <TimeSeriesChart store={store} xTitle={tr("Sekunden")} yTitle={tr("% Bereich")} height={290} />
      </Card>

      <div className="live-split">
        <div className="live-list-box">
          <TextInput
            value={search}
            onChange={setSearch}
            placeholder={tr("Messwert suchen, z.B. Öl oder Lambda")}
            type="search"
            aria-label={tr("Messwert suchen, z.B. Öl oder Lambda")}
          />
          <div className="live-list card flat" ref={listRef} role="list">
            {items.map((item, i) => {
              if (item.kind === "head") {
                if (needle) return null;
                return (
                  <div className="live-list-head" key={`h${i}`}>
                    {item.label}
                  </div>
                );
              }
              if (needle && !item.label.toLowerCase().includes(needle)) return null;
              const off = unsupported(item);
              return (
                <label
                  key={`${item.kind}${item.key}`}
                  className={cx("live-list-item", off && "off")}
                  title={off ? tr("Dieses Fahrzeug liefert den Wert nicht.") : item.tooltip}
                  role="listitem"
                >
                  <input
                    type="checkbox"
                    checked={checked.has(item.key) && !off}
                    disabled={off}
                    onChange={(e) =>
                      setChecked((prev) => {
                        const next = new Set(prev);
                        if (e.target.checked) next.add(item.key);
                        else next.delete(item.key);
                        return next;
                      })
                    }
                  />
                  <span>{item.label}</span>
                </label>
              );
            })}
          </div>
        </div>

        <div className="table-wrap live-table">
          <table className="table">
            <thead>
              <tr>
                <th>{tr("Messwert")}</th>
                <th className="num">{tr("Wert")}</th>
                <th>{tr("Einheit")}</th>
              </tr>
            </thead>
            <tbody>
              {tableRows.map((row) => (
                <tr key={row.pid} className="fade-in">
                  <td>{row.name}</td>
                  <td className={cx("num", row.warn && "warn")} title={row.text || undefined}>
                    <span className="ltr live-value">{row.text ? row.text : row.value.toFixed(2)}</span>
                  </td>
                  <td className="muted">{row.unit}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {nothingChosen ? (
        <Modal
          small
          title={tr("Nichts ausgewählt")}
          icon="info"
          onClose={() => setNothingChosen(false)}
          footer={
            <Button variant="primary" onClick={() => setNothingChosen(false)}>
              OK
            </Button>
          }
        >
          {tr("Links mindestens einen Messwert ankreuzen.")}
        </Modal>
      ) : null}
    </Page>
  );
}
