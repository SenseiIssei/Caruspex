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
use crate::series::{latest, max_of, median, min_of};

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

fn none() -> Message {
    Message::default()
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

// --- Faults and lamps ---------------------------------------------------------

fn check_faults(c: &mut Ctx) {
    let input = c.input;
    if !input.faults_read {
        c.skip(
            "faults",
            text("Fehlerspeicher", "Fault memory"),
            text(
                "Die aktiven Fehler (DM1) ließen sich nicht lesen.",
                "The active faults (DM1) could not be read.",
            ),
        );
        return;
    }
    let with_faults: Vec<&TruckUnit> = input
        .units
        .iter()
        .filter(|u| !u.active.is_empty())
        .collect();
    if with_faults.is_empty() {
        c.add(
            "faults_none",
            Level::Good,
            text("Keine aktiven Fehler", "No active faults"),
            text(
                "Keines der %1 Steuergeräte meldet gerade einen aktiven Fehler.",
                "None of the %1 control units reports an active fault right now.",
            )
            .num(input.units.len() as f64, 0),
            none(),
        );
    }
    for unit in with_faults {
        // The lamp the unit lights tells how urgent it finds its faults;
        // a red stop lamp is reported by the lamp rule as well.
        let level = if unit.lamps.red_stop || unit.lamps.amber || unit.lamps.mil {
            Level::Warning
        } else {
            Level::Info
        };
        let lines: Vec<Message> = unit.active.iter().map(TruckFault::line).collect();
        c.add(
            "faults_unit",
            level,
            text("%1: %2 aktive Fehler", "%1: %2 active faults")
                .plain(unit.label.clone())
                .num(unit.active.len() as f64, 0),
            text(
                "Gerade aktiv: %1. SPN nennt den Messwert oder das Bauteil, FMI die Art des Fehlers.",
                "Active right now: %1. SPN names the value or the part, FMI the kind of fault.",
            )
            .list(lines),
            if level == Level::Info {
                text(
                    "Beim nächsten Werkstattbesuch ansprechen; keine Warnlampe verlangt sofortiges Handeln.",
                    "Mention it at the next workshop visit; no warning lamp calls for immediate action.",
                )
            } else {
                text(
                    "Bald in einer Lkw-Werkstatt prüfen lassen; der Fehler bleibt im Speicher, bis die Ursache behoben ist.",
                    "Have it checked at a truck workshop soon; the fault stays in memory until the cause is fixed.",
                )
            },
        );
    }
}

fn check_lamps(c: &mut Ctx) {
    let input = c.input;
    if !input.faults_read {
        return;
    }
    let lit = |pick: fn(&TruckLamps) -> bool| -> Vec<Message> {
        input
            .units
            .iter()
            .filter(|u| pick(&u.lamps))
            .map(|u| raw(u.label.clone()))
            .collect()
    };
    let red = lit(|l| l.red_stop);
    let amber = lit(|l| l.amber || l.mil);
    // A lamp lit for faults that are listed anyway counts once: the fault
    // finding of that unit carries the warning.
    let amber_unexplained = input
        .units
        .iter()
        .any(|u| (u.lamps.amber || u.lamps.mil) && u.active.is_empty());
    let protect = lit(|l| l.protect);
    let all_off = red.is_empty() && amber.is_empty() && protect.is_empty();
    if !red.is_empty() {
        c.add(
            "lamp_red_stop",
            Level::Critical,
            text("Rote Stopplampe an", "Red stop lamp on"),
            text(
                "Die rote Stopplampe leuchtet bei: %1. Sie meldet einen Fehler, bei dem Weiterfahren den Motor oder die Sicherheit gefährdet.",
                "The red stop lamp is lit for: %1. It reports a fault where driving on puts the engine or safety at risk.",
            )
            .list(red),
            text(
                "Anhalten, sobald es sicher geht, und den Fehler klären, bevor der Lkw weiterfährt.",
                "Stop as soon as it is safe and clear up the fault before the truck drives on.",
            ),
        );
    }
    if !amber.is_empty() {
        c.add(
            "lamp_amber",
            if amber_unexplained {
                Level::Warning
            } else {
                Level::Info
            },
            text("Gelbe Warnlampe an", "Amber warning lamp on"),
            text(
                "Die gelbe Warnlampe oder die Abgaslampe leuchtet bei: %1. Der Lkw darf weiterfahren, der Fehler gehört aber bald in die Werkstatt.",
                "The amber warning lamp or the emission lamp is lit for: %1. The truck may drive on, but the fault belongs in the workshop soon.",
            )
            .list(amber),
            text(
                "In den nächsten Tagen einen Werkstatttermin machen.",
                "Book a workshop visit in the next few days.",
            ),
        );
    }
    if !protect.is_empty() {
        c.add(
            "lamp_protect",
            Level::Info,
            text("Schutzlampe an", "Protect lamp on"),
            text(
                "Die Schutzlampe leuchtet bei: %1. Sie meldet ein Problem außerhalb des Motors, etwa an Getriebe oder Aufbau.",
                "The protect lamp is lit for: %1. It reports a problem outside the engine, for instance at the gearbox or the body.",
            )
            .list(protect),
            none(),
        );
    }
    if all_off {
        c.add(
            "lamps_off",
            Level::Good,
            text("Alle Warnlampen aus", "All warning lamps off"),
            text(
                "Kein Steuergerät schaltet die rote Stopplampe, die gelbe Warnlampe, die Abgaslampe oder die Schutzlampe ein.",
                "No control unit switches on the red stop lamp, the amber warning lamp, the emission lamp or the protect lamp.",
            ),
            none(),
        );
    }
}

// --- Temperatures and pressures ----------------------------------------------

fn check_coolant(c: &mut Ctx) {
    let input = c.input;
    let title = text("Kühlmitteltemperatur", "Coolant temperature");
    match input.series(spn::COOLANT_TEMP) {
        None => c.missing("coolant_temp", title, spn::COOLANT_TEMP),
        Some(s) => {
            let hot = max_of(s);
            if hot >= limits::COOLANT_CRITICAL {
                c.add(
                    "coolant_temp",
                    Level::Critical,
                    text("Motor zu heiß: %1 °C", "Engine too hot: %1 °C").num(hot, 0),
                    text(
                        "Das Kühlmittel erreicht %1 °C. Warm liegt ein Lkw-Motor meist zwischen 80 und 95 °C; über 105 °C drohen Schäden an Zylinderkopf und Dichtungen.",
                        "The coolant reaches %1 °C. A warm truck engine usually sits between 80 and 95 °C; above 105 °C the cylinder head and the gaskets are at risk.",
                    )
                    .num(hot, 0),
                    text(
                        "Last wegnehmen, im Leerlauf abkühlen lassen und Kühlmittelstand, Lüfterkupplung und Kühler prüfen lassen.",
                        "Take the load off, let it cool down at idle and have the coolant level, the fan clutch and the radiator checked.",
                    ),
                );
            } else if hot >= limits::COOLANT_WARNING {
                c.add(
                    "coolant_temp",
                    Level::Warning,
                    text("Kühlmittel warm: %1 °C", "Coolant warm: %1 °C").num(hot, 0),
                    text(
                        "Das Kühlmittel erreicht %1 °C, mehr als die üblichen 80 bis 95 °C. Unter schwerer Last am Berg kann das vorkommen, im Stand nicht.",
                        "The coolant reaches %1 °C, more than the usual 80 to 95 °C. That can happen under heavy load uphill, not standing still.",
                    )
                    .num(hot, 0),
                    text(
                        "Kühler auf Schmutz, Lüfter und Thermostat prüfen lassen.",
                        "Have the radiator checked for dirt, and the fan and the thermostat.",
                    ),
                );
            } else if hot < limits::WARM {
                c.add(
                    "coolant_temp",
                    Level::Info,
                    text("Motor noch kalt: %1 °C", "Engine still cold: %1 °C").num(hot, 0),
                    text(
                        "Das Kühlmittel hat %1 °C. Öldruck und Ladedruck sind bei kaltem Motor höher als warm; für ein sicheres Urteil den Check mit warmem Motor wiederholen.",
                        "The coolant is at %1 °C. Oil pressure and boost are higher with a cold engine than warm; for a sound verdict repeat the check with a warm engine.",
                    )
                    .num(hot, 0),
                    none(),
                );
            } else {
                c.add(
                    "coolant_temp",
                    Level::Good,
                    text("Kühlmittel %1 °C", "Coolant %1 °C").num(hot, 0),
                    text(
                        "Im üblichen Bereich eines warmen Lkw-Motors (80 bis 95 °C).",
                        "Within the usual range of a warm truck engine (80 to 95 °C).",
                    ),
                    none(),
                );
            }
        }
    }

    let level_title = text("Kühlmittelstand", "Coolant level");
    if let Some(s) = input.series(spn::COOLANT_LEVEL) {
        let low = min_of(s);
        let (level, title, advice) = if low < limits::COOLANT_LEVEL_CRITICAL {
            (
                Level::Critical,
                text("Kühlmittel fast leer: %1 %", "Coolant nearly empty: %1 %"),
                text(
                    "Nicht weiterfahren, bevor Kühlmittel nachgefüllt und die Undichtigkeit gefunden ist.",
                    "Do not drive on before coolant is topped up and the leak is found.",
                ),
            )
        } else if low < limits::COOLANT_LEVEL_WARNING {
            (
                Level::Warning,
                text("Kühlmittelstand niedrig: %1 %", "Coolant level low: %1 %"),
                text(
                    "Bei kaltem Motor nachfüllen und beobachten; sinkt der Stand wieder, auf Undichtigkeit prüfen lassen.",
                    "Top up with a cold engine and keep an eye on it; if the level drops again, have it checked for leaks.",
                ),
            )
        } else {
            (
                Level::Good,
                text("Kühlmittelstand %1 %", "Coolant level %1 %"),
                none(),
            )
        };
        c.add(
            "coolant_level",
            level,
            title.num(low, 0),
            text(
                "Füllstand im Ausgleichsbehälter laut Sensor. Manche Lkw messen ihn nur als voll oder leer.",
                "Fill level in the expansion tank according to the sensor. Some trucks only measure it as full or empty.",
            ),
            advice,
        );
    } else {
        c.missing("coolant_level", level_title, spn::COOLANT_LEVEL);
    }
}

fn check_oil(c: &mut Ctx) {
    let input = c.input;
    let temp_title = text("Öltemperatur", "Oil temperature");
    match input.series(spn::OIL_TEMP) {
        None => c.missing("oil_temp", temp_title, spn::OIL_TEMP),
        Some(s) => {
            let hot = max_of(s);
            let (level, title, advice) = if hot >= limits::OIL_TEMP_CRITICAL {
                (
                    Level::Critical,
                    text("Öl zu heiß: %1 °C", "Oil too hot: %1 °C"),
                    text(
                        "Last wegnehmen und Ölkühler und Ölstand prüfen lassen; so heiß verliert das Öl seine Schmierfähigkeit.",
                        "Take the load off and have the oil cooler and the oil level checked; this hot the oil loses its lubricating power.",
                    ),
                )
            } else if hot >= limits::OIL_TEMP_WARNING {
                (
                    Level::Warning,
                    text("Öl heiß: %1 °C", "Oil hot: %1 °C"),
                    text(
                        "Ölkühler und Ölstand bei der nächsten Gelegenheit prüfen lassen.",
                        "Have the oil cooler and the oil level checked at the next opportunity.",
                    ),
                )
            } else {
                (
                    Level::Good,
                    text("Öltemperatur %1 °C", "Oil temperature %1 °C"),
                    none(),
                )
            };
            c.add(
                "oil_temp",
                level,
                title.num(hot, 0),
                text(
                    "Höchster Wert während der Messung. Warmes Motoröl liegt meist zwischen 85 und 110 °C.",
                    "Highest value during the measurement. Warm engine oil usually sits between 85 and 110 °C.",
                ),
                advice,
            );
        }
    }

    let pressure_title = text("Öldruck im Leerlauf", "Oil pressure at idle");
    let Some(pressure) = input.series(spn::OIL_PRESSURE) else {
        c.missing("oil_pressure", pressure_title, spn::OIL_PRESSURE);
        return;
    };
    let coolant = input.series(spn::COOLANT_TEMP).map(latest);
    let idle: Vec<f64> = pressure
        .iter()
        .filter(|v| {
            input
                .rpm_at(v.t)
                .is_some_and(|r| r > limits::RUNNING_RPM && r <= limits::IDLE_MAX_RPM)
        })
        .map(|v| v.value)
        .collect();
    if idle.is_empty() {
        c.skip(
            "oil_pressure",
            pressure_title,
            text(
                "Der Motor lief während der Messung nicht im Leerlauf; der Öldruck wird nur bei laufendem Motor im Leerlauf beurteilt.",
                "The engine did not idle during the measurement; the oil pressure is only judged with the engine idling.",
            ),
        );
        return;
    }
    let low = idle.iter().copied().fold(f64::MAX, f64::min);
    let cold = coolant.is_some_and(|t| t < limits::WARM);
    let (level, title, advice) = if low < limits::OIL_IDLE_CRITICAL {
        (
            Level::Critical,
            text(
                "Öldruck im Leerlauf zu niedrig: %1 kPa",
                "Oil pressure at idle too low: %1 kPa",
            ),
            text(
                "Motor abstellen und Ölstand prüfen. Bleibt der Druck niedrig, nicht weiterfahren: Lager und Turbolader leiden zuerst.",
                "Switch off the engine and check the oil level. If the pressure stays low, do not drive on: bearings and turbocharger suffer first.",
            ),
        )
    } else if low < limits::OIL_IDLE_WARNING {
        (
            Level::Warning,
            text(
                "Öldruck im Leerlauf knapp: %1 kPa",
                "Oil pressure at idle marginal: %1 kPa",
            ),
            text(
                "Ölstand und Ölsorte prüfen und den Druck mit einem Manometer in der Werkstatt nachmessen lassen.",
                "Check the oil level and grade and have the pressure measured with a gauge at the workshop.",
            ),
        )
    } else {
        (
            Level::Good,
            text("Öldruck im Leerlauf %1 kPa", "Oil pressure at idle %1 kPa"),
            none(),
        )
    };
    c.add(
        "oil_pressure",
        level,
        title.num(low, 0),
        if cold {
            text(
                "Niedrigster Wert im Leerlauf. Viele Hersteller verlangen warm mindestens etwa 70 kPa; kaltes Öl zeigt mehr Druck als warmes, der Wert ist also eher zu gut.",
                "Lowest value at idle. Many makers ask for at least about 70 kPa warm; cold oil shows more pressure than warm oil, so the value is on the optimistic side.",
            )
        } else {
            text(
                "Niedrigster Wert im warmen Leerlauf. Viele Hersteller verlangen hier mindestens etwa 70 kPa.",
                "Lowest value at warm idle. Many makers ask for at least about 70 kPa here.",
            )
        },
        advice,
    );
}

// --- Electrics -----------------------------------------------------------------

fn check_voltage(c: &mut Ctx) {
    let input = c.input;
    let charge_title = text("Ladespannung", "Charging voltage");
    let rest_title = text(
        "Batterie bei stehendem Motor",
        "Battery with the engine off",
    );
    let volts = input
        .series(spn::BATTERY)
        .or_else(|| input.series(spn::CHARGING));
    let Some(volts) = volts else {
        c.missing("charging", charge_title, spn::BATTERY);
        return;
    };
    // The system says its own voltage: a battery that never reaches 16 V
    // belongs to a 12 V system (many buses, US trucks), else it is 24 V.
    let twelve = max_of(volts) <= limits::TWELVE_VOLT_MAX;
    let factor = if twelve { 0.5 } else { 1.0 };
    c.system_volts = if twelve { 12 } else { 24 };
    let mut running = Vec::new();
    let mut off = Vec::new();
    for v in volts {
        match input.rpm_at(v.t) {
            Some(r) if r > limits::RUNNING_RPM => running.push(v.value),
            Some(r) if r < limits::STOPPED_RPM => off.push(v.value),
            _ => {}
        }
    }
    let system = if twelve {
        text("12-V-Anlage", "12 V system")
    } else {
        text("24-V-Anlage", "24 V system")
    };
    let engine_ran = !running.is_empty();
    if !engine_ran {
        c.skip(
            "charging",
            charge_title,
            text(
                "Der Motor lief während der Messung nicht.",
                "The engine was not running during the measurement.",
            ),
        );
    } else {
        let v = median(running);
        let normal = if twelve {
            text("13,8 bis 14,6 V", "13.8 to 14.6 V")
        } else {
            text("27,6 bis 28,8 V", "27.6 to 28.8 V")
        };
        let (level, title, advice) = if v < limits::CHARGING_NONE_24 * factor {
            (
                Level::Critical,
                text(
                    "Lichtmaschine lädt nicht: %1 V",
                    "Alternator not charging: %1 V",
                ),
                text(
                    "Bald prüfen lassen: Lichtmaschine, Riemen, Kabel und Masse. Bis dahin kann der Lkw liegen bleiben.",
                    "Have it checked soon: alternator, belt, cables and ground. Until then the truck may break down.",
                ),
            )
        } else if v < limits::CHARGING_LOW_24 * factor {
            (
                Level::Warning,
                text("Ladespannung niedrig: %1 V", "Charging voltage low: %1 V"),
                text(
                    "Lichtmaschine und Batterien bei Gelegenheit testen lassen.",
                    "Have the alternator and the batteries tested when convenient.",
                ),
            )
        } else if v > limits::CHARGING_HIGH_24 * factor {
            (
                Level::Warning,
                text(
                    "Ladespannung zu hoch: %1 V",
                    "Charging voltage too high: %1 V",
                ),
                text(
                    "Den Laderegler prüfen lassen; zu hohe Spannung kocht die Batterien aus.",
                    "Have the voltage regulator checked; too high a voltage boils the batteries dry.",
                ),
            )
        } else {
            (
                Level::Good,
                text("Ladespannung %1 V", "Charging voltage %1 V"),
                none(),
            )
        };
        c.add(
            "charging",
            level,
            title.num(v, 1),
            text(
                "%1, gemessen bei laufendem Motor. Üblich sind %2.",
                "%1, measured with the engine running. Usual is %2.",
            )
            .list(vec![system.clone()])
            .list(vec![normal]),
            advice,
        );
    }
    if !off.is_empty() {
        let v = median(off);
        let (level, title, advice) = if v < limits::REST_FLAT_24 * factor {
            (
                Level::Critical,
                text("Batterien fast leer: %1 V", "Batteries nearly flat: %1 V"),
                text(
                    "Batterien laden und testen lassen; so tief entladen leiden sie dauerhaft.",
                    "Have the batteries charged and tested; discharged this deep they suffer lasting damage.",
                ),
            )
        } else if v < limits::REST_LOW_24 * factor {
            (
                Level::Warning,
                text("Batterien schwach: %1 V", "Batteries weak: %1 V"),
                text(
                    "Batterien voll laden und testen lassen, besonders vor dem Winter.",
                    "Have the batteries fully charged and tested, especially before winter.",
                ),
            )
        } else {
            (
                Level::Good,
                text("Batterien %1 V", "Batteries %1 V"),
                none(),
            )
        };
        c.add(
            "battery_rest",
            level,
            title.num(v, 1),
            text(
                "%1 bei stehendem Motor. Volle Batterien liegen bei etwa %2.",
                "%1 with the engine off. Full batteries sit at about %2.",
            )
            .list(vec![system])
            .list(vec![if twelve {
                text("12,6 V", "12.6 V")
            } else {
                text("25,2 V", "25.2 V")
            }]),
            advice,
        );
    } else if engine_ran {
        c.skip(
            "battery_rest",
            rest_title,
            text(
                "Der Motor lief die ganze Zeit; die Ruhespannung lässt sich nur bei stehendem Motor messen.",
                "The engine ran the whole time; the resting voltage can only be measured with the engine off.",
            ),
        );
    }
}

// --- Exhaust aftertreatment ------------------------------------------------------

fn check_def(c: &mut Ctx) {
    let input = c.input;
    let title = text("AdBlue-Füllstand", "DEF (AdBlue) level");
    match input.series(spn::DEF_LEVEL) {
        None => c.missing("def_level", title, spn::DEF_LEVEL),
        Some(s) => {
            let level_now = latest(s);
            let (level, title, advice) = if level_now < limits::DEF_CRITICAL {
                (
                    Level::Critical,
                    text("AdBlue fast leer: %1 %", "DEF nearly empty: %1 %"),
                    text(
                        "Sofort AdBlue nachfüllen. Ist der Tank leer, drosselt der Motor seine Leistung und startet nach dem Abstellen nur noch eingeschränkt.",
                        "Top up DEF at once. Once the tank is empty, the engine cuts its power and after switching off only restarts with limits.",
                    ),
                )
            } else if level_now < limits::DEF_WARNING {
                (
                    Level::Warning,
                    text("AdBlue niedrig: %1 %", "DEF low: %1 %"),
                    text("Bald AdBlue nachfüllen.", "Top up DEF soon."),
                )
            } else {
                (Level::Good, text("AdBlue %1 %", "DEF %1 %"), none())
            };
            c.add(
                "def_level",
                level,
                title.num(level_now, 0),
                text(
                    "Füllstand des AdBlue-Tanks. Ohne AdBlue verliert der Motor nach einer Warnphase Leistung, das schreibt die Abgasnorm vor.",
                    "Fill level of the DEF tank. Without DEF the engine loses power after a warning phase; the emission rules demand it.",
                ),
                advice,
            );
        }
    }

    if !input.faults_read {
        return;
    }
    let mut active = Vec::new();
    let mut previous = Vec::new();
    for unit in &input.units {
        active.extend(
            unit.active
                .iter()
                .filter(|f| spn::DEF_SYSTEM.contains(&f.spn))
                .map(TruckFault::line),
        );
        previous.extend(
            unit.previous
                .iter()
                .filter(|f| spn::DEF_SYSTEM.contains(&f.spn))
                .map(TruckFault::line),
        );
    }
    let explain = text(
        "AdBlue-Fehler: %1. Häufige Ursachen sind verunreinigtes oder verdünntes AdBlue, Kristalle an Dosierventil und Leitungen oder ein defekter Qualitätssensor. Bleibt so ein Fehler bestehen, drosselt der Motor nach einer Warnphase die Leistung.",
        "DEF faults: %1. Common causes are contaminated or diluted DEF, crystals at the dosing valve and lines, or a faulty quality sensor. If such a fault stays, the engine cuts its power after a warning phase.",
    );
    if !active.is_empty() {
        c.add(
            "def_faults",
            Level::Warning,
            text("Aktive AdBlue-Fehler", "Active DEF faults"),
            explain.list(active),
            text(
                "Nur frisches AdBlue nach ISO 22241 tanken und die Anlage in der Werkstatt prüfen lassen.",
                "Fill only fresh DEF to ISO 22241 and have the system checked at the workshop.",
            ),
        );
    } else if !previous.is_empty() {
        c.add(
            "def_faults",
            Level::Info,
            text("Frühere AdBlue-Fehler", "Previous DEF faults"),
            explain.list(previous),
            none(),
        );
    }
}

/// Words for the state of SPN 3701.
fn dpf_state_text(state: f64) -> Message {
    match state as i32 {
        0 => text("Freibrennen nicht nötig", "no regeneration needed"),
        1 => text(
            "Freibrennen nötig, niedrige Stufe",
            "regeneration needed, low level",
        ),
        2 => text(
            "Freibrennen nötig, mittlere Stufe",
            "regeneration needed, moderate level",
        ),
        _ => text(
            "Freibrennen nötig, höchste Stufe",
            "regeneration needed, highest level",
        ),
    }
}

fn check_dpf(c: &mut Ctx) {
    let input = c.input;
    let soot = input.series(spn::SOOT_LOAD).map(latest);
    let state = input.series(spn::DPF_STATE).map(latest);
    let regen = input.series(spn::REGENERATION).map(latest);
    if soot.is_none() && state.is_none() && regen.is_none() {
        c.skip(
            "dpf",
            text("Partikelfilter", "Particulate filter"),
            text(
                "Der Lkw sendet weder Rußbeladung noch Regenerationszustand (SPN 3719, 3701, 3700). Ältere Lkw ohne Partikelfilter haben beides nicht.",
                "The truck sends neither the soot load nor the regeneration state (SPN 3719, 3701, 3700). Older trucks without a particulate filter have neither.",
            ),
        );
        return;
    }
    let mut facts = Vec::new();
    if let Some(s) = soot {
        facts.push(text("Rußbeladung %1 %", "soot load %1 %").num(s, 0));
    }
    if let Some(s) = state {
        facts.push(dpf_state_text(s));
    }
    let running = regen.is_some_and(|r| r as i32 == 1);
    if running {
        facts.push(text("Freibrennen läuft", "regeneration running"));
    }
    let worst_state = state.map_or(0, |s| s as i32);
    let soot_pct = soot.unwrap_or(0.0);
    let (level, title, advice) = if soot_pct >= limits::SOOT_CRITICAL || worst_state >= 3 {
        (
            Level::Critical,
            text("Partikelfilter voll", "Particulate filter full"),
            text(
                "Bald in die Werkstatt: ein so voller Filter braucht oft ein Freibrennen im Stand mit Diagnosegerät, sonst drosselt der Motor.",
                "To the workshop soon: a filter this full often needs a parked regeneration with a diagnostic tool, otherwise the engine cuts its power.",
            ),
        )
    } else if soot_pct >= limits::SOOT_WARNING
        || worst_state >= 1
        || regen.is_some_and(|r| r as i32 == 2)
    {
        (
            Level::Warning,
            text(
                "Partikelfilter muss freibrennen",
                "Particulate filter needs a regeneration",
            ),
            text(
                "Eine längere Fahrt mit gleichmäßiger Last, etwa eine halbe Stunde Autobahn, lässt den Filter meist von selbst freibrennen. Klappt das nicht, das Freibrennen im Stand nach Herstelleranleitung starten.",
                "A longer drive at steady load, about half an hour on the motorway, usually lets the filter regenerate by itself. If that does not work, start a parked regeneration as the maker describes.",
            ),
        )
    } else if running {
        (
            Level::Info,
            text(
                "Partikelfilter brennt gerade frei",
                "Particulate filter is regenerating",
            ),
            text(
                "Den Motor jetzt nicht abstellen, bis das Freibrennen fertig ist.",
                "Do not switch off the engine now until the regeneration is done.",
            ),
        )
    } else {
        (
            Level::Good,
            text("Partikelfilter in Ordnung", "Particulate filter fine"),
            none(),
        )
    };
    c.add(
        "dpf",
        level,
        title,
        text(
            "%1. Der Filter sammelt Ruß und verbrennt ihn regelmäßig bei hoher Abgastemperatur. Viel Kurzstrecke und Leerlauf lassen ihn schneller voll werden.",
            "%1. The filter collects soot and burns it off regularly at high exhaust temperature. Lots of short trips and idling fill it faster.",
        )
        .list(facts),
        advice,
    );

    if let Some(ash) = input.series(spn::ASH_LOAD).map(latest)
        && ash >= limits::ASH_WARNING
    {
        c.add(
            "dpf_ash",
            Level::Warning,
            text("Partikelfilter voller Asche: %1 %", "Particulate filter full of ash: %1 %").num(ash, 0),
            text(
                "Asche aus dem Motoröl brennt nicht ab und setzt den Filter mit der Zeit zu. Ab etwa 80 % steht eine Reinigung oder ein Tausch an.",
                "Ash from the engine oil does not burn off and clogs the filter over time. From about 80 % a cleaning or a replacement is due.",
            ),
            text(
                "Filterreinigung bei der Werkstatt einplanen.",
                "Plan a filter cleaning at the workshop.",
            ),
        );
    }
}

// --- Air path --------------------------------------------------------------------

fn check_air(c: &mut Ctx) {
    let input = c.input;
    let intake_title = text("Ladelufttemperatur", "Intake manifold temperature");
    if input.sensor_errors.contains(&spn::INTAKE_TEMP) {
        c.add(
            "intake_temp",
            Level::Warning,
            text(
                "Sensor der Ladelufttemperatur meldet Fehler",
                "Intake manifold temperature sensor reports a fault",
            ),
            text(
                "Das Motorsteuergerät kann die Temperatur im Ansaugkrümmer nicht messen und rechnet mit einem Ersatzwert. Der Motor läuft, aber oft mit weniger Leistung und mehr Verbrauch.",
                "The engine control unit cannot measure the temperature in the intake manifold and works with a substitute value. The engine runs, but often with less power and more fuel.",
            ),
            text(
                "Stecker und Kabel des Sensors prüfen lassen, dann den Sensor.",
                "Have the plug and the wiring of the sensor checked, then the sensor.",
            ),
        );
    } else if let Some(s) = input.series(spn::INTAKE_TEMP) {
        let hot = max_of(s);
        let (level, title, advice) = if hot >= limits::INTAKE_WARNING {
            (
                Level::Warning,
                text("Ladeluft heiß: %1 °C", "Intake air hot: %1 °C"),
                text(
                    "Ladeluftkühler außen reinigen und auf Undichtigkeit prüfen lassen.",
                    "Have the charge air cooler cleaned outside and checked for leaks.",
                ),
            )
        } else {
            (
                Level::Good,
                text("Ladeluft %1 °C", "Intake air %1 °C"),
                none(),
            )
        };
        c.add(
            "intake_temp",
            level,
            title.num(hot, 0),
            text(
                "Höchster Wert hinter dem Ladeluftkühler. Heiße Ladeluft kostet Leistung und erhöht die Abgastemperatur.",
                "Highest value after the charge air cooler. Hot intake air costs power and raises the exhaust temperature.",
            ),
            advice,
        );
    } else {
        c.missing("intake_temp", intake_title, spn::INTAKE_TEMP);
    }

    let boost_title = text("Ladedruck", "Boost pressure");
    let Some(boost) = input.series(spn::BOOST) else {
        c.missing("boost", boost_title, spn::BOOST);
        return;
    };
    let load = input.series(spn::ENGINE_LOAD);
    let under_load: Vec<f64> = boost
        .iter()
        .filter(|v| {
            let loaded = load.is_some_and(|l| {
                l.iter()
                    .filter(|x| (x.t - v.t).abs() <= 3.0)
                    .any(|x| x.value >= limits::BOOST_LOAD)
            });
            loaded && input.rpm_at(v.t).is_some_and(|r| r >= 1000.0)
        })
        .map(|v| v.value)
        .collect();
    if under_load.is_empty() {
        c.skip(
            "boost",
            boost_title,
            text(
                "Unter Last nicht gemessen. Im Stand ohne Last baut der Turbolader kaum Druck auf, der Wert sagt dann nichts.",
                "Not measured under load. Standing without load the turbocharger builds hardly any pressure, so the value says nothing.",
            ),
        );
        return;
    }
    let typical = median(under_load);
    let (level, title, advice) = if typical < limits::BOOST_LOW {
        (
            Level::Warning,
            text(
                "Ladedruck unter Last niedrig: %1 kPa",
                "Boost under load low: %1 kPa",
            ),
            text(
                "Ladeluftschläuche auf Risse und den Turbolader prüfen lassen.",
                "Have the charge air hoses checked for cracks and the turbocharger inspected.",
            ),
        )
    } else {
        (
            Level::Good,
            text("Ladedruck unter Last %1 kPa", "Boost under load %1 kPa"),
            none(),
        )
    };
    c.add(
        "boost",
        level,
        title.num(typical, 0),
        text(
            "Mittlerer Überdruck im Ansaugkrümmer bei mindestens 40 % Last. Ein undichter Ladeluftweg oder ein müder Turbolader zeigen sich hier zuerst.",
            "Typical pressure above ambient in the intake manifold at 40 % load or more. A leaking charge air path or a tired turbocharger show here first.",
        ),
        advice,
    );
}

fn check_hours(c: &mut Ctx) {
    let Some(hours) = c.input.engine_hours else {
        c.skip(
            "engine_hours",
            text("Motorbetriebsstunden", "Engine hours"),
            text(
                "Der Motor hat auf die Anfrage nach den Betriebsstunden (SPN 247) nicht geantwortet.",
                "The engine did not answer the request for its hours (SPN 247).",
            ),
        );
        return;
    };
    c.add(
        "engine_hours",
        Level::Good,
        text("%1 Motorbetriebsstunden", "%1 engine hours").num(hours, 0),
        text(
            "Viele Wartungen von Lkw und Bussen richten sich nach Stunden statt nach Kilometern, etwa bei viel Leerlauf oder Nebenantrieb. Mit dem Wartungsheft vergleichen.",
            "Many services of trucks and buses go by hours instead of kilometres, for instance with a lot of idling or power take-off. Compare with the service records.",
        ),
        none(),
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use caruspex_i18n::{Language, with_language};

    /// A warm 24 V truck at idle, as the demo truck sends it: everything in
    /// order except the particulate filter, which needs a regeneration.
    fn warm_idle() -> TruckHealthInput {
        let mut input = TruckHealthInput {
            faults_read: true,
            engine_hours: Some(12_345.6),
            units: vec![
                TruckUnit {
                    address: 0,
                    label: "Engine (00)".into(),
                    ..TruckUnit::default()
                },
                TruckUnit {
                    address: 0x17,
                    label: "Instrument cluster (17)".into(),
                    ..TruckUnit::default()
                },
            ],
            ..TruckHealthInput::default()
        };
        for i in 0..10 {
            let t = f64::from(i);
            for (spn, value) in [
                (spn::ENGINE_SPEED, 600.0),
                (spn::ENGINE_LOAD, 8.0),
                (spn::COOLANT_TEMP, 85.0),
                (spn::COOLANT_LEVEL, 92.0),
                (spn::OIL_TEMP, 82.0),
                (spn::OIL_PRESSURE, 240.0),
                (spn::BATTERY, 27.8),
                (spn::DEF_LEVEL, 48.0),
                (spn::SOOT_LOAD, 82.0),
                (spn::ASH_LOAD, 38.0),
                (spn::REGENERATION, 0.0),
                (spn::DPF_STATE, 2.0),
                (spn::BOOST, 0.0),
                (spn::INTAKE_TEMP, 35.0),
            ] {
                input.add_value(spn, value, t);
            }
        }
        input
    }

    fn level(report: &TruckHealthReport, id: &str) -> Level {
        report
            .find(id)
            .unwrap_or_else(|| panic!("no finding {id}"))
            .level
    }

    #[test]
    fn the_demo_truck_is_amber_for_its_particulate_filter() {
        let report = run_truck_health_check(&warm_idle());
        assert_eq!(report.grade, TrafficLight::Yellow);
        assert_eq!(level(&report, "dpf"), Level::Warning);
        for id in [
            "lamps_off",
            "faults_none",
            "coolant_temp",
            "coolant_level",
            "oil_temp",
            "oil_pressure",
            "charging",
            "def_level",
            "intake_temp",
            "engine_hours",
        ] {
            assert_eq!(level(&report, id), Level::Good, "{id}");
        }
        assert_eq!(report.system_volts, 24);
        assert!(report.score <= 84 && report.score >= 80, "{}", report.score);
        // Boost is not judged at idle, and says why.
        assert!(report.not_checked.iter().any(|s| s.id == "boost"));
        assert!(report.not_checked.iter().any(|s| s.id == "battery_rest"));
        let text = with_language(Language::English, || report.findings[0].explanation.text());
        assert!(text.contains("soot load 82 %"), "{text}");
    }

    #[test]
    fn temperatures_pressures_and_levels_cross_their_limits() {
        let mut input = warm_idle();
        input.values.clear();
        for (spn, value) in [
            (spn::ENGINE_SPEED, 650.0),
            (spn::COOLANT_TEMP, 107.0),
            (spn::COOLANT_LEVEL, 70.0),
            (spn::OIL_TEMP, 118.0),
            (spn::OIL_PRESSURE, 60.0),
            (spn::BATTERY, 25.0),
            (spn::DEF_LEVEL, 3.0),
            (spn::SOOT_LOAD, 104.0),
            (spn::INTAKE_TEMP, 80.0),
        ] {
            input.add_value(spn, value, 0.0);
        }
        let report = run_truck_health_check(&input);
        assert_eq!(report.grade, TrafficLight::Red);
        assert!(report.score <= 49);
        assert_eq!(level(&report, "coolant_temp"), Level::Critical);
        assert_eq!(level(&report, "coolant_level"), Level::Warning);
        assert_eq!(level(&report, "oil_temp"), Level::Warning);
        assert_eq!(level(&report, "oil_pressure"), Level::Critical);
        assert_eq!(level(&report, "charging"), Level::Critical);
        assert_eq!(level(&report, "def_level"), Level::Critical);
        assert_eq!(level(&report, "dpf"), Level::Critical);
        assert_eq!(level(&report, "intake_temp"), Level::Warning);
        // Worst first.
        assert_eq!(report.findings[0].level, Level::Critical);
    }

    #[test]
    fn twelve_volt_systems_are_judged_on_their_own_scale() {
        let mut input = TruckHealthInput::default();
        input.add_value(spn::ENGINE_SPEED, 0.0, 0.0);
        input.add_value(spn::BATTERY, 12.1, 0.0);
        input.add_value(spn::ENGINE_SPEED, 700.0, 10.0);
        input.add_value(spn::BATTERY, 14.1, 10.0);
        let report = run_truck_health_check(&input);
        assert_eq!(report.system_volts, 12);
        assert_eq!(level(&report, "charging"), Level::Good);
        assert_eq!(level(&report, "battery_rest"), Level::Warning);
        // A 24 V truck at 25.8 V with the engine running charges too little.
        let mut input = TruckHealthInput::default();
        input.add_value(spn::ENGINE_SPEED, 700.0, 0.0);
        input.add_value(spn::BATTERY, 25.8, 0.0);
        let report = run_truck_health_check(&input);
        assert_eq!(report.system_volts, 24);
        assert_eq!(level(&report, "charging"), Level::Warning);
        input.values.clear();
        input.add_value(spn::ENGINE_SPEED, 700.0, 0.0);
        input.add_value(spn::BATTERY, 30.2, 0.0);
        assert_eq!(
            level(&run_truck_health_check(&input), "charging"),
            Level::Warning
        );
    }

    #[test]
    fn faults_lamps_and_def_faults() {
        let mut input = warm_idle();
        input.units[0].lamps = TruckLamps {
            mil: true,
            amber: true,
            ..TruckLamps::default()
        };
        input.units[0].active = vec![
            TruckFault {
                spn: 105,
                fmi: 3,
                text: "Intake manifold temperature: voltage too high".into(),
                occurrences: Some(7),
            },
            TruckFault {
                spn: 3364,
                fmi: 2,
                ..TruckFault::default()
            },
        ];
        input.units[1].lamps.red_stop = true;
        input.add_sensor_error(spn::INTAKE_TEMP);
        let report = run_truck_health_check(&input);
        assert_eq!(level(&report, "lamp_red_stop"), Level::Critical);
        // The engine's faults explain its lamp, so the lamp counts once.
        assert_eq!(level(&report, "lamp_amber"), Level::Info);
        assert_eq!(level(&report, "faults_unit"), Level::Warning);
        assert_eq!(level(&report, "def_faults"), Level::Warning);
        assert_eq!(level(&report, "intake_temp"), Level::Warning);
        assert!(report.find("faults_none").is_none());
        let english = with_language(Language::English, || {
            report.find("faults_unit").unwrap().explanation.text()
        });
        assert!(
            english
                .contains("SPN 105 FMI 3 (Intake manifold temperature: voltage too high), 7 times"),
            "{english}"
        );
        let german = with_language(Language::German, || {
            report.find("lamp_red_stop").unwrap().explanation.text()
        });
        assert!(german.contains("Instrument cluster (17)"), "{german}");
    }

    #[test]
    fn missing_values_are_listed_with_their_spn() {
        let input = TruckHealthInput::default();
        let report = run_truck_health_check(&input);
        assert!(report.findings.is_empty());
        assert_eq!(report.grade, TrafficLight::Green);
        let reason = with_language(Language::English, || {
            report
                .not_checked
                .iter()
                .find(|s| s.id == "coolant_temp")
                .unwrap()
                .reason
                .text()
        });
        assert_eq!(reason, "The truck does not send this value (SPN 110).");
        assert!(report.not_checked.iter().any(|s| s.id == "faults"));
        assert!(report.not_checked.iter().any(|s| s.id == "dpf"));
    }

    #[test]
    fn boost_is_judged_under_load_only() {
        let mut input = TruckHealthInput::default();
        for i in 0..5 {
            let t = f64::from(i);
            input.add_value(spn::ENGINE_SPEED, 1400.0, t);
            input.add_value(spn::ENGINE_LOAD, 70.0, t);
            input.add_value(spn::BOOST, 35.0, t);
        }
        assert_eq!(
            level(&run_truck_health_check(&input), "boost"),
            Level::Warning
        );
        input.values.remove(&spn::BOOST);
        for i in 0..5 {
            input.add_value(spn::BOOST, 160.0, f64::from(i));
        }
        assert_eq!(level(&run_truck_health_check(&input), "boost"), Level::Good);
    }
}
