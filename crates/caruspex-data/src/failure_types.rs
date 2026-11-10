//! The failure type byte of a UDS fault code (SAE J2012-DA, also listed in
//! ISO 15031-6 and ISO 14229-1 annex D).
//!
//! A UDS DTC has three bytes: the first two are the familiar code ("C0035"),
//! the third says what kind of fault it is. The high nibble is the category
//! (electrical, signal, internal, mechanical ...), the low nibble the exact
//! type within it. Only the subtypes with a well known meaning are listed;
//! for the others the category is still worth showing, so the text falls
//! back to it. 0xF0 to 0xFF belong to the manufacturer.

use caruspex_i18n::{Text, l};

struct FailureType {
    value: i32,
    text: Text,
}

const fn ft(value: i32, de: &'static str, en: &'static str) -> FailureType {
    FailureType {
        value,
        text: Text::new(de, en),
    }
}

static FAILURE_TYPES: &[FailureType] = &[
    ft(
        0x01,
        "allgemeiner elektrischer Fehler",
        "general electrical failure",
    ),
    ft(0x02, "allgemeiner Signalfehler", "general signal failure"),
    ft(
        0x03,
        "Fehler im Frequenz- oder PWM-Signal",
        "FM (frequency modulated) or PWM failure",
    ),
    ft(
        0x04,
        "interner Fehler des Systems",
        "system internal failure",
    ),
    ft(
        0x05,
        "Programmierfehler des Systems",
        "system programming failure",
    ),
    ft(
        0x06,
        "Fehler aus einer Berechnung",
        "algorithm based failure",
    ),
    ft(0x07, "mechanischer Fehler", "mechanical failure"),
    ft(
        0x08,
        "Fehler im Bussignal oder in der Botschaft",
        "bus signal or message failure",
    ),
    ft(0x09, "Bauteilfehler", "component failure"),
    ft(0x11, "Kurzschluss nach Masse", "circuit short to ground"),
    ft(0x12, "Kurzschluss nach Plus", "circuit short to battery"),
    ft(0x13, "Unterbrechung", "circuit open"),
    ft(
        0x14,
        "Kurzschluss nach Masse oder Unterbrechung",
        "circuit short to ground or open",
    ),
    ft(
        0x15,
        "Kurzschluss nach Plus oder Unterbrechung",
        "circuit short to battery or open",
    ),
    ft(
        0x16,
        "Spannung unter der Schwelle",
        "circuit voltage below threshold",
    ),
    ft(
        0x17,
        "Spannung über der Schwelle",
        "circuit voltage above threshold",
    ),
    ft(
        0x18,
        "Strom unter der Schwelle",
        "circuit current below threshold",
    ),
    ft(
        0x19,
        "Strom über der Schwelle",
        "circuit current above threshold",
    ),
    ft(
        0x1A,
        "Widerstand unter der Schwelle",
        "circuit resistance below threshold",
    ),
    ft(
        0x1B,
        "Widerstand über der Schwelle",
        "circuit resistance above threshold",
    ),
    ft(
        0x1C,
        "Spannung außerhalb des Bereichs",
        "circuit voltage out of range",
    ),
    ft(
        0x1D,
        "Strom außerhalb des Bereichs",
        "circuit current out of range",
    ),
    ft(
        0x1E,
        "Widerstand außerhalb des Bereichs",
        "circuit resistance out of range",
    ),
    ft(
        0x1F,
        "sporadischer Fehler im Stromkreis",
        "circuit intermittent",
    ),
    ft(
        0x21,
        "Signalamplitude zu klein",
        "signal amplitude below minimum",
    ),
    ft(
        0x22,
        "Signalamplitude zu groß",
        "signal amplitude above maximum",
    ),
    ft(0x23, "Signal bleibt auf niedrig stehen", "signal stuck low"),
    ft(0x24, "Signal bleibt auf hoch stehen", "signal stuck high"),
    ft(
        0x25,
        "Signalform falsch",
        "signal shape or waveform failure",
    ),
    ft(
        0x26,
        "Signal ändert sich zu langsam",
        "signal rate of change below threshold",
    ),
    ft(
        0x27,
        "Signal ändert sich zu schnell",
        "signal rate of change above threshold",
    ),
    ft(
        0x28,
        "Nullpunkt oder Grundwert des Signals außerhalb des Bereichs",
        "signal bias level out of range or zero adjustment failure",
    ),
    ft(0x29, "Signal ungültig", "signal invalid"),
    ft(
        0x2A,
        "Signal bleibt innerhalb des Bereichs stehen",
        "signal stuck in range",
    ),
    ft(
        0x2B,
        "Signal mit einem anderen verkoppelt",
        "signal cross coupled",
    ),
    ft(0x2F, "Signal unruhig", "signal erratic"),
    ft(0x31, "kein Signal", "no signal"),
    ft(
        0x32,
        "Low-Zeit des Signals zu kurz",
        "signal low time below minimum",
    ),
    ft(
        0x33,
        "Low-Zeit des Signals zu lang",
        "signal low time above maximum",
    ),
    ft(
        0x34,
        "High-Zeit des Signals zu kurz",
        "signal high time below minimum",
    ),
    ft(
        0x35,
        "High-Zeit des Signals zu lang",
        "signal high time above maximum",
    ),
    ft(
        0x36,
        "Signalfrequenz zu niedrig",
        "signal frequency too low",
    ),
    ft(0x37, "Signalfrequenz zu hoch", "signal frequency too high"),
    ft(0x38, "Signalfrequenz falsch", "signal frequency incorrect"),
    ft(0x39, "zu wenige Impulse", "signal has too few pulses"),
    ft(0x3A, "zu viele Impulse", "signal has too many pulses"),
    ft(0x41, "Prüfsummenfehler", "general checksum failure"),
    ft(0x42, "Speicherfehler", "general memory failure"),
    ft(0x43, "Fehler im Sonderspeicher", "special memory failure"),
    ft(0x44, "Fehler im Datenspeicher (RAM)", "data memory failure"),
    ft(0x45, "Fehler im Programmspeicher", "program memory failure"),
    ft(
        0x46,
        "Fehler im Kalibrier- oder Parameterspeicher",
        "calibration or parameter memory failure",
    ),
    ft(
        0x47,
        "Fehler der Überwachung (Watchdog)",
        "watchdog or safety processor failure",
    ),
    ft(
        0x48,
        "Fehler der Überwachungssoftware",
        "supervision software failure",
    ),
    ft(
        0x49,
        "interner elektronischer Fehler",
        "internal electronic failure",
    ),
    ft(
        0x4A,
        "falsches Bauteil eingebaut",
        "incorrect component installed",
    ),
    ft(0x4B, "Übertemperatur", "over temperature"),
    ft(0x51, "nicht programmiert", "not programmed"),
    ft(0x52, "nicht aktiviert", "not activated"),
    ft(0x53, "deaktiviert", "deactivated"),
    ft(0x54, "Kalibrierung fehlt", "missing calibration"),
    ft(0x55, "nicht konfiguriert", "not configured"),
    ft(
        0x56,
        "ungültige oder unpassende Konfiguration",
        "invalid or incompatible configuration",
    ),
    ft(
        0x57,
        "ungültige oder unpassende Software",
        "invalid or incompatible software component",
    ),
    ft(
        0x61,
        "Fehler bei der Signalberechnung",
        "signal calculation failure",
    ),
    ft(
        0x62,
        "Signalvergleich fehlgeschlagen",
        "signal compare failure",
    ),
    ft(
        0x63,
        "Schutzabschaltung nach Zeitüberschreitung",
        "circuit or component protection time-out",
    ),
    ft(0x64, "Signal unplausibel", "signal plausibility failure"),
    ft(
        0x65,
        "zu wenige Signalwechsel",
        "signal has too few transitions or events",
    ),
    ft(
        0x66,
        "zu viele Signalwechsel",
        "signal has too many transitions or events",
    ),
    ft(
        0x67,
        "Signal nach dem Ereignis falsch",
        "signal incorrect after event",
    ),
    ft(0x68, "Ereignisinformation", "event information"),
    ft(0x71, "Stellglied klemmt", "actuator stuck"),
    ft(0x72, "Stellglied klemmt offen", "actuator stuck open"),
    ft(
        0x73,
        "Stellglied klemmt geschlossen",
        "actuator stuck closed",
    ),
    ft(0x74, "Stellglied rutscht durch", "actuator slipping"),
    ft(
        0x75,
        "Notlaufstellung nicht erreichbar",
        "emergency position not reachable",
    ),
    ft(0x76, "falsche Einbaulage", "wrong mounting position"),
    ft(
        0x77,
        "Sollstellung nicht erreichbar",
        "commanded position not reachable",
    ),
    ft(
        0x78,
        "Einstellung oder Ausrichtung falsch",
        "alignment or adjustment incorrect",
    ),
    ft(
        0x79,
        "mechanische Verbindung defekt",
        "mechanical linkage failure",
    ),
    ft(0x7A, "Undichtigkeit", "fluid leak or seal failure"),
    ft(0x7B, "Flüssigkeitsstand zu niedrig", "low fluid level"),
    ft(
        0x81,
        "ungültige Daten empfangen",
        "invalid serial data received",
    ),
    ft(
        0x82,
        "Botschaftszähler falsch oder nicht aktualisiert",
        "alive or sequence counter incorrect or not updated",
    ),
    ft(
        0x83,
        "Prüfwert der Botschaft falsch",
        "value of signal protection calculation incorrect",
    ),
    ft(
        0x84,
        "Signal unter dem zulässigen Bereich",
        "signal below allowable range",
    ),
    ft(
        0x85,
        "Signal über dem zulässigen Bereich",
        "signal above allowable range",
    ),
    ft(0x86, "Signal ungültig", "invalid signal"),
    ft(0x87, "Botschaft fehlt", "missing message"),
    ft(0x88, "Bus abgeschaltet (Bus off)", "bus off"),
    ft(0x91, "Kennwerte außerhalb", "parametric"),
    ft(
        0x92,
        "Leistung zu gering oder falsche Funktion",
        "performance or incorrect operation",
    ),
    ft(0x93, "keine Funktion", "no operation"),
    ft(0x94, "unerwartete Funktion", "unexpected operation"),
    ft(0x95, "falsch montiert", "incorrect assembly"),
    ft(0x96, "interner Bauteilfehler", "component internal failure"),
    ft(
        0x97,
        "Funktion behindert oder blockiert",
        "component or system operation obstructed or blocked",
    ),
    ft(
        0x98,
        "Übertemperatur des Bauteils",
        "component or system over temperature",
    ),
];

/// Name of the category in the high nibble.
fn category_name(category: i32) -> Option<&'static str> {
    Some(match category {
        0x0 => l("allgemeiner Fehler", "general failure"),
        0x1 => l("elektrischer Fehler", "electrical failure"),
        0x2 => l("Signalfehler", "signal failure"),
        0x3 => l("Fehler im Frequenz- oder PWM-Signal", "FM or PWM failure"),
        0x4 => l(
            "interner Fehler des Steuergeräts",
            "system internal failure",
        ),
        0x5 => l(
            "Programmier- oder Konfigurationsfehler",
            "programming failure",
        ),
        0x6 => l("Fehler aus einer Berechnung", "algorithm based failure"),
        0x7 => l("mechanischer Fehler", "mechanical failure"),
        0x8 => l("Fehler im Bussignal", "bus signal or message failure"),
        0x9 => l("Bauteilfehler", "component failure"),
        _ => return None,
    })
}

/// Text for the failure type byte of a UDS DTC (SAE J2012-DA), active
/// language. Empty for 0x00 (no subtype).
pub fn failure_type_text(failure_type: i32) -> String {
    let failure_type = failure_type & 0xFF;
    if failure_type == 0x00 {
        return String::new();
    }
    if let Some(f) = FAILURE_TYPES.iter().find(|f| f.value == failure_type) {
        return f.text.get().to_string();
    }
    if failure_type >= 0xF0 {
        return l(
            "herstellerspezifische Fehlerart",
            "manufacturer specific failure type",
        )
        .to_string();
    }
    if let Some(category) = category_name(failure_type >> 4) {
        return category.to_string();
    }
    // 0xA0 to 0xEF are reserved by the standard; show the raw value.
    format!("{} 0x{failure_type:02X}", l("Fehlerart", "failure type"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{english, german};

    #[test]
    fn failure_type_texts_after_sae_j2012_da() {
        {
            let _lang = german();
            assert_eq!(failure_type_text(0x11), "Kurzschluss nach Masse");
            assert_eq!(failure_type_text(0x13), "Unterbrechung");
            assert_eq!(failure_type_text(0x1C), "Spannung außerhalb des Bereichs");
            assert_eq!(failure_type_text(0x64), "Signal unplausibel");
        }
        let _lang = english();
        assert!(failure_type_text(0x00).is_empty());
        assert_eq!(failure_type_text(0x11), "circuit short to ground");
        assert_eq!(failure_type_text(0x12), "circuit short to battery");
        assert_eq!(failure_type_text(0x29), "signal invalid");
        assert_eq!(failure_type_text(0x87), "missing message");
        assert_eq!(
            failure_type_text(0x92),
            "performance or incorrect operation"
        );
        // Subtypes without an entry fall back to their category.
        assert_eq!(failure_type_text(0x2D), "signal failure");
        assert_eq!(failure_type_text(0x4F), "system internal failure");
        assert_eq!(
            failure_type_text(0xF3),
            "manufacturer specific failure type"
        );
        assert_eq!(failure_type_text(0xB2), "failure type 0xB2");
        // Every byte gives something, and nothing but 0x00 is empty.
        for i in 1..0x100 {
            assert!(!failure_type_text(i).is_empty(), "{i:#x}");
        }
    }

    #[test]
    fn every_listed_type_is_unique_and_has_both_texts() {
        let mut seen = std::collections::HashSet::new();
        for f in FAILURE_TYPES {
            assert!(seen.insert(f.value), "{:#x} twice", f.value);
            assert!(!f.text.de.is_empty() && !f.text.en.is_empty());
        }
        assert_eq!(FAILURE_TYPES.len(), 99);
    }
}
