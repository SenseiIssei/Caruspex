// Checks that the settings really reach the core.
//
// A dialog where you can change something without the behaviour changing
// would be worse than no dialog at all. That is why this measures and counts
// instead of just querying the value.
#include <catch2/catch_test_macros.hpp>

#include <chrono>
#include <memory>
#include <string>
#include <vector>

#include "core/Elm327.h"
#include "core/Transport.h"

using namespace caruspex;

namespace {

// Answers like an ELM327 on the simulator. With silent the adapter can be
// muted to run into a timeout.
class FakeTransport : public ITransport {
public:
    bool silent = false;

    bool write(const std::string& data) override {
        if (silent) return true;
        std::string cmd;
        for (char c : data) {
            if (c == '\r' || c == '\n' || c == ' ') continue;
            cmd += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
        }
        if (cmd == "ATZ") {
            pending_ = "ELM327 v1.5\r\r>";
        } else if (cmd.rfind("AT", 0) == 0) {
            pending_ = "OK\r\r>";
        } else if (cmd == "0100") {
            pending_ = "41 00 BE 1F A8 13\r\r>";
        } else {
            pending_ = "NO DATA\r\r>";
        }
        return true;
    }

    std::string read(int) override {
        if (pending_.empty()) return {};
        std::string out;
        out.swap(pending_);
        return out;
    }

    void flushInput() override { pending_.clear(); }
    std::string name() const override { return "Test"; }

private:
    std::string pending_;
};

struct Fixture {
    Elm327 elm;
    FakeTransport* transport = nullptr;
    std::vector<std::string> lines;

    Fixture() {
        auto owned = std::make_unique<FakeTransport>();
        transport = owned.get();
        elm.setLog([this](const std::string& line) { lines.push_back(line); });
        std::string error;
        REQUIRE(elm.connect(std::move(owned), "6", error));
    }

    // How many lines show the raw communication.
    std::size_t rawLines() const {
        std::size_t count = 0;
        for (const std::string& line : lines) {
            if (line.rfind("  > ", 0) == 0) ++count;
        }
        return count;
    }
};

}  // namespace

TEST_CASE("The raw log can really be switched off", "[settings]") {
    Fixture f;

    f.elm.setVerbose(true);
    f.lines.clear();
    f.elm.request("0105", 0x01);
    const std::size_t withLog = f.rawLines();
    REQUIRE(withLog > 0);

    f.elm.setVerbose(false);
    f.lines.clear();
    f.elm.request("0105", 0x01);
    REQUIRE(f.rawLines() == 0);

    // And on again, so the setting works in both directions.
    f.elm.setVerbose(true);
    f.lines.clear();
    f.elm.request("0105", 0x01);
    REQUIRE(f.rawLines() == withLog);
}

TEST_CASE("The timeout factor stretches the waiting time", "[settings]") {
    Fixture f;
    f.transport->silent = true;   // from now on nothing comes back

    const auto measure = [&f](double scale, int timeoutMs) {
        f.elm.setTimeoutScale(scale);
        const auto start = std::chrono::steady_clock::now();
        f.elm.send("0105", timeoutMs);
        return std::chrono::duration_cast<std::chrono::milliseconds>(
                   std::chrono::steady_clock::now() - start)
            .count();
    };

    // A quarter of 800 ms is 200 ms, double is 1600 ms. The bounds are
    // deliberately wide so the test does not fail for no reason on a busy
    // machine.
    const auto quarter = measure(0.25, 800);
    const auto doubled = measure(2.0, 800);

    REQUIRE(quarter < 600);
    REQUIRE(doubled > 1200);
    REQUIRE(doubled > quarter * 2);
}

TEST_CASE("A nonsensical factor is caught", "[settings]") {
    Fixture f;
    f.transport->silent = true;

    // Zero or negative would push every timeout to zero and make every
    // request fail immediately. setTimeoutScale catches that.
    f.elm.setTimeoutScale(0.0);
    const auto start = std::chrono::steady_clock::now();
    f.elm.send("0105", 300);
    const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
                             std::chrono::steady_clock::now() - start)
                             .count();
    REQUIRE(elapsed >= 250);
}
