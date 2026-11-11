// Driving mode: four values in large type for a laptop or tablet on the
// dashboard (speed, engine speed, coolant, battery voltage) and the
// warnings that matter while driving. Nothing to press while the car moves;
// the page says so at the top. It reads the values only while it is on
// screen and stops again when left.
import { useEffect, useRef, useState, type CSSProperties } from "react";
import { call } from "../api";
import { Icon, type IconName } from "../components/Icon";
import { Button, EmptyState, Page } from "../components/ui";
import { localeNumber, t, translator } from "../i18n";
import { useBackendEvent, usePageActive } from "../lib/hooks";
import { navigate } from "../shell/nav";
import { live, useSession } from "../state/session";
import "./DrivePage.css";

const tu = translator("caruspex::Usability");

export const DRIVE_PIDS = [0x0d, 0x0c, 0x05, 0x42] as const;

interface Reading {
  value: number;
  unit: string;
  warn: boolean;
}

// The coolant is too hot from 110 °C, the voltage is off below 12 V or
// above 15 V (a running engine charges with about 13.8 to 14.7 V).
export function driveWarnings(values: Map<number, Reading>, mil: boolean, faults: number): { icon: IconName; text: string; tone: string }[] {
  const out: { icon: IconName; text: string; tone: string }[] = [];
  if (mil) out.push({ icon: "engine", text: tu("Motorkontrollleuchte an"), tone: "var(--warn)" });
  if (faults > 0) out.push({ icon: "alert", text: tu("Gespeicherte Fehler: %1", faults), tone: "var(--warn)" });
  const coolant = values.get(0x05);
  if (coolant && coolant.value >= 110) out.push({ icon: "thermometer", text: tu("Motor zu heiß, anhalten und abkühlen lassen"), tone: "var(--bad)" });
  const volts = values.get(0x42);
  if (volts && volts.value > 0 && volts.value < 12) out.push({ icon: "battery", text: tu("Spannung zu niedrig, die Lichtmaschine lädt nicht"), tone: "var(--bad)" });
  if (volts && volts.value > 15) out.push({ icon: "battery", text: tu("Spannung zu hoch, der Regler überlädt"), tone: "var(--bad)" });
  const odd = [...values].some(([pid, r]) => r.warn && pid !== 0x05 && pid !== 0x42);
  if (odd) out.push({ icon: "alert", text: tu("Ein Wert liegt außerhalb des üblichen Bereichs"), tone: "var(--warn)" });
  return out;
}

function Big({ caption, reading, decimals, icon }: { caption: string; reading?: Reading; decimals: number; icon: IconName }) {
  const text = reading ? localeNumber(reading.value, decimals) : "--";
  return (
    <div className={`drive-value${reading?.warn ? " warn" : ""}`} role="group" aria-label={`${caption}: ${text} ${reading?.unit ?? ""}`}>
      <div className="drive-caption">
        <Icon name={icon} size={22} />
        <span>{caption}</span>
      </div>
      <div className="drive-number num" aria-hidden>
        {text}
        <span className="drive-unit">{reading?.unit ?? ""}</span>
      </div>
    </div>
  );
}

export default function DrivePage() {
  const active = usePageActive();
  const connected = useSession((s) => s.connected);
  const mil = useSession((s) => s.status?.mil ?? false);
  const faults = useSession((s) => s.faults.filter((f) => f.stored).length);
  const [values, setValues] = useState<Map<number, Reading>>(() => {
    const m = new Map<number, Reading>();
    for (const pid of DRIVE_PIDS) {
      const row = live.get(pid);
      if (row) m.set(pid, { value: row.value, unit: row.unit, warn: row.warn });
    }
    return m;
  });
  const started = useRef(false);

  // Reads the four values while the page is on screen.
  useEffect(() => {
    if (!active || !connected) return;
    started.current = true;
    void call("start_live", { pids: [...DRIVE_PIDS] });
    return () => {
      if (started.current) void call("stop_live");
      started.current = false;
    };
  }, [active, connected]);

  useBackendEvent("live_value", ({ row }) => {
    if (!(DRIVE_PIDS as readonly number[]).includes(row.pid)) return;
    setValues((before) => new Map(before).set(row.pid, { value: row.value, unit: row.unit, warn: row.warn }));
  });
  useBackendEvent("closed", () => setValues(new Map()));

  const warnings = driveWarnings(values, mil, faults);

  return (
    <Page className="drive-page" wide>
      <div className="drive-safety" role="note">
        <Icon name="alert" size={22} />
        <span>
          {tu(
            "Nicht während der Fahrt bedienen. Den Rechner sicher befestigen, so dass er nichts verdeckt; den Blick auf der Straße lassen. Bedienen darf nur der Beifahrer oder wer steht.",
          )}
        </span>
      </div>

      {!connected ? (
        <EmptyState icon="steering" title={tu("Fahrmodus")} text={tu("Erst verbinden, dann zeigt der Fahrmodus Tempo, Drehzahl, Temperatur und Spannung in großer Schrift.")}>
          <Button variant="primary" icon="plug" onClick={() => navigate("connect")}>
            {t("caruspex::MainWindow", "Verbinden")}
          </Button>
        </EmptyState>
      ) : (
        <>
          <div className="drive-grid">
            <Big caption={tu("Geschwindigkeit")} icon="gauge" reading={values.get(0x0d)} decimals={0} />
            <Big caption={tu("Drehzahl")} icon="activity" reading={values.get(0x0c)} decimals={0} />
            <Big caption={tu("Kühlmittel")} icon="thermometer" reading={values.get(0x05)} decimals={0} />
            <Big caption={tu("Bordspannung")} icon="battery" reading={values.get(0x42)} decimals={1} />
          </div>
          <div className="drive-warnings" aria-live="polite">
            {warnings.length === 0 ? (
              <div className="drive-warning ok" style={{ "--tone": "var(--good)" } as CSSProperties}>
                <Icon name="check-circle" size={26} />
                <span>{tu("Keine Warnung")}</span>
              </div>
            ) : (
              warnings.map((w, i) => (
                <div key={i} className="drive-warning" style={{ "--tone": w.tone } as CSSProperties}>
                  <Icon name={w.icon} size={26} />
                  <span>{w.text}</span>
                </div>
              ))
            )}
          </div>
        </>
      )}
    </Page>
  );
}
