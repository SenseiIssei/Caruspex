//! J2534 end to end through a real driver DLL (fake-j2534/, built here):
//! loading, connecting CAN, the pass filter, own echoes skipped, ISO-TP with
//! flow control and the battery voltage. Windows only, like J2534 itself.
#![cfg(windows)]

mod common;
mod fake_driver;

use caruspex_net::can_bus::{CanElm, open_j2534};
use common::MiniElm;
use fake_driver::fake_driver;

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
