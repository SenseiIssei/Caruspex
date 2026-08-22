// Fahrzeugprofile: welches Protokoll wird zuerst probiert, was ist zu erwarten.
#pragma once

#include <string>
#include <vector>

namespace caruspex {

enum class BusKind {
    KLine,    // ISO 9141-2 / ISO 14230, alte Europaeer und Japaner
    Can,      // ISO 15765-4, ab etwa 2004 Pflicht in Europa
    Pwm,      // SAE J1850 PWM, alte Ford USA
    Vpw,      // SAE J1850 VPW, alte GM USA
    Mixed,    // Uebergangszeit, beides moeglich
};

struct VehicleProfile {
    std::string brand;
    std::string model;       // leer = ganze Marke
    int yearFrom = 0;
    int yearTo = 0;
    BusKind bus = BusKind::Mixed;
    std::string protocolOrder;   // ELM327-Protokollnummern in Probierreihenfolge
    std::string note;
};

// Alle hinterlegten Profile.
const std::vector<VehicleProfile>& vehicleProfiles();

// Marken alphabetisch, fuer die Auswahlliste.
std::vector<std::string> vehicleBrands();

// Profile einer Marke.
std::vector<const VehicleProfile*> profilesForBrand(const std::string& brand);

// Bestes Profil fuer Marke und Baujahr, sonst nullptr.
const VehicleProfile* findProfile(const std::string& brand, int year);

// Reihenfolge, wenn nichts bekannt ist: schnelle Protokolle zuerst.
extern const char* const kAutoProtocolOrder;

std::string busName(BusKind bus);

}  // namespace caruspex
