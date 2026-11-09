// Adapter test: what the connected ELM327 really is and what it really can do.
//
// Nearly every cheap adapter says "ELM327" when asked, and many claim a
// version that their firmware does not implement. The probe therefore does
// not trust the version string. It asks a set of commands whose existence
// depends on the firmware version and checks whether each one is answered
// the way the original chip answers it. From that it derives the command set
// the adapter behaves like and compares it with what it claims.
//
// Everything sent is either read-only or sets a value that Elm327::connect
// has already set to the same value. Nothing writes to the adapter's
// non-volatile memory (no ATPP ... SV/ON/OFF, no AT@3, no STSBR).
//
// The verdict is worded carefully: from the outside a good clone can look
// exactly like the original, and the probe says so instead of pretending to
// know.
#pragma once

#include <string>
#include <vector>

#include "core/Elm327.h"
#include "core/Lang.h"

namespace caruspex {

// A text built at runtime in both languages. get() picks the active one.
struct BilingualText {
    std::string de;
    std::string en;
    const std::string& get() const { return language() == Language::English ? en : de; }
};

// One version-dependent command and how the adapter answered it.
struct CommandCheck {
    std::string command;       // as sent, e.g. "ATCSM1"
    int introducedIn = 0;      // ELM327 version level, see versionText()
    bool supported = false;    // answered like the original chip
    std::string response;      // first response line, as received
    BilingualText purpose;     // what the command is for
};

struct AdapterReport {
    bool reachable = false;          // answered ATI at all

    std::string versionText;         // ATI, e.g. "ELM327 v1.5"
    std::string description;         // AT@1
    int claimedLevel = 0;            // parsed from ATI, 0 if not parseable
    bool claimedVersionExists = true;   // false for v1.5 and other versions never released

    bool isStn = false;              // STN chip (OBDLink and others)
    std::string stnId;               // STI
    std::string stnDevice;           // STDI

    bool hasVoltage = false;
    double voltage = 0.0;            // ATRV, volts

    // Round trip of "0100", averaged over a few tries, in milliseconds.
    // Only measured when Elm327 has a connection to a control unit.
    int roundTripSamples = 0;
    double roundTripMs = 0.0;
    // The same with the expected number of answers appended ("0100 1").
    bool responseCountHint = false;
    double roundTripWithHintMs = 0.0;

    bool acceptsUnknownCommands = false;   // answered "OK" to a command that does not exist
    std::vector<CommandCheck> checks;
    int behavesLikeLevel = 0;        // highest command set fully answered

    std::vector<BilingualText> facts;        // what was observed, plain sentences
    std::vector<BilingualText> speedTips;    // settings this adapter allows for more speed
    BilingualText verdict;                   // hedged conclusion
};

// "v1.4b" for 142, "v2.1" for 210. Levels are major * 100 + minor * 10 +
// letter (a = 1, b = 2).
std::string versionText(int level);

// Parses "ELM327 v1.4b" and similar into a level; 0 if no version is found.
int parseVersionLevel(const std::string& text);

// Runs the test over an open Elm327. Best called right after connect, before
// header or filter settings are changed: the probe resets the CAN receive
// filter (ATCRA) as part of the test.
AdapterReport probeAdapter(Elm327& elm);

}  // namespace caruspex
