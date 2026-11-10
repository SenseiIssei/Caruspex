//! Raw CAN interfaces: the ELM327 front end with its own ISO-TP over a
//! scripted car on 11 and 29 bit, the BMW raw frames, the bus monitor, and
//! the SLCAN text protocol. Frames worked out by hand from ISO 15765-2/-4.

mod common;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use caruspex_link::Transport;
use caruspex_net::can_bus::{CanElm, CanFrame, CanLink, SlcanLink, open_can_interface};
use common::MiniElm;

const VIN: &str = "WVWZZZAUZLW000001";

/// A car on the bus: engine unit with OBD on 11 and 29 bit, a BMW style
/// unit 12 behind 6F1, and a broadcast frame 0x132.
#[derive(Default)]
struct FakeCar {
    out: VecDeque<CanFrame>,
    flow_controls: usize,
    pending_cf: Vec<CanFrame>,
}

impl FakeCar {
    fn reply(&mut self, id: u32, extended: bool, mut data: Vec<u8>) {
        data.resize(8, 0xAA);
        self.out.push_back(CanFrame { id, extended, data });
    }

    /// Answers a request payload; multi frame answers wait for flow control.
    fn answer(&mut self, id: u32, extended: bool, payload: &[u8], target: Option<u8>) {
        let lead: Vec<u8> = target.into_iter().collect();
        let room = 7 - lead.len();
        if payload.len() <= room {
            let mut d = lead.clone();
            d.push(payload.len() as u8);
            d.extend_from_slice(payload);
            self.reply(id, extended, d);
            return;
        }
        let mut ff = lead.clone();
        ff.push(0x10 | (payload.len() >> 8) as u8);
        ff.push((payload.len() & 0xFF) as u8);
        let first = 6 - lead.len();
        ff.extend_from_slice(&payload[..first]);
        self.reply(id, extended, ff);
        self.pending_cf.clear();
        let mut n = 1u8;
        let mut at = first;
        while at < payload.len() {
            let mut cf = lead.clone();
            cf.push(0x20 | (n & 0x0F));
            cf.extend_from_slice(&payload[at..(at + 7 - lead.len()).min(payload.len())]);
            cf.resize(8, 0xAA);
            self.pending_cf.push(CanFrame {
                id,
                extended,
                data: cf,
            });
            at += 7 - lead.len();
            n += 1;
        }
    }

    fn vin_payload(head: &[u8]) -> Vec<u8> {
        let mut out = head.to_vec();
        out.extend_from_slice(VIN.as_bytes());
        out
    }

    fn heard(&mut self, f: &CanFrame) {
        let d = &f.data;
        // Flow control from the tester: release the consecutive frames.
        let bmw = !f.extended && f.id == 0x6F1;
        let pci = usize::from(bmw);
        if d.len() > pci && (d[pci] >> 4) == 3 {
            self.flow_controls += 1;
            let frames = std::mem::take(&mut self.pending_cf);
            self.out.extend(frames);
            return;
        }
        if bmw {
            if d.len() >= 5 && d[0] == 0x12 && d[2] == 0x22 && d[3] == 0xF1 && d[4] == 0x90 {
                self.answer(
                    0x612,
                    false,
                    &Self::vin_payload(&[0x62, 0xF1, 0x90]),
                    Some(0xF1),
                );
            }
            if d.len() >= 4 && d[0] == 0x12 && d[2] == 0x3E {
                self.answer(0x612, false, &[0x7E, 0x00], Some(0xF1));
            }
            if d.len() >= 5 && d[0] == 0x12 && d[2] == 0x19 {
                self.answer(0x612, false, &[0x59, 0x02, 0xFF], Some(0xF1));
            }
            return;
        }
        let obd11 = !f.extended && (f.id == 0x7DF || f.id == 0x7E0);
        let obd29 = f.extended && (f.id == 0x18DB33F1 || f.id == 0x18DA10F1);
        if !obd11 && !obd29 {
            return;
        }
        let id = if obd11 { 0x7E8 } else { 0x18DAF110 };
        let req = d[1..1 + usize::from(d[0] & 0x0F)].to_vec();
        if req == [0x01, 0x00] {
            self.answer(id, f.extended, &[0x41, 0x00, 0xBE, 0x3F, 0xA8, 0x13], None);
        }
        if req == [0x09, 0x02] {
            self.answer(
                id,
                f.extended,
                &Self::vin_payload(&[0x49, 0x02, 0x01]),
                None,
            );
        }
        if req == [0x03] {
            self.answer(id, f.extended, &[0x43, 0x01, 0x01, 0x71], None);
        }
        if req == [0x07] || req == [0x0A] {
            self.answer(id, f.extended, &[req[0] + 0x40, 0x00], None);
        }
    }
}

type SharedCar = Arc<Mutex<FakeCar>>;

struct FakeLink {
    car: SharedCar,
    only29: bool,
    quiet: usize,
    sent: Arc<Mutex<Vec<CanFrame>>>,
}

impl FakeLink {
    fn new(car: SharedCar, extended_only: bool) -> Self {
        Self {
            car,
            only29: extended_only,
            quiet: 0,
            sent: Arc::default(),
        }
    }
}

impl CanLink for FakeLink {
    fn send(&mut self, frame: &CanFrame) -> bool {
        self.sent.lock().unwrap().push(frame.clone());
        if self.only29 && !frame.extended {
            return true; // an 11-bit request nobody answers
        }
        self.car.lock().unwrap().heard(frame);
        true
    }

    fn receive(&mut self, _timeout_ms: u64) -> Option<CanFrame> {
        let mut car = self.car.lock().unwrap();
        if let Some(frame) = car.out.pop_front() {
            return Some(frame);
        }
        // Between answers the car keeps broadcasting.
        self.quiet += 1;
        if self.quiet.is_multiple_of(3) {
            return Some(CanFrame {
                id: 0x132,
                extended: false,
                data: vec![0x10, 0x96, 0x38, 0xFF, 0, 0, 0, 0],
            });
        }
        None
    }

    fn name(&self) -> String {
        "fake".into()
    }

    fn battery_voltage(&mut self) -> Option<f64> {
        Some(12.6)
    }
}

/// An SLCAN stick on a serial port, wired to the same car.
struct FakeSlcanPort {
    car: SharedCar,
    line: String,
    reply: String,
    commands: Arc<Mutex<Vec<String>>>,
}

impl FakeSlcanPort {
    fn new(car: SharedCar) -> Self {
        Self {
            car,
            line: String::new(),
            reply: String::new(),
            commands: Arc::default(),
        }
    }
}

impl Transport for FakeSlcanPort {
    fn write(&mut self, data: &[u8]) -> bool {
        self.line += &String::from_utf8_lossy(data);
        while let Some(end) = self.line.find('\r') {
            let cmd: String = self.line[..end].to_string();
            self.line.drain(..=end);
            self.commands.lock().unwrap().push(cmd.clone());
            if cmd == "V" {
                self.reply += "V1013\r";
            } else if cmd.is_empty() {
                self.reply += "\x07";
            } else if cmd.starts_with('t') || cmd.starts_with('T') {
                let extended = cmd.starts_with('T');
                let n = if extended { 8 } else { 3 };
                let dlc = usize::from(cmd.as_bytes()[1 + n] - b'0');
                let frame = CanFrame {
                    extended,
                    id: u32::from_str_radix(&cmd[1..1 + n], 16).unwrap(),
                    data: (0..dlc)
                        .map(|i| {
                            u8::from_str_radix(&cmd[2 + n + 2 * i..4 + n + 2 * i], 16).unwrap()
                        })
                        .collect(),
                };
                self.car.lock().unwrap().heard(&frame);
                self.reply += if extended { "Z\r" } else { "z\r" };
            } else {
                self.reply += "\r";
            }
        }
        true
    }

    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        let mut out = std::mem::take(&mut self.reply);
        let mut car = self.car.lock().unwrap();
        while let Some(f) = car.out.pop_front() {
            out += &if f.extended {
                format!("T{:08X}{}", f.id, f.data.len())
            } else {
                format!("t{:03X}{}", f.id, f.data.len())
            };
            for b in &f.data {
                out += &format!("{b:02X}");
            }
            out.push('\r');
        }
        out.into_bytes()
    }

    fn flush_input(&mut self) {}

    fn name(&self) -> String {
        "COM9".into()
    }
}

#[test]
fn can_front_end_obd_on_11_bit_with_iso_tp_and_flow_control() {
    for headers in [false, true] {
        let car = SharedCar::default();
        let link = FakeLink::new(car.clone(), false);
        let sent = link.sent.clone();
        let mut elm = MiniElm::new(CanElm::new(Box::new(link), "fake (500 kbit/s)", 500000));
        assert_eq!(elm.connect("6").unwrap(), "ISO 15765-4 (CAN 11/500)");
        if headers {
            assert!(elm.set_headers(true));
        }
        assert_eq!(elm.read_vin(), VIN, "headers {headers}");
        assert!(car.lock().unwrap().flow_controls >= 1);
        // The flow control went to the engine's request id.
        assert!(
            sent.lock()
                .unwrap()
                .iter()
                .any(|f| f.id == 0x7E0 && f.data.first() == Some(&0x30))
        );
        let faults = elm.read_faults();
        assert_eq!(faults.len(), 1, "headers {headers}");
        assert_eq!(faults[0].1, "P0171");
        assert_eq!(
            elm.send("ATRV", 1000).first().map(String::as_str),
            Some("12.6V")
        );
    }
}

#[test]
fn can_front_end_29_bit_cars_are_found_as_well() {
    let car = SharedCar::default();
    let mut elm = MiniElm::new(CanElm::new(
        Box::new(FakeLink::new(car, true)),
        "fake",
        500000,
    ));
    // The car ignores 11 bit; the protocol order tries 29 bit next.
    assert_eq!(elm.connect("67").unwrap(), "ISO 15765-4 (CAN 29/500)");
    assert_eq!(elm.read_vin(), VIN);
}

#[test]
fn can_front_end_bmw_raw_frames_and_the_bus_monitor() {
    let car = SharedCar::default();
    let mut elm = MiniElm::new(CanElm::new(
        Box::new(FakeLink::new(car, false)),
        "fake",
        500000,
    ));
    elm.connect("6").unwrap();
    // The BMW engine unit through 6F1, framing by the caller itself.
    assert!(elm.address_extended(0x6F1, 0x612));
    assert_eq!(
        elm.extended_request(0x12, 0xF1, &[0x3E, 0x00], 600),
        Some(vec![0x7E, 0x00])
    );
    let vin = elm
        .extended_request(0x12, 0xF1, &[0x22, 0xF1, 0x90], 1500)
        .unwrap();
    assert_eq!(&vin[3..], VIN.as_bytes());
    elm.restore_default();
    // Listening: the broadcast frame 0x132 through the receive filter.
    let frames = elm.capture_frames(0x132, 200, 3);
    assert!(!frames.is_empty());
    assert_eq!(frames[0].len(), 8);
    assert_eq!(frames[0][1], 0x96);
}

#[test]
fn can_front_end_answers_like_an_elm327() {
    let car = SharedCar::default();
    let mut elm = CanElm::new(
        Box::new(FakeLink::new(car, false)),
        "fake (500 kbit/s)",
        250000,
    );
    assert_eq!(elm.name(), "fake (500 kbit/s)");
    elm.write(b"ATZ\r");
    assert_eq!(elm.read(100), b"ATZ\rELM327 v2.1\r\r>");
    elm.write(b"ATE0\r");
    elm.read(100);
    elm.write(b"AT@1\r");
    assert_eq!(elm.read(100), b"fake (500 kbit/s)\r\r>");
    elm.write(b"ATDPN\r");
    assert_eq!(elm.read(100), b"8\r\r>");
    elm.write(b"ATSP3\r");
    assert_eq!(elm.read(100), b"?\r\r>");
    elm.write(b"ATSP7\r");
    elm.read(100);
    elm.write(b"ATDP\r");
    assert_eq!(elm.read(100), b"ISO 15765-4 (CAN 29/250)\r\r>");
    elm.write(b"ATCP1\r");
    assert_eq!(elm.read(100), b"?\r\r>");
    elm.write(b"ATRV\r");
    assert_eq!(elm.read(100), b"12.6V\r\r>");
}

#[test]
fn slcan_bit_rate_channel_and_frames_in_the_lawicel_text_protocol() {
    let car = SharedCar::default();
    let port = FakeSlcanPort::new(car.clone());
    let commands = port.commands.clone();
    let mut link = SlcanLink::new(Box::new(port));
    link.open(500000).unwrap();
    assert!(commands.lock().unwrap().iter().any(|c| c == "S6"));
    assert!(commands.lock().unwrap().iter().any(|c| c == "O"));
    assert_eq!(link.name(), "SLCAN COM9 V1013");

    let mut elm = MiniElm::new(CanElm::new(Box::new(link), "SLCAN", 500000));
    elm.connect("6").unwrap();
    assert_eq!(elm.read_vin(), VIN);
    // Requests went out as t7DF8 frames.
    assert!(
        commands
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.starts_with("t7DF8"))
    );
    // Closing the link closes the channel.
    drop(elm);
    assert_eq!(
        commands.lock().unwrap().last().map(String::as_str),
        Some("C")
    );

    let mut wrong = SlcanLink::new(Box::new(FakeSlcanPort::new(car)));
    assert!(wrong.open(123456).is_err());
}

#[test]
fn can_interface_specs_are_parsed_and_refused_cleanly() {
    let no_serial = |port: &str, _baud: u32| -> Result<Box<dyn Transport>, String> {
        Err(format!("no port {port}"))
    };
    let error = open_can_interface("nonsense:x", &no_serial).err().unwrap();
    assert!(!error.is_empty());
    assert!(error.ends_with("nonsense:x"));
    // SLCAN opens its port at 115200 baud and passes the reason on.
    assert_eq!(
        open_can_interface("slcan:COM5@250000", &no_serial)
            .err()
            .unwrap(),
        "no port COM5"
    );
    // A port that does not speak SLCAN.
    let car = SharedCar::default();
    let silent = move |_: &str, baud: u32| -> Result<Box<dyn Transport>, String> {
        assert_eq!(baud, 115200);
        let mut port = FakeSlcanPort::new(car.clone());
        port.reply.clear();
        Ok(Box::new(Mute(port)))
    };
    assert!(open_can_interface("slcan:COM5", &silent).is_err());
    // A good one: the name carries the bit rate.
    let car = SharedCar::default();
    let good = move |_: &str, _: u32| -> Result<Box<dyn Transport>, String> {
        Ok(Box::new(FakeSlcanPort::new(car.clone())))
    };
    let elm = open_can_interface("slcan:COM5@250000", &good).unwrap();
    assert_eq!(elm.name(), "SLCAN COM9 V1013 (250 kbit/s)");
    #[cfg(not(target_os = "linux"))]
    assert!(open_can_interface("socketcan:can0", &no_serial).is_err());
    #[cfg(not(windows))]
    {
        assert!(open_can_interface("j2534:x.dll", &no_serial).is_err());
        assert!(caruspex_net::j2534_devices().is_empty());
    }
}

/// A serial port where nothing answers.
struct Mute(FakeSlcanPort);

impl Transport for Mute {
    fn write(&mut self, _data: &[u8]) -> bool {
        true
    }
    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        Vec::new()
    }
    fn flush_input(&mut self) {}
    fn name(&self) -> String {
        self.0.name()
    }
}
