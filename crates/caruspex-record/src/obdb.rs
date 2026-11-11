//! Manufacturer values from OBDb (github.com/OBDb): one signal set per car
//! model, converted by tools/gen_obdb.py into packs of custom values, one
//! pack per range of model years in which the same commands apply.
//!
//! The data is not part of the MIT code: it lives in data/obdb/ under
//! CC BY-SA 4.0 (see data/obdb/LICENSE and data/obdb/ATTRIBUTION.md) and is
//! embedded compressed. It is unpacked on first use; a pack's values are
//! built only when asked for.
//!
//! Signal names stay as OBDb writes them (English); only the pack titles are
//! translated. Keys are [`OBDB_KEY_BASE`] + the position of the value in the
//! data file, so they stay the same within one build.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

use caruspex_data::pids::PidCategory;
use caruspex_i18n::l;

use crate::custom_pids::{CustomPid, PACK_KEY_BASE, USER_KEY_BASE};

/// Keys of the OBDb values: after the built-in packs, before the imports.
pub const OBDB_KEY_BASE: i32 = PACK_KEY_BASE + 0x1000;

/// Every OBDb pack id starts with this.
pub const OBDB_PREFIX: &str = "obdb-";

static DATA: &[u8] = include_bytes!("../../../data/obdb/obdb-packs.z");

/// One pack: the commands of one OBDb repository for a range of model years.
#[derive(Clone, Debug)]
pub struct ObdbPack {
    /// "obdb-toyota-prius-2016-2022".
    pub id: String,
    /// The OBDb repository, "Toyota-Prius".
    pub repo: String,
    /// Make and model to show, "Toyota", "Prius".
    pub make: String,
    pub model: String,
    /// First and last model year, 0 = open.
    pub year_from: i32,
    pub year_to: i32,
    /// Brand and model of the vehicle catalog the repository stands for.
    pub catalog: Vec<(String, String)>,
    /// Holds traction battery values (state of charge, health, voltage).
    pub ev: bool,
    /// Number of values.
    pub values: usize,
    repo_index: usize,
    segment: usize,
}

impl ObdbPack {
    /// Whether the pack fits a car of the catalog: same brand and model, and
    /// the model year inside the pack's years (any year when `year` <= 0).
    pub fn fits(&self, brand: &str, model: &str, year: i32) -> bool {
        self.catalog
            .iter()
            .any(|(b, m)| b.eq_ignore_ascii_case(brand) && m.eq_ignore_ascii_case(model))
            && (year <= 0 || self.covers(year))
    }

    pub fn covers(&self, year: i32) -> bool {
        (self.year_from == 0 || year >= self.year_from)
            && (self.year_to == 0 || year <= self.year_to)
    }

    /// "Toyota Prius, model years 2016 to 2022 (OBDb)", active language.
    pub fn title(&self) -> String {
        let name = format!("{} {}", self.make, self.model);
        let from = self.year_from.to_string();
        let to = self.year_to.to_string();
        let pattern = match (self.year_from, self.year_to) {
            (0, 0) => return format!("{name} (OBDb)"),
            (_, 0) => l("%1, ab Modelljahr %2 (OBDb)", "%1, model year %2 on (OBDb)"),
            (0, _) => {
                return l(
                    "%1, bis Modelljahr %2 (OBDb)",
                    "%1, up to model year %2 (OBDb)",
                )
                .replace("%1", &name)
                .replace("%2", &to);
            }
            (a, b) if a == b => l("%1, Modelljahr %2 (OBDb)", "%1, model year %2 (OBDb)"),
            _ => {
                return l(
                    "%1, Modelljahre %2 bis %3 (OBDb)",
                    "%1, model years %2 to %3 (OBDb)",
                )
                .replace("%1", &name)
                .replace("%2", &from)
                .replace("%3", &to);
            }
        };
        pattern.replace("%1", &name).replace("%2", &from)
    }

    /// Where the values come from, shown next to each value.
    pub fn source(&self) -> String {
        format!("OBDb {} (CC BY-SA 4.0)", self.repo)
    }

    /// Category the values are shown under.
    pub fn category(&self) -> PidCategory {
        if self.ev {
            PidCategory::HybridEv
        } else {
            PidCategory::Custom
        }
    }
}

/// Totals of the import, from the data file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObdbStats {
    /// Car models with at least one value.
    pub vehicles: usize,
    /// OBDb repositories read.
    pub repositories: usize,
    /// Signals imported and skipped (not expressible exactly).
    pub signals: usize,
    pub skipped: usize,
    pub packs: usize,
}

struct Repo {
    /// Line range of the repository's C and V lines.
    lines: std::ops::Range<usize>,
    /// Key of its first value.
    first_key: i32,
    values: usize,
    /// Index of its first pack in `Index::packs`.
    first_pack: usize,
}

struct Index {
    text: String,
    /// Byte ranges of the lines of `text`.
    lines: Vec<std::ops::Range<usize>>,
    repos: Vec<Repo>,
    packs: Vec<ObdbPack>,
    stats: ObdbStats,
}

impl Index {
    fn line(&self, i: usize) -> &str {
        &self.text[self.lines[i].clone()]
    }
}

fn index() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| build_index(DATA).unwrap_or_else(|e| panic!("data/obdb: {e}")))
}

fn number(text: &str, what: &str) -> Result<usize, String> {
    text.parse::<usize>()
        .map_err(|_| format!("{what} '{text}' is not a number"))
}

fn build_index(compressed: &[u8]) -> Result<Index, String> {
    let bytes = miniz_oxide::inflate::decompress_to_vec_zlib(compressed)
        .map_err(|e| format!("cannot unpack: {e:?}"))?;
    let text = String::from_utf8(bytes).map_err(|_| "not UTF-8".to_string())?;
    let mut lines = Vec::new();
    let mut start = 0;
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            lines.push(start..i);
            start = i + 1;
        }
    }
    if start < text.len() {
        lines.push(start..text.len());
    }
    let mut index = Index {
        text: String::new(),
        lines: Vec::new(),
        repos: Vec::new(),
        packs: Vec::new(),
        stats: ObdbStats::default(),
    };
    let mut key = OBDB_KEY_BASE;
    for (n, range) in lines.iter().enumerate() {
        let line = &text[range.clone()];
        let cells: Vec<&str> = line.split('\t').collect();
        match cells[0] {
            "O" if cells.len() == 6 => {
                index.stats = ObdbStats {
                    vehicles: number(cells[1], "vehicles")?,
                    repositories: number(cells[2], "repositories")?,
                    signals: number(cells[3], "signals")?,
                    skipped: number(cells[4], "skipped")?,
                    packs: number(cells[5], "packs")?,
                };
            }
            "R" if cells.len() == 7 => {
                if let Some(last) = index.repos.last_mut() {
                    last.lines.end = n;
                }
                let repo_index = index.repos.len();
                let first_pack = index.packs.len();
                let catalog: Vec<(String, String)> = cells[5]
                    .split(';')
                    .filter_map(|pair| pair.split_once('|'))
                    .map(|(b, m)| (b.to_string(), m.to_string()))
                    .collect();
                for (segment, years) in cells[4].split(',').enumerate() {
                    let (from, to) = years
                        .split_once('-')
                        .ok_or_else(|| format!("line {}: years '{years}'", n + 1))?;
                    let year_from = number(from, "year")? as i32;
                    let year_to = number(to, "year")? as i32;
                    let single = cells[4].split(',').count() == 1;
                    index.packs.push(ObdbPack {
                        id: pack_id(cells[1], year_from, year_to, single),
                        repo: cells[1].to_string(),
                        make: cells[2].to_string(),
                        model: cells[3].to_string(),
                        year_from,
                        year_to,
                        catalog: catalog.clone(),
                        ev: cells[6] == "1",
                        values: 0,
                        repo_index,
                        segment,
                    });
                }
                index.repos.push(Repo {
                    lines: n + 1..n + 1,
                    first_key: key,
                    values: 0,
                    first_pack,
                });
            }
            "C" if cells.len() == 5 => {
                let repo = index
                    .repos
                    .last()
                    .ok_or_else(|| format!("line {}: C before R", n + 1))?;
                let mask = u64::from_str_radix(cells[1], 16)
                    .map_err(|_| format!("line {}: mask '{}'", n + 1, cells[1]))?;
                // The number of values follows from the V lines; count them
                // into the packs as they come.
                let count = lines[n + 1..]
                    .iter()
                    .take_while(|r| text[(*r).clone()].starts_with("V\t"))
                    .count();
                let first = repo.first_pack;
                let packs = index.packs.len() - first;
                for (segment, pack) in index.packs[first..].iter_mut().enumerate() {
                    if segment < packs && mask & (1 << segment) != 0 {
                        pack.values += count;
                    }
                }
            }
            "V" if cells.len() == 7 => {
                let repo = index
                    .repos
                    .last_mut()
                    .ok_or_else(|| format!("line {}: V before R", n + 1))?;
                repo.values += 1;
                key += 1;
            }
            "" => {}
            _ => return Err(format!("line {}: unexpected '{line}'", n + 1)),
        }
    }
    if let Some(last) = index.repos.last_mut() {
        last.lines.end = lines.len();
    }
    if key >= USER_KEY_BASE {
        return Err("too many values for the key range".to_string());
    }
    index.text = text;
    index.lines = lines;
    Ok(index)
}

fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn pack_id(repo: &str, from: i32, to: i32, single: bool) -> String {
    let base = format!("{OBDB_PREFIX}{}", slug(repo));
    if single && from == 0 && to == 0 {
        return base;
    }
    let year = |y: i32| {
        if y == 0 {
            "x".to_string()
        } else {
            y.to_string()
        }
    };
    format!("{base}-{}-{}", year(from), year(to))
}

/// All OBDb packs, by make and model, oldest years first.
pub fn obdb_packs() -> &'static [ObdbPack] {
    &index().packs
}

/// Totals of the import.
pub fn obdb_stats() -> ObdbStats {
    index().stats
}

/// One pack by id.
pub fn obdb_pack(id: &str) -> Option<&'static ObdbPack> {
    if !id.starts_with(OBDB_PREFIX) {
        return None;
    }
    index().packs.iter().find(|p| p.id == id)
}

/// The packs for a car of the vehicle catalog: its brand and model, and the
/// model year when known (0 = all years).
pub fn obdb_packs_for(brand: &str, model: &str, year: i32) -> Vec<&'static ObdbPack> {
    index()
        .packs
        .iter()
        .filter(|p| p.fits(brand, model, year))
        .collect()
}

/// Packs whose make, model or repository contain every word of the query; a
/// four digit word is a model year the pack must cover. For the CLI.
pub fn obdb_search(query: &str) -> Vec<&'static ObdbPack> {
    let words: Vec<String> = query
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    index()
        .packs
        .iter()
        .filter(|p| {
            let hay = format!("{} {} {} {}", p.make, p.model, p.repo, p.id).to_lowercase();
            words.iter().all(|w| match w.parse::<i32>() {
                Ok(year) if w.len() == 4 => p.covers(year),
                _ => hay.contains(w.as_str()),
            })
        })
        .collect()
}

/// Builds the values of one pack.
fn build_values(index: &Index, pack: &ObdbPack) -> Vec<CustomPid> {
    let repo = &index.repos[pack.repo_index];
    let mut out = Vec::new();
    let mut key = repo.first_key;
    let mut mask = 0u64;
    let mut address = ("", "", "");
    let mut metrics: BTreeSet<&str> = BTreeSet::new();
    for n in repo.lines.clone() {
        let cells: Vec<&str> = index.line(n).split('\t').collect();
        if cells[0] == "C" {
            mask = u64::from_str_radix(cells[1], 16).unwrap_or(0);
            address = (cells[2], cells[3], cells[4]);
            continue;
        }
        if cells[0] != "V" {
            continue;
        }
        let this_key = key;
        key += 1;
        if mask & (1 << pack.segment) == 0 {
            continue;
        }
        // The EV page finds its values by these short names; the first value
        // of a kind in the pack gets it.
        let metric = cells[2];
        let short = if !metric.is_empty() && metrics.insert(metric) {
            metric
        } else {
            cells[1]
        };
        out.push(CustomPid {
            name: cells[1].to_string(),
            short_name: short.to_string(),
            request: address.0.to_string(),
            header: address.1.to_string(),
            response: address.2.to_string(),
            equation: cells[3].to_string(),
            min_value: cells[4].parse().unwrap_or(0.0),
            max_value: cells[5].parse().unwrap_or(0.0),
            unit: cells[6].to_string(),
            pack: pack.id.clone(),
            source: pack.source(),
            key: this_key,
            category: pack.category(),
        });
    }
    out
}

/// The values of a pack, built once per run: the live view asks for them
/// again on every reading.
fn cached_values(pack: &'static ObdbPack) -> Arc<Vec<CustomPid>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Vec<CustomPid>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    // The short names do not depend on the language, the titles are not
    // stored, so one entry per pack is enough.
    let mut map = cache.lock().unwrap_or_else(|e| e.into_inner());
    map.entry(pack.id.clone())
        .or_insert_with(|| Arc::new(build_values(index(), pack)))
        .clone()
}

/// The values of one OBDb pack, empty for an unknown id.
pub fn obdb_pack_pids(id: &str) -> Vec<CustomPid> {
    obdb_pack(id).map_or_else(Vec::new, |pack| cached_values(pack).to_vec())
}

/// The OBDb value with this key, as part of the first pack that holds it.
pub fn obdb_pid(key: i32) -> Option<CustomPid> {
    if !(OBDB_KEY_BASE..USER_KEY_BASE).contains(&key) {
        return None;
    }
    let index = index();
    let at = index
        .repos
        .partition_point(|r| r.first_key + r.values as i32 <= key);
    let repo = index.repos.get(at)?;
    if key < repo.first_key {
        return None;
    }
    // The commands above the value say which packs hold it.
    let mut mask = 0u64;
    let mut next = repo.first_key;
    for n in repo.lines.clone() {
        let line = index.line(n);
        if let Some(rest) = line.strip_prefix("C\t") {
            mask = u64::from_str_radix(rest.split('\t').next().unwrap_or(""), 16).unwrap_or(0);
        } else if line.starts_with("V\t") {
            if next == key {
                break;
            }
            next += 1;
        }
    }
    let pack = index
        .packs
        .get(repo.first_pack + mask.trailing_zeros() as usize)?;
    cached_values(pack).iter().find(|p| p.key == key).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_lower_case_words_with_the_years() {
        assert_eq!(
            pack_id("Toyota-Prius", 2016, 2022, false),
            "obdb-toyota-prius-2016-2022"
        );
        assert_eq!(
            pack_id("Volkswagen-ID.4", 2021, 0, false),
            "obdb-volkswagen-id-4-2021-x"
        );
        assert_eq!(pack_id("Kia-EV6", 0, 0, true), "obdb-kia-ev6");
    }

    #[test]
    fn a_small_data_file_is_read() {
        let text = "O\t1\t1\t3\t1\t2\n\
                    R\tAcme-Rocket\tAcme\tRocket\t2010-2014,2015-0\tAcme|Rocket\t1\n\
                    C\t3\t220101\t7E4\t7EC\n\
                    V\tState of charge\tSOC\tA/2\t0\t100\t%\n\
                    C\t2\t220102\t7E4\t\n\
                    V\tCell 1\t\tINT16(A:B)/1000\t0\t5\tV\n\
                    V\tCell 2\t\tINT16(C:D)/1000\t0\t5\tV\n";
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(text.as_bytes(), 6);
        let index = build_index(&packed).unwrap();
        assert_eq!(index.stats.signals, 3);
        assert_eq!(index.packs.len(), 2);
        assert_eq!(index.packs[0].id, "obdb-acme-rocket-2010-2014");
        assert_eq!(index.packs[1].id, "obdb-acme-rocket-2015-x");
        assert_eq!(index.packs[0].values, 1);
        assert_eq!(index.packs[1].values, 3);
        assert!(index.packs[1].fits("Acme", "Rocket", 2020));
        assert!(!index.packs[0].fits("Acme", "Rocket", 2020));
        let early = build_values(&index, &index.packs[0]);
        assert_eq!(early.len(), 1);
        assert_eq!(early[0].short_name, "SOC");
        assert_eq!(early[0].key, OBDB_KEY_BASE);
        let late = build_values(&index, &index.packs[1]);
        assert_eq!(late.len(), 3);
        assert_eq!(late[2].key, OBDB_KEY_BASE + 2);
        assert_eq!(late[1].response, "");
        assert_eq!(late[2].pack, "obdb-acme-rocket-2015-x");
    }
}
