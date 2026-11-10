//! The adapter probe against the fake demo adapter in all its variants
//! (tests/test_probe.cpp).

mod common;

use caruspex_link::adapter_probe::{parse_version_level, probe_adapter, version_text};
use common::{DemoAdapter, DemoRig};

#[test]
fn adapter_probe_v1_5_clone() {
    let mut rig = DemoRig::new(DemoAdapter::CloneV15, true, 0.0);
    assert!(rig.connected, "{}", rig.error);
    rig.sent.lock().unwrap().clear();
    let report = probe_adapter(&mut rig.elm);

    assert!(report.reachable);
    assert_eq!(report.version_text, "ELM327 v1.5");
    assert_eq!(report.claimed_level, 150);
    assert!(!report.claimed_version_exists);
    assert_eq!(report.description, "OBDII to RS232 Interpreter");
    assert!(!report.is_stn);
    assert!(!report.accepts_unknown_commands);
    assert!(report.has_voltage);
    assert!((report.voltage - 14.1).abs() <= 0.2);
    assert_eq!(report.behaves_like_level, 140);
    assert_eq!(report.round_trip_samples, 5);
    assert!(report.response_count_hint);
    assert!(report.verdict.en.contains("Probably a clone"));
    assert!(report.verdict.en.contains("v1.4"));
    assert!(report.verdict.de.contains("Nachbau"));
    assert!(!report.facts.is_empty());

    // Nothing that writes to the adapter's memory went out.
    for line in rig.sent.lock().unwrap().iter() {
        assert!(!line.contains("> ATPP0"), "{line}");
        assert!(!line.contains("SV"), "{line}");
        assert!(!line.contains("> AT@3"), "{line}");
        assert!(!line.contains("> ATSD"), "{line}");
        assert!(!line.contains("> STSBR"), "{line}");
        assert!(!line.contains("> ATZ"), "{line}");
    }
}

#[test]
fn adapter_probe_genuine_v1_4b_behaviour() {
    let mut rig = DemoRig::new(DemoAdapter::GenuineV14b, true, 0.0);
    assert!(rig.connected);
    let report = probe_adapter(&mut rig.elm);
    assert_eq!(report.claimed_level, 142);
    assert!(report.claimed_version_exists);
    assert_eq!(report.behaves_like_level, 142);
    assert!(report.verdict.en.contains("matches a genuine ELM327 v1.4b"));
    assert!(report.verdict.en.contains("cannot be told apart"));
    assert!(report.verdict.de.contains("echten ELM327 v1.4b"));
}

#[test]
fn adapter_probe_clone_that_claims_v2_1() {
    let mut rig = DemoRig::new(DemoAdapter::CloneV21, true, 0.0);
    assert!(rig.connected);
    let report = probe_adapter(&mut rig.elm);
    assert_eq!(report.claimed_level, 210);
    assert_eq!(report.behaves_like_level, 140);
    assert!(
        report
            .verdict
            .en
            .contains("claims v2.1 but behaves like v1.4")
    );
    assert!(report.verdict.de.contains("als v2.1 ausgibt"));
}

#[test]
fn adapter_probe_clone_that_says_ok_to_everything() {
    let mut rig = DemoRig::new(DemoAdapter::CloneOkToAll, true, 0.0);
    assert!(rig.connected);
    let report = probe_adapter(&mut rig.elm);
    assert!(report.accepts_unknown_commands);
    assert!(report.verdict.en.contains("cannot be tested reliably"));
}

#[test]
fn adapter_probe_stn_chip() {
    let mut rig = DemoRig::new(DemoAdapter::ObdLinkStn, true, 0.0);
    assert!(rig.connected);
    let report = probe_adapter(&mut rig.elm);
    assert!(report.is_stn);
    assert!(report.stn_id.starts_with("STN"));
    assert_eq!(report.stn_device, "OBDLink SX r4.2");
    assert!(report.verdict.en.contains("STN chip"));
    assert!(report.speed_tips.len() >= 2);
}

#[test]
fn adapter_probe_measures_the_demos_answer_delay() {
    let mut rig = DemoRig::new(DemoAdapter::CloneV15, true, 1.0);
    assert!(rig.connected);
    let report = probe_adapter(&mut rig.elm);
    assert_eq!(report.round_trip_samples, 5);
    assert!(report.round_trip_ms >= 5.0);
    assert!(report.response_count_hint);
}

#[test]
fn elm327_version_strings() {
    assert_eq!(parse_version_level("ELM327 v1.5"), 150);
    assert_eq!(parse_version_level("ELM327 v1.4b"), 142);
    assert_eq!(parse_version_level("ELM327 v2.1"), 210);
    assert_eq!(parse_version_level("ELM327 1.3a"), 131);
    assert_eq!(parse_version_level("OBDII"), 0);
    assert_eq!(version_text(142), "v1.4b");
    assert_eq!(version_text(200), "v2.0");
}

#[test]
fn a_silent_adapter_cannot_be_tested() {
    let mut elm = caruspex_link::Elm327::new();
    let report = probe_adapter(&mut elm);
    assert!(!report.reachable);
    assert!(report.verdict.en.contains("does not answer"));
}
