//! The air path (boost, intake) and the engine hours.

use super::{Ctx, limits, none, spn};
use crate::common::Level;
use crate::message::text;
use crate::series::{max_of, median};

pub(super) fn check_air(c: &mut Ctx) {
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

pub(super) fn check_hours(c: &mut Ctx) {
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
