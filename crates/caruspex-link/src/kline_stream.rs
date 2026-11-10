//! KWP1281 over a KKL cable (VAG 409.1).

use std::collections::VecDeque;

use caruspex_i18n::l;

use crate::serial::Serial;
use crate::timing::{now_ms, sleep_ms, sleep_until_ms};
use crate::transport::ByteStream;

/// 5 baud means 200 milliseconds per bit.
const BIT_MS: u64 = 200;

/// The ten bits of the 5-baud wake-up address: start bit, 7 data bits (LSB
/// first), odd parity, stop bit. `true` is a one (line released).
pub fn wake_up_bits(address: u8) -> [bool; 10] {
    let mut bits = [false; 10]; // bits[0] is the start bit
    let mut ones = 0;
    for i in 0..7 {
        let bit = (address >> i) & 0x01 != 0;
        bits[1 + i] = bit;
        if bit {
            ones += 1;
        }
    }
    bits[8] = ones % 2 == 0; // odd parity
    bits[9] = true; // stop bit
    bits
}

/// Implements the byte stream interface on a real serial line.
/// The 5-baud wake-up address is clocked out by hand because no UART runs that
/// slowly: for each bit the transmit line is pulled low or released for
/// 200 ms.
pub struct KLineStream {
    port: Serial,
    baud_after_init: u32,
    buffer: VecDeque<u8>,
    last_error: String,
}

impl Default for KLineStream {
    fn default() -> Self {
        Self {
            port: Serial::new(),
            baud_after_init: 9600,
            buffer: VecDeque::new(),
            last_error: String::new(),
        }
    }
}

impl KLineStream {
    pub fn new() -> Self {
        Self::default()
    }

    /// After the wake-up address the line continues at this baud rate.
    /// Almost all VAG control units of the K-line era use 9600.
    pub fn open(&mut self, device: &str, baud_after_init: u32) -> Result<(), String> {
        self.baud_after_init = baud_after_init;
        self.buffer.clear();
        if let Err(error) = self.port.open(device, baud_after_init) {
            self.last_error = error.clone();
            return Err(error);
        }
        Ok(())
    }

    pub fn close(&mut self) {
        self.port.close();
    }

    pub fn is_open(&self) -> bool {
        self.port.is_open()
    }

    pub fn last_error(&self) -> &str {
        &self.last_error
    }
}

impl ByteStream for KLineStream {
    fn write_byte(&mut self, value: u8) -> bool {
        self.port.write(&[value])
    }

    fn read_byte(&mut self, timeout_ms: u64) -> Option<u8> {
        if let Some(value) = self.buffer.pop_front() {
            return Some(value);
        }
        let deadline = now_ms() + timeout_ms;
        while now_ms() < deadline {
            let chunk = self.port.read(20);
            if !chunk.is_empty() {
                self.buffer.extend(chunk);
                return self.buffer.pop_front();
            }
        }
        None
    }

    // The wake-up address goes over the line at 5 baud, 7 data bits, odd parity
    // and one stop bit. No UART can do that, so every bit is clocked by hand:
    // break on means zero, break off means one.
    fn slow_init(&mut self, address: u8) -> bool {
        if !self.port.is_open() {
            return false;
        }

        self.port.flush_input();
        self.buffer.clear();

        // The line has to idle briefly before the wake-up.
        self.port.set_break(false);
        sleep_ms(350);

        // Every bit gets a fixed target time, counted from the start of the
        // wake-up address. That way the operating system's overshoot does not
        // carry over from bit to bit.
        let start = now_ms();
        for (index, bit) in wake_up_bits(address).into_iter().enumerate() {
            self.port.set_break(!bit); // an active break pulls the line low
            sleep_until_ms(start + (index as u64 + 1) * BIT_MS);
        }
        self.port.set_break(false);

        // The control unit answers with 0x55 after about 60 milliseconds.
        // Everything our own wake-up address left behind as an echo must go.
        sleep_ms(30);
        self.port.flush_input();
        self.buffer.clear();

        if !self.port.change_baud(self.baud_after_init) {
            self.last_error = l(
                "Baudrate lässt sich nach der Weckadresse nicht setzen",
                "Cannot set the baud rate after the wake-up address",
            )
            .to_string();
            return false;
        }
        true
    }

    fn echoes_own_bytes(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_up_bits_of_the_engine_address() {
        // 0x01: data bits 1000000, one 1 -> parity bit 0.
        assert_eq!(
            wake_up_bits(0x01),
            [
                false, true, false, false, false, false, false, false, false, true
            ]
        );
        // 0x03: two ones -> parity 1.
        assert!(wake_up_bits(0x03)[8]);
        // The eighth address bit is not sent.
        assert_eq!(wake_up_bits(0x81), wake_up_bits(0x01));
    }

    #[test]
    fn a_closed_stream_refuses() {
        let mut k = KLineStream::new();
        assert!(!k.is_open());
        assert!(!k.slow_init(0x01));
        assert!(!k.write_byte(0x55));
        assert_eq!(k.read_byte(5), None);
    }
}
