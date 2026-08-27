#include "data/VagFaults.h"

#include <cstdio>
#include <unordered_map>
#include <vector>

#include "data/DtcDatabase.h"

namespace caruspex {
namespace {

// Ab dieser Nummer sind es umgerechnete EOBD-Codes.
constexpr int kPCodeOffset = 16384;

using Table = std::unordered_map<int, const char*>;

// Nummern, die in jedem Steuergeraet dasselbe bedeuten: Spannungsversorgung,
// Codierung und die Meldungen ueber ausgefallene Gespraechspartner am Bus.
const Table kShared = {
    {532, "Spannungsversorgung Klemme 30"},
    {668, "Bordspannung Klemme 30: Spannung zu klein"},
    {1044, "Steuergeraet falsch codiert"},
    {1312, "Datenbus Antrieb: gestoert"},
    {1314, "Motorsteuergeraet: keine Verbindung"},
    {1316, "Bremsensteuergeraet: keine Verbindung"},
    {1317, "Kombiinstrument: keine Verbindung"},
    {1321, "Airbag-Steuergeraet: keine Verbindung"},
    {1304, "Radio: keine Verbindung"},
    {4096, "Steuergeraet defekt"},
    {8213, "Steuergeraet falsch codiert"},
    {65535, "Interner Steuergeraetefehler"},
};

// --- Adresse 01, Motor -------------------------------------------------------
// Das ist der Bereich, der bei den beiden A2 am meisten hergibt: der 1.4 MPI
// wie auch der 1.6 FSI melden hierueber ihre Geber und Stellglieder.
const Table kEngine = {
    {281, "Geber fuer Geschwindigkeit G22"},
    {513, "Motordrehzahlgeber G28"},
    {514, "Nockenwellengeber G40"},
    {515, "Hallgeber G40"},
    {516, "Leerlaufschalter F60"},
    {517, "Vollastschalter F81"},
    {518, "Drosselklappenpotentiometer G69"},
    {519, "Saugrohrdruckgeber"},
    {522, "Kuehlmitteltemperaturgeber G62"},
    {523, "Ansauglufttemperaturgeber G72"},
    {524, "Klopfsensor 1 G61"},
    {525, "Lambdasonde G39"},
    {526, "Bremslichtschalter F"},
    {527, "Ansauglufttemperaturgeber G42"},
    {528, "Hoehengeber F96"},
    {530, "Drosselklappenpotentiometer G88"},
    {531, "Saugrohrdruck: Steuergeraet defekt"},
    {533, "Leerlaufregelung: Regelgrenze erreicht"},
    {535, "Klopfsensor 2 G66"},
    {537, "Lambdaregelung: Regelgrenze erreicht"},
    {539, "Kraftstoffdruckgeber"},
    {540, "Klimakompressor abgeschaltet"},
    {543, "Hoechstzulaessige Motordrehzahl ueberschritten"},
    {545, "Kupplungsschalter"},
    {546, "Datenbus Antrieb: keine Botschaft"},
    {549, "Kraftstoffverbrauchssignal"},
    {550, "Motorlastsignal"},
    {552, "Luftmassenmesser G70: Signal zu klein"},
    {553, "Luftmassenmesser G70"},
    {555, "Lambdaregelung: Regelgrenze unterschritten"},
    {558, "Einspritzventile: Regelabweichung"},
    {560, "Lambdasonde: Steuerung"},
    {561, "Gemischadaption: Regelgrenze ueberschritten"},
    {575, "Saugrohrdruck: Regelgrenze"},
    {577, "Klopfregelung Zylinder 1"},
    {578, "Klopfregelung Zylinder 2"},
    {579, "Klopfregelung Zylinder 3"},
    {580, "Klopfregelung Zylinder 4"},
    {585, "AGR-Temperaturgeber"},
    {586, "Abgasrueckfuehrung"},
    {587, "Magnetventil fuer Abgasrueckfuehrung N18"},
    {609, "Zuendsignal: Endstufe"},
    {624, "Klimaanlage: Signal"},
    {625, "Geschwindigkeitssignal"},
    {635, "Lambdasondenheizung Z19"},
    {640, "Drosselklappenpotentiometer: Signal unplausibel"},
    {652, "Lambdasonde nach Katalysator G130"},
    {660, "Ventil fuer Nockenwellenverstellung N205"},
    {670, "Leerlaufsteller: Regelgrenze"},
    {750, "Fehlerlampe K83"},
    {1165, "Drosselklappensteuerung: Regelbereich"},
    {1177, "Steuergeraet: Anpassung"},
    {1247, "Magnetventil fuer Aktivkohlebehaelter N80"},
    {1249, "Einspritzventil Zylinder 1 N30"},
    {1250, "Einspritzventil Zylinder 2 N31"},
    {1251, "Einspritzventil Zylinder 3 N32"},
    {1252, "Einspritzventil Zylinder 4 N33"},
    {1253, "Einspritzventil Zylinder 5 N83"},
    {1254, "Einspritzventil Zylinder 6 N84"},
    {1262, "Magnetventil fuer Ladedruckbegrenzung N75"},
    {1265, "Ventil fuer Abgasrueckfuehrung N18"},
    {1266, "Ventil fuer Saugrohrumschaltung N156"},
    {1268, "Mengensteller"},
    {1271, "Drosselklappensteuereinheit J338"},
    {1276, "Bremslichtschalter F"},
    {1289, "Wegfahrsperre: keine Freigabe"},
    {1393, "Steuergeraet: Endstufe defekt"},
    {1500, "Kraftstoffpumpenrelais J17"},
    {2142, "Kombiinstrument: Datenbus"},
    {2564, "Klimasteuergeraet: keine Verbindung"},
};

// --- Adresse 03, Bremsenelektronik -------------------------------------------
// Fuer den A2 der wichtigste Bereich neben dem Motor, weil ABS-Fehler dort als
// VAG-Nummer und nicht als C-Code herauskommen.
const Table kBrakes = {
    {283, "Raddrehzahlfuehler vorne links G47"},
    {285, "Raddrehzahlfuehler vorne rechts G45"},
    {287, "Raddrehzahlfuehler hinten rechts G44"},
    {290, "Raddrehzahlfuehler hinten links G46"},
    {778, "Lenkwinkelgeber G85"},
    {1130, "ABS-Betrieb: Regelung dauerhaft aktiv"},
    {1201, "Hydraulikpumpe fuer ABS V64"},
    {1202, "Magnetventile fuer ABS"},
    {1203, "Einlassventil vorne links N101"},
    {1204, "Auslassventil vorne links N102"},
    {1205, "Einlassventil vorne rechts N99"},
    {1206, "Auslassventil vorne rechts N100"},
    {1276, "Hydraulikpumpe fuer ABS V64: Stromkreis"},
    {1279, "Laengsbeschleunigungsgeber G251"},
    {1423, "Querbeschleunigungsgeber G200"},
    {1435, "Bremsdruckgeber 1 G201"},
};

// --- Adresse 15, Airbag ------------------------------------------------------
const Table kAirbag = {
    {588, "Zuender fuer Fahrerairbag N95"},
    {589, "Zuender fuer Beifahrerairbag N131"},
    {591, "Zuender fuer Gurtstraffer Fahrerseite N153"},
    {592, "Zuender fuer Gurtstraffer Beifahrerseite N154"},
    {595, "Seitenairbag Fahrerseite N199"},
    {596, "Seitenairbag Beifahrerseite N200"},
    {1217, "Seitenairbag Fahrerseite: Widerstand"},
    {1218, "Seitenairbag Beifahrerseite: Widerstand"},
    {1219, "Abschaltung Beifahrerairbag: Schalter"},
    {1221, "Crashsensor Fahrerseite G179"},
    {1222, "Crashsensor Beifahrerseite G180"},
    {1223, "Sitzbelegungserkennung Beifahrer"},
    {1225, "Kopfairbag Fahrerseite"},
    {1226, "Kopfairbag Beifahrerseite"},
};

// --- Adresse 17, Kombiinstrument ---------------------------------------------
const Table kCluster = {
    {771, "Geber fuer Kraftstoffvorrat G"},
    {281, "Geber fuer Geschwindigkeit G22"},
};

// --- Adresse 46, Komfortsystem -----------------------------------------------
const Table kComfort = {
    {928, "Zentralverriegelung Fahrertuer"},
    {929, "Zentralverriegelung Beifahrertuer"},
    {1331, "Tuersteuergeraet Fahrerseite: keine Verbindung"},
    {1332, "Tuersteuergeraet Beifahrerseite: keine Verbindung"},
    {1333, "Tuersteuergeraet hinten links: keine Verbindung"},
    {1334, "Tuersteuergeraet hinten rechts: keine Verbindung"},
};

// --- Adresse 25, Wegfahrsperre -----------------------------------------------
const Table kImmobilizer = {
    {1176, "Schluessel: Signal fehlt oder Schluessel nicht angelernt"},
    {1177, "Motorsteuergeraet: Anpassung fehlt oder stimmt nicht"},
    {1179, "Schluesselanpassung unvollstaendig"},
};

// --- Adresse 16, Lenkradelektronik -------------------------------------------
const Table kSteering = {
    {778, "Lenkwinkelgeber G85"},
};

// --- Adresse 02, Getriebe ----------------------------------------------------
const Table kTransmission = {
    {281, "Geber fuer Geschwindigkeit G22"},
    {525, "Getriebeoeltemperatur"},
};

struct AreaEntry {
    FaultArea area;
    const Table* table;
    const char* label;
};

const std::vector<AreaEntry>& areaTables() {
    static const std::vector<AreaEntry> tables = {
        {FaultArea::Engine, &kEngine, "Motor"},
        {FaultArea::Transmission, &kTransmission, "Getriebe"},
        {FaultArea::Brakes, &kBrakes, "Bremsen"},
        {FaultArea::Airbag, &kAirbag, "Airbag"},
        {FaultArea::Cluster, &kCluster, "Kombiinstrument"},
        {FaultArea::Comfort, &kComfort, "Komfort"},
        {FaultArea::Immobilizer, &kImmobilizer, "Wegfahrsperre"},
        {FaultArea::Steering, &kSteering, "Lenkung"},
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
        case 0x11:  // Motor 2 bei Achtzylindern
            return FaultArea::Engine;
        case 0x02:
        case 0x12:
            return FaultArea::Transmission;
        case 0x03:
        case 0x13:  // Abstandsregelung haengt am Bremsenbereich
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
        case 0x42:  // Tuer Fahrerseite
        case 0x52:  // Tuer Beifahrerseite
        case 0x62:  // Tuer hinten links
        case 0x72:  // Tuer hinten rechts
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

    // Der Bereich des meldenden Steuergeraets hat Vorrang, denn dieselbe
    // Nummer heisst dort etwas anderes als anderswo.
    if (const Table* table = tableFor(area)) {
        const auto it = table->find(vagCode);
        if (it != table->end()) return it->second;
    }

    const auto shared = kShared.find(vagCode);
    if (shared != kShared.end()) return shared->second;

    // Adresse unbekannt: in allen Bereichen nachsehen und den Fund benennen,
    // damit klar bleibt, dass die Zuordnung nur wahrscheinlich ist.
    if (area == FaultArea::Unknown) {
        for (const AreaEntry& entry : areaTables()) {
            const auto it = entry.table->find(vagCode);
            if (it != entry.table->end()) {
                return std::string(it->second) + " (vermutlich, Bereich " + entry.label + ")";
            }
        }
    }

    return "kein Klartext hinterlegt";
}

std::size_t vagFaultCount() {
    std::size_t total = kShared.size();
    for (const AreaEntry& entry : areaTables()) total += entry.table->size();
    return total;
}

}  // namespace caruspex
