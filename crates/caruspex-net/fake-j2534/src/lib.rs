//! A J2534-1 (04.04) driver DLL for the tests: no hardware, an engine unit
//! that answers 0100 and the VIN (multi frame, after flow control) on CAN.
//! Built only on Windows, loaded by the j2534 tests of caruspex-net through
//! LoadLibrary like a real PassThru driver.
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

struct State {
    inbox: VecDeque<PassThruMsg>,
    /// Consecutive frames waiting for flow control.
    held: Vec<Vec<u8>>,
    opened: bool,
    connected: bool,
    filtered: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    inbox: VecDeque::new(),
    held: Vec::new(),
    opened: false,
    connected: false,
    filtered: false,
});

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn push(state: &mut State, id: u32, mut data: Vec<u8>) {
    data.resize(8, 0xAA);
    let mut m = PassThruMsg {
        protocol_id: 5,
        rx_status: 0,
        tx_flags: 0,
        timestamp: 0,
        data_size: 12,
        extra_data_index: 0,
        data: [0; 4128],
    };
    m.data[..4].copy_from_slice(&id.to_be_bytes());
    m.data[4..12].copy_from_slice(&data);
    state.inbox.push_back(m);
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
    _flags: u32,
    baud: u32,
    channel: *mut u32,
) -> i32 {
    let mut s = state();
    if !s.opened || protocol != 5 || baud != 500000 {
        return 0x07; // ERR_INVALID_PROTOCOL_ID / baud
    }
    s.connected = true;
    unsafe { *channel = 2 };
    0
}

#[unsafe(no_mangle)]
pub extern "system" fn PassThruDisconnect(_channel: u32) -> i32 {
    state().connected = false;
    0
}

/// # Safety
/// `msgs` must have room for one message, `count` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn PassThruReadMsgs(
    _channel: u32,
    msgs: *mut PassThruMsg,
    count: *mut u32,
    _timeout: u32,
) -> i32 {
    let mut s = state();
    if s.inbox.is_empty() || !s.filtered {
        unsafe { *count = 0 };
        return 0x10; // ERR_BUFFER_EMPTY
    }
    let m = s.inbox.pop_front().expect("checked above");
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
    _channel: u32,
    msgs: *mut PassThruMsg,
    count: *mut u32,
    _timeout: u32,
) -> i32 {
    let mut s = state();
    if !s.connected {
        return 0x02;
    }
    let n = unsafe { *count } as usize;
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
    _channel: u32,
    kind: u32,
    _mask: *mut PassThruMsg,
    _pattern: *mut PassThruMsg,
    _flow: *mut PassThruMsg,
    id: *mut u32,
) -> i32 {
    state().filtered = kind == 1;
    unsafe { *id = 3 };
    0
}

/// # Safety
/// For READ_VBATT `out` must point to a writable u32.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn PassThruIoctl(
    _channel: u32,
    ioctl: u32,
    _input: *mut c_void,
    out: *mut c_void,
) -> i32 {
    if ioctl == 0x03 {
        unsafe { *out.cast::<u32>() = 12480 }; // READ_VBATT in mV
        return 0;
    }
    0x1A
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
