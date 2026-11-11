//! Performance timer on synthetic runs with known answers.

use caruspex_analysis::performance_timer::{
    PerformanceReport, SpeedSample, analyze_performance, clean_speed_samples,
};

fn near(a: f64, b: f64, margin: f64) -> bool {
    (a - b).abs() <= margin
}

/// Standing for a second, then a constant 10 km/h per second (2.78 m/s^2)
/// up to 200 km/h, sampled at the given rate.
fn steady_launch(hz: f64, whole_kmh: bool) -> Vec<SpeedSample> {
    let mut s = Vec::new();
    let dt = 1.0 / hz;
    let standing = hz.round() as i32;
    for i in -standing..=0 {
        s.push(SpeedSample {
            t: f64::from(i) * dt,
            kmh: 0.0,
        });
    }
    let mut i = 1;
    while f64::from(i) * dt <= 20.0 + 1e-9 {
        let t = f64::from(i) * dt;
        let mut v = 10.0 * t;
        if whole_kmh {
            v = v.floor(); // OBD reports whole km/h
        }
        s.push(SpeedSample { t, kmh: v });
        i += 1;
    }
    s
}

fn report(s: Vec<SpeedSample>) -> PerformanceReport {
    analyze_performance(s)
}

#[test]
fn zero_to_100_zero_to_60_mph_80_120_and_quarter_mile_on_a_steady_launch() {
    let r = report(steady_launch(10.0, false));
    assert!(r.zero_to_100.found);
    assert!(near(r.zero_to_100.seconds, 10.0, 0.01));
    assert!(r.zero_to_100.reliable);
    assert!(near(r.zero_to_60_mph.seconds, 9.656, 0.01));
    assert!(near(r.eighty_to_120.seconds, 4.0, 0.01));
    // Distance 0.5 * a * t^2 with a = 2.778 m/s^2 reaches 402.3 m after 17.02 s.
    assert!(r.quarter_mile.found);
    assert!(near(r.quarter_mile.seconds, 17.02, 0.02));
    assert!(near(r.quarter_mile.trap_speed_kmh, 170.2, 0.2));
    assert!(near(r.sample_rate_hz, 10.0, 0.01));
    assert!(near(r.accuracy_seconds, 0.2, 1e-6));
    assert!(!r.brake_100_to_0.found);
}

#[test]
fn whole_kmh_at_5_hz_stays_within_the_stated_accuracy() {
    let r = report(steady_launch(5.0, true));
    assert!(r.zero_to_100.found);
    assert!((r.zero_to_100.seconds - 10.0).abs() <= r.accuracy_seconds);
    assert!((r.eighty_to_120.seconds - 4.0).abs() <= r.accuracy_seconds);
    assert!(r.accuracy_hint.de.contains("0,2 s"));
}

#[test]
fn braking_from_100_kmh() {
    // 110 km/h cruise, then 10 m/s^2 (36 km/h per second) to a standstill.
    // From 100 km/h (27.78 m/s) that takes 2.78 s and 38.6 m.
    let mut s = Vec::new();
    for i in 0..=10 {
        s.push(SpeedSample {
            t: f64::from(i) * 0.1,
            kmh: 110.0,
        });
    }
    let brake_start = 1.0;
    for i in 1..=50 {
        let t = brake_start + f64::from(i) * 0.1;
        s.push(SpeedSample {
            t,
            kmh: (110.0 - 36.0 * (t - brake_start)).max(0.0),
        });
    }
    let r = report(s);
    assert!(r.brake_100_to_0.found);
    assert!(near(r.brake_100_to_0.seconds, 2.78, 0.1));
    assert!(near(r.brake_100_to_0.distance_meters, 38.6, 0.6));
    assert!(!r.zero_to_100.found);
}

#[test]
fn runs_that_do_not_finish_are_not_reported() {
    // Up to 70 km/h and back to a stop: no 0-100, no braking from 100.
    let mut s = vec![
        SpeedSample { t: 0.0, kmh: 0.0 },
        SpeedSample { t: 1.0, kmh: 0.0 },
    ];
    for i in 1..=7 {
        s.push(SpeedSample {
            t: 1.0 + f64::from(i),
            kmh: f64::from(i) * 10.0,
        });
    }
    for i in 1..=7 {
        s.push(SpeedSample {
            t: 8.0 + f64::from(i),
            kmh: 70.0 - f64::from(i) * 10.0,
        });
    }
    let r = report(s);
    assert!(!r.zero_to_100.found);
    assert!(!r.quarter_mile.found);
    assert!(!r.brake_100_to_0.found);
    assert!(!r.eighty_to_120.found);
}

#[test]
fn spikes_duplicates_and_unsorted_samples_are_cleaned() {
    let mut s = steady_launch(10.0, false);
    s[40].kmh = 250.0; // a single glitch at 3 s
    s.push(s[70]); // a duplicate
    s.swap(20, 25); // out of order
    s.push(SpeedSample {
        t: -5.0,
        kmh: f64::NAN,
    });
    let (clean, dropped) = clean_speed_samples(s.clone());
    assert_eq!(dropped, 3);
    for w in clean.windows(2) {
        assert!(w[1].t > w[0].t);
    }

    let r = report(s);
    assert!(near(r.zero_to_100.seconds, 10.0, 0.01));
}

#[test]
fn slow_polling_is_called_out() {
    let r = report(steady_launch(2.0, false));
    assert!(near(r.sample_rate_hz, 2.0, 0.01));
    assert!(near(r.accuracy_seconds, 0.5, 1e-6));
    assert!(r.accuracy_hint.de.contains("mindestens 5"));
    assert!(r.accuracy_hint.en.contains("at least 5"));
}
