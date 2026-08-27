// VAG-Fehlernummern in Klartext.
//
// Die herstellereigene Diagnose liefert fuenfstellige Zahlen statt P-Codes.
// Ab 16384 sind das umgerechnete EOBD-Codes: Nummer minus 16384 ergibt die
// Ziffernfolge des P-Codes. Damit laesst sich die vorhandene Fehlerdatenbank
// weiterbenutzen. Darunter liegen die alten VAG-eigenen Nummern.
//
// Wichtig: diese alten Nummern gelten je Steuergeraet. Die 01276 heisst im
// Motorsteuergeraet "Bremslichtschalter", im ABS-Steuergeraet dagegen
// "Hydraulikpumpe". Eine einzige flache Liste waere also schlicht falsch.
// Deshalb wird der Klartext immer zusammen mit der Steuergeraeteadresse
// gesucht. Ist die Adresse nicht bekannt, wird das Motorsteuergeraet
// angenommen und ein Treffer aus einem anderen Bereich als solcher benannt.
#pragma once

#include <cstddef>
#include <cstdint>
#include <string>

namespace caruspex {

// Fachbereich eines Steuergeraets. Mehrere Adressen koennen sich einen
// Bereich teilen, etwa alle Tuersteuergeraete das Komfortsystem.
enum class FaultArea {
    Engine,
    Transmission,
    Brakes,
    Airbag,
    Cluster,
    Comfort,
    Immobilizer,
    Steering,
    Unknown,
};

// Adresse (0x01 Motor, 0x03 ABS, ...) zum Fachbereich.
FaultArea areaForEcuAddress(int address);

// Umrechnung in den P-Code, leer wenn die Nummer zu klein dafuer ist.
std::string vagCodeToPCode(int vagCode);

// Klartext zu einer VAG-Fehlernummer. ecuAddress ist die Adresse des
// Steuergeraets, das den Fehler gemeldet hat; -1 wenn unbekannt.
std::string describeVagFault(int vagCode, int ecuAddress = -1);

// Wie viele alte VAG-Nummern insgesamt hinterlegt sind.
std::size_t vagFaultCount();

}  // namespace caruspex
