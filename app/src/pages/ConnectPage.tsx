// Connecting, step by step: where the socket is, which adapter, then one
// button (ConnectPage.cpp). The page keeps its own picture of the
// connection from the backend events, like the Qt page did with the
// Worker signals MainWindow connected to it, and writes the log lines
// MainWindow wrote before each attempt.
import { useRef } from "react";
import { TermStrip } from "../components/Term";
import { Button, Card, Page, PageHeader, Spinner, cx } from "../components/ui";
import { t, translator } from "../i18n";
import { openSetup } from "../state/setup";
import { AdapterOptions } from "./connect/AdapterOptions";
import { ConnectChecklist } from "./connect/parts";
import type { ConnectLatest } from "./connect/shared";
import { StepsCard } from "./connect/StepsCard";
import { useAdapterChoice } from "./connect/useAdapterChoice";
import { useBluetooth } from "./connect/useBluetooth";
import { useConnectHandovers } from "./connect/useConnectHandovers";
import { useConnection } from "./connect/useConnection";
import { useVehicleChoice } from "./connect/useVehicleChoice";
import { VehiclePicker } from "./connect/VehiclePicker";
import "./ConnectPage.css";
import "../components/J1939.css";

const tr = translator("caruspex::ConnectPage");
const tbmw = translator("caruspex::BmwPage");

export default function ConnectPage() {
  // The latest values for the handlers that run outside a render (app events
  // that connect right after changing the mode or the port).
  const latest = useRef({} as ConnectLatest);
  const adapter = useAdapterChoice(latest);
  const { mode, setMode, port, address, replayPath, replayRealTime, enetHost, canDevices, canIndex, canRate, canFd, truck, truckRate } = adapter;
  const bt = useBluetooth(mode);
  const { ble } = bt;
  const vehicle = useVehicleChoice(adapter.setTruck);
  const { generation } = vehicle;
  const { connected, connecting, stateTone, details, error, help, startConnect } = useConnection(latest);
  latest.current = { mode, port, address, replayPath, replayRealTime, ble, enetHost, canDevices, canIndex, canRate, canFd, truck, truckRate, generation, connected, connecting };

  // The hand-overs from other pages and the command line (connect:demo of
  // ?demo among them): registered when the page mounts.
  useConnectHandovers({
    latest,
    refreshPorts: adapter.refreshPorts,
    setPort: adapter.setPort,
    setMode,
    setAddress: adapter.setAddress,
    setTruck: adapter.setTruck,
    setGeneration: vehicle.setGeneration,
    startConnect,
  });

  const stateText = connecting ? tr("Verbinde ...") : connected ? tr("Verbunden") : tr("Nicht verbunden");

  return (
    <Page className="connect-page">
      <PageHeader
        eyebrow={tr("Verbindung")}
        title={tr("Mit dem Auto verbinden")}
        subtitle={tr(
          "Ein Adapter in der OBD-Buchse, die Zündung an, ein Klick. " +
            "Welches Protokoll das Auto spricht, findet Caruspex " +
            "selbst heraus.",
        )}
        actions={
          <Button icon="sparkles" onClick={() => openSetup()}>
            {t("caruspex::Usability", "Einrichtungs-Assistent")}
          </Button>
        }
      />
      <TermStrip ids={["obd", "elm327", "kkl", "can", "vin"]} />

      <div className="connect-columns">
        <StepsCard socketHint={vehicle.socketHint} truck={truck} motoSteps={vehicle.motoSteps} />

        <Card className="connect-adapter" title={tr("Adapter")}>
          <AdapterOptions adapter={adapter} bt={bt} startConnect={startConnect} />

          <hr className="divider" />

          <VehiclePicker vehicle={vehicle} mode={mode} setMode={setMode} />

          <hr className="divider" />

          <Button
            variant={connected ? "danger" : "primary"}
            icon={connected ? "unplug" : "plug"}
            className={cx("connect-button", !connected && !connecting && "pulse")}
            disabled={connecting}
            onClick={() => startConnect()}
          >
            {connected ? tr("Trennen") : mode === "bmw" ? tbmw("Zur BMW-Seite") : tr("Verbinden")}
          </Button>

          <div className="connect-state">
            {connecting ? <Spinner size={16} /> : null}
            <span className={cx("connect-state-label", stateTone)}>{stateText}</span>
          </div>
          {details ? <div className="hint wrap selectable connect-details">{details}</div> : null}

          {error !== null ? (
            <div className="connect-error fade-in" role="alert">
              <div className="connect-error-head">{tr("Das hat nicht geklappt")}</div>
              <div className="wrap selectable">{error}</div>
              {help ? null : (
                <div className="subtle">
                  {tr(
                    "Häufige Gründe: Zündung aus, Adapter nicht ganz eingesteckt, falscher " +
                      "Anschluss gewählt, oder ein anderes Programm hält den Anschluss " +
                      "offen. Bei Bluetooth hilft oft, den Adapter neu zu koppeln.",
                  )}
                </div>
              )}
            </div>
          ) : null}
          {error !== null && help ? <ConnectChecklist help={help} /> : null}
        </Card>
      </div>
    </Page>
  );
}
