#include "core/Recording.h"

#include <algorithm>
#include <cctype>
#include <chrono>
#include <cstdio>
#include <filesystem>
#include <map>
#include <sstream>
#include <utility>

#include "core/Elm327.h"
#include "core/Lang.h"
#include "core/Timing.h"

namespace caruspex {
namespace {

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

// Paths come in as UTF-8 (from Qt or the command line). On Windows a plain
// std::string path would be read in the ANSI code page, so go through
// char8_t to keep umlauts in folder names intact.
std::filesystem::path utf8Path(const std::string& path) {
    return std::filesystem::path(std::u8string(path.begin(), path.end()));
}

std::string fileNameOf(const std::string& path) {
    const size_t slash = path.find_last_of("/\\");
    return slash == std::string::npos ? path : path.substr(slash + 1);
}

std::string upperAscii(std::string s) {
    for (char& c : s) c = static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    return s;
}

// Seconds with three decimals, written without printf so the decimal point
// does not depend on the C locale (Qt sets it from the system on Linux).
std::string formatSeconds(std::uint64_t ms) {
    char buffer[32];
    std::snprintf(buffer, sizeof(buffer), "%llu.%03u", static_cast<unsigned long long>(ms / 1000),
                  static_cast<unsigned>(ms % 1000));
    return buffer;
}

// Current time in UTC as "2026-11-09T18:02:11Z". The calendar conversion is
// done by hand (days to civil date), which keeps it free of gmtime and its
// thread-safety differences between platforms.
std::string utcNow() {
    using namespace std::chrono;
    const long long secs =
        duration_cast<seconds>(system_clock::now().time_since_epoch()).count();
    long long days = secs / 86400;
    long long rest = secs % 86400;
    if (rest < 0) {
        rest += 86400;
        --days;
    }
    days += 719468;
    const long long era = (days >= 0 ? days : days - 146096) / 146097;
    const long long doe = days - era * 146097;
    const long long yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    long long year = yoe + era * 400;
    const long long doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    const long long mp = (5 * doy + 2) / 153;
    const long long day = doy - (153 * mp + 2) / 5 + 1;
    const long long month = mp < 10 ? mp + 3 : mp - 9;
    if (month <= 2) ++year;
    char buffer[32];
    std::snprintf(buffer, sizeof(buffer), "%04lld-%02lld-%02lldT%02lld:%02lld:%02lldZ", year, month,
                  day, rest / 3600, (rest / 60) % 60, rest % 60);
    return buffer;
}

// ---------------------------------------------------------------------------
// JSON, just as much as this format needs
// ---------------------------------------------------------------------------

// Escapes a string for JSON. With `bytes` every byte outside printable ASCII
// becomes \u00XX and means exactly that byte; this is how the raw adapter
// traffic is kept, which need not be valid UTF-8. Without `bytes` (header
// texts) UTF-8 passes through and only control characters are escaped.
std::string jsonString(const std::string& s, bool bytes) {
    static const char digits[] = "0123456789abcdef";
    std::string out = "\"";
    for (char ch : s) {
        const unsigned char c = static_cast<unsigned char>(ch);
        switch (c) {
            case '"': out += "\\\""; continue;
            case '\\': out += "\\\\"; continue;
            case '\r': out += "\\r"; continue;
            case '\n': out += "\\n"; continue;
            case '\t': out += "\\t"; continue;
            default: break;
        }
        if (c < 0x20 || c == 0x7F || (bytes && c >= 0x80)) {
            out += "\\u00";
            out += digits[c >> 4];
            out += digits[c & 0xF];
        } else {
            out += ch;
        }
    }
    out += '"';
    return out;
}

struct JsonValue {
    bool isString = false;
    bool isNumber = false;
    std::string text;
    double number = 0.0;
};

class FlatJsonParser {
public:
    explicit FlatJsonParser(const std::string& s) : s_(s) {}

    // Parses one object whose values are strings, numbers, booleans or null.
    // Nested objects and arrays are not part of the format and fail.
    bool parse(std::map<std::string, JsonValue>& out) {
        skipSpace();
        if (!take('{')) return false;
        skipSpace();
        if (take('}')) return atEnd();
        for (;;) {
            skipSpace();
            std::string key;
            if (!parseString(key)) return false;
            skipSpace();
            if (!take(':')) return false;
            skipSpace();
            JsonValue value;
            if (peek() == '"') {
                value.isString = true;
                if (!parseString(value.text)) return false;
            } else if (peek() == '-' || std::isdigit(static_cast<unsigned char>(peek()))) {
                value.isNumber = true;
                if (!parseNumber(value.number)) return false;
            } else if (!parseLiteral()) {
                return false;
            }
            out[key] = std::move(value);
            skipSpace();
            if (take(',')) continue;
            if (take('}')) return atEnd();
            return false;
        }
    }

private:
    char peek() const { return i_ < s_.size() ? s_[i_] : '\0'; }
    bool take(char c) {
        if (peek() != c || i_ >= s_.size()) return false;
        ++i_;
        return true;
    }
    void skipSpace() {
        while (i_ < s_.size() && (s_[i_] == ' ' || s_[i_] == '\t' || s_[i_] == '\r' || s_[i_] == '\n'))
            ++i_;
    }
    bool atEnd() {
        skipSpace();
        return i_ == s_.size();
    }

    static int hexValue(char c) {
        if (c >= '0' && c <= '9') return c - '0';
        if (c >= 'a' && c <= 'f') return c - 'a' + 10;
        if (c >= 'A' && c <= 'F') return c - 'A' + 10;
        return -1;
    }

    bool parseHex4(unsigned& value) {
        if (i_ + 4 > s_.size()) return false;
        value = 0;
        for (int k = 0; k < 4; ++k) {
            const int v = hexValue(s_[i_ + static_cast<size_t>(k)]);
            if (v < 0) return false;
            value = value * 16 + static_cast<unsigned>(v);
        }
        i_ += 4;
        return true;
    }

    static void appendUtf8(std::string& out, unsigned cp) {
        if (cp < 0x80) {
            out += static_cast<char>(cp);
        } else if (cp < 0x800) {
            out += static_cast<char>(0xC0 | (cp >> 6));
            out += static_cast<char>(0x80 | (cp & 0x3F));
        } else if (cp < 0x10000) {
            out += static_cast<char>(0xE0 | (cp >> 12));
            out += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
            out += static_cast<char>(0x80 | (cp & 0x3F));
        } else {
            out += static_cast<char>(0xF0 | (cp >> 18));
            out += static_cast<char>(0x80 | ((cp >> 12) & 0x3F));
            out += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
            out += static_cast<char>(0x80 | (cp & 0x3F));
        }
    }

    bool parseString(std::string& out) {
        if (!take('"')) return false;
        while (i_ < s_.size()) {
            const char c = s_[i_++];
            if (c == '"') return true;
            if (static_cast<unsigned char>(c) < 0x20) return false;
            if (c != '\\') {
                out += c;
                continue;
            }
            if (i_ >= s_.size()) return false;
            const char e = s_[i_++];
            switch (e) {
                case '"': out += '"'; break;
                case '\\': out += '\\'; break;
                case '/': out += '/'; break;
                case 'b': out += '\b'; break;
                case 'f': out += '\f'; break;
                case 'n': out += '\n'; break;
                case 'r': out += '\r'; break;
                case 't': out += '\t'; break;
                case 'u': {
                    unsigned cp = 0;
                    if (!parseHex4(cp)) return false;
                    if (cp < 0x100) {
                        // One raw byte, see jsonString.
                        out += static_cast<char>(cp);
                    } else if (cp >= 0xD800 && cp < 0xDC00) {
                        unsigned low = 0;
                        if (!take('\\') || !take('u') || !parseHex4(low) || low < 0xDC00 || low > 0xDFFF)
                            return false;
                        appendUtf8(out, 0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00));
                    } else {
                        appendUtf8(out, cp);
                    }
                    break;
                }
                default: return false;
            }
        }
        return false;   // the closing quote is missing: a cut-off line
    }

    // Decimal number with optional fraction and exponent, parsed by hand so
    // the result does not depend on the C locale either.
    bool parseNumber(double& out) {
        bool negative = take('-');
        if (!std::isdigit(static_cast<unsigned char>(peek()))) return false;
        double value = 0.0;
        while (std::isdigit(static_cast<unsigned char>(peek()))) value = value * 10.0 + (s_[i_++] - '0');
        if (take('.')) {
            if (!std::isdigit(static_cast<unsigned char>(peek()))) return false;
            double scale = 0.1;
            while (std::isdigit(static_cast<unsigned char>(peek()))) {
                value += (s_[i_++] - '0') * scale;
                scale /= 10.0;
            }
        }
        if (peek() == 'e' || peek() == 'E') {
            ++i_;
            bool negExp = false;
            if (peek() == '+' || peek() == '-') negExp = s_[i_++] == '-';
            if (!std::isdigit(static_cast<unsigned char>(peek()))) return false;
            int exponent = 0;
            while (std::isdigit(static_cast<unsigned char>(peek())) && exponent < 400)
                exponent = exponent * 10 + (s_[i_++] - '0');
            while (std::isdigit(static_cast<unsigned char>(peek()))) ++i_;
            for (int k = 0; k < exponent; ++k) value = negExp ? value / 10.0 : value * 10.0;
        }
        out = negative ? -value : value;
        return true;
    }

    bool parseLiteral() {
        for (const char* word : {"true", "false", "null"}) {
            const std::string w = word;
            if (s_.compare(i_, w.size(), w) == 0) {
                i_ += w.size();
                return true;
            }
        }
        return false;
    }

    const std::string& s_;
    size_t i_ = 0;
};

std::string stringField(const std::map<std::string, JsonValue>& fields, const char* key) {
    const auto it = fields.find(key);
    return it != fields.end() && it->second.isString ? it->second.text : std::string();
}

// The answer lines of one exchange as Elm327 would see them: without the
// prompt, the echo of the command and "SEARCHING...".
std::vector<std::string> answerLines(const std::string& command, const std::string& answer) {
    std::vector<std::string> lines;
    std::string current;
    auto flush = [&] {
        const size_t a = current.find_first_not_of(" \t");
        std::string line = a == std::string::npos ? std::string() : current.substr(a);
        while (!line.empty() && (line.back() == ' ' || line.back() == '\t')) line.pop_back();
        current.clear();
        if (line.empty()) return;
        if (normalizeCommand(line) == command) return;
        if (upperAscii(line).rfind("SEARCHING", 0) == 0) return;
        lines.push_back(line);
    };
    for (char c : answer) {
        if (c == '>') continue;
        if (c == '\r' || c == '\n') flush();
        else current += c;
    }
    flush();
    return lines;
}

bool isAtOrSt(const std::string& command) {
    return command.rfind("AT", 0) == 0 || command.rfind("ST", 0) == 0;
}

// "ATSP6", "ATSPA6", "ATTP6": the protocol digit, or 0 if it is not one.
char protocolDigit(const std::string& command) {
    if (command.rfind("ATSP", 0) != 0 && command.rfind("ATTP", 0) != 0) return 0;
    std::string arg = command.substr(4);
    if (arg.size() == 2 && arg[0] == 'A') return 'A';   // automatic, starting at a protocol
    if (arg.size() != 1 || !std::isxdigit(static_cast<unsigned char>(arg[0]))) return 0;
    return arg[0];
}

// "12,3" in German, "12.3" in English.
std::string oneDecimal(double value) {
    const long long tenths = static_cast<long long>(value * 10.0 + 0.5);
    return std::to_string(tenths / 10) + L(",", ".") + std::to_string(tenths % 10);
}

}  // namespace

// ---------------------------------------------------------------------------
// Reading and describing
// ---------------------------------------------------------------------------

std::string normalizeCommand(const std::string& command) {
    std::string out;
    out.reserve(command.size());
    for (char c : command) {
        if (c == ' ' || c == '\t' || c == '\r' || c == '\n') continue;
        out += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    }
    return out;
}

bool parseRecording(const std::string& text, Recording& out, std::string& error) {
    out = Recording();
    bool haveHeader = false;
    size_t start = 0;
    while (start < text.size()) {
        size_t end = text.find('\n', start);
        const bool lastWithoutNewline = end == std::string::npos;
        if (lastWithoutNewline) end = text.size();
        std::string line = text.substr(start, end - start);
        start = end + 1;
        if (!line.empty() && line.back() == '\r') line.pop_back();
        if (line.find_first_not_of(" \t") == std::string::npos) continue;

        std::map<std::string, JsonValue> fields;
        const bool parsed = FlatJsonParser(line).parse(fields);

        if (!haveHeader) {
            if (!parsed || stringField(fields, "format") != kRecordingFormat) {
                error = L("Die Datei ist keine Caruspex-Aufnahme.",
                          "The file is not a Caruspex recording.");
                return false;
            }
            const auto version = fields.find("version");
            out.version = version != fields.end() && version->second.isNumber
                              ? static_cast<int>(version->second.number)
                              : 0;
            if (out.version < 1) {
                error = L("Die Aufnahme nennt keine gültige Formatversion.",
                          "The recording does not state a valid format version.");
                return false;
            }
            out.created = stringField(fields, "created");
            out.transport = stringField(fields, "transport");
            out.adapter = stringField(fields, "adapter");
            out.protocol = stringField(fields, "protocol");
            out.protocolId = stringField(fields, "protocolId");
            haveHeader = true;
            continue;
        }

        RecordedEvent event;
        bool valid = parsed;
        if (valid) {
            const auto t = fields.find("t");
            const auto data = fields.find("data");
            const std::string dir = stringField(fields, "dir");
            valid = t != fields.end() && t->second.isNumber && data != fields.end() &&
                    data->second.isString;
            if (dir == "tx") event.direction = RecordedDirection::Tx;
            else if (dir == "rx") event.direction = RecordedDirection::Rx;
            else if (dir == "note") event.direction = RecordedDirection::Note;
            else valid = false;
            if (valid) {
                event.t = t->second.number;
                event.data = data->second.text;
            }
        }
        if (!valid) {
            // A broken last line without a line end is what a crash while
            // writing leaves behind. Anything else broken is counted.
            if (lastWithoutNewline) out.truncated = true;
            else ++out.skippedLines;
            continue;
        }
        out.events.push_back(std::move(event));
    }
    if (!haveHeader) {
        error = L("Die Aufnahme ist leer.", "The recording is empty.");
        return false;
    }
    return true;
}

bool loadRecording(const std::string& path, Recording& out, std::string& error) {
    std::ifstream file(utf8Path(path), std::ios::binary);
    if (!file) {
        error = L("Die Aufnahme lässt sich nicht öffnen: ", "Cannot open the recording: ") + path;
        return false;
    }
    std::ostringstream content;
    content << file.rdbuf();
    return parseRecording(content.str(), out, error);
}

RecordingInfo describeRecording(const Recording& recording) {
    RecordingInfo info;
    info.created = recording.created;
    info.transport = recording.transport;
    info.adapter = recording.adapter;
    info.protocol = recording.protocol;
    info.protocolId = recording.protocolId;
    info.skippedLines = recording.skippedLines;
    info.truncated = recording.truncated;

    std::string command;
    std::string answer;
    char lastProtocol = 0;   // digit of the last ATSP / ATTP
    auto finish = [&] {
        if (command.empty()) return;
        if (const char digit = protocolDigit(command)) lastProtocol = digit;
        const std::vector<std::string> lines = answerLines(command, answer);
        if (!lines.empty()) {
            const std::string& first = lines.front();
            const bool real = first != "?" && upperAscii(first) != "OK";
            if (info.adapter.empty() && real &&
                (command == "ATZ" || command == "ATI" || command == "ATWS"))
                info.adapter = first;
            if (real && command == "ATDP") {
                if (recording.protocol.empty()) info.protocol = first;
                // Elm327::connect asks ATDP right after the protocol it set
                // with ATSP has answered, so that ATSP names the protocol.
                if (recording.protocolId.empty() && lastProtocol != 0 && lastProtocol != 'A' &&
                    lastProtocol != '0')
                    info.protocolId = std::string(1, lastProtocol);
            }
        }
    };
    for (const RecordedEvent& e : recording.events) {
        info.durationSeconds = std::max(info.durationSeconds, e.t);
        if (e.direction == RecordedDirection::Tx) {
            finish();
            command = normalizeCommand(e.data);
            answer.clear();
            if (!command.empty()) ++info.exchanges;
        } else if (e.direction == RecordedDirection::Rx) {
            answer += e.data;
        }
    }
    finish();
    return info;
}

std::string recordingSummary(const RecordingInfo& info) {
    const std::string unknown = L("unbekannt", "unknown");
    std::string text;
    text += L("Dauer: ", "Duration: ") + oneDecimal(info.durationSeconds) + " s\n";
    text += L("Befehle: ", "Commands: ") + std::to_string(info.exchanges) + "\n";
    text += L("Adapter: ", "Adapter: ") + (info.adapter.empty() ? unknown : info.adapter) + "\n";
    text += L("Protokoll: ", "Protocol: ") + (info.protocol.empty() ? unknown : info.protocol) + "\n";
    if (!info.transport.empty()) text += L("Verbindung: ", "Connection: ") + info.transport + "\n";
    if (!info.created.empty()) text += L("Aufgenommen: ", "Recorded: ") + info.created + "\n";
    if (info.skippedLines > 0)
        text += L("Nicht lesbare Zeilen übersprungen: ", "Unreadable lines skipped: ") +
                std::to_string(info.skippedLines) + "\n";
    if (info.truncated)
        text += L("Die letzte Zeile ist abgeschnitten und wurde ausgelassen.\n",
                  "The last line is cut off and was left out.\n");
    return text;
}

// ---------------------------------------------------------------------------
// RecordingTransport
// ---------------------------------------------------------------------------

RecordingTransport::RecordingTransport(std::unique_ptr<ITransport> inner, const std::string& path,
                                       RecordingMeta meta)
    : inner_(std::move(inner)), path_(path), startMs_(nowMs()) {
    file_.open(utf8Path(path), std::ios::binary | std::ios::trunc);
    if (!file_) {
        error_ = L("Die Aufnahme lässt sich nicht anlegen: ", "Cannot create the recording: ") + path;
        return;
    }
    std::string header = std::string("{\"format\":") + jsonString(kRecordingFormat, false) +
                         ",\"version\":" + std::to_string(kRecordingVersion) +
                         ",\"created\":" + jsonString(utcNow(), false) +
                         ",\"transport\":" + jsonString(inner_ ? inner_->name() : "", false);
    if (!meta.adapter.empty()) header += ",\"adapter\":" + jsonString(meta.adapter, false);
    if (!meta.protocol.empty()) header += ",\"protocol\":" + jsonString(meta.protocol, false);
    if (!meta.protocolId.empty()) header += ",\"protocolId\":" + jsonString(meta.protocolId, false);
    header += "}\n";
    file_ << header;
    file_.flush();
}

RecordingTransport::~RecordingTransport() = default;

void RecordingTransport::writeEvent(const char* direction, const std::string& data) {
    if (!file_.is_open()) return;
    file_ << "{\"t\":" << formatSeconds(nowMs() - startMs_) << ",\"dir\":\"" << direction
          << "\",\"data\":" << jsonString(data, true) << "}\n";
    // Flushed line by line: if the program dies, everything up to the last
    // complete line is still in the file.
    file_.flush();
}

bool RecordingTransport::write(const std::string& data) {
    writeEvent("tx", data);
    if (!normalizeCommand(data).empty()) ++exchanges_;
    return inner_ ? inner_->write(data) : false;
}

std::string RecordingTransport::read(int timeoutMs) {
    if (!inner_) return {};
    std::string chunk = inner_->read(timeoutMs);
    if (!chunk.empty()) writeEvent("rx", chunk);
    return chunk;
}

void RecordingTransport::flushInput() {
    if (inner_) inner_->flushInput();
}

std::string RecordingTransport::name() const { return inner_ ? inner_->name() : std::string(); }

void RecordingTransport::note(const std::string& text) {
    if (!file_.is_open()) return;
    file_ << "{\"t\":" << formatSeconds(nowMs() - startMs_) << ",\"dir\":\"note\",\"data\":"
          << jsonString(text, false) << "}\n";
    file_.flush();
}

std::unique_ptr<ITransport> RecordingTransport::release() {
    if (file_.is_open()) file_.close();
    return std::move(inner_);
}

std::unique_ptr<ITransport> RecordingTransport::unwrap(std::unique_ptr<ITransport> transport) {
    if (auto* recording = dynamic_cast<RecordingTransport*>(transport.get()))
        return recording->release();
    return transport;
}

RecordingTransport* activeRecording(Elm327& elm) {
    return dynamic_cast<RecordingTransport*>(elm.transport());
}

bool startRecording(Elm327& elm, const std::string& path, std::string& error) {
    if (!elm.connected() || !elm.transport()) {
        error = L("Keine Verbindung zum Adapter.", "No connection to the adapter.");
        return false;
    }
    if (activeRecording(elm)) {
        error = L("Es läuft schon eine Aufnahme.", "A recording is already running.");
        return false;
    }
    RecordingMeta meta;
    meta.adapter = elm.version();
    meta.protocol = elm.protocolText();
    if (elm.protocolId() != '0') meta.protocolId = std::string(1, elm.protocolId());

    RecordingTransport* started = nullptr;
    elm.wrapTransport([&](std::unique_ptr<ITransport> inner) -> std::unique_ptr<ITransport> {
        auto recording = std::make_unique<RecordingTransport>(std::move(inner), path, meta);
        if (!recording->isRecording()) {
            error = recording->error();
            return recording->release();
        }
        started = recording.get();
        return recording;
    });
    if (!started) return false;

    // The connect sequence is missing from this recording. These commands
    // stand in for it, in the order Elm327::connect sends them: each one
    // only reads or sets what connect has set already (the timing to the
    // level in effect), and the header setting of the session is restored
    // at the end. With them a replay finds the identification, the format
    // settings, the timing, an answer to 0100 and the protocol, which is all
    // Elm327::connect needs.
    started->note("recording started in a running session");
    const bool headers = elm.headersOn();
    elm.send("ATI", 1500);
    elm.send("ATE0", 1500);
    elm.send("ATL0", 1500);
    elm.send("ATS0", 1500);
    elm.send("ATH0", 1500);
    if (elm.adaptiveTimingLevel() > 0)
        elm.send("ATAT" + std::to_string(elm.adaptiveTimingLevel()), 1500);
    if (elm.isStn()) elm.send("STI", 1500);
    elm.request("0100", 0x01);
    elm.send("ATDP", 1500);
    if (headers) elm.setHeaders(true);
    started->note("session continues");
    return true;
}

bool stopRecording(Elm327& elm) {
    if (!activeRecording(elm)) return false;
    elm.wrapTransport([](std::unique_ptr<ITransport> transport) {
        return RecordingTransport::unwrap(std::move(transport));
    });
    return true;
}

// ---------------------------------------------------------------------------
// ReplayTransport
// ---------------------------------------------------------------------------

ReplayTransport::ReplayTransport(const std::string& path, ReplayPacing pacing)
    : displayName_(fileNameOf(path)), pacing_(pacing) {
    open_ = loadRecording(path, recording_, error_);
    if (open_) build();
}

ReplayTransport::ReplayTransport(Recording recording, std::string displayName, ReplayPacing pacing)
    : recording_(std::move(recording)), displayName_(std::move(displayName)), pacing_(pacing) {
    open_ = true;
    build();
}

void ReplayTransport::build() {
    exchanges_.clear();
    double sentAt = 0.0;
    for (const RecordedEvent& e : recording_.events) {
        if (e.direction == RecordedDirection::Tx) {
            exchanges_.push_back({normalizeCommand(e.data), {}});
            sentAt = e.t;
        } else if (e.direction == RecordedDirection::Rx && !exchanges_.empty()) {
            // Whatever arrived before the first command (a greeting) has
            // no command to belong to and is left out.
            exchanges_.back().answer.push_back({std::max(0.0, e.t - sentAt), e.data});
        }
    }
    const RecordingInfo info = describeRecording(recording_);
    adapterId_ = info.adapter;
    protocolId_ = info.protocolId;
    position_ = 0;
}

int ReplayTransport::find(const std::string& command) {
    const size_t count = exchanges_.size();
    const size_t windowEnd = std::min(count, position_ + static_cast<size_t>(kSearchWindow));
    auto take = [&](size_t i) {
        position_ = i + 1;
        return static_cast<int>(i);
    };
    // Forward from where the replay stands, a bounded distance.
    for (size_t i = position_; i < windowEnd; ++i) {
        if (exchanges_[i].command == command) return take(i);
    }
    // From the start: polls that run longer than the recording repeat.
    for (size_t i = 0; i < std::min(position_, count); ++i) {
        if (exchanges_[i].command == command) {
            ++wrapped_;
            return take(i);
        }
    }
    // Last resort: further ahead than the window reaches.
    for (size_t i = windowEnd; i < count; ++i) {
        if (exchanges_[i].command == command) return take(i);
    }
    return -1;
}

std::string ReplayTransport::fallbackAnswer(const std::string& command) const {
    const std::string end = "\r\r>";
    if (command.empty()) return ">";
    if (command == "ATZ" || command == "ATWS")
        return adapterId_.empty() ? "?" + end : "\r\r" + adapterId_ + end;
    if (command == "ATI") return (adapterId_.empty() ? "?" : adapterId_) + end;
    if (protocolDigit(command) != 0) return "OK" + end;
    if (isAtOrSt(command)) return "?" + end;
    if (wrongProtocol_) return "UNABLE TO CONNECT" + end;
    return "NO DATA" + end;
}

bool ReplayTransport::write(const std::string& data) {
    if (!open_) return false;
    const std::string command = normalizeCommand(data);
    const std::uint64_t now = nowMs();
    // While a wrong protocol is selected, requests do not reach any
    // recorded answer, so they do not move the position either.
    const bool refused = wrongProtocol_ && !command.empty() && !isAtOrSt(command);
    const int index = refused ? -1 : find(command);

    const char digit = protocolDigit(command);
    if (digit != 0) {
        // Only an unrecorded protocol switch needs the bookkeeping: a
        // recorded one comes with its recorded consequences.
        wrongProtocol_ = index < 0 && !protocolId_.empty() && digit != 'A' && digit != '0' &&
                         std::string(1, digit) != protocolId_;
    }

    if (index < 0) {
        ++unmatched_;
        pending_.push_back({now, fallbackAnswer(command)});
        return true;
    }
    ++matched_;
    for (const Chunk& chunk : exchanges_[static_cast<size_t>(index)].answer) {
        const std::uint64_t delay =
            pacing_ == ReplayPacing::RealTime ? static_cast<std::uint64_t>(chunk.offset * 1000.0) : 0;
        pending_.push_back({now + delay, chunk.data});
    }
    return true;
}

std::string ReplayTransport::read(int timeoutMs) {
    const std::uint64_t deadline = nowMs() + static_cast<std::uint64_t>(std::max(0, timeoutMs));
    for (;;) {
        std::string result;
        const std::uint64_t now = nowMs();
        while (!pending_.empty() && pending_.front().readyAtMs <= now) {
            result += pending_.front().data;
            pending_.pop_front();
        }
        if (!result.empty()) return result;
        if (now >= deadline) return {};
        // Like a quiet serial port: wait until something is due or the
        // timeout is over.
        std::uint64_t wake = deadline;
        if (!pending_.empty() && pending_.front().readyAtMs < wake) wake = pending_.front().readyAtMs;
        sleepMs(static_cast<int>(wake - now));
    }
}

void ReplayTransport::flushInput() {
    // Elm327 flushes right before every command; whatever is still queued
    // belongs to the previous one.
    pending_.clear();
}

std::string ReplayTransport::name() const {
    return L("Aufnahme: ", "Recording: ") + displayName_;
}

}  // namespace caruspex
