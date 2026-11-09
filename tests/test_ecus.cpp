// Tests for the ECU address lists and the generic sweep.

#include <catch2/catch_test_macros.hpp>

#include <algorithm>
#include <cstring>
#include <set>
#include <string>
#include <utility>

#include "core/Lang.h"
#include "data/DtcDatabase.h"
#include "data/EcuAddresses.h"

using namespace caruspex;

namespace {

struct LanguageScope {
    explicit LanguageScope(Language l) { setLanguage(l); }
    ~LanguageScope() { setLanguage(Language::German); }
};

bool hasLongDash(const std::string& text) {
    return text.find("\xE2\x80\x94") != std::string::npos || text.find("\xE2\x80\x93") != std::string::npos;
}

bool idFits(std::uint32_t id, bool extended29) { return id <= (extended29 ? 0x1FFFFFFFu : 0x7FFu); }

const EcuKind kAllKinds[] = {
    EcuKind::Engine, EcuKind::Gearbox, EcuKind::Brakes, EcuKind::Airbag, EcuKind::Steering,
    EcuKind::Instruments, EcuKind::Gateway, EcuKind::Body, EcuKind::Climate, EcuKind::Parking,
    EcuKind::Lighting, EcuKind::Doors, EcuKind::Seats, EcuKind::Infotainment, EcuKind::Battery,
    EcuKind::Charger, EcuKind::Hybrid, EcuKind::Other,
};

}  // namespace

TEST_CASE("Every ECU family is a brand family of the DTC database", "[ecus]") {
    const auto families = ecuFamilies();
    REQUIRE(families.size() >= 5);
    const auto& known = brandFamilies();
    std::set<std::string> seen;
    for (const auto& f : families) {
        INFO(f);
        REQUIRE(std::find(known.begin(), known.end(), f) != known.end());
        REQUIRE(seen.insert(f).second);
    }
    // The families the plan names first.
    for (const char* f : {"VAG", "FORD", "TOYOTA", "HYUNDAI", "PSA"})
        REQUIRE(std::find(families.begin(), families.end(), f) != families.end());
}

TEST_CASE("Every family list is complete and consistent", "[ecus]") {
    for (const auto& family : ecuFamilies()) {
        INFO(family);
        const auto list = ecuDefinitions(family);
        REQUIRE_FALSE(list.empty());
        REQUIRE(list.front().kind == EcuKind::Engine);

        std::set<std::pair<bool, std::uint32_t>> requests, responses;
        for (std::size_t i = 0; i < list.size(); ++i) {
            const EcuDefinition& e = list[i];
            INFO(e.name);
            REQUIRE(e.family == family);
            REQUIRE_FALSE(e.name.empty());
            REQUIRE_FALSE(hasLongDash(e.name));
            REQUIRE(e.request != 0);
            REQUIRE(e.response != 0);
            REQUIRE(e.request != e.response);
            REQUIRE(idFits(e.request, e.extended29));
            REQUIRE(idFits(e.response, e.extended29));
            if (!e.extended29) REQUIRE(e.request != 0x7DF);
            REQUIRE(requests.insert({e.extended29, e.request}).second);
            REQUIRE(responses.insert({e.extended29, e.response}).second);
            // Scan order: engine first, safety relevant units next.
            if (i > 0) REQUIRE(static_cast<int>(list[i - 1].kind) <= static_cast<int>(e.kind));
        }
    }
}

TEST_CASE("Unknown families have no list", "[ecus]") {
    REQUIRE(ecuDefinitions("").empty());
    REQUIRE(ecuDefinitions("NOPE").empty());
    REQUIRE(ecuDefinitions("vag").empty());  // keys are exact
}

TEST_CASE("Well known addresses are in place", "[ecus]") {
    auto find = [](const std::string& family, std::uint32_t request) {
        for (const auto& e : ecuDefinitions(family))
            if (e.request == request) return e;
        return EcuDefinition{};
    };
    const EcuDefinition vagAbs = find("VAG", 0x713);
    REQUIRE(vagAbs.response == 0x77D);
    REQUIRE(vagAbs.kind == EcuKind::Brakes);
    REQUIRE(find("VAG", 0x710).kind == EcuKind::Gateway);
    REQUIRE(find("FORD", 0x760).response == 0x768);
    REQUIRE(find("FORD", 0x737).kind == EcuKind::Airbag);
    REQUIRE(find("GM", 0x7E2).kind == EcuKind::Gearbox);
    REQUIRE(find("PSA", 0x752).response == 0x652);
}

TEST_CASE("Names follow the active language", "[ecus]") {
    {
        LanguageScope en(Language::English);
        const auto list = ecuDefinitions("VAG");
        REQUIRE(list.front().name == "Engine control");
    }
    LanguageScope de(Language::German);
    const auto list = ecuDefinitions("VAG");
    REQUIRE(list.front().name == "Motorsteuerung");
}

TEST_CASE("Every kind has a name in both languages", "[ecus]") {
    for (Language lang : {Language::German, Language::English}) {
        LanguageScope scope(lang);
        std::set<std::string> names;
        for (EcuKind k : kAllKinds) {
            const char* name = ecuKindName(k);
            REQUIRE(name != nullptr);
            REQUIRE(std::strlen(name) > 0);
            REQUIRE_FALSE(hasLongDash(name));
            REQUIRE(names.insert(name).second);
        }
    }
    LanguageScope de(Language::German);
    REQUIRE(std::string(ecuKindName(EcuKind::Brakes)) == "Bremsen und ABS");
    LanguageScope en(Language::English);
    REQUIRE(std::string(ecuKindName(EcuKind::Brakes)) == "Brakes and ABS");
}

TEST_CASE("The generic sweep covers the usual 11-bit range", "[ecus]") {
    const auto sweep = genericEcuSweep();
    // 7E0 to 7E7, then 0x700 to 0x7F7 without 7E0 to 7EF and without 7DF.
    REQUIRE(sweep.size() == 8 + (0xF8 - 16 - 1));

    for (std::size_t i = 0; i < 8; ++i) REQUIRE(sweep[i].request == 0x7E0 + i);
    REQUIRE(sweep[0].kind == EcuKind::Engine);
    REQUIRE(sweep[1].kind == EcuKind::Gearbox);
    REQUIRE(sweep[2].kind == EcuKind::Other);

    std::set<std::uint32_t> requests;
    for (const auto& e : sweep) {
        INFO(e.request);
        REQUIRE(e.response == e.request + 8);
        REQUIRE_FALSE(e.extended29);
        REQUIRE(e.family.empty());
        REQUIRE(e.request >= 0x700);
        REQUIRE(e.request <= 0x7F7);
        REQUIRE(e.request != 0x7DF);
        REQUIRE_FALSE(e.name.empty());
        REQUIRE(requests.insert(e.request).second);
        if (e.request >= 0x7E8 && e.request <= 0x7EF) FAIL("OBD response id used as request");
        if (e.request > 0x7E7) REQUIRE(e.kind == EcuKind::Other);
    }
    REQUIRE(requests.count(0x700) == 1);
    REQUIRE(requests.count(0x7F7) == 1);
}

TEST_CASE("Sweep names carry the address", "[ecus]") {
    const auto find = [](std::uint32_t request) {
        for (const auto& e : genericEcuSweep())
            if (e.request == request) return e.name;
        return std::string();
    };
    {
        LanguageScope de(Language::German);
        REQUIRE(find(0x760) == "Steuergerät 0x760");
        REQUIRE(find(0x7E0) == "Motor (0x7E0)");
    }
    LanguageScope en(Language::English);
    REQUIRE(find(0x760) == "Control unit 0x760");
    REQUIRE(find(0x7E1) == "Gearbox (0x7E1)");
}
