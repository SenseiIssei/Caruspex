//! The engine and the gearbox control unit over OBD-II: fault state over
//! time, PIDs, freeze frames, mode 06 and the answers to every mode.

use super::*;

impl Impl {
    fn run_since_clear(&self, t: f64) -> f64 {
        if !self.cleared {
            return NEVER;
        }
        (engine_run_time(t, self.options.engine_start_seconds) - self.clear_run).max(0.0)
    }

    pub(super) fn stored_codes(&self, t: f64) -> Vec<&'static str> {
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

    pub(super) fn pending_codes(&self, t: f64) -> Vec<&'static str> {
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

    pub(super) fn clear_engine(&mut self, t: f64) {
        let start = self.options.engine_start_seconds;
        self.cleared = true;
        self.clear_run = engine_run_time(t, start);
        self.clear_km = simulate(t, start).distance_km;
    }

    pub(super) fn engine(&mut self, request: &[u8], physical: bool, t: f64) -> Option<Bytes> {
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

    pub(super) fn gearbox(&self, request: &[u8], physical: bool, t: f64) -> Option<Bytes> {
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
}
