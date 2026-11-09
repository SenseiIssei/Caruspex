// Trip computer with concrete drives.
#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include <string>

#include "core/analysis/TripComputer.h"

using namespace caruspex::analysis;
using Catch::Approx;

namespace {

TripConfig configFor(FuelType fuel) {
    TripConfig cfg;
    cfg.fuel = fuel;
    return cfg;
}

}  // namespace

TEST_CASE("One hour at 100 km/h with 6 l/h", "[analysis][trip]") {
    TripConfig cfg;
    cfg.fuel = FuelType::Petrol;
    cfg.fuelPricePerLitre = 1.80;
    TripComputer trip(cfg);
    for (int t = 0; t <= 3600; ++t) {
        trip.addSample(0x0D, 100, t);
        trip.addSample(0x0C, 2200, t);
        trip.addSample(0x5E, 6.0, t);
    }
    const TripSummary s = trip.summary();
    REQUIRE(s.distanceKm == Approx(100.0).margin(0.01));
    REQUIRE(s.fuelLitres == Approx(6.0).margin(0.01));
    REQUIRE(s.averageLitresPer100Km == Approx(6.0).margin(0.01));
    REQUIRE(s.liveLitresPer100Km == Approx(6.0).margin(0.01));
    REQUIRE(s.liveLitresPerHour == Approx(6.0));
    REQUIRE(s.co2GramsPerKm == Approx(142.2).margin(0.1));   // 6 l * 2.37 kg / 100 km
    REQUIRE(s.co2Kg == Approx(14.22).margin(0.01));
    REQUIRE(s.cost == Approx(10.80).margin(0.01));
    REQUIRE(s.averageSpeedKmh == Approx(100.0).margin(0.01));
    REQUIRE(s.maxSpeedKmh == 100.0);
    REQUIRE(s.idleShare == 0.0);
    REQUIRE(s.fuelSource == FuelSource::FuelRate);
    REQUIRE_FALSE(s.fuelIsRough);
    REQUIRE(s.eco.total == 100);
    REQUIRE(s.eco.harshAccelerations == 0);
}

TEST_CASE("Fuel from mass air flow and speed density", "[analysis][trip]") {
    TripComputer petrol(configFor(FuelType::Petrol));
    // 10 g/s air / 14.7 = 0.680 g/s petrol = 0.000913 l/s = 3.287 l/h
    REQUIRE(petrol.fuelRateFromMaf(10.0) == Approx(3.2873).margin(0.001));

    TripConfig sd;
    sd.displacementLitres = 2.0;
    sd.volumetricEfficiency = 0.85;
    TripComputer engine(sd);
    // 100 kPa, 300 K, 3000 rpm: 1.974 g per two turns, 25 per second.
    REQUIRE(engine.airFlowFromSpeedDensity(100.0, 26.85, 3000.0) == Approx(49.35).margin(0.05));

    // A drive on MAF only: 600 s at 50 km/h with 10 g/s.
    TripComputer maf(configFor(FuelType::Petrol));
    for (int t = 0; t <= 600; ++t) {
        maf.addSample(0x0D, 50, t);
        maf.addSample(0x10, 10.0, t);
    }
    const TripSummary s = maf.summary();
    REQUIRE(s.fuelSource == FuelSource::MassAirFlow);
    REQUIRE(s.fuelLitres == Approx(3.2873 / 6.0).margin(0.001));
    REQUIRE(s.distanceKm == Approx(50.0 / 6.0).margin(0.001));
    REQUIRE(s.averageLitresPer100Km == Approx(6.575).margin(0.01));
    REQUIRE_FALSE(s.fuelIsRough);

    // The same on a diesel is only a rough estimate and says so.
    TripComputer diesel(configFor(FuelType::Diesel));
    for (int t = 0; t <= 60; ++t) {
        diesel.addSample(0x0D, 50, t);
        diesel.addSample(0x10, 10.0, t);
    }
    const TripSummary d = diesel.summary();
    REQUIRE(d.fuelIsRough);
    REQUIRE(d.fuelNote.de.find("grobe Schätzung") != std::string::npos);
    REQUIRE(d.co2Kg == Approx(d.fuelLitres * 2.65));
}

TEST_CASE("Idling counts as l/h and idle share", "[analysis][trip]") {
    TripComputer trip;
    for (int t = 0; t <= 600; ++t) {
        trip.addSample(0x0D, 0, t);
        trip.addSample(0x0C, 800, t);
        trip.addSample(0x5E, 0.8, t);
    }
    const TripSummary s = trip.summary();
    REQUIRE(s.distanceKm == 0.0);
    REQUIRE(s.idleSeconds == Approx(600));
    REQUIRE(s.idleLitresPerHour == Approx(0.8));
    REQUIRE(s.idleShare == Approx(1.0));
    REQUIRE(s.averageLitresPer100Km == 0.0);   // no distance, no l/100 km
    REQUIRE(s.liveLitresPer100Km == 0.0);
    REQUIRE(s.eco.idling == 0);
}

TEST_CASE("Gaps are skipped, not bridged", "[analysis][trip]") {
    TripComputer trip;
    for (int t = 0; t <= 100; ++t) trip.addSample(0x0D, 100, t);
    for (int t = 200; t <= 300; ++t) trip.addSample(0x0D, 100, t);
    const TripSummary s = trip.summary();
    // 200 s at 100 km/h; the 100 s without data are not counted.
    REQUIRE(s.distanceKm == Approx(200.0 / 36.0).margin(0.001));
    REQUIRE(s.gapSeconds == Approx(100.0));
}

TEST_CASE("Glitches and jittery timestamps", "[analysis][trip]") {
    TripComputer trip;
    for (int i = 0; i <= 120; ++i) {
        const double t = i * 0.5;
        // One reading of 255 km/h out of nowhere.
        trip.addSample(0x0D, i == 60 ? 255 : 50, t);
    }
    // A reading with a timestamp slightly in the past must not count twice.
    trip.addSample(0x0D, 50, 59.8);
    const TripSummary s = trip.summary();
    REQUIRE(s.maxSpeedKmh == 50.0);
    REQUIRE(s.distanceKm == Approx(50.0 * 60.0 / 3600.0).margin(0.01));

    // A real change is kept once it repeats.
    TripComputer jump;
    jump.addSample(0x0D, 0, 0.0);
    jump.addSample(0x0D, 80, 0.5);   // impossible within half a second
    jump.addSample(0x0D, 80, 1.0);   // but it repeats: accepted
    REQUIRE(jump.summary().maxSpeedKmh == 80.0);
}

TEST_CASE("Eco score counts hard accelerations", "[analysis][trip]") {
    TripComputer trip;
    double t = 0;
    for (int round = 0; round < 3; ++round) {
        // 0 to 60 km/h in 5 s (3.3 m/s^2, hard), cruise 2 minutes, stop gently.
        for (int i = 0; i <= 50; ++i, t += 0.1) {
            trip.addSample(0x0D, i * 1.2, t);
            trip.addSample(0x0C, 3500, t);
        }
        for (int i = 0; i < 120; ++i, t += 1.0) {
            trip.addSample(0x0D, 60, t);
            trip.addSample(0x0C, 2000, t);
        }
        for (int i = 0; i <= 30; ++i, t += 1.0) {
            trip.addSample(0x0D, 60 - i * 2, t);   // 2 km/h per second, gentle
            trip.addSample(0x0C, 1200, t);
        }
    }
    const TripSummary s = trip.summary();
    REQUIRE(s.eco.harshAccelerations == 3);
    REQUIRE(s.eco.harshBrakings == 0);
    REQUIRE(s.eco.smoothness < 100);
    REQUIRE(s.eco.engineSpeed >= 0);
    REQUIRE(s.eco.total < 100);
    REQUIRE(s.eco.explanation.de.find("3 starke Beschleunigungen") != std::string::npos);
}
