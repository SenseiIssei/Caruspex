use super::*;
use crate::test_support::{english, german};

fn fault(code: &str, stored: bool) -> FaultInput {
    FaultInput {
        code: code.into(),
        kind: if stored { "gespeichert" } else { "sporadisch" }.into(),
        stored,
        ecu: "Motor (7E8)".into(),
        text: String::new(),
    }
}

fn monitor(name: &str, ready: bool) -> ReadinessInput {
    ReadinessInput {
        name: name.into(),
        ready,
    }
}

/// What the demo car reports after connecting: P0171 and P0420 stored
/// with the lamp on, P0300 pending, two readiness monitors open.
fn demo_car() -> AdviceInput {
    AdviceInput {
        faults_read: true,
        faults: vec![
            fault("P0171", true),
            fault("P0420", true),
            fault("P0300", false),
        ],
        mil_on: Some(true),
        readiness: vec![
            monitor("Zündaussetzer", true),
            monitor("Kraftstoffsystem", true),
            monitor("Bauteile", true),
            monitor("Katalysator", false),
            monitor("Lambdasonde", false),
        ],
        ..AdviceInput::default()
    }
}

fn codes(report: &AdviceReport, urgency: Urgency) -> Vec<String> {
    report
        .items
        .iter()
        .filter(|i| i.urgency == urgency)
        .map(|i| {
            if i.code.is_empty() {
                format!("{:?}", i.source)
            } else {
                i.code.clone()
            }
        })
        .collect()
}

#[test]
fn the_demo_car_goes_to_the_workshop_soon() {
    let _lang = german();
    let report = what_to_do(&demo_car());
    assert!(!report.nothing_read);
    assert!(codes(&report, Urgency::StopNow).is_empty());
    // The pending misfire weighs more than the two medium codes.
    assert_eq!(codes(&report, Urgency::Soon), ["P0300", "P0171", "P0420"]);
    assert_eq!(codes(&report, Urgency::CanWait), ["Readiness"]);
    assert_eq!(report.headline, "Bald in die Werkstatt");

    // The lamp belongs to the stored codes, it gets no line of its own.
    assert!(report.items.iter().all(|i| i.source != Source::WarningLamp));

    let lean = report.items.iter().find(|i| i.code == "P0171").unwrap();
    assert_eq!(lean.page, "faults");
    assert!(lean.workshop.contains("P0171"), "{}", lean.workshop);
    assert!(lean.workshop.contains("Motorkontrollleuchte ist an"));
    assert!(lean.workshop.contains("Motor (7E8)"));
    assert!(!lean.why.is_empty());
    assert!(!lean.cost_text.is_empty());

    let catalyst = report.items.iter().find(|i| i.code == "P0420").unwrap();
    assert_ne!(catalyst.cost, Cost::Unknown);

    let readiness = report.items.last().unwrap();
    assert_eq!(readiness.cost, Cost::Free);
    assert!(readiness.title.contains("Katalysator, Lambdasonde"));
    assert!(readiness.workshop.contains("Abgasuntersuchung"));
}

#[test]
fn health_findings_back_up_the_matching_code() {
    let _lang = german();
    let mut input = demo_car();
    input.findings = vec![
        FindingInput {
            id: "fuel_trim_bank1".into(),
            level: Level::Warning,
            title: "Gemisch zu mager (Bank 1)".into(),
            explanation: "Die Korrektur liegt bei +14 %.".into(),
        },
        // Repeats what the fault memory says: not listed twice.
        FindingInput {
            id: "faults".into(),
            level: Level::Warning,
            title: "3 Fehler gespeichert".into(),
            explanation: String::new(),
        },
        FindingInput {
            id: "thermostat".into(),
            level: Level::Good,
            title: "Thermostat in Ordnung".into(),
            explanation: String::new(),
        },
    ];
    let report = what_to_do(&input);
    assert_eq!(report.items.len(), 4);
    let lean = report.items.iter().find(|i| i.code == "P0171").unwrap();
    assert_eq!(lean.evidence, ["Gemisch zu mager (Bank 1)"]);
    assert!(
        lean.workshop
            .contains("Messwerte dazu: Gemisch zu mager (Bank 1).")
    );
}

#[test]
fn a_measured_misfire_turns_the_code_into_stop_now() {
    let _lang = german();
    let mut input = demo_car();
    input.findings = vec![FindingInput {
        id: "misfire".into(),
        level: Level::Critical,
        title: "Zylinder 3 setzt aus".into(),
        explanation: "Viele Aussetzer in kurzer Zeit.".into(),
    }];
    let report = what_to_do(&input);
    assert_eq!(codes(&report, Urgency::StopNow), ["P0300"]);
    let misfire = &report.items[0];
    assert_eq!(misfire.why, "Viele Aussetzer in kurzer Zeit.");
    assert_eq!(report.headline, "Bitte anhalten");
}

#[test]
fn a_flashing_lamp_means_stop_now() {
    let _lang = german();
    let mut input = demo_car();
    input.mil_flashing = true;
    let report = what_to_do(&input);
    assert_eq!(codes(&report, Urgency::StopNow), ["P0300"]);
    assert!(report.items[0].why.contains("Katalysator"));
    assert!(report.items[0].workshop.contains("geblinkt"));

    // Without a misfire code the observation stands on its own.
    let report = what_to_do(&AdviceInput {
        faults_read: true,
        mil_on: Some(true),
        mil_flashing: true,
        ..AdviceInput::default()
    });
    assert_eq!(codes(&report, Urgency::StopNow), ["Observation"]);
    assert_eq!(codes(&report, Urgency::Soon), ["WarningLamp"]);
}

#[test]
fn a_red_lamp_and_critical_codes_mean_stop_now() {
    let _lang = german();
    let report = what_to_do(&AdviceInput {
        faults_read: true,
        faults: vec![fault("P0524", true)],
        mil_on: Some(true),
        red_warning_lamp: true,
        ..AdviceInput::default()
    });
    assert_eq!(codes(&report, Urgency::StopNow), ["Observation", "P0524"]);
    // A critical code stays critical while pending.
    let report = what_to_do(&AdviceInput {
        faults_read: true,
        faults: vec![fault("P0217", false)],
        ..AdviceInput::default()
    });
    assert_eq!(codes(&report, Urgency::StopNow), ["P0217"]);
}

#[test]
fn nothing_to_do_without_faults() {
    let _lang = german();
    let report = what_to_do(&AdviceInput {
        faults_read: true,
        mil_on: Some(false),
        readiness: vec![monitor("Katalysator", true)],
        ..AdviceInput::default()
    });
    assert!(report.items.is_empty());
    assert!(!report.nothing_read);
    assert_eq!(report.headline, "Alles in Ordnung");
}

#[test]
fn nothing_read_says_so() {
    let _lang = german();
    let report = what_to_do(&AdviceInput::default());
    assert!(report.nothing_read);
    assert!(report.items.is_empty());
    assert_eq!(report.headline, "Noch nichts gelesen");
}

#[test]
fn only_pending_codes_can_mostly_wait() {
    let _lang = german();
    let report = what_to_do(&AdviceInput {
        faults_read: true,
        faults: vec![fault("P0171", false), fault("P0420", false)],
        mil_on: Some(false),
        ..AdviceInput::default()
    });
    assert_eq!(codes(&report, Urgency::CanWait), ["P0171", "P0420"]);
    assert!(report.items[0].why.starts_with("Noch nicht bestätigt"));
    assert!(!report.items[0].workshop.contains("Motorkontrollleuchte"));
    assert_eq!(report.headline, "Nichts Dringendes");
}

#[test]
fn the_lamp_without_a_code_is_listed() {
    let _lang = german();
    let report = what_to_do(&AdviceInput {
        faults_read: true,
        faults: vec![fault("P0171", false)],
        mil_on: Some(true),
        ..AdviceInput::default()
    });
    assert_eq!(codes(&report, Urgency::Soon), ["WarningLamp"]);
    assert_eq!(codes(&report, Urgency::CanWait), ["P0171"]);
}

#[test]
fn live_values_overheating_and_charging() {
    let _lang = german();
    let report = what_to_do(&AdviceInput {
        live: vec![(0x05, 118.0), (0x0C, 800.0), (0x42, 12.1)],
        // The same overheating once more from the health check: one line.
        findings: vec![FindingInput {
            id: "overheating".into(),
            level: Level::Critical,
            title: "Motor überhitzt".into(),
            explanation: "Das Kühlmittel erreicht 118 °C.".into(),
        }],
        ..AdviceInput::default()
    });
    assert_eq!(codes(&report, Urgency::StopNow), ["Finding"]);
    assert_eq!(report.items[0].evidence, ["Kühlmittel sehr heiß: 118 °C"]);
    assert_eq!(codes(&report, Urgency::Soon), ["LiveValue"]);
    assert_eq!(report.items[1].title, "Lichtmaschine lädt nicht: 12,1 V");
    assert_eq!(report.items[1].page, "battery");

    // With the engine off a low voltage says nothing about charging.
    let report = what_to_do(&AdviceInput {
        live: vec![(0x42, 12.1)],
        ..AdviceInput::default()
    });
    assert!(report.items.is_empty());
}

#[test]
fn texts_follow_the_language() {
    let _lang = english();
    let report = what_to_do(&demo_car());
    assert_eq!(report.headline, "Visit a workshop soon");
    let lean = report.items.iter().find(|i| i.code == "P0171").unwrap();
    assert!(lean.workshop.starts_with("Trouble code P0171"));
    assert_eq!(cost_text(Cost::Free), "Costs nothing");
}

#[test]
fn misfire_codes() {
    assert!(is_misfire_code("P0300"));
    assert!(is_misfire_code("P0312"));
    assert!(!is_misfire_code("P0313"));
    assert!(!is_misfire_code("P030A"));
    assert!(!is_misfire_code("P0171"));
}
