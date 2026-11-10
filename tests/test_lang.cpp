// Languages of the core: the list, codes, and that the catalogs really
// deliver translations (and English where an entry is missing).

#include <catch2/catch_test_macros.hpp>

#include <set>
#include <string>

#include "core/Lang.h"
#include "data/DtcDatabase.h"

using namespace caruspex;

TEST_CASE("Languages have codes, names and a direction", "[lang]") {
    const auto& all = languages();
    REQUIRE(all.size() == 18);
    std::set<std::string> codes;
    for (const LanguageInfo& info : all) {
        REQUIRE(codes.insert(info.code).second);
        REQUIRE(std::string(info.nativeName).size() > 1);
        REQUIRE(languageFromCode(info.code) == info.id);
    }
    REQUIRE(languageInfo(Language::Arabic).rightToLeft);
    REQUIRE_FALSE(languageInfo(Language::French).rightToLeft);
    REQUIRE(languageFromCode("pt_BR") == Language::Portuguese);
    REQUIRE(languageFromCode("zh-Hans") == Language::Chinese);
    REQUIRE(languageFromCode("xx") == Language::English);
}

TEST_CASE("Every catalog translates the core texts", "[lang]") {
    setLanguage(Language::English);
    const std::string english = lookupDtc("P0171").text;
    REQUIRE_FALSE(english.empty());
    for (const LanguageInfo& info : languages()) {
        if (info.id == Language::English) continue;
        INFO(info.code);
        setLanguage(info.id);
        const std::string text = lookupDtc("P0171").text;
        REQUIRE_FALSE(text.empty());
        REQUIRE(text != english);
        REQUIRE(std::string(L("Motor", "Engine")) != "Engine");
    }
    // Unknown texts fall back to English, never to nothing.
    REQUIRE(std::string(translateCore(Language::French, "no such text here")) == "no such text here");
    setLanguage(Language::German);
}
