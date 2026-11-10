//! Decode trouble codes and translate them into plain language.
//!
//! The database knows the generic SAE J2012 / ISO 15031-6 codes, which mean
//! the same on every car, and the manufacturer-specific codes of the big
//! brand families. P1xxx codes in particular mean different things per
//! manufacturer, so a lookup takes the brand into account when it is known.
//! For the common faults there is plain-language help: may I keep driving,
//! the usual causes, what a non-mechanic can check and a rough cost class.
//!
//! All texts come out in the active language (see `caruspex_i18n`).

pub mod tables;

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use caruspex_i18n::{Language, Text, l, language};

use crate::severity::Severity;
use tables::{Detail, Family, NO_DETAIL, RangeRule, Record};

/// The area of the car a code belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum DtcSystem {
    #[default]
    Unknown,
    /// Mixture, fuel trim, fuel pressure and pump.
    FuelAir,
    /// Air flow, manifold pressure, intake temperature, intake flaps.
    AirIntake,
    /// Throttle body and accelerator pedal.
    Throttle,
    /// Misfire, coils, knock, crankshaft position.
    Ignition,
    /// Camshaft timing, cam sensors, cylinder deactivation.
    ValveTiming,
    /// Injectors and injection pump.
    Injection,
    /// Turbo and boost control.
    Turbo,
    /// Coolant, thermostat, fans.
    Cooling,
    /// Oil pressure and oil temperature.
    Lubrication,
    /// Oxygen sensors, catalytic converter, exhaust temperature.
    Exhaust,
    /// Exhaust gas recirculation and secondary air.
    Egr,
    /// Fuel vapour system (EVAP).
    Evap,
    /// Particulate filter, AdBlue/SCR, NOx, glow plugs.
    DieselAftertreatment,
    /// Battery and alternator.
    Charging,
    /// Control unit, idle, speed signal, cruise control.
    EngineControl,
    /// Gearbox and clutch.
    Transmission,
    /// Hybrid and electric drive.
    Hybrid,
    /// Data bus and lost communication.
    Network,
    /// Brakes, ABS, stability control.
    Brakes,
    Steering,
    Chassis,
    /// Airbags and seat belt pretensioners.
    Airbag,
    /// Body and comfort electronics.
    Body,
    Immobilizer,
    /// Air conditioning and heating.
    Climate,
}

/// Name of the area in the active language.
pub fn dtc_system_name(system: DtcSystem) -> &'static str {
    match system {
        DtcSystem::Unknown => l("Allgemein", "General"),
        DtcSystem::FuelAir => l("Gemischbildung und Kraftstoff", "Fuel and mixture"),
        DtcSystem::AirIntake => l("Luftansaugung", "Air intake"),
        DtcSystem::Throttle => l(
            "Drosselklappe und Fahrpedal",
            "Throttle and accelerator pedal",
        ),
        DtcSystem::Ignition => l("Zündung und Verbrennung", "Ignition and combustion"),
        DtcSystem::ValveTiming => l("Steuerzeiten und Nockenwelle", "Valve timing and camshaft"),
        DtcSystem::Injection => l("Einspritzung", "Fuel injection"),
        DtcSystem::Turbo => l("Turbolader und Ladedruck", "Turbocharger and boost"),
        DtcSystem::Cooling => l("Kühlung", "Cooling"),
        DtcSystem::Lubrication => l("Motoröl und Schmierung", "Engine oil and lubrication"),
        DtcSystem::Exhaust => l(
            "Abgas: Lambdasonden und Katalysator",
            "Exhaust: oxygen sensors and catalytic converter",
        ),
        DtcSystem::Egr => l("Abgasrückführung und Sekundärluft", "EGR and secondary air"),
        DtcSystem::Evap => l("Tankentlüftung", "Fuel vapour system (EVAP)"),
        DtcSystem::DieselAftertreatment => l(
            "Diesel: Partikelfilter, AdBlue, Glühanlage",
            "Diesel: particulate filter, AdBlue, glow plugs",
        ),
        DtcSystem::Charging => l("Batterie und Lichtmaschine", "Battery and charging"),
        DtcSystem::EngineControl => l("Motorsteuerung", "Engine control"),
        DtcSystem::Transmission => l("Getriebe und Kupplung", "Transmission and clutch"),
        DtcSystem::Hybrid => l("Hybrid- und Elektroantrieb", "Hybrid and electric drive"),
        DtcSystem::Network => l("Datenbus und Vernetzung", "Data bus and networking"),
        DtcSystem::Brakes => l("Bremsen, ABS und ESP", "Brakes, ABS and stability control"),
        DtcSystem::Steering => l("Lenkung", "Steering"),
        DtcSystem::Chassis => l("Fahrwerk", "Chassis"),
        DtcSystem::Airbag => l(
            "Airbag und Gurtstraffer",
            "Airbags and seat belt pretensioners",
        ),
        DtcSystem::Body => l("Karosserie und Komfort", "Body and comfort"),
        DtcSystem::Immobilizer => l("Wegfahrsperre", "Immobilizer"),
        DtcSystem::Climate => l("Klimaanlage und Heizung", "Air conditioning and heating"),
    }
}

/// Rough repair cost including parts and labour at a workshop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum CostClass {
    /// The likely causes are too far apart to say.
    #[default]
    Unknown,
    /// Below about 200 Euro.
    Cheap,
    /// About 200 to 800 Euro.
    Medium,
    /// Above about 800 Euro.
    Expensive,
}

/// "günstig" / "cheap" and so on, empty for Unknown.
pub fn cost_class_label(cost: CostClass) -> &'static str {
    match cost {
        CostClass::Cheap => l("günstig", "cheap"),
        CostClass::Medium => l("mittel", "moderate"),
        CostClass::Expensive => l("teuer", "expensive"),
        CostClass::Unknown => "",
    }
}

/// One sentence with the rough price band, empty for Unknown.
pub fn cost_class_hint(cost: CostClass) -> &'static str {
    match cost {
        CostClass::Cheap => l(
            "Reparatur in der Werkstatt grob unter 200 Euro",
            "Workshop repair roughly below 200 euros",
        ),
        CostClass::Medium => l(
            "Reparatur in der Werkstatt grob 200 bis 800 Euro",
            "Workshop repair roughly 200 to 800 euros",
        ),
        CostClass::Expensive => l(
            "Reparatur in der Werkstatt grob über 800 Euro",
            "Workshop repair roughly above 800 euros",
        ),
        CostClass::Unknown => "",
    }
}

/// General "may I keep driving?" advice for a severity, used when a code has
/// no specific advice of its own.
pub fn severity_advice(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => l(
            "Kein Fehler im eigentlichen Sinn, Weiterfahren ist in Ordnung.",
            "Not a fault as such, it is fine to keep driving.",
        ),
        Severity::Low => l(
            "Weiterfahren ist in Ordnung. Bei Gelegenheit prüfen lassen.",
            "It is fine to keep driving. Have it checked when convenient.",
        ),
        Severity::Medium => l(
            "Weiterfahren ist möglich, aber bald in die Werkstatt: Verbrauch, Abgaswerte \
             oder Komfort können leiden.",
            "You can keep driving, but go to a workshop soon: consumption, emissions or \
             comfort may suffer.",
        ),
        Severity::High => l(
            "Nur noch behutsam und auf kurzen Strecken fahren und in den nächsten Tagen \
             prüfen lassen, sonst können Folgeschäden entstehen.",
            "Drive gently and only short distances, and have it checked within the next \
             few days, otherwise further damage can follow.",
        ),
        Severity::Critical => l(
            "Nicht weiterfahren. Anhalten, sobald es sicher geht, Motor abstellen und \
             Hilfe holen.",
            "Do not drive on. Stop as soon as it is safe, switch off the engine and get \
             help.",
        ),
    }
}

/// Everything the database knows about one code, in the active language.
#[derive(Clone, Debug, PartialEq)]
pub struct DtcInfo {
    /// Normalised, for example "P0301".
    pub code: String,
    /// Description, or the group meaning when not known.
    pub text: String,
    pub severity: Severity,
    pub system: DtcSystem,
    /// `dtc_system_name(system)`.
    pub system_name: String,
    /// May I keep driving?
    pub advice: String,
    /// Most likely first; empty when not known.
    pub causes: Vec<String>,
    /// Things a non-mechanic can look at.
    pub checks: Vec<String>,
    pub cost_class: CostClass,
    /// The code lies in a manufacturer range.
    pub manufacturer_specific: bool,
    /// The exact code is in the database.
    pub known: bool,
    /// Causes and checks come from the database.
    pub has_details: bool,
    pub throttle_related: bool,
    /// Brand family that supplied the meaning, empty for SAE.
    pub family: String,
    /// "Ford", "Toyota/Lexus", ...
    pub family_label: String,
}

impl Default for DtcInfo {
    fn default() -> Self {
        Self {
            code: String::new(),
            text: String::new(),
            severity: Severity::Medium,
            system: DtcSystem::Unknown,
            system_name: String::new(),
            advice: String::new(),
            causes: Vec::new(),
            checks: Vec::new(),
            cost_class: CostClass::Unknown,
            manufacturer_specific: false,
            known: false,
            has_details: false,
            throttle_related: false,
            family: String::new(),
            family_label: String::new(),
        }
    }
}

/// One line of the offline lexicon.
#[derive(Clone, Debug, PartialEq)]
pub struct DtcLexiconEntry {
    pub code: String,
    /// Empty for the generic SAE codes.
    pub family: String,
    /// "SAE/ISO (alle Marken)" or the brand family.
    pub family_label: String,
    pub text: String,
    pub severity: Severity,
    pub system: DtcSystem,
}

const SYSTEM_LETTERS: [char; 4] = ['P', 'C', 'B', 'U'];

fn generic_for(letter: char) -> &'static [Record] {
    match letter {
        'P' => tables::generic_powertrain(),
        'C' => tables::generic_chassis(),
        'B' => tables::generic_body(),
        'U' => tables::generic_network(),
        _ => &[],
    }
}

/// The tables are sorted by code, so a binary search finds an entry.
fn find_record(table: &'static [Record], code: &str) -> Option<&'static Record> {
    table
        .binary_search_by(|r| r.code.cmp(code))
        .ok()
        .map(|i| &table[i])
}

fn normalize_code(raw: &str) -> String {
    raw.chars()
        .filter(|c| !matches!(c, ' ' | '\t'..='\r'))
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn is_valid_code(code: &str) -> bool {
    let b = code.as_bytes();
    if b.len() != 5 {
        return false;
    }
    if !matches!(b[0], b'P' | b'C' | b'B' | b'U') {
        return false;
    }
    if !(b'0'..=b'3').contains(&b[1]) {
        return false;
    }
    b[2..].iter().all(|c| c.is_ascii_hexdigit())
}

fn code_number(code: &str) -> i32 {
    i32::from_str_radix(&code[1..], 16).unwrap_or(0)
}

/// SAE leaves P1, P30 to P33 and the 1 and 2 blocks of C, B and U to the
/// manufacturers.
fn is_manufacturer_range(code: &str) -> bool {
    let b = code.as_bytes();
    let second = b[1];
    if b[0] == b'P' {
        if second == b'1' {
            return true;
        }
        if second == b'3' {
            return (b'0'..=b'3').contains(&b[2]);
        }
        return false;
    }
    second == b'1' || second == b'2'
}

fn find_rule(code: &str) -> Option<&'static RangeRule> {
    let number = code_number(code);
    let letter = code.as_bytes()[0] as char;
    tables::range_rules()
        .iter()
        .find(|r| r.letter == letter && number >= r.lo && number <= r.hi)
}

/// Folds the accented Latin letters that occur in brand names (Škoda,
/// Citroën) to plain ASCII. Two-byte UTF-8 sequences only; anything else
/// is dropped, like in `normalize_brand()` of the generator.
fn fold_latin(lead: u8, trail: u8) -> Option<char> {
    if lead == 0xC3 {
        if trail == 0x9F {
            return None; // sharp s has no plain form here
        }
        let x = trail | 0x20; // upper case to lower case
        return match x {
            0xA0..=0xA5 => Some('a'),
            0xA7 => Some('c'),
            0xA8..=0xAB => Some('e'),
            0xAC..=0xAF => Some('i'),
            0xB1 => Some('n'),
            0xB2..=0xB6 => Some('o'),
            0xB9..=0xBC => Some('u'),
            _ if x == 0xBD || trail == 0xBF => Some('y'),
            _ => None,
        };
    }
    if lead == 0xC4 && (trail == 0x8C || trail == 0x8D) {
        return Some('c');
    }
    if lead == 0xC5 {
        if trail == 0xA0 || trail == 0xA1 {
            return Some('s');
        }
        if trail == 0xBD || trail == 0xBE {
            return Some('z');
        }
    }
    None
}

/// "Mercedes-Benz" -> "mercedesbenz", "Škoda" -> "skoda".
fn normalize_brand(brand: &str) -> String {
    let mut out = String::new();
    let mut buf = [0u8; 4];
    for ch in brand.chars() {
        if ch.is_ascii() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
            }
            continue;
        }
        let bytes = ch.encode_utf8(&mut buf).as_bytes();
        if bytes.len() == 2 {
            out.extend(fold_latin(bytes[0], bytes[1]));
        }
    }
    out
}

fn family_by_key(key: &str) -> Option<&'static Family> {
    if key.is_empty() {
        return None;
    }
    tables::families().iter().find(|f| f.key == key)
}

/// Exact match on a family key or brand name first. Failing that, the
/// longest brand name the input starts with, so "VW Nutzfahrzeuge" and
/// "Mini Cooper" find their family too.
fn family_for_brand(brand: &str) -> Option<&'static Family> {
    let name = normalize_brand(brand);
    if name.is_empty() {
        return None;
    }
    let mut best: Option<&'static Family> = None;
    let mut best_length = 0;
    for f in tables::families() {
        if name == normalize_brand(f.key) {
            return Some(f);
        }
        for alias in f.brands.split('|') {
            if name == alias {
                return Some(f);
            }
            if alias.len() >= 3 && alias.len() > best_length && name.starts_with(alias) {
                best = Some(f);
                best_length = alias.len();
            }
        }
    }
    best
}

fn split_lines(text: &str) -> Vec<String> {
    text.split('\n')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// The lookup behind every public function. `family` may be `None`. With
/// `label_family` the fallback text of a manufacturer code names the family.
fn lookup(raw: &str, family: Option<&'static Family>, label_family: bool) -> DtcInfo {
    let mut info = DtcInfo {
        code: normalize_code(raw),
        ..DtcInfo::default()
    };
    info.system_name = dtc_system_name(DtcSystem::Unknown).to_string();
    if !is_valid_code(&info.code) {
        info.text = l("unbekannter Code", "unknown code").to_string();
        info.advice = l(
            "Das ist kein gültiger Fehlercode.",
            "This is not a valid trouble code.",
        )
        .to_string();
        return info;
    }
    info.manufacturer_specific = is_manufacturer_range(&info.code);

    let mut record = None;
    if let Some(family) = family {
        record = find_record(family.records, &info.code);
        if record.is_some() {
            info.family = family.key.to_string();
            info.family_label = family.label.get().to_string();
        }
    }
    let letter = info.code.as_bytes()[0] as char;
    if record.is_none() {
        record = find_record(generic_for(letter), &info.code);
    }
    let rule = find_rule(&info.code);

    let mut detail = NO_DETAIL;
    if let Some(record) = record {
        info.known = true;
        info.text = record.text.get().to_string();
        info.severity = record.severity;
        info.system = record.system;
        info.throttle_related = record.throttle;
        detail = record.detail;
    } else {
        let mut text = rule
            .map_or_else(|| l("Fehlercode", "Trouble code"), |r| r.text.get())
            .to_string();
        if info.manufacturer_specific {
            match family {
                Some(family) if label_family => {
                    text.push_str(l(" (herstellerspezifisch, ", " (manufacturer-specific, "));
                    text.push_str(family.label.get());
                    text.push(')');
                }
                _ => text.push_str(l(" (herstellerspezifisch)", " (manufacturer-specific)")),
            }
        }
        text.push_str(l(
            " - kein Klartext hinterlegt",
            " - no description available",
        ));
        info.text = text;
        if let Some(rule) = rule {
            info.severity = rule.severity;
            info.system = rule.system;
        }
    }
    // A code without help of its own borrows the help of its range, where
    // the range has one (airbags, lost communication and the like).
    if detail == NO_DETAIL {
        detail = rule.map_or(NO_DETAIL, |r| r.detail);
    }

    info.system_name = dtc_system_name(info.system).to_string();
    let table = tables::details();
    if let Some(d) = table.get(detail as usize) {
        let d: &Detail = d;
        info.has_details = true;
        info.advice = d.advice.get().to_string();
        info.causes = split_lines(d.causes.get());
        info.checks = split_lines(d.checks.get());
        info.cost_class = d.cost;
    } else {
        info.advice = severity_advice(info.severity).to_string();
    }
    info
}

fn vag_family() -> Option<&'static Family> {
    static VAG: OnceLock<Option<&'static Family>> = OnceLock::new();
    *VAG.get_or_init(|| family_by_key("VAG"))
}

/// The generic table plus the VAG table, which is what the database held
/// before brands were known. Kept for the existing callers.
fn for_each_legacy_record(mut f: impl FnMut(&'static Record)) {
    for letter in SYSTEM_LETTERS {
        generic_for(letter).iter().for_each(&mut f);
    }
    if let Some(vag) = vag_family() {
        vag.records.iter().for_each(&mut f);
    }
}

/// Full lookup. `brand` is a plain brand name like "Audi", "BMW",
/// "Mercedes-Benz" or "Opel"; a family key such as "VAG" or "PSA" works too.
/// With an empty or unknown brand only the generic SAE meanings are used.
/// The brand's own table is consulted first, then the generic one, then the
/// meaning of the code's number range.
pub fn lookup_dtc(code: &str, brand: &str) -> DtcInfo {
    lookup(code, family_for_brand(brand), true)
}

/// Brand family key for a brand name ("Skoda" -> "VAG", "Lexus" ->
/// "TOYOTA"), empty if the brand belongs to no known family. Case, spaces,
/// hyphens and accents do not matter.
///
/// Opel and Vauxhall map to "OPEL", which only knows the generic codes,
/// because the manufacturer codes of their older GM engines and their newer
/// PSA engines differ. Pass "PSA" for models on a PSA platform.
pub fn brand_family(brand: &str) -> String {
    family_for_brand(brand)
        .map(|f| f.key.to_string())
        .unwrap_or_default()
}

/// Display label of a family key, for example "Hyundai/Kia". Empty for an
/// unknown key; "SAE/ISO" style label for the empty key.
pub fn brand_family_label(family: &str) -> String {
    if family.is_empty() {
        return l("SAE/ISO (alle Marken)", "SAE/ISO (all brands)").to_string();
    }
    family_by_key(family)
        .map(|f| f.label.get().to_string())
        .unwrap_or_default()
}

/// All family keys in display order.
pub fn brand_families() -> &'static [String] {
    static KEYS: OnceLock<Vec<String>> = OnceLock::new();
    KEYS.get_or_init(|| {
        tables::families()
            .iter()
            .map(|f| f.key.to_string())
            .collect()
    })
}

/// Raw bytes -> code like "P0301". Empty string for 0x00 0x00 (no fault).
pub fn decode_dtc(byte_a: i32, byte_b: i32) -> String {
    if byte_a == 0 && byte_b == 0 {
        return String::new();
    }
    format!(
        "{}{:X}{:X}{:X}{:X}",
        SYSTEM_LETTERS[((byte_a >> 6) & 0x03) as usize],
        (byte_a >> 4) & 0x03,
        byte_a & 0x0F,
        (byte_b >> 4) & 0x0F,
        byte_b & 0x0F
    )
}

/// Plain-language text in the active language. Without a brand this knows
/// the generic and the VAG meanings, as it always did; unknown codes fall
/// back to the meaning of the code's group.
pub fn describe_dtc(code: &str) -> String {
    lookup(code, vag_family(), false).text
}

/// Same, with the meaning for the given brand (see `lookup_dtc`).
pub fn describe_dtc_for_brand(code: &str, brand: &str) -> String {
    lookup_dtc(code, brand).text
}

/// Does the code concern the throttle body?
pub fn is_throttle_dtc(code: &str) -> bool {
    dtc_throttle_codes().contains(code)
}

/// All generic and VAG codes with their description in both languages.
pub fn dtc_texts() -> &'static HashMap<&'static str, Text> {
    static TEXTS: OnceLock<HashMap<&'static str, Text>> = OnceLock::new();
    TEXTS.get_or_init(|| {
        let mut map = HashMap::new();
        for_each_legacy_record(|r| {
            map.entry(r.code).or_insert(r.text);
        });
        map
    })
}

pub fn dtc_throttle_codes() -> &'static HashSet<&'static str> {
    static CODES: OnceLock<HashSet<&'static str>> = OnceLock::new();
    CODES.get_or_init(|| {
        let mut set = HashSet::new();
        for_each_legacy_record(|r| {
            if r.throttle {
                set.insert(r.code);
            }
        });
        set
    })
}

fn build_descriptions(lang: Language) -> HashMap<&'static str, &'static str> {
    dtc_texts()
        .iter()
        .map(|(code, text)| {
            (
                *code,
                if lang == Language::English {
                    text.en
                } else {
                    text.de
                },
            )
        })
        .collect()
}

/// All generic and VAG codes with their description in the active language
/// (English, or German for every other language).
pub fn dtc_descriptions() -> &'static HashMap<&'static str, &'static str> {
    static ENGLISH: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    static GERMAN: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    if language() == Language::English {
        ENGLISH.get_or_init(|| build_descriptions(Language::English))
    } else {
        GERMAN.get_or_init(|| build_descriptions(Language::German))
    }
}

/// Every code in the database in the active language: first the generic
/// ones, then each brand family, each part sorted by code. The same code can
/// appear several times with different families.
pub fn dtc_lexicon() -> Vec<DtcLexiconEntry> {
    let mut out = Vec::new();
    let generic_label = brand_family_label("");
    let entry = |r: &Record, family: &str, label: &str| DtcLexiconEntry {
        code: r.code.to_string(),
        family: family.to_string(),
        family_label: label.to_string(),
        text: r.text.get().to_string(),
        severity: r.severity,
        system: r.system,
    };
    for letter in SYSTEM_LETTERS {
        for r in generic_for(letter) {
            out.push(entry(r, "", &generic_label));
        }
    }
    for f in tables::families() {
        let label = f.label.get();
        for r in f.records {
            out.push(entry(r, f.key, label));
        }
    }
    out
}

/// Number of codes per family key ("" for generic).
pub fn dtc_count(family: &str) -> usize {
    if family.is_empty() {
        return SYSTEM_LETTERS.iter().map(|&c| generic_for(c).len()).sum();
    }
    family_by_key(family).map_or(0, |f| f.records.len())
}

#[cfg(test)]
mod tests;
