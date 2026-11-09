// How urgent something is, shared by the fault code database and the health
// check so both rate a finding the same way.
#pragma once

#include "core/Lang.h"

namespace caruspex {

enum class Severity {
    Info,       // worth knowing, nothing to do
    Low,        // fix it some time, the car is fine to drive
    Medium,     // fix it soon, consumption or emissions suffer
    High,       // get it looked at in the next days, damage can follow
    Critical,   // stop driving, risk of engine damage or to safety
};

// Short label for the traffic light, in the active language.
inline const char* severityLabel(Severity s) {
    switch (s) {
        case Severity::Info: return L("Hinweis", "Note");
        case Severity::Low: return L("Gering", "Low");
        case Severity::Medium: return L("Mittel", "Medium");
        case Severity::High: return L("Hoch", "High");
        case Severity::Critical: return L("Kritisch", "Critical");
    }
    return "";
}

}  // namespace caruspex
