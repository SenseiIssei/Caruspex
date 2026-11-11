//! Encoding helpers: value scaling, DTC bytes, support masks, hex text and
//! ISO-TP framing.

pub(crate) type Bytes = Vec<u8>;

fn lround(value: f64) -> i64 {
    value.round() as i64
}

pub(crate) fn u8_of(value: f64) -> u8 {
    lround(value).clamp(0, 255) as u8
}

pub(crate) fn put16(out: &mut Bytes, value: f64) {
    let v = lround(value).clamp(0, 65535);
    out.push(((v >> 8) & 0xFF) as u8);
    out.push((v & 0xFF) as u8);
}

pub(crate) fn put32(out: &mut Bytes, value: f64) {
    let v = lround(value).clamp(0, 0xFFFF_FFFF);
    for shift in [24, 16, 8, 0] {
        out.push(((v >> shift) & 0xFF) as u8);
    }
}

pub(crate) fn percent(value: f64) -> u8 {
    u8_of(value * 255.0 / 100.0)
}

pub(crate) fn trim_byte(value: f64) -> u8 {
    u8_of(value * 128.0 / 100.0 + 128.0)
}

/// "P0171" -> 0x01 0x71
pub(crate) fn put_dtc(out: &mut Bytes, code: &str) {
    let c = code.as_bytes();
    let first = match c[0] {
        b'C' => 1,
        b'B' => 2,
        b'U' => 3,
        _ => 0,
    };
    let digit = |ch: u8| -> u8 {
        if ch >= b'A' {
            ch - b'A' + 10
        } else {
            ch - b'0'
        }
    };
    out.push((first << 6) | (digit(c[1]) << 4) | digit(c[2]));
    out.push((digit(c[3]) << 4) | digit(c[4]));
}

/// Support bitmask for the identifiers base+1 .. base+32. The last bit says
/// whether the next range has anything, the way J1979 chains them.
pub(crate) fn support_mask(ids: &[i32], base: i32) -> Bytes {
    let mut mask: u32 = 0;
    for &id in ids {
        if id > base && id <= base + 32 {
            mask |= 1u32 << (32 - (id - base));
        }
        if id > base + 32 {
            mask |= 1;
        }
    }
    vec![
        (mask >> 24) as u8,
        (mask >> 16) as u8,
        (mask >> 8) as u8,
        mask as u8,
    ]
}

/// A range request is only answered when the chain reaches it.
pub(crate) fn range_reachable(ids: &[i32], base: i32) -> bool {
    if base == 0 {
        return true;
    }
    ids.iter().any(|&id| id > base)
}

const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

pub(crate) fn hex_digit(value: usize) -> char {
    HEX_DIGITS[value & 0xF] as char
}

pub(crate) fn hex2(value: i32) -> String {
    let v = value as usize;
    [hex_digit(v >> 4), hex_digit(v)].iter().collect()
}

pub(crate) fn hex3(value: i32) -> String {
    let v = value as usize;
    [hex_digit(v >> 8), hex_digit(v >> 4), hex_digit(v)]
        .iter()
        .collect()
}

pub(crate) fn is_hex(c: char) -> bool {
    c.is_ascii_digit() || ('A'..='F').contains(&c)
}

pub(crate) fn hex_value(c: char) -> i32 {
    if c <= '9' {
        c as i32 - '0' as i32
    } else {
        c as i32 - 'A' as i32 + 10
    }
}

pub(crate) fn all_hex(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_hex)
}

/// `c` == `prefix` followed by exactly `digits` hex digits; gives the value.
pub(crate) fn hex_arg(c: &str, prefix: &str, digits: usize) -> Option<i32> {
    if c.len() != prefix.len() + digits || !c.starts_with(prefix) {
        return None;
    }
    let rest = c.get(prefix.len()..)?;
    if !all_hex(rest) {
        return None;
    }
    Some(u64::from_str_radix(rest, 16).ok()? as i32)
}

/// Text after the first `n` bytes, empty when there are fewer.
pub(crate) fn after(c: &str, n: usize) -> &str {
    c.get(n..).unwrap_or("")
}

/// ISO-TP framing for one response on CAN, padded to eight bytes.
pub(crate) fn iso_tp_frames(payload: &[u8]) -> Vec<[u8; 8]> {
    let mut frames = Vec::new();
    let mut frame = [0xAAu8; 8];
    if payload.len() <= 7 {
        frame[0] = payload.len() as u8;
        frame[1..1 + payload.len()].copy_from_slice(payload);
        frames.push(frame);
        return frames;
    }
    frame[0] = 0x10 | ((payload.len() >> 8) & 0x0F) as u8;
    frame[1] = (payload.len() & 0xFF) as u8;
    frame[2..8].copy_from_slice(&payload[..6]);
    frames.push(frame);
    let mut pos = 6;
    let mut sequence = 1;
    while pos < payload.len() {
        let mut frame = [0xAAu8; 8];
        frame[0] = 0x20 | (sequence & 0x0F) as u8;
        let n = 7.min(payload.len() - pos);
        frame[1..1 + n].copy_from_slice(&payload[pos..pos + n]);
        frames.push(frame);
        pos += n;
        sequence += 1;
    }
    frames
}

pub(crate) fn protocol_description(protocol: i32) -> &'static str {
    match protocol {
        1 => "SAE J1850 PWM",
        2 => "SAE J1850 VPW",
        3 => "ISO 9141-2",
        4 => "ISO 14230-4 (KWP 5BAUD)",
        5 => "ISO 14230-4 (KWP FAST)",
        6 => "ISO 15765-4 (CAN 11/500)",
        7 => "ISO 15765-4 (CAN 29/500)",
        8 => "ISO 15765-4 (CAN 11/250)",
        9 => "ISO 15765-4 (CAN 29/250)",
        10 => "SAE J1939 (CAN 29/250)",
        11 => "USER1 (CAN 11/125)",
        12 => "USER2 (CAN 11/50)",
        _ => "AUTO",
    }
}
