//! Clearing, lamps and faults per unit, the short health check and what no
//! unit tells.

use super::{TruckUsedInput, limits};
use crate::common::{Level, TrafficLight};
use crate::message::{Message, none, raw, text};
use crate::truck_health::{TruckFault, TruckUnit};
use crate::used_car::{Ctx, Section};

pub(super) fn check_clearing(c: &mut Ctx, input: &TruckUsedInput) {
    let h = input.lamp_history;
    let mut facts = Vec::new();
    if let Some(km) = h.since_clear_km {
        facts.push(text("%1 km", "%1 km").num(km, 0));
    }
    if let Some(minutes) = h.since_clear_minutes {
        facts.push(text("%1 Stunden Betrieb", "%1 hours of operation").num(minutes / 60.0, 0));
    }
    if facts.is_empty() {
        c.skip(
            Section::Clearing,
            text("Zuletzt gelöscht", "Last cleared"),
            text(
                "Der Lkw beantwortet DM21 nicht und meldet so nicht, wann der Fehlerspeicher zuletzt gelöscht wurde.",
                "The truck does not answer DM21, so it does not tell when the fault memory was last cleared.",
            ),
        );
    } else {
        let recent = h.since_clear_km.is_some_and(|k| k < limits::RECENT_KM)
            || h.since_clear_minutes
                .is_some_and(|m| m < limits::RECENT_MINUTES);
        if recent {
            c.add(
                "cleared_recently",
                Section::Clearing,
                Level::Warning,
                text(
                    "Fehlerspeicher vor Kurzem gelöscht",
                    "Fault memory cleared recently",
                ),
                text(
                    "Seit dem letzten Löschen: %1. Wer löscht, schaltet auch die Warnlampen aus; Fehler, die erst nach einigen Fahrten wiederkommen, sind bei der Besichtigung dann nicht zu sehen. Es gibt harmlose Gründe, etwa eine Reparatur oder eine Inspektion.",
                    "Since the last clearing: %1. Clearing also switches off the warning lamps; faults that only come back after a few drives are then not visible at the viewing. There are harmless reasons too, such as a repair or a service.",
                )
                .list(facts),
                text(
                    "Wann und warum wurde der Fehlerspeicher zuletzt gelöscht? Gibt es dazu eine Rechnung?",
                    "When and why was the fault memory last cleared? Is there an invoice for it?",
                ),
            );
        } else {
            c.add(
                "not_cleared_recently",
                Section::Clearing,
                Level::Good,
                text("Nicht frisch gelöscht", "Not freshly cleared"),
                text(
                    "Seit dem letzten Löschen: %1. Kein Hinweis darauf, dass der Fehlerspeicher kurz vor der Besichtigung gelöscht wurde.",
                    "Since the last clearing: %1. No sign that the fault memory was cleared shortly before the viewing.",
                )
                .list(facts),
                none(),
            );
        }
    }
    if let Some(km) = h.mil_km.filter(|k| *k > 0.0) {
        c.add(
            "mil_distance",
            Section::Clearing,
            Level::Info,
            text(
                "Mit Abgaslampe gefahren: %1 km",
                "Driven with the emission lamp on: %1 km",
            )
            .num(km, 0),
            text(
                "Seit dem letzten Löschen ist der Lkw %1 km mit leuchtender Abgaslampe gefahren. J1939 führt keinen Verlauf der roten und gelben Lampe; nur diese Strecke und die Zeit mit Abgaslampe bleiben gespeichert.",
                "Since the last clearing the truck has been driven %1 km with the emission lamp on. J1939 keeps no history of the red and amber lamps; only this distance and the time with the emission lamp are stored.",
            )
            .num(km, 0),
            text(
                "Warum wurde mit leuchtender Abgaslampe weitergefahren?",
                "Why was the truck driven on with the emission lamp on?",
            ),
        );
    }
}

fn fault_lines(unit: &TruckUnit, faults: &[TruckFault]) -> Message {
    let lines: Vec<Message> = faults
        .iter()
        .map(|f| {
            let base = if f.text.is_empty() {
                raw(f.code())
            } else {
                text("%1 (%2)", "%1 (%2)")
                    .plain(f.code())
                    .plain(f.text.clone())
            };
            match f.occurrences {
                Some(n) if n > 1 => text("%1, %2-mal", "%1, %2 times")
                    .list(vec![base])
                    .num(f64::from(n), 0),
                _ => base,
            }
        })
        .collect();
    text("%1: %2", "%1: %2")
        .plain(unit.label.clone())
        .list(lines)
}

pub(super) fn check_faults(c: &mut Ctx, input: &TruckUsedInput) {
    if !input.faults_read {
        c.skip(
            Section::Faults,
            text("Fehlerspeicher", "Fault memory"),
            text(
                "Der Fehlerspeicher ließ sich nicht lesen.",
                "The fault memory could not be read.",
            ),
        );
        return;
    }
    let red: Vec<Message> = input
        .faults
        .iter()
        .filter(|u| u.lamps.red_stop)
        .map(|u| raw(u.label.clone()))
        .collect();
    if !red.is_empty() {
        c.add(
            "lamp_red_stop",
            Section::Faults,
            Level::Critical,
            text("Rote Stopplampe an", "Red stop lamp on"),
            text(
                "Die rote Stopplampe leuchtet bei: %1. Mit so einem Fehler sollte der Lkw nicht weiterfahren.",
                "The red stop lamp is lit for: %1. With such a fault the truck should not drive on.",
            )
            .list(red),
            text(
                "Was ist die Ursache der roten Stopplampe, und wer zahlt die Reparatur?",
                "What is the cause of the red stop lamp, and who pays for the repair?",
            ),
        );
    }
    let active: Vec<&TruckUnit> = input
        .faults
        .iter()
        .filter(|u| !u.active.is_empty())
        .collect();
    if active.is_empty() {
        c.add(
            "no_active_faults",
            Section::Faults,
            Level::Good,
            text("Keine aktiven Fehler", "No active faults"),
            text(
                "Kein Steuergerät meldet gerade einen aktiven Fehler.",
                "No control unit reports an active fault right now.",
            ),
            none(),
        );
    } else {
        let lit = active
            .iter()
            .any(|u| u.lamps.amber || u.lamps.mil || u.lamps.red_stop);
        let count: usize = active.iter().map(|u| u.active.len()).sum();
        c.add(
            "faults_active",
            Section::Faults,
            if lit { Level::Warning } else { Level::Info },
            text("Aktive Fehler: %1", "Active faults: %1").num(count as f64, 0),
            text(
                "%1. Aktive Fehler liegen jetzt an; sie gehen erst weg, wenn die Ursache behoben ist.",
                "%1. Active faults are present now; they only go away once the cause is fixed.",
            )
            .list(active.iter().map(|u| fault_lines(u, &u.active)).collect()),
            text(
                "Was ist die Ursache der aktiven Fehler, und was würde die Reparatur kosten?",
                "What is the cause of the active faults, and what would the repair cost?",
            ),
        );
    }

    if !input.previous_read {
        c.skip(
            Section::Faults,
            text("Frühere Fehler", "Previous faults"),
            text(
                "Kein Steuergerät hat seine früheren Fehler (DM2) genannt.",
                "No control unit gave its previous faults (DM2).",
            ),
        );
        return;
    }
    let previous: Vec<&TruckUnit> = input
        .faults
        .iter()
        .filter(|u| !u.previous.is_empty())
        .collect();
    if previous.is_empty() {
        c.add(
            "no_previous_faults",
            Section::Faults,
            Level::Good,
            text("Keine früheren Fehler", "No previous faults"),
            text(
                "Kein Steuergerät hat frühere Fehler gespeichert. Das ist gut, kann aber auch ein frisch gelöschter Speicher sein.",
                "No control unit has previous faults stored. That is good, but it can also be a freshly cleared memory.",
            ),
            none(),
        );
        return;
    }
    let count: usize = previous.iter().map(|u| u.previous.len()).sum();
    let recurring = previous.iter().any(|u| {
        u.previous.len() >= limits::MANY_PREVIOUS
            || u.previous
                .iter()
                .any(|f| f.occurrences.is_some_and(|n| n >= limits::RECURRING))
    });
    c.add(
        "faults_previous",
        Section::Faults,
        if recurring { Level::Warning } else { Level::Info },
        text("Frühere Fehler: %1", "Previous faults: %1").num(count as f64, 0),
        text(
            "%1. Diese Fehler waren aktiv und sind es nicht mehr. Kommt einer immer wieder, steckt oft ein Kabel- oder Steckerproblem dahinter.",
            "%1. These faults were active and are no longer. If one keeps coming back, a wiring or plug problem is often behind it.",
        )
        .list(previous.iter().map(|u| fault_lines(u, &u.previous)).collect()),
        if recurring {
            text(
                "Welche Fehler kamen immer wieder, und was wurde dagegen gemacht?",
                "Which faults kept coming back, and what was done about them?",
            )
        } else {
            none()
        },
    );
}

pub(super) fn check_health(c: &mut Ctx, input: &TruckUsedInput) {
    let Some(health) = &input.health else {
        return;
    };
    let level = match health.grade {
        TrafficLight::Green => Level::Good,
        TrafficLight::Yellow => Level::Info,
        TrafficLight::Red => Level::Warning,
    };
    c.add(
        "health",
        Section::Health,
        level,
        text(
            "Gesundheitscheck: %1 von 100 Punkten",
            "Health check: %1 of 100 points",
        )
        .num(f64::from(health.score), 0),
        raw(health.summary.clone()),
        none(),
    );
}

/// What no control unit tells, so the buyer knows where to look further.
pub(super) fn blind_spots(c: &mut Ctx) {
    c.skip(
        Section::Mileage,
        text("Speicher des Tachographen", "Tachograph memory"),
        text(
            "Der Kilometerverlauf im Massenspeicher des Tachographen ist nur mit Unternehmens- oder Werkstattkarte lesbar, nicht über J1939.",
            "The mileage history in the tachograph's mass memory can only be read with a company or workshop card, not over J1939.",
        ),
    );
    c.skip(
        Section::Faults,
        text("Herstellerspeicher", "Maker specific memory"),
        text(
            "Herstellereigene Zähler (Bremsbeläge, Kupplung, Getriebe, Retarder) und Fehlerspeicher außerhalb von J1939 liest diese Prüfung nicht.",
            "Maker specific counters (brake pads, clutch, gearbox, retarder) and fault memories outside J1939 are not read by this check.",
        ),
    );
    c.skip(
        Section::Health,
        text("Mechanischer Zustand", "Mechanical condition"),
        text(
            "Rahmen, Achsen, Bremsen, Reifen, Aufbau und Unfallschäden sieht kein Steuergerät.",
            "No control unit sees the frame, axles, brakes, tyres, body or accident damage.",
        ),
    );
}
