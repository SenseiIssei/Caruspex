#include "core/KLineStream.h"

#include "core/Timing.h"

namespace caruspex {
namespace {

// 5 Baud heisst 200 Millisekunden pro Bit.
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

// Die Weckadresse geht mit 5 Baud, 7 Datenbits, ungerader Paritaet und einem
// Stoppbit ueber die Leitung. Kein UART kann das, also wird jedes Bit von Hand
// getaktet: Break an heisst Null, Break aus heisst Eins.
bool KLineStream::slowInit(std::uint8_t address) {
    if (!port_.isOpen()) return false;

    port_.flushInput();
    buffer_.clear();

    // Die Leitung muss vor dem Wecken kurz ruhen.
    port_.setBreak(false);
    sleepMs(350);

    bool bits[10];
    bits[0] = false;                      // Startbit
    int ones = 0;
    for (int i = 0; i < 7; ++i) {
        const bool bit = ((address >> i) & 0x01) != 0;
        bits[1 + i] = bit;
        if (bit) ++ones;
    }
    bits[8] = (ones % 2) == 0;            // ungerade Paritaet
    bits[9] = true;                       // Stoppbit

    for (bool bit : bits) {
        port_.setBreak(!bit);             // Break aktiv zieht die Leitung auf Null
        sleepMs(kBitMs);
    }
    port_.setBreak(false);

    // Das Steuergeraet antwortet nach etwa 60 Millisekunden mit 0x55.
    // Alles, was die eigene Weckadresse als Echo hinterlassen hat, muss weg.
    sleepMs(30);
    port_.flushInput();
    buffer_.clear();

    if (!port_.changeBaud(baudAfterInit_)) {
        lastError_ = "Baudrate laesst sich nach der Weckadresse nicht setzen";
        return false;
    }
    return true;
}

}  // namespace caruspex
