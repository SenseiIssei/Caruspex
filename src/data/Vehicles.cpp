#include "data/Vehicles.h"

#include <algorithm>
#include <set>

#include "core/Lang.h"

namespace caruspex {

// Protocol orders as ELM327 numbers:
//   1 J1850 PWM, 2 J1850 VPW, 3 ISO 9141-2, 4 KWP 5-baud, 5 KWP fast,
//   6 CAN 11/500, 7 CAN 29/500, 8 CAN 11/250, 9 CAN 29/250
const char* const kAutoProtocolOrder = "6785439121";

namespace {

constexpr const char* kCan = "6789543";
constexpr const char* kKLine = "5436789";
constexpr const char* kMixed = "6543789";
constexpr const char* kPwm = "1265439";
constexpr const char* kVpw = "2165439";

const Text kAutoDetect = {"Automatisch erkennen", "Auto-detect"};

// Notes that occur many times.
const Text kNoteCan = {"CAN.", "CAN."};
const Text kNoteIso = {"ISO 9141-2.", "ISO 9141-2."};
const Text kNoteKLineEra = {"K-Line-Zeit.", "K-Line era."};
const Text kNoteNone = {"", ""};

// Builds the profile list for one language. Only the notes and the
// auto-detect entry differ between the languages.
std::vector<VehicleProfile> build(Language lang) {
    const bool en = lang == Language::English;
    std::vector<VehicleProfile> v;
    auto add = [&v, en](const char* brand, const char* model, int from, int to, BusKind bus,
                        const char* order, const Text& note) {
        v.push_back({brand, model, from, to, bus, order, en ? note.en : note.de});
    };

    // --- Volkswagen Group ---------------------------------------------------
    add("Audi", "A2", 2000, 2005, BusKind::KLine, kKLine,
        {"K-Line. EOBD ueber das Motorsteuergeraet, alles Weitere braucht KKL.",
         "K-Line. EOBD via the engine control unit, everything else needs a KKL cable."});
    add("Audi", "", 1996, 2003, BusKind::KLine, kKLine,
        {"K-Line-Zeit, KWP1281 oder KWP2000.", "K-Line era, KWP1281 or KWP2000."});
    add("Audi", "", 2004, 2030, BusKind::Can, kCan,
        {"CAN, ab Werk EOBD-konform.", "CAN, EOBD compliant from the factory."});
    add("Volkswagen", "", 1996, 2003, BusKind::KLine, kKLine, kNoteKLineEra);
    add("Volkswagen", "", 2004, 2030, BusKind::Can, kCan, kNoteCan);
    add("Skoda", "", 1998, 2003, BusKind::KLine, kKLine, kNoteKLineEra);
    add("Skoda", "", 2004, 2030, BusKind::Can, kCan, kNoteCan);
    add("Seat", "", 1998, 2003, BusKind::KLine, kKLine, kNoteKLineEra);
    add("Seat", "", 2004, 2030, BusKind::Can, kCan, kNoteCan);
    add("Porsche", "", 1998, 2030, BusKind::Mixed, kMixed,
        {"Ab 2003 durchgaengig CAN.", "CAN throughout from 2003."});

    // --- German manufacturers -----------------------------------------------
    add("BMW", "", 1996, 2003, BusKind::KLine, kKLine,
        {"ISO 9141-2, teils BMW-eigen.", "ISO 9141-2, partly BMW-specific."});
    add("BMW", "", 2004, 2030, BusKind::Can, kCan, kNoteCan);
    add("Mini", "", 2001, 2030, BusKind::Mixed, kMixed,
        {"Bis 2006 teils K-Line.", "Partly K-Line until 2006."});
    add("Mercedes-Benz", "", 1996, 2003, BusKind::KLine, kKLine, kNoteIso);
    add("Mercedes-Benz", "", 2004, 2030, BusKind::Can, kCan, kNoteCan);
    add("Smart", "", 1998, 2030, BusKind::Mixed, kMixed,
        {"Frueh K-Line, spaeter CAN.", "K-Line early on, CAN later."});
    add("Opel", "", 1996, 2005, BusKind::Mixed, kMixed,
        {"Teils ISO 9141-2, teils J1850 VPW.", "Partly ISO 9141-2, partly J1850 VPW."});
    add("Opel", "", 2006, 2030, BusKind::Can, kCan, kNoteCan);

    // --- France -------------------------------------------------------------
    add("Renault", "", 1996, 2004, BusKind::KLine, kKLine, kNoteIso);
    add("Renault", "", 2005, 2030, BusKind::Can, kCan, kNoteCan);
    add("Dacia", "", 2004, 2030, BusKind::Mixed, kMixed,
        {"Technik von Renault.", "Renault technology."});
    add("Peugeot", "", 1996, 2004, BusKind::KLine, kKLine, kNoteIso);
    add("Peugeot", "", 2005, 2030, BusKind::Can, kCan, kNoteCan);
    add("Citroen", "", 1996, 2004, BusKind::KLine, kKLine, kNoteIso);
    add("Citroen", "", 2005, 2030, BusKind::Can, kCan, kNoteCan);
    add("DS", "", 2014, 2030, BusKind::Can, kCan, kNoteCan);

    // --- Italy --------------------------------------------------------------
    add("Fiat", "", 1996, 2005, BusKind::KLine, kKLine,
        {"ISO 9141-2, teils KWP.", "ISO 9141-2, partly KWP."});
    add("Fiat", "", 2006, 2030, BusKind::Can, kCan, kNoteCan);
    add("Alfa Romeo", "", 1996, 2005, BusKind::KLine, kKLine, kNoteIso);
    add("Alfa Romeo", "", 2006, 2030, BusKind::Can, kCan, kNoteCan);
    add("Lancia", "", 1996, 2005, BusKind::KLine, kKLine, kNoteIso);
    add("Ferrari", "", 1998, 2030, BusKind::Mixed, kMixed, kNoteNone);
    add("Maserati", "", 2002, 2030, BusKind::Mixed, kMixed, kNoteNone);

    // --- Sweden and Great Britain -------------------------------------------
    add("Volvo", "", 1996, 2004, BusKind::KLine, kKLine, kNoteIso);
    add("Volvo", "", 2005, 2030, BusKind::Can, kCan, kNoteCan);
    add("Saab", "", 1996, 2011, BusKind::Mixed, kMixed,
        {"Teils J1850 VPW wie GM.", "Partly J1850 VPW like GM."});
    add("Jaguar", "", 1996, 2030, BusKind::Mixed, kMixed, kNoteNone);
    add("Land Rover", "", 1996, 2030, BusKind::Mixed, kMixed, kNoteNone);

    // --- Japan --------------------------------------------------------------
    add("Toyota", "", 1996, 2003, BusKind::KLine, kKLine, kNoteIso);
    add("Toyota", "", 2004, 2030, BusKind::Can, kCan, kNoteCan);
    add("Lexus", "", 1998, 2030, BusKind::Mixed, kMixed, kNoteNone);
    add("Honda", "", 1996, 2003, BusKind::KLine, kKLine, kNoteIso);
    add("Honda", "", 2004, 2030, BusKind::Can, kCan, kNoteCan);
    add("Nissan", "", 1996, 2004, BusKind::KLine, kKLine, kNoteIso);
    add("Nissan", "", 2005, 2030, BusKind::Can, kCan, kNoteCan);
    add("Mazda", "", 1996, 2004, BusKind::Mixed, kMixed,
        {"Teils J1850 PWM wie Ford.", "Partly J1850 PWM like Ford."});
    add("Mazda", "", 2005, 2030, BusKind::Can, kCan, kNoteCan);
    add("Mitsubishi", "", 1996, 2005, BusKind::KLine, kKLine, kNoteIso);
    add("Mitsubishi", "", 2006, 2030, BusKind::Can, kCan, kNoteCan);
    add("Subaru", "", 1996, 2005, BusKind::KLine, kKLine, kNoteIso);
    add("Subaru", "", 2006, 2030, BusKind::Can, kCan, kNoteCan);
    add("Suzuki", "", 1998, 2005, BusKind::KLine, kKLine, kNoteIso);
    add("Suzuki", "", 2006, 2030, BusKind::Can, kCan, kNoteCan);
    add("Daihatsu", "", 1998, 2030, BusKind::Mixed, kMixed, kNoteNone);

    // --- Korea --------------------------------------------------------------
    add("Hyundai", "", 1998, 2005, BusKind::KLine, kKLine, kNoteIso);
    add("Hyundai", "", 2006, 2030, BusKind::Can, kCan, kNoteCan);
    add("Kia", "", 1998, 2005, BusKind::KLine, kKLine, kNoteIso);
    add("Kia", "", 2006, 2030, BusKind::Can, kCan, kNoteCan);
    add("SsangYong", "", 2000, 2030, BusKind::Mixed, kMixed, kNoteNone);

    // --- USA ----------------------------------------------------------------
    add("Ford", "", 1996, 2003, BusKind::Pwm, kPwm, {"SAE J1850 PWM.", "SAE J1850 PWM."});
    add("Ford", "", 2004, 2030, BusKind::Can, kCan, kNoteCan);
    add("Chevrolet", "", 1996, 2005, BusKind::Vpw, kVpw, {"SAE J1850 VPW.", "SAE J1850 VPW."});
    add("Chevrolet", "", 2006, 2030, BusKind::Can, kCan, kNoteCan);
    add("Chrysler", "", 1996, 2005, BusKind::Mixed, kMixed, kNoteNone);
    add("Jeep", "", 1996, 2005, BusKind::Mixed, kMixed, kNoteNone);
    add("Jeep", "", 2006, 2030, BusKind::Can, kCan, kNoteCan);
    add("Dodge", "", 1996, 2030, BusKind::Mixed, kMixed, kNoteNone);
    add("Tesla", "", 2012, 2030, BusKind::Can, kCan,
        {"CAN, EOBD nur eingeschraenkt.", "CAN, EOBD only partly supported."});

    // --- Other --------------------------------------------------------------
    add("Iveco", "", 2000, 2030, BusKind::Mixed, kMixed,
        {"Nutzfahrzeug.", "Commercial vehicle."});
    add("MAN", "", 2000, 2030, BusKind::Can, kCan,
        {"Nutzfahrzeug, teils J1939.", "Commercial vehicle, partly J1939."});
    add(en ? kAutoDetect.en : kAutoDetect.de, "", 1996, 2030, BusKind::Mixed, kAutoProtocolOrder,
        {"Probiert alle Protokolle der Reihe nach durch. Funktioniert immer, "
         "dauert nur ein paar Sekunden laenger.",
         "Tries all protocols one after another. Always works, "
         "it just takes a few seconds longer."});
    return v;
}

}  // namespace

// One list per language, both built on first use. The active language
// decides which one is handed out.
const std::vector<VehicleProfile>& vehicleProfiles() {
    static const std::vector<VehicleProfile> german = build(Language::German);
    static const std::vector<VehicleProfile> english = build(Language::English);
    return language() == Language::English ? english : german;
}

std::vector<std::string> vehicleBrands() {
    std::set<std::string> unique;
    for (const VehicleProfile& p : vehicleProfiles()) unique.insert(p.brand);
    return {unique.begin(), unique.end()};
}

std::vector<const VehicleProfile*> profilesForBrand(const std::string& brand) {
    std::vector<const VehicleProfile*> out;
    for (const VehicleProfile& p : vehicleProfiles()) {
        if (p.brand == brand) out.push_back(&p);
    }
    return out;
}

const VehicleProfile* findProfile(const std::string& brand, int year) {
    const VehicleProfile* fallback = nullptr;
    for (const VehicleProfile& p : vehicleProfiles()) {
        if (p.brand != brand) continue;
        fallback = &p;
        if (year >= p.yearFrom && year <= p.yearTo) return &p;
    }
    return fallback;
}

const char* autoDetectBrand() { return kAutoDetect.get(); }

std::string busName(BusKind bus) {
    switch (bus) {
        case BusKind::KLine: return "K-Line (ISO 9141-2 / KWP2000)";
        case BusKind::Can: return "CAN (ISO 15765-4)";
        case BusKind::Pwm: return "SAE J1850 PWM";
        case BusKind::Vpw: return "SAE J1850 VPW";
        case BusKind::Mixed: return L("gemischt, wird ermittelt", "mixed, will be detected");
    }
    return L("unbekannt", "unknown");
}

}  // namespace caruspex
