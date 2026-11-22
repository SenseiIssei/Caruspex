//! Coolant and oil: temperatures and pressures.

use super::{Ctx, limits, none, spn};
use crate::common::Level;
use crate::message::text;
use crate::series::{latest, max_of, min_of};

pub(super) fn check_coolant(c: &mut Ctx) {
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

pub(super) fn check_oil(c: &mut Ctx) {
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
