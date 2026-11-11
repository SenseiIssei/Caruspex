// What the backend hands to the interface, the structs of Session.h with
// camelCase names (app/src/api/types.ts mirrors them).
use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FaultRow {
    pub code: String,
    pub kind: String,
    pub text: String,
    pub throttle_related: bool,
    pub stored: bool,
    pub ecu: String,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LiveRow {
    pub pid: i32,
    pub name: String,
    pub unit: String,
    pub text: String,
    pub value: f64,
    pub warn: bool,
    pub time: f64,
    pub display_min: f64,
    pub display_max: f64,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FreezeFrameRow {
    pub frame: i32,
    pub dtc: String,
    pub ecu: String,
    pub values: Vec<LiveRow>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UdsDtcRow {
    pub code: String,
    pub failure_text: String,
    pub text: String,
    pub test_failed: bool,
    pub pending: bool,
    pub confirmed: bool,
    pub warning_lamp: bool,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UdsEcuRow {
    pub request: u32,
    pub response: u32,
    pub extended29: bool,
    pub kind: i32,
    pub name: String,
    pub present: bool,
    pub dtcs_read: bool,
    pub part_number: String,
    pub software_version: String,
    pub hardware_version: String,
    pub system_name: String,
    pub serial: String,
    pub dtcs: Vec<UdsDtcRow>,
    pub note: String,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomRow {
    pub key: i32,
    pub name: String,
    pub short_name: String,
    pub unit: String,
    pub pack: String,
    pub source: String,
    pub request: String,
    pub header: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub short_name_en: String,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HealthFindingRow {
    /// The stable rule id ("fuel_trim_bank1"), for "Was soll ich jetzt tun?".
    pub id: String,
    pub level: i32,
    pub area: String,
    pub title: String,
    pub explanation: String,
    pub advice: String,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorGuideRow {
    pub name: String,
    pub complete: bool,
    pub explanation: String,
    pub drive_pattern: String,
    pub typical_time: String,
    pub cold_start: bool,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdapterCheckRow {
    pub command: String,
    pub version: String,
    pub supported: bool,
    pub purpose: String,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdapterRow {
    pub reachable: bool,
    pub version: String,
    pub description: String,
    pub is_stn: bool,
    pub stn_id: String,
    pub voltage: f64,
    pub has_voltage: bool,
    pub round_trip_ms: f64,
    pub round_trip_with_hint_ms: f64,
    pub behaves_like: String,
    pub accepts_unknown: bool,
    pub verdict: String,
    pub facts: Vec<String>,
    pub speed_tips: Vec<String>,
    pub checks: Vec<AdapterCheckRow>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HealthRow {
    pub score: i32,
    pub grade: i32,
    pub summary: String,
    pub findings: Vec<HealthFindingRow>,
    pub not_checked: Vec<String>,
    pub monitors: Vec<MonitorGuideRow>,
    pub checks_run: i32,
    pub diesel: bool,
}

impl Default for HealthRow {
    fn default() -> Self {
        Self {
            score: -1,
            grade: 0,
            summary: String::new(),
            findings: Vec::new(),
            not_checked: Vec::new(),
            monitors: Vec::new(),
            checks_run: 0,
            diesel: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EcuScanRow {
    pub address: i32,
    pub name: String,
    pub present: bool,
    pub identification: String,
    pub faults: Vec<FaultRow>,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorRow {
    pub name: String,
    pub component: String,
    pub value: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub unit: String,
    pub has_minimum: bool,
    pub has_maximum: bool,
    pub passed: bool,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OxygenRow {
    pub name: String,
    pub sensor: i32,
    pub value: f64,
    pub unit: String,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusRow {
    pub valid: bool,
    pub mil: bool,
    pub fault_count: i32,
    pub monitor_names: Vec<String>,
    pub monitor_ready: Vec<bool>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BleDeviceRow {
    pub id: String,
    pub name: String,
    pub score: i32,
    pub kinds: i32,
}

/// One finding of the used car check.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsedCarItemRow {
    pub id: String,
    /// "identity", "mileage", "clearing", "readiness", "faults", "health", "battery".
    pub section: String,
    /// 0 good .. 3 critical.
    pub level: i32,
    pub title: String,
    pub explanation: String,
    /// What to ask the seller, empty if nothing.
    pub ask: String,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsedCarMileageRow {
    pub source: String,
    pub km: f64,
}

/// The result of the used car check. Numbers are -1 where the car said
/// nothing.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsedCarRow {
    /// 0 green, 1 yellow, 2 red.
    pub light: i32,
    pub headline: String,
    pub summary: String,
    pub vin: String,
    pub papers_vin: String,
    pub vehicle: String,
    pub model_year: i32,
    pub mileage: Vec<UsedCarMileageRow>,
    pub distance_since_clear_km: f64,
    pub minutes_since_clear: f64,
    pub warm_ups: f64,
    pub readiness_complete: i32,
    pub readiness_total: i32,
    pub stored_faults: i32,
    pub pending_faults: i32,
    pub permanent_faults: i32,
    pub health_score: i32,
    pub electric: bool,
    pub battery_pack: String,
    pub battery_soh: f64,
    pub battery_soc: f64,
    pub items: Vec<UsedCarItemRow>,
    pub questions: Vec<String>,
    pub not_checked: Vec<String>,
}

impl Default for UsedCarRow {
    fn default() -> Self {
        Self {
            light: 0,
            headline: String::new(),
            summary: String::new(),
            vin: String::new(),
            papers_vin: String::new(),
            vehicle: String::new(),
            model_year: 0,
            mileage: Vec::new(),
            distance_since_clear_km: -1.0,
            minutes_since_clear: -1.0,
            warm_ups: -1.0,
            readiness_complete: -1,
            readiness_total: -1,
            stored_faults: -1,
            pending_faults: -1,
            permanent_faults: -1,
            health_score: -1,
            electric: false,
            battery_pack: String::new(),
            battery_soh: -1.0,
            battery_soc: -1.0,
            items: Vec::new(),
            questions: Vec::new(),
            not_checked: Vec::new(),
        }
    }
}
