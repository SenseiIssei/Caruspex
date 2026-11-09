// EOBD services: read/clear faults, live values, freeze frame, readiness, VIN.
//
// Several control units: on CAN the engine (7E8), the transmission (7E9) and
// sometimes a hybrid controller answer the same request. With headers off
// their answers are mixed; switch them on with Elm327::setHeaders(true) and
// every fault, readiness block, mode 06 and mode 09 result carries the
// address of its control unit (ecuSource) and a plain name (ecu).
#pragma once

#include <map>
#include <set>
#include <string>
#include <utility>
#include <vector>

#include "core/Elm327.h"
#include "data/Pids.h"

namespace caruspex {

struct Fault {
    std::string code;
    std::string kind;   // "gespeichert"/"stored", "sporadisch"/"intermittent",
                        // "dauerhaft"/"permanent" (active language)
    std::string text;
    bool throttleRelated = false;
    // Which control unit reported it, e.g. "Motor (7E8)" / "Engine (7E8)".
    // Empty while headers are off.
    std::string ecu;
    std::string ecuSource;   // raw address: "7E8", "18DAF110", "10"
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
    bool compressionIgnition = false;  // diesel: different readiness monitors
    std::string ecu;                   // see Fault::ecu
    std::string ecuSource;
};

// A result from on-board component monitoring (mode 06).
//
// This is the most detailed part of EOBD: for every internal test the control
// unit reports not just passed or failed, but the measured value together
// with its lower and upper limit. So you can see how close a component is to
// its limits long before the warning lamp comes on.
//
// On CAN, componentId is the monitor ID (OBDMID, e.g. 0xA2 = misfire
// cylinder 1) and testId the test ID (TID, e.g. 0x0C = misfires in the
// current driving cycle). On the K-line, testId is the test number and
// componentId the component or limit ID.
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
    bool can = false;        // decoded with the CAN layout (OBDMID/TID)
    int unitAndScaling = 0;  // CAN only: the unit and scaling ID
    std::string ecu;         // see Fault::ecu
    std::string ecuSource;
};

// Misfire counts of one cylinder (or of the whole engine), taken from the
// mode 06 results on CAN: monitor 0xA1 is the engine as a whole, 0xA2 to
// 0xAD are cylinders 1 to 12. Test 0x0B is the weighted average over the
// last ten driving cycles, test 0x0C the count of the current (or last)
// driving cycle.
struct MisfireCount {
    int cylinder = 0;            // 0 = whole engine
    std::string name;            // "Zylinder 3" / "Cylinder 3", "Motor gesamt"
    double currentCycle = 0.0;
    double averageTenCycles = 0.0;
    bool hasCurrent = false;
    bool hasAverage = false;
    std::string text;            // a plain sentence for non-mechanics
    std::string ecu;
    std::string ecuSource;
};

// Pulls the misfire counters out of the mode 06 results. Cylinders come in
// order, the whole engine first. Empty if the control unit has no misfire
// monitor results (K-line vehicles: the test numbers are not standardized
// there, so nothing is guessed).
std::vector<MisfireCount> misfireCounts(const std::vector<MonitorResult>& results);

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
    std::string calibrationId;            // several are joined with ", "
    std::string calibrationVerification;  // CVN as hex, several joined with ", "
    std::string ecuName;
};

// In-use performance tracking (mode 09, InfoType 0x08 for spark ignition,
// 0x0B for compression ignition). For every monitor the control unit counts
// how often the test ran to the end (completed) and how often the driving
// conditions would have allowed it (conditions). The ratio shows whether a
// monitor runs often enough in everyday driving.
struct InUseCounter {
    std::string name;
    int completed = 0;
    int conditions = 0;
    double ratio = 0.0;      // completed / conditions, 0 if conditions == 0
    bool hasRatio = false;
    std::string text;        // plain sentence
};

struct InUsePerformance {
    bool valid = false;
    bool sparkIgnition = true;  // 0x08 spark ignition, 0x0B compression ignition
    int obdConditions = 0;      // driving cycles that met the OBD conditions
    int ignitionCycles = 0;     // ignition cycles counted
    std::vector<InUseCounter> monitors;
};

// Everything mode 09 tells about one control unit.
struct EcuInfo {
    std::string ecuSource;       // "7E8", empty while headers are off
    std::string ecu;             // "Motor (7E8)", empty while headers are off
    std::string name;            // InfoType 0x0A, e.g. "ECM-EngineControl"
    std::string vin;             // InfoType 0x02
    std::vector<std::string> calibrationIds;   // InfoType 0x04
    std::vector<std::string> cvns;             // InfoType 0x06, hex "1A2B3C4D"
    std::string esn;             // InfoType 0x0D, engine serial number
    InUsePerformance inUse;      // InfoType 0x08 or 0x0B
};

// Mode 09 decoders, public for tests. messages are the responses of one
// control unit without the service byte, each starting with the InfoType.
// On CAN one message holds everything (InfoType, number of items, data); on
// the K-line every message holds a sequence number and four data bytes.
std::vector<int> infoPayload(const std::vector<std::vector<int>>& messages);
std::string infoText(const std::vector<int>& payload);
std::vector<std::string> infoCalibrationIds(const std::vector<int>& payload);
std::vector<std::string> infoCvns(const std::vector<int>& payload);
InUsePerformance infoInUsePerformance(int infoType, const std::vector<int>& payload);

struct LiveValue {
    int pid = 0;
    int key = 0;        // series id, see liveKey() in data/Pids.h; == pid for sub 0
    int sub = 0;
    std::string name;
    double value = 0.0;
    std::string text;   // set if this is a status value
    std::string unit;
    bool warn = false;
    PidCategory category = PidCategory::Engine;
    double displayMin = 0.0;
    double displayMax = 0.0;
    std::string ecu;    // see Fault::ecu
    std::string ecuSource;
};

// One freeze frame (mode 02). A control unit may keep several: frame 0 is
// the one the standard requires, further frames belong to later faults.
// PID 02 of each frame names the fault that stored it.
struct FreezeFrame {
    int frame = 0;
    std::string dtc;          // e.g. "P0300"
    std::string ecu;          // see Fault::ecu, empty while headers are off
    std::string ecuSource;
    std::vector<LiveValue> values;   // all values of the frame, sub-values included
};

// Splits one mode 01 answer to a multi-PID request into its values. data is
// the message without the service byte: PID, its data bytes, next PID, ...
// The length of each block comes from the PID table. Only PIDs listed in
// requested are accepted, each once. Padding bytes after the last block are
// ignored. Returns false if the message does not add up (unknown PID, short
// data, leftover bytes); blocks is then incomplete and must not be used.
// Public for tests.
bool splitMultiPidAnswer(const std::vector<int>& data, const std::vector<int>& requested,
                         std::vector<std::pair<int, std::vector<int>>>& blocks);

// Whether a PID may go into a multi-PID request: known to the PID table,
// not a range marker, and with a length that does not vary between control
// units (0x68 is three bytes on older ones and seven on newer ones).
bool batchablePid(int pid);

class Obd2 {
public:
    explicit Obd2(Elm327& elm) : elm_(elm) {}

    std::vector<Fault> readFaults(std::string& error);
    bool clearFaults(std::string& error);
    // Status of the first control unit that answers (normally the engine).
    VehicleStatus readStatus();
    // Status of every control unit that answers. Needs headers to tell them
    // apart; without headers the entries simply have no ecu name.
    std::vector<VehicleStatus> readStatusPerEcu();
    // Supported PIDs 0x01 to 0xE0, merged over all control units.
    std::set<int> supportedPids();
    // Primary value of a PID (sub 0). If the control unit does not report
    // the primary value of a multi-value PID, the first value it does report.
    bool readLive(int pid, LiveValue& out);
    // One value per PID, in the order of pids, like readLive(int) for each.
    // Uses readLiveBatch underneath, so on CAN it is just as fast.
    std::vector<LiveValue> readLive(const std::vector<int>& pids);
    // All values of one PID the control unit reports, e.g. both coolant
    // sensors of PID 0x67. Empty if the PID is unknown or not answered.
    std::vector<LiveValue> readLiveAll(int pid);

    // All values of several PIDs, sub-values included: the same as calling
    // readLiveAll for each PID, in the order of pids, but much faster on CAN.
    //
    // On CAN (ISO 15765-4) SAE J1979 allows up to six mode 01 PIDs in one
    // request ("010C0D0511"); the answer holds each PID followed by its data
    // bytes. The PIDs are grouped by six and every group costs one round
    // trip instead of six. On the K-line and J1850, and for PIDs whose
    // length varies (see batchablePid), every PID is asked on its own.
    //
    // If a group gets NO DATA, a refusal or an answer that does not add up,
    // its PIDs are read one by one and remembered: for the rest of the
    // session (this Obd2 object, or until resetBatchMemory) they are no
    // longer put into groups. The same happens to a PID a control unit
    // leaves out of a group answer.
    //
    // Several control units: like readLiveAll, a PID keeps the values of
    // the first control unit that answers it (the engine on CAN). With
    // everyEcu the values of every control unit are kept, each with its
    // ecu/ecuSource (needs headers on to tell them apart).
    std::vector<LiveValue> readLiveBatch(const std::vector<int>& pids, bool everyEcu = false);
    // PIDs that are read one by one for the rest of the session because a
    // group request with them failed.
    const std::set<int>& batchRefusedPids() const { return batchRefused_; }
    void resetBatchMemory() { batchRefused_.clear(); }

    std::string readVin();
    // Frame 0 only, primary values of a fixed set of PIDs. Kept for older
    // callers; readFreezeFrames gives every frame with all values.
    std::vector<LiveValue> readFreezeFrame(std::string& triggerCode);

    // Freeze frames 0 to maxFrames - 1. Stops at the first frame that
    // reports no fault code (00 00) or gets no answer. The values of each
    // frame are the PIDs the control unit lists as supported for it; if it
    // does not list them, a fixed set of the usual ones.
    std::vector<FreezeFrame> readFreezeFrames(int maxFrames = 4);

    // Mode 09: VIN, calibration ID, checksum, control unit name. Anything
    // missing stays empty.
    VehicleIdentity readIdentity();

    // Mode 09 in full, per control unit: name, calibration IDs, CVNs,
    // in-use performance counters, engine serial number.
    std::vector<EcuInfo> readEcuInfo();

    // Mode 06: on-board component monitoring with value and limits.
    // On the K-line the tests are queried one by one, on CAN monitor by
    // monitor. Both are covered here.
    std::vector<MonitorResult> readMonitorResults(std::string& error);

    // Mode 05: oxygen sensor test results. Only exists without CAN.
    std::vector<OxygenSensorTest> readOxygenSensorTests(std::string& error);

private:
    std::vector<Fault> readCodes(const std::string& service, int sid, const char* kind);
    LiveValue makeValue(const PidSpec& spec, const PidReading& reading,
                        const std::string& source) const;
    // readLiveAll, optionally keeping the values of every control unit.
    std::vector<LiveValue> readPid(int pid, bool everyEcu);
    // Freeze frame PIDs a control unit lists as supported for one frame.
    std::vector<int> freezeFramePids(int frame);
    // One group request; false if it has to be read PID by PID instead.
    bool readGroup(const std::vector<int>& group, bool everyEcu,
                   std::map<int, std::vector<LiveValue>>& values, std::vector<int>& missing);

    Elm327& elm_;
    std::set<int> batchRefused_;
};

}  // namespace caruspex
