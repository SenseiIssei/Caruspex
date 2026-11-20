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
use caruspex_link::{AdaptiveTiming, Elm327, StnBus, Transport};
use caruspex_net::SharedCanLink;
use caruspex_protocols::{Obd2, VagDiagnosis};
use caruspex_record::CustomPid;
use serde::Serialize;
use serde_json::{Value, json};

use crate::rows::*;
use crate::settings::AppSettings;

mod adapter;
/// Bluetooth LE and classic Bluetooth through caruspex-link, as the worker
/// needs it (android.rs has the same functions for the phone).
#[cfg(not(target_os = "android"))]
mod ble;
mod bmw;
mod connect;
mod convert;
mod faults;
mod health;
mod kkl;
mod live;
mod truck;
mod truck_check;
mod uds;
mod used_car;
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
    /// BMW E series on the K-line: every known unit; with `clear` every
    /// unit with entries is cleared (the page asked first).
    ScanBmw {
        device: String,
        clear: bool,
    },
    /// Clears one BMW unit found by the scan ("ds2", "kwp2000",
    /// "bmw-fast").
    ClearBmwUnit {
        device: String,
        address: i32,
        protocol: String,
    },
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
    /// The next tick one interval after `start`, at once if that is past.
    fn rearm_from(&mut self, start: Instant) {
        if self.active {
            self.next = Some(start + Duration::from_millis(self.interval_ms));
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
    /// The simulated BMWs of the BMW page, kept so a cleared fault memory
    /// stays cleared until the program ends.
    bmw_sims: Vec<(caruspex_demo::bmw::BmwCar, caruspex_demo::bmw::BmwSimLine)>,
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
            bmw_sims: Vec::new(),
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
    // The frequent events hand their json! straight to the sink: through
    // `emit` the finished Value would be copied once more by to_value.
    fn log(&self, line: impl Into<String>) {
        self.sink.emit("log_line", json!({ "line": line.into() }));
    }
    fn busy(&self, active: bool) {
        self.sink.emit("busy", json!({ "active": active }));
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
        self.sink.emit("live_value", json!({ "row": row }));
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
        self.batch_refused = obd.into_batch_memory();
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
            // The interval counts from the start of the tick: a slow adapter
            // goes straight on to the next round instead of idling another
            // interval, a fast one still gets no more ticks than the
            // interval allows.
            self.poll_timer.rearm_from(now);
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
            Job::ScanBmw { device, clear } => self.scan_bmw(&device, clear),
            Job::ClearBmwUnit {
                device,
                address,
                protocol,
            } => self.clear_bmw_unit(&device, address, &protocol),
            Job::ApplySettings(values) => self.apply_settings(values),
            Job::SetTruck(rate) => self.truck_request = rate,
            Job::Shutdown => self.shutdown(),
        }
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
mod tests;
