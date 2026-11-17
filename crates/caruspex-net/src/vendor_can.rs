//! CAN interfaces through the makers' own libraries on Windows, for people
//! without a J2534 driver: PEAK PCAN-Basic (PCANBasic.dll, the PCAN-USB
//! family) and Kvaser CANlib (canlib32.dll, Leaf, U100, Memorator). Both
//! DLLs come with the makers' drivers; Caruspex lists their channels when
//! they are there. Raw frames only, the ISO-TP framing is CanElm's.
//!
//! All unsafe code of these libraries stays in this module: loading the
//! DLL, calling its functions and passing the frame buffers.
#![allow(unsafe_code)]

use std::ffi::{CStr, c_char, c_void};

use caruspex_i18n::l;
use caruspex_link::timing::{now_ms, sleep_ms};
use libloading::Library;

use crate::can_bus::{CanFrame, CanLink, VendorChannel};

/// Looks a function up in a library; `None` if it is missing.
///
/// # Safety
/// `T` must be the function pointer type of the exported function.
unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> Option<T> {
    // SAFETY: the caller names the right type; the pointer is copied out and
    // only used while the library stays loaded (it lives next to it).
    unsafe { library.get::<T>(name).ok().map(|s| *s) }
}

fn load(path: &str) -> Option<Library> {
    // SAFETY: loading the maker's CAN library runs its initialisation, which
    // its installer put there for this.
    unsafe { Library::new(path) }.ok()
}

fn not_complete(path: &str) -> String {
    format!(
        "{}{}",
        l(
            "Die CAN-Bibliothek des Herstellers ist unvollständig: ",
            "The maker's CAN library is incomplete: "
        ),
        path
    )
}

fn not_found(path: &str) -> String {
    format!(
        "{}{}{}",
        l(
            "Die CAN-Bibliothek des Herstellers fehlt: ",
            "The maker's CAN library is missing: "
        ),
        path,
        l(
            ". Ist der Treiber des Herstellers installiert?",
            ". Is the maker's driver installed?"
        )
    )
}

fn unknown_rate() -> String {
    l(
        "Diese Bitrate kennt die Schnittstelle nicht.",
        "The interface does not know this bit rate.",
    )
    .into()
}

/// How long to wait between two looks when a library has no blocking read.
const POLL_MS: u64 = 1;

// ---------------------------------------------------------------------------
// PEAK PCAN-Basic
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct PcanMsg {
    id: u32,
    msg_type: u8,
    len: u8,
    data: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PcanTimestamp {
    millis: u32,
    millis_overflow: u16,
    micros: u16,
}

type PcanInitialize = unsafe extern "system" fn(u16, u16, u8, u32, u16) -> u32;
type PcanUninitialize = unsafe extern "system" fn(u16) -> u32;
type PcanRead = unsafe extern "system" fn(u16, *mut PcanMsg, *mut PcanTimestamp) -> u32;
type PcanWrite = unsafe extern "system" fn(u16, *mut PcanMsg) -> u32;
type PcanGetValue = unsafe extern "system" fn(u16, u8, *mut c_void, u32) -> u32;
type PcanGetErrorText = unsafe extern "system" fn(u32, u16, *mut c_char) -> u32;

const PCAN_ERROR_OK: u32 = 0;
const PCAN_ERROR_QRCVEMPTY: u32 = 0x0020;
const PCAN_CHANNEL_CONDITION: u8 = 0x0D;
const PCAN_CHANNEL_AVAILABLE: u32 = 0x01;
const PCAN_MESSAGE_EXTENDED: u8 = 0x02;
/// RTR, error and status frames and the echo of our own: not for us.
const PCAN_MESSAGE_SKIP: u8 = 0x01 | 0x20 | 0x40 | 0x80;
/// English for CAN_GetErrorText.
const PCAN_ENGLISH: u16 = 0x09;

/// The channel names of PCAN-Basic and their handles: USB1 to USB16, PCI1
/// to PCI16, LAN1 to LAN16.
fn pcan_handles() -> Vec<(String, u16)> {
    let mut out = Vec::new();
    for (kind, low, high) in [
        ("USB", 0x51u16, 0x509u16),
        ("PCI", 0x41, 0x409),
        ("LAN", 0x801, 0x809),
    ] {
        for n in 1..=16u16 {
            // USB and PCI go on at another number after the eighth channel.
            let handle = if kind == "LAN" || n <= 8 {
                low + n - 1
            } else {
                high + n - 9
            };
            out.push((format!("{kind}{n}"), handle));
        }
    }
    out
}

/// BTR0/BTR1 values of PCAN-Basic.
fn pcan_btr(bitrate: u32) -> Option<u16> {
    Some(match bitrate {
        1_000_000 => 0x0014,
        800_000 => 0x0016,
        500_000 => 0x001C,
        250_000 => 0x011C,
        125_000 => 0x031C,
        100_000 => 0x432F,
        95_000 => 0xC34E,
        83_000 | 83_333 => 0x852B,
        50_000 => 0x472F,
        47_000 => 0x1414,
        33_000 | 33_333 => 0x8B2F,
        20_000 => 0x532F,
        10_000 => 0x672F,
        5_000 => 0x7F7F,
        _ => return None,
    })
}

struct Pcan {
    initialize: PcanInitialize,
    uninitialize: PcanUninitialize,
    read: PcanRead,
    write: PcanWrite,
    get_value: PcanGetValue,
    error_text: Option<PcanGetErrorText>,
    _library: Library,
}

impl Pcan {
    fn load(path: &str) -> Result<Pcan, String> {
        let library = load(path).ok_or_else(|| not_found(path))?;
        // SAFETY: the types follow PCANBasic.h.
        let functions = unsafe {
            (
                symbol::<PcanInitialize>(&library, b"CAN_Initialize\0"),
                symbol::<PcanUninitialize>(&library, b"CAN_Uninitialize\0"),
                symbol::<PcanRead>(&library, b"CAN_Read\0"),
                symbol::<PcanWrite>(&library, b"CAN_Write\0"),
                symbol::<PcanGetValue>(&library, b"CAN_GetValue\0"),
                symbol::<PcanGetErrorText>(&library, b"CAN_GetErrorText\0"),
            )
        };
        let (
            Some(initialize),
            Some(uninitialize),
            Some(read),
            Some(write),
            Some(get_value),
            error_text,
        ) = functions
        else {
            return Err(not_complete(path));
        };
        Ok(Pcan {
            initialize,
            uninitialize,
            read,
            write,
            get_value,
            error_text,
            _library: library,
        })
    }

    fn available(&self, handle: u16) -> bool {
        let mut condition = 0u32;
        // SAFETY: PCAN_CHANNEL_CONDITION writes one DWORD into the buffer.
        let status = unsafe {
            (self.get_value)(
                handle,
                PCAN_CHANNEL_CONDITION,
                (&mut condition as *mut u32).cast::<c_void>(),
                4,
            )
        };
        status == PCAN_ERROR_OK && condition & PCAN_CHANNEL_AVAILABLE != 0
    }

    fn error(&self, status: u32) -> String {
        let Some(text) = self.error_text else {
            return format!("0x{status:X}");
        };
        let mut buffer = [0 as c_char; 256];
        // SAFETY: CAN_GetErrorText writes at most 255 characters and the terminator.
        if unsafe { text(status, PCAN_ENGLISH, buffer.as_mut_ptr()) } != PCAN_ERROR_OK {
            return format!("0x{status:X}");
        }
        buffer[255] = 0;
        // SAFETY: terminated above.
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }
}

struct PcanLink {
    pcan: Pcan,
    handle: u16,
    channel: String,
}

impl Drop for PcanLink {
    fn drop(&mut self) {
        // SAFETY: the channel was initialised by this link.
        unsafe { (self.pcan.uninitialize)(self.handle) };
    }
}

impl CanLink for PcanLink {
    fn send(&mut self, frame: &CanFrame) -> bool {
        let size = frame.data.len().min(8);
        let mut m = PcanMsg {
            id: frame.id,
            msg_type: if frame.extended {
                PCAN_MESSAGE_EXTENDED
            } else {
                0
            },
            len: size as u8,
            data: [0; 8],
        };
        m.data[..size].copy_from_slice(&frame.data[..size]);
        // SAFETY: one valid message.
        unsafe { (self.pcan.write)(self.handle, &mut m) == PCAN_ERROR_OK }
    }

    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame> {
        let deadline = now_ms() + timeout_ms;
        loop {
            let mut m = PcanMsg {
                id: 0,
                msg_type: 0,
                len: 0,
                data: [0; 8],
            };
            let mut time = PcanTimestamp {
                millis: 0,
                millis_overflow: 0,
                micros: 0,
            };
            // SAFETY: room for one message and its time stamp.
            let status = unsafe { (self.pcan.read)(self.handle, &mut m, &mut time) };
            if status == PCAN_ERROR_OK {
                if m.msg_type & PCAN_MESSAGE_SKIP != 0 {
                    continue;
                }
                return Some(CanFrame {
                    id: m.id,
                    extended: m.msg_type & PCAN_MESSAGE_EXTENDED != 0,
                    data: m.data[..usize::from(m.len).min(8)].to_vec(),
                });
            }
            if status != PCAN_ERROR_QRCVEMPTY || now_ms() >= deadline {
                return None;
            }
            sleep_ms(POLL_MS);
        }
    }

    fn name(&self) -> String {
        format!("PCAN {}", self.channel)
    }
}

pub(crate) fn pcan_channels(library: &str) -> Vec<VendorChannel> {
    let Ok(pcan) = Pcan::load(library) else {
        return Vec::new();
    };
    pcan_handles()
        .into_iter()
        .filter(|(_, handle)| pcan.available(*handle))
        .map(|(name, _)| VendorChannel {
            label: format!("PCAN · {name}"),
            spec: format!("pcan:{name}"),
        })
        .collect()
}

pub(crate) fn open_pcan(
    library: &str,
    channel: &str,
    bitrate: u32,
) -> Result<Box<dyn CanLink>, String> {
    let pcan = Pcan::load(library)?;
    let wanted = if channel.is_empty() { "USB1" } else { channel };
    let Some(handle) = pcan_handles()
        .into_iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(wanted))
        .map(|(_, h)| h)
    else {
        return Err(format!(
            "{}{}",
            l("Unbekannter PCAN-Kanal: ", "Unknown PCAN channel: "),
            channel
        ));
    };
    let btr = pcan_btr(bitrate).ok_or_else(unknown_rate)?;
    // SAFETY: plug and play hardware takes no type, port or interrupt.
    let status = unsafe { (pcan.initialize)(handle, btr, 0, 0, 0) };
    if status != PCAN_ERROR_OK {
        return Err(format!(
            "{}{}",
            l(
                "Der PCAN-Kanal lässt sich nicht öffnen: ",
                "The PCAN channel cannot be opened: "
            ),
            pcan.error(status)
        ));
    }
    Ok(Box::new(PcanLink {
        pcan,
        handle,
        channel: wanted.to_ascii_uppercase(),
    }))
}

// ---------------------------------------------------------------------------
// Kvaser CANlib
// ---------------------------------------------------------------------------

type KvInitializeLibrary = unsafe extern "system" fn();
type KvOpenChannel = unsafe extern "system" fn(i32, i32) -> i32;
type KvSetBusParams = unsafe extern "system" fn(i32, i32, u32, u32, u32, u32, u32) -> i32;
type KvHandle = unsafe extern "system" fn(i32) -> i32;
type KvWrite = unsafe extern "system" fn(i32, i32, *mut c_void, u32, u32) -> i32;
type KvReadWait =
    unsafe extern "system" fn(i32, *mut i32, *mut c_void, *mut u32, *mut u32, *mut u32, u32) -> i32;
type KvNumberOfChannels = unsafe extern "system" fn(*mut i32) -> i32;
type KvChannelData = unsafe extern "system" fn(i32, i32, *mut c_void, usize) -> i32;
type KvErrorText = unsafe extern "system" fn(i32, *mut c_char, u32) -> i32;

const CAN_OK: i32 = 0;
const CAN_OPEN_ACCEPT_VIRTUAL: i32 = 0x0020;
const CAN_MSG_RTR: u32 = 0x0001;
const CAN_MSG_STD: u32 = 0x0002;
const CAN_MSG_EXT: u32 = 0x0004;
const CAN_MSG_ERROR_FRAME: u32 = 0x0020;
const CAN_MSG_TXACK: u32 = 0x0040;
const CAN_MSG_TXRQ: u32 = 0x0080;
const CANCHANNELDATA_CHANNEL_CAP: i32 = 1;
const CANCHANNELDATA_CHAN_NO_ON_CARD: i32 = 9;
const CANCHANNELDATA_DEVDESCR_ASCII: i32 = 26;
const CAN_CHANNEL_CAP_VIRTUAL: u32 = 0x0001_0000;

/// The predefined bit rates of canSetBusParams (canBITRATE_...).
fn kvaser_rate(bitrate: u32) -> Option<i32> {
    Some(match bitrate {
        1_000_000 => -1,
        500_000 => -2,
        250_000 => -3,
        125_000 => -4,
        100_000 => -5,
        62_000 | 62_500 => -6,
        50_000 => -7,
        83_000 | 83_333 => -8,
        10_000 => -9,
        _ => return None,
    })
}

struct Kvaser {
    open_channel: KvOpenChannel,
    set_bus_params: KvSetBusParams,
    bus_on: KvHandle,
    bus_off: KvHandle,
    close: KvHandle,
    write: KvWrite,
    read_wait: KvReadWait,
    number_of_channels: KvNumberOfChannels,
    channel_data: KvChannelData,
    error_text: Option<KvErrorText>,
    _library: Library,
}

impl Kvaser {
    fn load(path: &str) -> Result<Kvaser, String> {
        let library = load(path).ok_or_else(|| not_found(path))?;
        // SAFETY: the types follow canlib.h (CANLIBAPI is __stdcall).
        let (init, functions) = unsafe {
            (
                symbol::<KvInitializeLibrary>(&library, b"canInitializeLibrary\0"),
                (
                    symbol::<KvOpenChannel>(&library, b"canOpenChannel\0"),
                    symbol::<KvSetBusParams>(&library, b"canSetBusParams\0"),
                    symbol::<KvHandle>(&library, b"canBusOn\0"),
                    symbol::<KvHandle>(&library, b"canBusOff\0"),
                    symbol::<KvHandle>(&library, b"canClose\0"),
                    symbol::<KvWrite>(&library, b"canWrite\0"),
                    symbol::<KvReadWait>(&library, b"canReadWait\0"),
                    symbol::<KvNumberOfChannels>(&library, b"canGetNumberOfChannels\0"),
                    symbol::<KvChannelData>(&library, b"canGetChannelData\0"),
                    symbol::<KvErrorText>(&library, b"canGetErrorText\0"),
                ),
            )
        };
        let (
            Some(init),
            (
                Some(open_channel),
                Some(set_bus_params),
                Some(bus_on),
                Some(bus_off),
                Some(close),
                Some(write),
                Some(read_wait),
                Some(number_of_channels),
                Some(channel_data),
                error_text,
            ),
        ) = (init, functions)
        else {
            return Err(not_complete(path));
        };
        // SAFETY: must come before every other call; calling it again is harmless.
        unsafe { init() };
        Ok(Kvaser {
            open_channel,
            set_bus_params,
            bus_on,
            bus_off,
            close,
            write,
            read_wait,
            number_of_channels,
            channel_data,
            error_text,
            _library: library,
        })
    }

    fn error(&self, status: i32) -> String {
        let Some(text) = self.error_text else {
            return status.to_string();
        };
        let mut buffer = [0 as c_char; 256];
        // SAFETY: the buffer size is passed along.
        if unsafe { text(status, buffer.as_mut_ptr(), 255) } != CAN_OK {
            return status.to_string();
        }
        buffer[255] = 0;
        // SAFETY: terminated above.
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }

    fn data_u32(&self, channel: i32, item: i32) -> Option<u32> {
        let mut value = 0u32;
        // SAFETY: the buffer size is passed along.
        let status =
            unsafe { (self.channel_data)(channel, item, (&mut value as *mut u32).cast(), 4) };
        (status == CAN_OK).then_some(value)
    }

    fn description(&self, channel: i32) -> String {
        let mut buffer = [0u8; 256];
        // SAFETY: the buffer size is passed along.
        let status = unsafe {
            (self.channel_data)(
                channel,
                CANCHANNELDATA_DEVDESCR_ASCII,
                buffer.as_mut_ptr().cast(),
                buffer.len() - 1,
            )
        };
        if status != CAN_OK {
            return "Kvaser".into();
        }
        let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        String::from_utf8_lossy(&buffer[..end]).trim().to_string()
    }
}

struct KvaserLink {
    kvaser: Kvaser,
    handle: i32,
    channel: i32,
}

impl Drop for KvaserLink {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by this link.
        unsafe {
            (self.kvaser.bus_off)(self.handle);
            (self.kvaser.close)(self.handle);
        }
    }
}

impl CanLink for KvaserLink {
    fn send(&mut self, frame: &CanFrame) -> bool {
        let mut data = [0u8; 8];
        let size = frame.data.len().min(8);
        data[..size].copy_from_slice(&frame.data[..size]);
        let flag = if frame.extended {
            CAN_MSG_EXT
        } else {
            CAN_MSG_STD
        };
        // SAFETY: `size` bytes of data in a valid buffer.
        let status = unsafe {
            (self.kvaser.write)(
                self.handle,
                frame.id as i32,
                data.as_mut_ptr().cast(),
                size as u32,
                flag,
            )
        };
        status == CAN_OK
    }

    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame> {
        let deadline = now_ms() + timeout_ms;
        loop {
            let mut id = 0i32;
            let mut data = [0u8; 64];
            let mut dlc = 0u32;
            let mut flags = 0u32;
            let mut time = 0u32;
            let wait = u32::try_from(deadline.saturating_sub(now_ms())).unwrap_or(u32::MAX);
            // SAFETY: room for the largest frame, every out value valid.
            let status = unsafe {
                (self.kvaser.read_wait)(
                    self.handle,
                    &mut id,
                    data.as_mut_ptr().cast(),
                    &mut dlc,
                    &mut flags,
                    &mut time,
                    wait,
                )
            };
            if status != CAN_OK {
                return None; // canERR_NOMSG: nothing within the time
            }
            if flags & (CAN_MSG_RTR | CAN_MSG_ERROR_FRAME | CAN_MSG_TXACK | CAN_MSG_TXRQ) != 0 {
                if now_ms() >= deadline {
                    return None;
                }
                continue;
            }
            return Some(CanFrame {
                id: id as u32,
                extended: flags & CAN_MSG_EXT != 0,
                data: data[..(dlc as usize).min(8)].to_vec(),
            });
        }
    }

    fn name(&self) -> String {
        format!(
            "{} ({}{})",
            self.kvaser.description(self.channel),
            l("Kanal ", "channel "),
            self.channel
        )
    }
}

pub(crate) fn kvaser_channels(library: &str) -> Vec<VendorChannel> {
    let Ok(kvaser) = Kvaser::load(library) else {
        return Vec::new();
    };
    let mut count = 0i32;
    // SAFETY: writes one int.
    if unsafe { (kvaser.number_of_channels)(&mut count) } != CAN_OK {
        return Vec::new();
    }
    (0..count.clamp(0, 64))
        .filter(|&c| {
            // The virtual channels the driver brings along are no hardware.
            kvaser
                .data_u32(c, CANCHANNELDATA_CHANNEL_CAP)
                .is_some_and(|cap| cap & CAN_CHANNEL_CAP_VIRTUAL == 0)
        })
        .map(|c| {
            let on_card = kvaser
                .data_u32(c, CANCHANNELDATA_CHAN_NO_ON_CARD)
                .unwrap_or(0);
            VendorChannel {
                label: format!(
                    "Kvaser · {} {}{}",
                    kvaser.description(c),
                    l("Kanal ", "channel "),
                    on_card + 1
                ),
                spec: format!("kvaser:{c}"),
            }
        })
        .collect()
}

pub(crate) fn open_kvaser(
    library: &str,
    channel: &str,
    bitrate: u32,
) -> Result<Box<dyn CanLink>, String> {
    let kvaser = Kvaser::load(library)?;
    let number: i32 = if channel.is_empty() {
        0
    } else {
        channel.parse().map_err(|_| {
            format!(
                "{}{}",
                l("Unbekannter Kvaser-Kanal: ", "Unknown Kvaser channel: "),
                channel
            )
        })?
    };
    let rate = kvaser_rate(bitrate).ok_or_else(unknown_rate)?;
    let failed = |kvaser: &Kvaser, status: i32| {
        format!(
            "{}{}",
            l(
                "Der Kvaser-Kanal lässt sich nicht öffnen: ",
                "The Kvaser channel cannot be opened: "
            ),
            kvaser.error(status)
        )
    };
    // SAFETY: plain calls with numbers.
    let handle = unsafe { (kvaser.open_channel)(number, CAN_OPEN_ACCEPT_VIRTUAL) };
    if handle < 0 {
        return Err(failed(&kvaser, handle));
    }
    // SAFETY: a valid handle; the predefined rate needs no timing values.
    let status = unsafe { (kvaser.set_bus_params)(handle, rate, 0, 0, 0, 0, 0) };
    // SAFETY: a valid handle.
    let status = if status == CAN_OK {
        unsafe { (kvaser.bus_on)(handle) }
    } else {
        status
    };
    if status != CAN_OK {
        // SAFETY: the handle is closed once, here.
        unsafe { (kvaser.close)(handle) };
        return Err(failed(&kvaser, status));
    }
    Ok(Box::new(KvaserLink {
        kvaser,
        handle,
        channel: number,
    }))
}
