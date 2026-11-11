//! The OBD-II side of tests/test_obd_extended.cpp: several control units with
//! headers, mode 09 per control unit and misfires from mode 06. The PID table
//! and parser cases live in caruspex-data and caruspex-link; here only their
//! Obd2 halves (the mode 09 decoders). The byte examples are worked out by
//! hand from the formulas of SAE J1979.

mod common;

use caruspex_data::pids::PidCategory;
use caruspex_i18n::{Language, set_language};
use caruspex_link::{HeaderFormat, decode_messages};
use caruspex_protocols::ecu_address::ecu_display_name;
use caruspex_protocols::obd2::{
    MonitorResult, Obd2, info_calibration_ids, info_cvns, info_in_use_performance, info_payload,
    info_text, misfire_counts,
};
use common::*;

// ---------------------------------------------------------------------------
// Several control units and headers
// ---------------------------------------------------------------------------

#[test]
fn control_units_are_named_by_their_address() {
    let _lang = german();
    assert_eq!(ecu_display_name("7E8"), "Motor (7E8)");
    assert_eq!(ecu_display_name("7E9"), "Getriebe (7E9)");
    assert_eq!(ecu_display_name("7EA"), "Steuergerät (7EA)");
    assert_eq!(ecu_display_name("18DAF110"), "Motor (18DAF110)");
    assert_eq!(ecu_display_name("18DAF118"), "Getriebe (18DAF118)");
    assert_eq!(ecu_display_name("10"), "Motor (10)");
    assert_eq!(ecu_display_name("1A"), "Getriebe (1A)");
    assert!(ecu_display_name("").is_empty());
    set_language(Language::English);
    assert_eq!(ecu_display_name("7E8"), "Engine (7E8)");
    assert_eq!(ecu_display_name("7E9"), "Transmission (7E9)");
}

#[test]
fn headers_off_several_vins_decode_apart() {
    // The mode 09 half of "Headers off: several answers stay apart".
    let _lang = german();
    let vins = decode_messages(
        &[
            "014",
            "0:490201575657",
            "1:5A5A5A314B5A41",
            "2:57303030303031",
            "014",
            "0:490201575657",
            "1:5A5A5A314B5A41",
            "2:57303030303032",
        ],
        0x09,
        HeaderFormat::None,
    );
    assert_eq!(vins.messages.len(), 2);
    assert_eq!(
        info_text(&info_payload(&[vins.messages[0].clone()])),
        "WVWZZZ1KZAW000001"
    );
    assert_eq!(
        info_text(&info_payload(&[vins.messages[1].clone()])),
        "WVWZZZ1KZAW000002"
    );
}

#[test]
fn can_11_bit_with_headers_vins_decode_per_sender() {
    // The mode 09 half of "CAN 11 bit with headers: single and multi frame".
    let _lang = german();
    let vin = decode_messages(
        &[
            "7E8 10 14 49 02 01 57 56 57",
            "7E9 10 14 49 02 01 57 56 57",
            "7E8 21 5A 5A 5A 31 4B 5A 41",
            "7E9 21 5A 5A 5A 31 4B 5A 41",
            "7E9 22 57 30 30 30 30 30 32",
            "7E8 22 57 30 30 30 30 30 31",
        ],
        0x09,
        HeaderFormat::Can11,
    );
    assert_eq!(vin.sources, ["7E9", "7E8"]);
    assert_eq!(
        info_text(&info_payload(&[vin.messages[0].clone()])),
        "WVWZZZ1KZAW000002"
    );
    assert_eq!(
        info_text(&info_payload(&[vin.messages[1].clone()])),
        "WVWZZZ1KZAW000001"
    );

    // Without spaces (ATS0) the same.
    let compact = decode_messages(
        &[
            "7E8101449020157565 7",
            "7E8215A5A5A314B5A41",
            "7E82257303030303031",
        ],
        0x09,
        HeaderFormat::Can11,
    );
    assert_eq!(compact.messages.len(), 1);
    assert_eq!(
        info_text(&info_payload(&compact.messages)),
        "WVWZZZ1KZAW000001"
    );
}

// ---------------------------------------------------------------------------
// Mode 09 decoders
// ---------------------------------------------------------------------------

#[test]
fn mode_09_items_are_decoded() {
    let _lang = german();
    // Two calibration IDs from one CAN message.
    let cal = decode_messages(
        &[
            "023",
            "0:49040230334C",
            "1:39303630313844",
            "2:4E000000000030",
            "3:334C3930363031",
            "4:38585800000000",
            "5:00",
        ],
        0x09,
        HeaderFormat::None,
    );
    assert_eq!(cal.messages.len(), 1);
    assert_eq!(
        info_calibration_ids(&info_payload(&cal.messages)),
        ["03L906018DN", "03L906018XX"]
    );

    // CVNs are binary, shown as hex.
    assert_eq!(
        info_cvns(&info_payload(&[vec![
            0x06, 0x02, 0x1A, 0x2B, 0x3C, 0x4D, 0x00, 0x00, 0xBE, 0xEF
        ]])),
        ["1A2B3C4D", "0000BEEF"]
    );

    // K-line: one CVN per message with a sequence number, out of order.
    assert_eq!(
        info_cvns(&info_payload(&[
            vec![0x06, 0x02, 0x00, 0x00, 0x00, 0x02],
            vec![0x06, 0x01, 0x00, 0x00, 0x00, 0x01],
        ])),
        ["00000001", "00000002"]
    );

    // ECU name with a zero byte between acronym and name.
    let name = decode_messages(
        &[
            "017",
            "0:490A0145434D",
            "1:002D456E67696E",
            "2:65436F6E74726F",
            "3:6C0000",
        ],
        0x09,
        HeaderFormat::None,
    );
    assert_eq!(
        info_text(&info_payload(&name.messages)),
        "ECM-EngineControl"
    );

    // In-use performance, spark ignition: 40 OBD cycles, 100 ignitions,
    // catalyst bank 1 checked 12 times in 40 suitable trips.
    let mut ipt = vec![0x00, 0x28, 0x00, 0x64, 0x00, 0x0C, 0x00, 0x28];
    ipt.resize(40, 0);
    let perf = info_in_use_performance(0x08, &ipt);
    assert!(perf.valid);
    assert!(perf.spark_ignition);
    assert_eq!(perf.obd_conditions, 40);
    assert_eq!(perf.ignition_cycles, 100);
    assert_eq!(perf.monitors.len(), 9);
    assert_eq!(perf.monitors[0].name, "Katalysator Bank 1");
    assert!(approx(perf.monitors[0].ratio, 0.3));
    assert!(perf.monitors[0].text.contains("0,30"));
    assert!(!perf.monitors[1].has_ratio);

    let diesel = info_in_use_performance(0x0B, &ipt);
    assert!(!diesel.spark_ignition);
    assert_eq!(diesel.monitors[0].name, "Oxidationskatalysator (NMHC)");
}

// ---------------------------------------------------------------------------
// End to end over a scripted adapter
// ---------------------------------------------------------------------------

fn two_ecu_car(s: &mut ScriptState) {
    // Faults: engine P0133 + P0301, transmission P0700.
    s.set(
        "03",
        reply(
            &["43020133 0301", "43010700"],
            &["7E8 06 43 02 01 33 03 01 AA", "7E9 04 43 01 07 00 AA AA AA"],
        ),
    );
    s.set(
        "0101",
        reply(
            &["41018307E500", "4101000400 00"],
            &["7E8 06 41 01 83 07 E5 00 AA", "7E9 06 41 01 00 04 00 00 AA"],
        ),
    );
    s.set(
        "010D",
        reply(
            &["410D32", "410D33"],
            &["7E8 03 41 0D 32", "7E9 03 41 0D 33"],
        ),
    );
    s.set(
        "0167",
        reply(&["4167035A64"], &["7E8 05 41 67 03 5A 64 AA AA"]),
    );
    s.set(
        "090A",
        reply(
            &[
                "017",
                "0:490A0145434D",
                "1:002D456E67696E",
                "2:65436F6E74726F",
                "3:6C0000",
            ],
            &[
                "7E8 10 17 49 0A 01 45 43 4D",
                "7E9 10 17 49 0A 01 54 43 4D",
                "7E8 21 00 2D 45 6E 67 69 6E",
                "7E9 21 00 2D 54 72 61 6E 73",
                "7E8 22 65 43 6F 6E 74 72 6F",
                "7E9 22 6D 69 73 73 69 6F 6E",
                "7E8 23 6C 00 00 AA AA AA AA",
                "7E9 23 43 74 6C AA AA AA AA",
            ],
        ),
    );
    s.set(
        "0906",
        reply(&["490601 1A2B3C4D"], &["7E8 07 49 06 01 1A 2B 3C 4D"]),
    );
    // Mode 06: monitors A1 and A2 (misfire) on the engine.
    for base in ["0600", "0620", "0640", "0660", "0680"] {
        let b = &base[2..];
        let plain = format!("46{b}00000001");
        let headed = format!("7E8 06 46 {b} 00 00 00 01 AA");
        s.set(base, reply(&[&plain], &[&headed]));
    }
    s.set(
        "06A0",
        reply(&["46A0C0000000"], &["7E8 06 46 A0 C0 00 00 00 AA"]),
    );
    s.set(
        "06A1",
        reply(
            &[
                "013",
                "0:46A10B240000",
                "1:0000FFFFA10C24",
                "2:00030000FFFF",
            ],
            &[
                "7E8 10 13 46 A1 0B 24 00 00",
                "7E8 21 00 00 FF FF A1 0C 24",
                "7E8 22 00 03 00 00 FF FF AA",
            ],
        ),
    );
    s.set(
        "06A2",
        reply(
            &[
                "013",
                "0:46A20B240002",
                "1:0000FFFFA20C24",
                "2:00050000FFFF",
            ],
            &[],
        ),
    );
}

#[test]
fn faults_and_status_per_control_unit() {
    let _lang = german();
    let (mut elm, _script) = script_rig_with("6", two_ecu_car);
    assert!(elm.is_can());

    {
        let mut obd = Obd2::new(&mut elm);
        // Headers off: the codes are found, but nobody knows from whom.
        let faults = obd.read_faults().expect("faults");
        assert_eq!(faults.len(), 3);
        assert!(faults[0].ecu.is_empty());
    }

    assert!(elm.set_headers(true));
    assert!(elm.headers_on());
    {
        let mut obd = Obd2::new(&mut elm);
        let faults = obd.read_faults().expect("faults");
        assert_eq!(faults.len(), 3);
        assert_eq!(faults[0].code, "P0133");
        assert_eq!(faults[0].ecu, "Motor (7E8)");
        assert_eq!(faults[1].code, "P0301");
        assert_eq!(faults[2].code, "P0700");
        assert_eq!(faults[2].ecu, "Getriebe (7E9)");
        assert_eq!(faults[2].ecu_source, "7E9");

        let status = obd.read_status_per_ecu();
        assert_eq!(status.len(), 2);
        assert!(status[0].mil_on);
        assert_eq!(status[0].fault_count, 3);
        assert_eq!(status[0].ecu, "Motor (7E8)");
        assert!(!status[1].mil_on);
        assert_eq!(obd.read_status().ecu_source, "7E8");

        // Live values: the first answer (engine) counts, with its sender.
        let speed = obd.read_live(0x0D).expect("speed");
        assert_eq!(speed.value, 50.0);
        assert_eq!(speed.key, 0x0D);
        assert_eq!(speed.ecu_source, "7E8");
        let coolant = obd.read_live_all(0x67);
        assert_eq!(coolant.len(), 2);
        assert_eq!(coolant[1].key, 0x167);
        assert_eq!(coolant[1].value, 60.0);
        assert_eq!(coolant[1].category, PidCategory::Temperatures);
    }

    // A reset by hand switches headers off again.
    elm.send("ATZ", 5000);
    assert!(!elm.headers_on());
}

#[test]
fn mode_09_per_control_unit() {
    let _lang = german();
    let (mut elm, _script) = script_rig_with("6", two_ecu_car);
    assert!(elm.set_headers(true));
    let mut obd = Obd2::new(&mut elm);

    let ecus = obd.read_ecu_info();
    assert_eq!(ecus.len(), 2);
    assert_eq!(ecus[0].ecu_source, "7E8");
    assert_eq!(ecus[0].name, "ECM-EngineControl");
    assert_eq!(ecus[0].cvns, ["1A2B3C4D"]);
    assert_eq!(ecus[1].ecu, "Getriebe (7E9)");
    assert_eq!(ecus[1].name, "TCM-TransmissionCtl");

    // The old identity still works and picks the first control unit.
    let id = obd.read_identity();
    assert_eq!(id.ecu_name, "ECM-EngineControl");
    assert_eq!(id.calibration_verification, "1A2B3C4D");
}

#[test]
fn calibration_ids_of_two_control_units_stay_apart_without_headers() {
    let _lang = german();
    let (mut elm, _script) = script_rig_with("6", |s| {
        two_ecu_car(s);
        // Engine with two IDs, then the transmission with one, each multi-frame.
        s.set(
            "0904",
            reply(
                &[
                    "023",
                    "0:49040230334C",
                    "1:39303630313844",
                    "2:4E000000000030",
                    "3:334C3930363031",
                    "4:38585800000000",
                    "5:00",
                    "013",
                    "0:490401303947",
                    "1:3932373735304B",
                    "2:4D0000000000",
                ],
                &[],
            ),
        );
    });
    let mut obd = Obd2::new(&mut elm);
    assert_eq!(
        obd.read_identity().calibration_id,
        "03L906018DN, 03L906018XX, 09G927750KM"
    );

    // A short ID without padding, as some K-line units send it.
    assert_eq!(
        info_calibration_ids(&info_payload(&[vec![
            0x04, 0x01, b'0', b'3', b'6', b'9', b'X'
        ]])),
        ["0369X"]
    );
}

#[test]
fn misfire_counts_come_out_of_mode_06() {
    let _lang = german();
    let (mut elm, _script) = script_rig_with("6", two_ecu_car);
    let mut obd = Obd2::new(&mut elm);

    let results = obd.read_monitor_results().expect("results");
    assert_eq!(results.len(), 4);
    assert!(results[0].can);
    assert_eq!(results[0].component_id, 0xA1);
    assert_eq!(results[1].test_id, 0x0C);
    assert_eq!(results[1].value, 3.0);
    assert_eq!(results[2].component, "Verbrennungsaussetzer Zylinder 1");
    assert_eq!(results[3].name, "Aussetzer in der aktuellen Fahrt");

    let misfires = misfire_counts(&results);
    assert_eq!(misfires.len(), 2);
    assert_eq!(misfires[0].cylinder, 0);
    assert_eq!(misfires[0].current_cycle, 3.0);
    assert_eq!(misfires[1].cylinder, 1);
    assert_eq!(misfires[1].name, "Zylinder 1");
    assert_eq!(misfires[1].current_cycle, 5.0);
    assert_eq!(misfires[1].average_ten_cycles, 2.0);
    assert_eq!(
        misfires[1].text,
        "Zylinder 1: 5 Aussetzer in der aktuellen Fahrt, im Schnitt 2,0 je Fahrt über die letzten 10 Fahrten."
    );

    // K-line style results are left alone.
    let kline = MonitorResult {
        test_id: 0x0C,
        component_id: 0xA2,
        ..MonitorResult::default()
    };
    assert!(misfire_counts(&[kline]).is_empty());
}
