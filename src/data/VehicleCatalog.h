// Vehicle catalog: brands, models and model generations from about 1980 on,
// with what each generation offers for diagnosis.
//
// The data lives in data/vehicles/*.csv and is turned into
// VehicleCatalog.gen.cpp by tools/gen_vehicles.py. One row is one model
// generation (Golf IV, BMW E46, Mercedes W203). It says which diagnostic
// connector the car has, which bus and which ELM327 protocols to try, where
// the socket sits, and a short note.
//
// Diagnosis is described honestly: cars before OBD-II have either no
// readable diagnosis or a manufacturer plug that an ELM327 cannot talk to.
// VAG cars from about 1990 speak KWP1281 over a KKL cable, which Caruspex
// supports. EU petrol cars have EOBD from 2001, diesels from 2004, US cars
// OBD-II from 1996. Pure electric cars often answer OBD only in part.
//
// All texts come out in the active language (see core/Lang.h).
#pragma once

#include <cstddef>
#include <string>
#include <string_view>
#include <vector>

#include "core/Lang.h"
#include "data/Vehicles.h"
#include "data/Vin.h"

namespace caruspex {

enum class BodyType {
    Hatchback,
    Sedan,
    Estate,
    Suv,
    Van,
    Coupe,
    Convertible,
    Pickup,
    Mpv,
    Microcar,
    Roadster,
};

// Bit flags, several per generation.
enum Powertrain : unsigned {
    PtPetrol = 1u << 0,
    PtDiesel = 1u << 1,
    PtHybrid = 1u << 2,     // full or mild hybrid without a charging plug
    PtPlugIn = 1u << 3,     // plug-in hybrid or range extender
    PtElectric = 1u << 4,   // battery electric only
    PtGas = 1u << 5,        // LPG or CNG from the factory
};

enum class Connector {
    None,       // no diagnosis an adapter can read (at most blink codes)
    Obd2,       // 16-pin socket after SAE J1962 / ISO 15031-3
    Vag2x2,     // VAG round plugs (2x2 pins), KWP1281 over a KKL cable
    Bmw20,      // BMW round 20-pin plug in the engine bay
    Mb38,       // Mercedes-Benz round 38-pin plug in the engine bay
    Other,      // other manufacturer plug (GM ALDL, Ford EEC-IV, Toyota DLC1 and so on)
};

enum class SocketLocation {
    Unknown,              // not recorded for this model, the text names the usual spots
    UnderDashLeft,        // under the dashboard on the driver side, near the steering column
    UnderSteeringColumn,  // directly below the steering column
    FootwellCover,        // behind a cover in the driver's footwell
    FuseBox,              // in or beside the fuse box on the driver side
    CentreConsole,        // centre console, often behind a cover in front of the gear lever
    BehindAshtray,        // behind or under the ashtray
    PassengerSide,        // under the dashboard on the passenger side
    EngineBay,            // in the engine bay
    RearConsole,          // rear end of the centre console or under the rear seat
};

// What Caruspex can do with a car of this generation and year.
enum class ObdSupport {
    None,          // nothing to read
    Manufacturer,  // only the maker's own tester
    Kkl,           // VAG KWP1281 / KWP2000 over a KKL cable, no generic OBD
    Partial,       // OBD-II socket, but generic OBD is incomplete or not guaranteed
    Full,          // generic OBD-II / EOBD
};

struct VehicleGeneration {
    const char* brand;
    const char* model;
    const char* generation;      // "IV", "E46", "W203", "8Z"
    int yearFrom;
    int yearTo;                  // 0 = still built
    BodyType body;
    unsigned powertrains;        // Powertrain flags
    Connector connector;         // at the start of the generation
    Connector lateConnector;     // from connectorChange on
    int connectorChange;         // 0 = the connector never changed
    BusKind bus;
    const char* protocolOrder;   // ELM327 protocol numbers, empty = no OBD-II protocol
    SocketLocation location;
    Text note;                   // may be empty
    const char* vinModelCodes;   // VIN model codes, space separated ("1J", "203")
    const char* aliases;         // extra search words, '|' separated ("Golf 4|Mk4")
    bool usMarket;               // built for North America (OBD-II from 1996)
};

struct BrandInfo {
    const char* name;
    Text country;
    const char* group;   // "vag", "bmw", "mb", "ford", "gm", "chrysler" or empty
};

struct CatalogMatch {
    const VehicleGeneration* generation = nullptr;   // best guess, may be nullptr
    std::string brand;                              // known even without a generation
    int year = 0;                                   // the year the match settled on
    int confidence = 0;                             // 0 to 100
    std::vector<const VehicleGeneration*> alternatives;   // other generations that fit
    std::string reason;                             // how the guess came about
};

namespace catalog {

// The whole catalog, in the order of the CSV files.
const VehicleGeneration* all(std::size_t& count);
std::size_t size();

// Brands in alphabetical order.
std::vector<std::string> brands();
const BrandInfo* brandInfo(std::string_view brand);

// Models of a brand in alphabetical order.
std::vector<std::string> models(std::string_view brand);

// Generations of one model, oldest first.
std::vector<const VehicleGeneration*> generations(std::string_view brand, std::string_view model);

// Generation built in that year. Without a year (0) the newest one.
const VehicleGeneration* findGeneration(std::string_view brand, std::string_view model, int year);

// Best catalog guess for a decoded VIN.
CatalogMatch matchVin(const VinInfo& vin);

// Tolerant search: "golf 4", "a2", "e46", "w203", "octavia 2008", "model 3".
// Upper/lower case, umlauts and accents do not matter. A four digit number
// between 1950 and 2049 counts as a year. Best hits first.
std::vector<const VehicleGeneration*> search(std::string_view query, std::size_t limit = 20);

// ELM327 protocol order for this generation; empty if it has no OBD-II.
std::string protocolOrderFor(const VehicleGeneration& generation);

// Connector of a car of this generation in a given year (0 = first year).
Connector connectorFor(const VehicleGeneration& generation, int year);

// What diagnosis is possible for this generation in a given year
// (0 = first year) and with which engine (0 = as listed).
ObdSupport supportFor(const VehicleGeneration& generation, int year, unsigned powertrain = 0);

// True when KWP1281 / KWP2000 over a KKL cable is worth a try.
bool kklPossible(const VehicleGeneration& generation, int year);

// Display name: "Volkswagen Golf IV (1997 bis 2006)" / "... (1997-2006)".
std::string displayName(const VehicleGeneration& generation);
std::string yearsText(const VehicleGeneration& generation);

// Plain sentence about the diagnosis of this car, plus the note of the row.
std::string diagnosisText(const VehicleGeneration& generation, int year);

}  // namespace catalog

// Searches the catalog; the same as catalog::search.
inline std::vector<const VehicleGeneration*> searchCatalog(std::string_view query,
                                                           std::size_t limit = 20) {
    return catalog::search(query, limit);
}

// Texts for the enums, in the active language.
const char* bodyName(BodyType body);
const char* connectorName(Connector connector);
const char* locationText(SocketLocation location);
// Location with the connector taken into account (an unknown spot of a
// BMW round plug is still the engine bay).
std::string locationText(const VehicleGeneration& generation, int year);
std::string powertrainText(unsigned powertrains);
const char* supportName(ObdSupport support);

}  // namespace caruspex
