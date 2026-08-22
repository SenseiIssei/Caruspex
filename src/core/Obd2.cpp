#include "core/Obd2.h"

#include <windows.h>

#include <cstdio>

#include "data/DtcDatabase.h"
#include "data/Pids.h"

namespace caruspex {
namespace {

std::string hexCommand(const char* fmt, int value) {
    char buffer[16];
    std::snprintf(buffer, sizeof(buffer), fmt, value);
    return buffer;
}

}  // namespace

std::vector<Fault> Obd2::readCodes(const std::string& service, int sid, const char* kind) {
    std::vector<Fault> out;
    ElmResult result = elm_.request(service, sid);
    if (!result.ok()) return out;

    for (const std::vector<int>& data : result.messages) {
        // Bei CAN steht vorne die Anzahl der Codes, bei K-Line nicht.
        // Ungerade Laenge heisst: erstes Byte ist der Zaehler.
        const size_t start = (data.size() % 2 == 1) ? 1 : 0;
        for (size_t i = start; i + 1 < data.size(); i += 2) {
            const std::string code = decodeDtc(data[i], data[i + 1]);
            if (code.empty()) continue;
            bool known = false;
            for (const Fault& f : out) {
                if (f.code == code) known = true;
            }
            if (known) continue;
            Fault f;
            f.code = code;
            f.kind = kind;
            f.text = describeDtc(code);
            f.throttleRelated = isThrottleDtc(code);
            out.push_back(std::move(f));
        }
    }
    return out;
}

std::vector<Fault> Obd2::readFaults(std::string& error) {
    error.clear();
    std::vector<Fault> all = readCodes("03", 0x03, "gespeichert");
    const struct {
        const char* service;
        int sid;
        const char* kind;
    } extra[] = {{"07", 0x07, "sporadisch"}, {"0A", 0x0A, "dauerhaft"}};

    for (const auto& e : extra) {
        for (Fault& f : readCodes(e.service, e.sid, e.kind)) {
            bool known = false;
            for (const Fault& existing : all) {
                if (existing.code == f.code) known = true;
            }
            if (!known) all.push_back(std::move(f));
        }
    }
    return all;
}

bool Obd2::clearFaults(std::string& error) {
    ElmResult result = elm_.request("04", 0x04, 8000);
    if (result.ok()) return true;
    if (result.status == ElmStatus::AdapterError || result.status == ElmStatus::BusError) {
        error = result.message;
        return false;
    }
    // Manche Steuergeraete quittieren stumm: nachsehen, ob es gewirkt hat.
    std::string ignored;
    return readFaults(ignored).empty();
}

VehicleStatus Obd2::readStatus() {
    VehicleStatus status;
    ElmResult result = elm_.request("0101", 0x01);
    if (!result.ok() || result.messages.empty()) return status;

    std::vector<int> data = result.messages.front();
    if (!data.empty() && data[0] == 0x01) data.erase(data.begin());  // PID-Echo
    if (data.size() < 4) return status;

    status.valid = true;
    status.milOn = (data[0] & 0x80) != 0;
    status.faultCount = data[0] & 0x7F;
    const int b = data[1], c = data[2], d = data[3];
    for (const MonitorBit& m : monitorsB()) {
        if (b & m.mask) status.readiness.push_back({m.name, (b & (m.mask << 4)) == 0});
    }
    if ((b & 0x08) == 0) {  // Ottomotor
        for (const MonitorBit& m : monitorsC()) {
            if (c & m.mask) status.readiness.push_back({m.name, (d & m.mask) == 0});
        }
    }
    return status;
}

std::set<int> Obd2::supportedPids() {
    std::set<int> found;
    for (int base : {0x00, 0x20, 0x40, 0x60}) {
        ElmResult result = elm_.request(hexCommand("01%02X", base), 0x01);
        if (!result.ok() || result.messages.empty()) break;
        std::vector<int> data = result.messages.front();
        if (!data.empty() && data[0] == base) data.erase(data.begin());
        if (data.size() < 4) break;
        const unsigned mask = (static_cast<unsigned>(data[0]) << 24) |
                              (static_cast<unsigned>(data[1]) << 16) |
                              (static_cast<unsigned>(data[2]) << 8) |
                              static_cast<unsigned>(data[3]);
        for (int i = 0; i < 32; ++i) {
            if (mask & (1u << (31 - i))) found.insert(base + i + 1);
        }
        if ((mask & 1u) == 0) break;  // naechste Bank nicht unterstuetzt
    }
    return found;
}

bool Obd2::readLive(int pid, LiveValue& out) {
    const PidSpec* spec = findPid(pid);
    if (!spec) return false;
    ElmResult result = elm_.request(hexCommand("01%02X", pid), 0x01, 4000);
    if (!result.ok() || result.messages.empty()) return false;

    std::vector<int> data = result.messages.front();
    if (!data.empty() && data[0] == pid) data.erase(data.begin());
    if (static_cast<int>(data.size()) < spec->bytes) return false;

    out.pid = pid;
    out.name = spec->name;
    out.unit = spec->unit;
    out.value = spec->calc(data);
    out.text = spec->text ? spec->text(out.value) : std::string();
    out.warn = (spec->warnLow != 0.0 || spec->warnHigh != 0.0) &&
               (out.value < spec->warnLow || out.value > spec->warnHigh);
    return true;
}

std::vector<LiveValue> Obd2::readLive(const std::vector<int>& pids) {
    std::vector<LiveValue> out;
    for (int pid : pids) {
        LiveValue value;
        if (readLive(pid, value)) out.push_back(std::move(value));
    }
    return out;
}

std::string Obd2::readVin() {
    ElmResult result = elm_.request("0902", 0x09, 8000);
    if (!result.ok()) return {};
    std::string text;
    for (const std::vector<int>& msg : result.messages) {
        size_t start = (!msg.empty() && msg[0] == 0x02) ? 1 : 0;
        for (size_t i = start; i < msg.size(); ++i) {
            if (msg[i] > 32 && msg[i] < 127) text += static_cast<char>(msg[i]);
        }
    }
    if (text.size() >= 17) return text.substr(text.size() - 17);
    return text;
}

std::vector<LiveValue> Obd2::readFreezeFrame(std::string& triggerCode) {
    triggerCode.clear();
    ElmResult trigger = elm_.request("0202", 0x02);
    if (trigger.ok() && !trigger.messages.empty()) {
        const std::vector<int>& msg = trigger.messages.front();
        if (msg.size() >= 4) triggerCode = decodeDtc(msg[2], msg[3]);
    }

    std::vector<LiveValue> rows;
    for (int pid : {0x0C, 0x0D, 0x05, 0x04, 0x11, 0x0F, 0x10, 0x0B, 0x03}) {
        const PidSpec* spec = findPid(pid);
        if (!spec) continue;
        ElmResult result = elm_.request(hexCommand("02%02X00", pid), 0x02, 4000);
        if (!result.ok() || result.messages.empty()) continue;
        std::vector<int> data = result.messages.front();
        if (data.size() >= 2 && data[0] == pid) data.erase(data.begin(), data.begin() + 2);
        if (static_cast<int>(data.size()) < spec->bytes) continue;
        LiveValue value;
        value.pid = pid;
        value.name = spec->name;
        value.unit = spec->unit;
        value.value = spec->calc(data);
        value.text = spec->text ? spec->text(value.value) : std::string();
        rows.push_back(std::move(value));
    }
    return rows;
}

}  // namespace caruspex
