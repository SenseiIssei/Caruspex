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
