// Konsolen-Testtool fuer den Kern. Die eigentliche Anwendung ist caruspex.exe.
//
//   caruspex_cli ports
//   caruspex_cli read --tcp 127.0.0.1:35000
//   caruspex_cli read --port COM5 --baud 38400

#include <cstdio>
#include <cstring>
#include <memory>
#include <string>

#include "core/Elm327.h"
#include "core/Obd2.h"
#include "core/Transport.h"
#include "data/Pids.h"

using namespace caruspex;

namespace {

void printPorts() {
    const std::vector<PortInfo> ports = enumeratePorts();
    if (ports.empty()) {
        std::printf("Kein COM-Port gefunden.\n");
        return;
    }
    for (const PortInfo& p : ports) {
        std::printf("  %-8s %s\n", p.device.c_str(), p.description.c_str());
        if (p.vid) std::printf("           USB VID_%04X PID_%04X\n", p.vid, p.pid);
        if (!p.chipGuess.empty()) std::printf("           Chip: %s\n", p.chipGuess.c_str());
    }
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

    std::string error;
    std::unique_ptr<ITransport> transport = makeTransport(tcp, port, baud, error);
    if (!transport) {
        std::printf("Verbindung fehlgeschlagen: %s\n", error.c_str());
        return 2;
    }

    Elm327 elm;
    if (verbose) elm.setLog([](const std::string& m) { std::printf("%s\n", m.c_str()); });
    if (!elm.connect(std::move(transport), kProtocolOrderKLineFirst, error)) {
        std::printf("%s\n", error.c_str());
        return 3;
    }
    std::printf("Adapter    : %s\n", elm.version().c_str());
    std::printf("Verbindung : %s\n", elm.transportName().c_str());
    std::printf("Protokoll  : %s\n\n", elm.protocolText().c_str());

    Obd2 obd(elm);

    if (command == "read") {
        const VehicleStatus status = obd.readStatus();
        if (status.valid) {
            std::printf("Fehlerlampe: %s   gemeldete Fehler: %d\n",
                        status.milOn ? "AN" : "aus", status.faultCount);
            for (const ReadinessItem& r : status.readiness) {
                std::printf("   %-26s %s\n", r.name.c_str(), r.ready ? "bereit" : "nicht bereit");
            }
        }
        std::string err;
        const std::vector<Fault> faults = obd.readFaults(err);
        std::printf("\n");
        if (faults.empty()) {
            std::printf("Kein Fehler gespeichert.\n");
        } else {
            std::printf("%zu Fehler gefunden:\n", faults.size());
            for (const Fault& f : faults) {
                std::printf("  %s [%s]\n      %s\n", f.code.c_str(), f.kind.c_str(),
                            f.text.c_str());
            }
        }
    } else if (command == "clear") {
        std::string err;
        std::printf(obd.clearFaults(err) ? "Fehlerspeicher geloescht.\n"
                                         : "Loeschen fehlgeschlagen.\n");
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
        std::printf("Fahrgestellnummer: %s\n", vin.empty() ? "(nicht geliefert)" : vin.c_str());
        std::string trigger;
        const std::vector<LiveValue> frame = obd.readFreezeFrame(trigger);
        if (!trigger.empty()) std::printf("Freeze Frame zu Code %s:\n", trigger.c_str());
        for (const LiveValue& v : frame) {
            if (!v.text.empty()) {
                std::printf("  %-32s %s\n", v.name.c_str(), v.text.c_str());
            } else {
                std::printf("  %-32s %10.2f %s\n", v.name.c_str(), v.value, v.unit.c_str());
            }
        }
    } else {
        std::printf("Unbekannter Befehl: %s\n", command.c_str());
    }

    elm.disconnect();
    return 0;
}
