//! When the fault memory was last cleared, and the readiness monitors.

use super::faults::{codes, fault_list};
use super::{Ctx, Drive, FaultKind, Section, UsedCarFault, UsedCarInput, limits, none};
use crate::common::Level;
use crate::message::{Message, text};
use crate::readiness::{MonitorState, monitor_name};

pub(super) fn check_clearing(c: &mut Ctx, input: &UsedCarInput) {
    let mut facts = Vec::new();
    if let Some(km) = input.distance_since_clear_km {
        facts.push(text("%1 km", "%1 km").num(km, 0));
    }
    if let Some(n) = input.warm_ups {
        facts.push(text("%1 Warmläufe", "%1 warm-ups").num(n, 0));
    }
    if let Some(minutes) = input.minutes_since_clear {
        facts.push(text("%1 Stunden Betrieb", "%1 hours of operation").num(minutes / 60.0, 0));
    }
    if facts.is_empty() {
        c.skip(
            Section::Clearing,
            text("Zuletzt gelöscht", "Last cleared"),
            text(
                "Das Auto meldet nicht, wann der Fehlerspeicher zuletzt gelöscht wurde (PID 30, 31 und 4E).",
                "The car does not report when the fault memory was last cleared (PID 30, 31 and 4E).",
            ),
        );
    } else {
        let recent = input.warm_ups.is_some_and(|w| w < limits::RECENT_WARM_UPS)
            || input
                .distance_since_clear_km
                .is_some_and(|d| d < limits::RECENT_DISTANCE_KM)
            || input
                .minutes_since_clear
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
                    "Seit dem letzten Löschen: %1. Wer den Fehlerspeicher löscht, schaltet auch die Warnlampe aus und setzt die Selbsttests des Autos zurück. Fehler, die erst nach einigen Fahrten wiederkommen, sind bei der Besichtigung dann noch nicht zu sehen. Kurz vor einem Verkauf kann das heißen, dass ein Fehler versteckt werden soll. Es gibt auch harmlose Gründe, etwa eine Reparatur, eine Inspektion oder eine abgeklemmte Batterie.",
                    "Since the last clearing: %1. Clearing the fault memory also switches off the warning lamp and resets the car's self-tests. Faults that only come back after a few drives are then not visible at the viewing. Shortly before a sale this can mean that a fault is being hidden. There are harmless reasons too, such as a repair, a service or a disconnected battery.",
                )
                .list(facts),
                text(
                    "Wann und warum wurde der Fehlerspeicher zuletzt gelöscht? Gibt es dazu eine Rechnung?",
                    "When and why was the fault memory last cleared? Is there an invoice for it?",
                ),
            );
            let permanent: Vec<&UsedCarFault> = input
                .faults
                .iter()
                .filter(|f| f.kind == FaultKind::Permanent)
                .collect();
            if !permanent.is_empty() {
                c.add(
                    "cleared_but_permanent",
                    Section::Clearing,
                    Level::Critical,
                    text(
                        "Gelöscht, aber nicht repariert",
                        "Cleared, but not repaired",
                    ),
                    text(
                        "Trotz des Löschens meldet das Auto dauerhafte Fehler: %1. Diese Codes lassen sich nicht löschen; sie verschwinden erst, wenn das Auto selbst festgestellt hat, dass der Fehler behoben ist. Der Fehler war also vor dem Löschen da und ist es vermutlich noch.",
                        "Despite the clearing the car reports permanent faults: %1. These codes cannot be cleared; they only go away once the car itself has seen that the fault is fixed. So the fault was there before the clearing and probably still is.",
                    )
                    .list(fault_list(&permanent)),
                    text(
                        "Warum wurde %1 gelöscht, ohne den Fehler zu reparieren?",
                        "Why was %1 cleared without repairing the fault?",
                    )
                    .plain(codes(&permanent)),
                );
            }
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

    if let Some(km) = input.mil_distance_km
        && km > 0.0
    {
        c.add(
            "mil_distance",
            Section::Clearing,
            Level::Info,
            text(
                "Mit Warnlampe gefahren: %1 km",
                "Driven with the warning lamp on: %1 km",
            )
            .num(km, 0),
            text(
                "Seit dem letzten Löschen ist das Auto %1 km mit leuchtender Motorkontrollleuchte gefahren. Je länger ein Fehler ignoriert wird, desto eher entstehen Folgeschäden.",
                "Since the last clearing the car has been driven %1 km with the check engine light on. The longer a fault is ignored, the more likely it causes further damage.",
            )
            .num(km, 0),
            text(
                "Warum wurde mit leuchtender Motorkontrollleuchte weitergefahren?",
                "Why was the car driven on with the check engine light on?",
            ),
        );
    }
}

pub(super) fn check_readiness(c: &mut Ctx, input: &UsedCarInput) {
    if !input.readiness_read {
        c.skip(
            Section::Readiness,
            text("Bereitschaftstests", "Readiness monitors"),
            text(
                "Das Auto hat den Stand der Selbsttests (PID 01) nicht gemeldet.",
                "The car did not report the state of its self-tests (PID 01).",
            ),
        );
        return;
    }
    let supported: Vec<&MonitorState> = input.readiness.iter().filter(|m| m.supported).collect();
    if supported.is_empty() {
        if input.drive == Drive::Electric {
            c.add(
                "readiness_none_electric",
                Section::Readiness,
                Level::Good,
                text(
                    "Keine Abgas-Selbsttests",
                    "No emission self-tests",
                ),
                text(
                    "Ein reines E-Auto hat keinen Auspuff und damit keine Bereitschaftstests. Das ist richtig so.",
                    "A fully electric car has no exhaust and so no readiness monitors. That is as it should be.",
                ),
                none(),
            );
        } else {
            c.add(
                "readiness_none",
                Section::Readiness,
                Level::Info,
                text(
                    "Keine Bereitschaftstests gemeldet",
                    "No readiness monitors reported",
                ),
                text(
                    "Das Auto meldet keine Selbsttests. Manche Steuergeräte tun das nicht; dann lässt sich nicht sehen, ob die Tests seit dem letzten Löschen gelaufen sind.",
                    "The car reports no self-tests. Some control units do not; then it cannot be seen whether the tests have run since the last clearing.",
                ),
                none(),
            );
        }
        return;
    }
    let open: Vec<&MonitorState> = supported.iter().copied().filter(|m| !m.complete).collect();
    if open.is_empty() {
        c.add(
            "readiness_complete",
            Section::Readiness,
            Level::Good,
            text(
                "Alle Bereitschaftstests abgeschlossen",
                "All readiness monitors complete",
            ),
            text(
                "Das Auto hat alle %1 Selbsttests seit dem letzten Löschen beendet. Das spricht dagegen, dass kurz vor der Besichtigung gelöscht wurde.",
                "The car has finished all %1 self-tests since the last clearing. That speaks against a clearing shortly before the viewing.",
            )
            .num(supported.len() as f64, 0),
            none(),
        );
        return;
    }
    let names: Vec<Message> = open
        .iter()
        .map(|m| Message::default().name(monitor_name(m.monitor)))
        .collect();
    let level = if open.len() == 1 {
        Level::Info
    } else {
        Level::Warning
    };
    c.add(
        "readiness_open",
        Section::Readiness,
        level,
        text(
            "Offene Bereitschaftstests: %1 von %2",
            "Readiness monitors not complete: %1 of %2",
        )
        .num(open.len() as f64, 0)
        .num(supported.len() as f64, 0),
        text(
            "Noch nicht fertig: %1. Nach dem Löschen des Fehlerspeichers oder dem Abklemmen der Batterie braucht das Auto einige Fahrten, bis diese Selbsttests wieder durch sind. Einzelne offene Tests sind normal, viele passen zu einem frischen Löschen.",
            "Not complete yet: %1. After the fault memory was cleared or the battery disconnected, the car needs a few drives until these self-tests are done again. Single open tests are normal, many fit a fresh clearing.",
        )
        .list(names),
        if open.len() == 1 {
            none()
        } else {
            text(
                "Wurde in letzter Zeit die Batterie abgeklemmt oder der Fehlerspeicher gelöscht?",
                "Was the battery disconnected or the fault memory cleared recently?",
            )
        },
    );
}
