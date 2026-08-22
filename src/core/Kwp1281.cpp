#include "core/Kwp1281.h"

#include <cmath>
#include <cstdio>

#include "core/Timing.h"

namespace caruspex {
namespace {

constexpr int kByteTimeoutMs = 1200;
constexpr int kAckTimeoutMs = 500;
constexpr std::uint8_t kBlockEnd = 0x03;

std::string hex(std::uint8_t value) {
    char buffer[8];
    std::snprintf(buffer, sizeof(buffer), "%02X", value);
    return buffer;
}

}  // namespace

std::string ecuName(Ecu ecu) {
    switch (ecu) {
        case Ecu::Engine: return "01 Motor";
        case Ecu::Gearbox: return "02 Getriebe";
        case Ecu::Abs: return "03 Bremsenelektronik (ABS)";
        case Ecu::AirConditioning: return "08 Klimaanlage";
        case Ecu::CentralElectrics: return "09 Zentralelektrik";
        case Ecu::Airbag: return "15 Airbag";
        case Ecu::SteeringWheel: return "16 Lenkradelektronik";
        case Ecu::InstrumentCluster: return "17 Kombiinstrument";
        case Ecu::Immobilizer: return "25 Wegfahrsperre";
        case Ecu::ComfortSystem: return "46 Komfortsystem";
        case Ecu::Radio: return "56 Radio";
    }
    return "unbekannt";
}

const std::vector<Ecu>& knownEcus() {
    static const std::vector<Ecu> list = {
        Ecu::Engine,   Ecu::Gearbox,           Ecu::Abs,        Ecu::AirConditioning,
        Ecu::CentralElectrics, Ecu::Airbag,    Ecu::SteeringWheel,
        Ecu::InstrumentCluster, Ecu::Immobilizer, Ecu::ComfortSystem, Ecu::Radio};
    return list;
}

Kwp1281::Kwp1281(IByteStream& stream, LogFn2 log) : stream_(stream), log_(std::move(log)) {}

// --- Byteweiser Takt --------------------------------------------------------

// Auf der Eindrahtleitung hoert man sich selbst. Das Echo muss weg, sonst
// haelt man es fuer die Antwort der Gegenstelle.
bool Kwp1281::readEcho(std::uint8_t expected, std::string& error) {
    if (!stream_.echoesOwnBytes()) return true;
    std::uint8_t echo = 0;
    if (!stream_.readByte(echo, kAckTimeoutMs)) {
        error = "Kein Echo auf " + hex(expected) + ", haengt das Kabel richtig?";
        return false;
    }
    if (echo != expected) {
        error = "Echo passt nicht: erwartet " + hex(expected) + ", bekommen " + hex(echo);
        return false;
    }
    return true;
}

// Byte senden, Echo abraeumen, dann auf das Komplement der Gegenstelle warten.
bool Kwp1281::sendByteAcked(std::uint8_t value, std::string& error) {
    sleepMs(2);   // das Steuergeraet braucht eine kurze Pause zwischen den Bytes
    if (!stream_.writeByte(value)) {
        error = "Senden fehlgeschlagen";
        return false;
    }
    if (!readEcho(value, error)) return false;

    std::uint8_t ack = 0;
    if (!stream_.readByte(ack, kByteTimeoutMs)) {
        error = "Steuergeraet quittiert " + hex(value) + " nicht";
        return false;
    }
    const std::uint8_t expected = static_cast<std::uint8_t>(~value);
    if (ack != expected) {
        error = "Falsche Quittung: erwartet " + hex(expected) + ", bekommen " + hex(ack);
        return false;
    }
    return true;
}

// Byte empfangen und mit dem Komplement quittieren.
bool Kwp1281::receiveByteAcked(std::uint8_t& value, bool sendAck, std::string& error) {
    if (!stream_.readByte(value, kByteTimeoutMs)) {
        error = "Steuergeraet antwortet nicht mehr";
        return false;
    }
    if (!sendAck) return true;

    const std::uint8_t ack = static_cast<std::uint8_t>(~value);
    sleepMs(2);
    if (!stream_.writeByte(ack)) {
        error = "Quittung laesst sich nicht senden";
        return false;
    }
    return readEcho(ack, error);
}

// --- Bloecke ----------------------------------------------------------------

// Aufbau: Laenge, Zaehler, Titel, Nutzdaten, 0x03. Die Laenge zaehlt alles
// ausser sich selbst. Das abschliessende 0x03 wird nicht quittiert.
bool Kwp1281::sendBlock(std::uint8_t title, const std::vector<std::uint8_t>& payload,
                        std::string& error) {
    const std::uint8_t length = static_cast<std::uint8_t>(payload.size() + 3);
    counter_ = static_cast<std::uint8_t>(counter_ + 1);

    if (log_) log_("   -> Block " + hex(title) + " (Zaehler " + hex(counter_) + ")");

    if (!sendByteAcked(length, error)) return false;
    if (!sendByteAcked(counter_, error)) return false;
    if (!sendByteAcked(title, error)) return false;
    for (std::uint8_t byte : payload) {
        if (!sendByteAcked(byte, error)) return false;
    }

    sleepMs(2);
    if (!stream_.writeByte(kBlockEnd)) {
        error = "Blockende laesst sich nicht senden";
        return false;
    }
    return readEcho(kBlockEnd, error);
}

bool Kwp1281::receiveBlock(std::uint8_t& title, std::vector<std::uint8_t>& payload,
                           std::string& error) {
    payload.clear();

    std::uint8_t length = 0;
    if (!receiveByteAcked(length, true, error)) return false;
    if (length < 3) {
        error = "Unsinnige Blocklaenge " + hex(length);
        return false;
    }

    std::uint8_t receivedCounter = 0;
    if (!receiveByteAcked(receivedCounter, true, error)) return false;
    counter_ = receivedCounter;

    if (!receiveByteAcked(title, true, error)) return false;

    const int dataBytes = length - 3;
    for (int i = 0; i < dataBytes; ++i) {
        std::uint8_t byte = 0;
        if (!receiveByteAcked(byte, true, error)) return false;
        payload.push_back(byte);
    }

    std::uint8_t end = 0;
    if (!receiveByteAcked(end, false, error)) return false;
    if (end != kBlockEnd) {
        error = "Blockende fehlt, stattdessen " + hex(end);
        return false;
    }
    if (log_) log_("   <- Block " + hex(title) + ", " + std::to_string(dataBytes) + " Bytes");
    return true;
}

// --- Verbindung -------------------------------------------------------------

bool Kwp1281::connect(Ecu ecu, std::string& error) {
    identification_.clear();
    counter_ = 0;
    connected_ = false;

    if (!stream_.slowInit(static_cast<std::uint8_t>(ecu))) {
        error = "Steuergeraet " + ecuName(ecu) +
                " meldet sich nicht.\n"
                "  - Zuendung an?\n"
                "  - Hat das Fahrzeug dieses Steuergeraet ueberhaupt?\n"
                "  - Bei neueren Fahrzeugen laeuft die Diagnose ueber CAN, nicht ueber K-Line.";
        return false;
    }

    // Nach der Weckadresse kommen Synchronbyte und zwei Schluesselbytes.
    std::uint8_t sync = 0, key1 = 0, key2 = 0;
    if (!stream_.readByte(sync, 2000) || sync != 0x55) {
        error = "Kein Synchronbyte 0x55 vom Steuergeraet (bekommen " + hex(sync) + ")";
        return false;
    }
    if (!stream_.readByte(key1, 1000) || !stream_.readByte(key2, 1000)) {
        error = "Schluesselbytes fehlen";
        return false;
    }
    if (log_) log_("   Schluesselbytes " + hex(key1) + " " + hex(key2));

    // Das zweite Schluesselbyte wird invertiert zurueckgegeben.
    sleepMs(30);
    if (!stream_.writeByte(static_cast<std::uint8_t>(~key2))) {
        error = "Antwort auf die Schluesselbytes laesst sich nicht senden";
        return false;
    }
    if (!readEcho(static_cast<std::uint8_t>(~key2), error)) return false;

    connected_ = true;
    if (!drainIdentification(error)) {
        connected_ = false;
        return false;
    }
    return true;
}

// Direkt nach dem Verbinden schickt das Steuergeraet seine Kennung als
// ASCII-Bloecke, bis es mit einer Quittung fertig meldet.
bool Kwp1281::drainIdentification(std::string& error) {
    for (int guard = 0; guard < 20; ++guard) {
        std::uint8_t title = 0;
        std::vector<std::uint8_t> payload;
        if (!receiveBlock(title, payload, error)) return false;

        if (title == block::kAsciiData) {
            std::string text;
            for (std::uint8_t byte : payload) {
                text += static_cast<char>(byte & 0x7F);
            }
            identification_.push_back(text);
            if (!sendBlock(block::kAck, {}, error)) return false;
            continue;
        }
        if (title == block::kAck) return true;
        // Etwas Unerwartetes: trotzdem quittieren und weitermachen.
        if (!sendBlock(block::kAck, {}, error)) return false;
    }
    error = "Das Steuergeraet hoert nicht auf, sich vorzustellen";
    return false;
}

void Kwp1281::disconnect() {
    if (!connected_) return;
    std::string ignored;
    sendBlock(block::kEndOutput, {}, ignored);
    connected_ = false;
    counter_ = 0;
}

// --- Dienste ----------------------------------------------------------------

bool Kwp1281::readFaults(std::vector<Kwp1281Fault>& out, std::string& error) {
    out.clear();
    if (!connected_) {
        error = "Nicht verbunden";
        return false;
    }
    if (!sendBlock(block::kReadFaults, {}, error)) return false;

    for (int guard = 0; guard < 20; ++guard) {
        std::uint8_t title = 0;
        std::vector<std::uint8_t> payload;
        if (!receiveBlock(title, payload, error)) return false;

        if (title == block::kFaultList) {
            // Je Fehler drei Bytes: zwei fuer die Nummer, eines fuer den Status.
            for (size_t i = 0; i + 2 < payload.size(); i += 3) {
                const int code = (payload[i] << 8) | payload[i + 1];
                if (code == 0xFFFF || code == 0) continue;
                Kwp1281Fault fault;
                fault.code = code;
                fault.status = payload[i + 2];
                fault.sporadic = (payload[i + 2] & 0x80) != 0;
                out.push_back(fault);
            }
            if (!sendBlock(block::kAck, {}, error)) return false;
            continue;
        }
        if (title == block::kAck) return true;
        if (title == block::kNotSupported) {
            error = "Dieses Steuergeraet kennt den Dienst nicht";
            return false;
        }
        if (!sendBlock(block::kAck, {}, error)) return false;
    }
    error = "Zu viele Bloecke beim Lesen des Fehlerspeichers";
    return false;
}

bool Kwp1281::clearFaults(std::string& error) {
    if (!connected_) {
        error = "Nicht verbunden";
        return false;
    }
    if (!sendBlock(block::kClearFaults, {}, error)) return false;

    std::uint8_t title = 0;
    std::vector<std::uint8_t> payload;
    if (!receiveBlock(title, payload, error)) return false;
    if (title == block::kAck) return true;
    if (title == block::kFaultList) {
        // Manche Steuergeraete schicken die geleerte Liste zurueck.
        return sendBlock(block::kAck, {}, error);
    }
    error = "Unerwartete Antwort " + hex(title) + " auf das Loeschen";
    return false;
}

bool Kwp1281::readGroup(int group, std::vector<MeasuringValue>& out, std::string& error) {
    out.clear();
    if (!connected_) {
        error = "Nicht verbunden";
        return false;
    }
    if (!sendBlock(block::kReadGroup, {static_cast<std::uint8_t>(group)}, error)) return false;

    std::uint8_t title = 0;
    std::vector<std::uint8_t> payload;
    if (!receiveBlock(title, payload, error)) return false;

    if (title == block::kNotSupported) {
        error = "Messwertblock " + std::to_string(group) + " gibt es hier nicht";
        return false;
    }
    if (title != block::kGroupReading) {
        error = "Unerwartete Antwort " + hex(title) + " auf den Messwertblock";
        return false;
    }
    // Je Anzeigefeld drei Bytes: Formel, Wert A, Wert B.
    for (size_t i = 0; i + 2 < payload.size(); i += 3) {
        out.push_back(decodeMeasurement(payload[i], payload[i + 1], payload[i + 2]));
    }
    return true;
}

// Grundeinstellung. Aufbau wie beim Messwertblock, aber das Steuergeraet
// regelt dabei aktiv: bei Block 060 beziehungsweise 098 faehrt es die
// Drosselklappe an ihre Anschlaege und uebernimmt die Werte.
bool Kwp1281::basicSetting(int group, std::vector<MeasuringValue>& out, std::string& error) {
    out.clear();
    if (!connected_) {
        error = "Nicht verbunden";
        return false;
    }
    if (!sendBlock(block::kBasicSetting, {static_cast<std::uint8_t>(group)}, error)) {
        return false;
    }

    std::uint8_t title = 0;
    std::vector<std::uint8_t> payload;
    if (!receiveBlock(title, payload, error)) return false;

    if (title == block::kNotSupported) {
        error = "Grundeinstellung " + std::to_string(group) +
                " kennt dieses Steuergeraet nicht";
        return false;
    }
    if (title != block::kGroupReading) {
        error = "Unerwartete Antwort " + hex(title) + " auf die Grundeinstellung";
        return false;
    }
    for (size_t i = 0; i + 2 < payload.size(); i += 3) {
        out.push_back(decodeMeasurement(payload[i], payload[i + 1], payload[i + 2]));
    }
    return true;
}

// Ohne regelmaessigen Verkehr faellt die Verbindung nach etwa einer Sekunde.
bool Kwp1281::keepAlive(std::string& error) {
    if (!connected_) return false;
    if (!sendBlock(block::kAck, {}, error)) return false;
    std::uint8_t title = 0;
    std::vector<std::uint8_t> payload;
    return receiveBlock(title, payload, error);
}

// --- Messwertformeln --------------------------------------------------------

// VAG kodiert jeden Wert mit einer Formelnummer und zwei Bytes. Das hier sind
// die Formeln, die an Motorsteuergeraeten tatsaechlich vorkommen.
MeasuringValue decodeMeasurement(std::uint8_t formula, std::uint8_t a, std::uint8_t b) {
    MeasuringValue m;
    m.valid = true;
    const double da = a;
    const double db = b;

    switch (formula) {
        case 0x00: m.valid = false; m.label = "leer"; break;
        case 0x01: m.value = 0.2 * da * db;          m.unit = "1/min";  m.label = "Drehzahl"; break;
        case 0x02: m.value = da * 0.002 * db;        m.unit = "%";      m.label = "Anteil"; break;
        case 0x03: m.value = 0.002 * da * db;        m.unit = "Grad";   m.label = "Winkel"; break;
        case 0x04: m.value = std::fabs(db - 127.0) * 0.01 * da; m.unit = "Grad v.OT"; m.label = "Zuendwinkel"; break;
        case 0x05: m.value = da * (db - 100.0) * 0.1; m.unit = "Grad C"; m.label = "Temperatur"; break;
        case 0x06: m.value = 0.001 * da * db;        m.unit = "V";      m.label = "Spannung"; break;
        case 0x07: m.value = 0.01 * da * db;         m.unit = "km/h";   m.label = "Geschwindigkeit"; break;
        case 0x08: m.value = 0.1 * da * db;          m.unit = "";       m.label = "Wert"; break;
        case 0x09: m.value = (db - 127.0) * 0.02 * da; m.unit = "Grad"; m.label = "Winkel"; break;
        case 0x0A: m.value = db;                     m.unit = "";
                   m.label = (b == 0) ? "kalt" : "warm"; break;
        case 0x0B: m.value = (db - 128.0) * 0.0001 * da + 1.0; m.unit = ""; m.label = "Faktor"; break;
        case 0x0C: m.value = 0.001 * da * db;        m.unit = "Ohm";    m.label = "Widerstand"; break;
        case 0x0D: m.value = (db - 127.0) * 0.001 * da; m.unit = "mm";  m.label = "Weg"; break;
        case 0x0E: m.value = 0.005 * da * db;        m.unit = "bar";    m.label = "Druck"; break;
        case 0x0F: m.value = 0.01 * da * db;         m.unit = "ms";     m.label = "Zeit"; break;
        case 0x12: m.value = 0.04 * da * db;         m.unit = "mbar";   m.label = "Druck"; break;
        case 0x13: m.value = 0.01 * da * db;         m.unit = "l";      m.label = "Menge"; break;
        case 0x14: m.value = da * (db - 128.0) / 128.0; m.unit = "%";   m.label = "Anpassung"; break;
        case 0x15: m.value = 0.001 * da * db;        m.unit = "V";      m.label = "Spannung"; break;
        case 0x16: m.value = 0.001 * da * db;        m.unit = "ms";     m.label = "Zeit"; break;
        case 0x17: m.value = db / 256.0 * da;        m.unit = "%";      m.label = "Anteil"; break;
        case 0x19: m.value = da * 1.421 + db / 182.0; m.unit = "g/s";   m.label = "Luftmasse"; break;
        case 0x1A: m.value = db - da;                m.unit = "Grad C"; m.label = "Temperaturdifferenz"; break;
        case 0x1B: m.value = std::fabs(db - 128.0) * 0.01 * da; m.unit = "Grad v.OT"; m.label = "Winkel"; break;
        case 0x1C: m.value = db - da;                m.unit = "";       m.label = "Differenz"; break;
        case 0x1E: m.value = da * db / 12.0;         m.unit = "km/h";   m.label = "Geschwindigkeit"; break;
        case 0x1F: m.value = da * db / 2560.0;       m.unit = "";       m.label = "Wert"; break;
        case 0x21: m.value = (a == 0) ? (100.0 * db / 255.0) : (100.0 * db / da);
                   m.unit = "%"; m.label = "Anteil"; break;
        case 0x22: m.value = (da * db) / 100.0 - 400.0; m.unit = "kW";  m.label = "Leistung"; break;
        case 0x23: m.value = da * db / 100.0;        m.unit = "l/h";    m.label = "Verbrauch"; break;
        case 0x24: m.value = da * 2560.0 + db * 10.0; m.unit = "km";    m.label = "Strecke"; break;
        case 0x25: m.value = db;                     m.unit = "";       m.label = "Bitmuster"; break;
        case 0x27: m.value = db / da;                m.unit = "mg/H";   m.label = "Einspritzmenge"; break;
        case 0x31: m.value = db / 40.0 * da;         m.unit = "mg/h";   m.label = "Menge"; break;
        case 0x33: m.value = (db - 128.0) * da / 255.0; m.unit = "mg/H"; m.label = "Menge"; break;
        case 0x36: m.value = da * 256.0 + db;        m.unit = "";       m.label = "Zaehler"; break;
        case 0x37: m.value = da * db / 200.0;        m.unit = "s";      m.label = "Zeit"; break;
        default:
            m.value = da * 256.0 + db;
            m.unit = "";
            m.label = "Formel " + hex(formula);
            break;
    }
    return m;
}

}  // namespace caruspex
