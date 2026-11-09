#include "core/BleProfile.h"

#include <algorithm>
#include <cctype>

namespace caruspex {
namespace {

constexpr const char* kBaseSuffix = "-0000-1000-8000-00805f9b34fb";

bool isHex(const std::string& s) {
    return !s.empty() && std::all_of(s.begin(), s.end(), [](unsigned char c) { return std::isxdigit(c); });
}

std::string shortUuid(const char* hex16) {
    return std::string("0000") + hex16 + kBaseSuffix;
}

struct KnownProfile {
    const char* name;
    std::string service;
    std::string notify;   // empty: any notifying characteristic of the service
    std::string write;    // empty: any writable characteristic of the service
};

// Collected from adapters on the market and the apps that talk to them.
const std::vector<KnownProfile>& knownProfiles() {
    static const std::vector<KnownProfile> list = {
        // Most ELM327 BLE clones, Vgate iCar Pro BLE, Veepeak OBDCheck BLE
        {"FFF0", shortUuid("fff0"), shortUuid("fff1"), shortUuid("fff2")},
        // Vgate vLinker MC+, FS and BM+ in BLE mode
        {"vLinker", shortUuid("18f0"), shortUuid("2af0"), shortUuid("2af1")},
        // HM-10 style modules, one characteristic for both directions
        {"FFE0", shortUuid("ffe0"), shortUuid("ffe1"), shortUuid("ffe1")},
        // OBDLink CX, LELink and others with this vendor service
        {"E781", "e7810a71-73ae-499d-8c15-faa9aef0c3f2", "bef8d6c9-9c21-4c9e-b632-bd58c1009f9f",
         "bef8d6c9-9c21-4c9e-b632-bd58c1009f9f"},
        // Nordic UART service
        {"NUS", "6e400001-b5a3-f393-e0a9-e50e24dcca9e", "6e400003-b5a3-f393-e0a9-e50e24dcca9e",
         "6e400002-b5a3-f393-e0a9-e50e24dcca9e"},
    };
    return list;
}

bool isStandardService(const std::string& uuid) {
    for (const char* s : {"1800", "1801", "180a", "180f", "1805", "180d", "fe59"}) {
        if (uuid == shortUuid(s)) return true;
    }
    return false;
}

bool canNotify(const BleCharacteristicDesc& c) { return c.notify || c.indicate; }
bool canWrite(const BleCharacteristicDesc& c) { return c.write || c.writeNoResponse; }

// Picks the characteristics inside one service; the wanted UUIDs win, any
// suitable one is the fallback.
BleChannel pickInService(const BleServiceDesc& service, int index, const std::string& notifyUuid,
                         const std::string& writeUuid) {
    BleChannel ch;
    ch.service = index;
    const auto& chars = service.characteristics;
    for (int i = 0; i < static_cast<int>(chars.size()); ++i) {
        const std::string u = normalizeBleUuid(chars[i].uuid);
        if (ch.notifyCharacteristic < 0 && canNotify(chars[i]) && !notifyUuid.empty() && u == notifyUuid) {
            ch.notifyCharacteristic = i;
        }
        if (ch.writeCharacteristic < 0 && canWrite(chars[i]) && !writeUuid.empty() && u == writeUuid) {
            ch.writeCharacteristic = i;
        }
    }
    for (int i = 0; i < static_cast<int>(chars.size()); ++i) {
        if (ch.notifyCharacteristic < 0 && canNotify(chars[i])) ch.notifyCharacteristic = i;
    }
    // A separate write characteristic is preferred over writing to the
    // notifying one, unless that is the only choice.
    for (int i = 0; i < static_cast<int>(chars.size()); ++i) {
        if (ch.writeCharacteristic < 0 && canWrite(chars[i]) && i != ch.notifyCharacteristic) {
            ch.writeCharacteristic = i;
        }
    }
    if (ch.writeCharacteristic < 0 && ch.notifyCharacteristic >= 0 &&
        canWrite(chars[ch.notifyCharacteristic])) {
        ch.writeCharacteristic = ch.notifyCharacteristic;
    }
    if (ch.writeCharacteristic >= 0) {
        const BleCharacteristicDesc& w = chars[ch.writeCharacteristic];
        ch.writeWithoutResponse = w.writeNoResponse;
    }
    return ch;
}

}  // namespace

std::string normalizeBleUuid(const std::string& uuid) {
    std::string s;
    for (char c : uuid) {
        if (c == '{' || c == '}' || std::isspace(static_cast<unsigned char>(c))) continue;
        s += static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    }
    if (s.rfind("0x", 0) == 0) s = s.substr(2);
    if (s.size() == 4 && isHex(s)) return "0000" + s + kBaseSuffix;
    if (s.size() == 8 && isHex(s)) return s + kBaseSuffix;
    if (s.size() == 36 && s[8] == '-' && s[13] == '-' && s[18] == '-' && s[23] == '-') {
        std::string digits = s;
        digits.erase(std::remove(digits.begin(), digits.end(), '-'), digits.end());
        if (digits.size() == 32 && isHex(digits)) return s;
    }
    return {};
}

BleChannel chooseBleChannel(const std::vector<BleServiceDesc>& services) {
    for (const KnownProfile& profile : knownProfiles()) {
        for (int i = 0; i < static_cast<int>(services.size()); ++i) {
            if (normalizeBleUuid(services[i].uuid) != profile.service) continue;
            BleChannel ch = pickInService(services[i], i, profile.notify, profile.write);
            if (ch.valid()) {
                ch.profile = profile.name;
                return ch;
            }
        }
    }
    for (int i = 0; i < static_cast<int>(services.size()); ++i) {
        const std::string u = normalizeBleUuid(services[i].uuid);
        if (u.empty() || isStandardService(u)) continue;
        BleChannel ch = pickInService(services[i], i, {}, {});
        if (ch.valid()) {
            ch.profile = "generic";
            return ch;
        }
    }
    return {};
}

int obdAdapterNameScore(const std::string& name) {
    std::string n;
    for (char c : name) n += static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    if (n.empty()) return 0;
    // Strong hints first: names that only OBD adapters use.
    for (const char* strong : {"obd", "elm327", "elm 327", "vlink", "v-link", "vgate", "icar",
                               "obdlink", "veepeak", "konnwei", "kiwi", "lelink", "carista",
                               "vlinker", "scan tool", "obdii"}) {
        if (n.find(strong) != std::string::npos) return 3;
    }
    for (const char* weak : {"elm", "car", "diag", "can", "ble", "spp"}) {
        if (n.find(weak) != std::string::npos) return 1;
    }
    return 0;
}

}  // namespace caruspex
