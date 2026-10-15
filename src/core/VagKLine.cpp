#include "core/VagKLine.h"

#include "core/Lang.h"

namespace caruspex {

VagKLine::VagKLine(IByteStream& stream, LogFn2 log)
    : stream_(stream), log_(std::move(log)) {}

VagKLine::~VagKLine() { disconnect(); }

bool VagKLine::connect(Ecu ecu, std::string& error) {
    disconnect();

    std::uint8_t key1 = 0, key2 = 0;
    if (!wakeUpKLine(stream_, ecu, key1, key2, protocol_, error, log_)) {
        protocol_ = KLineProtocol::Unknown;
        return false;
    }

    if (protocol_ == KLineProtocol::Kwp1281) {
        old_ = std::make_unique<Kwp1281>(stream_, log_);
        old_->setEcu(ecu);
        if (!old_->attach(error)) {
            old_.reset();
            return false;
        }
        return true;
    }

    modern_ = std::make_unique<Kwp2000>(stream_, log_);
    if (!modern_->start(ecu, key1, key2, error)) {
        modern_.reset();
        return false;
    }
    return true;
}

void VagKLine::disconnect() {
    if (old_) {
        old_->disconnect();
        old_.reset();
    }
    if (modern_) {
        modern_->stop();
        modern_.reset();
    }
    protocol_ = KLineProtocol::Unknown;
}

bool VagKLine::connected() const {
    if (old_) return old_->connected();
    if (modern_) return modern_->started();
    return false;
}

std::vector<std::string> VagKLine::identification() const {
    if (old_) return old_->identification();
    if (modern_) return modern_->identification();
    return {};
}

bool VagKLine::readFaults(std::vector<Kwp1281Fault>& out, std::string& error) {
    if (old_) return old_->readFaults(out, error);
    if (modern_) return modern_->readFaults(out, error);
    error = L("Nicht verbunden", "Not connected");
    return false;
}

bool VagKLine::clearFaults(std::string& error) {
    if (old_) return old_->clearFaults(error);
    if (modern_) return modern_->clearFaults(error);
    error = L("Nicht verbunden", "Not connected");
    return false;
}

bool VagKLine::readGroup(int group, std::vector<MeasuringValue>& out, std::string& error) {
    if (old_) return old_->readGroup(group, out, error);
    if (modern_) return modern_->readGroup(group, out, error);
    error = L("Nicht verbunden", "Not connected");
    return false;
}

bool VagKLine::basicSetting(int group, std::vector<MeasuringValue>& out,
                            std::string& error) {
    if (old_) return old_->basicSetting(group, out, error);
    if (modern_) return modern_->basicSetting(group, out, error, basicSettingSession_);
    error = L("Nicht verbunden", "Not connected");
    return false;
}

bool VagKLine::keepAlive(std::string& error) {
    if (old_) return old_->keepAlive(error);
    if (modern_) return modern_->testerPresent(error);
    error = L("Nicht verbunden", "Not connected");
    return false;
}

bool VagKLine::readAdaptation(int channel, int& value, std::vector<MeasuringValue>& out,
                              std::string& error) {
    if (old_) return old_->readAdaptation(channel, value, out, error);
    if (modern_) {
        error = L(
            "Anpassungskanaele lesen ist bisher nur fuer KWP1281 umgesetzt. Dieses "
            "Steuergeraet spricht KWP2000.",
            "Reading adaptation channels is only implemented for KWP1281 so far. This "
            "control unit speaks KWP2000.");
        return false;
    }
    error = L("Nicht verbunden", "Not connected");
    return false;
}

bool VagKLine::actuatorTest(std::string& activeActuator, std::string& error) {
    if (old_) return old_->actuatorTest(activeActuator, error);
    if (modern_) {
        error = L(
            "Die Stellglieddiagnose ist bisher nur fuer KWP1281 umgesetzt. Dieses "
            "Steuergeraet spricht KWP2000.",
            "The output test is only implemented for KWP1281 so far. This "
            "control unit speaks KWP2000.");
        return false;
    }
    error = L("Nicht verbunden", "Not connected");
    return false;
}

}  // namespace caruspex
