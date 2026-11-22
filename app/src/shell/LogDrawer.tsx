// The log panel under the pages (MainWindow::buildLogDrawer): every line the
// backend reports, time stamped, with a button to record the session into a
// file for playing it back later.
import { useEffect, useRef, useState } from "react";
import { call, type SharedRecording } from "../api";
import { Icon } from "../components/Icon";
import { toast } from "../components/Toast";
import { Button } from "../components/ui";
import { t } from "../i18n";
import { useStore } from "../lib/store";
import { clearLog, logStore, session, useSession } from "../state/session";
import { ShareDialog, prepareShare } from "./ShareRecording";

const MW = "caruspex::MainWindow";

function stamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}_${p(d.getHours())}-${p(d.getMinutes())}`;
}

type LogState = ReturnType<typeof logStore.get>;

// What the panel shows: one block per batch of lines. With the raw log on a
// live session adds lines every 150 ms; setting the whole text again joined
// up to 5000 lines and laid all of them out each time (about 190 ms of
// layout a second at 1700 lines). A new batch is appended and laid out on its
// own, and lines that fall off the top shorten the first blocks.
interface LogView {
  total: number;
  clears: number;
  count: number;
  blocks: { el: HTMLElement; lines: string[] }[];
}

function appendBlock(pre: HTMLElement, view: LogView, lines: string[]): void {
  if (lines.length === 0) return;
  const el = document.createElement("span");
  el.className = "log-block";
  el.textContent = lines.join("\n");
  pre.appendChild(el);
  view.blocks.push({ el, lines });
  view.count += lines.length;
}

function syncLog(pre: HTMLElement, view: LogView, state: LogState): void {
  const added = state.total - view.total;
  if (state.clears !== view.clears || added < 0 || added > state.lines.length) {
    pre.replaceChildren();
    view.blocks = [];
    view.count = 0;
    appendBlock(pre, view, state.lines);
  } else if (added > 0) {
    appendBlock(pre, view, state.lines.slice(state.lines.length - added));
  }
  let extra = view.count - state.lines.length;
  while (extra > 0) {
    const first = view.blocks[0]!;
    if (first.lines.length <= extra) {
      first.el.remove();
      view.blocks.shift();
      view.count -= first.lines.length;
      extra -= first.lines.length;
    } else {
      first.lines = first.lines.slice(extra);
      first.el.textContent = first.lines.join("\n");
      view.count -= extra;
      extra = 0;
    }
  }
  view.total = state.total;
  view.clears = state.clears;
}

export function LogDrawer({ open, onClose }: { open: boolean; onClose: () => void }) {
  // A closed drawer keeps the text it showed and catches up when it opens,
  // so new lines cost nothing while it is shut.
  const shown = useRef(logStore.get());
  const log = useStore(logStore, (s) => (open ? s : shown.current));
  shown.current = log;
  const view = useRef<LogView>({ total: 0, clears: 0, count: 0, blocks: [] });
  const recording = useSession((s) => s.recording);
  const box = useRef<HTMLPreElement>(null);
  const [share, setShare] = useState<{ path: string; shared: SharedRecording; version: string } | null>(null);

  const openShare = async () => {
    const prepared = await prepareShare();
    if (!prepared) return;
    const version = await call("app_info")
      .then((info) => info.version)
      .catch(() => "");
    setShare({ ...prepared, version });
  };

  // Follows the end like QPlainTextEdit, unless the user scrolled up.
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const atEnd = el.scrollHeight - el.scrollTop - el.clientHeight < 60;
    syncLog(el, view.current, log);
    if (atEnd) el.scrollTop = el.scrollHeight;
  }, [log, open]);

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
          <Button
            variant="ghost"
            small
            icon="file"
            disabled={recording}
            title={t(
              "caruspex::Share",
              "Eine Aufnahme ohne die letzten Stellen der VIN und ohne Anschlussnamen zum Anhängen an ein GitHub-Issue packen",
            )}
            onClick={() => void openShare()}
          >
            {t("caruspex::Share", "Mitschnitt teilen")}
          </Button>
          <Button variant="ghost" small onClick={clearLog}>
            {t(MW, "Leeren")}
          </Button>
          <Button variant="ghost" small icon="chevron-down" title={t("caruspex::TitleBar", "Protokoll ein- oder ausblenden")} onClick={onClose} />
        </div>
        {/* Filled by syncLog, not by React. */}
        <pre ref={box} className="log-text mono selectable" />
      </div>
      {share ? <ShareDialog path={share.path} shared={share.shared} version={share.version} onClose={() => setShare(null)} /> : null}
    </section>
  );
}
