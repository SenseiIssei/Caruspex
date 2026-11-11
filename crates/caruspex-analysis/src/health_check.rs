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

use crate::common::{
    FuelType, Level, Phrase, TrafficLight, format, number, same, signed_number, worse,
};
use crate::inputs::{Fault, LiveValue, MonitorResult, VehicleStatus};
use crate::readiness::{Monitor, MonitorState, monitor_name, readiness_from_status};

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

fn latest(s: &Series) -> f64 {
    let mut best = &s[0];
    for v in s {
        if v.t >= best.t {
            best = v;
        }
    }
    best.value
}

fn max_of(s: &Series) -> f64 {
    let mut m = s[0].value;
    for v in s {
        m = m.max(v.value);
    }
    m
}

fn min_of(s: &Series) -> f64 {
    let mut m = s[0].value;
    for v in s {
        m = m.min(v.value);
    }
    m
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

struct Ctx<'a> {
    input: &'a HealthInput,
    out: HealthReport,
}

impl<'a> Ctx<'a> {
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

// --- Fault codes -----------------------------------------------------------

fn check_faults(c: &mut Ctx) {
    let input = c.input;
    if !input.faults_read {
        c.skip(
            "faults",
            format("Fehlerspeicher", "Fault memory", &[]),
            format(
                "Der Fehlerspeicher wurde nicht gelesen.",
                "The fault memory was not read.",
                &[],
            ),
        );
        return;
    }
    c.ran();
    if input.faults.is_empty() {
        c.add(
            "faults",
            Level::Good,
            Area::Faults,
            format("Keine Fehler gespeichert", "No fault codes stored", &[]),
            format(
                "Das Motorsteuergerät meldet keinen gespeicherten, vorgemerkten oder \
                 dauerhaften Fehler.",
                "The engine control unit reports no stored, pending or permanent fault.",
                &[],
            ),
            none(),
        );
        return;
    }
    for f in &input.faults {
        let level = match &input.fault_level {
            Some(cb) => cb(f),
            None => Level::Warning,
        };
        let kind = f.kind.to_ascii_lowercase();
        let meaning = if kind.contains("dauerhaft") || kind.contains("permanent") {
            format(
                "Dieser Fehler ist dauerhaft gespeichert. Er lässt sich nicht löschen und \
                 verschwindet erst, wenn das Steuergerät selbst festgestellt hat, dass alles \
                 wieder in Ordnung ist.",
                "This fault is stored as permanent. It cannot be cleared and only goes away once \
                 the control unit has confirmed by itself that everything is fine again.",
                &[],
            )
        } else if kind.contains("sporadisch")
            || kind.contains("intermittent")
            || kind.contains("vorgemerkt")
            || kind.contains("pending")
        {
            format(
                "Dieser Fehler ist erst vorgemerkt: Das Steuergerät hat ihn einmal bemerkt, aber \
                 noch nicht bestätigt. Taucht er bei den nächsten Fahrten wieder auf, wird er \
                 fest gespeichert.",
                "This fault is only pending: the control unit noticed it once but has not \
                 confirmed it yet. If it shows up again on the next trips, it gets stored for \
                 good.",
                &[],
            )
        } else {
            format(
                "Dieser Fehler ist fest gespeichert.",
                "This fault is stored.",
                &[],
            )
        };
        let advice = match level {
            Level::Critical => format(
                "Bald in die Werkstatt, und bis dahin möglichst wenig und \
                 schonend fahren.",
                "Have it seen by a garage soon, and until then drive as little \
                 and as gently as possible.",
                &[],
            ),
            Level::Warning => format(
                "Bei Gelegenheit prüfen lassen. In der Fehlerliste stehen \
                 typische Ursachen.",
                "Have it checked when convenient. The fault list shows typical \
                 causes.",
                &[],
            ),
            _ => format(
                "Im Auge behalten. Kommt er nach dem Löschen wieder, genauer \
                 nachsehen.",
                "Keep an eye on it. If it comes back after clearing, look into \
                 it more closely.",
                &[],
            ),
        };
        let title = if f.text.is_empty() {
            f.code.clone()
        } else {
            format!("{} {}", f.code, f.text)
        };
        c.add(
            format!("fault:{}", f.code),
            level,
            Area::Faults,
            same(&title),
            meaning,
            advice,
        );
    }
}

// --- Warning lamp ----------------------------------------------------------

fn check_mil(c: &mut Ctx) {
    let Some(mil_on) = c.input.mil_on else {
        c.skip(
            "mil",
            format("Motorkontrollleuchte", "Check engine lamp", &[]),
            format(
                "Der Zustand der Leuchte (PID 01) wurde nicht gelesen.",
                "The lamp state (PID 01) was not read.",
                &[],
            ),
        );
        return;
    };
    c.ran();
    if mil_on {
        c.add(
            "mil",
            Level::Warning,
            Area::Faults,
            format("Motorkontrollleuchte an", "Check engine lamp on", &[]),
            format(
                "Das Steuergerät hat die gelbe Motorkontrollleuchte eingeschaltet. Es hat \
                 also einen Fehler erkannt, der die Abgaswerte verschlechtern kann. Mit \
                 leuchtender Lampe fällt ein Auto in vielen Ländern durch die \
                 Abgasuntersuchung.",
                "The control unit has switched on the yellow check engine lamp. It has \
                 found a fault that can make the emissions worse. With the lamp on, a car \
                 fails the emissions test in many countries.",
                &[],
            ),
            format(
                "Die Fehlerliste zeigt den Grund. Blinkt die Lampe während der Fahrt, \
                 sofort Gas wegnehmen und bald anhalten: Das bedeutet starke Aussetzer, die \
                 den Katalysator zerstören können.",
                "The fault list shows the reason. If the lamp flashes while driving, ease \
                 off at once and stop soon: that means heavy misfires that can destroy the \
                 catalytic converter.",
                &[],
            ),
        );
    } else {
        c.add(
            "mil",
            Level::Good,
            Area::Faults,
            format("Motorkontrollleuchte aus", "Check engine lamp off", &[]),
            format(
                "Das Steuergerät meldet die Warnlampe als aus.",
                "The control unit reports the warning lamp as off.",
                &[],
            ),
            none(),
        );
    }
}

// --- Readiness -------------------------------------------------------------

fn check_readiness(c: &mut Ctx) {
    let input = c.input;
    if !input.readiness_read {
        c.skip(
            "readiness",
            format("Bereitschaftstests", "Readiness monitors", &[]),
            format(
                "Die Bereitschaft (PID 01) wurde nicht gelesen.",
                "Readiness (PID 01) was not read.",
                &[],
            ),
        );
        return;
    }
    c.ran();
    let mut supported = 0;
    let mut open: Vec<Monitor> = Vec::new();
    let mut diesel_monitors = false;
    for m in &input.readiness {
        if !m.supported {
            continue;
        }
        supported += 1;
        if m.monitor >= Monitor::NmhcCatalyst {
            diesel_monitors = true;
        }
        if !m.complete {
            open.push(m.monitor);
        }
    }
    let diesel = c.diesel() || diesel_monitors;

    if open.is_empty() {
        c.add(
            "readiness",
            Level::Good,
            Area::Readiness,
            format(
                "Alle Bereitschaftstests fertig",
                "All readiness monitors complete",
                &[],
            ),
            format(
                "{} von {} Selbsttests sind abgeschlossen. Das ist die beste Voraussetzung \
                 für die Abgasuntersuchung.",
                "{} of {} self-tests are complete. That is the best starting point for \
                 the emissions test.",
                &[
                    number(f64::from(supported), 0),
                    number(f64::from(supported), 0),
                ],
            ),
            none(),
        );
        return;
    }

    let mut list = Phrase::default();
    for (i, m) in open.iter().enumerate() {
        let name = monitor_name(*m);
        if i > 0 {
            list.de.push_str(", ");
            list.en.push_str(", ");
        }
        list.de.push_str(name.de);
        list.en.push_str(name.en);
    }
    let level = if diesel {
        Level::Warning
    } else if open.len() == 1 {
        Level::Info
    } else {
        Level::Warning
    };
    let title = if open.len() == 1 {
        format(
            "1 Bereitschaftstest noch offen",
            "1 readiness monitor not complete",
            &[],
        )
    } else {
        format(
            "{} Bereitschaftstests noch offen",
            "{} readiness monitors not complete",
            &[number(open.len() as f64, 0)],
        )
    };
    let explanation = format(
        "Noch nicht fertig: {}. Das Auto hat diese Selbsttests seit dem letzten Löschen des \
         Fehlerspeichers (oder seit die Batterie abgeklemmt war) noch nicht zu Ende gebracht. \
         Das ist an sich kein Defekt, solange keine Fehler gespeichert sind.",
        "Not complete yet: {}. The car has not finished these self-tests since the fault \
         memory was last cleared (or since the battery was disconnected). That alone is not a \
         defect, as long as no faults are stored.",
        &[list],
    );
    let rule = if diesel {
        format(
            "Für die Abgasuntersuchung sollten beim Diesel möglichst alle Tests fertig \
             sein; die genauen Regeln hängen vom Land und der Prüfstelle ab.",
            "For the emissions test, a diesel should have all monitors complete if \
             possible; the exact rules depend on the country and the testing station.",
            &[],
        )
    } else {
        format(
            "Als Faustregel für die Abgasuntersuchung: Beim Benziner wird oft ein offener \
             Test hingenommen (so regelt es zum Beispiel die US-Umweltbehörde EPA für \
             Autos ab Baujahr 2001), mehrere offene Tests meist nicht. Die genauen Regeln \
             hängen vom Land und der Prüfstelle ab.",
            "As a rule of thumb for the emissions test: a petrol car is often accepted \
             with one incomplete monitor (that is the US EPA rule for model year 2001 and \
             later), with several usually not. The exact rules depend on the country and \
             the testing station.",
            &[],
        )
    };
    let advice = rule
        + format(
            "Am sichersten ist es, vorher die passenden Fahrten zu machen (siehe Fahrzyklus-Hilfe) \
             und danach erneut zu prüfen.",
            "The safest way is to do the matching drives first (see the drive cycle help) and \
             check again afterwards.",
            &[],
        );
    c.add(
        "readiness",
        level,
        Area::Readiness,
        title,
        explanation,
        advice,
    );
}

// --- Clearing history ------------------------------------------------------

fn check_history(c: &mut Ctx) {
    let input = c.input;
    let warmups = get(input, PID_WARMUPS);
    let distance = get(input, PID_CLEAR_DISTANCE);
    if warmups.is_none() && distance.is_none() {
        c.skip(
            "cleared_recently",
            format("Zuletzt gelöscht", "Last cleared", &[]),
            format(
                "Warmläufe und Strecke seit dem Löschen (PID 30 und 31) fehlen.",
                "Warm-ups and distance since clearing (PID 30 and 31) are missing.",
                &[],
            ),
        );
    } else {
        c.ran();
        let w = warmups.map(latest);
        let d = distance.map(latest);
        let recent = w.is_some_and(|w| w < f64::from(limits::FEW_WARMUPS))
            || d.is_some_and(|d| d < limits::SHORT_DISTANCE_KM);

        let facts = match (w, d) {
            (Some(w), Some(d)) => format(
                "Seit dem letzten Löschen: {} Warmläufe und {} km.",
                "Since the last clearing: {} warm-ups and {} km.",
                &[number(w, 0), number(d, 0)],
            ),
            (Some(w), None) => format(
                "Seit dem letzten Löschen: {} Warmläufe.",
                "Since the last clearing: {} warm-ups.",
                &[number(w, 0)],
            ),
            (None, d) => format(
                "Seit dem letzten Löschen: {} km.",
                "Since the last clearing: {} km.",
                &[number(d.unwrap_or(0.0), 0)],
            ),
        };

        if recent {
            let mut explanation = facts
                + format(
                    "Das ist sehr wenig. Mit dem Löschen werden auch die Bereitschaftstests \
                     zurückgesetzt, und Fehler, die erst nach einiger Zeit wiederkommen, sind noch \
                     nicht zu sehen. Beim Gebrauchtwagenkauf kann das ein Hinweis sein, dass \
                     jemand Fehler kurz vor der Besichtigung gelöscht hat. Es gibt aber auch \
                     harmlose Gründe, etwa eine Reparatur oder eine abgeklemmte Batterie.",
                    "That is very little. Clearing also resets the readiness monitors, and faults \
                     that only come back after a while are not visible yet. When buying a used \
                     car, this can be a hint that someone cleared faults shortly before the \
                     viewing. There are harmless reasons too, such as a repair or a disconnected \
                     battery.",
                    &[],
                );
            if c.has_incomplete_readiness() {
                explanation = explanation
                    + format(
                        "Dazu passen die noch offenen Bereitschaftstests.",
                        "The incomplete readiness monitors fit that picture.",
                        &[],
                    );
            }
            c.add(
                "cleared_recently",
                Level::Warning,
                Area::History,
                format(
                    "Fehlerspeicher vor Kurzem gelöscht",
                    "Fault memory cleared recently",
                    &[],
                ),
                explanation,
                format(
                    "Beim Kauf nachfragen, warum gelöscht wurde. Am besten nach einer \
                     Probefahrt von 20 bis 30 Minuten oder nach ein paar Tagen Fahrt \
                     noch einmal prüfen.",
                    "When buying, ask why it was cleared. Ideally check again after a test \
                     drive of 20 to 30 minutes or after a few days of driving.",
                    &[],
                ),
            );
        } else {
            c.add(
                "cleared_recently",
                Level::Good,
                Area::History,
                format("Nicht frisch gelöscht", "Not freshly cleared", &[]),
                facts
                    + format(
                        "Kein Hinweis auf ein Löschen kurz vor der Prüfung.",
                        "No sign of clearing shortly before the check.",
                        &[],
                    ),
                none(),
            );
        }
    }

    if let Some(mil) = get(input, PID_MIL_DISTANCE) {
        c.ran();
        let km = latest(mil);
        if km > 0.0 {
            c.add(
                "mil_distance",
                Level::Info,
                Area::History,
                format(
                    "Warnlampe war {} km lang an",
                    "Warning lamp was on for {} km",
                    &[number(km, 0)],
                ),
                format(
                    "Seit dem letzten Löschen ist das Auto {} km mit leuchtender \
                     Motorkontrollleuchte gefahren.",
                    "Since the last clearing the car has been driven {} km with the check \
                     engine lamp on.",
                    &[number(km, 0)],
                ),
                format(
                    "Je länger ein Fehler ignoriert wird, desto eher entstehen Folgeschäden. \
                     Den Grund in der Fehlerliste nachsehen.",
                    "The longer a fault is ignored, the more likely it causes further \
                     damage. Look up the reason in the fault list.",
                    &[],
                ),
            );
        }
    }
}

// --- Fuel trims ------------------------------------------------------------

fn check_trims(c: &mut Ctx, bank: i32) {
    let input = c.input;
    let st_pid = if bank == 1 { 0x06 } else { 0x08 };
    let lt_pid = if bank == 1 { 0x07 } else { 0x09 };
    let st = get(input, st_pid);
    let lt = get(input, lt_pid);
    let id = if bank == 1 {
        "fuel_trim_bank1"
    } else {
        "fuel_trim_bank2"
    };
    let bank_name = format("Bank {}", "bank {}", &[number(f64::from(bank), 0)]);
    let check_title = format(
        "Gemischregelung ({})",
        "Mixture control ({})",
        std::slice::from_ref(&bank_name),
    );

    if st.is_none() && lt.is_none() {
        // Bank 2 only exists on V and boxer engines; its absence is normal.
        if bank == 1 {
            let reason = if c.diesel() {
                format(
                    "Diesel melden meist keine Gemischkorrektur (PID 06 bis 09).",
                    "Diesels usually report no fuel trims (PID 06 to 09).",
                    &[],
                )
            } else {
                format(
                    "Die Gemischkorrektur (PID 06 und 07) wurde nicht gelesen.",
                    "The fuel trims (PID 06 and 07) were not read.",
                    &[],
                )
            };
            c.skip(id, check_title, reason);
        }
        return;
    }

    let base = st.or(lt).unwrap_or(&[]);
    let other = if st.is_some() { lt } else { None };
    let rpm = get(input, PID_RPM);
    let mut all: Vec<f64> = Vec::new();
    let mut idle: Vec<f64> = Vec::new();
    let mut cruise: Vec<f64> = Vec::new();
    for s in base {
        if !c.closed_loop_warm(s.t) {
            continue;
        }
        let mut total = s.value;
        if let Some(other) = other {
            match nearest(Some(other), s.t) {
                Some(o) => total += o,
                None => continue,
            }
        }
        all.push(total);
        if let Some(r) = nearest(rpm, s.t) {
            if r < 1100.0 {
                idle.push(total);
            } else if r >= 1800.0 {
                cruise.push(total);
            }
        }
    }
    if all.is_empty() {
        c.skip(
            id,
            check_title,
            format(
                "Keine Werte bei warmem Motor im geschlossenen Regelkreis. Die Korrektur \
                 sagt erst etwas aus, wenn der Motor warm ist und die Lambdaregelung \
                 arbeitet.",
                "No readings with a warm engine in closed loop. The trims only mean \
                 something once the engine is warm and the oxygen control is active.",
                &[],
            ),
        );
        return;
    }
    c.ran();
    let avg = mean(&all);
    let magnitude = avg.abs();
    let value = signed_number(avg, 1);
    let partial = if st.is_some() && lt.is_some() {
        Phrase::default()
    } else {
        format(
            "(Nur ein Teil der Korrektur wurde gelesen, das Ergebnis ist deshalb ungenauer.)",
            "(Only part of the correction was read, so the result is less precise.)",
            &[],
        )
    };

    if magnitude < limits::TRIM_WARN {
        c.add(
            id,
            Level::Good,
            Area::Mixture,
            format(
                "Gemischregelung in Ordnung ({})",
                "Mixture control fine ({})",
                &[bank_name],
            ),
            format(
                "Die Korrektur liegt im Schnitt bei {} %, also im normalen Bereich von \
                 plus/minus 10 %.",
                "The correction averages {}%, well within the normal range of plus/minus \
                 10%.",
                &[value],
            ) + partial,
            none(),
        );
        return;
    }
    let level = if magnitude >= limits::TRIM_BAD {
        Level::Critical
    } else {
        Level::Warning
    };

    // Comparing idle and higher engine speed narrows the cause down: an air
    // leak matters most at idle, where little air flows anyway.
    let mut pattern = Phrase::default();
    if avg > 0.0 && idle.len() >= 3 && cruise.len() >= 3 {
        let at_idle = mean(&idle);
        let at_cruise = mean(&cruise);
        if at_idle >= limits::TRIM_WARN && at_cruise < 5.0 {
            pattern = format(
                "Im Leerlauf ist die Korrektur hoch ({} %), bei höherer Drehzahl \
                 fast normal ({} %). Das spricht stark für Falschluft.",
                "At idle the correction is high ({}%), at higher engine speed \
                 almost normal ({}%). That points strongly to an air leak.",
                &[signed_number(at_idle, 1), signed_number(at_cruise, 1)],
            );
        } else if at_idle >= limits::TRIM_WARN && at_cruise >= limits::TRIM_WARN {
            pattern = format(
                "Die Korrektur ist im Leerlauf und bei höherer Drehzahl hoch. Das \
                 spricht eher für den Luftmassenmesser oder den Kraftstoffdruck \
                 als für Falschluft.",
                "The correction is high both at idle and at higher engine speed. \
                 That points more to the mass air flow sensor or the fuel \
                 pressure than to an air leak.",
                &[],
            );
        }
    }

    if avg > 0.0 {
        c.add(
            id,
            level,
            Area::Mixture,
            format(
                "Gemisch zu mager ({})",
                "Mixture too lean ({})",
                &[bank_name],
            ),
            format(
                "Das Steuergerät gibt im Schnitt {} % mehr Kraftstoff dazu, als es \
                 berechnet hat. Es kommt also mehr Luft in den Motor, als gemessen wird, \
                 oder zu wenig Kraftstoff an. Häufige Gründe: Falschluft durch einen \
                 undichten Schlauch oder eine Dichtung hinter dem Luftmassenmesser, ein \
                 verschmutzter Luftmassenmesser oder zu wenig Kraftstoffdruck.",
                "The control unit adds on average {}% more fuel than it calculated. So \
                 either more air gets into the engine than is measured, or too little \
                 fuel arrives. Common reasons: an air leak through a cracked hose or \
                 gasket after the mass air flow sensor, a dirty mass air flow sensor or \
                 low fuel pressure.",
                &[value],
            ) + pattern
                + partial,
            format(
                "Im Leerlauf auf Zischen achten und Unterdruckschläuche ansehen. Eine \
                 Werkstatt findet Falschluft schnell mit einem Rauchtest.",
                "Listen for hissing at idle and look at the vacuum hoses. A garage finds \
                 air leaks quickly with a smoke test.",
                &[],
            ),
        );
    } else {
        c.add(
            id,
            level,
            Area::Mixture,
            format(
                "Gemisch zu fett ({})",
                "Mixture too rich ({})",
                &[bank_name],
            ),
            format(
                "Das Steuergerät nimmt im Schnitt {} % Kraftstoff weg. Es kommt also zu \
                 viel Kraftstoff oder zu wenig Luft an. Typische Gründe: ein undichtes \
                 Einspritzventil, zu hoher Kraftstoffdruck, ein hängendes \
                 Tankentlüftungsventil, ein falsch messender Luftmassenmesser oder \
                 Kraftstoff im Motoröl.",
                "The control unit takes away on average {}% fuel. So too much fuel or \
                 too little air arrives. Typical reasons: a leaking injector, too high \
                 fuel pressure, a stuck purge valve, a mass air flow sensor that reads \
                 wrong, or fuel in the engine oil.",
                &[number(magnitude, 1)],
            ) + partial,
            format(
                "Bald prüfen lassen, bevor der Katalysator Schaden nimmt. Ölstand ansehen: \
                 Riecht das Öl nach Kraftstoff oder steigt der Stand, ist Kraftstoff im Öl.",
                "Have it checked soon, before the catalytic converter suffers. Look at the \
                 oil: if it smells of fuel or the level rises, there is fuel in the oil.",
                &[],
            ),
        );
    }
}

// --- Battery and charging --------------------------------------------------

fn check_voltage(c: &mut Ctx) {
    let input = c.input;
    let charge_title = format("Ladespannung", "Charging voltage", &[]);
    let rest_title = format(
        "Batterie bei stehendem Motor",
        "Battery with engine off",
        &[],
    );
    let Some(volts) = get(input, PID_VOLTAGE) else {
        let why = format(
            "Die Spannung (PID 42) wurde nicht gelesen.",
            "The voltage (PID 42) was not read.",
            &[],
        );
        c.skip("charging", charge_title, why.clone());
        c.skip("battery_rest", rest_title, why);
        return;
    };
    let Some(rpm) = get(input, PID_RPM) else {
        let why = format(
            "Ohne Drehzahl ist nicht klar, ob der Motor lief.",
            "Without engine speed it is unclear whether the engine ran.",
            &[],
        );
        c.skip("charging", charge_title, why.clone());
        c.skip("battery_rest", rest_title, why);
        return;
    };
    let mut running: Vec<f64> = Vec::new();
    let mut off: Vec<f64> = Vec::new();
    for v in volts {
        let Some(r) = nearest(Some(rpm), v.t) else {
            continue;
        };
        if r > RUNNING_RPM {
            running.push(v.value);
        } else if r < STOPPED_RPM {
            off.push(v.value);
        }
    }

    let source_note = format(
        "Gemessen am Diagnosestecker; dort liegt die Spannung oft 0,1 bis 0,3 V unter der an \
         den Batteriepolen.",
        "Measured at the diagnostic socket, where the voltage is often 0.1 to 0.3 V below the \
         battery terminals.",
        &[],
    );

    if running.is_empty() {
        c.skip(
            "charging",
            charge_title,
            format(
                "Der Motor lief während der Messung nicht.",
                "The engine was not running during the measurement.",
                &[],
            ),
        );
    } else {
        c.ran();
        let v = median(running);
        let value = number(v, 2);
        let smart = format(
            "Autos mit Start-Stopp und intelligentem Lademanagement (oft mit AGM- oder \
             EFB-Batterie) senken die Ladespannung bei voller Batterie bewusst ab; dort kann \
             ein Wert um 12,5 bis 13 V normal sein.",
            "Cars with start-stop and smart charging (often with an AGM or EFB battery) \
             lower the charging voltage on purpose when the battery is full; there a value of \
             about 12.5 to 13 V can be normal.",
            &[],
        );
        if v < limits::CHARGING_NONE {
            c.add(
                "charging",
                Level::Critical,
                Area::Charging,
                format(
                    "Lichtmaschine lädt nicht",
                    "Alternator is not charging",
                    &[],
                ),
                format(
                    "Bei laufendem Motor liegen nur {} V an. Normal sind 13,8 bis 14,8 V. \
                     Die Batterie wird also nicht geladen, sondern leer gefahren.",
                    "With the engine running there are only {} V. Normal is 13.8 to \
                     14.8 V. So the battery is not being charged but drained.",
                    &[value],
                ) + smart
                    + source_note.clone(),
                format(
                    "Bald prüfen lassen: Lichtmaschine, Keilrippenriemen und Kabel. Bis \
                     dahin kann das Auto unterwegs liegen bleiben.",
                    "Have it checked soon: alternator, drive belt and cables. Until then the \
                     car may stop on the road.",
                    &[],
                ),
            );
        } else if v < limits::CHARGING_LOW {
            c.add(
                "charging",
                Level::Warning,
                Area::Charging,
                format("Ladespannung niedrig", "Charging voltage low", &[]),
                format(
                    "Bei laufendem Motor liegen {} V an. Normal sind 13,8 bis 14,8 V; \
                     unter 13,2 V lädt die Lichtmaschine meist zu schwach.",
                    "With the engine running there are {} V. Normal is 13.8 to 14.8 V; \
                     below 13.2 V the alternator usually charges too weakly.",
                    &[value],
                ) + smart
                    + source_note.clone(),
                format(
                    "Mit dem geführten Batterietest nachmessen, auch mit eingeschaltetem \
                     Licht und Gebläse.",
                    "Measure again with the guided battery test, also with the lights and \
                     blower switched on.",
                    &[],
                ),
            );
        } else if v > limits::CHARGING_TOO_HIGH {
            c.add(
                "charging",
                Level::Critical,
                Area::Charging,
                format(
                    "Ladespannung viel zu hoch",
                    "Charging voltage far too high",
                    &[],
                ),
                format(
                    "Bei laufendem Motor liegen {} V an. Über 15,5 V wird die Batterie \
                     überladen und Elektronik kann Schaden nehmen. Meist ist der Regler der \
                     Lichtmaschine defekt.",
                    "With the engine running there are {} V. Above 15.5 V the battery is \
                     overcharged and electronics can be damaged. Usually the alternator's \
                     regulator has failed.",
                    &[value],
                ),
                format(
                    "Bald in die Werkstatt.",
                    "Have it seen by a garage soon.",
                    &[],
                ),
            );
        } else if v > limits::CHARGING_HIGH {
            c.add(
                "charging",
                Level::Warning,
                Area::Charging,
                format("Ladespannung hoch", "Charging voltage high", &[]),
                format(
                    "Bei laufendem Motor liegen {} V an. Normal sind 13,8 bis 14,8 V. \
                     AGM-Batterien werden zeitweise mit bis zu etwa 15 V geladen, \
                     dauerhaft mehr deutet auf einen defekten Regler hin.",
                    "With the engine running there are {} V. Normal is 13.8 to 14.8 V. \
                     AGM batteries are sometimes charged at up to about 15 V; more than \
                     that all the time points to a faulty regulator.",
                    &[value],
                ),
                format(
                    "Mit dem geführten Batterietest nachmessen.",
                    "Measure again with the guided battery test.",
                    &[],
                ),
            );
        } else {
            c.add(
                "charging",
                Level::Good,
                Area::Charging,
                format("Lichtmaschine lädt", "Alternator is charging", &[]),
                format(
                    "Bei laufendem Motor liegen {} V an, das ist im normalen Bereich.",
                    "With the engine running there are {} V, which is normal.",
                    &[value],
                ),
                none(),
            );
        }
    }

    if off.is_empty() {
        c.skip(
            "battery_rest",
            rest_title,
            format(
                "Der Motor lief die ganze Zeit. Die Ruhespannung misst der geführte \
                 Batterietest.",
                "The engine ran the whole time. The guided battery test measures the \
                 resting voltage.",
                &[],
            ),
        );
    } else {
        c.ran();
        let v = median(off);
        let value = number(v, 2);
        let load_note = format(
            "Bei eingeschalteter Zündung ziehen Steuergeräte und Pumpen Strom, der Wert liegt \
             deshalb etwas unter der echten Ruhespannung.",
            "With the ignition on, control units and pumps draw current, so the value is a \
             little below the true resting voltage.",
            &[],
        );
        if v < limits::BATTERY_FLAT {
            c.add(
                "battery_rest",
                Level::Critical,
                Area::Charging,
                format("Batterie fast leer", "Battery almost flat", &[]),
                format(
                    "Bei stehendem Motor liegen nur {} V an. Damit springt das Auto \
                     womöglich bald nicht mehr an.",
                    "With the engine off there are only {} V. The car may soon fail to \
                     start.",
                    &[value],
                ) + load_note
                    + source_note.clone(),
                format(
                    "Batterie laden und danach mit dem geführten Batterietest prüfen. \
                     Ist sie bald wieder leer, ist sie verbraucht oder etwas zieht Strom \
                     im Stand.",
                    "Charge the battery and test it with the guided battery test. If it \
                     goes flat again soon, it is worn out or something drains it while \
                     parked.",
                    &[],
                ),
            );
        } else if v < limits::BATTERY_WEAK {
            c.add(
                "battery_rest",
                Level::Warning,
                Area::Charging,
                format("Batterie schwach", "Battery weak", &[]),
                format(
                    "Bei stehendem Motor liegen {} V an. Eine volle Batterie hat etwa \
                     12,6 V, unter 12,2 V ist sie nur noch etwa halb geladen oder \
                     gealtert.",
                    "With the engine off there are {} V. A full battery has about 12.6 V; \
                     below 12.2 V it is only about half charged or aged.",
                    &[value],
                ) + load_note
                    + source_note.clone(),
                format(
                    "Eine längere Fahrt machen oder die Batterie laden und dann mit dem \
                     geführten Batterietest nachmessen.",
                    "Take a longer drive or charge the battery, then measure again with \
                     the guided battery test.",
                    &[],
                ),
            );
        } else {
            c.add(
                "battery_rest",
                Level::Good,
                Area::Charging,
                format("Batteriespannung in Ordnung", "Battery voltage fine", &[]),
                format(
                    "Bei stehendem Motor liegen {} V an.",
                    "With the engine off there are {} V.",
                    &[value],
                ) + load_note,
                none(),
            );
        }
    }
}

// --- Temperatures ----------------------------------------------------------

/// How long the engine has been running, in seconds, if it can be told.
fn engine_run_time(input: &HealthInput) -> Option<f64> {
    if let Some(rt) = get(input, PID_RUN_TIME) {
        return Some(latest(rt));
    }
    if let Some(rpm) = get(input, PID_RPM) {
        let mut first = 0.0;
        let mut last = 0.0;
        let mut any = false;
        for v in rpm {
            if v.value <= RUNNING_RPM {
                continue;
            }
            if !any {
                first = v.t;
                last = v.t;
                any = true;
            }
            first = f64::min(first, v.t);
            last = f64::max(last, v.t);
        }
        if any {
            return Some(last - first);
        }
    }
    None
}

fn check_cooling(c: &mut Ctx) {
    let input = c.input;
    let thermo_title = format(
        "Thermostat und Betriebstemperatur",
        "Thermostat and operating temperature",
        &[],
    );
    let Some(coolant) = get(input, PID_COOLANT) else {
        c.skip(
            "thermostat",
            thermo_title,
            format(
                "Die Kühlmitteltemperatur (PID 05) wurde nicht gelesen.",
                "The coolant temperature (PID 05) was not read.",
                &[],
            ),
        );
        return;
    };
    c.ran();
    if min_of(coolant) <= -39.5 {
        // -40 C is the lowest value the PID can carry; sensors with an open
        // circuit report exactly that.
        c.add(
            "sensor_coolant",
            Level::Warning,
            Area::Sensors,
            format(
                "Kühlmittelfühler meldet -40 °C",
                "Coolant sensor reports -40 °C",
                &[],
            ),
            format(
                "-40 °C ist der kleinste Wert, den das Steuergerät melden kann. Das passiert \
                 fast immer, wenn der Fühler oder sein Kabel unterbrochen ist. Dann läuft \
                 der Motor mit einem Ersatzwert, oft zu fett, und der Lüfter springt \
                 dauernd an.",
                "-40 °C is the lowest value the control unit can report. That almost always \
                 happens when the sensor or its wire is broken. The engine then runs on a \
                 substitute value, often too rich, and the fan may run all the time.",
                &[],
            ),
            format(
                "Stecker und Kabel des Kühlmittelfühlers prüfen lassen.",
                "Have the coolant sensor's plug and wire checked.",
                &[],
            ),
        );
        return;
    }

    let hottest = max_of(coolant);
    if hottest >= limits::COOLANT_OVERHEAT {
        c.add(
            "overheating",
            Level::Critical,
            Area::Cooling,
            format("Motor zu heiß", "Engine overheating", &[]),
            format(
                "Das Kühlmittel hat {} °C erreicht. Ab etwa 115 °C droht Schaden an \
                 Zylinderkopf und Dichtung.",
                "The coolant reached {} °C. From about 115 °C the cylinder head and \
                 gasket are at risk.",
                &[number(hottest, 0)],
            ),
            format(
                "Anhalten, Motor abkühlen lassen und Kühlmittelstand prüfen (erst wenn er \
                 kalt ist, den Deckel öffnen). Nicht weiterfahren, bis die Ursache klar ist.",
                "Stop, let the engine cool down and check the coolant level (only open the \
                 cap once it is cold). Do not drive on until the cause is clear.",
                &[],
            ),
        );
    } else if hottest >= limits::COOLANT_HOT {
        c.add(
            "overheating",
            Level::Warning,
            Area::Cooling,
            format("Kühlmittel sehr warm", "Coolant very hot", &[]),
            format(
                "Das Kühlmittel hat {} °C erreicht. Viele Motoren laufen unter Last bis \
                 etwa 105 °C, darüber wird es knapp.",
                "The coolant reached {} °C. Many engines run up to about 105 °C under load; \
                 above that it gets tight.",
                &[number(hottest, 0)],
            ),
            format(
                "Kühlmittelstand prüfen und darauf achten, ob der Lüfter anspringt.",
                "Check the coolant level and watch whether the radiator fan comes on.",
                &[],
            ),
        );
    }

    let run_time = engine_run_time(input);
    let hottest_text = number(hottest, 0);
    if let Some(rt) = run_time.filter(|rt| *rt >= limits::LONG_RUN_SECONDS) {
        let minutes = number(rt / 60.0, 0);
        if hottest < limits::COLD_THERMOSTAT {
            c.add(
                "thermostat",
                Level::Warning,
                Area::Cooling,
                format("Motor wird nicht warm", "Engine does not warm up", &[]),
                format(
                    "Nach {} Minuten Laufzeit hat das Kühlmittel nur {} °C erreicht. \
                     Normal sind 80 bis 105 °C. Meist hängt das Thermostat offen: Der \
                     Motor wird nicht richtig warm, verbraucht mehr und die Heizung bleibt \
                     lau. Seltener misst der Temperaturfühler falsch. Bei Frost im Leerlauf \
                     oder bei sparsamen Dieseln kann es allerdings deutlich länger dauern.",
                    "After {} minutes of running the coolant only reached {} °C. Normal is \
                     80 to 105 °C. Usually the thermostat is stuck open: the engine never \
                     gets properly warm, uses more fuel and the heater stays lukewarm. Less \
                     often the temperature sensor reads wrong. In freezing weather at idle, \
                     or on frugal diesels, it can take much longer though.",
                    &[minutes, hottest_text],
                ),
                format(
                    "Thermostat und Kühlmittelfühler prüfen lassen. Kein Notfall, aber auf \
                     Dauer schlecht für Motor und Verbrauch.",
                    "Have the thermostat and coolant sensor checked. Not an emergency, but \
                     bad for the engine and fuel use in the long run.",
                    &[],
                ),
            );
        } else if hottest < limits::WARM_COOLANT {
            c.add(
                "thermostat",
                Level::Info,
                Area::Cooling,
                format("Motor bleibt eher kühl", "Engine stays rather cool", &[]),
                format(
                    "Nach {} Minuten Laufzeit hat das Kühlmittel {} °C erreicht. Normal \
                     sind 80 bis 105 °C. Bei kaltem Wetter und ruhiger Fahrt kann das \
                     noch in Ordnung sein.",
                    "After {} minutes of running the coolant reached {} °C. Normal is 80 \
                     to 105 °C. In cold weather and with gentle driving this can still be \
                     fine.",
                    &[minutes, hottest_text],
                ),
                format(
                    "Bei der nächsten längeren Fahrt noch einmal ansehen.",
                    "Look at it again on the next longer drive.",
                    &[],
                ),
            );
        } else {
            c.add(
                "thermostat",
                Level::Good,
                Area::Cooling,
                format(
                    "Betriebstemperatur erreicht",
                    "Operating temperature reached",
                    &[],
                ),
                format(
                    "Das Kühlmittel hat {} °C erreicht. Das Thermostat arbeitet.",
                    "The coolant reached {} °C. The thermostat works.",
                    &[hottest_text],
                ),
                none(),
            );
        }
    } else if hottest >= limits::WARM_COOLANT {
        c.add(
            "thermostat",
            Level::Good,
            Area::Cooling,
            format(
                "Betriebstemperatur erreicht",
                "Operating temperature reached",
                &[],
            ),
            format(
                "Das Kühlmittel hat {} °C erreicht. Das Thermostat arbeitet.",
                "The coolant reached {} °C. The thermostat works.",
                &[hottest_text],
            ),
            none(),
        );
    } else {
        let reason = match run_time {
            Some(rt) => format(
                "Der Motor lief erst {} Minuten. Für eine Aussage braucht es etwa 15 \
                 Minuten Fahrt.",
                "The engine had only run for {} minutes. A verdict needs about 15 \
                 minutes of driving.",
                &[number(rt / 60.0, 0)],
            ),
            None => format(
                "Die Laufzeit des Motors ist unbekannt (PID 1F oder Drehzahl fehlt), \
                 und der Motor war noch nicht warm.",
                "The engine run time is unknown (PID 1F or engine speed missing), \
                 and the engine was not warm yet.",
                &[],
            ),
        };
        c.skip("thermostat", thermo_title, reason);
    }
}

fn check_other_temperatures(c: &mut Ctx) {
    let input = c.input;
    if let Some(iat) = get(input, PID_INTAKE_TEMP) {
        c.ran();
        if min_of(iat) <= -39.5 {
            c.add(
                "sensor_intake",
                Level::Warning,
                Area::Sensors,
                format(
                    "Ansauglufttemperatur meldet -40 °C",
                    "Intake air temperature reads -40 °C",
                    &[],
                ),
                format(
                    "-40 °C ist der kleinste Wert, den das Steuergerät melden kann. Fast \
                     immer ist dann der Fühler oder sein Kabel unterbrochen. Das Gemisch \
                     stimmt dadurch nicht mehr genau.",
                    "-40 °C is the lowest value the control unit can report. Almost always \
                     the sensor or its wire is broken then. The mixture is no longer \
                     quite right as a result.",
                    &[],
                ),
                format(
                    "Stecker und Kabel des Fühlers prüfen lassen (er sitzt oft im \
                     Luftmassenmesser).",
                    "Have the sensor's plug and wire checked (it often sits in the mass \
                     air flow sensor).",
                    &[],
                ),
            );
        }
    }
    if let Some(oil) = get(input, PID_OIL_TEMP) {
        c.ran();
        let hottest = max_of(oil);
        if hottest >= limits::OIL_HOT {
            let level = if hottest >= limits::OIL_OVERHEAT {
                Level::Critical
            } else {
                Level::Warning
            };
            c.add(
                "oil_temp",
                level,
                Area::Cooling,
                format("Öl sehr heiß", "Oil very hot", &[]),
                format(
                    "Das Motoröl hat {} °C erreicht. Normal sind etwa 90 bis 120 °C, bei \
                     sportlicher Fahrt auch kurz mehr. Ab etwa 150 °C altert das Öl sehr \
                     schnell und schmiert schlechter.",
                    "The engine oil reached {} °C. Normal is about 90 to 120 °C, briefly \
                     more when driven hard. From about 150 °C the oil ages very quickly and \
                     lubricates less well.",
                    &[number(hottest, 0)],
                ),
                format(
                    "Ölstand prüfen und den Motor schonen. Bleibt es so, Kühlung prüfen \
                     lassen.",
                    "Check the oil level and go easy on the engine. If it stays like this, \
                     have the cooling checked.",
                    &[],
                ),
            );
        }
    }
    if let Some(cat) = get(input, PID_CATALYST_TEMP) {
        c.ran();
        let hottest = max_of(cat);
        if hottest >= limits::CATALYST_HOT {
            c.add(
                "catalyst_temp",
                Level::Warning,
                Area::Catalyst,
                format("Katalysator sehr heiß", "Catalytic converter very hot", &[]),
                format(
                    "Das Steuergerät meldet {} °C am Katalysator. Ab etwa 900 °C kann er \
                     auf Dauer Schaden nehmen. Häufigster Grund sind Zündaussetzer, bei \
                     denen unverbrannter Kraftstoff im Katalysator nachbrennt.",
                    "The control unit reports {} °C at the catalytic converter. From about \
                     900 °C it can be damaged over time. The most common reason is \
                     misfiring, where unburnt fuel burns in the converter.",
                    &[number(hottest, 0)],
                ),
                format(
                    "Auf Aussetzer und Fehlercodes achten und hohe Last vermeiden, bis die \
                     Ursache klar ist.",
                    "Watch for misfires and fault codes and avoid heavy load until the cause \
                     is clear.",
                    &[],
                ),
            );
        }
    }
}

// --- Oxygen sensors --------------------------------------------------------

#[derive(Default)]
struct Switching {
    samples: i32,
    duration: f64,
    switches: i32,
    low: f64,
    high: f64,
}

/// Counts rich/lean changes of a narrowband sensor. A hysteresis around the
/// 0.45 V midpoint keeps noise from counting as switching.
fn analyse_switching(c: &Ctx, s: &Series) -> Switching {
    let mut out = Switching::default();
    let mut state = 0; // -1 lean, +1 rich, 0 unknown
    let mut first = 0.0;
    let mut last = 0.0;
    for v in s {
        if !c.closed_loop_warm(v.t) {
            continue;
        }
        if out.samples == 0 {
            first = v.t;
            last = v.t;
            out.low = v.value;
            out.high = v.value;
        }
        out.samples += 1;
        first = f64::min(first, v.t);
        last = f64::max(last, v.t);
        out.low = out.low.min(v.value);
        out.high = out.high.max(v.value);
        let mut now = 0;
        if v.value < 0.3 {
            now = -1;
        } else if v.value > 0.6 {
            now = 1;
        }
        if now != 0 {
            if state != 0 && now != state {
                out.switches += 1;
            }
            state = now;
        }
    }
    out.duration = last - first;
    out
}

fn check_oxygen_sensors(c: &mut Ctx) {
    if c.diesel() {
        return; // diesels have no switching sensors
    }
    let input = c.input;
    let title = format("Lambdasonden", "Oxygen sensors", &[]);
    // PIDs 14 and 18 are sensor 1 of bank 1 and 2 (in front of the
    // converter) for the usual sensor layout from PID 13; 15 and 19 are the
    // sensors behind it.
    const UPSTREAM: [i32; 2] = [0x14, 0x18];
    const DOWNSTREAM: [i32; 2] = [0x15, 0x19];
    if get(input, UPSTREAM[0]).is_none() && get(input, UPSTREAM[1]).is_none() {
        c.skip(
            "o2_switching",
            title,
            format(
                "Keine Spannungen der Lambdasonden (PID 14 bis 1B). Autos mit \
                 Breitbandsonden melden sie anders, dann ist diese Prüfung hier nicht \
                 möglich.",
                "No oxygen sensor voltages (PID 14 to 1B). Cars with wideband sensors \
                 report them differently, and then this check is not possible here.",
                &[],
            ),
        );
        return;
    }
    for bank in 0..2usize {
        let Some(up) = get(input, UPSTREAM[bank]) else {
            continue;
        };
        let id_suffix = if bank == 0 { "bank1" } else { "bank2" };
        let bank_name = format("Bank {}", "bank {}", &[number((bank + 1) as f64, 0)]);
        let sw = analyse_switching(c, up);
        if sw.samples < 10 || sw.duration < 10.0 {
            c.skip(
                format!("o2_switching_{id_suffix}"),
                format(
                    "Lambdasonde vorn ({})",
                    "Front oxygen sensor ({})",
                    std::slice::from_ref(&bank_name),
                ),
                format(
                    "Zu wenige Messwerte bei warmem Motor im geschlossenen Regelkreis. \
                     Nötig sind mindestens 10 Werte über 10 Sekunden.",
                    "Too few readings with a warm engine in closed loop. At least 10 \
                     readings over 10 seconds are needed.",
                    &[],
                ),
            );
            continue;
        }
        c.ran();
        let range = format(
            "{} und {} V",
            "{} and {} V",
            &[number(sw.low, 2), number(sw.high, 2)],
        );
        if sw.switches == 0 {
            let cause = if sw.high < 0.3 {
                format(
                    "Sie zeigt dauernd mager. Das kann die Sonde selbst sein, aber \
                     auch Falschluft oder ein Abgasleck vor der Sonde.",
                    "It reads lean all the time. That can be the sensor itself, but \
                     also an air leak or an exhaust leak in front of the sensor.",
                    &[],
                )
            } else if sw.low > 0.6 {
                format(
                    "Sie zeigt dauernd fett. Das kann die Sonde selbst sein oder \
                     wirklich zu viel Kraftstoff.",
                    "It reads rich all the time. That can be the sensor itself or \
                     genuinely too much fuel.",
                    &[],
                )
            } else {
                format(
                    "Die Spannung steht fast still. Typisch für eine gealterte oder \
                     verrußte Sonde.",
                    "The voltage barely moves. Typical of an aged or sooted sensor.",
                    &[],
                )
            };
            c.add(
                format!("o2_switching_{id_suffix}"),
                Level::Warning,
                Area::OxygenSensors,
                format(
                    "Lambdasonde vorn regelt nicht ({})",
                    "Front oxygen sensor not switching ({})",
                    &[bank_name],
                ),
                format(
                    "Bei warmem Motor sollte die Spannung der Sonde vor dem Katalysator \
                     ständig zwischen etwa 0,1 und 0,9 V hin und her springen. Hier lag \
                     sie {} Sekunden lang nur zwischen {}.",
                    "With a warm engine the voltage of the sensor in front of the catalytic \
                     converter should keep jumping between about 0.1 and 0.9 V. Here, for \
                     {} seconds, it only stayed between {}.",
                    &[number(sw.duration, 0), range],
                ) + cause,
                format(
                    "Sonde, Stecker und Kabel prüfen lassen.",
                    "Have the sensor, its plug and wire checked.",
                    &[],
                ),
            );
            continue;
        }
        let rate = f64::from(sw.switches) / sw.duration;
        if rate < 0.1 && sw.duration >= 30.0 {
            c.add(
                format!("o2_switching_{id_suffix}"),
                Level::Info,
                Area::OxygenSensors,
                format(
                    "Lambdasonde vorn eher träge ({})",
                    "Front oxygen sensor rather slow ({})",
                    std::slice::from_ref(&bank_name),
                ),
                format(
                    "Die Sonde hat in {} Sekunden nur {}-mal gewechselt. Eine frische Sonde \
                     wechselt bei warmem Motor etwa einmal pro Sekunde. Allerdings fragt der \
                     Adapter jeden Wert nur ein paar Mal pro Sekunde ab und kann schnelle \
                     Wechsel verpassen.",
                    "In {} seconds the sensor switched only {} times. A fresh sensor \
                     switches about once per second with a warm engine. However, the adapter \
                     polls each value only a few times per second and can miss fast changes.",
                    &[number(sw.duration, 0), number(f64::from(sw.switches), 0)],
                ),
                format(
                    "Bei 2000 bis 2500/min noch einmal messen. Bleibt sie träge, ist sie \
                     wahrscheinlich gealtert.",
                    "Measure again at 2000 to 2500 rpm. If it stays slow, it has probably \
                     aged.",
                    &[],
                ),
            );
        } else {
            c.add(
                format!("o2_switching_{id_suffix}"),
                Level::Good,
                Area::OxygenSensors,
                format(
                    "Lambdasonde vorn regelt ({})",
                    "Front oxygen sensor switching ({})",
                    std::slice::from_ref(&bank_name),
                ),
                format(
                    "{} Wechsel in {} Sekunden, Spannung zwischen {}.",
                    "{} switches in {} seconds, voltage between {}.",
                    &[
                        number(f64::from(sw.switches), 0),
                        number(sw.duration, 0),
                        range,
                    ],
                ),
                none(),
            );
        }

        // Catalyst: a good converter stores oxygen and smooths the signal
        // behind it. If the rear sensor copies the front one, it no longer
        // does that.
        let Some(down) = get(input, DOWNSTREAM[bank]) else {
            continue;
        };
        if sw.switches < 6 {
            continue;
        }
        let rear = analyse_switching(c, down);
        if rear.samples < 10 {
            continue;
        }
        c.ran();
        let ratio = f64::from(rear.switches) / f64::from(sw.switches);
        let caveat = format(
            "Das ist eine Schätzung aus den Live-Werten. Die genaue Prüfung macht das \
             Steuergerät selbst (Bereitschaftstest Katalysator und Modus 06).",
            "This is an estimate from live values. The control unit does the exact check \
             itself (catalyst readiness monitor and mode 06).",
            &[],
        );
        if ratio >= 0.7 {
            c.add(
                format!("catalyst_{id_suffix}"),
                Level::Warning,
                Area::Catalyst,
                format(
                    "Katalysator speichert wenig Sauerstoff ({})",
                    "Catalytic converter stores little oxygen ({})",
                    &[bank_name],
                ),
                format(
                    "Die Sonde hinter dem Katalysator wechselt fast so oft wie die davor \
                     ({} zu {} Wechsel). Ein guter Katalysator glättet das Signal \
                     dahinter. Das deutet auf einen gealterten Katalysator hin.",
                    "The sensor behind the converter switches almost as often as the one \
                     in front ({} to {} switches). A good converter smooths the signal \
                     behind it. That points to an aged converter.",
                    &[
                        number(f64::from(rear.switches), 0),
                        number(f64::from(sw.switches), 0),
                    ],
                ) + caveat,
                format(
                    "Auf den Fehler P0420/P0430 achten und vor einem Tausch die hintere \
                     Sonde und Abgaslecks prüfen lassen.",
                    "Watch for fault P0420/P0430, and before replacing anything have the \
                     rear sensor and exhaust leaks checked.",
                    &[],
                ),
            );
        } else {
            c.add(
                format!("catalyst_{id_suffix}"),
                Level::Good,
                Area::Catalyst,
                format(
                    "Katalysator arbeitet ({})",
                    "Catalytic converter working ({})",
                    &[bank_name],
                ),
                format(
                    "Die Sonde hinter dem Katalysator bleibt ruhig ({} Wechsel gegen {} \
                     vorn). So soll es sein.",
                    "The sensor behind the converter stays calm ({} switches against {} \
                     in front). That is how it should be.",
                    &[
                        number(f64::from(rear.switches), 0),
                        number(f64::from(sw.switches), 0),
                    ],
                ) + caveat,
                none(),
            );
        }
    }
}

// --- Mode 06 ---------------------------------------------------------------

fn is_misfire_component(r: &MonitorResult) -> bool {
    r.component_id >= 0xA1 && r.component_id <= 0xAD
}

fn not_run_yet(r: &MonitorResult) -> bool {
    r.value == 0.0 && r.minimum == 0.0 && r.maximum == 0.0
}

fn test_name(r: &MonitorResult) -> Phrase {
    let mut name = r.name.clone();
    if !r.component.is_empty() {
        name = if name.is_empty() {
            r.component.clone()
        } else {
            format!("{}: {}", r.component, name)
        };
    }
    if name.is_empty() {
        name = format!("Test {:02X}/{:02X}", r.component_id, r.test_id);
    }
    same(&name)
}

fn check_mode06(c: &mut Ctx) {
    let input = c.input;
    let title = format(
        "Bauteilprüfungen (Modus 06)",
        "Component tests (mode 06)",
        &[],
    );
    if !input.monitor_results_read {
        c.skip(
            "mode06",
            title,
            format(
                "Die Ergebnisse der Bauteilprüfungen wurden nicht gelesen.",
                "The component test results were not read.",
                &[],
            ),
        );
        return;
    }
    c.ran();
    let mut evaluated = 0;
    let mut problems = 0;
    for r in &input.monitor_results {
        if is_misfire_component(r) || not_run_yet(r) {
            continue;
        }
        evaluated += 1;
        let id = format!("mode06:{:02X}:{:02X}", r.component_id, r.test_id);
        let name = test_name(r);
        let measured = format(
            "Gemessen {} {}, erlaubt {} bis {}.",
            "Measured {} {}, allowed {} to {}.",
            &[
                number(r.value, 3),
                same(&r.unit),
                if r.has_minimum {
                    number(r.minimum, 3)
                } else {
                    same("-")
                },
                if r.has_maximum {
                    number(r.maximum, 3)
                } else {
                    same("-")
                },
            ],
        );
        if !r.passed {
            problems += 1;
            c.add(
                id,
                Level::Warning,
                Area::Monitoring,
                format("Prüfung nicht bestanden: {}", "Test failed: {}", &[name]),
                format(
                    "Das Steuergerät hat diesen Wert außerhalb seiner Grenzen gemessen. \
                     Meist folgt bald ein Fehlercode.",
                    "The control unit measured this value outside its limits. A fault \
                     code usually follows soon.",
                    &[],
                ) + measured,
                format(
                    "Das betroffene Bauteil prüfen lassen, bevor die Warnlampe angeht.",
                    "Have the component checked before the warning lamp comes on.",
                    &[],
                ),
            );
            continue;
        }
        // Zero limits are usually placeholders ("no lower limit"), so only
        // real limits count for the "close to the limit" check.
        let mut near = false;
        if r.has_maximum
            && r.maximum != 0.0
            && r.maximum - r.value <= limits::NEAR_LIMIT_SHARE * r.maximum.abs()
        {
            near = true;
        }
        if r.has_minimum
            && r.minimum != 0.0
            && r.value - r.minimum <= limits::NEAR_LIMIT_SHARE * r.minimum.abs()
        {
            near = true;
        }
        if near {
            problems += 1;
            c.add(
                id,
                Level::Info,
                Area::Monitoring,
                format("Knapp an der Grenze: {}", "Close to the limit: {}", &[name]),
                format(
                    "Die Prüfung ist bestanden, aber der Wert liegt weniger als 10 % von \
                     seiner Grenze entfernt. So kündigen sich Fehler oft lange vor der \
                     Warnlampe an.",
                    "The test passed, but the value is less than 10% away from its limit. \
                     That is how faults often show up long before the warning lamp.",
                    &[],
                ) + measured,
                format(
                    "Bei der nächsten Prüfung wieder ansehen. Wandert der Wert weiter zur \
                     Grenze, das Bauteil prüfen lassen.",
                    "Look again at the next check. If the value keeps moving towards the \
                     limit, have the component checked.",
                    &[],
                ),
            );
        }
    }
    if problems == 0 {
        c.add(
            "mode06",
            Level::Good,
            Area::Monitoring,
            format(
                "Bauteilprüfungen im grünen Bereich",
                "Component tests well within limits",
                &[],
            ),
            if evaluated > 0 {
                format(
                    "Alle {} Prüfungen liegen mit Abstand innerhalb ihrer Grenzen.",
                    "All {} tests are comfortably within their limits.",
                    &[number(f64::from(evaluated), 0)],
                )
            } else {
                format(
                    "Das Steuergerät hat noch keine Prüfung abgeschlossen.",
                    "The control unit has not completed any test yet.",
                    &[],
                )
            },
            none(),
        );
    }
}

fn check_misfire(c: &mut Ctx) {
    let input = c.input;
    let title = format("Zündaussetzer je Zylinder", "Misfires per cylinder", &[]);
    if !input.monitor_results_read {
        c.skip(
            "misfire",
            title,
            format(
                "Die Ergebnisse der Bauteilprüfungen (Modus 06) wurden nicht gelesen.",
                "The component test results (mode 06) were not read.",
                &[],
            ),
        );
        return;
    }
    // CAN mode 06: component A2..AD is cylinder 1..12, test 0B is the
    // average over the last ten drives, test 0C the count of the current or
    // last drive.
    let mut per_cylinder: BTreeMap<i32, f64> = BTreeMap::new();
    let mut failed = false;
    for r in &input.monitor_results {
        if r.component_id < 0xA2 || r.component_id > 0xAD {
            continue;
        }
        if r.test_id != 0x0B && r.test_id != 0x0C {
            continue;
        }
        let cyl = r.component_id - 0xA1;
        let entry = per_cylinder.entry(cyl).or_insert(0.0);
        *entry = entry.max(r.value);
        if !r.passed {
            failed = true;
        }
    }
    if per_cylinder.is_empty() {
        c.skip(
            "misfire",
            title,
            format(
                "Das Auto meldet keine Aussetzerzähler in Modus 06 (gibt es meist nur bei \
                 CAN-Fahrzeugen).",
                "The car reports no misfire counters in mode 06 (usually only on CAN \
                 vehicles).",
                &[],
            ),
        );
        return;
    }
    c.ran();
    let mut total = 0.0;
    let mut worst = 0.0;
    let mut worst_cyl = 0;
    let mut list = Phrase::default();
    for (cyl, count) in &per_cylinder {
        let (cyl, count) = (*cyl, *count);
        total += count;
        if count > worst {
            worst = count;
            worst_cyl = cyl;
        }
        if count > 0.0 {
            let item = format(
                "Zylinder {}: {}",
                "cylinder {}: {}",
                &[number(f64::from(cyl), 0), number(count, 0)],
            );
            if !list.is_empty() {
                list.de.push_str(", ");
                list.en.push_str(", ");
            }
            list.de.push_str(&item.de);
            list.en.push_str(&item.en);
        }
    }
    if total <= 0.0 && !failed {
        c.add(
            "misfire",
            Level::Good,
            Area::Misfire,
            format("Keine Zündaussetzer", "No misfires", &[]),
            format(
                "Alle {} Zylinder zünden sauber, die Zähler stehen auf null.",
                "All {} cylinders fire cleanly, the counters are at zero.",
                &[number(per_cylinder.len() as f64, 0)],
            ),
            none(),
        );
        return;
    }
    let level = if failed || worst >= 20.0 {
        Level::Warning
    } else {
        Level::Info
    };
    let mut focus = Phrase::default();
    if total >= 10.0 && worst >= 0.6 * total {
        focus = format(
            "Fast alle Aussetzer kommen von Zylinder {}. Das spricht für ein Teil \
             genau dieses Zylinders: Zündkerze, Zündspule oder Einspritzventil. Ein \
             einfacher Test: Zündspule mit einem Nachbarzylinder tauschen und sehen, \
             ob der Fehler mitwandert.",
            "Almost all misfires come from cylinder {}. That points to a part of \
             exactly this cylinder: spark plug, ignition coil or injector. A simple \
             test: swap the coil with a neighbouring cylinder and see whether the \
             fault moves with it.",
            &[number(f64::from(worst_cyl), 0)],
        );
    }
    c.add(
        "misfire",
        level,
        Area::Misfire,
        if level == Level::Warning {
            format("Zündaussetzer erkannt", "Misfires detected", &[])
        } else {
            format("Vereinzelte Zündaussetzer", "Occasional misfires", &[])
        },
        format(
            "Gezählte Aussetzer: {}. Einzelne Aussetzer kommen vor, etwa beim Kaltstart \
             oder bei schlechtem Kraftstoff. Viele davon schütteln den Motor und schaden \
             dem Katalysator. Ab wann das Steuergerät einen Fehler setzt, legt jeder \
             Hersteller selbst fest.",
            "Misfires counted: {}. A few misfires happen, for example on a cold start or \
             with poor fuel. Many of them shake the engine and harm the catalytic \
             converter. Each manufacturer decides itself when a fault is set.",
            &[list],
        ) + focus,
        if level == Level::Warning {
            format(
                "Zündkerzen und Zündspulen prüfen lassen.",
                "Have the spark plugs and ignition coils checked.",
                &[],
            )
        } else {
            format(
                "Im Auge behalten und nach einigen Fahrten wieder prüfen.",
                "Keep an eye on it and check again after a few drives.",
                &[],
            )
        },
    );
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
    if !r.not_checked.is_empty() {
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
    check_other_temperatures(&mut c);
    check_mode06(&mut c);
    check_misfire(&mut c);

    let mut report = c.out;
    // Stable sort, worst level first.
    report.findings.sort_by_key(|f| std::cmp::Reverse(f.level));
    score(&mut report);
    report
}
