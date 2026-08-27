// Klartext und Umrechnung fuer die Bauteilueberwachung, Modus 05 und 06.
#pragma once

#include <string>

namespace caruspex {

// Modus 06 auf der K-Line: die Testnummern sind herstellerspezifisch, ein paar
// Bereiche sind aber genormt.
std::string monitorTestName(int testId);

// Modus 06 auf CAN: genormte Kennungen fuer die ueberwachte Baugruppe.
std::string monitorComponentName(int componentId);

// Rechnet den Rohwert eines CAN-Tests in Zahl und Einheit um. Die Kennung
// bestimmt die Formel.
double monitorScale(int unitAndScaling, int raw, std::string& unit);

// Modus 05: die genormten Testnummern der Lambdasondenpruefung.
std::string oxygenTestName(int testId);
double oxygenScale(int testId, int raw, std::string& unit);

}  // namespace caruspex
