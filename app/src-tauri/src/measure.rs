// The measure pages on top of core/analysis: trip computer, performance
// timer and battery test.
use std::sync::Mutex;

use caruspex_analysis::battery_test::{
    BatteryInput, BatteryReport, BatteryType, VoltageSample, battery_test_steps as steps,
    evaluate_battery as evaluate,
};
use caruspex_analysis::common::FuelType;
use caruspex_analysis::performance_timer::{
    PerformanceReport, SpeedSample, TimedRun, analyze_performance as analyze,
};
use caruspex_analysis::trip_computer::{TripComputer, TripConfig, TripSummary};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::rows::LiveRow;

fn fuel_type(code: i32) -> FuelType {
    match code {
        1 => FuelType::Petrol,
        2 => FuelType::Diesel,
        _ => FuelType::Unknown,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TripConfigArg {
    pub fuel: i32,
    pub displacement_litres: f64,
    pub volumetric_efficiency: f64,
    pub fuel_price_per_litre: f64,
    pub max_gap_seconds: f64,
    pub live_window_seconds: f64,
}

/// The trip computer of the trip page. While started, every live value the
/// worker reads goes in, like TripPage::onLiveValue.
pub struct Trip {
    computer: TripComputer,
    running: bool,
}

impl Default for Trip {
    fn default() -> Self {
        Self {
            computer: TripComputer::new(TripConfig::default()),
            running: false,
        }
    }
}

pub type TripState = Mutex<Trip>;

impl Trip {
    pub fn reset(&mut self, config: &TripConfigArg) {
        self.computer = TripComputer::new(TripConfig {
            fuel: fuel_type(config.fuel),
            displacement_litres: config.displacement_litres,
            volumetric_efficiency: config.volumetric_efficiency,
            fuel_price_per_litre: config.fuel_price_per_litre,
            max_gap_seconds: config.max_gap_seconds,
            live_window_seconds: config.live_window_seconds,
        });
    }
    pub fn start(&mut self) {
        self.running = true;
    }
    pub fn stop(&mut self) -> Value {
        self.running = false;
        summary_json(&self.computer.summary())
    }
    pub fn summary(&self) -> Value {
        summary_json(&self.computer.summary())
    }
    pub fn feed(&mut self, row: &LiveRow) {
        if self.running {
            self.computer.add_sample(row.pid, row.value, row.time);
        }
    }
}

fn summary_json(s: &TripSummary) -> Value {
    json!({
        "distanceKm": s.distance_km,
        "fuelLitres": s.fuel_litres,
        "drivingSeconds": s.driving_seconds,
        "movingSeconds": s.moving_seconds,
        "idleSeconds": s.idle_seconds,
        "gapSeconds": s.gap_seconds,
        "averageLitresPer100Km": s.average_litres_per_100_km,
        "liveLitresPer100Km": s.live_litres_per_100_km,
        "liveLitresPerHour": s.live_litres_per_hour,
        "idleLitresPerHour": s.idle_litres_per_hour,
        "co2GramsPerKm": s.co2_grams_per_km,
        "co2Kg": s.co2_kg,
        "cost": s.cost,
        "averageSpeedKmh": s.average_speed_kmh,
        "averageMovingSpeedKmh": s.average_moving_speed_kmh,
        "maxSpeedKmh": s.max_speed_kmh,
        "idleShare": s.idle_share,
        "fuelSource": s.fuel_source as i32,
        "fuelIsRough": s.fuel_is_rough,
        "fuelNote": s.fuel_note.text(),
        "eco": {
            "total": s.eco.total,
            "smoothness": s.eco.smoothness,
            "engineSpeed": s.eco.engine_speed,
            "idling": s.eco.idling,
            "harshAccelerations": s.eco.harsh_accelerations,
            "harshBrakings": s.eco.harsh_brakings,
            "explanation": s.eco.explanation.text(),
        },
    })
}

#[derive(Deserialize)]
pub struct SpeedSampleArg {
    pub t: f64,
    pub kmh: f64,
}

fn run_json(r: &TimedRun) -> Value {
    json!({
        "found": r.found,
        "seconds": r.seconds,
        "startTime": r.start_time,
        "endTime": r.end_time,
        "distanceMeters": r.distance_meters,
        "trapSpeedKmh": r.trap_speed_kmh,
        "reliable": r.reliable,
    })
}

fn performance_json(r: &PerformanceReport) -> Value {
    json!({
        "zeroTo100": run_json(&r.zero_to_100),
        "eightyTo120": run_json(&r.eighty_to_120),
        "zeroTo60mph": run_json(&r.zero_to_60_mph),
        "quarterMile": run_json(&r.quarter_mile),
        "brake100To0": run_json(&r.brake_100_to_0),
        "sampleRateHz": r.sample_rate_hz,
        "accuracySeconds": r.accuracy_seconds,
        "samplesUsed": r.samples_used,
        "samplesDropped": r.samples_dropped,
        "accuracyHint": r.accuracy_hint.text(),
    })
}

pub fn analyze_performance(samples: &[SpeedSampleArg]) -> Value {
    let samples = samples
        .iter()
        .map(|s| SpeedSample { t: s.t, kmh: s.kmh })
        .collect();
    performance_json(&analyze(samples))
}

#[derive(Deserialize)]
pub struct VoltageSampleArg {
    pub t: f64,
    pub volts: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInputArg {
    #[serde(rename = "type")]
    pub battery_type: i32,
    pub start_stop: bool,
    pub resting_volts: Option<f64>,
    pub minutes_since_engine_off: Option<f64>,
    pub cranking: Vec<VoltageSampleArg>,
    pub cranking_minimum_volts: Option<f64>,
    pub idle_volts: Option<f64>,
    pub loaded_volts: Option<f64>,
    pub ambient_celsius: Option<f64>,
}

fn battery_json(r: &BatteryReport) -> Value {
    json!({
        "overall": r.overall as i32,
        "stateOfChargePercent": r.state_of_charge_percent,
        "steps": r.steps.iter().map(|s| json!({
            "id": s.id,
            "level": s.level as i32,
            "volts": s.volts,
            "title": s.title.text(),
            "explanation": s.explanation.text(),
            "advice": s.advice.text(),
        })).collect::<Vec<_>>(),
        "missingSteps": r.missing_steps,
        "summary": r.summary.text(),
        "crankingSampleRateHz": r.cranking_sample_rate_hz,
    })
}

pub fn evaluate_battery(input: &BatteryInputArg) -> Value {
    let input = BatteryInput {
        battery_type: match input.battery_type {
            1 => BatteryType::Flooded,
            2 => BatteryType::Efb,
            3 => BatteryType::Agm,
            _ => BatteryType::Unknown,
        },
        start_stop: input.start_stop,
        resting_volts: input.resting_volts,
        minutes_since_engine_off: input.minutes_since_engine_off,
        cranking: input
            .cranking
            .iter()
            .map(|s| VoltageSample {
                t: s.t,
                volts: s.volts,
            })
            .collect(),
        cranking_minimum_volts: input.cranking_minimum_volts,
        idle_volts: input.idle_volts,
        loaded_volts: input.loaded_volts,
        ambient_celsius: input.ambient_celsius,
    };
    battery_json(&evaluate(&input))
}

pub fn battery_test_steps() -> Value {
    Value::Array(
        steps()
            .iter()
            .map(|s| json!({ "id": s.id, "instruction": s.instruction.get() }))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trip_takes_values_only_while_started() {
        let mut trip = Trip::default();
        let row = |pid: i32, value: f64, time: f64| LiveRow {
            pid,
            value,
            time,
            ..LiveRow::default()
        };
        trip.feed(&row(0x0D, 50.0, 1.0));
        assert_eq!(trip.summary()["distanceKm"], 0.0);
        trip.start();
        for i in 0..20 {
            trip.feed(&row(0x0D, 50.0, 2.0 + f64::from(i)));
        }
        let s = trip.stop();
        assert!(s["distanceKm"].as_f64().unwrap() > 0.1, "{s}");
        assert!(battery_test_steps().as_array().unwrap().len() >= 3);
    }

    #[test]
    fn performance_and_battery_shapes() {
        let samples: Vec<SpeedSampleArg> = (0..120)
            .map(|i| SpeedSampleArg {
                t: f64::from(i) * 0.1,
                kmh: (f64::from(i) * 1.2).min(130.0),
            })
            .collect();
        let report = analyze_performance(&samples);
        assert!(report["zeroTo100"]["found"].as_bool().unwrap());
        let battery = evaluate_battery(&BatteryInputArg {
            battery_type: 1,
            start_stop: false,
            resting_volts: Some(12.6),
            minutes_since_engine_off: Some(120.0),
            cranking: vec![],
            cranking_minimum_volts: None,
            idle_volts: Some(14.1),
            loaded_volts: None,
            ambient_celsius: None,
        });
        assert!(battery["stateOfChargePercent"].as_i64().is_some());
    }
}
