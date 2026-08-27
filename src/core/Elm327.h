// ELM327-Treiber: Initialisierung, Protokollwahl, Kommandos, Antwort-Parsing.
#pragma once

#include <functional>
#include <memory>
#include <string>
#include <vector>

#include "core/Transport.h"

namespace caruspex {

using LogFn = std::function<void(const std::string&)>;

enum class ElmStatus {
    Ok,
    NoData,        // Steuergeraet kennt den Dienst nicht
    BusError,      // kein Kontakt zum Fahrzeug
    AdapterError,  // Adapter antwortet gar nicht / kein ELM327
    Negative,      // Steuergeraet lehnt ab (7F ...)
};

struct ElmResult {
    ElmStatus status = ElmStatus::Ok;
    std::vector<std::string> lines;              // Rohzeilen
    std::vector<std::vector<int>> messages;      // dekodierte Nutzdaten ohne Dienst-Byte
    std::string message;                         // Klartext bei Fehlern
    bool ok() const { return status == ElmStatus::Ok; }
};

// Reihenfolge passend zu VAG: K-Line zuerst (A2), dann CAN (A4 B7).
extern const char* const kProtocolOrderKLineFirst;
extern const char* const kProtocolOrderCanFirst;

std::string protocolName(char id);

class Elm327 {
public:
    void setLog(LogFn log) { log_ = std::move(log); }
    // Jedes Zeitlimit wird mit diesem Faktor multipliziert. Traege Klone
    // brauchen mehr Zeit; die Abstufung zwischen kurzen und langen Wartezeiten
    // bleibt dabei erhalten, weil nicht alles auf denselben Wert gesetzt wird.
    void setTimeoutScale(double scale) { timeoutScale_ = scale > 0.0 ? scale : 1.0; }
    // Jede gesendete Zeile samt Antwort mitschreiben.
    void setVerbose(bool verbose) { verbose_ = verbose; }

    // uebernimmt den Transport
    bool connect(std::unique_ptr<ITransport> transport, const std::string& protocolOrder,
                 std::string& error);
    void disconnect();
    bool connected() const { return connected_; }

    ElmResult send(const std::string& command, int timeoutMs = 5000);
    // Sendet und dekodiert die Antwort auf den erwarteten Dienst (z.B. 0x03 -> 0x43).
    ElmResult request(const std::string& command, int expectedService, int timeoutMs = 6000);

    const std::string& version() const { return version_; }
    const std::string& protocolText() const { return protocolText_; }
    std::string transportName() const { return transport_ ? transport_->name() : ""; }

private:
    ElmResult at(const std::string& command, int timeoutMs = 5000);
    void drainGreeting();

    std::unique_ptr<ITransport> transport_;
    LogFn log_;
    double timeoutScale_ = 1.0;
    bool verbose_ = true;
    std::string version_ = "?";
    std::string protocolText_ = "unbekannt";
    bool connected_ = false;
};

// Hilfsfunktionen, auch fuer Tests nutzbar.
std::vector<int> hexToBytes(const std::string& text);
ElmResult decodeMessages(const std::vector<std::string>& lines, int expectedService);

}  // namespace caruspex
