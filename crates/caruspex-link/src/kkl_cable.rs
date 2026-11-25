//! What the computer knows about a KKL cable before anything talks to the
//! car: the USB serial chip from its vendor and product id, and for FTDI
//! chips the latency timer of the driver. Read only: nothing here opens the
//! port or changes a setting of the system.
//!
//! The latency timer matters for KWP1281 because the protocol moves single
//! bytes: the control unit sends one byte and waits for its complement. An
//! FTDI chip keeps a received byte until its buffer is full or the latency
//! timer runs out, 16 ms by default. Every byte of the handshake can sit that
//! long in the chip, and the reply to the key bytes has to be on the line
//! within 50 ms (ISO 9141-2, W4).

use caruspex_i18n::l;

use crate::serial::PortInfo;

/// The value to set; anything above gets the hint.
pub(crate) const LATENCY_GOOD_MS: u32 = 1;

/// Which USB serial chip sits in the cable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipFamily {
    Ftdi,
    Ch340,
    Prolific,
    SiliconLabs,
    /// A USB chip of another maker.
    Other,
    /// No USB ids: a built-in port, Bluetooth, or unknown.
    NotUsb,
}

impl ChipFamily {
    pub fn from_ids(vid: u16, _pid: u16) -> Self {
        match vid {
            0x0403 => Self::Ftdi,
            0x1A86 => Self::Ch340,
            // 0x2303 is the vendor id some Windows drivers report for PL2303
            // clones (serial.rs knows it as "PL2303 (variant)").
            0x067B | 0x2303 => Self::Prolific,
            0x10C4 => Self::SiliconLabs,
            0 => Self::NotUsb,
            _ => Self::Other,
        }
    }

    /// A short key for the interface and the test report.
    pub fn key(self) -> &'static str {
        match self {
            Self::Ftdi => "ftdi",
            Self::Ch340 => "ch340",
            Self::Prolific => "prolific",
            Self::SiliconLabs => "cp210x",
            Self::Other => "other",
            Self::NotUsb => "none",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Ftdi => "FTDI",
            Self::Ch340 => "WCH CH340/CH341",
            Self::Prolific => "Prolific PL2303",
            Self::SiliconLabs => "Silicon Labs CP210x",
            Self::Other => l("anderer USB-Chip", "other USB chip"),
            Self::NotUsb => l("kein USB-Chip erkannt", "no USB chip recognised"),
        }
    }

    /// What this chip means for the K-line, in one or two sentences.
    pub fn advice(self) -> &'static str {
        match self {
            Self::Ftdi => l(
                "FTDI ist der Chip, mit dem KWP1281 am verlässlichsten läuft.",
                "FTDI is the chip KWP1281 runs most reliably with.",
            ),
            Self::Ch340 => l(
                "CH340-Kabel gehen oft, schalten die Break-Leitung aber manchmal zu langsam für die 5-Baud-Weckadresse. Der Test unten misst das.",
                "CH340 cables often work, but sometimes switch the break line too slowly for the 5-baud wake-up address. The test below measures it.",
            ),
            Self::Prolific => l(
                "Bei PL2303-Kabeln sind viele Chips Nachbauten, die neuere Treiber abweisen (Code 10 im Geräte-Manager). Läuft der Port, misst der Test unten die Zeiten.",
                "Many PL2303 cables carry cloned chips that newer drivers refuse (code 10 in the Device Manager). If the port works, the test below measures the timing.",
            ),
            Self::SiliconLabs => l(
                "CP210x-Kabel kommen mit der K-Line in der Regel gut zurecht.",
                "CP210x cables usually handle the K-line well.",
            ),
            Self::Other => l(
                "Diesen Chip kennt Caruspex nicht. Der Test unten zeigt, ob die Zeiten reichen.",
                "Caruspex does not know this chip. The test below shows whether the timing is good enough.",
            ),
            Self::NotUsb => l(
                "Für diesen Port gibt es keine USB-Kennung. Ein KKL-Kabel meldet sich normalerweise als USB-Seriell-Port.",
                "This port has no USB ids. A KKL cable normally shows up as a USB serial port.",
            ),
        }
    }
}

/// The facts about a cable that can be read without opening it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CableFacts {
    pub family: ChipFamily,
    /// The latency timer of an FTDI driver in milliseconds, when it could be
    /// read (Windows registry, Linux sysfs).
    pub latency_ms: Option<u32>,
}

impl CableFacts {
    pub fn of_port(port: &PortInfo) -> Self {
        let family = ChipFamily::from_ids(port.vid, port.pid);
        let latency_ms = if family == ChipFamily::Ftdi {
            read_latency_timer(&port.device)
        } else {
            None
        };
        Self { family, latency_ms }
    }

    /// True when the latency timer is known to be higher than the K-line
    /// likes.
    #[cfg(test)]
    pub(crate) fn latency_too_high(&self) -> bool {
        self.latency_ms.is_some_and(|ms| ms > LATENCY_GOOD_MS)
    }

    /// What to do about the latency timer, empty when nothing is needed.
    pub fn latency_advice(&self) -> String {
        match (self.family, self.latency_ms) {
            (ChipFamily::Ftdi, Some(ms)) if ms > LATENCY_GOOD_MS => latency_hint(ms),
            (ChipFamily::Ftdi, None) => l(
                "Den Latenz-Timer des FTDI-Treibers konnte Caruspex nicht lesen. Für die K-Line sollte er auf 1 ms stehen: Geräte-Manager, Anschlüsse (COM & LPT), den USB Serial Port öffnen, Anschlusseinstellungen, Erweitert, Latency Timer (msec) auf 1.",
                "Caruspex could not read the latency timer of the FTDI driver. For the K-line it should be 1 ms: Device Manager, Ports (COM & LPT), open the USB Serial Port, Port Settings, Advanced, Latency Timer (msec) to 1.",
            )
            .to_string(),
            _ => String::new(),
        }
    }
}

/// The hint for a latency timer that is set too high.
pub(crate) fn latency_hint(ms: u32) -> String {
    l(
        "Der Latenz-Timer des FTDI-Treibers steht auf %1 ms. Für die K-Line besser 1 ms: Geräte-Manager öffnen, Anschlüsse (COM & LPT), den USB Serial Port doppelt anklicken, Anschlusseinstellungen, Erweitert, unter BM Options Latency Timer (msec) auf 1 stellen, mit OK bestätigen und das Kabel einmal ab- und wieder anstecken. Caruspex ändert diese Einstellung nicht selbst.",
        "The latency timer of the FTDI driver is set to %1 ms. The K-line works better with 1 ms: open the Device Manager, Ports (COM & LPT), double-click the USB Serial Port, Port Settings, Advanced, under BM Options set Latency Timer (msec) to 1, confirm with OK and unplug the cable once. Caruspex does not change this setting itself.",
    )
    .replace("%1", &ms.to_string())
}

/// The latency timer of the FTDI driver behind a port, read only.
#[cfg(windows)]
pub(crate) fn read_latency_timer(device: &str) -> Option<u32> {
    // The FTDI driver keeps its settings per device under Enum\FTDIBUS:
    // <VID_0403+PID_6001+serial>\0000\Device Parameters with PortName and
    // LatencyTimer. Reading the registry needs no rights and opens nothing.
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let bus = hklm
        .open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Enum\FTDIBUS", KEY_READ)
        .ok()?;
    for device_key in bus.enum_keys().flatten() {
        let Ok(d) = bus.open_subkey_with_flags(&device_key, KEY_READ) else {
            continue;
        };
        for instance in d.enum_keys().flatten() {
            let Ok(params) =
                d.open_subkey_with_flags(format!(r"{instance}\Device Parameters"), KEY_READ)
            else {
                continue;
            };
            let port: String = params.get_value("PortName").unwrap_or_default();
            if !port.eq_ignore_ascii_case(device) {
                continue;
            }
            if let Ok(ms) = params.get_value::<u32, _>("LatencyTimer") {
                return Some(ms);
            }
        }
    }
    None
}

/// The latency timer of the ftdi_sio driver, from sysfs.
#[cfg(target_os = "linux")]
pub(crate) fn read_latency_timer(device: &str) -> Option<u32> {
    let name = device.rsplit('/').next()?;
    let text = std::fs::read_to_string(format!("/sys/bus/usb-serial/devices/{name}/latency_timer"))
        .ok()?;
    text.trim().parse().ok()
}

#[cfg(not(any(windows, target_os = "linux")))]
pub(crate) fn read_latency_timer(_device: &str) -> Option<u32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chip_comes_from_the_vendor_id() {
        assert_eq!(ChipFamily::from_ids(0x0403, 0x6001), ChipFamily::Ftdi);
        assert_eq!(ChipFamily::from_ids(0x1A86, 0x7523), ChipFamily::Ch340);
        assert_eq!(ChipFamily::from_ids(0x067B, 0x2303), ChipFamily::Prolific);
        assert_eq!(ChipFamily::from_ids(0x2303, 0x1234), ChipFamily::Prolific);
        assert_eq!(
            ChipFamily::from_ids(0x10C4, 0xEA60),
            ChipFamily::SiliconLabs
        );
        assert_eq!(ChipFamily::from_ids(0x1234, 0x0001), ChipFamily::Other);
        assert_eq!(ChipFamily::from_ids(0, 0), ChipFamily::NotUsb);
    }

    #[test]
    fn a_high_latency_timer_gets_a_hint_with_the_value() {
        let facts = CableFacts {
            family: ChipFamily::Ftdi,
            latency_ms: Some(16),
        };
        assert!(facts.latency_too_high());
        let hint = facts.latency_advice();
        assert!(hint.contains("16 ms"), "{hint}");
        let fine = CableFacts {
            family: ChipFamily::Ftdi,
            latency_ms: Some(1),
        };
        assert!(!fine.latency_too_high());
        assert!(fine.latency_advice().is_empty());
        // Other chips have no such setting.
        let ch340 = CableFacts {
            family: ChipFamily::Ch340,
            latency_ms: None,
        };
        assert!(ch340.latency_advice().is_empty());
    }

    #[test]
    fn an_unknown_port_has_no_latency_timer() {
        assert_eq!(read_latency_timer("COM_NO_SUCH_PORT"), None);
    }
}
