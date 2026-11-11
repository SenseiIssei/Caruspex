//! Where the control units of a car answer on CAN, per brand family.
//!
//! Each entry is one ECU with its request and response id, as used for UDS
//! (ISO 14229) through the diagnostic socket. The lists are maintained as CSV
//! in data/ecus/ and turned into Rust by tools/gen_ecus.py, like the vehicle
//! catalog. Only addresses with a reliable source go in; for unknown brands a
//! generic sweep over the usual 11-bit range is offered instead.

use caruspex_i18n::{Text, l};

use crate::generated::ecu_rows::ECU_ROWS;

/// The order of the variants is the scan order: engine, gearbox, brakes,
/// airbag, steering, instruments, gateway, then the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum EcuKind {
    Engine,
    Gearbox,
    Brakes,
    Airbag,
    Steering,
    Instruments,
    Gateway,
    Body,
    Climate,
    Parking,
    Lighting,
    Doors,
    Seats,
    Infotainment,
    Battery,
    Charger,
    Hybrid,
    #[default]
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EcuDefinition {
    /// E.g. 0x7E0, or 0x18DA10F1 for 29 bit.
    pub request: u32,
    /// E.g. 0x7E8.
    pub response: u32,
    pub extended29: bool,
    /// BMW style extended addressing (11 bit): every request goes to 6F1 and
    /// starts with this target byte, the answer comes from 6xx and starts
    /// with F1, the tester. -1 for normal addressing.
    pub extended_address: i32,
    pub kind: EcuKind,
    /// Display name in the active language.
    pub name: String,
    /// "VAG", "" for the generic sweep.
    pub family: String,
}

impl Default for EcuDefinition {
    /// Empty, with normal addressing (no extended address byte).
    fn default() -> Self {
        Self {
            request: 0,
            response: 0,
            extended29: false,
            extended_address: -1,
            kind: EcuKind::Other,
            name: String::new(),
            family: String::new(),
        }
    }
}

/// One row of the generated table (tools/gen_ecus.py).
#[derive(Clone, Copy, Debug)]
pub struct EcuRow {
    pub family: &'static str,
    pub request: u32,
    pub response: u32,
    pub extended29: bool,
    /// -1, or the BMW style target byte.
    pub extended_address: i32,
    pub kind: EcuKind,
    pub name: Text,
}

fn from_row(row: &EcuRow) -> EcuDefinition {
    EcuDefinition {
        request: row.request,
        response: row.response,
        extended29: row.extended29,
        extended_address: row.extended_address,
        kind: row.kind,
        name: row.name.get().to_string(),
        family: row.family.to_string(),
    }
}

fn sweep_entry(request: u32, kind: EcuKind) -> EcuDefinition {
    let id = format!("0x{request:03X}");
    let name = if kind == EcuKind::Other {
        format!("{}{id}", l("Steuergerät ", "Control unit "))
    } else {
        format!("{} ({id})", ecu_kind_name(kind))
    };
    EcuDefinition {
        request,
        response: request + 8,
        kind,
        name,
        ..Default::default()
    }
}

/// Plain name of a kind in the active language ("Bremsen und ABS").
pub fn ecu_kind_name(kind: EcuKind) -> &'static str {
    match kind {
        EcuKind::Engine => l("Motor", "Engine"),
        EcuKind::Gearbox => l("Getriebe", "Gearbox"),
        EcuKind::Brakes => l("Bremsen und ABS", "Brakes and ABS"),
        EcuKind::Airbag => l("Airbag", "Airbag"),
        EcuKind::Steering => l("Lenkung", "Steering"),
        EcuKind::Instruments => l("Kombiinstrument", "Instrument cluster"),
        EcuKind::Gateway => l("Gateway", "Gateway"),
        EcuKind::Body => l("Zentralelektronik", "Body electronics"),
        EcuKind::Climate => l("Klimaanlage", "Climate control"),
        EcuKind::Parking => l("Einparkhilfe", "Parking aid"),
        EcuKind::Lighting => l("Licht", "Lighting"),
        EcuKind::Doors => l("Türen", "Doors"),
        EcuKind::Seats => l("Sitze", "Seats"),
        EcuKind::Infotainment => l("Radio und Navigation", "Radio and navigation"),
        EcuKind::Battery => l("Batterie", "Battery"),
        EcuKind::Charger => l("Ladegerät", "Charger"),
        EcuKind::Hybrid => l("Hybrid- und Elektroantrieb", "Hybrid and electric drive"),
        EcuKind::Other => l("Sonstiges", "Other"),
    }
}

/// All ECUs known for a brand family key (see `brand_family()` in
/// `crate::dtc`), in a sensible scan order: engine first, then the
/// safety relevant ones. Empty if the family has no list.
pub fn ecu_definitions(family: &str) -> Vec<EcuDefinition> {
    if family.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<EcuDefinition> = ECU_ROWS
        .iter()
        .filter(|row| row.family == family)
        .map(from_row)
        .collect();
    // Stable, so that two units of the same kind keep their CSV order.
    out.sort_by_key(|d| d.kind);
    out
}

/// Families that have a list.
pub fn ecu_families() -> Vec<String> {
    // The generator writes the rows grouped by family, in the order of the
    // DTC database, so first appearance gives the display order.
    let mut out: Vec<String> = Vec::new();
    for row in ECU_ROWS {
        if !out.iter().any(|f| f == row.family) {
            out.push(row.family.to_string());
        }
    }
    out
}

/// The generic sweep: the OBD engine and gearbox addresses plus 0x700 to 0x7F7
/// with the response at request + 8, which many makers use. Slower, and some
/// ECUs will not show up, but it works without knowing the car.
pub fn generic_ecu_sweep() -> Vec<EcuDefinition> {
    let mut out = Vec::new();
    // The OBD addresses first: by ISO 15765-4 convention 7E0 is the engine
    // and 7E1 usually the gearbox, 7E2 to 7E7 are free for other units.
    for id in 0x7E0..=0x7E7u32 {
        let kind = match id {
            0x7E0 => EcuKind::Engine,
            0x7E1 => EcuKind::Gearbox,
            _ => EcuKind::Other,
        };
        out.push(sweep_entry(id, kind));
    }
    // Then the rest of the usual range. Three groups are left out on purpose:
    // 7E0 to 7E7 were asked above already, 7DF is the OBD broadcast address
    // (a request there reaches every unit at once, which must never happen
    // with a clear command), and 7E8 to 7EF are where the OBD units answer,
    // so sending on them would look like a reply to everyone on the bus.
    for id in 0x700..=0x7F7u32 {
        if (0x7E0..=0x7EF).contains(&id) {
            continue;
        }
        if id == 0x7DF {
            continue;
        }
        out.push(sweep_entry(id, EcuKind::Other));
    }
    out
}

/// BMW and Mini from the F series on (about 2007): every diagnostic address
/// 0x00 to 0x7F over the gateway, with the names of the known units. Slower
/// than the list, finds units the list does not know.
pub fn bmw_address_sweep() -> Vec<EcuDefinition> {
    let known = ecu_definitions("BMW");
    let mut out = Vec::new();
    // Known units first, in their scan order, then every other address.
    out.extend(known.iter().cloned());
    for address in 0x00..=0x7Fi32 {
        if known.iter().any(|d| d.extended_address == address) {
            continue;
        }
        out.push(EcuDefinition {
            request: 0x6F1,
            response: 0x600 + address as u32,
            extended_address: address,
            family: "BMW".to_string(),
            name: format!("{}0x{address:02X}", l("Steuergerät ", "Control unit ")),
            ..Default::default()
        });
    }
    out
}

/// The definition for a unit by its ids, as the UI keeps them: request 6F1
/// with a response 6xx means BMW style extended addressing.
pub fn ecu_from_ids(request: u32, response: u32, extended29: bool, name: &str) -> EcuDefinition {
    let mut d = EcuDefinition {
        request,
        response,
        extended29,
        name: name.to_string(),
        ..Default::default()
    };
    if !extended29 && request == 0x6F1 && (response & 0xF00) == 0x600 {
        d.extended_address = (response & 0xFF) as i32;
    }
    d
}

pub use crate::failure_types::failure_type_text;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::dtc::brand_families;
    use crate::test_support::{english, german};

    fn has_long_dash(text: &str) -> bool {
        text.contains('\u{2014}') || text.contains('\u{2013}')
    }

    fn id_fits(id: u32, extended29: bool) -> bool {
        id <= if extended29 { 0x1FFF_FFFF } else { 0x7FF }
    }

    const ALL_KINDS: [EcuKind; 18] = [
        EcuKind::Engine,
        EcuKind::Gearbox,
        EcuKind::Brakes,
        EcuKind::Airbag,
        EcuKind::Steering,
        EcuKind::Instruments,
        EcuKind::Gateway,
        EcuKind::Body,
        EcuKind::Climate,
        EcuKind::Parking,
        EcuKind::Lighting,
        EcuKind::Doors,
        EcuKind::Seats,
        EcuKind::Infotainment,
        EcuKind::Battery,
        EcuKind::Charger,
        EcuKind::Hybrid,
        EcuKind::Other,
    ];

    #[test]
    fn every_ecu_family_is_a_brand_family_of_the_dtc_database() {
        let _lang = german();
        let families = ecu_families();
        assert!(families.len() >= 5);
        let known = brand_families();
        let mut seen = HashSet::new();
        for f in &families {
            assert!(known.contains(f), "{f}");
            assert!(seen.insert(f.clone()), "{f}");
        }
        // The families the plan names first.
        for f in ["VAG", "FORD", "TOYOTA", "HYUNDAI", "PSA"] {
            assert!(families.iter().any(|x| x == f), "{f}");
        }
    }

    #[test]
    fn every_family_list_is_complete_and_consistent() {
        let _lang = german();
        for family in ecu_families() {
            let list = ecu_definitions(&family);
            assert!(!list.is_empty(), "{family}");
            assert_eq!(list[0].kind, EcuKind::Engine, "{family}");

            let mut requests = HashSet::new();
            let mut responses = HashSet::new();
            for (i, e) in list.iter().enumerate() {
                let what = format!("{family} {}", e.name);
                assert_eq!(e.family, family, "{what}");
                assert!(!e.name.is_empty(), "{what}");
                assert!(!has_long_dash(&e.name), "{what}");
                assert_ne!(e.request, 0, "{what}");
                assert_ne!(e.response, 0, "{what}");
                assert_ne!(e.request, e.response, "{what}");
                assert!(id_fits(e.request, e.extended29), "{what}");
                assert!(id_fits(e.response, e.extended29), "{what}");
                if !e.extended29 {
                    assert_ne!(e.request, 0x7DF, "{what}");
                }
                // With BMW style extended addressing all units share the request
                // id; there the target byte has to be unique (it is the low byte
                // of the response id, checked below).
                if e.extended_address < 0 {
                    assert!(requests.insert((e.extended29, e.request)), "{what}");
                } else {
                    assert_eq!(e.response, 0x600 + e.extended_address as u32, "{what}");
                }
                assert!(responses.insert((e.extended29, e.response)), "{what}");
                // Scan order: engine first, safety relevant units next.
                if i > 0 {
                    assert!(list[i - 1].kind <= e.kind, "{what}");
                }
            }
        }
    }

    #[test]
    fn unknown_families_have_no_list() {
        assert!(ecu_definitions("").is_empty());
        assert!(ecu_definitions("NOPE").is_empty());
        assert!(ecu_definitions("vag").is_empty()); // keys are exact
    }

    #[test]
    fn well_known_addresses_are_in_place() {
        let find = |family: &str, request: u32| {
            ecu_definitions(family)
                .into_iter()
                .find(|e| e.request == request)
                .unwrap_or_default()
        };
        let vag_abs = find("VAG", 0x713);
        assert_eq!(vag_abs.response, 0x77D);
        assert_eq!(vag_abs.kind, EcuKind::Brakes);
        assert_eq!(find("VAG", 0x710).kind, EcuKind::Gateway);
        assert_eq!(find("FORD", 0x760).response, 0x768);
        assert_eq!(find("FORD", 0x737).kind, EcuKind::Airbag);
        assert_eq!(find("GM", 0x7E2).kind, EcuKind::Gearbox);
        assert_eq!(find("PSA", 0x752).response, 0x652);
    }

    #[test]
    fn names_follow_the_active_language() {
        {
            let _lang = english();
            let list = ecu_definitions("VAG");
            assert_eq!(list[0].name, "Engine control");
        }
        let _lang = german();
        let list = ecu_definitions("VAG");
        assert_eq!(list[0].name, "Motorsteuerung");
    }

    #[test]
    fn every_kind_has_a_name_in_both_languages() {
        for make in [crate::test_support::german, crate::test_support::english] {
            let _lang = make();
            let mut names = HashSet::new();
            for k in ALL_KINDS {
                let name = ecu_kind_name(k);
                assert!(!name.is_empty());
                assert!(!has_long_dash(name));
                assert!(names.insert(name), "{name}");
            }
        }
        {
            let _lang = german();
            assert_eq!(ecu_kind_name(EcuKind::Brakes), "Bremsen und ABS");
        }
        let _lang = english();
        assert_eq!(ecu_kind_name(EcuKind::Brakes), "Brakes and ABS");
    }

    #[test]
    fn the_generic_sweep_covers_the_usual_11_bit_range() {
        let _lang = german();
        let sweep = generic_ecu_sweep();
        // 7E0 to 7E7, then 0x700 to 0x7F7 without 7E0 to 7EF and without 7DF.
        assert_eq!(sweep.len(), 8 + (0xF8 - 16 - 1));

        for (i, e) in sweep.iter().take(8).enumerate() {
            assert_eq!(e.request, 0x7E0 + i as u32);
        }
        assert_eq!(sweep[0].kind, EcuKind::Engine);
        assert_eq!(sweep[1].kind, EcuKind::Gearbox);
        assert_eq!(sweep[2].kind, EcuKind::Other);

        let mut requests = HashSet::new();
        for e in &sweep {
            let what = format!("{:#x}", e.request);
            assert_eq!(e.response, e.request + 8, "{what}");
            assert!(!e.extended29, "{what}");
            assert!(e.family.is_empty(), "{what}");
            assert!(e.request >= 0x700, "{what}");
            assert!(e.request <= 0x7F7, "{what}");
            assert_ne!(e.request, 0x7DF, "{what}");
            assert!(!e.name.is_empty(), "{what}");
            assert!(requests.insert(e.request), "{what}");
            assert!(
                !(0x7E8..=0x7EF).contains(&e.request),
                "OBD response id used as request {what}"
            );
            if e.request > 0x7E7 {
                assert_eq!(e.kind, EcuKind::Other, "{what}");
            }
        }
        assert!(requests.contains(&0x700));
        assert!(requests.contains(&0x7F7));
    }

    #[test]
    fn sweep_names_carry_the_address() {
        let find = |request: u32| {
            generic_ecu_sweep()
                .into_iter()
                .find(|e| e.request == request)
                .map(|e| e.name)
                .unwrap_or_default()
        };
        {
            let _lang = german();
            assert_eq!(find(0x760), "Steuergerät 0x760");
            assert_eq!(find(0x7E0), "Motor (0x7E0)");
        }
        let _lang = english();
        assert_eq!(find(0x760), "Control unit 0x760");
        assert_eq!(find(0x7E1), "Gearbox (0x7E1)");
    }

    #[test]
    fn bmw_address_list_and_full_sweep() {
        let _lang = german();
        let list = ecu_definitions("BMW");
        assert!(list.len() >= 20);
        assert_eq!(list[0].kind, EcuKind::Engine);
        assert_eq!(list[0].extended_address, 0x12);
        for d in &list {
            assert_eq!(d.request, 0x6F1);
            assert_eq!(d.response, 0x600 + d.extended_address as u32);
            assert!(!d.extended29);
        }
        let sweep = bmw_address_sweep();
        assert_eq!(sweep.len(), 128);
        let addresses: std::collections::BTreeSet<i32> =
            sweep.iter().map(|d| d.extended_address).collect();
        assert_eq!(addresses.len(), 128);
        assert_eq!(*addresses.first().unwrap(), 0x00);
        assert_eq!(*addresses.last().unwrap(), 0x7F);
        // The known units keep their names and come first.
        assert_eq!(sweep[0].extended_address, 0x12);
        assert_eq!(sweep[0].name, list[0].name);
    }

    #[test]
    fn ids_from_the_ui_map_to_bmw_addressing() {
        let bmw = ecu_from_ids(0x6F1, 0x612, false, "DME");
        assert_eq!(bmw.extended_address, 0x12);
        assert_eq!(bmw.name, "DME");
        let plain = ecu_from_ids(0x7E0, 0x7E8, false, "Motor");
        assert_eq!(plain.extended_address, -1);
        let wide = ecu_from_ids(0x6F1, 0x612, true, "x");
        assert_eq!(wide.extended_address, -1);
    }
}
