#include "core/EcuAddress.h"

#include <cctype>

#include "core/Lang.h"

namespace caruspex {
namespace {

bool allHex(const std::string& s) {
    if (s.empty()) return false;
    for (char c : s) {
        if (!std::isxdigit(static_cast<unsigned char>(c))) return false;
    }
    return true;
}

// Source byte on K-line and J1850 (SAE J2178), also the low byte of a
// 29-bit CAN identifier.
EcuRole roleForSourceByte(int address) {
    if (address >= 0x10 && address <= 0x17) return EcuRole::Engine;
    if (address >= 0x18 && address <= 0x1F) return EcuRole::Transmission;
    return EcuRole::Other;
}

}  // namespace

EcuRole ecuRole(const std::string& source) {
    if (!allHex(source)) return EcuRole::Unknown;
    if (source.size() > 8) return EcuRole::Other;
    const unsigned long long value = std::stoull(source, nullptr, 16);
    switch (source.size()) {
        case 3:  // CAN 11 bit
            if (value == 0x7E8) return EcuRole::Engine;
            if (value == 0x7E9) return EcuRole::Transmission;
            return EcuRole::Other;
        case 8:  // CAN 29 bit, 18DAF1xx
            return roleForSourceByte(static_cast<int>(value & 0xFF));
        case 2:  // K-line and J1850 source byte
            return roleForSourceByte(static_cast<int>(value));
        default:
            return EcuRole::Other;
    }
}

std::string ecuRoleName(EcuRole role) {
    switch (role) {
        case EcuRole::Engine: return L("Motor", "Engine");
        case EcuRole::Transmission: return L("Getriebe", "Transmission");
        case EcuRole::Other: return L("Steuergerät", "Control unit");
        case EcuRole::Unknown: break;
    }
    return {};
}

std::string ecuDisplayName(const std::string& source) {
    if (source.empty()) return {};
    const EcuRole role = ecuRole(source);
    if (role == EcuRole::Unknown) return source;
    return ecuRoleName(role) + " (" + source + ")";
}

}  // namespace caruspex
