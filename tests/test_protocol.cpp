// Tests fuer die Teile, die man ohne Auto pruefen kann: Parser und Datenbanken.

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "core/Elm327.h"
#include "data/DtcDatabase.h"
#include "data/DtcLocation.h"
#include "data/MonitorTests.h"
#include "data/Pids.h"
#include "data/VagFaults.h"
#include "data/Vehicles.h"

using namespace caruspex;

TEST_CASE("Hexzeilen werden zu Bytes", "[parser]") {
    REQUIRE(hexToBytes("4100BE1FA813") ==
            std::vector<int>{0x41, 0x00, 0xBE, 0x1F, 0xA8, 0x13});
    REQUIRE(hexToBytes("41 00 BE") == std::vector<int>{0x41, 0x00, 0xBE});
    REQUIRE(hexToBytes("").empty());
    // Ungerade Anzahl Ziffern: das letzte halbe Byte faellt weg.
    REQUIRE(hexToBytes("4100B") == std::vector<int>{0x41, 0x00});
}

TEST_CASE("Antworten auf Dienst 03 werden erkannt", "[parser]") {
    const ElmResult result = decodeMessages({"430507155900 00"}, 0x03);
    REQUIRE(result.ok());
    REQUIRE(result.messages.size() == 1);
    REQUIRE(result.messages[0][0] == 0x05);
    REQUIRE(result.messages[0][1] == 0x07);
}

TEST_CASE("Mehrzeilige CAN-Antworten werden zusammengesetzt", "[parser]") {
    const ElmResult result = decodeMessages({"014", "0:4902014741", "1:554E45"}, 0x09);
    REQUIRE(result.ok());
}

TEST_CASE("Adaptermeldungen werden als Fehler gemeldet", "[parser]") {
    REQUIRE(decodeMessages({"NO DATA"}, 0x01).status == ElmStatus::NoData);
    REQUIRE(decodeMessages({"UNABLE TO CONNECT"}, 0x01).status == ElmStatus::BusError);
    REQUIRE(decodeMessages({"BUS INIT: ERROR"}, 0x01).status == ElmStatus::BusError);
    REQUIRE(decodeMessages({"7F0312"}, 0x03).status == ElmStatus::Negative);
}

TEST_CASE("Fehlercodes werden korrekt dekodiert", "[dtc]") {
    REQUIRE(decodeDtc(0x01, 0x33) == "P0133");
    REQUIRE(decodeDtc(0x43, 0x21) == "C0321");
    REQUIRE(decodeDtc(0x81, 0x23) == "B0123");
    REQUIRE(decodeDtc(0xC1, 0x00) == "U0100");
    REQUIRE(decodeDtc(0x05, 0x07) == "P0507");
    // 0x0000 heisst: kein Fehler an dieser Stelle.
    REQUIRE(decodeDtc(0x00, 0x00).empty());
}

TEST_CASE("Die Fehlerdatenbank kennt die wichtigen Codes", "[dtc]") {
    REQUIRE(dtcDescriptions().size() > 300);
    REQUIRE_FALSE(describeDtc("P0301").empty());
    REQUIRE_FALSE(describeDtc("P1559").empty());
    REQUIRE_FALSE(describeDtc("U0121").empty());
    // Unbekannte Codes fallen auf die Gruppenbedeutung zurueck statt leer zu bleiben.
    REQUIRE_FALSE(describeDtc("P0999").empty());
    REQUIRE(isThrottleDtc("P0507"));
    REQUIRE_FALSE(isThrottleDtc("P0301"));
}

TEST_CASE("Messwerte rechnen richtig um", "[pids]") {
    const PidSpec* rpm = findPid(0x0C);
    REQUIRE(rpm != nullptr);
    REQUIRE(rpm->calc({0x0F, 0xA0}) == 1000.0);   // (0x0FA0) / 4

    const PidSpec* coolant = findPid(0x05);
    REQUIRE(coolant->calc({0x5A}) == 50.0);       // 0x5A - 40

    const PidSpec* voltage = findPid(0x42);
    REQUIRE(voltage->calc({0x36, 0xB0}) == 14.0); // 14000 mV
}

TEST_CASE("Fahrzeugprofile treffen die richtige Epoche", "[vehicles]") {
    const VehicleProfile* a2 = findProfile("Audi", 2002);
    REQUIRE(a2 != nullptr);
    REQUIRE(a2->bus == BusKind::KLine);

    const VehicleProfile* b7 = findProfile("Audi", 2007);
    REQUIRE(b7 != nullptr);
    REQUIRE(b7->bus == BusKind::Can);

    REQUIRE(vehicleBrands().size() > 25);
}

TEST_CASE("Fehlercodes zeigen auf die richtige Stelle am Fahrzeug", "[location]") {
    REQUIRE(locateDtc("P0507").zone == CarZone::Throttle);
    REQUIRE(locateDtc("P1559").zone == CarZone::Throttle);   // nicht der Turbo
    REQUIRE(locateDtc("P1555").zone == CarZone::Turbo);
    REQUIRE(locateDtc("P0301").zone == CarZone::Cylinders);
    REQUIRE(locateDtc("P0420").zone == CarZone::Exhaust);
    REQUIRE(locateDtc("P0442").zone == CarZone::FuelTank);
    REQUIRE(locateDtc("P0118").zone == CarZone::Cooling);
    REQUIRE(locateDtc("C0035").zone == CarZone::Wheels);
    REQUIRE(locateDtc("B0001").zone == CarZone::Cabin);
    REQUIRE(locateDtc("U0121").zone == CarZone::Network);
    REQUIRE(locateDtc("P0700").zone == CarZone::Gearbox);
    // Zu jeder Zone gehoert ein Hinweis, wo man nachsieht.
    REQUIRE_FALSE(locateDtc("P0507").hint.empty());
    REQUIRE_FALSE(locateDtc("P0507").name.empty());
}

TEST_CASE("VAG-Nummern werden in P-Codes uebersetzt", "[vag]") {
    // Ab 16384 sind es umgerechnete EOBD-Codes.
    REQUIRE(vagCodeToPCode(16486) == "P0102");
    REQUIRE(vagCodeToPCode(16384) == "P0000");
    REQUIRE(vagCodeToPCode(17747) == "P1363");
    // Darunter gibt es keinen P-Code.
    REQUIRE(vagCodeToPCode(561).empty());
    REQUIRE(vagCodeToPCode(0).empty());

    REQUIRE(describeVagFault(16486).find("Luftmassenmesser") != std::string::npos);
    REQUIRE(describeVagFault(522).find("Kuehlmitteltemperaturgeber") != std::string::npos);
    REQUIRE(describeVagFault(1249).find("Zylinder 1") != std::string::npos);
    // Unbekanntes faellt nicht um.
    REQUIRE_FALSE(describeVagFault(9999).empty());
    REQUIRE(vagFaultCount() > 60);
}

TEST_CASE("Der Messwertsatz deckt den Standard ab", "[pids]") {
    REQUIRE(pidTable().size() >= 70);
    // Stichproben quer durch die Bereiche.
    for (int pid : {0x03, 0x04, 0x05, 0x0C, 0x10, 0x11, 0x14, 0x1C, 0x24,
                    0x2C, 0x33, 0x3C, 0x42, 0x45, 0x51, 0x5C, 0x5E, 0x63}) {
        INFO("PID 0x" << std::hex << pid);
        REQUIRE(findPid(pid) != nullptr);
    }
}

TEST_CASE("Die neuen Messwerte rechnen richtig", "[pids]") {
    // Katalysatortemperatur: (256A+B)/10 - 40
    const PidSpec* cat = findPid(0x3C);
    REQUIRE(cat->unit == std::string("C"));
    REQUIRE(cat->calc({0x13, 0x88}) == Catch::Approx(5000 / 10.0 - 40.0));

    // Breitbandsonde: Lambda = (256A+B) * 2 / 65536
    const PidSpec* wide = findPid(0x24);
    REQUIRE(wide->calc({0x80, 0x00, 0x00, 0x00}) == Catch::Approx(1.0));

    // Bezugsdrehmoment ist eine schlichte 16-Bit-Zahl.
    REQUIRE(findPid(0x63)->calc({0x01, 0x00}) == Catch::Approx(256.0));

    // Einspritzbeginn: ((256A+B) - 26880) / 128
    REQUIRE(findPid(0x5D)->calc({0x69, 0x00}) == Catch::Approx((26880 - 26880) / 128.0));

    // Gemischkorrektur Bank 2 verhaelt sich wie Bank 1.
    REQUIRE(findPid(0x08)->calc({128}) == Catch::Approx(0.0));
    REQUIRE(findPid(0x09)->calc({0}) == Catch::Approx(-100.0));
}

TEST_CASE("Statuswerte kommen als Text, nicht als Zahl", "[pids]") {
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

TEST_CASE("Bauteilueberwachung wird richtig umgerechnet", "[monitor]") {
    std::string unit;
    // Kennung 0x0B: Spannung in Millivolt.
    REQUIRE(monitorScale(0x0B, 1000, unit) == Catch::Approx(1.0));
    REQUIRE(unit == "V");
    // Kennung 0x24: Temperatur, ein Zehntel Grad mit Versatz.
    REQUIRE(monitorScale(0x24, 800, unit) == Catch::Approx(40.0));
    REQUIRE(unit == "Grad C");
    // Kennung 0x1E: Drehzahl in Vierteln.
    REQUIRE(monitorScale(0x1E, 4000, unit) == Catch::Approx(1000.0));
    REQUIRE(unit == "1/min");
    // Unbekannte Kennung liefert den Rohwert statt Unsinn.
    REQUIRE(monitorScale(0xFE, 1234, unit) == Catch::Approx(1234.0));

    REQUIRE_FALSE(monitorTestName(0x21).empty());
    REQUIRE(monitorComponentName(0x21).find("Katalysator") != std::string::npos);
    // Unbekanntes bleibt benannt statt leer.
    REQUIRE_FALSE(monitorTestName(0xF3).empty());
    REQUIRE_FALSE(monitorComponentName(0xF3).empty());
}

TEST_CASE("Lambdasondenpruefung wird richtig umgerechnet", "[monitor]") {
    std::string unit;
    // Spannungstests: fünf Millivolt je Schritt.
    REQUIRE(oxygenScale(0x01, 100, unit) == Catch::Approx(0.5));
    REQUIRE(unit == "V");
    // Zeittests: vier Millisekunden je Schritt.
    REQUIRE(oxygenScale(0x05, 250, unit) == Catch::Approx(1.0));
    REQUIRE(unit == "s");
    REQUIRE(oxygenTestName(0x01).find("mager nach fett") != std::string::npos);
    REQUIRE_FALSE(oxygenTestName(0x7F).empty());
}
