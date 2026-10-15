#include "core/Kwp2000.h"

#include <cstdio>

#include "core/Lang.h"
#include "core/Timing.h"
#include "data/VagFaults.h"

namespace caruspex {
namespace {

constexpr int kByteTimeoutMs = 1200;
constexpr int kEchoTimeoutMs = 600;
constexpr int kInterByteGapMs = 5;

std::string hex2(std::uint8_t value) {
    char buffer[8];
    std::snprintf(buffer, sizeof(buffer), "%02X", value);
    return buffer;
}

}  // namespace

KLineProtocol protocolFromKeyBytes(std::uint8_t key1, std::uint8_t key2) {
    // The seventh bit is parity in each case and gets in the way of comparing.
    const std::uint8_t k1 = key1 & 0x7F;
    const std::uint8_t k2 = key2 & 0x7F;

    // 0x01 0x0A identifies the old block-protocol control units.
    if (k1 == 0x01 && k2 == 0x0A) return KLineProtocol::Kwp1281;
    // 0x6B 0x0F and relatives stand for KWP2000.
    if (k1 == 0x6B || k1 == 0x0F || k1 == 0x6D || k1 == 0x6E) {
        return KLineProtocol::Kwp2000;
    }
    // Anything with the upper bit set in the second key byte also speaks
    // KWP2000, in our experience.
    if (k2 == 0x0F || k2 == 0x8F) return KLineProtocol::Kwp2000;
    return KLineProtocol::Unknown;
}

std::string protocolLabel(KLineProtocol protocol) {
    switch (protocol) {
        case KLineProtocol::Kwp1281: return L("KWP1281 (Blockprotokoll)", "KWP1281 (block protocol)");
        case KLineProtocol::Kwp2000: return "KWP2000 (ISO 14230)";
        case KLineProtocol::Unknown: return L("unbekannt", "unknown");
    }
    return L("unbekannt", "unknown");
}

// The start is the same for both protocols: wake-up address at 5 baud, then
// the control unit responds with the sync byte 0x55 and two key bytes. The
// tester answers with the inverted second key byte. Only after that do the
// paths diverge.
bool wakeUpKLine(IByteStream& stream, Ecu ecu, std::uint8_t& key1, std::uint8_t& key2,
                 KLineProtocol& protocol, std::string& error, const LogFn2& log) {
    key1 = key2 = 0;
    protocol = KLineProtocol::Unknown;

    if (!stream.slowInit(static_cast<std::uint8_t>(ecu))) {
        error = L("Steuergeraet ", "Control unit ") + ecuName(ecu) +
                L(" meldet sich nicht.\n"
                  "  - Zuendung an?\n"
                  "  - Hat das Fahrzeug dieses Steuergeraet ueberhaupt?\n"
                  "  - Bei neueren Fahrzeugen laeuft die Diagnose ueber CAN, nicht K-Line.",
                  " does not respond.\n"
                  "  - Ignition on?\n"
                  "  - Does the vehicle have this control unit at all?\n"
                  "  - Newer vehicles are diagnosed over CAN, not K-Line.");
        return false;
    }

    std::uint8_t sync = 0;
    if (!stream.readByte(sync, 2000) || sync != 0x55) {
        error = L("Kein Synchronbyte 0x55 vom Steuergeraet (bekommen ",
                  "No sync byte 0x55 from the control unit (got ") +
                hex2(sync) + ")";
        return false;
    }
    if (!stream.readByte(key1, 1000) || !stream.readByte(key2, 1000)) {
        error = L("Schluesselbytes fehlen", "Key bytes missing");
        return false;
    }

    protocol = protocolFromKeyBytes(key1, key2);
    if (log) {
        log(L("   Schluesselbytes ", "   Key bytes ") + hex2(key1) + " " + hex2(key2) + " -> " +
            protocolLabel(protocol));
    }

    sleepMs(30);
    const std::uint8_t answer = static_cast<std::uint8_t>(~key2);
    if (!stream.writeByte(answer)) {
        error = L("Antwort auf die Schluesselbytes laesst sich nicht senden",
                  "Cannot send the reply to the key bytes");
        return false;
    }
    if (stream.echoesOwnBytes()) {
        std::uint8_t echo = 0;
        if (!stream.readByte(echo, 600) || echo != answer) {
            error = L("Kein sauberes Echo auf die Antwort", "No clean echo of the reply");
            return false;
        }
    }

    // KWP2000 control units then also send the inverted address. If it is
    // missing that is no big deal, so only listen briefly.
    if (protocol == KLineProtocol::Kwp2000) {
        std::uint8_t invertedAddress = 0;
        stream.readByte(invertedAddress, 400);
    }

    if (protocol == KLineProtocol::Unknown) {
        error = L("Unbekannte Schluesselbytes ", "Unknown key bytes ") + hex2(key1) + " " +
                hex2(key2) +
                L(". Das Steuergeraet spricht weder KWP1281 noch KWP2000 so, wie erwartet.",
                  ". The control unit speaks neither KWP1281 nor KWP2000 as expected.");
        return false;
    }
    return true;
}

std::string negativeResponseText(std::uint8_t code) {
    switch (code) {
        case 0x10: return L("Dienst abgelehnt", "Service rejected");
        case 0x11: return L("Dienst wird nicht unterstuetzt", "Service not supported");
        case 0x12: return L("Unterfunktion wird nicht unterstuetzt", "Sub-function not supported");
        case 0x21: return L("Steuergeraet beschaeftigt, spaeter erneut versuchen",
                            "Control unit busy, try again later");
        case 0x22: return L("Bedingungen stimmen nicht (Motor laeuft? Zuendung aus?)",
                            "Conditions not correct (engine running? ignition off?)");
        case 0x31: return L("Angefragter Wert liegt ausserhalb des Bereichs",
                            "Requested value is out of range");
        case 0x33: return L("Zugriff verweigert, Freischaltcode noetig",
                            "Access denied, security code required");
        case 0x35: return L("Ungueltiger Schluessel", "Invalid key");
        case 0x78: return L("Antwort dauert laenger, Steuergeraet arbeitet noch",
                            "Response pending, control unit still working");
        default: return L("Ablehnungsgrund 0x", "Rejection reason 0x") + hex2(code);
    }
}

Kwp1281Fault decodeKwp2000Fault(std::uint8_t high, std::uint8_t low, std::uint8_t status,
                                int ecuAddress) {
    Kwp1281Fault fault;
    fault.code = (high << 8) | low;
    fault.status = status;
    // Bit 7 set means the fault is currently present. Otherwise it is intermittent.
    fault.sporadic = (status & 0x80) == 0;
    fault.text = describeVagFault(fault.code, ecuAddress);
    return fault;
}

Kwp2000::Kwp2000(IByteStream& stream, LogFn2 log) : stream_(stream), log_(std::move(log)) {}

// --- Messages --------------------------------------------------------------

bool Kwp2000::sendMessage(const std::vector<std::uint8_t>& payload, std::string& error) {
    if (payload.empty() || payload.size() > 63) {
        error = L("Nachricht hat eine unmoegliche Laenge", "Message has an impossible length");
        return false;
    }

    std::vector<std::uint8_t> frame;
    frame.push_back(static_cast<std::uint8_t>(0x80 | payload.size()));
    frame.push_back(target_);
    frame.push_back(source_);
    frame.insert(frame.end(), payload.begin(), payload.end());

    std::uint8_t checksum = 0;
    for (std::uint8_t byte : frame) checksum = static_cast<std::uint8_t>(checksum + byte);
    frame.push_back(checksum);

    if (log_) {
        std::string text;
        for (std::uint8_t byte : frame) text += hex2(byte) + " ";
        log_("   -> " + text);
    }

    for (std::uint8_t byte : frame) {
        if (!stream_.writeByte(byte)) {
            error = L("Senden fehlgeschlagen", "Send failed");
            return false;
        }
        // Single-wire line: our own byte comes back as an echo and must be removed.
        if (stream_.echoesOwnBytes()) {
            std::uint8_t echo = 0;
            if (!stream_.readByte(echo, kEchoTimeoutMs)) {
                error = L("Kein Echo auf ", "No echo for ") + hex2(byte) +
                        L(", haengt das Kabel richtig?", ", is the cable connected properly?");
                return false;
            }
            if (echo != byte) {
                error = L("Echo passt nicht: erwartet ", "Echo mismatch: expected ") + hex2(byte) +
                        L(", bekommen ", ", got ") + hex2(echo);
                return false;
            }
        }
        sleepMs(kInterByteGapMs);
    }
    return true;
}

bool Kwp2000::receiveMessage(std::vector<std::uint8_t>& payload, std::string& error) {
    payload.clear();

    std::uint8_t format = 0;
    if (!stream_.readByte(format, kByteTimeoutMs)) {
        error = L("Steuergeraet antwortet nicht", "Control unit does not respond");
        return false;
    }
    if ((format & 0xC0) != 0x80) {
        error = L("Unerwartetes Formatbyte ", "Unexpected format byte ") + hex2(format);
        return false;
    }

    std::uint8_t target = 0, source = 0;
    if (!stream_.readByte(target, kByteTimeoutMs) ||
        !stream_.readByte(source, kByteTimeoutMs)) {
        error = L("Adressbytes fehlen in der Antwort", "Address bytes missing in the response");
        return false;
    }

    int length = format & 0x3F;
    if (length == 0) {
        // The length is in a separate byte after the addresses.
        std::uint8_t extra = 0;
        if (!stream_.readByte(extra, kByteTimeoutMs)) {
            error = L("Laengenbyte fehlt", "Length byte missing");
            return false;
        }
        length = extra;
    }

    std::uint8_t checksum = static_cast<std::uint8_t>(format + target + source);
    for (int i = 0; i < length; ++i) {
        std::uint8_t byte = 0;
        if (!stream_.readByte(byte, kByteTimeoutMs)) {
            error = L("Antwort bricht nach ", "Response breaks off after ") + std::to_string(i) +
                    L(" von ", " of ") + std::to_string(length) + L(" Bytes ab", " bytes");
            return false;
        }
        payload.push_back(byte);
        checksum = static_cast<std::uint8_t>(checksum + byte);
    }

    std::uint8_t received = 0;
    if (!stream_.readByte(received, kByteTimeoutMs)) {
        error = L("Pruefsumme fehlt", "Checksum missing");
        return false;
    }
    if (received != checksum) {
        error = L("Pruefsumme stimmt nicht: erwartet ", "Checksum mismatch: expected ") +
                hex2(checksum) + L(", bekommen ", ", got ") + hex2(received);
        return false;
    }

    if (log_) {
        std::string text;
        for (std::uint8_t byte : payload) text += hex2(byte) + " ";
        log_("   <- " + text);
    }
    return true;
}

bool Kwp2000::request(const std::vector<std::uint8_t>& payload,
                      std::vector<std::uint8_t>& reply, std::string& error) {
    if (!sendMessage(payload, error)) return false;

    // The control unit may ask for a delay with 0x7F ... 0x78. The actual
    // response then follows shortly afterwards.
    for (int attempt = 0; attempt < 6; ++attempt) {
        if (!receiveMessage(reply, error)) return false;
        if (reply.size() >= 3 && reply[0] == service::kNegative) {
            if (reply[2] == 0x78) {
                sleepMs(120);
                continue;   // control unit still working
            }
            error = negativeResponseText(reply[2]);
            return false;
        }
        if (!reply.empty() && reply[0] == payload[0] + 0x40) return true;
        // Something unexpected: listen once more.
    }
    error = L("Steuergeraet antwortet nicht sinnvoll", "Control unit does not respond meaningfully");
    return false;
}

// --- Session ----------------------------------------------------------------

bool Kwp2000::start(Ecu ecu, std::uint8_t key1, std::uint8_t key2, std::string& error) {
    identification_.clear();
    target_ = static_cast<std::uint8_t>(ecu);
    started_ = false;

    if (protocolFromKeyBytes(key1, key2) != KLineProtocol::Kwp2000) {
        error = L("Dieses Steuergeraet spricht kein KWP2000 (Schluesselbytes ",
                  "This control unit does not speak KWP2000 (key bytes ") +
                hex2(key1) + " " + hex2(key2) + ")";
        return false;
    }

    std::vector<std::uint8_t> reply;
    if (!request({service::kStartDiagnosticSession, session::kDiagnostics}, reply, error)) {
        return false;
    }
    started_ = true;
    readEcuIdentification();
    return true;
}

void Kwp2000::stop() {
    if (!started_) return;
    std::string ignored;
    std::vector<std::uint8_t> reply;
    sendMessage({service::kStopCommunication}, ignored);
    receiveMessage(reply, ignored);
    started_ = false;
}

bool Kwp2000::readEcuIdentification() {
    std::vector<std::uint8_t> reply;
    std::string ignored;
    // 0x9B returns part number and description as readable text.
    if (!request({service::kReadEcuIdentification, 0x9B}, reply, ignored)) return false;

    std::string text;
    for (size_t i = 1; i < reply.size(); ++i) {
        const char c = static_cast<char>(reply[i] & 0x7F);
        if (c >= 0x20 && c < 0x7F) text += c;
    }
    if (!text.empty()) identification_.push_back(text);
    return true;
}

bool Kwp2000::testerPresent(std::string& error) {
    if (!started_) {
        error = L("Keine Sitzung offen", "No session open");
        return false;
    }
    std::vector<std::uint8_t> reply;
    return request({service::kTesterPresent, 0x01}, reply, error);
}

// --- Services ---------------------------------------------------------------

bool Kwp2000::readFaults(std::vector<Kwp1281Fault>& out, std::string& error) {
    out.clear();
    if (!started_) {
        error = L("Keine Sitzung offen", "No session open");
        return false;
    }

    std::vector<std::uint8_t> reply;
    // 0xFF 0x00 means: all faults, regardless of status.
    if (!request({service::kReadDtcByStatus, 0x02, 0xFF, 0x00}, reply, error)) return false;
    if (reply.size() < 2) return true;   // response without entries

    const int count = reply[1];
    for (int i = 0; i < count; ++i) {
        const size_t at = 2 + static_cast<size_t>(i) * 3;
        if (at + 2 >= reply.size()) break;
        const Kwp1281Fault fault = decodeKwp2000Fault(reply[at], reply[at + 1], reply[at + 2],
                                                      static_cast<int>(target_));
        if (fault.code != 0 && fault.code != 0xFFFF) out.push_back(fault);
    }
    return true;
}

bool Kwp2000::clearFaults(std::string& error) {
    if (!started_) {
        error = L("Keine Sitzung offen", "No session open");
        return false;
    }
    std::vector<std::uint8_t> reply;
    return request({service::kClearDiagnosticInformation, 0xFF, 0x00}, reply, error);
}

bool Kwp2000::readGroup(int group, std::vector<MeasuringValue>& out, std::string& error) {
    out.clear();
    if (!started_) {
        error = L("Keine Sitzung offen", "No session open");
        return false;
    }

    std::vector<std::uint8_t> reply;
    if (!request({service::kReadDataByLocalId, static_cast<std::uint8_t>(group)}, reply,
                 error)) {
        return false;
    }
    // Response: 0x61, group number, then three bytes per display field.
    for (size_t i = 2; i + 2 < reply.size(); i += 3) {
        out.push_back(decodeMeasurement(reply[i], reply[i + 1], reply[i + 2]));
    }
    return true;
}

// Basic settings over KWP2000.
//
// Unlike KWP1281 there is no dedicated block title for this. The usual way is
// to open a different session type first and then read the same measuring
// block: the control unit then actively regulates. Which session type a
// given control unit requires is not uniform, so it can be specified.
bool Kwp2000::basicSetting(int group, std::vector<MeasuringValue>& out, std::string& error,
                           std::uint8_t sessionMode) {
    out.clear();
    if (!started_) {
        error = L("Keine Sitzung offen", "No session open");
        return false;
    }

    std::vector<std::uint8_t> reply;
    if (!request({service::kStartDiagnosticSession, sessionMode}, reply, error)) {
        error = L("Sitzungsart 0x", "Session type 0x") + hex2(sessionMode) +
                L(" abgelehnt: ", " rejected: ") + error;
        return false;
    }
    return readGroup(group, out, error);
}

}  // namespace caruspex
