//! The odometers of every source against each other, and how the truck was
//! used (hours, idle, fuel).

use super::{
    OdometerReading, SPN_DASHBOARD_DISTANCE, TruckUsedFacts, TruckUsedInput, ask_records, limits,
};
use crate::common::Level;
use crate::message::{Message, none, text};
use crate::used_car::{Ctx, Section};

pub(super) fn source_name(r: &OdometerReading) -> Message {
    if r.spn == SPN_DASHBOARD_DISTANCE {
        text("%1, Gesamtstrecke", "%1, total distance").plain(r.label.clone())
    } else {
        text("%1, Gesamtstrecke fein", "%1, fine total distance").plain(r.label.clone())
    }
}

fn mileage_tolerance(km: f64) -> f64 {
    (km * limits::MILEAGE_TOLERANCE_SHARE).max(limits::MILEAGE_TOLERANCE_KM)
}

pub(super) fn check_mileage(c: &mut Ctx, input: &TruckUsedInput) -> Option<f64> {
    let readings = &input.odometers;
    if readings.is_empty() {
        c.add(
            "mileage_unknown",
            Section::Mileage,
            Level::Info,
            text("Kilometerstand nicht auslesbar", "Mileage cannot be read"),
            text(
                "Kein Steuergerät sendet eine Gesamtstrecke (SPN 245 oder 917). Den Tachostand mit Wartungsheft, Prüfberichten und Rechnungen vergleichen.",
                "No control unit sends a total distance (SPN 245 or 917). Compare the odometer with the service records, inspection reports and invoices.",
            ),
            ask_records(),
        );
        return None;
    }
    let highest = readings.iter().map(|r| r.km).fold(f64::MIN, f64::max);
    let lowest = readings.iter().map(|r| r.km).fold(f64::MAX, f64::min);
    let list: Vec<Message> = readings
        .iter()
        .map(|r| {
            text("%1: %2 km", "%1: %2 km")
                .list(vec![source_name(r)])
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
                "Nur eine Quelle: %1. Ein Vergleich zwischen Steuergeräten ist nicht möglich; der Wert muss zum Tacho und zu den Belegen passen.",
                "Only one source: %1. A comparison between control units is not possible; the value must match the dashboard and the records.",
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
                "Die Steuergeräte zählen verschieden: %1. Der Unterschied beträgt %2 km. Ein wenig ist normal, so viel nicht. Das Kombiinstrument lässt sich tauschen oder umprogrammieren, Motorsteuergerät und Tachograph zählen getrennt davon; meist stimmt der höhere Wert.",
                "The control units count differently: %1. The difference is %2 km. A little is normal, this much is not. The instrument cluster can be replaced or reprogrammed, the engine control unit and the tachograph count separately from it; usually the higher value is the true one.",
            )
            .list(list)
            .num(highest - lowest, 0),
            text(
                "Warum zeigen die Steuergeräte verschiedene Kilometerstände? Wurde das Kombiinstrument, das Motorsteuergerät oder der Tachograph getauscht, und gibt es dafür einen Beleg?",
                "Why do the control units show different mileages? Was the instrument cluster, the engine control unit or the tachograph replaced, and is there proof of it?",
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
                "Alle Quellen stimmen überein: %1. Ein gutes Zeichen, aber kein Beweis: wer alle Steuergeräte gleich verstellt, fällt hier nicht auf.",
                "All sources agree: %1. A good sign, but no proof: someone who alters every control unit alike does not show up here.",
            )
            .list(list),
            ask_records(),
        );
    }
    Some(highest)
}

pub(super) fn check_usage(c: &mut Ctx, input: &TruckUsedInput, km: Option<f64>) -> TruckUsedFacts {
    let mut facts = TruckUsedFacts {
        odometer_km: km,
        engine_hours: input.engine_hours,
        ..TruckUsedFacts::default()
    };
    let Some(hours) = input.engine_hours.filter(|h| *h > 0.0) else {
        c.skip(
            Section::Usage,
            text("Motorbetriebsstunden", "Engine hours"),
            text(
                "Der Motor hat auf die Anfrage nach den Betriebsstunden (SPN 247) nicht geantwortet.",
                "The engine did not answer the request for its hours (SPN 247).",
            ),
        );
        return facts;
    };
    c.add(
        "engine_hours",
        Section::Usage,
        Level::Good,
        text("%1 Motorbetriebsstunden", "%1 engine hours").num(hours, 0),
        text(
            "Der Zähler steckt im Motorsteuergerät. Wird es getauscht, beginnt er neu oder bekommt einen Wert eingetragen; mit dem Wartungsheft vergleichen.",
            "The counter sits in the engine control unit. If it is replaced, it starts again or gets a value written in; compare with the service records.",
        ),
        none(),
    );

    if let Some(km) = km {
        let speed = km / hours;
        facts.average_speed = Some(speed);
        let (id, level, title, explanation, ask) = if speed > limits::SPEED_TOO_HIGH {
            (
                "hours_too_few",
                Level::Warning,
                text(
                    "Zu wenige Betriebsstunden für die Kilometer",
                    "Too few engine hours for the kilometres",
                ),
                text(
                    "Im Schnitt %1 km je Betriebsstunde. Das schafft kein Lkw, Stopps und Leerlauf eingerechnet; üblich sind 20 bis 60 km/h. Vermutlich wurde das Motorsteuergerät oder der Motor getauscht.",
                    "On average %1 km per engine hour. No truck manages that with stops and idling counted; 20 to 60 km/h is usual. Probably the engine control unit or the engine was replaced.",
                ),
                text(
                    "Wurde der Motor oder das Motorsteuergerät getauscht, und bei welchem Kilometerstand?",
                    "Was the engine or the engine control unit replaced, and at what mileage?",
                ),
            )
        } else if speed < limits::SPEED_TOO_LOW {
            (
                "km_too_few",
                Level::Warning,
                text(
                    "Wenige Kilometer für die Betriebsstunden",
                    "Few kilometres for the engine hours",
                ),
                text(
                    "Im Schnitt nur %1 km je Betriebsstunde. Das passt zu Arbeit im Stand, etwa mit Kran, Pumpe oder Müllaufbau, aber auch zu einem zurückgestellten Kilometerstand.",
                    "On average only %1 km per engine hour. That fits work while standing, such as with a crane, a pump or a refuse body, but also a turned back odometer.",
                ),
                text(
                    "Wofür wurde das Fahrzeug eingesetzt, und lief der Motor viel im Stand?",
                    "What was the vehicle used for, and did the engine run a lot while standing?",
                ),
            )
        } else {
            (
                "speed_plausible",
                Level::Good,
                text(
                    "Kilometer passen zu den Betriebsstunden",
                    "Kilometres fit the engine hours",
                ),
                text(
                    "Im Schnitt %1 km je Betriebsstunde, Stopps und Leerlauf eingerechnet. Fernverkehr liegt meist bei 45 bis 60, Verteilerverkehr bei 25 bis 40, Stadtbusse bei 15 bis 20.",
                    "On average %1 km per engine hour, stops and idling included. Long haul is mostly 45 to 60, distribution 25 to 40, city buses 15 to 20.",
                ),
                none(),
            )
        };
        c.add(
            id,
            Section::Usage,
            level,
            title,
            explanation.num(speed, 1),
            ask,
        );
    }

    if let Some(idle) = input.idle_hours.filter(|i| *i >= 0.0 && *i <= hours) {
        let share = idle / hours;
        facts.idle_share = Some(share);
        let level = if share > limits::IDLE_HIGH {
            Level::Warning
        } else if share > limits::IDLE_NOTABLE {
            Level::Info
        } else {
            Level::Good
        };
        c.add(
            "idle_share",
            Section::Usage,
            level,
            text("Leerlaufanteil %1 %", "Idle share %1 %").num(share * 100.0, 0),
            text(
                "%1 von %2 Betriebsstunden lief der Motor im Leerlauf. Leerlauf verschleißt den Motor ohne Kilometer, lässt den Partikelfilter schneller verrußen und verdünnt das Öl mit Diesel. Bis etwa ein Viertel ist üblich, Standheizung über den Motor treibt den Wert hoch.",
                "The engine idled for %1 of %2 engine hours. Idling wears the engine without kilometres, fills the particulate filter with soot faster and thins the oil with diesel. Up to about a quarter is usual; heating the cab with the engine pushes the share up.",
            )
            .num(idle, 0)
            .num(hours, 0),
            if level == Level::Good {
                none()
            } else {
                text(
                    "Warum lief der Motor so viel im Leerlauf, und wurde das Öl nach Stunden gewechselt?",
                    "Why did the engine idle so much, and was the oil changed by hours?",
                )
            },
        );
    } else {
        c.skip(
            Section::Usage,
            text("Leerlaufstunden", "Idle hours"),
            text(
                "Der Motor meldet keine Leerlaufstunden (SPN 235).",
                "The engine reports no idle hours (SPN 235).",
            ),
        );
    }

    match (input.fuel_total_l, km) {
        (Some(fuel), Some(km)) if km > 1000.0 && fuel > 0.0 => {
            let per_100 = fuel / km * 100.0;
            facts.fuel_per_100km = Some(per_100);
            let odd = !(limits::FUEL_LOW..=limits::FUEL_HIGH).contains(&per_100);
            c.add(
                "fuel_average",
                Section::Usage,
                if odd { Level::Warning } else { Level::Good },
                if odd {
                    text(
                        "Verbrauch passt nicht zum Kilometerstand: %1 l/100 km",
                        "Fuel does not fit the mileage: %1 l/100 km",
                    )
                } else {
                    text(
                        "Durchschnittsverbrauch %1 l/100 km",
                        "Average fuel use %1 l/100 km",
                    )
                }
                .num(per_100, 1),
                text(
                    "%1 Liter seit dem Bau des Motors auf %2 km. Schwere Lkw und Busse brauchen meist 25 bis 40 l/100 km. Der Zähler steckt im Motorsteuergerät; weicht der Schnitt stark ab, passen Motor und Kilometerstand nicht zusammen.",
                    "%1 litres since the engine was built over %2 km. Heavy trucks and buses mostly use 25 to 40 l/100 km. The counter sits in the engine control unit; if the average is far off, engine and mileage do not belong together.",
                )
                .num(fuel, 0)
                .num(km, 0),
                if odd {
                    text(
                        "Wurde der Motor oder das Motorsteuergerät getauscht, oder der Kilometerstand verändert?",
                        "Was the engine or the engine control unit replaced, or the mileage altered?",
                    )
                } else {
                    none()
                },
            );
        }
        _ => c.skip(
            Section::Usage,
            text("Kraftstoff gesamt", "Total fuel used"),
            text(
                "Ohne Gesamtverbrauch (SPN 250) und Kilometerstand lässt sich der Durchschnitt nicht rechnen.",
                "Without the total fuel used (SPN 250) and the mileage the average cannot be worked out.",
            ),
        ),
    }
    facts
}
