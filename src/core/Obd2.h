// EOBD-Dienste: Fehler lesen/loeschen, Live-Werte, Freeze Frame, Readiness, FIN.
#pragma once

#include <set>
#include <string>
#include <vector>

#include "core/Elm327.h"

namespace caruspex {

struct Fault {
    std::string code;
    std::string kind;   // "gespeichert", "sporadisch", "dauerhaft"
    std::string text;
    bool throttleRelated = false;
};

struct ReadinessItem {
    std::string name;
    bool ready = false;
};

struct VehicleStatus {
    bool milOn = false;
    int faultCount = 0;
    std::vector<ReadinessItem> readiness;
    bool valid = false;
};

struct LiveValue {
    int pid = 0;
    std::string name;
    double value = 0.0;
    std::string text;   // gesetzt, wenn es ein Statuswert ist
    std::string unit;
    bool warn = false;
};

class Obd2 {
public:
    explicit Obd2(Elm327& elm) : elm_(elm) {}

    std::vector<Fault> readFaults(std::string& error);
    bool clearFaults(std::string& error);
    VehicleStatus readStatus();
    std::set<int> supportedPids();
    bool readLive(int pid, LiveValue& out);
    std::vector<LiveValue> readLive(const std::vector<int>& pids);
    std::string readVin();
    std::vector<LiveValue> readFreezeFrame(std::string& triggerCode);

private:
    std::vector<Fault> readCodes(const std::string& service, int sid, const char* kind);

    Elm327& elm_;
};

}  // namespace caruspex
