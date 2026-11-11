//! Small timing helpers.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

fn start() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// Milliseconds since an arbitrary starting point, monotonically increasing.
pub fn now_ms() -> u64 {
    start().elapsed().as_millis() as u64
}

pub fn sleep_ms(milliseconds: u64) {
    std::thread::sleep(Duration::from_millis(milliseconds));
}

/// Waits until a fixed point in time instead of for a duration.
///
/// Operating systems like to sleep a few milliseconds too long. The 5-baud
/// wake-up address strings ten bits together, so the overshoot adds up. With a
/// fixed target per bit the error stays the same for every bit instead of
/// accumulating.
pub fn sleep_until_ms(deadline: u64) {
    loop {
        let now = now_ms();
        if now >= deadline {
            return;
        }
        let remaining = deadline - now;
        // Busy-wait for the last bit, otherwise we oversleep the deadline.
        if remaining > 3 {
            std::thread::sleep(Duration::from_millis(remaining - 2));
        } else {
            std::thread::yield_now();
        }
    }
}
