// EOBD services: read/clear faults, live values, freeze frame, readiness, VIN.
#pragma once

#include <set>
#include <string>
#include <vector>

#include "core/Elm327.h"

namespace caruspex {

struct Fault {
    std::string code;
    std::string kind;   // "gespeichert"/"stored", "sporadisch"/"intermittent",
                        // "dauerhaft"/"permanent" (active language)
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

// A result from on-board component monitoring (mode 06).
//
// This is the most detailed part of EOBD: for every internal test the control
// unit reports not just passed or failed, but the measured value together
// with its lower and upper limit. So you can see how close a component is to
// its limits long before the warning lamp comes on.
struct MonitorResult {
    int testId = 0;
    int componentId = 0;
    std::string name;        // plain-text name of the test, if known
    std::string component;   // plain-text name of the component
    double value = 0.0;
    double minimum = 0.0;
    double maximum = 0.0;
    std::string unit;
    bool hasMinimum = false;
    bool hasMaximum = false;
    bool passed = true;
};

// Result of an oxygen sensor test (mode 05). Only exists on vehicles without
// CAN, i.e. exactly the older ones like the Audi A2.
struct OxygenSensorTest {
    int testId = 0;
    int sensor = 0;
    std::string name;
    double value = 0.0;
    std::string unit;
};

// Identifiers from mode 09.
struct VehicleIdentity {
    std::string vin;
    std::string calibrationId;
    std::string calibrationVerification;
    std::string ecuName;
};

struct LiveValue {
    int pid = 0;
    std::string name;
    double value = 0.0;
    std::string text;   // set if this is a status value
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

    // Mode 09: VIN, calibration ID, checksum, control unit name. Anything
    // missing stays empty.
    VehicleIdentity readIdentity();

    // Mode 06: on-board component monitoring with value and limits.
    // On the K-line the tests are queried one by one, on CAN all at once.
    // Both are covered here.
    std::vector<MonitorResult> readMonitorResults(std::string& error);

    // Mode 05: oxygen sensor test results. Only exists without CAN.
    std::vector<OxygenSensorTest> readOxygenSensorTests(std::string& error);

private:
    std::vector<Fault> readCodes(const std::string& service, int sid, const char* kind);

    Elm327& elm_;
};

}  // namespace caruspex
