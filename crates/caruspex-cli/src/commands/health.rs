//! The health check over a few seconds of live values.

use std::sync::Arc;

use caruspex_analysis::common::{FuelType, Level, TrafficLight, level_key};
use caruspex_analysis::health_check::{
    Area, HealthInput, HealthReport, fuel_type_from_obd, health_live_pids, run_health_check,
};
use caruspex_analysis::inputs;
use caruspex_data::dtc::lookup_dtc;
use caruspex_data::severity::Severity;
use caruspex_link::Elm327;
use caruspex_link::timing::now_ms;
use caruspex_protocols::{Fault, Obd2, VehicleStatus};
use caruspex_record::JsonWriter;

use super::{VinContext, is_stored, json_adapter, json_faults, json_status, read_vin_context};
use crate::out::{flush, say};

fn grade_key(grade: TrafficLight) -> &'static str {
    match grade {
        TrafficLight::Green => "green",
        TrafficLight::Yellow => "yellow",
        TrafficLight::Red => "red",
    }
}

fn area_key(area: Area) -> &'static str {
    match area {
        Area::Faults => "faults",
        Area::Readiness => "readiness",
        Area::Mixture => "mixture",
        Area::OxygenSensors => "oxygen_sensors",
        Area::Catalyst => "catalyst",
        Area::Misfire => "misfire",
        Area::Cooling => "cooling",
        Area::Charging => "charging",
        Area::Sensors => "sensors",
        Area::Monitoring => "monitoring",
        Area::History => "history",
    }
}

fn fuel_key(fuel: FuelType) -> &'static str {
    match fuel {
        FuelType::Petrol => "petrol",
        FuelType::Diesel => "diesel",
        FuelType::Unknown => "unknown",
    }
}

/// What the health check reads and finds, shared by `health` and `report`
/// so both ask the car the same questions in the same order.
pub(crate) struct HealthRun {
    pub status: VehicleStatus,
    pub faults: Vec<Fault>,
    /// Why the fault memory could not be read; empty if it was.
    pub faults_error: String,
    pub vin: VinContext,
    pub monitors: Vec<caruspex_protocols::MonitorResult>,
    pub input: HealthInput,
    pub pids: Vec<i32>,
    pub snapshots: i32,
    pub report: HealthReport,
}

/// Status, faults, VIN, mode 06 and a live series of `seconds`, then the
/// health check. `progress` gets the texts of the steps.
pub(crate) fn run_health(obd: &mut Obd2, seconds: i32, progress: &dyn Fn(&str)) -> HealthRun {
    let mut input = HealthInput::default();
    progress("Reading readiness and warning lamp ...");
    let status = obd.read_status();
    input.set_status(&inputs::VehicleStatus {
        mil_on: status.mil_on,
        fault_count: status.fault_count,
        readiness: status
            .readiness
            .iter()
            .map(|r| inputs::ReadinessItem {
                name: r.name.clone(),
                ready: r.ready,
            })
            .collect(),
        valid: status.valid,
        compression_ignition: status.compression_ignition,
    });

    progress("Reading the fault memory ...");
    let read = obd.read_faults();
    // "Not read" and "no faults" are different findings (faultsRead =
    // error.empty() in the C++ version).
    input.faults_read = read.is_ok();
    let faults_error = read.as_ref().err().cloned().unwrap_or_default();
    let faults = read.unwrap_or_default();
    input.faults = faults
        .iter()
        .map(|f| inputs::Fault {
            code: f.code.clone(),
            kind: f.kind.clone(),
            text: f.text.clone(),
            throttle_related: f.throttle_related,
        })
        .collect();
    let vin = read_vin_context(obd);
    let level_brand = vin.brand.clone();
    input.fault_level = Some(Arc::new(move |f: &inputs::Fault| {
        // As in the application: the database knows how urgent a code is,
        // pending codes count one step lower since they may still go away.
        let severity = lookup_dtc(&f.code, &level_brand).severity;
        let mut level = Level::Info;
        if severity == Severity::Critical {
            level = Level::Critical;
        } else if severity == Severity::High || severity == Severity::Medium {
            level = Level::Warning;
        }
        if !is_stored(&f.kind) && level == Level::Warning {
            level = Level::Info;
        }
        level
    }));

    progress("Reading component monitoring (mode 06) ...");
    let monitors = obd.read_monitor_results().unwrap_or_default();
    input.monitor_results = monitors
        .iter()
        .map(|m| inputs::MonitorResult {
            test_id: m.test_id,
            component_id: m.component_id,
            name: m.name.clone(),
            component: m.component.clone(),
            value: m.value,
            minimum: m.minimum,
            maximum: m.maximum,
            unit: m.unit.clone(),
            has_minimum: m.has_minimum,
            has_maximum: m.has_maximum,
            passed: m.passed,
        })
        .collect();
    input.monitor_results_read = true;

    if let Some(fuel_type) = obd.read_live(0x51) {
        input.fuel = fuel_type_from_obd(fuel_type.value as i32);
    }

    // Live series: only what the car offers.
    let supported = obd.supported_pids();
    let pids: Vec<i32> = health_live_pids()
        .iter()
        .copied()
        .filter(|pid| supported.is_empty() || supported.contains(pid))
        .collect();
    progress(&format!(
        "Collecting {} live values for {} s ...",
        pids.len(),
        seconds
    ));
    let mut snapshots = 0;
    let start = now_ms();
    while !pids.is_empty() && now_ms() - start < (seconds as u64) * 1000 {
        let values = obd.read_live_batch(&pids, false);
        if values.is_empty() {
            break;
        }
        let snapshot: Vec<inputs::LiveValue> = values
            .iter()
            .map(|v| inputs::LiveValue {
                pid: v.pid,
                value: v.value,
            })
            .collect();
        input.add_snapshot(&snapshot, (now_ms() - start) as f64 / 1000.0);
        snapshots += 1;
    }

    let report = run_health_check(&input);
    HealthRun {
        status,
        faults,
        faults_error,
        vin,
        monitors,
        input,
        pids,
        snapshots,
        report,
    }
}

pub fn health(elm: &mut Elm327, as_json: bool, seconds: i32) -> i32 {
    let progress = |text: &str| {
        if !as_json {
            say(&format!("{text}\n"));
            flush();
        }
    };
    let mut obd = Obd2::new(elm);
    let HealthRun {
        status,
        faults,
        vin,
        input,
        pids,
        snapshots,
        report,
        ..
    } = run_health(&mut obd, seconds, &progress);
    let brand = vin.brand.clone();

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object();
        json_adapter(&mut json, obd.elm());
        json.field("score", &report.score)
            .field("grade", grade_key(report.grade))
            .field("summary", &report.summary.text())
            .field("checksRun", &report.checks_run)
            .field("fuel", fuel_key(input.fuel))
            .field("brand", &brand)
            .key("liveSeries")
            .begin_object()
            .field("seconds", &seconds)
            .field("snapshots", &snapshots)
            .key("pids")
            .begin_array();
        for pid in &pids {
            json.value(pid);
        }
        json.end_array().end_object().key("findings").begin_array();
        for f in &report.findings {
            json.begin_object()
                .field("id", &f.id)
                .field("level", level_key(f.level))
                .field("area", area_key(f.area))
                .field("title", &f.title.text())
                .field("explanation", &f.explanation.text())
                .field("advice", &f.advice.text())
                .end_object();
        }
        json.end_array().key("notChecked").begin_array();
        for s in &report.not_checked {
            json.begin_object()
                .field("id", &s.id)
                .field("title", &s.title.text())
                .field("reason", &s.reason.text())
                .end_object();
        }
        json.end_array();
        json_status(&mut json, &status);
        json_faults(&mut json, &faults, &brand);
        json.end_object();
        say(&json.text());
        return 0;
    }

    say(&format!(
        "{} snapshots of {} values.\n\n",
        snapshots,
        pids.len()
    ));
    say(&format!(
        "Score {}/100 ({}), {} checks run\n",
        report.score,
        grade_key(report.grade),
        report.checks_run
    ));
    say(&format!("{}\n", report.summary.text()));
    if !report.findings.is_empty() {
        say("\nFindings:\n");
    }
    for f in &report.findings {
        say(&format!("  [{}] {}\n", level_key(f.level), f.title.text()));
        if !f.explanation.is_empty() {
            say(&format!("      {}\n", f.explanation.text()));
        }
        if !f.advice.is_empty() {
            say(&format!("      Advice: {}\n", f.advice.text()));
        }
    }
    if !report.not_checked.is_empty() {
        say("\nNot checked:\n");
    }
    for s in &report.not_checked {
        say(&format!("  {}: {}\n", s.title.text(), s.reason.text()));
    }
    0
}
