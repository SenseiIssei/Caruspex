//! BMW ENET (HSFZ): frames, the identification answer, the session with
//! acknowledgement, alive check and response pending, and the ELM327 front
//! end end to end with an ELM327 dialogue on top of a scripted car.
//! Frame layout after EdiabasLib (EdInterfaceEnet); the byte examples are
//! worked out by hand from it and from ISO 14229 / SAE J1979.

mod common;

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use caruspex_link::Transport;
use caruspex_net::enet::{
    DiagAnswer, DiagLink, DoipSession, EnetElm, Flavor, HsfzSession, NetStream, doip, hsfz,
};
use common::MiniElm;

const VIN: &str = "WBA4J1C50KBM12345";

fn vin_answer(prefix: &[u8]) -> Vec<u8> {
    let mut out = prefix.to_vec();
    out.extend_from_slice(VIN.as_bytes());
    out
}

type Respond = Box<dyn FnMut(&hsfz::Frame) -> Vec<hsfz::Frame> + Send>;

#[derive(Default)]
struct CarSide {
    respond: Option<Respond>,
    sent: Vec<hsfz::Frame>,
    dead: bool,
    parser: hsfz::Parser,
    inbound: VecDeque<u8>,
}

/// The car's side of the TCP connection: every diagnostic frame the session
/// sends is handed to respond, whose frames are queued for receive.
#[derive(Clone, Default)]
struct FakeCarStream(Arc<Mutex<CarSide>>);

impl NetStream for FakeCarStream {
    fn send(&mut self, data: &[u8]) -> bool {
        let mut guard = self.0.lock().unwrap();
        let CarSide {
            respond,
            sent,
            dead,
            parser,
            inbound,
        } = &mut *guard;
        if *dead {
            return false;
        }
        parser.feed(data);
        for f in parser.by_ref() {
            sent.push(f.clone());
            if let Some(respond) = respond.as_mut() {
                for out in respond(&f) {
                    inbound.extend(hsfz::encode(out.control, &out.payload));
                }
            }
        }
        true
    }

    fn receive(&mut self, buffer: &mut [u8], _timeout_ms: u64) -> Option<usize> {
        let mut side = self.0.lock().unwrap();
        if side.inbound.is_empty() {
            return if side.dead { None } else { Some(0) };
        }
        // Hand out small pieces, as TCP may.
        let n = buffer.len().min(5).min(side.inbound.len());
        for slot in buffer.iter_mut().take(n) {
            *slot = side.inbound.pop_front().unwrap();
        }
        Some(n)
    }
}

fn ack(request: &hsfz::Frame) -> hsfz::Frame {
    hsfz::Frame {
        control: hsfz::ACK,
        payload: request.payload.clone(),
    }
}

fn answer(source: u8, uds: &[u8]) -> hsfz::Frame {
    let mut payload = vec![source, hsfz::TESTER];
    payload.extend_from_slice(uds);
    hsfz::Frame {
        control: hsfz::DIAGNOSTIC,
        payload,
    }
}

fn frame(control: u8, payload: &[u8]) -> hsfz::Frame {
    hsfz::Frame {
        control,
        payload: payload.to_vec(),
    }
}

/// Every request the scripted car was asked: unit and bytes.
type Asked = Arc<Mutex<Vec<(u16, Vec<u8>)>>>;

/// A scripted BMW behind DiagLink: answers by unit and request bytes.
struct FakeBmw {
    replies: HashMap<(u16, Vec<u8>), Vec<u8>>,
    asked: Asked,
}

impl DiagLink for FakeBmw {
    fn request(
        &mut self,
        target: u16,
        data: &[u8],
        _timeout_ms: u64,
    ) -> Result<Vec<DiagAnswer>, String> {
        self.asked.lock().unwrap().push((target, data.to_vec()));
        match self.replies.get(&(target, data.to_vec())) {
            Some(reply) => Ok(vec![DiagAnswer {
                source: target,
                data: reply.clone(),
            }]),
            None => Err("no answer".into()),
        }
    }
}

fn scripted_bmw() -> FakeBmw {
    let dme = u16::from(hsfz::ENGINE);
    let mut replies = HashMap::new();
    replies.insert(
        (dme, vec![0x01, 0x00]),
        vec![0x41, 0x00, 0xBE, 0x3F, 0xA8, 0x13],
    );
    replies.insert((dme, vec![0x01, 0x42]), vec![0x41, 0x42, 0x31, 0x9C]); // 12.7 V
    replies.insert((dme, vec![0x03]), vec![0x43, 0x01, 0x01, 0x71]); // P0171
    replies.insert((dme, vec![0x09, 0x02]), vin_answer(&[0x49, 0x02, 0x01])); // 20 bytes
    replies.insert((dme, vec![0x3E, 0x00]), vec![0x7E, 0x00]);
    replies.insert(
        (dme, vec![0x22, 0xF1, 0x90]),
        vin_answer(&[0x62, 0xF1, 0x90]),
    );
    replies.insert(
        (dme, vec![0x19, 0x02, 0xFF]),
        vec![0x59, 0x02, 0xFF, 0x48, 0x0A, 0x10, 0x2F],
    );
    replies.insert((0x29, vec![0x3E, 0x00]), vec![0x7E, 0x00]);
    replies.insert((0x29, vec![0x19, 0x02, 0xFF]), vec![0x59, 0x02, 0xFF]);
    FakeBmw {
        replies,
        asked: Arc::default(),
    }
}

#[test]
fn hsfz_frames_encode_cut_from_a_stream_refuse_nonsense() {
    let encoded = hsfz::encode(hsfz::DIAGNOSTIC, &[0xF4, 0x12, 0x22, 0xF1, 0x90]);
    assert_eq!(
        encoded,
        [
            0x00, 0x00, 0x00, 0x05, 0x00, 0x01, 0xF4, 0x12, 0x22, 0xF1, 0x90
        ]
    );
    assert_eq!(hsfz::encode(hsfz::IDENT, &[]), [0, 0, 0, 0, 0, 0x11]);

    let mut parser = hsfz::Parser::new();
    parser.feed(&encoded[..4]);
    assert!(parser.next().is_none());
    parser.feed(&encoded[4..]);
    parser.feed(&hsfz::encode(hsfz::ALIVE_CHECK, &[]));
    let f = parser.next().unwrap();
    assert_eq!(f.control, hsfz::DIAGNOSTIC);
    assert_eq!(f.payload, [0xF4, 0x12, 0x22, 0xF1, 0x90]);
    let f = parser.next().unwrap();
    assert_eq!(f.control, hsfz::ALIVE_CHECK);
    assert!(f.payload.is_empty());
    assert!(parser.next().is_none());

    let mut bad = hsfz::Parser::new();
    bad.feed(&[0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x01]);
    assert!(bad.next().is_none());
    assert!(bad.broken());
}

#[test]
fn hsfz_vehicle_identification_answer() {
    let mut d = hsfz::encode(hsfz::IDENT, &[]);
    let text = format!("DIAGADR10BMWMAC001A37AABBCCBMWVIN{VIN}");
    d.extend_from_slice(text.as_bytes());
    d[3] = text.len() as u8;
    let ident = hsfz::parse_ident(&d).unwrap();
    assert_eq!(ident.diag_address, Some(0x10));
    assert_eq!(ident.mac, "001A37AABBCC");
    assert_eq!(ident.vin, VIN);

    // Short answers without the VIN still count, other datagrams do not.
    let ident = hsfz::parse_ident(&d[..44]).unwrap();
    assert!(ident.vin.is_empty());
    assert!(hsfz::parse_ident(&[b'x'; 60]).is_none());
}

#[test]
fn hsfz_session_acknowledgement_alive_check_pending_and_refusals() {
    let car = FakeCarStream::default();
    let pending_rounds = Arc::new(AtomicUsize::new(0));
    let nacks = Arc::new(AtomicUsize::new(0));
    {
        let pending_rounds = pending_rounds.clone();
        let nacks = nacks.clone();
        car.0.lock().unwrap().respond = Some(Box::new(move |input: &hsfz::Frame| {
            if input.control != hsfz::DIAGNOSTIC {
                return Vec::new();
            }
            let uds = &input.payload[2..];
            let target = input.payload[1];
            if uds == [0x3E, 0x00] {
                // Alive check before the answer, and a frame for another tester.
                return vec![
                    ack(input),
                    frame(hsfz::ALIVE_CHECK, &[]),
                    frame(hsfz::DIAGNOSTIC, &[0x12, 0xF1, 0x7E, 0x00]),
                    answer(target, &[0x7E, 0x00]),
                ];
            }
            if uds == [0x22, 0xF1, 0x87] {
                pending_rounds.fetch_add(1, Ordering::SeqCst);
                return vec![
                    ack(input),
                    answer(target, &[0x7F, 0x22, 0x78]),
                    answer(target, &[0x7F, 0x22, 0x78]),
                    answer(target, &[0x62, 0xF1, 0x87, 0x41]),
                ];
            }
            if uds == [0x22, 0xF1, 0x97] {
                if nacks.fetch_add(1, Ordering::SeqCst) == 0 {
                    return vec![frame(hsfz::NACK, &[])];
                }
                return vec![ack(input), answer(target, &[0x62, 0xF1, 0x97, 0x42])];
            }
            if uds == [0x01, 0x00] && target == hsfz::FUNCTIONAL {
                return vec![
                    ack(input),
                    answer(0x12, &[0x41, 0x00, 0xBE]),
                    answer(0x18, &[0x41, 0x00, 0x80]),
                ];
            }
            if target == 0x55 {
                return vec![frame(0x41, &[])]; // gateway: unknown target
            }
            vec![ack(input)] // acknowledged, never answered
        }));
    }
    let mut session = HsfzSession::new(Box::new(car.clone()));

    let answers = session.request(0x12, &[0x3E, 0x00], 500).unwrap();
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].source, 0x12);
    assert_eq!(answers[0].data, [0x7E, 0x00]);
    {
        let side = car.0.lock().unwrap();
        // The alive check was answered: control 13, payload 00 F4.
        assert!(
            side.sent
                .iter()
                .any(|f| f.control == hsfz::ALIVE_REPLY && f.payload == [0x00, 0xF4])
        );
        // The request itself: tester F4, target 12.
        assert_eq!(side.sent[0].payload, [0xF4, 0x12, 0x3E, 0x00]);
    }

    let answers = session.request(0x12, &[0x22, 0xF1, 0x87], 500).unwrap();
    assert_eq!(answers[0].data, [0x62, 0xF1, 0x87, 0x41]);
    assert_eq!(pending_rounds.load(Ordering::SeqCst), 1);

    let answers = session.request(0x29, &[0x22, 0xF1, 0x97], 500).unwrap();
    assert_eq!(answers[0].source, 0x29);
    assert_eq!(nacks.load(Ordering::SeqCst), 2);

    let answers = session
        .request(u16::from(hsfz::FUNCTIONAL), &[0x01, 0x00], 500)
        .unwrap();
    assert_eq!(answers.len(), 2);

    let error = session.request(0x40, &[0x22, 0xF1, 0x90], 100).unwrap_err();
    assert!(!error.is_empty());

    let error = session
        .request(0x55, &[0x22, 0xF1, 0x90], 5000)
        .unwrap_err();
    assert!(error.contains("0x41"), "{error}");

    car.0.lock().unwrap().dead = true;
    assert!(session.request(0x12, &[0x3E, 0x00], 100).is_err());
    assert!(session.request(0x12, &[0x3E, 0x00], 100).is_err());
    assert!(session.closed());
}

/// The C++ test runs Elm327, Obd2 and Uds on top; here the same dialogue
/// goes through the stand-in in tests/common.
#[test]
fn enet_front_end_elm_dialogue_obd_and_bmw_units_over_a_scripted_car() {
    for headers in [false, true] {
        let link = scripted_bmw();
        let asked = link.asked.clone();
        let mut elm = MiniElm::new(EnetElm::new(Box::new(link), "ENET test", Flavor::Bmw));
        assert_eq!(
            elm.connect("6").unwrap(),
            "ISO 15765-4 (CAN 11/500)",
            "headers {headers}"
        );
        if headers {
            assert!(elm.set_headers(true));
        }

        let faults = elm.read_faults();
        assert_eq!(faults.len(), 1, "headers {headers}");
        assert_eq!(faults[0].1, "P0171");
        assert_eq!(elm.read_vin(), VIN, "headers {headers}");

        // BMW units through the raw frames of the front end (6F1, 6xx).
        let mut present = 0;
        for unit in [0x12u8, 0x18, 0x29, 0x40, 0x60] {
            assert!(elm.address_extended(0x6F1, 0x600 | u32::from(unit)));
            if elm.extended_request(unit, 0xF1, &[0x3E, 0x00], 600) != Some(vec![0x7E, 0x00]) {
                elm.restore_default();
                continue;
            }
            present += 1;
            let dtcs = elm
                .extended_request(unit, 0xF1, &[0x19, 0x02, 0xFF], 1500)
                .unwrap();
            assert_eq!(&dtcs[..3], [0x59, 0x02, 0xFF]);
            let codes: Vec<String> = dtcs[3..]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| format!("{:02X}{:02X}{:02X}", c[0], c[1], c[2]))
                .collect();
            if unit == 0x12 {
                let vin = elm
                    .extended_request(unit, 0xF1, &[0x22, 0xF1, 0x90], 1500)
                    .unwrap();
                assert_eq!(&vin[3..], VIN.as_bytes(), "headers {headers}");
                assert_eq!(codes, ["480A10"]);
            }
            if unit == 0x29 {
                assert!(codes.is_empty());
            }
            elm.restore_default();
        }
        assert_eq!(present, 2, "headers {headers}");

        // Normal OBD works again after the scan.
        assert_eq!(elm.read_vin(), VIN);
        // The engine unit stands in for the adapter's voltage pin.
        assert_eq!(
            elm.send("ATRV", 2000).first().map(String::as_str),
            Some("12.7V")
        );
        assert!(!asked.lock().unwrap().is_empty());
    }
}

#[test]
fn enet_front_end_answers_like_an_elm327() {
    let mut elm = EnetElm::new(Box::new(scripted_bmw()), "ENET test", Flavor::Bmw);
    assert_eq!(elm.name(), "ENET test");
    elm.write(b"ATZ\r");
    assert_eq!(elm.read(100), b"ATZ\rELM327 v2.1\r\r>");
    elm.write(b"ATE0\r");
    assert_eq!(elm.read(100), b"ATE0\rOK\r\r>");
    elm.write(b"0100\r");
    assert_eq!(elm.read(100), b"41 00 BE 3F A8 13\r\r>");
    elm.write(b"0902\r");
    assert_eq!(
        String::from_utf8(elm.read(100)).unwrap(),
        "014\r0: 49 02 01 57 42 41\r1: 34 4A 31 43 35 30 4B\r2: 42 4D 31 32 33 34 35\r\r>"
    );
    elm.write(b"ATSHFFFF\r");
    assert_eq!(elm.read(100), b"?\r\r>");
    elm.write(b"0100\r");
    elm.flush_input();
    assert!(elm.read(100).is_empty());
}

// ---------------------------------------------------------------------------
// DoIP (ISO 13400-2)

type DoipRespond = Box<dyn FnMut(u16, &[u8]) -> Vec<doip::Message> + Send>;

struct VehicleSide {
    routing_code: u8,
    sent: Vec<doip::Message>,
    respond: Option<DoipRespond>,
    parser: doip::Parser,
    inbound: VecDeque<u8>,
}

#[derive(Clone)]
struct FakeDoipVehicle(Arc<Mutex<VehicleSide>>);

impl FakeDoipVehicle {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(VehicleSide {
            routing_code: 0x10,
            sent: Vec::new(),
            respond: None,
            parser: doip::Parser::new(),
            inbound: VecDeque::new(),
        })))
    }
}

fn message(payload_type: u16, payload: &[u8]) -> doip::Message {
    doip::Message {
        payload_type,
        payload: payload.to_vec(),
    }
}

impl NetStream for FakeDoipVehicle {
    fn send(&mut self, data: &[u8]) -> bool {
        let mut guard = self.0.lock().unwrap();
        let VehicleSide {
            routing_code,
            sent,
            respond,
            parser,
            inbound,
        } = &mut *guard;
        parser.feed(data);
        for m in parser.by_ref() {
            sent.push(m.clone());
            let mut out = Vec::new();
            let p = &m.payload;
            if m.payload_type == doip::ROUTING_ACTIVATION_REQUEST {
                out.push(message(
                    doip::ROUTING_ACTIVATION_RESPONSE,
                    &[p[0], p[1], 0x10, 0x10, *routing_code, 0, 0, 0, 0],
                ));
                out.push(message(doip::ALIVE_CHECK_REQUEST, &[]));
            } else if m.payload_type == doip::DIAGNOSTIC_MESSAGE
                && let Some(respond) = respond.as_mut()
            {
                let target = u16::from_be_bytes([p[2], p[3]]);
                out.push(message(
                    doip::DIAGNOSTIC_ACK,
                    &[p[2], p[3], p[0], p[1], 0x00],
                ));
                out.extend(respond(target, &p[4..]));
            }
            for o in out {
                inbound.extend(doip::encode(o.payload_type, &o.payload));
            }
        }
        true
    }

    fn receive(&mut self, buffer: &mut [u8], _timeout_ms: u64) -> Option<usize> {
        let mut side = self.0.lock().unwrap();
        let n = buffer.len().min(7).min(side.inbound.len());
        for slot in buffer.iter_mut().take(n) {
            *slot = side.inbound.pop_front().unwrap();
        }
        Some(n)
    }
}

fn diag(source: u16, uds: &[u8]) -> doip::Message {
    let mut p = source.to_be_bytes().to_vec();
    p.extend_from_slice(&doip::TESTER.to_be_bytes());
    p.extend_from_slice(uds);
    message(doip::DIAGNOSTIC_MESSAGE, &p)
}

/// A newer car behind DoIP: engine 0x1001, gearbox 0x1002; the engine only
/// speaks OBD on UDS when classic is false.
struct FakeDoipCar {
    classic: bool,
}

impl DiagLink for FakeDoipCar {
    fn request(
        &mut self,
        target: u16,
        data: &[u8],
        _timeout_ms: u64,
    ) -> Result<Vec<DiagAnswer>, String> {
        let mut answers = Vec::new();
        let functional = target == doip::OBD_FUNCTIONAL;
        let mut answer = |source: u16, d: Vec<u8>| {
            if functional || target == source {
                answers.push(DiagAnswer { source, data: d });
            }
        };
        if self.classic {
            if data == [0x01, 0x00] {
                answer(0x1001, vec![0x41, 0x00, 0xBE, 0x3F, 0xA8, 0x13]);
                answer(0x1002, vec![0x41, 0x00, 0x80, 0x00, 0x00, 0x01]);
            }
            if data == [0x03] {
                answer(0x1001, vec![0x43, 0x01, 0x04, 0x20]);
            }
            if data == [0x09, 0x02] {
                answer(0x1001, vin_answer(&[0x49, 0x02, 0x01]));
            }
            if data == [0x01, 0x42] {
                answer(0x1001, vec![0x41, 0x42, 0x31, 0x9C]);
            }
        } else {
            if data[0] == 0x01 {
                answer(0x1001, vec![0x7F, 0x01, 0x11]);
            }
            if data == [0x22, 0xF4, 0x00] {
                answer(0x1001, vec![0x62, 0xF4, 0x00, 0xBE, 0x3F, 0xA8, 0x13]);
            }
            if data == [0x22, 0xF8, 0x02] {
                answer(0x1001, vin_answer(&[0x62, 0xF8, 0x02]));
            }
        }
        if answers.is_empty() {
            Err("no answer".into())
        } else {
            Ok(answers)
        }
    }
}

#[test]
fn doip_frames_and_the_vehicle_announcement() {
    let f = doip::encode(
        doip::DIAGNOSTIC_MESSAGE,
        &[0x0E, 0x80, 0x10, 0x01, 0x3E, 0x00],
    );
    assert_eq!(
        f,
        [
            0x02, 0xFD, 0x80, 0x01, 0, 0, 0, 6, 0x0E, 0x80, 0x10, 0x01, 0x3E, 0x00
        ]
    );
    assert_eq!(
        doip::encode_with_version(doip::VEHICLE_IDENT_REQUEST, &[], 0xFF),
        [0xFF, 0x00, 0x00, 0x01, 0, 0, 0, 0]
    );

    let mut parser = doip::Parser::new();
    parser.feed(&f[..5]);
    assert!(parser.next().is_none());
    parser.feed(&f[5..]);
    let m = parser.next().unwrap();
    assert_eq!(m.payload_type, doip::DIAGNOSTIC_MESSAGE);
    assert_eq!(m.payload.len(), 6);

    let mut bad = doip::Parser::new();
    bad.feed(&[0x02, 0x02, 0x80, 0x01, 0, 0, 0, 0]);
    assert!(bad.next().is_none());
    assert!(bad.broken());

    let mut announcement = VIN.as_bytes().to_vec();
    announcement.extend_from_slice(&[0x10, 0x10, 1, 2, 3, 4, 5, 6, 1, 2, 3, 4, 5, 6, 0x00]);
    let e = doip::parse_announcement(&announcement).unwrap();
    assert_eq!(e.vin, VIN);
    assert_eq!(e.logical_address, Some(0x1010));
    assert!(doip::parse_announcement(&[0; 10]).is_none());
}

#[test]
fn doip_session_routing_acknowledgement_alive_check_functional_and_refusals() {
    let car = FakeDoipVehicle::new();
    car.0.lock().unwrap().respond = Some(Box::new(|target, uds| {
        if target == 0x1001 && uds == [0x22, 0xF1, 0x90] {
            return vec![
                diag(0x1001, &[0x7F, 0x22, 0x78]),
                diag(0x1001, &vin_answer(&[0x62, 0xF1, 0x90])),
            ];
        }
        if target == doip::OBD_FUNCTIONAL && uds == [0x01, 0x00] {
            return vec![
                diag(0x1001, &[0x41, 0x00, 0xBE]),
                diag(0x1002, &[0x41, 0x00, 0x80]),
            ];
        }
        if target == 0x2000 {
            return vec![message(
                doip::DIAGNOSTIC_NACK,
                &[0x20, 0x00, 0x0E, 0x80, 0x03],
            )];
        }
        Vec::new()
    }));
    let mut session = DoipSession::new(Box::new(car.clone()));
    session.activate().unwrap();
    {
        let side = car.0.lock().unwrap();
        assert_eq!(side.sent[0].payload_type, doip::ROUTING_ACTIVATION_REQUEST);
        assert_eq!(side.sent[0].payload[0], 0x0E);
    }

    let answers = session.request(0x1001, &[0x22, 0xF1, 0x90], 500).unwrap();
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].data[0], 0x62);
    // The alive check after the routing answer was answered with our address.
    assert!(
        car.0
            .lock()
            .unwrap()
            .sent
            .iter()
            .any(|m| m.payload_type == doip::ALIVE_CHECK_RESPONSE && m.payload == [0x0E, 0x80])
    );

    let answers = session
        .request(doip::OBD_FUNCTIONAL, &[0x01, 0x00], 500)
        .unwrap();
    assert_eq!(answers.len(), 2);

    let error = session.request(0x2000, &[0x3E, 0x00], 500).unwrap_err();
    assert!(error.contains("0x03"), "{error}");
    assert!(session.request(0x3000, &[0x3E, 0x00], 100).is_err());

    // Routing denied (0x06: unsupported activation type).
    let denying = FakeDoipVehicle::new();
    denying.0.lock().unwrap().routing_code = 0x06;
    let mut refused = DoipSession::new(Box::new(denying));
    let error = refused.activate().unwrap_err();
    assert!(error.contains("0x06"), "{error}");
}

#[test]
fn doip_front_end_obd_through_the_functional_address_also_on_uds() {
    for classic in [true, false] {
        let car = FakeDoipCar { classic };
        let mut elm = MiniElm::new(EnetElm::new(Box::new(car), "DoIP test", Flavor::Doip));
        elm.connect("6").unwrap();
        assert_eq!(elm.obd_on_uds, !classic);
        assert!(elm.set_headers(true));
        assert_eq!(elm.read_vin(), VIN, "classic {classic}");
        if classic {
            let faults = elm.read_faults();
            assert_eq!(faults.len(), 1);
            assert_eq!(faults[0].1, "P0420");
            assert_eq!(faults[0].0, "7E8");
            assert_eq!(
                elm.send("ATRV", 2000).first().map(String::as_str),
                Some("12.7V")
            );
        }
    }
}

#[test]
fn network_car_without_a_car_says_why() {
    // Nothing listens on this address (TEST-NET-1, RFC 5737): both tries fail.
    let error = caruspex_net::open_network_car("192.0.2.1x").err().unwrap();
    assert!(error.contains('\n'));
    assert!(error.contains("192.0.2.1x"));
}
