#include "core/analysis/PerformanceTimer.h"

#include <algorithm>
#include <cmath>

namespace caruspex::analysis {

namespace {

constexpr double kStandstill = 0.5;      // km/h; below this the car stands
constexpr double kSpike = 20.0;          // km/h jump that is undone by the next sample
constexpr double kReliableGap = 1.0;     // s; larger sample gaps make a run unreliable
constexpr double kDropTolerance = 2.0;   // km/h a rolling run may sag below its start
constexpr double kBrakeRebound = 3.0;    // km/h of speed gain that ends a braking run

double interpolateTime(const SpeedSample& a, const SpeedSample& b, double kmh) {
    if (b.kmh == a.kmh) return b.t;
    return a.t + (kmh - a.kmh) / (b.kmh - a.kmh) * (b.t - a.t);
}

// Better run: reliable before unreliable, then faster (or shorter).
bool better(const TimedRun& candidate, const TimedRun& best, bool byDistance) {
    if (!candidate.found) return false;
    if (!best.found) return true;
    if (candidate.reliable != best.reliable) return candidate.reliable;
    return byDistance ? candidate.distanceMeters < best.distanceMeters
                      : candidate.seconds < best.seconds;
}

// The moment the car set off. i is the last standing sample, i + 1 the first
// moving one. The start lies somewhere in between; a straight line through
// the first two moving samples, followed back to zero, gives a good guess.
double launchTime(const std::vector<SpeedSample>& s, size_t i) {
    const SpeedSample& stand = s[i];
    const SpeedSample& m1 = s[i + 1];
    if (i + 2 < s.size()) {
        const SpeedSample& m2 = s[i + 2];
        if (m2.kmh > m1.kmh && m2.t > m1.t) {
            const double slope = (m2.kmh - m1.kmh) / (m2.t - m1.t);
            const double t0 = m1.t - m1.kmh / slope;
            return std::clamp(t0, stand.t, m1.t);
        }
    }
    return stand.t;
}

}  // namespace

std::vector<SpeedSample> cleanSpeedSamples(std::vector<SpeedSample> samples, int* dropped) {
    const size_t before = samples.size();
    samples.erase(std::remove_if(samples.begin(), samples.end(),
                                 [](const SpeedSample& x) {
                                     return !std::isfinite(x.t) || !std::isfinite(x.kmh) ||
                                            x.kmh < 0 || x.kmh > 400;
                                 }),
                  samples.end());
    std::stable_sort(samples.begin(), samples.end(),
                     [](const SpeedSample& a, const SpeedSample& b) { return a.t < b.t; });
    // Equal timestamps: keep the later reading.
    std::vector<SpeedSample> unique;
    for (const SpeedSample& x : samples) {
        if (!unique.empty() && unique.back().t == x.t) unique.back() = x;
        else unique.push_back(x);
    }
    // A single reading far off both neighbours, in opposite directions, is
    // a transmission glitch.
    std::vector<SpeedSample> out;
    for (size_t i = 0; i < unique.size(); ++i) {
        if (i > 0 && i + 1 < unique.size()) {
            const double up = unique[i].kmh - unique[i - 1].kmh;
            const double down = unique[i + 1].kmh - unique[i].kmh;
            if (std::fabs(up) > kSpike && std::fabs(down) > kSpike && up * down < 0) continue;
        }
        out.push_back(unique[i]);
    }
    if (dropped) *dropped = static_cast<int>(before - out.size());
    return out;
}

TimedRun measureFromStandstill(const std::vector<SpeedSample>& s, double targetKmh) {
    TimedRun best;
    for (size_t i = 0; i + 1 < s.size(); ++i) {
        if (s[i].kmh >= kStandstill || s[i + 1].kmh < kStandstill) continue;
        TimedRun run;
        run.startTime = launchTime(s, i);
        double maxGap = s[i + 1].t - s[i].t;
        for (size_t j = i + 1; j < s.size(); ++j) {
            maxGap = std::max(maxGap, s[j].t - s[j - 1].t);
            if (s[j].kmh < kStandstill) break;   // stopped again before the target
            if (s[j].kmh >= targetKmh) {
                run.endTime = interpolateTime(s[j - 1], s[j], targetKmh);
                run.seconds = run.endTime - run.startTime;
                run.found = run.seconds > 0;
                run.reliable = maxGap <= kReliableGap;
                break;
            }
        }
        if (better(run, best, false)) best = run;
    }
    return best;
}

TimedRun measureRolling(const std::vector<SpeedSample>& s, double fromKmh, double toKmh) {
    TimedRun best;
    for (size_t i = 1; i < s.size(); ++i) {
        if (!(s[i - 1].kmh < fromKmh && s[i].kmh >= fromKmh)) continue;
        TimedRun run;
        run.startTime = interpolateTime(s[i - 1], s[i], fromKmh);
        double maxGap = s[i].t - s[i - 1].t;
        for (size_t j = i; j < s.size(); ++j) {
            if (j > i) maxGap = std::max(maxGap, s[j].t - s[j - 1].t);
            if (s[j].kmh < fromKmh - kDropTolerance) break;   // lifted off, no run
            if (s[j].kmh >= toKmh) {
                run.endTime = j == i ? run.startTime : interpolateTime(s[j - 1], s[j], toKmh);
                run.seconds = run.endTime - run.startTime;
                run.found = run.seconds > 0;
                run.reliable = maxGap <= kReliableGap;
                break;
            }
        }
        if (better(run, best, false)) best = run;
    }
    return best;
}

TimedRun measureQuarterMile(const std::vector<SpeedSample>& s) {
    TimedRun best;
    for (size_t i = 0; i + 1 < s.size(); ++i) {
        if (s[i].kmh >= kStandstill || s[i + 1].kmh < kStandstill) continue;
        TimedRun run;
        run.startTime = launchTime(s, i);
        // From the launch moment the speed rises linearly to the first
        // moving sample; afterwards trapezoids between samples.
        SpeedSample prev{run.startTime, 0.0};
        double distance = 0.0;
        double maxGap = s[i + 1].t - s[i].t;
        for (size_t j = i + 1; j < s.size(); ++j) {
            const SpeedSample& cur = s[j];
            if (j > i + 1) maxGap = std::max(maxGap, cur.t - s[j - 1].t);
            if (cur.kmh < kStandstill) break;
            const double dt = cur.t - prev.t;
            if (dt <= 0) continue;
            const double va = prev.kmh / 3.6, vb = cur.kmh / 3.6;
            const double segment = (va + vb) / 2.0 * dt;
            if (distance + segment >= kQuarterMileMeters) {
                // Solve for the moment inside the segment where the
                // distance is reached, with the speed rising linearly.
                const double remaining = kQuarterMileMeters - distance;
                const double a = (vb - va) / dt;
                double tau;
                if (std::fabs(a) < 1e-9) {
                    tau = remaining / va;
                } else {
                    tau = (-va + std::sqrt(std::max(0.0, va * va + 2.0 * a * remaining))) / a;
                }
                run.endTime = prev.t + tau;
                run.seconds = run.endTime - run.startTime;
                run.trapSpeedKmh = (va + a * tau) * 3.6;
                run.distanceMeters = kQuarterMileMeters;
                run.found = true;
                run.reliable = maxGap <= kReliableGap;
                break;
            }
            distance += segment;
            prev = cur;
        }
        if (better(run, best, false)) best = run;
    }
    return best;
}

TimedRun measureBraking(const std::vector<SpeedSample>& s, double fromKmh) {
    TimedRun best;
    for (size_t i = 1; i < s.size(); ++i) {
        if (!(s[i - 1].kmh >= fromKmh && s[i].kmh < fromKmh)) continue;
        TimedRun run;
        run.startTime = interpolateTime(s[i - 1], s[i], fromKmh);
        SpeedSample prev{run.startTime, fromKmh};
        double distance = 0.0;
        double lowest = fromKmh;
        double maxGap = s[i].t - s[i - 1].t;
        for (size_t j = i; j < s.size(); ++j) {
            const SpeedSample& cur = s[j];
            if (j > i) maxGap = std::max(maxGap, cur.t - s[j - 1].t);
            if (cur.kmh > lowest + kBrakeRebound) break;   // braking was interrupted
            lowest = std::min(lowest, cur.kmh);
            const double dt = cur.t - prev.t;
            if (dt > 0) distance += (prev.kmh + cur.kmh) / 2.0 / 3.6 * dt;
            if (cur.kmh < kStandstill) {
                run.endTime = cur.t;
                run.seconds = run.endTime - run.startTime;
                run.distanceMeters = distance;
                run.found = run.seconds > 0;
                run.reliable = maxGap <= kReliableGap;
                break;
            }
            prev = cur;
        }
        if (better(run, best, true)) best = run;
    }
    return best;
}

PerformanceReport analyzePerformance(std::vector<SpeedSample> samples) {
    PerformanceReport r;
    const std::vector<SpeedSample> s = cleanSpeedSamples(std::move(samples), &r.samplesDropped);
    r.samplesUsed = static_cast<int>(s.size());

    if (s.size() >= 2) {
        std::vector<double> gaps;
        for (size_t i = 1; i < s.size(); ++i) gaps.push_back(s[i].t - s[i - 1].t);
        std::nth_element(gaps.begin(), gaps.begin() + gaps.size() / 2, gaps.end());
        const double medianGap = gaps[gaps.size() / 2];
        if (medianGap > 0) r.sampleRateHz = 1.0 / medianGap;
        // Interpolation keeps the error at each end below half a sample
        // interval in theory; whole-km/h steps and adapter jitter add to it.
        // In practice about 0.2 s at 5 to 10 Hz, one interval when slower.
        r.accuracySeconds = std::max(0.2, medianGap);
    }

    r.zeroTo100 = measureFromStandstill(s, 100.0);
    r.zeroTo60mph = measureFromStandstill(s, kSixtyMphKmh);
    r.eightyTo120 = measureRolling(s, 80.0, 120.0);
    r.quarterMile = measureQuarterMile(s);
    r.brake100To0 = measureBraking(s, 100.0);

    r.accuracyHint = format(
        "{} Messwerte pro Sekunde, die Zeiten sind damit auf etwa plus/minus {} s genau. Die "
        "Geschwindigkeit über OBD stammt von den Raddrehzahlfühlern und ist in ganzen km/h "
        "gestuft; durchdrehende oder blockierende Räder verfälschen sie. Bitte nur dort messen, "
        "wo es erlaubt und sicher ist, etwa auf einer abgesperrten Strecke.",
        "{} samples per second, so the times are good to about plus/minus {} s. The speed via "
        "OBD comes from the wheel speed sensors and is stepped in whole km/h; spinning or "
        "locked wheels distort it. Please only measure where it is legal and safe, such as on "
        "a closed track.",
        {number(r.sampleRateHz, 1), number(r.accuracySeconds, 1)});
    if (r.sampleRateHz > 0 && r.sampleRateHz < 4.0) {
        r.accuracyHint = r.accuracyHint +
            format("Für brauchbare Zeiten sollten es mindestens 5 Messwerte pro Sekunde sein. "
                   "Dafür während der Messung nur die Geschwindigkeit abfragen.",
                   "Usable times need at least 5 samples per second. For that, poll only the "
                   "speed during the measurement.");
    }
    return r;
}

}  // namespace caruspex::analysis
