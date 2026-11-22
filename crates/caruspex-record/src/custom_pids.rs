//! Custom values: anything a car offers beyond the standard OBD PIDs.
//!
//! Makers keep the interesting values behind their own identifiers: service
//! 22 (UDS, read data by identifier) or service 21 (KWP2000, read data by
//! local identifier), often on a control unit other than the engine. The
//! battery management of an electric car is the typical case: state of
//! health, cell voltages and temperatures are nowhere in standard OBD.
//!
//! A custom value says whom to ask (header), what to ask (request) and how to
//! turn the answer into a number (equation). The format is the one of Torque
//! Pro, which the community uses to share such values, so existing lists can
//! be imported directly:
//!
//! ```text
//! Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header
//! State of health,SOH,220105,((Z<8)+AA)/10,0,100,%,7E4
//! ```
//!
//! The equation sees the data bytes of the positive answer as A, B, C, ...
//! Z, AA, AB, ... where A is the first byte after the echoed service and
//! identifier (for 220105 the answer starts 62 01 05, so A is the byte after
//! that).
//!
//! Supported in equations (case does not matter):
//!
//! - numbers: 12, 0.5, .25
//! - bytes: A .. Z, AA .. ZZ
//! - arithmetic: + - * / and parentheses, unary minus
//! - shifts: `x<n` and `x>n` shift the integer part of x by n bits. Not in
//!   the official Torque documentation, but used by the widely shared EV
//!   lists and accepted by Torque. They bind weaker than + and -, as in C:
//!   (A<8)+B, not A<8+B.
//! - bits: {A:3} is bit 3 of A (0 or 1); BIT(x:n) does the same for any
//!   expression
//! - functions: SIGNED(x) or SIGNED8(x): x as 8-bit two's complement;
//!   SIGNED16(x), SIGNED24(x), SIGNED32(x): the same with more bits;
//!   INT16(a:b), INT24(a:b:c), INT32(a:b:c:d): bytes joined, most
//!   significant first, unsigned; MIN(a:b), MAX(a:b), ABS(x), INT(x) (drops
//!   the fraction)
//!
//! Not supported, and reported as an error: val{...} (references to other
//! values), the averaging and delay functions, LOOKUP, FLOAT32.
//!
//! Our own lists may carry a ninth column "Response" with the id the control
//! unit answers on. It matters where it is not header + 8, for example on the
//! Renault-Nissan battery controller (79B answers on 7BB).
//!
//! Values the car broadcasts by itself have the request "CAN132" (CAN plus an
//! 11-bit id) and a DBC signal as equation, "DBC:start|len@order+(factor,offset)"
//! with an optional multiplexer " mN@s|l". They are listened for, not asked.

use caruspex_data::pids::PidCategory;
use caruspex_i18n::l;

// The equations, the Torque CSV format and the reading from the car.
mod csv;
mod equation;
mod reader;

pub use csv::{parse_request_hex, parse_torque_csv, write_torque_csv};
pub use equation::TorqueEquation;
pub use reader::{CustomPidReader, CustomReading, read_custom};

/// Keys of custom values in `LiveValue::key`. OBD keys are pid | (sub << 8)
/// and stay far below these ranges, so both can share one series map.
/// Values of the built-in packs.
pub const PACK_KEY_BASE: i32 = 0x10_0000;
/// Values imported from a file.
pub const USER_KEY_BASE: i32 = 0x20_0000;

pub const fn is_custom_key(key: i32) -> bool {
    key >= PACK_KEY_BASE
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomPid {
    pub name: String,
    pub short_name: String,
    /// Hex, normalized: "220101", "2101", "015C". Or "CAN132" for a value the
    /// car broadcasts by itself.
    pub request: String,
    /// Request id, e.g. "7E4"; empty = functional OBD addressing.
    pub header: String,
    /// Response id; empty = header + 8 (11 bit) or the swapped 29-bit address
    /// (18DA44F1 -> 18DAF144).
    pub response: String,
    pub equation: String,
    pub min_value: f64,
    pub max_value: f64,
    pub unit: String,
    /// Id of the built-in pack, empty for imported values.
    pub pack: String,
    /// Where the definition comes from.
    pub source: String,
    /// Series id, see [`PACK_KEY_BASE`].
    pub key: i32,
    pub category: PidCategory,
    /// The diagnostic session the request needs: 0 for the default one,
    /// 0x03 for the extended session (ISO 14229), which the reader enters
    /// only through [`ExtendedSessions`].
    pub session: u8,
}

impl Default for CustomPid {
    fn default() -> Self {
        Self {
            name: String::new(),
            short_name: String::new(),
            request: String::new(),
            header: String::new(),
            response: String::new(),
            equation: String::new(),
            min_value: 0.0,
            max_value: 0.0,
            unit: String::new(),
            pack: String::new(),
            source: String::new(),
            key: 0,
            category: PidCategory::Custom,
            session: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Small text helpers
// ---------------------------------------------------------------------------

fn trim(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\r', '\n'])
}

fn is_hex_text(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_hexdigit())
}

fn strip_hex_prefix(s: &str) -> &str {
    if s.len() >= 2 && s.as_bytes()[0] == b'0' && matches!(s.as_bytes()[1], b'x' | b'X') {
        &s[2..]
    } else {
        s
    }
}

/// C `isspace` in the C locale.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

/// A decimal number in the C locale: optional sign, digits, optional point
/// and fraction, optional exponent. Locale independent on purpose: the GUI
/// may run under a German one where the point is a comma. The syntax is
/// checked here; the value itself comes from the standard library, which
/// rounds correctly, so "8.354" is exactly the double other programs read
/// (the OBDb import relies on that).
fn parse_number(text: &str) -> Option<f64> {
    let t = trim(text);
    let s = t.as_bytes();
    let mut i = 0;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        i += 1;
    }
    let mut digits = false;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
        digits = true;
    }
    if i < s.len() && s[i] == b'.' {
        i += 1;
        while i < s.len() && s[i].is_ascii_digit() {
            i += 1;
            digits = true;
        }
    }
    if !digits {
        return None;
    }
    if i < s.len() && (s[i] == b'e' || s[i] == b'E') {
        i += 1;
        if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
            i += 1;
        }
        let mut exp_digits = false;
        while i < s.len() && s[i].is_ascii_digit() {
            i += 1;
            exp_digits = true;
        }
        if !exp_digits {
            return None;
        }
    }
    if i != s.len() {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// The opposite, also independent of the locale: `%.10g`.
fn format_number(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    if value == 0.0 {
        return if value.is_sign_negative() { "-0" } else { "0" }.to_string();
    }
    // Ten significant digits, rounded correctly; the exponent decides between
    // fixed and scientific notation, as %g does.
    let scientific = format!("{value:.9e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let trim_zeros = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if !(-4..10).contains(&exponent) {
        let sign = if exponent < 0 { '-' } else { '+' };
        format!("{}e{sign}{:02}", trim_zeros(mantissa), exponent.abs())
    } else {
        let decimals = (9 - exponent).max(0) as usize;
        trim_zeros(&format!("{value:.decimals$}"))
    }
}

fn line_text(line: usize) -> String {
    format!("{}{line}: ", l("Zeile ", "Line "))
}

#[cfg(test)]
mod tests {
    use super::{format_number, parse_number};

    #[test]
    fn numbers_are_written_like_printf_g_with_ten_digits() {
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(100.0), "100");
        assert_eq!(format_number(-0.25), "-0.25");
        assert_eq!(format_number(1234.5), "1234.5");
        assert_eq!(format_number(16777215.0), "16777215");
        assert_eq!(format_number(1.0 / 3.0), "0.3333333333");
        assert_eq!(format_number(1.0e10), "1e+10");
        assert_eq!(format_number(0.0001), "0.0001");
        assert_eq!(format_number(0.00001), "1e-05");
        assert_eq!(format_number(123456789012.0), "1.23456789e+11");
    }

    #[test]
    fn numbers_are_read_independent_of_the_locale() {
        assert_eq!(parse_number(" 1.5 "), Some(1.5));
        assert_eq!(parse_number("-230"), Some(-230.0));
        assert_eq!(parse_number("1e3"), Some(1000.0));
        assert_eq!(parse_number("1,5"), None);
        assert_eq!(parse_number("e3"), None);
        assert_eq!(parse_number("low"), None);
    }
}
