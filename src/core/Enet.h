// BMW ENET: diagnosis over Ethernet with HSFZ, the protocol of the ENET cable
// for BMW and Mini from the F series on (about 2007 to today).
//
// The OBD socket of these cars carries Ethernet on pins 3/11 and 12/13; the
// cable bridges pin 8 to switch the gateway's Ethernet on. The car gives
// itself an automatic address (169.254.x.x), the computer does the same when
// no DHCP answers. Port 6811 answers a broadcast with the VIN, port 6801
// takes the diagnosis.
//
// HSFZ frame: 4-byte length (big endian, counts the payload), 0x00, control
// byte, payload. Diagnosis (control 0x01) carries source address, target
// address and the UDS bytes; the car first acknowledges every request
// (0x02, the request echoed), then answers. It also asks "still there?"
// (0x12), which the tester answers with 0x13.
// Protocol facts after the open source EdiabasLib (EdInterfaceEnet).
//
// So that everything else in Caruspex works unchanged, EnetElm puts an
// ELM327 front end on top: OBD requests go to the engine (DME, address 12),
// BMW style extended addressing (ATSH 6F1, raw frames) goes to the unit in
// the first byte. To the rest of the program the car looks like an ELM327 on
// CAN 11 bit 500 kbit.
#pragma once

#include <cstdint>
#include <memory>
#include <string>
#include <vector>

#include "core/Transport.h"

namespace caruspex {

using ByteVector = std::vector<std::uint8_t>;

namespace hsfz {

constexpr int kDiagnosticPort = 6801;
constexpr int kIdentPort = 6811;
constexpr int kTester = 0xF4;        // ENET testers use F4, D-CAN uses F1
constexpr int kFunctional = 0xDF;    // all units
constexpr int kEngine = 0x12;        // DME
constexpr int kGearbox = 0x18;       // EGS

enum Control : int {
    Diagnostic = 0x01,
    Ack = 0x02,
    Ident = 0x11,
    AliveCheck = 0x12,
    AliveReply = 0x13,
    Nack = 0xFF,
};

struct Frame {
    int control = 0;
    ByteVector payload;
};

ByteVector encode(int control, const ByteVector& payload);

// Cuts a byte stream into frames.
class Parser {
public:
    void feed(const std::uint8_t* data, std::size_t size);
    bool next(Frame& frame);   // false until a whole frame is there
    bool broken() const { return broken_; }   // a length no car would send

private:
    ByteVector buffer_;
    bool broken_ = false;
};

struct VehicleIdent {
    std::string address;      // IP of the gateway
    std::string vin;
    std::string mac;
    int diagAddress = -1;     // the gateway's diagnostic address, usually 0x10
};

// The answer to the identification broadcast:
// header, "DIAGADR10BMWMAC001A37xxxxxxBMWVINWBA...". False if it is none.
bool parseIdent(const ByteVector& datagram, VehicleIdent& out);

}  // namespace hsfz

// The bytes under the session: TCP in the car, a fake in the tests.
class ByteStream {
public:
    virtual ~ByteStream() = default;
    virtual bool send(const ByteVector& data) = 0;
    // Bytes read, 0 after the timeout, -1 when the connection is gone.
    virtual int receive(std::uint8_t* buffer, int size, int timeoutMs) = 0;
};

struct DiagAnswer {
    int source = 0;
    ByteVector data;   // UDS bytes, service byte first
};

// One UDS request to one unit (or the functional address) and its final
// answers; "response pending" (7F xx 78) is waited out inside.
class DiagLink {
public:
    virtual ~DiagLink() = default;
    virtual bool request(int target, const ByteVector& data, int timeoutMs,
                         std::vector<DiagAnswer>& answers, std::string& error) = 0;
};

class HsfzSession : public DiagLink {
public:
    explicit HsfzSession(std::unique_ptr<ByteStream> stream, int tester = hsfz::kTester);
    bool request(int target, const ByteVector& data, int timeoutMs, std::vector<DiagAnswer>& answers,
                 std::string& error) override;
    bool closed() const { return closed_; }

private:
    bool readFrame(hsfz::Frame& frame, std::uint64_t deadline);
    void answerAliveCheck();

    std::unique_ptr<ByteStream> stream_;
    hsfz::Parser parser_;
    int tester_;
    bool closed_ = false;
};

// An ELM327 as far as Caruspex uses one, on top of a DiagLink.
class EnetElm : public ITransport {
public:
    EnetElm(std::unique_ptr<DiagLink> link, std::string name);
    bool write(const std::string& data) override;
    std::string read(int timeoutMs) override;
    void flushInput() override { pending_.clear(); }
    std::string name() const override { return name_; }

private:
    std::vector<std::string> handle(const std::string& command);
    std::vector<std::string> at(const std::string& command);
    std::vector<std::string> obd(const ByteVector& bytes);
    std::vector<std::string> raw(const ByteVector& bytes);
    void reset();
    std::string hex(const ByteVector& bytes) const;
    std::string idText(int id) const;

    std::unique_ptr<DiagLink> link_;
    std::string name_;
    std::string pending_;
    bool echo_ = true, linefeeds_ = false, spaces_ = true, headers_ = false, autoFormat_ = true;
    int header_ = 0x7DF;
    int receiveFilter_ = -1;
    int rawTarget_ = -1;                  // unit whose long answer waits for flow control
    std::vector<ByteVector> rawPending_;  // its consecutive frames
};

// Looks for cars on the local network (identification broadcast on 6811,
// to 255.255.255.255 and the link-local 169.254.255.255).
std::vector<hsfz::VehicleIdent> discoverEnet(int timeoutMs, std::string& error);

// Connects to the car: host empty or "auto" looks for it first. Returns the
// ELM front end, or nullptr with the reason in error.
std::unique_ptr<ITransport> openEnet(const std::string& host, std::string& error,
                                     hsfz::VehicleIdent* found = nullptr);

}  // namespace caruspex
