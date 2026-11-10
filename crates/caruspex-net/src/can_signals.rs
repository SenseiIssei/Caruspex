//! Values a car broadcasts on its CAN bus by itself, as a DBC file describes
//! them: no request, the control units send these frames all the time. Tesla
//! is the main case (no standard OBD at all), but many makers broadcast
//! battery and drive values the same way.
//!
//! A signal is written the way DBC files write it, prefixed with "DBC:":
//!
//! ```text
//! DBC:0|16@1+(0.01,0)              bits 0 to 15, Intel order, unsigned
//! DBC:16|16@1-(-0.1,0)             signed, factor -0.1
//! DBC:2|12@1+(0.002,0) m1@0|2@1    only in frames whose multiplexer
//!                                  (bits 0 and 1) has the value 1
//! ```
//!
//! start|length@order sign (factor,offset); order 1 is Intel (little
//! endian, start is the lowest bit), 0 is Motorola (big endian, start is the
//! highest bit in DBC numbering). The value is raw * factor + offset.
//! The request of such a value is "CAN" plus the 11-bit id: "CAN132".

use caruspex_i18n::l;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanSignal {
    pub start: i32,
    pub length: i32,
    pub intel: bool,
    pub is_signed: bool,
    pub factor: f64,
    pub offset: f64,
    /// Multiplexed signals: only frames whose multiplexer field has `mux_value`.
    pub mux_start: i32,
    pub mux_length: i32,
    pub mux_intel: bool,
    /// -1: not multiplexed.
    pub mux_value: i32,
}

impl Default for CanSignal {
    fn default() -> Self {
        Self {
            start: 0,
            length: 0,
            intel: true,
            is_signed: false,
            factor: 1.0,
            offset: 0.0,
            mux_start: -1,
            mux_length: 0,
            mux_intel: true,
            mux_value: -1,
        }
    }
}

/// Decimal digits from `at` on, as `atoi` reads them.
fn number(s: &[u8], at: &mut usize) -> Option<i32> {
    let begin = *at;
    while *at < s.len() && s[*at].is_ascii_digit() {
        *at += 1;
    }
    if *at == begin {
        return None;
    }
    Some(digits(&s[begin..*at]))
}

/// An all-digit text as a number; one that does not fit is too big for
/// every check that follows.
fn digits(s: &[u8]) -> i32 {
    std::str::from_utf8(s)
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(i32::MAX)
}

/// "start|length@order sign": the field part of a DBC signal.
/// Returns start, length, Intel order and (if asked for) the sign.
fn parse_field(s: &[u8], at: &mut usize, with_sign: bool) -> Option<(i32, i32, bool, bool)> {
    let start = number(s, at)?;
    if *at >= s.len() || s[*at] != b'|' {
        return None;
    }
    *at += 1;
    let length = number(s, at)?;
    if *at >= s.len() || s[*at] != b'@' {
        return None;
    }
    *at += 1;
    if *at >= s.len() || (s[*at] != b'0' && s[*at] != b'1') {
        return None;
    }
    let intel = s[*at] == b'1';
    *at += 1;
    let mut signed = false;
    if with_sign {
        if *at >= s.len() || (s[*at] != b'+' && s[*at] != b'-') {
            return None;
        }
        signed = s[*at] == b'-';
        *at += 1;
    }
    ((1..=64).contains(&length) && (0..512).contains(&start))
        .then_some((start, length, intel, signed))
}

/// `strtod` over the whole text: an empty text is 0, anything left over fails.
fn whole_number(s: &[u8]) -> Option<f64> {
    if s.is_empty() {
        return Some(0.0);
    }
    std::str::from_utf8(s).ok()?.parse().ok()
}

/// "DBC:..." as described above. `Err` with a reason in the active language.
pub fn parse_can_signal(text: &str) -> Result<CanSignal, String> {
    let s: Vec<u8> = text
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != 0x0B)
        .collect();
    let fail = || {
        format!(
            "{}{}",
            l(
                "Kein gültiges CAN-Signal (erwartet z.B. DBC:0|16@1+(0.01,0)): ",
                "Not a valid CAN signal (expected e.g. DBC:0|16@1+(0.01,0)): "
            ),
            text
        )
    };
    if !s.starts_with(b"DBC:") {
        return Err(fail());
    }
    let mut at = 4;
    let mut sig = CanSignal::default();
    let (start, length, intel, signed) = parse_field(&s, &mut at, true).ok_or_else(fail)?;
    sig.start = start;
    sig.length = length;
    sig.intel = intel;
    sig.is_signed = signed;
    if at >= s.len() || s[at] != b'(' {
        return Err(fail());
    }
    at += 1;
    let comma = s[at..].iter().position(|&c| c == b',').map(|p| p + at);
    let close = s[at..].iter().position(|&c| c == b')').map(|p| p + at);
    let (Some(comma), Some(close)) = (comma, close) else {
        return Err(fail());
    };
    if comma > close {
        return Err(fail());
    }
    sig.factor = whole_number(&s[at..comma]).ok_or_else(fail)?;
    sig.offset = whole_number(&s[comma + 1..close]).ok_or_else(fail)?;
    at = close + 1;
    if at < s.len() {
        // Multiplexer: m<value>@<start>|<length>[@<order>]
        if s[at] != b'm' {
            return Err(fail());
        }
        at += 1;
        let begin = at;
        while at < s.len() && s[at].is_ascii_digit() {
            at += 1;
        }
        if at == begin || at >= s.len() || s[at] != b'@' {
            return Err(fail());
        }
        sig.mux_value = digits(&s[begin..at]);
        at += 1;
        // The order is optional here; Intel unless said otherwise.
        let mut rest = s[at..].to_vec();
        if !rest.contains(&b'@') {
            rest.extend_from_slice(b"@1");
        }
        let mut r = 0;
        let (mux_start, mux_length, mux_intel, _) =
            parse_field(&rest, &mut r, false).ok_or_else(fail)?;
        if r != rest.len() {
            return Err(fail());
        }
        sig.mux_start = mux_start;
        sig.mux_length = mux_length;
        sig.mux_intel = mux_intel;
    }
    Ok(sig)
}

/// "CAN132" -> 0x132. `None` if it is not "CAN" plus an 11-bit id in hex.
pub fn parse_monitor_request(request: &str) -> Option<u32> {
    let s = request.to_ascii_uppercase();
    if s.len() < 4 || s.len() > 6 || !s.starts_with("CAN") {
        return None;
    }
    let id = &s[3..];
    if !id.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let can_id = u32::from_str_radix(id, 16).ok()?;
    (can_id <= 0x7FF).then_some(can_id)
}

/// The raw bits of a field, as DBC counts them.
pub fn can_bits(frame: &[u8], start: i32, length: i32, intel: bool) -> u64 {
    let mut raw: u64 = 0;
    if intel {
        // Bit i of the frame is bit (i % 8) of byte i / 8, the field runs
        // upwards from start.
        for i in 0..length {
            let pos = start + i;
            let byte = (pos / 8) as usize;
            if byte >= frame.len() {
                break;
            }
            raw |= u64::from((frame[byte] >> (pos % 8)) & 1) << i;
        }
        return raw;
    }
    // Motorola: start is the most significant bit; within a byte the field
    // goes down towards bit 0, then continues at bit 7 of the next byte.
    let mut pos = start;
    for _ in 0..length {
        let byte = (pos / 8) as usize;
        let bit = if byte < frame.len() {
            (frame[byte] >> (pos % 8)) & 1
        } else {
            0
        };
        raw = (raw << 1) | u64::from(bit);
        pos = if pos % 8 == 0 { pos + 15 } else { pos - 1 };
    }
    raw
}

/// Highest frame byte a field touches, to tell a short frame.
fn last_byte(start: i32, length: i32, intel: bool) -> i32 {
    if intel {
        return (start + length - 1) / 8;
    }
    let mut pos = start;
    let mut highest = pos / 8;
    for _ in 1..length {
        pos = if pos % 8 == 0 { pos + 15 } else { pos - 1 };
        highest = highest.max(pos / 8);
    }
    highest
}

/// The value of a signal in one frame. `None` if the frame is too short or a
/// multiplexed signal is not in this frame.
pub fn decode_can_signal(signal: &CanSignal, frame: &[u8]) -> Option<f64> {
    if signal.length <= 0 {
        return None;
    }
    let size = frame.len() as i64;
    if i64::from(last_byte(signal.start, signal.length, signal.intel)) >= size {
        return None;
    }
    if signal.mux_value >= 0 {
        if i64::from(last_byte(
            signal.mux_start,
            signal.mux_length,
            signal.mux_intel,
        )) >= size
        {
            return None;
        }
        let mux = can_bits(frame, signal.mux_start, signal.mux_length, signal.mux_intel);
        if mux != signal.mux_value as u64 {
            return None;
        }
    }
    let raw = can_bits(frame, signal.start, signal.length, signal.intel);
    let mut number = raw as f64;
    if signal.is_signed && signal.length < 64 && (raw >> (signal.length - 1)) & 1 != 0 {
        number = (raw as i64 - (1i64 << signal.length)) as f64;
    }
    let value = number * signal.factor + signal.offset;
    value.is_finite().then_some(value)
}
