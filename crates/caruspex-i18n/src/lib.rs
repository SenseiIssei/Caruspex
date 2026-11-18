//! Languages of the core. German and English are written in the code next to
//! each other (`l(de, en)`, `Text { de, en }`); every other language comes
//! from the catalog built from `translations/<code>/core_*.json`, looked up by
//! the English text. A text missing in a catalog shows in English.

use std::cell::Cell;
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

thread_local! {
    /// The language `with_language` set for this thread only, so a test or
    /// a report in another language never changes what other threads see.
    static OVERRIDE: Cell<Option<Language>> = const { Cell::new(None) };
}

pub fn language() -> Language {
    if let Some(language) = OVERRIDE.with(Cell::get) {
        return language;
    }
    TABLE[CURRENT.load(Ordering::Relaxed) as usize].id
}

// The packed catalogs from build.rs: (code, deflate of "en\0text\0...").
include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));

type Catalog = HashMap<&'static str, &'static str>;

/// Unpacks one catalog. The texts stay in one block for the rest of the
/// run, the map only points into it.
fn unpack(packed: &[u8]) -> Catalog {
    let Ok(raw) = miniz_oxide::inflate::decompress_to_vec(packed) else {
        return Catalog::new();
    };
    let Ok(text) = String::from_utf8(raw) else {
        return Catalog::new();
    };
    let text: &'static str = Box::leak(text.into_boxed_str());
    let mut parts = text.split('\0');
    let mut map = Catalog::new();
    while let (Some(en), Some(translated)) = (parts.next(), parts.next()) {
        map.insert(en, translated);
    }
    map
}

/// The catalog of one language, unpacked the first time it is needed; a
/// program that only speaks German or English never unpacks any.
fn catalog(language: Language) -> Option<&'static Catalog> {
    static CATALOGS: [OnceLock<Catalog>; 16] = [const { OnceLock::new() }; 16];
    let code = language_info(language).code;
    let index = PACKED.iter().position(|(c, _)| *c == code)?;
    Some(CATALOGS[index].get_or_init(|| unpack(PACKED[index].1)))
}

/// Packed bytes of all catalogs together, for the size check in the tests.
pub fn packed_catalog_bytes() -> usize {
    PACKED.iter().map(|(_, bytes)| bytes.len()).sum()
}

/// The catalog text of `en` in the given language, `en` itself if there is none.
pub fn translate_core(language: Language, en: &'static str) -> &'static str {
    match language {
        Language::German | Language::English => en,
        other => catalog(other)
            .and_then(|m| m.get(en).copied())
            .unwrap_or(en),
    }
}

/// Same lookup for a text built at run time (owned strings).
pub fn translate_core_owned(language: Language, en: &str) -> String {
    match language {
        Language::German | Language::English => en.to_string(),
        other => catalog(other)
            .and_then(|m| m.get(en).copied())
            .unwrap_or(en)
            .to_string(),
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
/// For tests and for rendering a report in a chosen language. Only the
/// calling thread sees the other language; tests run in parallel and the
/// interface keeps its language meanwhile.
pub fn with_language<R>(language: Language, f: impl FnOnce() -> R) -> R {
    // Restores the previous state even if `f` panics.
    struct Restore(Option<Language>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OVERRIDE.with(|o| o.set(self.0));
        }
    }
    let _restore = Restore(OVERRIDE.with(|o| o.replace(Some(language))));
    f()
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
                catalog(info.id).map(|m| m.len()).unwrap_or(0) > 3000,
                "{}",
                info.code
            );
        }
        assert!(catalog(Language::German).is_none());
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

    #[test]
    fn catalogs_are_packed() {
        // About 6.9 MB of texts as JSON; packed they must stay far below.
        let packed = packed_catalog_bytes();
        assert!(packed > 100_000 && packed < 3_000_000, "{packed}");
        // A known text of the oldest files, in a language unpacked here.
        let es = catalog(Language::Spanish).unwrap();
        assert!(es.values().all(|t| !t.is_empty()));
        assert_ne!(
            translate_core(Language::Japanese, "unknown"),
            "unknown",
            "Japanese has the word"
        );
    }
}
