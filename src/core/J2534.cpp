// SAE J2534-1 (version 04.04) PassThru devices through their driver DLL.
// Used here in CAN mode (protocol 5): raw frames, the ISO-TP framing is done
// by CanElm like for every other CAN interface.

#include "core/CanBus.h"

#include "core/Lang.h"

#ifdef _WIN32
#include <windows.h>

#include <algorithm>
#include <cstring>
#endif

namespace caruspex {

#ifdef _WIN32
namespace {

constexpr unsigned long kProtocolCan = 5;
constexpr unsigned long kCanIdBoth = 0x800;
constexpr unsigned long kCan29BitId = 0x100;
constexpr unsigned long kTxMsgType = 0x01;   // RxStatus: echo of our own frame
constexpr unsigned long kPassFilter = 1;
constexpr unsigned long kReadVbatt = 0x03;
constexpr long kNoError = 0;

struct PassThruMsg {
    unsigned long ProtocolID;
    unsigned long RxStatus;
    unsigned long TxFlags;
    unsigned long Timestamp;
    unsigned long DataSize;
    unsigned long ExtraDataIndex;
    unsigned char Data[4128];
};

using OpenFn = long(WINAPI*)(void*, unsigned long*);
using CloseFn = long(WINAPI*)(unsigned long);
using ConnectFn = long(WINAPI*)(unsigned long, unsigned long, unsigned long, unsigned long, unsigned long*);
using DisconnectFn = long(WINAPI*)(unsigned long);
using ReadFn = long(WINAPI*)(unsigned long, PassThruMsg*, unsigned long*, unsigned long);
using WriteFn = long(WINAPI*)(unsigned long, PassThruMsg*, unsigned long*, unsigned long);
using FilterFn = long(WINAPI*)(unsigned long, unsigned long, PassThruMsg*, PassThruMsg*, PassThruMsg*, unsigned long*);
using IoctlFn = long(WINAPI*)(unsigned long, unsigned long, void*, void*);
using LastErrorFn = long(WINAPI*)(char*);

class J2534Link : public CanLink {
public:
    ~J2534Link() override {
        if (channel_ && disconnect_) disconnect_(channel_);
        if (device_ && close_) close_(device_);
        if (module_) FreeLibrary(module_);
    }

    bool open(const std::string& library, int bitrate, std::string& error) {
        module_ = LoadLibraryA(library.c_str());
        if (!module_) {
            error = L("Der J2534-Treiber lässt sich nicht laden: ", "The J2534 driver cannot be loaded: ") + library +
                    L(" (Caruspex ist ein 64-Bit-Programm und braucht einen 64-Bit-Treiber).",
                      " (Caruspex is a 64-bit program and needs a 64-bit driver).");
            return false;
        }
        open_ = reinterpret_cast<OpenFn>(GetProcAddress(module_, "PassThruOpen"));
        close_ = reinterpret_cast<CloseFn>(GetProcAddress(module_, "PassThruClose"));
        connect_ = reinterpret_cast<ConnectFn>(GetProcAddress(module_, "PassThruConnect"));
        disconnect_ = reinterpret_cast<DisconnectFn>(GetProcAddress(module_, "PassThruDisconnect"));
        read_ = reinterpret_cast<ReadFn>(GetProcAddress(module_, "PassThruReadMsgs"));
        write_ = reinterpret_cast<WriteFn>(GetProcAddress(module_, "PassThruWriteMsgs"));
        filter_ = reinterpret_cast<FilterFn>(GetProcAddress(module_, "PassThruStartMsgFilter"));
        ioctl_ = reinterpret_cast<IoctlFn>(GetProcAddress(module_, "PassThruIoctl"));
        lastError_ = reinterpret_cast<LastErrorFn>(GetProcAddress(module_, "PassThruGetLastError"));
        if (!open_ || !close_ || !connect_ || !disconnect_ || !read_ || !write_ || !filter_) {
            error = L("Das ist kein vollständiger J2534-Treiber.", "This is not a complete J2534 driver.");
            return false;
        }
        if (open_(nullptr, &device_) != kNoError) {
            error = L("Das J2534-Gerät antwortet nicht: ", "The J2534 device does not answer: ") + last();
            device_ = 0;
            return false;
        }
        if (connect_(device_, kProtocolCan, kCanIdBoth, static_cast<unsigned long>(bitrate), &channel_) != kNoError) {
            error = L("CAN lässt sich am J2534-Gerät nicht verbinden: ", "CAN cannot be connected on the J2534 device: ") +
                    last();
            channel_ = 0;
            return false;
        }
        // Let every frame through: mask and pattern zero.
        PassThruMsg mask{};
        PassThruMsg pattern{};
        mask.ProtocolID = pattern.ProtocolID = kProtocolCan;
        mask.DataSize = pattern.DataSize = 4;
        unsigned long filterId = 0;
        if (filter_(channel_, kPassFilter, &mask, &pattern, nullptr, &filterId) != kNoError) {
            error = L("Der Empfangsfilter am J2534-Gerät ging nicht: ", "The receive filter on the J2534 device failed: ") +
                    last();
            return false;
        }
        library_ = library;
        return true;
    }

    bool send(const CanFrame& frame) override {
        PassThruMsg m{};
        m.ProtocolID = kProtocolCan;
        m.TxFlags = frame.extended ? kCan29BitId : 0;
        m.Data[0] = static_cast<unsigned char>(frame.id >> 24);
        m.Data[1] = static_cast<unsigned char>(frame.id >> 16);
        m.Data[2] = static_cast<unsigned char>(frame.id >> 8);
        m.Data[3] = static_cast<unsigned char>(frame.id);
        std::memcpy(m.Data + 4, frame.data.data(), frame.data.size());
        m.DataSize = static_cast<unsigned long>(4 + frame.data.size());
        unsigned long count = 1;
        return write_(channel_, &m, &count, 100) == kNoError && count == 1;
    }

    bool receive(CanFrame& frame, int timeoutMs) override {
        for (;;) {
            PassThruMsg m{};
            unsigned long count = 1;
            const long status = read_(channel_, &m, &count, static_cast<unsigned long>(timeoutMs < 0 ? 0 : timeoutMs));
            if (status != kNoError || count == 0) return false;
            if (m.RxStatus & kTxMsgType) continue;   // our own frame, looped back
            if (m.DataSize < 4) continue;
            frame.extended = (m.RxStatus & kCan29BitId) != 0;
            frame.id = (static_cast<std::uint32_t>(m.Data[0]) << 24) | (static_cast<std::uint32_t>(m.Data[1]) << 16) |
                       (static_cast<std::uint32_t>(m.Data[2]) << 8) | m.Data[3];
            frame.data.assign(m.Data + 4, m.Data + std::min<unsigned long>(m.DataSize, 12));
            return true;
        }
    }

    std::string name() const override { return "J2534 " + library_; }

    double batteryVoltage() override {
        if (!ioctl_) return -1.0;
        unsigned long millivolts = 0;
        if (ioctl_(device_, kReadVbatt, nullptr, &millivolts) != kNoError) return -1.0;
        return millivolts / 1000.0;
    }

private:
    std::string last() const {
        if (!lastError_) return {};
        char text[256] = {};
        lastError_(text);
        return text;
    }

    HMODULE module_ = nullptr;
    OpenFn open_ = nullptr;
    CloseFn close_ = nullptr;
    ConnectFn connect_ = nullptr;
    DisconnectFn disconnect_ = nullptr;
    ReadFn read_ = nullptr;
    WriteFn write_ = nullptr;
    FilterFn filter_ = nullptr;
    IoctlFn ioctl_ = nullptr;
    LastErrorFn lastError_ = nullptr;
    unsigned long device_ = 0;
    unsigned long channel_ = 0;
    std::string library_;
};

std::string regString(HKEY key, const char* value) {
    char buffer[512] = {};
    DWORD size = sizeof(buffer);
    DWORD type = 0;
    if (RegQueryValueExA(key, value, nullptr, &type, reinterpret_cast<BYTE*>(buffer), &size) != ERROR_SUCCESS) return {};
    if (type != REG_SZ && type != REG_EXPAND_SZ) return {};
    return std::string(buffer);
}

void readDevices(REGSAM view, bool usable, std::vector<J2534Device>& out) {
    HKEY root = nullptr;
    if (RegOpenKeyExA(HKEY_LOCAL_MACHINE, "SOFTWARE\\PassThruSupport.04.04", 0, KEY_READ | view, &root) != ERROR_SUCCESS)
        return;
    char sub[256];
    for (DWORD i = 0;; ++i) {
        DWORD length = sizeof(sub);
        if (RegEnumKeyExA(root, i, sub, &length, nullptr, nullptr, nullptr, nullptr) != ERROR_SUCCESS) break;
        HKEY key = nullptr;
        if (RegOpenKeyExA(root, sub, 0, KEY_READ | view, &key) != ERROR_SUCCESS) continue;
        J2534Device d;
        d.name = regString(key, "Name");
        if (d.name.empty()) d.name = sub;
        d.vendor = regString(key, "Vendor");
        d.library = regString(key, "FunctionLibrary");
        d.usable = usable;
        RegCloseKey(key);
        if (d.library.empty()) continue;
        const bool known = std::any_of(out.begin(), out.end(), [&](const J2534Device& o) { return o.library == d.library; });
        if (!known) out.push_back(d);
    }
    RegCloseKey(root);
}

}  // namespace

std::vector<J2534Device> j2534Devices() {
    std::vector<J2534Device> out;
    readDevices(KEY_WOW64_64KEY, true, out);
    // Drivers only registered for 32-bit programs: listed, but not loadable.
    readDevices(KEY_WOW64_32KEY, false, out);
    return out;
}

std::unique_ptr<CanLink> openJ2534(const std::string& library, int bitrate, std::string& error) {
    if (library.empty()) {
        error = L("Kein J2534-Gerät eingerichtet. Ist der Treiber des Herstellers installiert?",
                  "No J2534 device set up. Is the maker's driver installed?");
        return nullptr;
    }
    auto link = std::make_unique<J2534Link>();
    if (!link->open(library, bitrate, error)) return nullptr;
    return link;
}

#else

std::vector<J2534Device> j2534Devices() { return {}; }

std::unique_ptr<CanLink> openJ2534(const std::string&, int, std::string& error) {
    error = L("J2534-Geräte gibt es nur unter Windows.", "J2534 devices exist on Windows only.");
    return nullptr;
}

#endif

}  // namespace caruspex
