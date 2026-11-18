//! SAE J1939 on an ELM327: protocol A (29 bit, 250 kbit/s, the 9-pin
//! socket of type 1) and, through the user protocol B, 500 kbit/s (type 2,
//! J1939-14).
//!
//! The adapter only carries frames here: CAN auto formatting is off
//! (`ATCAF0`), headers are on, and the J1939 transport protocol, requests
//! and address claims are done by `caruspex_protocols::j1939` above the
//! [`FrameLink`]. Two ways of listening:
//!
//! - nothing to send: `ATMA` prints every frame on the bus;
//! - a frame to send: the header goes in with `ATCP` (the top five bits) and
//!   `ATSH` (the lower 24), the data bytes as a line; the adapter sends them
//!   and prints what it receives afterwards. On a busy bus that does not
//!   end by itself, so the listening is stopped like a monitor, with a
//!   carriage return.
//!
//! 500 kbit/s: `ATPB 42 01` sets the user protocol B to 29 bit ids, eight
//! data bytes and the J1939 format at 500 kbit/s, `ATSPB` selects it. An STN
//! chip is first asked to keep protocol A and only change its bit rate
//! (`STPBR 500000`). None of this has been checked against a real adapter
//! on a truck yet; it is built after the command descriptions and tested
//! against the demo truck.

use caruspex_i18n::l;

use super::{
    DEFAULT_SEND_TIMEOUT_MS, Elm327, answered_ok, reply_is_garbled, reset_was_interrupted,
};
use crate::timing::now_ms;
use crate::transport::{BusFrame, FrameLink, Transport};

/// How long connect listens for traffic at each bit rate.
const LISTEN_FOR_TRAFFIC_MS: u64 = 1500;
/// Frames needed to call a bit rate right.
const TRAFFIC_FRAMES: usize = 2;

/// One line the adapter prints for a received frame, as a frame. Three
/// forms are understood: the id as one token of eight digits ("18FECA00 44
/// FF ..."), the id as four bytes ("18 FE CA 00 44 FF ..."), and the J1939
/// header formatting of the ELM327 (priority, PGN with the data page bit,
/// source: "6 0FECA 00 44 FF ..."). Without spaces the first eight digits
/// are the id. Anything else (status lines, prompts) is `None`.
pub fn parse_j1939_line(line: &str) -> Option<BusFrame> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.is_empty()
        || !tokens
            .iter()
            .all(|t| t.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return None;
    }
    let hex = |t: &str| u32::from_str_radix(t, 16).ok();
    let pairs = |rest: &[&str]| -> Option<Vec<u8>> {
        let joined: String = rest.concat();
        if !joined.len().is_multiple_of(2) {
            return None;
        }
        (0..joined.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&joined[i..i + 2], 16).ok())
            .collect()
    };
    let (id, data) = if tokens.len() == 1 && tokens[0].len() >= 8 {
        (hex(&tokens[0][..8])?, pairs(&[&tokens[0][8..]])?)
    } else if tokens[0].len() == 8 {
        (hex(tokens[0])?, pairs(&tokens[1..])?)
    } else if tokens.len() >= 3
        && tokens[0].len() == 1
        && tokens[1].len() == 5
        && tokens[2].len() == 2
    {
        let priority = hex(tokens[0])? & 0x7;
        let pgn = hex(tokens[1])? & 0x3FFFF;
        let source = hex(tokens[2])?;
        ((priority << 26) | (pgn << 8) | source, pairs(&tokens[3..])?)
    } else if tokens.len() >= 4 && tokens[..4].iter().all(|t| t.len() == 2) {
        (hex(&tokens[..4].concat())?, pairs(&tokens[4..])?)
    } else {
        return None;
    };
    if id > 0x1FFF_FFFF || data.len() > 8 {
        return None;
    }
    Some(BusFrame { id, data })
}

fn protocol_text(kbit: u32) -> String {
    format!("SAE J1939 (29 {}, {} kbit/s)", l("Bit", "bit"), kbit)
}

impl Elm327 {
    /// The J1939 bit rate in kbit/s while connected in protocol A or B,
    /// else 0.
    pub fn j1939_kbit(&self) -> u32 {
        self.j1939_kbit
    }

    /// Takes the transport, resets the adapter and switches it to SAE
    /// J1939 at the first rate of `rates_kbit` (250 or 500) on which frames
    /// can be heard. Returns that rate. Nothing is sent to the bus: J1939
    /// is mostly broadcast, so listening tells whether the rate is right.
    pub fn connect_j1939(
        &mut self,
        transport: Box<dyn Transport>,
        rates_kbit: &[u32],
    ) -> Result<u32, String> {
        self.transport = Some(transport);
        self.connected = false;
        self.j1939_kbit = 0;
        self.j1939_header = None;
        self.stn = false;
        self.stn_id.clear();
        self.stn_batch = false;
        self.timing_level = 0;
        self.obd_on_uds = false;
        self.listen_only = false;
        self.trace = Default::default();
        self.drain_greeting();

        let mut banner = self.at("Z", 6000);
        if banner.status == super::ElmStatus::AdapterError {
            return Err(banner.message);
        }
        if reset_was_interrupted(&banner.lines) {
            self.drain_greeting();
            banner = self.at("Z", 6000);
            if banner.status == super::ElmStatus::AdapterError {
                return Err(banner.message);
            }
        }
        let version = banner.lines.join(" ").trim().to_string();
        self.version = if version.is_empty() {
            "?".to_string()
        } else {
            version
        };
        if reply_is_garbled(&self.version) {
            return Err(l(
                "Der Adapter antwortet unleserlich, vermutlich passt die Baudrate nicht.",
                "The adapter answers garbled, probably the baud rate does not match.",
            )
            .to_string());
        }
        self.trace.adapter_answered = true;
        self.trace.adapter_id = self.version.clone();
        // Spaces stay on: they tell the forms of the header apart.
        for cmd in ["E0", "L0", "S1", "H1"] {
            self.at(cmd, DEFAULT_SEND_TIMEOUT_MS);
        }
        self.headers = true;
        self.detect_stn();

        for &kbit in rates_kbit {
            let id = if kbit == 500 { 'B' } else { 'A' };
            self.trace.protocols_tried.push(id);
            self.log(&format!(
                "{}{} kbit/s ...",
                l("  Versuche SAE J1939 mit ", "  Trying SAE J1939 at "),
                kbit
            ));
            if !self.select_j1939_rate(kbit) {
                self.log(l(
                    "  Der Adapter kann diese Bitrate nicht einstellen.",
                    "  The adapter cannot set this bit rate.",
                ));
                continue;
            }
            // Raw frames: the first data byte is not taken for an ISO-TP
            // PCI. The plain id (JHF0) is refused by older chips; the line
            // parser also reads the J1939 header formatting.
            self.at("CAF0", DEFAULT_SEND_TIMEOUT_MS);
            self.at("JHF0", DEFAULT_SEND_TIMEOUT_MS);
            self.at("AR", DEFAULT_SEND_TIMEOUT_MS);
            self.protocol_id = id;
            self.j1939_kbit = kbit;
            self.j1939_header = None;
            self.connected = true;
            let mut frames = 0usize;
            let heard = self.exchange(None, LISTEN_FOR_TRAFFIC_MS, &mut |_| {
                frames += 1;
                frames >= TRAFFIC_FRAMES
            });
            if heard.is_ok() && frames >= TRAFFIC_FRAMES {
                self.protocol_text = protocol_text(kbit);
                self.log(&format!(
                    "{}{}",
                    l("  Verkehr auf dem Bus: ", "  Traffic on the bus: "),
                    self.protocol_text
                ));
                return Ok(kbit);
            }
            self.connected = false;
            self.j1939_kbit = 0;
        }
        self.protocol_id = '0';
        Err(l(
            "Auf dem J1939-Bus ist nichts zu hören.\n  - Zündung an? Lkw-Steuergeräte schweigen bei Zündung aus.\n  - Steckt der Adapter über das Adapterkabel in der 9-poligen Diagnosebuchse?\n  - Schwarze Buchse (Typ 1): 250 kbit/s, grüne Buchse (Typ 2): 500 kbit/s.",
            "Nothing can be heard on the J1939 bus.\n  - Ignition on? Truck control units stay silent with the ignition off.\n  - Is the adapter in the 9-pin diagnostic socket through the adapter cable?\n  - Black socket (type 1): 250 kbit/s, green socket (type 2): 500 kbit/s.",
        )
        .to_string())
    }

    /// Protocol A for 250 kbit/s; for 500 kbit/s the bit rate command of an
    /// STN chip, else the user protocol B.
    fn select_j1939_rate(&mut self, kbit: u32) -> bool {
        if kbit != 500 {
            return answered_ok(&self.at("SPA", DEFAULT_SEND_TIMEOUT_MS));
        }
        if self.stn
            && answered_ok(&self.at("SPA", DEFAULT_SEND_TIMEOUT_MS))
            && answered_ok(&self.send("STPBR 500000", DEFAULT_SEND_TIMEOUT_MS))
        {
            return true;
        }
        answered_ok(&self.at("PB4201", DEFAULT_SEND_TIMEOUT_MS))
            && answered_ok(&self.at("SPB", DEFAULT_SEND_TIMEOUT_MS))
    }

    /// Sets the id the next frames go out with, if it changed.
    fn set_j1939_header(&mut self, id: u32) -> Result<(), String> {
        if self.j1939_header == Some(id) {
            return Ok(());
        }
        let cp = self.at(&format!("CP{:02X}", (id >> 24) & 0x1F), 2000);
        let sh = self.at(&format!("SH{:06X}", id & 0x00FF_FFFF), 2000);
        if !answered_ok(&cp) || !answered_ok(&sh) {
            self.j1939_header = None;
            return Err(l(
                "Der Adapter nimmt den J1939-Kopf nicht an (ATCP/ATSH).",
                "The adapter does not take the J1939 header (ATCP/ATSH).",
            )
            .to_string());
        }
        self.j1939_header = Some(id);
        Ok(())
    }
}

impl FrameLink for Elm327 {
    fn exchange(
        &mut self,
        send: Option<&BusFrame>,
        listen_ms: u64,
        seen: &mut dyn FnMut(&BusFrame) -> bool,
    ) -> Result<(), String> {
        if !self.connected || self.j1939_kbit == 0 {
            return Err(l(
                "Der Adapter ist nicht im J1939-Betrieb.",
                "The adapter is not in J1939 mode.",
            )
            .to_string());
        }
        let command = match send {
            Some(frame) => {
                self.set_j1939_header(frame.id)?;
                frame.data.iter().map(|b| format!("{b:02X}")).collect()
            }
            None => "ATMA".to_string(),
        };
        let Some(t) = self.transport.as_mut() else {
            return Err(l("Kein Adapter verbunden.", "No adapter connected.").to_string());
        };
        t.flush_input();
        let mut line = command.into_bytes();
        line.push(b'\r');
        if !t.write(&line) {
            return Err(l(
                "Der Adapter nimmt nichts mehr an.",
                "The adapter no longer takes anything.",
            )
            .to_string());
        }
        let deadline = now_ms() + listen_ms;
        let mut pending: Vec<u8> = Vec::new();
        let mut prompt = false;
        let mut done = false;
        while !done && !prompt && now_ms() < deadline {
            let chunk = t.read(40);
            if chunk.is_empty() {
                continue;
            }
            pending.extend_from_slice(&chunk);
            prompt = pending.contains(&b'>');
            // Only whole lines; the rest waits for the next chunk.
            while let Some(end) = pending.iter().position(|&c| c == b'\r' || c == b'\n') {
                let text: Vec<u8> = pending.drain(..=end).collect();
                let text = String::from_utf8_lossy(&text);
                if let Some(frame) = parse_j1939_line(text.trim_end_matches(['\r', '\n']))
                    && seen(&frame)
                {
                    done = true;
                    break;
                }
            }
        }
        if !prompt {
            // Any character ends the listening; the prompt follows.
            t.write(b"\r");
            let stop = now_ms() + 1500;
            let mut tail: Vec<u8> = Vec::new();
            while now_ms() < stop && !tail.contains(&b'>') {
                tail.extend(t.read(40));
            }
        }
        Ok(())
    }

    fn name(&self) -> String {
        format!("{}, {}", self.transport_name(), self.protocol_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_header_forms() {
        let plain = parse_j1939_line("18FECA00 44 FF 69 00 03 07 FF FF").unwrap();
        assert_eq!(plain.id, 0x18FE_CA00);
        assert_eq!(
            plain.data,
            vec![0x44, 0xFF, 0x69, 0x00, 0x03, 0x07, 0xFF, 0xFF]
        );
        let bytes = parse_j1939_line("18 FE CA 00 44 FF").unwrap();
        assert_eq!(bytes.id, 0x18FE_CA00);
        assert_eq!(bytes.data, vec![0x44, 0xFF]);
        let formatted = parse_j1939_line("6 0FECA 00 44 FF").unwrap();
        assert_eq!(formatted.id, 0x18FE_CA00);
        let tight = parse_j1939_line("0CF00400F07D7D9A1100FF7D").unwrap();
        assert_eq!(tight.id, 0x0CF0_0400);
        assert_eq!(tight.data.len(), 8);
    }

    #[test]
    fn status_lines_are_no_frames() {
        for line in [
            "STOPPED",
            "NO DATA",
            "OK",
            "?",
            "BUFFER FULL",
            "",
            "18FECA00 4",
        ] {
            assert_eq!(parse_j1939_line(line), None, "{line}");
        }
        // Nine data bytes do not fit into a classic frame.
        assert_eq!(
            parse_j1939_line("18FECA00 01 02 03 04 05 06 07 08 09"),
            None
        );
    }
}
