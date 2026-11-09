// Drive cycle help: what each readiness monitor checks and how to drive so
// that it finishes.
//
// The exact enabling conditions (temperatures, speeds, times) are defined by
// each manufacturer and often by each engine. The patterns here are the
// generic ones that work on most cars; every guide carries that note so
// nobody takes them for a factory specification.
#pragma once

#include <vector>

#include "core/Lang.h"
#include "core/analysis/Readiness.h"

namespace caruspex::analysis {

struct MonitorGuide {
    Monitor monitor;
    Text name;          // short name, e.g. "Tankentlüftung"
    Text explanation;   // what the test checks, for non-mechanics
    Text drivePattern;  // a generic way to drive that usually completes it
    Text typicalTime;   // rough idea of how long it takes
    bool needsColdStart = false;  // starts with an engine that sat for hours
};

// Guide for one monitor.
const MonitorGuide& driveCycleGuide(Monitor monitor);

// All guides, in the order of the readiness list.
const std::vector<MonitorGuide>& allDriveCycleGuides();

// The guides for all supported monitors that are not complete yet, cold
// start ones first, because a cold start is the hardest to arrange.
std::vector<const MonitorGuide*> guidesForIncomplete(const std::vector<MonitorState>& monitors);

// Conditions that help almost every monitor (cold start, fuel level, no
// stored faults, no clearing in between).
Text driveCycleBasics();

// The honest note: conditions vary by manufacturer.
Text manufacturerNote();

}  // namespace caruspex::analysis
