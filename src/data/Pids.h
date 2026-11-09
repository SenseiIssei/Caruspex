// Live values (OBD mode 01): definition and conversion.
//
// Names, units and status texts come out in the active language (see
// core/Lang.h). pidTable() and findPid() pick the table for the language that
// is active at the time of the call; the pointers stay valid for the whole
// program run.
//
// Many newer PIDs carry more than one value. PID 0x67 for example reports two
// coolant sensors, PID 0x7F three different run time counters. Each of those
// values is described by a PidValueSpec, and each one gets its own integer key
// that the UI can use as a series id:
//
//     key = pid | (sub << 8)
//
// sub 0 is the primary value of the PID, so for every single-value PID the
// key is simply the PID number, exactly as before. sub 1, 2, ... are the
// further values in the order the standard lists them. Example: PID 0x24
// (wide-band O2 sensor) has key 0x024 for lambda and 0x124 for the voltage.
#pragma once

#include <functional>
#include <string>
#include <vector>

namespace caruspex {

// Rough grouping for the live view, so a long list of values can be shown
// in sensible sections.
enum class PidCategory {
    Engine,        // speed, load, torque, ignition, pedal
    Fuel,          // mixture control, fuel pressure, fuel rate, tank
    Air,           // air mass, manifold pressure, throttle, turbocharger
    Exhaust,       // oxygen sensors, exhaust temperatures and pressure
    Emissions,     // EGR, evaporative system, particulate filter, NOx, AdBlue
    Temperatures,  // coolant, oil, intake and outside air
    Electrical,    // supply voltage
    Vehicle,       // road speed, gear, odometer, run time
    HybridEv,      // traction battery
    Diagnostics,   // counters since clearing, OBD standard, scaling limits
};

// Display name of a category in the active language.
const char* pidCategoryName(PidCategory category);
// All categories in display order.
const std::vector<PidCategory>& pidCategories();

// The key scheme described at the top of this file.
constexpr int liveKey(int pid, int sub) { return (pid & 0xFF) | (sub << 8); }
constexpr int liveKeyPid(int key) { return key & 0xFF; }
constexpr int liveKeySub(int key) { return key >> 8; }

using PidCalc = std::function<double(const std::vector<int>&)>;
using PidText = std::function<std::string(double)>;
using PidAvailable = std::function<bool(const std::vector<int>&)>;

// One value inside a PID response.
struct PidValueSpec {
    int sub = 0;
    int key = 0;              // liveKey(pid, sub)
    const char* name = "";
    const char* unit = "";
    int needBytes = 1;        // data bytes (after the PID byte) this value reads
    PidCalc calc;             // raw data bytes A, B, ... -> number
    PidText text;             // optional: number -> text (status values)
    // Optional: whether the control unit reports this value at all. Most
    // multi-value PIDs start with a byte A whose bits say which of the
    // following values are present. Empty = always present.
    PidAvailable available;
    double displayMin = 0.0;  // sensible gauge range
    double displayMax = 0.0;
    double warnLow = 0.0;     // for the colored display, 0/0 = no limits
    double warnHigh = 0.0;

    bool isAvailable(const std::vector<int>& data) const {
        if (static_cast<int>(data.size()) < needBytes) return false;
        return !available || available(data);
    }
    bool warns(double value) const {
        return (warnLow != 0.0 || warnHigh != 0.0) && (value < warnLow || value > warnHigh);
    }
};

struct PidSpec {
    int pid = 0;
    // The fields below describe the primary value (sub 0) and are kept so
    // code that only knows single-value PIDs keeps working.
    const char* name = "";
    const char* unit = "";
    int bytes = 1;            // data length according to the standard
    PidCalc calc;
    PidText text;
    double warnLow = 0.0;
    double warnHigh = 0.0;

    // Since the extension to the complete standard:
    const char* title = "";   // name of the PID as a whole (equals name for single values)
    PidCategory category = PidCategory::Engine;
    double displayMin = 0.0;  // gauge range of the primary value
    double displayMax = 0.0;
    std::vector<PidValueSpec> values;  // values[0] is the primary value, key == pid
};

const std::vector<PidSpec>& pidTable();
const PidSpec* findPid(int pid);
// Looks up a single value by its key (see liveKey).
const PidValueSpec* findPidValue(int key);
// Number of individual values over all PIDs.
int pidValueCount();

// One decoded value of a response.
struct PidReading {
    const PidValueSpec* spec = nullptr;
    double value = 0.0;
};

// Decodes all values the control unit reports in one response. data are the
// bytes after the PID byte. Values whose support bit is clear or whose bytes
// are missing are left out.
std::vector<PidReading> decodePid(const PidSpec& spec, const std::vector<int>& data);

// What the live view shows first.
const std::vector<int>& defaultLivePids();

struct MonitorBit {
    int mask;
    const char* name;
};

const std::vector<MonitorBit>& monitorsB();
// Readiness monitors in byte C/D of PID 0x01 for spark ignition engines.
const std::vector<MonitorBit>& monitorsC();
// The same bytes mean something else on compression ignition (diesel) engines.
const std::vector<MonitorBit>& monitorsCDiesel();

}  // namespace caruspex
