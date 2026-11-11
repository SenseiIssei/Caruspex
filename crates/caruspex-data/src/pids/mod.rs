//! Live values (OBD mode 01): definition and conversion.
//!
//! Names, units and status texts come out in the active language (see
//! `caruspex_i18n`). `pid_table()` and `find_pid()` pick the table for the
//! language that is active at the time of the call; the references stay valid
//! for the whole program run.
//!
//! Many newer PIDs carry more than one value. PID 0x67 for example reports two
//! coolant sensors, PID 0x7F three different run time counters. Each of those
//! values is described by a `PidValueSpec`, and each one gets its own integer
//! key that the UI can use as a series id:
//!
//! ```text
//! key = pid | (sub << 8)
//! ```
//!
//! `sub` 0 is the primary value of the PID, so for every single-value PID the
//! key is simply the PID number, exactly as before. `sub` 1, 2, ... are the
//! further values in the order the standard lists them. Example: PID 0x24
//! (wide-band O2 sensor) has key 0x024 for lambda and 0x124 for the voltage.

mod table;
mod text;

#[cfg(test)]
mod tests;

use std::sync::{Arc, OnceLock};

use caruspex_i18n::{Language, l, language};

/// Rough grouping for the live view, so a long list of values can be shown
/// in sensible sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PidCategory {
    /// Speed, load, torque, ignition, pedal.
    #[default]
    Engine,
    /// Mixture control, fuel pressure, fuel rate, tank.
    Fuel,
    /// Air mass, manifold pressure, throttle, turbocharger.
    Air,
    /// Oxygen sensors, exhaust temperatures and pressure.
    Exhaust,
    /// EGR, evaporative system, particulate filter, NOx, AdBlue.
    Emissions,
    /// Coolant, oil, intake and outside air.
    Temperatures,
    /// Supply voltage.
    Electrical,
    /// Road speed, gear, odometer, run time.
    Vehicle,
    /// Traction battery.
    HybridEv,
    /// Counters since clearing, OBD standard, scaling limits.
    Diagnostics,
    /// Manufacturer and user defined values (`caruspex-record`).
    Custom,
}

/// Display name of a category in the active language.
pub fn pid_category_name(category: PidCategory) -> &'static str {
    match category {
        PidCategory::Engine => l("Motor", "Engine"),
        PidCategory::Fuel => l("Kraftstoff", "Fuel"),
        PidCategory::Air => l("Luft und Aufladung", "Air and boost"),
        PidCategory::Exhaust => l("Abgas", "Exhaust"),
        PidCategory::Emissions => l("Abgasreinigung", "Emission control"),
        PidCategory::Temperatures => l("Temperaturen", "Temperatures"),
        PidCategory::Electrical => l("Elektrik", "Electrical"),
        PidCategory::Vehicle => l("Fahrzeug", "Vehicle"),
        PidCategory::HybridEv => l("Hybrid und Elektro", "Hybrid and EV"),
        PidCategory::Diagnostics => l("Diagnose", "Diagnostics"),
        PidCategory::Custom => l(
            "Hersteller und eigene Werte",
            "Manufacturer and custom values",
        ),
    }
}

/// All categories in display order.
pub fn pid_categories() -> &'static [PidCategory] {
    static ALL: [PidCategory; 11] = [
        PidCategory::Engine,
        PidCategory::Fuel,
        PidCategory::Air,
        PidCategory::Exhaust,
        PidCategory::Emissions,
        PidCategory::Temperatures,
        PidCategory::Electrical,
        PidCategory::Vehicle,
        PidCategory::HybridEv,
        PidCategory::Diagnostics,
        PidCategory::Custom,
    ];
    &ALL
}

/// The key scheme described at the top of this file.
pub const fn live_key(pid: i32, sub: i32) -> i32 {
    (pid & 0xFF) | (sub << 8)
}

pub const fn live_key_pid(key: i32) -> i32 {
    key & 0xFF
}

pub const fn live_key_sub(key: i32) -> i32 {
    key >> 8
}

/// Raw data bytes A, B, ... to a number.
pub type PidCalc = Arc<dyn Fn(&[i32]) -> f64 + Send + Sync>;
/// Number to text (status values).
pub type PidText = fn(f64) -> String;
/// Whether the control unit reports a value at all.
pub type PidAvailable = Arc<dyn Fn(&[i32]) -> bool + Send + Sync>;

/// One value inside a PID response.
#[derive(Clone)]
pub struct PidValueSpec {
    pub sub: i32,
    /// `live_key(pid, sub)`.
    pub key: i32,
    pub name: &'static str,
    pub unit: &'static str,
    /// Data bytes (after the PID byte) this value reads.
    pub need_bytes: usize,
    /// Raw data bytes A, B, ... to a number.
    pub calc: PidCalc,
    /// Optional: number to text (status values).
    pub text: Option<PidText>,
    /// Optional: whether the control unit reports this value at all. Most
    /// multi-value PIDs start with a byte A whose bits say which of the
    /// following values are present. `None` = always present.
    pub available: Option<PidAvailable>,
    /// Sensible gauge range.
    pub display_min: f64,
    pub display_max: f64,
    /// For the colored display, 0/0 = no limits.
    pub warn_low: f64,
    pub warn_high: f64,
}

impl PidValueSpec {
    pub fn is_available(&self, data: &[i32]) -> bool {
        if data.len() < self.need_bytes {
            return false;
        }
        self.available.as_ref().is_none_or(|f| f(data))
    }

    pub fn warns(&self, value: f64) -> bool {
        (self.warn_low != 0.0 || self.warn_high != 0.0)
            && (value < self.warn_low || value > self.warn_high)
    }

    /// The number the raw bytes stand for.
    pub fn calculate(&self, data: &[i32]) -> f64 {
        (self.calc)(data)
    }

    /// Status text for a value, if this value has one.
    pub fn text_of(&self, value: f64) -> Option<String> {
        self.text.map(|f| f(value))
    }
}

#[derive(Clone)]
pub struct PidSpec {
    pub pid: i32,
    // The fields below describe the primary value (sub 0) and are kept so
    // code that only knows single-value PIDs keeps working.
    pub name: &'static str,
    pub unit: &'static str,
    /// Data length according to the standard.
    pub bytes: i32,
    pub calc: PidCalc,
    pub text: Option<PidText>,
    pub warn_low: f64,
    pub warn_high: f64,

    // Since the extension to the complete standard:
    /// Name of the PID as a whole (equals `name` for single values).
    pub title: &'static str,
    pub category: PidCategory,
    /// Gauge range of the primary value.
    pub display_min: f64,
    pub display_max: f64,
    /// `values[0]` is the primary value, `key == pid`.
    pub values: Vec<PidValueSpec>,
}

impl PidSpec {
    /// The number the raw bytes stand for (primary value).
    pub fn calculate(&self, data: &[i32]) -> f64 {
        (self.calc)(data)
    }

    /// Status text for a primary value, if it has one.
    pub fn text_of(&self, value: f64) -> Option<String> {
        self.text.map(|f| f(value))
    }
}

/// One table per language, both built on first use. The active language
/// decides which one is handed out, so names and units follow it.
pub fn pid_table() -> &'static [PidSpec] {
    static GERMAN: OnceLock<Vec<PidSpec>> = OnceLock::new();
    static ENGLISH: OnceLock<Vec<PidSpec>> = OnceLock::new();
    if language() == Language::English {
        ENGLISH.get_or_init(|| table::build_table(true))
    } else {
        GERMAN.get_or_init(|| table::build_table(false))
    }
}

pub fn find_pid(pid: i32) -> Option<&'static PidSpec> {
    pid_table().iter().find(|s| s.pid == pid)
}

/// Looks up a single value by its key (see `live_key`).
pub fn find_pid_value(key: i32) -> Option<&'static PidValueSpec> {
    let spec = find_pid(live_key_pid(key))?;
    let sub = live_key_sub(key);
    if sub < 0 {
        return None;
    }
    spec.values.get(sub as usize)
}

/// Number of individual values over all PIDs.
pub fn pid_value_count() -> usize {
    pid_table().iter().map(|s| s.values.len()).sum()
}

/// One decoded value of a response.
#[derive(Clone, Copy)]
pub struct PidReading<'a> {
    pub spec: &'a PidValueSpec,
    pub value: f64,
}

/// Decodes all values the control unit reports in one response. `data` are the
/// bytes after the PID byte. Values whose support bit is clear or whose bytes
/// are missing are left out.
pub fn decode_pid<'a>(spec: &'a PidSpec, data: &[i32]) -> Vec<PidReading<'a>> {
    spec.values
        .iter()
        .filter(|v| v.is_available(data))
        .map(|v| PidReading {
            spec: v,
            value: v.calculate(data),
        })
        .collect()
}

/// What the live view shows first.
pub fn default_live_pids() -> &'static [i32] {
    static LIST: [i32; 12] = [
        0x0C, 0x0D, 0x05, 0x0F, 0x04, 0x11, 0x10, 0x0B, 0x06, 0x07, 0x42, 0x03,
    ];
    &LIST
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorBit {
    pub mask: i32,
    pub name: &'static str,
}

const fn mb(mask: i32, name: &'static str) -> MonitorBit {
    MonitorBit { mask, name }
}

pub fn monitors_b() -> &'static [MonitorBit] {
    static GERMAN: [MonitorBit; 3] = [
        mb(0x01, "Verbrennungsaussetzer"),
        mb(0x02, "Kraftstoffsystem"),
        mb(0x04, "Bauteilüberwachung"),
    ];
    static ENGLISH: [MonitorBit; 3] = [
        mb(0x01, "Misfire"),
        mb(0x02, "Fuel system"),
        mb(0x04, "Comprehensive components"),
    ];
    if language() == Language::English {
        &ENGLISH
    } else {
        &GERMAN
    }
}

/// Readiness monitors in byte C/D of PID 0x01 for spark ignition engines.
pub fn monitors_c() -> &'static [MonitorBit] {
    static GERMAN: [MonitorBit; 8] = [
        mb(0x01, "Katalysator"),
        mb(0x02, "Katalysatorheizung"),
        mb(0x04, "Tankentlüftung"),
        mb(0x08, "Sekundärluft"),
        mb(0x10, "Klimaanlage"),
        mb(0x20, "Lambdasonde"),
        mb(0x40, "Lambdasondenheizung"),
        mb(0x80, "Abgasrückführung"),
    ];
    static ENGLISH: [MonitorBit; 8] = [
        mb(0x01, "Catalyst"),
        mb(0x02, "Heated catalyst"),
        mb(0x04, "Evaporative system"),
        mb(0x08, "Secondary air system"),
        mb(0x10, "A/C refrigerant"),
        mb(0x20, "Oxygen sensor"),
        mb(0x40, "Oxygen sensor heater"),
        mb(0x80, "EGR system"),
    ];
    if language() == Language::English {
        &ENGLISH
    } else {
        &GERMAN
    }
}

/// The same bytes mean something else on compression ignition (diesel) engines.
pub fn monitors_c_diesel() -> &'static [MonitorBit] {
    static GERMAN: [MonitorBit; 6] = [
        mb(0x01, "Oxidationskatalysator (NMHC)"),
        mb(0x02, "Stickoxid-Nachbehandlung (SCR)"),
        mb(0x08, "Ladedruck"),
        mb(0x20, "Abgassensor"),
        mb(0x40, "Partikelfilter"),
        mb(0x80, "Abgasrückführung"),
    ];
    static ENGLISH: [MonitorBit; 6] = [
        mb(0x01, "NMHC catalyst"),
        mb(0x02, "NOx aftertreatment (SCR)"),
        mb(0x08, "Boost pressure"),
        mb(0x20, "Exhaust gas sensor"),
        mb(0x40, "Particulate filter"),
        mb(0x80, "EGR and VVT system"),
    ];
    if language() == Language::English {
        &ENGLISH
    } else {
        &GERMAN
    }
}
