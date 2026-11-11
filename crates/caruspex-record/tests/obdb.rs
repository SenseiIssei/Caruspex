// The packs imported from OBDb (tools/gen_obdb.py, data/obdb/).
//
// tests/fixtures/obdb_signals.json holds real OBDb commands with an answer
// recorded on a real car and the values OBDb's own decoder gives for it (its
// test cases). tools/gen_obdb.py --fixtures converts them the same way as the
// whole import into tests/fixtures/obdb_cases.json, and checks there that
// OBDb's decoder gives those values. Here the equations go through the Caruspex
// evaluator and, for the cars on 11-bit CAN, through the adapter as a pack.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use caruspex_data::pids::PidCategory;
use caruspex_i18n::Language;
use caruspex_record::{
    CustomPidReader, OBDB_KEY_BASE, TorqueEquation, USER_KEY_BASE, obdb_pack, obdb_pack_pids,
    obdb_packs, obdb_packs_for, obdb_pid, obdb_search, obdb_stats, pack_category, pack_pids,
    pack_title,
};
use caruspex_vehicles::catalog;
use serde_json::Value;

use common::{ScriptCar, approx, german, language};

fn cases() -> Vec<Value> {
    let text = include_str!("fixtures/obdb_cases.json");
    let json: Value = serde_json::from_str(text).expect("fixture is JSON");
    json["entries"].as_array().unwrap().clone()
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn converted_real_signals_give_obdbs_values() {
    let _lang = german();
    let mut checked = 0;
    let mut skipped = BTreeSet::new();
    for entry in cases() {
        if let Some(reason) = entry["skip"].as_str() {
            skipped.insert(reason.to_string());
            continue;
        }
        let data = hex(entry["data"].as_str().unwrap());
        for signal in entry["signals"].as_array().unwrap() {
            if let Some(reason) = signal["skip"].as_str() {
                skipped.insert(reason.to_string());
                continue;
            }
            let equation = signal["equation"].as_str().unwrap();
            let eq = TorqueEquation::new(equation);
            assert!(eq.ok(), "{equation}: {}", eq.error());
            let value = eq
                .evaluate(&data)
                .unwrap_or_else(|e| panic!("{equation}: {e}"));
            let expected = signal["expected"].as_f64().unwrap();
            assert!(
                approx(value, expected),
                "{} {}: {equation} gives {value}, OBDb {expected}",
                entry["repo"],
                signal["id"]
            );
            checked += 1;
        }
    }
    assert!(checked > 90, "{checked}");
    // The fixtures hold one example of each reason a signal is left out.
    for reason in ["enumeration", "null bound", "session"] {
        assert!(skipped.contains(reason), "{reason}: {skipped:?}");
    }
}

#[test]
fn every_imported_equation_parses_and_keys_are_unique() {
    let _lang = german();
    let stats = obdb_stats();
    let packs = obdb_packs();
    assert_eq!(packs.len(), stats.packs);
    assert!(stats.vehicles > 400, "{stats:?}");
    assert!(stats.signals > 100_000, "{stats:?}");
    let mut ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    let mut equations = BTreeSet::new();
    let mut repos = BTreeSet::new();
    for pack in packs {
        assert!(ids.insert(pack.id.clone()), "{} twice", pack.id);
        assert!(pack.id.starts_with("obdb-"));
        repos.insert(pack.repo.clone());
        let pids = obdb_pack_pids(&pack.id);
        assert_eq!(pids.len(), pack.values, "{}", pack.id);
        assert!(!pids.is_empty(), "{}", pack.id);
        for p in &pids {
            assert!((OBDB_KEY_BASE..USER_KEY_BASE).contains(&p.key));
            keys.insert(p.key);
            assert_eq!(p.pack, pack.id);
            assert!(p.source.contains(&pack.repo));
            assert!(!p.name.is_empty() && !p.request.is_empty());
            equations.insert(p.equation.clone());
        }
    }
    assert_eq!(repos.len(), stats.vehicles);
    // Values shared by packs of one car keep their key, so there are no more
    // keys than imported signals.
    assert_eq!(keys.len(), stats.signals);
    for equation in &equations {
        let eq = TorqueEquation::new(equation);
        assert!(eq.ok(), "{equation}: {}", eq.error());
    }
}

#[test]
fn a_value_is_found_by_its_key() {
    let _lang = german();
    let pack = obdb_packs()
        .iter()
        .find(|p| p.repo == "Toyota-Prius")
        .unwrap();
    let pids = obdb_pack_pids(&pack.id);
    for p in [&pids[0], &pids[pids.len() / 2], pids.last().unwrap()] {
        let found = obdb_pid(p.key).expect("known key");
        assert_eq!(found.key, p.key);
        assert_eq!(found.equation, p.equation);
        assert_eq!(found.request, p.request);
    }
    assert!(obdb_pid(OBDB_KEY_BASE - 1).is_none());
    assert!(obdb_pid(USER_KEY_BASE).is_none());
}

#[test]
fn packs_are_matched_to_the_catalog_by_brand_model_and_year() {
    let _lang = german();
    // Every catalog model a pack names exists in the catalog.
    for pack in obdb_packs() {
        for (brand, model) in &pack.catalog {
            assert!(
                !catalog::generations(brand, model).is_empty(),
                "{}: {brand} {model} is not in the catalog",
                pack.repo
            );
        }
    }
    let repos = |brand: &str, model: &str, year: i32| -> BTreeSet<String> {
        obdb_packs_for(brand, model, year)
            .iter()
            .map(|p| p.repo.clone())
            .collect()
    };
    // By name, through a search alias of the catalog ("Kona Electric"),
    // without the drive word ("Camry Hybrid") and through data/obdb/aliases.csv.
    assert!(repos("Hyundai", "Ioniq 5", 2023).contains("Hyundai-IONIQ-5"));
    assert!(repos("Hyundai", "Kona", 2020).contains("Hyundai-Kona-Electric"));
    assert!(repos("Toyota", "Camry", 2020).contains("Toyota-Camry-Hybrid"));
    assert!(repos("Audi", "A3", 2018).contains("Audi-S3"));
    assert!(repos("Mercedes-Benz", "C-Klasse", 2016).contains("Mercedes-Benz-C-Class"));
    // A model year outside every pack: nothing; an unknown model: nothing.
    assert!(repos("Hyundai", "Ioniq 5", 1990).is_empty());
    assert!(repos("Trabant", "601", 1980).is_empty());
    // Exactly one pack of a car covers a given year.
    for year in 2005..=2024 {
        let packs: Vec<_> = obdb_packs_for("Toyota", "Prius", year)
            .into_iter()
            .filter(|p| p.repo == "Toyota-Prius")
            .collect();
        assert_eq!(packs.len(), 1, "{year}");
        assert!(packs[0].covers(year));
    }
    // A detected car from a VIN finds its pack: a 2020 Kona Electric.
    let vin = caruspex_vehicles::vin::decode_vin("KMHK381GFLU000000", 0);
    let found = catalog::match_vin(&vin);
    if let Some(g) = found.generation {
        assert!(
            !obdb_packs_for(g.brand, g.model, found.year).is_empty(),
            "{} {} {}",
            g.brand,
            g.model,
            found.year
        );
    }
}

#[test]
fn packs_have_titles_categories_and_a_search() {
    let pack = obdb_packs()
        .iter()
        .find(|p| p.repo == "Hyundai-IONIQ-5")
        .unwrap();
    {
        let _lang = language(Language::English);
        let title = pack_title(&pack.id);
        assert!(title.starts_with("Hyundai IONIQ 5"), "{title}");
        assert!(title.ends_with("(OBDb)"), "{title}");
    }
    {
        let _lang = german();
        assert!(pack_title(&pack.id).contains("Modelljahr"));
        assert_eq!(pack_category(&pack.id), PidCategory::HybridEv);
        assert_eq!(pack_pids(&pack.id).len(), pack.values);
        assert!(obdb_pack("obdb-nope").is_none());
        assert!(pack_title("obdb-nope").is_empty());
        let hits = obdb_search("ioniq 5 2024");
        assert!(hits.iter().any(|p| p.id == pack.id || p.repo == pack.repo));
        assert!(hits.iter().all(|p| p.covers(2024)));
        // The EV page finds the battery values by their short names.
        let shorts: BTreeSet<String> = obdb_pack_pids(&pack.id)
            .into_iter()
            .map(|p| p.short_name)
            .collect();
        assert!(shorts.contains("SOC"), "{shorts:?}");
    }
}

/// One fixture command answered by a scripted adapter and read as part of
/// the imported pack of that car and year.
fn read_through_adapter(repo: &str, request: &str) {
    let entry = cases()
        .into_iter()
        .find(|e| e["repo"] == repo && e["request"] == request)
        .unwrap();
    let header = entry["header"].as_str().unwrap();
    let year = entry["year"].as_i64().unwrap() as i32;
    let pack = obdb_packs()
        .iter()
        .find(|p| p.repo == repo && p.covers(year))
        .unwrap_or_else(|| panic!("no {repo} pack for {year}"));
    let pids: Vec<_> = obdb_pack_pids(&pack.id)
        .into_iter()
        .filter(|p| p.request == request && p.header == header)
        .collect();
    let expected: BTreeMap<String, f64> = entry["signals"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["equation"].is_string())
        .map(|s| {
            (
                s["name"].as_str().unwrap().to_string(),
                s["expected"].as_f64().unwrap(),
            )
        })
        .collect();
    assert_eq!(pids.len(), expected.len(), "{repo} {request}");

    let mut car = ScriptCar::new();
    assert!(car.connected);
    let frames: Vec<String> = entry["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap().to_string())
        .collect();
    car.set_frames(&format!("{header}|{request}"), &frames);
    let readings = {
        let mut reader = CustomPidReader::new(&mut car.elm);
        reader.read_all(&pids)
    };
    assert_eq!(car.count(&format!("ATSH{header}")), 1);
    for (pid, reading) in pids.iter().zip(&readings) {
        assert!(reading.ok, "{}: {}", pid.name, reading.error);
        let want = expected[&pid.name];
        assert!(
            approx(reading.value.value, want),
            "{}: {} vs {want}",
            pid.name,
            reading.value.value
        );
    }
}

#[test]
fn an_imported_pack_reads_through_a_scripted_adapter() {
    let _lang = german();
    // A long multi-frame answer with byte-swapped and signed values.
    read_through_adapter("Hyundai-IONIQ-5", "22E001");
    // The battery management, with single bits and clamped values.
    read_through_adapter("Hyundai-IONIQ-5", "220101");
    // Service 21 of a Toyota, bits inside bytes.
    read_through_adapter("Toyota-Prius", "2106");
}
