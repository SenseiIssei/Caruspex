// Languages of the Qt-free core. German and English are written in the code
// next to each other (L(de, en), Text{de, en}); every other language comes
// from a catalog generated from translations/<code>/core_*.json, looked up
// by the English text. A text missing in a catalog shows in English.
#pragma once

#include <string>
#include <vector>

namespace caruspex {

enum class Language {
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
};

struct LanguageInfo {
    Language id;
    const char* code;          // ISO 639-1: "de", "en", "zh", ...
    const char* nativeName;    // as the language calls itself
    bool rightToLeft;
};

// All languages, in the order the settings list them.
const std::vector<LanguageInfo>& languages();
const LanguageInfo& languageInfo(Language language);
// By code ("fr", also "fr_FR" or "pt-BR"); English if the code is unknown.
Language languageFromCode(const std::string& code);

void setLanguage(Language language);
Language language();

// The catalog text of en in the given language, en itself if there is none.
const char* translateCore(Language language, const char* en);

// Picks the text for the active language.
inline const char* L(const char* de, const char* en) {
    const Language l = language();
    if (l == Language::German) return de;
    if (l == Language::English) return en;
    return translateCore(l, en);
}

// A text in both languages, for tables of static data.
struct Text {
    const char* de;
    const char* en;
    const char* get() const { return L(de, en); }
};

}  // namespace caruspex
