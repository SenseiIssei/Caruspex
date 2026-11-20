// Bluetooth on the connection page: the devices found, the scan, the
// remembered adapter and, on Android, the permission.
import { useEffect, useState } from "react";
import { call, type BleDeviceRow, type BluetoothAccess } from "../../api";
import { translator } from "../../i18n";
import { useBackendEvent } from "../../lib/hooks";
import { isAndroid } from "../../lib/platform";
import { BLE_KEYS, autoBle, initialBle, writeStored, type BleChoice, type Mode } from "./shared";

const tr = translator("caruspex::ConnectPage");
// Texts that came with Bluetooth LE on Android.
const tb = translator("caruspex::Bluetooth");

export function useBluetooth(mode: Mode) {
  const [ble, setBle] = useState(initialBle);
  const [bleScanning, setBleScanning] = useState(false);
  // The same on the phone: Bluetooth LE through Android's own GATT, classic
  // adapters once paired in the settings.
  const [bleNote, setBleNote] = useState(() =>
    tr(
      "Adapter mit Bluetooth LE (Vgate iCar Pro BLE, vLinker, OBDLink CX, " +
        "Veepeak BLE) findet Caruspex direkt, ohne Koppeln. Klassische " +
        "Bluetooth-Adapter einmal in den Systemeinstellungen koppeln, die PIN " +
        "ist meist 1234 oder 0000, danach stehen sie hier in der Liste.",
    ),
  );
  // Android: may the app use Bluetooth, and whether the user was asked.
  const [btAccess, setBtAccess] = useState<BluetoothAccess | null>(null);
  const [btAsked, setBtAsked] = useState(false);

  // Result of a Bluetooth scan (Worker::bleScanFinished).
  useBackendEvent("ble_scan_finished", ({ devices, error: scanError }) => {
    setBleScanning(false);
    const items: BleChoice[] = [autoBle()];
    let adapters = 0;
    for (const d of devices as BleDeviceRow[]) {
      const adapter = d.score >= 3;
      if (adapter) adapters++;
      const kind = d.kinds || 1;
      let label = d.name ? d.name : tr("Unbenanntes Gerät (%1)", d.id);
      label += `  ·  ${kind === 2 ? tr("klassisch") : kind === 3 ? tr("LE und klassisch") : tr("LE")}`;
      if (adapter) label += `  ·  ${tr("OBD-Adapter")}`;
      // A known serial service is a hint only: lamps use the same modules.
      else if (d.profile) label += `  ·  ${tb("Serieller Dienst %1", d.profile)}`;
      if (d.rssi) label += `  ·  ${d.rssi} dBm`;
      items.push({ id: d.id, name: d.name, kinds: kind, label });
    }
    setBle((before) => {
      const keep = before.selected && items.some((i) => i.id === before.selected) ? before.selected : null;
      return { items, selected: keep ?? (adapters > 0 ? items[1]!.id : "") };
    });
    if (scanError) {
      setBleNote(scanError);
    } else if (devices.length === 0) {
      setBleNote(
        tr(
          "Kein Bluetooth-Gerät gefunden. Steckt der Adapter im Auto, und " +
            "ist die Zündung an? Manche Adapter schlafen ein und wachen erst nach " +
            "einem Druck auf ihren Knopf wieder auf.",
        ),
      );
    } else {
      const count = devices.length === 1 ? tr("Ein Gerät gefunden") : tr("%1 Geräte gefunden", devices.length);
      setBleNote(
        adapters === 0
          ? tr("%1, keins davon sieht nach einem OBD-Adapter aus. Unbekannte " + "Adapter lassen sich trotzdem wählen.", count)
          : tr("%1, davon %2 als OBD-Adapter erkannt.", count, adapters),
      );
    }
  });

  // Remembers the device the Bluetooth connection went to.
  useBackendEvent("ble_adapter_found", ({ id, name, kinds }) => {
    writeStored(BLE_KEYS.id, id);
    writeStored(BLE_KEYS.name, name);
    writeStored(BLE_KEYS.kinds, String(kinds));
    setBle((before) => {
      const items = before.items.some((i) => i.id === id) ? before.items : [...before.items, { id, name, kinds, label: name || id }];
      return { items, selected: id };
    });
  });

  const scanBle = () => {
    setBleScanning(true);
    setBleNote(tr("Suche Bluetooth-Geräte, das dauert bis zu einer halben Minute ..."));
    void call("scan_ble", { milliseconds: 8000 });
  };

  // Android: the permission state each time the Bluetooth choice opens.
  useEffect(() => {
    if (!isAndroid || mode !== "ble") return;
    void call("bluetooth_access")
      .then(setBtAccess)
      .catch(() => undefined);
  }, [mode]);
  const askBluetooth = async () => {
    setBtAsked(true);
    const access = await call("request_bluetooth_access").catch(() => null);
    if (!access) return;
    setBtAccess(access);
    if (access.granted) scanBle();
  };
  const btBlocked = isAndroid && !!btAccess && btAccess.needsPermission && !btAccess.granted;
  const locationNeeded = isAndroid && !btBlocked && !!btAccess?.locationNeeded;

  return { ble, setBle, bleScanning, bleNote, btAsked, scanBle, askBluetooth, btBlocked, locationNeeded };
}

export type Bluetooth = ReturnType<typeof useBluetooth>;
