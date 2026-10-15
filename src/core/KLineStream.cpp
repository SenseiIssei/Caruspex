#include "core/KLineStream.h"

#include "core/Lang.h"
#include "core/Timing.h"

namespace caruspex {
namespace {

// 5 baud means 200 milliseconds per bit.
constexpr int kBitMs = 200;

}  // namespace

bool KLineStream::open(const std::string& device, int baudAfterInit, std::string& error) {
    baudAfterInit_ = baudAfterInit;
    buffer_.clear();
    if (!port_.open(device, baudAfterInit, error)) {
        lastError_ = error;
        return false;
    }
    return true;
}

bool KLineStream::writeByte(std::uint8_t value) {
    const char c = static_cast<char>(value);
    return port_.write(std::string(1, c));
}

bool KLineStream::readByte(std::uint8_t& value, int timeoutMs) {
    if (!buffer_.empty()) {
        value = static_cast<std::uint8_t>(buffer_.front());
        buffer_.erase(buffer_.begin());
        return true;
    }
    const std::uint64_t deadline = nowMs() + static_cast<std::uint64_t>(timeoutMs);
    while (nowMs() < deadline) {
        const std::string chunk = port_.read(20);
        if (!chunk.empty()) {
            buffer_ += chunk;
            value = static_cast<std::uint8_t>(buffer_.front());
            buffer_.erase(buffer_.begin());
            return true;
        }
    }
    return false;
}

// The wake-up address goes over the line at 5 baud, 7 data bits, odd parity
// and one stop bit. No UART can do that, so every bit is clocked by hand:
// break on means zero, break off means one.
bool KLineStream::slowInit(std::uint8_t address) {
    if (!port_.isOpen()) return false;

    port_.flushInput();
    buffer_.clear();

    // The line has to idle briefly before the wake-up.
    port_.setBreak(false);
    sleepMs(350);

    bool bits[10];
    bits[0] = false;                      // start bit
    int ones = 0;
    for (int i = 0; i < 7; ++i) {
        const bool bit = ((address >> i) & 0x01) != 0;
        bits[1 + i] = bit;
        if (bit) ++ones;
    }
    bits[8] = (ones % 2) == 0;            // odd parity
    bits[9] = true;                       // stop bit

    // Every bit gets a fixed target time, counted from the start of the
    // wake-up address. That way the operating system's overshoot does not
    // carry over from bit to bit.
    const std::uint64_t start = nowMs();
    int index = 0;
    for (bool bit : bits) {
        port_.setBreak(!bit);             // an active break pulls the line low
        ++index;
        sleepUntilMs(start + static_cast<std::uint64_t>(index) * kBitMs);
    }
    port_.setBreak(false);

    // The control unit answers with 0x55 after about 60 milliseconds.
    // Everything our own wake-up address left behind as an echo must go.
    sleepMs(30);
    port_.flushInput();
    buffer_.clear();

    if (!port_.changeBaud(baudAfterInit_)) {
        lastError_ = L("Baudrate laesst sich nach der Weckadresse nicht setzen",
                       "Cannot set the baud rate after the wake-up address");
        return false;
    }
    return true;
}

}  // namespace caruspex
