//! Vehicle profiles: which protocol is tried first, what to expect.
//!
//! Notes and the auto-detect entry come out in the active language (see
//! `caruspex_i18n`). The lookup functions pick the list for the language that
//! is active at the time of the call; the references stay valid for the whole
//! program run.

use caruspex_i18n::{Language, Text, l, language};
use std::collections::BTreeSet;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BusKind {
    /// ISO 9141-2 / ISO 14230, older European and Japanese cars.
    KLine,
    /// ISO 15765-4, mandatory in Europe from about 2004.
    Can,
    /// SAE J1850 PWM, older Ford USA.
    Pwm,
    /// SAE J1850 VPW, older GM USA.
    Vpw,
    /// Transition period, either is possible.
    #[default]
    Mixed,
}

#[derive(Clone, Debug, Default)]
pub struct VehicleProfile {
    pub brand: String,
    /// Empty = whole brand.
    pub model: String,
    pub year_from: i32,
    pub year_to: i32,
    pub bus: BusKind,
    /// ELM327 protocol numbers in the order they are tried.
    pub protocol_order: String,
    pub note: String,
}

// Protocol orders as ELM327 numbers:
//   1 J1850 PWM, 2 J1850 VPW, 3 ISO 9141-2, 4 KWP 5-baud, 5 KWP fast,
//   6 CAN 11/500, 7 CAN 29/500, 8 CAN 11/250, 9 CAN 29/250

/// Order when nothing is known: fast protocols first.
pub const AUTO_PROTOCOL_ORDER: &str = "6785439121";

const ORDER_CAN: &str = "6789543";
const ORDER_K_LINE: &str = "5436789";
const ORDER_MIXED: &str = "6543789";
const ORDER_PWM: &str = "1265439";
const ORDER_VPW: &str = "2165439";

const AUTO_DETECT: Text = Text::new("Automatisch erkennen", "Auto-detect");

// Notes that occur many times.
const NOTE_CAN: Text = Text::new("CAN.", "CAN.");
const NOTE_ISO: Text = Text::new("ISO 9141-2.", "ISO 9141-2.");
const NOTE_K_LINE_ERA: Text = Text::new("K-Line-Zeit.", "K-Line era.");
const NOTE_NONE: Text = Text::new("", "");

/// Builds the profile list for one language. Only the notes and the
/// auto-detect entry differ between the languages.
fn build(lang: Language) -> Vec<VehicleProfile> {
    let en = lang == Language::English;
    let mut v: Vec<VehicleProfile> = Vec::new();
    let mut add =
        |brand: &str, model: &str, from: i32, to: i32, bus: BusKind, order: &str, note: Text| {
            v.push(VehicleProfile {
                brand: brand.to_string(),
                model: model.to_string(),
                year_from: from,
                year_to: to,
                bus,
                protocol_order: order.to_string(),
                note: (if en { note.en } else { note.de }).to_string(),
            });
        };

    // --- Volkswagen Group ---------------------------------------------------
    add(
        "Audi",
        "A2",
        2000,
        2005,
        BusKind::KLine,
        ORDER_K_LINE,
        Text::new(
            "K-Line. EOBD über das Motorsteuergerät, alles Weitere braucht KKL.",
            "K-Line. EOBD via the engine control unit, everything else needs a KKL cable.",
        ),
    );
    add(
        "Audi",
        "",
        1996,
        2003,
        BusKind::KLine,
        ORDER_K_LINE,
        Text::new(
            "K-Line-Zeit, KWP1281 oder KWP2000.",
            "K-Line era, KWP1281 or KWP2000.",
        ),
    );
    add(
        "Audi",
        "",
        2004,
        2030,
        BusKind::Can,
        ORDER_CAN,
        Text::new(
            "CAN, ab Werk EOBD-konform.",
            "CAN, EOBD compliant from the factory.",
        ),
    );
    add(
        "Volkswagen",
        "",
        1996,
        2003,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_K_LINE_ERA,
    );
    add(
        "Volkswagen",
        "",
        2004,
        2030,
        BusKind::Can,
        ORDER_CAN,
        NOTE_CAN,
    );
    add(
        "Skoda",
        "",
        1998,
        2003,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_K_LINE_ERA,
    );
    add("Skoda", "", 2004, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Seat",
        "",
        1998,
        2003,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_K_LINE_ERA,
    );
    add("Seat", "", 2004, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Porsche",
        "",
        1998,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        Text::new("Ab 2003 durchgängig CAN.", "CAN throughout from 2003."),
    );

    // --- German manufacturers -----------------------------------------------
    add(
        "BMW",
        "",
        1996,
        2003,
        BusKind::KLine,
        ORDER_K_LINE,
        Text::new(
            "ISO 9141-2, teils BMW-eigen.",
            "ISO 9141-2, partly BMW-specific.",
        ),
    );
    add("BMW", "", 2004, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Mini",
        "",
        2001,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        Text::new("Bis 2006 teils K-Line.", "Partly K-Line until 2006."),
    );
    add(
        "Mercedes-Benz",
        "",
        1996,
        2003,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add(
        "Mercedes-Benz",
        "",
        2004,
        2030,
        BusKind::Can,
        ORDER_CAN,
        NOTE_CAN,
    );
    add(
        "Smart",
        "",
        1998,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        Text::new("Früh K-Line, später CAN.", "K-Line early on, CAN later."),
    );
    add(
        "Opel",
        "",
        1996,
        2005,
        BusKind::Mixed,
        ORDER_MIXED,
        Text::new(
            "Teils ISO 9141-2, teils J1850 VPW.",
            "Partly ISO 9141-2, partly J1850 VPW.",
        ),
    );
    add("Opel", "", 2006, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);

    // --- France -------------------------------------------------------------
    add(
        "Renault",
        "",
        1996,
        2004,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add("Renault", "", 2005, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Dacia",
        "",
        2004,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        Text::new("Technik von Renault.", "Renault technology."),
    );
    add(
        "Peugeot",
        "",
        1996,
        2004,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add("Peugeot", "", 2005, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Citroen",
        "",
        1996,
        2004,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add("Citroen", "", 2005, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add("DS", "", 2014, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);

    // --- Italy --------------------------------------------------------------
    add(
        "Fiat",
        "",
        1996,
        2005,
        BusKind::KLine,
        ORDER_K_LINE,
        Text::new("ISO 9141-2, teils KWP.", "ISO 9141-2, partly KWP."),
    );
    add("Fiat", "", 2006, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Alfa Romeo",
        "",
        1996,
        2005,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add(
        "Alfa Romeo",
        "",
        2006,
        2030,
        BusKind::Can,
        ORDER_CAN,
        NOTE_CAN,
    );
    add(
        "Lancia",
        "",
        1996,
        2005,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add(
        "Ferrari",
        "",
        1998,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        NOTE_NONE,
    );
    add(
        "Maserati",
        "",
        2002,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        NOTE_NONE,
    );

    // --- Sweden and Great Britain -------------------------------------------
    add(
        "Volvo",
        "",
        1996,
        2004,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add("Volvo", "", 2005, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Saab",
        "",
        1996,
        2011,
        BusKind::Mixed,
        ORDER_MIXED,
        Text::new("Teils J1850 VPW wie GM.", "Partly J1850 VPW like GM."),
    );
    add(
        "Jaguar",
        "",
        1996,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        NOTE_NONE,
    );
    add(
        "Land Rover",
        "",
        1996,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        NOTE_NONE,
    );

    // --- Japan --------------------------------------------------------------
    add(
        "Toyota",
        "",
        1996,
        2003,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add("Toyota", "", 2004, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Lexus",
        "",
        1998,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        NOTE_NONE,
    );
    add(
        "Honda",
        "",
        1996,
        2003,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add("Honda", "", 2004, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Nissan",
        "",
        1996,
        2004,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add("Nissan", "", 2005, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Mazda",
        "",
        1996,
        2004,
        BusKind::Mixed,
        ORDER_MIXED,
        Text::new("Teils J1850 PWM wie Ford.", "Partly J1850 PWM like Ford."),
    );
    add("Mazda", "", 2005, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Mitsubishi",
        "",
        1996,
        2005,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add(
        "Mitsubishi",
        "",
        2006,
        2030,
        BusKind::Can,
        ORDER_CAN,
        NOTE_CAN,
    );
    add(
        "Subaru",
        "",
        1996,
        2005,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add("Subaru", "", 2006, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Suzuki",
        "",
        1998,
        2005,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add("Suzuki", "", 2006, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Daihatsu",
        "",
        1998,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        NOTE_NONE,
    );

    // --- Korea --------------------------------------------------------------
    add(
        "Hyundai",
        "",
        1998,
        2005,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add("Hyundai", "", 2006, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Kia",
        "",
        1998,
        2005,
        BusKind::KLine,
        ORDER_K_LINE,
        NOTE_ISO,
    );
    add("Kia", "", 2006, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "SsangYong",
        "",
        2000,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        NOTE_NONE,
    );

    // --- USA ----------------------------------------------------------------
    add(
        "Ford",
        "",
        1996,
        2003,
        BusKind::Pwm,
        ORDER_PWM,
        Text::new("SAE J1850 PWM.", "SAE J1850 PWM."),
    );
    add("Ford", "", 2004, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Chevrolet",
        "",
        1996,
        2005,
        BusKind::Vpw,
        ORDER_VPW,
        Text::new("SAE J1850 VPW.", "SAE J1850 VPW."),
    );
    add(
        "Chevrolet",
        "",
        2006,
        2030,
        BusKind::Can,
        ORDER_CAN,
        NOTE_CAN,
    );
    add(
        "Chrysler",
        "",
        1996,
        2005,
        BusKind::Mixed,
        ORDER_MIXED,
        NOTE_NONE,
    );
    add(
        "Jeep",
        "",
        1996,
        2005,
        BusKind::Mixed,
        ORDER_MIXED,
        NOTE_NONE,
    );
    add("Jeep", "", 2006, 2030, BusKind::Can, ORDER_CAN, NOTE_CAN);
    add(
        "Dodge",
        "",
        1996,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        NOTE_NONE,
    );
    add(
        "Tesla",
        "",
        2012,
        2030,
        BusKind::Can,
        ORDER_CAN,
        Text::new(
            "CAN, EOBD nur eingeschränkt.",
            "CAN, EOBD only partly supported.",
        ),
    );

    // --- Other --------------------------------------------------------------
    add(
        "Iveco",
        "",
        2000,
        2030,
        BusKind::Mixed,
        ORDER_MIXED,
        Text::new("Nutzfahrzeug.", "Commercial vehicle."),
    );
    add(
        "MAN",
        "",
        2000,
        2030,
        BusKind::Can,
        ORDER_CAN,
        Text::new(
            "Nutzfahrzeug, teils J1939.",
            "Commercial vehicle, partly J1939.",
        ),
    );

    add(
        if en { AUTO_DETECT.en } else { AUTO_DETECT.de },
        "",
        1996,
        2030,
        BusKind::Mixed,
        AUTO_PROTOCOL_ORDER,
        Text::new(
            "Probiert alle Protokolle der Reihe nach durch. Funktioniert immer, \
             dauert nur ein paar Sekunden länger.",
            "Tries all protocols one after another. Always works, \
             it just takes a few seconds longer.",
        ),
    );
    v
}

/// One list per language, both built on first use. The active language
/// decides which one is handed out.
pub fn vehicle_profiles() -> &'static [VehicleProfile] {
    static GERMAN: OnceLock<Vec<VehicleProfile>> = OnceLock::new();
    static ENGLISH: OnceLock<Vec<VehicleProfile>> = OnceLock::new();
    if language() == Language::English {
        ENGLISH.get_or_init(|| build(Language::English))
    } else {
        GERMAN.get_or_init(|| build(Language::German))
    }
}

/// Brands in alphabetical order, for the selection list.
pub fn vehicle_brands() -> Vec<String> {
    let unique: BTreeSet<&str> = vehicle_profiles()
        .iter()
        .map(|p| p.brand.as_str())
        .collect();
    unique.into_iter().map(str::to_string).collect()
}

/// Profiles of one brand.
pub fn profiles_for_brand(brand: &str) -> Vec<&'static VehicleProfile> {
    vehicle_profiles()
        .iter()
        .filter(|p| p.brand == brand)
        .collect()
}

/// Best profile for brand and model year, otherwise `None`.
pub fn find_profile(brand: &str, year: i32) -> Option<&'static VehicleProfile> {
    let mut fallback = None;
    for p in vehicle_profiles() {
        if p.brand != brand {
            continue;
        }
        fallback = Some(p);
        if year >= p.year_from && year <= p.year_to {
            return Some(p);
        }
    }
    fallback
}

/// The "brand" entry that tries all protocols ("Automatisch erkennen" /
/// "Auto-detect"), in the active language. Use it to preselect that entry.
pub fn auto_detect_brand() -> &'static str {
    AUTO_DETECT.get()
}

pub fn bus_name(bus: BusKind) -> &'static str {
    match bus {
        BusKind::KLine => "K-Line (ISO 9141-2 / KWP2000)",
        BusKind::Can => "CAN (ISO 15765-4)",
        BusKind::Pwm => "SAE J1850 PWM",
        BusKind::Vpw => "SAE J1850 VPW",
        BusKind::Mixed => l("gemischt, wird ermittelt", "mixed, will be detected"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_are_found_by_brand_and_year() {
        let audi = profiles_for_brand("Audi");
        assert_eq!(audi.len(), 3);
        assert_eq!(audi[0].model, "A2");
        assert_eq!(
            find_profile("Volkswagen", 2000).unwrap().bus,
            BusKind::KLine
        );
        assert_eq!(find_profile("Volkswagen", 2010).unwrap().bus, BusKind::Can);
        // Outside every range the last profile of the brand is the fallback.
        assert_eq!(find_profile("Volkswagen", 1980).unwrap().year_from, 2004);
        assert!(find_profile("Gibtsnicht", 2000).is_none());
        assert!(vehicle_brands().windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn bus_names() {
        assert_eq!(bus_name(BusKind::Can), "CAN (ISO 15765-4)");
        assert_eq!(bus_name(BusKind::Vpw), "SAE J1850 VPW");
    }
}
