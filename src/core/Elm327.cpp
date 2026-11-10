#include "core/Elm327.h"

#include <algorithm>
#include <cctype>
#include <cstddef>
#include <cstdio>

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

// True if the adapter confirmed a setting with a line that says "OK".
bool answeredOk(const ElmResult& result) {
    if (result.status == ElmStatus::AdapterError) return false;
    for (const std::string& line : result.lines) {
        if (toUpper(line) == "OK") return true;
    }
    return false;
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

HeaderFormat headerFormatFor(char protocolId) {
    switch (protocolId) {
        case '1':
        case '2': return HeaderFormat::J1850;
        case '3':
        case '4':
        case '5': return HeaderFormat::KLine;
        case '6':
        case '8': return HeaderFormat::Can11;
        case '7':
        case '9': return HeaderFormat::Can29;
        default: return HeaderFormat::None;
    }
}

namespace {

bool isIndexedLine(const std::string& line) {
    return line.size() > 2 && line[1] == ':' && std::isxdigit(static_cast<unsigned char>(line[0]));
}

// Headers off. Single-frame responses are one line each, so several control
// units simply give several lines. Multi-frame CAN responses arrive numbered
// ("0:", "1:" ...), preceded by a line with the total length in three hex
// digits. The numbered lines start directly with the service byte, so
// nothing may be cut off. A new length line, or a "0:" once the announced
// length is complete, starts the next message: that is how the responses of
// two control units are kept apart. The index wraps from F to 0 on long
// messages, which is why "0:" alone is not enough.
std::vector<ElmMessage> splitWithoutHeaders(const std::vector<std::string>& lines) {
    std::vector<ElmMessage> out;
    std::vector<int> collected;
    int announced = -1;
    bool collecting = false;

    auto flush = [&]() {
        if (collecting && !collected.empty()) {
            if (announced > 0 && static_cast<int>(collected.size()) > announced)
                collected.resize(static_cast<size_t>(announced));
            out.push_back({std::string(), collected});
        }
        collected.clear();
        collecting = false;
        announced = -1;
    };

    for (const std::string& line : lines) {
        if (isIndexedLine(line)) {
            const bool first = line[0] == '0';
            const bool complete = announced > 0 && static_cast<int>(collected.size()) >= announced;
            if (first && collecting && !collected.empty() && (announced < 0 || complete)) flush();
            collecting = true;
            const std::vector<int> bytes = hexToBytes(line.substr(2));
            collected.insert(collected.end(), bytes.begin(), bytes.end());
        } else if (isHexLine(line)) {
            const std::string digits = stripSpaces(line);
            // Exactly three digits: a real message always has whole bytes.
            if (digits.size() == 3) {
                // Length line of the next multi-frame message.
                flush();
                announced = std::stoi(digits, nullptr, 16);
                collecting = true;
                continue;
            }
            flush();
            std::vector<int> bytes = hexToBytes(line);
            if (!bytes.empty()) out.push_back({std::string(), std::move(bytes)});
        }
    }
    flush();
    return out;
}

// Headers on. Every line is one frame with the sender in front.
std::vector<ElmMessage> splitWithHeaders(const std::vector<std::string>& lines,
                                         HeaderFormat format) {
    std::vector<ElmMessage> out;

    // ISO-TP reassembly per sender for CAN.
    struct Pending {
        std::string source;
        int total = 0;
        std::vector<int> data;
    };
    std::vector<Pending> pending;
    auto findPending = [&pending](const std::string& source) -> Pending* {
        for (Pending& p : pending) {
            if (p.source == source) return &p;
        }
        return nullptr;
    };
    auto finish = [&](const std::string& source) {
        for (size_t i = 0; i < pending.size(); ++i) {
            if (pending[i].source != source) continue;
            Pending done = std::move(pending[i]);
            pending.erase(pending.begin() + static_cast<std::ptrdiff_t>(i));
            if (static_cast<int>(done.data.size()) > done.total)
                done.data.resize(static_cast<size_t>(done.total));
            out.push_back({done.source, std::move(done.data)});
            return;
        }
    };

    for (const std::string& line : lines) {
        if (!isHexLine(line)) continue;
        const std::string digits = stripSpaces(toUpper(line));

        if (format == HeaderFormat::Can11 || format == HeaderFormat::Can29) {
            const size_t idLength = format == HeaderFormat::Can11 ? 3 : 8;
            if (digits.size() < idLength + 2) continue;
            const std::string source = digits.substr(0, idLength);
            const std::vector<int> frame = hexToBytes(digits.substr(idLength));
            if (frame.empty()) continue;
            const int type = frame[0] >> 4;
            if (type == 0) {
                // Single frame: the low nibble is the length. Anything after
                // it is padding.
                int length = frame[0] & 0x0F;
                size_t start = 1;
                if (length == 0 && frame.size() > 1) {  // CAN FD escape
                    length = frame[1];
                    start = 2;
                }
                const size_t end = std::min(frame.size(), start + static_cast<size_t>(length));
                out.push_back({source, std::vector<int>(frame.begin() + static_cast<std::ptrdiff_t>(start),
                                                        frame.begin() + static_cast<std::ptrdiff_t>(end))});
            } else if (type == 1 && frame.size() >= 2) {
                // First frame: twelve bits of total length, then data.
                finish(source);  // an unfinished older message from the same sender
                Pending p;
                p.source = source;
                p.total = ((frame[0] & 0x0F) << 8) | frame[1];
                p.data.assign(frame.begin() + 2, frame.end());
                pending.push_back(std::move(p));
                if (static_cast<int>(pending.back().data.size()) >= pending.back().total) finish(source);
            } else if (type == 2) {
                // Consecutive frame.
                Pending* p = findPending(source);
                if (!p) continue;
                p->data.insert(p->data.end(), frame.begin() + 1, frame.end());
                if (static_cast<int>(p->data.size()) >= p->total) finish(source);
            }
            // Type 3 is flow control, sent by the adapter itself.
            continue;
        }

        // K-line and J1850: header bytes, data, then one checksum byte.
        const std::vector<int> bytes = hexToBytes(digits);
        size_t header = 3;
        if (format == HeaderFormat::KLine && !bytes.empty() && (bytes[0] & 0xC0) != 0 &&
            (bytes[0] & 0x3F) == 0) {
            header = 4;  // ISO 14230 with a separate length byte
        }
        if (bytes.size() < header + 2) continue;
        char source[8];
        std::snprintf(source, sizeof(source), "%02X", bytes[2] & 0xFF);
        out.push_back({source, std::vector<int>(bytes.begin() + static_cast<std::ptrdiff_t>(header),
                                                bytes.end() - 1)});
    }

    // Whatever is still incomplete is handed out as far as it got.
    while (!pending.empty()) finish(pending.front().source);
    return out;
}

}  // namespace

std::vector<ElmMessage> splitMessages(const std::vector<std::string>& lines, HeaderFormat headers) {
    return headers == HeaderFormat::None ? splitWithoutHeaders(lines) : splitWithHeaders(lines, headers);
}

ElmResult decodeMessages(const std::vector<std::string>& lines, int expectedService,
                         HeaderFormat headers) {
    ElmResult result;
    result.lines = lines;

    for (const std::string& line : lines) {
        const std::string up = toUpper(line);
        if (startsWith(up, "NO DATA")) {
            result.status = ElmStatus::NoData;
            result.message = L("Steuergerät unterstützt diese Anfrage nicht (NO DATA).",
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

    const std::vector<ElmMessage> raw = splitMessages(lines, headers);

    // With several control units on the bus one of them may refuse while
    // another answers. Only if nobody answers is the refusal the result.
    int refusal = -1;
    for (const ElmMessage& msg : raw) {
        const std::vector<int>& bytes = msg.bytes;
        if (bytes.empty()) continue;
        if (bytes[0] == 0x7F && bytes.size() >= 3) {
            if (refusal < 0) refusal = bytes[2];
            continue;
        }
        if (bytes[0] == expectedService + 0x40 || bytes[0] == expectedService) {
            result.messages.emplace_back(bytes.begin() + 1, bytes.end());
            result.sources.push_back(msg.source);
        }
    }
    if (result.messages.empty() && refusal >= 0) {
        result.status = ElmStatus::Negative;
        result.message = std::string(L("Steuergerät lehnt den Dienst ab (Code 0x",
                                       "Control unit rejects the service (code 0x")) +
                         hexByte(refusal) + ")";
        return result;
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
    // Keep track of the header setting even when someone sends the AT
    // commands by hand (the raw console, the KKL reset). A reset switches
    // headers off again.
    const std::string sent = toUpper(stripSpaces(command));
    if (sent == "ATH1") headers_ = true;
    if (sent == "ATH0" || sent == "ATZ" || sent == "ATD" || sent == "ATWS") headers_ = false;

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
    if (obdOnUds_) return requestOnUds(command, expectedService, timeoutMs);
    ElmResult raw = send(command, timeoutMs);
    if (raw.status == ElmStatus::AdapterError) return raw;
    return decodeMessages(raw.lines, expectedService, headerFormat());
}

// ---------------------------------------------------------------------------
// OBD on UDS (SAE J1979-2)
//
//   01 pp      ->  22 F4 pp       PIDs of service 01 as data identifiers
//   09 ii      ->  22 F8 ii       vehicle information
//   03         ->  19 42 33 08 FF confirmed emission DTCs (WWH-OBD format)
//   07         ->  19 42 33 04 FF pending
//   0A         ->  19 55 33       permanent
//   04         ->  14 FF FF 33    clear the emission group
// ---------------------------------------------------------------------------

ElmResult Elm327::udsRequest(const std::string& command, int timeoutMs) {
    const int sid = std::stoi(command.substr(0, 2), nullptr, 16);
    ElmResult result;
    // A unit that is still busy says 7F xx 78; ask again, as UDS allows.
    for (int round = 0; round < 10; ++round) {
        ElmResult raw = send(command, timeoutMs);
        if (raw.status == ElmStatus::AdapterError) return raw;
        bool pending = false;
        for (const std::string& line : raw.lines) {
            const std::string compact = toUpper(stripSpaces(line));
            char needle[8];
            std::snprintf(needle, sizeof(needle), "7F%02X78", sid);
            if (compact.find(needle) != std::string::npos) pending = true;
        }
        result = decodeMessages(raw.lines, sid, headerFormat());
        if (result.ok() || !pending) return result;
        sleepMs(50);
    }
    return result;
}

ElmResult Elm327::requestOnUds(const std::string& command, int expectedService, int timeoutMs) {
    std::string hex = toUpper(stripSpaces(command));
    if (hex.size() % 2 == 1) hex.pop_back();   // "expected answers" digit
    std::vector<int> bytes;
    for (size_t i = 0; i + 1 < hex.size(); i += 2) bytes.push_back(std::stoi(hex.substr(i, 2), nullptr, 16));
    ElmResult none;
    none.status = ElmStatus::NoData;
    none.message = L("Im Modus OBD über UDS nicht verfügbar.", "Not available in OBD on UDS mode.");
    if (bytes.empty()) return none;
    (void)expectedService;

    // Collects the translated payloads per sending unit, in order.
    std::vector<std::string> order;
    std::vector<std::vector<int>> payloads;
    auto add = [&](const std::string& source, const std::vector<int>& data) {
        auto it = std::find(order.begin(), order.end(), source);
        if (it == order.end()) {
            order.push_back(source);
            payloads.push_back(data);
        } else {
            auto& p = payloads[static_cast<size_t>(it - order.begin())];
            p.insert(p.end(), data.begin(), data.end());
        }
    };
    auto finish = [&](const ElmResult& last) {
        if (payloads.empty()) return last.ok() ? none : last;
        ElmResult out;
        out.messages = payloads;
        out.sources = order;
        return out;
    };
    char cmd[32];
    ElmResult last;

    switch (bytes[0]) {
        case 0x01:
        case 0x09: {
            // One identifier per request; several PIDs are joined back into
            // one multi-PID answer per unit.
            const int high = bytes[0] == 0x01 ? 0xF4 : 0xF8;
            for (size_t i = 1; i < bytes.size(); ++i) {
                std::snprintf(cmd, sizeof(cmd), "22%02X%02X", high, bytes[i]);
                last = udsRequest(cmd, timeoutMs);
                if (!last.ok()) continue;
                for (size_t m = 0; m < last.messages.size(); ++m) {
                    const std::vector<int>& d = last.messages[m];
                    if (d.size() < 2 || d[0] != high || d[1] != bytes[i]) continue;
                    std::vector<int> classic(d.begin() + 1, d.end());
                    // Service 09 had a count byte in front of these items.
                    if (bytes[0] == 0x09 && (bytes[i] == 0x02 || bytes[i] == 0x0A)) classic.insert(classic.begin() + 1, 1);
                    add(m < last.sources.size() ? last.sources[m] : std::string(), classic);
                }
            }
            return finish(last);
        }
        case 0x03:
        case 0x07:
        case 0x0A: {
            const bool permanent = bytes[0] == 0x0A;
            last = udsRequest(permanent ? "195533" : (bytes[0] == 0x03 ? "19423308FF" : "19423304FF"), timeoutMs);
            if (!last.ok()) return last;
            for (size_t m = 0; m < last.messages.size(); ++m) {
                const std::vector<int>& d = last.messages[m];
                // 42: sub, group, status mask, severity mask, format, then
                // severity + DTC (3) + status. 55: sub, group, status mask,
                // format, then DTC (3) + status.
                const size_t start = permanent ? 4 : 5;
                const size_t step = permanent ? 4 : 5;
                const size_t dtcAt = permanent ? 0 : 1;
                if (d.size() < start) continue;
                std::vector<int> classic = {0};   // count first, as on CAN
                for (size_t i = start; i + step <= d.size(); i += step) {
                    classic.push_back(d[i + dtcAt]);
                    classic.push_back(d[i + dtcAt + 1]);
                }
                classic[0] = static_cast<int>((classic.size() - 1) / 2);
                add(m < last.sources.size() ? last.sources[m] : std::string(), classic);
            }
            return finish(last);
        }
        case 0x04: {
            last = udsRequest("14FFFF33", timeoutMs);
            if (!last.ok()) return last;
            ElmResult out;
            out.messages.assign(last.messages.size(), {});
            out.sources = last.sources;
            return out;
        }
        default:
            return none;   // 02, 05, 06, 08 have no simple counterpart
    }
}

bool Elm327::setHeaders(bool on) {
    ElmResult r = at(on ? "H1" : "H0", 3000);
    if (r.status == ElmStatus::AdapterError) return false;
    for (const std::string& line : r.lines) {
        if (toUpper(line).find("OK") != std::string::npos) {
            headers_ = on;
            return true;
        }
    }
    if (on) headers_ = false;
    return false;
}

void Elm327::wrapTransport(
    const std::function<std::unique_ptr<ITransport>(std::unique_ptr<ITransport>)>& wrap) {
    if (!transport_ || !wrap) return;
    std::unique_ptr<ITransport> next = wrap(std::move(transport_));
    transport_ = std::move(next);
    if (!transport_) connected_ = false;
}

void Elm327::detectStn() {
    // STI only reads the firmware identification. An ELM327 or a clone
    // answers "?" (an ST command is not an AT command, so even clones that
    // say "OK" to every AT command give themselves away here).
    stn_ = false;
    stnId_.clear();
    const ElmResult sti = send("STI", 1500);
    if (sti.status == ElmStatus::AdapterError || sti.lines.empty()) return;
    const std::string first = sti.lines.front();
    if (startsWith(toUpper(first), "STN")) {
        stn_ = true;
        stnId_ = first;
        if (log_) log_(L("  STN-Chip erkannt: ", "  STN chip detected: ") + first);
    }
}

void Elm327::applyAggressiveTiming() {
    const bool wanted = adaptiveTiming_ == AdaptiveTiming::Aggressive ||
                        (adaptiveTiming_ == AdaptiveTiming::Auto && stn_);
    if (!wanted) return;
    if (timingLevel_ == 0) {
        if (log_)
            log_(L("  Der Adapter kennt keine adaptive Zeitsteuerung (ATAT), ATAT2 entfällt.",
                   "  The adapter has no adaptive timing (ATAT), so ATAT2 is skipped."));
        return;
    }
    // Only now, with a protocol found: the search itself runs with the
    // careful ATAT1, so a slow control unit is not missed while looking.
    if (!answeredOk(at("AT2", 2000))) {
        at("AT1", 2000);
        timingLevel_ = 1;
        if (log_)
            log_(L("  ATAT2 abgelehnt, es bleibt bei ATAT1.",
                   "  ATAT2 refused, staying with ATAT1."));
        return;
    }
    // One request as a check that answers still come through with the
    // shorter wait. If not, back to the careful setting.
    const ElmResult check = request("0100", 0x01);
    if (!check.ok()) {
        at("AT1", 2000);
        timingLevel_ = 1;
        if (log_)
            log_(L("  Mit ATAT2 fehlen Antworten, zurück auf ATAT1.",
                   "  Answers go missing with ATAT2, back to ATAT1."));
        return;
    }
    timingLevel_ = 2;
    if (log_)
        log_(L("  Schnelle adaptive Zeitsteuerung (ATAT2) aktiv.",
               "  Aggressive adaptive timing (ATAT2) active."));
}

bool Elm327::connect(std::unique_ptr<ITransport> transport, const std::string& protocolOrder,
                     std::string& error) {
    transport_ = std::move(transport);
    connected_ = false;
    stn_ = false;
    stnId_.clear();
    timingLevel_ = 0;
    obdOnUds_ = false;
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

    for (const char* cmd : {"E0", "L0", "S0", "H0"}) at(cmd);
    // ATAT1 is the power-on default; set it anyway, since the adapter may
    // come out of an earlier session with different timing. Adapters older
    // than v1.2 know no adaptive timing at all.
    timingLevel_ = answeredOk(at("AT1")) ? 1 : 0;
    at("ST64");
    detectStn();

    for (char proto : protocolOrder) {
        at(std::string("SP") + proto);
        if (log_) log_(L("  Versuche Protokoll ", "  Trying protocol ") + std::string(1, proto) + " (" +
                       protocolName(proto) + ") ...");
        const int timeout = (proto >= '3' && proto <= '5') ? 12000 : 8000;
        for (int attempt = 0; attempt < 2; ++attempt) {
            ElmResult probe = request("0100", 0x01, timeout);
            if (!probe.ok() && probe.status != ElmStatus::AdapterError && proto >= '6' && proto <= '9') {
                // Newer cars may know the emission services only over UDS.
                ElmResult uds = udsRequest("22F400", timeout);
                if (uds.ok() && !uds.messages.empty() && uds.messages.front().size() >= 2 &&
                    uds.messages.front()[0] == 0xF4 && uds.messages.front()[1] == 0x00) {
                    obdOnUds_ = true;
                    probe = uds;
                    if (log_)
                        log_(L("  Das Auto spricht OBD über UDS (SAE J1979-2).",
                               "  The car speaks OBD on UDS (SAE J1979-2)."));
                }
            }
            if (probe.ok()) {
                ElmResult dp = at("DP");
                protocolText_ = dp.lines.empty() ? protocolName(proto) : dp.lines.front();
                protocolId_ = proto;
                connected_ = true;
                applyAggressiveTiming();
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
        "Kein Kontakt zum Steuergerät.\n"
        "  - Zündung an? (Motor darf aus sein)\n"
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
    headers_ = false;
    stn_ = false;
    obdOnUds_ = false;
    stnId_.clear();
    timingLevel_ = 0;
    protocolId_ = '0';
    protocolText_ = L("unbekannt", "unknown");
}

}  // namespace caruspex
