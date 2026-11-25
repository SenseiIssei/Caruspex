// The fault memory and what the control unit tells about itself: status and
// faults, clearing (also the persistent way), VIN and freeze frames, and the
// component monitoring of mode 06.
use caruspex_i18n::l;
use caruspex_link::elm327::DEFAULT_SEND_TIMEOUT_MS;
use caruspex_link::timing::sleep_ms;
use serde_json::json;

use super::Worker;
use super::convert::{fault_row, status_row, to_row};
use crate::rows::*;
use crate::texts::{args, w};

fn not_read(error: &str) -> String {
    args(
        l(
            "Der Fehlerspeicher ließ sich nicht lesen: %1",
            "The fault memory could not be read: %1",
        ),
        &[&error],
    )
}

impl Worker {
    pub(super) fn refresh_status_and_faults(&mut self) {
        if self.truck_connected() {
            self.refresh_truck_faults();
            return;
        }
        if !self.obd_ready {
            return;
        }
        self.busy(true);
        let st = self.with_obd(|o| o.read_status());
        self.emit("status", json!({ "row": status_row(&st) }));

        let faults = match self.with_obd(|o| o.read_faults()) {
            Ok(faults) => Some(faults),
            // A car that only broadcasts its values (Tesla) has no fault
            // memory over OBD; there is nothing to report as unread.
            Err(_) if self.elm.listen_only() && !self.elm.known_lost() => None,
            Err(error) => {
                // "Not read" must not look like "no faults". When it was the
                // link, check_link has closed the connection with its reason.
                self.busy(false);
                self.check_link();
                if self.elm.connected() && !self.interrupted() {
                    self.failed(not_read(&error));
                }
                return;
            }
        };
        if let Some(faults) = faults {
            let rows: Vec<FaultRow> = faults.iter().map(fault_row).collect();
            let count = rows.len();
            self.emit("faults", json!({ "rows": rows }));
            self.log(if count == 0 {
                w(
                    "Fehlerspeicher gelesen: kein Eintrag.",
                    "Fault memory read: no entries.",
                )
            } else {
                args(
                    &w(
                        "Fehlerspeicher gelesen: %1 Einträge.",
                        "Fault memory read: %1 entries.",
                    ),
                    &[&count],
                )
            });
        }

        let supported: Vec<i32> = self.with_obd(|o| o.supported_pids()).into_iter().collect();
        self.emit("supported_pids", json!({ "pids": supported }));
        self.busy(false);
    }

    pub(super) fn clear_faults(&mut self) {
        if self.truck_connected() {
            self.clear_truck_faults();
            return;
        }
        if !self.obd_ready {
            return;
        }
        self.busy(true);
        let result = self.with_obd(|o| o.clear_faults());
        self.log(match result {
            Ok(()) => w("Fehlerspeicher gelöscht.", "Fault memory cleared."),
            Err(error) => args(
                &w("Löschen fehlgeschlagen: %1", "Clearing failed: %1"),
                &[&error],
            ),
        });
        self.busy(false);
        self.refresh_status_and_faults();
    }

    pub(super) fn read_details(&mut self) {
        if self.truck_connected() {
            self.read_truck_details();
            return;
        }
        if !self.obd_ready {
            return;
        }
        self.busy(true);
        let vin = self.with_obd(|o| o.read_vin());
        // All freeze frames; the first one also goes out the old way for the
        // pages that only know a single frame.
        let frames: Vec<FreezeFrameRow> = self
            .with_obd(|o| o.read_freeze_frames(caruspex_protocols::obd2::DEFAULT_FREEZE_FRAMES))
            .iter()
            .map(|f| FreezeFrameRow {
                frame: f.frame,
                dtc: f.dtc.clone(),
                ecu: f.ecu.clone(),
                values: f.values.iter().map(to_row).collect(),
            })
            .collect();
        let (code, rows) = frames
            .first()
            .map(|f| (f.dtc.clone(), f.values.clone()))
            .unwrap_or_default();
        self.emit(
            "vehicle_info",
            json!({ "vin": vin, "freezeCode": code, "freezeRows": rows }),
        );
        self.emit("freeze_frames", json!({ "frames": frames }));

        // One reading of the values the start page shows as tiles, so they
        // are not empty until a measurement runs.
        let supported = self.with_obd(|o| o.supported_pids());
        for pid in [0x05, 0x2F, 0x42, 0xA6] {
            if !supported.is_empty() && !supported.contains(&pid) {
                continue;
            }
            if let Some(value) = self.with_obd(|o| o.read_live(pid)) {
                self.emit_live(&[to_row(&value)]);
            }
        }
        self.busy(false);
    }

    pub(super) fn read_monitors(&mut self) {
        if !self.obd_ready {
            self.emit(
                "monitor_results",
                json!({ "rows": [], "note": w("Nicht verbunden.", "Not connected.") }),
            );
            return;
        }
        self.busy(true);
        let id = self.with_obd(|o| o.read_identity());
        self.emit(
            "identity",
            json!({ "vin": id.vin, "calibration": id.calibration_id,
                    "checksum": id.calibration_verification, "ecuName": id.ecu_name }),
        );

        let (results, note) = match self.with_obd(|o| o.read_monitor_results()) {
            Ok(list) => (list, String::new()),
            Err(note) => (Vec::new(), note),
        };
        let rows: Vec<MonitorRow> = results
            .iter()
            .map(|r| MonitorRow {
                name: r.name.clone(),
                component: r.component.clone(),
                value: r.value,
                minimum: r.minimum,
                maximum: r.maximum,
                unit: r.unit.clone(),
                has_minimum: r.has_minimum,
                has_maximum: r.has_maximum,
                passed: r.passed,
            })
            .collect();
        let count = rows.len();
        self.emit("monitor_results", json!({ "rows": rows, "note": note }));

        let (tests, oxygen_note) = match self.with_obd(|o| o.read_oxygen_sensor_tests()) {
            Ok(list) => (list, String::new()),
            Err(note) => (Vec::new(), note),
        };
        let oxygen: Vec<OxygenRow> = tests
            .iter()
            .map(|t| OxygenRow {
                name: t.name.clone(),
                sensor: t.sensor,
                value: t.value,
                unit: t.unit.clone(),
            })
            .collect();
        let oxygen_count = oxygen.len();
        self.emit(
            "oxygen_tests",
            json!({ "rows": oxygen, "note": oxygen_note }),
        );

        self.busy(false);
        self.log(args(
            &w(
                "Bauteilüberwachung gelesen: %1 Tests, %2 Sondenprüfungen.",
                "Component monitoring read: %1 tests, %2 O2 sensor tests.",
            ),
            &[&count, &oxygen_count],
        ));
    }

    // Persistent clear over EOBD. Some control units only accept the command
    // right after the connection is established, others need several
    // attempts. That is why the connection is re-established in between.
    pub(super) fn brute_force_clear(&mut self, attempts: i32) {
        if !self.obd_ready {
            self.emit(
                "clear_progress",
                json!({ "line": w("Nicht verbunden.", "Not connected.") }),
            );
            self.emit(
                "clear_finished",
                json!({ "everythingGone": false, "remaining": 0 }),
            );
            return;
        }
        self.busy(true);
        let mut remaining = 0usize;
        for attempt in 1..=attempts {
            if self.elm.known_lost() || self.interrupted() {
                break;
            }
            let line = |s: String| json!({ "line": s });
            self.emit(
                "clear_progress",
                line(args(
                    &w("Versuch %1 von %2 ...", "Attempt %1 of %2 ..."),
                    &[&attempt, &attempts],
                )),
            );
            if let Err(error) = self.with_obd(|o| o.clear_faults()) {
                self.emit(
                    "clear_progress",
                    line(args(
                        &w(
                            "   Steuergerät quittiert nicht: %1",
                            "   Control unit does not acknowledge: %1",
                        ),
                        &[&error],
                    )),
                );
            }
            sleep_ms(600);

            // A memory that cannot be read back is not an empty one.
            let left = self.with_obd(|o| o.read_faults());
            if let Ok(left) = &left {
                remaining = left.len();
            }
            if left.as_ref().is_ok_and(|left| left.is_empty()) {
                self.emit(
                    "clear_progress",
                    line(w("   Speicher ist leer.", "   Memory is empty.")),
                );
                self.busy(false);
                self.emit(
                    "clear_finished",
                    json!({ "everythingGone": true, "remaining": 0 }),
                );
                self.refresh_status_and_faults();
                return;
            }
            self.emit(
                "clear_progress",
                line(match &left {
                    Ok(_) => args(
                        &w("   noch %1 Einträge", "   %1 entries left"),
                        &[&remaining],
                    ),
                    Err(error) => format!("   {}", not_read(error)),
                }),
            );

            // From the second failure on, re-establish the connection. Some
            // control units only open the clear window right after waking up.
            if attempt >= 2 && attempt < attempts {
                self.emit(
                    "clear_progress",
                    line(w(
                        "   Verbindung wird neu aufgebaut ...",
                        "   Re-establishing the connection ...",
                    )),
                );
                let _ = self.elm.send("ATZ", 6000);
                sleep_ms(400);
                for command in ["ATE0", "ATL0", "ATS0", "ATH0"] {
                    self.elm.send(command, DEFAULT_SEND_TIMEOUT_MS);
                }
                self.elm.send("0100", 10000);
                sleep_ms(200);
            }
        }
        self.busy(false);
        self.emit(
            "clear_finished",
            json!({ "everythingGone": false, "remaining": remaining }),
        );
        self.refresh_status_and_faults();
    }
}
