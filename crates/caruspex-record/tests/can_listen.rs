// The listen-only connection to a car without OBD answers and the Tesla pack
// read end to end through the ordinary reader, plus broadcast values in an
// imported CSV (tests/test_can_signals.cpp). The DBC signal tests live in
// caruspex-net.

mod common;

use std::collections::BTreeMap;

use caruspex_i18n::Language;
use caruspex_link::Elm327;
use caruspex_protocols::Obd2;
use caruspex_record::{CustomPidReader, USER_KEY_BASE, pack_pids, parse_torque_csv};

use common::{language, near, tesla};

#[test]
fn a_car_without_obd_answers_but_with_bus_traffic_is_connected_for_listening() {
    let _lang = language(Language::English);
    for headers in [false, true] {
        let mut elm = Elm327::new();
        elm.connect(Box::new(tesla()), "6").expect("connects");
        assert!(elm.listen_only());
        assert!(elm.protocol_text().contains("listen only"));
        if headers {
            assert!(elm.set_headers(true));
        }

        let frames = elm.capture_frames(0x332, 300, 10);
        assert!(frames.len() >= 2);
        assert_eq!(frames[0].len(), 6);

        let mut obd = Obd2::new(&mut elm);
        assert_eq!(obd.read_vin(), "5YJ3E1EA7KF123456");

        // The Tesla pack through the ordinary reader.
        let pack = pack_pids("tesla-model3-y");
        assert!(pack.len() >= 10);
        let mut reader = CustomPidReader::new(&mut elm);
        let mut values: BTreeMap<String, f64> = BTreeMap::new();
        for r in reader.read_all(&pack) {
            if !r.ok {
                continue;
            }
            for p in &pack {
                if p.key == r.value.key {
                    values.insert(p.short_name.clone(), r.value.value);
                }
            }
        }
        assert!(near(values["Batt V"], 384.16));
        assert!(near(values["Batt A"], 20.0));
        assert!(near(values["SOC display"], 80.5));
        assert!(near(values["SOC BMS"], 80.7));
        assert!(near(values["BOL kWh"], 82.0));
        assert!(near(values["Full kWh"], 74.6));
        assert!(near(values["Energy kWh"], 60.1));
        assert!(near(values["Cell V max"], 4.1));
        assert!(near(values["Cell V min"], 4.08));
        assert!(near(values["Batt T max"], 25.0));
        assert!(near(values["Batt T min"], 20.0));
        // Not on this bus: no value, an explanation instead.
        assert!(!values.contains_key("Speed"));
    }
}

#[test]
fn broadcast_values_in_an_imported_csv() {
    let _lang = language(Language::German);
    let (pids, errors) = parse_torque_csv(
        "Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header\n\
         Pack voltage,PackV,CAN132,\"DBC:0|16@1+(0.01,0)\",0,500,V,\n\
         Broken,X,CAN132,\"DBC:nonsense\",0,1,V,\n",
        USER_KEY_BASE,
    );
    assert_eq!(pids.len(), 1);
    assert_eq!(pids[0].request, "CAN132");
    assert_eq!(errors.len(), 1);
}
