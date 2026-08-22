#include "data/VagFaults.h"

#include <cstdio>
#include <unordered_map>

#include "data/DtcDatabase.h"

namespace caruspex {
namespace {

// Ab dieser Nummer sind es umgerechnete EOBD-Codes.
constexpr int kPCodeOffset = 16384;

// Die alten VAG-eigenen Nummern. Nur die, die an Motorsteuergeraeten der
// K-Line-Zeit tatsaechlich auftauchen.
const std::unordered_map<int, const char*> kVagFaults = {
    {281, "Geber fuer Geschwindigkeit G22"},
    {283, "Raddrehzahlfuehler vorne links"},
    {284, "Raddrehzahlfuehler vorne rechts"},
    {285, "Raddrehzahlfuehler hinten links"},
    {286, "Raddrehzahlfuehler hinten rechts"},
    {513, "Motordrehzahlgeber G28"},
    {514, "Nockenwellengeber"},
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
    {532, "Spannungsversorgung Klemme 30"},
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
    {668, "Spannungsversorgung Klemme 30: Spannung zu klein"},
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
    {4096, "Steuergeraet defekt"},
    {8213, "Steuergeraet falsch codiert"},
};

}  // namespace

std::string vagCodeToPCode(int vagCode) {
    if (vagCode < kPCodeOffset) return {};
    const int digits = vagCode - kPCodeOffset;
    if (digits < 0 || digits > 9999) return {};
    char buffer[8];
    std::snprintf(buffer, sizeof(buffer), "P%04d", digits);
    return buffer;
}

std::string describeVagFault(int vagCode) {
    const std::string pcode = vagCodeToPCode(vagCode);
    if (!pcode.empty()) return pcode + " - " + describeDtc(pcode);

    const auto it = kVagFaults.find(vagCode);
    if (it != kVagFaults.end()) return it->second;

    return "kein Klartext hinterlegt";
}

std::size_t vagFaultCount() { return kVagFaults.size(); }

}  // namespace caruspex
