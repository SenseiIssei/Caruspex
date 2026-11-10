// Broadcast values: DBC signals (Intel and Motorola order, signed, with a
// multiplexer), listening with ATMA, the listen-only connection to a car
// without OBD answers, and the Tesla pack end to end. Frame bytes are
// worked out by hand from the DBC definitions.

#include <catch2/catch_test_macros.hpp>

#include <cctype>
#include <cmath>
#include <map>
#include <memory>
#include <string>
#include <vector>

#include "core/CanSignals.h"
#include "core/CustomPids.h"
#include "core/Elm327.h"
#include "core/Lang.h"
#include "core/Obd2.h"
#include "core/Transport.h"
#include "data/PidPacks.h"

using namespace caruspex;

namespace {

bool near(double a, double b) { return std::fabs(a - b) < 1e-6; }

// An ELM327 on a bus where nobody answers OBD, but frames go by.
class ListeningBus : public ITransport {
public:
    std::map<int, std::vector<std::vector<int>>> frames;   // by id, sent in turn

    bool write(const std::string& data) override {
        std::string cmd;
        for (char c : data) {
            if (c == '\r' || c == ' ') continue;
            cmd += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
        }
        if (monitoring_) {
            // Any character stops the monitor.
            monitoring_ = false;
            pending_ += "STOPPED\r\r>";
            return true;
        }
        std::vector<std::string> lines;
        if (cmd == "ATZ") {
            lines = {"ELM327 v1.5"};
        } else if (cmd == "ATH1" || cmd == "ATH0") {
            headers_ = cmd == "ATH1";
            lines = {"OK"};
        } else if (cmd == "ATCRA" || cmd == "ATAR") {
            filter_ = -1;
            lines = {"OK"};
        } else if (cmd.rfind("ATCRA", 0) == 0) {
            filter_ = std::stoi(cmd.substr(5), nullptr, 16);
            lines = {"OK"};
        } else if (cmd == "ATMA") {
            monitoring_ = true;
            for (int round = 0; round < 3; ++round) {
                for (const auto& [id, list] : frames) {
                    if (filter_ >= 0 && id != filter_) continue;
                    for (const std::vector<int>& f : list) {
                        char head[8];
                        std::snprintf(head, sizeof(head), "%03X ", id);
                        std::string line = headers_ ? head : "";
                        for (int b : f) {
                            char hex[4];
                            std::snprintf(hex, sizeof(hex), "%02X ", b);
                            line += hex;
                        }
                        pending_ += line + "\r";
                    }
                }
            }
            return true;
        } else if (cmd.rfind("AT", 0) == 0) {
            lines = {"OK"};
        } else if (cmd.rfind("ST", 0) == 0) {
            lines = {"?"};
        } else {
            lines = {"NO DATA"};
        }
        for (const std::string& l : lines) pending_ += l + "\r";
        pending_ += "\r>";
        return true;
    }
    std::string read(int) override {
        std::string out;
        out.swap(pending_);
        return out;
    }
    void flushInput() override { pending_.clear(); }
    std::string name() const override { return "Listening bus"; }

private:
    std::string pending_;
    bool headers_ = false;
    bool monitoring_ = false;
    int filter_ = -1;
};

std::vector<int> ascii(int mux, const char* text) {
    std::vector<int> f = {mux};
    for (const char* p = text; *p; ++p) f.push_back(*p == '_' ? 0 : static_cast<unsigned char>(*p));
    return f;
}

// A Model 3 as the DBC describes its frames.
std::unique_ptr<ListeningBus> tesla() {
    auto bus = std::make_unique<ListeningBus>();
    // 132: 384.16 V (0x9610), smoothed current raw -200 -> 20.0 A.
    bus->frames[0x132] = {{0x10, 0x96, 0x38, 0xFF, 0x00, 0x00, 0x00, 0x00}};
    // 292: SOCmin 80.0, SOCUI 80.5, SOCmax 81.0, SOCave 80.7, BOL 82.0 kWh.
    //   bits 0-9 = 800, 10-19 = 805, 20-29 = 810, 30-39 = 807, 40-49 = 820
    {
        std::uint64_t v = 800ull | (805ull << 10) | (810ull << 20) | (807ull << 30) | (820ull << 40);
        std::vector<int> f;
        for (int i = 0; i < 8; ++i) f.push_back(static_cast<int>((v >> (8 * i)) & 0xFF));
        bus->frames[0x292] = {f};
    }
    // 352: full pack 74.6 kWh (746), remaining 60.1 kWh (601 at bit 11).
    {
        std::uint64_t v = 746ull | (601ull << 11);
        std::vector<int> f;
        for (int i = 0; i < 8; ++i) f.push_back(static_cast<int>((v >> (8 * i)) & 0xFF));
        bus->frames[0x352] = {f};
    }
    // 332: multiplexer 1 with cells 4.100 V / 4.080 V, multiplexer 0 with
    // temperatures 25 and 20 degrees.
    bus->frames[0x332] = {{0x09, 0x20, 0xF8, 0x07, 0x00, 0x00}, {0x00, 0x00, 0x82, 0x78, 0x00, 0x00}};
    // 405: the VIN in three parts.
    bus->frames[0x405] = {ascii(0x10, "____5YJ"), ascii(0x11, "3E1EA7K"), ascii(0x12, "F123456")};
    return bus;
}

}  // namespace

TEST_CASE("DBC signals are parsed", "[can]") {
    CanSignal s;
    std::string error;
    REQUIRE(parseCanSignal("DBC:0|16@1+(0.01,0)", s, error));
    REQUIRE(s.start == 0);
    REQUIRE(s.length == 16);
    REQUIRE(s.intel);
    REQUIRE_FALSE(s.isSigned);
    REQUIRE(near(s.factor, 0.01));
    REQUIRE(s.muxValue == -1);

    REQUIRE(parseCanSignal("DBC:16|16@1-(-0.1,0)", s, error));
    REQUIRE(s.isSigned);
    REQUIRE(near(s.factor, -0.1));

    REQUIRE(parseCanSignal("DBC:2|12@1+(0.002,0) m1@0|2", s, error));
    REQUIRE(s.muxValue == 1);
    REQUIRE(s.muxStart == 0);
    REQUIRE(s.muxLength == 2);
    REQUIRE(s.muxIntel);

    REQUIRE(parseCanSignal("DBC:7|8@0+(0.5,-40)", s, error));
    REQUIRE_FALSE(s.intel);
    REQUIRE(near(s.offset, -40));

    REQUIRE_FALSE(parseCanSignal("0|16@1+(1,0)", s, error));
    REQUIRE_FALSE(parseCanSignal("DBC:0|16@2+(1,0)", s, error));
    REQUIRE_FALSE(parseCanSignal("DBC:0|99@1+(1,0)", s, error));
    REQUIRE_FALSE(error.empty());

    int id = 0;
    REQUIRE(parseMonitorRequest("CAN132", id));
    REQUIRE(id == 0x132);
    REQUIRE(parseMonitorRequest("can3b6", id));
    REQUIRE(id == 0x3B6);
    REQUIRE_FALSE(parseMonitorRequest("CAN800", id));
    REQUIRE_FALSE(parseMonitorRequest("220101", id));
}

TEST_CASE("DBC signals are decoded in both byte orders", "[can]") {
    CanSignal s;
    std::string error;
    double v = 0;
    REQUIRE(parseCanSignal("DBC:0|16@1+(0.01,0)", s, error));
    REQUIRE(decodeCanSignal(s, {0x10, 0x96}, v));
    REQUIRE(near(v, 384.16));
    REQUIRE_FALSE(decodeCanSignal(s, {0x10}, v));   // frame too short

    REQUIRE(parseCanSignal("DBC:16|16@1-(-0.1,0)", s, error));
    REQUIRE(decodeCanSignal(s, {0, 0, 0x38, 0xFF}, v));
    REQUIRE(near(v, 20.0));

    // Motorola: 7|16 is byte 0 then byte 1; 4|8 runs from bit 4 of byte 0
    // down and on into bits 7 to 5 of byte 1.
    REQUIRE(parseCanSignal("DBC:7|16@0+(1,0)", s, error));
    REQUIRE(decodeCanSignal(s, {0x12, 0x34}, v));
    REQUIRE(near(v, 0x1234));
    REQUIRE(parseCanSignal("DBC:4|8@0+(1,0)", s, error));
    REQUIRE(decodeCanSignal(s, {0x16, 0xA0}, v));
    REQUIRE(near(v, 0xB5));

    // Multiplexed: only in frames with the right multiplexer value.
    REQUIRE(parseCanSignal("DBC:2|12@1+(0.002,0) m1@0|2", s, error));
    REQUIRE(decodeCanSignal(s, {0x09, 0x20, 0xF8, 0x07, 0x00, 0x00}, v));
    REQUIRE(near(v, 4.1));
    REQUIRE_FALSE(decodeCanSignal(s, {0x00, 0x00, 0x82, 0x78, 0x00, 0x00}, v));
    REQUIRE(parseCanSignal("DBC:16|8@1+(0.5,-40) m0@0|2", s, error));
    REQUIRE(decodeCanSignal(s, {0x00, 0x00, 0x82, 0x78, 0x00, 0x00}, v));
    REQUIRE(near(v, 25.0));
}

TEST_CASE("A car without OBD answers but with bus traffic is connected for listening", "[can][tesla]") {
    setLanguage(Language::English);
    for (bool headers : {false, true}) {
        INFO("headers " << headers);
        Elm327 elm;
        std::string error;
        REQUIRE(elm.connect(tesla(), "6", error));
        REQUIRE(elm.listenOnly());
        REQUIRE(elm.protocolText().find("listen only") != std::string::npos);
        if (headers) REQUIRE(elm.setHeaders(true));

        const std::vector<std::vector<int>> frames = elm.captureFrames(0x332, 300, 10);
        REQUIRE(frames.size() >= 2);
        REQUIRE(frames.front().size() == 6);

        Obd2 obd(elm);
        REQUIRE(obd.readVin() == "5YJ3E1EA7KF123456");

        // The Tesla pack through the ordinary reader.
        const std::vector<CustomPid> pack = packPids("tesla-model3-y");
        REQUIRE(pack.size() >= 10);
        CustomPidReader reader(elm);
        std::map<std::string, double> values;
        for (const CustomReading& r : reader.readAll(pack)) {
            if (!r.ok) continue;
            for (const CustomPid& p : pack)
                if (p.key == r.value.key) values[p.shortName] = r.value.value;
        }
        REQUIRE(near(values["Batt V"], 384.16));
        REQUIRE(near(values["Batt A"], 20.0));
        REQUIRE(near(values["SOC display"], 80.5));
        REQUIRE(near(values["SOC BMS"], 80.7));
        REQUIRE(near(values["BOL kWh"], 82.0));
        REQUIRE(near(values["Full kWh"], 74.6));
        REQUIRE(near(values["Energy kWh"], 60.1));
        REQUIRE(near(values["Cell V max"], 4.1));
        REQUIRE(near(values["Cell V min"], 4.08));
        REQUIRE(near(values["Batt T max"], 25.0));
        REQUIRE(near(values["Batt T min"], 20.0));
        // Not on this bus: no value, an explanation instead.
        REQUIRE(values.count("Speed") == 0);
    }
}

TEST_CASE("Broadcast values in an imported CSV", "[can]") {
    std::vector<std::string> errors;
    const std::vector<CustomPid> pids = parseTorqueCsv(
        "Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header\n"
        "Pack voltage,PackV,CAN132,\"DBC:0|16@1+(0.01,0)\",0,500,V,\n"
        "Broken,X,CAN132,\"DBC:nonsense\",0,1,V,\n",
        errors);
    REQUIRE(pids.size() == 1);
    REQUIRE(pids[0].request == "CAN132");
    REQUIRE(errors.size() == 1);
}
