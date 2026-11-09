#include "data/VagFaults.h"

#include <cstdio>
#include <unordered_map>
#include <vector>

#include "core/Lang.h"
#include "data/DtcDatabase.h"

namespace caruspex {
namespace {

// From this number on they are converted EOBD codes.
constexpr int kPCodeOffset = 16384;

// Texts are stored in both languages and resolved only at lookup time.
using Table = std::unordered_map<int, Text>;

// Numbers that mean the same in every control unit: power supply, coding and
// the messages about bus partners that have dropped out.
const Table kShared = {
    {532, {"Spannungsversorgung Klemme 30", "Power supply terminal 30"}},
    {668, {"Bordspannung Klemme 30: Spannung zu klein",
           "Battery voltage terminal 30: voltage too low"}},
    {1044, {"Steuergerät falsch codiert", "Control unit incorrectly coded"}},
    {1312, {"Datenbus Antrieb: gestört", "Drivetrain data bus: malfunction"}},
    {1314, {"Motorsteuergerät: keine Verbindung", "Engine control unit: no communication"}},
    {1316, {"Bremsensteuergerät: keine Verbindung", "Brake control unit: no communication"}},
    {1317, {"Kombiinstrument: keine Verbindung", "Instrument cluster: no communication"}},
    {1321, {"Airbag-Steuergerät: keine Verbindung", "Airbag control unit: no communication"}},
    {1304, {"Radio: keine Verbindung", "Radio: no communication"}},
    {4096, {"Steuergerät defekt", "Control unit faulty"}},
    {8213, {"Steuergerät falsch codiert", "Control unit incorrectly coded"}},
    {65535, {"Interner Steuergerätefehler", "Internal control unit fault"}},
};

// --- Address 01, engine ------------------------------------------------------
// This is the area that yields the most on both A2 variants: the 1.4 MPI as
// well as the 1.6 FSI report their sensors and actuators through it.
const Table kEngine = {
    {281, {"Geber für Geschwindigkeit G22", "Vehicle speed sensor G22"}},
    {513, {"Motordrehzahlgeber G28", "Engine speed sensor G28"}},
    {514, {"Nockenwellengeber G40", "Camshaft position sensor G40"}},
    {515, {"Hallgeber G40", "Hall sensor G40"}},
    {516, {"Leerlaufschalter F60", "Idle switch F60"}},
    {517, {"Vollastschalter F81", "Full throttle switch F81"}},
    {518, {"Drosselklappenpotentiometer G69", "Throttle position sensor G69"}},
    {519, {"Saugrohrdruckgeber", "Intake manifold pressure sensor"}},
    {522, {"Kühlmitteltemperaturgeber G62", "Coolant temperature sensor G62"}},
    {523, {"Ansauglufttemperaturgeber G72", "Intake air temperature sensor G72"}},
    {524, {"Klopfsensor 1 G61", "Knock sensor 1 G61"}},
    {525, {"Lambdasonde G39", "Oxygen sensor G39"}},
    {526, {"Bremslichtschalter F", "Brake light switch F"}},
    {527, {"Ansauglufttemperaturgeber G42", "Intake air temperature sensor G42"}},
    {528, {"Höhengeber F96", "Altitude sensor F96"}},
    {530, {"Drosselklappenpotentiometer G88", "Throttle position sensor G88"}},
    {531, {"Saugrohrdruck: Steuergerät defekt", "Intake manifold pressure: control unit faulty"}},
    {533, {"Leerlaufregelung: Regelgrenze erreicht", "Idle speed control: limit reached"}},
    {535, {"Klopfsensor 2 G66", "Knock sensor 2 G66"}},
    {537, {"Lambdaregelung: Regelgrenze erreicht", "Oxygen sensor control: limit reached"}},
    {539, {"Kraftstoffdruckgeber", "Fuel pressure sensor"}},
    {540, {"Klimakompressor abgeschaltet", "A/C compressor switched off"}},
    {543, {"Höchstzulässige Motordrehzahl überschritten",
           "Maximum permissible engine speed exceeded"}},
    {545, {"Kupplungsschalter", "Clutch switch"}},
    {546, {"Datenbus Antrieb: keine Botschaft", "Drivetrain data bus: no message"}},
    {549, {"Kraftstoffverbrauchssignal", "Fuel consumption signal"}},
    {550, {"Motorlastsignal", "Engine load signal"}},
    {552, {"Luftmassenmesser G70: Signal zu klein", "Mass air flow sensor G70: signal too low"}},
    {553, {"Luftmassenmesser G70", "Mass air flow sensor G70"}},
    {555, {"Lambdaregelung: Regelgrenze unterschritten",
           "Oxygen sensor control: below lower limit"}},
    {558, {"Einspritzventile: Regelabweichung", "Injectors: control deviation"}},
    {560, {"Lambdasonde: Steuerung", "Oxygen sensor: control"}},
    {561, {"Gemischadaption: Regelgrenze überschritten", "Mixture adaptation: limit exceeded"}},
    {575, {"Saugrohrdruck: Regelgrenze", "Intake manifold pressure: control limit"}},
    {577, {"Klopfregelung Zylinder 1", "Knock control cylinder 1"}},
    {578, {"Klopfregelung Zylinder 2", "Knock control cylinder 2"}},
    {579, {"Klopfregelung Zylinder 3", "Knock control cylinder 3"}},
    {580, {"Klopfregelung Zylinder 4", "Knock control cylinder 4"}},
    {585, {"AGR-Temperaturgeber", "EGR temperature sensor"}},
    {586, {"Abgasrückführung", "Exhaust gas recirculation"}},
    {587, {"Magnetventil für Abgasrückführung N18", "EGR solenoid valve N18"}},
    {609, {"Zündsignal: Endstufe", "Ignition signal: output stage"}},
    {624, {"Klimaanlage: Signal", "Air conditioning: signal"}},
    {625, {"Geschwindigkeitssignal", "Vehicle speed signal"}},
    {635, {"Lambdasondenheizung Z19", "Oxygen sensor heater Z19"}},
    {640, {"Drosselklappenpotentiometer: Signal unplausibel",
           "Throttle position sensor: implausible signal"}},
    {652, {"Lambdasonde nach Katalysator G130", "Oxygen sensor after catalytic converter G130"}},
    {660, {"Ventil für Nockenwellenverstellung N205", "Camshaft adjustment valve N205"}},
    {670, {"Leerlaufsteller: Regelgrenze", "Idle actuator: control limit"}},
    {750, {"Fehlerlampe K83", "Malfunction indicator lamp K83"}},
    {1165, {"Drosselklappensteuerung: Regelbereich", "Throttle body control: control range"}},
    {1177, {"Steuergerät: Anpassung", "Control unit: adaptation"}},
    {1247, {"Magnetventil für Aktivkohlebehälter N80", "EVAP canister purge valve N80"}},
    {1249, {"Einspritzventil Zylinder 1 N30", "Injector cylinder 1 N30"}},
    {1250, {"Einspritzventil Zylinder 2 N31", "Injector cylinder 2 N31"}},
    {1251, {"Einspritzventil Zylinder 3 N32", "Injector cylinder 3 N32"}},
    {1252, {"Einspritzventil Zylinder 4 N33", "Injector cylinder 4 N33"}},
    {1253, {"Einspritzventil Zylinder 5 N83", "Injector cylinder 5 N83"}},
    {1254, {"Einspritzventil Zylinder 6 N84", "Injector cylinder 6 N84"}},
    {1262, {"Magnetventil für Ladedruckbegrenzung N75", "Boost pressure control valve N75"}},
    {1265, {"Ventil für Abgasrückführung N18", "EGR valve N18"}},
    {1266, {"Ventil für Saugrohrumschaltung N156", "Intake manifold change-over valve N156"}},
    {1268, {"Mengensteller", "Fuel quantity adjuster"}},
    {1271, {"Drosselklappensteuereinheit J338", "Throttle body J338"}},
    {1276, {"Bremslichtschalter F", "Brake light switch F"}},
    {1289, {"Wegfahrsperre: keine Freigabe", "Immobilizer: not enabled"}},
    {1393, {"Steuergerät: Endstufe defekt", "Control unit: output stage faulty"}},
    {1500, {"Kraftstoffpumpenrelais J17", "Fuel pump relay J17"}},
    {2142, {"Kombiinstrument: Datenbus", "Instrument cluster: data bus"}},
    {2564, {"Klimasteuergerät: keine Verbindung", "Climate control unit: no communication"}},
};

// --- Address 03, brake electronics -------------------------------------------
// For the A2 the most important area besides the engine, because ABS faults
// come out there as VAG numbers and not as C-codes.
const Table kBrakes = {
    {283, {"Raddrehzahlfühler vorne links G47", "Wheel speed sensor front left G47"}},
    {285, {"Raddrehzahlfühler vorne rechts G45", "Wheel speed sensor front right G45"}},
    {287, {"Raddrehzahlfühler hinten rechts G44", "Wheel speed sensor rear right G44"}},
    {290, {"Raddrehzahlfühler hinten links G46", "Wheel speed sensor rear left G46"}},
    {778, {"Lenkwinkelgeber G85", "Steering angle sensor G85"}},
    {1130, {"ABS-Betrieb: Regelung dauerhaft aktiv", "ABS operation: control permanently active"}},
    {1201, {"Hydraulikpumpe für ABS V64", "ABS hydraulic pump V64"}},
    {1202, {"Magnetventile für ABS", "ABS solenoid valves"}},
    {1203, {"Einlassventil vorne links N101", "Inlet valve front left N101"}},
    {1204, {"Auslassventil vorne links N102", "Outlet valve front left N102"}},
    {1205, {"Einlassventil vorne rechts N99", "Inlet valve front right N99"}},
    {1206, {"Auslassventil vorne rechts N100", "Outlet valve front right N100"}},
    {1276, {"Hydraulikpumpe für ABS V64: Stromkreis", "ABS hydraulic pump V64: circuit"}},
    {1279, {"Längsbeschleunigungsgeber G251", "Longitudinal acceleration sensor G251"}},
    {1423, {"Querbeschleunigungsgeber G200", "Lateral acceleration sensor G200"}},
    {1435, {"Bremsdruckgeber 1 G201", "Brake pressure sensor 1 G201"}},
};

// --- Address 15, airbag ------------------------------------------------------
const Table kAirbag = {
    {588, {"Zünder für Fahrerairbag N95", "Driver airbag igniter N95"}},
    {589, {"Zünder für Beifahrerairbag N131", "Passenger airbag igniter N131"}},
    {591, {"Zünder für Gurtstraffer Fahrerseite N153", "Belt tensioner igniter, driver side N153"}},
    {592, {"Zünder für Gurtstraffer Beifahrerseite N154",
           "Belt tensioner igniter, passenger side N154"}},
    {595, {"Seitenairbag Fahrerseite N199", "Side airbag, driver side N199"}},
    {596, {"Seitenairbag Beifahrerseite N200", "Side airbag, passenger side N200"}},
    {1217, {"Seitenairbag Fahrerseite: Widerstand", "Side airbag, driver side: resistance"}},
    {1218, {"Seitenairbag Beifahrerseite: Widerstand", "Side airbag, passenger side: resistance"}},
    {1219, {"Abschaltung Beifahrerairbag: Schalter", "Passenger airbag deactivation: switch"}},
    {1221, {"Crashsensor Fahrerseite G179", "Crash sensor, driver side G179"}},
    {1222, {"Crashsensor Beifahrerseite G180", "Crash sensor, passenger side G180"}},
    {1223, {"Sitzbelegungserkennung Beifahrer", "Passenger seat occupancy detection"}},
    {1225, {"Kopfairbag Fahrerseite", "Head airbag, driver side"}},
    {1226, {"Kopfairbag Beifahrerseite", "Head airbag, passenger side"}},
};

// --- Address 17, instrument cluster ------------------------------------------
const Table kCluster = {
    {771, {"Geber für Kraftstoffvorrat G", "Fuel level sensor G"}},
    {281, {"Geber für Geschwindigkeit G22", "Vehicle speed sensor G22"}},
};

// --- Address 46, comfort system ----------------------------------------------
const Table kComfort = {
    {928, {"Zentralverriegelung Fahrertür", "Central locking, driver door"}},
    {929, {"Zentralverriegelung Beifahrertür", "Central locking, passenger door"}},
    {1331, {"Türsteuergerät Fahrerseite: keine Verbindung",
            "Door control unit, driver side: no communication"}},
    {1332, {"Türsteuergerät Beifahrerseite: keine Verbindung",
            "Door control unit, passenger side: no communication"}},
    {1333, {"Türsteuergerät hinten links: keine Verbindung",
            "Door control unit, rear left: no communication"}},
    {1334, {"Türsteuergerät hinten rechts: keine Verbindung",
            "Door control unit, rear right: no communication"}},
};

// --- Address 25, immobilizer -------------------------------------------------
const Table kImmobilizer = {
    {1176, {"Schlüssel: Signal fehlt oder Schlüssel nicht angelernt",
            "Key: no signal or key not adapted"}},
    {1177, {"Motorsteuergerät: Anpassung fehlt oder stimmt nicht",
            "Engine control unit: adaptation missing or incorrect"}},
    {1179, {"Schlüsselanpassung unvollständig", "Key adaptation incomplete"}},
};

// --- Address 16, steering wheel electronics ----------------------------------
const Table kSteering = {
    {778, {"Lenkwinkelgeber G85", "Steering angle sensor G85"}},
};

// --- Address 02, transmission ------------------------------------------------
const Table kTransmission = {
    {281, {"Geber für Geschwindigkeit G22", "Vehicle speed sensor G22"}},
    {525, {"Getriebeoeltemperatur", "Transmission fluid temperature"}},
};

struct AreaEntry {
    FaultArea area;
    const Table* table;
    Text label;
};

const std::vector<AreaEntry>& areaTables() {
    static const std::vector<AreaEntry> tables = {
        {FaultArea::Engine, &kEngine, {"Motor", "engine"}},
        {FaultArea::Transmission, &kTransmission, {"Getriebe", "transmission"}},
        {FaultArea::Brakes, &kBrakes, {"Bremsen", "brakes"}},
        {FaultArea::Airbag, &kAirbag, {"Airbag", "airbag"}},
        {FaultArea::Cluster, &kCluster, {"Kombiinstrument", "instrument cluster"}},
        {FaultArea::Comfort, &kComfort, {"Komfort", "comfort"}},
        {FaultArea::Immobilizer, &kImmobilizer, {"Wegfahrsperre", "immobilizer"}},
        {FaultArea::Steering, &kSteering, {"Lenkung", "steering"}},
    };
    return tables;
}

const Table* tableFor(FaultArea area) {
    for (const AreaEntry& entry : areaTables()) {
        if (entry.area == area) return entry.table;
    }
    return nullptr;
}

}  // namespace

FaultArea areaForEcuAddress(int address) {
    switch (address) {
        case 0x01:
        case 0x11:  // engine 2 on eight-cylinder engines
            return FaultArea::Engine;
        case 0x02:
        case 0x12:
            return FaultArea::Transmission;
        case 0x03:
        case 0x13:  // distance control belongs to the brake area
            return FaultArea::Brakes;
        case 0x15:
            return FaultArea::Airbag;
        case 0x17:
            return FaultArea::Cluster;
        case 0x16:
            return FaultArea::Steering;
        case 0x25:
            return FaultArea::Immobilizer;
        case 0x46:
        case 0x42:  // driver door
        case 0x52:  // passenger door
        case 0x62:  // rear left door
        case 0x72:  // rear right door
            return FaultArea::Comfort;
        default:
            return FaultArea::Unknown;
    }
}

std::string vagCodeToPCode(int vagCode) {
    if (vagCode < kPCodeOffset) return {};
    const int digits = vagCode - kPCodeOffset;
    if (digits < 0 || digits > 9999) return {};
    char buffer[8];
    std::snprintf(buffer, sizeof(buffer), "P%04d", digits);
    return buffer;
}

std::string describeVagFault(int vagCode, int ecuAddress) {
    const std::string pcode = vagCodeToPCode(vagCode);
    if (!pcode.empty()) return pcode + " - " + describeDtc(pcode);

    const FaultArea area = areaForEcuAddress(ecuAddress);

    // The area of the reporting control unit takes precedence, because the
    // same number means something different there than elsewhere.
    if (const Table* table = tableFor(area)) {
        const auto it = table->find(vagCode);
        if (it != table->end()) return it->second.get();
    }

    const auto shared = kShared.find(vagCode);
    if (shared != kShared.end()) return shared->second.get();

    // Address unknown: search all areas and name the match, so it stays clear
    // that the assignment is only probable.
    if (area == FaultArea::Unknown) {
        for (const AreaEntry& entry : areaTables()) {
            const auto it = entry.table->find(vagCode);
            if (it != entry.table->end()) {
                return std::string(it->second.get()) + L(" (vermutlich, Bereich ", " (probably, area ") +
                       entry.label.get() + ")";
            }
        }
    }

    return L("kein Klartext hinterlegt", "no description available");
}

std::size_t vagFaultCount() {
    std::size_t total = kShared.size();
    for (const AreaEntry& entry : areaTables()) total += entry.table->size();
    return total;
}

}  // namespace caruspex
