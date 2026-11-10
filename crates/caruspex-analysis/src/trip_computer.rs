//! Trip computer: distance, fuel, consumption, CO2, cost and an eco score
//! from timestamped live values.
//!
//! Feed it every reading as it arrives (PID, value, time in seconds). The
//! computer keeps the latest value of each PID and integrates over the time
//! between readings. Readings older than `max_gap_seconds` are treated as
//! unknown, and a jump in time larger than that is a gap that is skipped
//! instead of bridged, so a dropped connection does not invent kilometres.
//!
//! Fuel flow comes from the best source the car offers:
//!   1. engine fuel rate (PID 5E), measured by the control unit
//!   2. mass air flow (PID 10) divided by the stoichiometric air/fuel ratio
//!   3. speed density: manifold pressure (0B), intake temperature (0F) and
//!      engine speed (0C) with displacement and volumetric efficiency from the
//!      caller
//!
//! The MAF and speed-density routes assume a stoichiometric mixture. That is
//! close for petrol engines in normal driving, but diesels run with a lot of
//! excess air, so there they give only a rough upper estimate.

use std::collections::{BTreeMap, HashSet, VecDeque};

use crate::common::{FuelType, Phrase, format, number};
use crate::inputs::LiveValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum FuelSource {
    #[default]
    None,
    FuelRate,
    MassAirFlow,
    SpeedDensity,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TripConfig {
    pub fuel: FuelType,
    /// Needed for speed density only.
    pub displacement_litres: f64,
    /// Assumed filling of the cylinders.
    pub volumetric_efficiency: f64,
    /// 0 = no cost.
    pub fuel_price_per_litre: f64,
    /// Longer pauses are gaps.
    pub max_gap_seconds: f64,
    /// Smoothing of the live consumption.
    pub live_window_seconds: f64,
}

impl Default for TripConfig {
    fn default() -> Self {
        Self {
            fuel: FuelType::Petrol,
            displacement_litres: 0.0,
            volumetric_efficiency: 0.85,
            fuel_price_per_litre: 0.0,
            max_gap_seconds: 5.0,
            live_window_seconds: 5.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct EcoScore {
    /// 0..100.
    pub total: i32,
    /// Gentle acceleration and braking.
    pub smoothness: i32,
    /// Staying in a sensible rpm band (-1 = not known).
    pub engine_speed: i32,
    /// Little standing with the engine running.
    pub idling: i32,
    pub harsh_accelerations: i32,
    pub harsh_brakings: i32,
    /// What went into it, with tips.
    pub explanation: Phrase,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TripSummary {
    pub distance_km: f64,
    pub fuel_litres: f64,
    /// Engine on, including stops.
    pub driving_seconds: f64,
    pub moving_seconds: f64,
    /// Engine running, speed 0.
    pub idle_seconds: f64,
    /// Time skipped because data was missing.
    pub gap_seconds: f64,

    /// 0 if not enough distance yet.
    pub average_litres_per_100_km: f64,
    /// Over the last few seconds, 0 when standing.
    pub live_litres_per_100_km: f64,
    /// Current fuel flow.
    pub live_litres_per_hour: f64,
    /// Average while idling.
    pub idle_litres_per_hour: f64,
    pub co2_grams_per_km: f64,
    pub co2_kg: f64,
    pub cost: f64,

    /// Distance / driving time.
    pub average_speed_kmh: f64,
    /// Distance / moving time.
    pub average_moving_speed_kmh: f64,
    pub max_speed_kmh: f64,
    /// 0..1 of driving time.
    pub idle_share: f64,

    /// The one used most.
    pub fuel_source: FuelSource,
    /// MAF or speed density on a diesel, or speed density at all.
    pub fuel_is_rough: bool,
    /// How fuel was determined and how exact it is.
    pub fuel_note: Phrase,
    pub eco: EcoScore,
}

/// Thresholds of the eco score.
pub mod eco {
    /// m/s^2, about 9 km/h per second.
    pub const HARSH_ACCEL: f64 = 2.5;
    /// m/s^2.
    pub const HARSH_BRAKE: f64 = -3.0;
    pub const HIGH_RPM_PETROL: f64 = 3000.0;
    pub const HIGH_RPM_DIESEL: f64 = 2500.0;
}

const PID_RPM: i32 = 0x0C;
const PID_SPEED: i32 = 0x0D;
const PID_MAP: i32 = 0x0B;
const PID_INTAKE_TEMP: i32 = 0x0F;
const PID_MAF: i32 = 0x10;
const PID_FUEL_RATE: i32 = 0x5E;

const ENGINE_ON_RPM: f64 = 300.0;
const MOVING_KMH: f64 = 1.0;
/// J/(kg K), dry air.
const AIR_GAS_CONSTANT: f64 = 287.05;
/// m/s^2; faster changes are glitches.
const MAX_PLAUSIBLE_ACCEL: f64 = 15.0;
/// s over which acceleration is judged.
const ACCEL_WINDOW: f64 = 1.0;

#[derive(Clone, Copy, Debug, Default)]
struct Held {
    value: f64,
    t: f64,
    valid: bool,
}

#[derive(Clone, Copy, Debug)]
struct WindowStep {
    t: f64,
    fuel: f64,
    distance: f64,
}

#[derive(Clone, Debug)]
pub struct TripComputer {
    config: TripConfig,
    held: BTreeMap<i32, Held>,
    last_tick: f64,
    started: bool,

    distance_km: f64,
    fuel_litres: f64,
    driving_seconds: f64,
    moving_seconds: f64,
    idle_seconds: f64,
    idle_fuel: f64,
    gap_seconds: f64,
    max_speed: f64,
    source_seconds: BTreeMap<FuelSource, f64>,
    window: VecDeque<WindowStep>,
    live_rate: f64,

    // Eco tracking
    high_rpm_seconds: f64,
    rpm_known_driving_seconds: f64,
    harsh_accel: i32,
    harsh_brake: i32,
    in_harsh_accel: bool,
    in_harsh_brake: bool,
    anchor_t: f64,
    anchor_v: f64,
    anchor_valid: bool,
    last_speed: f64,
    last_speed_t: f64,
    last_speed_valid: bool,
    /// A suspicious speed jump, kept until confirmed.
    pending_jump: f64,
    pending_jump_valid: bool,
}

impl Default for TripComputer {
    fn default() -> Self {
        Self::new(TripConfig::default())
    }
}

impl TripComputer {
    pub fn new(mut config: TripConfig) -> Self {
        if config.max_gap_seconds <= 0.0 {
            config.max_gap_seconds = 5.0;
        }
        if config.live_window_seconds <= 0.0 {
            config.live_window_seconds = 5.0;
        }
        if config.volumetric_efficiency <= 0.0 {
            config.volumetric_efficiency = 0.85;
        }
        Self {
            config,
            held: BTreeMap::new(),
            last_tick: 0.0,
            started: false,
            distance_km: 0.0,
            fuel_litres: 0.0,
            driving_seconds: 0.0,
            moving_seconds: 0.0,
            idle_seconds: 0.0,
            idle_fuel: 0.0,
            gap_seconds: 0.0,
            max_speed: 0.0,
            source_seconds: BTreeMap::new(),
            window: VecDeque::new(),
            live_rate: 0.0,
            high_rpm_seconds: 0.0,
            rpm_known_driving_seconds: 0.0,
            harsh_accel: 0,
            harsh_brake: 0,
            in_harsh_accel: false,
            in_harsh_brake: false,
            anchor_t: 0.0,
            anchor_v: 0.0,
            anchor_valid: false,
            last_speed: 0.0,
            last_speed_t: 0.0,
            last_speed_valid: false,
            pending_jump: 0.0,
            pending_jump_valid: false,
        }
    }

    pub fn config(&self) -> &TripConfig {
        &self.config
    }

    /// Kg air per kg fuel.
    pub fn stoichiometric_ratio(fuel: FuelType) -> f64 {
        if fuel == FuelType::Diesel { 14.5 } else { 14.7 }
    }

    pub fn density_kg_per_litre(fuel: FuelType) -> f64 {
        if fuel == FuelType::Diesel {
            0.832
        } else {
            0.745
        }
    }

    pub fn co2_kg_per_litre(fuel: FuelType) -> f64 {
        if fuel == FuelType::Diesel { 2.65 } else { 2.37 }
    }

    /// Fuel flow in l/h from mass air flow (g/s) for the configured fuel.
    pub fn fuel_rate_from_maf(&self, grams_per_second: f64) -> f64 {
        // g air/s -> g fuel/s -> l fuel/s -> l/h
        let fuel_grams_per_second = grams_per_second / Self::stoichiometric_ratio(self.config.fuel);
        fuel_grams_per_second / (Self::density_kg_per_litre(self.config.fuel) * 1000.0) * 3600.0
    }

    /// Air mass flow in g/s from speed density.
    pub fn air_flow_from_speed_density(&self, map_kpa: f64, intake_c: f64, rpm: f64) -> f64 {
        // Ideal gas: the air in one full engine displacement is p V / (R T).
        // A four-stroke engine fills its displacement once every two turns.
        let pressure_pa = map_kpa * 1000.0;
        let volume_m3 = self.config.displacement_litres / 1000.0;
        let temperature_k = intake_c + 273.15;
        if temperature_k <= 0.0 {
            return 0.0;
        }
        let kg_per_fill = pressure_pa * volume_m3 * self.config.volumetric_efficiency
            / (AIR_GAS_CONSTANT * temperature_k);
        kg_per_fill * 1000.0 * (rpm / 120.0)
    }

    /// Forgets everything but the configuration.
    pub fn reset(&mut self) {
        *self = TripComputer::new(self.config);
    }

    pub fn add_snapshot(&mut self, values: &[LiveValue], t_seconds: f64) {
        // Only the first value of each PID: the primary one, see
        // HealthInput::add_snapshot.
        let mut seen: HashSet<i32> = HashSet::new();
        for v in values {
            if seen.insert(v.pid) {
                self.add_sample(v.pid, v.value, t_seconds);
            }
        }
    }

    fn fresh(&self, pid: i32, t: f64) -> Option<Held> {
        let h = self.held.get(&pid)?;
        if !h.valid {
            return None;
        }
        if t - h.t > self.config.max_gap_seconds {
            return None;
        }
        Some(*h)
    }

    fn current_fuel_rate(&self, t: f64) -> (f64, FuelSource) {
        let rpm = self.fresh(PID_RPM, t);
        if rpm.is_some_and(|r| r.value < ENGINE_ON_RPM) {
            return (0.0, FuelSource::None); // engine off burns nothing
        }
        if let Some(rate) = self.fresh(PID_FUEL_RATE, t) {
            return (rate.value.max(0.0), FuelSource::FuelRate);
        }
        if let Some(maf) = self.fresh(PID_MAF, t) {
            return (
                self.fuel_rate_from_maf(maf.value.max(0.0)),
                FuelSource::MassAirFlow,
            );
        }
        let map = self.fresh(PID_MAP, t);
        let iat = self.fresh(PID_INTAKE_TEMP, t);
        if let (Some(map), Some(iat), Some(rpm)) = (map, iat, rpm)
            && self.config.displacement_litres > 0.0
        {
            return (
                self.fuel_rate_from_maf(
                    self.air_flow_from_speed_density(map.value, iat.value, rpm.value),
                ),
                FuelSource::SpeedDensity,
            );
        }
        (0.0, FuelSource::None)
    }

    fn integrate(&mut self, t0: f64, t1: f64) {
        let dt = t1 - t0;
        if dt <= 0.0 {
            return;
        }
        let speed = self.fresh(PID_SPEED, t1);
        let rpm = self.fresh(PID_RPM, t1);
        let (rate, source) = self.current_fuel_rate(t1);
        if speed.is_none() && rpm.is_none() && source == FuelSource::None {
            return; // nothing known
        }

        let engine_on = match rpm {
            Some(r) => r.value >= ENGINE_ON_RPM,
            None => source != FuelSource::None || speed.is_some(),
        };

        let v = speed.map_or(0.0, |s| s.value);
        let d_distance = v * dt / 3600.0;
        let d_fuel = if engine_on { rate * dt / 3600.0 } else { 0.0 };

        self.distance_km += d_distance;
        self.fuel_litres += d_fuel;
        if engine_on {
            self.driving_seconds += dt;
            if source != FuelSource::None {
                *self.source_seconds.entry(source).or_insert(0.0) += dt;
            }
        }
        if v >= MOVING_KMH {
            self.moving_seconds += dt;
        }
        if engine_on && speed.is_some() && v < MOVING_KMH {
            self.idle_seconds += dt;
            self.idle_fuel += d_fuel;
        }
        self.live_rate = if engine_on { rate } else { 0.0 };

        self.window.push_back(WindowStep {
            t: t1,
            fuel: d_fuel,
            distance: d_distance,
        });
        while self
            .window
            .front()
            .is_some_and(|w| w.t < t1 - self.config.live_window_seconds)
        {
            self.window.pop_front();
        }

        if let Some(rpm) = rpm
            && engine_on
            && v >= 5.0
        {
            self.rpm_known_driving_seconds += dt;
            let limit = if self.config.fuel == FuelType::Diesel {
                eco::HIGH_RPM_DIESEL
            } else {
                eco::HIGH_RPM_PETROL
            };
            if rpm.value > limit {
                self.high_rpm_seconds += dt;
            }
        }
    }

    fn track_speed(&mut self, kmh: f64, t: f64) {
        self.max_speed = self.max_speed.max(kmh);
        if !self.anchor_valid
            || t - self.anchor_t > self.config.max_gap_seconds
            || t < self.anchor_t
        {
            self.anchor_t = t;
            self.anchor_v = kmh;
            self.anchor_valid = true;
            return;
        }
        // Speed comes in steps of 1 km/h. Over a fraction of a second that step
        // alone looks like a hard acceleration, so acceleration is judged over at
        // least a second.
        if t - self.anchor_t < ACCEL_WINDOW {
            return;
        }
        let a = (kmh - self.anchor_v) / 3.6 / (t - self.anchor_t);
        if a > eco::HARSH_ACCEL {
            if !self.in_harsh_accel {
                self.harsh_accel += 1;
            }
            self.in_harsh_accel = true;
        } else {
            self.in_harsh_accel = false;
        }
        if a < eco::HARSH_BRAKE {
            if !self.in_harsh_brake {
                self.harsh_brake += 1;
            }
            self.in_harsh_brake = true;
        } else {
            self.in_harsh_brake = false;
        }
        self.anchor_t = t;
        self.anchor_v = kmh;
    }

    pub fn add_sample(&mut self, pid: i32, value: f64, t_seconds: f64) {
        let mut t = t_seconds;
        if !value.is_finite() || !t.is_finite() {
            return;
        }

        if pid == PID_SPEED {
            if !(0.0..=400.0).contains(&value) {
                return;
            }
            // A single reading that would need an impossible acceleration is a
            // transmission glitch. It is dropped, unless the new value repeats,
            // which means the speed really changed.
            if self.last_speed_valid
                && t >= self.last_speed_t
                && t - self.last_speed_t <= self.config.max_gap_seconds
            {
                let dt = (t - self.last_speed_t).max(0.1);
                let a = (value - self.last_speed).abs() / 3.6 / dt;
                if a > MAX_PLAUSIBLE_ACCEL
                    && (!self.pending_jump_valid || (value - self.pending_jump).abs() > 5.0)
                {
                    self.pending_jump = value;
                    self.pending_jump_valid = true;
                    return;
                }
            }
            self.pending_jump_valid = false;
        }

        if !self.started {
            self.started = true;
            self.last_tick = t;
        } else if t < self.last_tick {
            if self.last_tick - t > self.config.max_gap_seconds {
                // The clock jumped back: start a new time base, keep the totals.
                for h in self.held.values_mut() {
                    h.valid = false;
                }
                self.anchor_valid = false;
                self.last_tick = t;
            } else {
                // Slightly out of order: use the value, but do not integrate
                // backwards in time.
                t = self.last_tick;
            }
        } else {
            let dt = t - self.last_tick;
            if dt > self.config.max_gap_seconds {
                self.gap_seconds += dt;
                self.anchor_valid = false;
            } else {
                self.integrate(self.last_tick, t);
            }
            self.last_tick = t;
        }

        self.held.insert(
            pid,
            Held {
                value,
                t,
                valid: true,
            },
        );
        if pid == PID_SPEED {
            self.last_speed = value;
            self.last_speed_t = t;
            self.last_speed_valid = true;
            self.track_speed(value, t);
        }
    }

    pub fn summary(&self) -> TripSummary {
        let mut s = TripSummary {
            distance_km: self.distance_km,
            fuel_litres: self.fuel_litres,
            driving_seconds: self.driving_seconds,
            moving_seconds: self.moving_seconds,
            idle_seconds: self.idle_seconds,
            gap_seconds: self.gap_seconds,
            max_speed_kmh: self.max_speed,
            ..Default::default()
        };

        if self.distance_km >= 0.1 {
            s.average_litres_per_100_km = self.fuel_litres / self.distance_km * 100.0;
        }
        let mut w_fuel = 0.0;
        let mut w_dist = 0.0;
        for w in &self.window {
            w_fuel += w.fuel;
            w_dist += w.distance;
        }
        let speed = self.fresh(PID_SPEED, self.last_tick);
        if speed.is_some_and(|sp| sp.value >= 5.0) && w_dist >= 0.005 {
            s.live_litres_per_100_km = w_fuel / w_dist * 100.0;
        }
        s.live_litres_per_hour = self.live_rate;
        if self.idle_seconds >= 5.0 {
            s.idle_litres_per_hour = self.idle_fuel / self.idle_seconds * 3600.0;
        }

        let co2 = Self::co2_kg_per_litre(self.config.fuel);
        s.co2_kg = self.fuel_litres * co2;
        s.co2_grams_per_km = s.average_litres_per_100_km * co2 * 10.0;
        s.cost = self.fuel_litres * self.config.fuel_price_per_litre;

        if self.driving_seconds > 0.0 {
            s.average_speed_kmh = self.distance_km / (self.driving_seconds / 3600.0);
            s.idle_share = self.idle_seconds / self.driving_seconds;
        }
        if self.moving_seconds > 0.0 {
            s.average_moving_speed_kmh = self.distance_km / (self.moving_seconds / 3600.0);
        }

        let mut best = 0.0;
        for (source, seconds) in &self.source_seconds {
            if *seconds > best {
                best = *seconds;
                s.fuel_source = *source;
            }
        }
        let diesel = self.config.fuel == FuelType::Diesel;
        match s.fuel_source {
            FuelSource::FuelRate => {
                s.fuel_note = format(
                    "Verbrauch aus dem Kraftstoffdurchsatz, den das Steuergerät \
                     selbst meldet. Das ist die genaueste Quelle.",
                    "Fuel use from the fuel rate the control unit reports itself. \
                     That is the most accurate source.",
                    &[],
                );
            }
            FuelSource::MassAirFlow => {
                s.fuel_is_rough = diesel;
                s.fuel_note = if diesel {
                    format(
                        "Verbrauch aus dem Luftmassenmesser geschätzt. Ein Diesel läuft mit \
                         viel Luftüberschuss, deshalb ist das nur eine grobe Schätzung, die \
                         deutlich zu hoch liegen kann.",
                        "Fuel use estimated from the mass air flow sensor. A diesel runs with \
                         a lot of excess air, so this is only a rough estimate that can be \
                         well too high.",
                        &[],
                    )
                } else {
                    format(
                        "Verbrauch aus dem Luftmassenmesser berechnet (Luft geteilt durch \
                         14,7). Meist auf etwa 5 bis 10 % genau; beim Ausrollen mit \
                         Schubabschaltung liegt der Wert etwas zu hoch.",
                        "Fuel use calculated from the mass air flow sensor (air divided by \
                         14.7). Usually accurate to about 5 to 10%; when coasting with fuel \
                         cut-off the value reads a little high.",
                        &[],
                    )
                };
            }
            FuelSource::SpeedDensity => {
                s.fuel_is_rough = true;
                s.fuel_note = format(
                    "Verbrauch aus Saugrohrdruck, Ansauglufttemperatur und Drehzahl \
                     geschätzt, mit angenommenem Füllungsgrad von {} %. Das ist eine \
                     grobe Schätzung; sie lässt sich mit einer vollen Tankfüllung \
                     nachjustieren.",
                    "Fuel use estimated from manifold pressure, intake temperature \
                     and engine speed with an assumed volumetric efficiency of {}%. \
                     That is a rough estimate; it can be calibrated against a full \
                     tank.",
                    &[number(self.config.volumetric_efficiency * 100.0, 0)],
                ) + if diesel {
                    format(
                        "Beim Diesel liegt sie zusätzlich deutlich zu hoch.",
                        "On a diesel it is also well too high.",
                        &[],
                    )
                } else {
                    Phrase::default()
                };
            }
            FuelSource::None => {
                s.fuel_note = format(
                    "Kein Verbrauch messbar: Es fehlen Kraftstoffdurchsatz (PID 5E), \
                     Luftmasse (PID 10) oder Saugrohrdruck mit Hubraum.",
                    "No fuel use available: fuel rate (PID 5E), mass air flow \
                     (PID 10) or manifold pressure with displacement are missing.",
                    &[],
                );
            }
        }

        // Eco score
        s.eco.harsh_accelerations = self.harsh_accel;
        s.eco.harsh_brakings = self.harsh_brake;
        if self.driving_seconds < 60.0 {
            s.eco.total = 0;
            s.eco.smoothness = 0;
            s.eco.idling = 0;
            s.eco.engine_speed = -1;
            s.eco.explanation = format(
                "Noch zu wenig Fahrt für eine Bewertung des Fahrstils.",
                "Not enough driving yet to rate the driving style.",
                &[],
            );
            return s;
        }
        // Smoothness: harsh events per ten minutes of moving, short trips count
        // as ten minutes so one event does not ruin them.
        let ten_minutes = (self.moving_seconds / 600.0).max(1.0);
        let event_rate = f64::from(self.harsh_accel + self.harsh_brake) / ten_minutes;
        let smoothness = (100.0 - 15.0 * event_rate).clamp(0.0, 100.0).round() as i32;
        let mut high_share = 0.0;
        let engine_speed = if self.rpm_known_driving_seconds >= 30.0 {
            high_share = self.high_rpm_seconds / self.rpm_known_driving_seconds;
            (100.0 - 200.0 * high_share).clamp(0.0, 100.0).round() as i32
        } else {
            -1
        };
        let idle_percent = s.idle_share * 100.0;
        let idling = (if idle_percent <= 10.0 {
            100.0
        } else {
            (100.0 - 2.0 * (idle_percent - 10.0)).clamp(0.0, 100.0)
        })
        .round() as i32;

        let mut weighted = 0.4 * f64::from(smoothness) + 0.3 * f64::from(idling);
        let mut weights = 0.7;
        if engine_speed >= 0 {
            weighted += 0.3 * f64::from(engine_speed);
            weights += 0.3;
        }
        let total = (weighted / weights).round() as i32;

        let mut text = format(
            "{} starke Beschleunigungen und {} harte Bremsungen, {} % der Zeit im \
             Stand mit laufendem Motor.",
            "{} hard accelerations and {} hard brakings, {}% of the time standing \
             with the engine running.",
            &[
                number(f64::from(self.harsh_accel), 0),
                number(f64::from(self.harsh_brake), 0),
                number(idle_percent, 0),
            ],
        );
        if engine_speed >= 0 {
            text = text
                + format(
                    "{} % der Fahrzeit über {}/min.",
                    "{}% of the driving time above {} rpm.",
                    &[
                        number(high_share * 100.0, 0),
                        number(
                            if self.config.fuel == FuelType::Diesel {
                                eco::HIGH_RPM_DIESEL
                            } else {
                                eco::HIGH_RPM_PETROL
                            },
                            0,
                        ),
                    ],
                );
        }
        // One tip for the weakest part, so the advice stays short.
        let mut weakest = smoothness;
        let mut tip = format(
            "Tipp: Vorausschauend fahren, früh vom Gas gehen und sanft beschleunigen \
             spart am meisten.",
            "Tip: looking ahead, lifting off early and accelerating gently saves \
             the most.",
            &[],
        );
        if engine_speed >= 0 && engine_speed < weakest {
            weakest = engine_speed;
            tip = format(
                "Tipp: Früher hochschalten. Die meisten Motoren arbeiten zwischen 1500 und \
                 2500/min am sparsamsten.",
                "Tip: shift up earlier. Most engines are most efficient between 1500 and \
                 2500 rpm.",
                &[],
            );
        }
        if idling < weakest {
            tip = format(
                "Tipp: Bei längeren Wartezeiten den Motor ausschalten. Im Leerlauf \
                 verbraucht das Auto Kraftstoff, ohne voranzukommen.",
                "Tip: switch the engine off when waiting longer. At idle the car burns fuel \
                 without getting anywhere.",
                &[],
            );
        }
        if total >= 85 {
            tip = format(
                "Sehr ruhiger, sparsamer Fahrstil.",
                "Very calm, economical driving style.",
                &[],
            );
        }
        s.eco.smoothness = smoothness;
        s.eco.engine_speed = engine_speed;
        s.eco.idling = idling;
        s.eco.total = total;
        s.eco.explanation = text + tip;
        s
    }
}
