//! Languages of the core. German and English are written in the code next to
//! each other (`l(de, en)`, `Text { de, en }`); every other language comes
//! from the catalog built from `translations/<code>/core_*.json`, looked up by
//! the English text. A text missing in a catalog shows in English.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Language {
    German,
    English,
    Spanish,
    French,
    Portuguese,
    Italian,
    Dutch,
    Polish,
    Czech,
    Russian,
    Ukrainian,
    Turkish,
    Arabic,
    Chinese,
    Japanese,
    Korean,
    Hindi,
    Indonesian,
}

#[derive(Clone, Copy, Debug)]
pub struct LanguageInfo {
    pub id: Language,
    /// ISO 639-1: "de", "en", "zh", ...
    pub code: &'static str,
    /// As the language calls itself.
    pub native_name: &'static str,
    pub right_to_left: bool,
}

const TABLE: [LanguageInfo; 18] = [
    LanguageInfo {
        id: Language::German,
        code: "de",
        native_name: "Deutsch",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::English,
        code: "en",
        native_name: "English",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Spanish,
        code: "es",
        native_name: "Español",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::French,
        code: "fr",
        native_name: "Français",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Portuguese,
        code: "pt",
        native_name: "Português",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Italian,
        code: "it",
        native_name: "Italiano",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Dutch,
        code: "nl",
        native_name: "Nederlands",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Polish,
        code: "pl",
        native_name: "Polski",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Czech,
        code: "cs",
        native_name: "Čeština",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Russian,
        code: "ru",
        native_name: "Русский",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Ukrainian,
        code: "uk",
        native_name: "Українська",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Turkish,
        code: "tr",
        native_name: "Türkçe",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Arabic,
        code: "ar",
        native_name: "العربية",
        right_to_left: true,
    },
    LanguageInfo {
        id: Language::Chinese,
        code: "zh",
        native_name: "中文",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Japanese,
        code: "ja",
        native_name: "日本語",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Korean,
        code: "ko",
        native_name: "한국어",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Hindi,
        code: "hi",
        native_name: "हिन्दी",
        right_to_left: false,
    },
    LanguageInfo {
        id: Language::Indonesian,
        code: "id",
        native_name: "Bahasa Indonesia",
        right_to_left: false,
    },
];

/// All languages, in the order the settings list them.
pub fn languages() -> &'static [LanguageInfo] {
    &TABLE
}

pub fn language_info(language: Language) -> &'static LanguageInfo {
    &TABLE[language as usize]
}

/// By code ("fr", also "fr_FR" or "pt-BR"); English if the code is unknown.
pub fn language_from_code(code: &str) -> Language {
    let short: String = code
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_lowercase();
    TABLE
        .iter()
        .find(|i| i.code == short)
        .map(|i| i.id)
        .unwrap_or(Language::English)
}

// German stays the default so the CLI and the tests behave as before unless
// someone asks for another language.
static CURRENT: AtomicU8 = AtomicU8::new(Language::German as u8);

pub fn set_language(language: Language) {
    CURRENT.store(language as u8, Ordering::Relaxed);
}

pub fn language() -> Language {
    TABLE[CURRENT.load(Ordering::Relaxed) as usize].id
}

type Catalog = HashMap<String, HashMap<String, String>>;

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/core_catalog.json")))
            .expect("generated catalog is valid")
    })
}

/// The catalog text of `en` in the given language, `en` itself if there is none.
pub fn translate_core(language: Language, en: &'static str) -> &'static str {
    match language {
        Language::German | Language::English => en,
        other => catalog()
            .get(language_info(other).code)
            .and_then(|m| m.get(en))
            .map(|s| s.as_str())
            .unwrap_or(en),
    }
}

/// Same lookup for a text built at run time (owned strings).
pub fn translate_core_owned(language: Language, en: &str) -> String {
    match language {
        Language::German | Language::English => en.to_string(),
        other => catalog()
            .get(language_info(other).code)
            .and_then(|m| m.get(en))
            .cloned()
            .unwrap_or_else(|| en.to_string()),
    }
}

/// Picks the text for the active language.
pub fn l(de: &'static str, en: &'static str) -> &'static str {
    match language() {
        Language::German => de,
        Language::English => en,
        other => translate_core(other, en),
    }
}

/// A text in both languages, for tables of static data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Text {
    pub de: &'static str,
    pub en: &'static str,
}

impl Text {
    pub const fn new(de: &'static str, en: &'static str) -> Self {
        Self { de, en }
    }
    pub fn get(&self) -> &'static str {
        l(self.de, self.en)
    }
    pub fn is_empty(&self) -> bool {
        self.de.is_empty() && self.en.is_empty()
    }
}

/// Runs `f` with another active language and switches back afterwards.
/// For tests and for rendering a report in a chosen language.
pub fn with_language<R>(language: Language, f: impl FnOnce() -> R) -> R {
    let before = self::language();
    set_language(language);
    let result = f();
    set_language(before);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_fallback() {
        assert_eq!(language_from_code("pt-BR"), Language::Portuguese);
        assert_eq!(language_from_code("fr_FR"), Language::French);
        assert_eq!(language_from_code("xx"), Language::English);
        assert_eq!(languages().len(), 18);
        assert!(language_info(Language::Arabic).right_to_left);
    }

    #[test]
    fn catalog_has_every_language() {
        for info in languages().iter().skip(2) {
            assert!(
                catalog().get(info.code).map(|m| m.len()).unwrap_or(0) > 3000,
                "{}",
                info.code
            );
        }
        let text = Text::new("Zündkerzen", "Spark plugs");
        assert_eq!(with_language(Language::German, || text.get()), "Zündkerzen");
        assert_eq!(
            with_language(Language::English, || text.get()),
            "Spark plugs"
        );
        assert_eq!(
            translate_core(Language::Spanish, "no such text 123"),
            "no such text 123"
        );
    }
}
