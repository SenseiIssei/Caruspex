// Ein Zugang zu beiden VAG-Protokollen auf der K-Leitung.
//
// Nach der Weckadresse verraten die Schluesselbytes, ob das Steuergeraet das
// alte Blockprotokoll KWP1281 oder das neuere KWP2000 spricht. Der Audi A2
// liegt genau auf dieser Grenze: der 1.4 mit Saugrohreinspritzung meist noch
// KWP1281, der 1.6 FSI mit seinem Bosch MED meist schon KWP2000. Wer die
// Software bedient, soll das nicht wissen muessen.
#pragma once

#include <memory>
#include <string>
#include <vector>

#include "core/Kwp1281.h"
#include "core/Kwp2000.h"

namespace caruspex {

class VagKLine {
public:
    explicit VagKLine(IByteStream& stream, LogFn2 log = {});
    ~VagKLine();

    bool connect(Ecu ecu, std::string& error);
    void disconnect();
    bool connected() const;

    KLineProtocol protocol() const { return protocol_; }
    std::string protocolName() const { return protocolLabel(protocol_); }
    std::vector<std::string> identification() const;

    // Von beiden Protokollen unterstuetzt.
    bool readFaults(std::vector<Kwp1281Fault>& out, std::string& error);
    bool clearFaults(std::string& error);
    bool readGroup(int group, std::vector<MeasuringValue>& out, std::string& error);
    bool basicSetting(int group, std::vector<MeasuringValue>& out, std::string& error);
    bool keepAlive(std::string& error);

    // Nur KWP1281 kennt diese beiden. Bei KWP2000 melden sie das sauber zurueck.
    bool readAdaptation(int channel, int& value, std::vector<MeasuringValue>& out,
                        std::string& error);
    bool actuatorTest(std::string& activeActuator, std::string& error);

    // Nur KWP2000: Sitzungsart fuer die Grundeinstellung. Welche das jeweilige
    // Steuergeraet verlangt, ist nicht einheitlich.
    void setBasicSettingSession(std::uint8_t mode) { basicSettingSession_ = mode; }
    std::uint8_t basicSettingSession() const { return basicSettingSession_; }

private:
    IByteStream& stream_;
    LogFn2 log_;
    KLineProtocol protocol_ = KLineProtocol::Unknown;
    std::unique_ptr<Kwp1281> old_;
    std::unique_ptr<Kwp2000> modern_;
    std::uint8_t basicSettingSession_ = session::kAdaptation;
};

}  // namespace caruspex
