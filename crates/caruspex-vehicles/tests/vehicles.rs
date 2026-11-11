//! Tests for the VIN decoder and the vehicle catalog (port of tests/test_vehicles.cpp).
//!
//! The active language is process wide, so every test takes the lock and
//! starts in German, like the C++ tests do.

use caruspex_i18n::{Language, set_language};
use caruspex_vehicles::catalog::{
    self, BodyType, Connector, ObdSupport, PT_DIESEL, PT_ELECTRIC, PT_PETROL, SocketLocation,
    VehicleGeneration, body_name, connector_name, location_text, location_text_for,
    powertrain_text, search_catalog, support_name,
};
use caruspex_vehicles::vehicles::BusKind;
use caruspex_vehicles::vin::{
    VinModelCodeRule, decode_vin, find_wmi, vin_base_year, vin_check_digit, vin_country,
    vin_region, wmi_table,
};
use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard};

static LANGUAGE_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    let guard = LANGUAGE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    set_language(Language::German);
    guard
}

/// Writes the correct check digit into position 9, for building North
/// American test VINs.
fn with_check_digit(vin: &str) -> String {
    let mut bytes = vin.as_bytes().to_vec();
    bytes[8] = b'0';
    let digit = vin_check_digit(std::str::from_utf8(&bytes).unwrap());
    bytes[8] = digit as u8;
    String::from_utf8(bytes).unwrap()
}

fn has_long_dash(text: &str) -> bool {
    text.contains('\u{2014}')
}

#[test]
fn the_classic_vin_passes_the_check_digit() {
    let _g = lock();
    assert_eq!(vin_check_digit("1M8GDM9AXKP042788"), 'X');

    let v = decode_vin("1M8GDM9AXKP042788", 2026);
    assert!(v.valid);
    assert!(v.problems.is_empty());
    assert_eq!(v.wmi, "1M8");
    assert_eq!(v.manufacturer, "Motor Coach Industries");
    assert!(v.north_america);
    assert!(v.check_digit_applies);
    assert!(v.check_digit_ok);
    assert_eq!(v.expected_check_digit, 'X');
    // Digit at position 7: first cycle, K = 1989.
    assert_eq!(v.model_year, 1989);
    assert_eq!(v.model_years.len(), 1);
    assert!(v.model_year_certain);
    assert_eq!(v.plant, 'P');
    assert_eq!(v.serial, "042788");
}

#[test]
fn a_wrong_check_digit_is_reported() {
    let _g = lock();
    let v = decode_vin("1M8GDM9A1KP042788", 2026);
    assert!(v.check_digit_applies);
    assert!(!v.check_digit_ok);
    assert!(!v.problems.is_empty());
}

#[test]
fn input_is_normalised_and_checked() {
    let _g = lock();
    let spaced = decode_vin(" wvw zzz-1jz2w123456 ", 2026);
    assert_eq!(spaced.vin, "WVWZZZ1JZ2W123456");
    assert!(spaced.valid);

    let ioq = decode_vin("WVWZZZ1JZ2W12345O", 2026);
    assert!(!ioq.valid);
    assert!(!ioq.problems.is_empty());

    let short_vin = decode_vin("WVWZZZ1JZ2W", 2026);
    assert!(!short_vin.valid);
    assert!(!short_vin.problems.is_empty());
    // The maker is still known from the first three characters.
    assert_eq!(short_vin.brand, "Volkswagen");

    assert!(!decode_vin("", 2026).valid);
}

#[test]
fn north_american_model_years_follow_position_7() {
    let _g = lock();
    // Letter at position 7: second cycle, A = 2010.
    let second = decode_vin(&with_check_digit("1G1ZB5ST0AF000001"), 2026);
    assert!(second.check_digit_ok);
    assert_eq!(second.model_year, 2010);
    assert!(second.model_year_certain);
    assert_eq!(second.brand, "Chevrolet");

    // Digit at position 7: first cycle, A = 1980.
    let first = decode_vin(&with_check_digit("1G1ZB5200AF000001"), 2026);
    assert_eq!(first.model_year, 1980);

    // Y is 2000 in the first cycle and 2030 in the second.
    assert_eq!(vin_base_year('Y'), 2000);
    assert_eq!(vin_base_year('1'), 2001);
    assert_eq!(vin_base_year('9'), 2009);
    assert_eq!(vin_base_year('0'), 0);
    assert_eq!(vin_base_year('U'), 0);
}

#[test]
fn european_model_years_keep_the_alternative() {
    let _g = lock();
    // Golf IV: 2 is 2002, 2032 would be in the future.
    let golf4 = decode_vin("WVWZZZ1JZ2W123456", 2026);
    assert_eq!(golf4.model_years, vec![2002]);
    assert!(golf4.model_year_certain);
    assert!(!golf4.check_digit_applies);
    assert_eq!(golf4.country, "Deutschland");

    // M is 1991 or 2021, the newer one first.
    let golf8 = decode_vin("WVWZZZCDZMW123456", 2026);
    assert_eq!(golf8.model_years, vec![2021, 1991]);
    assert!(!golf8.model_year_certain);

    // Position 10 = 0 carries no year.
    let no_year = decode_vin("WBAAA11000C123456", 2026);
    assert!(no_year.model_years.is_empty());
    assert_eq!(no_year.model_year, 0);
}

#[test]
fn wmi_lookups_cover_old_and_new_makers() {
    let _g = lock();
    let brand_of = |wmi: &str| find_wmi(wmi).map(|e| e.brands).unwrap_or("");
    assert_eq!(brand_of("WAU"), "Audi");
    assert_eq!(brand_of("TMB"), "Škoda");
    assert_eq!(brand_of("WBA"), "BMW");
    assert_eq!(brand_of("YS3"), "Saab");
    assert_eq!(brand_of("SAR"), "Rover|MG");
    assert_eq!(brand_of("1G2"), "Pontiac");
    assert_eq!(brand_of("1G3"), "Oldsmobile");
    assert_eq!(brand_of("1G8"), "Saturn");
    assert_eq!(brand_of("1ME"), "Mercury");
    assert_eq!(brand_of("1P3"), "Plymouth");
    assert_eq!(brand_of("5GR"), "Hummer");
    assert_eq!(brand_of("ZLA"), "Lancia");
    assert_eq!(brand_of("XTA"), "Lada");
    assert_eq!(brand_of("KLA"), "Daewoo");
    assert_eq!(brand_of("5YJ"), "Tesla");
    assert_eq!(brand_of("XP7"), "Tesla");
    assert_eq!(brand_of("LPS"), "Polestar");
    assert_eq!(brand_of("LGX"), "BYD");
    assert_eq!(brand_of("HJN"), "NIO");
    assert_eq!(brand_of("KMT"), "Genesis");
    assert_eq!(brand_of("7PD"), "Rivian");
    assert_eq!(brand_of("50E"), "Lucid");
    assert_eq!(brand_of("RLL"), "VinFast");
    assert_eq!(brand_of("LSJ"), "MG");
    assert_eq!(brand_of("VSS"), "SEAT|Cupra");
    // Two character entries catch the rest of a range.
    assert_eq!(brand_of("JTD"), "Toyota");
    assert_eq!(brand_of("JTH"), "Lexus");
    assert_eq!(brand_of("JN1"), "Nissan");
    assert!(find_wmi("QQQ").is_none());

    assert!(wmi_table().len() > 250);
}

#[test]
fn countries_come_from_the_first_two_characters() {
    let _g = lock();
    set_language(Language::English);
    assert_eq!(vin_country("WVW"), "Germany");
    assert_eq!(vin_country("VF1"), "France");
    assert_eq!(vin_country("ZFA"), "Italy");
    assert_eq!(vin_country("SAJ"), "United Kingdom");
    assert_eq!(vin_country("JTD"), "Japan");
    assert_eq!(vin_country("KMH"), "South Korea");
    assert_eq!(vin_country("LSJ"), "China");
    assert_eq!(vin_country("TMB"), "Czech Republic");
    assert_eq!(vin_country("YV1"), "Sweden");
    assert_eq!(vin_country("3VW"), "Mexico");
    assert_eq!(vin_country("9BW"), "Brazil");
    assert_eq!(vin_region("1G1"), "North America");
    assert_eq!(vin_region("7SA"), "North America");
    assert_eq!(vin_region("7AT"), "Oceania");
    assert_eq!(vin_region("WVW"), "Europe");
    // A plant country that differs from the code range.
    assert_eq!(decode_vin("XP7YGCEK0PB000001", 2026).country, "Germany");
    set_language(Language::German);
    assert_eq!(vin_country("WVW"), "Deutschland");
}

#[test]
fn vag_model_codes_find_the_generation() {
    let _g = lock();
    let a2 = decode_vin("WAUZZZ8ZZ1N012345", 2026);
    assert_eq!(a2.brand, "Audi");
    assert_eq!(a2.model_code, "8Z");
    assert_eq!(a2.model_code_rule, VinModelCodeRule::Vag78);
    let m = catalog::match_vin(&a2);
    assert_eq!(m.generation.unwrap().model, "A2");
    assert_eq!(m.year, 2001);
    assert!(m.confidence >= 85);

    let golf4 = catalog::match_vin(&decode_vin("WVWZZZ1JZ2W123456", 2026));
    let g = golf4.generation.unwrap();
    assert_eq!(g.model, "Golf");
    assert_eq!(g.generation, "IV");
    // The Bora shares the code 1J.
    assert!(!golf4.alternatives.is_empty());

    let golf5 = catalog::match_vin(&decode_vin("WVWZZZ1KZ6W123456", 2026));
    assert_eq!(golf5.generation.unwrap().generation, "V");

    // T = 2026 or 1996. The code 1H only fits 1996, so that year wins.
    let golf3 = catalog::match_vin(&decode_vin("WVWZZZ1HZTW123456", 2026));
    assert_eq!(golf3.generation.unwrap().generation, "III");
    assert_eq!(golf3.year, 1996);

    let golf8 = catalog::match_vin(&decode_vin("WVWZZZCDZMW123456", 2026));
    assert_eq!(golf8.generation.unwrap().generation, "VIII");
    assert_eq!(golf8.year, 2021);

    let octavia = catalog::match_vin(&decode_vin("TMBZZZ1ZZ8B123456", 2026));
    let o = octavia.generation.unwrap();
    assert_eq!(o.model, "Octavia");
    assert_eq!(o.generation, "II");
}

#[test]
fn mercedes_model_series_come_from_positions_4_to_6() {
    let _g = lock();
    let w203 = decode_vin("WDB2030461A123456", 2026);
    assert_eq!(w203.model_code, "203");
    assert_eq!(w203.model_code_rule, VinModelCodeRule::Mercedes456);
    // European Mercedes VINs carry no model year.
    assert!(w203.model_years.is_empty());
    let m = catalog::match_vin(&w203);
    assert_eq!(m.generation.unwrap().generation, "W203");
    assert!(m.confidence >= 70);

    let w204 = catalog::match_vin(&decode_vin("WDD2040081A123456", 2026));
    assert_eq!(w204.generation.unwrap().generation, "W204");
}

#[test]
fn tesla_writes_the_model_into_position_4() {
    let _g = lock();
    let v = decode_vin(&with_check_digit("5YJ3E1EA0LF000001"), 2026);
    assert!(v.check_digit_ok);
    assert_eq!(v.model_code, "3");
    assert_eq!(v.model_year, 2020);
    let m = catalog::match_vin(&v);
    assert_eq!(m.generation.unwrap().model, "Model 3");
}

#[test]
fn without_a_model_code_the_brand_is_still_known() {
    let _g = lock();
    let m = catalog::match_vin(&decode_vin("VF7AAAAAAAA000001", 2026));
    assert!(m.confidence <= 30);
    assert!(!m.reason.is_empty());

    let unknown = catalog::match_vin(&decode_vin("QQQ", 2026));
    assert!(unknown.generation.is_none());
    assert_eq!(unknown.confidence, 0);
}

#[test]
fn the_catalog_is_large_and_consistent() {
    let _g = lock();
    assert!(catalog::size() >= 1000);
    for g in catalog::all() {
        let name = catalog::display_name(g);
        assert!(!g.brand.is_empty(), "{name}");
        assert!(!g.model.is_empty(), "{name}");
        assert!(g.year_from >= 1930, "{name}");
        assert!(g.year_to == 0 || g.year_to >= g.year_from, "{name}");
        assert!(catalog::brand_info(g.brand).is_some(), "{name}");
        assert!(!has_long_dash(g.note.de), "{name}");
        assert!(!has_long_dash(g.note.en), "{name}");
        // An OBD-II socket at some point means there is a protocol order.
        let obd = g.connector == Connector::Obd2 || g.late_connector == Connector::Obd2;
        assert_eq!(obd, !g.protocol_order.is_empty(), "{name}");
    }

    let brands = catalog::brands();
    assert!(brands.len() >= 80);
    for b in [
        "Volkswagen",
        "Saab",
        "Pontiac",
        "Tesla",
        "BYD",
        "Lada",
        "Polestar",
        "Genesis",
        "Cupra",
        "VinFast",
        "Rover",
        "Hummer",
    ] {
        assert!(brands.iter().any(|x| x == b), "{b}");
    }
    let audi = catalog::models("Audi");
    assert!(audi.iter().any(|x| x == "A2"));

    let golfs = catalog::generations("Volkswagen", "Golf");
    assert!(golfs.len() >= 8);
    assert_eq!(golfs[0].generation, "I");
}

#[test]
fn generations_are_found_by_year() {
    let _g = lock();
    let gen_of = |b: &str, m: &str, y: i32| catalog::find_generation(b, m, y).unwrap().generation;
    assert_eq!(gen_of("Volkswagen", "Golf", 2002), "IV");
    assert_eq!(gen_of("BMW", "3er", 2001), "E46");
    assert_eq!(gen_of("Mercedes-Benz", "C-Klasse", 2005), "W203");
    // Case and accents do not matter; in change-over years the newer one wins.
    assert_eq!(gen_of("skoda", "octavia", 2008), "II");
    // Without a year: the newest.
    assert_eq!(gen_of("Volkswagen", "Golf", 0), "VIII");
    assert!(catalog::find_generation("Volkswagen", "Gibtsnicht", 2002).is_none());
}

#[test]
fn the_search_is_tolerant() {
    let _g = lock();
    let first = |q: &str| {
        search_catalog(q, 5)
            .first()
            .map(|g| catalog::display_name(g))
            .unwrap_or_default()
    };
    set_language(Language::English);
    assert_eq!(first("golf 4"), "Volkswagen Golf IV (1997 to 2006)");
    assert_eq!(first("Golf IV"), "Volkswagen Golf IV (1997 to 2006)");
    assert_eq!(first("golf4"), "Volkswagen Golf IV (1997 to 2006)");
    assert_eq!(first("a2"), "Audi A2 8Z (2000 to 2005)");
    assert_eq!(first("e46"), "BMW 3er E46 (1998 to 2006)");
    assert_eq!(first("w203"), "Mercedes-Benz C-Klasse W203 (2000 to 2007)");
    assert_eq!(first("octavia 2008"), "Škoda Octavia II (2004 to 2013)");
    assert_eq!(first("model 3"), "Tesla Model 3 I (since 2017)");
    assert!(first("peugeot 2008").starts_with("Peugeot 2008"));
    assert_eq!(first("citroen c3 2010"), "Citroën C3 II (2009 to 2016)");
    assert_eq!(first("mx5 nb"), "Mazda MX-5 NB (1998 to 2005)");
    assert!(search_catalog("xyzzy", 20).is_empty());
    assert!(search_catalog("", 20).is_empty());
    set_language(Language::German);
    assert_eq!(first("golf 4"), "Volkswagen Golf IV (1997 bis 2006)");
}

#[test]
fn diagnosis_is_described_honestly() {
    let _g = lock();
    let find = |b: &str, m: &str, y: i32| -> &'static VehicleGeneration {
        catalog::find_generation(b, m, y).unwrap()
    };

    let golf2 = find("Volkswagen", "Golf", 1985);
    assert_eq!(catalog::support_for(golf2, 1985, 0), ObdSupport::None);
    assert_eq!(catalog::support_for(golf2, 1991, 0), ObdSupport::Kkl);
    assert!(catalog::protocol_order_for(golf2).is_empty());

    let golf3 = find("Volkswagen", "Golf", 1993);
    assert_eq!(catalog::connector_for(golf3, 1993), Connector::Vag2x2);
    assert_eq!(catalog::connector_for(golf3, 1997), Connector::Obd2);
    assert_eq!(catalog::support_for(golf3, 1993, 0), ObdSupport::Kkl);

    let golf4 = find("Volkswagen", "Golf", 1999);
    assert_eq!(catalog::support_for(golf4, 1999, 0), ObdSupport::Partial);
    assert!(catalog::kkl_possible(golf4, 1999));
    assert_eq!(catalog::support_for(golf4, 2003, 0), ObdSupport::Full);
    assert_eq!(
        catalog::support_for(golf4, 2002, PT_DIESEL),
        ObdSupport::Partial
    );
    assert_eq!(golf4.location, SocketLocation::CentreConsole);

    let a2 = find("Audi", "A2", 2002);
    assert_eq!(catalog::support_for(a2, 2002, 0), ObdSupport::Full);
    assert_eq!(catalog::protocol_order_for(a2), "5436789");
    assert_eq!(a2.bus, BusKind::KLine);
    assert!(!catalog::diagnosis_text(a2, 2002).is_empty());

    let e36 = find("BMW", "3er", 1995);
    assert_eq!(catalog::support_for(e36, 1995, 0), ObdSupport::Manufacturer);
    assert_eq!(e36.location, SocketLocation::EngineBay);

    let f150 = find("Ford", "F-150", 1998);
    assert!(f150.us_market);
    assert_eq!(f150.bus, BusKind::Pwm);
    assert_eq!(catalog::support_for(f150, 1998, 0), ObdSupport::Full);

    let leaf = find("Nissan", "Leaf", 2015);
    assert_eq!(catalog::support_for(leaf, 2015, 0), ObdSupport::Partial);

    let model3 = find("Tesla", "Model 3", 2022);
    assert_eq!(
        catalog::support_for(model3, 2022, 0),
        ObdSupport::Manufacturer
    );
    assert_eq!(model3.location, SocketLocation::RearConsole);
    assert_eq!(
        catalog::location_for(model3, 2022),
        SocketLocation::RearConsole
    );
    assert_eq!(
        catalog::location_for(golf4, 2003),
        SocketLocation::CentreConsole
    );
}

#[test]
fn the_sketch_spot_follows_the_record_and_the_round_plugs() {
    for g in catalog::all() {
        let year = g.year_from;
        let spot = catalog::location_for(g, year);
        if g.location != SocketLocation::Unknown {
            assert_eq!(spot, g.location, "{} {}", g.brand, g.model);
        } else if matches!(
            catalog::connector_for(g, year),
            Connector::Bmw20 | Connector::Mb38
        ) {
            assert_eq!(spot, SocketLocation::EngineBay, "{} {}", g.brand, g.model);
        } else {
            assert_eq!(spot, SocketLocation::Unknown, "{} {}", g.brand, g.model);
        }
    }
}

#[test]
fn enum_texts_exist_in_both_languages() {
    let _g = lock();
    for lang in [Language::German, Language::English] {
        set_language(lang);
        for b in BodyType::ALL {
            assert!(!body_name(b).is_empty());
        }
        for c in Connector::ALL {
            assert!(!connector_name(c).is_empty());
            assert!(!has_long_dash(connector_name(c)));
        }
        for s in SocketLocation::ALL {
            assert!(!location_text(s).is_empty());
            assert!(!has_long_dash(location_text(s)));
        }
        for s in ObdSupport::ALL {
            assert!(!support_name(s).is_empty());
        }
    }
    set_language(Language::German);
    assert_eq!(powertrain_text(PT_PETROL | PT_DIESEL), "Benzin, Diesel");
    assert_eq!(body_name(BodyType::Estate), "Kombi");
    set_language(Language::English);
    assert_eq!(powertrain_text(PT_ELECTRIC), "Electric");
    set_language(Language::German);

    // Every diagnosis text of the catalog is free of long dashes.
    for g in catalog::all() {
        assert!(!has_long_dash(&catalog::diagnosis_text(g, 0)));
        assert!(!location_text_for(g, 0).is_empty());
    }
}

#[test]
fn new_brands_are_recognised_from_the_vin_and_found_in_the_catalog() {
    let _g = lock();
    // WMI after the Wikibooks list; the rest of each VIN is made up.
    let cases = [
        ("HXMAA1234R0000001", "Xiaomi"),
        ("L1NSPGHB5PA000001", "Xpeng"),
        ("LJ1EEAUU6P0000001", "NIO"),
        ("LW433B1A0P0000001", "Li Auto"),
        ("LFZ63AL55RD000001", "Leapmotor"),
        ("LM8F7G1A5PA000001", "Aito"),
        ("LDP95H961PE000001", "Voyah"),
        ("LS6A3E0A5PB000001", "Avatr"),
        ("SC6GM1CA5PE000001", "INEOS"),
        ("YSMYKEAE5PB000001", "Polestar"),
        ("LSH14J7C5PA000001", "Maxus"),
        ("LFPH4ACP5P1000001", "Hongqi"),
    ];
    for (vin, brand) in cases {
        let v = decode_vin(vin, 2026);
        assert_eq!(v.brand, brand, "{vin}");
        assert!(!catalog::models(brand).is_empty(), "{vin}");
    }
    // A code shared by several brands of one group lists all of them.
    let chery = decode_vin("LVVDB21B5PD000001", 2026);
    assert!(chery.brand_candidates.iter().any(|b| b == "Jaecoo"));
}

#[test]
fn every_catalog_brand_can_be_recognised_from_the_vin() {
    let _g = lock();
    let mut known: BTreeSet<&str> = BTreeSet::new();
    for e in wmi_table() {
        known.extend(e.brands.split('|'));
    }
    for brand in catalog::brands() {
        assert!(known.contains(brand.as_str()), "{brand}");
        // Every generation has sensible years.
        for model in catalog::models(&brand) {
            for g in catalog::generations(&brand, &model) {
                assert!(g.year_from >= 1900);
                assert!(g.year_to == 0 || g.year_to >= g.year_from);
            }
        }
    }
}
