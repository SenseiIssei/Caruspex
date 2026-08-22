// KWP1281 ueber ein KKL-Kabel (VAG 409.1).
#pragma once

#include <string>

#include "core/Kwp1281.h"
#include "core/Serial.h"

namespace caruspex {

// Setzt die Bytestrom-Schnittstelle auf einer echten seriellen Leitung um.
// Die 5-Baud-Weckadresse wird von Hand getaktet, weil kein UART so langsam
// arbeitet: fuer jedes Bit wird die Sendeleitung 200 ms lang auf Null gezogen
// oder losgelassen.
class KLineStream : public IByteStream {
public:
    // Nach der Weckadresse laeuft die Leitung mit dieser Baudrate weiter.
    // Fast alle VAG-Steuergeraete der K-Line-Zeit nutzen 9600.
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
