// The short text summary of a check that goes into the QR code: car, VIN,
// codes, score and date, readable as plain text by any phone camera. No
// account, no server: everything is in the code itself.
import type { GarageCar } from "../api";
import { localeDate, t } from "../i18n";
import { session } from "../state/session";

const US = "caruspex::Usability";

export interface ShareFacts {
  car: string;
  vin: string;
  // null: the fault memory was not read.
  codes: string[] | null;
  // 0..100, negative when not rated.
  score: number;
  date: Date;
}

// Up to this many codes are listed; the rest is counted, so the code stays
// small enough for an old phone camera.
const MAX_CODES = 12;

export function shareText(f: ShareFacts): string {
  const lines = ["Caruspex"];
  if (f.car) lines.push(t(US, "Fahrzeug: %1", f.car));
  if (f.vin) lines.push(`VIN: ${f.vin}`);
  if (f.codes === null) lines.push(t(US, "Fehler: nicht gelesen"));
  else if (f.codes.length === 0) lines.push(t(US, "Fehler: keine"));
  else {
    const shown = f.codes.slice(0, MAX_CODES).join(", ");
    const more = f.codes.length - MAX_CODES;
    lines.push(t(US, "Fehler: %1", more > 0 ? `${shown} ${t(US, "und %1 weitere", more)}` : shown));
  }
  if (f.score >= 0) lines.push(t(US, "Punkte: %1 von 100", Math.round(f.score)));
  lines.push(t(US, "Datum: %1", localeDate(f.date)));
  return lines.join("\n");
}

// The car that is connected right now.
export function sessionFacts(): ShareFacts {
  const s = session.get();
  return {
    car: s.identity?.title ?? "",
    vin: s.vin,
    codes: s.haveFaults ? s.faults.map((f) => f.code) : null,
    score: s.score,
    date: new Date(),
  };
}

// A car of the garage, from its last check.
export function garageFacts(car: GarageCar): ShareFacts {
  const last = car.checks[car.checks.length - 1];
  return {
    car: car.name,
    vin: car.vin,
    codes: last ? last.codes : null,
    score: last ? last.score : -1,
    date: last ? new Date(last.when) : new Date(),
  };
}
