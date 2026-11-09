// The built-in demo vehicle, end to end through the real Elm327 driver and
// the OBD services.

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include <algorithm>
#include <cstdio>
#include <memory>
#include <set>
#include <string>
#include <vector>

#include "core/DemoTransport.h"
#include "core/Elm327.h"
#include "core/Obd2.h"
#include "DemoRig.h"

using namespace caruspex;

namespace {

double live(Obd2& obd, int pid) {
    LiveValue value;
    REQUIRE(obd.readLive(pid, value));
    return value.value;
}

// Data bytes of the first answer to a mode 01 request, without the PID echo.
std::vector<int> pidBytes(Elm327& elm, const std::string& command) {
    const ElmResult result = elm.request(command, 0x01);
    REQUIRE(result.ok());
    std::vector<int> data = result.messages.front();
    data.erase(data.begin());
    return data;
}

// The North American check digit, computed independently of the demo.
char vinCheckDigit(const std::string& vin) {
    auto value = [](char c) -> int {
        if (c >= '0' && c <= '9') return c - '0';
        const std::string letters = "ABCDEFGHJKLMNPRSTUVWXYZ";
        const int values[] = {1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 7, 9, 2, 3, 4, 5, 6, 7, 8, 9};
        return values[letters.find(c)];
    };
    const int weights[17] = {8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2};
    int sum = 0;
    for (int i = 0; i < 17; ++i) sum += value(vin[i]) * weights[i];
    const int rest = sum % 11;
    return rest == 10 ? 'X' : static_cast<char>('0' + rest);
}

bool hasCode(const std::vector<Fault>& faults, const std::string& code, const char* kind) {
    for (const Fault& f : faults) {
        if (f.code == code && f.kind == kind) return true;
    }
    return false;
}

int notReady(const VehicleStatus& status) {
    return static_cast<int>(std::count_if(status.readiness.begin(), status.readiness.end(),
                                          [](const ReadinessItem& r) { return !r.ready; }));
}

}  // namespace

TEST_CASE("The demo adapter talks like an ELM327", "[demo]") {
    double now = 30.0;
    DemoOptions options;
    options.clock = [&] { return now; };
    options.latencyScale = 0.0;
    DemoTransport demo(options);
    demo.flushInput();   // power-on greeting

    auto ask = [&](const std::string& command) {
        demo.write(command + "\r");
        std::string out;
        for (int i = 0; i < 20 && out.find('>') == std::string::npos; ++i) out += demo.read(50);
        return out;
    };

    REQUIRE(ask("ATZ") == "ATZ\r\r\rELM327 v1.5\r\r>");
    REQUIRE(ask("ATI") == "ATI\rELM327 v1.5\r\r>");
    REQUIRE(ask("ATE0") == "ATE0\rOK\r\r>");   // still echoed
    REQUIRE(ask("ATI") == "ELM327 v1.5\r\r>");
    REQUIRE(ask("AT@1") == "OBDII to RS232 Interpreter\r\r>");
    REQUIRE(ask("ATXYZ") == "?\r\r>");
    REQUIRE(ask("HELLO") == "?\r\r>");
    REQUIRE(ask("ATDP") == "AUTO\r\r>");

    // Automatic search on the first request, then the protocol stays.
    const std::string first = ask("0100");
    REQUIRE(first.rfind("SEARCHING...\r41 00 ", 0) == 0);
    REQUIRE(first.find("\r41 00 ", 15) != std::string::npos);   // the gearbox answers too
    REQUIRE(ask("ATDPN") == "A6\r\r>");
    REQUIRE(ask("ATDP") == "AUTO, ISO 15765-4 (CAN 11/500)\r\r>");
    REQUIRE(ask("0100").rfind("41 00 ", 0) == 0);
    REQUIRE(ask("0108") == "NO DATA\r\r>");   // not supported by anyone
    REQUIRE(ask("") == "NO DATA\r\r>");   // a bare carriage return repeats 0108
    const std::string volts = ask("ATRV");   // engine running: charging voltage
    REQUIRE(volts.rfind("14.", 0) == 0);
    REQUIRE(volts.find("V\r\r>") == 4);

    // Spaces off, headers on: CAN identifier, PCI byte and padding.
    REQUIRE(ask("ATS0") == "OK\r\r>");
    REQUIRE(ask("ATH1") == "OK\r\r>");
    const std::string headed = ask("0100");
    REQUIRE(headed.rfind("7E806410", 0) == 0);
    REQUIRE(headed.find("\r7E906410") != std::string::npos);
    REQUIRE(ask("ATD1") == "OK\r\r>");
    REQUIRE(ask("0100").rfind("7E8806410", 0) == 0);
    REQUIRE(ask("ATCRA7E9") == "OK\r\r>");
    REQUIRE(ask("0100").rfind("7E9806410", 0) == 0);
    REQUIRE(ask("ATAR") == "OK\r\r>");

    // A physical request reaches only one control unit, and only it may
    // refuse a service.
    REQUIRE(ask("ATD0") == "OK\r\r>");
    REQUIRE(ask("ATSH7E1") == "OK\r\r>");
    REQUIRE(ask("0902") == "NO DATA\r\r>");   // the gearbox has no VIN
    REQUIRE(ask("22F190").rfind("7E9037F2211", 0) == 0);
    REQUIRE(ask("ATSH7DF") == "OK\r\r>");

    // A wrong fixed protocol fails like on a real car.
    REQUIRE(ask("ATSP5") == "OK\r\r>");
    REQUIRE(ask("0100") == "BUS INIT: ERROR\r\r>");
    REQUIRE(ask("ATSP8") == "OK\r\r>");
    REQUIRE(ask("0100") == "CAN ERROR\r\r>");
    REQUIRE(ask("ATSP6") == "OK\r\r>");
    REQUIRE(ask("ATDPN") == "6\r\r>");
    REQUIRE(ask("0100").rfind("7E806410", 0) == 0);

    // Monitoring a silent bus shows nothing until a character stops it.
    demo.write("ATMA\r");
    REQUIRE(demo.read(20).empty());
    REQUIRE(ask("X") == "STOPPED\r\r>");
}

TEST_CASE("Elm327 and Obd2 connect to the demo car", "[demo]") {
    DemoRig rig(300.0);
    REQUIRE(rig.connected);
    REQUIRE(rig.elm.version() == "ELM327 v1.5");
    REQUIRE(rig.elm.protocolText() == "ISO 15765-4 (CAN 11/500)");
    Obd2 obd(rig.elm);

    const std::string vin = obd.readVin();
    REQUIRE(vin == DemoTransport::vin());
    REQUIRE(vin.size() == 17);
    REQUIRE(vin.find_first_of("IOQ") == std::string::npos);
    REQUIRE(vin[8] == vinCheckDigit(vin));

    const std::set<int> pids = obd.supportedPids();
    for (int pid : {0x01, 0x05, 0x0C, 0x0D, 0x10, 0x14, 0x15, 0x2F, 0x42, 0x5C, 0x5E})
        REQUIRE(pids.count(pid) == 1);
    REQUIRE(pids.count(0x0A) == 0);
    // Every supported PID really answers, and nothing else does.
    for (int pid = 0x01; pid <= 0x7F; ++pid) {
        if (pid % 0x20 == 0) continue;
        char command[8];
        std::snprintf(command, sizeof(command), "01%02X", pid);
        const ElmResult result = rig.elm.request(command, 0x01);
        INFO("PID " << pid);
        REQUIRE(result.ok() == (pids.count(pid) == 1));
    }

    const VehicleIdentity identity = obd.readIdentity();
    REQUIRE(identity.vin == DemoTransport::vin());
    REQUIRE(identity.calibrationId.rfind("CX5EM21A0417", 0) == 0);
    REQUIRE(identity.ecuName.rfind("ECM-EngineControl", 0) == 0);
    REQUIRE_FALSE(identity.calibrationVerification.empty());
}

TEST_CASE("Without the gearbox the identification is unambiguous", "[demo]") {
    DemoRig rig(300.0, DemoAdapter::CloneV15, false);
    REQUIRE(rig.connected);
    Obd2 obd(rig.elm);
    const VehicleIdentity identity = obd.readIdentity();
    REQUIRE(identity.calibrationId == "CX5EM21A0417");
    REQUIRE(identity.ecuName == "ECM-EngineControl");
}

TEST_CASE("Automatic protocol search finds the demo car", "[demo]") {
    double now = 100.0;
    DemoOptions options;
    options.clock = [&] { return now; };
    options.latencyScale = 0.0;
    Elm327 elm;
    std::string error;
    REQUIRE(elm.connect(std::make_unique<DemoTransport>(options), "0", error));
    REQUIRE(elm.protocolText() == "AUTO, ISO 15765-4 (CAN 11/500)");
    REQUIRE(elm.connect(std::make_unique<DemoTransport>(options), kProtocolOrderCanFirst, error));
}

TEST_CASE("Demo faults: read, clear, and come back", "[demo]") {
    DemoRig rig(300.0);
    REQUIRE(rig.connected);
    Obd2 obd(rig.elm);
    std::string error;

    std::vector<Fault> faults = obd.readFaults(error);
    REQUIRE(hasCode(faults, "P0171", L("gespeichert", "stored")));
    REQUIRE(hasCode(faults, "P0420", L("gespeichert", "stored")));
    REQUIRE(hasCode(faults, "P0300", L("sporadisch", "intermittent")));
    REQUIRE(faults.size() == 3);

    VehicleStatus status = obd.readStatus();
    REQUIRE(status.valid);
    REQUIRE(status.milOn);
    REQUIRE(status.faultCount == 2);
    REQUIRE(notReady(status) == 2);   // EVAP and EGR/VVT
    REQUIRE(live(obd, 0x31) > 1800.0);

    std::string trigger;
    REQUIRE_FALSE(obd.readFreezeFrame(trigger).empty());
    REQUIRE(trigger == "P0171");

    REQUIRE(obd.clearFaults(error));

    // Stored and pending are gone; permanent codes stay until the monitors
    // pass again, exactly what mode 0A is for.
    faults = obd.readFaults(error);
    REQUIRE(faults.size() == 2);
    REQUIRE(hasCode(faults, "P0171", L("dauerhaft", "permanent")));
    REQUIRE(hasCode(faults, "P0420", L("dauerhaft", "permanent")));
    status = obd.readStatus();
    REQUIRE_FALSE(status.milOn);
    REQUIRE(status.faultCount == 0);
    REQUIRE(notReady(status) == 5);
    REQUIRE(live(obd, 0x31) == 0.0);
    REQUIRE(live(obd, 0x4E) == 0.0);
    REQUIRE(live(obd, 0x30) == 0.0);
    REQUIRE(obd.readFreezeFrame(trigger).empty());
    REQUIRE(trigger.empty());

    // After a while of driving the causes are still there.
    rig.now += 700.0;
    faults = obd.readFaults(error);
    REQUIRE(hasCode(faults, "P0171", L("gespeichert", "stored")));
    REQUIRE(hasCode(faults, "P0300", L("sporadisch", "intermittent")));
    REQUIRE(hasCode(faults, "P0420", L("sporadisch", "intermittent")));
    status = obd.readStatus();
    REQUIRE(status.milOn);
    REQUIRE(status.faultCount == 1);
    REQUIRE(notReady(status) == 1);   // EVAP never completes in the demo
    REQUIRE(live(obd, 0x31) > 5.0);
    REQUIRE(live(obd, 0x4E) >= 11.0);
    REQUIRE_FALSE(obd.readFreezeFrame(trigger).empty());
    REQUIRE(trigger == "P0300");
}

TEST_CASE("Demo live values follow the injected clock", "[demo]") {
    DemoRig rig(1.0);   // ignition on, engine not yet started
    REQUIRE(rig.connected);
    Obd2 obd(rig.elm);

    REQUIRE(live(obd, 0x0C) == 0.0);
    REQUIRE(live(obd, 0x0D) == 0.0);
    REQUIRE(live(obd, 0x42) == Catch::Approx(12.37).margin(0.1));
    REQUIRE(rig.elm.send("ATRV").lines.front() == "12.6V");
    REQUIRE(live(obd, 0x05) == Catch::Approx(12.0).margin(1.0));

    rig.now = 20.0;   // idling, cold
    const double coldCoolant = live(obd, 0x05);
    REQUIRE(live(obd, 0x0C) > 900.0);
    REQUIRE(live(obd, 0x0D) == 0.0);
    REQUIRE(live(obd, 0x42) > 13.0);
    REQUIRE(pidBytes(rig.elm, "0103")[0] == 0x01);   // open loop, cold

    // Driving starts 50 s after ignition on, the cycle lasts 120 s.
    // 70 s into a cycle the car cruises at 100 km/h in sixth gear.
    rig.now = 50.0 + 3 * 120.0 + 70.0;
    REQUIRE(live(obd, 0x05) > coldCoolant + 30.0);
    const double speed = live(obd, 0x0D);
    const double rpm = live(obd, 0x0C);
    REQUIRE(speed == Catch::Approx(100.0).margin(2.0));
    const std::vector<int> gear = pidBytes(rig.elm, "01A4");
    REQUIRE(gear.size() == 4);
    REQUIRE((gear[1] >> 4) == 6);
    const double ratio = ((gear[2] << 8) | gear[3]) / 1000.0;
    REQUIRE(ratio == Catch::Approx(0.82));
    // rpm from speed, gear ratio, final drive 3.2 and 1.94 m tyre circumference.
    REQUIRE(rpm == Catch::Approx(speed / 3.6 / 1.94 * 60.0 * ratio * 3.2).margin(40.0));
    REQUIRE(pidBytes(rig.elm, "0103")[0] == 0x02);   // closed loop
    REQUIRE(live(obd, 0x07) == Catch::Approx(17.2).margin(0.8));
    REQUIRE(live(obd, 0x10) > 5.0);
    REQUIRE(live(obd, 0x5E) > 2.0);

    // Several PIDs in one request, as CAN allows.
    const ElmResult multi = rig.elm.request("010C0D05", 0x01);
    REQUIRE(multi.ok());
    // 0C a b 0D a 05 a: eight bytes with the service byte, so two frames.
    // Without headers the padding of the last frame is dropped.
    REQUIRE(multi.messages.front().size() == 7);
    REQUIRE(multi.messages.front()[0] == 0x0C);
    REQUIRE(multi.messages.front()[3] == 0x0D);
    REQUIRE(multi.messages.front()[5] == 0x05);

    // The O2 sensor switches.
    double low = 1.0, high = 0.0;
    for (int i = 0; i < 20; ++i) {
        rig.now += 0.05;
        const double v = live(obd, 0x14);
        low = std::min(low, v);
        high = std::max(high, v);
    }
    REQUIRE(low < 0.2);
    REQUIRE(high > 0.7);

    // Braking: fuel cut.
    rig.now = 50.0 + 3 * 120.0 + 90.0;
    REQUIRE(pidBytes(rig.elm, "0103")[0] == 0x04);
    REQUIRE(live(obd, 0x5E) == 0.0);

    // Odometer and fuel level over a longer drive.
    // PID A6 is read raw: the odometer is newer than the PID table.
    auto odometer = [&] {
        const std::vector<int> d = pidBytes(rig.elm, "01A6");
        REQUIRE(d.size() == 4);
        return ((static_cast<double>(d[0]) * 256.0 + d[1]) * 256.0 + d[2]) * 256.0 / 10.0 +
               d[3] / 10.0;
    };
    const double odometerBefore = odometer();
    const double fuelBefore = live(obd, 0x2F);
    const double runBefore = live(obd, 0x1F);
    REQUIRE(odometerBefore > 48213.0);
    rig.now += 1200.0;
    const double odometerAfter = odometer();
    REQUIRE(odometerAfter - odometerBefore == Catch::Approx(18.3).margin(0.5));   // 54.8 km/h
    REQUIRE(live(obd, 0x2F) < fuelBefore);
    REQUIRE(live(obd, 0x1F) == Catch::Approx(runBefore + 1200.0).margin(1.0));
}

TEST_CASE("Demo answers are deterministic with an injected clock", "[demo]") {
    DemoRig a(777.7), b(777.7);
    REQUIRE(a.connected);
    REQUIRE(b.connected);
    for (const char* command : {"010C", "010D", "0114", "0110", "015E", "01A6", "06A4"})
        REQUIRE(a.elm.send(command).lines == b.elm.send(command).lines);
}

TEST_CASE("Demo multi-frame answers with and without headers", "[demo]") {
    DemoRig rig(300.0);
    REQUIRE(rig.connected);

    // Headers off: total length, then numbered lines.
    ElmResult plain = rig.elm.send("0902");
    REQUIRE(plain.lines.size() == 4);
    REQUIRE(plain.lines[0] == "014");
    REQUIRE(plain.lines[1].rfind("0:490201", 0) == 0);
    REQUIRE(plain.lines[2].rfind("1:", 0) == 0);

    // Headers on: identifier, PCI bytes and the reassembly is up to us.
    REQUIRE(rig.elm.send("ATH1").lines.front() == "OK");
    ElmResult headed = rig.elm.send("0902");
    REQUIRE(headed.lines.size() == 3);
    std::vector<int> payload;
    int length = 0;
    for (size_t i = 0; i < headed.lines.size(); ++i) {
        const std::string& line = headed.lines[i];
        REQUIRE(line.rfind("7E8", 0) == 0);
        const std::vector<int> frame = hexToBytes(line.substr(3));
        REQUIRE(frame.size() == 8);
        if (i == 0) {
            REQUIRE(frame[0] == 0x10);   // first frame
            length = frame[1];
            payload.insert(payload.end(), frame.begin() + 2, frame.end());
        } else {
            REQUIRE(frame[0] == static_cast<int>(0x20 + i));   // consecutive frames
            payload.insert(payload.end(), frame.begin() + 1, frame.end());
        }
    }
    REQUIRE(length == 20);
    payload.resize(length);
    REQUIRE(payload[0] == 0x49);
    REQUIRE(payload[1] == 0x02);
    REQUIRE(std::string(payload.begin() + 3, payload.end()) == DemoTransport::vin());

    // Both control units answer the calibration request, three frames each.
    ElmResult both = rig.elm.send("0904");
    REQUIRE(both.lines.size() == 6);
    REQUIRE(std::count_if(both.lines.begin(), both.lines.end(), [](const std::string& l) {
                return l.rfind("7E9", 0) == 0;
            }) == 3);
    REQUIRE(rig.elm.send("ATH0").lines.front() == "OK");
}

TEST_CASE("Demo mode 06 reports misfire counters per cylinder", "[demo]") {
    DemoRig rig(600.0);
    REQUIRE(rig.connected);

    const ElmResult mask = rig.elm.request("0600", 0x06);
    REQUIRE(mask.ok());
    REQUIRE(mask.messages.front() == std::vector<int>{0x00, 0xC0, 0x00, 0x00, 0x01});
    const ElmResult misfireMask = rig.elm.request("06A0", 0x06);
    REQUIRE(misfireMask.ok());
    REQUIRE(misfireMask.messages.front() == std::vector<int>{0xA0, 0xF8, 0x00, 0x00, 0x00});

    auto counters = [&](const char* command) {
        const ElmResult r = rig.elm.request(command, 0x06);
        REQUIRE(r.ok());
        const std::vector<int>& m = r.messages.front();
        REQUIRE(m.size() >= 18);
        REQUIRE(m[1] == 0x0B);   // EWMA over the last ten driving cycles
        REQUIRE(m[2] == 0x24);   // counts
        REQUIRE(m[10] == 0x0C);  // this driving cycle
        return std::vector<int>{(m[3] << 8) | m[4], (m[12] << 8) | m[13]};
    };
    const std::vector<int> cylinder1 = counters("06A2");
    const std::vector<int> cylinder3 = counters("06A4");
    REQUIRE(cylinder3[0] > cylinder1[0]);
    REQUIRE(cylinder3[1] > cylinder1[1]);

    // The catalyst test fails, which is what P0420 is about.
    const ElmResult catalyst = rig.elm.request("0621", 0x06);
    REQUIRE(catalyst.ok());
    const std::vector<int>& c = catalyst.messages.front();
    const int value = (c[3] << 8) | c[4];
    const int maximum = (c[7] << 8) | c[8];
    REQUIRE(value > maximum);
}
