// The log panel under the pages (MainWindow::buildLogDrawer): every line the
// backend reports, time stamped, with a button to record the session into a
// file for playing it back later.
import { useEffect, useRef } from "react";
import { call } from "../api";
import { Icon } from "../components/Icon";
import { toast } from "../components/Toast";
import { Button } from "../components/ui";
import { t } from "../i18n";
import { useStore } from "../lib/store";
import { clearLog, logStore, session, useSession } from "../state/session";

const MW = "caruspex::MainWindow";

function stamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}_${p(d.getHours())}-${p(d.getMinutes())}`;
}

export function LogDrawer({ open, onClose }: { open: boolean; onClose: () => void }) {
  const lines = useStore(logStore, (s) => s.lines);
  const recording = useSession((s) => s.recording);
  const box = useRef<HTMLPreElement>(null);

  // Follows the end like QPlainTextEdit, unless the user scrolled up.
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    if (el.scrollHeight - el.scrollTop - el.clientHeight < 60) el.scrollTop = el.scrollHeight;
  }, [lines, open]);

  const toggleRecording = async () => {
    if (recording) {
      await call("stop_recording");
      return;
    }
    if (!session.get().connected) {
      toast(t(MW, "Erst verbinden, dann aufzeichnen."), "info");
      return;
    }
    const path = await call("pick_save_path", {
      title: t(MW, "Aufzeichnung speichern"),
      suggestedName: `caruspex-${stamp()}.jsonl`,
      filters: [{ name: t(MW, "Caruspex-Aufnahme (*.jsonl)"), extensions: ["jsonl"] }],
    });
    if (path) await call("start_recording", { path });
  };

  return (
    <section className={`log-drawer${open ? " open" : ""}`} aria-hidden={!open}>
      <div className="log-inner">
        <div className="log-head">
          <Icon name="terminal" size={16} className="muted" />
          <span className="strong">{t(MW, "Protokoll")}</span>
          <span className="spacer" />
          <Button
            variant="ghost"
            small
            icon={recording ? "stop" : "play"}
            className={recording ? "recording" : undefined}
            title={t(MW, "Die laufende Sitzung in eine Datei aufzeichnen, zum späteren Abspielen")}
            onClick={() => void toggleRecording()}
          >
            {recording ? t(MW, "Aufzeichnung beenden") : t(MW, "Aufzeichnen")}
          </Button>
          <Button variant="ghost" small onClick={clearLog}>
            {t(MW, "Leeren")}
          </Button>
          <Button variant="ghost" small icon="chevron-down" onClick={onClose} />
        </div>
        <pre ref={box} className="log-text mono selectable">
          {lines.join("\n")}
        </pre>
      </div>
    </section>
  );
}
