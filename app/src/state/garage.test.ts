// The way a connection went, kept per car and tried first next time.
import { describe, expect, it } from "vitest";
import type { GarageCar } from "../api";
import { garageStore, rememberConnection, rememberedRoute } from "./garage";

function car(vin: string, lastSeen: string, connection?: GarageCar["connection"]): GarageCar {
  return {
    vin,
    name: "",
    description: "",
    bodyStyle: 0,
    note: "",
    firstSeen: lastSeen,
    lastSeen,
    checks: [],
    trips: [],
    fuel: [],
    services: [],
    ...(connection ? { connection } : {}),
  };
}

describe("remembered connections", () => {
  it("picks the car seen last with a way of that kind", () => {
    const cars = [
      car("A", "2026-11-01T10:00:00", { kind: "serial", device: "COM4", baud: 38400, protocol: "6" }),
      car("B", "2026-11-10T10:00:00", { kind: "serial", device: "COM7", baud: 115200, protocol: "3" }),
      car("C", "2026-11-12T10:00:00", { kind: "ble", device: "AA:BB", name: "OBDLink CX" }),
      car("D", "2026-11-15T10:00:00"),
    ];
    expect(rememberedRoute(cars, "serial")?.device).toBe("COM7");
    expect(rememberedRoute(cars, "ble")?.name).toBe("OBDLink CX");
    expect(rememberedRoute(cars, "tcp")).toBeNull();
  });

  it("stores the way with the adapter, but never the demo car", () => {
    garageStore.set({ cars: [car("WVWZZZ1KZ6W000001", "2026-11-17T08:00:00")], loaded: true });
    rememberConnection("WVWZZZ1KZ6W000001", { kind: "demo", device: "" }, "ELM327 v1.5");
    expect(garageStore.get().cars[0]!.connection).toBeUndefined();
    rememberConnection("WVWZZZ1KZ6W000001", { kind: "serial", device: "COM7", baud: 38400, protocol: "6" }, "ELM327 v1.5");
    const stored = garageStore.get().cars[0]!.connection!;
    expect(stored.device).toBe("COM7");
    expect(stored.adapter).toBe("ELM327 v1.5");
    expect(stored.when).not.toBe("");
  });
});
