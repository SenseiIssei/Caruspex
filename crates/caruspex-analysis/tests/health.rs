#![allow(clippy::field_reassign_with_default)]
//! Health check rules with concrete numbers: each test builds the input a car
//! would deliver and checks the finding that comes out.
//!
//! The test "Phrases format numbers per language" lives with the code it
//! tests, in `common.rs`.

use std::sync::Arc;

use caruspex_analysis::common::FuelType;
use caruspex_analysis::common::{Level, TrafficLight};
use caruspex_analysis::health_check::{HealthInput, HealthReport, run_health_check};
use caruspex_analysis::inputs::{Fault, MonitorResult};
use caruspex_analysis::readiness::decode_readiness;

fn is_skipped(r: &HealthReport, id: &str) -> bool {
    r.not_checked.iter().any(|s| s.id == id)
}

/// Steps `t` from `from` to `to` inclusive, like the C++ `for (t = from; t <= to; t += step)`.
fn times(from: f64, to: f64, step: f64) -> Vec<f64> {
    let mut v = Vec::new();
    let mut t = from;
    while t <= to {
        v.push(t);
        t += step;
    }
    v
}

/// Warm engine in closed loop at the given rpm, one reading per second.
fn warm_closed_loop(input: &mut HealthInput, from: f64, to: f64, rpm: f64) {
    for t in times(from, to, 1.0) {
        input.add_value(0x03, 2.0, t);
        input.add_value(0x05, 90.0, t);
        input.add_value(0x0C, rpm, t);
    }
}

fn trims(input: &mut HealthInput, from: f64, to: f64, st: f64, lt: f64) {
    for t in times(from, to, 1.0) {
        input.add_value(0x06, st, t);
        input.add_value(0x07, lt, t);
    }
}

#[test]
fn fuel_trims_lean_rich_and_fine() {
    // +8 and +7 make +15: lean warning
    {
        let mut input = HealthInput::default();
        warm_closed_loop(&mut input, 0.0, 30.0, 800.0);
        trims(&mut input, 0.0, 30.0, 8.0, 7.0);
        let r = run_health_check(&input);
        let f = r.find("fuel_trim_bank1").unwrap();
        assert_eq!(f.level, Level::Warning);
        assert!(f.title.de.contains("mager"));
        assert!(f.explanation.de.contains("+15,0 %"));
        assert!(f.explanation.en.contains("+15.0%"));
    }
    // -12 and -10 make -22: rich, critical
    {
        let mut input = HealthInput::default();
        warm_closed_loop(&mut input, 0.0, 30.0, 800.0);
        trims(&mut input, 0.0, 30.0, -12.0, -10.0);
        let report = run_health_check(&input);
        let f = report.find("fuel_trim_bank1").unwrap();
        assert_eq!(f.level, Level::Critical);
        assert!(f.title.en.contains("rich"));
    }
    // +3 and +1 is normal
    {
        let mut input = HealthInput::default();
        warm_closed_loop(&mut input, 0.0, 30.0, 800.0);
        trims(&mut input, 0.0, 30.0, 3.0, 1.0);
        let report = run_health_check(&input);
        let f = report.find("fuel_trim_bank1").unwrap();
        assert_eq!(f.level, Level::Good);
    }
    // lean at idle only points to an air leak
    {
        let mut input = HealthInput::default();
        warm_closed_loop(&mut input, 0.0, 19.0, 800.0); // 20 s idle
        trims(&mut input, 0.0, 19.0, 9.0, 7.0); // +16
        warm_closed_loop(&mut input, 20.0, 24.0, 2500.0); // 5 s at 2500
        trims(&mut input, 20.0, 24.0, 1.0, 1.0); // +2
        let report = run_health_check(&input);
        let f = report.find("fuel_trim_bank1").unwrap();
        assert_eq!(f.level, Level::Warning); // (20*16 + 5*2)/25 = 13.2
        assert!(f.explanation.de.contains("spricht stark für Falschluft"));
    }
    // open loop readings are not judged
    {
        let mut input = HealthInput::default();
        for t in times(0.0, 30.0, 1.0) {
            input.add_value(0x03, 1.0, t); // open loop, engine cold
            input.add_value(0x05, 30.0, t);
            input.add_value(0x0C, 1200.0, t);
        }
        trims(&mut input, 0.0, 30.0, 15.0, 10.0);
        let r = run_health_check(&input);
        assert!(r.find("fuel_trim_bank1").is_none());
        assert!(is_skipped(&r, "fuel_trim_bank1"));
    }
}

#[test]
fn charging_and_battery_voltage() {
    let run = |rpm: f64, volts: f64| {
        let mut input = HealthInput::default();
        for t in times(0.0, 9.0, 1.0) {
            input.add_value(0x0C, rpm, t);
            input.add_value(0x42, volts, t);
        }
        run_health_check(&input)
    };
    assert_eq!(
        run(800.0, 14.1).find("charging").unwrap().level,
        Level::Good
    );
    assert_eq!(
        run(800.0, 12.9).find("charging").unwrap().level,
        Level::Warning
    );
    assert_eq!(
        run(800.0, 12.3).find("charging").unwrap().level,
        Level::Critical
    );
    assert_eq!(
        run(800.0, 15.2).find("charging").unwrap().level,
        Level::Warning
    );
    assert_eq!(
        run(800.0, 15.8).find("charging").unwrap().level,
        Level::Critical
    );
    assert_eq!(
        run(0.0, 12.0).find("battery_rest").unwrap().level,
        Level::Warning
    );
    assert_eq!(
        run(0.0, 11.6).find("battery_rest").unwrap().level,
        Level::Critical
    );
    assert_eq!(
        run(0.0, 12.5).find("battery_rest").unwrap().level,
        Level::Good
    );

    let warn = run(800.0, 12.9);
    assert!(
        warn.find("charging")
            .unwrap()
            .explanation
            .de
            .contains("12,90 V")
    );
    // Smart charging is mentioned so nobody replaces a good alternator.
    assert!(
        warn.find("charging")
            .unwrap()
            .explanation
            .en
            .contains("smart charging")
    );

    // Without engine speed it is unclear whether the engine runs.
    let mut no_rpm = HealthInput::default();
    no_rpm.add_value(0x42, 14.0, 0.0);
    let r = run_health_check(&no_rpm);
    assert!(r.find("charging").is_none());
    assert!(is_skipped(&r, "charging"));
}

#[test]
fn thermostat_and_overheating() {
    // 20 minutes and only 68 C: stuck open
    {
        let mut input = HealthInput::default();
        input.add_value(0x1F, 1200.0, 0.0);
        for t in times(0.0, 59.0, 5.0) {
            input.add_value(0x05, 60.0 + t / 10.0, t); // up to 65.5
        }
        input.add_value(0x05, 68.0, 60.0);
        let report = run_health_check(&input);
        let f = report.find("thermostat").unwrap();
        assert_eq!(f.level, Level::Warning);
        assert!(f.explanation.de.contains("68 °C"));
        assert!(f.explanation.de.contains("20 Minuten"));
    }
    // 5 minutes and 60 C: too early to tell
    {
        let mut input = HealthInput::default();
        input.add_value(0x1F, 300.0, 0.0);
        input.add_value(0x05, 60.0, 0.0);
        let r = run_health_check(&input);
        assert!(r.find("thermostat").is_none());
        assert!(is_skipped(&r, "thermostat"));
    }
    // 90 C is fine, 117 C is overheating
    {
        let mut ok = HealthInput::default();
        ok.add_value(0x05, 90.0, 0.0);
        assert_eq!(
            run_health_check(&ok).find("thermostat").unwrap().level,
            Level::Good
        );

        let mut hot = HealthInput::default();
        hot.add_value(0x05, 117.0, 0.0);
        assert_eq!(
            run_health_check(&hot).find("overheating").unwrap().level,
            Level::Critical
        );
    }
    // -40 C means a broken sensor
    {
        let mut input = HealthInput::default();
        input.add_value(0x05, -40.0, 0.0);
        assert_eq!(
            run_health_check(&input)
                .find("sensor_coolant")
                .unwrap()
                .level,
            Level::Warning
        );
    }
}

#[test]
fn readiness_and_emissions_test_guidance() {
    // petrol with only EVAP open: info
    {
        let mut input = HealthInput {
            fuel: FuelType::Petrol,
            ..Default::default()
        };
        input.readiness = decode_readiness(0x00, 0x07, 0x65, 0x04).monitors;
        input.readiness_read = true;
        let report = run_health_check(&input);
        let f = report.find("readiness").unwrap();
        assert_eq!(f.level, Level::Info);
        assert!(f.explanation.de.contains("Tankentlüftung"));
        assert!(f.advice.en.contains("EPA"));
    }
    // petrol with two open: warning
    {
        let mut input = HealthInput::default();
        input.readiness = decode_readiness(0x00, 0x07, 0x65, 0x05).monitors;
        input.readiness_read = true;
        assert_eq!(
            run_health_check(&input).find("readiness").unwrap().level,
            Level::Warning
        );
    }
    // diesel with one open: warning
    {
        let mut input = HealthInput::default();
        input.readiness = decode_readiness(0x00, 0x0F, 0xE9, 0x40).monitors;
        input.readiness_read = true;
        let report = run_health_check(&input);
        let f = report.find("readiness").unwrap();
        assert_eq!(f.level, Level::Warning);
        assert!(f.explanation.en.contains("Particulate filter"));
    }
    // all complete
    {
        let mut input = HealthInput::default();
        input.readiness = decode_readiness(0x00, 0x07, 0x65, 0x00).monitors;
        input.readiness_read = true;
        let report = run_health_check(&input);
        let f = report.find("readiness").unwrap();
        assert_eq!(f.level, Level::Good);
        assert!(f.explanation.de.contains("7 von 7"));
    }
}

#[test]
fn recently_cleared_codes() {
    let mut fresh = HealthInput::default();
    fresh.add_value(0x30, 2.0, 0.0);
    fresh.add_value(0x31, 12.0, 0.0);
    fresh.readiness = decode_readiness(0x00, 0x07, 0x65, 0x25).monitors;
    fresh.readiness_read = true;
    let report = run_health_check(&fresh);
    let f = report.find("cleared_recently").unwrap();
    assert_eq!(f.level, Level::Warning);
    assert!(f.explanation.de.contains("2 Warmläufe und 12 km"));
    assert!(f.explanation.en.contains("incomplete readiness monitors"));

    let mut old = HealthInput::default();
    old.add_value(0x30, 40.0, 0.0);
    old.add_value(0x31, 1500.0, 0.0);
    assert_eq!(
        run_health_check(&old)
            .find("cleared_recently")
            .unwrap()
            .level,
        Level::Good
    );

    let mut lamp = HealthInput::default();
    lamp.add_value(0x21, 230.0, 0.0);
    let lamp_report = run_health_check(&lamp);
    let d = lamp_report.find("mil_distance").unwrap();
    assert!(d.title.de.contains("230 km"));
}

#[test]
fn oxygen_sensors_and_catalyst() {
    let pi = std::f64::consts::PI;
    let base = |input: &mut HealthInput| {
        // 30 s warm closed loop at 4 readings per second.
        for t in times(0.0, 30.0, 0.25) {
            input.add_value(0x03, 2.0, t);
            input.add_value(0x05, 90.0, t);
            input.add_value(0x0C, 2000.0, t);
        }
    };
    // switching front sensor, calm rear sensor
    {
        let mut input = HealthInput::default();
        base(&mut input);
        for t in times(0.0, 30.0, 0.25) {
            input.add_value(0x14, 0.45 + 0.4 * (2.0 * pi * 0.5 * t + 0.3).sin(), t);
            input.add_value(0x15, 0.65, t);
        }
        let r = run_health_check(&input);
        assert_eq!(r.find("o2_switching_bank1").unwrap().level, Level::Good);
        assert_eq!(r.find("catalyst_bank1").unwrap().level, Level::Good);
    }
    // rear sensor copies the front one: worn catalyst
    {
        let mut input = HealthInput::default();
        base(&mut input);
        for t in times(0.0, 30.0, 0.25) {
            let v = 0.45 + 0.4 * (2.0 * pi * 0.5 * t + 0.3).sin();
            input.add_value(0x14, v, t);
            input.add_value(0x15, v, t);
        }
        let report = run_health_check(&input);
        let f = report.find("catalyst_bank1").unwrap();
        assert_eq!(f.level, Level::Warning);
    }
    // front sensor stuck lean
    {
        let mut input = HealthInput::default();
        base(&mut input);
        for t in times(0.0, 30.0, 0.25) {
            input.add_value(0x14, 0.06, t);
        }
        let report = run_health_check(&input);
        let f = report.find("o2_switching_bank1").unwrap();
        assert_eq!(f.level, Level::Warning);
        assert!(f.explanation.de.contains("dauernd mager"));
    }
}

#[test]
fn mode_06_limits_and_misfire_counters() {
    let result = |tid: i32, cid: i32, value: f64, min: f64, max: f64| MonitorResult {
        test_id: tid,
        component_id: cid,
        name: "Test".to_string(),
        value,
        minimum: min,
        maximum: max,
        has_minimum: true,
        has_maximum: true,
        passed: value >= min && value <= max,
        ..Default::default()
    };
    let input = HealthInput {
        monitor_results_read: true,
        monitor_results: vec![
            result(0x81, 0x21, 0.95, 0.0, 1.0), // 5 % below the limit: close
            result(0x82, 0x21, 0.50, 0.0, 1.0), // far away: fine
            result(0x85, 0x01, 1.30, 0.0, 1.2), // failed
            result(0x86, 0x01, 0.0, 0.0, 0.0),  // not run yet: ignored
            result(0x0C, 0xA2, 2.0, 0.0, 65535.0), // cylinder 1
            result(0x0C, 0xA3, 1.0, 0.0, 65535.0), // cylinder 2
            result(0x0C, 0xA4, 40.0, 0.0, 65535.0), // cylinder 3
            result(0x0C, 0xA5, 0.0, 0.0, 65535.0), // cylinder 4
        ],
        ..Default::default()
    };
    let r = run_health_check(&input);
    assert_eq!(r.find("mode06:21:81").unwrap().level, Level::Info);
    assert!(r.find("mode06:21:82").is_none());
    assert_eq!(r.find("mode06:01:85").unwrap().level, Level::Warning);
    assert!(r.find("mode06:01:86").is_none());
    let m = r.find("misfire").unwrap();
    assert_eq!(m.level, Level::Warning);
    assert!(m.explanation.de.contains("Zylinder 3: 40"));
    assert!(m.explanation.en.contains("cylinder 3. That points"));

    let none = HealthInput {
        monitor_results_read: true,
        monitor_results: vec![
            result(0x0B, 0xA2, 0.0, 0.0, 65535.0),
            result(0x0B, 0xA3, 0.0, 0.0, 65535.0),
        ],
        ..Default::default()
    };
    assert_eq!(
        run_health_check(&none).find("misfire").unwrap().level,
        Level::Good
    );
}

#[test]
fn score_and_traffic_light() {
    // nothing read: everything is listed as not checked
    {
        let r = run_health_check(&HealthInput::default());
        assert!(r.findings.is_empty());
        assert_eq!(r.score, 100);
        assert!(is_skipped(&r, "faults"));
        assert!(is_skipped(&r, "readiness"));
        assert!(is_skipped(&r, "charging"));
        assert!(is_skipped(&r, "mode06"));
    }
    // clean car
    {
        let mut input = HealthInput {
            faults_read: true,
            mil_on: Some(false),
            ..Default::default()
        };
        input.readiness = decode_readiness(0x00, 0x07, 0x65, 0x00).monitors;
        input.readiness_read = true;
        let r = run_health_check(&input);
        assert_eq!(r.score, 100);
        assert_eq!(r.grade, TrafficLight::Green);
        assert_eq!(r.checks_run, 3);
    }
    // lamp on: yellow and at most 84
    {
        let input = HealthInput {
            mil_on: Some(true),
            ..Default::default()
        };
        let r = run_health_check(&input);
        assert_eq!(r.grade, TrafficLight::Yellow);
        assert!(r.score <= 84);
    }
    // the severity of a fault comes from the caller
    {
        let fault = |code: &str, kind: &str, text: &str| Fault {
            code: code.to_string(),
            kind: kind.to_string(),
            text: text.to_string(),
            throttle_related: false,
        };
        let input = HealthInput {
            faults_read: true,
            faults: vec![
                fault("P0301", "stored", "Cylinder 1 misfire"),
                fault("P0442", "pending", "EVAP small leak"),
            ],
            fault_level: Some(Arc::new(|f: &Fault| {
                if f.code == "P0301" {
                    Level::Critical
                } else {
                    Level::Info
                }
            })),
            ..Default::default()
        };
        let r = run_health_check(&input);
        assert_eq!(r.grade, TrafficLight::Red);
        assert!(r.score <= 49);
        assert_eq!(r.findings[0].id, "fault:P0301"); // worst first
        assert_eq!(r.find("fault:P0442").unwrap().level, Level::Info);
        assert!(
            r.find("fault:P0442")
                .unwrap()
                .explanation
                .en
                .contains("pending")
        );
    }
}

#[test]
fn no_em_dashes_in_health_check_texts() {
    let mut input = HealthInput {
        faults_read: true,
        mil_on: Some(true),
        ..Default::default()
    };
    input.readiness = decode_readiness(0x00, 0x07, 0x65, 0x25).monitors;
    input.readiness_read = true;
    input.add_value(0x30, 1.0, 0.0);
    input.add_value(0x42, 12.0, 0.0);
    input.add_value(0x0C, 0.0, 0.0);
    input.add_value(0x05, 120.0, 0.0);
    let r = run_health_check(&input);
    let em_dash = '\u{2014}';
    for f in &r.findings {
        for p in [&f.title, &f.explanation, &f.advice] {
            assert!(!p.de.contains(em_dash));
            assert!(!p.en.contains(em_dash));
        }
    }
    assert!(!r.summary.de.contains(em_dash));
}
