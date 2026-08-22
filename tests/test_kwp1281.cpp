// Prueft die KWP1281-Zustandsmaschine gegen ein nachgebautes Steuergeraet.
// Damit ist der Weg zur Drosselklappen-Grundeinstellung ohne Auto getestet.

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "FakeEcu.h"
#include "core/Kwp1281.h"

using namespace caruspex;
using caruspex::test::FakeEcu;
using Catch::Approx;

TEST_CASE("Verbindungsaufbau weckt das richtige Steuergeraet", "[kwp1281]") {
    FakeEcu ecu;
    Kwp1281 protocol(ecu);
    std::string error;

    REQUIRE(protocol.connect(Ecu::Engine, error));
    REQUIRE(error.empty());
    REQUIRE(ecu.wokenAddress() == 0x01);
    REQUIRE(protocol.connected());
}

TEST_CASE("Die Kennung des Steuergeraets wird eingelesen", "[kwp1281]") {
    FakeEcu ecu({"1C0906032AA ", "SIMOS 1.0 ", "Code 12345 "});
    Kwp1281 protocol(ecu);
    std::string error;

    REQUIRE(protocol.connect(Ecu::Engine, error));
    REQUIRE(protocol.identification().size() == 3);
    REQUIRE(protocol.identification()[0] == "1C0906032AA ");
    REQUIRE(protocol.identification()[1] == "SIMOS 1.0 ");
}

TEST_CASE("Andere Steuergeraete lassen sich adressieren", "[kwp1281]") {
    FakeEcu ecu;
    Kwp1281 protocol(ecu);
    std::string error;

    REQUIRE(protocol.connect(Ecu::Abs, error));
    REQUIRE(ecu.wokenAddress() == 0x03);
}

TEST_CASE("Fehlerspeicher lesen und loeschen", "[kwp1281]") {
    FakeEcu ecu;
    // Zwei Fehler: 0x111B (4379) und 0x2400 wird als 9216 gelesen.
    ecu.faultPayload = {0x11, 0x1B, 0x24, 0x11, 0x1C, 0xA4};
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    std::vector<Kwp1281Fault> faults;
    REQUIRE(protocol.readFaults(faults, error));
    REQUIRE(faults.size() == 2);
    REQUIRE(faults[0].code == 0x111B);
    REQUIRE(faults[1].code == 0x111C);
    // Bit 7 im Statusbyte heisst sporadisch.
    REQUIRE(faults[1].sporadic);
    REQUIRE_FALSE(faults[0].sporadic);

    REQUIRE(protocol.clearFaults(error));
    REQUIRE(ecu.clearWasCalled);

    std::vector<Kwp1281Fault> afterwards;
    REQUIRE(protocol.readFaults(afterwards, error));
    REQUIRE(afterwards.empty());
}

TEST_CASE("Messwertblock lesen", "[kwp1281]") {
    FakeEcu ecu;
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    std::vector<MeasuringValue> values;
    REQUIRE(protocol.readGroup(1, values, error));
    REQUIRE(ecu.lastGroupRequested == 1);
    REQUIRE(values.size() == 4);
    // Formel 0x01 ist die Drehzahl: 0,2 mal A mal B.
    REQUIRE(values[0].unit == "1/min");
    REQUIRE(values[0].value == Approx(0.2 * 0x40 * 0x64));
}

TEST_CASE("Grundeinstellung ist ein eigener Dienst", "[kwp1281][drosselklappe]") {
    FakeEcu ecu;
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));

    std::vector<MeasuringValue> values;
    REQUIRE(protocol.basicSetting(60, values, error));
    REQUIRE(ecu.lastBasicSettingGroup == 60);
    REQUIRE(ecu.lastGroupRequested == -1);   // nicht mit dem Lesen verwechselt
    REQUIRE_FALSE(values.empty());
}

TEST_CASE("Steuergeraete ohne Grundeinstellung melden das sauber", "[kwp1281]") {
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

TEST_CASE("Mehrere Anfragen hintereinander halten den Takt", "[kwp1281]") {
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

TEST_CASE("Trennen meldet sich beim Steuergeraet ab", "[kwp1281]") {
    FakeEcu ecu;
    Kwp1281 protocol(ecu);
    std::string error;
    REQUIRE(protocol.connect(Ecu::Engine, error));
    protocol.disconnect();
    REQUIRE_FALSE(protocol.connected());
    REQUIRE(ecu.endOutputCount == 1);
}

TEST_CASE("Messwertformeln rechnen richtig", "[kwp1281]") {
    // Drehzahl
    const MeasuringValue rpm = decodeMeasurement(0x01, 0x40, 0x64);
    REQUIRE(rpm.unit == "1/min");
    REQUIRE(rpm.value == Approx(1280.0));

    // Temperatur: A mal (B minus 100) mal 0,1
    const MeasuringValue temp = decodeMeasurement(0x05, 0x01, 0xE2);
    REQUIRE(temp.unit == "Grad C");
    REQUIRE(temp.value == Approx(0.1 * 1 * (226 - 100)));

    // Spannung
    const MeasuringValue volt = decodeMeasurement(0x06, 0x0A, 0x8C);
    REQUIRE(volt.unit == "V");
    REQUIRE(volt.value == Approx(0.001 * 10 * 140));

    // Leeres Feld
    REQUIRE_FALSE(decodeMeasurement(0x00, 0x00, 0x00).valid);

    // Unbekannte Formel faellt nicht um, sondern liefert die Rohzahl.
    const MeasuringValue unknown = decodeMeasurement(0xEE, 0x12, 0x34);
    REQUIRE(unknown.valid);
    REQUIRE(unknown.value == Approx(0x12 * 256 + 0x34));
}

TEST_CASE("Steuergeraetenamen sind hinterlegt", "[kwp1281]") {
    REQUIRE(ecuName(Ecu::Engine) == "01 Motor");
    REQUIRE(ecuName(Ecu::Abs) == "03 Bremsenelektronik (ABS)");
    REQUIRE(knownEcus().size() >= 10);
}
