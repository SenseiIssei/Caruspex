//! Several PIDs per request on CAN (read_live_batch) and freeze frames per
//! fault (read_freeze_frames), tests/test_obd_batch.cpp. The recorded byte
//! examples are worked out by hand from SAE J1979 and ISO 15765-2; the demo
//! car checks the same end to end.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use caruspex_data::pids::default_live_pids;
use caruspex_demo::DemoAdapter;
use caruspex_protocols::obd2::{
    DEFAULT_FREEZE_FRAMES, LiveValue, Obd2, batchable_pid, split_multi_pid_answer,
};
use common::*;

/// The single answers the fallback needs.
fn add_singles(s: &mut ScriptState) {
    s.set(
        "010C",
        reply(&["410C1AF8"], &["7E8 04 41 0C 1A F8 AA AA AA"]),
    );
    s.set(
        "010D",
        reply(
            &["410D32", "410D33"],
            &["7E8 03 41 0D 32 AA AA AA AA", "7E9 03 41 0D 33 AA AA AA AA"],
        ),
    );
    s.set("0105", reply(&["41057B"], &["7E8 03 41 05 7B AA AA AA AA"]));
    s.set("0111", reply(&["411140"], &["7E8 03 41 11 40 AA AA AA AA"]));
    s.set("010F", reply(&["410F48"], &["7E8 03 41 0F 48 AA AA AA AA"]));
    s.set("0104", reply(&["410480"], &["7E8 03 41 04 80 AA AA AA AA"]));
}

/// Six PIDs: 41 0C 1A F8 0D 32 05 7B 11 40 0F 48 04 80 is fourteen bytes, too
/// many for one CAN frame. First frame with six bytes, then two consecutive
/// frames. The transmission answers the speed in a single frame of its own.
fn six_pid_car(s: &mut ScriptState) {
    s.set(
        "010C0D05110F04",
        reply(
            &[
                "00E",
                "0:410C1AF80D32",
                "1:057B11400F4804",
                "2:80000000000000",
                "410D33",
            ],
            &[
                "7E8 10 0E 41 0C 1A F8 0D 32",
                "7E9 03 41 0D 33 AA AA AA AA",
                "7E8 21 05 7B 11 40 0F 48 04",
                "7E8 22 80 AA AA AA AA AA AA",
            ],
        ),
    );
    add_singles(s);
}

const SIX: [i32; 6] = [0x0C, 0x0D, 0x05, 0x11, 0x0F, 0x04];

fn check_six_values(values: &[LiveValue]) {
    let mut by_key = BTreeMap::new();
    for v in values {
        by_key.entry(v.key).or_insert(v.value);
    }
    assert!(approx(by_key[&0x0C], 1726.0));
    assert!(approx(by_key[&0x0D], 50.0));
    assert!(approx(by_key[&0x05], 83.0));
    assert!(approx(by_key[&0x11], 64.0 * 100.0 / 255.0));
    assert!(approx(by_key[&0x0F], 32.0));
    assert!(approx(by_key[&0x04], 128.0 * 100.0 / 255.0));
}

fn obd_sent(script: &Script) -> Vec<String> {
    script.lock().unwrap().obd_sent.clone()
}

fn clear_obd_sent(script: &Script) {
    script.lock().unwrap().obd_sent.clear();
}

// ---------------------------------------------------------------------------
// Splitting a combined answer
// ---------------------------------------------------------------------------

#[test]
fn a_combined_answer_splits_by_the_pid_table() {
    let _lang = german();
    let blocks = split_multi_pid_answer(
        &[0x0C, 0x1A, 0xF8, 0x0D, 0x32, 0x05, 0x7B],
        &[0x0C, 0x0D, 0x05],
    )
    .expect("adds up");
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].0, 0x0C);
    assert_eq!(blocks[0].1, [0x1A, 0xF8]);
    assert_eq!(blocks[1].1, [0x32]);
    assert_eq!(blocks[2].1, [0x7B]);

    // The control unit may leave out what it does not support, and it may
    // answer in a different order.
    let blocks =
        split_multi_pid_answer(&[0x05, 0x7B, 0x0C, 0x1A, 0xF8], &[0x0C, 0x0D, 0x05]).unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].0, 0x05);

    // Padding after the last block is fine.
    let blocks = split_multi_pid_answer(&[0x0D, 0x32, 0xAA, 0xAA, 0x00], &[0x0D]).unwrap();
    assert_eq!(blocks.len(), 1);

    // A multi-value PID takes all its bytes: 0x67 is three.
    let blocks =
        split_multi_pid_answer(&[0x67, 0x03, 0x5A, 0x64, 0x0D, 0x32], &[0x67, 0x0D]).unwrap();
    assert_eq!(blocks[0].1.len(), 3);
    assert_eq!(blocks[1].0, 0x0D);

    // What does not add up is refused.
    assert!(split_multi_pid_answer(&[0x0C, 0x1A], &[0x0C]).is_none()); // short
    assert!(split_multi_pid_answer(&[0x0C, 0x1A, 0xF8, 0x0E, 0x80], &[0x0C]).is_none()); // not asked
    assert!(split_multi_pid_answer(&[0x0D, 0x32, 0x0D, 0x33], &[0x0D]).is_none()); // twice
    assert!(split_multi_pid_answer(&[], &[0x0D]).is_none());
    assert!(split_multi_pid_answer(&[0xAA], &[0x0D]).is_none()); // padding only

    assert!(batchable_pid(0x0C));
    assert!(batchable_pid(0x67));
    assert!(!batchable_pid(0x68)); // three or seven bytes, depending on the car
    assert!(!batchable_pid(0x00));
    assert!(!batchable_pid(0x20));
    assert!(!batchable_pid(0x300));
}

// ---------------------------------------------------------------------------
// Recorded answers
// ---------------------------------------------------------------------------

#[test]
fn six_pids_in_one_request_headers_off_answer_over_three_frames() {
    let _lang = german();
    let (mut elm, script) = script_rig_with("6", six_pid_car);
    let mut obd = Obd2::new(&mut elm);

    clear_obd_sent(&script);
    let values = obd.read_live_batch(&SIX, false);
    assert_eq!(obd_sent(&script), ["010C0D05110F04"]);
    assert_eq!(values.len(), 6);
    check_six_values(&values);
    // In the order asked, the engine first (it answered first).
    assert_eq!(values[0].pid, 0x0C);
    assert_eq!(values[5].pid, 0x04);
    assert!(approx(values[1].value, 50.0));
    assert!(values[1].ecu.is_empty());

    // Every control unit: the transmission's speed comes along.
    let every = obd.read_live_batch(&SIX, true);
    assert_eq!(every.len(), 7);
    assert_eq!(every[1].pid, 0x0D);
    assert_eq!(every[2].pid, 0x0D);
    assert!(approx(every[2].value, 51.0));
    assert!(obd.batch_refused_pids().is_empty());

    // read_live_values rides on the same request.
    clear_obd_sent(&script);
    let primary = obd.read_live_values(&SIX);
    assert_eq!(obd_sent(&script).len(), 1);
    assert_eq!(primary.len(), 6);
    check_six_values(&primary);
}

#[test]
fn six_pids_in_one_request_headers_on_frames_of_two_control_units_interleaved() {
    let _lang = german();
    let (mut elm, script) = script_rig_with("6", six_pid_car);
    assert!(elm.set_headers(true));
    let mut obd = Obd2::new(&mut elm);

    clear_obd_sent(&script);
    let values = obd.read_live_batch(&SIX, false);
    assert_eq!(obd_sent(&script).len(), 1);
    // The single frame of 7E9 is complete before the multi-frame answer of
    // 7E8 is. As in read_live_all the first complete answer keeps the PID, so
    // the speed comes from the transmission here.
    assert_eq!(values.len(), 6);
    let speed = values.iter().find(|v| v.pid == 0x0D).expect("speed");
    assert_eq!(speed.ecu_source, "7E9");
    assert!(approx(speed.value, 51.0));
    let rpm = values.iter().find(|v| v.pid == 0x0C).expect("rpm");
    assert_eq!(rpm.ecu_source, "7E8");
    assert!(approx(rpm.value, 1726.0));

    let values = obd.read_live_batch(&SIX, true);
    assert_eq!(values.len(), 7);
    let from_engine = values.iter().filter(|v| v.ecu_source == "7E8").count();
    let from_gearbox = values.iter().filter(|v| v.ecu_source == "7E9").count();
    assert_eq!(from_engine, 6);
    assert_eq!(from_gearbox, 1);
}

#[test]
fn a_refused_or_empty_group_falls_back_to_single_pids_and_is_remembered() {
    let _lang = german();
    let (mut elm, script) = script_rig_with("6", |s| {
        add_singles(s);
        // NO DATA for the group (no reply scripted), a refusal for the second.
        s.set(
            "01110F04",
            reply(&["7F0112"], &["7E8 03 7F 01 12 AA AA AA AA"]),
        );
    });
    let mut obd = Obd2::new(&mut elm);

    clear_obd_sent(&script);
    let values = obd.read_live_batch(&[0x0C, 0x0D, 0x05], false);
    assert_eq!(obd_sent(&script), ["010C0D05", "010C", "010D", "0105"]);
    assert_eq!(values.len(), 3);
    assert_eq!(obd.batch_refused_pids().len(), 3);

    // Next time no group is tried for them.
    clear_obd_sent(&script);
    let values = obd.read_live_batch(&[0x0C, 0x0D, 0x05], false);
    assert_eq!(obd_sent(&script), ["010C", "010D", "0105"]);
    assert_eq!(values.len(), 3);

    clear_obd_sent(&script);
    let values = obd.read_live_batch(&[0x11, 0x0F, 0x04], false);
    assert_eq!(obd_sent(&script), ["01110F04", "0111", "010F", "0104"]);
    assert_eq!(values.len(), 3);
    assert_eq!(obd.batch_refused_pids().len(), 6);

    obd.reset_batch_memory();
    clear_obd_sent(&script);
    obd.read_live_batch(&[0x11, 0x0F], false);
    assert_eq!(obd_sent(&script)[0], "01110F");

    // The memory can be carried over to a new Obd2 on the same session.
    obd.read_live_batch(&[0x0C, 0x0D, 0x05], false);
    let memory = obd.batch_refused_pids().clone();
    drop(obd);
    let mut again = Obd2::with_batch_memory(&mut elm, memory);
    clear_obd_sent(&script);
    again.read_live_batch(&[0x0C, 0x0D, 0x05], false);
    assert_eq!(obd_sent(&script), ["010C", "010D", "0105"]);
}

#[test]
fn a_control_unit_that_answers_only_the_first_pid_of_a_group() {
    let _lang = german();
    let (mut elm, script) = script_rig_with("6", |s| {
        add_singles(s);
        s.set(
            "010C0D05",
            reply(&["410C1AF8"], &["7E8 04 41 0C 1A F8 AA AA AA"]),
        );
        s.set(
            "010C",
            reply(&["410C1B00"], &["7E8 04 41 0C 1B 00 AA AA AA"]),
        );
    });
    let mut obd = Obd2::new(&mut elm);

    clear_obd_sent(&script);
    let values = obd.read_live_batch(&[0x0C, 0x0D, 0x05], false);
    assert_eq!(obd_sent(&script), ["010C0D05", "010D", "0105"]);
    assert_eq!(values.len(), 3);
    assert!(approx(values[0].value, 1726.0));
    assert_eq!(*obd.batch_refused_pids(), BTreeSet::from([0x0D, 0x05]));

    // 0C is still fine in groups; alone in its group it is asked singly.
    clear_obd_sent(&script);
    let values = obd.read_live_batch(&[0x0C, 0x0D, 0x05], false);
    assert_eq!(obd_sent(&script), ["010C", "010D", "0105"]);
    assert!(approx(values[0].value, 1728.0));
}

#[test]
fn an_answer_that_does_not_add_up_spoils_the_whole_group() {
    let _lang = german();
    let (mut elm, script) = script_rig_with("6", |s| {
        add_singles(s);
        // The speed is missing its data byte, so 05 would be read as the speed.
        s.set(
            "010C0D05",
            reply(&["410C1AF80D057B"], &["7E8 06 41 0C 1A F8 0D 05 7B AA"]),
        );
    });
    let mut obd = Obd2::new(&mut elm);

    clear_obd_sent(&script);
    let values = obd.read_live_batch(&[0x0C, 0x0D, 0x05], false);
    assert_eq!(obd_sent(&script), ["010C0D05", "010C", "010D", "0105"]);
    assert_eq!(values.len(), 3);
    assert!(approx(values[1].value, 50.0));
    assert!(approx(values[2].value, 83.0));
}

#[test]
fn without_can_every_pid_is_asked_on_its_own() {
    let _lang = german();
    let (mut elm, script) = script_rig_with("3", add_singles); // ISO 9141-2
    assert!(!elm.is_can());
    let mut obd = Obd2::new(&mut elm);

    clear_obd_sent(&script);
    let values = obd.read_live_batch(&[0x0C, 0x0D, 0x05], false);
    assert_eq!(obd_sent(&script), ["010C", "010D", "0105"]);
    assert_eq!(values.len(), 3);
    assert!(obd.batch_refused_pids().is_empty());
}

// ---------------------------------------------------------------------------
// The demo car
// ---------------------------------------------------------------------------

#[test]
fn batch_reads_on_the_demo_car_match_single_reads() {
    let _lang = german();
    for headers in [false, true] {
        let mut rig = DemoRig::new(50.0 + 3.0 * 120.0 + 70.0); // cruising, the clock stands still
        assert!(rig.connected, "headers {headers}");
        if headers {
            assert!(rig.elm.set_headers(true));
        }
        let sent = rig.sent.clone();
        let count = |prefix: &str| {
            let needle = format!("  > {prefix}");
            sent.lock()
                .unwrap()
                .iter()
                .filter(|l| l.starts_with(&needle))
                .count()
        };
        let clear = || sent.lock().unwrap().clear();
        let mut obd = Obd2::new(&mut rig.elm);

        // 0x01 has sub-values and is answered by both control units,
        // 0x68 is never grouped, 0x0A is not supported by the demo.
        let pids = [
            0x01, 0x0C, 0x0D, 0x05, 0x04, 0x11, 0x0F, 0x10, 0x0B, 0x06, 0x07, 0x42, 0x5C, 0xA4,
            0x68, 0x0A,
        ];
        let mut single = Vec::new();
        for pid in pids {
            single.extend(obd.read_live_all(pid));
        }

        clear();
        let batch = obd.read_live_batch(&pids, false);
        assert_eq!(batch.len(), single.len(), "headers {headers}");
        for (b, s) in batch.iter().zip(&single) {
            assert_eq!(b.key, s.key, "key {}", b.key);
            assert!(approx(b.value, s.value), "key {}", b.key);
            assert_eq!(b.ecu_source, s.ecu_source, "key {}", b.key);
            assert_eq!(b.ecu, s.ecu, "key {}", b.key);
        }
        assert!(batch.iter().any(|v| v.sub > 0));
        // Groups of 6 + 6 + 3, then 0x68 alone and 0x0A, which the group
        // answer left out, once more on its own. The second time 0x0A is
        // no longer in a group: 6 + 6 + 2 and the same two singles.
        assert_eq!(count("01"), 5);
        assert_eq!(*obd.batch_refused_pids(), BTreeSet::from([0x0A]));

        clear();
        obd.read_live_batch(&pids, false);
        assert_eq!(count("01"), 5);

        if headers {
            // PID 01 from both control units.
            let every = obd.read_live_batch(&[0x01, 0xA4, 0x0C], true);
            assert!(every.iter().any(|v| v.pid == 0x01 && v.ecu_source == "7E9"));
            assert!(every.iter().any(|v| v.pid == 0x01 && v.ecu_source == "7E8"));
        }
    }
}

/// Not run by default: prints how many values per second the demo car gives
/// with its simulated answer times, one PID per request against six.
#[test]
#[ignore = "benchmark"]
fn demo_speed_of_single_and_batch_reads() {
    let _lang = german();
    let mut rig = DemoRig::with(500.0, DemoAdapter::CloneV15, true, 1.0);
    assert!(rig.connected);
    rig.elm.set_verbose(false);
    let mut obd = Obd2::new(&mut rig.elm);
    let pids = default_live_pids();

    let mut rate = |batch: bool| {
        let mut values = 0usize;
        let start = Instant::now();
        for _ in 0..10 {
            if batch {
                values += obd.read_live_batch(pids, false).len();
            } else {
                for &pid in pids {
                    values += obd.read_live_all(pid).len();
                }
            }
        }
        values as f64 / start.elapsed().as_secs_f64()
    };
    let single = rate(false);
    let batch = rate(true);
    println!(
        "{} PIDs: single {single:.0} values/s, batch {batch:.0} values/s, {:.1} times as fast",
        pids.len(),
        batch / single
    );
    assert!(batch > single);
}

// ---------------------------------------------------------------------------
// Freeze frames
// ---------------------------------------------------------------------------

#[test]
fn freeze_frames_per_fault_on_the_demo_car() {
    let _lang = german();
    let mut rig = DemoRig::new(1400.0);
    assert!(rig.connected);
    let clock = rig.clock.clone();
    let mut obd = Obd2::new(&mut rig.elm);

    // Before clearing: the stored lean mixture in frame 0, the pending
    // misfire in frame 1.
    let frames = obd.read_freeze_frames(DEFAULT_FREEZE_FRAMES);
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].frame, 0);
    assert_eq!(frames[0].dtc, "P0171");
    assert_eq!(frames[1].frame, 1);
    assert_eq!(frames[1].dtc, "P0300");
    assert!(frames[0].ecu.is_empty());
    for f in &frames {
        // The demo lists twelve PIDs besides 02; 03 has two values.
        assert!(f.values.len() >= 12);
        assert!(f.values.iter().all(|v| v.pid != 0x02));
    }
    let value = |f: &caruspex_protocols::FreezeFrame, key: i32| {
        f.values
            .iter()
            .find(|v| v.key == key)
            .map(|v| v.value)
            .expect("key missing")
    };
    assert!(near(value(&frames[0], 0x06), 7.0, 1.0)); // short term trim, lean
    assert!(value(&frames[0], 0x0C) > 500.0);
    assert!(value(&frames[1], 0x0C) > 500.0);

    // The old call still gives frame 0.
    let (rows, trigger) = obd.read_freeze_frame();
    assert!(!rows.is_empty());
    assert_eq!(trigger, "P0171");

    // Only as many as asked for.
    assert_eq!(obd.read_freeze_frames(1).len(), 1);

    // With headers every frame knows its control unit.
    assert!(obd.elm().set_headers(true));
    let frames = obd.read_freeze_frames(DEFAULT_FREEZE_FRAMES);
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[1].ecu_source, "7E8");
    assert!(!frames[1].ecu.is_empty());
    assert!(frames[1].values.len() >= 12);
    assert_eq!(frames[1].values[0].ecu_source, "7E8");
    assert!(obd.elm().set_headers(false));

    // Clearing empties every frame.
    obd.clear_faults().expect("cleared");
    assert!(obd.read_freeze_frames(DEFAULT_FREEZE_FRAMES).is_empty());

    // The misfire comes back first, then the lean mixture.
    clock.advance(200.0);
    let frames = obd.read_freeze_frames(DEFAULT_FREEZE_FRAMES);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].dtc, "P0300");
    clock.advance(100.0);
    let frames = obd.read_freeze_frames(DEFAULT_FREEZE_FRAMES);
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].dtc, "P0300");
    assert_eq!(frames[1].dtc, "P0171");
}

#[test]
fn freeze_frames_from_recorded_answers() {
    let _lang = german();
    let (mut elm, script) = script_rig_with("6", |s| {
        // Frame 0: P0301 with rpm and speed; frame 1: P0420; frame 2 empty.
        s.set(
            "020200",
            reply(&["42020003 01"], &["7E8 05 42 02 00 03 01 AA AA"]),
        );
        s.set(
            "020201",
            reply(&["42020104 20"], &["7E8 05 42 02 01 04 20 AA AA"]),
        );
        s.set(
            "020202",
            reply(&["4202020000"], &["7E8 05 42 02 02 00 00 AA AA"]),
        );
        // Supported PIDs per frame: 02, 0C and 0D. PID n is bit 32 - n of the
        // mask: 02 -> 0x40000000, 0C -> 0x00100000, 0D -> 0x00080000.
        s.set(
            "020000",
            reply(&["42000040180000"], &["7E8 07 42 00 00 40 18 00 00"]),
        );
        s.set(
            "020001",
            reply(&["42000140180000"], &["7E8 07 42 00 01 40 18 00 00"]),
        );
        s.set(
            "020C00",
            reply(&["420C001AF8"], &["7E8 05 42 0C 00 1A F8 AA AA"]),
        );
        s.set(
            "020D00",
            reply(&["420D0032"], &["7E8 04 42 0D 00 32 AA AA AA"]),
        );
        s.set(
            "020C01",
            reply(&["420C010FA0"], &["7E8 05 42 0C 01 0F A0 AA AA"]),
        );
        // A control unit that ignores the frame number would answer frame 0.
        s.set(
            "020D01",
            reply(&["420D0032"], &["7E8 04 42 0D 00 32 AA AA AA"]),
        );
    });

    for headers in [false, true] {
        assert!(elm.set_headers(headers));
        let mut obd = Obd2::new(&mut elm);
        let frames = obd.read_freeze_frames(8);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].dtc, "P0301");
        assert_eq!(frames[0].values.len(), 2);
        assert_eq!(frames[0].values[0].key, 0x0C);
        assert!(approx(frames[0].values[0].value, 1726.0));
        assert!(approx(frames[0].values[1].value, 50.0));
        assert_eq!(frames[1].dtc, "P0420");
        assert_eq!(frames[1].values.len(), 1); // the frame 0 speed is not taken
        assert!(approx(frames[1].values[0].value, 1000.0));
        assert_eq!(frames[1].ecu_source, if headers { "7E8" } else { "" });
    }
    // Frame 3 was never asked: the empty frame 2 ended the search.
    assert!(!obd_sent(&script).iter().any(|c| c == "020203"));
}
