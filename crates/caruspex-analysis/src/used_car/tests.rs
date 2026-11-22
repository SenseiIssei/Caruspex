use super::*;
use crate::readiness::Monitor;

fn monitor(monitor: Monitor, complete: bool) -> MonitorState {
    MonitorState {
        monitor,
        supported: true,
        complete,
    }
}

fn fault(code: &str, kind: FaultKind, level: Level) -> UsedCarFault {
    UsedCarFault {
        code: code.into(),
        text: String::new(),
        kind,
        level,
    }
}

const DEMO_VIN: &str = "YCXDE15C7MC048217";

/// The demo car as the worker reads it before anything was cleared:
/// PID A6 48 213 km, the cluster and the ABS agree, 1842 km and 36
/// warm-ups since the last clearing, evaporative system and EGR open,
/// P0171 and P0420 stored, P0300 pending, the lamp on.
fn demo() -> UsedCarInput {
    UsedCarInput {
        car_vin: Some(VinFacts {
            vin: DEMO_VIN.into(),
            valid: true,
            model_year: 2021,
            vehicle: "Caruspex Demo".into(),
            year_from: 2019,
            year_to: 0,
            ..VinFacts::default()
        }),
        papers_vin: String::new(),
        odometer_km: Some(48213.4),
        other_mileage: vec![
            MileageReading {
                source: "Instrument cluster".into(),
                km: 48213.0,
            },
            MileageReading {
                source: "ABS".into(),
                km: 48210.0,
            },
        ],
        warm_ups: Some(36.0),
        distance_since_clear_km: Some(1842.0),
        minutes_since_clear: Some(2580.0),
        mil_distance_km: Some(312.0),
        mil_on: Some(true),
        readiness: vec![
            monitor(Monitor::Misfire, true),
            monitor(Monitor::FuelSystem, true),
            monitor(Monitor::Components, true),
            monitor(Monitor::Catalyst, true),
            monitor(Monitor::Evap, false),
            monitor(Monitor::OxygenSensor, true),
            monitor(Monitor::OxygenSensorHeater, true),
            monitor(Monitor::Egr, false),
        ],
        readiness_read: true,
        faults: vec![
            fault("P0171", FaultKind::Stored, Level::Warning),
            fault("P0420", FaultKind::Stored, Level::Warning),
            fault("P0300", FaultKind::Pending, Level::Info),
        ],
        faults_read: true,
        health: Some(HealthSummary {
            score: 58,
            grade: TrafficLight::Yellow,
            summary: "Needs care".into(),
        }),
        drive: Drive::Combustion,
        battery: None,
        current_year: 2026,
    }
}

fn ids(report: &UsedCarReport) -> Vec<&'static str> {
    report.items.iter().map(|i| i.id).collect()
}

#[test]
fn the_demo_car_needs_questions_but_nothing_is_hidden() {
    let report = evaluate_used_car(&demo());
    assert_eq!(report.light, TrafficLight::Yellow, "{:?}", ids(&report));
    assert_eq!(report.find("vehicle").unwrap().level, Level::Good);
    assert_eq!(
        report.find("mileage_consistent").unwrap().level,
        Level::Good
    );
    assert_eq!(report.find("mileage_fits_age").unwrap().level, Level::Good);
    assert_eq!(
        report.find("not_cleared_recently").unwrap().level,
        Level::Good
    );
    assert_eq!(report.find("readiness_open").unwrap().level, Level::Warning);
    assert_eq!(report.find("faults_stored").unwrap().level, Level::Warning);
    assert_eq!(report.find("faults_pending").unwrap().level, Level::Info);
    assert_eq!(report.find("mil_on").unwrap().level, Level::Warning);
    assert_eq!(report.find("mil_distance").unwrap().level, Level::Info);
    assert_eq!(report.find("health").unwrap().level, Level::Info);
    assert!(report.find("cleared_recently").is_none());
    // A petrol car: no battery section.
    assert!(report.items.iter().all(|i| i.section != Section::Battery));
    // The papers were not entered: listed as not checked.
    assert_eq!(report.not_checked.len(), 1);
    assert_eq!(report.not_checked[0].section, Section::Identity);
    // Three readings, engine first.
    assert_eq!(report.mileage.len(), 3);
    assert_eq!(report.mileage[0].km, 48213.4);
    // Questions: the most serious first, every one has a text.
    let questions = report.questions();
    assert!(questions.len() >= 4);
    assert!(questions.iter().all(|q| !q.text().is_empty()));
    // Items come in section order.
    let sections: Vec<Section> = report.items.iter().map(|i| i.section).collect();
    let mut sorted = sections.clone();
    sorted.sort();
    assert_eq!(sections, sorted);
}

#[test]
fn a_fresh_clearing_is_a_warning_and_permanent_codes_make_it_red() {
    let mut input = demo();
    input.warm_ups = Some(1.0);
    input.distance_since_clear_km = Some(3.0);
    input.minutes_since_clear = Some(12.0);
    input.mil_distance_km = Some(0.0);
    input.mil_on = Some(false);
    input.faults = vec![
        fault("P0171", FaultKind::Permanent, Level::Warning),
        fault("P0420", FaultKind::Permanent, Level::Warning),
    ];
    input.readiness = input
        .readiness
        .iter()
        .map(|m| MonitorState {
            complete: matches!(
                m.monitor,
                Monitor::Misfire | Monitor::FuelSystem | Monitor::Components
            ),
            ..*m
        })
        .collect();
    let report = evaluate_used_car(&input);
    assert_eq!(
        report.find("cleared_recently").unwrap().level,
        Level::Warning
    );
    assert_eq!(
        report.find("cleared_but_permanent").unwrap().level,
        Level::Critical
    );
    assert_eq!(
        report.find("faults_permanent").unwrap().level,
        Level::Warning
    );
    assert!(report.find("mil_distance").is_none());
    assert!(report.find("mil_on").is_none());
    let open = report.find("readiness_open").unwrap();
    assert_eq!(open.level, Level::Warning);
    assert_eq!(open.title.args[0], Arg::Number(5.0, 0));
    assert_eq!(report.light, TrafficLight::Red);
    // The critical question comes first.
    let first = report.questions()[0];
    assert_eq!(first, &report.find("cleared_but_permanent").unwrap().ask);

    // Each counter alone is enough for "recently".
    for (w, d, m) in [
        (Some(2.0), None, None),
        (None, Some(20.0), None),
        (None, None, Some(30.0)),
    ] {
        let mut one = demo();
        one.warm_ups = w;
        one.distance_since_clear_km = d;
        one.minutes_since_clear = m;
        let report = evaluate_used_car(&one);
        assert!(
            report.find("cleared_recently").is_some(),
            "{w:?} {d:?} {m:?}"
        );
    }
}

#[test]
fn mileage_that_does_not_match_is_red() {
    let mut input = demo();
    input.other_mileage[0].km = 131_870.0;
    let report = evaluate_used_car(&input);
    let item = report.find("mileage_mismatch").unwrap();
    assert_eq!(item.level, Level::Critical);
    assert!(!item.ask.is_empty());
    assert_eq!(report.light, TrafficLight::Red);

    // A few kilometres apart is normal: units store at different times.
    let mut close = demo();
    close.other_mileage[1].km = 48_150.0;
    assert!(
        evaluate_used_car(&close)
            .find("mileage_consistent")
            .is_some()
    );
    // The tolerance grows with the reading: 1 % of 200 000 km.
    let mut high = demo();
    high.odometer_km = Some(200_000.0);
    high.other_mileage = vec![MileageReading {
        source: "Cluster".into(),
        km: 198_500.0,
    }];
    assert!(
        evaluate_used_car(&high)
            .find("mileage_consistent")
            .is_some()
    );

    // More counted since clearing than the odometer shows.
    let mut turned_back = demo();
    turned_back.distance_since_clear_km = Some(60_000.0);
    let report = evaluate_used_car(&turned_back);
    assert_eq!(
        report.find("mileage_below_clear_distance").unwrap().level,
        Level::Warning
    );
}

#[test]
fn mileage_per_year_and_missing_sources() {
    let mut low = demo();
    low.odometer_km = Some(9000.0);
    low.other_mileage.clear();
    let report = evaluate_used_car(&low);
    assert!(report.find("mileage_single").is_some());
    assert_eq!(
        report.find("mileage_low_for_age").unwrap().level,
        Level::Info
    );

    let mut high = demo();
    high.odometer_km = Some(300_000.0);
    high.other_mileage.clear();
    assert!(
        evaluate_used_car(&high)
            .find("mileage_high_for_age")
            .is_some()
    );

    let mut none = demo();
    none.odometer_km = None;
    none.other_mileage.clear();
    let report = evaluate_used_car(&none);
    let item = report.find("mileage_unknown").unwrap();
    assert_eq!(item.level, Level::Info);
    assert!(!item.ask.is_empty());
    assert!(report.mileage.is_empty());
    assert!(report.find("mileage_fits_age").is_none());
}

#[test]
fn vin_against_the_papers() {
    let mut same = demo();
    same.papers_vin = "ycx de15c7 mc048217".into();
    let report = evaluate_used_car(&same);
    assert_eq!(
        report.find("vin_matches_papers").unwrap().level,
        Level::Good
    );
    assert!(report.not_checked.is_empty());

    let mut other = demo();
    other.papers_vin = "WVWZZZ1JZ3W000001".into();
    let report = evaluate_used_car(&other);
    assert_eq!(
        report.find("vin_differs_from_papers").unwrap().level,
        Level::Critical
    );
    assert_eq!(report.light, TrafficLight::Red);

    let mut typo = demo();
    typo.papers_vin = "YCXDE15C7MC04821".into();
    assert_eq!(
        evaluate_used_car(&typo)
            .find("papers_vin_invalid")
            .unwrap()
            .level,
        Level::Info
    );

    let mut silent = demo();
    silent.car_vin = None;
    let report = evaluate_used_car(&silent);
    assert!(report.find("vin_missing").is_some());
    assert!(report.find("vehicle").is_none());
    assert!(report.find("mileage_fits_age").is_none());

    let mut garbage = demo();
    garbage.car_vin = Some(VinFacts {
        vin: "000000000000".into(),
        ..VinFacts::default()
    });
    assert_eq!(
        evaluate_used_car(&garbage)
            .find("vin_invalid")
            .unwrap()
            .level,
        Level::Warning
    );

    let mut digit = demo();
    if let Some(v) = digit.car_vin.as_mut() {
        v.check_digit_applies = true;
        v.check_digit_ok = false;
    }
    assert!(evaluate_used_car(&digit).find("vin_check_digit").is_some());
}

#[test]
fn no_readiness_and_unread_parts() {
    let mut none = demo();
    none.readiness.clear();
    let report = evaluate_used_car(&none);
    assert_eq!(report.find("readiness_none").unwrap().level, Level::Info);

    let mut unread = demo();
    unread.readiness_read = false;
    unread.faults_read = false;
    unread.warm_ups = None;
    unread.distance_since_clear_km = None;
    unread.minutes_since_clear = None;
    unread.health = None;
    let report = evaluate_used_car(&unread);
    let skipped: Vec<Section> = report.not_checked.iter().map(|n| n.section).collect();
    assert!(skipped.contains(&Section::Readiness));
    assert!(skipped.contains(&Section::Faults));
    assert!(skipped.contains(&Section::Clearing));
    assert!(report.find("health").is_none());
    assert!(report.find("no_faults").is_none());

    let mut complete = demo();
    for m in &mut complete.readiness {
        m.complete = true;
    }
    complete.faults.clear();
    complete.mil_on = Some(false);
    complete.mil_distance_km = Some(0.0);
    complete.health = Some(HealthSummary {
        score: 96,
        grade: TrafficLight::Green,
        summary: String::new(),
    });
    let report = evaluate_used_car(&complete);
    assert!(report.find("readiness_complete").is_some());
    assert!(report.find("no_faults").is_some());
    assert_eq!(report.light, TrafficLight::Green);
}

#[test]
fn electric_cars_get_the_battery_checked() {
    let mut ev = demo();
    ev.drive = Drive::Electric;
    ev.readiness.clear();
    ev.faults.clear();
    ev.mil_on = Some(false);
    ev.mil_distance_km = None;
    ev.health = None;
    ev.battery = Some(BatteryFacts {
        pack: "Hyundai Kona Electric".into(),
        read: true,
        soh: Some(91.0),
        soc: Some(64.0),
        cell_spread_v: Some(0.02),
        ..BatteryFacts::default()
    });
    let report = evaluate_used_car(&ev);
    assert_eq!(
        report.find("readiness_none_electric").unwrap().level,
        Level::Good
    );
    assert_eq!(report.find("battery_good").unwrap().level, Level::Good);
    assert_eq!(report.light, TrafficLight::Green);

    let mut worn = ev.clone();
    if let Some(b) = worn.battery.as_mut() {
        b.soh = Some(76.0);
        b.cell_spread_v = Some(0.15);
    }
    let report = evaluate_used_car(&worn);
    assert_eq!(report.find("battery_worn").unwrap().level, Level::Warning);
    assert!(report.find("battery_cells_uneven").is_some());
    assert_eq!(report.light, TrafficLight::Yellow);

    let mut dead = ev.clone();
    if let Some(b) = dead.battery.as_mut() {
        b.soh = Some(64.0);
    }
    assert_eq!(evaluate_used_car(&dead).light, TrafficLight::Red);

    let mut silent = ev.clone();
    silent.battery = Some(BatteryFacts {
        pack: "Renault Zoe".into(),
        read: false,
        error: "NO DATA".into(),
        ..BatteryFacts::default()
    });
    assert!(evaluate_used_car(&silent).find("battery_unread").is_some());

    let mut unknown = ev.clone();
    unknown.drive = Drive::Hybrid;
    unknown.battery = None;
    let report = evaluate_used_car(&unknown);
    assert!(!report.find("battery_unknown").unwrap().ask.is_empty());

    assert_eq!(drive_from_fuel_type(8), Drive::Electric);
    assert_eq!(drive_from_fuel_type(0x11), Drive::Hybrid);
    assert_eq!(drive_from_fuel_type(1), Drive::Combustion);
    assert_eq!(drive_from_fuel_type(0), Drive::Unknown);
}

#[test]
fn vins_are_normalised_and_checked() {
    assert_eq!(normalise_vin(" wvw-zzz1jz 3w000001 "), "WVWZZZ1JZ3W000001");
    assert!(vin_well_formed("WVWZZZ1JZ3W000001"));
    assert!(!vin_well_formed("WVWZZZ1JZ3W00000O"));
}
