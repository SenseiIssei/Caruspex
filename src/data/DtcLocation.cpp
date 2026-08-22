#include "data/DtcLocation.h"

#include <cstdlib>

namespace caruspex {
namespace {

struct Range {
    int lo;
    int hi;
    CarZone zone;
    const char* hint;
};

// Die EOBD-Nummernkreise sind nach Baugruppen geordnet. Das nutzen wir, um
// von der Nummer auf die Stelle am Fahrzeug zu schliessen.
const Range kPowertrain[] = {
    {0x0010, 0x002F, CarZone::Cylinders,
     "Nockenwellenversteller und Steuertrieb, Kette oder Riemen pruefen"},
    {0x0030, 0x006F, CarZone::Exhaust,
     "Lambdasonden und ihre Heizung, vor und nach dem Katalysator"},
    {0x0070, 0x008F, CarZone::Intake, "Aussentemperatur- und Ansauggeber"},
    {0x0087, 0x009F, CarZone::FuelTank, "Kraftstoffdruck, Pumpe und Filter"},
    {0x0100, 0x0104, CarZone::Intake, "Luftmassenmesser im Ansaugrohr"},
    {0x0105, 0x0109, CarZone::Intake, "Saugrohrdrucksensor"},
    {0x0110, 0x0114, CarZone::Intake, "Ansauglufttemperaturgeber"},
    {0x0115, 0x0119, CarZone::Cooling, "Kuehlmitteltemperaturgeber am Motor"},
    {0x0120, 0x012F, CarZone::Throttle, "Drosselklappe und Fahrpedalgeber"},
    {0x0130, 0x016F, CarZone::Exhaust, "Lambdasonden im Abgasstrang"},
    {0x0170, 0x017F, CarZone::Intake,
     "Gemischbildung: Falschluft, Schlaeuche, Kurbelgehaeuseentlueftung"},
    {0x0180, 0x019F, CarZone::FuelTank, "Kraftstoffdruck und Temperatur"},
    {0x0200, 0x0214, CarZone::Injection, "Einspritzventile am Zylinderkopf"},
    {0x0215, 0x021F, CarZone::Injection, "Einspritzanlage"},
    {0x0220, 0x022F, CarZone::Throttle, "Drosselklappensensoren"},
    {0x0230, 0x025F, CarZone::Turbo, "Lader, Ladedruckdose und Unterdruckschlaeuche"},
    {0x0260, 0x0299, CarZone::Injection, "Einspritzventile, Zylinder einzeln"},
    {0x0300, 0x0312, CarZone::Cylinders,
     "Verbrennungsaussetzer: Kerzen, Spulen, Kompression, Einspritzventil"},
    {0x0320, 0x0349, CarZone::Ignition, "Kurbel- und Nockenwellengeber, Klopfsensoren"},
    {0x0350, 0x036F, CarZone::Ignition, "Zuendspulen und Zuendkerzen"},
    {0x0370, 0x039F, CarZone::Cylinders, "Steuerzeiten und Drehzahlgeber"},
    {0x0400, 0x040F, CarZone::Exhaust, "Abgasrueckfuehrung, Ventil meist verrusst"},
    {0x0410, 0x041F, CarZone::Exhaust, "Sekundaerluftsystem"},
    {0x0420, 0x043F, CarZone::Exhaust, "Katalysator und die Sonden davor und dahinter"},
    {0x0440, 0x045F, CarZone::FuelTank,
     "Tankentlueftung: zuerst den Tankdeckel pruefen, dann die Schlaeuche"},
    {0x0460, 0x047F, CarZone::FuelTank, "Tankgeber und Fuellstand"},
    {0x0480, 0x048F, CarZone::Cooling, "Kuehlerluefter und deren Relais"},
    {0x0490, 0x04FF, CarZone::Exhaust, "Abgasnachbehandlung"},
    {0x0500, 0x0504, CarZone::Wheels, "Geschwindigkeitssignal von den Raddrehzahlfuehlern"},
    {0x0505, 0x0510, CarZone::Throttle, "Leerlaufregelung, Drosselklappe verschmutzt?"},
    {0x0511, 0x051F, CarZone::Throttle, "Leerlaufsteller"},
    {0x0520, 0x052F, CarZone::Ecu, "Oeldruckgeber"},
    {0x0530, 0x055F, CarZone::Cooling, "Klimadruck und Kuehlkreis"},
    {0x0560, 0x056F, CarZone::Electrics, "Bordspannung: Batterie und Lichtmaschine"},
    {0x0570, 0x059F, CarZone::Electrics, "Schalter und Bordnetz"},
    {0x0600, 0x062F, CarZone::Ecu, "Motorsteuergeraet selbst"},
    {0x0630, 0x064F, CarZone::Ecu, "Steuergeraet und angesteuerte Relais"},
    {0x0650, 0x069F, CarZone::Electrics, "Ausgaenge und Relais"},
    {0x0700, 0x08FF, CarZone::Gearbox, "Getriebe und dessen Steuergeraet"},
    {0x1000, 0x10FF, CarZone::Ecu, "herstellerspezifisch, Steuergeraet"},
    {0x1100, 0x11FF, CarZone::Exhaust, "herstellerspezifisch, Lambdaregelung"},
    {0x1120, 0x11FF, CarZone::Intake, "herstellerspezifisch, Gemisch"},
    {0x1500, 0x1557, CarZone::Turbo, "herstellerspezifisch, Ladedruck"},
    {0x1558, 0x159F, CarZone::Throttle,
     "herstellerspezifisch, Leerlaufregelung und Drosselklappenanpassung"},
    {0x1600, 0x16FF, CarZone::Network, "herstellerspezifisch, Datenbus und Bordspannung"},
    {0x1700, 0x17FF, CarZone::Gearbox, "herstellerspezifisch, Getriebe"},
    {0x2000, 0x20FF, CarZone::Exhaust, "Abgasnachbehandlung, SCR und AdBlue"},
    {0x2100, 0x213F, CarZone::Throttle, "Drosselklappensteller und Fahrpedal"},
    {0x2140, 0x23FF, CarZone::Injection, "Einspritzung und Gemisch"},
    {0x2400, 0x24FF, CarZone::Exhaust, "Partikelfilter und Differenzdruck"},
    {0x2500, 0x2FFF, CarZone::Ecu, "Steuergeraet und Peripherie"},
};

CarZone zoneFor(char letter, int number, std::string& hint) {
    if (letter == 'C') {
        hint = "Raddrehzahlfuehler, ABS-Hydraulik und die Bremsen";
        return CarZone::Wheels;
    }
    if (letter == 'B') {
        hint = "Innenraum: Airbag, Gurtstraffer, Komfortelektronik";
        return CarZone::Cabin;
    }
    if (letter == 'U') {
        hint = "Datenbus zwischen den Steuergeraeten, oft nur ein Steckerproblem";
        return CarZone::Network;
    }
    for (const Range& r : kPowertrain) {
        if (number >= r.lo && number <= r.hi) {
            hint = r.hint;
            return r.zone;
        }
    }
    hint = "keine Zuordnung hinterlegt";
    return CarZone::Unknown;
}

}  // namespace

std::string zoneName(CarZone zone) {
    switch (zone) {
        case CarZone::Intake: return "Ansaugtrakt";
        case CarZone::Throttle: return "Drosselklappe";
        case CarZone::Injection: return "Einspritzung";
        case CarZone::Ignition: return "Zuendanlage";
        case CarZone::Cylinders: return "Motor und Steuertrieb";
        case CarZone::Cooling: return "Kuehlung";
        case CarZone::Turbo: return "Turbolader";
        case CarZone::Exhaust: return "Abgasanlage";
        case CarZone::FuelTank: return "Kraftstoffanlage";
        case CarZone::Gearbox: return "Getriebe";
        case CarZone::Ecu: return "Steuergeraet";
        case CarZone::Electrics: return "Bordnetz";
        case CarZone::Network: return "Datenbus";
        case CarZone::Wheels: return "Raeder und Bremsen";
        case CarZone::Cabin: return "Innenraum";
        case CarZone::Unknown: return "nicht zugeordnet";
    }
    return "nicht zugeordnet";
}

const std::vector<CarZone>& allZones() {
    static const std::vector<CarZone> zones = {
        CarZone::Intake,   CarZone::Throttle, CarZone::Injection, CarZone::Ignition,
        CarZone::Cylinders, CarZone::Cooling, CarZone::Turbo,     CarZone::Exhaust,
        CarZone::FuelTank, CarZone::Gearbox,  CarZone::Ecu,       CarZone::Electrics,
        CarZone::Network,  CarZone::Wheels,   CarZone::Cabin};
    return zones;
}

ZoneInfo locateDtc(const std::string& code) {
    ZoneInfo info;
    if (code.size() != 5) return info;
    const int number = static_cast<int>(std::strtol(code.substr(1).c_str(), nullptr, 16));
    info.zone = zoneFor(code[0], number, info.hint);
    info.name = zoneName(info.zone);
    return info;
}

}  // namespace caruspex
