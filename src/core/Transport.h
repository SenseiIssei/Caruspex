// Common interface for serial adapters and Wi-Fi adapters.
#pragma once

#include <memory>
#include <string>

#include "core/Serial.h"

namespace caruspex {

class ITransport {
public:
    virtual ~ITransport() = default;
    virtual bool write(const std::string& data) = 0;
    virtual std::string read(int timeoutMs) = 0;
    virtual void flushInput() = 0;
    virtual std::string name() const = 0;
};

class SerialTransport : public ITransport {
public:
    bool open(const std::string& device, int baud, std::string& error);
    bool write(const std::string& data) override { return port_.write(data); }
    std::string read(int timeoutMs) override { return port_.read(timeoutMs); }
    void flushInput() override { port_.flushInput(); }
    std::string name() const override;

private:
    Serial port_;
};

// Wi-Fi dongle, typically 192.168.0.10:35000. Also handy for testing
// against the bundled simulator on 127.0.0.1.
class TcpTransport : public ITransport {
public:
    ~TcpTransport() override;
    bool open(const std::string& host, int port, std::string& error);
    bool write(const std::string& data) override;
    std::string read(int timeoutMs) override;
    void flushInput() override;
    std::string name() const override { return name_; }

private:
    unsigned long long socket_ = ~0ull;  // SOCKET, without winsock2.h in the header
    std::string name_;
};

}  // namespace caruspex
