// Serielle Schnittstelle unter Linux und macOS.
#include "core/Serial.h"

#include <dirent.h>
#include <fcntl.h>
#include <poll.h>
#include <termios.h>
#include <unistd.h>

#include <algorithm>
#include <cctype>
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <string>

#include "core/Timing.h"

namespace caruspex {
namespace {

struct ChipHint {
    unsigned vid;
    unsigned pid;
    const char* name;
};

const ChipHint kChips[] = {
    {0x0403, 0x6001, "FTDI FT232R"},   {0x0403, 0x6015, "FTDI FT231X"},
    {0x0403, 0x6010, "FTDI FT2232"},   {0x0403, 0xFA20, "FTDI (Ross-Tech HEX)"},
    {0x1A86, 0x7523, "CH340"},         {0x1A86, 0x5523, "CH341"},
    {0x10C4, 0xEA60, "CP2102"},        {0x067B, 0x2303, "PL2303"},
};

// Namen, unter denen USB-Adapter und Bluetooth-Ports auftauchen.
bool interestingName(const std::string& name) {
    static const char* kPrefixes[] = {"ttyUSB", "ttyACM", "rfcomm",
                                      "cu.usbserial", "cu.usbmodem", "cu.SLAB",
                                      "cu.wchusbserial", "cu.Bluetooth", "tty.usbserial"};
    for (const char* prefix : kPrefixes) {
        if (name.rfind(prefix, 0) == 0) return true;
    }
    return false;
}

// Unter Linux liegen VID und PID im sysfs neben dem Geraet.
bool readUsbIds(const std::string& name, unsigned& vid, unsigned& pid) {
    const std::string base = "/sys/class/tty/" + name + "/device/../";
    auto readHex = [](const std::string& path, unsigned& value) {
        std::ifstream file(path);
        if (!file) return false;
        std::string text;
        file >> text;
        if (text.empty()) return false;
        value = static_cast<unsigned>(std::strtoul(text.c_str(), nullptr, 16));
        return true;
    };
    return readHex(base + "idVendor", vid) && readHex(base + "idProduct", pid);
}

speed_t toSpeed(int baud) {
    switch (baud) {
        case 9600: return B9600;
        case 19200: return B19200;
        case 38400: return B38400;
        case 57600: return B57600;
        case 115200: return B115200;
        case 230400: return B230400;
#ifdef B460800
        case 460800: return B460800;
#endif
#ifdef B500000
        case 500000: return B500000;
#endif
#ifdef B921600
        case 921600: return B921600;
#endif
        default: return 0;
    }
}

}  // namespace

std::vector<PortInfo> enumeratePorts() {
    std::vector<PortInfo> out;
    DIR* dir = ::opendir("/dev");
    if (!dir) return out;

    while (dirent* entry = ::readdir(dir)) {
        const std::string name = entry->d_name;
        if (!interestingName(name)) continue;

        PortInfo info;
        info.device = "/dev/" + name;
        info.description = name;
        if (readUsbIds(name, info.vid, info.pid)) {
            char buffer[32];
            std::snprintf(buffer, sizeof(buffer), "USB VID_%04X PID_%04X", info.vid, info.pid);
            info.hardwareId = buffer;
            for (const ChipHint& chip : kChips) {
                if (chip.vid == info.vid && chip.pid == info.pid) {
                    info.chipGuess = chip.name;
                    break;
                }
            }
        }
        info.likelyAdapter = true;   // nach dem Namensfilter bleibt nur Passendes uebrig
        out.push_back(std::move(info));
    }
    ::closedir(dir);

    std::sort(out.begin(), out.end(),
              [](const PortInfo& a, const PortInfo& b) { return a.device < b.device; });
    return out;
}

Serial::~Serial() { close(); }

bool Serial::open(const std::string& device, int baud, std::string& error) {
    close();
    const int fd = ::open(device.c_str(), O_RDWR | O_NOCTTY | O_NONBLOCK);
    if (fd < 0) {
        error = device + " laesst sich nicht oeffnen: " + std::strerror(errno);
        if (errno == EACCES) {
            error += " (fehlt die Gruppe dialout? sudo usermod -aG dialout $USER)";
        }
        return false;
    }

    termios settings{};
    if (::tcgetattr(fd, &settings) != 0) {
        ::close(fd);
        error = "tcgetattr fehlgeschlagen: " + std::string(std::strerror(errno));
        return false;
    }

    ::cfmakeraw(&settings);
    settings.c_cflag |= CLOCAL | CREAD;
    settings.c_cflag &= ~static_cast<tcflag_t>(CRTSCTS);
    settings.c_cflag &= ~static_cast<tcflag_t>(CSTOPB);
    settings.c_cflag &= ~static_cast<tcflag_t>(PARENB);
    settings.c_cc[VMIN] = 0;
    settings.c_cc[VTIME] = 1;   // Zehntelsekunden

    const speed_t speed = toSpeed(baud);
    if (speed == 0) {
        ::close(fd);
        error = "Baudrate " + std::to_string(baud) + " wird nicht unterstuetzt";
        return false;
    }
    ::cfsetispeed(&settings, speed);
    ::cfsetospeed(&settings, speed);

    if (::tcsetattr(fd, TCSANOW, &settings) != 0) {
        ::close(fd);
        error = "Schnittstelle laesst sich nicht einstellen: " +
                std::string(std::strerror(errno));
        return false;
    }
    ::tcflush(fd, TCIOFLUSH);

    // Der Deskriptor wird als Zeiger abgelegt, damit der Header
    // betriebssystemfrei bleibt. Plus eins, damit fd 0 nicht wie leer aussieht.
    handle_ = reinterpret_cast<void*>(static_cast<intptr_t>(fd) + 1);
    device_ = device;
    baud_ = baud;
    sleepMs(150);   // manche Adapter brauchen einen Moment nach dem Oeffnen
    return true;
}

void Serial::close() {
    if (handle_) {
        ::close(static_cast<int>(reinterpret_cast<intptr_t>(handle_) - 1));
        handle_ = nullptr;
    }
    device_.clear();
    baud_ = 0;
}

bool Serial::write(const std::string& data) {
    if (!handle_) return false;
    const int fd = static_cast<int>(reinterpret_cast<intptr_t>(handle_) - 1);
    size_t written = 0;
    while (written < data.size()) {
        const ssize_t chunk = ::write(fd, data.data() + written, data.size() - written);
        if (chunk < 0) {
            if (errno == EAGAIN || errno == EINTR) continue;
            return false;
        }
        written += static_cast<size_t>(chunk);
    }
    return true;
}

std::string Serial::read(int timeoutMs) {
    if (!handle_) return {};
    const int fd = static_cast<int>(reinterpret_cast<intptr_t>(handle_) - 1);

    pollfd waiting{};
    waiting.fd = fd;
    waiting.events = POLLIN;
    const int ready = ::poll(&waiting, 1, timeoutMs);
    if (ready <= 0) return {};

    char buffer[512];
    const ssize_t got = ::read(fd, buffer, sizeof(buffer));
    if (got <= 0) return {};
    return std::string(buffer, static_cast<size_t>(got));
}

void Serial::flushInput() {
    if (handle_) {
        ::tcflush(static_cast<int>(reinterpret_cast<intptr_t>(handle_) - 1), TCIFLUSH);
    }
}

}  // namespace caruspex
