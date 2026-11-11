//! The active language is a process wide setting, but tests run on several
//! threads. Every test that reads texts holds this guard.

use std::sync::{Mutex, MutexGuard};

use caruspex_i18n::{Language, language, set_language};

static LOCK: Mutex<()> = Mutex::new(());

pub struct LanguageGuard {
    _lock: MutexGuard<'static, ()>,
    before: Language,
}

impl Drop for LanguageGuard {
    fn drop(&mut self) {
        set_language(self.before);
    }
}

pub fn with(lang: Language) -> LanguageGuard {
    let lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let before = language();
    set_language(lang);
    LanguageGuard {
        _lock: lock,
        before,
    }
}

pub fn german() -> LanguageGuard {
    with(Language::German)
}

pub fn english() -> LanguageGuard {
    with(Language::English)
}
