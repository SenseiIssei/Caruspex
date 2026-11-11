// The glossary under Wissen: every technical word Caruspex uses, in plain
// words. The words themselves carry a short hint wherever they appear
// (components/Term.tsx); a click there jumps here.
import { useEffect, useRef, useState } from "react";
import { Icon } from "../components/Icon";
import { Card, EmptyState, Page, PageHeader, TextInput, cx } from "../components/ui";
import { locale, translator } from "../i18n";
import { GLOSSARY, type TermId } from "../lib/glossary";
import { useAppEvent } from "../lib/hooks";
import "./GlossaryPage.css";

const tu = translator("caruspex::Usability");

export default function GlossaryPage() {
  const [query, setQuery] = useState("");
  const [marked, setMarked] = useState<TermId | null>(null);
  const list = useRef<HTMLDivElement>(null);

  // A click on a word elsewhere: show that entry.
  useAppEvent<TermId>("glossary:show", (id) => {
    setQuery("");
    setMarked(id);
  });

  useEffect(() => {
    if (!marked) return;
    const el = list.current?.querySelector<HTMLElement>(`[data-term="${marked}"]`);
    el?.scrollIntoView({ block: "center", behavior: "smooth" });
  }, [marked]);

  const needle = query.trim().toLocaleLowerCase();
  const entries = GLOSSARY.map((e) => ({ id: e.id, term: e.term(), short: e.short(), long: e.long() }))
    .filter((e) => !needle || `${e.term} ${e.short} ${e.long}`.toLocaleLowerCase().includes(needle))
    .sort((a, b) => a.term.localeCompare(b.term, locale()));

  return (
    <Page className="glossary-page">
      <PageHeader
        eyebrow={tu("WISSEN")}
        title={tu("Glossar")}
        subtitle={tu(
          "Die Fachwörter der Diagnose in einfachen Worten. Unterstrichene Wörter in Caruspex zeigen ihre Erklärung beim Darüberfahren, ein Klick führt hierher.",
        )}
      />
      <div className="glossary-search">
        <Icon name="search" size={16} className="muted" />
        <TextInput
          className="grow"
          value={query}
          onChange={setQuery}
          placeholder={tu("Begriff suchen ...")}
          aria-label={tu("Begriff suchen ...")}
        />
      </div>
      {entries.length === 0 ? (
        <EmptyState icon="book" title={tu("Kein Begriff gefunden")} />
      ) : (
        <div className="glossary-list" ref={list}>
          {entries.map((e) => (
            <Card key={e.id} className={cx("glossary-entry fade-in", marked === e.id && "marked")}>
              <div data-term={e.id}>
                <h2 className="section-title selectable">{e.term}</h2>
                <p className="glossary-short selectable">{e.short}</p>
                <p className="subtle glossary-long selectable">{e.long}</p>
              </div>
            </Card>
          ))}
        </div>
      )}
    </Page>
  );
}
