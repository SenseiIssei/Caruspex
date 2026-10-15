#include "core/Transport.h"

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <arpa/inet.h>
#include <fcntl.h>
#include <netdb.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/types.h>
#include <unistd.h>
#endif

#include <cerrno>
#include <cstring>

#include "core/Lang.h"

namespace caruspex {
namespace {

#ifdef _WIN32
using RawSocket = SOCKET;
constexpr RawSocket kInvalid = INVALID_SOCKET;

bool ensureNetwork() {
    static const bool ready = [] {
        WSADATA wsa{};
        return WSAStartup(MAKEWORD(2, 2), &wsa) == 0;
    }();
    return ready;
}

void closeSocket(RawSocket s) { ::closesocket(s); }

void setReceiveTimeout(RawSocket s, int milliseconds) {
    DWORD value = static_cast<DWORD>(milliseconds);
    ::setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, reinterpret_cast<const char*>(&value),
                 sizeof(value));
}

void setBlocking(RawSocket s, bool blocking) {
    u_long mode = blocking ? 0 : 1;
    ::ioctlsocket(s, FIONBIO, &mode);
}
#else
using RawSocket = int;
constexpr RawSocket kInvalid = -1;

bool ensureNetwork() { return true; }

void closeSocket(RawSocket s) { ::close(s); }

void setReceiveTimeout(RawSocket s, int milliseconds) {
    timeval value{};
    value.tv_sec = milliseconds / 1000;
    value.tv_usec = (milliseconds % 1000) * 1000;
    ::setsockopt(s, SOL_SOCKET, SO_RCVTIMEO, &value, sizeof(value));
}

void setBlocking(RawSocket s, bool blocking) {
    int flags = ::fcntl(s, F_GETFL, 0);
    if (flags < 0) return;
    ::fcntl(s, F_SETFL, blocking ? (flags & ~O_NONBLOCK) : (flags | O_NONBLOCK));
}
#endif

constexpr unsigned long long kNoSocket = ~0ull;

RawSocket toRaw(unsigned long long handle) { return static_cast<RawSocket>(handle); }

}  // namespace

bool SerialTransport::open(const std::string& device, int baud, std::string& error) {
    return port_.open(device, baud, error);
}

std::string SerialTransport::name() const {
    return port_.device() + " @ " + std::to_string(port_.baud()) + L(" Baud", " baud");
}

TcpTransport::~TcpTransport() {
    if (socket_ != kNoSocket) closeSocket(toRaw(socket_));
}

bool TcpTransport::open(const std::string& host, int port, std::string& error) {
    if (!ensureNetwork()) {
        error = L("Netzwerk laesst sich nicht starten", "Cannot start networking");
        return false;
    }
    addrinfo hints{};
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    addrinfo* result = nullptr;
    const std::string service = std::to_string(port);
    if (::getaddrinfo(host.c_str(), service.c_str(), &hints, &result) != 0 || !result) {
        error = L("Adresse ", "Cannot resolve address ") + host + L(" nicht aufloesbar", "");
        return false;
    }

    RawSocket handle = kInvalid;
    for (addrinfo* it = result; it; it = it->ai_next) {
        handle = ::socket(it->ai_family, it->ai_socktype, it->ai_protocol);
        if (handle == kInvalid) continue;
        if (::connect(handle, it->ai_addr, static_cast<int>(it->ai_addrlen)) == 0) break;
        closeSocket(handle);
        handle = kInvalid;
    }
    ::freeaddrinfo(result);

    if (handle == kInvalid) {
        error = L("Keine Verbindung zu ", "No connection to ") + host + ":" + service +
                L(" (ist das WLAN des Adapters verbunden?)",
                  " (are you connected to the adapter's Wi-Fi?)");
        return false;
    }

    int one = 1;
    ::setsockopt(handle, IPPROTO_TCP, TCP_NODELAY, reinterpret_cast<const char*>(&one),
                 sizeof(one));
    socket_ = static_cast<unsigned long long>(handle);
    name_ = host + ":" + service + L(" (WLAN)", " (Wi-Fi)");
    return true;
}

bool TcpTransport::write(const std::string& data) {
    if (socket_ == kNoSocket) return false;
    const auto sent = ::send(toRaw(socket_), data.data(), static_cast<int>(data.size()), 0);
    return sent == static_cast<decltype(sent)>(data.size());
}

std::string TcpTransport::read(int timeoutMs) {
    if (socket_ == kNoSocket) return {};
    const RawSocket s = toRaw(socket_);
    setReceiveTimeout(s, timeoutMs);
    char buffer[512];
    const auto got = ::recv(s, buffer, sizeof(buffer), 0);
    if (got <= 0) return {};
    return std::string(buffer, static_cast<size_t>(got));
}

void TcpTransport::flushInput() {
    if (socket_ == kNoSocket) return;
    const RawSocket s = toRaw(socket_);
    setBlocking(s, false);
    char buffer[512];
    while (::recv(s, buffer, sizeof(buffer), 0) > 0) {
    }
    setBlocking(s, true);
}

}  // namespace caruspex
