// The sidebar of MainWindow: entries with icon, text and an optional badge,
// grouped into sections, the garage, adapter and settings below a divider.
// Collapses to icons only.
import { useEffect, useRef, type CSSProperties, type KeyboardEvent } from "react";
import { Icon } from "../components/Icon";
import { t } from "../i18n";
import { ACCESS } from "../lib/access";
import { useProMode, useSession } from "../state/session";
import { navigate, useCurrentPage } from "./nav";
import { PAGES, type PageEntry } from "./pages";

function Badge({ text, tone }: { text: string; tone: string }) {
  return (
    <span className="nav-badge" style={{ "--tone": tone } as CSSProperties}>
      {text}
    </span>
  );
}

function NavEntry({ entry, active, compact, badge }: { entry: PageEntry; active: boolean; compact: boolean; badge?: { text: string; tone: string } }) {
  const label = entry.label();
  return (
    <button
      type="button"
      className={`nav-button${active ? " active" : ""}`}
      onClick={() => navigate(entry.id)}
      title={compact ? label : undefined}
      aria-label={compact ? label : undefined}
      aria-current={active ? "page" : undefined}
    >
      <span className="nav-indicator" />
      <Icon name={entry.icon} size={18} className="nav-icon" />
      <span className="nav-text">{label}</span>
      {badge ? compact ? <span className="nav-dot" style={{ "--tone": badge.tone } as CSSProperties} /> : <Badge {...badge} /> : null}
    </button>
  );
}

export function Sidebar({ compact }: { compact: boolean }) {
  const page = useCurrentPage();
  const faults = useSession((s) => (s.connected ? s.faults.length : 0));
  const due = useSession((s) => s.servicesDue);

  const badgeFor = (entry: PageEntry) => {
    if (entry.id === "faults" && faults > 0) return { text: String(faults), tone: "var(--bad)" };
    if (entry.id === "garage" && due > 0) return { text: String(due), tone: "var(--warn)" };
    return undefined;
  };

  // Keeps the active entry in view, e.g. after a jump to a page further down.
  const list = useRef<HTMLDivElement>(null);
  useEffect(() => {
    list.current?.querySelector(".nav-button.active")?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [page]);

  // Beginners see the everyday pages only. A section heading goes above the
  // first entry of its section that is shown, so hiding the first one does
  // not take the heading with it.
  const pro = useProMode();
  const shown = (p: PageEntry) => pro || !p.pro;
  const top: { entry: PageEntry; heading?: () => string }[] = [];
  let section: (() => string) | undefined;
  let printed: (() => string) | undefined;
  for (const entry of PAGES.filter((p) => !p.bottom)) {
    if (entry.section) section = entry.section;
    if (!shown(entry)) continue;
    top.push({ entry, heading: section !== printed ? section : undefined });
    printed = section;
  }
  const bottom = PAGES.filter((p) => p.bottom && shown(p));

  // Arrow keys walk the entries, Home and End jump to the first and last,
  // like a menu; Tab still leaves the sidebar for the page.
  const onKey = (e: KeyboardEvent<HTMLElement>) => {
    const keys = ["ArrowDown", "ArrowUp", "Home", "End"];
    if (!keys.includes(e.key)) return;
    const buttons = Array.from(e.currentTarget.querySelectorAll<HTMLButtonElement>(".nav-button"));
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (at < 0 || buttons.length === 0) return;
    e.preventDefault();
    const next =
      e.key === "Home" ? 0 : e.key === "End" ? buttons.length - 1 : (at + (e.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
    buttons[next]!.focus();
  };

  return (
    <nav className={`sidebar${compact ? " compact" : ""}`} aria-label={t(ACCESS, "Seiten")} onKeyDown={onKey}>
      <div className="sidebar-scroll" ref={list}>
        {top.map(({ entry, heading }) => (
          <div key={entry.id} className="nav-group">
            {heading ? <div className="nav-section">{compact ? "" : heading()}</div> : null}
            <NavEntry entry={entry} active={page === entry.id} compact={compact} badge={badgeFor(entry)} />
          </div>
        ))}
      </div>
      <div className="sidebar-divider" />
      <div className="sidebar-bottom">
        {bottom.map((entry) => (
          <NavEntry key={entry.id} entry={entry} active={page === entry.id} compact={compact} badge={badgeFor(entry)} />
        ))}
      </div>
    </nav>
  );
}
