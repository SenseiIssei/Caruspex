//! The full-load model: air path, combustion, friction, exhaust, limits.
//!
//! One operating point is computed in three steps:
//! 1. **Air path.** The boost target is capped by what the turbo can build at
//!    this rpm (spool), by its pressure ratio and by its flow. The compressor
//!    heats the air, the intercooler takes part of that back out. Manifold
//!    pressure, charge air temperature and volumetric efficiency give the air
//!    mass per second.
//! 2. **Combustion.** The fuel follows from the air and the lambda target.
//!    The ignition is the stock timing plus the offset, but never later than
//!    the knock limit allows (the engine's knock control would pull it back).
//!    Indicated work is fuel energy times an efficiency that falls with the
//!    distance from the best timing and with lambda away from its optimum.
//! 3. **Brake output.** Friction is subtracted, torque and power follow.
//!
//! Many quantities are computed relative to the stock maps at reference
//! conditions (RON 98, 20 °C, 1013 hPa): the knock limit and the exhaust
//! temperature are known only as shapes, so the model says how far a change
//! moves them from stock rather than pretending to know absolute values.

use std::f64::consts::PI;

use crate::spec::{self, interp};

/// First rpm of the curve.
pub(crate) const RPM_START: f64 = 1000.0;
/// Last rpm of the curve: the redline.
pub(crate) const RPM_END: f64 = spec::REDLINE_RPM;
/// Distance of the points.
pub(crate) const RPM_STEP: f64 = 250.0;
/// Number of points from 1,000 to 7,000 rpm.
pub const POINTS: usize = 25;

/// The rpm of point `i`.
pub fn rpm_at(i: usize) -> f64 {
    RPM_START + RPM_STEP * i as f64
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gearbox {
    /// ZF 8HP45, the eight-speed automatic.
    Automatic8hp45,
    /// The six-speed manual.
    Manual6,
}

impl Gearbox {
    /// Rated input torque, Nm.
    pub(crate) fn rated_nm(self) -> f64 {
        match self {
            Gearbox::Automatic8hp45 => spec::GEARBOX_8HP45_NM,
            Gearbox::Manual6 => spec::GEARBOX_MANUAL_NM,
        }
    }
}

/// Everything around the engine that is not a map.
#[derive(Clone, Debug, PartialEq)]
pub struct Conditions {
    /// Outside air, °C.
    pub ambient_c: f64,
    /// Outside air pressure, hPa (1013 at sea level, about 900 at 1000 m).
    pub ambient_hpa: f64,
    /// Research octane number of the fuel: 95, 98, 100 or 102.
    pub octane: f64,
    pub gearbox: Gearbox,
}

impl Default for Conditions {
    fn default() -> Self {
        Self {
            ambient_c: 20.0,
            ambient_hpa: 1013.25,
            octane: 98.0,
            gearbox: Gearbox::Automatic8hp45,
        }
    }
}

impl Conditions {
    /// Input clamped to what the model can say something about.
    fn clamped(&self) -> Conditions {
        Conditions {
            ambient_c: finite_or(self.ambient_c, 20.0).clamp(-30.0, 50.0),
            ambient_hpa: finite_or(self.ambient_hpa, 1013.25).clamp(600.0, 1100.0),
            octane: finite_or(self.octane, 98.0).clamp(91.0, 102.0),
            gearbox: self.gearbox,
        }
    }
}

/// The three maps the simulator lets you change, one value per point of the
/// rpm grid (`rpm_at(i)`).
#[derive(Clone, Debug, PartialEq)]
pub struct Maps {
    /// Boost target, bar above standard atmosphere (1013 hPa). The engine
    /// control aims at an absolute pressure, so at altitude the turbo has to
    /// work harder for the same target.
    pub boost_bar: [f64; POINTS],
    /// Ignition, degrees more (+) or less (-) than the stock timing.
    pub ignition_offset_deg: [f64; POINTS],
    /// Lambda target at full load.
    pub lambda: [f64; POINTS],
}

/// An offset for one rpm band, as the simulator page sets it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandOffset {
    /// From this rpm (inclusive) ...
    pub from_rpm: f64,
    /// ... up to this rpm (exclusive, inclusive for the band that ends at
    /// the redline).
    pub to_rpm: f64,
    pub boost_bar: f64,
    pub ignition_deg: f64,
    pub lambda: f64,
}

/// Limits of the inputs. Outside these the model has nothing to say.
pub(crate) const BOOST_RANGE_BAR: (f64, f64) = (0.0, 1.8);
pub(crate) const IGNITION_OFFSET_RANGE_DEG: (f64, f64) = (-15.0, 10.0);
pub(crate) const LAMBDA_RANGE: (f64, f64) = (0.70, 1.10);

impl Maps {
    /// The stock maps ("Serie").
    pub fn stock() -> Maps {
        let mut maps = Maps {
            boost_bar: [0.0; POINTS],
            ignition_offset_deg: [0.0; POINTS],
            lambda: [1.0; POINTS],
        };
        for i in 0..POINTS {
            let rpm = rpm_at(i);
            maps.boost_bar[i] = interp(spec::STOCK_BOOST_BAR, rpm);
            maps.lambda[i] = interp(spec::STOCK_LAMBDA, rpm);
        }
        maps
    }

    /// These maps with offsets per rpm band added on top.
    pub fn with_bands(&self, bands: &[BandOffset]) -> Maps {
        let mut maps = self.clone();
        for i in 0..POINTS {
            let rpm = rpm_at(i);
            for band in bands {
                let inside = rpm >= band.from_rpm
                    && (rpm < band.to_rpm || (rpm == band.to_rpm && band.to_rpm >= RPM_END));
                if inside {
                    maps.boost_bar[i] += finite_or(band.boost_bar, 0.0);
                    maps.ignition_offset_deg[i] += finite_or(band.ignition_deg, 0.0);
                    maps.lambda[i] += finite_or(band.lambda, 0.0);
                }
            }
        }
        maps.clamped()
    }

    /// The maps with every value inside the input limits.
    pub fn clamped(&self) -> Maps {
        let mut maps = self.clone();
        for i in 0..POINTS {
            maps.boost_bar[i] =
                finite_or(maps.boost_bar[i], 0.0).clamp(BOOST_RANGE_BAR.0, BOOST_RANGE_BAR.1);
            maps.ignition_offset_deg[i] = finite_or(maps.ignition_offset_deg[i], 0.0)
                .clamp(IGNITION_OFFSET_RANGE_DEG.0, IGNITION_OFFSET_RANGE_DEG.1);
            maps.lambda[i] = finite_or(maps.lambda[i], 1.0).clamp(LAMBDA_RANGE.0, LAMBDA_RANGE.1);
        }
        maps
    }

    /// Values at any rpm between the grid points, linearly interpolated.
    pub fn at(&self, rpm: f64) -> (f64, f64, f64) {
        let x = ((rpm - RPM_START) / RPM_STEP).clamp(0.0, (POINTS - 1) as f64);
        let i = (x.floor() as usize).min(POINTS - 2);
        let f = x - i as f64;
        let mix = |a: &[f64; POINTS]| a[i] + (a[i + 1] - a[i]) * f;
        (
            mix(&self.boost_bar),
            mix(&self.ignition_offset_deg),
            mix(&self.lambda),
        )
    }
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

/// What capped the boost below its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoostCap {
    None,
    /// Too little exhaust energy at this rpm.
    Spool,
    /// The compressor's pressure ratio limit.
    PressureRatio,
    /// The compressor's flow limit.
    Flow,
}

/// One computed point of the curve. All values are estimates.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub rpm: f64,
    pub torque_nm: f64,
    pub power_kw: f64,
    pub power_ps: f64,
    /// Boost target asked for, bar above standard atmosphere.
    pub boost_target_bar: f64,
    /// Boost reached, bar above the ambient pressure.
    pub boost_bar: f64,
    /// Manifold pressure, kPa absolute (what OBD PID 0x0B reports).
    pub manifold_kpa: f64,
    pub boost_cap: BoostCap,
    pub charge_air_c: f64,
    pub air_mass_kg_h: f64,
    /// Compressor pressure ratio.
    pub pressure_ratio: f64,
    /// Share of the turbo's limit in use: the larger of pressure ratio and
    /// flow, each against its limit. 1.0 = at the limit.
    pub turbo_load: f64,
    pub lambda: f64,
    /// Ignition the map asks for, degrees before top dead centre.
    pub timing_requested_deg: f64,
    /// Ignition after knock control, degrees before top dead centre.
    pub timing_deg: f64,
    /// Knock limit minus requested ignition, degrees. Below 0 the knock
    /// control pulls timing.
    pub knock_margin_deg: f64,
    pub egt_c: f64,
    pub fuel_kg_h: f64,
    /// Share of the high-pressure pump's delivery in use.
    pub pump_load: f64,
    /// Share of the injection window in use.
    pub injector_load: f64,
    /// Brake mean effective pressure, bar (the measure for pre-ignition risk).
    pub bmep_bar: f64,
    /// Engine torque against the gearbox rating.
    pub gearbox_load: f64,
}

/// Conditions for one point. `measured` replaces the air path with logged
/// values (manifold pressure in bar absolute, charge air temperature in K),
/// for the comparison with a recording.
#[derive(Clone, Copy)]
pub(crate) struct Measured {
    pub manifold_bar: f64,
    pub charge_air_k: f64,
}

struct AirPath {
    p_amb: f64,
    p_manifold: f64,
    t_charge: f64,
    air_kg_s: f64,
    pressure_ratio: f64,
    flow_corrected: f64,
    cap: BoostCap,
}

fn air_density(p_bar: f64, t_k: f64) -> f64 {
    p_bar * 1e5 / (spec::R_AIR * t_k)
}

fn air_mass(rpm: f64, p_bar: f64, t_k: f64) -> f64 {
    let ve = interp(spec::VOLUMETRIC_EFFICIENCY, rpm);
    // A four-stroke fills its displacement once every two revolutions.
    ve * air_density(p_bar, t_k) * spec::DISPLACEMENT_M3 * rpm / 120.0
}

fn air_path(
    rpm: f64,
    target_abs_bar: f64,
    cond: &Conditions,
    measured: Option<Measured>,
) -> AirPath {
    let p_amb = cond.ambient_hpa / 1000.0;
    let t_amb = cond.ambient_c + 273.15;
    let t_inlet = t_amb + spec::INLET_HEAT_K;
    let drop = |base: f64, flow: f64| base * (flow / spec::DROP_REFERENCE_FLOW_KG_S).powi(2);

    if let Some(m) = measured {
        let air = air_mass(rpm, m.manifold_bar, m.charge_air_k);
        let p1 = p_amb - drop(spec::FILTER_DROP_BAR, air);
        let p2 = m.manifold_bar + drop(spec::INTERCOOLER_DROP_BAR, air);
        return AirPath {
            p_amb,
            p_manifold: m.manifold_bar,
            t_charge: m.charge_air_k,
            air_kg_s: air,
            pressure_ratio: (p2 / p1).max(1.0),
            flow_corrected: air * (t_inlet / spec::T_CORRECTED_K).sqrt()
                / (p1 / spec::P_STANDARD_BAR),
            cap: BoostCap::None,
        };
    }

    // At full load the throttle is open: never below ambient minus the
    // filter loss, even when the target asks for less.
    let mut wanted = target_abs_bar.max(p_amb - 0.03);
    let mut spooled = false;
    let spool = p_amb + interp(spec::SPOOL_LIMIT_BAR, rpm);
    if wanted > spool {
        wanted = spool;
        spooled = true;
    }

    let mut p = wanted;
    let mut cap = BoostCap::None;
    let mut t_charge = t_amb + 20.0;
    let mut pr = 1.0;
    let mut flow_corrected = 0.0;
    let corrected = |air: f64, p1: f64| {
        air * (t_inlet / spec::T_CORRECTED_K).sqrt() / (p1 / spec::P_STANDARD_BAR)
    };
    // Pressure drops, compressor heat and intercooler depend on the flow,
    // the flow on them; a few rounds settle it far below the model's error.
    // Every round starts from the wanted pressure again, so where the turbo
    // caps the boost the result does not depend on how far the target was
    // above the cap.
    for _ in 0..6 {
        p = wanted;
        cap = if spooled {
            BoostCap::Spool
        } else {
            BoostCap::None
        };
        let mut air = air_mass(rpm, p, t_charge);
        let mut p1 = p_amb - drop(spec::FILTER_DROP_BAR, air);
        flow_corrected = corrected(air, p1);
        if flow_corrected > spec::TURBO_MAX_FLOW_KG_S {
            // At a given charge temperature the flow grows with the manifold
            // pressure; two steps take the filter drop into account.
            for _ in 0..2 {
                p *= spec::TURBO_MAX_FLOW_KG_S / flow_corrected;
                air = air_mass(rpm, p, t_charge);
                p1 = p_amb - drop(spec::FILTER_DROP_BAR, air);
                flow_corrected = corrected(air, p1);
            }
            cap = BoostCap::Flow;
        }
        let mut p2 = p + drop(spec::INTERCOOLER_DROP_BAR, air);
        if p2 / p1 > spec::TURBO_MAX_PRESSURE_RATIO {
            p2 = spec::TURBO_MAX_PRESSURE_RATIO * p1;
            p = p2 - drop(spec::INTERCOOLER_DROP_BAR, air);
            cap = BoostCap::PressureRatio;
            flow_corrected = corrected(air_mass(rpm, p, t_charge), p1);
        }
        pr = (p2 / p1).max(1.0);
        let flow_share = flow_corrected / spec::TURBO_MAX_FLOW_KG_S;
        // Efficiency drops once the compressor nears choke.
        let eta = spec::COMPRESSOR_EFFICIENCY - 0.25 * (flow_share - 0.75).max(0.0);
        let t2 = t_inlet * (1.0 + (pr.powf(spec::KAPPA_EXP) - 1.0) / eta);
        let effectiveness = spec::INTERCOOLER_EFFECTIVENESS_LOW_FLOW
            - spec::INTERCOOLER_EFFECTIVENESS_DROP * flow_share.min(1.2);
        t_charge = t2 - effectiveness * (t2 - t_amb);
    }
    let air = air_mass(rpm, p, t_charge);
    AirPath {
        p_amb,
        p_manifold: p,
        t_charge,
        air_kg_s: air,
        pressure_ratio: pr,
        flow_corrected,
        cap,
    }
}

/// Power per unit of air relative to lambda 1. Rich mixtures give a little
/// more (the most at about lambda 0.88), lean ones less. Assumption: the
/// shape of the textbook curve, about 2 % gain at its best.
fn lambda_factor(lambda: f64) -> f64 {
    if lambda <= 1.0 {
        let r = 1.0 - lambda;
        1.0 + 0.35 * r - 1.6 * r * r
    } else {
        1.0 - 0.9 * (lambda - 1.0)
    }
}

fn spark_factor(timing: f64, mbt: f64) -> f64 {
    let d = mbt - timing;
    (1.0 - spec::SPARK_LOSS_PER_DEG2 * d * d).max(0.5)
}

/// The stock timing at an rpm: MBT or the knock limit minus the reserve,
/// whichever is earlier.
fn stock_timing(rpm: f64) -> f64 {
    let mbt = interp(spec::MBT_TIMING_DEG, rpm);
    let limit = interp(spec::KNOCK_LIMIT_REFERENCE_DEG, rpm);
    mbt.min(limit - spec::STOCK_KNOCK_RESERVE_DEG)
}

/// What the stock maps give at reference conditions: the anchor for the
/// knock limit and the exhaust temperature.
struct Reference {
    p_manifold: f64,
    t_charge: f64,
    lambda: f64,
    timing: f64,
}

fn reference(rpm: f64) -> Reference {
    let reference = Conditions::default();
    let target = spec::P_STANDARD_BAR + interp(spec::STOCK_BOOST_BAR, rpm);
    let air = air_path(rpm, target, &reference, None);
    Reference {
        p_manifold: air.p_manifold,
        t_charge: air.t_charge,
        lambda: interp(spec::STOCK_LAMBDA, rpm),
        timing: stock_timing(rpm),
    }
}

/// One operating point at full load.
pub(crate) fn point(rpm: f64, maps: &Maps, cond: &Conditions, measured: Option<Measured>) -> Point {
    let (boost_target, ignition_offset, lambda) = maps.at(rpm);
    let air = air_path(rpm, spec::P_STANDARD_BAR + boost_target, cond, measured);
    let reference = reference(rpm);

    // Ignition: what the map asks for, and what knock control lets through.
    let mbt = interp(spec::MBT_TIMING_DEG, rpm);
    let knock_limit = interp(spec::KNOCK_LIMIT_REFERENCE_DEG, rpm)
        + spec::KNOCK_DEG_PER_RON * (cond.octane - 98.0)
        - spec::KNOCK_DEG_PER_K * (air.t_charge - reference.t_charge)
        - spec::KNOCK_DEG_PER_BAR * (air.p_manifold - reference.p_manifold)
        + spec::KNOCK_DEG_PER_LAMBDA * (reference.lambda - lambda);
    let requested = stock_timing(rpm) + ignition_offset;
    let timing = requested.min(knock_limit);

    // Work per second from the air: stoichiometric fuel energy times the
    // efficiencies.
    let efficiency = spec::INDICATED_EFFICIENCY * spark_factor(timing, mbt) * lambda_factor(lambda);
    let indicated_w = air.air_kg_s / spec::AFR_STOICH * spec::LHV_J_PER_KG * efficiency;
    let cycles_per_s = rpm / 120.0;
    let imep_pa = indicated_w / cycles_per_s / spec::DISPLACEMENT_M3;
    let n = rpm / 1000.0;
    let [a, b, c] = spec::FRICTION_BAR;
    let fmep_pa = (a + b * n + c * n * n) * 1e5;
    let bmep_pa = (imep_pa - fmep_pa).max(0.0);
    // Four-stroke: one power stroke per cylinder every 4 pi of crank angle.
    let torque = bmep_pa * spec::DISPLACEMENT_M3 / (4.0 * PI);
    let power_w = torque * rpm * 2.0 * PI / 60.0;

    let fuel_kg_s = air.air_kg_s / (spec::AFR_STOICH * lambda);
    let injector_duty = fuel_kg_s / spec::CYLINDERS / spec::INJECTOR_FLOW_KG_S;

    let retard_change = (mbt - timing) - (mbt - reference.timing).max(0.0);
    let egt = interp(spec::STOCK_EGT_C, rpm)
        + spec::EGT_K_PER_DEG_RETARD * retard_change
        + spec::EGT_K_PER_CENTI_LAMBDA * (lambda - reference.lambda) * 100.0
        + spec::EGT_K_PER_BAR * (air.p_manifold - reference.p_manifold)
        + 0.5 * (air.t_charge - reference.t_charge);

    let turbo_load = (air.pressure_ratio / spec::TURBO_MAX_PRESSURE_RATIO)
        .max(air.flow_corrected / spec::TURBO_MAX_FLOW_KG_S);

    Point {
        rpm,
        torque_nm: torque,
        power_kw: power_w / 1000.0,
        // 1 PS = 735.49875 W (DIN 70020).
        power_ps: power_w / 735.49875,
        boost_target_bar: boost_target,
        boost_bar: air.p_manifold - air.p_amb,
        manifold_kpa: air.p_manifold * 100.0,
        boost_cap: air.cap,
        charge_air_c: air.t_charge - 273.15,
        air_mass_kg_h: air.air_kg_s * 3600.0,
        pressure_ratio: air.pressure_ratio,
        turbo_load,
        lambda,
        timing_requested_deg: requested,
        timing_deg: timing,
        knock_margin_deg: knock_limit - requested,
        egt_c: egt,
        fuel_kg_h: fuel_kg_s * 3600.0,
        pump_load: fuel_kg_s / spec::HPFP_MAX_KG_S,
        injector_load: injector_duty / spec::INJECTION_WINDOW,
        bmep_bar: bmep_pa / 1e5,
        gearbox_load: torque / cond.gearbox.rated_nm(),
    }
}

/// The whole curve from 1,000 to 7,000 rpm.
pub fn curve(maps: &Maps, cond: &Conditions) -> Vec<Point> {
    let maps = maps.clamped();
    let cond = cond.clamped();
    (0..POINTS)
        .map(|i| point(rpm_at(i), &maps, &cond, None))
        .collect()
}

/// One point at any rpm between 1,000 and 7,000, for the log comparison.
pub(crate) fn point_at(
    rpm: f64,
    maps: &Maps,
    cond: &Conditions,
    measured: Option<Measured>,
) -> Point {
    let rpm = rpm.clamp(RPM_START, RPM_END);
    point(rpm, &maps.clamped(), &cond.clamped(), measured)
}
