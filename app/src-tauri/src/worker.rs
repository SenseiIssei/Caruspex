// Link between the interface and the diagnostic core (Session.cpp).
//
// One thread owns the ELM327 and runs the jobs the commands put into its
// channel, one after another, like the Qt Worker in its own thread. Results
// go out as events through a Sink. The timers of the Qt version (live loop,
// basic settings, voltage) are deadlines checked between two jobs, so a stop
// request always gets through before the next tick.
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use caruspex_advice::connect_help::{ConnectFacts, LinkKind, connect_help};
use caruspex_analysis::common::{FuelType, Level};
use caruspex_analysis::drive_cycle_guide::drive_cycle_guide;
use caruspex_analysis::health_check::{self, HealthInput};
use caruspex_analysis::inputs as ai;
use caruspex_analysis::readiness::readiness_from_status;
use caruspex_analysis::used_car::{
    BatteryFacts, Drive, FaultKind, HealthSummary, MileageReading, UsedCarFault, UsedCarInput,
    UsedCarReport, VinFacts, drive_from_fuel_type, evaluate_used_car, normalise_vin,
};
use caruspex_data::dtc::lookup_dtc;
use caruspex_data::ecu_addresses::{
    EcuDefinition, bmw_address_sweep, ecu_definitions, ecu_from_ids, generic_ecu_sweep,
};
use caruspex_data::pids::PidCategory;
use caruspex_data::severity::Severity;
use caruspex_demo::DemoTransport;
use caruspex_i18n::l;
use caruspex_link::adapter_probe::version_text;
use caruspex_link::elm327::DEFAULT_SEND_TIMEOUT_MS;
use caruspex_link::timing::sleep_ms;
use caruspex_link::{
    AdaptiveTiming, Elm327, KLineStream, SerialTransport, TcpTransport, Transport, enumerate_ports,
    probe_adapter,
};
use caruspex_net::{
    CanElm, KLineKind, SharedCanLink, SlcanLink, Tp20, open_can_interface, open_j2534_kline,
    open_network_car, probe_slcan,
};
use caruspex_protocols::kwp1281::{ecu_name, known_ecus};
use caruspex_protocols::{
    Ecu, Fault, KLineProtocol, LiveValue, MeasuringValue, Obd2, Uds, UdsEcu, VagDiagnosis,
    VagKLine, VagTp20, VehicleStatus, scan_ecus,
};
use caruspex_record::{
    CustomPid, CustomPidReader, ReplayPacing, ReplayTransport, USER_KEY_BASE, active_recording,
    built_in_packs, obdb_pid, odometer_values, pack_category, pack_title, pack_values_en,
    packs_for_model, parse_torque_csv, recording_summary, start_recording, stop_recording,
};
use caruspex_vehicles::catalog;
use caruspex_vehicles::vin::{decode_vin, system_year};
use serde::Serialize;
use serde_json::{Value, json};

use crate::rows::*;
use crate::settings::AppSettings;
use crate::texts::{args, w};

/// Seconds since the program started, monotonic (appSeconds()).
pub fn app_seconds() -> f64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

/// The port choice of the KKL pages for TP 2.0 on the connected CAN interface.
pub const TP20_DEVICE: &str = "tp20";
/// The port choice of the KKL pages for the K-line of a J2534 device:
/// this prefix and the driver.
pub const J2534_DEVICE_PREFIX: &str = "j2534:";
/// Keep-alive of an open KKL or TP 2.0 session.
const KKL_KEEP_ALIVE_MS: u64 = 1000;

/// Where the worker's events go: the Tauri window, or a list in the tests.
pub trait Sink: Send + Sync + 'static {
    fn emit(&self, event: &str, payload: Value);
    /// Every live value, before it goes out as an event (the trip computer
    /// listens here).
    fn live(&self, _row: &LiveRow) {}
}

pub enum Job {
    OpenSerial {
        device: String,
        baud: i32,
        protocol_order: String,
    },
    OpenTcp {
        host: String,
        port: i32,
        protocol_order: String,
    },
    OpenDemo,
    OpenEnet {
        host: String,
    },
    OpenCan {
        spec: String,
    },
    ScanBle {
        milliseconds: i32,
    },
    OpenBle {
        id: String,
        name: String,
        kinds: i32,
        protocol_order: String,
    },
    OpenReplay {
        path: String,
        real_time: bool,
        protocol_order: String,
    },
    /// Any ready transport; the tests hand in a DemoTransport without delays.
    OpenTransport {
        transport: Box<dyn Transport>,
        protocol_order: String,
    },
    Close,
    StartRecording(String),
    StopRecording,
    ReadFaults,
    ClearFaults,
    ReadDetails,
    StartLive(Vec<i32>),
    StopLive,
    SendRaw(String),
    ReadMonitors,
    HealthCheck(i32),
    /// The VIN from the papers, may be empty.
    UsedCarCheck(String),
    ProbeAdapter,
    BruteForceClear(i32),
    StartVoltagePoll(i32),
    StopVoltagePoll,
    ReadCustom(Vec<i32>),
    StartCustomPoll(Vec<i32>),
    ImportCustomCsv(String),
    ScanUds(String),
    ClearUdsEcu {
        request: u32,
        response: u32,
        extended29: bool,
        name: String,
    },
    OpenKkl {
        device: String,
        ecu_address: i32,
    },
    CloseKkl,
    StartBasicSetting(i32),
    StopBasicSetting,
    SetBasicSettingSession(i32),
    ScanAllEcus {
        device: String,
        clear_attempts: i32,
    },
    ReadAdaptation(i32),
    ActuatorTest,
    ApplySettings(AppSettings),
    Shutdown,
}

/// The commands' end of the worker.
#[derive(Clone)]
pub struct WorkerHandle {
    tx: Sender<Job>,
    cancel: Arc<AtomicBool>,
}

impl WorkerHandle {
    pub fn send(&self, job: Job) {
        let _ = self.tx.send(job);
    }
    /// Cancels a running scan. Allowed from any thread; the scan blocks the
    /// worker's loop, so a job would not get through.
    pub fn cancel_scan(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

/// Starts the worker thread.
pub fn spawn(sink: Arc<dyn Sink>, settings: AppSettings) -> WorkerHandle {
    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let mut worker = Worker::new(sink, cancel.clone());
    worker.apply_settings(settings);
    std::thread::Builder::new()
        .name("caruspex-worker".into())
        .spawn(move || worker.run(rx))
        .expect("worker thread starts");
    WorkerHandle { tx, cancel }
}

#[derive(Default)]
struct Timer {
    active: bool,
    interval_ms: u64,
    next: Option<Instant>,
}

impl Timer {
    fn start(&mut self) {
        self.active = true;
        self.next = Some(Instant::now() + Duration::from_millis(self.interval_ms));
    }
    fn stop(&mut self) {
        self.active = false;
        self.next = None;
    }
    fn set_interval(&mut self, ms: u64) {
        self.interval_ms = ms;
    }
    fn due(&self, now: Instant) -> bool {
        self.active && self.next.is_some_and(|n| n <= now)
    }
    fn rearm(&mut self) {
        if self.active {
            self.next = Some(Instant::now() + Duration::from_millis(self.interval_ms));
        }
    }
}

fn to_row(v: &LiveValue) -> LiveRow {
    // The key tells the values of one PID apart; it equals the PID for the
    // primary value, so everything keyed by PID keeps working.
    let pid = if v.key != 0 { v.key } else { v.pid };
    LiveRow {
        pid,
        name: crate::knowledge::live_name(pid, &v.name),
        unit: v.unit.clone(),
        text: v.text.clone(),
        value: v.value,
        warn: v.warn,
        time: app_seconds(),
        display_min: v.display_min,
        display_max: v.display_max,
    }
}

// The core reports the kind of a fault as display text in the active
// language. "stored" is the English wording the core uses for "gespeichert".
fn is_stored_kind(kind: &str) -> bool {
    kind == "gespeichert" || kind == "stored" || kind == l("gespeichert", "stored")
}

// Same for "dauerhaft", the permanent codes of service 0A.
fn is_permanent_kind(kind: &str) -> bool {
    kind == "dauerhaft" || kind == "permanent" || kind == l("dauerhaft", "permanent")
}

fn fault_row(f: &Fault) -> FaultRow {
    FaultRow {
        code: f.code.clone(),
        kind: f.kind.clone(),
        text: f.text.clone(),
        throttle_related: f.throttle_related,
        stored: is_stored_kind(&f.kind),
        permanent: is_permanent_kind(&f.kind),
        ecu: f.ecu.clone(),
    }
}

fn status_row(st: &VehicleStatus) -> StatusRow {
    StatusRow {
        valid: st.valid,
        mil: st.mil_on,
        fault_count: st.fault_count,
        monitor_names: st.readiness.iter().map(|i| i.name.clone()).collect(),
        monitor_ready: st.readiness.iter().map(|i| i.ready).collect(),
    }
}

fn uds_row(ecu: &UdsEcu) -> UdsEcuRow {
    UdsEcuRow {
        request: ecu.definition.request,
        response: ecu.definition.response,
        extended29: ecu.definition.extended29,
        kind: ecu.definition.kind as i32,
        name: ecu.definition.name.clone(),
        present: ecu.present,
        dtcs_read: ecu.dtcs_read,
        part_number: ecu.info.part_number.clone(),
        software_version: ecu.info.software_version.clone(),
        hardware_version: ecu.info.hardware_version.clone(),
        system_name: ecu.info.system_name.clone(),
        serial: ecu.info.serial.clone(),
        dtcs: ecu
            .dtcs
            .iter()
            .map(|d| UdsDtcRow {
                code: d.code.clone(),
                failure_text: d.failure_text.clone(),
                text: d.text.clone(),
                test_failed: d.test_failed,
                pending: d.pending,
                confirmed: d.confirmed,
                warning_lamp: d.warning_lamp,
            })
            .collect(),
        note: ecu.note.clone(),
    }
}

/// The fields of a measuring block as rows "Feld 1: ...". An invalid field
/// keeps its number, so the field numbers match the ones of the control unit.
fn measured_rows(values: &[MeasuringValue]) -> Vec<LiveRow> {
    let mut rows = Vec::new();
    for (i, m) in values.iter().enumerate() {
        let index = i as i32 + 1;
        if !m.valid {
            continue;
        }
        rows.push(LiveRow {
            pid: index,
            name: args(&w("Feld %1: %2", "Field %1: %2"), &[&index, &m.label]),
            unit: m.unit.clone(),
            value: m.value,
            ..LiveRow::default()
        });
    }
    rows
}

fn analysis_fault(f: &Fault) -> ai::Fault {
    ai::Fault {
        code: f.code.clone(),
        kind: f.kind.clone(),
        text: f.text.clone(),
        throttle_related: f.throttle_related,
    }
}

fn analysis_status(st: &VehicleStatus) -> ai::VehicleStatus {
    ai::VehicleStatus {
        mil_on: st.mil_on,
        fault_count: st.fault_count,
        readiness: st
            .readiness
            .iter()
            .map(|i| ai::ReadinessItem {
                name: i.name.clone(),
                ready: i.ready,
            })
            .collect(),
        valid: st.valid,
        compression_ignition: st.compression_ignition,
    }
}

/// How urgent a fault code is, from the database. Pending codes count one
/// step lower, they may still disappear on their own.
fn dtc_level(code: &str, stored: bool) -> Level {
    let severity = lookup_dtc(code, "").severity;
    let mut level = Level::Info;
    if severity == Severity::Critical {
        level = Level::Critical;
    } else if severity == Severity::High || severity == Severity::Medium {
        level = Level::Warning;
    }
    if !stored && level == Level::Warning {
        level = Level::Info;
    }
    level
}

/// What the VIN says about the car, the packs of manufacturer values that
/// fit it and the drive the catalog knows for sure.
fn used_car_identity(vin: &str) -> (Option<VinFacts>, Vec<&'static str>, Drive) {
    let vin = vin.trim();
    if vin.is_empty() {
        return (None, Vec::new(), Drive::Unknown);
    }
    let info = decode_vin(vin, 0);
    let mut facts = VinFacts {
        vin: info.vin.clone(),
        valid: info.valid,
        check_digit_applies: info.check_digit_applies,
        check_digit_ok: info.check_digit_ok,
        brand: info.brand.clone(),
        model_year: info.model_year,
        ..VinFacts::default()
    };
    // The demo car has a made-up maker code; its pack is the demo pack.
    if info.vin == DemoTransport::vin() {
        facts.vehicle = l("Caruspex Demo-Auto", "Caruspex demo car").to_string();
        facts.model_year = 2021;
        facts.year_from = 2021;
        return (Some(facts), vec!["caruspex-demo"], Drive::Unknown);
    }
    let found = catalog::match_vin(&info);
    if facts.brand.is_empty() {
        facts.brand = found.brand.clone();
    }
    let mut packs = Vec::new();
    let mut drive = Drive::Unknown;
    if let Some(g) = found.generation {
        facts.vehicle = catalog::display_name(g);
        facts.year_from = g.year_from;
        facts.year_to = g.year_to;
        if found.year > 0 {
            facts.model_year = found.year;
        }
        packs = packs_for_model(g.brand, g.model);
        if g.powertrains == catalog::PT_ELECTRIC {
            drive = Drive::Electric;
        }
    }
    (Some(facts), packs, drive)
}

/// Seconds of live values for the health check inside the used car check.
const USED_CAR_LIVE_SECONDS: i32 = 10;

fn used_car_row(input: &UsedCarInput, report: &UsedCarReport) -> UsedCarRow {
    let count = |kind: FaultKind| -> i32 {
        if input.faults_read {
            input.faults.iter().filter(|f| f.kind == kind).count() as i32
        } else {
            -1
        }
    };
    let supported: Vec<_> = input.readiness.iter().filter(|m| m.supported).collect();
    let battery = input.battery.as_ref();
    UsedCarRow {
        light: report.light as i32,
        headline: report.headline.text(),
        summary: report.summary.text(),
        vin: input
            .car_vin
            .as_ref()
            .map(|v| v.vin.clone())
            .unwrap_or_default(),
        papers_vin: normalise_vin(&input.papers_vin),
        vehicle: input
            .car_vin
            .as_ref()
            .map(|v| {
                if v.vehicle.is_empty() {
                    v.brand.clone()
                } else {
                    v.vehicle.clone()
                }
            })
            .unwrap_or_default(),
        model_year: input.car_vin.as_ref().map_or(0, |v| v.model_year),
        mileage: report
            .mileage
            .iter()
            .map(|m| UsedCarMileageRow {
                source: m.source.clone(),
                km: m.km,
            })
            .collect(),
        distance_since_clear_km: input.distance_since_clear_km.unwrap_or(-1.0),
        minutes_since_clear: input.minutes_since_clear.unwrap_or(-1.0),
        warm_ups: input.warm_ups.unwrap_or(-1.0),
        readiness_complete: if input.readiness_read {
            supported.iter().filter(|m| m.complete).count() as i32
        } else {
            -1
        },
        readiness_total: if input.readiness_read {
            supported.len() as i32
        } else {
            -1
        },
        stored_faults: count(FaultKind::Stored),
        pending_faults: count(FaultKind::Pending),
        permanent_faults: count(FaultKind::Permanent),
        health_score: input.health.as_ref().map_or(-1, |h| h.score),
        electric: input.drive.electric(),
        battery_pack: battery.map(|b| b.pack.clone()).unwrap_or_default(),
        battery_soh: battery.and_then(|b| b.soh).unwrap_or(-1.0),
        battery_soc: battery.and_then(|b| b.soc).unwrap_or(-1.0),
        items: report
            .items
            .iter()
            .map(|i| UsedCarItemRow {
                id: i.id.to_string(),
                section: i.section.key().to_string(),
                level: i.level as i32,
                title: i.title.text(),
                explanation: i.explanation.text(),
                ask: i.ask.text(),
            })
            .collect(),
        questions: report.questions().iter().map(|q| q.text()).collect(),
        not_checked: report
            .not_checked
            .iter()
            .map(|n| format!("{}: {}", n.title.text(), n.reason.text()))
            .collect(),
    }
}

fn analysis_monitor(r: &caruspex_protocols::MonitorResult) -> ai::MonitorResult {
    ai::MonitorResult {
        test_id: r.test_id,
        component_id: r.component_id,
        name: r.name.clone(),
        component: r.component.clone(),
        value: r.value,
        minimum: r.minimum,
        maximum: r.maximum,
        unit: r.unit.clone(),
        has_minimum: r.has_minimum,
        has_maximum: r.has_maximum,
        passed: r.passed,
    }
}

pub struct Worker {
    sink: Arc<dyn Sink>,
    cancel: Arc<AtomicBool>,
    elm: Elm327,
    // The Qt worker kept an Obd2 next to the Elm327 while connected. Obd2
    // borrows the driver here, so only its batch memory is kept.
    obd_ready: bool,
    batch_refused: BTreeSet<i32>,
    poll_timer: Timer,
    poll_pids: Vec<i32>,
    poll_index: usize,
    custom_poll_keys: Vec<i32>,
    custom_turn: bool,
    user_pids: Vec<CustomPid>,
    /// The VAG control unit session of the KKL pages: K-line (KKL cable or
    /// J2534) or TP 2.0 on the CAN interface of the connection.
    kwp: Option<Box<dyn VagDiagnosis>>,
    /// The session runs on `can_link` (TP 2.0).
    kkl_on_can: bool,
    /// The raw CAN interface of the current connection, shared with its ELM
    /// front end, for TP 2.0.
    can_link: Option<SharedCanLink>,
    kkl_keep_alive_timer: Timer,
    basic_setting_timer: Timer,
    basic_setting_group: i32,
    voltage_timer: Timer,
    settings: AppSettings,
    /// What the running connection attempt found out, for the connection
    /// assistant (caruspex_advice::connect_help).
    facts: ConnectFacts,
    /// Seconds of live values in the used car check (shorter in tests).
    used_car_live_seconds: i32,
}

impl Worker {
    pub fn new(sink: Arc<dyn Sink>, cancel: Arc<AtomicBool>) -> Self {
        Self {
            sink,
            cancel,
            elm: Elm327::new(),
            obd_ready: false,
            batch_refused: BTreeSet::new(),
            poll_timer: Timer::default(),
            poll_pids: Vec::new(),
            poll_index: 0,
            custom_poll_keys: Vec::new(),
            custom_turn: false,
            user_pids: Vec::new(),
            kwp: None,
            kkl_on_can: false,
            can_link: None,
            kkl_keep_alive_timer: Timer::default(),
            basic_setting_timer: Timer::default(),
            basic_setting_group: 60,
            voltage_timer: Timer::default(),
            settings: AppSettings::default(),
            facts: ConnectFacts::default(),
            used_car_live_seconds: USED_CAR_LIVE_SECONDS,
        }
    }

    // --- Plumbing -----------------------------------------------------------

    fn emit(&self, event: &str, payload: impl Serialize) {
        self.sink
            .emit(event, serde_json::to_value(payload).unwrap_or(Value::Null));
    }
    fn log(&self, line: impl Into<String>) {
        self.emit("log_line", json!({ "line": line.into() }));
    }
    fn busy(&self, active: bool) {
        self.emit("busy", json!({ "active": active }));
    }
    fn failed(&self, error: impl Into<String>) {
        self.emit("failed", json!({ "error": error.into() }));
    }
    /// A connection attempt starts: forget what the last one found out.
    fn begin_connect(&mut self, kind: LinkKind) {
        self.forget_can_link();
        self.facts = ConnectFacts {
            kind,
            ..ConnectFacts::default()
        };
    }
    /// Connecting failed: the error line and the checklist of the
    /// connection assistant, built from what the attempt found out.
    fn connect_failed(&self, error: impl Into<String>) {
        let help = crate::advice::help_json(&connect_help(&self.facts));
        self.emit("failed", json!({ "error": error.into(), "help": help }));
    }
    /// A port or interface did not open. Windows reports a port held by
    /// another program with its own sentence (serial.rs).
    fn port_failed(&mut self, error: &str) {
        self.facts.port_opened = Some(false);
        self.facts.port_busy = error.contains(l(
            ": belegt ein anderes Programm den Port?",
            ": is another program using the port?",
        ));
    }
    fn emit_live(&self, row: LiveRow) {
        self.sink.live(&row);
        self.emit("live_value", json!({ "row": row }));
    }
    fn kkl_message(&self, message: impl Into<String>, is_error: bool) {
        self.emit(
            "kkl_message",
            json!({ "message": message.into(), "isError": is_error }),
        );
    }
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    fn set_elm_log(&mut self) {
        let sink = self.sink.clone();
        self.elm.set_log(Box::new(move |m: &str| {
            sink.emit("log_line", json!({ "line": m }));
        }));
    }

    /// Runs `f` with an Obd2 on the driver, keeping its batch memory.
    fn with_obd<R>(&mut self, f: impl FnOnce(&mut Obd2<'_>) -> R) -> R {
        let refused = std::mem::take(&mut self.batch_refused);
        let mut obd = Obd2::with_batch_memory(&mut self.elm, refused);
        let result = f(&mut obd);
        self.batch_refused = obd.batch_refused_pids().clone();
        result
    }

    // --- The loop -------------------------------------------------------------

    fn next_deadline(&self) -> Option<Instant> {
        [
            &self.poll_timer,
            &self.basic_setting_timer,
            &self.voltage_timer,
            &self.kkl_keep_alive_timer,
        ]
        .iter()
        .filter(|t| t.active)
        .filter_map(|t| t.next)
        .min()
    }

    pub fn run(mut self, rx: Receiver<Job>) {
        loop {
            let job = match self.next_deadline() {
                None => match rx.recv() {
                    Ok(job) => Some(job),
                    Err(_) => break,
                },
                Some(deadline) => {
                    let wait = deadline.saturating_duration_since(Instant::now());
                    match rx.recv_timeout(wait) {
                        Ok(job) => Some(job),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            };
            match job {
                Some(Job::Shutdown) => break,
                Some(job) => self.handle(job),
                None => self.run_due_timers(),
            }
        }
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.stop_live();
        self.elm.disconnect();
        if let Some(mut kwp) = self.kwp.take() {
            kwp.disconnect();
        }
    }

    /// Runs every timer whose time has come (one tick each).
    pub fn run_due_timers(&mut self) {
        let now = Instant::now();
        if self.poll_timer.due(now) {
            self.poll_tick();
            self.poll_timer.rearm();
        }
        if self.basic_setting_timer.due(now) {
            self.basic_setting_tick();
            self.basic_setting_timer.rearm();
        }
        if self.voltage_timer.due(now) {
            self.voltage_tick();
            self.voltage_timer.rearm();
        }
        if self.kkl_keep_alive_timer.due(now) {
            self.kkl_keep_alive_tick();
            self.kkl_keep_alive_timer.rearm();
        }
    }

    pub fn handle(&mut self, job: Job) {
        match job {
            Job::OpenSerial {
                device,
                baud,
                protocol_order,
            } => self.open_serial(&device, baud, &protocol_order),
            Job::OpenTcp {
                host,
                port,
                protocol_order,
            } => self.open_tcp(&host, port, &protocol_order),
            Job::OpenDemo => self.open_demo(),
            Job::OpenEnet { host } => self.open_enet(&host),
            Job::OpenCan { spec } => self.open_can(&spec),
            Job::ScanBle { milliseconds } => self.scan_ble(milliseconds),
            Job::OpenBle {
                id,
                name,
                kinds,
                protocol_order,
            } => self.open_ble(&id, &name, kinds, &protocol_order),
            Job::OpenReplay {
                path,
                real_time,
                protocol_order,
            } => self.open_replay(&path, real_time, &protocol_order),
            Job::OpenTransport {
                transport,
                protocol_order,
            } => {
                self.busy(true);
                self.set_elm_log();
                self.begin_connect(LinkKind::Usb);
                let ok = self.finish_connect(transport, &protocol_order, true);
                self.busy(false);
                if ok {
                    self.refresh_status_and_faults();
                }
            }
            Job::Close => self.close_adapter(),
            Job::StartRecording(path) => self.start_recording(&path),
            Job::StopRecording => self.stop_recording(),
            Job::ReadFaults => self.refresh_status_and_faults(),
            Job::ClearFaults => self.clear_faults(),
            Job::ReadDetails => self.read_details(),
            Job::StartLive(pids) => self.start_live(pids),
            Job::StopLive => self.stop_live(),
            Job::SendRaw(command) => self.send_raw(&command),
            Job::ReadMonitors => self.read_monitors(),
            Job::HealthCheck(seconds) => self.run_health_check(seconds),
            Job::UsedCarCheck(papers_vin) => self.run_used_car_check(&papers_vin),
            Job::ProbeAdapter => self.probe_adapter(),
            Job::BruteForceClear(attempts) => self.brute_force_clear(attempts),
            Job::StartVoltagePoll(ms) => self.start_voltage_poll(ms),
            Job::StopVoltagePoll => self.voltage_timer.stop(),
            Job::ReadCustom(keys) => self.read_custom(&keys),
            Job::StartCustomPoll(keys) => self.start_custom_poll(keys),
            Job::ImportCustomCsv(path) => self.import_custom_csv(&path),
            Job::ScanUds(family) => self.scan_uds(&family),
            Job::ClearUdsEcu {
                request,
                response,
                extended29,
                name,
            } => self.clear_uds_ecu(request, response, extended29, &name),
            Job::OpenKkl {
                device,
                ecu_address,
            } => self.open_kkl(&device, ecu_address),
            Job::CloseKkl => self.close_kkl(),
            Job::StartBasicSetting(group) => self.start_basic_setting(group),
            Job::StopBasicSetting => self.stop_basic_setting(),
            Job::SetBasicSettingSession(mode) => self.set_basic_setting_session(mode),
            Job::ScanAllEcus {
                device,
                clear_attempts,
            } => self.scan_all_ecus(&device, clear_attempts),
            Job::ReadAdaptation(channel) => self.read_adaptation_channel(channel),
            Job::ActuatorTest => self.run_actuator_test(),
            Job::ApplySettings(values) => self.apply_settings(values),
            Job::Shutdown => self.shutdown(),
        }
    }

    // --- Connecting -----------------------------------------------------------

    /// Hands a ready transport to the ELM327 driver and reports the result.
    fn finish_connect(
        &mut self,
        transport: Box<dyn Transport>,
        protocol_order: &str,
        report_failure: bool,
    ) -> bool {
        match self.try_connect(transport, protocol_order) {
            Ok(()) => true,
            Err(error) => {
                if report_failure {
                    self.connect_failed(error);
                } else {
                    self.log(error);
                }
                false
            }
        }
    }

    /// Hands a ready transport to the ELM327 driver; on success reports the
    /// connection, on failure only notes what the driver found out.
    fn try_connect(
        &mut self,
        transport: Box<dyn Transport>,
        protocol_order: &str,
    ) -> Result<(), String> {
        self.facts.port_opened = Some(true);
        self.facts.port_busy = false;
        let result = self.elm.connect(transport, protocol_order);
        // When several ports or rates are tried, the attempt where the
        // adapter answered says the most; a silent one does not replace it.
        let trace = self.elm.connect_trace().clone();
        if trace.adapter_answered || self.facts.adapter_answered != Some(true) {
            self.facts.adapter_answered = Some(trace.adapter_answered);
            self.facts.adapter_id = trace.adapter_id;
            self.facts.protocols_tried = trace.protocols_tried;
            self.facts.bus_error = trace.bus_error;
        }
        if let Err(error) = result {
            // Let go of the port, so the next baud rate (or the SLCAN probe)
            // can open it again.
            self.elm.disconnect();
            return Err(error);
        }
        // On CAN several control units may answer; with headers on, every
        // fault keeps the unit it came from.
        if self.elm.is_can() {
            self.elm.set_headers(true);
        }
        self.obd_ready = true;
        self.batch_refused.clear();
        self.emit(
            "connected",
            json!({
                "adapter": self.elm.version(),
                "transport": self.elm.transport_name(),
                "protocol": self.elm.protocol_text(),
                "rawCan": self.can_link.is_some(),
            }),
        );
        Ok(())
    }

    fn open_serial(&mut self, device: &str, baud: i32, protocol_order: &str) {
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Usb);

        // An empty device means: look for the adapter on every port. Only
        // the usual rates are tried there, otherwise a computer with many
        // ports would keep the user waiting for minutes.
        let searching = device.is_empty();
        let devices: Vec<String> = if searching {
            let list: Vec<String> = enumerate_ports().into_iter().map(|p| p.device).collect();
            self.facts.ports_found = Some(list.len());
            if list.is_empty() {
                self.busy(false);
                self.connect_failed(w(
                    "Kein Anschluss gefunden. Steckt der Adapter am Rechner, und ist ein Bluetooth-Adapter gekoppelt?",
                    "No port found. Is the adapter plugged into the computer, and is the Bluetooth adapter paired?",
                ));
                return;
            }
            list
        } else {
            vec![device.to_string()]
        };
        let bauds: Vec<u32> = if baud > 0 {
            vec![baud as u32]
        } else if searching {
            vec![38400, 115200, 9600]
        } else {
            vec![38400, 9600, 115200, 500000]
        };

        let mut error = String::new();
        let mut any_opened = false;
        for candidate_device in &devices {
            if searching {
                self.log(args(
                    &w("Probiere %1 ...", "Trying %1 ..."),
                    &[candidate_device],
                ));
            }
            for (i, &b) in bauds.iter().enumerate() {
                let mut candidate = SerialTransport::new();
                if let Err(e) = candidate.open(candidate_device, b) {
                    // While searching, ports that do not open (paired
                    // Bluetooth ports out of reach) say nothing about the
                    // adapter; one that opened before counts.
                    if !(searching && any_opened) {
                        self.port_failed(&e);
                    }
                    error = e;
                    break;
                }
                any_opened = true;
                if !self.facts.bauds_tried.contains(&b) {
                    self.facts.bauds_tried.push(b);
                }
                self.log(args(
                    &w("Anschluss %1 offen bei %2 Baud", "Port %1 open at %2 baud"),
                    &[candidate_device, &b],
                ));
                match self.try_connect(Box::new(candidate), protocol_order) {
                    Ok(()) => {
                        if searching {
                            self.emit("adapter_found", json!({ "device": candidate_device }));
                        }
                        self.busy(false);
                        self.refresh_status_and_faults();
                        return;
                    }
                    Err(e) => {
                        // The adapter understood this rate, the car is the
                        // problem: other rates will not help. While
                        // searching, the next port is tried.
                        let stop = searching || self.elm.connect_trace().adapter_answered;
                        // The last error of a chosen port goes out with the
                        // failure, the others only into the log.
                        if searching || (!stop && i + 1 < bauds.len()) {
                            self.log(e.clone());
                        }
                        error = e;
                        if stop {
                            break;
                        }
                    }
                }
            }
            if searching {
                // No ELM327 here; perhaps a CAN stick with the SLCAN protocol.
                let mut serial = SerialTransport::new();
                if serial.open(candidate_device, 115200).is_ok()
                    && probe_slcan(&mut serial).is_some()
                {
                    self.log(args(
                        &w("SLCAN-Adapter an %1 gefunden", "SLCAN adapter found on %1"),
                        &[candidate_device],
                    ));
                    let mut link = SlcanLink::new(Box::new(serial));
                    if link.open(500000).is_ok() {
                        let front = CanElm::new(
                            Box::new(link),
                            format!("SLCAN {candidate_device}"),
                            500000,
                        );
                        self.can_link = Some(front.shared_link());
                        if self.finish_connect(Box::new(front), "67", false) {
                            self.emit("adapter_found", json!({ "device": candidate_device }));
                            self.busy(false);
                            self.refresh_status_and_faults();
                            return;
                        }
                        self.can_link = None;
                    }
                }
            }
            if !searching {
                self.busy(false);
                self.connect_failed(error);
                return;
            }
        }
        self.busy(false);
        self.connect_failed(if searching {
            w(
                "An keinem Anschluss hat ein ELM327 geantwortet. Zündung an und Adapter richtig eingesteckt?",
                "No ELM327 answered on any port. Is the ignition on and the adapter properly plugged in?",
            )
        } else {
            error
        });
    }

    fn open_tcp(&mut self, host: &str, port: i32, protocol_order: &str) {
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Wifi);
        let mut transport = TcpTransport::new();
        if let Err(error) = transport.open(host, port.clamp(0, 65535) as u16) {
            self.busy(false);
            self.port_failed(&error);
            self.connect_failed(error);
            return;
        }
        let ok = self.finish_connect(Box::new(transport), protocol_order, true);
        self.busy(false);
        if ok {
            self.refresh_status_and_faults();
        }
    }

    fn scan_ble(&mut self, milliseconds: i32) {
        self.busy(true);
        let (devices, error) = ble::scan(milliseconds);
        self.busy(false);
        self.emit(
            "ble_scan_finished",
            json!({ "devices": devices, "error": error }),
        );
    }

    fn open_ble(&mut self, id: &str, name: &str, kinds: i32, protocol_order: &str) {
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Bluetooth);
        let candidates = if id.is_empty() {
            self.log(w(
                "Suche Bluetooth-Adapter ...",
                "Searching for Bluetooth adapters ...",
            ));
            match ble::candidates() {
                Ok(list) if !list.is_empty() => {
                    self.facts.device_found = Some(true);
                    list
                }
                Ok(_) => {
                    self.busy(false);
                    self.facts.device_found = Some(false);
                    self.connect_failed(w(
                        "Kein Bluetooth-Adapter in der Nähe gefunden. Steckt er im Auto, und ist die Zündung an?",
                        "No Bluetooth adapter found nearby. Is it plugged into the car, and is the ignition on?",
                    ));
                    return;
                }
                Err(error) => {
                    self.busy(false);
                    self.connect_failed(error);
                    return;
                }
            }
        } else {
            vec![ble::device(id, name, kinds)]
        };

        let mut error = String::new();
        for d in candidates {
            let shown = if d.name.is_empty() {
                d.id.clone()
            } else {
                d.name.clone()
            };
            // Bluetooth LE first, a dual-mode adapter then also gets the
            // classic serial port as a second try.
            let mut transport: Option<Box<dyn Transport>> = None;
            if d.low_energy {
                self.log(args(
                    &w(
                        "Verbinde mit %1 über Bluetooth LE ...",
                        "Connecting to %1 over Bluetooth LE ...",
                    ),
                    &[&shown],
                ));
                match ble::open(&d) {
                    Ok((t, profile)) => {
                        self.log(args(
                            &w("Kanal gefunden (%1)", "Channel found (%1)"),
                            &[&profile],
                        ));
                        transport = Some(t);
                    }
                    Err(e) => {
                        self.log(e.clone());
                        error = e;
                    }
                }
            }
            if transport.is_none() && d.classic {
                self.log(args(
                    &w(
                        "Verbinde mit %1 über klassisches Bluetooth ...",
                        "Connecting to %1 over classic Bluetooth ...",
                    ),
                    &[&shown],
                ));
                match ble::open_classic(&d) {
                    Ok(t) => transport = Some(t),
                    Err(e) => {
                        self.log(e.clone());
                        error = e;
                    }
                }
            }
            let Some(transport) = transport else {
                if self.facts.port_opened.is_none() {
                    self.facts.port_opened = Some(false);
                }
                continue;
            };
            match self.try_connect(transport, protocol_order) {
                Ok(()) => {
                    self.emit(
                        "ble_adapter_found",
                        json!({ "id": d.id, "name": d.name, "kinds": ble::kinds(&d) }),
                    );
                    self.busy(false);
                    self.refresh_status_and_faults();
                    return;
                }
                Err(e) => {
                    self.log(e.clone());
                    error = e;
                }
            }
        }
        self.busy(false);
        self.connect_failed(error);
    }

    fn open_enet(&mut self, host: &str) {
        // ENET is always CAN behind the front end.
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Network);
        if host.is_empty() {
            self.log(w(
                "Suche das Auto im Netzwerk (ENET und DoIP) ...",
                "Searching for the car on the network (ENET and DoIP) ...",
            ));
        }
        let car = match open_network_car(host) {
            Ok(car) => car,
            Err(error) => {
                self.busy(false);
                self.facts.port_opened = Some(false);
                self.connect_failed(error);
                return;
            }
        };
        let vin = if car.vin.is_empty() {
            String::new()
        } else {
            format!(", VIN {}", car.vin)
        };
        self.log(args(
            &w("Verbunden über %1: %2%3", "Connected over %1: %2%3"),
            &[&car.kind, &car.transport.name(), &vin],
        ));
        let ok = self.finish_connect(Box::new(car.transport), "6", true);
        self.busy(false);
        if ok {
            self.refresh_status_and_faults();
        }
    }

    fn open_can(&mut self, spec: &str) {
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Can);
        // "j2534:<dll>@500000": the bit rate follows the last @.
        self.facts.can_bit_rate = spec
            .rsplit_once('@')
            .and_then(|(_, rate)| rate.trim().parse().ok())
            .unwrap_or(500_000);
        let opener = |device: &str, baud: u32| -> Result<Box<dyn Transport>, String> {
            let mut serial = SerialTransport::new();
            serial.open(device, baud)?;
            Ok(Box::new(serial))
        };
        let transport = match open_can_interface(spec, &opener) {
            Ok(t) => t,
            Err(error) => {
                self.busy(false);
                self.port_failed(&error);
                self.connect_failed(error);
                return;
            }
        };
        self.log(args(
            &w("CAN-Schnittstelle offen: %1", "CAN interface open: %1"),
            &[&transport.name()],
        ));
        // TP 2.0 for older VAG cars uses the same interface.
        self.can_link = Some(transport.shared_link());
        // 11 bit first, then 29 bit; a car that answers neither is listened to.
        let ok = self.finish_connect(Box::new(transport), "67", true);
        if !ok {
            self.can_link = None;
        }
        self.busy(false);
        if ok {
            self.refresh_status_and_faults();
        }
    }

    fn open_demo(&mut self) {
        // The demo car speaks CAN; the K-line protocols would only cost seconds.
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Usb);
        let ok = self.finish_connect(Box::new(caruspex_demo::DemoTransport::default()), "6", true);
        self.busy(false);
        if ok {
            self.refresh_status_and_faults();
        }
    }

    fn open_replay(&mut self, path: &str, real_time: bool, protocol_order: &str) {
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Replay);
        let replay = ReplayTransport::new(
            path,
            if real_time {
                ReplayPacing::RealTime
            } else {
                ReplayPacing::Fast
            },
        );
        if !replay.is_open() {
            self.busy(false);
            self.failed(args(
                &w(
                    "Aufnahme lässt sich nicht öffnen: %1",
                    "The recording cannot be opened: %1",
                ),
                &[&replay.error()],
            ));
            return;
        }
        self.log(recording_summary(&replay.info()));
        let order = replay.protocol_order(protocol_order);
        let ok = self.finish_connect(Box::new(replay), &order, true);
        self.busy(false);
        if ok {
            self.refresh_status_and_faults();
        }
    }

    fn start_recording(&mut self, path: &str) {
        if !self.elm.connected() {
            self.emit(
                "recording_state",
                json!({ "active": false, "path": path, "exchanges": 0,
                        "error": w("Erst verbinden, dann aufzeichnen.", "Connect first, then record.") }),
            );
            return;
        }
        match start_recording(&mut self.elm, path) {
            Ok(_) => self.emit(
                "recording_state",
                json!({ "active": true, "path": path, "exchanges": 0, "error": "" }),
            ),
            Err(error) => self.emit(
                "recording_state",
                json!({ "active": false, "path": path, "exchanges": 0, "error": error }),
            ),
        }
    }

    fn stop_recording(&mut self) {
        let (exchanges, path) = active_recording(&self.elm)
            .map(|h| (h.exchanges(), h.path().to_string()))
            .unwrap_or_default();
        stop_recording(&mut self.elm);
        self.emit(
            "recording_state",
            json!({ "active": false, "path": path, "exchanges": exchanges, "error": "" }),
        );
    }

    fn close_adapter(&mut self) {
        self.stop_live();
        self.forget_can_link();
        self.elm.disconnect();
        self.obd_ready = false;
        self.emit("closed", json!({}));
    }

    // --- Reading --------------------------------------------------------------

    fn refresh_status_and_faults(&mut self) {
        if !self.obd_ready {
            return;
        }
        self.busy(true);
        let st = self.with_obd(|o| o.read_status());
        self.emit("status", json!({ "row": status_row(&st) }));

        let rows: Vec<FaultRow> = self
            .with_obd(|o| o.read_faults())
            .unwrap_or_default()
            .iter()
            .map(fault_row)
            .collect();
        let count = rows.len();
        self.emit("faults", json!({ "rows": rows }));
        self.log(if count == 0 {
            w(
                "Fehlerspeicher gelesen: kein Eintrag.",
                "Fault memory read: no entries.",
            )
        } else {
            args(
                &w(
                    "Fehlerspeicher gelesen: %1 Einträge.",
                    "Fault memory read: %1 entries.",
                ),
                &[&count],
            )
        });

        let supported: Vec<i32> = self.with_obd(|o| o.supported_pids()).into_iter().collect();
        self.emit("supported_pids", json!({ "pids": supported }));
        self.busy(false);
    }

    fn clear_faults(&mut self) {
        if !self.obd_ready {
            return;
        }
        self.busy(true);
        let result = self.with_obd(|o| o.clear_faults());
        self.log(match result {
            Ok(()) => w("Fehlerspeicher gelöscht.", "Fault memory cleared."),
            Err(error) => args(
                &w("Löschen fehlgeschlagen: %1", "Clearing failed: %1"),
                &[&error],
            ),
        });
        self.busy(false);
        self.refresh_status_and_faults();
    }

    fn read_details(&mut self) {
        if !self.obd_ready {
            return;
        }
        self.busy(true);
        let vin = self.with_obd(|o| o.read_vin());
        // All freeze frames; the first one also goes out the old way for the
        // pages that only know a single frame.
        let frames: Vec<FreezeFrameRow> = self
            .with_obd(|o| o.read_freeze_frames(caruspex_protocols::obd2::DEFAULT_FREEZE_FRAMES))
            .iter()
            .map(|f| FreezeFrameRow {
                frame: f.frame,
                dtc: f.dtc.clone(),
                ecu: f.ecu.clone(),
                values: f.values.iter().map(to_row).collect(),
            })
            .collect();
        let (code, rows) = frames
            .first()
            .map(|f| (f.dtc.clone(), f.values.clone()))
            .unwrap_or_default();
        self.emit(
            "vehicle_info",
            json!({ "vin": vin, "freezeCode": code, "freezeRows": rows }),
        );
        self.emit("freeze_frames", json!({ "frames": frames }));

        // One reading of the values the start page shows as tiles, so they
        // are not empty until a measurement runs.
        let supported = self.with_obd(|o| o.supported_pids());
        for pid in [0x05, 0x2F, 0x42, 0xA6] {
            if !supported.is_empty() && !supported.contains(&pid) {
                continue;
            }
            if let Some(value) = self.with_obd(|o| o.read_live(pid)) {
                self.emit_live(to_row(&value));
            }
        }
        self.busy(false);
    }

    fn start_live(&mut self, pids: Vec<i32>) {
        self.poll_pids = pids;
        if !self.obd_ready || self.poll_pids.is_empty() {
            return;
        }
        self.poll_index = 0;
        // No faster than the configured interval. On the K-Line a request
        // takes longer anyway, but a fast CAN adapter or the simulator would
        // otherwise flood the interface with events.
        self.poll_timer
            .set_interval(self.settings.poll_interval_ms.max(0) as u64);
        self.poll_timer.start();
        self.log(args(
            &w(
                "Messung gestartet, %1 Werte im Umlauf.",
                "Measurement started, cycling through %1 values.",
            ),
            &[&self.poll_pids.len()],
        ));
    }

    fn stop_live(&mut self) {
        self.custom_poll_keys.clear();
        if self.poll_timer.active {
            self.poll_timer.stop();
            self.log(w("Messung gestoppt.", "Measurement stopped."));
        }
    }

    // One slice per tick. That keeps the loop free and stop requests from
    // the interface get through immediately.
    fn poll_tick(&mut self) {
        if !self.obd_ready {
            return;
        }
        // Every other tick belongs to the manufacturer values when some are
        // selected. They change slowly and cost an addressing round each time.
        if !self.custom_poll_keys.is_empty() {
            self.custom_turn = !self.custom_turn;
            if self.custom_turn || self.poll_pids.is_empty() {
                let keys = self.custom_poll_keys.clone();
                self.read_custom_now(&keys);
                return;
            }
        }
        if self.poll_pids.is_empty() {
            return;
        }
        // Up to six PIDs per tick: on CAN they go out as one request,
        // elsewhere the core reads them one by one.
        let mut slice = Vec::new();
        let n = self.poll_pids.len();
        for _ in 0..6.min(n) {
            slice.push(self.poll_pids[self.poll_index % n]);
            self.poll_index = (self.poll_index + 1) % n;
        }
        for value in self.with_obd(|o| o.read_live_batch(&slice, false)) {
            self.emit_live(to_row(&value));
        }
    }

    fn send_raw(&mut self, command: &str) {
        let result = self.elm.send(command, 8000);
        let reply = if result.lines.is_empty() {
            w("(keine Antwort)", "(no reply)")
        } else {
            result.lines.join("\n")
        };
        self.emit("raw_reply", json!({ "reply": reply }));
    }

    fn read_monitors(&mut self) {
        if !self.obd_ready {
            self.emit(
                "monitor_results",
                json!({ "rows": [], "note": w("Nicht verbunden.", "Not connected.") }),
            );
            return;
        }
        self.busy(true);
        let id = self.with_obd(|o| o.read_identity());
        self.emit(
            "identity",
            json!({ "vin": id.vin, "calibration": id.calibration_id,
                    "checksum": id.calibration_verification, "ecuName": id.ecu_name }),
        );

        let (results, note) = match self.with_obd(|o| o.read_monitor_results()) {
            Ok(list) => (list, String::new()),
            Err(note) => (Vec::new(), note),
        };
        let rows: Vec<MonitorRow> = results
            .iter()
            .map(|r| MonitorRow {
                name: r.name.clone(),
                component: r.component.clone(),
                value: r.value,
                minimum: r.minimum,
                maximum: r.maximum,
                unit: r.unit.clone(),
                has_minimum: r.has_minimum,
                has_maximum: r.has_maximum,
                passed: r.passed,
            })
            .collect();
        let count = rows.len();
        self.emit("monitor_results", json!({ "rows": rows, "note": note }));

        let (tests, oxygen_note) = match self.with_obd(|o| o.read_oxygen_sensor_tests()) {
            Ok(list) => (list, String::new()),
            Err(note) => (Vec::new(), note),
        };
        let oxygen: Vec<OxygenRow> = tests
            .iter()
            .map(|t| OxygenRow {
                name: t.name.clone(),
                sensor: t.sensor,
                value: t.value,
                unit: t.unit.clone(),
            })
            .collect();
        let oxygen_count = oxygen.len();
        self.emit(
            "oxygen_tests",
            json!({ "rows": oxygen, "note": oxygen_note }),
        );

        self.busy(false);
        self.log(args(
            &w(
                "Bauteilüberwachung gelesen: %1 Tests, %2 Sondenprüfungen.",
                "Component monitoring read: %1 tests, %2 O2 sensor tests.",
            ),
            &[&count, &oxygen_count],
        ));
    }

    // Modes 06, 05 and 09 in one go: everything the control unit reveals
    // about itself and its running self-tests.
    fn run_health_check(&mut self, seconds: i32) {
        if !self.obd_ready {
            self.failed(w(
                "Für den Gesundheitscheck erst verbinden.",
                "Connect first to run the health check.",
            ));
            return;
        }
        self.stop_live();
        self.cancel.store(false, Ordering::SeqCst);
        self.busy(true);
        let mut input = HealthInput::default();
        let progress = |me: &Self, step: String, percent: i32| {
            me.emit(
                "health_progress",
                json!({ "step": step, "percent": percent }),
            );
        };

        progress(
            self,
            w(
                "Bereitschaft und Fehlerlampe",
                "Readiness and check engine light",
            ),
            4,
        );
        let st = self.with_obd(|o| o.read_status());
        input.set_status(&analysis_status(&st));
        self.emit("status", json!({ "row": status_row(&st) }));

        progress(self, w("Fehlerspeicher", "Fault memory"), 10);
        let read = self.with_obd(|o| o.read_faults());
        // "Not read" is a different finding from "no faults" (faultsRead =
        // error.empty() in Session.cpp).
        input.faults_read = read.is_ok();
        let faults = read.unwrap_or_default();
        input.faults = faults.iter().map(analysis_fault).collect();
        let rows: Vec<FaultRow> = faults.iter().map(fault_row).collect();
        self.emit("faults", json!({ "rows": rows }));
        input.fault_level = Some(Arc::new(|f: &ai::Fault| {
            dtc_level(&f.code, is_stored_kind(&f.kind))
        }));

        progress(
            self,
            w(
                "Bauteilprüfung (Modus 06)",
                "Component monitoring (mode 06)",
            ),
            18,
        );
        input.monitor_results = self
            .with_obd(|o| o.read_monitor_results())
            .unwrap_or_default()
            .iter()
            .map(analysis_monitor)
            .collect();
        input.monitor_results_read = true;

        if let Some(fuel) = self.with_obd(|o| o.read_live(0x51)) {
            input.fuel = health_check::fuel_type_from_obd(fuel.value as i32);
        }

        // Live series: only what the car offers.
        let supported = self.with_obd(|o| o.supported_pids());
        let pids: Vec<i32> = health_check::health_live_pids()
            .iter()
            .copied()
            .filter(|p| supported.is_empty() || supported.contains(p))
            .collect();
        let start = app_seconds();
        let total = seconds.max(1);
        while !pids.is_empty() && app_seconds() - start < f64::from(seconds) && !self.cancelled() {
            let values = self.with_obd(|o| o.read_live_batch(&pids, false));
            let t = app_seconds() - start;
            let snapshot: Vec<ai::LiveValue> = values
                .iter()
                .map(|v| ai::LiveValue {
                    pid: v.pid,
                    value: v.value,
                })
                .collect();
            input.add_snapshot(&snapshot, t);
            for value in &values {
                self.emit_live(to_row(value));
            }
            let elapsed = app_seconds() - start;
            let left = (seconds - elapsed as i32).max(0);
            let percent = 25 + (70.0 * elapsed / f64::from(total)) as i32;
            progress(
                self,
                args(
                    &w(
                        "Messwerte sammeln, noch %1 s",
                        "Collecting values, %1 s left",
                    ),
                    &[&left],
                ),
                percent.min(95),
            );
            if values.is_empty() {
                // Nothing answers (adapter gone): do not spin.
                sleep_ms(100);
            }
        }

        let report = health_check::run_health_check(&input);
        let mut row = HealthRow {
            score: report.score,
            grade: report.grade as i32,
            summary: report.summary.text(),
            checks_run: report.checks_run,
            diesel: input.fuel == FuelType::Diesel,
            ..HealthRow::default()
        };
        for f in &report.findings {
            row.findings.push(HealthFindingRow {
                id: f.id.clone(),
                level: f.level as i32,
                area: String::new(),
                title: f.title.text(),
                explanation: f.explanation.text(),
                advice: f.advice.text(),
            });
        }
        for skipped in &report.not_checked {
            row.not_checked.push(format!(
                "{}: {}",
                skipped.title.text(),
                skipped.reason.text()
            ));
        }
        for m in &input.readiness {
            if !m.supported {
                continue;
            }
            let guide = drive_cycle_guide(m.monitor);
            row.monitors.push(MonitorGuideRow {
                name: guide.name.get().to_string(),
                complete: m.complete,
                explanation: guide.explanation.get().to_string(),
                drive_pattern: guide.drive_pattern.get().to_string(),
                typical_time: guide.typical_time.get().to_string(),
                cold_start: guide.needs_cold_start,
            });
        }
        progress(self, w("Fertig", "Done"), 100);
        self.emit("health_result", json!({ "row": row }));
        self.busy(false);
    }

    // The used car check: the VIN against the catalog and the papers, the
    // odometer of every control unit with a documented identifier, when the
    // fault memory was last cleared, readiness, faults, a short health check
    // and the battery of electric cars, judged by analysis::used_car.
    fn run_used_car_check(&mut self, papers_vin: &str) {
        if !self.obd_ready {
            self.failed(
                l(
                    "Für die Gebrauchtwagen-Prüfung erst verbinden.",
                    "Connect first to check a used car.",
                )
                .to_string(),
            );
            return;
        }
        self.stop_live();
        self.cancel.store(false, Ordering::SeqCst);
        self.busy(true);
        let progress = |me: &Self, step: String, percent: i32| {
            me.emit(
                "used_car_progress",
                json!({ "step": step, "percent": percent }),
            );
        };
        let mut input = UsedCarInput {
            papers_vin: papers_vin.to_string(),
            current_year: system_year(),
            ..UsedCarInput::default()
        };

        progress(
            self,
            l("Fahrgestellnummer", "Vehicle identification number").to_string(),
            4,
        );
        let vin = self.with_obd(|o| o.read_vin());
        let (facts, packs, catalog_drive) = used_car_identity(&vin);
        input.car_vin = facts;
        input.drive = catalog_drive;

        progress(
            self,
            w(
                "Bereitschaft und Fehlerlampe",
                "Readiness and check engine light",
            ),
            10,
        );
        let st = self.with_obd(|o| o.read_status());
        self.emit("status", json!({ "row": status_row(&st) }));
        let status = analysis_status(&st);
        if st.valid {
            input.mil_on = Some(st.mil_on);
            input.readiness_read = true;
            input.readiness = readiness_from_status(&status);
        }

        progress(self, w("Fehlerspeicher", "Fault memory"), 16);
        let read = self.with_obd(|o| o.read_faults());
        input.faults_read = read.is_ok();
        let faults = read.unwrap_or_default();
        self.emit(
            "faults",
            json!({ "rows": faults.iter().map(fault_row).collect::<Vec<_>>() }),
        );
        let permanent = l("dauerhaft", "permanent");
        input.faults = faults
            .iter()
            .map(|f| {
                let stored = is_stored_kind(&f.kind);
                UsedCarFault {
                    code: f.code.clone(),
                    text: f.text.clone(),
                    kind: if stored {
                        FaultKind::Stored
                    } else if f.kind == permanent || f.kind == "permanent" {
                        FaultKind::Permanent
                    } else {
                        FaultKind::Pending
                    },
                    level: dtc_level(&f.code, stored),
                }
            })
            .collect();

        progress(
            self,
            l(
                "Kilometerstand und letztes Löschen",
                "Mileage and last clearing",
            )
            .to_string(),
            22,
        );
        let supported = self.with_obd(|o| o.supported_pids());
        let mut fuel_code = None;
        for pid in [0xA6, 0x31, 0x30, 0x4E, 0x21, 0x51] {
            if !supported.is_empty() && !supported.contains(&pid) {
                continue;
            }
            let Some(value) = self.with_obd(|o| o.read_live(pid)) else {
                continue;
            };
            self.emit_live(to_row(&value));
            let v = Some(value.value);
            match pid {
                0xA6 => input.odometer_km = v,
                0x31 => input.distance_since_clear_km = v,
                0x30 => input.warm_ups = v,
                0x4E => input.minutes_since_clear = v,
                0x21 => input.mil_distance_km = v,
                _ => fuel_code = Some(value.value as i32),
            }
        }
        if let Some(code) = fuel_code {
            let drive = drive_from_fuel_type(code);
            if drive != Drive::Unknown {
                input.drive = drive;
            }
        }

        // The odometer of other control units: only identifiers documented
        // in a pack that fits this car.
        let odometers: Vec<CustomPid> = packs.iter().flat_map(|p| odometer_values(p)).collect();
        if !odometers.is_empty() {
            progress(
                self,
                l(
                    "Kilometerstand in weiteren Steuergeräten",
                    "Mileage in other control units",
                )
                .to_string(),
                30,
            );
            let readings = {
                let mut reader = CustomPidReader::new(&mut self.elm);
                reader.read_all(&odometers)
            };
            for (pid, reading) in odometers.iter().zip(readings) {
                if reading.ok {
                    self.emit_live(to_row(&reading.value));
                    input.other_mileage.push(MileageReading {
                        source: pid.name.clone(),
                        km: reading.value.value,
                    });
                }
            }
        }

        // The traction battery, from the first pack with battery values.
        if input.drive.electric() {
            let battery_pack = packs
                .iter()
                .find(|p| pack_category(p) == PidCategory::HybridEv)
                .copied();
            if let Some(pack) = battery_pack {
                progress(self, l("Akku", "Battery").to_string(), 36);
                input.battery = Some(self.read_battery(pack));
            }
        }

        // A short health check along the way.
        let mut health = HealthInput::default();
        health.set_status(&status);
        health.faults_read = input.faults_read;
        health.faults = faults.iter().map(analysis_fault).collect();
        health.fault_level = Some(Arc::new(|f: &ai::Fault| {
            dtc_level(&f.code, is_stored_kind(&f.kind))
        }));
        progress(
            self,
            w(
                "Bauteilprüfung (Modus 06)",
                "Component monitoring (mode 06)",
            ),
            42,
        );
        health.monitor_results = self
            .with_obd(|o| o.read_monitor_results())
            .unwrap_or_default()
            .iter()
            .map(analysis_monitor)
            .collect();
        health.monitor_results_read = true;
        if let Some(code) = fuel_code {
            health.fuel = health_check::fuel_type_from_obd(code);
        }
        let pids: Vec<i32> = health_check::health_live_pids()
            .iter()
            .copied()
            .filter(|p| supported.is_empty() || supported.contains(p))
            .collect();
        let start = app_seconds();
        let live_seconds = self.used_car_live_seconds.max(1);
        let seconds = f64::from(live_seconds);
        while !pids.is_empty() && app_seconds() - start < seconds && !self.cancelled() {
            let values = self.with_obd(|o| o.read_live_batch(&pids, false));
            let t = app_seconds() - start;
            let snapshot: Vec<ai::LiveValue> = values
                .iter()
                .map(|v| ai::LiveValue {
                    pid: v.pid,
                    value: v.value,
                })
                .collect();
            health.add_snapshot(&snapshot, t);
            for value in &values {
                self.emit_live(to_row(value));
            }
            let elapsed = app_seconds() - start;
            let left = (live_seconds - elapsed as i32).max(0);
            progress(
                self,
                args(
                    &w(
                        "Messwerte sammeln, noch %1 s",
                        "Collecting values, %1 s left",
                    ),
                    &[&left],
                ),
                (48 + (47.0 * elapsed / seconds) as i32).min(95),
            );
            if values.is_empty() {
                sleep_ms(100);
            }
        }
        if !self.cancelled() {
            let report = health_check::run_health_check(&health);
            input.health = Some(HealthSummary {
                score: report.score,
                grade: report.grade,
                summary: report.summary.text(),
            });
        }

        let report = evaluate_used_car(&input);
        let row = used_car_row(&input, &report);
        progress(self, w("Fertig", "Done"), 100);
        self.emit("used_car_result", json!({ "row": row }));
        self.busy(false);
    }

    /// Health, charge and cell spread of the traction battery from a pack.
    fn read_battery(&mut self, pack: &str) -> BatteryFacts {
        const WANTED: [&str; 11] = [
            "SOH",
            "SOH LBC",
            "Full kWh",
            "BOL kWh",
            "SOC display",
            "SOC",
            "SOC BMS",
            "SOC LBC",
            "Cell V max",
            "Cell V min",
            "Cell V diff",
        ];
        let values: Vec<_> = pack_values_en(pack)
            .into_iter()
            .filter(|v| WANTED.contains(&v.short_en))
            .collect();
        let pids: Vec<CustomPid> = values.iter().map(|v| v.pid.clone()).collect();
        let readings = {
            let mut reader = CustomPidReader::new(&mut self.elm);
            reader.read_all(&pids)
        };
        let mut facts = BatteryFacts {
            pack: pack_title(pack),
            ..BatteryFacts::default()
        };
        let mut got = std::collections::HashMap::new();
        for (v, reading) in values.iter().zip(readings) {
            if reading.ok {
                self.emit_live(to_row(&reading.value));
                got.insert(v.short_en, reading.value.value);
            } else if facts.error.is_empty() {
                facts.error = reading.error;
            }
        }
        facts.read = !got.is_empty();
        let get = |key: &str| got.get(key).copied();
        facts.soh = get("SOH").or_else(|| get("SOH LBC")).or_else(|| {
            match (get("Full kWh"), get("BOL kWh")) {
                (Some(full), Some(new)) if new > 1.0 => Some(100.0 * full / new),
                _ => None,
            }
        });
        facts.soc = get("SOC display")
            .or_else(|| get("SOC"))
            .or_else(|| get("SOC BMS"))
            .or_else(|| get("SOC LBC"));
        facts.cell_spread_v =
            get("Cell V diff").or_else(|| match (get("Cell V max"), get("Cell V min")) {
                (Some(max), Some(min)) => Some(max - min),
                _ => None,
            });
        facts
    }

    fn probe_adapter(&mut self) {
        if !self.elm.connected() {
            self.failed(w(
                "Für den Adaptertest erst verbinden.",
                "Connect first to run the adapter test.",
            ));
            return;
        }
        self.stop_live();
        self.voltage_timer.stop();
        self.busy(true);
        let report = probe_adapter(&mut self.elm);
        let row = AdapterRow {
            reachable: report.reachable,
            version: report.version_text.clone(),
            description: report.description.clone(),
            is_stn: report.is_stn,
            stn_id: report.stn_id.clone(),
            voltage: report.voltage,
            has_voltage: report.has_voltage,
            round_trip_ms: report.round_trip_ms,
            round_trip_with_hint_ms: report.round_trip_with_hint_ms,
            behaves_like: if report.behaves_like_level > 0 {
                version_text(report.behaves_like_level)
            } else {
                String::new()
            },
            accepts_unknown: report.accepts_unknown_commands,
            verdict: report.verdict.get(),
            facts: report.facts.iter().map(|f| f.get()).collect(),
            speed_tips: report.speed_tips.iter().map(|t| t.get()).collect(),
            checks: report
                .checks
                .iter()
                .map(|c| AdapterCheckRow {
                    command: c.command.clone(),
                    version: version_text(c.introduced_in),
                    supported: c.supported,
                    purpose: c.purpose.get(),
                })
                .collect(),
        };
        self.emit("adapter_report", json!({ "row": row }));
        self.busy(false);
    }

    // Persistent clear over EOBD. Some control units only accept the command
    // right after the connection is established, others need several
    // attempts. That is why the connection is re-established in between.
    fn brute_force_clear(&mut self, attempts: i32) {
        if !self.obd_ready {
            self.emit(
                "clear_progress",
                json!({ "line": w("Nicht verbunden.", "Not connected.") }),
            );
            self.emit(
                "clear_finished",
                json!({ "everythingGone": false, "remaining": 0 }),
            );
            return;
        }
        self.busy(true);
        let mut remaining = 0usize;
        for attempt in 1..=attempts {
            let line = |s: String| json!({ "line": s });
            self.emit(
                "clear_progress",
                line(args(
                    &w("Versuch %1 von %2 ...", "Attempt %1 of %2 ..."),
                    &[&attempt, &attempts],
                )),
            );
            if let Err(error) = self.with_obd(|o| o.clear_faults()) {
                self.emit(
                    "clear_progress",
                    line(args(
                        &w(
                            "   Steuergerät quittiert nicht: %1",
                            "   Control unit does not acknowledge: %1",
                        ),
                        &[&error],
                    )),
                );
            }
            sleep_ms(600);

            let left = self.with_obd(|o| o.read_faults()).unwrap_or_default();
            remaining = left.len();
            if left.is_empty() {
                self.emit(
                    "clear_progress",
                    line(w("   Speicher ist leer.", "   Memory is empty.")),
                );
                self.busy(false);
                self.emit(
                    "clear_finished",
                    json!({ "everythingGone": true, "remaining": 0 }),
                );
                self.refresh_status_and_faults();
                return;
            }
            self.emit(
                "clear_progress",
                line(args(
                    &w("   noch %1 Einträge", "   %1 entries left"),
                    &[&remaining],
                )),
            );

            // From the second failure on, re-establish the connection. Some
            // control units only open the clear window right after waking up.
            if attempt >= 2 && attempt < attempts {
                self.emit(
                    "clear_progress",
                    line(w(
                        "   Verbindung wird neu aufgebaut ...",
                        "   Re-establishing the connection ...",
                    )),
                );
                let _ = self.elm.send("ATZ", 6000);
                sleep_ms(400);
                for command in ["ATE0", "ATL0", "ATS0", "ATH0"] {
                    self.elm.send(command, DEFAULT_SEND_TIMEOUT_MS);
                }
                self.elm.send("0100", 10000);
                sleep_ms(200);
            }
        }
        self.busy(false);
        self.emit(
            "clear_finished",
            json!({ "everythingGone": false, "remaining": remaining }),
        );
        self.refresh_status_and_faults();
    }

    fn start_voltage_poll(&mut self, interval_ms: i32) {
        if !self.elm.connected() {
            return;
        }
        self.stop_live();
        self.voltage_timer.set_interval(interval_ms.max(20) as u64);
        self.voltage_timer.start();
    }

    fn voltage_tick(&mut self) {
        let result = self.elm.send("ATRV", 1000);
        for line in &result.lines {
            let text: String = line.trim().chars().filter(|&c| c != 'V').collect();
            if let Ok(volts) = text.trim().parse::<f64>()
                && volts > 3.0
                && volts < 32.0
            {
                self.emit(
                    "adapter_voltage",
                    json!({ "volts": volts, "time": app_seconds() }),
                );
                break;
            }
        }
    }

    // --- Manufacturer and user values (CustomPids) ------------------------------

    fn find_custom(&self, key: i32) -> Option<CustomPid> {
        built_in_packs()
            .iter()
            .find(|p| p.key == key)
            .or_else(|| self.user_pids.iter().find(|p| p.key == key))
            .cloned()
            .or_else(|| obdb_pid(key))
    }

    /// Reads custom values and emits them; returns (ok, failed, first error).
    fn read_custom_now(&mut self, keys: &[i32]) -> (usize, usize, String) {
        let list: Vec<CustomPid> = keys.iter().filter_map(|&k| self.find_custom(k)).collect();
        let mut ok = 0;
        let mut bad = 0;
        let mut first_error = String::new();
        let readings = {
            // The reader restores the normal OBD addressing when it goes out
            // of scope, so the next OBD request works as before.
            let mut reader = CustomPidReader::new(&mut self.elm);
            reader.read_all(&list)
        };
        for r in readings {
            if r.ok {
                ok += 1;
                self.emit_live(to_row(&r.value));
            } else {
                bad += 1;
                if first_error.is_empty() {
                    first_error = r.error;
                }
            }
        }
        (ok, bad, first_error)
    }

    fn read_custom(&mut self, keys: &[i32]) {
        if !self.obd_ready {
            self.emit(
                "custom_read",
                json!({ "ok": 0, "failed": keys.len(), "firstError": w("Erst verbinden.", "Connect first.") }),
            );
            return;
        }
        self.busy(true);
        let (ok, failed, error) = self.read_custom_now(keys);
        self.emit(
            "custom_read",
            json!({ "ok": ok, "failed": failed, "firstError": error }),
        );
        self.busy(false);
    }

    fn start_custom_poll(&mut self, keys: Vec<i32>) {
        if !self.obd_ready {
            return;
        }
        self.custom_poll_keys = keys;
        self.poll_timer
            .set_interval(self.settings.poll_interval_ms.max(0) as u64);
        if !self.custom_poll_keys.is_empty() && !self.poll_timer.active {
            self.poll_timer.start();
        }
    }

    fn import_custom_csv(&mut self, path: &str) {
        let data = match std::fs::read(path) {
            Ok(data) => data,
            Err(e) => {
                self.emit(
                    "custom_imported",
                    json!({ "rows": [], "errors": [args(&w("Datei lässt sich nicht öffnen: %1", "The file cannot be opened: %1"), &[&e])] }),
                );
                return;
            }
        };
        // New imports get keys after the ones already loaded.
        let base = USER_KEY_BASE + self.user_pids.len() as i32;
        let (parsed, problems) = parse_torque_csv(&String::from_utf8_lossy(&data), base);
        let mut rows = Vec::new();
        for mut p in parsed {
            p.pack.clear();
            rows.push(CustomRow {
                key: p.key,
                name: p.name.clone(),
                short_name: p.short_name.clone(),
                unit: p.unit.clone(),
                pack: String::new(),
                source: p.source.clone(),
                request: p.request.clone(),
                header: p.header.clone(),
                short_name_en: String::new(),
            });
            self.user_pids.push(p);
        }
        self.emit(
            "custom_imported",
            json!({ "rows": rows, "errors": problems }),
        );
    }

    // --- UDS ---------------------------------------------------------------------

    fn scan_uds(&mut self, family: &str) {
        let finished = |me: &Self,
                        present: usize,
                        with_faults: usize,
                        cancelled: bool,
                        note: String| {
            me.emit(
                "uds_scan_finished",
                json!({ "present": present, "withFaults": with_faults, "cancelled": cancelled, "note": note }),
            );
        };
        if !self.obd_ready {
            finished(self, 0, 0, false, w("Erst verbinden.", "Connect first."));
            return;
        }
        if !self.elm.is_can() {
            finished(
                self,
                0,
                0,
                false,
                w(
                    "Das Auto spricht kein CAN. UDS gibt es erst bei CAN-Autos, bei älteren VW und Audi gehen die Steuergeräte über das KKL-Kabel.",
                    "This car does not speak CAN. UDS only exists on CAN cars; on older VW and Audi the control units go through the KKL cable.",
                ),
            );
            return;
        }
        self.stop_live();
        self.voltage_timer.stop();
        self.cancel.store(false, Ordering::SeqCst);
        self.busy(true);

        // "BMW*" is the full sweep of the BMW diagnostic addresses.
        let mut list: Vec<EcuDefinition> = if family.is_empty() {
            generic_ecu_sweep()
        } else if family == "BMW*" {
            bmw_address_sweep()
        } else {
            ecu_definitions(family)
        };
        let generic = family.is_empty() || list.is_empty();
        if list.is_empty() {
            list = generic_ecu_sweep();
        }
        let total = list.len();
        self.emit(
            "uds_scan_started",
            json!({ "total": total, "generic": generic }),
        );

        // One unit at a time, so every answer reaches the page right away.
        let mut present = 0;
        let mut with_faults = 0;
        for (index, def) in list.iter().enumerate() {
            if self.cancelled() {
                break;
            }
            self.emit(
                "uds_scan_progress",
                json!({
                    "name": def.name,
                    "index": index,
                    "total": total,
                    "request": def.request,
                    "kind": def.kind as i32,
                }),
            );
            let cancel = self.cancel.clone();
            let is_cancelled = move || cancel.load(Ordering::SeqCst);
            let result = scan_ecus(
                &mut self.elm,
                std::slice::from_ref(def),
                None,
                Some(&is_cancelled),
            );
            let Some(ecu) = result.first().filter(|e| e.present) else {
                continue;
            };
            present += 1;
            if !ecu.dtcs.is_empty() {
                with_faults += 1;
            }
            self.emit("uds_ecu_result", json!({ "row": uds_row(ecu) }));
        }
        finished(self, present, with_faults, self.cancelled(), String::new());
        self.busy(false);
    }

    fn clear_uds_ecu(&mut self, request: u32, response: u32, extended29: bool, name: &str) {
        if !self.obd_ready {
            return;
        }
        self.stop_live();
        self.busy(true);
        let def = ecu_from_ids(request, response, extended29, name);
        let result = {
            let mut uds = Uds::new(&mut self.elm);
            uds.select(&def).and_then(|_| uds.clear_dtcs())
        };
        let message = match &result {
            Ok(()) => args(
                &w(
                    "Fehlerspeicher von %1 gelöscht.",
                    "Fault memory of %1 cleared.",
                ),
                &[&name],
            ),
            Err(error) => args(
                &w(
                    "Löschen bei %1 hat nicht geklappt: %2",
                    "Clearing %1 did not work: %2",
                ),
                &[&name, error],
            ),
        };
        self.emit(
            "uds_clear_result",
            json!({ "name": name, "ok": result.is_ok(), "message": message }),
        );
        // Read that unit again so the page shows what is left.
        let never = || false;
        let again = scan_ecus(
            &mut self.elm,
            std::slice::from_ref(&def),
            None,
            Some(&never),
        );
        if let Some(ecu) = again.first().filter(|e| e.present) {
            let mut row = uds_row(ecu);
            row.name = name.to_string();
            self.emit("uds_ecu_result", json!({ "row": row }));
        }
        self.busy(false);
    }

    // --- KWP1281 over KKL ---------------------------------------------------------

    /// The VAG session for a choice of the port lists: a serial port with a
    /// KKL cable, "j2534:<driver>" for the K-line of a PassThru device, or
    /// "tp20" for TP 2.0 on the CAN interface of the current connection.
    fn open_vag_unit(
        &mut self,
        device: &str,
        log: Option<caruspex_protocols::KLineLog>,
    ) -> Result<Box<dyn VagDiagnosis>, String> {
        if device == TP20_DEVICE {
            let Some(shared) = self.can_link.clone() else {
                return Err(l(
                    "Für TP 2.0 erst über eine CAN-Schnittstelle (J2534, SLCAN, SocketCAN) mit dem Auto verbinden.",
                    "For TP 2.0, connect to the car through a CAN interface (J2534, SLCAN, SocketCAN) first.",
                )
                .to_string());
            };
            self.kkl_on_can = true;
            let tp20 = Tp20::new(Box::new(shared));
            self.log(args(
                l("TP 2.0 über %1", "TP 2.0 via %1"),
                &[&caruspex_link::MessageLink::name(&tp20)],
            ));
            return Ok(Box::new(VagTp20::new(tp20, log)));
        }
        self.kkl_on_can = false;
        if let Some(driver) = device.strip_prefix(J2534_DEVICE_PREFIX) {
            let (stream, name) = open_j2534_kline(driver, KLineKind::Iso9141, 9600)?;
            self.log(args(l("K-Line über %1", "K-line via %1"), &[&name]));
            return Ok(Box::new(VagKLine::new(stream, log)));
        }
        let mut stream = KLineStream::new();
        stream.open(device, 9600)?;
        Ok(Box::new(VagKLine::new(stream, log)))
    }

    fn open_kkl(&mut self, device: &str, ecu_address: i32) {
        self.close_kkl();
        self.busy(true);
        self.kkl_message(
            if device == TP20_DEVICE {
                l(
                    "Öffne den TP-2.0-Kanal zum Steuergerät ...",
                    "Opening the TP 2.0 channel to the control unit ...",
                )
                .to_string()
            } else {
                w(
                    "Wecke das Steuergerät mit der 5-Baud-Adresse, das dauert etwa drei Sekunden ...",
                    "Waking the control unit with the 5-baud address, this takes about three seconds ...",
                )
            },
            false,
        );

        let sink = self.sink.clone();
        let log: caruspex_protocols::KLineLog =
            Arc::new(move |m: &str| sink.emit("log_line", json!({ "line": m })));
        let mut kwp = match self.open_vag_unit(device, Some(log)) {
            Ok(kwp) => kwp,
            Err(error) => {
                self.kkl_on_can = false;
                self.busy(false);
                self.kkl_message(error, true);
                return;
            }
        };
        // The configured session type must be set before basic settings can
        // be started.
        kwp.set_basic_setting_session(self.settings.basic_setting_session.clamp(0, 255) as u8);
        let ecu = Ecu::from_address(ecu_address.clamp(0, 255) as u8);
        if let Err(error) = kwp.connect(ecu) {
            drop(kwp);
            self.kkl_on_can = false;
            self.busy(false);
            self.emit(
                "kkl_connected",
                json!({ "connected": false, "identification": "" }),
            );
            self.kkl_message(error, true);
            return;
        }

        let identification: Vec<String> = kwp
            .identification()
            .iter()
            .map(|l| l.trim().to_string())
            .collect();
        let protocol_name = kwp.protocol_name();
        let is_kwp2000 = kwp.protocol() == KLineProtocol::Kwp2000;
        self.kwp = Some(kwp);
        // Without traffic the control unit closes the session after a few
        // seconds; basic settings keep it busy on their own.
        self.kkl_keep_alive_timer.set_interval(KKL_KEEP_ALIVE_MS);
        self.kkl_keep_alive_timer.start();
        self.busy(false);
        self.emit(
            "kkl_connected",
            json!({ "connected": true, "identification": identification.join("  |  ") }),
        );
        self.emit(
            "kkl_protocol",
            json!({ "protocolName": protocol_name, "isKwp2000": is_kwp2000 }),
        );
        self.kkl_message(
            args(
                &w(
                    "Verbunden mit %1 über %2. Vor dem Start die Punkte oben prüfen.",
                    "Connected to %1 via %2. Check the points above before starting.",
                ),
                &[&ecu_name(ecu), &protocol_name],
            ),
            false,
        );
        self.log(args(
            &w("KWP1281 verbunden: %1", "KWP1281 connected: %1"),
            &[&identification.join(" / ")],
        ));
    }

    fn close_kkl(&mut self) {
        self.stop_basic_setting();
        self.kkl_keep_alive_timer.stop();
        // Signing off happens in disconnect, the port closes when it is dropped.
        if let Some(mut kwp) = self.kwp.take() {
            kwp.disconnect();
        }
        self.kkl_on_can = false;
        self.emit(
            "kkl_connected",
            json!({ "connected": false, "identification": "" }),
        );
    }

    fn kkl_connected(&self) -> bool {
        self.kwp.as_ref().is_some_and(|k| k.connected())
    }

    /// The CAN interface of the connection goes away: a TP 2.0 session on it
    /// ends first.
    fn forget_can_link(&mut self) {
        if self.kkl_on_can {
            self.close_kkl();
        }
        self.can_link = None;
    }

    fn kkl_keep_alive_tick(&mut self) {
        if self.basic_setting_timer.active {
            return; // the measuring block requests keep it open
        }
        if !self.kkl_connected() {
            self.kkl_keep_alive_timer.stop();
            return;
        }
        if let Some(Err(error)) = self.kwp.as_mut().map(|k| k.keep_alive()) {
            self.close_kkl();
            self.kkl_message(error, true);
        }
    }

    fn start_basic_setting(&mut self, group: i32) {
        if !self.kkl_connected() {
            self.kkl_message(
                w(
                    "Erst das KKL-Kabel verbinden.",
                    "Connect the KKL cable first.",
                ),
                true,
            );
            return;
        }
        self.basic_setting_group = group;
        // The control unit drops out of basic settings if nothing arrives for
        // about a second. We ask considerably more often.
        self.basic_setting_timer
            .set_interval(self.settings.basic_setting_interval_ms.max(0) as u64);
        self.basic_setting_timer.start();
        self.kkl_message(
            args(
                &w(
                    "Grundeinstellung im Messwertblock %1 läuft. Gaspedal nicht antippen.",
                    "Basic settings running in measuring block %1. Don't touch the accelerator pedal.",
                ),
                &[&group],
            ),
            false,
        );
        self.log(args(
            &w(
                "Grundeinstellung Block %1 gestartet",
                "Basic settings block %1 started",
            ),
            &[&group],
        ));
    }

    fn stop_basic_setting(&mut self) {
        if self.basic_setting_timer.active {
            self.basic_setting_timer.stop();
            self.kkl_message(
                w("Grundeinstellung beendet.", "Basic settings finished."),
                false,
            );
            self.log(w("Grundeinstellung beendet", "Basic settings finished"));
        }
    }

    fn basic_setting_tick(&mut self) {
        if !self.kkl_connected() {
            self.stop_basic_setting();
            return;
        }
        let group = self.basic_setting_group.clamp(0, 255) as u8;
        let result = self.kwp.as_mut().map(|k| k.basic_setting(group));
        match result {
            Some(Ok(measured)) => {
                let rows = measured_rows(&measured);
                self.emit("basic_setting_values", json!({ "values": rows }));
            }
            Some(Err(error)) => {
                self.stop_basic_setting();
                self.kkl_message(error, true);
            }
            None => self.stop_basic_setting(),
        }
    }

    // Searches all known control units one after another. Each attempt takes
    // a few seconds because of the 5-baud wake-up address; for control units
    // that are not fitted, the time spent waiting for a reply comes on top.
    fn scan_all_ecus(&mut self, device: &str, clear_attempts: i32) {
        self.close_kkl();
        self.cancel.store(false, Ordering::SeqCst);

        let mut probe = match self.open_vag_unit(device, None) {
            Ok(probe) => probe,
            Err(error) => {
                self.kkl_on_can = false;
                self.kkl_message(error, true);
                self.emit(
                    "scan_finished",
                    json!({ "present": 0, "withFaults": 0, "cancelled": false }),
                );
                return;
            }
        };

        let list = known_ecus();
        let total = list.len();
        self.emit("scan_started", json!({ "total": total }));
        self.busy(true);

        let mut present = 0i32;
        let mut with_faults = 0i32;
        for (i, &ecu) in list.iter().enumerate() {
            if self.cancelled() {
                break;
            }
            let index = i + 1;
            let mut row = EcuScanRow {
                address: i32::from(ecu.address()),
                name: ecu_name(ecu).to_string(),
                ..EcuScanRow::default()
            };
            self.emit(
                "scan_progress",
                json!({
                    "ecuName": row.name,
                    "index": index,
                    "total": total,
                    "address": row.address,
                }),
            );

            if probe.connect(ecu).is_err() {
                row.present = false;
                row.note = w(
                    "meldet sich nicht, vermutlich nicht verbaut",
                    "does not respond, probably not fitted",
                );
                self.emit("scan_ecu_result", json!({ "row": row }));
                continue;
            }

            row.present = true;
            present += 1;
            let identification: Vec<String> = probe
                .identification()
                .iter()
                .map(|l| l.trim().to_string())
                .collect();
            row.identification = format!(
                "[{}]  {}",
                probe.protocol_name(),
                identification.join("  |  ")
            );

            match probe.read_faults() {
                Ok(faults) => {
                    for f in &faults {
                        row.faults.push(FaultRow {
                            code: format!("{:05}", f.code),
                            kind: if f.sporadic {
                                w("sporadisch", "intermittent")
                            } else {
                                w("gespeichert", "stored")
                            },
                            text: f.text.clone(),
                            throttle_related: false,
                            stored: !f.sporadic,
                            permanent: false,
                            ecu: String::new(),
                        });
                    }
                    if !row.faults.is_empty() {
                        with_faults += 1;
                    }
                    row.note = if row.faults.is_empty() {
                        w("kein Eintrag", "no entries")
                    } else {
                        args(&w("%1 Einträge", "%1 entries"), &[&row.faults.len()])
                    };

                    // Persistent clear: read back after every attempt. Some
                    // control units only accept the clear command on the
                    // second try.
                    if clear_attempts > 0 && !faults.is_empty() {
                        let mut cleared = false;
                        let mut attempt = 1;
                        while attempt <= clear_attempts && !cleared {
                            self.emit(
                                "clear_progress",
                                json!({ "line": args(&w("%1: lösche, Versuch %2 von %3", "%1: clearing, attempt %2 of %3"), &[&row.name, &attempt, &clear_attempts]) }),
                            );
                            if let Err(error) = probe.clear_faults() {
                                self.emit(
                                    "clear_progress",
                                    json!({ "line": args(&w("   abgelehnt: %1", "   rejected: %1"), &[&error]) }),
                                );
                                sleep_ms(300);
                                attempt += 1;
                                continue;
                            }
                            sleep_ms(400);
                            if probe.read_faults().is_ok_and(|rest| rest.is_empty()) {
                                cleared = true;
                            }
                            attempt += 1;
                        }
                        if cleared {
                            row.faults.clear();
                            row.note = w("gelöscht", "cleared");
                            with_faults -= 1;
                            self.emit(
                                "clear_progress",
                                json!({ "line": args(&w("   %1: leer", "   %1: empty"), &[&row.name]) }),
                            );
                        } else {
                            row.note = args(
                                &w(
                                    "%1 Einträge, Löschen ohne Wirkung",
                                    "%1 entries, clearing had no effect",
                                ),
                                &[&row.faults.len()],
                            );
                            self.emit(
                                "clear_progress",
                                json!({ "line": args(&w("   %1: bleibt bestehen, die Ursache ist noch da", "   %1: remains, the cause is still present"), &[&row.name]) }),
                            );
                        }
                    }
                }
                Err(error) => row.note = error,
            }
            probe.disconnect();
            self.emit("scan_ecu_result", json!({ "row": row }));
        }

        drop(probe);
        self.kkl_on_can = false;
        self.busy(false);
        self.emit(
            "scan_finished",
            json!({ "present": present, "withFaults": with_faults, "cancelled": self.cancelled() }),
        );
    }

    fn read_adaptation_channel(&mut self, channel: i32) {
        if !self.kkl_connected() {
            self.kkl_message(
                w(
                    "Erst das KKL-Kabel verbinden.",
                    "Connect the KKL cable first.",
                ),
                true,
            );
            return;
        }
        self.busy(true);
        let result = self
            .kwp
            .as_mut()
            .map(|k| k.read_adaptation(channel.clamp(0, 255) as u8));
        match result {
            Some(Ok((value, shown))) => {
                let rows = measured_rows(&shown);
                self.busy(false);
                self.emit(
                    "adaptation_result",
                    json!({ "channel": channel, "value": value, "shown": rows }),
                );
                self.kkl_message(
                    args(
                        &w(
                            "Anpassungskanal %1 steht auf %2.",
                            "Adaptation channel %1 is set to %2.",
                        ),
                        &[&channel, &value],
                    ),
                    false,
                );
            }
            Some(Err(error)) => {
                self.busy(false);
                self.kkl_message(error, true);
            }
            None => self.busy(false),
        }
    }

    fn run_actuator_test(&mut self) {
        if !self.kkl_connected() {
            self.kkl_message(
                w(
                    "Erst das KKL-Kabel verbinden.",
                    "Connect the KKL cable first.",
                ),
                true,
            );
            return;
        }
        self.busy(true);
        let result = self.kwp.as_mut().map(|k| k.actuator_test());
        self.busy(false);
        match result {
            Some(Ok(actuator)) => self.emit("actuator_result", json!({ "actuator": actuator })),
            Some(Err(error)) => self.kkl_message(error, true),
            None => {}
        }
    }

    fn set_basic_setting_session(&mut self, mode: i32) {
        if let Some(kwp) = self.kwp.as_mut() {
            kwp.set_basic_setting_session(mode.clamp(0, 255) as u8);
        }
        self.log(args(
            &w(
                "Sitzungsart für die Grundeinstellung: 0x%1",
                "Session type for basic settings: 0x%1",
            ),
            &[&format!("{mode:02X}")],
        ));
    }

    /// Applies the settings. Called at startup and after every change; also
    /// takes effect during a running measurement.
    pub fn apply_settings(&mut self, values: AppSettings) {
        self.settings = values;
        // Update running timers right away, otherwise a change would only
        // take effect after the next connection.
        self.poll_timer
            .set_interval(self.settings.poll_interval_ms.max(0) as u64);
        self.basic_setting_timer
            .set_interval(self.settings.basic_setting_interval_ms.max(0) as u64);

        self.elm
            .set_timeout_scale(f64::from(self.settings.timeout_percent) / 100.0);
        self.elm
            .set_adaptive_timing(match self.settings.adaptive_timing {
                2 => AdaptiveTiming::Aggressive,
                1 => AdaptiveTiming::Auto,
                _ => AdaptiveTiming::Standard,
            });
        self.elm.set_verbose(self.settings.log_raw);
        if let Some(kwp) = self.kwp.as_mut() {
            kwp.set_basic_setting_session(self.settings.basic_setting_session.clamp(0, 255) as u8);
        }
    }

    #[cfg(test)]
    pub fn live_running(&self) -> bool {
        self.poll_timer.active
    }
}

/// Bluetooth LE and classic Bluetooth through caruspex-link, as the worker
/// needs it.
mod ble {
    use caruspex_link::Transport;
    use caruspex_link::ble::BleTransport;
    use caruspex_link::bluetooth::{BleDevice, bluetooth_kinds, scan_bluetooth_devices};
    use caruspex_link::bt_classic::RfcommTransport;

    use crate::rows::BleDeviceRow;

    /// LE and classic devices (paired or in range), OBD adapters first.
    pub fn scan(milliseconds: i32) -> (Vec<BleDeviceRow>, String) {
        match scan_bluetooth_devices(milliseconds.max(0) as u64) {
            Ok(devices) => (
                devices
                    .iter()
                    .map(|d| BleDeviceRow {
                        id: d.id.clone(),
                        name: d.name.clone(),
                        score: d.score,
                        kinds: bluetooth_kinds(d),
                    })
                    .collect(),
                String::new(),
            ),
            Err(error) => (Vec::new(), error),
        }
    }

    /// The devices of a six second search that look like OBD adapters.
    pub fn candidates() -> Result<Vec<BleDevice>, String> {
        Ok(scan_bluetooth_devices(6000)?
            .into_iter()
            .filter(|d| d.score >= 3)
            .collect())
    }

    pub fn device(id: &str, name: &str, kinds: i32) -> BleDevice {
        let mut d = BleDevice {
            id: id.to_string(),
            name: name.to_string(),
            low_energy: kinds & 1 != 0,
            classic: kinds & 2 != 0,
            ..BleDevice::default()
        };
        if !d.low_energy && !d.classic {
            d.low_energy = true;
        }
        d
    }

    pub fn open(d: &BleDevice) -> Result<(Box<dyn Transport>, String), String> {
        let transport = BleTransport::open(d)?;
        let profile = transport.profile().to_string();
        Ok((Box::new(transport), profile))
    }

    /// The serial port of a classic adapter (RFCOMM), without a COM port.
    pub fn open_classic(d: &BleDevice) -> Result<Box<dyn Transport>, String> {
        Ok(Box::new(RfcommTransport::open(d)?))
    }

    pub fn kinds(d: &BleDevice) -> i32 {
        bluetooth_kinds(d)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use caruspex_demo::{DemoOptions, DemoTransport};

    use super::*;

    #[derive(Default)]
    pub struct Collect(pub Mutex<Vec<(String, Value)>>);

    impl Sink for Collect {
        fn emit(&self, event: &str, payload: Value) {
            self.0.lock().unwrap().push((event.to_string(), payload));
        }
    }

    impl Collect {
        fn take(&self) -> Vec<(String, Value)> {
            std::mem::take(&mut *self.0.lock().unwrap())
        }
        fn named(events: &[(String, Value)], name: &str) -> Vec<Value> {
            events
                .iter()
                .filter(|(e, _)| e == name)
                .map(|(_, p)| p.clone())
                .collect()
        }
    }

    fn demo_worker() -> (Worker, Arc<Collect>) {
        let sink = Arc::new(Collect::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
        worker.apply_settings(AppSettings::default());
        let options = DemoOptions {
            latency_scale: 0.0,
            // The engine runs from the start, so the live values move.
            engine_start_seconds: 0.0,
            ..DemoOptions::default()
        };
        worker.handle(Job::OpenTransport {
            transport: Box::new(DemoTransport::new(options)),
            protocol_order: "6".into(),
        });
        (worker, sink)
    }

    #[test]
    fn connects_and_reads_status_and_faults() {
        let (_worker, sink) = demo_worker();
        let events = sink.take();
        let connected = Collect::named(&events, "connected");
        assert_eq!(connected.len(), 1, "{events:?}");
        assert!(!connected[0]["protocol"].as_str().unwrap().is_empty());
        let status = Collect::named(&events, "status");
        assert_eq!(status[0]["row"]["valid"], true);
        let faults = Collect::named(&events, "faults");
        let rows = faults[0]["rows"].as_array().unwrap();
        assert!(!rows.is_empty(), "the demo car has stored codes");
        assert!(rows.iter().any(|r| r["stored"] == true));
        assert!(
            !Collect::named(&events, "supported_pids")[0]["pids"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        // busy goes on and off in pairs.
        let busy = Collect::named(&events, "busy");
        assert_eq!(busy.last().unwrap()["active"], false);
    }

    #[test]
    fn details_bring_vin_and_freeze_frames() {
        let (mut worker, sink) = demo_worker();
        sink.take();
        worker.handle(Job::ReadDetails);
        let events = sink.take();
        let info = &Collect::named(&events, "vehicle_info")[0];
        assert_eq!(info["vin"], DemoTransport::vin());
        assert!(
            !Collect::named(&events, "freeze_frames")[0]["frames"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(!Collect::named(&events, "live_value").is_empty());
    }

    #[test]
    fn live_loop_reads_values_and_stops() {
        let (mut worker, sink) = demo_worker();
        sink.take();
        worker.handle(Job::StartLive(vec![0x0C, 0x0D, 0x05]));
        assert!(worker.live_running());
        for _ in 0..3 {
            std::thread::sleep(Duration::from_millis(25));
            worker.run_due_timers();
        }
        worker.handle(Job::StopLive);
        assert!(!worker.live_running());
        let events = sink.take();
        let values = Collect::named(&events, "live_value");
        assert!(values.iter().any(|v| v["row"]["pid"] == 0x0C), "{values:?}");
        assert!(
            values
                .iter()
                .all(|v| v["row"]["time"].as_f64().unwrap() > 0.0)
        );
    }

    #[test]
    fn clearing_empties_the_demo_memory() {
        let (mut worker, sink) = demo_worker();
        sink.take();
        worker.handle(Job::ClearFaults);
        let events = sink.take();
        let faults = Collect::named(&events, "faults");
        let stored = faults[0]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["stored"] == true)
            .count();
        assert_eq!(stored, 0);
    }

    #[test]
    fn health_check_reports_a_score() {
        let (mut worker, sink) = demo_worker();
        sink.take();
        worker.handle(Job::HealthCheck(1));
        let events = sink.take();
        let result = &Collect::named(&events, "health_result")[0]["row"];
        let score = result["score"].as_i64().unwrap();
        assert!((0..=100).contains(&score), "{result}");
        assert!(result["checksRun"].as_i64().unwrap() > 0);
        let progress = Collect::named(&events, "health_progress");
        assert_eq!(progress.last().unwrap()["percent"], 100);
    }

    fn items(row: &Value) -> Vec<String> {
        row["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["id"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn used_car_check_on_the_demo_car() {
        let (mut worker, sink) = demo_worker();
        worker.used_car_live_seconds = 1;
        sink.take();
        worker.handle(Job::UsedCarCheck("YCX DE15C7 MC048217".into()));
        let events = sink.take();
        let row = &Collect::named(&events, "used_car_result")[0]["row"];
        assert_eq!(row["vin"], DemoTransport::vin());
        assert_eq!(row["papersVin"], DemoTransport::vin());
        assert!(!row["vehicle"].as_str().unwrap().is_empty());
        assert_eq!(row["modelYear"], 2021);
        // PID A6, the cluster and the ABS, and they agree.
        let mileage = row["mileage"].as_array().unwrap();
        assert_eq!(mileage.len(), 3, "{row}");
        assert!(mileage[2]["km"].as_f64().unwrap() > 48_000.0);
        let ids = items(row);
        for id in [
            "vehicle",
            "vin_matches_papers",
            "mileage_consistent",
            "mileage_fits_age",
            "not_cleared_recently",
            "readiness_open",
            "faults_stored",
            "health",
        ] {
            assert!(ids.iter().any(|i| i == id), "{id} missing in {ids:?}");
        }
        // Stored faults and open monitors, but nothing hidden: yellow.
        assert_eq!(row["light"], 1, "{ids:?}");
        assert_eq!(row["storedFaults"], 2);
        assert!(row["distanceSinceClearKm"].as_f64().unwrap() > 1800.0);
        assert!(row["healthScore"].as_i64().unwrap() >= 0);
        assert!(!row["electric"].as_bool().unwrap());
        assert!(!row["questions"].as_array().unwrap().is_empty());
        let progress = Collect::named(&events, "used_car_progress");
        assert_eq!(progress.last().unwrap()["percent"], 100);
    }

    #[test]
    fn used_car_check_after_clearing_is_red() {
        let (mut worker, sink) = demo_worker();
        worker.used_car_live_seconds = 1;
        worker.handle(Job::ClearFaults);
        sink.take();
        worker.handle(Job::UsedCarCheck(String::new()));
        let events = sink.take();
        let row = &Collect::named(&events, "used_car_result")[0]["row"];
        let ids = items(row);
        assert!(ids.iter().any(|i| i == "cleared_recently"), "{ids:?}");
        // P0171 and P0420 are permanent: they survive mode 04.
        assert!(ids.iter().any(|i| i == "cleared_but_permanent"), "{ids:?}");
        assert_eq!(row["permanentFaults"], 2);
        assert_eq!(row["light"], 2);
        // No VIN from the papers: listed as not checked.
        assert_eq!(row["notChecked"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn used_car_check_needs_a_connection() {
        let sink = Arc::new(Collect::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
        worker.handle(Job::UsedCarCheck(String::new()));
        let events = sink.take();
        assert_eq!(Collect::named(&events, "failed").len(), 1);
        assert!(Collect::named(&events, "used_car_result").is_empty());
    }

    #[test]
    fn uds_scan_finds_the_demo_units() {
        let (mut worker, sink) = demo_worker();
        sink.take();
        // The generic sweep asks the engine first; the demo answers on
        // 7E0, 7E1 and four body units.
        worker.handle(Job::ScanUds("".into()));
        let events = sink.take();
        let started = &Collect::named(&events, "uds_scan_started")[0];
        assert_eq!(started["generic"], true);
        let found = Collect::named(&events, "uds_ecu_result");
        assert!(found.len() >= 4, "{found:?}");
        assert!(found.iter().any(|r| r["row"]["request"] == 0x7E0));
        let finished = &Collect::named(&events, "uds_scan_finished")[0];
        assert_eq!(finished["present"].as_u64().unwrap() as usize, found.len());
        assert_eq!(finished["cancelled"], false);
    }

    /// Cancels the scan once it reaches the sixth unit, from inside the
    /// event stream, the way the interface's cancel button would.
    struct CancelAt {
        events: Collect,
        cancel: Arc<AtomicBool>,
    }

    impl Sink for CancelAt {
        fn emit(&self, event: &str, payload: Value) {
            if event == "uds_scan_progress" && payload["index"] == 5 {
                self.cancel.store(true, Ordering::SeqCst);
            }
            self.events.emit(event, payload);
        }
    }

    #[test]
    fn uds_scan_can_be_cancelled() {
        let cancel = Arc::new(AtomicBool::new(false));
        let sink = Arc::new(CancelAt {
            events: Collect::default(),
            cancel: cancel.clone(),
        });
        let mut worker = Worker::new(sink.clone(), cancel);
        worker.apply_settings(AppSettings::default());
        worker.handle(Job::OpenTransport {
            transport: Box::new(DemoTransport::new(DemoOptions {
                latency_scale: 0.0,
                ..DemoOptions::default()
            })),
            protocol_order: "6".into(),
        });
        worker.handle(Job::ScanUds("".into()));
        let events = sink.events.take();
        let progress = Collect::named(&events, "uds_scan_progress");
        assert_eq!(
            progress.len(),
            6,
            "stops right after the unit it was cancelled at"
        );
        let finished = &Collect::named(&events, "uds_scan_finished")[0];
        assert_eq!(finished["cancelled"], true);
    }

    #[test]
    fn adapter_probe_and_raw_command() {
        let (mut worker, sink) = demo_worker();
        sink.take();
        worker.handle(Job::ProbeAdapter);
        worker.handle(Job::SendRaw("ATI".into()));
        let events = sink.take();
        let report = &Collect::named(&events, "adapter_report")[0]["row"];
        assert_eq!(report["reachable"], true);
        assert!(!report["verdict"].as_str().unwrap().is_empty());
        let reply = &Collect::named(&events, "raw_reply")[0]["reply"];
        assert!(reply.as_str().unwrap().contains("ELM327"), "{reply}");
    }

    #[test]
    fn custom_values_of_the_demo_pack() {
        let (mut worker, sink) = demo_worker();
        sink.take();
        let keys: Vec<i32> = built_in_packs()
            .iter()
            .filter(|p| p.pack == "caruspex-demo")
            .map(|p| p.key)
            .collect();
        assert!(!keys.is_empty());
        worker.handle(Job::ReadCustom(keys.clone()));
        let events = sink.take();
        let read = &Collect::named(&events, "custom_read")[0];
        assert!(read["ok"].as_u64().unwrap() > 0, "{read}");
        assert!(
            Collect::named(&events, "live_value")
                .iter()
                .any(|v| keys.contains(&(v["row"]["pid"].as_i64().unwrap() as i32)))
        );
    }

    /// An adapter that answers, in front of a car that does not.
    #[derive(Default)]
    struct SilentCar {
        reply: Vec<u8>,
    }

    impl Transport for SilentCar {
        fn write(&mut self, data: &[u8]) -> bool {
            let command = String::from_utf8_lossy(data).trim().to_ascii_uppercase();
            let answer = match command.as_str() {
                "" => "",
                "ATZ" => "ELM327 v1.5",
                "STI" => "?",
                "0100" => "UNABLE TO CONNECT",
                _ => "OK",
            };
            self.reply = format!("{answer}\r\r>").into_bytes();
            true
        }
        fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
            std::mem::take(&mut self.reply)
        }
        fn flush_input(&mut self) {}
        fn name(&self) -> String {
            "silent".into()
        }
    }

    #[test]
    fn a_failed_connect_comes_with_the_checklist() {
        let sink = Arc::new(Collect::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
        worker.handle(Job::OpenTransport {
            transport: Box::new(SilentCar::default()),
            protocol_order: "3".into(),
        });
        let events = sink.take();
        let failed = Collect::named(&events, "failed");
        assert_eq!(failed.len(), 1);
        let help = &failed[0]["help"];
        let state = |id: &str| {
            help["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|i| i["id"] == id)
                .map(|i| i["state"].as_str().unwrap_or_default().to_string())
        };
        assert_eq!(state("bus").as_deref(), Some("problem"));
        assert_eq!(state("ignition").as_deref(), Some("problem"));
        assert_eq!(state("clone").as_deref(), Some("check"));
        assert_eq!(state("power").as_deref(), Some("fine"));
        assert!(
            help["found"]
                .as_array()
                .unwrap()
                .iter()
                .any(|l| l.as_str().unwrap().contains("UNABLE TO CONNECT"))
        );
    }

    #[test]
    fn not_connected_answers() {
        let sink = Arc::new(Collect::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
        worker.handle(Job::ScanUds("VAG".into()));
        worker.handle(Job::BruteForceClear(3));
        worker.handle(Job::ReadMonitors);
        worker.handle(Job::StartRecording("x.jsonl".into()));
        let events = sink.take();
        assert_eq!(
            Collect::named(&events, "uds_scan_finished")[0]["present"],
            0
        );
        assert_eq!(
            Collect::named(&events, "clear_finished")[0]["everythingGone"],
            false
        );
        assert_eq!(
            Collect::named(&events, "monitor_results")[0]["rows"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            Collect::named(&events, "recording_state")[0]["active"],
            false
        );
    }

    #[test]
    fn closing_reports_closed() {
        let (mut worker, sink) = demo_worker();
        sink.take();
        worker.handle(Job::Close);
        let events = sink.take();
        assert_eq!(Collect::named(&events, "closed").len(), 1);
        worker.handle(Job::ReadFaults);
        assert!(Collect::named(&sink.take(), "faults").is_empty());
    }

    /// A VAG engine unit behind TP 2.0, just enough for the KKL pages: the
    /// channel, short KWP2000 answers, the fault memory. Other units refuse
    /// the channel at once, so the scan stays quick.
    #[derive(Default)]
    struct Tp20Engine {
        out: std::collections::VecDeque<caruspex_net::CanFrame>,
        inbound: Vec<u8>,
        seq: u8,
        faults: bool,
    }

    impl Tp20Engine {
        fn say(&mut self, id: u32, data: Vec<u8>) {
            self.out.push_back(caruspex_net::CanFrame {
                id,
                extended: false,
                data,
            });
        }
    }

    impl caruspex_net::CanLink for Tp20Engine {
        fn send(&mut self, f: &caruspex_net::CanFrame) -> bool {
            let first = f.data[0];
            if f.id == 0x200 {
                let reply = if first == 0x01 {
                    vec![0x00, 0xD0, 0x00, 0x03, 0x40, 0x07, 0x01]
                } else {
                    vec![0x00, 0xD6, 0x00, 0x10, 0x00, 0x10, 0x01]
                };
                self.say(0x200 + u32::from(first), reply);
                return true;
            }
            match first {
                0xA0 | 0xA3 => self.say(0x300, vec![0xA1, 0x0F, 0x8A, 0xFF, 0x32, 0xFF]),
                0xA8 => self.say(0x300, vec![0xA8]),
                _ if first >> 4 <= 3 => {
                    self.inbound.extend_from_slice(&f.data[1..]);
                    self.say(0x300, vec![0xB0 | ((first + 1) & 0x0F)]);
                    if first >> 4 == 1 {
                        let request = std::mem::take(&mut self.inbound);
                        let answer = match request[2] {
                            0x10 => vec![0x50, request[3]],
                            0x1A => [&[0x5A, 0x9B][..], b"1K0907115"].concat(),
                            0x18 if self.faults => vec![0x58, 0x01, 0x40, 0x66, 0xE0],
                            0x18 => vec![0x58, 0x00],
                            0x14 => {
                                self.faults = false;
                                vec![0x54, 0xFF, 0x00]
                            }
                            0x21 => vec![0x61, request[3], 0x01, 0xC8, 0x14],
                            service => vec![service + 0x40],
                        };
                        let mut data = (answer.len() as u16).to_be_bytes().to_vec();
                        data.extend_from_slice(&answer);
                        let chunks: Vec<Vec<u8>> = data.chunks(7).map(<[u8]>::to_vec).collect();
                        for (i, chunk) in chunks.iter().enumerate() {
                            let kind: u8 = if i + 1 == chunks.len() { 0x1 } else { 0x2 };
                            let mut packet = vec![(kind << 4) | self.seq];
                            self.seq = (self.seq + 1) & 0x0F;
                            packet.extend_from_slice(chunk);
                            self.say(0x300, packet);
                        }
                    }
                }
                _ => {}
            }
            true
        }
        fn receive(&mut self, _timeout_ms: u64) -> Option<caruspex_net::CanFrame> {
            self.out.pop_front()
        }
        fn name(&self) -> String {
            "TP 2.0 test".into()
        }
    }

    #[test]
    fn kkl_pages_use_tp20_on_the_can_interface() {
        let sink = Arc::new(Collect::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));

        // Without a CAN connection TP 2.0 says what is missing.
        worker.handle(Job::OpenKkl {
            device: TP20_DEVICE.into(),
            ecu_address: 0x01,
        });
        let events = sink.take();
        assert_eq!(Collect::named(&events, "kkl_message")[1]["isError"], true);

        worker.can_link = Some(SharedCanLink::new(Box::new(Tp20Engine {
            faults: true,
            ..Tp20Engine::default()
        })));
        worker.handle(Job::OpenKkl {
            device: TP20_DEVICE.into(),
            ecu_address: 0x01,
        });
        let events = sink.take();
        let connected = Collect::named(&events, "kkl_connected");
        assert_eq!(connected.last().unwrap()["connected"], true);
        assert_eq!(connected.last().unwrap()["identification"], "1K0907115");
        assert_eq!(
            Collect::named(&events, "kkl_protocol")[0]["protocolName"],
            "KWP2000 (TP 2.0)"
        );
        assert!(worker.kkl_keep_alive_timer.active);
        worker.kkl_keep_alive_tick();
        assert!(worker.kkl_connected());

        worker.handle(Job::StartBasicSetting(1));
        worker.basic_setting_tick();
        let events = sink.take();
        let values = &Collect::named(&events, "basic_setting_values")[0]["values"];
        assert_eq!(values.as_array().unwrap().len(), 1);
        worker.handle(Job::CloseKkl);
        assert!(!worker.kkl_connected());

        // The scan of all units: the engine with its fault, cleared.
        worker.handle(Job::ScanAllEcus {
            device: TP20_DEVICE.into(),
            clear_attempts: 2,
        });
        let events = sink.take();
        let finished = &Collect::named(&events, "scan_finished")[0];
        assert_eq!(finished["present"], 1);
        assert_eq!(finished["withFaults"], 0);
        let engine = &Collect::named(&events, "scan_ecu_result")[0]["row"];
        assert_eq!(engine["present"], true);
        assert!(
            engine["identification"]
                .as_str()
                .unwrap()
                .contains("TP 2.0")
        );

        // Closing the adapter lets go of the interface.
        worker.handle(Job::Close);
        assert!(worker.can_link.is_none());
    }
}
