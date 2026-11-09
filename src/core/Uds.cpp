#include "core/Uds.h"

#include <algorithm>
#include <cctype>
#include <cstdio>
#include <utility>

#include "core/Lang.h"
#include "core/Timing.h"
#include "data/DtcDatabase.h"

namespace caruspex {
namespace {

// How long an ECU may keep answering 7F xx 78 ("still working") before we
// give up. ISO 14229 allows 5 s per pending answer (P2* server); clearing a
// big fault memory can need a few of them.
constexpr int kPendingLimitMs = 10000;
constexpr int kMaxRounds = 25;

// Timeouts on the PC side. The adapter itself gives up after its own ATST
// timer and prints "NO DATA", which normally comes much earlier.
constexpr int kAtTimeoutMs = 2000;
constexpr int kShortRequestMs = 1000;
constexpr int kClearRequestMs = 5000;

std::string hexId(std::uint32_t value, int digits) {
    char buffer[16];
    std::snprintf(buffer, sizeof(buffer), "%0*X", digits, static_cast<unsigned>(value));
    return buffer;
}

std::string upper(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(),
                   [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
    return s;
}

// True if the adapter answered the AT command with OK.
bool atOk(Elm327& elm, const std::string& command) {
    const ElmResult r = elm.send(command, kAtTimeoutMs);
    if (r.status == ElmStatus::AdapterError) return false;
    for (const std::string& line : r.lines) {
        if (upper(line).find("OK") != std::string::npos) return true;
    }
    return false;
}

bool bus29(const Elm327& elm) {
    return elm.protocolId() == '7' || elm.protocolId() == '9';
}

// Plain text for a negative response code (ISO 14229-1 annex A).
std::string nrcText(int nrc) {
    const char* text = nullptr;
    switch (nrc) {
        case 0x10: text = L("allgemeine Ablehnung", "general reject"); break;
        case 0x11: text = L("Dienst wird nicht unterstützt", "service not supported"); break;
        case 0x12: text = L("Unterfunktion wird nicht unterstützt", "sub-function not supported"); break;
        case 0x13: text = L("falsche Länge oder falsches Format", "incorrect message length or format"); break;
        case 0x14: text = L("Antwort wäre zu lang", "response too long"); break;
        case 0x21: text = L("beschäftigt, bitte wiederholen", "busy, repeat request"); break;
        case 0x22:
            text = L("Bedingungen nicht erfüllt (Zündung, Motorzustand, Geschwindigkeit?)",
                     "conditions not correct (ignition, engine state, speed?)");
            break;
        case 0x24: text = L("falsche Reihenfolge der Anfragen", "request sequence error"); break;
        case 0x31: text = L("Kennung oder Wert außerhalb des Bereichs", "request out of range"); break;
        case 0x33:
            text = L("Zugriff gesperrt, braucht eine Freischaltung (Security Access)",
                     "security access denied");
            break;
        case 0x35: text = L("falscher Schlüssel", "invalid key"); break;
        case 0x36: text = L("zu viele Versuche", "exceeded number of attempts"); break;
        case 0x37: text = L("Wartezeit noch nicht abgelaufen", "required time delay not expired"); break;
        case 0x72: text = L("allgemeiner Programmierfehler", "general programming failure"); break;
        case 0x78: text = L("Antwort noch in Arbeit", "response pending"); break;
        case 0x7E:
            text = L("Unterfunktion in der aktiven Sitzung nicht unterstützt",
                     "sub-function not supported in active session");
            break;
        case 0x7F:
            text = L("Dienst in der aktiven Sitzung nicht unterstützt",
                     "service not supported in active session");
            break;
        default: break;
    }
    char code[8];
    std::snprintf(code, sizeof(code), "0x%02X", nrc & 0xFF);
    std::string out = L("Steuergerät lehnt ab: ", "Control unit refuses: ");
    if (text) return out + text + " (" + code + ")";
    return out + L("Code ", "code ") + code;
}

// What came back for one request.
struct Exchange {
    bool answered = false;   // any answer at all, negative ones included
    bool positive = false;
    int nrc = -1;            // final negative response code
    std::vector<int> data;   // positive response without the service id
    std::string error;
};

Exchange exchange(Elm327& elm, const std::vector<int>& bytes, int timeoutMs) {
    Exchange ex;
    if (bytes.empty() || bytes.size() > 7) {
        // The ELM327 only sends single frames: at most seven data bytes.
        ex.error = L("Anfrage zu lang für den Adapter (höchstens 7 Bytes).",
                     "Request too long for the adapter (at most 7 bytes).");
        return ex;
    }
    std::string command;
    for (int b : bytes) command += hexId(static_cast<std::uint32_t>(b & 0xFF), 2);
    const int sid = bytes[0] & 0xFF;

    const std::uint64_t start = nowMs();
    for (int round = 0; round < kMaxRounds; ++round) {
        const ElmResult raw = elm.send(command, timeoutMs);
        if (raw.status == ElmStatus::AdapterError) {
            ex.error = raw.message;
            return ex;
        }
        // decodeMessages knows the adapter's status lines (NO DATA, CAN
        // ERROR, ...). The service filter does not matter for that part.
        const ElmResult status = decodeMessages(raw.lines, sid, elm.headerFormat());
        if (status.status == ElmStatus::BusError) {
            ex.error = status.message;
            return ex;
        }
        bool pending = false;
        int busy = -1;
        int refusal = -1;
        for (const ElmMessage& msg : splitMessages(raw.lines, elm.headerFormat())) {
            const std::vector<int>& b = msg.bytes;
            if (b.empty()) continue;
            if (b[0] == sid + 0x40) {
                ex.answered = true;
                ex.positive = true;
                ex.data.assign(b.begin() + 1, b.end());
                return ex;   // a positive answer wins over a pending one before it
            }
            if (b[0] == 0x7F && b.size() >= 3 && b[1] == sid) {
                ex.answered = true;
                if (b[2] == 0x78) pending = true;
                else if (b[2] == 0x21) busy = b[2];
                else if (refusal < 0) refusal = b[2];
            }
        }
        if (refusal >= 0) {
            ex.nrc = refusal;
            ex.error = nrcText(refusal);
            return ex;
        }
        if (pending || busy >= 0) {
            // The ECU is still working. The adapter stopped listening at its
            // own timeout, so the late answer can only be had by asking
            // again. Every service used here may be repeated safely.
            if (nowMs() - start > static_cast<std::uint64_t>(kPendingLimitMs)) {
                ex.nrc = pending ? 0x78 : 0x21;
                ex.error = L("Steuergerät bleibt beschäftigt und liefert keine Antwort.",
                             "Control unit stays busy and never delivers the answer.");
                return ex;
            }
            sleepMs(busy >= 0 && !pending ? 100 : 20);
            continue;
        }
        if (status.status == ElmStatus::NoData && !raw.lines.empty() &&
            upper(raw.lines.front()).rfind("NO DATA", 0) == 0) {
            ex.error = L("Keine Antwort vom Steuergerät (NO DATA).",
                         "No answer from the control unit (NO DATA).");
            return ex;
        }
        ex.error = L("Keine verwertbare Antwort erhalten.", "No usable response received.");
        return ex;
    }
    ex.error = L("Steuergerät bleibt beschäftigt und liefert keine Antwort.",
                 "Control unit stays busy and never delivers the answer.");
    return ex;
}

// Printable ASCII only, trimmed. Identification strings are often padded
// with spaces, zero bytes or 0xFF.
std::string cleanAscii(const std::vector<int>& bytes) {
    std::string out;
    for (int b : bytes) {
        if (b >= 0x20 && b <= 0x7E) out += static_cast<char>(b);
    }
    const size_t a = out.find_first_not_of(' ');
    if (a == std::string::npos) return {};
    const size_t e = out.find_last_not_of(' ');
    return out.substr(a, e - a + 1);
}

}  // namespace

UdsDtc decodeUdsDtc(int b1, int b2, int b3, int status) {
    UdsDtc dtc;
    dtc.code = decodeDtc(b1 & 0xFF, b2 & 0xFF);
    if (dtc.code.empty()) dtc.code = "P0000";
    dtc.failureType = b3 & 0xFF;
    dtc.failureText = failureTypeText(dtc.failureType);
    dtc.status = status & 0xFF;
    dtc.testFailed = (dtc.status & 0x01) != 0;
    dtc.pending = (dtc.status & 0x04) != 0;
    dtc.confirmed = (dtc.status & 0x08) != 0;
    dtc.warningLamp = (dtc.status & 0x80) != 0;
    dtc.text = lookupDtc(dtc.code).text;
    return dtc;
}

Uds::Uds(Elm327& elm) : elm_(elm) {}

Uds::~Uds() {
    if (addressed_) restoreDefault();
}

bool Uds::select(const EcuDefinition& ecu, std::string& error) {
    error.clear();
    if (!elm_.connected()) {
        error = L("Kein Adapter verbunden.", "No adapter connected.");
        return false;
    }
    if (!elm_.isCan()) {
        error = L("UDS geht über den ELM327 nur auf CAN; dieses Auto spricht ein anderes Protokoll.",
                  "UDS through the ELM327 needs CAN; this car speaks another protocol.");
        return false;
    }
    if (ecu.extended29 != bus29(elm_)) {
        error = ecu.extended29
                    ? L("Steuergerät mit 29-Bit-Adresse, das Auto ist aber mit 11 Bit verbunden.",
                        "Control unit with a 29-bit address, but the car is connected with 11 bit.")
                    : L("Steuergerät mit 11-Bit-Adresse, das Auto ist aber mit 29 Bit verbunden.",
                        "Control unit with an 11-bit address, but the car is connected with 29 bit.");
        return false;
    }

    // From here on the adapter is changed, so the destructor puts it back
    // even if a step fails halfway.
    addressed_ = true;
    bool ok = true;
    if (ecu.extended29) {
        // ATSH takes the lower three bytes, ATCP the priority byte on top.
        ok = atOk(elm_, "ATCP" + hexId((ecu.request >> 24) & 0x1F, 2)) &&
             atOk(elm_, "ATSH" + hexId(ecu.request & 0xFFFFFF, 6)) &&
             atOk(elm_, "ATCRA" + hexId(ecu.response & 0x1FFFFFFF, 8));
    } else {
        ok = atOk(elm_, "ATSH" + hexId(ecu.request & 0x7FF, 3)) &&
             atOk(elm_, "ATCRA" + hexId(ecu.response & 0x7FF, 3));
    }
    if (!ok) {
        error = L("Der Adapter nimmt die Adresse nicht an (ATSH/ATCRA).",
                  "The adapter does not accept the address (ATSH/ATCRA).");
        restoreDefault();
        return false;
    }
    // Flow control for multi-frame answers goes to the request id. In its
    // automatic mode the adapter guesses the id from the "+8" convention,
    // which not every maker follows, so it is set explicitly. Older clones
    // without these commands still work for ECUs that follow the
    // convention, which is why a refusal is not an error.
    if (atOk(elm_, "ATFCSH" + hexId(ecu.request, ecu.extended29 ? 8 : 3)) &&
        atOk(elm_, "ATFCSD300000")) {
        atOk(elm_, "ATFCSM1");
    }
    // CAN auto formatting (PCI bytes handled by the adapter) is the default,
    // and the ISO-TP parsing relies on it.
    atOk(elm_, "ATCAF1");
    return true;
}

void Uds::restoreDefault() {
    addressed_ = false;
    if (!elm_.connected() || !elm_.isCan()) return;
    atOk(elm_, "ATFCSM0");
    if (bus29(elm_)) {
        atOk(elm_, "ATCP18");
        atOk(elm_, "ATSHDB33F1");
    } else {
        atOk(elm_, "ATSH7DF");
    }
    // ATCRA without an address came with v1.4b; clones with an older
    // command set reset the receive filter with ATAR.
    if (!atOk(elm_, "ATCRA")) atOk(elm_, "ATAR");
}

bool Uds::request(const std::vector<int>& bytes, std::vector<int>& response, std::string& error,
                  int timeoutMs) {
    response.clear();
    Exchange ex = exchange(elm_, bytes, timeoutMs);
    error = ex.error;
    if (!ex.positive) return false;
    response = std::move(ex.data);
    return true;
}

bool Uds::testerPresent() {
    // Any answer counts, a refusal too: then the ECU is there but does not
    // like the service in its current session.
    return exchange(elm_, {0x3E, 0x00}, kShortRequestMs).answered;
}

std::vector<UdsDtc> Uds::readDtcs(std::string& error, int statusMask) {
    std::vector<UdsDtc> out;
    std::vector<int> response;
    if (!request({0x19, 0x02, statusMask & 0xFF}, response, error)) return out;
    // 59 02 <availability mask> then four bytes per fault.
    if (response.size() < 2 || response[0] != 0x02) {
        error = L("Unerwartete Antwort auf 19 02.", "Unexpected answer to 19 02.");
        return out;
    }
    for (size_t i = 2; i + 3 < response.size(); i += 4) {
        if (response[i] == 0 && response[i + 1] == 0 && response[i + 2] == 0) continue;
        out.push_back(decodeUdsDtc(response[i], response[i + 1], response[i + 2], response[i + 3]));
    }
    return out;
}

UdsEcuInfo Uds::readInfo() {
    // One identifier, empty if the ECU does not have it.
    auto did = [this](int id) -> std::string {
        std::vector<int> response;
        std::string error;
        if (!request({0x22, (id >> 8) & 0xFF, id & 0xFF}, response, error, kShortRequestMs))
            return {};
        if (response.size() < 2 || response[0] != ((id >> 8) & 0xFF) || response[1] != (id & 0xFF))
            return {};
        return cleanAscii(std::vector<int>(response.begin() + 2, response.end()));
    };
    UdsEcuInfo info;
    info.partNumber = did(0xF187);
    info.softwareVersion = did(0xF189);
    if (info.softwareVersion.empty()) info.softwareVersion = did(0xF195);
    info.hardwareVersion = did(0xF191);
    if (info.hardwareVersion.empty()) info.hardwareVersion = did(0xF193);
    info.systemName = did(0xF197);
    info.serial = did(0xF18C);
    info.vin = did(0xF190);
    return info;
}

bool Uds::clearDtcs(std::string& error) {
    std::vector<int> response;
    return request({0x14, 0xFF, 0xFF, 0xFF}, response, error, kClearRequestMs);
}

std::vector<UdsEcu> scanEcus(Elm327& elm, const std::vector<EcuDefinition>& ecus,
                             const std::function<void(int, int, const std::string&)>& progress,
                             const std::function<bool()>& cancelled) {
    std::vector<UdsEcu> out;
    out.reserve(ecus.size());
    const bool can = elm.connected() && elm.isCan();

    // A missing ECU costs the adapter's full answer timeout ("NO DATA"),
    // twice per address. 200 ms instead of the 400 ms set at connect is
    // still four times what UDS gives an ECU for its first answer (P2 =
    // 50 ms), and a slow ECU says 7F xx 78 well within it.
    const bool shortened = can && atOk(elm, "ATST32");

    {
        Uds uds(elm);
        const int total = static_cast<int>(ecus.size());
        for (int i = 0; i < total; ++i) {
            if (cancelled && cancelled()) break;
            const EcuDefinition& def = ecus[static_cast<size_t>(i)];
            if (progress) progress(i, total, def.name);

            UdsEcu ecu;
            ecu.definition = def;
            std::string error;
            if (!uds.select(def, error)) {
                ecu.note = error;
                out.push_back(std::move(ecu));
                continue;
            }
            ecu.present = uds.testerPresent();
            if (!ecu.present) {
                // Some ECUs ignore 3E outside an extended session but do
                // answer the identification.
                ecu.present = exchange(elm, {0x22, 0xF1, 0x87}, kShortRequestMs).answered;
            }
            if (!ecu.present) {
                ecu.note = L("Keine Antwort: nicht verbaut, oder das Gateway lässt die Anfrage nicht durch.",
                             "No answer: not fitted, or the gateway does not pass the request on.");
                out.push_back(std::move(ecu));
                continue;
            }
            ecu.info = uds.readInfo();
            ecu.dtcs = uds.readDtcs(error);
            ecu.dtcsRead = error.empty();
            if (!ecu.dtcsRead) {
                ecu.note = L("Fehlerspeicher nicht lesbar. ", "Fault memory not readable. ") + error;
            }
            out.push_back(std::move(ecu));
        }
        uds.restoreDefault();
    }

    if (shortened) atOk(elm, "ATST64");   // the value Elm327::connect sets
    return out;
}

}  // namespace caruspex
