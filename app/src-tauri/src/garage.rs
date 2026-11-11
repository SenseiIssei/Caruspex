// The garage file (Garage.cpp): every car seen with its checks, trips, fuel
// log and service plan. The interface keeps the logic; here the file is read
// and written in the format and at the place of the Qt version, so existing
// garages load: garage.json in QStandardPaths::AppDataLocation, which is
// <data dir>/Caruspex/Caruspex (organisation and application name).
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::settings::write_atomically;

fn minus_one() -> i32 {
    -1
}
fn minus_one_f() -> f64 {
    -1.0
}
fn yes() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GarageCheck {
    #[serde(default)]
    pub when: String,
    #[serde(default = "minus_one")]
    pub score: i32,
    #[serde(default)]
    pub mil: bool,
    #[serde(default)]
    pub open_monitors: i32,
    #[serde(default = "minus_one_f")]
    pub odometer_km: f64,
    #[serde(default)]
    pub codes: Vec<String>,
    #[serde(default)]
    pub texts: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GarageTrip {
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub end: String,
    #[serde(default)]
    pub distance_km: f64,
    #[serde(default)]
    pub fuel_litres: f64,
    #[serde(default, rename = "averageL100")]
    pub average_l100: f64,
    #[serde(default)]
    pub cost: f64,
    #[serde(default)]
    pub co2_kg: f64,
    #[serde(default)]
    pub max_speed_kmh: f64,
    #[serde(default = "minus_one")]
    pub eco_score: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GarageFuel {
    #[serde(default)]
    pub date: String,
    #[serde(default = "minus_one_f")]
    pub odometer_km: f64,
    #[serde(default)]
    pub litres: f64,
    #[serde(default)]
    pub price_per_litre: f64,
    #[serde(default = "yes")]
    pub full: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GarageService {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub interval_km: i32,
    #[serde(default)]
    pub interval_months: i32,
    #[serde(default)]
    pub last_date: String,
    #[serde(default = "minus_one_f")]
    pub last_km: f64,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GarageCar {
    #[serde(default)]
    pub vin: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub body_style: i32,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub first_seen: String,
    #[serde(default)]
    pub last_seen: String,
    #[serde(default)]
    pub checks: Vec<GarageCheck>,
    #[serde(default)]
    pub trips: Vec<GarageTrip>,
    #[serde(default)]
    pub fuel: Vec<GarageFuel>,
    #[serde(default)]
    pub services: Vec<GarageService>,
    /// A 3D model of this car the owner loaded for the X-ray view (path of
    /// a .glb or .gltf), and whether it had to be turned around.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub xray_model: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub xray_model_turned: bool,
}

#[derive(Serialize, Deserialize)]
struct GarageFile {
    #[serde(default)]
    version: i32,
    #[serde(default)]
    cars: Vec<serde_json::Value>,
}

/// Where the Qt version keeps the garage. CARUSPEX_GARAGE lets tests and
/// screenshots use their own.
pub fn garage_path(data_dir: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("CARUSPEX_GARAGE").filter(|p| !p.is_empty()) {
        return PathBuf::from(path);
    }
    data_dir
        .join("Caruspex")
        .join("Caruspex")
        .join("garage.json")
}

fn new_id() -> String {
    // A random enough id in the form of a UUID (QUuid::createUuid): time,
    // process and a counter mixed through a small hash.
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut x = nanos
        ^ (u64::from(std::process::id()) << 32)
        ^ COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let a = next();
    let b = next();
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (a >> 32) as u32,
        (a >> 16) as u16,
        a as u16 & 0x0FFF,
        ((b >> 48) as u16 & 0x3FFF) | 0x8000,
        b & 0xFFFF_FFFF_FFFF
    )
}

/// Parses the file; cars without a VIN and entries that are not objects are
/// dropped, like Garage::parse.
pub fn parse(data: &str) -> Vec<GarageCar> {
    let Ok(file) = serde_json::from_str::<GarageFile>(data) else {
        return Vec::new();
    };
    let mut cars = Vec::new();
    for value in file.cars {
        let Ok(mut car) = serde_json::from_value::<GarageCar>(value) else {
            continue;
        };
        if car.vin.is_empty() {
            continue;
        }
        for s in &mut car.services {
            if s.id.is_empty() {
                s.id = new_id();
            }
        }
        cars.push(car);
    }
    cars
}

pub fn serialise(cars: &[GarageCar]) -> String {
    let file = serde_json::json!({ "version": 2, "cars": cars });
    // QJsonDocument::Indented: four spaces.
    let mut out = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
    if file.serialize(&mut ser).is_err() {
        return String::new();
    }
    let mut text = String::from_utf8(out).unwrap_or_default();
    text.push('\n');
    text
}

pub fn load(path: &Path) -> Vec<GarageCar> {
    std::fs::read_to_string(path)
        .map(|t| parse(&t))
        .unwrap_or_default()
}

pub fn save(path: &Path, cars: &[GarageCar]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    write_atomically(path, serialise(cars).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    const QT_FILE: &str = r#"{
    "cars": [
        {
            "bodyStyle": 2,
            "checks": [
                { "codes": ["P0171"], "mil": true, "odometerKm": 123456, "openMonitors": 2,
                  "score": 71, "texts": ["System too lean"], "when": "2026-10-01T09:30:00" }
            ],
            "description": "VW Golf IV",
            "firstSeen": "2026-09-30T18:00:00",
            "fuel": [ { "date": "2026-10-02", "litres": 40.5, "odometerKm": 123500, "pricePerLitre": 1.79 } ],
            "lastSeen": "2026-10-01T09:30:00",
            "name": "Golf",
            "note": "",
            "services": [ { "intervalKm": 15000, "intervalMonths": 12, "lastDate": "", "title": "Oil" } ],
            "trips": [],
            "vin": "WVWZZZ1JZ3W000001"
        },
        { "name": "no vin" }
    ],
    "version": 2
}"#;

    #[test]
    fn reads_the_qt_file_with_its_defaults() {
        let cars = parse(QT_FILE);
        assert_eq!(cars.len(), 1);
        let car = &cars[0];
        assert_eq!(car.checks[0].score, 71);
        assert!(car.fuel[0].full, "full defaults to true");
        assert_eq!(car.services[0].last_km, -1.0);
        assert!(!car.services[0].id.is_empty());
        let again = parse(&serialise(&cars));
        assert_eq!(again, cars);
        assert!(serialise(&cars).contains("\n    \"cars\""));
        // The X-ray model is only written when there is one.
        assert!(!serialise(&cars).contains("xrayModel"));
        let mut with_model = cars.clone();
        with_model[0].xray_model = "C:/models/golf.glb".into();
        with_model[0].xray_model_turned = true;
        let text = serialise(&with_model);
        assert!(text.contains("\"xrayModel\": \"C:/models/golf.glb\""));
        assert_eq!(parse(&text), with_model);
    }

    #[test]
    fn ids_differ() {
        assert_ne!(new_id(), new_id());
        assert_eq!(new_id().len(), 36);
    }
}
