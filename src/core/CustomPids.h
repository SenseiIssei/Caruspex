// Custom values: anything a car offers beyond the standard OBD PIDs.
//
// Makers keep the interesting values behind their own identifiers: service
// 22 (UDS, read data by identifier) or service 21 (KWP2000, read data by
// local identifier), often on a control unit other than the engine. The
// battery management of an electric car is the typical case: state of
// health, cell voltages and temperatures are nowhere in standard OBD.
//
// A custom value says whom to ask (header), what to ask (request) and how to
// turn the answer into a number (equation). The format is the one of Torque
// Pro, which the community uses to share such values, so existing lists can
// be imported directly:
//
//     Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header
//     State of health,SOH,220105,((Z<8)+AA)/10,0,100,%,7E4
//
// The equation sees the data bytes of the positive answer as A, B, C, ...
// Z, AA, AB, ... where A is the first byte after the echoed service and
// identifier (for 220105 the answer starts 62 01 05, so A is the byte after
// that).
//
// Supported in equations (case does not matter):
//   numbers        12, 0.5, .25
//   bytes          A .. Z, AA .. ZZ
//   arithmetic     + - * / and parentheses, unary minus
//   shifts         x<n and x>n shift the integer part of x by n bits. Not
//                  in the official Torque documentation, but used by the
//                  widely shared EV lists and accepted by Torque. They bind
//                  weaker than + and -, as in C: (A<8)+B, not A<8+B.
//   bits           {A:3} is bit 3 of A (0 or 1); BIT(x:n) does the same for
//                  any expression
//   functions      SIGNED(x) or SIGNED8(x): x as 8-bit two's complement
//                  SIGNED16(x), SIGNED24(x), SIGNED32(x): the same with more bits
//                  INT16(a:b), INT24(a:b:c), INT32(a:b:c:d): bytes joined,
//                  most significant first, unsigned
//                  MIN(a:b), MAX(a:b), ABS(x), INT(x) (drops the fraction)
// Not supported, and reported as an error: val{...} (references to other
// values), the averaging and delay functions, LOOKUP, FLOAT32.
//
// Our own lists may carry a ninth column "Response" with the id the control
// unit answers on. It matters where it is not header + 8, for example on the
// Renault-Nissan battery controller (79B answers on 7BB).
#pragma once

#include <cstdint>
#include <memory>
#include <string>
#include <vector>

#include "core/Elm327.h"
#include "core/Obd2.h"
#include "data/Pids.h"

namespace caruspex {

// Keys of custom values in LiveValue::key. OBD keys are pid | (sub << 8) and
// stay far below these ranges, so both can share one series map.
constexpr int kPackKeyBase = 0x100000;   // values of the built-in packs
constexpr int kUserKeyBase = 0x200000;   // values imported from a file
constexpr bool isCustomKey(int key) { return key >= kPackKeyBase; }

struct CustomPid {
    std::string name;
    std::string shortName;
    std::string request;    // hex, normalized: "220101", "2101", "015C"
    std::string header;     // request id, e.g. "7E4"; empty = functional OBD addressing
    std::string response;   // response id; empty = header + 8 (11 bit) or the
                            // swapped 29-bit address (18DA44F1 -> 18DAF144)
    std::string equation;
    double minValue = 0.0;
    double maxValue = 0.0;
    std::string unit;
    std::string pack;       // id of the built-in pack, empty for imported values
    std::string source;     // where the definition comes from
    int key = 0;            // series id, see kPackKeyBase
    PidCategory category = PidCategory::Custom;
};

// A parsed equation. Parse once, evaluate for every answer.
class TorqueEquation {
public:
    TorqueEquation() = default;
    explicit TorqueEquation(const std::string& text);

    bool ok() const { return program_ != nullptr; }
    // Why parsing failed, active language. Empty if ok().
    const std::string& error() const { return error_; }
    const std::string& text() const { return text_; }
    // Number of data bytes the equation reads at most (highest letter + 1).
    int bytesNeeded() const { return bytesNeeded_; }

    // Evaluates against the data bytes (A = data[0]). False with a message
    // if a byte is missing, a division by zero happens or the result is not
    // a finite number.
    bool evaluate(const std::vector<int>& data, double& value, std::string& error) const;

    // Index of a byte name: "A" -> 0, "Z" -> 25, "AA" -> 26. -1 if it is not
    // one or two letters.
    static int byteIndex(const std::string& name);

    // The parsed form, defined in the .cpp. Shared, so copies are cheap.
    struct Program;

private:
    std::shared_ptr<const Program> program_;
    int bytesNeeded_ = 0;
    std::string text_;
    std::string error_;
};

// The request as bytes: "0x220101" -> 22 01 01. False if it is not 1 to 7
// whole hex bytes.
bool parseRequestHex(const std::string& text, std::vector<int>& bytes);

// Reads a Torque CSV. A header line is optional; if present, the columns are
// found by name (Name, ShortName, ModeAndPID, Equation, Min Value, Max Value,
// Units, Header, Response; anything else is ignored), otherwise they are
// taken in that order. Lines starting with '#' are comments. Fields may be
// quoted ("a, b" and "say ""hi"""). A line that does not make a valid value
// is skipped and described in errors ("Zeile 7: ..."), the rest is still
// read. Keys are keyBase + index of the value in the result.
std::vector<CustomPid> parseTorqueCsv(const std::string& text, std::vector<std::string>& errors,
                                      int keyBase = kUserKeyBase);

// Writes values in the same format, with a header line. The Response column
// is added only if a value has one.
std::string writeTorqueCsv(const std::vector<CustomPid>& pids);

// One value read once, the simple way. Addresses the header (if any),
// restores the default addressing afterwards. For more than one value use
// CustomPidReader, which is much faster.
bool readCustom(Elm327& elm, const CustomPid& pid, LiveValue& out, std::string& error);

// Result of one value in CustomPidReader::readAll.
struct CustomReading {
    bool ok = false;
    LiveValue value;
    std::string error;
};

// Reads custom values over and over, as the live view does. It remembers
// which control unit the adapter is pointed at, so consecutive values of the
// same header cost no extra AT commands, and it parses every equation only
// once. The destructor puts the adapter back on the default OBD addressing.
class CustomPidReader {
public:
    explicit CustomPidReader(Elm327& elm);
    ~CustomPidReader();

    CustomPidReader(const CustomPidReader&) = delete;
    CustomPidReader& operator=(const CustomPidReader&) = delete;

    bool read(const CustomPid& pid, LiveValue& out, std::string& error);

    // All values, in the order given. Values sharing a header and a request
    // are served from one answer (the battery management of an EV puts some
    // thirty values into one), and values of the same control unit are
    // asked one after another to save re-addressing.
    std::vector<CustomReading> readAll(const std::vector<CustomPid>& pids);

    // Back to the default OBD addressing now (also done by the destructor).
    void restoreDefault();

private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};

}  // namespace caruspex
