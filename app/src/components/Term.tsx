// A technical word with its explanation at hand: hovering or focusing it
// shows one sentence from the glossary, a click (or Enter) opens the
// glossary page at that entry. The hint is drawn into the body, so cards
// that clip their content do not cut it off.
import { useId, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { t } from "../i18n";
import { glossaryEntry, type TermId } from "../lib/glossary";
import { emitApp } from "../lib/store";

// A line of words under a page header ("Begriffe: DTC, MIL, ..."), each
// with its hint, for pages whose own texts come from the car.
export function TermStrip({ ids }: { ids: readonly TermId[] }) {
  return (
    <div className="term-strip hint">
      <span>{t("caruspex::Usability", "Begriffe:")}</span>
      {ids.map((id, i) => (
        <span key={id}>
          <Term id={id} />
          {i < ids.length - 1 ? "," : ""}
        </span>
      ))}
    </div>
  );
}

interface Spot {
  x: number;
  y: number;
  above: boolean;
}

export function Term({ id, children }: { id: TermId; children?: ReactNode }) {
  const entry = glossaryEntry(id);
  const tipId = useId();
  const word = useRef<HTMLSpanElement>(null);
  const [spot, setSpot] = useState<Spot | null>(null);
  if (!entry) return <>{children}</>;

  const show = () => {
    const r = word.current?.getBoundingClientRect();
    if (!r) return;
    // Below the word, or above it near the bottom of the window.
    const above = r.bottom + 120 > window.innerHeight;
    const x = Math.min(Math.max(12, r.left), Math.max(12, window.innerWidth - 332));
    setSpot({ x, y: above ? r.top - 8 : r.bottom + 8, above });
  };
  const hide = () => setSpot(null);
  const open = () => {
    hide();
    emitApp("navigate", "glossary");
    emitApp("glossary:show", id);
  };

  return (
    <>
      <span
        ref={word}
        className="term"
        tabIndex={0}
        role="button"
        aria-describedby={tipId}
        onMouseEnter={show}
        onMouseLeave={hide}
        onFocus={show}
        onBlur={hide}
        onClick={open}
        onKeyDown={(e) => {
          if (e.key === "Escape") hide();
          else if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            open();
          }
        }}
      >
        {children ?? entry.term()}
      </span>
      {/* Always in the document, so screen readers find the description. */}
      {createPortal(
        <span
          id={tipId}
          role="tooltip"
          className={`term-tip${spot ? " open" : ""}${spot?.above ? " above" : ""}`}
          style={spot ? { left: spot.x, top: spot.y } : undefined}
        >
          <span className="term-tip-title">{entry.term()}</span>
          <span className="term-tip-text">{entry.short()}</span>
        </span>,
        document.body,
      )}
    </>
  );
}
