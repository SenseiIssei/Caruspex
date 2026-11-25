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
    /// The rows of every `live_values` event.
    fn live_rows(events: &[(String, Value)]) -> Vec<Value> {
        Self::named(events, "live_values")
            .iter()
            .flat_map(|v| v["rows"].as_array().cloned().unwrap_or_default())
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
    assert!(!Collect::live_rows(&events).is_empty());
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
    // One event per tick with all its values, not one per value.
    let ticks = Collect::named(&events, "live_values");
    assert!((1..=3).contains(&ticks.len()), "{ticks:?}");
    assert!(
        ticks
            .iter()
            .all(|t| t["rows"].as_array().unwrap().len() == 3)
    );
    let values = Collect::live_rows(&events);
    assert!(values.iter().any(|v| v["pid"] == 0x0C), "{values:?}");
    assert!(values.iter().all(|v| v["time"].as_f64().unwrap() > 0.0));
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
        Collect::live_rows(&events)
            .iter()
            .any(|v| keys.contains(&(v["pid"].as_i64().unwrap() as i32)))
    );
}

// A VAG diesel value from OBDb that needs the extended session, read from
// the demo engine (which keeps 22 114E in that session): only in pro mode,
// marked as read in the session, and the session is left when the value is
// no longer polled.
#[test]
fn extended_session_values_in_pro_mode_only_and_left_afterwards() {
    let key = caruspex_record::obdb_packs()
        .iter()
        .flat_map(|p| caruspex_record::obdb_pack_pids(&p.id))
        .find(|p| p.session == 0x03 && p.request == "22114E" && p.header == "7E0")
        .expect("an OBDb soot mass value")
        .key;
    let (mut worker, sink) = demo_worker();
    sink.take();

    // Beginner mode (the default): shown as unavailable, with the reason.
    worker.handle(Job::ReadCustom(vec![key]));
    let rows = Collect::live_rows(&sink.take());
    assert_eq!(rows.len(), 1);
    assert!(rows[0].get("session").is_none());
    assert!(
        rows[0]["reason"].as_str().unwrap().contains("Profimodus"),
        "{}",
        rows[0]
    );
    assert!(!worker.sessions.any_active());

    // Pro mode: read in the session; read once only, so it is left again.
    worker.apply_settings(AppSettings {
        pro_mode: true,
        ..AppSettings::default()
    });
    worker.handle(Job::ReadCustom(vec![key]));
    let rows = Collect::live_rows(&sink.take());
    assert_eq!(rows[0]["session"], true, "{}", rows[0]);
    assert!(rows[0].get("reason").is_none());
    assert!(!worker.sessions.any_active());

    // Polled: the engine stays in the session between ticks ...
    worker.handle(Job::StartCustomPoll(vec![key]));
    worker.poll_tick();
    assert!(worker.sessions.any_active());
    // ... until the values change to ones without it, or the measurement stops.
    worker.handle(Job::StartCustomPoll(vec![]));
    assert!(!worker.sessions.any_active());
    worker.handle(Job::StartCustomPoll(vec![key]));
    worker.poll_tick();
    assert!(worker.sessions.any_active());
    worker.handle(Job::StopLive);
    assert!(!worker.sessions.any_active());

    // Switched off in the settings: left at once, then shown as unavailable.
    worker.handle(Job::StartCustomPoll(vec![key]));
    worker.poll_tick();
    assert!(worker.sessions.any_active());
    worker.apply_settings(AppSettings {
        pro_mode: true,
        extended_session_values: false,
        ..AppSettings::default()
    });
    assert!(!worker.sessions.any_active());
    sink.take();
    worker.handle(Job::ReadCustom(vec![key]));
    let rows = Collect::live_rows(&sink.take());
    assert!(
        rows[0]["reason"]
            .as_str()
            .unwrap()
            .contains("Einstellungen"),
        "{}",
        rows[0]
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

fn bare_worker() -> (Worker, Arc<Collect>) {
    let sink = Arc::new(Collect::default());
    let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
    worker.apply_settings(AppSettings::default());
    (worker, sink)
}

#[test]
fn the_bmw_scan_runs_on_the_simulated_e46_and_clears_only_when_asked() {
    let (mut worker, sink) = bare_worker();
    worker.handle(Job::ScanBmw {
        device: "sim:e46".into(),
        clear: false,
    });
    let events = sink.take();
    let finished = &Collect::named(&events, "bmw_scan_finished")[0];
    assert_eq!(finished["present"], 9, "{events:?}");
    assert_eq!(finished["withFaults"], 4);
    assert_eq!(finished["error"], "");
    let units = Collect::named(&events, "bmw_unit");
    let dme = units
        .iter()
        .map(|u| &u["row"])
        .find(|r| r["address"] == 0x12)
        .unwrap();
    assert_eq!(dme["protocol"], "ds2");
    assert_eq!(dme["protocolLabel"], "DS2");
    assert_eq!(dme["faults"][0]["code"], "0x2A");
    assert!(dme["faults"][0]["present"].is_null());
    assert!(Collect::named(&events, "bmw_clear_result").is_empty());
    assert!(Collect::named(&events, "bmw_scan_progress").len() >= 9);

    // One unit cleared: the airbag is empty afterwards.
    worker.handle(Job::ClearBmwUnit {
        device: "sim:e46".into(),
        address: 0xA4,
        protocol: "ds2".into(),
    });
    let events = sink.take();
    let result = &Collect::named(&events, "bmw_clear_result")[0];
    assert_eq!(result["ok"], true);
    assert_eq!(result["remaining"], 0);
    let row = &Collect::named(&events, "bmw_unit")[0]["row"];
    assert_eq!(row["faults"].as_array().unwrap().len(), 0);

    // The simulated car remembers it in the next scan.
    worker.handle(Job::ScanBmw {
        device: "sim:e46".into(),
        clear: false,
    });
    let events = sink.take();
    assert_eq!(
        Collect::named(&events, "bmw_scan_finished")[0]["withFaults"],
        3
    );
}

#[test]
fn the_bmw_scan_with_clearing_leaves_only_faults_with_a_cause() {
    let (mut worker, sink) = bare_worker();
    worker.handle(Job::ScanBmw {
        device: "sim:e60".into(),
        clear: true,
    });
    let events = sink.take();
    let finished = &Collect::named(&events, "bmw_scan_finished")[0];
    assert_eq!(finished["present"], 8);
    // The engine keeps the fault whose cause is still there.
    assert_eq!(finished["withFaults"], 1);
    let clears = Collect::named(&events, "bmw_clear_result");
    assert_eq!(clears.len(), 4);
    assert!(clears.iter().all(|c| c["ok"] == true));
    assert!(clears.iter().any(|c| c["remaining"] == 1));
}

#[test]
fn the_bmw_page_needs_a_serial_cable() {
    let (mut worker, sink) = bare_worker();
    worker.handle(Job::ScanBmw {
        device: TP20_DEVICE.into(),
        clear: false,
    });
    let events = sink.take();
    let finished = &Collect::named(&events, "bmw_scan_finished")[0];
    assert_eq!(finished["present"], 0);
    assert!(finished["error"].as_str().unwrap().contains("K+DCAN"));
    worker.handle(Job::ClearBmwUnit {
        device: "j2534:x.dll".into(),
        address: 0x12,
        protocol: "ds2".into(),
    });
    let events = sink.take();
    assert_eq!(Collect::named(&events, "bmw_clear_result")[0]["ok"], false);
}

// --- Threads: closing and ending while something runs -----------------------------

/// The demo car behind an adapter that can fall silent, like a Bluetooth
/// adapter out of range or a cable pulled: writes vanish and every read
/// waits its whole time for nothing.
struct Muted {
    inner: DemoTransport,
    mute: Arc<AtomicBool>,
}

impl Transport for Muted {
    fn write(&mut self, data: &[u8]) -> bool {
        if self.mute.load(Ordering::SeqCst) {
            return true;
        }
        self.inner.write(data)
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        if self.mute.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(timeout_ms));
            return Vec::new();
        }
        self.inner.read(timeout_ms)
    }
    fn flush_input(&mut self) {
        self.inner.flush_input();
    }
    fn name(&self) -> String {
        "muted".into()
    }
}

/// Waits up to `limit` until the sink has `count` events named `name`.
fn wait_for(sink: &Collect, name: &str, count: usize, limit: Duration) -> bool {
    let until = Instant::now() + limit;
    while Instant::now() < until {
        let seen = sink
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|(e, _)| e == name)
            .count();
        if seen >= count {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

fn spawned_demo(transport: Box<dyn Transport>) -> (WorkerHandle, Arc<Collect>) {
    let sink = Arc::new(Collect::default());
    let handle = spawn(sink.clone(), AppSettings::default());
    handle.send(Job::OpenTransport {
        transport,
        protocol_order: "6".into(),
    });
    assert!(wait_for(&sink, "connected", 1, Duration::from_secs(10)));
    (handle, sink)
}

#[test]
fn disconnect_on_a_silent_adapter_does_not_wait_for_the_live_round() {
    let mute = Arc::new(AtomicBool::new(false));
    let (handle, sink) = spawned_demo(Box::new(Muted {
        inner: DemoTransport::new(DemoOptions {
            latency_scale: 0.0,
            ..DemoOptions::default()
        }),
        mute: mute.clone(),
    }));
    handle.send(Job::StartLive(vec![0x0C, 0x0D, 0x05, 0x11, 0x04, 0x0F]));
    assert!(wait_for(&sink, "live_values", 3, Duration::from_secs(10)));
    // The adapter falls silent: the live request in flight waits for its
    // four seconds, and the next ones would wait as long.
    mute.store(true, Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(300));
    let started = Instant::now();
    handle.close();
    assert!(wait_for(&sink, "closed", 1, Duration::from_secs(20)));
    let took = started.elapsed();
    // What is left is the adapter's ATPC (at most 2 s; whether the adapter
    // is gone or only had not answered yet is not known when the request
    // was cut short). Before, the close waited for the request in flight
    // (4 s), the six single requests the round fell back to (4 s each) and
    // then the ATPC: about 30 s.
    assert!(took < Duration::from_millis(2600), "took {took:?}");

    // The flags are down again: the next connection works as usual.
    sink.take();
    handle.send(Job::OpenTransport {
        transport: quick_demo(),
        protocol_order: "6".into(),
    });
    assert!(wait_for(&sink, "connected", 1, Duration::from_secs(10)));
    handle.send(Job::StartLive(vec![0x0C, 0x0D]));
    assert!(wait_for(&sink, "live_values", 4, Duration::from_secs(10)));
    assert!(handle.shutdown(Duration::from_secs(5)));
}

#[test]
fn ending_the_program_during_a_health_check_frees_the_worker() {
    let (handle, sink) = spawned_demo(quick_demo());
    handle.send(Job::HealthCheck(60));
    assert!(wait_for(&sink, "live_values", 3, Duration::from_secs(10)));
    let started = Instant::now();
    assert!(handle.shutdown(Duration::from_secs(5)), "the worker ended");
    let took = started.elapsed();
    assert!(took < Duration::from_millis(1500), "took {took:?}");
    // A second caller does not wait again.
    assert!(!handle.shutdown(Duration::from_millis(10)));
}

#[test]
fn commands_from_many_threads_at_once_cannot_hang_the_worker() {
    let (handle, sink) = spawned_demo(quick_demo());
    let threads: Vec<_> = (0..4)
        .map(|n| {
            let handle = handle.clone();
            std::thread::spawn(move || {
                for i in 0..40 {
                    match (i + n) % 8 {
                        0 => handle.send(Job::StartLive(vec![0x0C, 0x0D, 0x05])),
                        1 => handle.send(Job::StopLive),
                        2 => handle.send(Job::ReadFaults),
                        3 => handle.close(),
                        4 => handle.cancel_scan(),
                        5 => handle.send(Job::OpenTransport {
                            transport: quick_demo(),
                            protocol_order: "6".into(),
                        }),
                        6 => handle.send(Job::SendRaw("ATI".into())),
                        _ => handle.send(Job::StartVoltagePoll(20)),
                    }
                    if i % 5 == 0 {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    // Everything queued gets through and the worker still answers.
    handle.close();
    assert!(wait_for(&sink, "closed", 21, Duration::from_secs(60)));
    assert!(handle.shutdown(Duration::from_secs(10)));
}

// --- The way to the adapter goes away ---------------------------------------------

/// The demo car on a cable that can be pulled: the transport says so, every
/// write fails and every read waits for nothing.
struct Pulled {
    inner: DemoTransport,
    gone: Arc<AtomicBool>,
}

impl Transport for Pulled {
    fn write(&mut self, data: &[u8]) -> bool {
        !self.gone.load(Ordering::SeqCst) && self.inner.write(data)
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        if self.gone.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(timeout_ms));
            return Vec::new();
        }
        self.inner.read(timeout_ms)
    }
    fn flush_input(&mut self) {
        self.inner.flush_input();
    }
    fn name(&self) -> String {
        "pulled".into()
    }
    fn lost(&mut self) -> bool {
        self.gone.load(Ordering::SeqCst)
    }
}

fn pulled_demo() -> (Pulled, Arc<AtomicBool>) {
    let gone = Arc::new(AtomicBool::new(false));
    let cable = Pulled {
        inner: DemoTransport::new(DemoOptions {
            latency_scale: 0.0,
            ..DemoOptions::default()
        }),
        gone: gone.clone(),
    };
    (cable, gone)
}

fn lost_reason(sink: &Collect) -> String {
    let events = sink.0.lock().unwrap();
    let closed = Collect::named(&events, "closed");
    assert_eq!(closed.len(), 1, "{closed:?}");
    closed[0]["lost"].as_str().unwrap_or_default().to_string()
}

#[test]
fn a_pulled_cable_is_noticed_while_nothing_runs() {
    let (cable, gone) = pulled_demo();
    let (handle, sink) = spawned_demo(Box::new(cable));
    assert!(wait_for(
        &sink,
        "supported_pids",
        1,
        Duration::from_secs(10)
    ));
    gone.store(true, Ordering::SeqCst);
    let started = Instant::now();
    assert!(wait_for(&sink, "closed", 1, Duration::from_secs(10)));
    let took = started.elapsed();
    assert!(took < Duration::from_millis(3500), "took {took:?}");
    assert!(!lost_reason(&sink).is_empty());
    // It is reported once, and the next connection works as usual.
    std::thread::sleep(Duration::from_millis(300));
    lost_reason(&sink);
    sink.take();
    handle.send(Job::OpenTransport {
        transport: quick_demo(),
        protocol_order: "6".into(),
    });
    assert!(wait_for(&sink, "connected", 1, Duration::from_secs(10)));
    assert!(handle.shutdown(Duration::from_secs(5)));
}

#[test]
fn a_pulled_cable_is_noticed_at_once_while_values_are_read() {
    let (cable, gone) = pulled_demo();
    let (handle, sink) = spawned_demo(Box::new(cable));
    handle.send(Job::StartLive(vec![0x0C, 0x0D, 0x05, 0x11, 0x04, 0x0F]));
    assert!(wait_for(&sink, "live_values", 3, Duration::from_secs(10)));
    gone.store(true, Ordering::SeqCst);
    let started = Instant::now();
    assert!(wait_for(&sink, "closed", 1, Duration::from_secs(10)));
    let took = started.elapsed();
    // Before: never. The live loop went on with a timeout per round.
    assert!(took < Duration::from_millis(1000), "took {took:?}");
    assert!(!lost_reason(&sink).is_empty());
    assert!(handle.shutdown(Duration::from_secs(5)));
}

#[test]
fn an_adapter_that_fell_silent_is_noticed() {
    let mute = Arc::new(AtomicBool::new(false));
    let sink = Arc::new(Collect::default());
    let settings = AppSettings {
        timeout_percent: 10,
        ..AppSettings::default()
    };
    let handle = spawn(sink.clone(), settings);
    handle.send(Job::OpenTransport {
        transport: Box::new(Muted {
            inner: DemoTransport::new(DemoOptions {
                latency_scale: 0.0,
                ..DemoOptions::default()
            }),
            mute: mute.clone(),
        }),
        protocol_order: "6".into(),
    });
    assert!(wait_for(&sink, "connected", 1, Duration::from_secs(10)));
    handle.send(Job::StartLive(vec![0x0C, 0x0D]));
    assert!(wait_for(&sink, "live_values", 2, Duration::from_secs(10)));
    mute.store(true, Ordering::SeqCst);
    assert!(wait_for(&sink, "closed", 1, Duration::from_secs(10)));
    assert!(!lost_reason(&sink).is_empty());
    assert!(handle.shutdown(Duration::from_secs(5)));
}

#[test]
fn a_link_lost_while_reading_faults_is_not_an_empty_fault_memory() {
    let (cable, gone) = pulled_demo();
    let sink = Arc::new(Collect::default());
    let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
    worker.handle(Job::OpenTransport {
        transport: Box::new(cable),
        protocol_order: "6".into(),
    });
    sink.take();
    gone.store(true, Ordering::SeqCst);
    worker.handle(Job::ReadFaults);
    let events = sink.take();
    // Before: "faults" with no rows, shown as "no entries".
    assert!(Collect::named(&events, "faults").is_empty(), "{events:?}");
    assert_eq!(Collect::named(&events, "closed").len(), 1);
    assert!(Collect::named(&events, "failed").is_empty());
}

#[test]
fn a_link_lost_while_clearing_is_not_an_empty_fault_memory() {
    let (cable, gone) = pulled_demo();
    let sink = Arc::new(Collect::default());
    let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
    worker.handle(Job::OpenTransport {
        transport: Box::new(cable),
        protocol_order: "6".into(),
    });
    sink.take();
    gone.store(true, Ordering::SeqCst);
    worker.handle(Job::BruteForceClear(5));
    let events = sink.take();
    // Before: "Memory is empty" and everythingGone true.
    let finished = Collect::named(&events, "clear_finished");
    assert_eq!(finished.len(), 1);
    assert_eq!(finished[0]["everythingGone"], false);
    assert_eq!(Collect::named(&events, "closed").len(), 1);
}

#[test]
fn a_link_lost_in_a_check_gives_no_result() {
    let (cable, gone) = pulled_demo();
    let (handle, sink) = spawned_demo(Box::new(cable));
    handle.send(Job::HealthCheck(60));
    assert!(wait_for(&sink, "live_values", 3, Duration::from_secs(10)));
    gone.store(true, Ordering::SeqCst);
    assert!(wait_for(&sink, "closed", 1, Duration::from_secs(10)));
    assert!(!lost_reason(&sink).is_empty());
    let events = sink.take();
    assert!(Collect::named(&events, "health_result").is_empty());
    assert_eq!(
        Collect::named(&events, "busy").last().unwrap()["active"],
        false
    );
    assert!(handle.shutdown(Duration::from_secs(5)));
}

#[test]
fn a_recording_that_ends_with_the_link_is_kept() {
    let (cable, gone) = pulled_demo();
    let (handle, sink) = spawned_demo(Box::new(cable));
    let path =
        std::env::temp_dir().join(format!("caruspex-link-lost-{}.jsonl", std::process::id()));
    handle.send(Job::StartRecording(path.to_string_lossy().into_owned()));
    handle.send(Job::StartLive(vec![0x0C]));
    assert!(wait_for(&sink, "live_values", 3, Duration::from_secs(10)));
    gone.store(true, Ordering::SeqCst);
    assert!(wait_for(&sink, "closed", 1, Duration::from_secs(10)));
    let states = Collect::named(&sink.take(), "recording_state");
    let last = states.last().unwrap();
    assert_eq!(last["active"], false);
    assert!(last["exchanges"].as_u64().unwrap() > 0);
    assert!(std::fs::metadata(&path).unwrap().len() > 0);
    std::fs::remove_file(&path).ok();
    assert!(handle.shutdown(Duration::from_secs(5)));
}

/// A car that answers the adapter only, and slowly: every protocol search
/// takes its whole time.
struct Searching;

impl Transport for Searching {
    fn write(&mut self, _data: &[u8]) -> bool {
        true
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        std::thread::sleep(Duration::from_millis(timeout_ms));
        Vec::new()
    }
    fn flush_input(&mut self) {}
    fn name(&self) -> String {
        "searching".into()
    }
}

fn cancel_after(job: Job, wait: Duration, limit: Duration) -> Vec<(String, Value)> {
    let sink = Arc::new(Collect::default());
    let handle = spawn(sink.clone(), AppSettings::default());
    handle.send(job);
    std::thread::sleep(wait);
    assert!(wait_for(&sink, "busy", 1, Duration::from_secs(5)));
    let started = Instant::now();
    handle.close();
    assert!(wait_for(&sink, "closed", 1, Duration::from_secs(30)));
    let took = started.elapsed();
    assert!(took < limit, "took {took:?}");
    let events = sink.take();
    // Cancelling is no failure, and it was asked for, so it is no lost link.
    assert!(Collect::named(&events, "failed").is_empty(), "{events:?}");
    assert!(Collect::named(&events, "closed")[0]["lost"].is_null());
    assert_eq!(
        Collect::named(&events, "busy").last().unwrap()["active"],
        false
    );
    // The flags are down again: the next connection works.
    handle.send(Job::OpenTransport {
        transport: quick_demo(),
        protocol_order: "6".into(),
    });
    assert!(wait_for(&sink, "connected", 1, Duration::from_secs(10)));
    assert!(handle.shutdown(Duration::from_secs(5)));
    events
}

#[test]
fn connecting_can_be_cancelled_while_the_adapter_is_silent() {
    cancel_after(
        Job::OpenTransport {
            transport: Box::new(Searching),
            protocol_order: "54312678".into(),
        },
        Duration::from_millis(400),
        Duration::from_millis(700),
    );
}

#[test]
fn connecting_can_be_cancelled_while_nobody_answers_on_the_network() {
    // TEST-NET-1 (RFC 5737): routed nowhere, five seconds until it gives up.
    cancel_after(
        Job::OpenTcp {
            host: "192.0.2.1".into(),
            port: 35000,
            protocol_order: "6".into(),
        },
        Duration::from_millis(300),
        Duration::from_millis(700),
    );
}

#[test]
fn cancelling_the_remembered_way_does_not_start_the_search() {
    let events = cancel_after(
        Job::OpenRemembered {
            route: ConnectRoute {
                kind: "tcp".into(),
                device: "192.0.2.1:35000".into(),
                ..ConnectRoute::default()
            },
            then: Box::new(Job::OpenTransport {
                transport: quick_demo(),
                protocol_order: "6".into(),
            }),
        },
        Duration::from_millis(300),
        Duration::from_millis(700),
    );
    assert!(Collect::named(&events, "connected").is_empty());
}

/// A car that answers no request and only broadcasts frames, like a Tesla.
#[derive(Default)]
struct Broadcasting {
    reply: Vec<u8>,
    monitoring: bool,
}

impl Transport for Broadcasting {
    fn write(&mut self, data: &[u8]) -> bool {
        if std::mem::take(&mut self.monitoring) {
            self.reply.extend_from_slice(b"STOPPED\r\r>");
            return true;
        }
        let command = String::from_utf8_lossy(data).trim().to_ascii_uppercase();
        let answer = match command.as_str() {
            "" => "",
            "ATZ" => "ELM327 v1.5",
            "ATMA" => {
                self.monitoring = true;
                self.reply = b"132 10 96 38 FF\r".repeat(12);
                return true;
            }
            c if c.starts_with("AT") => "OK",
            "STI" => "?",
            _ => "NO DATA",
        };
        self.reply = format!("{answer}\r\r>").into_bytes();
        true
    }
    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        std::mem::take(&mut self.reply)
    }
    fn flush_input(&mut self) {
        self.reply.clear();
    }
    fn name(&self) -> String {
        "broadcasting".into()
    }
}

#[test]
fn a_car_that_only_broadcasts_is_no_failed_fault_memory() {
    let sink = Arc::new(Collect::default());
    let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
    worker.apply_settings(AppSettings::default());
    worker.handle(Job::OpenTransport {
        transport: Box::new(Broadcasting::default()),
        protocol_order: "6".into(),
    });
    let events = sink.take();
    assert_eq!(Collect::named(&events, "connected").len(), 1, "{events:?}");
    assert!(worker.elm.listen_only());
    // Neither a failure (it has no fault memory over OBD) nor "no entries".
    assert!(Collect::named(&events, "failed").is_empty(), "{events:?}");
    assert!(Collect::named(&events, "faults").is_empty());
    assert_eq!(Collect::named(&events, "supported_pids").len(), 1);
    assert!(Collect::named(&events, "closed").is_empty());
}
