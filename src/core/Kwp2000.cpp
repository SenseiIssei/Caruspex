#include "core/Kwp2000.h"

#include <cstdio>

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
    // Das siebte Bit ist jeweils Paritaet und stoert beim Vergleich.
    const std::uint8_t k1 = key1 & 0x7F;
    const std::uint8_t k2 = key2 & 0x7F;

    // 0x01 0x0A ist die Kennung der alten Blockprotokoll-Steuergeraete.
    if (k1 == 0x01 && k2 == 0x0A) return KLineProtocol::Kwp1281;
    // 0x6B 0x0F und Verwandte stehen fuer KWP2000.
    if (k1 == 0x6B || k1 == 0x0F || k1 == 0x6D || k1 == 0x6E) {
        return KLineProtocol::Kwp2000;
    }
    // Alles mit gesetztem oberen Bit im zweiten Schluesselbyte spricht
    // erfahrungsgemaess ebenfalls KWP2000.
    if (k2 == 0x0F || k2 == 0x8F) return KLineProtocol::Kwp2000;
    return KLineProtocol::Unknown;
}

std::string protocolLabel(KLineProtocol protocol) {
    switch (protocol) {
        case KLineProtocol::Kwp1281: return "KWP1281 (Blockprotokoll)";
        case KLineProtocol::Kwp2000: return "KWP2000 (ISO 14230)";
        case KLineProtocol::Unknown: return "unbekannt";
    }
    return "unbekannt";
}

// Der Anfang ist bei beiden Protokollen gleich: Weckadresse mit 5 Baud, dann
// meldet sich das Steuergeraet mit dem Synchronbyte 0x55 und zwei
// Schluesselbytes. Der Tester antwortet mit dem invertierten zweiten
// Schluesselbyte. Erst danach unterscheiden sich die Wege.
bool wakeUpKLine(IByteStream& stream, Ecu ecu, std::uint8_t& key1, std::uint8_t& key2,
                 KLineProtocol& protocol, std::string& error, const LogFn2& log) {
    key1 = key2 = 0;
    protocol = KLineProtocol::Unknown;

    if (!stream.slowInit(static_cast<std::uint8_t>(ecu))) {
        error = "Steuergeraet " + ecuName(ecu) +
                " meldet sich nicht.\n"
                "  - Zuendung an?\n"
                "  - Hat das Fahrzeug dieses Steuergeraet ueberhaupt?\n"
                "  - Bei neueren Fahrzeugen laeuft die Diagnose ueber CAN, nicht K-Line.";
        return false;
    }

    std::uint8_t sync = 0;
    if (!stream.readByte(sync, 2000) || sync != 0x55) {
        error = "Kein Synchronbyte 0x55 vom Steuergeraet (bekommen " + hex2(sync) + ")";
        return false;
    }
    if (!stream.readByte(key1, 1000) || !stream.readByte(key2, 1000)) {
        error = "Schluesselbytes fehlen";
        return false;
    }

    protocol = protocolFromKeyBytes(key1, key2);
    if (log) {
        log("   Schluesselbytes " + hex2(key1) + " " + hex2(key2) + " -> " +
            protocolLabel(protocol));
    }

    sleepMs(30);
    const std::uint8_t answer = static_cast<std::uint8_t>(~key2);
    if (!stream.writeByte(answer)) {
        error = "Antwort auf die Schluesselbytes laesst sich nicht senden";
        return false;
    }
    if (stream.echoesOwnBytes()) {
        std::uint8_t echo = 0;
        if (!stream.readByte(echo, 600) || echo != answer) {
            error = "Kein sauberes Echo auf die Antwort";
            return false;
        }
    }

    // KWP2000-Steuergeraete schicken danach noch die invertierte Adresse.
    // Fehlt sie, ist das kein Beinbruch, deshalb nur kurz hinhoeren.
    if (protocol == KLineProtocol::Kwp2000) {
        std::uint8_t invertedAddress = 0;
        stream.readByte(invertedAddress, 400);
    }

    if (protocol == KLineProtocol::Unknown) {
        error = "Unbekannte Schluesselbytes " + hex2(key1) + " " + hex2(key2) +
                ". Das Steuergeraet spricht weder KWP1281 noch KWP2000 so, wie erwartet.";
        return false;
    }
    return true;
}

std::string negativeResponseText(std::uint8_t code) {
    switch (code) {
        case 0x10: return "Dienst abgelehnt";
        case 0x11: return "Dienst wird nicht unterstuetzt";
        case 0x12: return "Unterfunktion wird nicht unterstuetzt";
        case 0x21: return "Steuergeraet beschaeftigt, spaeter erneut versuchen";
        case 0x22: return "Bedingungen stimmen nicht (Motor laeuft? Zuendung aus?)";
        case 0x31: return "Angefragter Wert liegt ausserhalb des Bereichs";
        case 0x33: return "Zugriff verweigert, Freischaltcode noetig";
        case 0x35: return "Ungueltiger Schluessel";
        case 0x78: return "Antwort dauert laenger, Steuergeraet arbeitet noch";
        default: return "Ablehnungsgrund 0x" + hex2(code);
    }
}

Kwp1281Fault decodeKwp2000Fault(std::uint8_t high, std::uint8_t low, std::uint8_t status) {
    Kwp1281Fault fault;
    fault.code = (high << 8) | low;
    fault.status = status;
    // Bit 7 gesetzt heisst: der Fehler steht gerade an. Sonst ist er sporadisch.
    fault.sporadic = (status & 0x80) == 0;
    fault.text = describeVagFault(fault.code);
    return fault;
}

Kwp2000::Kwp2000(IByteStream& stream, LogFn2 log) : stream_(stream), log_(std::move(log)) {}

// --- Nachrichten ------------------------------------------------------------

bool Kwp2000::sendMessage(const std::vector<std::uint8_t>& payload, std::string& error) {
    if (payload.empty() || payload.size() > 63) {
        error = "Nachricht hat eine unmoegliche Laenge";
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
            error = "Senden fehlgeschlagen";
            return false;
        }
        // Eindrahtleitung: das eigene Byte kommt als Echo zurueck und muss weg.
        if (stream_.echoesOwnBytes()) {
            std::uint8_t echo = 0;
            if (!stream_.readByte(echo, kEchoTimeoutMs)) {
                error = "Kein Echo auf " + hex2(byte) + ", haengt das Kabel richtig?";
                return false;
            }
            if (echo != byte) {
                error = "Echo passt nicht: erwartet " + hex2(byte) + ", bekommen " +
                        hex2(echo);
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
        error = "Steuergeraet antwortet nicht";
        return false;
    }
    if ((format & 0xC0) != 0x80) {
        error = "Unerwartetes Formatbyte " + hex2(format);
        return false;
    }

    std::uint8_t target = 0, source = 0;
    if (!stream_.readByte(target, kByteTimeoutMs) ||
        !stream_.readByte(source, kByteTimeoutMs)) {
        error = "Adressbytes fehlen in der Antwort";
        return false;
    }

    int length = format & 0x3F;
    if (length == 0) {
        // Laenge steht in einem eigenen Byte hinter den Adressen.
        std::uint8_t extra = 0;
        if (!stream_.readByte(extra, kByteTimeoutMs)) {
            error = "Laengenbyte fehlt";
            return false;
        }
        length = extra;
    }

    std::uint8_t checksum = static_cast<std::uint8_t>(format + target + source);
    for (int i = 0; i < length; ++i) {
        std::uint8_t byte = 0;
        if (!stream_.readByte(byte, kByteTimeoutMs)) {
            error = "Antwort bricht nach " + std::to_string(i) + " von " +
                    std::to_string(length) + " Bytes ab";
            return false;
        }
        payload.push_back(byte);
        checksum = static_cast<std::uint8_t>(checksum + byte);
    }

    std::uint8_t received = 0;
    if (!stream_.readByte(received, kByteTimeoutMs)) {
        error = "Pruefsumme fehlt";
        return false;
    }
    if (received != checksum) {
        error = "Pruefsumme stimmt nicht: erwartet " + hex2(checksum) + ", bekommen " +
                hex2(received);
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

    // Das Steuergeraet darf mit 0x7F ... 0x78 um Aufschub bitten. Dann kommt
    // die eigentliche Antwort kurz darauf.
    for (int attempt = 0; attempt < 6; ++attempt) {
        if (!receiveMessage(reply, error)) return false;
        if (reply.size() >= 3 && reply[0] == service::kNegative) {
            if (reply[2] == 0x78) {
                sleepMs(120);
                continue;   // Steuergeraet arbeitet noch
            }
            error = negativeResponseText(reply[2]);
            return false;
        }
        if (!reply.empty() && reply[0] == payload[0] + 0x40) return true;
        // Etwas Unerwartetes: noch einmal hinhoeren.
    }
    error = "Steuergeraet antwortet nicht sinnvoll";
    return false;
}

// --- Sitzung ----------------------------------------------------------------

bool Kwp2000::start(Ecu ecu, std::uint8_t key1, std::uint8_t key2, std::string& error) {
    identification_.clear();
    target_ = static_cast<std::uint8_t>(ecu);
    started_ = false;

    if (protocolFromKeyBytes(key1, key2) != KLineProtocol::Kwp2000) {
        error = "Dieses Steuergeraet spricht kein KWP2000 (Schluesselbytes " + hex2(key1) +
                " " + hex2(key2) + ")";
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
    // 0x9B liefert Teilenummer und Bezeichnung als lesbaren Text.
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
        error = "Keine Sitzung offen";
        return false;
    }
    std::vector<std::uint8_t> reply;
    return request({service::kTesterPresent, 0x01}, reply, error);
}

// --- Dienste ----------------------------------------------------------------

bool Kwp2000::readFaults(std::vector<Kwp1281Fault>& out, std::string& error) {
    out.clear();
    if (!started_) {
        error = "Keine Sitzung offen";
        return false;
    }

    std::vector<std::uint8_t> reply;
    // 0xFF 0x00 heisst: alle Fehler, unabhaengig vom Zustand.
    if (!request({service::kReadDtcByStatus, 0x02, 0xFF, 0x00}, reply, error)) return false;
    if (reply.size() < 2) return true;   // Antwort ohne Eintraege

    const int count = reply[1];
    for (int i = 0; i < count; ++i) {
        const size_t at = 2 + static_cast<size_t>(i) * 3;
        if (at + 2 >= reply.size()) break;
        const Kwp1281Fault fault = decodeKwp2000Fault(reply[at], reply[at + 1], reply[at + 2]);
        if (fault.code != 0 && fault.code != 0xFFFF) out.push_back(fault);
    }
    return true;
}

bool Kwp2000::clearFaults(std::string& error) {
    if (!started_) {
        error = "Keine Sitzung offen";
        return false;
    }
    std::vector<std::uint8_t> reply;
    return request({service::kClearDiagnosticInformation, 0xFF, 0x00}, reply, error);
}

bool Kwp2000::readGroup(int group, std::vector<MeasuringValue>& out, std::string& error) {
    out.clear();
    if (!started_) {
        error = "Keine Sitzung offen";
        return false;
    }

    std::vector<std::uint8_t> reply;
    if (!request({service::kReadDataByLocalId, static_cast<std::uint8_t>(group)}, reply,
                 error)) {
        return false;
    }
    // Antwort: 0x61, Gruppennummer, danach je Anzeigefeld drei Bytes.
    for (size_t i = 2; i + 2 < reply.size(); i += 3) {
        out.push_back(decodeMeasurement(reply[i], reply[i + 1], reply[i + 2]));
    }
    return true;
}

// Grundeinstellung ueber KWP2000.
//
// Anders als bei KWP1281 gibt es dafuer keinen eigenen Blocktitel. Ueblich ist,
// erst eine andere Sitzungsart zu eroeffnen und dann denselben Messwertblock
// zu lesen: das Steuergeraet regelt dann aktiv mit. Welche Sitzungsart das
// jeweilige Steuergeraet dafuer verlangt, ist nicht einheitlich, deshalb laesst
// sie sich vorgeben.
bool Kwp2000::basicSetting(int group, std::vector<MeasuringValue>& out, std::string& error,
                           std::uint8_t sessionMode) {
    out.clear();
    if (!started_) {
        error = "Keine Sitzung offen";
        return false;
    }

    std::vector<std::uint8_t> reply;
    if (!request({service::kStartDiagnosticSession, sessionMode}, reply, error)) {
        error = "Sitzungsart 0x" + hex2(sessionMode) + " abgelehnt: " + error;
        return false;
    }
    return readGroup(group, out, error);
}

}  // namespace caruspex
