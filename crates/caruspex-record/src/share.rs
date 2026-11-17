//! "Mitschnitt teilen": a recording made fit to attach to a GitHub issue.
//!
//! What goes: the last digits of every VIN (the serial number, wherever it
//! appears: as hex in the answers to 09 02, 22 F1 90 or 1A 90, split over
//! several frames or not, and as plain text), the name of the port or the
//! Bluetooth or network address the recording was made on, and the time of
//! day it was made. Recordings hold no location; nothing else about the
//! user is in them. What stays is the traffic itself, which is the point:
//! the control units, their addresses and answers, proven on a real car.
//!
//! [`share_recording`] returns the new file and a preview of what is in it,
//! so the user sees it before the file is saved or attached anywhere.

use std::collections::BTreeMap;

use caruspex_i18n::l;
use caruspex_link::{HeaderFormat, split_messages};

use crate::recording::{
    RecordedDirection, Recording, describe_recording, normalize_command, write_recording,
};

/// How many characters at the end of a VIN are hidden: the serial number
/// (positions 12 to 17). Maker, model, year and plant stay readable, which
/// is what an issue about a car needs.
pub const VIN_HIDDEN: usize = 6;

/// What replaces each hidden character: '0' is a valid VIN character, so
/// the file still replays and decodes like the original.
const MASK: u8 = b'0';

/// What the user sees before sharing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SharePreview {
    pub duration_seconds: f64,
    pub exchanges: i32,
    pub adapter: String,
    pub protocol: String,
    /// Only the date, without the time of day.
    pub created: String,
    /// Every VIN found, as it now reads ("WVWZZZ1KZ6W000000").
    pub vins: Vec<String>,
    /// How many places in the traffic had a VIN.
    pub vin_places: usize,
    /// The port, Bluetooth or network name that was removed, if any.
    pub transport_removed: bool,
    /// Requests by kind, in the active language: ("Live-Werte (01)", 120).
    pub requests: Vec<(String, usize)>,
    /// The first lines of the file to share.
    pub sample: Vec<String>,
    pub bytes: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SharedRecording {
    /// The file to share, in the recording format.
    pub file: Vec<u8>,
    pub preview: SharePreview,
}

fn vin_char(b: u8) -> bool {
    (b.is_ascii_uppercase() && !matches!(b, b'I' | b'O' | b'Q')) || b.is_ascii_digit()
}

/// Looks like a VIN: 17 VIN characters with letters and digits.
fn plausible_vin(bytes: &[u8]) -> bool {
    bytes.len() == 17
        && bytes.iter().all(|&b| vin_char(b))
        && bytes.iter().any(u8::is_ascii_digit)
        && bytes.iter().any(u8::is_ascii_uppercase)
}

/// The answer lines of all exchanges, in order (rx bytes split at the line
/// ends, the prompt dropped).
fn answers(recording: &Recording) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut current = Vec::<u8>::new();
    let flush = |current: &mut Vec<u8>, out: &mut Vec<Vec<String>>| {
        if current.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(current).replace('>', "");
        let lines: Vec<String> = text
            .split(['\r', '\n'])
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        out.push(lines);
        current.clear();
    };
    for e in &recording.events {
        match e.direction {
            RecordedDirection::Tx => flush(&mut current, &mut out),
            RecordedDirection::Rx => current.extend_from_slice(&e.data),
            RecordedDirection::Note => {}
        }
    }
    flush(&mut current, &mut out);
    out
}

/// The VINs in the traffic: in the answers to 09 02 (also the old
/// line-by-line form of K-line and J1850), 22 F1 90 and 1A 90, with and
/// without headers.
pub fn find_vins(recording: &Recording) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let add = |bytes: &[u8], found: &mut Vec<String>| {
        for window in bytes.windows(17) {
            if plausible_vin(window) {
                let vin = String::from_utf8_lossy(window).into_owned();
                if !found.contains(&vin) {
                    found.push(vin);
                }
            }
        }
    };
    for lines in answers(recording) {
        for format in [
            HeaderFormat::None,
            HeaderFormat::Can11,
            HeaderFormat::Can29,
            HeaderFormat::KLine,
            HeaderFormat::J1850,
        ] {
            let messages = split_messages(&lines, format);
            // The old form: one "49 02 0N a b c d" per line, four bytes each.
            let mut legacy = Vec::new();
            for m in &messages {
                let b = &m.bytes;
                if b.len() == 7 && b[0] == 0x49 && b[1] == 0x02 {
                    legacy.extend_from_slice(&b[3..]);
                }
                // Only the answers that carry a VIN, so a calibration id
                // or another text is never taken for one.
                if b.starts_with(&[0x49, 0x02])
                    || b.starts_with(&[0x62, 0xF1, 0x90])
                    || b.starts_with(&[0x5A, 0x90])
                {
                    add(b, &mut found);
                }
            }
            add(&legacy, &mut found);
        }
    }
    found
}

/// Hex digits of one line: (position in the buffer, value).
fn line_digits(buffer: &[u8]) -> Vec<Vec<(usize, u8)>> {
    let mut lines = vec![Vec::new()];
    for (i, &c) in buffer.iter().enumerate() {
        if c == b'\r' || c == b'\n' {
            if !lines.last().is_some_and(Vec::is_empty) {
                lines.push(Vec::new());
            }
        } else if let Some(d) = (c as char).to_digit(16) {
            lines.last_mut().unwrap().push((i, d as u8));
        }
    }
    lines.retain(|l| !l.is_empty());
    lines
}

/// Header and frame bytes a continuation line may start with before the
/// VIN goes on: "7E8 21", "18DAF110 21", "1:", "48 6B 10 49 02 02".
const MAX_PREFIX: usize = 16;
/// A checksum byte at the end of a K-line or J1850 line.
const MAX_SUFFIX: usize = 2;

/// Matches `target` (nibbles) from line `li`, digit `at` on; returns the
/// buffer positions of the matched nibbles.
fn match_from(
    lines: &[Vec<(usize, u8)>],
    li: usize,
    at: usize,
    target: &[u8],
    out: &mut Vec<usize>,
) -> bool {
    if target.is_empty() {
        return true;
    }
    let line = &lines[li];
    let mut k = 0;
    while k < target.len() && at + k < line.len() && line[at + k].1 == target[k] {
        out.push(line[at + k].0);
        k += 1;
    }
    if k == target.len() {
        return true;
    }
    // Only a whole byte may continue on the next line, and only once this
    // line is used up (apart from a checksum).
    let rest = line.len() - (at + k);
    if k % 2 == 0 && rest <= MAX_SUFFIX && li + 1 < lines.len() {
        for skip in 0..=MAX_PREFIX.min(lines[li + 1].len()) {
            let before = out.len();
            if match_from(lines, li + 1, skip, &target[k..], out) {
                return true;
            }
            out.truncate(before);
        }
    }
    out.truncate(out.len() - k);
    false
}

/// Replaces the last [`VIN_HIDDEN`] characters of `vin` wherever its bytes
/// appear as hex in `buffer`. Returns how many places were found.
fn mask_hex(buffer: &mut [u8], vin: &[u8]) -> usize {
    let target: Vec<u8> = vin.iter().flat_map(|&b| [b >> 4, b & 0x0F]).collect();
    let keep = 2 * (vin.len() - VIN_HIDDEN);
    let lines = line_digits(buffer);
    let mut hidden = std::collections::BTreeSet::new();
    let mut places = 0;
    for li in 0..lines.len() {
        for at in 0..lines[li].len() {
            let mut positions = Vec::new();
            if lines[li][at].1 != target[0] || !match_from(&lines, li, at, &target, &mut positions)
            {
                continue;
            }
            // A place counts once, however many ways lead to it.
            let mut new = false;
            for (n, &p) in positions.iter().enumerate().skip(keep) {
                // Each hidden character becomes "30".
                buffer[p] = if n % 2 == 0 { b'3' } else { b'0' };
                new |= hidden.insert(p);
            }
            if new {
                places += 1;
            }
        }
    }
    places
}

/// Replaces the last characters of `vin` where it stands as plain text.
fn mask_text(data: &mut [u8], vin: &[u8]) -> usize {
    let mut places = 0;
    let mut i = 0;
    while i + vin.len() <= data.len() {
        if data[i..i + vin.len()].eq_ignore_ascii_case(vin) {
            for b in &mut data[i + vin.len() - VIN_HIDDEN..i + vin.len()] {
                *b = MASK;
            }
            places += 1;
            i += vin.len();
        } else {
            i += 1;
        }
    }
    places
}

fn masked(vin: &str) -> String {
    let keep: String = vin.chars().take(17 - VIN_HIDDEN).collect();
    format!("{keep}{}", (MASK as char).to_string().repeat(VIN_HIDDEN))
}

/// The kind of a request, for the preview.
fn request_kind(command: &[u8]) -> String {
    if command.starts_with(b"AT") || command.starts_with(b"ST") {
        return l(
            "Adapter-Einstellungen (AT, ST)",
            "Adapter settings (AT, ST)",
        )
        .to_string();
    }
    let service = if command.len() >= 2 && command[..2].iter().all(u8::is_ascii_hexdigit) {
        String::from_utf8_lossy(&command[..2]).into_owned()
    } else {
        String::new()
    };
    let name = match service.as_str() {
        "01" => l("Live-Werte", "Live values"),
        "02" => l("Freeze Frame", "Freeze frame"),
        "03" | "07" | "0A" => l("Fehlerspeicher (OBD)", "Fault memory (OBD)"),
        "04" | "14" => l("Fehler löschen", "Clearing faults"),
        "05" | "06" => l("Testergebnisse", "Test results"),
        "09" => l(
            "Fahrzeugdaten (VIN, Software)",
            "Vehicle data (VIN, software)",
        ),
        "10" | "3E" => l("Diagnosesitzung", "Diagnostic session"),
        "19" | "18" => l("Fehlerspeicher (UDS, KWP)", "Fault memory (UDS, KWP)"),
        "1A" | "21" | "22" => l(
            "Herstellerwerte und Kennungen",
            "Manufacturer values and IDs",
        ),
        "" => l("Sonstiges", "Other"),
        _ => l("Weitere Dienste", "Further services"),
    };
    if service.is_empty() {
        name.to_string()
    } else {
        format!("{name} ({service})")
    }
}

/// The recording made fit to share, see the module text. `known_vins` are
/// VINs the program knows from elsewhere (the connected car); the ones in
/// the traffic are found anyway.
pub fn share_recording(recording: &Recording, known_vins: &[&str]) -> SharedRecording {
    let mut vins: Vec<String> = known_vins
        .iter()
        .map(|v| v.trim().to_ascii_uppercase())
        .filter(|v| plausible_vin(v.as_bytes()))
        .collect();
    for vin in find_vins(recording) {
        if !vins.contains(&vin) {
            vins.push(vin);
        }
    }

    let mut out = recording.clone();
    let transport_removed = !out.transport.is_empty();
    out.transport.clear();
    // The date stays, the time of day goes.
    out.created = out.created.chars().take(10).collect();
    out.skipped_lines = 0;
    out.truncated = false;

    // All rx bytes in one buffer, so a VIN split over two reads is found;
    // masking keeps every length, so the pieces go back where they were.
    let mut rx: Vec<u8> = Vec::new();
    for e in out
        .events
        .iter()
        .filter(|e| e.direction == RecordedDirection::Rx)
    {
        rx.extend_from_slice(&e.data);
    }
    let mut places = 0;
    for vin in &vins {
        places += mask_hex(&mut rx, vin.as_bytes());
        places += mask_text(&mut rx, vin.as_bytes());
    }
    let mut from = 0;
    for e in out.events.iter_mut() {
        if e.direction == RecordedDirection::Rx {
            let n = e.data.len();
            e.data.copy_from_slice(&rx[from..from + n]);
            from += n;
        } else {
            for vin in &vins {
                places += mask_text(&mut e.data, vin.as_bytes());
            }
        }
    }
    for field in [&mut out.adapter, &mut out.protocol] {
        let mut bytes = std::mem::take(field).into_bytes();
        for vin in &vins {
            places += mask_text(&mut bytes, vin.as_bytes());
        }
        *field = String::from_utf8_lossy(&bytes).into_owned();
    }

    let file = write_recording(&out);
    let info = describe_recording(&out);
    let mut requests: BTreeMap<String, usize> = BTreeMap::new();
    for e in out
        .events
        .iter()
        .filter(|e| e.direction == RecordedDirection::Tx)
    {
        let command = normalize_command(&e.data);
        if !command.is_empty() {
            *requests.entry(request_kind(&command)).or_default() += 1;
        }
    }
    let mut requests: Vec<(String, usize)> = requests.into_iter().collect();
    requests.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let sample = String::from_utf8_lossy(&file)
        .lines()
        .take(12)
        .map(str::to_string)
        .collect();
    SharedRecording {
        preview: SharePreview {
            duration_seconds: info.duration_seconds,
            exchanges: info.exchanges,
            adapter: info.adapter,
            protocol: info.protocol,
            created: out.created.clone(),
            vins: vins.iter().map(|v| masked(v)).collect(),
            vin_places: places,
            transport_removed,
            requests,
            sample,
            bytes: file.len(),
        },
        file,
    }
}
