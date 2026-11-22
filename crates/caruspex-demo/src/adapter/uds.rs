//! The control units that only answer UDS (ISO 14229), and the UDS side of
//! the engine.

use super::*;

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
/// "response = request + 8" convention, so a generic sweep finds them too,
/// except the one on the single wire CAN, which follows GM's 24x/64x scheme.
pub(super) struct UdsUnit {
    request: i32,
    pub(super) response: i32,
    pub(super) obd: bool, // also answers the OBD services (engine, gearbox)
    bus: Bus,             // where it is wired
    dids: Vec<UdsDid>,
    faults: Vec<UdsFault>, // fault memory; the engine computes its own
    slow_fault_read: bool, // answers 19 02 with 7F 19 78 first
}

/// The CAN buses of the OBD socket. An ELM327 only reaches the high speed
/// one; an OBDLink with the transceivers switches with STP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bus {
    High,       // pins 6 and 14
    Medium,     // pins 3 and 11, 125 kbit
    SingleWire, // pin 1, 33.3 kbit
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

pub(super) fn make_uds_units() -> Vec<UdsUnit> {
    // Fictional part numbers in the usual "three groups and an index" style.
    vec![
        UdsUnit {
            request: 0x7E0,
            response: 0x7E8,
            obd: true,
            bus: Bus::High,
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
            bus: Bus::High,
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
            bus: Bus::High,
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
            bus: Bus::High,
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
            bus: Bus::High,
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
            bus: Bus::High,
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
        // Body control on the medium speed CAN: the driver's door switch
        // reported an open circuit once.
        UdsUnit {
            request: 0x726,
            response: 0x72E,
            obd: false,
            bus: Bus::Medium,
            dids: vec![
                did(0xF187, "CX7 937 090 B"),
                did(0xF189, "0214"),
                did(0xF191, "H09"),
                did(0xF197, "BODY CTRL MS"),
                did(0xF18C, "BC21A0077410"),
                did(0xF190, VIN),
            ],
            faults: vec![fault("B1342", 0x13, UDS_STORED)],
            slow_fault_read: false,
        },
        // Body control on the single wire CAN, nothing stored.
        UdsUnit {
            request: 0x241,
            response: 0x641,
            obd: false,
            bus: Bus::SingleWire,
            dids: vec![
                did(0xF187, "CX7 937 091 A"),
                did(0xF189, "0108"),
                did(0xF191, "H02"),
                did(0xF197, "BODY CTRL SW"),
                did(0xF18C, "BS21A0012093"),
            ],
            faults: vec![],
            slow_fault_read: false,
        },
    ]
}

pub(super) fn is_uds_service(sid: i32) -> bool {
    sid == 0x10 || sid == 0x14 || sid == 0x19 || sid == 0x22 || sid == 0x3E
}

/// The engine leaves the extended session by itself this long after the
/// last request (S3 server timer, 5 s in ISO 14229).
const S3_SECONDS: f64 = 5.0;

impl Impl {
    pub(super) fn uds_unit(&self, request: i32) -> Option<usize> {
        if request == 0x7E1 && !self.options.transmission_ecu {
            return None;
        }
        self.uds_units
            .iter()
            .position(|u| u.request == request && u.bus == self.bus)
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
    //
    // The instrument cluster and the ABS keep the odometer too, so the used
    // car check has something to compare PID A6 with. Made up for the demo
    // as well, the same identifier on both units:
    //   1011  cluster 720: total distance in whole km, 32 bit
    //   1011  ABS 760: total distance in steps of 10 km, 16 bit (it only
    //         stores every ten kilometres, so it lags a little behind)
    fn odometer_did(&self, unit_request: i32, id: i32, t: f64) -> Option<Bytes> {
        if id != 0x1011 {
            return None;
        }
        let km = simulate(t, self.options.engine_start_seconds).odometer;
        let mut out: Bytes = vec![0x62, 0x10, 0x11];
        match unit_request {
            0x720 => out.extend_from_slice(&(km.floor() as u32).to_be_bytes()),
            0x760 => out.extend_from_slice(&((km / 10.0).floor() as u16).to_be_bytes()),
            _ => return None,
        }
        Some(out)
    }

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

    // Values the engine keeps only in the extended session, in the layout
    // OBDb gives the Volkswagen group diesels (made up for the demo):
    //   114E  soot mass measured, hundredths of a gram, signed 16 bit
    //   1156  distance since the last regeneration, metres, 32 bit
    fn engine_session_did(&self, id: i32, t: f64) -> Option<Bytes> {
        let s = simulate(t, self.options.engine_start_seconds);
        let mut out: Bytes = vec![0x62, (id >> 8) as u8, (id & 0xFF) as u8];
        match id {
            0x114E => {
                let grams = 9.0 + (s.run_time / 600.0).min(6.0);
                out.extend_from_slice(&((grams * 100.0).round() as i16).to_be_bytes());
            }
            0x1156 => {
                let metres = ((s.odometer % 412.0) * 1000.0).round() as u32;
                out.extend_from_slice(&metres.to_be_bytes());
            }
            _ => return None,
        }
        Some(out)
    }

    // Answers of one unit to a physical request: none, one, or two when it
    // first reports "response pending" (7F xx 78).
    pub(super) fn uds_service(&mut self, unit_index: usize, request: &[u8], t: f64) -> Vec<Bytes> {
        let sid = request[0];
        let negative = |nrc: u8| -> Bytes { vec![0x7F, sid, nrc] };
        let unit_request = self.uds_units[unit_index].request;
        if unit_request == 0x7E0 {
            // Without a request for longer than S3 the engine is back in
            // the default session; every request starts the timer again.
            if self.engine_session != 0x01 && t - self.engine_last_request > S3_SECONDS {
                self.engine_session = 0x01;
            }
            self.engine_last_request = t;
        }
        match sid {
            0x10 => {
                // diagnostic session control: the engine knows the default
                // and the extended session, the other units none
                if unit_request != 0x7E0 {
                    return vec![negative(0x11)];
                }
                if request.len() != 2 {
                    return vec![negative(0x13)];
                }
                match request[1] {
                    0x01 => self.engine_session = 0x01,
                    0x03 if self.options.refuse_extended_session => return vec![negative(0x22)],
                    0x03 => self.engine_session = 0x03,
                    _ => return vec![negative(0x12)],
                }
                // 50 xx with P2 = 50 ms and P2* = 5 s
                vec![vec![0x50, request[1], 0x00, 0x32, 0x01, 0xF4]]
            }
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
                    && let Some(value) = self.engine_session_did(id, t)
                {
                    // In the default session the engine does not know them.
                    if self.engine_session != 0x03 {
                        return vec![negative(0x31)];
                    }
                    return vec![value];
                }
                if unit_request == 0x7E0
                    && let Some(value) = self.engine_did(id, t)
                {
                    return vec![value];
                }
                if let Some(value) = self.odometer_did(unit_request, id, t) {
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
}
