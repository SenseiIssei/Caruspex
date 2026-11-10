//! Serial port (USB adapters, Bluetooth SPP, KKL cable) on top of the
//! serialport crate, with the port list and adapter guesses of the C++
//! version.

use std::io::{Read, Write};
use std::time::Duration;

use caruspex_i18n::l;
use serialport::{ClearBuffer, DataBits, FlowControl, Parity, SerialPort, StopBits};

use crate::transport::Transport;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PortInfo {
    /// "COM5", "/dev/ttyUSB0"
    pub device: String,
    /// "USB-SERIAL CH340 (COM5)"
    pub description: String,
    /// "USB\\VID_1A86&PID_7523\\..."
    pub hardware_id: String,
    pub vid: u16,
    pub pid: u16,
    pub likely_adapter: bool,
    /// "CH340", "FTDI FT232R", ...
    pub chip_guess: String,
}

struct ChipHint {
    vid: u16,
    pid: u16,
    name: &'static str,
    /// Only where the name contains German.
    name_en: Option<&'static str>,
}

// Chips found in diagnostic adapters and KKL cables.
const CHIPS: [ChipHint; 9] = [
    ChipHint {
        vid: 0x0403,
        pid: 0x6001,
        name: "FTDI FT232R",
        name_en: None,
    },
    ChipHint {
        vid: 0x0403,
        pid: 0x6015,
        name: "FTDI FT231X",
        name_en: None,
    },
    ChipHint {
        vid: 0x0403,
        pid: 0x6010,
        name: "FTDI FT2232",
        name_en: None,
    },
    ChipHint {
        vid: 0x0403,
        pid: 0xFA20,
        name: "FTDI (Ross-Tech HEX)",
        name_en: None,
    },
    ChipHint {
        vid: 0x1A86,
        pid: 0x7523,
        name: "CH340",
        name_en: None,
    },
    ChipHint {
        vid: 0x1A86,
        pid: 0x5523,
        name: "CH341",
        name_en: None,
    },
    ChipHint {
        vid: 0x10C4,
        pid: 0xEA60,
        name: "CP2102",
        name_en: None,
    },
    ChipHint {
        vid: 0x067B,
        pid: 0x2303,
        name: "PL2303",
        name_en: None,
    },
    // Only the Windows list knows this one (any product id).
    ChipHint {
        vid: 0x2303,
        pid: 0x0000,
        name: "PL2303 (Variante)",
        name_en: Some("PL2303 (variant)"),
    },
];

/// Reads up to four hex digits like `sscanf("%4x")`.
fn scan_hex4(text: &str) -> u16 {
    let digits: String = text
        .chars()
        .take(4)
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    u16::from_str_radix(&digits, 16).unwrap_or(0)
}

/// VID and PID out of a Windows hardware id ("USB\\VID_1A86&PID_7523\\...",
/// "FTDIBUS\\VID_0403+PID_6001+...").
pub fn parse_usb_id(hardware_id: &str) -> (u16, u16) {
    let up = hardware_id.to_ascii_uppercase();
    let vid = up
        .find("VID_")
        .map(|v| scan_hex4(&up[v + 4..]))
        .unwrap_or(0);
    let pid = up
        .find("PID_")
        .map(|p| scan_hex4(&up[p + 4..]))
        .unwrap_or(0);
    (vid, pid)
}

/// Whether the description or hardware id look like a diagnostic adapter.
pub fn looks_like_adapter(text: &str) -> bool {
    const HINTS: [&str; 14] = [
        "OBD",
        "ELM",
        "FTDI",
        "CH340",
        "CH341",
        "CP210",
        "PL2303",
        "USB SERIAL",
        "USB-SERIAL",
        "BLUETOOTH",
        "SILICON LABS",
        "PROLIFIC",
        "VCI",
        "KKL",
    ];
    let up = text.to_ascii_uppercase();
    HINTS.iter().any(|h| up.contains(h))
}

/// The chip guess of the Windows list: an entry with product id 0 matches
/// every product of that vendor.
pub fn chip_guess(vid: u16, pid: u16) -> String {
    CHIPS
        .iter()
        .find(|c| c.vid == vid && (c.pid == pid || c.pid == 0))
        .map(|c| match c.name_en {
            Some(en) => l(c.name, en).to_string(),
            None => c.name.to_string(),
        })
        .unwrap_or_default()
}

/// Fills VID, PID, chip guess and the adapter guess the way the Windows code
/// does, from device, description and hardware id.
pub fn describe_windows_port(device: &str, description: &str, hardware_id: &str) -> PortInfo {
    let (vid, pid) = parse_usb_id(hardware_id);
    let mut info = PortInfo {
        device: device.to_string(),
        description: if description.is_empty() {
            device.to_string()
        } else {
            description.to_string()
        },
        hardware_id: hardware_id.to_string(),
        vid,
        pid,
        likely_adapter: false,
        chip_guess: chip_guess(vid, pid),
    };
    info.likely_adapter = !info.chip_guess.is_empty()
        || looks_like_adapter(&format!("{} {}", info.description, info.hardware_id));
    info
}

/// Adapters first, then by name.
pub fn sort_ports(ports: &mut [PortInfo]) {
    ports.sort_by(|a, b| {
        b.likely_adapter
            .cmp(&a.likely_adapter)
            .then_with(|| a.device.cmp(&b.device))
    });
}

/// All COM ports with description and USB ID.
#[cfg(windows)]
pub fn enumerate_ports() -> Vec<PortInfo> {
    windows_ports::enumerate()
}

#[cfg(windows)]
mod windows_ports {
    //! The C++ version asks SetupAPI for the present devices of the "Ports"
    //! class and reads PortName, the friendly name and the hardware id. The
    //! same values live in the registry under Enum, which is readable without
    //! unsafe code; the serialport crate says which ports are present.

    use std::collections::HashMap;

    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

    use super::{PortInfo, describe_windows_port, sort_ports};

    /// GUID_DEVCLASS_PORTS
    const PORTS_CLASS: &str = "{4d36e978-e325-11ce-bfc1-08002be10318}";

    struct Entry {
        description: String,
        hardware_id: String,
    }

    fn registry_ports() -> HashMap<String, Entry> {
        let mut out = HashMap::new();
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let Ok(root) = hklm.open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Enum", KEY_READ)
        else {
            return out;
        };
        for enumerator in root.enum_keys().flatten() {
            let Ok(e) = root.open_subkey_with_flags(&enumerator, KEY_READ) else {
                continue;
            };
            for device in e.enum_keys().flatten() {
                let Ok(d) = e.open_subkey_with_flags(&device, KEY_READ) else {
                    continue;
                };
                for instance in d.enum_keys().flatten() {
                    let Ok(i) = d.open_subkey_with_flags(&instance, KEY_READ) else {
                        continue;
                    };
                    let class: String = i.get_value("ClassGUID").unwrap_or_default();
                    if !class.eq_ignore_ascii_case(PORTS_CLASS) {
                        continue;
                    }
                    let Ok(params) = i.open_subkey_with_flags("Device Parameters", KEY_READ) else {
                        continue;
                    };
                    let port: String = params.get_value("PortName").unwrap_or_default();
                    if !port.starts_with("COM") {
                        continue;
                    }
                    let description: String = i.get_value("FriendlyName").unwrap_or_default();
                    // A multi-string; the C++ buffer only saw the first entry.
                    let hardware_id = i
                        .get_value::<Vec<String>, _>("HardwareID")
                        .ok()
                        .and_then(|v| v.into_iter().next())
                        .unwrap_or_default();
                    out.insert(
                        port,
                        Entry {
                            description,
                            hardware_id,
                        },
                    );
                }
            }
        }
        out
    }

    pub fn enumerate() -> Vec<PortInfo> {
        let present = serialport::available_ports().unwrap_or_default();
        let registry = registry_ports();
        let mut out = Vec::new();
        for port in present {
            if !port.port_name.starts_with("COM") {
                continue;
            }
            if let Some(entry) = registry.get(&port.port_name) {
                out.push(describe_windows_port(
                    &port.port_name,
                    &entry.description,
                    &entry.hardware_id,
                ));
                continue;
            }
            // Not in the registry walk (no rights, or not of the Ports
            // class): what the serialport crate knows.
            let (description, hardware_id) = match &port.port_type {
                serialport::SerialPortType::UsbPort(usb) => (
                    usb.product.clone().unwrap_or_default(),
                    format!("USB\\VID_{:04X}&PID_{:04X}", usb.vid, usb.pid),
                ),
                _ => (String::new(), String::new()),
            };
            out.push(describe_windows_port(
                &port.port_name,
                &description,
                &hardware_id,
            ));
        }
        sort_ports(&mut out);
        out
    }
}

/// All USB and Bluetooth serial ports under /dev.
#[cfg(unix)]
pub fn enumerate_ports() -> Vec<PortInfo> {
    // Names under which USB adapters and Bluetooth ports appear.
    const PREFIXES: [&str; 9] = [
        "ttyUSB",
        "ttyACM",
        "rfcomm",
        "cu.usbserial",
        "cu.usbmodem",
        "cu.SLAB",
        "cu.wchusbserial",
        "cu.Bluetooth",
        "tty.usbserial",
    ];
    // On Linux, VID and PID live in sysfs next to the device.
    fn read_usb_ids(name: &str) -> Option<(u16, u16)> {
        let base = format!("/sys/class/tty/{name}/device/../");
        let read_hex = |file: &str| -> Option<u16> {
            let text = std::fs::read_to_string(format!("{base}{file}")).ok()?;
            let word = text.split_whitespace().next()?;
            let digits: String = word.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
            Some(u32::from_str_radix(&digits, 16).unwrap_or(0) as u16)
        };
        Some((read_hex("idVendor")?, read_hex("idProduct")?))
    }

    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir("/dev") else {
        return out;
    };
    for entry in dir.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !PREFIXES.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        let mut info = PortInfo {
            device: format!("/dev/{name}"),
            description: name.clone(),
            ..PortInfo::default()
        };
        if let Some((vid, pid)) = read_usb_ids(&name) {
            info.vid = vid;
            info.pid = pid;
            info.hardware_id = format!("USB VID_{vid:04X} PID_{pid:04X}");
            // The POSIX list matches exact product ids only.
            if let Some(chip) = CHIPS[..8].iter().find(|c| c.vid == vid && c.pid == pid) {
                info.chip_guess = chip.name.to_string();
            }
        }
        info.likely_adapter = true; // after the name filter only suitable ports remain
        out.push(info);
    }
    out.sort_by(|a, b| a.device.cmp(&b.device));
    out
}

#[cfg(not(any(windows, unix)))]
pub fn enumerate_ports() -> Vec<PortInfo> {
    Vec::new()
}

/// Baud rates the POSIX version can set.
#[cfg(unix)]
fn baud_supported(baud: u32) -> bool {
    matches!(
        baud,
        9600 | 19200 | 38400 | 57600 | 115200 | 230400 | 460800 | 500000 | 921600
    )
}

#[cfg(not(unix))]
fn baud_supported(_baud: u32) -> bool {
    true
}

fn unsupported_baud(baud: u32) -> String {
    format!(
        "{}{}{}",
        l("Baudrate ", "Baud rate "),
        baud,
        l(" wird nicht unterstützt", " is not supported")
    )
}

/// One serial port, 8N1 without flow control, DTR and RTS on.
#[derive(Default)]
pub struct Serial {
    port: Option<Box<dyn SerialPort>>,
    device: String,
    baud: u32,
}

impl Serial {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, device: &str, baud: u32) -> Result<(), String> {
        self.close();
        if !baud_supported(baud) {
            return Err(unsupported_baud(baud));
        }
        let opened = serialport::new(device, baud)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .dtr_on_open(true)
            .timeout(Duration::from_millis(60))
            .open();
        let mut port = match opened {
            Ok(port) => port,
            Err(e) => return Err(open_error(device, baud, &e)),
        };
        let _ = port.write_request_to_send(true);
        let _ = port.clear(ClearBuffer::All);
        self.port = Some(port);
        self.device = device.to_string();
        self.baud = baud;
        #[cfg(unix)]
        crate::timing::sleep_ms(150); // some adapters need a moment after opening
        Ok(())
    }

    pub fn close(&mut self) {
        self.port = None;
        self.device.clear();
        self.baud = 0;
    }

    pub fn is_open(&self) -> bool {
        self.port.is_some()
    }

    pub fn write(&mut self, data: &[u8]) -> bool {
        let Some(port) = self.port.as_mut() else {
            return false;
        };
        let _ = port.set_timeout(Duration::from_millis(500));
        port.write_all(data).is_ok()
    }

    /// Reads whatever arrives within `timeout_ms`. Empty = nothing there.
    /// Returns as soon as something came in; the caller decides whether it
    /// needs more.
    pub fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let Some(port) = self.port.as_mut() else {
            return Vec::new();
        };
        let _ = port.set_timeout(Duration::from_millis(timeout_ms.max(1)));
        let mut buffer = [0u8; 512];
        match port.read(&mut buffer) {
            Ok(n) => buffer[..n].to_vec(),
            Err(_) => Vec::new(),
        }
    }

    pub fn flush_input(&mut self) {
        if let Some(port) = self.port.as_ref() {
            let _ = port.clear(ClearBuffer::Input);
        }
    }

    /// Pulls the transmit line permanently low. Needed for the 5-baud
    /// wake-up address of KWP1281, which is clocked out by hand.
    pub fn set_break(&mut self, active: bool) -> bool {
        let Some(port) = self.port.as_ref() else {
            return false;
        };
        if active {
            port.set_break().is_ok()
        } else {
            port.clear_break().is_ok()
        }
    }

    /// Change the baud rate on the fly without closing the port.
    pub fn change_baud(&mut self, baud: u32) -> bool {
        if !baud_supported(baud) {
            return false;
        }
        let Some(port) = self.port.as_mut() else {
            return false;
        };
        if port.set_baud_rate(baud).is_err() {
            return false;
        }
        self.baud = baud;
        true
    }

    pub fn device(&self) -> &str {
        &self.device
    }
    pub fn baud(&self) -> u32 {
        self.baud
    }
}

#[cfg(windows)]
fn open_error(device: &str, baud: u32, _error: &serialport::Error) -> String {
    // The serialport crate keeps only the text of the Windows error. Opening
    // the same path once more with the standard library gives the number.
    let path = if device.starts_with('\\') {
        device.to_string()
    } else {
        format!(r"\\.\{device}")
    };
    match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
    {
        // The port itself opens: SetCommState refused the settings.
        Ok(_) => unsupported_baud(baud),
        Err(e) => {
            let code = e.raw_os_error().unwrap_or(0);
            let mut text = format!(
                "{}{}{})",
                device,
                l(
                    " lässt sich nicht öffnen (Windows-Fehler ",
                    " cannot be opened (Windows error "
                ),
                code
            );
            const ERROR_FILE_NOT_FOUND: i32 = 2;
            const ERROR_ACCESS_DENIED: i32 = 5;
            if code == ERROR_ACCESS_DENIED {
                text.push_str(l(
                    ": belegt ein anderes Programm den Port?",
                    ": is another program using the port?",
                ));
            }
            if code == ERROR_FILE_NOT_FOUND {
                text.push_str(l(": Port existiert nicht.", ": port does not exist."));
            }
            text
        }
    }
}

#[cfg(not(windows))]
fn open_error(device: &str, _baud: u32, error: &serialport::Error) -> String {
    let mut text = format!(
        "{}{}{}",
        device,
        l(" lässt sich nicht öffnen: ", " cannot be opened: "),
        error.description
    );
    if matches!(
        error.kind,
        serialport::ErrorKind::Io(std::io::ErrorKind::PermissionDenied)
    ) {
        text.push_str(l(
            " (fehlt die Gruppe dialout? sudo usermod -aG dialout $USER)",
            " (missing the dialout group? sudo usermod -aG dialout $USER)",
        ));
    }
    text
}

/// An ELM327 on a serial port (USB, or Bluetooth SPP as a COM port).
#[derive(Default)]
pub struct SerialTransport {
    port: Serial,
}

impl SerialTransport {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn open(&mut self, device: &str, baud: u32) -> Result<(), String> {
        self.port.open(device, baud)
    }
    pub fn serial(&self) -> &Serial {
        &self.port
    }
    pub fn serial_mut(&mut self) -> &mut Serial {
        &mut self.port
    }
}

impl Transport for SerialTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        self.port.write(data)
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        self.port.read(timeout_ms)
    }
    fn flush_input(&mut self) {
        self.port.flush_input()
    }
    fn name(&self) -> String {
        format!(
            "{} @ {}{}",
            self.port.device(),
            self.port.baud(),
            l(" Baud", " baud")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_ids_come_out_of_the_hardware_id() {
        assert_eq!(
            parse_usb_id(r"USB\VID_1A86&PID_7523\5&2D"),
            (0x1A86, 0x7523)
        );
        assert_eq!(
            parse_usb_id(r"FTDIBUS\VID_0403+PID_6001+A1"),
            (0x0403, 0x6001)
        );
        assert_eq!(parse_usb_id(r"usb\vid_10c4&pid_ea60"), (0x10C4, 0xEA60));
        assert_eq!(parse_usb_id(r"BTHENUM\{00001101-0000}"), (0, 0));
    }

    #[test]
    fn windows_ports_are_guessed_like_before() {
        let ch340 =
            describe_windows_port("COM5", "USB-SERIAL CH340 (COM5)", r"USB\VID_1A86&PID_7523");
        assert_eq!(ch340.chip_guess, "CH340");
        assert!(ch340.likely_adapter);

        let variant = describe_windows_port("COM6", "", r"USB\VID_2303&PID_1234");
        assert!(variant.chip_guess.starts_with("PL2303 ("));
        assert_eq!(variant.description, "COM6");

        let bt = describe_windows_port(
            "COM7",
            "Standard Serial over Bluetooth link (COM7)",
            r"BTHENUM\{00001101-0000-1000-8000-00805F9B34FB}",
        );
        assert!(bt.chip_guess.is_empty());
        assert!(bt.likely_adapter);

        let modem =
            describe_windows_port("COM1", "Kommunikationsanschluss (COM1)", r"ACPI\PNP0501");
        assert!(!modem.likely_adapter);

        let mut list = vec![modem, bt, ch340];
        sort_ports(&mut list);
        let names: Vec<&str> = list.iter().map(|p| p.device.as_str()).collect();
        assert_eq!(names, ["COM5", "COM7", "COM1"]);
    }

    #[test]
    fn a_closed_port_does_nothing() {
        let mut serial = Serial::new();
        assert!(!serial.is_open());
        assert!(!serial.write(b"ATZ\r"));
        assert!(serial.read(1).is_empty());
        assert!(!serial.set_break(true));
        assert!(!serial.change_baud(9600));
    }
}
