#include "core/analysis/Readiness.h"

#include <cctype>
#include <string>

namespace caruspex::analysis {

DecodedReadiness decodeReadiness(int a, int b, int c, int d) {
    DecodedReadiness out;
    out.milOn = (a & 0x80) != 0;
    out.storedFaults = a & 0x7F;
    out.compressionIgnition = (b & 0x08) != 0;

    // Byte B: bits 0..2 say "supported", bits 4..6 say "not yet complete".
    static const Monitor continuous[3] = {Monitor::Misfire, Monitor::FuelSystem,
                                          Monitor::Components};
    for (int bit = 0; bit < 3; ++bit) {
        if (b & (1 << bit)) {
            out.monitors.push_back({continuous[bit], true, (b & (1 << (bit + 4))) == 0});
        }
    }

    // Bytes C and D: C says "supported", D says "not yet complete". The bit
    // meaning follows the engine type (SAE J1979). Bits marked reserved for
    // diesel are skipped.
    static const Monitor spark[8] = {Monitor::Catalyst,      Monitor::HeatedCatalyst,
                                     Monitor::Evap,          Monitor::SecondaryAir,
                                     Monitor::AcRefrigerant, Monitor::OxygenSensor,
                                     Monitor::OxygenSensorHeater, Monitor::Egr};
    static const int none = -1;
    static const int diesel[8] = {static_cast<int>(Monitor::NmhcCatalyst),
                                  static_cast<int>(Monitor::NoxScr),
                                  none,
                                  static_cast<int>(Monitor::BoostPressure),
                                  none,
                                  static_cast<int>(Monitor::ExhaustGasSensor),
                                  static_cast<int>(Monitor::ParticulateFilter),
                                  static_cast<int>(Monitor::Egr)};
    for (int bit = 0; bit < 8; ++bit) {
        if (!(c & (1 << bit))) continue;
        Monitor m = Monitor::Misfire;
        if (out.compressionIgnition) {
            if (diesel[bit] == none) continue;
            m = static_cast<Monitor>(diesel[bit]);
        } else {
            m = spark[bit];
        }
        out.monitors.push_back({m, true, (d & (1 << bit)) == 0});
    }
    return out;
}

namespace {

std::string lower(const std::string& s) {
    std::string out = s;
    for (char& ch : out) {
        // Only ASCII letters change; the bytes of UTF-8 umlauts stay as they
        // are, so "Tankentlüftung" and "Tankentlueftung" share the prefix
        // "tankentl" either way.
        if (ch >= 'A' && ch <= 'Z') ch = static_cast<char>(ch - 'A' + 'a');
    }
    return out;
}

bool has(const std::string& haystack, const char* needle) {
    return haystack.find(needle) != std::string::npos;
}

bool matchName(const std::string& rawName, Monitor& out) {
    const std::string n = lower(rawName);
    // The more specific names come first ("heater" before "sensor",
    // "heated catalyst" before "catalyst").
    if (has(n, "aussetz") || has(n, "misfire")) { out = Monitor::Misfire; return true; }
    if (has(n, "kraftstoffsystem") || has(n, "fuel system")) { out = Monitor::FuelSystem; return true; }
    if (has(n, "bauteil") || has(n, "component")) { out = Monitor::Components; return true; }
    if (has(n, "nmhc")) { out = Monitor::NmhcCatalyst; return true; }
    if (has(n, "nox") || has(n, "scr")) { out = Monitor::NoxScr; return true; }
    if (has(n, "katalysatorheiz") || has(n, "heated catalyst")) { out = Monitor::HeatedCatalyst; return true; }
    if (has(n, "katalysator") || has(n, "catalyst")) { out = Monitor::Catalyst; return true; }
    if (has(n, "tankentl") || has(n, "evap")) { out = Monitor::Evap; return true; }
    if (has(n, "sekund") || has(n, "secondary air")) { out = Monitor::SecondaryAir; return true; }
    if (has(n, "klima") || has(n, "a/c") || has(n, "refrigerant")) { out = Monitor::AcRefrigerant; return true; }
    if (has(n, "lambdasondenheiz") || has(n, "sensor heater")) { out = Monitor::OxygenSensorHeater; return true; }
    if (has(n, "lambdasonde") || has(n, "oxygen sensor")) { out = Monitor::OxygenSensor; return true; }
    if (has(n, "abgasr") || has(n, "egr")) { out = Monitor::Egr; return true; }
    if (has(n, "ladedruck") || has(n, "boost")) { out = Monitor::BoostPressure; return true; }
    if (has(n, "abgassensor") || has(n, "exhaust gas sensor")) { out = Monitor::ExhaustGasSensor; return true; }
    if (has(n, "partikel") || has(n, "particulate") || has(n, "pm filter")) {
        out = Monitor::ParticulateFilter;
        return true;
    }
    return false;
}

}  // namespace

std::vector<MonitorState> readinessFromStatus(const VehicleStatus& status) {
    std::vector<MonitorState> out;
    for (const ReadinessItem& item : status.readiness) {
        Monitor m = Monitor::Misfire;
        if (matchName(item.name, m)) out.push_back({m, true, item.ready});
    }
    return out;
}

Text monitorName(Monitor monitor) {
    switch (monitor) {
        case Monitor::Misfire: return {"Verbrennungsaussetzer", "Misfire"};
        case Monitor::FuelSystem: return {"Kraftstoffsystem", "Fuel system"};
        case Monitor::Components: return {"Bauteilüberwachung", "Comprehensive components"};
        case Monitor::Catalyst: return {"Katalysator", "Catalyst"};
        case Monitor::HeatedCatalyst: return {"Katalysatorheizung", "Heated catalyst"};
        case Monitor::Evap: return {"Tankentlüftung", "Evaporative system"};
        case Monitor::SecondaryAir: return {"Sekundärluft", "Secondary air system"};
        case Monitor::AcRefrigerant: return {"Klimaanlage", "A/C refrigerant"};
        case Monitor::OxygenSensor: return {"Lambdasonde", "Oxygen sensor"};
        case Monitor::OxygenSensorHeater: return {"Lambdasondenheizung", "Oxygen sensor heater"};
        case Monitor::Egr: return {"Abgasrückführung / Nockenwellenverstellung", "EGR / variable valve timing"};
        case Monitor::NmhcCatalyst: return {"Oxidationskatalysator (NMHC)", "NMHC catalyst"};
        case Monitor::NoxScr: return {"Stickoxid-Nachbehandlung (NOx/SCR)", "NOx aftertreatment (SCR)"};
        case Monitor::BoostPressure: return {"Ladedruck", "Boost pressure"};
        case Monitor::ExhaustGasSensor: return {"Abgassensor", "Exhaust gas sensor"};
        case Monitor::ParticulateFilter: return {"Partikelfilter", "Particulate filter"};
    }
    return {"?", "?"};
}

bool isContinuousMonitor(Monitor monitor) {
    return monitor == Monitor::Misfire || monitor == Monitor::FuelSystem ||
           monitor == Monitor::Components;
}

}  // namespace caruspex::analysis
