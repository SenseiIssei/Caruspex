//! The odometers of the control units against each other.

use super::{Ctx, MileageReading, Section, UsedCarInput, limits};
use crate::common::Level;
use crate::message::{Message, none, text};

fn mileage_tolerance(km: f64) -> f64 {
    (km * limits::MILEAGE_TOLERANCE_SHARE).max(limits::MILEAGE_TOLERANCE_KM)
}

fn ask_records() -> Message {
    text(
        "Gibt es Serviceheft, Rechnungen und Prüfberichte, die den Kilometerstand belegen?",
        "Are there a service book, invoices and inspection reports that back up the mileage?",
    )
}

pub(super) fn check_mileage(c: &mut Ctx, input: &UsedCarInput, readings: &[MileageReading]) {
    if readings.is_empty() {
        c.add(
            "mileage_unknown",
            Section::Mileage,
            Level::Info,
            text(
                "Kilometerstand nicht auslesbar",
                "Mileage cannot be read",
            ),
            text(
                "Das Auto meldet seinen Kilometerstand nicht über OBD (PID A6 gibt es erst bei neueren Autos), und für dieses Modell sind keine weiteren Kennungen belegt. Den Tachostand mit Serviceheft, Prüfberichten und Rechnungen vergleichen.",
                "The car does not report its mileage over OBD (PID A6 only exists on newer cars), and no further identifiers are documented for this model. Compare the odometer with the service book, inspection reports and invoices.",
            ),
            ask_records(),
        );
        return;
    }

    let highest = readings.iter().map(|r| r.km).fold(f64::MIN, f64::max);
    let lowest = readings.iter().map(|r| r.km).fold(f64::MAX, f64::min);
    let list: Vec<Message> = readings
        .iter()
        .map(|r| {
            text("%1: %2 km", "%1: %2 km")
                .plain(r.source.clone())
                .num(r.km, 0)
        })
        .collect();

    if readings.len() == 1 {
        c.add(
            "mileage_single",
            Section::Mileage,
            Level::Info,
            text("Kilometerstand: %1 km", "Mileage: %1 km").num(highest, 0),
            text(
                "Nur eine Quelle: %1. Für dieses Modell sind keine weiteren Steuergeräte mit belegter Kennung bekannt, ein Vergleich ist also nicht möglich. Der Wert muss zum Tacho passen.",
                "Only one source: %1. No further control units with a documented identifier are known for this model, so no comparison is possible. The value must match the dashboard.",
            )
            .list(list),
            ask_records(),
        );
    } else if highest - lowest > mileage_tolerance(highest) {
        c.add(
            "mileage_mismatch",
            Section::Mileage,
            Level::Critical,
            text(
                "Kilometerstände passen nicht zusammen",
                "Mileage readings do not match",
            ),
            text(
                "Die Steuergeräte speichern verschiedene Werte: %1. Der Unterschied beträgt %2 km. Ein wenig ist normal, weil nicht jedes Gerät gleich oft speichert. So viel deutet aber darauf hin, dass der Tacho verstellt oder ein Steuergerät aus einem anderen Auto eingebaut wurde.",
                "The control units store different values: %1. The difference is %2 km. A little is normal, because not every unit stores equally often. This much points to an altered odometer or a control unit from another car.",
            )
            .list(list)
            .num(highest - lowest, 0),
            text(
                "Warum zeigen die Steuergeräte verschiedene Kilometerstände? Wurde der Tacho oder ein Steuergerät getauscht, und gibt es dafür einen Beleg?",
                "Why do the control units show different mileages? Was the instrument cluster or a control unit replaced, and is there proof of it?",
            ),
        );
    } else {
        c.add(
            "mileage_consistent",
            Section::Mileage,
            Level::Good,
            text(
                "Kilometerstände passen zusammen",
                "Mileage readings agree",
            ),
            text(
                "Alle Quellen stimmen überein: %1. Kleine Unterschiede sind normal, weil nicht jedes Gerät gleich oft speichert.",
                "All sources agree: %1. Small differences are normal, because not every unit stores equally often.",
            )
            .list(list),
            ask_records(),
        );
    }

    // More driven since clearing than the odometer shows: the engine
    // control unit has counted kilometres that the odometer does not have.
    if let Some(since) = input.distance_since_clear_km
        && since > highest + mileage_tolerance(highest)
    {
        c.add(
            "mileage_below_clear_distance",
            Section::Mileage,
            Level::Warning,
            text(
                "Kilometerstand kleiner als die Strecke seit dem Löschen",
                "Mileage lower than the distance since clearing",
            ),
            text(
                "Seit dem letzten Löschen hat das Motorsteuergerät %1 km gezählt, der Kilometerstand zeigt aber nur %2 km. Das geht nur, wenn der Tacho zurückgestellt oder ein Steuergerät getauscht wurde.",
                "Since the last clearing the engine control unit has counted %1 km, but the mileage shows only %2 km. That is only possible if the odometer was turned back or a control unit was replaced.",
            )
            .num(since, 0)
            .num(highest, 0),
            text(
                "Wurde der Tacho oder das Motorsteuergerät getauscht?",
                "Was the instrument cluster or the engine control unit replaced?",
            ),
        );
    }

    // Kilometres per year, if the age is known.
    let model_year = input.car_vin.as_ref().map_or(0, |v| v.model_year);
    if model_year > 0 && input.current_year >= model_year {
        let age = f64::from(input.current_year - model_year) + 0.5;
        let per_year = highest / age;
        if age >= 3.0 && per_year < limits::LOW_KM_PER_YEAR {
            c.add(
                "mileage_low_for_age",
                Section::Mileage,
                Level::Info,
                text(
                    "Auffallend wenig gefahren",
                    "Remarkably low mileage",
                ),
                text(
                    "Im Schnitt etwa %1 km pro Jahr. Das gibt es, etwa bei Zweitwagen, ist aber auch das, was ein zurückgedrehter Tacho zeigt.",
                    "On average about %1 km per year. That happens, for a second car for instance, but it is also what a turned back odometer shows.",
                )
                .num((per_year / 100.0).round() * 100.0, 0),
                ask_records(),
            );
        } else if per_year > limits::HIGH_KM_PER_YEAR {
            c.add(
                "mileage_high_for_age",
                Section::Mileage,
                Level::Info,
                text("Viel gefahren", "High mileage"),
                text(
                    "Im Schnitt etwa %1 km pro Jahr, meist Langstrecke. Das schont den Motor, aber Verschleißteile wie Kupplung, Bremsen und Fahrwerk sind entsprechend beansprucht.",
                    "On average about %1 km per year, mostly long distances. That is easy on the engine, but wear parts such as clutch, brakes and suspension have worked accordingly.",
                )
                .num((per_year / 100.0).round() * 100.0, 0),
                text(
                    "Wurden die Wartungen nach Plan gemacht, und wann wurden Kupplung, Bremsen und Zahnriemen zuletzt erneuert?",
                    "Was the car serviced to plan, and when were the clutch, brakes and timing belt last renewed?",
                ),
            );
        } else {
            c.add(
                "mileage_fits_age",
                Section::Mileage,
                Level::Good,
                text("Kilometerstand passt zum Alter", "Mileage fits the age"),
                text(
                    "Im Schnitt etwa %1 km pro Jahr, ein üblicher Wert.",
                    "On average about %1 km per year, a usual figure.",
                )
                .num((per_year / 100.0).round() * 100.0, 0),
                none(),
            );
        }
    }
}
