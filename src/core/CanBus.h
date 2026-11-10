// Interfaces that hand over raw CAN frames instead of speaking ELM327:
// SAE J2534 PassThru devices, SLCAN/Lawicel sticks (CANable, USBtin,
// CANUSB) and SocketCAN on Linux. CanElm puts an ELM327 front end on top of
// them, with the ISO-TP framing (ISO 15765-2) done here, so OBD, UDS, the
// BMW addressing and listening to broadcast frames work through every one
// of them exactly as through an ELM327.
#pragma once

#include <cstdint>
#include <memory>
#include <string>
#include <vector>

#include "core/Transport.h"

namespace caruspex {

struct CanFrame {
    std::uint32_t id = 0;
    bool extended = false;          // 29-bit id
    std::vector<std::uint8_t> data; // 0 to 8 bytes
};

// One CAN interface at a fixed bit rate.
class CanLink {
public:
    virtual ~CanLink() = default;
    virtual bool send(const CanFrame& frame) = 0;
    // The next frame within timeoutMs; false if none came.
    virtual bool receive(CanFrame& frame, int timeoutMs) = 0;
    virtual std::string name() const = 0;
    // Battery voltage at the socket if the interface can measure it, else < 0.
    virtual double batteryVoltage() { return -1.0; }
};

// An ELM327 as far as Caruspex uses one, on top of a CanLink: AT commands,
// OBD and UDS requests with ISO-TP (11 and 29 bit, functional and
// physical), raw frames with auto formatting off (BMW 6F1 addressing) and
// the bus monitor (ATMA) with the receive filter.
class CanElm : public ITransport {
public:
    CanElm(std::unique_ptr<CanLink> link, std::string name, int bitrate);
    bool write(const std::string& data) override;
    std::string read(int timeoutMs) override;
    void flushInput() override;
    std::string name() const override { return name_; }

private:
    std::vector<std::string> handle(const std::string& command);
    std::vector<std::string> at(const std::string& command);
    std::vector<std::string> request(const std::vector<std::uint8_t>& bytes);
    std::vector<std::string> raw(const std::vector<std::uint8_t>& bytes);
    bool accepts(const CanFrame& frame, bool functional) const;
    std::uint32_t flowControlId(const CanFrame& response) const;
    std::string line(const CanFrame& frame, bool withPci) const;
    std::string hex(const std::vector<std::uint8_t>& bytes) const;
    void reset();

    std::unique_ptr<CanLink> link_;
    std::string name_;
    int bitrate_;
    std::string pending_;
    bool echo_ = true, linefeeds_ = false, spaces_ = true, headers_ = false, autoFormat_ = true;
    bool extended_ = false;            // ATSP7/9: 29-bit ids
    std::uint32_t header_ = 0x7DF;     // ATSH (with ATCP on top for 29 bit)
    std::uint32_t priority_ = 0x18;
    std::int64_t filter_ = -1;         // ATCRA
    std::int64_t flowHeader_ = -1;     // ATFCSH
    bool monitoring_ = false;
};

// SLCAN / Lawicel ASCII protocol over a serial port: "S6\r" (500 kbit),
// "O\r" (open), "t7DF8020100..." (11-bit frame), "T18DB33F1..." (29-bit).
class SlcanLink : public CanLink {
public:
    explicit SlcanLink(std::unique_ptr<ITransport> serial);
    ~SlcanLink() override;
    // Sets the bit rate and opens the channel. False with the reason.
    bool open(int bitrate, std::string& error);
    bool send(const CanFrame& frame) override;
    bool receive(CanFrame& frame, int timeoutMs) override;
    std::string name() const override;

private:
    bool command(const std::string& text, std::string* reply = nullptr);
    std::unique_ptr<ITransport> serial_;
    std::string buffer_;
    std::string version_;
    bool open_ = false;
};

// Whether a serial port speaks SLCAN (answers "V" with a version).
bool probeSlcan(ITransport& serial, std::string& version);

// --- SocketCAN (Linux) ------------------------------------------------------

// CAN network interfaces of the system ("can0", "vcan0"), empty elsewhere.
std::vector<std::string> socketCanInterfaces();
// The interface must already be up at the right bit rate
// (ip link set can0 up type can bitrate 500000).
std::unique_ptr<CanLink> openSocketCan(const std::string& interface, std::string& error);

// --- SAE J2534 PassThru (Windows) ---------------------------------------------

struct J2534Device {
    std::string name;      // as registered, "OpenPort 2.0 J2534 ISO/CAN/VPW/PWM"
    std::string vendor;
    std::string library;   // path of the driver DLL
    bool usable = true;    // false: a 32-bit driver, which a 64-bit program cannot load
};

// The registered PassThru drivers (HKLM\SOFTWARE\PassThruSupport.04.04).
std::vector<J2534Device> j2534Devices();
// Opens the device through its driver DLL and connects CAN at the bit rate.
std::unique_ptr<CanLink> openJ2534(const std::string& library, int bitrate, std::string& error);

// --- Choosing -------------------------------------------------------------------

// "slcan:COM5", "slcan:/dev/ttyACM0", "socketcan:can0", "j2534:<dll path or
// device name>"; an optional "@250000" sets the bit rate (500000 if left out).
// Returns the ELM front end, or nullptr with the reason in error.
std::unique_ptr<ITransport> openCanInterface(const std::string& spec, std::string& error);

}  // namespace caruspex
