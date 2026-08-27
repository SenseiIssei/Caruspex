// Nachgebautes KWP2000-Steuergeraet fuer die Tests.
//
// Antwortet mit Schluesselbytes, die auf KWP2000 zeigen, und beantwortet
// Nachrichten mit Kopf und Pruefsumme, so wie es ein Bosch MED im Audi A2
// 1.6 FSI tut.
#pragma once

#include <cstdint>
#include <deque>
#include <string>
#include <vector>

#include "core/Kwp2000.h"

namespace caruspex::test {

class FakeEcu2000 : public IByteStream {
public:
    // Fehler als Dreiergruppen: zwei Bytes Nummer, ein Byte Zustand.
    std::vector<std::uint8_t> faults{0x40, 0x66, 0xE0, 0x02, 0x31, 0x60};
    std::vector<std::uint8_t> groupPayload{0x01, 0x40, 0x64, 0x05, 0x28, 0x8C,
                                           0x21, 0x00, 0x50, 0x00, 0x00, 0x00};
    std::string identification = "8E0906018AK  1.6l MED 7.5.11";
    bool clearWasCalled = false;
    int lastGroup = -1;
    int lastSession = -1;
    int sessionCount = 0;
    bool stopped = false;
    // Wenn gesetzt, lehnt das Steuergeraet diese Sitzungsart ab.
    int refuseSession = -1;

    bool writeByte(std::uint8_t value) override;
    bool readByte(std::uint8_t& value, int timeoutMs) override;
    bool slowInit(std::uint8_t address) override;
    bool echoesOwnBytes() const override { return true; }

    std::uint8_t wokenAddress() const { return address_; }

private:
    void queue(std::uint8_t value) { toTester_.push_back(value); }
    void reply(const std::vector<std::uint8_t>& payload);
    void handleRequest(const std::vector<std::uint8_t>& payload);

    std::deque<std::uint8_t> toTester_;
    std::vector<std::uint8_t> incoming_;
    int expectedLength_ = -1;
    bool awake_ = false;
    bool handshakePending_ = false;
    std::uint8_t address_ = 0;
};

}  // namespace caruspex::test
