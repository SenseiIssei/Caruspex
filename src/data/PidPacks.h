// Built-in packs of custom values: manufacturer values (service 22 and 21)
// for particular cars, starting with electric cars, plus a pack for the
// demo car.
//
// The packs are maintained as Torque CSV in data/pids/ and turned into C++
// by tools/gen_pids.py, like the vehicle catalog. Only values with a
// reliable public source go in; the source of each value is in
// CustomPid::source.
//
// Names come out in the active language (see core/Lang.h), like the PID
// table: builtInPacks() hands out the list for the language active at the
// time of the call, and the references stay valid for the whole run.
//
// Keys are kPackKeyBase + position in builtInPacks(). They stay the same
// within one build but move when a pack gains or loses values, so anything
// stored across runs should name a value by pack and request plus name, not
// by key.
#pragma once

#include <string>
#include <vector>

#include "core/CustomPids.h"

namespace caruspex {

// All values of all packs, packs in the order of packNames().
const std::vector<CustomPid>& builtInPacks();

// Ids of the packs ("caruspex-demo", "hyundai-kona-niro-ev", ...), the demo
// pack first.
std::vector<std::string> packNames();

// Display title of a pack in the active language, empty for an unknown id.
std::string packTitle(const std::string& name);

// Category the values of a pack are shown under (HybridEv for the electric
// cars, Custom for the demo). Custom for an unknown id.
PidCategory packCategory(const std::string& name);

// The values of one pack, empty for an unknown id.
std::vector<CustomPid> packPids(const std::string& name);

}  // namespace caruspex
