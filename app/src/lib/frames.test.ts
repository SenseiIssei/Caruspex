// Live values arrive far more often than the screen changes: listeners hear
// about them once per frame, together, and only for keys they show.
import { describe, expect, it } from "vitest";
import { FrameNotifier } from "./frames";

function manual() {
  const queue: (() => void)[] = [];
  return {
    schedule: (run: () => void) => queue.push(run),
    frame: () => queue.splice(0).forEach((run) => run()),
    queued: () => queue.length,
  };
}

describe("FrameNotifier", () => {
  it("a hundred readings in one frame are one call", () => {
    const clock = manual();
    const notifier = new FrameNotifier<number>(clock.schedule);
    let calls = 0;
    notifier.subscribe([0x0c, 0x0d], () => calls++);
    for (let i = 0; i < 100; i++) notifier.mark(i % 2 ? 0x0c : 0x0d);
    expect(clock.queued()).toBe(1);
    expect(calls).toBe(0);
    clock.frame();
    expect(calls).toBe(1);
    clock.frame();
    expect(calls).toBe(1);
  });

  it("only listeners of a changed key are called", () => {
    const clock = manual();
    const notifier = new FrameNotifier<number>(clock.schedule);
    const seen: string[] = [];
    notifier.subscribe([1], () => seen.push("one"));
    notifier.subscribe([2], () => seen.push("two"));
    notifier.mark(2);
    clock.frame();
    expect(seen).toEqual(["two"]);
  });

  it("a key nobody shows schedules nothing", () => {
    const clock = manual();
    const notifier = new FrameNotifier<number>(clock.schedule);
    notifier.mark(7);
    expect(clock.queued()).toBe(0);
  });

  it("markAll reaches every listener once, and unsubscribing leaves nothing behind", () => {
    const clock = manual();
    const notifier = new FrameNotifier<number>(clock.schedule);
    let a = 0;
    let b = 0;
    const stopA = notifier.subscribe([1, 2, 3], () => a++);
    const stopB = notifier.subscribe([3], () => b++);
    notifier.markAll();
    clock.frame();
    expect([a, b]).toEqual([1, 1]);
    expect(notifier.listeners()).toBe(4);
    stopA();
    stopB();
    expect(notifier.listeners()).toBe(0);
    notifier.mark(3);
    expect(clock.queued()).toBe(0);
  });
});
