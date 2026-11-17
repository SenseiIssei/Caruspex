//! Joins translations/source/core_*.json (id, de, en) with
//! translations/<code>/core_*.json (id -> text) into one catalog per language,
//! keyed by the English text. The same files the Qt version used; nothing is
//! translated twice.
//!
//! Each catalog is written to OUT_DIR/core_<code>.bin as pairs of English
//! text and translation, every text ended by a zero byte, and packed with
//! deflate. The program carries the packed catalogs and unpacks only the one
//! of the language in use, the first time a text of it is asked for
//! (see `catalog` in lib.rs). OUT_DIR/catalogs.rs lists them.

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
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

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

    let mut list = String::from("pub(crate) static PACKED: [(&str, &[u8]); 16] = [\n");
    for code in CODES {
        // Sorted by the English text, so the file is the same on every build.
        let mut map: BTreeMap<&str, String> = BTreeMap::new();
        for file in json_files(&root.join(code), "core_") {
            let texts: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
            for (id, text) in texts.as_object().expect("language file is an object") {
                if let (Some(en), Some(text)) = (en_by_id.get(id), text.as_str())
                    && !text.is_empty()
                {
                    map.insert(en.as_str(), text.to_string());
                }
            }
        }
        let mut raw = Vec::new();
        for (en, text) in &map {
            assert!(
                !en.contains('\0') && !text.contains('\0'),
                "a text with a zero byte: {en}"
            );
            raw.extend_from_slice(en.as_bytes());
            raw.push(0);
            raw.extend_from_slice(text.as_bytes());
            raw.push(0);
        }
        let packed = miniz_oxide::deflate::compress_to_vec(&raw, 10);
        let name = format!("core_{code}.bin");
        fs::write(out_dir.join(&name), packed).unwrap();
        list += &format!(
            "    (\"{code}\", include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{name}\"))),\n"
        );
    }
    list += "];\n";
    fs::write(out_dir.join("catalogs.rs"), list).unwrap();
}
