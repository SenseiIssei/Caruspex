#include "core/Obd2.h"

#include <algorithm>
#include <cstddef>
#include <cstdio>
#include <iterator>
#include <map>

#include "core/EcuAddress.h"
#include "core/Lang.h"
#include "data/DtcDatabase.h"
#include "data/MonitorTests.h"
#include "data/Pids.h"

namespace caruspex {
namespace {

std::string hexCommand(const char* fmt, int value) {
    char buffer[16];
    std::snprintf(buffer, sizeof(buffer), fmt, value);
    return buffer;
}

const std::string& sourceAt(const ElmResult& result, size_t index) {
    static const std::string none;
    return index < result.sources.size() ? result.sources[index] : none;
}

// Numbers in the active language: a decimal comma in German.
std::string decimal(double value, int digits) {
    char buffer[32];
    std::snprintf(buffer, sizeof(buffer), "%.*f", digits, value);
    std::string out = buffer;
    if (language() == Language::German) std::replace(out.begin(), out.end(), '.', ',');
    return out;
}

// Groups the messages of a response by sender, in the order the senders
// first appear. Without headers everything lands in one group.
std::vector<std::pair<std::string, std::vector<std::vector<int>>>> groupBySource(
    const ElmResult& result) {
    std::vector<std::pair<std::string, std::vector<std::vector<int>>>> groups;
    for (size_t i = 0; i < result.messages.size(); ++i) {
        const std::string& source = sourceAt(result, i);
        auto it = std::find_if(groups.begin(), groups.end(),
                               [&source](const auto& g) { return g.first == source; });
        if (it == groups.end()) {
            groups.push_back({source, {}});
            it = groups.end() - 1;
        }
        it->second.push_back(result.messages[i]);
    }
    return groups;
}

VehicleStatus decodeStatus(std::vector<int> data) {
    VehicleStatus status;
    if (!data.empty() && data[0] == 0x01) data.erase(data.begin());  // PID echo
    if (data.size() < 4) return status;

    status.valid = true;
    status.milOn = (data[0] & 0x80) != 0;
    status.faultCount = data[0] & 0x7F;
    const int b = data[1], c = data[2], d = data[3];
    for (const MonitorBit& m : monitorsB()) {
        if (b & m.mask) status.readiness.push_back({m.name, (b & (m.mask << 4)) == 0});
    }
    // Bit 3 of byte B: 0 = spark ignition, 1 = compression ignition. The
    // bits of bytes C and D mean different monitors on the two.
    status.compressionIgnition = (b & 0x08) != 0;
    const std::vector<MonitorBit>& monitors =
        status.compressionIgnition ? monitorsCDiesel() : monitorsC();
    for (const MonitorBit& m : monitors) {
        if (c & m.mask) status.readiness.push_back({m.name, (d & m.mask) == 0});
    }
    return status;
}

}  // namespace

std::vector<Fault> Obd2::readCodes(const std::string& service, int sid, const char* kind) {
    std::vector<Fault> out;
    ElmResult result = elm_.request(service, sid);
    if (!result.ok()) return out;

    for (size_t m = 0; m < result.messages.size(); ++m) {
        const std::vector<int>& data = result.messages[m];
        const std::string& source = sourceAt(result, m);
        // On CAN the number of codes comes first, on K-line it does not.
        // Odd length means: the first byte is the count.
        const size_t start = (data.size() % 2 == 1) ? 1 : 0;
        for (size_t i = start; i + 1 < data.size(); i += 2) {
            const std::string code = decodeDtc(data[i], data[i + 1]);
            if (code.empty()) continue;
            bool known = false;
            for (const Fault& f : out) {
                if (f.code == code && f.ecuSource == source) known = true;
            }
            if (known) continue;
            Fault f;
            f.code = code;
            f.kind = kind;
            f.text = describeDtc(code);
            f.throttleRelated = isThrottleDtc(code);
            f.ecuSource = source;
            f.ecu = ecuDisplayName(source);
            out.push_back(std::move(f));
        }
    }
    return out;
}

std::vector<Fault> Obd2::readFaults(std::string& error) {
    error.clear();
    std::vector<Fault> all = readCodes("03", 0x03, L("gespeichert", "stored"));
    const struct {
        const char* service;
        int sid;
        const char* kind;
    } extra[] = {{"07", 0x07, L("sporadisch", "intermittent")},
                 {"0A", 0x0A, L("dauerhaft", "permanent")}};

    for (const auto& e : extra) {
        for (Fault& f : readCodes(e.service, e.sid, e.kind)) {
            bool known = false;
            for (const Fault& existing : all) {
                if (existing.code == f.code && existing.ecuSource == f.ecuSource) known = true;
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
    // Some control units acknowledge silently: check whether it worked.
    std::string ignored;
    return readFaults(ignored).empty();
}

VehicleStatus Obd2::readStatus() {
    const std::vector<VehicleStatus> all = readStatusPerEcu();
    for (const VehicleStatus& s : all) {
        if (s.valid) return s;
    }
    return {};
}

std::vector<VehicleStatus> Obd2::readStatusPerEcu() {
    std::vector<VehicleStatus> out;
    ElmResult result = elm_.request("0101", 0x01);
    if (!result.ok()) return out;
    for (size_t i = 0; i < result.messages.size(); ++i) {
        VehicleStatus status = decodeStatus(result.messages[i]);
        if (!status.valid) continue;
        status.ecuSource = sourceAt(result, i);
        status.ecu = ecuDisplayName(status.ecuSource);
        out.push_back(std::move(status));
    }
    return out;
}

std::set<int> Obd2::supportedPids() {
    std::set<int> found;
    for (int base = 0x00; base <= 0xC0; base += 0x20) {
        ElmResult result = elm_.request(hexCommand("01%02X", base), 0x01);
        if (!result.ok() || result.messages.empty()) break;
        // Several control units each send their own mask; together they
        // say what the vehicle can answer.
        bool next = false;
        for (std::vector<int> data : result.messages) {
            if (!data.empty() && data[0] == base) data.erase(data.begin());
            if (data.size() < 4) continue;
            const unsigned mask = (static_cast<unsigned>(data[0]) << 24) |
                                  (static_cast<unsigned>(data[1]) << 16) |
                                  (static_cast<unsigned>(data[2]) << 8) |
                                  static_cast<unsigned>(data[3]);
            for (int i = 0; i < 32; ++i) {
                if (mask & (1u << (31 - i))) found.insert(base + i + 1);
            }
            if (mask & 1u) next = true;
        }
        if (!next) break;  // next range not supported
    }
    // The range markers themselves are no live values.
    for (int marker = 0x20; marker <= 0xE0; marker += 0x20) found.erase(marker);
    return found;
}

LiveValue Obd2::makeValue(const PidSpec& spec, const PidReading& reading,
                          const std::string& source) const {
    LiveValue out;
    out.pid = spec.pid;
    out.key = reading.spec->key;
    out.sub = reading.spec->sub;
    out.name = reading.spec->name;
    out.unit = reading.spec->unit;
    out.value = reading.value;
    out.text = reading.spec->text ? reading.spec->text(reading.value) : std::string();
    out.warn = reading.spec->warns(reading.value);
    out.category = spec.category;
    out.displayMin = reading.spec->displayMin;
    out.displayMax = reading.spec->displayMax;
    out.ecuSource = source;
    out.ecu = ecuDisplayName(source);
    return out;
}

std::vector<LiveValue> Obd2::readPid(int pid, bool everyEcu) {
    std::vector<LiveValue> out;
    const PidSpec* spec = findPid(pid);
    if (!spec) return out;
    ElmResult result = elm_.request(hexCommand("01%02X", pid), 0x01, 4000);
    if (!result.ok() || result.messages.empty()) return out;

    // Several control units may answer (vehicle speed comes from engine and
    // transmission); the first answer counts, which is the engine on CAN.
    for (size_t i = 0; i < result.messages.size(); ++i) {
        std::vector<int> data = result.messages[i];
        if (data.empty() || data[0] != pid) continue;
        data.erase(data.begin());
        for (const PidReading& reading : decodePid(*spec, data)) {
            out.push_back(makeValue(*spec, reading, sourceAt(result, i)));
        }
        if (!out.empty() && !everyEcu) break;
    }
    return out;
}

std::vector<LiveValue> Obd2::readLiveAll(int pid) { return readPid(pid, false); }

bool Obd2::readLive(int pid, LiveValue& out) {
    std::vector<LiveValue> all = readLiveAll(pid);
    if (all.empty()) return false;
    out = std::move(all.front());
    return true;
}

std::vector<LiveValue> Obd2::readLive(const std::vector<int>& pids) {
    // readLive(int) keeps the first value of a PID, which is the primary
    // value whenever the control unit reports it. The same here.
    const std::vector<LiveValue> all = readLiveBatch(pids);
    std::vector<LiveValue> out;
    for (int pid : pids) {
        const auto it = std::find_if(all.begin(), all.end(),
                                     [pid](const LiveValue& v) { return v.pid == pid; });
        if (it != all.end()) out.push_back(*it);
    }
    return out;
}

// ---------------------------------------------------------------------------
// Several PIDs per request (CAN only)
// ---------------------------------------------------------------------------

bool batchablePid(int pid) {
    if (pid <= 0 || pid > 0xFF || pid % 0x20 == 0) return false;
    // PID 0x68 (intake air temperature sensors) is three bytes on older
    // control units and seven on newer ones. Inside a combined answer a
    // wrong length would shift every PID after it, so it is asked alone.
    if (pid == 0x68) return false;
    const PidSpec* spec = findPid(pid);
    return spec && spec->bytes >= 1;
}

bool splitMultiPidAnswer(const std::vector<int>& data, const std::vector<int>& requested,
                         std::vector<std::pair<int, std::vector<int>>>& blocks) {
    blocks.clear();
    std::set<int> seen;
    size_t pos = 0;
    while (pos < data.size()) {
        const int pid = data[pos];
        const bool wanted = std::find(requested.begin(), requested.end(), pid) != requested.end() &&
                            seen.count(pid) == 0;
        if (!wanted) {
            // Some control units fill the rest of the frame. Bytes that can
            // only be padding are fine after at least one complete block.
            const bool padding =
                !blocks.empty() && std::all_of(data.begin() + static_cast<std::ptrdiff_t>(pos),
                                               data.end(), [](int b) {
                                                   return b == 0x00 || b == 0x55 || b == 0xAA ||
                                                          b == 0xCC;
                                               });
            return padding;
        }
        const PidSpec* spec = findPid(pid);
        if (!spec || spec->bytes < 1) return false;
        const size_t length = static_cast<size_t>(spec->bytes);
        if (pos + 1 + length > data.size()) return false;
        blocks.push_back({pid, std::vector<int>(data.begin() + static_cast<std::ptrdiff_t>(pos + 1),
                                                data.begin() + static_cast<std::ptrdiff_t>(pos + 1 + length))});
        seen.insert(pid);
        pos += 1 + length;
    }
    return !blocks.empty();
}

bool Obd2::readGroup(const std::vector<int>& group, bool everyEcu,
                     std::map<int, std::vector<LiveValue>>& values, std::vector<int>& missing) {
    std::string command = "01";
    for (int pid : group) command += hexCommand("%02X", pid);
    const ElmResult result = elm_.request(command, 0x01, 4000);
    if (!result.ok() || result.messages.empty()) return false;

    // Decode everything first: one answer that does not add up spoils the
    // group, and nothing of it may land in values then.
    std::map<int, std::vector<LiveValue>> found;
    std::set<int> answered;   // present in an answer, even if no value was usable
    for (size_t i = 0; i < result.messages.size(); ++i) {
        std::vector<std::pair<int, std::vector<int>>> blocks;
        if (!splitMultiPidAnswer(result.messages[i], group, blocks)) return false;
        for (const auto& [pid, data] : blocks) {
            answered.insert(pid);
            // As in readLiveAll: the first control unit that answers a PID
            // with something usable keeps it, unless every one is wanted.
            if (!everyEcu && found.count(pid)) continue;
            const PidSpec* spec = findPid(pid);
            for (const PidReading& reading : decodePid(*spec, data)) {
                found[pid].push_back(makeValue(*spec, reading, sourceAt(result, i)));
            }
        }
    }
    for (auto& [pid, list] : found) {
        std::vector<LiveValue>& target = values[pid];
        target.insert(target.end(), list.begin(), list.end());
    }
    missing.clear();
    for (int pid : group) {
        if (!answered.count(pid)) missing.push_back(pid);
    }
    return true;
}

std::vector<LiveValue> Obd2::readLiveBatch(const std::vector<int>& pids, bool everyEcu) {
    // Every PID once, in the order asked; unknown PIDs are dropped like in
    // readLiveAll.
    std::vector<int> order;
    for (int pid : pids) {
        if (findPid(pid) && std::find(order.begin(), order.end(), pid) == order.end())
            order.push_back(pid);
    }

    std::map<int, std::vector<LiveValue>> values;
    std::vector<int> single;
    if (elm_.isCan()) {
        std::vector<int> grouped;
        for (int pid : order) {
            if (batchablePid(pid) && !batchRefused_.count(pid)) grouped.push_back(pid);
            else single.push_back(pid);
        }
        constexpr size_t kMaxPerRequest = 6;   // SAE J1979 on ISO 15765-4
        for (size_t start = 0; start < grouped.size(); start += kMaxPerRequest) {
            const size_t end = std::min(grouped.size(), start + kMaxPerRequest);
            const std::vector<int> group(grouped.begin() + static_cast<std::ptrdiff_t>(start),
                                         grouped.begin() + static_cast<std::ptrdiff_t>(end));
            if (group.size() == 1) {
                single.push_back(group.front());
                continue;
            }
            std::vector<int> missing;
            if (!readGroup(group, everyEcu, values, missing)) missing = group;
            // Left out or spoilt: from now on these are asked one by one.
            // That also catches control units that only answer the first PID
            // of a combined request.
            for (int pid : missing) {
                batchRefused_.insert(pid);
                single.push_back(pid);
            }
        }
        // Single requests go out in the order asked, which keeps a log of
        // the traffic easy to follow.
        const auto position = [&order](int pid) {
            return std::find(order.begin(), order.end(), pid) - order.begin();
        };
        std::stable_sort(single.begin(), single.end(),
                         [&position](int a, int b) { return position(a) < position(b); });
    } else {
        single = order;
    }

    for (int pid : single) {
        std::vector<LiveValue> list = readPid(pid, everyEcu);
        std::vector<LiveValue>& target = values[pid];
        target.insert(target.end(), list.begin(), list.end());
    }

    std::vector<LiveValue> out;
    for (int pid : order) {
        const auto it = values.find(pid);
        if (it != values.end()) out.insert(out.end(), it->second.begin(), it->second.end());
    }
    return out;
}

// ---------------------------------------------------------------------------
// Mode 09
// ---------------------------------------------------------------------------

std::vector<int> infoPayload(const std::vector<std::vector<int>>& messages) {
    std::vector<int> payload;
    if (messages.empty()) return payload;

    // K-line: several short messages "InfoType, sequence number, 4 bytes".
    // Put them in sequence order, the adapter keeps the order anyway.
    bool kline = messages.size() > 1;
    for (const std::vector<int>& m : messages) {
        if (m.size() > 6) kline = false;
    }
    std::vector<std::vector<int>> ordered = messages;
    if (kline) {
        std::stable_sort(ordered.begin(), ordered.end(),
                         [](const std::vector<int>& a, const std::vector<int>& b) {
                             return (a.size() > 1 ? a[1] : 0) < (b.size() > 1 ? b[1] : 0);
                         });
    }
    // Both layouts start with the InfoType and one counting byte (number of
    // data items on CAN, sequence number on the K-line).
    for (const std::vector<int>& m : ordered) {
        if (m.size() > 2) payload.insert(payload.end(), m.begin() + 2, m.end());
    }
    return payload;
}

std::string infoText(const std::vector<int>& payload) {
    std::string text;
    for (int b : payload) {
        if (b >= 0x20 && b < 0x7F) text += static_cast<char>(b);
    }
    while (!text.empty() && text.front() == ' ') text.erase(text.begin());
    while (!text.empty() && text.back() == ' ') text.pop_back();
    return text;
}

// Calibration IDs are 16 characters each, padded with zero bytes.
std::vector<std::string> infoCalibrationIds(const std::vector<int>& payload) {
    std::vector<std::string> out;
    for (size_t i = 0; i + 16 <= payload.size(); i += 16) {
        const std::string id =
            infoText(std::vector<int>(payload.begin() + static_cast<std::ptrdiff_t>(i),
                                      payload.begin() + static_cast<std::ptrdiff_t>(i + 16)));
        if (!id.empty()) out.push_back(id);
    }
    // Some control units leave out the padding of the last ID; whatever
    // text is left over still counts.
    const size_t rest = payload.size() - payload.size() % 16;
    const std::string tail =
        infoText(std::vector<int>(payload.begin() + static_cast<std::ptrdiff_t>(rest), payload.end()));
    if (!tail.empty()) out.push_back(tail);
    return out;
}

// Calibration verification numbers are four binary bytes each.
std::vector<std::string> infoCvns(const std::vector<int>& payload) {
    std::vector<std::string> out;
    for (size_t i = 0; i + 4 <= payload.size(); i += 4) {
        char buffer[16];
        std::snprintf(buffer, sizeof(buffer), "%02X%02X%02X%02X", payload[i] & 0xFF,
                      payload[i + 1] & 0xFF, payload[i + 2] & 0xFF, payload[i + 3] & 0xFF);
        out.push_back(buffer);
    }
    return out;
}

InUsePerformance infoInUsePerformance(int infoType, const std::vector<int>& payload) {
    InUsePerformance perf;
    perf.sparkIgnition = infoType != 0x0B;

    // After the two general counters come pairs: completed, conditions met.
    static const Text spark[] = {
        {"Katalysator Bank 1", "Catalyst bank 1"},
        {"Katalysator Bank 2", "Catalyst bank 2"},
        {"Lambdasonde Bank 1", "Oxygen sensor bank 1"},
        {"Lambdasonde Bank 2", "Oxygen sensor bank 2"},
        {"Abgasrückführung und Nockenwellenverstellung", "EGR and VVT"},
        {"Sekundärluft", "Secondary air"},
        {"Tankentlüftung", "Evaporative system"},
        {"Hintere Lambdasonde Bank 1", "Secondary oxygen sensor bank 1"},
        {"Hintere Lambdasonde Bank 2", "Secondary oxygen sensor bank 2"},
        {"Gemischverteilung der Zylinder Bank 1", "Air-fuel ratio imbalance bank 1"},
        {"Gemischverteilung der Zylinder Bank 2", "Air-fuel ratio imbalance bank 2"},
        {"Benzinpartikelfilter Bank 1", "Gasoline particulate filter bank 1"},
        {"Benzinpartikelfilter Bank 2", "Gasoline particulate filter bank 2"},
    };
    static const Text compression[] = {
        {"Oxidationskatalysator (NMHC)", "NMHC catalyst"},
        {"Stickoxid-Katalysator (SCR)", "NOx catalyst (SCR)"},
        {"NOx-Speicherkatalysator", "NOx adsorber"},
        {"Partikelfilter", "Particulate filter"},
        {"Abgassensor", "Exhaust gas sensor"},
        {"Abgasrückführung und Nockenwellenverstellung", "EGR and VVT"},
        {"Ladedruck", "Boost pressure"},
        {"Kraftstoffsystem", "Fuel system"},
    };
    const Text* names = perf.sparkIgnition ? spark : compression;
    const size_t nameCount = perf.sparkIgnition ? std::size(spark) : std::size(compression);

    std::vector<int> counters;
    for (size_t i = 0; i + 1 < payload.size(); i += 2) counters.push_back((payload[i] << 8) | payload[i + 1]);
    if (counters.size() < 2) return perf;

    perf.valid = true;
    perf.obdConditions = counters[0];
    perf.ignitionCycles = counters[1];
    for (size_t pair = 0; 2 + 2 * pair + 1 < counters.size(); ++pair) {
        InUseCounter c;
        c.name = pair < nameCount ? names[pair].get()
                                  : L("Überwachung ", "Monitor ") + std::to_string(pair + 1);
        c.completed = counters[2 + 2 * pair];
        c.conditions = counters[2 + 2 * pair + 1];
        c.hasRatio = c.conditions > 0;
        c.ratio = c.hasRatio ? static_cast<double>(c.completed) / c.conditions : 0.0;
        if (!c.hasRatio) {
            c.text = c.completed == 0
                         ? L("noch keine passende Fahrt erkannt", "no suitable trip recorded yet")
                         : std::to_string(c.completed) + L(" Mal geprüft", " times checked");
        } else {
            c.text = std::to_string(c.completed) + L(" Mal geprüft bei ", " times checked in ") +
                     std::to_string(c.conditions) + L(" passenden Fahrten, Quote ",
                                                      " suitable trips, ratio ") +
                     decimal(c.ratio, 2);
        }
        perf.monitors.push_back(std::move(c));
    }
    return perf;
}

std::string Obd2::readVin() {
    ElmResult result = elm_.request("0902", 0x09, 8000);
    if (!result.ok()) return {};
    std::string text;
    for (const auto& group : groupBySource(result)) {
        text = infoText(infoPayload(group.second));
        if (text.size() >= 17) break;
    }
    // Some control units put zero bytes or spaces in front; the VIN is the
    // last 17 characters.
    std::string compact;
    for (char c : text) {
        if (c != ' ') compact += c;
    }
    if (compact.size() >= 17) return compact.substr(compact.size() - 17);
    return compact;
}

VehicleIdentity Obd2::readIdentity() {
    VehicleIdentity identity;
    identity.vin = readVin();

    // Several control units may report the same number; it is listed once.
    auto join = [](const std::vector<std::string>& parts) {
        std::string out;
        for (size_t i = 0; i < parts.size(); ++i) {
            if (std::find(parts.begin(), parts.begin() + static_cast<std::ptrdiff_t>(i), parts[i]) !=
                parts.begin() + static_cast<std::ptrdiff_t>(i))
                continue;
            if (!out.empty()) out += ", ";
            out += parts[i];
        }
        return out;
    };

    std::vector<std::string> calIds, cvns;
    ElmResult cal = elm_.request("0904", 0x09, 6000);
    if (cal.ok()) {
        for (const auto& group : groupBySource(cal)) {
            for (const std::string& id : infoCalibrationIds(infoPayload(group.second))) calIds.push_back(id);
        }
    }
    ElmResult cvn = elm_.request("0906", 0x09, 6000);
    if (cvn.ok()) {
        for (const auto& group : groupBySource(cvn)) {
            for (const std::string& c : infoCvns(infoPayload(group.second))) cvns.push_back(c);
        }
    }
    ElmResult name = elm_.request("090A", 0x09, 6000);
    if (name.ok()) {
        const auto groups = groupBySource(name);
        if (!groups.empty()) identity.ecuName = infoText(infoPayload(groups.front().second));
    }
    identity.calibrationId = join(calIds);
    identity.calibrationVerification = join(cvns);
    return identity;
}

std::vector<EcuInfo> Obd2::readEcuInfo() {
    std::vector<EcuInfo> ecus;
    auto entry = [&ecus](const std::string& source) -> EcuInfo& {
        for (EcuInfo& e : ecus) {
            if (e.ecuSource == source) return e;
        }
        EcuInfo e;
        e.ecuSource = source;
        e.ecu = ecuDisplayName(source);
        ecus.push_back(std::move(e));
        return ecus.back();
    };

    for (int type : {0x02, 0x0A, 0x04, 0x06, 0x08, 0x0B, 0x0D}) {
        ElmResult result = elm_.request(hexCommand("09%02X", type), 0x09, 6000);
        if (!result.ok()) continue;
        for (const auto& group : groupBySource(result)) {
            // On CAN without headers every message may come from another
            // control unit, so each is decoded on its own.
            std::vector<std::vector<std::vector<int>>> parts;
            if (elm_.isCan()) {
                for (const std::vector<int>& m : group.second) parts.push_back({m});
            } else {
                parts.push_back(group.second);
            }
            EcuInfo& e = entry(group.first);
            for (const auto& part : parts) {
                const std::vector<int> payload = infoPayload(part);
                switch (type) {
                    case 0x02: {
                        const std::string vin = infoText(payload);
                        e.vin = vin.size() > 17 ? vin.substr(vin.size() - 17) : vin;
                        break;
                    }
                    case 0x0A: e.name = infoText(payload); break;
                    case 0x04:
                        for (const std::string& id : infoCalibrationIds(payload)) e.calibrationIds.push_back(id);
                        break;
                    case 0x06:
                        for (const std::string& c : infoCvns(payload)) e.cvns.push_back(c);
                        break;
                    case 0x08:
                    case 0x0B: {
                        InUsePerformance perf = infoInUsePerformance(type, payload);
                        if (perf.valid) e.inUse = std::move(perf);
                        break;
                    }
                    case 0x0D: e.esn = infoText(payload); break;
                    default: break;
                }
            }
        }
    }
    return ecus;
}

// ---------------------------------------------------------------------------
// Mode 06
// ---------------------------------------------------------------------------

// Mode 06, on-board component monitoring.
//
// On CAN the request names a monitor (OBDMID); the answer holds all its
// tests, nine bytes each: OBDMID, TID, unit and scaling ID, value, lower
// limit, upper limit (two bytes each). Which monitors exist is asked first
// with the IDs 0x00, 0x20, 0x40 ... like the supported PIDs.
// The K-line does not have that; there each test is queried individually and
// the response has no unit byte.
std::vector<MonitorResult> Obd2::readMonitorResults(std::string& error) {
    error.clear();
    std::vector<MonitorResult> out;

    if (elm_.isCan() || elm_.protocolId() == '0') {
        std::set<int> monitors;
        for (int base = 0x00; base <= 0xE0; base += 0x20) {
            ElmResult block = elm_.request(hexCommand("06%02X", base), 0x06, 6000);
            if (!block.ok()) break;
            bool next = false;
            for (const std::vector<int>& msg : block.messages) {
                if (msg.size() < 5 || msg[0] != base) continue;
                const unsigned mask = (static_cast<unsigned>(msg[1]) << 24) |
                                      (static_cast<unsigned>(msg[2]) << 16) |
                                      (static_cast<unsigned>(msg[3]) << 8) |
                                      static_cast<unsigned>(msg[4]);
                for (int i = 0; i < 31; ++i) {
                    if (mask & (1u << (31 - i))) monitors.insert(base + i + 1);
                }
                if (mask & 1u) next = true;
            }
            if (!next) break;
        }

        for (int mid : monitors) {
            ElmResult result = elm_.request(hexCommand("06%02X", mid), 0x06, 6000);
            if (!result.ok()) continue;
            for (size_t m = 0; m < result.messages.size(); ++m) {
                const std::vector<int>& msg = result.messages[m];
                for (size_t i = 0; i + 8 < msg.size(); i += 9) {
                    MonitorResult r;
                    r.can = true;
                    r.componentId = msg[i];
                    r.testId = msg[i + 1];
                    r.unitAndScaling = msg[i + 2];
                    r.value = monitorScale(r.unitAndScaling, (msg[i + 3] << 8) | msg[i + 4], r.unit);
                    std::string ignored;
                    r.minimum = monitorScale(r.unitAndScaling, (msg[i + 5] << 8) | msg[i + 6], ignored);
                    r.maximum = monitorScale(r.unitAndScaling, (msg[i + 7] << 8) | msg[i + 8], ignored);
                    r.hasMinimum = r.hasMaximum = true;
                    r.name = monitorTestNameCan(r.componentId, r.testId);
                    r.component = monitorComponentName(r.componentId);
                    r.passed = r.value >= r.minimum && r.value <= r.maximum;
                    r.ecuSource = sourceAt(result, m);
                    r.ecu = ecuDisplayName(r.ecuSource);
                    out.push_back(std::move(r));
                }
            }
        }
        // Once the control unit has listed its monitors, the K-line route
        // below must not run: its requests would be read in the wrong layout.
        if (!monitors.empty()) {
            if (out.empty()) {
                error = L("Das Steuergerät nennt Überwachungen, liefert aber noch keine "
                          "Ergebnisse. Nach dem Löschen der Fehler braucht es erst einige Fahrten.",
                          "The control unit lists monitors but has no results yet. After "
                          "clearing the codes it needs a few trips first.");
            }
            return out;
        }
    }

    // The route for vehicles without CAN: go through the tests one by one.
    for (int testId = 0x01; testId <= 0x40; ++testId) {
        ElmResult result = elm_.request(hexCommand("06%02X", testId), 0x06, 2500);
        if (!result.ok()) continue;
        for (size_t m = 0; m < result.messages.size(); ++m) {
            const std::vector<int>& msg = result.messages[m];
            // Layout without the service byte: ID, component, value (2),
            // lower limit (2), upper limit (2). That is eight bytes, not
            // nine. Shorter responses occur as well.
            if (msg.size() < 5) continue;
            MonitorResult r;
            r.testId = msg[0];
            r.componentId = msg[1];
            std::string unit;
            r.value = oxygenScale(r.testId, (msg[2] << 8) | msg[3], unit);
            r.unit = unit;
            if (msg.size() >= 6) {
                r.minimum = oxygenScale(r.testId, (msg[4] << 8) | msg[5], unit);
                r.hasMinimum = true;
            }
            if (msg.size() >= 8) {
                r.maximum = oxygenScale(r.testId, (msg[6] << 8) | msg[7], unit);
                r.hasMaximum = true;
            }
            r.name = monitorTestName(r.testId);
            r.component = monitorComponentName(r.componentId);
            r.passed = (!r.hasMinimum || r.value >= r.minimum) &&
                       (!r.hasMaximum || r.value <= r.maximum);
            r.ecuSource = sourceAt(result, m);
            r.ecu = ecuDisplayName(r.ecuSource);
            out.push_back(std::move(r));
        }
    }

    if (out.empty()) {
        error = L(
            "Das Steuergerät liefert keine Bauteilüberwachung. Bei Fahrzeugen vor etwa "
            "2003 ist das normal.",
            "The control unit provides no component monitoring results. This is normal "
            "for vehicles from before about 2003.");
    }
    return out;
}

std::vector<MisfireCount> misfireCounts(const std::vector<MonitorResult>& results) {
    // Keyed by sender and cylinder so two engines' worth of data (rare, but
    // possible with headers on) stay apart and the order is stable.
    std::map<std::pair<std::string, int>, MisfireCount> found;
    for (const MonitorResult& r : results) {
        if (!r.can || r.componentId < 0xA1 || r.componentId > 0xAD) continue;
        if (r.testId != 0x0B && r.testId != 0x0C) continue;
        const int cylinder = r.componentId - 0xA1;   // 0 = whole engine
        MisfireCount& m = found[{r.ecuSource, cylinder}];
        m.cylinder = cylinder;
        m.ecu = r.ecu;
        m.ecuSource = r.ecuSource;
        if (r.testId == 0x0B) {
            m.averageTenCycles = r.value;
            m.hasAverage = true;
        } else {
            m.currentCycle = r.value;
            m.hasCurrent = true;
        }
    }

    std::vector<MisfireCount> out;
    for (auto& [key, m] : found) {
        m.name = m.cylinder == 0 ? L("Motor gesamt", "Whole engine")
                                 : L("Zylinder ", "Cylinder ") + std::to_string(m.cylinder);
        const auto count = [](double v) { return std::to_string(static_cast<long>(v + 0.5)); };
        std::string text = m.name + ": ";
        if (m.hasCurrent) {
            text += m.currentCycle < 0.5
                        ? std::string(L("keine Aussetzer in der aktuellen Fahrt",
                                        "no misfires in the current trip"))
                        : count(m.currentCycle) + L(" Aussetzer in der aktuellen Fahrt",
                                                    " misfires in the current trip");
        }
        if (m.hasAverage) {
            if (m.hasCurrent) text += ", ";
            text += L("im Schnitt ", "on average ") + decimal(m.averageTenCycles, 1) +
                    L(" je Fahrt über die letzten 10 Fahrten", " per trip over the last 10 trips");
        }
        m.text = text + ".";
        out.push_back(std::move(m));
    }
    return out;
}

// Mode 05, the oxygen sensor test. Only exists without CAN, i.e. exactly on
// the older vehicles.
std::vector<OxygenSensorTest> Obd2::readOxygenSensorTests(std::string& error) {
    error.clear();
    std::vector<OxygenSensorTest> out;

    for (int testId = 0x01; testId <= 0x0A; ++testId) {
        for (int sensor = 1; sensor <= 4; ++sensor) {
            char command[16];
            std::snprintf(command, sizeof(command), "05%02X%02X", testId, sensor);
            ElmResult result = elm_.request(command, 0x05, 2500);
            if (!result.ok() || result.messages.empty()) continue;

            // Layout without the service byte (SAE J1979): test ID, sensor,
            // value, then optionally lower and upper limit, one byte each.
            const std::vector<int>& msg = result.messages.front();
            if (msg.size() < 3) continue;
            OxygenSensorTest test;
            test.testId = testId;
            test.sensor = sensor;
            test.name = oxygenTestName(testId);
            std::string unit;
            test.value = oxygenScale(testId, msg[2], unit);
            test.unit = unit;
            out.push_back(std::move(test));
        }
    }

    if (out.empty()) {
        error = L(
            "Das Steuergerät liefert keine Lambdasondenprüfung. Bei Fahrzeugen mit CAN "
            "ist dieser Dienst abgeschafft, die Werte stecken dort in der "
            "Bauteilüberwachung.",
            "The control unit provides no oxygen sensor test results. Vehicles with CAN "
            "no longer have this service; the values are part of component "
            "monitoring there.");
    }
    return out;
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
        // Mode 02 answers with PID and frame number in front of the data.
        if (data.size() >= 2 && data[0] == pid) data.erase(data.begin(), data.begin() + 2);
        const std::vector<PidReading> readings = decodePid(*spec, data);
        if (readings.empty() || readings.front().spec->sub != 0) continue;
        rows.push_back(makeValue(*spec, readings.front(), sourceAt(result, 0)));
    }
    return rows;
}

std::vector<int> Obd2::freezeFramePids(int frame) {
    // The same masks as for mode 01, but with the frame number after the
    // PID: request "0200 01" for frame 1, answer "42 00 01 BE 1F ...".
    std::set<int> found;
    for (int base = 0x00; base <= 0xC0; base += 0x20) {
        char command[16];
        std::snprintf(command, sizeof(command), "02%02X%02X", base, frame & 0xFF);
        const ElmResult result = elm_.request(command, 0x02, 4000);
        if (!result.ok()) break;
        bool next = false;
        for (const std::vector<int>& msg : result.messages) {
            if (msg.size() < 6 || msg[0] != base || msg[1] != frame) continue;
            const unsigned mask = (static_cast<unsigned>(msg[2]) << 24) |
                                  (static_cast<unsigned>(msg[3]) << 16) |
                                  (static_cast<unsigned>(msg[4]) << 8) |
                                  static_cast<unsigned>(msg[5]);
            for (int i = 0; i < 32; ++i) {
                if (mask & (1u << (31 - i))) found.insert(base + i + 1);
            }
            if (mask & 1u) next = true;
        }
        if (!next) break;
    }
    std::vector<int> pids;
    for (int pid : found) {
        // PID 02 is the fault code itself, the markers are no values.
        if (pid == 0x02 || pid % 0x20 == 0 || !findPid(pid)) continue;
        pids.push_back(pid);
    }
    if (pids.empty()) pids = {0x03, 0x04, 0x05, 0x06, 0x07, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11};
    return pids;
}

std::vector<FreezeFrame> Obd2::readFreezeFrames(int maxFrames) {
    std::vector<FreezeFrame> out;
    for (int frame = 0; frame < maxFrames && frame <= 0xFF; ++frame) {
        // PID 02 of the frame: which fault stored it. 00 00 means the frame
        // is empty, and so are all after it.
        char command[16];
        std::snprintf(command, sizeof(command), "0202%02X", frame);
        const ElmResult trigger = elm_.request(command, 0x02, 4000);
        if (!trigger.ok()) break;

        std::vector<FreezeFrame> found;
        for (size_t i = 0; i < trigger.messages.size(); ++i) {
            const std::vector<int>& msg = trigger.messages[i];
            // A control unit that ignores the frame number would hand out
            // frame 0 again and again; only an answer for this frame counts.
            if (msg.size() < 4 || msg[0] != 0x02 || msg[1] != frame) continue;
            const std::string code = decodeDtc(msg[2], msg[3]);
            if (code.empty()) continue;
            FreezeFrame f;
            f.frame = frame;
            f.dtc = code;
            f.ecuSource = sourceAt(trigger, i);
            f.ecu = ecuDisplayName(f.ecuSource);
            found.push_back(std::move(f));
        }
        if (found.empty()) break;

        std::vector<std::set<int>> filled(found.size());
        for (int pid : freezeFramePids(frame)) {
            const PidSpec* spec = findPid(pid);
            if (!spec) continue;
            std::snprintf(command, sizeof(command), "02%02X%02X", pid, frame);
            const ElmResult result = elm_.request(command, 0x02, 4000);
            if (!result.ok()) continue;
            for (size_t i = 0; i < result.messages.size(); ++i) {
                std::vector<int> data = result.messages[i];
                // Mode 02 answers with PID and frame number in front of the data.
                if (data.size() < 3 || data[0] != pid || data[1] != frame) continue;
                data.erase(data.begin(), data.begin() + 2);
                // The frame of the control unit that sent it. Without headers
                // every source is empty and the answers go in order.
                const std::string& source = sourceAt(result, i);
                for (size_t k = 0; k < found.size(); ++k) {
                    if (found[k].ecuSource != source || filled[k].count(pid)) continue;
                    filled[k].insert(pid);
                    for (const PidReading& reading : decodePid(*spec, data)) {
                        found[k].values.push_back(makeValue(*spec, reading, source));
                    }
                    break;
                }
            }
        }
        out.insert(out.end(), found.begin(), found.end());
    }
    return out;
}

}  // namespace caruspex
