// Internal layout of the generated fault code tables (tools/gen_dtc.py).
//
// The tables are plain constant arrays sorted by code, so a lookup is a
// binary search and nothing has to be built at startup. Only DtcDatabase.cpp
// and the generated files include this header.
#pragma once

#include <cstdint>
#include <span>

#include "core/Lang.h"
#include "data/DtcDatabase.h"
#include "data/Severity.h"

namespace caruspex::dtc {

inline constexpr std::uint16_t kNoDetail = 0xFFFF;

struct Record {
    const char* code;
    Text text;
    Severity severity;
    DtcSystem system;
    std::uint16_t detail;  // index into details(), or kNoDetail
    bool throttle;
};

// Plain-language help shared by many codes. Causes and checks hold one
// entry per line, causes ordered by likelihood.
struct Detail {
    Text advice;
    Text causes;
    Text checks;
    CostClass cost;
};

struct Family {
    const char* key;
    Text label;
    const char* brands;  // normalised brand names separated by '|'
    std::span<const Record> records;
};

// Meaning of a number range for codes that are in no table. lo and hi are
// the hex value of the four digits after the letter.
struct RangeRule {
    char letter;
    int lo;
    int hi;
    Text text;
    Severity severity;
    DtcSystem system;
    std::uint16_t detail;
};

std::span<const Record> genericPowertrain();
std::span<const Record> genericChassis();
std::span<const Record> genericBody();
std::span<const Record> genericNetwork();
std::span<const Family> families();
std::span<const Detail> details();
std::span<const RangeRule> rangeRules();

}  // namespace caruspex::dtc
