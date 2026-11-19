// BMW's E series on the K-line (worker/bmw.rs): a K+DCAN cable switched to
// K-line or a KKL cable, every known module address in DS2, KWP2000 and
// BMW-FAST, the units found with their identification and fault memory.
// Clearing only after asking, one unit or every unit with entries. Two
// simulated cars (E46 with DS2, E60 with KWP2000) are in the port list for
// trying it without a car.
import { Fragment, useCallback, useEffect, useState } from "react";
import { call } from "../api";
import type { BmwUnitRow } from "../api";
import { confirm } from "../components/Modal";
import { toast } from "../components/Toast";
import { Button, Note, Page, PageHeader, Progress, type Option } from "../components/ui";
import { translator } from "../i18n";
import { useAppEvent, useBackendEvent } from "../lib/hooks";
import { navigate } from "../shell/nav";
import { setProMode } from "../state/session";
import { PortChoice, useMessageBox } from "./WorkshopShared";

const tb = translator("caruspex::BmwPage");
// Texts this page shares with the KKL scan, the unit page and the worker.
const ts = translator("caruspex::ScanPage");
const te = translator("caruspex::EcuPage");
const tw = translator("caruspex::Worker");

// worker/bmw.rs: the simulated cars of caruspex_demo::bmw.
export const BMW_SIM_E46 = "sim:e46";
export const BMW_SIM_E60 = "sim:e60";

function unitKey(row: { address: number; protocol: string }): string {
  return `${row.protocol}-${row.address}`;
}

function faultState(present: boolean | null): string {
  if (present === null) return "";
  return present ? te("gerade aktiv") : te("gespeichert");
}

export default function BmwPage() {
  const [messageBox, showMessage] = useMessageBox();
  const [options, setOptions] = useState<Option<string>[]>([]);
  const [port, setPort] = useState(BMW_SIM_E46);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<{ value: number; total: number } | null>(null);
  const [status, setStatus] = useState(() => ts("Noch nicht durchsucht."));
  const [statusError, setStatusError] = useState(false);
  const [units, setUnits] = useState<BmwUnitRow[]>([]);

  // The serial ports, then the two simulated cars. A real port comes first
  // when there is one.
  const refresh = useCallback(async (keep?: string) => {
    let list: Option<string>[] = [];
    try {
      const ports = await call("list_serial_ports");
      list = ports.map((p) => ({ value: p.device, label: p.description ? `${p.device}  [${p.description}]` : p.device }));
    } catch {
      // No ports is the same as no answer.
    }
    list.push({ value: BMW_SIM_E46, label: tb("Demo: BMW E46 mit DS2 (simuliert)") });
    list.push({ value: BMW_SIM_E60, label: tb("Demo: BMW E60 mit KWP2000 (simuliert)") });
    setOptions(list);
    // A fresh list starts at its first entry; "Suchen" keeps the choice.
    setPort(keep !== undefined && list.some((o) => o.value === keep) ? keep : list[0]!.value);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const startScan = (device: string, clear: boolean) => {
    setUnits([]);
    setStatusError(false);
    setStatus(tb("Öffne die K-Line ..."));
    void call("scan_bmw", { device, clear });
  };

  // --page bmw-ds2 and bmw-e60: the simulated car, scanned right away.
  useAppEvent<string>("dev:page", (name) => {
    const device = name === "bmw-ds2" ? BMW_SIM_E46 : name === "bmw-e60" ? BMW_SIM_E60 : "";
    if (!device) return;
    setProMode(true);
    navigate("bmw");
    void refresh(device).then(() => startScan(device, false));
  });

  useBackendEvent("bmw_scan_started", ({ total }) => {
    setRunning(true);
    setProgress({ value: 0, total });
  });

  useBackendEvent("bmw_scan_progress", ({ index, total, name, protocol }) => {
    setRunning(true);
    setProgress({ value: index, total });
    setStatus(tb("Frage %1 über %2 an (%3 von %4) ...", name, protocol, index + 1, total));
  });

  // A unit found by the scan, or read again after clearing.
  useBackendEvent("bmw_unit", ({ row }) => {
    setUnits((list) => {
      const at = list.findIndex((u) => unitKey(u) === unitKey(row));
      if (at < 0) return [...list, row];
      const next = [...list];
      next[at] = row;
      return next;
    });
  });

  useBackendEvent("bmw_scan_finished", ({ present, withFaults, cancelled, error }) => {
    setRunning(false);
    setProgress(null);
    if (error) {
      setStatusError(true);
      setStatus(error);
      return;
    }
    if (cancelled) {
      setStatus(ts("Abgebrochen. %1 Steuergeräte geprüft.", present));
      return;
    }
    if (present === 0) {
      setStatusError(true);
      setStatus(
        tb(
          "Kein Steuergerät hat geantwortet. Zündung an? Steht der Schalter des Kabels auf K-Line? Bei BMW ab 03/2007 läuft die Diagnose über D-CAN, nicht über die K-Line.",
        ),
      );
      return;
    }
    setStatus(
      withFaults === 0
        ? ts("Fertig. %1 Steuergeräte gefunden, keines meldet einen Fehler.", present)
        : ts("Fertig. %1 Steuergeräte gefunden, %2 davon mit Einträgen im Speicher.", present, withFaults),
    );
  });

  useBackendEvent("bmw_clear_result", ({ address, protocol, ok, remaining, error }) => {
    const unit = units.find((u) => unitKey(u) === unitKey({ address, protocol }));
    const name = unit?.name ?? `0x${address.toString(16).toUpperCase().padStart(2, "0")}`;
    if (!ok) toast(tw("Löschen bei %1 hat nicht geklappt: %2", name, error), "error");
    else if (remaining > 0) toast(tb("%1: gelöscht, aber %2 Einträge sind gleich wieder da. Die Ursache besteht noch.", name, remaining), "warning");
    else toast(tw("Fehlerspeicher von %1 gelöscht.", name), "success");
  });

  const startOrCancel = () => {
    if (running) {
      void call("cancel_scan");
      setStatus(ts("Abbruch angefordert, das laufende Steuergerät wird noch zu Ende gefragt ..."));
      return;
    }
    if (!port) {
      showMessage(ts("Kein Anschluss"), tb("Erst ein K+DCAN- oder KKL-Kabel anstecken und auf Suchen drücken."));
      return;
    }
    startScan(port, false);
  };

  const clearEverywhere = async () => {
    const ok = await confirm({
      title: ts("Überall löschen"),
      icon: "alert",
      danger: true,
      text: (
        <>
          <div className="strong">{ts("In jedem gefundenen Steuergerät den Fehlerspeicher löschen?")}</div>
          <div className="subtle" style={{ marginTop: 8 }}>
            {tb(
              "Caruspex sucht alle Steuergeräte, löscht bei jedem mit Einträgen einmal und liest danach nach. Ein Fehler, dessen Ursache noch besteht, steht sofort wieder drin. Löschen ersetzt keine Reparatur.\n\nZündung an, Motor aus.",
            )}
          </div>
        </>
      ),
      confirm: ts("Ja, überall löschen"),
      cancel: ts("Abbrechen"),
    });
    if (ok) startScan(port, true);
  };

  const clearUnit = async (unit: BmwUnitRow) => {
    const ok = await confirm({
      title: te("Fehler löschen"),
      text: te(
        "Den Fehlerspeicher von %1 löschen? Ein Fehler, dessen Ursache noch besteht, kommt wieder. Bei Airbag und ABS erst die Ursache beheben.",
        unit.name,
      ),
      confirm: te("Fehler löschen"),
      cancel: te("Abbrechen"),
      danger: true,
      icon: "trash",
    });
    if (ok) void call("clear_bmw_unit", { device: port, address: unit.address, protocol: unit.protocol });
  };

  return (
    <Page className="scan-page bmw-page">
      <PageHeader
        eyebrow={ts("WERKSTATT")}
        title={tb("BMW E-Reihe über K-Line")}
        subtitle={tb(
          "Liest bei BMW von etwa 1995 bis 2007 jedes Steuergerät über die K-Line: die älteren Modelle sprechen DS2, die E60-Generation KWP2000. Dafür braucht es ein K+DCAN-Kabel im K-Line-Modus oder ein KKL-Kabel; ein ELM327 kommt an diese Steuergeräte nicht heran.",
        )}
      />

      <Note icon="cable" title={tb("So kommt das Kabel auf die K-Line")}>
        <ul className="bmw-steps">
          <li>
            {tb(
              "K+DCAN-Kabel mit Schalter: vor dem Einstecken auf K-Line stellen, bei den meisten Kabeln die linke Stellung. Sie verbindet Pin 7 und 8 der Buchse, damit auch die Steuergeräte an Pin 8 der älteren Modelle antworten.",
            )}
          </li>
          <li>
            {tb(
              "Kabel ohne Schalter oder KKL-Kabel: sie erreichen sicher die Steuergeräte an Pin 7 (meist Motor, Getriebe und ABS). Für Airbag, Kombi und Karosserie der älteren Modelle müssen Pin 7 und 8 verbunden sein, das steht in der Beschreibung des Kabels.",
            )}
          </li>
          <li>
            {tb(
              "Zündung an, Motor aus. Der Suchlauf fragt jede bekannte Adresse über DS2, KWP2000 und BMW-FAST und dauert etwa eine halbe Minute. Caruspex liest nur und löscht Fehler auf Nachfrage; Codieren und Anpassen gehören in die Werkstattsoftware.",
            )}
          </li>
          <li>
            {tb(
              "BMW ab 03/2007 und die F- und G-Reihe sprechen D-CAN oder Ethernet: dafür auf der Verbindungsseite einen ELM327 oder das Netzwerkkabel nehmen.",
            )}
          </li>
        </ul>
      </Note>

      <div className="ws-bar">
        <PortChoice caption={ts("Anschluss:")} options={options} value={port} onChange={setPort} disabled={running} />
        <Button icon="refresh" disabled={running} onClick={() => void refresh(port)}>
          {ts("Suchen")}
        </Button>
        <Button variant={running ? "default" : "primary"} icon={running ? "stop" : "search"} onClick={startOrCancel}>
          {running ? ts("Abbrechen") : ts("Durchsuchen")}
        </Button>
        <Button variant="danger" icon="trash" disabled={running} onClick={() => void clearEverywhere()}>
          {ts("Überall löschen")}
        </Button>
      </div>

      {progress ? <Progress value={progress.total > 0 ? (100 * Math.max(0, progress.value)) / progress.total : 0} /> : null}
      <div className={statusError ? "ws-message error wrap" : "subtle wrap"}>{status}</div>

      <div className="table-wrap">
        <table className="table scan-table bmw-table">
          <thead>
            <tr>
              <th>{ts("Steuergerät")}</th>
              <th>{tb("Protokoll")}</th>
              <th>{ts("Zustand")}</th>
              <th>{ts("Kennung")}</th>
              <th aria-hidden />
            </tr>
          </thead>
          <tbody>
            {units.length === 0 ? (
              <tr className="placeholder">
                <td colSpan={5}>{running ? tb("Noch keine Antwort ...") : tb("Noch keine Steuergeräte.")}</td>
              </tr>
            ) : null}
            {units.map((unit) => {
              const count = unit.faults.length;
              const state = !unit.faultsRead ? te("Fehlerspeicher nicht lesbar") : count === 0 ? ts("in Ordnung") : count === 1 ? ts("1 Fehler") : ts("%1 Fehler", count);
              return (
                <Fragment key={unitKey(unit)}>
                  <tr className="unit fade-in">
                    <td>
                      {unit.name}
                      <span className="faint ltr bmw-address"> 0x{unit.address.toString(16).toUpperCase().padStart(2, "0")}</span>
                    </td>
                    <td className="ltr">{unit.protocolLabel}</td>
                    <td className={!unit.faultsRead ? "state" : count === 0 ? "state ok" : "state bad"}>{state}</td>
                    <td className="ident">
                      <span className="ltr">{unit.identification}</span>
                      {unit.note ? <div className="hint">{unit.note}</div> : null}
                    </td>
                    <td className="bmw-action">
                      {count > 0 ? (
                        <Button variant="ghost" icon="trash" disabled={running} onClick={() => void clearUnit(unit)}>
                          {te("Fehler löschen")}
                        </Button>
                      ) : null}
                    </td>
                  </tr>
                  {unit.faults.map((fault, i) => (
                    <tr key={`${unitKey(unit)}-${fault.code}-${i}`} className="fault fade-in">
                      <td className="ltr">{fault.code}</td>
                      <td className="state">{faultState(fault.present)}</td>
                      <td colSpan={3}>
                        {fault.text}
                        {fault.detail ? <span className="faint ltr bmw-detail"> ({fault.detail})</span> : null}
                      </td>
                    </tr>
                  ))}
                </Fragment>
              );
            })}
          </tbody>
        </table>
      </div>
      <div className="hint">
        {tb("BMW-Fehlercodes stehen als Hexzahl da. Texte dazu veröffentlicht BMW nicht, die gibt es nur mit den Herstellerdaten.")}
      </div>
      {messageBox}
    </Page>
  );
}
