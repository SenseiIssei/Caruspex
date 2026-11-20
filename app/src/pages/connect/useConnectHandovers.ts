// The hand-overs MainWindow did, and the dev pages of the command line:
// auto-connect, the demo car, a car chosen elsewhere, the first-start
// assistant. They listen as soon as the page is mounted.
import type { RefObject } from "react";
import { call, type SerialPortInfo, type VehicleGenerationInfo } from "../../api";
import { t } from "../../i18n";
import { useAppEvent } from "../../lib/hooks";
import { navigate } from "../../shell/nav";
import type { PageId } from "../../shell/pages";
import { appendLog } from "../../state/session";
import type { SetupChoice } from "../../shell/SetupAssistant";
import type { ConnectLatest, Mode, StartConnect } from "./shared";

const MW = "caruspex::MainWindow";

export function useConnectHandovers({
  latest,
  refreshPorts,
  setPort,
  setMode,
  setAddress,
  setTruck,
  setGeneration,
  startConnect,
}: {
  latest: RefObject<ConnectLatest>;
  refreshPorts: () => Promise<SerialPortInfo[]>;
  setPort: (device: string) => void;
  setMode: (m: Mode) => void;
  setAddress: (address: string) => void;
  setTruck: (on: boolean) => void;
  setGeneration: (g: VehicleGenerationInfo | null, wanted: number) => void;
  startConnect: StartConnect;
}) {
  // --- App events (the hand-overs MainWindow did) ------------------------------------------------

  // Auto-connect from the settings: select the saved port and connect.
  useAppEvent<string>("connect:auto", (lastPort) => {
    void (async () => {
      const list = await refreshPorts();
      if (list.some((p) => p.device === lastPort)) {
        setPort(lastPort);
        appendLog(t(MW, "Verbinde selbsttätig mit %1, so eingestellt.", lastPort));
        setTimeout(() => startConnect({ port: lastPort }), 0);
      } else {
        appendLog(t(MW, "Selbsttätiges Verbinden: Anschluss %1 ist nicht da.", lastPort));
      }
    })();
  });

  // ?demo: straight to the built-in demo car (autoConnectDemo).
  useAppEvent("connect:demo", () => {
    setMode("demo");
    startConnect({ mode: "demo" });
  });

  // The automatic search found the adapter on this port.
  useAppEvent<string>("connect:port-found", (device) => {
    void refreshPorts().then((list) => {
      if (list.some((p) => p.device === device)) setPort(device);
    });
  });

  // A car chosen on the vehicles page.
  useAppEvent<{ generation: VehicleGenerationInfo; year: number }>("connect:generation", (choice) => {
    if (choice?.generation) setGeneration(choice.generation, choice.year);
  });

  // The first-start assistant hands over the adapter, the car and maybe
  // the wish to connect right away.
  useAppEvent<SetupChoice>("connect:setup", (choice) => {
    setMode(choice.mode);
    if (choice.generation) setGeneration(choice.generation, choice.year);
    // After the render, so the connection uses the new choice.
    if (choice.connect) setTimeout(() => startConnect({ mode: choice.mode }), 60);
  });

  // --page connect-help: a Wi-Fi attempt to a closed port of this computer,
  // to see the connection assistant without any adapter.
  useAppEvent<string>("dev:page", (name) => {
    if (name !== "connect-help") return;
    navigate("connect");
    setMode("wifi");
    setAddress("127.0.0.1:1");
    latest.current.address = "127.0.0.1:1";
    startConnect({ mode: "wifi" });
  });

  // --page truck-faults / truck-live / truck-health / truck-used: the demo
  // truck over SAE J1939, then its faults, its live values, its health
  // check or its used vehicle check (with the VIN of its papers).
  useAppEvent<string>("dev:page", (name) => {
    const pages: Record<string, PageId> = {
      "truck-faults": "faults",
      "truck-live": "live",
      "truck-health": "health",
      "truck-used": "used-car",
    };
    const page = pages[name];
    if (!page) return;
    navigate("connect");
    setMode("demo");
    setTruck(true);
    latest.current.truck = true;
    latest.current.truckRate = 0;
    startConnect({ mode: "demo" });
    setTimeout(() => {
      navigate(page);
      if (name === "truck-live") void call("start_live", { pids: [] });
      if (name === "truck-health") void call("run_health_check", { seconds: 8 });
      // The VIN of the demo truck (caruspex_demo::truck::TRUCK_VIN).
      if (name === "truck-used") void call("run_used_car_check", { papersVin: "YCXTR42H4NC051739" });
    }, 7000);
  });

  // --page connect-bmw: the way for BMW's E series with an E46 chosen.
  useAppEvent<string>("dev:page", (name) => {
    if (name !== "connect-bmw") return;
    navigate("connect");
    void call("catalog_search", { query: "BMW E46", limit: 1 }).then((found) => {
      if (found?.[0]) setGeneration(found[0], 2003);
      setMode("bmw");
    });
  });

  // Opens the Bluetooth choice and scans, without connecting.
  useAppEvent<string>("dev:page", (name) => {
    if (name !== "connect-ble") return;
    navigate("connect");
    setMode("ble");
    void call("scan_ble", { milliseconds: 5000 });
  });
}
