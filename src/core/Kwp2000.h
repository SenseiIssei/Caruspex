// KWP2000 auf der K-Leitung, der Nachfolger von KWP1281 bei VAG.
//
// Ab etwa 2001 haben die Steuergeraete umgestellt. Der Audi A2 liegt genau auf
// der Grenze: der 1.4 mit Saugrohreinspritzung spricht meist noch KWP1281, der
// 1.6 FSI mit seinem Bosch MED meist schon KWP2000.
//
// Anders als KWP1281 ist das kein Blockprotokoll mit Quittung je Byte, sondern
// es werden ganze Nachrichten mit Kopf und Pruefsumme geschickt:
//
//     [Format] [Ziel] [Quelle] [Daten ...] [Pruefsumme]
//
// Das Formatbyte traegt die Laenge der Daten in den unteren sechs Bit.
#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "core/Kwp1281.h"   // IByteStream, Ecu, MeasuringValue

namespace caruspex {

// Die beiden Schluesselbytes nach der Weckadresse sagen, was das Steuergeraet
// spricht.
enum class KLineProtocol {
    Unknown,
    Kwp1281,
    Kwp2000,
};

KLineProtocol protocolFromKeyBytes(std::uint8_t key1, std::uint8_t key2);
std::string protocolLabel(KLineProtocol protocol);

// Weckt ein Steuergeraet mit der 5-Baud-Adresse und liest die Schluesselbytes.
// Danach steht fest, welches Protokoll gefahren wird. Beide Protokolle teilen
// sich diesen Anfang, deshalb liegt er hier zentral.
bool wakeUpKLine(IByteStream& stream, Ecu ecu, std::uint8_t& key1, std::uint8_t& key2,
                 KLineProtocol& protocol, std::string& error, const LogFn2& log = {});

// Dienste, die VAG-Steuergeraete auf KWP2000 anbieten.
namespace service {
constexpr std::uint8_t kStartDiagnosticSession = 0x10;
constexpr std::uint8_t kClearDiagnosticInformation = 0x14;
constexpr std::uint8_t kReadDtcByStatus = 0x18;
constexpr std::uint8_t kReadEcuIdentification = 0x1A;
constexpr std::uint8_t kReadDataByLocalId = 0x21;
constexpr std::uint8_t kSecurityAccess = 0x27;
constexpr std::uint8_t kStartRoutineByLocalId = 0x31;
constexpr std::uint8_t kStopRoutineByLocalId = 0x32;
constexpr std::uint8_t kTesterPresent = 0x3E;
constexpr std::uint8_t kStopCommunication = 0x82;
constexpr std::uint8_t kNegative = 0x7F;
}  // namespace service

// Sitzungsarten. 0x89 ist die uebliche Diagnosesitzung bei VAG.
namespace session {
constexpr std::uint8_t kDefault = 0x81;
constexpr std::uint8_t kDiagnostics = 0x89;
constexpr std::uint8_t kAdaptation = 0x85;
}  // namespace session

class Kwp2000 {
public:
    explicit Kwp2000(IByteStream& stream, LogFn2 log = {});

    // Die Weckadresse und die Schluesselbytes hat der Aufrufer schon erledigt,
    // hier wird nur noch die Sitzung eroeffnet.
    bool start(Ecu ecu, std::uint8_t key1, std::uint8_t key2, std::string& error);
    void stop();
    bool started() const { return started_; }

    const std::vector<std::string>& identification() const { return identification_; }

    bool readFaults(std::vector<Kwp1281Fault>& out, std::string& error);
    bool clearFaults(std::string& error);
    bool readGroup(int group, std::vector<MeasuringValue>& out, std::string& error);
    // Grundeinstellung. Welcher Weg das Steuergeraet dafuer verlangt, ist bei
    // KWP2000 nicht einheitlich. Deshalb laesst sich die Sitzungsart vorgeben.
    bool basicSetting(int group, std::vector<MeasuringValue>& out, std::string& error,
                      std::uint8_t sessionMode = session::kAdaptation);
    // Ohne Verkehr faellt die Verbindung nach etwa fuenf Sekunden.
    bool testerPresent(std::string& error);

    // Rohdienst fuer alles, was hier nicht abgebildet ist.
    bool request(const std::vector<std::uint8_t>& payload, std::vector<std::uint8_t>& reply,
                 std::string& error);

private:
    bool sendMessage(const std::vector<std::uint8_t>& payload, std::string& error);
    bool receiveMessage(std::vector<std::uint8_t>& payload, std::string& error);
    bool readEcuIdentification();

    IByteStream& stream_;
    LogFn2 log_;
    std::vector<std::string> identification_;
    std::uint8_t target_ = 0x01;
    std::uint8_t source_ = 0xF1;
    bool started_ = false;
};

// Klartext zu einer abgelehnten Anfrage.
std::string negativeResponseText(std::uint8_t code);

// Ein Fehlereintrag aus KWP2000 belegt drei Bytes: zwei fuer die Nummer, eines
// fuer den Zustand.
Kwp1281Fault decodeKwp2000Fault(std::uint8_t high, std::uint8_t low, std::uint8_t status);

}  // namespace caruspex
