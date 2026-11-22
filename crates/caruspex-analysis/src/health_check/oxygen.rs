//! Switching of the oxygen sensors.

use super::{Area, Ctx, Series, get, none};
use crate::common::{Level, format, number};

#[derive(Default)]
struct Switching {
    samples: i32,
    duration: f64,
    switches: i32,
    low: f64,
    high: f64,
}

/// Counts rich/lean changes of a narrowband sensor. A hysteresis around the
/// 0.45 V midpoint keeps noise from counting as switching.
fn analyse_switching(c: &Ctx, s: &Series) -> Switching {
    let mut out = Switching::default();
    let mut state = 0; // -1 lean, +1 rich, 0 unknown
    let mut first = 0.0;
    let mut last = 0.0;
    for v in s {
        if !c.closed_loop_warm(v.t) {
            continue;
        }
        if out.samples == 0 {
            first = v.t;
            last = v.t;
            out.low = v.value;
            out.high = v.value;
        }
        out.samples += 1;
        first = f64::min(first, v.t);
        last = f64::max(last, v.t);
        out.low = out.low.min(v.value);
        out.high = out.high.max(v.value);
        let mut now = 0;
        if v.value < 0.3 {
            now = -1;
        } else if v.value > 0.6 {
            now = 1;
        }
        if now != 0 {
            if state != 0 && now != state {
                out.switches += 1;
            }
            state = now;
        }
    }
    out.duration = last - first;
    out
}

pub(super) fn check_oxygen_sensors(c: &mut Ctx) {
    if c.diesel() {
        return; // diesels have no switching sensors
    }
    let input = c.input;
    let title = format("Lambdasonden", "Oxygen sensors", &[]);
    // PIDs 14 and 18 are sensor 1 of bank 1 and 2 (in front of the
    // converter) for the usual sensor layout from PID 13; 15 and 19 are the
    // sensors behind it.
    const UPSTREAM: [i32; 2] = [0x14, 0x18];
    const DOWNSTREAM: [i32; 2] = [0x15, 0x19];
    if get(input, UPSTREAM[0]).is_none() && get(input, UPSTREAM[1]).is_none() {
        c.skip(
            "o2_switching",
            title,
            format(
                "Keine Spannungen der Lambdasonden (PID 14 bis 1B). Autos mit \
                 Breitbandsonden melden sie anders, dann ist diese Prüfung hier nicht \
                 möglich.",
                "No oxygen sensor voltages (PID 14 to 1B). Cars with wideband sensors \
                 report them differently, and then this check is not possible here.",
                &[],
            ),
        );
        return;
    }
    for bank in 0..2usize {
        let Some(up) = get(input, UPSTREAM[bank]) else {
            continue;
        };
        let id_suffix = if bank == 0 { "bank1" } else { "bank2" };
        let bank_name = format("Bank {}", "bank {}", &[number((bank + 1) as f64, 0)]);
        let sw = analyse_switching(c, up);
        if sw.samples < 10 || sw.duration < 10.0 {
            c.skip(
                format!("o2_switching_{id_suffix}"),
                format(
                    "Lambdasonde vorn ({})",
                    "Front oxygen sensor ({})",
                    std::slice::from_ref(&bank_name),
                ),
                format(
                    "Zu wenige Messwerte bei warmem Motor im geschlossenen Regelkreis. \
                     Nötig sind mindestens 10 Werte über 10 Sekunden.",
                    "Too few readings with a warm engine in closed loop. At least 10 \
                     readings over 10 seconds are needed.",
                    &[],
                ),
            );
            continue;
        }
        c.ran();
        let range = format(
            "{} und {} V",
            "{} and {} V",
            &[number(sw.low, 2), number(sw.high, 2)],
        );
        if sw.switches == 0 {
            let cause = if sw.high < 0.3 {
                format(
                    "Sie zeigt dauernd mager. Das kann die Sonde selbst sein, aber \
                     auch Falschluft oder ein Abgasleck vor der Sonde.",
                    "It reads lean all the time. That can be the sensor itself, but \
                     also an air leak or an exhaust leak in front of the sensor.",
                    &[],
                )
            } else if sw.low > 0.6 {
                format(
                    "Sie zeigt dauernd fett. Das kann die Sonde selbst sein oder \
                     wirklich zu viel Kraftstoff.",
                    "It reads rich all the time. That can be the sensor itself or \
                     genuinely too much fuel.",
                    &[],
                )
            } else {
                format(
                    "Die Spannung steht fast still. Typisch für eine gealterte oder \
                     verrußte Sonde.",
                    "The voltage barely moves. Typical of an aged or sooted sensor.",
                    &[],
                )
            };
            c.add(
                format!("o2_switching_{id_suffix}"),
                Level::Warning,
                Area::OxygenSensors,
                format(
                    "Lambdasonde vorn regelt nicht ({})",
                    "Front oxygen sensor not switching ({})",
                    &[bank_name],
                ),
                format(
                    "Bei warmem Motor sollte die Spannung der Sonde vor dem Katalysator \
                     ständig zwischen etwa 0,1 und 0,9 V hin und her springen. Hier lag \
                     sie {} Sekunden lang nur zwischen {}.",
                    "With a warm engine the voltage of the sensor in front of the catalytic \
                     converter should keep jumping between about 0.1 and 0.9 V. Here, for \
                     {} seconds, it only stayed between {}.",
                    &[number(sw.duration, 0), range],
                ) + cause,
                format(
                    "Sonde, Stecker und Kabel prüfen lassen.",
                    "Have the sensor, its plug and wire checked.",
                    &[],
                ),
            );
            continue;
        }
        let rate = f64::from(sw.switches) / sw.duration;
        if rate < 0.1 && sw.duration >= 30.0 {
            c.add(
                format!("o2_switching_{id_suffix}"),
                Level::Info,
                Area::OxygenSensors,
                format(
                    "Lambdasonde vorn eher träge ({})",
                    "Front oxygen sensor rather slow ({})",
                    std::slice::from_ref(&bank_name),
                ),
                format(
                    "Die Sonde hat in {} Sekunden nur {}-mal gewechselt. Eine frische Sonde \
                     wechselt bei warmem Motor etwa einmal pro Sekunde. Allerdings fragt der \
                     Adapter jeden Wert nur ein paar Mal pro Sekunde ab und kann schnelle \
                     Wechsel verpassen.",
                    "In {} seconds the sensor switched only {} times. A fresh sensor \
                     switches about once per second with a warm engine. However, the adapter \
                     polls each value only a few times per second and can miss fast changes.",
                    &[number(sw.duration, 0), number(f64::from(sw.switches), 0)],
                ),
                format(
                    "Bei 2000 bis 2500/min noch einmal messen. Bleibt sie träge, ist sie \
                     wahrscheinlich gealtert.",
                    "Measure again at 2000 to 2500 rpm. If it stays slow, it has probably \
                     aged.",
                    &[],
                ),
            );
        } else {
            c.add(
                format!("o2_switching_{id_suffix}"),
                Level::Good,
                Area::OxygenSensors,
                format(
                    "Lambdasonde vorn regelt ({})",
                    "Front oxygen sensor switching ({})",
                    std::slice::from_ref(&bank_name),
                ),
                format(
                    "{} Wechsel in {} Sekunden, Spannung zwischen {}.",
                    "{} switches in {} seconds, voltage between {}.",
                    &[
                        number(f64::from(sw.switches), 0),
                        number(sw.duration, 0),
                        range,
                    ],
                ),
                none(),
            );
        }

        // Catalyst: a good converter stores oxygen and smooths the signal
        // behind it. If the rear sensor copies the front one, it no longer
        // does that.
        let Some(down) = get(input, DOWNSTREAM[bank]) else {
            continue;
        };
        if sw.switches < 6 {
            continue;
        }
        let rear = analyse_switching(c, down);
        if rear.samples < 10 {
            continue;
        }
        c.ran();
        let ratio = f64::from(rear.switches) / f64::from(sw.switches);
        let caveat = format(
            "Das ist eine Schätzung aus den Live-Werten. Die genaue Prüfung macht das \
             Steuergerät selbst (Bereitschaftstest Katalysator und Modus 06).",
            "This is an estimate from live values. The control unit does the exact check \
             itself (catalyst readiness monitor and mode 06).",
            &[],
        );
        if ratio >= 0.7 {
            c.add(
                format!("catalyst_{id_suffix}"),
                Level::Warning,
                Area::Catalyst,
                format(
                    "Katalysator speichert wenig Sauerstoff ({})",
                    "Catalytic converter stores little oxygen ({})",
                    &[bank_name],
                ),
                format(
                    "Die Sonde hinter dem Katalysator wechselt fast so oft wie die davor \
                     ({} zu {} Wechsel). Ein guter Katalysator glättet das Signal \
                     dahinter. Das deutet auf einen gealterten Katalysator hin.",
                    "The sensor behind the converter switches almost as often as the one \
                     in front ({} to {} switches). A good converter smooths the signal \
                     behind it. That points to an aged converter.",
                    &[
                        number(f64::from(rear.switches), 0),
                        number(f64::from(sw.switches), 0),
                    ],
                ) + caveat,
                format(
                    "Auf den Fehler P0420/P0430 achten und vor einem Tausch die hintere \
                     Sonde und Abgaslecks prüfen lassen.",
                    "Watch for fault P0420/P0430, and before replacing anything have the \
                     rear sensor and exhaust leaks checked.",
                    &[],
                ),
            );
        } else {
            c.add(
                format!("catalyst_{id_suffix}"),
                Level::Good,
                Area::Catalyst,
                format(
                    "Katalysator arbeitet ({})",
                    "Catalytic converter working ({})",
                    &[bank_name],
                ),
                format(
                    "Die Sonde hinter dem Katalysator bleibt ruhig ({} Wechsel gegen {} \
                     vorn). So soll es sein.",
                    "The sensor behind the converter stays calm ({} switches against {} \
                     in front). That is how it should be.",
                    &[
                        number(f64::from(rear.switches), 0),
                        number(f64::from(sw.switches), 0),
                    ],
                ) + caveat,
                none(),
            );
        }
    }
}
