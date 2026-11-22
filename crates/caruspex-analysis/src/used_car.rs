//! Used car check: for someone standing next to a car they might buy.
//!
//! The caller reads what the car tells about itself (the VIN of the engine
//! control unit, the odometer of every control unit that has a documented
//! identifier for it, when the fault memory was last cleared, readiness,
//! faults, a short health check and, for electric cars, the battery) and
//! hands it in; this module judges it and answers with a traffic light, the
//! findings in plain words and the questions to ask the seller.
//!
//! Like the health check, every rule only runs on what the car really
//! answered; whatever could not be checked is listed separately.
//!
//! Texts are kept as [`Message`]: the German and English pattern with its
//! arguments, rendered in the active language when shown. The patterns are
//! looked up in the catalog like every other core text, so the result is
//! translated into all languages, numbers included.

use caruspex_i18n::l;

use crate::common::{Level, TrafficLight};
pub use crate::message::{Arg, Message, raw, text};
use crate::readiness::MonitorState;

// The rules, one module per topic of the report.
mod battery;
mod faults;
mod history;
mod identity;
mod mileage;
#[cfg(test)]
mod tests;

use battery::check_battery;
use faults::{check_faults, check_health};
use history::{check_clearing, check_readiness};
use identity::check_identity;
use mileage::check_mileage;

// --- Input -------------------------------------------------------------------

/// What the VIN says, decoded and matched against the catalog by the caller
/// (caruspex-vehicles decode_vin and match_vin).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct VinFacts {
    /// Normalised, as the control unit reported it.
    pub vin: String,
    /// 17 characters, none of I, O, Q.
    pub valid: bool,
    /// The ninth character is a check digit (North America, China).
    pub check_digit_applies: bool,
    pub check_digit_ok: bool,
    /// Empty if the maker code is unknown.
    pub brand: String,
    /// 0 = unknown.
    pub model_year: i32,
    /// Display name of the catalog generation, "Audi A2 (8Z)"; empty if none.
    pub vehicle: String,
    /// Production years of that generation; `year_to` 0 = still built.
    pub year_from: i32,
    pub year_to: i32,
}

/// The odometer as one control unit stores it.
#[derive(Clone, Debug, PartialEq)]
pub struct MileageReading {
    /// Where it comes from, in the active language ("Engine (OBD, PID A6)").
    pub source: String,
    pub km: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FaultKind {
    /// Confirmed (mode 03).
    #[default]
    Stored,
    /// Seen once, not confirmed yet (mode 07).
    Pending,
    /// Survives clearing until the car has seen the repair (mode 0A).
    Permanent,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UsedCarFault {
    pub code: String,
    /// Meaning in the active language, may be empty.
    pub text: String,
    pub kind: FaultKind,
    /// How serious (from the fault database).
    pub level: Level,
}

/// How the car is driven, from the catalog and from PID 51.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Drive {
    #[default]
    Unknown,
    Combustion,
    Hybrid,
    Electric,
}

impl Drive {
    /// Has a traction battery.
    pub fn electric(self) -> bool {
        matches!(self, Drive::Hybrid | Drive::Electric)
    }
}

/// Drive from the fuel type of PID 51 (SAE J1979 coding); Unknown for codes
/// that say nothing about it.
pub fn drive_from_fuel_type(code: i32) -> Drive {
    match code {
        8 => Drive::Electric,
        // bifuel with electricity, hybrids of every kind
        0x0F..=0x16 => Drive::Hybrid,
        1..=7 | 9..=0x0E | 0x17 => Drive::Combustion,
        _ => Drive::Unknown,
    }
}

/// The traction battery, read from a documented pack of manufacturer values.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BatteryFacts {
    /// Title of the pack in the active language.
    pub pack: String,
    /// The battery control unit answered.
    pub read: bool,
    /// Why not, if it did not.
    pub error: String,
    /// State of health in %.
    pub soh: Option<f64>,
    /// State of charge in %.
    pub soc: Option<f64>,
    /// Highest minus lowest cell voltage in V.
    pub cell_spread_v: Option<f64>,
}

/// The result of the short health check that runs along.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HealthSummary {
    pub score: i32,
    pub grade: TrafficLight,
    /// In the active language.
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UsedCarInput {
    /// The VIN of the engine control unit (mode 09); `None` if it gave none.
    pub car_vin: Option<VinFacts>,
    /// The VIN from the registration papers as typed; empty if not entered.
    pub papers_vin: String,
    /// PID A6, the odometer of the engine control unit.
    pub odometer_km: Option<f64>,
    /// The odometer of other control units (documented identifiers only).
    pub other_mileage: Vec<MileageReading>,
    /// PID 30.
    pub warm_ups: Option<f64>,
    /// PID 31.
    pub distance_since_clear_km: Option<f64>,
    /// PID 4E.
    pub minutes_since_clear: Option<f64>,
    /// PID 21.
    pub mil_distance_km: Option<f64>,
    pub mil_on: Option<bool>,
    /// Supported monitors only.
    pub readiness: Vec<MonitorState>,
    pub readiness_read: bool,
    pub faults: Vec<UsedCarFault>,
    pub faults_read: bool,
    pub health: Option<HealthSummary>,
    pub drive: Drive,
    /// `None`: no documented pack for this model.
    pub battery: Option<BatteryFacts>,
    /// For the mileage per year.
    pub current_year: i32,
}

// --- Output ------------------------------------------------------------------

/// The part of the check a finding belongs to, in the order of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Section {
    #[default]
    Identity,
    /// The control units of a truck and how they identify themselves.
    Units,
    Mileage,
    /// Engine hours, idle share and fuel of a truck.
    Usage,
    Clearing,
    Readiness,
    Faults,
    Health,
    Battery,
}

impl Section {
    pub const ALL: [Section; 9] = [
        Section::Identity,
        Section::Units,
        Section::Mileage,
        Section::Usage,
        Section::Clearing,
        Section::Readiness,
        Section::Faults,
        Section::Health,
        Section::Battery,
    ];

    /// Stable key for the interface ("identity", "mileage", ...).
    pub fn key(self) -> &'static str {
        match self {
            Section::Identity => "identity",
            Section::Units => "units",
            Section::Mileage => "mileage",
            Section::Usage => "usage",
            Section::Clearing => "clearing",
            Section::Readiness => "readiness",
            Section::Faults => "faults",
            Section::Health => "health",
            Section::Battery => "battery",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// Stable rule id, e.g. "mileage_mismatch".
    pub id: &'static str,
    pub section: Section,
    pub level: Level,
    pub title: Message,
    pub explanation: Message,
    /// What to ask the seller; empty if nothing.
    pub ask: Message,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NotChecked {
    pub section: Section,
    pub title: Message,
    pub reason: Message,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsedCarReport {
    pub light: TrafficLight,
    pub headline: Message,
    pub summary: Message,
    /// In the order of [`Section`].
    pub items: Vec<Item>,
    pub not_checked: Vec<NotChecked>,
    /// The odometer readings that were compared, engine first.
    pub mileage: Vec<MileageReading>,
}

impl UsedCarReport {
    pub fn find(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    /// The questions for the seller, the most serious first.
    pub fn questions(&self) -> Vec<&Message> {
        let mut asked: Vec<&Item> = self.items.iter().filter(|i| !i.ask.is_empty()).collect();
        asked.sort_by_key(|a| std::cmp::Reverse(a.level));
        asked.into_iter().map(|i| &i.ask).collect()
    }
}

/// Thresholds of the rules.
pub mod limits {
    /// Fewer warm-ups than this since clearing: cleared recently.
    pub const RECENT_WARM_UPS: f64 = 5.0;
    /// Less distance since clearing: cleared recently.
    pub const RECENT_DISTANCE_KM: f64 = 50.0;
    /// Less time since clearing: cleared recently.
    pub const RECENT_MINUTES: f64 = 180.0;
    /// Control units store the odometer at different moments and in steps;
    /// this much difference is normal.
    pub const MILEAGE_TOLERANCE_KM: f64 = 100.0;
    /// Or this share of the reading, whichever is larger.
    pub const MILEAGE_TOLERANCE_SHARE: f64 = 0.01;
    /// Below this per year (cars of three years and more): remarkably little.
    pub const LOW_KM_PER_YEAR: f64 = 3000.0;
    /// Above this per year: a lot.
    pub const HIGH_KM_PER_YEAR: f64 = 40000.0;
    /// State of health in %.
    pub const SOH_WORN: f64 = 80.0;
    /// Many makers guarantee this much for eight years.
    pub const SOH_WARRANTY: f64 = 70.0;
    /// V between the highest and the lowest cell.
    pub const CELL_SPREAD_V: f64 = 0.08;
}

// --- Rules -------------------------------------------------------------------

/// Collects the findings while the rules run; the used truck check uses it
/// too, its report has the same shape.
pub(crate) struct Ctx {
    pub(crate) items: Vec<Item>,
    pub(crate) not_checked: Vec<NotChecked>,
}

impl Ctx {
    pub(crate) fn add(
        &mut self,
        id: &'static str,
        section: Section,
        level: Level,
        title: Message,
        explanation: Message,
        ask: Message,
    ) {
        self.items.push(Item {
            id,
            section,
            level,
            title,
            explanation,
            ask,
        });
    }

    pub(crate) fn skip(&mut self, section: Section, title: Message, reason: Message) {
        self.not_checked.push(NotChecked {
            section,
            title,
            reason,
        });
    }
}

/// Upper case without the separators people type along.
pub fn normalise_vin(input: &str) -> String {
    input
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '.' | '\t' | '*' | '\r' | '\n'))
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// 17 letters and digits without I, O and Q.
pub fn vin_well_formed(vin: &str) -> bool {
    vin.len() == 17
        && vin
            .chars()
            .all(|c| c.is_ascii_alphanumeric() && !matches!(c, 'I' | 'O' | 'Q'))
}

/// The odometer readings to compare: the engine (PID A6) first, then the
/// other control units.
pub fn mileage_readings(input: &UsedCarInput) -> Vec<MileageReading> {
    let mut out = Vec::new();
    if let Some(km) = input.odometer_km {
        out.push(MileageReading {
            source: l("Motorsteuergerät (OBD)", "Engine control unit (OBD)").to_string(),
            km,
        });
    }
    out.extend(input.other_mileage.iter().cloned());
    out
}

/// Runs every rule and sums up.
pub fn evaluate_used_car(input: &UsedCarInput) -> UsedCarReport {
    let mut c = Ctx {
        items: Vec::new(),
        not_checked: Vec::new(),
    };
    let readings = mileage_readings(input);
    check_identity(&mut c, input);
    check_mileage(&mut c, input, &readings);
    check_clearing(&mut c, input);
    check_readiness(&mut c, input);
    check_faults(&mut c, input);
    check_health(&mut c, input);
    check_battery(&mut c, input);
    // Stable: the rules add in section order already, this keeps it so.
    c.items.sort_by_key(|i| i.section);

    let count = |level: Level| c.items.iter().filter(|i| i.level == level).count() as f64;
    let critical = count(Level::Critical);
    let warnings = count(Level::Warning);
    let (light, headline, summary) = if critical > 0.0 {
        (
            TrafficLight::Red,
            text(
                "Vorsicht: vor dem Kauf klären",
                "Careful: clear up before buying",
            ),
            text(
                "Punkte, die gegen einen Kauf ohne weitere Prüfung sprechen: %1. Die Fragen unten zuerst mit dem Verkäufer klären und das Auto in einer Werkstatt ansehen lassen.",
                "Points that speak against buying without a closer look: %1. Clear up the questions below with the seller first and have the car looked at by a workshop.",
            )
            .num(critical, 0),
        )
    } else if warnings > 0.0 {
        (
            TrafficLight::Yellow,
            text("Mit Fragen kaufen", "Buy with questions"),
            text(
                "Punkte, die Sie mit dem Verkäufer besprechen sollten: %1. Nichts davon schließt einen Kauf aus, aber es gehört in den Preis oder in den Kaufvertrag.",
                "Points to talk through with the seller: %1. None of them rules out a purchase, but they belong in the price or in the contract.",
            )
            .num(warnings, 0),
        )
    } else {
        (
            TrafficLight::Green,
            text("Unauffällig", "Nothing unusual"),
            text(
                "Die Steuergeräte zeigen nichts, was gegen das Auto spricht. Eine Probefahrt und ein Blick in die Wartungsnachweise ersetzt das nicht.",
                "The control units show nothing that speaks against the car. That does not replace a test drive and a look at the service records.",
            ),
        )
    };
    UsedCarReport {
        light,
        headline,
        summary,
        items: c.items,
        not_checked: c.not_checked,
        mileage: readings,
    }
}
