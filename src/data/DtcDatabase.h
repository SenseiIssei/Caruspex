// Decode trouble codes and translate them into plain language.
//
// The database knows the generic SAE J2012 / ISO 15031-6 codes, which mean
// the same on every car, and the manufacturer-specific codes of the big
// brand families. P1xxx codes in particular mean different things per
// manufacturer, so a lookup takes the brand into account when it is known.
// For the common faults there is plain-language help: may I keep driving,
// the usual causes, what a non-mechanic can check and a rough cost class.
//
// All texts come out in the active language (see core/Lang.h).
#pragma once

#include <cstddef>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <vector>

#include "core/Lang.h"
#include "data/Severity.h"

namespace caruspex {

// The area of the car a code belongs to.
enum class DtcSystem : unsigned char {
    Unknown,
    FuelAir,               // mixture, fuel trim, fuel pressure and pump
    AirIntake,             // air flow, manifold pressure, intake temperature, intake flaps
    Throttle,              // throttle body and accelerator pedal
    Ignition,              // misfire, coils, knock, crankshaft position
    ValveTiming,           // camshaft timing, cam sensors, cylinder deactivation
    Injection,             // injectors and injection pump
    Turbo,                 // turbo and boost control
    Cooling,               // coolant, thermostat, fans
    Lubrication,           // oil pressure and oil temperature
    Exhaust,               // oxygen sensors, catalytic converter, exhaust temperature
    Egr,                   // exhaust gas recirculation and secondary air
    Evap,                  // fuel vapour system (EVAP)
    DieselAftertreatment,  // particulate filter, AdBlue/SCR, NOx, glow plugs
    Charging,              // battery and alternator
    EngineControl,         // control unit, idle, speed signal, cruise control
    Transmission,          // gearbox and clutch
    Hybrid,                // hybrid and electric drive
    Network,               // data bus and lost communication
    Brakes,                // brakes, ABS, stability control
    Steering,
    Chassis,
    Airbag,                // airbags and seat belt pretensioners
    Body,                  // body and comfort electronics
    Immobilizer,
    Climate,               // air conditioning and heating
};

// Name of the area in the active language.
const char* dtcSystemName(DtcSystem system);

// Rough repair cost including parts and labour at a workshop.
enum class CostClass : unsigned char {
    Unknown,    // the likely causes are too far apart to say
    Cheap,      // below about 200 Euro
    Medium,     // about 200 to 800 Euro
    Expensive,  // above about 800 Euro
};

// "günstig" / "cheap" and so on, empty for Unknown.
const char* costClassLabel(CostClass cost);

// One sentence with the rough price band, empty for Unknown.
const char* costClassHint(CostClass cost);

// General "may I keep driving?" advice for a severity, used when a code has
// no specific advice of its own.
const char* severityAdvice(Severity severity);

// Everything the database knows about one code, in the active language.
struct DtcInfo {
    std::string code;          // normalised, for example "P0301"
    std::string text;          // description, or the group meaning when not known
    Severity severity = Severity::Medium;
    DtcSystem system = DtcSystem::Unknown;
    std::string systemName;    // dtcSystemName(system)
    std::string advice;        // may I keep driving?
    std::vector<std::string> causes;   // most likely first; empty when not known
    std::vector<std::string> checks;   // things a non-mechanic can look at
    CostClass costClass = CostClass::Unknown;
    bool manufacturerSpecific = false;  // the code lies in a manufacturer range
    bool known = false;        // the exact code is in the database
    bool hasDetails = false;   // causes and checks come from the database
    bool throttleRelated = false;
    std::string family;        // brand family that supplied the meaning, empty for SAE
    std::string familyLabel;   // "Ford", "Toyota/Lexus", ...
};

// Full lookup. brand is a plain brand name like "Audi", "BMW",
// "Mercedes-Benz" or "Opel"; a family key such as "VAG" or "PSA" works too.
// With an empty or unknown brand only the generic SAE meanings are used.
// The brand's own table is consulted first, then the generic one, then the
// meaning of the code's number range.
DtcInfo lookupDtc(const std::string& code, const std::string& brand = "");

// Brand family key for a brand name ("Skoda" -> "VAG", "Lexus" ->
// "TOYOTA"), empty if the brand belongs to no known family. Case, spaces,
// hyphens and accents do not matter.
//
// Opel and Vauxhall map to "OPEL", which only knows the generic codes,
// because the manufacturer codes of their older GM engines and their newer
// PSA engines differ. Pass "PSA" for models on a PSA platform.
std::string brandFamily(const std::string& brand);

// Display label of a family key, for example "Hyundai/Kia". Empty for an
// unknown key; "SAE/ISO" style label for the empty key.
std::string brandFamilyLabel(const std::string& family);

// All family keys in display order.
const std::vector<std::string>& brandFamilies();

// Raw bytes -> code like "P0301". Empty string for 0x00 0x00 (no fault).
std::string decodeDtc(int byteA, int byteB);

// Plain-language text in the active language. Without a brand this knows
// the generic and the VAG meanings, as it always did; unknown codes fall
// back to the meaning of the code's group.
std::string describeDtc(const std::string& code);

// Same, with the meaning for the given brand (see lookupDtc).
std::string describeDtc(const std::string& code, const std::string& brand);

// Does the code concern the throttle body?
bool isThrottleDtc(const std::string& code);

// All generic and VAG codes with their description in the active language.
const std::unordered_map<std::string, std::string>& dtcDescriptions();

// All generic and VAG codes with their description in both languages.
const std::unordered_map<std::string, Text>& dtcTexts();

const std::unordered_set<std::string>& dtcThrottleCodes();

// One line of the offline lexicon.
struct DtcLexiconEntry {
    std::string code;
    std::string family;       // empty for the generic SAE codes
    std::string familyLabel;  // "SAE/ISO (alle Marken)" or the brand family
    std::string text;
    Severity severity = Severity::Medium;
    DtcSystem system = DtcSystem::Unknown;
};

// Every code in the database in the active language: first the generic
// ones, then each brand family, each part sorted by code. The same code can
// appear several times with different families.
std::vector<DtcLexiconEntry> dtcLexicon();

// Number of codes per family key ("" for generic).
std::size_t dtcCount(const std::string& family = "");

}  // namespace caruspex
