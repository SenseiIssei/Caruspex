//! Stored and pending fault codes, and the warning lamp.

use super::{Area, Ctx, none};
use crate::common::{Level, format, same};

pub(super) fn check_faults(c: &mut Ctx) {
    let input = c.input;
    if !input.faults_read {
        c.skip(
            "faults",
            format("Fehlerspeicher", "Fault memory", &[]),
            format(
                "Der Fehlerspeicher wurde nicht gelesen.",
                "The fault memory was not read.",
                &[],
            ),
        );
        return;
    }
    c.ran();
    if input.faults.is_empty() {
        c.add(
            "faults",
            Level::Good,
            Area::Faults,
            format("Keine Fehler gespeichert", "No fault codes stored", &[]),
            format(
                "Das Motorsteuergerät meldet keinen gespeicherten, vorgemerkten oder \
                 dauerhaften Fehler.",
                "The engine control unit reports no stored, pending or permanent fault.",
                &[],
            ),
            none(),
        );
        return;
    }
    for f in &input.faults {
        let level = match &input.fault_level {
            Some(cb) => cb(f),
            None => Level::Warning,
        };
        let kind = f.kind.to_ascii_lowercase();
        let meaning = if kind.contains("dauerhaft") || kind.contains("permanent") {
            format(
                "Dieser Fehler ist dauerhaft gespeichert. Er lässt sich nicht löschen und \
                 verschwindet erst, wenn das Steuergerät selbst festgestellt hat, dass alles \
                 wieder in Ordnung ist.",
                "This fault is stored as permanent. It cannot be cleared and only goes away once \
                 the control unit has confirmed by itself that everything is fine again.",
                &[],
            )
        } else if kind.contains("sporadisch")
            || kind.contains("intermittent")
            || kind.contains("vorgemerkt")
            || kind.contains("pending")
        {
            format(
                "Dieser Fehler ist erst vorgemerkt: Das Steuergerät hat ihn einmal bemerkt, aber \
                 noch nicht bestätigt. Taucht er bei den nächsten Fahrten wieder auf, wird er \
                 fest gespeichert.",
                "This fault is only pending: the control unit noticed it once but has not \
                 confirmed it yet. If it shows up again on the next trips, it gets stored for \
                 good.",
                &[],
            )
        } else {
            format(
                "Dieser Fehler ist fest gespeichert.",
                "This fault is stored.",
                &[],
            )
        };
        let advice = match level {
            Level::Critical => format(
                "Bald in die Werkstatt, und bis dahin möglichst wenig und \
                 schonend fahren.",
                "Have it seen by a garage soon, and until then drive as little \
                 and as gently as possible.",
                &[],
            ),
            Level::Warning => format(
                "Bei Gelegenheit prüfen lassen. In der Fehlerliste stehen \
                 typische Ursachen.",
                "Have it checked when convenient. The fault list shows typical \
                 causes.",
                &[],
            ),
            _ => format(
                "Im Auge behalten. Kommt er nach dem Löschen wieder, genauer \
                 nachsehen.",
                "Keep an eye on it. If it comes back after clearing, look into \
                 it more closely.",
                &[],
            ),
        };
        let title = if f.text.is_empty() {
            f.code.clone()
        } else {
            format!("{} {}", f.code, f.text)
        };
        c.add(
            format!("fault:{}", f.code),
            level,
            Area::Faults,
            same(&title),
            meaning,
            advice,
        );
    }
}

// --- Warning lamp ----------------------------------------------------------

pub(super) fn check_mil(c: &mut Ctx) {
    let Some(mil_on) = c.input.mil_on else {
        c.skip(
            "mil",
            format("Motorkontrollleuchte", "Check engine lamp", &[]),
            format(
                "Der Zustand der Leuchte (PID 01) wurde nicht gelesen.",
                "The lamp state (PID 01) was not read.",
                &[],
            ),
        );
        return;
    };
    c.ran();
    if mil_on {
        c.add(
            "mil",
            Level::Warning,
            Area::Faults,
            format("Motorkontrollleuchte an", "Check engine lamp on", &[]),
            format(
                "Das Steuergerät hat die gelbe Motorkontrollleuchte eingeschaltet. Es hat \
                 also einen Fehler erkannt, der die Abgaswerte verschlechtern kann. Mit \
                 leuchtender Lampe fällt ein Auto in vielen Ländern durch die \
                 Abgasuntersuchung.",
                "The control unit has switched on the yellow check engine lamp. It has \
                 found a fault that can make the emissions worse. With the lamp on, a car \
                 fails the emissions test in many countries.",
                &[],
            ),
            format(
                "Die Fehlerliste zeigt den Grund. Blinkt die Lampe während der Fahrt, \
                 sofort Gas wegnehmen und bald anhalten: Das bedeutet starke Aussetzer, die \
                 den Katalysator zerstören können.",
                "The fault list shows the reason. If the lamp flashes while driving, ease \
                 off at once and stop soon: that means heavy misfires that can destroy the \
                 catalytic converter.",
                &[],
            ),
        );
    } else {
        c.add(
            "mil",
            Level::Good,
            Area::Faults,
            format("Motorkontrollleuchte aus", "Check engine lamp off", &[]),
            format(
                "Das Steuergerät meldet die Warnlampe als aus.",
                "The control unit reports the warning lamp as off.",
                &[],
            ),
            none(),
        );
    }
}
