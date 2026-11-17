// Android: the ways to the adapter that only the Java side of the system
// can open. AdapterPlugin.kt (in gen/android) opens classic Bluetooth
// (serial port profile) and USB serial adapters on an OTG cable and hands
// the bytes over; here they become a caruspex_link::Transport, so the worker
// talks to them like to any other ELM327. WLAN adapters and the demo car need
// nothing of this: TCP works the same as on the desktop.
//
// Every call blocks until the Java side answers. They come from the worker
// thread and from blocking tasks, never from the main thread, which the
// Java side needs to receive them.
use std::sync::OnceLock;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use caruspex_i18n::l;
use caruspex_link::Transport;
use caruspex_link::serial::{PortInfo, chip_guess};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tauri::Wry;
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};

use crate::commands::BluetoothAccess;
use crate::texts::args;

static ADAPTER: OnceLock<PluginHandle<Wry>> = OnceLock::new();

/// The plugin that loads AdapterPlugin.kt; added to the builder in run().
pub fn plugin() -> TauriPlugin<Wry> {
    Builder::new("caruspex-adapter")
        .setup(|_app, api| {
            let handle =
                api.register_android_plugin("dev.senseiissei.caruspex", "AdapterPlugin")?;
            let _ = ADAPTER.set(handle);
            Ok(())
        })
        .build()
}

fn call<T: DeserializeOwned>(command: &str, payload: Value) -> Result<T, String> {
    let handle = ADAPTER.get().ok_or_else(|| String::from("AdapterPlugin"))?;
    handle
        .run_mobile_plugin(command, payload)
        .map_err(|e| e.to_string())
}

// --- Permissions -------------------------------------------------------------------------

pub fn bluetooth_access() -> Result<BluetoothAccess, String> {
    call("bluetoothStatus", json!({}))
}

/// Shows the system question (after the interface has explained it).
pub fn request_bluetooth_access() -> Result<BluetoothAccess, String> {
    call("requestBluetooth", json!({}))
}

fn permission_text() -> String {
    l(
        "Caruspex darf Bluetooth noch nicht nutzen. Bitte den Zugriff auf Geräte in der Nähe erlauben.",
        "Caruspex may not use Bluetooth yet. Please allow access to nearby devices.",
    )
    .to_string()
}

// --- WLAN ----------------------------------------------------------------------------------

/// A WLAN adapter has no internet, so Android would send the sockets out
/// over mobile data; while connected they go through the WLAN.
pub fn bind_wifi(on: bool) {
    let _ = call::<Value>("bindWifi", json!({ "on": on }));
}

// --- An open link -------------------------------------------------------------------------

#[derive(Deserialize)]
struct Opened {
    id: i64,
}

#[derive(Deserialize)]
struct Received {
    data: String,
}

/// A link the Java side holds open, by its number.
struct Link {
    id: i64,
}

impl Link {
    fn write(&mut self, data: &[u8]) -> bool {
        call::<Value>(
            "write",
            json!({ "id": self.id, "data": BASE64.encode(data) }),
        )
        .is_ok_and(|v| v["ok"].as_bool() == Some(true))
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        call::<Received>("read", json!({ "id": self.id, "timeout": timeout_ms }))
            .ok()
            .and_then(|r| BASE64.decode(r.data).ok())
            .unwrap_or_default()
    }

    fn flush(&mut self) {
        let _ = call::<Value>("flush", json!({ "id": self.id }));
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        let _ = call::<Value>("close", json!({ "id": self.id }));
    }
}

// --- USB serial adapters ------------------------------------------------------------------

#[derive(Deserialize)]
struct UsbDevice {
    device: String,
    vid: u16,
    pid: u16,
    product: String,
    driver: String,
}

#[derive(Deserialize)]
struct UsbList {
    devices: Vec<UsbDevice>,
}

/// The USB serial adapters on the OTG cable, in the shape of the desktop
/// port list.
pub fn enumerate_ports() -> Vec<PortInfo> {
    let Ok(list) = call::<UsbList>("usbDevices", json!({})) else {
        return Vec::new();
    };
    list.devices
        .into_iter()
        .map(|d| {
            let chip = chip_guess(d.vid, d.pid);
            let name = if d.product.is_empty() {
                d.driver.clone()
            } else {
                d.product.clone()
            };
            PortInfo {
                description: format!("{name} ({})", d.device),
                hardware_id: format!("USB VID_{:04X} PID_{:04X}", d.vid, d.pid),
                vid: d.vid,
                pid: d.pid,
                likely_adapter: true,
                chip_guess: if chip.is_empty() { d.driver } else { chip },
                device: d.device,
            }
        })
        .collect()
}

/// An ELM327 on a USB serial adapter, under the name the worker uses on the
/// desktop for its serial ports.
#[derive(Default)]
pub struct SerialTransport {
    link: Option<Link>,
    device: String,
    baud: u32,
}

impl SerialTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, device: &str, baud: u32) -> Result<(), String> {
        self.link = None;
        let opened =
            call::<Opened>("openUsb", json!({ "device": device, "baud": baud })).map_err(|e| {
                if e.contains("permission") {
                    l(
                        "Android hat den Zugriff auf den USB-Adapter nicht erlaubt.",
                        "Android did not allow access to the USB adapter.",
                    )
                    .to_string()
                } else {
                    args(
                        l(
                            "Der USB-Adapter ließ sich nicht öffnen: %1",
                            "The USB adapter could not be opened: %1",
                        ),
                        &[&e],
                    )
                }
            })?;
        self.link = Some(Link { id: opened.id });
        self.device = device.to_string();
        self.baud = baud;
        Ok(())
    }
}

impl Transport for SerialTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        self.link.as_mut().is_some_and(|link| link.write(data))
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        self.link
            .as_mut()
            .map(|link| link.read(timeout_ms))
            .unwrap_or_default()
    }
    fn flush_input(&mut self) {
        if let Some(link) = self.link.as_mut() {
            link.flush();
        }
    }
    fn name(&self) -> String {
        format!("{} @ {}{}", self.device, self.baud, l(" Baud", " baud"))
    }
}

// --- Classic Bluetooth --------------------------------------------------------------------

/// A paired classic Bluetooth adapter (serial port profile).
struct BluetoothTransport {
    link: Link,
    name: String,
}

impl Transport for BluetoothTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        self.link.write(data)
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        self.link.read(timeout_ms)
    }
    fn flush_input(&mut self) {
        self.link.flush();
    }
    fn name(&self) -> String {
        self.name.clone()
    }
}

/// The worker's Bluetooth functions (worker.rs, mod ble) on Android: the
/// paired classic adapters. Bluetooth LE needs btleplug's Java side and is
/// not in the Android app yet.
pub mod ble {
    use caruspex_i18n::l;
    use caruspex_link::Transport;
    use caruspex_link::ble_profile::obd_adapter_name_score;
    use caruspex_link::bluetooth::{BleDevice, bluetooth_kinds, sort_devices};
    use serde::Deserialize;
    use serde_json::json;

    use super::{BluetoothTransport, Link, Opened, call, permission_text};
    use crate::rows::BleDeviceRow;
    use crate::texts::args;

    #[derive(Deserialize)]
    struct Paired {
        address: String,
        name: String,
    }

    #[derive(Deserialize)]
    struct PairedList {
        devices: Vec<Paired>,
    }

    fn paired() -> Result<Vec<BleDevice>, String> {
        let access = super::bluetooth_access()?;
        if !access.available {
            return Err(l(
                "Dieses Gerät hat kein Bluetooth.",
                "This device has no Bluetooth.",
            )
            .to_string());
        }
        if !access.granted {
            return Err(permission_text());
        }
        if !access.enabled {
            return Err(l(
                "Bluetooth ist aus. Bitte in den Android-Einstellungen einschalten.",
                "Bluetooth is off. Please switch it on in the Android settings.",
            )
            .to_string());
        }
        let list: PairedList = call("pairedDevices", json!({})).map_err(|e| {
            if e.contains("permission") {
                permission_text()
            } else {
                e
            }
        })?;
        let mut devices: Vec<BleDevice> = list
            .devices
            .into_iter()
            .map(|p| BleDevice {
                score: obd_adapter_name_score(&p.name),
                id: p.address,
                name: p.name,
                classic: true,
                ..BleDevice::default()
            })
            .collect();
        sort_devices(&mut devices);
        Ok(devices)
    }

    /// The paired classic devices, OBD adapters first. Nothing is scanned:
    /// an adapter is paired once in the Android settings.
    pub fn scan(_milliseconds: i32) -> (Vec<BleDeviceRow>, String) {
        match paired() {
            Ok(devices) if devices.is_empty() => (
                Vec::new(),
                l(
                    "Kein gekoppelter Bluetooth-Adapter. Bitte einmal in den Android-Einstellungen koppeln, die PIN ist meist 1234 oder 0000.",
                    "No paired Bluetooth adapter. Please pair it once in the Android settings, the PIN is usually 1234 or 0000.",
                )
                .to_string(),
            ),
            Ok(devices) => (
                devices
                    .iter()
                    .map(|d| BleDeviceRow {
                        id: d.id.clone(),
                        name: d.name.clone(),
                        score: d.score,
                        kinds: bluetooth_kinds(d),
                    })
                    .collect(),
                String::new(),
            ),
            Err(error) => (Vec::new(), error),
        }
    }

    /// The paired devices that look like OBD adapters.
    pub fn candidates() -> Result<Vec<BleDevice>, String> {
        Ok(paired()?.into_iter().filter(|d| d.score >= 3).collect())
    }

    pub fn device(id: &str, name: &str, kinds: i32) -> BleDevice {
        // Every device of the Android list is a classic one.
        let _ = kinds;
        BleDevice {
            id: id.to_string(),
            name: name.to_string(),
            classic: true,
            ..BleDevice::default()
        }
    }

    pub fn open(_d: &BleDevice) -> Result<(Box<dyn Transport>, String), String> {
        Err(l(
            "Bluetooth LE gibt es in der Android-App noch nicht. Klassische Bluetooth-Adapter gehen, wenn sie gekoppelt sind.",
            "Bluetooth LE is not in the Android app yet. Classic Bluetooth adapters work once they are paired.",
        )
        .to_string())
    }

    pub fn open_classic(d: &BleDevice) -> Result<Box<dyn Transport>, String> {
        let opened: Opened = call("openBluetooth", json!({ "address": d.id })).map_err(|e| {
            if e.contains("permission") {
                permission_text()
            } else {
                args(
                    l(
                        "Die Bluetooth-Verbindung zum Adapter ging nicht: %1",
                        "The Bluetooth connection to the adapter failed: %1",
                    ),
                    &[&e],
                )
            }
        })?;
        let name = if d.name.is_empty() {
            d.id.clone()
        } else {
            format!("{} ({})", d.name, d.id)
        };
        Ok(Box::new(BluetoothTransport {
            link: Link { id: opened.id },
            name,
        }))
    }

    pub fn kinds(d: &BleDevice) -> i32 {
        bluetooth_kinds(d)
    }
}

// --- Files of the system dialog -----------------------------------------------------------

#[derive(Deserialize)]
struct ReadFile {
    name: String,
    data: String,
}

/// Writes a file the save dialog handed out as a content:// address.
pub fn write_uri(uri: &str, data: &[u8]) -> Result<(), String> {
    call::<Value>(
        "writeUri",
        json!({ "uri": uri, "data": BASE64.encode(data) }),
    )
    .map(|_| ())
}

/// Reads a file the open dialog handed out: its name and its bytes.
pub fn read_uri(uri: &str) -> Result<(String, Vec<u8>), String> {
    let file: ReadFile = call("readUri", json!({ "uri": uri }))?;
    let bytes = BASE64.decode(file.data).map_err(|e| e.to_string())?;
    Ok((file.name, bytes))
}
