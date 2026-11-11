//! Small text helpers: placeholders and numbers in the active language.

use caruspex_analysis::common::format_number;
use caruspex_i18n::{Language, language};

/// Replaces %1, %2, ... in a translated template, like QString::arg().
pub fn args(template: &str, values: &[&str]) -> String {
    let mut out = template.to_string();
    // From the highest number down, so %1 never eats the start of %10.
    for (i, value) in values.iter().enumerate().rev() {
        out = out.replace(&format!("%{}", i + 1), value);
    }
    out
}

/// A measured number with the decimal comma in German, a point otherwise.
pub fn number(value: f64, decimals: i32) -> String {
    format_number(value, decimals, language() == Language::German)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_filled_in_any_order() {
        assert_eq!(args("%2 und %1", &["a", "b"]), "b und a");
        assert_eq!(args("ohne", &["a"]), "ohne");
    }
}
