//! A small JSON writer for machine-readable output (the console tool's
//! --json, reports). No dependency, nothing clever: containers are opened and
//! closed explicitly, inside an object every value is preceded by `key()`,
//! and the output is indented by two spaces.
//!
//! Strings are escaped after RFC 8259. Bytes that are not well-formed UTF-8
//! (a clone adapter may send any byte in its version string) become U+FFFD,
//! so the output always parses. Numbers that are not finite become null.

/// Anything `JsonWriter::value` and `JsonWriter::field` accept.
pub trait ToJson {
    fn write_json(&self, json: &mut JsonWriter);
}

impl ToJson for str {
    fn write_json(&self, json: &mut JsonWriter) {
        json.separate();
        json.quote(self.as_bytes());
    }
}
impl ToJson for String {
    fn write_json(&self, json: &mut JsonWriter) {
        self.as_str().write_json(json);
    }
}
impl ToJson for bool {
    fn write_json(&self, json: &mut JsonWriter) {
        json.separate();
        json.out.push_str(if *self { "true" } else { "false" });
    }
}
macro_rules! json_integer {
    ($($t:ty),*) => {$(
        impl ToJson for $t {
            fn write_json(&self, json: &mut JsonWriter) {
                json.separate();
                json.out.push_str(&self.to_string());
            }
        }
    )*};
}
json_integer!(i32, i64, u32, u64, usize);
impl ToJson for f64 {
    fn write_json(&self, json: &mut JsonWriter) {
        if !self.is_finite() {
            json.null();
            return;
        }
        json.separate();
        json.out.push_str(&format_g10(*self));
    }
}
impl<T: ToJson + ?Sized> ToJson for &T {
    fn write_json(&self, json: &mut JsonWriter) {
        (**self).write_json(json);
    }
}

/// `printf("%.10g")` for a finite number.
fn format_g10(d: f64) -> String {
    if d == 0.0 {
        return if d.is_sign_negative() { "-0" } else { "0" }.to_string();
    }
    const PRECISION: i32 = 10;
    let sci = format!("{:.*e}", (PRECISION - 1) as usize, d);
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    if !(-4..PRECISION).contains(&exp) {
        let mantissa = if mantissa.contains('.') {
            mantissa.trim_end_matches('0').trim_end_matches('.')
        } else {
            mantissa
        };
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{mantissa}e{sign}{:02}", exp.abs())
    } else {
        let fixed = format!("{:.*}", (PRECISION - 1 - exp).max(0) as usize, d);
        if fixed.contains('.') {
            fixed
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        } else {
            fixed
        }
    }
}

#[derive(Default)]
pub struct JsonWriter {
    out: String,
    // One flag per open container: still empty?
    levels: Vec<bool>,
    after_key: bool,
}

impl JsonWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin_object(&mut self) -> &mut Self {
        self.open('{')
    }
    pub fn end_object(&mut self) -> &mut Self {
        self.close('}')
    }
    pub fn begin_array(&mut self) -> &mut Self {
        self.open('[')
    }
    pub fn end_array(&mut self) -> &mut Self {
        self.close(']')
    }

    pub fn key(&mut self, name: &str) -> &mut Self {
        self.separate();
        self.quote(name.as_bytes());
        self.out.push_str(": ");
        self.after_key = true;
        self
    }

    pub fn value<T: ToJson + ?Sized>(&mut self, v: &T) -> &mut Self {
        v.write_json(self);
        self
    }

    /// A string that may hold bytes outside UTF-8; those become U+FFFD.
    pub fn value_bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.separate();
        self.quote(bytes);
        self
    }

    pub fn null(&mut self) -> &mut Self {
        self.separate();
        self.out.push_str("null");
        self
    }

    /// Shorthand for a key with its value.
    pub fn field<T: ToJson + ?Sized>(&mut self, name: &str, v: &T) -> &mut Self {
        self.key(name);
        self.value(v)
    }

    pub fn string_list<S: AsRef<str>>(&mut self, name: &str, list: &[S]) -> &mut Self {
        self.key(name).begin_array();
        for s in list {
            self.value(s.as_ref());
        }
        self.end_array()
    }

    pub fn text(&self) -> String {
        format!("{}\n", self.out)
    }

    fn open(&mut self, bracket: char) -> &mut Self {
        self.separate();
        self.out.push(bracket);
        self.levels.push(true);
        self
    }

    fn close(&mut self, bracket: char) -> &mut Self {
        let empty = self.levels.last().copied().unwrap_or(true);
        self.levels.pop();
        if !empty {
            self.newline();
        }
        self.out.push(bracket);
        self
    }

    // Comma and line break before the next element, nothing after a key.
    fn separate(&mut self) {
        if self.after_key {
            self.after_key = false;
            return;
        }
        let Some(empty) = self.levels.last_mut() else {
            return;
        };
        if !*empty {
            self.out.push(',');
        }
        *empty = false;
        self.newline();
    }

    fn newline(&mut self) {
        self.out.push('\n');
        for _ in 0..self.levels.len() * 2 {
            self.out.push(' ');
        }
    }

    fn quote(&mut self, s: &[u8]) {
        self.out.push('"');
        let mut i = 0;
        while i < s.len() {
            let c = s[i];
            if c < 0x80 {
                match c {
                    b'"' => self.out.push_str("\\\""),
                    b'\\' => self.out.push_str("\\\\"),
                    0x08 => self.out.push_str("\\b"),
                    0x0C => self.out.push_str("\\f"),
                    b'\n' => self.out.push_str("\\n"),
                    b'\r' => self.out.push_str("\\r"),
                    b'\t' => self.out.push_str("\\t"),
                    _ if c < 0x20 => self.out.push_str(&format!("\\u{c:04x}")),
                    _ => self.out.push(c as char),
                }
                i += 1;
                continue;
            }
            match utf8_length(s, i) {
                0 => {
                    self.out.push_str("\\ufffd");
                    i += 1;
                }
                length => {
                    // Well-formed, checked by utf8_length.
                    self.out
                        .push_str(&String::from_utf8_lossy(&s[i..i + length]));
                    i += length;
                }
            }
        }
        self.out.push('"');
    }
}

/// Length of the well-formed UTF-8 sequence starting at s[i], 0 if it is
/// not one (stray continuation byte, overlong form, surrogate, cut off).
fn utf8_length(s: &[u8], i: usize) -> usize {
    let c = s[i];
    let (length, mut code, min) = match c {
        0xC2..=0xDF => (2, u32::from(c & 0x1F), 0x80),
        0xE0..=0xEF => (3, u32::from(c & 0x0F), 0x800),
        0xF0..=0xF4 => (4, u32::from(c & 0x07), 0x10000),
        _ => return 0,
    };
    if i + length > s.len() {
        return 0;
    }
    for k in 1..length {
        if s[i + k] & 0xC0 != 0x80 {
            return 0;
        }
        code = (code << 6) | u32::from(s[i + k] & 0x3F);
    }
    if code < min || code > 0x10FFFF || (0xD800..=0xDFFF).contains(&code) {
        return 0;
    }
    length
}

#[cfg(test)]
mod tests {
    use super::*;

    // One string value as the writer puts it out, without the line break.
    fn quoted(s: &[u8]) -> String {
        let mut json = JsonWriter::new();
        json.value_bytes(s);
        let mut out = json.text();
        out.pop();
        out
    }

    #[test]
    fn json_strings_are_escaped() {
        assert_eq!(quoted(b"plain"), "\"plain\"");
        assert_eq!(quoted(b"say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(quoted(b"C:\\obd"), "\"C:\\\\obd\"");
        assert_eq!(quoted(b"a\nb\r\tc"), "\"a\\nb\\r\\tc\"");
        assert_eq!(quoted(b"\x01\x1f"), "\"\\u0001\\u001f\"");
        assert_eq!(quoted(b"a\0b"), "\"a\\u0000b\"");
        assert_eq!(quoted(b"/"), "\"/\"");

        // Well-formed UTF-8 stays as it is.
        let text = "Kühlmittel 90 °C, Škoda, \u{20AC}, \u{1F697}";
        assert_eq!(quoted(text.as_bytes()), format!("\"{text}\""));

        // Anything else becomes U+FFFD, byte by byte.
        assert_eq!(quoted(b"ELM\xFF"), "\"ELM\\ufffd\"");
        assert_eq!(quoted(b"\x80"), "\"\\ufffd\""); // stray continuation byte
        assert_eq!(quoted(b"\xC3"), "\"\\ufffd\""); // cut off
        assert_eq!(quoted(b"\xC0\xAF"), "\"\\ufffd\\ufffd\""); // overlong
        assert_eq!(quoted(b"\xED\xA0\x80"), "\"\\ufffd\\ufffd\\ufffd\""); // surrogate
        assert_eq!(quoted(b"\xC3x"), "\"\\ufffdx\"");
    }

    #[test]
    fn json_structure_numbers_and_indentation() {
        let mut json = JsonWriter::new();
        json.begin_object()
            .field("name", "demo")
            .field("count", &3)
            .field("ok", &true)
            .field("ratio", &0.25)
            .field("nan", &f64::NAN)
            .field("inf", &f64::INFINITY)
            .key("none")
            .null()
            .key("empty")
            .begin_array()
            .end_array()
            .key("list")
            .begin_array()
            .value(&1)
            .begin_object()
            .end_object()
            .end_array()
            .string_list("words", &["a", "b"])
            .end_object();
        let expected = [
            "{",
            "  \"name\": \"demo\",",
            "  \"count\": 3,",
            "  \"ok\": true,",
            "  \"ratio\": 0.25,",
            "  \"nan\": null,",
            "  \"inf\": null,",
            "  \"none\": null,",
            "  \"empty\": [],",
            "  \"list\": [",
            "    1,",
            "    {}",
            "  ],",
            "  \"words\": [",
            "    \"a\",",
            "    \"b\"",
            "  ]",
            "}",
            "",
        ]
        .join("\n");
        assert_eq!(json.text(), expected);

        let mut numbers = JsonWriter::new();
        numbers
            .begin_array()
            .value(&-0.5)
            .value(&1234.5678)
            .value(&1e-7)
            .value(&-12)
            .end_array();
        assert_eq!(
            numbers.text(),
            "[\n  -0.5,\n  1234.5678,\n  1e-07,\n  -12\n]\n"
        );
    }

    #[test]
    fn g10_matches_printf() {
        assert_eq!(format_g10(0.0), "0");
        assert_eq!(format_g10(100.0), "100");
        assert_eq!(format_g10(1e10), "1e+10");
        assert_eq!(format_g10(123456789.0), "123456789");
        assert_eq!(format_g10(0.0001), "0.0001");
        assert_eq!(format_g10(0.00001234), "1.234e-05");
        assert_eq!(format_g10(1.0 / 3.0), "0.3333333333");
        assert_eq!(format_g10(12345678901.0), "1.23456789e+10");
        assert_eq!(format_g10(9999999999.5), "1e+10");
    }
}
