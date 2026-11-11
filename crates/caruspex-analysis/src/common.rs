//! Shared building blocks of the evaluation modules (health check, trip
//! computer, performance timer, battery test).
//!
//! Everything in this crate is pure computation: no I/O, no clock. The caller
//! collects the data (from the adapter, from a recording or from a test) and
//! hands it in; the modules only judge it. That keeps every rule testable with
//! concrete numbers.
//!
//! Texts that contain measured numbers cannot live in static tables, so they
//! are built at run time as a [`Phrase`]: both languages side by side, and the
//! active one is picked when it is shown. That way a result can be computed
//! once and still follow a later language switch.

use std::ops::Add;

use caruspex_i18n::{Language, language, translate_core_owned};

/// How serious a single finding is. The order matters: higher is worse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Level {
    /// Checked and fine.
    #[default]
    Good,
    /// Worth knowing, nothing to do right now.
    Info,
    /// Should be looked at soon.
    Warning,
    /// Needs attention before driving much further.
    Critical,
}

/// The overall result as a traffic light.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum TrafficLight {
    #[default]
    Green,
    Yellow,
    Red,
}

/// Kind of fuel. Decides formulas (air/fuel ratio, density, CO2) and some
/// rules (readiness for diesel engines).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FuelType {
    #[default]
    Unknown,
    Petrol,
    Diesel,
}

/// A text in both languages, built at run time.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Phrase {
    pub de: String,
    pub en: String,
}

impl Phrase {
    pub fn new(de: impl Into<String>, en: impl Into<String>) -> Self {
        Self {
            de: de.into(),
            en: en.into(),
        }
    }

    /// The text in the active language.
    pub fn text(&self) -> String {
        match language() {
            Language::German => self.de.clone(),
            Language::English => self.en.clone(),
            other => translate_core_owned(other, &self.en),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.de.is_empty() && self.en.is_empty()
    }
}

/// Joins two phrases with a space in each language.
impl Add for &Phrase {
    type Output = Phrase;
    fn add(self, b: &Phrase) -> Phrase {
        fn join(x: &str, y: &str) -> String {
            if x.is_empty() {
                y.to_string()
            } else if y.is_empty() {
                x.to_string()
            } else {
                format!("{x} {y}")
            }
        }
        Phrase {
            de: join(&self.de, &b.de),
            en: join(&self.en, &b.en),
        }
    }
}

impl Add for Phrase {
    type Output = Phrase;
    fn add(self, b: Phrase) -> Phrase {
        &self + &b
    }
}

/// Formats a number with the decimal separator of the language: comma in
/// German, point in English. Negative zero is printed as 0.
pub fn format_number(value: f64, decimals: i32, german: bool) -> String {
    let decimals = decimals.clamp(0, 6) as usize;
    let mut out = format!("{value:.decimals$}");
    // "-0" or "-0.0" looks odd next to a measured value.
    let all_zero = out.chars().all(|c| c == '-' || c == '0' || c == '.');
    if all_zero && out.starts_with('-') {
        out.remove(0);
    }
    if german {
        out = out.replace('.', ",");
    }
    out
}

/// Formats a number for both languages at once, e.g. `("12,4", "12.4")`.
pub fn number(value: f64, decimals: i32) -> Phrase {
    Phrase {
        de: format_number(value, decimals, true),
        en: format_number(value, decimals, false),
    }
}

/// Same, with a leading plus sign for positive values (for fuel trims).
pub fn signed_number(value: f64, decimals: i32) -> Phrase {
    let mut p = number(value, decimals);
    // Only add the plus if the rounded value is really above zero.
    let scale = 10f64.powi(decimals);
    if (value * scale).round() > 0.0 {
        p.de.insert(0, '+');
        p.en.insert(0, '+');
    }
    p
}

/// Builds a phrase from a German and an English pattern. Each `{}` is replaced
/// by the next argument in the matching language, e.g.
/// `format("{} V gemessen", "{} V measured", &[number(12.4, 1)])`.
pub fn format(de: &str, en: &str, args: &[Phrase]) -> Phrase {
    fn fill(pattern: &str, args: &[Phrase], german: bool) -> String {
        let mut out = String::new();
        let mut arg = args.iter();
        let mut chars = pattern.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' && chars.peek() == Some(&'}') {
                chars.next();
                if let Some(a) = arg.next() {
                    out.push_str(if german { &a.de } else { &a.en });
                }
            } else {
                out.push(c);
            }
        }
        out
    }
    Phrase {
        de: fill(de, args, true),
        en: fill(en, args, false),
    }
}

/// A phrase that reads the same in both languages (codes, units, names that
/// were already resolved in one language).
pub fn same(s: &str) -> Phrase {
    Phrase {
        de: s.to_string(),
        en: s.to_string(),
    }
}

/// Worst of two levels.
pub fn worse(a: Level, b: Level) -> Level {
    a.max(b)
}

/// Short name of a level, for logs and tests ("good", "info", ...).
pub fn level_key(level: Level) -> &'static str {
    match level {
        Level::Good => "good",
        Level::Info => "info",
        Level::Warning => "warning",
        Level::Critical => "critical",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use caruspex_i18n::with_language;

    #[test]
    fn phrases_format_numbers_per_language() {
        assert_eq!(number(12.4, 1).de, "12,4");
        assert_eq!(number(12.4, 1).en, "12.4");
        assert_eq!(signed_number(14.0, 0).de, "+14");
        assert_eq!(signed_number(-3.26, 1).en, "-3.3");
        assert_eq!(number(-0.01, 1).en, "0.0");
        let p = format(
            "{} V bei {}/min",
            "{} V at {} rpm",
            &[number(13.85, 2), number(800.0, 0)],
        );
        assert_eq!(p.de, "13,85 V bei 800/min");
        assert_eq!(p.en, "13.85 V at 800 rpm");

        assert_eq!(
            with_language(Language::English, || p.text()),
            "13.85 V at 800 rpm"
        );
        assert_eq!(
            with_language(Language::German, || p.text()),
            "13,85 V bei 800/min"
        );
    }
}
