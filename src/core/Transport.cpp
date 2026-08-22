#include "core/Transport.h"

#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>

#pragma comment(lib, "ws2_32.lib")

namespace caruspex {
namespace {

bool ensureWinsock() {
    static bool ready = [] {
        WSADATA wsa{};
        return WSAStartup(MAKEWORD(2, 2), &wsa) == 0;
    }();
    return ready;
}

}  // namespace

bool SerialTransport::open(const std::string& device, int baud, std::string& error) {
    return port_.open(device, baud, error);
}

std::string SerialTransport::name() const {
    return port_.device() + " @ " + std::to_string(port_.baud()) + " Baud";
}

TcpTransport::~TcpTransport() {
    if (socket_ != ~0ull) closesocket(static_cast<SOCKET>(socket_));
}

bool TcpTransport::open(const std::string& host, int port, std::string& error) {
    if (!ensureWinsock()) {
        error = "Windows-Netzwerk laesst sich nicht starten";
        return false;
    }
    addrinfo hints{};
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    addrinfo* result = nullptr;
    const std::string service = std::to_string(port);
    if (getaddrinfo(host.c_str(), service.c_str(), &hints, &result) != 0 || !result) {
        error = "Adresse " + host + " nicht aufloesbar";
        return false;
    }
    SOCKET s = ::socket(result->ai_family, result->ai_socktype, result->ai_protocol);
    if (s == INVALID_SOCKET) {
        freeaddrinfo(result);
        error = "Socket laesst sich nicht anlegen";
        return false;
    }
    if (::connect(s, result->ai_addr, static_cast<int>(result->ai_addrlen)) == SOCKET_ERROR) {
        freeaddrinfo(result);
        closesocket(s);
        error = "Keine Verbindung zu " + host + ":" + service +
                " (WLAN des Adapters verbunden?)";
        return false;
    }
    freeaddrinfo(result);
    socket_ = static_cast<unsigned long long>(s);
    name_ = host + ":" + service + " (WLAN)";
    return true;
}

bool TcpTransport::write(const std::string& data) {
    if (socket_ == ~0ull) return false;
    const int sent = ::send(static_cast<SOCKET>(socket_), data.data(),
                            static_cast<int>(data.size()), 0);
    return sent == static_cast<int>(data.size());
}

std::string TcpTransport::read(int timeoutMs) {
    if (socket_ == ~0ull) return {};
    SOCKET s = static_cast<SOCKET>(socket_);
    DWORD tv = static_cast<DWORD>(timeoutMs);
    setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, reinterpret_cast<const char*>(&tv), sizeof(tv));
    char buffer[512];
    const int got = ::recv(s, buffer, sizeof(buffer), 0);
    if (got <= 0) return {};
    return std::string(buffer, static_cast<size_t>(got));
}

void TcpTransport::flushInput() {
    if (socket_ == ~0ull) return;
    SOCKET s = static_cast<SOCKET>(socket_);
    u_long nonblocking = 1;
    ioctlsocket(s, FIONBIO, &nonblocking);
    char buffer[512];
    while (::recv(s, buffer, sizeof(buffer), 0) > 0) {
    }
    nonblocking = 0;
    ioctlsocket(s, FIONBIO, &nonblocking);
}

}  // namespace caruspex
