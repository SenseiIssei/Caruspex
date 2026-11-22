//! read, vin and freeze: faults, readiness, the VIN with what it says, and
//! the freeze frame.

use caruspex_link::Elm327;
use caruspex_protocols::Obd2;
use caruspex_record::JsonWriter;
use caruspex_vehicles::catalog;

use super::{
    fault_text, json_adapter, json_faults, json_status, json_value, number_or_null, print_value,
    read_vin_context,
};
use crate::out::{ljust, say};

pub fn read(elm: &mut Elm327, as_json: bool) -> i32 {
    let mut obd = Obd2::new(elm);
    let status = obd.read_status();
    let (faults, err) = match obd.read_faults() {
        Ok(faults) => (faults, String::new()),
        Err(err) => (Vec::new(), err),
    };
    let vin = read_vin_context(&mut obd);

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object();
        json_adapter(&mut json, obd.elm());
        json.key("vin");
        if vin.vin.is_empty() {
            json.null();
        } else {
            json.value(&vin.vin);
        }
        json.field("brand", &vin.brand);
        json_status(&mut json, &status);
        json_faults(&mut json, &faults, &vin.brand);
        if !err.is_empty() {
            json.field("error", &err);
        }
        json.end_object();
        say(&json.text());
        return 0;
    }

    if !vin.vin.is_empty() {
        let brand = if vin.brand.is_empty() {
            String::new()
        } else {
            format!("  ({})", vin.brand)
        };
        say(&format!("VIN: {}{}\n", vin.vin, brand));
    }
    if status.valid {
        say(&format!(
            "MIL: {}   reported faults: {}\n",
            if status.mil_on { "ON" } else { "off" },
            status.fault_count
        ));
        for r in &status.readiness {
            say(&format!(
                "   {} {}\n",
                ljust(&r.name, 26),
                if r.ready { "ready" } else { "not ready" }
            ));
        }
    }
    say("\n");
    if faults.is_empty() {
        say("No faults stored.\n");
    } else {
        say(&format!("{} fault(s) found:\n", faults.len()));
        for f in &faults {
            say(&format!(
                "  {} [{}]{}{}\n      {}\n",
                f.code,
                f.kind,
                if f.ecu.is_empty() { "" } else { " " },
                f.ecu,
                fault_text(&f.code, &vin.brand)
            ));
        }
    }
    if !err.is_empty() {
        say(&format!("{err}\n"));
    }
    0
}

pub fn vin(elm: &mut Elm327, as_json: bool) -> i32 {
    let mut obd = Obd2::new(elm);
    let c = read_vin_context(&mut obd);
    let info = &c.info;
    let matched = &c.matched;

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object();
        json_adapter(&mut json, obd.elm());
        json.key("vin");
        if c.vin.is_empty() {
            json.null();
            json.field(
                "error",
                "The car does not report a VIN (mode 09, InfoType 02).",
            );
            json.end_object();
            say(&json.text());
            return 4;
        }
        json.value(&c.vin)
            .field("valid", &info.valid)
            .field("wmi", &info.wmi)
            .field("manufacturer", &info.manufacturer)
            .field("brand", &info.brand)
            .string_list("brandCandidates", &info.brand_candidates)
            .field("country", &info.country)
            .field("region", &info.region)
            .key("modelYear");
        number_or_null(&mut json, info.model_year > 0, info.model_year);
        json.field("modelYearCertain", &info.model_year_certain)
            .key("modelYears")
            .begin_array();
        for year in &info.model_years {
            json.value(year);
        }
        json.end_array()
            .field("checkDigitApplies", &info.check_digit_applies)
            .field("checkDigitOk", &info.check_digit_ok)
            .field("serial", &info.serial)
            .field("modelCode", &info.model_code)
            .string_list("problems", &info.problems)
            .string_list("notes", &info.notes)
            .key("catalog")
            .begin_object()
            .field("brand", &matched.brand)
            .field("confidence", &matched.confidence)
            .field("reason", &matched.reason)
            .key("year");
        number_or_null(&mut json, matched.year > 0, matched.year);
        json.key("generation");
        if let Some(g) = matched.generation {
            json.begin_object()
                .field("name", &catalog::display_name(g))
                .field("brand", g.brand)
                .field("model", g.model)
                .field("generation", g.generation)
                .field("yearFrom", &g.year_from)
                .key("yearTo");
            number_or_null(&mut json, g.year_to > 0, g.year_to);
            json.field("diagnosis", &catalog::diagnosis_text(g, matched.year))
                .end_object();
        } else {
            json.null();
        }
        json.key("alternatives").begin_array();
        for g in &matched.alternatives {
            json.value(&catalog::display_name(g));
        }
        json.end_array().end_object().end_object();
        say(&json.text());
        return 0;
    }

    if c.vin.is_empty() {
        say("The car does not report a VIN (mode 09, InfoType 02).\n");
        return 4;
    }
    say(&format!(
        "VIN          : {}{}\n",
        c.vin,
        if info.valid {
            ""
        } else {
            "  (not a valid VIN)"
        }
    ));
    say(&format!(
        "Manufacturer : {}\n",
        if info.manufacturer.is_empty() {
            "unknown"
        } else {
            &info.manufacturer
        }
    ));
    if !info.country.is_empty() {
        say(&format!(
            "Country      : {}, {}\n",
            info.country, info.region
        ));
    }
    if info.model_year > 0 {
        say(&format!(
            "Model year   : {}{}\n",
            info.model_year,
            if info.model_year_certain {
                ""
            } else {
                " (not certain)"
            }
        ));
    }
    if info.check_digit_applies {
        say(&format!(
            "Check digit  : {}\n",
            if info.check_digit_ok {
                "correct"
            } else {
                "WRONG"
            }
        ));
    }
    if !info.serial.is_empty() {
        say(&format!("Serial       : {}\n", info.serial));
    }
    for p in &info.problems {
        say(&format!("Problem      : {p}\n"));
    }
    for n in &info.notes {
        say(&format!("Note         : {n}\n"));
    }
    say("\n");
    if let Some(g) = matched.generation {
        say(&format!(
            "Catalog      : {}, {} % sure\n",
            catalog::display_name(g),
            matched.confidence
        ));
        if !matched.reason.is_empty() {
            say(&format!("               {}\n", matched.reason));
        }
        say(&format!(
            "Diagnosis    : {}\n",
            catalog::diagnosis_text(g, matched.year)
        ));
        for alt in &matched.alternatives {
            say(&format!("Also possible: {}\n", catalog::display_name(alt)));
        }
    } else if !matched.brand.is_empty() {
        say(&format!(
            "Catalog      : {}, model not recognised\n",
            matched.brand
        ));
    } else {
        say("Catalog      : no match\n");
    }
    0
}

pub fn freeze(elm: &mut Elm327, as_json: bool) -> i32 {
    let mut obd = Obd2::new(elm);
    let frames = obd.read_freeze_frames(caruspex_protocols::obd2::DEFAULT_FREEZE_FRAMES);
    let brand = if frames.is_empty() {
        String::new()
    } else {
        read_vin_context(&mut obd).brand
    };

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object();
        json_adapter(&mut json, obd.elm());
        json.field("brand", &brand).key("frames").begin_array();
        for f in &frames {
            json.begin_object()
                .field("frame", &f.frame)
                .field("dtc", &f.dtc)
                .field("text", &fault_text(&f.dtc, &brand));
            if !f.ecu_source.is_empty() {
                json.field("ecu", &f.ecu).field("ecuSource", &f.ecu_source);
            }
            json.key("values").begin_array();
            for v in &f.values {
                json_value(&mut json, v);
            }
            json.end_array().end_object();
        }
        json.end_array().end_object();
        say(&json.text());
        return 0;
    }

    if frames.is_empty() {
        say("No freeze frame stored.\n");
        return 0;
    }
    for f in &frames {
        say(&format!(
            "Freeze frame {}: {} {}{}{}\n",
            f.frame,
            f.dtc,
            fault_text(&f.dtc, &brand),
            if f.ecu.is_empty() { "" } else { "  " },
            f.ecu
        ));
        for v in &f.values {
            print_value(v);
        }
        say("\n");
    }
    0
}
