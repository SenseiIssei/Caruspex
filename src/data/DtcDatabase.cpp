#include "data/DtcDatabase.h"

#include <array>
#include <cstdio>
#include <cstdlib>

namespace caruspex {
namespace {

constexpr char kSystems[] = {'P', 'C', 'B', 'U'};

struct Range {
    int lo;
    int hi;
    Text text;
};

constexpr std::array<Range, 9> kRanges{{
    {0x0000, 0x0099,
     {"Kraftstoff-/Luftgemisch und Zusatzemission",
      "Fuel/air mixture and auxiliary emission controls"}},
    {0x0100, 0x0199,
     {"Kraftstoff- und Luftmessung (Sensoren, Gemischbildung)",
      "Fuel and air metering (sensors, mixture formation)"}},
    {0x0200, 0x0299,
     {"Kraftstoff- und Luftmessung (Einspritzung, Ladedruck)",
      "Fuel and air metering (injection, boost pressure)"}},
    {0x0300, 0x0399, {"Zuendanlage / Verbrennungsaussetzer", "Ignition system / misfire"}},
    {0x0400, 0x0499,
     {"Zusatzemission (AGR, Sekundaerluft, Tankentlueftung, Luefter)",
      "Auxiliary emission controls (EGR, secondary air, EVAP, fan)"}},
    {0x0500, 0x0599,
     {"Geschwindigkeit, Leerlaufregelung, Nebenaggregate",
      "Vehicle speed, idle control, auxiliary units"}},
    {0x0600, 0x0699,
     {"Steuergeraet, Rechner, Ausgaenge", "Control unit, computer, outputs"}},
    {0x0700, 0x0899, {"Getriebe", "Transmission"}},
    {0x1000, 0x3FFF,
     {"herstellerspezifischer Code (VAG)", "manufacturer-specific code (VAG)"}},
}};

// One map per language, built on first use so that L() is never called
// during static initialisation.
std::unordered_map<std::string, std::string> buildDescriptions(Language lang) {
    std::unordered_map<std::string, std::string> map;
    map.reserve(dtcTexts().size());
    for (const auto& [code, text] : dtcTexts())
        map.emplace(code, lang == Language::English ? text.en : text.de);
    return map;
}

}  // namespace

const std::unordered_map<std::string, std::string>& dtcDescriptions() {
    if (language() == Language::English) {
        static const auto english = buildDescriptions(Language::English);
        return english;
    }
    static const auto german = buildDescriptions(Language::German);
    return german;
}

std::string decodeDtc(int byteA, int byteB) {
    if (byteA == 0 && byteB == 0) return {};
    char buffer[8];
    std::snprintf(buffer, sizeof(buffer), "%c%X%X%X%X", kSystems[(byteA >> 6) & 0x03],
                  (byteA >> 4) & 0x03, byteA & 0x0F, (byteB >> 4) & 0x0F, byteB & 0x0F);
    return buffer;
}

std::string describeDtc(const std::string& code) {
    const auto& table = dtcTexts();
    const auto it = table.find(code);
    if (it != table.end()) return it->second.get();
    if (code.size() != 5) return L("unbekannter Code", "unknown code");

    const char* noText = L(" - kein Klartext hinterlegt", " - no description available");
    if (code[0] == 'P') {
        const int number = static_cast<int>(std::strtol(code.substr(1).c_str(), nullptr, 16));
        for (const Range& r : kRanges) {
            if (number >= r.lo && number <= r.hi) {
                std::string text = r.text.get();
                if (code[1] == '1' || code[1] == '7')
                    text += L(" (herstellerspezifisch, VAG)", " (manufacturer-specific, VAG)");
                return text + noText;
            }
        }
    }
    switch (code[0]) {
        case 'C': return std::string(L("Fahrwerk (ABS/ESP)", "Chassis (ABS/ESP)")) + noText;
        case 'B':
            return std::string(L("Karosserie (Airbag/Komfort)", "Body (airbag/comfort)")) + noText;
        case 'U': return std::string(L("Netzwerk/Bus", "Network/bus")) + noText;
        default: return L("kein Klartext hinterlegt", "no description available");
    }
}

bool isThrottleDtc(const std::string& code) {
    const auto& set = dtcThrottleCodes();
    return set.find(code) != set.end();
}

}  // namespace caruspex
