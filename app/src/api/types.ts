// Data the backend hands to the interface. Each type mirrors a struct of the
// backend (app/src-tauri/src: worker, settings, garage, report; ported from
// the Qt version) or of the crates they use. Field names follow the Rust
// names in camelCase; the backend serialises with
// #[serde(rename_all = "camelCase")]. Texts arrive in the active language.

// --- Session.h ----------------------------------------------------------------

export interface FaultRow {
  code: string;
  // Display text of the kind ("gespeichert", "sporadisch", "permanent").
  kind: string;
  text: string;
  throttleRelated: boolean;
  // Confirmed (stored) fault, as opposed to pending or permanent.
  stored: boolean;
  // Which control unit reported it, e.g. "Motor (7E8)". Empty on the K-line.
  ecu: string;
}

export interface LiveRow {
  // The live key: pid | (sub << 8). Equals the PID for single values.
  pid: number;
  name: string;
  unit: string;
  // Set for status values ("geschlossener Regelkreis").
  text: string;
  value: number;
  warn: boolean;
  // Seconds since the program started, taken right after the reply.
  time: number;
  // Sensible scale for charts and gauges, 0/0 when the table has none.
  displayMin: number;
  displayMax: number;
}

export interface FreezeFrameRow {
  frame: number;
  dtc: string;
  ecu: string;
  values: LiveRow[];
}

export interface UdsDtcRow {
  code: string;
  failureText: string;
  text: string;
  testFailed: boolean;
  pending: boolean;
  confirmed: boolean;
  warningLamp: boolean;
}

// EcuKind of core/Uds.h.
export type EcuKind = number;

export interface UdsEcuRow {
  request: number;
  response: number;
  extended29: boolean;
  kind: EcuKind;
  name: string;
  present: boolean;
  dtcsRead: boolean;
  partNumber: string;
  softwareVersion: string;
  hardwareVersion: string;
  systemName: string;
  serial: string;
  dtcs: UdsDtcRow[];
  note: string;
}

export interface CustomRow {
  key: number;
  name: string;
  shortName: string;
  unit: string;
  // Empty for imported values.
  pack: string;
  source: string;
  request: string;
  header: string;
  // The English short name, the same in every language; the EV page finds
  // its values ("SOH", "Batt V") by it. Falls back to shortName when absent.
  shortNameEn?: string;
}

// analysis::Level: 0 good, 1 info, 2 warning, 3 critical.
export type Level = 0 | 1 | 2 | 3;

export interface HealthFindingRow {
  // The stable rule id of the health check, e.g. "fuel_trim_bank1".
  id: string;
  level: Level;
  area: string;
  title: string;
  explanation: string;
  advice: string;
}

export interface MonitorGuideRow {
  name: string;
  complete: boolean;
  explanation: string;
  drivePattern: string;
  typicalTime: string;
  coldStart: boolean;
}

export interface AdapterCheckRow {
  command: string;
  // The ELM version that introduced the command.
  version: string;
  supported: boolean;
  purpose: string;
}

export interface AdapterRow {
  reachable: boolean;
  version: string;
  description: string;
  isStn: boolean;
  stnId: string;
  voltage: number;
  hasVoltage: boolean;
  roundTripMs: number;
  roundTripWithHintMs: number;
  behavesLike: string;
  acceptsUnknown: boolean;
  verdict: string;
  facts: string[];
  speedTips: string[];
  checks: AdapterCheckRow[];
}

export interface HealthRow {
  score: number;
  // analysis::TrafficLight: 0 green, 1 yellow, 2 red.
  grade: 0 | 1 | 2;
  summary: string;
  findings: HealthFindingRow[];
  // "title: reason"
  notChecked: string[];
  monitors: MonitorGuideRow[];
  checksRun: number;
  diesel: boolean;
}

// The used car check (analysis::used_car through Worker::runUsedCarCheck).
export type UsedCarSection = "identity" | "mileage" | "clearing" | "readiness" | "faults" | "health" | "battery";

export interface UsedCarItemRow {
  // Stable rule id, e.g. "mileage_mismatch".
  id: string;
  section: UsedCarSection;
  level: Level;
  title: string;
  explanation: string;
  // What to ask the seller, empty if nothing.
  ask: string;
}

export interface UsedCarMileageRow {
  source: string;
  km: number;
}

// Numbers are -1 where the car said nothing.
export interface UsedCarRow {
  // 0 green, 1 yellow, 2 red.
  light: 0 | 1 | 2;
  headline: string;
  summary: string;
  vin: string;
  papersVin: string;
  vehicle: string;
  modelYear: number;
  // Engine (PID A6) first, then the other control units.
  mileage: UsedCarMileageRow[];
  distanceSinceClearKm: number;
  minutesSinceClear: number;
  warmUps: number;
  readinessComplete: number;
  readinessTotal: number;
  storedFaults: number;
  pendingFaults: number;
  permanentFaults: number;
  healthScore: number;
  electric: boolean;
  batteryPack: string;
  batterySoh: number;
  batterySoc: number;
  // In the order of the sections.
  items: UsedCarItemRow[];
  // The most serious first.
  questions: string[];
  // "title: reason"
  notChecked: string[];
}

export interface EcuScanRow {
  address: number;
  name: string;
  present: boolean;
  identification: string;
  faults: FaultRow[];
  note: string;
}

export interface MonitorRow {
  name: string;
  component: string;
  value: number;
  minimum: number;
  maximum: number;
  unit: string;
  hasMinimum: boolean;
  hasMaximum: boolean;
  passed: boolean;
}

export interface OxygenRow {
  name: string;
  sensor: number;
  value: number;
  unit: string;
}

export interface StatusRow {
  valid: boolean;
  mil: boolean;
  faultCount: number;
  monitorNames: string[];
  monitorReady: boolean[];
}

// --- Settings.h -----------------------------------------------------------------

export interface AppSettings {
  // Connection. 0 means: try the usual baud rates.
  baudOverride: number;
  // Order in which the protocols are tried: "kline" suits the A2, "can" the A4 B7.
  protocolOrder: string;
  // All timeouts are multiplied by this factor (percent).
  timeoutPercent: number;
  // 0 standard (ATAT1), 1 faster on STN adapters only, 2 always ATAT2.
  adaptiveTiming: number;
  lastPort: string;
  autoConnect: boolean;
  // Live values. Zero means: as fast as possible.
  pollIntervalMs: number;
  chartWindowSeconds: number;
  // Clearing.
  clearAttempts: number;
  confirmClear: boolean;
  // K-Line.
  basicSettingSession: number;
  basicSettingIntervalMs: number;
  kklEcuAddress: number;
  // Log.
  logRaw: boolean;
  logToFile: boolean;
  logPath: string;
  // Window.
  rememberGeometry: boolean;
  checkUpdates: boolean;
  // "auto" or a language code.
  language: string;
  // "auto", "dark" or "light".
  appearance: string;
  // Profi mode shows every page, beginners the everyday ones. New installs
  // start as beginners, settings from before the switch as Profi.
  proMode: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  baudOverride: 0,
  protocolOrder: "kline",
  timeoutPercent: 100,
  adaptiveTiming: 0,
  lastPort: "",
  autoConnect: false,
  pollIntervalMs: 20,
  chartWindowSeconds: 60,
  clearAttempts: 5,
  confirmClear: true,
  basicSettingSession: 0x85,
  basicSettingIntervalMs: 180,
  kklEcuAddress: 0x01,
  logRaw: true,
  logToFile: false,
  logPath: "",
  rememberGeometry: true,
  checkUpdates: true,
  language: "auto",
  appearance: "auto",
  proMode: false,
};

// --- Garage.h -------------------------------------------------------------------
// Dates travel as ISO 8601 strings: date-times as "2026-11-10T21:04:00",
// plain dates as "2026-11-10", empty for "not set".

export interface GarageCheck {
  when: string;
  score: number;
  mil: boolean;
  openMonitors: number;
  odometerKm: number;
  codes: string[];
  texts: string[];
}

export interface GarageTrip {
  start: string;
  end: string;
  distanceKm: number;
  fuelLitres: number;
  averageL100: number;
  cost: number;
  co2Kg: number;
  maxSpeedKmh: number;
  ecoScore: number;
}

export interface GarageFuel {
  date: string;
  odometerKm: number;
  litres: number;
  pricePerLitre: number;
  full: boolean;
}

export interface GarageService {
  id: string;
  title: string;
  intervalKm: number;
  intervalMonths: number;
  lastDate: string;
  lastKm: number;
  note: string;
}

export interface GarageCar {
  vin: string;
  name: string;
  description: string;
  bodyStyle: number;
  note: string;
  firstSeen: string;
  lastSeen: string;
  checks: GarageCheck[];
  trips: GarageTrip[];
  fuel: GarageFuel[];
  services: GarageService[];
  // The owner's 3D model for the X-ray view (path of a .glb or .gltf) and
  // whether it had to be turned around; absent without one.
  xrayModel?: string;
  xrayModelTurned?: boolean;
}

// --- Report.h -------------------------------------------------------------------

export interface ReportFinding {
  severity: string;
  title: string;
  text: string;
}

export interface ReportExtras {
  score: number;
  scoreCaption: string;
  findings: ReportFinding[];
  ecus: UdsEcuRow[];
}

// --- Knowledge from the core (data/*.h) ---------------------------------------

// data/Severity.h: 0 Info, 1 Low, 2 Medium, 3 High, 4 Critical.
export type Severity = 0 | 1 | 2 | 3 | 4;
// data/DtcDatabase.h CostClass: 0 Unknown, 1 Cheap, 2 Medium, 3 Expensive.
export type CostClass = 0 | 1 | 2 | 3;

export interface DtcInfo {
  code: string;
  text: string;
  severity: Severity;
  // Short label of the severity in the active language ("Mittel").
  severityLabel: string;
  // DtcSystem as a number, plus its name.
  system: number;
  systemName: string;
  advice: string;
  causes: string[];
  checks: string[];
  costClass: CostClass;
  // costClassLabel() and costClassHint(), empty for Unknown.
  costLabel: string;
  costHint: string;
  manufacturerSpecific: boolean;
  known: boolean;
  hasDetails: boolean;
  throttleRelated: boolean;
  family: string;
  familyLabel: string;
}

// data/DtcLocation.h CarZone, in the order of the enum.
export enum CarZone {
  Unknown,
  Intake,
  Throttle,
  Injection,
  Ignition,
  Cylinders,
  Cooling,
  Turbo,
  Exhaust,
  FuelTank,
  Gearbox,
  Ecu,
  Electrics,
  Network,
  Wheels,
  Cabin,
}

export interface ZoneInfo {
  zone: CarZone;
  name: string;
  hint: string;
}

export interface DtcLexiconEntry {
  code: string;
  family: string;
  familyLabel: string;
  text: string;
  severity: Severity;
  system: number;
  // dtcSystemName(system) and severityLabel(severity) in the active
  // language, for the lexicon columns.
  systemName?: string;
  severityLabel?: string;
}

export interface BrandFamily {
  key: string;
  label: string;
  count: number;
}

// One K-line control unit of core/Kwp1281.h: the VCDS address and its name
// ("01 Motor").
export interface KklEcuInfo {
  address: number;
  name: string;
}

// One value of the PID table (data/Pids.h PidValueSpec plus its PID).
export interface PidInfo {
  key: number;
  pid: number;
  sub: number;
  name: string;
  unit: string;
  // PidCategory as a number, plus its name in the active language.
  category: number;
  categoryName: string;
  displayMin: number;
  displayMax: number;
  warnLow: number;
  warnHigh: number;
  // True for status values that come with a text.
  isText: boolean;
  // Name of the PID as a whole (PidSpec::title), for PIDs with several
  // values; the live list shows one entry per PID under this name.
  title?: string;
}

// data/PidPacks.h: one pack of manufacturer values.
export interface PidPackInfo {
  id: string;
  title: string;
  brand: string;
  description: string;
  values: CustomRow[];
  // packCategory(): PidCategory as a number, 8 (HybridEv) for the electric
  // cars, 10 (Custom) for the demo pack. The EV page lists the first kind.
  category: number;
  // True for a pack imported from OBDb (obdb_packs); its value names are
  // OBDb's own and stay English.
  obdb?: boolean;
}

// --- core/analysis (the measure pages) -------------------------------------------

// FuelType of analysis/Common.h: 0 unknown, 1 petrol, 2 diesel.
export type FuelType = 0 | 1 | 2;

// analysis/TripComputer.h
export interface TripConfig {
  fuel: FuelType;
  // Needed for speed density only.
  displacementLitres: number;
  volumetricEfficiency: number;
  // 0 = no cost.
  fuelPricePerLitre: number;
  maxGapSeconds: number;
  liveWindowSeconds: number;
}

export interface EcoScore {
  total: number;
  smoothness: number;
  // -1 = not known.
  engineSpeed: number;
  idling: number;
  harshAccelerations: number;
  harshBrakings: number;
  explanation: string;
}

// FuelSource: 0 none, 1 fuel rate, 2 mass air flow, 3 speed density.
export interface TripSummary {
  distanceKm: number;
  fuelLitres: number;
  drivingSeconds: number;
  movingSeconds: number;
  idleSeconds: number;
  gapSeconds: number;
  averageLitresPer100Km: number;
  liveLitresPer100Km: number;
  liveLitresPerHour: number;
  idleLitresPerHour: number;
  co2GramsPerKm: number;
  co2Kg: number;
  cost: number;
  averageSpeedKmh: number;
  averageMovingSpeedKmh: number;
  maxSpeedKmh: number;
  idleShare: number;
  fuelSource: 0 | 1 | 2 | 3;
  fuelIsRough: boolean;
  fuelNote: string;
  eco: EcoScore;
}

// analysis/PerformanceTimer.h
export interface SpeedSample {
  t: number;
  kmh: number;
}

export interface TimedRun {
  found: boolean;
  seconds: number;
  startTime: number;
  endTime: number;
  distanceMeters: number;
  trapSpeedKmh: number;
  reliable: boolean;
}

export interface PerformanceReport {
  zeroTo100: TimedRun;
  eightyTo120: TimedRun;
  zeroTo60mph: TimedRun;
  quarterMile: TimedRun;
  brake100To0: TimedRun;
  sampleRateHz: number;
  accuracySeconds: number;
  samplesUsed: number;
  samplesDropped: number;
  accuracyHint: string;
}

// analysis/BatteryTest.h. BatteryType: 0 unknown, 1 flooded, 2 EFB, 3 AGM.
export type BatteryType = 0 | 1 | 2 | 3;

export interface VoltageSample {
  t: number;
  volts: number;
}

// The optionals of the C++ struct are null when not measured.
export interface BatteryInput {
  type: BatteryType;
  startStop: boolean;
  restingVolts: number | null;
  minutesSinceEngineOff: number | null;
  cranking: VoltageSample[];
  crankingMinimumVolts: number | null;
  idleVolts: number | null;
  loadedVolts: number | null;
  ambientCelsius: number | null;
}

export interface BatteryStepResult {
  // "resting", "cranking", "charging_idle", "charging_load"
  id: string;
  level: Level;
  volts: number;
  title: string;
  explanation: string;
  advice: string;
}

export interface BatteryReport {
  overall: Level;
  stateOfChargePercent: number | null;
  steps: BatteryStepResult[];
  missingSteps: string[];
  summary: string;
  crankingSampleRateHz: number;
}

// batteryTestSteps(): the instruction of each step, in order.
export interface BatteryStepInfo {
  id: string;
  instruction: string;
}

// data/VehicleCatalog.h, flattened for the interface.
export enum BodyType {
  Hatchback,
  Sedan,
  Estate,
  Suv,
  Van,
  Coupe,
  Convertible,
  Pickup,
  Mpv,
  Microcar,
  Roadster,
}

export interface VehicleGenerationInfo {
  // Stable index into the catalog, for passing a generation back.
  id: number;
  brand: string;
  model: string;
  generation: string;
  yearFrom: number;
  yearTo: number;
  body: BodyType;
  bodyName: string;
  powertrains: number;
  powertrainText: string;
  displayName: string;
  yearsText: string;
  protocolOrder: string;
  note: string;
  usMarket: boolean;
}

// PT_* flags of data/VehicleCatalog.h (VehicleGenerationInfo.powertrains).
export const Powertrain = {
  Petrol: 1 << 0,
  Diesel: 1 << 1,
  Hybrid: 1 << 2,
  PlugIn: 1 << 3,
  Electric: 1 << 4,
  Gas: 1 << 5,
} as const;

// vehicles::anatomy, in the order of the enums.
export enum EngineLayout {
  None,
  FrontTransverse,
  FrontLongitudinal,
  Mid,
  Rear,
}

export enum DrivenAxles {
  Front,
  Rear,
  All,
}

export enum TractionBattery {
  None,
  Hybrid,
  PlugIn,
  Floor,
}

// Shape of a car for the X-ray view: outer dimensions in millimetres and
// where engine, motors and batteries sit.
export interface VehicleAnatomy {
  // The catalog generation (brand and model also for the demo car; generation
  // is empty without a catalog entry) and the year shown.
  brand: string;
  model: string;
  generation: string;
  year: number;
  lengthMm: number;
  widthMm: number;
  heightMm: number;
  wheelbaseMm: number;
  trackMm: number;
  wheelMm: number;
  // True when length or wheelbase come from Wikidata (CC0).
  measured: boolean;
  dimensionSource: "wikidata" | "typical";
  // Bits of the real values: 1 length, 2 width, 4 height, 8 wheelbase, 16 track.
  measuredFields: number;
  // The Wikidata item ("Q62362"), "" without.
  wikidataId: string;
  // Empty weight from Wikidata, 0 without.
  massKg: number;
  // The name of every body type (BodyType order), for the body variants.
  bodyNames: string[];
  dimensionsText: string;
  powertrain: number;
  powertrainChoices: { flag: number; name: string }[];
  engine: EngineLayout;
  engineName: string;
  driven: DrivenAxles;
  drivenName: string;
  frontMotor: boolean;
  rearMotor: boolean;
  battery: TractionBattery;
  fuelTank: boolean;
  reason: string;
}

// What diagnosis is possible for a generation in a given year.
export interface VehicleDiagnosisInfo {
  generation: VehicleGenerationInfo;
  year: number;
  // ObdSupport: 0 None, 1 Manufacturer, 2 Kkl, 3 Partial, 4 Full.
  support: number;
  supportName: string;
  connector: number;
  connectorName: string;
  locationText: string;
  diagnosisText: string;
  kklPossible: boolean;
  protocolOrder: string;
}

export interface VinInfo {
  valid: boolean;
  vin: string;
  wmi: string;
  manufacturer: string;
  brand: string;
  brandCandidates: string[];
  country: string;
  region: string;
  northAmerica: boolean;
  modelYears: number[];
  modelYear: number;
  modelYearCertain: boolean;
  yearCode: string;
  checkDigitApplies: boolean;
  checkDigitOk: boolean;
  checkDigit: string;
  expectedCheckDigit: string;
  plant: string;
  serial: string;
  modelCode: string;
  problems: string[];
  notes: string[];
}

export interface CatalogMatch {
  generation: VehicleGenerationInfo | null;
  brand: string;
  year: number;
  confidence: number;
  alternatives: VehicleGenerationInfo[];
  reason: string;
}

// app/VehicleDiagram.h BodyStyle, in the order of the enum.
export type BodyStyle = number;

// app/VehicleIdentity.h
export interface IdentifiedVehicle {
  known: boolean;
  title: string;
  subtitle: string;
  brand: string;
  year: number;
  confidence: number;
  body: BodyStyle;
  generationId: number | null;
  socketHint: string;
}

// --- Ports and adapters ------------------------------------------------------

export interface SerialPortInfo {
  device: string;
  description: string;
  // True for ports that look like an OBD adapter (USB serial chips, Bluetooth SPP).
  likelyAdapter: boolean;
  // The chip the USB ids point to ("CH340", "FTDI FT232R"), empty if unknown (PortInfo::chipGuess).
  chipGuess?: string;
}

export interface BleDeviceRow {
  id: string;
  name: string;
  score: number;
  // 1 = Bluetooth LE, 2 = classic, 3 = both.
  kinds: number;
}

export interface J2534DeviceRow {
  name: string;
  vendor: string;
  library: string;
  // False when only a 32 bit driver is installed.
  usable: boolean;
}

// --- App ---------------------------------------------------------------------

export interface AppInfo {
  version: string;
  // True when the BLE support is compiled in.
  hasBle: boolean;
  platform: "windows" | "macos" | "linux" | "browser";
  documentsDir: string;
}

export interface UpdateInfo {
  version: string;
  url: string;
}

export interface FileFilter {
  name: string;
  extensions: string[];
}

export interface OpenedTextFile {
  path: string;
  content: string;
}

// --- caruspex-advice: "Was soll ich jetzt tun?" --------------------------------------

export interface AdviceInput {
  brand: string;
  // The fault memory has been read; tells "no faults" from "not read".
  faultsRead: boolean;
  faults: { code: string; kind: string; stored: boolean; ecu: string; text: string }[];
  // null when PID 01 has not been read.
  milOn: boolean | null;
  readiness: { name: string; ready: boolean }[];
  findings: { id: string; level: Level; title: string; explanation: string }[];
  // The latest value per PID.
  live: { pid: number; value: number }[];
  // What the driver sees; OBD cannot tell a flashing lamp from a steady one.
  milFlashing: boolean;
  redWarningLamp: boolean;
}

// 0 stop now, 1 workshop soon, 2 can wait.
export type Urgency = 0 | 1 | 2;
// 0 unknown, 1 free, 2 below about 200 euros, 3 about 200 to 800, 4 above.
export type AdviceCost = 0 | 1 | 2 | 3 | 4;

export interface AdviceItem {
  urgency: Urgency;
  source: "observation" | "fault" | "finding" | "live" | "lamp" | "readiness";
  // The fault code, empty for other items.
  code: string;
  title: string;
  // One plain sentence: why it is in this step.
  why: string;
  cost: AdviceCost;
  costText: string;
  // What to tell the workshop.
  workshop: string;
  // Measured values that back the item up.
  evidence: string[];
  // The page with the details, empty for none.
  page: "" | "faults" | "health" | "battery";
}

export interface AdviceReport {
  headline: string;
  summary: string;
  nothingRead: boolean;
  // Most urgent first.
  items: AdviceItem[];
}

// --- caruspex-advice: the connection assistant ---------------------------------------

export interface ConnectCheck {
  id: string;
  // problem: most likely the reason; check: worth a look; fine: known good.
  state: "problem" | "check" | "fine";
  title: string;
  text: string;
}

export interface ConnectHelp {
  // What the attempt found out, one line each.
  found: string[];
  items: ConnectCheck[];
}
