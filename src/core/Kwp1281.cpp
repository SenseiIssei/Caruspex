#include "core/Kwp1281.h"

#include <cmath>
#include <cstdio>

#include "core/Lang.h"
#include "core/Timing.h"
#include "data/VagFaults.h"

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
        case Ecu::Engine: return L("01 Motor", "01 Engine");
        case Ecu::Gearbox: return L("02 Getriebe", "02 Transmission");
        case Ecu::Abs: return L("03 Bremsenelektronik (ABS)", "03 Brake electronics (ABS)");
        case Ecu::AirConditioning: return L("08 Klimaanlage", "08 Air conditioning");
        case Ecu::CentralElectrics: return L("09 Zentralelektrik", "09 Central electrics");
        case Ecu::Airbag: return L("15 Airbag", "15 Airbags");
        case Ecu::SteeringWheel: return L("16 Lenkradelektronik", "16 Steering wheel electronics");
        case Ecu::InstrumentCluster: return L("17 Kombiinstrument", "17 Instrument cluster");
        case Ecu::Immobilizer: return L("25 Wegfahrsperre", "25 Immobilizer");
        case Ecu::ComfortSystem: return L("46 Komfortsystem", "46 Comfort system");
        case Ecu::Radio: return "56 Radio";
    }
    return L("unbekannt", "unknown");
}

const std::vector<Ecu>& knownEcus() {
    static const std::vector<Ecu> list = {
        Ecu::Engine,   Ecu::Gearbox,           Ecu::Abs,        Ecu::AirConditioning,
        Ecu::CentralElectrics, Ecu::Airbag,    Ecu::SteeringWheel,
        Ecu::InstrumentCluster, Ecu::Immobilizer, Ecu::ComfortSystem, Ecu::Radio};
    return list;
}

Kwp1281::Kwp1281(IByteStream& stream, LogFn2 log) : stream_(stream), log_(std::move(log)) {}

// --- Byte-by-byte rhythm ----------------------------------------------------

// On the single-wire line you hear yourself. The echo has to be removed,
// otherwise it would be mistaken for the other side's answer.
bool Kwp1281::readEcho(std::uint8_t expected, std::string& error) {
    if (!stream_.echoesOwnBytes()) return true;
    std::uint8_t echo = 0;
    if (!stream_.readByte(echo, kAckTimeoutMs)) {
        error = L("Kein Echo auf ", "No echo for ") + hex(expected) +
                L(", hängt das Kabel richtig?", ", is the cable connected properly?");
        return false;
    }
    if (echo != expected) {
        error = L("Echo passt nicht: erwartet ", "Echo mismatch: expected ") + hex(expected) +
                L(", bekommen ", ", got ") + hex(echo);
        return false;
    }
    return true;
}

// Send a byte, discard the echo, then wait for the other side's complement.
bool Kwp1281::sendByteAcked(std::uint8_t value, std::string& error) {
    sleepMs(2);   // the control unit needs a short pause between bytes
    if (!stream_.writeByte(value)) {
        error = L("Senden fehlgeschlagen", "Send failed");
        return false;
    }
    if (!readEcho(value, error)) return false;

    std::uint8_t ack = 0;
    if (!stream_.readByte(ack, kByteTimeoutMs)) {
        error = L("Steuergerät quittiert ", "Control unit does not acknowledge ") + hex(value) +
                L(" nicht", "");
        return false;
    }
    const std::uint8_t expected = static_cast<std::uint8_t>(~value);
    if (ack != expected) {
        error = L("Falsche Quittung: erwartet ", "Wrong acknowledgement: expected ") + hex(expected) +
                L(", bekommen ", ", got ") + hex(ack);
        return false;
    }
    return true;
}

// Receive a byte and acknowledge it with its complement.
bool Kwp1281::receiveByteAcked(std::uint8_t& value, bool sendAck, std::string& error) {
    if (!stream_.readByte(value, kByteTimeoutMs)) {
        error = L("Steuergerät antwortet nicht mehr", "Control unit stopped responding");
        return false;
    }
    if (!sendAck) return true;

    const std::uint8_t ack = static_cast<std::uint8_t>(~value);
    sleepMs(2);
    if (!stream_.writeByte(ack)) {
        error = L("Quittung lässt sich nicht senden", "Cannot send acknowledgement");
        return false;
    }
    return readEcho(ack, error);
}

// --- Blocks ----------------------------------------------------------------

// Layout: length, counter, title, payload, 0x03. The length counts
// everything except itself. The final 0x03 is not acknowledged.
bool Kwp1281::sendBlock(std::uint8_t title, const std::vector<std::uint8_t>& payload,
                        std::string& error) {
    const std::uint8_t length = static_cast<std::uint8_t>(payload.size() + 3);
    counter_ = static_cast<std::uint8_t>(counter_ + 1);

    if (log_) log_("   -> Block " + hex(title) + L(" (Zähler ", " (counter ") + hex(counter_) + ")");

    if (!sendByteAcked(length, error)) return false;
    if (!sendByteAcked(counter_, error)) return false;
    if (!sendByteAcked(title, error)) return false;
    for (std::uint8_t byte : payload) {
        if (!sendByteAcked(byte, error)) return false;
    }

    sleepMs(2);
    if (!stream_.writeByte(kBlockEnd)) {
        error = L("Blockende lässt sich nicht senden", "Cannot send block end");
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
        error = L("Unsinnige Blocklänge ", "Implausible block length ") + hex(length);
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
        error = L("Blockende fehlt, stattdessen ", "Block end missing, got ") + hex(end);
        return false;
    }
    if (log_) log_("   <- Block " + hex(title) + ", " + std::to_string(dataBytes) + L(" Bytes", " bytes"));
    return true;
}

// --- Connection -------------------------------------------------------------

bool Kwp1281::connect(Ecu ecu, std::string& error) {
    ecu_ = ecu;
    identification_.clear();
    counter_ = 0;
    connected_ = false;

    if (!stream_.slowInit(static_cast<std::uint8_t>(ecu))) {
        error = L("Steuergerät ", "Control unit ") + ecuName(ecu) +
                L(" meldet sich nicht.\n"
                  "  - Zündung an?\n"
                  "  - Hat das Fahrzeug dieses Steuergerät überhaupt?\n"
                  "  - Bei neueren Fahrzeugen läuft die Diagnose über CAN, nicht über K-Line.",
                  " does not respond.\n"
                  "  - Ignition on?\n"
                  "  - Does the vehicle have this control unit at all?\n"
                  "  - Newer vehicles are diagnosed over CAN, not over K-Line.");
        return false;
    }

    // After the wake-up address come the sync byte and two key bytes.
    std::uint8_t sync = 0, key1 = 0, key2 = 0;
    if (!stream_.readByte(sync, 2000) || sync != 0x55) {
        error = L("Kein Synchronbyte 0x55 vom Steuergerät (bekommen ",
                  "No sync byte 0x55 from the control unit (got ") +
                hex(sync) + ")";
        return false;
    }
    if (!stream_.readByte(key1, 1000) || !stream_.readByte(key2, 1000)) {
        error = L("Schlüsselbytes fehlen", "Key bytes missing");
        return false;
    }
    if (log_) log_(L("   Schlüsselbytes ", "   Key bytes ") + hex(key1) + " " + hex(key2));

    // The second key byte is sent back inverted.
    sleepMs(30);
    if (!stream_.writeByte(static_cast<std::uint8_t>(~key2))) {
        error = L("Antwort auf die Schlüsselbytes lässt sich nicht senden",
                  "Cannot send the reply to the key bytes");
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

// Someone else has already done the wake-up; here we only fetch the
// identification. Needed when the key bytes revealed that this control unit
// speaks the old block protocol.
bool Kwp1281::attach(std::string& error) {
    identification_.clear();
    counter_ = 0;
    connected_ = true;
    if (!drainIdentification(error)) {
        connected_ = false;
        return false;
    }
    return true;
}

// Right after connecting, the control unit sends its identification as
// ASCII blocks until it signals completion with an acknowledgement.
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
        // Something unexpected: acknowledge anyway and carry on.
        if (!sendBlock(block::kAck, {}, error)) return false;
    }
    error = L("Das Steuergerät hört nicht auf, sich vorzustellen",
              "The control unit keeps sending identification blocks");
    return false;
}

void Kwp1281::disconnect() {
    if (!connected_) return;
    std::string ignored;
    sendBlock(block::kEndOutput, {}, ignored);
    connected_ = false;
    counter_ = 0;
}

// --- Services ---------------------------------------------------------------

bool Kwp1281::readFaults(std::vector<Kwp1281Fault>& out, std::string& error) {
    out.clear();
    if (!connected_) {
        error = L("Nicht verbunden", "Not connected");
        return false;
    }
    if (!sendBlock(block::kReadFaults, {}, error)) return false;

    for (int guard = 0; guard < 20; ++guard) {
        std::uint8_t title = 0;
        std::vector<std::uint8_t> payload;
        if (!receiveBlock(title, payload, error)) return false;

        if (title == block::kFaultList) {
            // Three bytes per fault: two for the number, one for the status.
            for (size_t i = 0; i + 2 < payload.size(); i += 3) {
                const int code = (payload[i] << 8) | payload[i + 1];
                if (code == 0xFFFF || code == 0) continue;
                Kwp1281Fault fault;
                fault.code = code;
                fault.status = payload[i + 2];
                fault.sporadic = (payload[i + 2] & 0x80) != 0;
                fault.text = describeVagFault(code, static_cast<int>(ecu_));
                out.push_back(fault);
            }
            if (!sendBlock(block::kAck, {}, error)) return false;
            continue;
        }
        if (title == block::kAck) return true;
        if (title == block::kNotSupported) {
            error = L("Dieses Steuergerät kennt den Dienst nicht",
                      "This control unit does not support the service");
            return false;
        }
        if (!sendBlock(block::kAck, {}, error)) return false;
    }
    error = L("Zu viele Blöcke beim Lesen des Fehlerspeichers",
              "Too many blocks while reading the fault memory");
    return false;
}

bool Kwp1281::clearFaults(std::string& error) {
    if (!connected_) {
        error = L("Nicht verbunden", "Not connected");
        return false;
    }
    if (!sendBlock(block::kClearFaults, {}, error)) return false;

    std::uint8_t title = 0;
    std::vector<std::uint8_t> payload;
    if (!receiveBlock(title, payload, error)) return false;
    if (title == block::kAck) return true;
    if (title == block::kFaultList) {
        // Some control units send back the emptied list.
        return sendBlock(block::kAck, {}, error);
    }
    error = L("Unerwartete Antwort ", "Unexpected response ") + hex(title) +
            L(" auf das Löschen", " to the clear request");
    return false;
}

bool Kwp1281::readGroup(int group, std::vector<MeasuringValue>& out, std::string& error) {
    out.clear();
    if (!connected_) {
        error = L("Nicht verbunden", "Not connected");
        return false;
    }
    if (!sendBlock(block::kReadGroup, {static_cast<std::uint8_t>(group)}, error)) return false;

    std::uint8_t title = 0;
    std::vector<std::uint8_t> payload;
    if (!receiveBlock(title, payload, error)) return false;

    if (title == block::kNotSupported) {
        error = L("Messwertblock ", "Measuring block ") + std::to_string(group) +
                L(" gibt es hier nicht", " does not exist here");
        return false;
    }
    if (title != block::kGroupReading) {
        error = L("Unerwartete Antwort ", "Unexpected response ") + hex(title) +
                L(" auf den Messwertblock", " to the measuring block");
        return false;
    }
    // Three bytes per display field: formula, value A, value B.
    for (size_t i = 0; i + 2 < payload.size(); i += 3) {
        out.push_back(decodeMeasurement(payload[i], payload[i + 1], payload[i + 2]));
    }
    return true;
}

// Basic settings. Same layout as the measuring block, but the control unit
// actively regulates: with block 060 or 098 it drives the throttle body to
// its end stops and stores the values.
bool Kwp1281::basicSetting(int group, std::vector<MeasuringValue>& out, std::string& error) {
    out.clear();
    if (!connected_) {
        error = L("Nicht verbunden", "Not connected");
        return false;
    }
    if (!sendBlock(block::kBasicSetting, {static_cast<std::uint8_t>(group)}, error)) {
        return false;
    }

    std::uint8_t title = 0;
    std::vector<std::uint8_t> payload;
    if (!receiveBlock(title, payload, error)) return false;

    if (title == block::kNotSupported) {
        error = L("Grundeinstellung ", "Basic settings ") + std::to_string(group) +
                L(" kennt dieses Steuergerät nicht", " are not supported by this control unit");
        return false;
    }
    if (title != block::kGroupReading) {
        error = L("Unerwartete Antwort ", "Unexpected response ") + hex(title) +
                L(" auf die Grundeinstellung", " to the basic settings");
        return false;
    }
    for (size_t i = 0; i + 2 < payload.size(); i += 3) {
        out.push_back(decodeMeasurement(payload[i], payload[i + 1], payload[i + 2]));
    }
    return true;
}

bool Kwp1281::readAdaptation(int channel, int& value, std::vector<MeasuringValue>& out,
                             std::string& error) {
    out.clear();
    value = 0;
    if (!connected_) {
        error = L("Nicht verbunden", "Not connected");
        return false;
    }
    if (!sendBlock(block::kReadAdaptation, {static_cast<std::uint8_t>(channel)}, error)) {
        return false;
    }

    std::uint8_t title = 0;
    std::vector<std::uint8_t> payload;
    if (!receiveBlock(title, payload, error)) return false;

    if (title == block::kNotSupported) {
        error = L("Anpassungskanal ", "Adaptation channel ") + std::to_string(channel) +
                L(" gibt es hier nicht", " does not exist here");
        return false;
    }
    if (title != block::kAdaptationValue && title != block::kGroupReading) {
        error = L("Unerwartete Antwort ", "Unexpected response ") + hex(title) +
                L(" auf den Anpassungskanal", " to the adaptation channel");
        return false;
    }
    // First the four display fields, then two bytes with the channel value.
    size_t i = 0;
    for (; i + 2 < payload.size() && out.size() < 4; i += 3) {
        out.push_back(decodeMeasurement(payload[i], payload[i + 1], payload[i + 2]));
    }
    if (i + 1 < payload.size()) value = (payload[i] << 8) | payload[i + 1];
    return true;
}

bool Kwp1281::actuatorTest(std::string& activeActuator, std::string& error) {
    activeActuator.clear();
    if (!connected_) {
        error = L("Nicht verbunden", "Not connected");
        return false;
    }
    if (!sendBlock(block::kActuatorTest, {}, error)) return false;

    std::uint8_t title = 0;
    std::vector<std::uint8_t> payload;
    if (!receiveBlock(title, payload, error)) return false;

    if (title == block::kNotSupported) {
        error = L("Stellglieddiagnose kennt dieses Steuergerät nicht",
                  "This control unit does not support the output test");
        return false;
    }
    if (title == block::kAck) {
        activeActuator = L("Ende der Stellglieddiagnose", "End of output test");
        return true;
    }
    // The control unit names the active actuator either as text or as a
    // two-byte number.
    if (title == block::kAsciiData) {
        for (std::uint8_t byte : payload) activeActuator += static_cast<char>(byte & 0x7F);
        return true;
    }
    if (payload.size() >= 2) {
        const int number = (payload[0] << 8) | payload[1];
        activeActuator = L("Stellglied ", "Actuator ") + std::to_string(number);
        return true;
    }
    activeActuator = L("unbekanntes Stellglied", "unknown actuator");
    return true;
}

// Without regular traffic the connection drops after about one second.
bool Kwp1281::keepAlive(std::string& error) {
    if (!connected_) return false;
    if (!sendBlock(block::kAck, {}, error)) return false;
    std::uint8_t title = 0;
    std::vector<std::uint8_t> payload;
    return receiveBlock(title, payload, error);
}

// --- Measuring value formulas---------------------------------------------------

// VAG encodes every value with a formula number and two bytes. These are the
// formulas that actually occur on engine control units.
MeasuringValue decodeMeasurement(std::uint8_t formula, std::uint8_t a, std::uint8_t b) {
    MeasuringValue m;
    m.valid = true;
    const double da = a;
    const double db = b;

    switch (formula) {
        case 0x00: m.valid = false; m.label = L("leer", "empty"); break;
        case 0x01: m.value = 0.2 * da * db;          m.unit = "1/min";  m.label = L("Drehzahl", "Engine speed"); break;
        case 0x02: m.value = da * 0.002 * db;        m.unit = "%";      m.label = L("Anteil", "Ratio"); break;
        case 0x03: m.value = 0.002 * da * db;        m.unit = L("Grad", "deg");   m.label = L("Winkel", "Angle"); break;
        case 0x04: m.value = std::fabs(db - 127.0) * 0.01 * da; m.unit = L("Grad v.OT", "deg BTDC"); m.label = L("Zündwinkel", "Ignition timing"); break;
        case 0x05: m.value = da * (db - 100.0) * 0.1; m.unit = L("Grad C", "deg C"); m.label = L("Temperatur", "Temperature"); break;
        case 0x06: m.value = 0.001 * da * db;        m.unit = "V";      m.label = L("Spannung", "Voltage"); break;
        case 0x07: m.value = 0.01 * da * db;         m.unit = "km/h";   m.label = L("Geschwindigkeit", "Speed"); break;
        case 0x08: m.value = 0.1 * da * db;          m.unit = "";       m.label = L("Wert", "Value"); break;
        case 0x09: m.value = (db - 127.0) * 0.02 * da; m.unit = L("Grad", "deg"); m.label = L("Winkel", "Angle"); break;
        case 0x0A: m.value = db;                     m.unit = "";
                   m.label = (b == 0) ? L("kalt", "cold") : L("warm", "warm"); break;
        case 0x0B: m.value = (db - 128.0) * 0.0001 * da + 1.0; m.unit = ""; m.label = L("Faktor", "Factor"); break;
        case 0x0C: m.value = 0.001 * da * db;        m.unit = "Ohm";    m.label = L("Widerstand", "Resistance"); break;
        case 0x0D: m.value = (db - 127.0) * 0.001 * da; m.unit = "mm";  m.label = L("Weg", "Travel"); break;
        case 0x0E: m.value = 0.005 * da * db;        m.unit = "bar";    m.label = L("Druck", "Pressure"); break;
        case 0x0F: m.value = 0.01 * da * db;         m.unit = "ms";     m.label = L("Zeit", "Time"); break;
        case 0x12: m.value = 0.04 * da * db;         m.unit = "mbar";   m.label = L("Druck", "Pressure"); break;
        case 0x13: m.value = 0.01 * da * db;         m.unit = "l";      m.label = L("Menge", "Quantity"); break;
        case 0x14: m.value = da * (db - 128.0) / 128.0; m.unit = "%";   m.label = L("Anpassung", "Adaptation"); break;
        case 0x15: m.value = 0.001 * da * db;        m.unit = "V";      m.label = L("Spannung", "Voltage"); break;
        case 0x16: m.value = 0.001 * da * db;        m.unit = "ms";     m.label = L("Zeit", "Time"); break;
        case 0x17: m.value = db / 256.0 * da;        m.unit = "%";      m.label = L("Anteil", "Ratio"); break;
        case 0x19: m.value = da * 1.421 + db / 182.0; m.unit = "g/s";   m.label = L("Luftmasse", "Air mass"); break;
        case 0x1A: m.value = db - da;                m.unit = L("Grad C", "deg C"); m.label = L("Temperaturdifferenz", "Temperature difference"); break;
        case 0x1B: m.value = std::fabs(db - 128.0) * 0.01 * da; m.unit = L("Grad v.OT", "deg BTDC"); m.label = L("Winkel", "Angle"); break;
        case 0x1C: m.value = db - da;                m.unit = "";       m.label = L("Differenz", "Difference"); break;
        case 0x1E: m.value = da * db / 12.0;         m.unit = "km/h";   m.label = L("Geschwindigkeit", "Speed"); break;
        case 0x1F: m.value = da * db / 2560.0;       m.unit = "";       m.label = L("Wert", "Value"); break;
        case 0x21: m.value = (a == 0) ? (100.0 * db / 255.0) : (100.0 * db / da);
                   m.unit = "%"; m.label = L("Anteil", "Ratio"); break;
        case 0x22: m.value = (da * db) / 100.0 - 400.0; m.unit = "kW";  m.label = L("Leistung", "Power"); break;
        case 0x23: m.value = da * db / 100.0;        m.unit = "l/h";    m.label = L("Verbrauch", "Consumption"); break;
        case 0x24: m.value = da * 2560.0 + db * 10.0; m.unit = "km";    m.label = L("Strecke", "Distance"); break;
        case 0x25: m.value = db;                     m.unit = "";       m.label = L("Bitmuster", "Bit pattern"); break;
        case 0x27: m.value = db / da;                m.unit = "mg/H";   m.label = L("Einspritzmenge", "Injection quantity"); break;
        case 0x31: m.value = db / 40.0 * da;         m.unit = "mg/h";   m.label = L("Menge", "Quantity"); break;
        case 0x33: m.value = (db - 128.0) * da / 255.0; m.unit = "mg/H"; m.label = L("Menge", "Quantity"); break;
        case 0x36: m.value = da * 256.0 + db;        m.unit = "";       m.label = L("Zähler", "Counter"); break;
        case 0x37: m.value = da * db / 200.0;        m.unit = "s";      m.label = L("Zeit", "Time"); break;
        default:
            m.value = da * 256.0 + db;
            m.unit = "";
            m.label = L("Formel ", "Formula ") + hex(formula);
            break;
    }
    return m;
}

}  // namespace caruspex
