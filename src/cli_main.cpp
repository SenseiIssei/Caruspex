// Console tool for the core. The actual application is caruspex.exe.
//
//   caruspex_cli ports
//   caruspex_cli kklcheck --port COM3
//   caruspex_cli read --tcp 127.0.0.1:35000
//   caruspex_cli read --port COM5 --baud 38400
//   caruspex_cli health --demo --json
//   caruspex_cli ecus --tcp 127.0.0.1:35000   (per control unit, all live values)
//   caruspex_cli uds --demo --ids 760,737      (UDS: identification and faults per ECU)
//   caruspex_cli read --port COM5 --record car.jsonl   (record the session)
//   caruspex_cli read --replay car.jsonl               (play it back without a car)
//   caruspex_cli custom --port COM5 --pack hyundai-kona-niro-ev   (maker values)
//
// `caruspex_cli help` lists every command and option.

#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <memory>
#include <set>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

#include "core/AdapterProbe.h"
#include "core/CustomPids.h"
#include "core/DemoTransport.h"
#include "core/Elm327.h"
#include "core/JsonWriter.h"
#include "core/KLineStream.h"
#include "core/Lang.h"
#include "core/Obd2.h"
#include "core/Recording.h"
#include "core/Timing.h"
#include "core/Transport.h"
#include "core/Uds.h"
#include "core/analysis/HealthCheck.h"
#include "data/DtcDatabase.h"
#include "data/PidPacks.h"
#include "data/Pids.h"
#include "data/VehicleCatalog.h"
#include "data/Vin.h"

using namespace caruspex;

namespace {

const char* const kUsage =
    "Usage: caruspex_cli <command> [options]\n"
    "\n"
    "Commands:\n"
    "  ports       list the serial ports\n"
    "  kklcheck    self-test of a KKL cable (needs --port, not plugged into the car)\n"
    "  read        warning lamp, readiness and fault codes\n"
    "  clear       clear the fault memory\n"
    "  live        one snapshot of the usual live values\n"
    "  info        VIN and the first freeze frame\n"
    "  vin         VIN, what it says, and the matching car from the catalog\n"
    "  freeze      every freeze frame, one per stored fault\n"
    "  health      health check: faults, readiness, mode 06 and a live series\n"
    "  probe       adapter test: what the ELM327 really is and can do\n"
    "  ecus        everything per control unit (headers on)\n"
    "  uds         UDS on CAN: identification and fault memory of every control unit\n"
    "  custom      maker and custom values (mode 22 / 21) of a pack or a Torque CSV, once\n"
    "  version     the version of this program\n"
    "  help        this text\n"
    "\n"
    "Connection (one of them):\n"
    "  --demo              built-in demo car, warm and on the road\n"
    "  --tcp HOST[:PORT]   Wi-Fi adapter, port 35000 if left out\n"
    "  --port NAME         serial port, COM3 if left out (/dev/ttyUSB0 and so on)\n"
    "  --baud N            baud rate of the serial port, 38400 if left out\n"
    "  --replay FILE       play back a recording instead of talking to an adapter\n"
    "\n"
    "Options:\n"
    "  --json              machine-readable output for read, vin, health, probe, freeze, uds,\n"
    "                      custom\n"
    "  --seconds N         length of the live series of health, 15 if left out\n"
    "  --ids A,B,...       request ids for uds in hex (response = request + 8; 29-bit\n"
    "                      ids like 18DA28F1 answer on 18DAF128); the demo car's six\n"
    "                      control units if left out\n"
    "  --record FILE       record the whole session with the adapter into FILE\n"
    "                      (JSON Lines, replayable with --replay)\n"
    "  --fast-timing       aggressive adaptive timing (ATAT2), back to ATAT1 if the\n"
    "                      adapter refuses it or answers go missing\n"
    "  --pack NAME         built-in pack for custom (the demo car's pack with --demo if\n"
    "                      left out); an unknown name lists the packs\n"
    "  --csv FILE          Torque CSV for custom (Name,ShortName,ModeAndPID,Equation,\n"
    "                      Min Value,Max Value,Units,Header), instead of a pack\n"
    "  -v                  log every command sent to the adapter\n";

void printPorts() {
    const std::vector<PortInfo> ports = enumeratePorts();
    if (ports.empty()) {
        std::printf("No COM port found.\n");
        return;
    }
    for (const PortInfo& p : ports) {
        std::printf("  %-8s %s\n", p.device.c_str(), p.description.c_str());
        if (p.vid) std::printf("           USB VID_%04X PID_%04X\n", p.vid, p.pid);
        if (!p.chipGuess.empty()) std::printf("           Chip: %s\n", p.chipGuess.c_str());
    }
}


// Checks a KKL cable without it being plugged into the car. It tests what the
// 5-baud wake-up address needs: toggling the break line and changing the baud
// rate on the fly. It also measures how accurately the bit times are kept.
int kklCheck(const std::string& port) {
    std::printf("KKL self-test on %s\n", port.c_str());
    std::printf("The cable does NOT need to be plugged into the car for this.\n\n");

    Serial line;
    std::string error;
    if (!line.open(port, 9600, error)) {
        std::printf("[fail]  Cannot open the port: %s\n", error.c_str());
        return 2;
    }
    std::printf("[ok]    Port opened at 9600 baud\n");

    // 1) Change the baud rate on the fly.
    bool baudOk = true;
    for (int baud : {9600, 10400, 4800, 9600}) {
        if (!line.changeBaud(baud)) {
            std::printf("[fail]  Baud rate %d is not accepted\n", baud);
            baudOk = false;
        }
    }
    if (baudOk) std::printf("[ok]    Baud rate can be switched on the fly\n");

    // 2) Toggle the break line. This is what clocks out the wake-up address.
    if (!line.setBreak(true) || !line.setBreak(false)) {
        std::printf("[fail]  Cannot toggle the break line. Without it the 5-baud "
                    "wake-up address does not work.\n");
        line.close();
        return 3;
    }
    std::printf("[ok]    Break line can be toggled\n");

    // 3) Measure the bit times. 5 baud means 200 ms per bit. Windows likes to
    //    tends to oversleep, and the control unit cannot follow overly long bits.
    std::printf("\nMeasuring bit times (10 bits as in a real "
                "wake-up address) ...\n");
    // Clocked exactly like the real wake-up address: fixed target times,
    // counted from the start, so that nothing accumulates.
    double worst = 200.0;
    const auto begin = std::chrono::steady_clock::now();
    const std::uint64_t start = nowMs();
    double previous = 0.0;
    for (int bit = 0; bit < 10; ++bit) {
        line.setBreak(bit % 2 == 0);
        sleepUntilMs(start + static_cast<std::uint64_t>(bit + 1) * 200);
        const double elapsed = std::chrono::duration<double, std::milli>(
                                   std::chrono::steady_clock::now() - begin)
                                   .count();
        const double ms = elapsed - previous;
        previous = elapsed;
        std::printf("        Bit %2d: %.1f ms\n", bit + 1, ms);
        if (std::abs(ms - 200.0) > std::abs(worst - 200.0)) worst = ms;
    }
    // Stop before closing: closing a USB port takes a good hundred
    // milliseconds and would distort the measurement.
    const double total = previous;
    line.setBreak(false);
    line.close();

    const double average = total / 10.0;
    std::printf("        Average %.1f ms, largest deviation %.1f ms\n", average, worst);
    std::printf("        Total length %.0f ms (target 2000 ms)\n", total);

    if (std::abs(average - 200.0) <= 8.0) {
        std::printf("[ok]    Timing accurate enough for the wake-up address\n");
    } else {
        std::printf("[warn]  Timing deviates. The wake-up address may fail because of it.\n");
    }

    std::printf("\nResult: the cable can do everything the protocol needs.\n");
    std::printf("Whether the control unit answers only shows on the car with the ignition on.\n");
    return 0;
}

std::unique_ptr<ITransport> makeTransport(const std::string& tcp, const std::string& port,
                                          int baud, std::string& error) {
    if (!tcp.empty()) {
        const size_t colon = tcp.find(':');
        auto t = std::make_unique<TcpTransport>();
        const std::string host = tcp.substr(0, colon);
        const int p = colon == std::string::npos ? 35000 : std::stoi(tcp.substr(colon + 1));
        if (!t->open(host, p, error)) return nullptr;
        return t;
    }
    auto t = std::make_unique<SerialTransport>();
    if (!t->open(port, baud, error)) return nullptr;
    return t;
}

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

void printValue(const LiveValue& v) {
    if (!v.text.empty()) {
        std::printf("  %-40s %s\n", v.name.c_str(), v.text.c_str());
    } else {
        std::printf("  %-40s %10.2f %s\n", v.name.c_str(), v.value, v.unit.c_str());
    }
}

void jsonValue(JsonWriter& json, const LiveValue& v) {
    json.beginObject()
        .field("pid", v.pid)
        .field("key", v.key)
        .field("sub", v.sub)
        .field("name", v.name)
        .field("value", v.value)
        .field("unit", v.unit);
    if (!v.text.empty()) json.field("text", v.text);
    if (!v.ecuSource.empty()) json.field("ecu", v.ecu).field("ecuSource", v.ecuSource);
    json.endObject();
}

void jsonAdapter(JsonWriter& json, Elm327& elm) {
    json.key("adapter")
        .beginObject()
        .field("version", elm.version())
        .field("connection", elm.transportName())
        .field("protocol", elm.protocolText())
        .field("can", elm.isCan())
        .endObject();
}

// The brand behind the VIN, for manufacturer-specific code texts. Empty if
// the car gives no VIN or the VIN names no single brand.
struct VinContext {
    std::string vin;
    VinInfo info;
    CatalogMatch match;
    std::string brand;
};

VinContext readVinContext(Obd2& obd) {
    VinContext c;
    c.vin = obd.readVin();
    if (c.vin.empty()) return c;
    c.info = decodeVin(c.vin);
    c.match = catalog::matchVin(c.info);
    c.brand = !c.match.brand.empty() ? c.match.brand : c.info.brand;
    return c;
}

// The fault text, with the meaning for the brand where it differs.
std::string faultText(const std::string& code, const std::string& brand) {
    return lookupDtc(code, brand).text;
}

bool isStored(const Fault& f) { return f.kind == L("gespeichert", "stored"); }

void jsonFaults(JsonWriter& json, const std::vector<Fault>& faults, const std::string& brand) {
    json.key("faults").beginArray();
    for (const Fault& f : faults) {
        const DtcInfo info = lookupDtc(f.code, brand);
        json.beginObject()
            .field("code", f.code)
            .field("kind", f.kind)
            .field("text", info.text)
            .field("severity", severityLabel(info.severity))
            .field("system", info.systemName);
        if (!f.ecuSource.empty()) json.field("ecu", f.ecu).field("ecuSource", f.ecuSource);
        json.endObject();
    }
    json.endArray();
}

void jsonStatus(JsonWriter& json, const VehicleStatus& status) {
    json.key("status");
    if (!status.valid) {
        json.null();
        return;
    }
    json.beginObject()
        .field("milOn", status.milOn)
        .field("faultCount", status.faultCount)
        .field("diesel", status.compressionIgnition)
        .key("readiness")
        .beginArray();
    for (const ReadinessItem& r : status.readiness) {
        json.beginObject().field("name", r.name).field("ready", r.ready).endObject();
    }
    json.endArray().endObject();
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

int commandRead(Elm327& elm, Obd2& obd, bool asJson) {
    const VehicleStatus status = obd.readStatus();
    std::string err;
    const std::vector<Fault> faults = obd.readFaults(err);
    const VinContext vin = readVinContext(obd);

    if (asJson) {
        JsonWriter json;
        json.beginObject();
        jsonAdapter(json, elm);
        json.key("vin");
        if (vin.vin.empty()) json.null();
        else json.value(vin.vin);
        json.field("brand", vin.brand);
        jsonStatus(json, status);
        jsonFaults(json, faults, vin.brand);
        if (!err.empty()) json.field("error", err);
        json.endObject();
        std::fputs(json.text().c_str(), stdout);
        return 0;
    }

    if (!vin.vin.empty()) {
        std::printf("VIN: %s%s%s\n", vin.vin.c_str(), vin.brand.empty() ? "" : "  (",
                    vin.brand.empty() ? "" : (vin.brand + ")").c_str());
    }
    if (status.valid) {
        std::printf("MIL: %s   reported faults: %d\n", status.milOn ? "ON" : "off", status.faultCount);
        for (const ReadinessItem& r : status.readiness) {
            std::printf("   %-26s %s\n", r.name.c_str(), r.ready ? "ready" : "not ready");
        }
    }
    std::printf("\n");
    if (faults.empty()) {
        std::printf("No faults stored.\n");
    } else {
        std::printf("%zu fault(s) found:\n", faults.size());
        for (const Fault& f : faults) {
            std::printf("  %s [%s]%s%s\n      %s\n", f.code.c_str(), f.kind.c_str(),
                        f.ecu.empty() ? "" : " ", f.ecu.c_str(), faultText(f.code, vin.brand).c_str());
        }
    }
    if (!err.empty()) std::printf("%s\n", err.c_str());
    return 0;
}

int commandVin(Elm327& elm, Obd2& obd, bool asJson) {
    const VinContext c = readVinContext(obd);
    const VinInfo& info = c.info;
    const CatalogMatch& match = c.match;

    if (asJson) {
        JsonWriter json;
        json.beginObject();
        jsonAdapter(json, elm);
        json.key("vin");
        if (c.vin.empty()) {
            json.null();
            json.field("error", "The car does not report a VIN (mode 09, InfoType 02).");
            json.endObject();
            std::fputs(json.text().c_str(), stdout);
            return 4;
        }
        json.value(c.vin)
            .field("valid", info.valid)
            .field("wmi", info.wmi)
            .field("manufacturer", info.manufacturer)
            .field("brand", info.brand)
            .stringList("brandCandidates", info.brandCandidates)
            .field("country", info.country)
            .field("region", info.region)
            .key("modelYear");
        if (info.modelYear > 0) json.value(info.modelYear);
        else json.null();
        json.field("modelYearCertain", info.modelYearCertain).key("modelYears").beginArray();
        for (int year : info.modelYears) json.value(year);
        json.endArray()
            .field("checkDigitApplies", info.checkDigitApplies)
            .field("checkDigitOk", info.checkDigitOk)
            .field("serial", info.serial)
            .field("modelCode", info.modelCode)
            .stringList("problems", info.problems)
            .stringList("notes", info.notes)
            .key("catalog")
            .beginObject()
            .field("brand", match.brand)
            .field("confidence", match.confidence)
            .field("reason", match.reason)
            .key("year");
        if (match.year > 0) json.value(match.year);
        else json.null();
        json.key("generation");
        if (match.generation) {
            const VehicleGeneration& g = *match.generation;
            json.beginObject()
                .field("name", catalog::displayName(g))
                .field("brand", g.brand)
                .field("model", g.model)
                .field("generation", g.generation)
                .field("yearFrom", g.yearFrom)
                .key("yearTo");
            if (g.yearTo > 0) json.value(g.yearTo);
            else json.null();
            json.field("diagnosis", catalog::diagnosisText(g, match.year)).endObject();
        } else {
            json.null();
        }
        json.key("alternatives").beginArray();
        for (const VehicleGeneration* g : match.alternatives) json.value(catalog::displayName(*g));
        json.endArray().endObject().endObject();
        std::fputs(json.text().c_str(), stdout);
        return 0;
    }

    if (c.vin.empty()) {
        std::printf("The car does not report a VIN (mode 09, InfoType 02).\n");
        return 4;
    }
    std::printf("VIN          : %s%s\n", c.vin.c_str(), info.valid ? "" : "  (not a valid VIN)");
    std::printf("Manufacturer : %s\n", info.manufacturer.empty() ? "unknown" : info.manufacturer.c_str());
    if (!info.country.empty()) std::printf("Country      : %s, %s\n", info.country.c_str(), info.region.c_str());
    if (info.modelYear > 0) {
        std::printf("Model year   : %d%s\n", info.modelYear, info.modelYearCertain ? "" : " (not certain)");
    }
    if (info.checkDigitApplies) {
        std::printf("Check digit  : %s\n", info.checkDigitOk ? "correct" : "WRONG");
    }
    if (!info.serial.empty()) std::printf("Serial       : %s\n", info.serial.c_str());
    for (const std::string& p : info.problems) std::printf("Problem      : %s\n", p.c_str());
    for (const std::string& n : info.notes) std::printf("Note         : %s\n", n.c_str());
    std::printf("\n");
    if (match.generation) {
        std::printf("Catalog      : %s, %d %% sure\n", catalog::displayName(*match.generation).c_str(),
                    match.confidence);
        if (!match.reason.empty()) std::printf("               %s\n", match.reason.c_str());
        std::printf("Diagnosis    : %s\n",
                    catalog::diagnosisText(*match.generation, match.year).c_str());
        for (const VehicleGeneration* g : match.alternatives) {
            std::printf("Also possible: %s\n", catalog::displayName(*g).c_str());
        }
    } else if (!match.brand.empty()) {
        std::printf("Catalog      : %s, model not recognised\n", match.brand.c_str());
    } else {
        std::printf("Catalog      : no match\n");
    }
    return 0;
}

int commandFreeze(Elm327& elm, Obd2& obd, bool asJson) {
    const std::vector<FreezeFrame> frames = obd.readFreezeFrames();
    const std::string brand = frames.empty() ? std::string() : readVinContext(obd).brand;

    if (asJson) {
        JsonWriter json;
        json.beginObject();
        jsonAdapter(json, elm);
        json.field("brand", brand).key("frames").beginArray();
        for (const FreezeFrame& f : frames) {
            json.beginObject()
                .field("frame", f.frame)
                .field("dtc", f.dtc)
                .field("text", faultText(f.dtc, brand));
            if (!f.ecuSource.empty()) json.field("ecu", f.ecu).field("ecuSource", f.ecuSource);
            json.key("values").beginArray();
            for (const LiveValue& v : f.values) jsonValue(json, v);
            json.endArray().endObject();
        }
        json.endArray().endObject();
        std::fputs(json.text().c_str(), stdout);
        return 0;
    }

    if (frames.empty()) {
        std::printf("No freeze frame stored.\n");
        return 0;
    }
    for (const FreezeFrame& f : frames) {
        std::printf("Freeze frame %d: %s %s%s%s\n", f.frame, f.dtc.c_str(), faultText(f.dtc, brand).c_str(),
                    f.ecu.empty() ? "" : "  ", f.ecu.c_str());
        for (const LiveValue& v : f.values) printValue(v);
        std::printf("\n");
    }
    return 0;
}

const char* gradeKey(analysis::TrafficLight grade) {
    switch (grade) {
        case analysis::TrafficLight::Green: return "green";
        case analysis::TrafficLight::Yellow: return "yellow";
        case analysis::TrafficLight::Red: return "red";
    }
    return "?";
}

const char* areaKey(analysis::Area area) {
    using analysis::Area;
    switch (area) {
        case Area::Faults: return "faults";
        case Area::Readiness: return "readiness";
        case Area::Mixture: return "mixture";
        case Area::OxygenSensors: return "oxygen_sensors";
        case Area::Catalyst: return "catalyst";
        case Area::Misfire: return "misfire";
        case Area::Cooling: return "cooling";
        case Area::Charging: return "charging";
        case Area::Sensors: return "sensors";
        case Area::Monitoring: return "monitoring";
        case Area::History: return "history";
    }
    return "?";
}

const char* fuelKey(analysis::FuelType fuel) {
    switch (fuel) {
        case analysis::FuelType::Petrol: return "petrol";
        case analysis::FuelType::Diesel: return "diesel";
        case analysis::FuelType::Unknown: return "unknown";
    }
    return "?";
}

int commandHealth(Elm327& elm, Obd2& obd, bool asJson, int seconds) {
    using namespace analysis;
    auto progress = [asJson](const char* text) {
        if (!asJson) {
            std::printf("%s\n", text);
            std::fflush(stdout);
        }
    };

    HealthInput input;
    progress("Reading readiness and warning lamp ...");
    const VehicleStatus status = obd.readStatus();
    input.setStatus(status);

    progress("Reading the fault memory ...");
    std::string error;
    input.faults = obd.readFaults(error);
    input.faultsRead = error.empty();
    const VinContext vin = readVinContext(obd);
    const std::string brand = vin.brand;
    input.faultLevel = [brand](const Fault& f) {
        // As in the application: the database knows how urgent a code is,
        // pending codes count one step lower since they may still go away.
        const Severity severity = lookupDtc(f.code, brand).severity;
        Level level = Level::Info;
        if (severity == Severity::Critical) level = Level::Critical;
        else if (severity == Severity::High || severity == Severity::Medium) level = Level::Warning;
        if (!isStored(f) && level == Level::Warning) level = Level::Info;
        return level;
    };

    progress("Reading component monitoring (mode 06) ...");
    std::string monitorNote;
    input.monitorResults = obd.readMonitorResults(monitorNote);
    input.monitorResultsRead = true;

    LiveValue fuelType;
    if (obd.readLive(0x51, fuelType)) input.fuel = fuelTypeFromObd(static_cast<int>(fuelType.value));

    // Live series: only what the car offers.
    const std::set<int> supported = obd.supportedPids();
    std::vector<int> pids;
    for (int pid : healthLivePids()) {
        if (supported.empty() || supported.count(pid)) pids.push_back(pid);
    }
    if (!asJson) {
        std::printf("Collecting %zu live values for %d s ...\n", pids.size(), seconds);
        std::fflush(stdout);
    }
    int snapshots = 0;
    const std::uint64_t start = nowMs();
    while (!pids.empty() && nowMs() - start < static_cast<std::uint64_t>(seconds) * 1000u) {
        const std::vector<LiveValue> values = obd.readLiveBatch(pids);
        if (values.empty()) break;
        input.addSnapshot(values, static_cast<double>(nowMs() - start) / 1000.0);
        ++snapshots;
    }

    const HealthReport report = runHealthCheck(input);

    if (asJson) {
        JsonWriter json;
        json.beginObject();
        jsonAdapter(json, elm);
        json.field("score", report.score)
            .field("grade", gradeKey(report.grade))
            .field("summary", report.summary.text())
            .field("checksRun", report.checksRun)
            .field("fuel", fuelKey(input.fuel))
            .field("brand", brand)
            .key("liveSeries")
            .beginObject()
            .field("seconds", seconds)
            .field("snapshots", snapshots)
            .key("pids")
            .beginArray();
        for (int pid : pids) json.value(pid);
        json.endArray().endObject().key("findings").beginArray();
        for (const Finding& f : report.findings) {
            json.beginObject()
                .field("id", f.id)
                .field("level", levelKey(f.level))
                .field("area", areaKey(f.area))
                .field("title", f.title.text())
                .field("explanation", f.explanation.text())
                .field("advice", f.advice.text())
                .endObject();
        }
        json.endArray().key("notChecked").beginArray();
        for (const SkippedCheck& s : report.notChecked) {
            json.beginObject()
                .field("id", s.id)
                .field("title", s.title.text())
                .field("reason", s.reason.text())
                .endObject();
        }
        json.endArray();
        jsonStatus(json, status);
        jsonFaults(json, input.faults, brand);
        json.endObject();
        std::fputs(json.text().c_str(), stdout);
        return 0;
    }

    std::printf("%d snapshots of %zu values.\n\n", snapshots, pids.size());
    std::printf("Score %d/100 (%s), %d checks run\n", report.score, gradeKey(report.grade),
                report.checksRun);
    std::printf("%s\n", report.summary.text().c_str());
    if (!report.findings.empty()) std::printf("\nFindings:\n");
    for (const Finding& f : report.findings) {
        std::printf("  [%s] %s\n", levelKey(f.level), f.title.text().c_str());
        if (!f.explanation.empty()) std::printf("      %s\n", f.explanation.text().c_str());
        if (!f.advice.empty()) std::printf("      Advice: %s\n", f.advice.text().c_str());
    }
    if (!report.notChecked.empty()) std::printf("\nNot checked:\n");
    for (const SkippedCheck& s : report.notChecked) {
        std::printf("  %s: %s\n", s.title.text().c_str(), s.reason.text().c_str());
    }
    return 0;
}

int commandProbe(Elm327& elm, bool asJson) {
    const AdapterReport r = probeAdapter(elm);

    if (asJson) {
        JsonWriter json;
        json.beginObject();
        jsonAdapter(json, elm);
        json.field("reachable", r.reachable)
            .field("versionText", r.versionText)
            .field("description", r.description)
            .key("claimedVersion");
        if (r.claimedLevel > 0) json.value(versionText(r.claimedLevel));
        else json.null();
        json.field("claimedVersionExists", r.claimedVersionExists).key("behavesLike");
        if (r.behavesLikeLevel > 0) json.value(versionText(r.behavesLikeLevel));
        else json.null();
        json.field("acceptsUnknownCommands", r.acceptsUnknownCommands).field("stn", r.isStn);
        if (r.isStn) json.field("stnId", r.stnId).field("stnDevice", r.stnDevice);
        json.key("voltage");
        if (r.hasVoltage) json.value(r.voltage);
        else json.null();
        json.key("roundTripMs");
        if (r.roundTripSamples > 0) json.value(r.roundTripMs);
        else json.null();
        json.key("roundTripWithCountHintMs");
        if (r.responseCountHint) json.value(r.roundTripWithHintMs);
        else json.null();
        json.key("checks").beginArray();
        for (const CommandCheck& c : r.checks) {
            json.beginObject()
                .field("command", c.command)
                .field("introducedIn", versionText(c.introducedIn))
                .field("supported", c.supported)
                .field("response", c.response)
                .field("purpose", c.purpose.get())
                .endObject();
        }
        json.endArray().key("facts").beginArray();
        for (const BilingualText& t : r.facts) json.value(t.get());
        json.endArray().key("speedTips").beginArray();
        for (const BilingualText& t : r.speedTips) json.value(t.get());
        json.endArray().field("verdict", r.verdict.get()).endObject();
        std::fputs(json.text().c_str(), stdout);
        return r.reachable ? 0 : 3;
    }

    if (!r.reachable) {
        std::printf("The adapter does not answer.\n");
        return 3;
    }
    std::printf("Claims       : %s\n", r.versionText.c_str());
    if (!r.description.empty()) std::printf("Description  : %s\n", r.description.c_str());
    if (r.behavesLikeLevel > 0)
        std::printf("Behaves like : ELM327 %s\n", versionText(r.behavesLikeLevel).c_str());
    if (r.isStn) std::printf("STN chip     : %s %s\n", r.stnId.c_str(), r.stnDevice.c_str());
    if (r.hasVoltage) std::printf("Voltage      : %.1f V\n", r.voltage);
    if (r.roundTripSamples > 0) std::printf("Round trip   : %.0f ms\n", r.roundTripMs);
    if (r.responseCountHint)
        std::printf("With hint    : %.0f ms (expected answer count appended)\n", r.roundTripWithHintMs);
    std::printf("\nCommands:\n");
    for (const CommandCheck& c : r.checks) {
        std::printf("  %-4s %-10s %-6s %s\n", c.supported ? "ok" : "--", c.command.c_str(),
                    versionText(c.introducedIn).c_str(), c.purpose.get().c_str());
    }
    if (!r.facts.empty()) std::printf("\nObserved:\n");
    for (const BilingualText& t : r.facts) std::printf("  %s\n", t.get().c_str());
    if (!r.speedTips.empty()) std::printf("\nFor more speed:\n");
    for (const BilingualText& t : r.speedTips) std::printf("  %s\n", t.get().c_str());
    std::printf("\nVerdict: %s\n", r.verdict.get().c_str());
    return 0;
}

// Prints an error as text, or as {"error": ...} with --json.
int fail(bool asJson, const std::string& message, int code) {
    if (asJson) {
        JsonWriter json;
        json.beginObject().field("error", message).endObject();
        std::fputs(json.text().c_str(), stdout);
    } else {
        std::printf("%s\n", message.c_str());
    }
    return code;
}

// The control units of the demo car, which also suit many real cars that
// follow the "+8" convention.
std::vector<EcuDefinition> defaultUdsList() {
    struct Entry {
        std::uint32_t request;
        EcuKind kind;
        const char* name;
    };
    static const Entry entries[] = {
        {0x7E0, EcuKind::Engine, "Engine"},
        {0x7E1, EcuKind::Gearbox, "Gearbox"},
        {0x760, EcuKind::Brakes, "ABS"},
        {0x737, EcuKind::Airbag, "Airbag"},
        {0x720, EcuKind::Instruments, "Instruments"},
        {0x744, EcuKind::Climate, "Climate"},
    };
    std::vector<EcuDefinition> out;
    for (const Entry& e : entries) {
        EcuDefinition d;
        d.request = e.request;
        d.response = e.request + 8;
        d.kind = e.kind;
        d.name = e.name;
        out.push_back(d);
    }
    return out;
}

// "760,737,18DA28F1" -> definitions. 11-bit ids answer on request + 8,
// 29-bit ids (18DA<target><source>) with target and source swapped.
bool parseUdsIds(const std::string& text, std::vector<EcuDefinition>& out, std::string& error) {
    size_t start = 0;
    for (;;) {
        const size_t comma = text.find(',', start);
        std::string item =
            text.substr(start, comma == std::string::npos ? std::string::npos : comma - start);
        if (item.rfind("0x", 0) == 0 || item.rfind("0X", 0) == 0) item = item.substr(2);
        if (!item.empty()) {
            std::uint32_t id = 0;
            try {
                size_t used = 0;
                id = static_cast<std::uint32_t>(std::stoul(item, &used, 16));
                if (used != item.size()) throw std::invalid_argument(item);
            } catch (const std::exception&) {
                error = "Not a hex id: " + item;
                return false;
            }
            EcuDefinition d;
            d.request = id;
            if (id > 0x7FF) {
                d.extended29 = true;
                d.response = (id & 0xFFFF0000u) | ((id & 0xFFu) << 8) | ((id >> 8) & 0xFFu);
            } else {
                d.response = id + 8;
            }
            char name[24];
            std::snprintf(name, sizeof(name), d.extended29 ? "ECU %08X" : "ECU %03X",
                          static_cast<unsigned>(id));
            d.name = name;
            out.push_back(d);
        }
        if (comma == std::string::npos) break;
        start = comma + 1;
    }
    if (out.empty()) {
        error = "No ids given after --ids";
        return false;
    }
    return true;
}

std::string idText(const EcuDefinition& d, bool response) {
    char buffer[16];
    const unsigned id = static_cast<unsigned>(response ? d.response : d.request);
    std::snprintf(buffer, sizeof(buffer), d.extended29 ? "%08X" : "%03X", id);
    return buffer;
}

std::string dtcStatusText(const UdsDtc& d) {
    std::string out;
    auto add = [&out](const char* word) {
        if (!out.empty()) out += ", ";
        out += word;
    };
    if (d.testFailed) add("failing now");
    if (d.confirmed) add("confirmed");
    if (d.pending) add("pending");
    if (d.warningLamp) add("warning lamp");
    if (out.empty()) add("not failing");
    return out;
}

int commandUds(Elm327& elm, const std::vector<EcuDefinition>& list, bool asJson) {
    if (!elm.isCan()) return fail(asJson, "UDS needs a CAN connection; this car speaks " +
                                              elm.protocolText() + ".", 3);
    const std::uint64_t started = nowMs();
    const std::vector<UdsEcu> ecus = scanEcus(
        elm, list,
        [asJson](int index, int total, const std::string& name) {
            if (!asJson) std::fprintf(stderr, "  [%d/%d] %s ...\n", index + 1, total, name.c_str());
        },
        nullptr);
    const double seconds = static_cast<double>(nowMs() - started) / 1000.0;

    int found = 0;
    for (const UdsEcu& e : ecus) found += e.present ? 1 : 0;

    if (asJson) {
        JsonWriter json;
        json.beginObject()
            .field("seconds", seconds)
            .field("asked", static_cast<int>(ecus.size()))
            .field("present", found)
            .key("ecus")
            .beginArray();
        for (const UdsEcu& e : ecus) {
            json.beginObject()
                .field("name", e.definition.name)
                .field("request", idText(e.definition, false))
                .field("response", idText(e.definition, true))
                .field("present", e.present);
            if (e.present) {
                json.key("info")
                    .beginObject()
                    .field("partNumber", e.info.partNumber)
                    .field("softwareVersion", e.info.softwareVersion)
                    .field("hardwareVersion", e.info.hardwareVersion)
                    .field("systemName", e.info.systemName)
                    .field("serial", e.info.serial)
                    .field("vin", e.info.vin)
                    .endObject();
                json.field("dtcsRead", e.dtcsRead).key("dtcs").beginArray();
                for (const UdsDtc& d : e.dtcs) {
                    json.beginObject()
                        .field("code", d.code)
                        .field("failureType", d.failureType)
                        .field("failureText", d.failureText)
                        .field("status", d.status)
                        .field("testFailed", d.testFailed)
                        .field("pending", d.pending)
                        .field("confirmed", d.confirmed)
                        .field("warningLamp", d.warningLamp)
                        .field("text", d.text)
                        .endObject();
                }
                json.endArray();
            }
            if (!e.note.empty()) json.field("note", e.note);
            json.endObject();
        }
        json.endArray().endObject();
        std::fputs(json.text().c_str(), stdout);
        return 0;
    }

    for (const UdsEcu& e : ecus) {
        std::printf("%s (%s/%s): %s\n", e.definition.name.c_str(), idText(e.definition, false).c_str(),
                    idText(e.definition, true).c_str(), e.present ? "present" : "no answer");
        if (!e.present) continue;
        const UdsEcuInfo& i = e.info;
        if (!i.systemName.empty()) std::printf("  System    %s\n", i.systemName.c_str());
        if (!i.partNumber.empty()) std::printf("  Part no.  %s\n", i.partNumber.c_str());
        if (!i.softwareVersion.empty()) std::printf("  Software  %s\n", i.softwareVersion.c_str());
        if (!i.hardwareVersion.empty()) std::printf("  Hardware  %s\n", i.hardwareVersion.c_str());
        if (!i.serial.empty()) std::printf("  Serial    %s\n", i.serial.c_str());
        if (!i.vin.empty()) std::printf("  VIN       %s\n", i.vin.c_str());
        if (e.dtcsRead && e.dtcs.empty()) std::printf("  No fault codes stored.\n");
        for (const UdsDtc& d : e.dtcs) {
            std::printf("  %s", d.code.c_str());
            if (!d.failureText.empty()) std::printf("-%02X (%s)", d.failureType, d.failureText.c_str());
            std::printf(" [%s]\n      %s\n", dtcStatusText(d).c_str(), d.text.c_str());
        }
        if (!e.note.empty()) std::printf("  Note: %s\n", e.note.c_str());
    }
    std::printf("\n%d of %d control units answered, %.1f s.\n", found,
                static_cast<int>(ecus.size()), seconds);
    return 0;
}

// The values for `custom`: a built-in pack or a Torque CSV file. Done before
// connecting, so a typo costs no adapter time.
bool loadCustomPids(const std::string& pack, const std::string& csvPath, bool demo,
                    std::vector<CustomPid>& out, std::string& title, std::string& error,
                    std::vector<std::string>& warnings) {
    auto packList = [] {
        std::string names;
        for (const std::string& n : packNames()) names += "\n  " + n + "  (" + packTitle(n) + ")";
        return names;
    };
    if (!csvPath.empty()) {
        std::ifstream file(csvPath, std::ios::binary);
        if (!file) {
            error = "Cannot open " + csvPath;
            return false;
        }
        std::ostringstream text;
        text << file.rdbuf();
        out = parseTorqueCsv(text.str(), warnings);
        title = csvPath;
        if (out.empty()) {
            error = "No usable values in " + csvPath;
            return false;
        }
        return true;
    }
    std::string name = pack;
    if (name.empty()) {
        if (!demo) {
            error = "Give --pack NAME or --csv FILE. Built-in packs:" + packList();
            return false;
        }
        name = "caruspex-demo";
    }
    out = packPids(name);
    if (out.empty()) {
        error = "Unknown pack: " + name + ". Built-in packs:" + packList();
        return false;
    }
    title = packTitle(name);
    return true;
}

// Up to three decimals, without trailing zeros: 36.5, 3.6, 5614.8, 12.
std::string valueText(double v) {
    char buffer[40];
    std::snprintf(buffer, sizeof(buffer), "%.3f", v);
    std::string s = buffer;
    while (!s.empty() && s.back() == '0') s.pop_back();
    if (!s.empty() && s.back() == '.') s.pop_back();
    return s == "-0" ? "0" : s;
}

int commandCustom(Elm327& elm, const std::vector<CustomPid>& pids, const std::string& title,
                  const std::vector<std::string>& warnings, bool asJson) {
    const std::uint64_t started = nowMs();
    std::vector<CustomReading> readings;
    {
        CustomPidReader reader(elm);
        readings = reader.readAll(pids);
    }
    const double seconds = static_cast<double>(nowMs() - started) / 1000.0;
    int good = 0;
    for (const CustomReading& r : readings) good += r.ok ? 1 : 0;

    if (asJson) {
        JsonWriter json;
        json.beginObject()
            .field("title", title)
            .field("seconds", seconds)
            .field("read", good)
            .field("failed", static_cast<int>(readings.size()) - good)
            .stringList("importErrors", warnings)
            .key("values")
            .beginArray();
        for (size_t i = 0; i < pids.size(); ++i) {
            const CustomPid& p = pids[i];
            const CustomReading& r = readings[i];
            json.beginObject()
                .field("name", p.name)
                .field("shortName", p.shortName)
                .field("key", p.key)
                .field("request", p.request)
                .field("header", p.header)
                .field("response", r.value.ecuSource)
                .field("ok", r.ok);
            json.key("value");
            if (r.ok) json.value(r.value.value);
            else json.null();
            json.field("unit", p.unit).field("error", r.error).field("source", p.source).endObject();
        }
        json.endArray().endObject();
        std::fputs(json.text().c_str(), stdout);
    } else {
        for (const std::string& w : warnings) std::printf("Skipped: %s\n", w.c_str());
        std::printf("%s\n", title.c_str());
        for (size_t i = 0; i < pids.size(); ++i) {
            const CustomReading& r = readings[i];
            if (r.ok) {
                std::printf("  %-48s %12s %s\n", pids[i].name.c_str(), valueText(r.value.value).c_str(),
                            pids[i].unit.c_str());
            } else {
                std::printf("  %-48s %12s (%s)\n", pids[i].name.c_str(), "-", r.error.c_str());
            }
        }
        std::printf("\n%d of %d values read, %.1f s.\n", good, static_cast<int>(readings.size()),
                    seconds);
    }
    return good > 0 ? 0 : 4;
}

}  // namespace

int main(int argc, char** argv) {
    // The console tool speaks English, including the texts that come from the core.
    setLanguage(Language::English);

    std::string command = argc > 1 ? argv[1] : "ports";
    std::string tcp, port = "COM3";
    int baud = 38400;
    int seconds = 15;
    bool verbose = false;
    bool demo = false;
    bool asJson = false;
    bool fastTiming = false;
    std::string udsIds, recordPath, replayPath;
    std::string pack, csvPath;
    for (int i = 2; i < argc; ++i) {
        const std::string arg = argv[i];
        try {
            if (arg == "--tcp" && i + 1 < argc) tcp = argv[++i];
            else if (arg == "--port" && i + 1 < argc) port = argv[++i];
            else if (arg == "--baud" && i + 1 < argc) baud = std::stoi(argv[++i]);
            else if (arg == "--seconds" && i + 1 < argc) seconds = std::stoi(argv[++i]);
            else if (arg == "--ids" && i + 1 < argc) udsIds = argv[++i];
            else if (arg == "--record" && i + 1 < argc) recordPath = argv[++i];
            else if (arg == "--replay" && i + 1 < argc) replayPath = argv[++i];
            else if (arg == "--fast-timing") fastTiming = true;
            else if (arg == "--pack" && i + 1 < argc) pack = argv[++i];
            else if (arg == "--csv" && i + 1 < argc) csvPath = argv[++i];
            else if (arg == "--demo") demo = true;
            else if (arg == "--json") asJson = true;
            else if (arg == "-v") verbose = true;
            else {
                std::printf("Unknown option: %s\n\n%s", arg.c_str(), kUsage);
                return 1;
            }
        } catch (const std::exception&) {
            std::printf("Not a number after %s\n", arg.c_str());
            return 1;
        }
    }
    if (seconds < 0) seconds = 0;

    if (command == "help" || command == "--help" || command == "-h") {
        std::fputs(kUsage, stdout);
        return 0;
    }
    if (command == "version" || command == "--version") {
        std::printf("caruspex_cli %s\n", CARUSPEX_VERSION);
        return 0;
    }
    if (command == "ports") {
        printPorts();
        return 0;
    }
    if (command == "kklcheck") {
        return kklCheck(port);
    }
    static const char* const known[] = {"read", "clear", "live", "info", "vin",
                                        "freeze", "health", "probe", "ecus", "uds",
                                        "custom"};
    bool isKnown = false;
    for (const char* k : known) isKnown = isKnown || command == k;
    if (!isKnown) {
        std::printf("Unknown command: %s\n\n%s", command.c_str(), kUsage);
        return 1;
    }

    std::string error;
    std::vector<EcuDefinition> udsList;
    if (command == "uds") {
        if (udsIds.empty()) udsList = defaultUdsList();
        else if (!parseUdsIds(udsIds, udsList, error)) return fail(asJson, error, 1);
    }
    std::vector<CustomPid> customPids;
    std::string customTitle;
    std::vector<std::string> customWarnings;
    if (command == "custom" &&
        !loadCustomPids(pack, csvPath, demo, customPids, customTitle, error, customWarnings)) {
        for (const std::string& w : customWarnings) std::fprintf(stderr, "%s\n", w.c_str());
        return fail(asJson, error, 1);
    }

    std::unique_ptr<ITransport> transport;
    std::string protocolOrder = demo ? kProtocolOrderCanFirst : kProtocolOrderKLineFirst;
    if (!replayPath.empty()) {
        auto replay = std::make_unique<ReplayTransport>(replayPath);
        if (!replay->isOpen()) return fail(asJson, replay->error(), 2);
        if (!asJson) std::printf("%s\n", recordingSummary(replay->info()).c_str());
        // The recorded protocol first, so the replay does not have to step
        // through the ones the car never answered on.
        protocolOrder = replay->protocolOrder(protocolOrder);
        transport = std::move(replay);
    } else if (demo) {
        // Ten minutes after ignition on: the engine is warm and the demo car
        // is out on its drive cycle, so the live values say something.
        DemoOptions options;
        const auto origin = std::chrono::steady_clock::now();
        options.clock = [origin] {
            return 600.0 + std::chrono::duration<double>(std::chrono::steady_clock::now() - origin).count();
        };
        transport = std::make_unique<DemoTransport>(options);
    } else {
        transport = makeTransport(tcp, port, baud, error);
    }
    if (!transport) return fail(asJson, "Connection failed: " + error, 2);

    RecordingTransport* recording = nullptr;
    if (!recordPath.empty()) {
        auto wrapped = std::make_unique<RecordingTransport>(std::move(transport), recordPath);
        if (!wrapped->isRecording()) return fail(asJson, wrapped->error(), 2);
        recording = wrapped.get();
        transport = std::move(wrapped);
    }

    Elm327 elm;
    if (fastTiming) elm.setAdaptiveTiming(AdaptiveTiming::Aggressive);
    if (verbose) {
        // With --json the log goes to stderr so stdout stays parseable.
        elm.setLog([asJson](const std::string& m) { std::fprintf(asJson ? stderr : stdout, "%s\n", m.c_str()); });
    }
    if (!elm.connect(std::move(transport), protocolOrder, error)) {
        return fail(asJson, error, 3);
    }
    if (!asJson) {
        std::printf("Adapter    : %s\n", elm.version().c_str());
        if (elm.isStn()) std::printf("STN chip   : %s\n", elm.stnId().c_str());
        std::printf("Connection : %s\n", elm.transportName().c_str());
        std::printf("Protocol   : %s\n", elm.protocolText().c_str());
        if (elm.adaptiveTimingLevel() > 0)
            std::printf("Timing     : ATAT%d\n", elm.adaptiveTimingLevel());
        if (recording) std::printf("Recording  : %s\n", recording->path().c_str());
        std::printf("\n");
    }

    Obd2 obd(elm);
    int result = 0;

    if (command == "read") {
        result = commandRead(elm, obd, asJson);
    } else if (command == "vin") {
        result = commandVin(elm, obd, asJson);
    } else if (command == "freeze") {
        result = commandFreeze(elm, obd, asJson);
    } else if (command == "health") {
        result = commandHealth(elm, obd, asJson, seconds);
    } else if (command == "probe") {
        result = commandProbe(elm, asJson);
    } else if (command == "clear") {
        std::string err;
        std::printf(obd.clearFaults(err) ? "Fault memory cleared.\n" : "Clearing failed.\n");
        if (!err.empty()) std::printf("%s\n", err.c_str());
    } else if (command == "live") {
        for (const LiveValue& v : obd.readLive(defaultLivePids())) printValue(v);
    } else if (command == "info") {
        const std::string vin = obd.readVin();
        std::printf("VIN: %s\n", vin.empty() ? "(not provided)" : vin.c_str());
        std::string trigger;
        const std::vector<LiveValue> frame = obd.readFreezeFrame(trigger);
        if (!trigger.empty()) std::printf("Freeze frame for code %s:\n", trigger.c_str());
        for (const LiveValue& v : frame) printValue(v);
    } else if (command == "uds") {
        result = commandUds(elm, udsList, asJson);
    } else if (command == "custom") {
        result = commandCustom(elm, customPids, customTitle, customWarnings, asJson);
    } else if (command == "ecus") {
        // Everything per control unit: headers on, so every answer keeps
        // its sender.
        if (!elm.setHeaders(true)) std::printf("Adapter refused headers (ATH1).\n");
        std::string err;
        for (const VehicleStatus& s : obd.readStatusPerEcu()) {
            std::printf("%-20s MIL %s, %d code(s), %s\n", s.ecu.c_str(), s.milOn ? "ON" : "off",
                        s.faultCount, s.compressionIgnition ? "diesel" : "spark ignition");
        }
        for (const Fault& f : obd.readFaults(err)) {
            std::printf("  %s [%s] %s\n", f.code.c_str(), f.kind.c_str(), f.ecu.c_str());
        }
        for (const EcuInfo& e : obd.readEcuInfo()) {
            std::printf("\n%s: %s\n", e.ecu.c_str(), e.name.c_str());
            if (!e.vin.empty()) std::printf("  VIN  %s\n", e.vin.c_str());
            for (const std::string& id : e.calibrationIds) std::printf("  CALID %s\n", id.c_str());
            for (const std::string& c : e.cvns) std::printf("  CVN  %s\n", c.c_str());
            for (const InUseCounter& c : e.inUse.monitors) {
                std::printf("  %-40s %s\n", c.name.c_str(), c.text.c_str());
            }
        }
        std::printf("\n");
        for (const MisfireCount& m : misfireCounts(obd.readMonitorResults(err))) {
            std::printf("  %s\n", m.text.c_str());
        }
        std::printf("\n");
        const std::set<int> supported = obd.supportedPids();
        for (const LiveValue& v : obd.readLiveBatch(std::vector<int>(supported.begin(), supported.end()))) {
            if (!v.text.empty()) {
                std::printf("  %03X %-50s %s\n", v.key, v.name.c_str(), v.text.c_str());
            } else {
                std::printf("  %03X %-50s %10.2f %s\n", v.key, v.name.c_str(), v.value, v.unit.c_str());
            }
        }
    }

    elm.disconnect();
    return result;
}
