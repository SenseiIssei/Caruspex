#include "core/analysis/TripComputer.h"

#include <algorithm>
#include <cmath>
#include <set>

namespace caruspex::analysis {

namespace {

constexpr int kPidRpm = 0x0C;
constexpr int kPidSpeed = 0x0D;
constexpr int kPidMap = 0x0B;
constexpr int kPidIntakeTemp = 0x0F;
constexpr int kPidMaf = 0x10;
constexpr int kPidFuelRate = 0x5E;

constexpr double kEngineOnRpm = 300.0;
constexpr double kMovingKmh = 1.0;
constexpr double kAirGasConstant = 287.05;   // J/(kg K), dry air
constexpr double kMaxPlausibleAccel = 15.0;  // m/s^2; faster changes are glitches
constexpr double kAccelWindow = 1.0;         // s over which acceleration is judged

}  // namespace

TripComputer::TripComputer(TripConfig config) : config_(config) {
    if (config_.maxGapSeconds <= 0) config_.maxGapSeconds = 5.0;
    if (config_.liveWindowSeconds <= 0) config_.liveWindowSeconds = 5.0;
    if (config_.volumetricEfficiency <= 0) config_.volumetricEfficiency = 0.85;
}

double TripComputer::stoichiometricRatio(FuelType fuel) {
    return fuel == FuelType::Diesel ? 14.5 : 14.7;
}

double TripComputer::densityKgPerLitre(FuelType fuel) {
    return fuel == FuelType::Diesel ? 0.832 : 0.745;
}

double TripComputer::co2KgPerLitre(FuelType fuel) {
    return fuel == FuelType::Diesel ? 2.65 : 2.37;
}

double TripComputer::fuelRateFromMaf(double gramsPerSecond) const {
    // g air/s -> g fuel/s -> l fuel/s -> l/h
    const double fuelGramsPerSecond = gramsPerSecond / stoichiometricRatio(config_.fuel);
    return fuelGramsPerSecond / (densityKgPerLitre(config_.fuel) * 1000.0) * 3600.0;
}

double TripComputer::airFlowFromSpeedDensity(double mapKpa, double intakeC, double rpm) const {
    // Ideal gas: the air in one full engine displacement is p V / (R T).
    // A four-stroke engine fills its displacement once every two turns.
    const double pressurePa = mapKpa * 1000.0;
    const double volumeM3 = config_.displacementLitres / 1000.0;
    const double temperatureK = intakeC + 273.15;
    if (temperatureK <= 0) return 0.0;
    const double kgPerFill =
        pressurePa * volumeM3 * config_.volumetricEfficiency / (kAirGasConstant * temperatureK);
    return kgPerFill * 1000.0 * (rpm / 120.0);
}

void TripComputer::reset() { *this = TripComputer(config_); }

void TripComputer::addSnapshot(const std::vector<LiveValue>& values, double tSeconds) {
    // Only the first value of each PID: the primary one, see
    // HealthInput::addSnapshot.
    std::set<int> seen;
    for (const LiveValue& v : values) {
        if (seen.insert(v.pid).second) addSample(v.pid, v.value, tSeconds);
    }
}

const TripComputer::Held* TripComputer::fresh(int pid, double t) const {
    auto it = held_.find(pid);
    if (it == held_.end() || !it->second.valid) return nullptr;
    if (t - it->second.t > config_.maxGapSeconds) return nullptr;
    return &it->second;
}

double TripComputer::currentFuelRate(double t, FuelSource& source) const {
    source = FuelSource::None;
    const Held* rpm = fresh(kPidRpm, t);
    if (rpm && rpm->value < kEngineOnRpm) return 0.0;   // engine off burns nothing
    if (const Held* rate = fresh(kPidFuelRate, t)) {
        source = FuelSource::FuelRate;
        return std::max(0.0, rate->value);
    }
    if (const Held* maf = fresh(kPidMaf, t)) {
        source = FuelSource::MassAirFlow;
        return fuelRateFromMaf(std::max(0.0, maf->value));
    }
    const Held* map = fresh(kPidMap, t);
    const Held* iat = fresh(kPidIntakeTemp, t);
    if (map && iat && rpm && config_.displacementLitres > 0) {
        source = FuelSource::SpeedDensity;
        return fuelRateFromMaf(airFlowFromSpeedDensity(map->value, iat->value, rpm->value));
    }
    return 0.0;
}

void TripComputer::integrate(double t0, double t1) {
    const double dt = t1 - t0;
    if (dt <= 0) return;
    const Held* speed = fresh(kPidSpeed, t1);
    const Held* rpm = fresh(kPidRpm, t1);
    FuelSource source = FuelSource::None;
    const double rate = currentFuelRate(t1, source);
    if (!speed && !rpm && source == FuelSource::None) return;   // nothing known

    bool engineOn;
    if (rpm) engineOn = rpm->value >= kEngineOnRpm;
    else engineOn = source != FuelSource::None || speed != nullptr;

    const double v = speed ? speed->value : 0.0;
    const double dDistance = v * dt / 3600.0;
    const double dFuel = engineOn ? rate * dt / 3600.0 : 0.0;

    distanceKm_ += dDistance;
    fuelLitres_ += dFuel;
    if (engineOn) {
        drivingSeconds_ += dt;
        if (source != FuelSource::None) sourceSeconds_[source] += dt;
    }
    if (v >= kMovingKmh) movingSeconds_ += dt;
    if (engineOn && speed && v < kMovingKmh) {
        idleSeconds_ += dt;
        idleFuel_ += dFuel;
    }
    liveRate_ = engineOn ? rate : 0.0;

    window_.push_back({t1, dFuel, dDistance});
    while (!window_.empty() && window_.front().t < t1 - config_.liveWindowSeconds) {
        window_.pop_front();
    }

    if (engineOn && rpm && v >= 5.0) {
        rpmKnownDrivingSeconds_ += dt;
        const double limit =
            config_.fuel == FuelType::Diesel ? eco::kHighRpmDiesel : eco::kHighRpmPetrol;
        if (rpm->value > limit) highRpmSeconds_ += dt;
    }
}

void TripComputer::trackSpeed(double kmh, double t) {
    maxSpeed_ = std::max(maxSpeed_, kmh);
    if (!anchorValid_ || t - anchorT_ > config_.maxGapSeconds || t < anchorT_) {
        anchorT_ = t;
        anchorV_ = kmh;
        anchorValid_ = true;
        return;
    }
    // Speed comes in steps of 1 km/h. Over a fraction of a second that step
    // alone looks like a hard acceleration, so acceleration is judged over at
    // least a second.
    if (t - anchorT_ < kAccelWindow) return;
    const double a = (kmh - anchorV_) / 3.6 / (t - anchorT_);
    if (a > eco::kHarshAccel) {
        if (!inHarshAccel_) ++harshAccel_;
        inHarshAccel_ = true;
    } else {
        inHarshAccel_ = false;
    }
    if (a < eco::kHarshBrake) {
        if (!inHarshBrake_) ++harshBrake_;
        inHarshBrake_ = true;
    } else {
        inHarshBrake_ = false;
    }
    anchorT_ = t;
    anchorV_ = kmh;
}

void TripComputer::addSample(int pid, double value, double t) {
    if (!std::isfinite(value) || !std::isfinite(t)) return;

    if (pid == kPidSpeed) {
        if (value < 0 || value > 400) return;
        // A single reading that would need an impossible acceleration is a
        // transmission glitch. It is dropped, unless the new value repeats,
        // which means the speed really changed.
        if (lastSpeedValid_ && t >= lastSpeedT_ && t - lastSpeedT_ <= config_.maxGapSeconds) {
            const double dt = std::max(t - lastSpeedT_, 0.1);
            const double a = std::fabs(value - lastSpeed_) / 3.6 / dt;
            if (a > kMaxPlausibleAccel) {
                if (!pendingJumpValid_ || std::fabs(value - pendingJump_) > 5.0) {
                    pendingJump_ = value;
                    pendingJumpValid_ = true;
                    return;
                }
            }
        }
        pendingJumpValid_ = false;
    }

    if (!started_) {
        started_ = true;
        lastTick_ = t;
    } else if (t < lastTick_) {
        if (lastTick_ - t > config_.maxGapSeconds) {
            // The clock jumped back: start a new time base, keep the totals.
            for (auto& [id, h] : held_) h.valid = false;
            anchorValid_ = false;
            lastTick_ = t;
        } else {
            // Slightly out of order: use the value, but do not integrate
            // backwards in time.
            t = lastTick_;
        }
    } else {
        const double dt = t - lastTick_;
        if (dt > config_.maxGapSeconds) {
            gapSeconds_ += dt;
            anchorValid_ = false;
        } else {
            integrate(lastTick_, t);
        }
        lastTick_ = t;
    }

    held_[pid] = {value, t, true};
    if (pid == kPidSpeed) {
        lastSpeed_ = value;
        lastSpeedT_ = t;
        lastSpeedValid_ = true;
        trackSpeed(value, t);
    }
}

TripSummary TripComputer::summary() const {
    TripSummary s;
    s.distanceKm = distanceKm_;
    s.fuelLitres = fuelLitres_;
    s.drivingSeconds = drivingSeconds_;
    s.movingSeconds = movingSeconds_;
    s.idleSeconds = idleSeconds_;
    s.gapSeconds = gapSeconds_;
    s.maxSpeedKmh = maxSpeed_;

    if (distanceKm_ >= 0.1) s.averageLitresPer100Km = fuelLitres_ / distanceKm_ * 100.0;
    double wFuel = 0, wDist = 0;
    for (const WindowStep& w : window_) {
        wFuel += w.fuel;
        wDist += w.distance;
    }
    const Held* speed = fresh(kPidSpeed, lastTick_);
    if (speed && speed->value >= 5.0 && wDist >= 0.005) s.liveLitresPer100Km = wFuel / wDist * 100.0;
    s.liveLitresPerHour = liveRate_;
    if (idleSeconds_ >= 5.0) s.idleLitresPerHour = idleFuel_ / idleSeconds_ * 3600.0;

    const double co2 = co2KgPerLitre(config_.fuel);
    s.co2Kg = fuelLitres_ * co2;
    s.co2GramsPerKm = s.averageLitresPer100Km * co2 * 10.0;
    s.cost = fuelLitres_ * config_.fuelPricePerLitre;

    if (drivingSeconds_ > 0) {
        s.averageSpeedKmh = distanceKm_ / (drivingSeconds_ / 3600.0);
        s.idleShare = idleSeconds_ / drivingSeconds_;
    }
    if (movingSeconds_ > 0) s.averageMovingSpeedKmh = distanceKm_ / (movingSeconds_ / 3600.0);

    double best = 0;
    for (const auto& [source, seconds] : sourceSeconds_) {
        if (seconds > best) {
            best = seconds;
            s.fuelSource = source;
        }
    }
    const bool diesel = config_.fuel == FuelType::Diesel;
    switch (s.fuelSource) {
        case FuelSource::FuelRate:
            s.fuelNote = format("Verbrauch aus dem Kraftstoffdurchsatz, den das Steuergerät "
                                "selbst meldet. Das ist die genaueste Quelle.",
                                "Fuel use from the fuel rate the control unit reports itself. "
                                "That is the most accurate source.");
            break;
        case FuelSource::MassAirFlow:
            s.fuelIsRough = diesel;
            s.fuelNote = diesel
                ? format("Verbrauch aus dem Luftmassenmesser geschätzt. Ein Diesel läuft mit "
                         "viel Luftüberschuss, deshalb ist das nur eine grobe Schätzung, die "
                         "deutlich zu hoch liegen kann.",
                         "Fuel use estimated from the mass air flow sensor. A diesel runs with "
                         "a lot of excess air, so this is only a rough estimate that can be "
                         "well too high.")
                : format("Verbrauch aus dem Luftmassenmesser berechnet (Luft geteilt durch "
                         "14,7). Meist auf etwa 5 bis 10 % genau; beim Ausrollen mit "
                         "Schubabschaltung liegt der Wert etwas zu hoch.",
                         "Fuel use calculated from the mass air flow sensor (air divided by "
                         "14.7). Usually accurate to about 5 to 10%; when coasting with fuel "
                         "cut-off the value reads a little high.");
            break;
        case FuelSource::SpeedDensity:
            s.fuelIsRough = true;
            s.fuelNote = format("Verbrauch aus Saugrohrdruck, Ansauglufttemperatur und Drehzahl "
                                "geschätzt, mit angenommenem Füllungsgrad von {} %. Das ist eine "
                                "grobe Schätzung; sie lässt sich mit einer vollen Tankfüllung "
                                "nachjustieren.",
                                "Fuel use estimated from manifold pressure, intake temperature "
                                "and engine speed with an assumed volumetric efficiency of {}%. "
                                "That is a rough estimate; it can be calibrated against a full "
                                "tank.",
                                {number(config_.volumetricEfficiency * 100.0, 0)}) +
                         (diesel ? format("Beim Diesel liegt sie zusätzlich deutlich zu hoch.",
                                          "On a diesel it is also well too high.")
                                 : Phrase{});
            break;
        case FuelSource::None:
            s.fuelNote = format("Kein Verbrauch messbar: Es fehlen Kraftstoffdurchsatz (PID 5E), "
                                "Luftmasse (PID 10) oder Saugrohrdruck mit Hubraum.",
                                "No fuel use available: fuel rate (PID 5E), mass air flow "
                                "(PID 10) or manifold pressure with displacement are missing.");
            break;
    }

    // Eco score
    EcoScore& e = s.eco;
    e.harshAccelerations = harshAccel_;
    e.harshBrakings = harshBrake_;
    if (drivingSeconds_ < 60.0) {
        e.total = 0;
        e.smoothness = e.idling = 0;
        e.engineSpeed = -1;
        e.explanation = format("Noch zu wenig Fahrt für eine Bewertung des Fahrstils.",
                               "Not enough driving yet to rate the driving style.");
        return s;
    }
    // Smoothness: harsh events per ten minutes of moving, short trips count
    // as ten minutes so one event does not ruin them.
    const double tenMinutes = std::max(movingSeconds_ / 600.0, 1.0);
    const double eventRate = (harshAccel_ + harshBrake_) / tenMinutes;
    e.smoothness = static_cast<int>(std::lround(std::clamp(100.0 - 15.0 * eventRate, 0.0, 100.0)));
    double highShare = 0;
    if (rpmKnownDrivingSeconds_ >= 30.0) {
        highShare = highRpmSeconds_ / rpmKnownDrivingSeconds_;
        e.engineSpeed = static_cast<int>(std::lround(std::clamp(100.0 - 200.0 * highShare, 0.0, 100.0)));
    } else {
        e.engineSpeed = -1;
    }
    const double idlePercent = s.idleShare * 100.0;
    e.idling = static_cast<int>(std::lround(
        idlePercent <= 10.0 ? 100.0 : std::clamp(100.0 - 2.0 * (idlePercent - 10.0), 0.0, 100.0)));

    double weighted = 0.4 * e.smoothness + 0.3 * e.idling;
    double weights = 0.7;
    if (e.engineSpeed >= 0) {
        weighted += 0.3 * e.engineSpeed;
        weights += 0.3;
    }
    e.total = static_cast<int>(std::lround(weighted / weights));

    Phrase text = format("{} starke Beschleunigungen und {} harte Bremsungen, {} % der Zeit im "
                         "Stand mit laufendem Motor.",
                         "{} hard accelerations and {} hard brakings, {}% of the time standing "
                         "with the engine running.",
                         {number(harshAccel_, 0), number(harshBrake_, 0), number(idlePercent, 0)});
    if (e.engineSpeed >= 0) {
        text = text + format("{} % der Fahrzeit über {}/min.", "{}% of the driving time above {} rpm.",
                             {number(highShare * 100.0, 0),
                              number(config_.fuel == FuelType::Diesel ? eco::kHighRpmDiesel
                                                                      : eco::kHighRpmPetrol,
                                     0)});
    }
    // One tip for the weakest part, so the advice stays short.
    int weakest = e.smoothness;
    Phrase tip = format("Tipp: Vorausschauend fahren, früh vom Gas gehen und sanft beschleunigen "
                        "spart am meisten.",
                        "Tip: looking ahead, lifting off early and accelerating gently saves "
                        "the most.");
    if (e.engineSpeed >= 0 && e.engineSpeed < weakest) {
        weakest = e.engineSpeed;
        tip = format("Tipp: Früher hochschalten. Die meisten Motoren arbeiten zwischen 1500 und "
                     "2500/min am sparsamsten.",
                     "Tip: shift up earlier. Most engines are most efficient between 1500 and "
                     "2500 rpm.");
    }
    if (e.idling < weakest) {
        tip = format("Tipp: Bei längeren Wartezeiten den Motor ausschalten. Im Leerlauf "
                     "verbraucht das Auto Kraftstoff, ohne voranzukommen.",
                     "Tip: switch the engine off when waiting longer. At idle the car burns fuel "
                     "without getting anywhere.");
    }
    if (e.total >= 85) {
        tip = format("Sehr ruhiger, sparsamer Fahrstil.", "Very calm, economical driving style.");
    }
    e.explanation = text + tip;
    return s;
}

}  // namespace caruspex::analysis
