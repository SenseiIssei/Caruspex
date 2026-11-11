// Demo answer for the X-ray view: crates/caruspex-vehicles/src/anatomy.rs in
// TypeScript, so the browser preview builds the same car as the app.
import { BodyType, DrivenAxles, EngineLayout, Powertrain, TractionBattery, type VehicleAnatomy } from "../../types";
import { L } from "../core";
import type { PartialHandlers } from "../runtime";
import { catalogGeneration, powertrainText, type MeasuredRecord } from "./knowledge";

// Length, width, height, wheelbase, track from about 1995 to 2009.
const BASE: Record<BodyType, [number, number, number, number, number]> = {
  [BodyType.Hatchback]: [4120, 1720, 1470, 2540, 1490],
  [BodyType.Sedan]: [4620, 1770, 1440, 2710, 1530],
  [BodyType.Estate]: [4600, 1770, 1490, 2690, 1530],
  [BodyType.Suv]: [4520, 1850, 1690, 2670, 1580],
  [BodyType.Van]: [4900, 1910, 1950, 3000, 1630],
  [BodyType.Coupe]: [4450, 1790, 1360, 2620, 1530],
  [BodyType.Convertible]: [4380, 1770, 1390, 2580, 1510],
  [BodyType.Pickup]: [5250, 1860, 1790, 3090, 1570],
  [BodyType.Mpv]: [4560, 1800, 1640, 2760, 1540],
  [BodyType.Microcar]: [2700, 1560, 1550, 1870, 1360],
  [BodyType.Roadster]: [4020, 1740, 1270, 2420, 1480],
};

function typicalDimensions(body: BodyType, year: number) {
  const [length, width, height, wheelbase, track] = BASE[body] ?? BASE[BodyType.Hatchback];
  let [grow, wide, wheel] = year > 0 && year < 1995 ? [960, 950, 590] : year >= 2010 ? [1030, 1035, 670] : [1000, 1000, 630];
  if (body === BodyType.Suv || body === BodyType.Pickup) wheel += 80;
  else if (body === BodyType.Van) wheel += 30;
  else if (body === BodyType.Microcar) wheel -= 60;
  const scale = (v: number, perMille: number) => Math.floor((v * perMille + 500) / 1000);
  return {
    lengthMm: scale(length, grow),
    widthMm: scale(width, wide),
    heightMm: height,
    wheelbaseMm: scale(wheelbase, grow),
    trackMm: scale(track, wide),
    wheelMm: wheel,
  };
}

type Dims = ReturnType<typeof typicalDimensions>;

// anatomy::merge_measured: the real values put in, a missing one following
// the real ones in proportion. Returns the dimensions and the bits of the
// real values.
function mergeMeasured(typical: Dims, m: MeasuredRecord): [Dims, number] {
  const [len, width, height, wb, track] = m;
  const ratio = (real: number, typ: number, value: number) => (real === 0 || typ === 0 ? value : Math.floor((value * real + Math.floor(typ / 2)) / typ));
  const d = { ...typical };
  let bits = 0;
  const [length, wheelbase] =
    len === 0 && wb === 0
      ? [typical.lengthMm, typical.wheelbaseMm]
      : len === 0
        ? [ratio(wb, typical.wheelbaseMm, typical.lengthMm), wb]
        : wb === 0
          ? [len, ratio(len, typical.lengthMm, typical.wheelbaseMm)]
          : [len, wb];
  d.lengthMm = length;
  d.wheelbaseMm = Math.min(wheelbase, Math.floor((length * 4) / 5));
  if (len > 0) bits |= 1;
  if (wb > 0 && d.wheelbaseMm === wb) bits |= 8;
  if (width > 0) {
    d.widthMm = width;
    bits |= 2;
  }
  if (track > 0) {
    d.trackMm = track;
    bits |= 16;
  } else {
    d.trackMm = ratio(d.widthMm, typical.widthMm, typical.trackMm);
  }
  d.trackMm = Math.min(d.trackMm, Math.max(0, d.widthMm - 150));
  if (height > 0) {
    d.heightMm = height;
    bits |= 4;
  }
  d.wheelMm = Math.min(d.wheelMm, Math.floor(d.heightMm / 2));
  return [d, bits];
}

const ORDER = [Powertrain.Petrol, Powertrain.Diesel, Powertrain.Gas, Powertrain.Hybrid, Powertrain.PlugIn, Powertrain.Electric];

function defaultPowertrain(flags: number): number {
  return ORDER.find((f) => flags & f) ?? Powertrain.Petrol;
}

const { FrontTransverse: Across, FrontLongitudinal: Long, Mid, Rear: Back } = EngineLayout;
const { Front, Rear, All } = DrivenAxles;

// [brand, model prefix, from, to, engine, driven]; the first match wins.
type Row = [string, string, number, number, EngineLayout, DrivenAxles];
const ex = (brand: string, model: string, engine: EngineLayout, driven: DrivenAxles): Row => [brand, model, 0, 9999, engine, driven];
const exYears = (brand: string, model: string, from: number, to: number, engine: EngineLayout, driven: DrivenAxles): Row => [
  brand, model, from, to, engine, driven,
];

const EXCEPTIONS: Row[] = [
  ex("Porsche", "911", Back, Rear), ex("Porsche", "718", Mid, Rear), ex("Porsche", "Boxster", Mid, Rear),
  ex("Porsche", "Cayman", Mid, Rear), ex("Porsche", "Carrera GT", Mid, Rear), ex("Porsche", "918", Mid, All),
  ex("Porsche", "Cayenne", Long, All), ex("Porsche", "Macan", Long, All), ex("Porsche", "", Long, Rear),
  ex("Volkswagen", "Käfer", Back, Rear), ex("Smart", "Fortwo", Back, Rear), ex("Smart", "Roadster", Back, Rear),
  exYears("Smart", "Forfour", 2014, 9999, Back, Rear), exYears("Renault", "Twingo", 2014, 2024, Back, Rear),
  ex("DeLorean", "", Back, Rear), ex("Škoda", "105", Back, Rear),
  ex("Ferrari", "456", Long, Rear), ex("Ferrari", "550", Long, Rear), ex("Ferrari", "575", Long, Rear),
  ex("Ferrari", "599", Long, Rear), ex("Ferrari", "612", Long, Rear), ex("Ferrari", "812", Long, Rear),
  ex("Ferrari", "California", Long, Rear), ex("Ferrari", "F12", Long, Rear), ex("Ferrari", "FF", Long, All),
  ex("Ferrari", "Portofino", Long, Rear), ex("Ferrari", "Purosangue", Long, All), ex("Ferrari", "Roma", Long, Rear),
  ex("Ferrari", "", Mid, Rear), ex("Lamborghini", "Urus", Long, All), ex("Lamborghini", "", Mid, All),
  ex("McLaren", "", Mid, Rear), ex("Lotus", "Elise", Mid, Rear), ex("Lotus", "Exige", Mid, Rear),
  ex("Lotus", "Evora", Mid, Rear), ex("Lotus", "Emira", Mid, Rear), ex("Lotus", "Esprit", Mid, Rear),
  ex("Audi", "R8", Mid, All), ex("Honda", "NSX", Mid, Rear), ex("Alpine", "A110", Mid, Rear),
  ex("Alpine", "A610", Back, Rear), ex("Toyota", "MR2", Mid, Rear), ex("Alfa Romeo", "4C", Mid, Rear),
  ex("Audi", "A1", Across, Front), ex("Audi", "A2", Across, Front), ex("Audi", "A3", Across, Front),
  ex("Audi", "Q2", Across, Front), ex("Audi", "Q3", Across, Front), ex("Audi", "TT", Across, Front),
  ex("Audi", "Q5", Long, All), ex("Audi", "Q7", Long, All), ex("Audi", "Q8", Long, All), ex("Audi", "A8", Long, All),
  ex("Audi", "A6 allroad", Long, All), ex("Audi", "Quattro", Long, All), ex("Audi", "", Long, Front),
  exYears("BMW", "1er", 2019, 9999, Across, Front), ex("BMW", "2er Active", Across, Front),
  ex("BMW", "2er Gran Tourer", Across, Front), ex("BMW", "X2", Across, Front),
  exYears("BMW", "X1", 2015, 9999, Across, Front), ex("BMW", "X", Long, All), ex("BMW", "", Long, Rear),
  ex("Alpina", "", Long, Rear),
  ex("Mercedes-Benz", "A-Klasse", Across, Front), ex("Mercedes-Benz", "B-Klasse", Across, Front),
  ex("Mercedes-Benz", "CLA", Across, Front), ex("Mercedes-Benz", "GLA", Across, Front),
  ex("Mercedes-Benz", "GLB", Across, Front), ex("Mercedes-Benz", "Vaneo", Across, Front),
  ex("Mercedes-Benz", "Citan", Across, Front), ex("Mercedes-Benz", "G-Klasse", Long, All),
  ex("Mercedes-Benz", "GL", Long, All), ex("Mercedes-Benz", "M-Klasse", Long, All),
  ex("Mercedes-Benz", "X-Klasse", Long, All), ex("Mercedes-Benz", "", Long, Rear),
  ex("Jaguar", "", Long, Rear), ex("Land Rover", "Discovery Sport", Across, All),
  ex("Land Rover", "Range Rover Evoque", Across, All), ex("Land Rover", "Freelander", Across, All),
  ex("Land Rover", "", Long, All), ex("Maserati", "", Long, Rear), ex("Aston Martin", "", Long, Rear),
  ex("Bentley", "", Long, All), ex("Rolls-Royce", "", Long, Rear),
  ex("Subaru", "BRZ", Long, Rear), ex("Subaru", "Trezia", Across, Front), ex("Subaru", "Justy", Across, Front),
  ex("Subaru", "", Long, All), ex("Alfa Romeo", "75", Long, Rear), ex("Alfa Romeo", "Alfetta", Long, Rear),
  exYears("Alfa Romeo", "Giulia", 2015, 9999, Long, Rear), ex("Alfa Romeo", "Stelvio", Long, All),
  ex("Lexus", "GS", Long, Rear), ex("Lexus", "IS", Long, Rear), ex("Lexus", "LS", Long, Rear),
  ex("Lexus", "LC", Long, Rear), ex("Lexus", "SC", Long, Rear), ex("Lexus", "LFA", Long, Rear),
  ex("Lexus", "GX", Long, All), ex("Lexus", "LX", Long, All),
  ex("Volvo", "240", Long, Rear), ex("Volvo", "740", Long, Rear), ex("Volvo", "760", Long, Rear),
  ex("Volvo", "940", Long, Rear), ex("Volvo", "960", Long, Rear),
  ex("Toyota", "Supra", Long, Rear), ex("Toyota", "GT86", Long, Rear), ex("Toyota", "GR86", Long, Rear),
  ex("Toyota", "Land Cruiser", Long, All), ex("Toyota", "4Runner", Long, All), ex("Toyota", "FJ Cruiser", Long, All),
  ex("Toyota", "Sequoia", Long, All), ex("Nissan", "GT-R", Long, All), ex("Nissan", "350Z", Long, Rear),
  ex("Nissan", "370Z", Long, Rear), ex("Nissan", "Patrol", Long, All), ex("Mazda", "MX-5", Long, Rear),
  ex("Mazda", "RX-", Long, Rear), ex("Mazda", "CX-60", Long, Rear), ex("Mazda", "CX-80", Long, Rear),
  ex("Mazda", "CX-90", Long, Rear), ex("Jeep", "Wrangler", Long, All), ex("Jeep", "Grand Cherokee", Long, All),
  ex("Jeep", "Gladiator", Long, All), ex("Jeep", "Commander", Long, All), ex("Jeep", "Wagoneer", Long, All),
  ex("Dodge", "Challenger", Long, Rear), ex("Dodge", "Charger", Long, Rear), ex("Dodge", "Viper", Long, Rear),
  ex("Chevrolet", "Camaro", Long, Rear), ex("Chevrolet", "Corvette", Long, Rear), ex("Chevrolet", "Tahoe", Long, Rear),
  ex("Ford", "Mustang", Long, Rear), ex("Ford", "Sierra", Long, Rear), ex("Ford", "Scorpio", Long, Rear),
  ex("Ford", "Granada", Long, Rear), ex("Ford", "Capri", Long, Rear),
  ex("Volkswagen", "Touareg", Long, All), ex("Volkswagen", "Phaeton", Long, All),
  exYears("Volkswagen", "Passat", 1996, 2005, Long, Front), exYears("Škoda", "Superb", 2001, 2008, Long, Front),
];

function rearMotorPlatform(brand: string, model: string): boolean {
  if (["Tesla", "BMW", "Mercedes-Benz", "Porsche", "Smart"].some((b) => b.toLowerCase() === brand.toLowerCase())) {
    return !(brand === "Mercedes-Benz" && (model === "EQA" || model === "EQB"));
  }
  switch (brand) {
    case "Volkswagen": return model.startsWith("ID.");
    case "Škoda": return model.startsWith("Enyaq") || model.startsWith("Elroq");
    case "Cupra": return model.startsWith("Born") || model.startsWith("Tavascan");
    case "Audi": return model.startsWith("Q4");
    case "Renault": return model.startsWith("Twingo");
  }
  return false;
}

function drivetrain(brand: string, model: string, body: BodyType, year: number, variant: number) {
  if (variant === 0) variant = Powertrain.Petrol;
  if (variant & Powertrain.Electric) {
    const dual =
      (body === BodyType.Suv && !["Volkswagen", "Škoda", "Cupra"].includes(brand)) ||
      (brand === "Tesla" && (model.includes("S") || model.includes("X"))) ||
      model.includes("GT");
    const rear = rearMotorPlatform(brand, model);
    return {
      engine: EngineLayout.None,
      driven: dual ? All : rear ? Rear : Front,
      frontMotor: dual || !rear,
      rearMotor: dual || rear,
      battery: TractionBattery.Floor,
      fuelTank: false,
    };
  }
  const row = EXCEPTIONS.find(
    ([b, m, from, to]) => b.toLowerCase() === brand.toLowerCase() && model.startsWith(m) && (year <= 0 || (year >= from && year <= to)),
  );
  let [engine, driven] = row ? [row[4], row[5]] : body === BodyType.Pickup ? [Long, Rear] : [Across, Front];
  if (body === BodyType.Pickup && engine === Long && driven === Front) driven = All;
  const battery = variant === Powertrain.Hybrid ? TractionBattery.Hybrid : variant === Powertrain.PlugIn ? TractionBattery.PlugIn : TractionBattery.None;
  const hybrid = battery !== TractionBattery.None;
  return {
    engine,
    driven,
    frontMotor: hybrid && (engine === Across || engine === Long) && driven !== Rear,
    rearMotor: hybrid && (driven === Rear || engine === Mid || engine === Back),
    battery,
    fuelTank: (variant & (Powertrain.Petrol | Powertrain.Diesel | Powertrain.Gas | Powertrain.Hybrid | Powertrain.PlugIn)) !== 0,
  };
}

function engineName(e: EngineLayout): string {
  switch (e) {
    case EngineLayout.None: return L("Kein Verbrennungsmotor", "No combustion engine");
    case EngineLayout.FrontTransverse: return L("Motor vorn quer", "Front engine, transverse");
    case EngineLayout.FrontLongitudinal: return L("Motor vorn längs", "Front engine, longitudinal");
    case EngineLayout.Mid: return L("Mittelmotor", "Mid engine");
    case EngineLayout.Rear: return L("Heckmotor", "Rear engine");
  }
}

function drivenName(d: DrivenAxles): string {
  switch (d) {
    case DrivenAxles.Front: return L("Frontantrieb", "Front-wheel drive");
    case DrivenAxles.Rear: return L("Hinterradantrieb", "Rear-wheel drive");
    case DrivenAxles.All: return L("Allradantrieb", "All-wheel drive");
  }
}

function reason(engine: EngineLayout, battery: TractionBattery): string {
  if (engine === EngineLayout.None) {
    return L(
      "Elektroauto: der Akku liegt flach im Boden zwischen den Achsen, die Motoren sitzen direkt an den angetriebenen Achsen.",
      "Electric car: the battery lies flat in the floor between the axles, the motors sit right at the driven axles.",
    );
  }
  if (battery === TractionBattery.Hybrid || battery === TractionBattery.PlugIn) {
    return L(
      "Hybrid: der Verbrenner bleibt, wo er bei diesem Modell sitzt, dazu ein Elektromotor am Getriebe und ein Akku unter der Rückbank.",
      "Hybrid: the combustion engine stays where it sits in this model, plus an electric motor at the gearbox and a battery under the rear seat.",
    );
  }
  if (engine === EngineLayout.FrontTransverse) {
    return L(
      "Motor vorn quer mit Frontantrieb: so sind die meisten Autos seit den 1980ern gebaut.",
      "Front engine across the car with front-wheel drive: the layout of most cars since the 1980s.",
    );
  }
  if (engine === EngineLayout.FrontLongitudinal) {
    return L(
      "Motor vorn längs: typisch für diese Marke oder dieses Modell, meist mit Hinterrad- oder Allradantrieb.",
      "Front engine along the car: typical for this brand or model, mostly with rear or all-wheel drive.",
    );
  }
  return L(
    "Motor hinten: bei diesem Modell sitzt er hinter dem Fahrer und treibt die Hinterräder an.",
    "Engine at the back: in this model it sits behind the driver and drives the rear wheels.",
  );
}

// The anatomy of a catalog entry (powertrains: all its PT_* flags; variant:
// the one to show, 0 for the default). Also used by the layout tests.
export function anatomyOf(
  body: BodyType,
  brand: string,
  model: string,
  year: number,
  powertrains: number,
  variant: number,
  generation = "",
  measured: MeasuredRecord | null = null,
): VehicleAnatomy {
  const variants = ORDER.filter((f) => powertrains & f);
  const chosen = variant && powertrains & variant ? variant : defaultPowertrain(powertrains);
  const t = drivetrain(brand, model, body, year, chosen);
  const typical = typicalDimensions(body, year);
  const [dims, bits] = measured ? mergeMeasured(typical, measured) : [typical, 0];
  const wikidata = !!measured && (measured[0] > 0 || measured[3] > 0);
  return {
    brand,
    model,
    generation,
    year,
    ...dims,
    measured: wikidata,
    dimensionSource: wikidata ? "wikidata" : "typical",
    measuredFields: bits,
    wikidataId: measured ? (measured[6].split("|")[0] ?? "") : "",
    massKg: measured ? measured[5] : 0,
    dimensionsText: wikidata
      ? L(
          "Maße dieses Modells aus Wikidata, die Form ist ihm nachempfunden; mit ≈ markierte Werte fehlen dort und sind geschätzt.",
          "Dimensions of this model from Wikidata, the shape is modelled after it; values marked ≈ are missing there and estimated.",
        )
      : L(
          "Typische Maße für diesen Aufbau und dieses Baujahr, nicht die genauen des Modells.",
          "Typical dimensions for this body type and year, not the exact ones of the model.",
        ),
    powertrain: chosen,
    powertrainChoices: (variants.length > 0 ? variants : [chosen]).map((flag) => ({ flag, name: powertrainText(flag) })),
    ...t,
    engineName: engineName(t.engine),
    drivenName: drivenName(t.driven),
    reason: reason(t.engine, t.battery),
  };
}

export const xrayHandlers: PartialHandlers = {
  vehicle_anatomy: async ({ generationId, year, body, brand, model, powertrain }): Promise<VehicleAnatomy> => {
    const g = generationId >= 0 ? await catalogGeneration(generationId) : null;
    if (g) return anatomyOf(g.body as BodyType, g.brand, g.model, year > 0 ? year : g.yearFrom, g.powertrains, powertrain, g.generation, g.measured);
    const variant = defaultPowertrain(powertrain);
    return anatomyOf(body as BodyType, brand, model, year, variant, variant);
  },
};
