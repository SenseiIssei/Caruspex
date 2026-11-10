//! Readiness monitors (mode 01, PID 01) as a fixed list of identifiers.
//!
//! The control unit runs a set of self-tests ("monitors") while you drive.
//! PID 01 tells for each one whether the car has it at all and whether it has
//! finished since the fault memory was last cleared. The meaning of the bits
//! depends on the engine type: from byte B bit 3 on, petrol and diesel engines
//! use the same bits for different monitors.
//!
//! The evaluation needs to know which monitor is which, independent of the
//! language, so this module offers two ways in: decoding the raw bytes (exact,
//! covers diesel as well) and matching the names of the parsed status (a
//! fallback for code that only has the named readiness items).

use caruspex_i18n::Text;

pub use crate::inputs::{ReadinessItem, VehicleStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Monitor {
    // Continuous monitors, both engine types.
    Misfire,
    FuelSystem,
    Components,
    // Petrol engines (spark ignition).
    Catalyst,
    HeatedCatalyst,
    Evap,
    SecondaryAir,
    AcRefrigerant,
    OxygenSensor,
    OxygenSensorHeater,
    /// EGR and/or variable valve timing, both engine types.
    Egr,
    // Diesel engines (compression ignition).
    NmhcCatalyst,
    NoxScr,
    BoostPressure,
    ExhaustGasSensor,
    ParticulateFilter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorState {
    pub monitor: Monitor,
    /// The car has this monitor.
    pub supported: bool,
    /// Finished since the last clearing.
    pub complete: bool,
}

impl Default for MonitorState {
    fn default() -> Self {
        Self {
            monitor: Monitor::Misfire,
            supported: true,
            complete: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DecodedReadiness {
    pub mil_on: bool,
    pub stored_faults: i32,
    /// Diesel.
    pub compression_ignition: bool,
    /// Only the supported ones.
    pub monitors: Vec<MonitorState>,
}

/// Decodes the four data bytes A, B, C, D of PID 01 (without the 41 01 echo).
pub fn decode_readiness(a: i32, b: i32, c: i32, d: i32) -> DecodedReadiness {
    let mut out = DecodedReadiness {
        mil_on: (a & 0x80) != 0,
        stored_faults: a & 0x7F,
        compression_ignition: (b & 0x08) != 0,
        monitors: Vec::new(),
    };

    // Byte B: bits 0..2 say "supported", bits 4..6 say "not yet complete".
    const CONTINUOUS: [Monitor; 3] = [Monitor::Misfire, Monitor::FuelSystem, Monitor::Components];
    for (bit, monitor) in CONTINUOUS.iter().enumerate() {
        if b & (1 << bit) != 0 {
            out.monitors.push(MonitorState {
                monitor: *monitor,
                supported: true,
                complete: (b & (1 << (bit + 4))) == 0,
            });
        }
    }

    // Bytes C and D: C says "supported", D says "not yet complete". The bit
    // meaning follows the engine type (SAE J1979). Bits marked reserved for
    // diesel are skipped.
    const SPARK: [Monitor; 8] = [
        Monitor::Catalyst,
        Monitor::HeatedCatalyst,
        Monitor::Evap,
        Monitor::SecondaryAir,
        Monitor::AcRefrigerant,
        Monitor::OxygenSensor,
        Monitor::OxygenSensorHeater,
        Monitor::Egr,
    ];
    const DIESEL: [Option<Monitor>; 8] = [
        Some(Monitor::NmhcCatalyst),
        Some(Monitor::NoxScr),
        None,
        Some(Monitor::BoostPressure),
        None,
        Some(Monitor::ExhaustGasSensor),
        Some(Monitor::ParticulateFilter),
        Some(Monitor::Egr),
    ];
    for bit in 0..8 {
        if c & (1 << bit) == 0 {
            continue;
        }
        let m = if out.compression_ignition {
            match DIESEL[bit] {
                Some(m) => m,
                None => continue,
            }
        } else {
            SPARK[bit]
        };
        out.monitors.push(MonitorState {
            monitor: m,
            supported: true,
            complete: (d & (1 << bit)) == 0,
        });
    }
    out
}

fn match_name(raw_name: &str) -> Option<Monitor> {
    // Only ASCII letters change; the bytes of UTF-8 umlauts stay as they are,
    // so "Tankentlüftung" and "Tankentlueftung" share the prefix "tankentl"
    // either way.
    let n = raw_name.to_ascii_lowercase();
    let has = |needle: &str| n.contains(needle);
    // The more specific names come first ("heater" before "sensor",
    // "heated catalyst" before "catalyst").
    if has("aussetz") || has("misfire") {
        return Some(Monitor::Misfire);
    }
    if has("kraftstoffsystem") || has("fuel system") {
        return Some(Monitor::FuelSystem);
    }
    if has("bauteil") || has("component") {
        return Some(Monitor::Components);
    }
    if has("nmhc") {
        return Some(Monitor::NmhcCatalyst);
    }
    if has("nox") || has("scr") {
        return Some(Monitor::NoxScr);
    }
    if has("katalysatorheiz") || has("heated catalyst") {
        return Some(Monitor::HeatedCatalyst);
    }
    if has("katalysator") || has("catalyst") {
        return Some(Monitor::Catalyst);
    }
    if has("tankentl") || has("evap") {
        return Some(Monitor::Evap);
    }
    if has("sekund") || has("secondary air") {
        return Some(Monitor::SecondaryAir);
    }
    if has("klima") || has("a/c") || has("refrigerant") {
        return Some(Monitor::AcRefrigerant);
    }
    if has("lambdasondenheiz") || has("sensor heater") {
        return Some(Monitor::OxygenSensorHeater);
    }
    if has("lambdasonde") || has("oxygen sensor") {
        return Some(Monitor::OxygenSensor);
    }
    if has("abgasr") || has("egr") {
        return Some(Monitor::Egr);
    }
    if has("ladedruck") || has("boost") {
        return Some(Monitor::BoostPressure);
    }
    if has("abgassensor") || has("exhaust gas sensor") {
        return Some(Monitor::ExhaustGasSensor);
    }
    if has("partikel") || has("particulate") || has("pm filter") {
        return Some(Monitor::ParticulateFilter);
    }
    None
}

/// Maps the named readiness items of the vehicle status to monitors. Matches
/// German (with or without umlauts) and English names. Items that cannot be
/// matched are left out.
pub fn readiness_from_status(status: &VehicleStatus) -> Vec<MonitorState> {
    status
        .readiness
        .iter()
        .filter_map(|item| {
            match_name(&item.name).map(|m| MonitorState {
                monitor: m,
                supported: true,
                complete: item.ready,
            })
        })
        .collect()
}

/// Plain name of a monitor in both languages, e.g. ("Tankentlüftung",
/// "Evaporative system").
pub fn monitor_name(monitor: Monitor) -> Text {
    match monitor {
        Monitor::Misfire => Text::new("Verbrennungsaussetzer", "Misfire"),
        Monitor::FuelSystem => Text::new("Kraftstoffsystem", "Fuel system"),
        Monitor::Components => Text::new("Bauteilüberwachung", "Comprehensive components"),
        Monitor::Catalyst => Text::new("Katalysator", "Catalyst"),
        Monitor::HeatedCatalyst => Text::new("Katalysatorheizung", "Heated catalyst"),
        Monitor::Evap => Text::new("Tankentlüftung", "Evaporative system"),
        Monitor::SecondaryAir => Text::new("Sekundärluft", "Secondary air system"),
        Monitor::AcRefrigerant => Text::new("Klimaanlage", "A/C refrigerant"),
        Monitor::OxygenSensor => Text::new("Lambdasonde", "Oxygen sensor"),
        Monitor::OxygenSensorHeater => Text::new("Lambdasondenheizung", "Oxygen sensor heater"),
        Monitor::Egr => Text::new(
            "Abgasrückführung / Nockenwellenverstellung",
            "EGR / variable valve timing",
        ),
        Monitor::NmhcCatalyst => Text::new("Oxidationskatalysator (NMHC)", "NMHC catalyst"),
        Monitor::NoxScr => Text::new(
            "Stickoxid-Nachbehandlung (NOx/SCR)",
            "NOx aftertreatment (SCR)",
        ),
        Monitor::BoostPressure => Text::new("Ladedruck", "Boost pressure"),
        Monitor::ExhaustGasSensor => Text::new("Abgassensor", "Exhaust gas sensor"),
        Monitor::ParticulateFilter => Text::new("Partikelfilter", "Particulate filter"),
    }
}

/// True for the three monitors that run all the time and usually report
/// complete within seconds of starting the engine.
pub fn is_continuous_monitor(monitor: Monitor) -> bool {
    matches!(
        monitor,
        Monitor::Misfire | Monitor::FuelSystem | Monitor::Components
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(list: &[MonitorState], m: Monitor) -> Option<&MonitorState> {
        list.iter().find(|s| s.monitor == m)
    }

    #[test]
    fn pid01_decodes_petrol_monitors() {
        // Lamp on with 2 codes; misfire, fuel and components supported and
        // complete; catalyst, EVAP, O2 sensor and heater supported; EVAP open.
        let r = decode_readiness(0x82, 0x07, 0x65, 0x04);
        assert!(r.mil_on);
        assert_eq!(r.stored_faults, 2);
        assert!(!r.compression_ignition);
        assert_eq!(r.monitors.len(), 7);
        assert!(!find(&r.monitors, Monitor::Evap).unwrap().complete);
        assert!(find(&r.monitors, Monitor::Catalyst).unwrap().complete);
        assert!(find(&r.monitors, Monitor::OxygenSensorHeater).is_some());
        assert!(find(&r.monitors, Monitor::SecondaryAir).is_none());

        // Misfire not complete: bit 4 of byte B.
        let m = decode_readiness(0x00, 0x17, 0x00, 0x00);
        assert!(!find(&m.monitors, Monitor::Misfire).unwrap().complete);
    }

    #[test]
    fn pid01_decodes_diesel_monitors() {
        // Byte B bit 3 says compression ignition; C bits 0, 3, 5, 6, 7 are
        // NMHC, boost, exhaust sensor, particulate filter and EGR. Bit 2 is
        // reserved for diesel and must be skipped.
        let r = decode_readiness(0x00, 0x0F, 0xED, 0x40);
        assert!(r.compression_ignition);
        assert_eq!(r.monitors.len(), 8); // 3 continuous + 5 diesel
        assert!(find(&r.monitors, Monitor::NmhcCatalyst).unwrap().complete);
        assert!(find(&r.monitors, Monitor::BoostPressure).is_some());
        assert!(
            !find(&r.monitors, Monitor::ParticulateFilter)
                .unwrap()
                .complete
        );
        assert!(find(&r.monitors, Monitor::Egr).is_some());
        assert!(find(&r.monitors, Monitor::Evap).is_none());
    }

    #[test]
    fn readiness_names_are_matched_in_both_languages() {
        let items = [
            ("Verbrennungsaussetzer", true),
            ("Bauteilueberwachung", true),
            ("Tankentlüftung", false),
            ("Tankentlueftung", false),
            ("Katalysatorheizung", true),
            ("Katalysator", true),
            ("Lambdasondenheizung", true),
            ("Lambdasonde", false),
            ("Sekundärluft", true),
            ("Abgasrueckfuehrung", true),
            ("Evaporative system", false),
            ("Oxygen sensor heater", true),
            ("Something unknown", true),
        ];
        let status = VehicleStatus {
            valid: true,
            readiness: items
                .iter()
                .map(|(n, r)| ReadinessItem {
                    name: n.to_string(),
                    ready: *r,
                })
                .collect(),
            ..Default::default()
        };
        let m = readiness_from_status(&status);
        assert_eq!(m.len(), 12);
        assert_eq!(m[2].monitor, Monitor::Evap);
        assert_eq!(m[3].monitor, Monitor::Evap);
        assert_eq!(m[4].monitor, Monitor::HeatedCatalyst);
        assert_eq!(m[5].monitor, Monitor::Catalyst);
        assert_eq!(m[6].monitor, Monitor::OxygenSensorHeater);
        assert_eq!(m[7].monitor, Monitor::OxygenSensor);
        assert!(!m[7].complete);
        assert_eq!(m[8].monitor, Monitor::SecondaryAir);
        assert_eq!(m[9].monitor, Monitor::Egr);
        assert_eq!(m[10].monitor, Monitor::Evap);
        assert_eq!(m[11].monitor, Monitor::OxygenSensorHeater);
    }
}
