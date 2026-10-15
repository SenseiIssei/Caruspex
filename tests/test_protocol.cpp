// Tests for the parts that can be checked without a car: parsers and databases.

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include <cctype>
#include <memory>

#include "core/Elm327.h"
#include "core/Kwp1281.h"
#include "core/Kwp2000.h"
#include "core/Lang.h"
#include "core/Obd2.h"
#include "core/Transport.h"
#include "data/DtcDatabase.h"
#include "data/DtcLocation.h"
#include "data/MonitorTests.h"
#include "data/Pids.h"
#include "data/VagFaults.h"
#include "data/Vehicles.h"

using namespace caruspex;

TEST_CASE("Hex lines become bytes", "[parser]") {
    REQUIRE(hexToBytes("4100BE1FA813") ==
            std::vector<int>{0x41, 0x00, 0xBE, 0x1F, 0xA8, 0x13});
    REQUIRE(hexToBytes("41 00 BE") == std::vector<int>{0x41, 0x00, 0xBE});
    REQUIRE(hexToBytes("").empty());
    // Odd number of digits: the last half byte is dropped.
    REQUIRE(hexToBytes("4100B") == std::vector<int>{0x41, 0x00});
}

TEST_CASE("Responses to service 03 are recognized", "[parser]") {
    const ElmResult result = decodeMessages({"430507155900 00"}, 0x03);
    REQUIRE(result.ok());
    REQUIRE(result.messages.size() == 1);
    REQUIRE(result.messages[0][0] == 0x05);
    REQUIRE(result.messages[0][1] == 0x07);
}

TEST_CASE("Multi-line CAN responses are assembled", "[parser]") {
    const ElmResult result = decodeMessages({"014", "0:4902014741", "1:554E45"}, 0x09);
    REQUIRE(result.ok());
}

TEST_CASE("Adapter messages are reported as errors", "[parser]") {
    REQUIRE(decodeMessages({"NO DATA"}, 0x01).status == ElmStatus::NoData);
    REQUIRE(decodeMessages({"UNABLE TO CONNECT"}, 0x01).status == ElmStatus::BusError);
    REQUIRE(decodeMessages({"BUS INIT: ERROR"}, 0x01).status == ElmStatus::BusError);
    REQUIRE(decodeMessages({"7F0312"}, 0x03).status == ElmStatus::Negative);
}

TEST_CASE("Trouble codes are decoded correctly", "[dtc]") {
    REQUIRE(decodeDtc(0x01, 0x33) == "P0133");
    REQUIRE(decodeDtc(0x43, 0x21) == "C0321");
    REQUIRE(decodeDtc(0x81, 0x23) == "B0123");
    REQUIRE(decodeDtc(0xC1, 0x00) == "U0100");
    REQUIRE(decodeDtc(0x05, 0x07) == "P0507");
    // 0x0000 means: no fault in this slot.
    REQUIRE(decodeDtc(0x00, 0x00).empty());
}

TEST_CASE("The fault database knows the important codes", "[dtc]") {
    REQUIRE(dtcDescriptions().size() > 300);
    REQUIRE_FALSE(describeDtc("P0301").empty());
    REQUIRE_FALSE(describeDtc("P1559").empty());
    REQUIRE_FALSE(describeDtc("U0121").empty());
    // Unknown codes fall back to the group meaning instead of staying empty.
    REQUIRE_FALSE(describeDtc("P0999").empty());
    REQUIRE(isThrottleDtc("P0507"));
    REQUIRE_FALSE(isThrottleDtc("P0301"));
}

TEST_CASE("Live values convert correctly", "[pids]") {
    const PidSpec* rpm = findPid(0x0C);
    REQUIRE(rpm != nullptr);
    REQUIRE(rpm->calc({0x0F, 0xA0}) == 1000.0);   // (0x0FA0) / 4

    const PidSpec* coolant = findPid(0x05);
    REQUIRE(coolant->calc({0x5A}) == 50.0);       // 0x5A - 40

    const PidSpec* voltage = findPid(0x42);
    REQUIRE(voltage->calc({0x36, 0xB0}) == 14.0); // 14000 mV
}

TEST_CASE("Vehicle profiles hit the right era", "[vehicles]") {
    const VehicleProfile* a2 = findProfile("Audi", 2002);
    REQUIRE(a2 != nullptr);
    REQUIRE(a2->bus == BusKind::KLine);

    const VehicleProfile* b7 = findProfile("Audi", 2007);
    REQUIRE(b7 != nullptr);
    REQUIRE(b7->bus == BusKind::Can);

    REQUIRE(vehicleBrands().size() > 25);
}

TEST_CASE("Trouble codes point to the right spot on the vehicle", "[location]") {
    REQUIRE(locateDtc("P0507").zone == CarZone::Throttle);
    REQUIRE(locateDtc("P1559").zone == CarZone::Throttle);   // not the turbo
    REQUIRE(locateDtc("P1555").zone == CarZone::Turbo);
    REQUIRE(locateDtc("P0301").zone == CarZone::Cylinders);
    REQUIRE(locateDtc("P0420").zone == CarZone::Exhaust);
    REQUIRE(locateDtc("P0442").zone == CarZone::FuelTank);
    REQUIRE(locateDtc("P0118").zone == CarZone::Cooling);
    REQUIRE(locateDtc("C0035").zone == CarZone::Wheels);
    REQUIRE(locateDtc("B0001").zone == CarZone::Cabin);
    REQUIRE(locateDtc("U0121").zone == CarZone::Network);
    REQUIRE(locateDtc("P0700").zone == CarZone::Gearbox);
    // Every zone comes with a hint where to look.
    REQUIRE_FALSE(locateDtc("P0507").hint.empty());
    REQUIRE_FALSE(locateDtc("P0507").name.empty());
}

TEST_CASE("VAG numbers are translated into P-codes", "[vag]") {
    // From 16384 upwards these are converted EOBD codes.
    REQUIRE(vagCodeToPCode(16486) == "P0102");
    REQUIRE(vagCodeToPCode(16384) == "P0000");
    REQUIRE(vagCodeToPCode(17747) == "P1363");
    // Below that there is no P-code.
    REQUIRE(vagCodeToPCode(561).empty());
    REQUIRE(vagCodeToPCode(0).empty());

    REQUIRE(describeVagFault(16486).find("Luftmassenmesser") != std::string::npos);
    REQUIRE(describeVagFault(522).find("Kuehlmitteltemperaturgeber") != std::string::npos);
    REQUIRE(describeVagFault(1249).find("Zylinder 1") != std::string::npos);
    // Unknown numbers do not fall over.
    REQUIRE_FALSE(describeVagFault(9999).empty());
    REQUIRE(vagFaultCount() > 60);
}

TEST_CASE("The same VAG number means something different per control unit", "[vag]") {
    // This is the crux: 01276 is the brake light switch in the engine, the
    // hydraulic pump in the ABS. Without the address one of the two answers
    // would be wrong.
    REQUIRE(describeVagFault(1276, 0x01).find("Bremslichtschalter") != std::string::npos);
    REQUIRE(describeVagFault(1276, 0x03).find("Hydraulikpumpe") != std::string::npos);

    // And 01177: in the engine the control unit adaptation, in the
    // immobilizer the missing adaptation to the engine.
    REQUIRE(describeVagFault(1177, 0x01).find("Anpassung") != std::string::npos);
    REQUIRE(describeVagFault(1177, 0x25).find("Motorsteuergeraet") != std::string::npos);

    // Wheel speed sensors only exist in the brake control unit.
    REQUIRE(describeVagFault(283, 0x03).find("vorne links") != std::string::npos);
    REQUIRE(describeVagFault(290, 0x03).find("hinten links") != std::string::npos);

    // Likewise, airbag igniters only exist there.
    REQUIRE(describeVagFault(588, 0x15).find("Fahrerairbag") != std::string::npos);
    REQUIRE(describeVagFault(1221, 0x15).find("Crashsensor") != std::string::npos);

    // Door control units belong to the comfort area, even under their own address.
    REQUIRE(areaForEcuAddress(0x42) == FaultArea::Comfort);
    REQUIRE(describeVagFault(1331, 0x42).find("Tuersteuergeraet") != std::string::npos);

    // Power supply applies everywhere, no matter which control unit reports it.
    REQUIRE(describeVagFault(532, 0x01).find("Klemme 30") != std::string::npos);
    REQUIRE(describeVagFault(532, 0x15).find("Klemme 30") != std::string::npos);
    REQUIRE(describeVagFault(65535, 0x46).find("Steuergeraetefehler") != std::string::npos);

    // Without an address it guesses, but labels the match as a guess.
    REQUIRE(describeVagFault(283, -1).find("vermutlich") != std::string::npos);
    // Engine codes still stay unambiguous without an address because the
    // engine area is searched first.
    REQUIRE(describeVagFault(1249, -1).find("vermutlich") != std::string::npos);
}

TEST_CASE("The live value set covers the standard", "[pids]") {
    REQUIRE(pidTable().size() >= 70);
    // Samples across the ranges.
    for (int pid : {0x03, 0x04, 0x05, 0x0C, 0x10, 0x11, 0x14, 0x1C, 0x24,
                    0x2C, 0x33, 0x3C, 0x42, 0x45, 0x51, 0x5C, 0x5E, 0x63}) {
        INFO("PID 0x" << std::hex << pid);
        REQUIRE(findPid(pid) != nullptr);
    }
}

TEST_CASE("The newer live values compute correctly", "[pids]") {
    // Catalyst temperature: (256A+B)/10 - 40
    const PidSpec* cat = findPid(0x3C);
    REQUIRE(cat->unit == std::string("C"));
    REQUIRE(cat->calc({0x13, 0x88}) == Catch::Approx(5000 / 10.0 - 40.0));

    // Wideband O2 sensor: lambda = (256A+B) * 2 / 65536
    const PidSpec* wide = findPid(0x24);
    REQUIRE(wide->calc({0x80, 0x00, 0x00, 0x00}) == Catch::Approx(1.0));

    // Reference torque is a plain 16-bit number.
    REQUIRE(findPid(0x63)->calc({0x01, 0x00}) == Catch::Approx(256.0));

    // Injection timing: ((256A+B) - 26880) / 128
    REQUIRE(findPid(0x5D)->calc({0x69, 0x00}) == Catch::Approx((26880 - 26880) / 128.0));

    // Fuel trim bank 2 behaves like bank 1.
    REQUIRE(findPid(0x08)->calc({128}) == Catch::Approx(0.0));
    REQUIRE(findPid(0x09)->calc({0}) == Catch::Approx(-100.0));
}

TEST_CASE("Status values come as text, not as a number", "[pids]") {
    const PidSpec* fuel = findPid(0x03);
    REQUIRE(fuel->text);
    REQUIRE(fuel->text(2).find("geschlossener Regelkreis") != std::string::npos);

    const PidSpec* type = findPid(0x51);
    REQUIRE(type->text);
    REQUIRE(type->text(4) == "Diesel");
    REQUIRE(type->text(1) == "Benzin");

    const PidSpec* norm = findPid(0x1C);
    REQUIRE(norm->text(6).find("EOBD") != std::string::npos);
}

TEST_CASE("Component monitoring converts correctly", "[monitor]") {
    std::string unit;
    // ID 0x0B: voltage in millivolts.
    REQUIRE(monitorScale(0x0B, 1000, unit) == Catch::Approx(1.0));
    REQUIRE(unit == "V");
    // ID 0x24: temperature, a tenth of a degree with offset.
    REQUIRE(monitorScale(0x24, 800, unit) == Catch::Approx(40.0));
    REQUIRE(unit == "Grad C");
    // ID 0x1E: engine speed in quarters.
    REQUIRE(monitorScale(0x1E, 4000, unit) == Catch::Approx(1000.0));
    REQUIRE(unit == "1/min");
    // An unknown ID returns the raw value instead of nonsense.
    REQUIRE(monitorScale(0xFE, 1234, unit) == Catch::Approx(1234.0));

    REQUIRE_FALSE(monitorTestName(0x21).empty());
    REQUIRE(monitorComponentName(0x21).find("Katalysator") != std::string::npos);
    // Unknown entries stay named instead of empty.
    REQUIRE_FALSE(monitorTestName(0xF3).empty());
    REQUIRE_FALSE(monitorComponentName(0xF3).empty());
}

TEST_CASE("Oxygen sensor test converts correctly", "[monitor]") {
    std::string unit;
    // Voltage tests: five millivolts per step.
    REQUIRE(oxygenScale(0x01, 100, unit) == Catch::Approx(0.5));
    REQUIRE(unit == "V");
    // Time tests: four milliseconds per step.
    REQUIRE(oxygenScale(0x05, 250, unit) == Catch::Approx(1.0));
    REQUIRE(unit == "s");
    REQUIRE(oxygenTestName(0x01).find("mager nach fett") != std::string::npos);
    REQUIRE_FALSE(oxygenTestName(0x7F).empty());
}

namespace {

// Switches the core to English for one test and back to German afterwards,
// even if a check fails, so the other tests keep seeing the default.
struct EnglishScope {
    EnglishScope() { setLanguage(Language::English); }
    ~EnglishScope() { setLanguage(Language::German); }
};

// Just enough of an ELM327 to connect and read faults: P0507 stored (mode 03)
// and P0121 pending (mode 07).
class FaultTransport : public ITransport {
public:
    bool write(const std::string& data) override {
        std::string cmd;
        for (char c : data) {
            if (c == '\r' || c == '\n' || c == ' ') continue;
            cmd += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
        }
        if (cmd == "ATZ") pending_ = "ELM327 v1.5\r\r>";
        else if (cmd.rfind("AT", 0) == 0) pending_ = "OK\r\r>";
        else if (cmd == "0100") pending_ = "41 00 BE 1F A8 13\r\r>";
        else if (cmd == "03") pending_ = "43 05 07 00 00 00 00\r\r>";
        else if (cmd == "07") pending_ = "47 01 21 00 00 00 00\r\r>";
        else if (cmd == "050101") pending_ = "45 01 01 8C\r\r>";
        else if (cmd == "0601") pending_ = "46 01 01 00 96 00 50 00 C8\r\r>";
        else pending_ = "NO DATA\r\r>";
        return true;
    }
    std::string read(int) override {
        std::string out;
        out.swap(pending_);
        return out;
    }
    void flushInput() override { pending_.clear(); }
    std::string name() const override { return "Test"; }

private:
    std::string pending_;
};

std::vector<Fault> readTestFaults() {
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::make_unique<FaultTransport>(), "6", error));
    Obd2 obd(elm);
    return obd.readFaults(error);
}

}  // namespace

TEST_CASE("Oxygen sensor test and K-line monitoring are decoded", "[monitor]") {
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::make_unique<FaultTransport>(), "6", error));
    Obd2 obd(elm);

    // Mode 05: one byte per value, 0.005 V per bit.
    const std::vector<OxygenSensorTest> o2 = obd.readOxygenSensorTests(error);
    REQUIRE(o2.size() == 1);
    REQUIRE(o2[0].testId == 0x01);
    REQUIRE(o2[0].sensor == 1);
    REQUIRE(o2[0].value == Catch::Approx(0.700));
    REQUIRE(o2[0].unit == "V");

    // Mode 06 without CAN: ID, component, value, lower and upper limit.
    const std::vector<MonitorResult> monitor = obd.readMonitorResults(error);
    REQUIRE(monitor.size() == 1);
    REQUIRE(monitor[0].value == Catch::Approx(0.750));
    REQUIRE(monitor[0].minimum == Catch::Approx(0.400));
    REQUIRE(monitor[0].maximum == Catch::Approx(1.000));
    REQUIRE(monitor[0].passed);
}

TEST_CASE("Core texts follow the active language", "[lang]") {
    REQUIRE(language() == Language::German);

    // German is the default.
    {
        const std::vector<Fault> faults = readTestFaults();
        REQUIRE(faults.size() == 2);
        REQUIRE(faults[0].kind == "gespeichert");
        REQUIRE(faults[1].kind == "sporadisch");
    }

    {
        EnglishScope english;

        // Protocol and adapter messages from the core itself.
        const std::vector<Fault> faults = readTestFaults();
        REQUIRE(faults.size() == 2);
        REQUIRE(faults[0].code == "P0507");
        REQUIRE(faults[0].kind == "stored");
        REQUIRE(faults[1].kind == "intermittent");
        REQUIRE(ecuName(Ecu::Engine) == "01 Engine");
        REQUIRE(negativeResponseText(0x22).find("Conditions not correct") != std::string::npos);
        REQUIRE(decodeMessages({"NO DATA"}, 0x01).message.find("does not support") !=
                std::string::npos);
        REQUIRE(decodeMeasurement(0x01, 0x40, 0x64).label == "Engine speed");

        // Data tables, resolved through the public lookups.
        REQUIRE(std::string(findPid(0x0C)->name).find("Engine") != std::string::npos);
        REQUIRE(describeDtc("P0171").find("lean") != std::string::npos);
        const std::string vag = describeVagFault(16486, 0x01);
        REQUIRE(vag.find("P0102") != std::string::npos);
        REQUIRE(vag.find("Luftmassenmesser") == std::string::npos);
    }

    // The scope restored German.
    REQUIRE(language() == Language::German);
    REQUIRE(ecuName(Ecu::Engine) == "01 Motor");
    REQUIRE(describeDtc("P0171").find("mager") != std::string::npos);
}
