// Android: the ways to the adapter that only the Java side of the system
// can open. AdapterPlugin.kt (in gen/android) opens classic Bluetooth
// (serial port profile), Bluetooth LE adapters (GATT) and USB serial
// adapters on an OTG cable and hands the bytes over; here they become a
// caruspex_link::Transport, so the worker talks to them like to any other
// ELM327. WLAN adapters and the demo car need nothing of this: TCP works
// the same as on the desktop.
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
    /// The other side has gone (socket closed, device disconnected).
    #[serde(default)]
    closed: bool,
}

impl Received {
    fn bytes(&self) -> Vec<u8> {
        BASE64.decode(&self.data).unwrap_or_default()
    }
}

/// A link the Java side holds open, by its number.
struct Link {
    id: i64,
    /// The Java side said that the device has gone.
    closed: bool,
    /// Bytes that came in while `lost` looked whether the link is alive.
    early: Vec<u8>,
}

impl Link {
    fn new(id: i64) -> Self {
        Self {
            id,
            closed: false,
            early: Vec::new(),
        }
    }

    fn write(&mut self, data: &[u8]) -> bool {
        call::<Value>(
            "write",
            json!({ "id": self.id, "data": BASE64.encode(data) }),
        )
        .is_ok_and(|v| v["ok"].as_bool() == Some(true))
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        if !self.early.is_empty() {
            return std::mem::take(&mut self.early);
        }
        let received = call::<Received>("read", json!({ "id": self.id, "timeout": timeout_ms }))
            .unwrap_or(Received {
                data: String::new(),
                closed: true,
            });
        self.closed = received.closed;
        received.bytes()
    }

    /// The device has gone (cable pulled, Bluetooth connection closed); what
    /// came in with the look is kept for the next read.
    fn lost(&mut self) -> bool {
        if !self.closed && self.early.is_empty() {
            self.early = self.read(0);
        }
        self.closed && self.early.is_empty()
    }

    fn flush(&mut self) {
        self.early.clear();
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
        self.link = Some(Link::new(opened.id));
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
    fn lost(&mut self) -> bool {
        self.link.as_mut().is_none_or(Link::lost)
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
    fn lost(&mut self) -> bool {
        self.link.lost()
    }
}

/// The worker's Bluetooth functions (worker.rs, mod ble) on Android: the
/// paired classic adapters and the Bluetooth LE adapters in range.
pub mod ble {
    use caruspex_i18n::l;
    use caruspex_link::Transport;
    use caruspex_link::ble_gatt::{
        GattLink, GattTransport, WANTED_MTU, characteristic, plan_channel,
    };
    use caruspex_link::ble_profile::{
        BleServiceDesc, advertised_profile, obd_adapter_name_score, scan_score,
    };
    use caruspex_link::bluetooth::{BleDevice, bluetooth_kinds, merge_classic, sort_devices};
    use serde::Deserialize;
    use serde_json::json;

    use super::{BluetoothTransport, Link, Opened, call, permission_text};
    use crate::rows::BleDeviceRow;
    use crate::texts::args;

    /// The reason word AdapterPlugin.kt rejects with, out of the error text
    /// of the plugin call. Longer words first: "location-off" is no "off".
    fn reason(error: &str) -> &'static str {
        const REASONS: [&str; 8] = [
            "location-permission",
            "location-off",
            "no-bluetooth",
            "scan-failed",
            "permission",
            "services",
            "connect",
            "off",
        ];
        let words: Vec<&str> = error
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .collect();
        REASONS
            .into_iter()
            .find(|r| words.contains(r))
            .unwrap_or("")
    }

    fn no_bluetooth() -> String {
        l(
            "Dieses Gerät hat kein Bluetooth.",
            "This device has no Bluetooth.",
        )
        .to_string()
    }

    fn bluetooth_off() -> String {
        l(
            "Bluetooth ist aus. Bitte in den Android-Einstellungen einschalten.",
            "Bluetooth is off. Please switch it on in the Android settings.",
        )
        .to_string()
    }

    /// The text for an error of the Java side, `other` when it is none of
    /// the known ones.
    fn explain(error: &str, other: impl FnOnce(&str) -> String) -> String {
        match reason(error) {
            "location-permission" => l(
                "Bis Android 11 findet die Suche Adapter mit Bluetooth LE nur mit Zugriff auf den Standort. Bitte oben erlauben; gekoppelte klassische Adapter gehen auch ohne.",
                "Up to Android 11 the search only finds Bluetooth LE adapters with access to the location. Please allow it above; paired classic adapters work without it.",
            )
            .to_string(),
            "location-off" => l(
                "Bis Android 11 sucht Android nur nach Adaptern mit Bluetooth LE, wenn der Standort eingeschaltet ist. Bitte in den Schnelleinstellungen einschalten und neu suchen.",
                "Up to Android 11, Android only searches for Bluetooth LE adapters while the location is switched on. Please switch it on in the quick settings and search again.",
            )
            .to_string(),
            "scan-failed" => l(
                "Android hat die Bluetooth-Suche nicht gestartet. Bitte Bluetooth kurz aus- und wieder einschalten und neu suchen.",
                "Android did not start the Bluetooth search. Please switch Bluetooth off and on again and search again.",
            )
            .to_string(),
            "no-bluetooth" => no_bluetooth(),
            "permission" => permission_text(),
            "off" => bluetooth_off(),
            _ => other(error),
        }
    }

    /// Bluetooth there, allowed and switched on.
    fn usable() -> Result<(), String> {
        let access = super::bluetooth_access()?;
        if !access.available {
            return Err(no_bluetooth());
        }
        if !access.granted {
            return Err(permission_text());
        }
        if !access.enabled {
            return Err(bluetooth_off());
        }
        Ok(())
    }

    #[derive(Deserialize)]
    struct Paired {
        address: String,
        name: String,
    }

    #[derive(Deserialize)]
    struct PairedList {
        devices: Vec<Paired>,
    }

    /// The paired classic devices: a classic adapter is paired once in the
    /// Android settings.
    fn paired() -> Result<Vec<BleDevice>, String> {
        let list: PairedList =
            call("pairedDevices", json!({})).map_err(|e| explain(&e, str::to_string))?;
        Ok(list
            .devices
            .into_iter()
            .map(|p| BleDevice {
                score: obd_adapter_name_score(&p.name),
                id: p.address,
                name: p.name,
                classic: true,
                ..BleDevice::default()
            })
            .collect())
    }

    #[derive(Deserialize)]
    struct Advertised {
        address: String,
        name: String,
        rssi: i32,
        #[serde(default)]
        services: Vec<String>,
    }

    #[derive(Deserialize)]
    struct AdvertisedList {
        devices: Vec<Advertised>,
    }

    /// The Bluetooth LE devices in range. Only listens.
    fn low_energy(milliseconds: u64) -> Result<Vec<BleDevice>, String> {
        let list: AdvertisedList = call("bleScan", json!({ "timeout": milliseconds }))
            .map_err(|e| explain(&e, str::to_string))?;
        Ok(list
            .devices
            .into_iter()
            .map(|a| {
                let profile = advertised_profile(&a.services);
                BleDevice {
                    score: scan_score(&a.name, profile),
                    id: a.address,
                    name: a.name,
                    rssi: a.rssi,
                    low_energy: true,
                    classic: false,
                    profile: profile.unwrap_or_default().to_string(),
                }
            })
            .collect())
    }

    /// Both kinds, OBD adapters first. A failed LE search still shows the
    /// paired classic adapters; the error then goes along as the note.
    fn devices(milliseconds: u64) -> (Vec<BleDevice>, String) {
        if let Err(error) = usable() {
            return (Vec::new(), error);
        }
        let classic = paired().unwrap_or_default();
        let (mut found, error) = match low_energy(milliseconds) {
            Ok(list) => (list, String::new()),
            Err(error) => (Vec::new(), error),
        };
        merge_classic(&mut found, classic);
        sort_devices(&mut found);
        (found, error)
    }

    pub fn scan(milliseconds: i32) -> (Vec<BleDeviceRow>, String) {
        let (found, error) = devices(milliseconds.max(0) as u64);
        (found.iter().map(BleDeviceRow::from).collect(), error)
    }

    /// The devices of a six second search that look like OBD adapters.
    pub fn candidates() -> Result<Vec<BleDevice>, String> {
        let (found, error) = devices(6000);
        let adapters: Vec<BleDevice> = found.into_iter().filter(|d| d.score >= 3).collect();
        if adapters.is_empty() && !error.is_empty() {
            return Err(error);
        }
        Ok(adapters)
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

    #[derive(Deserialize)]
    struct GattCharacteristic {
        uuid: String,
        properties: u32,
    }

    #[derive(Deserialize)]
    struct GattService {
        uuid: String,
        characteristics: Vec<GattCharacteristic>,
    }

    #[derive(Deserialize)]
    struct GattOpened {
        id: i64,
        #[serde(default)]
        name: String,
        services: Vec<GattService>,
    }

    #[derive(Deserialize)]
    struct GattReady {
        mtu: u32,
    }

    /// The set up GATT connection the Java side holds.
    struct AndroidGatt {
        link: Link,
    }

    impl GattLink for AndroidGatt {
        fn write_value(&mut self, value: &[u8]) -> bool {
            self.link.write(value)
        }
        fn notifications(&mut self, timeout_ms: u64) -> Vec<u8> {
            self.link.read(timeout_ms)
        }
        fn clear(&mut self) {
            self.link.flush();
        }
        fn closed(&self) -> bool {
            self.link.closed
        }
    }

    fn could_not_connect(shown: &str) -> String {
        args(
            l(
                "Verbindung zu %1 kam nicht zustande",
                "Could not connect to %1",
            ),
            &[&shown],
        )
    }

    /// Connects, reads the services and, only for a recognised OBD adapter,
    /// switches the notifications on. Any other device is disconnected
    /// again without a single write.
    pub fn open(d: &BleDevice) -> Result<(Box<dyn Transport>, String), String> {
        let shown = if d.name.is_empty() { &d.id } else { &d.name };
        let opened: GattOpened = call("bleConnect", json!({ "address": d.id, "timeout": 15000 }))
            .map_err(|e| {
            explain(&e, |_| match reason(&e) {
                "services" => l(
                    "Das Gerät hat seine Dienste nicht gemeldet.",
                    "The device did not report its services.",
                )
                .to_string(),
                _ => could_not_connect(shown),
            })
        })?;
        // From here on the Java side closes the connection when `link` is
        // dropped, also on every early return.
        let link = Link::new(opened.id);
        let services: Vec<BleServiceDesc> = opened
            .services
            .iter()
            .map(|s| BleServiceDesc {
                uuid: s.uuid.clone(),
                characteristics: s
                    .characteristics
                    .iter()
                    .map(|c| characteristic(&c.uuid, c.properties))
                    .collect(),
            })
            .collect();
        let name = if d.name.is_empty() {
            opened.name.trim().to_string()
        } else {
            d.name.clone()
        };
        let plan = plan_channel(&name, &services)?;
        let ready: GattReady = call(
            "bleSetup",
            json!({
                "id": link.id,
                "service": plan.service,
                "notify": plan.notify,
                "write": plan.write,
                "noResponse": plan.write_without_response,
                "indicate": plan.indicate,
                "mtu": WANTED_MTU,
            }),
        )
        .map_err(|e| explain(&e, |_| could_not_connect(shown)))?;
        let transport = GattTransport::new(
            AndroidGatt { link },
            ready.mtu,
            format!("Bluetooth LE {shown}"),
        );
        Ok((
            Box::new(transport),
            format!("{}, MTU {}", plan.profile, ready.mtu),
        ))
    }

    pub fn open_classic(d: &BleDevice) -> Result<Box<dyn Transport>, String> {
        let opened: Opened = call("openBluetooth", json!({ "address": d.id })).map_err(|e| {
            explain(&e, |e| {
                args(
                    l(
                        "Die Bluetooth-Verbindung zum Adapter ging nicht: %1",
                        "The Bluetooth connection to the adapter failed: %1",
                    ),
                    &[&e],
                )
            })
        })?;
        let name = if d.name.is_empty() {
            d.id.clone()
        } else {
            format!("{} ({})", d.name, d.id)
        };
        Ok(Box::new(BluetoothTransport {
            link: Link::new(opened.id),
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
