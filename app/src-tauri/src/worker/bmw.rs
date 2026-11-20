// BMW's E series on the K-line: the scan over every known module address
// (DS2, KWP2000 in BMW style, BMW-FAST) and clearing one unit or all of
// them, through a K+DCAN cable switched to K-line, a KKL cable or the
// simulated E46 and E60 of caruspex-demo. Every job opens the cable, does
// its work and closes it again; nothing stays open between two jobs.
use std::sync::Arc;
use std::sync::atomic::Ordering;

use caruspex_demo::bmw::{BmwCar, BmwSimLine};
use caruspex_i18n::l;
use caruspex_link::{ByteStream, KLineStream};
use caruspex_protocols::bmw_kline::ScanEvent;
use caruspex_protocols::{BmwKLine, BmwProtocol, BmwUnit};
use serde::Serialize;
use serde_json::json;

use super::{J2534_DEVICE_PREFIX, TP20_DEVICE, Worker};
use crate::texts::args;

/// One entry of a BMW fault memory for the page. The page writes "active"
/// or "stored" itself from `present`.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BmwFaultRow {
    pub code: String,
    /// `None` when the unit does not say (DS2).
    pub present: Option<bool>,
    pub detail: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BmwUnitRow {
    pub address: i32,
    /// "ds2", "kwp2000", "bmw-fast": what the clear job gets back.
    pub protocol: String,
    pub protocol_label: String,
    pub name: String,
    pub identification: String,
    pub faults_read: bool,
    pub faults: Vec<BmwFaultRow>,
    pub note: String,
}

fn unit_row(unit: &BmwUnit) -> BmwUnitRow {
    BmwUnitRow {
        address: i32::from(unit.address),
        protocol: unit.protocol.key().to_string(),
        protocol_label: unit.protocol.label().to_string(),
        name: unit.name.clone(),
        identification: unit.identification.clone(),
        faults_read: unit.faults_read,
        faults: unit
            .faults
            .iter()
            .map(|f| BmwFaultRow {
                code: f.code.clone(),
                present: f.present,
                detail: f.detail.clone(),
                text: f.text.clone(),
            })
            .collect(),
        note: unit.note.clone(),
    }
}

impl Worker {
    /// The line for a choice of the BMW page: a serial port with a K+DCAN
    /// or KKL cable, or "sim:e46" / "sim:e60" for the simulated cars,
    /// which keep their fault memory from job to job.
    fn open_bmw_line(&mut self, device: &str) -> Result<Box<dyn ByteStream>, String> {
        if let Some(car) = BmwCar::from_device(device) {
            if let Some((_, sim)) = self.bmw_sims.iter().find(|(c, _)| *c == car) {
                return Ok(Box::new(sim.connect()));
            }
            let sim = BmwSimLine::new(car);
            let line = sim.connect();
            self.bmw_sims.push((car, sim));
            return Ok(Box::new(line));
        }
        if device.is_empty() || device == TP20_DEVICE || device.starts_with(J2534_DEVICE_PREFIX) {
            return Err(l(
                "Für die BMW-K-Line braucht es ein K+DCAN-Kabel mit Schalter auf K-Line oder ein KKL-Kabel an einem COM-Anschluss.",
                "BMW's K-line needs a K+DCAN cable switched to K-line or a KKL cable on a COM port.",
            )
            .to_string());
        }
        let mut stream = KLineStream::new();
        stream.open_bmw(device)?;
        Ok(Box::new(stream))
    }

    fn bmw_link(&mut self, device: &str) -> Result<BmwKLine<Box<dyn ByteStream>>, String> {
        let line = self.open_bmw_line(device)?;
        let sink = self.sink.clone();
        let log: caruspex_protocols::KLineLog =
            Arc::new(move |m: &str| sink.emit("log_line", json!({ "line": m })));
        Ok(BmwKLine::new(line, Some(log)))
    }

    /// Asks every known address in every protocol that fits it. With
    /// `clear` (only after the page asked the user), every unit with
    /// entries is cleared and read again.
    pub(super) fn scan_bmw(&mut self, device: &str, clear: bool) {
        self.close_kkl();
        self.cancel.store(false, Ordering::SeqCst);
        let finished = |me: &Self, present: usize, with_faults: usize, error: &str| {
            me.emit(
                "bmw_scan_finished",
                json!({
                    "present": present,
                    "withFaults": with_faults,
                    "cancelled": me.cancelled(),
                    "error": error,
                }),
            );
        };
        let mut link = match self.bmw_link(device) {
            Ok(link) => link,
            Err(error) => {
                finished(self, 0, 0, &error);
                return;
            }
        };
        self.busy(true);
        self.log(args(
            l("BMW-K-Line über %1", "BMW K-line via %1"),
            &[&device],
        ));
        let sink = self.sink.clone();
        let cancel = self.cancel.clone();
        let mut total_sent = false;
        let result = link.scan(|event| {
            match event {
                ScanEvent::Trying {
                    index,
                    total,
                    protocol,
                    address,
                    name,
                } => {
                    if !total_sent {
                        sink.emit("bmw_scan_started", json!({ "total": total }));
                        total_sent = true;
                    }
                    sink.emit(
                        "bmw_scan_progress",
                        json!({
                            "index": index,
                            "total": total,
                            "name": name,
                            "protocol": protocol.label(),
                            "address": address,
                        }),
                    );
                }
                ScanEvent::Found(unit) => {
                    sink.emit("bmw_unit", json!({ "row": unit_row(unit) }));
                }
            }
            !cancel.load(Ordering::SeqCst)
        });
        let units = match result {
            Ok(units) => units,
            Err(error) => {
                self.busy(false);
                finished(self, 0, 0, &error);
                return;
            }
        };

        let mut with_faults = units.iter().filter(|u| !u.faults.is_empty()).count();
        if clear && !self.cancelled() {
            for unit in units.iter().filter(|u| !u.faults.is_empty()) {
                if self.cancelled() {
                    break;
                }
                let after = self.clear_one(&mut link, unit.protocol, unit.address);
                if after.is_some_and(|u| u.faults.is_empty()) {
                    with_faults -= 1;
                }
            }
        }
        drop(link.into_stream());
        self.busy(false);
        finished(self, units.len(), with_faults, "");
    }

    /// Clears one unit, reports how it went and sends the unit as it is
    /// afterwards.
    fn clear_one(
        &self,
        link: &mut BmwKLine<Box<dyn ByteStream>>,
        protocol: BmwProtocol,
        address: u8,
    ) -> Option<BmwUnit> {
        let result = link.clear_and_check(protocol, address);
        let (ok, remaining, error) = match &result {
            Ok(unit) => (true, unit.faults.len(), String::new()),
            Err(error) => (false, 0, error.clone()),
        };
        self.emit(
            "bmw_clear_result",
            json!({
                "address": address,
                "protocol": protocol.key(),
                "ok": ok,
                "remaining": remaining,
                "error": error,
            }),
        );
        let unit = result.ok()?;
        self.emit("bmw_unit", json!({ "row": unit_row(&unit) }));
        Some(unit)
    }

    /// Clears the unit at `address` that the scan found with `protocol`.
    /// The page asks the user before it sends this job.
    pub(super) fn clear_bmw_unit(&mut self, device: &str, address: i32, protocol: &str) {
        let Some(protocol) = BmwProtocol::from_key(protocol) else {
            return;
        };
        let address = address.clamp(0, 255) as u8;
        let mut link = match self.bmw_link(device) {
            Ok(link) => link,
            Err(error) => {
                self.emit(
                    "bmw_clear_result",
                    json!({
                        "address": address,
                        "protocol": protocol.key(),
                        "ok": false,
                        "remaining": 0,
                        "error": error,
                    }),
                );
                return;
            }
        };
        self.busy(true);
        self.clear_one(&mut link, protocol, address);
        drop(link.into_stream());
        self.busy(false);
    }
}
