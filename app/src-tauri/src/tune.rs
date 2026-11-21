// The tuning simulator page on top of caruspex-tune. Simulation only: these
// commands compute curves and read recordings from disk, they never reach the
// worker or a car.
use std::time::Instant;

use caruspex_i18n::l;
use caruspex_record::{LiveFrame, live_frames, load_recording};
use caruspex_tune::spec::uncertainty;
use caruspex_tune::{
    BandOffset, BoostCap, Conditions, Gearbox, LogSample, Maps, Pair, Point, Severity, Simulation,
    Warning, compare_log, simulate,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct BandArg {
    pub from_rpm: f64,
    pub to_rpm: f64,
    pub boost_bar: f64,
    pub ignition_deg: f64,
    pub lambda: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TuneInputArg {
    pub bands: Vec<BandArg>,
    pub ambient_c: f64,
    pub ambient_hpa: f64,
    pub octane: f64,
    /// "automatic" or "manual".
    pub gearbox: String,
}

impl TuneInputArg {
    fn maps(&self) -> Maps {
        let bands: Vec<BandOffset> = self
            .bands
            .iter()
            .map(|b| BandOffset {
                from_rpm: b.from_rpm,
                to_rpm: b.to_rpm,
                boost_bar: b.boost_bar,
                ignition_deg: b.ignition_deg,
                lambda: b.lambda,
            })
            .collect();
        Maps::stock().with_bands(&bands)
    }

    fn conditions(&self) -> Conditions {
        Conditions {
            ambient_c: self.ambient_c,
            ambient_hpa: self.ambient_hpa,
            octane: self.octane,
            gearbox: if self.gearbox == "manual" {
                Gearbox::Manual6
            } else {
                Gearbox::Automatic8hp45
            },
        }
    }
}

fn cap_key(cap: BoostCap) -> &'static str {
    match cap {
        BoostCap::None => "none",
        BoostCap::Spool => "spool",
        BoostCap::PressureRatio => "pressure",
        BoostCap::Flow => "flow",
    }
}

fn severity_key(severity: Severity) -> &'static str {
    match severity {
        Severity::Note => "note",
        Severity::Warning => "warning",
        Severity::Limit => "limit",
    }
}

fn point_json(p: &Point) -> Value {
    json!({
        "rpm": p.rpm,
        "torqueNm": p.torque_nm,
        "powerKw": p.power_kw,
        "powerPs": p.power_ps,
        "boostTargetBar": p.boost_target_bar,
        "boostBar": p.boost_bar,
        "manifoldKpa": p.manifold_kpa,
        "boostCap": cap_key(p.boost_cap),
        "chargeAirC": p.charge_air_c,
        "airKgH": p.air_mass_kg_h,
        "pressureRatio": p.pressure_ratio,
        "turboLoad": p.turbo_load,
        "lambda": p.lambda,
        "timingRequestedDeg": p.timing_requested_deg,
        "timingDeg": p.timing_deg,
        "knockMarginDeg": p.knock_margin_deg,
        "egtC": p.egt_c,
        "fuelKgH": p.fuel_kg_h,
        "pumpLoad": p.pump_load,
        "injectorLoad": p.injector_load,
        "bmepBar": p.bmep_bar,
        "gearboxLoad": p.gearbox_load,
    })
}

fn warning_json(w: &Warning) -> Value {
    json!({
        "kind": w.kind.key(),
        "severity": severity_key(w.severity),
        "title": w.title(),
        "reason": w.reason(),
        "fromRpm": w.from_rpm,
        "toRpm": w.to_rpm,
    })
}

fn curve_json(sim: &Simulation) -> Value {
    json!({
        "points": sim.points.iter().map(point_json).collect::<Vec<_>>(),
        "warnings": sim.warnings.iter().map(warning_json).collect::<Vec<_>>(),
        "peakTorque": { "rpm": sim.peak_torque.0, "nm": sim.peak_torque.1 },
        "peakPower": {
            "rpm": sim.peak_power.0,
            "kw": sim.peak_power.1,
            "ps": sim.peak_power.1 * 1000.0 / 735.49875,
        },
    })
}

/// Stock and changed curve under the same conditions.
pub fn simulate_json(input: &TuneInputArg) -> Value {
    let conditions = input.conditions();
    let started = Instant::now();
    let stock = simulate(&Maps::stock(), &conditions);
    let changed = simulate(&input.maps(), &conditions);
    // Both curves together; the interface shows it so the cost stays visible.
    let micros = started.elapsed().as_secs_f64() * 1e6;
    json!({
        "stock": curve_json(&stock),
        "changed": curve_json(&changed),
        "micros": micros,
        "uncertainty": {
            "torqueShare": uncertainty::TORQUE_SHARE,
            "boostBar": uncertainty::BOOST_BAR,
            "chargeAirK": uncertainty::CHARGE_AIR_K,
            "knockDeg": uncertainty::KNOCK_DEG,
            "egtK": uncertainty::EGT_K,
        },
    })
}

/// The lambda of a frame: a wide-band sensor first, else the commanded one.
fn frame_lambda(frame: &LiveFrame) -> Option<f64> {
    (0x24..=0x2B)
        .chain(0x34..=0x3B)
        .chain(std::iter::once(0x44))
        .find_map(|pid| frame.values.get(&pid).copied())
        .filter(|v| *v > 0.3)
}

pub fn sample_of(frame: &LiveFrame) -> Option<LogSample> {
    let get = |pid: i32| frame.values.get(&pid).copied();
    Some(LogSample {
        t: frame.t,
        rpm: get(0x0C)?,
        load_pct: get(0x04),
        absolute_load_pct: get(0x43),
        throttle_pct: get(0x11),
        manifold_kpa: get(0x0B),
        intake_c: get(0x0F),
        timing_deg: get(0x0E),
        lambda: frame_lambda(frame),
        maf_g_s: get(0x10),
        baro_kpa: get(0x33),
        ambient_c: get(0x46),
    })
}

fn pair_json(pair: &Pair) -> Value {
    json!({
        "measured": pair.measured,
        "simulated": pair.simulated,
        "difference": pair.difference(),
        "deviationPct": pair.deviation_pct(),
    })
}

fn comparison_json(file: &str, samples: &[LogSample], input: &TuneInputArg) -> Value {
    let result = compare_log(samples, &input.maps(), &input.conditions());
    let bands: Vec<Value> = result
        .bands
        .iter()
        .map(|b| {
            json!({
                "fromRpm": b.from_rpm,
                "toRpm": b.to_rpm,
                "samples": b.samples,
                "manifoldKpa": pair_json(&b.manifold_kpa),
                "chargeAirC": pair_json(&b.charge_air_c),
                "airGs": pair_json(&b.air_g_s),
                "timingDeg": pair_json(&b.timing_deg),
                "lambda": pair_json(&b.lambda),
            })
        })
        .collect();
    json!({
        "file": file,
        "total": result.total,
        "used": result.used,
        "partLoad": result.part_load,
        "outOfRange": result.out_of_range,
        "bands": bands,
    })
}

/// The comparison of a recording with the maps and conditions of the page.
pub fn compare_json(path: &str, input: &TuneInputArg) -> Result<Value, String> {
    let recording = load_recording(path)?;
    let samples: Vec<LogSample> = live_frames(&recording)
        .iter()
        .filter_map(sample_of)
        .collect();
    if samples.is_empty() {
        return Err(l(
            "In der Aufnahme stehen keine Live-Werte mit Motordrehzahl. Nimm eine Fahrt mit offenen Live-Werten auf (Drehzahl, Saugrohrdruck, Ansauglufttemperatur, Zündzeitpunkt, Last).",
            "The recording holds no live values with engine speed. Record a drive with the live values open (engine speed, manifold pressure, intake air temperature, ignition timing, load).",
        )
        .to_string());
    }
    let name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(comparison_json(&name, &samples, input))
}

/// An invented full-load pull from 1,500 to 6,500 rpm: the stock model with
/// small fixed differences of the kind a real car shows (a little more
/// boost, warmer intake air, a little less ignition and air mass), plus a
/// few part-load samples that must stay out. It only shows how the table
/// reads; it is not a measurement and the page says so.
pub fn example_samples() -> Vec<LogSample> {
    let points = caruspex_tune::curve(&Maps::stock(), &Conditions::default());
    let mut out = Vec::new();
    for (i, p) in points
        .iter()
        .filter(|p| p.rpm >= 1500.0 && p.rpm <= 6500.0)
        .enumerate()
    {
        // Two samples per point, a little apart, like a real pull.
        for (j, wobble) in [-0.5, 0.5].into_iter().enumerate() {
            out.push(LogSample {
                t: 10.0 + i as f64 * 0.3 + j as f64 * 0.15,
                rpm: p.rpm + 40.0 * wobble,
                load_pct: Some(100.0),
                absolute_load_pct: None,
                throttle_pct: Some(99.0),
                manifold_kpa: Some((p.manifold_kpa + 4.0 + wobble).round()),
                intake_c: Some((p.charge_air_c + 5.0).round()),
                timing_deg: Some(((p.timing_deg - 0.8 + wobble * 0.5) * 2.0).round() / 2.0),
                lambda: Some(p.lambda + 0.01),
                maf_g_s: Some(p.air_mass_kg_h / 3.6 * 0.97),
                baro_kpa: Some(101.0),
                ambient_c: Some(20.0),
            });
        }
    }
    for k in 0..6 {
        out.push(LogSample {
            t: k as f64,
            rpm: 1800.0 + 100.0 * k as f64,
            load_pct: Some(30.0),
            throttle_pct: Some(15.0),
            manifold_kpa: Some(60.0),
            intake_c: Some(30.0),
            ..LogSample::default()
        });
    }
    out
}

pub fn compare_example_json(input: &TuneInputArg) -> Value {
    comparison_json("", &example_samples(), input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> TuneInputArg {
        TuneInputArg {
            bands: vec![BandArg {
                from_rpm: 2000.0,
                to_rpm: 4000.0,
                boost_bar: 0.1,
                ignition_deg: 0.0,
                lambda: 0.0,
            }],
            ambient_c: 20.0,
            ambient_hpa: 1013.0,
            octane: 98.0,
            gearbox: "automatic".into(),
        }
    }

    #[test]
    fn simulate_answers_both_curves_quickly() {
        let v = simulate_json(&input());
        assert_eq!(v["stock"]["points"].as_array().unwrap().len(), 25);
        let stock = v["stock"]["points"][8]["torqueNm"].as_f64().unwrap();
        let changed = v["changed"]["points"][8]["torqueNm"].as_f64().unwrap();
        assert!(changed > stock);
        assert!(v["micros"].as_f64().unwrap() < 5000.0);
    }

    // A synthetic recording of a full-load pull on a car that matches the
    // model: written as the recorder writes it, read back through
    // caruspex-record and compared.
    #[test]
    fn a_synthetic_recording_is_compared_band_by_band() {
        let points = caruspex_tune::curve(&Maps::stock(), &Conditions::default());
        let mut text = String::from(
            "{\"format\":\"caruspex-recording\",\"version\":1,\"created\":\"2026-11-20T10:00:00Z\",\"transport\":\"test\"}\n",
        );
        let mut t = 0.0;
        for p in points.iter().filter(|p| p.rpm >= 2000.0 && p.rpm <= 6000.0) {
            let rpm = (p.rpm * 4.0).round() as u32;
            let map = p.manifold_kpa.round() as u32;
            let iat = (p.charge_air_c + 40.0).round() as u32;
            let timing = ((p.timing_deg + 64.0) * 2.0).round() as u32;
            text += &format!("{{\"t\":{t:.3},\"dir\":\"tx\",\"data\":\"010C0B0F0E04\\r\"}}\n");
            text += &format!(
                "{{\"t\":{:.3},\"dir\":\"rx\",\"data\":\"00C\\r0:410C{:04X}0B{:02X}\\r1:0F{:02X}0E{:02X}04FF\\r\\r>\"}}\n",
                t + 0.03,
                rpm,
                map,
                iat,
                timing
            );
            t += 0.2;
        }
        let path = std::env::temp_dir().join(format!(
            "caruspex_rs_test_tune_pull_{}.jsonl",
            std::process::id()
        ));
        std::fs::write(&path, text).unwrap();
        let v = compare_json(
            path.to_str().unwrap(),
            &TuneInputArg {
                bands: vec![],
                ..input()
            },
        )
        .expect("compares");
        let _ = std::fs::remove_file(&path);
        assert_eq!(v["used"].as_u64(), Some(17));
        let bands = v["bands"].as_array().unwrap();
        assert!(bands.len() >= 8);
        for band in bands {
            let d = band["manifoldKpa"]["deviationPct"].as_f64().unwrap();
            // Rounded to whole kPa on the wire.
            assert!(d.abs() < 1.0, "{band}");
            let timing = band["timingDeg"]["difference"].as_f64().unwrap();
            assert!(timing.abs() < 0.6, "{band}");
        }
    }

    #[test]
    fn a_recording_without_rpm_is_explained() {
        let path = std::env::temp_dir().join(format!(
            "caruspex_rs_test_tune_empty_{}.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &path,
            "{\"format\":\"caruspex-recording\",\"version\":1,\"created\":\"2026-11-20T10:00:00Z\",\"transport\":\"test\"}\n",
        )
        .unwrap();
        let err = compare_json(path.to_str().unwrap(), &input()).unwrap_err();
        let _ = std::fs::remove_file(&path);
        assert!(!err.is_empty());
    }
}
