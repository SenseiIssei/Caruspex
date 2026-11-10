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

// ---------------------------------------------------------------------------
// DoIP (ISO 13400-2)

namespace {

class FakeDoipVehicle : public ByteStream {
public:
    int routingCode = 0x10;
    std::vector<doip::Message> sent;
    std::function<std::vector<doip::Message>(int target, const ByteVector& uds)> respond;

    bool send(const ByteVector& data) override {
        parser_.feed(data.data(), data.size());
        doip::Message m;
        while (parser_.next(m)) {
            sent.push_back(m);
            std::vector<doip::Message> out;
            if (m.type == doip::RoutingActivationRequest) {
                out.push_back({doip::RoutingActivationResponse,
                               {m.payload[0], m.payload[1], 0x10, 0x10, static_cast<std::uint8_t>(routingCode), 0, 0, 0, 0}});
                out.push_back({doip::AliveCheckRequest, {}});
            } else if (m.type == doip::DiagnosticMessage && respond) {
                const int target = (m.payload[2] << 8) | m.payload[3];
                out.push_back({doip::DiagnosticAck, {m.payload[2], m.payload[3], m.payload[0], m.payload[1], 0x00}});
                for (const doip::Message& r : respond(target, ByteVector(m.payload.begin() + 4, m.payload.end())))
                    out.push_back(r);
            }
            for (const doip::Message& o : out) {
                const ByteVector bytes = doip::encode(o.type, o.payload);
                inbound_.insert(inbound_.end(), bytes.begin(), bytes.end());
            }
        }
        return true;
    }
    int receive(std::uint8_t* buffer, int size, int) override {
        const int n = std::min<int>(size, std::min<int>(7, static_cast<int>(inbound_.size())));
        for (int i = 0; i < n; ++i) {
            buffer[i] = inbound_.front();
            inbound_.pop_front();
        }
        return n;
    }

private:
    doip::Parser parser_;
    std::deque<std::uint8_t> inbound_;
};

doip::Message diag(int source, const ByteVector& uds) {
    ByteVector p = {static_cast<std::uint8_t>(source >> 8), static_cast<std::uint8_t>(source),
                    static_cast<std::uint8_t>(doip::kTester >> 8), static_cast<std::uint8_t>(doip::kTester & 0xFF)};
    p.insert(p.end(), uds.begin(), uds.end());
    return {doip::DiagnosticMessage, p};
}

// A newer car behind DoIP: engine 0x1001, gearbox 0x1002; the engine only
// speaks OBD on UDS when classic is false.
class FakeDoipCar : public DiagLink {
public:
    bool classic = true;
    bool request(int target, const ByteVector& data, int, std::vector<DiagAnswer>& answers, std::string& error) override {
        answers.clear();
        const bool functional = target == doip::kObdFunctional;
        auto answer = [&](int source, const ByteVector& d) {
            if (functional || target == source) answers.push_back({source, d});
        };
        if (classic) {
            if (data == ByteVector{0x01, 0x00}) {
                answer(0x1001, {0x41, 0x00, 0xBE, 0x3F, 0xA8, 0x13});
                answer(0x1002, {0x41, 0x00, 0x80, 0x00, 0x00, 0x01});
            }
            if (data == ByteVector{0x03}) answer(0x1001, {0x43, 0x01, 0x04, 0x20});
            if (data == ByteVector{0x09, 0x02}) answer(0x1001, vinAnswer({0x49, 0x02, 0x01}));
            if (data == ByteVector{0x01, 0x42}) answer(0x1001, {0x41, 0x42, 0x31, 0x9C});
        } else {
            if (data[0] == 0x01) answer(0x1001, {0x7F, 0x01, 0x11});
            if (data == ByteVector{0x22, 0xF4, 0x00}) answer(0x1001, {0x62, 0xF4, 0x00, 0xBE, 0x3F, 0xA8, 0x13});
            if (data == ByteVector{0x22, 0xF8, 0x02}) answer(0x1001, vinAnswer({0x62, 0xF8, 0x02}));
        }
        if (answers.empty()) error = "no answer";
        return !answers.empty();
    }
};

}  // namespace

TEST_CASE("DoIP frames and the vehicle announcement", "[doip]") {
    const ByteVector f = doip::encode(doip::DiagnosticMessage, {0x0E, 0x80, 0x10, 0x01, 0x3E, 0x00});
    REQUIRE(f == ByteVector{0x02, 0xFD, 0x80, 0x01, 0, 0, 0, 6, 0x0E, 0x80, 0x10, 0x01, 0x3E, 0x00});
    REQUIRE(doip::encode(doip::VehicleIdentRequest, {}, 0xFF) == ByteVector{0xFF, 0x00, 0x00, 0x01, 0, 0, 0, 0});

    doip::Parser parser;
    doip::Message m;
    parser.feed(f.data(), 5);
    REQUIRE_FALSE(parser.next(m));
    parser.feed(f.data() + 5, f.size() - 5);
    REQUIRE(parser.next(m));
    REQUIRE(m.type == doip::DiagnosticMessage);
    REQUIRE(m.payload.size() == 6);

    doip::Parser bad;
    const ByteVector junk = {0x02, 0x02, 0x80, 0x01, 0, 0, 0, 0};
    bad.feed(junk.data(), junk.size());
    REQUIRE_FALSE(bad.next(m));
    REQUIRE(bad.broken());

    ByteVector announcement = bytesOf(kVin);
    for (std::uint8_t b : {0x10, 0x10, 1, 2, 3, 4, 5, 6, 1, 2, 3, 4, 5, 6, 0x00}) announcement.push_back(b);
    doip::Entity e;
    REQUIRE(doip::parseAnnouncement(announcement, e));
    REQUIRE(e.vin == kVin);
    REQUIRE(e.logicalAddress == 0x1010);
    REQUIRE_FALSE(doip::parseAnnouncement(ByteVector(10, 0), e));
}

TEST_CASE("DoIP session: routing, acknowledgement, alive check, functional and refusals", "[doip]") {
    auto stream = std::make_unique<FakeDoipVehicle>();
    FakeDoipVehicle* car = stream.get();
    car->respond = [](int target, const ByteVector& uds) -> std::vector<doip::Message> {
        if (target == 0x1001 && uds == ByteVector{0x22, 0xF1, 0x90})
            return {diag(0x1001, {0x7F, 0x22, 0x78}), diag(0x1001, vinAnswer({0x62, 0xF1, 0x90}))};
        if (target == doip::kObdFunctional && uds == ByteVector{0x01, 0x00})
            return {diag(0x1001, {0x41, 0x00, 0xBE}), diag(0x1002, {0x41, 0x00, 0x80})};
        if (target == 0x2000) return {{doip::DiagnosticNack, {0x20, 0x00, 0x0E, 0x80, 0x03}}};
        return {};
    };
    DoipSession session(std::move(stream));
    std::string error;
    REQUIRE(session.activate(error));
    REQUIRE(car->sent.front().type == doip::RoutingActivationRequest);
    REQUIRE(car->sent.front().payload[0] == 0x0E);

    std::vector<DiagAnswer> answers;
    REQUIRE(session.request(0x1001, {0x22, 0xF1, 0x90}, 500, answers, error));
    REQUIRE(answers.size() == 1);
    REQUIRE(answers[0].data[0] == 0x62);
    // The alive check after the routing answer was answered with our address.
    bool alive = false;
    for (const doip::Message& m : car->sent)
        if (m.type == doip::AliveCheckResponse && m.payload == ByteVector{0x0E, 0x80}) alive = true;
    REQUIRE(alive);

    REQUIRE(session.request(doip::kObdFunctional, {0x01, 0x00}, 500, answers, error));
    REQUIRE(answers.size() == 2);

    REQUIRE_FALSE(session.request(0x2000, {0x3E, 0x00}, 500, answers, error));
    REQUIRE(error.find("0x03") != std::string::npos);
    REQUIRE_FALSE(session.request(0x3000, {0x3E, 0x00}, 100, answers, error));

    // Routing denied (0x06: unsupported activation type).
    auto denying = std::make_unique<FakeDoipVehicle>();
    denying->routingCode = 0x06;
    DoipSession refused(std::move(denying));
    REQUIRE_FALSE(refused.activate(error));
    REQUIRE(error.find("0x06") != std::string::npos);
}

TEST_CASE("DoIP front end: OBD through the functional address, also on UDS", "[doip]") {
    setLanguage(Language::English);
    for (bool classic : {true, false}) {
        INFO("classic " << classic);
        auto car = std::make_unique<FakeDoipCar>();
        car->classic = classic;
        Elm327 elm;
        std::string error;
        REQUIRE(elm.connect(std::make_unique<EnetElm>(std::move(car), "DoIP test", EnetElm::Flavor::Doip), "6", error));
        REQUIRE(elm.obdOnUds() == !classic);
        REQUIRE(elm.setHeaders(true));
        Obd2 obd(elm);
        REQUIRE(obd.readVin() == kVin);
        if (classic) {
            const std::vector<Fault> faults = obd.readFaults(error);
            REQUIRE(faults.size() == 1);
            REQUIRE(faults[0].code == "P0420");
            REQUIRE(faults[0].ecuSource == "7E8");
            const ElmResult volts = elm.send("ATRV", 2000);
            REQUIRE(volts.lines.front() == "12.7V");
        }
    }
}
