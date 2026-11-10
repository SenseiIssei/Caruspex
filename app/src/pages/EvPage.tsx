// The traction battery of electric cars and hybrids (EvPage.cpp): health,
// charge, cells and temperatures from the manufacturer values of the
// built-in packs, read straight from the battery control unit. The source
// of every value is shown next to it.
import { useEffect, useMemo, useRef, useState } from "react";
import { call } from "../api";
import type { CustomRow, PidPackInfo } from "../api";
import { ScoreRing } from "../components/ScoreRing";
import { StatTile } from "../components/Tiles";
import { Button, Card, Page, PageHeader, Select } from "../components/ui";
import { localeNumber, translator } from "../i18n";
import { useAppEvent, useBackendEvent } from "../lib/hooks";
import { navigate } from "../shell/nav";
import { useSession } from "../state/session";
import "./EvPage.css";

const tr = translator("caruspex::EvPage");

// PidCategory::HybridEv
const HYBRID_EV = 8;

const num = (v: number, decimals: number) => localeNumber(v, decimals);

// The stable name a value is found by ("SOH", "Batt V").
const shortKey = (row: CustomRow) => row.shortNameEn || row.shortName;

export default function EvPage() {
  const connected = useSession((s) => s.connected);
  const identity = useSession((s) => s.identity);
  const [packs, setPacks] = useState<PidPackInfo[]>([]);
  const [packId, setPackId] = useState("");
  const [running, setRunning] = useState(false);
  const [status, setStatus] = useState(
    tr(
      "Für Hyundai Kona und Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf und Tesla Model 3 und Y (per Mithören, mit Adapterkabel an der Diagnosebuchse). Andere E-Autos lassen sich über eigene Werte auf der Seite Live-Werte ergänzen.",
    ),
  );
  // ShortName -> last value, and key -> what the table shows.
  const [values, setValues] = useState<Map<string, number>>(new Map());
  const [cells, setCells] = useState<Map<number, string>>(new Map());
  const [refreshed, setRefreshed] = useState(false);
  const runningRef = useRef(false);
  const connectedRef = useRef(connected);
  connectedRef.current = connected;

  useEffect(() => {
    void call("pid_packs").then((list) => {
      const ev = list.filter((p) => p.category === HYBRID_EV);
      setPacks(ev);
      setPackId((current) => current || ev[0]?.id || "");
    });
  }, []);

  const pack = packs.find((p) => p.id === packId);
  const packKeys = () => (pack ? pack.values.map((v) => v.key) : []);
  const byKey = useMemo(() => new Map((pack?.values ?? []).map((v) => [v.key, v])), [pack]);

  // Preselects the pack that fits the connected car: the one whose name
  // mentions the model, else the brand.
  useEffect(() => {
    if (!identity || packs.length === 0) return;
    const brand = identity.brand;
    const hay = `${brand} ${identity.title}`.toLowerCase();
    let best = -1;
    packs.forEach((p, i) => {
      const text = p.title.toLowerCase();
      for (const word of ["ioniq", "kona", "niro", "zoe", "leaf"]) {
        if (hay.includes(word) && text.includes(word)) best = i;
      }
      if (best < 0 && brand && text.includes(brand.toLowerCase())) best = i;
    });
    if (best >= 0) choosePack(packs[best]!.id);
  }, [identity, packs]);

  const choosePack = (id: string) => {
    if (id === packId) return;
    setPackId(id);
    setValues(new Map());
    setCells(new Map());
    setRefreshed(true);
    if (runningRef.current) {
      const next = packs.find((p) => p.id === id);
      void startPoll(next ? next.values.map((v) => v.key) : []);
    }
  };

  const startPoll = async (keys: number[]) => {
    await call("start_live", { pids: [] });
    await call("start_custom_poll", { keys });
  };

  const readNow = () => {
    setStatus(tr("Lese ..."));
    void call("read_custom", { keys: packKeys() });
  };

  const toggleLive = () => {
    const next = !runningRef.current;
    runningRef.current = next;
    setRunning(next);
    if (next) void startPoll(packKeys());
    else void call("stop_live");
  };

  useEffect(() => {
    if (connected) return;
    runningRef.current = false;
    setRunning(false);
  }, [connected]);

  useAppEvent<string>("dev:page", (name) => {
    if (name !== "ev-run") return;
    navigate("ev");
    if (connectedRef.current) readNow();
  });

  useBackendEvent("live_value", ({ row }) => {
    const spec = byKey.get(row.pid);
    if (!spec) return;
    setValues((prev) => new Map(prev).set(shortKey(spec), row.value));
    setCells((prev) => new Map(prev).set(row.pid, row.text ? row.text : `${num(row.value, 2)} ${row.unit}`.trim()));
    setRefreshed(true);
  });

  useBackendEvent("custom_read", ({ ok, failed, firstError }) => {
    if (ok === 0 && failed > 0) {
      setStatus(tr("Keine Antwort vom Batteriesteuergerät: %1. Passt das Paket zum Auto, und ist die Zündung an?", firstError));
    } else if (failed > 0) {
      setStatus(tr("%1 Werte gelesen, %2 ohne Antwort.", ok, failed));
    } else {
      setStatus(tr.n("%n Wert(e) gelesen.", ok));
    }
  });

  // Values some cars do not send as such, worked out from what they do send.
  const v = new Map(values);
  if (!v.has("SOH") && v.has("Full kWh") && (v.get("BOL kWh") ?? 0) > 1) v.set("SOH", (100 * v.get("Full kWh")!) / v.get("BOL kWh")!);
  if (!v.has("Batt kW") && v.has("Batt V") && v.has("Batt A")) v.set("Batt kW", (v.get("Batt V")! * v.get("Batt A")!) / 1000);
  const has = (key: string) => v.has(key);
  const get = (key: string) => v.get(key) ?? 0;

  const sohKey = has("SOH") ? "SOH" : "SOH LBC";
  let ring = { score: -1, caption: tr("Akkugesundheit"), sub: refreshed ? tr("noch nicht gelesen") : tr("SOH") };
  if (has(sohKey)) {
    const soh = get(sohKey);
    ring = {
      score: Math.min(100, Math.max(0, Math.round(soh))),
      caption: soh >= 90 ? tr("Wie neu") : soh >= 80 ? tr("Gut") : soh >= 70 ? tr("Gebraucht") : tr("Deutlich gealtert"),
      sub: tr("Akkugesundheit"),
    };
  }
  const socKey = has("SOC display") ? "SOC display" : has("SOC") ? "SOC" : "SOC BMS";
  const charge = has(socKey) ? `${num(get(socKey), 0)} %` : "--";
  const chargeNote =
    has("SOC BMS") && has("SOC display") ? tr("Anzeige %1 %, Batteriesteuergerät intern %2 %", num(get("SOC display"), 1), num(get("SOC BMS"), 1)) : "";

  const cellsKnown = has("Cell V max") && has("Cell V min");
  const spreadMv = cellsKnown ? (has("Cell V diff") ? get("Cell V diff") * 1000 : (get("Cell V max") - get("Cell V min")) * 1000) : 0;
  // A healthy pack keeps its cells within a few hundredths of a volt.
  const spreadTone = spreadMv < 30 ? "var(--good)" : spreadMv < 80 ? "var(--warn)" : "var(--bad)";
  const tKey = has("Batt T max") ? "Batt T max" : has("Batt T") ? "Batt T" : "Batt T avg";
  const temp = has(tKey) ? get(tKey) : null;

  // One plain sentence on what the numbers mean.
  const advice: string[] = [];
  if (has(sohKey) && get(sohKey) < 70) {
    advice.push(
      tr(
        "Der Akku hat deutlich an Kapazität verloren. Reichweite und Ladeleistung sind spürbar geringer; Garantiebedingungen prüfen, viele Hersteller garantieren 70 %.",
      ),
    );
  }
  if (cellsKnown && get("Cell V max") - get("Cell V min") > 0.08) {
    advice.push(
      tr(
        "Die Zellen sind ungleich geladen. Einmal langsam auf 100 % laden und einige Stunden stehen lassen hilft dem Ausgleich; bleibt es so, in die Werkstatt.",
      ),
    );
  }
  const shown = refreshed;

  return (
    <Page className="ev-page">
      <PageHeader
        eyebrow={tr("Messen")}
        title={tr("Akku von E-Auto und Hybrid")}
        subtitle={tr(
          "Akkugesundheit, Ladestand, Zellen und Temperaturen direkt aus dem Batteriesteuergerät. Die Werte kommen aus belegten Herstellerlisten, die Quelle steht bei jedem Wert.",
        )}
      />

      <div className="ev-bar">
        <Select<string>
          className="ev-pack"
          value={packId}
          options={packs.map((p) => ({ value: p.id, label: p.title }))}
          onChange={choosePack}
        />
        <Button variant="primary" icon="battery" disabled={!connected} onClick={readNow}>
          {tr("Akku lesen")}
        </Button>
        <Button icon={running ? "pause" : "play"} disabled={!connected} onClick={toggleLive}>
          {running ? tr("Anhalten") : tr("Laufend lesen")}
        </Button>
      </div>
      <div className="hint ev-status">{status}</div>

      <Card className="ev-hero">
        <div className="ev-hero-row">
          <ScoreRing size={220} score={ring.score} caption={ring.caption} subcaption={ring.sub} />
          <div className="ev-charge">
            <div className="eyebrow">{tr("LADESTAND")}</div>
            <div className="ev-charge-value num ltr">{charge}</div>
            {chargeNote ? <div className="subtle">{chargeNote}</div> : null}
            {advice.length > 0 ? <div className="ev-advice wrap">{advice.join("\n")}</div> : null}
          </div>
        </div>
      </Card>

      <div className="ev-tiles">
        <StatTile icon="zap" caption={tr("Spannung")} value={shown ? (has("Batt V") ? num(get("Batt V"), 1) : "--") : undefined} unit={shown ? "V" : undefined} />
        <StatTile icon="activity" caption={tr("Strom")} value={shown ? (has("Batt A") ? num(get("Batt A"), 1) : "--") : undefined} unit={shown ? "A" : undefined} />
        <StatTile icon="bolt-circle" caption={tr("Leistung")} value={shown ? (has("Batt kW") ? num(get("Batt kW"), 1) : "--") : undefined} unit={shown ? "kW" : undefined} />
        <StatTile
          icon="battery"
          caption={tr("Zellen")}
          value={cellsKnown ? `${num(get("Cell V min"), 2)} / ${num(get("Cell V max"), 2)}` : undefined}
          unit={cellsKnown ? "V" : undefined}
        />
        <StatTile
          icon="sliders"
          caption={tr("Zellspreizung")}
          value={cellsKnown ? num(spreadMv, 0) : undefined}
          unit={cellsKnown ? "mV" : undefined}
          state={cellsKnown ? spreadTone : undefined}
          note={cellsKnown ? (spreadMv < 30 ? tr("gleichmäßig") : spreadMv < 80 ? tr("etwas ungleich") : tr("deutlich ungleich")) : undefined}
        />
        <StatTile
          icon="thermometer"
          caption={tr("Akkutemperatur")}
          value={temp !== null ? num(temp, 0) : undefined}
          unit={temp !== null ? "°C" : undefined}
          state={temp !== null ? (temp > 45 ? "var(--bad)" : temp < 0 ? "var(--info)" : "var(--good)") : undefined}
          note={temp !== null && has("Batt T min") ? tr("kälteste Stelle %1 °C", num(get("Batt T min"), 0)) : undefined}
        />
        <StatTile
          icon="battery"
          caption={tr("12-Volt-Batterie")}
          value={has("12V") ? num(get("12V"), 1) : undefined}
          unit={has("12V") ? "V" : undefined}
          state={has("12V") ? (get("12V") < 12 ? "var(--warn)" : "var(--good)") : undefined}
        />
      </div>

      <Card title={tr("Alle Werte")}>
        <div className="table-wrap ev-table">
          <table className="table">
            <thead>
              <tr>
                <th>{tr("Wert")}</th>
                <th>{tr("Messung")}</th>
                <th>{tr("Quelle")}</th>
              </tr>
            </thead>
            <tbody>
              {(pack?.values ?? []).map((row) => (
                <tr key={row.key}>
                  <td>{row.name}</td>
                  <td className="num ev-cell">
                    <span className="ltr">{cells.get(row.key) ?? "--"}</span>
                  </td>
                  <td className="faint ev-source">{row.source}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </Page>
  );
}
