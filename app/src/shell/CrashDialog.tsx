// Crash reports, only on request (crash.rs): the report lies on this
// computer already, without vehicle data. This dialog shows it and offers a
// prefilled GitHub issue; the user reads and changes it there before
// anything is posted. Nothing is sent by itself.
import { call, type CrashReport } from "../api";
import { Modal } from "../components/Modal";
import { Button } from "../components/ui";
import { translator } from "../i18n";
import { ISSUE_PAGE } from "./ShareRecording";

const tr = translator("caruspex::Crash");

// GitHub takes about 8000 characters in the address of a new issue.
const MAX_REPORT = 5000;

// In English for the maintainers; the user reads and changes it on GitHub.
export function crashIssueUrl(report: CrashReport): string {
  const firstLine = report.text.split("\n").find((l) => l.startsWith("Message: ")) ?? "";
  const title = `Crash: ${firstLine.replace("Message: ", "").slice(0, 80) || "panic"}`;
  const text = report.text.length > MAX_REPORT ? `${report.text.slice(0, MAX_REPORT)}\n...` : report.text;
  const body = ["What I was doing:", "", "", "Report (without vehicle data):", "", "```", text, "```"].join("\n");
  return `${ISSUE_PAGE}?title=${encodeURIComponent(title)}&body=${encodeURIComponent(body)}`;
}

export function CrashDialog({ report, live, onClose }: { report: CrashReport; live: boolean; onClose: () => void }) {
  const done = () => {
    void call("crash_report_done", { path: report.path }).catch(() => undefined);
    onClose();
  };
  const openIssue = () => {
    void call("open_url", { url: crashIssueUrl(report) });
    done();
  };
  return (
    <Modal
      icon="alert"
      title={live ? tr("Ein Teil von Caruspex ist abgestürzt") : tr("Caruspex wurde beim letzten Mal unerwartet beendet")}
      onClose={onClose}
      wide
      footer={
        <>
          <Button variant="ghost" onClick={done}>
            {tr("Nicht melden")}
          </Button>
          <Button variant="primary" icon="chevron-right" onClick={openIssue}>
            {tr("Auf GitHub melden")}
          </Button>
        </>
      }
    >
      <p className="muted">
        {tr(
          "Ein Bericht liegt auf diesem Rechner. Er enthält Version, System und die Stelle im Programm, aber keine Fahrzeugdaten: VIN, Werte und Bytes der Verbindung sind herausgenommen. Verschickt wird nichts; mit dem Knopf öffnet sich ein vorausgefülltes Issue, das du vor dem Absenden lesen und ändern kannst.",
        )}
      </p>
      {live ? <p>{tr("Die Verbindung zum Auto ist ab jetzt gestört. Bitte Caruspex neu starten.")}</p> : null}
      <p className="muted small ltr">{report.path}</p>
      <pre className="share-sample mono selectable ltr">{report.text}</pre>
    </Modal>
  );
}
