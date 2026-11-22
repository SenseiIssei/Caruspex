//! Exhaust aftertreatment: AdBlue and the particulate filter.

use super::{Ctx, TruckFault, limits, none, spn};
use crate::common::Level;
use crate::message::{Message, text};
use crate::series::latest;

pub(super) fn check_def(c: &mut Ctx) {
    let input = c.input;
    let title = text("AdBlue-Füllstand", "DEF (AdBlue) level");
    match input.series(spn::DEF_LEVEL) {
        None => c.missing("def_level", title, spn::DEF_LEVEL),
        Some(s) => {
            let level_now = latest(s);
            let (level, title, advice) = if level_now < limits::DEF_CRITICAL {
                (
                    Level::Critical,
                    text("AdBlue fast leer: %1 %", "DEF nearly empty: %1 %"),
                    text(
                        "Sofort AdBlue nachfüllen. Ist der Tank leer, drosselt der Motor seine Leistung und startet nach dem Abstellen nur noch eingeschränkt.",
                        "Top up DEF at once. Once the tank is empty, the engine cuts its power and after switching off only restarts with limits.",
                    ),
                )
            } else if level_now < limits::DEF_WARNING {
                (
                    Level::Warning,
                    text("AdBlue niedrig: %1 %", "DEF low: %1 %"),
                    text("Bald AdBlue nachfüllen.", "Top up DEF soon."),
                )
            } else {
                (Level::Good, text("AdBlue %1 %", "DEF %1 %"), none())
            };
            c.add(
                "def_level",
                level,
                title.num(level_now, 0),
                text(
                    "Füllstand des AdBlue-Tanks. Ohne AdBlue verliert der Motor nach einer Warnphase Leistung, das schreibt die Abgasnorm vor.",
                    "Fill level of the DEF tank. Without DEF the engine loses power after a warning phase; the emission rules demand it.",
                ),
                advice,
            );
        }
    }

    if !input.faults_read {
        return;
    }
    let mut active = Vec::new();
    let mut previous = Vec::new();
    for unit in &input.units {
        active.extend(
            unit.active
                .iter()
                .filter(|f| spn::DEF_SYSTEM.contains(&f.spn))
                .map(TruckFault::line),
        );
        previous.extend(
            unit.previous
                .iter()
                .filter(|f| spn::DEF_SYSTEM.contains(&f.spn))
                .map(TruckFault::line),
        );
    }
    let explain = text(
        "AdBlue-Fehler: %1. Häufige Ursachen sind verunreinigtes oder verdünntes AdBlue, Kristalle an Dosierventil und Leitungen oder ein defekter Qualitätssensor. Bleibt so ein Fehler bestehen, drosselt der Motor nach einer Warnphase die Leistung.",
        "DEF faults: %1. Common causes are contaminated or diluted DEF, crystals at the dosing valve and lines, or a faulty quality sensor. If such a fault stays, the engine cuts its power after a warning phase.",
    );
    if !active.is_empty() {
        c.add(
            "def_faults",
            Level::Warning,
            text("Aktive AdBlue-Fehler", "Active DEF faults"),
            explain.list(active),
            text(
                "Nur frisches AdBlue nach ISO 22241 tanken und die Anlage in der Werkstatt prüfen lassen.",
                "Fill only fresh DEF to ISO 22241 and have the system checked at the workshop.",
            ),
        );
    } else if !previous.is_empty() {
        c.add(
            "def_faults",
            Level::Info,
            text("Frühere AdBlue-Fehler", "Previous DEF faults"),
            explain.list(previous),
            none(),
        );
    }
}

/// Words for the state of SPN 3701.
fn dpf_state_text(state: f64) -> Message {
    match state as i32 {
        0 => text("Freibrennen nicht nötig", "no regeneration needed"),
        1 => text(
            "Freibrennen nötig, niedrige Stufe",
            "regeneration needed, low level",
        ),
        2 => text(
            "Freibrennen nötig, mittlere Stufe",
            "regeneration needed, moderate level",
        ),
        _ => text(
            "Freibrennen nötig, höchste Stufe",
            "regeneration needed, highest level",
        ),
    }
}

pub(super) fn check_dpf(c: &mut Ctx) {
    let input = c.input;
    let soot = input.series(spn::SOOT_LOAD).map(latest);
    let state = input.series(spn::DPF_STATE).map(latest);
    let regen = input.series(spn::REGENERATION).map(latest);
    if soot.is_none() && state.is_none() && regen.is_none() {
        c.skip(
            "dpf",
            text("Partikelfilter", "Particulate filter"),
            text(
                "Der Lkw sendet weder Rußbeladung noch Regenerationszustand (SPN 3719, 3701, 3700). Ältere Lkw ohne Partikelfilter haben beides nicht.",
                "The truck sends neither the soot load nor the regeneration state (SPN 3719, 3701, 3700). Older trucks without a particulate filter have neither.",
            ),
        );
        return;
    }
    let mut facts = Vec::new();
    if let Some(s) = soot {
        facts.push(text("Rußbeladung %1 %", "soot load %1 %").num(s, 0));
    }
    if let Some(s) = state {
        facts.push(dpf_state_text(s));
    }
    let running = regen.is_some_and(|r| r as i32 == 1);
    if running {
        facts.push(text("Freibrennen läuft", "regeneration running"));
    }
    let worst_state = state.map_or(0, |s| s as i32);
    let soot_pct = soot.unwrap_or(0.0);
    let (level, title, advice) = if soot_pct >= limits::SOOT_CRITICAL || worst_state >= 3 {
        (
            Level::Critical,
            text("Partikelfilter voll", "Particulate filter full"),
            text(
                "Bald in die Werkstatt: ein so voller Filter braucht oft ein Freibrennen im Stand mit Diagnosegerät, sonst drosselt der Motor.",
                "To the workshop soon: a filter this full often needs a parked regeneration with a diagnostic tool, otherwise the engine cuts its power.",
            ),
        )
    } else if soot_pct >= limits::SOOT_WARNING
        || worst_state >= 1
        || regen.is_some_and(|r| r as i32 == 2)
    {
        (
            Level::Warning,
            text(
                "Partikelfilter muss freibrennen",
                "Particulate filter needs a regeneration",
            ),
            text(
                "Eine längere Fahrt mit gleichmäßiger Last, etwa eine halbe Stunde Autobahn, lässt den Filter meist von selbst freibrennen. Klappt das nicht, das Freibrennen im Stand nach Herstelleranleitung starten.",
                "A longer drive at steady load, about half an hour on the motorway, usually lets the filter regenerate by itself. If that does not work, start a parked regeneration as the maker describes.",
            ),
        )
    } else if running {
        (
            Level::Info,
            text(
                "Partikelfilter brennt gerade frei",
                "Particulate filter is regenerating",
            ),
            text(
                "Den Motor jetzt nicht abstellen, bis das Freibrennen fertig ist.",
                "Do not switch off the engine now until the regeneration is done.",
            ),
        )
    } else {
        (
            Level::Good,
            text("Partikelfilter in Ordnung", "Particulate filter fine"),
            none(),
        )
    };
    c.add(
        "dpf",
        level,
        title,
        text(
            "%1. Der Filter sammelt Ruß und verbrennt ihn regelmäßig bei hoher Abgastemperatur. Viel Kurzstrecke und Leerlauf lassen ihn schneller voll werden.",
            "%1. The filter collects soot and burns it off regularly at high exhaust temperature. Lots of short trips and idling fill it faster.",
        )
        .list(facts),
        advice,
    );

    if let Some(ash) = input.series(spn::ASH_LOAD).map(latest)
        && ash >= limits::ASH_WARNING
    {
        c.add(
            "dpf_ash",
            Level::Warning,
            text("Partikelfilter voller Asche: %1 %", "Particulate filter full of ash: %1 %").num(ash, 0),
            text(
                "Asche aus dem Motoröl brennt nicht ab und setzt den Filter mit der Zeit zu. Ab etwa 80 % steht eine Reinigung oder ein Tausch an.",
                "Ash from the engine oil does not burn off and clogs the filter over time. From about 80 % a cleaning or a replacement is due.",
            ),
            text(
                "Filterreinigung bei der Werkstatt einplanen.",
                "Plan a filter cleaning at the workshop.",
            ),
        );
    }
}
