//! VAG TP 2.0 against a simulated control unit on a fake CAN interface:
//! channel setup and parameters, segmentation with block size and
//! acknowledgements in both directions, timeouts, the keep-alive, closing
//! the channel from either side, and KWP2000 on top (identification, fault
//! memory, measuring blocks, basic settings). The simulated unit follows the
//! public description of TP 2.0, independently of the code under test.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use caruspex_link::MessageLink;
use caruspex_net::can_bus::{CanElm, CanFrame, CanLink};
use caruspex_net::tp20::Tp20;
use caruspex_protocols::kwp1281::Ecu;
use caruspex_protocols::kwp2000::{KLineProtocol, session};
use caruspex_protocols::vag_kline::VagDiagnosis;
use caruspex_protocols::vag_tp20::VagTp20;

const UNIT_RX: u32 = 0x740;

/// One control unit behind the gateway.
#[derive(Clone)]
struct Unit {
    ident: String,
    /// (fault number, status)
    faults: Vec<(u16, u8)>,
    /// Block size the unit asks the tester for in A1.
    tester_block: u8,
    /// Packets the unit sends before it wants an acknowledgement.
    own_block: usize,
}

impl Default for Unit {
    fn default() -> Self {
        Self {
            ident: "4F0907401A  3.2l V6 FSI     0030".into(),
            // 0x4066 = 16486 (P0102), 0x0231 = 561
            faults: vec![(0x4066, 0xE0), (0x0231, 0x60)],
            tester_block: 15,
            own_block: 15,
        }
    }
}

/// The open channel, seen from the unit.
struct Open {
    address: u8,
    /// The unit sends here (what the tester asked for).
    tx: u32,
    expect_seq: u8,
    next_seq: u8,
    inbound: Vec<u8>,
    /// Packets of an answer still to go out, waiting for an acknowledgement.
    outbound: VecDeque<Vec<u8>>,
    last_heard: Instant,
}

#[derive(Default)]
struct Car {
    units: HashMap<u8, Unit>,
    refusing: Vec<u8>,
    open: Option<Open>,
    out: VecDeque<CanFrame>,
    /// Everything the tester sent.
    heard: Vec<CanFrame>,
    /// The unit forgets the channel after this much silence.
    idle_limit: Option<Duration>,
    /// Acknowledge the tester's packets (off: a unit that hangs).
    no_acks: bool,
    /// Answer A3.
    no_channel_test: bool,
    /// "Response pending" before the next answer.
    pending_once: bool,
    /// Close the channel with A8 instead of answering the next request.
    hang_up_next: bool,
    sessions: Vec<u8>,
}

impl Car {
    fn with_engine() -> Self {
        let mut car = Car::default();
        car.units.insert(0x01, Unit::default());
        car
    }

    fn send(&mut self, id: u32, data: Vec<u8>) {
        self.out.push_back(CanFrame {
            id,
            extended: false,
            data,
        });
    }

    fn heard(&mut self, f: &CanFrame) {
        self.heard.push(f.clone());
        if f.id == 0x200 {
            self.setup(f);
            return;
        }
        if f.id != UNIT_RX || self.open.is_none() {
            return;
        }
        if let Some(limit) = self.idle_limit
            && self.open.as_ref().unwrap().last_heard.elapsed() > limit
        {
            self.open = None; // timed out, the unit is gone
            return;
        }
        self.open.as_mut().unwrap().last_heard = Instant::now();
        let (address, tx) = {
            let o = self.open.as_ref().unwrap();
            (o.address, o.tx)
        };
        let unit = self.units[&address].clone();
        let first = f.data[0];
        match first {
            0xA0 => self.send(tx, vec![0xA1, unit.tester_block, 0x8A, 0xFF, 0x32, 0xFF]),
            0xA3 => {
                if !self.no_channel_test {
                    self.send(tx, vec![0xA1, unit.tester_block, 0x8A, 0xFF, 0x32, 0xFF]);
                }
            }
            0xA8 => {
                self.send(tx, vec![0xA8]);
                self.open = None;
            }
            _ => match first >> 4 {
                0xB => self.flush(&unit),
                0x0..=0x3 => self.packet(f, &unit),
                _ => {}
            },
        }
    }

    fn setup(&mut self, f: &CanFrame) {
        let address = f.data[0];
        assert_eq!(f.data[1], 0xC0, "setup request");
        assert_eq!(f.data[6], 0x01, "application type KWP2000");
        if self.refusing.contains(&address) {
            self.send(
                0x200 + u32::from(address),
                vec![0x00, 0xD7, 0, 0x10, 0, 0x10, 0x01],
            );
            return;
        }
        if !self.units.contains_key(&address) {
            return; // not fitted
        }
        // The tester asked the unit to send on the id in bytes 4 and 5.
        assert_eq!(f.data[5] & 0x10, 0, "tester names its id");
        let tx = u32::from(f.data[4]) | (u32::from(f.data[5] & 0x07) << 8);
        self.open = Some(Open {
            address,
            tx,
            expect_seq: 0,
            next_seq: 0,
            inbound: Vec::new(),
            outbound: VecDeque::new(),
            last_heard: Instant::now(),
        });
        let rx = UNIT_RX.to_le_bytes();
        self.send(
            0x200 + u32::from(address),
            vec![0x00, 0xD0, tx as u8, (tx >> 8) as u8, rx[0], rx[1], 0x01],
        );
    }

    fn packet(&mut self, f: &CanFrame, unit: &Unit) {
        let kind = f.data[0] >> 4;
        let seq = f.data[0] & 0x0F;
        let tx = self.open.as_ref().unwrap().tx;
        {
            let o = self.open.as_mut().unwrap();
            assert_eq!(seq, o.expect_seq, "tester packets counted in order");
            o.expect_seq = (seq + 1) & 0x0F;
            o.inbound.extend_from_slice(&f.data[1..]);
        }
        if (kind == 0 || kind == 1) && !self.no_acks {
            let next = self.open.as_ref().unwrap().expect_seq;
            self.send(tx, vec![0xB0 | next]);
        }
        if kind == 1 || kind == 3 {
            let data = std::mem::take(&mut self.open.as_mut().unwrap().inbound);
            let length = usize::from(u16::from_be_bytes([data[0], data[1]]));
            assert!(data.len() >= 2 + length, "length matches the packets");
            self.request(&data[2..2 + length], unit);
        }
    }

    fn request(&mut self, payload: &[u8], unit: &Unit) {
        if self.hang_up_next {
            self.hang_up_next = false;
            let tx = self.open.as_ref().unwrap().tx;
            self.send(tx, vec![0xA8]);
            self.open = None;
            return;
        }
        if self.pending_once {
            self.pending_once = false;
            self.answer(vec![0x7F, payload[0], 0x78], unit);
        }
        let address = self.open.as_ref().unwrap().address;
        let answer = match payload[0] {
            0x10 => {
                self.sessions.push(payload[1]);
                vec![0x50, payload[1]]
            }
            0x1A => {
                let mut a = vec![0x5A, 0x9B];
                a.extend_from_slice(unit.ident.as_bytes());
                a
            }
            0x18 => {
                let faults = &self.units[&address].faults;
                let mut a = vec![0x58, faults.len() as u8];
                for (code, status) in faults {
                    a.extend_from_slice(&code.to_be_bytes());
                    a.push(*status);
                }
                a
            }
            0x14 => {
                self.units.get_mut(&address).unwrap().faults.clear();
                vec![0x54, 0xFF, 0x00]
            }
            // Group: engine speed 800 /min (formula 1: a*b/5), coolant 90 C
            // (formula 5: a*(b-100)/10), and two more fields.
            0x21 => vec![
                0x61, payload[1], 0x01, 0xC8, 0x14, 0x05, 0x0A, 0xBE, 0x01, 0xC8, 0x14, 0x05, 0x0A,
                0xBE,
            ],
            0x3E => vec![0x7E],
            // Anything else comes back whole: a long echo.
            service => {
                let mut a = vec![service + 0x40];
                a.extend_from_slice(&payload[1..]);
                a
            }
        };
        self.answer(answer, unit);
    }

    /// Splits an answer into packets; they go out block by block.
    fn answer(&mut self, message: Vec<u8>, unit: &Unit) {
        let mut data = (message.len() as u16).to_be_bytes().to_vec();
        data.extend_from_slice(&message);
        let chunks: Vec<Vec<u8>> = data.chunks(7).map(<[u8]>::to_vec).collect();
        let n = chunks.len();
        let o = self.open.as_mut().unwrap();
        for (i, chunk) in chunks.into_iter().enumerate() {
            let last = i + 1 == n;
            let block_end = (i + 1) % unit.own_block == 0;
            let kind: u8 = match (last, block_end) {
                (true, _) => 0x1,
                (false, true) => 0x0,
                (false, false) => 0x2,
            };
            let mut p = vec![(kind << 4) | o.next_seq];
            o.next_seq = (o.next_seq + 1) & 0x0F;
            p.extend_from_slice(&chunk);
            o.outbound.push_back(p);
        }
        self.flush(unit);
    }

    /// Sends packets up to the next one that wants an acknowledgement.
    fn flush(&mut self, _unit: &Unit) {
        let Some(o) = self.open.as_mut() else {
            return;
        };
        let tx = o.tx;
        let mut frames = Vec::new();
        while let Some(p) = o.outbound.pop_front() {
            let wants_ack = p[0] >> 4 == 0x0 || p[0] >> 4 == 0x1;
            frames.push(p);
            if wants_ack {
                break;
            }
        }
        for p in frames {
            self.send(tx, p);
        }
    }
}

#[derive(Clone)]
struct FakeBus(Arc<Mutex<Car>>);

impl FakeBus {
    fn new(car: Car) -> Self {
        Self(Arc::new(Mutex::new(car)))
    }
    fn car(&self) -> std::sync::MutexGuard<'_, Car> {
        self.0.lock().unwrap()
    }
    fn tp20(&self) -> Tp20 {
        Tp20::new(Box::new(self.clone()))
    }
    /// What the tester sent to an id, in order.
    fn sent_to(&self, id: u32) -> Vec<Vec<u8>> {
        self.car()
            .heard
            .iter()
            .filter(|f| f.id == id)
            .map(|f| f.data.clone())
            .collect()
    }
}

impl CanLink for FakeBus {
    fn send(&mut self, frame: &CanFrame) -> bool {
        self.car().heard(frame);
        true
    }
    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame> {
        let frame = self.car().out.pop_front();
        if frame.is_none() {
            // Everything the car says follows something we sent, from this
            // thread: when nothing is there, nothing comes within the time.
            std::thread::sleep(Duration::from_millis(timeout_ms.min(2000)));
        }
        frame
    }
    fn name(&self) -> String {
        "fake CAN".into()
    }
}

#[test]
fn tp20_channel_setup_asks_for_the_ids_and_takes_the_parameters() {
    let mut car = Car::with_engine();
    car.units.get_mut(&0x01).unwrap().tester_block = 4;
    let bus = FakeBus::new(car);
    let mut tp = bus.tp20();
    tp.open_channel(0x01).expect("channel opens");
    assert!(tp.channel_open());
    // We send on the id the unit named, it sends on the one we asked for.
    assert_eq!(tp.channel_ids(), Some((UNIT_RX, 0x300)));
    assert_eq!(tp.block_size(), Some(4));
    assert_eq!(
        bus.sent_to(0x200),
        vec![vec![0x01, 0xC0, 0x00, 0x10, 0x00, 0x03, 0x01]]
    );
    assert_eq!(
        bus.sent_to(UNIT_RX)[0],
        vec![0xA0, 0x0F, 0x8A, 0xFF, 0x32, 0xFF]
    );
    assert_eq!(tp.name(), "fake CAN TP 2.0");
}

#[test]
fn tp20_an_absent_or_refusing_unit_is_reported_in_time() {
    let mut car = Car::with_engine();
    car.refusing.push(0x17);
    let bus = FakeBus::new(car);
    let mut tp = bus.tp20();

    let started = Instant::now();
    let error = tp.open_channel(0x22).unwrap_err();
    assert!(error.contains("vermutlich nicht verbaut"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!tp.channel_open());

    let error = tp.open_channel(0x17).unwrap_err();
    assert!(error.contains("lehnt den TP-2.0-Kanal ab"), "{error}");
    assert!(error.contains("D7"), "{error}");
}

#[test]
fn tp20_long_messages_are_split_with_block_size_and_acknowledged_both_ways() {
    let mut car = Car::with_engine();
    // Two packets from us before an acknowledgement, three from the unit.
    car.units.get_mut(&0x01).unwrap().tester_block = 2;
    car.units.get_mut(&0x01).unwrap().own_block = 3;
    let bus = FakeBus::new(car);
    let mut tp = bus.tp20();
    tp.open_channel(0x01).unwrap();

    // 20 bytes: with the length 22, four packets of 7, 7, 7 and 1.
    let request: Vec<u8> = std::iter::once(0x31).chain(1..20).collect();
    tp.send_message(&request).expect("sent");
    let packets: Vec<Vec<u8>> = bus
        .sent_to(UNIT_RX)
        .into_iter()
        .filter(|d| d[0] >> 4 <= 3)
        .collect();
    let heads: Vec<u8> = packets.iter().map(|p| p[0]).collect();
    // 2: more follow; 0: block full, acknowledge; 1: the last one.
    assert_eq!(heads, vec![0x20, 0x01, 0x22, 0x13]);
    assert_eq!(&packets[0][1..3], &[0x00, 20]);

    // The echo comes back in four packets too, the third and the last want
    // an acknowledgement from us.
    let answer = tp.receive_message(1000).expect("answer");
    assert_eq!(answer[0], 0x71);
    assert_eq!(&answer[1..], &request[1..]);
    let acks: Vec<u8> = bus
        .sent_to(UNIT_RX)
        .into_iter()
        .filter(|d| d[0] >> 4 == 0xB)
        .map(|d| d[0])
        .collect();
    assert_eq!(acks, vec![0xB3, 0xB4]);

    // Counting goes on across messages and wraps after 15.
    for _ in 0..6 {
        tp.send_message(&request).unwrap();
        assert_eq!(tp.receive_message(1000).unwrap()[0], 0x71);
    }
}

#[test]
fn tp20_a_missing_acknowledgement_or_answer_ends_in_an_error() {
    let mut car = Car::with_engine();
    car.no_acks = true;
    let bus = FakeBus::new(car);
    let mut tp = bus.tp20();
    tp.open_channel(0x01).unwrap();
    let started = Instant::now();
    let error = tp.send_message(&[0x10, 0x89]).unwrap_err();
    assert!(error.contains("bestätigt"), "{error}");
    // T1 is 100 ms.
    assert!(started.elapsed() >= Duration::from_millis(90));

    bus.car().no_acks = false;
    // No request, no answer.
    let error = tp.receive_message(50).unwrap_err();
    assert!(error.contains("antwortet nicht"), "{error}");
    assert!(tp.send_message(&[]).is_err());

    tp.close_channel();
    assert!(tp.send_message(&[0x10, 0x89]).is_err());
}

#[test]
fn tp20_the_channel_test_keeps_the_channel_open() {
    let mut car = Car::with_engine();
    car.idle_limit = Some(Duration::from_millis(150));
    let bus = FakeBus::new(car);
    let mut tp = bus.tp20();

    // Left alone, the unit forgets the channel.
    tp.open_channel(0x01).unwrap();
    std::thread::sleep(Duration::from_millis(250));
    tp.send_message(&[0x3E, 0x01]).unwrap_err();

    // With a channel test every 60 ms it stays.
    tp.open_channel(0x01).unwrap();
    for _ in 0..5 {
        std::thread::sleep(Duration::from_millis(60));
        tp.keep_alive().expect("unit answers A1");
        assert!(tp.idle_ms().unwrap() < 50);
    }
    tp.send_message(&[0x3E, 0x01]).unwrap();
    assert_eq!(tp.receive_message(1000).unwrap(), vec![0x7E]);
    assert!(bus.sent_to(UNIT_RX).iter().filter(|d| d[0] == 0xA3).count() >= 5);

    // A unit that no longer answers the test is gone.
    bus.car().no_channel_test = true;
    assert!(tp.keep_alive().is_err());
    assert!(!tp.channel_open());
}

#[test]
fn tp20_disconnect_from_either_side() {
    let bus = FakeBus::new(Car::with_engine());
    let mut tp = bus.tp20();
    tp.open_channel(0x01).unwrap();
    tp.close_channel();
    assert!(!tp.channel_open());
    assert_eq!(bus.sent_to(UNIT_RX).last(), Some(&vec![0xA8]));
    assert!(bus.car().open.is_none());

    // The unit hangs up instead of answering.
    tp.open_channel(0x01).unwrap();
    bus.car().hang_up_next = true;
    tp.send_message(&[0x21, 0x01]).unwrap();
    let error = tp.receive_message(1000).unwrap_err();
    assert!(error.contains("geschlossen"), "{error}");
    assert!(!tp.channel_open());

    // Dropping closes an open channel.
    tp.open_channel(0x01).unwrap();
    drop(tp);
    assert!(bus.car().open.is_none());
}

#[test]
fn tp20_kwp2000_identification_faults_groups_and_basic_settings() {
    let bus = FakeBus::new(Car::with_engine());
    let mut unit = VagTp20::new(bus.tp20(), None);
    unit.connect(Ecu::Engine).expect("connects");
    assert!(unit.connected());
    assert_eq!(unit.protocol(), KLineProtocol::Kwp2000);
    assert_eq!(unit.protocol_name(), "KWP2000 (TP 2.0)");
    assert_eq!(
        unit.identification(),
        vec!["4F0907401A  3.2l V6 FSI     0030"]
    );
    assert_eq!(bus.car().sessions, vec![session::DIAGNOSTICS]);

    let faults = unit.read_faults().expect("faults");
    assert_eq!(faults.len(), 2);
    assert_eq!(faults[0].code, 16486);
    assert!(!faults[0].sporadic);
    assert_eq!(faults[1].code, 561);
    assert!(faults[1].sporadic);

    unit.clear_faults().expect("cleared");
    assert!(unit.read_faults().unwrap().is_empty());

    // Response pending first, then the values.
    bus.car().pending_once = true;
    let values = unit.read_group(1).expect("values");
    assert_eq!(values.len(), 4);
    assert!(
        (values[0].value - 800.0).abs() < 0.01,
        "{}",
        values[0].value
    );
    assert!((values[1].value - 90.0).abs() < 0.01, "{}", values[1].value);

    unit.set_basic_setting_session(session::ADAPTATION);
    assert_eq!(unit.basic_setting(1).unwrap().len(), 4);
    assert_eq!(bus.car().sessions.last(), Some(&session::ADAPTATION));

    unit.keep_alive().expect("channel test and tester present");
    assert!(unit.read_adaptation(1).is_err());
    assert!(unit.actuator_test().is_err());

    unit.disconnect();
    assert!(!unit.connected());
    assert!(unit.read_faults().is_err());
    assert!(bus.car().open.is_none());

    // The next unit on the same link: absent units fail cleanly.
    assert!(unit.connect(Ecu::Abs).is_err());
    assert!(!unit.connected());
    unit.connect(Ecu::Engine).expect("again");
}

#[test]
fn tp20_shares_the_interface_with_the_elm_front_end() {
    let bus = FakeBus::new(Car::with_engine());
    let elm = CanElm::new(Box::new(bus.clone()), "fake", 500000);
    let mut unit = VagTp20::new(Tp20::new(Box::new(elm.shared_link())), None);
    unit.connect(Ecu::Engine)
        .expect("connects through the shared link");
    assert_eq!(unit.read_faults().unwrap().len(), 2);
    drop(elm);
    // The interface stays open for TP 2.0 as long as it holds it.
    assert_eq!(unit.read_faults().unwrap().len(), 2);
}
