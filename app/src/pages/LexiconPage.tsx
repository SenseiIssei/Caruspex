// Offline reference for trouble codes, works without a car (LexiconPage.cpp):
// search by code or keyword, filter by brand family. A click on a row shows
// what the fault page shows for a code: advice, where to look, causes,
// checks and the rough cost.
import { useEffect, useMemo, useState } from "react";
import { call } from "../api";
import type { BrandFamily, DtcInfo, DtcLexiconEntry, ZoneInfo } from "../api";
import { CarZone } from "../api";
import { Icon } from "../components/Icon";
import { Button, Card, Page, PageHeader, Pill, Select, cx } from "../components/ui";
import { VehicleDiagram } from "../components/VehicleDiagram";
import { t, translator } from "../i18n";
import { BodyStyles } from "../lib/vehicleIdentity";
import "./LexiconPage.css";

const tr = translator("caruspex::LexiconPage");
const FP = "caruspex::FaultsPage";

// Filling thousands of rows at once would stall the window; the first few
// thousand are plenty for a search result.
const MAX_ROWS = 3000;
const ALL = "*";

// FaultsPage::severityColour.
export function severityTone(severity: number): string {
  switch (severity) {
    case 0: return "var(--info)";
    case 1: return "var(--good)";
    case 2: return "var(--warn)";
    case 3: return "var(--bad)";
    case 4: return "color-mix(in srgb, var(--bad) 78%, var(--text))";
  }
  return "var(--text-muted)";
}

type Entry = DtcLexiconEntry & { systemName: string; severityLabel: string };

export default function LexiconPage() {
  const [entries, setEntries] = useState<Entry[]>([]);
  const [families, setFamilies] = useState<BrandFamily[]>([]);
  const [query, setQuery] = useState("");
  const [family, setFamily] = useState<string>(ALL);
  const [selected, setSelected] = useState<Entry | null>(null);
  const [details, setDetails] = useState<{ info: DtcInfo; zone: ZoneInfo } | null>(null);

  // Built once in the language that is active at startup, like the Qt page.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [list, fams] = await Promise.all([call("dtc_lexicon"), call("brand_families")]);
        if (cancelled) return;
        setEntries(list.map((e) => ({ ...e, systemName: e.systemName ?? "", severityLabel: e.severityLabel ?? "" })));
        setFamilies(fams);
      } catch {
        if (!cancelled) setEntries([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const hits = useMemo(() => {
    const q = query.trim().toLocaleLowerCase();
    return entries.filter(
      (e) =>
        (family === ALL || e.family === family) &&
        (!q || e.code.toLowerCase().includes(q) || e.text.toLocaleLowerCase().includes(q) || e.systemName.toLocaleLowerCase().includes(q)),
    );
  }, [entries, query, family]);
  const shown = Math.min(hits.length, MAX_ROWS);

  // The generic codes under their label, then every family that has codes.
  const familyOptions = useMemo(() => {
    const generic = families.find((f) => f.key === "");
    return [
      { value: ALL, label: tr("Alle Marken") },
      ...(generic ? [{ value: "", label: generic.label }] : []),
      ...families.filter((f) => f.key !== "" && f.count > 0).map((f) => ({ value: f.key, label: f.label })),
    ];
  }, [families]);

  useEffect(() => {
    if (!selected) {
      setDetails(null);
      return;
    }
    let cancelled = false;
    void Promise.all([call("lookup_dtc", { code: selected.code, brand: "" }), call("locate_dtc", { code: selected.code })])
      .then(([info, zone]) => {
        if (!cancelled) setDetails({ info, zone });
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [selected]);

  const info = details?.info;
  const zone = details?.zone;
  const tone = selected ? severityTone(selected.severity) : "var(--accent)";
  // A brand code means what its own table says; the generic lookup only
  // knows the group it falls into.
  const ownText = selected && (selected.family !== "" || !info?.known);

  return (
    <Page className="lexicon-page">
      <PageHeader
        eyebrow={tr("Wissen")}
        title={tr("Fehlercode-Lexikon")}
        subtitle={tr(
          "Nachschlagewerk für unterwegs, funktioniert auch ohne Auto. Suche nach Code oder Stichwort, zum Beispiel Drosselklappe oder Lambdasonde. Herstellercodes bedeuten je Marke etwas anderes, deshalb lässt sich nach Marke filtern.",
        )}
      />

      <div className="lexicon-bar">
        <div className="lexicon-search">
          <Icon name="search" size={16} className="faint" />
          <input
            className="input"
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={tr("P0301, Lambdasonde, Zylinder 3 ...")}
            spellCheck={false}
          />
        </div>
        <Select className="lexicon-family" value={family} options={familyOptions} onChange={setFamily} />
        <span className="subtle lexicon-count num">
          {hits.length > shown ? tr("%1 Treffer, die ersten %2 gezeigt", hits.length, shown) : tr("%1 von %2 Codes", hits.length, entries.length)}
        </span>
      </div>

      <div className={cx("lexicon-columns", selected && "with-detail")}>
        <div className="table-wrap lexicon-table">
          <table className="table">
            <thead>
              <tr>
                <th>{tr("Code")}</th>
                <th>{tr("Bedeutung")}</th>
                <th>{tr("Bereich")}</th>
                <th>{tr("Dringlichkeit")}</th>
                <th>{tr("Gilt für")}</th>
              </tr>
            </thead>
            <tbody>
              {hits.slice(0, shown).map((e) => (
                <tr
                  key={`${e.family}|${e.code}`}
                  className={cx(selected?.code === e.code && selected.family === e.family && "selected")}
                  onClick={() => setSelected(e)}
                >
                  <td className="mono strong ltr" style={{ color: severityTone(e.severity) }}>
                    {e.code}
                  </td>
                  <td className="lexicon-text">{e.text}</td>
                  <td className="muted lexicon-nowrap">{e.systemName}</td>
                  <td className="lexicon-nowrap" style={{ color: severityTone(e.severity) }}>
                    {e.severityLabel}
                  </td>
                  <td className="faint lexicon-nowrap">{e.familyLabel}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {selected ? (
          <Card className="lexicon-detail fade-in" key={`${selected.family}|${selected.code}`}>
            <div className="row">
              <span className="lexicon-code mono ltr" style={{ color: tone }}>
                {selected.code}
              </span>
              <Pill tone={tone}>{selected.severityLabel}</Pill>
              <span className="spacer" />
              <Button variant="ghost" small icon="x" onClick={() => setSelected(null)} />
            </div>
            <h2 className="section-title selectable">{ownText ? selected.text : (info?.text ?? selected.text)}</h2>
            <div className="subtle">
              {[selected.systemName, selected.familyLabel].filter(Boolean).join("  ·  ")}
            </div>
            {info && !ownText ? (
              <div className="lexicon-advice" style={{ ["--tone" as string]: tone }}>
                <div className="strong">{t(FP, "Darf ich weiterfahren?")}</div>
                <div>{info.advice}</div>
              </div>
            ) : null}
            {zone && zone.zone !== CarZone.Unknown ? (
              <>
                <VehicleDiagram className="lexicon-diagram" body={BodyStyles.Hatchback} anatomy focus={zone.zone} affected={[zone.zone]} />
                <div className="subtle">{t(FP, "Wo nachsehen: %1. %2", zone.name, zone.hint)}</div>
              </>
            ) : null}
            {info && info.causes.length > 0 && !ownText ? (
              <div className="col lexicon-block">
                <div className="strong">{t(FP, "Häufige Ursachen")}</div>
                {info.causes.map((c, i) => (
                  <div key={c}>{`${i + 1}.  ${c}`}</div>
                ))}
              </div>
            ) : null}
            {info && info.checks.length > 0 && !ownText ? (
              <div className="col lexicon-block">
                <div className="strong">{t(FP, "Selbst nachsehen")}</div>
                {info.checks.map((c) => (
                  <div key={c} className="row top">
                    <Icon name="check" size={14} style={{ color: "var(--accent)", marginTop: 3, flex: "none" }} />
                    <span>{c}</span>
                  </div>
                ))}
              </div>
            ) : null}
            {info && info.costClass !== 0 && !ownText ? (
              <div className="hint">{t(FP, "Kosten in der Werkstatt: %1. %2", info.costLabel, info.costHint)}</div>
            ) : null}
          </Card>
        ) : null}
      </div>
    </Page>
  );
}
