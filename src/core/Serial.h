// Serielle Schnittstelle unter Windows (USB-Adapter, Bluetooth-SPP, KKL-Kabel).
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

// Alle COM-Ports mit Beschreibung und USB-Kennung.
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
    // Liest, was innerhalb von timeoutMs anliegt. Leerer String = nichts da.
    std::string read(int timeoutMs = 100);
    void flushInput();

    // Zieht die Sendeleitung dauerhaft auf Null. Wird fuer die 5-Baud-
    // Weckadresse von KWP1281 gebraucht, die von Hand getaktet wird.
    bool setBreak(bool active);
    // Baudrate im laufenden Betrieb aendern, ohne den Port zu schliessen.
    bool changeBaud(int baud);

    const std::string& device() const { return device_; }
    int baud() const { return baud_; }

private:
    void* handle_ = nullptr;   // HANDLE, ohne windows.h im Header
    std::string device_;
    int baud_ = 0;
};

}  // namespace caruspex
