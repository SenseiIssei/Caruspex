#include "data/DtcDatabase.h"

#include <algorithm>
#include <cctype>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "data/DtcTables.h"

namespace caruspex {
namespace {

using dtc::Detail;
using dtc::Family;
using dtc::RangeRule;
using dtc::Record;

constexpr char kSystems[] = {'P', 'C', 'B', 'U'};

std::span<const Record> genericFor(char letter) {
    switch (letter) {
        case 'P': return dtc::genericPowertrain();
        case 'C': return dtc::genericChassis();
        case 'B': return dtc::genericBody();
        case 'U': return dtc::genericNetwork();
        default: return {};
    }
}

// The tables are sorted by code, so a binary search finds an entry.
const Record* findRecord(std::span<const Record> table, const std::string& code) {
    const auto it = std::lower_bound(
        table.begin(), table.end(), code,
        [](const Record& r, const std::string& c) { return std::strcmp(r.code, c.c_str()) < 0; });
    if (it != table.end() && code == it->code) return &*it;
    return nullptr;
}

std::string normalizeCode(const std::string& raw) {
    std::string out;
    out.reserve(raw.size());
    for (const char ch : raw) {
        const auto c = static_cast<unsigned char>(ch);
        if (std::isspace(c)) continue;
        out += static_cast<char>(std::toupper(c));
    }
    return out;
}

bool isValidCode(const std::string& code) {
    if (code.size() != 5) return false;
    if (code[0] != 'P' && code[0] != 'C' && code[0] != 'B' && code[0] != 'U') return false;
    if (code[1] < '0' || code[1] > '3') return false;
    for (std::size_t i = 2; i < 5; ++i)
        if (!std::isxdigit(static_cast<unsigned char>(code[i]))) return false;
    return true;
}

int codeNumber(const std::string& code) {
    return static_cast<int>(std::strtol(code.c_str() + 1, nullptr, 16));
}

// SAE leaves P1, P30 to P33 and the 1 and 2 blocks of C, B and U to the
// manufacturers.
bool isManufacturerRange(const std::string& code) {
    const char second = code[1];
    if (code[0] == 'P') {
        if (second == '1') return true;
        if (second == '3') return code[2] >= '0' && code[2] <= '3';
        return false;
    }
    return second == '1' || second == '2';
}

const RangeRule* findRule(const std::string& code) {
    const int number = codeNumber(code);
    for (const RangeRule& r : dtc::rangeRules())
        if (r.letter == code[0] && number >= r.lo && number <= r.hi) return &r;
    return nullptr;
}

// Folds the accented Latin letters that occur in brand names (Škoda,
// Citroën) to plain ASCII. Two-byte UTF-8 sequences only; anything else
// is dropped, like in normalize_brand() of the generator.
char foldLatin(unsigned char lead, unsigned char trail) {
    if (lead == 0xC3) {
        if (trail == 0x9F) return 0;  // sharp s has no plain form here
        const unsigned char x = trail | 0x20;  // upper case to lower case
        if (x >= 0xA0 && x <= 0xA5) return 'a';
        if (x == 0xA7) return 'c';
        if (x >= 0xA8 && x <= 0xAB) return 'e';
        if (x >= 0xAC && x <= 0xAF) return 'i';
        if (x == 0xB1) return 'n';
        if (x >= 0xB2 && x <= 0xB6) return 'o';
        if (x >= 0xB9 && x <= 0xBC) return 'u';
        if (x == 0xBD || trail == 0xBF) return 'y';
        return 0;
    }
    if (lead == 0xC4 && (trail == 0x8C || trail == 0x8D)) return 'c';
    if (lead == 0xC5) {
        if (trail == 0xA0 || trail == 0xA1) return 's';
        if (trail == 0xBD || trail == 0xBE) return 'z';
    }
    return 0;
}

// "Mercedes-Benz" -> "mercedesbenz", "Škoda" -> "skoda".
std::string normalizeBrand(const std::string& brand) {
    std::string out;
    for (std::size_t i = 0; i < brand.size(); ++i) {
        const auto c = static_cast<unsigned char>(brand[i]);
        if (c < 0x80) {
            if (std::isalnum(c)) out += static_cast<char>(std::tolower(c));
            continue;
        }
        if (c >= 0xC0 && c < 0xE0 && i + 1 < brand.size()) {
            const char folded = foldLatin(c, static_cast<unsigned char>(brand[i + 1]));
            if (folded) out += folded;
            ++i;
        }
    }
    return out;
}

const Family* familyByKey(const std::string& key) {
    if (key.empty()) return nullptr;
    for (const Family& f : dtc::families())
        if (key == f.key) return &f;
    return nullptr;
}

// Exact match on a family key or brand name first. Failing that, the
// longest brand name the input starts with, so "VW Nutzfahrzeuge" and
// "Mini Cooper" find their family too.
const Family* familyForBrand(const std::string& brand) {
    const std::string name = normalizeBrand(brand);
    if (name.empty()) return nullptr;
    const Family* best = nullptr;
    std::size_t bestLength = 0;
    for (const Family& f : dtc::families()) {
        if (name == normalizeBrand(f.key)) return &f;
        const char* p = f.brands;
        while (*p) {
            const char* end = std::strchr(p, '|');
            const std::size_t length = end ? static_cast<std::size_t>(end - p) : std::strlen(p);
            const std::string alias(p, length);
            if (name == alias) return &f;
            if (alias.size() >= 3 && alias.size() > bestLength && name.compare(0, alias.size(), alias) == 0) {
                best = &f;
                bestLength = alias.size();
            }
            p += length;
            if (*p == '|') ++p;
        }
    }
    return best;
}

std::vector<std::string> splitLines(const char* text) {
    std::vector<std::string> out;
    const char* p = text;
    while (*p) {
        const char* end = std::strchr(p, '\n');
        const std::size_t length = end ? static_cast<std::size_t>(end - p) : std::strlen(p);
        if (length > 0) out.emplace_back(p, length);
        p += length;
        if (*p == '\n') ++p;
    }
    return out;
}

// The lookup behind every public function. family may be null. With
// labelFamily the fallback text of a manufacturer code names the family.
DtcInfo lookup(const std::string& raw, const Family* family, bool labelFamily) {
    DtcInfo info;
    info.code = normalizeCode(raw);
    info.systemName = dtcSystemName(DtcSystem::Unknown);
    if (!isValidCode(info.code)) {
        info.text = L("unbekannter Code", "unknown code");
        info.advice = L("Das ist kein gültiger Fehlercode.", "This is not a valid trouble code.");
        return info;
    }
    info.manufacturerSpecific = isManufacturerRange(info.code);

    const Record* record = nullptr;
    if (family) {
        record = findRecord(family->records, info.code);
        if (record) {
            info.family = family->key;
            info.familyLabel = family->label.get();
        }
    }
    if (!record) record = findRecord(genericFor(info.code[0]), info.code);
    const RangeRule* rule = findRule(info.code);

    std::uint16_t detail = dtc::kNoDetail;
    if (record) {
        info.known = true;
        info.text = record->text.get();
        info.severity = record->severity;
        info.system = record->system;
        info.throttleRelated = record->throttle;
        detail = record->detail;
    } else {
        std::string text = rule ? rule->text.get() : L("Fehlercode", "Trouble code");
        if (info.manufacturerSpecific) {
            if (family && labelFamily)
                text += std::string(L(" (herstellerspezifisch, ", " (manufacturer-specific, ")) +
                        family->label.get() + ")";
            else
                text += L(" (herstellerspezifisch)", " (manufacturer-specific)");
        }
        info.text = text + L(" - kein Klartext hinterlegt", " - no description available");
        if (rule) {
            info.severity = rule->severity;
            info.system = rule->system;
        }
    }
    // A code without help of its own borrows the help of its range, where
    // the range has one (airbags, lost communication and the like).
    if (detail == dtc::kNoDetail && rule) detail = rule->detail;

    info.systemName = dtcSystemName(info.system);
    const auto table = dtc::details();
    if (detail < table.size()) {
        const Detail& d = table[detail];
        info.hasDetails = true;
        info.advice = d.advice.get();
        info.causes = splitLines(d.causes.get());
        info.checks = splitLines(d.checks.get());
        info.costClass = d.cost;
    } else {
        info.advice = severityAdvice(info.severity);
    }
    return info;
}

const Family* vagFamily() {
    static const Family* vag = familyByKey("VAG");
    return vag;
}

// The generic table plus the VAG table, which is what the database held
// before brands were known. Kept for the existing callers.
template <typename Fn>
void forEachLegacyRecord(Fn fn) {
    for (const char letter : kSystems)
        for (const Record& r : genericFor(letter)) fn(r);
    if (const Family* vag = vagFamily())
        for (const Record& r : vag->records) fn(r);
}

std::unordered_map<std::string, std::string> buildDescriptions(Language lang) {
    std::unordered_map<std::string, std::string> map;
    map.reserve(dtcTexts().size());
    for (const auto& [code, text] : dtcTexts())
        map.emplace(code, lang == Language::English ? text.en : text.de);
    return map;
}

}  // namespace

const char* dtcSystemName(DtcSystem system) {
    switch (system) {
        case DtcSystem::Unknown: return L("Allgemein", "General");
        case DtcSystem::FuelAir: return L("Gemischbildung und Kraftstoff", "Fuel and mixture");
        case DtcSystem::AirIntake: return L("Luftansaugung", "Air intake");
        case DtcSystem::Throttle: return L("Drosselklappe und Fahrpedal", "Throttle and accelerator pedal");
        case DtcSystem::Ignition: return L("Zündung und Verbrennung", "Ignition and combustion");
        case DtcSystem::ValveTiming: return L("Steuerzeiten und Nockenwelle", "Valve timing and camshaft");
        case DtcSystem::Injection: return L("Einspritzung", "Fuel injection");
        case DtcSystem::Turbo: return L("Turbolader und Ladedruck", "Turbocharger and boost");
        case DtcSystem::Cooling: return L("Kühlung", "Cooling");
        case DtcSystem::Lubrication: return L("Motoröl und Schmierung", "Engine oil and lubrication");
        case DtcSystem::Exhaust:
            return L("Abgas: Lambdasonden und Katalysator",
                     "Exhaust: oxygen sensors and catalytic converter");
        case DtcSystem::Egr: return L("Abgasrückführung und Sekundärluft", "EGR and secondary air");
        case DtcSystem::Evap: return L("Tankentlüftung", "Fuel vapour system (EVAP)");
        case DtcSystem::DieselAftertreatment:
            return L("Diesel: Partikelfilter, AdBlue, Glühanlage",
                     "Diesel: particulate filter, AdBlue, glow plugs");
        case DtcSystem::Charging: return L("Batterie und Lichtmaschine", "Battery and charging");
        case DtcSystem::EngineControl: return L("Motorsteuerung", "Engine control");
        case DtcSystem::Transmission: return L("Getriebe und Kupplung", "Transmission and clutch");
        case DtcSystem::Hybrid: return L("Hybrid- und Elektroantrieb", "Hybrid and electric drive");
        case DtcSystem::Network: return L("Datenbus und Vernetzung", "Data bus and networking");
        case DtcSystem::Brakes: return L("Bremsen, ABS und ESP", "Brakes, ABS and stability control");
        case DtcSystem::Steering: return L("Lenkung", "Steering");
        case DtcSystem::Chassis: return L("Fahrwerk", "Chassis");
        case DtcSystem::Airbag: return L("Airbag und Gurtstraffer", "Airbags and seat belt pretensioners");
        case DtcSystem::Body: return L("Karosserie und Komfort", "Body and comfort");
        case DtcSystem::Immobilizer: return L("Wegfahrsperre", "Immobilizer");
        case DtcSystem::Climate: return L("Klimaanlage und Heizung", "Air conditioning and heating");
    }
    return L("Allgemein", "General");
}

const char* costClassLabel(CostClass cost) {
    switch (cost) {
        case CostClass::Cheap: return L("günstig", "cheap");
        case CostClass::Medium: return L("mittel", "moderate");
        case CostClass::Expensive: return L("teuer", "expensive");
        case CostClass::Unknown: break;
    }
    return "";
}

const char* costClassHint(CostClass cost) {
    switch (cost) {
        case CostClass::Cheap:
            return L("Reparatur in der Werkstatt grob unter 200 Euro",
                     "Workshop repair roughly below 200 euros");
        case CostClass::Medium:
            return L("Reparatur in der Werkstatt grob 200 bis 800 Euro",
                     "Workshop repair roughly 200 to 800 euros");
        case CostClass::Expensive:
            return L("Reparatur in der Werkstatt grob über 800 Euro",
                     "Workshop repair roughly above 800 euros");
        case CostClass::Unknown: break;
    }
    return "";
}

const char* severityAdvice(Severity severity) {
    switch (severity) {
        case Severity::Info:
            return L("Kein Fehler im eigentlichen Sinn, Weiterfahren ist in Ordnung.",
                     "Not a fault as such, it is fine to keep driving.");
        case Severity::Low:
            return L("Weiterfahren ist in Ordnung. Bei Gelegenheit prüfen lassen.",
                     "It is fine to keep driving. Have it checked when convenient.");
        case Severity::Medium:
            return L("Weiterfahren ist möglich, aber bald in die Werkstatt: Verbrauch, Abgaswerte "
                     "oder Komfort können leiden.",
                     "You can keep driving, but go to a workshop soon: consumption, emissions or "
                     "comfort may suffer.");
        case Severity::High:
            return L("Nur noch behutsam und auf kurzen Strecken fahren und in den nächsten Tagen "
                     "prüfen lassen, sonst können Folgeschäden entstehen.",
                     "Drive gently and only short distances, and have it checked within the next "
                     "few days, otherwise further damage can follow.");
        case Severity::Critical:
            return L("Nicht weiterfahren. Anhalten, sobald es sicher geht, Motor abstellen und "
                     "Hilfe holen.",
                     "Do not drive on. Stop as soon as it is safe, switch off the engine and get "
                     "help.");
    }
    return "";
}

DtcInfo lookupDtc(const std::string& code, const std::string& brand) {
    return lookup(code, familyForBrand(brand), true);
}

std::string brandFamily(const std::string& brand) {
    const Family* f = familyForBrand(brand);
    return f ? f->key : std::string();
}

std::string brandFamilyLabel(const std::string& family) {
    if (family.empty()) return L("SAE/ISO (alle Marken)", "SAE/ISO (all brands)");
    const Family* f = familyByKey(family);
    return f ? f->label.get() : std::string();
}

const std::vector<std::string>& brandFamilies() {
    static const std::vector<std::string> keys = [] {
        std::vector<std::string> out;
        for (const Family& f : dtc::families()) out.emplace_back(f.key);
        return out;
    }();
    return keys;
}

std::string decodeDtc(int byteA, int byteB) {
    if (byteA == 0 && byteB == 0) return {};
    char buffer[8];
    std::snprintf(buffer, sizeof(buffer), "%c%X%X%X%X", kSystems[(byteA >> 6) & 0x03],
                  (byteA >> 4) & 0x03, byteA & 0x0F, (byteB >> 4) & 0x0F, byteB & 0x0F);
    return buffer;
}

std::string describeDtc(const std::string& code) {
    return lookup(code, vagFamily(), false).text;
}

std::string describeDtc(const std::string& code, const std::string& brand) {
    return lookupDtc(code, brand).text;
}

bool isThrottleDtc(const std::string& code) {
    const auto& set = dtcThrottleCodes();
    return set.find(code) != set.end();
}

const std::unordered_map<std::string, Text>& dtcTexts() {
    static const std::unordered_map<std::string, Text> texts = [] {
        std::unordered_map<std::string, Text> map;
        forEachLegacyRecord([&](const Record& r) { map.emplace(r.code, r.text); });
        return map;
    }();
    return texts;
}

const std::unordered_set<std::string>& dtcThrottleCodes() {
    static const std::unordered_set<std::string> codes = [] {
        std::unordered_set<std::string> set;
        forEachLegacyRecord([&](const Record& r) {
            if (r.throttle) set.emplace(r.code);
        });
        return set;
    }();
    return codes;
}

// One map per language, built on first use so that L() is never called
// during static initialisation.
const std::unordered_map<std::string, std::string>& dtcDescriptions() {
    if (language() == Language::English) {
        static const auto english = buildDescriptions(Language::English);
        return english;
    }
    static const auto german = buildDescriptions(Language::German);
    return german;
}

std::vector<DtcLexiconEntry> dtcLexicon() {
    std::vector<DtcLexiconEntry> out;
    const std::string genericLabel = brandFamilyLabel("");
    auto add = [&](const Record& r, const std::string& family, const std::string& label) {
        out.push_back({r.code, family, label, r.text.get(), r.severity, r.system});
    };
    for (const char letter : kSystems)
        for (const Record& r : genericFor(letter)) add(r, std::string(), genericLabel);
    for (const Family& f : dtc::families()) {
        const std::string label = f.label.get();
        for (const Record& r : f.records) add(r, f.key, label);
    }
    return out;
}

std::size_t dtcCount(const std::string& family) {
    if (family.empty()) {
        std::size_t total = 0;
        for (const char letter : kSystems) total += genericFor(letter).size();
        return total;
    }
    const Family* f = familyByKey(family);
    return f ? f->records.size() : 0;
}

}  // namespace caruspex
