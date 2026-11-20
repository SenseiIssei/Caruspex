// The adapter part of the connection page: the way to the car with its
// settings, and the lists of serial ports and CAN interfaces.
import { useCallback, useEffect, useState, type RefObject } from "react";
import { call, type SerialPortInfo } from "../../api";
import { translator } from "../../i18n";
import type { CanChoice, ConnectLatest, Mode } from "./shared";

const tr = translator("caruspex::ConnectPage");

export function useAdapterChoice(latest: RefObject<ConnectLatest>) {
  const [mode, setModeState] = useState<Mode>("cable");
  const [hasBle, setHasBle] = useState(true);

  // Cable
  const [ports, setPorts] = useState<SerialPortInfo[]>([]);
  const [port, setPort] = useState("");
  // Wi-Fi
  const [address, setAddress] = useState("192.168.0.10:35000");
  // Replay
  const [replayPath, setReplayPath] = useState("");
  const [replayRealTime, setReplayRealTime] = useState(false);
  // ENET
  const [enetHost, setEnetHost] = useState("");
  // CAN
  const [canDevices, setCanDevices] = useState<CanChoice[]>([]);
  const [canIndex, setCanIndex] = useState(0);
  const [canRate, setCanRate] = useState(500000);
  // CAN FD, off by default; only SocketCAN and J2534 devices offer it.
  const [canFd, setCanFd] = useState(false);
  // A truck or bus over SAE J1939, with the bit rate (0: 250, then 500).
  const [truck, setTruck] = useState(false);
  const [truckRate, setTruckRate] = useState(0);

  // --- Lists -----------------------------------------------------------------------------

  // Fills the port list and keeps the previous choice when it is still there.
  const refreshPorts = useCallback(async (): Promise<SerialPortInfo[]> => {
    let list: SerialPortInfo[] = [];
    try {
      list = await call("list_serial_ports");
    } catch {
      list = [];
    }
    setPorts(list);
    setPort((previous) => (previous && list.some((p) => p.device === previous) ? previous : ""));
    return list;
  }, []);

  // Lists the CAN interfaces of this computer again: J2534, SocketCAN, then
  // every serial port as a possible SLCAN stick.
  const refreshCanDevices = useCallback(async () => {
    const [j2534, vendors, socketCan, serial] = await Promise.all([
      call("list_j2534_devices").catch(() => []),
      call("list_can_vendor_channels").catch(() => []),
      call("list_socketcan_interfaces").catch(() => [] as string[]),
      call("list_serial_ports").catch(() => [] as SerialPortInfo[]),
    ]);
    const items: CanChoice[] = [];
    for (const d of j2534 ?? []) {
      let label = `J2534 · ${d.name}`;
      if (!d.usable) label += ` · ${tr("nur 32-Bit-Treiber")}`;
      else if (d.bridged) label += ` · ${tr("32-Bit-Treiber über Hilfsprogramm")}`;
      items.push({ label, spec: d.usable ? `j2534:${d.library}` : "" });
    }
    for (const v of vendors ?? []) items.push({ label: v.label, spec: v.spec });
    for (const name of socketCan ?? []) items.push({ label: `SocketCAN · ${name}`, spec: `socketcan:${name}` });
    for (const p of serial ?? []) items.push({ label: `SLCAN · ${p.device}`, spec: `slcan:${p.device}` });
    const { canDevices: before, canIndex: index } = latest.current;
    const previous = before[index]?.spec ?? "";
    const keep = items.findIndex((i) => i.spec === previous);
    setCanDevices(items);
    setCanIndex(keep >= 0 ? keep : 0);
  }, []);

  useEffect(() => {
    void refreshPorts();
    void refreshCanDevices();
    call("app_info")
      .then((info) => setHasBle(info?.hasBle ?? true))
      .catch(() => undefined);
  }, [refreshPorts, refreshCanDevices]);

  const setMode = (m: Mode) => setModeState(m);

  const pickReplay = async () => {
    const [recording, all] = tr("Caruspex-Aufnahme (*.jsonl);;Alle Dateien (*)").split(";;");
    const path = await call("pick_open_path", {
      title: tr("Aufnahme wählen"),
      filters: [
        { name: recording ?? "", extensions: ["jsonl"] },
        { name: all ?? "", extensions: ["*"] },
      ],
    }).catch(() => null);
    if (path) setReplayPath(path);
  };

  return {
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
  };
}

export type AdapterChoice = ReturnType<typeof useAdapterChoice>;
