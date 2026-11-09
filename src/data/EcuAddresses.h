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
    // BMW style extended addressing (11 bit): every request goes to 6F1 and
    // starts with this target byte, the answer comes from 6xx and starts
    // with F1, the tester. -1 for normal addressing.
    int extendedAddress = -1;
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

// BMW and Mini from the F series on (about 2007): every diagnostic address
// 0x00 to 0x7F over the gateway, with the names of the known units. Slower
// than the list, finds units the list does not know.
std::vector<EcuDefinition> bmwAddressSweep();

// The definition for a unit by its ids, as the UI keeps them: request 6F1
// with a response 6xx means BMW style extended addressing.
EcuDefinition ecuFromIds(std::uint32_t request, std::uint32_t response, bool extended29,
                         const std::string& name);

// Text for the failure type byte of a UDS DTC (SAE J2012-DA), active
// language. Empty for 0x00 (no subtype).
std::string failureTypeText(int failureType);

}  // namespace caruspex
