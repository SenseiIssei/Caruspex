// The recording of the live page keeps every reading for the CSV export, in
// a fraction of the memory objects took.
import { describe, expect, it } from "vitest";
import { SampleLog } from "./sampleLog";

describe("SampleLog", () => {
  it("gives back every reading in order, across growing", () => {
    const log = new SampleLog();
    for (let i = 0; i < 5000; i++) log.push(i / 10, i % 3, `name ${i % 3}`, i * 1.5, i % 3 === 0 ? "km/h" : "°C");
    expect(log.length).toBe(5000);
    expect(log.at(0)).toEqual({ seconds: 0, pid: 0, name: "name 0", value: 0, unit: "km/h" });
    expect(log.at(4998)).toEqual({ seconds: 499.8, pid: 0, name: "name 0", value: 7497, unit: "km/h" });
    expect(log.at(2501)).toEqual({ seconds: 250.1, pid: 2, name: "name 2", value: 3751.5, unit: "°C" });
  });

  it("keeps a new name or unit of the same key", () => {
    const log = new SampleLog();
    log.push(1, 5, "Coolant", 80, "°C");
    log.push(2, 5, "Coolant", 176, "°F");
    log.push(3, 5, "Coolant", 81, "°C");
    expect([0, 1, 2].map((i) => log.at(i).unit)).toEqual(["°C", "°F", "°C"]);
  });

  it("an hour at a hundred readings a second stays near 10 MB", () => {
    const log = new SampleLog();
    for (let i = 0; i < 360_000; i++) log.push(i / 100, i % 8, "x", i, "");
    expect(log.length).toBe(360_000);
    expect(log.bytes()).toBeLessThan(11 * 1024 * 1024);
  });

  it("clear starts again and gives a long recording's memory back", () => {
    const log = new SampleLog();
    for (let i = 0; i < 10_000; i++) log.push(i, 1, "a", i, "");
    log.clear();
    expect(log.length).toBe(0);
    expect(log.bytes()).toBeLessThan(64 * 1024);
    log.push(1, 2, "b", 3, "V");
    expect(log.at(0)).toEqual({ seconds: 1, pid: 2, name: "b", value: 3, unit: "V" });
  });
});
