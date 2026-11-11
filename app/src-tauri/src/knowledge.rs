// Lookups into the core data, answered directly: fault codes, zones, the PID
// table, the manufacturer value packs, the vehicle catalog and the VIN.
use std::collections::HashMap;
use std::sync::OnceLock;

use caruspex_data::dtc::{
    brand_families as family_keys, brand_family as family_of_brand, brand_family_label,
    cost_class_hint, cost_class_label, dtc_count, dtc_lexicon as lexicon, dtc_system_name,
    lookup_dtc as lookup,
};
use caruspex_data::dtc_location::locate_dtc as locate;
use caruspex_data::ecu_addresses::{ecu_definitions, ecu_families as ecu_family_keys};
use caruspex_data::pids::{
    PidCategory, default_live_pids as default_pids, pid_category_name, pid_table as table,
};
use caruspex_data::severity::severity_label;
use caruspex_i18n::{Language, language, translate_core_owned, with_language};
use caruspex_protocols::kwp1281::{ecu_name, known_ecus};
use caruspex_record::{built_in_packs, pack_category, pack_names, pack_title};
use caruspex_vehicles::catalog::{self, VehicleGeneration};
use caruspex_vehicles::vin::VinInfo;
use serde_json::{Value, json};

use crate::rows::CustomRow;

/// English names of the PID values and English short names of the pack
/// values, by key. The core tables come in German or English only; every
/// other language looks the English text up in the core catalog. Filled once
/// at start-up, before any other thread runs, because switching the language
/// to build the English tables is global.
struct EnglishNames {
    pids: HashMap<i32, String>,
    units: HashMap<i32, String>,
    titles: HashMap<i32, String>,
    pack_short: HashMap<i32, String>,
    pack_names: HashMap<i32, String>,
    pack_titles: HashMap<String, String>,
}

static ENGLISH: OnceLock<EnglishNames> = OnceLock::new();

pub fn init_english_names() {
    ENGLISH.get_or_init(|| {
        with_language(Language::English, || {
            let mut names = EnglishNames {
                pids: HashMap::new(),
                units: HashMap::new(),
                titles: HashMap::new(),
                pack_short: HashMap::new(),
                pack_names: HashMap::new(),
                pack_titles: HashMap::new(),
            };
            for spec in table() {
                names.titles.insert(spec.pid, spec.title.to_string());
                for v in &spec.values {
                    names.pids.insert(v.key, v.name.to_string());
                    names.units.insert(v.key, v.unit.to_string());
                }
            }
            for p in built_in_packs() {
                names.pack_short.insert(p.key, p.short_name.clone());
                names.pack_names.insert(p.key, p.name.clone());
            }
            for id in pack_names() {
                names.pack_titles.insert(id.to_string(), pack_title(id));
            }
            names
        })
    });
}

fn english() -> Option<&'static EnglishNames> {
    ENGLISH.get()
}

fn other_language() -> Option<Language> {
    match language() {
        Language::German | Language::English => None,
        other => Some(other),
    }
}

/// The name of a live value in the active language.
pub fn live_name(key: i32, name: &str) -> String {
    let Some(lang) = other_language() else {
        return name.to_string();
    };
    let en = english().and_then(|e| e.pids.get(&key).or_else(|| e.pack_names.get(&key)));
    match en {
        Some(en) => translate_core_owned(lang, en),
        None => name.to_string(),
    }
}

fn localized(map: impl Fn(&EnglishNames) -> Option<&String>, fallback: &str) -> String {
    match (other_language(), english().and_then(map)) {
        (Some(lang), Some(en)) => translate_core_owned(lang, en),
        _ => fallback.to_string(),
    }
}

// --- Fault codes -------------------------------------------------------------------

pub fn lookup_dtc(code: &str, brand: &str) -> Value {
    let d = lookup(code, brand);
    json!({
        "code": d.code,
        "text": d.text,
        "severity": d.severity as i32,
        "severityLabel": severity_label(d.severity),
        "system": d.system as i32,
        "systemName": d.system_name,
        "advice": d.advice,
        "causes": d.causes,
        "checks": d.checks,
        "costClass": d.cost_class as i32,
        "costLabel": cost_class_label(d.cost_class),
        "costHint": cost_class_hint(d.cost_class),
        "manufacturerSpecific": d.manufacturer_specific,
        "known": d.known,
        "hasDetails": d.has_details,
        "throttleRelated": d.throttle_related,
        "family": d.family,
        "familyLabel": d.family_label,
    })
}

pub fn locate_dtc(code: &str) -> Value {
    let z = locate(code);
    json!({ "zone": z.zone as i32, "name": z.name, "hint": z.hint })
}

pub fn dtc_lexicon() -> Value {
    let mut systems: HashMap<i32, &'static str> = HashMap::new();
    let mut severities: HashMap<i32, &'static str> = HashMap::new();
    let entries: Vec<Value> = lexicon()
        .into_iter()
        .map(|e| {
            let system = *systems
                .entry(e.system as i32)
                .or_insert_with(|| dtc_system_name(e.system));
            let severity = *severities
                .entry(e.severity as i32)
                .or_insert_with(|| severity_label(e.severity));
            json!({
                "code": e.code,
                "family": e.family,
                "familyLabel": e.family_label,
                "text": e.text,
                "severity": e.severity as i32,
                "system": e.system as i32,
                "systemName": system,
                "severityLabel": severity,
            })
        })
        .collect();
    Value::Array(entries)
}

pub fn brand_families() -> Value {
    let mut out =
        vec![json!({ "key": "", "label": brand_family_label(""), "count": dtc_count("") })];
    for key in family_keys() {
        out.push(json!({ "key": key, "label": brand_family_label(key), "count": dtc_count(key) }));
    }
    Value::Array(out)
}

pub fn brand_family(brand: &str) -> String {
    family_of_brand(brand)
}

// --- PIDs ----------------------------------------------------------------------------

pub fn pid_table() -> Value {
    let mut out = Vec::new();
    for spec in table() {
        let several = spec.values.len() > 1;
        for v in &spec.values {
            let mut info = json!({
                "key": v.key,
                "pid": spec.pid,
                "sub": v.sub,
                "name": localized(|e| e.pids.get(&v.key), v.name),
                "unit": localized(|e| e.units.get(&v.key), v.unit),
                "category": spec.category as i32,
                "categoryName": pid_category_name(spec.category),
                "displayMin": v.display_min,
                "displayMax": v.display_max,
                "warnLow": v.warn_low,
                "warnHigh": v.warn_high,
                "isText": v.text.is_some(),
            });
            if several {
                info["title"] = Value::String(localized(|e| e.titles.get(&spec.pid), spec.title));
            }
            out.push(info);
        }
    }
    Value::Array(out)
}

pub fn default_live_pids() -> Vec<i32> {
    default_pids().to_vec()
}

pub fn pid_packs() -> Value {
    let all = built_in_packs();
    let mut out = Vec::new();
    for id in pack_names() {
        let category = pack_category(id);
        let values: Vec<CustomRow> = all
            .iter()
            .filter(|p| p.pack == id)
            .map(|p| CustomRow {
                key: p.key,
                name: localized(|e| e.pack_names.get(&p.key), &p.name),
                short_name: localized(|e| e.pack_short.get(&p.key), &p.short_name),
                unit: p.unit.clone(),
                pack: p.pack.clone(),
                source: p.source.clone(),
                request: p.request.clone(),
                header: p.header.clone(),
                short_name_en: english()
                    .and_then(|e| e.pack_short.get(&p.key).cloned())
                    .unwrap_or_else(|| p.short_name.clone()),
            })
            .collect();
        let english_title = english()
            .and_then(|e| e.pack_titles.get(id).cloned())
            .unwrap_or_else(|| pack_title(id));
        // The electric car packs are named after the brand ("Hyundai Kona
        // Electric"); the EV page groups them by it.
        let brand = if category == PidCategory::HybridEv {
            english_title.split(' ').next().unwrap_or("").to_string()
        } else {
            String::new()
        };
        out.push(json!({
            "id": id,
            "title": localized(|e| e.pack_titles.get(id), &pack_title(id)),
            "brand": brand,
            "description": "",
            "values": values,
            "category": category as i32,
        }));
    }
    Value::Array(out)
}

// --- Control units -----------------------------------------------------------------

pub fn ecu_families() -> Value {
    Value::Array(
        ecu_family_keys()
            .into_iter()
            .map(|key| {
                let label = brand_family_label(&key);
                let count = ecu_definitions(&key).len();
                json!({ "label": if label.is_empty() { key.clone() } else { label }, "key": key, "count": count })
            })
            .collect(),
    )
}

pub fn kkl_ecus() -> Value {
    Value::Array(
        known_ecus()
            .iter()
            .map(|&e| json!({ "address": e.address(), "name": ecu_name(e) }))
            .collect(),
    )
}

// --- Vehicle catalog -----------------------------------------------------------------

fn index_of(g: &VehicleGeneration) -> usize {
    catalog::all()
        .iter()
        .position(|x| std::ptr::eq(x, g))
        .unwrap_or(0)
}

pub fn generation_info(g: &VehicleGeneration) -> Value {
    json!({
        "id": index_of(g),
        "brand": g.brand,
        "model": g.model,
        "generation": g.generation,
        "yearFrom": g.year_from,
        "yearTo": g.year_to,
        "body": g.body as i32,
        "bodyName": catalog::body_name(g.body),
        "powertrains": g.powertrains,
        "powertrainText": catalog::powertrain_text(g.powertrains),
        "displayName": catalog::display_name(g),
        "yearsText": catalog::years_text(g),
        "protocolOrder": catalog::protocol_order_for(g),
        "note": g.note.get(),
        "usMarket": g.us_market,
    })
}

pub fn catalog_brands() -> Vec<String> {
    catalog::brands()
}

pub fn catalog_models(brand: &str) -> Vec<String> {
    catalog::models(brand)
}

pub fn catalog_generations(brand: &str, model: &str) -> Value {
    Value::Array(
        catalog::generations(brand, model)
            .into_iter()
            .map(generation_info)
            .collect(),
    )
}

pub fn catalog_search(query: &str, limit: usize) -> Value {
    Value::Array(
        catalog::search(query, limit)
            .into_iter()
            .map(generation_info)
            .collect(),
    )
}

pub fn catalog_size() -> usize {
    catalog::size()
}

pub fn catalog_diagnosis(generation_id: usize, year: i32) -> Result<Value, String> {
    let g = catalog::all()
        .get(generation_id)
        .ok_or_else(|| format!("no generation {generation_id}"))?;
    let y = if year <= 0 { g.year_from } else { year };
    let support = catalog::support_for(g, y, 0);
    let connector = catalog::connector_for(g, y);
    Ok(json!({
        "generation": generation_info(g),
        "year": y,
        "support": support as i32,
        "supportName": catalog::support_name(support),
        "connector": connector as i32,
        "connectorName": catalog::connector_name(connector),
        "locationText": catalog::location_text_for(g, y),
        "diagnosisText": catalog::diagnosis_text(g, y),
        "kklPossible": catalog::kkl_possible(g, y),
        "protocolOrder": catalog::protocol_order_for(g),
    }))
}

fn char_text(c: char) -> String {
    if c == '\0' {
        String::new()
    } else {
        c.to_string()
    }
}

fn vin_info(v: &VinInfo) -> Value {
    json!({
        "valid": v.valid,
        "vin": v.vin,
        "wmi": v.wmi,
        "manufacturer": v.manufacturer,
        "brand": v.brand,
        "brandCandidates": v.brand_candidates,
        "country": v.country,
        "region": v.region,
        "northAmerica": v.north_america,
        "modelYears": v.model_years,
        "modelYear": v.model_year,
        "modelYearCertain": v.model_year_certain,
        "yearCode": char_text(v.year_code),
        "checkDigitApplies": v.check_digit_applies,
        "checkDigitOk": v.check_digit_ok,
        "checkDigit": char_text(v.check_digit),
        "expectedCheckDigit": char_text(v.expected_check_digit),
        "plant": char_text(v.plant),
        "serial": v.serial,
        "modelCode": v.model_code,
        "problems": v.problems,
        "notes": v.notes,
    })
}

pub fn decode_vin(vin: &str) -> Value {
    vin_info(&caruspex_vehicles::vin::decode_vin(vin, 0))
}

pub fn match_vin(vin: &str) -> Value {
    let info = caruspex_vehicles::vin::decode_vin(vin, 0);
    let m = catalog::match_vin(&info);
    json!({
        "generation": m.generation.map(generation_info),
        "brand": m.brand,
        "year": m.year,
        "confidence": m.confidence,
        "alternatives": m.alternatives.into_iter().map(generation_info).collect::<Vec<_>>(),
        "reason": m.reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookups_have_the_interface_shape() {
        init_english_names();
        let d = lookup_dtc("P0301", "");
        assert_eq!(d["code"], "P0301");
        assert!(d["known"].as_bool().unwrap());
        assert!(!pid_table().as_array().unwrap().is_empty());
        let families = brand_families();
        assert_eq!(families[0]["key"], "");
        assert!(families.as_array().unwrap().len() > 3);
        let packs = pid_packs();
        assert!(
            packs
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["id"] == "caruspex-demo")
        );
        assert!(catalog_size() > 100);
        let found = catalog_search("golf 4", 5);
        let first = &found[0];
        let diagnosis = catalog_diagnosis(first["id"].as_u64().unwrap() as usize, 2001).unwrap();
        assert_eq!(diagnosis["generation"]["model"], first["model"]);
        let vin = decode_vin("WVWZZZ1JZ3W000001");
        assert_eq!(vin["wmi"], "WVW");
        assert!(match_vin("WVWZZZ1JZ3W000001")["brand"].as_str().is_some());
        assert_eq!(kkl_ecus()[0]["address"], 1);
        assert!(!ecu_families().as_array().unwrap().is_empty());
    }
}
