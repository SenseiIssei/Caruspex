//! Performance timer: acceleration, quarter mile and braking from timestamped
//! speed samples (PID 0D).
//!
//! OBD delivers the speed only a few times per second and in whole km/h. To
//! get usable times anyway, crossing points are interpolated linearly between
//! two samples, and the moment of setting off is extrapolated from the first
//! two moving samples. With 5 to 10 samples per second the results are good
//! to roughly 0.2 seconds; the report states the accuracy that fits the
//! actual sample rate.

use crate::common::{Phrase, format, number};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SpeedSample {
    /// Seconds.
    pub t: f64,
    pub kmh: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedRun {
    pub found: bool,
    pub seconds: f64,
    pub start_time: f64,
    pub end_time: f64,
    /// Quarter mile and braking.
    pub distance_meters: f64,
    /// Quarter mile: speed at the finish line.
    pub trap_speed_kmh: f64,
    /// False if samples were more than 1 s apart.
    pub reliable: bool,
}

impl Default for TimedRun {
    fn default() -> Self {
        Self {
            found: false,
            seconds: 0.0,
            start_time: 0.0,
            end_time: 0.0,
            distance_meters: 0.0,
            trap_speed_kmh: 0.0,
            reliable: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct PerformanceReport {
    /// 0-100 km/h.
    pub zero_to_100: TimedRun,
    /// 80-120 km/h.
    pub eighty_to_120: TimedRun,
    /// 0-96.56 km/h.
    pub zero_to_60_mph: TimedRun,
    /// 402.336 m from standstill.
    pub quarter_mile: TimedRun,
    /// Braking from 100 km/h to standstill.
    pub brake_100_to_0: TimedRun,
    pub sample_rate_hz: f64,
    pub accuracy_seconds: f64,
    pub samples_used: i32,
    pub samples_dropped: i32,
    pub accuracy_hint: Phrase,
}

pub const QUARTER_MILE_METERS: f64 = 402.336;
pub const SIXTY_MPH_KMH: f64 = 96.56064;

/// km/h; below this the car stands.
const STANDSTILL: f64 = 0.5;
/// km/h jump that is undone by the next sample.
const SPIKE: f64 = 20.0;
/// s; larger sample gaps make a run unreliable.
const RELIABLE_GAP: f64 = 1.0;
/// km/h a rolling run may sag below its start.
const DROP_TOLERANCE: f64 = 2.0;
/// km/h of speed gain that ends a braking run.
const BRAKE_REBOUND: f64 = 3.0;

fn interpolate_time(a: &SpeedSample, b: &SpeedSample, kmh: f64) -> f64 {
    if b.kmh == a.kmh {
        return b.t;
    }
    a.t + (kmh - a.kmh) / (b.kmh - a.kmh) * (b.t - a.t)
}

/// Better run: reliable before unreliable, then faster (or shorter).
fn better(candidate: &TimedRun, best: &TimedRun, by_distance: bool) -> bool {
    if !candidate.found {
        return false;
    }
    if !best.found {
        return true;
    }
    if candidate.reliable != best.reliable {
        return candidate.reliable;
    }
    if by_distance {
        candidate.distance_meters < best.distance_meters
    } else {
        candidate.seconds < best.seconds
    }
}

/// The moment the car set off. `i` is the last standing sample, `i + 1` the
/// first moving one. The start lies somewhere in between; a straight line
/// through the first two moving samples, followed back to zero, gives a good
/// guess.
fn launch_time(s: &[SpeedSample], i: usize) -> f64 {
    let stand = &s[i];
    let m1 = &s[i + 1];
    if i + 2 < s.len() {
        let m2 = &s[i + 2];
        if m2.kmh > m1.kmh && m2.t > m1.t {
            let slope = (m2.kmh - m1.kmh) / (m2.t - m1.t);
            let t0 = m1.t - m1.kmh / slope;
            // Same as std::clamp(t0, stand.t, m1.t).
            return if t0 < stand.t {
                stand.t
            } else if m1.t < t0 {
                m1.t
            } else {
                t0
            };
        }
    }
    stand.t
}

/// Sorts by time, drops duplicates, impossible values and single-sample
/// spikes. Returns the cleaned series and the number of dropped samples.
pub fn clean_speed_samples(mut samples: Vec<SpeedSample>) -> (Vec<SpeedSample>, i32) {
    let before = samples.len();
    samples.retain(|x| !(!x.t.is_finite() || !x.kmh.is_finite() || x.kmh < 0.0 || x.kmh > 400.0));
    // Stable sort by time, like std::stable_sort.
    samples.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    // Equal timestamps: keep the later reading.
    let mut unique: Vec<SpeedSample> = Vec::new();
    for x in &samples {
        match unique.last_mut() {
            Some(last) if last.t == x.t => *last = *x,
            _ => unique.push(*x),
        }
    }
    // A single reading far off both neighbours, in opposite directions, is
    // a transmission glitch.
    let mut out: Vec<SpeedSample> = Vec::new();
    for i in 0..unique.len() {
        if i > 0 && i + 1 < unique.len() {
            let up = unique[i].kmh - unique[i - 1].kmh;
            let down = unique[i + 1].kmh - unique[i].kmh;
            if up.abs() > SPIKE && down.abs() > SPIKE && up * down < 0.0 {
                continue;
            }
        }
        out.push(unique[i]);
    }
    let dropped = (before - out.len()) as i32;
    (out, dropped)
}

/// The single measurements on a cleaned series. Each returns the best
/// (fastest, or for braking shortest) run in the series.
pub fn measure_from_standstill(s: &[SpeedSample], target_kmh: f64) -> TimedRun {
    let mut best = TimedRun::default();
    for i in 0..s.len().saturating_sub(1) {
        if s[i].kmh >= STANDSTILL || s[i + 1].kmh < STANDSTILL {
            continue;
        }
        let mut run = TimedRun {
            start_time: launch_time(s, i),
            ..Default::default()
        };
        let mut max_gap = s[i + 1].t - s[i].t;
        for j in (i + 1)..s.len() {
            max_gap = max_gap.max(s[j].t - s[j - 1].t);
            if s[j].kmh < STANDSTILL {
                break; // stopped again before the target
            }
            if s[j].kmh >= target_kmh {
                run.end_time = interpolate_time(&s[j - 1], &s[j], target_kmh);
                run.seconds = run.end_time - run.start_time;
                run.found = run.seconds > 0.0;
                run.reliable = max_gap <= RELIABLE_GAP;
                break;
            }
        }
        if better(&run, &best, false) {
            best = run;
        }
    }
    best
}

pub fn measure_rolling(s: &[SpeedSample], from_kmh: f64, to_kmh: f64) -> TimedRun {
    let mut best = TimedRun::default();
    for i in 1..s.len() {
        if !(s[i - 1].kmh < from_kmh && s[i].kmh >= from_kmh) {
            continue;
        }
        let mut run = TimedRun {
            start_time: interpolate_time(&s[i - 1], &s[i], from_kmh),
            ..Default::default()
        };
        let mut max_gap = s[i].t - s[i - 1].t;
        for j in i..s.len() {
            if j > i {
                max_gap = max_gap.max(s[j].t - s[j - 1].t);
            }
            if s[j].kmh < from_kmh - DROP_TOLERANCE {
                break; // lifted off, no run
            }
            if s[j].kmh >= to_kmh {
                run.end_time = if j == i {
                    run.start_time
                } else {
                    interpolate_time(&s[j - 1], &s[j], to_kmh)
                };
                run.seconds = run.end_time - run.start_time;
                run.found = run.seconds > 0.0;
                run.reliable = max_gap <= RELIABLE_GAP;
                break;
            }
        }
        if better(&run, &best, false) {
            best = run;
        }
    }
    best
}

pub fn measure_quarter_mile(s: &[SpeedSample]) -> TimedRun {
    let mut best = TimedRun::default();
    for i in 0..s.len().saturating_sub(1) {
        if s[i].kmh >= STANDSTILL || s[i + 1].kmh < STANDSTILL {
            continue;
        }
        let mut run = TimedRun {
            start_time: launch_time(s, i),
            ..Default::default()
        };
        // From the launch moment the speed rises linearly to the first
        // moving sample; afterwards trapezoids between samples.
        let mut prev = SpeedSample {
            t: run.start_time,
            kmh: 0.0,
        };
        let mut distance = 0.0;
        let mut max_gap = s[i + 1].t - s[i].t;
        for j in (i + 1)..s.len() {
            let cur = s[j];
            if j > i + 1 {
                max_gap = max_gap.max(cur.t - s[j - 1].t);
            }
            if cur.kmh < STANDSTILL {
                break;
            }
            let dt = cur.t - prev.t;
            if dt <= 0.0 {
                continue;
            }
            let va = prev.kmh / 3.6;
            let vb = cur.kmh / 3.6;
            let segment = (va + vb) / 2.0 * dt;
            if distance + segment >= QUARTER_MILE_METERS {
                // Solve for the moment inside the segment where the
                // distance is reached, with the speed rising linearly.
                let remaining = QUARTER_MILE_METERS - distance;
                let a = (vb - va) / dt;
                let tau = if a.abs() < 1e-9 {
                    remaining / va
                } else {
                    (-va + (va * va + 2.0 * a * remaining).max(0.0).sqrt()) / a
                };
                run.end_time = prev.t + tau;
                run.seconds = run.end_time - run.start_time;
                run.trap_speed_kmh = (va + a * tau) * 3.6;
                run.distance_meters = QUARTER_MILE_METERS;
                run.found = true;
                run.reliable = max_gap <= RELIABLE_GAP;
                break;
            }
            distance += segment;
            prev = cur;
        }
        if better(&run, &best, false) {
            best = run;
        }
    }
    best
}

pub fn measure_braking(s: &[SpeedSample], from_kmh: f64) -> TimedRun {
    let mut best = TimedRun::default();
    for i in 1..s.len() {
        if !(s[i - 1].kmh >= from_kmh && s[i].kmh < from_kmh) {
            continue;
        }
        let mut run = TimedRun {
            start_time: interpolate_time(&s[i - 1], &s[i], from_kmh),
            ..Default::default()
        };
        let mut prev = SpeedSample {
            t: run.start_time,
            kmh: from_kmh,
        };
        let mut distance = 0.0;
        let mut lowest = from_kmh;
        let mut max_gap = s[i].t - s[i - 1].t;
        for j in i..s.len() {
            let cur = s[j];
            if j > i {
                max_gap = max_gap.max(cur.t - s[j - 1].t);
            }
            if cur.kmh > lowest + BRAKE_REBOUND {
                break; // braking was interrupted
            }
            lowest = lowest.min(cur.kmh);
            let dt = cur.t - prev.t;
            if dt > 0.0 {
                distance += (prev.kmh + cur.kmh) / 2.0 / 3.6 * dt;
            }
            if cur.kmh < STANDSTILL {
                run.end_time = cur.t;
                run.seconds = run.end_time - run.start_time;
                run.distance_meters = distance;
                run.found = run.seconds > 0.0;
                run.reliable = max_gap <= RELIABLE_GAP;
                break;
            }
            prev = cur;
        }
        if better(&run, &best, true) {
            best = run;
        }
    }
    best
}

/// Everything at once, on raw samples.
pub fn analyze_performance(samples: Vec<SpeedSample>) -> PerformanceReport {
    let mut r = PerformanceReport::default();
    let (s, dropped) = clean_speed_samples(samples);
    r.samples_dropped = dropped;
    r.samples_used = s.len() as i32;

    if s.len() >= 2 {
        let mut gaps: Vec<f64> = (1..s.len()).map(|i| s[i].t - s[i - 1].t).collect();
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_gap = gaps[gaps.len() / 2];
        if median_gap > 0.0 {
            r.sample_rate_hz = 1.0 / median_gap;
        }
        // Interpolation keeps the error at each end below half a sample
        // interval in theory; whole-km/h steps and adapter jitter add to it.
        // In practice about 0.2 s at 5 to 10 Hz, one interval when slower.
        r.accuracy_seconds = 0.2f64.max(median_gap);
    }

    r.zero_to_100 = measure_from_standstill(&s, 100.0);
    r.zero_to_60_mph = measure_from_standstill(&s, SIXTY_MPH_KMH);
    r.eighty_to_120 = measure_rolling(&s, 80.0, 120.0);
    r.quarter_mile = measure_quarter_mile(&s);
    r.brake_100_to_0 = measure_braking(&s, 100.0);

    r.accuracy_hint = format(
        "{} Messwerte pro Sekunde, die Zeiten sind damit auf etwa plus/minus {} s genau. Die \
         Geschwindigkeit über OBD stammt von den Raddrehzahlfühlern und ist in ganzen km/h \
         gestuft; durchdrehende oder blockierende Räder verfälschen sie. Bitte nur dort messen, \
         wo es erlaubt und sicher ist, etwa auf einer abgesperrten Strecke.",
        "{} samples per second, so the times are good to about plus/minus {} s. The speed via \
         OBD comes from the wheel speed sensors and is stepped in whole km/h; spinning or \
         locked wheels distort it. Please only measure where it is legal and safe, such as on \
         a closed track.",
        &[number(r.sample_rate_hz, 1), number(r.accuracy_seconds, 1)],
    );
    if r.sample_rate_hz > 0.0 && r.sample_rate_hz < 4.0 {
        r.accuracy_hint = r.accuracy_hint
            + format(
                "Für brauchbare Zeiten sollten es mindestens 5 Messwerte pro Sekunde sein. \
                 Dafür während der Messung nur die Geschwindigkeit abfragen.",
                "Usable times need at least 5 samples per second. For that, poll only the \
                 speed during the measurement.",
                &[],
            );
    }
    r
}
