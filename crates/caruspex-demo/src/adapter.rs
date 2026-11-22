//! The adapter and the control units behind it.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::codec::*;
use crate::sim::*;
use crate::truck::TruckSim;
use crate::{Chunk, DemoAdapter, DemoOptions, DemoVehicle, Output};

// The engine and gearbox over OBD-II, the UDS units, the AT and ST
// commands and the J1939 side.
mod at;
mod j1939;
mod obd;
mod uds;

use uds::{Bus, UdsUnit, is_uds_service, make_uds_units};

// ---------------------------------------------------------------------------
// Adapter profiles
// ---------------------------------------------------------------------------

// Command set levels: the ELM327 version in which a command first appeared,
// written as major * 100 + minor * 10 + letter (1.3a = 131, 1.4b = 142).
const V11: i32 = 110;
const V12: i32 = 120;
const V13: i32 = 130;
const V13A: i32 = 131;
const V14: i32 = 140;
const V14B: i32 = 142;
const V20: i32 = 200;
const V10: i32 = 100;

pub(crate) struct Profile {
    pub version: &'static str,            // ATI
    pub description: &'static str,        // AT@1
    pub level: i32,                       // highest command set it implements
    pub stn_id: Option<&'static str>,     // STI, None when not an STN chip
    pub stn_device: Option<&'static str>, // STDI
    pub ok_to_all: bool,                  // answers "OK" to unknown AT commands
    pub batch: bool,                      // STN: knows the batched commands (STBC)
    pub medium_speed: bool,               // STN: has the MS-CAN transceiver (STP 51 to 54)
    pub single_wire: bool,                // STN: has the SW-CAN transceiver (STP 61 to 64)
}

const fn elm(version: &'static str, level: i32, ok_to_all: bool) -> Profile {
    Profile {
        version,
        description: "OBDII to RS232 Interpreter",
        level,
        stn_id: None,
        stn_device: None,
        ok_to_all,
        batch: false,
        medium_speed: false,
        single_wire: false,
    }
}

// The STN chips answer ATI like an ELM327 v1.4b and AT@1 with the maker.
const fn stn(id: &'static str, device: &'static str, batch: bool, ms: bool, sw: bool) -> Profile {
    Profile {
        version: "ELM327 v1.4b",
        description: "SCANTOOL.NET LLC",
        level: V14B,
        stn_id: Some(id),
        stn_device: Some(device),
        ok_to_all: false,
        batch,
        medium_speed: ms,
        single_wire: sw,
    }
}

fn profile_for(adapter: DemoAdapter) -> Profile {
    match adapter {
        DemoAdapter::GenuineV14b => elm("ELM327 v1.4b", V14B, false),
        DemoAdapter::CloneV21 => elm("ELM327 v2.1", V14, false),
        DemoAdapter::CloneOkToAll => elm("ELM327 v2.1", V14, true),
        DemoAdapter::CloneV15 => elm("ELM327 v1.5", V14, false),
        DemoAdapter::ObdLinkStn => stn("STN1130 v5.6.19", "OBDLink SX r4.2", false, false, false),
        // Firmware ids as in the examples of the OBDLink manual (STI, STDI).
        DemoAdapter::ObdLinkEx => stn("STN2232 v5.10.3", "OBDLink EX", true, true, false),
        DemoAdapter::ObdLinkMxPlus => {
            stn("STN2256 v5.11.4", "OBDLink MX+ r3.2.1", true, true, true)
        }
    }
}

// ---------------------------------------------------------------------------
// The car
// ---------------------------------------------------------------------------

// Fictional manufacturer code "YCX", model year M (2021), plant C. Position 9
// is the check digit after the North American rule (it comes out as 7).
pub(crate) const VIN: &str = "YCXDE15C7MC048217";
const ENGINE_CALIBRATION: &[u8] = b"CX5EM21A0417";
const GEARBOX_CALIBRATION: &[u8] = b"CX6AT21B0203";
const ENGINE_CVN: [u8; 4] = [0x5E, 0x71, 0x0C, 0x93];
const GEARBOX_CVN: [u8; 4] = [0x2B, 0x90, 0x47, 0xD1];
// Mode 09 ECU name: four characters acronym, a separator, then the name,
// 20 bytes in total, padded with zero bytes.
const ENGINE_NAME: &[u8] = b"ECM\0-EngineControl";
const GEARBOX_NAME: &[u8] = b"TCM\0-TransmissionCtl";

// Engine PIDs in mode 01, without the range PIDs 00, 20, 40 ...
const ENGINE_PIDS: &[i32] = &[
    0x01, 0x03, 0x04, 0x05, 0x06, 0x07, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x13, 0x14, 0x15,
    0x1C, 0x1F, 0x21, 0x23, 0x2F, 0x30, 0x31, 0x33, 0x3C, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47,
    0x49, 0x4A, 0x4C, 0x4D, 0x4E, 0x51, 0x5C, 0x5E, 0x61, 0x62, 0x63, 0xA6,
];
const GEARBOX_PIDS: &[i32] = &[0x01, 0xA4];
const FREEZE_FRAME_PIDS: &[i32] = &[
    0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11,
];
const ENGINE_MIDS: &[i32] = &[0x01, 0x02, 0x21, 0x3C, 0x41, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5];
const ENGINE_INFO: &[i32] = &[0x02, 0x04, 0x06, 0x08, 0x0A];
const GEARBOX_INFO: &[i32] = &[0x04, 0x06, 0x0A];

// Run time since clearing at which a monitor completes again (s).
const HEATER_DONE: f64 = 90.0;
const OXYGEN_DONE: f64 = 300.0;
const CATALYST_DONE: f64 = 420.0;
const EGR_DONE: f64 = 540.0;
// When the faults come back after clearing (engine run time since the clear).
const MISFIRE_BACK: f64 = 180.0;
const LEAN_PENDING: f64 = 240.0;
const LEAN_CONFIRMED: f64 = 600.0;
const CATALYST_CONFIRMED: f64 = 900.0;
const NEVER: f64 = 1e18;

struct Reply {
    id: i32,
    payload: Bytes,
}

// ---------------------------------------------------------------------------
// The adapter and the two control units
// ---------------------------------------------------------------------------

struct Response {
    lines: Vec<String>,
    delay_ms: f64, // simulated time until the answer is complete
    prompt: bool,  // false while monitoring: no '>' until stopped
}

impl Response {
    fn new(lines: &[&str], delay_ms: f64) -> Self {
        Response {
            lines: lines.iter().map(|s| s.to_string()).collect(),
            delay_ms,
            prompt: true,
        }
    }

    fn one(line: impl Into<String>, delay_ms: f64) -> Self {
        Response {
            lines: vec![line.into()],
            delay_ms,
            prompt: true,
        }
    }

    fn empty() -> Self {
        Response {
            lines: Vec::new(),
            delay_ms: 2.0,
            prompt: true,
        }
    }
}

pub(crate) struct Impl {
    options: DemoOptions,
    pub profile: Profile,
    origin: Instant,

    pub output: Output,
    pub input_line: Vec<u8>,
    last_command: String,
    pub discard_line: bool,

    // Adapter settings, as after ATZ.
    echo: bool,
    linefeeds: bool,
    spaces: bool,
    headers: bool,
    auto_format: bool,
    show_dlc: bool,
    protocol: i32, // as set with ATSP / ATTP
    auto_search: bool,
    active: bool,        // a protocol has been found and is open
    header: i32,         // ATSH
    receive_filter: i32, // ATCRA
    can_filter: i32,
    can_mask: i32,
    pub monitoring: bool,
    last_frame: Bytes,
    saved_byte: i32, // ATSD / ATRD, kept in RAM only
    bus: Bus,        // STN: the bus selected with STP
    stp: i32,        // STN: the STP preset, 0 while an AT protocol is set
    batch: bool,     // STN: STBC 1

    // Simulated adapter time in milliseconds, as f64 bits (adapter_ms).
    pub adapter_clock: Arc<AtomicU64>,

    // The control units that answer UDS, with their own fault memory.
    uds_units: Vec<UdsUnit>,
    // The diagnostic session of the engine (01 default, 03 extended) and
    // the time of its last physical request, for the S3 timer.
    engine_session: u8,
    engine_last_request: f64,

    // Fault memory state.
    cleared: bool,
    clear_run: f64, // engine run time at the moment of clearing
    clear_km: f64,

    // SAE J1939 (protocols A and B) and the truck behind it.
    truck: Option<TruckSim>,
    cp: i32,        // ATCP, the top five bits of a 29 bit id
    header24: i32,  // ATSH with six digits, the lower 24 bits
    pb: (i32, i32), // ATPB: options and bit rate divisor of protocol B
    stn_rate: u32,  // STN: STPBR, 0 for the rate of the protocol
    jhf: bool,      // ATJHF: J1939 header formatting
    js: bool,       // ATJS: J1939 bytes as typed (ATJE reverses requests)
}

impl Impl {
    pub fn new(options: DemoOptions) -> Self {
        let profile = profile_for(options.adapter);
        let options_vehicle = options.vehicle;
        let odometer_mismatch = options.truck_odometer_mismatch;
        Impl {
            options,
            profile,
            origin: Instant::now(),
            output: Output::new(),
            input_line: Vec::new(),
            last_command: String::new(),
            discard_line: false,
            echo: true,
            linefeeds: false,
            spaces: true,
            headers: false,
            auto_format: true,
            show_dlc: false,
            protocol: 0,
            auto_search: true,
            active: false,
            header: 0x7DF,
            receive_filter: -1,
            can_filter: -1,
            can_mask: 0x7FF,
            monitoring: false,
            last_frame: vec![0x00; 8],
            saved_byte: 0x00,
            bus: Bus::High,
            stp: 0,
            batch: false,
            adapter_clock: Arc::new(AtomicU64::new(0f64.to_bits())),
            uds_units: make_uds_units(),
            engine_session: 0x01,
            engine_last_request: 0.0,
            cleared: false,
            clear_run: 0.0,
            clear_km: 0.0,
            truck: (options_vehicle == DemoVehicle::Truck).then(|| {
                let truck = TruckSim::new();
                if odometer_mismatch {
                    truck.with_odometer_mismatch()
                } else {
                    truck
                }
            }),
            cp: 0x18,
            header24: 0xEA_FF_F9,
            pb: (0xE0, 0x04),
            stn_rate: 0,
            jhf: true,
            js: false,
        }
    }

    fn now(&self) -> f64 {
        match &self.options.clock {
            Some(clock) => clock(),
            None => self.origin.elapsed().as_secs_f64(),
        }
    }

    pub fn line_end(&self) -> &'static str {
        if self.linefeeds { "\r\n" } else { "\r" }
    }

    fn supports(&self, level: i32) -> bool {
        self.profile.level >= level
    }

    /// The longest command line the adapter takes: the ELM327 buffer, or
    /// the 1024 characters of an STN chip (enough for batched commands).
    pub fn line_limit(&self) -> usize {
        if self.profile.stn_id.is_some() {
            1024
        } else {
            64
        }
    }

    fn spend(&self, ms: f64) {
        let total = f64::from_bits(self.adapter_clock.load(Ordering::SeqCst)) + ms;
        self.adapter_clock.store(total.to_bits(), Ordering::SeqCst);
    }

    pub fn emit(&mut self, text: Vec<u8>, delay_ms: f64) {
        let micros = (delay_ms * self.options.latency_scale * 1000.0) as i64;
        let mut ready = Instant::now() + Duration::from_micros(micros.max(0) as u64);
        if let Some(back) = self.output.back()
            && back.ready_at > ready
        {
            ready = back.ready_at;
        }
        self.output.push_back(Chunk {
            ready_at: ready,
            text,
        });
    }

    fn reset_settings(&mut self) {
        self.echo = true;
        self.linefeeds = false;
        self.spaces = true;
        self.headers = false;
        self.auto_format = true;
        self.show_dlc = false;
        self.protocol = 0;
        self.auto_search = true;
        self.active = false;
        self.header = 0x7DF;
        self.receive_filter = -1;
        self.can_filter = -1;
        self.can_mask = 0x7FF;
        self.monitoring = false;
        self.bus = Bus::High;
        self.stp = 0;
        self.batch = false; // volatile, a reset ends it
        self.cp = 0x18;
        self.header24 = 0xEA_FF_F9;
        self.pb = (0xE0, 0x04);
        self.stn_rate = 0;
        self.jhf = true;
        self.js = false;
    }

    // ---- output formatting -----------------------------------------------

    fn byte_text(&self, data: &[u8]) -> String {
        let mut out = String::new();
        for &b in data {
            out += &hex2(b as i32);
            if self.spaces {
                out.push(' ');
            }
        }
        out
    }

    fn format_reply(&self, reply: &Reply, lines: &mut Vec<String>) {
        let frames = iso_tp_frames(&reply.payload);
        let gap = if self.spaces { " " } else { "" };
        if self.headers || !self.auto_format {
            let mut prefix = String::new();
            if self.headers {
                prefix = hex3(reply.id) + gap;
                if self.show_dlc {
                    prefix += &format!("8{gap}");
                }
            }
            for frame in &frames {
                lines.push(prefix.clone() + &self.byte_text(frame));
            }
            return;
        }
        if frames.len() == 1 {
            lines.push(self.byte_text(&reply.payload));
            return;
        }
        // Without headers the ELM327 announces the total length and numbers
        // the lines, dropping the PCI bytes. Automatic formatting also drops
        // the padding of the last frame. With headers on (above) every frame
        // is shown whole, PCI and padding included.
        lines.push(hex3(reply.payload.len() as i32));
        let mut remaining = reply.payload.len();
        for (i, frame) in frames.iter().enumerate() {
            let mut line = String::new();
            line.push(hex_digit(i & 0xF));
            line += ":";
            line += gap;
            let offset = if i == 0 { 2 } else { 1 };
            let count = remaining.min(8 - offset);
            line += &self.byte_text(&frame[offset..offset + count]);
            remaining -= count;
            lines.push(line);
        }
    }

    fn passes_filter(&self, id: i32) -> bool {
        if self.receive_filter >= 0 {
            return id == self.receive_filter;
        }
        if self.can_filter >= 0 {
            return (id & self.can_mask) == (self.can_filter & self.can_mask);
        }
        true
    }

    // ---- OBD requests ----------------------------------------------------

    fn obd_request(&mut self, hex: &str) -> Response {
        if self.is_truck() {
            // The truck speaks J1939 only.
            return Response::new(&["NO DATA"], 120.0);
        }
        let mut hex: String = hex.to_string();
        let mut response = Response::empty();
        let mut count_hint = 0;
        if hex.len() % 2 == 1 {
            // A trailing single digit tells the adapter how many answers to
            // wait for (v1.3 and later), which saves the final timeout.
            if !self.supports(V13) {
                return self.unknown();
            }
            count_hint = hex_value(hex.chars().last().unwrap_or('0'));
            if count_hint == 0 {
                return self.unknown();
            }
            hex.pop();
        }
        let digits: Vec<char> = hex.chars().collect();
        let mut data: Bytes = Vec::new();
        let mut i = 0;
        while i + 1 < digits.len() {
            data.push((hex_value(digits[i]) * 16 + hex_value(digits[i + 1])) as u8);
            i += 2;
        }
        if !self.auto_format {
            // Without CAN auto formatting the first byte is the PCI byte.
            let length = data.first().map_or(0, |&b| b as usize);
            if !(1..=7).contains(&length) || length + 1 > data.len() {
                return Response::new(&["?"], 2.0);
            }
            data = data[1..1 + length].to_vec();
        }
        if data.is_empty() || data.len() > 7 {
            return Response::new(&["?"], 2.0);
        }

        // Protocol: the car only speaks protocol 6.
        if !self.active {
            if self.auto_search {
                response.lines.push("SEARCHING...".to_string());
                response.delay_ms += 80.0;
            } else if self.protocol != 6 {
                return match self.protocol {
                    3 | 4 => Response::new(&["BUS INIT: ...ERROR"], 300.0),
                    5 => Response::new(&["BUS INIT: ERROR"], 200.0),
                    8 | 9 | 11 | 12 => Response::new(&["CAN ERROR"], 150.0),
                    _ => Response::new(&["NO DATA"], 120.0),
                };
            }
            self.active = true;
        }

        self.last_frame = vec![0x00; 8];
        self.last_frame[0] = data.len() as u8;
        self.last_frame[1..1 + data.len()].copy_from_slice(&data);

        let t = self.now();
        let functional = self.header == 0x7DF;
        let mut replies: Vec<Reply> = Vec::new();
        let unit = if functional {
            None
        } else {
            self.uds_unit(self.header)
        };
        let uds_target =
            unit.filter(|&index| is_uds_service(data[0] as i32) || !self.uds_units[index].obd);
        if let Some(index) = uds_target {
            // A physical UDS request: only the addressed unit answers.
            let answers = self.uds_service(index, &data, t);
            let id = self.uds_units[index].response;
            if answers.len() > 1 {
                response.delay_ms += 60.0; // the "pending" pause
            }
            for payload in answers {
                replies.push(Reply { id, payload });
            }
        } else if self.bus == Bus::High {
            // Engine and gearbox sit on the high speed CAN only.
            if (functional || self.header == 0x7E0)
                && let Some(payload) = self.engine(&data, !functional, t)
            {
                replies.push(Reply { id: 0x7E8, payload });
            }
            if self.options.transmission_ecu
                && (functional || self.header == 0x7E1)
                && let Some(payload) = self.gearbox(&data, !functional, t)
            {
                replies.push(Reply { id: 0x7E9, payload });
            }
        }
        replies.retain(|r| self.passes_filter(r.id));
        if count_hint > 0 && replies.len() as i32 > count_hint {
            replies.truncate(count_hint as usize);
        }

        if replies.is_empty() {
            response.lines.push("NO DATA".to_string());
            response.delay_ms += 40.0;
            return response;
        }
        let mut frames = 0usize;
        for reply in &replies {
            self.format_reply(reply, &mut response.lines);
            frames += iso_tp_frames(&reply.payload).len();
        }
        // Without a count hint the adapter waits a little for further answers.
        response.delay_ms += 6.0 + frames as f64 + if count_hint > 0 { 0.0 } else { 12.0 };
        response
    }

    fn execute(&mut self, command: &str) -> Response {
        if self.batch && command.contains('|') {
            return self.batched(command);
        }
        self.execute_one(command)
    }

    fn execute_one(&mut self, command: &str) -> Response {
        if command.starts_with("AT") {
            return self.at_command(after(command, 2));
        }
        if self.profile.stn_id.is_some() && command.starts_with("ST") {
            return self.stn_command(command);
        }
        if all_hex(command) {
            if let Some(r) = self.j1939_line(command) {
                return r;
            }
            return self.obd_request(command);
        }
        Response::new(&["?"], 2.0)
    }

    pub fn handle_line(&mut self, raw: &[u8]) {
        let mut command = String::new();
        for &ch in raw {
            if ch == b' ' || ch == b'\t' {
                continue;
            }
            command.push(ch.to_ascii_uppercase() as char);
        }
        let end = self.line_end();
        // The echo goes out before the command changes anything (ATE0 is
        // still echoed).
        if self.echo {
            let mut text = raw.to_vec();
            text.extend_from_slice(end.as_bytes());
            self.emit(text, 0.0);
        }
        if command.is_empty() {
            // A bare carriage return repeats the last command.
            if self.last_command.is_empty() {
                self.emit(b">".to_vec(), 1.0);
                return;
            }
            command = self.last_command.clone();
        }
        self.last_command = command.clone();

        let r = self.execute(&command);
        self.spend(self.options.round_trip_ms + r.delay_ms);
        if !r.prompt {
            return;
        }
        let end = self.line_end();
        let mut text = String::new();
        for line in &r.lines {
            text += line;
            text += end;
        }
        text += end;
        text += ">";
        self.emit(text.into_bytes(), r.delay_ms);
    }
}
