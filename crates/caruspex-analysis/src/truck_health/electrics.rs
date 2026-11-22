//! Battery and charging voltage of 12 and 24 volt systems.

use super::{Ctx, limits, spn};
use crate::common::Level;
use crate::message::{none, text};
use crate::series::{max_of, median};

pub(super) fn check_voltage(c: &mut Ctx) {
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
