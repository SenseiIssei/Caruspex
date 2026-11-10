//! The commands of caruspex_cli. Every text and every JSON field is the one
//! of the C++ version.

use std::sync::Arc;
use std::time::Instant;

use caruspex_analysis::common::{FuelType, Level, TrafficLight, level_key};
use caruspex_analysis::health_check::{
    Area, HealthInput, fuel_type_from_obd, health_live_pids, run_health_check,
};
use caruspex_analysis::inputs;
use caruspex_data::dtc::{brand_family, brand_family_label, lookup_dtc};
use caruspex_data::ecu_addresses::{EcuDefinition, EcuKind, ecu_definitions, ecu_from_ids};
use caruspex_data::severity::{Severity, severity_label};
use caruspex_link::adapter_probe::version_text;
use caruspex_link::timing::{now_ms, sleep_until_ms};
use caruspex_link::{Elm327, Serial, probe_adapter};
use caruspex_protocols::{
    Fault, LiveValue, Obd2, UdsDtc, UdsEcu, VehicleStatus, misfire_counts, scan_ecus,
};
use caruspex_record::{
    CustomPid, CustomPidReader, JsonWriter, USER_KEY_BASE, pack_names, pack_pids, pack_title,
    parse_torque_csv,
};
use caruspex_vehicles::catalog::{self, CatalogMatch};
use caruspex_vehicles::vin::{VinInfo, decode_vin};

use crate::args::strto;
use crate::out::{flush, ljust, rjust, say, say_err};

/// `printf("%.Nf")` for the rounding the C library does.
fn fixed(value: f64, digits: usize) -> String {
    format!("{value:.digits$}")
}

// ---------------------------------------------------------------------------
// KKL cable self-test
// ---------------------------------------------------------------------------

/// Checks a KKL cable without it being plugged into the car. It tests what the
/// 5-baud wake-up address needs: toggling the break line and changing the baud
/// rate on the fly. It also measures how accurately the bit times are kept.
pub fn kkl_check(port: &str) -> i32 {
    say(&format!("KKL self-test on {port}\n"));
    say("The cable does NOT need to be plugged into the car for this.\n\n");

    let mut line = Serial::new();
    if let Err(error) = line.open(port, 9600) {
        say(&format!("[fail]  Cannot open the port: {error}\n"));
        return 2;
    }
    say("[ok]    Port opened at 9600 baud\n");

    // 1) Change the baud rate on the fly.
    let mut baud_ok = true;
    for baud in [9600u32, 10400, 4800, 9600] {
        if !line.change_baud(baud) {
            say(&format!("[fail]  Baud rate {baud} is not accepted\n"));
            baud_ok = false;
        }
    }
    if baud_ok {
        say("[ok]    Baud rate can be switched on the fly\n");
    }

    // 2) Toggle the break line. This is what clocks out the wake-up address.
    if !line.set_break(true) || !line.set_break(false) {
        say(
            "[fail]  Cannot toggle the break line. Without it the 5-baud wake-up address does not work.\n",
        );
        line.close();
        return 3;
    }
    say("[ok]    Break line can be toggled\n");

    // 3) Measure the bit times. 5 baud means 200 ms per bit. Windows likes to
    //    tends to oversleep, and the control unit cannot follow overly long bits.
    say("\nMeasuring bit times (10 bits as in a real wake-up address) ...\n");
    // Clocked exactly like the real wake-up address: fixed target times,
    // counted from the start, so that nothing accumulates.
    let mut worst = 200.0_f64;
    let begin = Instant::now();
    let start = now_ms();
    let mut previous = 0.0_f64;
    for bit in 0..10u64 {
        line.set_break(bit % 2 == 0);
        sleep_until_ms(start + (bit + 1) * 200);
        let elapsed = begin.elapsed().as_secs_f64() * 1000.0;
        let ms = elapsed - previous;
        previous = elapsed;
        say(&format!("        Bit {:2}: {} ms\n", bit + 1, fixed(ms, 1)));
        if (ms - 200.0).abs() > (worst - 200.0).abs() {
            worst = ms;
        }
    }
    // Stop before closing: closing a USB port takes a good hundred
    // milliseconds and would distort the measurement.
    let total = previous;
    line.set_break(false);
    line.close();

    let average = total / 10.0;
    say(&format!(
        "        Average {} ms, largest deviation {} ms\n",
        fixed(average, 1),
        fixed(worst, 1)
    ));
    say(&format!(
        "        Total length {} ms (target 2000 ms)\n",
        fixed(total, 0)
    ));

    if (average - 200.0).abs() <= 8.0 {
        say("[ok]    Timing accurate enough for the wake-up address\n");
    } else {
        say("[warn]  Timing deviates. The wake-up address may fail because of it.\n");
    }

    say("\nResult: the cable can do everything the protocol needs.\n");
    say("Whether the control unit answers only shows on the car with the ignition on.\n");
    0
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
struct VinContext {
    vin: String,
    info: VinInfo,
    matched: CatalogMatch,
    brand: String,
}

fn read_vin_context(obd: &mut Obd2) -> VinContext {
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

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

pub fn read(elm: &mut Elm327, as_json: bool) -> i32 {
    let mut obd = Obd2::new(elm);
    let status = obd.read_status();
    let faults = obd.read_faults();
    let err = String::new();
    let vin = read_vin_context(&mut obd);

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object();
        json_adapter(&mut json, obd.elm());
        json.key("vin");
        if vin.vin.is_empty() {
            json.null();
        } else {
            json.value(&vin.vin);
        }
        json.field("brand", &vin.brand);
        json_status(&mut json, &status);
        json_faults(&mut json, &faults, &vin.brand);
        if !err.is_empty() {
            json.field("error", &err);
        }
        json.end_object();
        say(&json.text());
        return 0;
    }

    if !vin.vin.is_empty() {
        let brand = if vin.brand.is_empty() {
            String::new()
        } else {
            format!("  ({})", vin.brand)
        };
        say(&format!("VIN: {}{}\n", vin.vin, brand));
    }
    if status.valid {
        say(&format!(
            "MIL: {}   reported faults: {}\n",
            if status.mil_on { "ON" } else { "off" },
            status.fault_count
        ));
        for r in &status.readiness {
            say(&format!(
                "   {} {}\n",
                ljust(&r.name, 26),
                if r.ready { "ready" } else { "not ready" }
            ));
        }
    }
    say("\n");
    if faults.is_empty() {
        say("No faults stored.\n");
    } else {
        say(&format!("{} fault(s) found:\n", faults.len()));
        for f in &faults {
            say(&format!(
                "  {} [{}]{}{}\n      {}\n",
                f.code,
                f.kind,
                if f.ecu.is_empty() { "" } else { " " },
                f.ecu,
                fault_text(&f.code, &vin.brand)
            ));
        }
    }
    if !err.is_empty() {
        say(&format!("{err}\n"));
    }
    0
}

pub fn vin(elm: &mut Elm327, as_json: bool) -> i32 {
    let mut obd = Obd2::new(elm);
    let c = read_vin_context(&mut obd);
    let info = &c.info;
    let matched = &c.matched;

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object();
        json_adapter(&mut json, obd.elm());
        json.key("vin");
        if c.vin.is_empty() {
            json.null();
            json.field(
                "error",
                "The car does not report a VIN (mode 09, InfoType 02).",
            );
            json.end_object();
            say(&json.text());
            return 4;
        }
        json.value(&c.vin)
            .field("valid", &info.valid)
            .field("wmi", &info.wmi)
            .field("manufacturer", &info.manufacturer)
            .field("brand", &info.brand)
            .string_list("brandCandidates", &info.brand_candidates)
            .field("country", &info.country)
            .field("region", &info.region)
            .key("modelYear");
        number_or_null(&mut json, info.model_year > 0, info.model_year);
        json.field("modelYearCertain", &info.model_year_certain)
            .key("modelYears")
            .begin_array();
        for year in &info.model_years {
            json.value(year);
        }
        json.end_array()
            .field("checkDigitApplies", &info.check_digit_applies)
            .field("checkDigitOk", &info.check_digit_ok)
            .field("serial", &info.serial)
            .field("modelCode", &info.model_code)
            .string_list("problems", &info.problems)
            .string_list("notes", &info.notes)
            .key("catalog")
            .begin_object()
            .field("brand", &matched.brand)
            .field("confidence", &matched.confidence)
            .field("reason", &matched.reason)
            .key("year");
        number_or_null(&mut json, matched.year > 0, matched.year);
        json.key("generation");
        if let Some(g) = matched.generation {
            json.begin_object()
                .field("name", &catalog::display_name(g))
                .field("brand", g.brand)
                .field("model", g.model)
                .field("generation", g.generation)
                .field("yearFrom", &g.year_from)
                .key("yearTo");
            number_or_null(&mut json, g.year_to > 0, g.year_to);
            json.field("diagnosis", &catalog::diagnosis_text(g, matched.year))
                .end_object();
        } else {
            json.null();
        }
        json.key("alternatives").begin_array();
        for g in &matched.alternatives {
            json.value(&catalog::display_name(g));
        }
        json.end_array().end_object().end_object();
        say(&json.text());
        return 0;
    }

    if c.vin.is_empty() {
        say("The car does not report a VIN (mode 09, InfoType 02).\n");
        return 4;
    }
    say(&format!(
        "VIN          : {}{}\n",
        c.vin,
        if info.valid {
            ""
        } else {
            "  (not a valid VIN)"
        }
    ));
    say(&format!(
        "Manufacturer : {}\n",
        if info.manufacturer.is_empty() {
            "unknown"
        } else {
            &info.manufacturer
        }
    ));
    if !info.country.is_empty() {
        say(&format!(
            "Country      : {}, {}\n",
            info.country, info.region
        ));
    }
    if info.model_year > 0 {
        say(&format!(
            "Model year   : {}{}\n",
            info.model_year,
            if info.model_year_certain {
                ""
            } else {
                " (not certain)"
            }
        ));
    }
    if info.check_digit_applies {
        say(&format!(
            "Check digit  : {}\n",
            if info.check_digit_ok {
                "correct"
            } else {
                "WRONG"
            }
        ));
    }
    if !info.serial.is_empty() {
        say(&format!("Serial       : {}\n", info.serial));
    }
    for p in &info.problems {
        say(&format!("Problem      : {p}\n"));
    }
    for n in &info.notes {
        say(&format!("Note         : {n}\n"));
    }
    say("\n");
    if let Some(g) = matched.generation {
        say(&format!(
            "Catalog      : {}, {} % sure\n",
            catalog::display_name(g),
            matched.confidence
        ));
        if !matched.reason.is_empty() {
            say(&format!("               {}\n", matched.reason));
        }
        say(&format!(
            "Diagnosis    : {}\n",
            catalog::diagnosis_text(g, matched.year)
        ));
        for alt in &matched.alternatives {
            say(&format!("Also possible: {}\n", catalog::display_name(alt)));
        }
    } else if !matched.brand.is_empty() {
        say(&format!(
            "Catalog      : {}, model not recognised\n",
            matched.brand
        ));
    } else {
        say("Catalog      : no match\n");
    }
    0
}

pub fn freeze(elm: &mut Elm327, as_json: bool) -> i32 {
    let mut obd = Obd2::new(elm);
    let frames = obd.read_freeze_frames(caruspex_protocols::obd2::DEFAULT_FREEZE_FRAMES);
    let brand = if frames.is_empty() {
        String::new()
    } else {
        read_vin_context(&mut obd).brand
    };

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object();
        json_adapter(&mut json, obd.elm());
        json.field("brand", &brand).key("frames").begin_array();
        for f in &frames {
            json.begin_object()
                .field("frame", &f.frame)
                .field("dtc", &f.dtc)
                .field("text", &fault_text(&f.dtc, &brand));
            if !f.ecu_source.is_empty() {
                json.field("ecu", &f.ecu).field("ecuSource", &f.ecu_source);
            }
            json.key("values").begin_array();
            for v in &f.values {
                json_value(&mut json, v);
            }
            json.end_array().end_object();
        }
        json.end_array().end_object();
        say(&json.text());
        return 0;
    }

    if frames.is_empty() {
        say("No freeze frame stored.\n");
        return 0;
    }
    for f in &frames {
        say(&format!(
            "Freeze frame {}: {} {}{}{}\n",
            f.frame,
            f.dtc,
            fault_text(&f.dtc, &brand),
            if f.ecu.is_empty() { "" } else { "  " },
            f.ecu
        ));
        for v in &f.values {
            print_value(v);
        }
        say("\n");
    }
    0
}

fn grade_key(grade: TrafficLight) -> &'static str {
    match grade {
        TrafficLight::Green => "green",
        TrafficLight::Yellow => "yellow",
        TrafficLight::Red => "red",
    }
}

fn area_key(area: Area) -> &'static str {
    match area {
        Area::Faults => "faults",
        Area::Readiness => "readiness",
        Area::Mixture => "mixture",
        Area::OxygenSensors => "oxygen_sensors",
        Area::Catalyst => "catalyst",
        Area::Misfire => "misfire",
        Area::Cooling => "cooling",
        Area::Charging => "charging",
        Area::Sensors => "sensors",
        Area::Monitoring => "monitoring",
        Area::History => "history",
    }
}

fn fuel_key(fuel: FuelType) -> &'static str {
    match fuel {
        FuelType::Petrol => "petrol",
        FuelType::Diesel => "diesel",
        FuelType::Unknown => "unknown",
    }
}

pub fn health(elm: &mut Elm327, as_json: bool, seconds: i32) -> i32 {
    let progress = |text: &str| {
        if !as_json {
            say(&format!("{text}\n"));
            flush();
        }
    };

    let mut obd = Obd2::new(elm);
    let mut input = HealthInput::default();
    progress("Reading readiness and warning lamp ...");
    let status = obd.read_status();
    input.set_status(&inputs::VehicleStatus {
        mil_on: status.mil_on,
        fault_count: status.fault_count,
        readiness: status
            .readiness
            .iter()
            .map(|r| inputs::ReadinessItem {
                name: r.name.clone(),
                ready: r.ready,
            })
            .collect(),
        valid: status.valid,
        compression_ignition: status.compression_ignition,
    });

    progress("Reading the fault memory ...");
    let faults = obd.read_faults();
    input.faults = faults
        .iter()
        .map(|f| inputs::Fault {
            code: f.code.clone(),
            kind: f.kind.clone(),
            text: f.text.clone(),
            throttle_related: f.throttle_related,
        })
        .collect();
    input.faults_read = true;
    let vin = read_vin_context(&mut obd);
    let brand = vin.brand.clone();
    let level_brand = brand.clone();
    input.fault_level = Some(Arc::new(move |f: &inputs::Fault| {
        // As in the application: the database knows how urgent a code is,
        // pending codes count one step lower since they may still go away.
        let severity = lookup_dtc(&f.code, &level_brand).severity;
        let mut level = Level::Info;
        if severity == Severity::Critical {
            level = Level::Critical;
        } else if severity == Severity::High || severity == Severity::Medium {
            level = Level::Warning;
        }
        if !is_stored(&f.kind) && level == Level::Warning {
            level = Level::Info;
        }
        level
    }));

    progress("Reading component monitoring (mode 06) ...");
    input.monitor_results = obd
        .read_monitor_results()
        .unwrap_or_default()
        .iter()
        .map(|m| inputs::MonitorResult {
            test_id: m.test_id,
            component_id: m.component_id,
            name: m.name.clone(),
            component: m.component.clone(),
            value: m.value,
            minimum: m.minimum,
            maximum: m.maximum,
            unit: m.unit.clone(),
            has_minimum: m.has_minimum,
            has_maximum: m.has_maximum,
            passed: m.passed,
        })
        .collect();
    input.monitor_results_read = true;

    if let Some(fuel_type) = obd.read_live(0x51) {
        input.fuel = fuel_type_from_obd(fuel_type.value as i32);
    }

    // Live series: only what the car offers.
    let supported = obd.supported_pids();
    let pids: Vec<i32> = health_live_pids()
        .iter()
        .copied()
        .filter(|pid| supported.is_empty() || supported.contains(pid))
        .collect();
    if !as_json {
        say(&format!(
            "Collecting {} live values for {} s ...\n",
            pids.len(),
            seconds
        ));
        flush();
    }
    let mut snapshots = 0;
    let start = now_ms();
    while !pids.is_empty() && now_ms() - start < (seconds as u64) * 1000 {
        let values = obd.read_live_batch(&pids, false);
        if values.is_empty() {
            break;
        }
        let snapshot: Vec<inputs::LiveValue> = values
            .iter()
            .map(|v| inputs::LiveValue {
                pid: v.pid,
                value: v.value,
            })
            .collect();
        input.add_snapshot(&snapshot, (now_ms() - start) as f64 / 1000.0);
        snapshots += 1;
    }

    let report = run_health_check(&input);

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object();
        json_adapter(&mut json, obd.elm());
        json.field("score", &report.score)
            .field("grade", grade_key(report.grade))
            .field("summary", &report.summary.text())
            .field("checksRun", &report.checks_run)
            .field("fuel", fuel_key(input.fuel))
            .field("brand", &brand)
            .key("liveSeries")
            .begin_object()
            .field("seconds", &seconds)
            .field("snapshots", &snapshots)
            .key("pids")
            .begin_array();
        for pid in &pids {
            json.value(pid);
        }
        json.end_array().end_object().key("findings").begin_array();
        for f in &report.findings {
            json.begin_object()
                .field("id", &f.id)
                .field("level", level_key(f.level))
                .field("area", area_key(f.area))
                .field("title", &f.title.text())
                .field("explanation", &f.explanation.text())
                .field("advice", &f.advice.text())
                .end_object();
        }
        json.end_array().key("notChecked").begin_array();
        for s in &report.not_checked {
            json.begin_object()
                .field("id", &s.id)
                .field("title", &s.title.text())
                .field("reason", &s.reason.text())
                .end_object();
        }
        json.end_array();
        json_status(&mut json, &status);
        json_faults(&mut json, &faults, &brand);
        json.end_object();
        say(&json.text());
        return 0;
    }

    say(&format!(
        "{} snapshots of {} values.\n\n",
        snapshots,
        pids.len()
    ));
    say(&format!(
        "Score {}/100 ({}), {} checks run\n",
        report.score,
        grade_key(report.grade),
        report.checks_run
    ));
    say(&format!("{}\n", report.summary.text()));
    if !report.findings.is_empty() {
        say("\nFindings:\n");
    }
    for f in &report.findings {
        say(&format!("  [{}] {}\n", level_key(f.level), f.title.text()));
        if !f.explanation.is_empty() {
            say(&format!("      {}\n", f.explanation.text()));
        }
        if !f.advice.is_empty() {
            say(&format!("      Advice: {}\n", f.advice.text()));
        }
    }
    if !report.not_checked.is_empty() {
        say("\nNot checked:\n");
    }
    for s in &report.not_checked {
        say(&format!("  {}: {}\n", s.title.text(), s.reason.text()));
    }
    0
}

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
    for f in obd.read_faults() {
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

// ---------------------------------------------------------------------------
// UDS
// ---------------------------------------------------------------------------

/// The control units of the demo car, which also suit many real cars that
/// follow the "+8" convention.
fn default_uds_list() -> Vec<EcuDefinition> {
    let entries: [(u32, EcuKind, &str); 6] = [
        (0x7E0, EcuKind::Engine, "Engine"),
        (0x7E1, EcuKind::Gearbox, "Gearbox"),
        (0x760, EcuKind::Brakes, "ABS"),
        (0x737, EcuKind::Airbag, "Airbag"),
        (0x720, EcuKind::Instruments, "Instruments"),
        (0x744, EcuKind::Climate, "Climate"),
    ];
    entries
        .iter()
        .map(|&(request, kind, name)| EcuDefinition {
            request,
            response: request + 8,
            kind,
            name: name.to_string(),
            ..EcuDefinition::default()
        })
        .collect()
}

/// "760,737,18DA28F1,b29" -> definitions. 11-bit ids answer on request + 8,
/// 29-bit ids (18DA<target><source>) with target and source swapped, "b"
/// plus two digits is a BMW unit by its diagnostic address (over 6F1).
pub fn parse_uds_ids(text: &str) -> Result<Vec<EcuDefinition>, String> {
    let mut out = Vec::new();
    for part in text.split(',') {
        let mut item = part.to_string();
        if item.starts_with("0x") || item.starts_with("0X") {
            item = item[2..].to_string();
        }
        if item.len() == 3 && (item.starts_with('b') || item.starts_with('B')) {
            let address = match strto(&item[1..], 16) {
                Some((address, 2)) => address as i32,
                _ => return Err(format!("Not a BMW address: {item}")),
            };
            let mut d = ecu_from_ids(0x6F1, 0x600 + address as u32, false, "");
            for known in ecu_definitions("BMW") {
                if known.extended_address == address {
                    d.name = known.name;
                }
            }
            if d.name.is_empty() {
                d.name = format!("BMW unit {:02X}", address as u32);
            }
            out.push(d);
            item.clear();
        }
        if !item.is_empty() {
            let id = match strto(&item, 16) {
                Some((id, used)) if used == item.len() && u32::try_from(id).is_ok() => id as u32,
                _ => return Err(format!("Not a hex id: {item}")),
            };
            let mut d = EcuDefinition {
                request: id,
                ..EcuDefinition::default()
            };
            if id > 0x7FF {
                d.extended29 = true;
                d.response = (id & 0xFFFF_0000) | ((id & 0xFF) << 8) | ((id >> 8) & 0xFF);
            } else {
                d.response = id + 8;
            }
            d.name = if d.extended29 {
                format!("ECU {id:08X}")
            } else {
                format!("ECU {id:03X}")
            };
            out.push(d);
        }
    }
    if out.is_empty() {
        return Err("No ids given after --ids".to_string());
    }
    Ok(out)
}

fn id_text(d: &EcuDefinition, response: bool) -> String {
    let id = if response { d.response } else { d.request };
    if d.extended29 {
        format!("{id:08X}")
    } else {
        format!("{id:03X}")
    }
}

fn dtc_status_text(d: &UdsDtc) -> String {
    let mut words: Vec<&str> = Vec::new();
    if d.test_failed {
        words.push("failing now");
    }
    if d.confirmed {
        words.push("confirmed");
    }
    if d.pending {
        words.push("pending");
    }
    if d.warning_lamp {
        words.push("warning lamp");
    }
    if words.is_empty() {
        words.push("not failing");
    }
    words.join(", ")
}

fn command_uds(elm: &mut Elm327, list: &[EcuDefinition], as_json: bool) -> i32 {
    if !elm.is_can() {
        return fail(
            as_json,
            &format!(
                "UDS needs a CAN connection; this car speaks {}.",
                elm.protocol_text()
            ),
            3,
        );
    }
    let started = now_ms();
    let mut progress = |index: usize, total: usize, name: &str| {
        if !as_json {
            say_err(&format!("  [{}/{}] {} ...\n", index + 1, total, name));
        }
    };
    let ecus: Vec<UdsEcu> = scan_ecus(elm, list, Some(&mut progress), None);
    let seconds = (now_ms() - started) as f64 / 1000.0;

    let found = ecus.iter().filter(|e| e.present).count() as i32;

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object()
            .field("seconds", &seconds)
            .field("asked", &(ecus.len() as i32))
            .field("present", &found)
            .key("ecus")
            .begin_array();
        for e in &ecus {
            json.begin_object()
                .field("name", &e.definition.name)
                .field("request", &id_text(&e.definition, false))
                .field("response", &id_text(&e.definition, true))
                .field("present", &e.present);
            if e.present {
                json.key("info")
                    .begin_object()
                    .field("partNumber", &e.info.part_number)
                    .field("softwareVersion", &e.info.software_version)
                    .field("hardwareVersion", &e.info.hardware_version)
                    .field("systemName", &e.info.system_name)
                    .field("serial", &e.info.serial)
                    .field("vin", &e.info.vin)
                    .end_object();
                json.field("dtcsRead", &e.dtcs_read)
                    .key("dtcs")
                    .begin_array();
                for d in &e.dtcs {
                    json.begin_object()
                        .field("code", &d.code)
                        .field("failureType", &u32::from(d.failure_type))
                        .field("failureText", &d.failure_text)
                        .field("status", &u32::from(d.status))
                        .field("testFailed", &d.test_failed)
                        .field("pending", &d.pending)
                        .field("confirmed", &d.confirmed)
                        .field("warningLamp", &d.warning_lamp)
                        .field("text", &d.text)
                        .end_object();
                }
                json.end_array();
            }
            if !e.note.is_empty() {
                json.field("note", &e.note);
            }
            json.end_object();
        }
        json.end_array().end_object();
        say(&json.text());
        return 0;
    }

    for e in &ecus {
        say(&format!(
            "{} ({}/{}): {}\n",
            e.definition.name,
            id_text(&e.definition, false),
            id_text(&e.definition, true),
            if e.present { "present" } else { "no answer" }
        ));
        if !e.present {
            continue;
        }
        let i = &e.info;
        if !i.system_name.is_empty() {
            say(&format!("  System    {}\n", i.system_name));
        }
        if !i.part_number.is_empty() {
            say(&format!("  Part no.  {}\n", i.part_number));
        }
        if !i.software_version.is_empty() {
            say(&format!("  Software  {}\n", i.software_version));
        }
        if !i.hardware_version.is_empty() {
            say(&format!("  Hardware  {}\n", i.hardware_version));
        }
        if !i.serial.is_empty() {
            say(&format!("  Serial    {}\n", i.serial));
        }
        if !i.vin.is_empty() {
            say(&format!("  VIN       {}\n", i.vin));
        }
        if e.dtcs_read && e.dtcs.is_empty() {
            say("  No fault codes stored.\n");
        }
        for d in &e.dtcs {
            say(&format!("  {}", d.code));
            if !d.failure_text.is_empty() {
                say(&format!("-{:02X} ({})", d.failure_type, d.failure_text));
            }
            say(&format!(" [{}]\n      {}\n", dtc_status_text(d), d.text));
        }
        if !e.note.is_empty() {
            say(&format!("  Note: {}\n", e.note));
        }
    }
    say(&format!(
        "\n{} of {} control units answered, {} s.\n",
        found,
        ecus.len(),
        fixed(seconds, 1)
    ));
    0
}

pub fn uds(elm: &mut Elm327, mut list: Vec<EcuDefinition>, as_json: bool) -> i32 {
    if list.is_empty() {
        // The list of the brand, by the VIN; the demo list otherwise.
        let vin = Obd2::new(elm).read_vin();
        let family = if vin.is_empty() {
            String::new()
        } else {
            brand_family(&decode_vin(&vin, 0).brand)
        };
        list = if family.is_empty() {
            Vec::new()
        } else {
            ecu_definitions(&family)
        };
        if list.is_empty() {
            list = default_uds_list();
        } else if !as_json {
            say(&format!(
                "Address list {} (from the VIN)\n",
                brand_family_label(&family)
            ));
        }
    }
    command_uds(elm, &list, as_json)
}

// ---------------------------------------------------------------------------
// Custom values
// ---------------------------------------------------------------------------

/// Values, title and import warnings for `custom`.
pub type CustomValues = (Vec<CustomPid>, String, Vec<String>);

/// The values for `custom`: a built-in pack or a Torque CSV file, with the
/// title and the import warnings. Done before connecting, so a typo costs no
/// adapter time. The error comes with the warnings collected so far.
pub fn load_custom_pids(
    pack: &str,
    csv_path: &str,
    demo: bool,
) -> Result<CustomValues, (String, Vec<String>)> {
    let pack_list = || {
        let mut names = String::new();
        for n in pack_names() {
            names.push_str(&format!("\n  {}  ({})", n, pack_title(n)));
        }
        names
    };
    if !csv_path.is_empty() {
        let Ok(bytes) = std::fs::read(csv_path) else {
            return Err((format!("Cannot open {csv_path}"), Vec::new()));
        };
        let (pids, warnings) = parse_torque_csv(&String::from_utf8_lossy(&bytes), USER_KEY_BASE);
        if pids.is_empty() {
            return Err((format!("No usable values in {csv_path}"), warnings));
        }
        return Ok((pids, csv_path.to_string(), warnings));
    }
    let mut name = pack.to_string();
    if name.is_empty() {
        if !demo {
            return Err((
                format!(
                    "Give --pack NAME or --csv FILE. Built-in packs:{}",
                    pack_list()
                ),
                Vec::new(),
            ));
        }
        name = "caruspex-demo".to_string();
    }
    let pids = pack_pids(&name);
    if pids.is_empty() {
        return Err((
            format!("Unknown pack: {name}. Built-in packs:{}", pack_list()),
            Vec::new(),
        ));
    }
    let title = pack_title(&name);
    Ok((pids, title, Vec::new()))
}

/// Up to three decimals, without trailing zeros: 36.5, 3.6, 5614.8, 12.
fn value_text(v: f64) -> String {
    let mut s = format!("{v:.3}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s == "-0" { "0".to_string() } else { s }
}

pub fn custom(
    elm: &mut Elm327,
    pids: &[CustomPid],
    title: &str,
    warnings: &[String],
    as_json: bool,
) -> i32 {
    let started = now_ms();
    let readings = {
        let mut reader = CustomPidReader::new(elm);
        reader.read_all(pids)
    };
    let seconds = (now_ms() - started) as f64 / 1000.0;
    let good = readings.iter().filter(|r| r.ok).count() as i32;

    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object()
            .field("title", title)
            .field("seconds", &seconds)
            .field("read", &good)
            .field("failed", &(readings.len() as i32 - good))
            .string_list("importErrors", warnings)
            .key("values")
            .begin_array();
        for (p, r) in pids.iter().zip(&readings) {
            json.begin_object()
                .field("name", &p.name)
                .field("shortName", &p.short_name)
                .field("key", &p.key)
                .field("request", &p.request)
                .field("header", &p.header)
                .field("response", &r.value.ecu_source)
                .field("ok", &r.ok);
            json.key("value");
            if r.ok {
                json.value(&r.value.value);
            } else {
                json.null();
            }
            json.field("unit", &p.unit)
                .field("error", &r.error)
                .field("source", &p.source)
                .end_object();
        }
        json.end_array().end_object();
        say(&json.text());
    } else {
        for w in warnings {
            say(&format!("Skipped: {w}\n"));
        }
        say(&format!("{title}\n"));
        for (p, r) in pids.iter().zip(&readings) {
            if r.ok {
                say(&format!(
                    "  {} {} {}\n",
                    ljust(&p.name, 48),
                    rjust(&value_text(r.value.value), 12),
                    p.unit
                ));
            } else {
                say(&format!(
                    "  {} {} ({})\n",
                    ljust(&p.name, 48),
                    rjust("-", 12),
                    r.error
                ));
            }
        }
        say(&format!(
            "\n{} of {} values read, {} s.\n",
            good,
            readings.len(),
            fixed(seconds, 1)
        ));
    }
    if good > 0 { 0 } else { 4 }
}
