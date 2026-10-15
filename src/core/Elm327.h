// ELM327 driver: initialisation, protocol selection, commands, response parsing.
#pragma once

#include <functional>
#include <memory>
#include <string>
#include <vector>

#include "core/Lang.h"
#include "core/Transport.h"

namespace caruspex {

using LogFn = std::function<void(const std::string&)>;

enum class ElmStatus {
    Ok,
    NoData,        // control unit does not know the service
    BusError,      // no contact with the vehicle
    AdapterError,  // adapter does not answer at all / not an ELM327
    Negative,      // control unit refuses (7F ...)
};

struct ElmResult {
    ElmStatus status = ElmStatus::Ok;
    std::vector<std::string> lines;              // raw lines
    std::vector<std::vector<int>> messages;      // decoded payload without the service byte
    std::string message;                         // plain-text message on errors
    bool ok() const { return status == ElmStatus::Ok; }
};

// Order suited to VAG: K-Line first (A2), then CAN (A4 B7).
extern const char* const kProtocolOrderKLineFirst;
extern const char* const kProtocolOrderCanFirst;

std::string protocolName(char id);

class Elm327 {
public:
    void setLog(LogFn log) { log_ = std::move(log); }
    // Every timeout is multiplied by this factor. Sluggish clones need more
    // time; the gradation between short and long waits is preserved because
    // not everything is set to the same value.
    void setTimeoutScale(double scale) { timeoutScale_ = scale > 0.0 ? scale : 1.0; }
    // Log every sent line together with its response.
    void setVerbose(bool verbose) { verbose_ = verbose; }

    // takes ownership of the transport
    bool connect(std::unique_ptr<ITransport> transport, const std::string& protocolOrder,
                 std::string& error);
    void disconnect();
    bool connected() const { return connected_; }

    ElmResult send(const std::string& command, int timeoutMs = 5000);
    // Sends and decodes the response to the expected service (e.g. 0x03 -> 0x43).
    ElmResult request(const std::string& command, int expectedService, int timeoutMs = 6000);

    const std::string& version() const { return version_; }
    const std::string& protocolText() const { return protocolText_; }
    std::string transportName() const { return transport_ ? transport_->name() : ""; }

private:
    ElmResult at(const std::string& command, int timeoutMs = 5000);
    void drainGreeting();

    std::unique_ptr<ITransport> transport_;
    LogFn log_;
    double timeoutScale_ = 1.0;
    bool verbose_ = true;
    std::string version_ = "?";
    std::string protocolText_ = L("unbekannt", "unknown");
    bool connected_ = false;
};

// Helper functions, also usable from tests.
std::vector<int> hexToBytes(const std::string& text);
ElmResult decodeMessages(const std::vector<std::string>& lines, int expectedService);

}  // namespace caruspex
