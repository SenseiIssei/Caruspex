// Test helper: an Elm327 connected to the built-in demo car, with a clock
// the test moves by hand.
#pragma once

#include <memory>
#include <string>
#include <vector>

#include "core/DemoTransport.h"
#include "core/Elm327.h"

namespace caruspex {

// Connects on protocol 6 without delays. Every line Elm327 logs (command and
// answer) ends up in `sent`, so a test can check what went to the adapter.
struct DemoRig {
    double now = 0.0;
    Elm327 elm;
    std::string error;
    bool connected = false;
    std::vector<std::string> sent;

    explicit DemoRig(double start, DemoAdapter adapter = DemoAdapter::CloneV15,
                     bool gearbox = true, double latency = 0.0) {
        now = start;
        DemoOptions options;
        options.adapter = adapter;
        options.clock = [this] { return now; };
        options.latencyScale = latency;
        options.transmissionEcu = gearbox;
        elm.setVerbose(true);
        elm.setLog([this](const std::string& line) { sent.push_back(line); });
        connected = elm.connect(std::make_unique<DemoTransport>(options), "6", error);
    }

    DemoRig(const DemoRig&) = delete;
    DemoRig& operator=(const DemoRig&) = delete;
};

}  // namespace caruspex
