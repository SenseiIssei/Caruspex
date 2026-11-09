// Recording a session with the adapter and playing it back.
//
// RecordingTransport sits between Elm327 and the real transport and writes
// down every write and every read with a timestamp. ReplayTransport reads
// such a file and plays the adapter's part: Elm327 talks to it like to a
// serial port, and every command gets the answer that was recorded for it.
// That makes a session with a real car reproducible on the desk, which helps
// when hunting bugs and when someone sends in a recording of their car.
//
// File format: JSON Lines, one object per line, UTF-8.
//
//   {"format":"caruspex-recording","version":1,"created":"2026-11-09T18:02:11Z",
//    "transport":"COM5","adapter":"ELM327 v1.5","protocol":"...","protocolId":"6"}
//   {"t":0.512,"dir":"tx","data":"ATZ\r"}
//   {"t":0.538,"dir":"rx","data":"ATZ\r\r\rELM327 v1.5\r\r>"}
//   {"t":4.100,"dir":"note","data":"free text, ignored on replay"}
//
// The first line is the header; "adapter", "protocol" and "protocolId" are
// only present when they were known when the recording started (that is,
// when it was started in a running session). "t" counts seconds since
// the start of the recording. "data" carries the bytes exactly as they went
// over the wire: bytes outside printable ASCII are written as \u00XX and
// stand for that single byte, not for a Unicode character. A reader skips
// lines it cannot parse, so a file cut off in the middle of its last line
// (the program crashed, the disk was full) still plays back.
#pragma once

#include <cstdint>
#include <deque>
#include <fstream>
#include <functional>
#include <memory>
#include <string>
#include <vector>

#include "core/Transport.h"

namespace caruspex {

class Elm327;

constexpr const char* kRecordingFormat = "caruspex-recording";
constexpr int kRecordingVersion = 1;

enum class RecordedDirection { Tx, Rx, Note };

struct RecordedEvent {
    double t = 0.0;                         // seconds since the start
    RecordedDirection direction = RecordedDirection::Tx;
    std::string data;                       // raw bytes
};

struct Recording {
    int version = 0;
    std::string created;                    // UTC, ISO 8601
    std::string transport;                  // name of the recorded transport
    std::string adapter;                    // adapter identification, if known
    std::string protocol;                   // protocol text, if known
    std::string protocolId;                 // "6" and so on, if known
    std::vector<RecordedEvent> events;
    int skippedLines = 0;                   // lines that could not be read
    bool truncated = false;                 // the last line was cut off
};

// What the header of a new recording says besides format and date.
struct RecordingMeta {
    std::string adapter;
    std::string protocol;
    std::string protocolId;   // protocol number as the adapter counts, "6" for CAN 11 bit 500k
};

// Reads a recording from text or from a file (UTF-8 path). Returns false
// with an error text if the content is not a Caruspex recording at all;
// single unreadable lines are only counted in skippedLines.
bool parseRecording(const std::string& text, Recording& out, std::string& error);
bool loadRecording(const std::string& path, Recording& out, std::string& error);

// A short overview of a recording.
struct RecordingInfo {
    double durationSeconds = 0.0;
    int exchanges = 0;          // commands sent (bare carriage returns not counted)
    std::string created;
    std::string transport;
    std::string adapter;        // from the header, else from the answer to ATZ or ATI
    std::string protocol;       // from the header, else from the answer to ATDP
    // Protocol number ("6"): from the header, else from the ATSP that
    // preceded the answered ATDP of the connect. Empty if unknown.
    std::string protocolId;
    int skippedLines = 0;
    bool truncated = false;
};

RecordingInfo describeRecording(const Recording& recording);
// The same as a few lines of text in the active language.
std::string recordingSummary(const RecordingInfo& info);

// Normalises a command for comparison: upper case, without spaces and line
// ends. "01 0c\r" and "010C" are the same command.
std::string normalizeCommand(const std::string& command);

// ---------------------------------------------------------------------------
// Recording
// ---------------------------------------------------------------------------

// Wraps another transport and writes every write and every non-empty read
// into a file. Everything is passed through unchanged; if the file cannot
// be opened, the wrapper simply passes through without recording (check
// isRecording()).
class RecordingTransport : public ITransport {
public:
    RecordingTransport(std::unique_ptr<ITransport> inner, const std::string& path,
                       RecordingMeta meta = {});
    ~RecordingTransport() override;

    RecordingTransport(const RecordingTransport&) = delete;
    RecordingTransport& operator=(const RecordingTransport&) = delete;

    bool write(const std::string& data) override;
    std::string read(int timeoutMs) override;
    void flushInput() override;
    std::string name() const override;

    bool isRecording() const { return file_.is_open(); }
    const std::string& error() const { return error_; }
    const std::string& path() const { return path_; }
    // Number of commands written so far.
    int exchanges() const { return exchanges_; }

    // Adds a free-text line to the recording (replay ignores it).
    void note(const std::string& text);

    // Ends the recording and hands back the wrapped transport.
    std::unique_ptr<ITransport> release();

    // If `transport` is a RecordingTransport, ends the recording and returns
    // the transport inside it; otherwise returns `transport` unchanged.
    static std::unique_ptr<ITransport> unwrap(std::unique_ptr<ITransport> transport);

private:
    void writeEvent(const char* direction, const std::string& data);

    std::unique_ptr<ITransport> inner_;
    std::ofstream file_;
    std::string path_;
    std::string error_;
    std::uint64_t startMs_ = 0;
    int exchanges_ = 0;
};

// Starts a recording on an Elm327 that is already connected, without
// reconnecting. A recording started in the middle of a session lacks the
// connect sequence, so a few harmless commands follow at once (ATI, the
// format settings connect already made, ATDP and 0100) and land in the
// file. That way the recording can be replayed on its own. Must be called
// from the thread that uses the Elm327, between two requests.
bool startRecording(Elm327& elm, const std::string& path, std::string& error);
// Ends a recording started with startRecording (or by wrapping the
// transport before connect). Returns false if none was running.
bool stopRecording(Elm327& elm);
// The running recording, or nullptr.
RecordingTransport* activeRecording(Elm327& elm);

// ---------------------------------------------------------------------------
// Replay
// ---------------------------------------------------------------------------

enum class ReplayPacing {
    Fast,       // every answer is there at once
    RealTime,   // answers arrive with the recorded delays
};

// Plays a recording back in place of a real adapter. On every write it looks
// for the next recorded command that matches (see normalizeCommand), first
// from the current position a bounded number of steps forward, then from the
// start of the recording. The second search is what lets a live display poll
// longer than it did while recording: the recorded polls simply repeat. A
// command that was never recorded gets the answer an ELM327 would give to
// something it cannot serve: "?" for AT and ST commands and "NO DATA" for
// requests. Three AT commands are answered the way any ELM327 would, since
// a recording started in a running session does not contain them: ATZ, ATWS
// and ATI give the recorded adapter identification, and ATSP / ATTP give
// "OK". If the recorded protocol is known, a request after an unrecorded
// ATSP for a different protocol gets "UNABLE TO CONNECT", so Elm327::connect
// steps on to the recorded protocol as it would on the car.
class ReplayTransport : public ITransport {
public:
    // Loads the file; check isOpen() and error() afterwards.
    explicit ReplayTransport(const std::string& path, ReplayPacing pacing = ReplayPacing::Fast);
    // Plays a recording already in memory. displayName shows up in name().
    ReplayTransport(Recording recording, std::string displayName,
                    ReplayPacing pacing = ReplayPacing::Fast);

    bool write(const std::string& data) override;
    std::string read(int timeoutMs) override;
    void flushInput() override;
    std::string name() const override;

    bool isOpen() const { return open_; }
    const std::string& error() const { return error_; }
    const Recording& recording() const { return recording_; }
    RecordingInfo info() const { return describeRecording(recording_); }
    // The protocol order to hand to Elm327::connect for this recording: the
    // recorded protocol if known, otherwise `fallback`.
    std::string protocolOrder(const std::string& fallback) const {
        return protocolId_.empty() ? fallback : protocolId_;
    }

    // How far forward from the current position a command is looked for
    // before the search starts over at the beginning.
    static constexpr int kSearchWindow = 64;

    // Statistics, mainly for tests and the log.
    int matched() const { return matched_; }
    int unmatched() const { return unmatched_; }
    int wrapped() const { return wrapped_; }

private:
    struct Chunk {
        double offset;          // seconds after the command
        std::string data;
    };
    struct Exchange {
        std::string command;    // normalised
        std::vector<Chunk> answer;
    };
    struct Pending {
        std::uint64_t readyAtMs;
        std::string data;
    };

    void build();
    int find(const std::string& command);
    std::string fallbackAnswer(const std::string& command) const;

    Recording recording_;
    std::string displayName_;
    ReplayPacing pacing_;
    bool open_ = false;
    std::string error_;
    std::string adapterId_;
    std::string protocolId_;
    std::vector<Exchange> exchanges_;
    size_t position_ = 0;
    std::deque<Pending> pending_;
    bool wrongProtocol_ = false;
    int matched_ = 0, unmatched_ = 0, wrapped_ = 0;
};

}  // namespace caruspex
