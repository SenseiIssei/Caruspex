//! What only STN chips (OBDLink) can do. Every command and number here is
//! taken from the OBDLink Family Reference and Programming Manual (FRPM,
//! revision F, OBD Solutions):
//!
//! - Batched commands (`STBC 1`, sections 8.15 and 16): several requests in
//!   one line, separated by `|`; the answers come back separated by `|` in the
//!   same order, ended by one prompt. The command buffer holds 1024
//!   characters. An error other than NO DATA ends the batch early. The
//!   setting is volatile, a reset or power cycle switches it off again.
//! - Monitoring with a count (`STMA n`, `STM n`, section 8.9): the chip stops
//!   by itself after n messages. STMA also opens the CAN hardware filter,
//!   STM keeps every filter as set.
//! - Pass filters (`STFPA pattern, mask`, sections 8.10 and 11.2; called
//!   `STFAP` before it was renamed): raw CAN frames that match no pass filter
//!   are not printed. `ATAR` clears them and returns to automatic filtering.
//! - Protocol presets (`STP p`, section 8.6): 33 to 36 are ISO 15765 on the
//!   high speed CAN of pins 6 and 14, 53 and 54 ISO 15765 on the medium speed
//!   CAN of pins 3 and 11 (Ford MS-CAN, 125 kbit), 63 and 64 ISO 15765 on the
//!   single wire CAN of pin 1 (GMLAN, 33.3 kbit). The single wire transceiver
//!   is put into normal mode by itself when such a protocol opens
//!   (`STCSWM`), and STP sets the CAN hardware filter to suit the protocol.
//!   An unsupported preset is answered with `?`.

use caruspex_i18n::l;

use super::{
    DEFAULT_SEND_TIMEOUT_MS, Elm327, ElmResult, ElmStatus, answered_ok, decode_messages, trim,
};

/// The longest batch line Caruspex builds; the chip takes 1024 characters
/// including the carriage return (FRPM 16.1).
const BATCH_LIMIT: usize = 1000;

/// A CAN bus of the OBD socket that an STN chip can be switched to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StnBus {
    /// Pins 6 and 14, the bus of the emission diagnostics.
    #[default]
    HighSpeed,
    /// Pins 3 and 11, 125 kbit: Ford's MS-CAN with the body modules.
    MediumSpeed,
    /// Pin 1, 33.3 kbit: GM's single wire CAN (GMLAN low speed).
    SingleWire,
}

impl StnBus {
    /// Short key for settings and the interface: "hs", "ms", "sw".
    pub fn key(self) -> &'static str {
        match self {
            StnBus::HighSpeed => "hs",
            StnBus::MediumSpeed => "ms",
            StnBus::SingleWire => "sw",
        }
    }

    pub fn from_key(key: &str) -> Option<StnBus> {
        match key {
            "hs" => Some(StnBus::HighSpeed),
            "ms" => Some(StnBus::MediumSpeed),
            "sw" => Some(StnBus::SingleWire),
            _ => None,
        }
    }

    /// Plain name in the active language.
    pub fn name(self) -> &'static str {
        match self {
            StnBus::HighSpeed => l(
                "High-Speed-CAN (Pins 6 und 14)",
                "High speed CAN (pins 6 and 14)",
            ),
            StnBus::MediumSpeed => l(
                "Medium-Speed-CAN (MS-CAN, Pins 3 und 11, 125 kBaud)",
                "Medium speed CAN (MS-CAN, pins 3 and 11, 125 kbaud)",
            ),
            StnBus::SingleWire => l(
                "Single-Wire-CAN (GMLAN, Pin 1, 33,3 kBaud)",
                "Single wire CAN (GMLAN, pin 1, 33.3 kbaud)",
            ),
        }
    }

    /// The STP preset for ISO 15765 on this bus, with the id size of the
    /// protocol the car was found on ('7' and '9' are 29 bit).
    pub fn stp_preset(self, protocol_id: char) -> u8 {
        let bits29 = matches!(protocol_id, '7' | '9');
        match self {
            StnBus::HighSpeed => match protocol_id {
                '7' => 34,
                '8' => 35,
                '9' => 36,
                _ => 33,
            },
            StnBus::MediumSpeed => {
                if bits29 {
                    54
                } else {
                    53
                }
            }
            StnBus::SingleWire => {
                if bits29 {
                    64
                } else {
                    63
                }
            }
        }
    }
}

// The device and IC tables of the FRPM (sections 3.1 and 3.2): which chips
// have the extra transceivers. Everything else (STN1100, STN1101, STN1110,
// STN1120, STN1130 in the OBDLink SX, STN1155 in the LX, STN2100, STN2310 in
// the CX, STN2320) only has the high speed CAN, and an unknown chip is not
// offered anything either.
const MS_AND_SW: [&str; 7] = [
    "STN1150", "STN1151", "STN1152", "STN1170", "STN2120", "STN2255", "STN2256",
];
const MS_ONLY: [&str; 3] = ["STN2230", "STN2231", "STN2232"];

/// The buses besides the high speed CAN that the chip of this STI answer
/// ("STN2232 v5.12.4") can switch to.
pub fn stn_buses(stn_id: &str) -> Vec<StnBus> {
    let chip = stn_id
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    if MS_AND_SW.contains(&chip.as_str()) {
        vec![StnBus::MediumSpeed, StnBus::SingleWire]
    } else if MS_ONLY.contains(&chip.as_str()) {
        vec![StnBus::MediumSpeed]
    } else {
        Vec::new()
    }
}

/// The answer to a batched line, cut into the answers of each command. The
/// chip puts a `|` in front of every answer after the first (FRPM 16.2.2);
/// fewer parts than commands mean the batch stopped at an error.
pub fn split_batch_answer<S: AsRef<str>>(lines: &[S]) -> Vec<Vec<String>> {
    let joined = lines
        .iter()
        .map(|line| line.as_ref())
        .collect::<Vec<_>>()
        .join("\r");
    joined
        .split('|')
        .map(|part| {
            part.split(['\r', '\n'])
                .map(trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect()
        })
        .collect()
}

impl Elm327 {
    /// Whether connect may switch on the batched commands of an STN chip
    /// (on by default). Takes effect on the next connect.
    pub fn set_stn_batching(&mut self, on: bool) {
        self.stn_batching = on;
    }

    /// The batched commands of the STN chip are on: [`Elm327::request_many`]
    /// sends several requests in one line.
    pub fn stn_batch(&self) -> bool {
        self.stn_batch
    }

    /// The bus the STN chip is switched to right now.
    pub fn stn_bus(&self) -> StnBus {
        self.stn_bus
    }

    /// The extra buses this adapter can reach in the current session: only
    /// with an STN chip that has the transceivers, connected to a car on CAN.
    pub fn stn_extra_buses(&self) -> Vec<StnBus> {
        if self.connected && self.stn && self.is_can() && !self.listen_only {
            stn_buses(&self.stn_id)
        } else {
            Vec::new()
        }
    }

    /// After a successful connect: what an STN chip offers on top.
    pub(super) fn setup_stn(&mut self) {
        self.stn_batch = false;
        self.stn_bus = StnBus::HighSpeed;
        self.hs_protocol_text = self.protocol_text.clone();
        if !self.stn || !self.stn_batching {
            return;
        }
        // Older firmware does not know the command and says "?".
        let reply = self.send("STBC1", 2000);
        self.stn_batch = answered_ok(&reply);
        self.log(if self.stn_batch {
            l(
                "  Sammelbefehle des STN-Chips (STBC) aktiv: mehrere Anfragen in einer Zeile.",
                "  Batched commands of the STN chip (STBC) active: several requests in one line.",
            )
        } else {
            l(
                "  Der STN-Chip kennt keine Sammelbefehle (STBC), Anfragen gehen einzeln.",
                "  The STN chip has no batched commands (STBC), requests go one by one.",
            )
        });
    }

    /// Several requests for the same service, the answers in the same order,
    /// each decoded like [`Elm327::request`]. With the batched commands of an
    /// STN chip they go out in as few lines as the command buffer allows, so
    /// the round trips to the adapter (a few milliseconds over USB, tens of
    /// milliseconds over Bluetooth) are paid once instead of once per
    /// request. Requests after an error that ended a batch early are sent on
    /// their own. Everywhere else this is one `request` after the other.
    pub fn request_many(
        &mut self,
        commands: &[String],
        expected_service: u8,
        timeout_ms: u64,
    ) -> Vec<ElmResult> {
        if !self.stn_batch || self.obd_on_uds || commands.len() < 2 {
            return commands
                .iter()
                .map(|command| self.request(command, expected_service, timeout_ms))
                .collect();
        }
        let mut out = Vec::with_capacity(commands.len());
        let mut start = 0;
        while start < commands.len() {
            let mut end = start + 1;
            let mut length = commands[start].len();
            while end < commands.len() && length + 1 + commands[end].len() <= BATCH_LIMIT {
                length += 1 + commands[end].len();
                end += 1;
            }
            let chunk = &commands[start..end];
            start = end;
            if chunk.len() == 1 {
                out.push(self.request(&chunk[0], expected_service, timeout_ms));
                continue;
            }
            let raw = self.send(&chunk.join("|"), timeout_ms * chunk.len() as u64);
            if raw.status == ElmStatus::AdapterError {
                out.extend(chunk.iter().map(|_| raw.clone()));
                continue;
            }
            let parts = split_batch_answer(&raw.lines);
            let format = self.header_format();
            for (i, command) in chunk.iter().enumerate() {
                match parts.get(i) {
                    Some(lines) => out.push(decode_messages(lines, expected_service, format)),
                    None => out.push(self.request(command, expected_service, timeout_ms)),
                }
            }
        }
        out
    }

    /// Frames of one 11-bit id on an STN chip. ATCRA has set the CAN
    /// hardware filter already; a pass filter on the same id lets the raw
    /// frames through to STM, which uses the filters as set and stops by
    /// itself after `max_frames`. The caller clears the filter with ATAR.
    pub(super) fn stn_monitor_id(
        &mut self,
        can_id: u32,
        duration_ms: u64,
        max_frames: usize,
    ) -> Vec<String> {
        let id = can_id & 0x7FF;
        // For 11-bit CAN the first two bytes of pattern and mask are the id.
        let pass = self.send(&format!("STFPA {id:04X},07FF"), DEFAULT_SEND_TIMEOUT_MS);
        if !answered_ok(&pass) {
            self.send(&format!("STFAP {id:04X},07FF"), DEFAULT_SEND_TIMEOUT_MS);
        }
        let command = format!("STM {}", max_frames.clamp(1, 32767));
        self.monitor_with(&command, duration_ms, max_frames)
    }

    /// Switches the STN chip to another CAN bus of the OBD socket with STP,
    /// for the control units that are only there (Ford's body modules on the
    /// MS-CAN, GM's on the single wire CAN). OBD requests to 7DF go nowhere
    /// on those buses; address the units with UDS and switch back to
    /// [`StnBus::HighSpeed`] afterwards.
    pub fn switch_stn_bus(&mut self, bus: StnBus) -> Result<(), String> {
        if !self.connected || !self.stn || !self.is_can() || self.listen_only {
            return Err(l(
                "Das geht nur mit einem OBDLink (STN-Chip) an einem Auto mit CAN.",
                "This needs an OBDLink (STN chip) on a car with CAN.",
            )
            .to_string());
        }
        if bus == self.stn_bus {
            return Ok(());
        }
        if bus != StnBus::HighSpeed && !stn_buses(&self.stn_id).contains(&bus) {
            return Err(format!(
                "{}{}",
                l(
                    "Dieser OBDLink hat keinen Anschluss für ",
                    "This OBDLink has no transceiver for "
                ),
                bus.name()
            ));
        }
        let preset = bus.stp_preset(self.protocol_id);
        let reply = self.send(&format!("STP {preset}"), DEFAULT_SEND_TIMEOUT_MS);
        if !answered_ok(&reply) {
            return Err(format!(
                "{}STP {preset}",
                l(
                    "Der Adapter lehnt das Protokoll ab: ",
                    "The adapter refuses the protocol: "
                )
            ));
        }
        self.stn_bus = bus;
        self.protocol_text = if bus == StnBus::HighSpeed {
            self.hs_protocol_text.clone()
        } else {
            format!("{} (STP {preset})", bus.name())
        };
        self.log(&format!(
            "{}{}",
            l("  Umgeschaltet auf ", "  Switched to "),
            bus.name()
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chips_with_extra_transceivers() {
        assert_eq!(stn_buses("STN2232 v5.12.4"), [StnBus::MediumSpeed]);
        assert_eq!(
            stn_buses("STN2255 v5.10.3"),
            [StnBus::MediumSpeed, StnBus::SingleWire]
        );
        assert!(stn_buses("STN1130 v5.6.19").is_empty());
        assert!(stn_buses("STN1155 v5.6.19").is_empty());
        assert!(stn_buses("").is_empty());
    }

    #[test]
    fn presets_follow_the_frpm_tables() {
        assert_eq!(StnBus::HighSpeed.stp_preset('6'), 33);
        assert_eq!(StnBus::HighSpeed.stp_preset('9'), 36);
        assert_eq!(StnBus::MediumSpeed.stp_preset('6'), 53);
        assert_eq!(StnBus::MediumSpeed.stp_preset('7'), 54);
        assert_eq!(StnBus::SingleWire.stp_preset('8'), 63);
        assert_eq!(StnBus::SingleWire.stp_preset('9'), 64);
        for bus in [StnBus::HighSpeed, StnBus::MediumSpeed, StnBus::SingleWire] {
            assert_eq!(StnBus::from_key(bus.key()), Some(bus));
        }
    }

    #[test]
    fn a_batch_answer_is_cut_at_the_pipes() {
        // The example of FRPM 8.15 with the verbose output format.
        let lines = ["OK", "|OK", "|OK", "|OK", "|7E8 06 41 00 BE 1F A8 13"];
        let parts = split_batch_answer(&lines);
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[4], ["7E8 06 41 00 BE 1F A8 13"]);
        // Several lines per answer, and NO DATA does not end the batch.
        let lines = [
            "7E8 03 41 0D 32",
            "7E9 03 41 0D 33",
            "|NO DATA",
            "|41 05 7B",
        ];
        let parts = split_batch_answer(&lines);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].len(), 2);
        assert_eq!(parts[1], ["NO DATA"]);
        assert_eq!(parts[2], ["41 05 7B"]);
    }
}
