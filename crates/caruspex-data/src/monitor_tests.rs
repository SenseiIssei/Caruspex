//! Plain text and conversion for on-board monitoring, modes 05 and 06.
//! Names and units come out in the active language (see `caruspex_i18n`).

use caruspex_i18n::l;

fn hex2(value: i32) -> String {
    format!("{:02X}", value & 0xFF)
}

/// Mode 06 on the K-Line: the test numbers are manufacturer-specific, but a few
/// ranges are standardized.
///
/// On the K-Line every manufacturer assigns the test numbers itself. Only the
/// ranges for oxygen sensors and catalytic converter are standardized, and
/// those are the ones you need in everyday use.
pub fn monitor_test_name(test_id: i32) -> String {
    let known = match test_id {
        0x01 => l(
            "Lambdasonde Bank 1 Sonde 1, Spannungsschwelle mager nach fett",
            "O2 sensor bank 1 sensor 1, lean-to-rich threshold voltage",
        ),
        0x02 => l(
            "Lambdasonde Bank 1 Sonde 1, Spannungsschwelle fett nach mager",
            "O2 sensor bank 1 sensor 1, rich-to-lean threshold voltage",
        ),
        0x03 => l(
            "Lambdasonde Bank 1 Sonde 1, tiefste Spannung",
            "O2 sensor bank 1 sensor 1, minimum voltage",
        ),
        0x04 => l(
            "Lambdasonde Bank 1 Sonde 1, höchste Spannung",
            "O2 sensor bank 1 sensor 1, maximum voltage",
        ),
        0x05 => l(
            "Lambdasonde Bank 1 Sonde 1, Umschaltzeit",
            "O2 sensor bank 1 sensor 1, switch time",
        ),
        0x06 => l(
            "Lambdasonde Bank 1 Sonde 1, Periodendauer",
            "O2 sensor bank 1 sensor 1, period",
        ),
        0x07 => l(
            "Lambdasonde Bank 1 Sonde 2, Spannungsschwelle mager nach fett",
            "O2 sensor bank 1 sensor 2, lean-to-rich threshold voltage",
        ),
        0x08 => l(
            "Lambdasonde Bank 1 Sonde 2, Spannungsschwelle fett nach mager",
            "O2 sensor bank 1 sensor 2, rich-to-lean threshold voltage",
        ),
        0x09 => l(
            "Lambdasonde Bank 1 Sonde 2, tiefste Spannung",
            "O2 sensor bank 1 sensor 2, minimum voltage",
        ),
        0x0A => l(
            "Lambdasonde Bank 1 Sonde 2, höchste Spannung",
            "O2 sensor bank 1 sensor 2, maximum voltage",
        ),
        0x21 => l(
            "Katalysator Bank 1, Wirkungsgrad",
            "Catalyst bank 1, efficiency",
        ),
        0x22 => l(
            "Katalysator Bank 2, Wirkungsgrad",
            "Catalyst bank 2, efficiency",
        ),
        0x31 => l(
            "Tankentlüftung, Leckprüfung",
            "Evaporative system, leak check",
        ),
        0x32 => l("Tankentlüftung, Durchfluss", "Evaporative system, flow"),
        0x39 => l("Abgasrückführung, Durchfluss", "EGR, flow"),
        0x3A => l("Sekundärluft, Durchfluss", "Secondary air, flow"),
        0x41 => l(
            "Lambdasondenheizung Bank 1 Sonde 1",
            "O2 sensor heater bank 1 sensor 1",
        ),
        0x42 => l(
            "Lambdasondenheizung Bank 1 Sonde 2",
            "O2 sensor heater bank 1 sensor 2",
        ),
        0x81 => l(
            "Verbrennungsaussetzer, Zähler Zylinder 1",
            "Misfire, counter cylinder 1",
        ),
        0x82 => l(
            "Verbrennungsaussetzer, Zähler Zylinder 2",
            "Misfire, counter cylinder 2",
        ),
        0x83 => l(
            "Verbrennungsaussetzer, Zähler Zylinder 3",
            "Misfire, counter cylinder 3",
        ),
        0x84 => l(
            "Verbrennungsaussetzer, Zähler Zylinder 4",
            "Misfire, counter cylinder 4",
        ),
        _ => {
            return format!(
                "Test 0x{}{}",
                hex2(test_id),
                l(" (herstellerspezifisch)", " (manufacturer-specific)")
            );
        }
    };
    known.to_string()
}

/// Mode 06 on CAN: standardized identifiers for the monitored component
/// (OBDMID), e.g. 0x21 catalyst bank 1, 0xA2 misfire cylinder 1.
///
/// The standardized monitor IDs of SAE J1979 come in blocks, so the names are
/// put together from bank and sensor.
pub fn monitor_component_name(id: i32) -> String {
    let bank_sensor = |index: i32, per_bank: i32| {
        format!(
            "{}{}{}{}",
            l("Bank ", "bank "),
            index / per_bank + 1,
            l(" Sonde ", " sensor "),
            index % per_bank + 1
        )
    };
    let bank = |index: i32| format!("{}{}", l(" Bank ", " bank "), index + 1);

    if (0x01..=0x10).contains(&id) {
        return format!(
            "{}{}",
            l("Lambdasonde ", "O2 sensor "),
            bank_sensor(id - 0x01, 4)
        );
    }
    if (0x21..=0x24).contains(&id) {
        return format!("{}{}", l("Katalysator", "Catalyst"), bank(id - 0x21));
    }
    if (0x31..=0x34).contains(&id) {
        return format!("{}{}", l("Abgasrückführung", "EGR"), bank(id - 0x31));
    }
    if (0x35..=0x38).contains(&id) {
        return format!(
            "{}{}",
            l("Nockenwellenverstellung", "Variable valve timing"),
            bank(id - 0x35)
        );
    }
    let evap = match id {
        0x39 => Some(l(
            "Tankentlüftung, Deckel offen oder großes Leck",
            "Evaporative system, cap off or large leak",
        )),
        0x3A => Some(l(
            "Tankentlüftung, Leck 0,090 Zoll",
            "Evaporative system, 0.090 inch leak",
        )),
        0x3B => Some(l(
            "Tankentlüftung, Leck 0,040 Zoll",
            "Evaporative system, 0.040 inch leak",
        )),
        0x3C => Some(l(
            "Tankentlüftung, Leck 0,020 Zoll",
            "Evaporative system, 0.020 inch leak",
        )),
        0x3D => Some(l(
            "Tankentlüftung, Spülstrom",
            "Evaporative system, purge flow",
        )),
        _ => None,
    };
    if let Some(text) = evap {
        return text.to_string();
    }
    if (0x41..=0x50).contains(&id) {
        return format!(
            "{}{}",
            l("Lambdasondenheizung ", "O2 sensor heater "),
            bank_sensor(id - 0x41, 4)
        );
    }
    if (0x61..=0x64).contains(&id) {
        return format!(
            "{}{}",
            l("Katalysatorheizung", "Heated catalyst"),
            bank(id - 0x61)
        );
    }
    if (0x71..=0x74).contains(&id) {
        return format!("{}{}", l("Sekundärluft ", "Secondary air "), id - 0x70);
    }
    if (0x81..=0x84).contains(&id) {
        return format!(
            "{}{}",
            l("Kraftstoffsystem", "Fuel system"),
            bank(id - 0x81)
        );
    }
    if (0x85..=0x88).contains(&id) {
        return format!(
            "{}{}",
            l("Ladedruckregelung", "Boost pressure control"),
            bank(id - 0x85)
        );
    }
    if (0x90..=0x91).contains(&id) {
        return format!(
            "{}{}",
            l("NOx-Speicherkatalysator", "NOx adsorber"),
            bank(id - 0x90)
        );
    }
    if (0x98..=0x99).contains(&id) {
        return format!(
            "{}{}",
            l("Stickoxid-Katalysator (SCR)", "NOx catalyst (SCR)"),
            bank(id - 0x98)
        );
    }
    if id == 0xA1 {
        return l(
            "Verbrennungsaussetzer, ganzer Motor",
            "Misfire, whole engine",
        )
        .to_string();
    }
    if (0xA2..=0xAD).contains(&id) {
        return format!(
            "{}{}",
            l("Verbrennungsaussetzer Zylinder ", "Misfire cylinder "),
            id - 0xA1
        );
    }
    if (0xB0..=0xB1).contains(&id) {
        return format!(
            "{}{}",
            l("Partikelfilter", "Particulate filter"),
            bank(id - 0xB0)
        );
    }
    format!("{}{}", l("Baugruppe 0x", "Component 0x"), hex2(id))
}

/// Mode 06 on CAN: name of a test (TID) within a monitor (OBDMID).
///
/// On CAN the tests of a monitor are numbered by TID. 0x01 to 0x0A are the
/// same oxygen sensor tests as in mode 05, 0x0B and 0x0C belong to the
/// misfire monitors, from 0x80 the manufacturer decides.
pub fn monitor_test_name_can(monitor_id: i32, test_id: i32) -> String {
    if (0xA1..=0xAD).contains(&monitor_id) {
        if test_id == 0x0B {
            return l(
                "Aussetzer im Schnitt der letzten 10 Fahrten",
                "Misfires, average of the last 10 trips",
            )
            .to_string();
        }
        if test_id == 0x0C {
            return l(
                "Aussetzer in der aktuellen Fahrt",
                "Misfires in the current trip",
            )
            .to_string();
        }
    }
    if (0x01..=0x0A).contains(&test_id) && (0x01..=0x10).contains(&monitor_id) {
        return oxygen_test_name(test_id);
    }
    format!(
        "Test 0x{}{}",
        hex2(test_id),
        l(" (herstellerspezifisch)", " (manufacturer-specific)")
    )
}

/// A value together with its unit, written unit first like the standard's
/// table reads.
fn s(unit: &'static str, value: f64) -> (f64, &'static str) {
    (value, unit)
}

/// Converts the raw value of a CAN test into a number and unit. The
/// identifier determines the formula.
///
/// On CAN a separate byte, the unit and scaling ID of SAE J1979 (appendix
/// "Unit and Scaling definitions for OBDMIDs"), says how the raw value is to
/// be read. IDs 0x01 to 0x7F are unsigned, 0x80 to 0xFE read the same two
/// bytes as a signed number.
pub fn monitor_scale(unit_and_scaling: i32, raw: i32) -> (f64, &'static str) {
    let u = f64::from(raw & 0xFFFF);
    let sg = f64::from(if raw & 0x8000 != 0 {
        (raw & 0xFFFF) - 0x10000
    } else {
        raw & 0xFFFF
    });
    let deg_c = l("Grad C", "deg C");
    let deg = l("Grad", "deg");
    match unit_and_scaling {
        // --- unsigned ---
        0x01 => s("", u),
        0x02 => s("", u * 0.1),
        0x03 => s("", u * 0.01),
        0x04 => s("", u * 0.001),
        0x05 => s("", u * 0.0000305),
        0x06 => s("", u * 0.000305),
        0x07 => s("1/min", u * 0.25),
        0x08 => s("km/h", u * 0.01),
        0x09 => s("km/h", u),
        0x0A => s("V", u * 0.000122),
        0x0B => s("V", u * 0.001),
        0x0C => s("V", u * 0.01),
        0x0D => s("mA", u * 0.00390625),
        0x0E => s("A", u * 0.001),
        0x0F => s("A", u * 0.01),
        0x10 => s("ms", u),
        0x11 => s("s", u * 0.1),
        0x12 => s("s", u),
        0x13 => s("Ohm", u * 0.001),
        0x14 => s("Ohm", u),
        0x15 => s("kOhm", u),
        0x16 => s(deg_c, u * 0.1 - 40.0),
        0x17 => s("kPa", u * 0.01),
        0x18 => s("kPa", u * 0.0117),
        0x19 => s("kPa", u * 0.079),
        0x1A => s("kPa", u),
        0x1B => s("kPa", u * 10.0),
        0x1C => s(deg, u * 0.01),
        0x1D => s(deg, u * 0.5),
        0x1E => s("lambda", u * 0.0000305),
        0x1F => s("A/F", u * 0.05),
        0x20 => s("", u * 0.0039062),
        0x21 => s("mHz", u),
        0x22 => s("Hz", u),
        0x23 => s("kHz", u),
        0x24 => s("", u), // counts, e.g. misfires
        0x25 => s("km", u),
        0x26 => s("mV/ms", u * 0.1),
        0x27 => s("g/s", u * 0.01),
        0x28 => s("g/s", u),
        0x29 => s("Pa/s", u * 0.25),
        0x2A => s("kg/h", u * 0.001),
        0x2B => s("", u), // switch counts
        0x2C => s("g/cyl", u * 0.01),
        0x2D => s("mg/stroke", u * 0.01),
        0x2E => s("", if u != 0.0 { 1.0 } else { 0.0 }), // true/false
        0x2F => s("%", u * 0.01),
        0x30 => s("%", u * 0.001526),
        0x31 => s("l", u * 0.001),
        0x32 => s("inch", u * 0.0000305),
        0x33 => s("lambda", u * 0.00024414),
        0x34 => s("min", u),
        0x35 => s("ms", u * 10.0),
        0x36 => s("g", u * 0.01),
        0x37 => s("g", u * 0.1),
        0x38 => s("g", u),
        0x39 => s("%", u * 0.01 - 327.68),
        0x3A => s("g", u * 0.001),
        0x3B => s("g", u * 0.0001),
        0x3C => s("us", u * 0.1),
        0x3D => s("mA", u * 0.01),
        0x3E => s("mm2", u * 0.00006103516),
        0x3F => s("l", u * 0.01),
        0x40 => s("ppm", u),
        0x41 => s("uA", u * 0.01),
        // --- signed ---
        0x81 => s("", sg),
        0x82 => s("", sg * 0.1),
        0x83 => s("", sg * 0.01),
        0x84 => s("", sg * 0.001),
        0x85 => s("", sg * 0.0000305),
        0x86 => s("", sg * 0.000305),
        0x87 => s("ppm", sg),
        0x8A => s("V", sg * 0.000122),
        0x8B => s("V", sg * 0.001),
        0x8C => s("V", sg * 0.01),
        0x8D => s("mA", sg * 0.00390625),
        0x8E => s("A", sg * 0.001),
        0x90 => s("ms", sg),
        0x96 => s(deg_c, sg * 0.1),
        0x99 => s("kPa", sg * 0.1),
        0x9C => s(deg, sg * 0.01),
        0x9D => s(deg, sg * 0.5),
        0xA8 => s("g/s", sg),
        0xA9 => s("Pa/s", sg * 0.25),
        0xAD => s("mg/stroke", sg * 0.01),
        0xAE => s("mg/stroke", sg * 0.1),
        0xAF => s("%", sg * 0.01),
        0xB0 => s("%", sg * 0.003052),
        0xB1 => s("mV/s", sg * 2.0),
        0xFC => s("kPa", sg * 0.01),
        0xFD => s("kPa", sg * 0.001),
        0xFE => s("Pa", sg * 0.25),
        _ => s("", u),
    }
}

/// Mode 05: the standardized test numbers of the oxygen sensor test.
pub fn oxygen_test_name(test_id: i32) -> String {
    match test_id {
        0x01 => l(
            "Spannungsschwelle mager nach fett",
            "Lean-to-rich threshold voltage",
        ),
        0x02 => l(
            "Spannungsschwelle fett nach mager",
            "Rich-to-lean threshold voltage",
        ),
        0x03 => l(
            "tiefste Spannung im Regelbetrieb",
            "Minimum voltage in closed loop",
        ),
        0x04 => l(
            "höchste Spannung im Regelbetrieb",
            "Maximum voltage in closed loop",
        ),
        0x05 => l("Umschaltzeit mager nach fett", "Lean-to-rich switch time"),
        0x06 => l("Umschaltzeit fett nach mager", "Rich-to-lean switch time"),
        0x07 => l("tiefste Sondenspannung", "Minimum sensor voltage"),
        0x08 => l("höchste Sondenspannung", "Maximum sensor voltage"),
        0x09 => l(
            "Zeit zwischen den Umschaltungen",
            "Time between transitions",
        ),
        0x0A => l("Periodendauer der Regelung", "Control period"),
        _ => return format!("Test {test_id}"),
    }
    .to_string()
}

/// Mode 05: converts the raw value of an oxygen sensor test.
pub fn oxygen_scale(test_id: i32, raw: i32) -> (f64, &'static str) {
    let raw = f64::from(raw);
    match test_id {
        0x01 | 0x02 | 0x03 | 0x04 | 0x07 | 0x08 => s("V", raw * 0.005),
        0x05 | 0x06 | 0x09 | 0x0A => s("s", raw * 0.004),
        _ => s("", raw),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{english, german};

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-6 * (1.0 + b.abs())
    }

    #[test]
    fn monitor_names_follow_the_standard() {
        let _lang = german();
        assert_eq!(monitor_component_name(0x01), "Lambdasonde Bank 1 Sonde 1");
        assert_eq!(monitor_component_name(0x06), "Lambdasonde Bank 2 Sonde 2");
        assert_eq!(monitor_component_name(0x22), "Katalysator Bank 2");
        assert!(monitor_component_name(0x3C).contains("0,020"));
        assert_eq!(
            monitor_component_name(0xAD),
            "Verbrennungsaussetzer Zylinder 12"
        );
        assert_eq!(monitor_component_name(0xB0), "Partikelfilter Bank 1");
        assert_eq!(monitor_test_name_can(0x01, 0x01), oxygen_test_name(0x01));
    }

    #[test]
    fn component_monitoring_converts_correctly() {
        let _lang = german();
        // ID 0x0B: voltage in millivolts.
        let (value, unit) = monitor_scale(0x0B, 1000);
        assert!(approx(value, 1.0));
        assert_eq!(unit, "V");
        // ID 0x16: temperature, a tenth of a degree with offset (SAE J1979).
        let (value, unit) = monitor_scale(0x16, 800);
        assert!(approx(value, 40.0));
        assert_eq!(unit, "Grad C");
        // ID 0x07: engine speed in quarters.
        let (value, unit) = monitor_scale(0x07, 4000);
        assert!(approx(value, 1000.0));
        assert_eq!(unit, "1/min");
        // ID 0x24: plain counts, as the misfire monitors use them.
        assert!(approx(monitor_scale(0x24, 17).0, 17.0));
        // Signed IDs read the two bytes as two's complement.
        assert!(approx(monitor_scale(0x96, 0xFFF6).0, -1.0));
        // An unknown ID returns the raw value instead of nonsense.
        assert!(approx(monitor_scale(0x7E, 1234).0, 1234.0));

        assert!(!monitor_test_name(0x21).is_empty());
        assert!(monitor_component_name(0x21).contains("Katalysator"));
        // Unknown entries stay named instead of empty.
        assert!(!monitor_test_name(0xF3).is_empty());
        assert!(!monitor_component_name(0xF3).is_empty());
    }

    #[test]
    fn oxygen_sensor_test_converts_correctly() {
        let _lang = german();
        // Voltage tests: five millivolts per step.
        let (value, unit) = oxygen_scale(0x01, 100);
        assert!(approx(value, 0.5));
        assert_eq!(unit, "V");
        // Time tests: four milliseconds per step.
        let (value, unit) = oxygen_scale(0x05, 250);
        assert!(approx(value, 1.0));
        assert_eq!(unit, "s");
        assert!(oxygen_test_name(0x01).contains("mager nach fett"));
        assert!(!oxygen_test_name(0x7F).is_empty());
    }

    #[test]
    fn names_follow_the_active_language() {
        {
            let _lang = german();
            assert_eq!(monitor_test_name(0xF3), "Test 0xF3 (herstellerspezifisch)");
            assert_eq!(monitor_component_name(0xF3), "Baugruppe 0xF3");
        }
        let _lang = english();
        assert_eq!(monitor_test_name(0xF3), "Test 0xF3 (manufacturer-specific)");
        assert_eq!(monitor_component_name(0x22), "Catalyst bank 2");
        assert_eq!(monitor_component_name(0x01), "O2 sensor bank 1 sensor 1");
        assert_eq!(monitor_scale(0x16, 800).1, "deg C");
    }
}
