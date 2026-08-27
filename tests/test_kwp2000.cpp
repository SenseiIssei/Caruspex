// Prueft KWP2000 und die Weiche, die anhand der Schluesselbytes entscheidet.
// Das ist der Weg, den ein Audi A2 1.6 FSI mit Bosch MED nimmt.

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "FakeEcu.h"
#include "FakeEcu2000.h"
#include "core/VagKLine.h"

using namespace caruspex;
using caruspex::test::FakeEcu;
using caruspex::test::FakeEcu2000;
using Catch::Approx;

TEST_CASE("Schluesselbytes verraten das Protokoll", "[kwp2000]") {
    // Das alte Blockprotokoll meldet sich mit 0x01 0x8A.
    REQUIRE(protocolFromKeyBytes(0x01, 0x8A) == KLineProtocol::Kwp1281);
    // KWP2000 meldet sich mit 0x6B oder 0x6F und Verwandten.
    REQUIRE(protocolFromKeyBytes(0xEF, 0x8F) == KLineProtocol::Kwp2000);
    REQUIRE(protocolFromKeyBytes(0x6B, 0x8F) == KLineProtocol::Kwp2000);
    // Unsinn bleibt unbekannt statt falsch geraten zu werden.
    REQUIRE(protocolFromKeyBytes(0x00, 0x00) == KLineProtocol::Unknown);
}

TEST_CASE("Die Weiche waehlt bei einem alten Steuergeraet KWP1281", "[kwp2000]") {
    FakeEcu ecu;
    VagKLine line(ecu);
    std::string error;

    REQUIRE(line.connect(Ecu::Engine, error));
    REQUIRE(line.protocol() == KLineProtocol::Kwp1281);
    REQUIRE(line.connected());
    REQUIRE_FALSE(line.identification().empty());
}

TEST_CASE("Die Weiche waehlt beim FSI-Steuergeraet KWP2000", "[kwp2000]") {
    FakeEcu2000 ecu;
    VagKLine line(ecu);
    std::string error;

    REQUIRE(line.connect(Ecu::Engine, error));
    REQUIRE(line.protocol() == KLineProtocol::Kwp2000);
    REQUIRE(line.connected());
    REQUIRE(ecu.wokenAddress() == 0x01);
    REQUIRE(ecu.lastSession == session::kDiagnostics);
    REQUIRE(line.identification().size() == 1);
    REQUIRE(line.identification()[0].find("MED") != std::string::npos);
}

TEST_CASE("Fehler lesen und loeschen ueber KWP2000", "[kwp2000]") {
    FakeEcu2000 ecu;
    // 0x4066 ist 16486 und damit P0102, 0x0231 ist die alte VAG-Nummer 561.
    ecu.faults = {0x40, 0x66, 0xE0, 0x02, 0x31, 0x60};
    VagKLine line(ecu);
    std::string error;
    REQUIRE(line.connect(Ecu::Engine, error));

    std::vector<Kwp1281Fault> faults;
    REQUIRE(line.readFaults(faults, error));
    REQUIRE(faults.size() == 2);
    REQUIRE(faults[0].code == 16486);
    REQUIRE(faults[0].text.find("Luftmassenmesser") != std::string::npos);
    REQUIRE(faults[1].code == 561);
    REQUIRE(faults[1].text.find("Gemischadaption") != std::string::npos);

    REQUIRE(line.clearFaults(error));
    REQUIRE(ecu.clearWasCalled);

    std::vector<Kwp1281Fault> afterwards;
    REQUIRE(line.readFaults(afterwards, error));
    REQUIRE(afterwards.empty());
}

TEST_CASE("Messwertblock ueber KWP2000", "[kwp2000]") {
    FakeEcu2000 ecu;
    VagKLine line(ecu);
    std::string error;
    REQUIRE(line.connect(Ecu::Engine, error));

    std::vector<MeasuringValue> values;
    REQUIRE(line.readGroup(3, values, error));
    REQUIRE(ecu.lastGroup == 3);
    REQUIRE(values.size() == 4);
    REQUIRE(values[0].unit == "1/min");
    REQUIRE(values[0].value == Approx(0.2 * 0x40 * 0x64));
}

TEST_CASE("Grundeinstellung ueber KWP2000 eroeffnet eine eigene Sitzung",
          "[kwp2000][drosselklappe]") {
    FakeEcu2000 ecu;
    VagKLine line(ecu);
    std::string error;
    REQUIRE(line.connect(Ecu::Engine, error));
    const int sessionsAfterConnect = ecu.sessionCount;

    std::vector<MeasuringValue> values;
    REQUIRE(line.basicSetting(98, values, error));
    REQUIRE(ecu.lastSession == session::kAdaptation);
    REQUIRE(ecu.sessionCount == sessionsAfterConnect + 1);
    REQUIRE(ecu.lastGroup == 98);
    REQUIRE_FALSE(values.empty());
}

TEST_CASE("Abgelehnte Sitzungsart wird sauber gemeldet", "[kwp2000]") {
    FakeEcu2000 ecu;
    ecu.refuseSession = session::kAdaptation;
    VagKLine line(ecu);
    std::string error;
    REQUIRE(line.connect(Ecu::Engine, error));

    std::vector<MeasuringValue> values;
    REQUIRE_FALSE(line.basicSetting(98, values, error));
    REQUIRE_FALSE(error.empty());
    // Der Grund soll im Klartext dastehen, nicht als nackte Nummer.
    REQUIRE(error.find("Bedingungen") != std::string::npos);
}

TEST_CASE("Andere Sitzungsart laesst sich vorgeben", "[kwp2000]") {
    FakeEcu2000 ecu;
    VagKLine line(ecu);
    std::string error;
    REQUIRE(line.connect(Ecu::Engine, error));

    line.setBasicSettingSession(0x86);
    std::vector<MeasuringValue> values;
    REQUIRE(line.basicSetting(60, values, error));
    REQUIRE(ecu.lastSession == 0x86);
}

TEST_CASE("Verbindung halten ueber KWP2000", "[kwp2000]") {
    FakeEcu2000 ecu;
    VagKLine line(ecu);
    std::string error;
    REQUIRE(line.connect(Ecu::Engine, error));
    REQUIRE(line.keepAlive(error));
}

TEST_CASE("Trennen meldet sich beim KWP2000-Steuergeraet ab", "[kwp2000]") {
    FakeEcu2000 ecu;
    VagKLine line(ecu);
    std::string error;
    REQUIRE(line.connect(Ecu::Engine, error));
    line.disconnect();
    REQUIRE_FALSE(line.connected());
    REQUIRE(ecu.stopped);
}

TEST_CASE("Was KWP2000 nicht kann, sagt es statt still zu scheitern", "[kwp2000]") {
    FakeEcu2000 ecu;
    VagKLine line(ecu);
    std::string error;
    REQUIRE(line.connect(Ecu::Engine, error));

    int value = 0;
    std::vector<MeasuringValue> shown;
    REQUIRE_FALSE(line.readAdaptation(1, value, shown, error));
    REQUIRE(error.find("KWP2000") != std::string::npos);

    std::string actuator;
    REQUIRE_FALSE(line.actuatorTest(actuator, error));
    REQUIRE(error.find("KWP2000") != std::string::npos);
}

TEST_CASE("Abgelehnte Dienste kommen im Klartext zurueck", "[kwp2000]") {
    REQUIRE(negativeResponseText(0x11).find("nicht unterstuetzt") != std::string::npos);
    REQUIRE(negativeResponseText(0x22).find("Bedingungen") != std::string::npos);
    REQUIRE(negativeResponseText(0x33).find("Freischaltcode") != std::string::npos);
    REQUIRE_FALSE(negativeResponseText(0xAB).empty());
}
