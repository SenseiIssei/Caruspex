#include "core/DemoTransport.h"

#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <deque>
#include <functional>
#include <optional>
#include <thread>
#include <utility>
#include <vector>

#include "core/Lang.h"

namespace caruspex {
namespace {

using Bytes = std::vector<std::uint8_t>;
using SteadyClock = std::chrono::steady_clock;

constexpr double kPi = 3.14159265358979323846;

// ---------------------------------------------------------------------------
// Adapter profiles
// ---------------------------------------------------------------------------

// Command set levels: the ELM327 version in which a command first appeared,
// written as major * 100 + minor * 10 + letter (1.3a = 131, 1.4b = 142).
constexpr int kV10 = 100, kV11 = 110, kV12 = 120, kV13 = 130, kV13a = 131, kV14 = 140,
              kV14b = 142, kV20 = 200;

struct Profile {
    const char* version;       // ATI
    const char* description;   // AT@1
    int level;                 // highest command set it implements
    const char* stnId;         // STI, nullptr when not an STN chip
    const char* stnDevice;     // STDI
    bool okToAll;              // answers "OK" to unknown AT commands
};

Profile profileFor(DemoAdapter adapter) {
    switch (adapter) {
        case DemoAdapter::GenuineV14b:
            return {"ELM327 v1.4b", "OBDII to RS232 Interpreter", kV14b, nullptr, nullptr, false};
        case DemoAdapter::CloneV21:
            return {"ELM327 v2.1", "OBDII to RS232 Interpreter", kV14, nullptr, nullptr, false};
        case DemoAdapter::CloneOkToAll:
            return {"ELM327 v2.1", "OBDII to RS232 Interpreter", kV14, nullptr, nullptr, true};
        case DemoAdapter::ObdLinkStn:
            return {"ELM327 v1.4b", "SCANTOOL.NET LLC", kV14b, "STN1130 v5.6.19",
                    "OBDLink SX r4.2", false};
        case DemoAdapter::CloneV15:
        default:
            return {"ELM327 v1.5", "OBDII to RS232 Interpreter", kV14, nullptr, nullptr, false};
    }
}

// ---------------------------------------------------------------------------
// The car
// ---------------------------------------------------------------------------

// Fictional manufacturer code "YCX", model year M (2021), plant C. Position 9
// is the check digit after the North American rule (it comes out as 7).
const char kVin[] = "YCXDE15C7MC048217";
const char kEngineCalibration[] = "CX5EM21A0417";
const char kGearboxCalibration[] = "CX6AT21B0203";
const std::uint8_t kEngineCvn[4] = {0x5E, 0x71, 0x0C, 0x93};
const std::uint8_t kGearboxCvn[4] = {0x2B, 0x90, 0x47, 0xD1};
// Mode 09 ECU name: four characters acronym, a separator, then the name,
// 20 bytes in total, padded with zero bytes.
const char kEngineName[] = "ECM\0-EngineControl";
const char kGearboxName[] = "TCM\0-TransmissionCtl";

constexpr double kAmbient = 12.0;        // deg C
constexpr double kBaro = 99.0;           // kPa
constexpr double kOdometerStart = 48213.4;
constexpr double kFuelStart = 64.3;      // percent
constexpr double kTankLitres = 50.0;
constexpr double kLongTermTrim = 17.2;   // percent: the reason for P0171
constexpr double kCrankSeconds = 1.0;
constexpr double kWarmIdleSeconds = 45.0;

// Six-speed automatic, 1.4 litre turbo-less petrol engine.
constexpr double kRatios[6] = {4.20, 2.50, 1.65, 1.25, 1.00, 0.82};
constexpr double kFinalDrive = 3.2;
constexpr double kTyreCircumference = 1.94;   // metres

enum class Phase { Off, Crank, Idle, Accelerate, Cruise, Brake };

struct Segment {
    double duration;  // s
    double v0;        // km/h
    double v1;
    Phase phase;
};

// One drive cycle, repeated forever after the warm-up idle.
const Segment kCycle[] = {
    {12.0, 0.0, 50.0, Phase::Accelerate},
    {28.0, 50.0, 50.0, Phase::Cruise},
    {15.0, 50.0, 100.0, Phase::Accelerate},
    {30.0, 100.0, 100.0, Phase::Cruise},
    {15.0, 100.0, 0.0, Phase::Brake},
    {20.0, 0.0, 0.0, Phase::Idle},
};
constexpr double kCruiseWobble = 1.2;   // km/h, two full periods per cruise

// Speed and distance (km) at a point within one segment.
void segmentPoint(const Segment& seg, double s, double& speed, double& km) {
    if (seg.phase == Phase::Cruise) {
        const double period = seg.duration / 2.0;
        const double w = 2.0 * kPi / period;
        speed = seg.v0 + kCruiseWobble * std::sin(w * s);
        km = (seg.v0 * s + kCruiseWobble / w * (1.0 - std::cos(w * s))) / 3600.0;
        return;
    }
    const double slope = (seg.v1 - seg.v0) / seg.duration;
    speed = seg.v0 + slope * s;
    km = (seg.v0 * s + 0.5 * slope * s * s) / 3600.0;
}

double cycleSeconds() {
    double total = 0.0;
    for (const Segment& seg : kCycle) total += seg.duration;
    return total;
}

double cycleKm() {
    double total = 0.0;
    for (const Segment& seg : kCycle) {
        double v = 0.0, km = 0.0;
        segmentPoint(seg, seg.duration, v, km);
        total += km;
    }
    return total;
}

// Everything a sensor could report at one moment.
struct Sample {
    Phase phase = Phase::Off;
    double runTime = 0.0;   // s since the engine runs on its own
    double speed = 0.0, rpm = 0.0, distanceKm = 0.0;
    int gear = 0;
    double gearRatio = 0.0;
    double load = 0.0, throttleRel = 0.0, throttleAbs = 14.5, throttleCmd = 0.0;
    double pedalD = 14.9, pedalE = 7.5;
    double map = kBaro, maf = 0.0, fuelRate = 0.0, timing = 0.0;
    double iat = kAmbient + 2.0, coolant = kAmbient, oil = kAmbient, catTemp = kAmbient;
    double rail = 0.0;   // kPa
    double stft = 0.0, ltft = kLongTermTrim, o2up = 0.45, o2down = 0.45, lambda = 1.0;
    int fuelStatus = 0x00;
    double battery = 12.6;
    double fuelLevel = kFuelStart, odometer = kOdometerStart;
    double torqueDemand = 0.0, torqueActual = 0.0;
};

double engineRunTime(double t, double start) {
    return std::max(0.0, t - start - kCrankSeconds);
}

// The whole car as a pure function of time. No state, so any moment can be
// looked at again (the freeze frame does exactly that).
Sample simulate(double t, double start) {
    Sample s;
    if (t < start) {
        s.phase = Phase::Off;
        // A healthy battery at rest, sagging a little with the ignition on.
        s.battery = std::max(12.45, 12.62 - 0.0004 * t);
        return s;
    }
    if (t < start + kCrankSeconds) {
        s.phase = Phase::Crank;
        s.rpm = 240.0 + 30.0 * std::sin(t * 40.0);
        s.battery = 10.4 + 0.2 * std::sin(t * 25.0);
        s.fuelStatus = 0x01;
        s.lambda = 0.9;
        s.rail = 3000.0;
        return s;
    }

    const double run = t - start - kCrankSeconds;
    s.runTime = run;

    // Warm-up. The thermostat holds the coolant near 90 deg C once warm.
    const double warm = 1.0 - std::exp(-run / 200.0);
    s.coolant = kAmbient + (90.0 - kAmbient) * warm +
                1.5 * std::sin(run / 45.0) * warm * warm * warm * warm;
    s.oil = kAmbient + (96.0 - kAmbient) * (1.0 - std::exp(-run / 380.0));
    const double cold = std::clamp((70.0 - s.coolant) / 58.0, 0.0, 1.0);

    // Where in the drive cycle are we?
    s.phase = Phase::Idle;
    double segTime = 0.0;
    const double driveTime = run - kWarmIdleSeconds;
    if (driveTime > 0.0) {
        const double length = cycleSeconds();
        const double cycles = std::floor(driveTime / length);
        double tau = driveTime - cycles * length;
        double km = cycles * cycleKm();
        for (const Segment& seg : kCycle) {
            if (tau <= seg.duration) {
                double v = 0.0, part = 0.0;
                segmentPoint(seg, tau, v, part);
                s.speed = std::max(0.0, v);
                km += part;
                s.phase = seg.phase;
                segTime = tau;
                break;
            }
            double v = 0.0, part = 0.0;
            segmentPoint(seg, seg.duration, v, part);
            km += part;
            tau -= seg.duration;
        }
        s.distanceKm = km;
    }

    // Gearbox: shifts up early when accelerating gently, holds the highest
    // gear that keeps the engine above about 1250 rpm when cruising.
    auto rpmAt = [](double kmh, int gear) {
        return kmh / 3.6 / kTyreCircumference * 60.0 * kRatios[gear - 1] * kFinalDrive;
    };
    int gear = 1;
    if (s.speed >= 3.0) {
        if (s.phase == Phase::Accelerate) {
            gear = 6;
            for (int g = 1; g <= 6; ++g) {
                if (rpmAt(s.speed, g) <= 2700.0) {
                    gear = g;
                    break;
                }
            }
        } else {
            const double lowest = s.phase == Phase::Brake ? 1050.0 : 1250.0;
            for (int g = 6; g >= 1; --g) {
                if (rpmAt(s.speed, g) >= lowest) {
                    gear = g;
                    break;
                }
            }
        }
    }
    s.gear = gear;
    s.gearRatio = kRatios[gear - 1];

    const double idle = 760.0 + 340.0 * cold;
    double rpm = s.speed < 3.0 ? idle : rpmAt(s.speed, gear);
    if (s.phase == Phase::Accelerate && s.speed < 25.0)
        rpm = std::max(rpm, 1500.0 + 40.0 * s.speed);   // torque converter slip
    s.rpm = std::max(rpm, idle) + 8.0 * std::sin(t * 7.3);

    // Driver, throttle and load.
    switch (s.phase) {
        case Phase::Accelerate:
            s.throttleRel = 36.0 + 6.0 * std::sin(segTime * 0.9);
            s.load = 68.0 + 8.0 * std::sin(segTime * 0.7);
            s.timing = 16.0 + 2.0 * std::sin(t * 1.3);
            s.torqueDemand = 58.0;
            s.rail = 14000.0;
            break;
        case Phase::Cruise:
            s.throttleRel = 9.0 + 0.09 * s.speed + 1.5 * std::sin(t * 0.8);
            s.load = 20.0 + 0.14 * s.speed + 2.0 * std::sin(t * 0.8);
            s.timing = 32.0 - 0.25 * s.load;
            s.torqueDemand = s.load * 0.85;
            s.rail = 7000.0 + 30.0 * s.speed;
            break;
        case Phase::Brake:
            s.throttleRel = 0.0;
            s.load = 13.0 + 2.0 * std::sin(t);
            s.timing = 22.0;
            s.torqueDemand = 0.0;
            s.rail = 4000.0;
            break;
        default:
            s.throttleRel = 0.0;
            s.load = 20.0 + 6.0 * cold + 1.0 * std::sin(t * 1.7);
            s.timing = 7.0 + 3.0 * std::sin(t * 2.1) - 4.0 * cold;
            s.torqueDemand = 8.0;
            s.rail = 4500.0;
            break;
    }
    const bool fuelCut = s.phase == Phase::Brake && s.rpm > 1150.0 && cold < 0.5;
    s.torqueActual = fuelCut ? 0.0 : s.load * 0.92;
    s.throttleAbs = 14.5 + s.throttleRel * 0.82;
    s.throttleCmd = s.throttleRel * 0.95 + (s.speed < 3.0 ? 3.1 + 2.0 * cold : 0.0);
    s.pedalD = 14.9 + s.throttleRel * 0.75;
    s.pedalE = s.pedalD / 2.0;
    s.map = std::min(kBaro, 22.0 + s.load * 0.78);
    s.maf = s.rpm / 120.0 * 1.4 * 1.18 * s.load / 100.0 * 1.05;
    s.iat = kAmbient + 3.0 + 7.0 * std::exp(-s.speed / 40.0) + 3.0 * (1.0 - std::exp(-run / 300.0));

    // Mixture control: open loop while cold, fuel cut on overrun, otherwise
    // closed loop with the upstream sensor switching around lambda 1.
    if (run <= 25.0 || s.coolant <= 35.0) {
        s.fuelStatus = 0x01;
        s.lambda = 0.94;
        s.o2up = run < 15.0 ? 0.45 : 0.62;
        s.o2down = 0.45;
        s.stft = 0.0;
    } else if (fuelCut) {
        s.fuelStatus = 0x04;
        s.lambda = 1.99;
        s.o2up = 0.04;
        s.o2down = 0.09;
        s.stft = 0.0;
    } else {
        s.fuelStatus = 0x02;
        s.lambda = 1.0;
        const double phase = 2.0 * kPi * 1.1 * t;
        s.o2up = 0.45 + 0.38 * std::tanh(3.0 * std::sin(phase));
        s.stft = 2.0 - 5.5 * std::tanh(3.0 * std::sin(phase - 0.5));
        // A worn catalyst stores little oxygen, so the downstream sensor
        // follows the upstream one instead of staying calm (P0420).
        s.o2down = 0.47 + 0.24 * std::tanh(2.0 * std::sin(phase - 1.1));
    }
    s.fuelRate = fuelCut ? 0.0 : s.maf / (14.7 * s.lambda) * 3.6 / 0.745;

    double catTarget = 430.0;
    if (s.phase == Phase::Cruise) catTarget = 560.0 + 1.2 * s.speed;
    if (s.phase == Phase::Accelerate) catTarget = 700.0;
    if (s.phase == Phase::Brake) catTarget = 520.0;
    s.catTemp = kAmbient + (catTarget - kAmbient) * (1.0 - std::exp(-run / 170.0));

    s.battery = 14.2 + 0.06 * std::sin(t * 0.9) - 0.5 * std::exp(-run / 25.0);

    const double fuelUsed = 0.74 * run / 3600.0 + 0.068 * s.distanceKm;
    s.fuelLevel = kFuelStart - fuelUsed / kTankLitres * 100.0;
    s.odometer = kOdometerStart + s.distanceKm;
    return s;
}

// Run time at which the coolant first reaches 70 deg C, which counts as a
// warm-up cycle.
double warmUpRunTime() {
    return 200.0 * std::log((90.0 - kAmbient) / (90.0 - 70.0));
}

// ---------------------------------------------------------------------------
// Encoding helpers
// ---------------------------------------------------------------------------

std::uint8_t u8(double value) {
    return static_cast<std::uint8_t>(std::clamp(std::lround(value), 0L, 255L));
}

void put16(Bytes& out, double value) {
    const long v = std::clamp(std::lround(value), 0L, 65535L);
    out.push_back(static_cast<std::uint8_t>((v >> 8) & 0xFF));
    out.push_back(static_cast<std::uint8_t>(v & 0xFF));
}

void put32(Bytes& out, double value) {
    const long long v = std::clamp(std::llround(value), 0LL, 0xFFFFFFFFLL);
    for (int shift = 24; shift >= 0; shift -= 8)
        out.push_back(static_cast<std::uint8_t>((v >> shift) & 0xFF));
}

std::uint8_t percent(double value) { return u8(value * 255.0 / 100.0); }
std::uint8_t trimByte(double value) { return u8(value * 128.0 / 100.0 + 128.0); }

// "P0171" -> 0x01 0x71
void putDtc(Bytes& out, const char* code) {
    int first = 0;
    switch (code[0]) {
        case 'C': first = 1; break;
        case 'B': first = 2; break;
        case 'U': first = 3; break;
        default: first = 0; break;
    }
    auto digit = [](char c) { return c >= 'A' ? c - 'A' + 10 : c - '0'; };
    out.push_back(static_cast<std::uint8_t>((first << 6) | (digit(code[1]) << 4) | digit(code[2])));
    out.push_back(static_cast<std::uint8_t>((digit(code[3]) << 4) | digit(code[4])));
}

// Support bitmask for the identifiers base+1 .. base+32. The last bit says
// whether the next range has anything, the way J1979 chains them.
Bytes supportMask(const std::vector<int>& ids, int base) {
    std::uint32_t mask = 0;
    for (int id : ids) {
        if (id > base && id <= base + 32) mask |= 1u << (32 - (id - base));
        if (id > base + 32) mask |= 1u;
    }
    return {static_cast<std::uint8_t>(mask >> 24), static_cast<std::uint8_t>(mask >> 16),
            static_cast<std::uint8_t>(mask >> 8), static_cast<std::uint8_t>(mask)};
}

// A range request is only answered when the chain reaches it.
bool rangeReachable(const std::vector<int>& ids, int base) {
    if (base == 0) return true;
    for (int id : ids) {
        if (id > base) return true;
    }
    return false;
}

constexpr char hexDigits[] = "0123456789ABCDEF";

std::string hex2(int value) {
    return {hexDigits[(value >> 4) & 0xF], hexDigits[value & 0xF]};
}

std::string hex3(int value) {
    return {hexDigits[(value >> 8) & 0xF], hexDigits[(value >> 4) & 0xF], hexDigits[value & 0xF]};
}

bool isHex(char c) {
    return (c >= '0' && c <= '9') || (c >= 'A' && c <= 'F');
}

int hexValue(char c) { return c <= '9' ? c - '0' : c - 'A' + 10; }

bool allHex(const std::string& s) {
    return !s.empty() && std::all_of(s.begin(), s.end(), isHex);
}

// c == prefix followed by exactly `digits` hex digits.
bool hexArg(const std::string& c, const char* prefix, size_t digits, int* value = nullptr) {
    const std::string p(prefix);
    if (c.size() != p.size() + digits || c.compare(0, p.size(), p) != 0) return false;
    const std::string rest = c.substr(p.size());
    if (!allHex(rest)) return false;
    if (value) *value = static_cast<int>(std::stoul(rest, nullptr, 16));
    return true;
}

// ISO-TP framing for one response on CAN, padded to eight bytes.
std::vector<std::array<std::uint8_t, 8>> isoTpFrames(const Bytes& payload) {
    std::vector<std::array<std::uint8_t, 8>> frames;
    std::array<std::uint8_t, 8> frame{};
    if (payload.size() <= 7) {
        frame.fill(0xAA);
        frame[0] = static_cast<std::uint8_t>(payload.size());
        std::copy(payload.begin(), payload.end(), frame.begin() + 1);
        frames.push_back(frame);
        return frames;
    }
    frame.fill(0xAA);
    frame[0] = static_cast<std::uint8_t>(0x10 | ((payload.size() >> 8) & 0x0F));
    frame[1] = static_cast<std::uint8_t>(payload.size() & 0xFF);
    std::copy(payload.begin(), payload.begin() + 6, frame.begin() + 2);
    frames.push_back(frame);
    size_t pos = 6;
    int sequence = 1;
    while (pos < payload.size()) {
        frame.fill(0xAA);
        frame[0] = static_cast<std::uint8_t>(0x20 | (sequence & 0x0F));
        const size_t n = std::min<size_t>(7, payload.size() - pos);
        std::copy(payload.begin() + static_cast<std::ptrdiff_t>(pos),
                  payload.begin() + static_cast<std::ptrdiff_t>(pos + n), frame.begin() + 1);
        frames.push_back(frame);
        pos += n;
        ++sequence;
    }
    return frames;
}

std::string protocolDescription(int protocol) {
    switch (protocol) {
        case 1: return "SAE J1850 PWM";
        case 2: return "SAE J1850 VPW";
        case 3: return "ISO 9141-2";
        case 4: return "ISO 14230-4 (KWP 5BAUD)";
        case 5: return "ISO 14230-4 (KWP FAST)";
        case 6: return "ISO 15765-4 (CAN 11/500)";
        case 7: return "ISO 15765-4 (CAN 29/500)";
        case 8: return "ISO 15765-4 (CAN 11/250)";
        case 9: return "ISO 15765-4 (CAN 29/250)";
        case 10: return "SAE J1939 (CAN 29/250)";
        case 11: return "USER1 (CAN 11/125)";
        case 12: return "USER2 (CAN 11/50)";
        default: return "AUTO";
    }
}

// Engine PIDs in mode 01, without the range PIDs 00, 20, 40 ...
const std::vector<int> kEnginePids = {
    0x01, 0x03, 0x04, 0x05, 0x06, 0x07, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x13, 0x14,
    0x15, 0x1C, 0x1F, 0x21, 0x23, 0x2F, 0x30, 0x31, 0x33, 0x3C, 0x41, 0x42, 0x43, 0x44, 0x45,
    0x46, 0x47, 0x49, 0x4A, 0x4C, 0x4D, 0x4E, 0x51, 0x5C, 0x5E, 0x61, 0x62, 0x63, 0xA6};
const std::vector<int> kGearboxPids = {0x01, 0xA4};
const std::vector<int> kFreezeFramePids = {0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x0B,
                                           0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11};
const std::vector<int> kEngineMids = {0x01, 0x02, 0x21, 0x3C, 0x41,
                                      0xA1, 0xA2, 0xA3, 0xA4, 0xA5};
const std::vector<int> kEngineInfo = {0x02, 0x04, 0x06, 0x08, 0x0A};
const std::vector<int> kGearboxInfo = {0x04, 0x06, 0x0A};

// Run time since clearing at which a monitor completes again (s).
constexpr double kHeaterDone = 90.0;
constexpr double kOxygenDone = 300.0;
constexpr double kCatalystDone = 420.0;
constexpr double kEgrDone = 540.0;
// When the faults come back after clearing (engine run time since the clear).
constexpr double kMisfireBack = 180.0;
constexpr double kLeanPending = 240.0;
constexpr double kLeanConfirmed = 600.0;
constexpr double kCatalystConfirmed = 900.0;
constexpr double kNever = 1e18;

struct Reply {
    int id;
    Bytes payload;
};

}  // namespace

// ---------------------------------------------------------------------------
// The adapter and the two control units
// ---------------------------------------------------------------------------

struct DemoTransport::Impl {
    DemoOptions options;
    Profile profile;
    SteadyClock::time_point origin = SteadyClock::now();

    struct Chunk {
        SteadyClock::time_point readyAt;
        std::string text;
    };
    std::deque<Chunk> output;
    std::string inputLine;
    std::string lastCommand;
    bool discardLine = false;

    // Adapter settings, as after ATZ.
    bool echo = true, linefeeds = false, spaces = true, headers = false, autoFormat = true,
         showDlc = false;
    int protocol = 0;          // as set with ATSP / ATTP
    bool autoSearch = true;
    bool active = false;       // a protocol has been found and is open
    int header = 0x7DF;        // ATSH
    int receiveFilter = -1;    // ATCRA
    int canFilter = -1, canMask = 0x7FF;
    bool monitoring = false;
    Bytes lastFrame = Bytes(8, 0x00);
    int savedByte = 0x00;      // ATSD / ATRD, kept in RAM only

    // Fault memory state.
    bool cleared = false;
    double clearRun = 0.0;     // engine run time at the moment of clearing
    double clearKm = 0.0;

    struct Response {
        std::vector<std::string> lines;
        double delayMs = 2.0;   // simulated time until the answer is complete
        bool prompt = true;     // false while monitoring: no '>' until stopped

        Response() = default;
        Response(std::vector<std::string> text, double delay)
            : lines(std::move(text)), delayMs(delay) {}
    };

    double now() const {
        if (options.clock) return options.clock();
        return std::chrono::duration<double>(SteadyClock::now() - origin).count();
    }

    std::string lineEnd() const { return linefeeds ? "\r\n" : "\r"; }

    bool supports(int level) const { return profile.level >= level; }

    void emit(std::string text, double delayMs) {
        auto ready = SteadyClock::now() + std::chrono::microseconds(static_cast<long long>(
                                              delayMs * options.latencyScale * 1000.0));
        if (!output.empty() && output.back().readyAt > ready) ready = output.back().readyAt;
        output.push_back({ready, std::move(text)});
    }

    void resetSettings() {
        echo = true;
        linefeeds = false;
        spaces = true;
        headers = false;
        autoFormat = true;
        showDlc = false;
        protocol = 0;
        autoSearch = true;
        active = false;
        header = 0x7DF;
        receiveFilter = -1;
        canFilter = -1;
        canMask = 0x7FF;
        monitoring = false;
    }

    // ---- fault state over time ------------------------------------------

    double runSinceClear(double t) const {
        if (!cleared) return kNever;
        return std::max(0.0, engineRunTime(t, options.engineStartSeconds) - clearRun);
    }

    std::vector<const char*> storedCodes(double t) const {
        if (!cleared) return {"P0171", "P0420"};
        const double r = runSinceClear(t);
        std::vector<const char*> codes;
        if (r >= kLeanConfirmed) codes.push_back("P0171");
        if (r >= kCatalystConfirmed) codes.push_back("P0420");
        return codes;
    }

    std::vector<const char*> pendingCodes(double t) const {
        if (!cleared) return {"P0300"};
        const double r = runSinceClear(t);
        std::vector<const char*> codes;
        if (r >= kMisfireBack) codes.push_back("P0300");
        if (r >= kLeanPending) codes.push_back("P0171");
        if (r >= kCatalystDone) codes.push_back("P0420");
        return codes;
    }

    // Permanent codes survive mode 04 on purpose: only a passed monitor
    // removes them, which is the whole point of mode 0A.
    std::vector<const char*> permanentCodes(double) const { return {"P0171", "P0420"}; }

    // The freeze frame belongs to the first fault after clearing. Before the
    // demo is cleared it shows an older moment from a warm cruise.
    bool freezeFrame(double t, Sample& sample, const char*& code) const {
        const double start = options.engineStartSeconds;
        if (!cleared) {
            sample = simulate(start + kCrankSeconds + 1300.0, start);
            sample.stft = 7.0;
            code = "P0171";
            return true;
        }
        if (runSinceClear(t) < kMisfireBack) return false;
        sample = simulate(start + kCrankSeconds + clearRun + kMisfireBack, start);
        code = "P0300";
        return true;
    }

    // Readiness: supported monitors in byte C, incomplete ones in byte D.
    std::uint8_t incompleteMonitors(double r) const {
        std::uint8_t d = 0x04;   // evaporative system: needs a cold soak, never done here
        if (r < kCatalystDone) d |= 0x01;
        if (r < kOxygenDone) d |= 0x20;
        if (r < kHeaterDone) d |= 0x40;
        if (r < kEgrDone) d |= 0x80;
        return d;
    }

    std::uint8_t readinessD(double t) const {
        if (!cleared) return 0x84;   // EVAP and EGR/VVT not yet completed
        return incompleteMonitors(runSinceClear(t));
    }

    bool monitorComplete(int mid, double t) const {
        const double r = runSinceClear(t);
        switch (mid) {
            case 0x01:
            case 0x02: return r >= kOxygenDone;
            case 0x21: return r >= kCatalystDone;
            case 0x3C: return false;
            case 0x41: return r >= kHeaterDone;
            default: return true;
        }
    }

    // ---- engine control unit ---------------------------------------------

    std::optional<Bytes> enginePid(int pid, double t) const {
        const double start = options.engineStartSeconds;
        const Sample s = simulate(t, start);
        const double run = s.runTime;
        const double r = runSinceClear(t);
        Bytes d;
        switch (pid) {
            case 0x01: {
                const auto stored = storedCodes(t);
                const int mil = stored.empty() ? 0 : 0x80;
                d = {static_cast<std::uint8_t>(mil | static_cast<int>(stored.size())), 0x07, 0xE5,
                     readinessD(t)};
                break;
            }
            case 0x03: d = {static_cast<std::uint8_t>(s.fuelStatus), 0x00}; break;
            case 0x04: d = {percent(s.load)}; break;
            case 0x05: d = {u8(s.coolant + 40.0)}; break;
            case 0x06: d = {trimByte(s.stft)}; break;
            case 0x07: d = {trimByte(s.ltft)}; break;
            case 0x0B: d = {u8(s.map)}; break;
            case 0x0C: put16(d, s.rpm * 4.0); break;
            case 0x0D: d = {u8(s.speed)}; break;
            case 0x0E: d = {u8((s.timing + 64.0) * 2.0)}; break;
            case 0x0F: d = {u8(s.iat + 40.0)}; break;
            case 0x10: put16(d, s.maf * 100.0); break;
            case 0x11: d = {percent(s.throttleAbs)}; break;
            case 0x13: d = {0x03}; break;   // bank 1, sensors 1 and 2
            case 0x14:
                d = {u8(s.o2up * 200.0), s.fuelStatus == 0x02 ? trimByte(s.stft) : std::uint8_t{0xFF}};
                break;
            case 0x15: d = {u8(s.o2down * 200.0), 0xFF}; break;
            case 0x1C: d = {0x06}; break;   // EOBD
            case 0x1F: put16(d, run); break;
            case 0x21: {
                double km = 0.0;
                if (!cleared) {
                    km = 312.0 + s.distanceKm;
                } else if (r >= kLeanConfirmed) {
                    const Sample on = simulate(start + kCrankSeconds + clearRun + kLeanConfirmed, start);
                    km = s.distanceKm - on.distanceKm;
                }
                put16(d, km);
                break;
            }
            case 0x23: put16(d, s.rail / 10.0); break;
            case 0x2F: d = {percent(s.fuelLevel)}; break;
            case 0x30: {
                const double warmAt = warmUpRunTime();
                const bool warmedNow = run >= warmAt;
                int count = cleared ? 0 : 36;
                if (warmedNow && (!cleared || clearRun < warmAt)) ++count;
                d = {static_cast<std::uint8_t>(count)};
                break;
            }
            case 0x31: put16(d, cleared ? s.distanceKm - clearKm : 1842.0 + s.distanceKm); break;
            case 0x33: d = {u8(kBaro)}; break;
            case 0x3C: put16(d, (s.catTemp + 40.0) * 10.0); break;
            case 0x41: {
                // This drive cycle: the same monitors, counted from engine start.
                const std::uint8_t done = s.phase == Phase::Off || s.phase == Phase::Crank
                                              ? 0xE1
                                              : incompleteMonitors(run);
                d = {0x00, 0x07, 0xE1, static_cast<std::uint8_t>(done & 0xE1)};
                break;
            }
            case 0x42: put16(d, (s.battery - 0.25) * 1000.0); break;
            case 0x43: put16(d, s.load * 0.95 * 255.0 / 100.0); break;
            case 0x44: put16(d, s.lambda * 32768.0); break;
            case 0x45: d = {percent(s.throttleRel)}; break;
            case 0x46: d = {u8(kAmbient + 40.0)}; break;
            case 0x47: d = {percent(s.throttleAbs + 1.2)}; break;
            case 0x49: d = {percent(s.pedalD)}; break;
            case 0x4A: d = {percent(s.pedalE)}; break;
            case 0x4C: d = {percent(s.throttleCmd)}; break;
            case 0x4D: {
                double minutes = 0.0;
                if (!cleared) minutes = 1270.0 + run / 60.0;
                else if (r >= kLeanConfirmed) minutes = (r - kLeanConfirmed) / 60.0;
                put16(d, std::floor(minutes));
                break;
            }
            case 0x4E: put16(d, std::floor(cleared ? r / 60.0 : 2580.0 + run / 60.0)); break;
            case 0x51: d = {0x01}; break;   // petrol
            case 0x5C: d = {u8(s.oil + 40.0)}; break;
            case 0x5E: put16(d, s.fuelRate * 20.0); break;
            case 0x61: d = {u8(s.torqueDemand + 125.0)}; break;
            case 0x62: d = {u8(s.torqueActual + 125.0)}; break;
            case 0x63: put16(d, 240.0); break;   // reference torque, Nm
            case 0xA6: put32(d, s.odometer * 10.0); break;
            default: return std::nullopt;
        }
        return d;
    }

    // The same PIDs from a frozen moment, for mode 02.
    static std::optional<Bytes> freezePid(int pid, const Sample& s, const char* code) {
        Bytes d;
        switch (pid) {
            case 0x02: putDtc(d, code); break;
            case 0x03: d = {static_cast<std::uint8_t>(s.fuelStatus), 0x00}; break;
            case 0x04: d = {percent(s.load)}; break;
            case 0x05: d = {u8(s.coolant + 40.0)}; break;
            case 0x06: d = {trimByte(s.stft)}; break;
            case 0x07: d = {trimByte(s.ltft)}; break;
            case 0x0B: d = {u8(s.map)}; break;
            case 0x0C: put16(d, s.rpm * 4.0); break;
            case 0x0D: d = {u8(s.speed)}; break;
            case 0x0E: d = {u8((s.timing + 64.0) * 2.0)}; break;
            case 0x0F: d = {u8(s.iat + 40.0)}; break;
            case 0x10: put16(d, s.maf * 100.0); break;
            case 0x11: d = {percent(s.throttleAbs)}; break;
            default: return std::nullopt;
        }
        return d;
    }

    // Mode 06 records: MID, TID, unit and scaling ID, value, minimum, maximum.
    void monitorRecords(int mid, double t, Bytes& out) const {
        const bool done = monitorComplete(mid, t);
        auto record = [&](int tid, int uas, int value, int low, int high) {
            out.push_back(static_cast<std::uint8_t>(mid));
            out.push_back(static_cast<std::uint8_t>(tid));
            out.push_back(static_cast<std::uint8_t>(uas));
            // A test that has not run since clearing reports zeros.
            if (!done) value = low = high = 0;
            put16(out, value);
            put16(out, low);
            put16(out, high);
        };
        const Sample s = simulate(t, options.engineStartSeconds);
        const double r = runSinceClear(t);
        // Misfire counters: cylinder 3 misfires now and then (P0300).
        const double cycleRun = std::min(s.runTime, r);
        const int current[4] = {static_cast<int>(cycleRun * 0.002), 0,
                                static_cast<int>(cycleRun * 0.06),
                                static_cast<int>(cycleRun * 0.004)};
        const int average[4] = {cleared ? std::min(1, static_cast<int>(r / 300.0)) : 1, 0,
                                cleared ? std::min(41, static_cast<int>(r / 15.0)) : 41,
                                cleared ? std::min(2, static_cast<int>(r / 200.0)) : 2};
        switch (mid) {
            case 0x01:   // O2 sensor bank 1 sensor 1: switch times in ms
                record(0x05, 0x10, 68, 0, 150);
                record(0x06, 0x10, 74, 0, 150);
                break;
            case 0x02:   // O2 sensor bank 1 sensor 2: min and max voltage in mV
                record(0x07, 0x0B, 84, 0, 400);
                record(0x08, 0x0B, 812, 600, 1100);
                break;
            case 0x21:   // catalyst bank 1: oxygen storage ratio, fails (P0420)
                record(0x80, 0x20, 210, 0, 154);
                break;
            case 0x3C:   // EVAP 0.020 inch leak: never completed
                record(0x80, 0x01, 0, 0, 0);
                break;
            case 0x41:   // O2 heater bank 1 sensor 1: warm-up time in ms
                record(0x80, 0x10, 14200, 0, 40000);
                break;
            case 0xA1: {
                int sumAverage = 0, sumCurrent = 0;
                for (int i = 0; i < 4; ++i) {
                    sumAverage += average[i];
                    sumCurrent += current[i];
                }
                record(0x0B, 0x24, sumAverage, 0, 0xFFFF);
                record(0x0C, 0x24, sumCurrent, 0, 0xFFFF);
                break;
            }
            case 0xA2:
            case 0xA3:
            case 0xA4:
            case 0xA5: {
                const int cylinder = mid - 0xA2;
                record(0x0B, 0x24, average[cylinder], 0, 0xFFFF);
                record(0x0C, 0x24, current[cylinder], 0, 0xFFFF);
                break;
            }
            default: break;
        }
    }

    // Copies `length` characters (embedded zero bytes included) and pads
    // with zero bytes up to `field`.
    static void putText(Bytes& out, const char* text, size_t length, size_t field) {
        for (size_t i = 0; i < length; ++i) out.push_back(static_cast<std::uint8_t>(text[i]));
        for (size_t i = length; i < field; ++i) out.push_back(0x00);
    }

    static void putPadded(Bytes& out, const char* text, size_t length) {
        size_t i = 0;
        for (; text[i] && i < length; ++i) out.push_back(static_cast<std::uint8_t>(text[i]));
        for (; i < length; ++i) out.push_back(0x00);
    }

    static std::optional<Bytes> codeList(int sid, const std::vector<const char*>& codes) {
        Bytes out = {static_cast<std::uint8_t>(sid + 0x40), static_cast<std::uint8_t>(codes.size())};
        for (const char* code : codes) putDtc(out, code);
        return out;
    }

    static std::optional<Bytes> rangeOrPids(const Bytes& request, const std::vector<int>& ids,
                                            const std::function<std::optional<Bytes>(int)>& value) {
        Bytes out = {0x41};
        for (size_t i = 1; i < request.size(); ++i) {
            const int pid = request[i];
            if (pid % 0x20 == 0) {
                if (!rangeReachable(ids, pid)) continue;
                const Bytes mask = supportMask(ids, pid);
                out.push_back(static_cast<std::uint8_t>(pid));
                out.insert(out.end(), mask.begin(), mask.end());
                continue;
            }
            if (std::find(ids.begin(), ids.end(), pid) == ids.end()) continue;
            const auto data = value(pid);
            if (!data) continue;
            out.push_back(static_cast<std::uint8_t>(pid));
            out.insert(out.end(), data->begin(), data->end());
        }
        if (out.size() == 1) return std::nullopt;
        return out;
    }

    std::optional<Bytes> engine(const Bytes& request, bool physical, double t) {
        const int sid = request[0];
        switch (sid) {
            case 0x01:
                if (request.size() < 2) return std::nullopt;
                return rangeOrPids(request, kEnginePids, [&](int pid) { return enginePid(pid, t); });
            case 0x02: {
                if (request.size() < 2) return std::nullopt;
                const int pid = request[1];
                const int frame = request.size() > 2 ? request[2] : 0;
                if (frame != 0) return std::nullopt;
                Sample frozen;
                const char* code = nullptr;
                const bool present = freezeFrame(t, frozen, code);
                Bytes out = {0x42, static_cast<std::uint8_t>(pid), 0x00};
                if (!present) {
                    if (pid != 0x02) return std::nullopt;
                    out.push_back(0x00);
                    out.push_back(0x00);
                    return out;
                }
                if (pid % 0x20 == 0) {
                    if (!rangeReachable(kFreezeFramePids, pid)) return std::nullopt;
                    const Bytes mask = supportMask(kFreezeFramePids, pid);
                    out.insert(out.end(), mask.begin(), mask.end());
                    return out;
                }
                const auto data = freezePid(pid, frozen, code);
                if (!data) return std::nullopt;
                out.insert(out.end(), data->begin(), data->end());
                return out;
            }
            case 0x03: return codeList(0x03, storedCodes(t));
            case 0x07: return codeList(0x07, pendingCodes(t));
            case 0x0A: return codeList(0x0A, permanentCodes(t));
            case 0x04: {
                cleared = true;
                clearRun = engineRunTime(t, options.engineStartSeconds);
                clearKm = simulate(t, options.engineStartSeconds).distanceKm;
                return Bytes{0x44};
            }
            case 0x06: {
                if (request.size() != 2) return std::nullopt;
                const int mid = request[1];
                Bytes out = {0x46};
                if (mid % 0x20 == 0) {
                    if (!rangeReachable(kEngineMids, mid)) return std::nullopt;
                    out.push_back(static_cast<std::uint8_t>(mid));
                    const Bytes mask = supportMask(kEngineMids, mid);
                    out.insert(out.end(), mask.begin(), mask.end());
                    return out;
                }
                if (std::find(kEngineMids.begin(), kEngineMids.end(), mid) == kEngineMids.end())
                    return std::nullopt;
                monitorRecords(mid, t, out);
                return out;
            }
            case 0x09: {
                if (request.size() != 2) return std::nullopt;
                const int item = request[1];
                Bytes out = {0x49, static_cast<std::uint8_t>(item)};
                switch (item) {
                    case 0x00: {
                        const Bytes mask = supportMask(kEngineInfo, 0);
                        out.insert(out.end(), mask.begin(), mask.end());
                        return out;
                    }
                    case 0x02:
                        out.push_back(0x01);
                        putText(out, kVin, 17, 17);
                        return out;
                    case 0x04:
                        out.push_back(0x01);
                        putPadded(out, kEngineCalibration, 16);
                        return out;
                    case 0x06:
                        out.push_back(0x01);
                        out.insert(out.end(), kEngineCvn, kEngineCvn + 4);
                        return out;
                    case 0x08: {
                        // In-use performance tracking, 16 counters for a petrol engine:
                        // OBD conditions, ignition cycles, then completions and
                        // conditions per monitor (catalyst 1/2, O2 1/2, EGR, AIR, EVAP).
                        const int counters[16] = {214, 612, 41, 198, 0, 0, 77, 198,
                                                  0,   0,   35, 198, 0, 0, 9,  198};
                        out.push_back(0x10);
                        for (int c : counters) put16(out, c);
                        return out;
                    }
                    case 0x0A:
                        out.push_back(0x01);
                        putText(out, kEngineName, sizeof(kEngineName) - 1, 20);
                        return out;
                    default: return std::nullopt;
                }
            }
            default:
                if (physical) return Bytes{0x7F, static_cast<std::uint8_t>(sid), 0x11};
                return std::nullopt;
        }
    }

    // ---- transmission control unit ---------------------------------------

    std::optional<Bytes> gearbox(const Bytes& request, bool physical, double t) {
        const int sid = request[0];
        switch (sid) {
            case 0x01: {
                if (request.size() < 2) return std::nullopt;
                const Sample s = simulate(t, options.engineStartSeconds);
                return rangeOrPids(request, kGearboxPids, [&](int pid) -> std::optional<Bytes> {
                    if (pid == 0x01) return Bytes{0x00, 0x04, 0x00, 0x00};
                    // PID A4: actual gear in the upper nibble, ratio in 1/1000.
                    const bool driving = s.phase != Phase::Off && s.phase != Phase::Crank;
                    Bytes d = {0x02, static_cast<std::uint8_t>(driving ? s.gear << 4 : 0)};
                    put16(d, driving ? s.gearRatio * 1000.0 : 0.0);
                    return d;
                });
            }
            case 0x03: return codeList(0x03, {});
            case 0x07: return codeList(0x07, {});
            case 0x0A: return codeList(0x0A, {});
            case 0x04: return Bytes{0x44};
            case 0x09: {
                if (request.size() != 2) return std::nullopt;
                const int item = request[1];
                Bytes out = {0x49, static_cast<std::uint8_t>(item)};
                switch (item) {
                    case 0x00: {
                        const Bytes mask = supportMask(kGearboxInfo, 0);
                        out.insert(out.end(), mask.begin(), mask.end());
                        return out;
                    }
                    case 0x04:
                        out.push_back(0x01);
                        putPadded(out, kGearboxCalibration, 16);
                        return out;
                    case 0x06:
                        out.push_back(0x01);
                        out.insert(out.end(), kGearboxCvn, kGearboxCvn + 4);
                        return out;
                    case 0x0A:
                        out.push_back(0x01);
                        putText(out, kGearboxName, sizeof(kGearboxName) - 1, 20);
                        return out;
                    default: return std::nullopt;
                }
            }
            default:
                if (physical) return Bytes{0x7F, static_cast<std::uint8_t>(sid), 0x11};
                return std::nullopt;
        }
    }

    // ---- output formatting -----------------------------------------------

    std::string byteText(const std::uint8_t* data, size_t count) const {
        std::string out;
        for (size_t i = 0; i < count; ++i) {
            out += hex2(data[i]);
            if (spaces) out += ' ';
        }
        return out;
    }

    void formatReply(const Reply& reply, std::vector<std::string>& lines) const {
        const auto frames = isoTpFrames(reply.payload);
        const std::string gap = spaces ? " " : "";
        if (headers || !autoFormat) {
            std::string prefix;
            if (headers) {
                prefix = hex3(reply.id) + gap;
                if (showDlc) prefix += "8" + gap;
            }
            for (const auto& frame : frames) lines.push_back(prefix + byteText(frame.data(), 8));
            return;
        }
        if (frames.size() == 1) {
            lines.push_back(byteText(reply.payload.data(), reply.payload.size()));
            return;
        }
        // Without headers the ELM327 announces the total length and numbers
        // the lines, dropping the PCI bytes. Automatic formatting also drops
        // the padding of the last frame. With headers on (above) every frame
        // is shown whole, PCI and padding included.
        lines.push_back(hex3(static_cast<int>(reply.payload.size())));
        size_t remaining = reply.payload.size();
        for (size_t i = 0; i < frames.size(); ++i) {
            std::string line(1, hexDigits[i & 0xF]);
            line += ":" + gap;
            const size_t offset = i == 0 ? 2 : 1;
            const size_t count = std::min(remaining, 8 - offset);
            line += byteText(frames[i].data() + offset, count);
            remaining -= count;
            lines.push_back(line);
        }
    }

    bool passesFilter(int id) const {
        if (receiveFilter >= 0) return id == receiveFilter;
        if (canFilter >= 0) return (id & canMask) == (canFilter & canMask);
        return true;
    }

    // ---- OBD requests ----------------------------------------------------

    Response obdRequest(std::string hex) {
        Response response;
        int countHint = 0;
        if (hex.size() % 2 == 1) {
            // A trailing single digit tells the adapter how many answers to
            // wait for (v1.3 and later), which saves the final timeout.
            if (!supports(kV13)) return unknown();
            countHint = hexValue(hex.back());
            if (countHint == 0) return unknown();
            hex.pop_back();
        }
        Bytes data;
        for (size_t i = 0; i + 1 < hex.size(); i += 2)
            data.push_back(static_cast<std::uint8_t>(hexValue(hex[i]) * 16 + hexValue(hex[i + 1])));
        if (!autoFormat) {
            // Without CAN auto formatting the first byte is the PCI byte.
            const int length = data.empty() ? 0 : data[0];
            if (length < 1 || length > 7 || static_cast<size_t>(length) + 1 > data.size())
                return {{"?"}, 2.0};
            data = Bytes(data.begin() + 1, data.begin() + 1 + length);
        }
        if (data.empty() || data.size() > 7) return {{"?"}, 2.0};

        // Protocol: the car only speaks protocol 6.
        if (!active) {
            if (autoSearch) {
                response.lines.push_back("SEARCHING...");
                response.delayMs += 80.0;
            } else if (protocol != 6) {
                switch (protocol) {
                    case 3:
                    case 4: return {{"BUS INIT: ...ERROR"}, 300.0};
                    case 5: return {{"BUS INIT: ERROR"}, 200.0};
                    case 8:
                    case 9:
                    case 11:
                    case 12: return {{"CAN ERROR"}, 150.0};
                    default: return {{"NO DATA"}, 120.0};
                }
            }
            active = true;
        }

        lastFrame.assign(8, 0x00);
        lastFrame[0] = static_cast<std::uint8_t>(data.size());
        std::copy(data.begin(), data.end(), lastFrame.begin() + 1);

        const double t = now();
        const bool functional = header == 0x7DF;
        std::vector<Reply> replies;
        if (functional || header == 0x7E0) {
            if (auto p = engine(data, !functional, t)) replies.push_back({0x7E8, *p});
        }
        if (options.transmissionEcu && (functional || header == 0x7E1)) {
            if (auto p = gearbox(data, !functional, t)) replies.push_back({0x7E9, *p});
        }
        replies.erase(std::remove_if(replies.begin(), replies.end(),
                                     [&](const Reply& r) { return !passesFilter(r.id); }),
                      replies.end());
        if (countHint > 0 && static_cast<int>(replies.size()) > countHint) replies.resize(countHint);

        if (replies.empty()) {
            response.lines.push_back("NO DATA");
            response.delayMs += 40.0;
            return response;
        }
        size_t frames = 0;
        for (const Reply& reply : replies) {
            formatReply(reply, response.lines);
            frames += isoTpFrames(reply.payload).size();
        }
        // Without a count hint the adapter waits a little for further answers.
        response.delayMs += 6.0 + static_cast<double>(frames) + (countHint > 0 ? 0.0 : 12.0);
        return response;
    }

    // ---- AT commands -----------------------------------------------------

    Response unknown() const { return {{profile.okToAll ? "OK" : "?"}, 2.0}; }

    Response ok(int level = kV10) const {
        if (!supports(level)) return unknown();
        return {{"OK"}, 2.0};
    }

    Response flag(bool& target, bool value, int level = kV10) {
        if (!supports(level)) return unknown();
        target = value;
        return {{"OK"}, 2.0};
    }

    std::string describeProtocol() const {
        const int shown = active ? 6 : protocol;
        if (!autoSearch) return protocolDescription(shown);
        if (shown == 0) return "AUTO";
        return "AUTO, " + protocolDescription(shown);
    }

    // ATSP and ATTP: the same here, since nothing is stored anyway.
    Response setProtocol(const std::string& arg) {
        std::string rest = arg;
        bool automatic = false;
        if (rest.size() == 2 && rest[0] == 'A') {
            automatic = true;
            rest = rest.substr(1);
        }
        if (rest == "00" && supports(kV14)) rest = "0";
        if (rest.size() != 1 || !isHex(rest[0]) || hexValue(rest[0]) > 12) return unknown();
        protocol = hexValue(rest[0]);
        autoSearch = automatic || protocol == 0;
        active = false;
        return {{"OK"}, 2.0};
    }

    Response programmableParameters() const {
        // Factory values of the parameters the ELM327 documents; everything
        // else is shown as FF (off), as on most adapters.
        auto value = [](int index) {
            switch (index) {
                case 0x03: return 0x32;
                case 0x04: return 0x01;
                case 0x06: return 0xF1;
                case 0x07: return 0x09;
                case 0x0A: return 0x0A;
                case 0x0C: return 0x68;
                case 0x0D: return 0x0D;
                case 0x0E: return 0x9A;
                case 0x10: return 0x0D;
                case 0x11: return 0x00;
                case 0x13: return 0x32;
                case 0x16: return 0x92;
                case 0x17: return 0x92;
                case 0x18: return 0x00;
                case 0x19: return 0x28;
                case 0x1A: return 0x0A;
                case 0x24: return 0x00;
                case 0x25: return 0x00;
                case 0x26: return 0x00;
                case 0x2A: return 0x38;
                case 0x2B: return 0x02;
                case 0x2C: return 0xE0;
                case 0x2D: return 0x04;
                case 0x2E: return 0x80;
                case 0x2F: return 0x0A;
                default: return 0xFF;
            }
        };
        Response r;
        for (int row = 0; row < 0x30; row += 4) {
            std::string line;
            for (int i = row; i < row + 4; ++i) {
                if (i > row) line += "  ";
                line += hex2(i) + ":" + hex2(value(i)) + " F";
            }
            r.lines.push_back(line);
        }
        r.delayMs = 8.0;
        return r;
    }

    Response atCommand(const std::string& c) {
        int value = 0;

        if (c == "Z" || c == "WS") {
            resetSettings();
            return {{"", "", profile.version}, c == "Z" ? 40.0 : 20.0};
        }
        if (c == "D") {
            const bool keepEcho = echo;
            resetSettings();
            echo = keepEcho;
            return {{"OK"}, 2.0};
        }
        if (c == "I") return {{profile.version}, 2.0};
        if (c == "@1") return {{profile.description}, 2.0};
        if (c == "@2") return supports(kV13) ? Response{{"?"}, 2.0} : unknown();
        if (c.rfind("@3", 0) == 0 && c.size() == 14) return ok(kV13);   // stored nowhere here
        if (c == "E0") return flag(echo, false);
        if (c == "E1") return flag(echo, true);
        if (c == "L0") return flag(linefeeds, false);
        if (c == "L1") return flag(linefeeds, true);
        if (c == "S0") return flag(spaces, false, kV13);
        if (c == "S1") return flag(spaces, true, kV13);
        if (c == "H0") return flag(headers, false);
        if (c == "H1") return flag(headers, true);
        if (c == "D0") return flag(showDlc, false, kV13);
        if (c == "D1") return flag(showDlc, true, kV13);
        if (c == "CAF0") return flag(autoFormat, false);
        if (c == "CAF1") return flag(autoFormat, true);
        if (c == "M0" || c == "M1" || c == "R0" || c == "R1" || c == "CFC0" || c == "CFC1" ||
            c == "AL" || c == "NL" || c == "BI" || c == "IB10" || c == "IB96")
            return ok();
        if (c == "V0" || c == "V1" || c == "JE" || c == "JS") return ok(kV13);
        if (c == "AT0" || c == "AT1" || c == "AT2" || c == "KW0" || c == "KW1") return ok(kV12);
        if (c == "FE") return ok(kV13a);
        if (c == "CSM0" || c == "CSM1" || c == "SS" || c == "LP" || c == "CEA" || c == "IB48" ||
            c == "JTM1" || c == "JTM5" || c == "CV0000")
            return ok(kV14);
        if (c == "JHF0" || c == "JHF1") return ok(kV14b);
        if (c == "FCSH" || c == "FCSD") return unknown();   // need arguments
        if (c == "FCSM0" || c == "FCSM1" || c == "FCSM2") return ok(kV11);
        if (c.rfind("FCSH", 0) == 0 && allHex(c.substr(4)) && (c.size() == 7 || c.size() == 12))
            return ok(kV11);
        if (c.rfind("FCSD", 0) == 0 && allHex(c.substr(4)) && c.size() >= 6 && c.size() <= 14 &&
            c.size() % 2 == 0)
            return ok(kV11);
        if (c == "AR") {
            if (!supports(kV12)) return unknown();
            receiveFilter = canFilter = -1;
            return {{"OK"}, 2.0};
        }
        if (c == "CRA") {
            if (!supports(kV14b)) return unknown();
            receiveFilter = canFilter = -1;
            return {{"OK"}, 2.0};
        }
        if (hexArg(c, "CRA", 3, &value)) {
            if (!supports(kV13)) return unknown();
            receiveFilter = value;
            return {{"OK"}, 2.0};
        }
        if (hexArg(c, "CRA", 8)) {
            if (!supports(kV13)) return unknown();
            receiveFilter = 0x7FFFFFFF;   // a 29 bit address: nothing on this bus matches
            return {{"OK"}, 2.0};
        }
        if (hexArg(c, "CF", 3, &value)) {
            canFilter = value;
            return {{"OK"}, 2.0};
        }
        if (hexArg(c, "CM", 3, &value)) {
            canMask = value;
            if (canFilter < 0) canFilter = 0;
            return {{"OK"}, 2.0};
        }
        if (hexArg(c, "CF", 8) || hexArg(c, "CM", 8) || hexArg(c, "CP", 2) ||
            hexArg(c, "SW", 2) || hexArg(c, "SR", 2) || hexArg(c, "ST", 2))
            return ok();
        if (hexArg(c, "IIA", 2) || hexArg(c, "BRT", 2)) return ok(kV12);
        if (hexArg(c, "RA", 2)) return ok(kV13);
        if (hexArg(c, "TA", 2) || hexArg(c, "CEA", 2) || hexArg(c, "PB", 4)) return ok(kV14);
        if (hexArg(c, "CER", 2) || hexArg(c, "AMT", 2)) return ok(kV20);
        if (hexArg(c, "SD", 2, &value)) {
            if (!supports(kV14)) return unknown();
            savedByte = value;
            return {{"OK"}, 2.0};
        }
        if (c == "RD") return supports(kV14) ? Response{{hex2(savedByte)}, 2.0} : unknown();
        if (c.rfind("WM", 0) == 0 && allHex(c.substr(2)) && c.size() >= 8 && c.size() <= 14 &&
            c.size() % 2 == 0)
            return ok();
        if (c.rfind("CV", 0) == 0 && c.size() == 6 &&
            std::all_of(c.begin() + 2, c.end(), [](char ch) { return ch >= '0' && ch <= '9'; }))
            return ok();
        if (hexArg(c, "SH", 3, &value) || hexArg(c, "SH", 6, &value)) {
            // With xxyyzz only the low 11 bits matter on an 11 bit bus.
            header = value & 0x7FF;
            return {{"OK"}, 2.0};
        }
        if (hexArg(c, "SH", 8, &value)) {
            if (!supports(kV20)) return unknown();
            header = -1;   // 29 bit: nobody answers on this car
            return {{"OK"}, 2.0};
        }
        if (c.rfind("SP", 0) == 0 || c.rfind("TP", 0) == 0) return setProtocol(c.substr(2));
        if (c == "DP") return {{describeProtocol()}, 2.0};
        if (c == "DPN") {
            const int shown = active ? 6 : protocol;
            return {{std::string(autoSearch ? "A" : "") + hexDigits[shown & 0xF]}, 2.0};
        }
        if (c == "PC") {
            active = false;
            return {{"OK"}, 2.0};
        }
        if (c == "RV") {
            // Measured at pin 16, a little below the battery itself.
            const Sample s = simulate(now(), options.engineStartSeconds);
            char buffer[16];
            std::snprintf(buffer, sizeof(buffer), "%.1fV", s.battery - 0.05);
            return {{buffer}, 3.0};
        }
        if (c == "IGN") return supports(kV14) ? Response{{"ON"}, 2.0} : unknown();
        if (c == "AMC") return supports(kV20) ? Response{{"00"}, 2.0} : unknown();
        if (c == "PPS") return supports(kV11) ? programmableParameters() : unknown();
        if (c.rfind("PP", 0) == 0 && c.size() >= 6 && isHex(c[2]) && isHex(c[3])) {
            // Accepted, but the demo has no memory that outlives it anyway.
            const std::string rest = c.substr(4);
            if (rest == "ON" || rest == "OFF" || (rest.size() == 4 && rest.rfind("SV", 0) == 0 &&
                                                  isHex(rest[2]) && isHex(rest[3])))
                return ok(kV11);
            return unknown();
        }
        if (c == "CS") return {{"T:00 R:00 "}, 2.0};
        if (c == "BD") {
            std::string line = hex2(static_cast<int>(lastFrame.size()));
            for (int i = 0; i < 12; ++i) {
                line += spaces ? " " : "";
                line += hex2(i < static_cast<int>(lastFrame.size()) ? lastFrame[i] : 0);
            }
            return {{line}, 2.0};
        }
        if (c == "RTR") return supports(kV13) ? Response{{"NO DATA"}, 40.0} : unknown();
        if (c == "MA" || hexArg(c, "MR", 2) || hexArg(c, "MT", 2)) {
            // The bus behind the gateway stays silent: monitoring shows
            // nothing until any character stops it.
            Response r;
            r.prompt = false;
            monitoring = true;
            return r;
        }
        return unknown();
    }

    // STN (OBDLink) commands; only the read-only identification is modelled.
    Response stnCommand(const std::string& c) const {
        if (c == "STI") return {{profile.stnId}, 2.0};
        if (c == "STDI") return {{profile.stnDevice}, 2.0};
        return {{"?"}, 2.0};
    }

    Response execute(const std::string& command) {
        if (command.rfind("AT", 0) == 0) return atCommand(command.substr(2));
        if (profile.stnId && command.rfind("ST", 0) == 0) return stnCommand(command);
        if (allHex(command)) return obdRequest(command);
        return {{"?"}, 2.0};
    }

    void handleLine(const std::string& raw) {
        std::string command;
        for (char ch : raw) {
            if (ch == ' ' || ch == '\t') continue;
            command += static_cast<char>(ch >= 'a' && ch <= 'z' ? ch - 'a' + 'A' : ch);
        }
        const std::string end = lineEnd();
        // The echo goes out before the command changes anything (ATE0 is
        // still echoed).
        if (echo) emit(raw + end, 0.0);
        if (command.empty()) {
            // A bare carriage return repeats the last command.
            if (lastCommand.empty()) {
                emit(">", 1.0);
                return;
            }
            command = lastCommand;
        }
        lastCommand = command;

        const Response r = execute(command);
        if (!r.prompt) return;
        std::string text;
        for (const std::string& line : r.lines) text += line + end;
        text += end + ">";
        emit(std::move(text), r.delayMs);
    }
};

DemoTransport::DemoTransport(DemoOptions options) : impl_(std::make_unique<Impl>()) {
    impl_->options = std::move(options);
    impl_->profile = profileFor(impl_->options.adapter);
    // Power-on greeting; Elm327::connect clears it away like a real one.
    impl_->emit(std::string(impl_->profile.version) + "\r\r>", 0.0);
}

DemoTransport::~DemoTransport() = default;

bool DemoTransport::write(const std::string& data) {
    Impl& d = *impl_;
    for (char ch : data) {
        if (d.monitoring) {
            // Any character ends monitoring; the rest of that line is lost,
            // which is why programs send a lone carriage return first.
            d.monitoring = false;
            d.emit("STOPPED" + d.lineEnd() + d.lineEnd() + ">", 1.0);
            d.discardLine = ch != '\r';
            continue;
        }
        if (d.discardLine) {
            if (ch == '\r') d.discardLine = false;
            continue;
        }
        if (ch == '\r') {
            const std::string line = d.inputLine;
            d.inputLine.clear();
            d.handleLine(line);
        } else if (ch != '\n' && ch != '\0') {
            if (d.inputLine.size() < 64) d.inputLine += ch;
        }
    }
    return true;
}

std::string DemoTransport::read(int timeoutMs) {
    Impl& d = *impl_;
    const auto deadline = SteadyClock::now() + std::chrono::milliseconds(std::max(0, timeoutMs));
    for (;;) {
        std::string result;
        const auto now = SteadyClock::now();
        while (!d.output.empty() && d.output.front().readyAt <= now) {
            result += d.output.front().text;
            d.output.pop_front();
        }
        if (!result.empty()) return result;
        if (now >= deadline) return {};
        auto wake = deadline;
        if (!d.output.empty() && d.output.front().readyAt < wake) wake = d.output.front().readyAt;
        std::this_thread::sleep_until(wake);
    }
}

void DemoTransport::flushInput() {
    // Only what the adapter has already sent is thrown away, like a serial
    // port buffer; answers still on their way arrive later.
    Impl& d = *impl_;
    const auto now = SteadyClock::now();
    while (!d.output.empty() && d.output.front().readyAt <= now) d.output.pop_front();
}

std::string DemoTransport::name() const {
    return L("Demo-Fahrzeug (eingebaut)", "Demo vehicle (built-in)");
}

const char* DemoTransport::vin() { return kVin; }

}  // namespace caruspex
