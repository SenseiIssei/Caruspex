//! Stored and pending faults, and the short health check.

use super::{Ctx, FaultKind, Section, UsedCarFault, UsedCarInput, none};
use crate::common::{Level, TrafficLight};
use crate::message::{Message, raw, text};

pub(super) fn fault_list(faults: &[&UsedCarFault]) -> Vec<Message> {
    faults
        .iter()
        .map(|f| {
            if f.text.is_empty() {
                raw(f.code.clone())
            } else {
                text("%1 (%2)", "%1 (%2)")
                    .plain(f.code.clone())
                    .plain(f.text.clone())
            }
        })
        .collect()
}

pub(super) fn codes(faults: &[&UsedCarFault]) -> String {
    faults
        .iter()
        .map(|f| f.code.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn check_faults(c: &mut Ctx, input: &UsedCarInput) {
    if input.mil_on == Some(true) {
        c.add(
            "mil_on",
            Section::Faults,
            Level::Warning,
            text(
                "Motorkontrollleuchte an",
                "Check engine light on",
            ),
            text(
                "Das Steuergerät meldet die Motorkontrollleuchte als eingeschaltet. Bei laufendem Motor muss sie aus sein; leuchtet sie, liegt ein bestätigter Fehler an der Abgasreinigung oder am Motor vor.",
                "The control unit reports the check engine light as on. With the engine running it must be off; if it is lit, there is a confirmed fault in the emission control or the engine.",
            ),
            text(
                "Seit wann leuchtet die Motorkontrollleuchte, und was hat die Werkstatt dazu gesagt?",
                "Since when has the check engine light been on, and what did the workshop say about it?",
            ),
        );
    }
    if !input.faults_read {
        c.skip(
            Section::Faults,
            text("Fehlerspeicher", "Fault memory"),
            text(
                "Der Fehlerspeicher ließ sich nicht lesen.",
                "The fault memory could not be read.",
            ),
        );
        return;
    }
    let of = |kind: FaultKind| -> Vec<&UsedCarFault> {
        input.faults.iter().filter(|f| f.kind == kind).collect()
    };
    let stored = of(FaultKind::Stored);
    let pending = of(FaultKind::Pending);
    let permanent = of(FaultKind::Permanent);

    if stored.is_empty() && pending.is_empty() && permanent.is_empty() {
        c.add(
            "no_faults",
            Section::Faults,
            Level::Good,
            text("Kein Fehler gespeichert", "No faults stored"),
            text(
                "Weder gespeicherte noch vorläufige noch dauerhafte Fehler im Motorsteuergerät.",
                "No stored, pending or permanent faults in the engine control unit.",
            ),
            none(),
        );
        return;
    }
    if !stored.is_empty() {
        let level = stored
            .iter()
            .map(|f| f.level)
            .max()
            .unwrap_or(Level::Warning)
            .max(Level::Info);
        c.add(
            "faults_stored",
            Section::Faults,
            level,
            text("Gespeicherte Fehler: %1", "Stored faults: %1").num(stored.len() as f64, 0),
            text(
                "%1. Diese Fehler hat das Steuergerät bestätigt. Was sie bedeuten und was eine Reparatur ungefähr kostet, steht im Fehlerspeicher bei jedem Code.",
                "%1. The control unit has confirmed these faults. What they mean and roughly what a repair costs is shown with each code in the fault memory.",
            )
            .list(fault_list(&stored)),
            text(
                "Was ist die Ursache für %1, und was würde die Reparatur kosten?",
                "What is the cause of %1, and what would the repair cost?",
            )
            .plain(codes(&stored)),
        );
    }
    if !pending.is_empty() {
        c.add(
            "faults_pending",
            Section::Faults,
            Level::Info,
            text("Vorläufige Fehler: %1", "Pending faults: %1").num(pending.len() as f64, 0),
            text(
                "%1. Diese Fehler hat das Steuergerät einmal gesehen, aber noch nicht bestätigt. Sie können verschwinden oder zu gespeicherten Fehlern werden.",
                "%1. The control unit has seen these faults once but not confirmed them yet. They can disappear or become stored faults.",
            )
            .list(fault_list(&pending)),
            none(),
        );
    }
    if !permanent.is_empty() {
        c.add(
            "faults_permanent",
            Section::Faults,
            Level::Warning,
            text("Dauerhafte Fehler: %1", "Permanent faults: %1").num(permanent.len() as f64, 0),
            text(
                "%1. Dauerhafte Fehler lassen sich nicht löschen. Sie verschwinden erst, wenn das Auto selbst festgestellt hat, dass der Fehler behoben ist.",
                "%1. Permanent faults cannot be cleared. They only go away once the car itself has seen that the fault is fixed.",
            )
            .list(fault_list(&permanent)),
            text(
                "Wurde der Fehler hinter %1 repariert, und gibt es dafür eine Rechnung?",
                "Was the fault behind %1 repaired, and is there an invoice for it?",
            )
            .plain(codes(&permanent)),
        );
    }
}

pub(super) fn check_health(c: &mut Ctx, input: &UsedCarInput) {
    let Some(health) = &input.health else {
        return;
    };
    let level = match health.grade {
        TrafficLight::Green => Level::Good,
        TrafficLight::Yellow => Level::Info,
        TrafficLight::Red => Level::Warning,
    };
    c.add(
        "health",
        Section::Health,
        level,
        text(
            "Gesundheitscheck: %1 von 100 Punkten",
            "Health check: %1 of 100 points",
        )
        .num(f64::from(health.score), 0),
        raw(health.summary.clone()),
        none(),
    );
}
