// Live values (OBD mode 01): definition and conversion.
//
// Names, units and status texts come out in the active language (see
// core/Lang.h). pidTable() and findPid() pick the table for the language that
// is active at the time of the call; the pointers stay valid for the whole
// program run.
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
    // Raw bytes -> numeric value
    std::function<double(const std::vector<int>&)> calc;
    // Optional: numeric value -> text (for status values)
    std::function<std::string(double)> text;
    double warnLow = 0.0;   // for the colored display, 0/0 = no limits
    double warnHigh = 0.0;
};

const std::vector<PidSpec>& pidTable();
const PidSpec* findPid(int pid);

// What the live view shows first.
const std::vector<int>& defaultLivePids();

struct MonitorBit {
    int mask;
    const char* name;
};

const std::vector<MonitorBit>& monitorsB();
const std::vector<MonitorBit>& monitorsC();

}  // namespace caruspex
