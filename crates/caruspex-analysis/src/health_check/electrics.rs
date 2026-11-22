//! Battery and charging voltage.

use super::{
    Area, Ctx, PID_RPM, PID_VOLTAGE, RUNNING_RPM, STOPPED_RPM, get, limits, median, nearest, none,
};
use crate::common::{Level, format, number};

pub(super) fn check_voltage(c: &mut Ctx) {
    let input = c.input;
    let charge_title = format("Ladespannung", "Charging voltage", &[]);
    let rest_title = format(
        "Batterie bei stehendem Motor",
        "Battery with engine off",
        &[],
    );
    let Some(volts) = get(input, PID_VOLTAGE) else {
        let why = format(
            "Die Spannung (PID 42) wurde nicht gelesen.",
            "The voltage (PID 42) was not read.",
            &[],
        );
        c.skip("charging", charge_title, why.clone());
        c.skip("battery_rest", rest_title, why);
        return;
    };
    let Some(rpm) = get(input, PID_RPM) else {
        let why = format(
            "Ohne Drehzahl ist nicht klar, ob der Motor lief.",
            "Without engine speed it is unclear whether the engine ran.",
            &[],
        );
        c.skip("charging", charge_title, why.clone());
        c.skip("battery_rest", rest_title, why);
        return;
    };
    let mut running: Vec<f64> = Vec::new();
    let mut off: Vec<f64> = Vec::new();
    for v in volts {
        let Some(r) = nearest(Some(rpm), v.t) else {
            continue;
        };
        if r > RUNNING_RPM {
            running.push(v.value);
        } else if r < STOPPED_RPM {
            off.push(v.value);
        }
    }

    let source_note = format(
        "Gemessen am Diagnosestecker; dort liegt die Spannung oft 0,1 bis 0,3 V unter der an \
         den Batteriepolen.",
        "Measured at the diagnostic socket, where the voltage is often 0.1 to 0.3 V below the \
         battery terminals.",
        &[],
    );

    if running.is_empty() {
        c.skip(
            "charging",
            charge_title,
            format(
                "Der Motor lief während der Messung nicht.",
                "The engine was not running during the measurement.",
                &[],
            ),
        );
    } else {
        c.ran();
        let v = median(running);
        let value = number(v, 2);
        let smart = format(
            "Autos mit Start-Stopp und intelligentem Lademanagement (oft mit AGM- oder \
             EFB-Batterie) senken die Ladespannung bei voller Batterie bewusst ab; dort kann \
             ein Wert um 12,5 bis 13 V normal sein.",
            "Cars with start-stop and smart charging (often with an AGM or EFB battery) \
             lower the charging voltage on purpose when the battery is full; there a value of \
             about 12.5 to 13 V can be normal.",
            &[],
        );
        if v < limits::CHARGING_NONE {
            c.add(
                "charging",
                Level::Critical,
                Area::Charging,
                format(
                    "Lichtmaschine lädt nicht",
                    "Alternator is not charging",
                    &[],
                ),
                format(
                    "Bei laufendem Motor liegen nur {} V an. Normal sind 13,8 bis 14,8 V. \
                     Die Batterie wird also nicht geladen, sondern leer gefahren.",
                    "With the engine running there are only {} V. Normal is 13.8 to \
                     14.8 V. So the battery is not being charged but drained.",
                    &[value],
                ) + smart
                    + source_note.clone(),
                format(
                    "Bald prüfen lassen: Lichtmaschine, Keilrippenriemen und Kabel. Bis \
                     dahin kann das Auto unterwegs liegen bleiben.",
                    "Have it checked soon: alternator, drive belt and cables. Until then the \
                     car may stop on the road.",
                    &[],
                ),
            );
        } else if v < limits::CHARGING_LOW {
            c.add(
                "charging",
                Level::Warning,
                Area::Charging,
                format("Ladespannung niedrig", "Charging voltage low", &[]),
                format(
                    "Bei laufendem Motor liegen {} V an. Normal sind 13,8 bis 14,8 V; \
                     unter 13,2 V lädt die Lichtmaschine meist zu schwach.",
                    "With the engine running there are {} V. Normal is 13.8 to 14.8 V; \
                     below 13.2 V the alternator usually charges too weakly.",
                    &[value],
                ) + smart
                    + source_note.clone(),
                format(
                    "Mit dem geführten Batterietest nachmessen, auch mit eingeschaltetem \
                     Licht und Gebläse.",
                    "Measure again with the guided battery test, also with the lights and \
                     blower switched on.",
                    &[],
                ),
            );
        } else if v > limits::CHARGING_TOO_HIGH {
            c.add(
                "charging",
                Level::Critical,
                Area::Charging,
                format(
                    "Ladespannung viel zu hoch",
                    "Charging voltage far too high",
                    &[],
                ),
                format(
                    "Bei laufendem Motor liegen {} V an. Über 15,5 V wird die Batterie \
                     überladen und Elektronik kann Schaden nehmen. Meist ist der Regler der \
                     Lichtmaschine defekt.",
                    "With the engine running there are {} V. Above 15.5 V the battery is \
                     overcharged and electronics can be damaged. Usually the alternator's \
                     regulator has failed.",
                    &[value],
                ),
                format(
                    "Bald in die Werkstatt.",
                    "Have it seen by a garage soon.",
                    &[],
                ),
            );
        } else if v > limits::CHARGING_HIGH {
            c.add(
                "charging",
                Level::Warning,
                Area::Charging,
                format("Ladespannung hoch", "Charging voltage high", &[]),
                format(
                    "Bei laufendem Motor liegen {} V an. Normal sind 13,8 bis 14,8 V. \
                     AGM-Batterien werden zeitweise mit bis zu etwa 15 V geladen, \
                     dauerhaft mehr deutet auf einen defekten Regler hin.",
                    "With the engine running there are {} V. Normal is 13.8 to 14.8 V. \
                     AGM batteries are sometimes charged at up to about 15 V; more than \
                     that all the time points to a faulty regulator.",
                    &[value],
                ),
                format(
                    "Mit dem geführten Batterietest nachmessen.",
                    "Measure again with the guided battery test.",
                    &[],
                ),
            );
        } else {
            c.add(
                "charging",
                Level::Good,
                Area::Charging,
                format("Lichtmaschine lädt", "Alternator is charging", &[]),
                format(
                    "Bei laufendem Motor liegen {} V an, das ist im normalen Bereich.",
                    "With the engine running there are {} V, which is normal.",
                    &[value],
                ),
                none(),
            );
        }
    }

    if off.is_empty() {
        c.skip(
            "battery_rest",
            rest_title,
            format(
                "Der Motor lief die ganze Zeit. Die Ruhespannung misst der geführte \
                 Batterietest.",
                "The engine ran the whole time. The guided battery test measures the \
                 resting voltage.",
                &[],
            ),
        );
    } else {
        c.ran();
        let v = median(off);
        let value = number(v, 2);
        let load_note = format(
            "Bei eingeschalteter Zündung ziehen Steuergeräte und Pumpen Strom, der Wert liegt \
             deshalb etwas unter der echten Ruhespannung.",
            "With the ignition on, control units and pumps draw current, so the value is a \
             little below the true resting voltage.",
            &[],
        );
        if v < limits::BATTERY_FLAT {
            c.add(
                "battery_rest",
                Level::Critical,
                Area::Charging,
                format("Batterie fast leer", "Battery almost flat", &[]),
                format(
                    "Bei stehendem Motor liegen nur {} V an. Damit springt das Auto \
                     womöglich bald nicht mehr an.",
                    "With the engine off there are only {} V. The car may soon fail to \
                     start.",
                    &[value],
                ) + load_note
                    + source_note.clone(),
                format(
                    "Batterie laden und danach mit dem geführten Batterietest prüfen. \
                     Ist sie bald wieder leer, ist sie verbraucht oder etwas zieht Strom \
                     im Stand.",
                    "Charge the battery and test it with the guided battery test. If it \
                     goes flat again soon, it is worn out or something drains it while \
                     parked.",
                    &[],
                ),
            );
        } else if v < limits::BATTERY_WEAK {
            c.add(
                "battery_rest",
                Level::Warning,
                Area::Charging,
                format("Batterie schwach", "Battery weak", &[]),
                format(
                    "Bei stehendem Motor liegen {} V an. Eine volle Batterie hat etwa \
                     12,6 V, unter 12,2 V ist sie nur noch etwa halb geladen oder \
                     gealtert.",
                    "With the engine off there are {} V. A full battery has about 12.6 V; \
                     below 12.2 V it is only about half charged or aged.",
                    &[value],
                ) + load_note
                    + source_note.clone(),
                format(
                    "Eine längere Fahrt machen oder die Batterie laden und dann mit dem \
                     geführten Batterietest nachmessen.",
                    "Take a longer drive or charge the battery, then measure again with \
                     the guided battery test.",
                    &[],
                ),
            );
        } else {
            c.add(
                "battery_rest",
                Level::Good,
                Area::Charging,
                format("Batteriespannung in Ordnung", "Battery voltage fine", &[]),
                format(
                    "Bei stehendem Motor liegen {} V an.",
                    "With the engine off there are {} V.",
                    &[value],
                ) + load_note,
                none(),
            );
        }
    }
}
