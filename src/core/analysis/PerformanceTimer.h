// Performance timer: acceleration, quarter mile and braking from timestamped
// speed samples (PID 0D).
//
// OBD delivers the speed only a few times per second and in whole km/h. To
// get usable times anyway, crossing points are interpolated linearly between
// two samples, and the moment of setting off is extrapolated from the first
// two moving samples. With 5 to 10 samples per second the results are good
// to roughly 0.2 seconds; the report states the accuracy that fits the
// actual sample rate.
#pragma once

#include <vector>

#include "core/analysis/Common.h"

namespace caruspex::analysis {

struct SpeedSample {
    double t = 0.0;     // seconds
    double kmh = 0.0;
};

struct TimedRun {
    bool found = false;
    double seconds = 0.0;
    double startTime = 0.0;
    double endTime = 0.0;
    double distanceMeters = 0.0;   // quarter mile and braking
    double trapSpeedKmh = 0.0;     // quarter mile: speed at the finish line
    bool reliable = true;          // false if samples were more than 1 s apart
};

struct PerformanceReport {
    TimedRun zeroTo100;     // 0-100 km/h
    TimedRun eightyTo120;   // 80-120 km/h
    TimedRun zeroTo60mph;   // 0-96.56 km/h
    TimedRun quarterMile;   // 402.336 m from standstill
    TimedRun brake100To0;   // braking from 100 km/h to standstill
    double sampleRateHz = 0.0;
    double accuracySeconds = 0.0;
    int samplesUsed = 0;
    int samplesDropped = 0;
    Phrase accuracyHint;
};

inline constexpr double kQuarterMileMeters = 402.336;
inline constexpr double kSixtyMphKmh = 96.56064;

// Sorts by time, drops duplicates, impossible values and single-sample
// spikes. Returns the cleaned series.
std::vector<SpeedSample> cleanSpeedSamples(std::vector<SpeedSample> samples, int* dropped = nullptr);

// The single measurements on a cleaned series. Each returns the best
// (fastest, or for braking shortest) run in the series.
TimedRun measureFromStandstill(const std::vector<SpeedSample>& s, double targetKmh);
TimedRun measureRolling(const std::vector<SpeedSample>& s, double fromKmh, double toKmh);
TimedRun measureQuarterMile(const std::vector<SpeedSample>& s);
TimedRun measureBraking(const std::vector<SpeedSample>& s, double fromKmh);

// Everything at once, on raw samples.
PerformanceReport analyzePerformance(std::vector<SpeedSample> samples);

}  // namespace caruspex::analysis
