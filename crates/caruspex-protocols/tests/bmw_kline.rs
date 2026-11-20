//! BMW's K-line protocols against the simulated E46 (DS2) and E60
//! (KWP2000 in BMW style, BMW-FAST) of caruspex-demo.

use std::collections::VecDeque;

use caruspex_demo::bmw::{BmwCar, BmwSimLine};
use caruspex_link::{ByteStream, LineParity};
use caruspex_protocols::bmw_kline::{
    BmwError, ScanEvent, kwp, no_echo_text, request_allowed, scan_plan,
};
use caruspex_protocols::{BmwKLine, BmwProtocol};

fn scan(car: BmwCar) -> (Vec<caruspex_protocols::BmwUnit>, BmwSimLine) {
    let sim = BmwSimLine::new(car);
    let mut link = BmwKLine::new(sim.connect(), None);
    let units = link.scan(|_| true).expect("the line works");
    (units, sim)
}

/// Every frame the car heard was one the tester may send.
fn only_allowed_requests(sim: &BmwSimLine) {
    for frame in sim.sent_frames() {
        let ds2 = usize::from(frame[1]) == frame.len() && frame.iter().fold(0, |x, &b| x ^ b) == 0;
        let (protocol, data) = if ds2 {
            (BmwProtocol::Ds2, &frame[2..frame.len() - 1])
        } else {
            let header = if frame[0] & 0x3F == 0 { 4 } else { 3 };
            (BmwProtocol::Kwp2000, &frame[header..frame.len() - 1])
        };
        assert!(request_allowed(protocol, data), "{frame:02X?}");
    }
}

#[test]
fn the_e46_scan_finds_every_ds2_unit_with_its_faults() {
    let (units, sim) = scan(BmwCar::E46);
    assert_eq!(units.len(), 9, "{units:#?}");
    assert!(units.iter().all(|u| u.protocol == BmwProtocol::Ds2));

    let dme = units.iter().find(|u| u.address == 0x12).unwrap();
    assert!(dme.name.contains("DME"));
    assert!(dme.identification.starts_with("7519308"));
    assert!(dme.faults_read);
    let codes: Vec<&str> = dme.faults.iter().map(|f| f.code.as_str()).collect();
    assert_eq!(codes, ["0x2A", "0x51"]);
    assert_eq!(dme.faults[0].detail, "01 10 05");
    assert_eq!(dme.faults[0].present, None);

    // The DSC wants a block number (B0 to 04 alone), the cluster is busy
    // at first (A1): both still come through.
    let dsc = units.iter().find(|u| u.address == 0x56).unwrap();
    assert_eq!(dsc.faults.len(), 1);
    let ike = units.iter().find(|u| u.address == 0x80).unwrap();
    assert!(ike.identification.contains("IKE"), "{ike:?}");
    assert!(ike.note.is_empty());

    // Units that are not fitted stay out, also in the KWP2000 passes.
    assert!(!units.iter().any(|u| u.address == 0x60));
    only_allowed_requests(&sim);
    assert_eq!(sim.clear_count(0x12), 0, "a scan never clears");
}

#[test]
fn the_e60_scan_opens_each_unit_with_fast_init() {
    let (units, sim) = scan(BmwCar::E60);
    assert_eq!(units.len(), 8, "{units:#?}");
    assert!(units.iter().all(|u| u.protocol == BmwProtocol::Kwp2000));

    // The identification of the engine is longer than 63 bytes and comes
    // with the extra length byte.
    let dme = units.iter().find(|u| u.address == 0x12).unwrap();
    assert!(dme.identification.contains("MSV70"));
    assert!(dme.identification.contains("YEAR 05"));
    assert_eq!(dme.faults.len(), 2);
    assert_eq!(dme.faults[0].code, "0x2A82");
    assert_eq!(dme.faults[0].present, Some(true));
    assert_eq!(dme.faults[1].present, Some(false));

    // The DSC asks for time (7F 18 78) before its answer.
    let dsc = units.iter().find(|u| u.address == 0x29).unwrap();
    assert!(dsc.faults_read, "{dsc:?}");
    assert_eq!(dsc.faults.len(), 1);
    // Names come from the KWP2000 address map.
    let kombi = units.iter().find(|u| u.address == 0x60).unwrap();
    assert!(kombi.name.contains("KOMBI"));

    // Every address of the KWP2000 pass got a fast init, every session
    // that opened was closed again.
    let frames = sim.sent_frames();
    let starts = frames
        .iter()
        .filter(|f| f.get(3) == Some(&kwp::START_COMMUNICATION))
        .count();
    let stops = frames
        .iter()
        .filter(|f| f.get(3) == Some(&kwp::STOP_COMMUNICATION))
        .count();
    let kwp_addresses = scan_plan()
        .iter()
        .filter(|p| p.0 == BmwProtocol::Kwp2000)
        .count();
    assert_eq!(starts, kwp_addresses);
    assert_eq!(stops, 8);
    only_allowed_requests(&sim);
}

#[test]
fn bmw_fast_needs_no_init() {
    let (units, sim) = scan(BmwCar::E60Fast);
    assert_eq!(units.len(), 8);
    assert!(units.iter().all(|u| u.protocol == BmwProtocol::BmwFast));
    let acsm = units.iter().find(|u| u.address == 0x01).unwrap();
    assert_eq!(acsm.faults[0].code, "0x934B");
    only_allowed_requests(&sim);
}

#[test]
fn clearing_reads_back_what_is_left() {
    let sim = BmwSimLine::new(BmwCar::E46);
    let mut link = BmwKLine::new(sim.connect(), None);
    // The engine keeps the fault whose cause is still there.
    let dme = link.clear_and_check(BmwProtocol::Ds2, 0x12).unwrap();
    assert_eq!(sim.clear_count(0x12), 1);
    assert_eq!(dme.faults.len(), 1);
    assert_eq!(dme.faults[0].code, "0x51");
    let lcm = link.clear_and_check(BmwProtocol::Ds2, 0xD0).unwrap();
    assert!(lcm.faults.is_empty());
    // Nobody at 0x60 of this car.
    assert!(link.clear_and_check(BmwProtocol::Ds2, 0x60).is_err());

    let e60 = BmwSimLine::new(BmwCar::E60);
    let mut link = BmwKLine::new(e60.connect(), None);
    let acsm = link.clear_and_check(BmwProtocol::Kwp2000, 0x01).unwrap();
    assert!(acsm.faults.is_empty());
    assert_eq!(e60.fault_count(0x01), 0);
    only_allowed_requests(&e60);
}

#[test]
fn the_state_of_the_car_survives_a_new_connection() {
    let sim = BmwSimLine::new(BmwCar::E46);
    BmwKLine::new(sim.connect(), None)
        .clear_and_check(BmwProtocol::Ds2, 0xA4)
        .unwrap();
    let mut again = BmwKLine::new(sim.connect(), None);
    let mrs = again.probe(BmwProtocol::Ds2, 0xA4).unwrap().unwrap();
    assert!(mrs.faults.is_empty());
}

#[test]
fn without_echo_the_scan_stops_and_says_why() {
    let sim = BmwSimLine::new(BmwCar::E46);
    sim.set_on_kline(false);
    let mut link = BmwKLine::new(sim.connect(), None);
    let mut tried = 0;
    let result = link.scan(|event| {
        if matches!(event, ScanEvent::Trying { .. }) {
            tried += 1;
        }
        true
    });
    assert_eq!(result.unwrap_err(), no_echo_text());
    assert_eq!(
        tried, 1,
        "the first unit already shows the cable is not on the K-line"
    );
    assert_eq!(
        link.identify(BmwProtocol::Ds2, 0x12, false),
        Err(BmwError::NoEcho)
    );
}

#[test]
fn a_cancelled_scan_stops_after_the_current_unit() {
    let sim = BmwSimLine::new(BmwCar::E46);
    let mut link = BmwKLine::new(sim.connect(), None);
    let mut seen = 0;
    let units = link
        .scan(|event| {
            if matches!(event, ScanEvent::Trying { .. }) {
                seen += 1;
            }
            seen < 3
        })
        .unwrap();
    assert_eq!(seen, 3);
    assert_eq!(units.len(), 2);
}

/// A line where nobody answers; it notes how long each read may wait.
#[derive(Default)]
struct Silent {
    echo: VecDeque<u8>,
    waits: Vec<u64>,
    lines: Vec<(u32, LineParity)>,
    fast_inits: usize,
}

impl ByteStream for Silent {
    fn write_byte(&mut self, value: u8) -> bool {
        self.echo.push_back(value);
        true
    }
    fn read_byte(&mut self, timeout_ms: u64) -> Option<u8> {
        let byte = self.echo.pop_front();
        if byte.is_none() {
            self.waits.push(timeout_ms);
        }
        byte
    }
    fn slow_init(&mut self, _address: u8) -> bool {
        false
    }
    fn fast_init(&mut self, _start: &[u8]) -> Option<Vec<u8>> {
        self.fast_inits += 1;
        None
    }
    fn set_line(&mut self, baud: u32, parity: LineParity) -> bool {
        self.lines.push((baud, parity));
        true
    }
}

#[test]
fn missing_units_cost_only_the_short_scan_timeout() {
    let mut link = BmwKLine::new(Silent::default(), None);
    assert_eq!(link.probe(BmwProtocol::Ds2, 0x12), Ok(None));
    assert_eq!(link.probe(BmwProtocol::BmwFast, 0x12), Ok(None));
    assert_eq!(link.probe(BmwProtocol::Kwp2000, 0x12), Ok(None));
    // Outside the scan the unit gets the full second.
    assert_eq!(
        link.identify(BmwProtocol::Ds2, 0x12, false),
        Err(BmwError::NoAnswer)
    );
    let stream = link.into_stream();
    assert_eq!(stream.waits, [300, 300, 1000]);
    assert_eq!(stream.fast_inits, 1);
    // DS2 at 9600 8E1, BMW-FAST at 115200 8N1, KWP2000 at 10400 8N1.
    assert_eq!(
        stream.lines,
        [
            (9600, LineParity::Even),
            (115200, LineParity::None),
            (10400, LineParity::None),
            (9600, LineParity::Even),
        ]
    );
}

#[test]
fn whole_scan_on_a_silent_line_finds_nothing_quickly() {
    let mut link = BmwKLine::new(Silent::default(), None);
    let started = std::time::Instant::now();
    assert!(link.scan(|_| true).unwrap().is_empty());
    let stream = link.into_stream();
    assert!(stream.waits.iter().all(|&w| w <= 300));
    // The pauses between requests are all that is left without a cable.
    assert!(started.elapsed().as_secs() < 10);
    // The line was switched once per protocol, not per address.
    assert_eq!(stream.lines.len(), 3);
}
