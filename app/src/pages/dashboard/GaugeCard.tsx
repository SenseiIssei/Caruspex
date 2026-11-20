// The live gauges of the start page, with the menu to choose them.
import { useEffect, useRef, useState, type CSSProperties } from "react";
import { call } from "../../api";
import type { LiveRow, PidInfo } from "../../api";
import { Gauge } from "../../components/Gauge";
import { Icon } from "../../components/Icon";
import { Button, Card } from "../../components/ui";
import { translator } from "../../i18n";
import { useAppEvent, useBackendEvent } from "../../lib/hooks";
import { navigate } from "../../shell/nav";
import { live } from "../../state/session";

const tr = translator("caruspex::DashboardPage");

// What can be put on a gauge, in menu order. Names, units and ranges come
// from the PID table, so they follow the language and the standard.
const GAUGE_CHOICES = [0x0c, 0x0d, 0x05, 0x04, 0x11, 0x42, 0x0f, 0x10, 0x0b, 0x2f, 0x5c, 0x5e, 0x0e, 0x06, 0x07, 0x3c, 0x46, 0x49, 0x70, 0xa4];
const DEFAULT_GAUGES = [0x0c, 0x0d, 0x05, 0x04, 0x11, 0x42];
// The QSettings key of the Qt version.
const GAUGE_KEY = "start/anzeigen";

function loadGauges(): number[] {
  try {
    const stored = JSON.parse(localStorage.getItem(GAUGE_KEY) ?? "null") as unknown;
    if (Array.isArray(stored)) {
      const pids = stored.map((p) => Number(p)).filter((p) => Number.isInteger(p) && p > 0);
      if (pids.length > 0) return pids;
    }
  } catch {
    // No stored choice: the defaults.
  }
  return DEFAULT_GAUGES;
}

function saveGauges(pids: number[]): void {
  try {
    localStorage.setItem(GAUGE_KEY, JSON.stringify(pids.map(String)));
  } catch {
    // Not remembered; the choice still applies for this run.
  }
}

export function GaugeCard({ connected }: { connected: boolean }) {
  const [table, setTable] = useState<Map<number, PidInfo>>(new Map());
  const [pids, setPids] = useState<number[]>(loadGauges);
  const [running, setRunning] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    void call("pid_table")
      .then((rows) => {
        if (!cancelled) setTable(new Map(rows.map((r) => [r.key, r])));
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  // Only values the table knows; the defaults if none of the stored ones are.
  const shownPids = (() => {
    if (table.size === 0) return pids;
    const known = pids.filter((p) => table.has(p));
    return known.length > 0 ? known : DEFAULT_GAUGES;
  })();

  useBackendEvent("closed", () => {
    setRunning(false);
    setMenuOpen(false);
  });

  const toggle = () => {
    const next = !running;
    setRunning(next);
    if (next) void call("start_live", { pids: shownPids });
    else void call("stop_live");
  };
  const toggleRef = useRef(toggle);
  toggleRef.current = toggle;
  const runningRef = useRef(running);
  runningRef.current = running;

  // --page start-live: open the start page and start the gauges as if the
  // button was pressed.
  useAppEvent<string>("dev:page", (name) => {
    if (name !== "start-live") return;
    navigate("start");
    if (connected && !runningRef.current) toggleRef.current();
  });

  useEffect(() => {
    if (!menuOpen) return;
    const away = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setMenuOpen(false);
    };
    const key = (e: globalThis.KeyboardEvent) => e.key === "Escape" && setMenuOpen(false);
    window.addEventListener("mousedown", away);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("mousedown", away);
      window.removeEventListener("keydown", key);
    };
  }, [menuOpen]);

  const pick = (pid: number) => {
    let next = shownPids;
    if (!shownPids.includes(pid)) next = [...shownPids, pid];
    else if (shownPids.length > 1) next = shownPids.filter((p) => p !== pid);
    else return;
    setPids(next);
    saveGauges(next);
    if (running) void call("start_live", { pids: next });
  };

  const perRow = shownPids.length > 6 ? 4 : Math.max(1, shownPids.length);

  return (
    <Card className="dash-gauges">
      <div className="dash-gauges-head">
        <h2 className="section-title grow">{tr("Live")}</h2>
        <Button variant="ghost" small icon={running ? "pause" : "play"} disabled={!connected} onClick={toggle}>
          {running ? tr("Live anhalten") : tr("Live starten")}
        </Button>
        <div className="dash-menu-anchor" ref={menuRef}>
          <Button variant="ghost" small icon="sliders" aria-expanded={menuOpen} onClick={() => setMenuOpen((v) => !v)}>
            {tr("Anzeigen wählen")}
          </Button>
          {menuOpen ? (
            <div className="dash-menu fade-in" role="menu">
              {GAUGE_CHOICES.map((pid) => {
                const spec = table.get(pid);
                if (!spec) return null;
                const checked = shownPids.includes(pid);
                // At most eight, so the gauges stay readable.
                const enabled = checked || shownPids.length < 8;
                return (
                  <button
                    key={pid}
                    type="button"
                    role="menuitemcheckbox"
                    aria-checked={checked}
                    disabled={!enabled}
                    className={`dash-menu-item${checked ? " on" : ""}`}
                    onClick={() => {
                      pick(pid);
                      setMenuOpen(false);
                    }}
                  >
                    <span className="dash-menu-check">{checked ? <Icon name="check" size={14} /> : null}</span>
                    <span>{spec.name}</span>
                  </button>
                );
              })}
            </div>
          ) : null}
        </div>
      </div>
      <div className="dash-gauge-grid" style={{ "--per-row": perRow } as CSSProperties}>
        {shownPids.map((pid) => {
          const spec = table.get(pid);
          return spec ? <LiveGauge key={pid} spec={spec} /> : null;
        })}
      </div>
    </Card>
  );
}

// One gauge that follows its own value.
function LiveGauge({ spec }: { spec: PidInfo }) {
  const [value, setValue] = useState<number | null>(() => {
    const last: LiveRow | undefined = live.get(spec.key);
    return last ? last.value : null;
  });
  useBackendEvent("live_value", ({ row }) => {
    if (row.pid === spec.key) setValue(row.value);
  });
  useBackendEvent("closed", () => setValue(null));

  let low = spec.displayMin;
  let high = spec.displayMax;
  if (high <= low) {
    low = 0;
    high = 100;
  }
  const decimals = high - low <= 20 ? 1 : 0;
  const warn = spec.warnLow !== 0 || spec.warnHigh !== 0;
  return (
    <div className="dash-gauge">
      <Gauge
        title={spec.name}
        unit={spec.unit}
        min={low}
        max={high}
        value={value}
        decimals={decimals}
        warnFrom={warn ? spec.warnLow : undefined}
        warnTo={warn ? spec.warnHigh : undefined}
        size={170}
      />
    </div>
  );
}
