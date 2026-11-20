// The names the X-ray view shows (layers, buses, parts, zones) and the
// tables that map control unit addresses to their kind.
import { CarZone } from "../../api";
import { t, translator } from "../../i18n";
import { EcuKind, type BusKind, type Layer } from "../../xray/layout";
import type { HeavyKind } from "../../xray/heavy";
import type { MotoStyle } from "../../xray/moto";

const tr = translator("caruspex::XrayPage");
const VD = "caruspex::VehicleDiagram";

// The live values that get a label, and where.
export const LIVE_KEYS: { pid: number; anchor: string; decimals: number }[] = [
  { pid: 0x05, anchor: "cooling", decimals: 0 },
  { pid: 0x0c, anchor: "engine", decimals: 0 },
  { pid: 0x42, anchor: "battery", decimals: 1 },
  { pid: 0x0d, anchor: "wheel", decimals: 0 },
  { pid: 0x2f, anchor: "fuel", decimals: 0 },
  { pid: 0x0f, anchor: "intake", decimals: 0 },
  { pid: 0x5b, anchor: "hv", decimals: 0 },
];

// VAG K-line addresses and the kind of unit behind them.
export const KKL_KINDS: Record<number, number> = {
  0x01: EcuKind.Engine, 0x02: EcuKind.Gearbox, 0x03: EcuKind.Brakes, 0x08: EcuKind.Climate,
  0x09: EcuKind.Body, 0x15: EcuKind.Airbag, 0x16: EcuKind.Steering, 0x17: EcuKind.Instruments,
  0x19: EcuKind.Gateway, 0x44: EcuKind.Steering, 0x46: EcuKind.Body, 0x56: EcuKind.Infotainment,
  0x76: EcuKind.Parking, 0x36: EcuKind.Seats, 0x42: EcuKind.Doors, 0x52: EcuKind.Doors,
};

export const udsKey = (request: number) => `uds-${request.toString(16)}`;

export function layerName(layer: Layer, energy: string, moto: boolean): string {
  switch (layer) {
    case "body": return moto ? tr("Rahmen") : tr("Karosserie");
    case "drivetrain": return tr("Antrieb");
    case "cooling": return t(VD, "Kühlung");
    case "energy": return energy;
    case "electrics": return tr("Elektrik");
    case "network": return tr("Netz");
  }
}

export function busName(bus: BusKind): string {
  switch (bus) {
    case "powertrain": return tr("Antriebs-CAN");
    case "comfort": return tr("Komfort-CAN");
    case "diagnostic": return tr("Diagnose-CAN");
    case "kline": return tr("K-Leitung");
    case "infotainment": return tr("Infotainment-CAN");
    case "mscan": return tr("MS-CAN (Ford)");
    case "swcan": return tr("SW-CAN (GM)");
  }
}

export function heavyKindName(kind: HeavyKind): string {
  switch (kind) {
    case "tractor": return tr("Sattelzugmaschine");
    case "rigid": return tr("Lkw mit Kofferaufbau");
    case "citybus": return tr("Stadtbus");
    case "coach": return tr("Reisebus");
  }
}

// SAE J1939 source addresses and the kind of unit behind them (J1939-71
// preferred addresses); the tachograph (0xEE) goes by its name.
export const J1939_KINDS: Record<number, number> = {
  0x00: EcuKind.Engine,
  0x01: EcuKind.Engine,
  0x03: EcuKind.Gearbox,
  0x04: EcuKind.Gearbox,
  0x0b: EcuKind.Brakes,
  0x17: EcuKind.Instruments,
  0x19: EcuKind.Climate,
  0x21: EcuKind.Body,
  0x31: EcuKind.Body,
};

export function motoStyleName(style: MotoStyle): string {
  switch (style) {
    case "naked": return tr("Naked Bike");
    case "sport": return tr("Sportler mit Verkleidung");
    case "touring": return tr("Tourer");
    case "enduro": return tr("Reiseenduro");
    case "cruiser": return tr("Cruiser");
    case "scooter": return tr("Roller");
  }
}

// What a part of the model is called (the zone names of the side view
// where they fit).
export function partName(id: string): string {
  const is = (...prefixes: string[]) => prefixes.some((p) => id === p || id.startsWith(`${p}-`));
  // The harness and the socket.
  if (is("main-fuses")) return tr("Hauptsicherungen");
  if (is("ground", "ground-point")) return tr("Masse");
  if (is("starter", "starter-cable")) return tr("Anlasser");
  if (is("engine-harness")) return tr("Motorkabelbaum");
  if (is("harness", "harness-front", "harness-rear", "obd-feed")) return tr("Kabelbaum");
  if (is("obd-adapter")) return tr("OBD-Adapter");
  if (is("obd-cable")) return tr("Adapterkabel");
  // The motorcycle.
  if (is("frame", "steering-head", "subframe", "underrun-guard", "fifth-wheel-mount")) return tr("Rahmen");
  // Trucks and buses, off-roaders.
  if (is("fifth-wheel")) return tr("Sattelkupplung");
  if (is("adblue-tank", "adblue-line")) return tr("AdBlue-Tank");
  if (is("spare-wheel")) return tr("Reserverad");
  if (is("front-axle", "rear-axle")) return tr("Achse");
  if (is("fork", "fork-slider", "clamp")) return tr("Gabel");
  if (is("swingarm")) return tr("Schwinge");
  if (is("shock")) return tr("Federbein");
  if (id === "seat" || id === "pillion") return tr("Sitzbank");
  if (is("handlebar", "grip")) return tr("Lenker");
  if (is("headlight")) return tr("Scheinwerfer");
  if (is("tail-light")) return tr("Rücklicht");
  if (is("dash")) return tr("Instrumente");
  if (is("chain", "belt", "shaft", "final-drive", "sprocket")) return tr("Endantrieb");
  if (is("silencer")) return t(VD, "Abgas");
  if (is("footrest")) return tr("Fußrasten");
  if (is("engine", "cylinder")) return t(VD, "Motor");
  if (is("coil")) return t(VD, "Zündung");
  if (is("intake", "airbox")) return t(VD, "Ansaugung");
  if (is("fuel-rail")) return t(VD, "Einspritzung");
  if (is("throttle")) return t(VD, "Drosselklappe");
  if (is("gearbox", "front-diff", "rear-diff", "shaft", "prop")) return t(VD, "Getriebe");
  if (is("catalyst")) return tr("Katalysator");
  if (is("exhaust", "middle-silencer", "rear-silencer")) return t(VD, "Abgas");
  if (is("turbo")) return t(VD, "Turbo");
  if (is("radiator", "fan", "hose", "expansion")) return t(VD, "Kühlung");
  if (is("fuel-tank", "filler", "fuel-line")) return t(VD, "Tank");
  if (is("hv-battery")) return tr("Hochvoltakku");
  if (is("hv-cable", "inverter")) return tr("Hochvolt");
  if (is("motor")) return tr("Elektromotor");
  if (is("battery12", "alternator", "fuse-box", "harness")) return t(VD, "Bordnetz");
  if (id === "obd") return tr("OBD-Buchse");
  return t(VD, "Innenraum");
}

export function zoneLabel(zone: CarZone): string {
  switch (zone) {
    case CarZone.Intake: return t(VD, "Ansaugung");
    case CarZone.Throttle: return t(VD, "Drosselklappe");
    case CarZone.Injection: return t(VD, "Einspritzung");
    case CarZone.Ignition: return t(VD, "Zündung");
    case CarZone.Cylinders: return t(VD, "Motor");
    case CarZone.Cooling: return t(VD, "Kühlung");
    case CarZone.Turbo: return t(VD, "Turbo");
    case CarZone.Exhaust: return t(VD, "Abgas");
    case CarZone.FuelTank: return t(VD, "Tank");
    case CarZone.Gearbox: return t(VD, "Getriebe");
    case CarZone.Ecu: return t(VD, "Steuergerät");
    case CarZone.Electrics: return t(VD, "Bordnetz");
    case CarZone.Network: return t(VD, "Datenbus");
    case CarZone.Wheels: return t(VD, "Räder und ABS");
    case CarZone.Cabin: return t(VD, "Innenraum");
  }
  return "";
}

// "Motor (7E8)" -> 0x7E8; 0 when the text has no address.
export function responseOf(ecu: string): number {
  const m = ecu.match(/\(([0-9A-Fa-f]{3,8})\)/);
  return m ? Number.parseInt(m[1]!, 16) : 0;
}
