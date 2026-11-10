//! UDS (ISO 14229) through the ELM327, tests/test_uds.cpp: addressing, ISO-TP
//! answers with and without headers, negative responses, fault records and
//! the failure type byte. The recorded byte examples are worked out by hand
//! from ISO 14229-1, ISO 15765-2 and the ELM327 data sheet; the demo car
//! checks the same end to end, including that normal OBD still works
//! afterwards. The pure table cases (failure type texts, the BMW address
//! list) are ported in caruspex-data.

mod common;

use std::cell::Cell;
use std::collections::BTreeSet;

use caruspex_data::ecu_addresses::{EcuDefinition, EcuKind, ecu_from_ids};
use caruspex_demo::DemoTransport;
use caruspex_i18n::{Language, set_language};
use caruspex_link::elm327::DEFAULT_REQUEST_TIMEOUT_MS;
use caruspex_protocols::obd2::Obd2;
use caruspex_protocols::uds::{
    DEFAULT_UDS_TIMEOUT_MS, Uds, UdsEcu, decode_uds_dtc, extended_request, scan_ecus,
};
use common::*;

fn ecu(request: u32, response: u32, name: &str) -> EcuDefinition {
    ecu_of(request, response, name, EcuKind::Other, false)
}

fn ecu_of(request: u32, response: u32, name: &str, kind: EcuKind, extended: bool) -> EcuDefinition {
    EcuDefinition {
        request,
        response,
        extended29: extended,
        kind,
        name: name.to_string(),
        ..EcuDefinition::default()
    }
}

/// "CX5906018B" in F187: 62 F1 87 plus ten characters, thirteen bytes, so a
/// first frame with six bytes and one consecutive frame with seven.
fn part_number() -> Reply {
    reply(
        &["00D", "0: 62 F1 87 43 58 35", "1: 39 30 36 30 31 38 42"],
        &["7E8 10 0D 62 F1 87 43 58 35", "7E8 21 39 30 36 30 31 38 42"],
    )
}

fn scan(elm: &mut caruspex_link::Elm327, list: &[EcuDefinition]) -> Vec<UdsEcu> {
    scan_ecus(elm, list, None, None)
}

fn find_ecu(ecus: &[UdsEcu], request: u32) -> &UdsEcu {
    ecus.iter()
        .find(|e| e.definition.request == request)
        .expect("in the list")
}

fn demo_ecus() -> Vec<EcuDefinition> {
    vec![
        ecu_of(0x7E0, 0x7E8, "Engine", EcuKind::Engine, false),
        ecu_of(0x7E1, 0x7E9, "Gearbox", EcuKind::Gearbox, false),
        ecu_of(0x760, 0x768, "ABS", EcuKind::Brakes, false),
        ecu_of(0x737, 0x73F, "Airbag", EcuKind::Airbag, false),
        ecu_of(0x720, 0x728, "Instruments", EcuKind::Instruments, false),
        ecu_of(0x744, 0x74C, "Climate", EcuKind::Climate, false),
        ecu_of(0x710, 0x718, "Nobody", EcuKind::Other, false),
        ecu_of(0x7A0, 0x7A8, "Nobody either", EcuKind::Other, false),
    ]
}

fn sent(script: &Script) -> Vec<String> {
    script.lock().unwrap().sent.clone()
}

fn clear_sent(script: &Script) {
    script.lock().unwrap().sent.clear();
}

fn count(script: &Script, cmd: &str) -> usize {
    script.lock().unwrap().count(cmd)
}

#[test]
fn selecting_an_ecu_sets_header_receive_filter_and_flow_control() {
    let _lang = german();
    let (mut elm, script) = script_rig("6");
    {
        let mut uds = Uds::new(&mut elm);
        clear_sent(&script);
        assert_eq!(uds.select(&ecu(0x760, 0x768, "ABS")), Ok(()));
        assert_eq!(
            sent(&script),
            [
                "ATSH760",
                "ATCRA768",
                "ATFCSH760",
                "ATFCSD300000",
                "ATFCSM1",
                "ATCAF1"
            ]
        );
        clear_sent(&script);
    }
    // Dropping puts the functional OBD addressing back.
    assert_eq!(sent(&script), ["ATFCSM0", "ATSH7DF", "ATCRA"]);
}

#[test]
fn addressing_with_29_bit_uses_atcp_for_the_priority_byte() {
    let _lang = german();
    let (mut elm, script) = script_rig("7");
    let mut uds = Uds::new(&mut elm);
    clear_sent(&script);
    uds.select(&ecu_of(
        0x18DA28F1,
        0x18DAF128,
        "ABS",
        EcuKind::Brakes,
        true,
    ))
    .expect("selected");
    let s = sent(&script);
    assert_eq!(s[0], "ATCP18");
    assert_eq!(s[1], "ATSHDA28F1");
    assert_eq!(s[2], "ATCRA18DAF128");
    assert_eq!(s[3], "ATFCSH18DA28F1");

    // An 11-bit address does not fit this bus.
    let error = uds.select(&ecu(0x760, 0x768, "ABS")).expect_err("refused");
    assert!(!error.is_empty());

    clear_sent(&script);
    uds.restore_default();
    assert_eq!(sent(&script), ["ATFCSM0", "ATCP18", "ATSHDB33F1", "ATCRA"]);
}

#[test]
fn old_clones_reset_the_receive_filter_with_atar() {
    let _lang = german();
    let (mut elm, script) = script_rig("6");
    script.lock().unwrap().refused = BTreeSet::from(["ATCRA".into(), "ATFCSH760".into()]);
    let mut uds = Uds::new(&mut elm);
    clear_sent(&script);
    // Flow control setup is optional; the address itself is not.
    uds.select(&ecu(0x760, 0x768, "ABS")).expect("selected");
    assert_eq!(count(&script, "ATFCSM1"), 0);
    uds.restore_default();
    assert_eq!(sent(&script).last().unwrap(), "ATAR");

    script.lock().unwrap().refused = BTreeSet::from(["ATCRA768".into(), "ATCRA".into()]);
    let error = uds.select(&ecu(0x760, 0x768, "ABS")).expect_err("refused");
    assert!(!error.is_empty());
    assert_eq!(sent(&script).last().unwrap(), "ATAR"); // put back at once
}

#[test]
fn uds_answers_in_single_and_multi_frames_with_and_without_headers() {
    let _lang = german();
    for headers in [false, true] {
        let (mut elm, script) = script_rig_with("6", |s| {
            s.set("3E00", reply(&["7E 00"], &["7E8 02 7E 00 AA AA AA AA AA"]));
            s.set("22F187", part_number());
            s.set(
                "22F189",
                reply(&["62 F1 89 30 34 31 37"], &["7E8 07 62 F1 89 30 34 31 37"]),
            );
            // A VIN padded with a zero byte and a space, both cleaned away.
            s.set(
                "22F190",
                reply(
                    &[
                        "016",
                        "0: 62 F1 90 59 43 58",
                        "1: 44 45 31 35 43 37 4D",
                        "2: 43 30 34 38 32 31 37",
                        "3: 00 20",
                    ],
                    &[
                        "7E8 10 16 62 F1 90 59 43 58",
                        "7E8 21 44 45 31 35 43 37 4D",
                        "7E8 22 43 30 34 38 32 31 37",
                        "7E8 23 00 20 AA AA AA AA AA",
                    ],
                ),
            );
        });
        let _ = &script;
        if headers {
            assert!(elm.set_headers(true));
        }

        let mut uds = Uds::new(&mut elm);
        uds.select(&ecu(0x7E0, 0x7E8, "Engine")).expect("selected");
        assert!(uds.tester_present());

        let response = uds
            .request(&[0x22, 0xF1, 0x87], DEFAULT_UDS_TIMEOUT_MS)
            .expect("answer");
        assert_eq!(response.len(), 12);
        assert_eq!(response[0], 0xF1);
        assert_eq!(response[1], 0x87);
        assert_eq!(*response.last().unwrap(), 0x42);

        let info = uds.read_info();
        assert_eq!(info.part_number, "CX5906018B");
        assert_eq!(info.software_version, "0417");
        assert_eq!(info.vin, "YCXDE15C7MC048217");
        assert!(info.hardware_version.is_empty()); // NO DATA for F191 and F193
        assert!(info.system_name.is_empty());
    }
}

#[test]
fn response_pending_in_the_same_answer_and_after_asking_again() {
    let _lang = german();
    for headers in [false, true] {
        let (mut elm, script) = script_rig("6");
        if headers {
            assert!(elm.set_headers(true));
        }
        let mut uds = Uds::new(&mut elm);
        uds.select(&ecu(0x760, 0x768, "ABS")).expect("selected");

        // 7F 19 78, then within the adapter's timeout the real answer.
        script.lock().unwrap().set(
            "1902FF",
            reply(
                &["7F 19 78", "59 02 FF 40 35 1C AF"],
                &["768 03 7F 19 78 AA AA AA AA", "768 07 59 02 FF 40 35 1C AF"],
            ),
        );
        let dtcs = uds.read_dtcs(0xFF).expect("faults");
        assert_eq!(dtcs.len(), 1);
        assert_eq!(dtcs[0].code, "C0035");
        assert_eq!(count(&script, "1902FF"), 1);

        // The answer comes too late for the adapter: two pending answers,
        // then the positive one on the third request.
        clear_sent(&script);
        script.lock().unwrap().set_all(
            "1902FF",
            vec![
                reply(&["7F 19 78"], &["768 03 7F 19 78 AA AA AA AA"]),
                reply(&["7F 19 78"], &["768 03 7F 19 78 AA AA AA AA"]),
                reply(&["59 02 FF 40 35 1C AF"], &["768 07 59 02 FF 40 35 1C AF"]),
            ],
        );
        let dtcs = uds.read_dtcs(0xFF).expect("faults");
        assert_eq!(dtcs.len(), 1);
        assert_eq!(count(&script, "1902FF"), 3);
    }
}

#[test]
fn negative_responses_give_readable_errors() {
    let _lang = german();
    let (mut elm, script) = script_rig("6");
    let mut uds = Uds::new(&mut elm);
    uds.select(&ecu(0x760, 0x768, "ABS")).expect("selected");

    script.lock().unwrap().set(
        "22F187",
        reply(&["7F 22 11"], &["768 03 7F 22 11 AA AA AA AA"]),
    );
    set_language(Language::English);
    let error = uds
        .request(&[0x22, 0xF1, 0x87], DEFAULT_UDS_TIMEOUT_MS)
        .expect_err("refused");
    assert!(error.contains("service not supported"));
    assert!(error.contains("0x11"));
    set_language(Language::German);
    let error = uds
        .request(&[0x22, 0xF1, 0x87], DEFAULT_UDS_TIMEOUT_MS)
        .expect_err("refused");
    assert!(error.contains("Dienst wird nicht unterstützt"));

    let texts = [
        (0x12, "Unterfunktion"),
        (0x22, "Bedingungen"),
        (0x31, "Bereich"),
        (0x33, "Security Access"),
        (0x7E, "Sitzung"),
        (0x7F, "Sitzung"),
    ];
    for (nrc, word) in texts {
        let line = format!("7F 14 {nrc:02X}");
        script.lock().unwrap().set("14FFFFFF", reply(&[&line], &[]));
        let error = uds.clear_dtcs().expect_err("refused");
        assert!(error.contains(word), "{error}");
    }

    // No answer at all.
    let error = uds
        .request(&[0x22, 0xF1, 0x90], DEFAULT_UDS_TIMEOUT_MS)
        .expect_err("no answer");
    assert!(error.contains("NO DATA"));
    // Too long for a single frame.
    assert!(
        uds.request(&[0x2E, 1, 2, 3, 4, 5, 6, 7], DEFAULT_UDS_TIMEOUT_MS)
            .is_err()
    );

    // A refusal still means the ECU is there.
    script
        .lock()
        .unwrap()
        .set("3E00", reply(&["7F 3E 11"], &[]));
    assert!(uds.tester_present());
    script.lock().unwrap().replies.remove("3E00");
    assert!(!uds.tester_present());
}

#[test]
fn decoding_a_three_byte_uds_fault_code_with_status() {
    let _lang = language(Language::English);
    let c = decode_uds_dtc(0x40, 0x35, 0x1C, 0xAF);
    assert_eq!(c.code, "C0035");
    assert_eq!(c.failure_type, 0x1C);
    assert_eq!(c.failure_text, "circuit voltage out of range");
    assert_eq!(c.status, 0xAF);
    assert!(c.test_failed);
    assert!(c.pending);
    assert!(c.confirmed);
    assert!(c.warning_lamp);
    assert!(c.text.contains("wheel speed"));

    let u = decode_uds_dtc(0xC1, 0x00, 0x00, 0x28);
    assert_eq!(u.code, "U0100");
    assert!(u.failure_text.is_empty());
    assert!(u.confirmed);
    assert!(!u.test_failed);
    assert!(!u.pending);
    assert!(!u.warning_lamp);

    let b = decode_uds_dtc(0x90, 0x00, 0x49, 0x24);
    assert_eq!(b.code, "B1000");
    assert!(b.pending);
    assert!(!b.confirmed);
    assert_eq!(b.failure_text, "internal electronic failure");

    assert_eq!(decode_uds_dtc(0x01, 0x71, 0x00, 0x08).code, "P0171");
    assert_eq!(decode_uds_dtc(0xD5, 0x5A, 0x00, 0x08).code, "U155A");
}

#[test]
fn scanning_the_demo_car_finds_all_six_uds_control_units() {
    let _lang = language(Language::English);
    for headers in [false, true] {
        let mut rig = DemoRig::new(600.0);
        assert!(rig.connected);
        if headers {
            assert!(rig.elm.set_headers(true));
        }

        let mut names: Vec<String> = Vec::new();
        let mut progress = |index: usize, total: usize, name: &str| {
            assert_eq!(total, 8);
            assert_eq!(index, names.len());
            names.push(name.to_string());
        };
        let never = || false;
        let ecus = scan_ecus(
            &mut rig.elm,
            &demo_ecus(),
            Some(&mut progress),
            Some(&never),
        );
        assert_eq!(names.len(), 8);
        assert_eq!(ecus.len(), 8);
        assert_eq!(ecus.iter().filter(|e| e.present).count(), 6);

        let engine = find_ecu(&ecus, 0x7E0);
        assert!(engine.present);
        assert!(engine.dtcs_read);
        assert_eq!(engine.info.part_number, "CX5 906 018 B");
        assert_eq!(engine.info.system_name, "ECM 1.4 MPI");
        assert_eq!(engine.info.vin, DemoTransport::vin());
        assert_eq!(engine.dtcs.len(), 3); // P0171, P0420 stored, P0300 pending
        assert_eq!(engine.dtcs[0].code, "P0171");
        assert!(engine.dtcs[0].confirmed);
        assert!(engine.dtcs[0].warning_lamp);
        assert_eq!(engine.dtcs[2].code, "P0300");
        assert!(engine.dtcs[2].pending);
        assert!(!engine.dtcs[2].confirmed);

        let gearbox = find_ecu(&ecus, 0x7E1);
        assert!(gearbox.present);
        assert!(gearbox.dtcs.is_empty());
        assert!(gearbox.info.vin.is_empty());
        assert_eq!(gearbox.info.software_version, "0203");

        let brakes = find_ecu(&ecus, 0x760);
        assert!(brakes.present);
        assert!(brakes.dtcs_read); // after 7F 19 78
        assert_eq!(brakes.dtcs.len(), 1);
        assert_eq!(brakes.dtcs[0].code, "C0035");
        assert_eq!(brakes.dtcs[0].failure_type, 0x1C);
        assert_eq!(brakes.dtcs[0].failure_text, "circuit voltage out of range");
        assert!(brakes.dtcs[0].test_failed);
        assert!(brakes.dtcs[0].warning_lamp);
        assert!(brakes.dtcs[0].text.contains("wheel speed"));
        assert_eq!(brakes.info.part_number, "CX1 614 517 C");

        let airbag = find_ecu(&ecus, 0x737);
        assert_eq!(airbag.dtcs.len(), 1);
        assert_eq!(airbag.dtcs[0].code, "B0001");
        assert!(airbag.dtcs[0].pending);
        assert!(!airbag.dtcs[0].confirmed);
        assert_eq!(
            airbag.dtcs[0].failure_text,
            "circuit resistance below threshold"
        );

        let cluster = find_ecu(&ecus, 0x720);
        assert!(cluster.present);
        assert!(cluster.dtcs_read);
        assert!(cluster.dtcs.is_empty());
        assert!(cluster.note.is_empty());

        let climate = find_ecu(&ecus, 0x744);
        assert_eq!(climate.dtcs.len(), 1);
        assert_eq!(climate.dtcs[0].code, "U0100");
        assert!(climate.dtcs[0].confirmed);
        assert!(climate.dtcs[0].failure_text.is_empty());
        // F195 and F193 stand in for F189 and F191.
        assert_eq!(climate.info.software_version, "0109");
        assert_eq!(climate.info.hardware_version, "H04");

        let nobody = find_ecu(&ecus, 0x710);
        assert!(!nobody.present);
        assert!(!nobody.dtcs_read);
        assert!(!nobody.note.is_empty());

        // Normal OBD works afterwards: mode 03 over the functional address.
        let codes: Vec<String> = Obd2::new(&mut rig.elm)
            .read_faults()
            .into_iter()
            .map(|f| f.code)
            .collect();
        assert!(codes.iter().any(|c| c == "P0171"));
        assert!(codes.iter().any(|c| c == "P0420"));
        assert_eq!(rig.elm.headers_on(), headers);
    }
}

#[test]
fn clearing_one_uds_control_unit_leaves_the_others_alone() {
    let _lang = language(Language::English);
    let mut rig = DemoRig::new(600.0);
    assert!(rig.connected);
    {
        let mut uds = Uds::new(&mut rig.elm);
        uds.select(&ecu(0x760, 0x768, "ABS")).expect("selected");
        uds.clear_dtcs().expect("cleared");
        assert!(uds.read_dtcs(0xFF).expect("faults").is_empty());

        // Status mask: only confirmed codes of the climate unit, none pending.
        uds.select(&ecu(0x744, 0x74C, "Climate")).expect("selected");
        assert_eq!(uds.read_dtcs(0x08).expect("faults").len(), 1);
        assert!(uds.read_dtcs(0x04).expect("faults").is_empty());

        // OBD services stay with the engine and gearbox.
        let error = uds
            .request(&[0x09, 0x02], DEFAULT_UDS_TIMEOUT_MS)
            .expect_err("refused");
        assert!(error.contains("0x11"));
        // Unknown identifiers are out of range.
        let error = uds
            .request(&[0x22, 0x12, 0x34], DEFAULT_UDS_TIMEOUT_MS)
            .expect_err("refused");
        assert!(error.contains("0x31"));
    }
    let ecus = scan(&mut rig.elm, &demo_ecus());
    assert!(find_ecu(&ecus, 0x760).dtcs.is_empty());
    assert_eq!(find_ecu(&ecus, 0x737).dtcs.len(), 1);
    assert_eq!(find_ecu(&ecus, 0x744).dtcs.len(), 1);
    assert_eq!(find_ecu(&ecus, 0x7E0).dtcs.len(), 3);

    // Clearing the engine over UDS is the same as mode 04.
    {
        let mut uds = Uds::new(&mut rig.elm);
        uds.select(&ecu(0x7E0, 0x7E8, "Engine")).expect("selected");
        uds.clear_dtcs().expect("cleared");
    }
    let mut obd = Obd2::new(&mut rig.elm);
    assert_eq!(obd.read_faults().len(), 2); // only the permanent codes stay
    for f in obd.read_faults() {
        assert_eq!(f.kind, "permanent");
    }
}

#[test]
fn a_scan_can_be_cancelled_and_still_restores_the_addressing() {
    let _lang = german();
    let mut rig = DemoRig::new(600.0);
    assert!(rig.connected);
    let asked = Cell::new(0);
    let mut progress = |_: usize, _: usize, _: &str| asked.set(asked.get() + 1);
    let cancelled = || asked.get() >= 2;
    let ecus = scan_ecus(
        &mut rig.elm,
        &demo_ecus(),
        Some(&mut progress),
        Some(&cancelled),
    );
    assert_eq!(ecus.len(), 2);
    let log = rig.sent();
    assert!(log.last().unwrap().contains("ATST64"));
    assert!(log.iter().any(|line| line.contains("ATSH7DF")));

    // Functional OBD answers as before, from both OBD control units.
    let r = rig.elm.request("0100", 0x01, DEFAULT_REQUEST_TIMEOUT_MS);
    assert!(r.ok());
    assert_eq!(r.messages.len(), 2);
}

#[test]
fn without_can_there_is_no_uds() {
    let _lang = german();
    let (mut elm, _script) = script_rig("5");
    let ecus = scan(&mut elm, &[ecu(0x760, 0x768, "ABS")]);
    assert_eq!(ecus.len(), 1);
    assert!(!ecus[0].present);
    assert!(!ecus[0].note.is_empty());
}

// ---------------------------------------------------------------------------
// BMW style extended addressing: request 6F1 with the target byte first,
// answer from 6xx with F1 first. The adapter runs with CAN auto formatting
// off, so the ISO-TP frames below are the raw eight bytes on the bus.
// ---------------------------------------------------------------------------

fn bmw_unit(address: u32, name: &str) -> EcuDefinition {
    ecu_from_ids(0x6F1, 0x600 + address, false, name)
}

/// The headers-on form of a raw BMW frame from unit 12.
fn dme(bytes: &str) -> String {
    format!("612 {bytes}")
}

#[test]
fn bmw_addressing_ids_raw_mode_and_the_way_back() {
    let _lang = german();
    let d = bmw_unit(0x12, "DME");
    assert_eq!(d.extended_address, 0x12);
    assert_eq!(ecu_from_ids(0x7E0, 0x7E8, false, "x").extended_address, -1);
    assert_eq!(ecu_from_ids(0x6F1, 0x7E8, false, "x").extended_address, -1);

    let (mut elm, script) = script_rig("6");
    {
        let mut uds = Uds::new(&mut elm);
        uds.select(&d).expect("selected");
        assert_eq!(count(&script, "ATSH6F1"), 1);
        assert_eq!(count(&script, "ATCRA612"), 1);
        assert_eq!(count(&script, "ATCAF0"), 1);
    }
    // Back to normal OBD: auto formatting on, functional header.
    assert!(count(&script, "ATCAF1") >= 1);
    let s = sent(&script);
    assert_eq!(s.last().unwrap(), "ATCRA");
    assert!(s.iter().any(|c| c == "ATSH7DF"));
}

#[test]
fn bmw_addressing_single_and_multi_frame_answers() {
    let _lang = german();
    for headers in [false, true] {
        let line = |bytes: &str| {
            if headers {
                dme(bytes)
            } else {
                bytes.to_string()
            }
        };
        let (mut elm, script) = script_rig_with("6", |s| {
            // 3E 00 as a single frame: 12 02 3E 00, padded to eight bytes.
            s.set(
                "12023E0000000000",
                reply(
                    &["F1 02 7E 00 00 00 00 00"],
                    &[&dme("F1 02 7E 00 00 00 00 00")],
                ),
            );
            // VIN, 20 bytes: first frame with five, then 6 + 6 + 3.
            s.set(
                "120322F190000000",
                reply(
                    &["F1 10 14 62 F1 90 57 42"],
                    &[&line("F1 10 14 62 F1 90 57 42")],
                ),
            );
            let more = [
                "F1 21 41 34 4A 31 43 35",
                "F1 22 30 4B 42 4D 31 32",
                "F1 23 33 34 35 00 00 00",
            ];
            let headed: Vec<String> = more.iter().map(|m| line(m)).collect();
            let headed: Vec<&str> = headed.iter().map(String::as_str).collect();
            s.set("1230000000000000", reply(&more, &headed));
        });
        let _ = &script;
        if headers {
            assert!(elm.set_headers(true));
        }

        let mut uds = Uds::new(&mut elm);
        uds.select(&bmw_unit(0x12, "DME")).expect("selected");
        assert!(uds.tester_present());

        let response = uds
            .request(&[0x22, 0xF1, 0x90], DEFAULT_UDS_TIMEOUT_MS)
            .expect("answer");
        assert_eq!(response.len(), 19);
        assert_eq!(uds.read_info().vin, "WBA4J1C50KBM12345");
    }
}

#[test]
fn bmw_addressing_fault_codes_in_bmws_own_format() {
    let _lang = german();
    let (mut elm, _script) = script_rig_with("6", |s| {
        // 59 02 FF, then 480A10 status 2F and 930B21 status 08: eleven bytes.
        s.set(
            "12031902FF000000",
            reply(
                &["F1 10 0B 59 02 FF 48 0A"],
                &[&dme("F1 10 0B 59 02 FF 48 0A")],
            ),
        );
        s.set(
            "1230000000000000",
            reply(
                &["F1 21 10 2F 93 0B 21 08"],
                &[&dme("F1 21 10 2F 93 0B 21 08")],
            ),
        );
    });

    let mut uds = Uds::new(&mut elm);
    uds.select(&bmw_unit(0x12, "DME")).expect("selected");
    let dtcs = uds.read_dtcs(0xFF).expect("faults");
    assert_eq!(dtcs.len(), 2);
    assert_eq!(dtcs[0].code, "480A10");
    assert!(dtcs[0].test_failed);
    assert!(dtcs[0].confirmed);
    assert!(dtcs[0].failure_text.is_empty());
    assert!(!dtcs[0].text.is_empty());
    assert_eq!(dtcs[1].code, "930B21");
    assert!(!dtcs[1].test_failed);
    assert!(dtcs[1].confirmed);
}

#[test]
fn bmw_addressing_busy_refusals_and_foreign_frames() {
    let _lang = german();
    let (mut elm, script) = script_rig_with("6", |s| {
        // First 7F 22 78 (still busy), the second ask gets the answer.
        s.set_all(
            "120322F187000000",
            vec![
                reply(&["F1 03 7F 22 78 00 00 00"], &[]),
                reply(&["F1 06 62 F1 87 41 42 43"], &[]),
            ],
        );
        // A refusal: request out of range.
        s.set("120322F197000000", reply(&["F1 03 7F 22 31 00 00 00"], &[]));
        // A frame for another tester (F4, an ENET tool) is not ours.
        s.set("120322F18C000000", reply(&["F4 06 62 F1 8C 41 42 43"], &[]));
    });

    let mut uds = Uds::new(&mut elm);
    uds.select(&bmw_unit(0x12, "DME")).expect("selected");
    let response = uds
        .request(&[0x22, 0xF1, 0x87], DEFAULT_UDS_TIMEOUT_MS)
        .expect("answer");
    assert_eq!(response, [0xF1, 0x87, 0x41, 0x42, 0x43]);
    assert_eq!(count(&script, "120322F187000000"), 2);

    let error = uds
        .request(&[0x22, 0xF1, 0x97], DEFAULT_UDS_TIMEOUT_MS)
        .expect_err("refused");
    assert!(!error.is_empty());
    assert!(
        uds.request(&[0x22, 0xF1, 0x8C], DEFAULT_UDS_TIMEOUT_MS)
            .is_err()
    );

    // Requests longer than one single frame are refused before sending.
    assert!(
        uds.request(
            &[0x22, 0xF1, 0x90, 0xF1, 0x87, 0xF1, 0x89],
            DEFAULT_UDS_TIMEOUT_MS
        )
        .is_err()
    );
    let reply = extended_request(uds.elm(), 0x12, 0xF1, &[0; 7], 1000);
    assert!(!reply.answered);
    assert!(!reply.error.is_empty());
}
