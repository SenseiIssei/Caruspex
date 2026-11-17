//! The demo truck: four control units on an SAE J1939 bus at 250 kbit/s.
//!
//! - engine (address 00): speed, torque, temperatures, pressures, fuel,
//!   battery, boost, AdBlue; two active faults (intake manifold temperature
//!   sensor shorted to plus, coolant level slightly low), so its DM1 is ten
//!   bytes long and goes out as a broadcast announcement (BAM) with two
//!   data packets; one previously active fault (AdBlue level);
//! - transmission (03): gears and oil temperature;
//! - brakes (0B): brake pedal, front axle speed, air supply; one
//!   previously active fault (front axle speed erratic);
//! - instrument cluster (17): vehicle speed, fuel level, distances,
//!   ambient air.
//!
//! Everything is a function of the seconds since ignition on, like the
//! demo car: the engine starts after two seconds and the truck then drives
//! a two minute cycle (stand, pull away, cruise at 80 km/h, brake, stand).
//! The broadcast rates are lower than on a real truck (EEC1 every 100 ms
//! instead of 10 to 20 ms), so an ELM327 at 38400 baud keeps up.
//!
//! The simulation works on whole frames and knows nothing about adapters:
//! [`TruckSim::broadcasts`] gives the frames of a time window, and
//! [`TruckSim::on_frame`] the answers to a frame of the tester (requests,
//! CTS of the transport protocol, address claim, clearing with DM11 and
//! DM3). The ELM327 of the demo prints them in protocol A; a raw CAN test
//! puts them on a simulated bus.

/// One frame at a moment of the simulation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedFrame {
    /// Seconds since ignition on, in milliseconds.
    pub at_ms: u64,
    pub id: u32,
    pub data: Vec<u8>,
}

/// The VIN of the demo truck: fictional maker code, model year N (2022),
/// check digit after the North American rule.
pub const TRUCK_VIN: &str = "YCXTR42H4NC051739";

pub const ENGINE: u8 = 0x00;
pub const TRANSMISSION: u8 = 0x03;
pub const BRAKES: u8 = 0x0B;
pub const CLUSTER: u8 = 0x17;
const UNITS: [u8; 4] = [ENGINE, TRANSMISSION, BRAKES, CLUSTER];

const REQUEST: u32 = 0xEA00;
const ACK: u32 = 0xE800;
const TP_CM: u32 = 0xEC00;
const TP_DT: u32 = 0xEB00;
const CLAIM: u32 = 0xEE00;
const DM1: u32 = 0xFECA;
const DM2: u32 = 0xFECB;
const DM3: u32 = 0xFECC;
const DM11: u32 = 0xFED3;
const VIN_PGN: u32 = 0xFEEC;
const COMPONENT: u32 = 0xFEEB;
const SOFTWARE: u32 = 0xFEDA;
const HOURS: u32 = 0xFEE5;
const FUEL_USED: u32 = 0xFEE9;

const EEC1: u32 = 61444;
const EEC2: u32 = 61443;
const ETC2: u32 = 61445;
const EBC1: u32 = 61441;
const ET1: u32 = 65262;
const EFL_P1: u32 = 65263;
const DASH: u32 = 65276;
const CCVS1: u32 = 65265;
const VEP1: u32 = 65271;
const DISTANCE: u32 = 65248;
const DISTANCE_HR: u32 = 65217;
const FUEL_ECONOMY: u32 = 65266;
const AMBIENT: u32 = 65269;
const INLET: u32 = 65270;
const TRANS_FLUIDS: u32 = 65272;
const AIR: u32 = 65198;
const DEF_TANK: u32 = 65110;
const WHEELS: u32 = 65215;

/// The NAME of the other diagnostic tool [`TruckSim::with_tool_on_f9`]
/// puts on the bus: lower than any tool that can take any address.
pub const OTHER_TOOL_NAME: u64 = 0x0081_0000_0000_1234;

fn id(priority: u32, pgn: u32, destination: u8, source: u8) -> u32 {
    let pf = (pgn >> 8) & 0xFF;
    let ps = if pf < 240 {
        u32::from(destination)
    } else {
        pgn & 0xFF
    };
    (priority << 26) | (((pgn >> 8) & 0x3FF) << 16) | (ps << 8) | u32::from(source)
}

fn unit_name(address: u8) -> u64 {
    let function: u64 = match address {
        ENGINE => 0,
        TRANSMISSION => 3,
        BRAKES => 9,
        _ => 23,
    };
    (function << 40) | (0x1F0 << 21) | (0x4_2000 + u64::from(address))
}

/// One fault: SPN, FMI, occurrence count.
type Fault = (u32, u8, u8);

const ENGINE_ACTIVE: [Fault; 2] = [(105, 3, 7), (111, 17, 2)];

fn dm(lamps: [u8; 2], faults: &[Fault]) -> Vec<u8> {
    let mut out = lamps.to_vec();
    if faults.is_empty() {
        out.extend_from_slice(&[0, 0, 0, 0]);
    }
    for &(spn, fmi, count) in faults {
        out.extend_from_slice(&[
            spn as u8,
            (spn >> 8) as u8,
            (((spn >> 16) as u8) << 5) | (fmi & 0x1F),
            count & 0x7F,
        ]);
    }
    out.resize(out.len().max(8), 0xFF);
    out
}

/// Where a value sits and how it is scaled, for building frames.
fn put(data: &mut [u8], byte: usize, bytes: usize, value: f64, scale: f64, offset: f64) {
    let raw = ((value - offset) / scale).round().max(0.0) as u64;
    for i in 0..bytes {
        data[byte + i] = (raw >> (8 * i)) as u8;
    }
}

/// A transfer waiting for the tester's CTS.
struct Transfer {
    from: u8,
    to: u8,
    pgn: u32,
    payload: Vec<u8>,
}

/// The state of the truck at one moment.
struct State {
    running: bool,
    speed: f64,
    rpm: f64,
    braking: bool,
    gear: f64,
    coolant: f64,
}

pub struct TruckSim {
    engine_start: f64,
    active: bool,
    previous_engine: Vec<Fault>,
    previous_brakes: Vec<Fault>,
    transfers: Vec<Transfer>,
    tool_on_f9: bool,
}

impl Default for TruckSim {
    fn default() -> Self {
        Self::new()
    }
}

impl TruckSim {
    pub fn new() -> Self {
        TruckSim {
            engine_start: 2.0,
            active: true,
            previous_engine: vec![(1761, 18, 2)],
            previous_brakes: vec![(904, 2, 4)],
            transfers: Vec::new(),
            tool_on_f9: false,
        }
    }

    /// Another diagnostic tool already holds address F9 and defends it.
    pub fn with_tool_on_f9(mut self) -> Self {
        self.tool_on_f9 = true;
        self
    }

    /// True while the engine still reports its two active faults.
    pub fn has_active_faults(&self) -> bool {
        self.active
    }

    fn state(&self, t: f64) -> State {
        let running = t >= self.engine_start;
        let run = (t - self.engine_start).max(0.0);
        // Two minute cycle: stand 20 s, pull away 20 s, cruise 50 s, brake
        // 15 s, stand 15 s.
        let c = run % 120.0;
        let (speed, braking) = if !running || c < 20.0 {
            (0.0, false)
        } else if c < 40.0 {
            ((c - 20.0) * 4.0, false)
        } else if c < 90.0 {
            (80.0, false)
        } else if c < 105.0 {
            (80.0 - (c - 90.0) * 80.0 / 15.0, true)
        } else {
            (0.0, false)
        };
        let gear = if speed < 1.0 {
            0.0
        } else {
            (1.0 + speed / 7.0).floor().min(12.0)
        };
        let rpm = if !running {
            0.0
        } else if speed < 1.0 {
            600.0
        } else {
            900.0 + (speed % 7.0) * 60.0 + gear * 20.0
        };
        State {
            running,
            speed,
            rpm,
            braking,
            gear,
            coolant: if running {
                (20.0 + run * 0.6).min(85.0)
            } else {
                20.0
            },
        }
    }

    /// The current data of a broadcast group of a unit.
    fn group(&self, unit: u8, pgn: u32, t: f64) -> Option<Vec<u8>> {
        let s = self.state(t);
        let mut d = vec![0xFF; 8];
        match (unit, pgn) {
            (ENGINE, EEC1) => {
                let torque = if !s.running {
                    0.0
                } else if s.speed < 1.0 {
                    8.0
                } else if s.braking {
                    0.0
                } else {
                    45.0
                };
                d[0] = 0xF0;
                put(&mut d, 1, 1, torque + 2.0, 1.0, -125.0);
                put(&mut d, 2, 1, torque, 1.0, -125.0);
                put(&mut d, 3, 2, s.rpm, 0.125, 0.0);
                d[5] = ENGINE;
            }
            (ENGINE, EEC2) => {
                let pedal = if s.speed > 1.0 && !s.braking {
                    38.0
                } else {
                    0.0
                };
                put(&mut d, 1, 1, pedal, 0.4, 0.0);
                put(
                    &mut d,
                    2,
                    1,
                    if s.running { pedal + 12.0 } else { 0.0 },
                    1.0,
                    0.0,
                );
            }
            (ENGINE, ET1) => {
                put(&mut d, 0, 1, s.coolant, 1.0, -40.0);
                put(&mut d, 1, 1, 28.0, 1.0, -40.0);
                put(&mut d, 2, 2, s.coolant - 3.0, 0.03125, -273.0);
            }
            (ENGINE, EFL_P1) => {
                put(&mut d, 0, 1, if s.running { 520.0 } else { 0.0 }, 4.0, 0.0);
                put(&mut d, 2, 1, 86.0, 0.4, 0.0);
                put(
                    &mut d,
                    3,
                    1,
                    if s.running { 150.0 + s.rpm * 0.15 } else { 0.0 },
                    4.0,
                    0.0,
                );
                put(&mut d, 4, 2, 0.4, 0.0078125, -250.0);
                put(&mut d, 6, 1, if s.running { 110.0 } else { 0.0 }, 2.0, 0.0);
                put(
                    &mut d,
                    7,
                    1,
                    if self.active { 72.0 } else { 88.0 },
                    0.4,
                    0.0,
                );
            }
            (ENGINE, FUEL_ECONOMY) => {
                let rate = if s.running { 3.0 + s.rpm / 100.0 } else { 0.0 };
                put(&mut d, 0, 2, rate, 0.05, 0.0);
                put(
                    &mut d,
                    2,
                    2,
                    if s.speed > 1.0 {
                        s.speed / rate.max(0.1)
                    } else {
                        0.0
                    },
                    1.0 / 512.0,
                    0.0,
                );
                put(&mut d, 6, 1, 100.0, 0.4, 0.0);
            }
            (ENGINE, INLET) => {
                put(
                    &mut d,
                    1,
                    1,
                    if s.running {
                        (s.rpm - 600.0) / 6.0
                    } else {
                        0.0
                    },
                    2.0,
                    0.0,
                );
                // The sensor of the active fault reports "error".
                if self.active {
                    d[2] = 0xFE;
                } else {
                    put(&mut d, 2, 1, 35.0, 1.0, -40.0);
                }
                put(&mut d, 3, 1, 100.0, 2.0, 0.0);
                put(
                    &mut d,
                    5,
                    2,
                    if s.running { 180.0 + s.rpm / 5.0 } else { 15.0 },
                    0.03125,
                    -273.0,
                );
            }
            (ENGINE, VEP1) => {
                let volts = if s.running { 27.8 } else { 24.8 };
                put(
                    &mut d,
                    0,
                    1,
                    if s.running { 4.0 } else { -2.0 },
                    1.0,
                    -125.0,
                );
                put(&mut d, 1, 1, if s.running { 32.0 } else { 0.0 }, 1.0, 0.0);
                put(&mut d, 2, 2, volts, 0.05, 0.0);
                put(&mut d, 4, 2, volts, 0.05, 0.0);
                put(&mut d, 6, 2, volts - 0.2, 0.05, 0.0);
            }
            (ENGINE, DEF_TANK) => {
                put(&mut d, 0, 1, 48.0, 0.4, 0.0);
                put(&mut d, 1, 1, 14.0, 1.0, -40.0);
            }
            (ENGINE, HOURS) => {
                put(&mut d, 0, 4, 12_345.6 + t / 3600.0, 0.05, 0.0);
                put(&mut d, 4, 4, 98_765_000.0 + t * s.rpm / 60.0, 1000.0, 0.0);
            }
            (ENGINE, FUEL_USED) => {
                put(&mut d, 0, 4, 412.5, 0.5, 0.0);
                put(&mut d, 4, 4, 187_654.5, 0.5, 0.0);
            }
            (TRANSMISSION, ETC2) => {
                put(&mut d, 0, 1, s.gear, 1.0, -125.0);
                put(&mut d, 1, 2, 1.0, 0.001, 0.0);
                put(&mut d, 3, 1, s.gear, 1.0, -125.0);
            }
            (TRANSMISSION, TRANS_FLUIDS) => {
                put(&mut d, 4, 2, (s.coolant - 10.0).max(20.0), 0.03125, -273.0);
            }
            (BRAKES, EBC1) => {
                put(&mut d, 1, 1, if s.braking { 30.0 } else { 0.0 }, 0.4, 0.0);
            }
            (BRAKES, WHEELS) => put(&mut d, 0, 2, s.speed, 1.0 / 256.0, 0.0),
            (BRAKES, AIR) => put(&mut d, 0, 1, 848.0, 8.0, 0.0),
            (CLUSTER, CCVS1) => {
                d[0] = if s.speed < 1.0 { 0xF7 } else { 0xF3 };
                put(&mut d, 1, 2, s.speed, 1.0 / 256.0, 0.0);
            }
            (CLUSTER, DASH) => put(&mut d, 1, 1, 62.0, 0.4, 0.0),
            (CLUSTER, DISTANCE) => {
                put(&mut d, 0, 4, 214.25 + t * 0.012, 0.125, 0.0);
                put(&mut d, 4, 4, 512_345.5 + t * 0.012, 0.125, 0.0);
            }
            (CLUSTER, DISTANCE_HR) => {
                put(&mut d, 0, 4, 512_345.5 + t * 0.012, 0.005, 0.0);
                put(&mut d, 4, 4, 214.25 + t * 0.012, 0.005, 0.0);
            }
            (CLUSTER, AMBIENT) => {
                put(&mut d, 0, 1, 100.5, 0.5, 0.0);
                put(&mut d, 1, 2, 21.0, 0.03125, -273.0);
                put(&mut d, 3, 2, 12.5, 0.03125, -273.0);
                put(&mut d, 5, 1, 18.0, 1.0, -40.0);
            }
            _ => return None,
        }
        Some(d)
    }

    fn dm1(&self, unit: u8) -> Vec<u8> {
        if unit == ENGINE && self.active {
            // MIL and amber warning lamp on, steady.
            return dm([0x44, 0xFF], &ENGINE_ACTIVE);
        }
        dm([0x00, 0xFF], &[])
    }

    fn dm2(&self, unit: u8) -> Vec<u8> {
        match unit {
            ENGINE => dm([0x00, 0xFF], &self.previous_engine),
            BRAKES => dm([0x00, 0xFF], &self.previous_brakes),
            _ => dm([0x00, 0xFF], &[]),
        }
    }

    /// The answer of `unit` to a request for `pgn`, if it has one.
    fn answer(&self, unit: u8, pgn: u32, t: f64) -> Option<Vec<u8>> {
        match pgn {
            DM1 => Some(self.dm1(unit)),
            DM2 => Some(self.dm2(unit)),
            VIN_PGN if unit == ENGINE || unit == CLUSTER => {
                Some(format!("{TRUCK_VIN}*").into_bytes())
            }
            COMPONENT => {
                let (model, serial, name) = match unit {
                    ENGINE => ("CX-D13", "E7712093", "ENGINE"),
                    TRANSMISSION => ("CX-AMT12", "T5520418", "GEARBOX"),
                    BRAKES => ("CX-EBS5", "B3301977", "BRAKES"),
                    _ => ("CX-IC2", "C0091534", "CLUSTER"),
                };
                Some(format!("CARUSPEX*{model}*{serial}*{name}*").into_bytes())
            }
            SOFTWARE => {
                let text = match unit {
                    ENGINE => "ECM 4.2.1*BOOT 1.07*",
                    TRANSMISSION => "TCU 2.9.0*",
                    BRAKES => "EBS 5.3.2*",
                    _ => "IC 1.4.8*",
                };
                let fields = text.matches('*').count() as u8;
                let mut out = vec![fields];
                out.extend_from_slice(text.as_bytes());
                Some(out)
            }
            CLAIM => Some(unit_name(unit).to_le_bytes().to_vec()),
            _ => self.group(unit, pgn, t),
        }
    }

    /// What each unit sends by itself: (unit, PGN, period in ms, priority).
    fn schedule() -> &'static [(u8, u32, u64, u32)] {
        &[
            (ENGINE, EEC1, 100, 3),
            (ENGINE, EEC2, 200, 3),
            (ENGINE, ET1, 1000, 6),
            (ENGINE, EFL_P1, 500, 6),
            (ENGINE, FUEL_ECONOMY, 500, 6),
            (ENGINE, INLET, 500, 6),
            (ENGINE, VEP1, 1000, 6),
            (ENGINE, DEF_TANK, 1000, 6),
            (ENGINE, DM1, 1000, 6),
            (TRANSMISSION, ETC2, 200, 6),
            (TRANSMISSION, TRANS_FLUIDS, 1000, 6),
            (TRANSMISSION, DM1, 1000, 6),
            (BRAKES, EBC1, 200, 6),
            (BRAKES, WHEELS, 500, 6),
            (BRAKES, AIR, 1000, 6),
            (BRAKES, DM1, 1000, 6),
            (CLUSTER, CCVS1, 100, 6),
            (CLUSTER, DASH, 1000, 6),
            (CLUSTER, DISTANCE, 1000, 6),
            (CLUSTER, DISTANCE_HR, 1000, 6),
            (CLUSTER, AMBIENT, 1000, 6),
            (CLUSTER, DM1, 1000, 6),
        ]
    }

    /// Every frame the units broadcast in the window `(from_ms, to_ms]`
    /// (milliseconds since ignition on), in time order.
    pub fn broadcasts(&self, from_ms: u64, to_ms: u64) -> Vec<TimedFrame> {
        let mut out = Vec::new();
        // The address claims right after power on.
        for (i, unit) in UNITS.iter().enumerate() {
            let at = 200 + 7 * i as u64;
            if at > from_ms && at <= to_ms {
                out.push(TimedFrame {
                    at_ms: at,
                    id: id(6, CLAIM, 0xFF, *unit),
                    data: unit_name(*unit).to_le_bytes().to_vec(),
                });
            }
        }
        if self.tool_on_f9 && from_ms < 230 && to_ms >= 230 {
            out.push(TimedFrame {
                at_ms: 230,
                id: id(6, CLAIM, 0xFF, 0xF9),
                data: OTHER_TOOL_NAME.to_le_bytes().to_vec(),
            });
        }
        for (index, &(unit, pgn, period, priority)) in Self::schedule().iter().enumerate() {
            // Spread the senders a little, as real units are never in step.
            let phase = 3 * index as u64 + 11 * u64::from(unit);
            let first = if from_ms < phase {
                0
            } else {
                (from_ms - phase) / period + 1
            };
            let mut k = first;
            loop {
                let at = phase + k * period;
                if at > to_ms {
                    break;
                }
                k += 1;
                if at <= from_ms {
                    continue;
                }
                let t = at as f64 / 1000.0;
                let Some(data) = (if pgn == DM1 {
                    Some(self.dm1(unit))
                } else {
                    self.group(unit, pgn, t)
                }) else {
                    continue;
                };
                if data.len() > 8 {
                    out.extend(bam(unit, pgn, &data, at));
                } else {
                    out.push(TimedFrame {
                        at_ms: at,
                        id: id(priority, pgn, 0xFF, unit),
                        data,
                    });
                }
            }
        }
        out.sort_by_key(|f| f.at_ms);
        out
    }

    /// The answers to one frame of the tester, sent at `at_ms`.
    pub fn on_frame(&mut self, frame_id: u32, data: &[u8], at_ms: u64) -> Vec<TimedFrame> {
        let pf = (frame_id >> 16) & 0xFF;
        let destination = ((frame_id >> 8) & 0xFF) as u8;
        let source = (frame_id & 0xFF) as u8;
        let pgn = if pf < 240 {
            (frame_id >> 8) & 0x3_FF00
        } else {
            (frame_id >> 8) & 0x3_FFFF
        };
        let t = at_ms as f64 / 1000.0;
        match pgn {
            REQUEST if data.len() >= 3 => {
                let wanted =
                    u32::from(data[0]) | (u32::from(data[1]) << 8) | (u32::from(data[2]) << 16);
                self.request(wanted, destination, source, at_ms, t)
            }
            TP_CM if data.len() >= 8 => self.connection(data, destination, source, at_ms),
            CLAIM if self.tool_on_f9 && source == 0xF9 => {
                // The other tool has the lower NAME and keeps its address.
                let ours = u64::from_le_bytes(data[..8].try_into().unwrap_or([0xFF; 8]));
                if ours > OTHER_TOOL_NAME {
                    vec![TimedFrame {
                        at_ms: at_ms + 5,
                        id: id(6, CLAIM, 0xFF, 0xF9),
                        data: OTHER_TOOL_NAME.to_le_bytes().to_vec(),
                    }]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    fn request(
        &mut self,
        wanted: u32,
        destination: u8,
        requester: u8,
        at_ms: u64,
        t: f64,
    ) -> Vec<TimedFrame> {
        let global = destination == 0xFF;
        let mut out = Vec::new();
        if wanted == CLAIM && global && self.tool_on_f9 {
            out.push(TimedFrame {
                at_ms: at_ms + 4,
                id: id(6, CLAIM, 0xFF, 0xF9),
                data: OTHER_TOOL_NAME.to_le_bytes().to_vec(),
            });
        }
        let responders: Vec<u8> = UNITS
            .iter()
            .copied()
            .filter(|u| global || *u == destination)
            .collect();
        if wanted == DM11 || wanted == DM3 {
            if wanted == DM11 && self.active {
                self.previous_engine.extend_from_slice(&ENGINE_ACTIVE);
                self.active = false;
            }
            if wanted == DM3 {
                self.previous_engine.clear();
                self.previous_brakes.clear();
            }
            for (i, unit) in responders.iter().enumerate() {
                out.push(TimedFrame {
                    at_ms: at_ms + 8 + 3 * i as u64,
                    id: id(6, ACK, 0xFF, *unit),
                    data: vec![
                        0x00,
                        0xFF,
                        0xFF,
                        0xFF,
                        requester,
                        wanted as u8,
                        (wanted >> 8) as u8,
                        (wanted >> 16) as u8,
                    ],
                });
            }
            return out;
        }
        for (i, &unit) in responders.iter().enumerate() {
            let at = at_ms + 10 + 4 * i as u64;
            match self.answer(unit, wanted, t) {
                Some(payload) if payload.len() > 8 => {
                    if global {
                        let start = self.bam_slot(unit, at, payload.len().div_ceil(7) as u64);
                        out.extend(bam(unit, wanted, &payload, start));
                    } else {
                        let packets = payload.len().div_ceil(7) as u8;
                        let size = (payload.len() as u16).to_le_bytes();
                        out.push(TimedFrame {
                            at_ms: at,
                            id: id(7, TP_CM, requester, unit),
                            data: vec![
                                16,
                                size[0],
                                size[1],
                                packets,
                                0xFF,
                                wanted as u8,
                                (wanted >> 8) as u8,
                                (wanted >> 16) as u8,
                            ],
                        });
                        self.transfers
                            .retain(|x| !(x.from == unit && x.to == requester));
                        self.transfers.push(Transfer {
                            from: unit,
                            to: requester,
                            pgn: wanted,
                            payload,
                        });
                    }
                }
                Some(mut payload) => {
                    let destination = if wanted == CLAIM { 0xFF } else { requester };
                    payload.resize(8, 0xFF);
                    out.push(TimedFrame {
                        at_ms: at,
                        id: id(6, wanted, destination, unit),
                        data: payload,
                    });
                }
                None if !global => {
                    // Asked directly for something it does not have.
                    out.push(TimedFrame {
                        at_ms: at,
                        id: id(6, ACK, 0xFF, unit),
                        data: vec![
                            0x01,
                            0xFF,
                            0xFF,
                            0xFF,
                            requester,
                            wanted as u8,
                            (wanted >> 8) as u8,
                            (wanted >> 16) as u8,
                        ],
                    });
                }
                None => {}
            }
        }
        out
    }

    /// A unit runs one BAM at a time: an answer in packets waits until the
    /// engine's own DM1 announcement (every second while it has two
    /// faults) is through.
    fn bam_slot(&self, unit: u8, at_ms: u64, packets: u64) -> u64 {
        if unit != ENGINE || !self.active {
            return at_ms;
        }
        let Some(index) = Self::schedule()
            .iter()
            .position(|&(u, pgn, _, _)| u == ENGINE && pgn == DM1)
        else {
            return at_ms;
        };
        let phase = 3 * index as u64;
        let mut start = at_ms;
        for _ in 0..3 {
            let k = start.saturating_sub(phase) / 1000;
            let clash = [k, k + 1].into_iter().find(|&k| {
                let dm1 = phase + k * 1000;
                start <= dm1 + 100 && dm1 <= start + 50 * packets
            });
            match clash {
                Some(k) => start = phase + k * 1000 + 105,
                None => break,
            }
        }
        start
    }

    /// TP.CM from the tester to a unit: CTS, end of message, abort.
    fn connection(&mut self, data: &[u8], unit: u8, tester: u8, at_ms: u64) -> Vec<TimedFrame> {
        let pgn = u32::from(data[5]) | (u32::from(data[6]) << 8) | (u32::from(data[7]) << 16);
        let Some(index) = self
            .transfers
            .iter()
            .position(|x| x.from == unit && x.to == tester && x.pgn == pgn)
        else {
            return Vec::new();
        };
        match data[0] {
            17 => {
                let transfer = &self.transfers[index];
                let packets: Vec<&[u8]> = transfer.payload.chunks(7).collect();
                let count = usize::from(data[1]);
                let next = usize::from(data[2]).max(1);
                (next..next + count)
                    .filter(|n| *n <= packets.len())
                    .enumerate()
                    .map(|(i, n)| {
                        let mut frame = vec![n as u8];
                        frame.extend_from_slice(packets[n - 1]);
                        frame.resize(8, 0xFF);
                        TimedFrame {
                            at_ms: at_ms + 5 + 2 * i as u64,
                            id: id(7, TP_DT, tester, unit),
                            data: frame,
                        }
                    })
                    .collect()
            }
            19 | 255 => {
                self.transfers.remove(index);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }
}

/// A long message to everybody: the announcement, then the packets 50 ms
/// apart.
fn bam(unit: u8, pgn: u32, payload: &[u8], at_ms: u64) -> Vec<TimedFrame> {
    let packets = payload.len().div_ceil(7) as u8;
    let size = (payload.len() as u16).to_le_bytes();
    let mut out = vec![TimedFrame {
        at_ms,
        id: id(7, TP_CM, 0xFF, unit),
        data: vec![
            32,
            size[0],
            size[1],
            packets,
            0xFF,
            pgn as u8,
            (pgn >> 8) as u8,
            (pgn >> 16) as u8,
        ],
    }];
    for (i, chunk) in payload.chunks(7).enumerate() {
        let mut frame = vec![(i + 1) as u8];
        frame.extend_from_slice(chunk);
        frame.resize(8, 0xFF);
        out.push(TimedFrame {
            at_ms: at_ms + 50 * (i as u64 + 1),
            id: id(7, TP_DT, 0xFF, unit),
            data: frame,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_of_broadcasts() {
        let truck = TruckSim::new();
        let frames = truck.broadcasts(600_000, 601_000);
        assert!(frames.len() > 40, "{}", frames.len());
        assert!(frames.windows(2).all(|w| w[0].at_ms <= w[1].at_ms));
        // The engine's DM1 has two faults, so it comes as a BAM.
        assert!(
            frames
                .iter()
                .any(|f| f.id == 0x1CEC_FF00 && f.data[0] == 32)
        );
        assert!(frames.iter().any(|f| f.id == 0x1CEB_FF00 && f.data[0] == 2));
        // EEC1 ten times.
        assert_eq!(frames.iter().filter(|f| f.id == 0x0CF0_0400).count(), 10);
        // Windows next to each other give every frame once.
        let a = truck.broadcasts(600_000, 600_500);
        let b = truck.broadcasts(600_500, 601_000);
        assert_eq!(a.len() + b.len(), frames.len());
    }

    #[test]
    fn vin_directly_comes_with_rts_and_cts() {
        let mut truck = TruckSim::new();
        let answers = truck.on_frame(0x18EA_00F9, &[0xEC, 0xFE, 0x00], 1000);
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].id, 0x1CEC_F900);
        assert_eq!(answers[0].data[0], 16);
        let packets = answers[0].data[3];
        assert_eq!(packets, 3);
        let dt = truck.on_frame(
            0x1CEC_00F9,
            &[17, packets, 1, 0xFF, 0xFF, 0xEC, 0xFE, 0x00],
            1100,
        );
        assert_eq!(dt.len(), 3);
        let text: Vec<u8> = dt.iter().flat_map(|f| f.data[1..].to_vec()).collect();
        assert!(text.starts_with(TRUCK_VIN.as_bytes()));
    }

    #[test]
    fn clearing_moves_the_faults() {
        let mut truck = TruckSim::new();
        assert!(truck.has_active_faults());
        let acks = truck.on_frame(0x18EA_FFF9, &[0xD3, 0xFE, 0x00], 5000);
        assert_eq!(acks.len(), 4);
        assert!(acks.iter().all(|f| f.data[0] == 0 && f.data[4] == 0xF9));
        assert!(!truck.has_active_faults());
        assert_eq!(truck.dm2(ENGINE).len(), 14);
        truck.on_frame(0x18EA_FFF9, &[0xCC, 0xFE, 0x00], 6000);
        assert_eq!(truck.dm2(ENGINE)[2..6], [0, 0, 0, 0]);
    }
}
