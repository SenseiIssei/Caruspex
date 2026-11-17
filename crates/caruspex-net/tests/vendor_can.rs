//! PEAK PCAN-Basic and Kvaser CANlib end to end through a real DLL
//! (fake-can-vendors/, built here, exporting both libraries): the channel
//! list with only the connected hardware, opening at the bit rate, OBD with
//! ISO-TP and flow control, echoes, status and error frames skipped, and
//! clear refusals. Windows only, like the libraries.
#![cfg(windows)]

mod common;

use std::path::PathBuf;
use std::process::Command;

use caruspex_net::can_bus::{
    CanElm, CanVendor, VendorChannel, open_vendor_can, vendor_channels_in,
};
use common::MiniElm;

fn fake_library() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fake-can-vendors")
        .join("Cargo.toml");
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fake-can-vendors");
    let output = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--manifest-path"])
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&target)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_TARGET")
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "fake-can-vendors builds:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    target.join("debug").join("fake_can_vendors.dll")
}

fn vin_through(link: Box<dyn caruspex_net::CanLink>) -> String {
    let mut elm = MiniElm::new(CanElm::new(link, "vendor test", 500000));
    elm.connect("6").unwrap();
    elm.read_vin()
}

#[test]
fn pcan_basic_and_kvaser_canlib_carry_obd_on_can() {
    let library = fake_library();
    let library = library.to_str().unwrap();

    // PCAN: only the free USB channel is offered.
    assert_eq!(
        vendor_channels_in(CanVendor::Pcan, library),
        vec![VendorChannel {
            label: "PCAN · USB1".into(),
            spec: "pcan:USB1".into(),
        }]
    );
    let link = open_vendor_can(CanVendor::Pcan, library, "usb1", 500000).unwrap();
    assert_eq!(link.name(), "PCAN USB1");
    assert_eq!(vin_through(link), "WAUZZZ8V5LA012345");
    let error = open_vendor_can(CanVendor::Pcan, library, "USB1", 250000)
        .err()
        .unwrap();
    assert!(error.ends_with("fake pcan 0x8000"), "{error}");
    let error = open_vendor_can(CanVendor::Pcan, library, "USB2", 500000)
        .err()
        .unwrap();
    assert!(error.ends_with("fake pcan 0x1400"), "{error}");
    assert!(open_vendor_can(CanVendor::Pcan, library, "COM3", 500000).is_err());
    assert!(open_vendor_can(CanVendor::Pcan, library, "USB1", 123456).is_err());

    // Kvaser: the virtual channel of the driver is left out.
    let channels = vendor_channels_in(CanVendor::Kvaser, library);
    assert_eq!(channels.len(), 1, "{channels:?}");
    assert_eq!(channels[0].spec, "kvaser:0");
    assert!(
        channels[0]
            .label
            .starts_with("Kvaser · Kvaser Leaf Light v2")
    );
    let link = open_vendor_can(CanVendor::Kvaser, library, "0", 500000).unwrap();
    assert!(
        link.name().starts_with("Kvaser Leaf Light v2"),
        "{}",
        link.name()
    );
    assert_eq!(vin_through(link), "WAUZZZ8V5LA012345");
    let error = open_vendor_can(CanVendor::Kvaser, library, "0", 250000)
        .err()
        .unwrap();
    assert!(error.ends_with("fake kvaser -1"), "{error}");
    let error = open_vendor_can(CanVendor::Kvaser, library, "3", 500000)
        .err()
        .unwrap();
    assert!(error.ends_with("fake kvaser -3"), "{error}");
    assert!(open_vendor_can(CanVendor::Kvaser, library, "x", 500000).is_err());

    // Without the libraries: nothing listed, a clear message.
    assert!(vendor_channels_in(CanVendor::Pcan, "C:/no/such/PCANBasic.dll").is_empty());
    let error = open_vendor_can(CanVendor::Kvaser, "C:/no/such/canlib32.dll", "0", 500000)
        .err()
        .unwrap();
    assert!(error.contains("canlib32.dll"), "{error}");
}
