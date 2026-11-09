// Which control unit is behind a response address.
//
// With headers switched on (Elm327::setHeaders) every OBD response carries
// the address of its sender. OBD itself does not say what is behind an
// address, but the conventions are firm enough for a name:
//
//   CAN 11 bit   7E8 is the engine, 7E9 the transmission, 7EA to 7EF others
//   CAN 29 bit   18DAF1xx, xx = 10 engine, 18 transmission (as below)
//   K-line/J1850 source byte 10 to 17 engine, 18 to 1F transmission
//
// The name from mode 09 (InfoType 0x0A) is more precise and wins when the
// control unit provides it.
#pragma once

#include <string>

namespace caruspex {

enum class EcuRole { Unknown, Engine, Transmission, Other };

// Role by address. Unknown for an empty source (headers off).
EcuRole ecuRole(const std::string& source);

// Plain name of the role in the active language: "Motor", "Getriebe",
// "Steuergerät". Empty for Unknown.
std::string ecuRoleName(EcuRole role);

// Name with the address in brackets, e.g. "Motor (7E8)" or
// "Getriebe (18DAF118)". Empty for an empty source.
std::string ecuDisplayName(const std::string& source);

}  // namespace caruspex
