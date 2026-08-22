// VAG-Fehlernummern in Klartext.
//
// Die herstellereigene Diagnose liefert fuenfstellige Zahlen statt P-Codes.
// Ab 16384 sind das umgerechnete EOBD-Codes: Nummer minus 16384 ergibt die
// Ziffernfolge des P-Codes. Damit laesst sich die vorhandene Fehlerdatenbank
// weiterbenutzen. Darunter liegen die alten VAG-eigenen Nummern, die eine
// eigene Liste brauchen.
#pragma once

#include <string>

namespace caruspex {

// Umrechnung in den P-Code, leer wenn die Nummer zu klein dafuer ist.
std::string vagCodeToPCode(int vagCode);

// Klartext zu einer VAG-Fehlernummer.
std::string describeVagFault(int vagCode);

// Wie viele alte VAG-Nummern hinterlegt sind.
std::size_t vagFaultCount();

}  // namespace caruspex
