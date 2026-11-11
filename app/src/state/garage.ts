// The garage (Garage.cpp): every car Caruspex has seen, recognised by its
// VIN, with the history of its checks, its trips, a fuel log and its service
// plan. The backend only stores the file; the rules live here.
import { call } from "../api";
import type { GarageCar, GarageCheck, GarageFuel, GarageService, GarageTrip } from "../api";
import { t, tn } from "../i18n";
import { createStore, useStore } from "../lib/store";

const G = "caruspex::Garage";

export const garageStore = createStore<{ cars: GarageCar[]; loaded: boolean }>({ cars: [], loaded: false });

export function useGarage<S>(selector: (s: { cars: GarageCar[]; loaded: boolean }) => S): S {
  return useStore(garageStore, selector);
}

// Local time as ISO 8601 without zone, like QDateTime::toString(Qt::ISODate).
export function isoNow(date = new Date()): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${p(date.getMonth() + 1)}-${p(date.getDate())}T${p(date.getHours())}:${p(date.getMinutes())}:${p(date.getSeconds())}`;
}

export function isoDate(date = new Date()): string {
  return isoNow(date).slice(0, 10);
}

export function parseIso(text: string): Date | null {
  if (!text) return null;
  const d = new Date(text.length === 10 ? `${text}T00:00:00` : text);
  return Number.isNaN(d.getTime()) ? null : d;
}

function uuid(): string {
  return typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `${Date.now().toString(16)}-${Math.random().toString(16).slice(2)}`;
}

let saveTimer: ReturnType<typeof setTimeout> | null = null;

function commit(cars: GarageCar[]): void {
  garageStore.set({ cars });
  // Bundles quick successive changes into one write.
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveTimer = null;
    void call("garage_save", { cars: garageStore.get().cars });
  }, 150);
}

function update(vin: string, change: (car: GarageCar) => GarageCar | null): void {
  const cars = garageStore.get().cars;
  const i = cars.findIndex((c) => c.vin === vin);
  if (i < 0) return;
  const next = change(structuredClone(cars[i]!));
  if (!next) return;
  const copy = [...cars];
  copy[i] = next;
  commit(copy);
}

export async function loadGarage(): Promise<void> {
  try {
    const cars = (await call("garage_load")) ?? [];
    garageStore.set({ cars: cars.filter((c) => c.vin), loaded: true });
  } catch {
    garageStore.set({ cars: [], loaded: true });
  }
}

export function findCar(vin: string): GarageCar | undefined {
  return garageStore.get().cars.find((c) => c.vin === vin);
}

// Templates for the "add service" dialog. Common intervals; the owner's
// manual knows the exact ones.
export function serviceTemplates(): GarageService[] {
  const make = (title: string, km: number, months: number): GarageService => ({
    id: "",
    title,
    intervalKm: km,
    intervalMonths: months,
    lastDate: "",
    lastKm: -1,
    note: "",
  });
  return [
    make(t(G, "Ölwechsel"), 15000, 12),
    make(t(G, "Inspektion"), 30000, 24),
    make(t(G, "Hauptuntersuchung (HU/AU)"), 0, 24),
    make(t(G, "Zahnriemen"), 120000, 72),
    make(t(G, "Bremsflüssigkeit"), 0, 24),
    make(t(G, "Innenraumfilter"), 15000, 12),
    make(t(G, "Luftfilter"), 30000, 24),
    make(t(G, "Zündkerzen"), 60000, 48),
    make(t(G, "Kühlmittel"), 0, 60),
    make(t(G, "Reifen wechseln (Sommer/Winter)"), 0, 6),
  ];
}

// Creates or updates the entry and marks it as seen now.
export function touchCar(vin: string, name: string, description: string, bodyStyle: number): void {
  if (!vin) return;
  const now = isoNow();
  const cars = [...garageStore.get().cars];
  let i = cars.findIndex((c) => c.vin === vin);
  if (i < 0) {
    // A new car starts with the items almost everybody needs.
    const services = serviceTemplates()
      .filter((s) => s.title === t(G, "Ölwechsel") || s.title === t(G, "Hauptuntersuchung (HU/AU)"))
      .map((s) => ({ ...s, id: uuid() }));
    cars.push({
      vin,
      name: "",
      description: "",
      bodyStyle: 0,
      note: "",
      firstSeen: now,
      lastSeen: now,
      checks: [],
      trips: [],
      fuel: [],
      services,
    });
    i = cars.length - 1;
  }
  const car = { ...cars[i]! };
  if (name) car.name = name;
  if (description) car.description = description;
  car.bodyStyle = bodyStyle;
  car.lastSeen = now;
  cars[i] = car;
  commit(cars);
}

// Adds a check. A second check within fifteen minutes replaces the first, so
// reading the faults three times in a row stays one entry.
export function addCheck(vin: string, check: GarageCheck): void {
  update(vin, (car) => {
    const last = car.checks[car.checks.length - 1];
    const lastWhen = last ? parseIso(last.when) : null;
    const when = parseIso(check.when);
    if (last && lastWhen && when && (when.getTime() - lastWhen.getTime()) / 1000 < 15 * 60) {
      car.checks[car.checks.length - 1] = check;
    } else {
      car.checks.push(check);
    }
    car.lastSeen = check.when;
    return car;
  });
}

export function setNote(vin: string, note: string): void {
  update(vin, (car) => ({ ...car, note }));
}

export function removeCar(vin: string): void {
  commit(garageStore.get().cars.filter((c) => c.vin !== vin));
}

export function addTrip(vin: string, trip: GarageTrip): void {
  update(vin, (car) => ({ ...car, trips: [...car.trips, trip] }));
}

export function addFuel(vin: string, fuel: GarageFuel): void {
  update(vin, (car) => {
    const list = [...car.fuel, fuel];
    list.sort((a, b) => (a.date !== b.date ? (a.date < b.date ? -1 : 1) : a.odometerKm - b.odometerKm));
    return { ...car, fuel: list };
  });
}

export function removeFuel(vin: string, index: number): void {
  update(vin, (car) => {
    if (index < 0 || index >= car.fuel.length) return null;
    return { ...car, fuel: car.fuel.filter((_, i) => i !== index) };
  });
}

// Adds or replaces (same id) a service item.
export function setService(vin: string, service: GarageService): void {
  update(vin, (car) => {
    const copy = { ...service, id: service.id || uuid() };
    const i = car.services.findIndex((s) => s.id === copy.id);
    const services = [...car.services];
    if (i >= 0) services[i] = copy;
    else services.push(copy);
    return { ...car, services };
  });
}

export function removeService(vin: string, id: string): void {
  update(vin, (car) => ({ ...car, services: car.services.filter((s) => s.id !== id) }));
}

export function markServiceDone(vin: string, id: string, date: string, km: number): void {
  update(vin, (car) => ({
    ...car,
    services: car.services.map((s) => (s.id === id ? { ...s, lastDate: date, lastKm: km } : s)),
  }));
}

// Latest known odometer reading of a car: last check or fuel entry.
export function latestOdometer(vin: string): number {
  const car = findCar(vin);
  if (!car) return -1;
  let best = -1;
  for (const c of car.checks) best = Math.max(best, c.odometerKm);
  for (const f of car.fuel) best = Math.max(best, f.odometerKm);
  return best;
}

export type ServiceState = "ok" | "soon" | "due" | "unknown";

export interface ServiceStatus {
  service: GarageService;
  state: ServiceState;
  kmLeft: number;
  daysLeft: number;
  byKm: boolean;
  text: string;
}

function addMonths(date: Date, months: number): Date {
  const d = new Date(date.getTime());
  const day = d.getDate();
  d.setDate(1);
  d.setMonth(d.getMonth() + months);
  const last = new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate();
  d.setDate(Math.min(day, last));
  return d;
}

function daysBetween(a: Date, b: Date): number {
  const ua = Date.UTC(a.getFullYear(), a.getMonth(), a.getDate());
  const ub = Date.UTC(b.getFullYear(), b.getMonth(), b.getDate());
  return Math.round((ub - ua) / 86400000);
}

// Every service item with its state, most urgent first. currentKm < 0 means
// unknown.
export function serviceStatus(car: GarageCar, currentKm: number, today = new Date()): ServiceStatus[] {
  const out: ServiceStatus[] = [];
  for (const s of car.services) {
    const st: ServiceStatus = { service: s, state: "unknown", kmLeft: 0, daysLeft: 0, byKm: false, text: "" };
    let haveKm = false;
    let haveDays = false;
    const last = parseIso(s.lastDate);
    if (s.intervalKm > 0 && s.lastKm >= 0 && currentKm >= 0) {
      st.kmLeft = Math.round(s.lastKm + s.intervalKm - currentKm);
      haveKm = true;
    }
    if (s.intervalMonths > 0 && last) {
      st.daysLeft = daysBetween(today, addMonths(last, s.intervalMonths));
      haveDays = true;
    }
    if (!haveKm && !haveDays) {
      st.text =
        last || s.lastKm >= 0
          ? t(G, "Kilometerstand unbekannt, beim nächsten Verbinden berechnet")
          : t(G, "Noch nicht eingetragen, wann es zuletzt gemacht wurde");
      out.push(st);
      continue;
    }
    // Whichever limit comes first decides. A kilometre is roughly worth a
    // day here; good enough to pick the closer one.
    st.byKm = haveKm && (!haveDays || Math.trunc(st.kmLeft / 40) < st.daysLeft);
    const over = (haveKm && st.kmLeft < 0) || (haveDays && st.daysLeft < 0);
    const soon = (haveKm && st.kmLeft < 1500) || (haveDays && st.daysLeft < 30);
    st.state = over ? "due" : soon ? "soon" : "ok";
    if (st.byKm) {
      st.text = st.kmLeft >= 0 ? t(G, "noch %1 km", st.kmLeft) : t(G, "seit %1 km überfällig", -st.kmLeft);
    } else if (st.daysLeft >= 60) {
      st.text = tn(G, "noch etwa %n Monat(e)", Math.trunc(st.daysLeft / 30));
    } else if (st.daysLeft >= 0) {
      st.text = tn(G, "noch %n Tag(e)", st.daysLeft);
    } else {
      st.text = tn(G, "seit %n Tag(en) überfällig", -st.daysLeft);
    }
    out.push(st);
  }
  const rank: Record<ServiceState, number> = { due: 0, soon: 1, ok: 2, unknown: 3 };
  return out
    .map((s, i) => ({ s, i }))
    .sort((a, b) => rank[a.s.state] - rank[b.s.state] || a.i - b.i)
    .map((x) => x.s);
}

export interface OdometerJump {
  earlier: string;
  earlierKm: number;
  later: string;
  laterKm: number;
}

// Odometer readings that went down between two checks: a hint, not proof.
export function odometerJumps(car: GarageCar): OdometerJump[] {
  const jumps: OdometerJump[] = [];
  let previous: GarageCheck | null = null;
  for (const c of car.checks) {
    if (c.odometerKm < 0) continue;
    // More than a kilometre back cannot be rounding.
    if (previous && c.odometerKm + 1 < previous.odometerKm) {
      jumps.push({ earlier: previous.when, earlierKm: previous.odometerKm, later: c.when, laterKm: c.odometerKm });
    }
    previous = c;
  }
  return jumps;
}

// Consumption between full tanks: litres per 100 km, -1 if not enough data.
// Classic full-tank method: from the first full tank, add every litre until
// the last full tank and divide by the distance between them.
export function fuelLogConsumption(car: GarageCar): number {
  let first = -1;
  let last = -1;
  car.fuel.forEach((f, i) => {
    if (!f.full || f.odometerKm < 0) return;
    if (first < 0) first = i;
    last = i;
  });
  if (first < 0 || last <= first) return -1;
  const distance = car.fuel[last]!.odometerKm - car.fuel[first]!.odometerKm;
  if (distance < 50) return -1;
  let litres = 0;
  for (let i = first + 1; i <= last; i++) litres += car.fuel[i]!.litres;
  return (litres / distance) * 100;
}

// The whole garage as the JSON file of the Qt version, and back.
export function serialiseGarage(cars: GarageCar[] = garageStore.get().cars): string {
  return JSON.stringify({ version: 2, cars }, null, 4);
}

// Merges by VIN; for a car that exists already, entries are added, nothing
// is removed. Returns the number of cars, or throws with the parse error.
export function importGarage(text: string): number {
  const doc = JSON.parse(text) as { cars?: Partial<GarageCar>[] };
  const incoming: GarageCar[] = (doc.cars ?? [])
    .filter((c) => typeof c.vin === "string" && c.vin !== "")
    .map((c) => ({
      vin: c.vin ?? "",
      name: c.name ?? "",
      description: c.description ?? "",
      bodyStyle: c.bodyStyle ?? 0,
      note: c.note ?? "",
      firstSeen: c.firstSeen ?? "",
      lastSeen: c.lastSeen ?? "",
      checks: c.checks ?? [],
      trips: c.trips ?? [],
      fuel: c.fuel ?? [],
      services: (c.services ?? []).map((s) => ({ ...s, id: s.id || uuid() })),
    }));
  const cars = structuredClone(garageStore.get().cars);
  let count = 0;
  for (const car of incoming) {
    const mine = cars.find((c) => c.vin === car.vin);
    if (!mine) {
      cars.push(car);
      count++;
      continue;
    }
    for (const c of car.checks) if (!mine.checks.some((m) => m.when === c.when)) mine.checks.push(c);
    mine.checks.sort((a, b) => (a.when < b.when ? -1 : a.when > b.when ? 1 : 0));
    for (const tr of car.trips) if (!mine.trips.some((m) => m.start === tr.start)) mine.trips.push(tr);
    for (const f of car.fuel)
      if (!mine.fuel.some((m) => m.date === f.date && Math.abs(m.odometerKm - f.odometerKm) < 0.5)) mine.fuel.push(f);
    for (const s of car.services) if (!mine.services.some((m) => m.id === s.id)) mine.services.push(s);
    count++;
  }
  commit(cars);
  return count;
}
