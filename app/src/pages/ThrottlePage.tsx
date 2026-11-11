// Throttle body adaptation (ThrottlePage.cpp). Two ways, depending on which
// adapter is plugged in:
//  - KKL cable: real basic settings via KWP1281 or KWP2000, measuring block
//    060 or 098
//  - ELM327: observe only, plus the adaptation attempt without a scan tool
import { useEffect, useState } from "react";
import { call } from "../api";
import type { LiveRow } from "../api";
import { Gauge } from "../components/Gauge";
import { Button, Card, Page, PageHeader, Progress, Select } from "../components/ui";
import { fixed, translator } from "../i18n";
import { useBackendEvent } from "../lib/hooks";
import { useSession, useSettings } from "../state/session";
import { KklMessage, KklState, PortChoice, noWay, useMessageBox, usePorts } from "./WorkshopShared";

const tr = translator("caruspex::ThrottlePage");

// Address of the engine control unit (Ecu::Engine).
const ENGINE = 0x01;

const SESSIONS = [0x85, 0x86, 0x87, 0x89];

const needsCable = () =>
  tr("Für die echte Grundeinstellung wird ein KKL-Kabel gebraucht. Ohne das Kabel bleibt nur der Anlernversuch weiter unten.");

export default function ThrottlePage() {
  const eobdConnected = useSession((s) => s.connected);
  const settings = useSettings();
  // ThrottlePage::refreshPorts names the chip, or else the description.
  const ports = usePorts(tr, (device, description) => (description ? `${device}  ${description}` : device));
  const [messageBox, showMessage] = useMessageBox();

  const [kklConnected, setKklConnected] = useState(false);
  const [identification, setIdentification] = useState("");
  const [protocol, setProtocol] = useState("");
  const [isKwp2000, setIsKwp2000] = useState(false);
  const [running, setRunning] = useState(false);
  const [group, setGroup] = useState(60);
  const [session, setSession] = useState(0x85);
  const [message, setMessage] = useState(() => ({ text: needsCable(), error: false }));
  const [values, setValues] = useState<LiveRow[]>([]);
  const [angle, setAngle] = useState<number | null>(null);
  const [rpm, setRpm] = useState<number | null>(null);

  // The session type the Worker sets on connecting comes from the settings.
  useEffect(() => {
    setSession(SESSIONS.includes(settings.basicSettingSession) ? settings.basicSettingSession : 0x85);
  }, [settings.basicSettingSession]);

  // updateAvailability() after the OBD connection changed.
  useEffect(() => {
    if (!kklConnected) setMessage((m) => (m.error ? m : { text: needsCable(), error: false }));
    // Only the OBD side triggers this; the K-line events handle themselves
    // below, so kklConnected is left out of the dependencies on purpose.
  }, [eobdConnected]);

  useBackendEvent("kkl_connected", ({ connected, identification: id }) => {
    setKklConnected(connected);
    if (!connected) setRunning(false);
    setIdentification(id);
    setProtocol("");
    if (!connected) setMessage({ text: needsCable(), error: false });
  });

  useBackendEvent("kkl_protocol", ({ protocolName, isKwp2000: modern }) => {
    // The session type only matters for KWP2000.
    setIsKwp2000(modern);
    setProtocol(protocolName);
  });

  useBackendEvent("kkl_message", ({ message: text, isError }) => {
    setMessage({ text, error: isError });
    if (isError) setRunning(false);
  });

  useBackendEvent("basic_setting_values", ({ values: rows }) => {
    setRunning(true);
    setValues(rows);
    for (const v of rows) {
      if (v.unit === "1/min") setRpm(v.value);
      // The core may name the angle unit in either language.
      if (v.unit === "%" || v.unit === "Grad" || v.unit === "deg" || v.unit === "°") setAngle(v.value);
    }
  });

  // As long as only EOBD is connected, the throttle can at least be observed.
  useBackendEvent("live_value", ({ row }) => {
    if (row.pid === 0x11 || row.pid === 0x45) setAngle(row.value);
    if (row.pid === 0x0c) setRpm(row.value);
  });

  const connectOrClose = () => {
    if (kklConnected) {
      void call("close_kkl");
      return;
    }
    if (!ports.port) {
      showMessage(tr("Kein Anschluss"), noWay());
      return;
    }
    void call("open_kkl", { device: ports.port, ecuAddress: ENGINE });
  };

  const runOrStop = () => {
    if (running) {
      void call("stop_basic_setting");
      setRunning(false);
      return;
    }
    void call("start_basic_setting", { group });
  };

  const changeSession = (mode: number) => {
    setSession(mode);
    void call("set_basic_setting_session", { mode });
  };

  const shownIdentification = protocol ? `${protocol}\n${identification}` : identification;

  return (
    <Page className="throttle-page">
      <PageHeader eyebrow={tr("WERKSTATT")} title={tr("Drosselklappe anlernen")} />

      <Card title={tr("Zwei Wege, je nach Adapter")} icon="info">
        <div className="ws-explain">
          {tr(
            "Die Grundeinstellung ist keine EOBD-Funktion. Über einen ELM327-Adapter kommt man nicht daran. Mit einem KKL-Kabel (VAG 409.1) geht es, Caruspex spricht die VAG-Protokolle selbst.\n\nWelches Protokoll das Steuergerät spricht, verrät es nach der Weckadresse selbst, und Caruspex richtet sich danach:\n  - KWP1281, das alte Blockprotokoll. Üblich beim 1.4 mit Saugrohreinspritzung. Dort gibt es einen eigenen Befehl für die Grundeinstellung.\n  - KWP2000, der Nachfolger. Üblich beim 1.6 FSI mit Bosch MED. Dort wird stattdessen eine andere Sitzungsart eröffnet.\n\nDer Messwertblock hängt am Motor: 060 bei Saugrohreinspritzung, 098 bei Direkteinspritzung. Meldet das Steuergerät, dass es den Block nicht kennt, nimm den anderen.",
          )}
        </div>
      </Card>

      <Card title={tr("KKL-Kabel verbinden")} icon="cable">
        <div className="ws-bar">
          <PortChoice caption={tr("Anschluss:")} options={ports.options} value={ports.port} onChange={ports.setPort} />
          <Button icon="refresh" onClick={() => void ports.refresh()}>
            {tr("Suchen")}
          </Button>
          <Button variant={kklConnected ? "default" : "primary"} icon={kklConnected ? "unplug" : "plug"} onClick={connectOrClose}>
            {kklConnected ? tr("Trennen") : tr("Verbinden")}
          </Button>
        </div>
        <KklState connected={kklConnected} text={kklConnected ? tr("verbunden mit dem Motorsteuergerät") : tr("nicht verbunden")} />
        <div className="ws-ident">{shownIdentification}</div>
      </Card>

      <Card title={tr("Vorher prüfen")} icon="check-circle">
        <div className="ws-explain">
          {tr(
            "Damit das Anlernen sauber durchläuft, müssen diese Punkte stimmen:\n  - Motor betriebswarm, mindestens 80 Grad Kühlmitteltemperatur\n  - Alle Verbraucher aus: Licht, Gebläse, Klima, Heckscheibe\n  - Lenkrad geradeaus, Getriebe in Leerlauf, Handbremse angezogen\n  - Gaspedal nicht antippen, während die Grundeinstellung läuft\n  - Fehlerspeicher vorher löschen, sonst bricht das Steuergerät ab",
          )}
        </div>
      </Card>

      <Card title={tr("Grundeinstellung starten")} icon="throttle">
        <div className="ws-bar">
          <Select
            className="ws-grow"
            value={group}
            onChange={setGroup}
            options={[
              { value: 60, label: tr("Messwertblock 060 (Saugrohreinspritzung, z.B. 1.4 MPI)") },
              { value: 98, label: tr("Messwertblock 098 (Direkteinspritzung, z.B. 1.6 FSI)") },
              { value: 1, label: tr("Messwertblock 001 (nur beobachten, verändert nichts)") },
            ]}
          />
          <Button variant={running ? "danger" : "primary"} icon={running ? "stop" : "play"} disabled={!kklConnected} onClick={runOrStop}>
            {running ? tr("Grundeinstellung beenden") : tr("Grundeinstellung starten")}
          </Button>
        </div>

        {/* Only with KWP2000: there is no dedicated service for basic settings
            there; a different session type is opened instead, and which one
            the control unit expects is not consistent. */}
        {isKwp2000 ? (
          <>
            <div className="throttle-session fade-in">
              <span className="ws-caption">{tr("Sitzungsart:")}</span>
              <Select
                value={session}
                onChange={changeSession}
                options={[
                  { value: 0x85, label: tr("Sitzung 0x85 (Anpassung, üblich)") },
                  { value: 0x86, label: tr("Sitzung 0x86") },
                  { value: 0x87, label: tr("Sitzung 0x87") },
                  { value: 0x89, label: tr("Sitzung 0x89 (Diagnose, nur lesen)") },
                ]}
              />
            </div>
            <div className="hint wrap">
              {tr(
                "Steuergeräte mit KWP2000 kennen keinen eigenen Befehl für die Grundeinstellung. Stattdessen wird eine andere Sitzungsart eröffnet und derselbe Messwertblock gelesen, das Steuergerät regelt dann mit. Welche Sitzungsart es verlangt, steht in keinem öffentlichen Papier. Lehnt es die erste ab, nimm die nächste.",
              )}
            </div>
          </>
        ) : null}

        {running ? <Progress indeterminate /> : null}
        <KklMessage text={message.text} error={message.error} />

        <div className="throttle-gauges">
          <Gauge title={tr("Klappenwinkel")} unit={tr("%")} min={0} max={100} decimals={1} value={angle} />
          <Gauge title={tr("Leerlauf")} unit={tr("1/min")} min={0} max={3000} value={rpm} warnFrom={600} warnTo={1100} />
        </div>

        <div className="table-wrap" style={{ minHeight: 170 }}>
          <table className="table ws-values">
            <thead>
              <tr>
                <th>{tr("Feld")}</th>
                <th className="value">{tr("Wert")}</th>
                <th>{tr("Einheit")}</th>
              </tr>
            </thead>
            <tbody>
              {values.map((v, i) => (
                <tr key={`${i}-${v.name}`}>
                  <td>{v.name}</td>
                  <td className="value ltr">{fixed(v.value, 2)}</td>
                  <td className="ltr">{v.unit}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      <Card title={tr("Anlernversuch ohne Tester")} icon="wrench">
        <div className="ws-explain">
          {tr(
            "Wenn kein KKL-Kabel da ist, klappt das bei vielen VAG-Motoren auch so. Garantiert ist es nicht.\n\n  1. Fehlerspeicher löschen\n  2. Zündung aus, Batterie 30 Minuten abklemmen, wieder anklemmen\n  3. Zündung an, 30 Sekunden warten, Motor NICHT starten. Die Klappe fährt dabei hörbar ihre Endanschläge ab\n  4. Motor starten, 3 bis 5 Minuten im Leerlauf warm laufen lassen, Gaspedal nicht antippen\n  5. Fehlerspeicher erneut auslesen",
          )}
        </div>
      </Card>
      {messageBox}
    </Page>
  );
}
