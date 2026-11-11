//! The simulated car: its state is a pure function of the time since
//! "ignition on", so any moment can be looked at again.

use std::f64::consts::PI;

pub(crate) const AMBIENT: f64 = 12.0; // deg C
pub(crate) const BARO: f64 = 99.0; // kPa
pub(crate) const ODOMETER_START: f64 = 48213.4;
pub(crate) const FUEL_START: f64 = 64.3; // percent
pub(crate) const TANK_LITRES: f64 = 50.0;
pub(crate) const LONG_TERM_TRIM: f64 = 17.2; // percent: the reason for P0171
pub(crate) const CRANK_SECONDS: f64 = 1.0;
pub(crate) const WARM_IDLE_SECONDS: f64 = 45.0;

// Six-speed automatic, 1.4 litre turbo-less petrol engine.
const RATIOS: [f64; 6] = [4.20, 2.50, 1.65, 1.25, 1.00, 0.82];
const FINAL_DRIVE: f64 = 3.2;
const TYRE_CIRCUMFERENCE: f64 = 1.94; // metres

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Off,
    Crank,
    Idle,
    Accelerate,
    Cruise,
    Brake,
}

struct Segment {
    duration: f64, // s
    v0: f64,       // km/h
    v1: f64,
    phase: Phase,
}

const fn seg(duration: f64, v0: f64, v1: f64, phase: Phase) -> Segment {
    Segment {
        duration,
        v0,
        v1,
        phase,
    }
}

// One drive cycle, repeated forever after the warm-up idle.
const CYCLE: [Segment; 6] = [
    seg(12.0, 0.0, 50.0, Phase::Accelerate),
    seg(28.0, 50.0, 50.0, Phase::Cruise),
    seg(15.0, 50.0, 100.0, Phase::Accelerate),
    seg(30.0, 100.0, 100.0, Phase::Cruise),
    seg(15.0, 100.0, 0.0, Phase::Brake),
    seg(20.0, 0.0, 0.0, Phase::Idle),
];
const CRUISE_WOBBLE: f64 = 1.2; // km/h, two full periods per cruise

/// Speed and distance (km) at a point within one segment.
fn segment_point(seg: &Segment, s: f64) -> (f64, f64) {
    if seg.phase == Phase::Cruise {
        let period = seg.duration / 2.0;
        let w = 2.0 * PI / period;
        let speed = seg.v0 + CRUISE_WOBBLE * (w * s).sin();
        let km = (seg.v0 * s + CRUISE_WOBBLE / w * (1.0 - (w * s).cos())) / 3600.0;
        return (speed, km);
    }
    let slope = (seg.v1 - seg.v0) / seg.duration;
    let speed = seg.v0 + slope * s;
    let km = (seg.v0 * s + 0.5 * slope * s * s) / 3600.0;
    (speed, km)
}

fn cycle_seconds() -> f64 {
    let mut total = 0.0;
    for seg in &CYCLE {
        total += seg.duration;
    }
    total
}

fn cycle_km() -> f64 {
    let mut total = 0.0;
    for seg in &CYCLE {
        total += segment_point(seg, seg.duration).1;
    }
    total
}

/// Everything a sensor could report at one moment.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sample {
    pub phase: Phase,
    pub run_time: f64, // s since the engine runs on its own
    pub speed: f64,
    pub rpm: f64,
    pub distance_km: f64,
    pub gear: i32,
    pub gear_ratio: f64,
    pub load: f64,
    pub throttle_rel: f64,
    pub throttle_abs: f64,
    pub throttle_cmd: f64,
    pub pedal_d: f64,
    pub pedal_e: f64,
    pub map: f64,
    pub maf: f64,
    pub fuel_rate: f64,
    pub timing: f64,
    pub iat: f64,
    pub coolant: f64,
    pub oil: f64,
    pub cat_temp: f64,
    pub rail: f64, // kPa
    pub stft: f64,
    pub ltft: f64,
    pub o2up: f64,
    pub o2down: f64,
    pub lambda: f64,
    pub fuel_status: i32,
    pub battery: f64,
    pub fuel_level: f64,
    pub odometer: f64,
    pub torque_demand: f64,
    pub torque_actual: f64,
}

impl Default for Sample {
    fn default() -> Self {
        Sample {
            phase: Phase::Off,
            run_time: 0.0,
            speed: 0.0,
            rpm: 0.0,
            distance_km: 0.0,
            gear: 0,
            gear_ratio: 0.0,
            load: 0.0,
            throttle_rel: 0.0,
            throttle_abs: 14.5,
            throttle_cmd: 0.0,
            pedal_d: 14.9,
            pedal_e: 7.5,
            map: BARO,
            maf: 0.0,
            fuel_rate: 0.0,
            timing: 0.0,
            iat: AMBIENT + 2.0,
            coolant: AMBIENT,
            oil: AMBIENT,
            cat_temp: AMBIENT,
            rail: 0.0,
            stft: 0.0,
            ltft: LONG_TERM_TRIM,
            o2up: 0.45,
            o2down: 0.45,
            lambda: 1.0,
            fuel_status: 0x00,
            battery: 12.6,
            fuel_level: FUEL_START,
            odometer: ODOMETER_START,
            torque_demand: 0.0,
            torque_actual: 0.0,
        }
    }
}

pub(crate) fn engine_run_time(t: f64, start: f64) -> f64 {
    (t - start - CRANK_SECONDS).max(0.0)
}

fn rpm_at(kmh: f64, gear: i32) -> f64 {
    kmh / 3.6 / TYRE_CIRCUMFERENCE * 60.0 * RATIOS[(gear - 1) as usize] * FINAL_DRIVE
}

/// The whole car as a pure function of time. No state, so any moment can be
/// looked at again (the freeze frame does exactly that).
pub(crate) fn simulate(t: f64, start: f64) -> Sample {
    let mut s = Sample::default();
    if t < start {
        s.phase = Phase::Off;
        // A healthy battery at rest, sagging a little with the ignition on.
        s.battery = (12.62 - 0.0004 * t).max(12.45);
        return s;
    }
    if t < start + CRANK_SECONDS {
        s.phase = Phase::Crank;
        s.rpm = 240.0 + 30.0 * (t * 40.0).sin();
        s.battery = 10.4 + 0.2 * (t * 25.0).sin();
        s.fuel_status = 0x01;
        s.lambda = 0.9;
        s.rail = 3000.0;
        return s;
    }

    let run = t - start - CRANK_SECONDS;
    s.run_time = run;

    // Warm-up. The thermostat holds the coolant near 90 deg C once warm.
    let warm = 1.0 - (-run / 200.0).exp();
    s.coolant =
        AMBIENT + (90.0 - AMBIENT) * warm + 1.5 * (run / 45.0).sin() * warm * warm * warm * warm;
    s.oil = AMBIENT + (96.0 - AMBIENT) * (1.0 - (-run / 380.0).exp());
    let cold = ((70.0 - s.coolant) / 58.0).clamp(0.0, 1.0);

    // Where in the drive cycle are we?
    s.phase = Phase::Idle;
    let mut seg_time = 0.0;
    let drive_time = run - WARM_IDLE_SECONDS;
    if drive_time > 0.0 {
        let length = cycle_seconds();
        let cycles = (drive_time / length).floor();
        let mut tau = drive_time - cycles * length;
        let mut km = cycles * cycle_km();
        for seg in &CYCLE {
            if tau <= seg.duration {
                let (v, part) = segment_point(seg, tau);
                s.speed = v.max(0.0);
                km += part;
                s.phase = seg.phase;
                seg_time = tau;
                break;
            }
            let (_, part) = segment_point(seg, seg.duration);
            km += part;
            tau -= seg.duration;
        }
        s.distance_km = km;
    }

    // Gearbox: shifts up early when accelerating gently, holds the highest
    // gear that keeps the engine above about 1250 rpm when cruising.
    let mut gear = 1;
    if s.speed >= 3.0 {
        if s.phase == Phase::Accelerate {
            gear = 6;
            for g in 1..=6 {
                if rpm_at(s.speed, g) <= 2700.0 {
                    gear = g;
                    break;
                }
            }
        } else {
            let lowest = if s.phase == Phase::Brake {
                1050.0
            } else {
                1250.0
            };
            for g in (1..=6).rev() {
                if rpm_at(s.speed, g) >= lowest {
                    gear = g;
                    break;
                }
            }
        }
    }
    s.gear = gear;
    s.gear_ratio = RATIOS[(gear - 1) as usize];

    let idle = 760.0 + 340.0 * cold;
    let mut rpm = if s.speed < 3.0 {
        idle
    } else {
        rpm_at(s.speed, gear)
    };
    if s.phase == Phase::Accelerate && s.speed < 25.0 {
        rpm = rpm.max(1500.0 + 40.0 * s.speed); // torque converter slip
    }
    s.rpm = rpm.max(idle) + 8.0 * (t * 7.3).sin();

    // Driver, throttle and load.
    match s.phase {
        Phase::Accelerate => {
            s.throttle_rel = 36.0 + 6.0 * (seg_time * 0.9).sin();
            s.load = 68.0 + 8.0 * (seg_time * 0.7).sin();
            s.timing = 16.0 + 2.0 * (t * 1.3).sin();
            s.torque_demand = 58.0;
            s.rail = 14000.0;
        }
        Phase::Cruise => {
            s.throttle_rel = 9.0 + 0.09 * s.speed + 1.5 * (t * 0.8).sin();
            s.load = 20.0 + 0.14 * s.speed + 2.0 * (t * 0.8).sin();
            s.timing = 32.0 - 0.25 * s.load;
            s.torque_demand = s.load * 0.85;
            s.rail = 7000.0 + 30.0 * s.speed;
        }
        Phase::Brake => {
            s.throttle_rel = 0.0;
            s.load = 13.0 + 2.0 * t.sin();
            s.timing = 22.0;
            s.torque_demand = 0.0;
            s.rail = 4000.0;
        }
        _ => {
            s.throttle_rel = 0.0;
            s.load = 20.0 + 6.0 * cold + 1.0 * (t * 1.7).sin();
            s.timing = 7.0 + 3.0 * (t * 2.1).sin() - 4.0 * cold;
            s.torque_demand = 8.0;
            s.rail = 4500.0;
        }
    }
    let fuel_cut = s.phase == Phase::Brake && s.rpm > 1150.0 && cold < 0.5;
    s.torque_actual = if fuel_cut { 0.0 } else { s.load * 0.92 };
    s.throttle_abs = 14.5 + s.throttle_rel * 0.82;
    s.throttle_cmd = s.throttle_rel * 0.95 + if s.speed < 3.0 { 3.1 + 2.0 * cold } else { 0.0 };
    s.pedal_d = 14.9 + s.throttle_rel * 0.75;
    s.pedal_e = s.pedal_d / 2.0;
    s.map = BARO.min(22.0 + s.load * 0.78);
    s.maf = s.rpm / 120.0 * 1.4 * 1.18 * s.load / 100.0 * 1.05;
    s.iat = AMBIENT + 3.0 + 7.0 * (-s.speed / 40.0).exp() + 3.0 * (1.0 - (-run / 300.0).exp());

    // Mixture control: open loop while cold, fuel cut on overrun, otherwise
    // closed loop with the upstream sensor switching around lambda 1.
    if run <= 25.0 || s.coolant <= 35.0 {
        s.fuel_status = 0x01;
        s.lambda = 0.94;
        s.o2up = if run < 15.0 { 0.45 } else { 0.62 };
        s.o2down = 0.45;
        s.stft = 0.0;
    } else if fuel_cut {
        s.fuel_status = 0x04;
        s.lambda = 1.99;
        s.o2up = 0.04;
        s.o2down = 0.09;
        s.stft = 0.0;
    } else {
        s.fuel_status = 0x02;
        s.lambda = 1.0;
        let phase = 2.0 * PI * 1.1 * t;
        s.o2up = 0.45 + 0.38 * (3.0 * phase.sin()).tanh();
        s.stft = 2.0 - 5.5 * (3.0 * (phase - 0.5).sin()).tanh();
        // A worn catalyst stores little oxygen, so the downstream sensor
        // follows the upstream one instead of staying calm (P0420).
        s.o2down = 0.47 + 0.24 * (2.0 * (phase - 1.1).sin()).tanh();
    }
    s.fuel_rate = if fuel_cut {
        0.0
    } else {
        s.maf / (14.7 * s.lambda) * 3.6 / 0.745
    };

    let mut cat_target = 430.0;
    if s.phase == Phase::Cruise {
        cat_target = 560.0 + 1.2 * s.speed;
    }
    if s.phase == Phase::Accelerate {
        cat_target = 700.0;
    }
    if s.phase == Phase::Brake {
        cat_target = 520.0;
    }
    s.cat_temp = AMBIENT + (cat_target - AMBIENT) * (1.0 - (-run / 170.0).exp());

    s.battery = 14.2 + 0.06 * (t * 0.9).sin() - 0.5 * (-run / 25.0).exp();

    let fuel_used = 0.74 * run / 3600.0 + 0.068 * s.distance_km;
    s.fuel_level = FUEL_START - fuel_used / TANK_LITRES * 100.0;
    s.odometer = ODOMETER_START + s.distance_km;
    s
}

/// Run time at which the coolant first reaches 70 deg C, which counts as a
/// warm-up cycle.
pub(crate) fn warm_up_run_time() -> f64 {
    200.0 * ((90.0 - AMBIENT) / (90.0 - 70.0)).ln()
}
