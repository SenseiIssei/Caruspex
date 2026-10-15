// Decode trouble codes and translate them into plain language.
#pragma once

#include <string>
#include <unordered_map>
#include <unordered_set>

#include "core/Lang.h"

namespace caruspex {

// Raw bytes -> code like "P0301". Empty string for 0x00 0x00 (no fault).
std::string decodeDtc(int byteA, int byteB);

// Plain-language text in the active language, falls back to the meaning of
// the code's group.
std::string describeDtc(const std::string& code);

// Does the code concern the throttle body?
bool isThrottleDtc(const std::string& code);

// All known codes with their description in the active language.
const std::unordered_map<std::string, std::string>& dtcDescriptions();

// All known codes with their description in both languages.
const std::unordered_map<std::string, Text>& dtcTexts();

const std::unordered_set<std::string>& dtcThrottleCodes();

}  // namespace caruspex
