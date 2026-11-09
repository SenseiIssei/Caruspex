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

// The standardized monitor IDs (OBDMID) of SAE J1979. They come in blocks,
// so the names are put together from bank and sensor.
std::string monitorComponentName(int id) {
    const auto bankSensor = [](int index, int perBank) {
        return L("Bank ", "bank ") + number(index / perBank + 1) + L(" Sonde ", " sensor ") +
               number(index % perBank + 1);
    };
    const auto bank = [](int index) { return L(" Bank ", " bank ") + number(index + 1); };

    if (id >= 0x01 && id <= 0x10) return L("Lambdasonde ", "O2 sensor ") + bankSensor(id - 0x01, 4);
    if (id >= 0x21 && id <= 0x24) return L("Katalysator", "Catalyst") + bank(id - 0x21);
    if (id >= 0x31 && id <= 0x34) return L("Abgasrückführung", "EGR") + bank(id - 0x31);
    if (id >= 0x35 && id <= 0x38)
        return L("Nockenwellenverstellung", "Variable valve timing") + bank(id - 0x35);
    switch (id) {
        case 0x39: return L("Tankentlüftung, Deckel offen oder großes Leck",
                            "Evaporative system, cap off or large leak");
        case 0x3A: return L("Tankentlüftung, Leck 0,090 Zoll", "Evaporative system, 0.090 inch leak");
        case 0x3B: return L("Tankentlüftung, Leck 0,040 Zoll", "Evaporative system, 0.040 inch leak");
        case 0x3C: return L("Tankentlüftung, Leck 0,020 Zoll", "Evaporative system, 0.020 inch leak");
        case 0x3D: return L("Tankentlüftung, Spülstrom", "Evaporative system, purge flow");
        default: break;
    }
    if (id >= 0x41 && id <= 0x50)
        return L("Lambdasondenheizung ", "O2 sensor heater ") + bankSensor(id - 0x41, 4);
    if (id >= 0x61 && id <= 0x64) return L("Katalysatorheizung", "Heated catalyst") + bank(id - 0x61);
    if (id >= 0x71 && id <= 0x74) return L("Sekundärluft ", "Secondary air ") + number(id - 0x70);
    if (id >= 0x81 && id <= 0x84) return L("Kraftstoffsystem", "Fuel system") + bank(id - 0x81);
    if (id >= 0x85 && id <= 0x88) return L("Ladedruckregelung", "Boost pressure control") + bank(id - 0x85);
    if (id >= 0x90 && id <= 0x91) return L("NOx-Speicherkatalysator", "NOx adsorber") + bank(id - 0x90);
    if (id >= 0x98 && id <= 0x99) return L("Stickoxid-Katalysator (SCR)", "NOx catalyst (SCR)") + bank(id - 0x98);
    if (id == 0xA1) return L("Verbrennungsaussetzer, ganzer Motor", "Misfire, whole engine");
    if (id >= 0xA2 && id <= 0xAD)
        return L("Verbrennungsaussetzer Zylinder ", "Misfire cylinder ") + number(id - 0xA1);
    if (id >= 0xB0 && id <= 0xB1) return L("Partikelfilter", "Particulate filter") + bank(id - 0xB0);
    return L("Baugruppe 0x", "Component 0x") + hex2(id);
}

// On CAN the tests of a monitor are numbered by TID. 0x01 to 0x0A are the
// same oxygen sensor tests as in mode 05, 0x0B and 0x0C belong to the
// misfire monitors, from 0x80 the manufacturer decides.
std::string monitorTestNameCan(int monitorId, int testId) {
    if (monitorId >= 0xA1 && monitorId <= 0xAD) {
        if (testId == 0x0B)
            return L("Aussetzer im Schnitt der letzten 10 Fahrten",
                     "Misfires, average of the last 10 trips");
        if (testId == 0x0C)
            return L("Aussetzer in der aktuellen Fahrt", "Misfires in the current trip");
    }
    if (testId >= 0x01 && testId <= 0x0A && monitorId >= 0x01 && monitorId <= 0x10)
        return oxygenTestName(testId);
    return "Test 0x" + hex2(testId) + L(" (herstellerspezifisch)", " (manufacturer-specific)");
}

// On CAN a separate byte, the unit and scaling ID of SAE J1979 (appendix
// "Unit and Scaling definitions for OBDMIDs"), says how the raw value is to
// be read. IDs 0x01 to 0x7F are unsigned, 0x80 to 0xFE read the same two
// bytes as a signed number.
double monitorScale(int unitAndScaling, int raw, std::string& unit) {
    const double u = static_cast<double>(raw & 0xFFFF);
    const double s = static_cast<double>((raw & 0x8000) ? (raw & 0xFFFF) - 0x10000 : (raw & 0xFFFF));
    const char* degC = L("Grad C", "deg C");
    const char* deg = L("Grad", "deg");
    switch (unitAndScaling) {
        // --- unsigned ---
        case 0x01: unit = ""; return u;
        case 0x02: unit = ""; return u * 0.1;
        case 0x03: unit = ""; return u * 0.01;
        case 0x04: unit = ""; return u * 0.001;
        case 0x05: unit = ""; return u * 0.0000305;
        case 0x06: unit = ""; return u * 0.000305;
        case 0x07: unit = "1/min"; return u * 0.25;
        case 0x08: unit = "km/h"; return u * 0.01;
        case 0x09: unit = "km/h"; return u;
        case 0x0A: unit = "V"; return u * 0.000122;
        case 0x0B: unit = "V"; return u * 0.001;
        case 0x0C: unit = "V"; return u * 0.01;
        case 0x0D: unit = "mA"; return u * 0.00390625;
        case 0x0E: unit = "A"; return u * 0.001;
        case 0x0F: unit = "A"; return u * 0.01;
        case 0x10: unit = "ms"; return u;
        case 0x11: unit = "s"; return u * 0.1;
        case 0x12: unit = "s"; return u;
        case 0x13: unit = "Ohm"; return u * 0.001;
        case 0x14: unit = "Ohm"; return u;
        case 0x15: unit = "kOhm"; return u;
        case 0x16: unit = degC; return u * 0.1 - 40.0;
        case 0x17: unit = "kPa"; return u * 0.01;
        case 0x18: unit = "kPa"; return u * 0.0117;
        case 0x19: unit = "kPa"; return u * 0.079;
        case 0x1A: unit = "kPa"; return u;
        case 0x1B: unit = "kPa"; return u * 10.0;
        case 0x1C: unit = deg; return u * 0.01;
        case 0x1D: unit = deg; return u * 0.5;
        case 0x1E: unit = "lambda"; return u * 0.0000305;
        case 0x1F: unit = "A/F"; return u * 0.05;
        case 0x20: unit = ""; return u * 0.0039062;
        case 0x21: unit = "mHz"; return u;
        case 0x22: unit = "Hz"; return u;
        case 0x23: unit = "kHz"; return u;
        case 0x24: unit = ""; return u;   // counts, e.g. misfires
        case 0x25: unit = "km"; return u;
        case 0x26: unit = "mV/ms"; return u * 0.1;
        case 0x27: unit = "g/s"; return u * 0.01;
        case 0x28: unit = "g/s"; return u;
        case 0x29: unit = "Pa/s"; return u * 0.25;
        case 0x2A: unit = "kg/h"; return u * 0.001;
        case 0x2B: unit = ""; return u;   // switch counts
        case 0x2C: unit = "g/cyl"; return u * 0.01;
        case 0x2D: unit = "mg/stroke"; return u * 0.01;
        case 0x2E: unit = ""; return u != 0.0 ? 1.0 : 0.0;   // true/false
        case 0x2F: unit = "%"; return u * 0.01;
        case 0x30: unit = "%"; return u * 0.001526;
        case 0x31: unit = "l"; return u * 0.001;
        case 0x32: unit = "inch"; return u * 0.0000305;
        case 0x33: unit = "lambda"; return u * 0.00024414;
        case 0x34: unit = "min"; return u;
        case 0x35: unit = "ms"; return u * 10.0;
        case 0x36: unit = "g"; return u * 0.01;
        case 0x37: unit = "g"; return u * 0.1;
        case 0x38: unit = "g"; return u;
        case 0x39: unit = "%"; return u * 0.01 - 327.68;
        case 0x3A: unit = "g"; return u * 0.001;
        case 0x3B: unit = "g"; return u * 0.0001;
        case 0x3C: unit = "us"; return u * 0.1;
        case 0x3D: unit = "mA"; return u * 0.01;
        case 0x3E: unit = "mm2"; return u * 0.00006103516;
        case 0x3F: unit = "l"; return u * 0.01;
        case 0x40: unit = "ppm"; return u;
        case 0x41: unit = "uA"; return u * 0.01;
        // --- signed ---
        case 0x81: unit = ""; return s;
        case 0x82: unit = ""; return s * 0.1;
        case 0x83: unit = ""; return s * 0.01;
        case 0x84: unit = ""; return s * 0.001;
        case 0x85: unit = ""; return s * 0.0000305;
        case 0x86: unit = ""; return s * 0.000305;
        case 0x87: unit = "ppm"; return s;
        case 0x8A: unit = "V"; return s * 0.000122;
        case 0x8B: unit = "V"; return s * 0.001;
        case 0x8C: unit = "V"; return s * 0.01;
        case 0x8D: unit = "mA"; return s * 0.00390625;
        case 0x8E: unit = "A"; return s * 0.001;
        case 0x90: unit = "ms"; return s;
        case 0x96: unit = degC; return s * 0.1;
        case 0x99: unit = "kPa"; return s * 0.1;
        case 0x9C: unit = deg; return s * 0.01;
        case 0x9D: unit = deg; return s * 0.5;
        case 0xA8: unit = "g/s"; return s;
        case 0xA9: unit = "Pa/s"; return s * 0.25;
        case 0xAD: unit = "mg/stroke"; return s * 0.01;
        case 0xAE: unit = "mg/stroke"; return s * 0.1;
        case 0xAF: unit = "%"; return s * 0.01;
        case 0xB0: unit = "%"; return s * 0.003052;
        case 0xB1: unit = "mV/s"; return s * 2.0;
        case 0xFC: unit = "kPa"; return s * 0.01;
        case 0xFD: unit = "kPa"; return s * 0.001;
        case 0xFE: unit = "Pa"; return s * 0.25;
        default: unit = ""; return u;
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
