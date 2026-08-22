// Live-Messwerte (OBD-Modus 01): Definition und Umrechnung.
#pragma once

#include <functional>
#include <string>
#include <vector>

namespace caruspex {

struct PidSpec {
    int pid = 0;
    const char* name = "";
    const char* unit = "";
    int bytes = 1;
    // Rohbytes -> Zahlenwert
    std::function<double(const std::vector<int>&)> calc;
    // Optional: Zahlenwert -> Text (fuer Statuswerte)
    std::function<std::string(double)> text;
    double warnLow = 0.0;   // fuer die farbige Anzeige, 0/0 = keine Grenzen
    double warnHigh = 0.0;
};

const std::vector<PidSpec>& pidTable();
const PidSpec* findPid(int pid);

// Was in der Live-Ansicht zuerst gezeigt wird.
const std::vector<int>& defaultLivePids();

struct MonitorBit {
    int mask;
    const char* name;
};

const std::vector<MonitorBit>& monitorsB();
const std::vector<MonitorBit>& monitorsC();

}  // namespace caruspex
