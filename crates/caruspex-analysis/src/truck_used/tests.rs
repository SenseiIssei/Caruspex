use super::*;
use crate::truck_health::{TruckFault, TruckLamps};
use crate::used_car::Section;
use caruspex_i18n::{Language, with_language};

const VIN: &str = "YCXTR42H4NC051739";

fn unit(address: u8, label: &str) -> TruckUnit {
    TruckUnit {
        address,
        label: label.into(),
        ..TruckUnit::default()
    }
}

/// The demo truck as the worker reads it: three VINs alike, three
/// odometers within a few km, 12 346 hours with a third at idle, DM21
/// 8 412 km since clearing, two active and two previous faults.
fn demo() -> TruckUsedInput {
    let mut engine = unit(0x00, "Engine (00)");
    engine.lamps = TruckLamps {
        mil: true,
        amber: true,
        ..TruckLamps::default()
    };
    engine.active = vec![
        TruckFault {
            spn: 105,
            fmi: 3,
            occurrences: Some(7),
            ..TruckFault::default()
        },
        TruckFault {
            spn: 111,
            fmi: 17,
            occurrences: Some(2),
            ..TruckFault::default()
        },
    ];
    engine.previous = vec![TruckFault {
        spn: 1761,
        fmi: 18,
        occurrences: Some(2),
        ..TruckFault::default()
    }];
    let mut brakes = unit(0x0B, "Brakes (0B)");
    brakes.previous = vec![TruckFault {
        spn: 904,
        fmi: 2,
        occurrences: Some(4),
        ..TruckFault::default()
    }];
    TruckUsedInput {
        vins: ["Engine (00)", "Instrument cluster (17)", "Tachograph (EE)"]
            .iter()
            .map(|l| UnitVin {
                label: l.to_string(),
                vin: VIN.into(),
            })
            .collect(),
        units: vec![UnitIdentity {
            label: "Engine (00)".into(),
            make: "CARUSPEX".into(),
            model: "CX-D13".into(),
            serial: "E7712093".into(),
            software: vec!["ECM 4.2.1".into()],
        }],
        odometers: vec![
            OdometerReading {
                label: "Engine (00)".into(),
                spn: SPN_FINE_DISTANCE,
                km: 512_358.4,
            },
            OdometerReading {
                label: "Instrument cluster (17)".into(),
                spn: SPN_DASHBOARD_DISTANCE,
                km: 512_352.6,
            },
            OdometerReading {
                label: "Tachograph (EE)".into(),
                spn: SPN_FINE_DISTANCE,
                km: 512_352.1,
            },
        ],
        engine_hours: Some(12_345.6),
        idle_hours: Some(3_950.25),
        fuel_total_l: Some(187_654.5),
        idle_fuel_l: Some(9_875.5),
        lamp_history: LampHistory {
            mil_km: Some(37.0),
            since_clear_km: Some(8_412.0),
            mil_minutes: Some(55.0),
            since_clear_minutes: Some(9_870.0),
        },
        faults: vec![engine, unit(0x03, "Transmission (03)"), brakes],
        faults_read: true,
        previous_read: true,
        ..TruckUsedInput::default()
    }
}

fn level(report: &TruckUsedReport, id: &str) -> Level {
    report
        .report
        .find(id)
        .unwrap_or_else(|| panic!("no item {id}"))
        .level
}

#[test]
fn the_demo_truck_is_bought_with_questions() {
    let r = evaluate_used_truck(&demo());
    assert_eq!(r.report.light, TrafficLight::Yellow);
    assert_eq!(level(&r, "vins_agree"), Level::Good);
    assert_eq!(level(&r, "units"), Level::Good);
    assert_eq!(level(&r, "mileage_consistent"), Level::Good);
    assert_eq!(level(&r, "speed_plausible"), Level::Good);
    assert_eq!(level(&r, "idle_share"), Level::Info);
    assert_eq!(level(&r, "fuel_average"), Level::Good);
    assert_eq!(level(&r, "not_cleared_recently"), Level::Good);
    assert_eq!(level(&r, "mil_distance"), Level::Info);
    assert_eq!(level(&r, "faults_active"), Level::Warning);
    assert_eq!(level(&r, "faults_previous"), Level::Info);
    let speed = r.facts.average_speed.unwrap();
    assert!((speed - 41.5).abs() < 0.1, "{speed}");
    assert!((r.facts.idle_share.unwrap() - 0.32).abs() < 0.01);
    assert!((r.facts.fuel_per_100km.unwrap() - 36.6).abs() < 0.1);
    assert_eq!(r.report.mileage.len(), 3);
    // Sections in page order, and the blind spots are named.
    let sections: Vec<Section> = r.report.items.iter().map(|i| i.section).collect();
    assert!(sections.windows(2).all(|w| w[0] <= w[1]));
    assert!(r.report.not_checked.len() >= 3);
    let english = with_language(Language::English, || {
        r.report.find("faults_active").unwrap().explanation.text()
    });
    assert!(
        english.starts_with("Engine (00): SPN 105 FMI 3, 7 times, SPN 111 FMI 17, 2 times."),
        "{english}"
    );
}

#[test]
fn a_turned_back_dashboard_is_red() {
    let mut input = demo();
    input.odometers[1].km -= 200_000.0;
    let r = evaluate_used_truck(&input);
    assert_eq!(r.report.light, TrafficLight::Red);
    assert_eq!(level(&r, "mileage_mismatch"), Level::Critical);
    // The higher reading is the one the hours are measured against.
    assert!((r.facts.odometer_km.unwrap() - 512_358.4).abs() < 0.01);
    let text = with_language(Language::English, || {
        r.report
            .find("mileage_mismatch")
            .unwrap()
            .explanation
            .text()
    });
    assert!(
        text.contains("The difference is 200,006 km") || text.contains("200006 km"),
        "{text}"
    );
    // A few hundred km between sources stay within the tolerance.
    let mut input = demo();
    input.odometers[2].km += 2_000.0;
    assert_eq!(
        level(&evaluate_used_truck(&input), "mileage_consistent"),
        Level::Good
    );
}

#[test]
fn hours_idle_and_fuel_against_the_kilometres() {
    let mut input = demo();
    input.engine_hours = Some(5_000.0);
    input.idle_hours = Some(2_500.0);
    let r = evaluate_used_truck(&input);
    assert_eq!(level(&r, "hours_too_few"), Level::Warning);
    assert_eq!(level(&r, "idle_share"), Level::Warning);
    let mut input = demo();
    input.engine_hours = Some(80_000.0);
    input.fuel_total_l = Some(40_000.0);
    let r = evaluate_used_truck(&input);
    assert_eq!(level(&r, "km_too_few"), Level::Warning);
    assert_eq!(level(&r, "fuel_average"), Level::Warning);
    let mut input = demo();
    input.engine_hours = None;
    let r = evaluate_used_truck(&input);
    assert!(r.report.find("engine_hours").is_none());
    assert!(r.facts.average_speed.is_none());
}

#[test]
fn vins_between_units_and_against_the_papers() {
    let mut input = demo();
    input.vins[2].vin = "YCXTR42H0MC099999".into();
    let r = evaluate_used_truck(&input);
    assert_eq!(level(&r, "vins_differ"), Level::Critical);
    let mut input = demo();
    input.papers_vin = VIN.to_lowercase();
    assert_eq!(
        level(&evaluate_used_truck(&input), "vin_matches_papers"),
        Level::Good
    );
    input.papers_vin = "YCXTR42H4NC051730".into();
    assert_eq!(
        level(&evaluate_used_truck(&input), "vin_differs_from_papers"),
        Level::Critical
    );
    input.vins.clear();
    assert_eq!(
        level(&evaluate_used_truck(&input), "vin_missing"),
        Level::Info
    );
}

#[test]
fn a_fresh_clearing_and_recurring_faults() {
    let mut input = demo();
    input.lamp_history = LampHistory {
        mil_km: Some(0.0),
        since_clear_km: Some(12.0),
        mil_minutes: Some(0.0),
        since_clear_minutes: Some(25.0),
    };
    input.faults[2].previous[0].occurrences = Some(14);
    let r = evaluate_used_truck(&input);
    assert_eq!(level(&r, "cleared_recently"), Level::Warning);
    assert!(r.report.find("mil_distance").is_none());
    assert_eq!(level(&r, "faults_previous"), Level::Warning);
    input.faults[0].lamps.red_stop = true;
    let r = evaluate_used_truck(&input);
    assert_eq!(level(&r, "lamp_red_stop"), Level::Critical);
    assert_eq!(r.report.light, TrafficLight::Red);
}
