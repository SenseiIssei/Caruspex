#include "data/EcuAddresses.h"

#include <algorithm>
#include <cstddef>
#include <cstdio>

namespace caruspex {

namespace ecus::detail {
// Defined in EcuAddresses.gen.cpp with exactly this layout.
struct EcuRow {
    const char* family;
    std::uint32_t request;
    std::uint32_t response;
    bool extended29;
    int extendedAddress;   // -1, or the BMW style target byte
    EcuKind kind;
    Text name;
};
const EcuRow* ecuTable(std::size_t& count);
}  // namespace ecus::detail

namespace {

using ecus::detail::EcuRow;

// The scan order follows the order of the enum: engine, gearbox, brakes,
// airbag, steering, instruments, gateway, then the rest. Sorting by it keeps
// the CSV free of ordering rules.
int scanRank(EcuKind kind) { return static_cast<int>(kind); }

EcuDefinition fromRow(const EcuRow& row) {
    EcuDefinition d;
    d.request = row.request;
    d.response = row.response;
    d.extended29 = row.extended29;
    d.extendedAddress = row.extendedAddress;
    d.kind = row.kind;
    d.name = row.name.get();
    d.family = row.family;
    return d;
}

EcuDefinition sweepEntry(std::uint32_t request, EcuKind kind) {
    EcuDefinition d;
    d.request = request;
    d.response = request + 8;
    d.kind = kind;
    char id[8];
    std::snprintf(id, sizeof(id), "0x%03X", static_cast<unsigned>(request));
    if (kind == EcuKind::Other)
        d.name = std::string(L("Steuergerät ", "Control unit ")) + id;
    else
        d.name = std::string(ecuKindName(kind)) + " (" + id + ")";
    return d;
}

}  // namespace

const char* ecuKindName(EcuKind kind) {
    switch (kind) {
        case EcuKind::Engine:       return L("Motor", "Engine");
        case EcuKind::Gearbox:      return L("Getriebe", "Gearbox");
        case EcuKind::Brakes:       return L("Bremsen und ABS", "Brakes and ABS");
        case EcuKind::Airbag:       return L("Airbag", "Airbag");
        case EcuKind::Steering:     return L("Lenkung", "Steering");
        case EcuKind::Instruments:  return L("Kombiinstrument", "Instrument cluster");
        case EcuKind::Gateway:      return L("Gateway", "Gateway");
        case EcuKind::Body:         return L("Zentralelektronik", "Body electronics");
        case EcuKind::Climate:      return L("Klimaanlage", "Climate control");
        case EcuKind::Parking:      return L("Einparkhilfe", "Parking aid");
        case EcuKind::Lighting:     return L("Licht", "Lighting");
        case EcuKind::Doors:        return L("Türen", "Doors");
        case EcuKind::Seats:        return L("Sitze", "Seats");
        case EcuKind::Infotainment: return L("Radio und Navigation", "Radio and navigation");
        case EcuKind::Battery:      return L("Batterie", "Battery");
        case EcuKind::Charger:      return L("Ladegerät", "Charger");
        case EcuKind::Hybrid:       return L("Hybrid- und Elektroantrieb", "Hybrid and electric drive");
        case EcuKind::Other:        return L("Sonstiges", "Other");
    }
    return L("Sonstiges", "Other");
}

std::vector<EcuDefinition> ecuDefinitions(const std::string& family) {
    std::vector<EcuDefinition> out;
    if (family.empty()) return out;
    std::size_t count = 0;
    const EcuRow* rows = ecus::detail::ecuTable(count);
    for (std::size_t i = 0; i < count; ++i)
        if (family == rows[i].family) out.push_back(fromRow(rows[i]));
    // Stable, so that two units of the same kind keep their CSV order.
    std::stable_sort(out.begin(), out.end(), [](const EcuDefinition& a, const EcuDefinition& b) {
        return scanRank(a.kind) < scanRank(b.kind);
    });
    return out;
}

std::vector<std::string> ecuFamilies() {
    // The generator writes the rows grouped by family, in the order of the
    // DTC database, so first appearance gives the display order.
    std::vector<std::string> out;
    std::size_t count = 0;
    const EcuRow* rows = ecus::detail::ecuTable(count);
    for (std::size_t i = 0; i < count; ++i)
        if (std::find(out.begin(), out.end(), rows[i].family) == out.end())
            out.emplace_back(rows[i].family);
    return out;
}

std::vector<EcuDefinition> genericEcuSweep() {
    std::vector<EcuDefinition> out;
    // The OBD addresses first: by ISO 15765-4 convention 7E0 is the engine
    // and 7E1 usually the gearbox, 7E2 to 7E7 are free for other units.
    for (std::uint32_t id = 0x7E0; id <= 0x7E7; ++id)
        out.push_back(sweepEntry(id, id == 0x7E0 ? EcuKind::Engine
                                     : id == 0x7E1 ? EcuKind::Gearbox
                                                   : EcuKind::Other));
    // Then the rest of the usual range. Three groups are left out on purpose:
    // 7E0 to 7E7 were asked above already, 7DF is the OBD broadcast address
    // (a request there reaches every unit at once, which must never happen
    // with a clear command), and 7E8 to 7EF are where the OBD units answer,
    // so sending on them would look like a reply to everyone on the bus.
    for (std::uint32_t id = 0x700; id <= 0x7F7; ++id) {
        if (id >= 0x7E0 && id <= 0x7EF) continue;
        if (id == 0x7DF) continue;
        out.push_back(sweepEntry(id, EcuKind::Other));
    }
    return out;
}

}  // namespace caruspex

namespace caruspex {

std::vector<EcuDefinition> bmwAddressSweep() {
    const std::vector<EcuDefinition> known = ecuDefinitions("BMW");
    std::vector<EcuDefinition> out;
    // Known units first, in their scan order, then every other address.
    for (const EcuDefinition& d : known) out.push_back(d);
    for (int address = 0x00; address <= 0x7F; ++address) {
        const bool listed = std::any_of(known.begin(), known.end(), [address](const EcuDefinition& d) {
            return d.extendedAddress == address;
        });
        if (listed) continue;
        EcuDefinition d;
        d.request = 0x6F1;
        d.response = 0x600u + static_cast<std::uint32_t>(address);
        d.extendedAddress = address;
        d.family = "BMW";
        char id[8];
        std::snprintf(id, sizeof(id), "0x%02X", address);
        d.name = std::string(L("Steuergerät ", "Control unit ")) + id;
        out.push_back(d);
    }
    return out;
}

EcuDefinition ecuFromIds(std::uint32_t request, std::uint32_t response, bool extended29,
                         const std::string& name) {
    EcuDefinition d;
    d.request = request;
    d.response = response;
    d.extended29 = extended29;
    d.name = name;
    if (!extended29 && request == 0x6F1 && (response & 0xF00) == 0x600) {
        d.extendedAddress = static_cast<int>(response & 0xFF);
    }
    return d;
}

}  // namespace caruspex
