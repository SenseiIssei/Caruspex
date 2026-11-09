// Tests for the complete OBD-II side: all mode 01 values, several control
// units with headers, mode 09 per control unit and misfires from mode 06.
// The byte examples are worked out by hand from the formulas of SAE J1979.

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include <cctype>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <vector>

#include "core/EcuAddress.h"
#include "core/Elm327.h"
#include "core/Lang.h"
#include "core/Obd2.h"
#include "core/Transport.h"
#include "data/MonitorTests.h"
#include "data/Pids.h"

using namespace caruspex;
using Catch::Approx;

namespace {

// All values of one PID, keyed by sub index.
std::map<int, double> decode(int pid, const std::vector<int>& data) {
    const PidSpec* spec = findPid(pid);
    REQUIRE(spec != nullptr);
    std::map<int, double> out;
    for (const PidReading& r : decodePid(*spec, data)) out[r.spec->sub] = r.value;
    return out;
}

std::string textOf(int key, double value) {
    const PidValueSpec* v = findPidValue(key);
    REQUIRE(v != nullptr);
    REQUIRE(v->text);
    return v->text(value);
}

struct EnglishScope {
    EnglishScope() { setLanguage(Language::English); }
    ~EnglishScope() { setLanguage(Language::German); }
};

// A scripted ELM327. Every OBD request has an answer for headers off and
// one for headers on; ATH0/ATH1 switch between them like a real adapter.
class ScriptTransport : public ITransport {
public:
    struct Reply {
        std::vector<std::string> plain;
        std::vector<std::string> headers;
    };
    std::map<std::string, Reply> replies;
    std::string protocolReply = "41 00 BE 1F A8 13";

    bool write(const std::string& data) override {
        std::string cmd;
        for (char c : data) {
            if (c == '\r' || c == '\n' || c == ' ') continue;
            cmd += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
        }
        if (cmd == "ATZ") {
            headers_ = false;
            pending_ = "ELM327 v1.5\r\r>";
        } else if (cmd == "ATH1") {
            headers_ = true;
            pending_ = "OK\r\r>";
        } else if (cmd == "ATH0") {
            headers_ = false;
            pending_ = "OK\r\r>";
        } else if (cmd.rfind("AT", 0) == 0) {
            pending_ = "OK\r\r>";
        } else if (replies.count(cmd)) {
            const Reply& r = replies[cmd];
            pending_.clear();
            for (const std::string& line : headers_ ? r.headers : r.plain) pending_ += line + "\r";
            pending_ += "\r>";
        } else if (cmd == "0100") {
            pending_ = protocolReply + "\r\r>";
        } else {
            pending_ = "NO DATA\r\r>";
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

private:
    std::string pending_;
    bool headers_ = false;
};

}  // namespace

// ---------------------------------------------------------------------------
// Table structure and key scheme
// ---------------------------------------------------------------------------

TEST_CASE("Every live value has a unique key", "[pids][keys]") {
    REQUIRE(liveKey(0x24, 0) == 0x24);
    REQUIRE(liveKey(0x24, 1) == 0x124);
    REQUIRE(liveKeyPid(0x124) == 0x24);
    REQUIRE(liveKeySub(0x124) == 1);

    std::set<int> keys;
    for (const PidSpec& spec : pidTable()) {
        INFO("PID 0x" << std::hex << spec.pid);
        REQUIRE_FALSE(spec.values.empty());
        // Single-value PIDs keep key == pid, so existing code keeps working.
        REQUIRE(spec.values.front().key == spec.pid);
        REQUIRE(std::string(spec.name) == spec.values.front().name);
        for (const PidValueSpec& v : spec.values) {
            REQUIRE(keys.insert(v.key).second);
            REQUIRE(v.key == liveKey(spec.pid, v.sub));
            REQUIRE(findPidValue(v.key) == &v);
            REQUIRE(std::string(v.name).size() > 2);
            REQUIRE(v.needBytes >= 1);
            REQUIRE(v.calc);
        }
    }
    REQUIRE(pidTable().size() >= 140);
    REQUIRE(pidValueCount() >= 300);    REQUIRE(static_cast<int>(keys.size()) == pidValueCount());
}

TEST_CASE("The table covers the standard up to PID C4", "[pids]") {
    for (int pid : {0x01, 0x03, 0x13, 0x1D, 0x1E, 0x34, 0x3B, 0x4F, 0x50, 0x55, 0x58, 0x5F,
                    0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F,
                    0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x7B,
                    0x7C, 0x7D, 0x7E, 0x7F, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x89,
                    0x8A, 0x8B, 0x8D, 0x8E, 0x98, 0x99, 0x9A, 0x9B, 0x9D, 0x9E, 0xA1, 0xA2,
                    0xA4, 0xA5, 0xA6}) {
        INFO("PID 0x" << std::hex << pid);
        REQUIRE(findPid(pid) != nullptr);
    }
    // The range markers are no live values.
    REQUIRE(findPid(0x20) == nullptr);
    REQUIRE(findPid(0xA0) == nullptr);
}

TEST_CASE("Names exist in both languages without dashes", "[pids][lang]") {
    auto check = [] {
        for (const PidSpec& spec : pidTable()) {
            REQUIRE(std::string(spec.title).find("\xE2\x80\x94") == std::string::npos);
            for (const PidValueSpec& v : spec.values) {
                REQUIRE(std::string(v.name).find("\xE2\x80\x94") == std::string::npos);
            }
        }
        for (PidCategory c : pidCategories()) REQUIRE(std::string(pidCategoryName(c)).size() > 2);
    };
    check();
    REQUIRE(std::string(findPid(0xA6)->name) == "Kilometerstand");
    {
        EnglishScope english;
        check();
        REQUIRE(std::string(findPid(0xA6)->name) == "Odometer");
        REQUIRE(std::string(findPid(0x5D)->unit) == "deg BTDC");
        REQUIRE(textOf(0xA4, 3) == "gear 3");
    }
    REQUIRE(std::string(findPid(0x5D)->unit) == "Grad v.OT");
    REQUIRE(findPid(0x7F)->category == PidCategory::Vehicle);
    REQUIRE(findPid(0x9A)->category == PidCategory::HybridEv);
}

// ---------------------------------------------------------------------------
// Decoders with exact bytes
// ---------------------------------------------------------------------------

TEST_CASE("Status and narrow-band values", "[pids][decode]") {
    // 0x01: 0x83 = lamp on, three codes; byte B bit 3 clear = spark ignition.
    auto status = decode(0x01, {0x83, 0x07, 0xE5, 0x00});
    REQUIRE(status[0] == 1.0);
    REQUIRE(status[1] == 3.0);
    REQUIRE(status[2] == 0.0);
    REQUIRE(textOf(0x01, 1) == "an");

    // 0x03: second fuel system 0 = not present, so only one value.
    REQUIRE(decode(0x03, {0x02, 0x00}).size() == 1);
    REQUIRE(decode(0x03, {0x02, 0x01}).size() == 2);

    // 0x13: bank 1 sensors 1+2, bank 2 sensors 1+2.
    REQUIRE(textOf(0x13, 0x33) == "Bank 1: Sonde 1, 2; Bank 2: Sonde 1, 2");

    // 0x14: 200 * 0.005 V = 1.0 V, trim (128-128)*100/128 = 0 %.
    auto narrow = decode(0x14, {0xC8, 0x80});
    REQUIRE(narrow[0] == Approx(1.0));
    REQUIRE(narrow[1] == Approx(0.0));
    // B = 0xFF: sensor not used for the trim.
    REQUIRE(decode(0x14, {0xC8, 0xFF}).size() == 1);
}

TEST_CASE("Wide-band oxygen sensors", "[pids][decode]") {
    // 0x24: lambda 0x8000 * 2/65536 = 1.0, voltage 0x4000 * 8/65536 = 2.0 V.
    auto v = decode(0x24, {0x80, 0x00, 0x40, 0x00});
    REQUIRE(v[0] == Approx(1.0));
    REQUIRE(v[1] == Approx(2.0));
    REQUIRE(std::string(findPidValue(0x124)->unit) == "V");

    // 0x34: current (CD)/256 - 128 mA: 0x8000 is zero, 0x8100 is +1 mA.
    REQUIRE(decode(0x34, {0x80, 0x00, 0x80, 0x00})[1] == Approx(0.0));
    REQUIRE(decode(0x34, {0x80, 0x00, 0x81, 0x00})[1] == Approx(1.0));
    REQUIRE(decode(0x34, {0x80, 0x00, 0x7F, 0x00})[1] == Approx(-1.0));
    REQUIRE(decode(0x3B, {0x73, 0x33, 0x80, 0x80})[0] == Approx(0x7333 * 2.0 / 65536.0));
}

TEST_CASE("Signed vapor pressures and scaling limits", "[pids][decode]") {
    // 0x32 is a signed 16-bit value with 0.25 Pa per bit.
    REQUIRE(decode(0x32, {0xFF, 0xFC})[0] == Approx(-1.0));
    REQUIRE(decode(0x32, {0x00, 0x04})[0] == Approx(1.0));
    // 0x54 is signed with 1 Pa per bit.
    REQUIRE(decode(0x54, {0xFF, 0xFF})[0] == Approx(-1.0));
    REQUIRE(decode(0x54, {0x7F, 0xFF})[0] == Approx(32767.0));

    // 0x4F: lambda, volt, milliampere as raw bytes, manifold pressure x10.
    auto max = decode(0x4F, {0x02, 0x08, 0x80, 0x19});
    REQUIRE(max[0] == 2.0);
    REQUIRE(max[1] == 8.0);
    REQUIRE(max[2] == 128.0);
    REQUIRE(max[3] == 250.0);
    REQUIRE(decode(0x50, {0x0A, 0x00, 0x00, 0x00})[0] == 100.0);

    // 0x55: secondary trims bank 1 and bank 3.
    auto trim = decode(0x55, {0x80, 0x00});
    REQUIRE(trim[0] == Approx(0.0));
    REQUIRE(trim[1] == Approx(-100.0));
}

TEST_CASE("Fuel, injection and torque", "[pids][decode]") {
    REQUIRE(decode(0x5C, {0x82})[0] == 90.0);                       // A - 40
    REQUIRE(decode(0x5D, {0x69, 0x00})[0] == Approx(0.0));          // 26880/128 - 210
    REQUIRE(decode(0x5D, {0x00, 0x00})[0] == Approx(-210.0));
    REQUIRE(decode(0x5E, {0x00, 0x64})[0] == Approx(5.0));          // 100/20 l/h
    REQUIRE(decode(0x5B, {0xFF})[0] == Approx(100.0));
    REQUIRE(decode(0x61, {0xAF})[0] == 50.0);                       // 175 - 125
    REQUIRE(decode(0x8E, {0x7D})[0] == 0.0);

    auto points = decode(0x64, {0x7D, 0xAF, 0xE1, 0xFF, 0x00});
    REQUIRE(points.size() == 5);
    REQUIRE(points[0] == 0.0);
    REQUIRE(points[1] == 50.0);
    REQUIRE(points[2] == 100.0);
    REQUIRE(points[3] == 130.0);
    REQUIRE(points[4] == -125.0);

    // 0x9D: 0.02 g/s per bit, engine and vehicle.
    auto rate = decode(0x9D, {0x01, 0xF4, 0x00, 0x32});
    REQUIRE(rate[0] == Approx(10.0));
    REQUIRE(rate[1] == Approx(1.0));
    // 0xA2: 1/32 mg per stroke.
    REQUIRE(decode(0xA2, {0x01, 0x40})[0] == Approx(10.0));
}

TEST_CASE("Values with a support byte", "[pids][decode]") {
    // 0x65: PTO active, automatic in park, manual in gear, glow lamp off.
    auto aux = decode(0x65, {0x0F, 0x05});
    REQUIRE(aux.size() == 4);
    REQUIRE(aux[0] == 1.0);
    REQUIRE(aux[1] == 0.0);
    REQUIRE(aux[2] == 1.0);
    REQUIRE(aux[3] == 0.0);

    // 0x66: MAF A = 0x0C80/32 = 100 g/s, B = 0x20/32 = 1 g/s.
    auto maf = decode(0x66, {0x03, 0x0C, 0x80, 0x00, 0x20});
    REQUIRE(maf[0] == Approx(100.0));
    REQUIRE(maf[1] == Approx(1.0));
    REQUIRE(decode(0x66, {0x02, 0x0C, 0x80, 0x00, 0x20}).count(0) == 0);

    // 0x67: both coolant sensors, then only sensor 1.
    auto ect = decode(0x67, {0x03, 0x5A, 0x64});
    REQUIRE(ect[0] == 50.0);
    REQUIRE(ect[1] == 60.0);
    REQUIRE(decode(0x67, {0x01, 0x5A, 0x64}).size() == 1);
    // Only sensor 2: the primary value is missing, sensor 2 is still there.
    auto second = decode(0x67, {0x02, 0x00, 0x64});
    REQUIRE(second.size() == 1);
    REQUIRE(second[1] == 60.0);

    // 0x68 in the short three-byte form of older control units.
    auto iat = decode(0x68, {0x03, 0x46, 0x50});
    REQUIRE(iat.size() == 2);
    REQUIRE(iat[0] == 30.0);
    REQUIRE(iat[1] == 40.0);

    // 0x69: commanded 50.2 %, actual 25.1 %, error (0x90-128)*100/128 = 12.5 %.
    auto egr = decode(0x69, {0x07, 0x80, 0x40, 0x90, 0, 0, 0});
    REQUIRE(egr.size() == 3);
    REQUIRE(egr[0] == Approx(0x80 * 100.0 / 255.0));
    REQUIRE(egr[2] == Approx(12.5));

    // 0x6B: 1 C per bit, 4 C per bit with the wide range bit.
    REQUIRE(decode(0x6B, {0x01, 0x64, 0, 0, 0})[0] == 60.0);
    REQUIRE(decode(0x6B, {0x11, 0x64, 0, 0, 0})[0] == 360.0);

    // 0x6D: rail pressure 10 kPa per bit, fuel temperature A - 40.
    auto rail = decode(0x6D, {0x07, 0x27, 0x10, 0x27, 0x06, 0x50, 0, 0, 0, 0, 0});
    REQUIRE(rail[0] == 100000.0);
    REQUIRE(rail[1] == 99900.0);
    REQUIRE(rail[2] == 40.0);
}

TEST_CASE("Turbocharger and exhaust values", "[pids][decode]") {
    // 0x70: commanded 0x0C80/32 = 100 kPa, actual 0x0C00/32 = 96 kPa,
    // control loop A closed (2).
    auto boost = decode(0x70, {0x13, 0x0C, 0x80, 0x0C, 0x00, 0, 0, 0, 0, 0x02});
    REQUIRE(boost.size() == 3);
    REQUIRE(boost[0] == Approx(100.0));
    REQUIRE(boost[1] == Approx(96.0));
    REQUIRE(boost[4] == 2.0);
    REQUIRE(textOf(liveKey(0x70, 4), 2).find("geregelt") != std::string::npos);

    // 0x73: 0.01 kPa per bit. 0x74: 10 rpm per bit.
    REQUIRE(decode(0x73, {0x01, 0x27, 0x10, 0, 0})[0] == Approx(100.0));
    REQUIRE(decode(0x74, {0x01, 0x27, 0x10, 0, 0})[0] == 100000.0);

    // 0x75: two 1 C bytes, then two 0.1 C words.
    auto turbo = decode(0x75, {0x0F, 0x5A, 0x8C, 0x1F, 0x40, 0x1A, 0x2C});
    REQUIRE(turbo[0] == 50.0);
    REQUIRE(turbo[1] == 100.0);
    REQUIRE(turbo[2] == Approx(760.0));
    REQUIRE(turbo[3] == Approx(630.0));

    // 0x78: only sensor 1 is reported: 8000/10 - 40 = 760 C.
    auto egt = decode(0x78, {0x01, 0x1F, 0x40, 0, 0, 0, 0, 0, 0});
    REQUIRE(egt.size() == 1);
    REQUIRE(egt[0] == Approx(760.0));
    REQUIRE(decode(0x98, {0x08, 0, 0, 0, 0, 0, 0, 0x1F, 0x40})[3] == Approx(760.0));

    // 0x7A: signed differential pressure 0xFF9C = -100 * 0.01 kPa.
    auto dpf = decode(0x7A, {0x07, 0xFF, 0x9C, 0x27, 0x10, 0x27, 0x10});
    REQUIRE(dpf[0] == Approx(-1.0));
    REQUIRE(dpf[1] == Approx(100.0));
    REQUIRE(dpf[2] == Approx(100.0));
    // 0x7C: filter inlet temperature.
    REQUIRE(decode(0x7C, {0x01, 0x1F, 0x40, 0, 0, 0, 0, 0, 0})[0] == Approx(760.0));

    // 0x9E: 0.2 kg/h per bit.
    REQUIRE(decode(0x9E, {0x03, 0xE8})[0] == Approx(200.0));
}

TEST_CASE("Run time, emissions and vehicle values", "[pids][decode]") {
    // 0x7F: total 3 600 000 s = 1000 h, idle 3600 s = 1 h, PTO not reported.
    auto run = decode(0x7F, {0x03, 0x00, 0x36, 0xEE, 0x80, 0x00, 0x00, 0x0E, 0x10, 0, 0, 0, 0});
    REQUIRE(run.size() == 2);
    REQUIRE(run[0] == Approx(1000.0));
    REQUIRE(run[1] == Approx(1.0));

    // 0x83: 500 ppm at bank 1 sensor 1.
    REQUIRE(decode(0x83, {0x01, 0x01, 0xF4, 0, 0, 0, 0, 0, 0})[0] == 500.0);

    // 0x85: 200 * 0.005 = 1 l/h, 100 * 0.005 = 0.5 l/h, tank 0x80, 3600 s.
    auto def = decode(0x85, {0x0F, 0x00, 0xC8, 0x00, 0x64, 0x80, 0x00, 0x00, 0x0E, 0x10});
    REQUIRE(def[0] == Approx(1.0));
    REQUIRE(def[1] == Approx(0.5));
    REQUIRE(def[2] == Approx(0x80 * 100.0 / 255.0));
    REQUIRE(def[3] == 3600.0);

    // 0x86: 0.0125 mg/m3 per bit.
    REQUIRE(decode(0x86, {0x01, 0x00, 0x50, 0, 0})[0] == Approx(1.0));

    // 0x8B: regeneration running, actively triggered, trigger level 50.2 %.
    auto dpf = decode(0x8B, {0x13, 0x03, 0x80, 0, 0, 0, 0});
    REQUIRE(dpf.size() == 3);
    REQUIRE(dpf[0] == 1.0);
    REQUIRE(dpf[1] == 1.0);
    REQUIRE(dpf[4] == Approx(0x80 * 100.0 / 255.0));
    REQUIRE(textOf(0x8B, 1) == "läuft gerade");

    // 0x9A: 0x5DC0/64 = 375 V, current 0xFF38 = -200 * 0.1 A.
    auto ev = decode(0x9A, {0x06, 0x00, 0x5D, 0xC0, 0xFF, 0x38});
    REQUIRE(ev[0] == Approx(375.0));
    REQUIRE(ev[1] == Approx(-20.0));

    // 0xA4: third gear, ratio 3000/1000.
    auto gear = decode(0xA4, {0x03, 0x30, 0x0B, 0xB8});
    REQUIRE(gear[0] == 3.0);
    REQUIRE(gear[1] == Approx(3.0));
    REQUIRE(textOf(0xA4, 3) == "Gang 3");
    REQUIRE(textOf(0xA4, 0) == "Leerlauf");

    // 0xA5: 0.5 % per bit. 0xA6: 0.1 km per bit over four bytes.
    REQUIRE(decode(0xA5, {0x01, 0x64, 0, 0})[0] == Approx(50.0));
    REQUIRE(decode(0xA6, {0x00, 0x01, 0xE2, 0x40})[0] == Approx(12345.6));

    // AECD timers: AECD 1, timer 2 = 0x00010000 s.
    REQUIRE(decode(0x81, std::vector<int>{0x01, 0, 0, 0, 5, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                          0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                          0, 0, 0, 0, 0, 0, 0, 0})[1] == 65536.0);
}

TEST_CASE("Warn limits and gauge ranges", "[pids]") {
    const PidValueSpec* volt = findPidValue(0x42);
    REQUIRE(volt->warns(11.0));
    REQUIRE_FALSE(volt->warns(13.8));
    REQUIRE(volt->displayMax > volt->displayMin);
    const PidValueSpec* tank = findPidValue(0x2F);
    REQUIRE(tank->warns(5.0));
    REQUIRE_FALSE(tank->warns(100.0));
    for (const PidSpec& spec : pidTable()) {
        for (const PidValueSpec& v : spec.values) {
            if (v.text) continue;
            INFO("key 0x" << std::hex << v.key);
            REQUIRE(v.displayMax > v.displayMin);
        }
    }
}

// ---------------------------------------------------------------------------
// Several control units and headers
// ---------------------------------------------------------------------------

TEST_CASE("Control units are named by their address", "[ecu]") {
    REQUIRE(ecuDisplayName("7E8") == "Motor (7E8)");
    REQUIRE(ecuDisplayName("7E9") == "Getriebe (7E9)");
    REQUIRE(ecuDisplayName("7EA") == "Steuergerät (7EA)");
    REQUIRE(ecuDisplayName("18DAF110") == "Motor (18DAF110)");
    REQUIRE(ecuDisplayName("18DAF118") == "Getriebe (18DAF118)");
    REQUIRE(ecuDisplayName("10") == "Motor (10)");
    REQUIRE(ecuDisplayName("1A") == "Getriebe (1A)");
    REQUIRE(ecuDisplayName("").empty());
    EnglishScope english;
    REQUIRE(ecuDisplayName("7E8") == "Engine (7E8)");
    REQUIRE(ecuDisplayName("7E9") == "Transmission (7E9)");
}

TEST_CASE("Headers off: several answers stay apart", "[parser][multi]") {
    // Two single-frame answers to 0100.
    ElmResult single = decodeMessages({"4100BE3FA813", "410098180001"}, 0x01);
    REQUIRE(single.messages.size() == 2);
    REQUIRE(single.sources == std::vector<std::string>{"", ""});
    REQUIRE(single.messages[1][1] == 0x98);

    // Two multi-frame VINs one after the other, each with its length line.
    ElmResult vins = decodeMessages({"014", "0:490201575657", "1:5A5A5A314B5A41",
                                     "2:57303030303031", "014", "0:490201575657",
                                     "1:5A5A5A314B5A41", "2:57303030303032"},
                                    0x09);
    REQUIRE(vins.messages.size() == 2);
    REQUIRE(infoText(infoPayload({vins.messages[0]})) == "WVWZZZ1KZAW000001");
    REQUIRE(infoText(infoPayload({vins.messages[1]})) == "WVWZZZ1KZAW000002");

    // A one-byte answer (clear codes) is not mistaken for a length line.
    REQUIRE(decodeMessages({"44"}, 0x04).ok());
}

TEST_CASE("CAN 11 bit with headers: single and multi frame", "[parser][multi]") {
    // Padding after the single frame length is cut off.
    ElmResult r = decodeMessages({"7E8 06 41 00 BE 3F A8 13 00", "7E9064100981800 01AA"}, 0x01,
                                 HeaderFormat::Can11);
    REQUIRE(r.ok());
    REQUIRE(r.messages.size() == 2);
    REQUIRE(r.sources == std::vector<std::string>{"7E8", "7E9"});
    REQUIRE(r.messages[0] == std::vector<int>{0x00, 0xBE, 0x3F, 0xA8, 0x13});
    REQUIRE(r.messages[1] == std::vector<int>{0x00, 0x98, 0x18, 0x00, 0x01});

    // Two multi-frame VINs, interleaved frame by frame as on a real bus.
    ElmResult vin = decodeMessages(
        {"7E8 10 14 49 02 01 57 56 57", "7E9 10 14 49 02 01 57 56 57",
         "7E8 21 5A 5A 5A 31 4B 5A 41", "7E9 21 5A 5A 5A 31 4B 5A 41",
         "7E9 22 57 30 30 30 30 30 32", "7E8 22 57 30 30 30 30 30 31"},
        0x09, HeaderFormat::Can11);
    REQUIRE(vin.messages.size() == 2);
    REQUIRE(vin.sources == std::vector<std::string>{"7E9", "7E8"});
    REQUIRE(infoText(infoPayload({vin.messages[0]})) == "WVWZZZ1KZAW000002");
    REQUIRE(infoText(infoPayload({vin.messages[1]})) == "WVWZZZ1KZAW000001");

    // Without spaces (ATS0) the same.
    ElmResult compact = decodeMessages({"7E8101449020157565 7", "7E8215A5A5A314B5A41",
                                        "7E82257303030303031"},
                                       0x09, HeaderFormat::Can11);
    REQUIRE(compact.messages.size() == 1);
    REQUIRE(infoText(infoPayload(compact.messages)) == "WVWZZZ1KZAW000001");
}

TEST_CASE("CAN 29 bit with headers", "[parser][multi]") {
    // Engine answers, transmission refuses: the answer counts.
    ElmResult r = decodeMessages({"18DAF11004410C1AF8AAAA", "18DAF118037F0112"}, 0x01,
                                 HeaderFormat::Can29);
    REQUIRE(r.ok());
    REQUIRE(r.messages.size() == 1);
    REQUIRE(r.sources.front() == "18DAF110");
    REQUIRE(r.messages.front() == std::vector<int>{0x0C, 0x1A, 0xF8});

    // Only a refusal: that is the result.
    REQUIRE(decodeMessages({"18DAF118037F0112"}, 0x01, HeaderFormat::Can29).status ==
            ElmStatus::Negative);

    // Four codes in two frames, padding at the end is cut off.
    ElmResult dtc = decodeMessages({"18DAF110 10 0A 43 04 01 33 03 01",
                                    "18DAF110 21 04 20 07 00 AA AA AA"},
                                   0x03, HeaderFormat::Can29);
    REQUIRE(dtc.messages.size() == 1);
    REQUIRE(dtc.messages[0] ==
            std::vector<int>{0x04, 0x01, 0x33, 0x03, 0x01, 0x04, 0x20, 0x07, 0x00});
}

TEST_CASE("K-line and J1850 with headers", "[parser][multi]") {
    // ISO 9141-2: 48 6B source, data, checksum.
    ElmResult iso = decodeMessages({"48 6B 10 41 0C 1A F8 C2", "48 6B 18 41 0C 00 00 00"}, 0x01,
                                   HeaderFormat::KLine);
    REQUIRE(iso.messages.size() == 2);
    REQUIRE(iso.sources == std::vector<std::string>{"10", "18"});
    REQUIRE(iso.messages[0] == std::vector<int>{0x0C, 0x1A, 0xF8});

    // ISO 14230 with the length in the format byte, and with a length byte.
    ElmResult kwp = decodeMessages({"83F110410D32E6", "80F11803410D32F0"}, 0x01,
                                   HeaderFormat::KLine);
    REQUIRE(kwp.messages.size() == 2);
    REQUIRE(kwp.messages[0] == std::vector<int>{0x0D, 0x32});
    REQUIRE(kwp.messages[1] == std::vector<int>{0x0D, 0x32});
    REQUIRE(kwp.sources[1] == "18");

    ElmResult j1850 = decodeMessages({"48 6B 10 43 01 33 00 00 00 00 5A"}, 0x03,
                                     HeaderFormat::J1850);
    REQUIRE(j1850.messages.front() == std::vector<int>{0x01, 0x33, 0, 0, 0, 0});
    REQUIRE(headerFormatFor('6') == HeaderFormat::Can11);
    REQUIRE(headerFormatFor('9') == HeaderFormat::Can29);
    REQUIRE(headerFormatFor('5') == HeaderFormat::KLine);
}

// ---------------------------------------------------------------------------
// Mode 09 decoders
// ---------------------------------------------------------------------------

TEST_CASE("Mode 09 items are decoded", "[mode09]") {
    // Two calibration IDs from one CAN message.
    ElmResult cal = decodeMessages({"023", "0:49040230334C", "1:39303630313844", "2:4E000000000030",
                                    "3:334C3930363031", "4:38585800000000", "5:00"},
                                   0x09);
    REQUIRE(cal.messages.size() == 1);
    REQUIRE(infoCalibrationIds(infoPayload(cal.messages)) ==
            std::vector<std::string>{"03L906018DN", "03L906018XX"});

    // CVNs are binary, shown as hex.
    REQUIRE(infoCvns(infoPayload({{0x06, 0x02, 0x1A, 0x2B, 0x3C, 0x4D, 0x00, 0x00, 0xBE, 0xEF}})) ==
            std::vector<std::string>{"1A2B3C4D", "0000BEEF"});

    // K-line: one CVN per message with a sequence number, out of order.
    REQUIRE(infoCvns(infoPayload({{0x06, 0x02, 0x00, 0x00, 0x00, 0x02},
                                  {0x06, 0x01, 0x00, 0x00, 0x00, 0x01}})) ==
            std::vector<std::string>{"00000001", "00000002"});

    // ECU name with a zero byte between acronym and name.
    ElmResult name = decodeMessages({"017", "0:490A0145434D", "1:002D456E67696E",
                                     "2:65436F6E74726F", "3:6C0000"},
                                    0x09);
    REQUIRE(infoText(infoPayload(name.messages)) == "ECM-EngineControl");

    // In-use performance, spark ignition: 40 OBD cycles, 100 ignitions,
    // catalyst bank 1 checked 12 times in 40 suitable trips.
    std::vector<int> ipt = {0x00, 0x28, 0x00, 0x64, 0x00, 0x0C, 0x00, 0x28};
    ipt.resize(40, 0);
    const InUsePerformance perf = infoInUsePerformance(0x08, ipt);
    REQUIRE(perf.valid);
    REQUIRE(perf.sparkIgnition);
    REQUIRE(perf.obdConditions == 40);
    REQUIRE(perf.ignitionCycles == 100);
    REQUIRE(perf.monitors.size() == 9);
    REQUIRE(perf.monitors[0].name == "Katalysator Bank 1");
    REQUIRE(perf.monitors[0].ratio == Approx(0.3));
    REQUIRE(perf.monitors[0].text.find("0,30") != std::string::npos);
    REQUIRE_FALSE(perf.monitors[1].hasRatio);

    const InUsePerformance diesel = infoInUsePerformance(0x0B, ipt);
    REQUIRE_FALSE(diesel.sparkIgnition);
    REQUIRE(diesel.monitors[0].name == "Oxidationskatalysator (NMHC)");
}

// ---------------------------------------------------------------------------
// End to end over a scripted adapter
// ---------------------------------------------------------------------------

namespace {

std::unique_ptr<ScriptTransport> twoEcuCar() {
    auto t = std::make_unique<ScriptTransport>();
    // Faults: engine P0133 + P0301, transmission P0700.
    t->replies["03"] = {{"43020133 0301", "43010700"},
                        {"7E8 06 43 02 01 33 03 01 AA", "7E9 04 43 01 07 00 AA AA AA"}};
    t->replies["0101"] = {{"41018307E500", "4101000400 00"},
                          {"7E8 06 41 01 83 07 E5 00 AA", "7E9 06 41 01 00 04 00 00 AA"}};
    t->replies["010D"] = {{"410D32", "410D33"}, {"7E8 03 41 0D 32", "7E9 03 41 0D 33"}};
    t->replies["0167"] = {{"4167035A64"}, {"7E8 05 41 67 03 5A 64 AA AA"}};
    t->replies["090A"] = {{"017", "0:490A0145434D", "1:002D456E67696E", "2:65436F6E74726F",
                           "3:6C0000"},
                          {"7E8 10 17 49 0A 01 45 43 4D", "7E9 10 17 49 0A 01 54 43 4D",
                           "7E8 21 00 2D 45 6E 67 69 6E", "7E9 21 00 2D 54 72 61 6E 73",
                           "7E8 22 65 43 6F 6E 74 72 6F", "7E9 22 6D 69 73 73 69 6F 6E",
                           "7E8 23 6C 00 00 AA AA AA AA", "7E9 23 43 74 6C AA AA AA AA"}};
    t->replies["0906"] = {{"490601 1A2B3C4D"}, {"7E8 07 49 06 01 1A 2B 3C 4D"}};
    // Mode 06: monitors A1 and A2 (misfire) on the engine.
    for (const char* base : {"0600", "0620", "0640", "0660", "0680"}) {
        const std::string b = std::string(base).substr(2);
        t->replies[base] = {{"46" + b + "00000001"}, {"7E8 06 46 " + b + " 00 00 00 01 AA"}};
    }
    t->replies["06A0"] = {{"46A0C0000000"}, {"7E8 06 46 A0 C0 00 00 00 AA"}};
    t->replies["06A1"] = {{"013", "0:46A10B240000", "1:0000FFFFA10C24", "2:00030000FFFF"},
                          {"7E8 10 13 46 A1 0B 24 00 00", "7E8 21 00 00 FF FF A1 0C 24",
                           "7E8 22 00 03 00 00 FF FF AA"}};
    t->replies["06A2"] = {{"013", "0:46A20B240002", "1:0000FFFFA20C24", "2:00050000FFFF"},
                          {}};
    return t;
}

}  // namespace

TEST_CASE("Faults and status per control unit", "[obd][multi]") {
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(twoEcuCar(), "6", error));
    REQUIRE(elm.isCan());
    Obd2 obd(elm);

    // Headers off: the codes are found, but nobody knows from whom.
    std::vector<Fault> faults = obd.readFaults(error);
    REQUIRE(faults.size() == 3);
    REQUIRE(faults[0].ecu.empty());

    REQUIRE(elm.setHeaders(true));
    REQUIRE(elm.headersOn());
    faults = obd.readFaults(error);
    REQUIRE(faults.size() == 3);
    REQUIRE(faults[0].code == "P0133");
    REQUIRE(faults[0].ecu == "Motor (7E8)");
    REQUIRE(faults[1].code == "P0301");
    REQUIRE(faults[2].code == "P0700");
    REQUIRE(faults[2].ecu == "Getriebe (7E9)");
    REQUIRE(faults[2].ecuSource == "7E9");

    const std::vector<VehicleStatus> status = obd.readStatusPerEcu();
    REQUIRE(status.size() == 2);
    REQUIRE(status[0].milOn);
    REQUIRE(status[0].faultCount == 3);
    REQUIRE(status[0].ecu == "Motor (7E8)");
    REQUIRE_FALSE(status[1].milOn);
    REQUIRE(obd.readStatus().ecuSource == "7E8");

    // Live values: the first answer (engine) counts, with its sender.
    LiveValue speed;
    REQUIRE(obd.readLive(0x0D, speed));
    REQUIRE(speed.value == 50.0);
    REQUIRE(speed.key == 0x0D);
    REQUIRE(speed.ecuSource == "7E8");
    const std::vector<LiveValue> coolant = obd.readLiveAll(0x67);
    REQUIRE(coolant.size() == 2);
    REQUIRE(coolant[1].key == 0x167);
    REQUIRE(coolant[1].value == 60.0);
    REQUIRE(coolant[1].category == PidCategory::Temperatures);

    // A reset by hand switches headers off again.
    elm.send("ATZ");
    REQUIRE_FALSE(elm.headersOn());
}

TEST_CASE("Mode 09 per control unit", "[obd][mode09]") {
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(twoEcuCar(), "6", error));
    REQUIRE(elm.setHeaders(true));
    Obd2 obd(elm);

    const std::vector<EcuInfo> ecus = obd.readEcuInfo();
    REQUIRE(ecus.size() == 2);
    REQUIRE(ecus[0].ecuSource == "7E8");
    REQUIRE(ecus[0].name == "ECM-EngineControl");
    REQUIRE(ecus[0].cvns == std::vector<std::string>{"1A2B3C4D"});
    REQUIRE(ecus[1].ecu == "Getriebe (7E9)");
    REQUIRE(ecus[1].name == "TCM-TransmissionCtl");

    // The old identity still works and picks the first control unit.
    const VehicleIdentity id = obd.readIdentity();
    REQUIRE(id.ecuName == "ECM-EngineControl");
    REQUIRE(id.calibrationVerification == "1A2B3C4D");
}

TEST_CASE("Calibration IDs of two control units stay apart without headers", "[obd][mode09]") {
    auto car = twoEcuCar();
    // Engine with two IDs, then the transmission with one, each multi-frame.
    car->replies["0904"] = {{"023", "0:49040230334C", "1:39303630313844", "2:4E000000000030",
                             "3:334C3930363031", "4:38585800000000", "5:00", "013",
                             "0:490401303947", "1:3932373735304B", "2:4D0000000000"},
                            {}};
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::move(car), "6", error));
    Obd2 obd(elm);
    REQUIRE(obd.readIdentity().calibrationId == "03L906018DN, 03L906018XX, 09G927750KM");

    // A short ID without padding, as some K-line units send it.
    REQUIRE(infoCalibrationIds(infoPayload({{0x04, 0x01, '0', '3', '6', '9', 'X'}})) ==
            std::vector<std::string>{"0369X"});
}

TEST_CASE("Misfire counts come out of mode 06", "[obd][monitor]") {
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(twoEcuCar(), "6", error));
    Obd2 obd(elm);

    const std::vector<MonitorResult> results = obd.readMonitorResults(error);
    REQUIRE(results.size() == 4);
    REQUIRE(results[0].can);
    REQUIRE(results[0].componentId == 0xA1);
    REQUIRE(results[1].testId == 0x0C);
    REQUIRE(results[1].value == 3.0);
    REQUIRE(results[2].component == "Verbrennungsaussetzer Zylinder 1");
    REQUIRE(results[3].name == "Aussetzer in der aktuellen Fahrt");

    const std::vector<MisfireCount> misfires = misfireCounts(results);
    REQUIRE(misfires.size() == 2);
    REQUIRE(misfires[0].cylinder == 0);
    REQUIRE(misfires[0].currentCycle == 3.0);
    REQUIRE(misfires[1].cylinder == 1);
    REQUIRE(misfires[1].name == "Zylinder 1");
    REQUIRE(misfires[1].currentCycle == 5.0);
    REQUIRE(misfires[1].averageTenCycles == 2.0);
    REQUIRE(misfires[1].text ==
            "Zylinder 1: 5 Aussetzer in der aktuellen Fahrt, im Schnitt 2,0 je Fahrt über "
            "die letzten 10 Fahrten.");

    // K-line style results are left alone.
    MonitorResult kline;
    kline.testId = 0x0C;
    kline.componentId = 0xA2;
    REQUIRE(misfireCounts({kline}).empty());
}

TEST_CASE("Monitor names follow the standard", "[monitor]") {
    REQUIRE(monitorComponentName(0x01) == "Lambdasonde Bank 1 Sonde 1");
    REQUIRE(monitorComponentName(0x06) == "Lambdasonde Bank 2 Sonde 2");
    REQUIRE(monitorComponentName(0x22) == "Katalysator Bank 2");
    REQUIRE(monitorComponentName(0x3C).find("0,020") != std::string::npos);
    REQUIRE(monitorComponentName(0xAD) == "Verbrennungsaussetzer Zylinder 12");
    REQUIRE(monitorComponentName(0xB0) == "Partikelfilter Bank 1");
    REQUIRE(monitorTestNameCan(0x01, 0x01) == oxygenTestName(0x01));
}
