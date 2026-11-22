//! Active and previous faults of every unit, and the lamps.

use super::{Ctx, TruckFault, TruckLamps, TruckUnit};
use crate::common::Level;
use crate::message::{Message, none, raw, text};

pub(super) fn check_faults(c: &mut Ctx) {
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

pub(super) fn check_lamps(c: &mut Ctx) {
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
