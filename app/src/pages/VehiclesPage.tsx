// Every car Caruspex knows, from 1980 to today: search by name, check a VIN,
// and see for each generation which socket it has, where it sits and what
// works with it (VehiclesPage.cpp). Besides the search the catalog can be
// browsed brand by brand and model by model.
import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";
import { call } from "../api";
import type { VehicleDiagnosisInfo, VehicleGenerationInfo, VinInfo, CatalogMatch } from "../api";
import { Icon } from "../components/Icon";
import { FindingRow } from "../components/Tiles";
import { Button, Card, NumberInput, Page, PageHeader, Pill, Select, TextInput, cx } from "../components/ui";
import { VehicleDiagram } from "../components/VehicleDiagram";
import { t, translator } from "../i18n";
import { useAppEvent } from "../lib/hooks";
import { emitApp } from "../lib/store";
import { bodyStyleFor, identifyGeneration } from "../lib/vehicleIdentity";
import { navigate } from "../shell/nav";
import { useSession } from "../state/session";
import "./VehiclesPage.css";

const tr = translator("caruspex::VehiclesPage");

// ObdSupport: 0 None, 1 Manufacturer, 2 Kkl, 3 Partial, 4 Full.
function supportTone(support: number): string {
  switch (support) {
    case 4: return "var(--good)";
    case 3: return "var(--warn)";
    case 2: return "var(--accent)";
    case 1: return "var(--warn)";
    case 0: return "var(--bad)";
  }
  return "var(--text-muted)";
}

// The well-known models shown before anything is typed. The newest
// generation of each popular brand would be arbitrary.
const STARTERS = ["Golf", "A4", "3er", "C-Klasse", "Corsa", "Polo", "Octavia", "Model 3"];

interface VinResult {
  info: VinInfo;
  match: CatalogMatch | null;
}

interface Detail {
  generation: VehicleGenerationInfo;
  year: number;
  title: string;
  subtitle: string;
  diagnosis: VehicleDiagnosisInfo;
}

function FactRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="vehicles-fact">
      <span className="hint">{label}</span>
      <span className={cx("selectable", mono && "mono strong ltr")}>{value}</span>
    </div>
  );
}

export default function VehiclesPage() {
  const [size, setSize] = useState(0);
  const [brandList, setBrandList] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<VehicleGenerationInfo[]>([]);
  const [countText, setCountText] = useState("");
  const [current, setCurrent] = useState<VehicleGenerationInfo | null>(null);
  const [year, setYear] = useState(0);
  const [detail, setDetail] = useState<Detail | null>(null);
  const [vinText, setVinText] = useState("");
  const [vinResult, setVinResult] = useState<VinResult | null>(null);
  // Browsing: a brand and one of its models; empty = the search decides.
  const [brand, setBrand] = useState("");
  const [modelList, setModelList] = useState<string[]>([]);
  const [model, setModel] = useState("");
  const listRef = useRef<HTMLDivElement>(null);
  // Answers can come back out of order while typing; only the last counts.
  const searchTurn = useRef(0);
  const detailTurn = useRef(0);

  const sessionVin = useSession((s) => s.vin);

  useEffect(() => {
    void call("catalog_size").then(setSize).catch(() => setSize(0));
    void call("catalog_brands").then(setBrandList).catch(() => setBrandList([]));
  }, []);

  // The picture, the name and what diagnosis is possible in that year.
  const showDetail = useCallback(async (g: VehicleGenerationInfo, y: number) => {
    const turn = ++detailTurn.current;
    try {
      const [identity, diagnosis] = await Promise.all([
        identifyGeneration(g, y),
        call("catalog_diagnosis", { generationId: g.id, year: y }),
      ]);
      if (turn !== detailTurn.current) return;
      setDetail({ generation: g, year: y, title: identity.title, subtitle: identity.subtitle, diagnosis });
    } catch {
      // The lookup failed; keep what is shown.
    }
  }, []);

  const currentRef = useRef<VehicleGenerationInfo | null>(null);
  const yearRef = useRef(0);
  const selectGeneration = useCallback(
    (g: VehicleGenerationInfo, wantedYear?: number) => {
      const changed = currentRef.current?.id !== g.id;
      currentRef.current = g;
      setCurrent(g);
      let y = wantedYear ?? 0;
      if (!y) {
        // A newly picked generation starts at its last year; the year stays
        // when the same one is shown again.
        y = changed ? (g.yearTo > 0 ? g.yearTo : new Date().getFullYear()) : yearRef.current;
      }
      yearRef.current = y;
      setYear(y);
      void showDetail(g, y);
    },
    [showDetail],
  );

  // Fills the list and selects its first entry, like setCurrentRow(0).
  const showList = useCallback(
    (list: VehicleGenerationInfo[], count: string) => {
      setResults(list);
      setCountText(count);
      if (list.length > 0) selectGeneration(list[0]!);
    },
    [selectGeneration],
  );

  const runSearch = useCallback(
    async (text: string) => {
      const turn = ++searchTurn.current;
      const q = text.trim();
      let hits: VehicleGenerationInfo[] = [];
      try {
        if (!q) {
          const parts = await Promise.all(STARTERS.map((s) => call("catalog_search", { query: s, limit: 3 })));
          hits = parts.flat();
        } else {
          hits = await call("catalog_search", { query: q, limit: 80 });
        }
      } catch {
        hits = [];
      }
      if (turn !== searchTurn.current) return;
      showList(hits, q ? tr.n("%n Treffer", hits.length) : tr("Beliebte Modelle, oben suchen für alle"));
    },
    [showList],
  );

  useEffect(() => {
    void runSearch("");
  }, [runSearch]);

  const lastYear = current ? (current.yearTo > 0 ? current.yearTo : new Date().getFullYear()) : new Date().getFullYear() + 1;
  const firstYear = current ? current.yearFrom : 1980;

  // Typing a year passes through values outside the range ("2", "20", ...);
  // only a year the generation was built in is looked up, the field clamps
  // the rest when it loses focus.
  const changeYear = (value: number) => {
    if (!current) return;
    const y = Math.round(value);
    yearRef.current = y;
    setYear(y);
    if (y >= firstYear && y <= lastYear) void showDetail(current, y);
  };

  const onSearchChange = (text: string) => {
    setQuery(text);
    setBrand("");
    setModel("");
    setModelList([]);
    void runSearch(text);
  };

  // --- Browsing by brand and model -----------------------------------------------------

  const browseBrand = async (name: string) => {
    setBrand(name);
    setModel("");
    setModelList([]);
    if (!name) {
      void runSearch(query);
      return;
    }
    setQuery("");
    const turn = ++searchTurn.current;
    try {
      const list = await call("catalog_models", { brand: name });
      if (turn !== searchTurn.current) return;
      setModelList(list);
      const all = (await Promise.all(list.map((m) => call("catalog_generations", { brand: name, model: m })))).flat();
      if (turn !== searchTurn.current) return;
      showList(all, tr.n("%n Treffer", all.length));
    } catch {
      showList([], tr.n("%n Treffer", 0));
    }
  };

  const browseModel = async (name: string) => {
    setModel(name);
    if (!name) {
      void browseBrand(brand);
      return;
    }
    const turn = ++searchTurn.current;
    try {
      const list = await call("catalog_generations", { brand, model: name });
      if (turn !== searchTurn.current) return;
      showList(list, tr.n("%n Treffer", list.length));
    } catch {
      showList([], tr.n("%n Treffer", 0));
    }
  };

  // --- VIN ----------------------------------------------------------------------------------------

  const checkVin = useCallback(
    async (raw: string) => {
      const text = raw.trim().toUpperCase();
      if (!text) {
        setVinResult(null);
        return;
      }
      try {
        const info = await call("decode_vin", { vin: text });
        if (!info.valid) {
          setVinResult({ info, match: null });
          return;
        }
        const match = await call("match_vin", { vin: text });
        setVinResult({ info, match });
        if (match.generation) {
          // Show the match and its alternatives, without running a text search.
          searchTurn.current++;
          setQuery(match.generation.model);
          setBrand("");
          setModel("");
          setModelList([]);
          const list = [match.generation, ...match.alternatives.filter((g) => g.id !== match.generation!.id)];
          setResults(list);
          setCountText(tr("Treffer zur Fahrgestellnummer"));
          selectGeneration(match.generation, match.year > 0 ? clampYear(match.generation, match.year) : undefined);
        }
      } catch {
        setVinResult(null);
      }
    },
    [selectGeneration],
  );

  const showVin = useCallback(
    (vin: string) => {
      setVinText(vin);
      void checkVin(vin);
    },
    [checkVin],
  );

  // The VIN of the connected car, as soon as it is known.
  useEffect(() => {
    if (sessionVin) showVin(sessionVin);
  }, [sessionVin, showVin]);

  useAppEvent<string>("dev:page", (name) => {
    if (!name.startsWith("vehicles:")) return;
    navigate("vehicles");
    showVin(name.slice(9));
  });

  // --- List keyboard ----------------------------------------------------------------------------

  const onListKey = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const i = results.findIndex((g) => g.id === current?.id);
    const next = results[Math.max(0, Math.min(results.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)))];
    if (next) selectGeneration(next);
  };

  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(".vehicles-item.on");
    el?.scrollIntoView({ block: "nearest" });
  }, [current]);

  const info = vinResult?.info;
  const match = vinResult?.match;
  const years = info?.modelYears.map(String) ?? [];
  const support = detail?.diagnosis.support ?? 0;
  const tone = supportTone(support);

  return (
    <Page>
      <PageHeader
        eyebrow={tr("Wissen")}
        title={tr("Fahrzeuge")}
        subtitle={tr(
          "Über %1 Modellgenerationen von %2 Marken, von 1980 bis heute. Zu jeder steht, welchen Diagnosestecker sie hat, wo er sitzt und was Caruspex damit kann.",
          size,
          brandList.length,
        )}
      />

      <Card className="vehicles-vin">
        <h2 className="section-title">{tr("Fahrgestellnummer prüfen")}</h2>
        <p className="subtle vehicles-lead">
          {tr("Die 17 Zeichen stehen im Fahrzeugschein unter E, an der Windschutzscheibe unten links und im Motorraum.")}
        </p>
        <form
          className="vehicles-vin-row"
          onSubmit={(e) => {
            e.preventDefault();
            void checkVin(vinText);
          }}
        >
          <TextInput
            className="vehicles-vin-input mono ltr"
            value={vinText}
            onChange={setVinText}
            placeholder="WAUZZZ8Z12A123456"
            maxLength={24}
            spellCheck={false}
            autoComplete="off"
          />
          <Button type="submit" variant="primary" icon="search">
            {tr("Prüfen")}
          </Button>
        </form>

        {info ? (
          <div className="vehicles-vin-result fade-in" key={info.vin}>
            {info.problems.map((p) => (
              <FindingRow key={p} icon="alert" tint="var(--warn)" title={p} />
            ))}
            {info.valid ? (
              <>
                <div className="vehicles-facts">
                  <FactRow label={tr("Hersteller")} value={info.manufacturer || tr("unbekannt")} />
                  <FactRow label={tr("Herkunft")} value={`${info.country}, ${info.region}`} />
                  <FactRow
                    label={tr("Modelljahr")}
                    value={
                      years.length === 0
                        ? tr("steht nicht in der Nummer")
                        : info.modelYearCertain
                          ? years[0]!
                          : tr("%1 (nicht sicher)", years.join(" / "))
                    }
                  />
                  {info.modelCode ? <FactRow label={tr("Modellcode")} value={info.modelCode} mono /> : null}
                  <FactRow
                    label={tr("Prüfziffer")}
                    value={
                      !info.checkDigitApplies
                        ? tr("wird hier nicht verwendet")
                        : info.checkDigitOk
                          ? tr("stimmt")
                          : tr("stimmt nicht, erwartet wäre %1", info.expectedCheckDigit)
                    }
                  />
                  <FactRow label={tr("Seriennummer")} value={info.serial} mono />
                </div>
                {info.notes.map((n) => (
                  <div key={n} className="hint">
                    {n}
                  </div>
                ))}
                {match?.generation ? (
                  <FindingRow
                    icon="car"
                    tint="var(--accent)"
                    title={tr("Passt zu: %1", match.generation.displayName)}
                    text={tr("Sicherheit %1 von 100. %2", match.confidence, match.reason)}
                  />
                ) : null}
              </>
            ) : null}
          </div>
        ) : null}
      </Card>

      <div className="vehicles-columns">
        <Card className="vehicles-list-card">
          <div className="vehicles-search">
            <Icon name="search" size={16} className="faint" />
            <input
              className="input"
              type="search"
              value={query}
              onChange={(e) => onSearchChange(e.target.value)}
              placeholder={tr("Golf 4, A2, E46, W203, Octavia 2008, Model 3 ...")}
              spellCheck={false}
            />
          </div>
          <div className="vehicles-browse">
            <Select
              value={brand}
              options={[{ value: "", label: t("caruspex::LexiconPage", "Alle Marken") }, ...brandList.map((b) => ({ value: b, label: b }))]}
              onChange={(v) => void browseBrand(v)}
            />
            {brand ? (
              <Select
                value={model}
                options={[{ value: "", label: tr("Alle Modelle") }, ...modelList.map((m) => ({ value: m, label: m }))]}
                onChange={(v) => void browseModel(v)}
              />
            ) : null}
          </div>
          <div className="hint">{countText}</div>
          <div className="vehicles-list" ref={listRef} tabIndex={0} role="listbox" onKeyDown={onListKey}>
            {results.map((g) => (
              <button
                key={g.id}
                type="button"
                role="option"
                aria-selected={g.id === current?.id}
                className={cx("vehicles-item", g.id === current?.id && "on")}
                onClick={() => selectGeneration(g)}
              >
                <span className="vehicles-item-name">{g.displayName}</span>
                <span className="vehicles-item-years faint">{g.yearsText}</span>
              </button>
            ))}
          </div>
        </Card>

        {detail ? (
          <Card className="vehicles-detail fade-in">
            <VehicleDiagram className="vehicles-picture" body={bodyStyleFor(detail.generation.body)} />
            <h2 className="page-title selectable">{detail.title}</h2>
            <p className="subtle vehicles-sub">{detail.subtitle}</p>
            <div className="row wrap-row">
              <span className="hint">{tr("Baujahr")}</span>
              <NumberInput className="vehicles-year" value={year} min={firstYear} max={lastYear} onChange={changeYear} />
              <Pill tone={tone}>{detail.diagnosis.supportName}</Pill>
            </div>
            <p className="vehicles-diagnosis">{detail.diagnosis.diagnosisText}</p>
            <p className="subtle vehicles-line">{tr("Stecker: %1", detail.diagnosis.connectorName)}</p>
            <p className="subtle vehicles-line">{tr("Wo er sitzt: %1", detail.diagnosis.locationText)}</p>
            {detail.generation.note ? <p className="hint vehicles-line">{detail.generation.note}</p> : null}
            <div>
              <Button
                variant="primary"
                icon="plug"
                disabled={support === 0}
                onClick={() => {
                  emitApp("connect:generation", { generation: detail.generation, year: detail.year });
                  navigate("connect");
                }}
              >
                {tr("Mit diesem Auto verbinden")}
              </Button>
            </div>
          </Card>
        ) : null}
      </div>
    </Page>
  );
}

function clampYear(g: VehicleGenerationInfo, year: number): number {
  const last = g.yearTo > 0 ? g.yearTo : new Date().getFullYear();
  return Math.min(last, Math.max(g.yearFrom, year));
}
