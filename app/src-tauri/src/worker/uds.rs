// Control units over UDS: the scan of every unit the family knows (also on
// the MS-CAN and SW-CAN of an OBDLink) and clearing a single unit.
use std::sync::atomic::Ordering;

use caruspex_data::ecu_addresses::{
    EcuDefinition, bmw_address_sweep, ecu_definitions, ecu_from_ids, ford_ms_can_ecus,
    generic_ecu_sweep, gm_single_wire_ecus,
};
use caruspex_link::StnBus;
use caruspex_protocols::{Uds, UdsEcu, find_present_ecus, scan_ecus};
use serde_json::json;

use super::Worker;
use crate::rows::*;
use crate::texts::{args, w};

fn uds_row(ecu: &UdsEcu, bus: StnBus) -> UdsEcuRow {
    UdsEcuRow {
        request: ecu.definition.request,
        response: ecu.definition.response,
        extended29: ecu.definition.extended29,
        kind: ecu.definition.kind as i32,
        name: ecu.definition.name.clone(),
        present: ecu.present,
        dtcs_read: ecu.dtcs_read,
        part_number: ecu.info.part_number.clone(),
        software_version: ecu.info.software_version.clone(),
        hardware_version: ecu.info.hardware_version.clone(),
        system_name: ecu.info.system_name.clone(),
        serial: ecu.info.serial.clone(),
        dtcs: ecu
            .dtcs
            .iter()
            .map(|d| UdsDtcRow {
                code: d.code.clone(),
                failure_text: d.failure_text.clone(),
                text: d.text.clone(),
                test_failed: d.test_failed,
                pending: d.pending,
                confirmed: d.confirmed,
                warning_lamp: d.warning_lamp,
            })
            .collect(),
        note: ecu.note.clone(),
        bus: bus.key().to_string(),
    }
}

impl Worker {
    pub(super) fn scan_uds(&mut self, family: &str) {
        let finished = |me: &Self,
                        present: usize,
                        with_faults: usize,
                        cancelled: bool,
                        note: String| {
            me.emit(
                "uds_scan_finished",
                json!({ "present": present, "withFaults": with_faults, "cancelled": cancelled, "note": note }),
            );
        };
        if !self.obd_ready {
            finished(self, 0, 0, false, w("Erst verbinden.", "Connect first."));
            return;
        }
        if !self.elm.is_can() {
            finished(
                self,
                0,
                0,
                false,
                w(
                    "Das Auto spricht kein CAN. UDS gibt es erst bei CAN-Autos, bei älteren VW und Audi gehen die Steuergeräte über das KKL-Kabel.",
                    "This car does not speak CAN. UDS only exists on CAN cars; on older VW and Audi the control units go through the KKL cable.",
                ),
            );
            return;
        }
        // "FORD@MS" and "GM@SW": the body units on the medium speed or the
        // single wire CAN, through an OBDLink that switches buses.
        let bus = match family {
            "FORD@MS" => StnBus::MediumSpeed,
            "GM@SW" => StnBus::SingleWire,
            _ => StnBus::HighSpeed,
        };
        self.stop_live();
        self.voltage_timer.stop();
        self.cancel.store(false, Ordering::SeqCst);
        self.busy(true);
        if bus != StnBus::HighSpeed
            && let Err(error) = self.elm.switch_stn_bus(bus)
        {
            finished(self, 0, 0, false, error);
            self.busy(false);
            return;
        }
        self.uds_bus = bus;

        // "BMW*" is the full sweep of the BMW diagnostic addresses.
        let mut list: Vec<EcuDefinition> = if family.is_empty() {
            generic_ecu_sweep()
        } else if family == "BMW*" {
            bmw_address_sweep()
        } else if bus == StnBus::MediumSpeed {
            ford_ms_can_ecus()
        } else if bus == StnBus::SingleWire {
            gm_single_wire_ecus()
        } else {
            ecu_definitions(family)
        };
        let generic = family.is_empty() || list.is_empty();
        if list.is_empty() {
            list = generic_ecu_sweep();
        }
        let total = list.len();
        self.emit(
            "uds_scan_started",
            json!({ "total": total, "generic": generic, "bus": bus.key() }),
        );

        // Over a raw CAN interface the missing units are found first, many
        // at once; the scan below then only reads the ones that answered.
        let presence = {
            let cancel = self.cancel.clone();
            let is_cancelled = move || cancel.load(Ordering::SeqCst);
            find_present_ecus(&mut self.elm, &list, Some(&is_cancelled))
        };

        // One unit at a time, so every answer reaches the page right away.
        let mut present = 0;
        let mut with_faults = 0;
        for (index, def) in list.iter().enumerate() {
            if self.cancelled() {
                break;
            }
            self.emit(
                "uds_scan_progress",
                json!({
                    "name": def.name,
                    "index": index,
                    "total": total,
                    "request": def.request,
                    "kind": def.kind as i32,
                }),
            );
            if presence.as_ref().is_some_and(|found| !found[index]) {
                continue;
            }
            let cancel = self.cancel.clone();
            let is_cancelled = move || cancel.load(Ordering::SeqCst);
            let result = scan_ecus(
                &mut self.elm,
                std::slice::from_ref(def),
                None,
                Some(&is_cancelled),
            );
            let Some(ecu) = result.first().filter(|e| e.present) else {
                continue;
            };
            present += 1;
            if !ecu.dtcs.is_empty() {
                with_faults += 1;
            }
            self.emit("uds_ecu_result", json!({ "row": uds_row(ecu, bus) }));
        }
        self.back_to_high_speed();
        finished(self, present, with_faults, self.cancelled(), String::new());
        self.busy(false);
    }

    /// After work on the MS-CAN or SW-CAN of an OBDLink: back to the bus of
    /// the OBD services, so live values and fault codes work again.
    fn back_to_high_speed(&mut self) {
        if self.elm.stn_bus() == StnBus::HighSpeed {
            return;
        }
        if let Err(error) = self.elm.switch_stn_bus(StnBus::HighSpeed) {
            self.log(error);
        }
    }

    pub(super) fn clear_uds_ecu(
        &mut self,
        request: u32,
        response: u32,
        extended29: bool,
        name: &str,
    ) {
        if !self.obd_ready {
            return;
        }
        self.stop_live();
        self.busy(true);
        let def = ecu_from_ids(request, response, extended29, name);
        // A unit found on the MS-CAN or SW-CAN is cleared there.
        if self.uds_bus != StnBus::HighSpeed
            && let Err(error) = self.elm.switch_stn_bus(self.uds_bus)
        {
            self.emit(
                "uds_clear_result",
                json!({ "name": name, "ok": false, "message": error }),
            );
            self.busy(false);
            return;
        }
        let result = {
            let mut uds = Uds::new(&mut self.elm);
            uds.select(&def).and_then(|_| uds.clear_dtcs())
        };
        let message = match &result {
            Ok(()) => args(
                &w(
                    "Fehlerspeicher von %1 gelöscht.",
                    "Fault memory of %1 cleared.",
                ),
                &[&name],
            ),
            Err(error) => args(
                &w(
                    "Löschen bei %1 hat nicht geklappt: %2",
                    "Clearing %1 did not work: %2",
                ),
                &[&name, error],
            ),
        };
        self.emit(
            "uds_clear_result",
            json!({ "name": name, "ok": result.is_ok(), "message": message }),
        );
        // Read that unit again so the page shows what is left.
        let never = || false;
        let again = scan_ecus(
            &mut self.elm,
            std::slice::from_ref(&def),
            None,
            Some(&never),
        );
        if let Some(ecu) = again.first().filter(|e| e.present) {
            let mut row = uds_row(ecu, self.uds_bus);
            row.name = name.to_string();
            self.emit("uds_ecu_result", json!({ "row": row }));
        }
        self.back_to_high_speed();
        self.busy(false);
    }
}
