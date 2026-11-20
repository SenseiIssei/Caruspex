//! Small timing helpers.

use std::cell::Cell;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

thread_local! {
    /// Microseconds of a simulated clock on this thread, or None: while a
    /// simulator runs here, the short pauses of the protocol code move its
    /// clock instead of the thread, so a slow machine cannot make a
    /// simulated control unit see a reply as late.
    // It is const already; on Android (emulated thread locals) clippy
    // still asks for it.
    #[allow(clippy::missing_const_for_thread_local)]
    static VIRTUAL_US: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Switches this thread to a simulated clock starting at zero (true) or back
/// to real time (false).
pub fn use_virtual_clock(on: bool) {
    VIRTUAL_US.with(|c| c.set(on.then_some(0)));
}

/// The simulated clock of this thread, if one is in use.
pub fn virtual_now_us() -> Option<u64> {
    VIRTUAL_US.with(Cell::get)
}

/// Moves the simulated clock of this thread forward; no effect in real time.
pub fn advance_virtual_us(us: u64) {
    VIRTUAL_US.with(|c| {
        if let Some(now) = c.get() {
            c.set(Some(now + us));
        }
    });
}

fn start() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// Milliseconds since an arbitrary starting point, monotonically increasing.
pub fn now_ms() -> u64 {
    start().elapsed().as_millis() as u64
}

pub fn sleep_ms(milliseconds: u64) {
    if virtual_now_us().is_some() {
        advance_virtual_us(milliseconds * 1000);
        return;
    }
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
