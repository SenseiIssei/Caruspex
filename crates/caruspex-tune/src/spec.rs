//! Every number the model uses, with where it comes from.
//!
//! Two kinds of values live here:
//! - **Published**: taken from a public source named in the comment. The
//!   sources are listed again in `docs/site/content/<language>/tuning-simulator.md`.
//! - **Assumption**: a value I chose because nothing freely licensed publishes
//!   it. Each says so and gives the reasoning. Changing an assumption changes
//!   the answers, which is why the interface shows results as estimates.

/// A table of (rpm, value) pairs, sorted by rpm, read with [`interp`].
pub type Table = &'static [(f64, f64)];

/// Linear interpolation in a table, clamped at both ends.
pub fn interp(table: Table, x: f64) -> f64 {
    let first = table[0];
    if x <= first.0 {
        return first.1;
    }
    for pair in table.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        if x <= x1 {
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    table[table.len() - 1].1
}

// ---------------------------------------------------------------------------
// Published: engine geometry and rated output
// ---------------------------------------------------------------------------

/// Displacement in m³: 2,979 cm³. Published: Wikipedia "BMW N55" (CC BY-SA 4.0).
pub const DISPLACEMENT_M3: f64 = 2.979e-3;

/// Cylinders. Published: Wikipedia "BMW N55", straight six.
pub const CYLINDERS: f64 = 6.0;

/// Compression ratio 10.2:1. Published: Wikipedia "BMW N55". Only quoted in
/// the documentation; the efficiency below already contains it.
pub const COMPRESSION_RATIO: f64 = 10.2;

/// Redline 7,000 rpm. Published: Wikipedia "BMW N55".
pub const REDLINE_RPM: f64 = 7000.0;

/// Rated torque 400 Nm from 1,200 to 5,000 rpm. Published: Wikipedia "BMW N55",
/// row N55B30M0 (used in the F30 335i and the F32/F33/F36 435i), and BMW's
/// data for the 435i.
pub const RATED_TORQUE_NM: f64 = 400.0;
pub const RATED_TORQUE_FROM_RPM: f64 = 1200.0;
pub const RATED_TORQUE_TO_RPM: f64 = 5000.0;

/// Rated power 225 kW (306 PS). Wikipedia "BMW N55" gives 5,700 to 5,800 rpm
/// for N55B30M0, BMW's data for the 435i gives 5,800 to 6,000 rpm; the tests
/// accept the whole span from 5,700 to 6,000 rpm.
pub const RATED_POWER_KW: f64 = 225.0;
pub const RATED_POWER_FROM_RPM: f64 = 5700.0;
pub const RATED_POWER_TO_RPM: f64 = 6000.0;

/// Rated input torque of the ZF 8HP45 automatic: 450 Nm. Published:
/// Wikipedia "ZF 8HP transmission", key data table (CC BY-SA 4.0).
pub const GEARBOX_8HP45_NM: f64 = 450.0;

/// The six-speed manual of the F3x 35i models. Assumption: about 450 Nm for
/// gearbox and clutch together. I found no freely licensed source that names
/// the exact type and its rating; BMW's six-speed boxes carry the rated torque
/// in tens of Nm in their type name (GS6-45 = 450 Nm), so check the plate.
pub const GEARBOX_MANUAL_NM: f64 = 450.0;

// ---------------------------------------------------------------------------
// Published physical constants
// ---------------------------------------------------------------------------

/// Gas constant of air, J/(kg K). Textbook value.
pub const R_AIR: f64 = 287.05;

/// (kappa - 1) / kappa of air with kappa = 1.4. Textbook value.
pub const KAPPA_EXP: f64 = 0.2857;

/// Standard atmosphere at sea level, bar. ISO 2533.
pub const P_STANDARD_BAR: f64 = 1.01325;

/// Reference temperature of corrected compressor flow, K (25 °C). The usual
/// convention of compressor maps.
pub const T_CORRECTED_K: f64 = 298.15;

/// Stoichiometric air to fuel ratio of E5 petrol (Super Plus in Germany).
/// Assumption between pure petrol (about 14.7) and E10 (about 14.1).
pub const AFR_STOICH: f64 = 14.4;

/// Lower heating value of E5 petrol, J/kg. Assumption: 42.4 MJ/kg, between
/// petrol without ethanol (about 43 MJ/kg) and ethanol (about 27 MJ/kg) at
/// 5 % ethanol.
pub const LHV_J_PER_KG: f64 = 42.4e6;

// ---------------------------------------------------------------------------
// Assumptions: air path
// ---------------------------------------------------------------------------

/// Volumetric efficiency over rpm at full load, relative to the air density in
/// the manifold. Assumption: a typical shape for a four-valve engine with
/// variable cam timing (Valvetronic, double VANOS): lower at low rpm, best in
/// the middle, falling towards the redline as the ports choke.
pub const VOLUMETRIC_EFFICIENCY: Table = &[
    (1000.0, 0.86),
    (1500.0, 0.90),
    (2000.0, 0.93),
    (2500.0, 0.95),
    (3000.0, 0.96),
    (4000.0, 0.97),
    (4500.0, 0.97),
    (5000.0, 0.96),
    (5500.0, 0.95),
    (6000.0, 0.93),
    (6500.0, 0.90),
    (7000.0, 0.86),
];

/// The highest boost (bar above ambient) the turbo can build at a given rpm,
/// because at low rpm there is too little exhaust energy. Assumption: chosen
/// so the published 400 Nm start at 1,200 rpm and not much earlier.
pub const SPOOL_LIMIT_BAR: Table = &[
    (1000.0, 0.35),
    (1100.0, 0.62),
    (1200.0, 0.95),
    (1300.0, 1.20),
    (1500.0, 1.60),
    (2000.0, 2.00),
];

/// Pressure ratio limit of the compressor (surge and speed line).
/// Assumption: a generic small twin-scroll turbo of this size class, not a
/// measured map of the N55 turbo.
pub const TURBO_MAX_PRESSURE_RATIO: f64 = 2.55;

/// Corrected air flow limit of the compressor (choke), kg/s. Assumption: same
/// generic small compressor; about a quarter above the air flow the stock
/// engine needs at rated power (corrected to 25 °C and 1013 hPa).
pub const TURBO_MAX_FLOW_KG_S: f64 = 0.32;

/// Compressor efficiency in the middle of its map; it falls towards choke.
/// Assumption: typical value for small automotive compressors.
pub const COMPRESSOR_EFFICIENCY: f64 = 0.72;

/// Temperature rise of the intake air before the compressor (engine bay,
/// warm intake pipes), K. Assumption.
pub const INLET_HEAT_K: f64 = 5.0;

/// Intercooler effectiveness: share of the temperature rise it removes.
/// Assumption: an air-to-air intercooler at the front, better at low flow.
pub const INTERCOOLER_EFFECTIVENESS_LOW_FLOW: f64 = 0.85;
/// How much the effectiveness drops at the flow limit of the turbo. Assumption.
pub const INTERCOOLER_EFFECTIVENESS_DROP: f64 = 0.20;

/// Pressure drop over the intercooler and its pipes at 0.25 kg/s, bar; grows
/// with the square of the flow. Assumption.
pub const INTERCOOLER_DROP_BAR: f64 = 0.10;
/// Pressure drop over the air filter at 0.25 kg/s, bar. Assumption.
pub const FILTER_DROP_BAR: f64 = 0.025;
/// Flow at which the two drops above are given, kg/s.
pub const DROP_REFERENCE_FLOW_KG_S: f64 = 0.25;

// ---------------------------------------------------------------------------
// Assumptions: combustion
// ---------------------------------------------------------------------------

/// Indicated efficiency at the best ignition timing (MBT) and lambda 1, before
/// friction. Assumption: in the usual range of modern direct-injection petrol
/// engines at full load (0.36 to 0.42).
pub const INDICATED_EFFICIENCY: f64 = 0.39;

/// Best ignition timing (MBT) at full load, degrees before top dead centre.
/// Assumption: typical for a bore of 84 mm with fast burning.
pub const MBT_TIMING_DEG: Table = &[
    (1000.0, 18.0),
    (2000.0, 22.0),
    (3000.0, 25.0),
    (4000.0, 27.0),
    (5000.0, 29.0),
    (6000.0, 31.0),
    (7000.0, 32.0),
];

/// Knock-limited ignition timing with the stock maps on RON 98 at 20 °C and
/// 1013 hPa, degrees before top dead centre. Assumption: a turbo petrol engine
/// at full load runs far later than MBT at low rpm, where the mixture has the
/// most time to knock, and gets closer at high rpm.
pub const KNOCK_LIMIT_REFERENCE_DEG: Table = &[
    (1000.0, 3.0),
    (1500.0, 3.5),
    (2000.0, 5.0),
    (3000.0, 7.0),
    (4000.0, 9.0),
    (5000.0, 11.0),
    (6000.0, 13.5),
    (7000.0, 15.0),
];

/// Distance the stock ignition map keeps from the knock limit, degrees.
/// Assumption: series calibrations keep a small reserve for fuel and
/// temperature scatter.
pub const STOCK_KNOCK_RESERVE_DEG: f64 = 1.5;

/// Knock limit sensitivities. All assumptions in the range found in general
/// engine literature, not measured on an N55:
/// - degrees per octane number (RON),
pub const KNOCK_DEG_PER_RON: f64 = 1.0;
/// - degrees per kelvin of charge air temperature,
pub const KNOCK_DEG_PER_K: f64 = 0.10;
/// - degrees per bar of manifold pressure,
pub const KNOCK_DEG_PER_BAR: f64 = 6.0;
/// - degrees per unit of lambda richer than stock (0.1 richer = 2.5 degrees).
pub const KNOCK_DEG_PER_LAMBDA: f64 = 25.0;

/// Torque lost when the ignition is away from MBT: share per squared degree.
/// Assumption: the usual parabola, about 4 % at 10 degrees.
pub const SPARK_LOSS_PER_DEG2: f64 = 0.0004;

/// Friction mean effective pressure, bar: a + b * n + c * n² with n in
/// 1000 rpm. Assumption: the quadratic form of engine textbooks with
/// coefficients for a modern six with roller cams.
pub const FRICTION_BAR: [f64; 3] = [0.60, 0.12, 0.022];

/// Stock lambda at full load. Assumption: lambda 1 where the exhaust stays
/// cool enough, richer at high rpm to protect turbo and catalyst, as petrol
/// turbos of that time did.
pub const STOCK_LAMBDA: Table = &[
    (1000.0, 1.00),
    (3500.0, 1.00),
    (4000.0, 0.97),
    (4500.0, 0.93),
    (5000.0, 0.90),
    (5500.0, 0.87),
    (6000.0, 0.85),
    (7000.0, 0.85),
];

/// Stock boost target, bar above standard atmosphere (1013 hPa). The real
/// table is not public. Assumption: back-calculated with this model from the
/// published torque and power curve, so stock reproduces the curve by
/// construction; the values lie in the range of public discussions of this
/// engine (around 0.6 to 0.9 bar). The curve it was fitted to: 400 Nm from
/// 1,200 to 5,000 rpm, power rising linearly to 225 kW at 5,800 rpm, flat to
/// 6,000 rpm and falling to 205 kW at 7,000 rpm; about 290 Nm at 1,000 rpm
/// (the last two are assumptions, nothing is published there). See
/// `tests/model.rs` for the check.
pub const STOCK_BOOST_BAR: Table = &[
    (1000.0, 0.24),
    (1250.0, 0.73),
    (1500.0, 0.72),
    (2000.0, 0.70),
    (2500.0, 0.69),
    (3000.0, 0.71),
    (3500.0, 0.72),
    (4000.0, 0.72),
    (4500.0, 0.73),
    (4750.0, 0.76),
    (5000.0, 0.78),
    (5250.0, 0.75),
    (5500.0, 0.73),
    (5750.0, 0.72),
    (6000.0, 0.69),
    (6250.0, 0.62),
    (6500.0, 0.56),
    (6750.0, 0.51),
    (7000.0, 0.47),
];

// ---------------------------------------------------------------------------
// Assumptions: exhaust gas temperature
// ---------------------------------------------------------------------------

/// Exhaust gas temperature before the turbine with the stock maps at
/// reference conditions, °C. Assumption: typical full-load values of petrol
/// turbos with component protection.
pub const STOCK_EGT_C: Table = &[
    (1000.0, 760.0),
    (2000.0, 820.0),
    (3000.0, 860.0),
    (4000.0, 900.0),
    (5000.0, 930.0),
    (6000.0, 945.0),
    (7000.0, 955.0),
];
/// Kelvin per degree of ignition later than stock (more heat goes out with
/// the exhaust). Assumption.
pub const EGT_K_PER_DEG_RETARD: f64 = 12.0;
/// Kelvin per 0.01 lambda leaner than stock (less fuel to cool). Assumption.
pub const EGT_K_PER_CENTI_LAMBDA: f64 = 7.0;
/// Kelvin per bar more manifold pressure than stock. Assumption.
pub const EGT_K_PER_BAR: f64 = 60.0;
/// Turbine inlet temperature above which the warning starts, °C. Assumption:
/// series turbo housings of petrol engines are usually built for about
/// 950 to 1050 °C; the engine control enriches before that.
pub const EGT_LIMIT_C: f64 = 980.0;

// ---------------------------------------------------------------------------
// Assumptions: fuel system, pre-ignition
// ---------------------------------------------------------------------------

/// Fuel the high-pressure pump can deliver at full rail pressure, kg/s.
/// Assumption: about a third more than stock needs at rated power, a usual
/// design reserve; not measured.
pub const HPFP_MAX_KG_S: f64 = 0.027;

/// Static flow of one direct injector at full rail pressure, kg/s.
/// Assumption for a solenoid DI injector of this size.
pub const INJECTOR_FLOW_KG_S: f64 = 0.014;

/// Share of the engine cycle in which direct injection is possible (intake
/// and early compression stroke). Assumption: about 290 of 720 degrees.
pub const INJECTION_WINDOW: f64 = 0.40;

/// Below this rpm, high load raises the risk of low-speed pre-ignition.
/// Assumption from the general literature on downsized turbo engines.
pub const LSPI_RPM: f64 = 2500.0;
/// Brake mean effective pressure above which that risk counts as raised,
/// bar. Assumption: stock reaches about 16.9 bar (400 Nm); the literature
/// sees the risk climb steeply from about 18 to 20 bar at low rpm.
pub const LSPI_BMEP_BAR: f64 = 18.0;

/// Charge air temperature above which the knock limit falls noticeably, °C.
/// Assumption, used only for a note.
pub const CHARGE_AIR_NOTE_C: f64 = 65.0;

// ---------------------------------------------------------------------------
// Honest uncertainty
// ---------------------------------------------------------------------------

/// How far off the outputs may be, as a rough guide for the interface. These
/// are assumptions too: the model has not been checked against a real N55.
pub mod uncertainty {
    /// Torque and power, share of the value.
    pub const TORQUE_SHARE: f64 = 0.08;
    /// Boost, bar.
    pub const BOOST_BAR: f64 = 0.10;
    /// Charge air temperature, K.
    pub const CHARGE_AIR_K: f64 = 10.0;
    /// Knock margin, degrees.
    pub const KNOCK_DEG: f64 = 3.0;
    /// Exhaust gas temperature, K.
    pub const EGT_K: f64 = 60.0;
}
