// KWP1281 over a KKL cable (VAG 409.1).
#pragma once

#include <string>

#include "core/Kwp1281.h"
#include "core/Serial.h"

namespace caruspex {

// Implements the byte stream interface on a real serial line.
// The 5-baud wake-up address is clocked out by hand because no UART runs that
// slowly: for each bit the transmit line is pulled low or released for
// 200 ms.
class KLineStream : public IByteStream {
public:
    // After the wake-up address the line continues at this baud rate.
    // Almost all VAG control units of the K-line era use 9600.
    bool open(const std::string& device, int baudAfterInit, std::string& error);
    void close() { port_.close(); }
    bool isOpen() const { return port_.isOpen(); }

    bool writeByte(std::uint8_t value) override;
    bool readByte(std::uint8_t& value, int timeoutMs) override;
    bool slowInit(std::uint8_t address) override;
    bool echoesOwnBytes() const override { return true; }

    const std::string& lastError() const { return lastError_; }

private:
    Serial port_;
    int baudAfterInit_ = 9600;
    std::string buffer_;
    std::string lastError_;
};

}  // namespace caruspex
