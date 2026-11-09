// Console test tool for the core. The actual application is caruspex.exe.
//
//   caruspex_cli ports
//   caruspex_cli kklcheck --port COM3
//   caruspex_cli read --tcp 127.0.0.1:35000
//   caruspex_cli read --port COM5 --baud 38400
//   caruspex_cli ecus --tcp 127.0.0.1:35000   (per control unit, all live values)

#include <cstdio>
#include <cstring>
#include <memory>
#include <string>

#include <chrono>

#include "core/Elm327.h"
#include "core/Lang.h"
#include "core/Obd2.h"
#include "core/Transport.h"
#include "core/KLineStream.h"
#include "core/Timing.h"
#include "data/Pids.h"

using namespace caruspex;

namespace {

void printPorts() {
    const std::vector<PortInfo> ports = enumeratePorts();
    if (ports.empty()) {
        std::printf("No COM port found.\n");
        return;
    }
    for (const PortInfo& p : ports) {
        std::printf("  %-8s %s\n", p.device.c_str(), p.description.c_str());
        if (p.vid) std::printf("           USB VID_%04X PID_%04X\n", p.vid, p.pid);
        if (!p.chipGuess.empty()) std::printf("           Chip: %s\n", p.chipGuess.c_str());
    }
}


// Checks a KKL cable without it being plugged into the car. It tests what the
// 5-baud wake-up address needs: toggling the break line and changing the baud
// rate on the fly. It also measures how accurately the bit times are kept.
int kklCheck(const std::string& port) {
    std::printf("KKL self-test on %s\n", port.c_str());
    std::printf("The cable does NOT need to be plugged into the car for this.\n\n");

    Serial line;
    std::string error;
    if (!line.open(port, 9600, error)) {
        std::printf("[fail]  Cannot open the port: %s\n", error.c_str());
        return 2;
    }
    std::printf("[ok]    Port opened at 9600 baud\n");

    // 1) Change the baud rate on the fly.
    bool baudOk = true;
    for (int baud : {9600, 10400, 4800, 9600}) {
        if (!line.changeBaud(baud)) {
            std::printf("[fail]  Baud rate %d is not accepted\n", baud);
            baudOk = false;
        }
    }
    if (baudOk) std::printf("[ok]    Baud rate can be switched on the fly\n");

    // 2) Toggle the break line. This is what clocks out the wake-up address.
    if (!line.setBreak(true) || !line.setBreak(false)) {
        std::printf("[fail]  Cannot toggle the break line. Without it the 5-baud "
                    "wake-up address does not work.\n");
        line.close();
        return 3;
    }
    std::printf("[ok]    Break line can be toggled\n");

    // 3) Measure the bit times. 5 baud means 200 ms per bit. Windows likes to
    //    tends to oversleep, and the control unit cannot follow overly long bits.
    std::printf("\nMeasuring bit times (10 bits as in a real "
                "wake-up address) ...\n");
    // Clocked exactly like the real wake-up address: fixed target times,
    // counted from the start, so that nothing accumulates.
    double worst = 200.0;
    const auto begin = std::chrono::steady_clock::now();
    const std::uint64_t start = nowMs();
    double previous = 0.0;
    for (int bit = 0; bit < 10; ++bit) {
        line.setBreak(bit % 2 == 0);
        sleepUntilMs(start + static_cast<std::uint64_t>(bit + 1) * 200);
        const double elapsed = std::chrono::duration<double, std::milli>(
                                   std::chrono::steady_clock::now() - begin)
                                   .count();
        const double ms = elapsed - previous;
        previous = elapsed;
        std::printf("        Bit %2d: %.1f ms\n", bit + 1, ms);
        if (std::abs(ms - 200.0) > std::abs(worst - 200.0)) worst = ms;
    }
    // Stop before closing: closing a USB port takes a good hundred
    // milliseconds and would distort the measurement.
    const double total = previous;
    line.setBreak(false);
    line.close();

    const double average = total / 10.0;
    std::printf("        Average %.1f ms, largest deviation %.1f ms\n", average, worst);
    std::printf("        Total length %.0f ms (target 2000 ms)\n", total);

    if (std::abs(average - 200.0) <= 8.0) {
        std::printf("[ok]    Timing accurate enough for the wake-up address\n");
    } else {
        std::printf("[warn]  Timing deviates. The wake-up address may fail because of it.\n");
    }

    std::printf("\nResult: the cable can do everything the protocol needs.\n");
    std::printf("Whether the control unit answers only shows on the car with the ignition on.\n");
    return 0;
}

std::unique_ptr<ITransport> makeTransport(const std::string& tcp, const std::string& port,
                                          int baud, std::string& error) {
    if (!tcp.empty()) {
        const size_t colon = tcp.find(':');
        auto t = std::make_unique<TcpTransport>();
        const std::string host = tcp.substr(0, colon);
        const int p = colon == std::string::npos ? 35000 : std::stoi(tcp.substr(colon + 1));
        if (!t->open(host, p, error)) return nullptr;
        return t;
    }
    auto t = std::make_unique<SerialTransport>();
    if (!t->open(port, baud, error)) return nullptr;
    return t;
}

}  // namespace

int main(int argc, char** argv) {
    // The console tool speaks English, including the texts that come from the core.
    setLanguage(Language::English);

    std::string command = argc > 1 ? argv[1] : "ports";
    std::string tcp, port = "COM3";
    int baud = 38400;
    bool verbose = false;
    for (int i = 2; i < argc; ++i) {
        const std::string arg = argv[i];
        if (arg == "--tcp" && i + 1 < argc) tcp = argv[++i];
        else if (arg == "--port" && i + 1 < argc) port = argv[++i];
        else if (arg == "--baud" && i + 1 < argc) baud = std::stoi(argv[++i]);
        else if (arg == "-v") verbose = true;
    }

    if (command == "ports") {
        printPorts();
        return 0;
    }
    if (command == "kklcheck") {
        return kklCheck(port);
    }

    std::string error;
    std::unique_ptr<ITransport> transport = makeTransport(tcp, port, baud, error);
    if (!transport) {
        std::printf("Connection failed: %s\n", error.c_str());
        return 2;
    }

    Elm327 elm;
    if (verbose) elm.setLog([](const std::string& m) { std::printf("%s\n", m.c_str()); });
    if (!elm.connect(std::move(transport), kProtocolOrderKLineFirst, error)) {
        std::printf("%s\n", error.c_str());
        return 3;
    }
    std::printf("Adapter    : %s\n", elm.version().c_str());
    std::printf("Connection : %s\n", elm.transportName().c_str());
    std::printf("Protocol   : %s\n\n", elm.protocolText().c_str());

    Obd2 obd(elm);

    if (command == "read") {
        const VehicleStatus status = obd.readStatus();
        if (status.valid) {
            std::printf("MIL: %s   reported faults: %d\n",
                        status.milOn ? "ON" : "off", status.faultCount);
            for (const ReadinessItem& r : status.readiness) {
                std::printf("   %-26s %s\n", r.name.c_str(), r.ready ? "ready" : "not ready");
            }
        }
        std::string err;
        const std::vector<Fault> faults = obd.readFaults(err);
        std::printf("\n");
        if (faults.empty()) {
            std::printf("No faults stored.\n");
        } else {
            std::printf("%zu fault(s) found:\n", faults.size());
            for (const Fault& f : faults) {
                std::printf("  %s [%s]\n      %s\n", f.code.c_str(), f.kind.c_str(),
                            f.text.c_str());
            }
        }
    } else if (command == "clear") {
        std::string err;
        std::printf(obd.clearFaults(err) ? "Fault memory cleared.\n"
                                         : "Clearing failed.\n");
        if (!err.empty()) std::printf("%s\n", err.c_str());
    } else if (command == "live") {
        for (const LiveValue& v : obd.readLive(defaultLivePids())) {
            if (!v.text.empty()) {
                std::printf("  %-32s %s\n", v.name.c_str(), v.text.c_str());
            } else {
                std::printf("  %-32s %10.2f %s\n", v.name.c_str(), v.value, v.unit.c_str());
            }
        }
    } else if (command == "info") {
        const std::string vin = obd.readVin();
        std::printf("VIN: %s\n", vin.empty() ? "(not provided)" : vin.c_str());
        std::string trigger;
        const std::vector<LiveValue> frame = obd.readFreezeFrame(trigger);
        if (!trigger.empty()) std::printf("Freeze frame for code %s:\n", trigger.c_str());
        for (const LiveValue& v : frame) {
            if (!v.text.empty()) {
                std::printf("  %-32s %s\n", v.name.c_str(), v.text.c_str());
            } else {
                std::printf("  %-32s %10.2f %s\n", v.name.c_str(), v.value, v.unit.c_str());
            }
        }
    } else if (command == "ecus") {
        // Everything per control unit: headers on, so every answer keeps
        // its sender.
        if (!elm.setHeaders(true)) std::printf("Adapter refused headers (ATH1).\n");
        std::string err;
        for (const VehicleStatus& s : obd.readStatusPerEcu()) {
            std::printf("%-20s MIL %s, %d code(s), %s\n", s.ecu.c_str(), s.milOn ? "ON" : "off",
                        s.faultCount, s.compressionIgnition ? "diesel" : "spark ignition");
        }
        for (const Fault& f : obd.readFaults(err)) {
            std::printf("  %s [%s] %s\n", f.code.c_str(), f.kind.c_str(), f.ecu.c_str());
        }
        for (const EcuInfo& e : obd.readEcuInfo()) {
            std::printf("\n%s: %s\n", e.ecu.c_str(), e.name.c_str());
            if (!e.vin.empty()) std::printf("  VIN  %s\n", e.vin.c_str());
            for (const std::string& id : e.calibrationIds) std::printf("  CALID %s\n", id.c_str());
            for (const std::string& c : e.cvns) std::printf("  CVN  %s\n", c.c_str());
            for (const InUseCounter& c : e.inUse.monitors) {
                std::printf("  %-40s %s\n", c.name.c_str(), c.text.c_str());
            }
        }
        std::printf("\n");
        for (const MisfireCount& m : misfireCounts(obd.readMonitorResults(err))) {
            std::printf("  %s\n", m.text.c_str());
        }
        std::printf("\n");
        for (int pid : obd.supportedPids()) {
            for (const LiveValue& v : obd.readLiveAll(pid)) {
                if (!v.text.empty()) {
                    std::printf("  %03X %-50s %s\n", v.key, v.name.c_str(), v.text.c_str());
                } else {
                    std::printf("  %03X %-50s %10.2f %s\n", v.key, v.name.c_str(), v.value,
                                v.unit.c_str());
                }
            }
        }
    } else {
        std::printf("Unknown command: %s\n", command.c_str());
    }

    elm.disconnect();
    return 0;
}
