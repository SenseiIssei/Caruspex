#include "core/CanSignals.h"

#include <algorithm>
#include <cctype>
#include <cmath>
#include <cstdlib>

#include "core/Lang.h"

namespace caruspex {
namespace {

// "start|length@order sign": the field part of a DBC signal.
bool parseField(const std::string& s, std::size_t& at, int& start, int& length, bool& intel, bool* isSigned) {
    auto number = [&](int& out) {
        std::size_t begin = at;
        while (at < s.size() && std::isdigit(static_cast<unsigned char>(s[at]))) ++at;
        if (at == begin) return false;
        out = std::atoi(s.substr(begin, at - begin).c_str());
        return true;
    };
    if (!number(start) || at >= s.size() || s[at++] != '|') return false;
    if (!number(length) || at >= s.size() || s[at++] != '@') return false;
    if (at >= s.size() || (s[at] != '0' && s[at] != '1')) return false;
    intel = s[at++] == '1';
    if (isSigned) {
        if (at >= s.size() || (s[at] != '+' && s[at] != '-')) return false;
        *isSigned = s[at++] == '-';
    }
    return length >= 1 && length <= 64 && start >= 0 && start < 512;
}

}  // namespace

bool parseCanSignal(const std::string& text, CanSignal& out, std::string& error) {
    std::string s;
    for (char c : text)
        if (!std::isspace(static_cast<unsigned char>(c))) s += c;
    auto fail = [&] {
        error = L("Kein gültiges CAN-Signal (erwartet z.B. DBC:0|16@1+(0.01,0)): ",
                  "Not a valid CAN signal (expected e.g. DBC:0|16@1+(0.01,0)): ") + text;
        return false;
    };
    if (s.rfind("DBC:", 0) != 0) return fail();
    std::size_t at = 4;
    CanSignal sig;
    if (!parseField(s, at, sig.start, sig.length, sig.intel, &sig.isSigned)) return fail();
    if (at >= s.size() || s[at++] != '(') return fail();
    const std::size_t comma = s.find(',', at);
    const std::size_t close = s.find(')', at);
    if (comma == std::string::npos || close == std::string::npos || comma > close) return fail();
    char* end = nullptr;
    sig.factor = std::strtod(s.substr(at, comma - at).c_str(), &end);
    if (!end || *end) return fail();
    sig.offset = std::strtod(s.substr(comma + 1, close - comma - 1).c_str(), &end);
    if (!end || *end) return fail();
    at = close + 1;
    if (at < s.size()) {
        // Multiplexer: m<value>@<start>|<length>[@<order>]
        if (s[at++] != 'm') return fail();
        std::size_t begin = at;
        while (at < s.size() && std::isdigit(static_cast<unsigned char>(s[at]))) ++at;
        if (at == begin || at >= s.size() || s[at++] != '@') return fail();
        sig.muxValue = std::atoi(s.substr(begin, at - 1 - begin).c_str());
        bool muxIntel = true;
        int muxStart = 0, muxLength = 0;
        // The order is optional here; Intel unless said otherwise.
        std::string rest = s.substr(at);
        if (rest.find('@') == std::string::npos) rest += "@1";
        std::size_t r = 0;
        if (!parseField(rest, r, muxStart, muxLength, muxIntel, nullptr) || r != rest.size()) return fail();
        sig.muxStart = muxStart;
        sig.muxLength = muxLength;
        sig.muxIntel = muxIntel;
    }
    out = sig;
    error.clear();
    return true;
}

bool parseMonitorRequest(const std::string& request, int& canId) {
    std::string s;
    for (char c : request) s += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    if (s.size() < 4 || s.size() > 6 || s.rfind("CAN", 0) != 0) return false;
    const std::string id = s.substr(3);
    for (char c : id)
        if (!std::isxdigit(static_cast<unsigned char>(c))) return false;
    canId = static_cast<int>(std::strtol(id.c_str(), nullptr, 16));
    return canId >= 0 && canId <= 0x7FF;
}

std::uint64_t canBits(const std::vector<int>& frame, int start, int length, bool intel) {
    std::uint64_t raw = 0;
    if (intel) {
        // Bit i of the frame is bit (i % 8) of byte i / 8, the field runs
        // upwards from start.
        for (int i = 0; i < length; ++i) {
            const int pos = start + i;
            const std::size_t byte = static_cast<std::size_t>(pos / 8);
            if (byte >= frame.size()) break;
            raw |= static_cast<std::uint64_t>((frame[byte] >> (pos % 8)) & 1) << i;
        }
        return raw;
    }
    // Motorola: start is the most significant bit; within a byte the field
    // goes down towards bit 0, then continues at bit 7 of the next byte.
    int pos = start;
    for (int i = 0; i < length; ++i) {
        const std::size_t byte = static_cast<std::size_t>(pos / 8);
        const int bit = byte < frame.size() ? (frame[byte] >> (pos % 8)) & 1 : 0;
        raw = (raw << 1) | static_cast<std::uint64_t>(bit);
        pos = (pos % 8 == 0) ? pos + 15 : pos - 1;
    }
    return raw;
}

namespace {

// Highest frame byte a field touches, to tell a short frame.
int lastByte(int start, int length, bool intel) {
    if (intel) return (start + length - 1) / 8;
    int pos = start;
    int highest = pos / 8;
    for (int i = 1; i < length; ++i) {
        pos = (pos % 8 == 0) ? pos + 15 : pos - 1;
        highest = std::max(highest, pos / 8);
    }
    return highest;
}

}  // namespace

bool decodeCanSignal(const CanSignal& signal, const std::vector<int>& frame, double& value) {
    if (signal.length <= 0) return false;
    if (lastByte(signal.start, signal.length, signal.intel) >= static_cast<int>(frame.size())) return false;
    if (signal.muxValue >= 0) {
        if (lastByte(signal.muxStart, signal.muxLength, signal.muxIntel) >= static_cast<int>(frame.size())) return false;
        const auto mux = canBits(frame, signal.muxStart, signal.muxLength, signal.muxIntel);
        if (mux != static_cast<std::uint64_t>(signal.muxValue)) return false;
    }
    std::uint64_t raw = canBits(frame, signal.start, signal.length, signal.intel);
    double number = static_cast<double>(raw);
    if (signal.isSigned && signal.length < 64 && (raw >> (signal.length - 1)) & 1) {
        number = static_cast<double>(static_cast<std::int64_t>(raw) - (static_cast<std::int64_t>(1) << signal.length));
    }
    value = number * signal.factor + signal.offset;
    return std::isfinite(value);
}

}  // namespace caruspex
