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
mod connect;
mod convert;
mod faults;
mod health;
mod kkl;
mod live;
mod truck;
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
