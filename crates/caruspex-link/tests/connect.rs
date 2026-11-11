//! Connecting and talking through scripted adapters: the Elm327 parts of
//! tests/test_protocol.cpp, test_settings.cpp, test_obd_on_uds.cpp,
//! test_can_signals.cpp and the timing cases of test_recording.cpp.

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use caruspex_i18n::Language;
use caruspex_link::{AdaptiveTiming, Elm327, ElmStatus, StnBus, Transport};
use common::*;

type Log = Arc<Mutex<Vec<String>>>;

fn connect(transport: impl Transport + 'static, order: &str) -> Elm327 {
    let mut elm = Elm327::new();
    elm.connect(Box::new(transport), order).expect("connects");
    elm
}

// --- test_protocol.cpp ------------------------------------------------------

#[test]
fn oxygen_sensor_test_and_k_line_monitoring_answers_come_through() {
    // The Elm327 half of "Oxygen sensor test and K-line monitoring are
    // decoded"; the values themselves are Obd2's job.
    let mut elm = connect(FaultTransport::default(), "6");
    let o2 = elm.request("050101", 0x05, 6000);
    assert_eq!(o2.messages, [vec![0x01, 0x01, 0x8C]]);
    let monitor = elm.request("0601", 0x06, 6000);
    assert_eq!(
        monitor.messages,
        [vec![0x01, 0x01, 0x00, 0x96, 0x00, 0x50, 0x00, 0xC8]]
    );
}

#[test]
fn core_texts_follow_the_active_language() {
    // The Elm327 half of the C++ case: connecting works in both languages and
    // the driver's own texts switch with the language.
    {
        let _german = language(Language::German);
        let mut elm = connect(FaultTransport::default(), "6");
        let faults = elm.request("03", 0x03, 6000);
        assert_eq!(faults.messages, [vec![0x05, 0x07, 0x00, 0x00, 0x00, 0x00]]);
        elm.disconnect();
        assert_eq!(elm.protocol_text(), "unbekannt");
        assert_eq!(elm.send("ATI", 100).message, "Kein Adapter verbunden.");
    }
    let _english = language(Language::English);
    let mut elm = connect(FaultTransport::default(), "6");
    assert_eq!(
        elm.request("07", 0x07, 6000).messages,
        [vec![0x01, 0x21, 0, 0, 0, 0]]
    );
    let none = elm.request("0902", 0x09, 6000);
    assert!(none.message.contains("does not support"));
    elm.disconnect();
    assert_eq!(elm.protocol_text(), "unknown");
}

// --- test_settings.cpp --------------------------------------------------------

struct Fixture {
    elm: Elm327,
    silent: Arc<AtomicBool>,
    lines: Log,
}

impl Fixture {
    fn new() -> Self {
        let transport = SettingsTransport::default();
        let silent = transport.silent.clone();
        let mut elm = Elm327::new();
        let lines = capture_log(&mut elm);
        elm.connect(Box::new(transport), "6").expect("connects");
        Self { elm, silent, lines }
    }

    /// How many lines show the raw communication.
    fn raw_lines(&self) -> usize {
        self.lines
            .lock()
            .unwrap()
            .iter()
            .filter(|l| l.starts_with("  > "))
            .count()
    }
}

#[test]
fn the_raw_log_can_really_be_switched_off() {
    let mut f = Fixture::new();

    f.elm.set_verbose(true);
    f.lines.lock().unwrap().clear();
    f.elm.request("0105", 0x01, 6000);
    let with_log = f.raw_lines();
    assert!(with_log > 0);

    f.elm.set_verbose(false);
    f.lines.lock().unwrap().clear();
    f.elm.request("0105", 0x01, 6000);
    assert_eq!(f.raw_lines(), 0);

    // And on again, so the setting works in both directions.
    f.elm.set_verbose(true);
    f.lines.lock().unwrap().clear();
    f.elm.request("0105", 0x01, 6000);
    assert_eq!(f.raw_lines(), with_log);
}

#[test]
fn the_timeout_factor_stretches_the_waiting_time() {
    let mut f = Fixture::new();
    f.silent.store(true, Ordering::SeqCst); // from now on nothing comes back

    let mut measure = |scale: f64, timeout_ms: u64| {
        f.elm.set_timeout_scale(scale);
        let start = Instant::now();
        f.elm.send("0105", timeout_ms);
        start.elapsed().as_millis()
    };

    // A quarter of 800 ms is 200 ms, double is 1600 ms. The bounds are
    // deliberately wide so the test does not fail for no reason on a busy
    // machine.
    let quarter = measure(0.25, 800);
    let doubled = measure(2.0, 800);

    assert!(quarter < 600);
    assert!(doubled > 1200);
    assert!(doubled > quarter * 2);
}

#[test]
fn a_nonsensical_factor_is_caught() {
    let mut f = Fixture::new();
    f.silent.store(true, Ordering::SeqCst);

    // Zero or negative would push every timeout to zero and make every
    // request fail immediately. set_timeout_scale catches that.
    f.elm.set_timeout_scale(0.0);
    let start = Instant::now();
    let r = f.elm.send("0105", 300);
    assert!(start.elapsed().as_millis() >= 250);
    assert_eq!(r.status, ElmStatus::AdapterError);
}

// --- test_obd_on_uds.cpp ------------------------------------------------------

#[test]
fn obd_on_uds_detected_at_connect_classic_results_from_uds_answers() {
    // The C++ case reads through Obd2; here the same answers are checked one
    // level lower, in the classic form Obd2 receives.
    for headers in [false, true] {
        let car = scripted_uds_car();
        let sent = car.sent.clone();
        let mut elm = connect(car, "6");
        assert!(elm.obd_on_uds(), "headers {headers}");
        if headers {
            assert!(elm.set_headers(true));
        }
        let source = if headers { "7E8" } else { "" };

        let supported = elm.request("0100", 0x01, 6000);
        assert_eq!(supported.messages, [vec![0x00, 0xBE, 0x3F, 0xA8, 0x13]]);
        assert_eq!(supported.sources, [source]);

        let rpm = elm.request("010C", 0x01, 6000);
        assert_eq!(rpm.messages, [vec![0x0C, 0x0C, 0x80]]); // 0x0C80 / 4 = 800
        // After one "busy" answer.
        let coolant = elm.request("0105", 0x01, 6000);
        assert_eq!(coolant.messages, [vec![0x05, 0x7B]]); // 0x7B - 40 = 83
        assert_eq!(UdsOnlyCar::count(&sent, "22F405"), 2);

        // Service 09 gets its count byte back.
        let vin = elm.request("0902", 0x09, 6000);
        assert_eq!(vin.messages.len(), 1);
        assert_eq!(vin.messages[0][..2], [0x02, 0x01]);
        assert_eq!(
            String::from_utf8(vin.messages[0][2..].to_vec()).unwrap(),
            "WAUZZZ8V5LA012345"
        );

        // Confirmed: P0171 and P0420, count first as on CAN.
        let stored = elm.request("03", 0x03, 6000);
        assert_eq!(stored.messages, [vec![2, 0x01, 0x71, 0x04, 0x20]]);
        let pending = elm.request("07", 0x07, 6000);
        assert_eq!(pending.messages, [vec![0]]);
        let permanent = elm.request("0A", 0x0A, 6000);
        assert_eq!(permanent.messages, [vec![1, 0x03, 0x00]]); // P0300

        let cleared = elm.request("04", 0x04, 6000);
        assert!(cleared.ok());
        assert_eq!(UdsOnlyCar::count(&sent, "14FFFF33"), 1);
        // No classic service was sent after the connection was made.
        assert_eq!(UdsOnlyCar::count(&sent, "0300"), 0);
        assert_eq!(UdsOnlyCar::count(&sent, "03"), 0);
    }
}

#[test]
fn obd_on_uds_freeze_frames_and_test_results_say_no_data() {
    let mut elm = connect(scripted_uds_car(), "6");
    assert_eq!(elm.request("020C00", 0x02, 6000).status, ElmStatus::NoData);
    assert_eq!(elm.request("0600", 0x06, 6000).status, ElmStatus::NoData);
}

#[test]
fn a_classic_car_stays_classic() {
    // The UDS probe only runs when 0100 fails.
    let mut car = UdsOnlyCar::default();
    car.replies.insert("0100".into(), Vec::new());
    let sent = car.sent.clone();
    let mut elm = Elm327::new();
    // Refuses 0100, no UDS answer either.
    assert!(elm.connect(Box::new(car), "6").is_err());
    assert!(UdsOnlyCar::count(&sent, "22F400") >= 1);
    assert!(!elm.obd_on_uds());
}

// --- test_can_signals.cpp -----------------------------------------------------

#[test]
fn a_car_without_obd_answers_but_with_bus_traffic_is_connected_for_listening() {
    // The Elm327 half; the signals and the Tesla pack belong to other crates.
    let _english = language(Language::English);
    for headers in [false, true] {
        let mut elm = connect(tesla(), "6");
        assert!(elm.listen_only(), "headers {headers}");
        assert!(elm.protocol_text().contains("listen only"));
        if headers {
            assert!(elm.set_headers(true));
        }
        let frames = elm.capture_frames(0x332, 300, 10);
        assert!(frames.len() >= 2);
        assert_eq!(frames[0].len(), 6);
        assert_eq!(frames[0], [0x09, 0x20, 0xF8, 0x07, 0x00, 0x00]);

        let vin_parts = elm.capture_frames(0x405, 300, 10);
        assert_eq!(vin_parts[0][0], 0x10);
    }
}

#[test]
fn an_stn_chip_listens_with_stma_and_captures_one_id_with_a_pass_filter() {
    let mut bus = tesla();
    bus.stn = true;
    let sent = bus.sent.clone();
    let mut elm = connect(bus, "6");
    assert!(elm.listen_only());
    assert!(elm.is_stn());
    assert!(elm.stn_batch());
    // A listen-only connection offers no bus switching.
    assert!(elm.stn_extra_buses().is_empty());

    // Four frames asked: the chip stops by itself after them.
    let frames = elm.capture_frames(0x332, 300, 4);
    assert_eq!(frames.len(), 4);
    assert_eq!(frames[0], [0x09, 0x20, 0xF8, 0x07, 0x00, 0x00]);
    assert_eq!(frames[1], [0x00, 0x00, 0x82, 0x78, 0x00, 0x00]);
    let vin_parts = elm.capture_frames(0x405, 300, 3);
    assert_eq!(vin_parts.len(), 3);
    assert_eq!(vin_parts[2][0], 0x12);

    let sent = sent.lock().unwrap().clone();
    assert!(sent.contains(&"STMA12".to_string()), "{sent:?}");
    assert!(sent.contains(&"STFPA0332,07FF".to_string()), "{sent:?}");
    assert!(sent.contains(&"STM4".to_string()), "{sent:?}");
    assert!(!sent.iter().any(|c| c == "ATMA"), "{sent:?}");
    // Afterwards everything is back to the automatic filters.
    assert_eq!(sent[sent.len() - 2..], ["ATCAF1", "ATAR"]);
}

#[test]
fn bus_switching_needs_a_chip_with_the_transceivers() {
    let mut elm = connect(FakeAdapter::new(DemoAdapter::ObdLinkStn, true, 0.0), "6");
    assert!(elm.is_stn());
    // The OBDLink SX (STN1130) has only the high speed CAN.
    assert!(elm.stn_extra_buses().is_empty());
    assert!(elm.switch_stn_bus(StnBus::MediumSpeed).is_err());
    assert_eq!(elm.stn_bus(), StnBus::HighSpeed);
    // Without an STN chip there is nothing to switch at all.
    let mut clone = connect(FakeAdapter::new(DemoAdapter::CloneV15, true, 0.0), "6");
    assert!(clone.switch_stn_bus(StnBus::SingleWire).is_err());
    assert!(!clone.stn_batch());
}

// --- test_recording.cpp ([timing]) --------------------------------------------

fn connect_with(mode: AdaptiveTiming, adapter: DemoAdapter, rule: Option<Rule>) -> (Elm327, Log) {
    let mut elm = Elm327::new();
    elm.set_verbose(true);
    let log = capture_log(&mut elm);
    elm.set_adaptive_timing(mode);
    let mut t: Box<dyn Transport> = Box::new(FakeAdapter::new(adapter, true, 0.0));
    if let Some(rule) = rule {
        t = Box::new(InterceptTransport::new(t, rule));
    }
    elm.connect(t, "6").expect("connects");
    (elm, log)
}

fn sent_of(log: &Log) -> Vec<String> {
    sent_commands(&log.lock().unwrap())
}

#[test]
fn adaptive_timing_standard_stays_with_atat1() {
    let (elm, log) = connect_with(AdaptiveTiming::Standard, DemoAdapter::CloneV15, None);
    assert_eq!(elm.adaptive_timing_level(), 1);
    assert!(!sent_of(&log).iter().any(|c| c == "ATAT2"));
}

#[test]
fn adaptive_timing_aggressive_is_accepted() {
    let (elm, log) = connect_with(AdaptiveTiming::Aggressive, DemoAdapter::CloneV15, None);
    assert_eq!(elm.adaptive_timing_level(), 2);
    let sent = sent_of(&log);
    // ATAT2 only after the protocol was found.
    let at2 = sent.iter().position(|c| c == "ATAT2").expect("ATAT2 sent");
    let sp = sent.iter().rposition(|c| c == "ATSP6").expect("ATSP6 sent");
    assert!(at2 > sp);
}

#[test]
fn adaptive_timing_refused_atat2_falls_back_to_atat1() {
    let _english = language(Language::English);
    let rule: Rule = Box::new(|c: &str| (c == "ATAT2").then(|| b"?\r\r>".to_vec()));
    let (mut elm, log) = connect_with(
        AdaptiveTiming::Aggressive,
        DemoAdapter::CloneV15,
        Some(rule),
    );
    assert_eq!(elm.adaptive_timing_level(), 1);
    assert_eq!(sent_of(&log).last().map(String::as_str), Some("ATAT1"));
    assert!(
        log.lock()
            .unwrap()
            .iter()
            .any(|l| l.contains("ATAT2 refused"))
    );
    // The session works as before.
    assert!(elm.request("0100", 0x01, 6000).ok());
}

#[test]
fn adaptive_timing_missing_answers_under_atat2_fall_back_to_atat1() {
    let mut aggressive = false;
    let rule: Rule = Box::new(move |c: &str| {
        if c == "ATAT2" {
            aggressive = true;
        }
        if c == "ATAT1" {
            aggressive = false;
        }
        (aggressive && c == "0100").then(|| b"NO DATA\r\r>".to_vec())
    });
    let (elm, log) = connect_with(
        AdaptiveTiming::Aggressive,
        DemoAdapter::CloneV15,
        Some(rule),
    );
    assert_eq!(elm.adaptive_timing_level(), 1);
    assert_eq!(sent_of(&log).last().map(String::as_str), Some("ATAT1"));
}

#[test]
fn adaptive_timing_auto_uses_atat2_only_on_stn_chips() {
    let (clone, _) = connect_with(AdaptiveTiming::Auto, DemoAdapter::CloneV15, None);
    assert_eq!(clone.adaptive_timing_level(), 1);
    assert!(!clone.is_stn());
    let (stn, _) = connect_with(AdaptiveTiming::Auto, DemoAdapter::ObdLinkStn, None);
    assert!(stn.is_stn());
    assert_eq!(stn.adaptive_timing_level(), 2);
}

#[test]
fn adaptive_timing_adapter_without_adaptive_timing() {
    let rule: Rule = Box::new(|c: &str| c.starts_with("ATAT").then(|| b"?\r\r>".to_vec()));
    let (elm, log) = connect_with(
        AdaptiveTiming::Aggressive,
        DemoAdapter::CloneV15,
        Some(rule),
    );
    assert_eq!(elm.adaptive_timing_level(), 0);
    assert!(!sent_of(&log).iter().any(|c| c == "ATAT2"));
}

#[test]
fn stn_chips_are_detected_during_connect() {
    for (adapter, stn) in [
        (DemoAdapter::ObdLinkStn, true),
        (DemoAdapter::CloneV15, false),
        (DemoAdapter::CloneOkToAll, false),
        (DemoAdapter::GenuineV14b, false),
    ] {
        let mut elm = connect(FakeAdapter::new(adapter, true, 0.0), "6");
        assert_eq!(elm.is_stn(), stn, "{adapter:?}");
        if stn {
            assert_eq!(elm.stn_id(), "STN1130 v5.6.19");
        } else {
            assert!(elm.stn_id().is_empty());
        }
        elm.disconnect();
        assert!(!elm.is_stn());
        assert_eq!(elm.adaptive_timing_level(), 0);
    }
}

// --- Driver details without a C++ case of their own ---------------------------

#[test]
fn the_fake_demo_adapter_connects_with_echo_and_greeting() {
    let mut elm = connect(FakeAdapter::new(DemoAdapter::GenuineV14b, true, 0.0), "6");
    assert!(elm.connected());
    assert_eq!(elm.version(), "ELM327 v1.4b");
    assert_eq!(elm.protocol_text(), "ISO 15765-4 (CAN 11/500)");
    assert_eq!(elm.protocol_id(), '6');
    assert!(elm.is_can());
    // Both control units answer; with headers their addresses come along.
    assert_eq!(elm.request("0100", 0x01, 6000).messages.len(), 2);
    assert!(elm.set_headers(true));
    let r = elm.request("0100", 0x01, 6000);
    assert_eq!(r.sources, ["7E8", "7E9"]);
    // Headers typed by hand are tracked too.
    elm.send("ATH0", 1000);
    assert!(!elm.headers_on());
    // Monitoring a silent bus stops cleanly and leaves the adapter usable.
    assert!(elm.monitor_lines(100, 5).is_empty());
    assert!(elm.request("0100", 0x01, 6000).ok());
}

#[test]
fn a_failed_connect_leaves_a_trace_for_the_assistant() {
    // The adapter answers, the car does not: the K-line init fails.
    let rule = |c: &str| (c == "0100").then(|| b"BUS INIT: ...ERROR\r\r>".to_vec());
    let t = InterceptTransport::new(
        Box::new(FakeAdapter::new(DemoAdapter::CloneV15, true, 0.0)),
        rule,
    );
    let mut elm = Elm327::new();
    assert!(elm.connect(Box::new(t), "3").is_err());
    elm.disconnect();
    let trace = elm.connect_trace();
    assert!(trace.adapter_answered);
    assert_eq!(trace.adapter_id, "ELM327 v1.5");
    assert_eq!(trace.protocols_tried, "3");
    assert_eq!(trace.bus_error, "BUS INIT: ...ERROR");

    // A good connect starts a fresh trace.
    let elm = connect(FakeAdapter::new(DemoAdapter::GenuineV14b, true, 0.0), "6");
    let trace = elm.connect_trace();
    assert_eq!(trace.protocols_tried, "6");
    assert!(trace.bus_error.is_empty());
}

#[test]
fn wrapping_the_transport_keeps_the_session() {
    let mut elm = connect(FaultTransport::default(), "6");
    let seen: Log = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    elm.wrap_transport(move |inner| {
        let wrapped = InterceptTransport::new(inner, move |c: &str| {
            sink.lock().unwrap().push(c.to_string());
            None
        });
        Some(Box::new(wrapped))
    });
    assert!(elm.connected());
    assert!(elm.request("03", 0x03, 6000).ok());
    assert!(seen.lock().unwrap().iter().any(|c| c == "03"));
    elm.wrap_transport(|_| None);
    assert!(!elm.connected());
}
