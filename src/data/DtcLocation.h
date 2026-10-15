// Maps a trouble code to the spot on the vehicle where you have to look.
#pragma once

#include <string>
#include <vector>

namespace caruspex {

enum class CarZone {
    Unknown,
    Intake,        // Intake system, mass air flow sensor, intake manifold, vacuum leaks
    Throttle,      // Throttle body and accelerator pedal
    Injection,     // Injectors, fuel pressure in the engine bay
    Ignition,      // Ignition coils, spark plugs, knock sensor
    Cylinders,     // Combustion, timing drive, camshaft
    Cooling,       // Coolant, thermostat, fans
    Turbo,         // Turbocharger and boost pressure control
    Exhaust,       // Manifold, oxygen sensors, catalytic converter, EGR, particulate filter
    FuelTank,      // Tank, fuel pump, EVAP system
    Gearbox,       // Transmission
    Ecu,           // The control unit itself
    Electrics,     // Battery, alternator, electrical system
    Network,       // Data bus between the control units
    Wheels,        // Wheel speed sensors, ABS, brakes
    Cabin,         // Airbags, comfort system, instrument cluster
};

struct ZoneInfo {
    CarZone zone = CarZone::Unknown;
    std::string name;      // "Ansaugtrakt" / "Intake system", active language
    std::string hint;      // where exactly to look, active language
};

// Zone for a code like "P0171".
ZoneInfo locateDtc(const std::string& code);

// Zone name in the active language.
std::string zoneName(CarZone zone);

// All zones, in the order the drawing knows them.
const std::vector<CarZone>& allZones();

}  // namespace caruspex
