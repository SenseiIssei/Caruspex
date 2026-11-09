// One-button health check: faults, readiness, live values and mode 06
// results go in, a score from 0 to 100, a traffic light and a list of plain
// findings come out.
//
// Each rule only runs if its inputs are there. Whatever could not be checked
// is listed separately, so the result never pretends to know more than the
// car told us.
//
// The rules use widely accepted workshop values. Where practice varies
// between makes, the text says so instead of pretending precision.
#pragma once

#include <functional>
#include <map>
#include <optional>
#include <string>
#include <vector>

#include "core/Obd2.h"
#include "core/analysis/Common.h"
#include "core/analysis/Readiness.h"

namespace caruspex::analysis {

// The part of the car a finding is about.
enum class Area {
    Faults,
    Readiness,
    Mixture,
    OxygenSensors,
    Catalyst,
    Misfire,
    Cooling,
    Charging,
    Sensors,
    Monitoring,   // mode 06 results
    History,      // clearing, distance with lamp on
};

struct Finding {
    std::string id;       // stable rule id, e.g. "fuel_trim_bank1"
    Level level = Level::Info;
    Area area = Area::Faults;
    Phrase title;         // short, e.g. "Gemisch zu mager (Bank 1)"
    Phrase explanation;   // what it means, in plain words
    Phrase advice;        // what to do next
};

// A rule that could not run, with the reason (which values were missing).
struct SkippedCheck {
    std::string id;
    Phrase title;
    Phrase reason;
};

// One timestamped reading. Time in seconds on any monotonic clock.
struct TimedValue {
    double t = 0.0;
    double value = 0.0;
};

struct HealthInput {
    // Stored, pending and permanent codes as read from the car.
    std::vector<Fault> faults;
    bool faultsRead = false;   // tells "no faults" apart from "not read"
    // How serious each fault is. Supplied by the caller (from the DTC
    // database); without it every fault counts as a warning.
    std::function<Level(const Fault&)> faultLevel;

    // Warning lamp state from PID 01. Leave empty if it was not read.
    std::optional<bool> milOn;
    // Readiness monitors (decodeReadiness() or readinessFromStatus()).
    // Empty together with readinessRead == false means "not read".
    std::vector<MonitorState> readiness;
    bool readinessRead = false;

    FuelType fuel = FuelType::Unknown;

    // Live values by PID. A single snapshot is a series with one entry.
    // Values use the units of data/Pids (C, %, V, km, 1/min, g/s).
    std::map<int, std::vector<TimedValue>> live;

    // Mode 06 results. monitorResultsRead tells an empty list apart from
    // "not read".
    std::vector<MonitorResult> monitorResults;
    bool monitorResultsRead = false;

    // Fills readiness and MIL from a parsed VehicleStatus.
    void setStatus(const VehicleStatus& status);
    // Adds a snapshot of live values taken at time t.
    void addSnapshot(const std::vector<LiveValue>& values, double t);
    void addValue(int pid, double value, double t);
};

struct HealthReport {
    int score = 100;                  // 0..100
    TrafficLight grade = TrafficLight::Green;
    Phrase summary;                   // one or two sentences for the top
    std::vector<Finding> findings;    // worst first
    std::vector<SkippedCheck> notChecked;
    int checksRun = 0;

    const Finding* find(const std::string& id) const;
};

HealthReport runHealthCheck(const HealthInput& input);

// Mode 01 PIDs the rules look at, for collecting the live series. Ask only
// those the car supports (Obd2::supportedPids).
const std::vector<int>& healthLivePids();

// Fuel type from the value of PID 0x51 (SAE J1979 fuel type coding).
// Only the codes whose rules are clear are mapped; the rest is Unknown.
FuelType fuelTypeFromObd(int code);

// Thresholds used by the rules, exposed so the UI can show them and tests
// can refer to them.
namespace limits {
inline constexpr double kTrimWarn = 10.0;          // |STFT + LTFT| in %
inline constexpr double kTrimBad = 20.0;
inline constexpr double kChargingLow = 13.2;       // V, engine running
inline constexpr double kChargingNone = 12.6;      // V, engine running: not charging
inline constexpr double kChargingHigh = 15.0;      // V, engine running
inline constexpr double kChargingTooHigh = 15.5;
inline constexpr double kBatteryWeak = 12.2;       // V, engine off
inline constexpr double kBatteryFlat = 11.8;
inline constexpr double kWarmCoolant = 80.0;       // C after a long run
inline constexpr double kColdThermostat = 75.0;    // below this after a long run: stuck open
inline constexpr double kLongRunSeconds = 15 * 60; // "long run" for the thermostat rule
inline constexpr double kCoolantHot = 108.0;
inline constexpr double kCoolantOverheat = 115.0;
inline constexpr double kOilHot = 135.0;
inline constexpr double kOilOverheat = 150.0;
inline constexpr double kCatalystHot = 900.0;
inline constexpr double kNearLimitShare = 0.10;    // mode 06: within 10 % of the limit
inline constexpr int kFewWarmups = 5;              // since codes cleared
inline constexpr double kShortDistanceKm = 50.0;   // since codes cleared
}  // namespace limits

}  // namespace caruspex::analysis
