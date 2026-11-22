//! Custom values from a PID pack or a Torque CSV, and the OBDb packs.

use caruspex_link::Elm327;
use caruspex_link::timing::now_ms;
use caruspex_record::{
    CustomPid, CustomPidReader, JsonWriter, USER_KEY_BASE, obdb_search, obdb_stats, pack_names,
    pack_pids, pack_title, parse_torque_csv,
};

use super::fixed;
use crate::out::{ljust, rjust, say};

/// Values, title and import warnings for `custom`.
pub type CustomValues = (Vec<CustomPid>, String, Vec<String>);

/// The values for `custom`: a built-in pack or a Torque CSV file, with the
/// title and the import warnings. Done before connecting, so a typo costs no
/// adapter time. The error comes with the warnings collected so far.
pub fn load_custom_pids(
    pack: &str,
    csv_path: &str,
    demo: bool,
) -> Result<CustomValues, (String, Vec<String>)> {
    let pack_list = || {
        let mut names = String::new();
        for n in pack_names() {
            names.push_str(&format!("\n  {}  ({})", n, pack_title(n)));
        }
        names.push_str(
            "\nAnd the packs from OBDb (CC BY-SA 4.0): --pack \"obdb:MAKE MODEL YEAR\" lists them.",
        );
        names
    };
    if !csv_path.is_empty() {
        let Ok(bytes) = std::fs::read(csv_path) else {
            return Err((format!("Cannot open {csv_path}"), Vec::new()));
        };
        let (pids, warnings) = parse_torque_csv(&String::from_utf8_lossy(&bytes), USER_KEY_BASE);
        if pids.is_empty() {
            return Err((format!("No usable values in {csv_path}"), warnings));
        }
        return Ok((pids, csv_path.to_string(), warnings));
    }
    let mut name = pack.to_string();
    if name.is_empty() {
        if !demo {
            return Err((
                format!(
                    "Give --pack NAME or --csv FILE. Built-in packs:{}",
                    pack_list()
                ),
                Vec::new(),
            ));
        }
        name = "caruspex-demo".to_string();
    }
    let pids = pack_pids(&name);
    if pids.is_empty() {
        return Err((
            format!("Unknown pack: {name}. Built-in packs:{}", pack_list()),
            Vec::new(),
        ));
    }
    let title = pack_title(&name);
    Ok((pids, title, Vec::new()))
}

/// The query of `--pack obdb:QUERY` (or `--pack obdb` for all), None for any
/// other pack name.
pub fn obdb_query(pack: &str) -> Option<&str> {
    if pack.eq_ignore_ascii_case("obdb") {
        return Some("");
    }
    pack.strip_prefix("obdb:")
}

/// Lists the OBDb packs whose make, model or years fit the query. 0 when
/// something was found.
pub fn list_obdb_packs(query: &str, as_json: bool) -> i32 {
    let hits = obdb_search(query);
    let stats = obdb_stats();
    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object()
            .field("query", query)
            .field("source", "OBDb, github.com/OBDb, CC BY-SA 4.0")
            .key("packs")
            .begin_array();
        for p in &hits {
            json.begin_object()
                .field("id", &p.id)
                .field("title", &p.title())
                .field("repository", &p.repo)
                .field("yearFrom", &p.year_from)
                .field("yearTo", &p.year_to)
                .field("values", &(p.values as i32))
                .end_object();
        }
        json.end_array().end_object();
        say(&json.text());
    } else {
        say(&format!(
            "OBDb packs (github.com/OBDb, CC BY-SA 4.0): {} of {} for \"{query}\"\n",
            hits.len(),
            stats.packs
        ));
        for p in &hits {
            say(&format!(
                "  {}  ({}, {} values)\n",
                p.id,
                p.title(),
                p.values
            ));
        }
        if !hits.is_empty() {
            say("Read one with --pack ID.\n");
        }
    }
    if hits.is_empty() { 1 } else { 0 }
}

/// Up to three decimals, without trailing zeros: 36.5, 3.6, 5614.8, 12.
fn value_text(v: f64) -> String {
    let mut s = format!("{v:.3}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s == "-0" { "0".to_string() } else { s }
}

pub fn custom(
    elm: &mut Elm327,
    pids: &[CustomPid],
    title: &str,
    warnings: &[String],
    as_json: bool,
) -> i32 {
    let started = now_ms();
    let readings = {
        let mut reader = CustomPidReader::new(elm);
        reader.read_all(pids)
    };
    let seconds = (now_ms() - started) as f64 / 1000.0;
    let good = readings.iter().filter(|r| r.ok).count() as i32;

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object()
            .field("title", title)
            .field("seconds", &seconds)
            .field("read", &good)
            .field("failed", &(readings.len() as i32 - good))
            .string_list("importErrors", warnings)
            .key("values")
            .begin_array();
        for (p, r) in pids.iter().zip(&readings) {
            json.begin_object()
                .field("name", &p.name)
                .field("shortName", &p.short_name)
                .field("key", &p.key)
                .field("request", &p.request)
                .field("header", &p.header)
                .field("response", &r.value.ecu_source)
                .field("ok", &r.ok);
            json.key("value");
            if r.ok {
                json.value(&r.value.value);
            } else {
                json.null();
            }
            json.field("unit", &p.unit)
                .field("error", &r.error)
                .field("source", &p.source)
                .end_object();
        }
        json.end_array().end_object();
        say(&json.text());
    } else {
        for w in warnings {
            say(&format!("Skipped: {w}\n"));
        }
        say(&format!("{title}\n"));
        for (p, r) in pids.iter().zip(&readings) {
            if r.ok {
                say(&format!(
                    "  {} {} {}\n",
                    ljust(&p.name, 48),
                    rjust(&value_text(r.value.value), 12),
                    p.unit
                ));
            } else {
                say(&format!(
                    "  {} {} ({})\n",
                    ljust(&p.name, 48),
                    rjust("-", 12),
                    r.error
                ));
            }
        }
        say(&format!(
            "\n{} of {} values read, {} s.\n",
            good,
            readings.len(),
            fixed(seconds, 1)
        ));
    }
    if good > 0 { 0 } else { 4 }
}
