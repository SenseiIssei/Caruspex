#include "core/Enet.h"

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <arpa/inet.h>
#include <fcntl.h>
#include <netdb.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/select.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/types.h>
#include <unistd.h>
#endif

#include <algorithm>
#include <cctype>
#include <cerrno>
#include <cstdio>
#include <cstring>

#include "core/Lang.h"
#include "core/Timing.h"

namespace caruspex {

// ---------------------------------------------------------------------------
// HSFZ frames
// ---------------------------------------------------------------------------

namespace hsfz {

ByteVector encode(int control, const ByteVector& payload) {
    const auto size = static_cast<std::uint32_t>(payload.size());
    ByteVector out = {static_cast<std::uint8_t>(size >> 24), static_cast<std::uint8_t>(size >> 16),
                      static_cast<std::uint8_t>(size >> 8), static_cast<std::uint8_t>(size),
                      0x00, static_cast<std::uint8_t>(control & 0xFF)};
    out.insert(out.end(), payload.begin(), payload.end());
    return out;
}

void Parser::feed(const std::uint8_t* data, std::size_t size) { buffer_.insert(buffer_.end(), data, data + size); }

bool Parser::next(Frame& frame) {
    if (broken_ || buffer_.size() < 6) return false;
    const std::uint32_t length = (std::uint32_t(buffer_[0]) << 24) | (std::uint32_t(buffer_[1]) << 16) |
                                 (std::uint32_t(buffer_[2]) << 8) | std::uint32_t(buffer_[3]);
    // A UDS answer is at most 4095 bytes plus the two addresses.
    if (length > 0x10000) {
        broken_ = true;
        return false;
    }
    if (buffer_.size() < 6 + length) return false;
    frame.control = buffer_[5];
    frame.payload.assign(buffer_.begin() + 6, buffer_.begin() + 6 + static_cast<long>(length));
    buffer_.erase(buffer_.begin(), buffer_.begin() + 6 + static_cast<long>(length));
    return true;
}

bool parseIdent(const ByteVector& d, VehicleIdent& out) {
    auto text = [&d](std::size_t at, std::size_t count) {
        std::string s;
        for (std::size_t i = at; i < at + count && i < d.size(); ++i) s += static_cast<char>(d[i]);
        return s;
    };
    if (d.size() < 44 || text(6, 7) != "DIAGADR") return false;
    const std::string address = text(13, 2);
    out.diagAddress = std::isxdigit(static_cast<unsigned char>(address[0])) &&
                              std::isxdigit(static_cast<unsigned char>(address[1]))
                          ? std::stoi(address, nullptr, 16)
                          : -1;
    out.mac = text(21, 12);
    out.vin.clear();
    if (d.size() >= 56) {
        out.vin = text(39, 17);
        const bool printable = std::all_of(out.vin.begin(), out.vin.end(),
                                           [](char c) { return std::isalnum(static_cast<unsigned char>(c)); });
        if (!printable) out.vin.clear();
    }
    return true;
}

}  // namespace hsfz

// ---------------------------------------------------------------------------
// Session: acknowledgement, alive check, response pending
// ---------------------------------------------------------------------------

namespace {

constexpr int kPendingExtraMs = 5000;   // after every 7F xx 78
constexpr int kPendingLimitMs = 30000;  // overall, for very slow units
constexpr int kFunctionalQuietMs = 250; // more answers after the first one

}  // namespace

HsfzSession::HsfzSession(std::unique_ptr<ByteStream> stream, int tester)
    : stream_(std::move(stream)), tester_(tester) {}

bool HsfzSession::readFrame(hsfz::Frame& frame, std::uint64_t deadline) {
    std::uint8_t buffer[4096];
    while (!parser_.next(frame)) {
        if (parser_.broken()) {
            closed_ = true;
            return false;
        }
        const std::uint64_t now = nowMs();
        if (now >= deadline) return false;
        const int got = stream_->receive(buffer, sizeof(buffer), static_cast<int>(deadline - now));
        if (got < 0) {
            closed_ = true;
            return false;
        }
        if (got > 0) parser_.feed(buffer, static_cast<std::size_t>(got));
    }
    return true;
}

void HsfzSession::answerAliveCheck() {
    stream_->send(hsfz::encode(hsfz::AliveReply, {0x00, static_cast<std::uint8_t>(tester_)}));
}

bool HsfzSession::request(int target, const ByteVector& data, int timeoutMs, std::vector<DiagAnswer>& answers,
                          std::string& error) {
    answers.clear();
    error.clear();
    if (closed_) {
        error = L("Die ENET-Verbindung zum Auto ist getrennt.", "The ENET connection to the car is gone.");
        return false;
    }
    ByteVector payload = {static_cast<std::uint8_t>(tester_), static_cast<std::uint8_t>(target)};
    payload.insert(payload.end(), data.begin(), data.end());
    const ByteVector frame = hsfz::encode(hsfz::Diagnostic, payload);
    if (!stream_->send(frame)) {
        closed_ = true;
        error = L("Senden über ENET ging nicht.", "Sending over ENET failed.");
        return false;
    }

    const bool functional = target == hsfz::kFunctional;
    const std::uint64_t start = nowMs();
    std::uint64_t deadline = start + static_cast<std::uint64_t>(std::max(timeoutMs, 300));
    bool acknowledged = false;
    int resent = 0;
    hsfz::Frame in;
    while (readFrame(in, deadline)) {
        switch (in.control) {
            case hsfz::Ack:
                acknowledged = true;
                break;
            case hsfz::Nack:
                // The gateway did not take it; one more try, as EdiabasLib does.
                if (resent++ == 0 && stream_->send(frame)) break;
                error = L("Das Gateway nimmt die Anfrage nicht an.", "The gateway refuses the request.");
                return false;
            case hsfz::AliveCheck:
                answerAliveCheck();
                break;
            case hsfz::Diagnostic: {
                if (in.payload.size() < 3) break;
                const int source = in.payload[0];
                const int dest = in.payload[1];
                if (dest != tester_) break;
                if (!functional && source != target) break;
                DiagAnswer answer{source, ByteVector(in.payload.begin() + 2, in.payload.end())};
                const ByteVector& d = answer.data;
                if (d.size() >= 3 && d[0] == 0x7F && d[2] == 0x78) {
                    // Still working: wait longer, up to an overall limit.
                    deadline = std::min(nowMs() + kPendingExtraMs, start + kPendingLimitMs);
                    break;
                }
                answers.push_back(std::move(answer));
                if (!functional) return true;
                deadline = std::min(deadline, nowMs() + kFunctionalQuietMs);
                break;
            }
            default:
                // 0x40 to 0x4F are the gateway's error telegrams (unknown
                // target, wrong format, out of memory): no answer will come.
                if (in.control >= 0x40 && in.control <= 0x4F) {
                    char code[8];
                    std::snprintf(code, sizeof(code), "0x%02X", in.control);
                    error = std::string(L("Das Gateway meldet einen Fehler ", "The gateway reports error ")) +
                            code +
                            L(" (zum Beispiel ein unbekanntes Steuergerät).", " (for example an unknown control unit).");
                    return false;
                }
                break;   // other telegrams are ignored
        }
    }
    if (!answers.empty()) return true;
    if (closed_) {
        error = L("Das Auto hat die ENET-Verbindung getrennt.", "The car closed the ENET connection.");
    } else if (!acknowledged) {
        error = L("Das Gateway bestätigt die Anfrage nicht.", "The gateway does not acknowledge the request.");
    } else {
        error = L("Keine Antwort vom Steuergerät.", "No answer from the control unit.");
    }
    return false;
}

// ---------------------------------------------------------------------------
// ELM327 front end
// ---------------------------------------------------------------------------

namespace {

constexpr const char* kBanner = "ELM327 v2.1";
// UDS gives a unit 50 ms for its first answer (P2), a busy one says
// 7F xx 78 and then gets more time in the session. So the waits can be
// short, which matters when a scan asks 128 addresses: "are you there" (3E)
// shortest, everything else a little longer.
constexpr int kRequestTimeoutMs = 1500;
int timeoutFor(const ByteVector& request) {
    return !request.empty() && request[0] == 0x3E ? 600 : kRequestTimeoutMs;
}

std::string normalize(const std::string& data) {
    std::string out;
    for (char c : data) {
        if (c == '\r' || c == '\n' || c == ' ') continue;
        out += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    }
    return out;
}

bool isHex(const std::string& s) {
    return !s.empty() && std::all_of(s.begin(), s.end(), [](unsigned char c) { return std::isxdigit(c); });
}

ByteVector parseHex(const std::string& s) {
    ByteVector out;
    for (std::size_t i = 0; i + 1 < s.size(); i += 2) out.push_back(static_cast<std::uint8_t>(std::stoi(s.substr(i, 2), nullptr, 16)));
    return out;
}

bool startsWith(const std::string& s, const char* prefix) { return s.rfind(prefix, 0) == 0; }

}  // namespace

EnetElm::EnetElm(std::unique_ptr<DiagLink> link, std::string name, Flavor flavor)
    : link_(std::move(link)), name_(std::move(name)), flavor_(flavor) {}

void EnetElm::reset() {
    echo_ = true;
    linefeeds_ = false;
    spaces_ = true;
    headers_ = false;
    autoFormat_ = true;
    header_ = 0x7DF;
    receiveFilter_ = -1;
    rawTarget_ = -1;
    rawPending_.clear();
}

std::string EnetElm::hex(const ByteVector& bytes) const {
    std::string out;
    char b[4];
    for (std::size_t i = 0; i < bytes.size(); ++i) {
        std::snprintf(b, sizeof(b), "%02X", bytes[i]);
        if (spaces_ && i > 0) out += ' ';
        out += b;
    }
    return out;
}

std::string EnetElm::idText(int id) const {
    char b[8];
    std::snprintf(b, sizeof(b), "%03X", id & 0x7FF);
    return std::string(b) + (spaces_ ? " " : "");
}

bool EnetElm::write(const std::string& data) {
    const std::string command = normalize(data);
    std::string out;
    if (echo_) out += data.substr(0, data.find('\r')) + "\r";
    const std::string end = linefeeds_ ? "\r\n" : "\r";
    for (const std::string& line : handle(command)) out += line + end;
    out += end + ">";
    pending_ += out;
    return true;
}

std::string EnetElm::read(int timeoutMs) {
    (void)timeoutMs;   // the answer is complete when write returns
    std::string out;
    out.swap(pending_);
    return out;
}

std::vector<std::string> EnetElm::handle(const std::string& command) {
    if (command.empty()) return {};
    if (startsWith(command, "AT")) return at(command.substr(2));
    if (startsWith(command, "ST")) return {"?"};   // no STN commands here
    std::string digits = command;
    // A trailing single digit is the ELM327's "expected answers" hint.
    if (digits.size() % 2 == 1) digits.pop_back();
    if (!isHex(digits)) return {"?"};
    return autoFormat_ ? obd(parseHex(digits)) : raw(parseHex(digits));
}

std::vector<std::string> EnetElm::at(const std::string& c) {
    const std::vector<std::string> ok = {"OK"};
    if (c == "Z" || c == "WS" || c == "D") {
        reset();
        return c == "D" ? ok : std::vector<std::string>{kBanner};
    }
    if (c == "I") return {kBanner};
    if (c == "@1") return {"Caruspex ENET (BMW HSFZ)"};
    if (c == "E0" || c == "E1") { echo_ = c == "E1"; return ok; }
    if (c == "L0" || c == "L1") { linefeeds_ = c == "L1"; return ok; }
    if (c == "S0" || c == "S1") { spaces_ = c == "S1"; return ok; }
    if (c == "H0" || c == "H1") { headers_ = c == "H1"; return ok; }
    if (c == "CAF0" || c == "CAF1") { autoFormat_ = c == "CAF1"; return ok; }
    if (c == "DP") return {"ISO 15765-4 (CAN 11/500)"};
    if (c == "DPN") return {"6"};
    if (startsWith(c, "SH")) {
        const std::string id = c.substr(2);
        if (id.size() != 3 || !isHex(id)) return {"?"};   // only 11-bit ids
        header_ = std::stoi(id, nullptr, 16);
        return ok;
    }
    if (c == "CRA" || c == "AR") { receiveFilter_ = -1; return ok; }
    if (startsWith(c, "CRA")) {
        const std::string id = c.substr(3);
        if (id.size() != 3 || !isHex(id)) return {"?"};
        receiveFilter_ = std::stoi(id, nullptr, 16);
        return ok;
    }
    if (c == "RV") {
        // No voltage pin in the cable: the engine unit's own reading (PID 42).
        std::vector<DiagAnswer> answers;
        std::string error;
        const int engine = flavor_ == Flavor::Doip ? doip::kObdFunctional : hsfz::kEngine;
        if (link_->request(engine, {0x01, 0x42}, kRequestTimeoutMs, answers, error) && !answers.empty() &&
            answers.front().data.size() >= 4 && answers.front().data[0] == 0x41) {
            const ByteVector& d = answers.front().data;
            char volts[16];
            std::snprintf(volts, sizeof(volts), "%.1fV", ((d[2] << 8) | d[3]) / 1000.0);
            return {volts};
        }
        return {"?"};
    }
    // Settings that change nothing over Ethernet.
    for (const char* accepted : {"SP", "TP", "AT", "ST", "FCSH", "FCSD", "FCSM", "CP", "PC", "M0", "R1",
                                 "AL", "NL", "V0", "V1", "D0", "D1", "CFC1"}) {
        if (startsWith(c, accepted)) return ok;
    }
    return {"?"};
}

// Normal addressing: OBD (7DF, 7E0, 7E1) to the engine and the gearbox,
// framed by "the adapter" as an ELM327 with CAN auto formatting would.
std::vector<std::string> EnetElm::obd(const ByteVector& bytes) {
    if (bytes.empty() || bytes.size() > 7) return {"?"};
    int target = -1;
    const bool functional = header_ == 0x7DF;
    if (flavor_ == Flavor::Doip) {
        // Functional to all OBD units; 7E0 + n to the unit that answered as
        // 7E8 + n before.
        if (functional) target = doip::kObdFunctional;
        else if (header_ >= 0x7E0 && header_ <= 0x7E7 && static_cast<size_t>(header_ - 0x7E0) < learned_.size())
            target = learned_[static_cast<size_t>(header_ - 0x7E0)];
    } else if (functional || header_ == 0x7E0) {
        target = hsfz::kEngine;
    } else if (header_ == 0x7E1) {
        target = hsfz::kGearbox;
    }
    if (target < 0) return {"NO DATA"};

    std::vector<DiagAnswer> answers;
    std::string error;
    if (!link_->request(target, bytes, timeoutFor(bytes), answers, error)) return {"NO DATA"};

    std::vector<std::string> lines;
    for (const DiagAnswer& a : answers) {
        // Functional requests get no refusals, as on the bus.
        if (functional && !a.data.empty() && a.data[0] == 0x7F) continue;
        int id = a.source == hsfz::kGearbox ? 0x7E9 : 0x7E8;
        if (flavor_ == Flavor::Doip) {
            auto known = std::find(learned_.begin(), learned_.end(), a.source);
            if (known == learned_.end()) {
                if (learned_.size() >= 8) continue;
                learned_.push_back(a.source);
                known = learned_.end() - 1;
            }
            id = 0x7E8 + static_cast<int>(known - learned_.begin());
        }
        if (receiveFilter_ >= 0 && receiveFilter_ != id) continue;
        const ByteVector& d = a.data;
        if (d.size() <= 7) {
            if (headers_) {
                ByteVector frame = {static_cast<std::uint8_t>(d.size())};
                frame.insert(frame.end(), d.begin(), d.end());
                frame.resize(8, 0xAA);
                lines.push_back(idText(id) + hex(frame));
            } else {
                lines.push_back(hex(d));
            }
            continue;
        }
        // Multi frame: first frame with six bytes, then seven each.
        if (headers_) {
            ByteVector first = {static_cast<std::uint8_t>(0x10 | ((d.size() >> 8) & 0x0F)),
                                static_cast<std::uint8_t>(d.size() & 0xFF)};
            first.insert(first.end(), d.begin(), d.begin() + 6);
            lines.push_back(idText(id) + hex(first));
            int index = 1;
            for (std::size_t at = 6; at < d.size(); at += 7, ++index) {
                ByteVector cf = {static_cast<std::uint8_t>(0x20 | (index & 0x0F))};
                cf.insert(cf.end(), d.begin() + static_cast<long>(at),
                          d.begin() + static_cast<long>(std::min(at + 7, d.size())));
                cf.resize(8, 0xAA);
                lines.push_back(idText(id) + hex(cf));
            }
        } else {
            char length[8];
            std::snprintf(length, sizeof(length), "%03X", static_cast<unsigned>(d.size()));
            lines.push_back(length);
            int index = 0;
            for (std::size_t at = 0; at < d.size(); at += (at == 0 ? 6 : 7), ++index) {
                const std::size_t take = at == 0 ? 6 : 7;
                ByteVector part(d.begin() + static_cast<long>(at),
                                d.begin() + static_cast<long>(std::min(at + take, d.size())));
                lines.push_back(std::to_string(index % 10) + (spaces_ ? ": " : ":") + hex(part));
            }
        }
    }
    if (lines.empty()) return {"NO DATA"};
    return lines;
}

// CAN auto formatting off with ATSH 6F1: BMW style extended addressing,
// the ISO-TP frames come from the tester as they would go on the bus.
std::vector<std::string> EnetElm::raw(const ByteVector& bytes) {
    if (flavor_ != Flavor::Bmw || header_ != 0x6F1 || bytes.size() < 2) return {"NO DATA"};
    const int target = bytes[0];
    const int pci = bytes[1];
    const int responseId = 0x600 | target;
    if (receiveFilter_ >= 0 && receiveFilter_ != responseId) return {"NO DATA"};
    auto line = [&](const ByteVector& frame) {
        ByteVector padded = frame;
        padded.resize(8, 0x00);
        return (headers_ ? idText(responseId) : std::string()) + hex(padded);
    };

    if ((pci >> 4) == 3) {
        // Flow control from the tester: out with the rest of the long answer.
        if (target != rawTarget_ || rawPending_.empty()) return {"NO DATA"};
        std::vector<std::string> lines;
        for (const ByteVector& cf : rawPending_) lines.push_back(line(cf));
        rawPending_.clear();
        rawTarget_ = -1;
        return lines;
    }
    if ((pci >> 4) != 0) return {"NO DATA"};   // only single frame requests
    const std::size_t length = static_cast<std::size_t>(pci & 0x0F);
    if (length == 0 || bytes.size() < 2 + length) return {"NO DATA"};
    const ByteVector request(bytes.begin() + 2, bytes.begin() + 2 + static_cast<long>(length));

    std::vector<DiagAnswer> answers;
    std::string error;
    if (!link_->request(target, request, timeoutFor(request), answers, error)) return {"NO DATA"};
    const ByteVector& d = answers.front().data;
    const std::uint8_t tester = 0xF1;   // what a D-CAN tester would see
    if (d.size() <= 6) {
        ByteVector frame = {tester, static_cast<std::uint8_t>(d.size())};
        frame.insert(frame.end(), d.begin(), d.end());
        return {line(frame)};
    }
    // First frame now, the consecutive frames after the flow control.
    ByteVector first = {tester, static_cast<std::uint8_t>(0x10 | ((d.size() >> 8) & 0x0F)),
                        static_cast<std::uint8_t>(d.size() & 0xFF)};
    first.insert(first.end(), d.begin(), d.begin() + 5);
    rawPending_.clear();
    int index = 1;
    for (std::size_t at = 5; at < d.size(); at += 6, ++index) {
        ByteVector cf = {tester, static_cast<std::uint8_t>(0x20 | (index & 0x0F))};
        cf.insert(cf.end(), d.begin() + static_cast<long>(at), d.begin() + static_cast<long>(std::min(at + 6, d.size())));
        rawPending_.push_back(cf);
    }
    rawTarget_ = target;
    return {line(first)};
}

// ---------------------------------------------------------------------------
// DoIP
// ---------------------------------------------------------------------------

namespace doip {

ByteVector encode(int type, const ByteVector& payload, std::uint8_t version) {
    const auto size = static_cast<std::uint32_t>(payload.size());
    ByteVector out = {version, static_cast<std::uint8_t>(~version), static_cast<std::uint8_t>(type >> 8),
                      static_cast<std::uint8_t>(type), static_cast<std::uint8_t>(size >> 24),
                      static_cast<std::uint8_t>(size >> 16), static_cast<std::uint8_t>(size >> 8),
                      static_cast<std::uint8_t>(size)};
    out.insert(out.end(), payload.begin(), payload.end());
    return out;
}

void Parser::feed(const std::uint8_t* data, std::size_t size) { buffer_.insert(buffer_.end(), data, data + size); }

bool Parser::next(Message& message) {
    if (broken_ || buffer_.size() < 8) return false;
    if (static_cast<std::uint8_t>(buffer_[0] ^ buffer_[1]) != 0xFF) {
        broken_ = true;   // the inverse version does not match: not DoIP
        return false;
    }
    const std::uint32_t length = (std::uint32_t(buffer_[4]) << 24) | (std::uint32_t(buffer_[5]) << 16) |
                                 (std::uint32_t(buffer_[6]) << 8) | std::uint32_t(buffer_[7]);
    if (length > 0x10000) {
        broken_ = true;
        return false;
    }
    if (buffer_.size() < 8 + length) return false;
    message.type = (buffer_[2] << 8) | buffer_[3];
    message.payload.assign(buffer_.begin() + 8, buffer_.begin() + 8 + static_cast<long>(length));
    buffer_.erase(buffer_.begin(), buffer_.begin() + 8 + static_cast<long>(length));
    return true;
}

bool parseAnnouncement(const ByteVector& p, Entity& out) {
    if (p.size() < 32) return false;   // VIN 17, address 2, EID 6, GID 6, further action 1
    out.vin.clear();
    for (std::size_t i = 0; i < 17; ++i)
        if (std::isalnum(p[i])) out.vin += static_cast<char>(p[i]);
    if (out.vin.size() != 17) out.vin.clear();
    out.logicalAddress = (p[17] << 8) | p[18];
    return true;
}

}  // namespace doip

DoipSession::DoipSession(std::unique_ptr<ByteStream> stream, int tester) : stream_(std::move(stream)), tester_(tester) {}

bool DoipSession::readMessage(doip::Message& message, std::uint64_t deadline) {
    std::uint8_t buffer[4096];
    while (!parser_.next(message)) {
        if (parser_.broken()) {
            closed_ = true;
            return false;
        }
        const std::uint64_t now = nowMs();
        if (now >= deadline) return false;
        const int got = stream_->receive(buffer, sizeof(buffer), static_cast<int>(deadline - now));
        if (got < 0) {
            closed_ = true;
            return false;
        }
        if (got > 0) parser_.feed(buffer, static_cast<std::size_t>(got));
        // The entity may ask whether we are still there at any time.
    }
    if (message.type == doip::AliveCheckRequest) {
        stream_->send(doip::encode(doip::AliveCheckResponse,
                                   {static_cast<std::uint8_t>(tester_ >> 8), static_cast<std::uint8_t>(tester_)}));
    }
    return true;
}

bool DoipSession::activate(std::string& error) {
    const ByteVector request = {static_cast<std::uint8_t>(tester_ >> 8), static_cast<std::uint8_t>(tester_), 0x00,
                                0x00, 0x00, 0x00, 0x00};
    if (!stream_->send(doip::encode(doip::RoutingActivationRequest, request))) {
        error = L("Senden über DoIP ging nicht.", "Sending over DoIP failed.");
        return false;
    }
    const std::uint64_t deadline = nowMs() + 3000;
    doip::Message in;
    while (readMessage(in, deadline)) {
        if (in.type == doip::GenericNack) {
            error = L("Das Fahrzeug lehnt die DoIP-Nachricht ab.", "The vehicle rejects the DoIP message.");
            return false;
        }
        if (in.type != doip::RoutingActivationResponse || in.payload.size() < 5) continue;
        const int code = in.payload[4];
        if (code == 0x10) return true;
        char text[8];
        std::snprintf(text, sizeof(text), "0x%02X", code);
        error = std::string(L("Das Fahrzeug gibt die Diagnose nicht frei (Code ", "The vehicle does not grant routing (code ")) +
                text + L("). Manche Hersteller verlangen dafür eine Anmeldung in ihrer Software.",
                         "). Some makers require a login in their own software for it.");
        return false;
    }
    error = L("Keine Antwort auf die DoIP-Freigabe.", "No answer to the DoIP routing activation.");
    return false;
}

bool DoipSession::request(int target, const ByteVector& data, int timeoutMs, std::vector<DiagAnswer>& answers,
                          std::string& error) {
    answers.clear();
    error.clear();
    if (closed_) {
        error = L("Die DoIP-Verbindung zum Auto ist getrennt.", "The DoIP connection to the car is gone.");
        return false;
    }
    ByteVector payload = {static_cast<std::uint8_t>(tester_ >> 8), static_cast<std::uint8_t>(tester_),
                          static_cast<std::uint8_t>(target >> 8), static_cast<std::uint8_t>(target)};
    payload.insert(payload.end(), data.begin(), data.end());
    if (!stream_->send(doip::encode(doip::DiagnosticMessage, payload))) {
        closed_ = true;
        error = L("Senden über DoIP ging nicht.", "Sending over DoIP failed.");
        return false;
    }
    const bool functional = target >= 0xE000;
    const std::uint64_t start = nowMs();
    std::uint64_t deadline = start + static_cast<std::uint64_t>(std::max(timeoutMs, 300));
    doip::Message in;
    while (readMessage(in, deadline)) {
        if (in.type == doip::DiagnosticNack) {
            char text[8];
            std::snprintf(text, sizeof(text), "0x%02X", in.payload.size() > 4 ? in.payload[4] : 0);
            error = std::string(L("Das Fahrzeug nimmt die Anfrage nicht an (DoIP ", "The vehicle refuses the request (DoIP ")) +
                    text + ").";
            return false;
        }
        if (in.type != doip::DiagnosticMessage || in.payload.size() < 5) continue;
        const int source = (in.payload[0] << 8) | in.payload[1];
        const int dest = (in.payload[2] << 8) | in.payload[3];
        if (dest != tester_) continue;
        if (!functional && source != target) continue;
        DiagAnswer answer{source, ByteVector(in.payload.begin() + 4, in.payload.end())};
        const ByteVector& d = answer.data;
        if (d.size() >= 3 && d[0] == 0x7F && d[2] == 0x78) {
            deadline = std::min(nowMs() + kPendingExtraMs, start + kPendingLimitMs);
            continue;
        }
        answers.push_back(std::move(answer));
        if (!functional) return true;
        deadline = std::min(deadline, nowMs() + kFunctionalQuietMs);
    }
    if (!answers.empty()) return true;
    error = closed_ ? L("Das Auto hat die DoIP-Verbindung getrennt.", "The car closed the DoIP connection.")
                    : L("Keine Antwort vom Steuergerät.", "No answer from the control unit.");
    return false;
}

// ---------------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------------

namespace {

#ifdef _WIN32
using Socket = SOCKET;
constexpr Socket kNoSocket = INVALID_SOCKET;
bool startNetwork() {
    static const bool ready = [] {
        WSADATA wsa{};
        return WSAStartup(MAKEWORD(2, 2), &wsa) == 0;
    }();
    return ready;
}
void closeSocket(Socket s) { ::closesocket(s); }
void setNonBlocking(Socket s, bool on) {
    u_long mode = on ? 1 : 0;
    ::ioctlsocket(s, FIONBIO, &mode);
}
#else
using Socket = int;
constexpr Socket kNoSocket = -1;
bool startNetwork() { return true; }
void closeSocket(Socket s) { ::close(s); }
void setNonBlocking(Socket s, bool on) {
    const int flags = ::fcntl(s, F_GETFL, 0);
    if (flags >= 0) ::fcntl(s, F_SETFL, on ? (flags | O_NONBLOCK) : (flags & ~O_NONBLOCK));
}
#endif

// Waits until the socket can be read (or written), true if so.
bool waitFor(Socket s, int timeoutMs, bool forWrite) {
    fd_set set;
    FD_ZERO(&set);
    FD_SET(s, &set);
    timeval tv{};
    tv.tv_sec = timeoutMs / 1000;
    tv.tv_usec = (timeoutMs % 1000) * 1000;
    const int n = ::select(static_cast<int>(s) + 1, forWrite ? nullptr : &set, forWrite ? &set : nullptr,
                           nullptr, &tv);
    return n > 0;
}

class TcpStream : public ByteStream {
public:
    explicit TcpStream(Socket s) : socket_(s) {}
    ~TcpStream() override { closeSocket(socket_); }
    bool send(const ByteVector& data) override {
        const auto sent = ::send(socket_, reinterpret_cast<const char*>(data.data()), static_cast<int>(data.size()), 0);
        return sent == static_cast<decltype(sent)>(data.size());
    }
    int receive(std::uint8_t* buffer, int size, int timeoutMs) override {
        if (!waitFor(socket_, timeoutMs, false)) return 0;
        const auto got = ::recv(socket_, reinterpret_cast<char*>(buffer), size, 0);
        if (got <= 0) return -1;
        return static_cast<int>(got);
    }

private:
    Socket socket_;
};

Socket connectTcp(const std::string& host, int port, int timeoutMs) {
    sockaddr_in address{};
    address.sin_family = AF_INET;
    address.sin_port = htons(static_cast<std::uint16_t>(port));
    if (::inet_pton(AF_INET, host.c_str(), &address.sin_addr) != 1) return kNoSocket;
    const Socket s = ::socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (s == kNoSocket) return kNoSocket;
    setNonBlocking(s, true);
    ::connect(s, reinterpret_cast<sockaddr*>(&address), sizeof(address));
    bool ok = waitFor(s, timeoutMs, true);
    if (ok) {
        int problem = 0;
        socklen_t length = sizeof(problem);
        ::getsockopt(s, SOL_SOCKET, SO_ERROR, reinterpret_cast<char*>(&problem), &length);
        ok = problem == 0;
    }
    if (!ok) {
        closeSocket(s);
        return kNoSocket;
    }
    setNonBlocking(s, false);
    int one = 1;
    ::setsockopt(s, IPPROTO_TCP, TCP_NODELAY, reinterpret_cast<const char*>(&one), sizeof(one));
    return s;
}

}  // namespace

std::vector<hsfz::VehicleIdent> discoverEnet(int timeoutMs, std::string& error) {
    std::vector<hsfz::VehicleIdent> found;
    if (!startNetwork()) {
        error = L("Netzwerk lässt sich nicht starten.", "Cannot start networking.");
        return found;
    }
    const Socket s = ::socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (s == kNoSocket) {
        error = L("Kein UDP-Socket für die Fahrzeugsuche.", "No UDP socket for the vehicle search.");
        return found;
    }
    int one = 1;
    ::setsockopt(s, SOL_SOCKET, SO_BROADCAST, reinterpret_cast<const char*>(&one), sizeof(one));
    sockaddr_in local{};
    local.sin_family = AF_INET;
    ::bind(s, reinterpret_cast<sockaddr*>(&local), sizeof(local));

    const ByteVector request = hsfz::encode(hsfz::Ident, {});
    const std::uint64_t deadline = nowMs() + static_cast<std::uint64_t>(timeoutMs);
    for (int attempt = 0; attempt < 3 && found.empty(); ++attempt) {
        // The plain broadcast leaves on the default interface only; the
        // link-local one reaches the ENET cable on a second network port.
        for (const char* target : {"255.255.255.255", "169.254.255.255"}) {
            sockaddr_in to{};
            to.sin_family = AF_INET;
            to.sin_port = htons(hsfz::kIdentPort);
            ::inet_pton(AF_INET, target, &to.sin_addr);
            ::sendto(s, reinterpret_cast<const char*>(request.data()), static_cast<int>(request.size()), 0,
                     reinterpret_cast<sockaddr*>(&to), sizeof(to));
        }
        const std::uint64_t round = std::min(deadline, nowMs() + static_cast<std::uint64_t>(timeoutMs / 3 + 1));
        while (nowMs() < round) {
            if (!waitFor(s, static_cast<int>(round - nowMs()), false)) break;
            std::uint8_t buffer[512];
            sockaddr_in from{};
            socklen_t fromLength = sizeof(from);
            const auto got = ::recvfrom(s, reinterpret_cast<char*>(buffer), sizeof(buffer), 0,
                                        reinterpret_cast<sockaddr*>(&from), &fromLength);
            if (got <= 0) continue;
            hsfz::VehicleIdent ident;
            if (!hsfz::parseIdent(ByteVector(buffer, buffer + got), ident)) continue;
            char text[INET_ADDRSTRLEN] = {};
            ::inet_ntop(AF_INET, &from.sin_addr, text, sizeof(text));
            ident.address = text;
            const bool known = std::any_of(found.begin(), found.end(),
                                           [&](const hsfz::VehicleIdent& v) { return v.address == ident.address; });
            if (!known) found.push_back(ident);
        }
    }
    closeSocket(s);
    return found;
}

std::vector<doip::Entity> discoverDoip(int timeoutMs, std::string& error) {
    std::vector<doip::Entity> found;
    if (!startNetwork()) {
        error = L("Netzwerk lässt sich nicht starten.", "Cannot start networking.");
        return found;
    }
    const Socket s = ::socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (s == kNoSocket) return found;
    int one = 1;
    ::setsockopt(s, SOL_SOCKET, SO_BROADCAST, reinterpret_cast<const char*>(&one), sizeof(one));
    sockaddr_in local{};
    local.sin_family = AF_INET;
    ::bind(s, reinterpret_cast<sockaddr*>(&local), sizeof(local));
    // Version 0xFF is the "any version" the standard asks for here.
    const ByteVector request = doip::encode(doip::VehicleIdentRequest, {}, 0xFF);
    const std::uint64_t deadline = nowMs() + static_cast<std::uint64_t>(timeoutMs);
    for (int attempt = 0; attempt < 3 && found.empty(); ++attempt) {
        for (const char* target : {"255.255.255.255", "169.254.255.255"}) {
            sockaddr_in to{};
            to.sin_family = AF_INET;
            to.sin_port = htons(doip::kPort);
            ::inet_pton(AF_INET, target, &to.sin_addr);
            ::sendto(s, reinterpret_cast<const char*>(request.data()), static_cast<int>(request.size()), 0,
                     reinterpret_cast<sockaddr*>(&to), sizeof(to));
        }
        const std::uint64_t round = std::min(deadline, nowMs() + static_cast<std::uint64_t>(timeoutMs / 3 + 1));
        while (nowMs() < round) {
            if (!waitFor(s, static_cast<int>(round - nowMs()), false)) break;
            std::uint8_t buffer[512];
            sockaddr_in from{};
            socklen_t fromLength = sizeof(from);
            const auto got = ::recvfrom(s, reinterpret_cast<char*>(buffer), sizeof(buffer), 0,
                                        reinterpret_cast<sockaddr*>(&from), &fromLength);
            if (got <= 0) continue;
            doip::Parser parser;
            parser.feed(buffer, static_cast<std::size_t>(got));
            doip::Message message;
            if (!parser.next(message) || message.type != doip::VehicleAnnouncement) continue;
            doip::Entity entity;
            if (!doip::parseAnnouncement(message.payload, entity)) continue;
            char text[INET_ADDRSTRLEN] = {};
            ::inet_ntop(AF_INET, &from.sin_addr, text, sizeof(text));
            entity.address = text;
            const bool known = std::any_of(found.begin(), found.end(),
                                           [&](const doip::Entity& e) { return e.address == entity.address; });
            if (!known) found.push_back(entity);
        }
    }
    closeSocket(s);
    return found;
}

std::unique_ptr<ITransport> openDoip(const std::string& host, std::string& error, doip::Entity* found) {
    if (!startNetwork()) {
        error = L("Netzwerk lässt sich nicht starten.", "Cannot start networking.");
        return nullptr;
    }
    doip::Entity entity;
    std::string address = host;
    if (address.empty() || address == "auto") {
        const std::vector<doip::Entity> cars = discoverDoip(2500, error);
        if (cars.empty()) {
            if (error.empty())
                error = L("Kein Fahrzeug mit DoIP im Netzwerk gefunden.", "No vehicle with DoIP found on the network.");
            return nullptr;
        }
        entity = cars.front();
        address = entity.address;
    } else {
        entity.address = address;
    }
    const Socket s = connectTcp(address, doip::kPort, 5000);
    if (s == kNoSocket) {
        error = L("Keine DoIP-Verbindung zu ", "No DoIP connection to ") + address + ":" + std::to_string(doip::kPort);
        return nullptr;
    }
    auto session = std::make_unique<DoipSession>(std::make_unique<TcpStream>(s));
    if (!session->activate(error)) return nullptr;
    if (found) *found = entity;
    std::string name = "DoIP " + address;
    if (!entity.vin.empty()) name += " (" + entity.vin + ")";
    return std::make_unique<EnetElm>(std::move(session), name, EnetElm::Flavor::Doip);
}

std::unique_ptr<ITransport> openNetworkCar(const std::string& host, std::string& error, std::string& kind,
                                           std::string& vin) {
    std::string enetError, doipError;
    hsfz::VehicleIdent ident;
    if (auto t = openEnet(host, enetError, &ident)) {
        kind = "ENET";
        vin = ident.vin;
        return t;
    }
    doip::Entity entity;
    if (auto t = openDoip(host, doipError, &entity)) {
        kind = "DoIP";
        vin = entity.vin;
        return t;
    }
    error = enetError + "\n" + doipError;
    return nullptr;
}

std::unique_ptr<ITransport> openEnet(const std::string& host, std::string& error, hsfz::VehicleIdent* found) {
    if (!startNetwork()) {
        error = L("Netzwerk lässt sich nicht starten.", "Cannot start networking.");
        return nullptr;
    }
    hsfz::VehicleIdent ident;
    std::string address = host;
    if (address.empty() || address == "auto") {
        const std::vector<hsfz::VehicleIdent> cars = discoverEnet(3000, error);
        if (cars.empty()) {
            if (error.empty())
                error = L("Kein BMW über ENET gefunden. Steckt das ENET-Kabel im Auto und am Netzwerkanschluss, "
                          "und ist die Zündung an? Nach dem Einstecken braucht der Rechner bis zu einer Minute "
                          "für seine automatische Adresse (169.254.x.x), dann noch einmal versuchen.",
                          "No BMW found over ENET. Is the ENET cable plugged into the car and the network port, "
                          "and is the ignition on? After plugging in, the computer needs up to a minute for its "
                          "automatic address (169.254.x.x); then try again.");
            return nullptr;
        }
        ident = cars.front();
        address = ident.address;
    } else {
        ident.address = address;
    }

    const Socket s = connectTcp(address, hsfz::kDiagnosticPort, 5000);
    if (s == kNoSocket) {
        error = L("Keine Verbindung zu ", "No connection to ") + address + ":" +
                std::to_string(hsfz::kDiagnosticPort) +
                L(". Ist das die Adresse des Autos, und ist die Zündung an?",
                  ". Is this the car's address, and is the ignition on?");
        return nullptr;
    }
    if (found) *found = ident;
    std::string name = "ENET " + address;
    if (!ident.vin.empty()) name += " (" + ident.vin + ")";
    return std::make_unique<EnetElm>(std::make_unique<HsfzSession>(std::make_unique<TcpStream>(s)), name);
}

}  // namespace caruspex
