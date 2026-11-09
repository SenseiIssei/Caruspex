#include "core/analysis/HealthCheck.h"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <numeric>
#include <set>

namespace caruspex::analysis {

void HealthInput::setStatus(const VehicleStatus& status) {
    if (!status.valid) return;
    milOn = status.milOn;
    readiness = readinessFromStatus(status);
    readinessRead = true;
}

void HealthInput::addSnapshot(const std::vector<LiveValue>& values, double t) {
    // Obd2::readLiveBatch hands out sub-values as well (both sensors of
    // PID 0x67, ...). A PID's series takes its first value, which is the
    // primary one whenever the control unit reports it, like readLive.
    std::set<int> seen;
    for (const LiveValue& v : values) {
        if (seen.insert(v.pid).second) addValue(v.pid, v.value, t);
    }
}

const std::vector<int>& healthLivePids() {
    static const std::vector<int> pids = {0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0C,
                                          0x0D, 0x0F, 0x10, 0x14, 0x15, 0x16, 0x17, 0x18,
                                          0x19, 0x1F, 0x21, 0x30, 0x31, 0x3C, 0x42, 0x5C};
    return pids;
}

FuelType fuelTypeFromObd(int code) {
    switch (code) {
        case 4:    // diesel
        case 19:   // hybrid diesel
        case 23:   // bifuel, running on diesel
            return FuelType::Diesel;
        case 1:    // gasoline
        case 9:    // bifuel, running on gasoline
        case 17:   // hybrid gasoline
            return FuelType::Petrol;
        default: return FuelType::Unknown;
    }
}

void HealthInput::addValue(int pid, double value, double t) {
    if (!std::isfinite(value) || !std::isfinite(t)) return;
    live[pid].push_back({t, value});
}

const Finding* HealthReport::find(const std::string& id) const {
    for (const Finding& f : findings) {
        if (f.id == id) return &f;
    }
    return nullptr;
}

namespace {

using Series = std::vector<TimedValue>;

constexpr int kPidFuelStatus = 0x03;
constexpr int kPidCoolant = 0x05;
constexpr int kPidRpm = 0x0C;
constexpr int kPidIntakeTemp = 0x0F;
constexpr int kPidRunTime = 0x1F;
constexpr int kPidMilDistance = 0x21;
constexpr int kPidWarmups = 0x30;
constexpr int kPidClearDistance = 0x31;
constexpr int kPidCatalystTemp = 0x3C;
constexpr int kPidVoltage = 0x42;
constexpr int kPidOilTemp = 0x5C;

constexpr double kRunningRpm = 400.0;    // above this the engine runs
constexpr double kStoppedRpm = 50.0;     // below this it is off
constexpr double kMatchWindow = 3.0;     // s, how far apart two PIDs may be read
constexpr double kWarmForTrims = 70.0;   // C, below that trims are not meaningful

// Penalties for the score, per finding.
constexpr int kPenaltyInfo = 3;
constexpr int kPenaltyWarning = 12;
constexpr int kPenaltyCritical = 30;

const Series* get(const HealthInput& in, int pid) {
    auto it = in.live.find(pid);
    if (it == in.live.end() || it->second.empty()) return nullptr;
    return &it->second;
}

// The reading closest in time to t, if one lies within the window.
std::optional<double> nearest(const Series* s, double t, double window = kMatchWindow) {
    if (!s) return std::nullopt;
    const TimedValue* best = nullptr;
    for (const TimedValue& v : *s) {
        if (!best || std::fabs(v.t - t) < std::fabs(best->t - t)) best = &v;
    }
    if (!best || std::fabs(best->t - t) > window) return std::nullopt;
    return best->value;
}

double latest(const Series& s) {
    const TimedValue* best = &s.front();
    for (const TimedValue& v : s) {
        if (v.t >= best->t) best = &v;
    }
    return best->value;
}

double maxOf(const Series& s) {
    double m = s.front().value;
    for (const TimedValue& v : s) m = std::max(m, v.value);
    return m;
}

double minOf(const Series& s) {
    double m = s.front().value;
    for (const TimedValue& v : s) m = std::min(m, v.value);
    return m;
}

double mean(const std::vector<double>& v) {
    return v.empty() ? 0.0 : std::accumulate(v.begin(), v.end(), 0.0) / v.size();
}

double median(std::vector<double> v) {
    if (v.empty()) return 0.0;
    std::sort(v.begin(), v.end());
    const size_t n = v.size();
    return n % 2 ? v[n / 2] : (v[n / 2 - 1] + v[n / 2]) / 2.0;
}

std::string lower(std::string s) {
    for (char& c : s) {
        if (c >= 'A' && c <= 'Z') c = static_cast<char>(c - 'A' + 'a');
    }
    return s;
}

struct Ctx {
    const HealthInput& in;
    HealthReport& out;

    void add(std::string id, Level level, Area area, Phrase title, Phrase explanation,
             Phrase advice = {}) {
        out.findings.push_back(
            {std::move(id), level, area, std::move(title), std::move(explanation), std::move(advice)});
    }
    void skip(std::string id, Phrase title, Phrase reason) {
        out.notChecked.push_back({std::move(id), std::move(title), std::move(reason)});
    }
    void ran() { ++out.checksRun; }

    bool diesel() const { return in.fuel == FuelType::Diesel; }

    // True if the sample at time t was taken with a warm engine in closed
    // loop, as far as the available values tell.
    bool closedLoopWarm(double t) const {
        if (auto status = nearest(get(in, kPidFuelStatus), t)) {
            if (static_cast<int>(*status) != 2) return false;
        }
        if (auto coolant = nearest(get(in, kPidCoolant), t)) {
            if (*coolant < kWarmForTrims) return false;
        }
        if (auto rpm = nearest(get(in, kPidRpm), t)) {
            if (*rpm < kRunningRpm) return false;
        }
        return true;
    }

    bool hasIncompleteReadiness() const {
        for (const MonitorState& m : in.readiness) {
            if (m.supported && !m.complete) return true;
        }
        return false;
    }
};

// --- Fault codes -----------------------------------------------------------

void checkFaults(Ctx& c) {
    if (!c.in.faultsRead) {
        c.skip("faults", format("Fehlerspeicher", "Fault memory"),
               format("Der Fehlerspeicher wurde nicht gelesen.", "The fault memory was not read."));
        return;
    }
    c.ran();
    if (c.in.faults.empty()) {
        c.add("faults", Level::Good, Area::Faults,
              format("Keine Fehler gespeichert", "No fault codes stored"),
              format("Das Motorsteuergerät meldet keinen gespeicherten, vorgemerkten oder "
                     "dauerhaften Fehler.",
                     "The engine control unit reports no stored, pending or permanent fault."));
        return;
    }
    for (const Fault& f : c.in.faults) {
        const Level level = c.in.faultLevel ? c.in.faultLevel(f) : Level::Warning;
        const std::string kind = lower(f.kind);
        Phrase meaning;
        if (kind.find("dauerhaft") != std::string::npos || kind.find("permanent") != std::string::npos) {
            meaning = format(
                "Dieser Fehler ist dauerhaft gespeichert. Er lässt sich nicht löschen und "
                "verschwindet erst, wenn das Steuergerät selbst festgestellt hat, dass alles "
                "wieder in Ordnung ist.",
                "This fault is stored as permanent. It cannot be cleared and only goes away once "
                "the control unit has confirmed by itself that everything is fine again.");
        } else if (kind.find("sporadisch") != std::string::npos ||
                   kind.find("intermittent") != std::string::npos ||
                   kind.find("vorgemerkt") != std::string::npos ||
                   kind.find("pending") != std::string::npos) {
            meaning = format(
                "Dieser Fehler ist erst vorgemerkt: Das Steuergerät hat ihn einmal bemerkt, aber "
                "noch nicht bestätigt. Taucht er bei den nächsten Fahrten wieder auf, wird er "
                "fest gespeichert.",
                "This fault is only pending: the control unit noticed it once but has not "
                "confirmed it yet. If it shows up again on the next trips, it gets stored for "
                "good.");
        } else {
            meaning = format("Dieser Fehler ist fest gespeichert.", "This fault is stored.");
        }
        Phrase advice;
        switch (level) {
            case Level::Critical:
                advice = format("Bald in die Werkstatt, und bis dahin möglichst wenig und "
                                "schonend fahren.",
                                "Have it seen by a garage soon, and until then drive as little "
                                "and as gently as possible.");
                break;
            case Level::Warning:
                advice = format("Bei Gelegenheit prüfen lassen. In der Fehlerliste stehen "
                                "typische Ursachen.",
                                "Have it checked when convenient. The fault list shows typical "
                                "causes.");
                break;
            default:
                advice = format("Im Auge behalten. Kommt er nach dem Löschen wieder, genauer "
                                "nachsehen.",
                                "Keep an eye on it. If it comes back after clearing, look into "
                                "it more closely.");
                break;
        }
        const std::string title = f.text.empty() ? f.code : f.code + " " + f.text;
        c.add("fault:" + f.code, level, Area::Faults, same(title), meaning, advice);
    }
}

// --- Warning lamp ----------------------------------------------------------

void checkMil(Ctx& c) {
    if (!c.in.milOn) {
        c.skip("mil", format("Motorkontrollleuchte", "Check engine lamp"),
               format("Der Zustand der Leuchte (PID 01) wurde nicht gelesen.",
                      "The lamp state (PID 01) was not read."));
        return;
    }
    c.ran();
    if (*c.in.milOn) {
        c.add("mil", Level::Warning, Area::Faults,
              format("Motorkontrollleuchte an", "Check engine lamp on"),
              format("Das Steuergerät hat die gelbe Motorkontrollleuchte eingeschaltet. Es hat "
                     "also einen Fehler erkannt, der die Abgaswerte verschlechtern kann. Mit "
                     "leuchtender Lampe fällt ein Auto in vielen Ländern durch die "
                     "Abgasuntersuchung.",
                     "The control unit has switched on the yellow check engine lamp. It has "
                     "found a fault that can make the emissions worse. With the lamp on, a car "
                     "fails the emissions test in many countries."),
              format("Die Fehlerliste zeigt den Grund. Blinkt die Lampe während der Fahrt, "
                     "sofort Gas wegnehmen und bald anhalten: Das bedeutet starke Aussetzer, die "
                     "den Katalysator zerstören können.",
                     "The fault list shows the reason. If the lamp flashes while driving, ease "
                     "off at once and stop soon: that means heavy misfires that can destroy the "
                     "catalytic converter."));
    } else {
        c.add("mil", Level::Good, Area::Faults,
              format("Motorkontrollleuchte aus", "Check engine lamp off"),
              format("Das Steuergerät meldet die Warnlampe als aus.",
                     "The control unit reports the warning lamp as off."));
    }
}

// --- Readiness -------------------------------------------------------------

void checkReadiness(Ctx& c) {
    if (!c.in.readinessRead) {
        c.skip("readiness", format("Bereitschaftstests", "Readiness monitors"),
               format("Die Bereitschaft (PID 01) wurde nicht gelesen.",
                      "Readiness (PID 01) was not read."));
        return;
    }
    c.ran();
    int supported = 0;
    std::vector<Monitor> open;
    bool dieselMonitors = false;
    for (const MonitorState& m : c.in.readiness) {
        if (!m.supported) continue;
        ++supported;
        if (m.monitor >= Monitor::NmhcCatalyst) dieselMonitors = true;
        if (!m.complete) open.push_back(m.monitor);
    }
    const bool diesel = c.diesel() || dieselMonitors;

    if (open.empty()) {
        c.add("readiness", Level::Good, Area::Readiness,
              format("Alle Bereitschaftstests fertig", "All readiness monitors complete"),
              format("{} von {} Selbsttests sind abgeschlossen. Das ist die beste Voraussetzung "
                     "für die Abgasuntersuchung.",
                     "{} of {} self-tests are complete. That is the best starting point for "
                     "the emissions test.",
                     {number(supported, 0), number(supported, 0)}));
        return;
    }

    Phrase list;
    for (size_t i = 0; i < open.size(); ++i) {
        const Text name = monitorName(open[i]);
        if (i) {
            list.de += ", ";
            list.en += ", ";
        }
        list.de += name.de;
        list.en += name.en;
    }
    const Level level = diesel ? Level::Warning : (open.size() == 1 ? Level::Info : Level::Warning);
    const Phrase title = open.size() == 1
        ? format("1 Bereitschaftstest noch offen", "1 readiness monitor not complete")
        : format("{} Bereitschaftstests noch offen", "{} readiness monitors not complete",
                 {number(static_cast<double>(open.size()), 0)});
    const Phrase explanation = format(
        "Noch nicht fertig: {}. Das Auto hat diese Selbsttests seit dem letzten Löschen des "
        "Fehlerspeichers (oder seit die Batterie abgeklemmt war) noch nicht zu Ende gebracht. "
        "Das ist an sich kein Defekt, solange keine Fehler gespeichert sind.",
        "Not complete yet: {}. The car has not finished these self-tests since the fault "
        "memory was last cleared (or since the battery was disconnected). That alone is not a "
        "defect, as long as no faults are stored.",
        {list});
    const Phrase rule = diesel
        ? format("Für die Abgasuntersuchung sollten beim Diesel möglichst alle Tests fertig "
                 "sein; die genauen Regeln hängen vom Land und der Prüfstelle ab.",
                 "For the emissions test, a diesel should have all monitors complete if "
                 "possible; the exact rules depend on the country and the testing station.")
        : format("Als Faustregel für die Abgasuntersuchung: Beim Benziner wird oft ein offener "
                 "Test hingenommen (so regelt es zum Beispiel die US-Umweltbehörde EPA für "
                 "Autos ab Baujahr 2001), mehrere offene Tests meist nicht. Die genauen Regeln "
                 "hängen vom Land und der Prüfstelle ab.",
                 "As a rule of thumb for the emissions test: a petrol car is often accepted "
                 "with one incomplete monitor (that is the US EPA rule for model year 2001 and "
                 "later), with several usually not. The exact rules depend on the country and "
                 "the testing station.");
    const Phrase advice = rule + format(
        "Am sichersten ist es, vorher die passenden Fahrten zu machen (siehe Fahrzyklus-Hilfe) "
        "und danach erneut zu prüfen.",
        "The safest way is to do the matching drives first (see the drive cycle help) and "
        "check again afterwards.");
    c.add("readiness", level, Area::Readiness, title, explanation, advice);
}

// --- Clearing history ------------------------------------------------------

void checkHistory(Ctx& c) {
    const Series* warmups = get(c.in, kPidWarmups);
    const Series* distance = get(c.in, kPidClearDistance);
    if (!warmups && !distance) {
        c.skip("cleared_recently",
               format("Zuletzt gelöscht", "Last cleared"),
               format("Warmläufe und Strecke seit dem Löschen (PID 30 und 31) fehlen.",
                      "Warm-ups and distance since clearing (PID 30 and 31) are missing."));
    } else {
        c.ran();
        const std::optional<double> w = warmups ? std::optional<double>(latest(*warmups)) : std::nullopt;
        const std::optional<double> d = distance ? std::optional<double>(latest(*distance)) : std::nullopt;
        const bool recent = (w && *w < limits::kFewWarmups) || (d && *d < limits::kShortDistanceKm);

        Phrase facts;
        if (w && d) {
            facts = format("Seit dem letzten Löschen: {} Warmläufe und {} km.",
                           "Since the last clearing: {} warm-ups and {} km.",
                           {number(*w, 0), number(*d, 0)});
        } else if (w) {
            facts = format("Seit dem letzten Löschen: {} Warmläufe.",
                           "Since the last clearing: {} warm-ups.", {number(*w, 0)});
        } else {
            facts = format("Seit dem letzten Löschen: {} km.", "Since the last clearing: {} km.",
                           {number(*d, 0)});
        }

        if (recent) {
            Phrase explanation = facts + format(
                "Das ist sehr wenig. Mit dem Löschen werden auch die Bereitschaftstests "
                "zurückgesetzt, und Fehler, die erst nach einiger Zeit wiederkommen, sind noch "
                "nicht zu sehen. Beim Gebrauchtwagenkauf kann das ein Hinweis sein, dass "
                "jemand Fehler kurz vor der Besichtigung gelöscht hat. Es gibt aber auch "
                "harmlose Gründe, etwa eine Reparatur oder eine abgeklemmte Batterie.",
                "That is very little. Clearing also resets the readiness monitors, and faults "
                "that only come back after a while are not visible yet. When buying a used "
                "car, this can be a hint that someone cleared faults shortly before the "
                "viewing. There are harmless reasons too, such as a repair or a disconnected "
                "battery.");
            if (c.hasIncompleteReadiness()) {
                explanation = explanation + format("Dazu passen die noch offenen Bereitschaftstests.",
                                                   "The incomplete readiness monitors fit that picture.");
            }
            c.add("cleared_recently", Level::Warning, Area::History,
                  format("Fehlerspeicher vor Kurzem gelöscht", "Fault memory cleared recently"),
                  explanation,
                  format("Beim Kauf nachfragen, warum gelöscht wurde. Am besten nach einer "
                         "Probefahrt von 20 bis 30 Minuten oder nach ein paar Tagen Fahrt "
                         "noch einmal prüfen.",
                         "When buying, ask why it was cleared. Ideally check again after a test "
                         "drive of 20 to 30 minutes or after a few days of driving."));
        } else {
            c.add("cleared_recently", Level::Good, Area::History,
                  format("Nicht frisch gelöscht", "Not freshly cleared"),
                  facts + format("Kein Hinweis auf ein Löschen kurz vor der Prüfung.",
                                 "No sign of clearing shortly before the check."));
        }
    }

    if (const Series* mil = get(c.in, kPidMilDistance)) {
        c.ran();
        const double km = latest(*mil);
        if (km > 0) {
            c.add("mil_distance", Level::Info, Area::History,
                  format("Warnlampe war {} km lang an", "Warning lamp was on for {} km",
                         {number(km, 0)}),
                  format("Seit dem letzten Löschen ist das Auto {} km mit leuchtender "
                         "Motorkontrollleuchte gefahren.",
                         "Since the last clearing the car has been driven {} km with the check "
                         "engine lamp on.",
                         {number(km, 0)}),
                  format("Je länger ein Fehler ignoriert wird, desto eher entstehen Folgeschäden. "
                         "Den Grund in der Fehlerliste nachsehen.",
                         "The longer a fault is ignored, the more likely it causes further "
                         "damage. Look up the reason in the fault list."));
        }
    }
}

// --- Fuel trims ------------------------------------------------------------

void checkTrims(Ctx& c, int bank) {
    const int stPid = bank == 1 ? 0x06 : 0x08;
    const int ltPid = bank == 1 ? 0x07 : 0x09;
    const Series* st = get(c.in, stPid);
    const Series* lt = get(c.in, ltPid);
    const std::string id = bank == 1 ? "fuel_trim_bank1" : "fuel_trim_bank2";
    const Phrase bankName = format("Bank {}", "bank {}", {number(bank, 0)});
    const Phrase checkTitle = format("Gemischregelung ({})", "Mixture control ({})", {bankName});

    if (!st && !lt) {
        // Bank 2 only exists on V and boxer engines; its absence is normal.
        if (bank == 1) {
            c.skip(id, checkTitle,
                   c.diesel()
                       ? format("Diesel melden meist keine Gemischkorrektur (PID 06 bis 09).",
                                "Diesels usually report no fuel trims (PID 06 to 09).")
                       : format("Die Gemischkorrektur (PID 06 und 07) wurde nicht gelesen.",
                                "The fuel trims (PID 06 and 07) were not read."));
        }
        return;
    }

    const Series* base = st ? st : lt;
    const Series* other = st ? lt : nullptr;
    const Series* rpm = get(c.in, kPidRpm);
    std::vector<double> all, idle, cruise;
    for (const TimedValue& s : *base) {
        if (!c.closedLoopWarm(s.t)) continue;
        double total = s.value;
        if (other) {
            const auto o = nearest(other, s.t);
            if (!o) continue;
            total += *o;
        }
        all.push_back(total);
        if (const auto r = nearest(rpm, s.t)) {
            if (*r < 1100) idle.push_back(total);
            else if (*r >= 1800) cruise.push_back(total);
        }
    }
    if (all.empty()) {
        c.skip(id, checkTitle,
               format("Keine Werte bei warmem Motor im geschlossenen Regelkreis. Die Korrektur "
                      "sagt erst etwas aus, wenn der Motor warm ist und die Lambdaregelung "
                      "arbeitet.",
                      "No readings with a warm engine in closed loop. The trims only mean "
                      "something once the engine is warm and the oxygen control is active."));
        return;
    }
    c.ran();
    const double avg = mean(all);
    const double magnitude = std::fabs(avg);
    const Phrase value = signedNumber(avg, 1);
    const Phrase partial = (st && lt) ? Phrase{}
        : format("(Nur ein Teil der Korrektur wurde gelesen, das Ergebnis ist deshalb ungenauer.)",
                 "(Only part of the correction was read, so the result is less precise.)");

    if (magnitude < limits::kTrimWarn) {
        c.add(id, Level::Good, Area::Mixture,
              format("Gemischregelung in Ordnung ({})", "Mixture control fine ({})", {bankName}),
              format("Die Korrektur liegt im Schnitt bei {} %, also im normalen Bereich von "
                     "plus/minus 10 %.",
                     "The correction averages {}%, well within the normal range of plus/minus "
                     "10%.",
                     {value}) + partial);
        return;
    }
    const Level level = magnitude >= limits::kTrimBad ? Level::Critical : Level::Warning;

    // Comparing idle and higher engine speed narrows the cause down: an air
    // leak matters most at idle, where little air flows anyway.
    Phrase pattern;
    if (avg > 0 && idle.size() >= 3 && cruise.size() >= 3) {
        const double atIdle = mean(idle);
        const double atCruise = mean(cruise);
        if (atIdle >= limits::kTrimWarn && atCruise < 5.0) {
            pattern = format("Im Leerlauf ist die Korrektur hoch ({} %), bei höherer Drehzahl "
                             "fast normal ({} %). Das spricht stark für Falschluft.",
                             "At idle the correction is high ({}%), at higher engine speed "
                             "almost normal ({}%). That points strongly to an air leak.",
                             {signedNumber(atIdle, 1), signedNumber(atCruise, 1)});
        } else if (atIdle >= limits::kTrimWarn && atCruise >= limits::kTrimWarn) {
            pattern = format("Die Korrektur ist im Leerlauf und bei höherer Drehzahl hoch. Das "
                             "spricht eher für den Luftmassenmesser oder den Kraftstoffdruck "
                             "als für Falschluft.",
                             "The correction is high both at idle and at higher engine speed. "
                             "That points more to the mass air flow sensor or the fuel "
                             "pressure than to an air leak.");
        }
    }

    if (avg > 0) {
        c.add(id, level, Area::Mixture,
              format("Gemisch zu mager ({})", "Mixture too lean ({})", {bankName}),
              format("Das Steuergerät gibt im Schnitt {} % mehr Kraftstoff dazu, als es "
                     "berechnet hat. Es kommt also mehr Luft in den Motor, als gemessen wird, "
                     "oder zu wenig Kraftstoff an. Häufige Gründe: Falschluft durch einen "
                     "undichten Schlauch oder eine Dichtung hinter dem Luftmassenmesser, ein "
                     "verschmutzter Luftmassenmesser oder zu wenig Kraftstoffdruck.",
                     "The control unit adds on average {}% more fuel than it calculated. So "
                     "either more air gets into the engine than is measured, or too little "
                     "fuel arrives. Common reasons: an air leak through a cracked hose or "
                     "gasket after the mass air flow sensor, a dirty mass air flow sensor or "
                     "low fuel pressure.",
                     {value}) + pattern + partial,
              format("Im Leerlauf auf Zischen achten und Unterdruckschläuche ansehen. Eine "
                     "Werkstatt findet Falschluft schnell mit einem Rauchtest.",
                     "Listen for hissing at idle and look at the vacuum hoses. A garage finds "
                     "air leaks quickly with a smoke test."));
    } else {
        c.add(id, level, Area::Mixture,
              format("Gemisch zu fett ({})", "Mixture too rich ({})", {bankName}),
              format("Das Steuergerät nimmt im Schnitt {} % Kraftstoff weg. Es kommt also zu "
                     "viel Kraftstoff oder zu wenig Luft an. Typische Gründe: ein undichtes "
                     "Einspritzventil, zu hoher Kraftstoffdruck, ein hängendes "
                     "Tankentlüftungsventil, ein falsch messender Luftmassenmesser oder "
                     "Kraftstoff im Motoröl.",
                     "The control unit takes away on average {}% fuel. So too much fuel or "
                     "too little air arrives. Typical reasons: a leaking injector, too high "
                     "fuel pressure, a stuck purge valve, a mass air flow sensor that reads "
                     "wrong, or fuel in the engine oil.",
                     {number(magnitude, 1)}) + partial,
              format("Bald prüfen lassen, bevor der Katalysator Schaden nimmt. Ölstand ansehen: "
                     "Riecht das Öl nach Kraftstoff oder steigt der Stand, ist Kraftstoff im Öl.",
                     "Have it checked soon, before the catalytic converter suffers. Look at the "
                     "oil: if it smells of fuel or the level rises, there is fuel in the oil."));
    }
}

// --- Battery and charging --------------------------------------------------

void checkVoltage(Ctx& c) {
    const Phrase chargeTitle = format("Ladespannung", "Charging voltage");
    const Phrase restTitle = format("Batterie bei stehendem Motor", "Battery with engine off");
    const Series* volts = get(c.in, kPidVoltage);
    const Series* rpm = get(c.in, kPidRpm);
    if (!volts) {
        const Phrase why = format("Die Spannung (PID 42) wurde nicht gelesen.",
                                  "The voltage (PID 42) was not read.");
        c.skip("charging", chargeTitle, why);
        c.skip("battery_rest", restTitle, why);
        return;
    }
    if (!rpm) {
        const Phrase why = format("Ohne Drehzahl ist nicht klar, ob der Motor lief.",
                                  "Without engine speed it is unclear whether the engine ran.");
        c.skip("charging", chargeTitle, why);
        c.skip("battery_rest", restTitle, why);
        return;
    }
    std::vector<double> running, off;
    for (const TimedValue& v : *volts) {
        const auto r = nearest(rpm, v.t);
        if (!r) continue;
        if (*r > kRunningRpm) running.push_back(v.value);
        else if (*r < kStoppedRpm) off.push_back(v.value);
    }

    const Phrase sourceNote = format(
        "Gemessen am Diagnosestecker; dort liegt die Spannung oft 0,1 bis 0,3 V unter der an "
        "den Batteriepolen.",
        "Measured at the diagnostic socket, where the voltage is often 0.1 to 0.3 V below the "
        "battery terminals.");

    if (running.empty()) {
        c.skip("charging", chargeTitle,
               format("Der Motor lief während der Messung nicht.",
                      "The engine was not running during the measurement."));
    } else {
        c.ran();
        const double v = median(running);
        const Phrase value = number(v, 2);
        const Phrase smart = format(
            "Autos mit Start-Stopp und intelligentem Lademanagement (oft mit AGM- oder "
            "EFB-Batterie) senken die Ladespannung bei voller Batterie bewusst ab; dort kann "
            "ein Wert um 12,5 bis 13 V normal sein.",
            "Cars with start-stop and smart charging (often with an AGM or EFB battery) "
            "lower the charging voltage on purpose when the battery is full; there a value of "
            "about 12.5 to 13 V can be normal.");
        if (v < limits::kChargingNone) {
            c.add("charging", Level::Critical, Area::Charging,
                  format("Lichtmaschine lädt nicht", "Alternator is not charging"),
                  format("Bei laufendem Motor liegen nur {} V an. Normal sind 13,8 bis 14,8 V. "
                         "Die Batterie wird also nicht geladen, sondern leer gefahren.",
                         "With the engine running there are only {} V. Normal is 13.8 to "
                         "14.8 V. So the battery is not being charged but drained.",
                         {value}) + smart + sourceNote,
                  format("Bald prüfen lassen: Lichtmaschine, Keilrippenriemen und Kabel. Bis "
                         "dahin kann das Auto unterwegs liegen bleiben.",
                         "Have it checked soon: alternator, drive belt and cables. Until then the "
                         "car may stop on the road."));
        } else if (v < limits::kChargingLow) {
            c.add("charging", Level::Warning, Area::Charging,
                  format("Ladespannung niedrig", "Charging voltage low"),
                  format("Bei laufendem Motor liegen {} V an. Normal sind 13,8 bis 14,8 V; "
                         "unter 13,2 V lädt die Lichtmaschine meist zu schwach.",
                         "With the engine running there are {} V. Normal is 13.8 to 14.8 V; "
                         "below 13.2 V the alternator usually charges too weakly.",
                         {value}) + smart + sourceNote,
                  format("Mit dem geführten Batterietest nachmessen, auch mit eingeschaltetem "
                         "Licht und Gebläse.",
                         "Measure again with the guided battery test, also with the lights and "
                         "blower switched on."));
        } else if (v > limits::kChargingTooHigh) {
            c.add("charging", Level::Critical, Area::Charging,
                  format("Ladespannung viel zu hoch", "Charging voltage far too high"),
                  format("Bei laufendem Motor liegen {} V an. Über 15,5 V wird die Batterie "
                         "überladen und Elektronik kann Schaden nehmen. Meist ist der Regler der "
                         "Lichtmaschine defekt.",
                         "With the engine running there are {} V. Above 15.5 V the battery is "
                         "overcharged and electronics can be damaged. Usually the alternator's "
                         "regulator has failed.",
                         {value}),
                  format("Bald in die Werkstatt.", "Have it seen by a garage soon."));
        } else if (v > limits::kChargingHigh) {
            c.add("charging", Level::Warning, Area::Charging,
                  format("Ladespannung hoch", "Charging voltage high"),
                  format("Bei laufendem Motor liegen {} V an. Normal sind 13,8 bis 14,8 V. "
                         "AGM-Batterien werden zeitweise mit bis zu etwa 15 V geladen, "
                         "dauerhaft mehr deutet auf einen defekten Regler hin.",
                         "With the engine running there are {} V. Normal is 13.8 to 14.8 V. "
                         "AGM batteries are sometimes charged at up to about 15 V; more than "
                         "that all the time points to a faulty regulator.",
                         {value}),
                  format("Mit dem geführten Batterietest nachmessen.",
                         "Measure again with the guided battery test."));
        } else {
            c.add("charging", Level::Good, Area::Charging,
                  format("Lichtmaschine lädt", "Alternator is charging"),
                  format("Bei laufendem Motor liegen {} V an, das ist im normalen Bereich.",
                         "With the engine running there are {} V, which is normal.", {value}));
        }
    }

    if (off.empty()) {
        c.skip("battery_rest", restTitle,
               format("Der Motor lief die ganze Zeit. Die Ruhespannung misst der geführte "
                      "Batterietest.",
                      "The engine ran the whole time. The guided battery test measures the "
                      "resting voltage."));
    } else {
        c.ran();
        const double v = median(off);
        const Phrase value = number(v, 2);
        const Phrase loadNote = format(
            "Bei eingeschalteter Zündung ziehen Steuergeräte und Pumpen Strom, der Wert liegt "
            "deshalb etwas unter der echten Ruhespannung.",
            "With the ignition on, control units and pumps draw current, so the value is a "
            "little below the true resting voltage.");
        if (v < limits::kBatteryFlat) {
            c.add("battery_rest", Level::Critical, Area::Charging,
                  format("Batterie fast leer", "Battery almost flat"),
                  format("Bei stehendem Motor liegen nur {} V an. Damit springt das Auto "
                         "womöglich bald nicht mehr an.",
                         "With the engine off there are only {} V. The car may soon fail to "
                         "start.",
                         {value}) + loadNote + sourceNote,
                  format("Batterie laden und danach mit dem geführten Batterietest prüfen. "
                         "Ist sie bald wieder leer, ist sie verbraucht oder etwas zieht Strom "
                         "im Stand.",
                         "Charge the battery and test it with the guided battery test. If it "
                         "goes flat again soon, it is worn out or something drains it while "
                         "parked."));
        } else if (v < limits::kBatteryWeak) {
            c.add("battery_rest", Level::Warning, Area::Charging,
                  format("Batterie schwach", "Battery weak"),
                  format("Bei stehendem Motor liegen {} V an. Eine volle Batterie hat etwa "
                         "12,6 V, unter 12,2 V ist sie nur noch etwa halb geladen oder "
                         "gealtert.",
                         "With the engine off there are {} V. A full battery has about 12.6 V; "
                         "below 12.2 V it is only about half charged or aged.",
                         {value}) + loadNote + sourceNote,
                  format("Eine längere Fahrt machen oder die Batterie laden und dann mit dem "
                         "geführten Batterietest nachmessen.",
                         "Take a longer drive or charge the battery, then measure again with "
                         "the guided battery test."));
        } else {
            c.add("battery_rest", Level::Good, Area::Charging,
                  format("Batteriespannung in Ordnung", "Battery voltage fine"),
                  format("Bei stehendem Motor liegen {} V an.", "With the engine off there are {} V.",
                         {value}) + loadNote);
        }
    }
}

// --- Temperatures ----------------------------------------------------------

// How long the engine has been running, in seconds, if it can be told.
std::optional<double> engineRunTime(const HealthInput& in) {
    if (const Series* rt = get(in, kPidRunTime)) return latest(*rt);
    if (const Series* rpm = get(in, kPidRpm)) {
        double first = 0, last = 0;
        bool any = false;
        for (const TimedValue& v : *rpm) {
            if (v.value <= kRunningRpm) continue;
            if (!any) {
                first = last = v.t;
                any = true;
            }
            first = std::min(first, v.t);
            last = std::max(last, v.t);
        }
        if (any) return last - first;
    }
    return std::nullopt;
}

void checkCooling(Ctx& c) {
    const Phrase thermoTitle = format("Thermostat und Betriebstemperatur",
                                      "Thermostat and operating temperature");
    const Series* coolant = get(c.in, kPidCoolant);
    if (!coolant) {
        c.skip("thermostat", thermoTitle,
               format("Die Kühlmitteltemperatur (PID 05) wurde nicht gelesen.",
                      "The coolant temperature (PID 05) was not read."));
        return;
    }
    c.ran();
    if (minOf(*coolant) <= -39.5) {
        // -40 C is the lowest value the PID can carry; sensors with an open
        // circuit report exactly that.
        c.add("sensor_coolant", Level::Warning, Area::Sensors,
              format("Kühlmittelfühler meldet -40 °C", "Coolant sensor reports -40 °C"),
              format("-40 °C ist der kleinste Wert, den das Steuergerät melden kann. Das passiert "
                     "fast immer, wenn der Fühler oder sein Kabel unterbrochen ist. Dann läuft "
                     "der Motor mit einem Ersatzwert, oft zu fett, und der Lüfter springt "
                     "dauernd an.",
                     "-40 °C is the lowest value the control unit can report. That almost always "
                     "happens when the sensor or its wire is broken. The engine then runs on a "
                     "substitute value, often too rich, and the fan may run all the time."),
              format("Stecker und Kabel des Kühlmittelfühlers prüfen lassen.",
                     "Have the coolant sensor's plug and wire checked."));
        return;
    }

    const double hottest = maxOf(*coolant);
    if (hottest >= limits::kCoolantOverheat) {
        c.add("overheating", Level::Critical, Area::Cooling,
              format("Motor zu heiß", "Engine overheating"),
              format("Das Kühlmittel hat {} °C erreicht. Ab etwa 115 °C droht Schaden an "
                     "Zylinderkopf und Dichtung.",
                     "The coolant reached {} °C. From about 115 °C the cylinder head and "
                     "gasket are at risk.",
                     {number(hottest, 0)}),
              format("Anhalten, Motor abkühlen lassen und Kühlmittelstand prüfen (erst wenn er "
                     "kalt ist, den Deckel öffnen). Nicht weiterfahren, bis die Ursache klar ist.",
                     "Stop, let the engine cool down and check the coolant level (only open the "
                     "cap once it is cold). Do not drive on until the cause is clear."));
    } else if (hottest >= limits::kCoolantHot) {
        c.add("overheating", Level::Warning, Area::Cooling,
              format("Kühlmittel sehr warm", "Coolant very hot"),
              format("Das Kühlmittel hat {} °C erreicht. Viele Motoren laufen unter Last bis "
                     "etwa 105 °C, darüber wird es knapp.",
                     "The coolant reached {} °C. Many engines run up to about 105 °C under load; "
                     "above that it gets tight.",
                     {number(hottest, 0)}),
              format("Kühlmittelstand prüfen und darauf achten, ob der Lüfter anspringt.",
                     "Check the coolant level and watch whether the radiator fan comes on."));
    }

    const std::optional<double> runTime = engineRunTime(c.in);
    const Phrase hottestText = number(hottest, 0);
    if (runTime && *runTime >= limits::kLongRunSeconds) {
        const Phrase minutes = number(*runTime / 60.0, 0);
        if (hottest < limits::kColdThermostat) {
            c.add("thermostat", Level::Warning, Area::Cooling,
                  format("Motor wird nicht warm", "Engine does not warm up"),
                  format("Nach {} Minuten Laufzeit hat das Kühlmittel nur {} °C erreicht. "
                         "Normal sind 80 bis 105 °C. Meist hängt das Thermostat offen: Der "
                         "Motor wird nicht richtig warm, verbraucht mehr und die Heizung bleibt "
                         "lau. Seltener misst der Temperaturfühler falsch. Bei Frost im Leerlauf "
                         "oder bei sparsamen Dieseln kann es allerdings deutlich länger dauern.",
                         "After {} minutes of running the coolant only reached {} °C. Normal is "
                         "80 to 105 °C. Usually the thermostat is stuck open: the engine never "
                         "gets properly warm, uses more fuel and the heater stays lukewarm. Less "
                         "often the temperature sensor reads wrong. In freezing weather at idle, "
                         "or on frugal diesels, it can take much longer though.",
                         {minutes, hottestText}),
                  format("Thermostat und Kühlmittelfühler prüfen lassen. Kein Notfall, aber auf "
                         "Dauer schlecht für Motor und Verbrauch.",
                         "Have the thermostat and coolant sensor checked. Not an emergency, but "
                         "bad for the engine and fuel use in the long run."));
        } else if (hottest < limits::kWarmCoolant) {
            c.add("thermostat", Level::Info, Area::Cooling,
                  format("Motor bleibt eher kühl", "Engine stays rather cool"),
                  format("Nach {} Minuten Laufzeit hat das Kühlmittel {} °C erreicht. Normal "
                         "sind 80 bis 105 °C. Bei kaltem Wetter und ruhiger Fahrt kann das "
                         "noch in Ordnung sein.",
                         "After {} minutes of running the coolant reached {} °C. Normal is 80 "
                         "to 105 °C. In cold weather and with gentle driving this can still be "
                         "fine.",
                         {minutes, hottestText}),
                  format("Bei der nächsten längeren Fahrt noch einmal ansehen.",
                         "Look at it again on the next longer drive."));
        } else {
            c.add("thermostat", Level::Good, Area::Cooling,
                  format("Betriebstemperatur erreicht", "Operating temperature reached"),
                  format("Das Kühlmittel hat {} °C erreicht. Das Thermostat arbeitet.",
                         "The coolant reached {} °C. The thermostat works.", {hottestText}));
        }
    } else if (hottest >= limits::kWarmCoolant) {
        c.add("thermostat", Level::Good, Area::Cooling,
              format("Betriebstemperatur erreicht", "Operating temperature reached"),
              format("Das Kühlmittel hat {} °C erreicht. Das Thermostat arbeitet.",
                     "The coolant reached {} °C. The thermostat works.", {hottestText}));
    } else {
        c.skip("thermostat", thermoTitle,
               runTime
                   ? format("Der Motor lief erst {} Minuten. Für eine Aussage braucht es etwa 15 "
                            "Minuten Fahrt.",
                            "The engine had only run for {} minutes. A verdict needs about 15 "
                            "minutes of driving.",
                            {number(*runTime / 60.0, 0)})
                   : format("Die Laufzeit des Motors ist unbekannt (PID 1F oder Drehzahl fehlt), "
                            "und der Motor war noch nicht warm.",
                            "The engine run time is unknown (PID 1F or engine speed missing), "
                            "and the engine was not warm yet."));
    }
}

void checkOtherTemperatures(Ctx& c) {
    if (const Series* iat = get(c.in, kPidIntakeTemp)) {
        c.ran();
        if (minOf(*iat) <= -39.5) {
            c.add("sensor_intake", Level::Warning, Area::Sensors,
                  format("Ansauglufttemperatur meldet -40 °C", "Intake air temperature reads -40 °C"),
                  format("-40 °C ist der kleinste Wert, den das Steuergerät melden kann. Fast "
                         "immer ist dann der Fühler oder sein Kabel unterbrochen. Das Gemisch "
                         "stimmt dadurch nicht mehr genau.",
                         "-40 °C is the lowest value the control unit can report. Almost always "
                         "the sensor or its wire is broken then. The mixture is no longer "
                         "quite right as a result."),
                  format("Stecker und Kabel des Fühlers prüfen lassen (er sitzt oft im "
                         "Luftmassenmesser).",
                         "Have the sensor's plug and wire checked (it often sits in the mass "
                         "air flow sensor)."));
        }
    }
    if (const Series* oil = get(c.in, kPidOilTemp)) {
        c.ran();
        const double hottest = maxOf(*oil);
        if (hottest >= limits::kOilHot) {
            const Level level = hottest >= limits::kOilOverheat ? Level::Critical : Level::Warning;
            c.add("oil_temp", level, Area::Cooling,
                  format("Öl sehr heiß", "Oil very hot"),
                  format("Das Motoröl hat {} °C erreicht. Normal sind etwa 90 bis 120 °C, bei "
                         "sportlicher Fahrt auch kurz mehr. Ab etwa 150 °C altert das Öl sehr "
                         "schnell und schmiert schlechter.",
                         "The engine oil reached {} °C. Normal is about 90 to 120 °C, briefly "
                         "more when driven hard. From about 150 °C the oil ages very quickly and "
                         "lubricates less well.",
                         {number(hottest, 0)}),
                  format("Ölstand prüfen und den Motor schonen. Bleibt es so, Kühlung prüfen "
                         "lassen.",
                         "Check the oil level and go easy on the engine. If it stays like this, "
                         "have the cooling checked."));
        }
    }
    if (const Series* cat = get(c.in, kPidCatalystTemp)) {
        c.ran();
        const double hottest = maxOf(*cat);
        if (hottest >= limits::kCatalystHot) {
            c.add("catalyst_temp", Level::Warning, Area::Catalyst,
                  format("Katalysator sehr heiß", "Catalytic converter very hot"),
                  format("Das Steuergerät meldet {} °C am Katalysator. Ab etwa 900 °C kann er "
                         "auf Dauer Schaden nehmen. Häufigster Grund sind Zündaussetzer, bei "
                         "denen unverbrannter Kraftstoff im Katalysator nachbrennt.",
                         "The control unit reports {} °C at the catalytic converter. From about "
                         "900 °C it can be damaged over time. The most common reason is "
                         "misfiring, where unburnt fuel burns in the converter.",
                         {number(hottest, 0)}),
                  format("Auf Aussetzer und Fehlercodes achten und hohe Last vermeiden, bis die "
                         "Ursache klar ist.",
                         "Watch for misfires and fault codes and avoid heavy load until the cause "
                         "is clear."));
        }
    }
}

// --- Oxygen sensors --------------------------------------------------------

struct Switching {
    int samples = 0;
    double duration = 0.0;
    int switches = 0;
    double low = 0.0;
    double high = 0.0;
};

// Counts rich/lean changes of a narrowband sensor. A hysteresis around the
// 0.45 V midpoint keeps noise from counting as switching.
Switching analyseSwitching(const Ctx& c, const Series& s) {
    Switching out;
    int state = 0;   // -1 lean, +1 rich, 0 unknown
    double first = 0, last = 0;
    for (const TimedValue& v : s) {
        if (!c.closedLoopWarm(v.t)) continue;
        if (out.samples == 0) {
            first = last = v.t;
            out.low = out.high = v.value;
        }
        ++out.samples;
        first = std::min(first, v.t);
        last = std::max(last, v.t);
        out.low = std::min(out.low, v.value);
        out.high = std::max(out.high, v.value);
        int now = 0;
        if (v.value < 0.3) now = -1;
        else if (v.value > 0.6) now = 1;
        if (now != 0) {
            if (state != 0 && now != state) ++out.switches;
            state = now;
        }
    }
    out.duration = last - first;
    return out;
}

void checkOxygenSensors(Ctx& c) {
    if (c.diesel()) return;   // diesels have no switching sensors
    const Phrase title = format("Lambdasonden", "Oxygen sensors");
    // PIDs 14 and 18 are sensor 1 of bank 1 and 2 (in front of the
    // converter) for the usual sensor layout from PID 13; 15 and 19 are the
    // sensors behind it.
    const int upstream[2] = {0x14, 0x18};
    const int downstream[2] = {0x15, 0x19};
    if (!get(c.in, upstream[0]) && !get(c.in, upstream[1])) {
        c.skip("o2_switching", title,
               format("Keine Spannungen der Lambdasonden (PID 14 bis 1B). Autos mit "
                      "Breitbandsonden melden sie anders, dann ist diese Prüfung hier nicht "
                      "möglich.",
                      "No oxygen sensor voltages (PID 14 to 1B). Cars with wideband sensors "
                      "report them differently, and then this check is not possible here."));
        return;
    }
    for (int bank = 0; bank < 2; ++bank) {
        const Series* up = get(c.in, upstream[bank]);
        if (!up) continue;
        const std::string idSuffix = bank == 0 ? "bank1" : "bank2";
        const Phrase bankName = format("Bank {}", "bank {}", {number(bank + 1, 0)});
        const Switching sw = analyseSwitching(c, *up);
        if (sw.samples < 10 || sw.duration < 10.0) {
            c.skip("o2_switching_" + idSuffix,
                   format("Lambdasonde vorn ({})", "Front oxygen sensor ({})", {bankName}),
                   format("Zu wenige Messwerte bei warmem Motor im geschlossenen Regelkreis. "
                          "Nötig sind mindestens 10 Werte über 10 Sekunden.",
                          "Too few readings with a warm engine in closed loop. At least 10 "
                          "readings over 10 seconds are needed."));
            continue;
        }
        c.ran();
        const Phrase range = format("{} und {} V", "{} and {} V",
                                    {number(sw.low, 2), number(sw.high, 2)});
        if (sw.switches == 0) {
            Phrase cause;
            if (sw.high < 0.3) {
                cause = format("Sie zeigt dauernd mager. Das kann die Sonde selbst sein, aber "
                               "auch Falschluft oder ein Abgasleck vor der Sonde.",
                               "It reads lean all the time. That can be the sensor itself, but "
                               "also an air leak or an exhaust leak in front of the sensor.");
            } else if (sw.low > 0.6) {
                cause = format("Sie zeigt dauernd fett. Das kann die Sonde selbst sein oder "
                               "wirklich zu viel Kraftstoff.",
                               "It reads rich all the time. That can be the sensor itself or "
                               "genuinely too much fuel.");
            } else {
                cause = format("Die Spannung steht fast still. Typisch für eine gealterte oder "
                               "verrußte Sonde.",
                               "The voltage barely moves. Typical of an aged or sooted sensor.");
            }
            c.add("o2_switching_" + idSuffix, Level::Warning, Area::OxygenSensors,
                  format("Lambdasonde vorn regelt nicht ({})", "Front oxygen sensor not switching ({})",
                         {bankName}),
                  format("Bei warmem Motor sollte die Spannung der Sonde vor dem Katalysator "
                         "ständig zwischen etwa 0,1 und 0,9 V hin und her springen. Hier lag "
                         "sie {} Sekunden lang nur zwischen {}.",
                         "With a warm engine the voltage of the sensor in front of the catalytic "
                         "converter should keep jumping between about 0.1 and 0.9 V. Here, for "
                         "{} seconds, it only stayed between {}.",
                         {number(sw.duration, 0), range}) + cause,
                  format("Sonde, Stecker und Kabel prüfen lassen.",
                         "Have the sensor, its plug and wire checked."));
            continue;
        }
        const double rate = sw.switches / sw.duration;
        if (rate < 0.1 && sw.duration >= 30.0) {
            c.add("o2_switching_" + idSuffix, Level::Info, Area::OxygenSensors,
                  format("Lambdasonde vorn eher träge ({})", "Front oxygen sensor rather slow ({})",
                         {bankName}),
                  format("Die Sonde hat in {} Sekunden nur {}-mal gewechselt. Eine frische Sonde "
                         "wechselt bei warmem Motor etwa einmal pro Sekunde. Allerdings fragt der "
                         "Adapter jeden Wert nur ein paar Mal pro Sekunde ab und kann schnelle "
                         "Wechsel verpassen.",
                         "In {} seconds the sensor switched only {} times. A fresh sensor "
                         "switches about once per second with a warm engine. However, the adapter "
                         "polls each value only a few times per second and can miss fast changes.",
                         {number(sw.duration, 0), number(sw.switches, 0)}),
                  format("Bei 2000 bis 2500/min noch einmal messen. Bleibt sie träge, ist sie "
                         "wahrscheinlich gealtert.",
                         "Measure again at 2000 to 2500 rpm. If it stays slow, it has probably "
                         "aged."));
        } else {
            c.add("o2_switching_" + idSuffix, Level::Good, Area::OxygenSensors,
                  format("Lambdasonde vorn regelt ({})", "Front oxygen sensor switching ({})",
                         {bankName}),
                  format("{} Wechsel in {} Sekunden, Spannung zwischen {}.",
                         "{} switches in {} seconds, voltage between {}.",
                         {number(sw.switches, 0), number(sw.duration, 0), range}));
        }

        // Catalyst: a good converter stores oxygen and smooths the signal
        // behind it. If the rear sensor copies the front one, it no longer
        // does that.
        const Series* down = get(c.in, downstream[bank]);
        if (!down || sw.switches < 6) continue;
        const Switching rear = analyseSwitching(c, *down);
        if (rear.samples < 10) continue;
        c.ran();
        const double ratio = static_cast<double>(rear.switches) / sw.switches;
        const Phrase caveat = format(
            "Das ist eine Schätzung aus den Live-Werten. Die genaue Prüfung macht das "
            "Steuergerät selbst (Bereitschaftstest Katalysator und Modus 06).",
            "This is an estimate from live values. The control unit does the exact check "
            "itself (catalyst readiness monitor and mode 06).");
        if (ratio >= 0.7) {
            c.add("catalyst_" + idSuffix, Level::Warning, Area::Catalyst,
                  format("Katalysator speichert wenig Sauerstoff ({})",
                         "Catalytic converter stores little oxygen ({})", {bankName}),
                  format("Die Sonde hinter dem Katalysator wechselt fast so oft wie die davor "
                         "({} zu {} Wechsel). Ein guter Katalysator glättet das Signal "
                         "dahinter. Das deutet auf einen gealterten Katalysator hin.",
                         "The sensor behind the converter switches almost as often as the one "
                         "in front ({} to {} switches). A good converter smooths the signal "
                         "behind it. That points to an aged converter.",
                         {number(rear.switches, 0), number(sw.switches, 0)}) + caveat,
                  format("Auf den Fehler P0420/P0430 achten und vor einem Tausch die hintere "
                         "Sonde und Abgaslecks prüfen lassen.",
                         "Watch for fault P0420/P0430, and before replacing anything have the "
                         "rear sensor and exhaust leaks checked."));
        } else {
            c.add("catalyst_" + idSuffix, Level::Good, Area::Catalyst,
                  format("Katalysator arbeitet ({})", "Catalytic converter working ({})", {bankName}),
                  format("Die Sonde hinter dem Katalysator bleibt ruhig ({} Wechsel gegen {} "
                         "vorn). So soll es sein.",
                         "The sensor behind the converter stays calm ({} switches against {} "
                         "in front). That is how it should be.",
                         {number(rear.switches, 0), number(sw.switches, 0)}) + caveat);
        }
    }
}

// --- Mode 06 ---------------------------------------------------------------

bool isMisfireComponent(const MonitorResult& r) {
    return r.componentId >= 0xA1 && r.componentId <= 0xAD;
}

bool notRunYet(const MonitorResult& r) {
    return r.value == 0.0 && r.minimum == 0.0 && r.maximum == 0.0;
}

Phrase testName(const MonitorResult& r) {
    std::string name = r.name;
    if (!r.component.empty()) name = name.empty() ? r.component : r.component + ": " + name;
    if (name.empty()) {
        char buffer[32];
        std::snprintf(buffer, sizeof buffer, "Test %02X/%02X", r.componentId, r.testId);
        name = buffer;
    }
    return same(name);
}

void checkMode06(Ctx& c) {
    const Phrase title = format("Bauteilprüfungen (Modus 06)", "Component tests (mode 06)");
    if (!c.in.monitorResultsRead) {
        c.skip("mode06", title,
               format("Die Ergebnisse der Bauteilprüfungen wurden nicht gelesen.",
                      "The component test results were not read."));
        return;
    }
    c.ran();
    int evaluated = 0, problems = 0;
    for (const MonitorResult& r : c.in.monitorResults) {
        if (isMisfireComponent(r) || notRunYet(r)) continue;
        ++evaluated;
        char idBuf[32];
        std::snprintf(idBuf, sizeof idBuf, "mode06:%02X:%02X", r.componentId, r.testId);
        const Phrase name = testName(r);
        const Phrase measured = format("Gemessen {} {}, erlaubt {} bis {}.",
                                       "Measured {} {}, allowed {} to {}.",
                                       {number(r.value, 3), same(r.unit),
                                        r.hasMinimum ? number(r.minimum, 3) : same("-"),
                                        r.hasMaximum ? number(r.maximum, 3) : same("-")});
        if (!r.passed) {
            ++problems;
            c.add(idBuf, Level::Warning, Area::Monitoring,
                  format("Prüfung nicht bestanden: {}", "Test failed: {}", {name}),
                  format("Das Steuergerät hat diesen Wert außerhalb seiner Grenzen gemessen. "
                         "Meist folgt bald ein Fehlercode.",
                         "The control unit measured this value outside its limits. A fault "
                         "code usually follows soon.") + measured,
                  format("Das betroffene Bauteil prüfen lassen, bevor die Warnlampe angeht.",
                         "Have the component checked before the warning lamp comes on."));
            continue;
        }
        // Zero limits are usually placeholders ("no lower limit"), so only
        // real limits count for the "close to the limit" check.
        bool near = false;
        if (r.hasMaximum && r.maximum != 0.0 &&
            r.maximum - r.value <= limits::kNearLimitShare * std::fabs(r.maximum)) {
            near = true;
        }
        if (r.hasMinimum && r.minimum != 0.0 &&
            r.value - r.minimum <= limits::kNearLimitShare * std::fabs(r.minimum)) {
            near = true;
        }
        if (near) {
            ++problems;
            c.add(idBuf, Level::Info, Area::Monitoring,
                  format("Knapp an der Grenze: {}", "Close to the limit: {}", {name}),
                  format("Die Prüfung ist bestanden, aber der Wert liegt weniger als 10 % von "
                         "seiner Grenze entfernt. So kündigen sich Fehler oft lange vor der "
                         "Warnlampe an.",
                         "The test passed, but the value is less than 10% away from its limit. "
                         "That is how faults often show up long before the warning lamp.") +
                      measured,
                  format("Bei der nächsten Prüfung wieder ansehen. Wandert der Wert weiter zur "
                         "Grenze, das Bauteil prüfen lassen.",
                         "Look again at the next check. If the value keeps moving towards the "
                         "limit, have the component checked."));
        }
    }
    if (problems == 0) {
        c.add("mode06", Level::Good, Area::Monitoring,
              format("Bauteilprüfungen im grünen Bereich", "Component tests well within limits"),
              evaluated > 0
                  ? format("Alle {} Prüfungen liegen mit Abstand innerhalb ihrer Grenzen.",
                           "All {} tests are comfortably within their limits.",
                           {number(evaluated, 0)})
                  : format("Das Steuergerät hat noch keine Prüfung abgeschlossen.",
                           "The control unit has not completed any test yet."));
    }
}

void checkMisfire(Ctx& c) {
    const Phrase title = format("Zündaussetzer je Zylinder", "Misfires per cylinder");
    if (!c.in.monitorResultsRead) {
        c.skip("misfire", title,
               format("Die Ergebnisse der Bauteilprüfungen (Modus 06) wurden nicht gelesen.",
                      "The component test results (mode 06) were not read."));
        return;
    }
    // CAN mode 06: component A2..AD is cylinder 1..12, test 0B is the
    // average over the last ten drives, test 0C the count of the current or
    // last drive.
    std::map<int, double> perCylinder;
    bool failed = false;
    for (const MonitorResult& r : c.in.monitorResults) {
        if (r.componentId < 0xA2 || r.componentId > 0xAD) continue;
        if (r.testId != 0x0B && r.testId != 0x0C) continue;
        const int cyl = r.componentId - 0xA1;
        perCylinder[cyl] = std::max(perCylinder[cyl], r.value);
        if (!r.passed) failed = true;
    }
    if (perCylinder.empty()) {
        c.skip("misfire", title,
               format("Das Auto meldet keine Aussetzerzähler in Modus 06 (gibt es meist nur bei "
                      "CAN-Fahrzeugen).",
                      "The car reports no misfire counters in mode 06 (usually only on CAN "
                      "vehicles)."));
        return;
    }
    c.ran();
    double total = 0, worst = 0;
    int worstCyl = 0;
    Phrase list;
    for (const auto& [cyl, count] : perCylinder) {
        total += count;
        if (count > worst) {
            worst = count;
            worstCyl = cyl;
        }
        if (count > 0) {
            const Phrase item = format("Zylinder {}: {}", "cylinder {}: {}",
                                       {number(cyl, 0), number(count, 0)});
            if (!list.empty()) {
                list.de += ", ";
                list.en += ", ";
            }
            list.de += item.de;
            list.en += item.en;
        }
    }
    if (total <= 0 && !failed) {
        c.add("misfire", Level::Good, Area::Misfire,
              format("Keine Zündaussetzer", "No misfires"),
              format("Alle {} Zylinder zünden sauber, die Zähler stehen auf null.",
                     "All {} cylinders fire cleanly, the counters are at zero.",
                     {number(static_cast<double>(perCylinder.size()), 0)}));
        return;
    }
    const Level level = (failed || worst >= 20) ? Level::Warning : Level::Info;
    Phrase focus;
    if (total >= 10 && worst >= 0.6 * total) {
        focus = format("Fast alle Aussetzer kommen von Zylinder {}. Das spricht für ein Teil "
                       "genau dieses Zylinders: Zündkerze, Zündspule oder Einspritzventil. Ein "
                       "einfacher Test: Zündspule mit einem Nachbarzylinder tauschen und sehen, "
                       "ob der Fehler mitwandert.",
                       "Almost all misfires come from cylinder {}. That points to a part of "
                       "exactly this cylinder: spark plug, ignition coil or injector. A simple "
                       "test: swap the coil with a neighbouring cylinder and see whether the "
                       "fault moves with it.",
                       {number(worstCyl, 0)});
    }
    c.add("misfire", level, Area::Misfire,
          level == Level::Warning ? format("Zündaussetzer erkannt", "Misfires detected")
                                  : format("Vereinzelte Zündaussetzer", "Occasional misfires"),
          format("Gezählte Aussetzer: {}. Einzelne Aussetzer kommen vor, etwa beim Kaltstart "
                 "oder bei schlechtem Kraftstoff. Viele davon schütteln den Motor und schaden "
                 "dem Katalysator. Ab wann das Steuergerät einen Fehler setzt, legt jeder "
                 "Hersteller selbst fest.",
                 "Misfires counted: {}. A few misfires happen, for example on a cold start or "
                 "with poor fuel. Many of them shake the engine and harm the catalytic "
                 "converter. Each manufacturer decides itself when a fault is set.",
                 {list}) + focus,
          level == Level::Warning
              ? format("Zündkerzen und Zündspulen prüfen lassen.",
                       "Have the spark plugs and ignition coils checked.")
              : format("Im Auge behalten und nach einigen Fahrten wieder prüfen.",
                       "Keep an eye on it and check again after a few drives."));
}

// --- Score -----------------------------------------------------------------

void score(HealthReport& r) {
    int penalty = 0;
    Level worst = Level::Good;
    int warnings = 0, criticals = 0;
    for (const Finding& f : r.findings) {
        worst = worse(worst, f.level);
        switch (f.level) {
            case Level::Info: penalty += kPenaltyInfo; break;
            case Level::Warning: penalty += kPenaltyWarning; ++warnings; break;
            case Level::Critical: penalty += kPenaltyCritical; ++criticals; break;
            default: break;
        }
    }
    int s = std::clamp(100 - penalty, 0, 100);
    // A single serious finding must not hide behind many good ones.
    if (worst == Level::Critical) s = std::min(s, 49);
    else if (worst == Level::Warning) s = std::min(s, 84);
    r.score = s;
    r.grade = worst == Level::Critical ? TrafficLight::Red
            : worst == Level::Warning  ? TrafficLight::Yellow
                                       : TrafficLight::Green;

    switch (r.grade) {
        case TrafficLight::Green:
            r.summary = format("Alles im grünen Bereich. {} Prüfungen gemacht, nichts, was "
                               "Sorgen machen müsste.",
                               "All good. {} checks done, nothing to worry about.",
                               {number(r.checksRun, 0)});
            break;
        case TrafficLight::Yellow:
            r.summary = format("{} Punkte sollten bald angeschaut werden. Nichts davon ist "
                               "akut gefährlich.",
                               "{} points should be looked at soon. None of them is an "
                               "immediate danger.",
                               {number(warnings, 0)});
            if (warnings == 1) {
                r.summary = format("Ein Punkt sollte bald angeschaut werden. Er ist nicht akut "
                                   "gefährlich.",
                                   "One point should be looked at soon. It is not an immediate "
                                   "danger.");
            }
            break;
        case TrafficLight::Red:
            r.summary = criticals == 1
                ? format("Ein Punkt braucht bald Aufmerksamkeit, bitte zuerst ansehen.",
                         "One point needs attention soon, please look at it first.")
                : format("{} Punkte brauchen bald Aufmerksamkeit, bitte zuerst ansehen.",
                         "{} points need attention soon, please look at them first.",
                         {number(criticals, 0)});
            break;
    }
    if (!r.notChecked.empty()) {
        r.summary = r.summary + format("{} Prüfungen waren mangels Daten nicht möglich.",
                                       "{} checks were not possible for lack of data.",
                                       {number(static_cast<double>(r.notChecked.size()), 0)});
    }
}

}  // namespace

HealthReport runHealthCheck(const HealthInput& input) {
    HealthReport report;
    Ctx c{input, report};
    checkFaults(c);
    checkMil(c);
    checkReadiness(c);
    checkHistory(c);
    checkTrims(c, 1);
    checkTrims(c, 2);
    checkOxygenSensors(c);
    checkVoltage(c);
    checkCooling(c);
    checkOtherTemperatures(c);
    checkMode06(c);
    checkMisfire(c);

    std::stable_sort(report.findings.begin(), report.findings.end(),
                     [](const Finding& a, const Finding& b) { return a.level > b.level; });
    score(report);
    return report;
}

}  // namespace caruspex::analysis
