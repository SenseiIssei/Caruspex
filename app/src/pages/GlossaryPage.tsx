// The glossary under Wissen: every technical word Caruspex uses, in plain
// words. The words themselves carry a short hint wherever they appear
// (components/Term.tsx); a click there jumps here.
import { useEffect, useMemo, useRef, useState } from "react";
import { Icon } from "../components/Icon";
import { Card, EmptyState, Page, PageHeader, TextInput, cx } from "../components/ui";
import { language, locale, translator } from "../i18n";
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

  // Translated, lower-cased for the search and sorted once per language,
  // not on every render (the page renders each time it is shown).
  const lang = language();
  const all = useMemo(() => {
    const collator = new Intl.Collator(locale());
    return GLOSSARY.map((e) => {
      const entry = { id: e.id, term: e.term(), short: e.short(), long: e.long() };
      return { ...entry, haystack: `${entry.term} ${entry.short} ${entry.long}`.toLocaleLowerCase() };
    }).sort((a, b) => collator.compare(a.term, b.term));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lang]);
  const needle = query.trim().toLocaleLowerCase();
  const entries = needle ? all.filter((e) => e.haystack.includes(needle)) : all;

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
