//! The control unit scan over a raw CAN interface with several ISO-TP
//! sessions at once: a bus with four units that answer after a short delay
//! (one only to 22 F187 with a long answer and flow control, one busy with
//! 7F 3E 78 first, one refusing 3E) and many addresses where nobody is.
//! The parallel scan finds the same units with the same data as the scan
//! one by one, in a fraction of the time.

use std::collections::VecDeque;
use std::time::Instant;

use caruspex_data::ecu_addresses::EcuDefinition;
use caruspex_link::timing::{now_ms, sleep_ms};
use caruspex_link::{Elm327, IsoTpRequest, Transport};
use caruspex_net::can_bus::{CanElm, CanFrame, CanLink};
use caruspex_protocols::uds::{UdsEcu, find_present_ecus, scan_ecus};

/// How long a unit takes for an answer.
const DELAY_MS: u64 = 20;

#[derive(Clone, Copy, PartialEq)]
enum Unit {
    /// Answers 3E and everything else.
    Plain,
    /// Ignores 3E, answers 22 F187 with 20 bytes (first frame, flow control).
    Quiet,
    /// 7F 3E 78 first, the answer 150 ms later.
    Busy,
    /// Refuses 3E (7F 3E 11), answers the rest.
    Refusing,
}

struct Bus {
    units: Vec<(u32, Unit)>,
    /// When each frame may be read.
    queue: VecDeque<(u64, CanFrame)>,
    /// Consecutive frames held back until flow control, per answer id.
    held: Vec<(u32, Vec<CanFrame>)>,
}

impl Bus {
    fn new() -> Self {
        Self {
            units: vec![
                (0x7E0, Unit::Plain),
                (0x7E1, Unit::Quiet),
                (0x760, Unit::Busy),
                (0x714, Unit::Refusing),
            ],
            queue: VecDeque::new(),
            held: Vec::new(),
        }
    }

    fn later(&mut self, delay: u64, id: u32, mut data: Vec<u8>) {
        data.resize(8, 0xAA);
        let at = now_ms() + delay;
        let place = self.queue.iter().position(|(t, _)| *t > at);
        let frame = (
            at,
            CanFrame {
                id,
                extended: false,
                data,
            },
        );
        match place {
            Some(p) => self.queue.insert(p, frame),
            None => self.queue.push_back(frame),
        }
    }

    fn answer(&mut self, delay: u64, id: u32, payload: &[u8]) {
        if payload.len() <= 7 {
            let mut d = vec![payload.len() as u8];
            d.extend_from_slice(payload);
            self.later(delay, id, d);
            return;
        }
        let mut ff = vec![0x10 | (payload.len() >> 8) as u8, payload.len() as u8];
        ff.extend_from_slice(&payload[..6]);
        self.later(delay, id, ff);
        let frames = payload[6..]
            .chunks(7)
            .enumerate()
            .map(|(n, chunk)| {
                let mut d = vec![0x21 + n as u8];
                d.extend_from_slice(chunk);
                d.resize(8, 0xAA);
                CanFrame {
                    id,
                    extended: false,
                    data: d,
                }
            })
            .collect();
        self.held.push((id, frames));
    }

    fn heard(&mut self, f: &CanFrame) {
        let d = &f.data;
        if f.id == 0x7DF && d[..3] == [0x02, 0x01, 0x00] {
            self.answer(DELAY_MS, 0x7E8, &[0x41, 0x00, 0x98, 0x3B, 0x80, 0x13]);
            return;
        }
        let Some(&(_, unit)) = self.units.iter().find(|(id, _)| *id == f.id) else {
            return; // nobody at this address
        };
        let response = f.id + 8;
        if d[0] >> 4 == 3 {
            if let Some(p) = self.held.iter().position(|(id, _)| *id == response) {
                let (_, frames) = self.held.remove(p);
                for (n, cf) in frames.into_iter().enumerate() {
                    self.later(1 + n as u64, response, cf.data);
                }
            }
            return;
        }
        let request = d[1..1 + usize::from(d[0] & 0x0F)].to_vec();
        match (unit, request.as_slice()) {
            (Unit::Quiet, [0x3E, ..]) => {}
            (Unit::Busy, [0x3E, ..]) => {
                self.answer(DELAY_MS, response, &[0x7F, 0x3E, 0x78]);
                self.answer(150, response, &[0x7E, 0x00]);
            }
            (Unit::Refusing, [0x3E, ..]) => self.answer(DELAY_MS, response, &[0x7F, 0x3E, 0x11]),
            (_, [0x3E, ..]) => self.answer(DELAY_MS, response, &[0x7E, 0x00]),
            (Unit::Quiet, [0x22, 0xF1, 0x87]) => {
                let mut a = vec![0x62, 0xF1, 0x87];
                a.extend_from_slice(b"8E0907115AB  ZZZZZ");
                self.answer(DELAY_MS, response, &a);
            }
            (_, [0x22, 0xF1, 0x97]) => {
                let mut a = vec![0x62, 0xF1, 0x97];
                a.extend_from_slice(format!("UNIT{:03X}", f.id).as_bytes());
                self.answer(DELAY_MS, response, &a);
            }
            (_, [0x22, hi, lo]) => {
                let _ = (hi, lo);
                self.answer(DELAY_MS, response, &[0x7F, 0x22, 0x31]);
            }
            (_, [0x19, 0x02, ..]) => {
                let a = [0x59, 0x02, 0xFF, 0x01, 0x71, 0x00, 0x2F];
                self.answer(DELAY_MS, response, &a);
            }
            (_, [service, ..]) => self.answer(DELAY_MS, response, &[0x7F, *service, 0x11]),
            _ => {}
        }
    }
}

impl CanLink for Bus {
    fn send(&mut self, frame: &CanFrame) -> bool {
        self.heard(frame);
        true
    }

    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame> {
        let deadline = now_ms() + timeout_ms;
        loop {
            let now = now_ms();
            if let Some((at, _)) = self.queue.front()
                && *at <= now
            {
                return self.queue.pop_front().map(|(_, f)| f);
            }
            if now >= deadline {
                return None;
            }
            sleep_ms(1);
        }
    }

    fn name(&self) -> String {
        "bus with delays".into()
    }
}

/// The same front end, but without the batch: the scan one by one.
struct OneByOne(CanElm);

impl Transport for OneByOne {
    fn write(&mut self, data: &[u8]) -> bool {
        self.0.write(data)
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        self.0.read(timeout_ms)
    }
    fn flush_input(&mut self) {
        self.0.flush_input()
    }
    fn name(&self) -> String {
        self.0.name()
    }
}

fn ecu(request: u32, name: &str) -> EcuDefinition {
    EcuDefinition {
        request,
        response: request + 8,
        name: name.into(),
        ..EcuDefinition::default()
    }
}

fn units() -> Vec<EcuDefinition> {
    vec![
        ecu(0x7E0, "engine"),
        ecu(0x7E1, "gearbox"),
        ecu(0x760, "brakes"),
        ecu(0x714, "instruments"),
    ]
}

fn connect(transport: Box<dyn Transport>) -> Elm327 {
    let mut elm = Elm327::new();
    elm.connect(transport, "6").expect("connects");
    elm
}

#[test]
fn many_sessions_at_once_answer_with_flow_control_and_pending() {
    let mut elm = CanElm::new(Box::new(Bus::new()), "bus", 500000);
    let request = |id: u32, data: &[u8]| IsoTpRequest {
        request: id,
        response: id + 8,
        data: data.to_vec(),
        ..IsoTpRequest::default()
    };
    let mut requests: Vec<IsoTpRequest> = units()
        .iter()
        .map(|d| request(d.request, &[0x3E, 0x00]))
        .collect();
    requests.extend((0..20).map(|n| request(0x700 + n, &[0x3E, 0x00])));
    requests.push(request(0x7E1, &[0x22, 0xF1, 0x87]));
    let started = Instant::now();
    let batch = elm.iso_tp_batch().expect("a raw CAN front end has it");
    let replies = batch.exchange_all(&requests, 200);
    let took = started.elapsed().as_millis();
    assert_eq!(replies[0], vec![vec![0x7E, 0x00]]);
    assert!(replies[1].is_empty(), "the gearbox ignores 3E");
    assert_eq!(replies[2], vec![vec![0x7F, 0x3E, 0x78], vec![0x7E, 0x00]]);
    assert_eq!(replies[3], vec![vec![0x7F, 0x3E, 0x11]]);
    assert!(replies[4..24].iter().all(Vec::is_empty));
    // The second gearbox request answers on the same id: the next round.
    let mut part = vec![0x62, 0xF1, 0x87];
    part.extend_from_slice(b"8E0907115AB  ZZZZZ");
    assert_eq!(replies[24], vec![part]);
    // Two rounds of 200 ms at most, not 25 waits one after the other.
    assert!(took < 1000, "{took} ms");
}

#[test]
fn the_parallel_scan_finds_what_the_scan_one_by_one_finds() {
    // Four units and twelve empty addresses.
    let mut list = units();
    list.extend((0..12).map(|n| ecu(0x740 + n, "nobody")));

    let mut elm = connect(Box::new(CanElm::new(Box::new(Bus::new()), "bus", 500000)));
    let found = find_present_ecus(&mut elm, &list, None).expect("parallel over raw CAN");
    assert_eq!(&found[..5], [true, true, true, true, false]);
    assert!(found[4..].iter().all(|p| !p));

    let started = Instant::now();
    let parallel: Vec<UdsEcu> = scan_ecus(&mut elm, &list, None, None);
    let parallel_ms = started.elapsed().as_millis();
    // A missing unit costs a full answer time (1 s) twice one by one; all
    // twelve together cost two waits of 200 ms.
    assert!(parallel_ms < 4000, "{parallel_ms} ms");
    assert_eq!(parallel.iter().filter(|e| e.present).count(), 4);
    let gearbox = &parallel[1];
    assert_eq!(gearbox.info.part_number, "8E0907115AB  ZZZZZ".trim());
    assert_eq!(parallel[2].info.system_name, "UNIT760");
    assert!(
        parallel
            .iter()
            .take(4)
            .all(|e| e.dtcs_read && e.dtcs.len() == 1)
    );

    // One by one, through a front end without the batch: the same result
    // for the four units and one empty address.
    let mut elm = connect(Box::new(OneByOne(CanElm::new(
        Box::new(Bus::new()),
        "bus",
        500000,
    ))));
    assert!(find_present_ecus(&mut elm, &list, None).is_none());
    let short = &list[..5];
    let started = Instant::now();
    let one_by_one = scan_ecus(&mut elm, short, None, None);
    let one_by_one_ms = started.elapsed().as_millis();
    assert_eq!(one_by_one, parallel[..5]);
    eprintln!(
        "parallel: {} addresses in {parallel_ms} ms; one by one: {} in {one_by_one_ms} ms",
        list.len(),
        short.len()
    );
}
