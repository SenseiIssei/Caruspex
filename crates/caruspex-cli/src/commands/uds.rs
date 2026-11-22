//! The UDS fault memory of a list of control units.

use caruspex_data::dtc::{brand_family, brand_family_label};
use caruspex_data::ecu_addresses::{EcuDefinition, EcuKind, ecu_definitions, ecu_from_ids};
use caruspex_link::Elm327;
use caruspex_link::timing::now_ms;
use caruspex_protocols::{Obd2, UdsDtc, UdsEcu, scan_ecus};
use caruspex_record::JsonWriter;
use caruspex_vehicles::vin::decode_vin;

use super::{fail, fixed};
use crate::args::strto;
use crate::out::{say, say_err};

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
