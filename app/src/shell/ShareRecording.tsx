// "Mitschnitt teilen": a recording packed for a GitHub issue, so control unit
// lists of more cars come into the project, proven on a real car. The
// backend hides the last digits of the VIN, the port or address and the
// time of day (caruspex_record::share); this dialog shows what stays before
// anything leaves the computer. Saving the file and opening the issue page
// are two buttons; nothing is sent by itself.
import { call, type SharedRecording } from "../api";
import { Modal } from "../components/Modal";
import { toast } from "../components/Toast";
import { Button } from "../components/ui";
import { translator } from "../i18n";
import { session } from "../state/session";

const tr = translator("caruspex::Share");

export const ISSUE_PAGE = "https://github.com/SenseiIssei/Caruspex/issues/new";

// Picks the recording (the last one of this run, else asks) and prepares it.
export async function prepareShare(): Promise<{ path: string; shared: SharedRecording } | null> {
  let path = session.get().lastRecording;
  if (!path) {
    path =
      (await call("pick_open_path", {
        title: tr("Aufnahme zum Teilen wählen"),
        filters: [{ name: tr("Caruspex-Aufnahme (*.jsonl)"), extensions: ["jsonl"] }],
      })) ?? "";
  }
  if (!path) return null;
  try {
    const shared = await call("share_recording", { path, vin: session.get().vin });
    return shared ? { path, shared } : null;
  } catch (e) {
    toast(String(e), "error", 6000);
    return null;
  }
}

function fileName(path: string): string {
  const base = path.split(/[\\/]/).pop() ?? "caruspex.jsonl";
  return base.replace(/\.jsonl$/i, "") + "-shared.jsonl";
}

// The text of the new issue, in English for the maintainers; the user reads
// and changes it on GitHub before anything is posted, and attaches the file
// there.
export function issueUrl(shared: SharedRecording, carTitle: string, file: string, version: string): string {
  const p = shared.preview;
  const title = `Recording: ${carTitle || p.vins[0] || "unknown car"}`;
  const lines = [
    `A recording from Caruspex ${version} for the control unit lists.`,
    "",
    `- Car: ${carTitle || "?"}`,
    `- VIN: ${p.vins.join(", ") || "?"}`,
    `- Adapter: ${p.adapter || "?"}`,
    `- Protocol: ${p.protocol || "?"}`,
    `- ${p.exchanges} requests in ${p.durationSeconds.toFixed(1)} s`,
    "",
    `Please drag the file ${file} into this field so it is attached to the issue.`,
  ];
  return `${ISSUE_PAGE}?title=${encodeURIComponent(title)}&body=${encodeURIComponent(lines.join("\n"))}`;
}

export function ShareDialog({
  path,
  shared,
  version,
  onClose,
}: {
  path: string;
  shared: SharedRecording;
  version: string;
  onClose: () => void;
}) {
  const p = shared.preview;
  const name = fileName(path);
  const carTitle = session.get().identity?.title ?? "";

  const save = async () => {
    const where = await call("save_text_file", {
      title: tr("Mitschnitt zum Teilen speichern"),
      suggestedName: name,
      filters: [{ name: tr("Caruspex-Aufnahme (*.jsonl)"), extensions: ["jsonl"] }],
      content: shared.file,
    });
    if (where) toast(tr("Gespeichert: %1", where), "success");
  };

  const openIssue = () => void call("open_url", { url: issueUrl(shared, carTitle, name, version) });

  return (
    <Modal
      icon="file"
      title={tr("Mitschnitt teilen")}
      onClose={onClose}
      wide
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {tr("Schließen")}
          </Button>
          <Button icon="download" onClick={() => void save()}>
            {tr("Datei speichern")}
          </Button>
          <Button variant="primary" icon="chevron-right" onClick={openIssue}>
            {tr("GitHub-Issue öffnen")}
          </Button>
        </>
      }
    >
      <p className="muted">
        {tr(
          "So sieht die Datei aus, die du teilst. Sie bleibt auf deinem Rechner, bis du sie selbst speicherst und an ein GitHub-Issue hängst. Mit Mitschnitten echter Autos kommen deren Steuergeräte ins Projekt.",
        )}
      </p>
      <div className="share-columns">
        <section>
          <h3 className="section-title">{tr("Drin bleibt")}</h3>
          <ul className="share-list">
            <li>{tr("%1 Anfragen in %2 s", p.exchanges, p.durationSeconds.toFixed(1))}</li>
            <li>{tr("Adapter: %1", p.adapter || "?")}</li>
            <li>{tr("Protokoll: %1", p.protocol || "?")}</li>
            {p.created ? <li>{tr("Datum: %1", p.created)}</li> : null}
            {p.vins.map((v) => (
              <li key={v} className="ltr">
                VIN: {v}
              </li>
            ))}
            {p.requests.map((r) => (
              <li key={r.kind}>
                {r.kind}: {r.count}
              </li>
            ))}
          </ul>
        </section>
        <section>
          <h3 className="section-title">{tr("Entfernt")}</h3>
          <ul className="share-list">
            <li>
              {p.vinPlaces > 0
                ? tr("Die letzten %1 Stellen der VIN, an %2 Stellen im Mitschnitt", p.hiddenDigits, p.vinPlaces)
                : tr("Keine VIN im Mitschnitt gefunden")}
            </li>
            <li>{p.transportRemoved ? tr("Anschluss, Bluetooth- oder Netzwerkadresse") : tr("Kein Anschlussname enthalten")}</li>
            <li>{tr("Die Uhrzeit der Aufnahme")}</li>
            <li>{tr("Standort und Nutzerdaten: stehen in Aufnahmen nie drin")}</li>
          </ul>
        </section>
      </div>
      <h3 className="section-title">{tr("Anfang der Datei (%1 kB)", Math.max(1, Math.round(p.bytes / 1024)))}</h3>
      <pre className="share-sample mono selectable ltr">{p.sample.join("\n")}</pre>
    </Modal>
  );
}
