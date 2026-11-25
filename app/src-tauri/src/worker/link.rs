// The state of the way to the adapter: noticing that it is gone (cable
// pulled, Wi-Fi out of range, adapter hung) and giving up connecting when
// the user cancels.
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use caruspex_i18n::l;
use caruspex_net::CanLink;
use serde_json::json;

use super::Worker;
#[cfg(target_os = "android")]
use crate::android::SerialTransport;
#[cfg(not(target_os = "android"))]
use caruspex_link::SerialTransport;

/// How often an idle connection is looked at. The look asks the operating
/// system only, nothing goes to the adapter.
const LINK_CHECK_MS: u64 = 2000;
/// An adapter that said nothing at all to a command is asked for its name
/// this often, this long each, before it counts as gone.
const PROBE_TRIES: u32 = 2;
const PROBE_MS: u64 = 1500;

/// Runs `work`, which may block for long (opening a port, searching for
/// Bluetooth devices), on a thread of its own and waits for it. `None` as
/// soon as `stop` goes up: the thread is left to itself and its result is
/// dropped when it comes, which closes what it opened.
pub(super) fn abandonable<T: Send + 'static>(
    stop: &AtomicBool,
    work: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = channel();
    let thread = std::thread::Builder::new()
        .name("caruspex-open".into())
        .spawn(move || {
            let _ = tx.send(work());
        })
        .expect("thread starts");
    loop {
        if stop.load(Ordering::SeqCst) {
            return None;
        }
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(result) => return Some(result),
            Err(RecvTimeoutError::Timeout) => {}
            // The thread panicked: so would the worker without it.
            Err(RecvTimeoutError::Disconnected) => match thread.join() {
                Err(panic) => std::panic::resume_unwind(panic),
                Ok(()) => return None,
            },
        }
    }
}

/// Opens a serial port, unless Disconnect comes first. Opening the COM port
/// of a Bluetooth adapter out of range takes Windows many seconds. The next
/// try of the same port waits (cancellably) until an abandoned one is done:
/// otherwise the port would look as if another program held it.
pub(super) fn open_serial_port(
    stop: &AtomicBool,
    device: &str,
    baud: u32,
) -> Option<Result<SerialTransport, String>> {
    static OPENING: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    let turn = OPENING
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry(device.to_string())
        .or_default()
        .clone();
    let device = device.to_string();
    abandonable(stop, move || {
        let _turn = turn.lock().unwrap_or_else(|e| e.into_inner());
        let mut port = SerialTransport::new();
        port.open(&device, baud).map(|()| port)
    })
}

impl Worker {
    /// Disconnect was asked for while something runs.
    pub(super) fn interrupted(&self) -> bool {
        self.elm.interrupted()
    }

    /// Ends a connection attempt the user cancelled; true if so. The close
    /// that follows in the queue tells the interface.
    pub(super) fn stop_connecting(&mut self) -> bool {
        if !self.interrupted() {
            return false;
        }
        self.busy(false);
        self.log(l("Verbinden abgebrochen.", "Connecting cancelled."));
        true
    }

    /// The way to the adapter is known to be gone. Sends nothing.
    pub(super) fn link_gone(&mut self) -> bool {
        self.elm.link_lost() || self.can_link.as_mut().is_some_and(|can| can.lost())
    }

    /// After every job and tick: reports a link that went away under it.
    pub(super) fn check_link(&mut self) {
        if !self.elm.connected() && !self.truck_connected() {
            self.link_timer.stop();
            return;
        }
        if self.elm.known_lost() {
            self.lose_link(l(
                "Die Verbindung zum Adapter ist abgerissen. Kabel ab, WLAN oder Bluetooth außer Reichweite?",
                "The connection to the adapter broke off. Cable unplugged, Wi-Fi or Bluetooth out of range?",
            ));
        } else if self.elm.silent()
            && self.route.kind != "replay"
            && !self.interrupted()
            && !self.elm.answers(PROBE_TRIES, PROBE_MS)
            && !self.interrupted()
        {
            self.lose_link(l(
                "Der Adapter antwortet nicht mehr. Steckt er noch, und hat er Strom?",
                "The adapter no longer answers. Is it still plugged in, and does it have power?",
            ));
        }
    }

    /// Connected: from now on the link is watched.
    pub(super) fn watch_link(&mut self) {
        self.link_timer.set_interval(LINK_CHECK_MS);
        self.link_timer.start();
    }

    /// The look at an idle connection.
    pub(super) fn link_tick(&mut self) {
        if self.link_gone() {
            self.elm.give_up();
        }
    }

    /// The connection is over without anybody having asked for it: what
    /// still runs is stopped, nothing more is sent, and the interface gets
    /// the reason with "closed".
    pub(super) fn lose_link(&mut self, reason: &str) {
        self.elm.give_up();
        self.log(reason);
        if caruspex_record::active_recording(&self.elm).is_some() {
            // What was recorded until now is worth keeping.
            self.stop_recording();
        }
        self.end_connection();
        self.emit("closed", json!({ "lost": reason }));
    }

    /// Stops everything that uses the connection and lets go of it.
    pub(super) fn end_connection(&mut self) {
        self.link_timer.stop();
        self.voltage_timer.stop();
        self.stop_live();
        self.forget_can_link();
        self.elm.disconnect();
        self.obd_ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn work_that_hangs_is_left_behind_as_soon_as_somebody_stops_it() {
        let stop = Arc::new(AtomicBool::new(false));
        let raiser = stop.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            raiser.store(true, Ordering::SeqCst);
        });
        let dropped = Arc::new(AtomicBool::new(false));
        struct Port(Arc<AtomicBool>);
        impl Drop for Port {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let port = Port(dropped.clone());
        let started = Instant::now();
        let result = abandonable(&stop, move || {
            std::thread::sleep(Duration::from_millis(700));
            port
        });
        assert!(result.is_none());
        assert!(started.elapsed() < Duration::from_millis(500));
        // What the abandoned thread opened in the end is closed again.
        let until = Instant::now() + Duration::from_secs(5);
        while !dropped.load(Ordering::SeqCst) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[test]
    fn work_that_ends_gives_its_result() {
        let stop = AtomicBool::new(false);
        assert_eq!(abandonable(&stop, || 7), Some(7));
    }

    #[test]
    fn a_port_that_does_not_exist_is_an_error_as_before() {
        let stop = AtomicBool::new(false);
        let result = open_serial_port(&stop, "caruspex-no-such-port", 38400).unwrap();
        assert!(result.is_err());
    }
}
