//! Small statistics over recorded values, shared by the health checks of
//! cars and trucks. An empty series gives a neutral value instead of a
//! panic; the checks only call these with values they found.

use crate::health_check::TimedValue;

/// The value with the newest time stamp (the last one on a tie).
pub(crate) fn latest(s: &[TimedValue]) -> f64 {
    let mut best: Option<&TimedValue> = None;
    for v in s {
        if best.is_none_or(|b| v.t >= b.t) {
            best = Some(v);
        }
    }
    best.map_or(0.0, |b| b.value)
}

pub(crate) fn max_of(s: &[TimedValue]) -> f64 {
    s.iter().map(|v| v.value).fold(f64::MIN, f64::max)
}

pub(crate) fn min_of(s: &[TimedValue]) -> f64 {
    s.iter().map(|v| v.value).fold(f64::MAX, f64::min)
}

pub(crate) fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

pub(crate) fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        f64::midpoint(v[n / 2 - 1], v[n / 2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(points: &[(f64, f64)]) -> Vec<TimedValue> {
        points
            .iter()
            .map(|&(t, value)| TimedValue { t, value })
            .collect()
    }

    #[test]
    fn latest_takes_the_newest_and_the_last_on_a_tie() {
        let s = series(&[(1.0, 10.0), (3.0, 30.0), (2.0, 20.0)]);
        assert_eq!(latest(&s), 30.0);
        let tie = series(&[(5.0, 1.0), (5.0, 2.0)]);
        assert_eq!(latest(&tie), 2.0);
        assert_eq!(latest(&[]), 0.0);
    }

    #[test]
    fn extremes_mean_and_median() {
        let s = series(&[(0.0, 13.9), (1.0, 14.2), (2.0, 12.1)]);
        assert_eq!(max_of(&s), 14.2);
        assert_eq!(min_of(&s), 12.1);
        assert_eq!(mean(&[1.0, 2.0, 6.0]), 3.0);
        assert_eq!(mean(&[]), 0.0);
        assert_eq!(median(vec![5.0, 1.0, 3.0]), 3.0);
        assert_eq!(median(vec![4.0, 1.0, 3.0, 2.0]), 2.5);
        assert_eq!(median(Vec::new()), 0.0);
    }
}
