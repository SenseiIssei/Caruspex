//! A counting allocator for the performance tests (docs/performance.md).
//! Test binaries only: the programs use the system allocator untouched.
//!
//! Counts are kept per thread, so tests running in parallel in the same
//! binary do not see each other's allocations. Code wrapped in [`paused`]
//! is not counted: the simulated car behind the adapter allocates as it
//! likes, the measurement is about the code that talks to it.

#![allow(dead_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

pub struct Counting;

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
    static BYTES: Cell<u64> = const { Cell::new(0) };
    // Bytes allocated minus bytes freed on this thread, paused or not.
    static LIVE: Cell<i64> = const { Cell::new(0) };
    static PAUSED: Cell<u32> = const { Cell::new(0) };
}

fn count(size: usize) {
    let _ = LIVE.try_with(|l| l.set(l.get() + size as i64));
    if PAUSED.try_with(|p| p.get()).unwrap_or(1) == 0 {
        let _ = ALLOCS.try_with(|a| a.set(a.get() + 1));
        let _ = BYTES.try_with(|b| b.set(b.get() + size as u64));
    }
}

fn uncount(size: usize) {
    let _ = LIVE.try_with(|l| l.set(l.get() - size as i64));
}

// SAFETY: every call is passed on to the system allocator unchanged; the
// counters are plain thread-local cells that never allocate themselves.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        uncount(layout.size());
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        uncount(layout.size());
        count(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Allocation count, bytes allocated and live bytes of this thread so far.
#[derive(Clone, Copy, Debug, Default)]
pub struct Snapshot {
    pub allocs: u64,
    pub bytes: u64,
    pub live: i64,
}

pub fn snapshot() -> Snapshot {
    Snapshot {
        allocs: ALLOCS.with(Cell::get),
        bytes: BYTES.with(Cell::get),
        live: LIVE.with(Cell::get),
    }
}

impl Snapshot {
    pub fn since(self, earlier: Snapshot) -> Snapshot {
        Snapshot {
            allocs: self.allocs - earlier.allocs,
            bytes: self.bytes - earlier.bytes,
            live: self.live - earlier.live,
        }
    }
}

/// Runs `f` without counting its allocations (they still count as live).
pub fn paused<R>(f: impl FnOnce() -> R) -> R {
    PAUSED.with(|p| p.set(p.get() + 1));
    let r = f();
    PAUSED.with(|p| p.set(p.get() - 1));
    r
}

/// A transport whose own work is not counted: the simulated adapter and car.
pub struct Uncounted<T>(pub T);

impl<T: caruspex_link::Transport> caruspex_link::Transport for Uncounted<T> {
    fn write(&mut self, data: &[u8]) -> bool {
        paused(|| self.0.write(data))
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        paused(|| self.0.read(timeout_ms))
    }
    fn flush_input(&mut self) {
        paused(|| self.0.flush_input())
    }
    fn name(&self) -> String {
        self.0.name()
    }
}
