//! The answer count digit after a mode 01 request (ELM327 v1.3 and later):
//! learned from a plain request, used on the next identical one, dropped
//! for the session when the adapter does not take it.

mod common;

use std::sync::{Arc, Mutex};

use caruspex_link::{Elm327, Transport};
use common::*;

fn connect(transport: Box<dyn Transport>) -> (Elm327, Arc<Mutex<Vec<String>>>) {
    let mut elm = Elm327::new();
    elm.set_verbose(true);
    let log = capture_log(&mut elm);
    elm.connect(transport, "6").expect("connects");
    // As the worker does on CAN.
    assert!(elm.set_headers(true));
    log.lock().unwrap().clear();
    (elm, log)
}

fn sent(log: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    sent_commands(&log.lock().unwrap())
}

/// The fake demo adapter (a v1.5 clone with the v1.4 command set) with the
/// gearbox, answering at once.
fn demo() -> Box<dyn Transport> {
    Box::new(FakeAdapter::new(DemoAdapter::CloneV15, true, 0.0))
}

#[test]
fn a_repeated_request_carries_the_answer_count() {
    let (mut elm, log) = connect(demo());
    let first = elm.request("0100", 0x01, 4000);
    let second = elm.request("0100", 0x01, 4000);
    let third = elm.request("0100", 0x01, 4000);
    assert!(first.ok(), "{first:?}");
    // Engine and gearbox answer: two frames, so the digit is 2.
    assert_eq!(sent(&log), ["0100", "01002", "01002"]);
    assert!(elm.answer_count_hint());
    // The same answers either way.
    assert_eq!(first.messages.len(), second.messages.len());
    assert_eq!(first.sources, second.sources);
    assert_eq!(second.sources, third.sources);
}

#[test]
fn settings_in_between_make_it_learn_again() {
    let (mut elm, log) = connect(demo());
    elm.request("0100", 0x01, 4000);
    elm.send("ATSH7E0", 2000);
    elm.request("0100", 0x01, 4000);
    elm.request("0100", 0x01, 4000);
    let commands = sent(&log);
    assert_eq!(commands, ["0100", "ATSH7E0", "0100", "01002"]);
}

#[test]
fn only_mode_01_and_never_when_switched_off() {
    let transport = InterceptTransport::new(demo(), |cmd| {
        (cmd == "090A").then(|| answer(&["7E8 03 49 0A 01"]))
    });
    let (mut elm, log) = connect(Box::new(transport));
    assert!(elm.request("090A", 0x09, 4000).ok());
    assert!(elm.request("090A", 0x09, 4000).ok());
    elm.set_answer_count_hint(false);
    elm.request("0100", 0x01, 4000);
    elm.request("0100", 0x01, 4000);
    assert_eq!(sent(&log), ["090A", "090A", "0100", "0100"]);
    assert!(!elm.answer_count_hint());
}

/// An adapter older than v1.3 (or a clone that only says so): "?" to the
/// digit.
#[test]
fn an_adapter_that_refuses_the_digit_gets_plain_requests() {
    let inner = demo();
    let transport = InterceptTransport::new(inner, |cmd| {
        let hex = cmd.chars().all(|c| c.is_ascii_hexdigit());
        (hex && cmd.len() % 2 == 1).then(|| answer(&["?"]))
    });
    let (mut elm, log) = connect(Box::new(transport));
    for _ in 0..4 {
        let r = elm.request("0100", 0x01, 4000);
        assert!(r.ok(), "{r:?}");
    }
    assert_eq!(sent(&log)[..4], ["0100", "01002", "0100", "0100"]);
    assert_eq!(sent(&log).len(), 5);
    assert!(!elm.answer_count_hint());
}

/// A clone that takes the digit but loses the answer with it.
#[test]
fn a_clone_that_loses_answers_with_the_digit_falls_back() {
    let inner = demo();
    let transport = InterceptTransport::new(inner, |cmd| {
        let hex = cmd.chars().all(|c| c.is_ascii_hexdigit());
        (hex && cmd.len() % 2 == 1).then(|| answer(&["NO DATA"]))
    });
    let (mut elm, log) = connect(Box::new(transport));
    for _ in 0..6 {
        let r = elm.request("0100", 0x01, 4000);
        assert!(r.ok(), "every request still gets its value: {r:?}");
    }
    let commands = sent(&log);
    // Two misses, each followed by the plain request, then plain only.
    assert_eq!(commands.iter().filter(|c| *c == "01002").count(), 2);
    assert!(!elm.answer_count_hint());
    assert_eq!(commands.last().unwrap(), "0100");
}

/// A control unit that stops answering is the car's doing: the digit stays.
#[test]
fn no_data_on_the_plain_way_too_is_not_held_against_the_digit() {
    let silent = Arc::new(Mutex::new(false));
    let flag = silent.clone();
    let transport = InterceptTransport::new(demo(), move |cmd| {
        (*flag.lock().unwrap() && cmd.starts_with("0100")).then(|| answer(&["NO DATA"]))
    });
    let (mut elm, log) = connect(Box::new(transport));
    assert!(elm.request("0100", 0x01, 4000).ok());
    *silent.lock().unwrap() = true;
    for _ in 0..3 {
        assert!(!elm.request("0100", 0x01, 4000).ok());
    }
    *silent.lock().unwrap() = false;
    assert!(elm.request("0100", 0x01, 4000).ok());
    assert!(elm.request("0100", 0x01, 4000).ok());
    assert!(elm.answer_count_hint());
    // The digit, both ways once, then plain while nothing answers, learned
    // again, and the digit once more.
    assert_eq!(
        sent(&log),
        ["0100", "01002", "0100", "0100", "0100", "0100", "01002"]
    );
}

/// Now and then the plain way, so a unit that starts to answer is heard.
#[test]
fn every_32nd_request_goes_out_plain() {
    let (mut elm, log) = connect(demo());
    for _ in 0..40 {
        assert!(elm.request("0100", 0x01, 4000).ok());
    }
    let commands = sent(&log);
    let plain: Vec<usize> = (0..commands.len())
        .filter(|&i| commands[i] == "0100")
        .collect();
    assert_eq!(plain, [0, 32]);
}

/// One unit missing from an answer with the digit does not lower the count:
/// the next request still waits for both.
#[test]
fn a_short_answer_with_the_digit_keeps_the_count() {
    let mut first = true;
    let transport = InterceptTransport::new(demo(), move |cmd| {
        (cmd == "01002" && std::mem::take(&mut first))
            .then(|| answer(&["7E8 06 41 00 BE 1F A8 13"]))
    });
    let (mut elm, log) = connect(Box::new(transport));
    for _ in 0..3 {
        assert!(elm.request("0100", 0x01, 4000).ok());
    }
    assert_eq!(sent(&log), ["0100", "01002", "01002"]);
}
