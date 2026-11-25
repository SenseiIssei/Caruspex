//! Pages with drawing operators, and the file around them: header, objects,
//! cross-reference table, trailer.
//!
//! Coordinates are PDF points (1/72 inch) from the bottom left corner, as in
//! the file itself. The content streams stay uncompressed: a report is a few
//! kilobytes, and plain operators are easy to check in tests.

use std::fmt::Write as _;

use crate::encoding::encode;
use crate::fonts::Font;

/// A4 in points.
pub const A4_WIDTH: f32 = 595.28;
pub const A4_HEIGHT: f32 = 841.89;

/// An RGB colour, each part 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {
    /// From 0xRRGGBB, as in CSS.
    pub const fn hex(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xFF) as f32 / 255.0,
            g: ((rgb >> 8) & 0xFF) as f32 / 255.0,
            b: (rgb & 0xFF) as f32 / 255.0,
        }
    }
    pub const BLACK: Color = Color::hex(0x000000);
}

/// A number for a content stream: at most three decimals, no trailing
/// zeros, never "-0" or an exponent.
pub fn num(v: f32) -> String {
    if !v.is_finite() {
        return "0".to_string();
    }
    let mut s = format!("{:.3}", v);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".to_string();
    }
    s
}

/// A PDF literal string of WinAnsi bytes: ( ) and \ escaped, everything
/// outside printable ASCII as an octal escape, so the file stays ASCII.
pub(crate) fn literal(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() + 2);
    s.push('(');
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                s.push('\\');
                s.push(b as char);
            }
            0x20..=0x7E => s.push(b as char),
            _ => {
                let _ = write!(s, "\\{b:03o}");
            }
        }
    }
    s.push(')');
    s
}

/// One page and its content stream.
#[derive(Clone, Debug)]
pub struct Page {
    pub width: f32,
    pub height: f32,
    content: String,
    unknown: Vec<char>,
}

impl Page {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            content: String::new(),
            unknown: Vec::new(),
        }
    }

    /// The operators so far.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Characters that had to be written as "?" on this page.
    pub fn unknown_chars(&self) -> &[char] {
        &self.unknown
    }

    /// Text with its baseline starting at (x, y).
    pub fn text(&mut self, x: f32, y: f32, font: Font, size: f32, color: Color, text: &str) {
        self.text_spaced(x, y, font, size, color, 0.0, text);
    }

    /// The same with extra space after every character (Tc), for the
    /// spaced-out brand line.
    #[allow(clippy::too_many_arguments)]
    pub fn text_spaced(
        &mut self,
        x: f32,
        y: f32,
        font: Font,
        size: f32,
        color: Color,
        spacing: f32,
        text: &str,
    ) {
        let encoded = encode(text);
        self.unknown.extend(&encoded.unknown);
        if encoded.bytes.is_empty() {
            return;
        }
        let _ = write!(
            self.content,
            "BT\n{} {} {} rg\n/{} {} Tf\n",
            num(color.r),
            num(color.g),
            num(color.b),
            font.resource(),
            num(size)
        );
        if spacing != 0.0 {
            let _ = writeln!(self.content, "{} Tc", num(spacing));
        }
        let _ = write!(
            self.content,
            "{} {} Td\n{} Tj\n",
            num(x),
            num(y),
            literal(&encoded.bytes)
        );
        // Tc belongs to the graphics state and outlives ET.
        if spacing != 0.0 {
            self.content.push_str("0 Tc\n");
        }
        self.content.push_str("ET\n");
    }

    /// A straight line.
    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: Color) {
        let _ = write!(
            self.content,
            "{} {} {} RG\n{} w\n{} {} m\n{} {} l\nS\n",
            num(color.r),
            num(color.g),
            num(color.b),
            num(width),
            num(x1),
            num(y1),
            num(x2),
            num(y2)
        );
    }

    /// A filled rectangle, (x, y) its bottom left corner.
    pub(crate) fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let _ = write!(
            self.content,
            "{} {} {} rg\n{} {} {} {} re\nf\n",
            num(color.r),
            num(color.g),
            num(color.b),
            num(x),
            num(y),
            num(w),
            num(h)
        );
    }
}

/// A whole document.
#[derive(Clone, Debug, Default)]
pub struct Document {
    pub pages: Vec<Page>,
    /// Shown as the window title by most viewers.
    pub title: String,
    pub producer: String,
    /// "D:20261120143000" or empty; the writer has no clock of its own.
    pub creation_date: String,
}

impl Document {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            ..Self::default()
        }
    }

    /// Appends an empty page and returns it.
    pub(crate) fn add_page(&mut self, width: f32, height: f32) -> &mut Page {
        self.pages.push(Page::new(width, height));
        self.pages.last_mut().expect("just pushed")
    }

    /// Every character that was written as "?", in order, without repeats.
    pub fn unknown_chars(&self) -> Vec<char> {
        let mut out: Vec<char> = Vec::new();
        for c in self.pages.iter().flat_map(|p| p.unknown.iter()) {
            if !out.contains(c) {
                out.push(*c);
            }
        }
        out
    }

    /// The file. A document without pages gets one empty A4 page, since
    /// viewers refuse a PDF without any.
    pub fn to_bytes(&self) -> Vec<u8> {
        let blank = [Page::new(A4_WIDTH, A4_HEIGHT)];
        let pages: &[Page] = if self.pages.is_empty() {
            &blank
        } else {
            &self.pages
        };

        // Object numbers: 1 catalog, 2 page tree, 3 resources, 4 to 7 the
        // fonts, 8 the info dictionary, then page and content in turn.
        const FIRST_PAGE: usize = 9;
        let page_id = |i: usize| FIRST_PAGE + 2 * i;
        let mut objects: Vec<Vec<u8>> = Vec::new();
        objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
        let kids: Vec<String> = (0..pages.len())
            .map(|i| format!("{} 0 R", page_id(i)))
            .collect();
        objects.push(
            format!(
                "<< /Type /Pages /Kids [{}] /Count {} >>",
                kids.join(" "),
                pages.len()
            )
            .into_bytes(),
        );
        let fonts: Vec<String> = Font::ALL
            .iter()
            .enumerate()
            .map(|(i, f)| format!("/{} {} 0 R", f.resource(), 4 + i))
            .collect();
        objects.push(
            format!(
                "<< /Font << {} >> /ProcSet [/PDF /Text] >>",
                fonts.join(" ")
            )
            .into_bytes(),
        );
        for f in Font::ALL {
            objects.push(
                format!(
                    "<< /Type /Font /Subtype /Type1 /BaseFont /{} /Encoding /WinAnsiEncoding >>",
                    f.base_name()
                )
                .into_bytes(),
            );
        }
        let mut info = String::from("<<");
        if !self.title.is_empty() {
            let _ = write!(info, " /Title {}", text_string(&self.title));
        }
        if !self.producer.is_empty() {
            let _ = write!(info, " /Producer {}", text_string(&self.producer));
        }
        if !self.creation_date.is_empty() {
            let _ = write!(info, " /CreationDate {}", text_string(&self.creation_date));
        }
        info.push_str(" >>");
        objects.push(info.into_bytes());
        for (i, page) in pages.iter().enumerate() {
            objects.push(
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources 3 0 R /Contents {} 0 R >>",
                    num(page.width),
                    num(page.height),
                    page_id(i) + 1
                )
                .into_bytes(),
            );
            let data = page.content.as_bytes();
            let mut stream = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
            stream.extend_from_slice(data);
            stream.extend_from_slice(b"\nendstream");
            objects.push(stream);
        }

        // The second line marks the file as binary for transfer programs
        // (PDF 1.7, 7.5.2), although everything after it is ASCII.
        let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = Vec::with_capacity(objects.len());
        for (i, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        // Every entry is exactly 20 bytes, the line end included.
        let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
        for offset in offsets {
            // The space before the line feed is part of the entry.
            let _ = writeln!(table, "{offset:010} 00000 n ");
        }
        let _ = write!(
            table,
            "trailer\n<< /Size {} /Root 1 0 R /Info 8 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref
        );
        out.extend_from_slice(table.as_bytes());
        out
    }
}

/// A text string for the info dictionary: UTF-16BE with a byte order mark,
/// so titles in any language survive.
fn text_string(text: &str) -> String {
    let mut s = String::from("<FEFF");
    for unit in text.encode_utf16() {
        let _ = write!(s, "{unit:04X}");
    }
    s.push('>');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_short_and_plain() {
        assert_eq!(num(12.0), "12");
        assert_eq!(num(12.5), "12.5");
        assert_eq!(num(0.1234), "0.123");
        assert_eq!(num(-0.0001), "0");
        assert_eq!(num(f32::NAN), "0");
        assert_eq!(num(595.28), "595.28");
    }

    #[test]
    fn literal_strings_escape_parentheses_and_backslash() {
        assert_eq!(literal(b"a(b)c\\d"), "(a\\(b\\)c\\\\d)");
        assert_eq!(literal(&[0xFC, b'x', 0x0A]), "(\\374x\\012)");
        assert_eq!(literal(b""), "()");
    }

    #[test]
    fn text_strings_are_utf16() {
        assert_eq!(text_string("A\u{00E9}"), "<FEFF004100E9>");
        assert_eq!(text_string("\u{1F600}"), "<FEFFD83DDE00>");
    }

    #[test]
    fn text_operators() {
        let mut p = Page::new(100.0, 100.0);
        p.text(
            10.0,
            20.5,
            Font::HelveticaBold,
            9.0,
            Color::hex(0xFF0000),
            "(x)",
        );
        assert_eq!(
            p.content(),
            "BT\n1 0 0 rg\n/F2 9 Tf\n10 20.5 Td\n(\\(x\\)) Tj\nET\n"
        );
        p.text(0.0, 0.0, Font::Helvetica, 9.0, Color::BLACK, "\u{00AD}");
        assert!(
            p.content().ends_with("ET\n"),
            "nothing visible, no operators"
        );
        p.text(0.0, 0.0, Font::Helvetica, 9.0, Color::BLACK, "\u{4E2D}");
        assert_eq!(p.unknown_chars(), ['\u{4E2D}']);
    }
}
