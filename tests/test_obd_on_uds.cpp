// OBD on UDS (SAE J1979-2): a car that refuses the classic services and
// answers the emission data only over UDS. The translation in Elm327 has to
// give Obd2 the same results as a classic car. Byte examples worked out by
// hand from SAE J1979-2 / ISO 14229 (0x22 F4xx and F8xx, 0x19 42 / 55, 0x14).

#include <catch2/catch_test_macros.hpp>

#include <cctype>
#include <algorithm>
#include <map>
#include <set>
#include <memory>
#include <string>
#include <vector>

#include "core/Elm327.h"
#include "core/Lang.h"
#include "core/Obd2.h"
#include "core/Transport.h"

using namespace caruspex;

namespace {

class UdsOnlyCar : public ITransport {
public:
    struct Reply {
        std::vector<std::string> plain;
        std::vector<std::string> headers;
    };
    std::map<std::string, std::vector<Reply>> replies;   // given out in turn, the last one stays
    std::vector<std::string> sent;

    bool write(const std::string& data) override {
        std::string cmd;
        for (char c : data) {
            if (c == '\r' || c == ' ') continue;
            cmd += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
        }
        sent.push_back(cmd);
        std::vector<std::string> lines;
        if (cmd == "ATZ") {
            lines = {"ELM327 v2.1"};
            headers_ = false;
        } else if (cmd == "ATH1" || cmd == "ATH0") {
            headers_ = cmd == "ATH1";
            lines = {"OK"};
        } else if (cmd == "ATDP") {
            lines = {"ISO 15765-4 (CAN 11/500)"};
        } else if (cmd.rfind("AT", 0) == 0) {
            lines = {"OK"};
        } else if (cmd.rfind("ST", 0) == 0) {
            lines = {"?"};
        } else if (cmd.size() >= 2 && cmd[0] == '0') {
            // Every classic service is refused.
            lines = {headers_ ? "7E8 03 7F " + cmd.substr(0, 2) + " 11" : "7F " + cmd.substr(0, 2) + " 11"};
        } else {
            auto it = replies.find(cmd);
            if (it == replies.end() || it->second.empty()) {
                lines = {"NO DATA"};
            } else {
                const Reply r = it->second.front();
                if (it->second.size() > 1) it->second.erase(it->second.begin());
                lines = headers_ ? r.headers : r.plain;
            }
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
    std::string name() const override { return "UDS only"; }

    size_t count(const std::string& cmd) const {
        size_t n = 0;
        for (const std::string& s : sent) n += s == cmd ? 1 : 0;
        return n;
    }

private:
    std::string pending_;
    bool headers_ = false;
};

std::unique_ptr<UdsOnlyCar> scriptedCar() {
    auto car = std::make_unique<UdsOnlyCar>();
    auto& r = car->replies;
    r["22F400"] = {{{"62 F4 00 BE 3F A8 13"}, {"7E8 07 62 F4 00 BE 3F A8 13"}}};
    r["22F401"] = {{{"62 F4 01 81 07 65 04"}, {"7E8 07 62 F4 01 81 07 65 04"}}};
    r["22F40C"] = {{{"62 F4 0C 0C 80"}, {"7E8 05 62 F4 0C 0C 80 AA AA"}}};
    // The first answer is "busy", the second the value.
    r["22F405"] = {{{"7F 22 78"}, {"7E8 03 7F 22 78 AA AA AA AA"}},
                   {{"62 F4 05 7B"}, {"7E8 04 62 F4 05 7B AA AA AA"}}};
    // VIN in F802 without the count byte of service 09: 20 bytes.
    r["22F802"] = {{{"014", "0: 62 F8 02 57 41 55", "1: 5A 5A 5A 38 56 35 4C", "2: 41 30 31 32 33 34 35"},
                    {"7E8 10 14 62 F8 02 57 41 55", "7E8 21 5A 5A 5A 38 56 35 4C",
                     "7E8 22 41 30 31 32 33 34 35"}}};
    // Two confirmed emission DTCs, WWH-OBD records: severity, DTC (3), status.
    // 59 42 33 | avail FF | sev avail E0 | format 04 | 20 01 71 00 08 | 40 04 20 00 08
    r["19423308FF"] = {{{"010", "0: 59 42 33 FF E0 04", "1: 20 01 71 00 08 40 04", "2: 20 00 08"},
                        {"7E8 10 10 59 42 33 FF E0 04", "7E8 21 20 01 71 00 08 40 04",
                         "7E8 22 20 00 08 AA AA AA AA"}}};
    r["19423304FF"] = {{{"59 42 33 FF E0 04"}, {"7E8 06 59 42 33 FF E0 04 AA"}}};
    // One permanent DTC: 59 55 33 | avail FF | format 04 | 03 00 00 08
    r["195533"] = {{{"59 55 33 FF 04 03 00 00 08"}, {"7E8 10 09 59 55 33 FF 04 03", "7E8 21 00 00 08 AA AA AA AA"}}};
    r["14FFFF33"] = {{{"54"}, {"7E8 01 54 AA AA AA AA AA AA"}}};
    return car;
}

}  // namespace

TEST_CASE("OBD on UDS: detected at connect, classic results from UDS answers", "[obd][uds][j1979-2]") {
    setLanguage(Language::English);
    for (bool headers : {false, true}) {
        INFO("headers " << headers);
        auto owner = scriptedCar();
        UdsOnlyCar* car = owner.get();
        Elm327 elm;
        std::string error;
        REQUIRE(elm.connect(std::move(owner), "6", error));
        REQUIRE(elm.obdOnUds());
        if (headers) REQUIRE(elm.setHeaders(true));

        Obd2 obd(elm);
        const std::set<int> pids = obd.supportedPids();
        REQUIRE(pids.count(0x0C) == 1);
        REQUIRE(pids.count(0x05) == 1);

        LiveValue rpm;
        REQUIRE(obd.readLive(0x0C, rpm));
        REQUIRE(rpm.value == 800.0);   // 0x0C80 / 4
        LiveValue coolant;
        REQUIRE(obd.readLive(0x05, coolant));   // after one "busy" answer
        REQUIRE(coolant.value == 83.0);
        REQUIRE(car->count("22F405") == 2);

        REQUIRE(obd.readVin() == "WAUZZZ8V5LA012345");

        const std::vector<Fault> faults = obd.readFaults(error);
        std::vector<std::string> codes;
        for (const Fault& f : faults) codes.push_back(f.code);
        REQUIRE(std::find(codes.begin(), codes.end(), "P0171") != codes.end());
        REQUIRE(std::find(codes.begin(), codes.end(), "P0420") != codes.end());
        REQUIRE(std::find(codes.begin(), codes.end(), "P0300") != codes.end());   // permanent

        REQUIRE(obd.clearFaults(error));
        REQUIRE(car->count("14FFFF33") == 1);
        // No classic service was sent after the connection was made.
        REQUIRE(car->count("0300") == 0);
        REQUIRE(car->count("03") == 0);
    }
}

TEST_CASE("OBD on UDS: freeze frames and test results say no data", "[obd][uds][j1979-2]") {
    auto owner = scriptedCar();
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::move(owner), "6", error));
    const ElmResult freeze = elm.request("020C00", 0x02);
    REQUIRE(freeze.status == ElmStatus::NoData);
    const ElmResult tests = elm.request("0600", 0x06);
    REQUIRE(tests.status == ElmStatus::NoData);
}

TEST_CASE("A classic car stays classic", "[obd][uds][j1979-2]") {
    // The UDS probe only runs when 0100 fails.
    auto car = std::make_unique<UdsOnlyCar>();
    UdsOnlyCar* raw = car.get();
    car->replies["0100"] = {};
    Elm327 elm;
    std::string error;
    REQUIRE_FALSE(elm.connect(std::move(car), "6", error));   // refuses 0100, no UDS answer either
    REQUIRE(raw->count("22F400") >= 1);
    REQUIRE_FALSE(elm.obdOnUds());
}
