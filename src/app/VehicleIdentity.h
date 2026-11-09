// From a VIN to something a person recognises: "Audi A2 (8Z)", the year, the
// right body for the picture and the brand for manufacturer fault codes.
#pragma once

#include <QString>

#include "app/VehicleDiagram.h"
#include "data/VehicleCatalog.h"

namespace caruspex {

struct IdentifiedVehicle {
    bool known = false;               // at least the maker is known
    QString title;                    // "Audi A2 (8Z)" or "Audi" or "Unbekannter Hersteller"
    QString subtitle;                 // year, engine types, country
    QString brand;                    // for lookupDtc(), empty if unknown
    int year = 0;
    int confidence = 0;
    BodyStyle body = BodyStyle::Hatchback;
    const VehicleGeneration* generation = nullptr;
    QString socketHint;               // where the OBD socket is
};

BodyStyle bodyStyleFor(BodyType type);

IdentifiedVehicle identifyVin(const QString& vin);

// The same for a generation picked by hand.
IdentifiedVehicle identifyGeneration(const VehicleGeneration& generation, int year);

}  // namespace caruspex
