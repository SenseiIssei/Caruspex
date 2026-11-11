//! A J2534-1 (04.04) driver DLL for the tests: no hardware.
//!
//! On CAN (protocol 5, channel 2) an engine unit answers 0100 and the VIN
//! (multi frame, after flow control). On the K-line (protocols 3 and 4,
//! channel 4) a KWP2000 engine unit at address 0x01 wakes up through the
//! FIVE_BAUD_INIT and FAST_INIT ioctls and answers whole messages with
//! their checksum, as a device connected with ISO9141_NO_CHECKSUM hands
//! them over. Built only on Windows, loaded by the j2534 tests of
//! caruspex-net through LoadLibrary like a real PassThru driver.
#![cfg(windows)]
#![allow(non_snake_case)]

use std::collections::VecDeque;
use std::ffi::{c_char, c_void};
use std::sync::Mutex;

#[repr(C)]
#[derive(Clone)]
pub struct PassThruMsg {
    protocol_id: u32,
    rx_status: u32,
    tx_flags: u32,
    timestamp: u32,
    data_size: u32,
    extra_data_index: u32,
    data: [u8; 4128],
}

#[repr(C)]
pub struct SByteArray {
    num_of_bytes: u32,
    byte_ptr: *mut u8,
}

#[repr(C)]
pub struct SConfig {
    parameter: u32,
    value: u32,
}

#[repr(C)]
pub struct SConfigList {
    num_of_params: u32,
    config_ptr: *mut SConfig,
}

const CAN_CHANNEL: u32 = 2;
const KLINE_CHANNEL: u32 = 4;
const ISO9141_NO_CHECKSUM: u32 = 0x200;
const ERR_INVALID_CHANNEL_ID: i32 = 0x02;
const ERR_INVALID_FLAGS: i32 = 0x06;
const ERR_FAILED: i32 = 0x07;
const ERR_TIMEOUT: i32 = 0x09;
const ERR_BUFFER_EMPTY: i32 = 0x10;
const ERR_INVALID_IOCTL_ID: i32 = 0x1A;

struct State {
    inbox: VecDeque<PassThruMsg>,
    /// Consecutive frames waiting for flow control.
    held: Vec<Vec<u8>>,
    opened: bool,
    connected: bool,
    filtered: bool,
    // K-line
    k_protocol: u32,
    k_filtered: bool,
    k_inbox: VecDeque<PassThruMsg>,
    /// Tester bytes not yet a whole message.
    k_bytes: Vec<u8>,
    k_awake: bool,
    k_faults: Vec<u8>,
    five_baud_mod: u32,
}

static STATE: Mutex<State> = Mutex::new(State {
    inbox: VecDeque::new(),
    held: Vec::new(),
    opened: false,
    connected: false,
    filtered: false,
    k_protocol: 0,
    k_filtered: false,
    k_inbox: VecDeque::new(),
    k_bytes: Vec::new(),
    k_awake: false,
    k_faults: Vec::new(),
    five_baud_mod: 0,
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn message(protocol_id: u32, rx_status: u32, bytes: &[u8]) -> PassThruMsg {
    let mut m = PassThruMsg {
        protocol_id,
        rx_status,
        tx_flags: 0,
        timestamp: 0,
        data_size: bytes.len() as u32,
        extra_data_index: 0,
        data: [0; 4128],
    };
    m.data[..bytes.len()].copy_from_slice(bytes);
    m
}

fn push(state: &mut State, id: u32, mut data: Vec<u8>) {
    data.resize(8, 0xAA);
    let mut bytes = id.to_be_bytes().to_vec();
    bytes.extend_from_slice(&data);
    state.inbox.push_back(message(5, 0, &bytes));
}

fn handle(state: &mut State, m: &PassThruMsg) {
    // Every write comes back once as an echo, as real devices do.
    let mut echo = m.clone();
    echo.rx_status = 0x01;
    state.inbox.push_back(echo);
    let id = u32::from_be_bytes([m.data[0], m.data[1], m.data[2], m.data[3]]);
    let d = &m.data[4..];
    if id == 0x7E0 && (d[0] >> 4) == 3 {
        for cf in std::mem::take(&mut state.held) {
            push(state, 0x7E8, cf);
        }
        return;
    }
    if id != 0x7DF && id != 0x7E0 {
        return;
    }
    if d[0] == 2 && d[1] == 0x01 && d[2] == 0x00 {
        push(state, 0x7E8, vec![0x06, 0x41, 0x00, 0xBE, 0x3F, 0xA8, 0x13]);
    }
    if d[0] == 2 && d[1] == 0x09 && d[2] == 0x02 {
        // 49 02 01 + "WAUZZZ8V5LA012345": 20 bytes.
        push(
            state,
            0x7E8,
            vec![0x10, 0x14, 0x49, 0x02, 0x01, b'W', b'A', b'U'],
        );
        state.held = vec![
            vec![0x21, b'Z', b'Z', b'Z', b'8', b'V', b'5', b'L'],
            vec![0x22, b'A', b'0', b'1', b'2', b'3', b'4', b'5'],
        ];
    }
}

/// A KWP2000 message from the engine unit to the tester, with checksum.
fn kwp_answer(state: &mut State, payload: &[u8]) {
    let mut frame = vec![0x80 | payload.len() as u8, 0xF1, 0x01];
    frame.extend_from_slice(payload);
    frame.push(frame.iter().fold(0u8, |s, &b| s.wrapping_add(b)));
    let protocol = state.k_protocol;
    // Devices announce the start of a message before the bytes come.
    state.k_inbox.push_back(message(protocol, 0x02, &[]));
    state.k_inbox.push_back(message(protocol, 0, &frame));
}

/// Bytes the tester put on the K-line; whole messages get an answer.
fn kline(state: &mut State, bytes: &[u8]) {
    if !state.k_awake {
        return;
    }
    state.k_bytes.extend_from_slice(bytes);
    loop {
        let Some(&format) = state.k_bytes.first() else {
            return;
        };
        let length = usize::from(format & 0x3F);
        let total = 3 + length + 1;
        if state.k_bytes.len() < total {
            return;
        }
        let frame: Vec<u8> = state.k_bytes.drain(..total).collect();
        let sum = frame[..total - 1]
            .iter()
            .fold(0u8, |s, &b| s.wrapping_add(b));
        if sum != frame[total - 1] || frame[1] != 0x01 || frame[2] != 0xF1 {
            continue; // not for us
        }
        let payload = &frame[3..total - 1];
        let answer: Vec<u8> = match payload {
            [0x10, mode] => vec![0x50, *mode],
            [0x1A, 0x9B] => {
                let mut a = vec![0x5A, 0x9B];
                a.extend_from_slice(b"036906032A  MED9.5.10  ");
                a
            }
            [0x18, 0x02, 0xFF, 0x00] => {
                let mut a = vec![0x58, (state.k_faults.len() / 3) as u8];
                a.extend_from_slice(&state.k_faults);
                a
            }
            [0x14, 0xFF, 0x00] => {
                state.k_faults.clear();
                vec![0x54, 0xFF, 0x00]
            }
            [0x21, group] => vec![0x61, *group, 0x01, 0xC8, 0x14, 0x05, 0x0A, 0xBE],
            [0x3E, ..] => vec![0x7E],
            [0x82] => {
                state.k_awake = false;
                vec![0xC2]
            }
            [service, ..] => vec![0x7F, *service, 0x11],
            [] => continue,
        };
        kwp_answer(state, &answer);
    }
}

/// # Safety
/// `device` must point to a writable u32.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn PassThruOpen(_name: *mut c_void, device: *mut u32) -> i32 {
    state().opened = true;
    unsafe { *device = 1 };
    0
}

#[unsafe(no_mangle)]
pub extern "system" fn PassThruClose(_device: u32) -> i32 {
    state().opened = false;
    0
}

/// # Safety
/// `channel` must point to a writable u32.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn PassThruConnect(
    _device: u32,
    protocol: u32,
    flags: u32,
    baud: u32,
    channel: *mut u32,
) -> i32 {
    let mut s = state();
    if !s.opened {
        return ERR_FAILED;
    }
    if protocol == 3 || protocol == 4 {
        if baud != 9600 && baud != 10400 {
            return ERR_FAILED;
        }
        // The K-line protocols do their own checksum here.
        if flags & ISO9141_NO_CHECKSUM == 0 {
            return ERR_INVALID_FLAGS;
        }
        s.k_protocol = protocol;
        s.k_filtered = false;
        s.k_inbox.clear();
        s.k_bytes.clear();
        s.k_awake = false;
        // 0x4066 (P0102) present, 0x0231 (561) intermittent.
        s.k_faults = vec![0x40, 0x66, 0xE0, 0x02, 0x31, 0x60];
        s.five_baud_mod = 0;
        unsafe { *channel = KLINE_CHANNEL };
        return 0;
    }
    if protocol != 5 || baud != 500000 {
        return 0x07; // ERR_INVALID_PROTOCOL_ID / baud
    }
    s.connected = true;
    unsafe { *channel = CAN_CHANNEL };
    0
}

#[unsafe(no_mangle)]
pub extern "system" fn PassThruDisconnect(channel: u32) -> i32 {
    let mut s = state();
    if channel == KLINE_CHANNEL {
        s.k_protocol = 0;
        s.k_awake = false;
    } else {
        s.connected = false;
    }
    0
}

/// # Safety
/// `msgs` must have room for one message, `count` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn PassThruReadMsgs(
    channel: u32,
    msgs: *mut PassThruMsg,
    count: *mut u32,
    _timeout: u32,
) -> i32 {
    let mut s = state();
    let next = if channel == KLINE_CHANNEL {
        if s.k_filtered {
            s.k_inbox.pop_front()
        } else {
            None
        }
    } else if s.filtered {
        s.inbox.pop_front()
    } else {
        None
    };
    let Some(m) = next else {
        unsafe { *count = 0 };
        return ERR_BUFFER_EMPTY;
    };
    unsafe {
        *msgs = m;
        *count = 1;
    }
    0
}

/// # Safety
/// `msgs` must hold `*count` messages.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn PassThruWriteMsgs(
    channel: u32,
    msgs: *mut PassThruMsg,
    count: *mut u32,
    _timeout: u32,
) -> i32 {
    let mut s = state();
    let n = unsafe { *count } as usize;
    if channel == KLINE_CHANNEL {
        if s.k_protocol == 0 {
            return ERR_INVALID_CHANNEL_ID;
        }
        for i in 0..n {
            let m = unsafe { &*msgs.add(i) };
            let bytes = m.data[..m.data_size as usize].to_vec();
            kline(&mut s, &bytes);
        }
        return 0;
    }
    if !s.connected {
        return 0x02;
    }
    for i in 0..n {
        let m = unsafe { &*msgs.add(i) };
        handle(&mut s, m);
    }
    0
}

/// # Safety
/// `id` must point to a writable u32.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn PassThruStartMsgFilter(
    channel: u32,
    kind: u32,
    _mask: *mut PassThruMsg,
    _pattern: *mut PassThruMsg,
    _flow: *mut PassThruMsg,
    id: *mut u32,
) -> i32 {
    let mut s = state();
    if channel == KLINE_CHANNEL {
        s.k_filtered = kind == 1;
    } else {
        s.filtered = kind == 1;
    }
    unsafe { *id = 3 };
    0
}

/// # Safety
/// The pointers must be the structures J2534-1 names for the ioctl.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn PassThruIoctl(
    channel: u32,
    ioctl: u32,
    input: *mut c_void,
    out: *mut c_void,
) -> i32 {
    if ioctl == 0x03 {
        unsafe { *out.cast::<u32>() = 12480 }; // READ_VBATT in mV
        return 0;
    }
    if channel != KLINE_CHANNEL {
        return ERR_INVALID_IOCTL_ID;
    }
    let mut s = state();
    match ioctl {
        // SET_CONFIG: FIVE_BAUD_MOD is remembered, the rest accepted.
        0x02 => {
            let list = unsafe { &*input.cast::<SConfigList>() };
            for i in 0..list.num_of_params as usize {
                let c = unsafe { &*list.config_ptr.add(i) };
                if c.parameter == 0x20 {
                    s.five_baud_mod = c.value;
                }
            }
            0
        }
        // FIVE_BAUD_INIT: address in, key bytes out.
        0x04 => {
            let input = unsafe { &*input.cast::<SByteArray>() };
            let output = unsafe { &mut *out.cast::<SByteArray>() };
            if input.num_of_bytes != 1 || output.num_of_bytes < 2 {
                return ERR_FAILED;
            }
            let address = unsafe { *input.byte_ptr };
            if address != 0x01 {
                return ERR_TIMEOUT; // nobody there
            }
            unsafe {
                *output.byte_ptr = 0xEF;
                *output.byte_ptr.add(1) = 0x8F;
            }
            output.num_of_bytes = 2;
            s.k_awake = true;
            s.k_bytes.clear();
            // Modes 1 and 3: the device does not wait for the inverted
            // address, so it arrives as data.
            if s.five_baud_mod == 1 || s.five_baud_mod == 3 {
                let protocol = s.k_protocol;
                s.k_inbox.push_back(message(protocol, 0, &[!address]));
            }
            0
        }
        // FAST_INIT: StartCommunication in, its answer out.
        0x05 => {
            let request = unsafe { &*input.cast::<PassThruMsg>() };
            let answer = unsafe { &mut *out.cast::<PassThruMsg>() };
            if request.data[..request.data_size as usize] != [0x81, 0x01, 0xF1, 0x81, 0xF4] {
                return ERR_TIMEOUT;
            }
            let reply = [0x83, 0xF1, 0x01, 0xC1, 0xEF, 0x8F, 0xB4];
            *answer = message(s.k_protocol, 0, &reply);
            s.k_awake = true;
            s.k_bytes.clear();
            0
        }
        // CLEAR_TX_BUFFER, CLEAR_RX_BUFFER
        0x07 => 0,
        0x08 => {
            s.k_inbox.clear();
            0
        }
        _ => ERR_INVALID_IOCTL_ID,
    }
}

/// # Safety
/// `text` must have room for the message and its terminator.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn PassThruGetLastError(text: *mut c_char) -> i32 {
    let message = b"fake driver\0";
    unsafe {
        std::ptr::copy_nonoverlapping(message.as_ptr().cast::<c_char>(), text, message.len())
    };
    0
}
