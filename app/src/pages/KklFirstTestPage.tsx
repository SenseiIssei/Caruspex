// "Erster Test am Auto": the first session of a KKL cable on a car with a
// K-line, laid out for the Audi A2 (worker/kkl_test.rs). First the cable on
// USB only (chip, latency timer, echo, bit times of the break line), then a
// checklist and the guided run: wake the engine with the timing measured,
// identification, fault memory, measuring blocks 001 to 003, then 17, 03,
// 15, 46 and 09. Nothing is written: no clearing, no output test. At the end
// the report goes into a file in the config folder, and the test report form
// on GitHub can be opened with the summary filled in; the VIN only when the
// box is ticked. The port is opened only when a test starts.
import { useCallback, useEffect, useRef, useState } from "react";
import { call } from "../api";
import type { KklSavedReport, KklStepState, KklTestReport, KklTestStep, KklWakeRow } from "../api";
import { Icon, type IconName } from "../components/Icon";
import { toast } from "../components/Toast";
import { Button, Card, Check, Note, Page, PageHeader, Select, Spinner, type Option } from "../components/ui";
import { translator } from "../i18n";
import { useAppEvent, useBackendEvent } from "../lib/hooks";
import { navigate } from "../shell/nav";
import { settingsStore } from "../state/session";
import { PortChoice } from "./WorkshopShared";
import "./KklFirstTestPage.css";

const tr = translator("caruspex::KklFirstTestPage");

// worker/kkl_test.rs: the simulated A2 of caruspex_demo::a2, with a fault
// after a colon for the dev pages ("sim:a2:slow-read").
export const A2_SIM = "sim:a2";
const ISSUE_PAGE = "https://github.com/SenseiIssei/Caruspex/issues/new";

type Variant = "a2-14" | "a2-16fsi" | "other";

// The variant in the words of the report form, in English for the
// maintainers.
const VARIANT_FORM: Record<Variant, { car: string; year: string; engine: string }> = {
  "a2-14": { car: "Audi A2 (8Z)", year: "2001", engine: "1.4 petrol" },
  "a2-16fsi": { car: "Audi A2 (8Z)", year: "2002", engine: "1.6 FSI petrol" },
  other: { car: "", year: "", engine: "" },
};

const STATE_ICON: Record<KklStepState, IconName> = {
  pending: "clock",
  running: "clock",
  ok: "check-circle",
  warn: "alert",
  fail: "x-circle",
  skip: "chevron-right",
};

// The same masking as kkl_report.rs: a VIN keeps 11 characters, a VAG
// immobiliser id 7.
export function maskPrivate(text: string): string {
  return text.replace(/[A-Z0-9]+/g, (run) => {
    const digits = (run.match(/[0-9]/g) ?? []).length;
    const letters = (run.match(/[A-Z]/g) ?? []).length;
    if (run.length === 17 && /^[A-HJ-NPR-Z0-9]+$/.test(run) && digits > 0 && letters > 0) {
      return run.slice(0, 11) + "*".repeat(6);
    }
    if (run.length === 14 && /^[A-Z]{2}Z/.test(run) && digits >= 6) return run.slice(0, 7) + "*".repeat(7);
    return run;
  });
}

function findVin(report: KklTestReport): string {
  for (const s of report.steps) {
    for (const line of s.lines) {
      const hit = line.match(/\b[A-HJ-NPR-Z0-9]{17}\b/);
      if (hit && /[0-9]/.test(hit[0]) && /[A-Z]/.test(hit[0])) return hit[0];
    }
  }
  return "";
}

// The body of the issue: what ran, the verdict and the timing in short.
export function issueUrl(report: KklTestReport, variant: Variant, includeVin: boolean, savedText: string): string {
  const form = VARIANT_FORM[variant];
  const clean = (text: string) => (includeVin ? text : maskPrivate(text));
  const lines = [
    `KKL ${report.kind === "cable" ? "cable test" : "first test on the car"} from Caruspex ${report.version}.`,
    "",
    `Result: ${report.verdict.kind} (${report.verdict.headline})`,
  ];
  for (const w of report.verdict.warnings) lines.push(`! ${w}`);
  lines.push("");
  for (const s of report.steps) {
    lines.push(`[${s.state.toUpperCase()}] ${s.title}: ${clean(s.summary)}`);
    if (s.state === "ok" && (s.id === "ident" || s.id.startsWith("unit-"))) {
      for (const line of s.lines.slice(0, 4)) lines.push(`    ${clean(line)}`);
    }
  }
  const timing = report.wakes.slice(0, 8).map(
    (w) =>
      `- ${w.address.toString(16).padStart(2, "0").toUpperCase()} #${w.attempt} ${w.strategy} ${w.baud}: ${w.verdict}, edge error ${w.worstErrorMs} ms, ` +
      `sync ${w.firstByteAfterMs ?? "-"} ms, reply ${w.replyAfterMs ?? "-"} ms`,
  );
  if (timing.length > 0) lines.push("", "Wake-up timing:", ...timing);
  if (savedText) lines.push("", `Please drag the file ${savedText.split(/[\\/]/).pop()} into this field so it is attached.`);
  const vin = includeVin ? findVin(report).slice(0, 11) : "";
  const latency = report.cable.latencyMs !== null ? `, latency timer ${report.cable.latencyMs} ms` : "";
  const params: [string, string][] = [
    ["template", "test-report.yml"],
    ["title", `Test report: ${form.car || "car with K-line"}, KKL ${report.kind === "cable" ? "cable test" : "first test"}`],
    ["car", form.car],
    ["year", form.year],
    ["engine", form.engine],
    ["vin", vin],
    ["interface", "KKL cable (VAG 409.1)"],
    ["adapter", `${report.cable.description} (${report.cable.familyName}${latency})`],
    ["system", `${report.version} on ${report.platform}`],
    ["details", lines.join("\n").slice(0, 5000)],
  ];
  return `${ISSUE_PAGE}?${params
    .filter(([, v]) => v)
    .map(([k, v]) => `${k}=${encodeURIComponent(v)}`)
    .join("&")}`;
}

function StepRow({ step }: { step: KklTestStep }) {
  const details = step.lines.length > 0 || step.log.length > 0;
  const head = (
    <>
      <span className={`kft-icon st-${step.state}`}>
        {step.state === "running" ? <Spinner size={16} /> : <Icon name={STATE_ICON[step.state]} size={16} />}
      </span>
      <span className="kft-title">{step.title}</span>
      <span className="kft-summary">{step.summary}</span>
    </>
  );
  if (!details) return <div className={`kft-step st-${step.state}`}>{head}</div>;
  return (
    <details className={`kft-step st-${step.state}`} open={step.state === "fail"}>
      <summary>{head}</summary>
      {step.lines.length > 0 ? <div className="kft-lines ltr">{step.lines.join("\n")}</div> : null}
      {step.log.length > 0 ? (
        <>
          <div className="kft-log-title subtle">{tr("Rohprotokoll")}</div>
          <div className="ws-log kft-log ltr">{step.log.join("\n")}</div>
        </>
      ) : null}
    </details>
  );
}

// The bits of one wake-up: each stretch of one level as a bar as wide as it
// really was, the target in the tooltip.
function BitBar({ wake }: { wake: KklWakeRow }) {
  const total = wake.phases.reduce((sum, p) => sum + p.measuredMs, 0) || 1;
  return (
    <div className="kft-bits ltr" aria-hidden>
      {wake.phases.map((p, i) => {
        const off = Math.abs(p.measuredMs - p.targetMs);
        const tone = off > 20 ? "bad" : off > 10 ? "warn" : "good";
        return (
          <span
            key={i}
            className={`kft-bit ${p.one ? "one" : "zero"} b-${tone}`}
            style={{ flexGrow: p.measuredMs / total }}
            title={`${p.one ? "1" : "0"} x ${p.bits}: ${p.measuredMs} / ${p.targetMs} ms`}
          >
            {p.measuredMs}
          </span>
        );
      })}
    </div>
  );
}

function TimingCard({ wakes }: { wakes: KklWakeRow[] }) {
  if (wakes.length === 0) return null;
  return (
    <Card title={tr("Zeiten der Weckadresse")} icon="timer">
      <p className="subtle wrap">
        {tr(
          "5 Baud heißt 200 ms je Bit. Jeder Balken ist eine Strecke gleichen Pegels, so breit, wie sie auf diesem Rechner wirklich war. Mehr als 10 ms daneben ist knapp, mehr als 20 ms erkennt das Steuergerät meist nicht mehr.",
        )}
      </p>
      <div className="table-wrap">
        <table className="table kft-timing">
          <thead>
            <tr>
              <th>{tr("Steuergerät")}</th>
              <th>{tr("Bits")}</th>
              <th className="num">{tr("Abweichung")}</th>
              <th className="num">{tr("Break an / aus")}</th>
              <th className="num">{tr("Sync nach")}</th>
              <th className="num">{tr("Antwort nach")}</th>
              <th>{tr("Ergebnis")}</th>
            </tr>
          </thead>
          <tbody>
            {wakes.map((w, i) => (
              <tr key={i}>
                <td>
                  {w.unit}
                  {w.attempt > 1 ? <span className="subtle"> #{w.attempt}</span> : null}
                </td>
                <td className="kft-bits-cell">
                  <BitBar wake={w} />
                </td>
                <td className="num">{w.worstErrorMs} ms</td>
                <td className="num">
                  {w.onCallMs} / {w.offCallMs} ms
                </td>
                <td className="num">{w.firstByteAfterMs !== null ? `${w.firstByteAfterMs} ms` : "-"}</td>
                <td className="num">{w.replyAfterMs !== null ? `${w.replyAfterMs} ms` : "-"}</td>
                <td>{w.headline}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  );
}

export default function KklFirstTestPage() {
  const [options, setOptions] = useState<Option<string>[]>([]);
  const [port, setPort] = useState(A2_SIM);
  const [variant, setVariant] = useState<Variant>("a2-14");
  const [checks, setChecks] = useState({ ignition: false, plugged: false, direct: false, alone: false });
  const [running, setRunning] = useState<"cable" | "car" | null>(null);
  const [steps, setSteps] = useState<KklTestStep[]>([]);
  const [report, setReport] = useState<KklTestReport | null>(null);
  const [includeVin, setIncludeVin] = useState(false);
  const [saved, setSaved] = useState<KklSavedReport | null>(null);
  const resultRef = useRef<HTMLDivElement>(null);

  // The serial ports with their chip, then the simulated A2. A device the
  // dev page asks for joins the list.
  const refresh = useCallback(async (keep?: string) => {
    let list: Option<string>[] = [];
    try {
      const ports = await call("list_serial_ports");
      list = ports.map((p) => ({
        value: p.device,
        label: [p.device, p.description, p.chipGuess].filter(Boolean).join("  |  "),
      }));
    } catch {
      // No ports is the same as no answer.
    }
    list.push({ value: A2_SIM, label: tr("Demo: Audi A2 1.4 hinter einem KKL-Kabel (simuliert)") });
    if (keep && !list.some((o) => o.value === keep)) list.push({ value: keep, label: keep });
    setOptions(list);
    setPort(keep !== undefined && list.some((o) => o.value === keep) ? keep : list[0]!.value);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const start = (kind: "cable" | "car", device: string) => {
    setSteps([]);
    setReport(null);
    setSaved(null);
    setRunning(kind);
    void call(kind === "cable" ? "kkl_cable_check" : "kkl_first_test", { device });
  };

  // --page kkl-firsttest[:fault] and kkl-cabletest[:fault]: the simulated A2,
  // run right away. Pro mode for this run only, without saving settings.
  useAppEvent<string>("dev:page", (name) => {
    const [page, fault] = name.split(":");
    if (page !== "kkl-firsttest" && page !== "kkl-cabletest") return;
    settingsStore.set({ modeOverride: true });
    navigate("first-test");
    const cable = page === "kkl-cabletest";
    // The cable test is meant for a cable on USB only.
    const device = fault ? `${A2_SIM}:${fault}` : cable ? `${A2_SIM}:no-power` : A2_SIM;
    if (!cable) setChecks({ ignition: true, plugged: true, direct: true, alone: true });
    void refresh(device).then(() => start(cable ? "cable" : "car", device));
  });

  useBackendEvent("kkl_test_step", ({ step }) => {
    setSteps((list) => {
      const at = list.findIndex((s) => s.id === step.id);
      if (at < 0) return [...list, step];
      const next = [...list];
      next[at] = step;
      return next;
    });
  });

  useBackendEvent("kkl_test_finished", ({ report: done }) => {
    setRunning(null);
    setSteps(done.steps);
    setReport(done);
    // The verdict and the timing are what matters now.
    setTimeout(() => resultRef.current?.scrollIntoView({ behavior: "smooth", block: "start" }), 50);
  });

  const allChecked = checks.ignition && checks.plugged && checks.direct && checks.alone;

  const context = (): string[] => {
    const form = VARIANT_FORM[variant];
    const lines = [`Car: ${form.car ? `${form.car} ${form.engine} (${form.year})` : "other car with K-line"}`];
    if (report?.kind === "car") {
      lines.push(
        `Checklist: ignition on and engine off ${checks.ignition ? "yes" : "no"}, cable in the socket ${checks.plugged ? "yes" : "no"}, ` +
          `directly on the computer ${checks.direct ? "yes" : "no"}, no other diagnostic program ${checks.alone ? "yes" : "no"}`,
      );
    }
    return lines;
  };

  const save = async () => {
    if (!report) return;
    try {
      const where = await call("kkl_test_save_report", { report, context: context(), includeVin });
      setSaved(where);
      toast(tr("Bericht gespeichert: %1", where.text), "success", 6000);
    } catch (e) {
      toast(String(e), "error", 6000);
    }
  };

  const openIssue = () => {
    if (!report) return;
    void call("open_url", { url: issueUrl(report, variant, includeVin, saved?.text ?? "") });
  };

  const cable = report?.cable;
  const variants: Option<Variant>[] = [
    { value: "a2-14", label: tr("Audi A2 1.4 Benziner (etwa 2001)") },
    { value: "a2-16fsi", label: tr("Audi A2 1.6 FSI (etwa 2002)") },
    { value: "other", label: tr("anderes Auto mit K-Line") },
  ];

  return (
    <Page className="kft-page">
      <PageHeader
        eyebrow={tr("WERKSTATT")}
        title={tr("Erster Test am Auto")}
        subtitle={tr(
          "Prüft ein KKL-Kabel Schritt für Schritt an einem VW, Audi, Seat oder Skoda mit K-Line, zugeschnitten auf den Audi A2. Es wird nur gelesen: nichts gelöscht, kein Stellgliedtest, keine Grundeinstellung. Der Port wird erst beim Start geöffnet.",
        )}
      />

      <div className="ws-bar">
        <PortChoice caption={tr("Anschluss:")} options={options} value={port} onChange={setPort} disabled={running !== null} />
        <Button icon="refresh" onClick={() => void refresh(port)} disabled={running !== null}>
          {tr("Suchen")}
        </Button>
      </div>

      <div className="kft-grid">
        <Card title={tr("1. Kabel ohne Auto prüfen")} icon="cable">
          <p className="subtle wrap">
            {tr(
              "Das Kabel nur am USB anstecken, nicht am Auto. Caruspex liest den Chip und beim FTDI den Latenz-Timer, schaltet die Break-Leitung wie bei einer Weckadresse und misst die Zeiten. Ohne Auto kommt kein Echo zurück, das ist richtig so.",
            )}
          </p>
          <Button icon="play" onClick={() => start("cable", port)} disabled={running !== null || !port}>
            {tr("Kabel prüfen")}
          </Button>
        </Card>

        <Card title={tr("2. Am Auto")} icon="car">
          <label className="ws-field">
            <span className="ws-caption">{tr("Auto:")}</span>
            <Select className="grow" value={variant} options={variants} onChange={setVariant} disabled={running !== null} />
          </label>
          <div className="kft-checks">
            <Check checked={checks.ignition} onChange={(v) => setChecks((c) => ({ ...c, ignition: v }))}>
              {tr("Zündung an, Motor aus")}
            </Check>
            <Check checked={checks.plugged} onChange={(v) => setChecks((c) => ({ ...c, plugged: v }))}>
              {tr("Das Kabel steckt fest in der OBD-Buchse")}
            </Check>
            <Check checked={checks.direct} onChange={(v) => setChecks((c) => ({ ...c, direct: v }))}>
              {tr("Das Kabel hängt direkt am Rechner, nicht an einem USB-Hub")}
            </Check>
            <Check checked={checks.alone} onChange={(v) => setChecks((c) => ({ ...c, alone: v }))}>
              {tr("Kein anderes Diagnoseprogramm ist offen")}
            </Check>
          </div>
          <div className="ws-bar">
            <Button
              variant="primary"
              icon="play"
              onClick={() => start("car", port)}
              disabled={running !== null || !port || !allChecked}
              title={allChecked ? undefined : tr("Erst alle Punkte abhaken.")}
            >
              {tr("Ersten Test starten")}
            </Button>
            {running ? (
              <Button icon="stop" onClick={() => void call("cancel_scan")}>
                {tr("Abbrechen")}
              </Button>
            ) : null}
          </div>
          <p className="subtle wrap">
            {tr(
              "Ablauf: Echo, Motor (01) wecken, Kennung, Fehlerspeicher, Messwertblöcke 001 bis 003, dann Kombiinstrument 17, ABS 03, Airbag 15, Komfort 46 und Zentralelektrik 09. Jede Weckadresse dauert gut zwei Sekunden, der ganze Test etwa eine Minute.",
            )}
          </p>
        </Card>
      </div>

      {cable && (cable.latencyAdvice || cable.chipAdvice) ? (
        <Note tone={cable.latencyMs !== null && cable.latencyMs > 1 ? "var(--warn)" : undefined} icon="usb" title={`${cable.description} (${cable.familyName})`}>
          {[cable.chipAdvice, cable.latencyMs !== null ? tr("Latenz-Timer: %1 ms", cable.latencyMs) : "", cable.latencyAdvice]
            .filter(Boolean)
            .join("\n")}
        </Note>
      ) : null}

      {steps.length > 0 ? (
        <Card title={running === "cable" || report?.kind === "cable" ? tr("Kabeltest") : tr("Erster Test")} icon="list">
          <div className="kft-steps">
            {steps.map((s) => (
              <StepRow key={s.id} step={s} />
            ))}
          </div>
        </Card>
      ) : null}

      <div ref={resultRef} />
      {report ? (
        <Note
          tone={report.verdict.ok ? "var(--good)" : "var(--bad)"}
          icon={report.verdict.ok ? "check-circle" : "alert"}
          title={report.cancelled ? `${tr("Abgebrochen.")} ${report.verdict.headline}` : report.verdict.headline}
        >
          {[report.verdict.advice, ...report.verdict.warnings].filter(Boolean).join("\n\n")}
        </Note>
      ) : null}

      {report ? <TimingCard wakes={report.wakes} /> : null}

      {report ? (
        <Card title={tr("Bericht")} icon="file">
          <p className="subtle wrap">
            {tr(
              "Der Bericht kommt als Text und als JSON in den Einstellungsordner von Caruspex. Schick ihn uns über das Formular auf GitHub, das hilft am meisten. Ohne Haken sind FIN und Wegfahrsperren-Nummer gekürzt.",
            )}
          </p>
          <Check checked={includeVin} onChange={setIncludeVin}>
            {tr("FIN und Wegfahrsperren-Nummer vollständig mitschicken")}
          </Check>
          <div className="ws-bar">
            <Button variant="primary" icon="download" onClick={() => void save()}>
              {tr("Bericht speichern")}
            </Button>
            {saved ? (
              <Button icon="file" onClick={() => void call("kkl_test_reveal", { path: saved.text })}>
                {tr("Im Ordner zeigen")}
              </Button>
            ) : null}
            <Button icon="help" onClick={openIssue}>
              {tr("Testbericht auf GitHub öffnen")}
            </Button>
          </div>
          {saved ? <div className="subtle wrap ltr kft-path">{saved.text}</div> : null}
        </Card>
      ) : null}
    </Page>
  );
}
