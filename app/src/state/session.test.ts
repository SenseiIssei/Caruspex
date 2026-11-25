// The session store follows the events of the backend, here the demo
// backend of the browser preview.
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { call, on } from "../api";
import type { LiveRow } from "../api";
import { emit } from "../api/mock/runtime";
import { live, liveFrames, session, startSession } from "./session";

function row(pid: number, value: number): LiveRow {
  return { pid, name: "", unit: "", text: "", value, warn: false, time: 0, displayMin: 0, displayMax: 0 };
}

beforeAll(async () => {
  startSession();
  // The demo backend loads on first use; its events arrive from then on.
  await call("list_serial_ports").catch(() => undefined);
});

beforeEach(() => emit("closed", {}));

describe("the state of the connection", () => {
  const connect = () => emit("connected", { adapter: "ELM327 v1.5", transport: "COM5", protocol: "CAN" });

  it("shows a lost link as an error and forgets the car", () => {
    connect();
    expect(session.get().link).toBe("online");
    emit("live_values", { rows: [row(0x0c, 800)] });
    emit("closed", { lost: "The connection to the adapter broke off." });
    expect(session.get().link).toBe("error");
    expect(session.get().connected).toBe(false);
    expect(session.get().linkText).not.toBe("");
    expect(live.size).toBe(0);
  });

  it("shows a disconnect somebody asked for as offline", () => {
    connect();
    emit("closed", {});
    expect(session.get().link).toBe("offline");
  });
});

describe("live values", () => {
  it("takes every value of a tick and tells the pages once", () => {
    const marked = vi.spyOn(liveFrames, "mark");
    emit("live_values", { rows: [row(0x0c, 800), row(0x0d, 50), row(0x0c, 810)] });
    expect(live.get(0x0c)?.value).toBe(810);
    expect(live.get(0x0d)?.value).toBe(50);
    expect(marked).toHaveBeenCalledTimes(3);
    marked.mockRestore();
  });

  it("come as one event per tick from the demo car", async () => {
    const ticks: number[] = [];
    const stop = on("live_values", ({ rows }) => ticks.push(rows.length));
    await call("open_demo", { protocolOrder: "6" });
    await vi.waitFor(() => expect(session.get().connected).toBe(true), 5000);
    ticks.length = 0;
    await call("start_live", { pids: [0x0c, 0x0d, 0x05, 0x11, 0x04, 0x0f] });
    await vi.waitFor(() => expect(ticks.filter((n) => n >= 6).length).toBeGreaterThan(2), 5000);
    await call("close_adapter");
    stop();
    expect(session.get().connected).toBe(false);
  });
});

describe("cancelling a connection attempt", () => {
  it("reports closed and no failure", async () => {
    const failures: string[] = [];
    let closed = 0;
    const stops = [on("failed", ({ error }) => failures.push(error)), on("closed", () => closed++)];
    void call("open_tcp", { host: "192.168.0.10", port: 35000, protocolOrder: "6" });
    await new Promise((resolve) => setTimeout(resolve, 200));
    await call("close_adapter");
    // The attempt would have failed after 0.9 s.
    await new Promise((resolve) => setTimeout(resolve, 1200));
    expect(failures).toEqual([]);
    expect(closed).toBe(1);
    stops.forEach((stop) => stop());
  });
});
