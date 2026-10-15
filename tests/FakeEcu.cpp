#include "FakeEcu.h"

namespace caruspex::test {

bool FakeEcu::slowInit(std::uint8_t address) {
    address_ = address;
    awake_ = true;
    counter_ = 0;
    identificationIndex_ = 0;
    toTester_.clear();
    pendingBlock_.clear();
    incoming_.clear();
    expectedLength_ = -1;
    awaitingAck_ = false;

    // Sync byte and the two key bytes of a typical VAG engine control
    // unit.
    queue(0x55);
    queue(0x01);
    queue(0x8A);
    handshakePending_ = true;
    return true;
}

bool FakeEcu::readByte(std::uint8_t& value, int) {
    if (toTester_.empty()) return false;
    value = toTester_.front();
    toTester_.pop_front();
    return true;
}

bool FakeEcu::writeByte(std::uint8_t value) {
    if (!awake_) return false;

    // On the single-wire line the tester always hears itself.
    queue(value);

    // Reply to the key bytes: just swallow it, then the control unit
    // introduces itself with its identification blocks.
    if (handshakePending_) {
        handshakePending_ = false;
        if (identificationIndex_ < identification_.size()) {
            const std::string& text = identification_[identificationIndex_++];
            std::vector<std::uint8_t> payload(text.begin(), text.end());
            beginBlock(block::kAsciiData, payload);
        } else {
            beginBlock(block::kAck, {});
        }
        return true;
    }

    // Acknowledgement of a byte we have sent.
    if (awaitingAck_) {
        if (value != static_cast<std::uint8_t>(~lastSent_)) return false;
        awaitingAck_ = false;
        pumpOut();
        return true;
    }

    // From here on these are the tester's data bytes.
    incoming_.push_back(value);
    if (incoming_.size() == 1) {
        expectedLength_ = value;
        queue(static_cast<std::uint8_t>(~value));
        return true;
    }
    if (static_cast<int>(incoming_.size()) == expectedLength_ + 1) {
        // The block end is not acknowledged.
        handleCompleteBlock();
        incoming_.clear();
        expectedLength_ = -1;
        return true;
    }
    queue(static_cast<std::uint8_t>(~value));
    return true;
}

void FakeEcu::beginBlock(std::uint8_t title, const std::vector<std::uint8_t>& payload) {
    counter_ = static_cast<std::uint8_t>(counter_ + 1);
    pendingBlock_.clear();
    pendingBlock_.push_back(static_cast<std::uint8_t>(payload.size() + 3));
    pendingBlock_.push_back(counter_);
    pendingBlock_.push_back(title);
    for (std::uint8_t byte : payload) pendingBlock_.push_back(byte);
    pendingBlock_.push_back(0x03);
    pumpOut();
}

void FakeEcu::pumpOut() {
    if (awaitingAck_ || pendingBlock_.empty()) return;
    const std::uint8_t next = pendingBlock_.front();
    pendingBlock_.pop_front();
    queue(next);
    lastSent_ = next;
    // No acknowledgement follows the block end.
    awaitingAck_ = (next != 0x03) || !pendingBlock_.empty();
}

void FakeEcu::handleCompleteBlock() {
    if (incoming_.size() < 3) return;
    counter_ = incoming_[1];
    const std::uint8_t title = incoming_[2];
    const std::vector<std::uint8_t> payload(incoming_.begin() + 3, incoming_.end() - 1);

    switch (title) {
        case block::kAck:
            // During the introduction: next identification block, otherwise done.
            if (identificationIndex_ < identification_.size()) {
                const std::string& text = identification_[identificationIndex_++];
                beginBlock(block::kAsciiData,
                           std::vector<std::uint8_t>(text.begin(), text.end()));
            } else {
                beginBlock(block::kAck, {});
            }
            break;

        case block::kReadFaults:
            beginBlock(block::kFaultList, faultPayload);
            break;

        case block::kClearFaults:
            clearWasCalled = true;
            faultPayload.clear();
            beginBlock(block::kAck, {});
            break;

        case block::kReadGroup:
            lastGroupRequested = payload.empty() ? -1 : payload[0];
            beginBlock(block::kGroupReading, groupPayload);
            break;

        case block::kBasicSetting:
            lastBasicSettingGroup = payload.empty() ? -1 : payload[0];
            if (supportsBasicSetting) {
                beginBlock(block::kGroupReading, groupPayload);
            } else {
                beginBlock(block::kNotSupported, {});
            }
            break;

        case block::kReadAdaptation: {
            lastAdaptationChannel = payload.empty() ? -1 : payload[0];
            if (!supportsAdaptation) {
                beginBlock(block::kNotSupported, {});
                break;
            }
            std::vector<std::uint8_t> reply = groupPayload;
            reply.push_back(static_cast<std::uint8_t>((adaptationValue >> 8) & 0xFF));
            reply.push_back(static_cast<std::uint8_t>(adaptationValue & 0xFF));
            beginBlock(block::kAdaptationValue, reply);
            break;
        }

        case block::kActuatorTest:
            ++actuatorTestCount;
            if (!supportsActuatorTest) {
                beginBlock(block::kNotSupported, {});
            } else {
                beginBlock(block::kAsciiData,
                           std::vector<std::uint8_t>(actuatorName.begin(), actuatorName.end()));
            }
            break;

        case block::kEndOutput:
            ++endOutputCount;
            awake_ = false;
            break;

        default:
            beginBlock(block::kNotSupported, {});
            break;
    }
}

}  // namespace caruspex::test
