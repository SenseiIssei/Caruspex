// Row layout of the generated pack table (PidPacks.gen.cpp). Only
// PidPacks.cpp and the generated file use it; everything else goes through
// data/PidPacks.h.
#pragma once

#include <cstddef>

#include "core/Lang.h"
#include "data/Pids.h"

namespace caruspex::pidpacks::detail {

struct PackRow {
    const char* id;
    Text title;
    PidCategory category;
};

struct ValueRow {
    int pack;               // index into the pack table
    Text name;
    Text shortName;
    const char* request;    // normalized hex
    const char* header;     // empty = functional OBD addressing
    const char* response;   // empty = header + 8
    const char* equation;
    double minValue;
    double maxValue;
    const char* unit;
    const char* source;
};

const PackRow* packTable(std::size_t& count);
const ValueRow* valueTable(std::size_t& count);

}  // namespace caruspex::pidpacks::detail
