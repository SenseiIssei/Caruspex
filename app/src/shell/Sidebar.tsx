// The sidebar of MainWindow: entries with icon, text and an optional badge,
// grouped into sections, the garage, adapter and settings below a divider.
// Collapses to icons only.
import { useEffect, useRef, type CSSProperties } from "react";
import { Icon } from "../components/Icon";
import { useSession } from "../state/session";
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

  const top = PAGES.filter((p) => !p.bottom);
  const bottom = PAGES.filter((p) => p.bottom);

  return (
    <nav className={`sidebar${compact ? " compact" : ""}`}>
      <div className="sidebar-scroll" ref={list}>
        {top.map((entry) => (
          <div key={entry.id} className="nav-group">
            {entry.section ? <div className="nav-section">{compact ? "" : entry.section()}</div> : null}
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
