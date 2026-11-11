// "Was soll ich jetzt tun?": what the session knows about the car goes to
// the backend (crate caruspex-advice), the list in three steps comes back.
// Used by the page of its own and by the card on the start page.
import { useEffect, useState } from "react";
import { call } from "../api";
import type { AdviceInput, AdviceReport } from "../api";
import { live, readiness, session, useSession } from "./session";

// What the driver sees on the dashboard; OBD cannot tell.
export interface Observations {
  milFlashing: boolean;
  redWarningLamp: boolean;
}

export const NO_OBSERVATIONS: Observations = { milFlashing: false, redWarningLamp: false };

// Coolant, engine speed and voltage: the values the rules look at.
const LIVE_PIDS = [0x05, 0x0c, 0x42];

export function adviceInput(observations: Observations): AdviceInput {
  const s = session.get();
  return {
    brand: s.identity?.brand ?? "",
    faultsRead: s.haveFaults,
    faults: s.faults.map((f) => ({ code: f.code, kind: f.kind, stored: f.stored, ecu: f.ecu, text: f.text })),
    milOn: s.status ? s.status.mil : null,
    readiness: readiness(s.status),
    findings: s.healthFindings.map((f) => ({ id: f.id, level: f.level, title: f.title, explanation: f.explanation })),
    live: LIVE_PIDS.filter((pid) => live.has(pid)).map((pid) => ({ pid, value: live.get(pid)!.value })),
    ...observations,
  };
}

// The list for the car, recomputed when something about it changes and,
// while connected, every few seconds for the live values.
export function useAdvice(observations: Observations = NO_OBSERVATIONS): AdviceReport | null {
  const connected = useSession((s) => s.connected);
  const faults = useSession((s) => s.faults);
  const haveFaults = useSession((s) => s.haveFaults);
  const status = useSession((s) => s.status);
  const findings = useSession((s) => s.healthFindings);
  const identity = useSession((s) => s.identity);
  const [report, setReport] = useState<AdviceReport | null>(null);
  const [tick, setTick] = useState(0);

  useEffect(() => {
    if (!connected) return;
    const timer = setInterval(() => setTick((n) => n + 1), 4000);
    return () => clearInterval(timer);
  }, [connected]);

  const { milFlashing, redWarningLamp } = observations;
  useEffect(() => {
    let cancelled = false;
    void call("what_to_do", { input: adviceInput({ milFlashing, redWarningLamp }) })
      .then((r) => {
        if (!cancelled && r) setReport(r);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [connected, faults, haveFaults, status, findings, identity, milFlashing, redWarningLamp, tick]);

  return report;
}

// The note for the workshop: every point with what to tell them.
export function workshopNote(report: AdviceReport, heading: string): string {
  const lines = [heading, ""];
  for (const item of report.items) lines.push(`- ${item.workshop}`);
  return lines.join("\n");
}
