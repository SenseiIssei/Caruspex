// Start page (DashboardPage.cpp): the connected car as a picture, its health
// as a ring with every finding listed beside it, key values as tiles and the
// live gauges.
import { useEffect, useRef, useState, type CSSProperties } from "react";
import { call } from "../api";
import type { HealthFindingRow, LiveRow, PidInfo } from "../api";
import { Gauge } from "../components/Gauge";
import type { IconName } from "../components/Icon";
import { Icon } from "../components/Icon";
import { ScoreRing } from "../components/ScoreRing";
import { FindingRow, StatTile } from "../components/Tiles";
import { Button, Card, Page } from "../components/ui";
import { VehicleDiagram } from "../components/VehicleDiagram";
import { fixed, localeNumber, t, translator } from "../i18n";
import { useAppEvent, useBackendEvent } from "../lib/hooks";
import { emitApp } from "../lib/store";
import { navigate, openReport } from "../shell/nav";
import { live, recordCheck, session, useSession, type DashboardFinding } from "../state/session";
import { describeFault, severityColor, severityTone } from "./FaultsPage";
import "./DashboardPage.css";

const tr = translator("caruspex::DashboardPage");
const HP = "caruspex::HealthPage";

// What can be put on a gauge, in menu order. Names, units and ranges come
// from the PID table, so they follow the language and the standard.
const GAUGE_CHOICES = [0x0c, 0x0d, 0x05, 0x04, 0x11, 0x42, 0x0f, 0x10, 0x0b, 0x2f, 0x5c, 0x5e, 0x0e, 0x06, 0x07, 0x3c, 0x46, 0x49, 0x70, 0xa4];
const DEFAULT_GAUGES = [0x0c, 0x0d, 0x05, 0x04, 0x11, 0x42];
// The QSettings key of the Qt version.
const GAUGE_KEY = "start/anzeigen";

const TONE_COLOR: Record<DashboardFinding["tone"], string> = {
  bad: "var(--bad)",
  warn: "var(--warn)",
  good: "var(--good)",
  info: "var(--info)",
};

// One line of the findings list, with the exact tint (a critical code has
// its own red) next to the tone the report uses.
interface ShownFinding extends DashboardFinding {
  tint: string;
}

interface Shown {
  score: number;
  caption: string;
  subcaption: string;
  findings: ShownFinding[];
  clickableFaults: boolean;
}

// levelIcon / levelColour / levelLabel of HealthPage.cpp.
function levelIcon(level: number): IconName {
  return level === 0 ? "check-circle" : level === 1 ? "info" : level === 2 ? "alert" : "x-circle";
}
function levelTone(level: number): DashboardFinding["tone"] {
  return level === 0 ? "good" : level === 1 ? "info" : level === 2 ? "warn" : "bad";
}
function levelLabel(level: number): string {
  return level === 0 ? t(HP, "In Ordnung") : level === 1 ? t(HP, "Hinweis") : level === 2 ? t(HP, "Bald ansehen") : t(HP, "Dringend");
}

function finding(icon: string, tone: DashboardFinding["tone"], title: string, text: string, meta = "", tint?: string): ShownFinding {
  return { icon, tone, title, text, meta, tint: tint ?? TONE_COLOR[tone] };
}

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

export default function DashboardPage() {
  const connected = useSession((s) => s.connected);
  const busy = useSession((s) => s.busy);
  const identity = useSession((s) => s.identity);
  const adapter = useSession((s) => s.adapter);
  const transport = useSession((s) => s.transport);
  const protocol = useSession((s) => s.protocol);
  const faults = useSession((s) => s.faults);
  const haveRead = useSession((s) => s.haveFaults);
  const status = useSession((s) => s.status);
  const brand = identity?.brand ?? "";

  // The result of a full health check replaces the page's own rating until
  // the connection closes.
  const [external, setExternal] = useState<Shown | null>(null);
  const [shown, setShown] = useState<Shown>({ score: -1, caption: "", subcaption: "", findings: [], clickableFaults: false });

  // showFindings(): what the ring and the list show, and what the report and
  // the garage get from the start page.
  const show = (next: Shown) => {
    setShown(next);
    session.set({
      score: next.score,
      scoreCaption: next.caption,
      findings: next.findings.map(({ icon, tone, title, text, meta }) => ({ icon, tone, title, text, meta })),
    });
    if (next.score >= 0) recordCheck();
  };
  const showRef = useRef(show);
  showRef.current = show;

  useAppEvent<{ score: number; caption: string; findings: HealthFindingRow[] }>("dashboard:health", ({ score, caption, findings }) => {
    setExternal({
      score,
      caption,
      subcaption: tr("Gesundheit"),
      findings: findings.map((f) => finding(levelIcon(f.level), levelTone(f.level), f.title, f.explanation, levelLabel(f.level))),
      clickableFaults: false,
    });
  });

  useBackendEvent("closed", () => setExternal(null));

  // rebuildFindings(): a simple rating until the full health check reports.
  useEffect(() => {
    if (external) {
      showRef.current(external);
      return;
    }
    if (!connected) {
      showRef.current({
        score: -1,
        caption: tr("Noch offen"),
        subcaption: tr("Gesundheit"),
        findings: [
          finding("info", "info", tr("Noch nichts geprüft"), tr("Nach dem Verbinden steht hier, was mit dem Auto los ist, in einfachen Worten.")),
        ],
        clickableFaults: false,
      });
      return;
    }
    if (!haveRead) {
      showRef.current({
        score: -1,
        caption: tr("Bereit"),
        subcaption: tr("Auto prüfen drücken"),
        findings: [
          finding(
            "sparkles",
            "info",
            tr("Bereit zur Prüfung"),
            tr("Auto prüfen liest den Fehlerspeicher, die Bereitschaftstests und die Fahrzeugdaten."),
            "",
            "var(--accent)",
          ),
        ],
        clickableFaults: false,
      });
      return;
    }

    let cancelled = false;
    void Promise.all(faults.map((f) => describeFault(f.code, brand))).then((described) => {
      if (cancelled) return;
      // What is stored weighs most, then the lamp, then the open readiness
      // tests.
      let score = 100;
      const list: ShownFinding[] = [];
      const weight = [3, 6, 12, 20, 30];
      faults.forEach((f, i) => {
        const { info, where } = described[i]!;
        // Weight by how urgent the code is; a pending code counts half.
        let minus = weight[info.severity] ?? 12;
        if (!f.stored) minus = Math.floor((minus + 1) / 2);
        score -= minus;
        list.push(
          finding(
            "alert",
            severityTone(info.severity),
            `${f.code}  ${info.known ? info.text : f.text}`,
            `${info.severityLabel} · ${f.kind} · ${where.name}`,
            `−${minus}`,
            severityColor(info.severity),
          ),
        );
      });
      const mil = status?.mil ?? false;
      const incomplete = status ? status.monitorReady.filter((ready) => !ready).length : 0;
      if (status && mil) {
        score -= 12;
        list.push(
          finding(
            "engine",
            "bad",
            tr("Motorkontrollleuchte an"),
            tr("Das Steuergerät meldet einen abgasrelevanten Fehler. Bald in die Werkstatt oder selbst nachsehen."),
            "−12",
          ),
        );
      }
      if (status && incomplete > 0) {
        const minus = Math.min(9, incomplete * 3);
        score -= minus;
        list.push(
          finding(
            "clock",
            "info",
            incomplete === 1 ? tr("1 Bereitschaftstest noch offen") : tr("%1 Bereitschaftstests noch offen", incomplete),
            tr("Nach dem Löschen oder einem Batteriewechsel normal. Für die AU sollten sie abgeschlossen sein."),
            `−${minus}`,
          ),
        );
      }
      if (list.length === 0) {
        list.push(finding("check-circle", "good", tr("Alles in Ordnung"), tr("Kein Fehler gespeichert, die Lampe ist aus.")));
      }
      score = Math.max(0, Math.min(100, score));
      const caption = score >= 85 ? tr("Gut in Schuss") : score >= 60 ? tr("Braucht Pflege") : tr("Bitte ansehen");
      showRef.current({ score, caption, subcaption: tr("Gesundheit"), findings: list, clickableFaults: true });
    });
    return () => {
      cancelled = true;
    };
  }, [external, connected, haveRead, faults, status, brand]);

  // --- Hero ---------------------------------------------------------------------------
  let stateTone = "var(--accent)";
  let stateText = tr("Verbunden. Auto prüfen liest Fehler, Bereitschaft und Fahrzeugdaten.");
  if (haveRead) {
    const stored = faults.filter((f) => f.stored).length;
    if (faults.length === 0) {
      stateTone = "var(--good)";
      stateText = tr("Kein Fehler gespeichert.");
    } else {
      stateTone = stored > 0 ? "var(--bad)" : "var(--warn)";
      stateText = faults.length === 1 ? tr("1 Eintrag im Fehlerspeicher.") : tr("%1 Einträge im Fehlerspeicher.", faults.length);
    }
  }
  const subtitle = !connected
    ? tr("Adapter einstecken, Zündung an, verbinden. Oder erst einmal mit dem Demo-Auto ausprobieren.")
    : identity
      ? identity.subtitle
      : tr("%1 über %2, Protokoll %3", adapter, transport, protocol);

  const primary = () => {
    if (!connected) {
      navigate("connect");
      return;
    }
    void call("read_faults");
    void call("read_details");
  };

  return (
    <Page className="dashboard-page">
      <Card className="dash-hero">
        <div className="dash-hero-row">
          <div className="dash-hero-text">
            <div className="eyebrow">{connected ? tr("DEIN FAHRZEUG") : tr("WILLKOMMEN")}</div>
            <h1 className="headline">{connected ? identity?.title || tr("Verbundenes Fahrzeug") : tr("Noch kein Auto verbunden")}</h1>
            {subtitle ? <div className="subtle">{subtitle}</div> : null}
            {connected ? (
              <div className="dash-state fade-in" key={stateText}>
                <span className="dash-state-dot" style={{ background: stateTone }} />
                <span>{stateText}</span>
              </div>
            ) : null}
            <div className="dash-hero-buttons">
              <Button variant="primary" icon={connected ? "sparkles" : "plug"} onClick={primary}>
                {connected ? tr("Auto prüfen") : tr("Verbinden")}
              </Button>
              <Button
                icon="activity"
                disabled={!connected}
                onClick={() => {
                  navigate("live");
                  emitApp("live:toggle");
                }}
              >
                {tr("Live-Werte")}
              </Button>
              <Button icon="file" disabled={!connected} title={tr("Alles Gelesene als PDF speichern oder drucken")} onClick={openReport}>
                {tr("Prüfbericht")}
              </Button>
            </div>
          </div>
          <button
            type="button"
            className="dash-hero-car"
            title={t("caruspex::XrayPage", "Röntgenansicht öffnen")}
            onClick={() => navigate("xray")}
          >
            <VehicleDiagram body={connected ? (identity?.body ?? 0) : 0} scanning={busy && connected} />
            <span className="dash-hero-xray">
              <Icon name="cube" size={14} />
              {t("caruspex::XrayPage", "In 3D ansehen")}
            </span>
          </button>
        </div>
      </Card>

      <div className="dash-middle">
        <Card className="dash-health">
          <div className="dash-health-row">
            <ScoreRing score={shown.score} caption={shown.caption} subcaption={shown.subcaption} busy={busy && connected} size={220} />
            <div className="dash-findings">
              <h2 className="section-title">{tr("Was Caruspex gefunden hat")}</h2>
              <div className="subtle">{tr("Jeder Punkt Abzug steht hier, mit Grund.")}</div>
              <div className="dash-findings-list">
                {shown.findings.map((f, i) => (
                  <div key={`${f.title}-${i}`} className="fade-in" style={{ animationDelay: `${i * 60}ms` }}>
                    <FindingRow
                      icon={f.icon as IconName}
                      tint={f.tint}
                      title={f.title}
                      text={f.text}
                      meta={f.meta || undefined}
                      onClick={shown.clickableFaults && f.icon === "alert" ? () => navigate("faults") : undefined}
                    />
                  </div>
                ))}
              </div>
            </div>
          </div>
        </Card>
        <LiveTiles />
      </div>

      <GaugeCard connected={connected} />
    </Page>
  );
}

// --- Tiles ------------------------------------------------------------------------------

// Battery, coolant, odometer and tank. Kept in their own component so a live
// value redraws four tiles, not the whole page.
function LiveTiles() {
  const [values, setValues] = useState<Record<number, number>>({});
  useBackendEvent("live_value", ({ row }) => {
    if (row.pid !== 0x42 && row.pid !== 0x05 && row.pid !== 0xa6 && row.pid !== 0x2f) return;
    setValues((v) => (v[row.pid] === row.value ? v : { ...v, [row.pid]: row.value }));
  });
  useBackendEvent("closed", () => setValues({}));

  const battery = values[0x42];
  const coolant = values[0x05];
  const odometer = values[0xa6];
  const fuel = values[0x2f];
  return (
    <div className="dash-tiles">
      <StatTile
        icon="battery"
        caption={tr("Batterie")}
        value={battery !== undefined ? fixed(battery, 1) : undefined}
        unit={battery !== undefined ? "V" : undefined}
        state={battery === undefined ? undefined : battery < 11.8 ? "var(--bad)" : battery < 12.3 ? "var(--warn)" : "var(--good)"}
      />
      <StatTile
        icon="thermometer"
        caption={tr("Kühlmittel")}
        value={coolant !== undefined ? fixed(coolant, 0) : undefined}
        unit={coolant !== undefined ? "°C" : undefined}
        state={coolant === undefined ? undefined : coolant > 110 ? "var(--bad)" : coolant < 70 ? "var(--info)" : "var(--good)"}
        note={coolant !== undefined && coolant < 70 ? tr("noch nicht warm") : undefined}
      />
      <StatTile
        icon="route"
        caption={tr("Kilometerstand")}
        value={odometer !== undefined ? localeNumber(odometer, 0) : undefined}
        unit={odometer !== undefined ? "km" : undefined}
      />
      <StatTile
        icon="fuel"
        caption={tr("Tank")}
        value={fuel !== undefined ? fixed(fuel, 0) : undefined}
        unit={fuel !== undefined ? "%" : undefined}
        state={fuel === undefined ? undefined : fuel < 12 ? "var(--warn)" : "var(--good)"}
      />
    </div>
  );
}

// --- Gauges -------------------------------------------------------------------------------

function GaugeCard({ connected }: { connected: boolean }) {
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
