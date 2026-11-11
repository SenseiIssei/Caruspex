//! Joins translations/source/core_*.json (id, de, en) with
//! translations/<code>/core_*.json (id -> text) into one catalog per language,
//! keyed by the English text, written to OUT_DIR/core_catalog.json.
//! The same files the Qt version used; nothing is translated twice.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const CODES: [&str; 16] = [
    "es", "fr", "pt", "it", "nl", "pl", "cs", "ru", "uk", "tr", "ar", "zh", "ja", "ko", "hi", "id",
];

fn json_files(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with(prefix) && name.ends_with(".json")
        })
        .collect();
    out.sort();
    out
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../translations");
    println!("cargo:rerun-if-changed={}", root.display());

    let mut en_by_id: BTreeMap<String, String> = BTreeMap::new();
    for file in json_files(&root.join("source"), "core_") {
        let items: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        for item in items.as_array().expect("source file is a list") {
            let id = item["id"].as_str().unwrap().to_string();
            let en = item["en"].as_str().unwrap_or("").to_string();
            if !en.is_empty() {
                en_by_id.insert(id, en);
            }
        }
    }

    let mut catalog = serde_json::Map::new();
    for code in CODES {
        let mut map = serde_json::Map::new();
        for file in json_files(&root.join(code), "core_") {
            let texts: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
            for (id, text) in texts.as_object().expect("language file is an object") {
                if let (Some(en), Some(text)) = (en_by_id.get(id), text.as_str())
                    && !text.is_empty()
                {
                    map.insert(en.clone(), serde_json::Value::String(text.to_string()));
                }
            }
        }
        catalog.insert(code.to_string(), serde_json::Value::Object(map));
    }

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("core_catalog.json");
    fs::write(out, serde_json::to_string(&catalog).unwrap()).unwrap();
}
