// Custom title bar for the frameless window (TitleBar.cpp): logo, the
// connection capsule in the middle, quick toggles and the window buttons.
// The empty parts are a drag region; double-click maximises.
import { useEffect, useState, type CSSProperties } from "react";
import { insideTauri, type UpdateInfo } from "../api";
import { Icon } from "../components/Icon";
import { Spinner } from "../components/ui";
import { t } from "../i18n";
import { useSession } from "../state/session";
import { navigate } from "./nav";

const TB = "caruspex::TitleBar";

async function currentWindow() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

// The window buttons only act inside the app window; in a browser preview
// they are part of the picture.
function windowAction(action: "minimize" | "toggleMaximize" | "close"): void {
  if (!insideTauri) return;
  void currentWindow().then((w) => w[action]());
}

export function TitleBar({
  version,
  update,
  onOpenUpdate,
  logOpen,
  onToggleLog,
  onToggleSidebar,
}: {
  version: string;
  update: UpdateInfo | null;
  onOpenUpdate: () => void;
  logOpen: boolean;
  onToggleLog: () => void;
  onToggleSidebar: () => void;
}) {
  const link = useSession((s) => s.link);
  const linkText = useSession((s) => s.linkText);
  const [maximised, setMaximised] = useState(false);

  useEffect(() => {
    if (!insideTauri) return;
    let stop: (() => void) | undefined;
    void currentWindow().then(async (w) => {
      setMaximised(await w.isMaximized());
      stop = await w.onResized(async () => setMaximised(await w.isMaximized()));
    });
    return () => stop?.();
  }, []);

  useEffect(() => {
    document.documentElement.classList.toggle("maximised", maximised);
  }, [maximised]);

  const tone =
    link === "online" ? "var(--good)" : link === "connecting" ? "var(--warn)" : link === "error" ? "var(--bad)" : "var(--text-faint)";
  const text = linkText || t(TB, "Nicht verbunden");

  return (
    <header className="titlebar" data-tauri-drag-region>
      <button
        type="button"
        className="tool-button"
        title={t(TB, "Seitenleiste ein- oder ausklappen")}
        aria-label={t(TB, "Seitenleiste ein- oder ausklappen")}
        onClick={onToggleSidebar}
      >
        <Icon name="sidebar" size={18} />
      </button>
      <div className="logo-mark" data-tauri-drag-region>
        <Icon name="car" size={18} stroke={2} />
      </div>
      <span className="brand" data-tauri-drag-region>
        Caruspex
      </span>
      <span className="version" data-tauri-drag-region>
        v{version}
      </span>
      <div className="titlebar-centre" data-tauri-drag-region>
        <button
          className={`capsule capsule-${link}`}
          style={{ "--tone": tone } as CSSProperties}
          title={t(TB, "Verbindung öffnen")}
          onClick={() => navigate("connect")}
        >
          {link === "connecting" ? (
            <Spinner size={14} />
          ) : (
            <Icon name={link === "online" ? "plug" : "unplug"} size={14} className="capsule-icon" />
          )}
          <span className="capsule-text ltr-safe">{text}</span>
        </button>
      </div>
      {update ? (
        <button
          className="btn primary update-button"
          title={t(TB, "Zeigt, was neu ist, und aktualisiert Caruspex")}
          onClick={onOpenUpdate}
        >
          {t(TB, "Version %1 ist da", update.version)}
        </button>
      ) : null}
      <button
        className={`tool-button${logOpen ? " on" : ""}`}
        title={t(TB, "Protokoll ein- oder ausblenden")}
        aria-label={t(TB, "Protokoll ein- oder ausblenden")}
        aria-pressed={logOpen}
        onClick={onToggleLog}
      >
        <Icon name="terminal" size={18} />
      </button>
      <div className="window-buttons">
        <button
          type="button"
          className="window-button"
          title={t(TB, "Minimieren")}
          aria-label={t(TB, "Minimieren")}
          onClick={() => windowAction("minimize")}
        >
          <Icon name="minimise" size={16} />
        </button>
        <button
          type="button"
          className="window-button"
          title={maximised ? t(TB, "Wiederherstellen") : t(TB, "Maximieren")}
          aria-label={maximised ? t(TB, "Wiederherstellen") : t(TB, "Maximieren")}
          onClick={() => windowAction("toggleMaximize")}
        >
          <Icon name={maximised ? "restore" : "maximise"} size={16} />
        </button>
        <button
          type="button"
          className="window-button close"
          title={t(TB, "Schließen")}
          aria-label={t(TB, "Schließen")}
          onClick={() => windowAction("close")}
        >
          <Icon name="x" size={16} />
        </button>
      </div>
    </header>
  );
}
