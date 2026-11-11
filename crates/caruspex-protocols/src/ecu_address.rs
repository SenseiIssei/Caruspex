//! Which control unit is behind a response address.
//!
//! With headers switched on (`Elm327::set_headers`) every OBD response carries
//! the address of its sender. OBD itself does not say what is behind an
//! address, but the conventions are firm enough for a name:
//!
//! ```text
//!   CAN 11 bit   7E8 is the engine, 7E9 the transmission, 7EA to 7EF others
//!   CAN 29 bit   18DAF1xx, xx = 10 engine, 18 transmission (as below)
//!   K-line/J1850 source byte 10 to 17 engine, 18 to 1F transmission
//! ```
//!
//! The name from mode 09 (InfoType 0x0A) is more precise and wins when the
//! control unit provides it.

use caruspex_i18n::l;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum EcuRole {
    #[default]
    Unknown,
    Engine,
    Transmission,
    Other,
}

fn all_hex(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Source byte on K-line and J1850 (SAE J2178), also the low byte of a
/// 29-bit CAN identifier.
fn role_for_source_byte(address: u64) -> EcuRole {
    match address {
        0x10..=0x17 => EcuRole::Engine,
        0x18..=0x1F => EcuRole::Transmission,
        _ => EcuRole::Other,
    }
}

/// Role by address. Unknown for an empty source (headers off).
pub fn ecu_role(source: &str) -> EcuRole {
    if !all_hex(source) {
        return EcuRole::Unknown;
    }
    if source.len() > 8 {
        return EcuRole::Other;
    }
    let value = u64::from_str_radix(source, 16).unwrap_or(0);
    match source.len() {
        // CAN 11 bit
        3 => match value {
            0x7E8 => EcuRole::Engine,
            0x7E9 => EcuRole::Transmission,
            _ => EcuRole::Other,
        },
        // CAN 29 bit, 18DAF1xx
        8 => role_for_source_byte(value & 0xFF),
        // K-line and J1850 source byte
        2 => role_for_source_byte(value),
        _ => EcuRole::Other,
    }
}

/// Plain name of the role in the active language: "Motor", "Getriebe",
/// "Steuergerät". Empty for Unknown.
pub fn ecu_role_name(role: EcuRole) -> &'static str {
    match role {
        EcuRole::Engine => l("Motor", "Engine"),
        EcuRole::Transmission => l("Getriebe", "Transmission"),
        EcuRole::Other => l("Steuergerät", "Control unit"),
        EcuRole::Unknown => "",
    }
}

/// Name with the address in brackets, e.g. "Motor (7E8)" or
/// "Getriebe (18DAF118)". Empty for an empty source.
pub fn ecu_display_name(source: &str) -> String {
    if source.is_empty() {
        return String::new();
    }
    let role = ecu_role(source);
    if role == EcuRole::Unknown {
        return source.to_string();
    }
    format!("{} ({})", ecu_role_name(role), source)
}
