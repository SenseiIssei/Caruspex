// From a VIN to something a person recognises (VehicleIdentity.cpp): "Audi
// A2 (8Z)", the year, the right body for the picture and the brand for maker
// codes. The catalog lookups come from the backend.
import { call } from "../api";
import type { BodyStyle, VehicleGenerationInfo } from "../api";
import { BodyType } from "../api";
import { t } from "../i18n";

const V = "caruspex::VehicleIdentity";

export const DEMO_VIN = "YCXDE15C7MC048217";

// app/VehicleDiagram.h BodyStyle.
export const BodyStyles = {
  Hatchback: 0,
  Sedan: 1,
  Estate: 2,
  Suv: 3,
  Van: 4,
  Coupe: 5,
  Pickup: 6,
  Microcar: 7,
} as const;

export interface IdentifiedVehicle {
  known: boolean;
  title: string;
  subtitle: string;
  brand: string;
  year: number;
  confidence: number;
  body: BodyStyle;
  generation: VehicleGenerationInfo | null;
  socketHint: string;
}

export function bodyStyleFor(type: BodyType): BodyStyle {
  switch (type) {
    case BodyType.Hatchback: return BodyStyles.Hatchback;
    case BodyType.Sedan: return BodyStyles.Sedan;
    case BodyType.Estate: return BodyStyles.Estate;
    case BodyType.Suv: return BodyStyles.Suv;
    case BodyType.Van:
    case BodyType.Mpv: return BodyStyles.Van;
    case BodyType.Coupe:
    case BodyType.Convertible:
    case BodyType.Roadster: return BodyStyles.Coupe;
    case BodyType.Pickup: return BodyStyles.Pickup;
    case BodyType.Microcar: return BodyStyles.Microcar;
  }
  return BodyStyles.Hatchback;
}

function empty(): IdentifiedVehicle {
  return { known: false, title: "", subtitle: "", brand: "", year: 0, confidence: 0, body: BodyStyles.Hatchback, generation: null, socketHint: "" };
}

// The same for a generation picked by hand.
export async function identifyGeneration(g: VehicleGenerationInfo, year: number): Promise<IdentifiedVehicle> {
  const parts: string[] = [];
  if (year > 0) parts.push(t(V, "Baujahr %1", year));
  parts.push(g.yearsText);
  if (g.powertrainText) parts.push(g.powertrainText);
  parts.push(g.bodyName);
  let socketHint = "";
  try {
    socketHint = (await call("catalog_diagnosis", { generationId: g.id, year: year > 0 ? year : g.yearFrom })).locationText;
  } catch {
    socketHint = "";
  }
  return {
    known: true,
    title: g.displayName,
    subtitle: parts.join(" · "),
    brand: g.brand,
    year,
    confidence: 100,
    body: bodyStyleFor(g.body),
    generation: g,
    socketHint,
  };
}

export async function identifyVin(vinText: string): Promise<IdentifiedVehicle> {
  const vin = vinText.trim().toUpperCase();
  const v = empty();
  if (!vin) return v;

  // The built-in demo car has a made-up maker code; give it a proper name.
  if (vin === DEMO_VIN) {
    return {
      ...v,
      known: true,
      title: t(V, "Caruspex Demo-Auto"),
      subtitle: t(V, "Baujahr 2021 · 1.4 Benziner · Automatik · simuliert"),
      year: 2021,
      confidence: 100,
      body: BodyStyles.Hatchback,
    };
  }

  try {
    const [info, match] = await Promise.all([call("decode_vin", { vin }), call("match_vin", { vin })]);
    if (match?.generation) {
      const out = await identifyGeneration(match.generation, match.year);
      out.confidence = match.confidence;
      if (match.confidence < 60) out.subtitle += ` · ${t(V, "vermutet")}`;
      return out;
    }
    const brand = match?.brand || info?.brand || "";
    if (brand) {
      const year = match && match.year > 0 ? match.year : (info?.modelYear ?? 0);
      const parts: string[] = [];
      if (year > 0) parts.push(t(V, "Baujahr etwa %1", year));
      if (info?.country) parts.push(info.country);
      parts.push(t(V, "Modell nicht eindeutig"));
      return { ...v, known: true, brand, title: brand, year, subtitle: parts.join(" · ") };
    }
  } catch {
    // Lookups unavailable: fall through to "unknown".
  }
  return {
    ...v,
    title: t(V, "Unbekannter Hersteller"),
    subtitle: t(V, "Die Fahrgestellnummer passt zu keinem bekannten Hersteller."),
  };
}
