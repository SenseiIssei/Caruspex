//! Measured against simulated: live values of a recording next to what the
//! model says at the same rpm and load.
//!
//! The model only knows full load, so only samples at full load count (high
//! calculated load, wide-open throttle or clear boost). They are grouped in
//! rpm bands of 500 rpm. Per band two questions are asked:
//! - **Does the model reach the same state?** Boost and charge air
//!   temperature as the model predicts them for the maps you set.
//! - **Does the model explain the measured state?** With the measured
//!   manifold pressure and intake temperature put in, the air mass the model
//!   expects against the measured air mass (MAF), and the ignition it expects
//!   against the measured ignition.
//!
//! The samples come from the caller; turning a recording into samples is the
//! job of `caruspex-record` (this crate never reads car data itself).

use crate::model::{Conditions, Maps, Measured, RPM_END, RPM_START, point_at};

/// One moment of a recording. Values the car did not report are `None`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LogSample {
    /// Seconds since the start of the recording.
    pub t: f64,
    pub rpm: f64,
    /// PID 0x04, %.
    pub load_pct: Option<f64>,
    /// PID 0x43, %. Above 100 % only with boost.
    pub absolute_load_pct: Option<f64>,
    /// PID 0x11, %.
    pub throttle_pct: Option<f64>,
    /// PID 0x0B, kPa absolute. The standard caps it at 255 kPa.
    pub manifold_kpa: Option<f64>,
    /// PID 0x0F, °C. Which sensor the engine control reports here is not
    /// documented; the comparison treats it as the charge air temperature.
    pub intake_c: Option<f64>,
    /// PID 0x0E, degrees before top dead centre (on many engines the
    /// timing of one cylinder or an average).
    pub timing_deg: Option<f64>,
    /// Measured lambda (PID 0x24 or 0x34), else the commanded one (0x44).
    pub lambda: Option<f64>,
    /// PID 0x10, g/s.
    pub maf_g_s: Option<f64>,
    /// PID 0x33, kPa.
    pub baro_kpa: Option<f64>,
    /// PID 0x46, °C.
    pub ambient_c: Option<f64>,
}

impl LogSample {
    /// Whether the engine was at (or very near) full load.
    pub fn full_load(&self) -> bool {
        let load = self.load_pct.is_some_and(|v| v >= 85.0);
        let absolute = self.absolute_load_pct.is_some_and(|v| v >= 100.0);
        let throttle = self.throttle_pct.is_some_and(|v| v >= 80.0);
        // Without load or throttle values: clear boost.
        let boost = match (self.manifold_kpa, self.baro_kpa) {
            (Some(map), Some(baro)) => map > baro + 40.0,
            (Some(map), None) => map > 140.0,
            _ => false,
        };
        let has_load_values = self.load_pct.is_some()
            || self.absolute_load_pct.is_some()
            || self.throttle_pct.is_some();
        if has_load_values {
            load || absolute || throttle
        } else {
            boost
        }
    }
}

/// A measured and a simulated mean.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pair {
    pub measured: Option<f64>,
    pub simulated: Option<f64>,
}

impl Pair {
    /// simulated minus measured.
    pub fn difference(&self) -> Option<f64> {
        Some(self.simulated? - self.measured?)
    }

    /// Difference in % of the measured value.
    pub fn deviation_pct(&self) -> Option<f64> {
        let m = self.measured?;
        if m.abs() < 1e-9 {
            return None;
        }
        Some((self.simulated? - m) / m * 100.0)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompareBand {
    pub from_rpm: f64,
    pub to_rpm: f64,
    pub samples: usize,
    /// Manifold pressure, kPa absolute: measured, and simulated for the maps.
    pub manifold_kpa: Pair,
    /// Charge air (intake) temperature, °C: measured, and simulated for the maps.
    pub charge_air_c: Pair,
    /// Air mass, g/s: measured (MAF), and simulated at the measured pressure
    /// and temperature.
    pub air_g_s: Pair,
    /// Ignition, degrees: measured, and simulated at the measured pressure
    /// and temperature.
    pub timing_deg: Pair,
    /// Lambda: measured or commanded, and the target of the maps.
    pub lambda: Pair,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LogComparison {
    pub bands: Vec<CompareBand>,
    /// All samples handed in.
    pub total: usize,
    /// Samples at full load in the rpm range of the model.
    pub used: usize,
    /// Samples left out because the engine was not at full load.
    pub part_load: usize,
    /// Samples left out because the rpm was outside 1,000 to 7,000.
    pub out_of_range: usize,
}

/// Width of the comparison bands.
pub(crate) const BAND_RPM: f64 = 500.0;

#[derive(Default)]
struct Sum {
    measured: f64,
    measured_n: usize,
    simulated: f64,
    simulated_n: usize,
}

impl Sum {
    fn add(&mut self, measured: Option<f64>, simulated: Option<f64>) {
        // Only pairs count, so both means come from the same samples.
        if let (Some(m), Some(s)) = (measured, simulated)
            && m.is_finite()
            && s.is_finite()
        {
            self.measured += m;
            self.measured_n += 1;
            self.simulated += s;
            self.simulated_n += 1;
        }
    }

    fn pair(&self) -> Pair {
        let mean = |sum: f64, n: usize| (n > 0).then(|| sum / n as f64);
        Pair {
            measured: mean(self.measured, self.measured_n),
            simulated: mean(self.simulated, self.simulated_n),
        }
    }
}

#[derive(Default)]
struct BandSums {
    samples: usize,
    manifold: Sum,
    charge_air: Sum,
    air: Sum,
    timing: Sum,
    lambda: Sum,
}

/// Compares the full-load samples of a recording with the model.
/// `conditions` supplies what the log lacks: ambient pressure and temperature
/// when PIDs 0x33 and 0x46 are missing, and always the fuel and gearbox.
pub fn compare_log(samples: &[LogSample], maps: &Maps, conditions: &Conditions) -> LogComparison {
    let bands_count = ((RPM_END - RPM_START) / BAND_RPM).round() as usize;
    let mut sums: Vec<BandSums> = (0..bands_count).map(|_| BandSums::default()).collect();
    let mut out = LogComparison {
        total: samples.len(),
        ..LogComparison::default()
    };

    for s in samples {
        if !s.rpm.is_finite() || s.rpm < RPM_START || s.rpm > RPM_END {
            out.out_of_range += 1;
            continue;
        }
        if !s.full_load() {
            out.part_load += 1;
            continue;
        }
        out.used += 1;
        let band = (((s.rpm - RPM_START) / BAND_RPM) as usize).min(bands_count - 1);
        let sum = &mut sums[band];
        sum.samples += 1;

        let mut cond = conditions.clone();
        if let Some(baro) = s.baro_kpa {
            cond.ambient_hpa = baro * 10.0;
        }
        if let Some(ambient) = s.ambient_c {
            cond.ambient_c = ambient;
        }
        let predicted = point_at(s.rpm, maps, &cond, None);
        sum.manifold
            .add(s.manifold_kpa, Some(predicted.manifold_kpa));
        sum.charge_air.add(s.intake_c, Some(predicted.charge_air_c));
        sum.lambda.add(s.lambda, Some(predicted.lambda));

        if let (Some(map), Some(intake)) = (s.manifold_kpa, s.intake_c) {
            let explained = point_at(
                s.rpm,
                maps,
                &cond,
                Some(Measured {
                    manifold_bar: map / 100.0,
                    charge_air_k: intake + 273.15,
                }),
            );
            sum.air.add(s.maf_g_s, Some(explained.air_mass_kg_h / 3.6));
            sum.timing.add(s.timing_deg, Some(explained.timing_deg));
        }
    }

    for (i, sum) in sums.iter().enumerate() {
        if sum.samples == 0 {
            continue;
        }
        let from = RPM_START + BAND_RPM * i as f64;
        out.bands.push(CompareBand {
            from_rpm: from,
            to_rpm: from + BAND_RPM,
            samples: sum.samples,
            manifold_kpa: sum.manifold.pair(),
            charge_air_c: sum.charge_air.pair(),
            air_g_s: sum.air.pair(),
            timing_deg: sum.timing.pair(),
            lambda: sum.lambda.pair(),
        });
    }
    out
}
