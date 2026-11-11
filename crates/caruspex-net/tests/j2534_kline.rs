//! The K-line of a J2534 PassThru device end to end through a real driver
//! DLL (fake-j2534/, built here): connecting ISO 9141 and ISO 14230 without
//! the device's checksum, the 5-baud init through FIVE_BAUD_INIT with the
//! key bytes handed on and the reply swallowed, the fast init through
//! FAST_INIT, and the VAG K-line protocols on top (KWP2000: identification,
//! fault memory read and clear, measuring block). Windows only, like J2534.
#![cfg(windows)]

mod fake_driver;

use caruspex_link::ByteStream;
use caruspex_net::can_bus::{KLineKind, open_j2534_kline};
use caruspex_protocols::kwp1281::Ecu;
use caruspex_protocols::kwp2000::KLineProtocol;
use caruspex_protocols::vag_kline::VagKLine;
use fake_driver::fake_driver;

#[test]
fn j2534_kline_carries_kwp2000_after_the_five_baud_init() {
    let driver = fake_driver();
    let driver = driver.to_str().unwrap();

    let (stream, name) = open_j2534_kline(driver, KLineKind::Iso9141, 9600).expect("opens");
    assert_eq!(name, format!("J2534 {driver} K-Line (ISO 9141)"));
    assert!(!stream.echoes_own_bytes());

    let mut line = VagKLine::new(stream, None);
    line.connect(Ecu::Engine)
        .expect("wakes and starts the session");
    assert_eq!(line.protocol(), KLineProtocol::Kwp2000);
    assert_eq!(line.identification(), vec!["036906032A  MED9.5.10  "]);

    let faults = line.read_faults().expect("faults");
    let codes: Vec<i32> = faults.iter().map(|f| f.code).collect();
    assert_eq!(codes, vec![16486, 561]);
    assert!(!faults[0].sporadic);
    assert!(faults[1].sporadic);
    line.clear_faults().expect("cleared");
    assert!(line.read_faults().unwrap().is_empty());

    let values = line.read_group(1).expect("values");
    assert_eq!(values.len(), 2);
    assert!((values[0].value - 800.0).abs() < 0.01);
    line.keep_alive().expect("tester present");

    // A unit that is not there: the device reports the failed init.
    let error = line.connect(Ecu::Abs).unwrap_err();
    assert!(error.contains("meldet sich nicht"), "{error}");
    assert!(!line.connected());
    line.connect(Ecu::Engine).expect("the engine again");
    drop(line);

    // ISO 14230 with the fast init: StartCommunication in, the answer out.
    let (mut stream, name) = open_j2534_kline(driver, KLineKind::Iso14230, 10400).unwrap();
    assert!(name.ends_with("(ISO 14230)"), "{name}");
    let answer = stream
        .fast_init(&[0x81, 0x01, 0xF1, 0x81, 0xF4])
        .expect("the unit answers");
    assert_eq!(answer, vec![0x83, 0xF1, 0x01, 0xC1, 0xEF, 0x8F, 0xB4]);
    assert_eq!(stream.fast_init(&[0x81, 0x22, 0xF1, 0x81, 0x15]), None);
    drop(stream);

    // Rates and devices the driver does not take are refused clearly.
    let error = open_j2534_kline(driver, KLineKind::Iso9141, 4800)
        .err()
        .unwrap();
    assert!(error.ends_with("fake driver"), "{error}");
    assert!(open_j2534_kline("C:/no/such/driver.dll", KLineKind::Iso9141, 9600).is_err());
}
