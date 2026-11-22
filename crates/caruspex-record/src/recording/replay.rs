//! Replay: a transport that answers each command with what was recorded.

use std::collections::VecDeque;

use caruspex_i18n::l;
use caruspex_link::Transport;
use caruspex_link::timing::{now_ms, sleep_ms};

use super::{
    RecordedDirection, Recording, RecordingInfo, describe_recording, file_name_of, is_at_or_st,
    load_recording, normalize_command, protocol_digit, same_request,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplayPacing {
    /// Every answer is there at once.
    #[default]
    Fast,
    /// Answers arrive with the recorded delays.
    RealTime,
}

struct Chunk {
    /// Seconds after the command.
    offset: f64,
    data: Vec<u8>,
}

struct Exchange {
    /// Normalised.
    command: Vec<u8>,
    answer: Vec<Chunk>,
}

struct Pending {
    ready_at_ms: u64,
    data: Vec<u8>,
}

/// Plays a recording back in place of a real adapter. On every write it looks
/// for the next recorded command that matches (see [`normalize_command`]),
/// first from the current position a bounded number of steps forward, then
/// from the start of the recording. The second search is what lets a live
/// display poll longer than it did while recording: the recorded polls simply
/// repeat. A command that was never recorded gets the answer an ELM327 would
/// give to something it cannot serve: "?" for AT and ST commands and "NO
/// DATA" for requests. Three AT commands are answered the way any ELM327
/// would, since a recording started in a running session does not contain
/// them: ATZ, ATWS and ATI give the recorded adapter identification, and ATSP
/// / ATTP give "OK". If the recorded protocol is known, a request after an
/// unrecorded ATSP for a different protocol gets "UNABLE TO CONNECT", so
/// `Elm327::connect` steps on to the recorded protocol as it would on the car.
pub struct ReplayTransport {
    recording: Recording,
    display_name: String,
    pacing: ReplayPacing,
    open: bool,
    error: String,
    adapter_id: String,
    protocol_id: String,
    exchanges: Vec<Exchange>,
    position: usize,
    pending: VecDeque<Pending>,
    wrong_protocol: bool,
    matched: i32,
    unmatched: i32,
    wrapped: i32,
}

impl ReplayTransport {
    /// How far forward from the current position a command is looked for
    /// before the search starts over at the beginning.
    pub const SEARCH_WINDOW: usize = 64;

    fn empty(recording: Recording, display_name: String, pacing: ReplayPacing) -> Self {
        Self {
            recording,
            display_name,
            pacing,
            open: false,
            error: String::new(),
            adapter_id: String::new(),
            protocol_id: String::new(),
            exchanges: Vec::new(),
            position: 0,
            pending: VecDeque::new(),
            wrong_protocol: false,
            matched: 0,
            unmatched: 0,
            wrapped: 0,
        }
    }

    /// Loads the file; check `is_open()` and `error()` afterwards.
    pub fn new(path: &str, pacing: ReplayPacing) -> Self {
        let mut replay = Self::empty(Recording::default(), file_name_of(path), pacing);
        match load_recording(path) {
            Ok(recording) => {
                replay.recording = recording;
                replay.open = true;
                replay.build();
            }
            Err(error) => replay.error = error,
        }
        replay
    }

    /// Plays a recording already in memory. `display_name` shows up in `name()`.
    pub fn from_recording(
        recording: Recording,
        display_name: impl Into<String>,
        pacing: ReplayPacing,
    ) -> Self {
        let mut replay = Self::empty(recording, display_name.into(), pacing);
        replay.open = true;
        replay.build();
        replay
    }

    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn error(&self) -> &str {
        &self.error
    }
    pub fn recording(&self) -> &Recording {
        &self.recording
    }
    pub fn info(&self) -> RecordingInfo {
        describe_recording(&self.recording)
    }
    /// The protocol order to hand to `Elm327::connect` for this recording:
    /// the recorded protocol if known, otherwise `fallback`.
    pub fn protocol_order(&self, fallback: &str) -> String {
        if self.protocol_id.is_empty() {
            fallback.to_string()
        } else {
            self.protocol_id.clone()
        }
    }

    /// Statistics, mainly for tests and the log.
    pub fn matched(&self) -> i32 {
        self.matched
    }
    pub fn unmatched(&self) -> i32 {
        self.unmatched
    }
    pub fn wrapped(&self) -> i32 {
        self.wrapped
    }

    fn build(&mut self) {
        self.exchanges.clear();
        let mut sent_at = 0.0f64;
        for e in &self.recording.events {
            match e.direction {
                RecordedDirection::Tx => {
                    self.exchanges.push(Exchange {
                        command: normalize_command(&e.data),
                        answer: Vec::new(),
                    });
                    sent_at = e.t;
                }
                RecordedDirection::Rx => {
                    // Whatever arrived before the first command (a greeting)
                    // has no command to belong to and is left out.
                    if let Some(last) = self.exchanges.last_mut() {
                        last.answer.push(Chunk {
                            offset: (e.t - sent_at).max(0.0),
                            data: e.data.clone(),
                        });
                    }
                }
                RecordedDirection::Note => {}
            }
        }
        let info = describe_recording(&self.recording);
        self.adapter_id = info.adapter;
        self.protocol_id = info.protocol_id;
        self.position = 0;
    }

    fn find(&mut self, command: &[u8]) -> Option<usize> {
        let count = self.exchanges.len();
        let window_end = count.min(self.position + Self::SEARCH_WINDOW);
        let fits = |recorded: &[u8]| same_request(recorded, command);
        // Forward from where the replay stands, a bounded distance.
        for i in self.position..window_end {
            if fits(&self.exchanges[i].command) {
                self.position = i + 1;
                return Some(i);
            }
        }
        // From the start: polls that run longer than the recording repeat.
        for i in 0..self.position.min(count) {
            if fits(&self.exchanges[i].command) {
                self.wrapped += 1;
                self.position = i + 1;
                return Some(i);
            }
        }
        // Last resort: further ahead than the window reaches.
        for i in window_end..count {
            if fits(&self.exchanges[i].command) {
                self.position = i + 1;
                return Some(i);
            }
        }
        None
    }

    fn fallback_answer(&self, command: &[u8]) -> Vec<u8> {
        let end: &[u8] = b"\r\r>";
        let with_end = |head: &[u8]| [head, end].concat();
        if command.is_empty() {
            return b">".to_vec();
        }
        if command == b"ATZ" || command == b"ATWS" {
            return if self.adapter_id.is_empty() {
                with_end(b"?")
            } else {
                [b"\r\r", self.adapter_id.as_bytes(), end].concat()
            };
        }
        if command == b"ATI" {
            return if self.adapter_id.is_empty() {
                with_end(b"?")
            } else {
                with_end(self.adapter_id.as_bytes())
            };
        }
        if protocol_digit(command) != 0 {
            return with_end(b"OK");
        }
        if is_at_or_st(command) {
            return with_end(b"?");
        }
        if self.wrong_protocol {
            return with_end(b"UNABLE TO CONNECT");
        }
        with_end(b"NO DATA")
    }
}

impl Transport for ReplayTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        if !self.open {
            return false;
        }
        let command = normalize_command(data);
        let now = now_ms();
        // While a wrong protocol is selected, requests do not reach any
        // recorded answer, so they do not move the position either.
        let refused = self.wrong_protocol && !command.is_empty() && !is_at_or_st(&command);
        let index = if refused { None } else { self.find(&command) };

        let digit = protocol_digit(&command);
        if digit != 0 {
            // Only an unrecorded protocol switch needs the bookkeeping: a
            // recorded one comes with its recorded consequences.
            self.wrong_protocol = index.is_none()
                && !self.protocol_id.is_empty()
                && digit != b'A'
                && digit != b'0'
                && self.protocol_id.as_bytes() != [digit];
        }

        let Some(index) = index else {
            self.unmatched += 1;
            let answer = self.fallback_answer(&command);
            self.pending.push_back(Pending {
                ready_at_ms: now,
                data: answer,
            });
            return true;
        };
        self.matched += 1;
        for chunk in &self.exchanges[index].answer {
            let delay = if self.pacing == ReplayPacing::RealTime {
                (chunk.offset * 1000.0) as u64
            } else {
                0
            };
            self.pending.push_back(Pending {
                ready_at_ms: now + delay,
                data: chunk.data.clone(),
            });
        }
        true
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let deadline = now_ms() + timeout_ms;
        loop {
            let mut result = Vec::new();
            let now = now_ms();
            while self.pending.front().is_some_and(|p| p.ready_at_ms <= now) {
                if let Some(p) = self.pending.pop_front() {
                    result.extend(p.data);
                }
            }
            if !result.is_empty() {
                return result;
            }
            if now >= deadline {
                return Vec::new();
            }
            // Like a quiet serial port: wait until something is due or the
            // timeout is over.
            let mut wake = deadline;
            if let Some(p) = self.pending.front()
                && p.ready_at_ms < wake
            {
                wake = p.ready_at_ms;
            }
            sleep_ms(wake - now);
        }
    }

    fn flush_input(&mut self) {
        // Elm327 flushes right before every command; whatever is still queued
        // belongs to the previous one.
        self.pending.clear();
    }

    fn name(&self) -> String {
        format!("{}{}", l("Aufnahme: ", "Recording: "), self.display_name)
    }

    fn count_hint_allowed(&self) -> bool {
        false
    }
}
