//! VAG fault numbers in plain text.
//!
//! The manufacturer-specific diagnostics deliver five-digit numbers instead of
//! P-codes. From 16384 upwards these are converted EOBD codes: the number minus
//! 16384 gives the digits of the P-code. That way the existing fault database
//! can be reused. Below that lie the old VAG-specific numbers.
//!
//! Important: these old numbers are defined per control unit. 01276 means
//! "brake light switch" in the engine control unit, but "hydraulic pump" in
//! the ABS control unit. A single flat list would therefore simply be wrong.
//! That is why the plain text is always looked up together with the control
//! unit address. If the address is unknown, the engine control unit is
//! assumed and a match from another area is labelled as such.
//!
//! The texts come out in the active language (see `caruspex_i18n`).

use caruspex_i18n::{Text, l};

use crate::dtc::describe_dtc;

/// Functional area of a control unit. Several addresses can share an area,
/// for example all door control units belong to the comfort system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FaultArea {
    Engine,
    Transmission,
    Brakes,
    Airbag,
    Cluster,
    Comfort,
    Immobilizer,
    Steering,
    Unknown,
}

/// From this number on they are converted EOBD codes.
const P_CODE_OFFSET: i32 = 16384;

/// Texts are stored in both languages and resolved only at lookup time.
type Table = &'static [(i32, Text)];

const fn e(code: i32, de: &'static str, en: &'static str) -> (i32, Text) {
    (code, Text::new(de, en))
}

/// Numbers that mean the same in every control unit: power supply, coding and
/// the messages about bus partners that have dropped out.
static SHARED: Table = &[
    e(
        532,
        "Spannungsversorgung Klemme 30",
        "Power supply terminal 30",
    ),
    e(
        668,
        "Bordspannung Klemme 30: Spannung zu klein",
        "Battery voltage terminal 30: voltage too low",
    ),
    e(
        1044,
        "Steuergerät falsch codiert",
        "Control unit incorrectly coded",
    ),
    e(
        1312,
        "Datenbus Antrieb: gestört",
        "Drivetrain data bus: malfunction",
    ),
    e(
        1314,
        "Motorsteuergerät: keine Verbindung",
        "Engine control unit: no communication",
    ),
    e(
        1316,
        "Bremsensteuergerät: keine Verbindung",
        "Brake control unit: no communication",
    ),
    e(
        1317,
        "Kombiinstrument: keine Verbindung",
        "Instrument cluster: no communication",
    ),
    e(
        1321,
        "Airbag-Steuergerät: keine Verbindung",
        "Airbag control unit: no communication",
    ),
    e(1304, "Radio: keine Verbindung", "Radio: no communication"),
    e(4096, "Steuergerät defekt", "Control unit faulty"),
    e(
        8213,
        "Steuergerät falsch codiert",
        "Control unit incorrectly coded",
    ),
    e(
        65535,
        "Interner Steuergerätefehler",
        "Internal control unit fault",
    ),
];

// --- Address 01, engine ------------------------------------------------------
// This is the area that yields the most on both A2 variants: the 1.4 MPI as
// well as the 1.6 FSI report their sensors and actuators through it.
static ENGINE: Table = &[
    e(
        281,
        "Geber für Geschwindigkeit G22",
        "Vehicle speed sensor G22",
    ),
    e(513, "Motordrehzahlgeber G28", "Engine speed sensor G28"),
    e(514, "Nockenwellengeber G40", "Camshaft position sensor G40"),
    e(515, "Hallgeber G40", "Hall sensor G40"),
    e(516, "Leerlaufschalter F60", "Idle switch F60"),
    e(517, "Vollastschalter F81", "Full throttle switch F81"),
    e(
        518,
        "Drosselklappenpotentiometer G69",
        "Throttle position sensor G69",
    ),
    e(519, "Saugrohrdruckgeber", "Intake manifold pressure sensor"),
    e(
        522,
        "Kühlmitteltemperaturgeber G62",
        "Coolant temperature sensor G62",
    ),
    e(
        523,
        "Ansauglufttemperaturgeber G72",
        "Intake air temperature sensor G72",
    ),
    e(524, "Klopfsensor 1 G61", "Knock sensor 1 G61"),
    e(525, "Lambdasonde G39", "Oxygen sensor G39"),
    e(526, "Bremslichtschalter F", "Brake light switch F"),
    e(
        527,
        "Ansauglufttemperaturgeber G42",
        "Intake air temperature sensor G42",
    ),
    e(528, "Höhengeber F96", "Altitude sensor F96"),
    e(
        530,
        "Drosselklappenpotentiometer G88",
        "Throttle position sensor G88",
    ),
    e(
        531,
        "Saugrohrdruck: Steuergerät defekt",
        "Intake manifold pressure: control unit faulty",
    ),
    e(
        533,
        "Leerlaufregelung: Regelgrenze erreicht",
        "Idle speed control: limit reached",
    ),
    e(535, "Klopfsensor 2 G66", "Knock sensor 2 G66"),
    e(
        537,
        "Lambdaregelung: Regelgrenze erreicht",
        "Oxygen sensor control: limit reached",
    ),
    e(539, "Kraftstoffdruckgeber", "Fuel pressure sensor"),
    e(
        540,
        "Klimakompressor abgeschaltet",
        "A/C compressor switched off",
    ),
    e(
        543,
        "Höchstzulässige Motordrehzahl überschritten",
        "Maximum permissible engine speed exceeded",
    ),
    e(545, "Kupplungsschalter", "Clutch switch"),
    e(
        546,
        "Datenbus Antrieb: keine Botschaft",
        "Drivetrain data bus: no message",
    ),
    e(549, "Kraftstoffverbrauchssignal", "Fuel consumption signal"),
    e(550, "Motorlastsignal", "Engine load signal"),
    e(
        552,
        "Luftmassenmesser G70: Signal zu klein",
        "Mass air flow sensor G70: signal too low",
    ),
    e(553, "Luftmassenmesser G70", "Mass air flow sensor G70"),
    e(
        555,
        "Lambdaregelung: Regelgrenze unterschritten",
        "Oxygen sensor control: below lower limit",
    ),
    e(
        558,
        "Einspritzventile: Regelabweichung",
        "Injectors: control deviation",
    ),
    e(560, "Lambdasonde: Steuerung", "Oxygen sensor: control"),
    e(
        561,
        "Gemischadaption: Regelgrenze überschritten",
        "Mixture adaptation: limit exceeded",
    ),
    e(
        575,
        "Saugrohrdruck: Regelgrenze",
        "Intake manifold pressure: control limit",
    ),
    e(577, "Klopfregelung Zylinder 1", "Knock control cylinder 1"),
    e(578, "Klopfregelung Zylinder 2", "Knock control cylinder 2"),
    e(579, "Klopfregelung Zylinder 3", "Knock control cylinder 3"),
    e(580, "Klopfregelung Zylinder 4", "Knock control cylinder 4"),
    e(585, "AGR-Temperaturgeber", "EGR temperature sensor"),
    e(586, "Abgasrückführung", "Exhaust gas recirculation"),
    e(
        587,
        "Magnetventil für Abgasrückführung N18",
        "EGR solenoid valve N18",
    ),
    e(609, "Zündsignal: Endstufe", "Ignition signal: output stage"),
    e(624, "Klimaanlage: Signal", "Air conditioning: signal"),
    e(625, "Geschwindigkeitssignal", "Vehicle speed signal"),
    e(635, "Lambdasondenheizung Z19", "Oxygen sensor heater Z19"),
    e(
        640,
        "Drosselklappenpotentiometer: Signal unplausibel",
        "Throttle position sensor: implausible signal",
    ),
    e(
        652,
        "Lambdasonde nach Katalysator G130",
        "Oxygen sensor after catalytic converter G130",
    ),
    e(
        660,
        "Ventil für Nockenwellenverstellung N205",
        "Camshaft adjustment valve N205",
    ),
    e(
        670,
        "Leerlaufsteller: Regelgrenze",
        "Idle actuator: control limit",
    ),
    e(750, "Fehlerlampe K83", "Malfunction indicator lamp K83"),
    e(
        1165,
        "Drosselklappensteuerung: Regelbereich",
        "Throttle body control: control range",
    ),
    e(1177, "Steuergerät: Anpassung", "Control unit: adaptation"),
    e(
        1247,
        "Magnetventil für Aktivkohlebehälter N80",
        "EVAP canister purge valve N80",
    ),
    e(
        1249,
        "Einspritzventil Zylinder 1 N30",
        "Injector cylinder 1 N30",
    ),
    e(
        1250,
        "Einspritzventil Zylinder 2 N31",
        "Injector cylinder 2 N31",
    ),
    e(
        1251,
        "Einspritzventil Zylinder 3 N32",
        "Injector cylinder 3 N32",
    ),
    e(
        1252,
        "Einspritzventil Zylinder 4 N33",
        "Injector cylinder 4 N33",
    ),
    e(
        1253,
        "Einspritzventil Zylinder 5 N83",
        "Injector cylinder 5 N83",
    ),
    e(
        1254,
        "Einspritzventil Zylinder 6 N84",
        "Injector cylinder 6 N84",
    ),
    e(
        1262,
        "Magnetventil für Ladedruckbegrenzung N75",
        "Boost pressure control valve N75",
    ),
    e(1265, "Ventil für Abgasrückführung N18", "EGR valve N18"),
    e(
        1266,
        "Ventil für Saugrohrumschaltung N156",
        "Intake manifold change-over valve N156",
    ),
    e(1268, "Mengensteller", "Fuel quantity adjuster"),
    e(
        1271,
        "Drosselklappensteuereinheit J338",
        "Throttle body J338",
    ),
    e(1276, "Bremslichtschalter F", "Brake light switch F"),
    e(
        1289,
        "Wegfahrsperre: keine Freigabe",
        "Immobilizer: not enabled",
    ),
    e(
        1393,
        "Steuergerät: Endstufe defekt",
        "Control unit: output stage faulty",
    ),
    e(1500, "Kraftstoffpumpenrelais J17", "Fuel pump relay J17"),
    e(
        2142,
        "Kombiinstrument: Datenbus",
        "Instrument cluster: data bus",
    ),
    e(
        2564,
        "Klimasteuergerät: keine Verbindung",
        "Climate control unit: no communication",
    ),
];

// --- Address 03, brake electronics -------------------------------------------
// For the A2 the most important area besides the engine, because ABS faults
// come out there as VAG numbers and not as C-codes.
static BRAKES: Table = &[
    e(
        283,
        "Raddrehzahlfühler vorne links G47",
        "Wheel speed sensor front left G47",
    ),
    e(
        285,
        "Raddrehzahlfühler vorne rechts G45",
        "Wheel speed sensor front right G45",
    ),
    e(
        287,
        "Raddrehzahlfühler hinten rechts G44",
        "Wheel speed sensor rear right G44",
    ),
    e(
        290,
        "Raddrehzahlfühler hinten links G46",
        "Wheel speed sensor rear left G46",
    ),
    e(778, "Lenkwinkelgeber G85", "Steering angle sensor G85"),
    e(
        1130,
        "ABS-Betrieb: Regelung dauerhaft aktiv",
        "ABS operation: control permanently active",
    ),
    e(1201, "Hydraulikpumpe für ABS V64", "ABS hydraulic pump V64"),
    e(1202, "Magnetventile für ABS", "ABS solenoid valves"),
    e(
        1203,
        "Einlassventil vorne links N101",
        "Inlet valve front left N101",
    ),
    e(
        1204,
        "Auslassventil vorne links N102",
        "Outlet valve front left N102",
    ),
    e(
        1205,
        "Einlassventil vorne rechts N99",
        "Inlet valve front right N99",
    ),
    e(
        1206,
        "Auslassventil vorne rechts N100",
        "Outlet valve front right N100",
    ),
    e(
        1276,
        "Hydraulikpumpe für ABS V64: Stromkreis",
        "ABS hydraulic pump V64: circuit",
    ),
    e(
        1279,
        "Längsbeschleunigungsgeber G251",
        "Longitudinal acceleration sensor G251",
    ),
    e(
        1423,
        "Querbeschleunigungsgeber G200",
        "Lateral acceleration sensor G200",
    ),
    e(
        1435,
        "Bremsdruckgeber 1 G201",
        "Brake pressure sensor 1 G201",
    ),
];

// --- Address 15, airbag ------------------------------------------------------
static AIRBAG: Table = &[
    e(
        588,
        "Zünder für Fahrerairbag N95",
        "Driver airbag igniter N95",
    ),
    e(
        589,
        "Zünder für Beifahrerairbag N131",
        "Passenger airbag igniter N131",
    ),
    e(
        591,
        "Zünder für Gurtstraffer Fahrerseite N153",
        "Belt tensioner igniter, driver side N153",
    ),
    e(
        592,
        "Zünder für Gurtstraffer Beifahrerseite N154",
        "Belt tensioner igniter, passenger side N154",
    ),
    e(
        595,
        "Seitenairbag Fahrerseite N199",
        "Side airbag, driver side N199",
    ),
    e(
        596,
        "Seitenairbag Beifahrerseite N200",
        "Side airbag, passenger side N200",
    ),
    e(
        1217,
        "Seitenairbag Fahrerseite: Widerstand",
        "Side airbag, driver side: resistance",
    ),
    e(
        1218,
        "Seitenairbag Beifahrerseite: Widerstand",
        "Side airbag, passenger side: resistance",
    ),
    e(
        1219,
        "Abschaltung Beifahrerairbag: Schalter",
        "Passenger airbag deactivation: switch",
    ),
    e(
        1221,
        "Crashsensor Fahrerseite G179",
        "Crash sensor, driver side G179",
    ),
    e(
        1222,
        "Crashsensor Beifahrerseite G180",
        "Crash sensor, passenger side G180",
    ),
    e(
        1223,
        "Sitzbelegungserkennung Beifahrer",
        "Passenger seat occupancy detection",
    ),
    e(1225, "Kopfairbag Fahrerseite", "Head airbag, driver side"),
    e(
        1226,
        "Kopfairbag Beifahrerseite",
        "Head airbag, passenger side",
    ),
];

// --- Address 17, instrument cluster ------------------------------------------
static CLUSTER: Table = &[
    e(771, "Geber für Kraftstoffvorrat G", "Fuel level sensor G"),
    e(
        281,
        "Geber für Geschwindigkeit G22",
        "Vehicle speed sensor G22",
    ),
];

// --- Address 46, comfort system ----------------------------------------------
static COMFORT: Table = &[
    e(
        928,
        "Zentralverriegelung Fahrertür",
        "Central locking, driver door",
    ),
    e(
        929,
        "Zentralverriegelung Beifahrertür",
        "Central locking, passenger door",
    ),
    e(
        1331,
        "Türsteuergerät Fahrerseite: keine Verbindung",
        "Door control unit, driver side: no communication",
    ),
    e(
        1332,
        "Türsteuergerät Beifahrerseite: keine Verbindung",
        "Door control unit, passenger side: no communication",
    ),
    e(
        1333,
        "Türsteuergerät hinten links: keine Verbindung",
        "Door control unit, rear left: no communication",
    ),
    e(
        1334,
        "Türsteuergerät hinten rechts: keine Verbindung",
        "Door control unit, rear right: no communication",
    ),
];

// --- Address 25, immobilizer -------------------------------------------------
static IMMOBILIZER: Table = &[
    e(
        1176,
        "Schlüssel: Signal fehlt oder Schlüssel nicht angelernt",
        "Key: no signal or key not adapted",
    ),
    e(
        1177,
        "Motorsteuergerät: Anpassung fehlt oder stimmt nicht",
        "Engine control unit: adaptation missing or incorrect",
    ),
    e(
        1179,
        "Schlüsselanpassung unvollständig",
        "Key adaptation incomplete",
    ),
];

// --- Address 16, steering wheel electronics ----------------------------------
static STEERING: Table = &[e(778, "Lenkwinkelgeber G85", "Steering angle sensor G85")];

// --- Address 02, transmission ------------------------------------------------
static TRANSMISSION: Table = &[
    e(
        281,
        "Geber für Geschwindigkeit G22",
        "Vehicle speed sensor G22",
    ),
    e(
        525,
        "Getriebeoeltemperatur",
        "Transmission fluid temperature",
    ),
];

struct AreaEntry {
    area: FaultArea,
    table: Table,
    label: Text,
}

/// The order matters: with an unknown address the first area that knows the
/// number wins.
static AREA_TABLES: [AreaEntry; 8] = [
    AreaEntry {
        area: FaultArea::Engine,
        table: ENGINE,
        label: Text::new("Motor", "engine"),
    },
    AreaEntry {
        area: FaultArea::Transmission,
        table: TRANSMISSION,
        label: Text::new("Getriebe", "transmission"),
    },
    AreaEntry {
        area: FaultArea::Brakes,
        table: BRAKES,
        label: Text::new("Bremsen", "brakes"),
    },
    AreaEntry {
        area: FaultArea::Airbag,
        table: AIRBAG,
        label: Text::new("Airbag", "airbag"),
    },
    AreaEntry {
        area: FaultArea::Cluster,
        table: CLUSTER,
        label: Text::new("Kombiinstrument", "instrument cluster"),
    },
    AreaEntry {
        area: FaultArea::Comfort,
        table: COMFORT,
        label: Text::new("Komfort", "comfort"),
    },
    AreaEntry {
        area: FaultArea::Immobilizer,
        table: IMMOBILIZER,
        label: Text::new("Wegfahrsperre", "immobilizer"),
    },
    AreaEntry {
        area: FaultArea::Steering,
        table: STEERING,
        label: Text::new("Lenkung", "steering"),
    },
];

fn table_for(area: FaultArea) -> Option<Table> {
    AREA_TABLES
        .iter()
        .find(|entry| entry.area == area)
        .map(|entry| entry.table)
}

fn find(table: Table, code: i32) -> Option<&'static Text> {
    table.iter().find(|(c, _)| *c == code).map(|(_, text)| text)
}

/// Address (0x01 engine, 0x03 ABS, ...) to functional area.
pub fn area_for_ecu_address(address: i32) -> FaultArea {
    match address {
        // 0x11: engine 2 on eight-cylinder engines
        0x01 | 0x11 => FaultArea::Engine,
        0x02 | 0x12 => FaultArea::Transmission,
        // 0x13: distance control belongs to the brake area
        0x03 | 0x13 => FaultArea::Brakes,
        0x15 => FaultArea::Airbag,
        0x17 => FaultArea::Cluster,
        0x16 => FaultArea::Steering,
        0x25 => FaultArea::Immobilizer,
        // 0x42 driver door, 0x52 passenger door, 0x62 rear left, 0x72 rear right
        0x46 | 0x42 | 0x52 | 0x62 | 0x72 => FaultArea::Comfort,
        _ => FaultArea::Unknown,
    }
}

/// Conversion to the P-code, empty if the number is too small for that.
pub fn vag_code_to_p_code(vag_code: i32) -> String {
    if vag_code < P_CODE_OFFSET {
        return String::new();
    }
    let digits = vag_code - P_CODE_OFFSET;
    if !(0..=9999).contains(&digits) {
        return String::new();
    }
    format!("P{digits:04}")
}

/// Plain text for a VAG fault number. `ecu_address` is the address of the
/// control unit that reported the fault; -1 if unknown.
pub fn describe_vag_fault(vag_code: i32, ecu_address: i32) -> String {
    let p_code = vag_code_to_p_code(vag_code);
    if !p_code.is_empty() {
        return format!("{p_code} - {}", describe_dtc(&p_code));
    }

    let area = area_for_ecu_address(ecu_address);

    // The area of the reporting control unit takes precedence, because the
    // same number means something different there than elsewhere.
    if let Some(text) = table_for(area).and_then(|table| find(table, vag_code)) {
        return text.get().to_string();
    }

    if let Some(text) = find(SHARED, vag_code) {
        return text.get().to_string();
    }

    // Address unknown: search all areas and name the match, so it stays clear
    // that the assignment is only probable.
    if area == FaultArea::Unknown {
        for entry in &AREA_TABLES {
            if let Some(text) = find(entry.table, vag_code) {
                return format!(
                    "{}{}{})",
                    text.get(),
                    l(" (vermutlich, Bereich ", " (probably, area "),
                    entry.label.get()
                );
            }
        }
    }

    l("kein Klartext hinterlegt", "no description available").to_string()
}

/// How many old VAG numbers are stored in total.
pub fn vag_fault_count() -> usize {
    SHARED.len()
        + AREA_TABLES
            .iter()
            .map(|entry| entry.table.len())
            .sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{english, german};

    #[test]
    fn vag_numbers_are_translated_into_p_codes() {
        let _lang = german();
        // From 16384 upwards these are converted EOBD codes.
        assert_eq!(vag_code_to_p_code(16486), "P0102");
        assert_eq!(vag_code_to_p_code(16384), "P0000");
        assert_eq!(vag_code_to_p_code(17747), "P1363");
        // Below that there is no P-code.
        assert!(vag_code_to_p_code(561).is_empty());
        assert!(vag_code_to_p_code(0).is_empty());

        assert!(describe_vag_fault(16486, -1).contains("Luftmassenmesser"));
        assert!(describe_vag_fault(522, -1).contains("Kühlmitteltemperaturgeber"));
        assert!(describe_vag_fault(1249, -1).contains("Zylinder 1"));
        // Unknown numbers do not fall over.
        assert!(!describe_vag_fault(9999, -1).is_empty());
        assert!(vag_fault_count() > 60);
    }

    #[test]
    fn the_same_vag_number_means_something_different_per_control_unit() {
        let _lang = german();
        // This is the crux: 01276 is the brake light switch in the engine, the
        // hydraulic pump in the ABS. Without the address one of the two answers
        // would be wrong.
        assert!(describe_vag_fault(1276, 0x01).contains("Bremslichtschalter"));
        assert!(describe_vag_fault(1276, 0x03).contains("Hydraulikpumpe"));

        // And 01177: in the engine the control unit adaptation, in the
        // immobilizer the missing adaptation to the engine.
        assert!(describe_vag_fault(1177, 0x01).contains("Anpassung"));
        assert!(describe_vag_fault(1177, 0x25).contains("Motorsteuergerät"));

        // Wheel speed sensors only exist in the brake control unit.
        assert!(describe_vag_fault(283, 0x03).contains("vorne links"));
        assert!(describe_vag_fault(290, 0x03).contains("hinten links"));

        // Likewise, airbag igniters only exist there.
        assert!(describe_vag_fault(588, 0x15).contains("Fahrerairbag"));
        assert!(describe_vag_fault(1221, 0x15).contains("Crashsensor"));

        // Door control units belong to the comfort area, even under their own address.
        assert_eq!(area_for_ecu_address(0x42), FaultArea::Comfort);
        assert!(describe_vag_fault(1331, 0x42).contains("Türsteuergerät"));

        // Power supply applies everywhere, no matter which control unit reports it.
        assert!(describe_vag_fault(532, 0x01).contains("Klemme 30"));
        assert!(describe_vag_fault(532, 0x15).contains("Klemme 30"));
        assert!(describe_vag_fault(65535, 0x46).contains("Steuergerätefehler"));

        // Without an address it guesses, but labels the match as a guess.
        assert!(describe_vag_fault(283, -1).contains("vermutlich"));
        // Engine codes still stay unambiguous without an address because the
        // engine area is searched first.
        assert!(describe_vag_fault(1249, -1).contains("vermutlich"));
    }

    #[test]
    fn no_number_is_listed_twice_in_one_table() {
        // The count of stored numbers only means something without duplicates.
        for table in [
            SHARED,
            ENGINE,
            BRAKES,
            AIRBAG,
            CLUSTER,
            COMFORT,
            IMMOBILIZER,
            STEERING,
            TRANSMISSION,
        ] {
            let mut codes: Vec<i32> = table.iter().map(|(c, _)| *c).collect();
            codes.sort_unstable();
            codes.dedup();
            assert_eq!(codes.len(), table.len());
        }
    }

    #[test]
    fn a_guessed_area_is_named_in_both_languages() {
        {
            let _lang = german();
            assert!(describe_vag_fault(283, -1).ends_with("(vermutlich, Bereich Bremsen)"));
        }
        let _lang = english();
        assert!(describe_vag_fault(283, -1).ends_with("(probably, area brakes)"));
    }
}
