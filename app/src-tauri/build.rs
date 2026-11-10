// Builds the Tauri context and the catalog of the interface texts the
// backend writes itself (the Worker messages, the command-line help, the
// report messages). Those texts are in translations/source/ui_*.json with
// their Qt context and German source, and translations/<code>/ui_*.json maps
// the id to the text in that language, the same files the Qt version and the
// web interface use. The result is OUT_DIR/ui_catalog.json:
// {code: {"context|german": text}}.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const CONTEXTS: [&str; 5] = [
    "caruspex::Worker",
    "caruspex::MainWindow",
    "caruspex::Report",
    "caruspex::BleTransport",
    "main",
];

fn ui_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    name.starts_with("ui_") && name.ends_with(".json")
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

fn read_json(path: &Path) -> serde_json::Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn build_catalog() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../translations");
    println!("cargo:rerun-if-changed={}", root.display());

    // id -> "context|german"
    let mut keys: BTreeMap<String, String> = BTreeMap::new();
    for file in ui_files(&root.join("source")) {
        for item in read_json(&file).as_array().cloned().unwrap_or_default() {
            let context = item["context"].as_str().unwrap_or("");
            if !CONTEXTS.contains(&context) {
                continue;
            }
            let (Some(id), Some(de)) = (item["id"].as_str(), item["de"].as_str()) else {
                continue;
            };
            keys.insert(id.to_string(), format!("{context}|{de}"));
        }
    }

    let mut catalog = serde_json::Map::new();
    let mut codes: Vec<String> = fs::read_dir(&root)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(str::to_string))
                .filter(|name| name != "source")
                .collect()
        })
        .unwrap_or_default();
    codes.sort();
    for code in codes {
        let mut map = serde_json::Map::new();
        for file in ui_files(&root.join(&code)) {
            let texts = read_json(&file);
            let Some(object) = texts.as_object() else {
                continue;
            };
            for (id, text) in object {
                // Plural texts come as a list; the backend writes none.
                if let (Some(key), Some(text)) = (keys.get(id), text.as_str())
                    && !text.is_empty()
                {
                    map.insert(key.clone(), serde_json::Value::String(text.to_string()));
                }
            }
        }
        catalog.insert(code, serde_json::Value::Object(map));
    }

    let out =
        PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set")).join("ui_catalog.json");
    fs::write(out, serde_json::Value::Object(catalog).to_string()).expect("catalog written");
}

fn main() {
    build_catalog();
    tauri_build::build();
}
