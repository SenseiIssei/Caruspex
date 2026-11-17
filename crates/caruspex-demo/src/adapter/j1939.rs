//! Protocol A (SAE J1939, 250 kbit/s) and the J1939 form of protocol B
//! (`ATPB 42 01`: 29 bit, J1939 format, 500 kbit/s) of the demo adapter,
//! with the demo truck on the bus.
//!
//! A data line is one frame with the id from `ATCP` and `ATSH`; afterwards
//! the adapter prints what it hears, as while monitoring, until a character
//! stops it. `ATMA` prints the broadcasts. With headers on, the id is
//! shown as four bytes (or eight digits without spaces), or with the J1939
//! header formatting (`ATJHF1`, the default) as priority, PGN with data
//! page and source.

use super::*;
use crate::truck::TimedFrame;

/// How much bus traffic one monitor start produces, in milliseconds.
const SPAN_MS: u64 = 5000;

impl Impl {
    pub(crate) fn is_truck(&self) -> bool {
        self.truck.is_some()
    }

    fn j1939_protocol(&self) -> bool {
        self.protocol == 10
            || (self.protocol == 11 && self.pb.0 & 0x80 == 0 && self.pb.0 & 0x07 == 0x02)
    }

    /// The bit rate the adapter listens at in kbit/s.
    fn j1939_kbit(&self) -> u32 {
        match self.protocol {
            10 if self.stn_rate > 0 => self.stn_rate / 1000,
            10 => 250,
            _ if self.pb.1 > 0 => 500 / self.pb.1 as u32,
            _ => 0,
        }
    }

    fn truck_ms(&self) -> u64 {
        (self.now().max(0.0) * 1000.0) as u64
    }

    /// The AT commands that matter for J1939. `None`: not one of them (or
    /// one the usual handling also has to see).
    pub(super) fn j1939_at(&mut self, c: &str) -> Option<Response> {
        match c {
            "JHF0" | "JHF1" => {
                return Some(if self.supports(V14B) {
                    self.jhf = c == "JHF1";
                    Response::new(&["OK"], 2.0)
                } else {
                    self.unknown()
                });
            }
            "JE" | "JS" => {
                return Some(if self.supports(V13) {
                    self.js = c == "JS";
                    Response::new(&["OK"], 2.0)
                } else {
                    self.unknown()
                });
            }
            "MA" if self.is_truck() && self.j1939_protocol() => {
                let start = self.truck_ms();
                let frames = if self.j1939_kbit() == 250 {
                    self.truck
                        .as_ref()
                        .map(|t| t.broadcasts(start, start + SPAN_MS))
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                self.print_frames(frames, start);
                let mut r = Response::empty();
                r.prompt = false;
                self.monitoring = true;
                return Some(r);
            }
            _ => {}
        }
        if let Some(value) = hex_arg(c, "CP", 2) {
            self.cp = value & 0x1F;
            return Some(Response::new(&["OK"], 2.0));
        }
        if let Some(value) = hex_arg(c, "SH", 6) {
            self.header24 = value;
            return None; // the 11 bit handling keeps its part
        }
        if c.starts_with("PB") && c.len() == 6 && all_hex(after(c, 2)) {
            if !self.supports(V14) {
                return Some(self.unknown());
            }
            let options = hex_value_pair(&c[2..4]);
            let divisor = hex_value_pair(&c[4..6]);
            self.pb = (options, divisor);
            return Some(Response::new(&["OK"], 2.0));
        }
        None
    }

    /// STPBR: the bit rate of the current protocol, on STN chips.
    pub(super) fn stn_rate_command(&mut self, c: &str) -> Option<Response> {
        let rate = c.strip_prefix("STPBR")?;
        let value: u32 = rate.parse().ok()?;
        if !(10_000..=1_000_000).contains(&value) {
            return Some(Response::new(&["?"], 2.0));
        }
        self.stn_rate = value;
        Some(Response::new(&["OK"], 2.0))
    }

    /// A data line in a J1939 protocol: one frame on the bus, then the
    /// adapter listens.
    pub(super) fn j1939_line(&mut self, hex: &str) -> Option<Response> {
        if !self.j1939_protocol() {
            return None;
        }
        if hex.len() % 2 == 1 {
            return Some(Response::new(&["?"], 2.0));
        }
        let mut data: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| hex_value_pair(&hex[i..i + 2]) as u8)
            .collect();
        if data.is_empty() || data.len() > 8 {
            return Some(Response::new(&["?"], 2.0));
        }
        // ATJE: a three byte request is typed with the PGN the usual way
        // round and goes out least significant byte first.
        if self.auto_format && !self.js && data.len() == 3 {
            data.reverse();
        }
        if self.truck.is_none() || self.j1939_kbit() != 250 {
            // No truck, or the wrong bit rate: nobody acknowledges the frame.
            return Some(Response::new(&["CAN ERROR"], 60.0));
        }
        let id = ((self.cp as u32 & 0x1F) << 24) | (self.header24 as u32 & 0x00FF_FFFF);
        self.last_frame = data.clone();
        let start = self.truck_ms();
        let truck = self.truck.as_mut()?;
        let mut frames = truck.on_frame(id, &data, start);
        frames.extend(truck.broadcasts(start, start + SPAN_MS));
        frames.sort_by_key(|f| f.at_ms);
        self.print_frames(frames, start);
        let mut r = Response::empty();
        r.prompt = false;
        self.monitoring = true;
        Some(r)
    }

    fn print_frames(&mut self, frames: Vec<TimedFrame>, start: u64) {
        let end = self.line_end();
        for frame in frames {
            let line = self.frame_line(frame.id, &frame.data) + end;
            let delay = frame.at_ms.saturating_sub(start) as f64;
            self.emit(line.into_bytes(), delay);
        }
    }

    fn frame_line(&self, id: u32, data: &[u8]) -> String {
        let gap = if self.spaces { " " } else { "" };
        let mut head = String::new();
        if self.headers {
            if self.jhf && self.j1939_protocol() {
                let priority = (id >> 26) & 0x7;
                let pgn = (id >> 8) & 0x3_FFFF;
                head = format!("{priority:X}{gap}{pgn:05X}{gap}{:02X}{gap}", id & 0xFF);
            } else if self.spaces {
                for shift in [24, 16, 8, 0] {
                    head += &format!("{:02X} ", (id >> shift) & 0xFF);
                }
            } else {
                head = format!("{id:08X}");
            }
        }
        head + self.byte_text(data).trim_end()
    }
}

fn hex_value_pair(text: &str) -> i32 {
    text.chars().fold(0, |acc, c| acc * 16 + hex_value(c))
}
