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
use caruspex_i18n::l;
use caruspex_link::elm327::DEFAULT_SEND_TIMEOUT_MS;
use caruspex_link::timing::sleep_ms;
use caruspex_link::{AdaptiveTiming, Elm327, StnBus, TcpTransport, Transport};
#[cfg(not(target_os = "android"))]
use caruspex_link::{SerialTransport, enumerate_ports};
use caruspex_net::{
    CanElm, SharedCanLink, SlcanLink, open_can_interface, open_network_car, probe_slcan,
};
use caruspex_protocols::{Obd2, VagDiagnosis};
use caruspex_record::{
    CustomPid, ReplayPacing, ReplayTransport, active_recording, recording_summary, start_recording,
    stop_recording,
};
use serde::Serialize;
use serde_json::{Value, json};

// Android: USB adapters and classic Bluetooth through the Java side.
#[cfg(target_os = "android")]
use crate::android::{SerialTransport, ble, enumerate_ports};
use crate::rows::*;
use crate::settings::AppSettings;

mod adapter;
/// Bluetooth LE and classic Bluetooth through caruspex-link, as the worker
/// needs it (android.rs has the same functions for the phone).
#[cfg(not(target_os = "android"))]
mod ble;
mod convert;
mod health;
mod kkl;
mod live;
mod truck;
mod uds;
mod used_car;
use crate::texts::{args, w};
use convert::{fault_row, status_row, to_row};
use truck::TruckLink;
use used_car::USED_CAR_LIVE_SECONDS;

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

/// The protocol order a connect job carries; ENET, CAN and the demo choose
/// their own.
fn protocol_order_of(job: &Job) -> String {
    match job {
        Job::OpenSerial { protocol_order, .. }
        | Job::OpenTcp { protocol_order, .. }
        | Job::OpenBle { protocol_order, .. }
        | Job::OpenReplay { protocol_order, .. } => protocol_order.clone(),
        _ => "54312678".to_string(),
    }
}

/// "COM7, 38400 baud, protocol 6" for the log.
fn route_text(route: &ConnectRoute) -> String {
    let mut parts = vec![if route.name.is_empty() {
        route.device.clone()
    } else {
        route.name.clone()
    }];
    if route.baud > 0 {
        parts.push(format!("{} Baud", route.baud));
    }
    if !route.protocol.is_empty() {
        parts.push(args(l("Protokoll %1", "protocol %1"), &[&route.protocol]));
    }
    parts.retain(|p| !p.is_empty());
    parts.join(", ")
}

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
    /// First the way that worked last time for this car, quietly; only if
    /// no adapter answers there, `then` (the search the user asked for).
    OpenRemembered {
        route: ConnectRoute,
        then: Box<Job>,
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
    /// Trucks over SAE J1939 for the connect jobs that follow: the bit rate
    /// in kbit/s (250, 500, or 0 for both in turn); `None` for cars.
    SetTruck(Option<u32>),
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
    /// The bus of the last control unit scan, so clearing a unit found on
    /// the MS-CAN or SW-CAN of an OBDLink goes to the same bus.
    uds_bus: StnBus,
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
    /// The way the running connection attempt takes, reported with
    /// "connected" so the garage can remember it.
    route: ConnectRoute,
    /// The remembered protocol, tried first while `Job::OpenRemembered`
    /// runs (also in the search after it).
    prefer_protocol: String,
    /// While the remembered way is tried quietly: a failure is only noted
    /// here instead of reported.
    quiet: Option<Option<String>>,
    /// Connected to a truck: how its J1939 bus is reached.
    truck: Option<TruckLink>,
    /// What was heard on the J1939 bus, and the tester's address.
    truck_session: caruspex_protocols::J1939Session,
    /// The connect jobs go to a truck at this rate (see `Job::SetTruck`).
    truck_request: Option<u32>,
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
            uds_bus: StnBus::HighSpeed,
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
            route: ConnectRoute::default(),
            prefer_protocol: String::new(),
            quiet: None,
            truck: None,
            truck_session: caruspex_protocols::J1939Session::new(),
            truck_request: None,
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
    fn connect_failed(&mut self, error: impl Into<String>) {
        if let Some(quiet) = self.quiet.as_mut() {
            *quiet = Some(error.into());
            return;
        }
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
            } => {
                // A phone sends sockets out over mobile data otherwise.
                #[cfg(target_os = "android")]
                crate::android::bind_wifi(true);
                self.open_tcp(&host, port, &protocol_order)
            }
            Job::OpenDemo => self.open_demo(),
            Job::OpenEnet { host } => {
                #[cfg(target_os = "android")]
                crate::android::bind_wifi(true);
                self.open_enet(&host)
            }
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
            Job::OpenRemembered { route, then } => self.open_remembered(route, *then),
            Job::OpenTransport {
                transport,
                protocol_order,
            } => {
                self.route = ConnectRoute {
                    kind: "transport".into(),
                    ..ConnectRoute::default()
                };
                self.busy(true);
                self.set_elm_log();
                self.begin_connect(LinkKind::Usb);
                let ok = self.finish_connect(transport, &protocol_order, true);
                self.busy(false);
                if ok {
                    self.refresh_status_and_faults();
                }
            }
            Job::Close => {
                self.close_adapter();
                #[cfg(target_os = "android")]
                crate::android::bind_wifi(false);
            }
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
            Job::SetTruck(rate) => self.truck_request = rate,
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
        if let Some(kbit) = self.truck_request {
            return self.try_connect_truck(transport, kbit);
        }
        self.facts.port_opened = Some(true);
        self.facts.port_busy = false;
        let order = prefer_protocol(protocol_order, &self.prefer_protocol);
        let result = self.elm.connect(transport, &order);
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
        self.uds_bus = StnBus::HighSpeed;
        // The extra CAN buses an OBDLink can switch to ("ms", "sw"), for the
        // control unit page.
        let stn_buses: Vec<&str> = self.elm.stn_extra_buses().iter().map(|b| b.key()).collect();
        let mut route = self.route.clone();
        let protocol = self.elm.protocol_id();
        if protocol != '0' {
            route.protocol = protocol.to_string();
        }
        self.emit(
            "connected",
            json!({
                "adapter": self.elm.version(),
                "transport": self.elm.transport_name(),
                "protocol": self.elm.protocol_text(),
                "rawCan": self.can_link.is_some(),
                "stnBuses": stn_buses,
                "route": route,
            }),
        );
        Ok(())
    }

    /// The way that worked last time for this car, then the search the user
    /// asked for. The second attempt only runs when no adapter answered on
    /// the first: an adapter that answered has already been through every
    /// protocol, the car is the problem there, and that is reported.
    fn open_remembered(&mut self, route: ConnectRoute, then: Job) {
        self.log(args(
            l(
                "Versuche zuerst den Weg, der zuletzt ging: %1",
                "Trying the way that worked last time first: %1",
            ),
            &[&route_text(&route)],
        ));
        self.prefer_protocol = route.protocol.clone();
        self.quiet = Some(None);
        // The usual rates when the remembered one is unknown.
        let baud = i32::try_from(route.baud).unwrap_or(0);
        match route.kind.as_str() {
            "serial" if !route.device.is_empty() => {
                self.open_serial(&route.device, baud, &protocol_order_of(&then))
            }
            "tcp" => {
                let (host, port) = route
                    .device
                    .rsplit_once(':')
                    .and_then(|(h, p)| p.parse().ok().map(|p| (h.to_string(), p)))
                    .unwrap_or((route.device.clone(), 35000));
                self.open_tcp(&host, port, &protocol_order_of(&then))
            }
            "ble" if !route.device.is_empty() => self.open_ble(
                &route.device,
                &route.name,
                route.kinds,
                &protocol_order_of(&then),
            ),
            "can" if !route.device.is_empty() => self.open_can(&route.device),
            "enet" => self.open_enet(&route.device),
            _ => {}
        }
        let error = self.quiet.take().flatten();
        if self.elm.connected() || self.truck_connected() {
            self.prefer_protocol.clear();
            return;
        }
        // A CAN interface always "answers" (its ELM front end is ours); a
        // bit rate chosen now that differs from the remembered one is
        // still worth its own try.
        let other_rate = matches!(&then, Job::OpenCan { spec } if *spec != route.device);
        if self.facts.adapter_answered == Some(true)
            && !other_rate
            && let Some(error) = error
        {
            self.prefer_protocol.clear();
            self.connect_failed(error);
            return;
        }
        self.log(l(
            "Der gemerkte Weg geht diesmal nicht, suche wie gewohnt.",
            "The remembered way does not work this time, searching as usual.",
        ));
        self.handle(then);
        self.prefer_protocol.clear();
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
                self.route = ConnectRoute {
                    kind: "serial".into(),
                    device: candidate_device.clone(),
                    baud: b,
                    ..ConnectRoute::default()
                };
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
                        self.route = ConnectRoute {
                            kind: "can".into(),
                            device: format!("slcan:{candidate_device}@500000"),
                            ..ConnectRoute::default()
                        };
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
        self.route = ConnectRoute {
            kind: "tcp".into(),
            device: format!("{host}:{port}"),
            ..ConnectRoute::default()
        };
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
            self.route = ConnectRoute {
                kind: "ble".into(),
                device: d.id.clone(),
                name: d.name.clone(),
                kinds: ble::kinds(&d),
                ..ConnectRoute::default()
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
        self.route = ConnectRoute {
            kind: "enet".into(),
            device: host.to_string(),
            ..ConnectRoute::default()
        };
        let ok = self.finish_connect(Box::new(car.transport), "6", true);
        self.busy(false);
        if ok {
            self.refresh_status_and_faults();
        }
    }

    fn open_can(&mut self, spec: &str) {
        if let Some(kbit) = self.truck_request {
            self.open_can_truck(spec, kbit);
            return;
        }
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
        self.route = ConnectRoute {
            kind: "can".into(),
            device: spec.to_string(),
            ..ConnectRoute::default()
        };
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
        if let Some(kbit) = self.truck_request {
            self.open_demo_truck(kbit);
            return;
        }
        // The demo car speaks CAN; the K-line protocols would only cost seconds.
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Usb);
        self.route = ConnectRoute {
            kind: "demo".into(),
            ..ConnectRoute::default()
        };
        // CARUSPEX_DEMO_ADAPTER=obdlink-mx lets the demo car answer like an
        // OBDLink MX+ (MS-CAN and SW-CAN), for pictures of those buses.
        let adapter = match std::env::var("CARUSPEX_DEMO_ADAPTER").as_deref() {
            Ok("obdlink-mx") => caruspex_demo::DemoAdapter::ObdLinkMxPlus,
            Ok("obdlink-ex") => caruspex_demo::DemoAdapter::ObdLinkEx,
            _ => caruspex_demo::DemoAdapter::default(),
        };
        let demo = caruspex_demo::DemoTransport::new(caruspex_demo::DemoOptions {
            adapter,
            ..caruspex_demo::DemoOptions::default()
        });
        let ok = self.finish_connect(Box::new(demo), "6", true);
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
        self.route = ConnectRoute {
            kind: "replay".into(),
            ..ConnectRoute::default()
        };
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
        if self.truck_connected() {
            self.refresh_truck_faults();
            return;
        }
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
        if self.truck_connected() {
            self.clear_truck_faults();
            return;
        }
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
        if self.truck_connected() {
            self.read_truck_details();
            return;
        }
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

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use caruspex_demo::{DemoOptions, DemoTransport};
    use caruspex_record::built_in_packs;

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

    fn quick_demo() -> Box<dyn Transport> {
        Box::new(DemoTransport::new(DemoOptions {
            latency_scale: 0.0,
            ..DemoOptions::default()
        }))
    }

    fn log_lines(events: &[(String, Value)]) -> Vec<String> {
        Collect::named(events, "log_line")
            .iter()
            .filter_map(|v| v["line"].as_str().map(str::to_string))
            .collect()
    }

    #[test]
    fn connected_reports_the_way_it_went() {
        let (_worker, sink) = demo_worker();
        let events = sink.take();
        let route = &Collect::named(&events, "connected")[0]["route"];
        assert_eq!(route["kind"], "transport");
        assert_eq!(route["protocol"], "6");
    }

    #[test]
    fn a_remembered_port_that_is_gone_falls_back_to_the_search() {
        let sink = Arc::new(Collect::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
        worker.apply_settings(AppSettings::default());
        // A port that does not exist on any computer; the fallback is the
        // demo car, with the remembered protocol in front of the order.
        worker.handle(Job::OpenRemembered {
            route: ConnectRoute {
                kind: "serial".into(),
                device: "caruspex-no-such-port".into(),
                baud: 38400,
                protocol: "6".into(),
                ..ConnectRoute::default()
            },
            then: Box::new(Job::OpenTransport {
                transport: quick_demo(),
                protocol_order: "16".into(),
            }),
        });
        let events = sink.take();
        assert!(
            Collect::named(&events, "failed").is_empty(),
            "the quiet attempt reports nothing: {events:?}"
        );
        assert_eq!(Collect::named(&events, "connected").len(), 1);
        let lines = log_lines(&events);
        assert!(lines.iter().any(|l| l.contains("caruspex-no-such-port")));
        // Protocol 6 first: the K-line protocol 1 was never tried.
        assert!(!lines.iter().any(|l| l.contains(" 1 (")), "{lines:?}");
        assert!(worker.prefer_protocol.is_empty());
        assert!(worker.quiet.is_none());
    }

    #[test]
    fn without_a_remembered_way_failures_are_reported_as_before() {
        let sink = Arc::new(Collect::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
        worker.handle(Job::OpenRemembered {
            route: ConnectRoute {
                kind: "serial".into(),
                device: "caruspex-no-such-port".into(),
                ..ConnectRoute::default()
            },
            then: Box::new(Job::OpenSerial {
                device: "caruspex-no-such-port-either".into(),
                baud: 38400,
                protocol_order: "6".into(),
            }),
        });
        let events = sink.take();
        // Only the search the user asked for says it failed.
        let failed = Collect::named(&events, "failed");
        assert_eq!(failed.len(), 1, "{events:?}");
        assert!(failed[0]["help"].is_object() || failed[0]["help"].is_array());
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

    fn demo_worker_with(adapter: caruspex_demo::DemoAdapter) -> (Worker, Arc<Collect>) {
        let sink = Arc::new(Collect::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
        worker.apply_settings(AppSettings::default());
        let options = DemoOptions {
            adapter,
            latency_scale: 0.0,
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
    fn an_obdlink_ex_reaches_the_body_unit_on_the_ms_can_and_comes_back() {
        use caruspex_demo::DemoAdapter;
        let (mut worker, sink) = demo_worker_with(DemoAdapter::ObdLinkEx);
        let events = sink.take();
        let connected = &Collect::named(&events, "connected")[0];
        assert_eq!(connected["stnBuses"], json!(["ms"]));

        worker.handle(Job::ScanUds("FORD@MS".into()));
        let events = sink.take();
        let found = Collect::named(&events, "uds_ecu_result");
        assert_eq!(found.len(), 1, "{events:?}");
        assert_eq!(found[0]["row"]["request"], 0x726);
        // The row and the start of the scan name the bus, for the X-ray view.
        assert_eq!(found[0]["row"]["bus"], "ms");
        assert_eq!(Collect::named(&events, "uds_scan_started")[0]["bus"], "ms");
        assert_eq!(worker.elm.stn_bus(), StnBus::HighSpeed);
        assert!(worker.elm.request("0100", 0x01, 2000).ok());

        // The SW-CAN is not there on the EX: the scan says why.
        worker.handle(Job::ScanUds("GM@SW".into()));
        let events = sink.take();
        let finished = &Collect::named(&events, "uds_scan_finished")[0];
        assert_eq!(finished["present"], 0);
        assert!(!finished["note"].as_str().unwrap().is_empty());

        // A plain ELM327 offers no extra bus.
        let (_plain, sink) = demo_worker();
        let connected = &Collect::named(&sink.take(), "connected")[0];
        assert_eq!(connected["stnBuses"], json!([]));
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
