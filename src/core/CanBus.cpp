#include "core/CanBus.h"

#include <algorithm>
#include <cctype>
#include <cstdio>
#include <cstdlib>
#include <map>

#include "core/Lang.h"
#include "core/Timing.h"

namespace caruspex {
namespace {

constexpr int kFirstAnswerMs = 1000;   // ELM327 default answer time (ATST)
constexpr int kMoreAnswersMs = 60;     // more units after the first one
constexpr int kPendingMs = 5000;       // after 7F xx 78

std::string upper(const std::string& s) {
    std::string out;
    for (char c : s) {
        if (c == '\r' || c == '\n' || c == ' ') continue;
        out += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    }
    return out;
}

bool isHex(const std::string& s) {
    return !s.empty() && std::all_of(s.begin(), s.end(), [](unsigned char c) { return std::isxdigit(c); });
}

std::vector<std::uint8_t> parseHex(const std::string& s) {
    std::vector<std::uint8_t> out;
    for (std::size_t i = 0; i + 1 < s.size(); i += 2)
        out.push_back(static_cast<std::uint8_t>(std::strtol(s.substr(i, 2).c_str(), nullptr, 16)));
    return out;
}

bool starts(const std::string& s, const char* prefix) { return s.rfind(prefix, 0) == 0; }

}  // namespace

// ---------------------------------------------------------------------------
// ELM327 front end
// ---------------------------------------------------------------------------

CanElm::CanElm(std::unique_ptr<CanLink> link, std::string name, int bitrate)
    : link_(std::move(link)), name_(std::move(name)), bitrate_(bitrate) {}

void CanElm::reset() {
    echo_ = true;
    linefeeds_ = false;
    spaces_ = true;
    headers_ = false;
    autoFormat_ = true;
    extended_ = false;
    header_ = 0x7DF;
    priority_ = 0x18;
    filter_ = -1;
    flowHeader_ = -1;
    monitoring_ = false;
}

std::string CanElm::hex(const std::vector<std::uint8_t>& bytes) const {
    std::string out;
    char b[4];
    for (std::size_t i = 0; i < bytes.size(); ++i) {
        std::snprintf(b, sizeof(b), "%02X", bytes[i]);
        if (spaces_ && i > 0) out += ' ';
        out += b;
    }
    return out;
}

std::string CanElm::line(const CanFrame& frame, bool withId) const {
    std::string out;
    if (withId) {
        char id[12];
        std::snprintf(id, sizeof(id), frame.extended ? "%08X" : "%03X", static_cast<unsigned>(frame.id));
        out = std::string(id) + (spaces_ ? " " : "");
    }
    return out + hex(frame.data);
}

bool CanElm::write(const std::string& data) {
    if (monitoring_) {
        // Any character ends the monitor; the prompt follows.
        monitoring_ = false;
        pending_ += std::string(linefeeds_ ? "\r\n" : "\r") + ">";
        return true;
    }
    const std::string command = upper(data);
    std::string out;
    if (echo_) out += data.substr(0, data.find('\r')) + "\r";
    const std::string end = linefeeds_ ? "\r\n" : "\r";
    const std::vector<std::string> lines = handle(command);
    for (const std::string& l : lines) out += l + end;
    if (!monitoring_) out += end + ">";
    pending_ += out;
    return true;
}

std::string CanElm::read(int timeoutMs) {
    if (monitoring_ && pending_.empty()) {
        // Frames as they come, through the receive filter.
        const std::uint64_t deadline = nowMs() + static_cast<std::uint64_t>(std::max(1, timeoutMs));
        CanFrame f;
        while (nowMs() < deadline && link_->receive(f, static_cast<int>(deadline - nowMs()))) {
            if (filter_ >= 0 && f.id != static_cast<std::uint32_t>(filter_)) continue;
            pending_ += line(f, headers_) + (linefeeds_ ? "\r\n" : "\r");
            if (pending_.size() > 200) break;
        }
    }
    std::string out;
    out.swap(pending_);
    return out;
}

void CanElm::flushInput() {
    pending_.clear();
    // Frames that arrived between two requests belong to nobody.
    CanFrame f;
    while (!monitoring_ && link_->receive(f, 0)) {
    }
}

std::vector<std::string> CanElm::handle(const std::string& command) {
    if (command.empty()) return {};
    if (starts(command, "AT")) return at(command.substr(2));
    if (starts(command, "ST")) return {"?"};
    std::string digits = command;
    if (digits.size() % 2 == 1) digits.pop_back();   // "expected answers" digit
    if (!isHex(digits)) return {"?"};
    return autoFormat_ ? request(parseHex(digits)) : raw(parseHex(digits));
}

std::vector<std::string> CanElm::at(const std::string& c) {
    const std::vector<std::string> ok = {"OK"};
    if (c == "Z" || c == "WS") {
        reset();
        return {"ELM327 v2.1"};
    }
    if (c == "D") {
        reset();
        return ok;
    }
    if (c == "I") return {"ELM327 v2.1"};
    if (c == "@1") return {name_};
    if (c == "E0" || c == "E1") { echo_ = c == "E1"; return ok; }
    if (c == "L0" || c == "L1") { linefeeds_ = c == "L1"; return ok; }
    if (c == "S0" || c == "S1") { spaces_ = c == "S1"; return ok; }
    if (c == "H0" || c == "H1") { headers_ = c == "H1"; return ok; }
    if (c == "CAF0" || c == "CAF1") { autoFormat_ = c == "CAF1"; return ok; }
    if (starts(c, "SP") || starts(c, "TP")) {
        // 6/8 are 11 bit, 7/9 are 29 bit; the bit rate is the interface's.
        const char digit = c.back();
        if (digit == '6' || digit == '8') extended_ = false;
        if (digit == '7' || digit == '9') extended_ = true;
        if (digit >= '1' && digit <= '5') return {"?"};   // no K-line or J1850 on a CAN interface
        header_ = extended_ ? 0xDB33F1 : 0x7DF;
        return ok;
    }
    if (c == "DP" || c == "DPN") {
        const bool slow = bitrate_ == 250000;
        if (c == "DPN") return {std::string(1, extended_ ? (slow ? '9' : '7') : (slow ? '8' : '6'))};
        char text[64];
        std::snprintf(text, sizeof(text), "ISO 15765-4 (CAN %d/%d)", extended_ ? 29 : 11, bitrate_ / 1000);
        return {text};
    }
    if (starts(c, "SH")) {
        const std::string id = c.substr(2);
        if (!isHex(id) || (id.size() != 3 && id.size() != 6 && id.size() != 8)) return {"?"};
        const auto value = static_cast<std::uint32_t>(std::strtoul(id.c_str(), nullptr, 16));
        if (id.size() == 8) {
            priority_ = value >> 24;
            header_ = value & 0xFFFFFF;
        } else {
            header_ = value;
        }
        return ok;
    }
    if (starts(c, "CP") && c.size() == 4 && isHex(c.substr(2))) {
        priority_ = static_cast<std::uint32_t>(std::strtoul(c.substr(2).c_str(), nullptr, 16)) & 0x1F;
        return ok;
    }
    if (c == "CRA" || c == "AR") { filter_ = -1; return ok; }
    if (starts(c, "CRA")) {
        const std::string id = c.substr(3);
        if (!isHex(id) || (id.size() != 3 && id.size() != 8)) return {"?"};
        filter_ = static_cast<std::int64_t>(std::strtoul(id.c_str(), nullptr, 16));
        return ok;
    }
    if (starts(c, "FCSH")) {
        const std::string id = c.substr(4);
        if (!isHex(id)) return {"?"};
        flowHeader_ = static_cast<std::int64_t>(std::strtoul(id.c_str(), nullptr, 16));
        return ok;
    }
    if (c == "FCSM0") { flowHeader_ = -1; return ok; }
    if (c == "MA") {
        monitoring_ = true;
        return {};
    }
    if (c == "RV") {
        const double v = link_->batteryVoltage();
        if (v < 0) return {"?"};
        char text[16];
        std::snprintf(text, sizeof(text), "%.1fV", v);
        return {text};
    }
    for (const char* accepted : {"AT", "ST", "FCSD", "FCSM", "PC", "M0", "R1", "AL", "NL", "V0", "V1", "D0", "D1",
                                 "CFC1", "CM", "CF"}) {
        if (starts(c, accepted)) return ok;
    }
    return {"?"};
}

bool CanElm::accepts(const CanFrame& f, bool functional) const {
    if (f.extended != extended_ || f.data.empty()) return false;
    if (filter_ >= 0) return f.id == static_cast<std::uint32_t>(filter_);
    if (!extended_) return functional ? (f.id >= 0x7E8 && f.id <= 0x7EF) : f.id == header_ + 8;
    if (functional) return (f.id & 0x1FFF0000u) == 0x18DA0000u && ((f.id >> 8) & 0xFF) == 0xF1;
    const std::uint32_t request = (priority_ << 24) | header_;
    const std::uint32_t swapped = (request & 0x1FFF0000u) | ((request & 0xFF) << 8) | ((request >> 8) & 0xFF);
    return f.id == swapped;
}

std::uint32_t CanElm::flowControlId(const CanFrame& r) const {
    if (flowHeader_ >= 0) {
        if (r.extended && flowHeader_ <= 0xFFFFFF) return (priority_ << 24) | static_cast<std::uint32_t>(flowHeader_);
        return static_cast<std::uint32_t>(flowHeader_);
    }
    if (!r.extended) return r.id - 8;
    return (r.id & 0x1FFF0000u) | ((r.id & 0xFF) << 8) | ((r.id >> 8) & 0xFF);
}

std::vector<std::string> CanElm::request(const std::vector<std::uint8_t>& bytes) {
    if (bytes.empty() || bytes.size() > 7) return {"?"};
    const bool functional = extended_ ? (header_ & 0xFFFF00) == 0xDB3300 : header_ == 0x7DF;
    CanFrame out;
    out.extended = extended_;
    out.id = extended_ ? ((priority_ << 24) | header_) : header_;
    out.data = {static_cast<std::uint8_t>(bytes.size())};
    out.data.insert(out.data.end(), bytes.begin(), bytes.end());
    out.data.resize(8, 0x00);
    flushInput();
    if (!link_->send(out)) return {"CAN ERROR"};

    struct Answer {
        std::vector<CanFrame> frames;
        std::size_t total = 0;
        std::size_t have = 0;
        bool multi = false;
        bool done = false;
    };
    std::vector<std::uint32_t> order;
    std::map<std::uint32_t, Answer> answers;
    std::uint64_t deadline = nowMs() + kFirstAnswerMs;
    CanFrame f;
    while (nowMs() < deadline) {
        if (!link_->receive(f, static_cast<int>(deadline - nowMs()))) break;
        if (!accepts(f, functional)) continue;
        if (!answers.count(f.id)) order.push_back(f.id);
        Answer& a = answers[f.id];
        if (a.done) continue;
        const int type = f.data[0] >> 4;
        if (type == 0) {
            const std::size_t len = f.data[0] & 0x0F;
            a.frames.push_back(f);
            const bool pending = len >= 3 && f.data.size() > 3 && f.data[1] == 0x7F && f.data[3] == 0x78;
            if (pending) {
                deadline = std::max(deadline, nowMs() + kPendingMs);
                continue;
            }
            a.done = true;
        } else if (type == 1 && f.data.size() >= 2) {
            a.multi = true;
            a.total = (static_cast<std::size_t>(f.data[0] & 0x0F) << 8) | f.data[1];
            a.have = f.data.size() - 2;
            a.frames.push_back(f);
            // Flow control: send everything, no pause between frames.
            CanFrame fc;
            fc.extended = f.extended;
            fc.id = flowControlId(f);
            fc.data = {0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
            link_->send(fc);
            deadline = std::max(deadline, nowMs() + kFirstAnswerMs);
            continue;
        } else if (type == 2 && a.multi) {
            a.frames.push_back(f);
            a.have += f.data.size() - 1;
            if (a.have >= a.total) a.done = true;
            else continue;
        } else {
            continue;
        }
        // A complete answer: a physical request is done, a functional one
        // waits a moment for further units.
        if (!functional) break;
        deadline = std::min(deadline, nowMs() + kMoreAnswersMs);
    }

    std::vector<std::string> lines;
    for (std::uint32_t id : order) {
        const Answer& a = answers[id];
        if (headers_) {
            for (const CanFrame& fr : a.frames) lines.push_back(line(fr, true));
            continue;
        }
        if (!a.multi) {
            for (const CanFrame& fr : a.frames) {
                const std::size_t len = std::min<std::size_t>(fr.data[0] & 0x0F, fr.data.size() - 1);
                lines.push_back(hex(std::vector<std::uint8_t>(fr.data.begin() + 1, fr.data.begin() + 1 + len)));
            }
            continue;
        }
        char length[8];
        std::snprintf(length, sizeof(length), "%03X", static_cast<unsigned>(a.total));
        lines.push_back(length);
        int index = 0;
        for (const CanFrame& fr : a.frames) {
            const std::size_t skip = (fr.data[0] >> 4) == 1 ? 2 : 1;
            char prefix[4];
            std::snprintf(prefix, sizeof(prefix), "%X:", index % 16);
            lines.push_back(std::string(prefix) + (spaces_ ? " " : "") +
                            hex(std::vector<std::uint8_t>(fr.data.begin() + static_cast<long>(skip), fr.data.end())));
            ++index;
        }
    }
    if (lines.empty()) return {"NO DATA"};
    return lines;
}

std::vector<std::string> CanElm::raw(const std::vector<std::uint8_t>& bytes) {
    if (bytes.empty() || bytes.size() > 8) return {"?"};
    CanFrame out;
    out.extended = extended_;
    out.id = extended_ ? ((priority_ << 24) | header_) : header_;
    out.data = bytes;
    flushInput();
    if (!link_->send(out)) return {"CAN ERROR"};
    // Whatever answers, as it comes: the caller does the framing.
    std::vector<std::string> lines;
    std::uint64_t deadline = nowMs() + kFirstAnswerMs;
    CanFrame f;
    while (nowMs() < deadline && link_->receive(f, static_cast<int>(deadline - nowMs()))) {
        if (f.extended != extended_) continue;
        if (filter_ >= 0 && f.id != static_cast<std::uint32_t>(filter_)) continue;
        lines.push_back(line(f, headers_));
        deadline = std::min(deadline, nowMs() + kMoreAnswersMs);
    }
    if (lines.empty()) return {"NO DATA"};
    return lines;
}

// ---------------------------------------------------------------------------
// SLCAN / Lawicel
// ---------------------------------------------------------------------------

SlcanLink::SlcanLink(std::unique_ptr<ITransport> serial) : serial_(std::move(serial)) {}

SlcanLink::~SlcanLink() {
    if (open_) serial_->write("C\r");
}

bool SlcanLink::command(const std::string& text, std::string* reply) {
    serial_->write(text + "\r");
    std::string got;
    const std::uint64_t deadline = nowMs() + 300;
    while (nowMs() < deadline) {
        got += serial_->read(30);
        if (got.find('\r') != std::string::npos || got.find('\a') != std::string::npos) break;
    }
    if (reply) *reply = got;
    return got.find('\a') == std::string::npos;
}

bool probeSlcan(ITransport& serial, std::string& version) {
    serial.write("\r\r\r");   // ends whatever half command is pending
    serial.read(50);
    serial.flushInput();
    serial.write("V\r");
    std::string got;
    const std::uint64_t deadline = nowMs() + 300;
    while (nowMs() < deadline && got.find('\r') == std::string::npos) got += serial.read(30);
    const std::size_t v = got.find('V');
    if (v == std::string::npos || got.size() < v + 5) return false;
    version = got.substr(v, 5);
    return std::all_of(version.begin() + 1, version.end(), [](unsigned char c) { return std::isxdigit(c); });
}

bool SlcanLink::open(int bitrate, std::string& error) {
    const std::pair<int, char> rates[] = {{10000, '0'}, {20000, '1'}, {50000, '2'},  {100000, '3'}, {125000, '4'},
                                          {250000, '5'}, {500000, '6'}, {800000, '7'}, {1000000, '8'}};
    char code = 0;
    for (const auto& [rate, c] : rates)
        if (rate == bitrate) code = c;
    if (!code) {
        error = L("Diese Bitrate kennt SLCAN nicht.", "SLCAN does not know this bit rate.");
        return false;
    }
    if (!probeSlcan(*serial_, version_)) {
        error = L("Der Anschluss antwortet nicht wie ein SLCAN-Adapter (CANable, USBtin, CANUSB).",
                  "The port does not answer like an SLCAN adapter (CANable, USBtin, CANUSB).");
        return false;
    }
    command("C");   // closed first, so the bit rate can be set
    if (!command(std::string("S") + code)) {
        error = L("Der Adapter nimmt die Bitrate nicht an.", "The adapter refuses the bit rate.");
        return false;
    }
    command("Z0");   // no time stamps
    if (!command("O")) {
        error = L("Der CAN-Kanal lässt sich nicht öffnen.", "The CAN channel cannot be opened.");
        return false;
    }
    open_ = true;
    return true;
}

bool SlcanLink::send(const CanFrame& frame) {
    char head[16];
    if (frame.extended) std::snprintf(head, sizeof(head), "T%08X%u", static_cast<unsigned>(frame.id), static_cast<unsigned>(frame.data.size()));
    else std::snprintf(head, sizeof(head), "t%03X%u", static_cast<unsigned>(frame.id & 0x7FF), static_cast<unsigned>(frame.data.size()));
    std::string text = head;
    char b[4];
    for (std::uint8_t byte : frame.data) {
        std::snprintf(b, sizeof(b), "%02X", byte);
        text += b;
    }
    return serial_->write(text + "\r");
}

bool SlcanLink::receive(CanFrame& frame, int timeoutMs) {
    const std::uint64_t deadline = nowMs() + static_cast<std::uint64_t>(std::max(0, timeoutMs));
    for (;;) {
        std::size_t end;
        while ((end = buffer_.find('\r')) != std::string::npos) {
            const std::string l = buffer_.substr(0, end);
            buffer_.erase(0, end + 1);
            if (l.empty() || (l[0] != 't' && l[0] != 'T')) continue;   // "z"/"Z" send acks, BEL
            const bool ext = l[0] == 'T';
            const std::size_t idLen = ext ? 8 : 3;
            if (l.size() < 1 + idLen + 1) continue;
            const std::string id = l.substr(1, idLen);
            const int dlc = l[1 + idLen] - '0';
            if (!isHex(id) || dlc < 0 || dlc > 8 || l.size() < 2 + idLen + static_cast<std::size_t>(dlc) * 2) continue;
            frame.extended = ext;
            frame.id = static_cast<std::uint32_t>(std::strtoul(id.c_str(), nullptr, 16));
            frame.data = parseHex(l.substr(2 + idLen, static_cast<std::size_t>(dlc) * 2));
            return true;
        }
        const std::uint64_t now = nowMs();
        if (now >= deadline) return false;
        buffer_ += serial_->read(static_cast<int>(std::min<std::uint64_t>(deadline - now, 50)));
        if (timeoutMs == 0) {
            if (buffer_.find('\r') == std::string::npos) return false;
        }
    }
}

std::string SlcanLink::name() const { return "SLCAN " + serial_->name() + (version_.empty() ? "" : " " + version_); }

// ---------------------------------------------------------------------------
// Choosing an interface
// ---------------------------------------------------------------------------

std::unique_ptr<ITransport> openCanInterface(const std::string& spec, std::string& error) {
    std::string target = spec;
    int bitrate = 500000;
    const std::size_t at = target.rfind('@');
    if (at != std::string::npos) {
        bitrate = std::atoi(target.substr(at + 1).c_str());
        target = target.substr(0, at);
    }
    const std::size_t colon = target.find(':');
    const std::string kind = colon == std::string::npos ? target : target.substr(0, colon);
    const std::string where = colon == std::string::npos ? std::string() : target.substr(colon + 1);
    char rate[32];
    std::snprintf(rate, sizeof(rate), " (%d kbit/s)", bitrate / 1000);

    std::unique_ptr<CanLink> link;
    if (kind == "slcan") {
        auto serial = std::make_unique<SerialTransport>();
        if (!serial->open(where, 115200, error)) return nullptr;
        auto slcan = std::make_unique<SlcanLink>(std::move(serial));
        if (!slcan->open(bitrate, error)) return nullptr;
        link = std::move(slcan);
    } else if (kind == "socketcan") {
        link = openSocketCan(where.empty() ? std::string("can0") : where, error);
    } else if (kind == "j2534") {
        std::string library = where;
        for (const J2534Device& d : j2534Devices())
            if (d.name == where) library = d.library;
        if (library.empty()) {
            const auto all = j2534Devices();
            const auto usable = std::find_if(all.begin(), all.end(), [](const J2534Device& d) { return d.usable; });
            if (usable != all.end()) library = usable->library;
        }
        link = openJ2534(library, bitrate, error);
    } else {
        error = L("Unbekannte CAN-Schnittstelle: ", "Unknown CAN interface: ") + spec;
        return nullptr;
    }
    if (!link) return nullptr;
    const std::string name = link->name() + rate;
    return std::make_unique<CanElm>(std::move(link), name, bitrate);
}

}  // namespace caruspex
