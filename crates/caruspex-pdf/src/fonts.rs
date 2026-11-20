//! The standard fonts the writer uses and their advance widths, so text
//! can be measured for wrapping and right alignment.

/// The four fonts. They are not embedded: every PDF viewer has them (PDF
/// 1.7, 9.6.2.2), Helvetica usually as Arial or Nimbus Sans.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Font {
    Helvetica,
    HelveticaBold,
    Courier,
    CourierBold,
}

impl Font {
    pub const ALL: [Font; 4] = [
        Font::Helvetica,
        Font::HelveticaBold,
        Font::Courier,
        Font::CourierBold,
    ];

    /// The /BaseFont name.
    pub fn base_name(self) -> &'static str {
        match self {
            Font::Helvetica => "Helvetica",
            Font::HelveticaBold => "Helvetica-Bold",
            Font::Courier => "Courier",
            Font::CourierBold => "Courier-Bold",
        }
    }

    /// The name in the page resources.
    pub fn resource(self) -> &'static str {
        match self {
            Font::Helvetica => "F1",
            Font::HelveticaBold => "F2",
            Font::Courier => "F3",
            Font::CourierBold => "F4",
        }
    }

    /// Advance width of a WinAnsi byte in 1/1000 of the font size.
    pub fn width(self, byte: u8) -> u16 {
        if byte < 32 {
            return 0;
        }
        let i = (byte - 32) as usize;
        match self {
            Font::Helvetica => HELVETICA[i],
            Font::HelveticaBold => HELVETICA_BOLD[i],
            // Courier is monospaced; the unused codes stay at 0 as above.
            Font::Courier | Font::CourierBold => {
                if HELVETICA[i] == 0 {
                    0
                } else {
                    600
                }
            }
        }
    }

    /// Width of encoded text at `size` points.
    pub fn bytes_width(self, bytes: &[u8], size: f32) -> f32 {
        let units: u32 = bytes.iter().map(|&b| u32::from(self.width(b))).sum();
        units as f32 * size / 1000.0
    }

    /// Width of `text` at `size` points, as it will be written (stand-ins
    /// included).
    pub fn text_width(self, text: &str, size: f32) -> f32 {
        self.bytes_width(&crate::encoding::encode(text).bytes, size)
    }
}

// Advance widths from Adobe's Core 14 AFM files (Helvetica.afm and
// Helvetica-Bold.afm, version 002.000), one per WinAnsi code from 32 to
// 255; 0 for the codes WinAnsi leaves unused.
#[rustfmt::skip]
const HELVETICA: [u16; 224] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556,
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556,
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556,
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584, 0,
    556, 0, 222, 556, 333, 1000, 556, 556, 333, 1000, 667, 333, 1000, 0, 611, 0,
    0, 222, 222, 333, 333, 350, 556, 1000, 333, 1000, 500, 333, 944, 0, 500, 500,
    278, 333, 556, 556, 556, 556, 260, 556, 333, 737, 370, 556, 584, 333, 737, 333,
    400, 584, 333, 333, 333, 556, 537, 278, 333, 333, 365, 556, 834, 834, 834, 611,
    667, 667, 667, 667, 667, 667, 1000, 722, 667, 667, 667, 667, 278, 278, 278, 278,
    722, 722, 778, 778, 778, 778, 778, 584, 778, 722, 722, 722, 722, 667, 667, 611,
    556, 556, 556, 556, 556, 556, 889, 500, 556, 556, 556, 556, 278, 278, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 584, 611, 556, 556, 556, 556, 500, 556, 500,
];

#[rustfmt::skip]
const HELVETICA_BOLD: [u16; 224] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611,
    975, 722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 333, 278, 333, 584, 556,
    333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611,
    611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584, 0,
    556, 0, 278, 556, 500, 1000, 556, 556, 333, 1000, 667, 333, 1000, 0, 611, 0,
    0, 278, 278, 500, 500, 350, 556, 1000, 333, 1000, 556, 333, 944, 0, 500, 556,
    278, 333, 556, 556, 556, 556, 280, 556, 333, 737, 370, 556, 584, 333, 737, 333,
    400, 584, 333, 333, 333, 611, 556, 278, 333, 333, 365, 556, 834, 834, 834, 611,
    722, 722, 722, 722, 722, 722, 1000, 722, 667, 667, 667, 667, 278, 278, 278, 278,
    722, 722, 778, 778, 778, 778, 778, 584, 778, 722, 722, 722, 722, 667, 667, 611,
    556, 556, 556, 556, 556, 556, 889, 556, 556, 556, 556, 556, 278, 278, 278, 278,
    611, 611, 611, 611, 611, 611, 611, 584, 611, 611, 611, 611, 611, 556, 611, 556,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_widths() {
        assert_eq!(Font::Helvetica.width(b' '), 278);
        assert_eq!(Font::Helvetica.width(b'W'), 944);
        assert_eq!(Font::Helvetica.width(b'i'), 222);
        assert_eq!(Font::HelveticaBold.width(b'i'), 278);
        assert_eq!(Font::Helvetica.width(0xFC), 556);
        assert_eq!(Font::Helvetica.width(0x80), 556);
        assert_eq!(Font::Courier.width(b'i'), 600);
        assert!((Font::Helvetica.text_width("Hello", 10.0) - 22.78).abs() < 0.01);
    }

    #[test]
    fn every_used_code_has_a_width() {
        for b in 32..=255u8 {
            let unused = matches!(b, 0x7F | 0x81 | 0x8D | 0x8F | 0x90 | 0x9D);
            for font in Font::ALL {
                assert_eq!(font.width(b) == 0, unused, "{font:?} {b:02X}");
            }
        }
    }
}
