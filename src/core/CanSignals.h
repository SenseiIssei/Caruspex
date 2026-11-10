// Values a car broadcasts on its CAN bus by itself, as a DBC file describes
// them: no request, the control units send these frames all the time. Tesla
// is the main case (no standard OBD at all), but many makers broadcast
// battery and drive values the same way.
//
// A signal is written the way DBC files write it, prefixed with "DBC:":
//
//     DBC:0|16@1+(0.01,0)              bits 0 to 15, Intel order, unsigned
//     DBC:16|16@1-(-0.1,0)             signed, factor -0.1
//     DBC:2|12@1+(0.002,0) m1@0|2@1    only in frames whose multiplexer
//                                      (bits 0 and 1) has the value 1
//
// start|length@order sign (factor,offset); order 1 is Intel (little
// endian, start is the lowest bit), 0 is Motorola (big endian, start is the
// highest bit in DBC numbering). The value is raw * factor + offset.
// The request of such a value is "CAN" plus the 11-bit id: "CAN132".
#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace caruspex {

struct CanSignal {
    int start = 0;
    int length = 0;
    bool intel = true;
    bool isSigned = false;
    double factor = 1.0;
    double offset = 0.0;
    // Multiplexed signals: only frames whose multiplexer field has muxValue.
    int muxStart = -1;
    int muxLength = 0;
    bool muxIntel = true;
    int muxValue = -1;
};

// "DBC:..." as above. False with a reason in the active language.
bool parseCanSignal(const std::string& text, CanSignal& out, std::string& error);

// "CAN132" -> 0x132. False if it is not "CAN" plus an 11-bit id in hex.
bool parseMonitorRequest(const std::string& request, int& canId);

// The raw bits of a field, as DBC counts them.
std::uint64_t canBits(const std::vector<int>& frame, int start, int length, bool intel);

// The value of a signal in one frame. False if the frame is too short or a
// multiplexed signal is not in this frame.
bool decodeCanSignal(const CanSignal& signal, const std::vector<int>& frame, double& value);

}  // namespace caruspex
