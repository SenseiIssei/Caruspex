#include "core/Elm327.h"

#include <algorithm>
#include <cctype>

#include "core/Lang.h"
#include "core/Timing.h"

namespace caruspex {

const char* const kProtocolOrderKLineFirst = "54312678";
const char* const kProtocolOrderCanFirst = "67891543";

namespace {

const char* kErrors[] = {"UNABLE TO CONNECT", "BUS INIT: ERROR", "BUS INIT: ...ERROR",
                         "BUS ERROR", "CAN ERROR", "DATA ERROR", "BUS BUSY", "FB ERROR",
                         "LV RESET", "STOPPED", "ERR", "?"};

// Two upper-case hex digits, for negative response codes.
std::string hexByte(int value) {
    static const char digits[] = "0123456789ABCDEF";
    return {digits[(value >> 4) & 0xF], digits[value & 0xF]};
}

std::string trim(const std::string& s) {
    const size_t a = s.find_first_not_of(" \t\r\n");
    if (a == std::string::npos) return {};
    const size_t b = s.find_last_not_of(" \t\r\n");
    return s.substr(a, b - a + 1);
}

std::string toUpper(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(),
                   [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
    return s;
}

std::string stripSpaces(const std::string& s) {
    std::string out;
    for (char c : s) {
        if (!std::isspace(static_cast<unsigned char>(c))) out += c;
    }
    return out;
}

bool startsWith(const std::string& s, const std::string& prefix) {
    return s.rfind(prefix, 0) == 0;
}

bool isHexLine(const std::string& s) {
    if (s.empty()) return false;
    for (char c : s) {
        if (!std::isxdigit(static_cast<unsigned char>(c)) && !std::isspace(static_cast<unsigned char>(c)))
            return false;
    }
    return true;
}

}  // namespace

std::string protocolName(char id) {
    switch (id) {
        case '1': return "SAE J1850 PWM";
        case '2': return "SAE J1850 VPW";
        case '3': return L("ISO 9141-2 (K-Line, 5-Baud-Init)", "ISO 9141-2 (K-Line, 5-baud init)");
        case '4': return L("ISO 14230-4 KWP (5-Baud-Init)", "ISO 14230-4 KWP (5-baud init)");
        case '5': return L("ISO 14230-4 KWP (Fast-Init)", "ISO 14230-4 KWP (fast init)");
        case '6': return L("ISO 15765-4 CAN (11 Bit, 500 kBaud)", "ISO 15765-4 CAN (11 bit, 500 kbaud)");
        case '7': return L("ISO 15765-4 CAN (29 Bit, 500 kBaud)", "ISO 15765-4 CAN (29 bit, 500 kbaud)");
        case '8': return L("ISO 15765-4 CAN (11 Bit, 250 kBaud)", "ISO 15765-4 CAN (11 bit, 250 kbaud)");
        case '9': return L("ISO 15765-4 CAN (29 Bit, 250 kBaud)", "ISO 15765-4 CAN (29 bit, 250 kbaud)");
        default: return L("unbekannt", "unknown");
    }
}

std::vector<int> hexToBytes(const std::string& text) {
    std::string clean;
    for (char c : text) {
        if (std::isxdigit(static_cast<unsigned char>(c))) clean += c;
    }
    if (clean.size() % 2 == 1) clean.pop_back();
    std::vector<int> out;
    out.reserve(clean.size() / 2);
    for (size_t i = 0; i + 1 < clean.size(); i += 2) {
        out.push_back(std::stoi(clean.substr(i, 2), nullptr, 16));
    }
    return out;
}

ElmResult decodeMessages(const std::vector<std::string>& lines, int expectedService) {
    ElmResult result;
    result.lines = lines;

    for (const std::string& line : lines) {
        const std::string up = toUpper(line);
        if (startsWith(up, "NO DATA")) {
            result.status = ElmStatus::NoData;
            result.message = L("Steuergeraet unterstuetzt diese Anfrage nicht (NO DATA).",
                               "Control unit does not support this request (NO DATA).");
            return result;
        }
        for (const char* err : kErrors) {
            if (up == err || startsWith(up, std::string(err) + " ") ||
                startsWith(up, std::string(err) + ":")) {
                result.status = ElmStatus::BusError;
                result.message = L("Adapter meldet: ", "Adapter reports: ") + line;
                return result;
            }
        }
    }

    // Multi-line CAN responses arrive numbered ("0:", "1:" ...), preceded by a
    // separate line holding the total length. The numbered lines start
    // directly with the service byte, so nothing may be cut off.
    std::string indexed;
    std::vector<std::vector<int>> raw;
    for (const std::string& line : lines) {
        if (line.size() > 2 && line[1] == ':' &&
            std::isxdigit(static_cast<unsigned char>(line[0]))) {
            indexed += line.substr(2);
        } else if (isHexLine(line)) {
            std::vector<int> bytes = hexToBytes(line);
            if (!bytes.empty()) raw.push_back(std::move(bytes));
        }
    }
    if (!indexed.empty()) {
        // The length line is then just noise and would otherwise pass as a
        // message of its own.
        raw.clear();
        std::vector<int> bytes = hexToBytes(indexed);
        if (!bytes.empty()) raw.push_back(std::move(bytes));
    }

    for (std::vector<int>& msg : raw) {
        if (msg.empty()) continue;
        if (msg[0] == 0x7F && msg.size() >= 3) {
            result.status = ElmStatus::Negative;
            result.message = std::string(L("Steuergeraet lehnt den Dienst ab (Code 0x",
                                           "Control unit rejects the service (code 0x")) +
                             hexByte(msg[2]) + ")";
            return result;
        }
        if (msg[0] == expectedService + 0x40 || msg[0] == expectedService) {
            result.messages.emplace_back(msg.begin() + 1, msg.end());
        }
    }
    if (result.messages.empty()) {
        result.status = ElmStatus::NoData;
        result.message = L("Keine verwertbare Antwort erhalten.", "No usable response received.");
    }
    return result;
}

void Elm327::drainGreeting() {
    // Wi-Fi and Bluetooth adapters announce themselves unprompted. If this
    // greeting is left in the buffer, it answers the first command and all
    // following responses are shifted by one position.
    sleepMs(300);
    transport_->flushInput();
    transport_->write("\r");
    sleepMs(200);
    transport_->flushInput();
}

ElmResult Elm327::send(const std::string& command, int timeoutMs) {
    ElmResult result;
    if (!transport_) {
        result.status = ElmStatus::AdapterError;
        result.message = L("Kein Adapter verbunden.", "No adapter connected.");
        return result;
    }
    transport_->flushInput();
    transport_->write(command + "\r");

    // The configured factor applies here, at the only place where we
    // actually wait.
    const int waitMs = static_cast<int>(static_cast<double>(timeoutMs) * timeoutScale_);

    std::string buffer;
    const std::uint64_t deadline = nowMs() + static_cast<std::uint64_t>(waitMs);
    while (nowMs() < deadline) {
        const std::string chunk = transport_->read(120);
        if (!chunk.empty()) {
            buffer += chunk;
            if (buffer.find('>') != std::string::npos) break;
        }
    }
    if (buffer.empty()) {
        result.status = ElmStatus::AdapterError;
        result.message = L("Keine Antwort des Adapters auf '", "No response from the adapter to '") +
                         command +
                         L("'. Falsche Baudrate, oder Kabel/Bluetooth weg?",
                           "'. Wrong baud rate, or cable/Bluetooth disconnected?");
        return result;
    }

    std::string cleaned;
    for (char c : buffer) {
        if (c != '>') cleaned += c;
    }
    const std::string echo = toUpper(stripSpaces(command));
    std::string current;
    for (char c : cleaned + "\n") {
        if (c == '\r' || c == '\n') {
            const std::string line = trim(current);
            current.clear();
            if (line.empty()) continue;
            if (toUpper(stripSpaces(line)) == echo) continue;
            if (startsWith(toUpper(line), "SEARCHING")) continue;
            result.lines.push_back(line);
        } else {
            current += c;
        }
    }
    if (log_ && verbose_) {
        std::string joined;
        for (const std::string& l : result.lines) joined += l + " | ";
        log_("  > " + command + "   ->   " + joined);
    }
    return result;
}

ElmResult Elm327::at(const std::string& command, int timeoutMs) {
    return send("AT" + command, timeoutMs);
}

ElmResult Elm327::request(const std::string& command, int expectedService, int timeoutMs) {
    ElmResult raw = send(command, timeoutMs);
    if (raw.status == ElmStatus::AdapterError) return raw;
    return decodeMessages(raw.lines, expectedService);
}

bool Elm327::connect(std::unique_ptr<ITransport> transport, const std::string& protocolOrder,
                     std::string& error) {
    transport_ = std::move(transport);
    connected_ = false;
    drainGreeting();

    ElmResult banner = at("Z", 6000);
    if (banner.status == ElmStatus::AdapterError) {
        error = banner.message;
        return false;
    }
    version_.clear();
    for (const std::string& line : banner.lines) version_ += line + " ";
    version_ = trim(version_);
    if (version_.empty()) version_ = "?";
    if (toUpper(version_).find("ELM") == std::string::npos &&
        toUpper(version_).find("OBD") == std::string::npos) {
        if (log_)
            log_(L("  Hinweis: unerwartete Adapterkennung '", "  Note: unexpected adapter ID '") +
                 version_ + "'");
    }

    for (const char* cmd : {"E0", "L0", "S0", "H0", "AT1", "ST64"}) at(cmd);

    for (char proto : protocolOrder) {
        at(std::string("SP") + proto);
        if (log_) log_(L("  Versuche Protokoll ", "  Trying protocol ") + std::string(1, proto) + " (" +
                       protocolName(proto) + ") ...");
        const int timeout = (proto >= '3' && proto <= '5') ? 12000 : 8000;
        for (int attempt = 0; attempt < 2; ++attempt) {
            ElmResult probe = request("0100", 0x01, timeout);
            if (probe.ok()) {
                ElmResult dp = at("DP");
                protocolText_ = dp.lines.empty() ? protocolName(proto) : dp.lines.front();
                connected_ = true;
                return true;
            }
            if (probe.status == ElmStatus::AdapterError) {
                error = probe.message;
                return false;
            }
            sleepMs(400);
        }
    }

    error = L(
        "Kein Kontakt zum Steuergeraet.\n"
        "  - Zuendung an? (Motor darf aus sein)\n"
        "  - Stecker fest im OBD-Port?\n"
        "  - Manche Adapter brauchen einen zweiten Anlauf: abziehen, 10 s warten, neu stecken.",
        "No contact with the control unit.\n"
        "  - Ignition on? (The engine may be off)\n"
        "  - Plug firmly seated in the OBD port?\n"
        "  - Some adapters need a second attempt: unplug, wait 10 s, plug back in.");
    return false;
}

void Elm327::disconnect() {
    if (transport_ && connected_) at("PC", 2000);
    transport_.reset();
    connected_ = false;
    protocolText_ = L("unbekannt", "unknown");
}

}  // namespace caruspex
