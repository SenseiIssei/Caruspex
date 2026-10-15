// Vehicle profiles: which protocol is tried first, what to expect.
//
// Notes and the auto-detect entry come out in the active language (see
// core/Lang.h). The lookup functions pick the list for the language that is
// active at the time of the call; the pointers stay valid for the whole
// program run.
#pragma once

#include <string>
#include <vector>

namespace caruspex {

enum class BusKind {
    KLine,    // ISO 9141-2 / ISO 14230, older European and Japanese cars
    Can,      // ISO 15765-4, mandatory in Europe from about 2004
    Pwm,      // SAE J1850 PWM, older Ford USA
    Vpw,      // SAE J1850 VPW, older GM USA
    Mixed,    // transition period, either is possible
};

struct VehicleProfile {
    std::string brand;
    std::string model;       // empty = whole brand
    int yearFrom = 0;
    int yearTo = 0;
    BusKind bus = BusKind::Mixed;
    std::string protocolOrder;   // ELM327 protocol numbers in the order they are tried
    std::string note;
};

// All stored profiles.
const std::vector<VehicleProfile>& vehicleProfiles();

// Brands in alphabetical order, for the selection list.
std::vector<std::string> vehicleBrands();

// Profiles of one brand.
std::vector<const VehicleProfile*> profilesForBrand(const std::string& brand);

// Best profile for brand and model year, otherwise nullptr.
const VehicleProfile* findProfile(const std::string& brand, int year);

// The "brand" entry that tries all protocols ("Automatisch erkennen" /
// "Auto-detect"), in the active language. Use it to preselect that entry.
const char* autoDetectBrand();

// Order when nothing is known: fast protocols first.
extern const char* const kAutoProtocolOrder;

std::string busName(BusKind bus);

}  // namespace caruspex
