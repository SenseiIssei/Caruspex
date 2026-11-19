//! The ELM327 driver over a Bluetooth LE channel as Android hands it over:
//! answers arrive cut into notifications of the MTU, commands leave cut into
//! writes of the MTU. The fake adapter behind it is the scripted ELM327 of
//! the other tests.

mod common;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use caruspex_link::Transport;
use caruspex_link::ble_gatt::{GattLink, GattTransport, payload_size};
use caruspex_link::{Elm327, ElmStatus};
use common::*;

/// A GATT link to a scripted adapter that delivers one notification per
/// read, so every answer reaches the driver in pieces.
struct NotifyingAdapter {
    adapter: FakeAdapter,
    payload: usize,
    pending: VecDeque<Vec<u8>>,
    longest_write: Arc<Mutex<usize>>,
}

impl GattLink for NotifyingAdapter {
    fn write_value(&mut self, value: &[u8]) -> bool {
        assert!(value.len() <= self.payload, "a write longer than the MTU");
        let mut longest = self.longest_write.lock().unwrap();
        *longest = (*longest).max(value.len());
        self.adapter.write(value)
    }

    fn notifications(&mut self, timeout_ms: u64) -> Vec<u8> {
        if self.pending.is_empty() {
            let bytes = self.adapter.read(timeout_ms);
            self.pending
                .extend(bytes.chunks(self.payload).map(<[u8]>::to_vec));
        }
        self.pending.pop_front().unwrap_or_default()
    }

    fn clear(&mut self) {
        self.pending.clear();
        self.adapter.flush_input();
    }

    fn closed(&self) -> bool {
        false
    }
}

fn over_ble(mtu: u32) -> (Elm327, Arc<Mutex<usize>>) {
    let longest = Arc::new(Mutex::new(0));
    let link = NotifyingAdapter {
        // With a gearbox: two control units answer, several lines.
        adapter: FakeAdapter::new(DemoAdapter::CloneV15, true, 0.0),
        payload: payload_size(mtu),
        pending: VecDeque::new(),
        longest_write: longest.clone(),
    };
    let transport = GattTransport::new(link, mtu, "Bluetooth LE OBDII".into());
    let mut elm = Elm327::new();
    elm.connect(Box::new(transport), "6")
        .expect("connects over the notifications");
    (elm, longest)
}

#[test]
fn the_driver_connects_and_reads_through_minimum_mtu_notifications() {
    let (mut elm, longest) = over_ble(23);
    assert!(elm.connected());
    assert_eq!(elm.transport_name(), "Bluetooth LE OBDII");
    let pids = elm.request("0100", 0x01, 2000);
    assert_eq!(pids.status, ElmStatus::Ok, "{}", pids.message);
    assert_eq!(pids.messages.len(), 2, "engine and gearbox");
    // A command longer than one write arrives in pieces and still counts
    // as one line (the adapter ignores the spaces).
    let spread = "0 1          0          0";
    assert!(spread.len() > 20);
    let again = elm.send(spread, 2000);
    assert_eq!(again.status, ElmStatus::Ok, "{}", again.message);
    assert_eq!(*longest.lock().unwrap(), 20);
}

#[test]
fn a_larger_mtu_sends_whole_commands() {
    let (mut elm, longest) = over_ble(185);
    let spread = "0 1          0          0";
    let pids = elm.send(spread, 2000);
    assert_eq!(pids.status, ElmStatus::Ok, "{}", pids.message);
    assert_eq!(*longest.lock().unwrap(), spread.len() + 1);
}
