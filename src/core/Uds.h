// Unified Diagnostic Services (ISO 14229) over an ELM327, read-only plus
// clearing fault codes.
//
// Almost every car since about 2008 speaks UDS on CAN, not only the engine but
// also ABS, airbag, gearbox, instrument cluster, climate and the body units.
// Standard OBD-II (modes 01 to 0A) only reaches the emission related units;
// UDS reaches all of them, as long as the gateway lets the diagnostic socket
// talk to them. Caruspex only reads: fault memory, identification. The one
// writing service is clearing faults, with the same warnings as everywhere.
//
// The ELM327 is told whom to talk to with ATSH (request id) and ATCRA
// (response id); for 29-bit ids ATCP/ATSH and ATCRA work the same way.
// BMW style extended addressing (one target byte in front of every frame)
// is done in raw mode (ATCAF0) with the ISO-TP framing in Caruspex itself,
// because many ELM327 clones handle ATCEA badly or not at all.
// Afterwards the default OBD addressing is restored.
#pragma once

#include <cstdint>
#include <functional>
#include <string>
#include <vector>

#include "core/Elm327.h"
#include "data/EcuAddresses.h"

namespace caruspex {

// One fault from service 19: the two-byte SAE code plus the failure type
// byte (what kind of fault) and the status byte (how current it is).
struct UdsDtc {
    std::string code;        // "C0035", "B1000", "U0100"
    int failureType = 0;     // third byte, SAE J2012-DA failure type
    std::string failureText; // "Kurzschluss nach Masse" / "circuit short to ground"
    int status = 0;          // ISO 14229 status byte
    bool testFailed = false;          // bit 0: failing right now
    bool pending = false;             // bit 2
    bool confirmed = false;           // bit 3: stored
    bool warningLamp = false;         // bit 7
    std::string text;        // description from the DTC database
};

// What an ECU says about itself (service 22), empty where it does not answer.
struct UdsEcuInfo {
    std::string partNumber;       // F187 spare part number
    std::string softwareVersion;  // F189 or F195
    std::string hardwareVersion;  // F191 or F193
    std::string systemName;       // F197
    std::string serial;           // F18C
    std::string vin;              // F190
};

struct UdsEcu {
    EcuDefinition definition;
    bool present = false;          // answered at all
    bool dtcsRead = false;         // service 19 worked
    UdsEcuInfo info;
    std::vector<UdsDtc> dtcs;
    std::string note;              // plain text if something went wrong, active language
};

class Uds {
public:
    explicit Uds(Elm327& elm);
    ~Uds();   // restores the default OBD addressing

    // Points the adapter at one ECU. False if the adapter refused the setup.
    bool select(const EcuDefinition& ecu, std::string& error);
    // Back to the functional OBD address (7DF/7E8 or 18DB33F1).
    void restoreDefault();

    // 3E 00. True if the selected ECU answered.
    bool testerPresent();
    // 19 02 with the given status mask (default: everything the ECU reports).
    std::vector<UdsDtc> readDtcs(std::string& error, int statusMask = 0xFF);
    // 22 F187, F189, F191, F197, F18C, F190, each optional.
    UdsEcuInfo readInfo();
    // 14 FF FF FF: clears all groups of the selected ECU.
    bool clearDtcs(std::string& error);

    // Raw request/response for one service, handles 7F xx 78 (still busy).
    // Returns the positive response without the service id byte.
    bool request(const std::vector<int>& bytes, std::vector<int>& response, std::string& error,
                 int timeoutMs = 2000);

private:
    Elm327& elm_;
    bool addressed_ = false;
    int extended_ = -1;   // target byte with BMW style addressing, else -1
};

// One request with BMW style extended addressing, the framing done here:
// the ISO-TP frames as the raw lines of the adapter (CAN auto formatting
// off). Exposed for the tests. target is the unit's address, the answer has
// to start with tester. Requests up to six bytes (one single frame).
struct ExtendedReply {
    bool answered = false;
    std::vector<int> payload;   // reassembled ISO-TP payload, service byte first
    std::string error;
};
ExtendedReply extendedRequest(Elm327& elm, int target, int tester, const std::vector<int>& bytes,
                              int timeoutMs);

// Asks every ECU of the list once: is it there, what is it, what does it
// have stored. progress(index, total, name) is called before each ECU;
// cancelled() is checked between ECUs. The adapter ends up on the default
// OBD addressing again.
std::vector<UdsEcu> scanEcus(Elm327& elm, const std::vector<EcuDefinition>& ecus,
                             const std::function<void(int, int, const std::string&)>& progress,
                             const std::function<bool()>& cancelled);

// Decodes the three-byte DTC of service 19 ("C0035" plus failure type).
UdsDtc decodeUdsDtc(int b1, int b2, int b3, int status);

// BMW stores its own codes in the three bytes, shown as six hex digits the
// way ISTA does ("480A10"), without an SAE meaning or failure type.
UdsDtc decodeBmwDtc(int b1, int b2, int b3, int status);

}  // namespace caruspex
