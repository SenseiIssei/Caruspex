// Demo answers for the workshop pages: all control units over UDS, the
// adapter test, and the K-line services (scan, basic settings, adaptation,
// output test). The UDS units, their identification and fault memory are the
// ones of DemoTransport.cpp (makeUdsUnits); the address lists are those of
// data/EcuAddresses. The demo car has no K-line, so every KKL command answers
// the way the Worker does when the port does not open, unless the page was
// opened with ?kkl: then a simulated cable sits on COM7 with an older VAG
// behind it (?kkl=fsi gives a 1.6 FSI engine that speaks KWP2000).
import { t } from "../../../i18n";
import type {
  AdapterCheckRow,
  AdapterRow,
  AppSettings,
  BrandFamily,
  EcuScanRow,
  FaultRow,
  KklEcuInfo,
  LiveRow,
  UdsDtcRow,
  UdsEcuRow,
} from "../../types";
import { DEFAULT_SETTINGS } from "../../types";
import { ADAPTER_DESCRIPTION, ADAPTER_VERSION, DEMO_VIN, simulate } from "../car";
import { L } from "../core";
import { describeDtc } from "../dtc";
import { appSeconds, car, emit, link, log, subscribe, wait, type PartialHandlers } from "../runtime";
import { sessionHandlers } from "../session";

const W = "caruspex::Worker";

const hex = (value: number, digits: number) => value.toString(16).toUpperCase().padStart(digits, "0");

// The settings the Worker would hold: the demo keeps them in localStorage
// (mock/app.ts), so read them from there.
function settings(): AppSettings {
  try {
    const text = localStorage.getItem("caruspex.settings");
    return { ...DEFAULT_SETTINGS, ...(text ? (JSON.parse(text) as Partial<AppSettings>) : {}) };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

// ---------------------------------------------------------------------------
// Address lists (data/EcuAddresses.gen.cpp)
// ---------------------------------------------------------------------------

// EcuKind of data/EcuAddresses.h, in the order of the enum.
const enum Kind {
  Engine, Gearbox, Brakes, Airbag, Steering, Instruments, Gateway, Body, Climate,
  Parking, Lighting, Doors, Seats, Infotainment, Battery, Charger, Hybrid, Other,
}

function kindName(kind: Kind): string {
  switch (kind) {
    case Kind.Engine: return L("Motor", "Engine");
    case Kind.Gearbox: return L("Getriebe", "Gearbox");
    case Kind.Brakes: return L("Bremsen und ABS", "Brakes and ABS");
    case Kind.Airbag: return L("Airbag", "Airbag");
    case Kind.Steering: return L("Lenkung", "Steering");
    case Kind.Instruments: return L("Kombiinstrument", "Instrument cluster");
    case Kind.Gateway: return L("Gateway", "Gateway");
    case Kind.Body: return L("Zentralelektronik", "Body electronics");
    case Kind.Climate: return L("Klimaanlage", "Climate control");
    case Kind.Parking: return L("Einparkhilfe", "Parking aid");
    case Kind.Lighting: return L("Licht", "Lighting");
    case Kind.Doors: return L("Türen", "Doors");
    case Kind.Seats: return L("Sitze", "Seats");
    case Kind.Infotainment: return L("Radio und Navigation", "Radio and navigation");
    case Kind.Battery: return L("Batterie", "Battery");
    case Kind.Charger: return L("Ladegerät", "Charger");
    case Kind.Hybrid: return L("Hybrid- und Elektroantrieb", "Hybrid and electric drive");
    default: return L("Sonstiges", "Other");
  }
}

// family, request, response, 29 bit, BMW target byte (-1 none), kind, name de, name en
type EcuTableRow = [string, number, number, 0 | 1, number, Kind, string, string];

const ECU_TABLE: EcuTableRow[] = [
  ["VAG", 0x7E0, 0x7E8, 0, -1, 0, "Motorsteuerung", "Engine control"],
  ["VAG", 0x7E1, 0x7E9, 0, -1, 1, "Getriebesteuerung", "Gearbox control"],
  ["VAG", 0x713, 0x77D, 0, -1, 2, "Bremsen und ABS", "Brakes and ABS"],
  ["VAG", 0x715, 0x77F, 0, -1, 3, "Airbag", "Airbag"],
  ["VAG", 0x712, 0x77C, 0, -1, 4, "Lenkhilfe", "Power steering"],
  ["VAG", 0x714, 0x77E, 0, -1, 5, "Kombiinstrument", "Instrument cluster"],
  ["VAG", 0x710, 0x77A, 0, -1, 6, "Gateway", "Gateway"],
  ["VAG", 0x70E, 0x778, 0, -1, 7, "Zentralelektronik", "Central electronics"],
  ["VAG", 0x70C, 0x776, 0, -1, 7, "Lenksäulenelektronik", "Steering column electronics"],
  ["VAG", 0x746, 0x7B0, 0, -1, 8, "Klimaanlage", "Climate control"],
  ["VAG", 0x70A, 0x774, 0, -1, 9, "Einparkhilfe", "Parking aid"],
  ["BMW", 0x6F1, 0x612, 0, 18, 0, "Motorsteuerung (DME)", "Engine control (DME)"],
  ["BMW", 0x6F1, 0x618, 0, 24, 1, "Automatikgetriebe (EGS)", "Automatic gearbox (EGS)"],
  ["BMW", 0x6F1, 0x629, 0, 41, 2, "Fahrdynamik und ABS (DSC)", "Stability control and ABS (DSC)"],
  ["BMW", 0x6F1, 0x601, 0, 1, 3, "Airbag (ACSM)", "Airbag (ACSM)"],
  ["BMW", 0x6F1, 0x630, 0, 48, 4, "Elektrische Lenkung (EPS)", "Electric power steering (EPS)"],
  ["BMW", 0x6F1, 0x610, 0, 16, 6, "Gateway (FEM/ZGM)", "Gateway (FEM/ZGM)"],
  ["BMW", 0x6F1, 0x640, 0, 64, 7, "Karosserieelektronik (FEM/BDC)", "Body electronics (FEM/BDC)"],
  ["BMW", 0x6F1, 0x660, 0, 96, 5, "Kombiinstrument (KOMBI)", "Instrument cluster (KOMBI)"],
  ["BMW", 0x6F1, 0x678, 0, 120, 8, "Klimaautomatik (IHKA)", "Climate control (IHKA)"],
  ["BMW", 0x6F1, 0x663, 0, 99, 13, "Head Unit (NBT/CIC)", "Head unit (NBT/CIC)"],
  ["BMW", 0x6F1, 0x667, 0, 103, 13, "iDrive-Controller (ZBE)", "iDrive controller (ZBE)"],
  ["BMW", 0x6F1, 0x661, 0, 97, 13, "Telematik (ATM)", "Telematics (ATM)"],
  ["BMW", 0x6F1, 0x637, 0, 55, 13, "Verstärker (AMP)", "Amplifier (AMP)"],
  ["BMW", 0x6F1, 0x63F, 0, 63, 13, "Active Sound Design (ASD)", "Active Sound Design (ASD)"],
  ["BMW", 0x6F1, 0x656, 0, 86, 7, "Dachfunktionszentrum (FZD)", "Roof function centre (FZD)"],
  ["BMW", 0x6F1, 0x672, 0, 114, 7, "Heckelektronik (REM)", "Rear electronics (REM)"],
  ["BMW", 0x6F1, 0x617, 0, 23, 17, "Kraftstoffpumpe (EKPM)", "Fuel pump control (EKPM)"],
  ["BMW", 0x6F1, 0x61C, 0, 28, 17, "Fahrwerksregelung (ICM)", "Integrated chassis management (ICM)"],
  ["BMW", 0x6F1, 0x676, 0, 118, 17, "Dämpferregelung (VDC)", "Damper control (VDC)"],
  ["BMW", 0x6F1, 0x60F, 0, 15, 17, "Sperrdifferenzial hinten (GHAS)", "Rear differential lock (GHAS)"],
  ["BMW", 0x6F1, 0x65D, 0, 93, 17, "Kamera-Fahrerassistenz (KAFAS)", "Camera driver assistance (KAFAS)"],
  ["BMW", 0x6F1, 0x606, 0, 6, 9, "Rückfahr- und Umfeldkamera (TRSVC)", "Reversing and surround camera (TRSVC)"],
  ["BMW", 0x6F1, 0x641, 0, 65, 10, "Scheinwerfer-Treiber (TMS)", "Headlight driver (TMS)"],
  ["BMW", 0x6F1, 0x642, 0, 66, 10, "Scheinwerfer-Treiber 2 (TMS)", "Headlight driver 2 (TMS)"],
  ["BMW", 0x6F1, 0x643, 0, 67, 10, "Lichtmodul (LHM)", "Light module (LHM)"],
  ["BMW", 0x6F1, 0x644, 0, 68, 10, "Lichtmodul 2 (LHM)", "Light module 2 (LHM)"],
  ["BMW", 0x6F1, 0x66D, 0, 109, 12, "Sitzmodul Fahrer (SM)", "Seat module driver (SM)"],
  ["BMW", 0x6F1, 0x66E, 0, 110, 12, "Sitzmodul Beifahrer (SM)", "Seat module passenger (SM)"],
  ["FORD", 0x7E0, 0x7E8, 0, -1, 0, "Motorsteuerung (PCM)", "Powertrain control (PCM)"],
  ["FORD", 0x7E1, 0x7E9, 0, -1, 1, "Getriebesteuerung (TCM)", "Transmission control (TCM)"],
  ["FORD", 0x760, 0x768, 0, -1, 2, "Bremsen und ABS", "Brakes and ABS"],
  ["FORD", 0x737, 0x73F, 0, -1, 3, "Airbag (RCM)", "Airbag (RCM)"],
  ["FORD", 0x730, 0x738, 0, -1, 4, "Lenkhilfe (PSCM)", "Power steering (PSCM)"],
  ["FORD", 0x720, 0x728, 0, -1, 5, "Kombiinstrument (IPC)", "Instrument cluster (IPC)"],
  ["FORD", 0x716, 0x71E, 0, -1, 6, "Gateway (GWM)", "Gateway (GWM)"],
  ["FORD", 0x726, 0x72E, 0, -1, 7, "Bordnetzsteuergerät (BCM)", "Body control (BCM)"],
  ["FORD", 0x724, 0x72C, 0, -1, 7, "Lenksäulenschalter (SCCM)", "Steering column (SCCM)"],
  ["FORD", 0x733, 0x73B, 0, -1, 8, "Klimaanlage (HVAC)", "Climate control (HVAC)"],
  ["FORD", 0x736, 0x73E, 0, -1, 9, "Einparkhilfe (PAM)", "Parking aid (PAM)"],
  ["FORD", 0x740, 0x748, 0, -1, 11, "Tür Fahrer (DDM)", "Driver door (DDM)"],
  ["FORD", 0x741, 0x749, 0, -1, 11, "Tür Beifahrer (PDM)", "Passenger door (PDM)"],
  ["FORD", 0x727, 0x72F, 0, -1, 13, "Radio (ACM)", "Audio (ACM)"],
  ["FORD", 0x7D0, 0x7D8, 0, -1, 13, "SYNC (APIM)", "SYNC (APIM)"],
  ["MAZDA", 0x7E0, 0x7E8, 0, -1, 0, "Motorsteuerung (PCM)", "Powertrain control (PCM)"],
  ["MAZDA", 0x7E1, 0x7E9, 0, -1, 1, "Getriebesteuerung (TCM)", "Transmission control (TCM)"],
  ["MAZDA", 0x760, 0x768, 0, -1, 2, "Bremsen und ABS", "Brakes and ABS"],
  ["MAZDA", 0x737, 0x73F, 0, -1, 3, "Airbag (RCM)", "Airbag (RCM)"],
  ["MAZDA", 0x730, 0x738, 0, -1, 4, "Lenkhilfe (EPS)", "Power steering (EPS)"],
  ["MAZDA", 0x720, 0x728, 0, -1, 5, "Kombiinstrument", "Instrument cluster"],
  ["GM", 0x7E0, 0x7E8, 0, -1, 0, "Motorsteuerung (ECM)", "Engine control (ECM)"],
  ["GM", 0x7E2, 0x7EA, 0, -1, 1, "Getriebesteuerung (TCM)", "Transmission control (TCM)"],
  ["GM", 0x241, 0x641, 0, -1, 7, "Bordnetzsteuergerät (BCM)", "Body control (BCM)"],
  ["GM", 0x7E4, 0x7EC, 0, -1, 14, "Hochvoltbatterie (BECM)", "High voltage battery (BECM)"],
  ["TOYOTA", 0x7E0, 0x7E8, 0, -1, 0, "Motorsteuerung", "Engine control"],
  ["TOYOTA", 0x7E1, 0x7E9, 0, -1, 1, "Getriebesteuerung", "Transmission control"],
  ["TOYOTA", 0x7B0, 0x7B8, 0, -1, 2, "Bremsen und ABS", "Brakes and ABS"],
  ["TOYOTA", 0x780, 0x788, 0, -1, 3, "Airbag", "Airbag"],
  ["TOYOTA", 0x7A1, 0x7A9, 0, -1, 4, "Lenkhilfe (EPS)", "Power steering (EPS)"],
  ["TOYOTA", 0x7C0, 0x7C8, 0, -1, 5, "Kombiinstrument", "Instrument cluster"],
  ["TOYOTA", 0x7C4, 0x7CC, 0, -1, 8, "Klimaanlage", "Climate control"],
  ["TOYOTA", 0x7E2, 0x7EA, 0, -1, 16, "Hybridsteuerung", "Hybrid control"],
  ["TOYOTA", 0x7E3, 0x7EB, 0, -1, 14, "Hybridbatterie", "Hybrid battery"],
  ["NISSAN", 0x7E0, 0x7E8, 0, -1, 0, "Motorsteuerung", "Engine control"],
  ["NISSAN", 0x7E1, 0x7E9, 0, -1, 1, "Getriebesteuerung", "Transmission control"],
  ["NISSAN", 0x740, 0x760, 0, -1, 2, "Bremsen und ABS", "Brakes and ABS"],
  ["NISSAN", 0x743, 0x763, 0, -1, 5, "Kombiinstrument", "Instrument cluster"],
  ["NISSAN", 0x745, 0x765, 0, -1, 7, "Bordnetzsteuergerät (BCM)", "Body control (BCM)"],
  ["NISSAN", 0x79B, 0x7BB, 0, -1, 14, "Batteriesteuerung (LBC)", "Battery controller (LBC)"],
  ["NISSAN", 0x797, 0x79A, 0, -1, 16, "Fahrzeugsteuerung (VCM)", "Vehicle control (VCM)"],
  ["HYUNDAI", 0x7E0, 0x7E8, 0, -1, 0, "Motorsteuerung", "Engine control"],
  ["HYUNDAI", 0x7E1, 0x7E9, 0, -1, 1, "Getriebesteuerung", "Transmission control"],
  ["HYUNDAI", 0x7D1, 0x7D9, 0, -1, 2, "Bremsen und ESC", "Brakes and ESC"],
  ["HYUNDAI", 0x7D2, 0x7DA, 0, -1, 3, "Airbag", "Airbag"],
  ["HYUNDAI", 0x7D4, 0x7DC, 0, -1, 4, "Lenkhilfe (MDPS)", "Power steering (MDPS)"],
  ["HYUNDAI", 0x7C6, 0x7CE, 0, -1, 5, "Kombiinstrument", "Instrument cluster"],
  ["HYUNDAI", 0x7A0, 0x7A8, 0, -1, 7, "Bordnetzsteuergerät (BCM)", "Body control (BCM)"],
  ["HYUNDAI", 0x7B3, 0x7BB, 0, -1, 8, "Klimaanlage", "Climate control"],
  ["HYUNDAI", 0x7E2, 0x7EA, 0, -1, 16, "Fahrzeugsteuerung (VCU)", "Vehicle control (VCU)"],
  ["HYUNDAI", 0x7E4, 0x7EC, 0, -1, 14, "Batteriemanagement (BMS)", "Battery management (BMS)"],
  ["RENAULT", 0x7E0, 0x7E8, 0, -1, 0, "Motorsteuerung", "Engine control"],
  ["RENAULT", 0x7E1, 0x7E9, 0, -1, 1, "Getriebesteuerung", "Transmission control"],
  ["RENAULT", 0x740, 0x760, 0, -1, 2, "Bremsen und ABS", "Brakes and ABS"],
  ["RENAULT", 0x752, 0x772, 0, -1, 3, "Airbag", "Airbag"],
  ["RENAULT", 0x742, 0x762, 0, -1, 4, "Lenkhilfe", "Power steering"],
  ["RENAULT", 0x743, 0x763, 0, -1, 5, "Kombiinstrument", "Instrument cluster"],
  ["RENAULT", 0x745, 0x765, 0, -1, 7, "Zentralelektronik (UCH)", "Central electronics (UCH)"],
  ["RENAULT", 0x744, 0x764, 0, -1, 8, "Klimaanlage", "Climate control"],
  ["RENAULT", 0x7E4, 0x7EC, 0, -1, 16, "Fahrzeugsteuerung (EVC)", "Vehicle control (EVC)"],
  ["RENAULT", 0x79B, 0x7BB, 0, -1, 14, "Batteriesteuerung (LBC)", "Battery controller (LBC)"],
  ["RENAULT", 0x792, 0x793, 0, -1, 15, "Ladegerät", "Charger"],
  ["PSA", 0x7E0, 0x7E8, 0, -1, 0, "Motorsteuerung", "Engine control"],
  ["PSA", 0x6AD, 0x68D, 0, -1, 2, "Bremsen und ESP", "Brakes and ESP"],
  ["PSA", 0x772, 0x672, 0, -1, 3, "Airbag", "Airbag"],
  ["PSA", 0x752, 0x652, 0, -1, 7, "Bordnetzrechner (BSI)", "Body computer (BSI)"],
];

// The labels of the brand families (brandFamilyLabel in DtcDatabase.cpp).
const FAMILY_LABELS: Record<string, [string, string]> = {
  VAG: ["VAG (VW, Audi, Seat, Cupra, Škoda)", "VAG (VW, Audi, Seat, Cupra, Škoda)"],
  BMW: ["BMW/Mini", "BMW/Mini"],
  FORD: ["Ford", "Ford"],
  MAZDA: ["Mazda", "Mazda"],
  GM: ["GM (Chevrolet, Buick, Cadillac, GMC)", "GM (Chevrolet, Buick, Cadillac, GMC)"],
  TOYOTA: ["Toyota/Lexus", "Toyota/Lexus"],
  NISSAN: ["Nissan/Infiniti", "Nissan/Infiniti"],
  HYUNDAI: ["Hyundai/Kia", "Hyundai/Kia"],
  RENAULT: ["Renault/Dacia", "Renault/Dacia"],
  PSA: ["PSA/Stellantis (Peugeot, Citroën, DS)", "PSA/Stellantis (Peugeot, Citroën, DS)"],
};

interface EcuDefinition {
  request: number;
  response: number;
  extended29: boolean;
  extendedAddress: number;
  kind: Kind;
  name: string;
}

function fromRow(r: EcuTableRow): EcuDefinition {
  return { request: r[1], response: r[2], extended29: r[3] === 1, extendedAddress: r[4], kind: r[5], name: L(r[6], r[7]) };
}

// Sorted by kind like ecuDefinitions(); the sort is stable, so two units of
// the same kind keep their order.
function ecuDefinitions(family: string): EcuDefinition[] {
  return ECU_TABLE.filter((r) => r[0] === family)
    .map(fromRow)
    .sort((a, b) => a.kind - b.kind);
}

function ecuFamilies(): string[] {
  const out: string[] = [];
  for (const r of ECU_TABLE) if (!out.includes(r[0])) out.push(r[0]);
  return out;
}

function sweepEntry(request: number, kind: Kind): EcuDefinition {
  const id = `0x${hex(request, 3)}`;
  return {
    request,
    response: request + 8,
    extended29: false,
    extendedAddress: -1,
    kind,
    name: kind === Kind.Other ? L("Steuergerät ", "Control unit ") + id : `${kindName(kind)} (${id})`,
  };
}

function genericEcuSweep(): EcuDefinition[] {
  const out: EcuDefinition[] = [];
  for (let id = 0x7e0; id <= 0x7e7; id++) {
    out.push(sweepEntry(id, id === 0x7e0 ? Kind.Engine : id === 0x7e1 ? Kind.Gearbox : Kind.Other));
  }
  // 7E0 to 7EF and the broadcast address 7DF are left out, as in the C++.
  for (let id = 0x700; id <= 0x7f7; id++) {
    if (id >= 0x7e0 && id <= 0x7ef) continue;
    if (id === 0x7df) continue;
    out.push(sweepEntry(id, Kind.Other));
  }
  return out;
}

function bmwAddressSweep(): EcuDefinition[] {
  const known = ecuDefinitions("BMW");
  const out = [...known];
  for (let address = 0x00; address <= 0x7f; address++) {
    if (known.some((d) => d.extendedAddress === address)) continue;
    out.push({
      request: 0x6f1,
      response: 0x600 + address,
      extended29: false,
      extendedAddress: address,
      kind: Kind.Other,
      name: L("Steuergerät ", "Control unit ") + `0x${hex(address, 2)}`,
    });
  }
  return out;
}

// ---------------------------------------------------------------------------
// The UDS units of the demo car (makeUdsUnits in DemoTransport.cpp)
// ---------------------------------------------------------------------------

interface UdsFault {
  code: string;
  failureType: number;
  status: number;
}

interface UdsUnit {
  request: number;
  response: number;
  dids: Record<number, string>;
  faults: UdsFault[];
  // Answers 19 02 with 7F 19 78 ("response pending") first.
  slowFaultRead: boolean;
}

const UDS_ACTIVE = 0xaf; // failing now, pending, confirmed, warning lamp on
const UDS_PENDING = 0x24; // pending, failed since the last clear
const UDS_STORED = 0x28; // confirmed, not failing right now
const UDS_ENGINE_STORED = 0xa8; // confirmed with the MIL on

function makeUdsUnits(): UdsUnit[] {
  return [
    {
      request: 0x7e0, response: 0x7e8, slowFaultRead: false, faults: [],
      dids: { 0xf187: "CX5 906 018 B", 0xf189: "0417", 0xf191: "H05", 0xf197: "ECM 1.4 MPI   ", 0xf18c: "EM21A0048217", 0xf190: DEMO_VIN },
    },
    {
      request: 0x7e1, response: 0x7e9, slowFaultRead: false, faults: [],
      dids: { 0xf187: "CX6 927 156 A", 0xf189: "0203", 0xf191: "H03", 0xf197: "TCM AT6", 0xf18c: "TA21B0009144" },
    },
    // ABS: the left front wheel speed sensor reads a voltage out of range.
    {
      request: 0x760, response: 0x768, slowFaultRead: true,
      faults: [{ code: "C0035", failureType: 0x1c, status: UDS_ACTIVE }],
      dids: { 0xf187: "CX1 614 517 C", 0xf189: "0122", 0xf191: "H11", 0xf197: "ABS/ESP 9.3", 0xf18c: "AB21C0031877", 0xf190: DEMO_VIN },
    },
    // Airbag: resistance of the driver airbag squib too low, seen once.
    {
      request: 0x737, response: 0x73f, slowFaultRead: false,
      faults: [{ code: "B0001", failureType: 0x1a, status: UDS_PENDING }],
      dids: { 0xf187: "CX0 959 655 F", 0xf189: "0310", 0xf191: "H07", 0xf197: "AIRBAG 8CH", 0xf18c: "SR21D0007730", 0xf190: DEMO_VIN },
    },
    // Instrument cluster: nothing stored.
    {
      request: 0x720, response: 0x728, slowFaultRead: false, faults: [],
      dids: { 0xf187: "CX2 920 870 D", 0xf189: "0805", 0xf191: "H22", 0xf197: "INSTRUMENTS", 0xf18c: "IC21A0102265", 0xf190: DEMO_VIN },
    },
    // Climate: lost the engine on the bus once. It keeps its versions in
    // F195 and F193 instead of F189 and F191, as some makers do.
    {
      request: 0x744, response: 0x74c, slowFaultRead: false,
      faults: [{ code: "U0100", failureType: 0x00, status: UDS_STORED }],
      dids: { 0xf187: "CX8 907 044 A", 0xf195: "0109", 0xf193: "H04", 0xf197: "CLIMATE 2Z", 0xf18c: "HV21B0055102", 0xf190: DEMO_VIN },
    },
  ];
}

// A new DemoTransport comes with every connection, and with it a fresh
// fault memory in the units.
let udsUnits = makeUdsUnits();
subscribe("connected", () => {
  udsUnits = makeUdsUnits();
});

function failureTypeText(type: number): string {
  switch (type) {
    case 0x1a: return L("Widerstand unter der Schwelle", "circuit resistance below threshold");
    case 0x1c: return L("Spannung außerhalb des Bereichs", "circuit voltage out of range");
    default: return "";
  }
}

// Only physical requests with normal addressing reach a unit, and the
// adapter's receive filter only lets its own response id through.
function unitFor(def: EcuDefinition): UdsUnit | undefined {
  if (def.extended29 || def.extendedAddress >= 0) return undefined;
  return udsUnits.find((u) => u.request === def.request && u.response === def.response);
}

// The engine's UDS fault memory is its OBD one: stored codes confirmed with
// the MIL on, pending ones with the pending bit.
function unitFaults(unit: UdsUnit): UdsFault[] {
  if (unit.request !== 0x7e0) return unit.faults;
  const out: UdsFault[] = [];
  const add = (code: string, bits: number) => {
    const hit = out.find((f) => f.code === code);
    if (hit) hit.status |= bits;
    else out.push({ code, failureType: 0, status: bits });
  };
  for (const code of car.storedCodes()) add(code, UDS_ENGINE_STORED);
  for (const code of car.pendingCodes()) add(code, UDS_PENDING);
  return out;
}

// cleanAscii in Uds.cpp: printable characters, trimmed.
const cleanAscii = (text: string | undefined) => (text ?? "").replace(/[^\x20-\x7e]/g, "").trim();

// What scanEcus() makes of one definition.
async function scanOne(def: EcuDefinition): Promise<UdsEcuRow> {
  const row: UdsEcuRow = {
    request: def.request,
    response: def.response,
    extended29: def.extended29,
    kind: def.kind,
    name: def.name,
    present: false,
    dtcsRead: false,
    partNumber: "",
    softwareVersion: "",
    hardwareVersion: "",
    systemName: "",
    serial: "",
    dtcs: [],
    note: "",
  };
  const unit = unitFor(def);
  if (!unit) {
    // Twice the shortened adapter timeout (3E, then 22 F1 87), scaled down
    // so the sweep is watchable.
    await wait(45);
    row.note = L(
      "Keine Antwort: nicht verbaut, oder das Gateway lässt die Anfrage nicht durch.",
      "No answer: not fitted, or the gateway does not pass the request on.",
    );
    return row;
  }
  await wait(380 + (unit.slowFaultRead ? 260 : 0));
  row.present = true;
  row.partNumber = cleanAscii(unit.dids[0xf187]);
  row.softwareVersion = cleanAscii(unit.dids[0xf189] ?? unit.dids[0xf195]);
  row.hardwareVersion = cleanAscii(unit.dids[0xf191] ?? unit.dids[0xf193]);
  row.systemName = cleanAscii(unit.dids[0xf197]);
  row.serial = cleanAscii(unit.dids[0xf18c]);
  row.dtcsRead = true;
  row.dtcs = unitFaults(unit).map(
    (f): UdsDtcRow => ({
      code: f.code,
      failureText: failureTypeText(f.failureType),
      text: describeDtc(f.code),
      testFailed: (f.status & 0x01) !== 0,
      pending: (f.status & 0x04) !== 0,
      confirmed: (f.status & 0x08) !== 0,
      warningLamp: (f.status & 0x80) !== 0,
    }),
  );
  return row;
}

let udsRunning = false;

async function scanUds(family: string): Promise<void> {
  if (!link.connected) {
    emit("uds_scan_finished", { present: 0, withFaults: 0, cancelled: false, note: t(W, "Erst verbinden.") });
    return;
  }
  if (udsRunning) return;
  udsRunning = true;
  // The Worker stops the live loop first: the scan needs the adapter alone.
  if (link.liveTimer) void sessionHandlers.stop_live?.({});
  link.cancel = false;
  emit("busy", { active: true });
  let list = family === "" ? genericEcuSweep() : family === "BMW*" ? bmwAddressSweep() : ecuDefinitions(family);
  const generic = family === "" || list.length === 0;
  if (list.length === 0) list = genericEcuSweep();
  const total = list.length;
  emit("uds_scan_started", { total, generic });
  let present = 0;
  let withFaults = 0;
  let index = 0;
  for (const def of list) {
    if (link.cancel || !link.connected) break;
    emit("uds_scan_progress", { name: def.name, index: index++, total, request: def.request, kind: def.kind });
    const row = await scanOne(def);
    if (!row.present) continue;
    present++;
    if (row.dtcs.length > 0) withFaults++;
    emit("uds_ecu_result", { row });
  }
  emit("uds_scan_finished", { present, withFaults, cancelled: link.cancel, note: "" });
  emit("busy", { active: false });
  udsRunning = false;
}

async function clearUdsEcu(request: number, response: number, extended29: boolean, name: string): Promise<void> {
  if (!link.connected) return;
  if (link.liveTimer) void sessionHandlers.stop_live?.({});
  emit("busy", { active: true });
  // ecuFromIds(): a 6F1 request with a 6xx response is BMW style addressing.
  const def: EcuDefinition = {
    request,
    response,
    extended29,
    extendedAddress: !extended29 && request === 0x6f1 && (response & 0xf00) === 0x600 ? response & 0xff : -1,
    kind: Kind.Other,
    name,
  };
  await wait(500);
  const unit = unitFor(def);
  let ok = false;
  let error = "";
  if (unit) {
    if (unit.request === 0x7e0) car.clear();
    else unit.faults = [];
    ok = true;
  } else {
    error = L("Keine Antwort vom Steuergerät (NO DATA).", "No answer from the control unit (NO DATA).");
  }
  emit("uds_clear_result", {
    name,
    ok,
    message: ok ? t(W, "Fehlerspeicher von %1 gelöscht.", name) : t(W, "Löschen bei %1 hat nicht geklappt: %2", name, error),
  });
  // Read that unit again so the page shows what is left.
  const again = await scanOne(def);
  if (again.present) emit("uds_ecu_result", { row: { ...again, name } });
  emit("busy", { active: false });
}

// ---------------------------------------------------------------------------
// The adapter test (core/AdapterProbe.cpp) against DemoAdapter::CloneV15:
// claims v1.5, implements the command set of v1.4, says "?" to unknown
// commands and is no STN chip.
// ---------------------------------------------------------------------------

const ADAPTER_LEVEL = 140;

const CHECKS: { command: string; level: number; de: string; en: string }[] = [
  { command: "ATPPS", level: 110, de: "Übersicht der programmierbaren Parameter (nur lesen)", en: "Summary of the programmable parameters (read only)" },
  { command: "ATKW1", level: 120, de: "Schlüsselbytes beim K-Line-Aufbau prüfen (Standardwert)", en: "Check key words on K-Line init (default value)" },
  { command: "ATS0", level: 130, de: "Leerzeichen in Antworten weglassen (schon eingestellt)", en: "Leave out spaces in responses (already set)" },
  { command: "ATD0", level: 130, de: "CAN-Datenlänge nicht anzeigen (Standardwert)", en: "Do not show the CAN data length (default)" },
  { command: "ATFE", level: 131, de: "Gemerkte Ereignisse vergessen", en: "Forget events" },
  { command: "ATCSM1", level: 140, de: "Stilles Mithören auf dem CAN-Bus (Standardwert)", en: "Silent CAN monitoring (default)" },
  { command: "ATIGN", level: 140, de: "Zündungseingang lesen", en: "Read the ignition input" },
  { command: "ATCRA", level: 142, de: "CAN-Empfangsfilter zurücksetzen (Standardwert)", en: "Reset the CAN receive filter (default)" },
  { command: "ATJHF1", level: 142, de: "J1939-Kopfzeilen formatieren (Standardwert)", en: "Format J1939 headers (default)" },
  { command: "ATAMC", level: 200, de: "Zähler der Busaktivität anzeigen", en: "Show the activity monitor count" },
];

function versionText(level: number): string {
  if (level <= 0) return "?";
  const major = Math.floor(level / 100);
  const minor = Math.floor(level / 10) % 10;
  const letter = level % 10;
  return `v${major}.${minor}${letter > 0 ? String.fromCharCode(96 + letter) : ""}`;
}

function parseVersionLevel(text: string): number {
  const m = text.match(/(?:^|[vV ])(\d)\.(\d)([a-iA-I])?/);
  if (!m) return 0;
  return Number(m[1]) * 100 + Number(m[2]) * 10 + (m[3] ? m[3].toLowerCase().charCodeAt(0) - 96 : 0);
}

// "14.2" for English, "14,2" for German, like number() in AdapterProbe.cpp.
function number(value: number, decimals: number): { de: string; en: string } {
  const en = value.toFixed(decimals);
  return { de: en.replace(".", ","), en };
}

async function probeAdapter(): Promise<void> {
  if (!link.connected) {
    emit("failed", { error: t(W, "Für den Adaptertest erst verbinden.") });
    return;
  }
  if (link.liveTimer) void sessionHandlers.stop_live?.({});
  emit("busy", { active: true });
  const facts: string[] = [];
  const fact = (de: string, en: string) => facts.push(L(de, en));
  const version = ADAPTER_VERSION;
  const claimed = parseVersionLevel(version);
  await wait(300);
  fact(`ATI meldet: ${version}`, `ATI reports: ${version}`);
  fact(`AT@1 meldet: ${ADAPTER_DESCRIPTION}`, `AT@1 reports: ${ADAPTER_DESCRIPTION}`);
  if (claimed === 150) {
    fact(
      "Die Version v1.5 hat ELM Electronics nie veröffentlicht. Sie steht fast nur auf Nachbauten.",
      "ELM Electronics never released a v1.5. It appears almost only on clones.",
    );
  }
  // ATQX answers "?" and STI is unknown: an honest clone, no STN chip.
  await wait(400);
  const voltage = Math.round((simulate(car.now()).battery - 0.05) * 10) / 10;
  const volts = number(voltage, 1);
  fact(`Spannung an der OBD-Buchse: ${volts.de} V`, `Voltage at the OBD socket: ${volts.en} V`);

  const checks: AdapterCheckRow[] = [];
  for (const spec of CHECKS) {
    await wait(70);
    checks.push({
      command: spec.command,
      version: versionText(spec.level),
      supported: spec.level <= ADAPTER_LEVEL,
      purpose: L(spec.de, spec.en),
    });
  }
  let behavesLike = 100;
  const firstMissing = CHECKS.find((c) => c.level > ADAPTER_LEVEL)?.level ?? 0;
  for (const c of CHECKS) {
    if (firstMissing !== 0 && c.level >= firstMissing) break;
    behavesLike = Math.max(behavesLike, c.level);
  }
  const missing = CHECKS.filter((c) => c.level > ADAPTER_LEVEL)
    .map((c) => `${c.command} (${versionText(c.level)})`)
    .join(", ");
  const behaves = versionText(behavesLike);
  if (missing === "") {
    fact("Alle geprüften Befehle bis v2.0 werden wie beim Original beantwortet.", "Every tested command up to v2.0 is answered like the original.");
  } else {
    fact(`Befehlssatz wie ${behaves}. Nicht wie beim Original beantwortet: ${missing}.`, `Command set like ${behaves}. Not answered like the original: ${missing}.`);
  }

  // Round trip: engine and gearbox both answer 0100, and without an answer
  // count the adapter waits a little for more (DemoTransport: 6 ms plus one
  // per frame plus 12 ms, and the serial line on top).
  await wait(600);
  const roundTrip = 21;
  const withHint = 8;
  const ms = number(roundTrip, 0);
  fact(`Antwortzeit auf 0100: im Schnitt ${ms.de} ms (5 Versuche)`, `Round trip of 0100: ${ms.en} ms on average (5 tries)`);
  const hms = number(withHint, 0);
  fact(`Mit Antwortzahl (0100 1): im Schnitt ${hms.de} ms`, `With an answer count (0100 1): ${hms.en} ms on average`);

  const speedTips: string[] = [];
  if (withHint + 2 < roundTrip) {
    speedTips.push(
      L(
        `Die Zahl der erwarteten Antworten an die Anfrage hängen (etwa 0100 1). Dann wartet der Adapter nicht auf weitere Steuergeräte: hier ${hms.de} statt ${ms.de} ms.`,
        `Append the number of expected answers to a request (such as 0100 1). The adapter then stops waiting for further control units: ${hms.en} instead of ${ms.en} ms here.`,
      ),
    );
  }
  if (behavesLike >= 120) {
    speedTips.push(
      L(
        "Die schärfere adaptive Zeitsteuerung ATAT2 ist vorhanden und verkürzt die Wartezeit weiter. Bei fehlenden Antworten zurück auf ATAT1.",
        "The more aggressive adaptive timing ATAT2 is available and shortens the waiting further. Go back to ATAT1 if answers go missing.",
      ),
    );
  }

  const verdict = L(
    `Wahrscheinlich ein Nachbau: Er meldet v1.5, eine Version, die es vom Original nie gab, und verhält sich wie ein ELM327 ${behaves}.`,
    `Probably a clone: it reports v1.5, a version the original never had, and behaves like an ELM327 ${behaves}.`,
  );
  const row: AdapterRow = {
    reachable: true,
    version,
    description: ADAPTER_DESCRIPTION,
    isStn: false,
    stnId: "",
    voltage,
    hasVoltage: true,
    roundTripMs: roundTrip,
    roundTripWithHintMs: withHint,
    behavesLike: behaves,
    acceptsUnknown: false,
    verdict,
    facts,
    speedTips,
    checks,
  };
  emit("adapter_report", { row });
  emit("busy", { active: false });
}

// ---------------------------------------------------------------------------
// K-line: a KKL cable on COM7 with an older VAG behind it (only with ?kkl)
// ---------------------------------------------------------------------------

const kklParam = typeof location !== "undefined" ? new URLSearchParams(location.search).get("kkl") : null;
const KKL_PORT = "COM7";
// ?kkl=fsi: the 1.6 FSI with Bosch MED, which answers in KWP2000.
const fsiEngine = kklParam === "fsi" || kklParam === "kwp2000";

// knownEcus() of Kwp1281.cpp in its order, with ecuName().
const KKL_ECUS: [number, string, string][] = [
  [0x01, "01 Motor", "01 Engine"],
  [0x02, "02 Getriebe", "02 Transmission"],
  [0x03, "03 Bremsenelektronik (ABS)", "03 Brake electronics (ABS)"],
  [0x08, "08 Klimaanlage", "08 Air conditioning"],
  [0x09, "09 Zentralelektrik", "09 Central electrics"],
  [0x15, "15 Airbag", "15 Airbags"],
  [0x16, "16 Lenkradelektronik", "16 Steering wheel electronics"],
  [0x17, "17 Kombiinstrument", "17 Instrument cluster"],
  [0x25, "25 Wegfahrsperre", "25 Immobilizer"],
  [0x46, "46 Komfortsystem", "46 Comfort system"],
  [0x56, "56 Radio", "56 Radio"],
];

function kklEcuName(address: number): string {
  const hit = KKL_ECUS.find((e) => e[0] === address);
  return hit ? L(hit[1], hit[2]) : L("unbekannt", "unknown");
}

interface KlineFault {
  code: number;
  de: string;
  en: string;
  sporadic: boolean;
  // Clear attempts the unit ignores before the memory is really empty.
  stubborn: number;
}

interface KlineUnit {
  kwp2000: boolean;
  identification: string[];
  faults: KlineFault[];
  // Adaptation channel: stored value and the values shown alongside.
  channels: Record<number, { value: number; shown: () => MeasuredValue[] }>;
  actuators: number[];
}

interface MeasuredValue {
  label: string;
  value: number;
  unit: string;
}

const grad = () => L("Grad", "deg");

// The engine behind the cable idles warm on the workshop floor; it is not
// the demo car of the OBD side, which drives its cycle.
const klineIdle = () => 840 + Math.round(Math.sin(appSeconds() * 1.7) * 2) * 10;
const klineTrim = () => Math.round(Math.sin(appSeconds() * 0.9) * 32) / 10;
const gradC = () => L("Grad C", "deg C");

// The car behind the cable: an A2 1.4 with Marelli injection (or the FSI),
// manual gearbox, no airbag on the K-line (it sits on the CAN of the
// dashboard), immobilizer inside the cluster.
function makeKlineCar(): Map<number, KlineUnit> {
  const units = new Map<number, KlineUnit>();
  units.set(0x01, {
    kwp2000: fsiEngine,
    identification: fsiEngine ? ["036906034BL", "1.6l R4/4V FSI    MED7.5.10", "4802"] : ["036906034AM", "MARELLI 4LV", "3245"],
    faults: [
      {
        code: 533,
        de: "Leerlaufregelung: Regelgrenze erreicht",
        en: "Idle speed control: limit reached",
        sporadic: false,
        stubborn: 1,
      },
    ],
    channels: {
      1: {
        value: 128,
        shown: () => {
          return [
            { label: L("Drehzahl", "Engine speed"), value: klineIdle(), unit: "1/min" },
            { label: L("Temperatur", "Temperature"), value: 88, unit: gradC() },
          ];
        },
      },
    },
    // Evaporative valve N80, MIL, injectors 1 to 4 (as numbers, the way the
    // Marelli names them).
    actuators: [1247, 1025, 258, 259, 260, 261],
  });
  units.set(0x03, {
    kwp2000: false,
    identification: ["8Z0614517", "ABS ITT AE 20 GI", "0003"],
    faults: [],
    channels: {},
    actuators: [],
  });
  units.set(0x08, {
    kwp2000: false,
    identification: ["8Z0820043B", "KLIMAREGELUNG", "0101"],
    faults: [],
    channels: {},
    actuators: [],
  });
  units.set(0x17, {
    kwp2000: false,
    identification: ["8Z0920900F", "KOMBI+WEGFAHRS. VDO V08", "1234"],
    faults: [],
    channels: {
      9: {
        value: 128,
        shown: () => [{ label: L("Menge", "Quantity"), value: 21.4, unit: "l" }],
      },
    },
    actuators: [],
  });
  units.set(0x46, {
    kwp2000: false,
    identification: ["8Z0959433", "KOMFORTGERAET", "0105"],
    faults: [],
    channels: {},
    actuators: [],
  });
  return units;
}

const klineCar = makeKlineCar();

const kkl = {
  open: false,
  address: 0,
  kwp2000: false,
  group: 0,
  tick: 0,
  timer: 0 as ReturnType<typeof setInterval> | 0,
  actuator: -1,
  session: 0x85,
};

function protocolName(kwp2000: boolean): string {
  return kwp2000 ? "KWP2000 (ISO 14230)" : L("KWP1281 (Blockprotokoll)", "KWP1281 (block protocol)");
}

// The serial error of the Windows build when the port is not there.
function portError(device: string): string {
  return (
    device +
    L(" lässt sich nicht öffnen (Windows-Fehler ", " cannot be opened (Windows error ") +
    "2)" +
    L(": Port existiert nicht.", ": port does not exist.")
  );
}

function portOpens(device: string): boolean {
  return kklParam !== null && device === KKL_PORT;
}

function notResponding(address: number, kwp2000Wording: boolean): string {
  return (
    L("Steuergerät ", "Control unit ") +
    kklEcuName(address) +
    L(
      " meldet sich nicht.\n" +
        "  - Zündung an?\n" +
        "  - Hat das Fahrzeug dieses Steuergerät überhaupt?\n" +
        (kwp2000Wording
          ? "  - Bei neueren Fahrzeugen läuft die Diagnose über CAN, nicht K-Line."
          : "  - Bei neueren Fahrzeugen läuft die Diagnose über CAN, nicht über K-Line."),
      " does not respond.\n" +
        "  - Ignition on?\n" +
        "  - Does the vehicle have this control unit at all?\n" +
        (kwp2000Wording ? "  - Newer vehicles are diagnosed over CAN, not K-Line." : "  - Newer vehicles are diagnosed over CAN, not over K-Line."),
    )
  );
}

function stopBasicSetting(): void {
  if (!kkl.timer) return;
  clearInterval(kkl.timer);
  kkl.timer = 0;
  emit("kkl_message", { message: t(W, "Grundeinstellung beendet."), isError: false });
  log(t(W, "Grundeinstellung beendet"));
}

function closeKkl(): void {
  stopBasicSetting();
  kkl.open = false;
  kkl.actuator = -1;
  emit("kkl_connected", { connected: false, identification: "" });
}

async function openKkl(device: string, address: number): Promise<void> {
  closeKkl();
  emit("busy", { active: true });
  emit("kkl_message", { message: t(W, "Wecke das Steuergerät mit der 5-Baud-Adresse, das dauert etwa drei Sekunden ..."), isError: false });
  if (!portOpens(device)) {
    await wait(150);
    emit("busy", { active: false });
    emit("kkl_message", { message: portError(device), isError: true });
    return;
  }
  kkl.session = settings().basicSettingSession;
  await wait(2600);
  const unit = klineCar.get(address);
  if (!unit) {
    // The 5 baud address goes out, nobody answers with the sync byte.
    emit("busy", { active: false });
    emit("kkl_connected", { connected: false, identification: "" });
    emit("kkl_message", { message: notResponding(address, true), isError: true });
    return;
  }
  log(L("   Schlüsselbytes ", "   Key bytes ") + (unit.kwp2000 ? "EF 8F" : "01 8A"));
  kkl.open = true;
  kkl.address = address;
  kkl.kwp2000 = unit.kwp2000;
  kkl.actuator = -1;
  const identification = unit.identification.map((line) => line.trim());
  const name = protocolName(unit.kwp2000);
  emit("busy", { active: false });
  emit("kkl_connected", { connected: true, identification: identification.join("  |  ") });
  emit("kkl_protocol", { protocolName: name, isKwp2000: unit.kwp2000 });
  emit("kkl_message", {
    message: t(W, "Verbunden mit %1 über %2. Vor dem Start die Punkte oben prüfen.", kklEcuName(address), name),
    isError: false,
  });
  log(t(W, "KWP1281 verbunden: %1", identification.join(" / ")));
}

// Throttle positions while the control unit learns its end stops: closed,
// lower stop, wide open, back, emergency running position, settle. About
// six requests per step.
const STEP_ANGLES = [3.9, 2.0, 2.0, 2.0, 87.5, 87.5, 3.1, 3.1, 4.3, 3.9];
const TICKS_PER_STEP = 6;

function learningAngle(tick: number): { angle: number; step: number } {
  const step = Math.min(STEP_ANGLES.length - 1, Math.floor(tick / TICKS_PER_STEP));
  const previous = STEP_ANGLES[Math.max(0, step - 1)]!;
  const target = STEP_ANGLES[step]!;
  const fraction = Math.min(1, ((tick % TICKS_PER_STEP) + 1) / (TICKS_PER_STEP / 2));
  return { angle: step === STEP_ANGLES.length - 1 && tick >= STEP_ANGLES.length * TICKS_PER_STEP ? target : previous + (target - previous) * fraction, step };
}

// One basic setting request: the values, or the error the core reports.
function basicSettingValues(group: number, tick: number): MeasuredValue[] | string {
  const idle = klineIdle();
  if (kkl.address !== 0x01) {
    return kkl.kwp2000
      ? L("Angefragter Wert liegt außerhalb des Bereichs", "Requested value is out of range")
      : L("Grundeinstellung ", "Basic settings ") + group + L(" kennt dieses Steuergerät nicht", " are not supported by this control unit");
  }
  if (kkl.kwp2000) {
    // KWP2000 opens a session instead; which type the MED accepts is the
    // guesswork the page warns about. 0x89 only reads.
    if (kkl.session === 0x86 || kkl.session === 0x87) {
      return (
        L("Sitzungsart 0x", "Session type 0x") +
        hex(kkl.session, 2) +
        L(" abgelehnt: ", " rejected: ") +
        L("Unterfunktion wird nicht unterstützt", "Sub-function not supported")
      );
    }
    const learning = kkl.session !== 0x89;
    if (group === 98) {
      const { angle, step } = learningAngle(learning ? tick : 0);
      return [
        { label: L("Spannung", "Voltage"), value: Math.round((0.5 + angle * 0.04) * 1000) / 1000, unit: "V" },
        { label: L("Spannung", "Voltage"), value: Math.round((4.5 - angle * 0.04) * 1000) / 1000, unit: "V" },
        { label: L("Zähler", "Counter"), value: learning ? step : 0, unit: "" },
        { label: L("Winkel", "Angle"), value: Math.round(angle * 10) / 10, unit: grad() },
      ];
    }
    if (group === 1) {
      return [
        { label: L("Drehzahl", "Engine speed"), value: idle, unit: "1/min" },
        { label: L("Temperatur", "Temperature"), value: 88, unit: gradC() },
        { label: L("Anteil", "Ratio"), value: klineTrim(), unit: "%" },
        { label: L("Winkel", "Angle"), value: 3.5, unit: grad() },
      ];
    }
    return L("Angefragter Wert liegt außerhalb des Bereichs", "Requested value is out of range");
  }
  if (group === 60) {
    const { angle, step } = learningAngle(tick);
    return [
      { label: L("Anteil", "Ratio"), value: Math.round(angle * 10) / 10, unit: "%" },
      { label: L("Anteil", "Ratio"), value: Math.round((angle * 0.97 + 0.8) * 10) / 10, unit: "%" },
      { label: L("Zähler", "Counter"), value: step, unit: "" },
    ];
  }
  if (group === 1) {
    return [
      { label: L("Drehzahl", "Engine speed"), value: idle, unit: "1/min" },
      { label: L("Temperatur", "Temperature"), value: 88, unit: gradC() },
      { label: L("Anteil", "Ratio"), value: klineTrim(), unit: "%" },
      { label: L("Winkel", "Angle"), value: 3.5, unit: grad() },
    ];
  }
  return L("Grundeinstellung ", "Basic settings ") + group + L(" kennt dieses Steuergerät nicht", " are not supported by this control unit");
}

function toRows(values: MeasuredValue[]): LiveRow[] {
  return values.map((m, i) => ({
    pid: i + 1,
    name: t(W, "Feld %1: %2", i + 1, m.label),
    unit: m.unit,
    text: "",
    value: m.value,
    warn: false,
    time: appSeconds(),
    displayMin: 0,
    displayMax: 0,
  }));
}

function basicSettingTick(): void {
  if (!kkl.open) {
    stopBasicSetting();
    return;
  }
  const result = basicSettingValues(kkl.group, kkl.tick++);
  if (typeof result === "string") {
    stopBasicSetting();
    emit("kkl_message", { message: result, isError: true });
    return;
  }
  emit("basic_setting_values", { values: toRows(result) });
}

function startBasicSetting(group: number): void {
  if (!kkl.open) {
    emit("kkl_message", { message: t(W, "Erst das KKL-Kabel verbinden."), isError: true });
    return;
  }
  kkl.group = group;
  kkl.tick = 0;
  if (kkl.timer) clearInterval(kkl.timer);
  // The control unit drops out of basic settings after about a second
  // without a request; the Worker asks much more often.
  kkl.timer = setInterval(basicSettingTick, Math.max(100, settings().basicSettingIntervalMs));
  emit("kkl_message", { message: t(W, "Grundeinstellung im Messwertblock %1 läuft. Gaspedal nicht antippen.", group), isError: false });
  log(t(W, "Grundeinstellung Block %1 gestartet", group));
}

async function readAdaptation(channel: number): Promise<void> {
  if (!kkl.open) {
    emit("kkl_message", { message: t(W, "Erst das KKL-Kabel verbinden."), isError: true });
    return;
  }
  emit("busy", { active: true });
  await wait(450);
  emit("busy", { active: false });
  if (kkl.kwp2000) {
    emit("kkl_message", {
      message: L(
        "Anpassungskanäle lesen ist bisher nur für KWP1281 umgesetzt. Dieses Steuergerät spricht KWP2000.",
        "Reading adaptation channels is only implemented for KWP1281 so far. This control unit speaks KWP2000.",
      ),
      isError: true,
    });
    return;
  }
  const entry = klineCar.get(kkl.address)?.channels[channel];
  if (!entry) {
    emit("kkl_message", {
      message: L("Anpassungskanal ", "Adaptation channel ") + channel + L(" gibt es hier nicht", " does not exist here"),
      isError: true,
    });
    return;
  }
  emit("adaptation_result", { channel, value: entry.value, shown: toRows(entry.shown()) });
  emit("kkl_message", { message: t(W, "Anpassungskanal %1 steht auf %2.", channel, entry.value), isError: false });
}

async function runActuatorTest(): Promise<void> {
  if (!kkl.open) {
    emit("kkl_message", { message: t(W, "Erst das KKL-Kabel verbinden."), isError: true });
    return;
  }
  emit("busy", { active: true });
  await wait(600);
  emit("busy", { active: false });
  if (kkl.kwp2000) {
    emit("kkl_message", {
      message: L(
        "Die Stellglieddiagnose ist bisher nur für KWP1281 umgesetzt. Dieses Steuergerät spricht KWP2000.",
        "The output test is only implemented for KWP1281 so far. This control unit speaks KWP2000.",
      ),
      isError: true,
    });
    return;
  }
  const list = klineCar.get(kkl.address)?.actuators ?? [];
  if (list.length === 0) {
    emit("kkl_message", {
      message: L("Stellglieddiagnose kennt dieses Steuergerät nicht", "This control unit does not support the output test"),
      isError: true,
    });
    return;
  }
  // After the last actuator the control unit ends the test; the next press
  // starts it again from the first.
  kkl.actuator = kkl.actuator + 1 > list.length ? 0 : kkl.actuator + 1;
  const actuator =
    kkl.actuator === list.length
      ? L("Ende der Stellglieddiagnose", "End of output test")
      : L("Stellglied ", "Actuator ") + list[kkl.actuator]!;
  emit("actuator_result", { actuator });
}

// scanAllEcus: every known address, wake up, identify, read, maybe clear.
async function scanAllEcus(device: string, clearAttempts: number): Promise<void> {
  closeKkl();
  link.cancel = false;
  if (!portOpens(device)) {
    await wait(150);
    emit("kkl_message", { message: portError(device), isError: true });
    emit("scan_finished", { present: 0, withFaults: 0, cancelled: false });
    return;
  }
  const total = KKL_ECUS.length;
  emit("scan_started", { total });
  emit("busy", { active: true });
  let present = 0;
  let withFaults = 0;
  let index = 0;
  for (const [address] of KKL_ECUS) {
    if (link.cancel) break;
    ++index;
    const name = kklEcuName(address);
    const row: EcuScanRow = { address, name, present: false, identification: "", faults: [], note: "" };
    emit("scan_progress", { ecuName: name, index, total, address });
    const unit = klineCar.get(address);
    if (!unit) {
      // The wake-up address plus the wait for a sync byte that never comes.
      await wait(1100);
      row.note = t(W, "meldet sich nicht, vermutlich nicht verbaut");
      emit("scan_ecu_result", { row });
      continue;
    }
    await wait(900);
    row.present = true;
    ++present;
    row.identification = `[${protocolName(unit.kwp2000)}]  ${unit.identification.map((l) => l.trim()).join("  |  ")}`;
    const toFault = (f: KlineFault): FaultRow => ({
      code: String(f.code).padStart(5, "0"),
      kind: f.sporadic ? t(W, "sporadisch") : t(W, "gespeichert"),
      text: L(f.de, f.en),
      throttleRelated: false,
      stored: !f.sporadic,
      permanent: false,
      ecu: "",
    });
    row.faults = unit.faults.map(toFault);
    if (row.faults.length > 0) ++withFaults;
    row.note = row.faults.length === 0 ? t(W, "kein Eintrag") : t(W, "%1 Einträge", row.faults.length);

    // Persistent clear: read back after every attempt. Some control units
    // only accept the clear command on the second try.
    if (clearAttempts > 0 && unit.faults.length > 0) {
      let cleared = false;
      for (let attempt = 1; attempt <= clearAttempts && !cleared; attempt++) {
        emit("clear_progress", { line: t(W, "%1: lösche, Versuch %2 von %3", name, attempt, clearAttempts) });
        await wait(700);
        unit.faults = unit.faults.filter((f) => f.stubborn-- > 0);
        if (unit.faults.length === 0) cleared = true;
      }
      if (cleared) {
        row.faults = [];
        row.note = t(W, "gelöscht");
        --withFaults;
        emit("clear_progress", { line: t(W, "   %1: leer", name) });
      } else {
        row.note = t(W, "%1 Einträge, Löschen ohne Wirkung", row.faults.length);
        emit("clear_progress", { line: t(W, "   %1: bleibt bestehen, die Ursache ist noch da", name) });
      }
    }
    emit("scan_ecu_result", { row });
  }
  emit("busy", { active: false });
  emit("scan_finished", { present, withFaults, cancelled: link.cancel });
}

// ---------------------------------------------------------------------------

export const workshopHandlers: PartialHandlers = {
  ecu_families: (): BrandFamily[] =>
    ecuFamilies().map((key) => {
      const label = FAMILY_LABELS[key];
      return { key, label: label ? L(label[0], label[1]) : key, count: ecuDefinitions(key).length };
    }),
  scan_uds: ({ family }) => {
    void scanUds(family);
  },
  clear_uds_ecu: ({ request, response, extended29, name }) => {
    void clearUdsEcu(request, response, extended29, name);
  },
  probe_adapter: () => {
    void probeAdapter();
  },
  // With ?kkl the simulated cable shows up as a serial port next to
  // whatever the session lists.
  list_serial_ports: async () => {
    const base = (await sessionHandlers.list_serial_ports?.({})) ?? [];
    return kklParam === null
      ? base
      : [...base, { device: KKL_PORT, description: "FTDI FT232R USB UART (KKL 409.1)", likelyAdapter: true }];
  },
  kkl_ecus: (): KklEcuInfo[] => KKL_ECUS.map(([address]) => ({ address, name: kklEcuName(address) })),
  open_kkl: ({ device, ecuAddress }) => {
    void openKkl(device, ecuAddress);
  },
  close_kkl: () => closeKkl(),
  start_basic_setting: ({ group }) => startBasicSetting(group),
  stop_basic_setting: () => stopBasicSetting(),
  set_basic_setting_session: ({ mode }) => {
    kkl.session = mode;
    log(t(W, "Sitzungsart für die Grundeinstellung: 0x%1", hex(mode, 2)));
  },
  scan_all_ecus: ({ device, clearAttempts }) => {
    void scanAllEcus(device, clearAttempts);
  },
  read_adaptation_channel: ({ channel }) => {
    void readAdaptation(channel);
  },
  run_actuator_test: () => {
    void runActuatorTest();
  },
};
