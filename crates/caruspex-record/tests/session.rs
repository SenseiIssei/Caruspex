// Values in the extended diagnostic session (crates/caruspex-record/src/session.rs)
// against the demo engine, which keeps its particulate filter values
// (22 114E, 22 1156) only in that session and leaves it by itself after
// five seconds without a request (S3).

mod common;

use std::sync::Arc;

use caruspex_data::pids::PidCategory;
use caruspex_demo::{DemoClock, DemoOptions, DemoTransport};
use caruspex_link::Elm327;
use caruspex_record::{CustomPid, CustomPidReader, CustomReading, ExtendedSessions};

use common::{Log, capture_log, german, sent_commands};

struct Rig {
    clock: DemoClock,
    elm: Elm327,
    log: Log,
}

impl Rig {
    fn new(refuse: bool) -> Self {
        let clock = DemoClock::new(600.0);
        let options = DemoOptions {
            clock: Some(clock.source()),
            latency_scale: 0.0,
            refuse_extended_session: refuse,
            ..DemoOptions::default()
        };
        let mut elm = Elm327::new();
        elm.set_verbose(true);
        let log = capture_log(&mut elm);
        elm.connect(Box::new(DemoTransport::new(options)), "6")
            .expect("demo connects");
        Rig { clock, elm, log }
    }

    /// Sessions on the same clock as the demo car.
    fn sessions(&self) -> ExtendedSessions {
        let clock = self.clock.clone();
        ExtendedSessions::with_clock(Arc::new(move || (clock.get() * 1000.0) as u64))
    }

    /// The commands sent since the last call.
    fn take(&self) -> Vec<String> {
        let mut log = self.log.lock().unwrap();
        let out = sent_commands(&log);
        log.clear();
        out
    }

    fn read(
        &mut self,
        pids: &[CustomPid],
        sessions: Option<&mut ExtendedSessions>,
    ) -> Vec<CustomReading> {
        let mut reader = CustomPidReader::new(&mut self.elm);
        reader.read_all_with(pids, sessions)
    }
}

fn pid(name: &str, request: &str, equation: &str, session: u8, key: i32) -> CustomPid {
    CustomPid {
        name: name.to_string(),
        short_name: name.to_string(),
        request: request.to_string(),
        header: "7E0".to_string(),
        equation: equation.to_string(),
        key,
        category: PidCategory::Custom,
        session,
        ..CustomPid::default()
    }
}

/// The two session values as OBDb writes them for the VAG diesels, and an
/// ordinary engine value of the same unit.
fn values() -> Vec<CustomPid> {
    vec![
        pid(
            "Soot mass measured",
            "22114E",
            "SIGNED16(INT16(A:B))/100",
            3,
            0x10_0101,
        ),
        pid(
            "Distance since last regeneration",
            "221156",
            "INT32(A:B:C:D)/1000",
            3,
            0x10_0102,
        ),
        pid("Oil level", "221001", "A", 0, 0x10_0103),
    ]
}

fn requests(sent: &[String]) -> Vec<String> {
    sent.iter()
        .filter(|c| !c.starts_with("AT"))
        .cloned()
        .collect()
}

#[test]
fn enter_read_keep_and_leave_in_this_order() {
    let _lang = german();
    let mut rig = Rig::new(false);
    let mut sessions = rig.sessions();
    rig.take();

    let r = rig.read(&values(), Some(&mut sessions));
    assert!(
        r.iter().all(|r| r.ok),
        "{:?}",
        r.iter().map(|r| &r.error).collect::<Vec<_>>()
    );
    assert!(r[0].session && r[1].session && !r[2].session);
    assert!(
        (9.0..=15.0).contains(&r[0].value.value),
        "{}",
        r[0].value.value
    );
    assert!(r[1].value.value >= 0.0 && r[1].value.value < 412.0);
    // Enter first, then the reads; the ordinary value of the same unit goes
    // through the same path.
    assert_eq!(
        requests(&rig.take()),
        ["1003", "22114E", "221156", "221001"]
    );
    assert!(sessions.is_active("7E0/7E8"));

    // Soon after: no keep-alive needed, the reads keep the S3 timer going.
    rig.clock.advance(1.0);
    rig.read(&values(), Some(&mut sessions));
    assert_eq!(requests(&rig.take()), ["22114E", "221156", "221001"]);

    // After a pause of three seconds: tester present first.
    rig.clock.advance(3.0);
    let r = rig.read(&values(), Some(&mut sessions));
    assert!(r[0].ok && r[1].ok);
    assert_eq!(
        requests(&rig.take()),
        ["3E00", "22114E", "221156", "221001"]
    );

    // Nothing reads them any more: 10 01, and the engine is in its default
    // session again (the value is unknown there).
    sessions.leave_all(&mut rig.elm);
    assert_eq!(requests(&rig.take()), ["1001"]);
    assert!(!sessions.any_active());
    let r = rig.read(&values()[..1], None);
    assert!(!r[0].ok);
    assert!(
        requests(&rig.take()).is_empty(),
        "without sessions nothing is entered"
    );
}

#[test]
fn a_session_the_engine_left_by_itself_is_entered_again() {
    let _lang = german();
    let mut rig = Rig::new(false);
    let mut sessions = rig.sessions();
    rig.read(&values(), Some(&mut sessions));
    rig.take();

    // A long pause: the reader knows the engine has left and enters again
    // instead of sending tester present.
    rig.clock.advance(8.0);
    let r = rig.read(&values(), Some(&mut sessions));
    assert!(r[0].ok && r[1].ok);
    assert_eq!(
        requests(&rig.take()),
        ["1003", "22114E", "221156", "221001"]
    );

    // The engine left earlier than the reader thought (its clock runs on,
    // the reader's does not): the refusal 7F 22 31 enters once more.
    let frozen = rig.clock.get();
    let mut stuck = ExtendedSessions::with_clock(Arc::new(move || (frozen * 1000.0) as u64));
    rig.read(&values(), Some(&mut stuck));
    rig.take();
    rig.clock.advance(6.0);
    let r = rig.read(&values(), Some(&mut stuck));
    assert!(r[0].ok && r[1].ok, "{}", r[0].error);
    assert_eq!(
        requests(&rig.take()),
        ["22114E", "1003", "22114E", "221156", "221001"]
    );
}

#[test]
fn a_refused_session_shows_why_and_is_not_asked_again_at_once() {
    let _lang = german();
    let mut rig = Rig::new(true);
    let mut sessions = rig.sessions();
    rig.take();

    let r = rig.read(&values(), Some(&mut sessions));
    assert!(!r[0].ok && !r[1].ok);
    assert!(r[0].error.contains("nicht zu"), "{}", r[0].error);
    assert!(r[0].error.contains("30 Sekunden"), "{}", r[0].error);
    // The ordinary value is read normally.
    assert!(r[2].ok);
    assert_eq!(requests(&rig.take()), ["1003", "221001"]);
    assert!(!sessions.any_active());

    // Every tick for the next half minute: no new 10 03, the same reason.
    for _ in 0..10 {
        rig.clock.advance(2.0);
        let r = rig.read(&values(), Some(&mut sessions));
        assert!(r[0].error.contains("nicht zu"));
    }
    assert_eq!(requests(&rig.take()), vec!["221001"; 10]);

    // After 30 seconds it is tried again.
    rig.clock.advance(11.0);
    rig.read(&values(), Some(&mut sessions));
    assert_eq!(requests(&rig.take()), ["1003", "221001"]);
    // Nothing to leave.
    sessions.leave_all(&mut rig.elm);
    assert!(requests(&rig.take()).is_empty());
}

#[test]
fn switched_off_the_values_say_so_and_nothing_is_entered() {
    let _lang = german();
    let mut rig = Rig::new(false);
    let mut sessions = rig.sessions();
    sessions.set_enabled(false, "In den Einstellungen ausgeschaltet.");
    rig.take();
    let r = rig.read(&values(), Some(&mut sessions));
    assert_eq!(r[0].error, "In den Einstellungen ausgeschaltet.");
    assert!(r[2].ok);
    assert_eq!(requests(&rig.take()), ["221001"]);

    // The command line keeps no sessions.
    let r = rig.read(&values(), None);
    assert!(
        r[0].error.contains("erweiterte Diagnosesitzung"),
        "{}",
        r[0].error
    );
    assert_eq!(requests(&rig.take()), ["221001"]);
}
