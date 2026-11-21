//! Live values out of a recording, without replaying it.
//!
//! A recording holds the ELM327 dialogue as it went over the wire. This module
//! walks through it, pairs every mode 01 request with its answer and decodes
//! the answer with the code of the live view: the reply lines as
//! `Elm327::send` keeps them (`reply_lines`), the messages as the driver puts
//! them together (`decode_messages`, ISO-TP per sender included) and the
//! values as `Obd2::read_live_batch` takes them (`mode01_answer_values`: the
//! engine keeps a PID it answers, else the first other control unit). The batched lines of an
//! STN chip ("010C0D|0105") are cut apart like `Elm327::request_many` does.
//! The tuning simulator compares the series of frames with its model.
//!
//! How the adapter printed the answers follows from the dialogue, the way the
//! driver keeps track of it: ATH1 switches headers on, ATH0 and a reset off;
//! ATSP and ATTP, the answers to ATDPN and ATDP, ATCP and the STP presets of
//! an STN chip give the protocol and with it the header format: 11-bit or
//! 29-bit CAN identifiers ("7E8 06 41 0C ...", "18DAF110 06 41 0C ..."), the
//! header bytes of the K-line and J1850 ("48 6B 10 41 0C ...",
//! "83 F1 11 41 0C ..."). Where the recording does not say, because it was
//! started in a running session or written by hand, the format is read off
//! the answer lines.
//!
//! With headers on, the K-line and J1850 end every line with a check byte. A
//! line whose check byte does not match is dropped, and its values with it.
//! The live driver leaves that check to the adapter (an ELM327 marks such a
//! line with "<DATA ERROR"), but a recording may have been cut or edited.

use std::collections::BTreeMap;

use caruspex_link::{
    HeaderFormat, decode_messages, header_format_for, hex_to_bytes, reply_lines, split_batch_answer,
};
use caruspex_protocols::kwp2000::kwp_checksum;
use caruspex_protocols::mode01_answer_values;

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
fn requested_pids(command: &str) -> Option<Vec<i32>> {
    let rest = command.strip_prefix("01")?;
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

/// A protocol number as the adapter counts: '1' to '9', 'A' to 'C'. '0' is
/// the automatic search, which says nothing yet.
fn protocol_digit(text: &str) -> Option<char> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c @ ('1'..='9' | 'A'..='C')), None) => Some(c),
        _ => None,
    }
}

/// The protocol of an ATDP answer such as "AUTO, ISO 15765-4 (CAN 29/500)".
fn protocol_of_description(text: &str) -> Option<char> {
    let text = text.to_ascii_uppercase();
    let found = if text.contains("J1939") {
        'A'
    } else if text.contains("29/250") {
        '9'
    } else if text.contains("29/") {
        '7'
    } else if text.contains("11/250") {
        '8'
    } else if text.contains("11/") {
        '6'
    } else if text.contains("9141") {
        '3'
    } else if text.contains("14230") || text.contains("KWP") {
        '5'
    } else if text.contains("PWM") {
        '1'
    } else if text.contains("VPW") {
        '2'
    } else {
        return None;
    };
    Some(found)
}

/// SAE J1850 CRC: polynomial x^8 + x^4 + x^3 + x^2 + 1, start FF, inverted.
fn j1850_crc(bytes: &[u8]) -> u8 {
    let mut crc = 0xFFu8;
    for &byte in bytes {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x1D
            } else {
                crc << 1
            };
        }
    }
    !crc
}

/// The last byte of a K-line or J1850 line matches the bytes before it:
/// ISO 9141-2 and ISO 14230 add them up, J1850 uses a CRC. With the
/// protocol unknown either one will do.
fn check_byte_ok(bytes: &[u8], protocol: Option<char>) -> bool {
    let Some((&check, body)) = bytes.split_last() else {
        return false;
    };
    let sum = || kwp_checksum(body) == check;
    let crc = || j1850_crc(body) == check;
    match protocol {
        Some('1' | '2') => crc(),
        Some('3' | '4' | '5') => sum(),
        _ => sum() || crc(),
    }
}

fn compact_hex(line: &str) -> Option<String> {
    let digits: String = line
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_hexdigit())).then_some(digits)
}

/// The header format of an answer, read off its lines: an odd number of
/// digits is an 11-bit identifier in front of whole bytes, 18DA a 29-bit
/// one, 48 6B or a format byte with the tester F1 the K-line (or J1850).
/// Headers off is what is left: mode 01 answers start with 41 then.
fn guess_format(lines: &[String], protocol: Option<char>) -> HeaderFormat {
    let kline = if matches!(protocol, Some('1' | '2')) {
        HeaderFormat::J1850
    } else {
        HeaderFormat::KLine
    };
    for line in lines {
        // Indexed lines ("0:...") and length lines ("00B") are headers off;
        // status lines say nothing.
        let Some(digits) = compact_hex(line) else {
            continue;
        };
        if digits.len() < 5 {
            continue;
        }
        if digits.len() % 2 == 1 {
            return HeaderFormat::Can11;
        }
        let bytes = hex_to_bytes(&digits);
        if bytes.len() >= 6 && bytes[0] == 0x18 && matches!(bytes[1], 0xDA | 0xDB) {
            return HeaderFormat::Can29;
        }
        let iso9141_or_vpw = bytes[0] == 0x48 && bytes[1] == 0x6B;
        let kwp = bytes[0] & 0x80 != 0 && bytes[1] == 0xF1;
        // J1850 PWM answers with 41 6B, which also starts a headers-off
        // answer for PID 6B: only with the protocol known.
        let pwm = protocol == Some('1')
            && bytes[0] == 0x41
            && bytes[1] == 0x6B
            && matches!(bytes[3], 0x41 | 0x7F);
        if iso9141_or_vpw || kwp || pwm {
            return kline;
        }
    }
    HeaderFormat::None
}

/// What the dialogue so far says about the adapter's settings.
#[derive(Debug, Default)]
struct Session {
    /// Protocol number as the adapter counts, if known.
    protocol: Option<char>,
    /// Headers on (ATH1), if known.
    headers: Option<bool>,
}

impl Session {
    fn new(recording: &Recording) -> Self {
        Session {
            protocol: protocol_digit(&recording.protocol_id),
            headers: None,
        }
    }

    /// Follows an AT or ST command and the lines the adapter answered.
    fn follow(&mut self, command: &str, lines: &[String]) {
        // Like Elm327::send: by the command alone, whatever came back.
        match command {
            "ATH1" => self.headers = Some(true),
            "ATH0" | "ATZ" | "ATD" | "ATWS" => self.headers = Some(false),
            _ => {}
        }
        if lines.iter().any(|line| line == "?") {
            return;
        }
        if let Some(rest) = command
            .strip_prefix("ATSP")
            .or_else(|| command.strip_prefix("ATTP"))
        {
            self.protocol = protocol_digit(rest.strip_prefix('A').unwrap_or(rest));
        } else if command == "ATDPN" {
            if let Some(found) = lines
                .first()
                .and_then(|line| protocol_digit(line.strip_prefix('A').unwrap_or(line)))
            {
                self.protocol = Some(found);
            }
        } else if command == "ATDP" {
            if let Some(found) = lines.first().and_then(|line| protocol_of_description(line)) {
                self.protocol = Some(found);
            }
        } else if command.starts_with("ATCP") {
            // The priority bits only exist in 29-bit identifiers.
            if !matches!(self.protocol, Some('7' | '9' | 'A' | 'B' | 'C')) {
                self.protocol = Some('7');
            }
        } else if let Some(preset) = command.strip_prefix("STP") {
            // ISO 15765 presets of the STN chips (FRPM 8.6), as in stn.rs.
            match preset {
                "33" | "35" | "53" | "63" => self.protocol = Some('6'),
                "34" | "36" | "54" | "64" => self.protocol = Some('7'),
                _ => {}
            }
        }
    }

    /// The format the driver would have parsed these answer lines with.
    fn header_format(&self, lines: &[String]) -> HeaderFormat {
        match (self.headers, self.protocol) {
            (Some(false), _) => HeaderFormat::None,
            (Some(true), Some(protocol)) if header_format_for(protocol) != HeaderFormat::None => {
                header_format_for(protocol)
            }
            (_, protocol) => guess_format(lines, protocol),
        }
    }
}

/// Decodes the lines of one answer to a mode 01 request into (live key,
/// value) pairs.
fn decode_answer(mut lines: Vec<String>, pids: &[i32], session: &Session) -> Vec<(i32, f64)> {
    let format = session.header_format(&lines);
    if matches!(format, HeaderFormat::KLine | HeaderFormat::J1850) {
        lines.retain(|line| match compact_hex(line) {
            Some(digits) => check_byte_ok(&hex_to_bytes(&digits), session.protocol),
            None => true,
        });
    }
    let result = decode_messages(&lines, 0x01, format);
    mode01_answer_values(pids, &result)
        .into_iter()
        .map(|value| (value.key, value.value))
        .collect()
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

/// Walks through the dialogue and collects the frames.
struct Reader {
    session: Session,
    frames: Vec<LiveFrame>,
    latest: BTreeMap<i32, (f64, f64)>,
}

impl Reader {
    fn take(&mut self, values: Vec<(i32, f64)>, t: f64) {
        if values.iter().any(|(key, _)| *key == 0x0C)
            && let Some(frame) = snapshot(&self.latest)
        {
            self.frames.push(frame);
        }
        for (key, value) in values {
            self.latest.insert(key, (t, value));
        }
    }

    /// One command and everything the adapter answered to it.
    fn exchange(&mut self, command: &str, answer: &[u8], t: f64) {
        if command.starts_with("AT") || command.starts_with("ST") {
            self.session.follow(command, &reply_lines(answer, command));
            return;
        }
        if command.contains('|') {
            // Batched on an STN chip: the answers in the order of the parts.
            let parts = split_batch_answer(&reply_lines(answer, command));
            for (part, lines) in command.split('|').zip(parts) {
                if let Some(pids) = requested_pids(part) {
                    let values = decode_answer(lines, &pids, &self.session);
                    self.take(values, t);
                }
            }
            return;
        }
        if let Some(pids) = requested_pids(command) {
            let values = decode_answer(reply_lines(answer, command), &pids, &self.session);
            self.take(values, t);
        }
    }
}

/// The live values of a recording as frames. The live loop reads its PIDs in
/// rounds; a frame is one round, closed when the engine speed (PID 0x0C)
/// comes in again and at the end of the recording. It carries every value
/// that is not older than [`VALUE_MAX_AGE_S`]. Without engine speed there are
/// no frames.
pub fn live_frames(recording: &Recording) -> Vec<LiveFrame> {
    let mut reader = Reader {
        session: Session::new(recording),
        frames: Vec::new(),
        latest: BTreeMap::new(),
    };
    let mut command: Option<String> = None;
    let mut answer: Vec<u8> = Vec::new();
    let mut answer_t = 0.0;

    for event in &recording.events {
        match event.direction {
            RecordedDirection::Tx => {
                if let Some(command) = &command {
                    reader.exchange(command, &answer, answer_t);
                }
                answer.clear();
                command =
                    Some(String::from_utf8_lossy(&normalize_command(&event.data)).into_owned());
            }
            RecordedDirection::Rx => {
                answer.extend_from_slice(&event.data);
                answer_t = event.t;
            }
            RecordedDirection::Note => {}
        }
    }
    if let Some(command) = &command {
        reader.exchange(command, &answer, answer_t);
    }
    let mut frames = reader.frames;
    frames.extend(snapshot(&reader.latest));
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_bytes_of_the_k_line_and_j1850() {
        // The mode 01 PID 00 requests every OBD write-up shows.
        assert_eq!(kwp_checksum(&[0x68, 0x6A, 0xF1, 0x01, 0x00]), 0xC4);
        assert_eq!(j1850_crc(&[0x68, 0x6A, 0xF1, 0x01, 0x00]), 0x17);
        assert!(check_byte_ok(
            &[0x68, 0x6A, 0xF1, 0x01, 0x00, 0xC4],
            Some('3')
        ));
        assert!(!check_byte_ok(
            &[0x68, 0x6A, 0xF1, 0x01, 0x00, 0xC4],
            Some('2')
        ));
        assert!(check_byte_ok(&[0x68, 0x6A, 0xF1, 0x01, 0x00, 0x17], None));
        assert!(!check_byte_ok(&[0x68, 0x6A, 0xF1, 0x01, 0x00, 0x18], None));
    }

    #[test]
    fn the_protocol_follows_the_dialogue() {
        let mut session = Session::default();
        session.follow("ATSP7", &["OK".into()]);
        assert_eq!(session.protocol, Some('7'));
        session.follow("ATSPA3", &["OK".into()]);
        assert_eq!(session.protocol, Some('3'));
        session.follow("ATSP0", &["OK".into()]);
        assert_eq!(session.protocol, None);
        session.follow("ATDPN", &["A5".into()]);
        assert_eq!(session.protocol, Some('5'));
        session.follow("ATDP", &["AUTO, ISO 15765-4 (CAN 29/500)".into()]);
        assert_eq!(session.protocol, Some('7'));
        session.follow("STP33", &["OK".into()]);
        assert_eq!(session.protocol, Some('6'));
        session.follow("STP99", &["?".into()]);
        assert_eq!(session.protocol, Some('6'));
        session.follow("ATCP18", &["OK".into()]);
        assert_eq!(session.protocol, Some('7'));
        session.follow("ATH1", &["OK".into()]);
        assert_eq!(session.headers, Some(true));
        session.follow("ATZ", &["ELM327 v1.5".into()]);
        assert_eq!(session.headers, Some(false));
    }

    #[test]
    fn the_format_is_read_off_the_lines_when_the_recording_does_not_say() {
        let lines = |text: &[&str]| text.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            guess_format(&lines(&["NO DATA", "7E8 04 41 0C 1A F8"]), None),
            HeaderFormat::Can11
        );
        assert_eq!(
            guess_format(&lines(&["18DAF11004410C1AF8"]), None),
            HeaderFormat::Can29
        );
        assert_eq!(
            guess_format(&lines(&["48 6B 10 41 0C 1A F8 22"]), None),
            HeaderFormat::KLine
        );
        assert_eq!(
            guess_format(&lines(&["84 F1 11 41 0C 1A F8 E5"]), None),
            HeaderFormat::KLine
        );
        assert_eq!(
            guess_format(&lines(&["48 6B 10 41 0C 1A F8 22"]), Some('2')),
            HeaderFormat::J1850
        );
        assert_eq!(
            guess_format(&lines(&["41 6B 10 41 0C 1A F8 00"]), None),
            HeaderFormat::None
        );
        assert_eq!(
            guess_format(&lines(&["41 6B 10 41 0C 1A F8 00"]), Some('1')),
            HeaderFormat::J1850
        );
        assert_eq!(
            guess_format(&lines(&["00B", "0:410BB40F470E", "1:940C2EE0"]), None),
            HeaderFormat::None
        );
        assert_eq!(
            guess_format(&lines(&["410C1AF8"]), None),
            HeaderFormat::None
        );
    }
}
