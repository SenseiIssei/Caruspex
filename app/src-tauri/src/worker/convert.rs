// From the results of the core to the rows the interface shows.
use caruspex_i18n::l;
use caruspex_protocols::{Fault, LiveValue, VehicleStatus};

use super::app_seconds;
use crate::rows::*;

pub(super) fn to_row(v: &LiveValue) -> LiveRow {
    // The key tells the values of one PID apart; it equals the PID for the
    // primary value, so everything keyed by PID keeps working.
    let pid = if v.key != 0 { v.key } else { v.pid };
    LiveRow {
        pid,
        name: crate::knowledge::live_name(pid, &v.name),
        unit: v.unit.clone(),
        text: v.text.clone(),
        value: v.value,
        warn: v.warn,
        time: app_seconds(),
        display_min: v.display_min,
        display_max: v.display_max,
        ..LiveRow::default()
    }
}

// The core reports the kind of a fault as display text in the active
// language. "stored" is the English wording the core uses for "gespeichert".
pub(super) fn is_stored_kind(kind: &str) -> bool {
    kind == "gespeichert" || kind == "stored" || kind == l("gespeichert", "stored")
}

// Same for "dauerhaft", the permanent codes of service 0A.
fn is_permanent_kind(kind: &str) -> bool {
    kind == "dauerhaft" || kind == "permanent" || kind == l("dauerhaft", "permanent")
}

pub(super) fn fault_row(f: &Fault) -> FaultRow {
    FaultRow {
        code: f.code.clone(),
        kind: f.kind.clone(),
        text: f.text.clone(),
        throttle_related: f.throttle_related,
        stored: is_stored_kind(&f.kind),
        permanent: is_permanent_kind(&f.kind),
        ecu: f.ecu.clone(),
    }
}

pub(super) fn status_row(st: &VehicleStatus) -> StatusRow {
    StatusRow {
        valid: st.valid,
        mil: st.mil_on,
        fault_count: st.fault_count,
        monitor_names: st.readiness.iter().map(|i| i.name.clone()).collect(),
        monitor_ready: st.readiness.iter().map(|i| i.ready).collect(),
    }
}
