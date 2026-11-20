//! Text to the bytes of WinAnsiEncoding, the encoding every PDF viewer
//! knows for the standard fonts.
//!
//! Why no embedded font: the standard 14 fonts need no font file, so the
//! writer stays tiny and the PDF a few kilobytes. The fonts the repository
//! ships (Inter, JetBrains Mono) are already cut down to Latin-1, so
//! embedding them would show no more characters than WinAnsi does. Scripts
//! that WinAnsi cannot show (Cyrillic, Arabic, Devanagari, CJK, most of
//! Latin Extended-A) would need large fonts and, for Arabic and Hindi, text
//! shaping; the caller picks a language the encoding covers instead.
//!
//! No character is dropped or turned into garbage without a trace: what
//! WinAnsi lacks is written with an explicit stand-in from [`fallback`]
//! ("≥" as ">=", "ł" as "l"), and anything without one becomes "?" and is
//! listed in [`Encoded::unknown`], so the caller can say so.

/// Codes 0x80 to 0x9F of WinAnsiEncoding (PDF 1.7, annex D). Code page
/// 1252 leaves five of them unused (NUL here), and they are never written.
const HIGH: [char; 32] = [
    '\u{20AC}', '\0', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\0', '\u{017D}', '\0', '\0',
    '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}',
    '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\0', '\u{017E}', '\u{0178}',
];

/// The WinAnsi byte of `c`, if the encoding has it. Control characters and
/// the soft hyphen have none: they are no glyphs.
pub fn winansi_byte(c: char) -> Option<u8> {
    let u = c as u32;
    match u {
        0x20..=0x7E => Some(u as u8),
        // 0xAD is the soft hyphen; WinAnsi would print it as a visible hyphen.
        0xA0..=0xFF if u != 0xAD => Some(u as u8),
        _ if c != '\0' => HIGH.iter().position(|&h| h == c).map(|i| 0x80 + i as u8),
        _ => None,
    }
}

/// Latin Extended-A (U+0100 to U+017F) by its base letter, for names and
/// stray words in a text that is otherwise WinAnsi. Positions that WinAnsi
/// has itself (Œ œ Š š Ÿ Ž ž) never reach this table.
const LATIN_EXTENDED_A: &[u8; 128] = b"AaAaAaCcCcCcCcDd\
DdEeEeEeEeEeGgGg\
GgGgHhHhIiIiIiIi\
IiIiJjKkkLlLlLlL\
lLlNnNnNnnNnOoOo\
OoOoRrRrRrSsSsSs\
SsTtTtTtUuUuUuUu\
UuUuWwYyYZzZzZzs";

/// The explicit stand-in for a character WinAnsi does not have, made of
/// WinAnsi characters (possibly empty for invisible ones). None means there
/// is no sensible stand-in.
pub fn fallback(c: char) -> Option<&'static str> {
    if winansi_byte(c).is_some() {
        return None;
    }
    let u = c as u32;
    Some(match c {
        '\t' | '\n' | '\r' => " ",
        // Invisible by definition: soft hyphen, zero-width characters, BOM.
        '\u{00AD}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}' => "",
        '\u{2002}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}' => " ",
        '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2212}' => "-",
        '\u{2015}' => "-",
        '\u{201B}' | '\u{2032}' => "'",
        '\u{201F}' | '\u{2033}' => "\"",
        '\u{2044}' | '\u{2215}' => "/",
        '\u{2264}' => "<=",
        '\u{2265}' => ">=",
        '\u{2260}' => "!=",
        '\u{2248}' | '\u{223C}' => "~",
        '\u{2190}' => "<-",
        '\u{2192}' => "->",
        '\u{2194}' => "<->",
        '\u{2191}' => "^",
        '\u{2193}' => "v",
        '\u{2126}' | '\u{03A9}' => "Ohm",
        '\u{2103}' => "\u{00B0}C",
        '\u{2116}' => "No.",
        '\u{2070}' => "0",
        '\u{2074}'..='\u{2079}' => DIGITS[(u - 0x2070) as usize],
        '\u{2080}'..='\u{2089}' => DIGITS[(u - 0x2080) as usize],
        '\u{0132}' => "IJ",
        '\u{0133}' => "ij",
        '\u{0218}' => "S",
        '\u{0219}' => "s",
        '\u{021A}' => "T",
        '\u{021B}' => "t",
        '\u{0100}'..='\u{017F}' => {
            let i = (u - 0x100) as usize;
            LATIN_EXTENDED_A_STR.get(i..i + 1)?
        }
        _ => return None,
    })
}

const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

// The same table as text, so `fallback` can hand out slices of it.
const LATIN_EXTENDED_A_STR: &str = match std::str::from_utf8(LATIN_EXTENDED_A) {
    Ok(s) => s,
    Err(_) => panic!("ASCII table"),
};

/// Text in WinAnsi bytes and what could not be written as itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Encoded {
    pub bytes: Vec<u8>,
    /// Characters with an explicit stand-in from [`fallback`].
    pub approximated: Vec<char>,
    /// Characters without one, written as "?".
    pub unknown: Vec<char>,
}

/// Encodes `text` for the standard fonts.
pub fn encode(text: &str) -> Encoded {
    let mut out = Encoded::default();
    for c in text.chars() {
        if let Some(b) = winansi_byte(c) {
            out.bytes.push(b);
        } else if let Some(stand_in) = fallback(c) {
            // Every stand-in is plain WinAnsi, checked by a test.
            out.bytes.extend(stand_in.chars().filter_map(winansi_byte));
            out.approximated.push(c);
        } else {
            out.bytes.push(b'?');
            out.unknown.push(c);
        }
    }
    out
}

/// True if every character of `text` is in WinAnsi itself.
pub fn covers(text: &str) -> bool {
    text.chars()
        .all(|c| winansi_byte(c).is_some() || matches!(c, '\n' | '\r' | '\t'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_and_latin1_are_themselves() {
        assert_eq!(encode("P0171 ok").bytes, b"P0171 ok");
        assert_eq!(encode("Grüße").bytes, [b'G', b'r', 0xFC, 0xDF, b'e']);
        assert_eq!(encode("\u{00E9}\u{00F1}\u{00E7}").bytes, [0xE9, 0xF1, 0xE7]);
        assert!(encode("äöü ÄÖÜ ß àéèêç ñ ¿¡ ºª °µ²³").unknown.is_empty());
    }

    #[test]
    fn the_high_block_follows_code_page_1252() {
        assert_eq!(winansi_byte('\u{20AC}'), Some(0x80));
        assert_eq!(winansi_byte('\u{2026}'), Some(0x85));
        assert_eq!(winansi_byte('\u{0160}'), Some(0x8A));
        assert_eq!(winansi_byte('\u{2013}'), Some(0x96));
        assert_eq!(winansi_byte('\u{2014}'), Some(0x97));
        assert_eq!(winansi_byte('\u{0178}'), Some(0x9F));
        // The unused codes are never produced, and no byte stands for two
        // characters.
        let mut seen = [false; 256];
        for c in (0..=0xFFFFu32).filter_map(char::from_u32) {
            if let Some(b) = winansi_byte(c) {
                assert!(!seen[b as usize], "byte {b:02X} twice");
                seen[b as usize] = true;
            }
        }
        for b in [0x7Fusize, 0x81, 0x8D, 0x8F, 0x90, 0x9D, 0xAD] {
            assert!(!seen[b], "{b:02X}");
        }
        assert_eq!(seen.iter().filter(|&&s| s).count(), 256 - 32 - 7);
    }

    #[test]
    fn soft_hyphen_and_controls_are_no_glyphs() {
        assert_eq!(winansi_byte('\u{00AD}'), None);
        assert_eq!(winansi_byte('\u{0007}'), None);
        assert_eq!(winansi_byte('\0'), None);
        let e = encode("Ab\u{00AD}gas\tx");
        assert_eq!(e.bytes, b"Abgas x");
        assert!(e.unknown.is_empty());
    }

    #[test]
    fn missing_characters_get_an_explicit_stand_in() {
        let e = encode("\u{2265}0.5 \u{2264}2 \u{2248}3 CO\u{2082} \u{0141}\u{00F3}d\u{017A}");
        let mut expected = b">=0.5 <=2 ~3 CO2 L".to_vec();
        expected.extend([0xF3, b'd', b'z']);
        assert_eq!(e.bytes, expected);
        assert_eq!(
            e.approximated,
            vec![
                '\u{2265}', '\u{2264}', '\u{2248}', '\u{2082}', '\u{0141}', '\u{017A}'
            ]
        );
        assert!(e.unknown.is_empty());
    }

    #[test]
    fn unknown_characters_become_a_visible_question_mark() {
        let e = encode("\u{041C}\u{0438}\u{0440} \u{4E2D}");
        assert_eq!(e.bytes, b"??? ?");
        assert_eq!(
            e.unknown,
            vec!['\u{041C}', '\u{0438}', '\u{0440}', '\u{4E2D}']
        );
        assert!(e.approximated.is_empty());
    }

    #[test]
    fn every_stand_in_is_plain_winansi() {
        for c in (0..=0xFFFFu32).filter_map(char::from_u32) {
            if let Some(s) = fallback(c) {
                assert!(
                    s.chars().all(|x| winansi_byte(x).is_some()),
                    "stand-in of U+{:04X}",
                    c as u32
                );
                // A stand-in never hides a character WinAnsi has.
                assert!(winansi_byte(c).is_none(), "U+{:04X}", c as u32);
            }
        }
        // The whole of Latin Extended-A has one.
        for u in 0x100..=0x17F {
            let c = char::from_u32(u).unwrap();
            assert!(
                winansi_byte(c).is_some() || fallback(c).is_some(),
                "U+{u:04X}"
            );
        }
        assert_eq!(fallback('\u{0142}'), Some("l"));
        assert_eq!(fallback('\u{0131}'), Some("i"));
        assert_eq!(fallback('\u{015F}'), Some("s"));
        assert_eq!(fallback('\u{017C}'), Some("z"));
        assert_eq!(fallback('\u{017F}'), Some("s"));
    }

    #[test]
    fn covers_tells_whether_anything_is_replaced() {
        assert!(covers("Contr\u{00F4}le moteur, 12,5\u{00A0}%"));
        assert!(!covers("\u{2265}"));
        assert!(!covers("\u{0142}"));
    }
}
