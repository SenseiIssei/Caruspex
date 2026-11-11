// The update dialog: what is new in the found version, then "Jetzt
// aktualisieren" downloads it, checks its signature and installs it
// (update.rs). The download page takes the place of the button where this
// copy cannot replace itself (a .deb install, the portable zip).
import { useEffect, useState, type ReactNode } from "react";
import { call, on, type UpdateInfo } from "../api";
import { Modal } from "../components/Modal";
import { Button, Note, Progress } from "../components/ui";
import { localeDate, localeNumber, translator } from "../i18n";
import { appendLog } from "../state/session";

const tr = translator("caruspex::UpdateDialog");

type Phase = "ask" | "loading" | "restarting" | "page" | "failed";

// The change list is Markdown of changelog/<version>.md: bold lines as
// headings, "- " as points, the rest as paragraphs.
function Notes({ text }: { text: string }) {
  const blocks: ReactNode[] = [];
  let points: string[] = [];
  const flush = () => {
    if (points.length > 0) {
      blocks.push(
        <ul key={blocks.length} className="update-points">
          {points.map((p, i) => (
            <li key={i}>{p}</li>
          ))}
        </ul>,
      );
      points = [];
    }
  };
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (line.startsWith("- ")) {
      points.push(line.slice(2).replace(/\*\*/g, ""));
      continue;
    }
    flush();
    if (!line) continue;
    const heading = /^\*\*(.+)\*\*$/.exec(line);
    blocks.push(
      heading ? (
        <div key={blocks.length} className="update-heading">
          {heading[1]}
        </div>
      ) : (
        <p key={blocks.length}>{line.replace(/\*\*/g, "")}</p>
      ),
    );
  }
  flush();
  return <div className="update-notes">{blocks}</div>;
}

function megabytes(bytes: number): string {
  return localeNumber(bytes / (1024 * 1024), 1);
}

export function UpdateDialog({ update, autoStart, onClose }: { update: UpdateInfo; autoStart: boolean; onClose: () => void }) {
  const [phase, setPhase] = useState<Phase>("ask");
  const [progress, setProgress] = useState({ received: 0, total: 0 });
  const [error, setError] = useState("");

  useEffect(() => on("update_progress", (p) => setProgress(p)), []);

  const install = async () => {
    setPhase("loading");
    setError("");
    setProgress({ received: 0, total: 0 });
    appendLog(tr("Lade Version %1 ...", update.version));
    try {
      const outcome = await call("install_update");
      setPhase(outcome === "quit" ? "restarting" : "page");
      appendLog(
        outcome === "quit"
          ? tr("Version %1 ist geprüft und wird installiert, Caruspex startet neu.", update.version)
          : tr("Die Downloadseite ist im Browser geöffnet."),
      );
    } catch (e) {
      const message = String(e);
      setError(message);
      setPhase("failed");
      appendLog(message);
    }
  };

  // "Updates automatisch installieren": no question, straight to the download.
  useEffect(() => {
    if (autoStart && update.canInstall) void install();
    // Only once, for the version found at start.
  }, []);

  const busy = phase === "loading" || phase === "restarting";
  const percent = progress.total > 0 ? (progress.received / progress.total) * 100 : undefined;

  const footer =
    phase === "restarting" || phase === "page" ? (
      <Button onClick={onClose}>{tr("Schließen")}</Button>
    ) : (
      <>
        <Button onClick={onClose} disabled={busy}>
          {tr("Später")}
        </Button>
        {update.canInstall ? (
          <Button variant="primary" icon="download" busy={busy} disabled={busy} onClick={() => void install()}>
            {phase === "failed" ? tr("Noch einmal versuchen") : tr("Jetzt aktualisieren")}
          </Button>
        ) : (
          <Button variant="primary" icon="download" onClick={() => void call("open_url", { url: update.url })}>
            {tr("Zur Downloadseite")}
          </Button>
        )}
      </>
    );

  return (
    <Modal small icon="sparkles" title={tr("Caruspex %1 ist da", update.version)} onClose={busy ? () => undefined : onClose} footer={footer}>
      <div className="update-dialog">
        {update.date ? <div className="hint">{tr("Veröffentlicht am %1", localeDate(update.date))}</div> : null}
        {update.notes ? <Notes text={update.notes} /> : null}
        {phase === "loading" ? (
          <div className="update-progress">
            <Progress value={percent} indeterminate={percent === undefined} />
            <div className="hint">
              {progress.total > 0
                ? tr("%1 von %2 MB geladen", megabytes(progress.received), megabytes(progress.total))
                : tr("%1 MB geladen", megabytes(progress.received))}
            </div>
          </div>
        ) : null}
        {phase === "restarting" ? (
          <Note tone="var(--good)" icon="check-circle">
            {tr("Die Signatur stimmt. Caruspex schließt sich jetzt, installiert die neue Version und startet neu.")}
          </Note>
        ) : null}
        {phase === "page" ? (
          <Note icon="info">
            {tr("Diese Installation lässt sich nicht selbst ersetzen. Die Downloadseite ist im Browser geöffnet.")}
          </Note>
        ) : null}
        {phase === "failed" ? (
          <Note tone="var(--bad)" icon="alert">
            {error}
          </Note>
        ) : null}
        {phase === "ask" || phase === "failed" ? (
          <div className="hint">
            {update.canInstall
              ? tr("Caruspex lädt das Paket von GitHub und installiert es nur, wenn seine Signatur stimmt.")
              : tr("Diese Installation wird über die Downloadseite aktualisiert.")}
          </div>
        ) : null}
      </div>
    </Modal>
  );
}
