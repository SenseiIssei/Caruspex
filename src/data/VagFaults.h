// VAG fault numbers in plain text.
//
// The manufacturer-specific diagnostics deliver five-digit numbers instead of
// P-codes. From 16384 upwards these are converted EOBD codes: the number minus
// 16384 gives the digits of the P-code. That way the existing fault database
// can be reused. Below that lie the old VAG-specific numbers.
//
// Important: these old numbers are defined per control unit. 01276 means
// "brake light switch" in the engine control unit, but "hydraulic pump" in
// the ABS control unit. A single flat list would therefore simply be wrong.
// That is why the plain text is always looked up together with the control
// unit address. If the address is unknown, the engine control unit is
// assumed and a match from another area is labelled as such.
//
// The texts come out in the active language (see core/Lang.h).
#pragma once

#include <cstddef>
#include <cstdint>
#include <string>

namespace caruspex {

// Functional area of a control unit. Several addresses can share an area,
// for example all door control units belong to the comfort system.
enum class FaultArea {
    Engine,
    Transmission,
    Brakes,
    Airbag,
    Cluster,
    Comfort,
    Immobilizer,
    Steering,
    Unknown,
};

// Address (0x01 engine, 0x03 ABS, ...) to functional area.
FaultArea areaForEcuAddress(int address);

// Conversion to the P-code, empty if the number is too small for that.
std::string vagCodeToPCode(int vagCode);

// Plain text for a VAG fault number. ecuAddress is the address of the
// control unit that reported the fault; -1 if unknown.
std::string describeVagFault(int vagCode, int ecuAddress = -1);

// How many old VAG numbers are stored in total.
std::size_t vagFaultCount();

}  // namespace caruspex
