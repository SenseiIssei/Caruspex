//! Health check for trucks and buses over SAE J1939.
//!
//! The caller listens to the bus for a while and hands in the broadcast
//! values by SPN (engine speed, temperatures, oil pressure, voltages,
//! AdBlue, particulate filter, boost), the faults and lamps of every unit
//! and the engine hours. Every value gets a level with a short reason, the
//! same way as the car health check: a score from 0 to 100, a traffic
//! light and the findings, worst first.
//!
//! Only what J1939 really delivers is judged. A value the truck does not
//! send is listed as not checked, with the SPN that was missing.
//!
//! Texts are [`Message`] patterns, so the numbers stay out of the pattern
//! and every language finds its translation.

use std::collections::BTreeMap;

use crate::common::{Level, TrafficLight};
use crate::health_check::TimedValue;
use crate::message::{Message, raw, text};

// The rules, one module per system of the truck.
mod air;
mod electrics;
mod exhaust;
mod faults;
mod temperatures;
#[cfg(test)]
mod tests;

use air::{check_air, check_hours};
use electrics::check_voltage;
use exhaust::{check_def, check_dpf};
use faults::{check_faults, check_lamps};
use temperatures::{check_coolant, check_oil};

/// The SPNs the rules read.
pub mod spn {
    pub const ENGINE_SPEED: u32 = 190;
    pub const ENGINE_LOAD: u32 = 92;
    pub const COOLANT_TEMP: u32 = 110;
    pub const COOLANT_LEVEL: u32 = 111;
    pub const OIL_TEMP: u32 = 175;
    pub const OIL_PRESSURE: u32 = 100;
    pub const BATTERY: u32 = 168;
    pub const CHARGING: u32 = 167;
    pub const DEF_LEVEL: u32 = 1761;
    pub const SOOT_LOAD: u32 = 3719;
    pub const ASH_LOAD: u32 = 3720;
    pub const REGENERATION: u32 = 3700;
    pub const DPF_STATE: u32 = 3701;
    pub const BOOST: u32 = 102;
    pub const INTAKE_TEMP: u32 = 105;
    pub const ENGINE_HOURS: u32 = 247;

    /// Every SPN the health check samples while it listens.
    pub const SAMPLED: [u32; 15] = [
        ENGINE_SPEED,
        ENGINE_LOAD,
        COOLANT_TEMP,
        COOLANT_LEVEL,
        OIL_TEMP,
        OIL_PRESSURE,
        BATTERY,
        CHARGING,
        DEF_LEVEL,
        SOOT_LOAD,
        ASH_LOAD,
        REGENERATION,
        DPF_STATE,
        BOOST,
        INTAKE_TEMP,
    ];

    /// SPNs of the AdBlue (DEF) and SCR system whose faults point to the
    /// fluid, its quality or its dosing: tank level and temperature (1761,
    /// 3031, 3517), quality and concentration (3364, 3516), dosing (3361,
    /// 4334), SCR efficiency (4364), NOx limits exceeded for lack of
    /// reagent or its quality (4094, 4096) and the operator inducement
    /// (5245, 5246) that ends in reduced power.
    pub const DEF_SYSTEM: [u32; 12] = [
        1761, 3031, 3517, 3364, 3516, 3361, 4334, 4364, 4094, 4096, 5245, 5246,
    ];
}

/// Thresholds of the rules. Heavy diesel engines differ by make; these are
/// the common workshop values, and the texts say where makes differ.
pub mod limits {
    /// 1/min above which the engine counts as running.
    pub const RUNNING_RPM: f64 = 400.0;
    /// 1/min below which it is off.
    pub const STOPPED_RPM: f64 = 50.0;
    /// 1/min up to which a running engine counts as idling.
    pub const IDLE_MAX_RPM: f64 = 1000.0;
    /// °C from which the engine counts as warm.
    pub const WARM: f64 = 60.0;
    pub const COOLANT_WARNING: f64 = 100.0;
    pub const COOLANT_CRITICAL: f64 = 105.0;
    /// % in the expansion tank.
    pub const COOLANT_LEVEL_WARNING: f64 = 80.0;
    pub const COOLANT_LEVEL_CRITICAL: f64 = 40.0;
    pub const OIL_TEMP_WARNING: f64 = 115.0;
    pub const OIL_TEMP_CRITICAL: f64 = 130.0;
    /// kPa at warm idle. Many makers give about 70 kPa (10 psi) as the
    /// lowest allowed value at idle.
    pub const OIL_IDLE_CRITICAL: f64 = 70.0;
    pub const OIL_IDLE_WARNING: f64 = 105.0;
    /// Above this the battery reading belongs to a 24 V system.
    pub const TWELVE_VOLT_MAX: f64 = 16.0;
    /// 24 V system, engine running.
    pub const CHARGING_NONE_24: f64 = 25.5;
    pub const CHARGING_LOW_24: f64 = 26.8;
    pub const CHARGING_HIGH_24: f64 = 29.8;
    /// 24 V system, engine off.
    pub const REST_FLAT_24: f64 = 23.6;
    pub const REST_LOW_24: f64 = 24.4;
    /// %: below this the engine soon loses power (operator inducement).
    pub const DEF_CRITICAL: f64 = 5.0;
    pub const DEF_WARNING: f64 = 15.0;
    /// % soot load from which a regeneration is due soon.
    pub const SOOT_WARNING: f64 = 75.0;
    pub const SOOT_CRITICAL: f64 = 100.0;
    /// % ash load from which the filter needs cleaning soon.
    pub const ASH_WARNING: f64 = 80.0;
    /// °C in the intake manifold behind the charge air cooler.
    pub const INTAKE_WARNING: f64 = 75.0;
    /// % engine load from which boost is judged.
    pub const BOOST_LOAD: f64 = 40.0;
    /// kPa boost under load below which the turbo or the charge air path
    /// is suspect.
    pub const BOOST_LOW: f64 = 50.0;
}

/// The four lamps of one unit (from its DM1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TruckLamps {
    pub mil: bool,
    pub red_stop: bool,
    pub amber: bool,
    pub protect: bool,
}

impl TruckLamps {
    pub fn any(self) -> bool {
        self.mil || self.red_stop || self.amber || self.protect
    }
}

/// One fault of a unit.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TruckFault {
    pub spn: u32,
    pub fmi: u8,
    /// Plain meaning in the active language.
    pub text: String,
    /// How often it went from inactive to active, if the unit says.
    pub occurrences: Option<u8>,
}

impl TruckFault {
    pub fn code(&self) -> String {
        format!("SPN {} FMI {}", self.spn, self.fmi)
    }

    fn line(&self) -> Message {
        let base = if self.text.is_empty() {
            raw(self.code())
        } else {
            text("%1 (%2)", "%1 (%2)")
                .plain(self.code())
                .plain(self.text.clone())
        };
        match self.occurrences {
            Some(n) if n > 1 => text("%1, %2-mal", "%1, %2 times")
                .list(vec![base])
                .num(f64::from(n), 0),
            _ => base,
        }
    }
}

/// The faults of one unit: active from DM1, previously active from DM2.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TruckUnit {
    pub address: u8,
    /// "Engine (00)", in the active language.
    pub label: String,
    pub lamps: TruckLamps,
    pub active: Vec<TruckFault>,
    pub previous: Vec<TruckFault>,
}

#[derive(Clone, Debug, Default)]
pub struct TruckHealthInput {
    /// Broadcast values by SPN, in the units of the J1939 table.
    pub values: BTreeMap<u32, Vec<TimedValue>>,
    /// SPNs whose unit reported its own measurement as faulty.
    pub sensor_errors: Vec<u32>,
    /// Every unit heard, with its faults and lamps.
    pub units: Vec<TruckUnit>,
    /// The active faults (DM1) were read; tells "none" from "not read".
    pub faults_read: bool,
    /// Engine hours (SPN 247), on request.
    pub engine_hours: Option<f64>,
}

impl TruckHealthInput {
    pub fn add_value(&mut self, spn: u32, value: f64, t: f64) {
        if value.is_finite() && t.is_finite() {
            self.values
                .entry(spn)
                .or_default()
                .push(TimedValue { t, value });
        }
    }

    pub fn add_sensor_error(&mut self, spn: u32) {
        if !self.sensor_errors.contains(&spn) {
            self.sensor_errors.push(spn);
        }
    }

    fn series(&self, spn: u32) -> Option<&[TimedValue]> {
        self.values
            .get(&spn)
            .filter(|v| !v.is_empty())
            .map(Vec::as_slice)
    }

    fn rpm_at(&self, t: f64) -> Option<f64> {
        let s = self.series(spn::ENGINE_SPEED)?;
        s.iter()
            .min_by(|a, b| {
                (a.t - t)
                    .abs()
                    .partial_cmp(&(b.t - t).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .filter(|v| (v.t - t).abs() <= 3.0)
            .map(|v| v.value)
    }

    fn engine_ran(&self) -> bool {
        self.series(spn::ENGINE_SPEED)
            .is_some_and(|s| s.iter().any(|v| v.value > limits::RUNNING_RPM))
    }
}

/// One finding with its level and reason.
#[derive(Clone, Debug, PartialEq)]
pub struct TruckFinding {
    /// Stable rule id, e.g. "coolant_temp".
    pub id: &'static str,
    pub level: Level,
    pub title: Message,
    pub explanation: Message,
    /// What to do; empty when nothing is to do.
    pub advice: Message,
}

/// A rule that could not run, with the reason.
#[derive(Clone, Debug, PartialEq)]
pub struct TruckSkipped {
    pub id: &'static str,
    pub title: Message,
    pub reason: Message,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TruckHealthReport {
    /// 0..100.
    pub score: i32,
    pub grade: TrafficLight,
    pub summary: Message,
    /// Worst first.
    pub findings: Vec<TruckFinding>,
    pub not_checked: Vec<TruckSkipped>,
    pub checks_run: i32,
    /// 12 or 24, from the battery voltage; 0 when no voltage was heard.
    pub system_volts: i32,
}

impl TruckHealthReport {
    pub fn find(&self, id: &str) -> Option<&TruckFinding> {
        self.findings.iter().find(|f| f.id == id)
    }
}

struct Ctx<'a> {
    input: &'a TruckHealthInput,
    findings: Vec<TruckFinding>,
    skipped: Vec<TruckSkipped>,
    checks_run: i32,
    system_volts: i32,
}

impl Ctx<'_> {
    fn add(
        &mut self,
        id: &'static str,
        level: Level,
        title: Message,
        explanation: Message,
        advice: Message,
    ) {
        self.checks_run += 1;
        self.findings.push(TruckFinding {
            id,
            level,
            title,
            explanation,
            advice,
        });
    }

    fn skip(&mut self, id: &'static str, title: Message, reason: Message) {
        self.skipped.push(TruckSkipped { id, title, reason });
    }

    /// "Not sent" for a value the truck did not broadcast.
    fn missing(&mut self, id: &'static str, title: Message, spn: u32) {
        let reason = if self.input.sensor_errors.contains(&spn) {
            text(
                "Das Steuergerät meldet den Messwert als fehlerhaft (SPN %1).",
                "The control unit reports the reading as faulty (SPN %1).",
            )
        } else {
            text(
                "Der Lkw sendet diesen Wert nicht (SPN %1).",
                "The truck does not send this value (SPN %1).",
            )
        };
        self.skip(id, title, reason.num(f64::from(spn), 0));
    }
}

// --- Score -----------------------------------------------------------------------

const PENALTY_INFO: i32 = 3;
const PENALTY_WARNING: i32 = 12;
const PENALTY_CRITICAL: i32 = 30;

fn score(
    findings: &[TruckFinding],
    skipped: usize,
    checks_run: i32,
) -> (i32, TrafficLight, Message) {
    let mut penalty = 0;
    let mut worst = Level::Good;
    let (mut warnings, mut criticals) = (0, 0);
    for f in findings {
        worst = worst.max(f.level);
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
    // One serious finding must not hide behind many good ones.
    if worst == Level::Critical {
        s = s.min(49);
    } else if worst == Level::Warning {
        s = s.min(84);
    }
    let (grade, mut summary) = match worst {
        Level::Critical => (
            TrafficLight::Red,
            text(
                "Punkte, die bald Aufmerksamkeit brauchen: %1. Bitte zuerst ansehen.",
                "Points that need attention soon: %1. Please look at them first.",
            )
            .num(f64::from(criticals), 0),
        ),
        Level::Warning => (
            TrafficLight::Yellow,
            text(
                "Punkte, die bald angeschaut werden sollten: %1. Nichts davon ist akut gefährlich.",
                "Points that should be looked at soon: %1. None of them is an immediate danger.",
            )
            .num(f64::from(warnings), 0),
        ),
        _ => (
            TrafficLight::Green,
            text(
                "Alles im grünen Bereich. %1 Prüfungen gemacht, nichts, was Sorgen machen müsste.",
                "All good. %1 checks done, nothing to worry about.",
            )
            .num(f64::from(checks_run), 0),
        ),
    };
    if skipped > 0 {
        // One skipped check gets its own sentence, "1 Prüfungen" reads wrong.
        let skipped_text = if skipped == 1 {
            text(
                "Eine Prüfung war mangels Daten nicht möglich.",
                "One check was not possible for lack of data.",
            )
        } else {
            text(
                "%1 Prüfungen waren mangels Daten nicht möglich.",
                "%1 checks were not possible for lack of data.",
            )
            .num(skipped as f64, 0)
        };
        summary = Message::default()
            .list(vec![summary])
            .list(vec![skipped_text]);
    }
    (s, grade, summary)
}

/// Runs every rule and sums up.
pub fn run_truck_health_check(input: &TruckHealthInput) -> TruckHealthReport {
    let mut c = Ctx {
        input,
        findings: Vec::new(),
        skipped: Vec::new(),
        checks_run: 0,
        system_volts: 0,
    };
    if !input.engine_ran() && input.series(spn::ENGINE_SPEED).is_some() {
        c.skip(
            "engine_off",
            text("Motor aus", "Engine off"),
            text(
                "Der Motor lief während der Messung nicht. Für Öldruck und Ladespannung den Motor starten und den Check wiederholen.",
                "The engine did not run during the measurement. For oil pressure and charging voltage start the engine and repeat the check.",
            ),
        );
    }
    check_lamps(&mut c);
    check_faults(&mut c);
    check_coolant(&mut c);
    check_oil(&mut c);
    check_voltage(&mut c);
    check_def(&mut c);
    check_dpf(&mut c);
    check_air(&mut c);
    check_hours(&mut c);
    let mut findings = c.findings;
    // Stable sort, worst first.
    findings.sort_by_key(|f| std::cmp::Reverse(f.level));
    let (score, grade, summary) = score(&findings, c.skipped.len(), c.checks_run);
    TruckHealthReport {
        score,
        grade,
        summary,
        findings,
        not_checked: c.skipped,
        checks_run: c.checks_run,
        system_volts: c.system_volts,
    }
}
