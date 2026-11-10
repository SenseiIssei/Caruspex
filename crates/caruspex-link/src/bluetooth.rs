//! What Bluetooth LE and classic Bluetooth share: the device list of a scan,
//! the address format and the combined scan (scanBleDevices with classic
//! set in the C++ version).

/// One adapter seen by a scan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BleDevice {
    /// Address on Windows, the BlueZ path (LE) or the address (classic) on
    /// Linux, a UUID on macOS.
    pub id: String,
    pub name: String,
    pub rssi: i32,
    /// [`crate::ble_profile::obd_adapter_name_score`]
    pub score: i32,
    /// Offers Bluetooth LE.
    pub low_energy: bool,
    /// Offers classic Bluetooth (RFCOMM serial port).
    pub classic: bool,
}

/// Bit mask for passing the two flags around: 1 = LE, 2 = classic.
pub fn bluetooth_kinds(d: &BleDevice) -> i32 {
    i32::from(d.low_energy) | (i32::from(d.classic) << 1)
}

/// OBD adapters first, then named devices, then by signal strength.
pub fn sort_devices(devices: &mut [BleDevice]) {
    devices.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.name.is_empty().cmp(&b.name.is_empty()))
            .then_with(|| b.rssi.cmp(&a.rssi))
    });
}

/// The 48-bit device address in a device id: "AA:BB:CC:DD:EE:FF",
/// "AA-BB-CC-DD-EE-FF", twelve hex digits, or a BlueZ object path ending in
/// "dev_AA_BB_CC_DD_EE_FF". `None` for anything else (a macOS UUID) and for
/// the null address, which the C++ version rejected the same way.
pub fn parse_address(text: &str) -> Option<u64> {
    let text = text.trim();
    let text = match text.rfind("dev_") {
        Some(i) => &text[i + 4..],
        None => text,
    };
    let groups: Vec<&str> = text.split([':', '-', '_']).collect();
    let digits = match groups.as_slice() {
        [single] if single.len() == 12 => single.to_string(),
        parts if parts.len() == 6 && parts.iter().all(|p| p.len() == 2) => parts.concat(),
        _ => return None,
    };
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(&digits, 16)
        .ok()
        .filter(|&address| address != 0)
}

/// "AA:BB:CC:DD:EE:FF", the way QBluetoothAddress::toString wrote it.
pub fn format_address(address: u64) -> String {
    let bytes = address.to_be_bytes();
    bytes[2..]
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Adds the classic devices to the LE list. A dual-mode adapter shows up in
/// both with the same address and becomes one entry with both flags, like
/// the single QBluetoothDeviceInfo the C++ version got for it.
pub fn merge_classic(found: &mut Vec<BleDevice>, classic: Vec<BleDevice>) {
    for c in classic {
        let address = parse_address(&c.id);
        let existing = found
            .iter_mut()
            .find(|d| address.is_some() && parse_address(&d.id) == address);
        match existing {
            Some(d) => {
                d.classic = true;
                if d.name.is_empty() {
                    d.name = c.name;
                    d.score = c.score;
                }
            }
            None => found.push(c),
        }
    }
}

/// Scans for Bluetooth devices for the given time and returns them, OBD
/// adapters first: Bluetooth LE and, at the same time, classic devices
/// (paired or visible). The error is the reason when the system has no
/// usable Bluetooth and nothing was found.
pub fn scan_bluetooth_devices(milliseconds: u64) -> Result<Vec<BleDevice>, String> {
    #[cfg(feature = "bt-classic")]
    let classic = std::thread::Builder::new()
        .name("caruspex-bt-classic".into())
        .spawn(move || crate::bt_classic::classic_devices(milliseconds))
        .ok();

    #[cfg(feature = "ble")]
    let low_energy = crate::ble::scan_ble_devices(milliseconds);
    #[cfg(not(feature = "ble"))]
    let low_energy: Result<Vec<BleDevice>, String> = Err(caruspex_i18n::l(
        "Dieser Rechner hat kein Bluetooth LE, das Caruspex nutzen kann.",
        "This computer has no Bluetooth LE that Caruspex can use.",
    )
    .to_string());

    #[cfg(feature = "bt-classic")]
    let classic: Vec<BleDevice> = classic
        .and_then(|t| t.join().ok())
        .and_then(Result::ok)
        .unwrap_or_default();
    #[cfg(not(feature = "bt-classic"))]
    let classic: Vec<BleDevice> = Vec::new();

    let mut found = match low_energy {
        Ok(list) => list,
        // Like the C++ version: an error only counts when nothing was found.
        Err(error) if classic.is_empty() => return Err(error),
        Err(_) => Vec::new(),
    };
    merge_classic(&mut found, classic);
    sort_devices(&mut found);
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(name: &str, score: i32, rssi: i32) -> BleDevice {
        BleDevice {
            id: name.to_string(),
            name: name.to_string(),
            rssi,
            score,
            low_energy: true,
            classic: false,
        }
    }

    #[test]
    fn scan_list_order() {
        let mut list = vec![
            dev("", 0, -40),
            dev("Lamp", 0, -90),
            dev("OBDII", 3, -80),
            dev("BLE x", 1, -70),
        ];
        sort_devices(&mut list);
        let names: Vec<&str> = list.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["OBDII", "BLE x", "Lamp", ""]);
    }

    #[test]
    fn kinds_mask() {
        let mut d = dev("x", 0, 0);
        assert_eq!(bluetooth_kinds(&d), 1);
        d.classic = true;
        assert_eq!(bluetooth_kinds(&d), 3);
        d.low_energy = false;
        assert_eq!(bluetooth_kinds(&d), 2);
    }

    #[test]
    fn address_forms() {
        let want = Some(0x001D_A568_988B);
        assert_eq!(parse_address("00:1D:A5:68:98:8B"), want);
        assert_eq!(parse_address("00:1d:a5:68:98:8b"), want);
        assert_eq!(parse_address(" 00-1D-A5-68-98-8B "), want);
        assert_eq!(parse_address("001DA568988B"), want);
        assert_eq!(parse_address("hci0/dev_00_1D_A5_68_98_8B"), want);
        assert_eq!(parse_address("/org/bluez/hci0/dev_00_1D_A5_68_98_8B"), want);
    }

    #[test]
    fn address_rejects() {
        assert_eq!(parse_address(""), None);
        assert_eq!(parse_address("auto"), None);
        assert_eq!(parse_address("00:00:00:00:00:00"), None);
        assert_eq!(parse_address("00:1D:A5:68:98"), None);
        assert_eq!(parse_address("00:1D:A5:68:98:8B:11"), None);
        assert_eq!(parse_address("0:1D:A5:68:98:8B0"), None);
        assert_eq!(parse_address("G0:1D:A5:68:98:8B"), None);
        assert_eq!(parse_address("+0:1D:A5:68:98:8B"), None);
        // macOS identifies devices by a UUID, which has no address.
        assert_eq!(parse_address("6E400001-B5A3-F393-E0A9-E50E24DCCA9E"), None);
    }

    #[test]
    fn address_round_trip() {
        assert_eq!(format_address(0x001D_A568_988B), "00:1D:A5:68:98:8B");
        assert_eq!(format_address(0xAABB_CCDD_EEFF), "AA:BB:CC:DD:EE:FF");
        assert_eq!(
            parse_address(&format_address(0x1234_5678_9ABC)),
            Some(0x1234_5678_9ABC)
        );
    }

    #[test]
    fn dual_mode_adapter_becomes_one_entry() {
        let mut found = vec![BleDevice {
            id: "00:1d:a5:68:98:8b".into(),
            name: String::new(),
            rssi: -60,
            score: 0,
            low_energy: true,
            classic: false,
        }];
        let classic = vec![
            BleDevice {
                id: "00:1D:A5:68:98:8B".into(),
                name: "OBDII".into(),
                score: 4,
                low_energy: false,
                classic: true,
                ..BleDevice::default()
            },
            BleDevice {
                id: "F4:4E:FD:12:6B:70".into(),
                name: "JBL Flip 5".into(),
                low_energy: false,
                classic: true,
                ..BleDevice::default()
            },
        ];
        merge_classic(&mut found, classic);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "OBDII");
        assert_eq!(found[0].score, 4);
        assert_eq!(bluetooth_kinds(&found[0]), 3);
        assert_eq!(found[0].rssi, -60);
        assert_eq!(bluetooth_kinds(&found[1]), 2);
    }

    #[test]
    fn merge_keeps_the_le_name() {
        let mut found = vec![BleDevice {
            id: "hci0/dev_00_1D_A5_68_98_8B".into(),
            name: "IOS-Vlink".into(),
            score: 5,
            low_energy: true,
            ..BleDevice::default()
        }];
        let classic = vec![BleDevice {
            id: "00:1D:A5:68:98:8B".into(),
            name: "Other".into(),
            score: 1,
            classic: true,
            ..BleDevice::default()
        }];
        merge_classic(&mut found, classic);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "IOS-Vlink");
        assert_eq!(found[0].score, 5);
        assert!(found[0].classic);
    }
}
