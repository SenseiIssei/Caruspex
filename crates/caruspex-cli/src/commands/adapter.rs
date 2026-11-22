//! probe, info and ecus: the adapter and the control units on the bus.

use caruspex_link::adapter_probe::version_text;
use caruspex_link::{Elm327, probe_adapter};
use caruspex_protocols::{Obd2, misfire_counts};
use caruspex_record::JsonWriter;

use super::{fixed, json_adapter, print_value};
use crate::out::{ljust, say};

pub fn probe(elm: &mut Elm327, as_json: bool) -> i32 {
    let r = probe_adapter(elm);

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object();
        json_adapter(&mut json, elm);
        json.field("reachable", &r.reachable)
            .field("versionText", &r.version_text)
            .field("description", &r.description)
            .key("claimedVersion");
        if r.claimed_level > 0 {
            json.value(&version_text(r.claimed_level));
        } else {
            json.null();
        }
        json.field("claimedVersionExists", &r.claimed_version_exists)
            .key("behavesLike");
        if r.behaves_like_level > 0 {
            json.value(&version_text(r.behaves_like_level));
        } else {
            json.null();
        }
        json.field("acceptsUnknownCommands", &r.accepts_unknown_commands)
            .field("stn", &r.is_stn);
        if r.is_stn {
            json.field("stnId", &r.stn_id)
                .field("stnDevice", &r.stn_device);
        }
        json.key("voltage");
        if r.has_voltage {
            json.value(&r.voltage);
        } else {
            json.null();
        }
        json.key("roundTripMs");
        if r.round_trip_samples > 0 {
            json.value(&r.round_trip_ms);
        } else {
            json.null();
        }
        json.key("roundTripWithCountHintMs");
        if r.response_count_hint {
            json.value(&r.round_trip_with_hint_ms);
        } else {
            json.null();
        }
        json.key("checks").begin_array();
        for c in &r.checks {
            json.begin_object()
                .field("command", &c.command)
                .field("introducedIn", &version_text(c.introduced_in))
                .field("supported", &c.supported)
                .field("response", &c.response)
                .field("purpose", &c.purpose.get())
                .end_object();
        }
        json.end_array().key("facts").begin_array();
        for t in &r.facts {
            json.value(&t.get());
        }
        json.end_array().key("speedTips").begin_array();
        for t in &r.speed_tips {
            json.value(&t.get());
        }
        json.end_array()
            .field("verdict", &r.verdict.get())
            .end_object();
        say(&json.text());
        return if r.reachable { 0 } else { 3 };
    }

    if !r.reachable {
        say("The adapter does not answer.\n");
        return 3;
    }
    say(&format!("Claims       : {}\n", r.version_text));
    if !r.description.is_empty() {
        say(&format!("Description  : {}\n", r.description));
    }
    if r.behaves_like_level > 0 {
        say(&format!(
            "Behaves like : ELM327 {}\n",
            version_text(r.behaves_like_level)
        ));
    }
    if r.is_stn {
        say(&format!("STN chip     : {} {}\n", r.stn_id, r.stn_device));
    }
    if r.has_voltage {
        say(&format!("Voltage      : {} V\n", fixed(r.voltage, 1)));
    }
    if r.round_trip_samples > 0 {
        say(&format!(
            "Round trip   : {} ms\n",
            fixed(r.round_trip_ms, 0)
        ));
    }
    if r.response_count_hint {
        say(&format!(
            "With hint    : {} ms (expected answer count appended)\n",
            fixed(r.round_trip_with_hint_ms, 0)
        ));
    }
    say("\nCommands:\n");
    for c in &r.checks {
        say(&format!(
            "  {} {} {} {}\n",
            ljust(if c.supported { "ok" } else { "--" }, 4),
            ljust(&c.command, 10),
            ljust(&version_text(c.introduced_in), 6),
            c.purpose.get()
        ));
    }
    if !r.facts.is_empty() {
        say("\nObserved:\n");
    }
    for t in &r.facts {
        say(&format!("  {}\n", t.get()));
    }
    if !r.speed_tips.is_empty() {
        say("\nFor more speed:\n");
    }
    for t in &r.speed_tips {
        say(&format!("  {}\n", t.get()));
    }
    say(&format!("\nVerdict: {}\n", r.verdict.get()));
    0
}

pub fn info(elm: &mut Elm327) -> i32 {
    let mut obd = Obd2::new(elm);
    let vin = obd.read_vin();
    say(&format!(
        "VIN: {}\n",
        if vin.is_empty() {
            "(not provided)"
        } else {
            &vin
        }
    ));
    let (frame, trigger) = obd.read_freeze_frame();
    if !trigger.is_empty() {
        say(&format!("Freeze frame for code {trigger}:\n"));
    }
    for v in &frame {
        print_value(v);
    }
    0
}

pub fn ecus(elm: &mut Elm327) -> i32 {
    // Everything per control unit: headers on, so every answer keeps
    // its sender.
    if !elm.set_headers(true) {
        say("Adapter refused headers (ATH1).\n");
    }
    let mut obd = Obd2::new(elm);
    for s in obd.read_status_per_ecu() {
        say(&format!(
            "{} MIL {}, {} code(s), {}\n",
            ljust(&s.ecu, 20),
            if s.mil_on { "ON" } else { "off" },
            s.fault_count,
            if s.compression_ignition {
                "diesel"
            } else {
                "spark ignition"
            }
        ));
    }
    for f in obd.read_faults().unwrap_or_default() {
        say(&format!("  {} [{}] {}\n", f.code, f.kind, f.ecu));
    }
    for e in obd.read_ecu_info() {
        say(&format!("\n{}: {}\n", e.ecu, e.name));
        if !e.vin.is_empty() {
            say(&format!("  VIN  {}\n", e.vin));
        }
        for id in &e.calibration_ids {
            say(&format!("  CALID {id}\n"));
        }
        for c in &e.cvns {
            say(&format!("  CVN  {c}\n"));
        }
        for c in &e.in_use.monitors {
            say(&format!("  {} {}\n", ljust(&c.name, 40), c.text));
        }
    }
    say("\n");
    let results = obd.read_monitor_results().unwrap_or_default();
    for m in misfire_counts(&results) {
        say(&format!("  {}\n", m.text));
    }
    say("\n");
    let supported: Vec<i32> = obd.supported_pids().into_iter().collect();
    for v in obd.read_live_batch(&supported, false) {
        if !v.text.is_empty() {
            say(&format!(
                "  {:03X} {} {}\n",
                v.key,
                ljust(&v.name, 50),
                v.text
            ));
        } else {
            say(&format!(
                "  {:03X} {} {:>10.2} {}\n",
                v.key,
                ljust(&v.name, 50),
                v.value,
                v.unit
            ));
        }
    }
    0
}
