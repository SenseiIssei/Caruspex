// Checks the KWP1281 state machine against a simulated control unit.
// This tests the path to the throttle body basic settings without a car.

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "FakeEcu.h"
#include "core/Kwp1281.h"

using namespace caruspex;
using caruspex::test::FakeEcu;
using Catch::Approx;

TEST_CASE("Connecting wakes the right control unit", "[kwp1281]") {
    FakeEcu ecu;
    Kwp1281 protocol(ecu);
    std::string error;

    REQUIRE(protocol.connect(Ecu::Engine, error));
    REQUIRE(error.empty());
    REQUIRE(ecu.wokenAddress() == 0x01);
    REQUIRE(protocol.connected());
}

TEST_CASE("The control unit identification is read", "[kwp1281]") {
    FakeEcu ecu({"1C0906032AA ", "SIMOS 1.0 ", "Code 12345 "});
    Kwp1281 protocol(ecu);
    std::string error;

    REQUIRE(protocol.connect(Ecu::Engine, error));
    REQUIRE(protocol.identification().size() == 3);
    REQUIRE(protocol.identification()[0] == "1C0906032AA ");
    REQUIRE(protocol.identification()[1] == "SIMOS 1.0 ");
}

TEST_CASE("Other control units can be addressed", "[kwp1281]") {
    FakeEcu ecu;
    Kwp1281 protocol(ecu);
    std::string error;

    REQUIRE(protocol.connect(Ecu::Abs, error));
    REQUIRE(ecu.wokenAddress() == 0x03);
}

TEST_CASE("Read and clear the fault memory", "[kwp1281]") {
    FakeEcu ecu;
    // Two faults: 0x111B (4379) and 0x111C, the second one intermittent.
    ecu.faultPayload = {0x11, 0x1B, 0x24, 0x11, 0x1C, 0xA4};
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    std::vector<Kwp1281Fault> faults;
    REQUIRE(protocol.readFaults(faults, error));
    REQUIRE(faults.size() == 2);
    REQUIRE(faults[0].code == 0x111B);
    REQUIRE(faults[1].code == 0x111C);
    // Bit 7 in the status byte means intermittent.
    REQUIRE(faults[1].sporadic);
    REQUIRE_FALSE(faults[0].sporadic);

    REQUIRE(protocol.clearFaults(error));
    REQUIRE(ecu.clearWasCalled);

    std::vector<Kwp1281Fault> afterwards;
    REQUIRE(protocol.readFaults(afterwards, error));
    REQUIRE(afterwards.empty());
}

TEST_CASE("Read a measuring block", "[kwp1281]") {
    FakeEcu ecu;
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    std::vector<MeasuringValue> values;
    REQUIRE(protocol.readGroup(1, values, error));
    REQUIRE(ecu.lastGroupRequested == 1);
    REQUIRE(values.size() == 4);
    // Formula 0x01 is engine speed: 0.2 times A times B.
    REQUIRE(values[0].unit == "1/min");
    REQUIRE(values[0].value == Approx(0.2 * 0x40 * 0x64));
}

TEST_CASE("Basic settings is a separate service", "[kwp1281][throttle]") {
    FakeEcu ecu;
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    std::vector<MeasuringValue> values;
    REQUIRE(protocol.basicSetting(60, values, error));
    REQUIRE(ecu.lastBasicSettingGroup == 60);
    REQUIRE(ecu.lastGroupRequested == -1);   // not confused with reading
    REQUIRE_FALSE(values.empty());
}

TEST_CASE("Control units without basic settings report it cleanly", "[kwp1281]") {
    FakeEcu ecu;
    ecu.supportsBasicSetting = false;
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    std::vector<MeasuringValue> values;
    REQUIRE_FALSE(protocol.basicSetting(98, values, error));
    REQUIRE_FALSE(error.empty());
    REQUIRE(values.empty());
}

TEST_CASE("Several requests in a row keep the rhythm", "[kwp1281]") {
    FakeEcu ecu;
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    for (int round = 0; round < 5; ++round) {
        std::vector<MeasuringValue> values;
        REQUIRE(protocol.readGroup(round + 1, values, error));
        REQUIRE(ecu.lastGroupRequested == round + 1);
    }
}

TEST_CASE("Disconnecting signs off from the control unit", "[kwp1281]") {
    FakeEcu ecu;
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));
    protocol.disconnect();
    REQUIRE_FALSE(protocol.connected());
    REQUIRE(ecu.endOutputCount == 1);
}

TEST_CASE("Measuring value formulas compute correctly", "[kwp1281]") {
    // Engine speed
    const MeasuringValue rpm = decodeMeasurement(0x01, 0x40, 0x64);
    REQUIRE(rpm.unit == "1/min");
    REQUIRE(rpm.value == Approx(1280.0));

    // Temperature: A times (B minus 100) times 0.1
    const MeasuringValue temp = decodeMeasurement(0x05, 0x01, 0xE2);
    REQUIRE(temp.unit == "Grad C");
    REQUIRE(temp.value == Approx(0.1 * 1 * (226 - 100)));

    // Voltage
    const MeasuringValue volt = decodeMeasurement(0x06, 0x0A, 0x8C);
    REQUIRE(volt.unit == "V");
    REQUIRE(volt.value == Approx(0.001 * 10 * 140));

    // Empty field
    REQUIRE_FALSE(decodeMeasurement(0x00, 0x00, 0x00).valid);

    // An unknown formula does not fall over but returns the raw number.
    const MeasuringValue unknown = decodeMeasurement(0xEE, 0x12, 0x34);
    REQUIRE(unknown.valid);
    REQUIRE(unknown.value == Approx(0x12 * 256 + 0x34));
}

TEST_CASE("Control unit names are known", "[kwp1281]") {
    REQUIRE(ecuName(Ecu::Engine) == "01 Motor");
    REQUIRE(ecuName(Ecu::Abs) == "03 Bremsenelektronik (ABS)");
    REQUIRE(knownEcus().size() >= 10);
}

TEST_CASE("Read an adaptation channel", "[kwp1281]") {
    FakeEcu ecu;
    ecu.adaptationValue = 132;
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    int value = 0;
    std::vector<MeasuringValue> shown;
    REQUIRE(protocol.readAdaptation(1, value, shown, error));
    REQUIRE(ecu.lastAdaptationChannel == 1);
    REQUIRE(value == 132);
    REQUIRE(shown.size() == 4);
}

TEST_CASE("Control units without adaptation report it cleanly", "[kwp1281]") {
    FakeEcu ecu;
    ecu.supportsAdaptation = false;
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    int value = 0;
    std::vector<MeasuringValue> shown;
    REQUIRE_FALSE(protocol.readAdaptation(99, value, shown, error));
    REQUIRE_FALSE(error.empty());
}

TEST_CASE("Output test names the active component", "[kwp1281]") {
    FakeEcu ecu;
    ecu.actuatorName = "Tankentlueftungsventil N80";
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    std::string actuator;
    REQUIRE(protocol.actuatorTest(actuator, error));
    REQUIRE(actuator == "Tankentlueftungsventil N80");
    REQUIRE(ecu.actuatorTestCount == 1);
}

TEST_CASE("VAG fault numbers get plain text", "[kwp1281][vag]") {
    FakeEcu ecu;
    // 16486 corresponds to P0102, 561 is an old VAG-specific number.
    ecu.faultPayload = {0x40, 0x66, 0x24, 0x02, 0x31, 0x24};
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    std::vector<Kwp1281Fault> faults;
    REQUIRE(protocol.readFaults(faults, error));
    REQUIRE(faults.size() == 2);
    REQUIRE(faults[0].code == 16486);
    REQUIRE(faults[0].text.find("P0102") != std::string::npos);
    REQUIRE(faults[0].text.find("Luftmassenmesser") != std::string::npos);
    REQUIRE(faults[1].code == 561);
    REQUIRE(faults[1].text.find("Gemischadaption") != std::string::npos);
}
