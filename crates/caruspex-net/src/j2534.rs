//! SAE J2534-1 (version 04.04) PassThru devices through their driver DLL.
//! Used in CAN mode (protocol 5): raw frames, the ISO-TP framing is done by
//! CanElm like for every other CAN interface. And on the K-line (protocols 3,
//! ISO 9141, and 4, ISO 14230) as a [`ByteStream`] for KWP1281 and KWP2000,
//! with the 5-baud and the fast init done by the device through its ioctls.
//!
//! All unsafe code of the J2534 support stays in this module: loading the
//! DLL, calling its functions and passing the message buffers.
#![allow(unsafe_code)]

use std::collections::VecDeque;
use std::ffi::{CStr, c_char, c_void};

use caruspex_i18n::l;
use caruspex_link::ByteStream;
use caruspex_link::timing::now_ms;
use libloading::Library;
use winreg::RegKey;
use winreg::enums::{
    HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY, REG_EXPAND_SZ, REG_SZ,
};

use crate::can_bus::{CanFrame, CanLink, J2534Device, KLineKind};

const PROTOCOL_ISO9141: u32 = 3;
const PROTOCOL_ISO14230: u32 = 4;
const PROTOCOL_CAN: u32 = 5;
const CAN_ID_BOTH: u32 = 0x800;
const CAN_29BIT_ID: u32 = 0x100;
/// Connect flag: the device neither adds nor checks the checksum, the whole
/// message is data. KWP1281 and KWP2000 do their own.
const ISO9141_NO_CHECKSUM: u32 = 0x200;
/// Connect flag: K-line only, no L-line.
const ISO9141_K_LINE_ONLY: u32 = 0x1000;
/// RxStatus: echo of our own frame.
const TX_MSG_TYPE: u32 = 0x01;
/// RxStatus: only the indication that a message begins, no data.
const START_OF_MESSAGE: u32 = 0x02;
const PASS_FILTER: u32 = 1;
// Ioctl ids.
const SET_CONFIG: u32 = 0x02;
const READ_VBATT: u32 = 0x03;
const FIVE_BAUD_INIT: u32 = 0x04;
const FAST_INIT: u32 = 0x05;
const CLEAR_RX_BUFFER: u32 = 0x08;
// Configuration parameters (units of 0.5 ms for the P times).
const LOOPBACK: u32 = 0x03;
const P3_MIN: u32 = 0x0A;
const FIVE_BAUD_MOD: u32 = 0x20;
/// The device sends the inverted key byte 2 itself and does not wait for an
/// inverted address: KWP1281 units send none, KWP2000 units send one, which
/// then arrives as an ordinary byte.
const FIVE_BAUD_INVERTED_KEY_BYTE: u32 = 1;
const NO_ERROR: i32 = 0;

// `unsigned long` and `long` are 32 bit on Windows, WINAPI is "system".
#[repr(C)]
struct PassThruMsg {
    protocol_id: u32,
    rx_status: u32,
    tx_flags: u32,
    timestamp: u32,
    data_size: u32,
    extra_data_index: u32,
    data: [u8; 4128],
}

impl PassThruMsg {
    fn zeroed() -> Box<Self> {
        Box::new(Self {
            protocol_id: 0,
            rx_status: 0,
            tx_flags: 0,
            timestamp: 0,
            data_size: 0,
            extra_data_index: 0,
            data: [0; 4128],
        })
    }

    fn with(protocol_id: u32, bytes: &[u8]) -> Box<Self> {
        let mut m = Self::zeroed();
        m.protocol_id = protocol_id;
        let size = bytes.len().min(m.data.len());
        m.data[..size].copy_from_slice(&bytes[..size]);
        m.data_size = size as u32;
        m
    }

    fn bytes(&self) -> &[u8] {
        &self.data[..(self.data_size as usize).min(self.data.len())]
    }
}

#[repr(C)]
struct SByteArray {
    num_of_bytes: u32,
    byte_ptr: *mut u8,
}

#[repr(C)]
struct SConfig {
    parameter: u32,
    value: u32,
}

#[repr(C)]
struct SConfigList {
    num_of_params: u32,
    config_ptr: *mut SConfig,
}

type OpenFn = unsafe extern "system" fn(*mut c_void, *mut u32) -> i32;
type CloseFn = unsafe extern "system" fn(u32) -> i32;
type ConnectFn = unsafe extern "system" fn(u32, u32, u32, u32, *mut u32) -> i32;
type DisconnectFn = unsafe extern "system" fn(u32) -> i32;
type ReadFn = unsafe extern "system" fn(u32, *mut PassThruMsg, *mut u32, u32) -> i32;
type WriteFn = unsafe extern "system" fn(u32, *mut PassThruMsg, *mut u32, u32) -> i32;
type FilterFn = unsafe extern "system" fn(
    u32,
    u32,
    *mut PassThruMsg,
    *mut PassThruMsg,
    *mut PassThruMsg,
    *mut u32,
) -> i32;
type IoctlFn = unsafe extern "system" fn(u32, u32, *mut c_void, *mut c_void) -> i32;
type LastErrorFn = unsafe extern "system" fn(*mut c_char) -> i32;

struct Functions {
    open: OpenFn,
    close: CloseFn,
    connect: ConnectFn,
    disconnect: DisconnectFn,
    read: ReadFn,
    write: WriteFn,
    filter: FilterFn,
    ioctl: Option<IoctlFn>,
    last_error: Option<LastErrorFn>,
}

/// The loaded driver with one opened device and at most one channel.
struct Driver {
    functions: Functions,
    device: u32,
    channel: u32,
    library_path: String,
    // Dropped last, after Drop::drop has closed the device.
    _library: Library,
}

// The driver handles are plain numbers; a link is used by one thread at a time.
unsafe impl Send for Driver {}

/// Looks a function up in the driver; `None` if it is missing.
///
/// # Safety
/// `T` must be the function pointer type of the exported function.
unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> Option<T> {
    // SAFETY: the caller names the right type; the pointer is copied out and
    // only used while the library stays loaded (it lives in Driver).
    unsafe { library.get::<T>(name).ok().map(|s| *s) }
}

impl Driver {
    /// Loads the DLL and opens its first device.
    fn open(library: &str) -> Result<Driver, String> {
        // SAFETY: loading a PassThru driver runs its initialisation, which is
        // what the user asked for by choosing it.
        let Ok(module) = (unsafe { Library::new(library) }) else {
            return Err(format!(
                "{}{}{}",
                l(
                    "Der J2534-Treiber lässt sich nicht laden: ",
                    "The J2534 driver cannot be loaded: "
                ),
                library,
                l(
                    " (Caruspex ist ein 64-Bit-Programm und braucht einen 64-Bit-Treiber).",
                    " (Caruspex is a 64-bit program and needs a 64-bit driver)."
                )
            ));
        };
        // SAFETY: the types follow SAE J2534-1 04.04.
        let functions = unsafe {
            (
                symbol::<OpenFn>(&module, b"PassThruOpen\0"),
                symbol::<CloseFn>(&module, b"PassThruClose\0"),
                symbol::<ConnectFn>(&module, b"PassThruConnect\0"),
                symbol::<DisconnectFn>(&module, b"PassThruDisconnect\0"),
                symbol::<ReadFn>(&module, b"PassThruReadMsgs\0"),
                symbol::<WriteFn>(&module, b"PassThruWriteMsgs\0"),
                symbol::<FilterFn>(&module, b"PassThruStartMsgFilter\0"),
                symbol::<IoctlFn>(&module, b"PassThruIoctl\0"),
                symbol::<LastErrorFn>(&module, b"PassThruGetLastError\0"),
            )
        };
        let (
            Some(open),
            Some(close),
            Some(connect),
            Some(disconnect),
            Some(read),
            Some(write),
            Some(filter),
            ioctl,
            last_error,
        ) = functions
        else {
            return Err(l(
                "Das ist kein vollständiger J2534-Treiber.",
                "This is not a complete J2534 driver.",
            )
            .into());
        };
        let mut driver = Driver {
            functions: Functions {
                open,
                close,
                connect,
                disconnect,
                read,
                write,
                filter,
                ioctl,
                last_error,
            },
            device: 0,
            channel: 0,
            library_path: library.to_string(),
            _library: module,
        };
        let mut device = 0u32;
        // SAFETY: a null name opens the first device; the id is written to `device`.
        if unsafe { (driver.functions.open)(std::ptr::null_mut(), &mut device) } != NO_ERROR {
            return Err(format!(
                "{}{}",
                l(
                    "Das J2534-Gerät antwortet nicht: ",
                    "The J2534 device does not answer: "
                ),
                driver.last()
            ));
        }
        driver.device = device;
        Ok(driver)
    }

    fn last(&self) -> String {
        let Some(last_error) = self.functions.last_error else {
            return String::new();
        };
        let mut text = [0 as c_char; 256];
        // SAFETY: J2534 writes at most 80 characters plus the terminator into the buffer.
        unsafe { last_error(text.as_mut_ptr()) };
        text[255] = 0;
        // SAFETY: the buffer is terminated above.
        unsafe { CStr::from_ptr(text.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }

    fn connect(&mut self, protocol: u32, flags: u32, baud: u32) -> Result<(), String> {
        let mut channel = 0u32;
        // SAFETY: the channel id is written to `channel`.
        let status =
            unsafe { (self.functions.connect)(self.device, protocol, flags, baud, &mut channel) };
        if status != NO_ERROR {
            return Err(self.last());
        }
        self.channel = channel;
        Ok(())
    }

    /// Lets every message through: mask and pattern zero.
    fn pass_all(&mut self, protocol: u32, size: u32) -> Result<(), String> {
        let mut mask = PassThruMsg::zeroed();
        let mut pattern = PassThruMsg::zeroed();
        mask.protocol_id = protocol;
        pattern.protocol_id = protocol;
        mask.data_size = size;
        pattern.data_size = size;
        let mut filter_id = 0u32;
        // SAFETY: mask and pattern are valid messages, the flow control
        // message is not used with a pass filter.
        let status = unsafe {
            (self.functions.filter)(
                self.channel,
                PASS_FILTER,
                &mut *mask,
                &mut *pattern,
                std::ptr::null_mut(),
                &mut filter_id,
            )
        };
        if status != NO_ERROR {
            return Err(format!(
                "{}{}",
                l(
                    "Der Empfangsfilter am J2534-Gerät ging nicht: ",
                    "The receive filter on the J2534 device failed: "
                ),
                self.last()
            ));
        }
        Ok(())
    }

    /// An ioctl on the channel; false if the driver has none or refuses.
    fn ioctl(&mut self, id: u32, input: *mut c_void, output: *mut c_void) -> bool {
        let Some(ioctl) = self.functions.ioctl else {
            return false;
        };
        let target = if id == READ_VBATT {
            self.device
        } else {
            self.channel
        };
        // SAFETY: the caller passes the structures J2534-1 names for this id.
        unsafe { ioctl(target, id, input, output) == NO_ERROR }
    }

    fn set_config(&mut self, parameter: u32, value: u32) -> bool {
        let mut config = SConfig { parameter, value };
        let mut list = SConfigList {
            num_of_params: 1,
            config_ptr: &mut config,
        };
        self.ioctl(
            SET_CONFIG,
            (&mut list as *mut SConfigList).cast::<c_void>(),
            std::ptr::null_mut(),
        )
    }

    fn write(&mut self, message: &mut PassThruMsg) -> bool {
        let mut count = 1u32;
        // SAFETY: one valid message, its count in `count`.
        let status = unsafe { (self.functions.write)(self.channel, message, &mut count, 100) };
        status == NO_ERROR && count == 1
    }

    /// The next message within `timeout_ms`, echoes and indications included.
    fn read(&mut self, timeout_ms: u64) -> Option<Box<PassThruMsg>> {
        let mut m = PassThruMsg::zeroed();
        let mut count = 1u32;
        let timeout = u32::try_from(timeout_ms).unwrap_or(u32::MAX);
        // SAFETY: room for one message, as `count` says.
        let status = unsafe { (self.functions.read)(self.channel, &mut *m, &mut count, timeout) };
        (status == NO_ERROR && count == 1).then_some(m)
    }

    fn battery_voltage(&mut self) -> Option<f64> {
        let mut millivolts = 0u32;
        // READ_VBATT takes no input and writes one unsigned long.
        if !self.ioctl(
            READ_VBATT,
            std::ptr::null_mut(),
            (&mut millivolts as *mut u32).cast::<c_void>(),
        ) {
            return None;
        }
        Some(f64::from(millivolts) / 1000.0)
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        // SAFETY: device and channel came from this driver and are closed once.
        unsafe {
            if self.channel != 0 {
                (self.functions.disconnect)(self.channel);
            }
            if self.device != 0 {
                (self.functions.close)(self.device);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CAN
// ---------------------------------------------------------------------------

struct J2534Link {
    driver: Driver,
}

pub(crate) fn open(library: &str, bitrate: u32) -> Result<Box<dyn CanLink>, String> {
    let mut driver = Driver::open(library)?;
    if let Err(reason) = driver.connect(PROTOCOL_CAN, CAN_ID_BOTH, bitrate) {
        return Err(format!(
            "{}{}",
            l(
                "CAN lässt sich am J2534-Gerät nicht verbinden: ",
                "CAN cannot be connected on the J2534 device: "
            ),
            reason
        ));
    }
    driver.pass_all(PROTOCOL_CAN, 4)?;
    Ok(Box::new(J2534Link { driver }))
}

impl CanLink for J2534Link {
    fn send(&mut self, frame: &CanFrame) -> bool {
        let mut m = PassThruMsg::zeroed();
        m.protocol_id = PROTOCOL_CAN;
        m.tx_flags = if frame.extended { CAN_29BIT_ID } else { 0 };
        m.data[..4].copy_from_slice(&frame.id.to_be_bytes());
        let size = frame.data.len().min(m.data.len() - 4);
        m.data[4..4 + size].copy_from_slice(&frame.data[..size]);
        m.data_size = (4 + size) as u32;
        self.driver.write(&mut m)
    }

    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame> {
        loop {
            let m = self.driver.read(timeout_ms)?;
            if m.rx_status & TX_MSG_TYPE != 0 {
                continue; // our own frame, looped back
            }
            if m.data_size < 4 {
                continue;
            }
            let end = m.data_size.min(12) as usize;
            return Some(CanFrame {
                extended: m.rx_status & CAN_29BIT_ID != 0,
                id: u32::from_be_bytes([m.data[0], m.data[1], m.data[2], m.data[3]]),
                data: m.data[4..end].to_vec(),
            });
        }
    }

    fn name(&self) -> String {
        format!("J2534 {}", self.driver.library_path)
    }

    fn battery_voltage(&mut self) -> Option<f64> {
        self.driver.battery_voltage()
    }
}

// ---------------------------------------------------------------------------
// K-line
// ---------------------------------------------------------------------------

/// The K-line of a PassThru device as single bytes. The device frames
/// messages itself, so bytes written are collected and go out as one message
/// as soon as the caller waits for an answer: a whole KWP2000 message, or the
/// single byte KWP1281 acknowledges with its complement.
pub struct J2534KLine {
    driver: Driver,
    protocol: u32,
    rx: VecDeque<u8>,
    tx: Vec<u8>,
    /// The inverted key byte 2 the device already sent after the 5-baud init.
    key_reply: Option<u8>,
}

pub(crate) fn open_kline(library: &str, kind: KLineKind, baud: u32) -> Result<J2534KLine, String> {
    let protocol = match kind {
        KLineKind::Iso9141 => PROTOCOL_ISO9141,
        KLineKind::Iso14230 => PROTOCOL_ISO14230,
    };
    let mut driver = Driver::open(library)?;
    if let Err(reason) = driver.connect(protocol, ISO9141_NO_CHECKSUM | ISO9141_K_LINE_ONLY, baud) {
        return Err(format!(
            "{}{}",
            l(
                "Die K-Line lässt sich am J2534-Gerät nicht verbinden: ",
                "The K-line cannot be connected on the J2534 device: "
            ),
            reason
        ));
    }
    driver.pass_all(protocol, 1)?;
    // Best effort: not every driver knows every parameter, the defaults
    // work for most units.
    driver.set_config(LOOPBACK, 0);
    // KWP1281 answers every byte with its complement at once.
    driver.set_config(P3_MIN, 0);
    driver.set_config(FIVE_BAUD_MOD, FIVE_BAUD_INVERTED_KEY_BYTE);
    Ok(J2534KLine {
        driver,
        protocol,
        rx: VecDeque::new(),
        tx: Vec::new(),
        key_reply: None,
    })
}

impl J2534KLine {
    pub fn name(&self) -> String {
        let protocol = if self.protocol == PROTOCOL_ISO14230 {
            "ISO 14230"
        } else {
            "ISO 9141"
        };
        format!("J2534 {} K-Line ({protocol})", self.driver.library_path)
    }

    fn flush_tx(&mut self) -> bool {
        if self.tx.is_empty() {
            return true;
        }
        let mut m = PassThruMsg::with(self.protocol, &self.tx);
        self.tx.clear();
        self.driver.write(&mut m)
    }

    fn clear_rx(&mut self) {
        self.rx.clear();
        self.driver
            .ioctl(CLEAR_RX_BUFFER, std::ptr::null_mut(), std::ptr::null_mut());
    }
}

impl ByteStream for J2534KLine {
    fn write_byte(&mut self, value: u8) -> bool {
        if self.key_reply.take() == Some(value) {
            return true; // the device answered the key bytes already
        }
        self.tx.push(value);
        true
    }

    fn read_byte(&mut self, timeout_ms: u64) -> Option<u8> {
        if !self.flush_tx() {
            return None;
        }
        let deadline = now_ms() + timeout_ms;
        while self.rx.is_empty() {
            let now = now_ms();
            if now >= deadline {
                break;
            }
            let Some(m) = self.driver.read((deadline - now).min(50)) else {
                continue;
            };
            if m.rx_status & (TX_MSG_TYPE | START_OF_MESSAGE) != 0 {
                continue; // our own bytes or only the start of a message
            }
            self.rx.extend(m.bytes());
        }
        self.rx.pop_front()
    }

    /// The device clocks the address out at 5 baud, reads 0x55 and the key
    /// bytes and sends the inverted key byte 2. The bytes are handed on as
    /// if they came over the line, the reply is swallowed.
    fn slow_init(&mut self, address: u8) -> bool {
        self.tx.clear();
        self.key_reply = None;
        self.clear_rx();
        let mut target = [address];
        let mut input = SByteArray {
            num_of_bytes: 1,
            byte_ptr: target.as_mut_ptr(),
        };
        let mut keys = [0u8; 8];
        let mut output = SByteArray {
            num_of_bytes: keys.len() as u32,
            byte_ptr: keys.as_mut_ptr(),
        };
        // FIVE_BAUD_INIT: one address byte in, the two key bytes out.
        if !self.driver.ioctl(
            FIVE_BAUD_INIT,
            (&mut input as *mut SByteArray).cast::<c_void>(),
            (&mut output as *mut SByteArray).cast::<c_void>(),
        ) || output.num_of_bytes < 2
        {
            return false;
        }
        self.rx.extend([0x55, keys[0], keys[1]]);
        self.key_reply = Some(!keys[1]);
        true
    }

    fn echoes_own_bytes(&self) -> bool {
        false
    }

    fn fast_init(&mut self, start: &[u8]) -> Option<Vec<u8>> {
        self.tx.clear();
        self.key_reply = None;
        self.clear_rx();
        let mut input = PassThruMsg::with(self.protocol, start);
        let mut output = PassThruMsg::zeroed();
        // FAST_INIT: the StartCommunication request in, the answer out.
        self.driver
            .ioctl(
                FAST_INIT,
                (&mut *input as *mut PassThruMsg).cast::<c_void>(),
                (&mut *output as *mut PassThruMsg).cast::<c_void>(),
            )
            .then(|| output.bytes().to_vec())
    }
}

// ---------------------------------------------------------------------------
// Registered devices
// ---------------------------------------------------------------------------

fn reg_string(key: &RegKey, value: &str) -> String {
    let Ok(raw) = key.get_raw_value(value) else {
        return String::new();
    };
    if raw.vtype != REG_SZ && raw.vtype != REG_EXPAND_SZ {
        return String::new();
    }
    let wide: Vec<u16> = raw
        .bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..end])
}

fn read_devices(view: u32, usable: bool, out: &mut Vec<J2534Device>) {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let Ok(root) = hklm.open_subkey_with_flags("SOFTWARE\\PassThruSupport.04.04", KEY_READ | view)
    else {
        return;
    };
    for sub in root.enum_keys().map_while(Result::ok) {
        let Ok(key) = root.open_subkey_with_flags(&sub, KEY_READ | view) else {
            continue;
        };
        let mut name = reg_string(&key, "Name");
        if name.is_empty() {
            name = sub.clone();
        }
        let device = J2534Device {
            name,
            vendor: reg_string(&key, "Vendor"),
            library: reg_string(&key, "FunctionLibrary"),
            usable,
        };
        if device.library.is_empty() {
            continue;
        }
        if !out.iter().any(|o| o.library == device.library) {
            out.push(device);
        }
    }
}

pub(crate) fn devices() -> Vec<J2534Device> {
    let mut out = Vec::new();
    read_devices(KEY_WOW64_64KEY, true, &mut out);
    // Drivers only registered for 32-bit programs: listed, but not loadable.
    read_devices(KEY_WOW64_32KEY, false, &mut out);
    out
}
