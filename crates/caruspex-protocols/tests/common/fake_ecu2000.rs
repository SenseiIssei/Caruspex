//! Simulated KWP2000 control unit for the tests (tests/FakeEcu2000.*).
//!
//! Answers with key bytes that point to KWP2000 and responds to messages with
//! header and checksum, just like a Bosch MED in the Audi A2 1.6 FSI does.

use std::collections::VecDeque;

use caruspex_link::ByteStream;
use caruspex_protocols::kwp2000::service;

pub struct FakeEcu2000 {
    /// Faults as groups of three: two bytes number, one byte status.
    pub faults: Vec<u8>,
    pub group_payload: Vec<u8>,
    pub identification: String,
    pub clear_was_called: bool,
    pub last_group: i32,
    pub last_session: i32,
    pub session_count: i32,
    pub stopped: bool,
    /// If set, the control unit rejects this session type.
    pub refuse_session: i32,

    to_tester: VecDeque<u8>,
    incoming: Vec<u8>,
    expected_length: i32,
    awake: bool,
    handshake_pending: bool,
    address: u8,
}

impl Default for FakeEcu2000 {
    fn default() -> Self {
        Self {
            faults: vec![0x40, 0x66, 0xE0, 0x02, 0x31, 0x60],
            group_payload: vec![
                0x01, 0x40, 0x64, 0x05, 0x28, 0x8C, 0x21, 0x00, 0x50, 0x00, 0x00, 0x00,
            ],
            identification: "8E0906018AK  1.6l MED 7.5.11".into(),
            clear_was_called: false,
            last_group: -1,
            last_session: -1,
            session_count: 0,
            stopped: false,
            refuse_session: -1,
            to_tester: VecDeque::new(),
            incoming: Vec::new(),
            expected_length: -1,
            awake: false,
            handshake_pending: false,
            address: 0,
        }
    }
}

impl FakeEcu2000 {
    pub fn woken_address(&self) -> u8 {
        self.address
    }

    fn queue(&mut self, value: u8) {
        self.to_tester.push_back(value);
    }

    fn reply(&mut self, payload: &[u8]) {
        let mut frame = vec![0x80 | payload.len() as u8, 0xF1, self.address];
        // Target is the tester, source the control unit.
        frame.extend_from_slice(payload);
        let checksum = frame.iter().fold(0u8, |s, &b| s.wrapping_add(b));
        frame.push(checksum);
        for byte in frame {
            self.queue(byte);
        }
    }

    fn handle_request(&mut self, payload: &[u8]) {
        let Some(&sid) = payload.first() else {
            return;
        };
        let second = payload.get(1).map(|&b| i32::from(b)).unwrap_or(-1);
        match sid {
            service::START_DIAGNOSTIC_SESSION => {
                let mode = second;
                if mode == self.refuse_session {
                    self.reply(&[service::NEGATIVE, sid, 0x22]);
                    return;
                }
                self.last_session = mode;
                self.session_count += 1;
                self.reply(&[sid + 0x40, if mode < 0 { 0 } else { mode as u8 }]);
            }
            service::READ_ECU_IDENTIFICATION => {
                let mut out = vec![sid + 0x40, 0x9B];
                out.extend_from_slice(self.identification.as_bytes());
                self.reply(&out);
            }
            service::READ_DTC_BY_STATUS => {
                let mut out = vec![sid + 0x40, (self.faults.len() / 3) as u8];
                out.extend_from_slice(&self.faults);
                self.reply(&out);
            }
            service::CLEAR_DIAGNOSTIC_INFORMATION => {
                self.clear_was_called = true;
                self.faults.clear();
                self.reply(&[sid + 0x40]);
            }
            service::READ_DATA_BY_LOCAL_ID => {
                self.last_group = second;
                let mut out = vec![sid + 0x40, self.last_group as u8];
                out.extend_from_slice(&self.group_payload.clone());
                self.reply(&out);
            }
            service::TESTER_PRESENT => self.reply(&[sid + 0x40]),
            service::STOP_COMMUNICATION => {
                self.stopped = true;
                self.reply(&[0xC2]);
            }
            _ => self.reply(&[service::NEGATIVE, sid, 0x11]),
        }
    }
}

impl ByteStream for FakeEcu2000 {
    fn slow_init(&mut self, address: u8) -> bool {
        self.address = address;
        self.awake = true;
        self.stopped = false;
        self.to_tester.clear();
        self.incoming.clear();
        self.expected_length = -1;

        // Sync byte and key bytes of a KWP2000 control unit.
        self.queue(0x55);
        self.queue(0xEF); // 0x6F with parity bit, masked to 0x6F further down
        self.queue(0x8F);
        self.handshake_pending = true;
        true
    }

    fn read_byte(&mut self, _timeout_ms: u64) -> Option<u8> {
        self.to_tester.pop_front()
    }

    fn write_byte(&mut self, value: u8) -> bool {
        if !self.awake {
            return false;
        }

        // Single-wire line: the tester hears itself.
        self.queue(value);

        if self.handshake_pending {
            self.handshake_pending = false;
            // After the reply to the key bytes comes the inverted address.
            let inverted = !self.address;
            self.queue(inverted);
            return true;
        }

        self.incoming.push(value);
        if self.incoming.len() == 1 {
            // Format byte: the lower six bits are the payload length.
            self.expected_length = i32::from(value & 0x3F);
            return true;
        }
        // Header is three bytes, plus the payload and one checksum byte.
        let complete = 3 + self.expected_length as usize + 1;
        if self.incoming.len() < complete {
            return true;
        }

        let payload: Vec<u8> = self.incoming[3..self.incoming.len() - 1].to_vec();
        self.incoming.clear();
        self.expected_length = -1;
        self.handle_request(&payload);
        true
    }

    fn echoes_own_bytes(&self) -> bool {
        true
    }
}
