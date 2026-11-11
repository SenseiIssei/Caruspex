//! Simulated KWP1281 control unit for the tests (tests/FakeEcu.*).
//!
//! Reproduces the byte-by-byte rhythm of the real protocol: everything the
//! tester sends comes back as an echo (single-wire line), data bytes are
//! acknowledged with their one's complement, the block end 0x03 is not. This
//! way the state machine can be checked without a car and without a cable.

use std::collections::VecDeque;

use caruspex_link::ByteStream;
use caruspex_protocols::kwp1281::block;

pub struct FakeEcu {
    // What the simulated device returns on request.
    pub fault_payload: Vec<u8>,
    pub group_payload: Vec<u8>,
    pub supports_basic_setting: bool,
    pub supports_adaptation: bool,
    pub supports_actuator_test: bool,
    pub adaptation_value: i32,
    pub actuator_name: String,
    pub last_adaptation_channel: i32,
    pub actuator_test_count: i32,
    pub clear_was_called: bool,
    pub last_group_requested: i32,
    pub last_basic_setting_group: i32,
    pub end_output_count: i32,

    identification: Vec<String>,
    /// Waiting to be read.
    to_tester: VecDeque<u8>,
    /// Rest of the block we are sending.
    pending_block: VecDeque<u8>,
    incoming: Vec<u8>,
    expected_length: i32,
    /// We have sent and are waiting for the acknowledgement.
    awaiting_ack: bool,
    last_sent: u8,
    /// Next byte is the reply to KW2.
    handshake_pending: bool,
    address: u8,
    counter: u8,
    awake: bool,
    identification_index: usize,
}

impl Default for FakeEcu {
    fn default() -> Self {
        Self::with_identification(&["1C0906032AA ", "SIMOS 1.0 "])
    }
}

impl FakeEcu {
    pub fn with_identification(identification: &[&str]) -> Self {
        Self {
            fault_payload: vec![0x11, 0x1B, 0x24, 0x00, 0x77, 0xA4],
            group_payload: vec![
                0x01, 0x40, 0x64, 0x05, 0x28, 0x8C, 0x21, 0x00, 0x50, 0x00, 0x00, 0x00,
            ],
            supports_basic_setting: true,
            supports_adaptation: true,
            supports_actuator_test: true,
            adaptation_value: 128,
            actuator_name: "Tankentlueftungsventil N80".into(),
            last_adaptation_channel: -1,
            actuator_test_count: 0,
            clear_was_called: false,
            last_group_requested: -1,
            last_basic_setting_group: -1,
            end_output_count: 0,
            identification: identification.iter().map(|s| s.to_string()).collect(),
            to_tester: VecDeque::new(),
            pending_block: VecDeque::new(),
            incoming: Vec::new(),
            expected_length: -1,
            awaiting_ack: false,
            last_sent: 0,
            handshake_pending: false,
            address: 0,
            counter: 0,
            awake: false,
            identification_index: 0,
        }
    }

    pub fn woken_address(&self) -> u8 {
        self.address
    }

    fn queue(&mut self, value: u8) {
        self.to_tester.push_back(value);
    }

    fn next_identification_or_ack(&mut self) {
        if self.identification_index < self.identification.len() {
            let text = self.identification[self.identification_index].clone();
            self.identification_index += 1;
            self.begin_block(block::ASCII_DATA, text.as_bytes());
        } else {
            self.begin_block(block::ACK, &[]);
        }
    }

    fn begin_block(&mut self, title: u8, payload: &[u8]) {
        self.counter = self.counter.wrapping_add(1);
        self.pending_block.clear();
        self.pending_block.push_back((payload.len() + 3) as u8);
        self.pending_block.push_back(self.counter);
        self.pending_block.push_back(title);
        self.pending_block.extend(payload.iter().copied());
        self.pending_block.push_back(0x03);
        self.pump_out();
    }

    fn pump_out(&mut self) {
        if self.awaiting_ack {
            return;
        }
        let Some(next) = self.pending_block.pop_front() else {
            return;
        };
        self.queue(next);
        self.last_sent = next;
        // No acknowledgement follows the block end.
        self.awaiting_ack = next != 0x03 || !self.pending_block.is_empty();
    }

    fn handle_complete_block(&mut self) {
        if self.incoming.len() < 3 {
            return;
        }
        self.counter = self.incoming[1];
        let title = self.incoming[2];
        let payload: Vec<u8> = self.incoming[3..self.incoming.len() - 1].to_vec();
        let first = payload.first().map(|&b| i32::from(b)).unwrap_or(-1);

        match title {
            // During the introduction: next identification block, otherwise
            // done.
            block::ACK => self.next_identification_or_ack(),
            block::READ_FAULTS => {
                let faults = self.fault_payload.clone();
                self.begin_block(block::FAULT_LIST, &faults);
            }
            block::CLEAR_FAULTS => {
                self.clear_was_called = true;
                self.fault_payload.clear();
                self.begin_block(block::ACK, &[]);
            }
            block::READ_GROUP => {
                self.last_group_requested = first;
                let group = self.group_payload.clone();
                self.begin_block(block::GROUP_READING, &group);
            }
            block::BASIC_SETTING => {
                self.last_basic_setting_group = first;
                if self.supports_basic_setting {
                    let group = self.group_payload.clone();
                    self.begin_block(block::GROUP_READING, &group);
                } else {
                    self.begin_block(block::NOT_SUPPORTED, &[]);
                }
            }
            block::READ_ADAPTATION => {
                self.last_adaptation_channel = first;
                if !self.supports_adaptation {
                    self.begin_block(block::NOT_SUPPORTED, &[]);
                    return;
                }
                let mut reply = self.group_payload.clone();
                reply.push(((self.adaptation_value >> 8) & 0xFF) as u8);
                reply.push((self.adaptation_value & 0xFF) as u8);
                self.begin_block(block::ADAPTATION_VALUE, &reply);
            }
            block::ACTUATOR_TEST => {
                self.actuator_test_count += 1;
                if !self.supports_actuator_test {
                    self.begin_block(block::NOT_SUPPORTED, &[]);
                } else {
                    let name = self.actuator_name.clone();
                    self.begin_block(block::ASCII_DATA, name.as_bytes());
                }
            }
            block::END_OUTPUT => {
                self.end_output_count += 1;
                self.awake = false;
            }
            _ => self.begin_block(block::NOT_SUPPORTED, &[]),
        }
    }
}

impl ByteStream for FakeEcu {
    fn slow_init(&mut self, address: u8) -> bool {
        self.address = address;
        self.awake = true;
        self.counter = 0;
        self.identification_index = 0;
        self.to_tester.clear();
        self.pending_block.clear();
        self.incoming.clear();
        self.expected_length = -1;
        self.awaiting_ack = false;

        // Sync byte and the two key bytes of a typical VAG engine control
        // unit.
        self.queue(0x55);
        self.queue(0x01);
        self.queue(0x8A);
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

        // On the single-wire line the tester always hears itself.
        self.queue(value);

        // Reply to the key bytes: just swallow it, then the control unit
        // introduces itself with its identification blocks.
        if self.handshake_pending {
            self.handshake_pending = false;
            self.next_identification_or_ack();
            return true;
        }

        // Acknowledgement of a byte we have sent.
        if self.awaiting_ack {
            if value != !self.last_sent {
                return false;
            }
            self.awaiting_ack = false;
            self.pump_out();
            return true;
        }

        // From here on these are the tester's data bytes.
        self.incoming.push(value);
        if self.incoming.len() == 1 {
            self.expected_length = i32::from(value);
            self.queue(!value);
            return true;
        }
        if self.incoming.len() as i32 == self.expected_length + 1 {
            // The block end is not acknowledged.
            self.handle_complete_block();
            self.incoming.clear();
            self.expected_length = -1;
            return true;
        }
        self.queue(!value);
        true
    }

    fn echoes_own_bytes(&self) -> bool {
        true
    }
}
