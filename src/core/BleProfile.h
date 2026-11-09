// Bluetooth LE adapters carry the ELM327 text protocol over two GATT
// characteristics: one the computer writes to, one that notifies the answers.
// Makers use different services for it. This picks the pair from what a
// device offers, without any Bluetooth library, so it can be tested.
#pragma once

#include <string>
#include <vector>

namespace caruspex {

struct BleCharacteristicDesc {
    std::string uuid;              // any form: "fff1", "0xFFF1", "{0000fff1-...}"
    bool notify = false;
    bool indicate = false;
    bool write = false;
    bool writeNoResponse = false;
};

struct BleServiceDesc {
    std::string uuid;
    std::vector<BleCharacteristicDesc> characteristics;
};

struct BleChannel {
    int service = -1;              // index into the list that was passed in
    int notifyCharacteristic = -1;
    int writeCharacteristic = -1;
    bool writeWithoutResponse = false;
    std::string profile;           // "FFF0", "FFE0", "vLinker", ... or "generic"

    bool valid() const { return service >= 0 && notifyCharacteristic >= 0 && writeCharacteristic >= 0; }
};

// Full lower-case 128-bit form without braces; 16- and 32-bit short forms
// are expanded with the Bluetooth base UUID. Empty for garbage.
std::string normalizeBleUuid(const std::string& uuid);

// The best serial channel on the device: known adapter profiles first, then
// any vendor service with a notify and a write characteristic. Standard
// services (generic access, device information, battery) are never used.
BleChannel chooseBleChannel(const std::vector<BleServiceDesc>& services);

// How much a device name looks like an OBD adapter, 0 = not at all. Used to
// sort the scan list and to pick the device when searching automatically.
int obdAdapterNameScore(const std::string& name);

}  // namespace caruspex
