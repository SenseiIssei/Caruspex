//! Short and long term fuel trims per bank.

use super::{Area, Ctx, PID_RPM, get, limits, mean, nearest, none};
use crate::common::{Level, Phrase, format, number, signed_number};

pub(super) fn check_trims(c: &mut Ctx, bank: i32) {
    let input = c.input;
    let st_pid = if bank == 1 { 0x06 } else { 0x08 };
    let lt_pid = if bank == 1 { 0x07 } else { 0x09 };
    let st = get(input, st_pid);
    let lt = get(input, lt_pid);
    let id = if bank == 1 {
        "fuel_trim_bank1"
    } else {
        "fuel_trim_bank2"
    };
    let bank_name = format("Bank {}", "bank {}", &[number(f64::from(bank), 0)]);
    let check_title = format(
        "Gemischregelung ({})",
        "Mixture control ({})",
        std::slice::from_ref(&bank_name),
    );

    if st.is_none() && lt.is_none() {
        // Bank 2 only exists on V and boxer engines; its absence is normal.
        if bank == 1 {
            let reason = if c.diesel() {
                format(
                    "Diesel melden meist keine Gemischkorrektur (PID 06 bis 09).",
                    "Diesels usually report no fuel trims (PID 06 to 09).",
                    &[],
                )
            } else {
                format(
                    "Die Gemischkorrektur (PID 06 und 07) wurde nicht gelesen.",
                    "The fuel trims (PID 06 and 07) were not read.",
                    &[],
                )
            };
            c.skip(id, check_title, reason);
        }
        return;
    }

    let base = st.or(lt).unwrap_or(&[]);
    let other = if st.is_some() { lt } else { None };
    let rpm = get(input, PID_RPM);
    let mut all: Vec<f64> = Vec::new();
    let mut idle: Vec<f64> = Vec::new();
    let mut cruise: Vec<f64> = Vec::new();
    for s in base {
        if !c.closed_loop_warm(s.t) {
            continue;
        }
        let mut total = s.value;
        if let Some(other) = other {
            match nearest(Some(other), s.t) {
                Some(o) => total += o,
                None => continue,
            }
        }
        all.push(total);
        if let Some(r) = nearest(rpm, s.t) {
            if r < 1100.0 {
                idle.push(total);
            } else if r >= 1800.0 {
                cruise.push(total);
            }
        }
    }
    if all.is_empty() {
        c.skip(
            id,
            check_title,
            format(
                "Keine Werte bei warmem Motor im geschlossenen Regelkreis. Die Korrektur \
                 sagt erst etwas aus, wenn der Motor warm ist und die Lambdaregelung \
                 arbeitet.",
                "No readings with a warm engine in closed loop. The trims only mean \
                 something once the engine is warm and the oxygen control is active.",
                &[],
            ),
        );
        return;
    }
    c.ran();
    let avg = mean(&all);
    let magnitude = avg.abs();
    let value = signed_number(avg, 1);
    let partial = if st.is_some() && lt.is_some() {
        Phrase::default()
    } else {
        format(
            "(Nur ein Teil der Korrektur wurde gelesen, das Ergebnis ist deshalb ungenauer.)",
            "(Only part of the correction was read, so the result is less precise.)",
            &[],
        )
    };

    if magnitude < limits::TRIM_WARN {
        c.add(
            id,
            Level::Good,
            Area::Mixture,
            format(
                "Gemischregelung in Ordnung ({})",
                "Mixture control fine ({})",
                &[bank_name],
            ),
            format(
                "Die Korrektur liegt im Schnitt bei {} %, also im normalen Bereich von \
                 plus/minus 10 %.",
                "The correction averages {}%, well within the normal range of plus/minus \
                 10%.",
                &[value],
            ) + partial,
            none(),
        );
        return;
    }
    let level = if magnitude >= limits::TRIM_BAD {
        Level::Critical
    } else {
        Level::Warning
    };

    // Comparing idle and higher engine speed narrows the cause down: an air
    // leak matters most at idle, where little air flows anyway.
    let mut pattern = Phrase::default();
    if avg > 0.0 && idle.len() >= 3 && cruise.len() >= 3 {
        let at_idle = mean(&idle);
        let at_cruise = mean(&cruise);
        if at_idle >= limits::TRIM_WARN && at_cruise < 5.0 {
            pattern = format(
                "Im Leerlauf ist die Korrektur hoch ({} %), bei höherer Drehzahl \
                 fast normal ({} %). Das spricht stark für Falschluft.",
                "At idle the correction is high ({}%), at higher engine speed \
                 almost normal ({}%). That points strongly to an air leak.",
                &[signed_number(at_idle, 1), signed_number(at_cruise, 1)],
            );
        } else if at_idle >= limits::TRIM_WARN && at_cruise >= limits::TRIM_WARN {
            pattern = format(
                "Die Korrektur ist im Leerlauf und bei höherer Drehzahl hoch. Das \
                 spricht eher für den Luftmassenmesser oder den Kraftstoffdruck \
                 als für Falschluft.",
                "The correction is high both at idle and at higher engine speed. \
                 That points more to the mass air flow sensor or the fuel \
                 pressure than to an air leak.",
                &[],
            );
        }
    }

    if avg > 0.0 {
        c.add(
            id,
            level,
            Area::Mixture,
            format(
                "Gemisch zu mager ({})",
                "Mixture too lean ({})",
                &[bank_name],
            ),
            format(
                "Das Steuergerät gibt im Schnitt {} % mehr Kraftstoff dazu, als es \
                 berechnet hat. Es kommt also mehr Luft in den Motor, als gemessen wird, \
                 oder zu wenig Kraftstoff an. Häufige Gründe: Falschluft durch einen \
                 undichten Schlauch oder eine Dichtung hinter dem Luftmassenmesser, ein \
                 verschmutzter Luftmassenmesser oder zu wenig Kraftstoffdruck.",
                "The control unit adds on average {}% more fuel than it calculated. So \
                 either more air gets into the engine than is measured, or too little \
                 fuel arrives. Common reasons: an air leak through a cracked hose or \
                 gasket after the mass air flow sensor, a dirty mass air flow sensor or \
                 low fuel pressure.",
                &[value],
            ) + pattern
                + partial,
            format(
                "Im Leerlauf auf Zischen achten und Unterdruckschläuche ansehen. Eine \
                 Werkstatt findet Falschluft schnell mit einem Rauchtest.",
                "Listen for hissing at idle and look at the vacuum hoses. A garage finds \
                 air leaks quickly with a smoke test.",
                &[],
            ),
        );
    } else {
        c.add(
            id,
            level,
            Area::Mixture,
            format(
                "Gemisch zu fett ({})",
                "Mixture too rich ({})",
                &[bank_name],
            ),
            format(
                "Das Steuergerät nimmt im Schnitt {} % Kraftstoff weg. Es kommt also zu \
                 viel Kraftstoff oder zu wenig Luft an. Typische Gründe: ein undichtes \
                 Einspritzventil, zu hoher Kraftstoffdruck, ein hängendes \
                 Tankentlüftungsventil, ein falsch messender Luftmassenmesser oder \
                 Kraftstoff im Motoröl.",
                "The control unit takes away on average {}% fuel. So too much fuel or \
                 too little air arrives. Typical reasons: a leaking injector, too high \
                 fuel pressure, a stuck purge valve, a mass air flow sensor that reads \
                 wrong, or fuel in the engine oil.",
                &[number(magnitude, 1)],
            ) + partial,
            format(
                "Bald prüfen lassen, bevor der Katalysator Schaden nimmt. Ölstand ansehen: \
                 Riecht das Öl nach Kraftstoff oder steigt der Stand, ist Kraftstoff im Öl.",
                "Have it checked soon, before the catalytic converter suffers. Look at the \
                 oil: if it smells of fuel or the level rises, there is fuel in the oil.",
                &[],
            ),
        );
    }
}
