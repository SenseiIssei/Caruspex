//! One-button health check: faults, readiness, live values and mode 06
//! results go in, a score from 0 to 100, a traffic light and a list of plain
//! findings come out.
//!
//! Each rule only runs if its inputs are there. Whatever could not be checked
//! is listed separately, so the result never pretends to know more than the
//! car told us.
//!
//! The rules use widely accepted workshop values. Where practice varies
//! between makes, the text says so instead of pretending precision.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use crate::common::{FuelType, Level, Phrase, TrafficLight, format, number, worse};
use crate::inputs::{Fault, LiveValue, MonitorResult, VehicleStatus};
use crate::readiness::{MonitorState, readiness_from_status};

// The rules, one module per area of the car.
mod electrics;
mod faults;
mod misfire;
mod monitors;
mod oxygen;
mod temperatures;
mod trims;

use electrics::check_voltage;
use faults::{check_faults, check_mil};
use misfire::{check_misfire, check_mode06};
use monitors::{check_history, check_readiness};
use oxygen::check_oxygen_sensors;
use temperatures::{check_cooling, check_idle, check_other_temperatures};
use trims::check_trims;

/// The part of the car a finding is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Area {
    #[default]
    Faults,
    Readiness,
    Mixture,
    OxygenSensors,
    Catalyst,
    Misfire,
    Cooling,
    Charging,
    Sensors,
    /// Mode 06 results.
    Monitoring,
    /// Clearing, distance with lamp on.
    History,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    /// Stable rule id, e.g. "fuel_trim_bank1".
    pub id: String,
    pub level: Level,
    pub area: Area,
    /// Short, e.g. "Gemisch zu mager (Bank 1)".
    pub title: Phrase,
    /// What it means, in plain words.
    pub explanation: Phrase,
    /// What to do next.
    pub advice: Phrase,
}

/// A rule that could not run, with the reason (which values were missing).
#[derive(Clone, Debug, PartialEq)]
pub struct SkippedCheck {
    pub id: String,
    pub title: Phrase,
    pub reason: Phrase,
}

/// One timestamped reading. Time in seconds on any monotonic clock.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TimedValue {
    pub t: f64,
    pub value: f64,
}

/// Maps a fault to its severity.
pub type FaultLevelFn = Arc<dyn Fn(&Fault) -> Level + Send + Sync>;

#[derive(Clone, Default)]
pub struct HealthInput {
    /// Stored, pending and permanent codes as read from the car.
    pub faults: Vec<Fault>,
    /// Tells "no faults" apart from "not read".
    pub faults_read: bool,
    /// How serious each fault is. Supplied by the caller (from the DTC
    /// database); without it every fault counts as a warning.
    pub fault_level: Option<FaultLevelFn>,

    /// Warning lamp state from PID 01. Leave empty if it was not read.
    pub mil_on: Option<bool>,
    /// Readiness monitors (`decode_readiness()` or `readiness_from_status()`).
    /// Empty together with `readiness_read == false` means "not read".
    pub readiness: Vec<MonitorState>,
    pub readiness_read: bool,

    pub fuel: FuelType,

    /// Live values by PID. A single snapshot is a series with one entry.
    /// Values use the units of data/Pids (C, %, V, km, 1/min, g/s).
    pub live: BTreeMap<i32, Vec<TimedValue>>,

    /// Mode 06 results. `monitor_results_read` tells an empty list apart from
    /// "not read".
    pub monitor_results: Vec<MonitorResult>,
    pub monitor_results_read: bool,

    /// Target values of the engine, when the vehicle catalog knows its
    /// engine codes (caruspex_vehicles::engines). Without them the general
    /// limits apply and the idle speed is not judged.
    pub engine: Option<EngineLimits>,
}

/// Target values of one engine (or of several engine codes of a generation
/// that share them).
#[derive(Clone, Debug, PartialEq)]
pub struct EngineLimits {
    /// The engine codes these values are for, space separated ("CAYC CBZB").
    pub codes: String,
    /// Warm idle speed in 1/min, without load.
    pub idle_min: f64,
    pub idle_max: f64,
    /// Coolant temperature from which it counts as very hot, °C. Engines
    /// with a map-controlled thermostat run hotter at part load.
    pub coolant_hot: f64,
}

impl HealthInput {
    /// Fills readiness and MIL from a parsed vehicle status.
    pub fn set_status(&mut self, status: &VehicleStatus) {
        if !status.valid {
            return;
        }
        self.mil_on = Some(status.mil_on);
        self.readiness = readiness_from_status(status);
        self.readiness_read = true;
    }

    /// Adds a snapshot of live values taken at time `t`.
    pub fn add_snapshot(&mut self, values: &[LiveValue], t: f64) {
        // Obd2::readLiveBatch hands out sub-values as well (both sensors of
        // PID 0x67, ...). A PID's series takes its first value, which is the
        // primary one whenever the control unit reports it, like readLive.
        let mut seen: HashSet<i32> = HashSet::new();
        for v in values {
            if seen.insert(v.pid) {
                self.add_value(v.pid, v.value, t);
            }
        }
    }

    pub fn add_value(&mut self, pid: i32, value: f64, t: f64) {
        if !value.is_finite() || !t.is_finite() {
            return;
        }
        self.live
            .entry(pid)
            .or_default()
            .push(TimedValue { t, value });
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealthReport {
    /// 0..100.
    pub score: i32,
    pub grade: TrafficLight,
    /// One or two sentences for the top.
    pub summary: Phrase,
    /// Worst first.
    pub findings: Vec<Finding>,
    pub not_checked: Vec<SkippedCheck>,
    pub checks_run: i32,
}

impl Default for HealthReport {
    fn default() -> Self {
        Self {
            score: 100,
            grade: TrafficLight::Green,
            summary: Phrase::default(),
            findings: Vec::new(),
            not_checked: Vec::new(),
            checks_run: 0,
        }
    }
}

impl HealthReport {
    pub fn find(&self, id: &str) -> Option<&Finding> {
        self.findings.iter().find(|f| f.id == id)
    }
}

/// Mode 01 PIDs the rules look at, for collecting the live series. Ask only
/// those the car supports (`Obd2::supportedPids`).
pub fn health_live_pids() -> &'static [i32] {
    const PIDS: [i32; 24] = [
        0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0C, 0x0D, 0x0F, 0x10, 0x14, 0x15, 0x16, 0x17,
        0x18, 0x19, 0x1F, 0x21, 0x30, 0x31, 0x3C, 0x42, 0x5C,
    ];
    &PIDS
}

/// Fuel type from the value of PID 0x51 (SAE J1979 fuel type coding).
/// Only the codes whose rules are clear are mapped; the rest is Unknown.
pub fn fuel_type_from_obd(code: i32) -> FuelType {
    match code {
        // diesel, hybrid diesel, bifuel running on diesel
        4 | 19 | 23 => FuelType::Diesel,
        // gasoline, bifuel running on gasoline, hybrid gasoline
        1 | 9 | 17 => FuelType::Petrol,
        _ => FuelType::Unknown,
    }
}

/// Thresholds used by the rules, exposed so the UI can show them and tests
/// can refer to them.
pub mod limits {
    /// |STFT + LTFT| in %.
    pub const TRIM_WARN: f64 = 10.0;
    pub const TRIM_BAD: f64 = 20.0;
    /// V, engine running.
    pub const CHARGING_LOW: f64 = 13.2;
    /// V, engine running: not charging.
    pub const CHARGING_NONE: f64 = 12.6;
    /// V, engine running.
    pub const CHARGING_HIGH: f64 = 15.0;
    pub const CHARGING_TOO_HIGH: f64 = 15.5;
    /// V, engine off.
    pub const BATTERY_WEAK: f64 = 12.2;
    pub const BATTERY_FLAT: f64 = 11.8;
    /// C after a long run.
    pub const WARM_COOLANT: f64 = 80.0;
    /// Below this after a long run: stuck open.
    pub const COLD_THERMOSTAT: f64 = 75.0;
    /// "Long run" for the thermostat rule.
    pub const LONG_RUN_SECONDS: f64 = 15.0 * 60.0;
    pub const COOLANT_HOT: f64 = 108.0;
    pub const COOLANT_OVERHEAT: f64 = 115.0;
    pub const OIL_HOT: f64 = 135.0;
    pub const OIL_OVERHEAT: f64 = 150.0;
    pub const CATALYST_HOT: f64 = 900.0;
    /// Mode 06: within 10 % of the limit.
    pub const NEAR_LIMIT_SHARE: f64 = 0.10;
    /// Since codes cleared.
    pub const FEW_WARMUPS: i32 = 5;
    /// Since codes cleared.
    pub const SHORT_DISTANCE_KM: f64 = 50.0;
}

type Series = [TimedValue];

const PID_FUEL_STATUS: i32 = 0x03;
const PID_COOLANT: i32 = 0x05;
const PID_RPM: i32 = 0x0C;
const PID_SPEED: i32 = 0x0D;
const PID_INTAKE_TEMP: i32 = 0x0F;
const PID_RUN_TIME: i32 = 0x1F;
const PID_MIL_DISTANCE: i32 = 0x21;
const PID_WARMUPS: i32 = 0x30;
const PID_CLEAR_DISTANCE: i32 = 0x31;
const PID_CATALYST_TEMP: i32 = 0x3C;
const PID_VOLTAGE: i32 = 0x42;
const PID_OIL_TEMP: i32 = 0x5C;

/// Above this the engine runs.
const RUNNING_RPM: f64 = 400.0;
/// Below this it is off.
const STOPPED_RPM: f64 = 50.0;
/// s, how far apart two PIDs may be read.
const MATCH_WINDOW: f64 = 3.0;
/// C, below that trims are not meaningful.
const WARM_FOR_TRIMS: f64 = 70.0;

// Penalties for the score, per finding.
const PENALTY_INFO: i32 = 3;
const PENALTY_WARNING: i32 = 12;
const PENALTY_CRITICAL: i32 = 30;

fn get(input: &HealthInput, pid: i32) -> Option<&Series> {
    match input.live.get(&pid) {
        Some(v) if !v.is_empty() => Some(v.as_slice()),
        _ => None,
    }
}

/// The reading closest in time to `t`, if one lies within the window.
fn nearest(s: Option<&Series>, t: f64) -> Option<f64> {
    nearest_within(s, t, MATCH_WINDOW)
}

fn nearest_within(s: Option<&Series>, t: f64, window: f64) -> Option<f64> {
    let s = s?;
    let mut best: Option<&TimedValue> = None;
    for v in s {
        match best {
            Some(b) if (v.t - t).abs() >= (b.t - t).abs() => {}
            _ => best = Some(v),
        }
    }
    let best = best?;
    if (best.t - t).abs() > window {
        return None;
    }
    Some(best.value)
}

struct Ctx<'a> {
    input: &'a HealthInput,
    out: HealthReport,
}

impl Ctx<'_> {
    fn add(
        &mut self,
        id: impl Into<String>,
        level: Level,
        area: Area,
        title: Phrase,
        explanation: Phrase,
        advice: Phrase,
    ) {
        self.out.findings.push(Finding {
            id: id.into(),
            level,
            area,
            title,
            explanation,
            advice,
        });
    }

    fn skip(&mut self, id: impl Into<String>, title: Phrase, reason: Phrase) {
        self.out.not_checked.push(SkippedCheck {
            id: id.into(),
            title,
            reason,
        });
    }

    fn ran(&mut self) {
        self.out.checks_run += 1;
    }

    fn diesel(&self) -> bool {
        self.input.fuel == FuelType::Diesel
    }

    /// True if the sample at time `t` was taken with a warm engine in closed
    /// loop, as far as the available values tell.
    fn closed_loop_warm(&self, t: f64) -> bool {
        let input = self.input;
        if let Some(status) = nearest(get(input, PID_FUEL_STATUS), t)
            && status as i32 != 2
        {
            return false;
        }
        if let Some(coolant) = nearest(get(input, PID_COOLANT), t)
            && coolant < WARM_FOR_TRIMS
        {
            return false;
        }
        if let Some(rpm) = nearest(get(input, PID_RPM), t)
            && rpm < RUNNING_RPM
        {
            return false;
        }
        true
    }

    fn has_incomplete_readiness(&self) -> bool {
        self.input
            .readiness
            .iter()
            .any(|m| m.supported && !m.complete)
    }
}

fn none() -> Phrase {
    Phrase::default()
}

// --- Score -----------------------------------------------------------------

fn score(r: &mut HealthReport) {
    let mut penalty = 0;
    let mut worst = Level::Good;
    let mut warnings = 0;
    let mut criticals = 0;
    for f in &r.findings {
        worst = worse(worst, f.level);
        match f.level {
            Level::Info => penalty += PENALTY_INFO,
            Level::Warning => {
                penalty += PENALTY_WARNING;
                warnings += 1;
            }
            Level::Critical => {
                penalty += PENALTY_CRITICAL;
                criticals += 1;
            }
            Level::Good => {}
        }
    }
    let mut s = (100 - penalty).clamp(0, 100);
    // A single serious finding must not hide behind many good ones.
    if worst == Level::Critical {
        s = s.min(49);
    } else if worst == Level::Warning {
        s = s.min(84);
    }
    r.score = s;
    r.grade = match worst {
        Level::Critical => TrafficLight::Red,
        Level::Warning => TrafficLight::Yellow,
        _ => TrafficLight::Green,
    };

    match r.grade {
        TrafficLight::Green => {
            r.summary = format(
                "Alles im grünen Bereich. {} Prüfungen gemacht, nichts, was \
                 Sorgen machen müsste.",
                "All good. {} checks done, nothing to worry about.",
                &[number(f64::from(r.checks_run), 0)],
            );
        }
        TrafficLight::Yellow => {
            r.summary = format(
                "{} Punkte sollten bald angeschaut werden. Nichts davon ist \
                 akut gefährlich.",
                "{} points should be looked at soon. None of them is an \
                 immediate danger.",
                &[number(f64::from(warnings), 0)],
            );
            if warnings == 1 {
                r.summary = format(
                    "Ein Punkt sollte bald angeschaut werden. Er ist nicht akut \
                     gefährlich.",
                    "One point should be looked at soon. It is not an immediate \
                     danger.",
                    &[],
                );
            }
        }
        TrafficLight::Red => {
            r.summary = if criticals == 1 {
                format(
                    "Ein Punkt braucht bald Aufmerksamkeit, bitte zuerst ansehen.",
                    "One point needs attention soon, please look at it first.",
                    &[],
                )
            } else {
                format(
                    "{} Punkte brauchen bald Aufmerksamkeit, bitte zuerst ansehen.",
                    "{} points need attention soon, please look at them first.",
                    &[number(f64::from(criticals), 0)],
                )
            };
        }
    }
    // One skipped check gets its own sentence, "1 Prüfungen" reads wrong.
    if r.not_checked.len() == 1 {
        r.summary = r.summary.clone()
            + format(
                "Eine Prüfung war mangels Daten nicht möglich.",
                "One check was not possible for lack of data.",
                &[],
            );
    } else if !r.not_checked.is_empty() {
        r.summary = r.summary.clone()
            + format(
                "{} Prüfungen waren mangels Daten nicht möglich.",
                "{} checks were not possible for lack of data.",
                &[number(r.not_checked.len() as f64, 0)],
            );
    }
}

pub fn run_health_check(input: &HealthInput) -> HealthReport {
    let mut c = Ctx {
        input,
        out: HealthReport::default(),
    };
    check_faults(&mut c);
    check_mil(&mut c);
    check_readiness(&mut c);
    check_history(&mut c);
    check_trims(&mut c, 1);
    check_trims(&mut c, 2);
    check_oxygen_sensors(&mut c);
    check_voltage(&mut c);
    check_cooling(&mut c);
    check_idle(&mut c);
    check_other_temperatures(&mut c);
    check_mode06(&mut c);
    check_misfire(&mut c);

    let mut report = c.out;
    // Stable sort, worst level first.
    report.findings.sort_by_key(|f| std::cmp::Reverse(f.level));
    score(&mut report);
    report
}
