//! Recording: a transport between the ELM327 driver and the real one that
//! writes every write and read with a time stamp.

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use caruspex_i18n::l;
use caruspex_link::timing::now_ms;
use caruspex_link::{Elm327, Transport};

use super::json::json_string;
use super::{
    RECORDING_FORMAT, RECORDING_VERSION, RecordingMeta, format_seconds, normalize_command, utc_now,
};

struct Shared {
    inner: Mutex<Option<Box<dyn Transport>>>,
    file: Mutex<Option<File>>,
    path: String,
    error: String,
    start_ms: u64,
    exchanges: AtomicUsize,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Shared {
    fn write_line(&self, line: &[u8]) {
        if let Some(file) = lock(&self.file).as_mut() {
            // Written in one piece, line by line: if the program dies,
            // everything up to the last complete line is still in the file.
            let _ = file.write_all(line);
            let _ = file.flush();
        }
    }

    fn write_event(&self, direction: &str, data: &[u8], bytes: bool) {
        let mut line = format!(
            "{{\"t\":{},\"dir\":\"{direction}\",\"data\":",
            format_seconds(now_ms().saturating_sub(self.start_ms))
        )
        .into_bytes();
        line.extend(json_string(data, bytes));
        line.extend_from_slice(b"}\n");
        self.write_line(&line);
    }

    fn release(&self) -> Option<Box<dyn Transport>> {
        *lock(&self.file) = None;
        lock(&self.inner).take()
    }
}

// Transports living inside an Elm327 are only reachable as `dyn Transport`.
// To find out whether one of them is a recording (and to take the recording
// out again) without downcasting, every RecordingTransport registers itself
// under the address of its heap allocation.
static REGISTRY: Mutex<Option<HashMap<usize, Weak<Shared>>>> = Mutex::new(None);

fn registry_insert(address: usize, shared: &Arc<Shared>) {
    lock(&REGISTRY)
        .get_or_insert_with(HashMap::new)
        .insert(address, Arc::downgrade(shared));
}

fn registry_remove(address: usize) {
    if let Some(map) = lock(&REGISTRY).as_mut() {
        map.remove(&address);
    }
}

fn registry_find(transport: &dyn Transport) -> Option<Arc<Shared>> {
    let address = transport as *const dyn Transport as *const () as usize;
    lock(&REGISTRY)
        .as_ref()
        .and_then(|map| map.get(&address))
        .and_then(Weak::upgrade)
}

/// A cheap reference to a running recording, for the status line and for
/// notes. Stays valid after the recording has ended.
#[derive(Clone)]
pub struct RecordingHandle {
    shared: Arc<Shared>,
}

impl RecordingHandle {
    pub fn is_recording(&self) -> bool {
        lock(&self.shared.file).is_some()
    }
    pub fn error(&self) -> &str {
        &self.shared.error
    }
    pub fn path(&self) -> &str {
        &self.shared.path
    }
    /// Number of commands written so far.
    pub fn exchanges(&self) -> i32 {
        self.shared.exchanges.load(Ordering::Relaxed) as i32
    }
    /// Adds a free-text line to the recording (replay ignores it).
    pub fn note(&self, text: &str) {
        self.shared.write_event("note", text.as_bytes(), false);
    }
}

/// Wraps another transport and writes every write and every non-empty read
/// into a file. Everything is passed through unchanged; if the file cannot
/// be opened, the wrapper simply passes through without recording (check
/// `is_recording()`).
///
/// `new` returns a `Box` on purpose: [`active_recording`] and
/// [`RecordingTransport::unwrap`] find the recording again by the address of
/// the allocation, so do not move it out of the box.
pub struct RecordingTransport {
    shared: Arc<Shared>,
}

impl RecordingTransport {
    pub fn new(
        inner: Box<dyn Transport>,
        path: &str,
        meta: RecordingMeta,
    ) -> Box<RecordingTransport> {
        let start_ms = now_ms();
        let transport_name = inner.name();
        let mut error = String::new();
        let file = match File::create(path) {
            Ok(file) => Some(file),
            Err(_) => {
                error = format!(
                    "{}{}",
                    l(
                        "Die Aufnahme lässt sich nicht anlegen: ",
                        "Cannot create the recording: "
                    ),
                    path
                );
                None
            }
        };
        let shared = Arc::new(Shared {
            inner: Mutex::new(Some(inner)),
            file: Mutex::new(file),
            path: path.to_string(),
            error,
            start_ms,
            exchanges: AtomicUsize::new(0),
        });
        if lock(&shared.file).is_some() {
            let mut header = Vec::new();
            header.extend_from_slice(b"{\"format\":");
            header.extend(json_string(RECORDING_FORMAT.as_bytes(), false));
            header.extend_from_slice(format!(",\"version\":{RECORDING_VERSION}").as_bytes());
            header.extend_from_slice(b",\"created\":");
            header.extend(json_string(utc_now().as_bytes(), false));
            header.extend_from_slice(b",\"transport\":");
            header.extend(json_string(transport_name.as_bytes(), false));
            for (key, value) in [
                ("adapter", &meta.adapter),
                ("protocol", &meta.protocol),
                ("protocolId", &meta.protocol_id),
            ] {
                if !value.is_empty() {
                    header.extend_from_slice(format!(",\"{key}\":").as_bytes());
                    header.extend(json_string(value.as_bytes(), false));
                }
            }
            header.extend_from_slice(b"}\n");
            shared.write_line(&header);
        }
        let boxed = Box::new(RecordingTransport { shared });
        registry_insert(&*boxed as *const RecordingTransport as usize, &boxed.shared);
        boxed
    }

    pub fn handle(&self) -> RecordingHandle {
        RecordingHandle {
            shared: self.shared.clone(),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.handle().is_recording()
    }
    pub fn error(&self) -> &str {
        &self.shared.error
    }
    pub fn path(&self) -> &str {
        &self.shared.path
    }
    /// Number of commands written so far.
    pub fn exchanges(&self) -> i32 {
        self.handle().exchanges()
    }
    /// Adds a free-text line to the recording (replay ignores it).
    pub fn note(&self, text: &str) {
        self.handle().note(text);
    }

    /// Ends the recording and hands back the wrapped transport.
    pub fn release(&mut self) -> Option<Box<dyn Transport>> {
        self.shared.release()
    }

    /// If `transport` is a RecordingTransport, ends the recording and returns
    /// the transport inside it; otherwise returns `transport` unchanged.
    pub fn unwrap(transport: Box<dyn Transport>) -> Box<dyn Transport> {
        if let Some(shared) = registry_find(&*transport)
            && let Some(inner) = shared.release()
        {
            return inner;
        }
        transport
    }
}

impl Drop for RecordingTransport {
    fn drop(&mut self) {
        registry_remove(self as *const RecordingTransport as usize);
    }
}

impl Transport for RecordingTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        self.shared.write_event("tx", data, true);
        if !normalize_command(data).is_empty() {
            self.shared.exchanges.fetch_add(1, Ordering::Relaxed);
        }
        match lock(&self.shared.inner).as_mut() {
            Some(inner) => inner.write(data),
            None => false,
        }
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let chunk = match lock(&self.shared.inner).as_mut() {
            Some(inner) => inner.read(timeout_ms),
            None => return Vec::new(),
        };
        if !chunk.is_empty() {
            self.shared.write_event("rx", &chunk, true);
        }
        chunk
    }

    fn flush_input(&mut self) {
        if let Some(inner) = lock(&self.shared.inner).as_mut() {
            inner.flush_input();
        }
    }

    fn name(&self) -> String {
        lock(&self.shared.inner)
            .as_ref()
            .map(|t| t.name())
            .unwrap_or_default()
    }

    fn count_hint_allowed(&self) -> bool {
        lock(&self.shared.inner)
            .as_ref()
            .is_some_and(|t| t.count_hint_allowed())
    }

    fn lost(&mut self) -> bool {
        lock(&self.shared.inner).as_mut().is_some_and(|t| t.lost())
    }
}

/// The running recording of this session, if any.
pub fn active_recording(elm: &Elm327) -> Option<RecordingHandle> {
    elm.transport()
        .and_then(registry_find)
        .map(|shared| RecordingHandle { shared })
}

/// Starts a recording on an Elm327 that is already connected, without
/// reconnecting. A recording started in the middle of a session lacks the
/// connect sequence, so a few harmless commands follow at once (ATI, the
/// format settings connect already made, ATDP and 0100) and land in the
/// file. That way the recording can be replayed on its own. Must be called
/// from the thread that uses the Elm327, between two requests.
pub fn start_recording(elm: &mut Elm327, path: &str) -> Result<RecordingHandle, String> {
    if !elm.connected() || elm.transport().is_none() {
        return Err(l(
            "Keine Verbindung zum Adapter.",
            "No connection to the adapter.",
        )
        .to_string());
    }
    if active_recording(elm).is_some() {
        return Err(l(
            "Es läuft schon eine Aufnahme.",
            "A recording is already running.",
        )
        .to_string());
    }
    let meta = RecordingMeta {
        adapter: elm.version().to_string(),
        protocol: elm.protocol_text().to_string(),
        protocol_id: if elm.protocol_id() != '0' {
            elm.protocol_id().to_string()
        } else {
            String::new()
        },
    };

    let mut error = String::new();
    let mut started: Option<RecordingHandle> = None;
    elm.wrap_transport(|inner| {
        let mut recording = RecordingTransport::new(inner, path, meta);
        if !recording.is_recording() {
            error = recording.error().to_string();
            return recording.release();
        }
        started = Some(recording.handle());
        Some(recording as Box<dyn Transport>)
    });
    let Some(started) = started else {
        return Err(error);
    };

    // The connect sequence is missing from this recording. These commands
    // stand in for it, in the order Elm327::connect sends them: each one
    // only reads or sets what connect has set already (the timing to the
    // level in effect), and the header setting of the session is restored
    // at the end. With them a replay finds the identification, the format
    // settings, the timing, an answer to 0100 and the protocol, which is all
    // Elm327::connect needs.
    started.note("recording started in a running session");
    let headers = elm.headers_on();
    elm.send("ATI", 1500);
    elm.send("ATE0", 1500);
    elm.send("ATL0", 1500);
    elm.send("ATS0", 1500);
    elm.send("ATH0", 1500);
    if elm.adaptive_timing_level() > 0 {
        elm.send(&format!("ATAT{}", elm.adaptive_timing_level()), 1500);
    }
    if elm.is_stn() {
        elm.send("STI", 1500);
    }
    elm.request("0100", 0x01, 6000);
    elm.send("ATDP", 1500);
    // Last, as connect does: a replay switches the batched commands of an
    // STN chip on too and so finds the batched lines that follow.
    if elm.stn_batch() {
        elm.send("STBC1", 1500);
    }
    if headers {
        elm.set_headers(true);
    }
    started.note("session continues");
    Ok(started)
}

/// Ends a recording started with `start_recording` (or by wrapping the
/// transport before connect). Returns false if none was running.
pub fn stop_recording(elm: &mut Elm327) -> bool {
    if active_recording(elm).is_none() {
        return false;
    }
    elm.wrap_transport(|transport| Some(RecordingTransport::unwrap(transport)));
    true
}
