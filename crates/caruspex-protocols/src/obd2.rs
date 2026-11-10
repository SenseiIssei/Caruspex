//! EOBD services: read/clear faults, live values, freeze frame, readiness, VIN.
//!
//! Several control units: on CAN the engine (7E8), the transmission (7E9) and
//! sometimes a hybrid controller answer the same request. With headers off
//! their answers are mixed; switch them on with `Elm327::set_headers(true)` and
//! every fault, readiness block, mode 06 and mode 09 result carries the
//! address of its control unit (`ecu_source`) and a plain name (`ecu`).

use std::collections::{BTreeMap, BTreeSet};

use caruspex_data::dtc::{decode_dtc, describe_dtc, is_throttle_dtc};
use caruspex_data::monitor_tests::{
    monitor_component_name, monitor_scale, monitor_test_name, monitor_test_name_can, oxygen_scale,
    oxygen_test_name,
};
use caruspex_data::pids::{
    PidCategory, PidReading, PidSpec, decode_pid, find_pid, monitors_b, monitors_c,
    monitors_c_diesel,
};
use caruspex_i18n::{Language, Text, l, language};
use caruspex_link::elm327::DEFAULT_REQUEST_TIMEOUT_MS;
use caruspex_link::{Elm327, ElmResult, ElmStatus};

use crate::ecu_address::ecu_display_name;

/// How many freeze frames `Obd2::read_freeze_frames` asks for by default.
pub const DEFAULT_FREEZE_FRAMES: usize = 4;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fault {
    pub code: String,
    /// "gespeichert"/"stored", "sporadisch"/"intermittent",
    /// "dauerhaft"/"permanent" (active language)
    pub kind: String,
    pub text: String,
    pub throttle_related: bool,
    /// Which control unit reported it, e.g. "Motor (7E8)" / "Engine (7E8)".
    /// Empty while headers are off.
    pub ecu: String,
    /// Raw address: "7E8", "18DAF110", "10"
    pub ecu_source: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReadinessItem {
    pub name: String,
    pub ready: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VehicleStatus {
    pub mil_on: bool,
    pub fault_count: i32,
    pub readiness: Vec<ReadinessItem>,
    pub valid: bool,
    /// Diesel: different readiness monitors.
    pub compression_ignition: bool,
    /// See [`Fault::ecu`].
    pub ecu: String,
    pub ecu_source: String,
}

/// A result from on-board component monitoring (mode 06).
///
/// This is the most detailed part of EOBD: for every internal test the control
/// unit reports not just passed or failed, but the measured value together
/// with its lower and upper limit. So you can see how close a component is to
/// its limits long before the warning lamp comes on.
///
/// On CAN, `component_id` is the monitor ID (OBDMID, e.g. 0xA2 = misfire
/// cylinder 1) and `test_id` the test ID (TID, e.g. 0x0C = misfires in the
/// current driving cycle). On the K-line, `test_id` is the test number and
/// `component_id` the component or limit ID.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorResult {
    pub test_id: i32,
    pub component_id: i32,
    /// Plain-text name of the test, if known.
    pub name: String,
    /// Plain-text name of the component.
    pub component: String,
    pub value: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub unit: String,
    pub has_minimum: bool,
    pub has_maximum: bool,
    pub passed: bool,
    /// Decoded with the CAN layout (OBDMID/TID).
    pub can: bool,
    /// CAN only: the unit and scaling ID.
    pub unit_and_scaling: i32,
    /// See [`Fault::ecu`].
    pub ecu: String,
    pub ecu_source: String,
}

impl Default for MonitorResult {
    fn default() -> Self {
        Self {
            test_id: 0,
            component_id: 0,
            name: String::new(),
            component: String::new(),
            value: 0.0,
            minimum: 0.0,
            maximum: 0.0,
            unit: String::new(),
            has_minimum: false,
            has_maximum: false,
            passed: true,
            can: false,
            unit_and_scaling: 0,
            ecu: String::new(),
            ecu_source: String::new(),
        }
    }
}

/// Misfire counts of one cylinder (or of the whole engine), taken from the
/// mode 06 results on CAN: monitor 0xA1 is the engine as a whole, 0xA2 to
/// 0xAD are cylinders 1 to 12. Test 0x0B is the weighted average over the
/// last ten driving cycles, test 0x0C the count of the current (or last)
/// driving cycle.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MisfireCount {
    /// 0 = whole engine
    pub cylinder: i32,
    /// "Zylinder 3" / "Cylinder 3", "Motor gesamt"
    pub name: String,
    pub current_cycle: f64,
    pub average_ten_cycles: f64,
    pub has_current: bool,
    pub has_average: bool,
    /// A plain sentence for non-mechanics.
    pub text: String,
    pub ecu: String,
    pub ecu_source: String,
}

/// Result of an oxygen sensor test (mode 05). Only exists on vehicles without
/// CAN, i.e. exactly the older ones like the Audi A2.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OxygenSensorTest {
    pub test_id: i32,
    pub sensor: i32,
    pub name: String,
    pub value: f64,
    pub unit: String,
}

/// Identifiers from mode 09.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VehicleIdentity {
    pub vin: String,
    /// Several are joined with ", ".
    pub calibration_id: String,
    /// CVN as hex, several joined with ", ".
    pub calibration_verification: String,
    pub ecu_name: String,
}

/// In-use performance tracking (mode 09, InfoType 0x08 for spark ignition,
/// 0x0B for compression ignition). For every monitor the control unit counts
/// how often the test ran to the end (completed) and how often the driving
/// conditions would have allowed it (conditions). The ratio shows whether a
/// monitor runs often enough in everyday driving.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InUseCounter {
    pub name: String,
    pub completed: i32,
    pub conditions: i32,
    /// completed / conditions, 0 if conditions == 0
    pub ratio: f64,
    pub has_ratio: bool,
    /// Plain sentence.
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InUsePerformance {
    pub valid: bool,
    /// 0x08 spark ignition, 0x0B compression ignition
    pub spark_ignition: bool,
    /// Driving cycles that met the OBD conditions.
    pub obd_conditions: i32,
    /// Ignition cycles counted.
    pub ignition_cycles: i32,
    pub monitors: Vec<InUseCounter>,
}

impl Default for InUsePerformance {
    fn default() -> Self {
        Self {
            valid: false,
            spark_ignition: true,
            obd_conditions: 0,
            ignition_cycles: 0,
            monitors: Vec::new(),
        }
    }
}

/// Everything mode 09 tells about one control unit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EcuInfo {
    /// "7E8", empty while headers are off.
    pub ecu_source: String,
    /// "Motor (7E8)", empty while headers are off.
    pub ecu: String,
    /// InfoType 0x0A, e.g. "ECM-EngineControl".
    pub name: String,
    /// InfoType 0x02.
    pub vin: String,
    /// InfoType 0x04.
    pub calibration_ids: Vec<String>,
    /// InfoType 0x06, hex "1A2B3C4D".
    pub cvns: Vec<String>,
    /// InfoType 0x0D, engine serial number.
    pub esn: String,
    /// InfoType 0x08 or 0x0B.
    pub in_use: InUsePerformance,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LiveValue {
    pub pid: i32,
    /// Series id, see `live_key` in caruspex-data; == pid for sub 0.
    pub key: i32,
    pub sub: i32,
    pub name: String,
    pub value: f64,
    /// Set if this is a status value.
    pub text: String,
    pub unit: String,
    pub warn: bool,
    pub category: PidCategory,
    pub display_min: f64,
    pub display_max: f64,
    /// See [`Fault::ecu`].
    pub ecu: String,
    pub ecu_source: String,
}

/// One freeze frame (mode 02). A control unit may keep several: frame 0 is
/// the one the standard requires, further frames belong to later faults.
/// PID 02 of each frame names the fault that stored it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FreezeFrame {
    pub frame: i32,
    /// E.g. "P0300".
    pub dtc: String,
    /// See [`Fault::ecu`], empty while headers are off.
    pub ecu: String,
    pub ecu_source: String,
    /// All values of the frame, sub-values included.
    pub values: Vec<LiveValue>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn source_at(result: &ElmResult, index: usize) -> String {
    result.sources.get(index).cloned().unwrap_or_default()
}

/// The data bytes as the PID table reads them.
fn ints(data: &[u8]) -> Vec<i32> {
    data.iter().map(|&b| i32::from(b)).collect()
}

/// Four mask bytes as one big-endian number.
fn mask_of(bytes: &[u8]) -> u32 {
    (u32::from(bytes[0]) << 24)
        | (u32::from(bytes[1]) << 16)
        | (u32::from(bytes[2]) << 8)
        | u32::from(bytes[3])
}

/// Numbers in the active language: a decimal comma in German.
fn decimal(value: f64, digits: usize) -> String {
    let out = format!("{value:.digits$}");
    if language() == Language::German {
        out.replace('.', ",")
    } else {
        out
    }
}

/// Groups the messages of a response by sender, in the order the senders
/// first appear. Without headers everything lands in one group.
fn group_by_source(result: &ElmResult) -> Vec<(String, Vec<Vec<u8>>)> {
    let mut groups: Vec<(String, Vec<Vec<u8>>)> = Vec::new();
    for (i, message) in result.messages.iter().enumerate() {
        let source = source_at(result, i);
        match groups.iter_mut().find(|g| g.0 == source) {
            Some(group) => group.1.push(message.clone()),
            None => groups.push((source, vec![message.clone()])),
        }
    }
    groups
}

fn decode_status(data: &[u8]) -> VehicleStatus {
    let mut status = VehicleStatus::default();
    // PID echo
    let data = if data.first() == Some(&0x01) {
        &data[1..]
    } else {
        data
    };
    if data.len() < 4 {
        return status;
    }

    status.valid = true;
    status.mil_on = (data[0] & 0x80) != 0;
    status.fault_count = i32::from(data[0] & 0x7F);
    let (b, c, d) = (i32::from(data[1]), i32::from(data[2]), i32::from(data[3]));
    for m in monitors_b() {
        if b & m.mask != 0 {
            status.readiness.push(ReadinessItem {
                name: m.name.to_string(),
                ready: (b & (m.mask << 4)) == 0,
            });
        }
    }
    // Bit 3 of byte B: 0 = spark ignition, 1 = compression ignition. The
    // bits of bytes C and D mean different monitors on the two.
    status.compression_ignition = (b & 0x08) != 0;
    let monitors = if status.compression_ignition {
        monitors_c_diesel()
    } else {
        monitors_c()
    };
    for m in monitors {
        if c & m.mask != 0 {
            status.readiness.push(ReadinessItem {
                name: m.name.to_string(),
                ready: (d & m.mask) == 0,
            });
        }
    }
    status
}

fn make_value(spec: &PidSpec, reading: &PidReading<'_>, source: &str) -> LiveValue {
    LiveValue {
        pid: spec.pid,
        key: reading.spec.key,
        sub: reading.spec.sub,
        name: reading.spec.name.to_string(),
        unit: reading.spec.unit.to_string(),
        value: reading.value,
        text: reading.spec.text_of(reading.value).unwrap_or_default(),
        warn: reading.spec.warns(reading.value),
        category: spec.category,
        display_min: reading.spec.display_min,
        display_max: reading.spec.display_max,
        ecu_source: source.to_string(),
        ecu: ecu_display_name(source),
    }
}

// ---------------------------------------------------------------------------
// Several PIDs per request (CAN only)
// ---------------------------------------------------------------------------

/// Whether a PID may go into a multi-PID request: known to the PID table,
/// not a range marker, and with a length that does not vary between control
/// units (0x68 is three bytes on older ones and seven on newer ones).
pub fn batchable_pid(pid: i32) -> bool {
    if pid <= 0 || pid > 0xFF || pid % 0x20 == 0 {
        return false;
    }
    // PID 0x68 (intake air temperature sensors) is three bytes on older
    // control units and seven on newer ones. Inside a combined answer a
    // wrong length would shift every PID after it, so it is asked alone.
    if pid == 0x68 {
        return false;
    }
    find_pid(pid).is_some_and(|spec| spec.bytes >= 1)
}

/// Splits one mode 01 answer to a multi-PID request into its values. `data`
/// is the message without the service byte: PID, its data bytes, next PID,
/// ... The length of each block comes from the PID table. Only PIDs listed in
/// `requested` are accepted, each once. Padding bytes after the last block are
/// ignored. `None` if the message does not add up (unknown PID, short data,
/// leftover bytes).
pub fn split_multi_pid_answer(data: &[u8], requested: &[i32]) -> Option<Vec<(i32, Vec<u8>)>> {
    let mut blocks: Vec<(i32, Vec<u8>)> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut pos = 0;
    while pos < data.len() {
        let pid = i32::from(data[pos]);
        let wanted = requested.contains(&pid) && !seen.contains(&pid);
        if !wanted {
            // Some control units fill the rest of the frame. Bytes that can
            // only be padding are fine after at least one complete block.
            let padding = !blocks.is_empty()
                && data[pos..]
                    .iter()
                    .all(|&b| b == 0x00 || b == 0x55 || b == 0xAA || b == 0xCC);
            return padding.then_some(blocks);
        }
        let spec = find_pid(pid)?;
        if spec.bytes < 1 {
            return None;
        }
        let length = spec.bytes as usize;
        if pos + 1 + length > data.len() {
            return None;
        }
        blocks.push((pid, data[pos + 1..pos + 1 + length].to_vec()));
        seen.insert(pid);
        pos += 1 + length;
    }
    (!blocks.is_empty()).then_some(blocks)
}

// ---------------------------------------------------------------------------
// Mode 09 decoders, public for tests. `messages` are the responses of one
// control unit without the service byte, each starting with the InfoType.
// On CAN one message holds everything (InfoType, number of items, data); on
// the K-line every message holds a sequence number and four data bytes.
// ---------------------------------------------------------------------------

pub fn info_payload(messages: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = Vec::new();
    if messages.is_empty() {
        return payload;
    }

    // K-line: several short messages "InfoType, sequence number, 4 bytes".
    // Put them in sequence order, the adapter keeps the order anyway.
    let kline = messages.len() > 1 && messages.iter().all(|m| m.len() <= 6);
    let mut ordered: Vec<&Vec<u8>> = messages.iter().collect();
    if kline {
        ordered.sort_by_key(|m| if m.len() > 1 { m[1] } else { 0 });
    }
    // Both layouts start with the InfoType and one counting byte (number of
    // data items on CAN, sequence number on the K-line).
    for m in ordered {
        if m.len() > 2 {
            payload.extend_from_slice(&m[2..]);
        }
    }
    payload
}

pub fn info_text(payload: &[u8]) -> String {
    let text: String = payload
        .iter()
        .filter(|&&b| (0x20..0x7F).contains(&b))
        .map(|&b| b as char)
        .collect();
    text.trim_matches(' ').to_string()
}

/// Calibration IDs are 16 characters each, padded with zero bytes.
pub fn info_calibration_ids(payload: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 16 <= payload.len() {
        let id = info_text(&payload[i..i + 16]);
        if !id.is_empty() {
            out.push(id);
        }
        i += 16;
    }
    // Some control units leave out the padding of the last ID; whatever
    // text is left over still counts.
    let rest = payload.len() - payload.len() % 16;
    let tail = info_text(&payload[rest..]);
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// Calibration verification numbers are four binary bytes each.
pub fn info_cvns(payload: &[u8]) -> Vec<String> {
    payload
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| format!("{:02X}{:02X}{:02X}{:02X}", c[0], c[1], c[2], c[3]))
        .collect()
}

/// After the two general counters come pairs: completed, conditions met.
const SPARK_MONITORS: [Text; 13] = [
    Text::new("Katalysator Bank 1", "Catalyst bank 1"),
    Text::new("Katalysator Bank 2", "Catalyst bank 2"),
    Text::new("Lambdasonde Bank 1", "Oxygen sensor bank 1"),
    Text::new("Lambdasonde Bank 2", "Oxygen sensor bank 2"),
    Text::new(
        "Abgasrückführung und Nockenwellenverstellung",
        "EGR and VVT",
    ),
    Text::new("Sekundärluft", "Secondary air"),
    Text::new("Tankentlüftung", "Evaporative system"),
    Text::new(
        "Hintere Lambdasonde Bank 1",
        "Secondary oxygen sensor bank 1",
    ),
    Text::new(
        "Hintere Lambdasonde Bank 2",
        "Secondary oxygen sensor bank 2",
    ),
    Text::new(
        "Gemischverteilung der Zylinder Bank 1",
        "Air-fuel ratio imbalance bank 1",
    ),
    Text::new(
        "Gemischverteilung der Zylinder Bank 2",
        "Air-fuel ratio imbalance bank 2",
    ),
    Text::new(
        "Benzinpartikelfilter Bank 1",
        "Gasoline particulate filter bank 1",
    ),
    Text::new(
        "Benzinpartikelfilter Bank 2",
        "Gasoline particulate filter bank 2",
    ),
];

const COMPRESSION_MONITORS: [Text; 8] = [
    Text::new("Oxidationskatalysator (NMHC)", "NMHC catalyst"),
    Text::new("Stickoxid-Katalysator (SCR)", "NOx catalyst (SCR)"),
    Text::new("NOx-Speicherkatalysator", "NOx adsorber"),
    Text::new("Partikelfilter", "Particulate filter"),
    Text::new("Abgassensor", "Exhaust gas sensor"),
    Text::new(
        "Abgasrückführung und Nockenwellenverstellung",
        "EGR and VVT",
    ),
    Text::new("Ladedruck", "Boost pressure"),
    Text::new("Kraftstoffsystem", "Fuel system"),
];

pub fn info_in_use_performance(info_type: i32, payload: &[u8]) -> InUsePerformance {
    let mut perf = InUsePerformance {
        spark_ignition: info_type != 0x0B,
        ..InUsePerformance::default()
    };
    let names: &[Text] = if perf.spark_ignition {
        &SPARK_MONITORS
    } else {
        &COMPRESSION_MONITORS
    };

    let counters: Vec<i32> = payload
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| (i32::from(p[0]) << 8) | i32::from(p[1]))
        .collect();
    if counters.len() < 2 {
        return perf;
    }

    perf.valid = true;
    perf.obd_conditions = counters[0];
    perf.ignition_cycles = counters[1];
    let mut pair = 0;
    while 2 + 2 * pair + 1 < counters.len() {
        let name = match names.get(pair) {
            Some(text) => text.get().to_string(),
            None => format!("{}{}", l("Überwachung ", "Monitor "), pair + 1),
        };
        let completed = counters[2 + 2 * pair];
        let conditions = counters[2 + 2 * pair + 1];
        let has_ratio = conditions > 0;
        let ratio = if has_ratio {
            f64::from(completed) / f64::from(conditions)
        } else {
            0.0
        };
        let text = if !has_ratio {
            if completed == 0 {
                l(
                    "noch keine passende Fahrt erkannt",
                    "no suitable trip recorded yet",
                )
                .to_string()
            } else {
                format!("{}{}", completed, l(" Mal geprüft", " times checked"))
            }
        } else {
            format!(
                "{}{}{}{}{}",
                completed,
                l(" Mal geprüft bei ", " times checked in "),
                conditions,
                l(" passenden Fahrten, Quote ", " suitable trips, ratio "),
                decimal(ratio, 2)
            )
        };
        perf.monitors.push(InUseCounter {
            name,
            completed,
            conditions,
            ratio,
            has_ratio,
            text,
        });
        pair += 1;
    }
    perf
}

/// Pulls the misfire counters out of the mode 06 results. Cylinders come in
/// order, the whole engine first. Empty if the control unit has no misfire
/// monitor results (K-line vehicles: the test numbers are not standardized
/// there, so nothing is guessed).
pub fn misfire_counts(results: &[MonitorResult]) -> Vec<MisfireCount> {
    // Keyed by sender and cylinder so two engines' worth of data (rare, but
    // possible with headers on) stay apart and the order is stable.
    let mut found: BTreeMap<(String, i32), MisfireCount> = BTreeMap::new();
    for r in results {
        if !r.can || r.component_id < 0xA1 || r.component_id > 0xAD {
            continue;
        }
        if r.test_id != 0x0B && r.test_id != 0x0C {
            continue;
        }
        // 0 = whole engine
        let cylinder = r.component_id - 0xA1;
        let m = found.entry((r.ecu_source.clone(), cylinder)).or_default();
        m.cylinder = cylinder;
        m.ecu = r.ecu.clone();
        m.ecu_source = r.ecu_source.clone();
        if r.test_id == 0x0B {
            m.average_ten_cycles = r.value;
            m.has_average = true;
        } else {
            m.current_cycle = r.value;
            m.has_current = true;
        }
    }

    let count = |v: f64| ((v + 0.5) as i64).to_string();
    let mut out = Vec::new();
    for (_, mut m) in found {
        m.name = if m.cylinder == 0 {
            l("Motor gesamt", "Whole engine").to_string()
        } else {
            format!("{}{}", l("Zylinder ", "Cylinder "), m.cylinder)
        };
        let mut text = format!("{}: ", m.name);
        if m.has_current {
            if m.current_cycle < 0.5 {
                text.push_str(l(
                    "keine Aussetzer in der aktuellen Fahrt",
                    "no misfires in the current trip",
                ));
            } else {
                text.push_str(&count(m.current_cycle));
                text.push_str(l(
                    " Aussetzer in der aktuellen Fahrt",
                    " misfires in the current trip",
                ));
            }
        }
        if m.has_average {
            if m.has_current {
                text.push_str(", ");
            }
            text.push_str(l("im Schnitt ", "on average "));
            text.push_str(&decimal(m.average_ten_cycles, 1));
            text.push_str(l(
                " je Fahrt über die letzten 10 Fahrten",
                " per trip over the last 10 trips",
            ));
        }
        m.text = text + ".";
        out.push(m);
    }
    out
}

// ---------------------------------------------------------------------------
// The services
// ---------------------------------------------------------------------------

/// The EOBD services on a connected adapter.
pub struct Obd2<'a> {
    elm: &'a mut Elm327,
    batch_refused: BTreeSet<i32>,
}

impl<'a> Obd2<'a> {
    pub fn new(elm: &'a mut Elm327) -> Self {
        Self {
            elm,
            batch_refused: BTreeSet::new(),
        }
    }

    /// Like `new`, but with the batch memory of an earlier `Obd2` on the same
    /// session (see [`Obd2::batch_refused_pids`]), so a caller that cannot
    /// keep the borrow alive does not try the refused groups again.
    pub fn with_batch_memory(elm: &'a mut Elm327, refused: BTreeSet<i32>) -> Self {
        Self {
            elm,
            batch_refused: refused,
        }
    }

    /// The adapter underneath, e.g. to switch headers between two reads.
    pub fn elm(&mut self) -> &mut Elm327 {
        self.elm
    }

    fn read_codes(&mut self, service: &str, sid: u8, kind: &str) -> Vec<Fault> {
        let mut out: Vec<Fault> = Vec::new();
        let result = self.elm.request(service, sid, DEFAULT_REQUEST_TIMEOUT_MS);
        if !result.ok() {
            return out;
        }

        for (m, data) in result.messages.iter().enumerate() {
            let source = source_at(&result, m);
            // On CAN the number of codes comes first, on K-line it does not.
            // Odd length means: the first byte is the count.
            let start = if data.len() % 2 == 1 { 1 } else { 0 };
            let mut i = start;
            while i + 1 < data.len() {
                let code = decode_dtc(i32::from(data[i]), i32::from(data[i + 1]));
                i += 2;
                if code.is_empty() {
                    continue;
                }
                if out.iter().any(|f| f.code == code && f.ecu_source == source) {
                    continue;
                }
                out.push(Fault {
                    kind: kind.to_string(),
                    text: describe_dtc(&code),
                    throttle_related: is_throttle_dtc(&code),
                    ecu: ecu_display_name(&source),
                    ecu_source: source.clone(),
                    code,
                });
            }
        }
        out
    }

    /// Stored (03), pending (07) and permanent (0A) codes of every control
    /// unit that answers, each code once per control unit.
    pub fn read_faults(&mut self) -> Vec<Fault> {
        let mut all = self.read_codes("03", 0x03, l("gespeichert", "stored"));
        let extra = [
            ("07", 0x07, l("sporadisch", "intermittent")),
            ("0A", 0x0A, l("dauerhaft", "permanent")),
        ];
        for (service, sid, kind) in extra {
            for f in self.read_codes(service, sid, kind) {
                let known = all
                    .iter()
                    .any(|existing| existing.code == f.code && existing.ecu_source == f.ecu_source);
                if !known {
                    all.push(f);
                }
            }
        }
        all
    }

    /// Mode 04. On a silent acknowledgement the codes are read back; `Err`
    /// with an empty text if some are still there.
    pub fn clear_faults(&mut self) -> Result<(), String> {
        let result = self.elm.request("04", 0x04, 8000);
        if result.ok() {
            return Ok(());
        }
        if result.status == ElmStatus::AdapterError || result.status == ElmStatus::BusError {
            return Err(result.message);
        }
        // Some control units acknowledge silently: check whether it worked.
        if self.read_faults().is_empty() {
            Ok(())
        } else {
            Err(String::new())
        }
    }

    /// Status of the first control unit that answers (normally the engine).
    pub fn read_status(&mut self) -> VehicleStatus {
        self.read_status_per_ecu()
            .into_iter()
            .find(|s| s.valid)
            .unwrap_or_default()
    }

    /// Status of every control unit that answers. Needs headers to tell them
    /// apart; without headers the entries simply have no ecu name.
    pub fn read_status_per_ecu(&mut self) -> Vec<VehicleStatus> {
        let mut out = Vec::new();
        let result = self.elm.request("0101", 0x01, DEFAULT_REQUEST_TIMEOUT_MS);
        if !result.ok() {
            return out;
        }
        for (i, message) in result.messages.iter().enumerate() {
            let mut status = decode_status(message);
            if !status.valid {
                continue;
            }
            status.ecu_source = source_at(&result, i);
            status.ecu = ecu_display_name(&status.ecu_source);
            out.push(status);
        }
        out
    }

    /// Supported PIDs 0x01 to 0xE0, merged over all control units.
    pub fn supported_pids(&mut self) -> BTreeSet<i32> {
        let mut found = BTreeSet::new();
        for base in (0x00..=0xC0).step_by(0x20) {
            let result =
                self.elm
                    .request(&format!("01{base:02X}"), 0x01, DEFAULT_REQUEST_TIMEOUT_MS);
            if !result.ok() || result.messages.is_empty() {
                break;
            }
            // Several control units each send their own mask; together they
            // say what the vehicle can answer.
            let mut next = false;
            for message in &result.messages {
                let data = if message.first().map(|&b| i32::from(b)) == Some(base) {
                    &message[1..]
                } else {
                    &message[..]
                };
                if data.len() < 4 {
                    continue;
                }
                let mask = mask_of(data);
                for i in 0..32 {
                    if mask & (1u32 << (31 - i)) != 0 {
                        found.insert(base + i + 1);
                    }
                }
                if mask & 1 != 0 {
                    next = true;
                }
            }
            if !next {
                break; // next range not supported
            }
        }
        // The range markers themselves are no live values.
        for marker in (0x20..=0xE0).step_by(0x20) {
            found.remove(&marker);
        }
        found
    }

    /// `read_live_all`, optionally keeping the values of every control unit.
    fn read_pid(&mut self, pid: i32, every_ecu: bool) -> Vec<LiveValue> {
        let mut out = Vec::new();
        let Some(spec) = find_pid(pid) else {
            return out;
        };
        let result = self.elm.request(&format!("01{pid:02X}"), 0x01, 4000);
        if !result.ok() || result.messages.is_empty() {
            return out;
        }

        // Several control units may answer (vehicle speed comes from engine and
        // transmission); the first answer counts, which is the engine on CAN.
        for (i, message) in result.messages.iter().enumerate() {
            if message.first().map(|&b| i32::from(b)) != Some(pid) {
                continue;
            }
            let data = ints(&message[1..]);
            let source = source_at(&result, i);
            for reading in decode_pid(spec, &data) {
                out.push(make_value(spec, &reading, &source));
            }
            if !out.is_empty() && !every_ecu {
                break;
            }
        }
        out
    }

    /// Primary value of a PID (sub 0). If the control unit does not report
    /// the primary value of a multi-value PID, the first value it does report.
    pub fn read_live(&mut self, pid: i32) -> Option<LiveValue> {
        self.read_live_all(pid).into_iter().next()
    }

    /// One value per PID, in the order of `pids`, like [`Obd2::read_live`] for
    /// each. Uses `read_live_batch` underneath, so on CAN it is just as fast.
    pub fn read_live_values(&mut self, pids: &[i32]) -> Vec<LiveValue> {
        // read_live keeps the first value of a PID, which is the primary value
        // whenever the control unit reports it. The same here.
        let all = self.read_live_batch(pids, false);
        pids.iter()
            .filter_map(|&pid| all.iter().find(|v| v.pid == pid).cloned())
            .collect()
    }

    /// All values of one PID the control unit reports, e.g. both coolant
    /// sensors of PID 0x67. Empty if the PID is unknown or not answered.
    pub fn read_live_all(&mut self, pid: i32) -> Vec<LiveValue> {
        self.read_pid(pid, false)
    }

    /// One group request; `None` if it has to be read PID by PID instead,
    /// otherwise the PIDs the answers left out.
    fn read_group(
        &mut self,
        group: &[i32],
        every_ecu: bool,
        values: &mut BTreeMap<i32, Vec<LiveValue>>,
    ) -> Option<Vec<i32>> {
        let mut command = String::from("01");
        for pid in group {
            command.push_str(&format!("{pid:02X}"));
        }
        let result = self.elm.request(&command, 0x01, 4000);
        if !result.ok() || result.messages.is_empty() {
            return None;
        }

        // Decode everything first: one answer that does not add up spoils the
        // group, and nothing of it may land in values then.
        let mut found: BTreeMap<i32, Vec<LiveValue>> = BTreeMap::new();
        // Present in an answer, even if no value was usable.
        let mut answered = BTreeSet::new();
        for (i, message) in result.messages.iter().enumerate() {
            let blocks = split_multi_pid_answer(message, group)?;
            for (pid, data) in blocks {
                answered.insert(pid);
                // As in read_live_all: the first control unit that answers a
                // PID with something usable keeps it, unless every one is
                // wanted.
                if !every_ecu && found.contains_key(&pid) {
                    continue;
                }
                let Some(spec) = find_pid(pid) else {
                    continue;
                };
                let source = source_at(&result, i);
                for reading in decode_pid(spec, &ints(&data)) {
                    found
                        .entry(pid)
                        .or_default()
                        .push(make_value(spec, &reading, &source));
                }
            }
        }
        for (pid, list) in found {
            values.entry(pid).or_default().extend(list);
        }
        Some(
            group
                .iter()
                .copied()
                .filter(|pid| !answered.contains(pid))
                .collect(),
        )
    }

    /// All values of several PIDs, sub-values included: the same as calling
    /// `read_live_all` for each PID, in the order of `pids`, but much faster on
    /// CAN.
    ///
    /// On CAN (ISO 15765-4) SAE J1979 allows up to six mode 01 PIDs in one
    /// request ("010C0D0511"); the answer holds each PID followed by its data
    /// bytes. The PIDs are grouped by six and every group costs one round
    /// trip instead of six. On the K-line and J1850, and for PIDs whose
    /// length varies (see [`batchable_pid`]), every PID is asked on its own.
    ///
    /// If a group gets NO DATA, a refusal or an answer that does not add up,
    /// its PIDs are read one by one and remembered: for the rest of the
    /// session (this Obd2, or until `reset_batch_memory`) they are no longer
    /// put into groups. The same happens to a PID a control unit leaves out
    /// of a group answer.
    ///
    /// Several control units: like `read_live_all`, a PID keeps the values of
    /// the first control unit that answers it (the engine on CAN). With
    /// `every_ecu` the values of every control unit are kept, each with its
    /// ecu/ecu_source (needs headers on to tell them apart).
    pub fn read_live_batch(&mut self, pids: &[i32], every_ecu: bool) -> Vec<LiveValue> {
        // Every PID once, in the order asked; unknown PIDs are dropped like in
        // read_live_all.
        let mut order: Vec<i32> = Vec::new();
        for &pid in pids {
            if find_pid(pid).is_some() && !order.contains(&pid) {
                order.push(pid);
            }
        }

        let mut values: BTreeMap<i32, Vec<LiveValue>> = BTreeMap::new();
        let mut single: Vec<i32> = Vec::new();
        if self.elm.is_can() {
            let mut grouped: Vec<i32> = Vec::new();
            for &pid in &order {
                if batchable_pid(pid) && !self.batch_refused.contains(&pid) {
                    grouped.push(pid);
                } else {
                    single.push(pid);
                }
            }
            // SAE J1979 on ISO 15765-4
            const MAX_PER_REQUEST: usize = 6;
            for group in grouped.chunks(MAX_PER_REQUEST) {
                if group.len() == 1 {
                    single.push(group[0]);
                    continue;
                }
                let missing = self
                    .read_group(group, every_ecu, &mut values)
                    .unwrap_or_else(|| group.to_vec());
                // Left out or spoilt: from now on these are asked one by one.
                // That also catches control units that only answer the first
                // PID of a combined request.
                for pid in missing {
                    self.batch_refused.insert(pid);
                    single.push(pid);
                }
            }
            // Single requests go out in the order asked, which keeps a log of
            // the traffic easy to follow.
            single.sort_by_key(|pid| order.iter().position(|p| p == pid));
        } else {
            single = order.clone();
        }

        for pid in single {
            let list = self.read_pid(pid, every_ecu);
            values.entry(pid).or_default().extend(list);
        }

        let mut out = Vec::new();
        for pid in &order {
            if let Some(list) = values.get(pid) {
                out.extend(list.iter().cloned());
            }
        }
        out
    }

    /// PIDs that are read one by one for the rest of the session because a
    /// group request with them failed.
    pub fn batch_refused_pids(&self) -> &BTreeSet<i32> {
        &self.batch_refused
    }

    pub fn reset_batch_memory(&mut self) {
        self.batch_refused.clear();
    }

    pub fn read_vin(&mut self) -> String {
        if self.elm.listen_only() {
            // Tesla sends the VIN in three parts on 0x405 (multiplexer 0x10,
            // 0x11, 0x12 in the first byte, seven characters each after it).
            let mut parts: [String; 3] = Default::default();
            for f in self.elm.capture_frames(0x405, 1500, 60) {
                if f.len() < 2 || f[0] < 0x10 || f[0] > 0x12 {
                    continue;
                }
                parts[usize::from(f[0] - 0x10)] = f[1..]
                    .iter()
                    .filter(|&&b| (0x20..0x7F).contains(&b))
                    .map(|&b| b as char)
                    .collect();
            }
            let compact: String = parts
                .concat()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect();
            return if compact.len() >= 17 {
                compact[compact.len() - 17..].to_string()
            } else {
                String::new()
            };
        }
        let result = self.elm.request("0902", 0x09, 8000);
        if !result.ok() {
            return String::new();
        }
        let mut text = String::new();
        for (_, messages) in group_by_source(&result) {
            text = info_text(&info_payload(&messages));
            if text.len() >= 17 {
                break;
            }
        }
        // Some control units put zero bytes or spaces in front; the VIN is the
        // last 17 characters.
        let compact: String = text.chars().filter(|&c| c != ' ').collect();
        if compact.len() >= 17 {
            return compact[compact.len() - 17..].to_string();
        }
        compact
    }

    /// Frame 0 only, primary values of a fixed set of PIDs, and the code that
    /// stored the frame (empty if none). Kept for older callers;
    /// `read_freeze_frames` gives every frame with all values.
    pub fn read_freeze_frame(&mut self) -> (Vec<LiveValue>, String) {
        let mut trigger_code = String::new();
        let trigger = self.elm.request("0202", 0x02, DEFAULT_REQUEST_TIMEOUT_MS);
        if trigger.ok()
            && let Some(msg) = trigger.messages.first()
            && msg.len() >= 4
        {
            trigger_code = decode_dtc(i32::from(msg[2]), i32::from(msg[3]));
        }

        let mut rows = Vec::new();
        for pid in [0x0C, 0x0D, 0x05, 0x04, 0x11, 0x0F, 0x10, 0x0B, 0x03] {
            let Some(spec) = find_pid(pid) else {
                continue;
            };
            let result = self.elm.request(&format!("02{pid:02X}00"), 0x02, 4000);
            if !result.ok() || result.messages.is_empty() {
                continue;
            }
            let mut data: &[u8] = &result.messages[0];
            // Mode 02 answers with PID and frame number in front of the data.
            if data.len() >= 2 && i32::from(data[0]) == pid {
                data = &data[2..];
            }
            let readings = decode_pid(spec, &ints(data));
            let Some(first) = readings.first() else {
                continue;
            };
            if first.spec.sub != 0 {
                continue;
            }
            rows.push(make_value(spec, first, &source_at(&result, 0)));
        }
        (rows, trigger_code)
    }

    /// Freeze frame PIDs a control unit lists as supported for one frame.
    fn freeze_frame_pids(&mut self, frame: i32) -> Vec<i32> {
        // The same masks as for mode 01, but with the frame number after the
        // PID: request "0200 01" for frame 1, answer "42 00 01 BE 1F ...".
        let mut found = BTreeSet::new();
        for base in (0x00..=0xC0).step_by(0x20) {
            let command = format!("02{:02X}{:02X}", base, frame & 0xFF);
            let result = self.elm.request(&command, 0x02, 4000);
            if !result.ok() {
                break;
            }
            let mut next = false;
            for msg in &result.messages {
                if msg.len() < 6 || i32::from(msg[0]) != base || i32::from(msg[1]) != frame {
                    continue;
                }
                let mask = mask_of(&msg[2..]);
                for i in 0..32 {
                    if mask & (1u32 << (31 - i)) != 0 {
                        found.insert(base + i + 1);
                    }
                }
                if mask & 1 != 0 {
                    next = true;
                }
            }
            if !next {
                break;
            }
        }
        // PID 02 is the fault code itself, the markers are no values.
        let pids: Vec<i32> = found
            .into_iter()
            .filter(|&pid| pid != 0x02 && pid % 0x20 != 0 && find_pid(pid).is_some())
            .collect();
        if pids.is_empty() {
            return vec![
                0x03, 0x04, 0x05, 0x06, 0x07, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11,
            ];
        }
        pids
    }

    /// Freeze frames 0 to `max_frames - 1` ([`DEFAULT_FREEZE_FRAMES`] is the
    /// usual). Stops at the first frame that reports no fault code (00 00) or
    /// gets no answer. The values of each frame are the PIDs the control unit
    /// lists as supported for it; if it does not list them, a fixed set of
    /// the usual ones.
    pub fn read_freeze_frames(&mut self, max_frames: usize) -> Vec<FreezeFrame> {
        let mut out = Vec::new();
        let mut frame: i32 = 0;
        while (frame as usize) < max_frames && frame <= 0xFF {
            // PID 02 of the frame: which fault stored it. 00 00 means the frame
            // is empty, and so are all after it.
            let trigger = self.elm.request(&format!("0202{frame:02X}"), 0x02, 4000);
            if !trigger.ok() {
                break;
            }

            let mut found: Vec<FreezeFrame> = Vec::new();
            for (i, msg) in trigger.messages.iter().enumerate() {
                // A control unit that ignores the frame number would hand out
                // frame 0 again and again; only an answer for this frame counts.
                if msg.len() < 4 || msg[0] != 0x02 || i32::from(msg[1]) != frame {
                    continue;
                }
                let code = decode_dtc(i32::from(msg[2]), i32::from(msg[3]));
                if code.is_empty() {
                    continue;
                }
                let source = source_at(&trigger, i);
                found.push(FreezeFrame {
                    frame,
                    dtc: code,
                    ecu: ecu_display_name(&source),
                    ecu_source: source,
                    values: Vec::new(),
                });
            }
            if found.is_empty() {
                break;
            }

            let mut filled: Vec<BTreeSet<i32>> = vec![BTreeSet::new(); found.len()];
            for pid in self.freeze_frame_pids(frame) {
                let Some(spec) = find_pid(pid) else {
                    continue;
                };
                let command = format!("02{pid:02X}{frame:02X}");
                let result = self.elm.request(&command, 0x02, 4000);
                if !result.ok() {
                    continue;
                }
                for (i, message) in result.messages.iter().enumerate() {
                    // Mode 02 answers with PID and frame number in front of the
                    // data.
                    if message.len() < 3
                        || i32::from(message[0]) != pid
                        || i32::from(message[1]) != frame
                    {
                        continue;
                    }
                    let data = ints(&message[2..]);
                    // The frame of the control unit that sent it. Without
                    // headers every source is empty and the answers go in order.
                    let source = source_at(&result, i);
                    for (k, f) in found.iter_mut().enumerate() {
                        if f.ecu_source != source || filled[k].contains(&pid) {
                            continue;
                        }
                        filled[k].insert(pid);
                        for reading in decode_pid(spec, &data) {
                            f.values.push(make_value(spec, &reading, &source));
                        }
                        break;
                    }
                }
            }
            out.extend(found);
            frame += 1;
        }
        out
    }

    /// Mode 09: VIN, calibration ID, checksum, control unit name. Anything
    /// missing stays empty.
    pub fn read_identity(&mut self) -> VehicleIdentity {
        let mut identity = VehicleIdentity {
            vin: self.read_vin(),
            ..VehicleIdentity::default()
        };

        // Several control units may report the same number; it is listed once.
        let join = |parts: &[String]| {
            let mut out = String::new();
            for (i, part) in parts.iter().enumerate() {
                if parts[..i].contains(part) {
                    continue;
                }
                if !out.is_empty() {
                    out.push_str(", ");
                }
                out.push_str(part);
            }
            out
        };

        let mut cal_ids: Vec<String> = Vec::new();
        let mut cvns: Vec<String> = Vec::new();
        let cal = self.elm.request("0904", 0x09, 6000);
        if cal.ok() {
            for (_, messages) in group_by_source(&cal) {
                cal_ids.extend(info_calibration_ids(&info_payload(&messages)));
            }
        }
        let cvn = self.elm.request("0906", 0x09, 6000);
        if cvn.ok() {
            for (_, messages) in group_by_source(&cvn) {
                cvns.extend(info_cvns(&info_payload(&messages)));
            }
        }
        let name = self.elm.request("090A", 0x09, 6000);
        if name.ok()
            && let Some((_, messages)) = group_by_source(&name).first()
        {
            identity.ecu_name = info_text(&info_payload(messages));
        }
        identity.calibration_id = join(&cal_ids);
        identity.calibration_verification = join(&cvns);
        identity
    }

    /// Mode 09 in full, per control unit: name, calibration IDs, CVNs,
    /// in-use performance counters, engine serial number.
    pub fn read_ecu_info(&mut self) -> Vec<EcuInfo> {
        let mut ecus: Vec<EcuInfo> = Vec::new();
        let entry = |ecus: &mut Vec<EcuInfo>, source: &str| -> usize {
            if let Some(i) = ecus.iter().position(|e| e.ecu_source == source) {
                return i;
            }
            ecus.push(EcuInfo {
                ecu_source: source.to_string(),
                ecu: ecu_display_name(source),
                ..EcuInfo::default()
            });
            ecus.len() - 1
        };

        for info_type in [0x02, 0x0A, 0x04, 0x06, 0x08, 0x0B, 0x0D] {
            let result = self.elm.request(&format!("09{info_type:02X}"), 0x09, 6000);
            if !result.ok() {
                continue;
            }
            for (source, messages) in group_by_source(&result) {
                // On CAN without headers every message may come from another
                // control unit, so each is decoded on its own.
                let parts: Vec<Vec<Vec<u8>>> = if self.elm.is_can() {
                    messages.iter().map(|m| vec![m.clone()]).collect()
                } else {
                    vec![messages]
                };
                let index = entry(&mut ecus, &source);
                let e = &mut ecus[index];
                for part in &parts {
                    let payload = info_payload(part);
                    match info_type {
                        0x02 => {
                            let vin = info_text(&payload);
                            e.vin = if vin.len() > 17 {
                                vin[vin.len() - 17..].to_string()
                            } else {
                                vin
                            };
                        }
                        0x0A => e.name = info_text(&payload),
                        0x04 => e.calibration_ids.extend(info_calibration_ids(&payload)),
                        0x06 => e.cvns.extend(info_cvns(&payload)),
                        0x08 | 0x0B => {
                            let perf = info_in_use_performance(info_type, &payload);
                            if perf.valid {
                                e.in_use = perf;
                            }
                        }
                        0x0D => e.esn = info_text(&payload),
                        _ => {}
                    }
                }
            }
        }
        ecus
    }

    /// Mode 06: on-board component monitoring with value and limits.
    ///
    /// On CAN the request names a monitor (OBDMID); the answer holds all its
    /// tests, nine bytes each: OBDMID, TID, unit and scaling ID, value, lower
    /// limit, upper limit (two bytes each). Which monitors exist is asked
    /// first with the IDs 0x00, 0x20, 0x40 ... like the supported PIDs.
    /// The K-line does not have that; there each test is queried individually
    /// and the response has no unit byte. Both are covered here.
    ///
    /// `Err` with a plain explanation when there are no results.
    pub fn read_monitor_results(&mut self) -> Result<Vec<MonitorResult>, String> {
        let mut out: Vec<MonitorResult> = Vec::new();

        if self.elm.is_can() || self.elm.protocol_id() == '0' {
            let mut monitors = BTreeSet::new();
            for base in (0x00..=0xE0).step_by(0x20) {
                let block = self.elm.request(&format!("06{base:02X}"), 0x06, 6000);
                if !block.ok() {
                    break;
                }
                let mut next = false;
                for msg in &block.messages {
                    if msg.len() < 5 || i32::from(msg[0]) != base {
                        continue;
                    }
                    let mask = mask_of(&msg[1..]);
                    for i in 0..31 {
                        if mask & (1u32 << (31 - i)) != 0 {
                            monitors.insert(base + i + 1);
                        }
                    }
                    if mask & 1 != 0 {
                        next = true;
                    }
                }
                if !next {
                    break;
                }
            }

            for &mid in &monitors {
                let result = self.elm.request(&format!("06{mid:02X}"), 0x06, 6000);
                if !result.ok() {
                    continue;
                }
                for (m, msg) in result.messages.iter().enumerate() {
                    let word = |at: usize| (i32::from(msg[at]) << 8) | i32::from(msg[at + 1]);
                    let mut i = 0;
                    while i + 8 < msg.len() {
                        let unit_and_scaling = i32::from(msg[i + 2]);
                        let (value, unit) = monitor_scale(unit_and_scaling, word(i + 3));
                        let (minimum, _) = monitor_scale(unit_and_scaling, word(i + 5));
                        let (maximum, _) = monitor_scale(unit_and_scaling, word(i + 7));
                        let component_id = i32::from(msg[i]);
                        let test_id = i32::from(msg[i + 1]);
                        let source = source_at(&result, m);
                        out.push(MonitorResult {
                            can: true,
                            component_id,
                            test_id,
                            unit_and_scaling,
                            value,
                            unit: unit.to_string(),
                            minimum,
                            maximum,
                            has_minimum: true,
                            has_maximum: true,
                            name: monitor_test_name_can(component_id, test_id),
                            component: monitor_component_name(component_id),
                            passed: value >= minimum && value <= maximum,
                            ecu: ecu_display_name(&source),
                            ecu_source: source,
                        });
                        i += 9;
                    }
                }
            }
            // Once the control unit has listed its monitors, the K-line route
            // below must not run: its requests would be read in the wrong
            // layout.
            if !monitors.is_empty() {
                if out.is_empty() {
                    return Err(l(
                        "Das Steuergerät nennt Überwachungen, liefert aber noch keine Ergebnisse. Nach dem Löschen der Fehler braucht es erst einige Fahrten.",
                        "The control unit lists monitors but has no results yet. After clearing the codes it needs a few trips first.",
                    )
                    .to_string());
                }
                return Ok(out);
            }
        }

        // The route for vehicles without CAN: go through the tests one by one.
        for test_id in 0x01..=0x40 {
            let result = self.elm.request(&format!("06{test_id:02X}"), 0x06, 2500);
            if !result.ok() {
                continue;
            }
            for (m, msg) in result.messages.iter().enumerate() {
                // Layout without the service byte: ID, component, value (2),
                // lower limit (2), upper limit (2). That is eight bytes, not
                // nine. Shorter responses occur as well.
                if msg.len() < 5 {
                    continue;
                }
                let word = |at: usize| (i32::from(msg[at]) << 8) | i32::from(msg[at + 1]);
                let mut r = MonitorResult {
                    test_id: i32::from(msg[0]),
                    component_id: i32::from(msg[1]),
                    ..MonitorResult::default()
                };
                let (value, unit) = oxygen_scale(r.test_id, word(2));
                r.value = value;
                r.unit = unit.to_string();
                if msg.len() >= 6 {
                    r.minimum = oxygen_scale(r.test_id, word(4)).0;
                    r.has_minimum = true;
                }
                if msg.len() >= 8 {
                    r.maximum = oxygen_scale(r.test_id, word(6)).0;
                    r.has_maximum = true;
                }
                r.name = monitor_test_name(r.test_id);
                r.component = monitor_component_name(r.component_id);
                r.passed = (!r.has_minimum || r.value >= r.minimum)
                    && (!r.has_maximum || r.value <= r.maximum);
                r.ecu_source = source_at(&result, m);
                r.ecu = ecu_display_name(&r.ecu_source);
                out.push(r);
            }
        }

        if out.is_empty() {
            return Err(l(
                "Das Steuergerät liefert keine Bauteilüberwachung. Bei Fahrzeugen vor etwa 2003 ist das normal.",
                "The control unit provides no component monitoring results. This is normal for vehicles from before about 2003.",
            )
            .to_string());
        }
        Ok(out)
    }

    /// Mode 05, the oxygen sensor test. Only exists without CAN, i.e. exactly
    /// on the older vehicles. `Err` with a plain explanation when there are
    /// no results.
    pub fn read_oxygen_sensor_tests(&mut self) -> Result<Vec<OxygenSensorTest>, String> {
        let mut out = Vec::new();

        for test_id in 0x01..=0x0A {
            for sensor in 1..=4 {
                let command = format!("05{test_id:02X}{sensor:02X}");
                let result = self.elm.request(&command, 0x05, 2500);
                if !result.ok() || result.messages.is_empty() {
                    continue;
                }

                // Layout without the service byte (SAE J1979): test ID, sensor,
                // value, then optionally lower and upper limit, one byte each.
                let msg = &result.messages[0];
                if msg.len() < 3 {
                    continue;
                }
                let (value, unit) = oxygen_scale(test_id, i32::from(msg[2]));
                out.push(OxygenSensorTest {
                    test_id,
                    sensor,
                    name: oxygen_test_name(test_id),
                    value,
                    unit: unit.to_string(),
                });
            }
        }

        if out.is_empty() {
            return Err(l(
                "Das Steuergerät liefert keine Lambdasondenprüfung. Bei Fahrzeugen mit CAN ist dieser Dienst abgeschafft, die Werte stecken dort in der Bauteilüberwachung.",
                "The control unit provides no oxygen sensor test results. Vehicles with CAN no longer have this service; the values are part of component monitoring there.",
            )
            .to_string());
        }
        Ok(out)
    }
}
