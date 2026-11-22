//! Idle, cooling and the other temperatures.

use super::{
    Area, Ctx, HealthInput, PID_CATALYST_TEMP, PID_COOLANT, PID_INTAKE_TEMP, PID_OIL_TEMP, PID_RPM,
    PID_RUN_TIME, PID_SPEED, RUNNING_RPM, get, limits, nearest, none,
};
use crate::common::{Level, format, number, same};
use crate::series::{latest, max_of, median, min_of};

/// How long the engine has been running, in seconds, if it can be told.
fn engine_run_time(input: &HealthInput) -> Option<f64> {
    if let Some(rt) = get(input, PID_RUN_TIME) {
        return Some(latest(rt));
    }
    if let Some(rpm) = get(input, PID_RPM) {
        let mut first = 0.0;
        let mut last = 0.0;
        let mut any = false;
        for v in rpm {
            if v.value <= RUNNING_RPM {
                continue;
            }
            if !any {
                first = v.t;
                last = v.t;
                any = true;
            }
            first = f64::min(first, v.t);
            last = f64::max(last, v.t);
        }
        if any {
            return Some(last - first);
        }
    }
    None
}

/// Warm idle speed against the target of the engine: only samples standing
/// still (speed 0) with the engine warm and below 1500 1/min count.
pub(super) fn check_idle(c: &mut Ctx) {
    let input = c.input;
    let Some(engine) = input.engine.as_ref() else {
        return;
    };
    let Some(rpm) = get(input, PID_RPM) else {
        return;
    };
    let speed = get(input, PID_SPEED);
    let coolant = get(input, PID_COOLANT);
    let idle: Vec<f64> = rpm
        .iter()
        .filter(|s| (RUNNING_RPM..=1500.0).contains(&s.value))
        .filter(|s| nearest(speed, s.t).is_some_and(|v| v < 0.5))
        .filter(|s| nearest(coolant, s.t).is_some_and(|v| v >= limits::WARM_COOLANT))
        .map(|s| s.value)
        .collect();
    if idle.len() < 5 {
        return;
    }
    c.ran();
    let at_idle = median(idle);
    // A little room above and below the band: the control unit lifts idle
    // briefly for the air conditioning, the alternator or a cold battery.
    let (low, high) = (engine.idle_min - 50.0, engine.idle_max + 100.0);
    if at_idle >= low && at_idle <= high {
        return;
    }
    let args = [
        number(at_idle, 0),
        number(engine.idle_min, 0),
        number(engine.idle_max, 0),
        same(&engine.codes),
    ];
    if at_idle > high {
        c.add(
            "idle_speed",
            Level::Info,
            Area::Mixture,
            format("Leerlauf erhöht", "Idle speed raised", &[]),
            format(
                "Warm und im Stand dreht der Motor im Mittel {} 1/min. Üblich sind für diesen \
                 Motor ({}) etwa {} bis {} 1/min. Eingeschaltete Verbraucher heben den Leerlauf \
                 an; bleibt er ohne sie hoch, sind Falschluft, die Drosselklappe oder ein \
                 Fühler häufige Gründe.",
                "Warm and standing still the engine turns {} rpm on average. Usual for this \
                 engine ({}) are about {} to {} rpm. Switched-on consumers raise the idle; if \
                 it stays high without them, an air leak, the throttle body or a sensor are \
                 common reasons.",
                &[
                    args[0].clone(),
                    args[3].clone(),
                    args[1].clone(),
                    args[2].clone(),
                ],
            ),
            format(
                "Mit ausgeschalteter Klimaanlage und Licht noch einmal messen; bleibt es so, \
                 Schläuche und Drosselklappe prüfen lassen.",
                "Measure again with the air conditioning and lights off; if it stays, have the \
                 hoses and the throttle body checked.",
                &[],
            ),
        );
    } else {
        c.add(
            "idle_speed",
            Level::Warning,
            Area::Mixture,
            format("Leerlauf zu niedrig", "Idle speed too low", &[]),
            format(
                "Warm und im Stand dreht der Motor im Mittel nur {} 1/min. Üblich sind für \
                 diesen Motor ({}) etwa {} bis {} 1/min. Zu niedriger Leerlauf lässt den Motor \
                 schütteln oder ausgehen; oft sind die Drosselklappe verschmutzt oder ein \
                 Zylinder arbeitet schwach.",
                "Warm and standing still the engine turns only {} rpm on average. Usual for \
                 this engine ({}) are about {} to {} rpm. Too low an idle makes the engine \
                 shake or stall; often the throttle body is dirty or a cylinder works weakly.",
                &[
                    args[0].clone(),
                    args[3].clone(),
                    args[1].clone(),
                    args[2].clone(),
                ],
            ),
            format(
                "Drosselklappe reinigen lassen und auf Aussetzer achten.",
                "Have the throttle body cleaned and watch for misfires.",
                &[],
            ),
        );
    }
}

pub(super) fn check_cooling(c: &mut Ctx) {
    let input = c.input;
    let thermo_title = format(
        "Thermostat und Betriebstemperatur",
        "Thermostat and operating temperature",
        &[],
    );
    let Some(coolant) = get(input, PID_COOLANT) else {
        c.skip(
            "thermostat",
            thermo_title,
            format(
                "Die Kühlmitteltemperatur (PID 05) wurde nicht gelesen.",
                "The coolant temperature (PID 05) was not read.",
                &[],
            ),
        );
        return;
    };
    c.ran();
    if min_of(coolant) <= -39.5 {
        // -40 C is the lowest value the PID can carry; sensors with an open
        // circuit report exactly that.
        c.add(
            "sensor_coolant",
            Level::Warning,
            Area::Sensors,
            format(
                "Kühlmittelfühler meldet -40 °C",
                "Coolant sensor reports -40 °C",
                &[],
            ),
            format(
                "-40 °C ist der kleinste Wert, den das Steuergerät melden kann. Das passiert \
                 fast immer, wenn der Fühler oder sein Kabel unterbrochen ist. Dann läuft \
                 der Motor mit einem Ersatzwert, oft zu fett, und der Lüfter springt \
                 dauernd an.",
                "-40 °C is the lowest value the control unit can report. That almost always \
                 happens when the sensor or its wire is broken. The engine then runs on a \
                 substitute value, often too rich, and the fan may run all the time.",
                &[],
            ),
            format(
                "Stecker und Kabel des Kühlmittelfühlers prüfen lassen.",
                "Have the coolant sensor's plug and wire checked.",
                &[],
            ),
        );
        return;
    }

    let hottest = max_of(coolant);
    if hottest >= limits::COOLANT_OVERHEAT {
        c.add(
            "overheating",
            Level::Critical,
            Area::Cooling,
            format("Motor zu heiß", "Engine overheating", &[]),
            format(
                "Das Kühlmittel hat {} °C erreicht. Ab etwa 115 °C droht Schaden an \
                 Zylinderkopf und Dichtung.",
                "The coolant reached {} °C. From about 115 °C the cylinder head and \
                 gasket are at risk.",
                &[number(hottest, 0)],
            ),
            format(
                "Anhalten, Motor abkühlen lassen und Kühlmittelstand prüfen (erst wenn er \
                 kalt ist, den Deckel öffnen). Nicht weiterfahren, bis die Ursache klar ist.",
                "Stop, let the engine cool down and check the coolant level (only open the \
                 cap once it is cold). Do not drive on until the cause is clear.",
                &[],
            ),
        );
    } else if hottest
        >= input
            .engine
            .as_ref()
            .map_or(limits::COOLANT_HOT, |e| e.coolant_hot)
    {
        c.add(
            "overheating",
            Level::Warning,
            Area::Cooling,
            format("Kühlmittel sehr warm", "Coolant very hot", &[]),
            format(
                "Das Kühlmittel hat {} °C erreicht. Viele Motoren laufen unter Last bis \
                 etwa 105 °C, darüber wird es knapp.",
                "The coolant reached {} °C. Many engines run up to about 105 °C under load; \
                 above that it gets tight.",
                &[number(hottest, 0)],
            ),
            format(
                "Kühlmittelstand prüfen und darauf achten, ob der Lüfter anspringt.",
                "Check the coolant level and watch whether the radiator fan comes on.",
                &[],
            ),
        );
    }

    let run_time = engine_run_time(input);
    let hottest_text = number(hottest, 0);
    if let Some(rt) = run_time.filter(|rt| *rt >= limits::LONG_RUN_SECONDS) {
        let minutes = number(rt / 60.0, 0);
        if hottest < limits::COLD_THERMOSTAT {
            c.add(
                "thermostat",
                Level::Warning,
                Area::Cooling,
                format("Motor wird nicht warm", "Engine does not warm up", &[]),
                format(
                    "Nach {} Minuten Laufzeit hat das Kühlmittel nur {} °C erreicht. \
                     Normal sind 80 bis 105 °C. Meist hängt das Thermostat offen: Der \
                     Motor wird nicht richtig warm, verbraucht mehr und die Heizung bleibt \
                     lau. Seltener misst der Temperaturfühler falsch. Bei Frost im Leerlauf \
                     oder bei sparsamen Dieseln kann es allerdings deutlich länger dauern.",
                    "After {} minutes of running the coolant only reached {} °C. Normal is \
                     80 to 105 °C. Usually the thermostat is stuck open: the engine never \
                     gets properly warm, uses more fuel and the heater stays lukewarm. Less \
                     often the temperature sensor reads wrong. In freezing weather at idle, \
                     or on frugal diesels, it can take much longer though.",
                    &[minutes, hottest_text],
                ),
                format(
                    "Thermostat und Kühlmittelfühler prüfen lassen. Kein Notfall, aber auf \
                     Dauer schlecht für Motor und Verbrauch.",
                    "Have the thermostat and coolant sensor checked. Not an emergency, but \
                     bad for the engine and fuel use in the long run.",
                    &[],
                ),
            );
        } else if hottest < limits::WARM_COOLANT {
            c.add(
                "thermostat",
                Level::Info,
                Area::Cooling,
                format("Motor bleibt eher kühl", "Engine stays rather cool", &[]),
                format(
                    "Nach {} Minuten Laufzeit hat das Kühlmittel {} °C erreicht. Normal \
                     sind 80 bis 105 °C. Bei kaltem Wetter und ruhiger Fahrt kann das \
                     noch in Ordnung sein.",
                    "After {} minutes of running the coolant reached {} °C. Normal is 80 \
                     to 105 °C. In cold weather and with gentle driving this can still be \
                     fine.",
                    &[minutes, hottest_text],
                ),
                format(
                    "Bei der nächsten längeren Fahrt noch einmal ansehen.",
                    "Look at it again on the next longer drive.",
                    &[],
                ),
            );
        } else {
            c.add(
                "thermostat",
                Level::Good,
                Area::Cooling,
                format(
                    "Betriebstemperatur erreicht",
                    "Operating temperature reached",
                    &[],
                ),
                format(
                    "Das Kühlmittel hat {} °C erreicht. Das Thermostat arbeitet.",
                    "The coolant reached {} °C. The thermostat works.",
                    &[hottest_text],
                ),
                none(),
            );
        }
    } else if hottest >= limits::WARM_COOLANT {
        c.add(
            "thermostat",
            Level::Good,
            Area::Cooling,
            format(
                "Betriebstemperatur erreicht",
                "Operating temperature reached",
                &[],
            ),
            format(
                "Das Kühlmittel hat {} °C erreicht. Das Thermostat arbeitet.",
                "The coolant reached {} °C. The thermostat works.",
                &[hottest_text],
            ),
            none(),
        );
    } else {
        let reason = match run_time {
            Some(rt) => format(
                "Der Motor lief erst {} Minuten. Für eine Aussage braucht es etwa 15 \
                 Minuten Fahrt.",
                "The engine had only run for {} minutes. A verdict needs about 15 \
                 minutes of driving.",
                &[number(rt / 60.0, 0)],
            ),
            None => format(
                "Die Laufzeit des Motors ist unbekannt (PID 1F oder Drehzahl fehlt), \
                 und der Motor war noch nicht warm.",
                "The engine run time is unknown (PID 1F or engine speed missing), \
                 and the engine was not warm yet.",
                &[],
            ),
        };
        c.skip("thermostat", thermo_title, reason);
    }
}

pub(super) fn check_other_temperatures(c: &mut Ctx) {
    let input = c.input;
    if let Some(iat) = get(input, PID_INTAKE_TEMP) {
        c.ran();
        if min_of(iat) <= -39.5 {
            c.add(
                "sensor_intake",
                Level::Warning,
                Area::Sensors,
                format(
                    "Ansauglufttemperatur meldet -40 °C",
                    "Intake air temperature reads -40 °C",
                    &[],
                ),
                format(
                    "-40 °C ist der kleinste Wert, den das Steuergerät melden kann. Fast \
                     immer ist dann der Fühler oder sein Kabel unterbrochen. Das Gemisch \
                     stimmt dadurch nicht mehr genau.",
                    "-40 °C is the lowest value the control unit can report. Almost always \
                     the sensor or its wire is broken then. The mixture is no longer \
                     quite right as a result.",
                    &[],
                ),
                format(
                    "Stecker und Kabel des Fühlers prüfen lassen (er sitzt oft im \
                     Luftmassenmesser).",
                    "Have the sensor's plug and wire checked (it often sits in the mass \
                     air flow sensor).",
                    &[],
                ),
            );
        }
    }
    if let Some(oil) = get(input, PID_OIL_TEMP) {
        c.ran();
        let hottest = max_of(oil);
        if hottest >= limits::OIL_HOT {
            let level = if hottest >= limits::OIL_OVERHEAT {
                Level::Critical
            } else {
                Level::Warning
            };
            c.add(
                "oil_temp",
                level,
                Area::Cooling,
                format("Öl sehr heiß", "Oil very hot", &[]),
                format(
                    "Das Motoröl hat {} °C erreicht. Normal sind etwa 90 bis 120 °C, bei \
                     sportlicher Fahrt auch kurz mehr. Ab etwa 150 °C altert das Öl sehr \
                     schnell und schmiert schlechter.",
                    "The engine oil reached {} °C. Normal is about 90 to 120 °C, briefly \
                     more when driven hard. From about 150 °C the oil ages very quickly and \
                     lubricates less well.",
                    &[number(hottest, 0)],
                ),
                format(
                    "Ölstand prüfen und den Motor schonen. Bleibt es so, Kühlung prüfen \
                     lassen.",
                    "Check the oil level and go easy on the engine. If it stays like this, \
                     have the cooling checked.",
                    &[],
                ),
            );
        }
    }
    if let Some(cat) = get(input, PID_CATALYST_TEMP) {
        c.ran();
        let hottest = max_of(cat);
        if hottest >= limits::CATALYST_HOT {
            c.add(
                "catalyst_temp",
                Level::Warning,
                Area::Catalyst,
                format("Katalysator sehr heiß", "Catalytic converter very hot", &[]),
                format(
                    "Das Steuergerät meldet {} °C am Katalysator. Ab etwa 900 °C kann er \
                     auf Dauer Schaden nehmen. Häufigster Grund sind Zündaussetzer, bei \
                     denen unverbrannter Kraftstoff im Katalysator nachbrennt.",
                    "The control unit reports {} °C at the catalytic converter. From about \
                     900 °C it can be damaged over time. The most common reason is \
                     misfiring, where unburnt fuel burns in the converter.",
                    &[number(hottest, 0)],
                ),
                format(
                    "Auf Aussetzer und Fehlercodes achten und hohe Last vermeiden, bis die \
                     Ursache klar ist.",
                    "Watch for misfires and fault codes and avoid heavy load until the cause \
                     is clear.",
                    &[],
                ),
            );
        }
    }
}
