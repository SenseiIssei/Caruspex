#include "core/analysis/Common.h"

#include <cmath>
#include <cstdio>

#include "core/Lang.h"

namespace caruspex::analysis {

std::string Phrase::text() const { return L(de.c_str(), en.c_str()); }

Phrase operator+(const Phrase& a, const Phrase& b) {
    auto join = [](const std::string& x, const std::string& y) {
        if (x.empty()) return y;
        if (y.empty()) return x;
        return x + " " + y;
    };
    return {join(a.de, b.de), join(a.en, b.en)};
}

std::string formatNumber(double value, int decimals, bool german) {
    if (decimals < 0) decimals = 0;
    if (decimals > 6) decimals = 6;
    char buffer[64];
    std::snprintf(buffer, sizeof buffer, "%.*f", decimals, value);
    std::string out(buffer);
    // "-0" or "-0.0" looks odd next to a measured value.
    bool allZero = true;
    for (char c : out) {
        if (c != '-' && c != '0' && c != '.') {
            allZero = false;
            break;
        }
    }
    if (allZero && !out.empty() && out[0] == '-') out.erase(out.begin());
    if (german) {
        for (char& c : out) {
            if (c == '.') c = ',';
        }
    }
    return out;
}

Phrase number(double value, int decimals) {
    return {formatNumber(value, decimals, true), formatNumber(value, decimals, false)};
}

Phrase signedNumber(double value, int decimals) {
    Phrase p = number(value, decimals);
    // Only add the plus if the rounded value is really above zero.
    const double scale = std::pow(10.0, decimals);
    if (std::round(value * scale) > 0) {
        p.de = "+" + p.de;
        p.en = "+" + p.en;
    }
    return p;
}

Phrase format(const char* de, const char* en, std::initializer_list<Phrase> args) {
    auto fill = [&](const char* pattern, bool german) {
        std::string out;
        auto arg = args.begin();
        for (const char* p = pattern; *p; ++p) {
            if (p[0] == '{' && p[1] == '}') {
                if (arg != args.end()) {
                    out += german ? arg->de : arg->en;
                    ++arg;
                }
                ++p;
            } else {
                out += *p;
            }
        }
        return out;
    };
    return {fill(de, true), fill(en, false)};
}

const char* levelKey(Level level) {
    switch (level) {
        case Level::Good: return "good";
        case Level::Info: return "info";
        case Level::Warning: return "warning";
        case Level::Critical: return "critical";
    }
    return "?";
}

}  // namespace caruspex::analysis
