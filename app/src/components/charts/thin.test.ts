// A synthetic hour of live values: the chart keeps a few thousand points per
// curve instead of tens of thousands, and no peak gets lost.
import { describe, expect, it } from "vitest";
import { SeriesStore, THIN_TRIGGER, thinTrack } from "./TimeSeriesChart";

const info = { name: "x", unit: "", displayMin: 0, displayMax: 0 };

describe("thinning long recordings", () => {
  it("an hour at ten readings a second stays small and keeps its peaks", () => {
    const store = new SeriesStore();
    store.setWindow(3600);
    const tracks = 8;
    const rate = 10;
    const started = performance.now();
    let lowest = Infinity;
    let highest = -Infinity;
    for (let n = 0; n < 3600 * rate; n++) {
      const t = n / rate;
      for (let k = 0; k < tracks; k++) {
        // A slow wave with noise and a rare spike, like a coolant or a boost value.
        let v = 50 + 30 * Math.sin(t / (60 + k * 7)) + 5 * Math.sin(n * 0.37 + k);
        if (k === 0 && n === 12345) v = 400;
        if (k === 0 && n === 23456) v = -80;
        if (k === 0) {
          lowest = Math.min(lowest, v);
          highest = Math.max(highest, v);
        }
        store.add(k, t, v, info);
      }
    }
    const elapsed = performance.now() - started;
    const first = store.tracks.get(0)!;
    // 36000 readings per curve without thinning.
    expect(first.times.length).toBeLessThanOrEqual(THIN_TRIGGER);
    expect(first.times.length).toBeGreaterThan(1500);
    expect(Math.max(...first.values)).toBe(highest);
    expect(Math.min(...first.values)).toBe(lowest);
    // The start of the hour is still there, at the width of one bucket.
    expect(first.times[0]).toBeLessThan(3600 / 1000);
    expect(first.times[first.times.length - 1]).toBeCloseTo(3599.9);
    for (let i = 1; i < first.times.length; i++) expect(first.times[i]!).toBeGreaterThan(first.times[i - 1]!);
    // 288000 readings in well under a few seconds, even on a slow runner.
    expect(elapsed).toBeLessThan(5000);
  });

  it("the newest readings stay untouched", () => {
    const track = { times: [] as number[], values: [] as number[] };
    for (let i = 0; i < 4000; i++) {
      track.times.push(i * 0.1);
      track.values.push(i % 7);
    }
    const tail = track.values.slice(-500);
    thinTrack(track, 3600);
    expect(track.values.slice(-500)).toEqual(tail);
    expect(track.times.length).toBeLessThan(4000);
  });

  it("a short window is not thinned at all", () => {
    const store = new SeriesStore();
    store.setWindow(60);
    for (let n = 0; n < 6000; n++) store.add(1, n / 10, n % 13, info);
    const track = store.tracks.get(1)!;
    // 60 s at 10 per second, and every one of them.
    expect(track.times.length).toBeGreaterThanOrEqual(600);
    expect(track.times.length).toBeLessThanOrEqual(602);
  });
});
