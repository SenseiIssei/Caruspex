// Tests for the VIN decoder and the vehicle catalog.

#include <catch2/catch_test_macros.hpp>

#include <algorithm>
#include <cstring>
#include <initializer_list>
#include <set>
#include <string>

#include "core/Lang.h"
#include "data/VehicleCatalog.h"
#include "data/Vin.h"

using namespace caruspex;

namespace {

// Writes the correct check digit into position 9, for building North
// American test VINs.
std::string withCheckDigit(std::string vin) {
    vin[8] = '0';
    vin[8] = vinCheckDigit(vin);
    return vin;
}

bool hasLongDash(const char* text) { return text && std::strstr(text, "\xE2\x80\x94") != nullptr; }

}  // namespace

TEST_CASE("The classic VIN passes the check digit", "[vin]") {
    REQUIRE(vinCheckDigit("1M8GDM9AXKP042788") == 'X');

    const VinInfo v = decodeVin("1M8GDM9AXKP042788", 2026);
    REQUIRE(v.valid);
    REQUIRE(v.problems.empty());
    REQUIRE(v.wmi == "1M8");
    REQUIRE(v.manufacturer == "Motor Coach Industries");
    REQUIRE(v.northAmerica);
    REQUIRE(v.checkDigitApplies);
    REQUIRE(v.checkDigitOk);
    REQUIRE(v.expectedCheckDigit == 'X');
    // Digit at position 7: first cycle, K = 1989.
    REQUIRE(v.modelYear == 1989);
    REQUIRE(v.modelYears.size() == 1);
    REQUIRE(v.modelYearCertain);
    REQUIRE(v.plant == 'P');
    REQUIRE(v.serial == "042788");
}

TEST_CASE("A wrong check digit is reported", "[vin]") {
    const VinInfo v = decodeVin("1M8GDM9A1KP042788", 2026);
    REQUIRE(v.checkDigitApplies);
    REQUIRE_FALSE(v.checkDigitOk);
    REQUIRE_FALSE(v.problems.empty());
}

TEST_CASE("Input is normalised and checked", "[vin]") {
    const VinInfo spaced = decodeVin(" wvw zzz-1jz2w123456 ", 2026);
    REQUIRE(spaced.vin == "WVWZZZ1JZ2W123456");
    REQUIRE(spaced.valid);

    const VinInfo ioq = decodeVin("WVWZZZ1JZ2W12345O", 2026);
    REQUIRE_FALSE(ioq.valid);
    REQUIRE_FALSE(ioq.problems.empty());

    const VinInfo shortVin = decodeVin("WVWZZZ1JZ2W", 2026);
    REQUIRE_FALSE(shortVin.valid);
    REQUIRE_FALSE(shortVin.problems.empty());
    // The maker is still known from the first three characters.
    REQUIRE(shortVin.brand == "Volkswagen");

    REQUIRE_FALSE(decodeVin("", 2026).valid);
}

TEST_CASE("North American model years follow position 7", "[vin]") {
    // Letter at position 7: second cycle, A = 2010.
    const VinInfo second = decodeVin(withCheckDigit("1G1ZB5ST0AF000001"), 2026);
    REQUIRE(second.checkDigitOk);
    REQUIRE(second.modelYear == 2010);
    REQUIRE(second.modelYearCertain);
    REQUIRE(second.brand == "Chevrolet");

    // Digit at position 7: first cycle, A = 1980.
    const VinInfo first = decodeVin(withCheckDigit("1G1ZB5200AF000001"), 2026);
    REQUIRE(first.modelYear == 1980);

    // Y is 2000 in the first cycle and 2030 in the second.
    REQUIRE(vinBaseYear('Y') == 2000);
    REQUIRE(vinBaseYear('1') == 2001);
    REQUIRE(vinBaseYear('9') == 2009);
    REQUIRE(vinBaseYear('0') == 0);
    REQUIRE(vinBaseYear('U') == 0);
}

TEST_CASE("European model years keep the alternative", "[vin]") {
    // Golf IV: 2 is 2002, 2032 would be in the future.
    const VinInfo golf4 = decodeVin("WVWZZZ1JZ2W123456", 2026);
    REQUIRE(golf4.modelYears == std::vector<int>{2002});
    REQUIRE(golf4.modelYearCertain);
    REQUIRE_FALSE(golf4.checkDigitApplies);
    REQUIRE(golf4.country == "Deutschland");

    // M is 1991 or 2021, the newer one first.
    const VinInfo golf8 = decodeVin("WVWZZZCDZMW123456", 2026);
    REQUIRE(golf8.modelYears == std::vector<int>{2021, 1991});
    REQUIRE_FALSE(golf8.modelYearCertain);

    // Position 10 = 0 carries no year.
    const VinInfo noYear = decodeVin("WBAAA11000C123456", 2026);
    REQUIRE(noYear.modelYears.empty());
    REQUIRE(noYear.modelYear == 0);
}

TEST_CASE("WMI lookups cover old and new makers", "[vin]") {
    auto brandOf = [](const char* wmi) {
        const WmiEntry* e = findWmi(wmi);
        return e ? std::string(e->brands) : std::string();
    };
    REQUIRE(brandOf("WAU") == "Audi");
    REQUIRE(brandOf("TMB") == "Škoda");
    REQUIRE(brandOf("WBA") == "BMW");
    REQUIRE(brandOf("YS3") == "Saab");
    REQUIRE(brandOf("SAR") == "Rover|MG");
    REQUIRE(brandOf("1G2") == "Pontiac");
    REQUIRE(brandOf("1G3") == "Oldsmobile");
    REQUIRE(brandOf("1G8") == "Saturn");
    REQUIRE(brandOf("1ME") == "Mercury");
    REQUIRE(brandOf("1P3") == "Plymouth");
    REQUIRE(brandOf("5GR") == "Hummer");
    REQUIRE(brandOf("ZLA") == "Lancia");
    REQUIRE(brandOf("XTA") == "Lada");
    REQUIRE(brandOf("KLA") == "Daewoo");
    REQUIRE(brandOf("5YJ") == "Tesla");
    REQUIRE(brandOf("XP7") == "Tesla");
    REQUIRE(brandOf("LPS") == "Polestar");
    REQUIRE(brandOf("LGX") == "BYD");
    REQUIRE(brandOf("HJN") == "NIO");
    REQUIRE(brandOf("KMT") == "Genesis");
    REQUIRE(brandOf("7PD") == "Rivian");
    REQUIRE(brandOf("50E") == "Lucid");
    REQUIRE(brandOf("RLL") == "VinFast");
    REQUIRE(brandOf("LSJ") == "MG");
    REQUIRE(brandOf("VSS") == "SEAT|Cupra");
    // Two character entries catch the rest of a range.
    REQUIRE(brandOf("JTD") == "Toyota");
    REQUIRE(brandOf("JTH") == "Lexus");
    REQUIRE(brandOf("JN1") == "Nissan");
    REQUIRE(findWmi("QQQ") == nullptr);

    std::size_t count = 0;
    wmiTable(count);
    REQUIRE(count > 250);
}

TEST_CASE("Countries come from the first two characters", "[vin]") {
    setLanguage(Language::English);
    REQUIRE(vinCountry("WVW") == "Germany");
    REQUIRE(vinCountry("VF1") == "France");
    REQUIRE(vinCountry("ZFA") == "Italy");
    REQUIRE(vinCountry("SAJ") == "United Kingdom");
    REQUIRE(vinCountry("JTD") == "Japan");
    REQUIRE(vinCountry("KMH") == "South Korea");
    REQUIRE(vinCountry("LSJ") == "China");
    REQUIRE(vinCountry("TMB") == "Czech Republic");
    REQUIRE(vinCountry("YV1") == "Sweden");
    REQUIRE(vinCountry("3VW") == "Mexico");
    REQUIRE(vinCountry("9BW") == "Brazil");
    REQUIRE(vinRegion("1G1") == "North America");
    REQUIRE(vinRegion("7SA") == "North America");
    REQUIRE(vinRegion("7AT") == "Oceania");
    REQUIRE(vinRegion("WVW") == "Europe");
    // A plant country that differs from the code range.
    REQUIRE(decodeVin("XP7YGCEK0PB000001", 2026).country == "Germany");
    setLanguage(Language::German);
    REQUIRE(vinCountry("WVW") == "Deutschland");
}

TEST_CASE("VAG model codes find the generation", "[vin][catalog]") {
    const VinInfo a2 = decodeVin("WAUZZZ8ZZ1N012345", 2026);
    REQUIRE(a2.brand == "Audi");
    REQUIRE(a2.modelCode == "8Z");
    REQUIRE(a2.modelCodeRule == VinModelCodeRule::Vag78);
    const CatalogMatch m = catalog::matchVin(a2);
    REQUIRE(m.generation != nullptr);
    REQUIRE(std::string(m.generation->model) == "A2");
    REQUIRE(m.year == 2001);
    REQUIRE(m.confidence >= 85);

    const CatalogMatch golf4 = catalog::matchVin(decodeVin("WVWZZZ1JZ2W123456", 2026));
    REQUIRE(golf4.generation != nullptr);
    REQUIRE(std::string(golf4.generation->model) == "Golf");
    REQUIRE(std::string(golf4.generation->generation) == "IV");
    // The Bora shares the code 1J.
    REQUIRE_FALSE(golf4.alternatives.empty());

    const CatalogMatch golf5 = catalog::matchVin(decodeVin("WVWZZZ1KZ6W123456", 2026));
    REQUIRE(golf5.generation != nullptr);
    REQUIRE(std::string(golf5.generation->generation) == "V");

    // T = 2026 or 1996. The code 1H only fits 1996, so that year wins.
    const CatalogMatch golf3 = catalog::matchVin(decodeVin("WVWZZZ1HZTW123456", 2026));
    REQUIRE(golf3.generation != nullptr);
    REQUIRE(std::string(golf3.generation->generation) == "III");
    REQUIRE(golf3.year == 1996);

    const CatalogMatch golf8 = catalog::matchVin(decodeVin("WVWZZZCDZMW123456", 2026));
    REQUIRE(golf8.generation != nullptr);
    REQUIRE(std::string(golf8.generation->generation) == "VIII");
    REQUIRE(golf8.year == 2021);

    const CatalogMatch octavia = catalog::matchVin(decodeVin("TMBZZZ1ZZ8B123456", 2026));
    REQUIRE(octavia.generation != nullptr);
    REQUIRE(std::string(octavia.generation->model) == "Octavia");
    REQUIRE(std::string(octavia.generation->generation) == "II");
}

TEST_CASE("Mercedes model series come from positions 4 to 6", "[vin][catalog]") {
    const VinInfo w203 = decodeVin("WDB2030461A123456", 2026);
    REQUIRE(w203.modelCode == "203");
    REQUIRE(w203.modelCodeRule == VinModelCodeRule::Mercedes456);
    // European Mercedes VINs carry no model year.
    REQUIRE(w203.modelYears.empty());
    const CatalogMatch m = catalog::matchVin(w203);
    REQUIRE(m.generation != nullptr);
    REQUIRE(std::string(m.generation->generation) == "W203");
    REQUIRE(m.confidence >= 70);

    const CatalogMatch w204 = catalog::matchVin(decodeVin("WDD2040081A123456", 2026));
    REQUIRE(w204.generation != nullptr);
    REQUIRE(std::string(w204.generation->generation) == "W204");
}

TEST_CASE("Tesla writes the model into position 4", "[vin][catalog]") {
    const VinInfo v = decodeVin(withCheckDigit("5YJ3E1EA0LF000001"), 2026);
    REQUIRE(v.checkDigitOk);
    REQUIRE(v.modelCode == "3");
    REQUIRE(v.modelYear == 2020);
    const CatalogMatch m = catalog::matchVin(v);
    REQUIRE(m.generation != nullptr);
    REQUIRE(std::string(m.generation->model) == "Model 3");
}

TEST_CASE("Without a model code the brand is still known", "[vin][catalog]") {
    const CatalogMatch m = catalog::matchVin(decodeVin("VF7AAAAAAAA000001", 2026));
    REQUIRE(m.confidence <= 30);
    REQUIRE_FALSE(m.reason.empty());

    const CatalogMatch unknown = catalog::matchVin(decodeVin("QQQ", 2026));
    REQUIRE(unknown.generation == nullptr);
    REQUIRE(unknown.confidence == 0);
}

TEST_CASE("The catalog is large and consistent", "[catalog]") {
    REQUIRE(catalog::size() >= 1000);
    std::size_t n = 0;
    const VehicleGeneration* all = catalog::all(n);
    for (std::size_t i = 0; i < n; ++i) {
        const VehicleGeneration& g = all[i];
        INFO(catalog::displayName(g));
        REQUIRE(g.brand[0] != 0);
        REQUIRE(g.model[0] != 0);
        REQUIRE(g.yearFrom >= 1930);
        REQUIRE((g.yearTo == 0 || g.yearTo >= g.yearFrom));
        REQUIRE(catalog::brandInfo(g.brand) != nullptr);
        REQUIRE_FALSE(hasLongDash(g.note.de));
        REQUIRE_FALSE(hasLongDash(g.note.en));
        // An OBD-II socket at some point means there is a protocol order.
        const bool obd = g.connector == Connector::Obd2 || g.lateConnector == Connector::Obd2;
        REQUIRE(obd == (g.protocolOrder[0] != 0));
    }

    const auto brands = catalog::brands();
    REQUIRE(brands.size() >= 80);
    for (const char* b : {"Volkswagen", "Saab", "Pontiac", "Tesla", "BYD", "Lada", "Polestar",
                          "Genesis", "Cupra", "VinFast", "Rover", "Hummer"}) {
        INFO(b);
        REQUIRE(std::find(brands.begin(), brands.end(), b) != brands.end());
    }
    const auto audi = catalog::models("Audi");
    REQUIRE(std::find(audi.begin(), audi.end(), "A2") != audi.end());

    const auto golfs = catalog::generations("Volkswagen", "Golf");
    REQUIRE(golfs.size() >= 8);
    REQUIRE(std::string(golfs.front()->generation) == "I");
}

TEST_CASE("Generations are found by year", "[catalog]") {
    REQUIRE(std::string(catalog::findGeneration("Volkswagen", "Golf", 2002)->generation) == "IV");
    REQUIRE(std::string(catalog::findGeneration("BMW", "3er", 2001)->generation) == "E46");
    REQUIRE(std::string(catalog::findGeneration("Mercedes-Benz", "C-Klasse", 2005)->generation) == "W203");
    // Case and accents do not matter; in change-over years the newer one wins.
    REQUIRE(std::string(catalog::findGeneration("skoda", "octavia", 2008)->generation) == "II");
    // Without a year: the newest.
    REQUIRE(std::string(catalog::findGeneration("Volkswagen", "Golf", 0)->generation) == "VIII");
    REQUIRE(catalog::findGeneration("Volkswagen", "Gibtsnicht", 2002) == nullptr);
}

TEST_CASE("The search is tolerant", "[catalog]") {
    auto first = [](const char* q) {
        const auto r = searchCatalog(q, 5);
        return r.empty() ? std::string() : catalog::displayName(*r.front());
    };
    setLanguage(Language::English);
    REQUIRE(first("golf 4") == "Volkswagen Golf IV (1997 to 2006)");
    REQUIRE(first("Golf IV") == "Volkswagen Golf IV (1997 to 2006)");
    REQUIRE(first("golf4") == "Volkswagen Golf IV (1997 to 2006)");
    REQUIRE(first("a2") == "Audi A2 8Z (2000 to 2005)");
    REQUIRE(first("e46") == "BMW 3er E46 (1998 to 2006)");
    REQUIRE(first("w203") == "Mercedes-Benz C-Klasse W203 (2000 to 2007)");
    REQUIRE(first("octavia 2008") == "Škoda Octavia II (2004 to 2013)");
    REQUIRE(first("model 3") == "Tesla Model 3 I (since 2017)");
    REQUIRE(first("peugeot 2008").rfind("Peugeot 2008", 0) == 0);
    REQUIRE(first("citroen c3 2010") == "Citroën C3 II (2009 to 2016)");
    REQUIRE(first("mx5 nb") == "Mazda MX-5 NB (1998 to 2005)");
    REQUIRE(searchCatalog("xyzzy").empty());
    REQUIRE(searchCatalog("").empty());
    setLanguage(Language::German);
    REQUIRE(first("golf 4") == "Volkswagen Golf IV (1997 bis 2006)");
}

TEST_CASE("Diagnosis is described honestly", "[catalog]") {
    const VehicleGeneration* golf2 = catalog::findGeneration("Volkswagen", "Golf", 1985);
    REQUIRE(catalog::supportFor(*golf2, 1985) == ObdSupport::None);
    REQUIRE(catalog::supportFor(*golf2, 1991) == ObdSupport::Kkl);
    REQUIRE(catalog::protocolOrderFor(*golf2).empty());

    const VehicleGeneration* golf3 = catalog::findGeneration("Volkswagen", "Golf", 1993);
    REQUIRE(catalog::connectorFor(*golf3, 1993) == Connector::Vag2x2);
    REQUIRE(catalog::connectorFor(*golf3, 1997) == Connector::Obd2);
    REQUIRE(catalog::supportFor(*golf3, 1993) == ObdSupport::Kkl);

    const VehicleGeneration* golf4 = catalog::findGeneration("Volkswagen", "Golf", 1999);
    REQUIRE(catalog::supportFor(*golf4, 1999) == ObdSupport::Partial);
    REQUIRE(catalog::kklPossible(*golf4, 1999));
    REQUIRE(catalog::supportFor(*golf4, 2003) == ObdSupport::Full);
    REQUIRE(catalog::supportFor(*golf4, 2002, PtDiesel) == ObdSupport::Partial);
    REQUIRE(golf4->location == SocketLocation::CentreConsole);

    const VehicleGeneration* a2 = catalog::findGeneration("Audi", "A2", 2002);
    REQUIRE(catalog::supportFor(*a2, 2002) == ObdSupport::Full);
    REQUIRE(catalog::protocolOrderFor(*a2) == "5436789");
    REQUIRE(a2->bus == BusKind::KLine);
    REQUIRE_FALSE(catalog::diagnosisText(*a2, 2002).empty());

    const VehicleGeneration* e36 = catalog::findGeneration("BMW", "3er", 1995);
    REQUIRE(catalog::supportFor(*e36, 1995) == ObdSupport::Manufacturer);
    REQUIRE(e36->location == SocketLocation::EngineBay);

    const VehicleGeneration* f150 = catalog::findGeneration("Ford", "F-150", 1998);
    REQUIRE(f150->usMarket);
    REQUIRE(f150->bus == BusKind::Pwm);
    REQUIRE(catalog::supportFor(*f150, 1998) == ObdSupport::Full);

    const VehicleGeneration* leaf = catalog::findGeneration("Nissan", "Leaf", 2015);
    REQUIRE(catalog::supportFor(*leaf, 2015) == ObdSupport::Partial);

    const VehicleGeneration* model3 = catalog::findGeneration("Tesla", "Model 3", 2022);
    REQUIRE(catalog::supportFor(*model3, 2022) == ObdSupport::Manufacturer);
    REQUIRE(model3->location == SocketLocation::RearConsole);
}

TEST_CASE("Enum texts exist in both languages", "[catalog]") {
    for (Language lang : {Language::German, Language::English}) {
        setLanguage(lang);
        for (int i = 0; i <= static_cast<int>(BodyType::Roadster); ++i) {
            REQUIRE(bodyName(static_cast<BodyType>(i))[0] != 0);
        }
        for (int i = 0; i <= static_cast<int>(Connector::Other); ++i) {
            REQUIRE(connectorName(static_cast<Connector>(i))[0] != 0);
            REQUIRE_FALSE(hasLongDash(connectorName(static_cast<Connector>(i))));
        }
        for (int i = 0; i <= static_cast<int>(SocketLocation::RearConsole); ++i) {
            REQUIRE(locationText(static_cast<SocketLocation>(i))[0] != 0);
            REQUIRE_FALSE(hasLongDash(locationText(static_cast<SocketLocation>(i))));
        }
        for (int i = 0; i <= static_cast<int>(ObdSupport::Full); ++i) {
            REQUIRE(supportName(static_cast<ObdSupport>(i))[0] != 0);
        }
    }
    setLanguage(Language::German);
    REQUIRE(powertrainText(PtPetrol | PtDiesel) == "Benzin, Diesel");
    REQUIRE(bodyName(BodyType::Estate) == std::string("Kombi"));
    setLanguage(Language::English);
    REQUIRE(powertrainText(PtElectric) == "Electric");
    setLanguage(Language::German);

    // Every diagnosis text of the catalog is free of long dashes.
    std::size_t n = 0;
    const VehicleGeneration* all = catalog::all(n);
    for (std::size_t i = 0; i < n; ++i) {
        REQUIRE_FALSE(hasLongDash(catalog::diagnosisText(all[i], 0).c_str()));
        REQUIRE_FALSE(locationText(all[i], 0).empty());
    }
}

TEST_CASE("New brands are recognised from the VIN and found in the catalog", "[vehicles][vin]") {
    // WMI after the Wikibooks list; the rest of each VIN is made up.
    const std::pair<const char*, const char*> cases[] = {
        {"HXMAA1234R0000001", "Xiaomi"},  {"L1NSPGHB5PA000001", "Xpeng"},
        {"LJ1EEAUU6P0000001", "NIO"},     {"LW433B1A0P0000001", "Li Auto"},
        {"LFZ63AL55RD000001", "Leapmotor"}, {"LM8F7G1A5PA000001", "Aito"},
        {"LDP95H961PE000001", "Voyah"},   {"LS6A3E0A5PB000001", "Avatr"},
        {"SC6GM1CA5PE000001", "INEOS"},   {"YSMYKEAE5PB000001", "Polestar"},
        {"LSH14J7C5PA000001", "Maxus"},   {"LFPH4ACP5P1000001", "Hongqi"},
    };
    for (const auto& [vin, brand] : cases) {
        INFO(vin);
        const VinInfo v = decodeVin(vin, 2026);
        REQUIRE(v.brand == brand);
        REQUIRE_FALSE(catalog::models(brand).empty());
    }
    // A code shared by several brands of one group lists all of them.
    const VinInfo chery = decodeVin("LVVDB21B5PD000001", 2026);
    REQUIRE(std::find(chery.brandCandidates.begin(), chery.brandCandidates.end(), "Jaecoo") !=
            chery.brandCandidates.end());
}

TEST_CASE("Every catalog brand can be recognised from the VIN", "[vehicles][vin]") {
    std::size_t count = 0;
    const WmiEntry* table = wmiTable(count);
    std::set<std::string> known;
    for (std::size_t i = 0; i < count; ++i) {
        std::string brands = table[i].brands;
        std::size_t start = 0;
        for (;;) {
            const std::size_t bar = brands.find('|', start);
            known.insert(brands.substr(start, bar == std::string::npos ? std::string::npos : bar - start));
            if (bar == std::string::npos) break;
            start = bar + 1;
        }
    }
    for (const std::string& brand : catalog::brands()) {
        INFO(brand);
        REQUIRE(known.count(brand) == 1);
        // Every generation has sensible years.
        for (const std::string& model : catalog::models(brand)) {
            for (const VehicleGeneration* g : catalog::generations(brand, model)) {
                REQUIRE(g->yearFrom >= 1900);
                REQUIRE((g->yearTo == 0 || g->yearTo >= g->yearFrom));
            }
        }
    }
}
