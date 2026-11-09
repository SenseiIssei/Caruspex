#include "core/Serial.h"

#include <windows.h>
#include <setupapi.h>
#include <devguid.h>

#include <algorithm>
#include <array>
#include <cctype>
#include <cstdio>

#include "core/Lang.h"

namespace caruspex {
namespace {

struct ChipHint {
    unsigned vid;
    unsigned pid;
    const char* name;
    const char* nameEn = nullptr;   // only where the name contains German
};

// Chips found in diagnostic adapters and KKL cables.
constexpr std::array<ChipHint, 9> kChips{{
    {0x0403, 0x6001, "FTDI FT232R"},
    {0x0403, 0x6015, "FTDI FT231X"},
    {0x0403, 0x6010, "FTDI FT2232"},
    {0x0403, 0xFA20, "FTDI (Ross-Tech HEX)"},
    {0x1A86, 0x7523, "CH340"},
    {0x1A86, 0x5523, "CH341"},
    {0x10C4, 0xEA60, "CP2102"},
    {0x067B, 0x2303, "PL2303"},
    {0x2303, 0x0000, "PL2303 (Variante)", "PL2303 (variant)"},
}};

std::string toUpper(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(),
                   [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
    return s;
}

void parseUsbId(const std::string& hardwareId, unsigned& vid, unsigned& pid) {
    const std::string up = toUpper(hardwareId);
    const size_t v = up.find("VID_");
    const size_t p = up.find("PID_");
    if (v != std::string::npos) std::sscanf(up.c_str() + v + 4, "%4x", &vid);
    if (p != std::string::npos) std::sscanf(up.c_str() + p + 4, "%4x", &pid);
}

bool looksLikeAdapter(const std::string& text) {
    static const char* kHints[] = {"OBD", "ELM", "FTDI", "CH340", "CH341", "CP210",
                                   "PL2303", "USB SERIAL", "USB-SERIAL", "BLUETOOTH",
                                   "SILICON LABS", "PROLIFIC", "VCI", "KKL"};
    const std::string up = toUpper(text);
    for (const char* h : kHints) {
        if (up.find(h) != std::string::npos) return true;
    }
    return false;
}

}  // namespace

std::vector<PortInfo> enumeratePorts() {
    std::vector<PortInfo> out;
    HDEVINFO info = SetupDiGetClassDevsA(&GUID_DEVCLASS_PORTS, nullptr, nullptr, DIGCF_PRESENT);
    if (info == INVALID_HANDLE_VALUE) return out;

    SP_DEVINFO_DATA data{};
    data.cbSize = sizeof(data);
    for (DWORD i = 0; SetupDiEnumDeviceInfo(info, i, &data); ++i) {
        char portName[64] = {};
        HKEY key = SetupDiOpenDevRegKey(info, &data, DICS_FLAG_GLOBAL, 0, DIREG_DEV, KEY_READ);
        if (key != INVALID_HANDLE_VALUE) {
            DWORD size = sizeof(portName);
            DWORD type = 0;
            RegQueryValueExA(key, "PortName", nullptr, &type,
                             reinterpret_cast<BYTE*>(portName), &size);
            RegCloseKey(key);
        }
        if (portName[0] == '\0' || std::string(portName).rfind("COM", 0) != 0) continue;

        char desc[512] = {};
        SetupDiGetDeviceRegistryPropertyA(info, &data, SPDRP_FRIENDLYNAME, nullptr,
                                          reinterpret_cast<BYTE*>(desc), sizeof(desc), nullptr);
        char hwid[512] = {};
        SetupDiGetDeviceRegistryPropertyA(info, &data, SPDRP_HARDWAREID, nullptr,
                                          reinterpret_cast<BYTE*>(hwid), sizeof(hwid), nullptr);

        PortInfo pi;
        pi.device = portName;
        pi.description = desc[0] ? desc : portName;
        pi.hardwareId = hwid;
        parseUsbId(pi.hardwareId, pi.vid, pi.pid);
        for (const ChipHint& c : kChips) {
            if (c.vid == pi.vid && (c.pid == pi.pid || c.pid == 0)) {
                pi.chipGuess = c.nameEn ? L(c.name, c.nameEn) : c.name;
                break;
            }
        }
        pi.likelyAdapter = !pi.chipGuess.empty() ||
                           looksLikeAdapter(pi.description + " " + pi.hardwareId);
        out.push_back(std::move(pi));
    }
    SetupDiDestroyDeviceInfoList(info);

    std::sort(out.begin(), out.end(), [](const PortInfo& a, const PortInfo& b) {
        if (a.likelyAdapter != b.likelyAdapter) return a.likelyAdapter;
        return a.device < b.device;
    });
    return out;
}

Serial::~Serial() { close(); }

bool Serial::open(const std::string& device, int baud, std::string& error) {
    close();
    // From COM10 on, Windows needs the long form.
    const std::string path = "\\\\.\\" + device;
    HANDLE h = CreateFileA(path.c_str(), GENERIC_READ | GENERIC_WRITE, 0, nullptr,
                           OPEN_EXISTING, 0, nullptr);
    if (h == INVALID_HANDLE_VALUE) {
        const DWORD err = GetLastError();
        error = device + L(" lässt sich nicht öffnen (Windows-Fehler ",
                           " cannot be opened (Windows error ") +
                std::to_string(err) + ")";
        if (err == ERROR_ACCESS_DENIED)
            error += L(": belegt ein anderes Programm den Port?",
                       ": is another program using the port?");
        if (err == ERROR_FILE_NOT_FOUND) error += L(": Port existiert nicht.", ": port does not exist.");
        return false;
    }

    DCB dcb{};
    dcb.DCBlength = sizeof(dcb);
    if (!GetCommState(h, &dcb)) {
        CloseHandle(h);
        error = L("GetCommState fehlgeschlagen", "GetCommState failed");
        return false;
    }
    dcb.BaudRate = static_cast<DWORD>(baud);
    dcb.ByteSize = 8;
    dcb.Parity = NOPARITY;
    dcb.StopBits = ONESTOPBIT;
    dcb.fBinary = TRUE;
    dcb.fParity = FALSE;
    dcb.fOutxCtsFlow = FALSE;
    dcb.fOutxDsrFlow = FALSE;
    dcb.fDtrControl = DTR_CONTROL_ENABLE;
    dcb.fRtsControl = RTS_CONTROL_ENABLE;
    dcb.fOutX = FALSE;
    dcb.fInX = FALSE;
    if (!SetCommState(h, &dcb)) {
        CloseHandle(h);
        error = L("Baudrate ", "Baud rate ") + std::to_string(baud) +
                L(" wird nicht unterstützt", " is not supported");
        return false;
    }

    COMMTIMEOUTS to{};
    to.ReadIntervalTimeout = 30;
    to.ReadTotalTimeoutConstant = 60;
    to.ReadTotalTimeoutMultiplier = 0;
    to.WriteTotalTimeoutConstant = 500;
    to.WriteTotalTimeoutMultiplier = 0;
    SetCommTimeouts(h, &to);

    PurgeComm(h, PURGE_RXCLEAR | PURGE_TXCLEAR);
    handle_ = h;
    device_ = device;
    baud_ = baud;
    return true;
}

void Serial::close() {
    if (handle_) {
        CloseHandle(static_cast<HANDLE>(handle_));
        handle_ = nullptr;
    }
    device_.clear();
    baud_ = 0;
}

bool Serial::write(const std::string& data) {
    if (!handle_) return false;
    DWORD written = 0;
    const BOOL ok = WriteFile(static_cast<HANDLE>(handle_), data.data(),
                              static_cast<DWORD>(data.size()), &written, nullptr);
    return ok && written == data.size();
}

std::string Serial::read(int timeoutMs) {
    if (!handle_) return {};
    std::string out;
    char buffer[512];
    const ULONGLONG deadline = GetTickCount64() + static_cast<ULONGLONG>(timeoutMs);
    do {
        DWORD got = 0;
        if (!ReadFile(static_cast<HANDLE>(handle_), buffer, sizeof(buffer), &got, nullptr)) break;
        if (got > 0) {
            out.append(buffer, got);
            break;  // the caller decides whether it needs more
        }
    } while (GetTickCount64() < deadline);
    return out;
}

void Serial::flushInput() {
    if (handle_) PurgeComm(static_cast<HANDLE>(handle_), PURGE_RXCLEAR);
}

bool Serial::setBreak(bool active) {
    if (!handle_) return false;
    HANDLE h = static_cast<HANDLE>(handle_);
    return active ? SetCommBreak(h) != 0 : ClearCommBreak(h) != 0;
}

bool Serial::changeBaud(int baud) {
    if (!handle_) return false;
    HANDLE h = static_cast<HANDLE>(handle_);
    DCB dcb{};
    dcb.DCBlength = sizeof(dcb);
    if (!GetCommState(h, &dcb)) return false;
    dcb.BaudRate = static_cast<DWORD>(baud);
    if (!SetCommState(h, &dcb)) return false;
    baud_ = baud;
    return true;
}

}  // namespace caruspex
