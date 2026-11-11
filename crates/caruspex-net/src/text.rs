//! Small text helpers shared by the ELM327 front ends.

/// The command as the front ends compare it: upper case, without CR, LF and spaces.
pub(crate) fn normalize(data: &[u8]) -> String {
    data.iter()
        .filter(|&&c| c != b'\r' && c != b'\n' && c != b' ')
        .map(|&c| c.to_ascii_uppercase() as char)
        .collect()
}

pub(crate) fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_hexdigit())
}

/// Pairs of hex digits; the caller checked them with [`is_hex`].
pub(crate) fn parse_hex(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    (0..b.len() / 2)
        .map(|i| (digit(b[2 * i]) << 4) | digit(b[2 * i + 1]))
        .collect()
}

fn digit(c: u8) -> u8 {
    (c as char).to_digit(16).unwrap_or(0) as u8
}

/// Hex bytes, upper case, with or without a space between them.
pub(crate) fn hex(bytes: &[u8], spaces: bool) -> String {
    let mut out = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if spaces && i > 0 {
            out.push(' ');
        }
        out.push_str(&format!("{b:02X}"));
    }
    out
}

/// Leading blanks skipped, then an optional sign.
fn sign(text: &[u8]) -> (bool, &[u8]) {
    let start = text
        .iter()
        .position(|c| !matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C))
        .unwrap_or(text.len());
    let s = &text[start..];
    match s.first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    }
}

/// What C's `strtol(text, nullptr, 16)` makes of a short text: leading
/// blanks and a sign are taken, then hex digits as far as they go.
pub(crate) fn strtol_hex(text: &[u8]) -> i64 {
    let (negative, rest) = sign(text);
    let mut value: i64 = 0;
    for &c in rest {
        match (c as char).to_digit(16) {
            Some(d) => value = value.saturating_mul(16).saturating_add(i64::from(d)),
            None => break,
        }
    }
    if negative { -value } else { value }
}

/// C's `atoi`: leading blanks, a sign, decimal digits as far as they go.
pub(crate) fn atoi(text: &[u8]) -> i64 {
    let (negative, rest) = sign(text);
    let mut value: i64 = 0;
    for &c in rest {
        match (c as char).to_digit(10) {
            Some(d) => value = value.saturating_mul(10).saturating_add(i64::from(d)),
            None => break,
        }
    }
    if negative { -value } else { value }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_style_numbers() {
        assert_eq!(strtol_hex(b"1G"), 1);
        assert_eq!(strtol_hex(b"G1"), 0);
        assert_eq!(strtol_hex(b"-1"), -1);
        assert_eq!(strtol_hex(b"7E8"), 0x7E8);
        assert_eq!(atoi(b"500000"), 500_000);
        assert_eq!(atoi(b"x"), 0);
        assert_eq!(atoi(b" -25k"), -25);
        assert_eq!(parse_hex("0A1b"), vec![0x0A, 0x1B]);
        assert_eq!(hex(&[0x01, 0xAB], true), "01 AB");
        assert_eq!(hex(&[0x01, 0xAB], false), "01AB");
        assert_eq!(normalize(b"at sp 6\r\n"), "ATSP6");
    }
}
