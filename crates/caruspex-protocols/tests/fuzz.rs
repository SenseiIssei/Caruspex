//! Fuzz-style property tests of the OBD-II and UDS decoders: a connected
//! ELM327 whose car answers every request with random, half-broken lines.
//! Nothing may panic, whatever the car says. Normal `cargo test`, stable
//! toolchain; PROPTEST_CASES raises the number of cases.

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use caruspex_data::ecu_addresses::ecu_from_ids;
use caruspex_link::{Elm327, Transport};
use caruspex_protocols::{Obd2, Uds};
use proptest::prelude::*;

/// An ELM327 that answers the AT commands like a real one and the car's
/// part from the shared list of random replies.
struct FuzzElm {
    replies: Arc<Mutex<Vec<String>>>,
    next: usize,
    pending: Vec<u8>,
    connected: bool,
}

impl Transport for FuzzElm {
    fn write(&mut self, data: &[u8]) -> bool {
        let command: String = String::from_utf8_lossy(data)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_uppercase();
        let answer = if command.is_empty() {
            String::new()
        } else if command == "ATZ" || command == "ATI" {
            "ELM327 v1.5".to_string()
        } else if command == "ATDP" {
            "ISO 15765-4 (CAN 11/500)".to_string()
        } else if command.starts_with("ST") {
            "?".to_string()
        } else if command.starts_with("AT") && command != "ATRV" {
            "OK".to_string()
        } else if !self.connected && command == "0100" {
            self.connected = true;
            "41 00 BE 3F A8 13".to_string()
        } else {
            let replies = self.replies.lock().unwrap();
            self.next += 1;
            if replies.is_empty() {
                "NO DATA".to_string()
            } else {
                replies[self.next % replies.len()].clone()
            }
        };
        self.pending = format!("{answer}\r\r>").into_bytes();
        true
    }
    fn read(&mut self, _timeout_ms: u64) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }
    fn flush_input(&mut self) {}
    fn name(&self) -> String {
        "fuzz".into()
    }
}

/// The connected driver and the replies its car gives.
type Car = (Elm327, Arc<Mutex<Vec<String>>>);

thread_local! {
    // One connect per thread: connecting waits half a second for a
    // greeting, the cases only change what the car says.
    static ELM: RefCell<Option<Car>> = const { RefCell::new(None) };
}

fn with_car<R>(replies: Vec<String>, f: impl FnOnce(&mut Elm327) -> R) -> R {
    ELM.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            let shared = Arc::new(Mutex::new(Vec::new()));
            let mut elm = Elm327::new();
            elm.connect(
                Box::new(FuzzElm {
                    replies: shared.clone(),
                    next: 0,
                    pending: Vec::new(),
                    connected: false,
                }),
                "6",
            )
            .expect("the fake adapter connects");
            *slot = Some((elm, shared));
        }
        let (elm, shared) = slot.as_mut().unwrap();
        *shared.lock().unwrap() = replies;
        f(elm)
    })
}

/// What a car might say: proper looking OBD and UDS answers with random
/// bytes, multi-frame lines, headers, errors and noise.
fn car_line() -> impl Strategy<Value = String> {
    prop_oneof![
        "4[1-9A] [0-9A-F]{2}( [0-9A-F]{2}){0,8}",
        "(43|47|4A) [0-9A-F]{2}( [0-9A-F]{2}){0,10}",
        "49 0[0-9A] 0[0-9]( [0-9A-F]{2}){0,6}",
        "6[0-9A-F] [0-9A-F]{2}( [0-9A-F]{2}){0,12}",
        "7F [0-9A-F]{2} [0-9A-F]{2}",
        "[0-9A-F]{3}",
        "[0-9A-F]: ?([0-9A-F]{2} ?){0,7}",
        "7E[89A-F] ?[0-9A-F]{2}( ?[0-9A-F]{2}){0,7}",
        Just("NO DATA".to_string()),
        Just("CAN ERROR".to_string()),
        Just("?".to_string()),
        "[ -~]{0,24}",
    ]
}

fn car_reply() -> impl Strategy<Value = String> {
    prop::collection::vec(car_line(), 0..6).prop_map(|lines| lines.join("\r"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(128)
    ))]

    #[test]
    fn obd_decoders_survive_any_car(replies in prop::collection::vec(car_reply(), 1..10), headers in any::<bool>()) {
        with_car(replies, |elm| {
            elm.set_headers(headers);
            let mut obd = Obd2::new(elm);
            let _ = obd.read_faults();
            let _ = obd.read_status();
            let _ = obd.read_status_per_ecu();
            let _ = obd.supported_pids();
            let _ = obd.read_live_values(&[0x04, 0x05, 0x0C, 0x0D, 0x10, 0x14, 0x24, 0x34, 0x4F, 0xA6]);
            let _ = obd.read_live_all(0x0C);
            let _ = obd.read_live_batch(&[0x0C, 0x0D, 0x05, 0x11, 0x2F, 0x46, 0x5C], true);
            let _ = obd.read_vin();
            let _ = obd.read_freeze_frame();
            let _ = obd.read_freeze_frames(3);
            let _ = obd.read_identity();
            let _ = obd.read_ecu_info();
            let _ = obd.read_monitor_results();
            let _ = obd.read_oxygen_sensor_tests();
        });
    }

    #[test]
    fn uds_decoders_survive_any_unit(replies in prop::collection::vec(car_reply(), 1..10), bmw in any::<bool>()) {
        with_car(replies, |elm| {
            let mut uds = Uds::new(elm);
            let ecu = if bmw {
                ecu_from_ids(0x6F1, 0x612, false, "fuzz")
            } else {
                ecu_from_ids(0x7E0, 0x7E8, false, "fuzz")
            };
            let _ = uds.select(&ecu);
            let _ = uds.tester_present();
            let _ = uds.read_dtcs(0xFF);
            let _ = uds.read_info();
            let _ = uds.request(&[0x22, 0xF1, 0x90], 50);
            uds.restore_default();
        });
    }
}

#[test]
fn the_fake_car_reaches_the_decoders() {
    let faults = with_car(vec!["43 01 01 71".to_string()], |elm| {
        elm.set_headers(false);
        Obd2::new(elm).read_faults()
    })
    .unwrap();
    assert!(faults.iter().any(|f| f.code == "P0171"), "{faults:?}");
}
