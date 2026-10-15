// KWP2000 on the K-line, VAG's successor to KWP1281.
//
// Control units switched over from about 2001. The Audi A2 sits right on the
// boundary: the 1.4 with port injection mostly still speaks KWP1281, the
// 1.6 FSI with its Bosch MED mostly already KWP2000.
//
// Unlike KWP1281 this is not a block protocol with an acknowledgement per
// byte; whole messages with header and checksum are sent instead:
//
//     [Format] [Target] [Source] [Data ...] [Checksum]
//
// The format byte carries the data length in its lower six bits.
#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "core/Kwp1281.h"   // IByteStream, Ecu, MeasuringValue

namespace caruspex {

// The two key bytes after the wake-up address tell which protocol the
// control unit speaks.
enum class KLineProtocol {
    Unknown,
    Kwp1281,
    Kwp2000,
};

KLineProtocol protocolFromKeyBytes(std::uint8_t key1, std::uint8_t key2);
std::string protocolLabel(KLineProtocol protocol);

// Wakes a control unit with the 5-baud address and reads the key bytes.
// Afterwards it is clear which protocol is used. Both protocols share this
// start, which is why it lives here in one place.
bool wakeUpKLine(IByteStream& stream, Ecu ecu, std::uint8_t& key1, std::uint8_t& key2,
                 KLineProtocol& protocol, std::string& error, const LogFn2& log = {});

// Services that VAG control units offer over KWP2000.
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

// Session types. 0x89 is the usual diagnostic session at VAG.
namespace session {
constexpr std::uint8_t kDefault = 0x81;
constexpr std::uint8_t kDiagnostics = 0x89;
constexpr std::uint8_t kAdaptation = 0x85;
}  // namespace session

class Kwp2000 {
public:
    explicit Kwp2000(IByteStream& stream, LogFn2 log = {});

    // The caller has already handled the wake-up address and key bytes;
    // this only opens the session.
    bool start(Ecu ecu, std::uint8_t key1, std::uint8_t key2, std::string& error);
    void stop();
    bool started() const { return started_; }

    const std::vector<std::string>& identification() const { return identification_; }

    bool readFaults(std::vector<Kwp1281Fault>& out, std::string& error);
    bool clearFaults(std::string& error);
    bool readGroup(int group, std::vector<MeasuringValue>& out, std::string& error);
    // Basic settings. Which route the control unit requires for this is not
    // uniform with KWP2000, so the session type can be specified.
    bool basicSetting(int group, std::vector<MeasuringValue>& out, std::string& error,
                      std::uint8_t sessionMode = session::kAdaptation);
    // Without traffic the connection drops after about five seconds.
    bool testerPresent(std::string& error);

    // Raw service for everything not covered here.
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

// Plain-text reason for a rejected request.
std::string negativeResponseText(std::uint8_t code);

// A KWP2000 fault entry takes three bytes: two for the number, one for the
// status.
// ecuAddress is the address of the reporting control unit; it decides what a
// VAG number means. -1 if unknown.
Kwp1281Fault decodeKwp2000Fault(std::uint8_t high, std::uint8_t low, std::uint8_t status,
                                int ecuAddress = -1);

}  // namespace caruspex
