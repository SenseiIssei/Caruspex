// A J2534-1 (04.04) driver DLL for the tests: no hardware, an engine unit
// that answers 0100 and the VIN (multi frame, after flow control) on CAN.
// Built only on Windows, loaded by test_j2534.cpp through LoadLibrary like
// a real PassThru driver.

#include <windows.h>

#include <cstring>
#include <deque>
#include <vector>

namespace {

struct PassThruMsg {
    unsigned long ProtocolID, RxStatus, TxFlags, Timestamp, DataSize, ExtraDataIndex;
    unsigned char Data[4128];
};

std::deque<PassThruMsg> inbox;
std::vector<std::vector<unsigned char>> held;   // consecutive frames waiting for flow control
bool opened = false, connected = false, filtered = false;

void push(unsigned long id, std::vector<unsigned char> data) {
    data.resize(8, 0xAA);
    PassThruMsg m{};
    m.ProtocolID = 5;
    m.Data[0] = static_cast<unsigned char>(id >> 24);
    m.Data[1] = static_cast<unsigned char>(id >> 16);
    m.Data[2] = static_cast<unsigned char>(id >> 8);
    m.Data[3] = static_cast<unsigned char>(id);
    std::memcpy(m.Data + 4, data.data(), 8);
    m.DataSize = 12;
    inbox.push_back(m);
}

void handle(const PassThruMsg& m) {
    // Every write comes back once as an echo, as real devices do.
    PassThruMsg echo = m;
    echo.RxStatus = 0x01;
    inbox.push_back(echo);
    const unsigned long id = (unsigned long)m.Data[0] << 24 | (unsigned long)m.Data[1] << 16 | (unsigned long)m.Data[2] << 8 | m.Data[3];
    const unsigned char* d = m.Data + 4;
    if (id == 0x7E0 && (d[0] >> 4) == 3) {
        for (const auto& cf : held) push(0x7E8, cf);
        held.clear();
        return;
    }
    if (id != 0x7DF && id != 0x7E0) return;
    if (d[0] == 2 && d[1] == 0x01 && d[2] == 0x00) push(0x7E8, {0x06, 0x41, 0x00, 0xBE, 0x3F, 0xA8, 0x13});
    if (d[0] == 2 && d[1] == 0x09 && d[2] == 0x02) {
        // 49 02 01 + "WAUZZZ8V5LA012345": 20 bytes.
        push(0x7E8, {0x10, 0x14, 0x49, 0x02, 0x01, 'W', 'A', 'U'});
        held = {{0x21, 'Z', 'Z', 'Z', '8', 'V', '5', 'L'}, {0x22, 'A', '0', '1', '2', '3', '4', '5'}};
    }
}

}  // namespace

extern "C" {

__declspec(dllexport) long WINAPI PassThruOpen(void*, unsigned long* device) {
    opened = true;
    *device = 1;
    return 0;
}
__declspec(dllexport) long WINAPI PassThruClose(unsigned long) {
    opened = false;
    return 0;
}
__declspec(dllexport) long WINAPI PassThruConnect(unsigned long, unsigned long protocol, unsigned long, unsigned long baud,
                                                  unsigned long* channel) {
    if (!opened || protocol != 5 || baud != 500000) return 0x07;   // ERR_INVALID_PROTOCOL_ID / baud
    connected = true;
    *channel = 2;
    return 0;
}
__declspec(dllexport) long WINAPI PassThruDisconnect(unsigned long) {
    connected = false;
    return 0;
}
__declspec(dllexport) long WINAPI PassThruReadMsgs(unsigned long, PassThruMsg* msgs, unsigned long* count, unsigned long) {
    if (inbox.empty() || !filtered) {
        *count = 0;
        return 0x10;   // ERR_BUFFER_EMPTY
    }
    msgs[0] = inbox.front();
    inbox.pop_front();
    *count = 1;
    return 0;
}
__declspec(dllexport) long WINAPI PassThruWriteMsgs(unsigned long, PassThruMsg* msgs, unsigned long* count, unsigned long) {
    if (!connected) return 0x02;
    for (unsigned long i = 0; i < *count; ++i) handle(msgs[i]);
    return 0;
}
__declspec(dllexport) long WINAPI PassThruStartMsgFilter(unsigned long, unsigned long type, PassThruMsg*, PassThruMsg*,
                                                         PassThruMsg*, unsigned long* id) {
    filtered = type == 1;
    *id = 3;
    return 0;
}
__declspec(dllexport) long WINAPI PassThruIoctl(unsigned long, unsigned long ioctl, void*, void* out) {
    if (ioctl == 0x03) {
        *static_cast<unsigned long*>(out) = 12480;   // READ_VBATT in mV
        return 0;
    }
    return 0x1A;
}
__declspec(dllexport) long WINAPI PassThruGetLastError(char* text) {
    std::strcpy(text, "fake driver");
    return 0;
}

}  // extern "C"
