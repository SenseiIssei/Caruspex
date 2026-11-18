// The health check and the used vehicle check of trucks over SAE J1939.
//
// Both listen to what the units broadcast, ask for what comes only on
// request (engine hours, idle hours, fuel, odometers, DM21, VINs and
// identification) and hand it to analysis::truck_health and truck_used.
// The results go out as the same events and rows as the car checks, so the
// pages only switch their texts and tiles.
use std::sync::atomic::Ordering;

use caruspex_analysis::truck_health::{
    TruckFault, TruckHealthInput, TruckHealthReport, TruckLamps, TruckUnit, run_truck_health_check,
    spn,
};
use caruspex_analysis::truck_used::{
    LampHistory, OdometerReading, SPN_DASHBOARD_DISTANCE, SPN_FINE_DISTANCE, TruckUsedInput,
    TruckUsedReport, UnitIdentity, UnitVin, evaluate_used_truck,
};
use caruspex_analysis::used_car::{HealthSummary, normalise_vin};
use caruspex_demo::DemoTransport;
use caruspex_i18n::l;
use caruspex_protocols::j1939::params::Reading;
use caruspex_protocols::j1939::{J1939Value, UnitFaults, address_label, describe};
use caruspex_vehicles::vin::decode_vin;
use serde_json::json;

use super::truck::{LISTEN_MS, TruckFaults};
use super::{Worker, app_seconds};
use crate::rows::*;
use crate::texts::{args, w};

fn faults_of(unit: Option<&UnitFaults>) -> Vec<TruckFault> {
    unit.map(|u| {
        u.dtcs
            .iter()
            .map(|d| TruckFault {
                spn: d.spn,
                fmi: d.fmi,
                text: describe(d.spn, d.fmi),
                occurrences: d.occurrences,
            })
            .collect()
    })
    .unwrap_or_default()
}

/// Every unit heard with its lamps, active and previous faults.
fn analysis_units(found: &TruckFaults) -> Vec<TruckUnit> {
    found
        .units
        .iter()
        .map(|u| {
            let dm1 = found.active.iter().find(|a| a.address == u.address);
            let dm2 = found.previous.iter().find(|a| a.address == u.address);
            let lamps = dm1.map(|d| d.lamps).unwrap_or_default();
            TruckUnit {
                address: u.address,
                label: u.label.clone(),
                lamps: TruckLamps {
                    mil: lamps.mil.on(),
                    red_stop: lamps.red_stop.on(),
                    amber: lamps.amber_warning.on(),
                    protect: lamps.protect.on(),
                },
                active: faults_of(dm1),
                previous: faults_of(dm2),
            }
        })
        .collect()
}

/// The value of an SPN, the engine (lowest address) first.
fn reading(values: &[J1939Value], wanted: u32) -> Option<f64> {
    values
        .iter()
        .filter(|v| v.spn == wanted)
        .find_map(|v| match v.reading {
            Reading::Value(x) => Some(x),
            _ => None,
        })
}

fn health_row(report: &TruckHealthReport) -> HealthRow {
    HealthRow {
        score: report.score,
        grade: report.grade as i32,
        summary: report.summary.text(),
        checks_run: report.checks_run,
        diesel: true,
        truck: true,
        system_volts: report.system_volts,
        findings: report
            .findings
            .iter()
            .map(|f| HealthFindingRow {
                id: f.id.to_string(),
                level: f.level as i32,
                area: String::new(),
                title: f.title.text(),
                explanation: f.explanation.text(),
                advice: f.advice.text(),
            })
            .collect(),
        not_checked: report
            .not_checked
            .iter()
            .map(|s| format!("{}: {}", s.title.text(), s.reason.text()))
            .collect(),
        ..HealthRow::default()
    }
}

fn used_truck_row(input: &TruckUsedInput, result: &TruckUsedReport) -> UsedCarRow {
    let report = &result.report;
    let facts = &result.facts;
    let vin = input
        .vins
        .first()
        .map(|v| normalise_vin(&v.vin))
        .unwrap_or_default();
    let info = decode_vin(&vin, 0);
    let vehicle = if vin == DemoTransport::truck_vin() {
        l("Caruspex Demo-Lkw", "Caruspex demo truck").to_string()
    } else {
        info.brand.clone()
    };
    let active: usize = input.faults.iter().map(|u| u.active.len()).sum();
    let previous: usize = input.faults.iter().map(|u| u.previous.len()).sum();
    let first = input.vins.first().map(|v| normalise_vin(&v.vin));
    UsedCarRow {
        light: report.light as i32,
        headline: report.headline.text(),
        summary: report.summary.text(),
        vin,
        papers_vin: normalise_vin(&input.papers_vin),
        vehicle,
        model_year: if info.valid { info.model_year } else { 0 },
        mileage: report
            .mileage
            .iter()
            .map(|m| UsedCarMileageRow {
                source: m.source.clone(),
                km: m.km,
            })
            .collect(),
        distance_since_clear_km: input.lamp_history.since_clear_km.unwrap_or(-1.0),
        minutes_since_clear: input.lamp_history.since_clear_minutes.unwrap_or(-1.0),
        stored_faults: if input.faults_read { active as i32 } else { -1 },
        pending_faults: 0,
        permanent_faults: 0,
        health_score: input.health.as_ref().map_or(-1, |h| h.score),
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
        truck: Some(TruckUsedFactsRow {
            engine_hours: facts.engine_hours.unwrap_or(-1.0),
            idle_share: facts.idle_share.map_or(-1.0, |s| s * 100.0),
            average_speed: facts.average_speed.unwrap_or(-1.0),
            fuel_per_100km: facts.fuel_per_100km.unwrap_or(-1.0),
            active_faults: if input.faults_read { active as i32 } else { -1 },
            previous_faults: if input.previous_read {
                previous as i32
            } else {
                -1
            },
            vin_sources: input.vins.len() as i32,
            vins_agree: input
                .vins
                .iter()
                .all(|v| Some(normalise_vin(&v.vin)) == first),
            units: input.units.len() as i32,
        }),
        ..UsedCarRow::default()
    }
}

impl Worker {
    /// Listens for `seconds` and samples every value the health rules read,
    /// the engine's first. `progress` gets the seconds elapsed.
    fn sample_truck(
        &mut self,
        input: &mut TruckHealthInput,
        seconds: f64,
        progress: &dyn Fn(&Worker, f64),
    ) {
        let start = app_seconds();
        while app_seconds() - start < seconds && !self.cancelled() {
            let heard = self.with_truck(|s, link| {
                s.listen(link, LISTEN_MS)?;
                Ok::<_, String>(s.values())
            });
            let values = match heard {
                Some(Ok(values)) => values,
                Some(Err(error)) => {
                    self.log(args(
                        l("J1939: Mithören gestört: %1", "J1939: listening failed: %1"),
                        &[&error],
                    ));
                    break;
                }
                None => break,
            };
            let t = app_seconds() - start;
            for wanted in spn::SAMPLED {
                match values.iter().find(|v| v.spn == wanted).map(|v| v.reading) {
                    Some(Reading::Value(x)) => input.add_value(wanted, x, t),
                    Some(Reading::Error) => input.add_sensor_error(wanted),
                    _ => {}
                }
            }
            self.emit_truck_values();
            progress(self, app_seconds() - start);
        }
    }

    /// The health check of a truck: lamps and faults, the counters, then
    /// `seconds` of broadcast values.
    pub(super) fn run_truck_health_check(&mut self, seconds: i32) {
        self.stop_live();
        self.cancel.store(false, Ordering::SeqCst);
        self.busy(true);
        let progress = |me: &Worker, step: String, percent: i32| {
            me.emit(
                "health_progress",
                json!({ "step": step, "percent": percent }),
            );
        };
        let mut input = TruckHealthInput::default();

        progress(
            self,
            l(
                "Warnlampen und Fehlerspeicher",
                "Warning lamps and fault memory",
            )
            .to_string(),
            5,
        );
        match self.load_truck_faults() {
            Some(Ok(found)) => {
                input.faults_read = true;
                input.units = analysis_units(&found);
            }
            Some(Err(error)) => {
                self.log(args(l("Fehlerspeicher: %1", "Fault memory: %1"), &[&error]))
            }
            None => {}
        }

        progress(self, l("Betriebsstunden", "Engine hours").to_string(), 15);
        if let Some(Err(error)) = self.with_truck(|s, link| s.read_counters(link)) {
            self.log(args(
                l("Betriebsstunden: %1", "Engine hours: %1"),
                &[&error],
            ));
        }

        let total = f64::from(seconds.max(1));
        self.sample_truck(&mut input, total, &|me, elapsed| {
            let left = (total - elapsed).max(0.0) as i32;
            progress(
                me,
                args(
                    &w(
                        "Messwerte sammeln, noch %1 s",
                        "Collecting values, %1 s left",
                    ),
                    &[&left],
                ),
                (25 + (70.0 * elapsed / total) as i32).min(95),
            );
        });
        input.engine_hours = self
            .with_truck(|s, _| s.values())
            .and_then(|values| reading(&values, spn::ENGINE_HOURS));

        let report = run_truck_health_check(&input);
        progress(self, w("Fertig", "Done"), 100);
        self.emit("health_result", json!({ "row": health_row(&report) }));
        self.busy(false);
    }

    /// The used vehicle check of a truck: VIN and identification of every
    /// unit, the odometers, hours, idle and fuel counters, DM21, faults and
    /// a short health check.
    pub(super) fn run_truck_used_check(&mut self, papers_vin: &str) {
        self.stop_live();
        self.cancel.store(false, Ordering::SeqCst);
        self.busy(true);
        let progress = |me: &Worker, step: String, percent: i32| {
            me.emit(
                "used_car_progress",
                json!({ "step": step, "percent": percent }),
            );
        };
        let mut input = TruckUsedInput {
            papers_vin: papers_vin.to_string(),
            ..TruckUsedInput::default()
        };

        progress(
            self,
            l(
                "Fahrgestellnummer jedes Steuergeräts",
                "VIN of every control unit",
            )
            .to_string(),
            4,
        );
        if let Some(Ok(vins)) = self.with_truck(|s, link| s.read_vins(link)) {
            input.vins = vins
                .into_iter()
                .map(|(address, vin)| UnitVin {
                    label: address_label(address),
                    vin,
                })
                .collect();
        }
        self.emit(
            "vehicle_info",
            json!({
                "vin": input.vins.first().map(|v| v.vin.clone()).unwrap_or_default(),
                "freezeCode": "",
                "freezeRows": Vec::<LiveRow>::new(),
            }),
        );

        progress(
            self,
            l(
                "Kennung der Steuergeräte",
                "Identification of the control units",
            )
            .to_string(),
            12,
        );
        let ids = self.with_truck(|s, link| -> Result<_, String> {
            Ok((s.component_ids(link)?, s.software_ids(link)?))
        });
        let (components, software) = match ids {
            Some(Ok(found)) => found,
            _ => (Vec::new(), Vec::new()),
        };
        self.emit_truck_ids(&components, &software);

        progress(self, w("Fehlerspeicher", "Fault memory"), 22);
        let mut health = TruckHealthInput::default();
        match self.load_truck_faults() {
            Some(Ok(found)) => {
                input.faults_read = true;
                input.previous_read = !found.previous.is_empty();
                input.faults = analysis_units(&found);
                health.faults_read = true;
                health.units = input.faults.clone();
            }
            Some(Err(error)) => {
                self.log(args(l("Fehlerspeicher: %1", "Fault memory: %1"), &[&error]))
            }
            None => {}
        }
        let heard = self.with_truck(|s, _| s.units()).unwrap_or_default();
        input.units = heard
            .iter()
            .map(|u| {
                let c = components
                    .iter()
                    .find(|(a, _)| *a == u.address)
                    .map(|(_, c)| c.clone())
                    .unwrap_or_default();
                UnitIdentity {
                    label: u.label.clone(),
                    make: c.make,
                    model: c.model,
                    serial: c.serial,
                    software: software
                        .iter()
                        .find(|(a, _)| *a == u.address)
                        .map(|(_, s)| s.clone())
                        .unwrap_or_default(),
                }
            })
            .collect();

        progress(
            self,
            l(
                "Kilometer, Betriebsstunden, Leerlauf und Kraftstoff",
                "Kilometres, engine hours, idling and fuel",
            )
            .to_string(),
            32,
        );
        let counters = self.with_truck(|s, link| -> Result<_, String> {
            s.read_counters(link)?;
            s.read_usage(link)?;
            Ok(s.values())
        });
        let values = match counters {
            Some(Ok(values)) => values,
            Some(Err(error)) => {
                self.log(args(l("Zähler: %1", "Counters: %1"), &[&error]));
                self.with_truck(|s, _| s.values()).unwrap_or_default()
            }
            None => Vec::new(),
        };
        input.odometers = values
            .iter()
            .filter(|v| v.spn == SPN_DASHBOARD_DISTANCE || v.spn == SPN_FINE_DISTANCE)
            .filter_map(|v| match v.reading {
                Reading::Value(km) if km > 0.0 => Some(OdometerReading {
                    label: address_label(v.source),
                    spn: v.spn,
                    km,
                }),
                _ => None,
            })
            .collect();
        input.engine_hours = reading(&values, spn::ENGINE_HOURS);
        input.idle_hours = reading(&values, 235);
        input.fuel_total_l = reading(&values, 250);
        input.idle_fuel_l = reading(&values, 236);
        input.lamp_history = LampHistory {
            mil_km: reading(&values, 3069),
            since_clear_km: reading(&values, 3294),
            mil_minutes: reading(&values, 3295),
            since_clear_minutes: reading(&values, 3296),
        };

        // A short health check along the way.
        let seconds = f64::from(self.used_car_live_seconds.max(1));
        self.sample_truck(&mut health, seconds, &|me, elapsed| {
            let left = (seconds - elapsed).max(0.0) as i32;
            progress(
                me,
                args(
                    &w(
                        "Messwerte sammeln, noch %1 s",
                        "Collecting values, %1 s left",
                    ),
                    &[&left],
                ),
                (45 + (50.0 * elapsed / seconds) as i32).min(95),
            );
        });
        if !self.cancelled() {
            health.engine_hours = input.engine_hours;
            let report = run_truck_health_check(&health);
            input.health = Some(HealthSummary {
                score: report.score,
                grade: report.grade,
                summary: report.summary.text(),
            });
        }

        let result = evaluate_used_truck(&input);
        let row = used_truck_row(&input, &result);
        progress(self, w("Fertig", "Done"), 100);
        self.emit("used_car_result", json!({ "row": row }));
        self.busy(false);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};

    use caruspex_demo::{DemoClock, DemoOptions, DemoVehicle};
    use serde_json::Value;

    use super::*;
    use crate::settings::AppSettings;
    use crate::worker::{Job, Sink};

    #[derive(Default)]
    struct Collect(Mutex<Vec<(String, Value)>>);

    impl Sink for Collect {
        fn emit(&self, event: &str, payload: Value) {
            self.0.lock().unwrap().push((event.to_string(), payload));
        }
    }

    impl Collect {
        fn take(&self, name: &str) -> Vec<Value> {
            let mut all = self.0.lock().unwrap();
            let (wanted, rest): (Vec<_>, Vec<_>) = all.drain(..).partition(|(e, _)| e == name);
            *all = rest;
            wanted.into_iter().map(|(_, v)| v).collect()
        }
    }

    fn truck_worker(odometer_mismatch: bool) -> (Worker, Arc<Collect>) {
        let sink = Arc::new(Collect::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
        worker.apply_settings(AppSettings::default());
        let clock = DemoClock::new(600.0);
        let options = DemoOptions {
            clock: Some(clock.source()),
            latency_scale: 0.0,
            vehicle: DemoVehicle::Truck,
            truck_odometer_mismatch: odometer_mismatch,
            ..DemoOptions::default()
        };
        worker.handle(Job::SetTruck(Some(0)));
        worker
            .try_connect(Box::new(DemoTransport::new(options)), "")
            .expect("the demo truck answers");
        worker.truck_session.set_waits(300, 700);
        worker.used_car_live_seconds = 1;
        (worker, sink)
    }

    fn finding<'a>(rows: &'a [Value], id: &str) -> &'a Value {
        rows.iter()
            .find(|f| f["id"] == id)
            .unwrap_or_else(|| panic!("no finding {id} in {rows:?}"))
    }

    #[test]
    fn health_check_of_the_demo_truck() {
        let (mut worker, sink) = truck_worker(false);
        worker.handle(Job::HealthCheck(1));
        let row = sink.take("health_result")[0]["row"].clone();
        assert_eq!(row["truck"], true);
        assert_eq!(row["systemVolts"], 24);
        // Amber: active faults, coolant level, particulate filter, the
        // intake temperature sensor; nothing red. The lamp the faults
        // explain counts once.
        assert_eq!(row["grade"], 1, "{row}");
        let findings = row["findings"].as_array().unwrap();
        assert_eq!(finding(findings, "lamp_amber")["level"], 1);
        for id in ["faults_unit", "coolant_level", "dpf", "intake_temp"] {
            assert_eq!(finding(findings, id)["level"], 2, "{id}");
        }
        for id in [
            "coolant_temp",
            "oil_temp",
            "oil_pressure",
            "charging",
            "def_level",
            "engine_hours",
        ] {
            assert_eq!(finding(findings, id)["level"], 0, "{id}");
        }
        // The earlier AdBlue level fault is mentioned, not counted as amber.
        assert_eq!(finding(findings, "def_faults")["level"], 1);
        assert!(!sink.take("health_progress").is_empty());
        assert!(!sink.take("j1939_units").is_empty());
    }

    #[test]
    fn used_check_of_the_demo_truck() {
        let (mut worker, sink) = truck_worker(false);
        worker.handle(Job::UsedCarCheck(DemoTransport::truck_vin().to_string()));
        let row = sink.take("used_car_result")[0]["row"].clone();
        assert_eq!(row["vin"], DemoTransport::truck_vin());
        assert_eq!(row["light"], 1, "{row}");
        let items = row["items"].as_array().unwrap();
        let level = |id: &str| finding(items, id)["level"].as_i64().unwrap();
        assert_eq!(level("vins_agree"), 0);
        assert_eq!(level("vin_matches_papers"), 0);
        assert_eq!(level("units"), 0);
        assert_eq!(level("mileage_consistent"), 0);
        assert_eq!(level("speed_plausible"), 0);
        assert_eq!(level("idle_share"), 1);
        assert_eq!(level("fuel_average"), 0);
        assert_eq!(level("not_cleared_recently"), 0);
        assert_eq!(level("faults_active"), 2);
        assert!(
            finding(items, "units")["explanation"]
                .as_str()
                .unwrap()
                .contains("CX-DTCO4")
        );
        // Dashboard, engine and tachograph.
        assert_eq!(row["mileage"].as_array().unwrap().len(), 3);
        let truck = &row["truck"];
        assert_eq!(truck["vinSources"], 3);
        assert_eq!(truck["vinsAgree"], true);
        assert_eq!(truck["units"], 5);
        assert!((truck["engineHours"].as_f64().unwrap() - 12_345.8).abs() < 1.0);
        assert!((truck["idleShare"].as_f64().unwrap() - 32.0).abs() < 0.5);
        assert!((truck["averageSpeed"].as_f64().unwrap() - 41.5).abs() < 0.2);
        assert!(row["healthScore"].as_i64().unwrap() > 0);
        assert!(
            row["questions"].as_array().unwrap().iter().any(|q| q
                .as_str()
                .unwrap()
                .contains("aktiven Fehler")
                || q.as_str().unwrap().contains("active faults"))
        );
    }

    #[test]
    fn a_changed_dashboard_turns_the_used_check_red() {
        let (mut worker, sink) = truck_worker(true);
        worker.handle(Job::UsedCarCheck(String::new()));
        let row = sink.take("used_car_result")[0]["row"].clone();
        assert_eq!(row["light"], 2);
        let items = row["items"].as_array().unwrap();
        assert_eq!(finding(items, "mileage_mismatch")["level"], 3);
        let km: Vec<f64> = row["mileage"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["km"].as_f64().unwrap())
            .collect();
        let spread = km.iter().cloned().fold(f64::MIN, f64::max)
            - km.iter().cloned().fold(f64::MAX, f64::min);
        assert!((spread - 200_000.0).abs() < 50.0, "{km:?}");
    }
}
