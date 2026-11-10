//! Trip computer with concrete drives.

use caruspex_analysis::common::FuelType;
use caruspex_analysis::trip_computer::{FuelSource, TripComputer, TripConfig};

fn near(a: f64, b: f64, margin: f64) -> bool {
    (a - b).abs() <= margin
}

fn config_for(fuel: FuelType) -> TripConfig {
    TripConfig {
        fuel,
        ..Default::default()
    }
}

#[test]
fn one_hour_at_100_kmh_with_6_l_per_h() {
    let cfg = TripConfig {
        fuel: FuelType::Petrol,
        fuel_price_per_litre: 1.80,
        ..Default::default()
    };
    let mut trip = TripComputer::new(cfg);
    for t in 0..=3600 {
        let t = f64::from(t);
        trip.add_sample(0x0D, 100.0, t);
        trip.add_sample(0x0C, 2200.0, t);
        trip.add_sample(0x5E, 6.0, t);
    }
    let s = trip.summary();
    assert!(near(s.distance_km, 100.0, 0.01));
    assert!(near(s.fuel_litres, 6.0, 0.01));
    assert!(near(s.average_litres_per_100_km, 6.0, 0.01));
    assert!(near(s.live_litres_per_100_km, 6.0, 0.01));
    assert!(near(s.live_litres_per_hour, 6.0, 1e-6));
    assert!(near(s.co2_grams_per_km, 142.2, 0.1)); // 6 l * 2.37 kg / 100 km
    assert!(near(s.co2_kg, 14.22, 0.01));
    assert!(near(s.cost, 10.80, 0.01));
    assert!(near(s.average_speed_kmh, 100.0, 0.01));
    assert_eq!(s.max_speed_kmh, 100.0);
    assert_eq!(s.idle_share, 0.0);
    assert_eq!(s.fuel_source, FuelSource::FuelRate);
    assert!(!s.fuel_is_rough);
    assert_eq!(s.eco.total, 100);
    assert_eq!(s.eco.harsh_accelerations, 0);
}

#[test]
fn fuel_from_mass_air_flow_and_speed_density() {
    let petrol = TripComputer::new(config_for(FuelType::Petrol));
    // 10 g/s air / 14.7 = 0.680 g/s petrol = 0.000913 l/s = 3.287 l/h
    assert!(near(petrol.fuel_rate_from_maf(10.0), 3.2873, 0.001));

    let sd = TripConfig {
        displacement_litres: 2.0,
        volumetric_efficiency: 0.85,
        ..Default::default()
    };
    let engine = TripComputer::new(sd);
    // 100 kPa, 300 K, 3000 rpm: 1.974 g per two turns, 25 per second.
    assert!(near(
        engine.air_flow_from_speed_density(100.0, 26.85, 3000.0),
        49.35,
        0.05
    ));

    // A drive on MAF only: 600 s at 50 km/h with 10 g/s.
    let mut maf = TripComputer::new(config_for(FuelType::Petrol));
    for t in 0..=600 {
        maf.add_sample(0x0D, 50.0, f64::from(t));
        maf.add_sample(0x10, 10.0, f64::from(t));
    }
    let s = maf.summary();
    assert_eq!(s.fuel_source, FuelSource::MassAirFlow);
    assert!(near(s.fuel_litres, 3.2873 / 6.0, 0.001));
    assert!(near(s.distance_km, 50.0 / 6.0, 0.001));
    assert!(near(s.average_litres_per_100_km, 6.575, 0.01));
    assert!(!s.fuel_is_rough);

    // The same on a diesel is only a rough estimate and says so.
    let mut diesel = TripComputer::new(config_for(FuelType::Diesel));
    for t in 0..=60 {
        diesel.add_sample(0x0D, 50.0, f64::from(t));
        diesel.add_sample(0x10, 10.0, f64::from(t));
    }
    let d = diesel.summary();
    assert!(d.fuel_is_rough);
    assert!(d.fuel_note.de.contains("grobe Schätzung"));
    assert!(near(d.co2_kg, d.fuel_litres * 2.65, 1e-9));
}

#[test]
fn idling_counts_as_l_per_h_and_idle_share() {
    let mut trip = TripComputer::default();
    for t in 0..=600 {
        let t = f64::from(t);
        trip.add_sample(0x0D, 0.0, t);
        trip.add_sample(0x0C, 800.0, t);
        trip.add_sample(0x5E, 0.8, t);
    }
    let s = trip.summary();
    assert_eq!(s.distance_km, 0.0);
    assert!(near(s.idle_seconds, 600.0, 1e-6));
    assert!(near(s.idle_litres_per_hour, 0.8, 1e-6));
    assert!(near(s.idle_share, 1.0, 1e-6));
    assert_eq!(s.average_litres_per_100_km, 0.0); // no distance, no l/100 km
    assert_eq!(s.live_litres_per_100_km, 0.0);
    assert_eq!(s.eco.idling, 0);
}

#[test]
fn gaps_are_skipped_not_bridged() {
    let mut trip = TripComputer::default();
    for t in 0..=100 {
        trip.add_sample(0x0D, 100.0, f64::from(t));
    }
    for t in 200..=300 {
        trip.add_sample(0x0D, 100.0, f64::from(t));
    }
    let s = trip.summary();
    // 200 s at 100 km/h; the 100 s without data are not counted.
    assert!(near(s.distance_km, 200.0 / 36.0, 0.001));
    assert!(near(s.gap_seconds, 100.0, 1e-6));
}

#[test]
fn glitches_and_jittery_timestamps() {
    let mut trip = TripComputer::default();
    for i in 0..=120 {
        let t = f64::from(i) * 0.5;
        // One reading of 255 km/h out of nowhere.
        trip.add_sample(0x0D, if i == 60 { 255.0 } else { 50.0 }, t);
    }
    // A reading with a timestamp slightly in the past must not count twice.
    trip.add_sample(0x0D, 50.0, 59.8);
    let s = trip.summary();
    assert_eq!(s.max_speed_kmh, 50.0);
    assert!(near(s.distance_km, 50.0 * 60.0 / 3600.0, 0.01));

    // A real change is kept once it repeats.
    let mut jump = TripComputer::default();
    jump.add_sample(0x0D, 0.0, 0.0);
    jump.add_sample(0x0D, 80.0, 0.5); // impossible within half a second
    jump.add_sample(0x0D, 80.0, 1.0); // but it repeats: accepted
    assert_eq!(jump.summary().max_speed_kmh, 80.0);
}

#[test]
fn eco_score_counts_hard_accelerations() {
    let mut trip = TripComputer::default();
    let mut t = 0.0;
    for _round in 0..3 {
        // 0 to 60 km/h in 5 s (3.3 m/s^2, hard), cruise 2 minutes, stop gently.
        for i in 0..=50 {
            trip.add_sample(0x0D, f64::from(i) * 1.2, t);
            trip.add_sample(0x0C, 3500.0, t);
            t += 0.1;
        }
        for _ in 0..120 {
            trip.add_sample(0x0D, 60.0, t);
            trip.add_sample(0x0C, 2000.0, t);
            t += 1.0;
        }
        for i in 0..=30 {
            trip.add_sample(0x0D, 60.0 - f64::from(i) * 2.0, t); // 2 km/h per second, gentle
            trip.add_sample(0x0C, 1200.0, t);
            t += 1.0;
        }
    }
    let s = trip.summary();
    assert_eq!(s.eco.harsh_accelerations, 3);
    assert_eq!(s.eco.harsh_brakings, 0);
    assert!(s.eco.smoothness < 100);
    assert!(s.eco.engine_speed >= 0);
    assert!(s.eco.total < 100);
    assert!(s.eco.explanation.de.contains("3 starke Beschleunigungen"));
}
