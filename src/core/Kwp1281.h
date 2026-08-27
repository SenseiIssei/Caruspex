// KWP1281, das VAG-eigene Protokoll der Jahre 1995 bis etwa 2005.
//
// Anders als EOBD kommt man damit an jedes Steuergeraet und an die Dienste,
// die keine Abgasrelevanz haben: Messwertbloecke, Anpassungskanaele,
// Stellglieddiagnose und die Grundeinstellung, mit der die Drosselklappe
// angelernt wird.
//
// Warum das kein ELM327 kann: KWP1281 ist ein Blockprotokoll auf einer
// Eindrahtleitung. Jedes einzelne Byte wird von der Gegenstelle mit seinem
// Einerkomplement quittiert, bevor das naechste kommt. Die ELM327-Firmware
// kennt nur ganze Anfragen und Antworten und kann diesen Takt nicht abbilden.
// Es braucht ein KKL-Kabel, das die K-Leitung roh durchreicht.
#pragma once

#include <cstdint>
#include <functional>
#include <string>
#include <vector>

namespace caruspex {

using LogFn2 = std::function<void(const std::string&)>;

// Steuergeraeteadressen, wie sie auch VCDS benutzt.
enum class Ecu : std::uint8_t {
    Engine = 0x01,
    Gearbox = 0x02,
    Abs = 0x03,
    Airbag = 0x15,
    InstrumentCluster = 0x17,
    CentralElectrics = 0x09,
    ComfortSystem = 0x46,
    Immobilizer = 0x25,
    AirConditioning = 0x08,
    SteeringWheel = 0x16,
    Radio = 0x56,
};

std::string ecuName(Ecu ecu);
const std::vector<Ecu>& knownEcus();

// Blockkennungen des Protokolls.
namespace block {
constexpr std::uint8_t kClearFaults = 0x05;
constexpr std::uint8_t kEndOutput = 0x06;
constexpr std::uint8_t kReadFaults = 0x07;
constexpr std::uint8_t kAck = 0x09;
constexpr std::uint8_t kActuatorTest = 0x04;
constexpr std::uint8_t kBasicSetting = 0x28;   // Grundeinstellung
constexpr std::uint8_t kReadGroup = 0x29;      // Messwertblock lesen
constexpr std::uint8_t kReadAdaptation = 0x2A;
constexpr std::uint8_t kTestAdaptation = 0x2B;
constexpr std::uint8_t kSaveAdaptation = 0x2C;
constexpr std::uint8_t kLogin = 0x2B;
constexpr std::uint8_t kAsciiData = 0xF6;
constexpr std::uint8_t kFaultList = 0xFC;
constexpr std::uint8_t kGroupReading = 0xE7;
constexpr std::uint8_t kAdaptationValue = 0xE6;
constexpr std::uint8_t kNotSupported = 0x0A;
}  // namespace block

// Ein Byte hin, ein Byte her. Getrennt vom Kabel, damit die Zustandsmaschine
// gegen ein nachgebautes Steuergeraet getestet werden kann.
class IByteStream {
public:
    virtual ~IByteStream() = default;
    virtual bool writeByte(std::uint8_t value) = 0;
    // Liefert false bei Zeitueberschreitung.
    virtual bool readByte(std::uint8_t& value, int timeoutMs) = 0;
    // Die 5-Baud-Weckadresse. Danach muss der Bus auf 9600 Baud stehen.
    virtual bool slowInit(std::uint8_t address) = 0;
    // K-Leitung ist Eindraht: alles Gesendete kommt als Echo zurueck.
    virtual bool echoesOwnBytes() const { return true; }
};

struct Kwp1281Fault {
    int code = 0;              // VAG-eigene fuenfstellige Nummer
    std::uint8_t status = 0;
    std::string text;          // Klartext, falls hinterlegt
    bool sporadic = false;
};

struct MeasuringValue {
    std::string label;         // "Drehzahl"
    double value = 0.0;
    std::string unit;
    bool valid = false;
};

class Kwp1281 {
public:
    explicit Kwp1281(IByteStream& stream, LogFn2 log = {});

    // Weckt das Steuergeraet und liest die Kennung aus.
    bool connect(Ecu ecu, std::string& error);
    // Wie connect, aber das Wecken hat der Aufrufer schon erledigt. Wird
    // gebraucht, wenn erst nach den Schluesselbytes feststeht, welches
    // Protokoll das Steuergeraet spricht.
    bool attach(std::string& error);
    // Adresse des Steuergeraets, damit Fehlernummern richtig gedeutet
    // werden. Bei attach() muss der Aufrufer sie selbst setzen.
    void setEcu(Ecu ecu) { ecu_ = ecu; }
    void disconnect();
    bool connected() const { return connected_; }

    const std::vector<std::string>& identification() const { return identification_; }

    bool readFaults(std::vector<Kwp1281Fault>& out, std::string& error);
    // Anpassungskanal lesen: liefert den eingestellten Wert und die
    // Messwertanzeige, die das Steuergeraet dazu ausgibt.
    bool readAdaptation(int channel, int& value, std::vector<MeasuringValue>& out,
                        std::string& error);
    // Stellglieddiagnose: das Steuergeraet klappert der Reihe nach seine
    // Aktoren durch und nennt jeweils den, der gerade laeuft.
    bool actuatorTest(std::string& activeActuator, std::string& error);
    bool clearFaults(std::string& error);
    // Messwertblock lesen, liefert bis zu vier Werte.
    bool readGroup(int group, std::vector<MeasuringValue>& out, std::string& error);
    // Grundeinstellung: derselbe Block, aber das Steuergeraet regelt aktiv mit.
    bool basicSetting(int group, std::vector<MeasuringValue>& out, std::string& error);
    // Haelt die Verbindung am Leben, wenn gerade nichts angefragt wird.
    bool keepAlive(std::string& error);

    std::uint8_t blockCounter() const { return counter_; }

private:
    bool sendBlock(std::uint8_t title, const std::vector<std::uint8_t>& payload,
                   std::string& error);
    bool receiveBlock(std::uint8_t& title, std::vector<std::uint8_t>& payload,
                      std::string& error);
    bool sendByteAcked(std::uint8_t value, std::string& error);
    bool receiveByteAcked(std::uint8_t& value, bool sendAck, std::string& error);
    bool readEcho(std::uint8_t expected, std::string& error);
    bool drainIdentification(std::string& error);

    IByteStream& stream_;
    LogFn2 log_;
    std::vector<std::string> identification_;
    Ecu ecu_ = Ecu::Engine;
    std::uint8_t counter_ = 0;
    bool connected_ = false;
};

// Rohbytes eines Messwertblocks in Zahl und Einheit uebersetzen.
MeasuringValue decodeMeasurement(std::uint8_t formula, std::uint8_t a, std::uint8_t b);

}  // namespace caruspex
