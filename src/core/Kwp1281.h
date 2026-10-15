// KWP1281, VAG's own protocol from roughly 1995 to 2005.
//
// Unlike EOBD it reaches every control unit and the services that are not
// emissions-related: measuring blocks, adaptation channels, output tests and
// the basic settings used for throttle body adaptation.
//
// Why an ELM327 cannot do this: KWP1281 is a block protocol on a single-wire
// line. The other side acknowledges every single byte with its one's
// complement before the next one is sent. The ELM327 firmware only knows
// complete requests and responses and cannot reproduce this byte rhythm.
// It takes a KKL cable that passes the K-line through raw.
#pragma once

#include <cstdint>
#include <functional>
#include <string>
#include <vector>

namespace caruspex {

using LogFn2 = std::function<void(const std::string&)>;

// Control unit addresses, the same ones VCDS uses.
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

// Block titles of the protocol.
namespace block {
constexpr std::uint8_t kClearFaults = 0x05;
constexpr std::uint8_t kEndOutput = 0x06;
constexpr std::uint8_t kReadFaults = 0x07;
constexpr std::uint8_t kAck = 0x09;
constexpr std::uint8_t kActuatorTest = 0x04;
constexpr std::uint8_t kBasicSetting = 0x28;   // basic settings
constexpr std::uint8_t kReadGroup = 0x29;      // read measuring block
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

// One byte out, one byte in. Kept separate from the cable so the state
// machine can be tested against a simulated control unit.
class IByteStream {
public:
    virtual ~IByteStream() = default;
    virtual bool writeByte(std::uint8_t value) = 0;
    // Returns false on timeout.
    virtual bool readByte(std::uint8_t& value, int timeoutMs) = 0;
    // The 5-baud wake-up address. Afterwards the bus must run at 9600 baud.
    virtual bool slowInit(std::uint8_t address) = 0;
    // The K-line is single-wire: everything sent comes back as an echo.
    virtual bool echoesOwnBytes() const { return true; }
};

struct Kwp1281Fault {
    int code = 0;              // VAG's own five-digit number
    std::uint8_t status = 0;
    std::string text;          // plain text, if known
    bool sporadic = false;
};

struct MeasuringValue {
    std::string label;         // "Engine speed"
    double value = 0.0;
    std::string unit;
    bool valid = false;
};

class Kwp1281 {
public:
    explicit Kwp1281(IByteStream& stream, LogFn2 log = {});

    // Wakes the control unit and reads its identification.
    bool connect(Ecu ecu, std::string& error);
    // Like connect, but the caller has already done the wake-up. Needed when
    // only the key bytes reveal which protocol the control unit speaks.
    bool attach(std::string& error);
    // Address of the control unit, so fault numbers are interpreted
    // correctly. With attach() the caller has to set it.
    void setEcu(Ecu ecu) { ecu_ = ecu; }
    void disconnect();
    bool connected() const { return connected_; }

    const std::vector<std::string>& identification() const { return identification_; }

    bool readFaults(std::vector<Kwp1281Fault>& out, std::string& error);
    // Read an adaptation channel: returns the stored value and the live
    // values the control unit displays alongside it.
    bool readAdaptation(int channel, int& value, std::vector<MeasuringValue>& out,
                        std::string& error);
    // Output test: the control unit cycles through its actuators one by one
    // and reports the one that is currently active.
    bool actuatorTest(std::string& activeActuator, std::string& error);
    bool clearFaults(std::string& error);
    // Read a measuring block, returns up to four values.
    bool readGroup(int group, std::vector<MeasuringValue>& out, std::string& error);
    // Basic settings: the same block, but the control unit actively regulates.
    bool basicSetting(int group, std::vector<MeasuringValue>& out, std::string& error);
    // Keeps the connection alive while nothing is being requested.
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

// Translate the raw bytes of a measuring block into a number and unit.
MeasuringValue decodeMeasurement(std::uint8_t formula, std::uint8_t a, std::uint8_t b);

}  // namespace caruspex
