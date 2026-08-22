// Tests fuer die Teile, die man ohne Auto pruefen kann: Parser und Datenbanken.

#include <catch2/catch_test_macros.hpp>

#include "core/Elm327.h"
#include "data/DtcDatabase.h"
#include "data/DtcLocation.h"
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
