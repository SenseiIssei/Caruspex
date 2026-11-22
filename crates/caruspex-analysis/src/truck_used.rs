//! Used vehicle check for trucks and buses over SAE J1939: for someone
//! standing next to a truck they might buy.
//!
//! The caller asks every unit what J1939 offers on request and hands it in:
//! the VIN of every unit that keeps one, make, model, serial and software of
//! every unit, the odometer of every source (dashboard SPN 245, the fine
//! odometer SPN 917 of engine and tachograph), engine hours, idle hours,
//! total fuel, DM21 (distance with the lamp on and since the last clearing),
//! the active and previous faults per unit and a short health check. This
//! module compares them and answers with the same report as the car check:
//! a traffic light, the findings in plain words and the questions for the
//! seller.
//!
//! The texts say plainly what a dealer could have changed (an instrument
//! cluster can be swapped or reprogrammed, a fault memory cleared) and what
//! no control unit tells (the tachograph's own memory, maker specific
//! counters, the state of frame, brakes and tyres).

use crate::common::{Level, TrafficLight};
use crate::message::{Message, text};
use crate::truck_health::TruckUnit;
use crate::used_car::{Ctx, HealthSummary, MileageReading, UsedCarReport};

// The rules, one module per topic of the report.
mod faults;
mod identity;
mod mileage;
#[cfg(test)]
mod tests;

use faults::{blind_spots, check_clearing, check_faults, check_health};
use identity::{check_identity, check_units};
use mileage::{check_mileage, check_usage, source_name};

/// The SPNs of the odometers that are compared.
pub const SPN_DASHBOARD_DISTANCE: u32 = 245;
pub const SPN_FINE_DISTANCE: u32 = 917;

/// Thresholds of the rules.
pub mod limits {
    /// Units store the odometer at different moments, and engine and
    /// tachograph may be calibrated a little apart (tyre size); this much
    /// difference is normal.
    pub const MILEAGE_TOLERANCE_KM: f64 = 200.0;
    /// Or this share of the reading, whichever is larger.
    pub const MILEAGE_TOLERANCE_SHARE: f64 = 0.005;
    /// km/h over all engine hours. Above this the hours are too few for
    /// the distance: no truck averages more, stops and idling included.
    pub const SPEED_TOO_HIGH: f64 = 75.0;
    /// Below this the distance is very low for the hours: stationary work
    /// (crane, refuse, power take-off) or a turned back odometer.
    pub const SPEED_TOO_LOW: f64 = 8.0;
    /// Share of the engine hours at idle.
    pub const IDLE_HIGH: f64 = 0.40;
    pub const IDLE_NOTABLE: f64 = 0.25;
    /// l/100 km over the whole life outside this range does not fit a
    /// heavy truck or bus.
    pub const FUEL_LOW: f64 = 15.0;
    pub const FUEL_HIGH: f64 = 70.0;
    /// Less distance or time since clearing: cleared recently.
    pub const RECENT_KM: f64 = 50.0;
    pub const RECENT_MINUTES: f64 = 180.0;
    /// A fault that went active this often keeps coming back.
    pub const RECURRING: u8 = 10;
    /// This many previous faults in one unit are worth a question.
    pub const MANY_PREVIOUS: usize = 5;
}

/// The VIN one unit keeps.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct UnitVin {
    /// "Engine (00)", in the active language.
    pub label: String,
    pub vin: String,
}

/// How one unit identifies itself (component and software identification).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct UnitIdentity {
    pub label: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub software: Vec<String>,
}

/// One odometer reading: which unit, which SPN, how far.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct OdometerReading {
    pub label: String,
    pub spn: u32,
    pub km: f64,
}

/// DM21 of the engine; every field only if the unit sent it.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct LampHistory {
    /// SPN 3069.
    pub mil_km: Option<f64>,
    /// SPN 3294.
    pub since_clear_km: Option<f64>,
    /// SPN 3295.
    pub mil_minutes: Option<f64>,
    /// SPN 3296.
    pub since_clear_minutes: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TruckUsedInput {
    /// Every unit that answered the VIN request, address order.
    pub vins: Vec<UnitVin>,
    /// The VIN from the registration papers as typed; empty if not entered.
    pub papers_vin: String,
    /// Every unit heard on the bus, with what it says about itself; make
    /// and model stay empty for a unit that did not answer.
    pub units: Vec<UnitIdentity>,
    pub odometers: Vec<OdometerReading>,
    /// SPN 247.
    pub engine_hours: Option<f64>,
    /// SPN 235.
    pub idle_hours: Option<f64>,
    /// SPN 250, l.
    pub fuel_total_l: Option<f64>,
    /// SPN 236, l.
    pub idle_fuel_l: Option<f64>,
    pub lamp_history: LampHistory,
    /// Active (DM1) and previous (DM2) faults with the lamps per unit.
    pub faults: Vec<TruckUnit>,
    pub faults_read: bool,
    pub previous_read: bool,
    pub health: Option<HealthSummary>,
}

/// The numbers the page shows as tiles.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TruckUsedFacts {
    /// The highest odometer reading.
    pub odometer_km: Option<f64>,
    pub engine_hours: Option<f64>,
    /// 0..1.
    pub idle_share: Option<f64>,
    /// km per engine hour, idle included.
    pub average_speed: Option<f64>,
    /// l/100 km over the whole life.
    pub fuel_per_100km: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TruckUsedReport {
    pub report: UsedCarReport,
    pub facts: TruckUsedFacts,
}

fn ask_records() -> Message {
    text(
        "Gibt es Wartungsheft, Werkstattrechnungen und Prüfberichte (HU, SP), die Kilometer und Betriebsstunden belegen?",
        "Are there service records, workshop invoices and inspection reports that back up the kilometres and engine hours?",
    )
}

/// Runs every rule and sums up.
pub fn evaluate_used_truck(input: &TruckUsedInput) -> TruckUsedReport {
    let mut c = Ctx {
        items: Vec::new(),
        not_checked: Vec::new(),
    };
    check_identity(&mut c, input);
    check_units(&mut c, input);
    let km = check_mileage(&mut c, input);
    let facts = check_usage(&mut c, input, km);
    check_clearing(&mut c, input);
    check_faults(&mut c, input);
    check_health(&mut c, input);
    blind_spots(&mut c);
    c.items.sort_by_key(|i| i.section);
    c.not_checked.sort_by_key(|n| n.section);

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
                "Punkte, die gegen einen Kauf ohne weitere Prüfung sprechen: %1. Die Fragen zuerst mit dem Verkäufer klären und das Fahrzeug in einer Lkw-Werkstatt ansehen lassen.",
                "Points that speak against buying without a closer look: %1. Clear up the questions with the seller first and have the vehicle looked at by a truck workshop.",
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
                "Die Steuergeräte zeigen nichts, was gegen das Fahrzeug spricht. Probefahrt, Wartungsnachweise und ein Blick unter das Fahrzeug ersetzt das nicht.",
                "The control units show nothing that speaks against the vehicle. That does not replace a test drive, the service records and a look underneath.",
            ),
        )
    };
    let mileage = input
        .odometers
        .iter()
        .map(|r| MileageReading {
            source: source_name(r).text(),
            km: r.km,
        })
        .collect();
    TruckUsedReport {
        report: UsedCarReport {
            light,
            headline,
            summary,
            items: c.items,
            not_checked: c.not_checked,
            mileage,
        },
        facts,
    }
}
