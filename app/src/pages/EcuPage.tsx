// All control units over CAN (EcuPage.cpp): asks each one through UDS
// whether it is there, what it is and what it has stored. Works with a plain
// ELM327 on most cars from about 2008. Older VW and Audi go through the KKL
// page instead.
import { useEffect, useMemo, useRef, useState } from "react";
import { call } from "../api";
import type { BrandFamily, DtcInfo, UdsEcuRow } from "../api";
import { Icon, type IconName } from "../components/Icon";
import { confirm } from "../components/Modal";
import { FindingRow } from "../components/Tiles";
import { toast } from "../components/Toast";
import { Button, Card, Page, PageHeader, Progress, Select, cx, type Option } from "../components/ui";
import { translator } from "../i18n";
import { useAppEvent, useBackendEvent } from "../lib/hooks";
import { navigate } from "../shell/nav";
import { useSession } from "../state/session";
import "./EcuPage.css";

const tr = translator("caruspex::EcuPage");

// EcuKind of data/EcuAddresses.h.
const Kind = {
  Engine: 0, Gearbox: 1, Brakes: 2, Airbag: 3, Steering: 4, Instruments: 5, Gateway: 6,
  Body: 7, Climate: 8, Parking: 9, Lighting: 10, Doors: 11, Seats: 12, Infotainment: 13,
  Battery: 14, Charger: 15, Hybrid: 16, Other: 17,
} as const;

function hexId(id: number, extended: boolean): string {
  return id.toString(16).toUpperCase().padStart(extended ? 8 : 3, "0");
}

function kindIcon(kind: number): IconName {
  switch (kind) {
    case Kind.Engine: return "engine";
    case Kind.Gearbox: return "sliders";
    case Kind.Brakes: return "stop";
    case Kind.Airbag: return "shield";
    case Kind.Steering: return "steering";
    case Kind.Instruments: return "gauge";
    case Kind.Gateway: return "cpu";
    case Kind.Climate: return "thermometer";
    case Kind.Battery:
    case Kind.Charger:
    case Kind.Hybrid: return "battery";
    case Kind.Lighting: return "lightbulb";
    default: return "cpu";
  }
}

// The name from the address list, or, for the generic sweep where the list
// only knows the address, the name the unit reports itself (DID F197).
function displayName(r: UdsEcuRow): string {
  if (r.kind === Kind.Other && r.systemName) return r.systemName;
  return r.name;
}

// severityColour() of FaultsPage.cpp; critical is a brighter red there.
function severityTone(severity: number): string {
  switch (severity) {
    case 0: return "var(--info)";
    case 1: return "var(--good)";
    case 2: return "var(--warn)";
    case 3: return "var(--bad)";
    case 4: return "color-mix(in srgb, var(--bad) 78%, var(--text))";
    default: return "var(--text-muted)";
  }
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <>
      <span className="hint ecu-fact-key">{label}</span>
      <span className="mono ecu-fact-value ltr">{value}</span>
    </>
  );
}

export default function EcuPage() {
  const connected = useSession((s) => s.connected);
  const brand = useSession((s) => s.identity?.brand ?? "");
  const vin = useSession((s) => s.vin);

  const [families, setFamilies] = useState<BrandFamily[]>([]);
  const [family, setFamily] = useState("");
  const [brandFamily, setBrandFamily] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<{ value: number; total: number } | null>(null);
  const [status, setStatus] = useState("");
  const [emptyText, setEmptyText] = useState(() => tr("Noch nicht gesucht. Verbinden, dann auf Steuergeräte suchen."));
  // Keyed by request id like the QHash, in the order they answered.
  const [rows, setRows] = useState<UdsEcuRow[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [infos, setInfos] = useState<Record<string, DtcInfo>>({});
  const asked = useRef(new Set<string>());

  useEffect(() => {
    void call("ecu_families").then(setFamilies, () => setFamilies([]));
  }, []);

  const options = useMemo<Option<string>[]>(() => {
    const out: Option<string>[] = [{ value: "", label: tr("Alle üblichen Adressen durchsuchen (langsamer)") }];
    for (const f of families) {
      out.push({ value: f.key, label: tr("Adressliste %1", f.label) });
      if (f.key === "BMW") out.push({ value: "BMW*", label: tr("BMW und Mini: alle 128 Adressen (langsamer, findet alles)") });
    }
    return out;
  }, [families]);

  // setBrand(): every recognised car preselects its address list, or the
  // generic sweep when its brand has none.
  useEffect(() => {
    if (!vin) return;
    let stale = false;
    void call("brand_family", { brand }).then(
      (key) => !stale && setBrandFamily(key),
      () => !stale && setBrandFamily(""),
    );
    return () => {
      stale = true;
    };
  }, [vin, brand]);
  useEffect(() => {
    if (brandFamily === null) return;
    setFamily(options.some((o) => o.value === brandFamily) ? brandFamily : "");
  }, [brandFamily, options]);

  // setConnected(false): whatever ran is over.
  useEffect(() => {
    if (connected) return;
    setRunning(false);
    setProgress(null);
  }, [connected]);

  // The fault texts and severities with the brand of the car.
  useEffect(() => {
    for (const r of rows) {
      for (const d of r.dtcs) {
        const key = `${brand}|${d.code}`;
        if (asked.current.has(key)) continue;
        asked.current.add(key);
        void call("lookup_dtc", { code: d.code, brand }).then(
          (info) => setInfos((m) => ({ ...m, [key]: info })),
          () => asked.current.delete(key),
        );
      }
    }
  }, [rows, brand]);

  useBackendEvent("uds_scan_started", ({ total, generic }) => {
    setRunning(true);
    setRows([]);
    setSelected(null);
    setEmptyText(tr("Suche läuft ..."));
    setProgress({ value: 0, total });
    setStatus(generic ? tr("Durchsucht %1 Adressen. Das dauert ein, zwei Minuten.", total) : tr("Fragt %1 bekannte Steuergeräte.", total));
  });

  useBackendEvent("uds_scan_progress", ({ name, index, total }) => {
    setProgress({ value: index, total });
    setStatus(tr("Frage %1 (%2 von %3)", name, index + 1, total));
  });

  useBackendEvent("uds_ecu_result", ({ row }) => {
    setRows((list) => {
      const at = list.findIndex((r) => r.request === row.request);
      if (at < 0) return [...list, row];
      const next = list.slice();
      next[at] = row;
      return next;
    });
    // The first unit that answers is shown; later ones only join the list.
    setSelected((current) => (current === null ? row.request : current));
  });

  useBackendEvent("uds_scan_finished", ({ present, withFaults, cancelled, note }) => {
    setRunning(false);
    setProgress(null);
    if (note) {
      setStatus(note);
      setEmptyText(note);
      return;
    }
    let text = tr("%1 Steuergeräte gefunden, %2 davon mit Fehlern.", present, withFaults);
    if (cancelled) text += ` ${tr("Abgebrochen.")}`;
    if (present === 0) {
      text = tr(
        "Kein Steuergerät hat geantwortet. Manche Autos lassen über die Buchse nur das Motorsteuergerät antworten, dann hilft nur die Werkstatt.",
      );
      setEmptyText(text);
    }
    setStatus(text);
    toast(text, withFaults > 0 ? "warning" : "success");
  });

  useBackendEvent("uds_clear_result", ({ ok, message }) => {
    toast(message, ok ? "success" : "error");
  });

  // "ecus-run" and "ecus-run:<family>" of MainWindow::showPageByName.
  useAppEvent<string>("dev:page", (name) => {
    if (!name.startsWith("ecus-run")) return;
    navigate("ecus");
    void call("scan_uds", { family: name.slice(9) });
  });

  const current = selected === null ? undefined : rows.find((r) => r.request === selected);

  const clearUnit = async (r: UdsEcuRow) => {
    const ok = await confirm({
      title: tr("Fehler löschen"),
      text: tr(
        "Den Fehlerspeicher von %1 löschen? Ein Fehler, dessen Ursache noch besteht, kommt wieder. Bei Airbag und ABS erst die Ursache beheben.",
        r.name,
      ),
      confirm: tr("Fehler löschen"),
      cancel: tr("Abbrechen"),
      danger: true,
      icon: "trash",
    });
    if (ok) void call("clear_uds_ecu", { request: r.request, response: r.response, extended29: r.extended29, name: r.name });
  };

  return (
    <Page className="ecu-page">
      <PageHeader
        eyebrow={tr("Werkstatt")}
        title={tr("Alle Steuergeräte")}
        subtitle={tr(
          "Fragt über CAN jedes Steuergerät einzeln: Motor, Getriebe, ABS, Airbag, Kombi, Klima und mehr. Mit einem normalen ELM327, bei den meisten Autos ab etwa 2008. Ältere VW und Audi gehen über das KKL-Kabel.",
        )}
      />

      <div className="ecu-bar">
        <Select className="ecu-family" value={family} options={options} onChange={setFamily} />
        <Button
          variant="primary"
          icon="search"
          disabled={!connected || running}
          busy={running}
          onClick={() => void call("scan_uds", { family })}
        >
          {tr("Steuergeräte suchen")}
        </Button>
        {running ? (
          <Button variant="ghost" onClick={() => void call("cancel_scan")}>
            {tr("Abbrechen")}
          </Button>
        ) : null}
      </div>
      {progress ? <Progress value={progress.total > 0 ? (100 * progress.value) / progress.total : 0} /> : null}
      {status ? <div className="subtle ecu-status">{status}</div> : null}

      <div className="ecu-columns">
        <Card className="ecu-list-card">
          {rows.length === 0 ? (
            <div className="ecu-list-empty" />
          ) : (
            <ul className="ecu-list" role="listbox">
              {rows.map((r) => {
                const count = r.dtcs.length;
                const tone = count > 0 ? "var(--bad)" : "var(--good)";
                return (
                  <li key={r.request} className="fade-in">
                    <button
                      type="button"
                      role="option"
                      aria-selected={r.request === selected}
                      className={cx("ecu-item", r.request === selected && "on")}
                      onClick={() => setSelected(r.request)}
                    >
                      <span className="ecu-item-icon" style={{ color: tone }}>
                        <Icon name={kindIcon(r.kind)} size={18} />
                      </span>
                      <span className="ecu-item-text">
                        <span className="ecu-item-name">{displayName(r)}</span>
                        <span className="ecu-item-sub">
                          <span className="mono ltr">{hexId(r.request, r.extended29)}</span>
                          {"  ·  "}
                          <span style={{ color: tone }}>{count > 0 ? tr.n("%n Fehler", count) : tr("in Ordnung")}</span>
                        </span>
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </Card>

        {current ? (
          <Card className="ecu-detail fade-in" key={current.request}>
            <div>
              <h2 className="page-title ecu-name">{displayName(current)}</h2>
              <div className="hint">
                {tr("Anfrage %1, Antwort %2", hexId(current.request, current.extended29), hexId(current.response, current.extended29))}
              </div>
            </div>
            <div className="ecu-facts">
              {current.systemName ? <Fact label={tr("Bezeichnung")} value={current.systemName} /> : null}
              {current.partNumber ? <Fact label={tr("Teilenummer")} value={current.partNumber} /> : null}
              {current.softwareVersion ? <Fact label={tr("Software")} value={current.softwareVersion} /> : null}
              {current.hardwareVersion ? <Fact label={tr("Hardware")} value={current.hardwareVersion} /> : null}
              {current.serial ? <Fact label={tr("Seriennummer")} value={current.serial} /> : null}
            </div>
            {current.note ? <div className="hint wrap">{current.note}</div> : null}
            <hr className="divider" />
            <h3 className="section-title">{tr("Fehlerspeicher")}</h3>
            <div className="ecu-faults">
              {!current.dtcsRead ? (
                <FindingRow
                  icon="info"
                  tint="var(--info)"
                  title={tr("Fehlerspeicher nicht lesbar")}
                  text={tr("Das Steuergerät hat geantwortet, gibt seinen Fehlerspeicher aber nicht heraus.")}
                />
              ) : current.dtcs.length === 0 ? (
                <FindingRow icon="check-circle" tint="var(--good)" title={tr("Kein Fehler gespeichert")} />
              ) : null}
              {current.dtcs.map((d) => {
                const info = infos[`${brand}|${d.code}`];
                const text = info?.known ? info.text : d.text;
                const state: string[] = [];
                if (d.testFailed) state.push(tr("gerade aktiv"));
                if (d.confirmed) state.push(tr("gespeichert"));
                else if (d.pending) state.push(tr("vorläufig"));
                if (d.warningLamp) state.push(tr("Warnlampe an"));
                let detail = state.join(" · ");
                if (d.failureText) detail = `${d.failureText} · ${detail}`;
                if (info?.advice) detail += `\n${info.advice}`;
                return (
                  <FindingRow
                    key={d.code}
                    icon="alert"
                    tint={severityTone(info ? info.severity : 2)}
                    title={
                      <>
                        <span className="mono ltr">{d.code}</span>
                        {"  "}
                        {text}
                      </>
                    }
                    text={<span className="wrap">{detail}</span>}
                    meta={info?.severityLabel}
                  />
                );
              })}
            </div>
            {current.dtcsRead && current.dtcs.length > 0 ? (
              <div>
                <Button variant="danger" icon="trash" className="ecu-clear" onClick={() => void clearUnit(current)}>
                  {tr("Fehler dieses Steuergeräts löschen")}
                </Button>
              </div>
            ) : null}
          </Card>
        ) : (
          <Card className="ecu-empty">
            <div className="ecu-empty-inner fade-in" key={emptyText}>
              <Icon name="cpu" size={46} className="faint" />
              <div className="subtle">{emptyText}</div>
            </div>
          </Card>
        )}
      </div>
    </Page>
  );
}
