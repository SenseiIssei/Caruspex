//! Classic Bluetooth adapters over an RFCOMM serial port (the serial port
//! profile), without a COM port or an `rfcomm bind` in the system; the
//! RfcommTransport and the classic part of scanBleDevices in the C++
//! version, which had both from QtBluetooth.
//!
//! - Windows: devices from the Bluetooth API (paired, remembered and, with
//!   an inquiry, the ones in range), the connection is a Winsock
//!   AF_BTH/BTHPROTO_RFCOMM socket to the serial port service; Windows looks
//!   the channel up itself.
//! - Linux: the paired devices from BlueZ (bluetoothctl), the connection an
//!   AF_BLUETOOTH/BTPROTO_RFCOMM socket. The channel comes from an SDP query
//!   for the serial port service; if that fails, channel 1, which nearly
//!   every ELM327 clone uses.
//! - Elsewhere (macOS): not supported, with a text saying so. A paired
//!   adapter is a serial port there and goes through the cable mode.
//!
//! The adapter has to be paired once in the system settings (PIN usually
//! 1234 or 0000). This is the only module of the crate with `unsafe`, all of
//! it in the platform parts below.
#![allow(unsafe_code)]

use std::io;

use caruspex_i18n::l;

use crate::ble_profile::obd_adapter_name_score;
use crate::bluetooth::BleDevice;
pub use crate::bluetooth::{format_address, parse_address};
use crate::transport::Transport;

/// The serial port profile, 00001101-0000-1000-8000-00805F9B34FB. The
/// adapters offer nothing else.
pub const SPP_UUID: u128 = 0x0000_1101_0000_1000_8000_0080_5F9B_34FB;

/// How long a connection attempt may take, like the C++ version.
pub const CONNECT_TIMEOUT_MS: u64 = 20000;

/// Most a single read collects once the first bytes are there.
const READ_LIMIT: usize = 64 * 1024;

pub use os::Socket as SystemSocket;

/// The socket under [`RfcommTransport`], so the transport logic can be
/// tested without a radio.
pub trait RfcommSocket: Send {
    /// Sends some of `data` and says how much. `Err` when the connection is
    /// gone.
    fn send(&mut self, data: &[u8]) -> io::Result<usize>;
    /// Waits up to `timeout_ms` for bytes, appends what is there and says
    /// how much; 0 when nothing came in time. `Err` when the connection is
    /// closed or broken.
    fn receive(&mut self, out: &mut Vec<u8>, timeout_ms: u64) -> io::Result<usize>;
}

/// The text when this system has no direct classic Bluetooth.
pub fn not_supported() -> String {
    l(
        "Klassisches Bluetooth direkt wird auf diesem System nicht unterstützt. Einen gekoppelten Adapter über seinen seriellen Anschluss (Kabel) verbinden.",
        "Direct classic Bluetooth is not supported on this system. Connect a paired adapter through its serial port (cable) instead.",
    )
    .to_string()
}

/// The classic devices the system knows or sees within about `milliseconds`,
/// unsorted. Errors only when classic Bluetooth is not supported here; a
/// computer without a radio simply has none.
pub fn classic_devices(milliseconds: u64) -> Result<Vec<BleDevice>, String> {
    os::classic_devices(milliseconds)
}

#[cfg_attr(not(any(windows, target_os = "linux")), allow(dead_code))]
fn classic_device(address: u64, name: &str) -> BleDevice {
    let name = name.trim();
    // Without a name the system shows the address; that is no name.
    let name = if parse_address(name) == Some(address) {
        ""
    } else {
        name
    };
    BleDevice {
        id: format_address(address),
        name: name.to_string(),
        rssi: 0,
        score: obd_adapter_name_score(name),
        low_energy: false,
        classic: true,
    }
}

/// Classic Bluetooth adapter over an RFCOMM serial port.
pub struct RfcommTransport<S: RfcommSocket = SystemSocket> {
    socket: S,
    name: String,
    disconnected: bool,
}

impl RfcommTransport<SystemSocket> {
    /// Connects to the serial port service of the device (its id must hold
    /// the address). Takes up to [`CONNECT_TIMEOUT_MS`].
    pub fn open(device: &BleDevice) -> Result<Self, String> {
        if !os::SUPPORTED {
            return Err(not_supported());
        }
        open_with(device, os::connect)
    }
}

/// [`RfcommTransport::open`] with the connection made by `connect`, which
/// gets the address and returns the socket or the system's error text.
pub fn open_with<S: RfcommSocket>(
    device: &BleDevice,
    connect: impl FnOnce(u64) -> Result<S, String>,
) -> Result<RfcommTransport<S>, String> {
    let shown = if device.name.is_empty() {
        &device.id
    } else {
        &device.name
    };
    let Some(address) = parse_address(&device.id) else {
        return Err(l(
            "Klassisches Bluetooth braucht die Adresse des Geräts.",
            "Classic Bluetooth needs the address of the device.",
        )
        .to_string());
    };
    match connect(address) {
        Ok(socket) => Ok(RfcommTransport::with_socket(
            socket,
            format!("Bluetooth {shown}"),
        )),
        Err(last_error) => {
            let mut text = l(
                "Verbindung zu %1 kam nicht zustande. Ist der Adapter im System gekoppelt (PIN meist 1234 oder 0000)?",
                "Could not connect to %1. Is the adapter paired in the system (PIN usually 1234 or 0000)?",
            )
            .replace("%1", shown);
            if !last_error.is_empty() {
                text.push_str(" (");
                text.push_str(&last_error);
                text.push(')');
            }
            Err(text)
        }
    }
}

impl<S: RfcommSocket> RfcommTransport<S> {
    pub fn with_socket(socket: S, name: String) -> Self {
        Self {
            socket,
            name,
            disconnected: false,
        }
    }

    /// The adapter closed the connection or it broke.
    pub fn is_disconnected(&self) -> bool {
        self.disconnected
    }

    fn pull(&mut self, out: &mut Vec<u8>, timeout_ms: u64) -> usize {
        if self.disconnected {
            return 0;
        }
        match self.socket.receive(out, timeout_ms) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => 0,
            Err(_) => {
                self.disconnected = true;
                0
            }
        }
    }
}

impl<S: RfcommSocket> Transport for RfcommTransport<S> {
    fn write(&mut self, data: &[u8]) -> bool {
        if self.disconnected {
            return false;
        }
        let mut rest = data;
        while !rest.is_empty() {
            match self.socket.send(rest) {
                Ok(0) => return false,
                Ok(n) => rest = &rest[n.min(rest.len())..],
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => {
                    self.disconnected = true;
                    return false;
                }
            }
        }
        true
    }

    /// Waits for the first bytes, then takes whatever else is already there.
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let mut out = Vec::new();
        if self.pull(&mut out, timeout_ms) > 0 {
            while out.len() < READ_LIMIT && self.pull(&mut out, 0) > 0 {}
        }
        out
    }

    fn flush_input(&mut self) {
        let mut dropped = Vec::new();
        while self.pull(&mut dropped, 0) > 0 && dropped.len() < READ_LIMIT {}
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

// --- SDP: which RFCOMM channel the serial port service has -----------------

/// The few bytes of the Service Discovery Protocol needed to find the
/// channel: one ServiceSearchAttributeRequest for the serial port class,
/// asking only for the protocol descriptor list.
#[cfg(any(target_os = "linux", test))]
mod sdp {
    const RFCOMM: u32 = 0x0003;
    /// The Bluetooth base UUID after the first four bytes.
    const BASE_TAIL: [u8; 12] = [
        0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0x80, 0x5F, 0x9B, 0x34, 0xFB,
    ];
    const MAX_DEPTH: usize = 16;

    pub fn request(transaction: u16, continuation: &[u8]) -> Vec<u8> {
        let mut params = vec![
            0x35, 0x03, 0x19, 0x11, 0x01, // search pattern: the serial port class
            0x04, 0x00, // up to 1024 bytes of attributes per answer
            0x35, 0x03, 0x09, 0x00, 0x04, // attribute 0x0004, protocol descriptor list
        ];
        params.push(continuation.len().min(16) as u8);
        params.extend_from_slice(&continuation[..continuation.len().min(16)]);
        let mut pdu = vec![0x06];
        pdu.extend_from_slice(&transaction.to_be_bytes());
        pdu.extend_from_slice(&(params.len() as u16).to_be_bytes());
        pdu.extend_from_slice(&params);
        pdu
    }

    /// The attribute bytes of one answer and the continuation state (empty
    /// once the answer is complete). `None` for an error answer or garbage.
    pub fn response(pdu: &[u8], transaction: u16) -> Option<(Vec<u8>, Vec<u8>)> {
        if pdu.len() < 5 || pdu[0] != 0x07 || pdu[1..3] != transaction.to_be_bytes() {
            return None;
        }
        let length = usize::from(u16::from_be_bytes([pdu[3], pdu[4]]));
        let params = pdu.get(5..5 + length)?;
        let count = usize::from(u16::from_be_bytes([*params.first()?, *params.get(1)?]));
        let lists = params.get(2..2 + count)?;
        let state_length = usize::from(*params.get(2 + count)?);
        let state = params.get(3 + count..3 + count + state_length)?;
        Some((lists.to_vec(), state.to_vec()))
    }

    enum Element {
        Uint(u64),
        /// Short form of a UUID on the base UUID; `None` for any other.
        Uuid(Option<u32>),
        Seq(Vec<Element>),
        Other,
    }

    fn element(data: &[u8], depth: usize) -> Option<(Element, usize)> {
        let head = *data.first()?;
        let kind = head >> 3;
        let (length, start): (usize, usize) = match head & 7 {
            0 => (usize::from(kind != 0), 1),
            1 => (2, 1),
            2 => (4, 1),
            3 => (8, 1),
            4 => (16, 1),
            5 => (usize::from(*data.get(1)?), 2),
            6 => (
                usize::from(u16::from_be_bytes([*data.get(1)?, *data.get(2)?])),
                3,
            ),
            _ => {
                let bytes = data.get(1..5)?;
                let n = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                (usize::try_from(n).ok()?, 5)
            }
        };
        let body = data.get(start..start.checked_add(length)?)?;
        let value = match kind {
            1 if length <= 8 => {
                Element::Uint(body.iter().fold(0u64, |acc, &b| (acc << 8) | u64::from(b)))
            }
            3 => Element::Uuid(match length {
                2 => Some(u32::from(u16::from_be_bytes([body[0], body[1]]))),
                4 => Some(u32::from_be_bytes([body[0], body[1], body[2], body[3]])),
                16 if body[4..] == BASE_TAIL => {
                    Some(u32::from_be_bytes([body[0], body[1], body[2], body[3]]))
                }
                _ => None,
            }),
            6 | 7 if depth < MAX_DEPTH => {
                let mut items = Vec::new();
                let mut at = 0;
                while at < body.len() {
                    let (item, used) = element(&body[at..], depth + 1)?;
                    items.push(item);
                    at += used;
                }
                Element::Seq(items)
            }
            _ => Element::Other,
        };
        Some((value, start + length))
    }

    fn find(e: &Element) -> Option<u8> {
        let Element::Seq(items) = e else {
            return None;
        };
        if let [Element::Uuid(Some(RFCOMM)), Element::Uint(channel), ..] = items.as_slice()
            && (1..=30).contains(channel)
        {
            return Some(*channel as u8);
        }
        items.iter().find_map(find)
    }

    /// The RFCOMM channel in the collected attribute lists.
    pub fn rfcomm_channel(lists: &[u8]) -> Option<u8> {
        let (root, _) = element(lists, 0)?;
        find(&root)
    }
}

// --- BlueZ device list (bluetoothctl output) ---------------------------------

#[cfg(any(target_os = "linux", test))]
mod bluez {
    use super::parse_address;

    /// Drops colour codes and the readline markers bluetoothctl may print.
    fn plain(line: &str) -> String {
        let mut out = String::with_capacity(line.len());
        let mut chars = line.chars();
        while let Some(c) = chars.next() {
            match c {
                '\u{1b}' => {
                    for c in chars.by_ref() {
                        if c.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                '\u{1}' | '\u{2}' => {}
                c => out.push(c),
            }
        }
        out
    }

    /// "Device AA:BB:CC:DD:EE:FF Name" lines of `bluetoothctl devices`.
    pub fn device_lines(text: &str) -> Vec<(u64, String)> {
        text.lines()
            .filter_map(|line| {
                let line = plain(line);
                let rest = &line[line.find("Device ")? + 7..];
                let (address, name) = rest.split_once(' ').unwrap_or((rest, ""));
                Some((parse_address(address)?, name.trim().to_string()))
            })
            .collect()
    }

    /// Whether `bluetoothctl info` describes a classic device: it has a
    /// class of device or offers the serial port.
    pub fn info_is_classic(info: &str) -> bool {
        let lower = info.to_ascii_lowercase();
        lower.contains("class: 0x") || lower.contains("00001101-0000-1000-8000-00805f9b34fb")
    }
}

// --- Windows ----------------------------------------------------------------

#[cfg(windows)]
mod os {
    use std::io;
    use std::sync::Once;
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Devices::Bluetooth::{
        AF_BTH, BLUETOOTH_DEVICE_INFO, BLUETOOTH_DEVICE_SEARCH_PARAMS, BLUETOOTH_FIND_RADIO_PARAMS,
        BTHPROTO_RFCOMM, BluetoothFindDeviceClose, BluetoothFindFirstDevice,
        BluetoothFindFirstRadio, BluetoothFindNextDevice, BluetoothFindRadioClose, SOCKADDR_BTH,
    };
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Networking::WinSock::{
        FD_SET, FIONBIO, INVALID_SOCKET, SO_ERROR, SOCK_STREAM, SOCKADDR, SOCKET, SOCKET_ERROR,
        SOL_SOCKET, TIMEVAL, WSADATA, WSAEWOULDBLOCK, WSAGetLastError, WSAStartup, closesocket,
        connect as wsa_connect, getsockopt, ioctlsocket, recv, select, send,
    };
    use windows_sys::core::GUID;

    use super::{BleDevice, CONNECT_TIMEOUT_MS, RfcommSocket, SPP_UUID, classic_device};

    pub const SUPPORTED: bool = true;

    /// An open RFCOMM socket, closed on drop.
    pub struct Socket(SOCKET);

    impl Drop for Socket {
        fn drop(&mut self) {
            // SAFETY: the socket came from socket() and is closed only here.
            unsafe { closesocket(self.0) };
        }
    }

    fn startup() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            // SAFETY: WSADATA is plain data that WSAStartup fills in.
            unsafe {
                let mut data: WSADATA = std::mem::zeroed();
                WSAStartup(0x0202, &mut data);
            }
        });
    }

    fn last_error() -> io::Error {
        // SAFETY: no arguments, reads the thread's last Winsock error.
        io::Error::from_raw_os_error(unsafe { WSAGetLastError() })
    }

    /// Waits until the socket can be read (or written), or failed.
    fn wait(socket: SOCKET, write: bool, timeout_ms: u64) -> io::Result<bool> {
        let mut ready = FD_SET {
            fd_count: 1,
            fd_array: [0; 64],
        };
        ready.fd_array[0] = socket;
        let mut failed = ready;
        let timeout = TIMEVAL {
            tv_sec: (timeout_ms / 1000).min(i32::MAX as u64) as i32,
            tv_usec: ((timeout_ms % 1000) * 1000) as i32,
        };
        let (read_set, write_set) = if write {
            (std::ptr::null_mut(), &mut ready as *mut FD_SET)
        } else {
            (&mut ready as *mut FD_SET, std::ptr::null_mut())
        };
        // SAFETY: the sets and the timeout live until select returns.
        let n = unsafe { select(0, read_set, write_set, &mut failed, &timeout) };
        if n == SOCKET_ERROR {
            return Err(last_error());
        }
        Ok(n > 0)
    }

    fn socket_error(socket: SOCKET) -> i32 {
        let mut value: i32 = 0;
        let mut length = std::mem::size_of::<i32>() as i32;
        // SAFETY: value and length are valid for the size given.
        unsafe {
            getsockopt(
                socket,
                SOL_SOCKET,
                SO_ERROR,
                (&mut value as *mut i32).cast(),
                &mut length,
            )
        };
        value
    }

    pub fn connect(address: u64) -> Result<Socket, String> {
        startup();
        // SAFETY: plain socket creation; the handle goes into Socket.
        let raw = unsafe {
            windows_sys::Win32::Networking::WinSock::socket(
                i32::from(AF_BTH),
                SOCK_STREAM,
                BTHPROTO_RFCOMM as i32,
            )
        };
        if raw == INVALID_SOCKET {
            return Err(last_error().to_string());
        }
        let socket = Socket(raw);
        // Non-blocking, so the attempt can be given up after the timeout.
        let mut on: u32 = 1;
        // SAFETY: on is a valid u32 for FIONBIO.
        if unsafe { ioctlsocket(raw, FIONBIO, &mut on) } != 0 {
            return Err(last_error().to_string());
        }
        // Port 0 with the service class: Windows looks the channel up by SDP.
        let target = SOCKADDR_BTH {
            addressFamily: AF_BTH,
            btAddr: address,
            serviceClassId: GUID::from_u128(SPP_UUID),
            port: 0,
        };
        // SAFETY: target is a complete SOCKADDR_BTH of the length given.
        let rc = unsafe {
            wsa_connect(
                raw,
                (&target as *const SOCKADDR_BTH).cast::<SOCKADDR>(),
                std::mem::size_of::<SOCKADDR_BTH>() as i32,
            )
        };
        if rc != 0 {
            let error = last_error();
            if error.raw_os_error() != Some(WSAEWOULDBLOCK) {
                return Err(error.to_string());
            }
            if !wait(raw, true, CONNECT_TIMEOUT_MS).map_err(|e| e.to_string())? {
                return Err(io::Error::from(io::ErrorKind::TimedOut).to_string());
            }
            let code = socket_error(raw);
            if code != 0 {
                return Err(io::Error::from_raw_os_error(code).to_string());
            }
        }
        Ok(socket)
    }

    impl RfcommSocket for Socket {
        fn send(&mut self, data: &[u8]) -> io::Result<usize> {
            let length = data.len().min(i32::MAX as usize) as i32;
            let deadline = Instant::now() + Duration::from_millis(2000);
            loop {
                // SAFETY: data is valid for length bytes.
                let n = unsafe { send(self.0, data.as_ptr(), length, 0) };
                if n >= 0 {
                    return Ok(n as usize);
                }
                let error = last_error();
                if error.raw_os_error() != Some(WSAEWOULDBLOCK) {
                    return Err(error);
                }
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() || !wait(self.0, true, left.as_millis() as u64)? {
                    return Err(io::Error::from(io::ErrorKind::TimedOut));
                }
            }
        }

        fn receive(&mut self, out: &mut Vec<u8>, timeout_ms: u64) -> io::Result<usize> {
            if !wait(self.0, false, timeout_ms)? {
                return Ok(0);
            }
            let mut buffer = [0u8; 1024];
            // SAFETY: buffer is valid for its length.
            let n = unsafe { recv(self.0, buffer.as_mut_ptr(), buffer.len() as i32, 0) };
            if n == 0 {
                return Err(io::Error::from(io::ErrorKind::ConnectionAborted));
            }
            if n < 0 {
                let error = last_error();
                if error.raw_os_error() == Some(WSAEWOULDBLOCK) {
                    return Ok(0);
                }
                return Err(error);
            }
            out.extend_from_slice(&buffer[..n as usize]);
            Ok(n as usize)
        }
    }

    fn has_radio() -> bool {
        let params = BLUETOOTH_FIND_RADIO_PARAMS {
            dwSize: std::mem::size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
        };
        let mut radio: HANDLE = std::ptr::null_mut();
        // SAFETY: params and radio are valid; both handles are closed again.
        unsafe {
            let find = BluetoothFindFirstRadio(&params, &mut radio);
            if find.is_null() {
                return false;
            }
            CloseHandle(radio);
            BluetoothFindRadioClose(find);
        }
        true
    }

    fn device_of(info: &BLUETOOTH_DEVICE_INFO) -> BleDevice {
        let length = info
            .szName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(info.szName.len());
        let name = String::from_utf16_lossy(&info.szName[..length]);
        // SAFETY: both union members are plain bytes of the same address.
        let address = unsafe { info.Address.Anonymous.ullLong } & 0xFFFF_FFFF_FFFF;
        classic_device(address, &name)
    }

    fn empty_info() -> BLUETOOTH_DEVICE_INFO {
        BLUETOOTH_DEVICE_INFO {
            dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
            ..BLUETOOTH_DEVICE_INFO::default()
        }
    }

    pub fn classic_devices(milliseconds: u64) -> Result<Vec<BleDevice>, String> {
        if !has_radio() {
            return Ok(Vec::new());
        }
        // The inquiry runs in steps of 1.28 seconds.
        let multiplier = milliseconds.div_ceil(1280).clamp(1, 48) as u8;
        let params = BLUETOOTH_DEVICE_SEARCH_PARAMS {
            dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
            fReturnAuthenticated: 1,
            fReturnRemembered: 1,
            fReturnUnknown: 1,
            fReturnConnected: 1,
            fIssueInquiry: 1,
            cTimeoutMultiplier: multiplier,
            hRadio: std::ptr::null_mut(),
        };
        let mut found = Vec::new();
        let mut info = empty_info();
        // SAFETY: params and info are valid with their sizes set; the find
        // handle is closed at the end.
        unsafe {
            let find = BluetoothFindFirstDevice(&params, &mut info);
            if find.is_null() {
                return Ok(found);
            }
            loop {
                let device = device_of(&info);
                if !found.iter().any(|d: &BleDevice| d.id == device.id) {
                    found.push(device);
                }
                info = empty_info();
                if BluetoothFindNextDevice(find, &mut info) == 0 {
                    break;
                }
            }
            BluetoothFindDeviceClose(find);
        }
        Ok(found)
    }
}

// --- Linux ------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod os {
    use std::io::{self, Read};
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use super::{BleDevice, CONNECT_TIMEOUT_MS, RfcommSocket, bluez, classic_device, sdp};
    use crate::bluetooth::format_address;

    pub const SUPPORTED: bool = true;

    const BTPROTO_L2CAP: libc::c_int = 0;
    const BTPROTO_RFCOMM: libc::c_int = 3;
    const SDP_PSM: u16 = 1;
    const SDP_TIMEOUT_MS: u64 = 5000;
    /// Most ELM327 clones put the serial port there.
    const DEFAULT_CHANNEL: u8 = 1;

    #[repr(C)]
    struct SockaddrRc {
        rc_family: libc::sa_family_t,
        rc_bdaddr: [u8; 6],
        rc_channel: u8,
    }

    #[repr(C)]
    struct SockaddrL2 {
        l2_family: libc::sa_family_t,
        l2_psm: u16,
        l2_bdaddr: [u8; 6],
        l2_cid: u16,
        l2_bdaddr_type: u8,
    }

    /// An open RFCOMM socket, closed on drop.
    pub struct Socket(OwnedFd);

    /// BlueZ keeps the address bytes in reverse order.
    fn bdaddr(address: u64) -> [u8; 6] {
        let b = address.to_le_bytes();
        [b[0], b[1], b[2], b[3], b[4], b[5]]
    }

    fn open_socket(kind: libc::c_int, protocol: libc::c_int) -> io::Result<OwnedFd> {
        // SAFETY: plain socket creation; the descriptor goes into an OwnedFd.
        let fd = unsafe {
            libc::socket(
                libc::AF_BLUETOOTH,
                kind | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                protocol,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fd is a fresh descriptor nobody else owns.
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    /// Waits for `events` on the descriptor; false on timeout.
    fn poll(fd: &OwnedFd, events: libc::c_short, timeout_ms: u64) -> io::Result<bool> {
        let mut entry = libc::pollfd {
            fd: fd.as_raw_fd(),
            events,
            revents: 0,
        };
        let timeout = timeout_ms.min(i32::MAX as u64) as libc::c_int;
        // SAFETY: one valid pollfd.
        let n = unsafe { libc::poll(&mut entry, 1, timeout) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(n > 0)
    }

    fn connect_to<T>(fd: &OwnedFd, address: &T, timeout_ms: u64) -> io::Result<()> {
        // SAFETY: address is a complete socket address of the size given.
        let rc = unsafe {
            libc::connect(
                fd.as_raw_fd(),
                (address as *const T).cast::<libc::sockaddr>(),
                std::mem::size_of::<T>() as libc::socklen_t,
            )
        };
        if rc == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(error);
        }
        if !poll(fd, libc::POLLOUT, timeout_ms)? {
            return Err(io::Error::from(io::ErrorKind::TimedOut));
        }
        let mut value: libc::c_int = 0;
        let mut length = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
        // SAFETY: value and length are valid for the size given.
        unsafe {
            libc::getsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                (&mut value as *mut libc::c_int).cast(),
                &mut length,
            )
        };
        if value != 0 {
            return Err(io::Error::from_raw_os_error(value));
        }
        Ok(())
    }

    /// Asks the device's SDP server which RFCOMM channel its serial port
    /// service listens on.
    fn sdp_channel(address: u64) -> Option<u8> {
        let fd = open_socket(libc::SOCK_SEQPACKET, BTPROTO_L2CAP).ok()?;
        let target = SockaddrL2 {
            l2_family: libc::AF_BLUETOOTH as libc::sa_family_t,
            l2_psm: SDP_PSM.to_le(),
            l2_bdaddr: bdaddr(address),
            l2_cid: 0,
            l2_bdaddr_type: 0, // BDADDR_BREDR
        };
        connect_to(&fd, &target, SDP_TIMEOUT_MS).ok()?;
        let mut lists = Vec::new();
        let mut state = Vec::new();
        for transaction in 1..=16u16 {
            let request = sdp::request(transaction, &state);
            // SAFETY: request is valid for its length.
            let sent = unsafe {
                libc::send(
                    fd.as_raw_fd(),
                    request.as_ptr().cast(),
                    request.len(),
                    libc::MSG_NOSIGNAL,
                )
            };
            if sent != request.len() as isize || !poll(&fd, libc::POLLIN, SDP_TIMEOUT_MS).ok()? {
                return None;
            }
            let mut buffer = [0u8; 2048];
            // SAFETY: buffer is valid for its length.
            let n =
                unsafe { libc::recv(fd.as_raw_fd(), buffer.as_mut_ptr().cast(), buffer.len(), 0) };
            if n <= 0 {
                return None;
            }
            let (part, next) = sdp::response(&buffer[..n as usize], transaction)?;
            lists.extend_from_slice(&part);
            if next.is_empty() {
                return sdp::rfcomm_channel(&lists);
            }
            state = next;
        }
        None
    }

    pub fn connect(address: u64) -> Result<Socket, String> {
        let start = Instant::now();
        let channel = sdp_channel(address).unwrap_or(DEFAULT_CHANNEL);
        let fd = open_socket(libc::SOCK_STREAM, BTPROTO_RFCOMM).map_err(|e| e.to_string())?;
        let target = SockaddrRc {
            rc_family: libc::AF_BLUETOOTH as libc::sa_family_t,
            rc_bdaddr: bdaddr(address),
            rc_channel: channel,
        };
        let left = Duration::from_millis(CONNECT_TIMEOUT_MS).saturating_sub(start.elapsed());
        connect_to(&fd, &target, left.as_millis().max(1000) as u64).map_err(|e| e.to_string())?;
        Ok(Socket(fd))
    }

    impl RfcommSocket for Socket {
        fn send(&mut self, data: &[u8]) -> io::Result<usize> {
            let deadline = Instant::now() + Duration::from_millis(2000);
            loop {
                // SAFETY: data is valid for its length.
                let n = unsafe {
                    libc::send(
                        self.0.as_raw_fd(),
                        data.as_ptr().cast(),
                        data.len(),
                        libc::MSG_NOSIGNAL,
                    )
                };
                if n >= 0 {
                    return Ok(n as usize);
                }
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::WouldBlock {
                    return Err(error);
                }
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() || !poll(&self.0, libc::POLLOUT, left.as_millis() as u64)? {
                    return Err(io::Error::from(io::ErrorKind::TimedOut));
                }
            }
        }

        fn receive(&mut self, out: &mut Vec<u8>, timeout_ms: u64) -> io::Result<usize> {
            if !poll(&self.0, libc::POLLIN, timeout_ms)? {
                return Ok(0);
            }
            let mut buffer = [0u8; 1024];
            // SAFETY: buffer is valid for its length.
            let n = unsafe {
                libc::recv(
                    self.0.as_raw_fd(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    0,
                )
            };
            if n == 0 {
                return Err(io::Error::from(io::ErrorKind::ConnectionAborted));
            }
            if n < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::WouldBlock {
                    return Ok(0);
                }
                return Err(error);
            }
            out.extend_from_slice(&buffer[..n as usize]);
            Ok(n as usize)
        }
    }

    /// Runs bluetoothctl with a time limit: it waits forever when bluetoothd
    /// is not running.
    fn bluetoothctl(args: &[&str]) -> Option<String> {
        let mut child = Command::new("bluetoothctl")
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut stdout = child.stdout.take()?;
        let reader = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stdout.read_to_string(&mut text);
            text
        });
        let deadline = Instant::now() + Duration::from_millis(3000);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
            }
        };
        let text = reader.join().ok()?;
        status.filter(|s| s.success()).map(|_| text)
    }

    /// The paired devices BlueZ knows, the classic ones of them. Pairing is
    /// needed for these adapters anyway (PIN), so devices in range that were
    /// never paired are not searched for.
    pub fn classic_devices(_milliseconds: u64) -> Result<Vec<BleDevice>, String> {
        let listing = bluetoothctl(&["devices", "Paired"])
            .filter(|text| !bluez::device_lines(text).is_empty())
            .or_else(|| bluetoothctl(&["paired-devices"]))
            .unwrap_or_default();
        let mut found: Vec<BleDevice> = Vec::new();
        for (address, name) in bluez::device_lines(&listing) {
            let classic = bluetoothctl(&["info", &format_address(address)])
                .is_none_or(|info| bluez::info_is_classic(&info));
            let device = classic_device(address, &name);
            if classic && !found.iter().any(|d| d.id == device.id) {
                found.push(device);
            }
        }
        Ok(found)
    }
}

// --- Everything else (macOS) --------------------------------------------------

#[cfg(not(any(windows, target_os = "linux")))]
mod os {
    use std::io;

    use super::{BleDevice, RfcommSocket, not_supported};

    pub const SUPPORTED: bool = false;

    /// No socket exists here.
    pub enum Socket {}

    impl RfcommSocket for Socket {
        fn send(&mut self, _data: &[u8]) -> io::Result<usize> {
            match *self {}
        }
        fn receive(&mut self, _out: &mut Vec<u8>, _timeout_ms: u64) -> io::Result<usize> {
            match *self {}
        }
    }

    pub fn connect(_address: u64) -> Result<Socket, String> {
        Err(not_supported())
    }

    pub fn classic_devices(_milliseconds: u64) -> Result<Vec<BleDevice>, String> {
        Err(not_supported())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use caruspex_i18n::{Language, with_language};

    use super::*;

    /// What arrives from the fake adapter, in order.
    enum Incoming {
        Bytes(&'static [u8]),
        /// Nothing within the wait.
        Quiet,
        Closed,
    }

    #[derive(Default)]
    struct Wire {
        sent: Vec<u8>,
        sends: usize,
        waits: Vec<u64>,
    }

    /// A socket with a script instead of a radio.
    struct FakeSocket {
        incoming: VecDeque<Incoming>,
        /// Most bytes one send takes (partial writes).
        chunk: usize,
        fail_send: bool,
        wire: Arc<Mutex<Wire>>,
    }

    impl FakeSocket {
        fn new(incoming: Vec<Incoming>) -> (Self, Arc<Mutex<Wire>>) {
            let wire = Arc::new(Mutex::new(Wire::default()));
            (
                Self {
                    incoming: incoming.into(),
                    chunk: usize::MAX,
                    fail_send: false,
                    wire: wire.clone(),
                },
                wire,
            )
        }
    }

    impl RfcommSocket for FakeSocket {
        fn send(&mut self, data: &[u8]) -> io::Result<usize> {
            if self.fail_send {
                return Err(io::Error::from(io::ErrorKind::ConnectionReset));
            }
            let n = data.len().min(self.chunk);
            let mut wire = self.wire.lock().unwrap();
            wire.sent.extend_from_slice(&data[..n]);
            wire.sends += 1;
            Ok(n)
        }

        fn receive(&mut self, out: &mut Vec<u8>, timeout_ms: u64) -> io::Result<usize> {
            self.wire.lock().unwrap().waits.push(timeout_ms);
            match self.incoming.pop_front() {
                Some(Incoming::Bytes(b)) => {
                    out.extend_from_slice(b);
                    Ok(b.len())
                }
                Some(Incoming::Closed) => {
                    // Stays closed.
                    self.incoming.push_front(Incoming::Closed);
                    Err(io::Error::from(io::ErrorKind::ConnectionAborted))
                }
                Some(Incoming::Quiet) | None => Ok(0),
            }
        }
    }

    fn adapter(id: &str, name: &str) -> BleDevice {
        BleDevice {
            id: id.into(),
            name: name.into(),
            classic: true,
            ..BleDevice::default()
        }
    }

    fn transport(incoming: Vec<Incoming>) -> (RfcommTransport<FakeSocket>, Arc<Mutex<Wire>>) {
        let (socket, wire) = FakeSocket::new(incoming);
        (
            RfcommTransport::with_socket(socket, "Bluetooth OBDII".into()),
            wire,
        )
    }

    #[test]
    fn spp_uuid() {
        assert_eq!(
            format!("{SPP_UUID:032X}"),
            "0000110100001000800000805F9B34FB"
        );
    }

    #[test]
    fn write_sends_everything_in_pieces() {
        let (mut socket, wire) = FakeSocket::new(Vec::new());
        socket.chunk = 2;
        let mut t = RfcommTransport::with_socket(socket, "x".into());
        assert!(t.write(b"ATZ\r"));
        let wire = wire.lock().unwrap();
        assert_eq!(wire.sent, b"ATZ\r");
        assert_eq!(wire.sends, 2);
    }

    #[test]
    fn read_waits_once_then_takes_what_is_there() {
        let (mut t, wire) = transport(vec![
            Incoming::Bytes(b"ELM327 "),
            Incoming::Bytes(b"v1.5\r\r"),
            Incoming::Bytes(b">"),
            Incoming::Quiet,
            Incoming::Bytes(b"later"),
        ]);
        assert_eq!(t.read(1500), b"ELM327 v1.5\r\r>");
        // The first wait has the full time, the rest only look.
        assert_eq!(wire.lock().unwrap().waits, [1500, 0, 0, 0]);
        assert_eq!(t.read(100), b"later");
    }

    #[test]
    fn read_timeout_is_empty() {
        let (mut t, wire) = transport(vec![Incoming::Quiet]);
        assert!(t.read(250).is_empty());
        assert_eq!(wire.lock().unwrap().waits, [250]);
        assert!(!t.is_disconnected());
    }

    #[test]
    fn flush_drops_waiting_bytes() {
        let (mut t, _) = transport(vec![
            Incoming::Bytes(b"stale"),
            Incoming::Bytes(b" answer"),
            Incoming::Quiet,
            Incoming::Bytes(b"OK"),
        ]);
        t.flush_input();
        assert_eq!(t.read(100), b"OK");
    }

    #[test]
    fn closed_connection_stops_everything() {
        let (mut t, wire) = transport(vec![Incoming::Bytes(b"OK\r>"), Incoming::Closed]);
        // The bytes before the close still count.
        assert_eq!(t.read(100), b"OK\r>");
        assert!(t.is_disconnected());
        assert!(!t.write(b"0100\r"));
        assert!(t.read(100).is_empty());
        assert!(wire.lock().unwrap().sent.is_empty());
    }

    #[test]
    fn failed_send_counts_as_disconnected() {
        let (mut socket, _) = FakeSocket::new(Vec::new());
        socket.fail_send = true;
        let mut t = RfcommTransport::with_socket(socket, "x".into());
        assert!(!t.write(b"ATZ\r"));
        assert!(t.is_disconnected());
    }

    #[test]
    fn open_connects_to_the_address() {
        let mut asked = None;
        let t = open_with(&adapter("00:1D:A5:68:98:8B", "OBDII"), |address| {
            asked = Some(address);
            Ok(FakeSocket::new(Vec::new()).0)
        })
        .expect("open");
        assert_eq!(asked, Some(0x001D_A568_988B));
        assert_eq!(t.name(), "Bluetooth OBDII");

        let t = open_with(&adapter("00:1D:A5:68:98:8B", ""), |_| {
            Ok(FakeSocket::new(Vec::new()).0)
        })
        .expect("open");
        assert_eq!(t.name(), "Bluetooth 00:1D:A5:68:98:8B");
    }

    #[test]
    fn open_texts() {
        with_language(Language::English, || {
            let mut called = false;
            let error = open_with(&adapter("hci0", "OBDII"), |_| {
                called = true;
                Ok(FakeSocket::new(Vec::new()).0)
            })
            .err()
            .expect("no address");
            assert!(!called);
            assert_eq!(error, "Classic Bluetooth needs the address of the device.");

            let error = open_with::<FakeSocket>(&adapter("00:1D:A5:68:98:8B", "OBDII"), |_| {
                Err("Host is down".into())
            })
            .err()
            .expect("refused");
            assert_eq!(
                error,
                "Could not connect to OBDII. Is the adapter paired in the system (PIN usually 1234 or 0000)? (Host is down)"
            );

            let error =
                open_with::<FakeSocket>(&adapter("00:1D:A5:68:98:8B", ""), |_| Err(String::new()))
                    .err()
                    .expect("refused");
            assert_eq!(
                error,
                "Could not connect to 00:1D:A5:68:98:8B. Is the adapter paired in the system (PIN usually 1234 or 0000)?"
            );
        });
        with_language(Language::German, || {
            let error = open_with::<FakeSocket>(&adapter("x", ""), |_| Err(String::new()))
                .err()
                .expect("no address");
            assert_eq!(
                error,
                "Klassisches Bluetooth braucht die Adresse des Geräts."
            );
        });
    }

    #[test]
    fn elm_dialogue_over_rfcomm() {
        let (mut t, wire) = transport(vec![
            Incoming::Quiet,
            Incoming::Bytes(b"ATZ\r\r\rELM327 v1.5\r\r>"),
        ]);
        t.flush_input();
        assert!(t.write(b"ATZ\r"));
        let mut reply = Vec::new();
        for _ in 0..5 {
            if reply.contains(&b'>') {
                break;
            }
            reply.extend(t.read(250));
        }
        assert!(String::from_utf8_lossy(&reply).contains("ELM327 v1.5"));
        assert_eq!(wire.lock().unwrap().sent, b"ATZ\r");
    }

    #[test]
    fn classic_device_entry() {
        let d = classic_device(0x001D_A568_988B, " OBDII ");
        assert_eq!(d.id, "00:1D:A5:68:98:8B");
        assert_eq!(d.name, "OBDII");
        assert!(d.classic && !d.low_energy);
        assert!(d.score >= 3);
        // Unnamed devices show their address as the name.
        let d = classic_device(0x001D_A568_988B, "00-1D-A5-68-98-8B");
        assert!(d.name.is_empty());
    }

    // --- SDP ---

    /// An answer as an ELM327 clone gives it: one record with L2CAP and
    /// RFCOMM channel `channel`.
    fn record(channel: u8) -> Vec<u8> {
        vec![
            0x35, 0x13, // all records
            0x35, 0x11, // one record
            0x09, 0x00, 0x04, // attribute 0x0004
            0x35, 0x0C, // protocol descriptor list
            0x35, 0x03, 0x19, 0x01, 0x00, // L2CAP
            0x35, 0x05, 0x19, 0x00, 0x03, 0x08, channel, // RFCOMM, channel
        ]
    }

    fn answer(transaction: u16, lists: &[u8], state: &[u8]) -> Vec<u8> {
        let mut params = (lists.len() as u16).to_be_bytes().to_vec();
        params.extend_from_slice(lists);
        params.push(state.len() as u8);
        params.extend_from_slice(state);
        let mut pdu = vec![0x07];
        pdu.extend_from_slice(&transaction.to_be_bytes());
        pdu.extend_from_slice(&(params.len() as u16).to_be_bytes());
        pdu.extend_from_slice(&params);
        pdu
    }

    #[test]
    fn sdp_request_bytes() {
        assert_eq!(
            sdp::request(1, &[]),
            [
                0x06, 0x00, 0x01, 0x00, 0x0D, 0x35, 0x03, 0x19, 0x11, 0x01, 0x04, 0x00, 0x35, 0x03,
                0x09, 0x00, 0x04, 0x00
            ]
        );
        let again = sdp::request(2, &[0xAA, 0xBB]);
        assert_eq!(&again[1..5], [0x00, 0x02, 0x00, 0x0F]);
        assert_eq!(&again[again.len() - 3..], [0x02, 0xAA, 0xBB]);
    }

    #[test]
    fn sdp_finds_the_channel() {
        let lists = record(3);
        let pdu = answer(1, &lists, &[]);
        let (got, state) = sdp::response(&pdu, 1).expect("answer");
        assert!(state.is_empty());
        assert_eq!(sdp::rfcomm_channel(&got), Some(3));
    }

    #[test]
    fn sdp_answer_in_pieces() {
        let lists = record(1);
        let first = answer(1, &lists[..9], &[0x01, 0x09]);
        let second = answer(2, &lists[9..], &[]);
        let (a, state) = sdp::response(&first, 1).expect("first");
        assert_eq!(state, [0x01, 0x09]);
        let (b, state) = sdp::response(&second, 2).expect("second");
        assert!(state.is_empty());
        let all = [a, b].concat();
        assert_eq!(sdp::rfcomm_channel(&all), Some(1));
    }

    #[test]
    fn sdp_long_uuid_and_bad_answers() {
        // RFCOMM written as a full 128-bit UUID, channel as UINT16.
        let mut rfcomm = vec![0x1C, 0x00, 0x00, 0x00, 0x03];
        rfcomm.extend_from_slice(&[
            0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0x80, 0x5F, 0x9B, 0x34, 0xFB,
        ]);
        rfcomm.extend_from_slice(&[0x09, 0x00, 0x05]);
        let mut inner = vec![0x35, rfcomm.len() as u8];
        inner.extend_from_slice(&rfcomm);
        let mut lists = vec![0x35, inner.len() as u8];
        lists.extend_from_slice(&inner);
        assert_eq!(sdp::rfcomm_channel(&lists), Some(5));

        // Error answer, other transaction, cut off, no service.
        assert!(sdp::response(&[0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03], 1).is_none());
        assert!(sdp::response(&answer(2, &record(1), &[]), 1).is_none());
        let pdu = answer(1, &record(1), &[]);
        assert!(sdp::response(&pdu[..pdu.len() - 4], 1).is_none());
        assert_eq!(sdp::rfcomm_channel(&record(1)[..15]), None);
        assert_eq!(sdp::rfcomm_channel(&[0x35, 0x00]), None);
        assert_eq!(sdp::rfcomm_channel(&[]), None);
    }

    // --- bluetoothctl output ---

    #[test]
    fn bluez_device_list() {
        let text = "Device 00:1D:A5:68:98:8B OBDII\n\
                    \u{1b}[0;94m[bluetooth]\u{1b}[0m# \n\
                    Device F4:4E:FD:12:6B:70 JBL Flip 5\n\
                    [NEW] Device AA:BB:CC:DD:EE:01 AA-BB-CC-DD-EE-01\n\
                    Device nonsense\n";
        let list = bluez::device_lines(text);
        assert_eq!(list.len(), 3);
        assert_eq!(list[0], (0x001D_A568_988B, "OBDII".to_string()));
        assert_eq!(list[1].1, "JBL Flip 5");
        assert_eq!(classic_device(list[2].0, &list[2].1).name, "");
    }

    #[test]
    fn bluez_info_kind() {
        let classic = "Device 00:1D:A5:68:98:8B (public)\n\tName: OBDII\n\tClass: 0x00001f00\n\tPaired: yes\n";
        let spp = "Device 00:1D:A5:68:98:8B (public)\n\tUUID: Serial Port               (00001101-0000-1000-8000-00805f9b34fb)\n";
        let le = "Device D2:5C:41:7A:09:3E (random)\n\tName: IOS-Vlink\n\tUUID: Generic Access Profile (00001800-0000-1000-8000-00805f9b34fb)\n";
        assert!(bluez::info_is_classic(classic));
        assert!(bluez::info_is_classic(spp));
        assert!(!bluez::info_is_classic(le));
    }
}
