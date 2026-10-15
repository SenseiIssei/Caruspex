// Display language of the core.
//
// The core has no Qt and therefore no QTranslator. Every user-visible text
// that comes out of the core (fault descriptions, PID names, adapter and
// protocol messages) is written in both languages right where it is defined,
// and L() picks the active one. The GUI sets the language once at startup.
#pragma once

namespace caruspex {

enum class Language { German, English };

void setLanguage(Language language);
Language language();

// Picks the text for the active language.
inline const char* L(const char* de, const char* en) {
    return language() == Language::English ? en : de;
}

// A text in both languages, for tables of static data.
struct Text {
    const char* de;
    const char* en;
    const char* get() const { return L(de, en); }
};

}  // namespace caruspex
