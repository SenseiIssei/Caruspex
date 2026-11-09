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
    // Parallel to messages: which control unit sent each one. Only filled
    // while headers are switched on (Elm327::setHeaders), otherwise every
    // entry is empty. Format: "7E8" on CAN 11 bit, "18DAF110" on CAN 29 bit,
    // the source byte as "10" on K-line and J1850.
    std::vector<std::string> sources;
    std::string message;                         // plain-text message on errors
    bool ok() const { return status == ElmStatus::Ok; }
};

// How the adapter prints a response line while headers are on (ATH1).
enum class HeaderFormat {
    None,    // headers off: payload only, multi-frame CAN as "0:", "1:" lines
    Can11,   // "7E8 06 41 00 BE 3F A8 13": 11-bit ID, then the ISO-TP frame
    Can29,   // "18DAF110 06 41 00 ...": 29-bit ID, then the ISO-TP frame
    KLine,   // ISO 9141-2 and ISO 14230 (KWP): 3 or 4 header bytes, data, checksum
    J1850,   // SAE J1850 PWM/VPW: 3 header bytes, data, CRC
};

// Header format the adapter uses for a protocol number ('1' to '9').
HeaderFormat headerFormatFor(char protocolId);

// Order suited to VAG: K-Line first (A2), then CAN (A4 B7).
extern const char* const kProtocolOrderKLineFirst;
extern const char* const kProtocolOrderCanFirst;

std::string protocolName(char id);

// Adaptive timing of the adapter (ATAT). The adapter measures how fast the
// control units answer and shortens its own wait accordingly. ATAT1 is the
// power-on default and careful; ATAT2 waits less and so gets more requests
// through per second, at the risk of missing a slow control unit.
enum class AdaptiveTiming {
    Standard,     // ATAT1
    Aggressive,   // ATAT2, back to ATAT1 if refused or if answers go missing
    Auto,         // ATAT2 on STN chips (OBDLink and others), ATAT1 on everything else
};

class Elm327 {
public:
    void setLog(LogFn log) { log_ = std::move(log); }
    // Every timeout is multiplied by this factor. Sluggish clones need more
    // time; the gradation between short and long waits is preserved because
    // not everything is set to the same value.
    void setTimeoutScale(double scale) { timeoutScale_ = scale > 0.0 ? scale : 1.0; }
    // Log every sent line together with its response.
    void setVerbose(bool verbose) { verbose_ = verbose; }
    // Which adaptive timing connect sets up. Takes effect on the next connect.
    void setAdaptiveTiming(AdaptiveTiming mode) { adaptiveTiming_ = mode; }
    AdaptiveTiming adaptiveTiming() const { return adaptiveTiming_; }

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

    // The transport in use, or nullptr. Elm327 keeps ownership.
    ITransport* transport() const { return transport_.get(); }
    // Replaces the transport by whatever `wrap` builds around it, without
    // reconnecting: this is how a recording is switched on and off in a
    // running session. `wrap` receives the current transport and returns the
    // one to use from now on. Call it between two requests, from the thread
    // that uses this Elm327.
    void wrapTransport(
        const std::function<std::unique_ptr<ITransport>(std::unique_ptr<ITransport>)>& wrap);

    // The adaptive timing actually in effect after connect: 1 (ATAT1) or 2
    // (ATAT2). 0 before connect, or if the adapter refused both.
    int adaptiveTimingLevel() const { return timingLevel_; }

    // An STN chip (OBDLink and others) instead of an ELM327 or a clone of
    // it. Found out during connect with the read-only STI, which an STN
    // answers with its firmware ("STN1130 v5.6.19") and anything else with
    // "?".
    bool isStn() const { return stn_; }
    const std::string& stnId() const { return stnId_; }

    // Protocol number as the adapter counts ('1' to '9'), '0' if unknown.
    char protocolId() const { return protocolId_; }
    bool isCan() const { return protocolId_ >= '6' && protocolId_ <= '9'; }

    // Headers on (ATH1) keep the sender address of every response, so the
    // results can say which control unit replied when several answer at
    // once (engine 7E8, transmission 7E9, ...). Off by default. Returns
    // false if the adapter refused the command.
    bool setHeaders(bool on);
    bool headersOn() const { return headers_; }
    HeaderFormat headerFormat() const {
        return headers_ ? headerFormatFor(protocolId_) : HeaderFormat::None;
    }

private:
    ElmResult at(const std::string& command, int timeoutMs = 5000);
    void drainGreeting();
    void detectStn();
    void applyAggressiveTiming();

    std::unique_ptr<ITransport> transport_;
    LogFn log_;
    double timeoutScale_ = 1.0;
    bool verbose_ = true;
    AdaptiveTiming adaptiveTiming_ = AdaptiveTiming::Standard;
    int timingLevel_ = 0;
    bool stn_ = false;
    std::string stnId_;
    std::string version_ = "?";
    std::string protocolText_ = L("unbekannt", "unknown");
    char protocolId_ = '0';
    bool headers_ = false;
    bool connected_ = false;
};

// Helper functions, also usable from tests.
std::vector<int> hexToBytes(const std::string& text);
// Splits the adapter lines into messages. With headers the sender of each
// message is kept in ElmResult::sources and multi-frame CAN responses
// (ISO-TP first and consecutive frames) are put back together per sender.
ElmResult decodeMessages(const std::vector<std::string>& lines, int expectedService,
                         HeaderFormat headers = HeaderFormat::None);

// One complete message as it came off the bus, service byte first, and who
// sent it (empty while headers are off).
struct ElmMessage {
    std::string source;
    std::vector<int> bytes;
};

// The first half of decodeMessages: the adapter lines put back together into
// whole messages (ISO-TP reassembly included), without filtering by service.
// UDS needs this to see negative responses such as 7F 22 78 next to the
// positive one. Status lines like "NO DATA" are skipped.
std::vector<ElmMessage> splitMessages(const std::vector<std::string>& lines,
                                      HeaderFormat headers = HeaderFormat::None);

}  // namespace caruspex
