// Readiness monitors (mode 01, PID 01) as a fixed list of identifiers.
//
// The control unit runs a set of self-tests ("monitors") while you drive.
// PID 01 tells for each one whether the car has it at all and whether it has
// finished since the fault memory was last cleared. The meaning of the bits
// depends on the engine type: from byte B bit 3 on, petrol and diesel engines
// use the same bits for different monitors.
//
// core/Obd2.h hands out the readiness as named items in the active language.
// The evaluation needs to know which monitor is which, independent of the
// language, so this module offers two ways in: decoding the raw bytes (exact,
// covers diesel as well) and matching the names of VehicleStatus (a fallback
// for code that only has the parsed status).
#pragma once

#include <vector>

#include "core/Lang.h"
#include "core/Obd2.h"

namespace caruspex::analysis {

enum class Monitor {
    // Continuous monitors, both engine types.
    Misfire,
    FuelSystem,
    Components,
    // Petrol engines (spark ignition).
    Catalyst,
    HeatedCatalyst,
    Evap,
    SecondaryAir,
    AcRefrigerant,
    OxygenSensor,
    OxygenSensorHeater,
    Egr,          // EGR and/or variable valve timing, both engine types
    // Diesel engines (compression ignition).
    NmhcCatalyst,
    NoxScr,
    BoostPressure,
    ExhaustGasSensor,
    ParticulateFilter,
};

struct MonitorState {
    Monitor monitor = Monitor::Misfire;
    bool supported = true;   // the car has this monitor
    bool complete = false;   // finished since the last clearing
};

struct DecodedReadiness {
    bool milOn = false;
    int storedFaults = 0;
    bool compressionIgnition = false;   // diesel
    std::vector<MonitorState> monitors; // only the supported ones
};

// Decodes the four data bytes A, B, C, D of PID 01 (without the 41 01 echo).
DecodedReadiness decodeReadiness(int a, int b, int c, int d);

// Maps the named readiness items of VehicleStatus to monitors. Matches
// German (with or without umlauts) and English names. Items that cannot be
// matched are left out.
std::vector<MonitorState> readinessFromStatus(const VehicleStatus& status);

// Plain name of a monitor in both languages, e.g. {"Tankentlüftung",
// "Evaporative system"}.
Text monitorName(Monitor monitor);

// True for the three monitors that run all the time and usually report
// complete within seconds of starting the engine.
bool isContinuousMonitor(Monitor monitor);

}  // namespace caruspex::analysis
