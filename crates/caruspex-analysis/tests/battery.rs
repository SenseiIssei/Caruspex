//! Guided battery test: thresholds with concrete voltages.

use caruspex_analysis::battery_test::{
    BatteryInput, BatteryReport, BatteryStepResult, BatteryType, VoltageSample, battery_test_steps,
    estimate_state_of_charge, evaluate_battery,
};
use caruspex_analysis::common::Level;

fn step<'a>(r: &'a BatteryReport, id: &str) -> Option<&'a BatteryStepResult> {
    r.steps.iter().find(|s| s.id == id)
}

fn resting_level(v: f64, battery_type: BatteryType) -> Level {
    let input = BatteryInput {
        battery_type,
        resting_volts: Some(v),
        minutes_since_engine_off: Some(240.0),
        ..Default::default()
    };
    step(&evaluate_battery(&input), "resting").unwrap().level
}

fn idle_level(v: f64, start_stop: bool) -> Level {
    let input = BatteryInput {
        start_stop,
        idle_volts: Some(v),
        ..Default::default()
    };
    step(&evaluate_battery(&input), "charging_idle")
        .unwrap()
        .level
}

#[test]
fn resting_voltage_and_state_of_charge() {
    use BatteryType::{Agm, Flooded};
    assert_eq!(estimate_state_of_charge(12.65, Flooded), 100);
    assert_eq!(estimate_state_of_charge(12.4, Flooded), 75);
    assert_eq!(estimate_state_of_charge(12.3, Flooded), 65); // 62.5, in steps of 5
    assert_eq!(estimate_state_of_charge(12.2, Flooded), 50);
    assert_eq!(estimate_state_of_charge(11.5, Flooded), 0);
    assert_eq!(estimate_state_of_charge(12.5, Agm), 75); // AGM sits 0.1 V higher

    assert_eq!(resting_level(12.65, Flooded), Level::Good);
    assert_eq!(resting_level(12.45, Flooded), Level::Good);
    assert_eq!(resting_level(12.3, Flooded), Level::Info);
    assert_eq!(resting_level(12.0, Flooded), Level::Warning);
    assert_eq!(resting_level(11.7, Flooded), Level::Critical);
    assert_eq!(resting_level(12.65, Agm), Level::Good);
    assert_eq!(resting_level(12.35, Agm), Level::Info);
}

#[test]
fn surface_charge_right_after_driving_is_pointed_out() {
    let input = BatteryInput {
        resting_volts: Some(12.85),
        minutes_since_engine_off: Some(5.0),
        ..Default::default()
    };
    let r = evaluate_battery(&input);
    assert_eq!(r.state_of_charge_percent, Some(100));
    let text = &step(&r, "resting").unwrap().explanation.de;
    assert!(text.contains("Oberflächenladung"));
    assert!(text.contains("12,85 V"));
}

fn crank(lowest: f64, hz: f64) -> BatteryReport {
    let mut input = BatteryInput::default();
    let dt = 1.0 / hz;
    for i in 0..((2.0 * hz) as i32) {
        let t = f64::from(i) * dt;
        // 12.5 V, a short dip while the starter turns, then charging.
        let v = if t < 0.19 {
            12.5
        } else if t < 0.5 {
            lowest
        } else {
            14.0
        };
        input.cranking.push(VoltageSample { t, volts: v });
    }
    evaluate_battery(&input)
}

#[test]
fn cranking_dip() {
    let good = crank(10.4, 50.0);
    assert_eq!(step(&good, "cranking").unwrap().level, Level::Good);
    assert_eq!(step(&good, "cranking").unwrap().volts, 10.4);
    assert!(good.cranking_sample_rate_hz > 40.0);
    assert_eq!(
        step(&crank(9.8, 50.0), "cranking").unwrap().level,
        Level::Info
    );
    assert_eq!(
        step(&crank(9.3, 50.0), "cranking").unwrap().level,
        Level::Warning
    );
    assert_eq!(
        step(&crank(8.0, 50.0), "cranking").unwrap().level,
        Level::Critical
    );

    // At 5 readings per second the minimum may be missed; the text says so.
    let slow = crank(9.3, 5.0);
    assert!(
        step(&slow, "cranking")
            .unwrap()
            .explanation
            .en
            .contains("real minimum")
    );

    // A "dip" that hardly leaves the resting voltage was not captured.
    let missed = BatteryInput {
        resting_volts: Some(12.6),
        cranking_minimum_volts: Some(12.45),
        ..Default::default()
    };
    assert_eq!(
        step(&evaluate_battery(&missed), "cranking")
            .unwrap()
            .title
            .en,
        "No dip measured while cranking"
    );
}

#[test]
fn charging_voltage_with_and_without_smart_charging() {
    assert_eq!(idle_level(14.2, false), Level::Good);
    assert_eq!(idle_level(13.5, false), Level::Info);
    assert_eq!(idle_level(13.5, true), Level::Good);
    assert_eq!(idle_level(13.0, false), Level::Warning);
    assert_eq!(idle_level(13.0, true), Level::Info); // smart charging may pause
    assert_eq!(idle_level(12.4, false), Level::Critical);
    assert_eq!(idle_level(14.9, false), Level::Info);
    assert_eq!(idle_level(14.9, true), Level::Good);
    assert_eq!(idle_level(15.2, false), Level::Warning);
    assert_eq!(idle_level(15.7, false), Level::Critical);

    let mut load = BatteryInput {
        idle_volts: Some(14.2),
        loaded_volts: Some(12.9),
        ..Default::default()
    };
    let r = evaluate_battery(&load);
    assert_eq!(step(&r, "charging_load").unwrap().level, Level::Warning);
    assert!(
        step(&r, "charging_load")
            .unwrap()
            .explanation
            .de
            .contains("1,30 V weniger")
    );

    load.loaded_volts = Some(13.9);
    assert_eq!(
        step(&evaluate_battery(&load), "charging_load")
            .unwrap()
            .level,
        Level::Good
    );
    load.loaded_volts = Some(13.5); // 0.7 V drop: still fine but noticeable
    assert_eq!(
        step(&evaluate_battery(&load), "charging_load")
            .unwrap()
            .level,
        Level::Info
    );
}

#[test]
fn overall_verdict_and_missing_steps() {
    let input = BatteryInput {
        resting_volts: Some(12.6),
        minutes_since_engine_off: Some(120.0),
        idle_volts: Some(14.1),
        ..Default::default()
    };
    let r = evaluate_battery(&input);
    assert_eq!(r.overall, Level::Good);
    assert_eq!(
        r.missing_steps,
        vec!["cranking".to_string(), "charging_load".to_string()]
    );
    assert!(r.summary.de.contains("2 von 4 Schritten"));

    let empty = evaluate_battery(&BatteryInput::default());
    assert!(empty.steps.is_empty());
    assert_eq!(empty.missing_steps.len(), 4);

    assert_eq!(battery_test_steps().len(), 4);
    for s in battery_test_steps() {
        assert!(!s.instruction.de.contains('\u{2014}'));
        assert!(!s.instruction.en.contains('\u{2014}'));
    }
}
