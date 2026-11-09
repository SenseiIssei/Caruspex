#include "core/AdapterProbe.h"

#include <algorithm>
#include <cctype>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <iterator>

namespace caruspex {
namespace {

// What a correct answer to a check looks like.
enum class Expect {
    Ok,        // "OK"
    Table,     // a table of "nn:vv" entries (ATPPS)
    OnOff,     // "ON" or "OFF" (ATIGN)
    HexByte,   // one hex byte (ATAMC)
};

struct CheckSpec {
    const char* command;
    int level;
    Expect expect;
    Text purpose;
};

// Version-dependent commands, oldest first. Every one is read-only or sets
// the value that Elm327::connect (or the power-on default) has already set,
// so running them changes nothing. Versions follow the ELM327 data sheet.
const CheckSpec kChecks[] = {
    {"ATPPS", 110, Expect::Table,
     {"Übersicht der programmierbaren Parameter (nur lesen)",
      "Summary of the programmable parameters (read only)"}},
    {"ATKW1", 120, Expect::Ok,
     {"Schlüsselbytes beim K-Line-Aufbau prüfen (Standardwert)",
      "Check key words on K-Line init (default value)"}},
    {"ATS0", 130, Expect::Ok,
     {"Leerzeichen in Antworten weglassen (schon eingestellt)",
      "Leave out spaces in responses (already set)"}},
    {"ATD0", 130, Expect::Ok,
     {"CAN-Datenlänge nicht anzeigen (Standardwert)", "Do not show the CAN data length (default)"}},
    {"ATFE", 131, Expect::Ok, {"Gemerkte Ereignisse vergessen", "Forget events"}},
    {"ATCSM1", 140, Expect::Ok,
     {"Stilles Mithören auf dem CAN-Bus (Standardwert)", "Silent CAN monitoring (default)"}},
    {"ATIGN", 140, Expect::OnOff, {"Zündungseingang lesen", "Read the ignition input"}},
    {"ATCRA", 142, Expect::Ok,
     {"CAN-Empfangsfilter zurücksetzen (Standardwert)", "Reset the CAN receive filter (default)"}},
    {"ATJHF1", 142, Expect::Ok,
     {"J1939-Kopfzeilen formatieren (Standardwert)", "Format J1939 headers (default)"}},
    {"ATAMC", 200, Expect::HexByte,
     {"Zähler der Busaktivität anzeigen", "Show the activity monitor count"}},
};

// The highest level the checks above can tell apart.
constexpr int kHighestTested = 200;

// Versions ELM Electronics actually released, as far as Caruspex knows.
const int kReleased[] = {100, 110, 120, 121, 130, 131, 140, 142, 200, 210, 220, 230};

std::string firstLine(const ElmResult& result) {
    return result.lines.empty() ? std::string() : result.lines.front();
}

std::string upper(std::string s) {
    for (char& c : s) c = static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    return s;
}

bool answeredAsExpected(Expect expect, const std::string& line) {
    const std::string up = upper(line);
    switch (expect) {
        case Expect::Ok: return up == "OK";
        case Expect::Table: return up.find(':') != std::string::npos;
        case Expect::OnOff: return up == "ON" || up == "OFF";
        case Expect::HexByte:
            return !up.empty() && up.size() <= 2 &&
                   std::all_of(up.begin(), up.end(),
                               [](char c) { return std::isxdigit(static_cast<unsigned char>(c)); });
    }
    return false;
}

// "14.2" for English, "14,2" for German.
BilingualText number(double value, int decimals) {
    char buffer[32];
    std::snprintf(buffer, sizeof(buffer), "%.*f", decimals, value);
    std::string en = buffer;
    std::string de = en;
    std::replace(de.begin(), de.end(), '.', ',');
    return {de, en};
}

BilingualText both(const std::string& de, const std::string& en) { return {de, en}; }

double millisecondsSince(std::chrono::steady_clock::time_point start) {
    return std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start)
        .count();
}

// Average round trip of a request, counting only answers that carry data.
double measureRoundTrip(Elm327& elm, const std::string& command, int tries, int& good) {
    double total = 0.0;
    good = 0;
    for (int i = 0; i < tries; ++i) {
        const auto start = std::chrono::steady_clock::now();
        const ElmResult raw = elm.send(command, 3000);
        const double elapsed = millisecondsSince(start);
        if (raw.status == ElmStatus::AdapterError) continue;
        if (!decodeMessages(raw.lines, 0x01).ok()) continue;
        total += elapsed;
        ++good;
    }
    return good > 0 ? total / good : 0.0;
}

}  // namespace

std::string versionText(int level) {
    if (level <= 0) return "?";
    const int major = level / 100;
    const int minor = (level / 10) % 10;
    const int letter = level % 10;
    std::string text = "v" + std::to_string(major) + "." + std::to_string(minor);
    if (letter > 0) text += static_cast<char>('a' + letter - 1);
    return text;
}

int parseVersionLevel(const std::string& text) {
    for (size_t i = 0; i + 2 < text.size(); ++i) {
        const bool atStart = i == 0 || text[i - 1] == 'v' || text[i - 1] == 'V' || text[i - 1] == ' ';
        if (!atStart || !std::isdigit(static_cast<unsigned char>(text[i])) || text[i + 1] != '.' ||
            !std::isdigit(static_cast<unsigned char>(text[i + 2])))
            continue;
        int level = (text[i] - '0') * 100 + (text[i + 2] - '0') * 10;
        if (i + 3 < text.size()) {
            const char letter = static_cast<char>(std::tolower(static_cast<unsigned char>(text[i + 3])));
            if (letter >= 'a' && letter <= 'i') level += letter - 'a' + 1;
        }
        return level;
    }
    return 0;
}

AdapterReport probeAdapter(Elm327& elm) {
    AdapterReport report;
    auto fact = [&](const std::string& de, const std::string& en) { report.facts.push_back({de, en}); };

    // --- Identity -----------------------------------------------------------
    const ElmResult id = elm.send("ATI", 2000);
    if (id.status == ElmStatus::AdapterError || id.lines.empty()) {
        report.verdict = both("Der Adapter antwortet nicht. Ein Test ist so nicht möglich.",
                              "The adapter does not answer, so it cannot be tested.");
        return report;
    }
    report.reachable = true;
    report.versionText = firstLine(id);
    report.claimedLevel = parseVersionLevel(report.versionText);
    fact("ATI meldet: " + report.versionText, "ATI reports: " + report.versionText);

    const std::string description = firstLine(elm.send("AT@1", 1500));
    if (!description.empty() && description != "?") {
        report.description = description;
        fact("AT@1 meldet: " + description, "AT@1 reports: " + description);
    }

    if (report.claimedLevel > 0) {
        report.claimedVersionExists =
            std::find(std::begin(kReleased), std::end(kReleased), report.claimedLevel) !=
            std::end(kReleased);
        const std::string v = versionText(report.claimedLevel);
        if (report.claimedLevel == 150) {
            fact("Die Version v1.5 hat ELM Electronics nie veröffentlicht. Sie steht fast nur "
                 "auf Nachbauten.",
                 "ELM Electronics never released a v1.5. It appears almost only on clones.");
        } else if (!report.claimedVersionExists) {
            fact("Die gemeldete Version " + v + " ist unter den Caruspex bekannten Versionen "
                 "des Originals nicht dabei.",
                 "The reported version " + v + " is not among the original versions known to "
                 "Caruspex.");
        }
    }

    // --- A command that does not exist ---------------------------------------
    // The original chip answers "?". Some clones say "OK" to anything, and
    // then every "OK" below proves nothing.
    const std::string bogus = upper(firstLine(elm.send("ATQX", 1500)));
    report.acceptsUnknownCommands = bogus == "OK";
    if (report.acceptsUnknownCommands) {
        fact("Auf den erfundenen Befehl ATQX antwortet der Adapter mit OK statt mit ?.",
             "The adapter answers the made-up command ATQX with OK instead of ?.");
    }

    // --- STN chip? -----------------------------------------------------------
    const std::string sti = firstLine(elm.send("STI", 1500));
    if (upper(sti).rfind("STN", 0) == 0) {
        report.isStn = true;
        report.stnId = sti;
        fact("STI meldet: " + sti, "STI reports: " + sti);
        const std::string device = firstLine(elm.send("STDI", 1500));
        if (!device.empty() && device != "?") {
            report.stnDevice = device;
            fact("STDI meldet: " + device, "STDI reports: " + device);
        }
    }

    // --- Voltage -------------------------------------------------------------
    const std::string rv = firstLine(elm.send("ATRV", 1500));
    if (!rv.empty() && std::isdigit(static_cast<unsigned char>(rv[0]))) {
        report.voltage = std::strtod(rv.c_str(), nullptr);
        report.hasVoltage = true;
        const BilingualText volts = number(report.voltage, 1);
        fact("Spannung an der OBD-Buchse: " + volts.de + " V",
             "Voltage at the OBD socket: " + volts.en + " V");
    }

    // --- Version-dependent commands ------------------------------------------
    for (const CheckSpec& spec : kChecks) {
        const ElmResult result = elm.send(spec.command, 1500);
        CommandCheck check;
        check.command = spec.command;
        check.introducedIn = spec.level;
        check.response = firstLine(result);
        check.supported = result.status != ElmStatus::AdapterError &&
                          answeredAsExpected(spec.expect, check.response);
        check.purpose = {spec.purpose.de, spec.purpose.en};
        report.checks.push_back(std::move(check));
    }

    // Highest level up to which every check passed. Version 1.0 commands are
    // what Elm327::connect itself relies on, so they count as present.
    report.behavesLikeLevel = 100;
    int firstMissing = 0;
    for (const CommandCheck& check : report.checks) {
        if (!check.supported) {
            firstMissing = check.introducedIn;
            break;
        }
    }
    for (const CommandCheck& check : report.checks) {
        if (firstMissing != 0 && check.introducedIn >= firstMissing) break;
        report.behavesLikeLevel = std::max(report.behavesLikeLevel, check.introducedIn);
    }

    std::string missingDe, missingEn;
    for (const CommandCheck& check : report.checks) {
        if (check.supported) continue;
        const std::string item = check.command + " (" + versionText(check.introducedIn) + ")";
        missingDe += (missingDe.empty() ? "" : ", ") + item;
        missingEn += (missingEn.empty() ? "" : ", ") + item;
    }
    const std::string behaves = versionText(report.behavesLikeLevel);
    if (missingDe.empty()) {
        fact("Alle geprüften Befehle bis v2.0 werden wie beim Original beantwortet.",
             "Every tested command up to v2.0 is answered like the original.");
    } else {
        fact("Befehlssatz wie " + behaves + ". Nicht wie beim Original beantwortet: " + missingDe +
                 ".",
             "Command set like " + behaves + ". Not answered like the original: " + missingEn + ".");
    }

    // --- Round trip ----------------------------------------------------------
    if (elm.connected()) {
        int good = 0;
        report.roundTripMs = measureRoundTrip(elm, "0100", 5, good);
        report.roundTripSamples = good;
        if (good > 0) {
            const BilingualText ms = number(report.roundTripMs, 0);
            fact("Antwortzeit auf 0100: im Schnitt " + ms.de + " ms (" + std::to_string(good) +
                     " Versuche)",
                 "Round trip of 0100: " + ms.en + " ms on average (" + std::to_string(good) +
                     " tries)");
            // The number of expected answers after the request (v1.3 and
            // later) lets the adapter stop waiting after the first answer.
            int hinted = 0;
            report.roundTripWithHintMs = measureRoundTrip(elm, "01001", 3, hinted);
            report.responseCountHint = hinted > 0;
            if (report.responseCountHint) {
                const BilingualText hms = number(report.roundTripWithHintMs, 0);
                fact("Mit Antwortzahl (0100 1): im Schnitt " + hms.de + " ms",
                     "With an answer count (0100 1): " + hms.en + " ms on average");
            }
        }
    }

    // --- Settings for more speed ---------------------------------------------
    if (report.responseCountHint && report.roundTripWithHintMs + 2.0 < report.roundTripMs) {
        const BilingualText a = number(report.roundTripWithHintMs, 0);
        const BilingualText b = number(report.roundTripMs, 0);
        report.speedTips.push_back(
            {"Die Zahl der erwarteten Antworten an die Anfrage hängen (etwa 0100 1). Dann wartet "
             "der Adapter nicht auf weitere Steuergeräte: hier " + a.de + " statt " + b.de + " ms.",
             "Append the number of expected answers to a request (such as 0100 1). The adapter "
             "then stops waiting for further control units: " + a.en + " instead of " + b.en +
                 " ms here."});
    }
    if (report.behavesLikeLevel >= 120 && !report.acceptsUnknownCommands) {
        report.speedTips.push_back(
            {"Die schärfere adaptive Zeitsteuerung ATAT2 ist vorhanden und verkürzt die "
             "Wartezeit weiter. Bei fehlenden Antworten zurück auf ATAT1.",
             "The more aggressive adaptive timing ATAT2 is available and shortens the waiting "
             "further. Go back to ATAT1 if answers go missing."});
    }
    if (report.isStn) {
        report.speedTips.push_back(
            {"STN-Chip: Mit STBR lässt sich die serielle Geschwindigkeit für diese Sitzung "
             "erhöhen, bei USB- und seriellen Verbindungen bis in den Megabit-Bereich. "
             "Bluetooth und WLAN bestimmen ihr Tempo selbst. Dauerhaft gespeichert wird dabei "
             "nichts (das täte erst STSBR). Caruspex stellt das nicht selbst um.",
             "STN chip: STBR raises the serial speed for this session, into the megabit range "
             "on USB and serial connections. Bluetooth and Wi-Fi set their own speed. Nothing "
             "is stored permanently (STSBR would do that). Caruspex does not change it itself."});
        report.speedTips.push_back(
            {"STN-Chips verarbeiten Anfragen deutlich schneller als ELM327-Nachbauten und "
             "puffern mehr. Kürzere Abfrageintervalle sind hier unbedenklich.",
             "STN chips process requests much faster than ELM327 clones and buffer more. Shorter "
             "polling intervals are fine here."});
    }

    // --- Verdict -------------------------------------------------------------
    const std::string claimed = versionText(report.claimedLevel);
    const int comparable = std::min(report.claimedLevel, kHighestTested);
    if (report.acceptsUnknownCommands) {
        report.verdict = both(
            "Der Adapter nennt sich " + report.versionText + ", bestätigt aber auch Befehle, die "
            "es nicht gibt. Welche Funktionen er wirklich beherrscht, lässt sich deshalb nicht "
            "verlässlich prüfen. Das ist typisch für einfache Nachbauten.",
            "The adapter calls itself " + report.versionText + " but also confirms commands that "
            "do not exist. Which functions it really has therefore cannot be tested reliably. "
            "This is typical of simple clones.");
    } else if (report.isStn) {
        const std::string name = report.stnDevice.empty() ? report.stnId : report.stnDevice;
        report.verdict = both(
            "STN-Chip (" + name + "): kein ELM327 und auch kein Nachbau, sondern ein eigener, "
            "kompatibler Chip von ScanTool.net. Er beantwortet die ELM327-Befehle wie " + behaves +
                ".",
            "STN chip (" + name + "): not an ELM327 and not a clone either, but an independent "
            "compatible chip by ScanTool.net. It answers the ELM327 commands like " + behaves +
                ".");
    } else if (report.claimedLevel == 0) {
        report.verdict = both(
            "Der Adapter nennt keine erkennbare ELM327-Version. Er beantwortet die Befehle wie "
            "ein ELM327 " + behaves + ".",
            "The adapter reports no recognizable ELM327 version. It answers the commands like an "
            "ELM327 " + behaves + ".");
    } else if (report.claimedLevel == 150) {
        report.verdict = both(
            "Wahrscheinlich ein Nachbau: Er meldet v1.5, eine Version, die es vom Original nie "
            "gab, und verhält sich wie ein ELM327 " + behaves + ".",
            "Probably a clone: it reports v1.5, a version the original never had, and behaves "
            "like an ELM327 " + behaves + ".");
    } else if (report.behavesLikeLevel < comparable) {
        report.verdict = both(
            "Wahrscheinlich ein Nachbau, der sich als " + claimed + " ausgibt, sich aber wie " +
                behaves + " verhält: Befehle ab " + versionText(firstMissing) +
                " fehlen oder werden anders beantwortet.",
            "Probably a clone that claims " + claimed + " but behaves like " + behaves +
                ": commands from " + versionText(firstMissing) +
                " on are missing or answered differently.");
    } else if (report.behavesLikeLevel > report.claimedLevel) {
        report.verdict = both(
            "Der Adapter beherrscht mehr, als seine Angabe " + claimed + " erwarten lässt (wie " +
                behaves + "). Beim Original wäre das nicht so; vermutlich eine abweichende "
                "Firmware.",
            "The adapter can do more than its stated " + claimed + " suggests (like " + behaves +
                "). The original would not; most likely a different firmware.");
    } else {
        const bool beyond = report.claimedLevel > kHighestTested;
        report.verdict = both(
            "Das Verhalten passt zu einem echten ELM327 " + claimed +
                (beyond ? " (geprüft wurden Befehle bis v2.0)" : "") +
                ". Ein gut gemachter Nachbau lässt sich von außen allerdings nicht sicher davon "
                "unterscheiden.",
            "The behaviour matches a genuine ELM327 " + claimed +
                (beyond ? " (commands up to v2.0 were tested)" : "") +
                ". A well-made clone cannot be told apart from the outside with certainty, "
                "though.");
    }
    return report;
}

}  // namespace caruspex
