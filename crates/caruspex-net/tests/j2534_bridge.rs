//! A 32-bit PassThru driver through the 32-bit helper program: the fake
//! driver (fake-j2534/) and caruspex-j2534-bridge are built for
//! i686-pc-windows-msvc, and the 64-bit test talks to the driver over the
//! named pipe exactly as to a 64-bit one: CAN with ISO-TP and flow control,
//! the battery voltage, the K-line with the 5-baud init. Needs the 32-bit
//! standard library (`rustup target add i686-pc-windows-msvc`); without it
//! the test says so and passes.
#![cfg(windows)]

mod common;
mod fake_driver;

use caruspex_net::can_bus::{CanElm, KLineKind, open_j2534, open_j2534_kline, set_j2534_bridge};
use caruspex_protocols::kwp1281::Ecu;
use caruspex_protocols::vag_kline::VagKLine;
use common::MiniElm;
use fake_driver::{bridge_32, fake_driver_32, target_32_installed};

#[test]
fn j2534_a_32_bit_driver_works_through_the_bridge() {
    if !target_32_installed() {
        eprintln!("i686-pc-windows-msvc is not installed: 32-bit bridge not tested");
        return;
    }
    let driver = fake_driver_32();
    let driver = driver.to_str().unwrap();
    assert!(caruspex_j2534::is_32bit_dll(driver));
    let bridge = bridge_32();
    assert!(caruspex_j2534::is_32bit_dll(bridge.to_str().unwrap()));

    // Without the helper program the reason says what is missing.
    set_j2534_bridge(None);
    let error = open_j2534(driver, 500000).err().unwrap();
    assert!(error.contains("caruspex-j2534-bridge.exe"), "{error}");

    set_j2534_bridge(Some(bridge));
    let mut link = open_j2534(driver, 500000).unwrap();
    assert!(link.battery_voltage().unwrap() > 12.4);
    assert_eq!(link.name(), format!("J2534 {driver}"));
    let mut elm = MiniElm::new(CanElm::new(link, "J2534 32 bit", 500000));
    elm.connect("6").unwrap();
    assert_eq!(elm.read_vin(), "WAUZZZ8V5LA012345");
    drop(elm);

    // The driver's own refusal comes through the pipe.
    let error = open_j2534(driver, 250000).err().unwrap();
    assert!(error.ends_with("fake driver"), "{error}");

    // The K-line with the ioctls of the 5-baud init.
    let (stream, name) = open_j2534_kline(driver, KLineKind::Iso9141, 9600).expect("opens");
    assert!(name.ends_with("K-Line (ISO 9141)"), "{name}");
    let mut line = VagKLine::new(stream, None);
    line.connect(Ecu::Engine).expect("wakes up");
    assert_eq!(line.identification(), vec!["036906032A  MED9.5.10  "]);
    let codes: Vec<i32> = line.read_faults().unwrap().iter().map(|f| f.code).collect();
    assert_eq!(codes, vec![16486, 561]);
    drop(line);

    // A 32-bit file that is no driver at all: the helper itself.
    let helper = fake_driver::bridge_32();
    let error = open_j2534(helper.to_str().unwrap(), 500000).err().unwrap();
    assert!(error.contains("J2534"), "{error}");
    set_j2534_bridge(None);
}
