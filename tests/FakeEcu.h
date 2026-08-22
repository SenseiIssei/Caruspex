// Nachgebautes KWP1281-Steuergeraet fuer die Tests.
//
// Spielt den byteweisen Takt des echten Protokolls nach: alles, was der Tester
// sendet, kommt als Echo zurueck (Eindrahtleitung), Datenbytes werden mit dem
// Einerkomplement quittiert, das Blockende 0x03 nicht. Damit laesst sich die
// Zustandsmaschine ohne Auto und ohne Kabel pruefen.
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

    // Was das nachgebaute Geraet auf Anfrage liefert.
    std::vector<std::uint8_t> faultPayload{0x11, 0x1B, 0x24, 0x00, 0x77, 0xA4};
    std::vector<std::uint8_t> groupPayload{0x01, 0x40, 0x64, 0x05, 0x28, 0x8C,
                                           0x21, 0x00, 0x50, 0x00, 0x00, 0x00};
    bool supportsBasicSetting = true;
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
    std::deque<std::uint8_t> toTester_;    // wartet darauf, gelesen zu werden
    std::deque<std::uint8_t> pendingBlock_;  // Rest des Blocks, den wir senden

    std::vector<std::uint8_t> incoming_;
    int expectedLength_ = -1;

    bool awaitingAck_ = false;   // wir haben gesendet und warten auf die Quittung
    std::uint8_t lastSent_ = 0;
    bool handshakePending_ = false;  // naechstes Byte ist die Antwort auf KW2

    std::uint8_t address_ = 0;
    std::uint8_t counter_ = 0;
    bool awake_ = false;
    size_t identificationIndex_ = 0;
};

}  // namespace caruspex::test
