// Where the control units of a car answer on CAN, per brand family.
//
// Each entry is one ECU with its request and response id, as used for UDS
// (ISO 14229) through the diagnostic socket. The lists are maintained as CSV
// in data/ecus/ and turned into C++ by tools/gen_ecus.py, like the vehicle
// catalog. Only addresses with a reliable source go in; for unknown brands a
// generic sweep over the usual 11-bit range is offered instead.
#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "core/Lang.h"

namespace caruspex {

enum class EcuKind {
    Engine, Gearbox, Brakes, Airbag, Steering, Instruments, Gateway, Body, Climate,
    Parking, Lighting, Doors, Seats, Infotainment, Battery, Charger, Hybrid, Other,
};

struct EcuDefinition {
    std::uint32_t request = 0;    // e.g. 0x7E0, or 0x18DA10F1 for 29 bit
    std::uint32_t response = 0;   // e.g. 0x7E8
    bool extended29 = false;
    EcuKind kind = EcuKind::Other;
    std::string name;             // display name in the active language
    std::string family;           // "VAG", "" for the generic sweep
};

// Plain name of a kind in the active language ("Bremsen und ABS").
const char* ecuKindName(EcuKind kind);

// All ECUs known for a brand family key (see brandFamily() in
// data/DtcDatabase.h), in a sensible scan order: engine first, then the
// safety relevant ones. Empty if the family has no list.
std::vector<EcuDefinition> ecuDefinitions(const std::string& family);

// Families that have a list.
std::vector<std::string> ecuFamilies();

// The generic sweep: the OBD engine and gearbox addresses plus 0x700 to 0x7F7
// with the response at request + 8, which many makers use. Slower, and some
// ECUs will not show up, but it works without knowing the car.
std::vector<EcuDefinition> genericEcuSweep();

// Text for the failure type byte of a UDS DTC (SAE J2012-DA), active
// language. Empty for 0x00 (no subtype).
std::string failureTypeText(int failureType);

}  // namespace caruspex
