// Demo answers for the KKL first test (worker/kkl_test.rs) in the browser:
// "sim:a2" plays a short, fixed run of the simulated A2 with the numbers
// the Rust simulator gives, "sim:a2:no-power" stands for a cable on USB
// only. Any other port answers like a port that does not open, since the
// browser has no serial ports. The texts stay German and English here; the
// real run brings them from the core in every language.
import type { KklTestReport, KklTestStep, KklWakeRow } from "../../types";
import { L } from "../core";
import { emit, link, wait, type PartialHandlers } from "../runtime";

const SIM = "sim:a2";

function cable(device: string): KklTestReport["cable"] {
  const simulated = device.startsWith(SIM);
  return {
    device,
    description: simulated ? `FTDI FT232R USB UART (${L("simuliert", "simulated")})` : device,
    vid: simulated ? 0x0403 : 0,
    pid: simulated ? 0x6001 : 0,
    chip: simulated ? "FTDI FT232R" : "",
    family: simulated ? "ftdi" : "none",
    familyName: simulated ? "FTDI" : L("kein USB-Chip erkannt", "no USB chip recognised"),
    latencyMs: simulated ? 1 : null,
    chipAdvice: simulated
      ? L("FTDI ist der Chip, mit dem KWP1281 am verlässlichsten läuft.", "FTDI is the chip KWP1281 runs most reliably with.")
      : "",
    latencyAdvice: "",
    simulated,
  };
}

function wake(address: number, unit: string, phases: [boolean, number][]): KklWakeRow {
  return {
    address,
    unit,
    attempt: 1,
    strategy: "standard",
    baud: 9600,
    phases: phases.map(([one, bits]) => ({ one, bits, measuredMs: bits * 200, targetMs: bits * 200, callMs: 0.4 })),
    worstErrorMs: 0.4,
    addressMs: 2000,
    onCallMs: 0.4,
    offCallMs: 0.4,
    addressEcho: 2,
    firstByte: "55",
    firstByteAfterMs: 42,
    keyBytes: "01 8A",
    replyAfterMs: 30.3,
    replyEcho: true,
    replyRoundTripMs: 2,
    verdict: "connected",
    headline: L("Das Steuergerät hat sich gemeldet", "The control unit answered"),
    error: "",
  };
}

function step(id: string, title: string, state: KklTestStep["state"], summary: string, lines: string[] = []): KklTestStep {
  return { id, title, state, summary, lines, log: [] };
}

function base(kind: "cable" | "car", device: string): KklTestReport {
  return {
    kind,
    version: "demo",
    platform: "browser",
    date: new Date().toISOString().slice(0, 10),
    device,
    cable: cable(device),
    echo: null,
    wakes: [],
    steps: [],
    verdict: { kind: "", ok: false, headline: "", advice: "", warnings: [] },
    cancelled: false,
  };
}

function portProblem(report: KklTestReport): KklTestReport {
  const error = L(
    "Im Browser gibt es keine seriellen Ports. Für den echten Test die App nehmen.",
    "The browser has no serial ports. Use the app for the real test.",
  );
  report.steps = [step("port", L("Port öffnen und Baudrate wechseln", "Open the port and switch the baud rate"), "fail", error)];
  report.verdict = {
    kind: "port-problem",
    ok: false,
    headline: L("Das Kabel lässt sich nicht so schalten, wie die K-Line es braucht", "The cable cannot be switched the way the K-line needs"),
    advice: "",
    warnings: [error],
  };
  return report;
}

async function play(report: KklTestReport, steps: KklTestStep[]): Promise<void> {
  link.cancel = false;
  emit("busy", { active: true });
  for (const s of steps) emit("kkl_test_step", { step: { ...s, state: "pending", summary: "", lines: [] } });
  for (const s of steps) {
    if (link.cancel) {
      report.cancelled = true;
      break;
    }
    emit("kkl_test_step", { step: { ...s, state: "running", summary: "", lines: [] } });
    await wait(450);
    emit("kkl_test_step", { step: s });
  }
  report.steps = steps;
  emit("busy", { active: false });
  emit("kkl_test_finished", { report });
}

async function cableCheck(device: string): Promise<void> {
  const report = base("cable", device);
  if (!device.startsWith(SIM)) return play(portProblem(report), []).then(() => undefined);
  const powered = device === SIM;
  report.echo = { sent: "55 AA 0F", received: powered ? "55 AA 0F" : "", state: powered ? "clean" : "none", roundTripMs: powered ? 2.1 : null };
  const steps = [
    step("port", L("Port öffnen und Baudrate wechseln", "Open the port and switch the baud rate"), "ok", L("Port offen, 9600 und 10400 Baud lassen sich setzen", "Port open, 9600 and 10400 baud can be set")),
    step(
      "echo",
      L("Echo der K-Line", "Echo of the K-line"),
      "ok",
      powered
        ? L("Das Kabel hört sich selbst: es steckt an einer K-Line mit Strom", "The cable hears itself: it is on a powered K-line")
        : L("Kein Echo, so soll es ohne Auto sein", "No echo, as it should be without the car"),
    ),
  ];
  if (!powered) {
    const w = wake(1, L("01 Motor", "01 Engine"), [[false, 1], [true, 1], [false, 7], [true, 1]]);
    w.firstByte = "";
    w.firstByteAfterMs = null;
    w.keyBytes = "";
    w.replyAfterMs = null;
    w.replyEcho = null;
    w.replyRoundTripMs = null;
    w.verdict = "cable-ready";
    w.headline = L("Kabel in Ordnung, bereit für das Auto", "Cable OK, ready for the car");
    report.wakes = [w];
    steps.push(step("break", L("Break-Leitung und Bitzeiten", "Break line and bit times"), "ok", w.headline));
  }
  report.verdict = powered
    ? { kind: "cable-on-car", ok: true, headline: steps[1]!.summary, advice: "", warnings: [] }
    : {
        kind: "cable-ready",
        ok: true,
        headline: L("Kabel in Ordnung, bereit für das Auto", "Cable OK, ready for the car"),
        advice: L(
          "Ohne Auto kommt kein Echo zurück, das ist richtig so. Als Nächstes das Kabel ins Auto stecken und den ersten Test starten.",
          "Without the car no echo comes back, which is how it should be. Next, plug the cable into the car and start the first test.",
        ),
        warnings: [],
      };
  await play(report, steps);
}

async function firstTest(device: string): Promise<void> {
  const report = base("car", device);
  if (device !== SIM) return play(portProblem(report), []).then(() => undefined);
  report.echo = { sent: "55 AA 0F", received: "55 AA 0F", state: "clean", roundTripMs: 2.1 };
  const connected = L("Das Steuergerät hat sich gemeldet", "The control unit answered");
  const none = L("Kein Eintrag im Fehlerspeicher", "No entries in the fault memory");
  const one = L("1 Eintrag im Fehlerspeicher", "1 entry in the fault memory");
  const kwp = L("KWP1281 (Blockprotokoll)", "KWP1281 (block protocol)");
  report.wakes = [
    wake(0x01, L("01 Motor", "01 Engine"), [[false, 1], [true, 1], [false, 7], [true, 1]]),
    wake(0x17, L("17 Kombiinstrument", "17 Instrument cluster"), [[false, 1], [true, 3], [false, 1], [true, 1], [false, 2], [true, 2]]),
  ];
  const steps = [
    step("cable", L("Kabel und Echo", "Cable and echo"), "ok", L("Das Kabel hört sich selbst, die K-Line hat Strom", "The cable hears itself, the K-line has power"), ["gesendet 55 AA 0F, zurück 55 AA 0F"]),
    step("wake", L("Motor wecken (Adresse 01)", "Wake the engine (address 01)"), "ok", connected),
    step("ident", L("Kennung lesen", "Read the identification"), "ok", kwp, ["036906034AM", "MARELLI 4LV", "1,4l R4/4V      3170"]),
    step("faults", L("Fehlerspeicher lesen", "Read the fault memory"), "ok", one, ["16684 (sporadic): P0300"]),
    step("groups", L("Messwertblöcke 001 bis 003", "Measuring blocks 001 to 003"), "ok", L("3 von 3 Blöcken gelesen", "3 of 3 blocks read"), [
      "001: 800 1/min | 90 deg C | 0.98 % | 111",
      "002: 800 1/min | 20 % | 2.50 ms | 3.55 g/s",
      "003: 800 1/min | 3.55 g/s | 3.12 % | 12 deg BTDC",
    ]),
    step("unit-17", L("17 Kombiinstrument", "17 Instrument cluster"), "ok", `${kwp}, ${none}`, ["8Z0920900F  KOMBI+WEGFAHRSP VDO V05", "WAUZZZ8Z01N000417", "AUZ7Z0A0000417"]),
    step("unit-03", L("03 Bremsenelektronik (ABS)", "03 Brake electronics (ABS)"), "ok", `${kwp}, ${none}`),
    step("unit-15", L("15 Airbag", "15 Airbags"), "ok", `${kwp}, ${one}`),
    step("unit-46", L("46 Komfortsystem", "46 Comfort system"), "ok", `${kwp}, ${none}`),
    step("unit-09", L("09 Zentralelektrik", "09 Central electrics"), "warn", L("meldet sich nicht, vielleicht nicht verbaut", "does not answer, maybe not fitted")),
  ];
  report.verdict = {
    kind: "connected",
    ok: true,
    headline: connected,
    advice: L("Erster Test fertig: 5 von 6 Steuergeräten haben geantwortet.", "First test done: 5 of 6 control units answered."),
    warnings: [],
  };
  await play(report, steps);
}

export const kklTestHandlers: PartialHandlers = {
  kkl_cable_check: ({ device }) => {
    void cableCheck(device);
  },
  kkl_first_test: ({ device }) => {
    void firstTest(device);
  },
  kkl_test_save_report: () => {
    const folder = "%APPDATA%/dev.senseiissei.caruspex/test-reports";
    return { folder, json: `${folder}/kkl-car-demo-1.json`, text: `${folder}/kkl-car-demo-1.txt` };
  },
  kkl_test_reveal: () => undefined,
};
