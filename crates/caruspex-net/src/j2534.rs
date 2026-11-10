//! SAE J2534-1 (version 04.04) PassThru devices through their driver DLL.
//! Used here in CAN mode (protocol 5): raw frames, the ISO-TP framing is done
//! by CanElm like for every other CAN interface.
//!
//! All unsafe code of the J2534 support stays in this module: loading the
//! DLL, calling its functions and passing the message buffers.
#![allow(unsafe_code)]

use std::ffi::{CStr, c_char, c_void};

use caruspex_i18n::l;
use libloading::Library;
use winreg::RegKey;
use winreg::enums::{
    HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY, REG_EXPAND_SZ, REG_SZ,
};

use crate::can_bus::{CanFrame, CanLink, J2534Device};

const PROTOCOL_CAN: u32 = 5;
const CAN_ID_BOTH: u32 = 0x800;
const CAN_29BIT_ID: u32 = 0x100;
/// RxStatus: echo of our own frame.
const TX_MSG_TYPE: u32 = 0x01;
const PASS_FILTER: u32 = 1;
const READ_VBATT: u32 = 0x03;
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

struct J2534Link {
    functions: Functions,
    device: u32,
    channel: u32,
    library_path: String,
    // Dropped last, after Drop::drop has closed the device.
    _library: Library,
}

// The driver handles are plain numbers; the link is used by one thread at a time.
unsafe impl Send for J2534Link {}

/// Looks a function up in the driver; `None` if it is missing.
///
/// # Safety
/// `T` must be the function pointer type of the exported function.
unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> Option<T> {
    // SAFETY: the caller names the right type; the pointer is copied out and
    // only used while the library stays loaded (it lives in J2534Link).
    unsafe { library.get::<T>(name).ok().map(|s| *s) }
}

impl J2534Link {
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
}

impl Drop for J2534Link {
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

pub(crate) fn open(library: &str, bitrate: u32) -> Result<Box<dyn CanLink>, String> {
    // SAFETY: loading a PassThru driver runs its initialisation, which is what
    // the user asked for by choosing it.
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
    let mut link = J2534Link {
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
        library_path: String::new(),
        _library: module,
    };

    let mut device = 0u32;
    // SAFETY: a null name opens the first device; the id is written to `device`.
    if unsafe { (link.functions.open)(std::ptr::null_mut(), &mut device) } != NO_ERROR {
        return Err(format!(
            "{}{}",
            l(
                "Das J2534-Gerät antwortet nicht: ",
                "The J2534 device does not answer: "
            ),
            link.last()
        ));
    }
    link.device = device;
    let mut channel = 0u32;
    // SAFETY: the channel id is written to `channel`.
    if unsafe {
        (link.functions.connect)(
            link.device,
            PROTOCOL_CAN,
            CAN_ID_BOTH,
            bitrate,
            &mut channel,
        )
    } != NO_ERROR
    {
        return Err(format!(
            "{}{}",
            l(
                "CAN lässt sich am J2534-Gerät nicht verbinden: ",
                "CAN cannot be connected on the J2534 device: "
            ),
            link.last()
        ));
    }
    link.channel = channel;
    // Let every frame through: mask and pattern zero.
    let mut mask = PassThruMsg::zeroed();
    let mut pattern = PassThruMsg::zeroed();
    mask.protocol_id = PROTOCOL_CAN;
    pattern.protocol_id = PROTOCOL_CAN;
    mask.data_size = 4;
    pattern.data_size = 4;
    let mut filter_id = 0u32;
    // SAFETY: mask and pattern are valid messages, the flow control message
    // is not used with a pass filter.
    let status = unsafe {
        (link.functions.filter)(
            link.channel,
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
            link.last()
        ));
    }
    link.library_path = library.to_string();
    Ok(Box::new(link))
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
        let mut count = 1u32;
        // SAFETY: one valid message, its count in `count`.
        let status = unsafe { (self.functions.write)(self.channel, &mut *m, &mut count, 100) };
        status == NO_ERROR && count == 1
    }

    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame> {
        loop {
            let mut m = PassThruMsg::zeroed();
            let mut count = 1u32;
            let timeout = u32::try_from(timeout_ms).unwrap_or(u32::MAX);
            // SAFETY: room for one message, as `count` says.
            let status =
                unsafe { (self.functions.read)(self.channel, &mut *m, &mut count, timeout) };
            if status != NO_ERROR || count == 0 {
                return None;
            }
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
        format!("J2534 {}", self.library_path)
    }

    fn battery_voltage(&mut self) -> Option<f64> {
        let ioctl = self.functions.ioctl?;
        let mut millivolts = 0u32;
        // SAFETY: READ_VBATT takes no input and writes one unsigned long.
        let status = unsafe {
            ioctl(
                self.device,
                READ_VBATT,
                std::ptr::null_mut(),
                (&mut millivolts as *mut u32).cast::<c_void>(),
            )
        };
        if status != NO_ERROR {
            return None;
        }
        Some(f64::from(millivolts) / 1000.0)
    }
}

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
