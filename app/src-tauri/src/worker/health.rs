// The health check (modes 06, 05 and 09 in one go) and the inputs of the
// analysis it shares with the used car check.
use std::sync::Arc;
use std::sync::atomic::Ordering;

use caruspex_analysis::common::{FuelType, Level};
use caruspex_analysis::drive_cycle_guide::drive_cycle_guide;
use caruspex_analysis::health_check::{self, HealthInput};
use caruspex_analysis::inputs as ai;
use caruspex_data::dtc::lookup_dtc;
use caruspex_data::severity::Severity;
use caruspex_link::timing::sleep_ms;
use caruspex_protocols::{Fault, VehicleStatus};
use caruspex_vehicles::vin::{decode_vin, system_year};
use caruspex_vehicles::{catalog, engines};
use serde_json::json;

use super::convert::{fault_row, is_stored_kind, status_row, to_row};
use super::{Worker, app_seconds};
use crate::rows::*;
use crate::texts::{args, w};

pub(super) fn analysis_fault(f: &Fault) -> ai::Fault {
    ai::Fault {
        code: f.code.clone(),
        kind: f.kind.clone(),
        text: f.text.clone(),
        throttle_related: f.throttle_related,
    }
}

pub(super) fn analysis_status(st: &VehicleStatus) -> ai::VehicleStatus {
    ai::VehicleStatus {
        mil_on: st.mil_on,
        fault_count: st.fault_count,
        readiness: st
            .readiness
            .iter()
            .map(|i| ai::ReadinessItem {
                name: i.name.clone(),
                ready: i.ready,
            })
            .collect(),
        valid: st.valid,
        compression_ignition: st.compression_ignition,
    }
}

/// How urgent a fault code is, from the database. Pending codes count one
/// step lower, they may still disappear on their own.
pub(super) fn dtc_level(code: &str, stored: bool) -> Level {
    let severity = lookup_dtc(code, "").severity;
    let mut level = Level::Info;
    if severity == Severity::Critical {
        level = Level::Critical;
    } else if severity == Severity::High || severity == Severity::Medium {
        level = Level::Warning;
    }
    if !stored && level == Level::Warning {
        level = Level::Info;
    }
    level
}

/// Target values for the health check from the engine codes of the car's
/// generation (caruspex_vehicles::engines). The fuel from PID 51 picks petrol
/// or diesel; without it the engines of the generation must agree.
pub(super) fn engine_limits(vin: &str, fuel: FuelType) -> Option<health_check::EngineLimits> {
    let info = decode_vin(vin, system_year());
    if !info.valid {
        return None;
    }
    let g = catalog::match_vin(&info).generation?;
    let fuel = match fuel {
        FuelType::Petrol => Some(engines::EngineFuel::Petrol),
        FuelType::Diesel => Some(engines::EngineFuel::Diesel),
        _ => None,
    };
    let (t, codes) = engines::targets_for(g, fuel)?;
    Some(health_check::EngineLimits {
        codes: engines::codes_text(&codes),
        idle_min: t.idle_min,
        idle_max: t.idle_max,
        coolant_hot: t.coolant_hot,
    })
}

pub(super) fn analysis_monitor(r: &caruspex_protocols::MonitorResult) -> ai::MonitorResult {
    ai::MonitorResult {
        test_id: r.test_id,
        component_id: r.component_id,
        name: r.name.clone(),
        component: r.component.clone(),
        value: r.value,
        minimum: r.minimum,
        maximum: r.maximum,
        unit: r.unit.clone(),
        has_minimum: r.has_minimum,
        has_maximum: r.has_maximum,
        passed: r.passed,
    }
}

impl Worker {
    // Modes 06, 05 and 09 in one go: everything the control unit reveals
    // about itself and its running self-tests.
    pub(super) fn run_health_check(&mut self, seconds: i32) {
        if !self.obd_ready {
            self.failed(w(
                "Für den Gesundheitscheck erst verbinden.",
                "Connect first to run the health check.",
            ));
            return;
        }
        self.stop_live();
        self.cancel.store(false, Ordering::SeqCst);
        self.busy(true);
        let mut input = HealthInput::default();
        let progress = |me: &Self, step: String, percent: i32| {
            me.emit(
                "health_progress",
                json!({ "step": step, "percent": percent }),
            );
        };

        progress(
            self,
            w(
                "Bereitschaft und Fehlerlampe",
                "Readiness and check engine light",
            ),
            4,
        );
        let st = self.with_obd(|o| o.read_status());
        input.set_status(&analysis_status(&st));
        self.emit("status", json!({ "row": status_row(&st) }));

        progress(self, w("Fehlerspeicher", "Fault memory"), 10);
        let read = self.with_obd(|o| o.read_faults());
        // "Not read" is a different finding from "no faults" (faultsRead =
        // error.empty() in Session.cpp).
        input.faults_read = read.is_ok();
        let faults = read.unwrap_or_default();
        input.faults = faults.iter().map(analysis_fault).collect();
        let rows: Vec<FaultRow> = faults.iter().map(fault_row).collect();
        self.emit("faults", json!({ "rows": rows }));
        input.fault_level = Some(Arc::new(|f: &ai::Fault| {
            dtc_level(&f.code, is_stored_kind(&f.kind))
        }));

        progress(
            self,
            w(
                "Bauteilprüfung (Modus 06)",
                "Component monitoring (mode 06)",
            ),
            18,
        );
        input.monitor_results = self
            .with_obd(|o| o.read_monitor_results())
            .unwrap_or_default()
            .iter()
            .map(analysis_monitor)
            .collect();
        input.monitor_results_read = true;

        if let Some(fuel) = self.with_obd(|o| o.read_live(0x51)) {
            input.fuel = health_check::fuel_type_from_obd(fuel.value as i32);
        }
        // Target values of the engine, when the catalog knows its codes.
        let vin = self.with_obd(|o| o.read_vin());
        input.engine = engine_limits(&vin, input.fuel);

        // Live series: only what the car offers.
        let supported = self.with_obd(|o| o.supported_pids());
        let pids: Vec<i32> = health_check::health_live_pids()
            .iter()
            .copied()
            .filter(|p| supported.is_empty() || supported.contains(p))
            .collect();
        let start = app_seconds();
        let total = seconds.max(1);
        while !pids.is_empty() && app_seconds() - start < f64::from(seconds) && !self.cancelled() {
            let values = self.with_obd(|o| o.read_live_batch(&pids, false));
            let t = app_seconds() - start;
            let snapshot: Vec<ai::LiveValue> = values
                .iter()
                .map(|v| ai::LiveValue {
                    pid: v.pid,
                    value: v.value,
                })
                .collect();
            input.add_snapshot(&snapshot, t);
            for value in &values {
                self.emit_live(to_row(value));
            }
            let elapsed = app_seconds() - start;
            let left = (seconds - elapsed as i32).max(0);
            let percent = 25 + (70.0 * elapsed / f64::from(total)) as i32;
            progress(
                self,
                args(
                    &w(
                        "Messwerte sammeln, noch %1 s",
                        "Collecting values, %1 s left",
                    ),
                    &[&left],
                ),
                percent.min(95),
            );
            if values.is_empty() {
                // Nothing answers (adapter gone): do not spin.
                sleep_ms(100);
            }
        }

        let report = health_check::run_health_check(&input);
        let mut row = HealthRow {
            score: report.score,
            grade: report.grade as i32,
            summary: report.summary.text(),
            checks_run: report.checks_run,
            diesel: input.fuel == FuelType::Diesel,
            ..HealthRow::default()
        };
        for f in &report.findings {
            row.findings.push(HealthFindingRow {
                id: f.id.clone(),
                level: f.level as i32,
                area: String::new(),
                title: f.title.text(),
                explanation: f.explanation.text(),
                advice: f.advice.text(),
            });
        }
        for skipped in &report.not_checked {
            row.not_checked.push(format!(
                "{}: {}",
                skipped.title.text(),
                skipped.reason.text()
            ));
        }
        for m in &input.readiness {
            if !m.supported {
                continue;
            }
            let guide = drive_cycle_guide(m.monitor);
            row.monitors.push(MonitorGuideRow {
                name: guide.name.get().to_string(),
                complete: m.complete,
                explanation: guide.explanation.get().to_string(),
                drive_pattern: guide.drive_pattern.get().to_string(),
                typical_time: guide.typical_time.get().to_string(),
                cold_start: guide.needs_cold_start,
            });
        }
        progress(self, w("Fertig", "Done"), 100);
        self.emit("health_result", json!({ "row": row }));
        self.busy(false);
    }
}
