//! Maps a trouble code to the spot on the vehicle where you have to look.

use caruspex_i18n::{Text, l};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum CarZone {
    #[default]
    Unknown,
    /// Intake system, mass air flow sensor, intake manifold, vacuum leaks.
    Intake,
    /// Throttle body and accelerator pedal.
    Throttle,
    /// Injectors, fuel pressure in the engine bay.
    Injection,
    /// Ignition coils, spark plugs, knock sensor.
    Ignition,
    /// Combustion, timing drive, camshaft.
    Cylinders,
    /// Coolant, thermostat, fans.
    Cooling,
    /// Turbocharger and boost pressure control.
    Turbo,
    /// Manifold, oxygen sensors, catalytic converter, EGR, particulate filter.
    Exhaust,
    /// Tank, fuel pump, EVAP system.
    FuelTank,
    /// Transmission.
    Gearbox,
    /// The control unit itself.
    Ecu,
    /// Battery, alternator, electrical system.
    Electrics,
    /// Data bus between the control units.
    Network,
    /// Wheel speed sensors, ABS, brakes.
    Wheels,
    /// Airbags, comfort system, instrument cluster.
    Cabin,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ZoneInfo {
    pub zone: CarZone,
    /// "Ansaugtrakt" / "Intake system", active language.
    pub name: String,
    /// Where exactly to look, active language.
    pub hint: String,
}

struct Range {
    lo: i32,
    hi: i32,
    zone: CarZone,
    hint: Text,
}

const fn range(lo: i32, hi: i32, zone: CarZone, de: &'static str, en: &'static str) -> Range {
    Range {
        lo,
        hi,
        zone,
        hint: Text::new(de, en),
    }
}

/// The EOBD number ranges are ordered by assembly. We use that to infer the
/// spot on the vehicle from the number.
static POWERTRAIN: &[Range] = &[
    range(
        0x0010,
        0x002F,
        CarZone::Cylinders,
        "Nockenwellenversteller und Steuertrieb, Kette oder Riemen prüfen",
        "Camshaft adjusters and timing drive, check the chain or belt",
    ),
    range(
        0x0030,
        0x006F,
        CarZone::Exhaust,
        "Lambdasonden und ihre Heizung, vor und nach dem Katalysator",
        "Oxygen sensors and their heaters, before and after the catalytic converter",
    ),
    range(
        0x0070,
        0x008F,
        CarZone::Intake,
        "Außentemperatur- und Ansauggeber",
        "Ambient temperature and intake sensors",
    ),
    range(
        0x0087,
        0x009F,
        CarZone::FuelTank,
        "Kraftstoffdruck, Pumpe und Filter",
        "Fuel pressure, pump and filter",
    ),
    range(
        0x0100,
        0x0104,
        CarZone::Intake,
        "Luftmassenmesser im Ansaugrohr",
        "Mass air flow sensor in the intake duct",
    ),
    range(
        0x0105,
        0x0109,
        CarZone::Intake,
        "Saugrohrdrucksensor",
        "Intake manifold pressure sensor",
    ),
    range(
        0x0110,
        0x0114,
        CarZone::Intake,
        "Ansauglufttemperaturgeber",
        "Intake air temperature sensor",
    ),
    range(
        0x0115,
        0x0119,
        CarZone::Cooling,
        "Kühlmitteltemperaturgeber am Motor",
        "Coolant temperature sensor on the engine",
    ),
    range(
        0x0120,
        0x012F,
        CarZone::Throttle,
        "Drosselklappe und Fahrpedalgeber",
        "Throttle body and accelerator pedal sensor",
    ),
    range(
        0x0130,
        0x016F,
        CarZone::Exhaust,
        "Lambdasonden im Abgasstrang",
        "Oxygen sensors in the exhaust system",
    ),
    range(
        0x0170,
        0x017F,
        CarZone::Intake,
        "Gemischbildung: Falschluft, Schläuche, Kurbelgehäuseentlüftung",
        "Mixture formation: vacuum leaks, hoses, crankcase ventilation",
    ),
    range(
        0x0180,
        0x019F,
        CarZone::FuelTank,
        "Kraftstoffdruck und Temperatur",
        "Fuel pressure and temperature",
    ),
    range(
        0x0200,
        0x0214,
        CarZone::Injection,
        "Einspritzventile am Zylinderkopf",
        "Injectors on the cylinder head",
    ),
    range(
        0x0215,
        0x021F,
        CarZone::Injection,
        "Einspritzanlage",
        "Fuel injection system",
    ),
    range(
        0x0220,
        0x022F,
        CarZone::Throttle,
        "Drosselklappensensoren",
        "Throttle position sensors",
    ),
    range(
        0x0230,
        0x025F,
        CarZone::Turbo,
        "Lader, Ladedruckdose und Unterdruckschläuche",
        "Turbocharger, wastegate actuator and vacuum hoses",
    ),
    range(
        0x0260,
        0x0299,
        CarZone::Injection,
        "Einspritzventile, Zylinder einzeln",
        "Injectors, individual cylinders",
    ),
    range(
        0x0300,
        0x0312,
        CarZone::Cylinders,
        "Verbrennungsaussetzer: Kerzen, Spulen, Kompression, Einspritzventil",
        "Misfires: spark plugs, coils, compression, injector",
    ),
    range(
        0x0320,
        0x0349,
        CarZone::Ignition,
        "Kurbel- und Nockenwellengeber, Klopfsensoren",
        "Crankshaft and camshaft position sensors, knock sensors",
    ),
    range(
        0x0350,
        0x036F,
        CarZone::Ignition,
        "Zündspulen und Zündkerzen",
        "Ignition coils and spark plugs",
    ),
    range(
        0x0370,
        0x039F,
        CarZone::Cylinders,
        "Steuerzeiten und Drehzahlgeber",
        "Valve timing and engine speed sensor",
    ),
    range(
        0x0400,
        0x040F,
        CarZone::Exhaust,
        "Abgasrückführung, Ventil meist verrußt",
        "Exhaust gas recirculation, valve usually sooted up",
    ),
    range(
        0x0410,
        0x041F,
        CarZone::Exhaust,
        "Sekundärluftsystem",
        "Secondary air system",
    ),
    range(
        0x0420,
        0x043F,
        CarZone::Exhaust,
        "Katalysator und die Sonden davor und dahinter",
        "Catalytic converter and the sensors before and after it",
    ),
    range(
        0x0440,
        0x045F,
        CarZone::FuelTank,
        "Tankentlüftung: zuerst den Tankdeckel prüfen, dann die Schläuche",
        "EVAP system: check the fuel cap first, then the hoses",
    ),
    range(
        0x0460,
        0x047F,
        CarZone::FuelTank,
        "Tankgeber und Füllstand",
        "Fuel level sender and fuel level",
    ),
    range(
        0x0480,
        0x048F,
        CarZone::Cooling,
        "Kühlerlüfter und deren Relais",
        "Radiator fans and their relays",
    ),
    range(
        0x0490,
        0x04FF,
        CarZone::Exhaust,
        "Abgasnachbehandlung",
        "Exhaust aftertreatment",
    ),
    range(
        0x0500,
        0x0504,
        CarZone::Wheels,
        "Geschwindigkeitssignal von den Raddrehzahlfühlern",
        "Vehicle speed signal from the wheel speed sensors",
    ),
    range(
        0x0505,
        0x0510,
        CarZone::Throttle,
        "Leerlaufregelung, Drosselklappe verschmutzt?",
        "Idle control, is the throttle body dirty?",
    ),
    range(
        0x0511,
        0x051F,
        CarZone::Throttle,
        "Leerlaufsteller",
        "Idle actuator",
    ),
    range(
        0x0520,
        0x052F,
        CarZone::Ecu,
        "Öldruckgeber",
        "Oil pressure sensor",
    ),
    range(
        0x0530,
        0x055F,
        CarZone::Cooling,
        "Klimadruck und Kühlkreis",
        "A/C pressure and refrigerant circuit",
    ),
    range(
        0x0560,
        0x056F,
        CarZone::Electrics,
        "Bordspannung: Batterie und Lichtmaschine",
        "Battery voltage: battery and alternator",
    ),
    range(
        0x0570,
        0x059F,
        CarZone::Electrics,
        "Schalter und Bordnetz",
        "Switches and electrical system",
    ),
    range(
        0x0600,
        0x062F,
        CarZone::Ecu,
        "Motorsteuergerät selbst",
        "The engine control unit itself",
    ),
    range(
        0x0630,
        0x064F,
        CarZone::Ecu,
        "Steuergerät und angesteuerte Relais",
        "Control unit and the relays it drives",
    ),
    range(
        0x0650,
        0x069F,
        CarZone::Electrics,
        "Ausgänge und Relais",
        "Outputs and relays",
    ),
    range(
        0x0700,
        0x08FF,
        CarZone::Gearbox,
        "Getriebe und dessen Steuergerät",
        "Transmission and its control unit",
    ),
    range(
        0x1000,
        0x10FF,
        CarZone::Ecu,
        "herstellerspezifisch, Steuergerät",
        "manufacturer-specific, control unit",
    ),
    range(
        0x1100,
        0x11FF,
        CarZone::Exhaust,
        "herstellerspezifisch, Lambdaregelung",
        "manufacturer-specific, lambda control",
    ),
    range(
        0x1120,
        0x11FF,
        CarZone::Intake,
        "herstellerspezifisch, Gemisch",
        "manufacturer-specific, mixture",
    ),
    range(
        0x1500,
        0x1557,
        CarZone::Turbo,
        "herstellerspezifisch, Ladedruck",
        "manufacturer-specific, boost pressure",
    ),
    range(
        0x1558,
        0x159F,
        CarZone::Throttle,
        "herstellerspezifisch, Leerlaufregelung und Drosselklappenanpassung",
        "manufacturer-specific, idle control and throttle body adaptation",
    ),
    range(
        0x1600,
        0x16FF,
        CarZone::Network,
        "herstellerspezifisch, Datenbus und Bordspannung",
        "manufacturer-specific, data bus and battery voltage",
    ),
    range(
        0x1700,
        0x17FF,
        CarZone::Gearbox,
        "herstellerspezifisch, Getriebe",
        "manufacturer-specific, transmission",
    ),
    range(
        0x2000,
        0x20FF,
        CarZone::Exhaust,
        "Abgasnachbehandlung, SCR und AdBlue",
        "Exhaust aftertreatment, SCR and AdBlue",
    ),
    range(
        0x2100,
        0x213F,
        CarZone::Throttle,
        "Drosselklappensteller und Fahrpedal",
        "Throttle actuator and accelerator pedal",
    ),
    range(
        0x2140,
        0x23FF,
        CarZone::Injection,
        "Einspritzung und Gemisch",
        "Fuel injection and mixture",
    ),
    range(
        0x2400,
        0x24FF,
        CarZone::Exhaust,
        "Partikelfilter und Differenzdruck",
        "Particulate filter and differential pressure",
    ),
    range(
        0x2500,
        0x2FFF,
        CarZone::Ecu,
        "Steuergerät und Peripherie",
        "Control unit and peripherals",
    ),
];

fn zone_for(letter: char, number: i32) -> (CarZone, &'static str) {
    match letter {
        'C' => (
            CarZone::Wheels,
            l(
                "Raddrehzahlfühler, ABS-Hydraulik und die Bremsen",
                "Wheel speed sensors, ABS hydraulics and the brakes",
            ),
        ),
        'B' => (
            CarZone::Cabin,
            l(
                "Innenraum: Airbag, Gurtstraffer, Komfortelektronik",
                "Cabin: airbags, seat belt pretensioners, comfort electronics",
            ),
        ),
        'U' => (
            CarZone::Network,
            l(
                "Datenbus zwischen den Steuergeräten, oft nur ein Steckerproblem",
                "Data bus between the control units, often just a connector problem",
            ),
        ),
        _ => {
            for r in POWERTRAIN {
                if number >= r.lo && number <= r.hi {
                    return (r.zone, r.hint.get());
                }
            }
            (
                CarZone::Unknown,
                l("keine Zuordnung hinterlegt", "no location on file"),
            )
        }
    }
}

/// Zone name in the active language.
pub fn zone_name(zone: CarZone) -> &'static str {
    match zone {
        CarZone::Intake => l("Ansaugtrakt", "Intake system"),
        CarZone::Throttle => l("Drosselklappe", "Throttle body"),
        CarZone::Injection => l("Einspritzung", "Fuel injection"),
        CarZone::Ignition => l("Zündanlage", "Ignition system"),
        CarZone::Cylinders => l("Motor und Steuertrieb", "Engine and timing drive"),
        CarZone::Cooling => l("Kühlung", "Cooling system"),
        CarZone::Turbo => l("Turbolader", "Turbocharger"),
        CarZone::Exhaust => l("Abgasanlage", "Exhaust system"),
        CarZone::FuelTank => l("Kraftstoffanlage", "Fuel system"),
        CarZone::Gearbox => l("Getriebe", "Transmission"),
        CarZone::Ecu => l("Steuergerät", "Control unit"),
        CarZone::Electrics => l("Bordnetz", "Electrical system"),
        CarZone::Network => l("Datenbus", "Data bus"),
        CarZone::Wheels => l("Räder und Bremsen", "Wheels and brakes"),
        CarZone::Cabin => l("Innenraum", "Cabin"),
        CarZone::Unknown => l("nicht zugeordnet", "not assigned"),
    }
}

/// All zones, in the order the drawing knows them.
pub fn all_zones() -> &'static [CarZone] {
    static ZONES: [CarZone; 15] = [
        CarZone::Intake,
        CarZone::Throttle,
        CarZone::Injection,
        CarZone::Ignition,
        CarZone::Cylinders,
        CarZone::Cooling,
        CarZone::Turbo,
        CarZone::Exhaust,
        CarZone::FuelTank,
        CarZone::Gearbox,
        CarZone::Ecu,
        CarZone::Electrics,
        CarZone::Network,
        CarZone::Wheels,
        CarZone::Cabin,
    ];
    &ZONES
}

/// Zone for a code like "P0171".
pub fn locate_dtc(code: &str) -> ZoneInfo {
    if code.len() != 5 || !code.is_ascii() {
        return ZoneInfo::default();
    }
    // strtol with base 16 stops at the first character that is no hex digit.
    let digits: String = code[1..]
        .chars()
        .take_while(char::is_ascii_hexdigit)
        .collect();
    let number = i32::from_str_radix(&digits, 16).unwrap_or(0);
    let letter = code.as_bytes()[0] as char;
    let (zone, hint) = zone_for(letter, number);
    ZoneInfo {
        zone,
        name: zone_name(zone).to_string(),
        hint: hint.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::german;

    #[test]
    fn trouble_codes_point_to_the_right_spot_on_the_vehicle() {
        let _lang = german();
        assert_eq!(locate_dtc("P0507").zone, CarZone::Throttle);
        assert_eq!(locate_dtc("P1559").zone, CarZone::Throttle); // not the turbo
        assert_eq!(locate_dtc("P1555").zone, CarZone::Turbo);
        assert_eq!(locate_dtc("P0301").zone, CarZone::Cylinders);
        assert_eq!(locate_dtc("P0420").zone, CarZone::Exhaust);
        assert_eq!(locate_dtc("P0442").zone, CarZone::FuelTank);
        assert_eq!(locate_dtc("P0118").zone, CarZone::Cooling);
        assert_eq!(locate_dtc("C0035").zone, CarZone::Wheels);
        assert_eq!(locate_dtc("B0001").zone, CarZone::Cabin);
        assert_eq!(locate_dtc("U0121").zone, CarZone::Network);
        assert_eq!(locate_dtc("P0700").zone, CarZone::Gearbox);
        // Every zone comes with a hint where to look.
        assert!(!locate_dtc("P0507").hint.is_empty());
        assert!(!locate_dtc("P0507").name.is_empty());
    }

    #[test]
    fn every_zone_has_a_name_and_odd_input_gives_unknown() {
        let _lang = german();
        for zone in all_zones() {
            assert!(!zone_name(*zone).is_empty());
        }
        assert_eq!(locate_dtc("P01").zone, CarZone::Unknown);
        assert_eq!(locate_dtc("P0001").hint, "keine Zuordnung hinterlegt");
    }
}
