//! Vehicle catalog: brands, models and model generations from about 1980 on,
//! with what each generation offers for diagnosis.
//!
//! The data lives in data/vehicles/*.csv and is turned into
//! `generated/catalog.rs` by tools/gen_vehicles.py. One row is one model
//! generation (Golf IV, BMW E46, Mercedes W203). It says which diagnostic
//! connector the car has, which bus and which ELM327 protocols to try, where
//! the socket sits, and a short note.
//!
//! Diagnosis is described honestly: cars before OBD-II have either no
//! readable diagnosis or a manufacturer plug that an ELM327 cannot talk to.
//! VAG cars from about 1990 speak KWP1281 over a KKL cable, which Caruspex
//! supports. EU petrol cars have EOBD from 2001, diesels from 2004, US cars
//! OBD-II from 1996. Pure electric cars often answer OBD only in part.
//!
//! All texts come out in the active language (see `caruspex_i18n`).

use crate::generated::catalog::{BRANDS, GENERATIONS};
use crate::vehicles::BusKind;
use crate::vin::VinInfo;
use caruspex_i18n::{Text, l};
use std::collections::BTreeSet;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyType {
    Hatchback,
    Sedan,
    Estate,
    Suv,
    Van,
    Coupe,
    Convertible,
    Pickup,
    Mpv,
    Microcar,
    Roadster,
}

impl BodyType {
    /// Every variant, in declaration order.
    pub const ALL: [BodyType; 11] = [
        BodyType::Hatchback,
        BodyType::Sedan,
        BodyType::Estate,
        BodyType::Suv,
        BodyType::Van,
        BodyType::Coupe,
        BodyType::Convertible,
        BodyType::Pickup,
        BodyType::Mpv,
        BodyType::Microcar,
        BodyType::Roadster,
    ];
}

// Bit flags, several per generation (the `powertrains` field).
pub const PT_PETROL: u32 = 1 << 0;
pub const PT_DIESEL: u32 = 1 << 1;
/// Full or mild hybrid without a charging plug.
pub const PT_HYBRID: u32 = 1 << 2;
/// Plug-in hybrid or range extender.
pub const PT_PLUG_IN: u32 = 1 << 3;
/// Battery electric only.
pub const PT_ELECTRIC: u32 = 1 << 4;
/// LPG or CNG from the factory.
pub const PT_GAS: u32 = 1 << 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Connector {
    /// No diagnosis an adapter can read (at most blink codes).
    None,
    /// 16-pin socket after SAE J1962 / ISO 15031-3.
    Obd2,
    /// VAG round plugs (2x2 pins), KWP1281 over a KKL cable.
    Vag2x2,
    /// BMW round 20-pin plug in the engine bay.
    Bmw20,
    /// Mercedes-Benz round 38-pin plug in the engine bay.
    Mb38,
    /// Other manufacturer plug (GM ALDL, Ford EEC-IV, Toyota DLC1 and so on).
    Other,
}

impl Connector {
    /// Every variant, in declaration order.
    pub const ALL: [Connector; 6] = [
        Connector::None,
        Connector::Obd2,
        Connector::Vag2x2,
        Connector::Bmw20,
        Connector::Mb38,
        Connector::Other,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SocketLocation {
    /// Not recorded for this model, the text names the usual spots.
    Unknown,
    /// Under the dashboard on the driver side, near the steering column.
    UnderDashLeft,
    /// Directly below the steering column.
    UnderSteeringColumn,
    /// Behind a cover in the driver's footwell.
    FootwellCover,
    /// In or beside the fuse box on the driver side.
    FuseBox,
    /// Centre console, often behind a cover in front of the gear lever.
    CentreConsole,
    /// Behind or under the ashtray.
    BehindAshtray,
    /// Under the dashboard on the passenger side.
    PassengerSide,
    /// In the engine bay.
    EngineBay,
    /// Rear end of the centre console or under the rear seat.
    RearConsole,
}

impl SocketLocation {
    /// Every variant, in declaration order.
    pub const ALL: [SocketLocation; 10] = [
        SocketLocation::Unknown,
        SocketLocation::UnderDashLeft,
        SocketLocation::UnderSteeringColumn,
        SocketLocation::FootwellCover,
        SocketLocation::FuseBox,
        SocketLocation::CentreConsole,
        SocketLocation::BehindAshtray,
        SocketLocation::PassengerSide,
        SocketLocation::EngineBay,
        SocketLocation::RearConsole,
    ];
}

/// What Caruspex can do with a car of this generation and year.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ObdSupport {
    /// Nothing to read.
    None,
    /// Only the maker's own tester.
    Manufacturer,
    /// VAG KWP1281 / KWP2000 over a KKL cable, no generic OBD.
    Kkl,
    /// OBD-II socket, but generic OBD is incomplete or not guaranteed.
    Partial,
    /// Generic OBD-II / EOBD.
    Full,
}

impl ObdSupport {
    /// Every variant, in declaration order.
    pub const ALL: [ObdSupport; 5] = [
        ObdSupport::None,
        ObdSupport::Manufacturer,
        ObdSupport::Kkl,
        ObdSupport::Partial,
        ObdSupport::Full,
    ];
}

#[derive(Clone, Debug)]
pub struct VehicleGeneration {
    pub brand: &'static str,
    pub model: &'static str,
    /// "IV", "E46", "W203", "8Z".
    pub generation: &'static str,
    pub year_from: i32,
    /// 0 = still built.
    pub year_to: i32,
    pub body: BodyType,
    /// `PT_*` flags.
    pub powertrains: u32,
    /// At the start of the generation.
    pub connector: Connector,
    /// From `connector_change` on.
    pub late_connector: Connector,
    /// 0 = the connector never changed.
    pub connector_change: i32,
    pub bus: BusKind,
    /// ELM327 protocol numbers, empty = no OBD-II protocol.
    pub protocol_order: &'static str,
    pub location: SocketLocation,
    /// May be empty.
    pub note: Text,
    /// VIN model codes, space separated ("1J", "203").
    pub vin_model_codes: &'static str,
    /// Extra search words, '|' separated ("Golf 4|Mk4").
    pub aliases: &'static str,
    /// Built for North America (OBD-II from 1996).
    pub us_market: bool,
}

#[derive(Clone, Debug)]
pub struct BrandInfo {
    pub name: &'static str,
    pub country: Text,
    /// "vag", "bmw", "mb", "ford", "gm", "chrysler" or empty.
    pub group: &'static str,
}

#[derive(Clone, Debug, Default)]
pub struct CatalogMatch {
    /// Best guess, may be `None`.
    pub generation: Option<&'static VehicleGeneration>,
    /// Known even without a generation.
    pub brand: String,
    /// The year the match settled on.
    pub year: i32,
    /// 0 to 100.
    pub confidence: i32,
    /// Other generations that fit.
    pub alternatives: Vec<&'static VehicleGeneration>,
    /// How the guess came about.
    pub reason: String,
}

/// The newest model year a "still built" generation can have.
const OPEN_END: i32 = 2027;

fn end_year(g: &VehicleGeneration) -> i32 {
    if g.year_to != 0 { g.year_to } else { OPEN_END }
}

fn covers(g: &VehicleGeneration, year: i32) -> bool {
    year >= g.year_from && year <= end_year(g)
}

/// Lower case without accents, so "Škoda", "skoda" and "SKODA" are equal and
/// "Citroen" finds "Citroën". Only the letters that occur in car names are
/// mapped; everything else passes through. Works on the UTF-8 bytes, like the
/// text is stored.
fn fold(text: &str) -> Vec<u8> {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c < 0x80 {
            out.push(c.to_ascii_lowercase());
            i += 1;
            continue;
        }
        if i + 1 >= b.len() {
            break;
        }
        let d = b[i + 1];
        let mapped: Option<&[u8]> = match (c, d) {
            (0xC3, 0x80 | 0x81 | 0x82 | 0x84 | 0x85 | 0xA0 | 0xA1 | 0xA2 | 0xA4 | 0xA5) => {
                Some(b"a")
            }
            (0xC3, 0x87 | 0xA7) => Some(b"c"),
            (0xC3, 0x88 | 0x89 | 0x8A | 0x8B | 0xA8 | 0xA9 | 0xAA | 0xAB) => Some(b"e"),
            (0xC3, 0x8D | 0xAD) => Some(b"i"),
            (0xC3, 0x91 | 0xB1) => Some(b"n"),
            (0xC3, 0x93 | 0x94 | 0x96 | 0x98 | 0xB3 | 0xB4 | 0xB6 | 0xB8) => Some(b"o"),
            (0xC3, 0x9A | 0x9C | 0xBA | 0xBC) => Some(b"u"),
            (0xC3, 0x9F) => Some(b"ss"),
            (0xC4, 0x8C | 0x8D) => Some(b"c"),
            (0xC5, 0xA0 | 0xA1) => Some(b"s"),
            (0xC5, 0xBD | 0xBE) => Some(b"z"),
            _ => None,
        };
        match mapped {
            Some(m) => {
                out.extend_from_slice(m);
                i += 1;
            }
            None => out.push(c),
        }
        i += 1;
    }
    out
}

fn same(a: &str, b: &str) -> bool {
    fold(a) == fold(b)
}

fn is_alnum(c: u8) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit()
}

/// Splits folded text into words at everything that is not a letter or digit.
fn words(folded: &[u8]) -> Vec<String> {
    folded
        .split(|&c| !is_alnum(c))
        .filter(|w| !w.is_empty())
        .map(|w| String::from_utf8_lossy(w).into_owned())
        .collect()
}

fn joined(folded: &[u8]) -> String {
    folded
        .iter()
        .copied()
        .filter(|&c| is_alnum(c))
        .map(char::from)
        .collect()
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
}

const ROMAN: [&str; 14] = [
    "i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x", "xi", "xii", "xiii", "xiv",
];

fn roman_value(s: &str) -> i32 {
    ROMAN
        .iter()
        .position(|r| *r == s)
        .map_or(0, |i| i as i32 + 1)
}

fn roman_text(n: i32) -> &'static str {
    if (1..=14).contains(&n) {
        ROMAN[(n - 1) as usize]
    } else {
        ""
    }
}

/// Ordinal of a generation name: "IV" -> 4, "Mk4" -> 4, otherwise 0.
fn ordinal(folded_generation: &str) -> i32 {
    let r = roman_value(folded_generation);
    if r != 0 {
        return r;
    }
    if folded_generation.len() > 2
        && folded_generation.starts_with("mk")
        && is_digits(&folded_generation[2..])
    {
        return folded_generation[2..].parse().unwrap_or(0);
    }
    0
}

/// Search words of one generation, grouped by how much a hit counts.
#[derive(Default)]
struct SearchKeys {
    brand: Vec<String>,
    /// Words and the joined name.
    model: Vec<String>,
    /// Only the words, to check coverage.
    model_words: Vec<String>,
    /// Name, ordinal, VIN codes.
    generation: Vec<String>,
    alias: Vec<String>,
}

fn add_unique(v: &mut Vec<String>, s: String) {
    if !s.is_empty() && !v.contains(&s) {
        v.push(s);
    }
}

fn build_keys(g: &VehicleGeneration) -> SearchKeys {
    let mut k = SearchKeys::default();
    let brand = fold(g.brand);
    for w in words(&brand) {
        add_unique(&mut k.brand, w);
    }
    add_unique(&mut k.brand, joined(&brand));

    let model = fold(g.model);
    k.model_words = words(&model);
    for w in k.model_words.clone() {
        add_unique(&mut k.model, w);
    }
    add_unique(&mut k.model, joined(&model));

    let gen_name = fold(g.generation);
    for w in words(&gen_name) {
        add_unique(&mut k.generation, w);
    }
    add_unique(&mut k.generation, joined(&gen_name));
    let n = ordinal(&joined(&gen_name));
    if n != 0 {
        add_unique(&mut k.generation, n.to_string());
        add_unique(&mut k.generation, roman_text(n).to_string());
        add_unique(&mut k.generation, format!("mk{n}"));
    }
    for w in words(&fold(g.vin_model_codes)) {
        add_unique(&mut k.generation, w);
    }

    let alias = fold(g.aliases);
    for phrase in alias.split(|&c| c == b'|') {
        for w in words(phrase) {
            add_unique(&mut k.alias, w);
        }
        add_unique(&mut k.alias, joined(phrase));
    }
    k
}

fn prefix_of(v: &[String], s: &str) -> bool {
    if s.len() < 3 {
        return false;
    }
    v.iter().any(|w| w.len() > s.len() && w.starts_with(s))
}

/// Points for one query word; 0 = no hit.
fn score_word(k: &SearchKeys, w: &str) -> i32 {
    let has = |v: &[String]| v.iter().any(|x| x == w);
    if has(&k.generation) {
        return 6;
    }
    if has(&k.model) {
        return 5;
    }
    if has(&k.alias) {
        return 4;
    }
    if has(&k.brand) {
        return 3;
    }
    if prefix_of(&k.model, w) || prefix_of(&k.brand, w) || prefix_of(&k.alias, w) {
        return 2;
    }
    0
}

/// "golf4" -> "golf", "4"; used when a word as a whole finds nothing.
fn split_letters_digits(w: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in w.chars() {
        let digit = c.is_ascii_digit();
        if let Some(last) = cur.chars().last()
            && digit != last.is_ascii_digit()
        {
            out.push(std::mem::take(&mut cur));
        }
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn all_keys() -> &'static [SearchKeys] {
    static KEYS: OnceLock<Vec<SearchKeys>> = OnceLock::new();
    KEYS.get_or_init(|| GENERATIONS.iter().map(build_keys).collect())
}

fn has_code(g: &VehicleGeneration, code: &str) -> bool {
    if code.is_empty() {
        return false;
    }
    format!(" {} ", g.vin_model_codes).contains(&format!(" {code} "))
}

fn group_of(g: &VehicleGeneration) -> &'static str {
    brand_info(g.brand).map_or("", |b| b.group)
}

fn only_electric(pt: u32) -> bool {
    pt != 0 && (pt & !PT_ELECTRIC) == 0
}

fn with_year(pattern: &str, value: i32) -> String {
    pattern.replacen("%d", &value.to_string(), 1)
}

/// The whole catalog, in the order of the CSV files.
pub fn all() -> &'static [VehicleGeneration] {
    GENERATIONS
}

pub fn size() -> usize {
    GENERATIONS.len()
}

/// Brands in alphabetical order.
pub fn brands() -> Vec<String> {
    let mut out: Vec<String> = BRANDS.iter().map(|b| b.name.to_string()).collect();
    out.sort_by_cached_key(|b| fold(b));
    out
}

pub fn brand_info(brand: &str) -> Option<&'static BrandInfo> {
    BRANDS.iter().find(|b| same(b.name, brand))
}

/// Models of a brand in alphabetical order.
pub fn models(brand: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    for g in GENERATIONS {
        if same(g.brand, brand) && seen.insert(fold(g.model)) {
            out.push(g.model.to_string());
        }
    }
    out.sort_by_cached_key(|m| fold(m));
    out
}

/// Generations of one model, oldest first.
pub fn generations(brand: &str, model: &str) -> Vec<&'static VehicleGeneration> {
    let mut out: Vec<&'static VehicleGeneration> = GENERATIONS
        .iter()
        .filter(|g| same(g.brand, brand) && same(g.model, model))
        .collect();
    out.sort_by_key(|g| g.year_from);
    out
}

/// Generation built in that year. Without a year (0) the newest one.
pub fn find_generation(brand: &str, model: &str, year: i32) -> Option<&'static VehicleGeneration> {
    let list = generations(brand, model);
    if list.is_empty() {
        return None;
    }
    if year <= 0 {
        return list.last().copied();
    }
    // Generations overlap in their change-over years; the newer one wins.
    let mut best = None;
    for g in &list {
        if covers(g, year) {
            best = Some(*g);
        }
    }
    if best.is_some() {
        return best;
    }
    // Outside every range: the closest one.
    let mut best_distance = 1 << 30;
    for g in &list {
        let d = if year < g.year_from {
            g.year_from - year
        } else {
            year - end_year(g)
        };
        if d < best_distance {
            best_distance = d;
            best = Some(*g);
        }
    }
    best
}

/// Tolerant search: "golf 4", "a2", "e46", "w203", "octavia 2008", "model 3".
/// Upper/lower case, umlauts and accents do not matter. A four digit number
/// between 1950 and 2049 counts as a year. Best hits first. The usual
/// `limit` is 20.
pub fn search(query: &str, limit: usize) -> Vec<&'static VehicleGeneration> {
    let tokens = words(&fold(query));
    let mut out = Vec::new();
    if tokens.is_empty() {
        return out;
    }

    let t = GENERATIONS;
    let keys = all_keys();

    let mut year_query = false;
    let mut query_year = 0;
    struct Hit {
        index: usize,
        score: i32,
    }
    let mut hits: Vec<Hit> = Vec::new();

    for (i, k) in keys.iter().enumerate() {
        let mut score = 0;
        let mut ok = true;
        let mut matched: Vec<String> = Vec::new();
        for w in &tokens {
            let s = score_word(k, w);
            if s > 0 {
                score += s;
                matched.push(w.clone());
                continue;
            }
            // A year: filters the generations instead of matching text.
            if w.len() == 4 && is_digits(w) {
                let year: i32 = w.parse().unwrap_or(0);
                if (1950..=2049).contains(&year) {
                    year_query = true;
                    query_year = year;
                    if covers(&t[i], year) {
                        score += 2;
                        continue;
                    }
                    ok = false;
                    break;
                }
            }
            // "golf4" or "e46i": try the parts on their own.
            let parts = split_letters_digits(w);
            if parts.len() > 1 {
                let mut part_score = 0;
                let mut all_parts = true;
                for p in &parts {
                    let ps = score_word(k, p);
                    if ps == 0 {
                        all_parts = false;
                        break;
                    }
                    part_score += ps;
                    matched.push(p.clone());
                }
                if all_parts {
                    score += part_score;
                    continue;
                }
            }
            ok = false;
            break;
        }
        if !ok {
            continue;
        }
        // Bonus when the query names the whole model, so "golf 4" prefers
        // the Golf over the Golf Cabriolet.
        let covered =
            !k.model_words.is_empty() && k.model_words.iter().all(|mw| matched.contains(mw));
        if covered {
            score += 3;
        }
        hits.push(Hit { index: i, score });
    }

    hits.sort_by(|a, b| {
        if a.score != b.score {
            return b.score.cmp(&a.score);
        }
        if year_query {
            // Among equal hits the generation that had just started that year
            // is more likely than one at the end of its life.
            let ca = covers(&t[a.index], query_year);
            let cb = covers(&t[b.index], query_year);
            if ca != cb {
                return cb.cmp(&ca);
            }
            return t[b.index].year_from.cmp(&t[a.index].year_from);
        }
        std::cmp::Ordering::Equal
    });
    for h in hits {
        if out.len() >= limit {
            break;
        }
        out.push(&t[h.index]);
    }
    out
}

/// Searches the catalog; the same as [`search`].
pub fn search_catalog(query: &str, limit: usize) -> Vec<&'static VehicleGeneration> {
    search(query, limit)
}

/// Best catalog guess for a decoded VIN.
pub fn match_vin(vin: &VinInfo) -> CatalogMatch {
    let mut m = CatalogMatch::default();
    let mut brand_list: Vec<String> = vin.brand_candidates.clone();
    if brand_list.is_empty() && !vin.brand.is_empty() {
        brand_list.push(vin.brand.clone());
    }
    m.year = vin.model_year;

    if brand_list.is_empty() {
        m.reason = l(
            "Der Hersteller ist unbekannt, daher gibt es keinen Treffer im Katalog.",
            "The manufacturer is unknown, so there is no match in the catalog.",
        )
        .to_string();
        return m;
    }
    if brand_list.len() == 1 {
        m.brand = brand_list[0].clone();
    }

    let t = GENERATIONS;
    let brand_fits = |g: &VehicleGeneration| brand_list.iter().any(|b| same(g.brand, b));

    // 1. The model code from the VIN, checked against the year candidates.
    if !vin.model_code.is_empty() {
        let by_code: Vec<&'static VehicleGeneration> = t
            .iter()
            .filter(|g| brand_fits(g) && has_code(g, &vin.model_code))
            .collect();
        if !by_code.is_empty() {
            // Try the most likely year first, then the alternatives.
            for (yi, &year) in vin.model_years.iter().enumerate() {
                let fit: Vec<&'static VehicleGeneration> = by_code
                    .iter()
                    .copied()
                    .filter(|g| covers(g, year))
                    .collect();
                if fit.is_empty() {
                    continue;
                }
                // Several models can share a code (Golf IV and Bora are both
                // 1J); they keep the catalog order. Within one model the
                // newer generation wins in the change-over years.
                let mut ordered: Vec<&'static VehicleGeneration> = Vec::new();
                for g in fit {
                    let same_model = ordered
                        .iter()
                        .position(|o| o.model == g.model && o.brand == g.brand);
                    match same_model {
                        None => ordered.push(g),
                        Some(p) if g.year_from > ordered[p].year_from => ordered.insert(p, g),
                        Some(p) => ordered.insert(p + 1, g),
                    }
                }
                let fit = ordered;
                m.generation = Some(fit[0]);
                m.alternatives = fit[1..].to_vec();
                m.year = year;
                m.brand = fit[0].brand.to_string();
                m.confidence = if yi == 0 && vin.model_year_certain {
                    95
                } else {
                    85
                };
                if fit.len() > 1 {
                    m.confidence -= 15;
                }
                m.reason = if yi == 0 {
                    l(
                        "Modellcode und Baujahr aus der Fahrgestellnummer passen zu dieser Generation.",
                        "Model code and model year from the VIN match this generation.",
                    )
                } else {
                    l(
                        "Der Modellcode passt; das Baujahr ergibt sich erst über die Bauzeit dieser Generation.",
                        "The model code matches; the model year follows from the build period of this generation.",
                    )
                }
                .to_string();
                return m;
            }
            // No usable year: the code alone.
            m.generation = Some(by_code[0]);
            m.alternatives = by_code[1..].to_vec();
            m.brand = by_code[0].brand.to_string();
            m.confidence = if by_code.len() == 1 { 75 } else { 55 };
            if vin.model_years.is_empty() {
                m.year = 0;
                m.reason = l(
                    "Der Modellcode passt, ein Baujahr steht nicht in der Fahrgestellnummer.",
                    "The model code matches, the VIN carries no model year.",
                )
                .to_string();
            } else {
                m.confidence -= 20;
                m.reason = l(
                    "Der Modellcode passt, aber das Baujahr liegt außerhalb der bekannten Bauzeit.",
                    "The model code matches, but the model year lies outside the known build period.",
                )
                .to_string();
            }
            return m;
        }
    }

    // 2. Brand and year only: everything of that brand built in that year.
    for &year in &vin.model_years {
        for g in t {
            if brand_fits(g) && covers(g, year) {
                m.alternatives.push(g);
            }
        }
        if !m.alternatives.is_empty() {
            m.year = year;
            break;
        }
    }
    if m.alternatives.len() == 1 {
        let g = m.alternatives.remove(0);
        m.generation = Some(g);
        m.brand = g.brand.to_string();
        m.confidence = if vin.model_year_certain { 70 } else { 55 };
        m.reason = l(
            "Von diesem Hersteller gab es in diesem Baujahr nur dieses Modell.",
            "This manufacturer only built this model in that year.",
        )
        .to_string();
        return m;
    }
    m.confidence = if m.brand.is_empty() { 15 } else { 30 };
    m.reason = l(
        "Der Hersteller ist klar, das Modell steht nicht in der Fahrgestellnummer. \
         Bitte das Modell aus der Liste wählen.",
        "The manufacturer is clear, the model is not in the VIN. \
         Please pick the model from the list.",
    )
    .to_string();
    m
}

/// ELM327 protocol order for this generation; empty if it has no OBD-II.
pub fn protocol_order_for(g: &VehicleGeneration) -> String {
    g.protocol_order.to_string()
}

/// Connector of a car of this generation in a given year (0 = first year).
pub fn connector_for(g: &VehicleGeneration, year: i32) -> Connector {
    let year = if year <= 0 { g.year_from } else { year };
    if g.connector_change != 0 && year >= g.connector_change {
        g.late_connector
    } else {
        g.connector
    }
}

/// True when KWP1281 / KWP2000 over a KKL cable is worth a try.
pub fn kkl_possible(g: &VehicleGeneration, year: i32) -> bool {
    let year = if year <= 0 { g.year_from } else { year };
    if group_of(g) != "vag" || year < 1990 {
        return false;
    }
    let c = connector_for(g, year);
    if c == Connector::Vag2x2 {
        return true;
    }
    c == Connector::Obd2 && matches!(g.bus, BusKind::KLine | BusKind::Mixed) && year <= 2008
}

/// What diagnosis is possible for this generation in a given year
/// (0 = first year) and with which engine (0 = as listed).
pub fn support_for(g: &VehicleGeneration, year: i32, powertrain: u32) -> ObdSupport {
    let year = if year <= 0 { g.year_from } else { year };
    let pt = if powertrain != 0 {
        powertrain
    } else {
        g.powertrains
    };
    match connector_for(g, year) {
        Connector::None => return ObdSupport::None,
        Connector::Vag2x2 => return ObdSupport::Kkl,
        Connector::Bmw20 | Connector::Mb38 | Connector::Other => return ObdSupport::Manufacturer,
        Connector::Obd2 => {}
    }
    if only_electric(pt) {
        return ObdSupport::Partial;
    }
    if g.us_market {
        return if year >= 1996 {
            ObdSupport::Full
        } else {
            ObdSupport::Partial
        };
    }
    // EU: petrol cars (and hybrids, gas cars) from 2001, diesels from 2004.
    let petrol_like = (pt & (PT_PETROL | PT_HYBRID | PT_PLUG_IN | PT_GAS)) != 0;
    let threshold = if petrol_like { 2001 } else { 2004 };
    if year >= threshold {
        ObdSupport::Full
    } else {
        ObdSupport::Partial
    }
}

/// "since 2017", "1997 to 2006" or a single year, in the active language.
pub fn years_text(g: &VehicleGeneration) -> String {
    if g.year_to == 0 {
        return format!("{}{}", l("seit ", "since "), g.year_from);
    }
    if g.year_to == g.year_from {
        return g.year_from.to_string();
    }
    format!("{}{}{}", g.year_from, l(" bis ", " to "), g.year_to)
}

/// Display name: "Volkswagen Golf IV (1997 bis 2006)" / "... (1997 to 2006)".
pub fn display_name(g: &VehicleGeneration) -> String {
    let mut out = format!("{} {}", g.brand, g.model);
    if !g.generation.is_empty() && fold(g.generation) != fold(g.model) {
        out.push(' ');
        out.push_str(g.generation);
    }
    format!("{out} ({})", years_text(g))
}

/// Plain sentence about the diagnosis of this car, plus the note of the row.
pub fn diagnosis_text(g: &VehicleGeneration, year: i32) -> String {
    let year = if year <= 0 { g.year_from } else { year };
    let c = connector_for(g, year);
    let kkl = kkl_possible(g, year);
    let mut out: String;
    match support_for(g, year, 0) {
        ObdSupport::None => {
            out = l(
                "Dieses Auto hat noch keine Diagnose, die ein Adapter auslesen kann. Fehler zeigen sich \
                 höchstens als Blinkcode.",
                "This car has no diagnosis yet that an adapter could read. Faults show up as blink \
                 codes at most.",
            )
            .to_string();
        }
        ObdSupport::Kkl => {
            out = l(
                "VAG-Rundstecker (2x2 Pole). Mit einem KKL-Kabel liest Caruspex Fehlerspeicher und \
                 Messwerte über KWP1281.",
                "VAG round plugs (2x2 pins). With a KKL cable Caruspex reads fault memory and live \
                 values over KWP1281.",
            )
            .to_string();
        }
        ObdSupport::Manufacturer => {
            out = if c == Connector::Bmw20 {
                l(
                    "BMW-Rundstecker mit 20 Polen im Motorraum. Ein ELM327 versteht ihn nicht, dafür \
                     braucht es ein BMW-Diagnosegerät.",
                    "BMW round 20-pin plug in the engine bay. An ELM327 does not understand it, that \
                     takes a BMW diagnostic tool.",
                )
            } else if c == Connector::Mb38 {
                l(
                    "Mercedes-Rundstecker mit 38 Polen im Motorraum. Ein ELM327 versteht ihn nicht, \
                     dafür braucht es ein Mercedes-Diagnosegerät.",
                    "Mercedes round 38-pin plug in the engine bay. An ELM327 does not understand it, \
                     that takes a Mercedes diagnostic tool.",
                )
            } else if year >= 2008 {
                l(
                    "Herstellereigener Diagnoseanschluss statt der genormten OBD-II-Buchse. Ein \
                     ELM327 kann damit nicht ohne Weiteres sprechen.",
                    "Manufacturer diagnostic connector instead of the standard OBD-II socket. An \
                     ELM327 cannot simply talk to it.",
                )
            } else {
                l(
                    "Herstellereigener Diagnosestecker, falls vorhanden. Ein ELM327 kann damit nicht \
                     sprechen, oft lassen sich aber Blinkcodes auslesen.",
                    "Manufacturer diagnostic plug, if fitted. An ELM327 cannot talk to it, but blink \
                     codes can often be read.",
                )
            }
            .to_string();
        }
        ObdSupport::Partial => {
            out = if only_electric(g.powertrains) {
                l(
                    "Reines Elektroauto: Abgaswerte gibt es nicht, daher antworten oft nur wenige \
                     OBD-Messwerte und kaum Fehlercodes.",
                    "Pure electric car: there are no emissions to report, so often only a few OBD \
                     values and hardly any trouble codes answer.",
                )
            } else {
                l(
                    "16-polige OBD-Buchse vorhanden, aber OBD war für dieses Baujahr noch keine \
                     Pflicht (EU: Benziner ab 2001, Diesel ab 2004; USA ab 1996). Viele \
                     Motorsteuergeräte antworten trotzdem, ein Versuch lohnt sich.",
                    "16-pin OBD socket present, but OBD was not yet mandatory for this model year \
                     (EU: petrol from 2001, diesel from 2004; USA from 1996). Many engine control \
                     units answer anyway, it is worth a try.",
                )
            }
            .to_string();
            if kkl {
                out.push_str(l(
                    " Sicher klappt es mit einem KKL-Kabel über KWP1281.",
                    " A KKL cable with KWP1281 works for sure.",
                ));
            }
        }
        ObdSupport::Full => {
            out = l(
                "Volle OBD-II-Diagnose über die 16-polige Buchse: Fehlercodes, Messwerte und \
                 Bereitschaftstests.",
                "Full OBD-II diagnosis through the 16-pin socket: trouble codes, live values and \
                 readiness tests.",
            )
            .to_string();
            if !g.us_market && (g.powertrains & PT_DIESEL) != 0 && year < 2004 {
                out.push_str(l(
                    " Bei Dieselmotoren dieses Baujahrs war OBD noch keine Pflicht.",
                    " For diesel engines of this model year OBD was not yet mandatory.",
                ));
            }
            if kkl {
                out.push_str(l(
                    " Die übrigen Steuergeräte erreicht Caruspex mit einem KKL-Kabel.",
                    " Caruspex reaches the other control units with a KKL cable.",
                ));
            }
        }
    }
    if g.connector_change != 0 && year < g.connector_change && g.late_connector != g.connector {
        out.push(' ');
        out.push_str(&with_year(
            l("Ab etwa %d: ", "From about %d: "),
            g.connector_change,
        ));
        out.push_str(connector_name(g.late_connector));
        out.push('.');
    }
    let note = g.note.get();
    if !note.is_empty() {
        out.push(' ');
        out.push_str(note);
    }
    out
}

// Texts for the enums, in the active language.

pub fn body_name(body: BodyType) -> &'static str {
    match body {
        BodyType::Hatchback => l("Schrägheck", "Hatchback"),
        BodyType::Sedan => l("Limousine", "Sedan"),
        BodyType::Estate => l("Kombi", "Estate"),
        BodyType::Suv => l("SUV", "SUV"),
        BodyType::Van => l("Transporter", "Van"),
        BodyType::Coupe => l("Coupé", "Coupe"),
        BodyType::Convertible => l("Cabrio", "Convertible"),
        BodyType::Pickup => l("Pick-up", "Pickup"),
        BodyType::Mpv => l("Van (MPV)", "MPV"),
        BodyType::Microcar => l("Kleinstwagen", "Microcar"),
        BodyType::Roadster => l("Roadster", "Roadster"),
    }
}

pub fn connector_name(connector: Connector) -> &'static str {
    match connector {
        Connector::None => l("kein Diagnosestecker", "no diagnostic connector"),
        Connector::Obd2 => l("16-polige OBD-II-Buchse", "16-pin OBD-II socket"),
        Connector::Vag2x2 => l(
            "VAG-Rundstecker (2x2), KKL-Kabel",
            "VAG round plugs (2x2), KKL cable",
        ),
        Connector::Bmw20 => l("BMW-Rundstecker, 20 Pole", "BMW round plug, 20 pins"),
        Connector::Mb38 => l(
            "Mercedes-Rundstecker, 38 Pole",
            "Mercedes round plug, 38 pins",
        ),
        Connector::Other => l("herstellereigener Stecker", "manufacturer plug"),
    }
}

pub fn location_text(location: SocketLocation) -> &'static str {
    match location {
        SocketLocation::Unknown => l(
            "Meist unter dem Armaturenbrett auf der Fahrerseite, manchmal hinter einer Abdeckung \
             oder in der Mittelkonsole.",
            "Usually under the dashboard on the driver side, sometimes behind a cover or in the \
             centre console.",
        ),
        SocketLocation::UnderDashLeft => l(
            "Unter dem Armaturenbrett auf der Fahrerseite, nahe der Lenksäule.",
            "Under the dashboard on the driver side, near the steering column.",
        ),
        SocketLocation::UnderSteeringColumn => l(
            "Direkt unter der Lenksäule.",
            "Directly below the steering column.",
        ),
        SocketLocation::FootwellCover => l(
            "Im Fahrerfußraum hinter einer Abdeckung.",
            "In the driver's footwell behind a cover.",
        ),
        SocketLocation::FuseBox => l(
            "Am oder im Sicherungskasten auf der Fahrerseite, hinter dessen Deckel.",
            "At or in the fuse box on the driver side, behind its lid.",
        ),
        SocketLocation::CentreConsole => l(
            "In der Mittelkonsole hinter einer Abdeckung vor dem Schalthebel.",
            "In the centre console behind a cover in front of the gear lever.",
        ),
        SocketLocation::BehindAshtray => l(
            "Hinter oder unter dem Aschenbecher in der Mittelkonsole.",
            "Behind or below the ashtray in the centre console.",
        ),
        SocketLocation::PassengerSide => l(
            "Unter dem Armaturenbrett auf der Beifahrerseite.",
            "Under the dashboard on the passenger side.",
        ),
        SocketLocation::EngineBay => l(
            "Im Motorraum, meist unter einer runden Kappe.",
            "In the engine bay, usually under a round cap.",
        ),
        SocketLocation::RearConsole => l(
            "Hinten an der Mittelkonsole beziehungsweise unter den Rücksitzen.",
            "At the rear of the centre console or under the rear seats.",
        ),
    }
}

/// The spot for the sketch of the first-start assistant: the recorded one,
/// or the engine bay for the round plugs of BMW and Mercedes, otherwise
/// `Unknown` (the sketch then marks the usual spots).
pub fn location_for(g: &VehicleGeneration, year: i32) -> SocketLocation {
    if g.location != SocketLocation::Unknown {
        return g.location;
    }
    match connector_for(g, year) {
        Connector::Bmw20 | Connector::Mb38 => SocketLocation::EngineBay,
        _ => SocketLocation::Unknown,
    }
}

/// Location with the connector taken into account (an unknown spot of a
/// BMW round plug is still the engine bay).
pub fn location_text_for(g: &VehicleGeneration, year: i32) -> &'static str {
    if g.location != SocketLocation::Unknown {
        return location_text(g.location);
    }
    match connector_for(g, year) {
        Connector::None => return l("Kein Diagnosestecker.", "No diagnostic connector."),
        Connector::Bmw20 | Connector::Mb38 => return location_text(SocketLocation::EngineBay),
        Connector::Vag2x2 => {
            return l(
                "Je nach Modell im Sicherungskasten unter dem Armaturenbrett, unter der Schaltkulisse \
                 oder im Motorraum.",
                "Depending on the model in the fuse box under the dashboard, under the gear lever \
                 gaiter or in the engine bay.",
            );
        }
        Connector::Other => {
            return l(
                "Herstellerstecker, meist im Motorraum oder unter dem Armaturenbrett.",
                "Manufacturer plug, usually in the engine bay or under the dashboard.",
            );
        }
        Connector::Obd2 => {}
    }
    location_text(SocketLocation::Unknown)
}

/// The engines of a `PT_*` flag set, comma separated.
pub fn powertrain_text(powertrains: u32) -> String {
    const NAMES: [(u32, Text); 6] = [
        (PT_PETROL, Text::new("Benzin", "Petrol")),
        (PT_DIESEL, Text::new("Diesel", "Diesel")),
        (PT_HYBRID, Text::new("Hybrid", "Hybrid")),
        (PT_PLUG_IN, Text::new("Plug-in-Hybrid", "Plug-in hybrid")),
        (PT_ELECTRIC, Text::new("Elektro", "Electric")),
        (PT_GAS, Text::new("Gas (LPG/CNG)", "Gas (LPG/CNG)")),
    ];
    let mut out = String::new();
    for (flag, text) in NAMES {
        if powertrains & flag == 0 {
            continue;
        }
        if !out.is_empty() {
            out.push_str(", ");
        }
        out.push_str(text.get());
    }
    out
}

pub fn support_name(support: ObdSupport) -> &'static str {
    match support {
        ObdSupport::None => l("keine Diagnose", "no diagnosis"),
        ObdSupport::Manufacturer => l("nur Herstellerdiagnose", "manufacturer diagnosis only"),
        ObdSupport::Kkl => l("KKL-Kabel (KWP1281)", "KKL cable (KWP1281)"),
        ObdSupport::Partial => l("OBD-II teilweise", "OBD-II in part"),
        ObdSupport::Full => l("OBD-II vollständig", "full OBD-II"),
    }
}
