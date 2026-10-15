// A single entry point to both VAG protocols on the K-line.
//
// After the wake-up address, the key bytes reveal whether the control unit
// speaks the old block protocol KWP1281 or the newer KWP2000. The Audi A2 sits
// right on this boundary: the 1.4 with port injection mostly still KWP1281,
// the 1.6 FSI with its Bosch MED mostly already KWP2000. Whoever operates the
// software should not have to know this.
#pragma once

#include <memory>
#include <string>
#include <vector>

#include "core/Kwp1281.h"
#include "core/Kwp2000.h"

namespace caruspex {

class VagKLine {
public:
    explicit VagKLine(IByteStream& stream, LogFn2 log = {});
    ~VagKLine();

    bool connect(Ecu ecu, std::string& error);
    void disconnect();
    bool connected() const;

    KLineProtocol protocol() const { return protocol_; }
    std::string protocolName() const { return protocolLabel(protocol_); }
    std::vector<std::string> identification() const;

    // Supported by both protocols.
    bool readFaults(std::vector<Kwp1281Fault>& out, std::string& error);
    bool clearFaults(std::string& error);
    bool readGroup(int group, std::vector<MeasuringValue>& out, std::string& error);
    bool basicSetting(int group, std::vector<MeasuringValue>& out, std::string& error);
    bool keepAlive(std::string& error);

    // Only KWP1281 knows these two. With KWP2000 they report that cleanly.
    bool readAdaptation(int channel, int& value, std::vector<MeasuringValue>& out,
                        std::string& error);
    bool actuatorTest(std::string& activeActuator, std::string& error);

    // KWP2000 only: session type for basic settings. Which one a given
    // control unit requires is not uniform.
    void setBasicSettingSession(std::uint8_t mode) { basicSettingSession_ = mode; }
    std::uint8_t basicSettingSession() const { return basicSettingSession_; }

private:
    IByteStream& stream_;
    LogFn2 log_;
    KLineProtocol protocol_ = KLineProtocol::Unknown;
    std::unique_ptr<Kwp1281> old_;
    std::unique_ptr<Kwp2000> modern_;
    std::uint8_t basicSettingSession_ = session::kAdaptation;
};

}  // namespace caruspex
