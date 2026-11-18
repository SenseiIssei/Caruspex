// Connecting: serial ports and the search over all of them, Wi-Fi, Bluetooth
// LE and classic, ENET and DoIP, CAN interfaces, the demo car, recordings to
// replay, and the way that worked last time first. Also recording and
// closing.
use caruspex_advice::connect_help::LinkKind;
use caruspex_i18n::l;
#[cfg(not(target_os = "android"))]
use caruspex_link::{SerialTransport, enumerate_ports};
use caruspex_link::{StnBus, TcpTransport, Transport};
use caruspex_net::{CanElm, SlcanLink, open_can_interface, open_network_car, probe_slcan};
use caruspex_record::{
    ReplayPacing, ReplayTransport, active_recording, recording_summary, start_recording,
    stop_recording,
};
use serde_json::json;

#[cfg(not(target_os = "android"))]
use super::ble;
use super::{Job, Worker};
// Android: USB adapters and classic Bluetooth through the Java side.
#[cfg(target_os = "android")]
use crate::android::{SerialTransport, ble, enumerate_ports};
use crate::rows::*;
use crate::texts::{args, w};

/// The protocol order a connect job carries; ENET, CAN and the demo choose
/// their own.
fn protocol_order_of(job: &Job) -> String {
    match job {
        Job::OpenSerial { protocol_order, .. }
        | Job::OpenTcp { protocol_order, .. }
        | Job::OpenBle { protocol_order, .. }
        | Job::OpenReplay { protocol_order, .. } => protocol_order.clone(),
        _ => "54312678".to_string(),
    }
}

/// "COM7, 38400 baud, protocol 6" for the log.
fn route_text(route: &ConnectRoute) -> String {
    let mut parts = vec![if route.name.is_empty() {
        route.device.clone()
    } else {
        route.name.clone()
    }];
    if route.baud > 0 {
        parts.push(format!("{} Baud", route.baud));
    }
    if !route.protocol.is_empty() {
        parts.push(args(l("Protokoll %1", "protocol %1"), &[&route.protocol]));
    }
    parts.retain(|p| !p.is_empty());
    parts.join(", ")
}

impl Worker {
    /// Hands a ready transport to the ELM327 driver and reports the result.
    pub(super) fn finish_connect(
        &mut self,
        transport: Box<dyn Transport>,
        protocol_order: &str,
        report_failure: bool,
    ) -> bool {
        match self.try_connect(transport, protocol_order) {
            Ok(()) => true,
            Err(error) => {
                if report_failure {
                    self.connect_failed(error);
                } else {
                    self.log(error);
                }
                false
            }
        }
    }

    /// Hands a ready transport to the ELM327 driver; on success reports the
    /// connection, on failure only notes what the driver found out.
    pub(super) fn try_connect(
        &mut self,
        transport: Box<dyn Transport>,
        protocol_order: &str,
    ) -> Result<(), String> {
        if let Some(kbit) = self.truck_request {
            return self.try_connect_truck(transport, kbit);
        }
        self.facts.port_opened = Some(true);
        self.facts.port_busy = false;
        let order = prefer_protocol(protocol_order, &self.prefer_protocol);
        let result = self.elm.connect(transport, &order);
        // When several ports or rates are tried, the attempt where the
        // adapter answered says the most; a silent one does not replace it.
        let trace = self.elm.connect_trace().clone();
        if trace.adapter_answered || self.facts.adapter_answered != Some(true) {
            self.facts.adapter_answered = Some(trace.adapter_answered);
            self.facts.adapter_id = trace.adapter_id;
            self.facts.protocols_tried = trace.protocols_tried;
            self.facts.bus_error = trace.bus_error;
        }
        if let Err(error) = result {
            // Let go of the port, so the next baud rate (or the SLCAN probe)
            // can open it again.
            self.elm.disconnect();
            return Err(error);
        }
        // On CAN several control units may answer; with headers on, every
        // fault keeps the unit it came from.
        if self.elm.is_can() {
            self.elm.set_headers(true);
        }
        self.obd_ready = true;
        self.batch_refused.clear();
        self.uds_bus = StnBus::HighSpeed;
        // The extra CAN buses an OBDLink can switch to ("ms", "sw"), for the
        // control unit page.
        let stn_buses: Vec<&str> = self.elm.stn_extra_buses().iter().map(|b| b.key()).collect();
        let mut route = self.route.clone();
        let protocol = self.elm.protocol_id();
        if protocol != '0' {
            route.protocol = protocol.to_string();
        }
        self.emit(
            "connected",
            json!({
                "adapter": self.elm.version(),
                "transport": self.elm.transport_name(),
                "protocol": self.elm.protocol_text(),
                "rawCan": self.can_link.is_some(),
                "stnBuses": stn_buses,
                "route": route,
            }),
        );
        Ok(())
    }

    /// The way that worked last time for this car, then the search the user
    /// asked for. The second attempt only runs when no adapter answered on
    /// the first: an adapter that answered has already been through every
    /// protocol, the car is the problem there, and that is reported.
    pub(super) fn open_remembered(&mut self, route: ConnectRoute, then: Job) {
        self.log(args(
            l(
                "Versuche zuerst den Weg, der zuletzt ging: %1",
                "Trying the way that worked last time first: %1",
            ),
            &[&route_text(&route)],
        ));
        self.prefer_protocol = route.protocol.clone();
        self.quiet = Some(None);
        // The usual rates when the remembered one is unknown.
        let baud = i32::try_from(route.baud).unwrap_or(0);
        match route.kind.as_str() {
            "serial" if !route.device.is_empty() => {
                self.open_serial(&route.device, baud, &protocol_order_of(&then))
            }
            "tcp" => {
                let (host, port) = route
                    .device
                    .rsplit_once(':')
                    .and_then(|(h, p)| p.parse().ok().map(|p| (h.to_string(), p)))
                    .unwrap_or((route.device.clone(), 35000));
                self.open_tcp(&host, port, &protocol_order_of(&then))
            }
            "ble" if !route.device.is_empty() => self.open_ble(
                &route.device,
                &route.name,
                route.kinds,
                &protocol_order_of(&then),
            ),
            "can" if !route.device.is_empty() => self.open_can(&route.device),
            "enet" => self.open_enet(&route.device),
            _ => {}
        }
        let error = self.quiet.take().flatten();
        if self.elm.connected() || self.truck_connected() {
            self.prefer_protocol.clear();
            return;
        }
        // A CAN interface always "answers" (its ELM front end is ours); a
        // bit rate chosen now that differs from the remembered one is
        // still worth its own try.
        let other_rate = matches!(&then, Job::OpenCan { spec } if *spec != route.device);
        if self.facts.adapter_answered == Some(true)
            && !other_rate
            && let Some(error) = error
        {
            self.prefer_protocol.clear();
            self.connect_failed(error);
            return;
        }
        self.log(l(
            "Der gemerkte Weg geht diesmal nicht, suche wie gewohnt.",
            "The remembered way does not work this time, searching as usual.",
        ));
        self.handle(then);
        self.prefer_protocol.clear();
    }

    pub(super) fn open_serial(&mut self, device: &str, baud: i32, protocol_order: &str) {
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Usb);

        // An empty device means: look for the adapter on every port. Only
        // the usual rates are tried there, otherwise a computer with many
        // ports would keep the user waiting for minutes.
        let searching = device.is_empty();
        let devices: Vec<String> = if searching {
            let list: Vec<String> = enumerate_ports().into_iter().map(|p| p.device).collect();
            self.facts.ports_found = Some(list.len());
            if list.is_empty() {
                self.busy(false);
                self.connect_failed(w(
                    "Kein Anschluss gefunden. Steckt der Adapter am Rechner, und ist ein Bluetooth-Adapter gekoppelt?",
                    "No port found. Is the adapter plugged into the computer, and is the Bluetooth adapter paired?",
                ));
                return;
            }
            list
        } else {
            vec![device.to_string()]
        };
        let bauds: Vec<u32> = if baud > 0 {
            vec![baud as u32]
        } else if searching {
            vec![38400, 115200, 9600]
        } else {
            vec![38400, 9600, 115200, 500000]
        };

        let mut error = String::new();
        let mut any_opened = false;
        for candidate_device in &devices {
            if searching {
                self.log(args(
                    &w("Probiere %1 ...", "Trying %1 ..."),
                    &[candidate_device],
                ));
            }
            for (i, &b) in bauds.iter().enumerate() {
                let mut candidate = SerialTransport::new();
                if let Err(e) = candidate.open(candidate_device, b) {
                    // While searching, ports that do not open (paired
                    // Bluetooth ports out of reach) say nothing about the
                    // adapter; one that opened before counts.
                    if !(searching && any_opened) {
                        self.port_failed(&e);
                    }
                    error = e;
                    break;
                }
                any_opened = true;
                if !self.facts.bauds_tried.contains(&b) {
                    self.facts.bauds_tried.push(b);
                }
                self.log(args(
                    &w("Anschluss %1 offen bei %2 Baud", "Port %1 open at %2 baud"),
                    &[candidate_device, &b],
                ));
                self.route = ConnectRoute {
                    kind: "serial".into(),
                    device: candidate_device.clone(),
                    baud: b,
                    ..ConnectRoute::default()
                };
                match self.try_connect(Box::new(candidate), protocol_order) {
                    Ok(()) => {
                        if searching {
                            self.emit("adapter_found", json!({ "device": candidate_device }));
                        }
                        self.busy(false);
                        self.refresh_status_and_faults();
                        return;
                    }
                    Err(e) => {
                        // The adapter understood this rate, the car is the
                        // problem: other rates will not help. While
                        // searching, the next port is tried.
                        let stop = searching || self.elm.connect_trace().adapter_answered;
                        // The last error of a chosen port goes out with the
                        // failure, the others only into the log.
                        if searching || (!stop && i + 1 < bauds.len()) {
                            self.log(e.clone());
                        }
                        error = e;
                        if stop {
                            break;
                        }
                    }
                }
            }
            if searching {
                // No ELM327 here; perhaps a CAN stick with the SLCAN protocol.
                let mut serial = SerialTransport::new();
                if serial.open(candidate_device, 115200).is_ok()
                    && probe_slcan(&mut serial).is_some()
                {
                    self.log(args(
                        &w("SLCAN-Adapter an %1 gefunden", "SLCAN adapter found on %1"),
                        &[candidate_device],
                    ));
                    let mut link = SlcanLink::new(Box::new(serial));
                    if link.open(500000).is_ok() {
                        let front = CanElm::new(
                            Box::new(link),
                            format!("SLCAN {candidate_device}"),
                            500000,
                        );
                        self.can_link = Some(front.shared_link());
                        self.route = ConnectRoute {
                            kind: "can".into(),
                            device: format!("slcan:{candidate_device}@500000"),
                            ..ConnectRoute::default()
                        };
                        if self.finish_connect(Box::new(front), "67", false) {
                            self.emit("adapter_found", json!({ "device": candidate_device }));
                            self.busy(false);
                            self.refresh_status_and_faults();
                            return;
                        }
                        self.can_link = None;
                    }
                }
            }
            if !searching {
                self.busy(false);
                self.connect_failed(error);
                return;
            }
        }
        self.busy(false);
        self.connect_failed(if searching {
            w(
                "An keinem Anschluss hat ein ELM327 geantwortet. Zündung an und Adapter richtig eingesteckt?",
                "No ELM327 answered on any port. Is the ignition on and the adapter properly plugged in?",
            )
        } else {
            error
        });
    }

    pub(super) fn open_tcp(&mut self, host: &str, port: i32, protocol_order: &str) {
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Wifi);
        let mut transport = TcpTransport::new();
        if let Err(error) = transport.open(host, port.clamp(0, 65535) as u16) {
            self.busy(false);
            self.port_failed(&error);
            self.connect_failed(error);
            return;
        }
        self.route = ConnectRoute {
            kind: "tcp".into(),
            device: format!("{host}:{port}"),
            ..ConnectRoute::default()
        };
        let ok = self.finish_connect(Box::new(transport), protocol_order, true);
        self.busy(false);
        if ok {
            self.refresh_status_and_faults();
        }
    }

    pub(super) fn scan_ble(&mut self, milliseconds: i32) {
        self.busy(true);
        let (devices, error) = ble::scan(milliseconds);
        self.busy(false);
        self.emit(
            "ble_scan_finished",
            json!({ "devices": devices, "error": error }),
        );
    }

    pub(super) fn open_ble(&mut self, id: &str, name: &str, kinds: i32, protocol_order: &str) {
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Bluetooth);
        let candidates = if id.is_empty() {
            self.log(w(
                "Suche Bluetooth-Adapter ...",
                "Searching for Bluetooth adapters ...",
            ));
            match ble::candidates() {
                Ok(list) if !list.is_empty() => {
                    self.facts.device_found = Some(true);
                    list
                }
                Ok(_) => {
                    self.busy(false);
                    self.facts.device_found = Some(false);
                    self.connect_failed(w(
                        "Kein Bluetooth-Adapter in der Nähe gefunden. Steckt er im Auto, und ist die Zündung an?",
                        "No Bluetooth adapter found nearby. Is it plugged into the car, and is the ignition on?",
                    ));
                    return;
                }
                Err(error) => {
                    self.busy(false);
                    self.connect_failed(error);
                    return;
                }
            }
        } else {
            vec![ble::device(id, name, kinds)]
        };

        let mut error = String::new();
        for d in candidates {
            let shown = if d.name.is_empty() {
                d.id.clone()
            } else {
                d.name.clone()
            };
            // Bluetooth LE first, a dual-mode adapter then also gets the
            // classic serial port as a second try.
            let mut transport: Option<Box<dyn Transport>> = None;
            if d.low_energy {
                self.log(args(
                    &w(
                        "Verbinde mit %1 über Bluetooth LE ...",
                        "Connecting to %1 over Bluetooth LE ...",
                    ),
                    &[&shown],
                ));
                match ble::open(&d) {
                    Ok((t, profile)) => {
                        self.log(args(
                            &w("Kanal gefunden (%1)", "Channel found (%1)"),
                            &[&profile],
                        ));
                        transport = Some(t);
                    }
                    Err(e) => {
                        self.log(e.clone());
                        error = e;
                    }
                }
            }
            if transport.is_none() && d.classic {
                self.log(args(
                    &w(
                        "Verbinde mit %1 über klassisches Bluetooth ...",
                        "Connecting to %1 over classic Bluetooth ...",
                    ),
                    &[&shown],
                ));
                match ble::open_classic(&d) {
                    Ok(t) => transport = Some(t),
                    Err(e) => {
                        self.log(e.clone());
                        error = e;
                    }
                }
            }
            let Some(transport) = transport else {
                if self.facts.port_opened.is_none() {
                    self.facts.port_opened = Some(false);
                }
                continue;
            };
            self.route = ConnectRoute {
                kind: "ble".into(),
                device: d.id.clone(),
                name: d.name.clone(),
                kinds: ble::kinds(&d),
                ..ConnectRoute::default()
            };
            match self.try_connect(transport, protocol_order) {
                Ok(()) => {
                    self.emit(
                        "ble_adapter_found",
                        json!({ "id": d.id, "name": d.name, "kinds": ble::kinds(&d) }),
                    );
                    self.busy(false);
                    self.refresh_status_and_faults();
                    return;
                }
                Err(e) => {
                    self.log(e.clone());
                    error = e;
                }
            }
        }
        self.busy(false);
        self.connect_failed(error);
    }

    pub(super) fn open_enet(&mut self, host: &str) {
        // ENET is always CAN behind the front end.
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Network);
        if host.is_empty() {
            self.log(w(
                "Suche das Auto im Netzwerk (ENET und DoIP) ...",
                "Searching for the car on the network (ENET and DoIP) ...",
            ));
        }
        let car = match open_network_car(host) {
            Ok(car) => car,
            Err(error) => {
                self.busy(false);
                self.facts.port_opened = Some(false);
                self.connect_failed(error);
                return;
            }
        };
        let vin = if car.vin.is_empty() {
            String::new()
        } else {
            format!(", VIN {}", car.vin)
        };
        self.log(args(
            &w("Verbunden über %1: %2%3", "Connected over %1: %2%3"),
            &[&car.kind, &car.transport.name(), &vin],
        ));
        self.route = ConnectRoute {
            kind: "enet".into(),
            device: host.to_string(),
            ..ConnectRoute::default()
        };
        let ok = self.finish_connect(Box::new(car.transport), "6", true);
        self.busy(false);
        if ok {
            self.refresh_status_and_faults();
        }
    }

    pub(super) fn open_can(&mut self, spec: &str) {
        if let Some(kbit) = self.truck_request {
            self.open_can_truck(spec, kbit);
            return;
        }
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Can);
        // "j2534:<dll>@500000": the bit rate follows the last @.
        self.facts.can_bit_rate = spec
            .rsplit_once('@')
            .and_then(|(_, rate)| rate.trim().parse().ok())
            .unwrap_or(500_000);
        let opener = |device: &str, baud: u32| -> Result<Box<dyn Transport>, String> {
            let mut serial = SerialTransport::new();
            serial.open(device, baud)?;
            Ok(Box::new(serial))
        };
        let transport = match open_can_interface(spec, &opener) {
            Ok(t) => t,
            Err(error) => {
                self.busy(false);
                self.port_failed(&error);
                self.connect_failed(error);
                return;
            }
        };
        self.log(args(
            &w("CAN-Schnittstelle offen: %1", "CAN interface open: %1"),
            &[&transport.name()],
        ));
        // TP 2.0 for older VAG cars uses the same interface.
        self.can_link = Some(transport.shared_link());
        self.route = ConnectRoute {
            kind: "can".into(),
            device: spec.to_string(),
            ..ConnectRoute::default()
        };
        // 11 bit first, then 29 bit; a car that answers neither is listened to.
        let ok = self.finish_connect(Box::new(transport), "67", true);
        if !ok {
            self.can_link = None;
        }
        self.busy(false);
        if ok {
            self.refresh_status_and_faults();
        }
    }

    pub(super) fn open_demo(&mut self) {
        if let Some(kbit) = self.truck_request {
            self.open_demo_truck(kbit);
            return;
        }
        // The demo car speaks CAN; the K-line protocols would only cost seconds.
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Usb);
        self.route = ConnectRoute {
            kind: "demo".into(),
            ..ConnectRoute::default()
        };
        // CARUSPEX_DEMO_ADAPTER=obdlink-mx lets the demo car answer like an
        // OBDLink MX+ (MS-CAN and SW-CAN), for pictures of those buses.
        let adapter = match std::env::var("CARUSPEX_DEMO_ADAPTER").as_deref() {
            Ok("obdlink-mx") => caruspex_demo::DemoAdapter::ObdLinkMxPlus,
            Ok("obdlink-ex") => caruspex_demo::DemoAdapter::ObdLinkEx,
            _ => caruspex_demo::DemoAdapter::default(),
        };
        let demo = caruspex_demo::DemoTransport::new(caruspex_demo::DemoOptions {
            adapter,
            ..caruspex_demo::DemoOptions::default()
        });
        let ok = self.finish_connect(Box::new(demo), "6", true);
        self.busy(false);
        if ok {
            self.refresh_status_and_faults();
        }
    }

    pub(super) fn open_replay(&mut self, path: &str, real_time: bool, protocol_order: &str) {
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Replay);
        let replay = ReplayTransport::new(
            path,
            if real_time {
                ReplayPacing::RealTime
            } else {
                ReplayPacing::Fast
            },
        );
        if !replay.is_open() {
            self.busy(false);
            self.failed(args(
                &w(
                    "Aufnahme lässt sich nicht öffnen: %1",
                    "The recording cannot be opened: %1",
                ),
                &[&replay.error()],
            ));
            return;
        }
        self.log(recording_summary(&replay.info()));
        let order = replay.protocol_order(protocol_order);
        self.route = ConnectRoute {
            kind: "replay".into(),
            ..ConnectRoute::default()
        };
        let ok = self.finish_connect(Box::new(replay), &order, true);
        self.busy(false);
        if ok {
            self.refresh_status_and_faults();
        }
    }

    pub(super) fn start_recording(&mut self, path: &str) {
        if !self.elm.connected() {
            self.emit(
                "recording_state",
                json!({ "active": false, "path": path, "exchanges": 0,
                        "error": w("Erst verbinden, dann aufzeichnen.", "Connect first, then record.") }),
            );
            return;
        }
        match start_recording(&mut self.elm, path) {
            Ok(_) => self.emit(
                "recording_state",
                json!({ "active": true, "path": path, "exchanges": 0, "error": "" }),
            ),
            Err(error) => self.emit(
                "recording_state",
                json!({ "active": false, "path": path, "exchanges": 0, "error": error }),
            ),
        }
    }

    pub(super) fn stop_recording(&mut self) {
        let (exchanges, path) = active_recording(&self.elm)
            .map(|h| (h.exchanges(), h.path().to_string()))
            .unwrap_or_default();
        stop_recording(&mut self.elm);
        self.emit(
            "recording_state",
            json!({ "active": false, "path": path, "exchanges": exchanges, "error": "" }),
        );
    }

    pub(super) fn close_adapter(&mut self) {
        self.stop_live();
        self.forget_can_link();
        self.elm.disconnect();
        self.obd_ready = false;
        self.emit("closed", json!({}));
    }
}
