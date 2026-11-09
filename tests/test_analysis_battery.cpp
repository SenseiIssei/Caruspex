// Guided battery test: thresholds with concrete voltages.
#include <catch2/catch_test_macros.hpp>

#include <cstring>
#include <string>

#include "core/analysis/BatteryTest.h"

using namespace caruspex;
using namespace caruspex::analysis;

namespace {

const BatteryStepResult* step(const BatteryReport& r, const std::string& id) {
    for (const BatteryStepResult& s : r.steps) {
        if (s.id == id) return &s;
    }
    return nullptr;
}

Level restingLevel(double v, BatteryType type = BatteryType::Flooded) {
    BatteryInput in;
    in.type = type;
    in.restingVolts = v;
    in.minutesSinceEngineOff = 240;
    return step(evaluateBattery(in), "resting")->level;
}

Level idleLevel(double v, bool startStop = false) {
    BatteryInput in;
    in.startStop = startStop;
    in.idleVolts = v;
    return step(evaluateBattery(in), "charging_idle")->level;
}

}  // namespace

TEST_CASE("Resting voltage and state of charge", "[analysis][battery]") {
    REQUIRE(estimateStateOfCharge(12.65, BatteryType::Flooded) == 100);
    REQUIRE(estimateStateOfCharge(12.4, BatteryType::Flooded) == 75);
    REQUIRE(estimateStateOfCharge(12.3, BatteryType::Flooded) == 65);   // 62.5, in steps of 5
    REQUIRE(estimateStateOfCharge(12.2, BatteryType::Flooded) == 50);
    REQUIRE(estimateStateOfCharge(11.5, BatteryType::Flooded) == 0);
    REQUIRE(estimateStateOfCharge(12.5, BatteryType::Agm) == 75);       // AGM sits 0.1 V higher

    REQUIRE(restingLevel(12.65) == Level::Good);
    REQUIRE(restingLevel(12.45) == Level::Good);
    REQUIRE(restingLevel(12.3) == Level::Info);
    REQUIRE(restingLevel(12.0) == Level::Warning);
    REQUIRE(restingLevel(11.7) == Level::Critical);
    REQUIRE(restingLevel(12.65, BatteryType::Agm) == Level::Good);
    REQUIRE(restingLevel(12.35, BatteryType::Agm) == Level::Info);
}

TEST_CASE("Surface charge right after driving is pointed out", "[analysis][battery]") {
    BatteryInput in;
    in.restingVolts = 12.85;
    in.minutesSinceEngineOff = 5;
    const BatteryReport r = evaluateBattery(in);
    REQUIRE(r.stateOfChargePercent == 100);
    const std::string text = step(r, "resting")->explanation.de;
    REQUIRE(text.find("Oberflächenladung") != std::string::npos);
    REQUIRE(text.find("12,85 V") != std::string::npos);
}

TEST_CASE("Cranking dip", "[analysis][battery]") {
    auto crank = [](double lowest, double hz) {
        BatteryInput in;
        const double dt = 1.0 / hz;
        for (int i = 0; i < static_cast<int>(2 * hz); ++i) {
            const double t = i * dt;
            // 12.5 V, a short dip while the starter turns, then charging.
            const double v = t < 0.19 ? 12.5 : t < 0.5 ? lowest : 14.0;
            in.cranking.push_back({t, v});
        }
        return evaluateBattery(in);
    };
    const BatteryReport good = crank(10.4, 50);
    REQUIRE(step(good, "cranking")->level == Level::Good);
    REQUIRE(step(good, "cranking")->volts == 10.4);
    REQUIRE(good.crankingSampleRateHz > 40);
    REQUIRE(step(crank(9.8, 50), "cranking")->level == Level::Info);
    REQUIRE(step(crank(9.3, 50), "cranking")->level == Level::Warning);
    REQUIRE(step(crank(8.0, 50), "cranking")->level == Level::Critical);

    // At 5 readings per second the minimum may be missed; the text says so.
    const BatteryReport slow = crank(9.3, 5);
    REQUIRE(step(slow, "cranking")->explanation.en.find("real minimum") != std::string::npos);

    // A "dip" that hardly leaves the resting voltage was not captured.
    BatteryInput missed;
    missed.restingVolts = 12.6;
    missed.crankingMinimumVolts = 12.45;
    REQUIRE(step(evaluateBattery(missed), "cranking")->title.en == "No dip measured while cranking");
}

TEST_CASE("Charging voltage with and without smart charging", "[analysis][battery]") {
    REQUIRE(idleLevel(14.2) == Level::Good);
    REQUIRE(idleLevel(13.5) == Level::Info);
    REQUIRE(idleLevel(13.5, true) == Level::Good);
    REQUIRE(idleLevel(13.0) == Level::Warning);
    REQUIRE(idleLevel(13.0, true) == Level::Info);   // smart charging may pause
    REQUIRE(idleLevel(12.4) == Level::Critical);
    REQUIRE(idleLevel(14.9) == Level::Info);
    REQUIRE(idleLevel(14.9, true) == Level::Good);
    REQUIRE(idleLevel(15.2) == Level::Warning);
    REQUIRE(idleLevel(15.7) == Level::Critical);

    BatteryInput load;
    load.idleVolts = 14.2;
    load.loadedVolts = 12.9;
    const BatteryReport r = evaluateBattery(load);
    REQUIRE(step(r, "charging_load")->level == Level::Warning);
    REQUIRE(step(r, "charging_load")->explanation.de.find("1,30 V weniger") != std::string::npos);

    load.loadedVolts = 13.9;
    REQUIRE(step(evaluateBattery(load), "charging_load")->level == Level::Good);
    load.loadedVolts = 13.5;   // 0.7 V drop: still fine but noticeable
    REQUIRE(step(evaluateBattery(load), "charging_load")->level == Level::Info);
}

TEST_CASE("Overall verdict and missing steps", "[analysis][battery]") {
    BatteryInput in;
    in.restingVolts = 12.6;
    in.minutesSinceEngineOff = 120;
    in.idleVolts = 14.1;
    const BatteryReport r = evaluateBattery(in);
    REQUIRE(r.overall == Level::Good);
    REQUIRE(r.missingSteps == std::vector<std::string>{"cranking", "charging_load"});
    REQUIRE(r.summary.de.find("2 von 4 Schritten") != std::string::npos);

    const BatteryReport empty = evaluateBattery(BatteryInput{});
    REQUIRE(empty.steps.empty());
    REQUIRE(empty.missingSteps.size() == 4);

    REQUIRE(batteryTestSteps().size() == 4);
    for (const BatteryStep& s : batteryTestSteps()) {
        REQUIRE(std::strstr(s.instruction.de, "\xE2\x80\x94") == nullptr);
        REQUIRE(std::strstr(s.instruction.en, "\xE2\x80\x94") == nullptr);
    }
}
