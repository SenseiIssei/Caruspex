// Health check rules with concrete numbers: each test builds the input a car
// would deliver and checks the finding that comes out.
#include <catch2/catch_test_macros.hpp>

#include <cmath>
#include <string>

#include "core/analysis/HealthCheck.h"

using namespace caruspex;
using namespace caruspex::analysis;

namespace {

bool contains(const std::string& haystack, const std::string& needle) {
    return haystack.find(needle) != std::string::npos;
}

bool isSkipped(const HealthReport& r, const std::string& id) {
    for (const SkippedCheck& s : r.notChecked) {
        if (s.id == id) return true;
    }
    return false;
}

// Warm engine in closed loop at the given rpm, one reading per second.
void warmClosedLoop(HealthInput& in, double from, double to, double rpm) {
    for (double t = from; t <= to; t += 1.0) {
        in.addValue(0x03, 2, t);
        in.addValue(0x05, 90, t);
        in.addValue(0x0C, rpm, t);
    }
}

void trims(HealthInput& in, double from, double to, double st, double lt) {
    for (double t = from; t <= to; t += 1.0) {
        in.addValue(0x06, st, t);
        in.addValue(0x07, lt, t);
    }
}

}  // namespace

TEST_CASE("Phrases format numbers per language", "[analysis]") {
    REQUIRE(number(12.4, 1).de == "12,4");
    REQUIRE(number(12.4, 1).en == "12.4");
    REQUIRE(signedNumber(14.0, 0).de == "+14");
    REQUIRE(signedNumber(-3.26, 1).en == "-3.3");
    REQUIRE(number(-0.01, 1).en == "0.0");
    const Phrase p = format("{} V bei {}/min", "{} V at {} rpm", {number(13.85, 2), number(800, 0)});
    REQUIRE(p.de == "13,85 V bei 800/min");
    REQUIRE(p.en == "13.85 V at 800 rpm");

    setLanguage(Language::English);
    REQUIRE(p.text() == "13.85 V at 800 rpm");
    setLanguage(Language::German);
    REQUIRE(p.text() == "13,85 V bei 800/min");
}

TEST_CASE("Fuel trims: lean, rich and fine", "[analysis][health]") {
    SECTION("+8 and +7 make +15: lean warning") {
        HealthInput in;
        warmClosedLoop(in, 0, 30, 800);
        trims(in, 0, 30, 8, 7);
        const HealthReport r = runHealthCheck(in);
        const Finding* f = r.find("fuel_trim_bank1");
        REQUIRE(f);
        REQUIRE(f->level == Level::Warning);
        REQUIRE(contains(f->title.de, "mager"));
        REQUIRE(contains(f->explanation.de, "+15,0 %"));
        REQUIRE(contains(f->explanation.en, "+15.0%"));
    }
    SECTION("-12 and -10 make -22: rich, critical") {
        HealthInput in;
        warmClosedLoop(in, 0, 30, 800);
        trims(in, 0, 30, -12, -10);
        const HealthReport report = runHealthCheck(in);
        const Finding* f = report.find("fuel_trim_bank1");
        REQUIRE(f);
        REQUIRE(f->level == Level::Critical);
        REQUIRE(contains(f->title.en, "rich"));
    }
    SECTION("+3 and +1 is normal") {
        HealthInput in;
        warmClosedLoop(in, 0, 30, 800);
        trims(in, 0, 30, 3, 1);
        const HealthReport report = runHealthCheck(in);
        const Finding* f = report.find("fuel_trim_bank1");
        REQUIRE(f);
        REQUIRE(f->level == Level::Good);
    }
    SECTION("lean at idle only points to an air leak") {
        HealthInput in;
        warmClosedLoop(in, 0, 19, 800);       // 20 s idle
        trims(in, 0, 19, 9, 7);               // +16
        warmClosedLoop(in, 20, 24, 2500);     // 5 s at 2500
        trims(in, 20, 24, 1, 1);              // +2
        const HealthReport report = runHealthCheck(in);
        const Finding* f = report.find("fuel_trim_bank1");
        REQUIRE(f);
        REQUIRE(f->level == Level::Warning);  // (20*16 + 5*2)/25 = 13.2
        REQUIRE(contains(f->explanation.de, "spricht stark für Falschluft"));
    }
    SECTION("open loop readings are not judged") {
        HealthInput in;
        for (double t = 0; t <= 30; t += 1.0) {
            in.addValue(0x03, 1, t);   // open loop, engine cold
            in.addValue(0x05, 30, t);
            in.addValue(0x0C, 1200, t);
        }
        trims(in, 0, 30, 15, 10);
        const HealthReport r = runHealthCheck(in);
        REQUIRE(r.find("fuel_trim_bank1") == nullptr);
        REQUIRE(isSkipped(r, "fuel_trim_bank1"));
    }
}

TEST_CASE("Charging and battery voltage", "[analysis][health]") {
    auto run = [](double rpm, double volts) {
        HealthInput in;
        for (double t = 0; t < 10; t += 1.0) {
            in.addValue(0x0C, rpm, t);
            in.addValue(0x42, volts, t);
        }
        return runHealthCheck(in);
    };
    REQUIRE(run(800, 14.1).find("charging")->level == Level::Good);
    REQUIRE(run(800, 12.9).find("charging")->level == Level::Warning);
    REQUIRE(run(800, 12.3).find("charging")->level == Level::Critical);
    REQUIRE(run(800, 15.2).find("charging")->level == Level::Warning);
    REQUIRE(run(800, 15.8).find("charging")->level == Level::Critical);
    REQUIRE(run(0, 12.0).find("battery_rest")->level == Level::Warning);
    REQUIRE(run(0, 11.6).find("battery_rest")->level == Level::Critical);
    REQUIRE(run(0, 12.5).find("battery_rest")->level == Level::Good);

    const HealthReport warn = run(800, 12.9);
    REQUIRE(contains(warn.find("charging")->explanation.de, "12,90 V"));
    // Smart charging is mentioned so nobody replaces a good alternator.
    REQUIRE(contains(warn.find("charging")->explanation.en, "smart charging"));

    // Without engine speed it is unclear whether the engine runs.
    HealthInput noRpm;
    noRpm.addValue(0x42, 14.0, 0);
    const HealthReport r = runHealthCheck(noRpm);
    REQUIRE(r.find("charging") == nullptr);
    REQUIRE(isSkipped(r, "charging"));
}

TEST_CASE("Thermostat and overheating", "[analysis][health]") {
    SECTION("20 minutes and only 68 C: stuck open") {
        HealthInput in;
        in.addValue(0x1F, 1200, 0);
        for (double t = 0; t < 60; t += 5) in.addValue(0x05, 60 + t / 10.0, t);   // up to 65.5
        in.addValue(0x05, 68, 60);
        const HealthReport report = runHealthCheck(in);
        const Finding* f = report.find("thermostat");
        REQUIRE(f);
        REQUIRE(f->level == Level::Warning);
        REQUIRE(contains(f->explanation.de, "68 °C"));
        REQUIRE(contains(f->explanation.de, "20 Minuten"));
    }
    SECTION("5 minutes and 60 C: too early to tell") {
        HealthInput in;
        in.addValue(0x1F, 300, 0);
        in.addValue(0x05, 60, 0);
        const HealthReport r = runHealthCheck(in);
        REQUIRE(r.find("thermostat") == nullptr);
        REQUIRE(isSkipped(r, "thermostat"));
    }
    SECTION("90 C is fine, 117 C is overheating") {
        HealthInput ok;
        ok.addValue(0x05, 90, 0);
        REQUIRE(runHealthCheck(ok).find("thermostat")->level == Level::Good);

        HealthInput hot;
        hot.addValue(0x05, 117, 0);
        REQUIRE(runHealthCheck(hot).find("overheating")->level == Level::Critical);
    }
    SECTION("-40 C means a broken sensor") {
        HealthInput in;
        in.addValue(0x05, -40, 0);
        REQUIRE(runHealthCheck(in).find("sensor_coolant")->level == Level::Warning);
    }
}

TEST_CASE("Readiness and emissions test guidance", "[analysis][health]") {
    SECTION("petrol with only EVAP open: info") {
        HealthInput in;
        in.fuel = FuelType::Petrol;
        in.readiness = decodeReadiness(0x00, 0x07, 0x65, 0x04).monitors;
        in.readinessRead = true;
        const HealthReport report = runHealthCheck(in);
        const Finding* f = report.find("readiness");
        REQUIRE(f);
        REQUIRE(f->level == Level::Info);
        REQUIRE(contains(f->explanation.de, "Tankentlüftung"));
        REQUIRE(contains(f->advice.en, "EPA"));
    }
    SECTION("petrol with two open: warning") {
        HealthInput in;
        in.readiness = decodeReadiness(0x00, 0x07, 0x65, 0x05).monitors;
        in.readinessRead = true;
        REQUIRE(runHealthCheck(in).find("readiness")->level == Level::Warning);
    }
    SECTION("diesel with one open: warning") {
        HealthInput in;
        in.readiness = decodeReadiness(0x00, 0x0F, 0xE9, 0x40).monitors;
        in.readinessRead = true;
        const HealthReport report = runHealthCheck(in);
        const Finding* f = report.find("readiness");
        REQUIRE(f->level == Level::Warning);
        REQUIRE(contains(f->explanation.en, "Particulate filter"));
    }
    SECTION("all complete") {
        HealthInput in;
        in.readiness = decodeReadiness(0x00, 0x07, 0x65, 0x00).monitors;
        in.readinessRead = true;
        const HealthReport report = runHealthCheck(in);
        const Finding* f = report.find("readiness");
        REQUIRE(f->level == Level::Good);
        REQUIRE(contains(f->explanation.de, "7 von 7"));
    }
}

TEST_CASE("Recently cleared codes", "[analysis][health]") {
    HealthInput fresh;
    fresh.addValue(0x30, 2, 0);
    fresh.addValue(0x31, 12, 0);
    fresh.readiness = decodeReadiness(0x00, 0x07, 0x65, 0x25).monitors;
    fresh.readinessRead = true;
    const HealthReport report = runHealthCheck(fresh);
    const Finding* f = report.find("cleared_recently");
    REQUIRE(f);
    REQUIRE(f->level == Level::Warning);
    REQUIRE(contains(f->explanation.de, "2 Warmläufe und 12 km"));
    REQUIRE(contains(f->explanation.en, "incomplete readiness monitors"));

    HealthInput old;
    old.addValue(0x30, 40, 0);
    old.addValue(0x31, 1500, 0);
    REQUIRE(runHealthCheck(old).find("cleared_recently")->level == Level::Good);

    HealthInput lamp;
    lamp.addValue(0x21, 230, 0);
    const HealthReport lampReport = runHealthCheck(lamp);
    const Finding* d = lampReport.find("mil_distance");
    REQUIRE(d);
    REQUIRE(contains(d->title.de, "230 km"));
}

TEST_CASE("Oxygen sensors and catalyst", "[analysis][health]") {
    const double kPi = 3.14159265358979;
    auto base = [&](HealthInput& in) {
        // 30 s warm closed loop at 4 readings per second.
        for (double t = 0; t <= 30.0; t += 0.25) {
            in.addValue(0x03, 2, t);
            in.addValue(0x05, 90, t);
            in.addValue(0x0C, 2000, t);
        }
    };
    SECTION("switching front sensor, calm rear sensor") {
        HealthInput in;
        base(in);
        for (double t = 0; t <= 30.0; t += 0.25) {
            in.addValue(0x14, 0.45 + 0.4 * std::sin(2 * kPi * 0.5 * t + 0.3), t);
            in.addValue(0x15, 0.65, t);
        }
        const HealthReport r = runHealthCheck(in);
        REQUIRE(r.find("o2_switching_bank1")->level == Level::Good);
        REQUIRE(r.find("catalyst_bank1")->level == Level::Good);
    }
    SECTION("rear sensor copies the front one: worn catalyst") {
        HealthInput in;
        base(in);
        for (double t = 0; t <= 30.0; t += 0.25) {
            const double v = 0.45 + 0.4 * std::sin(2 * kPi * 0.5 * t + 0.3);
            in.addValue(0x14, v, t);
            in.addValue(0x15, v, t);
        }
        const HealthReport report = runHealthCheck(in);
        const Finding* f = report.find("catalyst_bank1");
        REQUIRE(f);
        REQUIRE(f->level == Level::Warning);
    }
    SECTION("front sensor stuck lean") {
        HealthInput in;
        base(in);
        for (double t = 0; t <= 30.0; t += 0.25) in.addValue(0x14, 0.06, t);
        const HealthReport report = runHealthCheck(in);
        const Finding* f = report.find("o2_switching_bank1");
        REQUIRE(f);
        REQUIRE(f->level == Level::Warning);
        REQUIRE(contains(f->explanation.de, "dauernd mager"));
    }
}

TEST_CASE("Mode 06 limits and misfire counters", "[analysis][health]") {
    auto result = [](int tid, int cid, double value, double min, double max) {
        MonitorResult m;
        m.testId = tid;
        m.componentId = cid;
        m.name = "Test";
        m.value = value;
        m.minimum = min;
        m.maximum = max;
        m.hasMinimum = m.hasMaximum = true;
        m.passed = value >= min && value <= max;
        return m;
    };
    HealthInput in;
    in.monitorResultsRead = true;
    in.monitorResults = {
        result(0x81, 0x21, 0.95, 0.0, 1.0),   // 5 % below the limit: close
        result(0x82, 0x21, 0.50, 0.0, 1.0),   // far away: fine
        result(0x85, 0x01, 1.30, 0.0, 1.2),   // failed
        result(0x86, 0x01, 0.0, 0.0, 0.0),    // not run yet: ignored
        result(0x0C, 0xA2, 2, 0, 65535),      // cylinder 1
        result(0x0C, 0xA3, 1, 0, 65535),      // cylinder 2
        result(0x0C, 0xA4, 40, 0, 65535),     // cylinder 3
        result(0x0C, 0xA5, 0, 0, 65535),      // cylinder 4
    };
    const HealthReport r = runHealthCheck(in);
    REQUIRE(r.find("mode06:21:81")->level == Level::Info);
    REQUIRE(r.find("mode06:21:82") == nullptr);
    REQUIRE(r.find("mode06:01:85")->level == Level::Warning);
    REQUIRE(r.find("mode06:01:86") == nullptr);
    const Finding* m = r.find("misfire");
    REQUIRE(m);
    REQUIRE(m->level == Level::Warning);
    REQUIRE(contains(m->explanation.de, "Zylinder 3: 40"));
    REQUIRE(contains(m->explanation.en, "cylinder 3. That points"));

    HealthInput none;
    none.monitorResultsRead = true;
    none.monitorResults = {result(0x0B, 0xA2, 0, 0, 65535), result(0x0B, 0xA3, 0, 0, 65535)};
    REQUIRE(runHealthCheck(none).find("misfire")->level == Level::Good);
}

TEST_CASE("Score and traffic light", "[analysis][health]") {
    SECTION("nothing read: everything is listed as not checked") {
        const HealthReport r = runHealthCheck(HealthInput{});
        REQUIRE(r.findings.empty());
        REQUIRE(r.score == 100);
        REQUIRE(isSkipped(r, "faults"));
        REQUIRE(isSkipped(r, "readiness"));
        REQUIRE(isSkipped(r, "charging"));
        REQUIRE(isSkipped(r, "mode06"));
    }
    SECTION("clean car") {
        HealthInput in;
        in.faultsRead = true;
        in.milOn = false;
        in.readiness = decodeReadiness(0x00, 0x07, 0x65, 0x00).monitors;
        in.readinessRead = true;
        const HealthReport r = runHealthCheck(in);
        REQUIRE(r.score == 100);
        REQUIRE(r.grade == TrafficLight::Green);
        REQUIRE(r.checksRun == 3);
    }
    SECTION("lamp on: yellow and at most 84") {
        HealthInput in;
        in.milOn = true;
        const HealthReport r = runHealthCheck(in);
        REQUIRE(r.grade == TrafficLight::Yellow);
        REQUIRE(r.score <= 84);
    }
    SECTION("the severity of a fault comes from the caller") {
        HealthInput in;
        in.faultsRead = true;
        in.faults = {{"P0301", "stored", "Cylinder 1 misfire", false},
                     {"P0442", "pending", "EVAP small leak", false}};
        in.faultLevel = [](const Fault& f) {
            return f.code == "P0301" ? Level::Critical : Level::Info;
        };
        const HealthReport r = runHealthCheck(in);
        REQUIRE(r.grade == TrafficLight::Red);
        REQUIRE(r.score <= 49);
        REQUIRE(r.findings.front().id == "fault:P0301");   // worst first
        REQUIRE(r.find("fault:P0442")->level == Level::Info);
        REQUIRE(contains(r.find("fault:P0442")->explanation.en, "pending"));
    }
}

TEST_CASE("No em dashes in health check texts", "[analysis][health]") {
    HealthInput in;
    in.faultsRead = true;
    in.milOn = true;
    in.readiness = decodeReadiness(0x00, 0x07, 0x65, 0x25).monitors;
    in.readinessRead = true;
    in.addValue(0x30, 1, 0);
    in.addValue(0x42, 12.0, 0);
    in.addValue(0x0C, 0, 0);
    in.addValue(0x05, 120, 0);
    const HealthReport r = runHealthCheck(in);
    const std::string emDash = "\xE2\x80\x94";
    for (const Finding& f : r.findings) {
        for (const Phrase* p : {&f.title, &f.explanation, &f.advice}) {
            REQUIRE_FALSE(contains(p->de, emDash));
            REQUIRE_FALSE(contains(p->en, emDash));
        }
    }
    REQUIRE_FALSE(contains(r.summary.de, emDash));
}
