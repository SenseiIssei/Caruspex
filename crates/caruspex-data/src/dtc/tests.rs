//! Tests for the fault code database: decoding, generic and brand meanings,
//! severity, plain-language help and both languages.

use std::collections::HashSet;

use caruspex_i18n::{Language, languages, translate_core};

use super::*;
use crate::test_support::{english, german, with};

/// No long dashes anywhere in user-visible text, only the plain " - ".
fn has_long_dash(text: &str) -> bool {
    text.contains('\u{2014}') || text.contains('\u{2013}')
}

#[test]
fn trouble_code_bytes_decode_to_the_five_character_form() {
    assert_eq!(decode_dtc(0x01, 0x71), "P0171");
    assert_eq!(decode_dtc(0x0A, 0x80), "P0A80");
    assert_eq!(decode_dtc(0x11, 0x28), "P1128");
    assert_eq!(decode_dtc(0x40, 0x35), "C0035");
    assert_eq!(decode_dtc(0x80, 0x01), "B0001");
    assert_eq!(decode_dtc(0xC1, 0x21), "U0121");
    assert!(decode_dtc(0x00, 0x00).is_empty());
    // From the parser tests.
    assert_eq!(decode_dtc(0x01, 0x33), "P0133");
    assert_eq!(decode_dtc(0x43, 0x21), "C0321");
    assert_eq!(decode_dtc(0x81, 0x23), "B0123");
    assert_eq!(decode_dtc(0xC1, 0x00), "U0100");
    assert_eq!(decode_dtc(0x05, 0x07), "P0507");
}

#[test]
fn generic_codes_have_meaning_area_severity_and_help() {
    let _lang = german();
    let misfire = lookup_dtc("P0301", "");
    assert!(misfire.known);
    assert!(!misfire.manufacturer_specific);
    assert!(misfire.family.is_empty());
    assert!(misfire.text.contains("Zylinder 1"));
    assert_eq!(misfire.system, DtcSystem::Ignition);
    assert!(misfire.has_details);
    assert!(misfire.causes.len() >= 3);
    assert!(!misfire.checks.is_empty());
    assert!(misfire.advice.contains("Katalysator"));
    assert!(!misfire.system_name.is_empty());

    let cat = lookup_dtc("P0420", "");
    assert_eq!(cat.system, DtcSystem::Exhaust);
    assert_eq!(cat.cost_class, CostClass::Expensive);
    assert_eq!(cost_class_label(cat.cost_class), "teuer");

    assert_eq!(lookup_dtc("U0100", "").system, DtcSystem::Network);
    assert_eq!(lookup_dtc("C0035", "").system, DtcSystem::Brakes);
    assert_eq!(lookup_dtc("B0001", "").system, DtcSystem::Airbag);
    assert_eq!(
        lookup_dtc("P2463", "").system,
        DtcSystem::DieselAftertreatment
    );
    assert!(lookup_dtc("P20EE", "").known);
    assert_eq!(lookup_dtc("P0A80", "").system, DtcSystem::Hybrid);
    assert!(lookup_dtc("P3425", "").known);

    // Input is tolerant about case and spaces.
    assert_eq!(lookup_dtc(" p0301 ", "").code, "P0301");
    assert!(lookup_dtc(" p0301 ", "").known);
}

#[test]
fn code_families_count_in_decimal_like_the_standard() {
    let _lang = german();
    // Cylinder 10 is P0210 and P0310, not P020A or P030A.
    assert!(lookup_dtc("P0210", "").text.contains("Zylinder 10"));
    assert!(lookup_dtc("P0310", "").text.contains("Zylinder 10"));
    // P020A and P030A exist, but with a meaning of their own (cylinder 1
    // injection timing, ignition A control signal), not cylinder 10.
    assert!(!lookup_dtc("P020A", "").text.contains("Zylinder 10"));
    assert!(!lookup_dtc("P030A", "").text.contains("Zylinder 10"));
    // Bank 1 sensor 2 runs from P0136 to P0141.
    assert!(lookup_dtc("P0140", "").text.contains("Bank 1, Sonde 2"));
    // P0736 is reverse, P0775 a pressure control solenoid.
    assert!(lookup_dtc("P0736", "").text.contains("Rückwärtsgang"));
    assert!(lookup_dtc("P0775", "").text.contains("Druckregelventil B"));
    // ABS loses communication as U0121, its invalid data is U0415.
    assert!(lookup_dtc("U0415", "").text.contains("ABS"));
    // Ignition coil L sits at the end of the P23xx block.
    assert!(lookup_dtc("P2335", "").text.contains("Zündspule L"));
}

#[test]
fn severity_follows_the_risk_of_driving_on() {
    let _lang = german();
    // Whether the check engine light flashes is not known to us, so a
    // misfire is at least High.
    assert!(lookup_dtc("P0300", "").severity >= Severity::High);
    for cyl in 1..=12 {
        let code = format!("P03{}{cyl}", if cyl < 10 { "0" } else { "" });
        assert!(lookup_dtc(&code, "").severity >= Severity::High, "{code}");
    }
    assert_eq!(lookup_dtc("P0420", "").severity, Severity::Medium);
    assert_eq!(lookup_dtc("P0171", "").severity, Severity::Medium);
    assert_eq!(lookup_dtc("P0442", "").severity, Severity::Low);
    assert_eq!(lookup_dtc("P0456", "").severity, Severity::Low);
    assert_eq!(lookup_dtc("P0128", "").severity, Severity::Low);
    assert_eq!(lookup_dtc("P0217", "").severity, Severity::Critical);
    assert_eq!(lookup_dtc("P0524", "").severity, Severity::Critical);
    assert_eq!(lookup_dtc("P0AA6", "").severity, Severity::Critical);
    assert_eq!(lookup_dtc("C0035", "").severity, Severity::High);
    assert_eq!(lookup_dtc("B0001", "").severity, Severity::High);
    assert_eq!(lookup_dtc("U0121", "").severity, Severity::High);
    assert_eq!(lookup_dtc("U0184", "").severity, Severity::Low);
    assert_eq!(lookup_dtc("P1000", "Ford").severity, Severity::Info);
}

#[test]
fn brand_families_are_found_from_plain_brand_names() {
    let _lang = german();
    assert_eq!(brand_family("Audi"), "VAG");
    assert_eq!(brand_family("Škoda"), "VAG");
    assert_eq!(brand_family("SKODA"), "VAG");
    assert_eq!(brand_family("VW Nutzfahrzeuge"), "VAG");
    assert_eq!(brand_family("Mercedes-Benz"), "MERCEDES");
    assert_eq!(brand_family("mercedes benz"), "MERCEDES");
    assert_eq!(brand_family("Lexus"), "TOYOTA");
    assert_eq!(brand_family("Kia"), "HYUNDAI");
    assert_eq!(brand_family("Citroën"), "PSA");
    assert_eq!(brand_family("Mini Cooper"), "BMW");
    assert_eq!(brand_family("Opel"), "OPEL");
    assert_eq!(brand_family("PSA"), "PSA");
    assert!(brand_family("Tesla").is_empty());
    assert!(brand_family("").is_empty());
    assert_eq!(brand_family_label("HYUNDAI"), "Hyundai/Kia");
    assert!(brand_families().len() >= 15);
}

#[test]
fn the_same_p1_code_means_different_things_per_brand() {
    let _lang = german();
    let vag = lookup_dtc("P1128", "Volkswagen");
    let honda = lookup_dtc("P1128", "Honda");
    let toyota = lookup_dtc("P1128", "Toyota");
    let generic = lookup_dtc("P1128", "");

    assert!(vag.known);
    assert_eq!(vag.family, "VAG");
    assert!(vag.manufacturer_specific);
    assert!(vag.text.contains("mager"));
    assert!(honda.known);
    assert_eq!(honda.family, "HONDA");
    assert!(honda.text.contains("Saugrohrdruck"));
    assert!(toyota.known);
    assert!(toyota.text.contains("Drosselklappe"));
    assert!(toyota.throttle_related);

    assert_ne!(vag.text, honda.text);
    assert_ne!(honda.text, toyota.text);
    // Without a brand the code is honestly unknown.
    assert!(!generic.known);
    assert!(generic.manufacturer_specific);
    assert_ne!(generic.text, vag.text);

    assert_eq!(describe_dtc_for_brand("P1128", "Honda"), honda.text);
}

#[test]
fn brand_tables_never_hide_the_generic_meaning() {
    let _lang = german();
    let info = lookup_dtc("P0301", "Ford");
    assert!(info.known);
    assert!(info.family.is_empty());
    assert_eq!(info.text, lookup_dtc("P0301", "").text);
}

#[test]
fn mazda_only_takes_over_the_ford_codes_it_really_shares() {
    let _lang = german();
    assert!(lookup_dtc("P1000", "Mazda").known);
    assert!(lookup_dtc("P1131", "Ford").known);
    assert!(!lookup_dtc("P1131", "Mazda").known);
}

#[test]
fn unknown_codes_fall_back_to_their_group() {
    let _lang = german();
    let renault = lookup_dtc("P1999", "Renault");
    assert!(!renault.known);
    assert!(renault.manufacturer_specific);
    assert!(renault.text.contains("herstellerspezifisch, Renault/Dacia"));
    assert!(renault.text.contains("kein Klartext"));
    assert!(!renault.advice.is_empty());

    let gearbox = lookup_dtc("P09FE", "");
    assert!(!gearbox.known);
    assert_eq!(gearbox.system, DtcSystem::Transmission);
    assert!(gearbox.text.contains("Getriebe"));

    // An unknown airbag code still gets the airbag help.
    let airbag = lookup_dtc("B00FE", "");
    assert!(!airbag.known);
    assert_eq!(airbag.system, DtcSystem::Airbag);
    assert!(airbag.has_details);
    assert_eq!(airbag.severity, Severity::High);

    let lost_comm = lookup_dtc("U02FE", "");
    assert!(!lost_comm.known);
    assert_eq!(lost_comm.system, DtcSystem::Network);
    assert!(lost_comm.has_details);

    assert_eq!(lookup_dtc("XYZ", "").text, "unbekannter Code");
    assert!(!lookup_dtc("P4000", "").known);
    assert_eq!(describe_dtc("12345"), "unbekannter Code");
}

#[test]
fn the_old_lookups_keep_working() {
    let _lang = german();
    assert!(dtc_descriptions().len() > 1000);
    // describe_dtc without a brand still knows the VAG codes.
    assert!(describe_dtc("P1559").contains("Drosselklappe"));
    assert!(describe_dtc("P0171").contains("mager"));
    assert!(is_throttle_dtc("P0507"));
    assert!(is_throttle_dtc("P1559"));
    assert!(is_throttle_dtc("P2135"));
    assert!(!is_throttle_dtc("P0301"));
    assert!(dtc_texts().contains_key("P0300"));
    assert!(dtc_throttle_codes().contains("P2101"));
}

#[test]
fn the_fault_database_knows_the_important_codes() {
    let _lang = german();
    assert!(dtc_descriptions().len() > 300);
    assert!(!describe_dtc("P0301").is_empty());
    assert!(!describe_dtc("P1559").is_empty());
    assert!(!describe_dtc("U0121").is_empty());
    // Unknown codes fall back to the group meaning instead of staying empty.
    assert!(!describe_dtc("P0999").is_empty());
    assert!(is_throttle_dtc("P0507"));
    assert!(!is_throttle_dtc("P0301"));
}

#[test]
fn the_lexicon_lists_generic_and_brand_codes() {
    let _lang = german();
    let all = dtc_lexicon();
    let brands: usize = brand_families().iter().map(|f| dtc_count(f)).sum();
    assert_eq!(all.len(), dtc_count("") + brands);
    assert!(dtc_count("") > 1300);

    let mut generic_found = false;
    let mut vag_found = false;
    let mut honda_found = false;
    for e in &all {
        assert!(!e.text.is_empty());
        assert!(!e.family_label.is_empty());
        if e.code == "P0301" && e.family.is_empty() {
            generic_found = true;
        }
        if e.code == "P1559" && e.family == "VAG" {
            vag_found = true;
        }
        if e.code == "P1128" && e.family == "HONDA" {
            honda_found = true;
        }
    }
    assert!(generic_found);
    assert!(vag_found);
    assert!(honda_found);
}

#[test]
fn common_codes_come_with_plain_language_help() {
    let _lang = german();
    let with_details = dtc_lexicon()
        .iter()
        .filter(|e| e.family.is_empty() && lookup_dtc(&e.code, "").has_details)
        .count();
    assert!(with_details >= 250);

    for code in [
        "P0171", "P0420", "P0442", "P0101", "P0128", "P0016", "P0401", "P2463", "P0380", "P20EE",
        "P0562", "U0100", "C0035", "B0001", "P0299",
    ] {
        let info = lookup_dtc(code, "");
        assert!(info.has_details, "{code}");
        assert!(!info.advice.is_empty(), "{code}");
        assert!(!info.causes.is_empty(), "{code}");
        assert!(!info.checks.is_empty(), "{code}");
    }
}

#[test]
fn fault_texts_follow_the_active_language() {
    let lang = german();
    assert!(lookup_dtc("P0217", "").text.contains("überhitzt"));
    lang.set(Language::English);
    let cat = lookup_dtc("P0420", "");
    assert!(cat.text.contains("Catalyst"));
    assert!(cat.advice.contains("emissions test"));
    assert_eq!(cost_class_label(cat.cost_class), "expensive");
    assert_eq!(
        cat.system_name,
        "Exhaust: oxygen sensors and catalytic converter"
    );
    assert!(
        lookup_dtc("P1999", "Renault")
            .text
            .contains("manufacturer-specific, Renault/Dacia")
    );
    assert!(describe_dtc_for_brand("P1128", "Honda").contains("MAP sensor"));
    assert_eq!(severity_label_of("P0300"), "High");
    assert_eq!(brand_family_label(""), "SAE/ISO (all brands)");
    // Back in German the same lookups read German again.
    lang.set(Language::German);
    assert!(lookup_dtc("P0420", "").text.contains("Katalysator"));
    assert!(describe_dtc("P0171").contains("mager"));
}

fn severity_label_of(code: &str) -> &'static str {
    crate::severity::severity_label(lookup_dtc(code, "").severity)
}

#[test]
fn english_texts_for_the_data_tables() {
    let _lang = english();
    assert!(describe_dtc("P0171").contains("lean"));
}

#[test]
fn no_long_dashes_and_no_empty_texts_in_either_language() {
    for lang in [Language::German, Language::English] {
        let _guard = with(lang);
        let mut seen = HashSet::new();
        for e in dtc_lexicon() {
            let what = format!("{} {}", e.code, e.family);
            assert!(!has_long_dash(&e.text), "{what}");
            if !seen.insert(format!("{}{}", e.code, e.family)) {
                continue;
            }
            let info = lookup_dtc(&e.code, &e.family);
            assert!(!info.advice.is_empty(), "{what}");
            assert!(!has_long_dash(&info.advice), "{what}");
            for line in &info.causes {
                assert!(!has_long_dash(line), "{what}");
            }
            for line in &info.checks {
                assert!(!has_long_dash(line), "{what}");
            }
        }
    }
}

#[test]
fn every_catalog_translates_the_core_texts() {
    let guard = english();
    let english_text = lookup_dtc("P0171", "").text;
    assert!(!english_text.is_empty());
    for info in languages() {
        if info.id == Language::English {
            continue;
        }
        guard.set(info.id);
        let text = lookup_dtc("P0171", "").text;
        assert!(!text.is_empty(), "{}", info.code);
        assert_ne!(text, english_text, "{}", info.code);
        assert_ne!(l("Motor", "Engine"), "Engine", "{}", info.code);
    }
    // Unknown texts fall back to English, never to nothing.
    assert_eq!(
        translate_core(Language::French, "no such text here"),
        "no such text here"
    );
}

#[test]
fn the_generated_tables_are_sorted_and_free_of_duplicates() {
    // The lookup is a binary search, so a table out of order would lose codes.
    let mut tables: Vec<(&str, &[Record])> = vec![
        ("P", tables::generic_powertrain()),
        ("C", tables::generic_chassis()),
        ("B", tables::generic_body()),
        ("U", tables::generic_network()),
    ];
    for f in tables::families() {
        tables.push((f.key, f.records));
    }
    for (name, table) in tables {
        for pair in table.windows(2) {
            assert!(
                pair[0].code < pair[1].code,
                "{name}: {} before {}",
                pair[0].code,
                pair[1].code
            );
        }
        for r in table {
            assert!(r.detail == NO_DETAIL || (r.detail as usize) < tables::details().len());
        }
    }
}

#[test]
fn composed_codes_read_like_written_ones() {
    let lang = german();
    // Part with a letter, then the fault type.
    let hv = lookup_dtc("P0ABC", "");
    assert!(hv.known);
    assert_eq!(hv.system, DtcSystem::Hybrid);
    assert!(hv.text.starts_with("Hybridbatterie"), "{}", hv.text);
    assert!(
        hv.text.contains(" A ") && hv.text.ends_with(" - Stromkreis Spannung zu niedrig"),
        "{}",
        hv.text
    );
    // Lost communication leads with the fault type.
    let comm = lookup_dtc("U0201", "");
    assert!(comm.known);
    assert!(comm.text.starts_with("Keine Verbindung: "), "{}", comm.text);
    assert!(comm.has_details);
    lang.set(Language::English);
    assert_eq!(
        lookup_dtc("P0ABC", "").text,
        "Hybrid/EV battery pack voltage sense A - circuit low"
    );
    assert_eq!(
        lookup_dtc("U0201", "").text,
        "Lost communication: Door control module C"
    );
    // Two arguments, bank and sensor.
    assert_eq!(
        lookup_dtc("P2082", "").text,
        "Exhaust gas temperature sensor bank 2 sensor 1 - circuit range/performance"
    );
    // Every other language translates the phrases instead of falling back to English.
    for info in languages() {
        if matches!(info.id, Language::English | Language::German) {
            continue;
        }
        lang.set(info.id);
        let text = lookup_dtc("U0201", "").text;
        assert!(
            !text.contains("Lost communication"),
            "{}: {text}",
            info.code
        );
        assert!(
            !lookup_dtc("P0ABC", "").text.contains("circuit low"),
            "{}",
            info.code
        );
    }
}

#[test]
fn every_composed_text_is_complete_in_every_language() {
    let lang = english();
    for info in languages() {
        lang.set(info.id);
        for letter in SYSTEM_LETTERS {
            for r in generic_for(letter) {
                let text = r.text();
                assert!(!text.is_empty(), "{} {}", info.code, r.code);
                assert!(!text.contains('%'), "{} {}: {text}", info.code, r.code);
                assert!(!has_long_dash(&text), "{} {}: {text}", info.code, r.code);
            }
        }
    }
}

#[test]
fn the_hybrid_network_body_and_chassis_blocks_are_filled() {
    let count = |letter: char, lo: i32, hi: i32| {
        generic_for(letter)
            .iter()
            .filter(|r| (lo..=hi).contains(&code_number(r.code)))
            .count()
    };
    assert!(
        count('P', 0x0A00, 0x0FFF) > 400,
        "hybrid and electric P0A00 to P0FFF"
    );
    assert!(count('P', 0x2A00, 0x2FFF) > 100, "P2A00 to P2FFF");
    assert!(count('U', 0x0000, 0x0FFF) > 200, "network U0");
    assert!(count('U', 0x3000, 0x3FFF) > 15, "network U3");
    assert!(count('C', 0x0000, 0x0FFF) > 100, "chassis C0");
    assert!(count('B', 0x0000, 0x0FFF) > 15, "body B0");
}

#[test]
fn unknown_codes_take_the_area_of_their_neighbours() {
    let _lang = german();
    // P013x is oxygen sensor bank 1 sensor 1 throughout; a code the table
    // does not list still names the exhaust area.
    let rule = find_rule("P013F").expect("rule");
    assert_eq!(rule.system, DtcSystem::Exhaust);
    assert_eq!(rule.hi - rule.lo, 0xF);
    // Wider ranges still answer where no block rule exists.
    let u06 = lookup_dtc("U06FE", "");
    assert!(!u06.known);
    assert_eq!(u06.system, DtcSystem::Network);
    assert!(u06.text.contains("Sensor oder Steller"), "{}", u06.text);
    assert!(u06.has_details);
    let hv = lookup_dtc("U3FFE", "");
    assert_eq!(hv.system, DtcSystem::Hybrid);
    assert_eq!(hv.severity, Severity::High);
    let power = lookup_dtc("U34FE", "");
    assert!(power.text.contains("Versorgung"), "{}", power.text);
    // Manufacturer ranges keep the wide group rules only.
    let maker = find_rule("P113F").expect("rule");
    assert!(maker.hi - maker.lo > 0xF);
    // Every code of every letter finds some group.
    for letter in SYSTEM_LETTERS {
        for second in 0..4 {
            for n in (0..0x1000).step_by(0x37) {
                let code = format!("{letter}{second}{n:03X}");
                assert!(find_rule(&code).is_some(), "{code}");
            }
        }
    }
}

#[test]
fn single_source_codes_say_so_in_every_language() {
    let lang = german();
    // Two sources agree on P0ABC: no note.
    let confirmed = lookup_dtc("P0ABC", "");
    assert!(!confirmed.single_source);
    assert_eq!(confirmed.source_note(), "");
    // Only obdex defines U0177 (lost communication with restraints sensor H).
    let single = lookup_dtc("U0177", "");
    assert!(single.known && single.single_source);
    assert_eq!(single.source_note(), "Bedeutung aus einer Quelle");
    assert!(single.text.contains(" H"), "{}", single.text);
    lang.set(Language::English);
    assert_eq!(
        lookup_dtc("U0177", "").source_note(),
        "Meaning from one source"
    );
    for info in languages() {
        if matches!(info.id, Language::English | Language::German) {
            continue;
        }
        lang.set(info.id);
        let note = lookup_dtc("U0177", "").source_note();
        assert!(
            !note.is_empty() && note != "Meaning from one source",
            "{}",
            info.code
        );
    }
    // Written-out codes and unknown codes carry no note.
    assert_eq!(lookup_dtc("P0300", "").source_note(), "");
    assert_eq!(lookup_dtc("P09FE", "").source_note(), "");
    // The lexicon carries the flag too.
    let lexicon = dtc_lexicon();
    assert!(lexicon.iter().any(|e| e.code == "U0177" && e.single_source));
    assert!(
        lexicon
            .iter()
            .any(|e| e.code == "P0ABC" && !e.single_source)
    );
}

#[test]
fn both_tiers_fill_every_sae_block() {
    let (mut confirmed, mut single) = (0, 0);
    for letter in SYSTEM_LETTERS {
        for r in generic_for(letter) {
            if r.single_source {
                single += 1;
            } else {
                confirmed += 1;
            }
        }
    }
    assert!(confirmed > 3000, "{confirmed}");
    assert!(single > 5000, "{single}");
    assert!(dtc_count("") > 9000);
}

#[test]
fn generic_codes_from_phrases_read_in_the_chosen_language() {
    // The interface of the app carries no fault code texts; this table is
    // their only source, so a code put together from phrases (B0003) and a
    // written out one (P0420) must come translated from here.
    let lang = english();
    let composed_en = lookup_dtc("B0003", "").text;
    let written_en = lookup_dtc("P0420", "").text;
    assert!(!composed_en.contains('%'), "{composed_en}");
    for code in ["fr", "hi", "ja"] {
        lang.set(caruspex_i18n::language_from_code(code));
        let composed = lookup_dtc("B0003", "").text;
        assert_ne!(composed, composed_en, "{code}");
        assert!(!composed.contains('%'), "{code}: {composed}");
        assert_ne!(lookup_dtc("P0420", "").text, written_en, "{code}");
    }
}
