// UDS (ISO 14229) through the ELM327: addressing, ISO-TP answers with and
// without headers, negative responses, fault records and the failure type
// byte. The recorded byte examples are worked out by hand from ISO 14229-1,
// ISO 15765-2 and the ELM327 data sheet; the demo car checks the same end
// to end, including that normal OBD still works afterwards.

#include <catch2/catch_test_macros.hpp>

#include <algorithm>
#include <cctype>
#include <cstdio>
#include <deque>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <vector>

#include "core/Elm327.h"
#include "core/Lang.h"
#include "core/Obd2.h"
#include "core/Transport.h"
#include "core/Uds.h"
#include "data/EcuAddresses.h"
#include "DemoRig.h"

using namespace caruspex;

namespace {

// A scripted ELM327 on CAN. Every request has an answer for headers off and
// one for headers on; several answers for the same request are given out
// one after the other (the last one stays).
class ScriptTransport : public ITransport {
public:
    struct Reply {
        std::vector<std::string> plain;
        std::vector<std::string> headers;
    };
    std::map<std::string, std::deque<Reply>> replies;
    std::set<std::string> refused;      // AT commands answered with "?"
    std::vector<std::string> sent;      // every command, in order

    bool write(const std::string& data) override {
        std::string cmd;
        for (char c : data) {
            if (c == '\r' || c == '\n' || c == ' ') continue;
            cmd += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
        }
        sent.push_back(cmd);
        if (refused.count(cmd)) {
            pending_ = "?\r\r>";
        } else if (cmd == "ATZ") {
            headers_ = false;
            pending_ = "ELM327 v1.5\r\r>";
        } else if (cmd == "ATH1" || cmd == "ATH0") {
            headers_ = cmd == "ATH1";
            pending_ = "OK\r\r>";
        } else if (cmd == "ATDP") {
            pending_ = "ISO 15765-4 (CAN)\r\r>";
        } else if (cmd.rfind("AT", 0) == 0) {
            pending_ = "OK\r\r>";
        } else if (cmd == "0100") {
            pending_ = "41 00 BE 1F A8 13\r\r>";
        } else {
            const auto it = replies.find(cmd);
            if (it == replies.end() || it->second.empty()) {
                pending_ = "NO DATA\r\r>";
            } else {
                const Reply reply = it->second.front();
                if (it->second.size() > 1) it->second.pop_front();
                pending_.clear();
                for (const std::string& line : headers_ ? reply.headers : reply.plain)
                    pending_ += line + "\r";
                pending_ += "\r>";
            }
        }
        return true;
    }
    std::string read(int) override {
        std::string out;
        out.swap(pending_);
        return out;
    }
    void flushInput() override { pending_.clear(); }
    std::string name() const override { return "Script"; }

    size_t count(const std::string& cmd) const {
        return static_cast<size_t>(std::count(sent.begin(), sent.end(), cmd));
    }

private:
    std::string pending_;
    bool headers_ = false;
};

struct ScriptRig {
    ScriptTransport* t = nullptr;
    Elm327 elm;
    std::string error;
    bool connected = false;

    explicit ScriptRig(char protocol = '6') {
        auto owner = std::make_unique<ScriptTransport>();
        t = owner.get();
        connected = elm.connect(std::move(owner), std::string(1, protocol), error);
    }
};

EcuDefinition ecu(std::uint32_t request, std::uint32_t response, const std::string& name,
                  EcuKind kind = EcuKind::Other, bool extended = false) {
    EcuDefinition d;
    d.request = request;
    d.response = response;
    d.extended29 = extended;
    d.kind = kind;
    d.name = name;
    return d;
}

// "CX5906018B" in F187: 62 F1 87 plus ten characters, thirteen bytes, so a
// first frame with six bytes and one consecutive frame with seven.
const ScriptTransport::Reply kPartNumber = {
    {"00D", "0: 62 F1 87 43 58 35", "1: 39 30 36 30 31 38 42"},
    {"7E8 10 0D 62 F1 87 43 58 35", "7E8 21 39 30 36 30 31 38 42"}};

std::vector<UdsEcu> scan(Elm327& elm, const std::vector<EcuDefinition>& list) {
    return scanEcus(elm, list, nullptr, nullptr);
}

const UdsEcu* findEcu(const std::vector<UdsEcu>& ecus, std::uint32_t request) {
    for (const UdsEcu& e : ecus) {
        if (e.definition.request == request) return &e;
    }
    return nullptr;
}

std::vector<EcuDefinition> demoEcus() {
    return {
        ecu(0x7E0, 0x7E8, "Engine", EcuKind::Engine),
        ecu(0x7E1, 0x7E9, "Gearbox", EcuKind::Gearbox),
        ecu(0x760, 0x768, "ABS", EcuKind::Brakes),
        ecu(0x737, 0x73F, "Airbag", EcuKind::Airbag),
        ecu(0x720, 0x728, "Instruments", EcuKind::Instruments),
        ecu(0x744, 0x74C, "Climate", EcuKind::Climate),
        ecu(0x710, 0x718, "Nobody", EcuKind::Other),
        ecu(0x7A0, 0x7A8, "Nobody either", EcuKind::Other),
    };
}

}  // namespace

TEST_CASE("Selecting an ECU sets header, receive filter and flow control", "[uds]") {
    ScriptRig rig;
    REQUIRE(rig.connected);
    {
        Uds uds(rig.elm);
        std::string error;
        rig.t->sent.clear();
        REQUIRE(uds.select(ecu(0x760, 0x768, "ABS"), error));
        REQUIRE(error.empty());
        const std::vector<std::string> expected = {"ATSH760", "ATCRA768", "ATFCSH760",
                                                   "ATFCSD300000", "ATFCSM1", "ATCAF1"};
        REQUIRE(rig.t->sent == expected);
        rig.t->sent.clear();
    }
    // The destructor puts the functional OBD addressing back.
    const std::vector<std::string> restored = {"ATFCSM0", "ATSH7DF", "ATCRA"};
    REQUIRE(rig.t->sent == restored);
}

TEST_CASE("29-bit addressing uses ATCP for the priority byte", "[uds]") {
    ScriptRig rig('7');
    REQUIRE(rig.connected);
    Uds uds(rig.elm);
    std::string error;
    rig.t->sent.clear();
    REQUIRE(uds.select(ecu(0x18DA28F1, 0x18DAF128, "ABS", EcuKind::Brakes, true), error));
    REQUIRE(rig.t->sent[0] == "ATCP18");
    REQUIRE(rig.t->sent[1] == "ATSHDA28F1");
    REQUIRE(rig.t->sent[2] == "ATCRA18DAF128");
    REQUIRE(rig.t->sent[3] == "ATFCSH18DA28F1");

    // An 11-bit address does not fit this bus.
    REQUIRE_FALSE(uds.select(ecu(0x760, 0x768, "ABS"), error));
    REQUIRE_FALSE(error.empty());

    rig.t->sent.clear();
    uds.restoreDefault();
    const std::vector<std::string> restored = {"ATFCSM0", "ATCP18", "ATSHDB33F1", "ATCRA"};
    REQUIRE(rig.t->sent == restored);
}

TEST_CASE("Old clones reset the receive filter with ATAR", "[uds]") {
    ScriptRig rig;
    REQUIRE(rig.connected);
    rig.t->refused = {"ATCRA", "ATFCSH760"};
    Uds uds(rig.elm);
    std::string error;
    rig.t->sent.clear();
    // Flow control setup is optional; the address itself is not.
    REQUIRE(uds.select(ecu(0x760, 0x768, "ABS"), error));
    REQUIRE(rig.t->count("ATFCSM1") == 0);
    uds.restoreDefault();
    REQUIRE(rig.t->sent.back() == "ATAR");

    rig.t->refused = {"ATCRA768", "ATCRA"};
    REQUIRE_FALSE(uds.select(ecu(0x760, 0x768, "ABS"), error));
    REQUIRE_FALSE(error.empty());
    REQUIRE(rig.t->sent.back() == "ATAR");   // put back at once
}

TEST_CASE("UDS answers in single and multi frames, with and without headers", "[uds]") {
    for (bool headers : {false, true}) {
        ScriptRig rig;
        REQUIRE(rig.connected);
        if (headers) REQUIRE(rig.elm.setHeaders(true));
        rig.t->replies["3E00"] = {{{"7E 00"}, {"7E8 02 7E 00 AA AA AA AA AA"}}};
        rig.t->replies["22F187"] = {kPartNumber};
        rig.t->replies["22F189"] = {{{"62 F1 89 30 34 31 37"}, {"7E8 07 62 F1 89 30 34 31 37"}}};
        // A VIN padded with a zero byte and a space, both cleaned away.
        rig.t->replies["22F190"] = {
            {{"016", "0: 62 F1 90 59 43 58", "1: 44 45 31 35 43 37 4D", "2: 43 30 34 38 32 31 37",
              "3: 00 20"},
             {"7E8 10 16 62 F1 90 59 43 58", "7E8 21 44 45 31 35 43 37 4D",
              "7E8 22 43 30 34 38 32 31 37", "7E8 23 00 20 AA AA AA AA AA"}}};

        Uds uds(rig.elm);
        std::string error;
        REQUIRE(uds.select(ecu(0x7E0, 0x7E8, "Engine"), error));
        REQUIRE(uds.testerPresent());

        std::vector<int> response;
        REQUIRE(uds.request({0x22, 0xF1, 0x87}, response, error));
        REQUIRE(response.size() == 12);
        REQUIRE(response[0] == 0xF1);
        REQUIRE(response[1] == 0x87);
        REQUIRE(response.back() == 0x42);

        const UdsEcuInfo info = uds.readInfo();
        REQUIRE(info.partNumber == "CX5906018B");
        REQUIRE(info.softwareVersion == "0417");
        REQUIRE(info.vin == "YCXDE15C7MC048217");
        REQUIRE(info.hardwareVersion.empty());   // NO DATA for F191 and F193
        REQUIRE(info.systemName.empty());
    }
}

TEST_CASE("Response pending: in the same answer and after asking again", "[uds]") {
    for (bool headers : {false, true}) {
        ScriptRig rig;
        REQUIRE(rig.connected);
        if (headers) REQUIRE(rig.elm.setHeaders(true));
        Uds uds(rig.elm);
        std::string error;
        REQUIRE(uds.select(ecu(0x760, 0x768, "ABS"), error));

        // 7F 19 78, then within the adapter's timeout the real answer.
        rig.t->replies["1902FF"] = {
            {{"7F 19 78", "59 02 FF 40 35 1C AF"},
             {"768 03 7F 19 78 AA AA AA AA", "768 07 59 02 FF 40 35 1C AF"}}};
        std::vector<UdsDtc> dtcs = uds.readDtcs(error);
        REQUIRE(error.empty());
        REQUIRE(dtcs.size() == 1);
        REQUIRE(dtcs[0].code == "C0035");
        REQUIRE(rig.t->count("1902FF") == 1);

        // The answer comes too late for the adapter: two pending answers,
        // then the positive one on the third request.
        rig.t->sent.clear();
        rig.t->replies["1902FF"] = {
            {{"7F 19 78"}, {"768 03 7F 19 78 AA AA AA AA"}},
            {{"7F 19 78"}, {"768 03 7F 19 78 AA AA AA AA"}},
            {{"59 02 FF 40 35 1C AF"}, {"768 07 59 02 FF 40 35 1C AF"}}};
        dtcs = uds.readDtcs(error);
        REQUIRE(error.empty());
        REQUIRE(dtcs.size() == 1);
        REQUIRE(rig.t->count("1902FF") == 3);
    }
}

TEST_CASE("Negative responses give readable errors", "[uds]") {
    ScriptRig rig;
    REQUIRE(rig.connected);
    Uds uds(rig.elm);
    std::string error;
    REQUIRE(uds.select(ecu(0x760, 0x768, "ABS"), error));

    rig.t->replies["22F187"] = {{{"7F 22 11"}, {"768 03 7F 22 11 AA AA AA AA"}}};
    std::vector<int> response;
    setLanguage(Language::English);
    REQUIRE_FALSE(uds.request({0x22, 0xF1, 0x87}, response, error));
    REQUIRE(response.empty());
    REQUIRE(error.find("service not supported") != std::string::npos);
    REQUIRE(error.find("0x11") != std::string::npos);
    setLanguage(Language::German);
    REQUIRE_FALSE(uds.request({0x22, 0xF1, 0x87}, response, error));
    REQUIRE(error.find("Dienst wird nicht unterstützt") != std::string::npos);

    const std::map<int, std::string> texts = {
        {0x12, "Unterfunktion"}, {0x22, "Bedingungen"}, {0x31, "Bereich"},
        {0x33, "Security Access"}, {0x7E, "Sitzung"}, {0x7F, "Sitzung"}};
    for (const auto& [nrc, word] : texts) {
        char line[16];
        std::snprintf(line, sizeof(line), "7F 14 %02X", nrc);
        rig.t->replies["14FFFFFF"] = {{{line}, {}}};
        REQUIRE_FALSE(uds.clearDtcs(error));
        INFO(error);
        REQUIRE(error.find(word) != std::string::npos);
    }

    // No answer at all.
    REQUIRE_FALSE(uds.request({0x22, 0xF1, 0x90}, response, error));
    REQUIRE(error.find("NO DATA") != std::string::npos);
    // Too long for a single frame.
    REQUIRE_FALSE(uds.request({0x2E, 1, 2, 3, 4, 5, 6, 7}, response, error));

    // A refusal still means the ECU is there.
    rig.t->replies["3E00"] = {{{"7F 3E 11"}, {}}};
    REQUIRE(uds.testerPresent());
    rig.t->replies.erase("3E00");
    REQUIRE_FALSE(uds.testerPresent());
}

TEST_CASE("Decoding a three-byte UDS fault code with status", "[uds]") {
    setLanguage(Language::English);
    UdsDtc c = decodeUdsDtc(0x40, 0x35, 0x1C, 0xAF);
    REQUIRE(c.code == "C0035");
    REQUIRE(c.failureType == 0x1C);
    REQUIRE(c.failureText == "circuit voltage out of range");
    REQUIRE(c.status == 0xAF);
    REQUIRE(c.testFailed);
    REQUIRE(c.pending);
    REQUIRE(c.confirmed);
    REQUIRE(c.warningLamp);
    REQUIRE(c.text.find("wheel speed") != std::string::npos);

    UdsDtc u = decodeUdsDtc(0xC1, 0x00, 0x00, 0x28);
    REQUIRE(u.code == "U0100");
    REQUIRE(u.failureText.empty());
    REQUIRE(u.confirmed);
    REQUIRE_FALSE(u.testFailed);
    REQUIRE_FALSE(u.pending);
    REQUIRE_FALSE(u.warningLamp);

    UdsDtc b = decodeUdsDtc(0x90, 0x00, 0x49, 0x24);
    REQUIRE(b.code == "B1000");
    REQUIRE(b.pending);
    REQUIRE_FALSE(b.confirmed);
    REQUIRE(b.failureText == "internal electronic failure");

    REQUIRE(decodeUdsDtc(0x01, 0x71, 0x00, 0x08).code == "P0171");
    REQUIRE(decodeUdsDtc(0xD5, 0x5A, 0x00, 0x08).code == "U155A");
}

TEST_CASE("Failure type texts after SAE J2012-DA", "[uds]") {
    setLanguage(Language::German);
    REQUIRE(failureTypeText(0x11) == "Kurzschluss nach Masse");
    REQUIRE(failureTypeText(0x13) == "Unterbrechung");
    REQUIRE(failureTypeText(0x1C) == "Spannung außerhalb des Bereichs");
    REQUIRE(failureTypeText(0x64) == "Signal unplausibel");
    setLanguage(Language::English);
    REQUIRE(failureTypeText(0x00).empty());
    REQUIRE(failureTypeText(0x11) == "circuit short to ground");
    REQUIRE(failureTypeText(0x12) == "circuit short to battery");
    REQUIRE(failureTypeText(0x29) == "signal invalid");
    REQUIRE(failureTypeText(0x87) == "missing message");
    REQUIRE(failureTypeText(0x92) == "performance or incorrect operation");
    // Subtypes without an entry fall back to their category.
    REQUIRE(failureTypeText(0x2D) == "signal failure");
    REQUIRE(failureTypeText(0x4F) == "system internal failure");
    REQUIRE(failureTypeText(0xF3) == "manufacturer specific failure type");
    REQUIRE(failureTypeText(0xB2) == "failure type 0xB2");
    // Every byte gives something, and nothing but 0x00 is empty.
    for (int i = 1; i < 0x100; ++i) REQUIRE_FALSE(failureTypeText(i).empty());
    setLanguage(Language::German);
}

TEST_CASE("Scanning the demo car finds all six UDS control units", "[uds][demo]") {
    setLanguage(Language::English);
    for (bool headers : {false, true}) {
        DemoRig rig(600.0);
        REQUIRE(rig.connected);
        if (headers) REQUIRE(rig.elm.setHeaders(true));

        std::vector<std::string> names;
        const std::vector<UdsEcu> ecus = scanEcus(
            rig.elm, demoEcus(),
            [&names](int index, int total, const std::string& name) {
                REQUIRE(total == 8);
                REQUIRE(index == static_cast<int>(names.size()));
                names.push_back(name);
            },
            [] { return false; });
        REQUIRE(names.size() == 8);
        REQUIRE(ecus.size() == 8);
        int present = 0;
        for (const UdsEcu& e : ecus) present += e.present ? 1 : 0;
        REQUIRE(present == 6);

        const UdsEcu* engine = findEcu(ecus, 0x7E0);
        REQUIRE(engine->present);
        REQUIRE(engine->dtcsRead);
        REQUIRE(engine->info.partNumber == "CX5 906 018 B");
        REQUIRE(engine->info.systemName == "ECM 1.4 MPI");
        REQUIRE(engine->info.vin == DemoTransport::vin());
        REQUIRE(engine->dtcs.size() == 3);   // P0171, P0420 stored, P0300 pending
        REQUIRE(engine->dtcs[0].code == "P0171");
        REQUIRE(engine->dtcs[0].confirmed);
        REQUIRE(engine->dtcs[0].warningLamp);
        REQUIRE(engine->dtcs[2].code == "P0300");
        REQUIRE(engine->dtcs[2].pending);
        REQUIRE_FALSE(engine->dtcs[2].confirmed);

        const UdsEcu* gearbox = findEcu(ecus, 0x7E1);
        REQUIRE(gearbox->present);
        REQUIRE(gearbox->dtcs.empty());
        REQUIRE(gearbox->info.vin.empty());
        REQUIRE(gearbox->info.softwareVersion == "0203");

        const UdsEcu* brakes = findEcu(ecus, 0x760);
        REQUIRE(brakes->present);
        REQUIRE(brakes->dtcsRead);   // after 7F 19 78
        REQUIRE(brakes->dtcs.size() == 1);
        REQUIRE(brakes->dtcs[0].code == "C0035");
        REQUIRE(brakes->dtcs[0].failureType == 0x1C);
        REQUIRE(brakes->dtcs[0].failureText == "circuit voltage out of range");
        REQUIRE(brakes->dtcs[0].testFailed);
        REQUIRE(brakes->dtcs[0].warningLamp);
        REQUIRE(brakes->dtcs[0].text.find("wheel speed") != std::string::npos);
        REQUIRE(brakes->info.partNumber == "CX1 614 517 C");

        const UdsEcu* airbag = findEcu(ecus, 0x737);
        REQUIRE(airbag->dtcs.size() == 1);
        REQUIRE(airbag->dtcs[0].code == "B0001");
        REQUIRE(airbag->dtcs[0].pending);
        REQUIRE_FALSE(airbag->dtcs[0].confirmed);
        REQUIRE(airbag->dtcs[0].failureText == "circuit resistance below threshold");

        const UdsEcu* cluster = findEcu(ecus, 0x720);
        REQUIRE(cluster->present);
        REQUIRE(cluster->dtcsRead);
        REQUIRE(cluster->dtcs.empty());
        REQUIRE(cluster->note.empty());

        const UdsEcu* climate = findEcu(ecus, 0x744);
        REQUIRE(climate->dtcs.size() == 1);
        REQUIRE(climate->dtcs[0].code == "U0100");
        REQUIRE(climate->dtcs[0].confirmed);
        REQUIRE(climate->dtcs[0].failureText.empty());
        // F195 and F193 stand in for F189 and F191.
        REQUIRE(climate->info.softwareVersion == "0109");
        REQUIRE(climate->info.hardwareVersion == "H04");

        const UdsEcu* nobody = findEcu(ecus, 0x710);
        REQUIRE_FALSE(nobody->present);
        REQUIRE_FALSE(nobody->dtcsRead);
        REQUIRE_FALSE(nobody->note.empty());

        // Normal OBD works afterwards: mode 03 over the functional address.
        Obd2 obd(rig.elm);
        std::string error;
        std::vector<std::string> codes;
        for (const Fault& f : obd.readFaults(error)) codes.push_back(f.code);
        REQUIRE(std::find(codes.begin(), codes.end(), "P0171") != codes.end());
        REQUIRE(std::find(codes.begin(), codes.end(), "P0420") != codes.end());
        REQUIRE(rig.elm.headersOn() == headers);
    }
    setLanguage(Language::German);
}

TEST_CASE("Clearing one UDS control unit leaves the others alone", "[uds][demo]") {
    setLanguage(Language::English);
    DemoRig rig(600.0);
    REQUIRE(rig.connected);
    {
        Uds uds(rig.elm);
        std::string error;
        REQUIRE(uds.select(ecu(0x760, 0x768, "ABS"), error));
        REQUIRE(uds.clearDtcs(error));
        REQUIRE(error.empty());
        REQUIRE(uds.readDtcs(error).empty());

        // Status mask: only confirmed codes of the climate unit, none pending.
        REQUIRE(uds.select(ecu(0x744, 0x74C, "Climate"), error));
        REQUIRE(uds.readDtcs(error, 0x08).size() == 1);
        REQUIRE(uds.readDtcs(error, 0x04).empty());

        // OBD services stay with the engine and gearbox.
        std::vector<int> response;
        REQUIRE_FALSE(uds.request({0x09, 0x02}, response, error));
        REQUIRE(error.find("0x11") != std::string::npos);
        // Unknown identifiers are out of range.
        REQUIRE_FALSE(uds.request({0x22, 0x12, 0x34}, response, error));
        REQUIRE(error.find("0x31") != std::string::npos);
    }
    const std::vector<UdsEcu> ecus = scan(rig.elm, demoEcus());
    REQUIRE(findEcu(ecus, 0x760)->dtcs.empty());
    REQUIRE(findEcu(ecus, 0x737)->dtcs.size() == 1);
    REQUIRE(findEcu(ecus, 0x744)->dtcs.size() == 1);
    REQUIRE(findEcu(ecus, 0x7E0)->dtcs.size() == 3);

    // Clearing the engine over UDS is the same as mode 04.
    {
        Uds uds(rig.elm);
        std::string error;
        REQUIRE(uds.select(ecu(0x7E0, 0x7E8, "Engine"), error));
        REQUIRE(uds.clearDtcs(error));
    }
    Obd2 obd(rig.elm);
    std::string error;
    REQUIRE(obd.readFaults(error).size() == 2);   // only the permanent codes stay
    for (const Fault& f : obd.readFaults(error)) REQUIRE(f.kind == "permanent");
    setLanguage(Language::German);
}

TEST_CASE("A scan can be cancelled and still restores the addressing", "[uds][demo]") {
    DemoRig rig(600.0);
    REQUIRE(rig.connected);
    int asked = 0;
    const std::vector<UdsEcu> ecus = scanEcus(
        rig.elm, demoEcus(), [&asked](int, int, const std::string&) { ++asked; },
        [&asked] { return asked >= 2; });
    REQUIRE(ecus.size() == 2);
    REQUIRE(rig.sent.back().find("ATST64") != std::string::npos);
    bool restored = false;
    for (const std::string& line : rig.sent) restored = restored || line.find("ATSH7DF") != std::string::npos;
    REQUIRE(restored);

    // Functional OBD answers as before, from both OBD control units.
    const ElmResult r = rig.elm.request("0100", 0x01);
    REQUIRE(r.ok());
    REQUIRE(r.messages.size() == 2);
}

TEST_CASE("Without CAN there is no UDS", "[uds]") {
    ScriptRig rig('5');
    REQUIRE(rig.connected);
    const std::vector<UdsEcu> ecus = scan(rig.elm, {ecu(0x760, 0x768, "ABS")});
    REQUIRE(ecus.size() == 1);
    REQUIRE_FALSE(ecus[0].present);
    REQUIRE_FALSE(ecus[0].note.empty());
}

// ---------------------------------------------------------------------------
// BMW style extended addressing: request 6F1 with the target byte first,
// answer from 6xx with F1 first. The adapter runs with CAN auto formatting
// off, so the ISO-TP frames below are the raw eight bytes on the bus.

namespace {

EcuDefinition bmwUnit(int address, const std::string& name) {
    EcuDefinition d = ecuFromIds(0x6F1, 0x600u + static_cast<std::uint32_t>(address), false, name);
    return d;
}

// The headers-on form of a raw BMW frame from unit 12.
std::string dme(const std::string& bytes) { return "612 " + bytes; }

}  // namespace

TEST_CASE("BMW addressing: ids, raw mode and the way back", "[uds][bmw]") {
    const EcuDefinition d = bmwUnit(0x12, "DME");
    REQUIRE(d.extendedAddress == 0x12);
    REQUIRE(ecuFromIds(0x7E0, 0x7E8, false, "x").extendedAddress == -1);
    REQUIRE(ecuFromIds(0x6F1, 0x7E8, false, "x").extendedAddress == -1);

    ScriptRig rig;
    REQUIRE(rig.connected);
    {
        Uds uds(rig.elm);
        std::string error;
        REQUIRE(uds.select(d, error));
        REQUIRE(rig.t->count("ATSH6F1") == 1);
        REQUIRE(rig.t->count("ATCRA612") == 1);
        REQUIRE(rig.t->count("ATCAF0") == 1);
    }
    // Back to normal OBD: auto formatting on, functional header.
    REQUIRE(rig.t->count("ATCAF1") >= 1);
    REQUIRE(rig.t->sent.back() == "ATCRA");
    REQUIRE(std::find(rig.t->sent.rbegin(), rig.t->sent.rend(), "ATSH7DF") != rig.t->sent.rend());
}

TEST_CASE("BMW addressing: single and multi frame answers", "[uds][bmw]") {
    for (bool headers : {false, true}) {
        ScriptRig rig;
        REQUIRE(rig.connected);
        if (headers) REQUIRE(rig.elm.setHeaders(true));
        auto line = [headers](const std::string& bytes) { return headers ? dme(bytes) : bytes; };

        // 3E 00 as a single frame: 12 02 3E 00, padded to eight bytes.
        rig.t->replies["12023E0000000000"] = {{{"F1 02 7E 00 00 00 00 00"}, {dme("F1 02 7E 00 00 00 00 00")}}};
        // VIN, 20 bytes: first frame with five, then 6 + 6 + 3.
        rig.t->replies["120322F190000000"] = {
            {{"F1 10 14 62 F1 90 57 42"}, {line("F1 10 14 62 F1 90 57 42")}}};
        rig.t->replies["1230000000000000"] = {
            {{"F1 21 41 34 4A 31 43 35", "F1 22 30 4B 42 4D 31 32", "F1 23 33 34 35 00 00 00"},
             {line("F1 21 41 34 4A 31 43 35"), line("F1 22 30 4B 42 4D 31 32"),
              line("F1 23 33 34 35 00 00 00")}}};

        Uds uds(rig.elm);
        std::string error;
        REQUIRE(uds.select(bmwUnit(0x12, "DME"), error));
        REQUIRE(uds.testerPresent());

        std::vector<int> response;
        REQUIRE(uds.request({0x22, 0xF1, 0x90}, response, error));
        REQUIRE(response.size() == 19);
        REQUIRE(uds.readInfo().vin == "WBA4J1C50KBM12345");
    }
}

TEST_CASE("BMW addressing: fault codes in BMW's own format", "[uds][bmw]") {
    ScriptRig rig;
    REQUIRE(rig.connected);
    // 59 02 FF, then 480A10 status 2F and 930B21 status 08: eleven bytes.
    rig.t->replies["12031902FF000000"] = {{{"F1 10 0B 59 02 FF 48 0A"}, {dme("F1 10 0B 59 02 FF 48 0A")}}};
    rig.t->replies["1230000000000000"] = {{{"F1 21 10 2F 93 0B 21 08"}, {dme("F1 21 10 2F 93 0B 21 08")}}};

    Uds uds(rig.elm);
    std::string error;
    REQUIRE(uds.select(bmwUnit(0x12, "DME"), error));
    const std::vector<UdsDtc> dtcs = uds.readDtcs(error);
    REQUIRE(error.empty());
    REQUIRE(dtcs.size() == 2);
    REQUIRE(dtcs[0].code == "480A10");
    REQUIRE(dtcs[0].testFailed);
    REQUIRE(dtcs[0].confirmed);
    REQUIRE(dtcs[0].failureText.empty());
    REQUIRE_FALSE(dtcs[0].text.empty());
    REQUIRE(dtcs[1].code == "930B21");
    REQUIRE_FALSE(dtcs[1].testFailed);
    REQUIRE(dtcs[1].confirmed);
}

TEST_CASE("BMW addressing: busy, refusals and foreign frames", "[uds][bmw]") {
    ScriptRig rig;
    REQUIRE(rig.connected);
    // First 7F 22 78 (still busy), the second ask gets the answer.
    rig.t->replies["120322F187000000"] = {{{"F1 03 7F 22 78 00 00 00"}, {}},
                                          {{"F1 06 62 F1 87 41 42 43"}, {}}};
    // A refusal: request out of range.
    rig.t->replies["120322F197000000"] = {{{"F1 03 7F 22 31 00 00 00"}, {}}};
    // A frame for another tester (F4, an ENET tool) is not ours.
    rig.t->replies["120322F18C000000"] = {{{"F4 06 62 F1 8C 41 42 43"}, {}}};

    Uds uds(rig.elm);
    std::string error;
    REQUIRE(uds.select(bmwUnit(0x12, "DME"), error));
    std::vector<int> response;
    REQUIRE(uds.request({0x22, 0xF1, 0x87}, response, error));
    REQUIRE(response == std::vector<int>{0xF1, 0x87, 0x41, 0x42, 0x43});
    REQUIRE(rig.t->count("120322F187000000") == 2);

    REQUIRE_FALSE(uds.request({0x22, 0xF1, 0x97}, response, error));
    REQUIRE_FALSE(error.empty());
    REQUIRE_FALSE(uds.request({0x22, 0xF1, 0x8C}, response, error));

    // Requests longer than one single frame are refused before sending.
    REQUIRE_FALSE(uds.request({0x22, 0xF1, 0x90, 0xF1, 0x87, 0xF1, 0x89}, response, error));
}

TEST_CASE("BMW address list and full sweep", "[uds][bmw]") {
    const std::vector<EcuDefinition> list = ecuDefinitions("BMW");
    REQUIRE(list.size() >= 20);
    REQUIRE(list.front().kind == EcuKind::Engine);
    REQUIRE(list.front().extendedAddress == 0x12);
    for (const EcuDefinition& d : list) {
        REQUIRE(d.request == 0x6F1);
        REQUIRE(d.response == 0x600u + static_cast<std::uint32_t>(d.extendedAddress));
        REQUIRE_FALSE(d.extended29);
    }
    const std::vector<EcuDefinition> sweep = bmwAddressSweep();
    REQUIRE(sweep.size() == 128);
    std::set<int> addresses;
    for (const EcuDefinition& d : sweep) addresses.insert(d.extendedAddress);
    REQUIRE(addresses.size() == 128);
    REQUIRE(*addresses.begin() == 0x00);
    REQUIRE(*addresses.rbegin() == 0x7F);
    // The known units keep their names and come first.
    REQUIRE(sweep.front().extendedAddress == 0x12);
    REQUIRE(sweep.front().name == list.front().name);
}
