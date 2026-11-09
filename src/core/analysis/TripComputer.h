// Trip computer: distance, fuel, consumption, CO2, cost and an eco score
// from timestamped live values.
//
// Feed it every reading as it arrives (PID, value, time in seconds). The
// computer keeps the latest value of each PID and integrates over the time
// between readings. Readings older than maxGapSeconds are treated as unknown,
// and a jump in time larger than that is a gap that is skipped instead of
// bridged, so a dropped connection does not invent kilometres.
//
// Fuel flow comes from the best source the car offers:
//   1. engine fuel rate (PID 5E), measured by the control unit
//   2. mass air flow (PID 10) divided by the stoichiometric air/fuel ratio
//   3. speed density: manifold pressure (0B), intake temperature (0F) and
//      engine speed (0C) with displacement and volumetric efficiency from the
//      caller
// The MAF and speed-density routes assume a stoichiometric mixture. That is
// close for petrol engines in normal driving, but diesels run with a lot of
// excess air, so there they give only a rough upper estimate.
#pragma once

#include <deque>
#include <map>
#include <vector>

#include "core/Obd2.h"
#include "core/analysis/Common.h"

namespace caruspex::analysis {

enum class FuelSource { None, FuelRate, MassAirFlow, SpeedDensity };

struct TripConfig {
    FuelType fuel = FuelType::Petrol;
    double displacementLitres = 0.0;    // needed for speed density only
    double volumetricEfficiency = 0.85; // assumed filling of the cylinders
    double fuelPricePerLitre = 0.0;     // 0 = no cost
    double maxGapSeconds = 5.0;         // longer pauses are gaps
    double liveWindowSeconds = 5.0;     // smoothing of the live consumption
};

struct EcoScore {
    int total = 0;          // 0..100
    int smoothness = 0;     // gentle acceleration and braking
    int engineSpeed = 0;    // staying in a sensible rpm band (-1 = not known)
    int idling = 0;         // little standing with the engine running
    int harshAccelerations = 0;
    int harshBrakings = 0;
    Phrase explanation;     // what went into it, with tips
};

struct TripSummary {
    double distanceKm = 0.0;
    double fuelLitres = 0.0;
    double drivingSeconds = 0.0;   // engine on, including stops
    double movingSeconds = 0.0;
    double idleSeconds = 0.0;      // engine running, speed 0
    double gapSeconds = 0.0;       // time skipped because data was missing

    double averageLitresPer100Km = 0.0;   // 0 if not enough distance yet
    double liveLitresPer100Km = 0.0;      // over the last few seconds, 0 when standing
    double liveLitresPerHour = 0.0;       // current fuel flow
    double idleLitresPerHour = 0.0;       // average while idling
    double co2GramsPerKm = 0.0;
    double co2Kg = 0.0;
    double cost = 0.0;

    double averageSpeedKmh = 0.0;         // distance / driving time
    double averageMovingSpeedKmh = 0.0;   // distance / moving time
    double maxSpeedKmh = 0.0;
    double idleShare = 0.0;               // 0..1 of driving time

    FuelSource fuelSource = FuelSource::None;   // the one used most
    bool fuelIsRough = false;    // MAF or speed density on a diesel, or speed density at all
    Phrase fuelNote;             // how fuel was determined and how exact it is
    EcoScore eco;
};

class TripComputer {
public:
    explicit TripComputer(TripConfig config = {});

    void addSample(int pid, double value, double tSeconds);
    void addSnapshot(const std::vector<LiveValue>& values, double tSeconds);
    void reset();

    TripSummary summary() const;
    const TripConfig& config() const { return config_; }

    // Fuel flow in l/h from mass air flow (g/s) for the configured fuel.
    double fuelRateFromMaf(double gramsPerSecond) const;
    // Air mass flow in g/s from speed density.
    double airFlowFromSpeedDensity(double mapKpa, double intakeC, double rpm) const;

    // Physical constants by fuel type.
    static double stoichiometricRatio(FuelType fuel);   // kg air per kg fuel
    static double densityKgPerLitre(FuelType fuel);
    static double co2KgPerLitre(FuelType fuel);

private:
    struct Held {
        double value = 0.0;
        double t = 0.0;
        bool valid = false;
    };
    struct WindowStep {
        double t;
        double fuel;
        double distance;
    };

    const Held* fresh(int pid, double t) const;
    double currentFuelRate(double t, FuelSource& source) const;
    void integrate(double t0, double t1);
    void trackSpeed(double kmh, double t);

    TripConfig config_;
    std::map<int, Held> held_;
    double lastTick_ = 0.0;
    bool started_ = false;

    double distanceKm_ = 0.0;
    double fuelLitres_ = 0.0;
    double drivingSeconds_ = 0.0;
    double movingSeconds_ = 0.0;
    double idleSeconds_ = 0.0;
    double idleFuel_ = 0.0;
    double gapSeconds_ = 0.0;
    double maxSpeed_ = 0.0;
    std::map<FuelSource, double> sourceSeconds_;
    std::deque<WindowStep> window_;
    double liveRate_ = 0.0;

    // Eco tracking
    double highRpmSeconds_ = 0.0;
    double rpmKnownDrivingSeconds_ = 0.0;
    int harshAccel_ = 0;
    int harshBrake_ = 0;
    bool inHarshAccel_ = false;
    bool inHarshBrake_ = false;
    double anchorT_ = 0.0;
    double anchorV_ = 0.0;
    bool anchorValid_ = false;
    double lastSpeed_ = 0.0;
    double lastSpeedT_ = 0.0;
    bool lastSpeedValid_ = false;
    double pendingJump_ = 0.0;      // a suspicious speed jump, kept until confirmed
    bool pendingJumpValid_ = false;
};

// Thresholds of the eco score.
namespace eco {
inline constexpr double kHarshAccel = 2.5;    // m/s^2, about 9 km/h per second
inline constexpr double kHarshBrake = -3.0;   // m/s^2
inline constexpr double kHighRpmPetrol = 3000.0;
inline constexpr double kHighRpmDiesel = 2500.0;
}  // namespace eco

}  // namespace caruspex::analysis
