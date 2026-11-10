//! Status texts. They are resolved when called, so they follow the language.

use caruspex_i18n::l;

fn status(value: f64, de: &'static str, en: &'static str) -> String {
    format!("{}{}", l(de, en), value as i32)
}

pub fn fuel_status_text(value: f64) -> String {
    match value as i32 {
        0 => l(
            "Motor aus oder nicht vorhanden",
            "engine off or not present",
        )
        .to_string(),
        1 => l(
            "offener Regelkreis (Motor noch kalt)",
            "open loop (engine still cold)",
        )
        .to_string(),
        2 => l("geschlossener Regelkreis (normal)", "closed loop (normal)").to_string(),
        4 => l(
            "offener Regelkreis (Volllast oder Schub)",
            "open loop (full load or overrun)",
        )
        .to_string(),
        8 => l(
            "offener Regelkreis (Systemfehler)",
            "open loop (system fault)",
        )
        .to_string(),
        16 => l(
            "geschlossen, aber eine Lambdasonde fällt aus",
            "closed loop, but an oxygen sensor has failed",
        )
        .to_string(),
        _ => status(value, "Status ", "status "),
    }
}

pub fn secondary_air_text(value: f64) -> String {
    match value as i32 {
        1 => l("vor dem Katalysator", "upstream of the catalytic converter").to_string(),
        2 => l(
            "hinter dem Katalysator",
            "downstream of the catalytic converter",
        )
        .to_string(),
        4 => l(
            "aus der Außenluft oder abgeschaltet",
            "from the outside air or off",
        )
        .to_string(),
        8 => l(
            "Pumpe für Prüfung eingeschaltet",
            "pump switched on for a check",
        )
        .to_string(),
        _ => status(value, "Status ", "status "),
    }
}

pub fn obd_standard_text(value: f64) -> String {
    let text = match value as i32 {
        1 => l("OBD-II (Kalifornien)", "OBD-II (California)"),
        2 => l("OBD (Bundesstaaten USA)", "OBD (federal, USA)"),
        3 => l("OBD und OBD-II", "OBD and OBD-II"),
        4 => "OBD-I",
        5 => l("nicht OBD-pflichtig", "not OBD compliant"),
        6 => l("EOBD (Europa)", "EOBD (Europe)"),
        7 => l("EOBD und OBD-II", "EOBD and OBD-II"),
        8 => l("EOBD und OBD", "EOBD and OBD"),
        9 => l("EOBD, OBD und OBD-II", "EOBD, OBD and OBD-II"),
        10 => "JOBD (Japan)",
        11 => l("JOBD und OBD-II", "JOBD and OBD-II"),
        12 => l("JOBD und EOBD", "JOBD and EOBD"),
        13 => l("JOBD, EOBD und OBD-II", "JOBD, EOBD and OBD-II"),
        14 => l("OBD, EOBD und KOBD", "OBD, EOBD and KOBD"),
        15 => l("OBD, OBD-II, EOBD und KOBD", "OBD, OBD-II, EOBD and KOBD"),
        17 => l(
            "Herstellerdiagnose (EMD)",
            "Engine manufacturer diagnostics (EMD)",
        ),
        18 => l(
            "Erweiterte Herstellerdiagnose (EMD+)",
            "Enhanced engine manufacturer diagnostics (EMD+)",
        ),
        19 => l(
            "Nutzfahrzeug-OBD, teilweise (HD OBD-C)",
            "Heavy duty OBD, partial (HD OBD-C)",
        ),
        20 => l("Nutzfahrzeug-OBD (HD OBD)", "Heavy duty OBD (HD OBD)"),
        21 => l(
            "Weltweit harmonisierte OBD (WWH-OBD)",
            "World wide harmonized OBD (WWH-OBD)",
        ),
        23 => l(
            "Nutzfahrzeug Euro-OBD Stufe I ohne NOx-Regelung",
            "Heavy duty Euro OBD stage I without NOx control",
        ),
        24 => l(
            "Nutzfahrzeug Euro-OBD Stufe I mit NOx-Regelung",
            "Heavy duty Euro OBD stage I with NOx control",
        ),
        25 => l(
            "Nutzfahrzeug Euro-OBD Stufe II ohne NOx-Regelung",
            "Heavy duty Euro OBD stage II without NOx control",
        ),
        26 => l(
            "Nutzfahrzeug Euro-OBD Stufe II mit NOx-Regelung",
            "Heavy duty Euro OBD stage II with NOx control",
        ),
        28 => l("Brasilien OBD Phase 1", "Brazil OBD phase 1"),
        29 => l("Brasilien OBD Phase 2", "Brazil OBD phase 2"),
        30 => l("Korea (KOBD)", "Korea (KOBD)"),
        31 => l("Indien OBD I", "India OBD I"),
        32 => l("Indien OBD II", "India OBD II"),
        33 => l(
            "Nutzfahrzeug Euro-OBD Stufe VI",
            "Heavy duty Euro OBD stage VI",
        ),
        _ => return status(value, "Norm ", "standard "),
    };
    text.to_string()
}

pub fn emission_design_text(value: f64) -> String {
    match value as i32 {
        0x0E => l("Nutzfahrzeug Euro IV (B1)", "Heavy duty Euro IV (B1)").to_string(),
        0x0F => l("Nutzfahrzeug Euro V (B2)", "Heavy duty Euro V (B2)").to_string(),
        0x10 => l("Nutzfahrzeug EEV (C)", "Heavy duty EEV (C)").to_string(),
        _ => status(value, "Anforderung ", "requirement "),
    }
}

pub fn fuel_type_text(value: f64) -> String {
    let text = match value as i32 {
        0 => l("nicht angegeben", "not available"),
        1 => l("Benzin", "Gasoline"),
        2 => "Methanol",
        3 => "Ethanol",
        4 => "Diesel",
        5 => l("LPG (Autogas)", "LPG"),
        6 => l("CNG (Erdgas)", "CNG (natural gas)"),
        7 => l("Propan", "Propane"),
        8 => l("Elektrisch", "Electric"),
        9 => l("Bifuel, fährt mit Benzin", "Bifuel running gasoline"),
        10 => l("Bifuel, fährt mit Methanol", "Bifuel running methanol"),
        11 => l("Bifuel, fährt mit Ethanol", "Bifuel running ethanol"),
        12 => l("Bifuel, fährt mit LPG", "Bifuel running LPG"),
        13 => l("Bifuel, fährt mit CNG", "Bifuel running CNG"),
        14 => l("Bifuel, fährt mit Propan", "Bifuel running propane"),
        15 => l("Bifuel, fährt elektrisch", "Bifuel running electricity"),
        16 => l(
            "Bifuel, elektrisch und Verbrenner",
            "Bifuel running electric and combustion",
        ),
        17 => l("Hybrid Benzin", "Hybrid gasoline"),
        18 => "Hybrid Ethanol",
        19 => "Hybrid Diesel",
        20 => l("Hybrid elektrisch", "Hybrid electric"),
        21 => l(
            "Hybrid, elektrisch und Verbrenner",
            "Hybrid running electric and combustion",
        ),
        22 => l("Hybrid, Rückgewinnung", "Hybrid regenerative"),
        23 => l("Bifuel, fährt mit Diesel", "Bifuel running diesel"),
        _ => return status(value, "Kraftstoffart ", "fuel type "),
    };
    text.to_string()
}

pub fn on_off_text(value: f64) -> String {
    if value != 0.0 {
        l("an", "on")
    } else {
        l("aus", "off")
    }
    .to_string()
}

pub fn active_text(value: f64) -> String {
    if value != 0.0 {
        l("aktiv", "active")
    } else {
        l("nicht aktiv", "not active")
    }
    .to_string()
}

pub fn engine_type_text(value: f64) -> String {
    if value != 0.0 {
        l("Diesel (Selbstzünder)", "Diesel (compression ignition)")
    } else {
        l("Benziner (Fremdzünder)", "Gasoline (spark ignition)")
    }
    .to_string()
}

pub fn automatic_range_text(value: f64) -> String {
    if value != 0.0 {
        l("Fahrstufe eingelegt", "drive range engaged")
    } else {
        l("Parken oder Neutral", "park or neutral")
    }
    .to_string()
}

pub fn manual_gear_text(value: f64) -> String {
    if value != 0.0 {
        l("Gang eingelegt", "in gear")
    } else {
        l(
            "Leerlauf oder Kupplung getreten",
            "neutral or clutch pressed",
        )
    }
    .to_string()
}

/// Boost pressure and turbocharger control: two bits per control loop.
pub fn control_loop_text(value: f64) -> String {
    match value as i32 {
        1 => l("gesteuert (offener Regelkreis)", "open loop"),
        2 => l("geregelt (geschlossener Regelkreis)", "closed loop"),
        3 => l("Fehler erkannt", "fault present"),
        _ => l("keine Angabe", "not reported"),
    }
    .to_string()
}

/// NOx and particulate "not to exceed" control area (heavy duty diesels).
pub fn nte_text(value: f64) -> String {
    let v = value as i32;
    let mut parts: Vec<&str> = Vec::new();
    if v & 0x01 != 0 {
        parts.push(l("im Kontrollbereich", "inside the control area"));
    }
    if v & 0x02 != 0 {
        parts.push(l(
            "außerhalb des Kontrollbereichs",
            "outside the control area",
        ));
    }
    if v & 0x04 != 0 {
        parts.push(l(
            "im Ausnahmebereich des Herstellers",
            "inside the manufacturer carve-out area",
        ));
    }
    if v & 0x08 != 0 {
        parts.push(l("Mangel aktiv", "deficiency active"));
    }
    if parts.is_empty() {
        return l("keine Angabe", "not reported").to_string();
    }
    parts.join(", ")
}

pub fn regeneration_text(value: f64) -> String {
    if value != 0.0 {
        l("läuft gerade", "in progress")
    } else {
        l("läuft nicht", "not running")
    }
    .to_string()
}

pub fn regeneration_type_text(value: f64) -> String {
    if value != 0.0 {
        l(
            "aktiv (vom Motor ausgelöst)",
            "active (triggered by the engine)",
        )
    } else {
        l(
            "passiv (von selbst beim Fahren)",
            "passive (on its own while driving)",
        )
    }
    .to_string()
}

pub fn gear_text(value: f64) -> String {
    let gear = value as i32;
    if gear == 0 {
        return l("Leerlauf", "neutral").to_string();
    }
    format!("{}{gear}", l("Gang ", "gear "))
}

/// Which oxygen sensors are fitted (PID 0x13 and 0x1D).
pub fn o2_present_text(mask: i32, banks: i32, per_bank: i32) -> String {
    let mut out = String::new();
    for bank in 0..banks {
        let sensors: Vec<String> = (0..per_bank)
            .filter(|s| mask & (1 << (bank * per_bank + s)) != 0)
            .map(|s| (s + 1).to_string())
            .collect();
        if sensors.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str("; ");
        }
        out.push_str(&format!(
            "{}{}{}{}",
            l("Bank ", "bank "),
            bank + 1,
            l(": Sonde ", ": sensor "),
            sensors.join(", ")
        ));
    }
    if out.is_empty() {
        out = l("keine", "none").to_string();
    }
    out
}
