// The garage (GaragePage.cpp): every car seen so far, each with its file:
// checks with their score trend, service plan, trips, fuel log and a note.
// The rules live in state/garage.ts (Garage.cpp); this page only shows them.
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { call } from "../api";
import type { GarageCar, GarageService } from "../api";
import { confirm, Modal } from "../components/Modal";
import { FindingRow, StatTile } from "../components/Tiles";
import { toast } from "../components/Toast";
import { Button, Card, Check, EmptyState, NumberInput, Page, PageHeader, Pill, Select, StatusLine, Tabs, TextInput, cx } from "../components/ui";
import type { IconName } from "../components/Icon";
import { VehicleDiagram } from "../components/VehicleDiagram";
import { localeDate, localeNumber, t, tn, translator } from "../i18n";
import { useAppEvent, useBackendEvent } from "../lib/hooks";
import { navigate, openGarageReport, openReport } from "../shell/nav";
import {
  fuelLogConsumption,
  importGarage,
  isoDate,
  latestOdometer,
  markServiceDone,
  odometerJumps,
  parseIso,
  removeCar,
  removeFuel,
  removeService,
  addFuel,
  serialiseGarage,
  serviceStatus,
  serviceTemplates,
  setNote,
  setService,
  useGarage,
} from "../state/garage";
import { useSession } from "../state/session";
import "./GaragePage.css";

const tr = translator("caruspex::GaragePage");
const Q = "QObject";
// QDialogButtonBox: the cancel text of the pages that have one.
const cancelText = () => t("caruspex::FaultsPage", "Abbrechen");

const shortDate = (iso: string) => {
  const d = parseIso(iso);
  return d ? localeDate(d, { dateStyle: "short" }) : "";
};
const shortDateTime = (iso: string) => {
  const d = parseIso(iso);
  return d ? localeDate(d, { dateStyle: "short", timeStyle: "short" }) : "";
};

// --- Car list ---------------------------------------------------------------------------------

function CarCard({ car, selected, current, onClick }: { car: GarageCar; selected: boolean; current: boolean; onClick: () => void }) {
  const last = car.checks[car.checks.length - 1];
  const faults = last ? last.codes.length : 0;
  const state = !last ? t(Q, "noch keine Prüfung") : faults === 0 ? t(Q, "zuletzt ohne Fehler") : tn(Q, "zuletzt %n Fehler", faults);
  const tone = !last ? "var(--text-faint)" : faults === 0 ? "var(--good)" : "var(--bad)";
  return (
    <div
      className={cx("garage-car", selected && "on")}
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && onClick()}
    >
      <VehicleDiagram className="garage-car-picture" body={car.bodyStyle} />
      <div className="garage-car-texts">
        <div className="garage-car-name">{car.name || t(Q, "Unbekanntes Fahrzeug")}</div>
        <div className="mono faint garage-car-vin ltr">{car.vin}</div>
        <StatusLine text={<span className="garage-car-state">{state}</span>} tone={tone} />
      </div>
      {current ? <Pill tone="var(--good)">{t(Q, "verbunden")}</Pill> : null}
    </div>
  );
}

// --- Score trend ------------------------------------------------------------------------------

// The points of every check over time, so a car getting worse shows at a
// glance.
function ScoreTrend({ car }: { car: GarageCar }) {
  const points = car.checks.filter((c) => c.score >= 0);
  if (points.length < 2) return null;
  const w = 600;
  const h = 70;
  const pad = 8;
  const x = (i: number) => pad + (i * (w - 2 * pad)) / (points.length - 1);
  const y = (score: number) => pad + ((100 - Math.max(0, Math.min(100, score))) * (h - 2 * pad)) / 100;
  const line = points.map((p, i) => `${i === 0 ? "M" : "L"}${x(i).toFixed(1)},${y(p.score).toFixed(1)}`).join(" ");
  const area = `${line} L${x(points.length - 1).toFixed(1)},${h - pad} L${x(0).toFixed(1)},${h - pad} Z`;
  const last = points[points.length - 1]!;
  const tone = (s: number) => (s >= 80 ? "var(--good)" : s >= 50 ? "var(--warn)" : "var(--bad)");
  return (
    <div className="garage-trend fade-in">
      <div className="row">
        <span className="hint grow">{tr("Punkte der Prüfungen")}</span>
        <span className="strong num" style={{ color: tone(last.score) }}>
          {tr("%1 Punkte", last.score)}
        </span>
      </div>
      <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" className="garage-trend-chart" aria-hidden>
        <defs>
          <linearGradient id="garage-trend-fill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="var(--accent)" stopOpacity="0.28" />
            <stop offset="1" stopColor="var(--accent)" stopOpacity="0" />
          </linearGradient>
        </defs>
        <line x1={pad} x2={w - pad} y1={y(50)} y2={y(50)} stroke="var(--track)" strokeDasharray="4 6" vectorEffect="non-scaling-stroke" />
        <path d={area} fill="url(#garage-trend-fill)" />
        <path d={line} fill="none" stroke="var(--accent)" strokeWidth={2} strokeLinejoin="round" vectorEffect="non-scaling-stroke" />
      </svg>
      <div className="garage-trend-dots">
        {points.map((p, i) => (
          <span
            key={`${p.when}-${i}`}
            className="garage-trend-dot"
            title={`${shortDateTime(p.when)}: ${tr("%1 Punkte", p.score)}`}
            style={{ insetInlineStart: `${(x(i) / w) * 100}%`, top: `${(y(p.score) / h) * 100}%`, background: tone(p.score) }}
          />
        ))}
      </div>
    </div>
  );
}

// --- Dialogs ----------------------------------------------------------------------------------

function DialogFooter({ onOk, onCancel, okDisabled }: { onOk: () => void; onCancel: () => void; okDisabled?: boolean }) {
  return (
    <>
      <Button onClick={onCancel}>{cancelText()}</Button>
      <Button variant="primary" onClick={onOk} disabled={okDisabled}>
        OK
      </Button>
    </>
  );
}

function FormRow({ label, children }: { label: ReactNode; children: ReactNode }) {
  return (
    <>
      <div className="form-label">{label}</div>
      <div className="garage-form-control">{children}</div>
    </>
  );
}

// "Wartung hinzufügen", also used to change an existing item.
function ServiceDialog({ vin, odometer, edit, onClose }: { vin: string; odometer: number; edit: GarageService | null; onClose: () => void }) {
  const templates = useMemo(() => serviceTemplates(), []);
  const startIndex = edit ? templates.findIndex((s) => s.title === edit.title) : 0;
  const [kind, setKind] = useState(startIndex >= 0 ? startIndex : templates.length);
  const custom = kind >= templates.length;
  const [title, setTitle] = useState(edit ? edit.title : (templates[0]?.title ?? ""));
  const [km, setKm] = useState(edit ? edit.intervalKm : (templates[0]?.intervalKm ?? 0));
  const [months, setMonths] = useState(edit ? edit.intervalMonths : (templates[0]?.intervalMonths ?? 0));
  const [known, setKnown] = useState(edit ? !!edit.lastDate || edit.lastKm >= 0 : false);
  const [lastDate, setLastDate] = useState(edit?.lastDate || isoDate());
  const [lastKm, setLastKm] = useState(edit && edit.lastKm >= 0 ? edit.lastKm : odometer > 0 ? Math.round(odometer) : 0);

  const pickKind = (index: number) => {
    setKind(index);
    const tpl = templates[index];
    setTitle(tpl ? tpl.title : "");
    if (tpl) {
      setKm(tpl.intervalKm);
      setMonths(tpl.intervalMonths);
    }
  };

  const accept = () => {
    if (!title.trim()) {
      onClose();
      return;
    }
    setService(vin, {
      id: edit?.id ?? "",
      title: title.trim(),
      intervalKm: Math.max(0, Math.round(km)),
      intervalMonths: Math.max(0, Math.round(months)),
      lastDate: known ? lastDate : "",
      lastKm: known ? (lastKm > 0 ? lastKm : -1) : -1,
      note: edit?.note ?? "",
    });
    onClose();
  };

  return (
    <Modal
      small
      icon="wrench"
      title={edit ? tr("Wartung bearbeiten") : tr("Wartung hinzufügen")}
      onClose={onClose}
      footer={<DialogFooter onOk={accept} onCancel={onClose} />}
    >
      <div className="form garage-form">
        <FormRow label={tr("Art")}>
          <Select
            value={kind}
            options={[...templates.map((s, i) => ({ value: i, label: s.title })), { value: templates.length, label: tr("Eigener Eintrag") }]}
            onChange={pickKind}
          />
        </FormRow>
        <FormRow label={tr("Bezeichnung")}>
          <TextInput value={title} onChange={setTitle} disabled={!custom} />
        </FormRow>
        <FormRow label={tr("Alle")}>
          <NumberInput value={km} min={0} max={1000000} step={1000} suffix={km === 0 ? tr("nicht nach km") : "km"} onChange={setKm} />
        </FormRow>
        <FormRow label={tr("oder alle")}>
          <NumberInput
            value={months}
            min={0}
            max={240}
            suffix={months === 0 ? tr("nicht nach Zeit") : tr(" Monate").trim()}
            onChange={setMonths}
          />
        </FormRow>
        <FormRow
          label={
            <Check checked={known} onChange={setKnown}>
              {tr("Zuletzt erledigt am")}
            </Check>
          }
        >
          <input className="input" type="date" value={lastDate} onChange={(e) => e.target.value && setLastDate(e.target.value)} />
        </FormRow>
        <FormRow label={tr("bei Kilometerstand")}>
          <NumberInput value={lastKm} min={0} max={2000000} suffix={lastKm === 0 ? tr("unbekannt") : "km"} onChange={setLastKm} />
        </FormRow>
      </div>
    </Modal>
  );
}

function DoneDialog({ vin, service, odometer, onClose }: { vin: string; service: GarageService; odometer: number; onClose: () => void }) {
  const [date, setDate] = useState(isoDate());
  const [km, setKm] = useState(odometer >= 0 ? Math.round(odometer) : 0);
  const accept = () => {
    markServiceDone(vin, service.id, date, km > 0 ? km : -1);
    onClose();
  };
  return (
    <Modal small icon="check" title={tr("%1 erledigt", service.title)} onClose={onClose} footer={<DialogFooter onOk={accept} onCancel={onClose} />}>
      <div className="form garage-form">
        <FormRow label={tr("Datum")}>
          <input className="input" type="date" value={date} onChange={(e) => e.target.value && setDate(e.target.value)} />
        </FormRow>
        <FormRow label={tr("Kilometerstand")}>
          <NumberInput value={km} min={0} max={2000000} suffix="km" onChange={setKm} />
        </FormRow>
      </div>
    </Modal>
  );
}

function FuelDialog({ vin, odometer, onClose }: { vin: string; odometer: number; onClose: () => void }) {
  const [date, setDate] = useState(isoDate());
  const [km, setKm] = useState(odometer > 0 ? Math.round(odometer) : 0);
  const [litres, setLitres] = useState(0);
  const [price, setPrice] = useState(1.799);
  const [full, setFull] = useState(true);
  const accept = () => {
    if (litres > 0) {
      addFuel(vin, { date, odometerKm: km > 0 ? km : -1, litres, pricePerLitre: price, full });
    }
    onClose();
  };
  return (
    <Modal small icon="fuel" title={tr("Tankfüllung eintragen")} onClose={onClose} footer={<DialogFooter onOk={accept} onCancel={onClose} />}>
      <div className="form garage-form">
        <FormRow label={tr("Datum")}>
          <input className="input" type="date" value={date} onChange={(e) => e.target.value && setDate(e.target.value)} />
        </FormRow>
        <FormRow label={tr("Kilometerstand")}>
          <NumberInput value={km} min={0} max={2000000} suffix="km" onChange={setKm} />
        </FormRow>
        <FormRow label={tr("Menge")}>
          <NumberInput value={litres} min={0} max={500} step={0.01} decimals={2} suffix="l" onChange={setLitres} />
        </FormRow>
        <FormRow label={tr("Preis")}>
          <NumberInput value={price} min={0} max={10} step={0.001} decimals={3} suffix="€/l" onChange={setPrice} />
        </FormRow>
        <div className="full">
          <Check checked={full} onChange={setFull}>
            {tr("voll getankt")}
          </Check>
        </div>
      </div>
    </Modal>
  );
}

// --- Tabs -----------------------------------------------------------------------------------------

function HistoryTab({ car }: { car: GarageCar }) {
  const rows: ReactNode[] = [];
  // A falling odometer between two checks is worth a word, especially when
  // buying a used car. It is a hint: a swapped instrument cluster or a
  // mistake in reading can explain it too.
  odometerJumps(car).forEach((j, i) =>
    rows.push(
      <FindingRow
        key={`jump-${i}`}
        icon="alert"
        tint="var(--bad)"
        title={tr("Kilometerstand ist gesunken")}
        text={tr(
          "Am %1 waren es %2 km, am %3 nur noch %4 km. Das kann ein getauschtes Kombiinstrument sein, aber auch ein zurückgedrehter Tacho. Beim Gebrauchtwagenkauf nachfragen.",
          shortDate(j.earlier),
          localeNumber(j.earlierKm, 0),
          shortDate(j.later),
          localeNumber(j.laterKm, 0),
        )}
      />,
    ),
  );
  if (car.checks.length === 0) {
    rows.push(
      <FindingRow
        key="none"
        icon="info"
        tint="var(--info)"
        title={tr("Noch keine Prüfung gespeichert")}
        text={tr("Auto prüfen auf der Startseite legt einen Eintrag an.")}
      />,
    );
  }
  for (let i = car.checks.length - 1; i >= 0; i--) {
    const c = car.checks[i]!;
    let title = shortDateTime(c.when);
    if (c.odometerKm >= 0) title += `  ·  ${localeNumber(c.odometerKm, 0)} km`;
    const parts = c.codes.map((code, k) => (k < c.texts.length ? `${code} ${c.texts[k]}` : code));
    let text = parts.length === 0 ? tr("Kein Fehler gespeichert.") : parts.join("\n");
    if (c.mil) text += `\n${tr("Motorkontrollleuchte an.")}`;
    if (c.openMonitors > 0) text += `\n${tr.n("%n Bereitschaftstest(s) offen.", c.openMonitors)}`;
    rows.push(
      <FindingRow
        key={`${c.when}-${i}`}
        icon={c.codes.length === 0 ? "check-circle" : "alert"}
        tint={c.codes.length === 0 ? "var(--good)" : "var(--bad)"}
        title={title}
        text={<span className="wrap">{text}</span>}
        meta={c.score >= 0 ? tr("%1 Punkte", c.score) : undefined}
      />,
    );
  }
  return (
    <div className="garage-tab">
      <ScoreTrend car={car} />
      <div className="garage-rows">{rows}</div>
    </div>
  );
}

function ServicesTab({ car, odometer }: { car: GarageCar; odometer: number }) {
  const [dialog, setDialog] = useState<{ kind: "add" } | { kind: "edit"; service: GarageService } | { kind: "done"; service: GarageService } | null>(null);
  const statuses = serviceStatus(car, odometer);
  return (
    <div className="garage-tab">
      <div className="garage-bar">
        <span className="subtle grow">{tr("Was wann fällig ist, nach Kilometer oder Zeit, je nachdem was zuerst kommt.")}</span>
        <Button icon="wrench" onClick={() => setDialog({ kind: "add" })}>
          {tr("Wartung hinzufügen")}
        </Button>
      </div>
      <div className="garage-rows">
        {statuses.map((st) => {
          const s = st.service;
          let tint = "var(--text-faint)";
          let icon: IconName = "clock";
          if (st.state === "due") {
            tint = "var(--bad)";
            icon = "alert";
          } else if (st.state === "soon") {
            tint = "var(--warn)";
          } else if (st.state === "ok") {
            tint = "var(--good)";
            icon = "check-circle";
          }
          const rule: string[] = [];
          if (s.intervalKm > 0) rule.push(tr("alle %1 km", localeNumber(s.intervalKm, 0)));
          if (s.intervalMonths > 0) rule.push(tr.n("alle %n Monat(e)", s.intervalMonths));
          let text = rule.join(tr(" oder "));
          if (s.lastDate || s.lastKm >= 0) {
            const last: string[] = [];
            if (s.lastDate) last.push(shortDate(s.lastDate));
            if (s.lastKm >= 0) last.push(`${localeNumber(s.lastKm, 0)} km`);
            text += `\n${tr("Zuletzt: %1", last.join(", "))}`;
          }
          return (
            <div key={s.id} className="garage-line fade-in">
              <div className="grow">
                <FindingRow
                  icon={icon}
                  tint={tint}
                  title={s.title}
                  text={<span className="wrap">{text.trim()}</span>}
                  meta={st.text}
                  onClick={() => setDialog({ kind: "edit", service: s })}
                />
              </div>
              <Button variant="ghost" small icon="check" onClick={() => setDialog({ kind: "done", service: s })}>
                {tr("Erledigt")}
              </Button>
              <Button variant="ghost" small icon="trash" title={tr("Entfernen")} onClick={() => removeService(car.vin, s.id)} />
            </div>
          );
        })}
        {car.services.length === 0 ? (
          <FindingRow
            icon="wrench"
            tint="var(--info)"
            title={tr("Noch keine Wartung eingetragen")}
            text={tr("Ölwechsel, Inspektion, HU und mehr: einmal eintragen, Caruspex erinnert beim Verbinden.")}
          />
        ) : null}
      </div>
      {dialog?.kind === "add" ? <ServiceDialog vin={car.vin} odometer={odometer} edit={null} onClose={() => setDialog(null)} /> : null}
      {dialog?.kind === "edit" ? <ServiceDialog vin={car.vin} odometer={odometer} edit={dialog.service} onClose={() => setDialog(null)} /> : null}
      {dialog?.kind === "done" ? <DoneDialog vin={car.vin} service={dialog.service} odometer={odometer} onClose={() => setDialog(null)} /> : null}
    </div>
  );
}

function TripsTab({ car }: { car: GarageCar }) {
  let distance = 0;
  let fuel = 0;
  let cost = 0;
  const rows: ReactNode[] = [];
  for (let i = car.trips.length - 1; i >= 0; i--) {
    const trip = car.trips[i]!;
    distance += trip.distanceKm;
    fuel += trip.fuelLitres;
    cost += trip.cost;
    let text = tr("%1 km, %2 l/100 km", localeNumber(trip.distanceKm, 1), trip.averageL100 > 0 ? localeNumber(trip.averageL100, 1) : "--");
    if (trip.cost > 0) text += tr(", %1 €", localeNumber(trip.cost, 2));
    if (trip.maxSpeedKmh > 0) text += tr(", höchstens %1 km/h", localeNumber(trip.maxSpeedKmh, 0));
    rows.push(
      <FindingRow
        key={`${trip.start}-${i}`}
        icon="route"
        tint="var(--accent)"
        title={shortDateTime(trip.start)}
        text={text}
        meta={trip.ecoScore >= 0 ? tr("Fahrstil %1", trip.ecoScore) : undefined}
      />,
    );
  }
  return (
    <div className="garage-tab">
      <div className="garage-tiles">
        <StatTile icon="route" caption={tr("Strecke gesamt")} value={localeNumber(distance, 0)} unit="km" />
        <StatTile icon="fuel" caption={tr("Verbrauch im Schnitt")} value={distance > 1 ? localeNumber((fuel / distance) * 100, 1) : "--"} unit="l/100 km" />
        <StatTile icon="file" caption={tr("Kosten gesamt")} value={cost > 0 ? localeNumber(cost, 2) : "--"} unit="€" />
      </div>
      <div className="garage-rows">
        {rows}
        {car.trips.length === 0 ? (
          <FindingRow
            icon="route"
            tint="var(--info)"
            title={tr("Noch keine Fahrt gespeichert")}
            text={tr("Der Bordcomputer legt hier jede Fahrt ab, sobald sie zurückgesetzt oder die Verbindung getrennt wird.")}
          />
        ) : null}
      </div>
    </div>
  );
}

function FuelTab({ car, odometer }: { car: GarageCar; odometer: number }) {
  const [adding, setAdding] = useState(false);
  let total = 0;
  const rows: ReactNode[] = [];
  for (let i = car.fuel.length - 1; i >= 0; i--) {
    const f = car.fuel[i]!;
    const price = f.litres * f.pricePerLitre;
    total += price;
    let text = tr("%1 l zu %2 €/l", localeNumber(f.litres, 2), localeNumber(f.pricePerLitre, 3));
    if (f.odometerKm >= 0) text += tr(", bei %1 km", localeNumber(f.odometerKm, 0));
    if (!f.full) text += tr(", nicht voll getankt");
    const index = i;
    rows.push(
      <div key={`${f.date}-${f.odometerKm}-${i}`} className="garage-line fade-in">
        <div className="grow">
          <FindingRow icon="fuel" tint="var(--accent)" title={shortDate(f.date)} text={text} meta={price > 0 ? `${localeNumber(price, 2)} €` : undefined} />
        </div>
        <Button variant="ghost" small icon="trash" title={tr("Entfernen")} onClick={() => removeFuel(car.vin, index)} />
      </div>,
    );
  }
  const consumption = fuelLogConsumption(car);
  return (
    <div className="garage-tab">
      <div className="garage-tiles">
        <StatTile
          icon="fuel"
          caption={tr("Verbrauch laut Tankbuch")}
          value={consumption > 0 ? localeNumber(consumption, 1) : "--"}
          unit="l/100 km"
          note={consumption > 0 ? undefined : tr("braucht zwei volle Tankfüllungen")}
        />
        <StatTile icon="file" caption={tr("Getankt gesamt")} value={total > 0 ? localeNumber(total, 2) : "--"} unit="€" />
        <div className="garage-tiles-action">
          <Button icon="fuel" onClick={() => setAdding(true)}>
            {tr("Tankfüllung eintragen")}
          </Button>
        </div>
      </div>
      <div className="garage-rows">
        {rows}
        {car.fuel.length === 0 ? (
          <FindingRow
            icon="fuel"
            tint="var(--info)"
            title={tr("Noch nichts eingetragen")}
            text={tr("Mit zwei vollen Tankfüllungen samt Kilometerstand rechnet Caruspex den echten Verbrauch aus.")}
          />
        ) : null}
      </div>
      {adding ? <FuelDialog vin={car.vin} odometer={odometer} onClose={() => setAdding(false)} /> : null}
    </div>
  );
}

// The note is saved shortly after typing stops, and right away when another
// car is picked, so nothing typed gets lost.
function NoteTab({ car }: { car: GarageCar }) {
  const [text, setText] = useState(car.note);
  const pending = useRef<{ vin: string; text: string } | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const flush = () => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    const p = pending.current;
    pending.current = null;
    if (p) setNote(p.vin, p.text);
  };

  useEffect(() => {
    flush();
    setText(car.note);
    // Only a change of car resets the field; the note itself follows below.
  }, [car.vin]);

  useEffect(() => {
    // A change from elsewhere (an import) shows, unless something is being typed.
    if (!pending.current && car.note !== text) setText(car.note);
  }, [car.note]);

  useEffect(() => () => flush(), []);

  return (
    <div className="garage-tab">
      <textarea
        className="textarea garage-note"
        value={text}
        placeholder={tr("z.B. Zahnriemen bei 180 000 km gewechselt")}
        onChange={(e) => {
          setText(e.target.value);
          pending.current = { vin: car.vin, text: e.target.value };
          if (timer.current) clearTimeout(timer.current);
          timer.current = setTimeout(flush, 700);
        }}
      />
    </div>
  );
}

// --- Page -------------------------------------------------------------------------------------------

export default function GaragePage() {
  const cars = useGarage((s) => s.cars);
  const sessionVin = useSession((s) => s.vin);
  const identity = useSession((s) => s.identity);
  const [selected, setSelected] = useState("");
  const [current, setCurrent] = useState("");
  const [currentKm, setCurrentKm] = useState(-1);
  const [tab, setTab] = useState(0);

  // The connected car is marked and picked once it is recognised and in the
  // garage (MainWindow called setCurrentVin right after Garage::touch).
  useEffect(() => {
    if (!sessionVin) {
      setCurrent("");
      return;
    }
    if (!identity) return;
    setCurrent(sessionVin);
    setSelected(sessionVin);
  }, [sessionVin, identity]);

  // Odometer of the connected car, for the service plan.
  useBackendEvent("live_value", ({ row }) => {
    if (row.pid === 0xa6) setCurrentKm(row.value);
  });

  useAppEvent<string>("dev:page", (name) => {
    if (!name.startsWith("garage:")) return;
    navigate("garage");
    const index = Number.parseInt(name.slice(7), 10);
    if (index >= 0 && index <= 4) setTab(index);
  });

  // Most recently seen first.
  const sorted = useMemo(() => [...cars].sort((a, b) => (a.lastSeen > b.lastSeen ? -1 : a.lastSeen < b.lastSeen ? 1 : 0)), [cars]);
  const car = cars.find((c) => c.vin === selected) ?? cars[cars.length - 1];
  const odometer = car ? (car.vin === current && currentKm >= 0 ? currentKm : latestOdometer(car.vin)) : -1;

  const exportGarage = async () => {
    try {
      const path = await call("save_text_file", {
        title: tr("Garage sichern"),
        suggestedName: `caruspex-garage-${isoDate()}.json`,
        filters: [{ name: tr("Garage (*.json)"), extensions: ["json"] }],
        content: serialiseGarage(),
      });
      if (path) toast(tr("Garage gesichert: %1", path), "success");
    } catch {
      toast(tr("Sichern hat nicht geklappt."), "error");
    }
  };

  const importFile = async () => {
    let file;
    try {
      file = await call("open_text_file", { title: tr("Sicherung laden"), filters: [{ name: tr("Garage (*.json)"), extensions: ["json"] }] });
    } catch (error) {
      toast(tr("Laden hat nicht geklappt: %1", String(error)), "error");
      return;
    }
    if (!file) return;
    try {
      const count = importGarage(file.content);
      toast(tr.n("%n Auto(s) übernommen, nichts wurde gelöscht.", count), "success");
    } catch (error) {
      toast(tr("Laden hat nicht geklappt: %1", error instanceof Error ? error.message : String(error)), "error");
    }
  };

  const remove = async () => {
    if (!car) return;
    const ok = await confirm({
      title: tr("Entfernen"),
      text: tr("Das Auto und seinen ganzen Verlauf aus der Garage entfernen? Das lässt sich nicht rückgängig machen."),
      confirm: tr("Entfernen"),
      cancel: cancelText(),
      danger: true,
    });
    if (!ok) return;
    const vin = car.vin;
    setSelected("");
    removeCar(vin);
  };

  const meta: string[] = [];
  if (car) {
    if (car.description) meta.push(car.description);
    meta.push(tr("VIN %1", car.vin));
    meta.push(tr("zuerst gesehen %1", shortDate(car.firstSeen)));
  }
  const isCurrent = !!car && car.vin === current;

  return (
    <Page>
      <PageHeader
        eyebrow={tr("Garage")}
        title={tr("Deine Fahrzeuge")}
        subtitle={tr(
          "Jedes Auto, das Caruspex einmal gesehen hat, erkannt an seiner Fahrgestellnummer. Mit allen Prüfungen und Kilometerständen. Bleibt auf diesem Rechner.",
        )}
        actions={
          <>
            <Button variant="ghost" icon="download" onClick={() => void exportGarage()}>
              {tr("Garage sichern")}
            </Button>
            <Button variant="ghost" icon="refresh" onClick={() => void importFile()}>
              {tr("Sicherung laden")}
            </Button>
          </>
        }
      />

      {car ? (
        <div className="garage-columns">
          <div className="garage-list">
            {sorted.map((c) => (
              <CarCard key={c.vin} car={c} selected={c.vin === car.vin} current={c.vin === current} onClick={() => setSelected(c.vin)} />
            ))}
          </div>

          <Card className="garage-detail">
            <div className="garage-head">
              <VehicleDiagram className="garage-picture" body={car.bodyStyle} />
              <div className="garage-head-text">
                <h2 className="page-title selectable">{car.name || tr("Unbekanntes Fahrzeug")}</h2>
                <div className="subtle selectable">{meta.join(" · ")}</div>
                <div className="row wrap-row garage-actions">
                  <Button icon="file" disabled={!isCurrent} title={isCurrent ? undefined : tr("Der Bericht braucht das verbundene Auto.")} onClick={openReport}>
                    {tr("Prüfbericht")}
                  </Button>
                  <Button
                    icon="printer"
                    title={tr("Prüfungen, Wartung, Fahrten und Tankbuch, auch ohne verbundenes Auto")}
                    onClick={() => openGarageReport(car.vin)}
                  >
                    {tr("Akte als PDF")}
                  </Button>
                  <Button variant="ghost" icon="trash" title={tr("Aus der Garage entfernen")} onClick={() => void remove()}>
                    {tr("Entfernen")}
                  </Button>
                </div>
              </div>
            </div>

            <Tabs
              value={tab}
              onChange={setTab}
              tabs={[
                { value: 0, label: tr("Verlauf") },
                { value: 1, label: tr("Wartung") },
                { value: 2, label: tr("Fahrten") },
                { value: 3, label: tr("Tankbuch") },
                { value: 4, label: tr("Notiz") },
              ]}
            />
            {tab === 0 ? <HistoryTab car={car} /> : null}
            {tab === 1 ? <ServicesTab car={car} odometer={odometer} /> : null}
            {tab === 2 ? <TripsTab car={car} /> : null}
            {tab === 3 ? <FuelTab car={car} odometer={odometer} /> : null}
            {tab === 4 ? <NoteTab car={car} /> : null}
          </Card>
        </div>
      ) : (
        <Card className="garage-empty">
          <EmptyState icon="garage" title={tr("Die Garage ist noch leer")}>
            <div className="subtle garage-empty-text">
              {tr(
                "Sobald ein Auto verbunden ist und seine Fahrgestellnummer verrät, steht es hier. Ältere Autos ohne VIN im Steuergerät lassen sich leider nicht wiedererkennen.",
              )}
            </div>
          </EmptyState>
        </Card>
      )}
    </Page>
  );
}
