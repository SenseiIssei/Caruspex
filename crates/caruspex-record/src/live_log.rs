//! Live values out of a recording, without replaying it.
//!
//! A recording holds the ELM327 dialogue as it went over the wire. This module
//! walks through it, pairs every mode 01 request with its answer, decodes the
//! PIDs with the same table as the live view (`caruspex_data`) and the same
//! splitting of combined answers (`split_multi_pid_answer`), and returns the
//! values as a series of frames. The tuning simulator compares such a series
//! with its model.
//!
//! What the answer text may look like:
//! - headers off (what Caruspex sets): one line per control unit, or an
//!   ISO-TP answer of several frames as "011", "0:...", "1:...";
//! - headers on with 11-bit CAN identifiers ("7E8 06 41 0C ..."), single and
//!   multi frame;
//! - with or without spaces, with the echo of the command.
//!
//! Answers with 29-bit headers or on the K-line with headers on are skipped.

use std::collections::BTreeMap;

use caruspex_data::pids::{decode_pid, find_pid};
use caruspex_protocols::split_multi_pid_answer;

use crate::recording::{RecordedDirection, Recording, normalize_command};

/// The values known at one moment of a recording.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveFrame {
    /// Seconds since the start of the recording.
    pub t: f64,
    /// Value by live key (`caruspex_data::pids::live_key`: PID, plus 256
    /// times the number of the value inside the PID).
    pub values: BTreeMap<i32, f64>,
}

/// How long a value stays valid for the next frames, seconds. The live loop
/// reads PIDs in rounds; a value older than this belongs to another moment.
pub const VALUE_MAX_AGE_S: f64 = 2.0;

/// The mode 01 PIDs of a request such as "010C0B0F" or "01 0C 0B" (the ELM
/// answer count digit at the end, as in "010C1", is ignored).
fn requested_pids(command: &[u8]) -> Option<Vec<i32>> {
    let text = std::str::from_utf8(command).ok()?;
    let rest = text.strip_prefix("01")?;
    let rest = if rest.len() % 2 == 1 {
        &rest[..rest.len() - 1]
    } else {
        rest
    };
    if rest.is_empty() {
        return None;
    }
    let mut pids = Vec::new();
    for pair in rest.as_bytes().chunks(2) {
        let pid = i32::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
        pids.push(pid);
    }
    // Ranges of supported PIDs (0x00, 0x20, ...) carry no values.
    pids.retain(|pid| pid % 0x20 != 0);
    (!pids.is_empty()).then_some(pids)
}

fn hex_bytes(text: &str) -> Option<Vec<u8>> {
    if text.is_empty() || text.len() % 2 == 1 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

/// The messages (service byte first) of one answer text.
fn answer_messages(answer: &[u8], command: &[u8]) -> Vec<Vec<u8>> {
    let text = String::from_utf8_lossy(answer);
    let lines: Vec<String> = text
        .split(['\r', '\n'])
        .map(|line| line.replace([' ', '>', '\t'], "").to_ascii_uppercase())
        .filter(|line| !line.is_empty() && line.as_bytes() != command)
        .collect();

    let mut messages = Vec::new();
    // Headers off, several frames: "011", then "0:...", "1:...".
    let mut pending: Option<(usize, Vec<u8>)> = None;
    // Headers on: one assembly per sender.
    let mut assembling: BTreeMap<String, (usize, Vec<u8>)> = BTreeMap::new();

    for line in &lines {
        if let Some((index, data)) = line.split_once(':') {
            if index.chars().all(|c| c.is_ascii_hexdigit())
                && let (Some((_, bytes)), Some(data)) = (pending.as_mut(), hex_bytes(data))
            {
                bytes.extend(data);
            }
            continue;
        }
        if line.len() == 3 && line.chars().all(|c| c.is_ascii_hexdigit()) {
            if let Some((length, bytes)) = pending.take() {
                messages.push(bytes.into_iter().take(length).collect());
            }
            pending = usize::from_str_radix(line, 16)
                .ok()
                .map(|n| (n, Vec::new()));
            continue;
        }
        if !line.chars().all(|c| c.is_ascii_hexdigit()) {
            // SEARCHING..., NO DATA, BUS INIT: nothing to decode.
            continue;
        }
        // An odd length starting with 7 is an 11-bit identifier ("7E8").
        if line.len() % 2 == 1 && line.len() >= 5 && line.starts_with('7') {
            let (sender, rest) = line.split_at(3);
            let Some(frame) = hex_bytes(rest) else {
                continue;
            };
            let pci = frame[0];
            match pci >> 4 {
                0 => {
                    let length = usize::from(pci & 0x0F);
                    messages.push(frame.iter().skip(1).take(length).copied().collect());
                }
                1 if frame.len() >= 2 => {
                    let length = (usize::from(pci & 0x0F) << 8) | usize::from(frame[1]);
                    assembling.insert(sender.to_string(), (length, frame[2..].to_vec()));
                }
                2 => {
                    if let Some((_, bytes)) = assembling.get_mut(sender) {
                        bytes.extend_from_slice(&frame[1..]);
                    }
                }
                _ => {}
            }
            continue;
        }
        if let Some(bytes) = hex_bytes(line) {
            messages.push(bytes);
        }
    }
    if let Some((length, bytes)) = pending {
        messages.push(bytes.into_iter().take(length).collect());
    }
    for (_, (length, bytes)) in assembling {
        if bytes.len() >= length {
            messages.push(bytes.into_iter().take(length).collect());
        }
    }
    messages
}

/// Decodes one answer to a mode 01 request into (live key, value) pairs.
fn decode_answer(answer: &[u8], command: &[u8], pids: &[i32]) -> Vec<(i32, f64)> {
    let mut out = Vec::new();
    for message in answer_messages(answer, command) {
        if message.first() != Some(&0x41) {
            continue;
        }
        let Some(blocks) = split_multi_pid_answer(&message[1..], pids) else {
            continue;
        };
        for (pid, data) in blocks {
            // The first control unit that answers a PID keeps it, as in the
            // live view.
            if out.iter().any(|(key, _): &(i32, f64)| key & 0xFF == pid) {
                continue;
            }
            let Some(spec) = find_pid(pid) else { continue };
            let raw: Vec<i32> = data.iter().map(|&b| i32::from(b)).collect();
            for reading in decode_pid(spec, &raw) {
                out.push((reading.spec.key, reading.value));
            }
        }
    }
    out
}

/// The values of `latest` that are not older than [`VALUE_MAX_AGE_S`],
/// seen from the newest of them.
fn snapshot(latest: &BTreeMap<i32, (f64, f64)>) -> Option<LiveFrame> {
    let t = latest.values().map(|(at, _)| *at).fold(f64::MIN, f64::max);
    let values: BTreeMap<i32, f64> = latest
        .iter()
        .filter(|(_, (at, _))| t - at <= VALUE_MAX_AGE_S)
        .map(|(key, (_, value))| (*key, *value))
        .collect();
    values
        .contains_key(&0x0C)
        .then_some(LiveFrame { t, values })
}

/// The live values of a recording as frames. The live loop reads its PIDs in
/// rounds; a frame is one round, closed when the engine speed (PID 0x0C)
/// comes in again and at the end of the recording. It carries every value
/// that is not older than [`VALUE_MAX_AGE_S`]. Without engine speed there are
/// no frames.
pub fn live_frames(recording: &Recording) -> Vec<LiveFrame> {
    let mut frames = Vec::new();
    let mut latest: BTreeMap<i32, (f64, f64)> = BTreeMap::new();
    let mut request: Option<(Vec<u8>, Vec<i32>)> = None;
    let mut answer: Vec<u8> = Vec::new();
    let mut answer_t = 0.0;

    let mut finish = |request: &Option<(Vec<u8>, Vec<i32>)>, answer: &[u8], t: f64| {
        let Some((command, pids)) = request else {
            return;
        };
        let values = decode_answer(answer, command, pids);
        if values.iter().any(|(key, _)| *key == 0x0C)
            && let Some(frame) = snapshot(&latest)
        {
            frames.push(frame);
        }
        for (key, value) in values {
            latest.insert(key, (t, value));
        }
    };

    for event in &recording.events {
        match event.direction {
            RecordedDirection::Tx => {
                finish(&request, &answer, answer_t);
                answer.clear();
                let command = normalize_command(&event.data);
                request = requested_pids(&command).map(|pids| (command, pids));
            }
            RecordedDirection::Rx => {
                answer.extend_from_slice(&event.data);
                answer_t = event.t;
            }
            RecordedDirection::Note => {}
        }
    }
    finish(&request, &answer, answer_t);
    frames.extend(snapshot(&latest));
    frames
}
