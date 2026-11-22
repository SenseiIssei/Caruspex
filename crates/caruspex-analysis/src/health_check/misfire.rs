//! Mode 06 test results and misfire counters.

use std::collections::BTreeMap;

use super::{Area, Ctx, limits, none};
use crate::common::{Level, Phrase, format, number, same};
use crate::inputs::MonitorResult;

fn is_misfire_component(r: &MonitorResult) -> bool {
    r.component_id >= 0xA1 && r.component_id <= 0xAD
}

fn not_run_yet(r: &MonitorResult) -> bool {
    r.value == 0.0 && r.minimum == 0.0 && r.maximum == 0.0
}

fn test_name(r: &MonitorResult) -> Phrase {
    let mut name = r.name.clone();
    if !r.component.is_empty() {
        name = if name.is_empty() {
            r.component.clone()
        } else {
            format!("{}: {}", r.component, name)
        };
    }
    if name.is_empty() {
        name = format!("Test {:02X}/{:02X}", r.component_id, r.test_id);
    }
    same(&name)
}

pub(super) fn check_mode06(c: &mut Ctx) {
    let input = c.input;
    let title = format(
        "Bauteilprüfungen (Modus 06)",
        "Component tests (mode 06)",
        &[],
    );
    if !input.monitor_results_read {
        c.skip(
            "mode06",
            title,
            format(
                "Die Ergebnisse der Bauteilprüfungen wurden nicht gelesen.",
                "The component test results were not read.",
                &[],
            ),
        );
        return;
    }
    c.ran();
    let mut evaluated = 0;
    let mut problems = 0;
    for r in &input.monitor_results {
        if is_misfire_component(r) || not_run_yet(r) {
            continue;
        }
        evaluated += 1;
        let id = format!("mode06:{:02X}:{:02X}", r.component_id, r.test_id);
        let name = test_name(r);
        let measured = format(
            "Gemessen {} {}, erlaubt {} bis {}.",
            "Measured {} {}, allowed {} to {}.",
            &[
                number(r.value, 3),
                same(&r.unit),
                if r.has_minimum {
                    number(r.minimum, 3)
                } else {
                    same("-")
                },
                if r.has_maximum {
                    number(r.maximum, 3)
                } else {
                    same("-")
                },
            ],
        );
        if !r.passed {
            problems += 1;
            c.add(
                id,
                Level::Warning,
                Area::Monitoring,
                format("Prüfung nicht bestanden: {}", "Test failed: {}", &[name]),
                format(
                    "Das Steuergerät hat diesen Wert außerhalb seiner Grenzen gemessen. \
                     Meist folgt bald ein Fehlercode.",
                    "The control unit measured this value outside its limits. A fault \
                     code usually follows soon.",
                    &[],
                ) + measured,
                format(
                    "Das betroffene Bauteil prüfen lassen, bevor die Warnlampe angeht.",
                    "Have the component checked before the warning lamp comes on.",
                    &[],
                ),
            );
            continue;
        }
        // Zero limits are usually placeholders ("no lower limit"), so only
        // real limits count for the "close to the limit" check.
        let mut near = false;
        if r.has_maximum
            && r.maximum != 0.0
            && r.maximum - r.value <= limits::NEAR_LIMIT_SHARE * r.maximum.abs()
        {
            near = true;
        }
        if r.has_minimum
            && r.minimum != 0.0
            && r.value - r.minimum <= limits::NEAR_LIMIT_SHARE * r.minimum.abs()
        {
            near = true;
        }
        if near {
            problems += 1;
            c.add(
                id,
                Level::Info,
                Area::Monitoring,
                format("Knapp an der Grenze: {}", "Close to the limit: {}", &[name]),
                format(
                    "Die Prüfung ist bestanden, aber der Wert liegt weniger als 10 % von \
                     seiner Grenze entfernt. So kündigen sich Fehler oft lange vor der \
                     Warnlampe an.",
                    "The test passed, but the value is less than 10% away from its limit. \
                     That is how faults often show up long before the warning lamp.",
                    &[],
                ) + measured,
                format(
                    "Bei der nächsten Prüfung wieder ansehen. Wandert der Wert weiter zur \
                     Grenze, das Bauteil prüfen lassen.",
                    "Look again at the next check. If the value keeps moving towards the \
                     limit, have the component checked.",
                    &[],
                ),
            );
        }
    }
    if problems == 0 {
        c.add(
            "mode06",
            Level::Good,
            Area::Monitoring,
            format(
                "Bauteilprüfungen im grünen Bereich",
                "Component tests well within limits",
                &[],
            ),
            if evaluated > 0 {
                format(
                    "Alle {} Prüfungen liegen mit Abstand innerhalb ihrer Grenzen.",
                    "All {} tests are comfortably within their limits.",
                    &[number(f64::from(evaluated), 0)],
                )
            } else {
                format(
                    "Das Steuergerät hat noch keine Prüfung abgeschlossen.",
                    "The control unit has not completed any test yet.",
                    &[],
                )
            },
            none(),
        );
    }
}

pub(super) fn check_misfire(c: &mut Ctx) {
    let input = c.input;
    let title = format("Zündaussetzer je Zylinder", "Misfires per cylinder", &[]);
    if !input.monitor_results_read {
        c.skip(
            "misfire",
            title,
            format(
                "Die Ergebnisse der Bauteilprüfungen (Modus 06) wurden nicht gelesen.",
                "The component test results (mode 06) were not read.",
                &[],
            ),
        );
        return;
    }
    // CAN mode 06: component A2..AD is cylinder 1..12, test 0B is the
    // average over the last ten drives, test 0C the count of the current or
    // last drive.
    let mut per_cylinder: BTreeMap<i32, f64> = BTreeMap::new();
    let mut failed = false;
    for r in &input.monitor_results {
        if r.component_id < 0xA2 || r.component_id > 0xAD {
            continue;
        }
        if r.test_id != 0x0B && r.test_id != 0x0C {
            continue;
        }
        let cyl = r.component_id - 0xA1;
        let entry = per_cylinder.entry(cyl).or_insert(0.0);
        *entry = entry.max(r.value);
        if !r.passed {
            failed = true;
        }
    }
    if per_cylinder.is_empty() {
        c.skip(
            "misfire",
            title,
            format(
                "Das Auto meldet keine Aussetzerzähler in Modus 06 (gibt es meist nur bei \
                 CAN-Fahrzeugen).",
                "The car reports no misfire counters in mode 06 (usually only on CAN \
                 vehicles).",
                &[],
            ),
        );
        return;
    }
    c.ran();
    let mut total = 0.0;
    let mut worst = 0.0;
    let mut worst_cyl = 0;
    let mut list = Phrase::default();
    for (cyl, count) in &per_cylinder {
        let (cyl, count) = (*cyl, *count);
        total += count;
        if count > worst {
            worst = count;
            worst_cyl = cyl;
        }
        if count > 0.0 {
            let item = format(
                "Zylinder {}: {}",
                "cylinder {}: {}",
                &[number(f64::from(cyl), 0), number(count, 0)],
            );
            if !list.is_empty() {
                list.de.push_str(", ");
                list.en.push_str(", ");
            }
            list.de.push_str(&item.de);
            list.en.push_str(&item.en);
        }
    }
    if total <= 0.0 && !failed {
        c.add(
            "misfire",
            Level::Good,
            Area::Misfire,
            format("Keine Zündaussetzer", "No misfires", &[]),
            format(
                "Alle {} Zylinder zünden sauber, die Zähler stehen auf null.",
                "All {} cylinders fire cleanly, the counters are at zero.",
                &[number(per_cylinder.len() as f64, 0)],
            ),
            none(),
        );
        return;
    }
    let level = if failed || worst >= 20.0 {
        Level::Warning
    } else {
        Level::Info
    };
    let mut focus = Phrase::default();
    if total >= 10.0 && worst >= 0.6 * total {
        focus = format(
            "Fast alle Aussetzer kommen von Zylinder {}. Das spricht für ein Teil \
             genau dieses Zylinders: Zündkerze, Zündspule oder Einspritzventil. Ein \
             einfacher Test: Zündspule mit einem Nachbarzylinder tauschen und sehen, \
             ob der Fehler mitwandert.",
            "Almost all misfires come from cylinder {}. That points to a part of \
             exactly this cylinder: spark plug, ignition coil or injector. A simple \
             test: swap the coil with a neighbouring cylinder and see whether the \
             fault moves with it.",
            &[number(f64::from(worst_cyl), 0)],
        );
    }
    c.add(
        "misfire",
        level,
        Area::Misfire,
        if level == Level::Warning {
            format("Zündaussetzer erkannt", "Misfires detected", &[])
        } else {
            format("Vereinzelte Zündaussetzer", "Occasional misfires", &[])
        },
        format(
            "Gezählte Aussetzer: {}. Einzelne Aussetzer kommen vor, etwa beim Kaltstart \
             oder bei schlechtem Kraftstoff. Viele davon schütteln den Motor und schaden \
             dem Katalysator. Ab wann das Steuergerät einen Fehler setzt, legt jeder \
             Hersteller selbst fest.",
            "Misfires counted: {}. A few misfires happen, for example on a cold start or \
             with poor fuel. Many of them shake the engine and harm the catalytic \
             converter. Each manufacturer decides itself when a fault is set.",
            &[list],
        ) + focus,
        if level == Level::Warning {
            format(
                "Zündkerzen und Zündspulen prüfen lassen.",
                "Have the spark plugs and ignition coils checked.",
                &[],
            )
        } else {
            format(
                "Im Auge behalten und nach einigen Fahrten wieder prüfen.",
                "Keep an eye on it and check again after a few drives.",
                &[],
            )
        },
    );
}
