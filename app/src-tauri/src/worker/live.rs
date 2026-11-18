// Live values: the measurement loop, one slice per tick, and the
// manufacturer and user values (CustomPids) that share its ticks.
use caruspex_i18n::l;
use caruspex_record::{
    CustomPid, CustomPidReader, USER_KEY_BASE, built_in_packs, obdb_pid, parse_torque_csv,
};
use serde_json::json;

use super::Worker;
use super::convert::to_row;
use crate::rows::*;
use crate::texts::{args, w};

impl Worker {
    pub(super) fn start_live(&mut self, pids: Vec<i32>) {
        if self.truck_connected() {
            // A truck sends its values by itself: each tick listens.
            self.poll_timer
                .set_interval(self.settings.poll_interval_ms.max(0) as u64);
            self.poll_timer.start();
            self.log(l(
                "Messung gestartet, die J1939-Werte werden mitgehört.",
                "Measurement started, listening to the J1939 values.",
            ));
            return;
        }
        self.poll_pids = pids;
        if !self.obd_ready || self.poll_pids.is_empty() {
            return;
        }
        self.poll_index = 0;
        // No faster than the configured interval. On the K-Line a request
        // takes longer anyway, but a fast CAN adapter or the simulator would
        // otherwise flood the interface with events.
        self.poll_timer
            .set_interval(self.settings.poll_interval_ms.max(0) as u64);
        self.poll_timer.start();
        self.log(args(
            &w(
                "Messung gestartet, %1 Werte im Umlauf.",
                "Measurement started, cycling through %1 values.",
            ),
            &[&self.poll_pids.len()],
        ));
    }

    pub(super) fn stop_live(&mut self) {
        self.custom_poll_keys.clear();
        if self.poll_timer.active {
            self.poll_timer.stop();
            self.log(w("Messung gestoppt.", "Measurement stopped."));
        }
    }

    // One slice per tick. That keeps the loop free and stop requests from
    // the interface get through immediately.
    pub(super) fn poll_tick(&mut self) {
        if self.truck_connected() {
            self.truck_tick();
            return;
        }
        if !self.obd_ready {
            return;
        }
        // Every other tick belongs to the manufacturer values when some are
        // selected. They change slowly and cost an addressing round each time.
        if !self.custom_poll_keys.is_empty() {
            self.custom_turn = !self.custom_turn;
            if self.custom_turn || self.poll_pids.is_empty() {
                let keys = self.custom_poll_keys.clone();
                self.read_custom_now(&keys);
                return;
            }
        }
        if self.poll_pids.is_empty() {
            return;
        }
        // Up to six PIDs per tick: on CAN they go out as one request,
        // elsewhere the core reads them one by one. With the batched
        // commands of an STN chip four such requests share one round trip,
        // so a tick takes four times as many.
        let per_tick = if self.elm.stn_batch() { 24 } else { 6 };
        let mut slice = Vec::new();
        let n = self.poll_pids.len();
        for _ in 0..per_tick.min(n) {
            slice.push(self.poll_pids[self.poll_index % n]);
            self.poll_index = (self.poll_index + 1) % n;
        }
        for value in self.with_obd(|o| o.read_live_batch(&slice, false)) {
            self.emit_live(to_row(&value));
        }
    }

    // --- Manufacturer and user values (CustomPids) ------------------------------

    fn find_custom(&self, key: i32) -> Option<CustomPid> {
        built_in_packs()
            .iter()
            .find(|p| p.key == key)
            .or_else(|| self.user_pids.iter().find(|p| p.key == key))
            .cloned()
            .or_else(|| obdb_pid(key))
    }

    /// Reads custom values and emits them; returns (ok, failed, first error).
    fn read_custom_now(&mut self, keys: &[i32]) -> (usize, usize, String) {
        let list: Vec<CustomPid> = keys.iter().filter_map(|&k| self.find_custom(k)).collect();
        let mut ok = 0;
        let mut bad = 0;
        let mut first_error = String::new();
        let readings = {
            // The reader restores the normal OBD addressing when it goes out
            // of scope, so the next OBD request works as before.
            let mut reader = CustomPidReader::new(&mut self.elm);
            reader.read_all(&list)
        };
        for r in readings {
            if r.ok {
                ok += 1;
                self.emit_live(to_row(&r.value));
            } else {
                bad += 1;
                if first_error.is_empty() {
                    first_error = r.error;
                }
            }
        }
        (ok, bad, first_error)
    }

    pub(super) fn read_custom(&mut self, keys: &[i32]) {
        if !self.obd_ready {
            self.emit(
                "custom_read",
                json!({ "ok": 0, "failed": keys.len(), "firstError": w("Erst verbinden.", "Connect first.") }),
            );
            return;
        }
        self.busy(true);
        let (ok, failed, error) = self.read_custom_now(keys);
        self.emit(
            "custom_read",
            json!({ "ok": ok, "failed": failed, "firstError": error }),
        );
        self.busy(false);
    }

    pub(super) fn start_custom_poll(&mut self, keys: Vec<i32>) {
        if !self.obd_ready {
            return;
        }
        self.custom_poll_keys = keys;
        self.poll_timer
            .set_interval(self.settings.poll_interval_ms.max(0) as u64);
        if !self.custom_poll_keys.is_empty() && !self.poll_timer.active {
            self.poll_timer.start();
        }
    }

    pub(super) fn import_custom_csv(&mut self, path: &str) {
        let data = match std::fs::read(path) {
            Ok(data) => data,
            Err(e) => {
                self.emit(
                    "custom_imported",
                    json!({ "rows": [], "errors": [args(&w("Datei lässt sich nicht öffnen: %1", "The file cannot be opened: %1"), &[&e])] }),
                );
                return;
            }
        };
        // New imports get keys after the ones already loaded.
        let base = USER_KEY_BASE + self.user_pids.len() as i32;
        let (parsed, problems) = parse_torque_csv(&String::from_utf8_lossy(&data), base);
        let mut rows = Vec::new();
        for mut p in parsed {
            p.pack.clear();
            rows.push(CustomRow {
                key: p.key,
                name: p.name.clone(),
                short_name: p.short_name.clone(),
                unit: p.unit.clone(),
                pack: String::new(),
                source: p.source.clone(),
                request: p.request.clone(),
                header: p.header.clone(),
                short_name_en: String::new(),
            });
            self.user_pids.push(p);
        }
        self.emit(
            "custom_imported",
            json!({ "rows": rows, "errors": problems }),
        );
    }
}
