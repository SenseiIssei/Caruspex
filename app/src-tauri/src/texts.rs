// Texts the backend writes itself. German and English stand in the code next
// to each other like the L(de, en) of the core; every other language comes
// from the interface catalog that build.rs makes from translations/, looked
// up by Qt context and German source the way tr() did.
use std::collections::HashMap;
use std::sync::OnceLock;

use caruspex_i18n::{Language, language, language_info, translate_core_owned};

type Catalog = HashMap<String, HashMap<String, String>>;

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/ui_catalog.json")))
            .unwrap_or_default()
    })
}

/// The text in the active language.
pub fn tr(context: &str, de: &str, en: &str) -> String {
    match language() {
        Language::German => de.to_string(),
        Language::English => en.to_string(),
        other => {
            let code = language_info(other).code;
            catalog()
                .get(code)
                .and_then(|m| m.get(&format!("{context}|{de}")))
                .cloned()
                .unwrap_or_else(|| translate_core_owned(other, en))
        }
    }
}

/// A text of the worker (Qt context caruspex::Worker).
pub fn w(de: &str, en: &str) -> String {
    tr("caruspex::Worker", de, en)
}

/// A text of the main window (Qt context caruspex::MainWindow).
pub fn mw(de: &str, en: &str) -> String {
    tr("caruspex::MainWindow", de, en)
}

/// A text of the command line (Qt context main).
pub fn main_text(de: &str, en: &str) -> String {
    tr("main", de, en)
}

/// Fills %1 to %9 like QString::arg, in one pass, so an inserted text that
/// itself contains "%2" is left alone.
pub fn args(text: &str, values: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%'
            && let Some(&d) = chars.peek()
            && let Some(n) = d.to_digit(10)
            && n >= 1
            && (n as usize) <= values.len()
        {
            chars.next();
            out.push_str(&values[n as usize - 1].to_string());
            continue;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use caruspex_i18n::with_language;

    #[test]
    fn placeholders_in_one_pass() {
        assert_eq!(args("%1 und %2", &[&"a%2", &7]), "a%2 und 7");
        assert_eq!(args("100%", &[&1]), "100%");
        assert_eq!(args("%3", &[&1]), "%3");
    }

    #[test]
    fn languages() {
        let de = "Fehlerspeicher gelöscht.";
        let en = "Fault memory cleared.";
        assert_eq!(with_language(Language::German, || w(de, en)), de);
        assert_eq!(with_language(Language::English, || w(de, en)), en);
        let fr = with_language(Language::French, || w(de, en));
        assert!(!fr.is_empty());
        assert_ne!(fr, de);
        // Unknown texts fall back to English.
        assert_eq!(with_language(Language::French, || w("xyz", "abc")), "abc");
    }
}
