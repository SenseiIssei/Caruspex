//! PEAK PCAN-Basic and Kvaser CANlib for the tests, without hardware, in
//! one DLL: it exports the CAN_* functions of PCANBasic.dll and the can*
//! functions of canlib32.dll. Behind both sits the same engine unit: it
//! answers 0100 and the VIN (multi frame, after flow control) on 7E8.
//!
//! PCAN: USB1 is there, USB2 is taken by another program, nothing else is
//! connected; only 500 kbit/s. Our own frames come back as echo frames and
//! a status frame comes first, as the real library may hand them over.
//! Kvaser: channel 0 is a Leaf Light, channel 1 the virtual channel of the
//! driver; only 500 kbit/s. An error frame comes before every answer.
#![cfg(windows)]
#![allow(non_snake_case)]

use std::collections::VecDeque;
use std::ffi::{c_char, c_void};
use std::sync::Mutex;

/// The engine unit: frames in, frames out.
#[derive(Default)]
struct Engine {
    /// Consecutive frames waiting for flow control.
    held: Vec<Vec<u8>>,
}

impl Engine {
    fn frame(&mut self, id: u32, d: &[u8]) -> Vec<Vec<u8>> {
        let pad = |mut v: Vec<u8>| {
            v.resize(8, 0xAA);
            v
        };
        if id == 0x7E0 && !d.is_empty() && (d[0] >> 4) == 3 {
            return std::mem::take(&mut self.held).into_iter().map(pad).collect();
        }
        if (id != 0x7DF && id != 0x7E0) || d.len() < 3 {
            return Vec::new();
        }
        if d[0] == 2 && d[1] == 0x01 && d[2] == 0x00 {
            return vec![pad(vec![0x06, 0x41, 0x00, 0xBE, 0x3F, 0xA8, 0x13])];
        }
        if d[0] == 2 && d[1] == 0x09 && d[2] == 0x02 {
            // 49 02 01 + "WAUZZZ8V5LA012345": 20 bytes.
            self.held = vec![
                vec![0x21, b'Z', b'Z', b'Z', b'8', b'V', b'5', b'L'],
                vec![0x22, b'A', b'0', b'1', b'2', b'3', b'4', b'5'],
            ];
            return vec![vec![0x10, 0x14, 0x49, 0x02, 0x01, b'W', b'A', b'U']];
        }
        Vec::new()
    }
}

fn write_text(text: &str, buffer: *mut c_char, room: usize) {
    let bytes = text.as_bytes();
    let n = bytes.len().min(room.saturating_sub(1));
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), buffer, n);
        *buffer.add(n) = 0;
    }
}

// ---------------------------------------------------------------------------
// PCAN-Basic
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TPCANMsg {
    id: u32,
    msg_type: u8,
    len: u8,
    data: [u8; 8],
}

#[repr(C)]
pub struct TPCANTimestamp {
    millis: u32,
    millis_overflow: u16,
    micros: u16,
}

const USB1: u16 = 0x51;
const USB2: u16 = 0x52;
const PCAN_ERROR_QRCVEMPTY: u32 = 0x0020;
const PCAN_ERROR_ILLHW: u32 = 0x1400;
const PCAN_ERROR_ILLPARAMVAL: u32 = 0x8000;
const PCAN_ERROR_INITIALIZE: u32 = 0x4000000;

struct Pcan {
    open: Option<u16>,
    inbox: VecDeque<TPCANMsg>,
    engine: Engine,
}

static PCAN: Mutex<Pcan> = Mutex::new(Pcan {
    open: None,
    inbox: VecDeque::new(),
    engine: Engine { held: Vec::new() },
});

fn pcan() -> std::sync::MutexGuard<'static, Pcan> {
    PCAN.lock().unwrap_or_else(|e| e.into_inner())
}

fn pcan_message(id: u32, msg_type: u8, data: &[u8]) -> TPCANMsg {
    let mut m = TPCANMsg {
        id,
        msg_type,
        len: data.len().min(8) as u8,
        data: [0; 8],
    };
    m.data[..data.len().min(8)].copy_from_slice(&data[..data.len().min(8)]);
    m
}

#[unsafe(no_mangle)]
pub extern "system" fn CAN_Initialize(
    channel: u16,
    btr0btr1: u16,
    _hw_type: u8,
    _io_port: u32,
    _interrupt: u16,
) -> u32 {
    let mut s = pcan();
    if channel != USB1 {
        return PCAN_ERROR_ILLHW;
    }
    if btr0btr1 != 0x001C {
        return PCAN_ERROR_ILLPARAMVAL;
    }
    s.open = Some(channel);
    s.inbox.clear();
    // A status frame first, as the bus comes up.
    s.inbox.push_back(pcan_message(0, 0x80, &[0, 0, 0, 0]));
    0
}

#[unsafe(no_mangle)]
pub extern "system" fn CAN_Uninitialize(channel: u16) -> u32 {
    let mut s = pcan();
    if s.open == Some(channel) {
        s.open = None;
    }
    0
}

/// # Safety
/// `message` must point to a writable TPCANMsg, `time` to a TPCANTimestamp or be null.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn CAN_Read(
    channel: u16,
    message: *mut TPCANMsg,
    _time: *mut TPCANTimestamp,
) -> u32 {
    let mut s = pcan();
    if s.open != Some(channel) {
        return PCAN_ERROR_INITIALIZE;
    }
    match s.inbox.pop_front() {
        Some(m) => {
            unsafe { *message = m };
            0
        }
        None => PCAN_ERROR_QRCVEMPTY,
    }
}

/// # Safety
/// `message` must point to a valid TPCANMsg.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn CAN_Write(channel: u16, message: *mut TPCANMsg) -> u32 {
    let mut s = pcan();
    if s.open != Some(channel) {
        return PCAN_ERROR_INITIALIZE;
    }
    let m = unsafe { *message };
    // Our own frame back as an echo.
    s.inbox
        .push_back(pcan_message(m.id, m.msg_type | 0x20, &m.data[..m.len as usize]));
    if m.msg_type & 0x02 == 0 {
        let answers = s.engine.frame(m.id, &m.data[..m.len as usize]);
        for a in answers {
            s.inbox.push_back(pcan_message(0x7E8, 0, &a));
        }
    }
    0
}

/// # Safety
/// `buffer` must have room for `length` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn CAN_GetValue(
    channel: u16,
    parameter: u8,
    buffer: *mut c_void,
    length: u32,
) -> u32 {
    if parameter != 0x0D || length < 4 {
        return PCAN_ERROR_ILLPARAMVAL;
    }
    // PCAN_CHANNEL_CONDITION: available, occupied or not there.
    let condition: u32 = match channel {
        USB1 => 1,
        USB2 => 2,
        _ => 0,
    };
    unsafe { *buffer.cast::<u32>() = condition };
    0
}

/// # Safety
/// `buffer` must have room for 256 characters.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn CAN_GetErrorText(error: u32, _language: u16, buffer: *mut c_char) -> u32 {
    write_text(&format!("fake pcan 0x{error:X}"), buffer, 256);
    0
}

// ---------------------------------------------------------------------------
// Kvaser CANlib
// ---------------------------------------------------------------------------

const CAN_ERR_PARAM: i32 = -1;
const CAN_ERR_NOMSG: i32 = -2;
const CAN_ERR_NOTFOUND: i32 = -3;
const CAN_MSG_STD: u32 = 0x02;
const CAN_MSG_EXT: u32 = 0x04;
const CAN_MSG_ERROR_FRAME: u32 = 0x20;

struct Kvaser {
    open: bool,
    on_bus: bool,
    /// id, data, flags
    inbox: VecDeque<(u32, Vec<u8>, u32)>,
    engine: Engine,
}

static KVASER: Mutex<Kvaser> = Mutex::new(Kvaser {
    open: false,
    on_bus: false,
    inbox: VecDeque::new(),
    engine: Engine { held: Vec::new() },
});

fn kvaser() -> std::sync::MutexGuard<'static, Kvaser> {
    KVASER.lock().unwrap_or_else(|e| e.into_inner())
}

#[unsafe(no_mangle)]
pub extern "system" fn canInitializeLibrary() {}

/// # Safety
/// `count` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn canGetNumberOfChannels(count: *mut i32) -> i32 {
    unsafe { *count = 2 };
    0
}

/// # Safety
/// `buffer` must have room for `size` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn canGetChannelData(
    channel: i32,
    item: i32,
    buffer: *mut c_void,
    size: usize,
) -> i32 {
    match (item, channel) {
        // canCHANNELDATA_CHANNEL_CAP: channel 1 is the virtual one.
        (1, 0) | (1, 1) if size >= 4 => {
            let cap: u32 = if channel == 1 { 0x0001_0000 } else { 0x0000_0100 };
            unsafe { *buffer.cast::<u32>() = cap };
            0
        }
        // canCHANNELDATA_CHAN_NO_ON_CARD
        (9, 0) | (9, 1) if size >= 4 => {
            unsafe { *buffer.cast::<u32>() = 0 };
            0
        }
        // canCHANNELDATA_DEVDESCR_ASCII
        (26, 0) | (26, 1) => {
            let name = if channel == 0 {
                "Kvaser Leaf Light v2"
            } else {
                "Kvaser Virtual CAN Driver"
            };
            write_text(name, buffer.cast(), size);
            0
        }
        _ => CAN_ERR_PARAM,
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn canOpenChannel(channel: i32, _flags: i32) -> i32 {
    if channel != 0 {
        return CAN_ERR_NOTFOUND;
    }
    let mut s = kvaser();
    s.open = true;
    s.inbox.clear();
    7 // the handle
}

#[unsafe(no_mangle)]
pub extern "system" fn canSetBusParams(
    handle: i32,
    freq: i32,
    _tseg1: u32,
    _tseg2: u32,
    _sjw: u32,
    _samples: u32,
    _sync: u32,
) -> i32 {
    if handle != 7 || freq != -2 {
        return CAN_ERR_PARAM;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "system" fn canBusOn(handle: i32) -> i32 {
    if handle != 7 {
        return CAN_ERR_PARAM;
    }
    kvaser().on_bus = true;
    0
}

#[unsafe(no_mangle)]
pub extern "system" fn canBusOff(_handle: i32) -> i32 {
    kvaser().on_bus = false;
    0
}

#[unsafe(no_mangle)]
pub extern "system" fn canClose(_handle: i32) -> i32 {
    let mut s = kvaser();
    s.open = false;
    s.on_bus = false;
    0
}

/// # Safety
/// `message` must hold `dlc` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn canWrite(
    handle: i32,
    id: i32,
    message: *mut c_void,
    dlc: u32,
    flags: u32,
) -> i32 {
    let mut s = kvaser();
    if handle != 7 || !s.on_bus {
        return CAN_ERR_PARAM;
    }
    let data = unsafe { std::slice::from_raw_parts(message.cast::<u8>(), dlc as usize) }.to_vec();
    if flags & CAN_MSG_EXT != 0 {
        return 0;
    }
    let answers = s.engine.frame(id as u32, &data);
    for a in answers {
        // A disturbance on the bus first, then the answer.
        s.inbox.push_back((0, vec![], CAN_MSG_ERROR_FRAME));
        s.inbox.push_back((0x7E8, a, CAN_MSG_STD));
    }
    0
}

/// # Safety
/// The pointers must be writable, `message` with room for 64 bytes.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn canReadWait(
    handle: i32,
    id: *mut i32,
    message: *mut c_void,
    dlc: *mut u32,
    flags: *mut u32,
    time: *mut u32,
    _timeout: u32,
) -> i32 {
    let mut s = kvaser();
    if handle != 7 || !s.on_bus {
        return CAN_ERR_PARAM;
    }
    let Some((frame_id, data, frame_flags)) = s.inbox.pop_front() else {
        return CAN_ERR_NOMSG;
    };
    unsafe {
        *id = frame_id as i32;
        std::ptr::copy_nonoverlapping(data.as_ptr(), message.cast::<u8>(), data.len());
        *dlc = data.len() as u32;
        *flags = frame_flags;
        *time = 0;
    }
    0
}

/// # Safety
/// `buffer` must have room for `size` characters.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn canGetErrorText(error: i32, buffer: *mut c_char, size: u32) -> i32 {
    write_text(&format!("fake kvaser {error}"), buffer, size as usize);
    0
}
