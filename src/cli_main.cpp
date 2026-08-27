// Konsolen-Testtool fuer den Kern. Die eigentliche Anwendung ist caruspex.exe.
//
//   caruspex_cli ports
//   caruspex_cli kklcheck --port COM3
//   caruspex_cli read --tcp 127.0.0.1:35000
//   caruspex_cli read --port COM5 --baud 38400

#include <cstdio>
#include <cstring>
#include <memory>
#include <string>

#include <chrono>

#include "core/Elm327.h"
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
        std::printf("Kein COM-Port gefunden.\n");
        return;
    }
    for (const PortInfo& p : ports) {
        std::printf("  %-8s %s\n", p.device.c_str(), p.description.c_str());
        if (p.vid) std::printf("           USB VID_%04X PID_%04X\n", p.vid, p.pid);
        if (!p.chipGuess.empty()) std::printf("           Chip: %s\n", p.chipGuess.c_str());
    }
}


// Prueft ein KKL-Kabel, ohne dass es am Auto haengt. Getestet wird, was die
// 5-Baud-Weckadresse braucht: Break-Leitung schalten und Baudrate im Betrieb
// aendern. Dazu wird nachgemessen, wie genau die Bitzeiten eingehalten werden.
int kklCheck(const std::string& port) {
    std::printf("KKL-Selbsttest auf %s\n", port.c_str());
    std::printf("Das Kabel muss dafuer NICHT am Auto stecken.\n\n");

    Serial line;
    std::string error;
    if (!line.open(port, 9600, error)) {
        std::printf("[fehlt] Anschluss laesst sich nicht oeffnen: %s\n", error.c_str());
        return 2;
    }
    std::printf("[ok]    Anschluss geoeffnet mit 9600 Baud\n");

    // 1) Baudrate im laufenden Betrieb aendern.
    bool baudOk = true;
    for (int baud : {9600, 10400, 4800, 9600}) {
        if (!line.changeBaud(baud)) {
            std::printf("[fehlt] Baudrate %d wird nicht angenommen\n", baud);
            baudOk = false;
        }
    }
    if (baudOk) std::printf("[ok]    Baudrate laesst sich im Betrieb umschalten\n");

    // 2) Break-Leitung schalten. Damit wird die Weckadresse getaktet.
    if (!line.setBreak(true) || !line.setBreak(false)) {
        std::printf("[fehlt] Break-Leitung laesst sich nicht schalten. Ohne das geht die "
                    "5-Baud-Weckadresse nicht.\n");
        line.close();
        return 3;
    }
    std::printf("[ok]    Break-Leitung laesst sich schalten\n");

    // 3) Bitzeiten nachmessen. 5 Baud heisst 200 ms je Bit. Windows schlaeft
    //    gern zu lange, und zu lange Bits versteht das Steuergeraet nicht.
    std::printf("\nBitzeiten werden gemessen (10 Bit wie bei einer echten "
                "Weckadresse) ...\n");
    // Genauso getaktet wie die echte Weckadresse: feste Zielzeitpunkte,
    // gerechnet vom Anfang, damit sich nichts aufaddiert.
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
    // Vor dem Schliessen stoppen: einen USB-Port zu schliessen dauert gut
    // hundert Millisekunden und wuerde die Messung verfaelschen.
    const double total = previous;
    line.setBreak(false);
    line.close();

    const double average = total / 10.0;
    std::printf("        Mittel %.1f ms, staerkste Abweichung %.1f ms\n", average, worst);
    std::printf("        Gesamtlaenge %.0f ms (Sollwert 2000 ms)\n", total);

    if (std::abs(average - 200.0) <= 8.0) {
        std::printf("[ok]    Taktung genau genug fuer die Weckadresse\n");
    } else {
        std::printf("[warn]  Taktung weicht ab. Die Weckadresse kann daran scheitern.\n");
    }

    std::printf("\nErgebnis: Das Kabel kann alles, was das Protokoll braucht.\n");
    std::printf("Ob das Steuergeraet antwortet, zeigt sich erst am Auto mit Zuendung an.\n");
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
