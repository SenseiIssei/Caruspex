// StatTile and FindingRow of Widgets.cpp.
import type { CSSProperties, ReactNode } from "react";
import { Icon, type IconName } from "./Icon";
import { Dot } from "./ui";

// Icon, caption, a big value with unit and a coloured state dot.
export function StatTile({
  icon,
  caption,
  value,
  unit,
  state,
  note,
  onClick,
}: {
  icon: IconName;
  caption: string;
  value?: string;
  unit?: string;
  // CSS colour of the dot, empty for none.
  state?: string;
  note?: ReactNode;
  onClick?: () => void;
}) {
  return (
    <div
      className={`stat-tile card flat${onClick ? " clickable" : ""}`}
      onClick={onClick}
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
      onKeyDown={
        onClick
          ? (e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onClick();
              }
            }
          : undefined
      }
    >
      <div className="stat-tile-top">
        <Icon name={icon} size={16} className="muted" />
        <span className="stat-tile-caption">{caption}</span>
        {state ? <Dot tone={state} glow /> : null}
      </div>
      <div className="stat-tile-value">
        <span className="stat-tile-number num">{value ?? "--"}</span>
        {unit ? <span className="stat-tile-unit">{unit}</span> : null}
      </div>
      {note ? <div className="hint">{note}</div> : null}
    </div>
  );
}

// One line in a "what was found" list: tinted icon tile, title, explanation,
// something small on the right.
export function FindingRow({
  icon,
  tint,
  title,
  text,
  meta,
  onClick,
}: {
  icon: IconName;
  // CSS colour, usually var(--bad) and the like.
  tint: string;
  title: ReactNode;
  text?: ReactNode;
  meta?: ReactNode;
  onClick?: () => void;
}) {
  return (
    <div
      className={`finding-row${onClick ? " clickable" : ""}`}
      onClick={onClick}
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
      onKeyDown={onClick ? (e) => (e.key === "Enter" || e.key === " ") && onClick() : undefined}
      style={{ "--tint": tint } as CSSProperties}
    >
      <div className="finding-icon">
        <Icon name={icon} size={18} />
      </div>
      <div className="finding-texts">
        <div className="finding-title">{title}</div>
        {text ? <div className="finding-text">{text}</div> : null}
      </div>
      {meta ? <div className="finding-meta">{meta}</div> : null}
    </div>
  );
}
