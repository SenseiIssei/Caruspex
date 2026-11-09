// Performance timer on synthetic runs with known answers.
#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include <algorithm>
#include <cmath>
#include <string>
#include <utility>

#include "core/analysis/PerformanceTimer.h"

using namespace caruspex::analysis;
using Catch::Approx;

namespace {

// Standing for a second, then a constant 10 km/h per second (2.78 m/s^2)
// up to 200 km/h, sampled at the given rate.
std::vector<SpeedSample> steadyLaunch(double hz, bool wholeKmh = false) {
    std::vector<SpeedSample> s;
    const double dt = 1.0 / hz;
    const int standing = static_cast<int>(std::lround(hz));
    for (int i = -standing; i <= 0; ++i) s.push_back({i * dt, 0.0});
    for (int i = 1; i * dt <= 20.0 + 1e-9; ++i) {
        const double t = i * dt;
        double v = 10.0 * t;
        if (wholeKmh) v = std::floor(v);   // OBD reports whole km/h
        s.push_back({t, v});
    }
    return s;
}

}  // namespace

TEST_CASE("0-100, 0-60 mph, 80-120 and quarter mile on a steady launch", "[analysis][performance]") {
    const PerformanceReport r = analyzePerformance(steadyLaunch(10.0));
    REQUIRE(r.zeroTo100.found);
    REQUIRE(r.zeroTo100.seconds == Approx(10.0).margin(0.01));
    REQUIRE(r.zeroTo100.reliable);
    REQUIRE(r.zeroTo60mph.seconds == Approx(9.656).margin(0.01));
    REQUIRE(r.eightyTo120.seconds == Approx(4.0).margin(0.01));
    // Distance 0.5 * a * t^2 with a = 2.778 m/s^2 reaches 402.3 m after 17.02 s.
    REQUIRE(r.quarterMile.found);
    REQUIRE(r.quarterMile.seconds == Approx(17.02).margin(0.02));
    REQUIRE(r.quarterMile.trapSpeedKmh == Approx(170.2).margin(0.2));
    REQUIRE(r.sampleRateHz == Approx(10.0).margin(0.01));
    REQUIRE(r.accuracySeconds == Approx(0.2));
    REQUIRE_FALSE(r.brake100To0.found);
}

TEST_CASE("Whole km/h at 5 Hz stays within the stated accuracy", "[analysis][performance]") {
    const PerformanceReport r = analyzePerformance(steadyLaunch(5.0, true));
    REQUIRE(r.zeroTo100.found);
    REQUIRE(std::fabs(r.zeroTo100.seconds - 10.0) <= r.accuracySeconds);
    REQUIRE(std::fabs(r.eightyTo120.seconds - 4.0) <= r.accuracySeconds);
    REQUIRE(r.accuracyHint.de.find("0,2 s") != std::string::npos);
}

TEST_CASE("Braking from 100 km/h", "[analysis][performance]") {
    // 110 km/h cruise, then 10 m/s^2 (36 km/h per second) to a standstill.
    // From 100 km/h (27.78 m/s) that takes 2.78 s and 38.6 m.
    std::vector<SpeedSample> s;
    for (int i = 0; i <= 10; ++i) s.push_back({i * 0.1, 110.0});
    const double brakeStart = 1.0;
    for (int i = 1; i <= 50; ++i) {
        const double t = brakeStart + i * 0.1;
        s.push_back({t, std::max(0.0, 110.0 - 36.0 * (t - brakeStart))});
    }
    const PerformanceReport r = analyzePerformance(s);
    REQUIRE(r.brake100To0.found);
    REQUIRE(r.brake100To0.seconds == Approx(2.78).margin(0.1));
    REQUIRE(r.brake100To0.distanceMeters == Approx(38.6).margin(0.6));
    REQUIRE_FALSE(r.zeroTo100.found);
}

TEST_CASE("Runs that do not finish are not reported", "[analysis][performance]") {
    // Up to 70 km/h and back to a stop: no 0-100, no braking from 100.
    std::vector<SpeedSample> s = {{0, 0}, {1, 0}};
    for (int i = 1; i <= 7; ++i) s.push_back({1.0 + i, i * 10.0});
    for (int i = 1; i <= 7; ++i) s.push_back({8.0 + i, 70.0 - i * 10.0});
    const PerformanceReport r = analyzePerformance(s);
    REQUIRE_FALSE(r.zeroTo100.found);
    REQUIRE_FALSE(r.quarterMile.found);
    REQUIRE_FALSE(r.brake100To0.found);
    REQUIRE_FALSE(r.eightyTo120.found);
}

TEST_CASE("Spikes, duplicates and unsorted samples are cleaned", "[analysis][performance]") {
    std::vector<SpeedSample> s = steadyLaunch(10.0);
    s[40].kmh = 250.0;                // a single glitch at 3 s
    s.push_back(s[70]);               // a duplicate
    std::swap(s[20], s[25]);          // out of order
    s.push_back({-5.0, NAN});
    int dropped = 0;
    const std::vector<SpeedSample> clean = cleanSpeedSamples(s, &dropped);
    REQUIRE(dropped == 3);
    for (size_t i = 1; i < clean.size(); ++i) REQUIRE(clean[i].t > clean[i - 1].t);

    const PerformanceReport r = analyzePerformance(s);
    REQUIRE(r.zeroTo100.seconds == Approx(10.0).margin(0.01));
}

TEST_CASE("Slow polling is called out", "[analysis][performance]") {
    const PerformanceReport r = analyzePerformance(steadyLaunch(2.0));
    REQUIRE(r.sampleRateHz == Approx(2.0).margin(0.01));
    REQUIRE(r.accuracySeconds == Approx(0.5));
    REQUIRE(r.accuracyHint.de.find("mindestens 5") != std::string::npos);
    REQUIRE(r.accuracyHint.en.find("at least 5") != std::string::npos);
}
