// The contract between the interface and the Rust backend.
//
// Commands are Tauri commands (snake_case names; the argument object uses
// camelCase keys, which Tauri maps to the snake_case parameters of the Rust
// function). Events are emitted by the backend with app.emit(name, payload).
//
// Most commands mirror a job of the backend worker (app/src-tauri/src/worker.rs) and most
// events one of its signals; long running work answers through events, the
// command itself returns at once. Commands below "Knowledge" are plain
// lookups into the core data and return their result directly.

import type {
  AdapterRow,
  AdviceInput,
  AdviceReport,
  ConnectHelp,
  ConnectRoute,
  CrashReport,
  AppInfo,
  AppSettings,
  BleDeviceRow,
  BmwUnitRow,
  BluetoothAccess,
  BrandFamily,
  CatalogMatch,
  CustomRow,
  DtcInfo,
  DtcLexiconEntry,
  EcuKind,
  EcuScanRow,
  FaultRow,
  FileFilter,
  FreezeFrameRow,
  GarageCar,
  HealthRow,
  J1939IdRow,
  J1939UnitRow,
  J1939ValueRow,
  J2534DeviceRow,
  KklEcuInfo,
  KklSavedReport,
  KklTestReport,
  KklTestStep,
  LiveRow,
  MonitorRow,
  OpenedTextFile,
  OxygenRow,
  PidInfo,
  PidPackInfo,
  SerialPortInfo,
  SharedRecording,
  StatusRow,
  UdsEcuRow,
  UpdateInfo,
  UpdateOutcome,
  UsedCarRow,
  VendorCanRow,
  VehicleDiagnosisInfo,
  VehicleAnatomy,
  VehicleGenerationInfo,
  VinInfo,
  ZoneInfo,
} from "./types";
// Types of the measure pages (trip computer, performance, battery test).
import type { BatteryInput, BatteryReport, BatteryStepInfo, PerformanceReport, SpeedSample, TripConfig, TripSummary } from "./types";
import type { TuneComparison, TuneInput, TuneResult } from "./types";

type Cmd<A, R> = { args: A; result: R };
type None = Record<never, never>;

export interface Commands {
  // --- App and window ---------------------------------------------------------
  app_info: Cmd<None, AppInfo>;
  // Tells the backend which language the interface shows, so core texts
  // (fault descriptions, value names, messages) arrive in it.
  set_language: Cmd<{ code: string }, void>;
  // Zoom of the whole window (the font size of the settings), 1 is normal.
  set_zoom: Cmd<{ scale: number }, void>;
  // Asks GitHub for latest.json of the newest release once; null when up
  // to date or offline.
  check_for_update: Cmd<None, UpdateInfo | null>;
  // Downloads the release check_for_update found, checks its signature and
  // installs it; progress comes as update_progress. Rejects with a message.
  install_update: Cmd<None, UpdateOutcome>;
  open_url: Cmd<{ url: string }, void>;
  // The newest crash report the user has not decided about; null if none.
  crash_report_pending: Cmd<None, CrashReport | null>;
  // The user decided: the report stays on the computer, is not offered again.
  crash_report_done: Cmd<{ path: string }, void>;

  // --- Settings (Settings.h) -------------------------------------------------
  load_settings: Cmd<None, AppSettings>;
  // Saves and applies: poll interval, timeouts, raw log, log file, session type.
  save_settings: Cmd<{ values: AppSettings }, void>;
  // Opens the folder that holds the settings file (SettingsPage "Ablageort öffnen").
  open_config_location: Cmd<None, void>;
  // A log line of the interface into the log file (if logging to a file).
  // The worker's log_line events are written by the backend itself.
  write_log: Cmd<{ line: string }, void>;

  // --- Files -----------------------------------------------------------------
  // Shows a save dialog and writes the text. Returns the path, null if cancelled.
  save_text_file: Cmd<{ title: string; suggestedName: string; filters: FileFilter[]; content: string }, string | null>;
  // Shows a save dialog and renders the HTML into a PDF. Returns the path;
  // null if cancelled, or where only the print dialog could be offered.
  save_pdf: Cmd<{ title: string; suggestedName: string; html: string }, string | null>;
  // Writes the report without asking (--report): PDF, or HTML for .html.
  // During a --report run the program ends afterwards.
  write_report: Cmd<{ path: string; html: string }, boolean>;
  cli_report_failed: Cmd<{ message: string }, void>;
  // Shows an open dialog and reads the file. null if cancelled.
  open_text_file: Cmd<{ title: string; filters: FileFilter[] }, OpenedTextFile | null>;
  // Only asks for a path (recording target, Torque CSV, replay file, log file).
  pick_save_path: Cmd<{ title: string; suggestedName: string; filters: FileFilter[] }, string | null>;
  pick_open_path: Cmd<{ title: string; filters: FileFilter[] }, string | null>;
  // The bytes of a 3D model for the X-ray view (.glb, .gltf and the files a
  // .gltf names next to it).
  read_model_file: Cmd<{ path: string }, ArrayBuffer>;

  // --- Garage (Garage.h): the whole file in and out ------------------------------
  // The interface keeps the logic (Garage.cpp); the backend stores the JSON
  // in the user's data folder, same format as the Qt version.
  garage_load: Cmd<None, GarageCar[]>;
  garage_save: Cmd<{ cars: GarageCar[] }, void>;

  // --- Connecting (Worker slots) ---------------------------------------------
  list_serial_ports: Cmd<None, SerialPortInfo[]>;
  list_j2534_devices: Cmd<None, J2534DeviceRow[]>;
  // SocketCAN interfaces of this computer (core/CanBus.h socketCanInterfaces), empty off Linux.
  list_socketcan_interfaces: Cmd<None, string[]>;
  // Channels of PEAK PCAN-Basic and Kvaser CANlib (Windows), when installed.
  list_can_vendor_channels: Cmd<None, VendorCanRow[]>;
  // Empty device: search every port.
  // remembered: the way that worked last time for this car; it is tried
  // first, quietly, and only if no adapter answers there the search runs.
  // truck: a truck or bus over SAE J1939 at this bit rate in kbit/s (250,
  // 500, or 0 for both in turn); left out for cars.
  open_serial: Cmd<{ device: string; baud: number; protocolOrder: string; remembered?: ConnectRoute; truck?: number }, void>;
  open_tcp: Cmd<{ host: string; port: number; protocolOrder: string; remembered?: ConnectRoute; truck?: number }, void>;
  open_demo: Cmd<{ protocolOrder: string; truck?: number }, void>;
  // Empty host: look for the car (ENET and DoIP).
  open_enet: Cmd<{ host: string; protocolOrder: string; remembered?: ConnectRoute }, void>;
  // "j2534:<dll>", "slcan:COM5", "socketcan:can0", with "@<bitrate>".
  open_can: Cmd<{ spec: string; protocolOrder: string; remembered?: ConnectRoute; truck?: number }, void>;
  // Answers with the ble_scan_finished event.
  scan_ble: Cmd<{ milliseconds: number }, void>;
  // Android: may the app use Bluetooth, and the system question for it.
  bluetooth_access: Cmd<None, BluetoothAccess>;
  request_bluetooth_access: Cmd<None, BluetoothAccess>;
  // kinds: 1 = Bluetooth LE, 2 = classic, 3 = both. Empty id scans.
  open_ble: Cmd<{ id: string; name: string; kinds: number; protocolOrder: string; remembered?: ConnectRoute; truck?: number }, void>;
  open_replay: Cmd<{ path: string; realTime: boolean; protocolOrder: string }, void>;
  close_adapter: Cmd<None, void>;
  start_recording: Cmd<{ path: string }, void>;
  stop_recording: Cmd<None, void>;
  // "Mitschnitt teilen": the recording without the last VIN digits, the
  // port or address and the time of day, with a preview. Writes nothing;
  // vin is the VIN of the connected car, empty if none.
  share_recording: Cmd<{ path: string; vin: string }, SharedRecording>;

  // --- Reading -----------------------------------------------------------------
  read_faults: Cmd<None, void>;
  clear_faults: Cmd<None, void>;
  // VIN, freeze frames, a first set of live values.
  read_details: Cmd<None, void>;
  // An empty list keeps the loop idle (only manufacturer values, if any).
  start_live: Cmd<{ pids: number[] }, void>;
  stop_live: Cmd<None, void>;
  send_raw: Cmd<{ command: string }, void>;
  read_monitors: Cmd<None, void>;
  run_health_check: Cmd<{ seconds: number }, void>;
  // The used car check; papersVin is the VIN from the papers, may be empty.
  // Answers with used_car_progress and used_car_result.
  run_used_car_check: Cmd<{ papersVin: string }, void>;
  probe_adapter: Cmd<None, void>;
  bruteforce_clear: Cmd<{ attempts: number }, void>;
  // Cancels a running scan or health check.
  cancel_scan: Cmd<None, void>;
  start_voltage_poll: Cmd<{ intervalMs: number }, void>;
  stop_voltage_poll: Cmd<None, void>;

  // --- Manufacturer and user values (CustomPids) -------------------------------
  read_custom: Cmd<{ keys: number[] }, void>;
  start_custom_poll: Cmd<{ keys: number[] }, void>;
  import_custom_csv: Cmd<{ path: string }, void>;

  // --- Measuring with core/analysis -------------------------------------------
  // Trip computer (analysis::TripComputer). trip_reset builds a new one with
  // the config (totals start from zero); while started, the backend feeds it
  // every live value the loop reads, like TripPage::onLiveValue did.
  trip_reset: Cmd<{ config: TripConfig }, void>;
  trip_start: Cmd<None, void>;
  // Stops feeding and answers with the summary at that moment.
  trip_stop: Cmd<None, TripSummary>;
  // Polled by the trip page every half second; also works after closing.
  trip_summary: Cmd<None, TripSummary>;
  // analysis::analyzePerformance on raw speed samples (PID 0D).
  analyze_performance: Cmd<{ samples: SpeedSample[] }, PerformanceReport>;
  // analysis::evaluateBattery and the step instructions of the guided test.
  evaluate_battery: Cmd<{ input: BatteryInput }, BatteryReport>;
  battery_test_steps: Cmd<None, BatteryStepInfo[]>;

  // --- Tuning simulator (caruspex-tune). Computes only, never reaches the car --
  // Stock and changed curve of the N55 model under the same conditions.
  tune_simulate: Cmd<{ input: TuneInput }, TuneResult>;
  // A recording's full-load samples against the model.
  tune_compare_log: Cmd<{ path: string; input: TuneInput }, TuneComparison>;
  // The same with an invented full-load pull, to see how the table reads.
  tune_compare_example: Cmd<{ input: TuneInput }, TuneComparison>;

  // --- UDS -----------------------------------------------------------------------
  // Empty family: generic sweep over the usual address range.
  // Brand families that have an address list (ecuFamilies()); count is the
  // number of units in the list.
  ecu_families: Cmd<None, BrandFamily[]>;
  scan_uds: Cmd<{ family: string }, void>;
  clear_uds_ecu: Cmd<{ request: number; response: number; extended29: boolean; name: string }, void>;

  // --- K-line (KWP1281 / KWP2000 over a KKL cable) ------------------------------
  open_kkl: Cmd<{ device: string; ecuAddress: number }, void>;
  close_kkl: Cmd<None, void>;
  start_basic_setting: Cmd<{ group: number }, void>;
  stop_basic_setting: Cmd<None, void>;
  set_basic_setting_session: Cmd<{ mode: number }, void>;
  // clearAttempts > 0: clear everywhere and verify.
  scan_all_ecus: Cmd<{ device: string; clearAttempts: number }, void>;
  read_adaptation_channel: Cmd<{ channel: number }, void>;
  run_actuator_test: Cmd<None, void>;
  // Control units the K-line pages offer (knownEcus() with ecuName()).
  kkl_ecus: Cmd<None, KklEcuInfo[]>;
  // The first test of a KKL cable (worker/kkl_test.rs), both read only and
  // only on the port given: the cable test on USB only, and the guided test
  // on the car. "sim:a2" and "sim:a2:<fault>" are the simulated A2. Steps
  // come as kkl_test_step, the end as kkl_test_finished.
  kkl_cable_check: Cmd<{ device: string }, void>;
  kkl_first_test: Cmd<{ device: string }, void>;
  // Writes the report (JSON and text) into test-reports in the config
  // folder; VIN and immobiliser id masked unless includeVin. context: lines
  // the page adds (car, checklist).
  kkl_test_save_report: Cmd<{ report: KklTestReport; context: string[]; includeVin: boolean }, KklSavedReport>;
  // Shows a saved report in the file manager (only inside test-reports).
  kkl_test_reveal: Cmd<{ path: string }, void>;

  // --- BMW E series on the K-line (DS2, KWP2000, BMW-FAST) ----------------------
  // device: a serial port with a K+DCAN or KKL cable, or "sim:e46" /
  // "sim:e60". clear: clear every unit with entries (only after asking).
  scan_bmw: Cmd<{ device: string; clear: boolean }, void>;
  // protocol: the key of the unit's row ("ds2", "kwp2000", "bmw-fast").
  clear_bmw_unit: Cmd<{ device: string; address: number; protocol: string }, void>;

  // --- Knowledge (core data, answered directly) --------------------------------
  lookup_dtc: Cmd<{ code: string; brand: string }, DtcInfo>;
  locate_dtc: Cmd<{ code: string }, ZoneInfo>;
  // "Was soll ich jetzt tun?" (crate caruspex-advice): everything the
  // interface knows about the car in, one list in three steps out.
  what_to_do: Cmd<{ input: AdviceInput }, AdviceReport>;
  dtc_lexicon: Cmd<None, DtcLexiconEntry[]>;
  brand_families: Cmd<None, BrandFamily[]>;
  // Family key of a brand ("Skoda" -> "VAG"), empty if none.
  brand_family: Cmd<{ brand: string }, string>;
  pid_table: Cmd<None, PidInfo[]>;
  default_live_pids: Cmd<None, number[]>;
  pid_packs: Cmd<None, PidPackInfo[]>;
  // The OBDb packs for the detected car: catalog generation and model year (0 = any).
  obdb_packs: Cmd<{ generationId: number | null; year: number }, PidPackInfo[]>;
  catalog_brands: Cmd<None, string[]>;
  catalog_models: Cmd<{ brand: string }, string[]>;
  catalog_generations: Cmd<{ brand: string; model: string }, VehicleGenerationInfo[]>;
  catalog_search: Cmd<{ query: string; limit: number }, VehicleGenerationInfo[]>;
  catalog_size: Cmd<None, number>;
  // Support, connector, socket and the plain sentence for one generation and year.
  catalog_diagnosis: Cmd<{ generationId: number; year: number }, VehicleDiagnosisInfo>;
  // Dimensions and drivetrain for the X-ray view (vehicles::anatomy). A
  // generation (generationId >= 0) brings its own body and brand; otherwise
  // body, brand and model are used, as for the demo car. powertrain is one
  // PT_* flag, 0 for the default variant.
  vehicle_anatomy: Cmd<
    { generationId: number; year: number; body: number; brand: string; model: string; powertrain: number },
    VehicleAnatomy
  >;
  decode_vin: Cmd<{ vin: string }, VinInfo>;
  match_vin: Cmd<{ vin: string }, CatalogMatch>;
  // VehicleIdentity (the name and picture for a VIN) is interface code and
  // lives in src/lib/vehicleIdentity.ts on top of these lookups.
}

export interface Events {
  log_line: { line: string };
  // Download of an update; total is 0 when the size is unknown.
  update_progress: { received: number; total: number };
  // rawCan: the connection runs over a raw CAN interface (J2534, SLCAN,
  // SocketCAN), so the KKL pages can use VAG TP 2.0 on it.
  // stnBuses: the extra CAN buses an OBDLink can switch to ("ms", "sw").
  // route: the way the connection went, for the garage.
  // j1939: connected to a truck over SAE J1939 (faults come from DM1 and
  // DM2, values by listening).
  connected: { adapter: string; transport: string; protocol: string; rawCan?: boolean; stnBuses?: string[]; route?: ConnectRoute; j1939?: boolean };
  // SAE J1939: the control units with their lamps (after reading faults),
  // their identification (after read_details) and the broadcast values
  // (each tick of the live loop).
  j1939_units: { units: J1939UnitRow[] };
  j1939_ids: { units: J1939IdRow[] };
  j1939_values: { rows: J1939ValueRow[] };
  // help: the checklist of the connection assistant when connecting
  // failed (absent for other failures and for recordings).
  failed: { error: string; help?: ConnectHelp };
  // The automatic search found the adapter on this port.
  adapter_found: { device: string };
  ble_adapter_found: { id: string; name: string; kinds: number };
  ble_scan_finished: { devices: BleDeviceRow[]; error: string };
  closed: None;
  busy: { active: boolean };
  faults: { rows: FaultRow[] };
  status: { row: StatusRow };
  live_value: { row: LiveRow };
  vehicle_info: { vin: string; freezeCode: string; freezeRows: LiveRow[] };
  supported_pids: { pids: number[] };
  raw_reply: { reply: string };
  kkl_connected: { connected: boolean; identification: string };
  kkl_message: { message: string; isError: boolean };
  kkl_protocol: { protocolName: string; isKwp2000: boolean };
  basic_setting_values: { values: LiveRow[] };
  scan_started: { total: number };
  // address: the K-line address of the unit being asked (the X-ray view
  // sends a packet to it).
  scan_progress: { ecuName: string; index: number; total: number; address: number };
  scan_ecu_result: { row: EcuScanRow };
  scan_finished: { present: number; withFaults: number; cancelled: boolean };
  clear_progress: { line: string };
  clear_finished: { everythingGone: boolean; remaining: number };
  kkl_test_step: { step: KklTestStep };
  kkl_test_finished: { report: KklTestReport };
  adaptation_result: { channel: number; value: number; shown: LiveRow[] };
  actuator_result: { actuator: string };
  monitor_results: { rows: MonitorRow[]; note: string };
  oxygen_tests: { rows: OxygenRow[]; note: string };
  identity: { vin: string; calibration: string; checksum: string; ecuName: string };
  health_progress: { step: string; percent: number };
  health_result: { row: HealthRow };
  used_car_progress: { step: string; percent: number };
  used_car_result: { row: UsedCarRow };
  adapter_voltage: { volts: number; time: number };
  adapter_report: { row: AdapterRow };
  recording_state: { active: boolean; path: string; exchanges: number; error: string };
  custom_read: { ok: number; failed: number; firstError: string };
  custom_imported: { rows: CustomRow[]; errors: string[] };
  freeze_frames: { frames: FreezeFrameRow[] };
  uds_scan_started: { total: number; generic: boolean; bus?: string };
  // request and kind of the unit being asked right now.
  uds_scan_progress: { name: string; index: number; total: number; request: number; kind: EcuKind };
  uds_ecu_result: { row: UdsEcuRow };
  uds_scan_finished: { present: number; withFaults: number; cancelled: boolean; note: string };
  uds_clear_result: { name: string; ok: boolean; message: string };
  // BMW on the K-line: index counts every address and protocol of the
  // plan; error says why the scan could not run at all.
  bmw_scan_started: { total: number };
  bmw_scan_progress: { index: number; total: number; name: string; protocol: string; address: number };
  bmw_unit: { row: BmwUnitRow };
  bmw_scan_finished: { present: number; withFaults: number; cancelled: boolean; error: string };
  // remaining: entries still there after reading back.
  bmw_clear_result: { address: number; protocol: string; ok: boolean; remaining: number; error: string };
  // Command line (main.cpp): the page of --page once a connection has
  // settled, and --report asking the interface for the document.
  cli_page: { page: string };
  cli_report: { path: string };
  // A panic in the backend while the window is still there (the worker
  // thread); the report is already on the computer, nothing was sent.
  crash_report: CrashReport;
}

export type CommandName = keyof Commands;
export type EventName = keyof Events;
export type ArgsOf<K extends CommandName> = Commands[K]["args"];
export type ResultOf<K extends CommandName> = Commands[K]["result"];

export type Unlisten = () => void;

// What both implementations (tauri.ts and mock/) provide.
export interface Backend {
  readonly kind: "tauri" | "mock";
  call<K extends CommandName>(command: K, ...args: keyof ArgsOf<K> extends never ? [] : [ArgsOf<K>]): Promise<ResultOf<K>>;
  on<E extends EventName>(event: E, handler: (payload: Events[E]) => void): Unlisten;
}
