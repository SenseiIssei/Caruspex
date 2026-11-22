use super::*;
use caruspex_i18n::{Language, with_language};

/// A warm 24 V truck at idle, as the demo truck sends it: everything in
/// order except the particulate filter, which needs a regeneration.
fn warm_idle() -> TruckHealthInput {
    let mut input = TruckHealthInput {
        faults_read: true,
        engine_hours: Some(12_345.6),
        units: vec![
            TruckUnit {
                address: 0,
                label: "Engine (00)".into(),
                ..TruckUnit::default()
            },
            TruckUnit {
                address: 0x17,
                label: "Instrument cluster (17)".into(),
                ..TruckUnit::default()
            },
        ],
        ..TruckHealthInput::default()
    };
    for i in 0..10 {
        let t = f64::from(i);
        for (spn, value) in [
            (spn::ENGINE_SPEED, 600.0),
            (spn::ENGINE_LOAD, 8.0),
            (spn::COOLANT_TEMP, 85.0),
            (spn::COOLANT_LEVEL, 92.0),
            (spn::OIL_TEMP, 82.0),
            (spn::OIL_PRESSURE, 240.0),
            (spn::BATTERY, 27.8),
            (spn::DEF_LEVEL, 48.0),
            (spn::SOOT_LOAD, 82.0),
            (spn::ASH_LOAD, 38.0),
            (spn::REGENERATION, 0.0),
            (spn::DPF_STATE, 2.0),
            (spn::BOOST, 0.0),
            (spn::INTAKE_TEMP, 35.0),
        ] {
            input.add_value(spn, value, t);
        }
    }
    input
}

fn level(report: &TruckHealthReport, id: &str) -> Level {
    report
        .find(id)
        .unwrap_or_else(|| panic!("no finding {id}"))
        .level
}

#[test]
fn the_demo_truck_is_amber_for_its_particulate_filter() {
    let report = run_truck_health_check(&warm_idle());
    assert_eq!(report.grade, TrafficLight::Yellow);
    assert_eq!(level(&report, "dpf"), Level::Warning);
    for id in [
        "lamps_off",
        "faults_none",
        "coolant_temp",
        "coolant_level",
        "oil_temp",
        "oil_pressure",
        "charging",
        "def_level",
        "intake_temp",
        "engine_hours",
    ] {
        assert_eq!(level(&report, id), Level::Good, "{id}");
    }
    assert_eq!(report.system_volts, 24);
    assert!(report.score <= 84 && report.score >= 80, "{}", report.score);
    // Boost is not judged at idle, and says why.
    assert!(report.not_checked.iter().any(|s| s.id == "boost"));
    assert!(report.not_checked.iter().any(|s| s.id == "battery_rest"));
    let text = with_language(Language::English, || report.findings[0].explanation.text());
    assert!(text.contains("soot load 82 %"), "{text}");
}

#[test]
fn temperatures_pressures_and_levels_cross_their_limits() {
    let mut input = warm_idle();
    input.values.clear();
    for (spn, value) in [
        (spn::ENGINE_SPEED, 650.0),
        (spn::COOLANT_TEMP, 107.0),
        (spn::COOLANT_LEVEL, 70.0),
        (spn::OIL_TEMP, 118.0),
        (spn::OIL_PRESSURE, 60.0),
        (spn::BATTERY, 25.0),
        (spn::DEF_LEVEL, 3.0),
        (spn::SOOT_LOAD, 104.0),
        (spn::INTAKE_TEMP, 80.0),
    ] {
        input.add_value(spn, value, 0.0);
    }
    let report = run_truck_health_check(&input);
    assert_eq!(report.grade, TrafficLight::Red);
    assert!(report.score <= 49);
    assert_eq!(level(&report, "coolant_temp"), Level::Critical);
    assert_eq!(level(&report, "coolant_level"), Level::Warning);
    assert_eq!(level(&report, "oil_temp"), Level::Warning);
    assert_eq!(level(&report, "oil_pressure"), Level::Critical);
    assert_eq!(level(&report, "charging"), Level::Critical);
    assert_eq!(level(&report, "def_level"), Level::Critical);
    assert_eq!(level(&report, "dpf"), Level::Critical);
    assert_eq!(level(&report, "intake_temp"), Level::Warning);
    // Worst first.
    assert_eq!(report.findings[0].level, Level::Critical);
}

#[test]
fn twelve_volt_systems_are_judged_on_their_own_scale() {
    let mut input = TruckHealthInput::default();
    input.add_value(spn::ENGINE_SPEED, 0.0, 0.0);
    input.add_value(spn::BATTERY, 12.1, 0.0);
    input.add_value(spn::ENGINE_SPEED, 700.0, 10.0);
    input.add_value(spn::BATTERY, 14.1, 10.0);
    let report = run_truck_health_check(&input);
    assert_eq!(report.system_volts, 12);
    assert_eq!(level(&report, "charging"), Level::Good);
    assert_eq!(level(&report, "battery_rest"), Level::Warning);
    // A 24 V truck at 25.8 V with the engine running charges too little.
    let mut input = TruckHealthInput::default();
    input.add_value(spn::ENGINE_SPEED, 700.0, 0.0);
    input.add_value(spn::BATTERY, 25.8, 0.0);
    let report = run_truck_health_check(&input);
    assert_eq!(report.system_volts, 24);
    assert_eq!(level(&report, "charging"), Level::Warning);
    input.values.clear();
    input.add_value(spn::ENGINE_SPEED, 700.0, 0.0);
    input.add_value(spn::BATTERY, 30.2, 0.0);
    assert_eq!(
        level(&run_truck_health_check(&input), "charging"),
        Level::Warning
    );
}

#[test]
fn faults_lamps_and_def_faults() {
    let mut input = warm_idle();
    input.units[0].lamps = TruckLamps {
        mil: true,
        amber: true,
        ..TruckLamps::default()
    };
    input.units[0].active = vec![
        TruckFault {
            spn: 105,
            fmi: 3,
            text: "Intake manifold temperature: voltage too high".into(),
            occurrences: Some(7),
        },
        TruckFault {
            spn: 3364,
            fmi: 2,
            ..TruckFault::default()
        },
    ];
    input.units[1].lamps.red_stop = true;
    input.add_sensor_error(spn::INTAKE_TEMP);
    let report = run_truck_health_check(&input);
    assert_eq!(level(&report, "lamp_red_stop"), Level::Critical);
    // The engine's faults explain its lamp, so the lamp counts once.
    assert_eq!(level(&report, "lamp_amber"), Level::Info);
    assert_eq!(level(&report, "faults_unit"), Level::Warning);
    assert_eq!(level(&report, "def_faults"), Level::Warning);
    assert_eq!(level(&report, "intake_temp"), Level::Warning);
    assert!(report.find("faults_none").is_none());
    let english = with_language(Language::English, || {
        report.find("faults_unit").unwrap().explanation.text()
    });
    assert!(
        english.contains("SPN 105 FMI 3 (Intake manifold temperature: voltage too high), 7 times"),
        "{english}"
    );
    let german = with_language(Language::German, || {
        report.find("lamp_red_stop").unwrap().explanation.text()
    });
    assert!(german.contains("Instrument cluster (17)"), "{german}");
}

#[test]
fn missing_values_are_listed_with_their_spn() {
    let input = TruckHealthInput::default();
    let report = run_truck_health_check(&input);
    assert!(report.findings.is_empty());
    assert_eq!(report.grade, TrafficLight::Green);
    let reason = with_language(Language::English, || {
        report
            .not_checked
            .iter()
            .find(|s| s.id == "coolant_temp")
            .unwrap()
            .reason
            .text()
    });
    assert_eq!(reason, "The truck does not send this value (SPN 110).");
    assert!(report.not_checked.iter().any(|s| s.id == "faults"));
    assert!(report.not_checked.iter().any(|s| s.id == "dpf"));
}

#[test]
fn boost_is_judged_under_load_only() {
    let mut input = TruckHealthInput::default();
    for i in 0..5 {
        let t = f64::from(i);
        input.add_value(spn::ENGINE_SPEED, 1400.0, t);
        input.add_value(spn::ENGINE_LOAD, 70.0, t);
        input.add_value(spn::BOOST, 35.0, t);
    }
    assert_eq!(
        level(&run_truck_health_check(&input), "boost"),
        Level::Warning
    );
    input.values.remove(&spn::BOOST);
    for i in 0..5 {
        input.add_value(spn::BOOST, 160.0, f64::from(i));
    }
    assert_eq!(level(&run_truck_health_check(&input), "boost"), Level::Good);
}
