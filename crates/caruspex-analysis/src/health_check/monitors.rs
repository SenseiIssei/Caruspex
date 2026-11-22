//! Readiness monitors and the history since the fault memory was last cleared.

use super::{Area, Ctx, PID_CLEAR_DISTANCE, PID_MIL_DISTANCE, PID_WARMUPS, get, limits, none};
use crate::common::{Level, Phrase, format, number};
use crate::readiness::{Monitor, monitor_name};
use crate::series::latest;

pub(super) fn check_readiness(c: &mut Ctx) {
    let input = c.input;
    if !input.readiness_read {
        c.skip(
            "readiness",
            format("Bereitschaftstests", "Readiness monitors", &[]),
            format(
                "Die Bereitschaft (PID 01) wurde nicht gelesen.",
                "Readiness (PID 01) was not read.",
                &[],
            ),
        );
        return;
    }
    c.ran();
    let mut supported = 0;
    let mut open: Vec<Monitor> = Vec::new();
    let mut diesel_monitors = false;
    for m in &input.readiness {
        if !m.supported {
            continue;
        }
        supported += 1;
        if m.monitor >= Monitor::NmhcCatalyst {
            diesel_monitors = true;
        }
        if !m.complete {
            open.push(m.monitor);
        }
    }
    let diesel = c.diesel() || diesel_monitors;

    if open.is_empty() {
        c.add(
            "readiness",
            Level::Good,
            Area::Readiness,
            format(
                "Alle Bereitschaftstests fertig",
                "All readiness monitors complete",
                &[],
            ),
            format(
                "{} von {} Selbsttests sind abgeschlossen. Das ist die beste Voraussetzung \
                 für die Abgasuntersuchung.",
                "{} of {} self-tests are complete. That is the best starting point for \
                 the emissions test.",
                &[
                    number(f64::from(supported), 0),
                    number(f64::from(supported), 0),
                ],
            ),
            none(),
        );
        return;
    }

    let mut list = Phrase::default();
    for (i, m) in open.iter().enumerate() {
        let name = monitor_name(*m);
        if i > 0 {
            list.de.push_str(", ");
            list.en.push_str(", ");
        }
        list.de.push_str(name.de);
        list.en.push_str(name.en);
    }
    let level = if diesel {
        Level::Warning
    } else if open.len() == 1 {
        Level::Info
    } else {
        Level::Warning
    };
    let title = if open.len() == 1 {
        format(
            "1 Bereitschaftstest noch offen",
            "1 readiness monitor not complete",
            &[],
        )
    } else {
        format(
            "{} Bereitschaftstests noch offen",
            "{} readiness monitors not complete",
            &[number(open.len() as f64, 0)],
        )
    };
    let explanation = format(
        "Noch nicht fertig: {}. Das Auto hat diese Selbsttests seit dem letzten Löschen des \
         Fehlerspeichers (oder seit die Batterie abgeklemmt war) noch nicht zu Ende gebracht. \
         Das ist an sich kein Defekt, solange keine Fehler gespeichert sind.",
        "Not complete yet: {}. The car has not finished these self-tests since the fault \
         memory was last cleared (or since the battery was disconnected). That alone is not a \
         defect, as long as no faults are stored.",
        &[list],
    );
    let rule = if diesel {
        format(
            "Für die Abgasuntersuchung sollten beim Diesel möglichst alle Tests fertig \
             sein; die genauen Regeln hängen vom Land und der Prüfstelle ab.",
            "For the emissions test, a diesel should have all monitors complete if \
             possible; the exact rules depend on the country and the testing station.",
            &[],
        )
    } else {
        format(
            "Als Faustregel für die Abgasuntersuchung: Beim Benziner wird oft ein offener \
             Test hingenommen (so regelt es zum Beispiel die US-Umweltbehörde EPA für \
             Autos ab Baujahr 2001), mehrere offene Tests meist nicht. Die genauen Regeln \
             hängen vom Land und der Prüfstelle ab.",
            "As a rule of thumb for the emissions test: a petrol car is often accepted \
             with one incomplete monitor (that is the US EPA rule for model year 2001 and \
             later), with several usually not. The exact rules depend on the country and \
             the testing station.",
            &[],
        )
    };
    let advice = rule
        + format(
            "Am sichersten ist es, vorher die passenden Fahrten zu machen (siehe Fahrzyklus-Hilfe) \
             und danach erneut zu prüfen.",
            "The safest way is to do the matching drives first (see the drive cycle help) and \
             check again afterwards.",
            &[],
        );
    c.add(
        "readiness",
        level,
        Area::Readiness,
        title,
        explanation,
        advice,
    );
}

// --- Clearing history ------------------------------------------------------

pub(super) fn check_history(c: &mut Ctx) {
    let input = c.input;
    let warmups = get(input, PID_WARMUPS);
    let distance = get(input, PID_CLEAR_DISTANCE);
    if warmups.is_none() && distance.is_none() {
        c.skip(
            "cleared_recently",
            format("Zuletzt gelöscht", "Last cleared", &[]),
            format(
                "Warmläufe und Strecke seit dem Löschen (PID 30 und 31) fehlen.",
                "Warm-ups and distance since clearing (PID 30 and 31) are missing.",
                &[],
            ),
        );
    } else {
        c.ran();
        let w = warmups.map(latest);
        let d = distance.map(latest);
        let recent = w.is_some_and(|w| w < f64::from(limits::FEW_WARMUPS))
            || d.is_some_and(|d| d < limits::SHORT_DISTANCE_KM);

        let facts = match (w, d) {
            (Some(w), Some(d)) => format(
                "Seit dem letzten Löschen: {} Warmläufe und {} km.",
                "Since the last clearing: {} warm-ups and {} km.",
                &[number(w, 0), number(d, 0)],
            ),
            (Some(w), None) => format(
                "Seit dem letzten Löschen: {} Warmläufe.",
                "Since the last clearing: {} warm-ups.",
                &[number(w, 0)],
            ),
            (None, d) => format(
                "Seit dem letzten Löschen: {} km.",
                "Since the last clearing: {} km.",
                &[number(d.unwrap_or(0.0), 0)],
            ),
        };

        if recent {
            let mut explanation = facts
                + format(
                    "Das ist sehr wenig. Mit dem Löschen werden auch die Bereitschaftstests \
                     zurückgesetzt, und Fehler, die erst nach einiger Zeit wiederkommen, sind noch \
                     nicht zu sehen. Beim Gebrauchtwagenkauf kann das ein Hinweis sein, dass \
                     jemand Fehler kurz vor der Besichtigung gelöscht hat. Es gibt aber auch \
                     harmlose Gründe, etwa eine Reparatur oder eine abgeklemmte Batterie.",
                    "That is very little. Clearing also resets the readiness monitors, and faults \
                     that only come back after a while are not visible yet. When buying a used \
                     car, this can be a hint that someone cleared faults shortly before the \
                     viewing. There are harmless reasons too, such as a repair or a disconnected \
                     battery.",
                    &[],
                );
            if c.has_incomplete_readiness() {
                explanation = explanation
                    + format(
                        "Dazu passen die noch offenen Bereitschaftstests.",
                        "The incomplete readiness monitors fit that picture.",
                        &[],
                    );
            }
            c.add(
                "cleared_recently",
                Level::Warning,
                Area::History,
                format(
                    "Fehlerspeicher vor Kurzem gelöscht",
                    "Fault memory cleared recently",
                    &[],
                ),
                explanation,
                format(
                    "Beim Kauf nachfragen, warum gelöscht wurde. Am besten nach einer \
                     Probefahrt von 20 bis 30 Minuten oder nach ein paar Tagen Fahrt \
                     noch einmal prüfen.",
                    "When buying, ask why it was cleared. Ideally check again after a test \
                     drive of 20 to 30 minutes or after a few days of driving.",
                    &[],
                ),
            );
        } else {
            c.add(
                "cleared_recently",
                Level::Good,
                Area::History,
                format("Nicht frisch gelöscht", "Not freshly cleared", &[]),
                facts
                    + format(
                        "Kein Hinweis auf ein Löschen kurz vor der Prüfung.",
                        "No sign of clearing shortly before the check.",
                        &[],
                    ),
                none(),
            );
        }
    }

    if let Some(mil) = get(input, PID_MIL_DISTANCE) {
        c.ran();
        let km = latest(mil);
        if km > 0.0 {
            c.add(
                "mil_distance",
                Level::Info,
                Area::History,
                format(
                    "Warnlampe war {} km lang an",
                    "Warning lamp was on for {} km",
                    &[number(km, 0)],
                ),
                format(
                    "Seit dem letzten Löschen ist das Auto {} km mit leuchtender \
                     Motorkontrollleuchte gefahren.",
                    "Since the last clearing the car has been driven {} km with the check \
                     engine lamp on.",
                    &[number(km, 0)],
                ),
                format(
                    "Je länger ein Fehler ignoriert wird, desto eher entstehen Folgeschäden. \
                     Den Grund in der Fehlerliste nachsehen.",
                    "The longer a fault is ignored, the more likely it causes further \
                     damage. Look up the reason in the fault list.",
                    &[],
                ),
            );
        }
    }
}
