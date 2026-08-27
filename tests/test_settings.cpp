// Prueft, dass die Einstellungen im Kern wirklich ankommen.
//
// Eine Maske, in der man etwas verstellen kann, ohne dass sich das Verhalten
// aendert, waere schlimmer als gar keine Maske. Deshalb wird hier gemessen und
// gezaehlt statt nur der Wert abgefragt.
#include <catch2/catch_test_macros.hpp>

#include <chrono>
#include <memory>
#include <string>
#include <vector>

#include "core/Elm327.h"
#include "core/Transport.h"

using namespace caruspex;

namespace {

// Antwortet wie ein ELM327 am Simulator. Mit silent laesst sich der Adapter
// stumm schalten, um in ein Zeitlimit zu laufen.
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

    // Wie viele Zeilen die Rohkommunikation zeigen.
    std::size_t rawLines() const {
        std::size_t count = 0;
        for (const std::string& line : lines) {
            if (line.rfind("  > ", 0) == 0) ++count;
        }
        return count;
    }
};

}  // namespace

TEST_CASE("Rohprotokoll laesst sich wirklich abschalten", "[einstellungen]") {
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

    // Und wieder an, damit die Einstellung in beide Richtungen wirkt.
    f.elm.setVerbose(true);
    f.lines.clear();
    f.elm.request("0105", 0x01);
    REQUIRE(f.rawLines() == withLog);
}

TEST_CASE("Der Zeitlimit-Faktor streckt die Wartezeit", "[einstellungen]") {
    Fixture f;
    f.transport->silent = true;   // ab jetzt kommt nichts mehr zurueck

    const auto measure = [&f](double scale, int timeoutMs) {
        f.elm.setTimeoutScale(scale);
        const auto start = std::chrono::steady_clock::now();
        f.elm.send("0105", timeoutMs);
        return std::chrono::duration_cast<std::chrono::milliseconds>(
                   std::chrono::steady_clock::now() - start)
            .count();
    };

    // Ein Viertel von 800 ms sind 200 ms, das Doppelte sind 1600 ms. Die
    // Grenzen sind bewusst weit, damit der Test auf einer ausgelasteten
    // Maschine nicht grundlos umfaellt.
    const auto quarter = measure(0.25, 800);
    const auto doubled = measure(2.0, 800);

    REQUIRE(quarter < 600);
    REQUIRE(doubled > 1200);
    REQUIRE(doubled > quarter * 2);
}

TEST_CASE("Ein unsinniger Faktor wird abgefangen", "[einstellungen]") {
    Fixture f;
    f.transport->silent = true;

    // Null oder negativ wuerde jedes Zeitlimit auf null druecken und jede
    // Abfrage sofort scheitern lassen. Das faengt setTimeoutScale ab.
    f.elm.setTimeoutScale(0.0);
    const auto start = std::chrono::steady_clock::now();
    f.elm.send("0105", 300);
    const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
                             std::chrono::steady_clock::now() - start)
                             .count();
    REQUIRE(elapsed >= 250);
}
