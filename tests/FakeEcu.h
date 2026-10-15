// Simulated KWP1281 control unit for the tests.
//
// Reproduces the byte-by-byte rhythm of the real protocol: everything the
// tester sends comes back as an echo (single-wire line), data bytes are
// acknowledged with their one's complement, the block end 0x03 is not. This
// way the state machine can be checked without a car and without a cable.
#pragma once

#include <cstdint>
#include <deque>
#include <string>
#include <vector>

#include "core/Kwp1281.h"

namespace caruspex::test {

class FakeEcu : public IByteStream {
public:
    explicit FakeEcu(std::vector<std::string> identification = {"1C0906032AA ", "SIMOS 1.0 "})
        : identification_(std::move(identification)) {}

    // What the simulated device returns on request.
    std::vector<std::uint8_t> faultPayload{0x11, 0x1B, 0x24, 0x00, 0x77, 0xA4};
    std::vector<std::uint8_t> groupPayload{0x01, 0x40, 0x64, 0x05, 0x28, 0x8C,
                                           0x21, 0x00, 0x50, 0x00, 0x00, 0x00};
    bool supportsBasicSetting = true;
    bool supportsAdaptation = true;
    bool supportsActuatorTest = true;
    int adaptationValue = 128;
    std::string actuatorName = "Tankentlueftungsventil N80";
    int lastAdaptationChannel = -1;
    int actuatorTestCount = 0;
    bool clearWasCalled = false;
    int lastGroupRequested = -1;
    int lastBasicSettingGroup = -1;
    int endOutputCount = 0;

    // --- IByteStream --------------------------------------------------------
    bool writeByte(std::uint8_t value) override;
    bool readByte(std::uint8_t& value, int timeoutMs) override;
    bool slowInit(std::uint8_t address) override;
    bool echoesOwnBytes() const override { return true; }

    std::uint8_t wokenAddress() const { return address_; }

private:
    void queue(std::uint8_t value) { toTester_.push_back(value); }
    void beginBlock(std::uint8_t title, const std::vector<std::uint8_t>& payload);
    void pumpOut();
    void handleCompleteBlock();

    std::vector<std::string> identification_;
    std::deque<std::uint8_t> toTester_;    // waiting to be read
    std::deque<std::uint8_t> pendingBlock_;  // rest of the block we are sending

    std::vector<std::uint8_t> incoming_;
    int expectedLength_ = -1;

    bool awaitingAck_ = false;   // we have sent and are waiting for the acknowledgement
    std::uint8_t lastSent_ = 0;
    bool handshakePending_ = false;  // next byte is the reply to KW2

    std::uint8_t address_ = 0;
    std::uint8_t counter_ = 0;
    bool awake_ = false;
    size_t identificationIndex_ = 0;
};

}  // namespace caruspex::test
