//! Built-in packs of custom values: manufacturer values (service 22 and 21)
//! for particular cars, starting with electric cars, plus a pack for the
//! demo car.
//!
//! The packs are maintained as Torque CSV in data/pids/ and turned into Rust
//! by tools/gen_pids.py, like the vehicle catalog. Only values with a
//! reliable public source go in; the source of each value is in
//! [`CustomPid::source`].
//!
//! Names come out in the active language: [`built_in_packs`] hands out the
//! list for the language active at the time of the call, and the slices stay
//! valid for the whole run.
//!
//! Keys are [`PACK_KEY_BASE`] + position in [`built_in_packs`]. They stay the
//! same within one build but move when a pack gains or loses values, so
//! anything stored across runs should name a value by pack and request plus
//! name, not by key.

use std::sync::OnceLock;

use caruspex_data::pids::PidCategory;
use caruspex_i18n::{Language, Text, language};

use crate::custom_pids::{CustomPid, PACK_KEY_BASE};
use crate::generated::pid_packs::{PACKS, VALUES};

/// One pack of the generated table.
pub struct PackRow {
    pub id: &'static str,
    pub title: Text,
    pub category: PidCategory,
}

/// One value of the generated table.
pub struct ValueRow {
    /// Index into the pack table.
    pub pack: usize,
    pub name: Text,
    pub short_name: Text,
    /// Normalized hex, or "CAN132" for a broadcast value.
    pub request: &'static str,
    /// Empty = functional OBD addressing.
    pub header: &'static str,
    /// Empty = header + 8.
    pub response: &'static str,
    pub equation: &'static str,
    pub min: f64,
    pub max: f64,
    pub unit: &'static str,
    pub source: &'static str,
}

fn build(english: bool) -> Vec<CustomPid> {
    VALUES
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let pack = &PACKS[r.pack];
            let text = |t: &Text| (if english { t.en } else { t.de }).to_string();
            CustomPid {
                name: text(&r.name),
                short_name: text(&r.short_name),
                request: r.request.to_string(),
                header: r.header.to_string(),
                response: r.response.to_string(),
                equation: r.equation.to_string(),
                min_value: r.min,
                max_value: r.max,
                unit: r.unit.to_string(),
                pack: pack.id.to_string(),
                source: r.source.to_string(),
                key: PACK_KEY_BASE + i as i32,
                category: pack.category,
            }
        })
        .collect()
}

fn find_pack(name: &str) -> Option<&'static PackRow> {
    PACKS.iter().find(|p| p.id == name)
}

/// All values of all packs, packs in the order of [`pack_names`].
pub fn built_in_packs() -> &'static [CustomPid] {
    static GERMAN: OnceLock<Vec<CustomPid>> = OnceLock::new();
    static ENGLISH: OnceLock<Vec<CustomPid>> = OnceLock::new();
    if language() == Language::English {
        ENGLISH.get_or_init(|| build(true))
    } else {
        GERMAN.get_or_init(|| build(false))
    }
}

/// Ids of the packs ("caruspex-demo", "hyundai-kona-niro-ev", ...), the demo
/// pack first.
pub fn pack_names() -> Vec<&'static str> {
    PACKS.iter().map(|p| p.id).collect()
}

/// Display title of a pack in the active language, empty for an unknown id.
pub fn pack_title(name: &str) -> String {
    find_pack(name).map_or_else(String::new, |p| p.title.get().to_string())
}

/// Category the values of a pack are shown under (`HybridEv` for the electric
/// cars, `Custom` for the demo). `Custom` for an unknown id.
pub fn pack_category(name: &str) -> PidCategory {
    find_pack(name).map_or(PidCategory::Custom, |p| p.category)
}

/// The values of one pack, empty for an unknown id.
pub fn pack_pids(name: &str) -> Vec<CustomPid> {
    built_in_packs()
        .iter()
        .filter(|p| p.pack == name)
        .cloned()
        .collect()
}

/// One value of a pack with its English short name, which stays the same in
/// every language and so is what code finds a value by ("SOH", "Cell V max").
#[derive(Clone, Debug, PartialEq)]
pub struct NamedValue {
    pub short_en: &'static str,
    pub name_en: &'static str,
    pub pid: CustomPid,
}

/// The values of one pack with their English names, empty for an unknown id.
pub fn pack_values_en(name: &str) -> Vec<NamedValue> {
    let all = built_in_packs();
    VALUES
        .iter()
        .zip(all.iter())
        .filter(|(row, _)| PACKS[row.pack].id == name)
        .map(|(row, pid)| NamedValue {
            short_en: row.short_name.en,
            name_en: row.name.en,
            pid: pid.clone(),
        })
        .collect()
}

/// The odometer values of a pack: every value whose English name starts
/// with "Odometer". Only documented identifiers are in the packs, so this is
/// the list a mileage comparison may ask.
pub fn odometer_values(name: &str) -> Vec<CustomPid> {
    pack_values_en(name)
        .into_iter()
        .filter(|v| v.name_en.starts_with("Odometer"))
        .map(|v| v.pid)
        .collect()
}

/// The packs that fit a car, from the brand and model name of the catalog
/// ("Hyundai", "Kona"). The demo pack is never in it; the caller knows the
/// demo car by its VIN.
pub fn packs_for_model(brand: &str, model: &str) -> Vec<&'static str> {
    let brand = brand.to_lowercase();
    let model = model.to_lowercase();
    // Pack id, brands, words of the model name.
    const TABLE: [(&str, &[&str], &[&str]); 5] = [
        ("hyundai-ioniq-28", &["hyundai"], &["ioniq"]),
        (
            "hyundai-kona-niro-ev",
            &["hyundai", "kia"],
            &["kona", "niro"],
        ),
        ("nissan-leaf-ze0-aze0", &["nissan"], &["leaf"]),
        ("renault-zoe-ph1", &["renault"], &["zoe"]),
        ("tesla-model3-y", &["tesla"], &["model 3", "model y"]),
    ];
    TABLE
        .iter()
        .filter(|(_, brands, words)| {
            brands.iter().any(|b| brand.contains(b)) && words.iter().any(|w| model.contains(w))
        })
        .map(|(id, _, _)| *id)
        .filter(|id| find_pack(id).is_some())
        .collect()
}
