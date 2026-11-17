//! CAN FD through a J2534 device (FD_CAN of J2534-2) with the fake driver:
//! the channel is connected with the FD protocol, an FD single frame with
//! the escape length and an FD first frame of 64 bytes are put back
//! together and shown as an ELM327 shows classic answers, with and without
//! headers. FD is off unless asked for, and only offered where it exists.
//! Windows only, like J2534.
#![cfg(windows)]

mod common;
mod fake_driver;

use caruspex_link::Transport;
use caruspex_net::can_bus::{CanElm, open_can_interface, open_j2534, open_j2534_fd};
use common::MiniElm;
use fake_driver::fake_driver;

fn part_number() -> Vec<u8> {
    let mut payload = vec![0x62, 0xF1, 0x87];
    payload.extend((0..67u8).map(|i| b'A' + i % 26));
    payload
}

fn hex_bytes(text: &str) -> Vec<u8> {
    let digits: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    (0..digits.len() / 2)
        .map(|i| u8::from_str_radix(&digits[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

#[test]
fn j2534_can_fd_answers_come_out_as_classic_elm_lines() {
    let driver = fake_driver();
    let driver = driver.to_str().unwrap();

    // Off by default: the classic channel.
    let classic = open_j2534(driver, 500000).unwrap();
    assert!(!classic.fd());
    drop(classic);

    let link = open_j2534_fd(driver, 500000).unwrap();
    assert!(link.fd());
    assert_eq!(link.name(), format!("J2534 {driver} CAN FD"));
    let mut elm = MiniElm::new(CanElm::new(link, "J2534 FD", 500000));
    elm.connect("6").unwrap();
    // The VIN in one FD single frame (escape length 20).
    assert_eq!(elm.read_vin(), "WAUZZZ8V5LA012345");

    // 70 bytes in an FD first frame and one consecutive frame: as an ELM327
    // shows 70 bytes on classic CAN, length line, six bytes, then seven each
    // (the driver talks without spaces).
    let lines = elm.send("22F187", 2000);
    assert_eq!(lines[0], "046", "{lines:?}");
    assert_eq!(lines.len(), 1 + 11, "{lines:?}");
    let mut joined = Vec::new();
    for (i, line) in lines[1..].iter().enumerate() {
        let (index, bytes) = line.split_once(':').unwrap();
        assert_eq!(index, format!("{:X}", i % 16));
        joined.extend(hex_bytes(bytes));
    }
    joined.truncate(0x46);
    assert_eq!(joined, part_number());

    // With headers: classic frames of 8 bytes, the first frame in front.
    assert_eq!(elm.send("ATH1", 500), ["OK"]);
    let lines = elm.send("22F187", 2000);
    assert_eq!(lines.len(), 11, "{lines:?}");
    assert_eq!(lines[0], "7E8104662F187414243", "{lines:?}");
    assert_eq!(lines[1], "7E8214445464748494A", "{lines:?}");
    assert_eq!(lines[10], "7E82A4F000000000000", "{lines:?}");
    let lines = elm.send("0902", 2000);
    assert_eq!(lines[0], "7E81014490201574155", "{lines:?}");
    drop(elm);

    // Through the interface text: "+fd" right before the bit rate.
    let no_serial = |_: &str, _: u32| -> Result<Box<dyn Transport>, String> {
        panic!("no serial port may be opened here")
    };
    let elm = open_can_interface(&format!("j2534:{driver}+fd@500000"), &no_serial).unwrap();
    assert!(
        elm.name().ends_with("CAN FD (500 kbit/s)"),
        "{}",
        elm.name()
    );
    drop(elm);
    let error = open_can_interface("slcan:COM9+fd@500000", &no_serial)
        .err()
        .unwrap();
    assert!(error.contains("CAN FD"), "{error}");
    let error = open_can_interface("pcan:USB1+fd", &no_serial)
        .err()
        .unwrap();
    assert!(error.contains("CAN FD"), "{error}");
}
