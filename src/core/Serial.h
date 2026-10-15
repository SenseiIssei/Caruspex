// Serial port under Windows (USB adapters, Bluetooth SPP, KKL cable).
#pragma once

#include <string>
#include <vector>

namespace caruspex {

struct PortInfo {
    std::string device;       // "COM5"
    std::string description;  // "USB-SERIAL CH340 (COM5)"
    std::string hardwareId;   // "USB\\VID_1A86&PID_7523\\..."
    unsigned vid = 0;
    unsigned pid = 0;
    bool likelyAdapter = false;
    std::string chipGuess;    // "CH340", "FTDI FT232R", ...
};

// All COM ports with description and USB ID.
std::vector<PortInfo> enumeratePorts();

class Serial {
public:
    Serial() = default;
    ~Serial();
    Serial(const Serial&) = delete;
    Serial& operator=(const Serial&) = delete;

    bool open(const std::string& device, int baud, std::string& error);
    void close();
    bool isOpen() const { return handle_ != nullptr; }

    bool write(const std::string& data);
    // Reads whatever arrives within timeoutMs. Empty string = nothing there.
    std::string read(int timeoutMs = 100);
    void flushInput();

    // Pulls the transmit line permanently low. Needed for the 5-baud
    // wake-up address of KWP1281, which is clocked out by hand.
    bool setBreak(bool active);
    // Change the baud rate on the fly without closing the port.
    bool changeBaud(int baud);

    const std::string& device() const { return device_; }
    int baud() const { return baud_; }

private:
    void* handle_ = nullptr;   // HANDLE, without windows.h in the header
    std::string device_;
    int baud_ = 0;
};

}  // namespace caruspex
