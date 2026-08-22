// Ordnet einen Fehlercode der Stelle am Fahrzeug zu, an der man suchen muss.
#pragma once

#include <string>
#include <vector>

namespace caruspex {

enum class CarZone {
    Unknown,
    Intake,        // Ansaugtrakt, Luftmassenmesser, Saugrohr, Falschluft
    Throttle,      // Drosselklappe und Fahrpedal
    Injection,     // Einspritzventile, Kraftstoffdruck im Motorraum
    Ignition,      // Zuendspulen, Kerzen, Klopfsensor
    Cylinders,     // Verbrennung, Steuertrieb, Nockenwelle
    Cooling,       // Kuehlmittel, Thermostat, Luefter
    Turbo,         // Lader und Ladedruckregelung
    Exhaust,       // Kruemmer, Lambdasonden, Kat, AGR, Partikelfilter
    FuelTank,      // Tank, Foerderpumpe, Tankentlueftung
    Gearbox,       // Getriebe
    Ecu,           // Steuergeraet selbst
    Electrics,     // Batterie, Lichtmaschine, Bordnetz
    Network,       // Datenbus zwischen den Steuergeraeten
    Wheels,        // Raddrehzahlfuehler, ABS, Bremsen
    Cabin,         // Airbag, Komfort, Kombiinstrument
};

struct ZoneInfo {
    CarZone zone = CarZone::Unknown;
    std::string name;      // "Ansaugtrakt"
    std::string hint;      // wo man konkret hinschaut
};

// Zone zu einem Code wie "P0171".
ZoneInfo locateDtc(const std::string& code);

std::string zoneName(CarZone zone);

// Alle Zonen, in der Reihenfolge, in der die Zeichnung sie kennt.
const std::vector<CarZone>& allZones();

}  // namespace caruspex
