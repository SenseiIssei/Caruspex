// Small pieces of the connection page: the permission note, a numbered
// step, the checklist of the connection assistant, the mode chips and the
// refresh button.
import type { CSSProperties } from "react";
import type { ConnectHelp } from "../../api";
import { Icon, type IconName } from "../../components/Icon";
import { Button, Chip } from "../../components/ui";
import { translator } from "../../i18n";
import type { Mode } from "./shared";

// Texts of the Android app.
const ta = translator("caruspex::Android");
// Texts that are new in the connection assistant.
const tc = translator("caruspex::ConnectHelp");

// Android 12 and newer: before the system asks for "nearby devices", the
// page says why, and where to allow it later when it was refused. Up to
// Android 11 the same for the location, which only the Bluetooth LE search
// needs there.
export function BluetoothPermission({ refused, location, onAllow }: { refused: boolean; location?: boolean; onAllow: () => void }) {
  return (
    <div className="connect-permission">
      <div className="connect-permission-head">
        <Icon name="bluetooth" size={18} />
        <span>{location ? ta("Standort für Bluetooth LE") : ta("Zugriff auf Geräte in der Nähe")}</span>
      </div>
      <div className="subtle">
        {location
          ? ta(
              "Bis Android 11 findet die Suche Adapter mit Bluetooth LE nur, wenn Caruspex den Standort " +
                "verwenden darf. Caruspex liest ihn nicht und schickt nichts weg. Gekoppelte klassische " +
                "Adapter gehen auch ohne.",
            )
          : ta(
              "Damit Caruspex mit dem Bluetooth-Adapter sprechen kann, fragt Android einmal nach dem " +
                "Zugriff auf Geräte in der Nähe. Caruspex verbindet sich damit nur mit dem Adapter, es " +
                "liest keinen Standort und schickt nichts weg.",
            )}
      </div>
      {refused ? (
        <div className="hint">
          {ta("Der Zugriff ist abgelehnt. Erlauben lässt er sich in den Android-Einstellungen unter Apps, Caruspex, Berechtigungen.")}
        </div>
      ) : null}
      <Button variant="primary" icon="shield" onClick={onAllow}>
        {ta("Zugriff erlauben")}
      </Button>
    </div>
  );
}

// One numbered step of the "how to" card, with the gradient circle.
export function Step({ n, icon, title, text }: { n: number; icon: IconName; title: string; text: string }) {
  return (
    <div className="connect-step">
      <div className="connect-step-number">{n}</div>
      <div className="connect-step-texts">
        <div className="connect-step-head">
          <Icon name={icon} size={16} className="muted" />
          <span>{title}</span>
        </div>
        <div className="subtle">{text}</div>
      </div>
    </div>
  );
}

// The connection assistant: what the attempt found out, then the points to
// check, the likely reason first.
const CHECK_LOOK: Record<string, { icon: IconName; tone: string }> = {
  problem: { icon: "x-circle", tone: "var(--bad)" },
  check: { icon: "info", tone: "var(--warn)" },
  fine: { icon: "check-circle", tone: "var(--good)" },
};

export function ConnectChecklist({ help }: { help: ConnectHelp }) {
  return (
    <div className="connect-help fade-in">
      {help.found.length > 0 ? (
        <div className="connect-help-found">
          <div className="connect-help-head">{tc("Was Caruspex herausgefunden hat")}</div>
          <ul>
            {help.found.map((line) => (
              <li key={line} className="selectable">
                {line}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      <div className="connect-help-head">{tc("Das kannst du prüfen")}</div>
      <div className="connect-help-list">
        {help.items.map((item) => {
          const look = CHECK_LOOK[item.state] ?? CHECK_LOOK.check!;
          return (
            <div key={item.id} className={`connect-check ${item.state}`} style={{ "--tone": look.tone } as CSSProperties}>
              <Icon name={look.icon} size={18} className="connect-check-icon" />
              <div className="connect-check-texts">
                <div className="connect-check-title">{item.title}</div>
                {item.text ? <div className="subtle">{item.text}</div> : null}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function ModeChip({ mode, current, icon, label, onPick }: { mode: Mode; current: Mode; icon: IconName; label: string; onPick: (m: Mode) => void }) {
  return (
    <Chip on={mode === current} onClick={() => onPick(mode)}>
      <Icon name={icon} size={16} />
      {label}
    </Chip>
  );
}

// The square refresh button next to a list.
export function RefreshButton({ title, onClick, disabled }: { title?: string; onClick: () => void; disabled?: boolean }) {
  return <Button icon="refresh" className="connect-refresh" title={title} aria-label={title} onClick={onClick} disabled={disabled} />;
}
