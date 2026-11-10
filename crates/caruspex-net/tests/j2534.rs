//! J2534 end to end through a real driver DLL (fake-j2534/, built here):
//! loading, connecting CAN, the pass filter, own echoes skipped, ISO-TP with
//! flow control and the battery voltage. Windows only, like J2534 itself.
#![cfg(windows)]

mod common;

use std::path::PathBuf;
use std::process::Command;

use caruspex_net::can_bus::{CanElm, open_j2534};
use common::MiniElm;

/// Builds the fake PassThru driver and returns the path of its DLL.
fn fake_driver() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fake-j2534")
        .join("Cargo.toml");
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fake-j2534");
    // The output (linker notes included) only matters when the build fails.
    let output = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&target)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_TARGET")
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "fake-j2534 builds:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    target.join("debug").join("fake_j2534.dll")
}

#[test]
fn j2534_a_passthru_driver_dll_carries_obd_on_can() {
    let driver = fake_driver();
    let driver = driver.to_str().unwrap();
    let mut link = open_j2534(driver, 500000).unwrap();
    assert!(link.battery_voltage().unwrap() > 12.4);
    assert_eq!(link.name(), format!("J2534 {driver}"));
    let mut elm = MiniElm::new(CanElm::new(link, "J2534 test", 500000));
    elm.connect("6").unwrap();
    assert_eq!(elm.read_vin(), "WAUZZZ8V5LA012345");
    assert_eq!(
        elm.send("ATRV", 1000).first().map(String::as_str),
        Some("12.5V")
    );

    // A wrong bit rate is refused by the driver with a clear message.
    let error = open_j2534(driver, 250000).err().unwrap();
    assert!(error.ends_with("fake driver"), "{error}");
    assert!(open_j2534("C:/no/such/driver.dll", 500000).is_err());
    assert!(open_j2534("", 500000).is_err());
}
