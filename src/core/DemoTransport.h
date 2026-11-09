// Built-in demo vehicle: a simulated ELM327 with a simulated car behind it.
//
// DemoTransport plugs into Elm327::connect like a serial port or a Wi-Fi
// dongle, but everything happens in-process. It plays the adapter character
// by character the way a real one talks: echo until ATE0, the '>' prompt,
// "SEARCHING..." on automatic protocol search, "NO DATA", "?" for unknown
// commands, and CAN frames with ISO-TP multi-frame responses, with or without
// headers.
//
// Behind the adapter sits a fictional modern petrol car on CAN (11 bit,
// 500 kbaud) with two control units: engine (answers on 7E8) and automatic
// transmission (7E9). Its state is a pure function of the elapsed time since
// "ignition on": the engine starts after a few seconds, warms up at idle and
// then repeats a drive cycle (accelerate, cruise, brake, stand). Fault codes,
// freeze frame, readiness, on-board monitoring and identification behave
// consistently, including clearing with mode 04.
//
// Addressed physically (ATSH, ATCRA) the car also speaks UDS (ISO 14229):
// tester present, identification (22 F187 ...), fault memory (19 02) and
// clearing (14 FF FF FF). Besides engine (7E0) and gearbox (7E1) four units
// answer only this way: ABS 760, airbag 737, instrument cluster 720 and
// climate 744, each on request + 8. The engine also has a few maker values
// under service 22 (oil level, oil temperature, starts, a multi-frame block),
// read by the demo pack of the custom values (data/pids/caruspex-demo.csv).
//
// For tests the clock can be injected and the answer delay switched off, so
// every response is deterministic.
#pragma once

#include <functional>
#include <memory>
#include <string>

#include "core/Transport.h"

namespace caruspex {

// Which adapter the demo pretends to be. The default is the most common one
// in the wild: a clone that reports "v1.5", a version the original
// manufacturer never released. The others exist mainly so the adapter probe
// can be tested against every verdict it knows.
enum class DemoAdapter {
    CloneV15,        // "ELM327 v1.5", command set of v1.4
    GenuineV14b,     // behaves like a real ELM327 v1.4b
    CloneV21,        // claims v2.1, command set of v1.4
    CloneOkToAll,    // claims v2.1 and answers "OK" even to commands it does not know
    ObdLinkStn,      // STN chip (OBDLink), ELM327 v1.4b compatible plus ST commands
};

struct DemoOptions {
    DemoAdapter adapter = DemoAdapter::CloneV15;

    // Seconds since "ignition on". Empty: real time since construction.
    // Tests pass a lambda over a variable they advance themselves.
    std::function<double()> clock;

    // Scales the simulated answer delays (a few milliseconds per request).
    // 0 answers at once, which keeps tests fast and deterministic.
    double latencyScale = 1.0;

    // Seconds after ignition on at which the engine is started.
    double engineStartSeconds = 4.0;

    // Whether the transmission control unit (7E9) is present.
    bool transmissionEcu = true;
};

class DemoTransport : public ITransport {
public:
    explicit DemoTransport(DemoOptions options = {});
    ~DemoTransport() override;

    DemoTransport(const DemoTransport&) = delete;
    DemoTransport& operator=(const DemoTransport&) = delete;

    bool write(const std::string& data) override;
    std::string read(int timeoutMs) override;
    void flushInput() override;
    std::string name() const override;

    // The vehicle identification number of the demo car (17 characters,
    // valid check digit, fictional manufacturer code).
    static const char* vin();

private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};

}  // namespace caruspex
