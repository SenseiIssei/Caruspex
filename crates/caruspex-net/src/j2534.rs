//! SAE J2534-1 (version 04.04) PassThru devices through their driver DLL.
//! Used in CAN mode (protocol 5): raw frames, the ISO-TP framing is done by
//! CanElm like for every other CAN interface. And on the K-line (protocols 3,
//! ISO 9141, and 4, ISO 14230) as a [`ByteStream`] for KWP1281 and KWP2000,
//! with the 5-baud and the fast init done by the device through its ioctls.
//!
//! The driver comes from caruspex-j2534: a 64-bit DLL is loaded into the
//! program, a 32-bit one (older interfaces) runs in the helper program
//! `caruspex-j2534-bridge.exe` next to Caruspex. Both answer alike, so
//! nothing below knows which it is.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Mutex;

use caruspex_i18n::l;
use caruspex_j2534::constants::{
    CAN_29BIT_ID, CAN_ID_BOTH, FIVE_BAUD_INVERTED_KEY_BYTE, FIVE_BAUD_MOD, ISO9141_K_LINE_ONLY,
    ISO9141_NO_CHECKSUM, LOOPBACK, P3_MIN, PASS_FILTER, PROTOCOL_CAN, PROTOCOL_ISO9141,
    PROTOCOL_ISO14230, START_OF_MESSAGE, TX_MSG_TYPE,
};
use caruspex_j2534::{Bridge, Dll, Ioctl, IoctlOut, LoadError, Message, PassThru};
use caruspex_link::ByteStream;
use caruspex_link::timing::now_ms;
use winreg::RegKey;
use winreg::enums::{
    HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY, REG_EXPAND_SZ, REG_SZ,
};

use crate::can_bus::{CanFrame, CanLink, J2534Device, KLineKind};

// ---------------------------------------------------------------------------
// The 32-bit bridge
// ---------------------------------------------------------------------------

static BRIDGE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Uses this helper program for 32-bit drivers instead of the one next to
/// the program (`None`: back to that one). For the tests and development
/// builds, where the 32-bit build lies elsewhere.
pub(crate) fn set_bridge(path: Option<PathBuf>) {
    *BRIDGE.lock().unwrap_or_else(|e| e.into_inner()) = path;
}

/// The helper program for 32-bit drivers, if there is one.
pub(crate) fn bridge() -> Option<PathBuf> {
    let chosen = BRIDGE.lock().unwrap_or_else(|e| e.into_inner()).clone();
    chosen.or_else(caruspex_j2534::bridge_next_to_program)
}

fn not_loadable(library: &str) -> String {
    format!(
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
    )
}

fn incomplete() -> String {
    l(
        "Das ist kein vollständiger J2534-Treiber.",
        "This is not a complete J2534 driver.",
    )
    .into()
}

/// The driver for a DLL: in the program, or in the bridge for a 32-bit one.
fn load(library: &str) -> Result<Box<dyn PassThru>, String> {
    if !caruspex_j2534::is_32bit_dll(library) {
        return match Dll::load(library) {
            Ok(dll) => Ok(Box::new(dll)),
            Err(LoadError::NotLoadable) => Err(not_loadable(library)),
            Err(LoadError::Incomplete) => Err(incomplete()),
        };
    }
    let Some(exe) = bridge() else {
        return Err(format!(
            "{}{}{}",
            l(
                "Das ist ein 32-Bit-J2534-Treiber: ",
                "This is a 32-bit J2534 driver: "
            ),
            library,
            l(
                ". Dafür braucht Caruspex sein Hilfsprogramm caruspex-j2534-bridge.exe, das neben dem Programm fehlt.",
                ". For it Caruspex needs its helper program caruspex-j2534-bridge.exe, which is missing next to the program."
            )
        ));
    };
    match Bridge::start(&exe, library) {
        Ok(bridge) => Ok(Box::new(bridge)),
        Err(reason) if reason == "NotLoadable" => Err(format!(
            "{}{}",
            l(
                "Der J2534-Treiber lässt sich nicht laden: ",
                "The J2534 driver cannot be loaded: "
            ),
            library
        )),
        Err(reason) if reason == "Incomplete" => Err(incomplete()),
        Err(reason) => Err(format!(
            "{}{}",
            l(
                "Das 32-Bit-Hilfsprogramm für alte J2534-Treiber startet nicht: ",
                "The 32-bit helper program for old J2534 drivers does not start: "
            ),
            reason
        )),
    }
}

// ---------------------------------------------------------------------------
// The driver with one device
// ---------------------------------------------------------------------------

/// The loaded driver with one opened device and at most one channel.
struct Driver {
    api: Box<dyn PassThru>,
    device: u32,
    channel: u32,
    /// The device is open, so Drop closes it.
    opened: bool,
}

impl Driver {
    /// Loads the DLL and opens its first device.
    fn open(library: &str) -> Result<Driver, String> {
        let mut driver = Driver {
            api: load(library)?,
            device: 0,
            channel: 0,
            opened: false,
        };
        match driver.api.open() {
            Ok(device) => {
                driver.device = device;
                driver.opened = true;
                Ok(driver)
            }
            Err(_) => Err(format!(
                "{}{}",
                l(
                    "Das J2534-Gerät antwortet nicht: ",
                    "The J2534 device does not answer: "
                ),
                driver.last()
            )),
        }
    }

    fn library(&self) -> String {
        self.api.library().to_string()
    }

    fn last(&mut self) -> String {
        self.api.last_error()
    }

    fn connect(&mut self, protocol: u32, flags: u32, baud: u32) -> Result<(), String> {
        match self.api.connect(self.device, protocol, flags, baud) {
            Ok(channel) => {
                self.channel = channel;
                Ok(())
            }
            Err(_) => Err(self.last()),
        }
    }

    /// Lets every message through: mask and pattern zero.
    fn pass_all(&mut self, protocol: u32, size: usize) -> Result<(), String> {
        let open = Message::new(protocol, &vec![0; size]);
        if self
            .api
            .start_filter(self.channel, PASS_FILTER, &open, &open, None)
            .is_err()
        {
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

    /// An ioctl on the channel (READ_VBATT on the device).
    fn ioctl(&mut self, request: &Ioctl) -> Option<IoctlOut> {
        let target = if *request == Ioctl::ReadVbatt {
            self.device
        } else {
            self.channel
        };
        self.api.ioctl(target, request).ok()
    }

    fn set_config(&mut self, parameter: u32, value: u32) -> bool {
        self.ioctl(&Ioctl::SetConfig(vec![(parameter, value)]))
            .is_some()
    }

    fn write(&mut self, message: &Message) -> bool {
        self.api.write(self.channel, message, 100) == Ok(1)
    }

    /// The next message within `timeout_ms`, echoes and indications included.
    fn read(&mut self, timeout_ms: u64) -> Option<Message> {
        let timeout = u32::try_from(timeout_ms).unwrap_or(u32::MAX);
        self.api.read(self.channel, timeout).ok().flatten()
    }

    fn battery_voltage(&mut self) -> Option<f64> {
        match self.ioctl(&Ioctl::ReadVbatt)? {
            IoctlOut::Value(millivolts) => Some(f64::from(millivolts) / 1000.0),
            _ => None,
        }
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        if self.channel != 0 {
            self.api.disconnect(self.channel);
        }
        if self.opened {
            self.api.close(self.device);
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
        let mut bytes = frame.id.to_be_bytes().to_vec();
        bytes.extend_from_slice(&frame.data);
        let mut m = Message::new(PROTOCOL_CAN, &bytes);
        m.tx_flags = if frame.extended { CAN_29BIT_ID } else { 0 };
        self.driver.write(&m)
    }

    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame> {
        loop {
            let m = self.driver.read(timeout_ms)?;
            if m.rx_status & TX_MSG_TYPE != 0 {
                continue; // our own frame, looped back
            }
            if m.data.len() < 4 {
                continue;
            }
            let end = m.data.len().min(12);
            return Some(CanFrame {
                extended: m.rx_status & CAN_29BIT_ID != 0,
                id: u32::from_be_bytes([m.data[0], m.data[1], m.data[2], m.data[3]]),
                data: m.data[4..end].to_vec(),
            });
        }
    }

    fn name(&self) -> String {
        format!("J2534 {}", self.driver.library())
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
        format!("J2534 {} K-Line ({protocol})", self.driver.library())
    }

    fn flush_tx(&mut self) -> bool {
        if self.tx.is_empty() {
            return true;
        }
        let m = Message::new(self.protocol, &self.tx);
        self.tx.clear();
        self.driver.write(&m)
    }

    fn clear_rx(&mut self) {
        self.rx.clear();
        self.driver.ioctl(&Ioctl::ClearRxBuffer);
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
            self.rx.extend(&m.data);
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
        // FIVE_BAUD_INIT: one address byte in, the two key bytes out.
        let Some(IoctlOut::Bytes(keys)) = self.driver.ioctl(&Ioctl::FiveBaudInit(vec![address], 8))
        else {
            return false;
        };
        if keys.len() < 2 {
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
        // FAST_INIT: the StartCommunication request in, the answer out.
        match self
            .driver
            .ioctl(&Ioctl::FastInit(Message::new(self.protocol, start)))?
        {
            IoctlOut::Message(answer) => Some(answer.data),
            _ => None,
        }
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

fn read_devices(view: u32, bridged: bool, usable: bool, out: &mut Vec<J2534Device>) {
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
            bridged,
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
    read_devices(KEY_WOW64_64KEY, false, true, &mut out);
    // Drivers only registered for 32-bit programs: through the bridge, or
    // listed as not loadable when it is missing.
    read_devices(KEY_WOW64_32KEY, true, bridge().is_some(), &mut out);
    out
}
