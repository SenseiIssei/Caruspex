// Guided battery and charging test: evaluation of the measured voltages.
//
// The UI walks the user through the steps (batteryTestSteps()) and collects
// the voltage readings, from PID 42 or the adapter's own voltage reading
// (AT RV). This module turns them into a verdict with plain explanations.
//
// Thresholds are the common workshop values for 12 V lead-acid batteries at
// room temperature. Cold batteries read lower, and cars with smart charging
// (start-stop, AGM or EFB batteries) regulate the charging voltage on
// purpose; the texts say so where it matters.
#pragma once

#include <optional>
#include <vector>

#include "core/Lang.h"
#include "core/analysis/Common.h"

namespace caruspex::analysis {

enum class BatteryType { Unknown, Flooded, Efb, Agm };

struct VoltageSample {
    double t = 0.0;      // seconds
    double volts = 0.0;
};

struct BatteryInput {
    BatteryType type = BatteryType::Unknown;
    bool startStop = false;   // car has start-stop / smart charging

    // Step 1: engine off for a while, ignition on or adapter voltage.
    std::optional<double> restingVolts;
    std::optional<double> minutesSinceEngineOff;   // to judge surface charge

    // Step 2: voltage during cranking, sampled as fast as possible.
    std::vector<VoltageSample> cranking;
    std::optional<double> crankingMinimumVolts;    // if the caller already has the minimum

    // Step 3: engine idling, no extra consumers.
    std::optional<double> idleVolts;
    // Step 4: engine idling with headlights, blower and rear window heating on.
    std::optional<double> loadedVolts;

    std::optional<double> ambientCelsius;   // optional, softens cold readings
};

struct BatteryStepResult {
    std::string id;          // "resting", "cranking", "charging_idle", "charging_load"
    Level level = Level::Info;
    double volts = 0.0;
    Phrase title;
    Phrase explanation;
    Phrase advice;
};

struct BatteryReport {
    Level overall = Level::Good;
    std::optional<int> stateOfChargePercent;   // estimate from the resting voltage
    std::vector<BatteryStepResult> steps;      // only the steps that were measured
    std::vector<std::string> missingSteps;     // ids of steps without a reading
    Phrase summary;
    double crankingSampleRateHz = 0.0;
};

// Rough state of charge from the resting voltage (lead-acid, about 20 C).
// AGM batteries sit about 0.1 V higher when full; that is taken into account.
int estimateStateOfCharge(double restingVolts, BatteryType type);

BatteryReport evaluateBattery(const BatteryInput& input);

// The instructions for the guided test, one per step, in order.
struct BatteryStep {
    const char* id;
    Text instruction;
};
const std::vector<BatteryStep>& batteryTestSteps();

namespace battery {
inline constexpr double kFull = 12.6;
inline constexpr double kThreeQuarters = 12.4;
inline constexpr double kWeak = 12.2;
inline constexpr double kFlat = 11.9;
inline constexpr double kCrankGood = 10.0;
inline constexpr double kCrankWeak = 9.6;
inline constexpr double kCrankBad = 8.5;
inline constexpr double kChargeLow = 13.8;
inline constexpr double kChargeHigh = 14.8;
inline constexpr double kChargeNone = 13.2;
inline constexpr double kChargeTooHigh = 15.0;
inline constexpr double kChargeDangerous = 15.5;
}  // namespace battery

}  // namespace caruspex::analysis
