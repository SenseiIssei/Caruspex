// The adapter choice of the connection page: the kinds as chips, the
// settings of the chosen one and the truck option.
import { Button, Check, Select, TextInput } from "../../components/ui";
import { translator } from "../../i18n";
import { isAndroid } from "../../lib/platform";
import { BluetoothPermission, ModeChip, RefreshButton } from "./parts";
import { CAN_RATES, canFdPossible, portLabel, type StartConnect } from "./shared";
import type { AdapterChoice } from "./useAdapterChoice";
import type { Bluetooth } from "./useBluetooth";

const tr = translator("caruspex::ConnectPage");
// Texts of the Android app.
const ta = translator("caruspex::Android");
// Texts of the truck option (SAE J1939).
const tk = translator("caruspex::Truck");
const tbmw = translator("caruspex::BmwPage");

export function AdapterOptions({ adapter, bt, startConnect }: { adapter: AdapterChoice; bt: Bluetooth; startConnect: StartConnect }) {
  const {
    mode,
    setMode,
    hasBle,
    ports,
    port,
    setPort,
    address,
    setAddress,
    replayPath,
    setReplayPath,
    replayRealTime,
    setReplayRealTime,
    enetHost,
    setEnetHost,
    canDevices,
    canIndex,
    setCanIndex,
    canRate,
    setCanRate,
    canFd,
    setCanFd,
    truck,
    setTruck,
    truckRate,
    setTruckRate,
    refreshPorts,
    refreshCanDevices,
    pickReplay,
  } = adapter;
  const { ble, setBle, bleScanning, bleNote, btAsked, scanBle, askBluetooth, btBlocked, locationNeeded } = bt;

  const portOptions = [{ value: "", label: tr("Automatisch suchen") }, ...ports.map((p) => ({ value: p.device, label: portLabel(p) }))];
  const bleOptions = ble.items.map((i) => ({ value: i.id, label: i.label }));
  const canOptions = canDevices.map((d, i) => ({ value: i, label: d.label }));
  const fdPossible = canFdPossible(canDevices[canIndex]?.spec ?? "");
  const rateOptions = CAN_RATES.map((r) => ({ value: r, label: `${r / 1000} kbit/s` }));
  const truckRateOptions = [
    { value: 0, label: tk("Automatisch (250, dann 500 kbit/s)") },
    { value: 250, label: tk("250 kbit/s (schwarze Buchse, Typ 1)") },
    { value: 500, label: tk("500 kbit/s (grüne Buchse, Typ 2)") },
  ];
  const truckPossible = mode !== "enet" && mode !== "replay" && mode !== "bmw";

  return (
    <>
      {/* Two rows, so the card keeps its width in every language: the
          cables first, then the ways without one. */}
      <div className="connect-chips">
        <ModeChip
          mode="cable"
          current={mode}
          icon="usb"
          label={isAndroid ? ta("USB-Kabel (OTG)") : tr("USB oder COM-Port")}
          onPick={setMode}
        />
        {hasBle ? <ModeChip mode="ble" current={mode} icon="bluetooth" label={tr("Bluetooth")} onPick={setMode} /> : null}
        <ModeChip mode="wifi" current={mode} icon="wifi" label={tr("WLAN")} onPick={setMode} />
        {/* A phone has neither a network cable nor CAN interfaces. */}
        {isAndroid ? null : <ModeChip mode="enet" current={mode} icon="ethernet" label={tr("Netzwerkkabel")} onPick={setMode} />}
      </div>
      <div className="connect-chips">
        {isAndroid ? null : <ModeChip mode="can" current={mode} icon="cpu" label={tr("CAN-Interface")} onPick={setMode} />}
        <ModeChip mode="demo" current={mode} icon="sparkles" label={tr("Demo-Auto")} onPick={setMode} />
        <ModeChip mode="replay" current={mode} icon="play" label={tr("Aufnahme")} onPick={setMode} />
        {/* The K-line cables need a COM port, which a phone does not have. */}
        {isAndroid ? null : <ModeChip mode="bmw" current={mode} icon="car" label={tbmw("BMW E-Reihe")} onPick={setMode} />}
      </div>

      <div className="connect-mode fade-in" key={mode}>
        {mode === "cable" ? (
          <>
            <div className="connect-line">
              <Select className="grow" value={port} options={portOptions} onChange={setPort} />
              <RefreshButton title={tr("Anschlüsse neu suchen")} onClick={() => void refreshPorts()} />
            </div>
            <div className="hint">
              {isAndroid
                ? ta(
                    "Ein USB-Adapter braucht ein OTG-Kabel am Handy. Android fragt beim ersten " +
                      "Verbinden, ob Caruspex ihn benutzen darf.",
                  )
                : tr(
                    "Bluetooth-Adapter erscheinen nach dem Koppeln in Windows als " +
                      "eigener COM-Anschluss. Bei Automatisch probiert Caruspex alle " +
                      "Anschlüsse der Reihe nach.",
                  )}
            </div>
          </>
        ) : null}

        {mode === "wifi" ? (
          <>
            <TextInput className="connect-field ltr" value={address} onChange={setAddress} onKeyDown={(e) => e.key === "Enter" && startConnect()} />
            <div className="hint">
              {tr("Erst mit dem WLAN des Adapters verbinden. Die meisten nutzen " + "192.168.0.10, Port 35000.")}
            </div>
          </>
        ) : null}

        {mode === "demo" ? (
          <div className="subtle connect-demo-text">
            {truck
              ? tk(
                  "Ein nachgebauter Lkw im Programm: Motor, Getriebe, Bremsen, Kombiinstrument und Tachograph am J1939-Bus, mit zwei aktiven und zwei früheren Fehlern.",
                )
              : tr(
                  "Ein nachgebautes Auto im Programm: es fährt, wärmt sich auf, " +
                    "hat ein paar Fehler gespeichert. Ideal zum Ausprobieren, ganz " +
                    "ohne Adapter.",
                )}
          </div>
        ) : null}

        {mode === "replay" ? (
          <>
            <div className="connect-line">
              <TextInput
                className="grow connect-field"
                value={replayPath}
                onChange={setReplayPath}
                placeholder={tr("Aufnahme wählen (.jsonl)")}
              />
              <Button className="connect-tall" onClick={() => void pickReplay()}>
                {tr("Wählen")}
              </Button>
            </div>
            <Check checked={replayRealTime} onChange={setReplayRealTime}>
              {tr("Im ursprünglichen Tempo abspielen")}
            </Check>
            <div className="hint">
              {tr(
                "Spielt eine aufgezeichnete Sitzung ab, als wäre das Auto da. Gut für " +
                  "die Fehlersuche, oder wenn jemand eine Aufnahme schickt.",
              )}
            </div>
          </>
        ) : null}

        {mode === "ble" && btBlocked ? <BluetoothPermission refused={btAsked} onAllow={() => void askBluetooth()} /> : null}

        {mode === "ble" && locationNeeded ? <BluetoothPermission location refused={btAsked} onAllow={() => void askBluetooth()} /> : null}

        {mode === "ble" && !btBlocked ? (
          <>
            <div className="connect-line">
              <Select
                className="grow"
                value={ble.selected}
                options={bleOptions}
                onChange={(id) => setBle((b) => ({ ...b, selected: id }))}
              />
              <RefreshButton title={tr("Nach Bluetooth-Geräten suchen")} onClick={scanBle} disabled={bleScanning} />
            </div>
            <div className="hint">{bleNote}</div>
          </>
        ) : null}

        {mode === "enet" ? (
          <>
            <TextInput
              className="connect-field"
              value={enetHost}
              onChange={setEnetHost}
              placeholder={tr("Automatisch suchen, oder die Adresse des Autos (169.254.x.x)")}
              onKeyDown={(e) => e.key === "Enter" && startConnect()}
            />
            <div className="hint">
              {tr(
                "Für ein ENET-Kabel (BMW und Mini ab der F-Baureihe) oder ein DoIP-Kabel " +
                  "(viele neuere Autos) zwischen OBD-Buchse und Netzwerkanschluss. " +
                  "Caruspex erkennt selbst, was das Auto spricht. Zündung an; nach dem " +
                  "Einstecken braucht der Rechner bis zu einer Minute für seine " +
                  "automatische Adresse. Caruspex liest nur, Codieren und Flashen gehören " +
                  "in die Werkstattsoftware.",
              )}
            </div>
          </>
        ) : null}

        {mode === "bmw" ? (
          <>
            <div className="hint">
              {tbmw(
                "Für BMW der E-Reihe von etwa 1995 bis 2007 (E36, E38, E39, E46, E53, E83, E85, dazu E60, E87 und E90 bis 03/2007): ein K+DCAN-Kabel mit dem Schalter auf K-Line oder ein KKL-Kabel. Caruspex fragt damit jedes Steuergerät über DS2 oder KWP2000.",
              )}
            </div>
            <div className="hint">
              {tbmw(
                "Den Schalter vor dem Einstecken umlegen; K-Line ist bei den meisten Kabeln die linke Stellung und verbindet Pin 7 und 8. BMW ab 03/2007 und die F- und G-Reihe sprechen D-CAN oder Ethernet: dafür USB, WLAN oder Netzwerkkabel wählen.",
              )}
            </div>
          </>
        ) : null}

        {mode === "can" ? (
          <>
            <div className="connect-line">
              <Select className="grow" value={canIndex} options={canOptions} onChange={setCanIndex} disabled={canOptions.length === 0} />
              <Select className="connect-rate" value={canRate} options={rateOptions} onChange={setCanRate} />
              <RefreshButton title={tr("Anschlüsse neu suchen")} onClick={() => void refreshCanDevices()} />
            </div>
            <Check checked={canFd && fdPossible} disabled={!fdPossible} onChange={setCanFd}>
              {tr("CAN FD (neueste Autos; nur SocketCAN und J2534-Geräte, die es können)")}
            </Check>
            <div className="hint">
              {tr(
                "Für Geräte, die rohe CAN-Nachrichten liefern: J2534-PassThru (Tactrix OpenPort, " +
                  "Mongoose, VCX Nano, Scanmatik und viele mehr, auch mit alten 32-Bit-Treibern), " +
                  "PEAK PCAN und Kvaser mit dem Treiber des Herstellers, SLCAN-Sticks (CANable, " +
                  "USBtin, CANUSB) und unter Linux SocketCAN. Autos mit OBD über CAN fahren mit 500 kbit/s.",
              )}
            </div>
          </>
        ) : null}
      </div>

      {truckPossible ? (
        <div className="connect-truck">
          <Check checked={truck} onChange={setTruck}>
            {tk("Lkw oder Bus (SAE J1939)")}
          </Check>
          {truck ? (
            <>
              <div className="connect-line">
                <span className="subtle">{tk("Bitrate")}</span>
                <Select className="grow" value={truckRate} options={truckRateOptions} onChange={setTruckRate} />
              </div>
              <div className="hint">
                {tk(
                  "Caruspex hört zuerst nur zu, J1939 sendet fast alles von selbst. Transporter wie Sprinter, Transit oder Crafter sprechen normales OBD-II, für sie das Häkchen auslassen.",
                )}
              </div>
            </>
          ) : null}
        </div>
      ) : null}
    </>
  );
}
