//! The Torque Pro CSV format of custom values, read and written.

use super::equation::TorqueEquation;
use caruspex_i18n::l;
use caruspex_net::can_signals::{parse_can_signal, parse_monitor_request};

use super::{
    CustomPid, format_number, is_hex_text, line_text, parse_number, strip_hex_prefix, trim,
};

/// The request as bytes: "0x220101" -> 22 01 01. `None` if it is not 1 to 7
/// whole hex bytes.
pub fn parse_request_hex(text: &str) -> Option<Vec<u8>> {
    let t = trim(text);
    let mut s = strip_hex_prefix(t).to_string();
    s.retain(|c| c != ' ');
    if !is_hex_text(&s) || !s.len().is_multiple_of(2) || s.len() < 2 || s.len() > 14 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// One CSV record with the line it started on.
struct Record {
    line: usize,
    cells: Vec<String>,
}

fn cell_text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Splits the text into records. Quoted fields may hold commas, doubled
/// quotes and line breaks. A line whose first non-blank character is '#'
/// outside a quoted field is a comment.
fn split_csv(text: &str) -> Vec<Record> {
    let t = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    // A UTF-8 byte order mark, as spreadsheet programs like to write.
    if t.starts_with(&[0xEF, 0xBB, 0xBF]) {
        i = 3;
    }
    while i < t.len() {
        // Start of a line: comment or blank?
        let mut j = i;
        while j < t.len() && (t[j] == b' ' || t[j] == b'\t') {
            j += 1;
        }
        if j < t.len() && t[j] == b'#' {
            while j < t.len() && t[j] != b'\n' {
                j += 1;
            }
            i = j + 1;
            line += 1;
            continue;
        }
        let mut record = Record {
            line,
            cells: Vec::new(),
        };
        let mut cell: Vec<u8> = Vec::new();
        let mut quoted = false;
        let mut done = false;
        while i < t.len() && !done {
            let c = t[i];
            if quoted {
                if c == b'"' {
                    if i + 1 < t.len() && t[i + 1] == b'"' {
                        cell.push(b'"');
                        i += 2;
                        continue;
                    }
                    quoted = false;
                } else {
                    if c == b'\n' {
                        line += 1;
                    }
                    cell.push(c);
                }
                i += 1;
                continue;
            }
            if c == b'"' {
                quoted = true;
            } else if c == b',' {
                record.cells.push(cell_text(std::mem::take(&mut cell)));
            } else if c == b'\n' {
                line += 1;
                done = true;
            } else if c != b'\r' {
                cell.push(c);
            }
            i += 1;
        }
        record.cells.push(cell_text(cell));
        if !record.cells.iter().all(|s| trim(s).is_empty()) {
            out.push(record);
        }
    }
    out
}

const COL_NAME: usize = 0;
const COL_SHORT: usize = 1;
const COL_REQUEST: usize = 2;
const COL_EQUATION: usize = 3;
const COL_MIN: usize = 4;
const COL_MAX: usize = 5;
const COL_UNIT: usize = 6;
const COL_HEADER: usize = 7;
const COL_RESPONSE: usize = 8;
const COLUMN_COUNT: usize = 9;

fn column_for(title: &str) -> Option<usize> {
    let t: String = trim(title)
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !matches!(c, ' ' | '_' | '-'))
        .collect();
    match t.as_str() {
        "name" => Some(COL_NAME),
        "shortname" => Some(COL_SHORT),
        "modeandpid" | "pid" => Some(COL_REQUEST),
        "equation" => Some(COL_EQUATION),
        "minvalue" | "min" => Some(COL_MIN),
        "maxvalue" | "max" => Some(COL_MAX),
        "units" | "unit" => Some(COL_UNIT),
        "header" => Some(COL_HEADER),
        "response" => Some(COL_RESPONSE),
        _ => None,
    }
}

/// "7E4" -> "7E4", "0x7e4" -> "7E4", "DA44F1" -> "18DA44F1". `None` if it is no
/// CAN id; the flag tells whether it is a 29-bit id.
fn normalize_id(text: &str) -> Option<(String, bool)> {
    let mut s = strip_hex_prefix(trim(text)).to_ascii_uppercase();
    if !is_hex_text(&s) {
        return None;
    }
    let bits29 = if s.len() <= 3 {
        if u64::from_str_radix(&s, 16).ok()? > 0x7FF {
            return None;
        }
        while s.len() < 3 {
            s.insert(0, '0');
        }
        false
    } else if s.len() == 6 || s.len() == 8 {
        if s.len() == 6 {
            s = format!("18{s}"); // the usual priority byte
        }
        if u64::from_str_radix(&s, 16).ok()? > 0x1FFF_FFFF {
            return None;
        }
        true
    } else {
        return None;
    };
    Some((s, bits29))
}

/// The id a control unit answers on when nothing else is said.
pub(super) fn default_response(header: &str) -> String {
    let id = u64::from_str_radix(header, 16).unwrap_or(0);
    if header.len() > 3 {
        // 18DA<target><source>: the answer swaps target and source.
        let r = (id & 0xFFFF_0000) | ((id & 0xFF) << 8) | ((id >> 8) & 0xFF);
        format!("{r:08X}")
    } else {
        format!("{:03X}", (id + 8) & 0x7FF)
    }
}

fn csv_field(s: &str) -> String {
    let needs_quotes = s.contains([',', '"', '\r', '\n']) || s.starts_with('#');
    if !needs_quotes {
        return s.to_string();
    }
    let mut out = String::from("\"");
    for c in s.chars() {
        if c == '"' {
            out.push('"');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Reads a Torque CSV. A header line is optional; if present, the columns are
/// found by name (Name, ShortName, ModeAndPID, Equation, Min Value, Max Value,
/// Units, Header, Response; anything else is ignored), otherwise they are
/// taken in that order. Lines starting with '#' are comments. Fields may be
/// quoted ("a, b" and "say ""hi"""). A line that does not make a valid value
/// is skipped and described in the returned errors ("Zeile 7: ..."), the rest
/// is still read. Keys are `key_base` + index of the value in the result
/// ([`USER_KEY_BASE`] for imports).
pub fn parse_torque_csv(text: &str, key_base: i32) -> (Vec<CustomPid>, Vec<String>) {
    let mut out: Vec<CustomPid> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let records = split_csv(text);

    // Column layout: by the header line if there is one, Torque's order otherwise.
    let mut map: [Option<usize>; COLUMN_COUNT] = std::array::from_fn(Some);
    let mut first = 0;
    if let Some(head_record) = records.first() {
        let head = &head_record.cells;
        let is_header = head
            .iter()
            .any(|title| matches!(column_for(title), Some(COL_REQUEST | COL_EQUATION)));
        if is_header {
            map = [None; COLUMN_COUNT];
            for (i, title) in head.iter().enumerate() {
                if let Some(col) = column_for(title)
                    && map[col].is_none()
                {
                    map[col] = Some(i);
                }
            }
            first = 1;
            if map[COL_NAME].is_none() || map[COL_REQUEST].is_none() || map[COL_EQUATION].is_none()
            {
                errors.push(format!(
                    "{}{}",
                    line_text(head_record.line),
                    l(
                        "Kopfzeile ohne die Spalten Name, ModeAndPID und Equation.",
                        "Header line without the columns Name, ModeAndPID and Equation."
                    )
                ));
                return (out, errors);
            }
        }
    }

    for record in &records[first..] {
        let cell = |col: usize| -> String {
            match map[col] {
                Some(i) if i < record.cells.len() => trim(&record.cells[i]).to_string(),
                _ => String::new(),
            }
        };
        let mut reject = |why: String| errors.push(format!("{}{why}", line_text(record.line)));

        let mut pid = CustomPid {
            name: cell(COL_NAME),
            short_name: cell(COL_SHORT),
            equation: cell(COL_EQUATION),
            unit: cell(COL_UNIT),
            ..CustomPid::default()
        };
        if pid.name.is_empty() {
            reject(l("Name fehlt.", "Name is missing.").to_string());
            continue;
        }
        let request = cell(COL_REQUEST);
        if let Some(can_id) = parse_monitor_request(&request) {
            // A value the car broadcasts by itself: "CAN132" with a DBC signal.
            if let Err(why) = parse_can_signal(&pid.equation) {
                reject(format!("{}{why}", l("Gleichung: ", "Equation: ")));
                continue;
            }
            pid.request = format!("CAN{can_id:03X}");
            let min_text = cell(COL_MIN);
            let max_text = cell(COL_MAX);
            if !read_range(&mut pid, &min_text, &max_text) {
                reject(
                    l(
                        "Min oder Max ist keine Zahl.",
                        "Min or max is not a number.",
                    )
                    .to_string(),
                );
                continue;
            }
            if pid.short_name.is_empty() {
                pid.short_name = pid.name.clone();
            }
            pid.key = key_base + out.len() as i32;
            out.push(pid);
            continue;
        }
        let Some(bytes) = parse_request_hex(&request) else {
            reject(if request.is_empty() {
                l("ModeAndPID fehlt.", "ModeAndPID is missing.").to_string()
            } else {
                format!(
                    "{}{request}",
                    l(
                        "ModeAndPID ist keine Anfrage in Hex: ",
                        "ModeAndPID is no hex request: "
                    )
                )
            });
            continue;
        };
        if bytes.len() < 2 {
            reject(
                l(
                    "ModeAndPID braucht Dienst und Kennung, z. B. 220101.",
                    "ModeAndPID needs service and identifier, e.g. 220101.",
                )
                .to_string(),
            );
            continue;
        }
        pid.request = bytes.iter().map(|b| format!("{b:02X}")).collect();

        let equation = TorqueEquation::new(&pid.equation);
        if !equation.ok() {
            reject(format!(
                "{}{}",
                l("Gleichung: ", "Equation: "),
                equation.error()
            ));
            continue;
        }

        let min_text = cell(COL_MIN);
        let max_text = cell(COL_MAX);
        if !read_range(&mut pid, &min_text, &max_text) {
            reject(
                l(
                    "Min oder Max ist keine Zahl.",
                    "Min or max is not a number.",
                )
                .to_string(),
            );
            continue;
        }

        // Header, optionally with the response id: "79B", "79B/7BB", "79B:7BB".
        let mut header = cell(COL_HEADER);
        let mut response = cell(COL_RESPONSE);
        if let Some(split) = header.find(['/', ':']) {
            if response.is_empty() {
                response = header[split + 1..].to_string();
            }
            header.truncate(split);
        }
        let mut header_wide = false;
        if !trim(&header).is_empty() {
            match normalize_id(&header) {
                Some((id, wide)) => {
                    pid.header = id;
                    header_wide = wide;
                }
                None => {
                    reject(format!(
                        "{}{header}",
                        l("Header ist keine CAN-Adresse: ", "Header is no CAN id: ")
                    ));
                    continue;
                }
            }
        }
        if !trim(&response).is_empty() {
            if pid.header.is_empty() {
                reject(l("Response ohne Header.", "Response without a header.").to_string());
                continue;
            }
            match normalize_id(&response) {
                Some((id, wide)) if wide == header_wide => pid.response = id,
                _ => {
                    reject(format!(
                        "{}{response}",
                        l(
                            "Response ist keine passende CAN-Adresse: ",
                            "Response is no matching CAN id: "
                        )
                    ));
                    continue;
                }
            }
        }
        if pid.short_name.is_empty() {
            pid.short_name = pid.name.clone();
        }
        pid.key = key_base + out.len() as i32;
        out.push(pid);
    }
    (out, errors)
}

/// Min and max columns; empty cells keep the default. False if one is not a
/// number.
fn read_range(pid: &mut CustomPid, min_text: &str, max_text: &str) -> bool {
    if !min_text.is_empty() {
        match parse_number(min_text) {
            Some(v) => pid.min_value = v,
            None => return false,
        }
    }
    if !max_text.is_empty() {
        match parse_number(max_text) {
            Some(v) => pid.max_value = v,
            None => return false,
        }
    }
    true
}

/// Writes values in the same format, with a header line. The Response column
/// is added only if a value has one.
pub fn write_torque_csv(pids: &[CustomPid]) -> String {
    let with_response = pids.iter().any(|p| !p.response.is_empty());
    let mut out =
        String::from("Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header");
    if with_response {
        out.push_str(",Response");
    }
    out.push('\n');
    for p in pids {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{}",
            csv_field(&p.name),
            csv_field(&p.short_name),
            csv_field(&p.request),
            csv_field(&p.equation),
            format_number(p.min_value),
            format_number(p.max_value),
            csv_field(&p.unit),
            csv_field(&p.header)
        ));
        if with_response {
            out.push(',');
            out.push_str(&csv_field(&p.response));
        }
        out.push('\n');
    }
    out
}
