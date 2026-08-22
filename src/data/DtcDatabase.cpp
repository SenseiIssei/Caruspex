#include "data/DtcDatabase.h"

#include <array>
#include <cstdio>

namespace caruspex {
namespace {

constexpr char kSystems[] = {'P', 'C', 'B', 'U'};

struct Range {
    int lo;
    int hi;
    const char* text;
};

constexpr std::array<Range, 9> kRanges{{
    {0x0000, 0x0099, "Kraftstoff-/Luftgemisch und Zusatzemission"},
    {0x0100, 0x0199, "Kraftstoff- und Luftmessung (Sensoren, Gemischbildung)"},
    {0x0200, 0x0299, "Kraftstoff- und Luftmessung (Einspritzung, Ladedruck)"},
    {0x0300, 0x0399, "Zuendanlage / Verbrennungsaussetzer"},
    {0x0400, 0x0499, "Zusatzemission (AGR, Sekundaerluft, Tankentlueftung, Luefter)"},
    {0x0500, 0x0599, "Geschwindigkeit, Leerlaufregelung, Nebenaggregate"},
    {0x0600, 0x0699, "Steuergeraet, Rechner, Ausgaenge"},
    {0x0700, 0x0899, "Getriebe"},
    {0x1000, 0x3FFF, "herstellerspezifischer Code (VAG)"},
}};

}  // namespace

std::string decodeDtc(int byteA, int byteB) {
    if (byteA == 0 && byteB == 0) return {};
    char buffer[8];
    std::snprintf(buffer, sizeof(buffer), "%c%X%X%X%X", kSystems[(byteA >> 6) & 0x03],
                  (byteA >> 4) & 0x03, byteA & 0x0F, (byteB >> 4) & 0x0F, byteB & 0x0F);
    return buffer;
}

std::string describeDtc(const std::string& code) {
    const auto& table = dtcDescriptions();
    const auto it = table.find(code);
    if (it != table.end()) return it->second;
    if (code.size() != 5) return "unbekannter Code";

    if (code[0] == 'P') {
        const int number = static_cast<int>(std::strtol(code.substr(1).c_str(), nullptr, 16));
        for (const Range& r : kRanges) {
            if (number >= r.lo && number <= r.hi) {
                std::string text = r.text;
                if (code[1] == '1' || code[1] == '7') text += " (herstellerspezifisch, VAG)";
                return text + " - kein Klartext hinterlegt";
            }
        }
    }
    switch (code[0]) {
        case 'C': return "Fahrwerk (ABS/ESP) - kein Klartext hinterlegt";
        case 'B': return "Karosserie (Airbag/Komfort) - kein Klartext hinterlegt";
        case 'U': return "Netzwerk/Bus - kein Klartext hinterlegt";
        default: return "kein Klartext hinterlegt";
    }
}

bool isThrottleDtc(const std::string& code) {
    const auto& set = dtcThrottleCodes();
    return set.find(code) != set.end();
}

}  // namespace caruspex
