// Tests for the fault code database: decoding, generic and brand meanings,
// severity, plain-language help and both languages.

#include <catch2/catch_test_macros.hpp>

#include <set>
#include <string>

#include "core/Lang.h"
#include "data/DtcDatabase.h"

using namespace caruspex;

namespace {

struct EnglishScope {
    EnglishScope() { setLanguage(Language::English); }
    ~EnglishScope() { setLanguage(Language::German); }
};

bool contains(const std::string& text, const std::string& part) {
    return text.find(part) != std::string::npos;
}

// No long dashes anywhere in user-visible text, only the plain " - ".
bool hasLongDash(const std::string& text) {
    return contains(text, "\xE2\x80\x94") || contains(text, "\xE2\x80\x93");
}

}  // namespace

TEST_CASE("Trouble code bytes decode to the five-character form", "[dtc]") {
    REQUIRE(decodeDtc(0x01, 0x71) == "P0171");
    REQUIRE(decodeDtc(0x0A, 0x80) == "P0A80");
    REQUIRE(decodeDtc(0x11, 0x28) == "P1128");
    REQUIRE(decodeDtc(0x40, 0x35) == "C0035");
    REQUIRE(decodeDtc(0x80, 0x01) == "B0001");
    REQUIRE(decodeDtc(0xC1, 0x21) == "U0121");
    REQUIRE(decodeDtc(0x00, 0x00).empty());
}

TEST_CASE("Generic codes have meaning, area, severity and help", "[dtc]") {
    const DtcInfo misfire = lookupDtc("P0301");
    REQUIRE(misfire.known);
    REQUIRE_FALSE(misfire.manufacturerSpecific);
    REQUIRE(misfire.family.empty());
    REQUIRE(contains(misfire.text, "Zylinder 1"));
    REQUIRE(misfire.system == DtcSystem::Ignition);
    REQUIRE(misfire.hasDetails);
    REQUIRE(misfire.causes.size() >= 3);
    REQUIRE_FALSE(misfire.checks.empty());
    REQUIRE(contains(misfire.advice, "Katalysator"));
    REQUIRE_FALSE(misfire.systemName.empty());

    const DtcInfo cat = lookupDtc("P0420");
    REQUIRE(cat.system == DtcSystem::Exhaust);
    REQUIRE(cat.costClass == CostClass::Expensive);
    REQUIRE(std::string(costClassLabel(cat.costClass)) == "teuer");

    REQUIRE(lookupDtc("U0100").system == DtcSystem::Network);
    REQUIRE(lookupDtc("C0035").system == DtcSystem::Brakes);
    REQUIRE(lookupDtc("B0001").system == DtcSystem::Airbag);
    REQUIRE(lookupDtc("P2463").system == DtcSystem::DieselAftertreatment);
    REQUIRE(lookupDtc("P20EE").known);
    REQUIRE(lookupDtc("P0A80").system == DtcSystem::Hybrid);
    REQUIRE(lookupDtc("P3425").known);

    // Input is tolerant about case and spaces.
    REQUIRE(lookupDtc(" p0301 ").code == "P0301");
    REQUIRE(lookupDtc(" p0301 ").known);
}

TEST_CASE("Code families count in decimal like the standard", "[dtc]") {
    // Cylinder 10 is P0210 and P0310, not P020A or P030A.
    REQUIRE(contains(lookupDtc("P0210").text, "Zylinder 10"));
    REQUIRE(contains(lookupDtc("P0310").text, "Zylinder 10"));
    REQUIRE_FALSE(lookupDtc("P020A").known);
    REQUIRE_FALSE(lookupDtc("P030A").known);
    // Bank 1 sensor 2 runs from P0136 to P0141.
    REQUIRE(contains(lookupDtc("P0140").text, "Bank 1, Sonde 2"));
    // P0736 is reverse, P0775 a pressure control solenoid.
    REQUIRE(contains(lookupDtc("P0736").text, "Rückwärtsgang"));
    REQUIRE(contains(lookupDtc("P0775").text, "Druckregelventil B"));
    // ABS loses communication as U0121, its invalid data is U0415.
    REQUIRE(contains(lookupDtc("U0415").text, "ABS"));
    // Ignition coil L sits at the end of the P23xx block.
    REQUIRE(contains(lookupDtc("P2335").text, "Zündspule L"));
}

TEST_CASE("Severity follows the risk of driving on", "[dtc]") {
    // Whether the check engine light flashes is not known to us, so a
    // misfire is at least High.
    REQUIRE(lookupDtc("P0300").severity >= Severity::High);
    for (int cyl = 1; cyl <= 12; ++cyl) {
        const std::string code = "P03" + std::string(cyl < 10 ? "0" : "") + std::to_string(cyl);
        REQUIRE(lookupDtc(code).severity >= Severity::High);
    }
    REQUIRE(lookupDtc("P0420").severity == Severity::Medium);
    REQUIRE(lookupDtc("P0171").severity == Severity::Medium);
    REQUIRE(lookupDtc("P0442").severity == Severity::Low);
    REQUIRE(lookupDtc("P0456").severity == Severity::Low);
    REQUIRE(lookupDtc("P0128").severity == Severity::Low);
    REQUIRE(lookupDtc("P0217").severity == Severity::Critical);
    REQUIRE(lookupDtc("P0524").severity == Severity::Critical);
    REQUIRE(lookupDtc("P0AA6").severity == Severity::Critical);
    REQUIRE(lookupDtc("C0035").severity == Severity::High);
    REQUIRE(lookupDtc("B0001").severity == Severity::High);
    REQUIRE(lookupDtc("U0121").severity == Severity::High);
    REQUIRE(lookupDtc("U0184").severity == Severity::Low);
    REQUIRE(lookupDtc("P1000", "Ford").severity == Severity::Info);
}

TEST_CASE("Brand families are found from plain brand names", "[dtc]") {
    REQUIRE(brandFamily("Audi") == "VAG");
    REQUIRE(brandFamily("Škoda") == "VAG");
    REQUIRE(brandFamily("SKODA") == "VAG");
    REQUIRE(brandFamily("VW Nutzfahrzeuge") == "VAG");
    REQUIRE(brandFamily("Mercedes-Benz") == "MERCEDES");
    REQUIRE(brandFamily("mercedes benz") == "MERCEDES");
    REQUIRE(brandFamily("Lexus") == "TOYOTA");
    REQUIRE(brandFamily("Kia") == "HYUNDAI");
    REQUIRE(brandFamily("Citroën") == "PSA");
    REQUIRE(brandFamily("Mini Cooper") == "BMW");
    REQUIRE(brandFamily("Opel") == "OPEL");
    REQUIRE(brandFamily("PSA") == "PSA");
    REQUIRE(brandFamily("Tesla").empty());
    REQUIRE(brandFamily("").empty());
    REQUIRE(brandFamilyLabel("HYUNDAI") == "Hyundai/Kia");
    REQUIRE(brandFamilies().size() >= 15);
}

TEST_CASE("The same P1 code means different things per brand", "[dtc]") {
    const DtcInfo vag = lookupDtc("P1128", "Volkswagen");
    const DtcInfo honda = lookupDtc("P1128", "Honda");
    const DtcInfo toyota = lookupDtc("P1128", "Toyota");
    const DtcInfo generic = lookupDtc("P1128");

    REQUIRE(vag.known);
    REQUIRE(vag.family == "VAG");
    REQUIRE(vag.manufacturerSpecific);
    REQUIRE(contains(vag.text, "mager"));
    REQUIRE(honda.known);
    REQUIRE(honda.family == "HONDA");
    REQUIRE(contains(honda.text, "Saugrohrdruck"));
    REQUIRE(toyota.known);
    REQUIRE(contains(toyota.text, "Drosselklappe"));
    REQUIRE(toyota.throttleRelated);

    REQUIRE(vag.text != honda.text);
    REQUIRE(honda.text != toyota.text);
    // Without a brand the code is honestly unknown.
    REQUIRE_FALSE(generic.known);
    REQUIRE(generic.manufacturerSpecific);
    REQUIRE(generic.text != vag.text);

    REQUIRE(describeDtc("P1128", "Honda") == honda.text);
}

TEST_CASE("Brand tables never hide the generic meaning", "[dtc]") {
    const DtcInfo info = lookupDtc("P0301", "Ford");
    REQUIRE(info.known);
    REQUIRE(info.family.empty());
    REQUIRE(info.text == lookupDtc("P0301").text);
}

TEST_CASE("Mazda only takes over the Ford codes it really shares", "[dtc]") {
    REQUIRE(lookupDtc("P1000", "Mazda").known);
    REQUIRE(lookupDtc("P1131", "Ford").known);
    REQUIRE_FALSE(lookupDtc("P1131", "Mazda").known);
}

TEST_CASE("Unknown codes fall back to their group", "[dtc]") {
    const DtcInfo renault = lookupDtc("P1999", "Renault");
    REQUIRE_FALSE(renault.known);
    REQUIRE(renault.manufacturerSpecific);
    REQUIRE(contains(renault.text, "herstellerspezifisch, Renault/Dacia"));
    REQUIRE(contains(renault.text, "kein Klartext"));
    REQUIRE_FALSE(renault.advice.empty());

    const DtcInfo gearbox = lookupDtc("P0999");
    REQUIRE_FALSE(gearbox.known);
    REQUIRE(gearbox.system == DtcSystem::Transmission);
    REQUIRE(contains(gearbox.text, "Getriebe"));

    // An unknown airbag code still gets the airbag help.
    const DtcInfo airbag = lookupDtc("B0099");
    REQUIRE_FALSE(airbag.known);
    REQUIRE(airbag.system == DtcSystem::Airbag);
    REQUIRE(airbag.hasDetails);
    REQUIRE(airbag.severity == Severity::High);

    const DtcInfo lostComm = lookupDtc("U0177");
    REQUIRE(lostComm.system == DtcSystem::Network);
    REQUIRE(lostComm.hasDetails);

    REQUIRE(lookupDtc("XYZ").text == "unbekannter Code");
    REQUIRE_FALSE(lookupDtc("P4000").known);
    REQUIRE(describeDtc("12345") == "unbekannter Code");
}

TEST_CASE("The old lookups keep working", "[dtc]") {
    REQUIRE(dtcDescriptions().size() > 1000);
    // describeDtc without a brand still knows the VAG codes.
    REQUIRE(contains(describeDtc("P1559"), "Drosselklappe"));
    REQUIRE(contains(describeDtc("P0171"), "mager"));
    REQUIRE(isThrottleDtc("P0507"));
    REQUIRE(isThrottleDtc("P1559"));
    REQUIRE(isThrottleDtc("P2135"));
    REQUIRE_FALSE(isThrottleDtc("P0301"));
    REQUIRE(dtcTexts().count("P0300") == 1);
    REQUIRE(dtcThrottleCodes().count("P2101") == 1);
}

TEST_CASE("The lexicon lists generic and brand codes", "[dtc]") {
    const std::vector<DtcLexiconEntry> all = dtcLexicon();
    REQUIRE(all.size() == dtcCount() + [] {
        std::size_t brands = 0;
        for (const std::string& f : brandFamilies()) brands += dtcCount(f);
        return brands;
    }());
    REQUIRE(dtcCount() > 1300);

    bool genericFound = false;
    bool vagFound = false;
    bool hondaFound = false;
    for (const DtcLexiconEntry& e : all) {
        REQUIRE_FALSE(e.text.empty());
        REQUIRE_FALSE(e.familyLabel.empty());
        if (e.code == "P0301" && e.family.empty()) genericFound = true;
        if (e.code == "P1559" && e.family == "VAG") vagFound = true;
        if (e.code == "P1128" && e.family == "HONDA") hondaFound = true;
    }
    REQUIRE(genericFound);
    REQUIRE(vagFound);
    REQUIRE(hondaFound);
}

TEST_CASE("Common codes come with plain-language help", "[dtc]") {
    std::size_t withDetails = 0;
    for (const DtcLexiconEntry& e : dtcLexicon())
        if (e.family.empty() && lookupDtc(e.code).hasDetails) ++withDetails;
    REQUIRE(withDetails >= 250);

    for (const char* code : {"P0171", "P0420", "P0442", "P0101", "P0128", "P0016", "P0401", "P2463",
                             "P0380", "P20EE", "P0562", "U0100", "C0035", "B0001", "P0299"}) {
        const DtcInfo info = lookupDtc(code);
        INFO(code);
        REQUIRE(info.hasDetails);
        REQUIRE_FALSE(info.advice.empty());
        REQUIRE_FALSE(info.causes.empty());
        REQUIRE_FALSE(info.checks.empty());
    }
}

TEST_CASE("Fault texts follow the active language", "[dtc][lang]") {
    REQUIRE(contains(lookupDtc("P0217").text, "überhitzt"));
    {
        EnglishScope english;
        const DtcInfo cat = lookupDtc("P0420");
        REQUIRE(contains(cat.text, "Catalyst"));
        REQUIRE(contains(cat.advice, "emissions test"));
        REQUIRE(std::string(costClassLabel(cat.costClass)) == "expensive");
        REQUIRE(cat.systemName == "Exhaust: oxygen sensors and catalytic converter");
        REQUIRE(contains(lookupDtc("P1999", "Renault").text, "manufacturer-specific, Renault/Dacia"));
        REQUIRE(contains(describeDtc("P1128", "Honda"), "MAP sensor"));
        REQUIRE(std::string(severityLabel(lookupDtc("P0300").severity)) == "High");
        REQUIRE(brandFamilyLabel("") == "SAE/ISO (all brands)");
    }
    REQUIRE(contains(lookupDtc("P0420").text, "Katalysator"));
}

TEST_CASE("No long dashes and no empty texts in either language", "[dtc][lang]") {
    for (const Language lang : {Language::German, Language::English}) {
        setLanguage(lang);
        std::set<std::string> seen;
        for (const DtcLexiconEntry& e : dtcLexicon()) {
            INFO(e.code << " " << e.family);
            REQUIRE_FALSE(hasLongDash(e.text));
            if (!seen.insert(e.code + e.family).second) continue;
            const DtcInfo info = lookupDtc(e.code, e.family);
            REQUIRE_FALSE(info.advice.empty());
            REQUIRE_FALSE(hasLongDash(info.advice));
            for (const std::string& line : info.causes) REQUIRE_FALSE(hasLongDash(line));
            for (const std::string& line : info.checks) REQUIRE_FALSE(hasLongDash(line));
        }
    }
    setLanguage(Language::German);
}
