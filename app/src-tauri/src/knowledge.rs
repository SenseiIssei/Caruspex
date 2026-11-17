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
use caruspex_i18n::{Language, l, language, translate_core_owned, with_language};
use caruspex_protocols::kwp1281::{ecu_name, known_ecus};
use caruspex_record::{
    built_in_packs, listen_packs_for, obdb_pack_pids, obdb_packs_for, pack_category, pack_listens,
    pack_names, pack_title,
};
use caruspex_vehicles::anatomy;
use caruspex_vehicles::catalog::{self, BodyType, VehicleGeneration};
use caruspex_vehicles::vin::VinInfo;
use caruspex_vehicles::{engines, moto};
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

/// A J1939 fault ("SPN 110 FMI 0") in the shape of the OBD lookup: what
/// the SPN and the FMI say; the lamps of the unit tell how urgent it is.
fn j1939_dtc(code: &str) -> Option<caruspex_data::dtc::DtcInfo> {
    use caruspex_data::severity::Severity;
    use caruspex_protocols::j1939::{describe, find_param, fmi_weight, parse_code};
    let (spn, fmi) = parse_code(code)?;
    Some(caruspex_data::dtc::DtcInfo {
        code: format!("SPN {spn} FMI {fmi}"),
        text: describe(spn, fmi),
        severity: match fmi_weight(fmi) {
            1 => Severity::Low,
            2 => Severity::Medium,
            _ => Severity::High,
        },
        system_name: "SAE J1939".into(),
        advice: l(
            "Fehler eines Lkw-Steuergeräts nach SAE J1939. Wie eilig er ist, zeigen die Lampen des Steuergeräts: Rote Stopplampe heißt anhalten, sobald es sicher geht, gelbe Warnlampe heißt bald in die Werkstatt.",
            "Fault of a truck control unit under SAE J1939. How urgent it is, the unit's lamps show: the red stop lamp means stop as soon as it is safe, the amber warning lamp means see a workshop soon.",
        )
        .into(),
        // SPN 520192 and up belong to the makers.
        manufacturer_specific: spn >= 520_192,
        known: find_param(spn).is_some(),
        ..Default::default()
    })
}

pub fn lookup_dtc(code: &str, brand: &str) -> Value {
    let d = j1939_dtc(code).unwrap_or_else(|| lookup(code, brand));
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

/// The built-in packs for every car. The listening packs of opendbc are left
/// out: they come with the car they were made for, see [`obdb_packs`].
pub fn pid_packs() -> Value {
    Value::Array(
        pack_names()
            .into_iter()
            .filter(|id| !pack_listens(id))
            .map(built_in_pack_json)
            .collect(),
    )
}

/// Said with every listening pack: the values only come if the bus they are
/// sent on reaches the OBD socket.
pub fn listen_note() -> &'static str {
    l(
        "Diese Werte sendet das Auto von selbst, Caruspex hört nur mit. Viele dieser Busse \
         liegen hinter dem Gateway und sind an der OBD-Buchse nicht zu hören; bleiben die \
         Werte leer, liegt das am Auto, nicht am Adapter. Quelle: opendbc (MIT).",
        "The car sends these values by itself, Caruspex only listens. Many of these buses sit \
         behind the gateway and cannot be heard at the OBD socket; if the values stay empty, \
         that is the car, not the adapter. Source: opendbc (MIT).",
    )
}

fn built_in_pack_json(id: &'static str) -> Value {
    let all = built_in_packs();
    {
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
        let listen = pack_listens(id);
        json!({
            "id": id,
            "title": localized(|e| e.pack_titles.get(id), &pack_title(id)),
            "brand": brand,
            "description": if listen { listen_note() } else { "" },
            "values": values,
            "category": category as i32,
            "listen": listen,
        })
    }
}

/// The OBDb packs for a car of the catalog (generation id and model year,
/// 0 = any), in the shape of [`pid_packs`], followed by the opendbc packs
/// for listening that fit it ("listen": true). Empty without a generation.
/// The OBDb value names stay as OBDb writes them; the titles are translated.
pub fn obdb_packs(generation_id: Option<usize>, year: i32) -> Value {
    let Some(g) = generation_id.and_then(|id| catalog::all().get(id)) else {
        return Value::Array(Vec::new());
    };
    let packs = obdb_packs_for(g.brand, g.model, year);
    // The opendbc packs for listening follow the OBDb ones. Without a model
    // year the generation's years decide.
    let listen = if year > 0 {
        listen_packs_for(g.brand, g.model, year)
    } else {
        let last = if g.year_to > 0 { g.year_to } else { 2030 };
        (g.year_from..=last)
            .flat_map(|y| listen_packs_for(g.brand, g.model, y))
            .fold(Vec::new(), |mut acc, id| {
                if !acc.contains(&id) {
                    acc.push(id);
                }
                acc
            })
    };
    let listen = listen.into_iter().map(built_in_pack_json);
    Value::Array(
        packs
            .into_iter()
            .map(|pack| {
                let values: Vec<CustomRow> = obdb_pack_pids(&pack.id)
                    .into_iter()
                    .map(|p| CustomRow {
                        key: p.key,
                        name: p.name,
                        short_name_en: p.short_name.clone(),
                        short_name: p.short_name,
                        unit: p.unit,
                        pack: p.pack,
                        source: p.source,
                        request: p.request,
                        header: p.header,
                    })
                    .collect();
                json!({
                    "id": pack.id,
                    "title": pack.title(),
                    "brand": g.brand,
                    "description": pack.source(),
                    "values": values,
                    "category": pack.category() as i32,
                    "obdb": true,
                })
            })
            .chain(listen)
            .collect(),
    )
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
        "locationText": catalog::socket_text_for(g, y),
        // SocketLocation in declaration order, for the sketch.
        "location": catalog::location_for(g, y) as i32,
        // SocketSpot in declaration order (0 = not known exactly) and
        // whether a cover hides the socket.
        "spot": g.spot as i32,
        "spotCover": g.spot_cover,
        "motorcycle": moto::is_motorcycle(g),
        // The connect help of a motorcycle (cable, kill switch, Euro 5).
        "connectSteps": if moto::is_motorcycle(g) { moto::connect_steps() } else { Vec::new() },
        "engineCodes": engine_codes_json(g),
        "diagnosisText": catalog::diagnosis_text(g, y),
        "kklPossible": catalog::kkl_possible(g, y),
        "protocolOrder": catalog::protocol_order_for(g),
    }))
}

/// The engine codes of a generation from Wikipedia, petrol and diesel, with
/// the article they come from; null without codes.
fn engine_codes_json(g: &VehicleGeneration) -> Value {
    match engines::engines_for(g) {
        Some(e) => json!({
            "petrol": e.petrol,
            "diesel": e.diesel,
            "source": e.source,
        }),
        None => Value::Null,
    }
}

/// Dimensions and drivetrain for the X-ray view. A generation of the
/// catalog (generation_id >= 0) brings its own body, brand and engines; the
/// demo car and unknown cars pass body, brand and model directly. variant is
/// one `PT_*` flag, 0 for the default.
pub fn vehicle_anatomy(
    generation_id: i64,
    year: i32,
    body: i32,
    brand: &str,
    model: &str,
    variant: u32,
) -> Value {
    let generation = usize::try_from(generation_id)
        .ok()
        .and_then(|i| catalog::all().get(i));
    let (dims, source, drivetrain, variants, variant) = match generation {
        Some(g) => {
            let (dims, source) = anatomy::dimensions_for(g, year);
            let variant = if variant == 0 || g.powertrains & variant == 0 {
                anatomy::default_powertrain(g.powertrains)
            } else {
                variant
            };
            let drivetrain = anatomy::generation_drivetrain(g, year, variant);
            let variants = anatomy::powertrain_variants(g.powertrains);
            (dims, source, drivetrain, variants, variant)
        }
        None => {
            let body = usize::try_from(body)
                .ok()
                .and_then(|i| BodyType::ALL.get(i).copied())
                .unwrap_or(BodyType::Hatchback);
            let variant = anatomy::default_powertrain(variant);
            let dims = anatomy::typical_dimensions(body, year);
            let drivetrain = anatomy::drivetrain(brand, model, body, year, variant);
            let source = anatomy::DimensionSource::Typical;
            (dims, source, drivetrain, vec![variant], variant)
        }
    };
    // Where the diagnostic socket sits (SocketLocation and SocketSpot in
    // declaration order, 0 = not known): the X-ray view puts it there.
    let (socket_location, socket_spot) = match generation {
        Some(g) => (catalog::location_for(g, year) as i32, g.spot as i32),
        None => (0, 0),
    };
    let motorcycle = match generation {
        Some(g) => g.body == BodyType::Motorcycle,
        None => usize::try_from(body)
            .ok()
            .and_then(|i| BodyType::ALL.get(i))
            .is_some_and(|b| *b == BodyType::Motorcycle),
    };
    let (gen_brand, gen_model, gen_code, item, mass, document) = match generation {
        Some(g) => (
            g.brand,
            g.model,
            g.generation,
            anatomy::wikidata_item(g),
            anatomy::wikidata_mass(g),
            anatomy::dimension_document(g),
        ),
        None => (brand, model, "", "", 0, ""),
    };
    json!({
        "brand": gen_brand,
        "model": gen_model,
        "generation": gen_code,
        "year": if year > 0 { year } else { generation.map_or(0, |g| g.year_from) },
        "dimensionSource": match source {
            anatomy::DimensionSource::Wikidata => "wikidata",
            anatomy::DimensionSource::Manufacturer => "manufacturer",
            anatomy::DimensionSource::Typical => "typical",
        },
        "dimensionDocument": document,
        "measuredFields": dims.measured,
        "wikidataId": item,
        "massKg": mass,
        "bodyNames": BodyType::ALL.iter().map(|&b| catalog::body_name(b)).collect::<Vec<_>>(),
        "lengthMm": dims.length,
        "widthMm": dims.width,
        "heightMm": dims.height,
        "wheelbaseMm": dims.wheelbase,
        "trackMm": dims.track,
        "wheelMm": dims.wheel,
        "measured": source != anatomy::DimensionSource::Typical,
        "dimensionsText": anatomy::dimension_source_text(source),
        "powertrain": variant,
        "powertrainChoices": variants
            .iter()
            .map(|&flag| json!({ "flag": flag, "name": catalog::powertrain_text(flag) }))
            .collect::<Vec<_>>(),
        "engine": drivetrain.engine as i32,
        "engineName": if motorcycle && drivetrain.engine != anatomy::EngineLayout::None {
            anatomy::motorcycle_engine_name()
        } else {
            anatomy::engine_layout_name(drivetrain.engine)
        },
        "driven": drivetrain.driven as i32,
        "drivenName": anatomy::driven_axles_name(drivetrain.driven),
        "frontMotor": drivetrain.front_motor,
        "rearMotor": drivetrain.rear_motor,
        "battery": drivetrain.battery as i32,
        "fuelTank": drivetrain.fuel_tank,
        "reason": if motorcycle {
            anatomy::motorcycle_reason()
        } else {
            anatomy::drivetrain_reason(&drivetrain)
        },
        "socketLocation": socket_location,
        "socketSpot": socket_spot,
    })
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
    use caruspex_vehicles::catalog::SocketSpot;

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
        let golf = vehicle_anatomy(first["id"].as_i64().unwrap(), 2001, 0, "", "", 0);
        assert_eq!(golf["engine"], 1);
        assert!(golf["wheelbaseMm"].as_u64().unwrap() < golf["lengthMm"].as_u64().unwrap());
        // The Golf IV's dimensions come from Wikidata (Q1419308).
        assert_eq!(golf["dimensionSource"], "wikidata");
        assert_eq!(golf["lengthMm"], 4148);
        assert_eq!(golf["wikidataId"], "Q1419308");
        assert_eq!(golf["generation"], "IV");
        assert_eq!(
            golf["bodyNames"].as_array().unwrap().len(),
            BodyType::ALL.len()
        );
        // The Golf IV has its socket in the centre console in front of the
        // gear lever (data/obd_sockets.csv).
        assert_eq!(golf["socketSpot"], SocketSpot::ConsoleFront as i32);
        // A motorcycle: engine in the frame, plug under the seat.
        let mt07 = catalog_search("MT-07", 5);
        let mt07 = vehicle_anatomy(mt07[0]["id"].as_i64().unwrap(), 2020, 0, "", "", 0);
        assert_eq!(mt07["engine"], anatomy::EngineLayout::Mid as i32);
        assert_eq!(mt07["socketSpot"], SocketSpot::UnderSeat as i32);
        assert_eq!(mt07["reason"], anatomy::motorcycle_reason());
        // The demo car: an estate with a petrol engine, no generation.
        let demo = vehicle_anatomy(-1, 2021, 2, "", "", 1);
        assert_eq!(demo["powertrain"], 1);
        assert_eq!(demo["socketSpot"], 0);
        assert!(demo["fuelTank"].as_bool().unwrap());
        let vin = decode_vin("WVWZZZ1JZ3W000001");
        assert_eq!(vin["wmi"], "WVW");
        assert!(match_vin("WVWZZZ1JZ3W000001")["brand"].as_str().is_some());
        assert_eq!(kkl_ecus()[0]["address"], 1);
        assert!(!ecu_families().as_array().unwrap().is_empty());
    }

    #[test]
    fn obdb_packs_come_for_the_detected_car() {
        let ioniq = catalog_search("ioniq 5", 5);
        let id = ioniq[0]["id"].as_u64().unwrap() as usize;
        let packs = obdb_packs(Some(id), 2023);
        let list = packs.as_array().unwrap();
        assert!(!list.is_empty());
        let pack = &list[0];
        assert_eq!(pack["obdb"], true);
        assert_eq!(pack["brand"], "Hyundai");
        assert_eq!(pack["category"], PidCategory::HybridEv as i32);
        assert!(pack["title"].as_str().unwrap().contains("OBDb"));
        let values = pack["values"].as_array().unwrap();
        assert!(values.len() > 50);
        assert!(
            values[0]["source"]
                .as_str()
                .unwrap()
                .contains("CC BY-SA 4.0")
        );
        // No car, a car without OBDb data, a year outside the packs.
        assert!(obdb_packs(None, 2023).as_array().unwrap().is_empty());
        assert!(obdb_packs(Some(id), 1990).as_array().unwrap().is_empty());
    }

    #[test]
    fn opendbc_packs_come_with_their_car_only() {
        let general = pid_packs();
        assert!(
            general
                .as_array()
                .unwrap()
                .iter()
                .all(|p| !p["id"].as_str().unwrap().starts_with("opendbc-"))
        );
        let golf = catalog_search("golf 7", 5);
        let id = golf[0]["id"].as_u64().unwrap() as usize;
        for year in [2017, 0] {
            let packs = obdb_packs(Some(id), year);
            let pack = packs
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["id"] == "opendbc-vw-mqb")
                .expect("MQB pack for the Golf 7")
                .clone();
            assert_eq!(pack["listen"], true);
            assert!(pack["description"].as_str().unwrap().contains("opendbc"));
            assert!(!pack["values"].as_array().unwrap().is_empty());
        }
    }
}
