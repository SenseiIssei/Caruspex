// Plain text and conversion for on-board monitoring, modes 05 and 06.
// Names and units come out in the active language (see core/Lang.h).
#pragma once

#include <string>

namespace caruspex {

// Mode 06 on the K-Line: the test numbers are manufacturer-specific, but a few
// ranges are standardized.
std::string monitorTestName(int testId);

// Mode 06 on CAN: standardized identifiers for the monitored component.
std::string monitorComponentName(int componentId);

// Converts the raw value of a CAN test into a number and unit. The identifier
// determines the formula.
double monitorScale(int unitAndScaling, int raw, std::string& unit);

// Mode 05: the standardized test numbers of the oxygen sensor test.
std::string oxygenTestName(int testId);
double oxygenScale(int testId, int raw, std::string& unit);

}  // namespace caruspex
