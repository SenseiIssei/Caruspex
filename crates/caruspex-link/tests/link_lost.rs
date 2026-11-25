//! A way to the adapter that goes away: the cable is pulled, the connection
//! closed. The driver notices without sending anything, and nothing waits
//! for its whole timeout any more.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use caruspex_link::elm327::ElmStatus;
use caruspex_link::{Elm327, Transport};
use common::FaultTransport;

/// A pulled USB cable: writes fail, reads wait out their time.
struct Pulled {
    inner: FaultTransport,
    gone: Arc<AtomicBool>,
    writes: Arc<AtomicUsize>,
}

impl Transport for Pulled {
    fn write(&mut self, data: &[u8]) -> bool {
        self.writes.fetch_add(1, Ordering::SeqCst);
        !self.gone.load(Ordering::SeqCst) && self.inner.write(data)
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        if self.gone.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(timeout_ms));
            return Vec::new();
        }
        self.inner.read(timeout_ms)
    }
    fn flush_input(&mut self) {
        self.inner.flush_input();
    }
    fn name(&self) -> String {
        "pulled".into()
    }
    fn lost(&mut self) -> bool {
        self.gone.load(Ordering::SeqCst)
    }
}

fn connected() -> (Elm327, Arc<AtomicBool>, Arc<AtomicUsize>) {
    let gone = Arc::new(AtomicBool::new(false));
    let writes = Arc::new(AtomicUsize::new(0));
    let mut elm = Elm327::new();
    elm.connect(
        Box::new(Pulled {
            inner: FaultTransport::default(),
            gone: gone.clone(),
            writes: writes.clone(),
        }),
        "6",
    )
    .expect("connects");
    (elm, gone, writes)
}

#[test]
fn an_idle_link_is_looked_at_without_a_byte() {
    let (mut elm, gone, writes) = connected();
    let before = writes.load(Ordering::SeqCst);
    assert!(!elm.link_lost());
    gone.store(true, Ordering::SeqCst);
    assert!(elm.link_lost());
    assert!(elm.known_lost());
    assert_eq!(writes.load(Ordering::SeqCst), before);
}

#[test]
fn a_command_on_a_pulled_cable_ends_at_once_and_later_ones_are_not_sent() {
    let (mut elm, gone, writes) = connected();
    gone.store(true, Ordering::SeqCst);
    let started = Instant::now();
    let result = elm.request("0100", 0x01, 6000);
    // Six seconds before: the whole timeout of the request.
    assert!(
        started.elapsed() < Duration::from_millis(600),
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(result.status, ElmStatus::AdapterError);
    assert!(elm.known_lost());

    // A scan over a hundred units ends instead of waiting for each of them.
    let before = writes.load(Ordering::SeqCst);
    let started = Instant::now();
    for _ in 0..100 {
        assert_eq!(
            elm.request("22F190", 0x22, 6000).status,
            ElmStatus::AdapterError
        );
    }
    assert!(started.elapsed() < Duration::from_millis(200));
    assert_eq!(writes.load(Ordering::SeqCst), before);

    // Disconnecting does not send the farewell either.
    let started = Instant::now();
    elm.disconnect();
    assert!(started.elapsed() < Duration::from_millis(200));
    assert_eq!(writes.load(Ordering::SeqCst), before);
    assert!(!elm.known_lost(), "a new connection starts fresh");
}

#[test]
fn listening_ends_when_the_link_goes() {
    let (mut elm, gone, _) = connected();
    gone.store(true, Ordering::SeqCst);
    let started = Instant::now();
    assert!(elm.monitor_lines(5000, 10).is_empty());
    assert!(started.elapsed() < Duration::from_millis(600));
    assert!(elm.known_lost());
}

#[test]
fn an_adapter_that_answers_the_probe_is_not_gone() {
    let (mut elm, gone, _) = connected();
    assert!(elm.answers(2, 300));
    assert!(!elm.silent());
    gone.store(true, Ordering::SeqCst);
    assert!(!elm.answers(2, 300));
}

#[test]
fn a_car_that_does_not_answer_is_no_lost_link() {
    let (mut elm, _, _) = connected();
    // NO DATA is an answer of the adapter.
    assert_eq!(elm.request("0A", 0x0A, 1000).status, ElmStatus::NoData);
    assert!(!elm.silent());
    assert!(!elm.link_lost());
}
