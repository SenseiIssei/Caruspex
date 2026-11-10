// BMW ENET (HSFZ): frames, the identification answer, the session with
// acknowledgement, alive check and response pending, and the ELM327 front
// end end to end with Elm327, Obd2 and Uds on top of a scripted car.
// Frame layout after EdiabasLib (EdInterfaceEnet); the byte examples are
// worked out by hand from it and from ISO 14229 / SAE J1979.

#include <catch2/catch_test_macros.hpp>

#include <deque>
#include <functional>
#include <map>
#include <memory>
#include <string>
#include <vector>

#include "core/Elm327.h"
#include "core/Enet.h"
#include "core/Lang.h"
#include "core/Obd2.h"
#include "core/Uds.h"
#include "data/EcuAddresses.h"

using namespace caruspex;

namespace {

ByteVector bytesOf(const std::string& text) {
    return ByteVector(text.begin(), text.end());
}

// The car's side of the TCP connection: every diagnostic frame the session
// sends is handed to respond(), whose frames are queued for receive().
class FakeCarStream : public ByteStream {
public:
    std::function<std::vector<hsfz::Frame>(const hsfz::Frame&)> respond;
    std::vector<hsfz::Frame> sent;
    bool dead = false;

    bool send(const ByteVector& data) override {
        if (dead) return false;
        parser_.feed(data.data(), data.size());
        hsfz::Frame f;
        while (parser_.next(f)) {
            sent.push_back(f);
            if (respond) {
                for (const hsfz::Frame& out : respond(f)) {
                    const ByteVector bytes = hsfz::encode(out.control, out.payload);
                    inbound_.insert(inbound_.end(), bytes.begin(), bytes.end());
                }
            }
        }
        return true;
    }
    int receive(std::uint8_t* buffer, int size, int) override {
        if (inbound_.empty()) return dead ? -1 : 0;
        // Hand out small pieces, as TCP may.
        const int n = std::min<int>(size, std::min<int>(5, static_cast<int>(inbound_.size())));
        for (int i = 0; i < n; ++i) {
            buffer[i] = inbound_.front();
            inbound_.pop_front();
        }
        return n;
    }

private:
    hsfz::Parser parser_;
    std::deque<std::uint8_t> inbound_;
};

hsfz::Frame ack(const hsfz::Frame& request) { return {hsfz::Ack, request.payload}; }

hsfz::Frame answer(int source, const ByteVector& uds) {
    ByteVector payload = {static_cast<std::uint8_t>(source), hsfz::kTester};
    payload.insert(payload.end(), uds.begin(), uds.end());
    return {hsfz::Diagnostic, payload};
}

// A scripted BMW behind DiagLink: answers by unit and request bytes.
class FakeBmw : public DiagLink {
public:
    std::map<std::pair<int, ByteVector>, ByteVector> replies;
    std::vector<std::pair<int, ByteVector>> asked;

    bool request(int target, const ByteVector& data, int, std::vector<DiagAnswer>& answers,
                 std::string& error) override {
        asked.push_back({target, data});
        answers.clear();
        const auto it = replies.find({target, data});
        if (it == replies.end()) {
            error = "no answer";
            return false;
        }
        answers.push_back({target, it->second});
        return true;
    }
};

const char* kVin = "WBA4J1C50KBM12345";

ByteVector vinAnswer(const ByteVector& prefix) {
    ByteVector out = prefix;
    for (const char* p = kVin; *p; ++p) out.push_back(static_cast<std::uint8_t>(*p));
    return out;
}

std::unique_ptr<FakeBmw> scriptedBmw() {
    auto car = std::make_unique<FakeBmw>();
    const int dme = hsfz::kEngine;
    car->replies[{dme, {0x01, 0x00}}] = {0x41, 0x00, 0xBE, 0x3F, 0xA8, 0x13};
    car->replies[{dme, {0x01, 0x42}}] = {0x41, 0x42, 0x31, 0x9C};          // 12.7 V
    car->replies[{dme, {0x03}}] = {0x43, 0x01, 0x01, 0x71};                 // P0171
    car->replies[{dme, {0x09, 0x02}}] = vinAnswer({0x49, 0x02, 0x01});      // 20 bytes
    car->replies[{dme, {0x3E, 0x00}}] = {0x7E, 0x00};
    car->replies[{dme, {0x22, 0xF1, 0x90}}] = vinAnswer({0x62, 0xF1, 0x90});
    car->replies[{dme, {0x19, 0x02, 0xFF}}] = {0x59, 0x02, 0xFF, 0x48, 0x0A, 0x10, 0x2F};
    car->replies[{0x29, {0x3E, 0x00}}] = {0x7E, 0x00};
    car->replies[{0x29, {0x19, 0x02, 0xFF}}] = {0x59, 0x02, 0xFF};
    return car;
}

}  // namespace

TEST_CASE("HSFZ frames: encode, cut from a stream, refuse nonsense", "[enet]") {
    const ByteVector frame = hsfz::encode(hsfz::Diagnostic, {0xF4, 0x12, 0x22, 0xF1, 0x90});
    REQUIRE(frame == ByteVector{0x00, 0x00, 0x00, 0x05, 0x00, 0x01, 0xF4, 0x12, 0x22, 0xF1, 0x90});
    REQUIRE(hsfz::encode(hsfz::Ident, {}) == ByteVector{0, 0, 0, 0, 0, 0x11});

    hsfz::Parser parser;
    hsfz::Frame f;
    parser.feed(frame.data(), 4);
    REQUIRE_FALSE(parser.next(f));
    parser.feed(frame.data() + 4, frame.size() - 4);
    const ByteVector alive = hsfz::encode(hsfz::AliveCheck, {});
    parser.feed(alive.data(), alive.size());
    REQUIRE(parser.next(f));
    REQUIRE(f.control == hsfz::Diagnostic);
    REQUIRE(f.payload == ByteVector{0xF4, 0x12, 0x22, 0xF1, 0x90});
    REQUIRE(parser.next(f));
    REQUIRE(f.control == hsfz::AliveCheck);
    REQUIRE(f.payload.empty());
    REQUIRE_FALSE(parser.next(f));

    hsfz::Parser bad;
    const ByteVector junk = {0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x01};
    bad.feed(junk.data(), junk.size());
    REQUIRE_FALSE(bad.next(f));
    REQUIRE(bad.broken());
}

TEST_CASE("HSFZ vehicle identification answer", "[enet]") {
    ByteVector d = hsfz::encode(hsfz::Ident, {});
    const ByteVector text = bytesOf(std::string("DIAGADR10BMWMAC001A37AABBCCBMWVIN") + kVin);
    d.insert(d.end(), text.begin(), text.end());
    d[3] = static_cast<std::uint8_t>(text.size());
    hsfz::VehicleIdent ident;
    REQUIRE(hsfz::parseIdent(d, ident));
    REQUIRE(ident.diagAddress == 0x10);
    REQUIRE(ident.mac == "001A37AABBCC");
    REQUIRE(ident.vin == kVin);

    // Short answers without the VIN still count, other datagrams do not.
    ByteVector shortOne(d.begin(), d.begin() + 44);
    REQUIRE(hsfz::parseIdent(shortOne, ident));
    REQUIRE(ident.vin.empty());
    REQUIRE_FALSE(hsfz::parseIdent(bytesOf(std::string(60, 'x')), ident));
}

TEST_CASE("HSFZ session: acknowledgement, alive check, pending and refusals", "[enet]") {
    auto stream = std::make_unique<FakeCarStream>();
    FakeCarStream* car = stream.get();
    int pendingRounds = 0;
    int nacks = 0;
    car->respond = [&](const hsfz::Frame& in) -> std::vector<hsfz::Frame> {
        if (in.control != hsfz::Diagnostic) return {};
        const ByteVector uds(in.payload.begin() + 2, in.payload.end());
        const int target = in.payload[1];
        if (uds == ByteVector{0x3E, 0x00}) {
            // Alive check before the answer, and a frame for another tester.
            return {ack(in), {hsfz::AliveCheck, {}}, {hsfz::Diagnostic, {0x12, 0xF1, 0x7E, 0x00}},
                    answer(target, {0x7E, 0x00})};
        }
        if (uds == ByteVector{0x22, 0xF1, 0x87}) {
            ++pendingRounds;
            return {ack(in), answer(target, {0x7F, 0x22, 0x78}), answer(target, {0x7F, 0x22, 0x78}),
                    answer(target, {0x62, 0xF1, 0x87, 0x41})};
        }
        if (uds == ByteVector{0x22, 0xF1, 0x97}) {
            if (nacks++ == 0) return {{hsfz::Nack, {}}};
            return {ack(in), answer(target, {0x62, 0xF1, 0x97, 0x42})};
        }
        if (uds == ByteVector{0x01, 0x00} && target == hsfz::kFunctional) {
            return {ack(in), answer(0x12, {0x41, 0x00, 0xBE}), answer(0x18, {0x41, 0x00, 0x80})};
        }
        if (target == 0x55) return {{0x41, {}}};   // gateway: unknown target
        return {ack(in)};   // acknowledged, never answered
    };
    HsfzSession session(std::move(stream));
    std::vector<DiagAnswer> answers;
    std::string error;

    REQUIRE(session.request(0x12, {0x3E, 0x00}, 500, answers, error));
    REQUIRE(answers.size() == 1);
    REQUIRE(answers[0].source == 0x12);
    REQUIRE(answers[0].data == ByteVector{0x7E, 0x00});
    // The alive check was answered: control 13, payload 00 F4.
    bool replied = false;
    for (const hsfz::Frame& f : car->sent)
        if (f.control == hsfz::AliveReply && f.payload == ByteVector{0x00, 0xF4}) replied = true;
    REQUIRE(replied);
    // The request itself: tester F4, target 12.
    REQUIRE(car->sent.front().payload == ByteVector{0xF4, 0x12, 0x3E, 0x00});

    REQUIRE(session.request(0x12, {0x22, 0xF1, 0x87}, 500, answers, error));
    REQUIRE(answers[0].data == ByteVector{0x62, 0xF1, 0x87, 0x41});
    REQUIRE(pendingRounds == 1);

    REQUIRE(session.request(0x29, {0x22, 0xF1, 0x97}, 500, answers, error));
    REQUIRE(answers[0].source == 0x29);
    REQUIRE(nacks == 2);

    REQUIRE(session.request(hsfz::kFunctional, {0x01, 0x00}, 500, answers, error));
    REQUIRE(answers.size() == 2);

    REQUIRE_FALSE(session.request(0x40, {0x22, 0xF1, 0x90}, 100, answers, error));
    REQUIRE_FALSE(error.empty());

    REQUIRE_FALSE(session.request(0x55, {0x22, 0xF1, 0x90}, 5000, answers, error));
    REQUIRE(error.find("0x41") != std::string::npos);

    car->dead = true;
    REQUIRE_FALSE(session.request(0x12, {0x3E, 0x00}, 100, answers, error));
    REQUIRE_FALSE(session.request(0x12, {0x3E, 0x00}, 100, answers, error));
}

TEST_CASE("ENET front end: Elm327, OBD and BMW units over a scripted car", "[enet]") {
    setLanguage(Language::English);
    for (bool headers : {false, true}) {
        INFO("headers " << headers);
        auto link = scriptedBmw();
        FakeBmw* car = link.get();
        Elm327 elm;
        std::string error;
        REQUIRE(elm.connect(std::make_unique<EnetElm>(std::move(link), "ENET test"), "6", error));
        REQUIRE(elm.isCan());
        REQUIRE(elm.protocolText() == "ISO 15765-4 (CAN 11/500)");
        if (headers) REQUIRE(elm.setHeaders(true));

        Obd2 obd(elm);
        const std::vector<Fault> faults = obd.readFaults(error);
        REQUIRE(faults.size() == 1);
        REQUIRE(faults[0].code == "P0171");
        REQUIRE(obd.readVin() == kVin);

        // Every BMW unit of the list, through the raw frames of the front end.
        const std::vector<EcuDefinition> list = ecuDefinitions("BMW");
        const std::vector<UdsEcu> ecus = scanEcus(elm, list, nullptr, nullptr);
        int present = 0;
        for (const UdsEcu& e : ecus) {
            if (!e.present) continue;
            ++present;
            if (e.definition.extendedAddress == 0x12) {
                REQUIRE(e.info.vin == kVin);
                REQUIRE(e.dtcs.size() == 1);
                REQUIRE(e.dtcs[0].code == "480A10");
            }
            if (e.definition.extendedAddress == 0x29) REQUIRE(e.dtcs.empty());
        }
        REQUIRE(present == 2);

        // Normal OBD works again after the scan.
        REQUIRE(obd.readVin() == kVin);
        // The engine unit stands in for the adapter's voltage pin.
        const ElmResult volts = elm.send("ATRV", 2000);
        REQUIRE_FALSE(volts.lines.empty());
        REQUIRE(volts.lines.front() == "12.7V");
        REQUIRE_FALSE(car->asked.empty());
    }
}
