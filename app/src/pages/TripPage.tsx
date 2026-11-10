// Trip computer (TripPage.cpp): consumption live and on average, distance,
// cost, CO2 and a gentle score for the driving style, also for cars without
// a trip computer of their own. The computing is analysis::TripComputer in
// the backend, fed from the live loop; the page starts and stops it, polls
// the summary twice a second and hands finished trips to the garage.
import { useEffect, useRef, useState } from "react";
import { call } from "../api";
import type { FuelType, GarageTrip, TripConfig, TripSummary } from "../api";
import { ScoreRing } from "../components/ScoreRing";
import { StatTile } from "../components/Tiles";
import { toast } from "../components/Toast";
import { Button, Card, NumberInput, Page, PageHeader, Select } from "../components/ui";
import { localeNumber, t, translator } from "../i18n";
import { useBackendEvent, useInterval } from "../lib/hooks";
import { addTrip, isoNow } from "../state/garage";
import { useSession } from "../state/session";
import "./TripPage.css";

const tr = translator("caruspex::TripPage");

// What the trip computer can use. The worker skips what the car lacks.
const TRIP_PIDS = [0x0d, 0x5e, 0x10, 0x0c, 0x0b, 0x0f, 0x04, 0x2f];

const STORE_KEY = "caruspex.bordcomputer";

interface Setup {
  price: number;
  displacement: number;
  fuel: FuelType;
}

// QSettings "bordcomputer/preis", "bordcomputer/hubraum", "bordcomputer/kraftstoff".
function loadSetup(): Setup {
  const fallback: Setup = { price: 1.799, displacement: 1.4, fuel: 1 };
  try {
    const text = localStorage.getItem(STORE_KEY);
    return text ? { ...fallback, ...(JSON.parse(text) as Partial<Setup>) } : fallback;
  } catch {
    return fallback;
  }
}

function saveSetup(setup: Setup): void {
  try {
    localStorage.setItem(STORE_KEY, JSON.stringify(setup));
  } catch {
    // Without storage the values only last for this session.
  }
}

function configOf(setup: Setup): TripConfig {
  return {
    fuel: setup.fuel,
    displacementLitres: setup.displacement,
    volumetricEfficiency: 0.85,
    fuelPricePerLitre: setup.price,
    maxGapSeconds: 5,
    liveWindowSeconds: 5,
  };
}

const num = (v: number, decimals: number) => localeNumber(v, decimals);

type StartLabel = "start" | "pause" | "continue";

export default function TripPage() {
  const connected = useSession((s) => s.connected);
  const vin = useSession((s) => s.vin);
  const [setup, setSetup] = useState<Setup>(loadSetup);
  const [running, setRunning] = useState(false);
  const [label, setLabel] = useState<StartLabel>("start");
  const [locked, setLocked] = useState(false);
  const [summary, setSummary] = useState<TripSummary | null>(null);
  const [eco, setEco] = useState<{ score: number; caption: string; text: string } | null>(null);

  const runningRef = useRef(false);
  const tripStart = useRef<Date | null>(null);
  const supported = useRef(new Set<number>());
  const wasConnected = useRef(false);
  // The car the trip belongs to; kept over the disconnect that ends it.
  const lastVin = useRef("");
  if (vin) lastVin.current = vin;
  const setupRef = useRef(setup);
  setupRef.current = setup;

  useBackendEvent("connected", () => {
    lastVin.current = "";
  });
  useBackendEvent("supported_pids", ({ pids }) => {
    supported.current = new Set(pids);
  });

  const rebuildComputer = () => call("trip_reset", { config: configOf(setupRef.current) });

  useEffect(() => {
    void rebuildComputer();
    // Only once: the backend keeps the computer between pages.
  }, []);

  const show = (s: TripSummary) => {
    setSummary(s);
    if (s.distanceKm > 0.3) {
      setEco({ score: s.eco.total, caption: tr("%1 starke Beschleunigungen", s.eco.harshAccelerations), text: s.eco.explanation });
    }
  };

  const refresh = async () => show(await call("trip_summary"));

  // A trip worth keeping has ended (reset or disconnect): into the garage.
  const handOverTrip = async () => {
    const s = await call("trip_summary");
    // Short hops are not worth a line in the garage.
    if (s.distanceKm < 0.3) return;
    const trip: GarageTrip = {
      start: isoNow(tripStart.current ?? new Date()),
      end: isoNow(),
      distanceKm: s.distanceKm,
      fuelLitres: s.fuelLitres,
      averageL100: s.averageLitresPer100Km,
      cost: s.cost,
      co2Kg: s.co2Kg,
      maxSpeedKmh: s.maxSpeedKmh,
      ecoScore: s.distanceKm > 0.3 ? s.eco.total : -1,
    };
    tripStart.current = null;
    const owner = lastVin.current;
    if (!owner) return;
    addTrip(owner, trip);
    toast(t("caruspex::MainWindow", "Fahrt in der Garage gespeichert."), "success");
  };

  // setConnected: a disconnect ends the trip and stops the computer.
  useEffect(() => {
    if (!connected && wasConnected.current) {
      void handOverTrip();
      if (runningRef.current) {
        runningRef.current = false;
        setRunning(false);
        setLabel("start");
        void call("trip_stop");
      }
    }
    wasConnected.current = connected;
  }, [connected]);

  useInterval(() => void refresh(), running ? 500 : null);

  const toggle = async () => {
    const next = !runningRef.current;
    runningRef.current = next;
    setRunning(next);
    if (next) {
      if (!tripStart.current) tripStart.current = new Date();
      const pids = TRIP_PIDS.filter((pid) => supported.current.size === 0 || supported.current.has(pid));
      await call("trip_start");
      await call("start_live", { pids });
      setLabel("pause");
      setLocked(true);
    } else {
      await call("stop_live");
      show(await call("trip_stop"));
      setLabel("continue");
    }
  };

  const resetTrip = async () => {
    await handOverTrip();
    await rebuildComputer();
    setLocked(false);
    if (!runningRef.current) setLabel("start");
    setSummary(null);
    setEco(null);
  };

  const change = (patch: Partial<Setup>) => {
    const next = { ...setupRef.current, ...patch };
    setupRef.current = next;
    setSetup(next);
    saveSetup(next);
    if (!runningRef.current) void rebuildComputer();
  };

  const s = summary;
  const standing = s !== null && s.liveLitresPer100Km <= 0 && s.liveLitresPerHour > 0;
  const liveText = s === null ? "--" : standing ? num(s.liveLitresPerHour, 1) : s.liveLitresPer100Km > 0 ? num(s.liveLitresPer100Km, 1) : "--";
  const startText = label === "pause" ? tr("Fahrt anhalten") : label === "continue" ? tr("Weiterfahren") : tr("Fahrt starten");

  return (
    <Page className="trip-page">
      <PageHeader
        eyebrow={tr("Messen")}
        title={tr("Bordcomputer")}
        subtitle={tr(
          "Verbrauch, Kosten und Fahrstil, auch bei Autos, die selbst keinen Bordcomputer haben. Am genauesten mit dem Kraftstoffdurchfluss, sonst aus der Luftmasse gerechnet.",
        )}
      />

      <Card className="trip-hero">
        <div className="trip-hero-row">
          <div className="trip-live">
            <div className="eyebrow">{standing ? tr("VERBRAUCH IM STAND") : tr("VERBRAUCH JETZT")}</div>
            <div className="trip-value-row">
              <span className="trip-value num ltr">{liveText}</span>
              <span className="trip-unit ltr">{standing ? "l/h" : "l/100 km"}</span>
            </div>
            {s?.fuelNote ? <div className="hint trip-note">{s.fuelNote}</div> : null}
            <div className="row wrap-row trip-controls">
              <Button variant="primary" icon={label === "pause" ? "pause" : "play"} disabled={!connected} onClick={() => void toggle()}>
                {startText}
              </Button>
              <Button icon="refresh" onClick={() => void resetTrip()}>
                {tr("Zurücksetzen")}
              </Button>
            </div>
            <div className="row wrap-row trip-setup">
              <Select<FuelType>
                value={setup.fuel}
                disabled={locked}
                options={[
                  { value: 1, label: tr("Benzin") },
                  { value: 2, label: tr("Diesel") },
                ]}
                onChange={(fuel) => change({ fuel })}
              />
              <NumberInput value={setup.price} min={0} max={5} step={0.01} decimals={3} suffix={tr(" €/l").trim()} onChange={(price) => change({ price })} />
              <span title={tr("Nur nötig, wenn das Auto weder Durchfluss noch Luftmasse meldet.")}>
                <NumberInput
                  value={setup.displacement}
                  min={0}
                  max={8}
                  step={0.1}
                  decimals={1}
                  disabled={locked}
                  suffix={tr(" l Hubraum").trim()}
                  onChange={(displacement) => change({ displacement })}
                />
              </span>
            </div>
          </div>
          <div className="trip-eco">
            <ScoreRing
              size={190}
              score={eco ? eco.score : -1}
              caption={tr("Fahrstil")}
              subcaption={eco ? eco.caption : tr("noch keine Fahrt")}
            />
            {eco?.text ? <div className="hint trip-eco-text">{eco.text}</div> : null}
          </div>
        </div>
      </Card>

      <div className="trip-tiles">
        <StatTile icon="route" caption={tr("Strecke")} value={s ? num(s.distanceKm, s.distanceKm < 10 ? 2 : 1) : undefined} unit={s ? "km" : undefined} />
        <StatTile
          icon="fuel"
          caption={tr("Verbrauch im Schnitt")}
          value={s ? (s.averageLitresPer100Km > 0 ? num(s.averageLitresPer100Km, 1) : "--") : undefined}
          unit={s ? "l/100 km" : undefined}
        />
        <StatTile icon="fuel" caption={tr("Kraftstoff verbraucht")} value={s ? num(s.fuelLitres, 2) : undefined} unit={s ? "l" : undefined} />
        <StatTile icon="file" caption={tr("Kosten")} value={s ? (setup.price > 0 ? num(s.cost, 2) : "--") : undefined} unit={s ? "€" : undefined} />
        <StatTile
          icon="leaf"
          caption={tr("CO₂")}
          value={s ? (s.co2GramsPerKm > 0 ? num(s.co2GramsPerKm, 0) : "--") : undefined}
          unit={s ? "g/km" : undefined}
          note={s ? tr("%1 kg gesamt", num(s.co2Kg, 2)) : undefined}
        />
        <StatTile icon="gauge" caption={tr("Tempo im Schnitt")} value={s ? num(s.averageSpeedKmh, 0) : undefined} unit={s ? "km/h" : undefined} />
        <StatTile icon="zap" caption={tr("Höchsttempo")} value={s ? num(s.maxSpeedKmh, 0) : undefined} unit={s ? "km/h" : undefined} />
        <StatTile
          icon="clock"
          caption={tr("Leerlauf")}
          value={s ? num(s.idleShare * 100, 0) : undefined}
          unit={s ? "%" : undefined}
          state={s ? (s.idleShare > 0.25 ? "var(--warn)" : "var(--good)") : undefined}
          note={s && s.idleLitresPerHour > 0 ? tr("%1 l/h im Stand", num(s.idleLitresPerHour, 2)) : undefined}
        />
      </div>
    </Page>
  );
}
