#include "FakeEcu2000.h"

namespace caruspex::test {

bool FakeEcu2000::slowInit(std::uint8_t address) {
    address_ = address;
    awake_ = true;
    stopped = false;
    toTester_.clear();
    incoming_.clear();
    expectedLength_ = -1;

    // Synchronbyte und Schluesselbytes eines KWP2000-Steuergeraets.
    queue(0x55);
    queue(0xEF);   // 0x6F mit Paritaetsbit, unten maskiert zu 0x6F
    queue(0x8F);
    handshakePending_ = true;
    return true;
}

bool FakeEcu2000::readByte(std::uint8_t& value, int) {
    if (toTester_.empty()) return false;
    value = toTester_.front();
    toTester_.pop_front();
    return true;
}

bool FakeEcu2000::writeByte(std::uint8_t value) {
    if (!awake_) return false;

    // Eindrahtleitung: der Tester hoert sich selbst.
    queue(value);

    if (handshakePending_) {
        handshakePending_ = false;
        // Nach der Antwort auf die Schluesselbytes kommt die invertierte Adresse.
        queue(static_cast<std::uint8_t>(~address_));
        return true;
    }

    incoming_.push_back(value);
    if (incoming_.size() == 1) {
        // Formatbyte: untere sechs Bit sind die Nutzlaenge.
        expectedLength_ = value & 0x3F;
        return true;
    }
    // Kopf ist drei Bytes, dazu die Nutzlast und ein Byte Pruefsumme.
    const size_t complete = 3 + static_cast<size_t>(expectedLength_) + 1;
    if (incoming_.size() < complete) return true;

    const std::vector<std::uint8_t> payload(incoming_.begin() + 3, incoming_.end() - 1);
    incoming_.clear();
    expectedLength_ = -1;
    handleRequest(payload);
    return true;
}

void FakeEcu2000::reply(const std::vector<std::uint8_t>& payload) {
    std::vector<std::uint8_t> frame;
    frame.push_back(static_cast<std::uint8_t>(0x80 | payload.size()));
    frame.push_back(0xF1);       // Ziel ist der Tester
    frame.push_back(address_);   // Quelle ist das Steuergeraet
    frame.insert(frame.end(), payload.begin(), payload.end());
    std::uint8_t checksum = 0;
    for (std::uint8_t byte : frame) checksum = static_cast<std::uint8_t>(checksum + byte);
    frame.push_back(checksum);
    for (std::uint8_t byte : frame) queue(byte);
}

void FakeEcu2000::handleRequest(const std::vector<std::uint8_t>& payload) {
    if (payload.empty()) return;
    const std::uint8_t sid = payload[0];

    switch (sid) {
        case service::kStartDiagnosticSession: {
            const int mode = payload.size() > 1 ? payload[1] : -1;
            if (mode == refuseSession) {
                reply({service::kNegative, sid, 0x22});
                return;
            }
            lastSession = mode;
            ++sessionCount;
            reply({static_cast<std::uint8_t>(sid + 0x40),
                   static_cast<std::uint8_t>(mode < 0 ? 0 : mode)});
            return;
        }

        case service::kReadEcuIdentification: {
            std::vector<std::uint8_t> out{static_cast<std::uint8_t>(sid + 0x40), 0x9B};
            for (char c : identification) out.push_back(static_cast<std::uint8_t>(c));
            reply(out);
            return;
        }

        case service::kReadDtcByStatus: {
            std::vector<std::uint8_t> out{static_cast<std::uint8_t>(sid + 0x40)};
            out.push_back(static_cast<std::uint8_t>(faults.size() / 3));
            out.insert(out.end(), faults.begin(), faults.end());
            reply(out);
            return;
        }

        case service::kClearDiagnosticInformation:
            clearWasCalled = true;
            faults.clear();
            reply({static_cast<std::uint8_t>(sid + 0x40)});
            return;

        case service::kReadDataByLocalId: {
            lastGroup = payload.size() > 1 ? payload[1] : -1;
            std::vector<std::uint8_t> out{static_cast<std::uint8_t>(sid + 0x40),
                                          static_cast<std::uint8_t>(lastGroup)};
            out.insert(out.end(), groupPayload.begin(), groupPayload.end());
            reply(out);
            return;
        }

        case service::kTesterPresent:
            reply({static_cast<std::uint8_t>(sid + 0x40)});
            return;

        case service::kStopCommunication:
            stopped = true;
            reply({0xC2});
            return;

        default:
            reply({service::kNegative, sid, 0x11});
            return;
    }
}

}  // namespace caruspex::test
