//! VAG TP 2.0, the transport protocol of VW, Audi, Seat and Skoda on the
//! diagnostic CAN from about 2004 to 2012, before UDS on ISO-TP took over.
//! KWP2000 runs on top of it (`VagTp20` in caruspex-protocols), with the
//! same services as on the K-line. An ELM327 cannot do this, a raw CAN
//! interface ([`CanLink`]: J2534, SLCAN, SocketCAN) can.
//!
//! The protocol as it is publicly described:
//!
//! - Channel setup: the tester sends seven bytes to CAN id 0x200,
//!   `[unit] C0 [rx lo] [rx hi] [tx lo] [tx hi] 01`: the logical address of
//!   the control unit (0x01 engine, the numbers of the K-line), the setup
//!   request C0, two CAN ids seen from the receiver of the message (0x10 in
//!   the high byte marks an id as not set) and the application type 01
//!   (KWP2000). The unit answers from 0x200 + its address with D0 and the
//!   ids the tester uses from now on: it listens on the first and sends on
//!   the second. D6 to D8 refuse the channel.
//! - Channel parameters: A0 from the tester, A1 back, each with the block
//!   size (packets before an acknowledgement is due), T1 (how long to wait
//!   for an acknowledgement), T2, T3 (gap between two packets) and T4. A
//!   time byte holds a count in its lower six bits and the unit in the upper
//!   two: 0.1 ms, 1 ms, 10 ms or 100 ms.
//! - A3 tests the channel (the unit answers A1 with its parameters) and keeps
//!   it open; A8 closes it and is answered with A8.
//! - Data: the high nibble of the first byte says what a packet is, the low
//!   nibble counts packets modulo 16, separately per direction. 0: more
//!   follow and an acknowledgement is due; 1: the last one, acknowledgement
//!   due; 2: more follow; 3: the last one without acknowledgement. B (ready)
//!   and 9 (not ready yet) acknowledge, with the number of the next packet
//!   expected. The first packet of a message starts with its length in two
//!   bytes; seven bytes of the message fit into one CAN frame.

use caruspex_i18n::l;
use caruspex_link::timing::{now_ms, sleep_ms};
use caruspex_link::{LogFn, MessageLink};

use crate::can_bus::{CanFrame, CanLink};

/// Channel setup requests go to this id, the answers come from it plus the
/// address of the unit.
pub const SETUP_ID: u32 = 0x200;
const SETUP_REQUEST: u8 = 0xC0;
const SETUP_POSITIVE: u8 = 0xD0;
const APPLICATION_KWP2000: u8 = 0x01;
const PARAMETERS_REQUEST: u8 = 0xA0;
const PARAMETERS_RESPONSE: u8 = 0xA1;
const CHANNEL_TEST: u8 = 0xA3;
const DISCONNECT: u8 = 0xA8;
/// High byte flag of a CAN id in the setup: not set.
const ID_NOT_SET: u8 = 0x10;
/// The id we ask the unit to send on.
pub const TESTER_RX_ID: u32 = 0x300;
/// Our parameters: block size 15, T1 100 ms, T2 unused, T3 5 ms, T4 unused.
const OUR_PARAMETERS: [u8; 6] = [PARAMETERS_REQUEST, 0x0F, 0x8A, 0xFF, 0x32, 0xFF];

/// How long the setup and the parameters may take.
const SETUP_MS: u64 = 500;
/// The shortest wait for an acknowledgement, whatever T1 says: USB
/// interfaces add a few milliseconds each way.
const MIN_ACK_MS: u64 = 100;
/// Between two packets of one message.
const NEXT_PACKET_MS: u64 = 1000;
/// Waiting for the A8 of the unit when we close.
const CLOSE_MS: u64 = 200;

/// A TP 2.0 time byte in microseconds.
pub fn time_byte_us(byte: u8) -> u64 {
    const UNIT_US: [u64; 4] = [100, 1_000, 10_000, 100_000];
    u64::from(byte & 0x3F) * UNIT_US[usize::from(byte >> 6)]
}

fn hex_line(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn no_channel() -> String {
    l("Kein TP-2.0-Kanal offen", "No TP 2.0 channel open").to_string()
}

fn send_failed() -> String {
    l(
        "Die CAN-Schnittstelle nimmt den Rahmen nicht an",
        "The CAN interface does not take the frame",
    )
    .to_string()
}

/// An open channel to one control unit.
#[derive(Clone, Debug)]
struct Channel {
    address: u8,
    /// We send on this id.
    tx_id: u32,
    /// The unit sends on this id.
    rx_id: u32,
    /// Packets we may send before an acknowledgement is due.
    block_size: u8,
    ack_timeout_ms: u64,
    gap_us: u64,
    tx_seq: u8,
    /// The number of the next packet from the unit.
    rx_seq: u8,
    last_traffic: u64,
}

/// TP 2.0 on a raw CAN interface. One channel at a time; dropping it closes
/// the channel.
pub struct Tp20 {
    link: Box<dyn CanLink>,
    channel: Option<Channel>,
    log: Option<LogFn>,
}

impl Tp20 {
    pub fn new(link: Box<dyn CanLink>) -> Self {
        Self {
            link,
            channel: None,
            log: None,
        }
    }

    /// Every frame and every step of the channel as a line.
    pub fn set_log(&mut self, log: LogFn) {
        self.log = Some(log);
    }

    fn log(&self, line: &str) {
        if let Some(log) = &self.log {
            log(line);
        }
    }

    /// (we send on, the unit sends on) while a channel is open.
    pub fn channel_ids(&self) -> Option<(u32, u32)> {
        self.channel.as_ref().map(|c| (c.tx_id, c.rx_id))
    }

    /// The block size the unit asked for.
    pub fn block_size(&self) -> Option<u8> {
        self.channel.as_ref().map(|c| c.block_size)
    }

    /// Milliseconds since the last exchange on the open channel.
    pub fn idle_ms(&self) -> Option<u64> {
        self.channel
            .as_ref()
            .map(|c| now_ms().saturating_sub(c.last_traffic))
    }

    fn send_frame(&mut self, id: u32, data: Vec<u8>) -> Result<(), String> {
        if self.log.is_some() {
            self.log(&format!("   TP2 -> {id:03X} {}", hex_line(&data)));
        }
        let frame = CanFrame {
            id,
            extended: false,
            data,
        };
        if self.link.send(&frame) {
            Ok(())
        } else {
            Err(send_failed())
        }
    }

    /// The next 11-bit frame from `id` before `deadline`.
    fn frame_from(&mut self, id: u32, deadline: u64) -> Option<CanFrame> {
        loop {
            let now = now_ms();
            if now >= deadline {
                return None;
            }
            let frame = self.link.receive(deadline - now)?;
            if frame.extended || frame.id != id || frame.data.is_empty() {
                continue;
            }
            if self.log.is_some() {
                self.log(&format!("   TP2 <- {id:03X} {}", hex_line(&frame.data)));
            }
            return Some(frame);
        }
    }

    /// Frames left over from before belong to nobody.
    fn drain(&mut self) {
        for _ in 0..1000 {
            if self.link.receive(0).is_none() {
                break;
            }
        }
    }

    fn channel(&self) -> Result<&Channel, String> {
        self.channel.as_ref().ok_or_else(no_channel)
    }

    fn channel_mut(&mut self) -> Result<&mut Channel, String> {
        self.channel.as_mut().ok_or_else(no_channel)
    }

    fn lost_channel(&mut self) -> String {
        self.channel = None;
        l(
            "Das Steuergerät hat den TP-2.0-Kanal geschlossen",
            "The control unit closed the TP 2.0 channel",
        )
        .to_string()
    }

    fn setup(&mut self, address: u8) -> Result<(u32, u32), String> {
        let rx = TESTER_RX_ID.to_le_bytes();
        self.send_frame(
            SETUP_ID,
            vec![
                address,
                SETUP_REQUEST,
                0x00,
                ID_NOT_SET,
                rx[0],
                rx[1],
                APPLICATION_KWP2000,
            ],
        )?;
        let answer_id = SETUP_ID + u32::from(address);
        let deadline = now_ms() + SETUP_MS;
        loop {
            let Some(f) = self.frame_from(answer_id, deadline) else {
                return Err(format!(
                    "{}{address:02X}{}",
                    l("Steuergerät ", "Control unit "),
                    l(
                        " antwortet nicht auf den TP-2.0-Kanalaufbau, vermutlich nicht verbaut",
                        " does not answer the TP 2.0 channel setup, probably not fitted"
                    )
                ));
            };
            if f.data.len() < 6 {
                continue;
            }
            match f.data[1] {
                SETUP_POSITIVE => {
                    let (rx_lo, rx_hi, tx_lo, tx_hi) = (f.data[2], f.data[3], f.data[4], f.data[5]);
                    if rx_hi & ID_NOT_SET != 0 || tx_hi & ID_NOT_SET != 0 {
                        return Err(l(
                            "Das Steuergerät nennt keine gültigen CAN-Kennungen für den Kanal",
                            "The control unit names no valid CAN ids for the channel",
                        )
                        .to_string());
                    }
                    let listen = u32::from(rx_lo) | (u32::from(rx_hi & 0x07) << 8);
                    let send = u32::from(tx_lo) | (u32::from(tx_hi & 0x07) << 8);
                    return Ok((send, listen));
                }
                code @ 0xD6..=0xD8 => {
                    return Err(format!(
                        "{}{code:02X})",
                        l(
                            "Das Steuergerät lehnt den TP-2.0-Kanal ab (Code ",
                            "The control unit refuses the TP 2.0 channel (code "
                        )
                    ));
                }
                _ => continue,
            }
        }
    }

    fn parameters(&mut self, tx_id: u32, rx_id: u32) -> Option<[u8; 6]> {
        self.send_frame(tx_id, OUR_PARAMETERS.to_vec()).ok()?;
        let deadline = now_ms() + SETUP_MS;
        loop {
            let f = self.frame_from(rx_id, deadline)?;
            if f.data[0] == PARAMETERS_RESPONSE && f.data.len() >= 6 {
                let mut out = [0u8; 6];
                out.copy_from_slice(&f.data[..6]);
                return Some(out);
            }
        }
    }

    fn acknowledge(&mut self, tx_id: u32, next: u8) -> Result<(), String> {
        self.send_frame(tx_id, vec![0xB0 | (next & 0x0F)])
    }

    fn wait_for_ack(&mut self, expected: u8) -> Result<(), String> {
        let (rx_id, t1) = {
            let c = self.channel()?;
            (c.rx_id, c.ack_timeout_ms)
        };
        let start = now_ms();
        let latest = start + t1 * 10;
        let mut deadline = start + t1;
        loop {
            let Some(f) = self.frame_from(rx_id, deadline) else {
                return Err(l(
                    "Das Steuergerät bestätigt das TP-2.0-Paket nicht",
                    "The control unit does not acknowledge the TP 2.0 packet",
                )
                .to_string());
            };
            let first = f.data[0];
            if first == DISCONNECT {
                return Err(self.lost_channel());
            }
            match first >> 4 {
                0xB => {
                    if first & 0x0F == expected {
                        return Ok(());
                    }
                    return Err(format!(
                        "{}{expected:X}{}{:X})",
                        l(
                            "TP-2.0-Bestätigung außer der Reihe (erwartet ",
                            "TP 2.0 acknowledgement out of order (expected "
                        ),
                        l(", bekommen ", ", got "),
                        first & 0x0F
                    ));
                }
                // Not ready yet: the ready one follows.
                0x9 => deadline = latest.min(now_ms() + t1),
                _ => {}
            }
        }
    }
}

impl MessageLink for Tp20 {
    fn open_channel(&mut self, address: u8) -> Result<(), String> {
        self.close_channel();
        self.drain();
        let (tx_id, rx_id) = self.setup(address)?;
        let Some(theirs) = self.parameters(tx_id, rx_id) else {
            let _ = self.send_frame(tx_id, vec![DISCONNECT]);
            return Err(l(
                "Das Steuergerät antwortet nicht auf die TP-2.0-Kanalparameter",
                "The control unit does not answer the TP 2.0 channel parameters",
            )
            .to_string());
        };
        let block_size = if theirs[1] == 0 { 15 } else { theirs[1] };
        let ack_timeout_ms = (time_byte_us(theirs[2]) / 1000).max(MIN_ACK_MS);
        let channel = Channel {
            address,
            tx_id,
            rx_id,
            block_size,
            ack_timeout_ms,
            gap_us: time_byte_us(theirs[4]),
            tx_seq: 0,
            rx_seq: 0,
            last_traffic: now_ms(),
        };
        self.log(&format!(
            "   TP 2.0: {address:02X} -> {tx_id:03X}, <- {rx_id:03X}, {}{block_size}",
            l("Blockgröße ", "block size ")
        ));
        self.channel = Some(channel);
        Ok(())
    }

    fn close_channel(&mut self) {
        let Some(c) = self.channel.take() else {
            return;
        };
        if self.send_frame(c.tx_id, vec![DISCONNECT]).is_ok() {
            let deadline = now_ms() + CLOSE_MS;
            while let Some(f) = self.frame_from(c.rx_id, deadline) {
                if f.data[0] == DISCONNECT {
                    break;
                }
            }
        }
        self.log(&format!(
            "   TP 2.0: {}{:02X}",
            l("Kanal geschlossen, ", "channel closed, "),
            c.address
        ));
    }

    fn channel_open(&self) -> bool {
        self.channel.is_some()
    }

    fn send_message(&mut self, payload: &[u8]) -> Result<(), String> {
        if payload.is_empty() || payload.len() > usize::from(u16::MAX) {
            return Err(l(
                "Nachricht hat eine unmögliche Länge",
                "Message has an impossible length",
            )
            .to_string());
        }
        let (tx_id, block_size, gap_us) = {
            let c = self.channel()?;
            (c.tx_id, c.block_size, c.gap_us)
        };
        let mut data = (payload.len() as u16).to_be_bytes().to_vec();
        data.extend_from_slice(payload);
        let packets: Vec<&[u8]> = data.chunks(7).collect();
        let mut since_ack = 0u8;
        for (index, chunk) in packets.iter().enumerate() {
            let last = index + 1 == packets.len();
            since_ack += 1;
            let wants_ack = last || since_ack >= block_size;
            let kind: u8 = match (wants_ack, last) {
                (true, false) => 0x0,
                (true, true) => 0x1,
                (false, false) => 0x2,
                (false, true) => 0x3,
            };
            let seq = {
                let c = self.channel_mut()?;
                let seq = c.tx_seq;
                c.tx_seq = (seq + 1) & 0x0F;
                seq
            };
            let mut frame = vec![(kind << 4) | seq];
            frame.extend_from_slice(chunk);
            self.send_frame(tx_id, frame)?;
            if wants_ack {
                self.wait_for_ack((seq + 1) & 0x0F)?;
                since_ack = 0;
            } else if gap_us >= 1000 {
                sleep_ms(gap_us / 1000);
            }
        }
        self.channel_mut()?.last_traffic = now_ms();
        Ok(())
    }

    fn receive_message(&mut self, timeout_ms: u64) -> Result<Vec<u8>, String> {
        let (tx_id, rx_id) = {
            let c = self.channel()?;
            (c.tx_id, c.rx_id)
        };
        let mut buffer: Vec<u8> = Vec::new();
        let mut deadline = now_ms() + timeout_ms;
        loop {
            let Some(f) = self.frame_from(rx_id, deadline) else {
                return Err(if buffer.is_empty() {
                    l(
                        "Steuergerät antwortet nicht",
                        "Control unit does not respond",
                    )
                    .to_string()
                } else {
                    l(
                        "Die TP-2.0-Nachricht bricht mittendrin ab",
                        "The TP 2.0 message breaks off halfway",
                    )
                    .to_string()
                });
            };
            let first = f.data[0];
            match first {
                DISCONNECT => {
                    let _ = self.send_frame(tx_id, vec![DISCONNECT]);
                    return Err(self.lost_channel());
                }
                CHANNEL_TEST => {
                    // The unit checks on us: our parameters back.
                    let mut answer = OUR_PARAMETERS.to_vec();
                    answer[0] = PARAMETERS_RESPONSE;
                    self.send_frame(tx_id, answer)?;
                    continue;
                }
                _ => {}
            }
            let kind = first >> 4;
            if kind > 0x3 {
                continue; // a late acknowledgement or parameters
            }
            let seq = first & 0x0F;
            let wants_ack = kind == 0x0 || kind == 0x1;
            let expected = self.channel()?.rx_seq;
            if seq == (expected + 15) & 0x0F && buffer.is_empty() && wants_ack {
                // Sent again because our acknowledgement got lost.
                self.acknowledge(tx_id, expected)?;
                continue;
            }
            let next = (seq + 1) & 0x0F;
            self.channel_mut()?.rx_seq = next;
            buffer.extend_from_slice(&f.data[1..]);
            if wants_ack {
                self.acknowledge(tx_id, next)?;
            }
            if kind == 0x1 || kind == 0x3 {
                self.channel_mut()?.last_traffic = now_ms();
                if buffer.len() < 2 {
                    return Err(l(
                        "TP-2.0-Nachricht ohne Länge",
                        "TP 2.0 message without length",
                    )
                    .to_string());
                }
                let length = usize::from(u16::from_be_bytes([buffer[0], buffer[1]]));
                if buffer.len() < 2 + length {
                    return Err(format!(
                        "{}{}{}{length}",
                        l(
                            "TP-2.0-Nachricht kürzer als angekündigt: ",
                            "TP 2.0 message shorter than announced: "
                        ),
                        buffer.len() - 2,
                        l(" statt ", " instead of ")
                    ));
                }
                return Ok(buffer[2..2 + length].to_vec());
            }
            deadline = deadline.max(now_ms() + NEXT_PACKET_MS);
        }
    }

    fn keep_alive(&mut self) -> Result<(), String> {
        let (tx_id, rx_id, t1) = {
            let c = self.channel()?;
            (c.tx_id, c.rx_id, c.ack_timeout_ms)
        };
        self.send_frame(tx_id, vec![CHANNEL_TEST])?;
        let deadline = now_ms() + t1.max(SETUP_MS);
        loop {
            let Some(f) = self.frame_from(rx_id, deadline) else {
                self.channel = None;
                return Err(l(
                    "Das Steuergerät antwortet nicht auf den TP-2.0-Kanaltest, die Verbindung ist weg",
                    "The control unit does not answer the TP 2.0 channel test, the connection is gone",
                )
                .to_string());
            };
            match f.data[0] {
                PARAMETERS_RESPONSE => {
                    self.channel_mut()?.last_traffic = now_ms();
                    return Ok(());
                }
                DISCONNECT => return Err(self.lost_channel()),
                _ => {}
            }
        }
    }

    fn name(&self) -> String {
        format!("{} TP 2.0", self.link.name())
    }
}

impl Drop for Tp20 {
    fn drop(&mut self) {
        self.close_channel();
    }
}

#[cfg(test)]
mod tests {
    use super::time_byte_us;

    #[test]
    fn time_bytes_carry_their_unit_in_the_upper_two_bits() {
        assert_eq!(time_byte_us(0x8A), 100_000); // 10 x 10 ms
        assert_eq!(time_byte_us(0x32), 5_000); // 50 x 0.1 ms
        assert_eq!(time_byte_us(0x4A), 10_000); // 10 x 1 ms
        assert_eq!(time_byte_us(0xC1), 100_000); // 1 x 100 ms
    }
}
