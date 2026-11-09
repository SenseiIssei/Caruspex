#include <catch2/catch_test_macros.hpp>

#include "core/BleProfile.h"

using namespace caruspex;

namespace {

BleCharacteristicDesc chr(const std::string& uuid, bool notify, bool write, bool writeNoResp = false) {
    BleCharacteristicDesc c;
    c.uuid = uuid;
    c.notify = notify;
    c.write = write;
    c.writeNoResponse = writeNoResp;
    return c;
}

BleServiceDesc deviceInfo() {
    return {"0x180A", {chr("2a29", false, false), chr("2a24", false, false)}};
}

}  // namespace

TEST_CASE("BLE UUIDs are normalised", "[ble]") {
    CHECK(normalizeBleUuid("fff0") == "0000fff0-0000-1000-8000-00805f9b34fb");
    CHECK(normalizeBleUuid("0xFFF0") == "0000fff0-0000-1000-8000-00805f9b34fb");
    CHECK(normalizeBleUuid("{0000FFF0-0000-1000-8000-00805F9B34FB}") ==
          "0000fff0-0000-1000-8000-00805f9b34fb");
    CHECK(normalizeBleUuid("E7810A71-73AE-499D-8C15-FAA9AEF0C3F2") ==
          "e7810a71-73ae-499d-8c15-faa9aef0c3f2");
    CHECK(normalizeBleUuid("nonsense").empty());
    CHECK(normalizeBleUuid("").empty());
}

TEST_CASE("The common FFF0 adapter uses FFF1 for answers and FFF2 for commands", "[ble]") {
    const std::vector<BleServiceDesc> services = {
        deviceInfo(),
        {"fff0", {chr("fff1", true, false), chr("fff2", false, true, true)}},
    };
    const BleChannel ch = chooseBleChannel(services);
    REQUIRE(ch.valid());
    CHECK(ch.profile == "FFF0");
    CHECK(ch.service == 1);
    CHECK(ch.notifyCharacteristic == 0);
    CHECK(ch.writeCharacteristic == 1);
    CHECK(ch.writeWithoutResponse);
}

TEST_CASE("vLinker and HM-10 style adapters are recognised", "[ble]") {
    const std::vector<BleServiceDesc> vlinker = {
        {"18f0", {chr("2af1", false, true), chr("2af0", true, false)}},
    };
    BleChannel ch = chooseBleChannel(vlinker);
    REQUIRE(ch.valid());
    CHECK(ch.profile == "vLinker");
    CHECK(ch.notifyCharacteristic == 1);
    CHECK(ch.writeCharacteristic == 0);
    CHECK_FALSE(ch.writeWithoutResponse);

    // One characteristic for both directions.
    const std::vector<BleServiceDesc> hm10 = {{"ffe0", {chr("ffe1", true, true, true)}}};
    ch = chooseBleChannel(hm10);
    REQUIRE(ch.valid());
    CHECK(ch.profile == "FFE0");
    CHECK(ch.notifyCharacteristic == 0);
    CHECK(ch.writeCharacteristic == 0);
}

TEST_CASE("A known profile wins over an unknown vendor service", "[ble]") {
    const std::vector<BleServiceDesc> services = {
        {"12345678-1234-1234-1234-123456789abc", {chr("aaaa", true, true)}},
        {"6e400001-b5a3-f393-e0a9-e50e24dcca9e",
         {chr("6e400002-b5a3-f393-e0a9-e50e24dcca9e", false, true),
          chr("6e400003-b5a3-f393-e0a9-e50e24dcca9e", true, false)}},
    };
    const BleChannel ch = chooseBleChannel(services);
    REQUIRE(ch.valid());
    CHECK(ch.profile == "NUS");
    CHECK(ch.service == 1);
    CHECK(ch.notifyCharacteristic == 1);
    CHECK(ch.writeCharacteristic == 0);
}

TEST_CASE("Unknown vendor services work as long as they can notify and be written", "[ble]") {
    const std::vector<BleServiceDesc> services = {
        deviceInfo(),
        {"0x1801", {chr("2a05", true, false)}},
        {"abcdef01-0000-4000-8000-000000000001",
         {chr("abcdef02-0000-4000-8000-000000000001", true, false),
          chr("abcdef03-0000-4000-8000-000000000001", false, false, true)}},
    };
    const BleChannel ch = chooseBleChannel(services);
    REQUIRE(ch.valid());
    CHECK(ch.profile == "generic");
    CHECK(ch.service == 2);
    CHECK(ch.writeWithoutResponse);
}

TEST_CASE("Devices with only standard services give no channel", "[ble]") {
    CHECK_FALSE(chooseBleChannel({deviceInfo(), {"1800", {chr("2a00", false, true)}}}).valid());
    CHECK_FALSE(chooseBleChannel({}).valid());
    // A known service without a notifying characteristic is not usable.
    CHECK_FALSE(chooseBleChannel({{"fff0", {chr("fff2", false, true)}}}).valid());
}

TEST_CASE("Adapter names are ranked", "[ble]") {
    CHECK(obdAdapterNameScore("OBDII") == 3);
    CHECK(obdAdapterNameScore("IOS-Vlink") == 3);
    CHECK(obdAdapterNameScore("vLinker MC-IOS") == 3);
    CHECK(obdAdapterNameScore("OBDLink CX") == 3);
    CHECK(obdAdapterNameScore("Android-Vlink") == 3);
    CHECK(obdAdapterNameScore("V-LINK") == 3);
    CHECK(obdAdapterNameScore("Veepeak") == 3);
    CHECK(obdAdapterNameScore("BLE Device") == 1);
    CHECK(obdAdapterNameScore("Jakobs Kopfhörer") == 0);
    CHECK(obdAdapterNameScore("") == 0);
}
