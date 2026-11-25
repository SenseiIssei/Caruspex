//! The failure mode identifier (FMI) of a J1939 fault: what is wrong with
//! the value or part the SPN names. The numbers are the standard ones; the
//! explanations are written for a driver or a workshop, in our own words.

use caruspex_i18n::l;

/// What FMI 0 to 31 means, in plain words.
pub(crate) fn fmi_text(fmi: u8) -> &'static str {
    match fmi {
        0 => l(
            "Wert viel zu hoch, über dem normalen Bereich (schwerste Stufe)",
            "Value far too high, above the normal range (most severe level)",
        ),
        1 => l(
            "Wert viel zu niedrig, unter dem normalen Bereich (schwerste Stufe)",
            "Value far too low, below the normal range (most severe level)",
        ),
        2 => l(
            "Wert unstet, springt oder passt nicht zu den anderen Werten",
            "Value unsteady, jumping or not matching the other values",
        ),
        3 => l(
            "Spannung zu hoch oder Kurzschluss nach Plus",
            "Voltage too high or short circuit to positive",
        ),
        4 => l(
            "Spannung zu niedrig oder Kurzschluss nach Masse",
            "Voltage too low or short circuit to ground",
        ),
        5 => l(
            "Strom zu niedrig oder Leitung unterbrochen",
            "Current too low or wire broken",
        ),
        6 => l(
            "Strom zu hoch oder Leitung an Masse",
            "Current too high or wire touching ground",
        ),
        7 => l(
            "Mechanik reagiert nicht richtig oder ist verstellt",
            "Mechanical part does not respond properly or is out of adjustment",
        ),
        8 => l(
            "Frequenz, Pulsbreite oder Periode stimmt nicht",
            "Frequency, pulse width or period is wrong",
        ),
        9 => l(
            "Wert kommt zu selten oder gar nicht mehr über den Bus",
            "Value arrives too seldom or no longer over the bus",
        ),
        10 => l(
            "Wert ändert sich ungewöhnlich schnell",
            "Value changes unusually fast",
        ),
        11 => l(
            "Ursache unbekannt, der Fehler ist aber da",
            "Cause unknown, but the fault is there",
        ),
        12 => l(
            "Bauteil oder Steuergerät defekt",
            "Component or control unit faulty",
        ),
        13 => l(
            "Kalibrierung stimmt nicht, Anlernen nötig",
            "Out of calibration, needs to be taught in again",
        ),
        14 => l(
            "Sonderfall, siehe Unterlagen des Herstellers",
            "Special case, see the maker's documentation",
        ),
        15 => l(
            "Wert etwas zu hoch (leichteste Stufe)",
            "Value slightly too high (least severe level)",
        ),
        16 => l(
            "Wert zu hoch (mittlere Stufe)",
            "Value too high (moderate level)",
        ),
        17 => l(
            "Wert etwas zu niedrig (leichteste Stufe)",
            "Value slightly too low (least severe level)",
        ),
        18 => l(
            "Wert zu niedrig (mittlere Stufe)",
            "Value too low (moderate level)",
        ),
        19 => l(
            "Über den Bus empfangener Wert ist fehlerhaft",
            "Value received over the bus is faulty",
        ),
        20 => l("Wert driftet nach oben", "Value drifts upwards"),
        21 => l("Wert driftet nach unten", "Value drifts downwards"),
        31 => l(
            "Zustand liegt vor (siehe Bedeutung des SPN)",
            "Condition present (see what the SPN stands for)",
        ),
        _ => l(
            "Reserviert, vom Standard noch nicht belegt",
            "Reserved, not yet assigned by the standard",
        ),
    }
}

/// How serious a failure mode is on its own, 1 (note) to 3 (act now), for
/// sorting and colour when the unit's lamps do not say more.
pub fn fmi_weight(fmi: u8) -> u8 {
    match fmi {
        0 | 1 => 3,
        15 | 17 | 20 | 21 | 31 => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fmi_has_a_text() {
        for fmi in 0..=31u8 {
            assert!(!fmi_text(fmi).is_empty());
        }
        assert_eq!(fmi_text(22), fmi_text(30));
        assert_ne!(fmi_text(0), fmi_text(1));
        assert_eq!(fmi_weight(0), 3);
        assert_eq!(fmi_weight(17), 1);
    }
}
