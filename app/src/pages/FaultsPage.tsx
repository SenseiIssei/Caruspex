// The fault memory (FaultsPage.cpp): every code with how urgent it is, and
// for the selected one what it means, whether the car may still be driven,
// where on the car to look, the usual causes and what one can check without
// a workshop.
import { useEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import { call } from "../api";
import type { DtcInfo, FaultRow, FreezeFrameRow, ZoneInfo } from "../api";
import { CarZone } from "../api";
import { Icon } from "../components/Icon";
import { ProHint } from "../components/ProHint";
import { confirm } from "../components/Modal";
import { toast } from "../components/Toast";
import { Button, Card, Page, PageHeader, Pill } from "../components/ui";
import { VehicleDiagram } from "../components/VehicleDiagram";
import { localeNumber, t, translator } from "../i18n";
import { emitApp } from "../lib/store";
import { navigate } from "../shell/nav";
import { useBackendEvent } from "../lib/hooks";
import type { DashboardFinding } from "../state/session";
import { useSession, useSettings } from "../state/session";
import "./FaultsPage.css";

const tr = translator("caruspex::FaultsPage");
const tw = translator("caruspex::WhatToDoPage");

// Colour for a Severity value (severityColour in FaultsPage.cpp): Info, Low,
// Medium, High, Critical.
export function severityColor(severity: number): string {
  switch (severity) {
    case 0: return "var(--info)";
    case 1: return "var(--good)";
    case 2: return "var(--warn)";
    case 3: return "var(--bad)";
    case 4: return "var(--critical)";
  }
  return "var(--text-muted)";
}

// The same as one of the four tones the start page and the report know.
export function severityTone(severity: number): DashboardFinding["tone"] {
  switch (severity) {
    case 0: return "info";
    case 1: return "good";
    case 2: return "warn";
  }
  return "bad";
}

// What the backend knows about a code, for the brand of the car. A failed
// lookup still gives a usable entry, the way an unknown code does.
export async function describeFault(code: string, brand: string): Promise<{ info: DtcInfo; where: ZoneInfo }> {
  const [info, where] = await Promise.all([
    call("lookup_dtc", { code, brand }).catch(() => null),
    call("locate_dtc", { code }).catch(() => null),
  ]);
  return {
    info: info ?? {
      code,
      text: "",
      severity: 2,
      severityLabel: "",
      system: 0,
      systemName: "",
      advice: "",
      causes: [],
      checks: [],
      costClass: 0,
      costLabel: "",
      costHint: "",
      manufacturerSpecific: false,
      known: false,
      hasDetails: false,
      throttleRelated: false,
      family: "",
      familyLabel: "",
    },
    where: where ?? { zone: CarZone.Unknown, name: "", hint: "" },
  };
}

interface Entry {
  fault: FaultRow;
  info: DtcInfo;
  where: ZoneInfo;
}

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

export default function FaultsPage() {
  const connected = useSession((s) => s.connected);
  const faults = useSession((s) => s.faults);
  const haveFaults = useSession((s) => s.haveFaults);
  const frames = useSession((s) => s.freezeFrames);
  const brand = useSession((s) => s.identity?.brand ?? "");
  const body = useSession((s) => s.identity?.body ?? 0);
  const settings = useSettings();

  const [entries, setEntries] = useState<Entry[]>([]);
  const [selected, setSelected] = useState(-1);
  const [clearLog, setClearLog] = useState<string[]>([]);
  const [clearLogVisible, setClearLogVisible] = useState(false);
  const logRef = useRef<HTMLPreElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // onFaults: every code with the meaning for this brand. A new list (or a
  // new brand) selects the first entry again.
  useEffect(() => {
    if (faults.length === 0) {
      setEntries([]);
      setSelected(-1);
      return;
    }
    let cancelled = false;
    void Promise.all(faults.map((f) => describeFault(f.code, brand))).then((found) => {
      if (cancelled) return;
      setEntries(faults.map((fault, i) => ({ fault, info: found[i]!.info, where: found[i]!.where })));
      setSelected(0);
    });
    return () => {
      cancelled = true;
    };
  }, [faults, brand]);

  useEffect(() => {
    const el = logRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [clearLog]);

  useBackendEvent("clear_progress", ({ line }) => {
    setClearLogVisible(true);
    setClearLog((lines) => [...lines, line]);
  });

  useBackendEvent("clear_finished", ({ everythingGone, remaining }) => {
    setClearLog((lines) => [
      ...lines,
      everythingGone
        ? tr("Fertig: der Speicher ist leer.")
        : tr(
            "Fertig: %1 Einträge bleiben stehen. Das heißt, die Ursache besteht noch. Der Reiter Fehlerspeicher zeigt oben rechts am Fahrzeug, wo zu suchen ist.",
            remaining,
          ),
    ]);
  });

  const throttle = entries.some((e) => e.fault.throttleRelated || e.info.throttleRelated);
  const zones = useMemo(() => {
    const set = new Set<CarZone>();
    for (const e of entries) if (e.where.zone !== CarZone.Unknown) set.add(e.where.zone);
    return [...set];
  }, [entries]);
  const current = selected >= 0 && selected < entries.length ? entries[selected]! : null;
  const clearAttempts = settings.clearAttempts > 0 ? settings.clearAttempts : 1;

  const askAndClear = async () => {
    if (settings.confirmClear) {
      const ok = await confirm({
        title: tr("Fehlerspeicher löschen"),
        text: (
          <>
            <div className="strong">{tr("Fehlerspeicher wirklich löschen?")}</div>
            <div className="subtle" style={{ marginTop: 8 }}>
              {tr(
                "Die Fehlerlampe geht aus und die Bereitschaftstests werden zurückgesetzt. Eine Abgasuntersuchung fällt direkt danach durch, bis das Auto wieder ein paar Fahrzyklen gelaufen ist.\n\nLöschen ersetzt keine Reparatur.",
              )}
            </div>
          </>
        ),
        confirm: tr("Ja, löschen"),
        cancel: tr("Abbrechen"),
        danger: true,
        icon: "alert",
      });
      if (!ok) return;
    }
    void call("clear_faults");
  };

  const askAndBruteForce = async () => {
    if (settings.confirmClear) {
      const ok = await confirm({
        title: tr("Hartnäckig löschen"),
        text: (
          <>
            <div className="strong">{tr("Mehrfach löschen und jedes Mal nachprüfen?")}</div>
            <div className="subtle" style={{ marginTop: 8 }}>
              {tr(
                "Es wird bis zu %1-mal gelöscht. Nach jedem Versuch wird nachgelesen, ob der Speicher wirklich leer ist. Bleibt etwas stehen, wird zwischendurch die Verbindung neu aufgebaut, weil manche Steuergeräte den Löschbefehl nur direkt nach dem Wecken annehmen.\n\nWas das nicht kann: einen Fehler beseitigen, dessen Ursache noch besteht. Der steht sofort wieder drin, egal wie oft gelöscht wird. Löschen ersetzt keine Reparatur.\n\nDie Bereitschaftstests werden dabei ebenfalls zurückgesetzt.",
                clearAttempts,
              )}
            </div>
          </>
        ),
        confirm: tr("Ja, hartnäckig löschen"),
        cancel: tr("Abbrechen"),
        danger: true,
        icon: "alert",
      });
      if (!ok) return;
    }
    setClearLog([]);
    setClearLogVisible(true);
    void call("bruteforce_clear", { attempts: clearAttempts });
  };

  const exportCsv = async () => {
    if (faults.length === 0) return;
    const now = new Date();
    const stamp = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}_${pad(now.getHours())}-${pad(now.getMinutes())}`;
    const lines = [tr("Code;Art;Bedeutung")];
    for (const f of faults) lines.push(`${f.code};${f.kind};${f.text.replaceAll(";", ",")}`);
    const filter = tr("CSV (*.csv)");
    try {
      await call("save_text_file", {
        title: tr("Speichern"),
        suggestedName: tr("fehlerspeicher_%1.csv", stamp),
        filters: [{ name: filter.replace(/\s*\(.*\)\s*$/, "") || "CSV", extensions: ["csv"] }],
        content: `${lines.join("\n")}\n`,
      });
    } catch {
      toast(`${tr("Fehler")}: ${tr("Datei lässt sich nicht schreiben.")}`, "error", 6000);
    }
  };

  // Arrow keys move through the list like they did in the QListWidget.
  const onListKey = (e: KeyboardEvent<HTMLDivElement>) => {
    if (entries.length === 0) return;
    let next = selected;
    if (e.key === "ArrowDown") next = Math.min(entries.length - 1, selected + 1);
    else if (e.key === "ArrowUp") next = Math.max(0, selected - 1);
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = entries.length - 1;
    else return;
    e.preventDefault();
    setSelected(next);
    listRef.current?.querySelectorAll<HTMLElement>(".fault-item")[next]?.focus();
  };

  // A click on a lit part of the car picks the first entry that sits there.
  const onZoneClick = (zone: CarZone) => {
    const index = entries.findIndex((e) => e.where.zone === zone);
    if (index >= 0) setSelected(index);
  };

  const emptyText =
    haveFaults && faults.length === 0
      ? tr("Kein Fehler gespeichert. Gut so.")
      : tr("Noch nichts gelesen. Nach dem Verbinden stehen hier die Einträge, mit Erklärung.");

  return (
    <Page className="faults-page">
      <PageHeader
        eyebrow={tr("Diagnose")}
        title={tr("Fehlerspeicher")}
        subtitle={tr(
          "Jeder Eintrag mit Dringlichkeit. Rechts steht, was er bedeutet, ob das Auto noch fahren darf und wo man nachsehen kann.",
        )}
      />

      <div className="faults-actions">
        <Button variant="primary" icon="refresh" disabled={!connected} onClick={() => void call("read_faults")}>
          {tr("Neu einlesen")}
        </Button>
        <Button variant="danger" icon="trash" disabled={!connected} onClick={() => void askAndClear()}>
          {tr("Fehler löschen")}
        </Button>
        <Button variant="danger" disabled={!connected} onClick={() => void askAndBruteForce()}>
          {tr("Hartnäckig löschen")}
        </Button>
        <Button icon="download" disabled={!connected || faults.length === 0} onClick={() => void exportCsv()}>
          {tr("Als CSV speichern")}
        </Button>
        <span className="spacer" />
        {connected && haveFaults ? (
          <span className="subtle faults-summary">
            {faults.length === 0 ? tr("Kein Eintrag im Speicher") : tr.n("%n Einträge", faults.length)}
          </span>
        ) : null}
      </div>

      {connected && throttle ? (
        <div className="faults-throttle fade-in">
          {tr("Mindestens ein Code betrifft die Drosselklappe. Löschen allein reicht da erfahrungsgemäß nicht, die Klappe muss angelernt werden.")}
        </div>
      ) : null}
      {connected && throttle ? <ProHint text={tw("Die Drosselklappe anlernen kann der Profimodus.")} page="throttle" /> : null}

      {clearLogVisible ? (
        <pre ref={logRef} className="faults-clear-log mono selectable fade-in">
          {clearLog.join("\n")}
        </pre>
      ) : null}

      <div className="faults-columns">
        <Card className="faults-list-card">
          <div className="faults-list" ref={listRef} role="listbox" aria-label={tr("Fehlerspeicher")} onKeyDown={onListKey}>
            {entries.map((e, i) => {
              const tone = severityColor(e.info.severity);
              const meta = [e.fault.kind];
              if (e.fault.ecu) meta.push(e.fault.ecu);
              if (e.info.systemName) meta.push(e.info.systemName);
              return (
                <button
                  key={`${e.fault.code}-${i}`}
                  type="button"
                  role="option"
                  aria-selected={i === selected}
                  tabIndex={i === selected || (selected < 0 && i === 0) ? 0 : -1}
                  className={`fault-item fade-in${i === selected ? " selected" : ""}`}
                  style={{ "--tone": tone, animationDelay: `${Math.min(i, 12) * 40}ms` } as CSSProperties}
                  onClick={() => setSelected(i)}
                >
                  <span className="fault-bar" />
                  <span className="fault-main">
                    <span className="fault-top">
                      <span className="fault-code mono ltr">{e.fault.code}</span>
                      <span className="fault-text">{e.info.known ? e.info.text : e.fault.text}</span>
                      <span className="fault-pill">{e.info.severityLabel}</span>
                    </span>
                    <span className="fault-meta">{meta.join("  ·  ")}</span>
                  </span>
                </button>
              );
            })}
          </div>
        </Card>

        {current ? (
          <FaultDetail
            key="detail"
            entry={current}
            frames={frames}
            zones={zones}
            body={body}
            onZoneClick={onZoneClick}
          />
        ) : (
          <Card className="faults-empty">
            <div className="faults-empty-inner fade-in" key={emptyText}>
              <Icon name="check-circle" size={46} stroke={1.4} className="faint" />
              <div className="subtle">{emptyText}</div>
            </div>
          </Card>
        )}
      </div>

      <ProHint
        text={tw("Hier steht der Fehlerspeicher des Motors. Airbag, ABS, Getriebe und alle anderen Steuergeräte liest der Profimodus.")}
        page="ecus"
      />

      <div className="hint faults-footer">
        {tr(
          "Löschen setzt auch die Bereitschaftstests zurück. Direkt danach fällt eine Abgasuntersuchung durch, bis das Auto wieder ein paar vollständige Fahrzyklen gelaufen ist. Ein Fehler, dessen Ursache noch da ist, kommt nach kurzer Fahrt zurück.",
        )}
      </div>
    </Page>
  );
}

function FaultDetail({
  entry,
  frames,
  zones,
  body,
  onZoneClick,
}: {
  entry: Entry;
  frames: FreezeFrameRow[];
  zones: CarZone[];
  body: number;
  onZoneClick: (zone: CarZone) => void;
}) {
  const { fault, info, where } = entry;
  const tone = severityColor(info.severity);
  const meta = [fault.kind];
  if (fault.ecu) meta.push(fault.ecu);
  if (info.systemName) meta.push(info.systemName);
  if (info.manufacturerSpecific && info.familyLabel) meta.push(tr("Herstellercode %1", info.familyLabel));
  if (!info.known) meta.push(tr("genaue Bedeutung unbekannt, das ist die Gruppe"));

  // The freeze frame the control unit stored together with this code.
  const match = frames.find((f) => f.dtc === fault.code) ?? null;
  const frameRows = match ? match.values.filter((v) => v.pid !== 0x02) : [];

  return (
    <Card className="faults-detail" style={{ "--tone": tone } as CSSProperties}>
      <div className="fade-in faults-detail-inner" key={fault.code}>
        <div className="faults-detail-head">
          <span className="faults-detail-code mono ltr">{fault.code}</span>
          <Pill tone={tone}>{info.severityLabel}</Pill>
        </div>
        <h2 className="section-title">{info.known ? info.text : fault.text}</h2>
        <div className="hint">{meta.join("  ·  ")}</div>

        <div className="faults-advice">
          <div className="strong">{tr("Darf ich weiterfahren?")}</div>
          <div>{info.advice}</div>
        </div>

        <VehicleDiagram
          className="faults-diagram"
          body={body}
          anatomy
          affected={zones}
          focus={where.zone}
          onZoneClick={onZoneClick}
        />
        <div className="faults-where">
          {where.zone !== CarZone.Unknown ? (
            <div className="subtle grow">{tr("Wo nachsehen: %1. %2", where.name, where.hint)}</div>
          ) : (
            <span className="grow" />
          )}
          <Button
            small
            icon="cube"
            onClick={() => {
              navigate("xray");
              if (where.zone !== CarZone.Unknown) emitApp("xray:zone", where.zone);
            }}
          >
            {t("caruspex::XrayPage", "In 3D ansehen")}
          </Button>
        </div>

        {info.causes.length > 0 ? (
          <section className="faults-section">
            <div className="eyebrow">{tr("Häufige Ursachen")}</div>
            {info.causes.map((c, i) => (
              <div key={i} className="faults-cause">
                {`${i + 1}.  ${c}`}
              </div>
            ))}
          </section>
        ) : null}

        {info.checks.length > 0 ? (
          <section className="faults-section">
            <div className="eyebrow">{tr("Selbst nachsehen")}</div>
            {info.checks.map((c, i) => (
              <div key={i} className="faults-check">
                <Icon name="check" size={14} className="faults-check-icon" />
                <span>{c}</span>
              </div>
            ))}
          </section>
        ) : null}

        {match && match.values.length > 0 ? (
          <section className="faults-section">
            <div className="eyebrow">{tr("Was im Moment des Fehlers los war")}</div>
            {frameRows.map((v) => (
              <div key={v.pid} className="faults-frame-row">
                <span className="subtle grow">{v.name}</span>
                <span className="strong num">{v.text ? v.text : `${localeNumber(v.value, 1)} ${v.unit}`}</span>
              </div>
            ))}
          </section>
        ) : null}

        {info.costClass !== 0 ? (
          <div className="hint">{tr("Kosten in der Werkstatt: %1. %2", info.costLabel, info.costHint)}</div>
        ) : null}
      </div>
    </Card>
  );
}
