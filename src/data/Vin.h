// Vehicle identification number (VIN) decoder.
//
// A 17 character VIN after ISO 3779 consists of three parts:
//   positions 1-3   WMI, world manufacturer identifier
//   positions 4-9   VDS, vehicle descriptor section (maker specific,
//                   position 9 is the check digit in North America and China)
//   positions 10-17 VIS, vehicle identifier section (10 model year,
//                   11 plant, 12-17 serial number)
//
// Only the WMI and the overall layout are standardised worldwide. The model
// year at position 10 is mandatory in North America and China, but optional
// in Europe, and several European makers put something else there. The
// decoder therefore says how sure it is instead of pretending.
//
// Texts in VinInfo (country, region, problems, notes) come out in the
// language that is active at decode time (see core/Lang.h).
#pragma once

#include <cstddef>
#include <string>
#include <string_view>
#include <vector>

#include "core/Lang.h"

namespace caruspex {

// How a maker uses position 10.
enum class VinYearRule {
    Standard,   // model year after the norm
    Unknown,    // not known for this maker, the value is only a guess
    None,       // the maker does not put the model year there
};

// Where the maker writes its own model code into the VIN, if we know it.
enum class VinModelCodeRule {
    None,
    Vag78,        // Volkswagen group: positions 7 and 8 (8Z = Audi A2, 1J = Golf IV)
    Mercedes456,  // Mercedes-Benz and smart, European layout: positions 4 to 6 (203 = W203)
    Porsche78,    // Porsche: positions 7 and 8 (99 = 911, Y1 = Taycan)
    Tesla4,       // Tesla: position 4 is the model (S, X, 3, Y)
};

struct WmiEntry {
    const char* wmi;            // two or three characters; two means "any third character"
    const char* manufacturer;   // company behind the code
    const char* brands;         // catalog brand names, several separated by '|'
    VinYearRule yearRule;
    VinModelCodeRule codeRule;
    Text country;               // only set where the plant country differs from the code range
};

struct VinInfo {
    bool valid = false;               // 17 characters, none of them forbidden
    std::string vin;                  // normalised: upper case, no spaces or dashes
    std::string wmi;                  // positions 1-3
    std::string manufacturer;         // empty if the WMI is unknown
    std::string brand;                // catalog brand, empty if unknown or ambiguous
    std::vector<std::string> brandCandidates;   // all brands the WMI stands for
    std::string country;              // from the first two characters
    std::string region;               // continent
    bool northAmerica = false;        // built for or in North America (positions 1 = 1..5, 7)

    std::vector<int> modelYears;      // candidates, most likely first; empty if not encoded
    int modelYear = 0;                // modelYears.front(), or 0
    bool modelYearCertain = false;    // false: several candidates or the maker is unknown
    char yearCode = 0;                // position 10 as written

    bool checkDigitApplies = false;   // the check digit is mandatory for this VIN
    bool checkDigitOk = false;        // position 9 matches the calculated digit
    char checkDigit = 0;              // position 9 as written
    char expectedCheckDigit = 0;      // calculated, 0 if the VIN is not complete

    char plant = 0;                   // position 11
    std::string serial;               // positions 12-17
    std::string modelCode;            // maker specific model code, see VinModelCodeRule
    VinModelCodeRule modelCodeRule = VinModelCodeRule::None;

    std::vector<std::string> problems;   // things that are wrong with the number
    std::vector<std::string> notes;      // things worth knowing, not errors
};

// Decodes a VIN. currentYear limits the model year guess (a car cannot be
// newer than next year's model); 0 takes the system clock.
VinInfo decodeVin(std::string_view vin, int currentYear = 0);

// Calculated check digit after the North American rule ('0'-'9' or 'X'),
// 0 if the text is not 17 valid characters.
char vinCheckDigit(std::string_view vin);

// Model year for a position 10 code: 1980-2009 or the same year 30 later.
// Returns 0 for characters that are no year code (0, I, O, Q, U, Z).
int vinBaseYear(char code);

// Looks up the WMI table: three characters first, then the two character
// prefix entries. nullptr if unknown.
const WmiEntry* findWmi(std::string_view vinOrWmi);

// The whole WMI table, for statistics and tests.
const WmiEntry* wmiTable(std::size_t& count);

// Country and continent from the first two characters (ISO 3780 ranges).
std::string vinCountry(std::string_view vin);
std::string vinRegion(std::string_view vin);

}  // namespace caruspex
