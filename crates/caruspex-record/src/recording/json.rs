//! JSON, just as much as the recording format needs: one flat object per
//! line with strings, numbers and byte strings.

use std::collections::HashMap;

/// Escapes a string for JSON. With `bytes` every byte outside printable ASCII
/// becomes \u00XX and means exactly that byte; this is how the raw adapter
/// traffic is kept, which need not be valid UTF-8. Without `bytes` (header
/// texts) UTF-8 passes through and only control characters are escaped.
pub(super) fn json_string(s: &[u8], bytes: bool) -> Vec<u8> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = vec![b'"'];
    for &c in s {
        match c {
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\t' => out.extend_from_slice(b"\\t"),
            _ if c < 0x20 || c == 0x7F || (bytes && c >= 0x80) => {
                out.extend_from_slice(b"\\u00");
                out.push(DIGITS[(c >> 4) as usize]);
                out.push(DIGITS[(c & 0xF) as usize]);
            }
            _ => out.push(c),
        }
    }
    out.push(b'"');
    out
}

pub(super) enum JsonValue {
    Text(Vec<u8>),
    Number(f64),
    Other,
}

pub(super) struct FlatJsonParser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> FlatJsonParser<'a> {
    pub(super) fn new(s: &'a [u8]) -> Self {
        Self { s, i: 0 }
    }

    /// Parses one object whose values are strings, numbers, booleans or null.
    /// Nested objects and arrays are not part of the format and fail.
    pub(super) fn parse(&mut self) -> Option<HashMap<Vec<u8>, JsonValue>> {
        let mut out = HashMap::new();
        self.skip_space();
        if !self.take(b'{') {
            return None;
        }
        self.skip_space();
        if self.take(b'}') {
            return self.at_end().then_some(out);
        }
        loop {
            self.skip_space();
            let key = self.parse_string()?;
            self.skip_space();
            if !self.take(b':') {
                return None;
            }
            self.skip_space();
            let value = if self.peek() == b'"' {
                JsonValue::Text(self.parse_string()?)
            } else if self.peek() == b'-' || self.peek().is_ascii_digit() {
                JsonValue::Number(self.parse_number()?)
            } else if self.parse_literal() {
                JsonValue::Other
            } else {
                return None;
            };
            out.insert(key, value);
            self.skip_space();
            if self.take(b',') {
                continue;
            }
            if self.take(b'}') {
                return self.at_end().then_some(out);
            }
            return None;
        }
    }

    fn peek(&self) -> u8 {
        self.s.get(self.i).copied().unwrap_or(0)
    }

    fn take(&mut self, c: u8) -> bool {
        if self.i >= self.s.len() || self.s[self.i] != c {
            return false;
        }
        self.i += 1;
        true
    }

    fn skip_space(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\r' | b'\n') {
            self.i += 1;
        }
    }

    fn at_end(&mut self) -> bool {
        self.skip_space();
        self.i == self.s.len()
    }

    fn parse_hex4(&mut self) -> Option<u32> {
        if self.i + 4 > self.s.len() {
            return None;
        }
        let mut value = 0u32;
        for k in 0..4 {
            let v = (self.s[self.i + k] as char).to_digit(16)?;
            value = value * 16 + v;
        }
        self.i += 4;
        Some(value)
    }

    fn append_utf8(out: &mut Vec<u8>, cp: u32) {
        if cp < 0x80 {
            out.push(cp as u8);
        } else if cp < 0x800 {
            out.push((0xC0 | (cp >> 6)) as u8);
            out.push((0x80 | (cp & 0x3F)) as u8);
        } else if cp < 0x10000 {
            out.push((0xE0 | (cp >> 12)) as u8);
            out.push((0x80 | ((cp >> 6) & 0x3F)) as u8);
            out.push((0x80 | (cp & 0x3F)) as u8);
        } else {
            out.push((0xF0 | (cp >> 18)) as u8);
            out.push((0x80 | ((cp >> 12) & 0x3F)) as u8);
            out.push((0x80 | ((cp >> 6) & 0x3F)) as u8);
            out.push((0x80 | (cp & 0x3F)) as u8);
        }
    }

    fn parse_string(&mut self) -> Option<Vec<u8>> {
        if !self.take(b'"') {
            return None;
        }
        let mut out = Vec::new();
        while self.i < self.s.len() {
            let c = self.s[self.i];
            self.i += 1;
            if c == b'"' {
                return Some(out);
            }
            if c < 0x20 {
                return None;
            }
            if c != b'\\' {
                out.push(c);
                continue;
            }
            if self.i >= self.s.len() {
                return None;
            }
            let e = self.s[self.i];
            self.i += 1;
            match e {
                b'"' => out.push(b'"'),
                b'\\' => out.push(b'\\'),
                b'/' => out.push(b'/'),
                b'b' => out.push(0x08),
                b'f' => out.push(0x0C),
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'u' => {
                    let cp = self.parse_hex4()?;
                    if cp < 0x100 {
                        // One raw byte, see json_string.
                        out.push(cp as u8);
                    } else if (0xD800..0xDC00).contains(&cp) {
                        if !self.take(b'\\') || !self.take(b'u') {
                            return None;
                        }
                        let low = self.parse_hex4()?;
                        if !(0xDC00..=0xDFFF).contains(&low) {
                            return None;
                        }
                        Self::append_utf8(
                            &mut out,
                            0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00),
                        );
                    } else {
                        Self::append_utf8(&mut out, cp);
                    }
                }
                _ => return None,
            }
        }
        None // the closing quote is missing: a cut-off line
    }

    /// Decimal number with optional fraction and exponent, parsed by hand
    /// exactly like the C++ version so both read the same time stamps.
    fn parse_number(&mut self) -> Option<f64> {
        let negative = self.take(b'-');
        if !self.peek().is_ascii_digit() {
            return None;
        }
        let mut value = 0.0f64;
        while self.peek().is_ascii_digit() {
            value = value * 10.0 + f64::from(self.s[self.i] - b'0');
            self.i += 1;
        }
        if self.take(b'.') {
            if !self.peek().is_ascii_digit() {
                return None;
            }
            let mut scale = 0.1f64;
            while self.peek().is_ascii_digit() {
                value += f64::from(self.s[self.i] - b'0') * scale;
                self.i += 1;
                scale /= 10.0;
            }
        }
        if self.peek() == b'e' || self.peek() == b'E' {
            self.i += 1;
            let mut neg_exp = false;
            if self.peek() == b'+' || self.peek() == b'-' {
                neg_exp = self.s[self.i] == b'-';
                self.i += 1;
            }
            if !self.peek().is_ascii_digit() {
                return None;
            }
            let mut exponent = 0;
            while self.peek().is_ascii_digit() && exponent < 400 {
                exponent = exponent * 10 + i32::from(self.s[self.i] - b'0');
                self.i += 1;
            }
            while self.peek().is_ascii_digit() {
                self.i += 1;
            }
            for _ in 0..exponent {
                value = if neg_exp { value / 10.0 } else { value * 10.0 };
            }
        }
        Some(if negative { -value } else { value })
    }

    fn parse_literal(&mut self) -> bool {
        for word in [&b"true"[..], b"false", b"null"] {
            if self.s[self.i.min(self.s.len())..].starts_with(word) {
                self.i += word.len();
                return true;
            }
        }
        false
    }
}

pub(super) fn text_field(fields: &HashMap<Vec<u8>, JsonValue>, key: &str) -> Vec<u8> {
    match fields.get(key.as_bytes()) {
        Some(JsonValue::Text(text)) => text.clone(),
        _ => Vec::new(),
    }
}

pub(super) fn string_field(fields: &HashMap<Vec<u8>, JsonValue>, key: &str) -> String {
    String::from_utf8_lossy(&text_field(fields, key)).into_owned()
}
