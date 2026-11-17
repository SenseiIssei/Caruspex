//! A PassThru driver DLL loaded into this program. All unsafe code of the
//! J2534 support is here: loading the DLL, calling its functions and
//! passing the message buffers.
#![allow(unsafe_code)]

use std::ffi::{CStr, c_char, c_void};

use libloading::Library;

use crate::api::constants::{ERR_NOT_SUPPORTED, MESSAGE_ROOM, NO_ERROR};
use crate::api::{Ioctl, IoctlOut, Message, PassThru};

// `unsigned long` and `long` are 32 bit on Windows, WINAPI is "system".
#[repr(C)]
struct PassThruMsg {
    protocol_id: u32,
    rx_status: u32,
    tx_flags: u32,
    timestamp: u32,
    data_size: u32,
    extra_data_index: u32,
    data: [u8; MESSAGE_ROOM],
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
            data: [0; MESSAGE_ROOM],
        })
    }

    fn from(m: &Message) -> Box<Self> {
        let mut out = Self::zeroed();
        out.protocol_id = m.protocol_id;
        out.rx_status = m.rx_status;
        out.tx_flags = m.tx_flags;
        out.timestamp = m.timestamp;
        out.extra_data_index = m.extra_data_index;
        let size = m.data.len().min(MESSAGE_ROOM);
        out.data[..size].copy_from_slice(&m.data[..size]);
        out.data_size = size as u32;
        out
    }

    fn to_message(&self) -> Message {
        Message {
            protocol_id: self.protocol_id,
            rx_status: self.rx_status,
            tx_flags: self.tx_flags,
            timestamp: self.timestamp,
            extra_data_index: self.extra_data_index,
            data: self.data[..(self.data_size as usize).min(MESSAGE_ROOM)].to_vec(),
        }
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

/// The driver DLL, loaded into this program.
pub struct Dll {
    functions: Functions,
    library_path: String,
    // Dropped last; whoever opened devices closes them before.
    _library: Library,
}

// The driver handles are plain numbers; a driver is used by one thread at a time.
unsafe impl Send for Dll {}

/// Looks a function up in the driver; `None` if it is missing.
///
/// # Safety
/// `T` must be the function pointer type of the exported function.
unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> Option<T> {
    // SAFETY: the caller names the right type; the pointer is copied out and
    // only used while the library stays loaded (it lives in Dll).
    unsafe { library.get::<T>(name).ok().map(|s| *s) }
}

/// Why a DLL is no usable driver.
#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    /// LoadLibrary failed: missing, or built for another machine.
    NotLoadable,
    /// A function of J2534-1 is missing.
    Incomplete,
}

impl Dll {
    /// Loads the driver DLL; no device is opened yet.
    pub fn load(library: &str) -> Result<Dll, LoadError> {
        // SAFETY: loading a PassThru driver runs its initialisation, which is
        // what the user asked for by choosing it.
        let module = unsafe { Library::new(library) }.map_err(|_| LoadError::NotLoadable)?;
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
            return Err(LoadError::Incomplete);
        };
        Ok(Dll {
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
            library_path: library.to_string(),
            _library: module,
        })
    }

    fn call_ioctl(&mut self, target: u32, id: u32, input: *mut c_void, output: *mut c_void) -> i32 {
        let Some(ioctl) = self.functions.ioctl else {
            return ERR_NOT_SUPPORTED;
        };
        // SAFETY: the caller passes the structures J2534-1 names for this id.
        unsafe { ioctl(target, id, input, output) }
    }
}

impl PassThru for Dll {
    fn open(&mut self) -> Result<u32, i32> {
        let mut device = 0u32;
        // SAFETY: a null name opens the first device; the id is written to `device`.
        let status = unsafe { (self.functions.open)(std::ptr::null_mut(), &mut device) };
        if status == NO_ERROR {
            Ok(device)
        } else {
            Err(status)
        }
    }

    fn close(&mut self, device: u32) -> i32 {
        // SAFETY: the id came from this driver.
        unsafe { (self.functions.close)(device) }
    }

    fn connect(&mut self, device: u32, protocol: u32, flags: u32, baud: u32) -> Result<u32, i32> {
        let mut channel = 0u32;
        // SAFETY: the channel id is written to `channel`.
        let status =
            unsafe { (self.functions.connect)(device, protocol, flags, baud, &mut channel) };
        if status == NO_ERROR {
            Ok(channel)
        } else {
            Err(status)
        }
    }

    fn disconnect(&mut self, channel: u32) -> i32 {
        // SAFETY: the id came from this driver.
        unsafe { (self.functions.disconnect)(channel) }
    }

    fn read(&mut self, channel: u32, timeout_ms: u32) -> Result<Option<Message>, i32> {
        let mut m = PassThruMsg::zeroed();
        let mut count = 1u32;
        // SAFETY: room for one message, as `count` says.
        let status = unsafe { (self.functions.read)(channel, &mut *m, &mut count, timeout_ms) };
        if status != NO_ERROR {
            return Err(status);
        }
        Ok((count == 1).then(|| m.to_message()))
    }

    fn write(&mut self, channel: u32, message: &Message, timeout_ms: u32) -> Result<u32, i32> {
        let mut m = PassThruMsg::from(message);
        let mut count = 1u32;
        // SAFETY: one valid message, its count in `count`.
        let status = unsafe { (self.functions.write)(channel, &mut *m, &mut count, timeout_ms) };
        if status == NO_ERROR {
            Ok(count)
        } else {
            Err(status)
        }
    }

    fn start_filter(
        &mut self,
        channel: u32,
        kind: u32,
        mask: &Message,
        pattern: &Message,
        flow: Option<&Message>,
    ) -> Result<u32, i32> {
        let mut mask = PassThruMsg::from(mask);
        let mut pattern = PassThruMsg::from(pattern);
        let mut flow = flow.map(PassThruMsg::from);
        let flow_ptr = match flow.as_mut() {
            Some(f) => &mut **f as *mut PassThruMsg,
            None => std::ptr::null_mut(),
        };
        let mut filter_id = 0u32;
        // SAFETY: mask and pattern are valid messages, the flow control
        // message is valid or null (not used with a pass filter).
        let status = unsafe {
            (self.functions.filter)(
                channel,
                kind,
                &mut *mask,
                &mut *pattern,
                flow_ptr,
                &mut filter_id,
            )
        };
        if status == NO_ERROR {
            Ok(filter_id)
        } else {
            Err(status)
        }
    }

    fn ioctl(&mut self, target: u32, request: &Ioctl) -> Result<IoctlOut, i32> {
        let id = request.id();
        let null = std::ptr::null_mut::<c_void>();
        let (status, out) = match request {
            Ioctl::SetConfig(pairs) => {
                let mut configs: Vec<SConfig> = pairs
                    .iter()
                    .map(|&(parameter, value)| SConfig { parameter, value })
                    .collect();
                let mut list = SConfigList {
                    num_of_params: configs.len() as u32,
                    config_ptr: configs.as_mut_ptr(),
                };
                let status =
                    self.call_ioctl(target, id, (&mut list as *mut SConfigList).cast(), null);
                (status, IoctlOut::Nothing)
            }
            Ioctl::GetConfig(parameters) => {
                let mut configs: Vec<SConfig> = parameters
                    .iter()
                    .map(|&parameter| SConfig {
                        parameter,
                        value: 0,
                    })
                    .collect();
                let mut list = SConfigList {
                    num_of_params: configs.len() as u32,
                    config_ptr: configs.as_mut_ptr(),
                };
                let status =
                    self.call_ioctl(target, id, (&mut list as *mut SConfigList).cast(), null);
                let values = configs.iter().map(|c| (c.parameter, c.value)).collect();
                (status, IoctlOut::Config(values))
            }
            Ioctl::ReadVbatt => {
                let mut millivolts = 0u32;
                // READ_VBATT takes no input and writes one unsigned long.
                let status =
                    self.call_ioctl(target, id, null, (&mut millivolts as *mut u32).cast());
                (status, IoctlOut::Value(millivolts))
            }
            Ioctl::FiveBaudInit(address, room) => {
                let mut target_bytes = address.clone();
                let mut input = SByteArray {
                    num_of_bytes: target_bytes.len() as u32,
                    byte_ptr: target_bytes.as_mut_ptr(),
                };
                let mut keys = vec![0u8; (*room).clamp(1, 255) as usize];
                let mut output = SByteArray {
                    num_of_bytes: keys.len() as u32,
                    byte_ptr: keys.as_mut_ptr(),
                };
                // FIVE_BAUD_INIT: the address bytes in, the key bytes out.
                let status = self.call_ioctl(
                    target,
                    id,
                    (&mut input as *mut SByteArray).cast(),
                    (&mut output as *mut SByteArray).cast(),
                );
                keys.truncate((output.num_of_bytes as usize).min(keys.len()));
                (status, IoctlOut::Bytes(keys))
            }
            Ioctl::FastInit(start) => {
                let mut input = PassThruMsg::from(start);
                let mut output = PassThruMsg::zeroed();
                // FAST_INIT: the StartCommunication request in, the answer out.
                let status = self.call_ioctl(
                    target,
                    id,
                    (&mut *input as *mut PassThruMsg).cast(),
                    (&mut *output as *mut PassThruMsg).cast(),
                );
                (status, IoctlOut::Message(output.to_message()))
            }
            Ioctl::ClearTxBuffer | Ioctl::ClearRxBuffer => {
                (self.call_ioctl(target, id, null, null), IoctlOut::Nothing)
            }
        };
        if status == NO_ERROR {
            Ok(out)
        } else {
            Err(status)
        }
    }

    fn last_error(&mut self) -> String {
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

    fn library(&self) -> &str {
        &self.library_path
    }
}
