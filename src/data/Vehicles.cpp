#include "data/Vehicles.h"

#include <algorithm>
#include <set>

namespace caruspex {

// Protokollreihenfolgen als ELM327-Nummern:
//   1 J1850 PWM, 2 J1850 VPW, 3 ISO 9141-2, 4 KWP 5-Baud, 5 KWP Fast,
//   6 CAN 11/500, 7 CAN 29/500, 8 CAN 11/250, 9 CAN 29/250
const char* const kAutoProtocolOrder = "6785439121";

namespace {

constexpr const char* kCan = "6789543";
constexpr const char* kKLine = "5436789";
constexpr const char* kMixed = "6543789";
constexpr const char* kPwm = "1265439";
constexpr const char* kVpw = "2165439";

std::vector<VehicleProfile> build() {
    std::vector<VehicleProfile> v;
    auto add = [&v](const char* brand, const char* model, int from, int to, BusKind bus,
                    const char* order, const char* note) {
        v.push_back({brand, model, from, to, bus, order, note});
    };

    // --- Volkswagen-Konzern -------------------------------------------------
    add("Audi", "A2", 2000, 2005, BusKind::KLine, kKLine,
        "K-Line. EOBD ueber das Motorsteuergeraet, alles Weitere braucht KKL.");
    add("Audi", "", 1996, 2003, BusKind::KLine, kKLine, "K-Line-Zeit, KWP1281 oder KWP2000.");
    add("Audi", "", 2004, 2030, BusKind::Can, kCan, "CAN, ab Werk EOBD-konform.");
    add("Volkswagen", "", 1996, 2003, BusKind::KLine, kKLine, "K-Line-Zeit.");
    add("Volkswagen", "", 2004, 2030, BusKind::Can, kCan, "CAN.");
    add("Skoda", "", 1998, 2003, BusKind::KLine, kKLine, "K-Line-Zeit.");
    add("Skoda", "", 2004, 2030, BusKind::Can, kCan, "CAN.");
    add("Seat", "", 1998, 2003, BusKind::KLine, kKLine, "K-Line-Zeit.");
    add("Seat", "", 2004, 2030, BusKind::Can, kCan, "CAN.");
    add("Porsche", "", 1998, 2030, BusKind::Mixed, kMixed, "Ab 2003 durchgaengig CAN.");

    // --- Deutsche Hersteller ------------------------------------------------
    add("BMW", "", 1996, 2003, BusKind::KLine, kKLine, "ISO 9141-2, teils BMW-eigen.");
    add("BMW", "", 2004, 2030, BusKind::Can, kCan, "CAN.");
    add("Mini", "", 2001, 2030, BusKind::Mixed, kMixed, "Bis 2006 teils K-Line.");
    add("Mercedes-Benz", "", 1996, 2003, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Mercedes-Benz", "", 2004, 2030, BusKind::Can, kCan, "CAN.");
    add("Smart", "", 1998, 2030, BusKind::Mixed, kMixed, "Frueh K-Line, spaeter CAN.");
    add("Opel", "", 1996, 2005, BusKind::Mixed, kMixed, "Teils ISO 9141-2, teils J1850 VPW.");
    add("Opel", "", 2006, 2030, BusKind::Can, kCan, "CAN.");

    // --- Frankreich ---------------------------------------------------------
    add("Renault", "", 1996, 2004, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Renault", "", 2005, 2030, BusKind::Can, kCan, "CAN.");
    add("Dacia", "", 2004, 2030, BusKind::Mixed, kMixed, "Technik von Renault.");
    add("Peugeot", "", 1996, 2004, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Peugeot", "", 2005, 2030, BusKind::Can, kCan, "CAN.");
    add("Citroen", "", 1996, 2004, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Citroen", "", 2005, 2030, BusKind::Can, kCan, "CAN.");
    add("DS", "", 2014, 2030, BusKind::Can, kCan, "CAN.");

    // --- Italien ------------------------------------------------------------
    add("Fiat", "", 1996, 2005, BusKind::KLine, kKLine, "ISO 9141-2, teils KWP.");
    add("Fiat", "", 2006, 2030, BusKind::Can, kCan, "CAN.");
    add("Alfa Romeo", "", 1996, 2005, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Alfa Romeo", "", 2006, 2030, BusKind::Can, kCan, "CAN.");
    add("Lancia", "", 1996, 2005, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Ferrari", "", 1998, 2030, BusKind::Mixed, kMixed, "");
    add("Maserati", "", 2002, 2030, BusKind::Mixed, kMixed, "");

    // --- Schweden und Grossbritannien --------------------------------------
    add("Volvo", "", 1996, 2004, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Volvo", "", 2005, 2030, BusKind::Can, kCan, "CAN.");
    add("Saab", "", 1996, 2011, BusKind::Mixed, kMixed, "Teils J1850 VPW wie GM.");
    add("Jaguar", "", 1996, 2030, BusKind::Mixed, kMixed, "");
    add("Land Rover", "", 1996, 2030, BusKind::Mixed, kMixed, "");

    // --- Japan --------------------------------------------------------------
    add("Toyota", "", 1996, 2003, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Toyota", "", 2004, 2030, BusKind::Can, kCan, "CAN.");
    add("Lexus", "", 1998, 2030, BusKind::Mixed, kMixed, "");
    add("Honda", "", 1996, 2003, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Honda", "", 2004, 2030, BusKind::Can, kCan, "CAN.");
    add("Nissan", "", 1996, 2004, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Nissan", "", 2005, 2030, BusKind::Can, kCan, "CAN.");
    add("Mazda", "", 1996, 2004, BusKind::Mixed, kMixed, "Teils J1850 PWM wie Ford.");
    add("Mazda", "", 2005, 2030, BusKind::Can, kCan, "CAN.");
    add("Mitsubishi", "", 1996, 2005, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Mitsubishi", "", 2006, 2030, BusKind::Can, kCan, "CAN.");
    add("Subaru", "", 1996, 2005, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Subaru", "", 2006, 2030, BusKind::Can, kCan, "CAN.");
    add("Suzuki", "", 1998, 2005, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Suzuki", "", 2006, 2030, BusKind::Can, kCan, "CAN.");
    add("Daihatsu", "", 1998, 2030, BusKind::Mixed, kMixed, "");

    // --- Korea --------------------------------------------------------------
    add("Hyundai", "", 1998, 2005, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Hyundai", "", 2006, 2030, BusKind::Can, kCan, "CAN.");
    add("Kia", "", 1998, 2005, BusKind::KLine, kKLine, "ISO 9141-2.");
    add("Kia", "", 2006, 2030, BusKind::Can, kCan, "CAN.");
    add("SsangYong", "", 2000, 2030, BusKind::Mixed, kMixed, "");

    // --- USA ----------------------------------------------------------------
    add("Ford", "", 1996, 2003, BusKind::Pwm, kPwm, "SAE J1850 PWM.");
    add("Ford", "", 2004, 2030, BusKind::Can, kCan, "CAN.");
    add("Chevrolet", "", 1996, 2005, BusKind::Vpw, kVpw, "SAE J1850 VPW.");
    add("Chevrolet", "", 2006, 2030, BusKind::Can, kCan, "CAN.");
    add("Chrysler", "", 1996, 2005, BusKind::Mixed, kMixed, "");
    add("Jeep", "", 1996, 2005, BusKind::Mixed, kMixed, "");
    add("Jeep", "", 2006, 2030, BusKind::Can, kCan, "CAN.");
    add("Dodge", "", 1996, 2030, BusKind::Mixed, kMixed, "");
    add("Tesla", "", 2012, 2030, BusKind::Can, kCan, "CAN, EOBD nur eingeschraenkt.");

    // --- Sonstige -----------------------------------------------------------
    add("Iveco", "", 2000, 2030, BusKind::Mixed, kMixed, "Nutzfahrzeug.");
    add("MAN", "", 2000, 2030, BusKind::Can, kCan, "Nutzfahrzeug, teils J1939.");
    add("Anderes Fahrzeug", "", 1996, 2030, BusKind::Mixed, kAutoProtocolOrder,
        "Alle Protokolle werden der Reihe nach probiert.");
    return v;
}

}  // namespace

const std::vector<VehicleProfile>& vehicleProfiles() {
    static const std::vector<VehicleProfile> list = build();
    return list;
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

std::string busName(BusKind bus) {
    switch (bus) {
        case BusKind::KLine: return "K-Line (ISO 9141-2 / KWP2000)";
        case BusKind::Can: return "CAN (ISO 15765-4)";
        case BusKind::Pwm: return "SAE J1850 PWM";
        case BusKind::Vpw: return "SAE J1850 VPW";
        case BusKind::Mixed: return "gemischt, wird ermittelt";
    }
    return "unbekannt";
}

}  // namespace caruspex
