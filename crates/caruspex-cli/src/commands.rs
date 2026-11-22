//! The commands of caruspex_cli. Every text and every JSON field is the one
//! of the C++ version.

use caruspex_data::dtc::lookup_dtc;
use caruspex_data::severity::severity_label;
use caruspex_link::Elm327;
use caruspex_protocols::{Fault, LiveValue, Obd2, VehicleStatus};
use caruspex_record::JsonWriter;
use caruspex_vehicles::catalog::{self, CatalogMatch};
use caruspex_vehicles::vin::{VinInfo, decode_vin};

use crate::out::{ljust, say};

// One module per group of commands; the output helpers stay here.
mod adapter;
mod custom;
mod health;
mod kkl;
mod read;
mod uds;

pub use adapter::{ecus, info, probe};
pub use custom::{custom, list_obdb_packs, load_custom_pids, obdb_query};
pub use health::health;
pub(crate) use health::{HealthRun, run_health};
pub use kkl::kkl_check;
pub use read::{freeze, read, vin};
pub use uds::{parse_uds_ids, uds};

/// `printf("%.Nf")` for the rounding the C library does.
fn fixed(value: f64, digits: usize) -> String {
    format!("{value:.digits$}")
}

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

pub fn print_value(v: &LiveValue) {
    if !v.text.is_empty() {
        say(&format!("  {} {}\n", ljust(&v.name, 40), v.text));
    } else {
        say(&format!(
            "  {} {:>10.2} {}\n",
            ljust(&v.name, 40),
            v.value,
            v.unit
        ));
    }
}

fn json_value(json: &mut JsonWriter, v: &LiveValue) {
    json.begin_object()
        .field("pid", &v.pid)
        .field("key", &v.key)
        .field("sub", &v.sub)
        .field("name", &v.name)
        .field("value", &v.value)
        .field("unit", &v.unit);
    if !v.text.is_empty() {
        json.field("text", &v.text);
    }
    if !v.ecu_source.is_empty() {
        json.field("ecu", &v.ecu).field("ecuSource", &v.ecu_source);
    }
    json.end_object();
}

fn json_adapter(json: &mut JsonWriter, elm: &Elm327) {
    json.key("adapter")
        .begin_object()
        .field("version", elm.version())
        .field("connection", &elm.transport_name())
        .field("protocol", elm.protocol_text())
        .field("can", &elm.is_can())
        .end_object();
}

/// The brand behind the VIN, for manufacturer-specific code texts. Empty if
/// the car gives no VIN or the VIN names no single brand.
#[derive(Default)]
pub(crate) struct VinContext {
    pub vin: String,
    pub info: VinInfo,
    pub matched: CatalogMatch,
    pub brand: String,
}

pub(crate) fn read_vin_context(obd: &mut Obd2) -> VinContext {
    let mut c = VinContext {
        vin: obd.read_vin(),
        ..VinContext::default()
    };
    if c.vin.is_empty() {
        return c;
    }
    c.info = decode_vin(&c.vin, 0);
    c.matched = catalog::match_vin(&c.info);
    c.brand = if c.matched.brand.is_empty() {
        c.info.brand.clone()
    } else {
        c.matched.brand.clone()
    };
    c
}

/// The fault text, with the meaning for the brand where it differs.
fn fault_text(code: &str, brand: &str) -> String {
    lookup_dtc(code, brand).text
}

fn is_stored(kind: &str) -> bool {
    kind == caruspex_i18n::l("gespeichert", "stored")
}

fn json_faults(json: &mut JsonWriter, faults: &[Fault], brand: &str) {
    json.key("faults").begin_array();
    for f in faults {
        let info = lookup_dtc(&f.code, brand);
        json.begin_object()
            .field("code", &f.code)
            .field("kind", &f.kind)
            .field("text", &info.text)
            .field("severity", severity_label(info.severity))
            .field("system", &info.system_name);
        if !f.ecu_source.is_empty() {
            json.field("ecu", &f.ecu).field("ecuSource", &f.ecu_source);
        }
        json.end_object();
    }
    json.end_array();
}

fn json_status(json: &mut JsonWriter, status: &VehicleStatus) {
    json.key("status");
    if !status.valid {
        json.null();
        return;
    }
    json.begin_object()
        .field("milOn", &status.mil_on)
        .field("faultCount", &status.fault_count)
        .field("diesel", &status.compression_ignition)
        .key("readiness")
        .begin_array();
    for r in &status.readiness {
        json.begin_object()
            .field("name", &r.name)
            .field("ready", &r.ready)
            .end_object();
    }
    json.end_array().end_object();
}

/// An optional number as `value`, or `null`.
fn number_or_null(json: &mut JsonWriter, positive: bool, value: i32) {
    if positive {
        json.value(&value);
    } else {
        json.null();
    }
}

/// Prints an error as text, or as {"error": ...} with --json.
pub fn fail(as_json: bool, message: &str, code: i32) -> i32 {
    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object().field("error", message).end_object();
        say(&json.text());
    } else {
        say(&format!("{message}\n"));
    }
    code
}
