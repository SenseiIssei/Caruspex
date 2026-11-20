// One backend subscription per event name, however many pages listen, and
// never a doubled delivery while a subscription is still being set up or
// taken down.
import { describe, expect, it } from "vitest";
import { createEventHub } from "./hub";
import { createLazyBackend } from "./lazy";
import type { Backend } from "./contract";

// A stand-in for Tauri's listen(): answers when told to.
function fakeListen() {
  const live = new Map<number, { event: string; deliver: (p: unknown) => void }>();
  const pending: (() => void)[] = [];
  let next = 0;
  let made = 0;
  return {
    subscribe(event: string, deliver: (p: unknown) => void): Promise<() => void> {
      made++;
      const id = next++;
      return new Promise((resolve) => {
        pending.push(() => {
          live.set(id, { event, deliver });
          resolve(() => live.delete(id));
        });
      });
    },
    answer: () => pending.splice(0).forEach((run) => run()),
    emit(event: string, payload: unknown) {
      for (const l of [...live.values()]) if (l.event === event) l.deliver(payload);
    },
    open: () => live.size,
    made: () => made,
  };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("event hub", () => {
  it("shares one subscription between many handlers", async () => {
    const tauri = fakeListen();
    const hub = createEventHub(tauri.subscribe);
    const got: number[] = [];
    for (let i = 0; i < 10; i++) hub.on("live_value", () => got.push(i));
    tauri.answer();
    await settle();
    expect(tauri.made()).toBe(1);
    tauri.emit("live_value", {});
    expect(got).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
  });

  it("drops the subscription with the last handler, also before it arrived", async () => {
    const tauri = fakeListen();
    const hub = createEventHub(tauri.subscribe);
    const stop = hub.on("faults", () => undefined);
    stop();
    tauri.answer();
    await settle();
    expect(tauri.open()).toBe(0);
    const stop2 = hub.on("faults", () => undefined);
    tauri.answer();
    await settle();
    expect(tauri.open()).toBe(1);
    stop2();
    expect(tauri.open()).toBe(0);
  });

  it("does not deliver twice when a channel empties and fills before the old one is gone", async () => {
    const tauri = fakeListen();
    const hub = createEventHub(tauri.subscribe);
    hub.on("status", () => undefined)();
    let calls = 0;
    hub.on("status", () => calls++);
    tauri.answer();
    await settle();
    tauri.emit("status", {});
    expect(calls).toBe(1);
    expect(tauri.open()).toBe(1);
  });

  it("one failing handler does not keep the event from the others", async () => {
    const tauri = fakeListen();
    const hub = createEventHub(tauri.subscribe);
    let reached = false;
    const quiet = console.error;
    console.error = () => undefined;
    hub.on("busy", () => {
      throw new Error("page broke");
    });
    hub.on("busy", () => (reached = true));
    tauri.answer();
    await settle();
    tauri.emit("busy", {});
    console.error = quiet;
    expect(reached).toBe(true);
  });
});

describe("lazy backend", () => {
  it("handlers subscribed before the module arrived hear the first call's events", async () => {
    const listeners = new Set<(p: unknown) => void>();
    const real = {
      kind: "mock",
      call: async () => {
        for (const fn of listeners) fn("connected");
        return undefined;
      },
      on: (_event: string, fn: (p: unknown) => void) => {
        listeners.add(fn);
        return () => listeners.delete(fn);
      },
    } as unknown as Backend;
    let loads = 0;
    const lazy = createLazyBackend("mock", async () => {
      loads++;
      return real;
    });
    const call = lazy.call as (command: string) => Promise<unknown>;
    const heard: unknown[] = [];
    lazy.on("connected", (p) => heard.push(p));
    const gone = lazy.on("closed", () => heard.push("never"));
    gone();
    await call("connect_demo");
    expect(heard).toEqual(["connected"]);
    expect(listeners.size).toBe(1);
    await call("connect_demo");
    expect(loads).toBe(1);
  });
});
