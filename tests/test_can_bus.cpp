// Raw CAN interfaces: the ELM327 front end with its own ISO-TP over a
// scripted car on 11 and 29 bit, the BMW raw frames, the bus monitor, and
// the SLCAN text protocol. Frames worked out by hand from ISO 15765-2/-4.

#include <catch2/catch_test_macros.hpp>

#include <algorithm>
#include <cstdio>
#include <deque>
#include <memory>
#include <string>
#include <vector>

#include "core/CanBus.h"
#include "core/Elm327.h"
#include "core/Lang.h"
#include "core/Obd2.h"
#include "core/Uds.h"
#include "data/EcuAddresses.h"

using namespace caruspex;

namespace {

const char* kVin = "WVWZZZAUZLW000001";

// A car on the bus: engine unit with OBD on 11 and 29 bit, a BMW style
// unit 12 behind 6F1, and a broadcast frame 0x132.
class FakeCar {
public:
    std::deque<CanFrame> out;
    int flowControls = 0;
    std::vector<CanFrame> pendingCf;

    void reply(std::uint32_t id, bool ext, std::vector<std::uint8_t> data) {
        data.resize(8, 0xAA);
        out.push_back({id, ext, data});
    }
    // Answers a request payload; multi frame answers wait for flow control.
    void answer(std::uint32_t id, bool ext, const std::vector<std::uint8_t>& payload, int target = -1) {
        std::vector<std::uint8_t> lead;
        if (target >= 0) lead.push_back(static_cast<std::uint8_t>(target));
        const std::size_t room = 7 - lead.size();
        if (payload.size() <= room) {
            std::vector<std::uint8_t> d = lead;
            d.push_back(static_cast<std::uint8_t>(payload.size()));
            d.insert(d.end(), payload.begin(), payload.end());
            reply(id, ext, d);
            return;
        }
        std::vector<std::uint8_t> ff = lead;
        ff.push_back(static_cast<std::uint8_t>(0x10 | (payload.size() >> 8)));
        ff.push_back(static_cast<std::uint8_t>(payload.size() & 0xFF));
        const std::size_t first = 6 - lead.size();
        ff.insert(ff.end(), payload.begin(), payload.begin() + static_cast<long>(first));
        reply(id, ext, ff);
        pendingCf.clear();
        int n = 1;
        for (std::size_t at = first; at < payload.size(); at += 7 - lead.size(), ++n) {
            std::vector<std::uint8_t> cf = lead;
            cf.push_back(static_cast<std::uint8_t>(0x20 | (n & 0x0F)));
            cf.insert(cf.end(), payload.begin() + static_cast<long>(at),
                      payload.begin() + static_cast<long>(std::min(at + 7 - lead.size(), payload.size())));
            cf.resize(8, 0xAA);
            pendingCf.push_back({id, ext, cf});
        }
    }

    static std::vector<std::uint8_t> vinPayload(std::vector<std::uint8_t> head) {
        for (const char* p = kVin; *p; ++p) head.push_back(static_cast<std::uint8_t>(*p));
        return head;
    }

    void heard(const CanFrame& f) {
        const auto& d = f.data;
        // Flow control from the tester: release the consecutive frames.
        const bool bmw = !f.extended && f.id == 0x6F1;
        const std::size_t pci = bmw ? 1 : 0;
        if (d.size() > pci && (d[pci] >> 4) == 3) {
            ++flowControls;
            for (const CanFrame& cf : pendingCf) out.push_back(cf);
            pendingCf.clear();
            return;
        }
        if (bmw) {
            if (d.size() >= 5 && d[0] == 0x12 && d[2] == 0x22 && d[3] == 0xF1 && d[4] == 0x90)
                answer(0x612, false, vinPayload({0x62, 0xF1, 0x90}), 0xF1);
            if (d.size() >= 4 && d[0] == 0x12 && d[2] == 0x3E) answer(0x612, false, {0x7E, 0x00}, 0xF1);
            if (d.size() >= 5 && d[0] == 0x12 && d[2] == 0x19) answer(0x612, false, {0x59, 0x02, 0xFF}, 0xF1);
            return;
        }
        const bool obd11 = !f.extended && (f.id == 0x7DF || f.id == 0x7E0);
        const bool obd29 = f.extended && (f.id == 0x18DB33F1 || f.id == 0x18DA10F1);
        if (!obd11 && !obd29) return;
        const std::uint32_t id = obd11 ? 0x7E8 : 0x18DAF110;
        const std::vector<std::uint8_t> req(d.begin() + 1, d.begin() + 1 + (d[0] & 0x0F));
        if (req == std::vector<std::uint8_t>{0x01, 0x00}) answer(id, f.extended, {0x41, 0x00, 0xBE, 0x3F, 0xA8, 0x13});
        if (req == std::vector<std::uint8_t>{0x09, 0x02}) answer(id, f.extended, vinPayload({0x49, 0x02, 0x01}));
        if (req == std::vector<std::uint8_t>{0x03}) answer(id, f.extended, {0x43, 0x01, 0x01, 0x71});
        if (req == std::vector<std::uint8_t>{0x07} || req == std::vector<std::uint8_t>{0x0A})
            answer(id, f.extended, {static_cast<std::uint8_t>(req[0] + 0x40), 0x00});
    }
};

class FakeLink : public CanLink {
public:
    explicit FakeLink(std::shared_ptr<FakeCar> car, bool extendedOnly = false) : car_(std::move(car)), only29_(extendedOnly) {}
    bool send(const CanFrame& frame) override {
        sent.push_back(frame);
        if (only29_ && !frame.extended) return true;   // an 11-bit request nobody answers
        car_->heard(frame);
        return true;
    }
    bool receive(CanFrame& frame, int) override {
        if (car_->out.empty()) {
            // Between answers the car keeps broadcasting.
            if (++quiet_ % 3 == 0) {
                frame = {0x132, false, {0x10, 0x96, 0x38, 0xFF, 0, 0, 0, 0}};
                return true;
            }
            return false;
        }
        frame = car_->out.front();
        car_->out.pop_front();
        return true;
    }
    std::string name() const override { return "fake"; }
    double batteryVoltage() override { return 12.6; }
    std::vector<CanFrame> sent;

private:
    std::shared_ptr<FakeCar> car_;
    bool only29_;
    int quiet_ = 0;
};

// An SLCAN stick on a serial port, wired to the same car.
class FakeSlcanPort : public ITransport {
public:
    explicit FakeSlcanPort(std::shared_ptr<FakeCar> car) : car_(std::move(car)) {}
    bool write(const std::string& data) override {
        line_ += data;
        std::size_t end;
        while ((end = line_.find('\r')) != std::string::npos) {
            const std::string cmd = line_.substr(0, end);
            line_.erase(0, end + 1);
            commands.push_back(cmd);
            if (cmd == "V") reply_ += "V1013\r";
            else if (cmd.empty()) reply_ += "\a";
            else if (cmd[0] == 't' || cmd[0] == 'T') {
                const bool ext = cmd[0] == 'T';
                const std::size_t n = ext ? 8 : 3;
                CanFrame f;
                f.extended = ext;
                f.id = static_cast<std::uint32_t>(std::stoul(cmd.substr(1, n), nullptr, 16));
                const int dlc = cmd[1 + n] - '0';
                for (int i = 0; i < dlc; ++i) f.data.push_back(static_cast<std::uint8_t>(std::stoul(cmd.substr(2 + n + 2 * i, 2), nullptr, 16)));
                car_->heard(f);
                reply_ += ext ? "Z\r" : "z\r";
            } else {
                reply_ += "\r";
            }
        }
        return true;
    }
    std::string read(int) override {
        std::string out;
        out.swap(reply_);
        while (!car_->out.empty()) {
            const CanFrame f = car_->out.front();
            car_->out.pop_front();
            char head[16];
            std::snprintf(head, sizeof(head), f.extended ? "T%08X%u" : "t%03X%u", static_cast<unsigned>(f.id),
                          static_cast<unsigned>(f.data.size()));
            out += head;
            for (std::uint8_t b : f.data) {
                char h[4];
                std::snprintf(h, sizeof(h), "%02X", b);
                out += h;
            }
            out += "\r";
        }
        return out;
    }
    void flushInput() override {}
    std::string name() const override { return "COM9"; }
    std::vector<std::string> commands;

private:
    std::shared_ptr<FakeCar> car_;
    std::string line_, reply_;
};

}  // namespace

TEST_CASE("CAN front end: OBD on 11 bit with ISO-TP and flow control", "[can][iso-tp]") {
    setLanguage(Language::English);
    for (bool headers : {false, true}) {
        INFO("headers " << headers);
        auto car = std::make_shared<FakeCar>();
        auto link = std::make_unique<FakeLink>(car);
        FakeLink* raw = link.get();
        Elm327 elm;
        std::string error;
        REQUIRE(elm.connect(std::make_unique<CanElm>(std::move(link), "fake (500 kbit/s)", 500000), "6", error));
        REQUIRE(elm.protocolText() == "ISO 15765-4 (CAN 11/500)");
        if (headers) REQUIRE(elm.setHeaders(true));
        Obd2 obd(elm);
        REQUIRE(obd.readVin() == kVin);
        REQUIRE(car->flowControls >= 1);
        // The flow control went to the engine's request id.
        bool fcTo7E0 = false;
        for (const CanFrame& f : raw->sent)
            if (f.id == 0x7E0 && !f.data.empty() && f.data[0] == 0x30) fcTo7E0 = true;
        REQUIRE(fcTo7E0);
        const std::vector<Fault> faults = obd.readFaults(error);
        REQUIRE(faults.size() == 1);
        REQUIRE(faults[0].code == "P0171");
        REQUIRE(elm.send("ATRV", 1000).lines.front() == "12.6V");
    }
}

TEST_CASE("CAN front end: 29-bit cars are found as well", "[can][iso-tp]") {
    auto car = std::make_shared<FakeCar>();
    Elm327 elm;
    std::string error;
    // The car ignores 11 bit; the protocol order tries 29 bit next.
    REQUIRE(elm.connect(std::make_unique<CanElm>(std::make_unique<FakeLink>(car, true), "fake", 500000), "67", error));
    REQUIRE(elm.protocolText() == "ISO 15765-4 (CAN 29/500)");
    Obd2 obd(elm);
    REQUIRE(obd.readVin() == kVin);
}

TEST_CASE("CAN front end: BMW raw frames and the bus monitor", "[can][bmw]") {
    auto car = std::make_shared<FakeCar>();
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::make_unique<CanElm>(std::make_unique<FakeLink>(car), "fake", 500000), "6", error));
    // The BMW engine unit through 6F1, framing by Uds itself.
    std::vector<EcuDefinition> dme;
    for (const EcuDefinition& d : ecuDefinitions("BMW"))
        if (d.extendedAddress == 0x12) dme.push_back(d);
    const std::vector<UdsEcu> found = scanEcus(elm, dme, nullptr, nullptr);
    REQUIRE(found.size() == 1);
    REQUIRE(found[0].present);
    REQUIRE(found[0].info.vin == kVin);
    // Listening: the broadcast frame 0x132 through the receive filter.
    const auto frames = elm.captureFrames(0x132, 200, 3);
    REQUIRE_FALSE(frames.empty());
    REQUIRE(frames.front().size() == 8);
    REQUIRE(frames.front()[1] == 0x96);
}

TEST_CASE("SLCAN: bit rate, channel and frames in the Lawicel text protocol", "[can][slcan]") {
    auto car = std::make_shared<FakeCar>();
    auto port = std::make_unique<FakeSlcanPort>(car);
    FakeSlcanPort* raw = port.get();
    auto link = std::make_unique<SlcanLink>(std::move(port));
    std::string error;
    REQUIRE(link->open(500000, error));
    REQUIRE(std::find(raw->commands.begin(), raw->commands.end(), "S6") != raw->commands.end());
    REQUIRE(std::find(raw->commands.begin(), raw->commands.end(), "O") != raw->commands.end());

    Elm327 elm;
    REQUIRE(elm.connect(std::make_unique<CanElm>(std::move(link), "SLCAN", 500000), "6", error));
    Obd2 obd(elm);
    REQUIRE(obd.readVin() == kVin);
    // Requests went out as t7DF8 frames.
    bool sent = false;
    for (const std::string& c : raw->commands)
        if (c.rfind("t7DF8", 0) == 0) sent = true;
    REQUIRE(sent);

    auto wrong = std::make_unique<SlcanLink>(std::make_unique<FakeSlcanPort>(car));
    REQUIRE_FALSE(wrong->open(123456, error));
}

TEST_CASE("CAN interface specs are parsed and refused cleanly", "[can]") {
    std::string error;
    REQUIRE(openCanInterface("nonsense:x", error) == nullptr);
    REQUIRE_FALSE(error.empty());
#ifndef __linux__
    REQUIRE(openCanInterface("socketcan:can0", error) == nullptr);
#endif
#ifndef _WIN32
    REQUIRE(openCanInterface("j2534:x.dll", error) == nullptr);
    REQUIRE(j2534Devices().empty());
#endif
}
