//! ELM327 over Bluetooth LE, on btleplug.
//!
//! btleplug is asynchronous; the rest of Caruspex talks to adapters through
//! the blocking [`Transport`]. Everything Bluetooth therefore runs on one
//! small tokio runtime owned by this module, and the blocking calls hand
//! their work over to it and wait. They may be called from any thread that
//! is not itself driving that runtime.
//!
//! Safety rule: nothing is written to a device that
//! [`crate::ble_profile::is_obd_adapter`] does not recognise as an OBD
//! adapter, not even the descriptor write that switches notifications on.
//! Other devices (lamps, headphones, watches) are at most looked at.
//!
//! Classic Bluetooth (RFCOMM) is not part of btleplug. On Windows a paired
//! SPP adapter is a COM port and on Linux an `rfcomm bind` gives
//! /dev/rfcommN, so [`crate::serial`] covers both; a direct RFCOMM socket
//! without a COM port, as the Qt version had, does not exist here.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::Duration;

use btleplug::api::{
    Central, CentralEvent, CentralState, CharPropFlags, Characteristic, Manager as _,
    Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use caruspex_i18n::l;
use futures::StreamExt;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use crate::ble_profile::{
    BleChannel, BleCharacteristicDesc, BleServiceDesc, choose_ble_channel, is_obd_adapter,
    obd_adapter_name_score,
};
use crate::transport::Transport;

/// One adapter seen by a scan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BleDevice {
    /// Address on Windows, the BlueZ path on Linux, a UUID on macOS.
    pub id: String,
    pub name: String,
    pub rssi: i32,
    /// [`obd_adapter_name_score`]
    pub score: i32,
    /// Offers Bluetooth LE.
    pub low_energy: bool,
    /// Offers classic Bluetooth (RFCOMM serial port). Always false here:
    /// btleplug only scans for Bluetooth LE.
    pub classic: bool,
}

/// Bit mask for passing the two flags around: 1 = LE, 2 = classic.
pub fn bluetooth_kinds(d: &BleDevice) -> i32 {
    i32::from(d.low_energy) | (i32::from(d.classic) << 1)
}

/// What a device offers, as seen without writing anything to it.
#[derive(Clone, Debug, Default)]
pub struct BleInspection {
    pub name: String,
    pub services: Vec<BleServiceDesc>,
    pub channel: BleChannel,
    /// [`is_obd_adapter`] for this device.
    pub recognised: bool,
}

fn runtime() -> Result<&'static Runtime, String> {
    static RUNTIME: OnceLock<Option<Runtime>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .thread_name("caruspex-ble")
                .enable_all()
                .build()
                .ok()
        })
        .as_ref()
        .ok_or_else(no_ble)
}

/// Runs a future on the Bluetooth runtime and waits for it.
fn run<F>(future: F) -> Result<F::Output, String>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let rt = runtime()?;
    let (tx, rx) = std::sync::mpsc::channel();
    rt.spawn(async move {
        let _ = tx.send(future.await);
    });
    rx.recv().map_err(|_| no_ble())
}

fn no_ble() -> String {
    l(
        "Dieser Rechner hat kein Bluetooth LE, das Caruspex nutzen kann.",
        "This computer has no Bluetooth LE that Caruspex can use.",
    )
    .to_string()
}

fn describe_error(e: &btleplug::Error) -> String {
    match e {
        btleplug::Error::PermissionDenied => l(
            "Caruspex darf Bluetooth nicht verwenden. Bitte in den Systemeinstellungen unter Datenschutz erlauben.",
            "Caruspex is not allowed to use Bluetooth. Please allow it in the system settings under privacy.",
        )
        .to_string(),
        btleplug::Error::NotSupported(_) => no_ble(),
        other => other.to_string(),
    }
}

/// The first Bluetooth adapter of the computer, shared by scans and
/// connections: a device found by one scan is only known to that adapter
/// object.
async fn central() -> Result<Adapter, String> {
    static CENTRAL: tokio::sync::OnceCell<Adapter> = tokio::sync::OnceCell::const_new();
    CENTRAL
        .get_or_try_init(|| async {
            let manager = Manager::new().await.map_err(|e| describe_error(&e))?;
            let adapters = manager.adapters().await.map_err(|e| describe_error(&e))?;
            adapters.into_iter().next().ok_or_else(no_ble)
        })
        .await
        .cloned()
}

async fn ensure_powered(central: &Adapter) -> Result<(), String> {
    if let Ok(CentralState::PoweredOff) = central.adapter_state().await {
        return Err(l("Bluetooth ist ausgeschaltet.", "Bluetooth is switched off.").to_string());
    }
    Ok(())
}

async fn device_of(p: &Peripheral) -> Option<BleDevice> {
    let props = p.properties().await.ok().flatten().unwrap_or_default();
    let name = props.local_name.unwrap_or_default().trim().to_string();
    let id = p.id().to_string();
    if id.is_empty() {
        return None;
    }
    Some(BleDevice {
        score: obd_adapter_name_score(&name),
        id,
        name,
        rssi: i32::from(props.rssi.unwrap_or(0)),
        low_energy: true,
        classic: false,
    })
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

async fn scan_async(milliseconds: u64) -> Result<Vec<BleDevice>, String> {
    let central = central().await?;
    ensure_powered(&central).await?;
    central
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| describe_error(&e))?;
    tokio::time::sleep(Duration::from_millis(milliseconds)).await;
    let _ = central.stop_scan().await;
    let peripherals = central
        .peripherals()
        .await
        .map_err(|e| describe_error(&e))?;
    let mut found = Vec::new();
    for p in &peripherals {
        if let Some(d) = device_of(p).await {
            found.push(d);
        }
    }
    sort_devices(&mut found);
    Ok(found)
}

/// Scans for Bluetooth LE devices for the given time and returns them, OBD
/// adapters first. The error is the reason when the system has no usable
/// Bluetooth.
pub fn scan_ble_devices(milliseconds: u64) -> Result<Vec<BleDevice>, String> {
    run(scan_async(milliseconds))?
}

fn same_id(p: &Peripheral, id: &str) -> bool {
    p.id().to_string().eq_ignore_ascii_case(id) || p.address().to_string().eq_ignore_ascii_case(id)
}

/// The peripheral with this id, scanning for it if this adapter has not seen
/// it yet.
async fn find_peripheral(central: &Adapter, id: &str) -> Option<Peripheral> {
    let known = |list: Vec<Peripheral>| list.into_iter().find(|p| same_id(p, id));
    if let Some(p) = central.peripherals().await.ok().and_then(known) {
        return Some(p);
    }
    central.start_scan(ScanFilter::default()).await.ok()?;
    let mut found = None;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(200)).await;
        if let Some(p) = central.peripherals().await.ok().and_then(known) {
            found = Some(p);
            break;
        }
    }
    let _ = central.stop_scan().await;
    found
}

fn could_not_connect(shown: &str, last_error: &str) -> String {
    let mut text = l(
        "Verbindung zu %1 kam nicht zustande",
        "Could not connect to %1",
    )
    .replace("%1", shown);
    if !last_error.is_empty() {
        text.push_str(": ");
        text.push_str(last_error);
    }
    text
}

struct Connected {
    peripheral: Peripheral,
    name: String,
    services: Vec<BleServiceDesc>,
    characteristics: Vec<Vec<Characteristic>>,
    channel: BleChannel,
}

/// Connects and reads the services. Nothing is written.
async fn connect_and_discover(device: BleDevice) -> Result<Connected, String> {
    let shown = if device.name.is_empty() {
        device.id.clone()
    } else {
        device.name.clone()
    };
    let central = central().await?;
    ensure_powered(&central).await?;
    let Some(peripheral) = find_peripheral(&central, &device.id).await else {
        return Err(could_not_connect(&shown, ""));
    };
    let name = if device.name.is_empty() {
        device_of(&peripheral)
            .await
            .map(|d| d.name)
            .unwrap_or_default()
    } else {
        device.name.clone()
    };

    if !peripheral.is_connected().await.unwrap_or(false) {
        match tokio::time::timeout(Duration::from_millis(15000), peripheral.connect()).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(could_not_connect(&shown, &describe_error(&e))),
            Err(_) => return Err(could_not_connect(&shown, "")),
        }
    }
    let discovered =
        tokio::time::timeout(Duration::from_millis(10000), peripheral.discover_services()).await;
    if !matches!(discovered, Ok(Ok(()))) {
        let _ = peripheral.disconnect().await;
        return Err(l(
            "Das Gerät hat seine Dienste nicht gemeldet.",
            "The device did not report its services.",
        )
        .to_string());
    }

    // The characteristics in the order they are described, so the indices
    // of the chosen channel point back to them.
    let mut services = Vec::new();
    let mut characteristics = Vec::new();
    for service in peripheral.services() {
        let mut desc = BleServiceDesc {
            uuid: service.uuid.to_string(),
            characteristics: Vec::new(),
        };
        let mut list = Vec::new();
        for c in service.characteristics {
            let p = c.properties;
            desc.characteristics.push(BleCharacteristicDesc {
                uuid: c.uuid.to_string(),
                notify: p.contains(CharPropFlags::NOTIFY),
                indicate: p.contains(CharPropFlags::INDICATE),
                write: p.contains(CharPropFlags::WRITE),
                write_no_response: p.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE),
            });
            list.push(c);
        }
        services.push(desc);
        characteristics.push(list);
    }
    let channel = choose_ble_channel(&services);
    Ok(Connected {
        peripheral,
        name,
        services,
        characteristics,
        channel,
    })
}

/// Connects, reads what the device offers and disconnects again, without
/// writing anything to it.
pub fn inspect_ble_device(device: &BleDevice) -> Result<BleInspection, String> {
    let device = device.clone();
    run(async move {
        let c = connect_and_discover(device).await?;
        let _ = c.peripheral.disconnect().await;
        Ok(BleInspection {
            recognised: is_obd_adapter(&c.name, &c.channel),
            name: c.name,
            services: c.services,
            channel: c.channel,
        })
    })?
}

struct Shared {
    buffer: Mutex<Vec<u8>>,
    ready: Condvar,
    disconnected: AtomicBool,
}

impl Shared {
    fn push(&self, data: &[u8]) {
        if let Ok(mut b) = self.buffer.lock() {
            b.extend_from_slice(data);
        }
        self.ready.notify_all();
    }
    fn hang_up(&self) {
        self.disconnected.store(true, Ordering::SeqCst);
        self.ready.notify_all();
    }
}

/// An ELM327 behind a Bluetooth LE serial channel.
pub struct BleTransport {
    peripheral: Peripheral,
    write_char: Characteristic,
    write_type: WriteType,
    chunk: usize,
    shared: Arc<Shared>,
    tasks: Vec<JoinHandle<()>>,
    name: String,
    profile: String,
}

impl BleTransport {
    /// Connects, finds the serial channel and switches notifications on.
    /// Refuses every device that is not recognised as an OBD adapter, before
    /// anything is written to it.
    pub fn open(device: &BleDevice) -> Result<Self, String> {
        let device = device.clone();
        run(Self::open_async(device))?
    }

    async fn open_async(device: BleDevice) -> Result<Self, String> {
        let label = format!(
            "Bluetooth LE {}",
            if device.name.is_empty() {
                &device.id
            } else {
                &device.name
            }
        );
        let c = connect_and_discover(device.clone()).await?;
        let (Some(service), Some(notify), Some(write)) = (
            c.channel.service,
            c.channel.notify_characteristic,
            c.channel.write_characteristic,
        ) else {
            let _ = c.peripheral.disconnect().await;
            return Err(l(
                "Auf diesem Gerät wurde kein serieller Kanal für OBD gefunden. Ist es ein OBD-Adapter mit Bluetooth LE?",
                "No serial channel for OBD was found on this device. Is it an OBD adapter with Bluetooth LE?",
            )
            .to_string());
        };
        if !is_obd_adapter(&c.name, &c.channel) {
            let _ = c.peripheral.disconnect().await;
            return Err(l(
                "Das Gerät sieht nicht wie ein OBD-Adapter aus. An unbekannte Geräte sendet Caruspex nichts.",
                "The device does not look like an OBD adapter. Caruspex sends nothing to unknown devices.",
            )
            .to_string());
        }
        let notify_char = c.characteristics[service][notify].clone();
        let write_char = c.characteristics[service][write].clone();
        let peripheral = c.peripheral;
        let shared = Arc::new(Shared {
            buffer: Mutex::new(Vec::new()),
            ready: Condvar::new(),
            disconnected: AtomicBool::new(false),
        });
        let mut tasks = Vec::new();

        // The stream first: notifications that arrive before anyone listens
        // are dropped.
        let mut stream = match peripheral.notifications().await {
            Ok(s) => s,
            Err(e) => {
                let _ = peripheral.disconnect().await;
                return Err(could_not_connect(&label, &describe_error(&e)));
            }
        };
        let pump_shared = shared.clone();
        let wanted = notify_char.uuid;
        tasks.push(tokio::spawn(async move {
            while let Some(n) = stream.next().await {
                if n.uuid == wanted {
                    pump_shared.push(&n.value);
                }
            }
            pump_shared.hang_up();
        }));
        if let Ok(central) = central().await
            && let Ok(mut events) = central.events().await
        {
            let id = peripheral.id();
            let watch_shared = shared.clone();
            tasks.push(tokio::spawn(async move {
                while let Some(event) = events.next().await {
                    if let CentralEvent::DeviceDisconnected(gone) = event
                        && gone == id
                    {
                        watch_shared.hang_up();
                        break;
                    }
                }
            }));
        }

        // Notifications (or indications) on through the CCC descriptor; the
        // adapter is usable even if the confirmation does not come in time.
        let _ = tokio::time::timeout(
            Duration::from_millis(3000),
            peripheral.subscribe(&notify_char),
        )
        .await;

        if shared.disconnected.load(Ordering::SeqCst) {
            for t in &tasks {
                t.abort();
            }
            return Err(could_not_connect(&label, ""));
        }
        Ok(Self {
            peripheral,
            write_type: if c.channel.write_without_response {
                WriteType::WithoutResponse
            } else {
                WriteType::WithResponse
            },
            write_char,
            // btleplug does not tell the negotiated MTU; 20 bytes fit into
            // every ATT packet.
            chunk: 20,
            shared,
            tasks,
            name: label,
            profile: c.channel.profile,
        })
    }

    /// Which channel was used ("FFF0", "vLinker", "generic", ...), for the log.
    pub fn profile(&self) -> &str {
        &self.profile
    }

    fn disconnected(&self) -> bool {
        self.shared.disconnected.load(Ordering::SeqCst)
    }
}

impl Transport for BleTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        if self.disconnected() {
            return false;
        }
        for piece in data.chunks(self.chunk) {
            let p = self.peripheral.clone();
            let c = self.write_char.clone();
            let kind = self.write_type;
            let bytes = piece.to_vec();
            let sent = run(async move {
                tokio::time::timeout(Duration::from_millis(2000), p.write(&c, &bytes, kind)).await
            });
            if !matches!(sent, Ok(Ok(Ok(())))) {
                return false;
            }
        }
        !self.disconnected()
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let Ok(mut buffer) = self.shared.buffer.lock() else {
            return Vec::new();
        };
        if buffer.is_empty() && !self.disconnected() {
            let waited = self.shared.ready.wait_timeout_while(
                buffer,
                Duration::from_millis(timeout_ms),
                |b| b.is_empty() && !self.shared.disconnected.load(Ordering::SeqCst),
            );
            match waited {
                Ok((b, _)) => buffer = b,
                Err(_) => return Vec::new(),
            }
        }
        std::mem::take(&mut *buffer)
    }

    fn flush_input(&mut self) {
        if let Ok(mut b) = self.shared.buffer.lock() {
            b.clear();
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

impl Drop for BleTransport {
    fn drop(&mut self) {
        for t in &self.tasks {
            t.abort();
        }
        let p = self.peripheral.clone();
        let _ = run(async move {
            let _ = tokio::time::timeout(Duration::from_millis(3000), p.disconnect()).await;
        });
    }
}

/// Developer aid behind `--ble-probe id|auto`: lists the Bluetooth LE
/// devices, connects to the given one (or the first adapter), sends ATZ and
/// prints everything. Devices that are not OBD adapters are only looked at.
/// Returns the exit code: 0 fine, 1 failed, 2 nothing to connect to.
pub fn ble_probe(wanted: &str, out: &mut dyn std::io::Write) -> i32 {
    let (devices, error) = match scan_ble_devices(5000) {
        Ok(list) => (list, String::new()),
        Err(e) => (Vec::new(), e),
    };
    let _ = writeln!(
        out,
        "{} device(s){}{}",
        devices.len(),
        if error.is_empty() { "" } else { ", error: " },
        error
    );
    for d in &devices {
        let kind = if d.low_energy && d.classic {
            "LE+BR"
        } else if d.classic {
            "classic"
        } else {
            "LE"
        };
        let _ = writeln!(
            out,
            "  {:<40} {:<30} {:<8} rssi {:>4} score {}",
            d.id, d.name, kind, d.rssi, d.score
        );
    }
    let automatic = wanted == "auto";
    let mut target = devices
        .iter()
        .find(|d| {
            if automatic {
                d.score >= 3
            } else {
                d.id == wanted
            }
        })
        .cloned()
        .unwrap_or_default();
    if target.id.is_empty() && !automatic {
        target.id = wanted.to_string();
        target.low_energy = true;
    }
    if target.id.is_empty() {
        return 2;
    }
    let _ = writeln!(out, "connecting to {} ...", target.id);

    // Only adapters get a command; other devices are just looked at.
    if target.score < 3 {
        return match inspect_ble_device(&target) {
            Ok(seen) => {
                let _ = writeln!(
                    out,
                    "LE channel {}",
                    if seen.channel.valid() {
                        seen.channel.profile.as_str()
                    } else {
                        "none"
                    }
                );
                let _ = writeln!(out, "not an OBD adapter by name, nothing sent");
                0
            }
            Err(e) => {
                let _ = writeln!(out, "LE open failed: {e}");
                1
            }
        };
    }
    let mut transport = match BleTransport::open(&target) {
        Ok(t) => {
            let _ = writeln!(out, "LE channel {}", t.profile());
            t
        }
        Err(e) => {
            let _ = writeln!(out, "LE open failed: {e}");
            return 1;
        }
    };
    let _ = writeln!(out, "sending ATZ");
    transport.write(b"ATZ\r");
    let mut reply = Vec::new();
    for _ in 0..20 {
        if reply.contains(&b'>') {
            break;
        }
        reply.extend(transport.read(250));
    }
    let _ = writeln!(out, "reply: {}", String::from_utf8_lossy(&reply));
    if reply.is_empty() { 1 } else { 0 }
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
    }

    #[test]
    fn connect_error_text() {
        caruspex_i18n::with_language(caruspex_i18n::Language::English, || {
            assert_eq!(could_not_connect("OBDII", ""), "Could not connect to OBDII");
            assert_eq!(
                could_not_connect("OBDII", "timeout"),
                "Could not connect to OBDII: timeout"
            );
        });
    }
}
