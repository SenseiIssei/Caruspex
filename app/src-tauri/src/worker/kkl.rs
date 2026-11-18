// The VAG pages: KWP1281 over a KKL cable or the K-line of a J2534 device,
// and TP 2.0 on the CAN interface of the connection. Opening a control unit,
// keep-alive, basic settings, adaptation, actuator test and the search over
// all known units.
use std::sync::Arc;
use std::sync::atomic::Ordering;

use caruspex_i18n::l;
use caruspex_link::KLineStream;
use caruspex_link::timing::sleep_ms;
use caruspex_net::{KLineKind, Tp20, open_j2534_kline};
use caruspex_protocols::kwp1281::{ecu_name, known_ecus};
use caruspex_protocols::{Ecu, KLineProtocol, MeasuringValue, VagDiagnosis, VagKLine, VagTp20};
use serde_json::json;

use super::{J2534_DEVICE_PREFIX, TP20_DEVICE, Worker};
use crate::rows::*;
use crate::texts::{args, w};

/// Keep-alive of an open KKL or TP 2.0 session.
const KKL_KEEP_ALIVE_MS: u64 = 1000;

/// The fields of a measuring block as rows "Feld 1: ...". An invalid field
/// keeps its number, so the field numbers match the ones of the control unit.
fn measured_rows(values: &[MeasuringValue]) -> Vec<LiveRow> {
    let mut rows = Vec::new();
    for (i, m) in values.iter().enumerate() {
        let index = i as i32 + 1;
        if !m.valid {
            continue;
        }
        rows.push(LiveRow {
            pid: index,
            name: args(&w("Feld %1: %2", "Field %1: %2"), &[&index, &m.label]),
            unit: m.unit.clone(),
            value: m.value,
            ..LiveRow::default()
        });
    }
    rows
}

impl Worker {
    /// The VAG session for a choice of the port lists: a serial port with a
    /// KKL cable, "j2534:<driver>" for the K-line of a PassThru device, or
    /// "tp20" for TP 2.0 on the CAN interface of the current connection.
    fn open_vag_unit(
        &mut self,
        device: &str,
        log: Option<caruspex_protocols::KLineLog>,
    ) -> Result<Box<dyn VagDiagnosis>, String> {
        if device == TP20_DEVICE {
            let Some(shared) = self.can_link.clone() else {
                return Err(l(
                    "Für TP 2.0 erst über eine CAN-Schnittstelle (J2534, SLCAN, SocketCAN) mit dem Auto verbinden.",
                    "For TP 2.0, connect to the car through a CAN interface (J2534, SLCAN, SocketCAN) first.",
                )
                .to_string());
            };
            self.kkl_on_can = true;
            let tp20 = Tp20::new(Box::new(shared));
            self.log(args(
                l("TP 2.0 über %1", "TP 2.0 via %1"),
                &[&caruspex_link::MessageLink::name(&tp20)],
            ));
            return Ok(Box::new(VagTp20::new(tp20, log)));
        }
        self.kkl_on_can = false;
        if let Some(driver) = device.strip_prefix(J2534_DEVICE_PREFIX) {
            let (stream, name) = open_j2534_kline(driver, KLineKind::Iso9141, 9600)?;
            self.log(args(l("K-Line über %1", "K-line via %1"), &[&name]));
            return Ok(Box::new(VagKLine::new(stream, log)));
        }
        let mut stream = KLineStream::new();
        stream.open(device, 9600)?;
        Ok(Box::new(VagKLine::new(stream, log)))
    }

    pub(super) fn open_kkl(&mut self, device: &str, ecu_address: i32) {
        self.close_kkl();
        self.busy(true);
        self.kkl_message(
            if device == TP20_DEVICE {
                l(
                    "Öffne den TP-2.0-Kanal zum Steuergerät ...",
                    "Opening the TP 2.0 channel to the control unit ...",
                )
                .to_string()
            } else {
                w(
                    "Wecke das Steuergerät mit der 5-Baud-Adresse, das dauert etwa drei Sekunden ...",
                    "Waking the control unit with the 5-baud address, this takes about three seconds ...",
                )
            },
            false,
        );

        let sink = self.sink.clone();
        let log: caruspex_protocols::KLineLog =
            Arc::new(move |m: &str| sink.emit("log_line", json!({ "line": m })));
        let mut kwp = match self.open_vag_unit(device, Some(log)) {
            Ok(kwp) => kwp,
            Err(error) => {
                self.kkl_on_can = false;
                self.busy(false);
                self.kkl_message(error, true);
                return;
            }
        };
        // The configured session type must be set before basic settings can
        // be started.
        kwp.set_basic_setting_session(self.settings.basic_setting_session.clamp(0, 255) as u8);
        let ecu = Ecu::from_address(ecu_address.clamp(0, 255) as u8);
        if let Err(error) = kwp.connect(ecu) {
            drop(kwp);
            self.kkl_on_can = false;
            self.busy(false);
            self.emit(
                "kkl_connected",
                json!({ "connected": false, "identification": "" }),
            );
            self.kkl_message(error, true);
            return;
        }

        let identification: Vec<String> = kwp
            .identification()
            .iter()
            .map(|l| l.trim().to_string())
            .collect();
        let protocol_name = kwp.protocol_name();
        let is_kwp2000 = kwp.protocol() == KLineProtocol::Kwp2000;
        self.kwp = Some(kwp);
        // Without traffic the control unit closes the session after a few
        // seconds; basic settings keep it busy on their own.
        self.kkl_keep_alive_timer.set_interval(KKL_KEEP_ALIVE_MS);
        self.kkl_keep_alive_timer.start();
        self.busy(false);
        self.emit(
            "kkl_connected",
            json!({ "connected": true, "identification": identification.join("  |  ") }),
        );
        self.emit(
            "kkl_protocol",
            json!({ "protocolName": protocol_name, "isKwp2000": is_kwp2000 }),
        );
        self.kkl_message(
            args(
                &w(
                    "Verbunden mit %1 über %2. Vor dem Start die Punkte oben prüfen.",
                    "Connected to %1 via %2. Check the points above before starting.",
                ),
                &[&ecu_name(ecu), &protocol_name],
            ),
            false,
        );
        self.log(args(
            &w("KWP1281 verbunden: %1", "KWP1281 connected: %1"),
            &[&identification.join(" / ")],
        ));
    }

    pub(super) fn close_kkl(&mut self) {
        self.stop_basic_setting();
        self.kkl_keep_alive_timer.stop();
        // Signing off happens in disconnect, the port closes when it is dropped.
        if let Some(mut kwp) = self.kwp.take() {
            kwp.disconnect();
        }
        self.kkl_on_can = false;
        self.emit(
            "kkl_connected",
            json!({ "connected": false, "identification": "" }),
        );
    }

    pub(super) fn kkl_connected(&self) -> bool {
        self.kwp.as_ref().is_some_and(|k| k.connected())
    }

    /// The CAN interface of the connection goes away: a TP 2.0 session on it
    /// ends first.
    pub(super) fn forget_can_link(&mut self) {
        if self.kkl_on_can {
            self.close_kkl();
        }
        self.can_link = None;
        self.forget_truck();
    }

    pub(super) fn kkl_keep_alive_tick(&mut self) {
        if self.basic_setting_timer.active {
            return; // the measuring block requests keep it open
        }
        if !self.kkl_connected() {
            self.kkl_keep_alive_timer.stop();
            return;
        }
        if let Some(Err(error)) = self.kwp.as_mut().map(|k| k.keep_alive()) {
            self.close_kkl();
            self.kkl_message(error, true);
        }
    }

    pub(super) fn start_basic_setting(&mut self, group: i32) {
        if !self.kkl_connected() {
            self.kkl_message(
                w(
                    "Erst das KKL-Kabel verbinden.",
                    "Connect the KKL cable first.",
                ),
                true,
            );
            return;
        }
        self.basic_setting_group = group;
        // The control unit drops out of basic settings if nothing arrives for
        // about a second. We ask considerably more often.
        self.basic_setting_timer
            .set_interval(self.settings.basic_setting_interval_ms.max(0) as u64);
        self.basic_setting_timer.start();
        self.kkl_message(
            args(
                &w(
                    "Grundeinstellung im Messwertblock %1 läuft. Gaspedal nicht antippen.",
                    "Basic settings running in measuring block %1. Don't touch the accelerator pedal.",
                ),
                &[&group],
            ),
            false,
        );
        self.log(args(
            &w(
                "Grundeinstellung Block %1 gestartet",
                "Basic settings block %1 started",
            ),
            &[&group],
        ));
    }

    pub(super) fn stop_basic_setting(&mut self) {
        if self.basic_setting_timer.active {
            self.basic_setting_timer.stop();
            self.kkl_message(
                w("Grundeinstellung beendet.", "Basic settings finished."),
                false,
            );
            self.log(w("Grundeinstellung beendet", "Basic settings finished"));
        }
    }

    pub(super) fn basic_setting_tick(&mut self) {
        if !self.kkl_connected() {
            self.stop_basic_setting();
            return;
        }
        let group = self.basic_setting_group.clamp(0, 255) as u8;
        let result = self.kwp.as_mut().map(|k| k.basic_setting(group));
        match result {
            Some(Ok(measured)) => {
                let rows = measured_rows(&measured);
                self.emit("basic_setting_values", json!({ "values": rows }));
            }
            Some(Err(error)) => {
                self.stop_basic_setting();
                self.kkl_message(error, true);
            }
            None => self.stop_basic_setting(),
        }
    }

    // Searches all known control units one after another. Each attempt takes
    // a few seconds because of the 5-baud wake-up address; for control units
    // that are not fitted, the time spent waiting for a reply comes on top.
    pub(super) fn scan_all_ecus(&mut self, device: &str, clear_attempts: i32) {
        self.close_kkl();
        self.cancel.store(false, Ordering::SeqCst);

        let mut probe = match self.open_vag_unit(device, None) {
            Ok(probe) => probe,
            Err(error) => {
                self.kkl_on_can = false;
                self.kkl_message(error, true);
                self.emit(
                    "scan_finished",
                    json!({ "present": 0, "withFaults": 0, "cancelled": false }),
                );
                return;
            }
        };

        let list = known_ecus();
        let total = list.len();
        self.emit("scan_started", json!({ "total": total }));
        self.busy(true);

        let mut present = 0i32;
        let mut with_faults = 0i32;
        for (i, &ecu) in list.iter().enumerate() {
            if self.cancelled() {
                break;
            }
            let index = i + 1;
            let mut row = EcuScanRow {
                address: i32::from(ecu.address()),
                name: ecu_name(ecu).to_string(),
                ..EcuScanRow::default()
            };
            self.emit(
                "scan_progress",
                json!({
                    "ecuName": row.name,
                    "index": index,
                    "total": total,
                    "address": row.address,
                }),
            );

            if probe.connect(ecu).is_err() {
                row.present = false;
                row.note = w(
                    "meldet sich nicht, vermutlich nicht verbaut",
                    "does not respond, probably not fitted",
                );
                self.emit("scan_ecu_result", json!({ "row": row }));
                continue;
            }

            row.present = true;
            present += 1;
            let identification: Vec<String> = probe
                .identification()
                .iter()
                .map(|l| l.trim().to_string())
                .collect();
            row.identification = format!(
                "[{}]  {}",
                probe.protocol_name(),
                identification.join("  |  ")
            );

            match probe.read_faults() {
                Ok(faults) => {
                    for f in &faults {
                        row.faults.push(FaultRow {
                            code: format!("{:05}", f.code),
                            kind: if f.sporadic {
                                w("sporadisch", "intermittent")
                            } else {
                                w("gespeichert", "stored")
                            },
                            text: f.text.clone(),
                            throttle_related: false,
                            stored: !f.sporadic,
                            permanent: false,
                            ecu: String::new(),
                        });
                    }
                    if !row.faults.is_empty() {
                        with_faults += 1;
                    }
                    row.note = if row.faults.is_empty() {
                        w("kein Eintrag", "no entries")
                    } else {
                        args(&w("%1 Einträge", "%1 entries"), &[&row.faults.len()])
                    };

                    // Persistent clear: read back after every attempt. Some
                    // control units only accept the clear command on the
                    // second try.
                    if clear_attempts > 0 && !faults.is_empty() {
                        let mut cleared = false;
                        let mut attempt = 1;
                        while attempt <= clear_attempts && !cleared {
                            self.emit(
                                "clear_progress",
                                json!({ "line": args(&w("%1: lösche, Versuch %2 von %3", "%1: clearing, attempt %2 of %3"), &[&row.name, &attempt, &clear_attempts]) }),
                            );
                            if let Err(error) = probe.clear_faults() {
                                self.emit(
                                    "clear_progress",
                                    json!({ "line": args(&w("   abgelehnt: %1", "   rejected: %1"), &[&error]) }),
                                );
                                sleep_ms(300);
                                attempt += 1;
                                continue;
                            }
                            sleep_ms(400);
                            if probe.read_faults().is_ok_and(|rest| rest.is_empty()) {
                                cleared = true;
                            }
                            attempt += 1;
                        }
                        if cleared {
                            row.faults.clear();
                            row.note = w("gelöscht", "cleared");
                            with_faults -= 1;
                            self.emit(
                                "clear_progress",
                                json!({ "line": args(&w("   %1: leer", "   %1: empty"), &[&row.name]) }),
                            );
                        } else {
                            row.note = args(
                                &w(
                                    "%1 Einträge, Löschen ohne Wirkung",
                                    "%1 entries, clearing had no effect",
                                ),
                                &[&row.faults.len()],
                            );
                            self.emit(
                                "clear_progress",
                                json!({ "line": args(&w("   %1: bleibt bestehen, die Ursache ist noch da", "   %1: remains, the cause is still present"), &[&row.name]) }),
                            );
                        }
                    }
                }
                Err(error) => row.note = error,
            }
            probe.disconnect();
            self.emit("scan_ecu_result", json!({ "row": row }));
        }

        drop(probe);
        self.kkl_on_can = false;
        self.busy(false);
        self.emit(
            "scan_finished",
            json!({ "present": present, "withFaults": with_faults, "cancelled": self.cancelled() }),
        );
    }

    pub(super) fn read_adaptation_channel(&mut self, channel: i32) {
        if !self.kkl_connected() {
            self.kkl_message(
                w(
                    "Erst das KKL-Kabel verbinden.",
                    "Connect the KKL cable first.",
                ),
                true,
            );
            return;
        }
        self.busy(true);
        let result = self
            .kwp
            .as_mut()
            .map(|k| k.read_adaptation(channel.clamp(0, 255) as u8));
        match result {
            Some(Ok((value, shown))) => {
                let rows = measured_rows(&shown);
                self.busy(false);
                self.emit(
                    "adaptation_result",
                    json!({ "channel": channel, "value": value, "shown": rows }),
                );
                self.kkl_message(
                    args(
                        &w(
                            "Anpassungskanal %1 steht auf %2.",
                            "Adaptation channel %1 is set to %2.",
                        ),
                        &[&channel, &value],
                    ),
                    false,
                );
            }
            Some(Err(error)) => {
                self.busy(false);
                self.kkl_message(error, true);
            }
            None => self.busy(false),
        }
    }

    pub(super) fn run_actuator_test(&mut self) {
        if !self.kkl_connected() {
            self.kkl_message(
                w(
                    "Erst das KKL-Kabel verbinden.",
                    "Connect the KKL cable first.",
                ),
                true,
            );
            return;
        }
        self.busy(true);
        let result = self.kwp.as_mut().map(|k| k.actuator_test());
        self.busy(false);
        match result {
            Some(Ok(actuator)) => self.emit("actuator_result", json!({ "actuator": actuator })),
            Some(Err(error)) => self.kkl_message(error, true),
            None => {}
        }
    }

    pub(super) fn set_basic_setting_session(&mut self, mode: i32) {
        if let Some(kwp) = self.kwp.as_mut() {
            kwp.set_basic_setting_session(mode.clamp(0, 255) as u8);
        }
        self.log(args(
            &w(
                "Sitzungsart für die Grundeinstellung: 0x%1",
                "Session type for basic settings: 0x%1",
            ),
            &[&format!("{mode:02X}")],
        ));
    }
}
