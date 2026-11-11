//! The adapter and the control units behind it.

use std::time::{Duration, Instant};

use crate::codec::*;
use crate::sim::*;
use crate::{Chunk, DemoAdapter, DemoOptions, Output};

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
}

fn profile_for(adapter: DemoAdapter) -> Profile {
    match adapter {
        DemoAdapter::GenuineV14b => Profile {
            version: "ELM327 v1.4b",
            description: "OBDII to RS232 Interpreter",
            level: V14B,
            stn_id: None,
            stn_device: None,
            ok_to_all: false,
        },
        DemoAdapter::CloneV21 => Profile {
            version: "ELM327 v2.1",
            description: "OBDII to RS232 Interpreter",
            level: V14,
            stn_id: None,
            stn_device: None,
            ok_to_all: false,
        },
        DemoAdapter::CloneOkToAll => Profile {
            version: "ELM327 v2.1",
            description: "OBDII to RS232 Interpreter",
            level: V14,
            stn_id: None,
            stn_device: None,
            ok_to_all: true,
        },
        DemoAdapter::ObdLinkStn => Profile {
            version: "ELM327 v1.4b",
            description: "SCANTOOL.NET LLC",
            level: V14B,
            stn_id: Some("STN1130 v5.6.19"),
            stn_device: Some("OBDLink SX r4.2"),
            ok_to_all: false,
        },
        DemoAdapter::CloneV15 => Profile {
            version: "ELM327 v1.5",
            description: "OBDII to RS232 Interpreter",
            level: V14,
            stn_id: None,
            stn_device: None,
            ok_to_all: false,
        },
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
// UDS (ISO 14229) control units
// ---------------------------------------------------------------------------

// Status bits of a UDS fault (ISO 14229-1 D.2) as the demo uses them.
const UDS_ACTIVE: i32 = 0xAF; // failing now, pending, confirmed, warning lamp on
const UDS_PENDING: i32 = 0x24; // pending, failed since the last clear
const UDS_STORED: i32 = 0x28; // confirmed, not failing right now
const UDS_ENGINE_STORED: i32 = 0xA8; // confirmed with the MIL on

#[derive(Clone)]
struct UdsFault {
    code: &'static str,
    failure_type: i32,
    status: i32,
}

struct UdsDid {
    id: i32,
    text: &'static str,
}

/// One control unit that answers UDS on its own request id. The engine and
/// the gearbox also speak OBD; the others are reachable only physically, the
/// way body and chassis units are on a real car. Every id follows the common
/// "response = request + 8" convention, so a generic sweep finds them too.
struct UdsUnit {
    request: i32,
    response: i32,
    obd: bool, // also answers the OBD services (engine, gearbox)
    dids: Vec<UdsDid>,
    faults: Vec<UdsFault>, // fault memory; the engine computes its own
    slow_fault_read: bool, // answers 19 02 with 7F 19 78 first
}

fn did(id: i32, text: &'static str) -> UdsDid {
    UdsDid { id, text }
}

fn fault(code: &'static str, failure_type: i32, status: i32) -> UdsFault {
    UdsFault {
        code,
        failure_type,
        status,
    }
}

fn make_uds_units() -> Vec<UdsUnit> {
    // Fictional part numbers in the usual "three groups and an index" style.
    vec![
        UdsUnit {
            request: 0x7E0,
            response: 0x7E8,
            obd: true,
            dids: vec![
                did(0xF187, "CX5 906 018 B"),
                did(0xF189, "0417"),
                did(0xF191, "H05"),
                did(0xF197, "ECM 1.4 MPI   "),
                did(0xF18C, "EM21A0048217"),
                did(0xF190, VIN),
            ],
            faults: vec![],
            slow_fault_read: false,
        },
        UdsUnit {
            request: 0x7E1,
            response: 0x7E9,
            obd: true,
            dids: vec![
                did(0xF187, "CX6 927 156 A"),
                did(0xF189, "0203"),
                did(0xF191, "H03"),
                did(0xF197, "TCM AT6"),
                did(0xF18C, "TA21B0009144"),
            ],
            faults: vec![],
            slow_fault_read: false,
        },
        // ABS: the left front wheel speed sensor reads a voltage out of range.
        UdsUnit {
            request: 0x760,
            response: 0x768,
            obd: false,
            dids: vec![
                did(0xF187, "CX1 614 517 C"),
                did(0xF189, "0122"),
                did(0xF191, "H11"),
                did(0xF197, "ABS/ESP 9.3"),
                did(0xF18C, "AB21C0031877"),
                did(0xF190, VIN),
            ],
            faults: vec![fault("C0035", 0x1C, UDS_ACTIVE)],
            slow_fault_read: true,
        },
        // Airbag: resistance of the driver airbag squib too low, seen once.
        UdsUnit {
            request: 0x737,
            response: 0x73F,
            obd: false,
            dids: vec![
                did(0xF187, "CX0 959 655 F"),
                did(0xF189, "0310"),
                did(0xF191, "H07"),
                did(0xF197, "AIRBAG 8CH"),
                did(0xF18C, "SR21D0007730"),
                did(0xF190, VIN),
            ],
            faults: vec![fault("B0001", 0x1A, UDS_PENDING)],
            slow_fault_read: false,
        },
        // Instrument cluster: nothing stored.
        UdsUnit {
            request: 0x720,
            response: 0x728,
            obd: false,
            dids: vec![
                did(0xF187, "CX2 920 870 D"),
                did(0xF189, "0805"),
                did(0xF191, "H22"),
                did(0xF197, "INSTRUMENTS"),
                did(0xF18C, "IC21A0102265"),
                did(0xF190, VIN),
            ],
            faults: vec![],
            slow_fault_read: false,
        },
        // Climate: lost the engine on the bus once (a weak battery at a cold
        // start does that). It keeps its versions in F195 and F193 instead of
        // F189 and F191, as some makers do.
        UdsUnit {
            request: 0x744,
            response: 0x74C,
            obd: false,
            dids: vec![
                did(0xF187, "CX8 907 044 A"),
                did(0xF195, "0109"),
                did(0xF193, "H04"),
                did(0xF197, "CLIMATE 2Z"),
                did(0xF18C, "HV21B0055102"),
                did(0xF190, VIN),
            ],
            faults: vec![fault("U0100", 0x00, UDS_STORED)],
            slow_fault_read: false,
        },
    ]
}

fn is_uds_service(sid: i32) -> bool {
    sid == 0x14 || sid == 0x19 || sid == 0x22 || sid == 0x3E
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

    // The control units that answer UDS, with their own fault memory.
    uds_units: Vec<UdsUnit>,

    // Fault memory state.
    cleared: bool,
    clear_run: f64, // engine run time at the moment of clearing
    clear_km: f64,
}

impl Impl {
    pub fn new(options: DemoOptions) -> Self {
        let profile = profile_for(options.adapter);
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
            uds_units: make_uds_units(),
            cleared: false,
            clear_run: 0.0,
            clear_km: 0.0,
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
    }

    // ---- fault state over time ------------------------------------------

    fn run_since_clear(&self, t: f64) -> f64 {
        if !self.cleared {
            return NEVER;
        }
        (engine_run_time(t, self.options.engine_start_seconds) - self.clear_run).max(0.0)
    }

    fn stored_codes(&self, t: f64) -> Vec<&'static str> {
        if !self.cleared {
            return vec!["P0171", "P0420"];
        }
        let r = self.run_since_clear(t);
        let mut codes = Vec::new();
        if r >= LEAN_CONFIRMED {
            codes.push("P0171");
        }
        if r >= CATALYST_CONFIRMED {
            codes.push("P0420");
        }
        codes
    }

    fn pending_codes(&self, t: f64) -> Vec<&'static str> {
        if !self.cleared {
            return vec!["P0300"];
        }
        let r = self.run_since_clear(t);
        let mut codes = Vec::new();
        if r >= MISFIRE_BACK {
            codes.push("P0300");
        }
        if r >= LEAN_PENDING {
            codes.push("P0171");
        }
        if r >= CATALYST_DONE {
            codes.push("P0420");
        }
        codes
    }

    // Permanent codes survive mode 04 on purpose: only a passed monitor
    // removes them, which is the whole point of mode 0A.
    fn permanent_codes(&self, _t: f64) -> Vec<&'static str> {
        vec!["P0171", "P0420"]
    }

    // Freeze frame 0 belongs to the first fault after clearing. Before the
    // demo is cleared it shows an older moment from a warm cruise. Frame 1
    // holds the next fault, the way control units with several frames do:
    // the pending misfire before clearing, the lean mixture after it.
    fn freeze_frame(&self, t: f64, frame: i32) -> Option<(Sample, &'static str)> {
        let start = self.options.engine_start_seconds;
        if !(0..=1).contains(&frame) {
            return None;
        }
        if !self.cleared {
            return Some(if frame == 0 {
                let mut sample = simulate(start + CRANK_SECONDS + 1300.0, start);
                sample.stft = 7.0;
                (sample, "P0171")
            } else {
                (simulate(start + CRANK_SECONDS + 1520.0, start), "P0300")
            });
        }
        let r = self.run_since_clear(t);
        if frame == 0 {
            if r < MISFIRE_BACK {
                return None;
            }
            let sample = simulate(start + CRANK_SECONDS + self.clear_run + MISFIRE_BACK, start);
            return Some((sample, "P0300"));
        }
        if r < LEAN_PENDING {
            return None;
        }
        let mut sample = simulate(start + CRANK_SECONDS + self.clear_run + LEAN_PENDING, start);
        sample.stft = 7.0;
        Some((sample, "P0171"))
    }

    // Readiness: supported monitors in byte C, incomplete ones in byte D.
    fn incomplete_monitors(r: f64) -> u8 {
        let mut d = 0x04; // evaporative system: needs a cold soak, never done here
        if r < CATALYST_DONE {
            d |= 0x01;
        }
        if r < OXYGEN_DONE {
            d |= 0x20;
        }
        if r < HEATER_DONE {
            d |= 0x40;
        }
        if r < EGR_DONE {
            d |= 0x80;
        }
        d
    }

    fn readiness_d(&self, t: f64) -> u8 {
        if !self.cleared {
            return 0x84; // EVAP and EGR/VVT not yet completed
        }
        Self::incomplete_monitors(self.run_since_clear(t))
    }

    fn monitor_complete(&self, mid: i32, t: f64) -> bool {
        let r = self.run_since_clear(t);
        match mid {
            0x01 | 0x02 => r >= OXYGEN_DONE,
            0x21 => r >= CATALYST_DONE,
            0x3C => false,
            0x41 => r >= HEATER_DONE,
            _ => true,
        }
    }

    // ---- engine control unit ---------------------------------------------

    fn engine_pid(&self, pid: i32, t: f64) -> Option<Bytes> {
        let start = self.options.engine_start_seconds;
        let s = simulate(t, start);
        let run = s.run_time;
        let r = self.run_since_clear(t);
        let mut d: Bytes = Vec::new();
        match pid {
            0x01 => {
                let stored = self.stored_codes(t);
                let mil = if stored.is_empty() { 0 } else { 0x80 };
                d = vec![mil | stored.len() as u8, 0x07, 0xE5, self.readiness_d(t)];
            }
            0x03 => d = vec![s.fuel_status as u8, 0x00],
            0x04 => d = vec![percent(s.load)],
            0x05 => d = vec![u8_of(s.coolant + 40.0)],
            0x06 => d = vec![trim_byte(s.stft)],
            0x07 => d = vec![trim_byte(s.ltft)],
            0x0B => d = vec![u8_of(s.map)],
            0x0C => put16(&mut d, s.rpm * 4.0),
            0x0D => d = vec![u8_of(s.speed)],
            0x0E => d = vec![u8_of((s.timing + 64.0) * 2.0)],
            0x0F => d = vec![u8_of(s.iat + 40.0)],
            0x10 => put16(&mut d, s.maf * 100.0),
            0x11 => d = vec![percent(s.throttle_abs)],
            0x13 => d = vec![0x03], // bank 1, sensors 1 and 2
            0x14 => {
                d = vec![
                    u8_of(s.o2up * 200.0),
                    if s.fuel_status == 0x02 {
                        trim_byte(s.stft)
                    } else {
                        0xFF
                    },
                ];
            }
            0x15 => d = vec![u8_of(s.o2down * 200.0), 0xFF],
            0x1C => d = vec![0x06], // EOBD
            0x1F => put16(&mut d, run),
            0x21 => {
                let mut km = 0.0;
                if !self.cleared {
                    km = 312.0 + s.distance_km;
                } else if r >= LEAN_CONFIRMED {
                    let on = simulate(
                        start + CRANK_SECONDS + self.clear_run + LEAN_CONFIRMED,
                        start,
                    );
                    km = s.distance_km - on.distance_km;
                }
                put16(&mut d, km);
            }
            0x23 => put16(&mut d, s.rail / 10.0),
            0x2F => d = vec![percent(s.fuel_level)],
            0x30 => {
                let warm_at = warm_up_run_time();
                let warmed_now = run >= warm_at;
                let mut count: i32 = if self.cleared { 0 } else { 36 };
                if warmed_now && (!self.cleared || self.clear_run < warm_at) {
                    count += 1;
                }
                d = vec![count as u8];
            }
            0x31 => put16(
                &mut d,
                if self.cleared {
                    s.distance_km - self.clear_km
                } else {
                    1842.0 + s.distance_km
                },
            ),
            0x33 => d = vec![u8_of(BARO)],
            0x3C => put16(&mut d, (s.cat_temp + 40.0) * 10.0),
            0x41 => {
                // This drive cycle: the same monitors, counted from engine start.
                let done = if s.phase == Phase::Off || s.phase == Phase::Crank {
                    0xE1
                } else {
                    Self::incomplete_monitors(run)
                };
                d = vec![0x00, 0x07, 0xE1, done & 0xE1];
            }
            0x42 => put16(&mut d, (s.battery - 0.25) * 1000.0),
            0x43 => put16(&mut d, s.load * 0.95 * 255.0 / 100.0),
            0x44 => put16(&mut d, s.lambda * 32768.0),
            0x45 => d = vec![percent(s.throttle_rel)],
            0x46 => d = vec![u8_of(AMBIENT + 40.0)],
            0x47 => d = vec![percent(s.throttle_abs + 1.2)],
            0x49 => d = vec![percent(s.pedal_d)],
            0x4A => d = vec![percent(s.pedal_e)],
            0x4C => d = vec![percent(s.throttle_cmd)],
            0x4D => {
                let mut minutes = 0.0;
                if !self.cleared {
                    minutes = 1270.0 + run / 60.0;
                } else if r >= LEAN_CONFIRMED {
                    minutes = (r - LEAN_CONFIRMED) / 60.0;
                }
                put16(&mut d, minutes.floor());
            }
            0x4E => {
                let minutes = if self.cleared {
                    r / 60.0
                } else {
                    2580.0 + run / 60.0
                };
                put16(&mut d, minutes.floor());
            }
            0x51 => d = vec![0x01], // petrol
            0x5C => d = vec![u8_of(s.oil + 40.0)],
            0x5E => put16(&mut d, s.fuel_rate * 20.0),
            0x61 => d = vec![u8_of(s.torque_demand + 125.0)],
            0x62 => d = vec![u8_of(s.torque_actual + 125.0)],
            0x63 => put16(&mut d, 240.0), // reference torque, Nm
            0xA6 => put32(&mut d, s.odometer * 10.0),
            _ => return None,
        }
        Some(d)
    }

    // The same PIDs from a frozen moment, for mode 02.
    fn freeze_pid(pid: i32, s: &Sample, code: &str) -> Option<Bytes> {
        let mut d: Bytes = Vec::new();
        match pid {
            0x02 => put_dtc(&mut d, code),
            0x03 => d = vec![s.fuel_status as u8, 0x00],
            0x04 => d = vec![percent(s.load)],
            0x05 => d = vec![u8_of(s.coolant + 40.0)],
            0x06 => d = vec![trim_byte(s.stft)],
            0x07 => d = vec![trim_byte(s.ltft)],
            0x0B => d = vec![u8_of(s.map)],
            0x0C => put16(&mut d, s.rpm * 4.0),
            0x0D => d = vec![u8_of(s.speed)],
            0x0E => d = vec![u8_of((s.timing + 64.0) * 2.0)],
            0x0F => d = vec![u8_of(s.iat + 40.0)],
            0x10 => put16(&mut d, s.maf * 100.0),
            0x11 => d = vec![percent(s.throttle_abs)],
            _ => return None,
        }
        Some(d)
    }

    // Mode 06 records: MID, TID, unit and scaling ID, value, minimum, maximum.
    fn monitor_records(&self, mid: i32, t: f64, out: &mut Bytes) {
        let done = self.monitor_complete(mid, t);
        let mut record = |tid: i32, uas: i32, value: i32, low: i32, high: i32| {
            out.push(mid as u8);
            out.push(tid as u8);
            out.push(uas as u8);
            // A test that has not run since clearing reports zeros.
            let (value, low, high) = if done { (value, low, high) } else { (0, 0, 0) };
            put16(out, value as f64);
            put16(out, low as f64);
            put16(out, high as f64);
        };
        let s = simulate(t, self.options.engine_start_seconds);
        let r = self.run_since_clear(t);
        // Misfire counters: cylinder 3 misfires now and then (P0300).
        let cycle_run = s.run_time.min(r);
        let current: [i32; 4] = [
            (cycle_run * 0.002) as i32,
            0,
            (cycle_run * 0.06) as i32,
            (cycle_run * 0.004) as i32,
        ];
        let average: [i32; 4] = if self.cleared {
            [
                1.min((r / 300.0) as i32),
                0,
                41.min((r / 15.0) as i32),
                2.min((r / 200.0) as i32),
            ]
        } else {
            [1, 0, 41, 2]
        };
        match mid {
            0x01 => {
                // O2 sensor bank 1 sensor 1: switch times in ms
                record(0x05, 0x10, 68, 0, 150);
                record(0x06, 0x10, 74, 0, 150);
            }
            0x02 => {
                // O2 sensor bank 1 sensor 2: min and max voltage in mV
                record(0x07, 0x0B, 84, 0, 400);
                record(0x08, 0x0B, 812, 600, 1100);
            }
            0x21 => {
                // catalyst bank 1: oxygen storage ratio, fails (P0420)
                record(0x80, 0x20, 210, 0, 154);
            }
            0x3C => {
                // EVAP 0.020 inch leak: never completed
                record(0x80, 0x01, 0, 0, 0);
            }
            0x41 => {
                // O2 heater bank 1 sensor 1: warm-up time in ms
                record(0x80, 0x10, 14200, 0, 40000);
            }
            0xA1 => {
                let sum_average: i32 = average.iter().sum();
                let sum_current: i32 = current.iter().sum();
                record(0x0B, 0x24, sum_average, 0, 0xFFFF);
                record(0x0C, 0x24, sum_current, 0, 0xFFFF);
            }
            0xA2..=0xA5 => {
                let cylinder = (mid - 0xA2) as usize;
                record(0x0B, 0x24, average[cylinder], 0, 0xFFFF);
                record(0x0C, 0x24, current[cylinder], 0, 0xFFFF);
            }
            _ => {}
        }
    }

    // Copies `text` (embedded zero bytes included) and pads with zero bytes
    // up to `field`.
    fn put_text(out: &mut Bytes, text: &[u8], field: usize) {
        out.extend_from_slice(text);
        for _ in text.len()..field {
            out.push(0x00);
        }
    }

    // Copies up to `length` characters, stopping at a zero byte, and pads
    // with zero bytes up to `length`.
    fn put_padded(out: &mut Bytes, text: &[u8], length: usize) {
        let mut i = 0;
        while i < text.len() && text[i] != 0 && i < length {
            out.push(text[i]);
            i += 1;
        }
        while i < length {
            out.push(0x00);
            i += 1;
        }
    }

    fn code_list(sid: i32, codes: &[&str]) -> Option<Bytes> {
        let mut out = vec![(sid + 0x40) as u8, codes.len() as u8];
        for code in codes {
            put_dtc(&mut out, code);
        }
        Some(out)
    }

    fn range_or_pids(
        request: &[u8],
        ids: &[i32],
        value: &dyn Fn(i32) -> Option<Bytes>,
    ) -> Option<Bytes> {
        let mut out: Bytes = vec![0x41];
        for &byte in &request[1..] {
            let pid = byte as i32;
            if pid % 0x20 == 0 {
                if !range_reachable(ids, pid) {
                    continue;
                }
                let mask = support_mask(ids, pid);
                out.push(pid as u8);
                out.extend_from_slice(&mask);
                continue;
            }
            if !ids.contains(&pid) {
                continue;
            }
            let Some(data) = value(pid) else { continue };
            out.push(pid as u8);
            out.extend_from_slice(&data);
        }
        if out.len() == 1 {
            return None;
        }
        Some(out)
    }

    fn clear_engine(&mut self, t: f64) {
        let start = self.options.engine_start_seconds;
        self.cleared = true;
        self.clear_run = engine_run_time(t, start);
        self.clear_km = simulate(t, start).distance_km;
    }

    // ---- UDS -------------------------------------------------------------

    fn uds_unit(&self, request: i32) -> Option<usize> {
        if request == 0x7E1 && !self.options.transmission_ecu {
            return None;
        }
        self.uds_units.iter().position(|u| u.request == request)
    }

    // The engine's UDS fault memory is the same as its OBD one: stored codes
    // are confirmed with the MIL on, pending ones carry the pending bit.
    fn uds_faults(&self, unit: &UdsUnit, t: f64) -> Vec<UdsFault> {
        if unit.request != 0x7E0 {
            return unit.faults.clone();
        }
        let mut out: Vec<UdsFault> = Vec::new();
        let mut add = |code: &'static str, bits: i32| {
            for f in out.iter_mut() {
                if f.code == code {
                    f.status |= bits;
                    return;
                }
            }
            out.push(UdsFault {
                code,
                failure_type: 0x00,
                status: bits,
            });
        };
        for code in self.stored_codes(t) {
            add(code, UDS_ENGINE_STORED);
        }
        for code in self.pending_codes(t) {
            add(code, UDS_PENDING);
        }
        out
    }

    // Maker values of the engine (service 22), the kind a custom value or a
    // pack reads (core/CustomPids.h). The identifiers are made up for the
    // demo; data/pids/caruspex-demo.csv describes them:
    //   1001  oil level, percent of the dipstick range, one byte
    //   1002  oil temperature in tenths of a degree, signed 16 bit
    //   1003  number of engine starts, 32 bit
    //   1004  status bits, bit 0 = engine running
    //   1010  operating data, 16 bytes and so a multi-frame answer: total
    //         engine run time in seconds (32 bit), four reserved bytes,
    //         supply voltage in millivolts (16 bit), six reserved bytes
    fn engine_did(&self, id: i32, t: f64) -> Option<Bytes> {
        let s = simulate(t, self.options.engine_start_seconds);
        let running = s.run_time > 0.0;
        let mut out: Bytes = vec![0x62, (id >> 8) as u8, (id & 0xFF) as u8];
        fn put(out: &mut Bytes, value: u32, bytes: i32) {
            for i in (0..bytes).rev() {
                out.push((value >> (8 * i)) as u8);
            }
        }
        match id {
            0x1001 => {
                // The oil drains back into the sump when the engine stops,
                // so the level reads higher at rest.
                let level = if running {
                    66.0 + 2.0 * (t * 0.3).sin()
                } else {
                    74.0
                };
                put(&mut out, level.round() as i64 as u32, 1);
                Some(out)
            }
            0x1002 => {
                let tenths = (s.oil * 10.0).round() as i64;
                put(&mut out, (tenths as i16) as u16 as u32, 2);
                Some(out)
            }
            0x1003 => {
                let started = if t >= self.options.engine_start_seconds {
                    1
                } else {
                    0
                };
                put(&mut out, 1834 + started, 4);
                Some(out)
            }
            0x1004 => {
                put(&mut out, if running { 0x01 } else { 0x00 }, 1);
                Some(out)
            }
            0x1010 => {
                put(&mut out, 2456000u32.wrapping_add(s.run_time as u32), 4);
                put(&mut out, 0, 4);
                put(&mut out, (s.battery * 1000.0).round() as i64 as u32, 2);
                put(&mut out, 0, 4);
                put(&mut out, 0, 2);
                Some(out)
            }
            _ => None,
        }
    }

    // Answers of one unit to a physical request: none, one, or two when it
    // first reports "response pending" (7F xx 78).
    fn uds_service(&mut self, unit_index: usize, request: &[u8], t: f64) -> Vec<Bytes> {
        let sid = request[0];
        let negative = |nrc: u8| -> Bytes { vec![0x7F, sid, nrc] };
        let unit_request = self.uds_units[unit_index].request;
        match sid {
            0x3E => {
                // tester present
                if request.len() != 2 {
                    return vec![negative(0x13)];
                }
                if request[1] == 0x80 {
                    return vec![]; // positive answer suppressed
                }
                if request[1] != 0x00 {
                    return vec![negative(0x12)];
                }
                vec![vec![0x7E, 0x00]]
            }
            0x22 => {
                // read data by identifier
                if request.len() != 3 {
                    return vec![negative(0x13)];
                }
                let id = (request[1] as i32) << 8 | request[2] as i32;
                if unit_request == 0x7E0
                    && let Some(value) = self.engine_did(id, t)
                {
                    return vec![value];
                }
                for did in &self.uds_units[unit_index].dids {
                    if did.id != id {
                        continue;
                    }
                    let mut out: Bytes = vec![0x62, request[1], request[2]];
                    out.extend_from_slice(did.text.as_bytes());
                    return vec![out];
                }
                vec![negative(0x31)]
            }
            0x19 => {
                // read DTC information, only "by status mask"
                if request.len() < 2 {
                    return vec![negative(0x13)];
                }
                if request[1] != 0x02 {
                    return vec![negative(0x12)];
                }
                if request.len() != 3 {
                    return vec![negative(0x13)];
                }
                let mut out: Bytes = vec![0x59, 0x02, 0xFF];
                let unit = &self.uds_units[unit_index];
                for f in self.uds_faults(unit, t) {
                    if (f.status & request[2] as i32) == 0 {
                        continue;
                    }
                    put_dtc(&mut out, f.code);
                    out.push(f.failure_type as u8);
                    out.push(f.status as u8);
                }
                if unit.slow_fault_read {
                    return vec![negative(0x78), out];
                }
                vec![out]
            }
            0x14 => {
                // clear diagnostic information
                if request.len() != 4 {
                    return vec![negative(0x13)];
                }
                if request[1] != 0xFF || request[2] != 0xFF || request[3] != 0xFF {
                    return vec![negative(0x31)];
                }
                if unit_request == 0x7E0 {
                    self.clear_engine(t);
                } else {
                    self.uds_units[unit_index].faults.clear();
                }
                vec![vec![0x54]]
            }
            _ => vec![negative(0x11)],
        }
    }

    fn engine(&mut self, request: &[u8], physical: bool, t: f64) -> Option<Bytes> {
        let sid = request[0] as i32;
        match sid {
            0x01 => {
                if request.len() < 2 {
                    return None;
                }
                Self::range_or_pids(request, ENGINE_PIDS, &|pid| self.engine_pid(pid, t))
            }
            0x02 => {
                if request.len() < 2 {
                    return None;
                }
                let pid = request[1] as i32;
                let frame = if request.len() > 2 {
                    request[2] as i32
                } else {
                    0
                };
                let frozen = self.freeze_frame(t, frame);
                let mut out: Bytes = vec![0x42, pid as u8, frame as u8];
                // An empty frame only answers the question which fault
                // stored it, with 00 00.
                let Some((sample, code)) = frozen else {
                    if pid != 0x02 {
                        return None;
                    }
                    out.push(0x00);
                    out.push(0x00);
                    return Some(out);
                };
                if pid % 0x20 == 0 {
                    if !range_reachable(FREEZE_FRAME_PIDS, pid) {
                        return None;
                    }
                    let mask = support_mask(FREEZE_FRAME_PIDS, pid);
                    out.extend_from_slice(&mask);
                    return Some(out);
                }
                let data = Self::freeze_pid(pid, &sample, code)?;
                out.extend_from_slice(&data);
                Some(out)
            }
            0x03 => Self::code_list(0x03, &self.stored_codes(t)),
            0x07 => Self::code_list(0x07, &self.pending_codes(t)),
            0x0A => Self::code_list(0x0A, &self.permanent_codes(t)),
            0x04 => {
                self.clear_engine(t);
                Some(vec![0x44])
            }
            0x06 => {
                if request.len() != 2 {
                    return None;
                }
                let mid = request[1] as i32;
                let mut out: Bytes = vec![0x46];
                if mid % 0x20 == 0 {
                    if !range_reachable(ENGINE_MIDS, mid) {
                        return None;
                    }
                    out.push(mid as u8);
                    let mask = support_mask(ENGINE_MIDS, mid);
                    out.extend_from_slice(&mask);
                    return Some(out);
                }
                if !ENGINE_MIDS.contains(&mid) {
                    return None;
                }
                self.monitor_records(mid, t, &mut out);
                Some(out)
            }
            0x09 => {
                if request.len() != 2 {
                    return None;
                }
                let item = request[1] as i32;
                let mut out: Bytes = vec![0x49, item as u8];
                match item {
                    0x00 => {
                        let mask = support_mask(ENGINE_INFO, 0);
                        out.extend_from_slice(&mask);
                        Some(out)
                    }
                    0x02 => {
                        out.push(0x01);
                        Self::put_text(&mut out, VIN.as_bytes(), 17);
                        Some(out)
                    }
                    0x04 => {
                        out.push(0x01);
                        Self::put_padded(&mut out, ENGINE_CALIBRATION, 16);
                        Some(out)
                    }
                    0x06 => {
                        out.push(0x01);
                        out.extend_from_slice(&ENGINE_CVN);
                        Some(out)
                    }
                    0x08 => {
                        // In-use performance tracking, 16 counters for a petrol engine:
                        // OBD conditions, ignition cycles, then completions and
                        // conditions per monitor (catalyst 1/2, O2 1/2, EGR, AIR, EVAP).
                        const COUNTERS: [i32; 16] = [
                            214, 612, 41, 198, 0, 0, 77, 198, 0, 0, 35, 198, 0, 0, 9, 198,
                        ];
                        out.push(0x10);
                        for c in COUNTERS {
                            put16(&mut out, c as f64);
                        }
                        Some(out)
                    }
                    0x0A => {
                        out.push(0x01);
                        Self::put_text(&mut out, ENGINE_NAME, 20);
                        Some(out)
                    }
                    _ => None,
                }
            }
            _ => {
                if physical {
                    return Some(vec![0x7F, sid as u8, 0x11]);
                }
                None
            }
        }
    }

    // ---- transmission control unit ---------------------------------------

    fn gearbox(&self, request: &[u8], physical: bool, t: f64) -> Option<Bytes> {
        let sid = request[0] as i32;
        match sid {
            0x01 => {
                if request.len() < 2 {
                    return None;
                }
                let s = simulate(t, self.options.engine_start_seconds);
                Self::range_or_pids(request, GEARBOX_PIDS, &|pid| {
                    if pid == 0x01 {
                        return Some(vec![0x00, 0x04, 0x00, 0x00]);
                    }
                    // PID A4: actual gear in the upper nibble, ratio in 1/1000.
                    let driving = s.phase != Phase::Off && s.phase != Phase::Crank;
                    let mut d: Bytes = vec![0x02, if driving { (s.gear << 4) as u8 } else { 0 }];
                    put16(&mut d, if driving { s.gear_ratio * 1000.0 } else { 0.0 });
                    Some(d)
                })
            }
            0x03 => Self::code_list(0x03, &[]),
            0x07 => Self::code_list(0x07, &[]),
            0x0A => Self::code_list(0x0A, &[]),
            0x04 => Some(vec![0x44]),
            0x09 => {
                if request.len() != 2 {
                    return None;
                }
                let item = request[1] as i32;
                let mut out: Bytes = vec![0x49, item as u8];
                match item {
                    0x00 => {
                        let mask = support_mask(GEARBOX_INFO, 0);
                        out.extend_from_slice(&mask);
                        Some(out)
                    }
                    0x04 => {
                        out.push(0x01);
                        Self::put_padded(&mut out, GEARBOX_CALIBRATION, 16);
                        Some(out)
                    }
                    0x06 => {
                        out.push(0x01);
                        out.extend_from_slice(&GEARBOX_CVN);
                        Some(out)
                    }
                    0x0A => {
                        out.push(0x01);
                        Self::put_text(&mut out, GEARBOX_NAME, 20);
                        Some(out)
                    }
                    _ => None,
                }
            }
            _ => {
                if physical {
                    return Some(vec![0x7F, sid as u8, 0x11]);
                }
                None
            }
        }
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
        } else {
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

    // ---- AT commands -----------------------------------------------------

    fn unknown(&self) -> Response {
        Response::one(if self.profile.ok_to_all { "OK" } else { "?" }, 2.0)
    }

    fn ok(&self, level: i32) -> Response {
        if !self.supports(level) {
            return self.unknown();
        }
        Response::new(&["OK"], 2.0)
    }

    fn flag(&mut self, level: i32, set: impl FnOnce(&mut Self)) -> Response {
        if !self.supports(level) {
            return self.unknown();
        }
        set(self);
        Response::new(&["OK"], 2.0)
    }

    fn describe_protocol(&self) -> String {
        let shown = if self.active { 6 } else { self.protocol };
        if !self.auto_search {
            return protocol_description(shown).to_string();
        }
        if shown == 0 {
            return "AUTO".to_string();
        }
        format!("AUTO, {}", protocol_description(shown))
    }

    // ATSP and ATTP: the same here, since nothing is stored anyway.
    fn set_protocol(&mut self, arg: &str) -> Response {
        let mut rest = arg;
        let mut automatic = false;
        if rest.len() == 2 && rest.starts_with('A') {
            automatic = true;
            rest = after(rest, 1);
        }
        if rest == "00" && self.supports(V14) {
            rest = "0";
        }
        let mut chars = rest.chars();
        let (Some(digit), None) = (chars.next(), chars.next()) else {
            return self.unknown();
        };
        if !is_hex(digit) || hex_value(digit) > 12 {
            return self.unknown();
        }
        self.protocol = hex_value(digit);
        self.auto_search = automatic || self.protocol == 0;
        self.active = false;
        Response::new(&["OK"], 2.0)
    }

    fn programmable_parameters(&self) -> Response {
        // Factory values of the parameters the ELM327 documents; everything
        // else is shown as FF (off), as on most adapters.
        fn value(index: i32) -> i32 {
            match index {
                0x03 => 0x32,
                0x04 => 0x01,
                0x06 => 0xF1,
                0x07 => 0x09,
                0x0A => 0x0A,
                0x0C => 0x68,
                0x0D => 0x0D,
                0x0E => 0x9A,
                0x10 => 0x0D,
                0x11 => 0x00,
                0x13 => 0x32,
                0x16 => 0x92,
                0x17 => 0x92,
                0x18 => 0x00,
                0x19 => 0x28,
                0x1A => 0x0A,
                0x24 => 0x00,
                0x25 => 0x00,
                0x26 => 0x00,
                0x2A => 0x38,
                0x2B => 0x02,
                0x2C => 0xE0,
                0x2D => 0x04,
                0x2E => 0x80,
                0x2F => 0x0A,
                _ => 0xFF,
            }
        }
        let mut r = Response::empty();
        let mut row = 0;
        while row < 0x30 {
            let mut line = String::new();
            for i in row..row + 4 {
                if i > row {
                    line += "  ";
                }
                line += &(hex2(i) + ":" + &hex2(value(i)) + " F");
            }
            r.lines.push(line);
            row += 4;
        }
        r.delay_ms = 8.0;
        r
    }

    fn at_command(&mut self, c: &str) -> Response {
        if c == "Z" || c == "WS" {
            self.reset_settings();
            let version = self.profile.version;
            return Response::new(&["", "", version], if c == "Z" { 40.0 } else { 20.0 });
        }
        if c == "D" {
            let keep_echo = self.echo;
            self.reset_settings();
            self.echo = keep_echo;
            return Response::new(&["OK"], 2.0);
        }
        if c == "I" {
            return Response::new(&[self.profile.version], 2.0);
        }
        if c == "@1" {
            return Response::new(&[self.profile.description], 2.0);
        }
        if c == "@2" {
            return if self.supports(V13) {
                Response::new(&["?"], 2.0)
            } else {
                self.unknown()
            };
        }
        if c.starts_with("@3") && c.len() == 14 {
            return self.ok(V13); // stored nowhere here
        }
        match c {
            "E0" => return self.flag(V10, |s| s.echo = false),
            "E1" => return self.flag(V10, |s| s.echo = true),
            "L0" => return self.flag(V10, |s| s.linefeeds = false),
            "L1" => return self.flag(V10, |s| s.linefeeds = true),
            "S0" => return self.flag(V13, |s| s.spaces = false),
            "S1" => return self.flag(V13, |s| s.spaces = true),
            "H0" => return self.flag(V10, |s| s.headers = false),
            "H1" => return self.flag(V10, |s| s.headers = true),
            "D0" => return self.flag(V13, |s| s.show_dlc = false),
            "D1" => return self.flag(V13, |s| s.show_dlc = true),
            "CAF0" => return self.flag(V10, |s| s.auto_format = false),
            "CAF1" => return self.flag(V10, |s| s.auto_format = true),
            _ => {}
        }
        if matches!(
            c,
            "M0" | "M1" | "R0" | "R1" | "CFC0" | "CFC1" | "AL" | "NL" | "BI" | "IB10" | "IB96"
        ) {
            return self.ok(V10);
        }
        if matches!(c, "V0" | "V1" | "JE" | "JS") {
            return self.ok(V13);
        }
        if matches!(c, "AT0" | "AT1" | "AT2" | "KW0" | "KW1") {
            return self.ok(V12);
        }
        if c == "FE" {
            return self.ok(V13A);
        }
        if matches!(
            c,
            "CSM0" | "CSM1" | "SS" | "LP" | "CEA" | "IB48" | "JTM1" | "JTM5" | "CV0000"
        ) {
            return self.ok(V14);
        }
        if c == "JHF0" || c == "JHF1" {
            return self.ok(V14B);
        }
        if c == "FCSH" || c == "FCSD" {
            return self.unknown(); // need arguments
        }
        if matches!(c, "FCSM0" | "FCSM1" | "FCSM2") {
            return self.ok(V11);
        }
        if c.starts_with("FCSH") && all_hex(after(c, 4)) && (c.len() == 7 || c.len() == 12) {
            return self.ok(V11);
        }
        if c.starts_with("FCSD")
            && all_hex(after(c, 4))
            && c.len() >= 6
            && c.len() <= 14
            && c.len().is_multiple_of(2)
        {
            return self.ok(V11);
        }
        if c == "AR" {
            if !self.supports(V12) {
                return self.unknown();
            }
            self.receive_filter = -1;
            self.can_filter = -1;
            return Response::new(&["OK"], 2.0);
        }
        if c == "CRA" {
            if !self.supports(V14B) {
                return self.unknown();
            }
            self.receive_filter = -1;
            self.can_filter = -1;
            return Response::new(&["OK"], 2.0);
        }
        if let Some(value) = hex_arg(c, "CRA", 3) {
            if !self.supports(V13) {
                return self.unknown();
            }
            self.receive_filter = value;
            return Response::new(&["OK"], 2.0);
        }
        if hex_arg(c, "CRA", 8).is_some() {
            if !self.supports(V13) {
                return self.unknown();
            }
            self.receive_filter = 0x7FFF_FFFF; // a 29 bit address: nothing on this bus matches
            return Response::new(&["OK"], 2.0);
        }
        if let Some(value) = hex_arg(c, "CF", 3) {
            self.can_filter = value;
            return Response::new(&["OK"], 2.0);
        }
        if let Some(value) = hex_arg(c, "CM", 3) {
            self.can_mask = value;
            if self.can_filter < 0 {
                self.can_filter = 0;
            }
            return Response::new(&["OK"], 2.0);
        }
        if hex_arg(c, "CF", 8).is_some()
            || hex_arg(c, "CM", 8).is_some()
            || hex_arg(c, "CP", 2).is_some()
            || hex_arg(c, "SW", 2).is_some()
            || hex_arg(c, "SR", 2).is_some()
            || hex_arg(c, "ST", 2).is_some()
        {
            return self.ok(V10);
        }
        if hex_arg(c, "IIA", 2).is_some() || hex_arg(c, "BRT", 2).is_some() {
            return self.ok(V12);
        }
        if hex_arg(c, "RA", 2).is_some() {
            return self.ok(V13);
        }
        if hex_arg(c, "TA", 2).is_some()
            || hex_arg(c, "CEA", 2).is_some()
            || hex_arg(c, "PB", 4).is_some()
        {
            return self.ok(V14);
        }
        if hex_arg(c, "CER", 2).is_some() || hex_arg(c, "AMT", 2).is_some() {
            return self.ok(V20);
        }
        if let Some(value) = hex_arg(c, "SD", 2) {
            if !self.supports(V14) {
                return self.unknown();
            }
            self.saved_byte = value;
            return Response::new(&["OK"], 2.0);
        }
        if c == "RD" {
            return if self.supports(V14) {
                Response::one(hex2(self.saved_byte), 2.0)
            } else {
                self.unknown()
            };
        }
        if c.starts_with("WM")
            && all_hex(after(c, 2))
            && c.len() >= 8
            && c.len() <= 14
            && c.len().is_multiple_of(2)
        {
            return self.ok(V10);
        }
        if c.starts_with("CV") && c.len() == 6 && after(c, 2).chars().all(|ch| ch.is_ascii_digit())
        {
            return self.ok(V10);
        }
        if let Some(value) = hex_arg(c, "SH", 3).or_else(|| hex_arg(c, "SH", 6)) {
            // With xxyyzz only the low 11 bits matter on an 11 bit bus.
            self.header = value & 0x7FF;
            return Response::new(&["OK"], 2.0);
        }
        if hex_arg(c, "SH", 8).is_some() {
            if !self.supports(V20) {
                return self.unknown();
            }
            self.header = -1; // 29 bit: nobody answers on this car
            return Response::new(&["OK"], 2.0);
        }
        if c.starts_with("SP") || c.starts_with("TP") {
            return self.set_protocol(after(c, 2));
        }
        if c == "DP" {
            return Response::one(self.describe_protocol(), 2.0);
        }
        if c == "DPN" {
            let shown = if self.active { 6 } else { self.protocol };
            let text = format!(
                "{}{}",
                if self.auto_search { "A" } else { "" },
                hex_digit(shown as usize)
            );
            return Response::one(text, 2.0);
        }
        if c == "PC" {
            self.active = false;
            return Response::new(&["OK"], 2.0);
        }
        if c == "RV" {
            // Measured at pin 16, a little below the battery itself.
            let s = simulate(self.now(), self.options.engine_start_seconds);
            return Response::one(format!("{:.1}V", s.battery - 0.05), 3.0);
        }
        if c == "IGN" {
            return if self.supports(V14) {
                Response::new(&["ON"], 2.0)
            } else {
                self.unknown()
            };
        }
        if c == "AMC" {
            return if self.supports(V20) {
                Response::new(&["00"], 2.0)
            } else {
                self.unknown()
            };
        }
        if c == "PPS" {
            return if self.supports(V11) {
                self.programmable_parameters()
            } else {
                self.unknown()
            };
        }
        if c.starts_with("PP") && c.len() >= 6 {
            let b = c.as_bytes();
            if is_hex(b[2] as char) && is_hex(b[3] as char) {
                // Accepted, but the demo has no memory that outlives it anyway.
                let rest = after(c, 4);
                let rb = rest.as_bytes();
                if rest == "ON"
                    || rest == "OFF"
                    || (rest.len() == 4
                        && rest.starts_with("SV")
                        && is_hex(rb[2] as char)
                        && is_hex(rb[3] as char))
                {
                    return self.ok(V11);
                }
                return self.unknown();
            }
        }
        if c == "CS" {
            return Response::new(&["T:00 R:00 "], 2.0);
        }
        if c == "BD" {
            let mut line = hex2(self.last_frame.len() as i32);
            for i in 0..12 {
                if self.spaces {
                    line.push(' ');
                }
                let byte = if i < self.last_frame.len() {
                    self.last_frame[i]
                } else {
                    0
                };
                line += &hex2(byte as i32);
            }
            return Response::one(line, 2.0);
        }
        if c == "RTR" {
            return if self.supports(V13) {
                Response::new(&["NO DATA"], 40.0)
            } else {
                self.unknown()
            };
        }
        if c == "MA" || hex_arg(c, "MR", 2).is_some() || hex_arg(c, "MT", 2).is_some() {
            // The bus behind the gateway stays silent: monitoring shows
            // nothing until any character stops it.
            let mut r = Response::empty();
            r.prompt = false;
            self.monitoring = true;
            return r;
        }
        self.unknown()
    }

    // STN (OBDLink) commands; only the read-only identification is modelled.
    fn stn_command(&self, c: &str) -> Response {
        if c == "STI" {
            return Response::one(self.profile.stn_id.unwrap_or(""), 2.0);
        }
        if c == "STDI" {
            return Response::one(self.profile.stn_device.unwrap_or(""), 2.0);
        }
        Response::new(&["?"], 2.0)
    }

    fn execute(&mut self, command: &str) -> Response {
        if command.starts_with("AT") {
            return self.at_command(after(command, 2));
        }
        if self.profile.stn_id.is_some() && command.starts_with("ST") {
            return self.stn_command(command);
        }
        if all_hex(command) {
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
