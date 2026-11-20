// Browser preview of the tuning simulator. The real model is Rust
// (crates/caruspex-tune); this stand-in only draws the published stock curve
// of the N55 and scales it roughly with the offsets, so the page can be
// looked at without the app. Warnings and numbers here are not the model's.
import type { TuneComparison, TuneCurve, TuneInput, TunePoint, TuneResult, TuneWarning } from "../../types";
import type { PartialHandlers } from "../runtime";

const PS = 735.49875;

// 400 Nm from 1,200 to 5,000 rpm, 225 kW at 5,800 to 6,000 rpm (Wikipedia
// "BMW N55", BMW's data for the 435i), the rest as in spec.rs.
function stockTorque(rpm: number): number {
  if (rpm < 1200) return 288 + (400 - 288) * ((rpm - 1000) / 200);
  if (rpm <= 5000) return 400;
  const kw = rpm <= 5800 ? 209.44 + (225 - 209.44) * ((rpm - 5000) / 800) : rpm <= 6000 ? 225 : 225 - 20 * ((rpm - 6000) / 1000);
  return (kw * 1000) / ((rpm * 2 * Math.PI) / 60);
}

function offsetsAt(input: TuneInput, rpm: number) {
  const band = input.bands.find((b) => rpm >= b.fromRpm && (rpm < b.toRpm || (rpm === 7000 && b.toRpm === 7000)));
  return band ?? { boostBar: 0, ignitionDeg: 0, lambda: 0 };
}

function curve(input: TuneInput | null): TuneCurve {
  const points: TunePoint[] = [];
  for (let rpm = 1000; rpm <= 7000; rpm += 250) {
    const o = input ? offsetsAt(input, rpm) : { boostBar: 0, ignitionDeg: 0, lambda: 0 };
    const factor = (1 + o.boostBar / 1.75) * (1 + 0.012 * Math.min(o.ignitionDeg, 1.5));
    const torque = stockTorque(rpm) * factor;
    const kw = (torque * rpm * 2 * Math.PI) / 60 / 1000;
    points.push({
      rpm,
      torqueNm: torque,
      powerKw: kw,
      powerPs: (kw * 1000) / PS,
      boostTargetBar: 0.72 + o.boostBar,
      boostBar: 0.72 + o.boostBar,
      manifoldKpa: 173 + o.boostBar * 100,
      boostCap: "none",
      chargeAirC: 38 + o.boostBar * 40,
      airKgH: 2.9 * kw + 50,
      pressureRatio: 1.75 + o.boostBar,
      turboLoad: 0.7 + o.boostBar * 0.4,
      lambda: 1 + o.lambda,
      timingRequestedDeg: 6 + o.ignitionDeg,
      timingDeg: 6 + Math.min(o.ignitionDeg, 1.5),
      knockMarginDeg: 1.5 - o.ignitionDeg - o.boostBar * 6,
      egtC: 860 + o.boostBar * 60,
      fuelKgH: 0.08 * kw,
      pumpLoad: kw / 290,
      injectorLoad: kw / 370,
      bmepBar: (torque * 4 * Math.PI) / 2.979e-3 / 1e5,
      gearboxLoad: torque / (input?.gearbox === "manual" ? 450 : 450),
    });
  }
  const warnings: TuneWarning[] = [];
  const over = points.filter((p) => p.torqueNm > 450);
  if (over.length) {
    warnings.push({
      kind: "gearbox",
      severity: "warning",
      title: "Getriebe",
      reason: "Browser-Vorschau: mehr als 450 Nm.",
      fromRpm: over[0]!.rpm,
      toRpm: over[over.length - 1]!.rpm,
    });
  }
  const peakT = points.reduce((a, b) => (b.torqueNm > a.torqueNm ? b : a));
  const peakP = points.reduce((a, b) => (b.powerKw > a.powerKw ? b : a));
  return {
    points,
    warnings,
    peakTorque: { rpm: peakT.rpm, nm: peakT.torqueNm },
    peakPower: { rpm: peakP.rpm, kw: peakP.powerKw, ps: peakP.powerPs },
  };
}

function simulate(input: TuneInput): TuneResult {
  return {
    stock: curve(null),
    changed: curve(input),
    micros: 0,
    uncertainty: { torqueShare: 0.08, boostBar: 0.1, chargeAirK: 10, knockDeg: 3, egtK: 60 },
  };
}

function example(): TuneComparison {
  const pair = (measured: number, simulated: number) => ({
    measured,
    simulated,
    difference: simulated - measured,
    deviationPct: ((simulated - measured) / measured) * 100,
  });
  const bands = [1500, 2500, 3500, 4500, 5500].map((from) => ({
    fromRpm: from,
    toRpm: from + 500,
    samples: 4,
    manifoldKpa: pair(177, 173),
    chargeAirC: pair(43, 38),
    airGs: pair(150, 155),
    timingDeg: pair(5, 5.5),
    lambda: pair(1.01, 1),
  }));
  return { file: "", total: 48, used: 42, partLoad: 6, outOfRange: 0, bands };
}

export const tuneHandlers: PartialHandlers = {
  tune_simulate: ({ input }) => simulate(input),
  tune_compare_example: () => example(),
  tune_compare_log: () => {
    throw new Error("Browser-Vorschau: Mitschnitte vergleicht nur die App.");
  },
};
