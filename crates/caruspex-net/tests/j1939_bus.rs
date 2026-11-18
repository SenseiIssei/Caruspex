//! SAE J1939 on a raw CAN interface: the demo truck on a simulated bus,
//! the J1939 session on top of [`J1939Can`]. The simulated bus runs four
//! times faster than the clock on the wall, so a second of truck traffic
//! takes a quarter second here.

use std::collections::VecDeque;
use std::time::Instant;

use caruspex_demo::truck::{TRUCK_VIN, TimedFrame, TruckSim};
use caruspex_net::{CanFrame, CanLink, J1939Can, SharedCanLink};
use caruspex_protocols::j1939::params::Reading;
use caruspex_protocols::j1939::{AckKind, J1939Session};

const SPEED: u64 = 4;
const START_MS: u64 = 600_000;

struct SimBus {
    truck: TruckSim,
    started: Instant,
    heard_up_to: u64,
    queue: VecDeque<TimedFrame>,
    sent: std::sync::Arc<std::sync::Mutex<Vec<u32>>>,
}

impl SimBus {
    fn new(truck: TruckSim) -> Self {
        SimBus {
            truck,
            started: Instant::now(),
            heard_up_to: START_MS,
            queue: VecDeque::new(),
            sent: Default::default(),
        }
    }

    fn now(&self) -> u64 {
        START_MS + self.started.elapsed().as_millis() as u64 * SPEED
    }

    fn refill(&mut self, now: u64) {
        if now <= self.heard_up_to {
            return;
        }
        let mut frames: Vec<TimedFrame> = self.queue.drain(..).collect();
        frames.extend(self.truck.broadcasts(self.heard_up_to, now));
        // Somebody else's 11-bit traffic on the same wires.
        frames.push(TimedFrame {
            at_ms: now,
            id: 0x7E8,
            data: vec![0x02, 0x41, 0x00],
        });
        frames.sort_by_key(|f| f.at_ms);
        self.queue = frames.into();
        self.heard_up_to = now;
    }
}

impl CanLink for SimBus {
    fn send(&mut self, frame: &CanFrame) -> bool {
        assert!(frame.extended, "J1939 frames are 29 bit");
        self.sent.lock().unwrap().push(frame.id);
        let now = self.now();
        self.refill(now);
        let mut frames: Vec<TimedFrame> = self.queue.drain(..).collect();
        frames.extend(self.truck.on_frame(frame.id, &frame.data, now));
        frames.sort_by_key(|f| f.at_ms);
        self.queue = frames.into();
        true
    }

    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame> {
        let deadline = Instant::now() + std::time::Duration::from_millis(timeout_ms);
        loop {
            let now = self.now();
            self.refill(now);
            if self.queue.front().is_some_and(|f| f.at_ms <= now) {
                let f = self.queue.pop_front()?;
                return Some(CanFrame {
                    id: f.id,
                    extended: f.id > 0x7FF,
                    data: f.data,
                });
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    fn name(&self) -> String {
        "simulated bus (250 kbit/s)".into()
    }
}

fn link(truck: TruckSim) -> (J1939Can, std::sync::Arc<std::sync::Mutex<Vec<u32>>>) {
    let bus = SimBus::new(truck);
    let sent = bus.sent.clone();
    (J1939Can::new(SharedCanLink::new(Box::new(bus))), sent)
}

#[test]
fn listening_reads_values_and_the_long_dm1() {
    let (mut can, sent) = link(TruckSim::new());
    let mut session = J1939Session::new();
    session.set_waits(400, 500);
    let frames = session.listen(&mut can, 300).unwrap();
    assert!(frames > 30, "{frames}");
    assert!(session.values().iter().any(|v| v.spn == 84));
    let active = session.active_faults(&mut can).unwrap();
    let engine = active
        .iter()
        .find(|u| u.address == 0)
        .expect("engine DM1 via BAM");
    assert_eq!(engine.dtcs.len(), 2);
    assert!(engine.lamps.mil.on());
    // Listening sends nothing, not even an address claim.
    assert!(sent.lock().unwrap().is_empty());
    assert_eq!(session.address(), (0xF9, false));
}

#[test]
fn requests_with_rts_cts_and_clearing() {
    let (mut can, sent) = link(TruckSim::new());
    let mut session = J1939Session::new();
    session.set_waits(400, 500);
    session.listen(&mut can, 200).unwrap();
    let vin = session.read_vin(&mut can).unwrap();
    assert_eq!(vin.as_deref(), Some(TRUCK_VIN));
    {
        let ids = sent.lock().unwrap();
        // Address claim, request to the engine, CTS, end of message.
        assert_eq!(ids[0], 0x18EE_FFF9);
        assert_eq!(ids[1], 0x18EA_00F9);
        assert!(ids.contains(&0x1CEC_00F9));
    }
    let previous = session.previous_faults(&mut can).unwrap();
    assert!(
        previous
            .iter()
            .any(|u| u.address == 0x0B && u.dtcs[0].spn == 904)
    );
    let cleared = session.clear_active(&mut can).unwrap();
    assert_eq!(cleared.acks.len(), 4);
    assert!(cleared.acks.iter().all(|(_, a)| *a == AckKind::Positive));
    let active = session.active_faults(&mut can).unwrap();
    assert!(active.iter().all(|u| u.dtcs.is_empty()));
    // The intake temperature sensor reports a value again.
    session.listen(&mut can, 200).unwrap();
    let intake = session.values().into_iter().find(|v| v.spn == 105).unwrap();
    assert_eq!(intake.reading, Reading::Value(35.0));
}

#[test]
fn the_tester_gives_way_when_f9_is_taken() {
    let (mut can, sent) = link(TruckSim::new().with_tool_on_f9());
    let mut session = J1939Session::new();
    session.set_waits(400, 500);
    let previous = session.previous_faults(&mut can).unwrap();
    assert!(!previous.is_empty());
    assert_eq!(session.address(), (0xFA, true));
    let ids = sent.lock().unwrap();
    assert!(ids.contains(&0x18EE_FFF9));
    assert!(ids.contains(&0x18EE_FFFA));
    assert!(ids.contains(&0x18EA_FFFA));
}
