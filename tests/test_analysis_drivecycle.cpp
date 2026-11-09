// Readiness decoding and the drive cycle help.
#include <catch2/catch_test_macros.hpp>

#include <cstring>
#include <string>

#include "core/analysis/DriveCycleGuide.h"
#include "core/analysis/Readiness.h"

using namespace caruspex;
using namespace caruspex::analysis;

namespace {

const MonitorState* findMonitor(const std::vector<MonitorState>& list, Monitor m) {
    for (const MonitorState& s : list) {
        if (s.monitor == m) return &s;
    }
    return nullptr;
}

bool hasEmDash(const char* s) { return std::strstr(s, "\xE2\x80\x94") != nullptr; }

}  // namespace

TEST_CASE("PID 01 decodes petrol monitors", "[analysis][readiness]") {
    // Lamp on with 2 codes; misfire, fuel and components supported and
    // complete; catalyst, EVAP, O2 sensor and heater supported; EVAP open.
    const DecodedReadiness r = decodeReadiness(0x82, 0x07, 0x65, 0x04);
    REQUIRE(r.milOn);
    REQUIRE(r.storedFaults == 2);
    REQUIRE_FALSE(r.compressionIgnition);
    REQUIRE(r.monitors.size() == 7);
    REQUIRE(findMonitor(r.monitors, Monitor::Evap)->complete == false);
    REQUIRE(findMonitor(r.monitors, Monitor::Catalyst)->complete);
    REQUIRE(findMonitor(r.monitors, Monitor::OxygenSensorHeater));
    REQUIRE(findMonitor(r.monitors, Monitor::SecondaryAir) == nullptr);

    // Misfire not complete: bit 4 of byte B.
    const DecodedReadiness m = decodeReadiness(0x00, 0x17, 0x00, 0x00);
    REQUIRE(findMonitor(m.monitors, Monitor::Misfire)->complete == false);
}

TEST_CASE("PID 01 decodes diesel monitors", "[analysis][readiness]") {
    // Byte B bit 3 says compression ignition; C bits 0, 3, 5, 6, 7 are NMHC,
    // boost, exhaust sensor, particulate filter and EGR. Bit 2 is reserved
    // for diesel and must be skipped.
    const DecodedReadiness r = decodeReadiness(0x00, 0x0F, 0xED, 0x40);
    REQUIRE(r.compressionIgnition);
    REQUIRE(r.monitors.size() == 8);   // 3 continuous + 5 diesel
    REQUIRE(findMonitor(r.monitors, Monitor::NmhcCatalyst)->complete);
    REQUIRE(findMonitor(r.monitors, Monitor::BoostPressure));
    REQUIRE(findMonitor(r.monitors, Monitor::ParticulateFilter)->complete == false);
    REQUIRE(findMonitor(r.monitors, Monitor::Egr));
    REQUIRE(findMonitor(r.monitors, Monitor::Evap) == nullptr);
}

TEST_CASE("Readiness names are matched in both languages", "[analysis][readiness]") {
    VehicleStatus status;
    status.valid = true;
    status.readiness = {{"Verbrennungsaussetzer", true},
                        {"Bauteilueberwachung", true},
                        {"Tankentlüftung", false},
                        {"Tankentlueftung", false},
                        {"Katalysatorheizung", true},
                        {"Katalysator", true},
                        {"Lambdasondenheizung", true},
                        {"Lambdasonde", false},
                        {"Sekundärluft", true},
                        {"Abgasrueckfuehrung", true},
                        {"Evaporative system", false},
                        {"Oxygen sensor heater", true},
                        {"Something unknown", true}};
    const std::vector<MonitorState> m = readinessFromStatus(status);
    REQUIRE(m.size() == 12);
    REQUIRE(m[2].monitor == Monitor::Evap);
    REQUIRE(m[3].monitor == Monitor::Evap);
    REQUIRE(m[4].monitor == Monitor::HeatedCatalyst);
    REQUIRE(m[5].monitor == Monitor::Catalyst);
    REQUIRE(m[6].monitor == Monitor::OxygenSensorHeater);
    REQUIRE(m[7].monitor == Monitor::OxygenSensor);
    REQUIRE(m[7].complete == false);
    REQUIRE(m[8].monitor == Monitor::SecondaryAir);
    REQUIRE(m[9].monitor == Monitor::Egr);
    REQUIRE(m[10].monitor == Monitor::Evap);
    REQUIRE(m[11].monitor == Monitor::OxygenSensorHeater);
}

TEST_CASE("Every monitor has a complete guide in both languages", "[analysis][drivecycle]") {
    const std::vector<MonitorGuide>& guides = allDriveCycleGuides();
    REQUIRE(guides.size() == 16);
    for (const MonitorGuide& g : guides) {
        REQUIRE(&driveCycleGuide(g.monitor) == &g);
        for (const Text* t : {&g.name, &g.explanation, &g.drivePattern, &g.typicalTime}) {
            REQUIRE(std::strlen(t->de) > 0);
            REQUIRE(std::strlen(t->en) > 0);
            REQUIRE_FALSE(hasEmDash(t->de));
            REQUIRE_FALSE(hasEmDash(t->en));
        }
    }
    REQUIRE(std::string(driveCycleGuide(Monitor::Evap).name.de) == "Tankentlüftung");
    REQUIRE(std::strstr(manufacturerNote().de, "Hersteller"));
    REQUIRE(std::strstr(manufacturerNote().en, "manufacturer"));
    REQUIRE_FALSE(hasEmDash(driveCycleBasics().de));
}

TEST_CASE("Open monitors are listed with cold start ones first", "[analysis][drivecycle]") {
    const DecodedReadiness r = decodeReadiness(0x00, 0x07, 0x6D, 0x25);
    // Open: catalyst (0x01), EVAP (0x04), O2 sensor (0x20); secondary air
    // (0x08) is supported and complete.
    const std::vector<const MonitorGuide*> list = guidesForIncomplete(r.monitors);
    REQUIRE(list.size() == 3);
    REQUIRE(list[0]->monitor == Monitor::Evap);   // needs a cold start
    REQUIRE(list[1]->monitor == Monitor::Catalyst);
    REQUIRE(list[2]->monitor == Monitor::OxygenSensor);
}
