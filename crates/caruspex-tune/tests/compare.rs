// The log comparison with synthetic recordings: a "car" that behaves exactly
// like the model, one with more boost than the model, and part-load samples
// that must stay out. There is no real N55 log yet.

use caruspex_tune::{Conditions, LogSample, Maps, compare_log, curve};

/// Full-load samples of a car that matches the stock model exactly, at the
/// grid points from 1,500 to 6,500 rpm.
fn model_car(boost_factor: f64) -> Vec<LogSample> {
    curve(&Maps::stock(), &Conditions::default())
        .iter()
        .filter(|p| p.rpm >= 1500.0 && p.rpm <= 6500.0)
        .enumerate()
        .map(|(i, p)| LogSample {
            t: i as f64 * 0.25,
            rpm: p.rpm,
            load_pct: Some(100.0),
            absolute_load_pct: None,
            throttle_pct: Some(99.0),
            manifold_kpa: Some(p.manifold_kpa * boost_factor),
            intake_c: Some(p.charge_air_c),
            timing_deg: Some(p.timing_deg),
            lambda: Some(p.lambda),
            maf_g_s: Some(p.air_mass_kg_h / 3.6),
            baro_kpa: Some(101.325),
            ambient_c: Some(20.0),
        })
        .collect()
}

#[test]
fn a_car_that_behaves_like_the_model_shows_no_deviation() {
    let samples = model_car(1.0);
    let result = compare_log(&samples, &Maps::stock(), &Conditions::default());
    assert_eq!(result.total, samples.len());
    assert_eq!(result.used, samples.len());
    assert_eq!(result.part_load, 0);
    assert!(!result.bands.is_empty());
    for band in &result.bands {
        for pair in [band.manifold_kpa, band.air_g_s] {
            let d = pair.deviation_pct().expect("both sides");
            assert!(d.abs() < 0.5, "{}-{}: {d:.2} %", band.from_rpm, band.to_rpm);
        }
        assert!(band.charge_air_c.difference().unwrap().abs() < 0.5);
        assert!(band.timing_deg.difference().unwrap().abs() < 0.2);
        assert!(band.lambda.difference().unwrap().abs() < 0.01);
    }
}

#[test]
fn more_boost_on_the_car_shows_up_as_a_negative_deviation() {
    let samples = model_car(1.10);
    let result = compare_log(&samples, &Maps::stock(), &Conditions::default());
    for band in &result.bands {
        // The model predicts stock boost: about 9 % below the measured value.
        let d = band.manifold_kpa.deviation_pct().unwrap();
        assert!((-10.5..=-7.5).contains(&d), "{}: {d:.2} %", band.from_rpm);
        // At the measured pressure the model expects more air than the car
        // reported (the MAF values were not raised): the model explains the
        // measured state differently, and the table shows it.
        assert!(band.air_g_s.deviation_pct().unwrap() > 5.0);
        // More pressure lowers the knock limit, but 10 % more boost stays
        // inside the reserve of the stock ignition map: never earlier.
        assert!(band.timing_deg.difference().unwrap() <= 1e-9);
    }
}

#[test]
fn part_load_and_out_of_range_samples_stay_out() {
    let mut samples = model_car(1.0);
    let full = samples.len();
    let mut idle = samples[0].clone();
    idle.rpm = 750.0;
    samples.push(idle);
    let mut cruise = samples[3].clone();
    cruise.load_pct = Some(35.0);
    cruise.throttle_pct = Some(18.0);
    samples.push(cruise);
    let result = compare_log(&samples, &Maps::stock(), &Conditions::default());
    assert_eq!(result.used, full);
    assert_eq!(result.part_load, 1);
    assert_eq!(result.out_of_range, 1);
}

#[test]
fn without_load_values_clear_boost_counts_as_full_load() {
    let sample = LogSample {
        rpm: 3000.0,
        manifold_kpa: Some(175.0),
        baro_kpa: Some(100.0),
        ..LogSample::default()
    };
    assert!(sample.full_load());
    let low = LogSample {
        manifold_kpa: Some(110.0),
        ..sample.clone()
    };
    assert!(!low.full_load());
}

#[test]
fn samples_are_grouped_in_500_rpm_bands() {
    let mut samples = model_car(1.0);
    samples.retain(|s| s.rpm >= 3000.0 && s.rpm < 4000.0);
    let result = compare_log(&samples, &Maps::stock(), &Conditions::default());
    let bands: Vec<(f64, f64, usize)> = result
        .bands
        .iter()
        .map(|b| (b.from_rpm, b.to_rpm, b.samples))
        .collect();
    assert_eq!(bands, vec![(3000.0, 3500.0, 2), (3500.0, 4000.0, 2)]);
}

#[test]
fn missing_values_leave_their_columns_empty() {
    let sample = LogSample {
        rpm: 4000.0,
        load_pct: Some(100.0),
        manifold_kpa: Some(170.0),
        ..LogSample::default()
    };
    let result = compare_log(&[sample], &Maps::stock(), &Conditions::default());
    let band = &result.bands[0];
    assert!(band.manifold_kpa.measured.is_some());
    assert_eq!(band.charge_air_c.measured, None);
    // Without an intake temperature the model cannot explain the state.
    assert_eq!(band.air_g_s.simulated, None);
    assert_eq!(band.timing_deg.deviation_pct(), None);
}
