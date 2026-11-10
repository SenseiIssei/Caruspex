// SocketCAN, the CAN layer of Linux: PEAK, Kvaser, CANable (candleLight),
// Raspberry Pi CAN hats and every other adapter with a kernel driver.

#include "core/CanBus.h"

#include "core/Lang.h"

#ifdef __linux__
#include <dirent.h>
#include <linux/can.h>
#include <linux/can/raw.h>
#include <net/if.h>
#include <poll.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <unistd.h>

#include <cstring>
#include <fstream>
#endif

namespace caruspex {

#ifdef __linux__
namespace {

class SocketCanLink : public CanLink {
public:
    explicit SocketCanLink(int s, std::string interface) : socket_(s), interface_(std::move(interface)) {}
    ~SocketCanLink() override { ::close(socket_); }

    bool send(const CanFrame& frame) override {
        can_frame f{};
        f.can_id = frame.extended ? (frame.id | CAN_EFF_FLAG) : (frame.id & CAN_SFF_MASK);
        f.can_dlc = static_cast<__u8>(std::min<std::size_t>(frame.data.size(), 8));
        std::memcpy(f.data, frame.data.data(), f.can_dlc);
        return ::write(socket_, &f, sizeof(f)) == static_cast<ssize_t>(sizeof(f));
    }

    bool receive(CanFrame& frame, int timeoutMs) override {
        pollfd p{socket_, POLLIN, 0};
        if (::poll(&p, 1, timeoutMs < 0 ? 0 : timeoutMs) <= 0) return false;
        can_frame f{};
        if (::read(socket_, &f, sizeof(f)) != static_cast<ssize_t>(sizeof(f))) return false;
        if (f.can_id & (CAN_ERR_FLAG | CAN_RTR_FLAG)) return false;
        frame.extended = (f.can_id & CAN_EFF_FLAG) != 0;
        frame.id = frame.extended ? (f.can_id & CAN_EFF_MASK) : (f.can_id & CAN_SFF_MASK);
        frame.data.assign(f.data, f.data + std::min<int>(f.can_dlc, 8));
        return true;
    }

    std::string name() const override { return "SocketCAN " + interface_; }

private:
    int socket_;
    std::string interface_;
};

}  // namespace

std::vector<std::string> socketCanInterfaces() {
    std::vector<std::string> out;
    DIR* dir = ::opendir("/sys/class/net");
    if (!dir) return out;
    while (dirent* e = ::readdir(dir)) {
        const std::string name = e->d_name;
        if (name == "." || name == "..") continue;
        std::ifstream type("/sys/class/net/" + name + "/type");
        int value = 0;
        if (type >> value && value == 280) out.push_back(name);   // ARPHRD_CAN
    }
    ::closedir(dir);
    return out;
}

std::unique_ptr<CanLink> openSocketCan(const std::string& interface, std::string& error) {
    const int s = ::socket(PF_CAN, SOCK_RAW, CAN_RAW);
    if (s < 0) {
        error = L("SocketCAN ist nicht verfügbar.", "SocketCAN is not available.");
        return nullptr;
    }
    ifreq ifr{};
    std::strncpy(ifr.ifr_name, interface.c_str(), IFNAMSIZ - 1);
    if (::ioctl(s, SIOCGIFINDEX, &ifr) < 0) {
        ::close(s);
        error = L("Die CAN-Schnittstelle gibt es nicht: ", "No such CAN interface: ") + interface +
                L(". Erst einrichten: ip link set can0 up type can bitrate 500000",
                  ". Set it up first: ip link set can0 up type can bitrate 500000");
        return nullptr;
    }
    sockaddr_can address{};
    address.can_family = AF_CAN;
    address.can_ifindex = ifr.ifr_ifindex;
    if (::bind(s, reinterpret_cast<sockaddr*>(&address), sizeof(address)) < 0) {
        ::close(s);
        error = L("Die CAN-Schnittstelle lässt sich nicht öffnen: ", "The CAN interface cannot be opened: ") + interface;
        return nullptr;
    }
    return std::make_unique<SocketCanLink>(s, interface);
}

#else

std::vector<std::string> socketCanInterfaces() { return {}; }

std::unique_ptr<CanLink> openSocketCan(const std::string&, std::string& error) {
    error = L("SocketCAN gibt es nur unter Linux.", "SocketCAN exists on Linux only.");
    return nullptr;
}

#endif

}  // namespace caruspex
