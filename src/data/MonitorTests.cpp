#include "data/MonitorTests.h"

#include <cstdio>
#include <unordered_map>

#include "core/Lang.h"

namespace caruspex {
namespace {

std::string number(int value) { return std::to_string(value); }

std::string hex2(int value) {
    char buffer[8];
    std::snprintf(buffer, sizeof(buffer), "%02X", value & 0xFF);
    return buffer;
}

}  // namespace

// On the K-Line every manufacturer assigns the test numbers itself. Only the
// ranges for oxygen sensors and catalytic converter are standardized, and
// those are the ones you need in everyday use.
std::string monitorTestName(int testId) {
    static const std::unordered_map<int, Text> known = {
        {0x01, {"Lambdasonde Bank 1 Sonde 1, Spannungsschwelle mager nach fett",
                "O2 sensor bank 1 sensor 1, lean-to-rich threshold voltage"}},
        {0x02, {"Lambdasonde Bank 1 Sonde 1, Spannungsschwelle fett nach mager",
                "O2 sensor bank 1 sensor 1, rich-to-lean threshold voltage"}},
        {0x03, {"Lambdasonde Bank 1 Sonde 1, tiefste Spannung",
                "O2 sensor bank 1 sensor 1, minimum voltage"}},
        {0x04, {"Lambdasonde Bank 1 Sonde 1, höchste Spannung",
                "O2 sensor bank 1 sensor 1, maximum voltage"}},
        {0x05, {"Lambdasonde Bank 1 Sonde 1, Umschaltzeit",
                "O2 sensor bank 1 sensor 1, switch time"}},
        {0x06, {"Lambdasonde Bank 1 Sonde 1, Periodendauer",
                "O2 sensor bank 1 sensor 1, period"}},
        {0x07, {"Lambdasonde Bank 1 Sonde 2, Spannungsschwelle mager nach fett",
                "O2 sensor bank 1 sensor 2, lean-to-rich threshold voltage"}},
        {0x08, {"Lambdasonde Bank 1 Sonde 2, Spannungsschwelle fett nach mager",
                "O2 sensor bank 1 sensor 2, rich-to-lean threshold voltage"}},
        {0x09, {"Lambdasonde Bank 1 Sonde 2, tiefste Spannung",
                "O2 sensor bank 1 sensor 2, minimum voltage"}},
        {0x0A, {"Lambdasonde Bank 1 Sonde 2, höchste Spannung",
                "O2 sensor bank 1 sensor 2, maximum voltage"}},
        {0x21, {"Katalysator Bank 1, Wirkungsgrad", "Catalyst bank 1, efficiency"}},
        {0x22, {"Katalysator Bank 2, Wirkungsgrad", "Catalyst bank 2, efficiency"}},
        {0x31, {"Tankentlüftung, Leckprüfung", "Evaporative system, leak check"}},
        {0x32, {"Tankentlüftung, Durchfluss", "Evaporative system, flow"}},
        {0x39, {"Abgasrückführung, Durchfluss", "EGR, flow"}},
        {0x3A, {"Sekundärluft, Durchfluss", "Secondary air, flow"}},
        {0x41, {"Lambdasondenheizung Bank 1 Sonde 1", "O2 sensor heater bank 1 sensor 1"}},
        {0x42, {"Lambdasondenheizung Bank 1 Sonde 2", "O2 sensor heater bank 1 sensor 2"}},
        {0x81, {"Verbrennungsaussetzer, Zähler Zylinder 1", "Misfire, counter cylinder 1"}},
        {0x82, {"Verbrennungsaussetzer, Zähler Zylinder 2", "Misfire, counter cylinder 2"}},
        {0x83, {"Verbrennungsaussetzer, Zähler Zylinder 3", "Misfire, counter cylinder 3"}},
        {0x84, {"Verbrennungsaussetzer, Zähler Zylinder 4", "Misfire, counter cylinder 4"}},
    };
    const auto it = known.find(testId);
    if (it != known.end()) return it->second.get();
    return "Test 0x" + hex2(testId) + L(" (herstellerspezifisch)", " (manufacturer-specific)");
}

std::string monitorComponentName(int componentId) {
    static const std::unordered_map<int, Text> known = {
        {0x01, {"Lambdasonde Bank 1 Sonde 1", "O2 sensor bank 1 sensor 1"}},
        {0x02, {"Lambdasonde Bank 1 Sonde 2", "O2 sensor bank 1 sensor 2"}},
        {0x03, {"Lambdasonde Bank 1 Sonde 3", "O2 sensor bank 1 sensor 3"}},
        {0x04, {"Lambdasonde Bank 1 Sonde 4", "O2 sensor bank 1 sensor 4"}},
        {0x05, {"Lambdasonde Bank 2 Sonde 1", "O2 sensor bank 2 sensor 1"}},
        {0x06, {"Lambdasonde Bank 2 Sonde 2", "O2 sensor bank 2 sensor 2"}},
        {0x07, {"Lambdasonde Bank 2 Sonde 3", "O2 sensor bank 2 sensor 3"}},
        {0x08, {"Lambdasonde Bank 2 Sonde 4", "O2 sensor bank 2 sensor 4"}},
        {0x21, {"Katalysator Bank 1", "Catalyst bank 1"}},
        {0x22, {"Katalysator Bank 2", "Catalyst bank 2"}},
        {0x31, {"Tankentlüftung, 0,04 Zoll", "Evaporative system, 0.040 inch"}},
        {0x32, {"Tankentlüftung, 0,02 Zoll", "Evaporative system, 0.020 inch"}},
        {0x33, {"Tankentlüftung, Spülventil", "Evaporative system, purge valve"}},
        {0x35, {"Tankentlüftung, Belüftungsventil", "Evaporative system, vent valve"}},
        {0x39, {"Abgasrückführung", "EGR"}},
        {0x3A, {"Sekundärluft", "Secondary air"}},
        {0x41, {"Lambdasondenheizung Bank 1 Sonde 1", "O2 sensor heater bank 1 sensor 1"}},
        {0x42, {"Lambdasondenheizung Bank 1 Sonde 2", "O2 sensor heater bank 1 sensor 2"}},
        {0xA1, {"Verbrennungsaussetzer, allgemein", "Misfire, general"}},
        {0xA2, {"Verbrennungsaussetzer Zylinder 1", "Misfire cylinder 1"}},
        {0xA3, {"Verbrennungsaussetzer Zylinder 2", "Misfire cylinder 2"}},
        {0xA4, {"Verbrennungsaussetzer Zylinder 3", "Misfire cylinder 3"}},
        {0xA5, {"Verbrennungsaussetzer Zylinder 4", "Misfire cylinder 4"}},
    };
    const auto it = known.find(componentId);
    if (it != known.end()) return it->second.get();
    return L("Baugruppe 0x", "Component 0x") + hex2(componentId);
}

// On CAN a separate byte says how the raw value is to be read.
double monitorScale(int unitAndScaling, int raw, std::string& unit) {
    switch (unitAndScaling) {
        case 0x01: unit = ""; return raw;
        case 0x02: unit = ""; return raw * 0.1;
        case 0x03: unit = ""; return raw * 0.01;
        case 0x04: unit = ""; return raw * 0.001;
        case 0x05: unit = ""; return raw * 0.0000305;
        case 0x06: unit = ""; return raw * 0.000305;
        case 0x0A: unit = "V"; return raw * 0.000122;
        case 0x0B: unit = "V"; return raw * 0.001;
        case 0x0C: unit = "V"; return raw * 0.01;
        case 0x10: unit = "ms"; return raw;
        case 0x11: unit = "s"; return raw * 0.1;
        case 0x14: unit = "mA"; return raw * 0.00390625;
        case 0x15: unit = "A"; return raw * 0.001;
        case 0x18: unit = "kPa"; return raw * 0.0305;
        case 0x19: unit = "kPa"; return raw * 0.001;
        case 0x1A: unit = "kPa"; return raw * 0.25;
        case 0x1B: unit = "kPa"; return raw;
        case 0x1E: unit = "1/min"; return raw * 0.25;
        case 0x1F: unit = "km/h"; return raw;
        case 0x22: unit = "g/s"; return raw * 0.00390625;
        case 0x23: unit = "g/s"; return raw * 0.001;
        case 0x24: unit = L("Grad C", "deg C"); return raw * 0.1 - 40.0;
        case 0x27: unit = "%"; return raw * 0.001526;
        case 0x28: unit = "%"; return raw * 0.1;
        case 0x2F: unit = L("Grad", "deg"); return raw * 0.01;
        case 0x30: unit = L("Grad", "deg"); return raw * 0.5 - 64.0;
        case 0x31: unit = "l"; return raw * 0.001;
        case 0x36: unit = "s"; return raw;
        case 0x38: unit = "min"; return raw;
        case 0x81: unit = ""; return raw;
        default: unit = ""; return raw;
    }
}

// Mode 05 has a short, standardized list.
std::string oxygenTestName(int testId) {
    switch (testId) {
        case 0x01: return L("Spannungsschwelle mager nach fett", "Lean-to-rich threshold voltage");
        case 0x02: return L("Spannungsschwelle fett nach mager", "Rich-to-lean threshold voltage");
        case 0x03: return L("tiefste Spannung im Regelbetrieb", "Minimum voltage in closed loop");
        case 0x04: return L("höchste Spannung im Regelbetrieb", "Maximum voltage in closed loop");
        case 0x05: return L("Umschaltzeit mager nach fett", "Lean-to-rich switch time");
        case 0x06: return L("Umschaltzeit fett nach mager", "Rich-to-lean switch time");
        case 0x07: return L("tiefste Sondenspannung", "Minimum sensor voltage");
        case 0x08: return L("höchste Sondenspannung", "Maximum sensor voltage");
        case 0x09: return L("Zeit zwischen den Umschaltungen", "Time between transitions");
        case 0x0A: return L("Periodendauer der Regelung", "Control period");
        default: return "Test " + number(testId);
    }
}

double oxygenScale(int testId, int raw, std::string& unit) {
    switch (testId) {
        case 0x01:
        case 0x02:
        case 0x03:
        case 0x04:
        case 0x07:
        case 0x08:
            unit = "V";
            return raw * 0.005;
        case 0x05:
        case 0x06:
        case 0x09:
        case 0x0A:
            unit = "s";
            return raw * 0.004;
        default:
            unit = "";
            return raw;
    }
}

}  // namespace caruspex
