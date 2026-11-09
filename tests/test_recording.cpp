// Recording and replay of an adapter session, adaptive timing and the STN
// detection during connect.

#include <catch2/catch_test_macros.hpp>

#include <deque>
#include <filesystem>
#include <functional>
#include <memory>
#include <string>
#include <vector>

#include "core/DemoTransport.h"
#include "core/Elm327.h"
#include "core/Lang.h"
#include "core/Obd2.h"
#include "core/Recording.h"
#include "data/Pids.h"

using namespace caruspex;

namespace {

std::string tempFile(const std::string& name) {
    const std::filesystem::path p = std::filesystem::temp_directory_path() / name;
    const std::u8string u = p.u8string();
    return std::string(u.begin(), u.end());
}

DemoOptions warmDemo(double* clock, DemoAdapter adapter = DemoAdapter::CloneV15) {
    DemoOptions options;
    options.adapter = adapter;
    options.clock = [clock] { return *clock; };
    options.latencyScale = 0.0;
    return options;
}

// What a session reads, in a form that can be compared.
struct Readout {
    std::vector<std::string> faults;
    std::string vin;
    std::vector<std::string> live;
    std::string protocol;
};

Readout readEverything(Elm327& elm) {
    Readout r;
    r.protocol = elm.protocolText();
    Obd2 obd(elm);
    std::string error;
    for (const Fault& f : obd.readFaults(error)) r.faults.push_back(f.code + "|" + f.kind + "|" + f.ecu);
    r.vin = obd.readVin();
    for (const LiveValue& v : obd.readLive(defaultLivePids()))
        r.live.push_back(std::to_string(v.key) + "=" + std::to_string(v.value) + v.text);
    return r;
}

// Wraps a transport and lets a test answer chosen commands itself.
class InterceptTransport : public ITransport {
public:
    using Rule = std::function<bool(const std::string& command, std::string& answer)>;
    InterceptTransport(std::unique_ptr<ITransport> inner, Rule rule)
        : inner_(std::move(inner)), rule_(std::move(rule)) {}

    bool write(const std::string& data) override {
        std::string answer;
        if (rule_(normalizeCommand(data), answer)) {
            queued_ += answer;
            return true;
        }
        return inner_->write(data);
    }
    std::string read(int timeoutMs) override {
        if (!queued_.empty()) {
            std::string out;
            out.swap(queued_);
            return out;
        }
        return inner_->read(timeoutMs);
    }
    void flushInput() override {
        queued_.clear();
        inner_->flushInput();
    }
    std::string name() const override { return inner_->name(); }

private:
    std::unique_ptr<ITransport> inner_;
    Rule rule_;
    std::string queued_;
};

// A transport that hands out fixed answers, for the byte-exact file test.
class ScriptedTransport : public ITransport {
public:
    std::deque<std::string> answers;
    std::vector<std::string> written;
    bool write(const std::string& data) override {
        written.push_back(data);
        return true;
    }
    std::string read(int) override {
        if (answers.empty()) return {};
        std::string a = answers.front();
        answers.pop_front();
        return a;
    }
    void flushInput() override {}
    std::string name() const override { return "Scripted \xC3\xA4"; }   // "Scripted ä"
};

std::string replayAnswer(ReplayTransport& replay, const std::string& command) {
    replay.flushInput();
    replay.write(command + "\r");
    return replay.read(50);
}

Recording smallRecording(const std::string& body) {
    Recording rec;
    std::string error;
    const std::string text =
        "{\"format\":\"caruspex-recording\",\"version\":1,\"adapter\":\"ELM327 v1.4b\"}\n" + body;
    REQUIRE(parseRecording(text, rec, error));
    return rec;
}

std::vector<std::string> sentCommands(const std::vector<std::string>& log) {
    std::vector<std::string> out;
    for (const std::string& line : log) {
        const size_t a = line.find("> ");
        const size_t b = line.find("   ->");
        if (a != std::string::npos && b != std::string::npos) out.push_back(line.substr(a + 2, b - a - 2));
    }
    return out;
}

bool contains(const std::vector<std::string>& v, const std::string& s) {
    for (const std::string& x : v) {
        if (x == s) return true;
    }
    return false;
}

}  // namespace

TEST_CASE("Recording and replay give the same faults, VIN and live values", "[recording]") {
    setLanguage(Language::English);
    const std::string path = tempFile("caruspex_test_roundtrip.jsonl");
    double now = 600.0;

    Readout live;
    {
        Elm327 elm;
        std::string error;
        auto recorder = std::make_unique<RecordingTransport>(
            std::make_unique<DemoTransport>(warmDemo(&now)), path);
        REQUIRE(recorder->isRecording());
        // Protocol 5 first: the demo car refuses it, so the recording also
        // holds a failed attempt.
        REQUIRE(elm.connect(std::move(recorder), "56", error));
        // Like the application: headers on, so every fault keeps its sender.
        REQUIRE(elm.setHeaders(true));
        live = readEverything(elm);
        REQUIRE(activeRecording(elm) != nullptr);
        REQUIRE(activeRecording(elm)->exchanges() > 20);
        elm.disconnect();
    }
    REQUIRE(live.faults.size() >= 2);
    REQUIRE(live.vin == DemoTransport::vin());
    REQUIRE(live.live.size() >= 5);

    // The demo car moves on; the replay must not care.
    now = 2000.0;
    auto replay = std::make_unique<ReplayTransport>(path);
    REQUIRE(replay->isOpen());
    ReplayTransport* player = replay.get();
    REQUIRE(player->name() == "Recording: caruspex_test_roundtrip.jsonl");

    const RecordingInfo info = player->info();
    REQUIRE(info.adapter == "ELM327 v1.5");
    REQUIRE(info.protocol.find("15765-4") != std::string::npos);
    REQUIRE(info.protocolId == "6");
    REQUIRE(info.exchanges > 20);
    REQUIRE(info.skippedLines == 0);
    REQUIRE_FALSE(info.truncated);
    REQUIRE(info.transport == "Demo vehicle (built-in)");
    REQUIRE(recordingSummary(info).find("Adapter: ELM327 v1.5") != std::string::npos);

    Elm327 elm;
    std::string error;
    // The same protocol order as when recording: every command matches.
    REQUIRE(elm.connect(std::move(replay), "56", error));
    REQUIRE(elm.version() == "ELM327 v1.5");
    REQUIRE(elm.isCan());
    REQUIRE(elm.setHeaders(true));
    const Readout again = readEverything(elm);
    REQUIRE(again.faults == live.faults);
    REQUIRE(again.vin == live.vin);
    REQUIRE(again.live == live.live);
    REQUIRE(again.protocol == live.protocol);
    REQUIRE(player->unmatched() == 0);
    elm.disconnect();

    // A different order goes straight to the recorded protocol.
    ReplayTransport second(path);
    REQUIRE(second.protocolOrder(kProtocolOrderKLineFirst) == "6");
    std::filesystem::remove(std::filesystem::path(std::u8string(path.begin(), path.end())));
    setLanguage(Language::German);
}

TEST_CASE("A recording started in a running session replays on its own", "[recording]") {
    setLanguage(Language::English);
    const std::string path = tempFile("caruspex_test_midsession.jsonl");
    double now = 600.0;
    std::vector<std::string> faults;
    {
        Elm327 elm;
        std::string error;
        REQUIRE(elm.connect(std::make_unique<DemoTransport>(warmDemo(&now)), "6", error));
        REQUIRE(elm.setHeaders(true));
        REQUIRE(startRecording(elm, path, error));
        REQUIRE_FALSE(startRecording(elm, path, error));   // only one at a time
        REQUIRE(elm.headersOn());                             // restored after the preamble
        Obd2 obd(elm);
        for (const Fault& f : obd.readFaults(error)) faults.push_back(f.code + "|" + f.ecu);
        REQUIRE(stopRecording(elm));
        REQUIRE_FALSE(stopRecording(elm));
        REQUIRE(elm.transportName() == "Demo vehicle (built-in)");
        // The session goes on unrecorded.
        REQUIRE(obd.readVin() == DemoTransport::vin());
    }
    REQUIRE(faults.size() >= 2);

    Recording rec;
    std::string error;
    REQUIRE(loadRecording(path, rec, error));
    REQUIRE(rec.protocolId == "6");
    REQUIRE(rec.adapter == "ELM327 v1.5");

    // Connect walks the K-line protocols first: the replay refuses them the
    // way a CAN car would, until protocol 6 comes up.
    for (const char* order : {"6", "56"}) {
        auto replay = std::make_unique<ReplayTransport>(path);
        Elm327 elm;
        REQUIRE(elm.connect(std::move(replay), order, error));
        REQUIRE(elm.protocolId() == '6');
        REQUIRE(elm.version() == "ELM327 v1.5");
        REQUIRE(elm.setHeaders(true));
        Obd2 obd(elm);
        std::vector<std::string> again;
        for (const Fault& f : obd.readFaults(error)) again.push_back(f.code + "|" + f.ecu);
        REQUIRE(again == faults);
    }
    std::filesystem::remove(std::filesystem::path(std::u8string(path.begin(), path.end())));
    setLanguage(Language::German);
}

TEST_CASE("Recording keeps every byte and the header text", "[recording]") {
    const std::string path = tempFile("caruspex_test_bytes.jsonl");
    std::string allBytes;
    for (int i = 0; i < 256; ++i) allBytes += static_cast<char>(i);
    {
        auto scripted = std::make_unique<ScriptedTransport>();
        scripted->answers = {allBytes, "\"quoted\" \\ back\r\n>"};
        RecordingMeta meta;
        meta.adapter = "Adapter \xC3\xA4\xC3\xB6\xC3\xBC";
        RecordingTransport rec(std::move(scripted), path, meta);
        REQUIRE(rec.isRecording());
        REQUIRE(rec.name() == "Scripted \xC3\xA4");
        rec.write("01 0c\r");
        REQUIRE(rec.read(10) == allBytes);
        REQUIRE(rec.read(10).size() == 18);
        REQUIRE(rec.read(10).empty());   // empty reads are not written down
        rec.note("a note");
    }
    Recording loaded;
    std::string error;
    REQUIRE(loadRecording(path, loaded, error));
    REQUIRE(loaded.version == kRecordingVersion);
    REQUIRE(loaded.adapter == "Adapter \xC3\xA4\xC3\xB6\xC3\xBC");
    REQUIRE(loaded.transport == "Scripted \xC3\xA4");
    REQUIRE(loaded.created.size() == 20);
    REQUIRE(loaded.events.size() == 4);
    REQUIRE(loaded.events[0].direction == RecordedDirection::Tx);
    REQUIRE(loaded.events[0].data == "01 0c\r");
    REQUIRE(loaded.events[1].data == allBytes);
    REQUIRE(loaded.events[2].data == "\"quoted\" \\ back\r\n>");
    REQUIRE(loaded.events[3].direction == RecordedDirection::Note);
    std::filesystem::remove(std::filesystem::path(std::u8string(path.begin(), path.end())));
}

TEST_CASE("Malformed and cut-off lines are skipped", "[recording]") {
    const std::string text =
        "{\"format\":\"caruspex-recording\",\"version\":1,\"created\":\"2026-11-09T10:00:00Z\"}\r\n"
        "{\"t\":0.100,\"dir\":\"tx\",\"data\":\"010C\\r\"}\r\n"
        "this is not json\n"
        "{\"t\":0.110,\"dir\":\"rx\"}\n"                              // no data
        "{\"t\":0.120,\"dir\":\"sideways\",\"data\":\"x\"}\n"        // unknown direction
        "{\"t\":0.130,\"dir\":\"rx\",\"data\":[1,2]}\n"              // not a string
        "\n"
        "{\"t\":0.140,\"dir\":\"rx\",\"data\":\"410C1AF8\\r\\r>\"}\n"
        "{\"t\":1.5,\"dir\":\"rx\",\"data\":\"41 0C";               // cut off
    Recording rec;
    std::string error;
    REQUIRE(parseRecording(text, rec, error));
    REQUIRE(rec.events.size() == 2);
    REQUIRE(rec.skippedLines == 4);
    REQUIRE(rec.truncated);
    REQUIRE(rec.created == "2026-11-09T10:00:00Z");

    const RecordingInfo info = describeRecording(rec);
    REQUIRE(info.exchanges == 1);
    REQUIRE(info.skippedLines == 4);
    REQUIRE(info.truncated);
    REQUIRE(info.durationSeconds > 0.13);

    ReplayTransport replay(rec, "test");
    REQUIRE(replayAnswer(replay, "010C") == "410C1AF8\r\r>");

    // Not a recording at all.
    REQUIRE_FALSE(parseRecording("{\"t\":0.1,\"dir\":\"tx\",\"data\":\"ATZ\"}\n", rec, error));
    REQUIRE_FALSE(error.empty());
    REQUIRE_FALSE(parseRecording("", rec, error));
    REQUIRE_FALSE(parseRecording("{\"format\":\"caruspex-recording\"}\n", rec, error));   // no version
    REQUIRE_FALSE(loadRecording(tempFile("caruspex_test_missing_file.jsonl"), rec, error));
}

TEST_CASE("Commands that were never recorded get ELM327 answers", "[recording]") {
    setLanguage(Language::German);
    ReplayTransport replay(smallRecording("{\"t\":0.1,\"dir\":\"tx\",\"data\":\"0100\\r\"}\n"
                                          "{\"t\":0.2,\"dir\":\"rx\",\"data\":\"4100BE3FA813\\r\\r>\"}\n"),
                           "small.jsonl");
    REQUIRE(replay.name() == "Aufnahme: small.jsonl");
    REQUIRE(replayAnswer(replay, "ATXYZ") == "?\r\r>");
    REQUIRE(replayAnswer(replay, "STI") == "?\r\r>");
    REQUIRE(replayAnswer(replay, "0902") == "NO DATA\r\r>");
    REQUIRE(replayAnswer(replay, "ATZ") == "\r\rELM327 v1.4b\r\r>");
    REQUIRE(replayAnswer(replay, "ATI") == "ELM327 v1.4b\r\r>");
    REQUIRE(replayAnswer(replay, "ATSP6") == "OK\r\r>");
    REQUIRE(replayAnswer(replay, "01 00") == "4100BE3FA813\r\r>");
    REQUIRE(replay.matched() == 1);
    REQUIRE(replay.unmatched() == 6);
}

TEST_CASE("Repeating live polls wrap around to the start", "[recording]") {
    ReplayTransport replay(smallRecording(
                               "{\"t\":0.0,\"dir\":\"tx\",\"data\":\"ATZ\\r\"}\n"
                               "{\"t\":0.1,\"dir\":\"rx\",\"data\":\"ELM327 v1.4b\\r\\r>\"}\n"
                               "{\"t\":1.0,\"dir\":\"tx\",\"data\":\"010C\\r\"}\n"
                               "{\"t\":1.1,\"dir\":\"rx\",\"data\":\"410C0FA0\\r\\r>\"}\n"
                               "{\"t\":1.2,\"dir\":\"tx\",\"data\":\"010D\\r\"}\n"
                               "{\"t\":1.3,\"dir\":\"rx\",\"data\":\"410D10\\r\\r>\"}\n"
                               "{\"t\":2.0,\"dir\":\"tx\",\"data\":\"010C\\r\"}\n"
                               "{\"t\":2.1,\"dir\":\"rx\",\"data\":\"410C\"}\n"     // in two pieces
                               "{\"t\":2.15,\"dir\":\"rx\",\"data\":\"1F40\\r\\r>\"}\n"
                               "{\"t\":2.2,\"dir\":\"tx\",\"data\":\"010D\\r\"}\n"
                               "{\"t\":2.3,\"dir\":\"rx\",\"data\":\"410D20\\r\\r>\"}\n"),
                           "polls");
    std::vector<std::string> rpm, speed;
    for (int round = 0; round < 5; ++round) {
        rpm.push_back(replayAnswer(replay, "01 0c"));
        speed.push_back(replayAnswer(replay, "010D"));
    }
    REQUIRE(rpm == std::vector<std::string>{"410C0FA0\r\r>", "410C1F40\r\r>", "410C0FA0\r\r>",
                                            "410C1F40\r\r>", "410C0FA0\r\r>"});
    REQUIRE(speed == std::vector<std::string>{"410D10\r\r>", "410D20\r\r>", "410D10\r\r>",
                                              "410D20\r\r>", "410D10\r\r>"});
    REQUIRE(replay.wrapped() == 2);
    REQUIRE(replay.unmatched() == 0);
}

TEST_CASE("Real-time replay honours the recorded delay", "[recording]") {
    ReplayTransport replay(smallRecording("{\"t\":1.000,\"dir\":\"tx\",\"data\":\"0100\\r\"}\n"
                                          "{\"t\":1.150,\"dir\":\"rx\",\"data\":\"4100BE3FA813\\r\\r>\"}\n"),
                           "paced", ReplayPacing::RealTime);
    replay.write("0100\r");
    REQUIRE(replay.read(20).empty());                       // not there yet
    REQUIRE(replay.read(500) == "4100BE3FA813\r\r>");       // arrives later
}

TEST_CASE("Adaptive timing: ATAT2 on request, with fallback", "[timing]") {
    setLanguage(Language::English);
    double now = 600.0;
    auto connectWith = [&](AdaptiveTiming mode, DemoAdapter adapter,
                           InterceptTransport::Rule rule, std::vector<std::string>& log) {
        auto elm = std::make_unique<Elm327>();
        elm->setVerbose(true);
        elm->setLog([&log](const std::string& line) { log.push_back(line); });
        elm->setAdaptiveTiming(mode);
        std::unique_ptr<ITransport> t = std::make_unique<DemoTransport>(warmDemo(&now, adapter));
        if (rule) t = std::make_unique<InterceptTransport>(std::move(t), rule);
        std::string error;
        REQUIRE(elm->connect(std::move(t), "6", error));
        return elm;
    };

    SECTION("standard stays with ATAT1") {
        std::vector<std::string> log;
        auto elm = connectWith(AdaptiveTiming::Standard, DemoAdapter::CloneV15, nullptr, log);
        REQUIRE(elm->adaptiveTimingLevel() == 1);
        REQUIRE_FALSE(contains(sentCommands(log), "ATAT2"));
    }
    SECTION("aggressive is accepted") {
        std::vector<std::string> log;
        auto elm = connectWith(AdaptiveTiming::Aggressive, DemoAdapter::CloneV15, nullptr, log);
        REQUIRE(elm->adaptiveTimingLevel() == 2);
        const std::vector<std::string> sent = sentCommands(log);
        REQUIRE(contains(sent, "ATAT2"));
        // ATAT2 only after the protocol was found.
        size_t at2 = 0, sp = 0;
        for (size_t i = 0; i < sent.size(); ++i) {
            if (sent[i] == "ATAT2") at2 = i;
            if (sent[i] == "ATSP6") sp = i;
        }
        REQUIRE(at2 > sp);
    }
    SECTION("refused ATAT2 falls back to ATAT1") {
        std::vector<std::string> log;
        auto elm = connectWith(
            AdaptiveTiming::Aggressive, DemoAdapter::CloneV15,
            [](const std::string& c, std::string& answer) {
                if (c != "ATAT2") return false;
                answer = "?\r\r>";
                return true;
            },
            log);
        REQUIRE(elm->adaptiveTimingLevel() == 1);
        const std::vector<std::string> sent = sentCommands(log);
        REQUIRE(sent.back() == "ATAT1");
        bool told = false;
        for (const std::string& l : log) told = told || l.find("ATAT2 refused") != std::string::npos;
        REQUIRE(told);
        // The session works as before.
        Obd2 obd(*elm);
        REQUIRE(obd.readVin() == DemoTransport::vin());
    }
    SECTION("missing answers under ATAT2 fall back to ATAT1") {
        std::vector<std::string> log;
        bool aggressive = false;
        auto elm = connectWith(
            AdaptiveTiming::Aggressive, DemoAdapter::CloneV15,
            [&aggressive](const std::string& c, std::string& answer) {
                if (c == "ATAT2") aggressive = true;
                if (c == "ATAT1") aggressive = false;
                if (!aggressive || c != "0100") return false;
                answer = "NO DATA\r\r>";
                return true;
            },
            log);
        REQUIRE(elm->adaptiveTimingLevel() == 1);
        REQUIRE(sentCommands(log).back() == "ATAT1");
    }
    SECTION("auto uses ATAT2 only on STN chips") {
        std::vector<std::string> log;
        auto clone = connectWith(AdaptiveTiming::Auto, DemoAdapter::CloneV15, nullptr, log);
        REQUIRE(clone->adaptiveTimingLevel() == 1);
        REQUIRE_FALSE(clone->isStn());
        log.clear();
        auto stn = connectWith(AdaptiveTiming::Auto, DemoAdapter::ObdLinkStn, nullptr, log);
        REQUIRE(stn->isStn());
        REQUIRE(stn->adaptiveTimingLevel() == 2);
    }
    SECTION("adapter without adaptive timing") {
        std::vector<std::string> log;
        auto elm = connectWith(
            AdaptiveTiming::Aggressive, DemoAdapter::CloneV15,
            [](const std::string& c, std::string& answer) {
                if (c.rfind("ATAT", 0) != 0) return false;
                answer = "?\r\r>";
                return true;
            },
            log);
        REQUIRE(elm->adaptiveTimingLevel() == 0);
        REQUIRE_FALSE(contains(sentCommands(log), "ATAT2"));
    }
    setLanguage(Language::German);
}

TEST_CASE("STN chips are detected during connect", "[timing]") {
    double now = 600.0;
    struct Case {
        DemoAdapter adapter;
        bool stn;
    };
    for (const Case& c : {Case{DemoAdapter::ObdLinkStn, true}, Case{DemoAdapter::CloneV15, false},
                          Case{DemoAdapter::CloneOkToAll, false},
                          Case{DemoAdapter::GenuineV14b, false}}) {
        Elm327 elm;
        std::string error;
        REQUIRE(elm.connect(std::make_unique<DemoTransport>(warmDemo(&now, c.adapter)), "6", error));
        REQUIRE(elm.isStn() == c.stn);
        if (c.stn) REQUIRE(elm.stnId() == "STN1130 v5.6.19");
        else REQUIRE(elm.stnId().empty());
        elm.disconnect();
        REQUIRE_FALSE(elm.isStn());
        REQUIRE(elm.adaptiveTimingLevel() == 0);
    }
}

TEST_CASE("Replay of a session recorded with ATAT1 answers ATAT2 with ?", "[recording][timing]") {
    // A recording made with standard timing, replayed by a driver that asks
    // for ATAT2: the replay answers "?" like an adapter without it, and the
    // driver stays with ATAT1.
    const std::string path = tempFile("caruspex_test_timing.jsonl");
    double now = 600.0;
    {
        Elm327 elm;
        std::string error;
        REQUIRE(elm.connect(std::make_unique<RecordingTransport>(
                                std::make_unique<DemoTransport>(warmDemo(&now)), path),
                            "6", error));
        elm.disconnect();
    }
    Elm327 elm;
    elm.setAdaptiveTiming(AdaptiveTiming::Aggressive);
    std::string error;
    REQUIRE(elm.connect(std::make_unique<ReplayTransport>(path), "6", error));
    REQUIRE(elm.adaptiveTimingLevel() == 1);
    elm.disconnect();
    std::filesystem::remove(std::filesystem::path(std::u8string(path.begin(), path.end())));
}
