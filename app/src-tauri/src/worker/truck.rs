// Trucks and buses over SAE J1939: connecting (an ELM327 in protocol A, a
// raw CAN interface, the demo truck), the faults of every unit with their
// lamps, the broadcast values, VIN and identification, and clearing.
//
// The session lives next to the ELM327 driver in the worker; the bus is
// either that driver (switched to J1939) or the raw CAN interface of the
// connection. Clearing only runs through the clear_faults job, which the
// interface sends after the user confirmed it, the same way as OBD mode 04.
use std::sync::Arc;
use std::time::Instant;

use caruspex_advice::connect_help::LinkKind;
use caruspex_demo::{DemoOptions, DemoTransport, DemoVehicle};
use caruspex_i18n::l;
use caruspex_link::{FrameLink, SerialTransport, Transport};
use caruspex_net::{J1939Can, open_can_interface};
use caruspex_protocols::j1939::params::{Reading, find_param};
use caruspex_protocols::j1939::{
    ComponentId, J1939Session, J1939Unit, UnitFaults, address_label, describe,
};
use serde::Serialize;
use serde_json::json;

use super::{Worker, app_seconds};
use crate::rows::{ConnectRoute, FaultRow, LiveRow, StatusRow};
use crate::texts::args;

/// How the J1939 bus of the connection is reached.
pub(super) enum TruckLink {
    /// The worker's ELM327, switched to protocol A or B.
    Elm,
    /// A raw CAN interface.
    Can(J1939Can),
}

/// How long the live loop listens per tick.
pub(super) const LISTEN_MS: u64 = 700;
/// How long a raw CAN interface is listened to before a bit rate counts.
const PROBE_MS: u64 = 1500;

/// The rates to try for what the interface asked: 0 is automatic.
fn rates_for(kbit: u32) -> Vec<u32> {
    match kbit {
        250 | 500 => vec![kbit],
        _ => vec![250, 500],
    }
}

/// One value of the live page.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ValueRow {
    key: i32,
    spn: u32,
    name: String,
    about: String,
    unit: String,
    value: f64,
    /// "value", or "error" when the unit reports its measurement as faulty.
    state: &'static str,
    source: u8,
    source_name: String,
    min: f64,
    max: f64,
    time: f64,
}

/// The values that also feed the tiles of the start page and the trip
/// computer, under the PID they have in OBD.
const AS_OBD_PID: [(u32, i32); 6] = [
    (190, 0x0C),
    (84, 0x0D),
    (110, 0x05),
    (96, 0x2F),
    (168, 0x42),
    (917, 0xA6),
];

/// DM1 and DM2 of every unit, with every unit heard.
pub(super) struct TruckFaults {
    pub active: Vec<UnitFaults>,
    pub previous: Vec<UnitFaults>,
    pub units: Vec<J1939Unit>,
}

impl Worker {
    pub(super) fn truck_connected(&self) -> bool {
        self.truck.is_some()
    }

    /// Runs `f` with the session and the bus, if connected to a truck.
    pub(super) fn with_truck<R>(
        &mut self,
        f: impl FnOnce(&mut J1939Session, &mut dyn FrameLink) -> R,
    ) -> Option<R> {
        let Worker {
            truck,
            truck_session,
            elm,
            ..
        } = self;
        let link: &mut dyn FrameLink = match truck.as_mut()? {
            TruckLink::Elm => elm,
            TruckLink::Can(can) => can,
        };
        Some(f(truck_session, link))
    }

    fn new_truck_session(&self) -> J1939Session {
        let mut session = J1939Session::new();
        let sink = self.sink.clone();
        session.set_log(Box::new(move |m: &str| {
            sink.emit("log_line", json!({ "line": m }));
        }));
        session
    }

    pub(super) fn forget_truck(&mut self) {
        self.truck = None;
    }

    fn truck_connected_event(&self, adapter: &str, transport: &str, protocol: &str) {
        let mut route = self.route.clone();
        route.protocol = "J1939".into();
        self.emit(
            "connected",
            json!({
                "adapter": adapter,
                "transport": transport,
                "protocol": protocol,
                "rawCan": self.can_link.is_some(),
                "stnBuses": Vec::<String>::new(),
                "route": route,
                "j1939": true,
            }),
        );
    }

    /// The ELM327 way: protocol A (or B for 500 kbit/s) on whatever the
    /// transport is (cable, Wi-Fi, Bluetooth, the demo).
    pub(super) fn try_connect_truck(
        &mut self,
        transport: Box<dyn Transport>,
        kbit: u32,
    ) -> Result<(), String> {
        self.facts.port_opened = Some(true);
        self.facts.port_busy = false;
        self.facts.j1939 = true;
        let result = self.elm.connect_j1939(transport, &rates_for(kbit));
        let trace = self.elm.connect_trace().clone();
        if trace.adapter_answered || self.facts.adapter_answered != Some(true) {
            self.facts.adapter_answered = Some(trace.adapter_answered);
            self.facts.adapter_id = trace.adapter_id;
            self.facts.protocols_tried = trace.protocols_tried;
            self.facts.bus_error = trace.bus_error;
        }
        if let Err(error) = result {
            self.elm.disconnect();
            return Err(error);
        }
        self.obd_ready = false;
        self.truck = Some(TruckLink::Elm);
        self.truck_session = self.new_truck_session();
        let (adapter, transport, protocol) = (
            self.elm.version().to_string(),
            self.elm.transport_name(),
            self.elm.protocol_text().to_string(),
        );
        self.truck_connected_event(&adapter, &transport, &protocol);
        Ok(())
    }

    /// A raw CAN interface: open it at the rate (or each rate in turn) and
    /// listen whether there is J1939 traffic.
    pub(super) fn open_can_truck(&mut self, spec: &str, kbit: u32) {
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Can);
        self.facts.j1939 = true;
        let base = spec.rsplit_once('@').map_or(spec, |(b, _)| b).to_string();
        let opener = |device: &str, baud: u32| -> Result<Box<dyn Transport>, String> {
            let mut serial = SerialTransport::new();
            serial.open(device, baud)?;
            Ok(Box::new(serial))
        };
        let mut last_error = String::new();
        for rate in rates_for(kbit) {
            let spec = format!("{base}@{}", rate * 1000);
            self.facts.can_bit_rate = rate * 1000;
            let front = match open_can_interface(&spec, &opener) {
                Ok(front) => front,
                Err(error) => {
                    self.port_failed(&error);
                    last_error = error;
                    break;
                }
            };
            self.facts.port_opened = Some(true);
            self.facts.adapter_answered = Some(true);
            let shared = front.shared_link();
            let name = Transport::name(&front);
            drop(front);
            let mut can = J1939Can::new(shared.clone());
            let mut session = self.new_truck_session();
            let heard = session.listen(&mut can, PROBE_MS).unwrap_or(0);
            self.log(args(
                l(
                    "J1939 auf %1: %2 Rahmen gehört.",
                    "J1939 on %1: %2 frames heard.",
                ),
                &[&name, &heard],
            ));
            if heard < 2 {
                last_error = l(
                    "Auf dem J1939-Bus ist nichts zu hören.\n  - Zündung an? Lkw-Steuergeräte schweigen bei Zündung aus.\n  - Steckt der Adapter über das Adapterkabel in der 9-poligen Diagnosebuchse?\n  - Schwarze Buchse (Typ 1): 250 kbit/s, grüne Buchse (Typ 2): 500 kbit/s.",
                    "Nothing can be heard on the J1939 bus.\n  - Ignition on? Truck control units stay silent with the ignition off.\n  - Is the adapter in the 9-pin diagnostic socket through the adapter cable?\n  - Black socket (type 1): 250 kbit/s, green socket (type 2): 500 kbit/s.",
                )
                .to_string();
                continue;
            }
            self.route = ConnectRoute {
                kind: "can".into(),
                device: spec.clone(),
                ..ConnectRoute::default()
            };
            self.can_link = Some(shared);
            self.truck = Some(TruckLink::Can(can));
            self.truck_session = session;
            self.obd_ready = false;
            let protocol = format!("SAE J1939 (29 {}, {rate} kbit/s)", l("Bit", "bit"));
            self.truck_connected_event("CAN", &name, &protocol);
            self.busy(false);
            self.refresh_truck_faults();
            return;
        }
        self.busy(false);
        self.connect_failed(last_error);
    }

    /// The demo truck, warm and on its drive cycle.
    pub(super) fn open_demo_truck(&mut self, kbit: u32) {
        self.busy(true);
        self.set_elm_log();
        self.begin_connect(LinkKind::Usb);
        self.route = ConnectRoute {
            kind: "demo".into(),
            ..ConnectRoute::default()
        };
        let origin = Instant::now();
        let options = DemoOptions {
            clock: Some(Arc::new(move || 600.0 + origin.elapsed().as_secs_f64())),
            vehicle: DemoVehicle::Truck,
            ..DemoOptions::default()
        };
        let result = self.try_connect_truck(Box::new(DemoTransport::new(options)), kbit);
        self.busy(false);
        match result {
            Ok(()) => self.refresh_truck_faults(),
            Err(error) => self.connect_failed(error),
        }
    }

    /// DM1 and DM2 of every unit: the fault list, the status row and the
    /// lamps per unit.
    pub(super) fn refresh_truck_faults(&mut self) {
        self.busy(true);
        if let Some(Err(error)) = self.load_truck_faults() {
            self.failed(error);
        }
        self.busy(false);
    }

    /// Reads DM1 and DM2 of every unit and sends the fault list, the status
    /// row and the lamps to the interface. `None` without a truck.
    pub(super) fn load_truck_faults(&mut self) -> Option<Result<TruckFaults, String>> {
        let result = self.with_truck(|s, link| -> Result<_, String> {
            let active = s.active_faults(link)?;
            let previous = s.previous_faults(link)?;
            Ok((active, previous, s.units()))
        })?;
        let (active, previous, units) = match result {
            Ok(found) => found,
            Err(error) => return Some(Err(error)),
        };
        let mut rows = Vec::new();
        let row = |unit: &UnitFaults, active: bool, rows: &mut Vec<FaultRow>| {
            for dtc in &unit.dtcs {
                let mut text = describe(dtc.spn, dtc.fmi);
                if let Some(count) = dtc.occurrences {
                    text.push_str(&format!(
                        " ({})",
                        args(l("%1-mal aufgetreten", "occurred %1 times"), &[&count])
                    ));
                }
                rows.push(FaultRow {
                    code: dtc.code(),
                    kind: if active {
                        l("aktiv", "active")
                    } else {
                        l("früher aktiv", "previously active")
                    }
                    .to_string(),
                    text,
                    throttle_related: false,
                    stored: active,
                    permanent: false,
                    ecu: address_label(unit.address),
                });
            }
        };
        for unit in &active {
            row(unit, true, &mut rows);
        }
        for unit in &previous {
            row(unit, false, &mut rows);
        }
        let active_count: usize = active.iter().map(|u| u.dtcs.len()).sum();
        let status = StatusRow {
            valid: true,
            mil: active.iter().any(|u| u.lamps.mil.on()),
            fault_count: active_count as i32,
            monitor_names: Vec::new(),
            monitor_ready: Vec::new(),
        };
        self.emit("status", json!({ "row": status }));
        self.emit("faults", json!({ "rows": rows }));
        let lamps: Vec<_> = units
            .iter()
            .map(|u| {
                let dm1 = active.iter().find(|a| a.address == u.address);
                let dm2 = previous.iter().find(|a| a.address == u.address);
                let lamps = dm1.map(|d| d.lamps).unwrap_or_default();
                json!({
                    "address": u.address,
                    "label": u.label,
                    "mil": lamps.mil.on(),
                    "redStop": lamps.red_stop.on(),
                    "amberWarning": lamps.amber_warning.on(),
                    "protect": lamps.protect.on(),
                    "active": dm1.map_or(0, |d| d.dtcs.len()),
                    "previous": dm2.map_or(0, |d| d.dtcs.len()),
                })
            })
            .collect();
        self.emit("j1939_units", json!({ "units": lamps }));
        self.emit("supported_pids", json!({ "pids": Vec::<i32>::new() }));
        self.log(args(
            l(
                "J1939: %1 Steuergeräte, %2 aktive und %3 frühere Fehler.",
                "J1939: %1 control units, %2 active and %3 previous faults.",
            ),
            &[
                &units.len(),
                &active_count,
                &previous.iter().map(|u| u.dtcs.len()).sum::<usize>(),
            ],
        ));
        Some(Ok(TruckFaults {
            active,
            previous,
            units,
        }))
    }

    /// DM11 (active) and DM3 (previously active) to every unit. Only after
    /// the user confirmed it in the interface.
    pub(super) fn clear_truck_faults(&mut self) {
        self.busy(true);
        let result = self.with_truck(|s, link| -> Result<_, String> {
            Ok((s.clear_active(link)?, s.clear_previous(link)?))
        });
        match result {
            Some(Ok((active, previous))) => {
                let refused = active
                    .acks
                    .iter()
                    .chain(previous.acks.iter())
                    .filter(|(_, a)| *a != caruspex_protocols::j1939::AckKind::Positive)
                    .count();
                self.log(if active.acks.is_empty() {
                    l(
                        "J1939: Kein Steuergerät hat das Löschen bestätigt.",
                        "J1939: no control unit confirmed the clearing.",
                    )
                    .to_string()
                } else if refused > 0 {
                    args(
                        l(
                            "J1939: Fehler gelöscht, %1 Antworten lehnten ab.",
                            "J1939: faults cleared, %1 answers refused.",
                        ),
                        &[&refused],
                    )
                } else {
                    l(
                        "J1939: Fehlerspeicher gelöscht.",
                        "J1939: fault memory cleared.",
                    )
                    .to_string()
                });
            }
            Some(Err(error)) => self.log(args(
                l("Löschen fehlgeschlagen: %1", "Clearing failed: %1"),
                &[&error],
            )),
            None => {}
        }
        self.busy(false);
        self.refresh_truck_faults();
    }

    /// VIN, identification of every unit and the counters that come only on
    /// request.
    pub(super) fn read_truck_details(&mut self) {
        self.busy(true);
        let result = self.with_truck(|s, link| -> Result<_, String> {
            let vin = s.read_vin(link)?;
            let components = s.component_ids(link)?;
            let software = s.software_ids(link)?;
            s.read_counters(link)?;
            Ok((vin, components, software))
        });
        match result {
            Some(Ok((vin, components, software))) => {
                self.emit(
                    "vehicle_info",
                    json!({
                        "vin": vin.unwrap_or_default(),
                        "freezeCode": "",
                        "freezeRows": Vec::<LiveRow>::new(),
                    }),
                );
                self.emit("freeze_frames", json!({ "frames": Vec::<LiveRow>::new() }));
                self.emit_truck_ids(&components, &software);
                self.emit_truck_values();
            }
            Some(Err(error)) => self.failed(error),
            None => {}
        }
        self.busy(false);
    }

    /// The identification of every unit that answered, for the unit cards.
    pub(super) fn emit_truck_ids(
        &self,
        components: &[(u8, ComponentId)],
        software: &[(u8, Vec<String>)],
    ) {
        let mut addresses: Vec<u8> = components.iter().map(|(a, _)| *a).collect();
        addresses.extend(software.iter().map(|(a, _)| *a));
        addresses.sort_unstable();
        addresses.dedup();
        let ids: Vec<_> = addresses
            .iter()
            .map(|a| {
                let c = components
                    .iter()
                    .find(|(x, _)| x == a)
                    .map(|(_, c)| c.clone())
                    .unwrap_or_default();
                let sw = software
                    .iter()
                    .find(|(x, _)| x == a)
                    .map(|(_, s)| s.clone())
                    .unwrap_or_default();
                json!({
                    "address": a,
                    "label": address_label(*a),
                    "make": c.make,
                    "model": c.model,
                    "serial": c.serial,
                    "unit": c.unit,
                    "software": sw,
                })
            })
            .collect();
        self.emit("j1939_ids", json!({ "units": ids }));
    }

    /// One tick of the live loop: listen, then everything decoded so far.
    pub(super) fn truck_tick(&mut self) {
        if let Some(Err(error)) = self.with_truck(|s, link| s.listen(link, LISTEN_MS)) {
            self.stop_live();
            self.failed(error);
            return;
        }
        self.emit_truck_values();
    }

    pub(super) fn emit_truck_values(&mut self) {
        let Some(values) = self.with_truck(|s, _| s.values()) else {
            return;
        };
        let time = app_seconds();
        let mut rows = Vec::new();
        for v in values {
            let (value, state) = match v.reading {
                Reading::Value(x) => (x, "value"),
                Reading::Error => (0.0, "error"),
                Reading::NotAvailable => continue,
            };
            let param = find_param(v.spn);
            if state == "value"
                && let Some(&(_, pid)) = AS_OBD_PID.iter().find(|(spn, _)| *spn == v.spn)
            {
                self.emit_live(LiveRow {
                    pid,
                    name: v.name.clone(),
                    unit: v.unit.clone(),
                    text: String::new(),
                    value,
                    warn: false,
                    time,
                    display_min: v.min,
                    display_max: v.max,
                });
            }
            rows.push(ValueRow {
                key: param.map_or(0, |p| p.key()),
                spn: v.spn,
                name: v.name,
                about: param.map(|p| p.about().to_string()).unwrap_or_default(),
                unit: v.unit,
                value,
                state,
                source: v.source,
                source_name: address_label(v.source),
                min: v.min,
                max: v.max,
                time,
            });
        }
        self.emit("j1939_values", json!({ "rows": rows }));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::AtomicBool;

    use caruspex_demo::DemoClock;
    use serde_json::Value;

    use super::*;
    use crate::settings::AppSettings;
    use crate::worker::{Job, Sink};

    #[derive(Default)]
    struct Collect(Mutex<Vec<(String, Value)>>);

    impl Sink for Collect {
        fn emit(&self, event: &str, payload: Value) {
            self.0.lock().unwrap().push((event.to_string(), payload));
        }
    }

    impl Collect {
        fn take(&self, name: &str) -> Vec<Value> {
            let mut all = self.0.lock().unwrap();
            let (wanted, rest): (Vec<_>, Vec<_>) = all.drain(..).partition(|(e, _)| e == name);
            *all = rest;
            wanted.into_iter().map(|(_, v)| v).collect()
        }
    }

    fn truck_worker() -> (Worker, Arc<Collect>) {
        let sink = Arc::new(Collect::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
        worker.apply_settings(AppSettings::default());
        let clock = DemoClock::new(600.0);
        let options = DemoOptions {
            clock: Some(clock.source()),
            latency_scale: 0.0,
            vehicle: DemoVehicle::Truck,
            ..DemoOptions::default()
        };
        worker.handle(Job::SetTruck(Some(0)));
        worker
            .try_connect(Box::new(DemoTransport::new(options)), "")
            .expect("the demo truck answers");
        worker.truck_session.set_waits(300, 700);
        (worker, sink)
    }

    #[test]
    fn faults_lamps_details_values_and_clearing() {
        let (mut worker, sink) = truck_worker();
        let connected = sink.take("connected");
        assert_eq!(connected[0]["j1939"], true);
        assert!(connected[0]["protocol"].as_str().unwrap().contains("J1939"));
        assert_eq!(connected[0]["route"]["protocol"], "J1939");

        worker.handle(Job::ReadFaults);
        let rows = sink.take("faults")[0]["rows"].as_array().unwrap().clone();
        let codes: Vec<&str> = rows.iter().map(|r| r["code"].as_str().unwrap()).collect();
        assert_eq!(
            codes,
            vec![
                "SPN 105 FMI 3",
                "SPN 111 FMI 17",
                "SPN 1761 FMI 18",
                "SPN 904 FMI 2"
            ]
        );
        assert_eq!(rows[0]["stored"], true);
        assert_eq!(rows[2]["stored"], false);
        assert!(rows[0]["ecu"].as_str().unwrap().ends_with("(00)"));
        let status = &sink.take("status")[0]["row"];
        assert_eq!(status["mil"], true);
        assert_eq!(status["faultCount"], 2);
        let units = sink.take("j1939_units")[0]["units"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(units.len(), 5);
        assert_eq!(units[0]["amberWarning"], true);
        assert_eq!(units[2]["previous"], 1);

        worker.handle(Job::ReadDetails);
        assert_eq!(
            sink.take("vehicle_info")[0]["vin"],
            DemoTransport::truck_vin()
        );
        let ids = sink.take("j1939_ids")[0]["units"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(ids[0]["model"], "CX-D13");
        let values = sink.take("j1939_values")[0]["rows"]
            .as_array()
            .unwrap()
            .clone();
        assert!(values.iter().any(|v| v["spn"] == 247));
        assert!(
            values
                .iter()
                .any(|v| v["spn"] == 105 && v["state"] == "error")
        );
        // Engine speed and the other tile values also go out as OBD values.
        let live = sink.take("live_value");
        assert!(live.iter().any(|v| v["row"]["pid"] == 0x0C));

        worker.handle(Job::StartLive(Vec::new()));
        assert!(worker.poll_timer.active);
        worker.truck_tick();
        assert!(!sink.take("j1939_values").is_empty());
        worker.handle(Job::StopLive);

        worker.handle(Job::ClearFaults);
        assert!(
            sink.take("faults")[0]["rows"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        worker.handle(Job::Close);
        assert!(!worker.truck_connected());
    }

    #[test]
    fn nothing_on_the_bus_fails_with_the_truck_checklist() {
        let sink = Arc::new(Collect::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
        worker.handle(Job::SetTruck(Some(500)));
        worker.begin_connect(LinkKind::Usb);
        let options = DemoOptions {
            latency_scale: 0.0,
            vehicle: DemoVehicle::Truck,
            ..DemoOptions::default()
        };
        let error = worker
            .try_connect(Box::new(DemoTransport::new(options)), "")
            .unwrap_err();
        assert!(error.contains("J1939"));
        assert!(worker.facts.j1939);
        worker.connect_failed(error);
        let failed = sink.take("failed");
        let ids: Vec<&str> = failed[0]["help"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|i| i["id"].as_str())
            .collect();
        assert!(ids.contains(&"j1939_plug"), "{ids:?}");
    }
}
