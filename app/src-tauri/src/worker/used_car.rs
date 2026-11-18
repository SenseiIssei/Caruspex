// The used car check: the VIN against the catalog and the papers, the
// odometers, readiness, faults, a short health check and the battery of
// electric cars, judged by analysis::used_car.
use std::sync::Arc;
use std::sync::atomic::Ordering;

use caruspex_analysis::health_check::{self, HealthInput};
use caruspex_analysis::inputs as ai;
use caruspex_analysis::readiness::readiness_from_status;
use caruspex_analysis::used_car::{
    BatteryFacts, Drive, FaultKind, HealthSummary, MileageReading, UsedCarFault, UsedCarInput,
    UsedCarReport, VinFacts, drive_from_fuel_type, evaluate_used_car, normalise_vin,
};
use caruspex_data::pids::PidCategory;
use caruspex_demo::DemoTransport;
use caruspex_i18n::l;
use caruspex_link::timing::sleep_ms;
use caruspex_record::{
    CustomPid, CustomPidReader, odometer_values, pack_category, pack_title, pack_values_en,
    packs_for_model,
};
use caruspex_vehicles::catalog;
use caruspex_vehicles::vin::{decode_vin, system_year};
use serde_json::json;

use super::convert::{fault_row, is_stored_kind, status_row, to_row};
use super::health::{analysis_fault, analysis_monitor, analysis_status, dtc_level, engine_limits};
use super::{Worker, app_seconds};
use crate::rows::*;
use crate::texts::{args, w};

/// What the VIN says about the car, the packs of manufacturer values that
/// fit it and the drive the catalog knows for sure.
fn used_car_identity(vin: &str) -> (Option<VinFacts>, Vec<&'static str>, Drive) {
    let vin = vin.trim();
    if vin.is_empty() {
        return (None, Vec::new(), Drive::Unknown);
    }
    let info = decode_vin(vin, 0);
    let mut facts = VinFacts {
        vin: info.vin.clone(),
        valid: info.valid,
        check_digit_applies: info.check_digit_applies,
        check_digit_ok: info.check_digit_ok,
        brand: info.brand.clone(),
        model_year: info.model_year,
        ..VinFacts::default()
    };
    // The demo car has a made-up maker code; its pack is the demo pack.
    if info.vin == DemoTransport::vin() {
        facts.vehicle = l("Caruspex Demo-Auto", "Caruspex demo car").to_string();
        facts.model_year = 2021;
        facts.year_from = 2021;
        return (Some(facts), vec!["caruspex-demo"], Drive::Unknown);
    }
    let found = catalog::match_vin(&info);
    if facts.brand.is_empty() {
        facts.brand = found.brand.clone();
    }
    let mut packs = Vec::new();
    let mut drive = Drive::Unknown;
    if let Some(g) = found.generation {
        facts.vehicle = catalog::display_name(g);
        facts.year_from = g.year_from;
        facts.year_to = g.year_to;
        if found.year > 0 {
            facts.model_year = found.year;
        }
        packs = packs_for_model(g.brand, g.model);
        if g.powertrains == catalog::PT_ELECTRIC {
            drive = Drive::Electric;
        }
    }
    (Some(facts), packs, drive)
}

/// Seconds of live values for the health check inside the used car check.
pub(super) const USED_CAR_LIVE_SECONDS: i32 = 10;

fn used_car_row(input: &UsedCarInput, report: &UsedCarReport) -> UsedCarRow {
    let count = |kind: FaultKind| -> i32 {
        if input.faults_read {
            input.faults.iter().filter(|f| f.kind == kind).count() as i32
        } else {
            -1
        }
    };
    let supported: Vec<_> = input.readiness.iter().filter(|m| m.supported).collect();
    let battery = input.battery.as_ref();
    UsedCarRow {
        light: report.light as i32,
        headline: report.headline.text(),
        summary: report.summary.text(),
        vin: input
            .car_vin
            .as_ref()
            .map(|v| v.vin.clone())
            .unwrap_or_default(),
        papers_vin: normalise_vin(&input.papers_vin),
        vehicle: input
            .car_vin
            .as_ref()
            .map(|v| {
                if v.vehicle.is_empty() {
                    v.brand.clone()
                } else {
                    v.vehicle.clone()
                }
            })
            .unwrap_or_default(),
        model_year: input.car_vin.as_ref().map_or(0, |v| v.model_year),
        mileage: report
            .mileage
            .iter()
            .map(|m| UsedCarMileageRow {
                source: m.source.clone(),
                km: m.km,
            })
            .collect(),
        distance_since_clear_km: input.distance_since_clear_km.unwrap_or(-1.0),
        minutes_since_clear: input.minutes_since_clear.unwrap_or(-1.0),
        warm_ups: input.warm_ups.unwrap_or(-1.0),
        readiness_complete: if input.readiness_read {
            supported.iter().filter(|m| m.complete).count() as i32
        } else {
            -1
        },
        readiness_total: if input.readiness_read {
            supported.len() as i32
        } else {
            -1
        },
        stored_faults: count(FaultKind::Stored),
        pending_faults: count(FaultKind::Pending),
        permanent_faults: count(FaultKind::Permanent),
        health_score: input.health.as_ref().map_or(-1, |h| h.score),
        electric: input.drive.electric(),
        battery_pack: battery.map(|b| b.pack.clone()).unwrap_or_default(),
        battery_soh: battery.and_then(|b| b.soh).unwrap_or(-1.0),
        battery_soc: battery.and_then(|b| b.soc).unwrap_or(-1.0),
        items: report
            .items
            .iter()
            .map(|i| UsedCarItemRow {
                id: i.id.to_string(),
                section: i.section.key().to_string(),
                level: i.level as i32,
                title: i.title.text(),
                explanation: i.explanation.text(),
                ask: i.ask.text(),
            })
            .collect(),
        questions: report.questions().iter().map(|q| q.text()).collect(),
        not_checked: report
            .not_checked
            .iter()
            .map(|n| format!("{}: {}", n.title.text(), n.reason.text()))
            .collect(),
        truck: None,
    }
}

impl Worker {
    // The used car check: the VIN against the catalog and the papers, the
    // odometer of every control unit with a documented identifier, when the
    // fault memory was last cleared, readiness, faults, a short health check
    // and the battery of electric cars, judged by analysis::used_car.
    pub(super) fn run_used_car_check(&mut self, papers_vin: &str) {
        if self.truck_connected() {
            self.run_truck_used_check(papers_vin);
            return;
        }
        if !self.obd_ready {
            self.failed(
                l(
                    "Für die Gebrauchtwagen-Prüfung erst verbinden.",
                    "Connect first to check a used car.",
                )
                .to_string(),
            );
            return;
        }
        self.stop_live();
        self.cancel.store(false, Ordering::SeqCst);
        self.busy(true);
        let progress = |me: &Self, step: String, percent: i32| {
            me.emit(
                "used_car_progress",
                json!({ "step": step, "percent": percent }),
            );
        };
        let mut input = UsedCarInput {
            papers_vin: papers_vin.to_string(),
            current_year: system_year(),
            ..UsedCarInput::default()
        };

        progress(
            self,
            l("Fahrgestellnummer", "Vehicle identification number").to_string(),
            4,
        );
        let vin = self.with_obd(|o| o.read_vin());
        let (facts, packs, catalog_drive) = used_car_identity(&vin);
        input.car_vin = facts;
        input.drive = catalog_drive;

        progress(
            self,
            w(
                "Bereitschaft und Fehlerlampe",
                "Readiness and check engine light",
            ),
            10,
        );
        let st = self.with_obd(|o| o.read_status());
        self.emit("status", json!({ "row": status_row(&st) }));
        let status = analysis_status(&st);
        if st.valid {
            input.mil_on = Some(st.mil_on);
            input.readiness_read = true;
            input.readiness = readiness_from_status(&status);
        }

        progress(self, w("Fehlerspeicher", "Fault memory"), 16);
        let read = self.with_obd(|o| o.read_faults());
        input.faults_read = read.is_ok();
        let faults = read.unwrap_or_default();
        self.emit(
            "faults",
            json!({ "rows": faults.iter().map(fault_row).collect::<Vec<_>>() }),
        );
        let permanent = l("dauerhaft", "permanent");
        input.faults = faults
            .iter()
            .map(|f| {
                let stored = is_stored_kind(&f.kind);
                UsedCarFault {
                    code: f.code.clone(),
                    text: f.text.clone(),
                    kind: if stored {
                        FaultKind::Stored
                    } else if f.kind == permanent || f.kind == "permanent" {
                        FaultKind::Permanent
                    } else {
                        FaultKind::Pending
                    },
                    level: dtc_level(&f.code, stored),
                }
            })
            .collect();

        progress(
            self,
            l(
                "Kilometerstand und letztes Löschen",
                "Mileage and last clearing",
            )
            .to_string(),
            22,
        );
        let supported = self.with_obd(|o| o.supported_pids());
        let mut fuel_code = None;
        for pid in [0xA6, 0x31, 0x30, 0x4E, 0x21, 0x51] {
            if !supported.is_empty() && !supported.contains(&pid) {
                continue;
            }
            let Some(value) = self.with_obd(|o| o.read_live(pid)) else {
                continue;
            };
            self.emit_live(to_row(&value));
            let v = Some(value.value);
            match pid {
                0xA6 => input.odometer_km = v,
                0x31 => input.distance_since_clear_km = v,
                0x30 => input.warm_ups = v,
                0x4E => input.minutes_since_clear = v,
                0x21 => input.mil_distance_km = v,
                _ => fuel_code = Some(value.value as i32),
            }
        }
        if let Some(code) = fuel_code {
            let drive = drive_from_fuel_type(code);
            if drive != Drive::Unknown {
                input.drive = drive;
            }
        }

        // The odometer of other control units: only identifiers documented
        // in a pack that fits this car.
        let odometers: Vec<CustomPid> = packs.iter().flat_map(|p| odometer_values(p)).collect();
        if !odometers.is_empty() {
            progress(
                self,
                l(
                    "Kilometerstand in weiteren Steuergeräten",
                    "Mileage in other control units",
                )
                .to_string(),
                30,
            );
            let readings = {
                let mut reader = CustomPidReader::new(&mut self.elm);
                reader.read_all(&odometers)
            };
            for (pid, reading) in odometers.iter().zip(readings) {
                if reading.ok {
                    self.emit_live(to_row(&reading.value));
                    input.other_mileage.push(MileageReading {
                        source: pid.name.clone(),
                        km: reading.value.value,
                    });
                }
            }
        }

        // The traction battery, from the first pack with battery values.
        if input.drive.electric() {
            let battery_pack = packs
                .iter()
                .find(|p| pack_category(p) == PidCategory::HybridEv)
                .copied();
            if let Some(pack) = battery_pack {
                progress(self, l("Akku", "Battery").to_string(), 36);
                input.battery = Some(self.read_battery(pack));
            }
        }

        // A short health check along the way.
        let mut health = HealthInput::default();
        health.set_status(&status);
        health.faults_read = input.faults_read;
        health.faults = faults.iter().map(analysis_fault).collect();
        health.fault_level = Some(Arc::new(|f: &ai::Fault| {
            dtc_level(&f.code, is_stored_kind(&f.kind))
        }));
        progress(
            self,
            w(
                "Bauteilprüfung (Modus 06)",
                "Component monitoring (mode 06)",
            ),
            42,
        );
        health.monitor_results = self
            .with_obd(|o| o.read_monitor_results())
            .unwrap_or_default()
            .iter()
            .map(analysis_monitor)
            .collect();
        health.monitor_results_read = true;
        if let Some(code) = fuel_code {
            health.fuel = health_check::fuel_type_from_obd(code);
        }
        health.engine = engine_limits(&vin, health.fuel);
        let pids: Vec<i32> = health_check::health_live_pids()
            .iter()
            .copied()
            .filter(|p| supported.is_empty() || supported.contains(p))
            .collect();
        let start = app_seconds();
        let live_seconds = self.used_car_live_seconds.max(1);
        let seconds = f64::from(live_seconds);
        while !pids.is_empty() && app_seconds() - start < seconds && !self.cancelled() {
            let values = self.with_obd(|o| o.read_live_batch(&pids, false));
            let t = app_seconds() - start;
            let snapshot: Vec<ai::LiveValue> = values
                .iter()
                .map(|v| ai::LiveValue {
                    pid: v.pid,
                    value: v.value,
                })
                .collect();
            health.add_snapshot(&snapshot, t);
            for value in &values {
                self.emit_live(to_row(value));
            }
            let elapsed = app_seconds() - start;
            let left = (live_seconds - elapsed as i32).max(0);
            progress(
                self,
                args(
                    &w(
                        "Messwerte sammeln, noch %1 s",
                        "Collecting values, %1 s left",
                    ),
                    &[&left],
                ),
                (48 + (47.0 * elapsed / seconds) as i32).min(95),
            );
            if values.is_empty() {
                sleep_ms(100);
            }
        }
        if !self.cancelled() {
            let report = health_check::run_health_check(&health);
            input.health = Some(HealthSummary {
                score: report.score,
                grade: report.grade,
                summary: report.summary.text(),
            });
        }

        let report = evaluate_used_car(&input);
        let row = used_car_row(&input, &report);
        progress(self, w("Fertig", "Done"), 100);
        self.emit("used_car_result", json!({ "row": row }));
        self.busy(false);
    }

    /// Health, charge and cell spread of the traction battery from a pack.
    fn read_battery(&mut self, pack: &str) -> BatteryFacts {
        const WANTED: [&str; 11] = [
            "SOH",
            "SOH LBC",
            "Full kWh",
            "BOL kWh",
            "SOC display",
            "SOC",
            "SOC BMS",
            "SOC LBC",
            "Cell V max",
            "Cell V min",
            "Cell V diff",
        ];
        let values: Vec<_> = pack_values_en(pack)
            .into_iter()
            .filter(|v| WANTED.contains(&v.short_en))
            .collect();
        let pids: Vec<CustomPid> = values.iter().map(|v| v.pid.clone()).collect();
        let readings = {
            let mut reader = CustomPidReader::new(&mut self.elm);
            reader.read_all(&pids)
        };
        let mut facts = BatteryFacts {
            pack: pack_title(pack),
            ..BatteryFacts::default()
        };
        let mut got = std::collections::HashMap::new();
        for (v, reading) in values.iter().zip(readings) {
            if reading.ok {
                self.emit_live(to_row(&reading.value));
                got.insert(v.short_en, reading.value.value);
            } else if facts.error.is_empty() {
                facts.error = reading.error;
            }
        }
        facts.read = !got.is_empty();
        let get = |key: &str| got.get(key).copied();
        facts.soh = get("SOH").or_else(|| get("SOH LBC")).or_else(|| {
            match (get("Full kWh"), get("BOL kWh")) {
                (Some(full), Some(new)) if new > 1.0 => Some(100.0 * full / new),
                _ => None,
            }
        });
        facts.soc = get("SOC display")
            .or_else(|| get("SOC"))
            .or_else(|| get("SOC BMS"))
            .or_else(|| get("SOC LBC"));
        facts.cell_spread_v =
            get("Cell V diff").or_else(|| match (get("Cell V max"), get("Cell V min")) {
                (Some(max), Some(min)) => Some(max - min),
                _ => None,
            });
        facts
    }
}
