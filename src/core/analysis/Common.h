// Shared building blocks of the evaluation modules (health check, trip
// computer, performance timer, battery test).
//
// Everything under core/analysis is pure computation: no I/O, no Qt, no
// clock. The caller collects the data (from the adapter, from a recording or
// from a test) and hands it in; the modules only judge it. That keeps every
// rule testable with concrete numbers.
//
// Texts that contain measured numbers cannot live in static tables, so they
// are built at run time as a Phrase: both languages side by side, and the
// active one is picked with L() from core/Lang.h when it is shown. That way
// a result can be computed once and still follow a later language switch.
#pragma once

#include <initializer_list>
#include <string>

namespace caruspex::analysis {

// How serious a single finding is. The order matters: higher is worse.
enum class Level {
    Good,       // checked and fine
    Info,       // worth knowing, nothing to do right now
    Warning,    // should be looked at soon
    Critical,   // needs attention before driving much further
};

// The overall result as a traffic light.
enum class TrafficLight { Green, Yellow, Red };

// Kind of fuel. Decides formulas (air/fuel ratio, density, CO2) and some
// rules (readiness for diesel engines).
enum class FuelType { Unknown, Petrol, Diesel };

// A text in both languages, built at run time.
struct Phrase {
    std::string de;
    std::string en;

    // The text in the active language.
    std::string text() const;
    bool empty() const { return de.empty() && en.empty(); }
};

// Joins two phrases with a space in each language.
Phrase operator+(const Phrase& a, const Phrase& b);

// Formats a number with the decimal separator of the language: comma in
// German, point in English. Negative zero is printed as 0.
std::string formatNumber(double value, int decimals, bool german);

// Formats a number for both languages at once, e.g. {"12,4", "12.4"}.
Phrase number(double value, int decimals);

// Same, with a leading plus sign for positive values (for fuel trims).
Phrase signedNumber(double value, int decimals);

// Builds a phrase from a German and an English pattern. Each "{}" is replaced
// by the next argument in the matching language, e.g.
// format("{} V gemessen", "{} V measured", {number(12.4, 1)}).
Phrase format(const char* de, const char* en, std::initializer_list<Phrase> args = {});

// A phrase that reads the same in both languages (codes, units, names that
// were already resolved in one language).
inline Phrase same(const std::string& s) { return {s, s}; }

// Worst of two levels.
inline Level worse(Level a, Level b) { return a < b ? b : a; }

// Short name of a level, for logs and tests ("good", "info", ...).
const char* levelKey(Level level);

}  // namespace caruspex::analysis
