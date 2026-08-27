#include "data/MonitorTests.h"

#include <cstdio>
#include <unordered_map>

namespace caruspex {
namespace {

std::string number(int value) { return std::to_string(value); }

std::string hex2(int value) {
    char buffer[8];
    std::snprintf(buffer, sizeof(buffer), "%02X", value & 0xFF);
    return buffer;
}

}  // namespace

// Auf der K-Line vergibt jeder Hersteller die Testnummern selbst. Genormt sind
// nur die Bereiche fuer Lambdasonden und Katalysator, und die sind es, die man
// im Alltag braucht.
std::string monitorTestName(int testId) {
    static const std::unordered_map<int, const char*> known = {
        {0x01, "Lambdasonde Bank 1 Sonde 1, Spannungsschwelle mager nach fett"},
        {0x02, "Lambdasonde Bank 1 Sonde 1, Spannungsschwelle fett nach mager"},
        {0x03, "Lambdasonde Bank 1 Sonde 1, tiefste Spannung"},
        {0x04, "Lambdasonde Bank 1 Sonde 1, hoechste Spannung"},
        {0x05, "Lambdasonde Bank 1 Sonde 1, Umschaltzeit"},
        {0x06, "Lambdasonde Bank 1 Sonde 1, Periodendauer"},
        {0x07, "Lambdasonde Bank 1 Sonde 2, Spannungsschwelle mager nach fett"},
        {0x08, "Lambdasonde Bank 1 Sonde 2, Spannungsschwelle fett nach mager"},
        {0x09, "Lambdasonde Bank 1 Sonde 2, tiefste Spannung"},
        {0x0A, "Lambdasonde Bank 1 Sonde 2, hoechste Spannung"},
        {0x21, "Katalysator Bank 1, Wirkungsgrad"},
        {0x22, "Katalysator Bank 2, Wirkungsgrad"},
        {0x31, "Tankentlueftung, Leckpruefung"},
        {0x32, "Tankentlueftung, Durchfluss"},
        {0x39, "Abgasrueckfuehrung, Durchfluss"},
        {0x3A, "Sekundaerluft, Durchfluss"},
        {0x41, "Lambdasondenheizung Bank 1 Sonde 1"},
        {0x42, "Lambdasondenheizung Bank 1 Sonde 2"},
        {0x81, "Verbrennungsaussetzer, Zaehler Zylinder 1"},
        {0x82, "Verbrennungsaussetzer, Zaehler Zylinder 2"},
        {0x83, "Verbrennungsaussetzer, Zaehler Zylinder 3"},
        {0x84, "Verbrennungsaussetzer, Zaehler Zylinder 4"},
    };
    const auto it = known.find(testId);
    if (it != known.end()) return it->second;
    return "Test 0x" + hex2(testId) + " (herstellerspezifisch)";
}

std::string monitorComponentName(int componentId) {
    static const std::unordered_map<int, const char*> known = {
        {0x01, "Lambdasonde Bank 1 Sonde 1"}, {0x02, "Lambdasonde Bank 1 Sonde 2"},
        {0x03, "Lambdasonde Bank 1 Sonde 3"}, {0x04, "Lambdasonde Bank 1 Sonde 4"},
        {0x05, "Lambdasonde Bank 2 Sonde 1"}, {0x06, "Lambdasonde Bank 2 Sonde 2"},
        {0x07, "Lambdasonde Bank 2 Sonde 3"}, {0x08, "Lambdasonde Bank 2 Sonde 4"},
        {0x21, "Katalysator Bank 1"},         {0x22, "Katalysator Bank 2"},
        {0x31, "Tankentlueftung, 0,04 Zoll"}, {0x32, "Tankentlueftung, 0,02 Zoll"},
        {0x33, "Tankentlueftung, Spuelventil"},
        {0x35, "Tankentlueftung, Belueftungsventil"},
        {0x39, "Abgasrueckfuehrung"},         {0x3A, "Sekundaerluft"},
        {0x41, "Lambdasondenheizung Bank 1 Sonde 1"},
        {0x42, "Lambdasondenheizung Bank 1 Sonde 2"},
        {0xA1, "Verbrennungsaussetzer, allgemein"},
        {0xA2, "Verbrennungsaussetzer Zylinder 1"},
        {0xA3, "Verbrennungsaussetzer Zylinder 2"},
        {0xA4, "Verbrennungsaussetzer Zylinder 3"},
        {0xA5, "Verbrennungsaussetzer Zylinder 4"},
    };
    const auto it = known.find(componentId);
    if (it != known.end()) return it->second;
    return "Baugruppe 0x" + hex2(componentId);
}

// Bei CAN sagt ein eigenes Byte, wie der Rohwert zu lesen ist.
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
        case 0x24: unit = "Grad C"; return raw * 0.1 - 40.0;
        case 0x27: unit = "%"; return raw * 0.001526;
        case 0x28: unit = "%"; return raw * 0.1;
        case 0x2F: unit = "Grad"; return raw * 0.01;
        case 0x30: unit = "Grad"; return raw * 0.5 - 64.0;
        case 0x31: unit = "l"; return raw * 0.001;
        case 0x36: unit = "s"; return raw;
        case 0x38: unit = "min"; return raw;
        case 0x81: unit = ""; return raw;
        default: unit = ""; return raw;
    }
}

// Modus 05 kennt eine kurze, genormte Liste.
std::string oxygenTestName(int testId) {
    switch (testId) {
        case 0x01: return "Spannungsschwelle mager nach fett";
        case 0x02: return "Spannungsschwelle fett nach mager";
        case 0x03: return "tiefste Spannung im Regelbetrieb";
        case 0x04: return "hoechste Spannung im Regelbetrieb";
        case 0x05: return "Umschaltzeit mager nach fett";
        case 0x06: return "Umschaltzeit fett nach mager";
        case 0x07: return "tiefste Sondenspannung";
        case 0x08: return "hoechste Sondenspannung";
        case 0x09: return "Zeit zwischen den Umschaltungen";
        case 0x0A: return "Periodendauer der Regelung";
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
