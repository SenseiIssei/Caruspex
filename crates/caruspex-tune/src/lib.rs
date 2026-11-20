//! Tuning simulator: a transparent, physics-based full-load model of the BMW
//! N55 engine (3.0 l straight six, single twin-scroll turbo, 225 kW / 400 Nm
//! in the F30 335i and F32/F36 435i).
//!
//! **Simulation only.** This crate computes curves on the desk. It has no way
//! to talk to a car, writes no engine maps, knows no flash file format, no
//! checksum and no map address, and reproduces no commercial tune. Real
//! tuning belongs with established tools and a workshop, with approval of the
//! change (in Germany by the TÜV).
//!
//! What it does:
//! - [`simulate`]: torque, power and the inner values (boost, charge air,
//!   knock margin, exhaust temperature, turbo, fuel system, gearbox) over rpm
//!   for given maps and conditions, with plain warnings for every limit.
//! - [`compare_log`]: measured live values of a recording next to what the
//!   model says at the same rpm and load.
//!
//! Every constant is in [`spec`] with its source or marked as an assumption.
//! The stock boost map is back-calculated from the published curve, so
//! agreement with that curve proves consistency, not truth: only real logs
//! can show how close the model is.
#![forbid(unsafe_code)]

pub mod compare;
pub mod limits;
pub mod model;
pub mod spec;

pub use compare::{CompareBand, LogComparison, LogSample, Pair, compare_log};
pub use limits::{Severity, Warning, WarningKind, warnings};
pub use model::{
    BandOffset, BoostCap, Conditions, Gearbox, Maps, POINTS, Point, RPM_END, RPM_START, RPM_STEP,
    curve, rpm_at,
};

/// A simulated curve with its warnings and peaks.
#[derive(Clone, Debug, PartialEq)]
pub struct Simulation {
    pub points: Vec<Point>,
    pub warnings: Vec<Warning>,
    /// (rpm, Nm)
    pub peak_torque: (f64, f64),
    /// (rpm, kW)
    pub peak_power: (f64, f64),
}

/// The curve for these maps and conditions, with warnings.
pub fn simulate(maps: &Maps, conditions: &Conditions) -> Simulation {
    let points = curve(maps, conditions);
    let warnings = warnings(&points, conditions);
    let peak = |value: fn(&Point) -> f64| {
        points
            .iter()
            .map(|p| (p.rpm, value(p)))
            .fold(
                (0.0, f64::MIN),
                |best, x| if x.1 > best.1 { x } else { best },
            )
    };
    let peak_torque = peak(|p| p.torque_nm);
    let peak_power = peak(|p| p.power_kw);
    Simulation {
        points,
        warnings,
        peak_torque,
        peak_power,
    }
}
