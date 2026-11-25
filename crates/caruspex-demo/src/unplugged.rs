//! A transport whose cable is pulled after a while, to see and test what a
//! lost connection looks like.

use std::time::{Duration, Instant};

use caruspex_link::Transport;

/// Passes everything on to `inner` until its time is over. From then on
/// every write fails, every read waits for nothing, and it says that it is
/// lost, like a serial port whose cable was pulled.
pub struct Unplugged<T> {
    inner: T,
    at: Instant,
}

impl<T: Transport> Unplugged<T> {
    pub fn after(inner: T, seconds: f64) -> Self {
        // Longer than a day is as good as never.
        let wait = Duration::from_secs_f64(seconds.clamp(0.0, 86_400.0));
        Self {
            inner,
            at: Instant::now() + wait,
        }
    }

    fn pulled(&self) -> bool {
        Instant::now() >= self.at
    }
}

impl<T: Transport> Transport for Unplugged<T> {
    fn write(&mut self, data: &[u8]) -> bool {
        !self.pulled() && self.inner.write(data)
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        if self.pulled() {
            std::thread::sleep(Duration::from_millis(timeout_ms));
            return Vec::new();
        }
        self.inner.read(timeout_ms)
    }
    fn flush_input(&mut self) {
        self.inner.flush_input();
    }
    fn name(&self) -> String {
        self.inner.name()
    }
    fn lost(&mut self) -> bool {
        self.pulled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DemoOptions, DemoTransport};

    #[test]
    fn the_cable_comes_off_after_its_time() {
        let quick = || {
            DemoTransport::new(DemoOptions {
                latency_scale: 0.0,
                ..DemoOptions::default()
            })
        };
        let mut plugged = Unplugged::after(quick(), 3600.0);
        assert!(!plugged.lost());
        assert!(plugged.write(b"ATI\r"));
        assert!(!plugged.read(100).is_empty());
        let mut pulled = Unplugged::after(quick(), 0.0);
        assert!(pulled.lost());
        assert!(!pulled.write(b"ATI\r"));
        assert!(pulled.read(1).is_empty());
    }
}
