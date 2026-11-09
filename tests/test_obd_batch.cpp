// Several PIDs per request on CAN (readLiveBatch) and freeze frames per fault
// (readFreezeFrames). The recorded byte examples are worked out by hand from
// SAE J1979 and ISO 15765-2; the demo car checks the same end to end.

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include <algorithm>
#include <cctype>
#include <chrono>
#include <cstdio>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <vector>

#include "core/DemoTransport.h"
#include "core/Elm327.h"
#include "core/Obd2.h"
#include "core/Transport.h"
#include "DemoRig.h"

using namespace caruspex;
using Catch::Approx;

namespace {

// A scripted ELM327 that also counts what was sent. Every OBD request has
// an answer for headers off and one for headers on.
class RecordingTransport : public ITransport {
public:
    struct Reply {
        std::vector<std::string> plain;
        std::vector<std::string> headers;
    };
    std::map<std::string, Reply> replies;
    std::vector<std::string> obdSent;   // every non-AT command, in order

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
        } else if (cmd == "0100") {
            pending_ = "41 00 BE 1F A8 13\r\r>";
        } else {
            obdSent.push_back(cmd);
            const auto it = replies.find(cmd);
            if (it == replies.end()) {
                pending_ = "NO DATA\r\r>";
            } else {
                pending_.clear();
                for (const std::string& line : headers_ ? it->second.headers : it->second.plain)
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
    std::string name() const override { return "Recording"; }

private:
    std::string pending_;
    bool headers_ = false;
};

// The single answers the fallback needs.
void addSingles(RecordingTransport& t) {
    t.replies["010C"] = {{"410C1AF8"}, {"7E8 04 41 0C 1A F8 AA AA AA"}};
    t.replies["010D"] = {{"410D32", "410D33"}, {"7E8 03 41 0D 32 AA AA AA AA", "7E9 03 41 0D 33 AA AA AA AA"}};
    t.replies["0105"] = {{"41057B"}, {"7E8 03 41 05 7B AA AA AA AA"}};
    t.replies["0111"] = {{"411140"}, {"7E8 03 41 11 40 AA AA AA AA"}};
    t.replies["010F"] = {{"410F48"}, {"7E8 03 41 0F 48 AA AA AA AA"}};
    t.replies["0104"] = {{"410480"}, {"7E8 03 41 04 80 AA AA AA AA"}};
}

// Six PIDs: 41 0C 1A F8 0D 32 05 7B 11 40 0F 48 04 80 is fourteen bytes, too
// many for one CAN frame. First frame with six bytes, then two consecutive
// frames. The transmission answers the speed in a single frame of its own.
RecordingTransport* sixPidCar(std::unique_ptr<RecordingTransport>& owner) {
    owner = std::make_unique<RecordingTransport>();
    RecordingTransport& t = *owner;
    t.replies["010C0D05110F04"] = {
        {"00E", "0:410C1AF80D32", "1:057B11400F4804", "2:80000000000000", "410D33"},
        {"7E8 10 0E 41 0C 1A F8 0D 32", "7E9 03 41 0D 33 AA AA AA AA",
         "7E8 21 05 7B 11 40 0F 48 04", "7E8 22 80 AA AA AA AA AA AA"}};
    addSingles(t);
    return &t;
}

void checkSixValues(const std::vector<LiveValue>& values) {
    std::map<int, double> byKey;
    for (const LiveValue& v : values) {
        if (!byKey.count(v.key)) byKey[v.key] = v.value;
    }
    REQUIRE(byKey.at(0x0C) == Approx(1726.0));
    REQUIRE(byKey.at(0x0D) == Approx(50.0));
    REQUIRE(byKey.at(0x05) == Approx(83.0));
    REQUIRE(byKey.at(0x11) == Approx(64.0 * 100.0 / 255.0));
    REQUIRE(byKey.at(0x0F) == Approx(32.0));
    REQUIRE(byKey.at(0x04) == Approx(128.0 * 100.0 / 255.0));
}

int countSent(const std::vector<std::string>& log, const std::string& prefix) {
    return static_cast<int>(std::count_if(log.begin(), log.end(), [&](const std::string& line) {
        return line.rfind("  > " + prefix, 0) == 0;
    }));
}

}  // namespace

// ---------------------------------------------------------------------------
// Splitting a combined answer
// ---------------------------------------------------------------------------

TEST_CASE("A combined answer splits by the PID table", "[batch]") {
    std::vector<std::pair<int, std::vector<int>>> blocks;

    REQUIRE(splitMultiPidAnswer({0x0C, 0x1A, 0xF8, 0x0D, 0x32, 0x05, 0x7B}, {0x0C, 0x0D, 0x05}, blocks));
    REQUIRE(blocks.size() == 3);
    REQUIRE(blocks[0].first == 0x0C);
    REQUIRE(blocks[0].second == std::vector<int>{0x1A, 0xF8});
    REQUIRE(blocks[1].second == std::vector<int>{0x32});
    REQUIRE(blocks[2].second == std::vector<int>{0x7B});

    // The control unit may leave out what it does not support, and it may
    // answer in a different order.
    REQUIRE(splitMultiPidAnswer({0x05, 0x7B, 0x0C, 0x1A, 0xF8}, {0x0C, 0x0D, 0x05}, blocks));
    REQUIRE(blocks.size() == 2);
    REQUIRE(blocks[0].first == 0x05);

    // Padding after the last block is fine.
    REQUIRE(splitMultiPidAnswer({0x0D, 0x32, 0xAA, 0xAA, 0x00}, {0x0D}, blocks));
    REQUIRE(blocks.size() == 1);

    // A multi-value PID takes all its bytes: 0x67 is three.
    REQUIRE(splitMultiPidAnswer({0x67, 0x03, 0x5A, 0x64, 0x0D, 0x32}, {0x67, 0x0D}, blocks));
    REQUIRE(blocks[0].second.size() == 3);
    REQUIRE(blocks[1].first == 0x0D);

    // What does not add up is refused.
    REQUIRE_FALSE(splitMultiPidAnswer({0x0C, 0x1A}, {0x0C}, blocks));              // short
    REQUIRE_FALSE(splitMultiPidAnswer({0x0C, 0x1A, 0xF8, 0x0E, 0x80}, {0x0C}, blocks));   // not asked
    REQUIRE_FALSE(splitMultiPidAnswer({0x0D, 0x32, 0x0D, 0x33}, {0x0D}, blocks));  // twice
    REQUIRE_FALSE(splitMultiPidAnswer({}, {0x0D}, blocks));
    REQUIRE_FALSE(splitMultiPidAnswer({0xAA}, {0x0D}, blocks));                    // padding only

    REQUIRE(batchablePid(0x0C));
    REQUIRE(batchablePid(0x67));
    REQUIRE_FALSE(batchablePid(0x68));   // three or seven bytes, depending on the car
    REQUIRE_FALSE(batchablePid(0x00));
    REQUIRE_FALSE(batchablePid(0x20));
    REQUIRE_FALSE(batchablePid(0x300));
}

// ---------------------------------------------------------------------------
// Recorded answers
// ---------------------------------------------------------------------------

TEST_CASE("Six PIDs in one request, headers off, answer over three frames", "[batch]") {
    std::unique_ptr<RecordingTransport> owner;
    RecordingTransport* t = sixPidCar(owner);
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::move(owner), "6", error));
    Obd2 obd(elm);

    t->obdSent.clear();
    const std::vector<LiveValue> values = obd.readLiveBatch({0x0C, 0x0D, 0x05, 0x11, 0x0F, 0x04});
    REQUIRE(t->obdSent == std::vector<std::string>{"010C0D05110F04"});
    REQUIRE(values.size() == 6);
    checkSixValues(values);
    // In the order asked, the engine first (it answered first).
    REQUIRE(values[0].pid == 0x0C);
    REQUIRE(values[5].pid == 0x04);
    REQUIRE(values[1].value == Approx(50.0));
    REQUIRE(values[1].ecu.empty());

    // Every control unit: the transmission's speed comes along.
    const std::vector<LiveValue> every = obd.readLiveBatch({0x0C, 0x0D, 0x05, 0x11, 0x0F, 0x04}, true);
    REQUIRE(every.size() == 7);
    REQUIRE(every[1].pid == 0x0D);
    REQUIRE(every[2].pid == 0x0D);
    REQUIRE(every[2].value == Approx(51.0));
    REQUIRE(obd.batchRefusedPids().empty());

    // readLive(vector) rides on the same request.
    t->obdSent.clear();
    const std::vector<LiveValue> primary = obd.readLive({0x0C, 0x0D, 0x05, 0x11, 0x0F, 0x04});
    REQUIRE(t->obdSent.size() == 1);
    REQUIRE(primary.size() == 6);
    checkSixValues(primary);
}

TEST_CASE("Six PIDs in one request, headers on, frames of two control units interleaved",
          "[batch]") {
    std::unique_ptr<RecordingTransport> owner;
    RecordingTransport* t = sixPidCar(owner);
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::move(owner), "6", error));
    REQUIRE(elm.setHeaders(true));
    Obd2 obd(elm);

    t->obdSent.clear();
    std::vector<LiveValue> values = obd.readLiveBatch({0x0C, 0x0D, 0x05, 0x11, 0x0F, 0x04});
    REQUIRE(t->obdSent.size() == 1);
    // The single frame of 7E9 is complete before the multi-frame answer of
    // 7E8 is. As in readLiveAll the first complete answer keeps the PID, so
    // the speed comes from the transmission here.
    REQUIRE(values.size() == 6);
    const auto speed = std::find_if(values.begin(), values.end(),
                                    [](const LiveValue& v) { return v.pid == 0x0D; });
    REQUIRE(speed != values.end());
    REQUIRE(speed->ecuSource == "7E9");
    REQUIRE(speed->value == Approx(51.0));
    const auto rpm = std::find_if(values.begin(), values.end(),
                                  [](const LiveValue& v) { return v.pid == 0x0C; });
    REQUIRE(rpm->ecuSource == "7E8");
    REQUIRE(rpm->value == Approx(1726.0));

    values = obd.readLiveBatch({0x0C, 0x0D, 0x05, 0x11, 0x0F, 0x04}, true);
    REQUIRE(values.size() == 7);
    int fromEngine = 0, fromGearbox = 0;
    for (const LiveValue& v : values) {
        if (v.ecuSource == "7E8") ++fromEngine;
        if (v.ecuSource == "7E9") ++fromGearbox;
    }
    REQUIRE(fromEngine == 6);
    REQUIRE(fromGearbox == 1);
}

TEST_CASE("A refused or empty group falls back to single PIDs and is remembered", "[batch]") {
    auto owner = std::make_unique<RecordingTransport>();
    RecordingTransport* t = owner.get();
    addSingles(*t);
    // NO DATA for the group (no reply scripted), a refusal for the second.
    t->replies["01110F04"] = {{"7F0112"}, {"7E8 03 7F 01 12 AA AA AA AA"}};
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::move(owner), "6", error));
    Obd2 obd(elm);

    t->obdSent.clear();
    std::vector<LiveValue> values = obd.readLiveBatch({0x0C, 0x0D, 0x05});
    REQUIRE(t->obdSent == std::vector<std::string>{"010C0D05", "010C", "010D", "0105"});
    REQUIRE(values.size() == 3);
    REQUIRE(obd.batchRefusedPids().size() == 3);

    // Next time no group is tried for them.
    t->obdSent.clear();
    values = obd.readLiveBatch({0x0C, 0x0D, 0x05});
    REQUIRE(t->obdSent == std::vector<std::string>{"010C", "010D", "0105"});
    REQUIRE(values.size() == 3);

    t->obdSent.clear();
    values = obd.readLiveBatch({0x11, 0x0F, 0x04});
    REQUIRE(t->obdSent == std::vector<std::string>{"01110F04", "0111", "010F", "0104"});
    REQUIRE(values.size() == 3);
    REQUIRE(obd.batchRefusedPids().size() == 6);

    obd.resetBatchMemory();
    t->obdSent.clear();
    obd.readLiveBatch({0x11, 0x0F});
    REQUIRE(t->obdSent.front() == "01110F");
}

TEST_CASE("A control unit that answers only the first PID of a group", "[batch]") {
    auto owner = std::make_unique<RecordingTransport>();
    RecordingTransport* t = owner.get();
    addSingles(*t);
    t->replies["010C0D05"] = {{"410C1AF8"}, {"7E8 04 41 0C 1A F8 AA AA AA"}};
    t->replies["010C"] = {{"410C1B00"}, {"7E8 04 41 0C 1B 00 AA AA AA"}};
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::move(owner), "6", error));
    Obd2 obd(elm);

    t->obdSent.clear();
    std::vector<LiveValue> values = obd.readLiveBatch({0x0C, 0x0D, 0x05});
    REQUIRE(t->obdSent == std::vector<std::string>{"010C0D05", "010D", "0105"});
    REQUIRE(values.size() == 3);
    REQUIRE(values[0].value == Approx(1726.0));
    REQUIRE(obd.batchRefusedPids() == std::set<int>{0x0D, 0x05});

    // 0C is still fine in groups; alone in its group it is asked singly.
    t->obdSent.clear();
    values = obd.readLiveBatch({0x0C, 0x0D, 0x05});
    REQUIRE(t->obdSent == std::vector<std::string>{"010C", "010D", "0105"});
    REQUIRE(values[0].value == Approx(1728.0));
}

TEST_CASE("An answer that does not add up spoils the whole group", "[batch]") {
    auto owner = std::make_unique<RecordingTransport>();
    RecordingTransport* t = owner.get();
    addSingles(*t);
    // The speed is missing its data byte, so 05 would be read as the speed.
    t->replies["010C0D05"] = {{"410C1AF80D057B"}, {"7E8 06 41 0C 1A F8 0D 05 7B AA"}};
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::move(owner), "6", error));
    Obd2 obd(elm);

    t->obdSent.clear();
    const std::vector<LiveValue> values = obd.readLiveBatch({0x0C, 0x0D, 0x05});
    REQUIRE(t->obdSent == std::vector<std::string>{"010C0D05", "010C", "010D", "0105"});
    REQUIRE(values.size() == 3);
    REQUIRE(values[1].value == Approx(50.0));
    REQUIRE(values[2].value == Approx(83.0));
}

TEST_CASE("Without CAN every PID is asked on its own", "[batch]") {
    auto owner = std::make_unique<RecordingTransport>();
    RecordingTransport* t = owner.get();
    addSingles(*t);
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::move(owner), "3", error));   // ISO 9141-2
    REQUIRE_FALSE(elm.isCan());
    Obd2 obd(elm);

    t->obdSent.clear();
    const std::vector<LiveValue> values = obd.readLiveBatch({0x0C, 0x0D, 0x05});
    REQUIRE(t->obdSent == std::vector<std::string>{"010C", "010D", "0105"});
    REQUIRE(values.size() == 3);
    REQUIRE(obd.batchRefusedPids().empty());
}

// ---------------------------------------------------------------------------
// The demo car
// ---------------------------------------------------------------------------

TEST_CASE("Batch reads on the demo car match single reads", "[batch][demo]") {
    for (const bool headers : {false, true}) {
        DYNAMIC_SECTION("headers " << (headers ? "on" : "off")) {
            DemoRig rig(50.0 + 3 * 120.0 + 70.0);   // cruising, the clock stands still
            REQUIRE(rig.connected);
            if (headers) REQUIRE(rig.elm.setHeaders(true));
            Obd2 obd(rig.elm);

            // 0x01 has sub-values and is answered by both control units,
            // 0x68 is never grouped, 0x0A is not supported by the demo.
            const std::vector<int> pids = {0x01, 0x0C, 0x0D, 0x05, 0x04, 0x11, 0x0F, 0x10,
                                           0x0B, 0x06, 0x07, 0x42, 0x5C, 0xA4, 0x68, 0x0A};
            std::vector<LiveValue> single;
            for (int pid : pids) {
                for (const LiveValue& v : obd.readLiveAll(pid)) single.push_back(v);
            }

            rig.sent.clear();
            const std::vector<LiveValue> batch = obd.readLiveBatch(pids);
            REQUIRE(batch.size() == single.size());
            for (size_t i = 0; i < batch.size(); ++i) {
                INFO("key " << batch[i].key);
                REQUIRE(batch[i].key == single[i].key);
                REQUIRE(batch[i].value == Approx(single[i].value));
                REQUIRE(batch[i].ecuSource == single[i].ecuSource);
                REQUIRE(batch[i].ecu == single[i].ecu);
            }
            REQUIRE(std::any_of(batch.begin(), batch.end(), [](const LiveValue& v) { return v.sub > 0; }));
            // Groups of 6 + 6 + 3, then 0x68 alone and 0x0A, which the group
            // answer left out, once more on its own. The second time 0x0A is
            // no longer in a group: 6 + 6 + 2 and the same two singles.
            REQUIRE(countSent(rig.sent, "01") == 5);
            REQUIRE(obd.batchRefusedPids() == std::set<int>{0x0A});

            rig.sent.clear();
            obd.readLiveBatch(pids);
            REQUIRE(countSent(rig.sent, "01") == 5);

            if (headers) {
                // PID 01 from both control units.
                const std::vector<LiveValue> every = obd.readLiveBatch({0x01, 0xA4, 0x0C}, true);
                REQUIRE(std::any_of(every.begin(), every.end(), [](const LiveValue& v) {
                    return v.pid == 0x01 && v.ecuSource == "7E9";
                }));
                REQUIRE(std::any_of(every.begin(), every.end(), [](const LiveValue& v) {
                    return v.pid == 0x01 && v.ecuSource == "7E8";
                }));
            }
        }
    }
}

// Not run by default: prints how many values per second the demo car gives
// with its simulated answer times, one PID per request against six.
TEST_CASE("Demo speed of single and batch reads", "[.][bench]") {
    DemoRig rig(500.0, DemoAdapter::CloneV15, true, 1.0);
    REQUIRE(rig.connected);
    rig.elm.setVerbose(false);
    Obd2 obd(rig.elm);
    const std::vector<int>& pids = defaultLivePids();
    using Clock = std::chrono::steady_clock;

    auto rate = [&](bool batch) {
        int values = 0;
        const auto start = Clock::now();
        for (int round = 0; round < 10; ++round) {
            if (batch) {
                values += static_cast<int>(obd.readLiveBatch(pids).size());
            } else {
                for (int pid : pids) values += static_cast<int>(obd.readLiveAll(pid).size());
            }
        }
        const double seconds = std::chrono::duration<double>(Clock::now() - start).count();
        return values / seconds;
    };
    const double single = rate(false);
    const double batch = rate(true);
    std::printf("%zu PIDs: single %.0f values/s, batch %.0f values/s, %.1f times as fast\n",
                pids.size(), single, batch, batch / single);
    REQUIRE(batch > single);
}

// ---------------------------------------------------------------------------
// Freeze frames
// ---------------------------------------------------------------------------

TEST_CASE("Freeze frames per fault on the demo car", "[freeze][demo]") {
    DemoRig rig(1400.0);
    REQUIRE(rig.connected);
    Obd2 obd(rig.elm);

    // Before clearing: the stored lean mixture in frame 0, the pending
    // misfire in frame 1.
    std::vector<FreezeFrame> frames = obd.readFreezeFrames();
    REQUIRE(frames.size() == 2);
    REQUIRE(frames[0].frame == 0);
    REQUIRE(frames[0].dtc == "P0171");
    REQUIRE(frames[1].frame == 1);
    REQUIRE(frames[1].dtc == "P0300");
    REQUIRE(frames[0].ecu.empty());
    for (const FreezeFrame& f : frames) {
        // The demo lists twelve PIDs besides 02; 03 has two values.
        REQUIRE(f.values.size() >= 12);
        REQUIRE(std::none_of(f.values.begin(), f.values.end(),
                             [](const LiveValue& v) { return v.pid == 0x02; }));
    }
    auto value = [](const FreezeFrame& f, int key) {
        for (const LiveValue& v : f.values) {
            if (v.key == key) return v.value;
        }
        FAIL("key missing");
        return 0.0;
    };
    REQUIRE(value(frames[0], 0x06) == Approx(7.0).margin(1.0));   // short term trim, lean
    REQUIRE(value(frames[0], 0x0C) > 500.0);
    REQUIRE(value(frames[1], 0x0C) > 500.0);

    // The old call still gives frame 0.
    std::string trigger;
    REQUIRE_FALSE(obd.readFreezeFrame(trigger).empty());
    REQUIRE(trigger == "P0171");

    // Only as many as asked for.
    REQUIRE(obd.readFreezeFrames(1).size() == 1);

    // With headers every frame knows its control unit.
    REQUIRE(rig.elm.setHeaders(true));
    frames = obd.readFreezeFrames();
    REQUIRE(frames.size() == 2);
    REQUIRE(frames[1].ecuSource == "7E8");
    REQUIRE_FALSE(frames[1].ecu.empty());
    REQUIRE(frames[1].values.size() >= 12);
    REQUIRE(frames[1].values.front().ecuSource == "7E8");
    REQUIRE(rig.elm.setHeaders(false));

    // Clearing empties every frame.
    std::string error;
    REQUIRE(obd.clearFaults(error));
    REQUIRE(obd.readFreezeFrames().empty());

    // The misfire comes back first, then the lean mixture.
    rig.now += 200.0;
    frames = obd.readFreezeFrames();
    REQUIRE(frames.size() == 1);
    REQUIRE(frames[0].dtc == "P0300");
    rig.now += 100.0;
    frames = obd.readFreezeFrames();
    REQUIRE(frames.size() == 2);
    REQUIRE(frames[0].dtc == "P0300");
    REQUIRE(frames[1].dtc == "P0171");
}

TEST_CASE("Freeze frames from recorded answers", "[freeze]") {
    auto owner = std::make_unique<RecordingTransport>();
    RecordingTransport* t = owner.get();
    // Frame 0: P0301 with rpm and speed; frame 1: P0420; frame 2 empty.
    t->replies["020200"] = {{"42020003 01"}, {"7E8 05 42 02 00 03 01 AA AA"}};
    t->replies["020201"] = {{"42020104 20"}, {"7E8 05 42 02 01 04 20 AA AA"}};
    t->replies["020202"] = {{"4202020000"}, {"7E8 05 42 02 02 00 00 AA AA"}};
    // Supported PIDs per frame: 02, 0C and 0D. PID n is bit 32 - n of the
    // mask: 02 -> 0x40000000, 0C -> 0x00100000, 0D -> 0x00080000.
    t->replies["020000"] = {{"42000040180000"}, {"7E8 07 42 00 00 40 18 00 00"}};
    t->replies["020001"] = {{"42000140180000"}, {"7E8 07 42 00 01 40 18 00 00"}};
    t->replies["020C00"] = {{"420C001AF8"}, {"7E8 05 42 0C 00 1A F8 AA AA"}};
    t->replies["020D00"] = {{"420D0032"}, {"7E8 04 42 0D 00 32 AA AA AA"}};
    t->replies["020C01"] = {{"420C010FA0"}, {"7E8 05 42 0C 01 0F A0 AA AA"}};
    // A control unit that ignores the frame number would answer frame 0.
    t->replies["020D01"] = {{"420D0032"}, {"7E8 04 42 0D 00 32 AA AA AA"}};
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::move(owner), "6", error));
    Obd2 obd(elm);

    for (const bool headers : {false, true}) {
        REQUIRE(elm.setHeaders(headers));
        const std::vector<FreezeFrame> frames = obd.readFreezeFrames(8);
        REQUIRE(frames.size() == 2);
        REQUIRE(frames[0].dtc == "P0301");
        REQUIRE(frames[0].values.size() == 2);
        REQUIRE(frames[0].values[0].key == 0x0C);
        REQUIRE(frames[0].values[0].value == Approx(1726.0));
        REQUIRE(frames[0].values[1].value == Approx(50.0));
        REQUIRE(frames[1].dtc == "P0420");
        REQUIRE(frames[1].values.size() == 1);   // the frame 0 speed is not taken
        REQUIRE(frames[1].values[0].value == Approx(1000.0));
        REQUIRE(frames[1].ecuSource == (headers ? "7E8" : ""));
    }
    // Frame 3 was never asked: the empty frame 2 ended the search.
    REQUIRE(std::find(t->obdSent.begin(), t->obdSent.end(), "020203") == t->obdSent.end());
}
