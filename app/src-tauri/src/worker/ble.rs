use caruspex_link::Transport;
use caruspex_link::ble::BleTransport;
use caruspex_link::bluetooth::{BleDevice, bluetooth_kinds, scan_bluetooth_devices};
use caruspex_link::bt_classic::RfcommTransport;

use crate::rows::BleDeviceRow;

/// LE and classic devices (paired or in range), OBD adapters first.
pub fn scan(milliseconds: i32) -> (Vec<BleDeviceRow>, String) {
    match scan_bluetooth_devices(milliseconds.max(0) as u64) {
        Ok(devices) => (
            devices.iter().map(BleDeviceRow::from).collect(),
            String::new(),
        ),
        Err(error) => (Vec::new(), error),
    }
}

/// The devices of a six second search that look like OBD adapters.
pub fn candidates() -> Result<Vec<BleDevice>, String> {
    Ok(scan_bluetooth_devices(6000)?
        .into_iter()
        .filter(|d| d.score >= 3)
        .collect())
}

pub fn device(id: &str, name: &str, kinds: i32) -> BleDevice {
    let mut d = BleDevice {
        id: id.to_string(),
        name: name.to_string(),
        low_energy: kinds & 1 != 0,
        classic: kinds & 2 != 0,
        ..BleDevice::default()
    };
    if !d.low_energy && !d.classic {
        d.low_energy = true;
    }
    d
}

pub fn open(d: &BleDevice) -> Result<(Box<dyn Transport>, String), String> {
    let transport = BleTransport::open(d)?;
    let profile = transport.profile().to_string();
    Ok((Box::new(transport), profile))
}

/// The serial port of a classic adapter (RFCOMM), without a COM port.
pub fn open_classic(d: &BleDevice) -> Result<Box<dyn Transport>, String> {
    Ok(Box::new(RfcommTransport::open(d)?))
}

pub fn kinds(d: &BleDevice) -> i32 {
    bluetooth_kinds(d)
}
