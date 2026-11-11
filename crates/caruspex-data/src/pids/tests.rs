//! Tests for the live value table: structure, key scheme and the decoders,
//! with byte examples worked out by hand from the formulas of SAE J1979.

use std::collections::{BTreeMap, HashSet};

use super::*;
use crate::test_support::{english, german};

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * (1.0 + b.abs())
}

macro_rules! assert_approx {
    ($a:expr, $b:expr) => {
        let (a, b) = ($a, $b);
        assert!(approx(a, b), "{} is not {}", a, b);
    };
}

/// All values of one PID, keyed by sub index.
fn decode(pid: i32, data: &[i32]) -> BTreeMap<i32, f64> {
    let spec = find_pid(pid).expect("PID is in the table");
    decode_pid(spec, data)
        .into_iter()
        .map(|r| (r.spec.sub, r.value))
        .collect()
}

fn text_of(key: i32, value: f64) -> String {
    let v = find_pid_value(key).expect("value is in the table");
    v.text_of(value).expect("value has a status text")
}

// ---------------------------------------------------------------------------
// Table structure and key scheme
// ---------------------------------------------------------------------------

#[test]
fn every_live_value_has_a_unique_key() {
    let _lang = german();
    assert_eq!(live_key(0x24, 0), 0x24);
    assert_eq!(live_key(0x24, 1), 0x124);
    assert_eq!(live_key_pid(0x124), 0x24);
    assert_eq!(live_key_sub(0x124), 1);

    let mut keys = HashSet::new();
    for spec in pid_table() {
        let what = format!("PID {:#x}", spec.pid);
        assert!(!spec.values.is_empty(), "{what}");
        // Single-value PIDs keep key == pid, so existing code keeps working.
        assert_eq!(spec.values[0].key, spec.pid, "{what}");
        assert_eq!(spec.name, spec.values[0].name, "{what}");
        for v in &spec.values {
            assert!(keys.insert(v.key), "{what}");
            assert_eq!(v.key, live_key(spec.pid, v.sub), "{what}");
            assert!(std::ptr::eq(find_pid_value(v.key).unwrap(), v), "{what}");
            assert!(v.name.len() > 2, "{what}");
            assert!(v.need_bytes >= 1, "{what}");
        }
    }
    assert!(pid_table().len() >= 140);
    assert!(pid_value_count() >= 300);
    assert_eq!(keys.len(), pid_value_count());
}

#[test]
fn the_table_covers_the_standard_up_to_pid_c4() {
    let _lang = german();
    for pid in [
        0x01, 0x03, 0x13, 0x1D, 0x1E, 0x34, 0x3B, 0x4F, 0x50, 0x55, 0x58, 0x5F, 0x64, 0x65, 0x66,
        0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75,
        0x76, 0x77, 0x78, 0x79, 0x7A, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F, 0x81, 0x82, 0x83, 0x84, 0x85,
        0x86, 0x87, 0x89, 0x8A, 0x8B, 0x8D, 0x8E, 0x98, 0x99, 0x9A, 0x9B, 0x9D, 0x9E, 0xA1, 0xA2,
        0xA4, 0xA5, 0xA6,
    ] {
        assert!(find_pid(pid).is_some(), "PID {pid:#x}");
    }
    // The range markers are no live values.
    assert!(find_pid(0x20).is_none());
    assert!(find_pid(0xA0).is_none());
}

#[test]
fn the_live_value_set_covers_the_standard() {
    let _lang = german();
    assert!(pid_table().len() >= 70);
    // Samples across the ranges.
    for pid in [
        0x03, 0x04, 0x05, 0x0C, 0x10, 0x11, 0x14, 0x1C, 0x24, 0x2C, 0x33, 0x3C, 0x42, 0x45, 0x51,
        0x5C, 0x5E, 0x63,
    ] {
        assert!(find_pid(pid).is_some(), "PID {pid:#x}");
    }
}

fn check_names() {
    for spec in pid_table() {
        assert!(!spec.title.contains('\u{2014}'));
        for v in &spec.values {
            assert!(!v.name.contains('\u{2014}'));
        }
    }
    for c in pid_categories() {
        assert!(pid_category_name(*c).len() > 2);
    }
}

#[test]
fn names_exist_in_both_languages_without_dashes() {
    {
        let lang = german();
        check_names();
        assert_eq!(find_pid(0xA6).unwrap().name, "Kilometerstand");
        assert_eq!(find_pid(0x5D).unwrap().unit, "Grad v.OT");
        assert_eq!(find_pid(0x7F).unwrap().category, PidCategory::Vehicle);
        assert_eq!(find_pid(0x9A).unwrap().category, PidCategory::HybridEv);
        lang.set(caruspex_i18n::Language::English);
        check_names();
        assert_eq!(find_pid(0xA6).unwrap().name, "Odometer");
        assert_eq!(find_pid(0x5D).unwrap().unit, "deg BTDC");
        assert_eq!(text_of(0xA4, 3.0), "gear 3");
        assert!(find_pid(0x0C).unwrap().name.contains("Engine"));
    }
}

// ---------------------------------------------------------------------------
// Decoders with exact bytes
// ---------------------------------------------------------------------------

#[test]
fn live_values_convert_correctly() {
    let rpm = find_pid(0x0C).unwrap();
    assert_eq!(rpm.calculate(&[0x0F, 0xA0]), 1000.0); // (0x0FA0) / 4
    let coolant = find_pid(0x05).unwrap();
    assert_eq!(coolant.calculate(&[0x5A]), 50.0); // 0x5A - 40
    let voltage = find_pid(0x42).unwrap();
    assert_eq!(voltage.calculate(&[0x36, 0xB0]), 14.0); // 14000 mV
}

#[test]
fn the_newer_live_values_compute_correctly() {
    let _lang = german();
    // Catalyst temperature: (256A+B)/10 - 40
    let cat = find_pid(0x3C).unwrap();
    assert_eq!(cat.unit, "°C");
    assert_approx!(cat.calculate(&[0x13, 0x88]), 5000.0 / 10.0 - 40.0);

    // Wideband O2 sensor: lambda = (256A+B) * 2 / 65536
    let wide = find_pid(0x24).unwrap();
    assert_approx!(wide.calculate(&[0x80, 0x00, 0x00, 0x00]), 1.0);

    // Reference torque is a plain 16-bit number.
    assert_approx!(find_pid(0x63).unwrap().calculate(&[0x01, 0x00]), 256.0);

    // Injection timing: ((256A+B) - 26880) / 128
    assert_approx!(find_pid(0x5D).unwrap().calculate(&[0x69, 0x00]), 0.0);

    // Fuel trim bank 2 behaves like bank 1.
    assert_approx!(find_pid(0x08).unwrap().calculate(&[128]), 0.0);
    assert_approx!(find_pid(0x09).unwrap().calculate(&[0]), -100.0);
}

#[test]
fn status_values_come_as_text_not_as_a_number() {
    let _lang = german();
    let fuel = find_pid(0x03).unwrap();
    assert!(fuel.text.is_some());
    assert!(
        fuel.text_of(2.0)
            .unwrap()
            .contains("geschlossener Regelkreis")
    );

    let kind = find_pid(0x51).unwrap();
    assert_eq!(kind.text_of(4.0).unwrap(), "Diesel");
    assert_eq!(kind.text_of(1.0).unwrap(), "Benzin");

    let norm = find_pid(0x1C).unwrap();
    assert!(norm.text_of(6.0).unwrap().contains("EOBD"));
}

#[test]
fn status_and_narrow_band_values() {
    let _lang = german();
    // 0x01: 0x83 = lamp on, three codes; byte B bit 3 clear = spark ignition.
    let status = decode(0x01, &[0x83, 0x07, 0xE5, 0x00]);
    assert_eq!(status[&0], 1.0);
    assert_eq!(status[&1], 3.0);
    assert_eq!(status[&2], 0.0);
    assert_eq!(text_of(0x01, 1.0), "an");

    // 0x03: second fuel system 0 = not present, so only one value.
    assert_eq!(decode(0x03, &[0x02, 0x00]).len(), 1);
    assert_eq!(decode(0x03, &[0x02, 0x01]).len(), 2);

    // 0x13: bank 1 sensors 1+2, bank 2 sensors 1+2.
    assert_eq!(
        text_of(0x13, f64::from(0x33)),
        "Bank 1: Sonde 1, 2; Bank 2: Sonde 1, 2"
    );

    // 0x14: 200 * 0.005 V = 1.0 V, trim (128-128)*100/128 = 0 %.
    let narrow = decode(0x14, &[0xC8, 0x80]);
    assert_approx!(narrow[&0], 1.0);
    assert_approx!(narrow[&1], 0.0);
    // B = 0xFF: sensor not used for the trim.
    assert_eq!(decode(0x14, &[0xC8, 0xFF]).len(), 1);
}

#[test]
fn wide_band_oxygen_sensors() {
    let _lang = german();
    // 0x24: lambda 0x8000 * 2/65536 = 1.0, voltage 0x4000 * 8/65536 = 2.0 V.
    let v = decode(0x24, &[0x80, 0x00, 0x40, 0x00]);
    assert_approx!(v[&0], 1.0);
    assert_approx!(v[&1], 2.0);
    assert_eq!(find_pid_value(0x124).unwrap().unit, "V");

    // 0x34: current (CD)/256 - 128 mA: 0x8000 is zero, 0x8100 is +1 mA.
    assert_approx!(decode(0x34, &[0x80, 0x00, 0x80, 0x00])[&1], 0.0);
    assert_approx!(decode(0x34, &[0x80, 0x00, 0x81, 0x00])[&1], 1.0);
    assert_approx!(decode(0x34, &[0x80, 0x00, 0x7F, 0x00])[&1], -1.0);
    assert_approx!(
        decode(0x3B, &[0x73, 0x33, 0x80, 0x80])[&0],
        f64::from(0x7333) * 2.0 / 65536.0
    );
}

#[test]
fn signed_vapor_pressures_and_scaling_limits() {
    let _lang = german();
    // 0x32 is a signed 16-bit value with 0.25 Pa per bit.
    assert_approx!(decode(0x32, &[0xFF, 0xFC])[&0], -1.0);
    assert_approx!(decode(0x32, &[0x00, 0x04])[&0], 1.0);
    // 0x54 is signed with 1 Pa per bit.
    assert_approx!(decode(0x54, &[0xFF, 0xFF])[&0], -1.0);
    assert_approx!(decode(0x54, &[0x7F, 0xFF])[&0], 32767.0);

    // 0x4F: lambda, volt, milliampere as raw bytes, manifold pressure x10.
    let max = decode(0x4F, &[0x02, 0x08, 0x80, 0x19]);
    assert_eq!(max[&0], 2.0);
    assert_eq!(max[&1], 8.0);
    assert_eq!(max[&2], 128.0);
    assert_eq!(max[&3], 250.0);
    assert_eq!(decode(0x50, &[0x0A, 0x00, 0x00, 0x00])[&0], 100.0);

    // 0x55: secondary trims bank 1 and bank 3.
    let trim = decode(0x55, &[0x80, 0x00]);
    assert_approx!(trim[&0], 0.0);
    assert_approx!(trim[&1], -100.0);
}

#[test]
fn fuel_injection_and_torque() {
    let _lang = german();
    assert_eq!(decode(0x5C, &[0x82])[&0], 90.0); // A - 40
    assert_approx!(decode(0x5D, &[0x69, 0x00])[&0], 0.0); // 26880/128 - 210
    assert_approx!(decode(0x5D, &[0x00, 0x00])[&0], -210.0);
    assert_approx!(decode(0x5E, &[0x00, 0x64])[&0], 5.0); // 100/20 l/h
    assert_approx!(decode(0x5B, &[0xFF])[&0], 100.0);
    assert_eq!(decode(0x61, &[0xAF])[&0], 50.0); // 175 - 125
    assert_eq!(decode(0x8E, &[0x7D])[&0], 0.0);

    let points = decode(0x64, &[0x7D, 0xAF, 0xE1, 0xFF, 0x00]);
    assert_eq!(points.len(), 5);
    assert_eq!(points[&0], 0.0);
    assert_eq!(points[&1], 50.0);
    assert_eq!(points[&2], 100.0);
    assert_eq!(points[&3], 130.0);
    assert_eq!(points[&4], -125.0);

    // 0x9D: 0.02 g/s per bit, engine and vehicle.
    let rate = decode(0x9D, &[0x01, 0xF4, 0x00, 0x32]);
    assert_approx!(rate[&0], 10.0);
    assert_approx!(rate[&1], 1.0);
    // 0xA2: 1/32 mg per stroke.
    assert_approx!(decode(0xA2, &[0x01, 0x40])[&0], 10.0);
}

#[test]
fn values_with_a_support_byte() {
    let _lang = german();
    // 0x65: PTO active, automatic in park, manual in gear, glow lamp off.
    let aux = decode(0x65, &[0x0F, 0x05]);
    assert_eq!(aux.len(), 4);
    assert_eq!(aux[&0], 1.0);
    assert_eq!(aux[&1], 0.0);
    assert_eq!(aux[&2], 1.0);
    assert_eq!(aux[&3], 0.0);

    // 0x66: MAF A = 0x0C80/32 = 100 g/s, B = 0x20/32 = 1 g/s.
    let maf = decode(0x66, &[0x03, 0x0C, 0x80, 0x00, 0x20]);
    assert_approx!(maf[&0], 100.0);
    assert_approx!(maf[&1], 1.0);
    assert!(!decode(0x66, &[0x02, 0x0C, 0x80, 0x00, 0x20]).contains_key(&0));

    // 0x67: both coolant sensors, then only sensor 1.
    let ect = decode(0x67, &[0x03, 0x5A, 0x64]);
    assert_eq!(ect[&0], 50.0);
    assert_eq!(ect[&1], 60.0);
    assert_eq!(decode(0x67, &[0x01, 0x5A, 0x64]).len(), 1);
    // Only sensor 2: the primary value is missing, sensor 2 is still there.
    let second = decode(0x67, &[0x02, 0x00, 0x64]);
    assert_eq!(second.len(), 1);
    assert_eq!(second[&1], 60.0);

    // 0x68 in the short three-byte form of older control units.
    let iat = decode(0x68, &[0x03, 0x46, 0x50]);
    assert_eq!(iat.len(), 2);
    assert_eq!(iat[&0], 30.0);
    assert_eq!(iat[&1], 40.0);

    // 0x69: commanded 50.2 %, actual 25.1 %, error (0x90-128)*100/128 = 12.5 %.
    let egr = decode(0x69, &[0x07, 0x80, 0x40, 0x90, 0, 0, 0]);
    assert_eq!(egr.len(), 3);
    assert_approx!(egr[&0], f64::from(0x80) * 100.0 / 255.0);
    assert_approx!(egr[&2], 12.5);

    // 0x6B: 1 C per bit, 4 C per bit with the wide range bit.
    assert_eq!(decode(0x6B, &[0x01, 0x64, 0, 0, 0])[&0], 60.0);
    assert_eq!(decode(0x6B, &[0x11, 0x64, 0, 0, 0])[&0], 360.0);

    // 0x6D: rail pressure 10 kPa per bit, fuel temperature A - 40.
    let rail = decode(0x6D, &[0x07, 0x27, 0x10, 0x27, 0x06, 0x50, 0, 0, 0, 0, 0]);
    assert_eq!(rail[&0], 100000.0);
    assert_eq!(rail[&1], 99900.0);
    assert_eq!(rail[&2], 40.0);
}

#[test]
fn turbocharger_and_exhaust_values() {
    let _lang = german();
    // 0x70: commanded 0x0C80/32 = 100 kPa, actual 0x0C00/32 = 96 kPa,
    // control loop A closed (2).
    let boost = decode(0x70, &[0x13, 0x0C, 0x80, 0x0C, 0x00, 0, 0, 0, 0, 0x02]);
    assert_eq!(boost.len(), 3);
    assert_approx!(boost[&0], 100.0);
    assert_approx!(boost[&1], 96.0);
    assert_eq!(boost[&4], 2.0);
    assert!(text_of(live_key(0x70, 4), 2.0).contains("geregelt"));

    // 0x73: 0.01 kPa per bit. 0x74: 10 rpm per bit.
    assert_approx!(decode(0x73, &[0x01, 0x27, 0x10, 0, 0])[&0], 100.0);
    assert_eq!(decode(0x74, &[0x01, 0x27, 0x10, 0, 0])[&0], 100000.0);

    // 0x75: two 1 C bytes, then two 0.1 C words.
    let turbo = decode(0x75, &[0x0F, 0x5A, 0x8C, 0x1F, 0x40, 0x1A, 0x2C]);
    assert_eq!(turbo[&0], 50.0);
    assert_eq!(turbo[&1], 100.0);
    assert_approx!(turbo[&2], 760.0);
    assert_approx!(turbo[&3], 630.0);

    // 0x78: only sensor 1 is reported: 8000/10 - 40 = 760 C.
    let egt = decode(0x78, &[0x01, 0x1F, 0x40, 0, 0, 0, 0, 0, 0]);
    assert_eq!(egt.len(), 1);
    assert_approx!(egt[&0], 760.0);
    assert_approx!(
        decode(0x98, &[0x08, 0, 0, 0, 0, 0, 0, 0x1F, 0x40])[&3],
        760.0
    );

    // 0x7A: signed differential pressure 0xFF9C = -100 * 0.01 kPa.
    let dpf = decode(0x7A, &[0x07, 0xFF, 0x9C, 0x27, 0x10, 0x27, 0x10]);
    assert_approx!(dpf[&0], -1.0);
    assert_approx!(dpf[&1], 100.0);
    assert_approx!(dpf[&2], 100.0);
    // 0x7C: filter inlet temperature.
    assert_approx!(
        decode(0x7C, &[0x01, 0x1F, 0x40, 0, 0, 0, 0, 0, 0])[&0],
        760.0
    );

    // 0x9E: 0.2 kg/h per bit.
    assert_approx!(decode(0x9E, &[0x03, 0xE8])[&0], 200.0);
}

#[test]
fn run_time_emissions_and_vehicle_values() {
    let _lang = german();
    // 0x7F: total 3 600 000 s = 1000 h, idle 3600 s = 1 h, PTO not reported.
    let run = decode(
        0x7F,
        &[
            0x03, 0x00, 0x36, 0xEE, 0x80, 0x00, 0x00, 0x0E, 0x10, 0, 0, 0, 0,
        ],
    );
    assert_eq!(run.len(), 2);
    assert_approx!(run[&0], 1000.0);
    assert_approx!(run[&1], 1.0);

    // 0x83: 500 ppm at bank 1 sensor 1.
    assert_eq!(
        decode(0x83, &[0x01, 0x01, 0xF4, 0, 0, 0, 0, 0, 0])[&0],
        500.0
    );

    // 0x85: 200 * 0.005 = 1 l/h, 100 * 0.005 = 0.5 l/h, tank 0x80, 3600 s.
    let def = decode(
        0x85,
        &[0x0F, 0x00, 0xC8, 0x00, 0x64, 0x80, 0x00, 0x00, 0x0E, 0x10],
    );
    assert_approx!(def[&0], 1.0);
    assert_approx!(def[&1], 0.5);
    assert_approx!(def[&2], f64::from(0x80) * 100.0 / 255.0);
    assert_eq!(def[&3], 3600.0);

    // 0x86: 0.0125 mg/m3 per bit.
    assert_approx!(decode(0x86, &[0x01, 0x00, 0x50, 0, 0])[&0], 1.0);

    // 0x8B: regeneration running, actively triggered, trigger level 50.2 %.
    let dpf = decode(0x8B, &[0x13, 0x03, 0x80, 0, 0, 0, 0]);
    assert_eq!(dpf.len(), 3);
    assert_eq!(dpf[&0], 1.0);
    assert_eq!(dpf[&1], 1.0);
    assert_approx!(dpf[&4], f64::from(0x80) * 100.0 / 255.0);
    assert_eq!(text_of(0x8B, 1.0), "läuft gerade");

    // 0x9A: 0x5DC0/64 = 375 V, current 0xFF38 = -200 * 0.1 A.
    let ev = decode(0x9A, &[0x06, 0x00, 0x5D, 0xC0, 0xFF, 0x38]);
    assert_approx!(ev[&0], 375.0);
    assert_approx!(ev[&1], -20.0);

    // 0xA4: third gear, ratio 3000/1000.
    let gear = decode(0xA4, &[0x03, 0x30, 0x0B, 0xB8]);
    assert_eq!(gear[&0], 3.0);
    assert_approx!(gear[&1], 3.0);
    assert_eq!(text_of(0xA4, 3.0), "Gang 3");
    assert_eq!(text_of(0xA4, 0.0), "Leerlauf");

    // 0xA5: 0.5 % per bit. 0xA6: 0.1 km per bit over four bytes.
    assert_approx!(decode(0xA5, &[0x01, 0x64, 0, 0])[&0], 50.0);
    assert_approx!(decode(0xA6, &[0x00, 0x01, 0xE2, 0x40])[&0], 12345.6);

    // AECD timers: AECD 1, timer 2 = 0x00010000 s.
    let aecd = [
        0x01, 0, 0, 0, 5, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    assert_eq!(decode(0x81, &aecd)[&1], 65536.0);
}

#[test]
fn warn_limits_and_gauge_ranges() {
    let _lang = german();
    let volt = find_pid_value(0x42).unwrap();
    assert!(volt.warns(11.0));
    assert!(!volt.warns(13.8));
    assert!(volt.display_max > volt.display_min);
    let tank = find_pid_value(0x2F).unwrap();
    assert!(tank.warns(5.0));
    assert!(!tank.warns(100.0));
    for spec in pid_table() {
        for v in &spec.values {
            if v.text.is_some() {
                continue;
            }
            assert!(v.display_max > v.display_min, "key {:#x}", v.key);
        }
    }
}

#[test]
fn short_answers_do_not_panic() {
    let _lang = german();
    // A direct call with too few bytes reads zeros instead of failing.
    for spec in pid_table() {
        let _ = spec.calculate(&[]);
        assert!(decode_pid(spec, &[]).is_empty());
    }
}

#[test]
fn readiness_monitors_and_defaults() {
    {
        let _lang = german();
        assert_eq!(monitors_b().len(), 3);
        assert_eq!(monitors_c().len(), 8);
        assert_eq!(monitors_c_diesel().len(), 6);
        assert_eq!(monitors_c()[0].name, "Katalysator");
    }
    let _lang = english();
    assert_eq!(monitors_c()[0].name, "Catalyst");
    assert_eq!(monitors_b()[0].mask, 0x01);
    assert_eq!(default_live_pids().len(), 12);
    for pid in default_live_pids() {
        assert!(find_pid(*pid).is_some(), "{pid:#x}");
    }
}
