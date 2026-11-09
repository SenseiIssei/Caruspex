// The adapter probe against the demo adapter in all its variants.

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include <string>

#include "DemoRig.h"
#include "core/AdapterProbe.h"

using namespace caruspex;

TEST_CASE("Adapter probe: v1.5 clone", "[probe]") {
    DemoRig rig(300.0);
    REQUIRE(rig.connected);
    rig.sent.clear();
    const AdapterReport report = probeAdapter(rig.elm);

    REQUIRE(report.reachable);
    REQUIRE(report.versionText == "ELM327 v1.5");
    REQUIRE(report.claimedLevel == 150);
    REQUIRE_FALSE(report.claimedVersionExists);
    REQUIRE(report.description == "OBDII to RS232 Interpreter");
    REQUIRE_FALSE(report.isStn);
    REQUIRE_FALSE(report.acceptsUnknownCommands);
    REQUIRE(report.hasVoltage);
    REQUIRE(report.voltage == Catch::Approx(14.1).margin(0.2));
    REQUIRE(report.behavesLikeLevel == 140);
    REQUIRE(report.roundTripSamples == 5);
    REQUIRE(report.responseCountHint);
    REQUIRE(report.verdict.en.find("Probably a clone") != std::string::npos);
    REQUIRE(report.verdict.en.find("v1.4") != std::string::npos);
    REQUIRE(report.verdict.de.find("Nachbau") != std::string::npos);
    REQUIRE_FALSE(report.facts.empty());

    // Nothing that writes to the adapter's memory went out.
    for (const std::string& line : rig.sent) {
        INFO(line);
        REQUIRE(line.find("> ATPP0") == std::string::npos);
        REQUIRE(line.find("SV") == std::string::npos);
        REQUIRE(line.find("> AT@3") == std::string::npos);
        REQUIRE(line.find("> ATSD") == std::string::npos);
        REQUIRE(line.find("> STSBR") == std::string::npos);
        REQUIRE(line.find("> ATZ") == std::string::npos);
    }
}

TEST_CASE("Adapter probe: genuine v1.4b behaviour", "[probe]") {
    DemoRig rig(300.0, DemoAdapter::GenuineV14b);
    REQUIRE(rig.connected);
    const AdapterReport report = probeAdapter(rig.elm);
    REQUIRE(report.claimedLevel == 142);
    REQUIRE(report.claimedVersionExists);
    REQUIRE(report.behavesLikeLevel == 142);
    REQUIRE(report.verdict.en.find("matches a genuine ELM327 v1.4b") != std::string::npos);
    REQUIRE(report.verdict.en.find("cannot be told apart") != std::string::npos);
    REQUIRE(report.verdict.de.find("echten ELM327 v1.4b") != std::string::npos);
}

TEST_CASE("Adapter probe: clone that claims v2.1", "[probe]") {
    DemoRig rig(300.0, DemoAdapter::CloneV21);
    REQUIRE(rig.connected);
    const AdapterReport report = probeAdapter(rig.elm);
    REQUIRE(report.claimedLevel == 210);
    REQUIRE(report.behavesLikeLevel == 140);
    REQUIRE(report.verdict.en.find("claims v2.1 but behaves like v1.4") != std::string::npos);
    REQUIRE(report.verdict.de.find("als v2.1 ausgibt") != std::string::npos);
}

TEST_CASE("Adapter probe: clone that says OK to everything", "[probe]") {
    DemoRig rig(300.0, DemoAdapter::CloneOkToAll);
    REQUIRE(rig.connected);
    const AdapterReport report = probeAdapter(rig.elm);
    REQUIRE(report.acceptsUnknownCommands);
    REQUIRE(report.verdict.en.find("cannot be tested reliably") != std::string::npos);
}

TEST_CASE("Adapter probe: STN chip", "[probe]") {
    DemoRig rig(300.0, DemoAdapter::ObdLinkStn);
    REQUIRE(rig.connected);
    const AdapterReport report = probeAdapter(rig.elm);
    REQUIRE(report.isStn);
    REQUIRE(report.stnId.rfind("STN", 0) == 0);
    REQUIRE(report.stnDevice == "OBDLink SX r4.2");
    REQUIRE(report.verdict.en.find("STN chip") != std::string::npos);
    REQUIRE(report.speedTips.size() >= 2);
}

TEST_CASE("Adapter probe measures the demo's answer delay", "[probe]") {
    DemoRig rig(300.0, DemoAdapter::CloneV15, true, 1.0);
    REQUIRE(rig.connected);
    const AdapterReport report = probeAdapter(rig.elm);
    REQUIRE(report.roundTripSamples == 5);
    REQUIRE(report.roundTripMs >= 5.0);
    REQUIRE(report.responseCountHint);
}

TEST_CASE("ELM327 version strings", "[probe]") {
    REQUIRE(parseVersionLevel("ELM327 v1.5") == 150);
    REQUIRE(parseVersionLevel("ELM327 v1.4b") == 142);
    REQUIRE(parseVersionLevel("ELM327 v2.1") == 210);
    REQUIRE(parseVersionLevel("ELM327 1.3a") == 131);
    REQUIRE(parseVersionLevel("OBDII") == 0);
    REQUIRE(versionText(142) == "v1.4b");
    REQUIRE(versionText(200) == "v2.0");
}
