// Fehlercodes dekodieren und in Klartext uebersetzen.
#pragma once

#include <string>
#include <unordered_map>
#include <unordered_set>

namespace caruspex {

// Rohbytes -> Code wie "P0301". Leerer String bei 0x00 0x00 (kein Fehler).
std::string decodeDtc(int byteA, int byteB);

// Klartext, faellt auf die Gruppenbedeutung zurueck.
std::string describeDtc(const std::string& code);

// Betrifft der Code die Drosselklappe?
bool isThrottleDtc(const std::string& code);

const std::unordered_map<std::string, std::string>& dtcDescriptions();
const std::unordered_set<std::string>& dtcThrottleCodes();

}  // namespace caruspex
