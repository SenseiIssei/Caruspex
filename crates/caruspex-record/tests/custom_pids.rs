// Custom values: the Torque CSV format, the equation evaluator, reading
// through the adapter (demo car and recorded answers of real cars) and the
// built-in packs (tests/test_custom_pids.cpp).
//
// The recorded answers of the Kona, the Leaf and the Zoe are test cases of
// OBDb (github.com/OBDb, tests/test_cases), taken from real cars; the
// expected numbers are the ones OBDb lists for them. So the packs are checked
// against an independent decoder, not against themselves.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use caruspex_data::pids::{PidCategory, live_key, pid_categories, pid_category_name};
use caruspex_i18n::{Language, set_language};
use caruspex_link::Elm327;
use caruspex_net::can_signals::{parse_can_signal, parse_monitor_request};
use caruspex_protocols::Obd2;
use caruspex_record::{
    CustomPid, CustomPidReader, PACK_KEY_BASE, TorqueEquation, USER_KEY_BASE, built_in_packs,
    is_custom_key, pack_category, pack_names, pack_pids, pack_title, parse_request_hex,
    parse_torque_csv, read_custom, write_torque_csv,
};

use common::{DemoRig, ScriptCar, approx, german, language};

fn eval(equation: &str, data: &[u8]) -> f64 {
    let eq = TorqueEquation::new(equation);
    assert!(eq.ok(), "{equation}: {}", eq.error());
    match eq.evaluate(data) {
        Ok(v) => v,
        Err(e) => panic!("{equation}: {e}"),
    }
}

fn parses(equation: &str) -> bool {
    TorqueEquation::new(equation).ok()
}

// Recorded by OBDb on a 2020 Kona Electric (test_cases/2020).
const KONA_0101: [&str; 9] = [
    "7EC103E620101FFF7E7",
    "7EC21FF490000000080",
    "7EC2200000DD9090908",
    "7EC23080908000007B4",
    "7EC2402B40100007E00",
    "7EC2500DB540000D915",
    "7EC260000525900004F",
    "7EC2757007B02130000",
    "7EC2800000000000BB8",
];
const KONA_0105: [&str; 7] = [
    "7EC102E620105003FFF",
    "7EC2190000000000000",
    "7EC220000077E000041",
    "7EC238C3E7800006407",
    "7EC244B03E032003137",
    "7EC254B0000B4B40000",
    "7EC260900000000AAAA",
];
// A 2013 Leaf (test_cases/2013 and 2015).
const LEAF_2101: [&str; 6] = [
    "7BB1029610100000012",
    "7BB210289FFFFFDE0FF",
    "7BB22FFFFFF0D5A2AF8",
    "7BB23968F381C038B00",
    "7BB24601D46000B9B35",
    "7BB2500081340800001",
];
const LEAF_2161: [&str; 2] = ["7BB100C6161140F1C9A", "7BB21FF1CA41C9A03FF"];
// A 2017 Zoe (test_cases/2017).
const ZOE_3203: [&str; 1] = ["7EC0562320302A2AAAA"];
const ZOE_3204: [&str; 1] = ["7EC056232047DEAAAAA"];

/// Reads a pack in English and returns name -> value of everything read.
fn read_pack(elm: &mut Elm327, pack: &str) -> (BTreeMap<String, f64>, Vec<String>) {
    set_language(Language::English);
    let pids = pack_pids(pack);
    let mut out = BTreeMap::new();
    let mut failures = Vec::new();
    {
        let mut reader = CustomPidReader::new(elm);
        let readings = reader.read_all(&pids);
        assert_eq!(readings.len(), pids.len());
        for (pid, reading) in pids.iter().zip(&readings) {
            if reading.ok {
                assert_eq!(reading.value.key, pid.key);
                assert_eq!(reading.value.name, pid.name);
                out.insert(pid.name.clone(), reading.value.value);
            } else {
                failures.push(format!("{}: {}", pid.name, reading.error));
            }
        }
    }
    set_language(Language::German);
    (out, failures)
}

// ---------------------------------------------------------------------------
// Equations
// ---------------------------------------------------------------------------

#[test]
fn equations_arithmetic_precedence_and_bytes() {
    let _lang = german();
    let d = [0x12, 0x34, 0xFF, 0x80, 0x7F, 0x00, 0x05];
    assert_eq!(eval("2+3*4", &d), 14.0);
    assert_eq!(eval("(2+3)*4", &d), 20.0);
    assert_eq!(eval("10-4-3", &d), 3.0);
    assert!(approx(eval("100/10/4", &d), 2.5));
    assert_eq!(eval("-A+5", &d), -f64::from(0x12) + 5.0);
    assert_eq!(eval("--3", &d), 3.0);
    assert_eq!(eval("2*-3", &d), -6.0);
    assert_eq!(eval("+4", &d), 4.0);
    assert_eq!(eval(" 1.5 * 2 ", &d), 3.0);
    assert_eq!(eval(".25*4", &d), 1.0);
    assert_eq!(eval("A", &d), f64::from(0x12));
    assert_eq!(eval("b", &d), f64::from(0x34)); // case does not matter
    assert_eq!(eval("G", &d), 5.0);
    assert_eq!(eval("(A*256)+B", &d), f64::from(0x1234));
    assert_eq!(
        eval("A*256+B/4", &d),
        f64::from(0x12) * 256.0 + f64::from(0x34) / 4.0
    );
}

#[test]
fn equations_byte_names_beyond_z() {
    let _lang = german();
    assert_eq!(TorqueEquation::byte_index("A"), Some(0));
    assert_eq!(TorqueEquation::byte_index("Z"), Some(25));
    assert_eq!(TorqueEquation::byte_index("AA"), Some(26));
    assert_eq!(TorqueEquation::byte_index("ab"), Some(27));
    assert_eq!(TorqueEquation::byte_index("AZ"), Some(51));
    assert_eq!(TorqueEquation::byte_index("BA"), Some(52));
    assert_eq!(TorqueEquation::byte_index("BV"), Some(73));
    assert_eq!(TorqueEquation::byte_index("ZZ"), Some(701));
    assert_eq!(TorqueEquation::byte_index("ABC"), None);
    assert_eq!(TorqueEquation::byte_index("A1"), None);
    assert_eq!(TorqueEquation::byte_index(""), None);

    let mut d = [0u8; 80];
    d[26] = 7;
    d[52] = 9;
    d[73] = 50;
    assert_eq!(eval("AA", &d), 7.0);
    assert_eq!(eval("ba", &d), 9.0);
    assert_eq!(eval("BV-40", &d), 10.0);
    assert_eq!(TorqueEquation::new("BV-40").bytes_needed(), 74);
    assert_eq!(TorqueEquation::new("A+C").bytes_needed(), 3);
    assert_eq!(TorqueEquation::new("12").bytes_needed(), 0);
}

#[test]
fn equations_shifts_as_the_ev_lists_write_them() {
    let _lang = german();
    let d = [0x01, 0x02, 0x03, 0x04, 0xF0];
    assert_eq!(eval("(A<8)+B", &d), f64::from(0x0102));
    assert!(approx(
        eval("((A<24)+(B<16)+(C<8)+D)/10", &d),
        f64::from(0x0102_0304) / 10.0
    ));
    assert_eq!(eval("E>4", &d), f64::from(0x0F));
    // Shifts bind weaker than + and -, as in C.
    assert_eq!(eval("A<8+B", &d), f64::from(1 << 10));
    assert_eq!(eval("(0-16)>2", &d), -4.0); // arithmetic right shift
    assert_eq!(eval("2.9<1", &d), 4.0); // the fraction is dropped first
    assert!(approx(eval("(E<24)<8", &d), 4026531840.0 * 256.0));
}

#[test]
fn equations_signed_and_the_integer_functions() {
    let _lang = german();
    let d = [0xFF, 0x80, 0x7F, 0x00, 0xFF, 0x38, 0x9C];
    assert_eq!(eval("Signed(A)", &d), -1.0);
    assert_eq!(eval("SIGNED(B)", &d), -128.0);
    assert_eq!(eval("signed(C)", &d), 127.0);
    assert_eq!(eval("SIGNED8(D)", &d), 0.0);
    // The current of the Hyundai/Kia BMS: signed high byte times 256 plus
    // the low byte.
    assert!(approx(eval("((Signed(A)*256)+G)/10", &d), -10.0));
    assert_eq!(eval("INT16(E:F)", &d), f64::from(0xFF38));
    assert_eq!(eval("SIGNED16(INT16(E:F))", &d), -200.0);
    assert_eq!(eval("SIGNED16(INT16(E,F))", &d), -200.0); // comma as separator too
    assert_eq!(eval("INT24(A:B:C)", &d), f64::from(0xFF807F));
    assert_eq!(
        eval("SIGNED24(INT24(A:B:C))", &d),
        f64::from(0xFF807F) - 16777216.0
    );
    assert!(approx(eval("INT32(A:B:C:D)", &d), 4286611200.0));
    assert!(approx(
        eval("SIGNED32(INT32(A:B:C:D))", &d),
        4286611200.0 - 4294967296.0
    ));
    assert_eq!(eval("MIN(A:C)", &d), f64::from(0x7F));
    assert_eq!(eval("MAX(A:C)", &d), f64::from(0xFF));
    assert_eq!(eval("ABS(0-5)", &d), 5.0);
    assert_eq!(eval("INT(7/2)", &d), 3.0);
    assert_eq!(eval("INT(0-7/2)", &d), -3.0);
}

#[test]
fn equations_bit_access() {
    let _lang = german();
    let d = [0x80, 0x01, 0x28];
    assert_eq!(eval("{A:7}", &d), 1.0);
    assert_eq!(eval("{A:6}", &d), 0.0);
    assert_eq!(eval("{b:0}", &d), 1.0);
    assert_eq!(eval("{C:3}+{C:5}", &d), 2.0);
    assert_eq!(eval("BIT(C:5)", &d), 1.0);
    assert_eq!(eval("BIT(C:4)", &d), 0.0);
    assert_eq!(eval("BIT((A<8)+B:15)", &d), 1.0);
}

#[test]
fn equations_errors_instead_of_crashes() {
    let _lang = german();
    for bad in [
        "",
        "   ",
        "A+",
        "(A",
        "A)",
        "A B",
        "2A",
        "foo(A)",
        "ABC",
        "1.2.3",
        "val{000_Battery Current}",
        "{A:40}",
        "{A}",
        "{1:2}",
        "MIN(A)",
        "SIGNED(A:B)",
        "A*/B",
        "#",
        "A=B",
        "INT16(A:B",
    ] {
        let eq = TorqueEquation::new(bad);
        assert!(!eq.ok(), "{bad}");
        assert!(!eq.error().is_empty(), "{bad}");
        let error = eq.evaluate(&[1, 2, 3]).unwrap_err();
        assert!(!error.is_empty(), "{bad}");
    }
    // Nesting deep enough to hurt a naive parser.
    assert!(!parses(&format!(
        "{}A{}",
        "(".repeat(5000),
        ")".repeat(5000)
    )));
    assert!(!parses(&format!("{}A", "-".repeat(5000))));
    assert!(parses(&format!("{}A{}", "(".repeat(20), ")".repeat(20))));

    let error = TorqueEquation::new("A/B").evaluate(&[5, 0]).unwrap_err();
    assert!(error.contains("null")); // German: "Division durch null."

    set_language(Language::English);
    let error = TorqueEquation::new("A/(B-B)")
        .evaluate(&[5, 3])
        .unwrap_err();
    assert_eq!(error, "Division by zero.");
    let error = TorqueEquation::new("((AA<8)+AB)/10")
        .evaluate(&[1, 2, 3])
        .unwrap_err();
    assert!(error.contains("AA"));
    assert!(error.contains("3 data bytes"));
    assert!(
        !TorqueEquation::new("A<70")
            .evaluate(&[1])
            .unwrap_err()
            .is_empty()
    );
    assert!(TorqueEquation::new("BIT(A:0-1)").evaluate(&[1]).is_err());
    assert!(
        TorqueEquation::new("10000000000*10000000000<1")
            .evaluate(&[])
            .is_err()
    );
    assert!(TorqueEquation::new("val{x}").error().contains("val{"));
}

#[test]
fn equations_parse_once_evaluate_many_times() {
    let _lang = german();
    let eq = TorqueEquation::new("((Signed(K)*256)+L)/10");
    let copy = eq.clone(); // copies share the parsed form
    let mut d = [0u8; 12];
    let mut raw = -300;
    while raw <= 300 {
        let u = (raw & 0xFFFF) as u32;
        d[10] = (u >> 8) as u8;
        d[11] = (u & 0xFF) as u8;
        let value = copy.evaluate(&d).unwrap();
        assert!(approx(value, f64::from(raw) / 10.0));
        raw += 7;
    }
    assert_eq!(eq.text(), "((Signed(K)*256)+L)/10");
}

// ---------------------------------------------------------------------------
// Torque CSV
// ---------------------------------------------------------------------------

#[test]
fn torque_csv_header_quotes_comments() {
    let _lang = german();
    let text = "\u{FEFF}# My Kona values\r\n\
        Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header\r\n\
        \r\n\
        \x20 # an indented comment\r\n\
        000_State of Charge BMS,SOC BMS,0x220101,e/2,0,100,%,7E4\r\n\
        \"Current, signed\",\"Batt \"\"A\"\"\",220101,((Signed(K)*256)+L)/10,-230,230,A,7e4\r\n\
        \"Two\nlines\",2L,2101,A,0,1,,\r\n\
        Odometer,Odo,22b002,INT24(g:h:i),0,16777215,km,7C6\r\n\
        Engine speed,RPM,010C,((A*256)+B)/4,0,8000,rpm,\r\n";
    let (pids, errors) = parse_torque_csv(text, USER_KEY_BASE);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(pids.len(), 5);

    assert_eq!(pids[0].name, "000_State of Charge BMS");
    assert_eq!(pids[0].short_name, "SOC BMS");
    assert_eq!(pids[0].request, "220101");
    assert_eq!(pids[0].equation, "e/2");
    assert_eq!(pids[0].max_value, 100.0);
    assert_eq!(pids[0].unit, "%");
    assert_eq!(pids[0].header, "7E4");
    assert!(pids[0].response.is_empty());
    assert_eq!(pids[0].key, USER_KEY_BASE);
    assert_eq!(pids[0].category, PidCategory::Custom);
    assert!(pids[0].pack.is_empty());

    assert_eq!(pids[1].name, "Current, signed");
    assert_eq!(pids[1].short_name, "Batt \"A\"");
    assert_eq!(pids[1].min_value, -230.0);
    assert_eq!(pids[1].header, "7E4");
    assert_eq!(pids[1].key, USER_KEY_BASE + 1);

    assert_eq!(pids[2].name, "Two\nlines");
    assert!(pids[2].header.is_empty());
    assert_eq!(pids[3].request, "22B002");
    assert_eq!(pids[3].header, "7C6");
    assert_eq!(pids[4].request, "010C");
    assert!(is_custom_key(pids[4].key));
    assert!(!is_custom_key(live_key(0x0C, 3)));
}

#[test]
fn torque_csv_without_header_by_column_name_extra_columns() {
    let _lang = german();
    // Torque's order without a header line, Header column left out.
    let (pids, errors) = parse_torque_csv(
        "Oil,Oil,221001,A,0,100,%\nTemp,T,221002,A-40,-40,215,C,7E0\n",
        0x30_0000,
    );
    assert!(errors.is_empty());
    assert_eq!(pids.len(), 2);
    assert!(pids[0].header.is_empty());
    assert_eq!(pids[1].header, "7E0");
    assert_eq!(pids[1].key, 0x30_0001);

    // Columns in another order, an unknown column, our Response column.
    let (pids, errors) = parse_torque_csv(
        "Equation,Header,Name,Comment,ModeAndPID,Response,Units\n\
         H/2,79B,SOH LBC,ignored,2161,7BB,%\n\
         A,79B:7BB,Pair,,2101,,\n\
         A,79b/7bb,Pair 2,,2101,,\n\
         A,DA44F1,Wide,,22F190,,\n",
        USER_KEY_BASE,
    );
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(pids.len(), 4);
    assert_eq!(pids[0].name, "SOH LBC");
    assert_eq!(pids[0].short_name, "SOH LBC"); // falls back to the name
    assert_eq!(pids[0].request, "2161");
    assert_eq!(pids[0].header, "79B");
    assert_eq!(pids[0].response, "7BB");
    assert_eq!(pids[0].min_value, 0.0);
    assert_eq!(pids[1].header, "79B");
    assert_eq!(pids[1].response, "7BB");
    assert_eq!(pids[2].response, "7BB");
    assert_eq!(pids[3].header, "18DA44F1");
}

#[test]
fn torque_csv_bad_lines_are_skipped_and_named() {
    let text = "Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header\n\
        Good,G,220101,A,0,1,,7E4\n\
        ,NoName,220101,A,0,1,,7E4\n\
        Bad request,B,22ZZ,A,0,1,,7E4\n\
        Odd request,B,22010,A,0,1,,7E4\n\
        Bad equation,B,220101,A+*,0,1,,7E4\n\
        Bad min,B,220101,A,low,1,,7E4\n\
        Bad header,B,220101,A,0,1,,7E4X\n\
        ~Modifications by someone,List Credit,,,,,,\n\
        !_ABRP_Battery,B,,val{000_Battery Current},-230,230,A,\n\
        Reference,R,220101,val{000_Battery Current},0,1,,7E4\n\
        Only service,S,22,A,0,1,,7E4\n\
        Too long,L,2201010101010101,A,0,1,,7E4\n\
        Header too big,H,220101,A,0,1,,FFF\n\
        Good too,G2,2101,B,0,1,,\n";
    let _lang = language(Language::English);
    let (pids, errors) = parse_torque_csv(text, USER_KEY_BASE);
    assert_eq!(pids.len(), 2);
    assert_eq!(pids[0].name, "Good");
    assert_eq!(pids[1].name, "Good too");
    assert_eq!(pids[1].key, USER_KEY_BASE + 1); // keys stay dense
    assert_eq!(errors.len(), 12);
    for line in 3..=14 {
        let prefix = format!("Line {line}: ");
        assert!(
            errors.iter().any(|e| e.starts_with(&prefix)),
            "{prefix} in {errors:?}"
        );
    }

    // A header line without the essential columns.
    let (pids, errors) = parse_torque_csv("Name,ModeAndPID,Units\nA,2101,%\n", USER_KEY_BASE);
    assert!(pids.is_empty());
    assert_eq!(errors.len(), 1);

    // Nothing at all.
    let (pids, errors) = parse_torque_csv("", USER_KEY_BASE);
    assert!(pids.is_empty() && errors.is_empty());
    let (pids, errors) = parse_torque_csv("# only a comment\n\n", USER_KEY_BASE);
    assert!(pids.is_empty() && errors.is_empty());
}

#[test]
fn torque_csv_writing_and_reading_back() {
    let _lang = german();
    let mut pids = vec![
        CustomPid::default(),
        CustomPid::default(),
        CustomPid::default(),
    ];
    pids[0].name = "Plain".into();
    pids[0].short_name = "P".into();
    pids[0].request = "220101".into();
    pids[0].equation = "E/2".into();
    pids[0].min_value = 0.0;
    pids[0].max_value = 100.0;
    pids[0].unit = "%".into();
    pids[0].header = "7E4".into();
    pids[1].name = "With, comma and \"quotes\"".into();
    pids[1].short_name = " padded ".into();
    pids[1].request = "2101".into();
    pids[1].equation = "MIN(A:B)".into();
    pids[1].min_value = -0.25;
    pids[1].max_value = 1234.5;
    pids[1].unit = "\u{b0}C".into();
    pids[2].name = "#not a comment".into();
    pids[2].short_name = "N".into();
    pids[2].request = "2161".into();
    pids[2].equation = "H/2".into();
    pids[2].header = "79B".into();

    let mut text = write_torque_csv(&pids);
    assert!(
        text.starts_with("Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header\n")
    );
    let (back, errors) = parse_torque_csv(&text, USER_KEY_BASE);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(back.len(), 3);
    for (a, b) in back.iter().zip(&pids) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.request, b.request);
        assert_eq!(a.equation, b.equation);
        assert_eq!(a.min_value, b.min_value);
        assert_eq!(a.max_value, b.max_value);
        assert_eq!(a.unit, b.unit);
        assert_eq!(a.header, b.header);
    }
    assert_eq!(back[1].short_name, "padded"); // cells are trimmed

    // The Response column appears only when needed.
    pids[2].response = "7BB".into();
    text = write_torque_csv(&pids);
    assert!(text.starts_with(
        "Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header,Response\n"
    ));
    let (back, _) = parse_torque_csv(&text, USER_KEY_BASE);
    assert_eq!(back.len(), 3);
    assert_eq!(back[2].response, "7BB");
    assert!(back[0].response.is_empty());

    // A pack survives the round trip with every value.
    let all = built_in_packs();
    let (back, errors) = parse_torque_csv(&write_torque_csv(all), USER_KEY_BASE);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(back.len(), all.len());
}

#[test]
fn request_hex() {
    assert_eq!(parse_request_hex("220101"), Some(vec![0x22, 0x01, 0x01]));
    assert_eq!(parse_request_hex("0x2101"), Some(vec![0x21, 0x01]));
    assert_eq!(parse_request_hex("22 f1 90").map(|b| b.len()), Some(3));
    assert_eq!(parse_request_hex(""), None);
    assert_eq!(parse_request_hex("221"), None);
    assert_eq!(parse_request_hex("22G101"), None);
    assert_eq!(parse_request_hex("2201010101010101"), None); // eight bytes
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

#[test]
fn reading_the_demo_pack_from_the_demo_car() {
    let _lang = german();
    for headers in [false, true] {
        let mut rig = DemoRig::new(600.0);
        assert!(rig.connected);
        if headers {
            assert!(rig.elm.set_headers(true));
        }
        rig.clear_sent();

        let (v, failures) = read_pack(&mut rig.elm, "caruspex-demo");
        assert!(failures.is_empty(), "{failures:?}");
        assert_eq!(v.len(), pack_pids("caruspex-demo").len());

        let oil = v["Oil level"];
        assert!((60.0..=80.0).contains(&oil));
        assert!(v["Oil temperature"] > 30.0);
        assert!(v["Oil temperature"] < 110.0);
        assert_eq!(v["Engine starts"], 1835.0);
        assert_eq!(v["Engine running (1 = yes)"], 1.0);
        // 2 456 000 s before the drive plus the run time: just over 682 h.
        assert!(v["Engine hours"] > 682.2);
        assert!(v["Engine hours"] < 682.5);
        assert!(v["Supply voltage (engine control unit)"] > 13.5);
        assert!(v["Supply voltage (engine control unit)"] < 14.5);
        assert!(v["Engine speed (OBD)"] > 600.0);
        // The odometer of the cluster (whole km) and the ABS (steps of 10 km).
        let cluster = v["Odometer (instrument cluster)"];
        assert!((48_213.0..48_300.0).contains(&cluster), "{cluster}");
        assert_eq!(cluster.fract(), 0.0);
        let abs = v["Odometer (ABS)"];
        assert_eq!(abs % 10.0, 0.0);
        assert!(cluster - abs >= 0.0 && cluster - abs < 10.0, "{abs}");

        // The engine was addressed once for all its values (one request for
        // the two values of 1010 too), and the default addressing is back.
        // Elm327 logs one line per command: "  > COMMAND   ->   answer".
        assert_eq!(rig.count("ATSH7E0"), 1);
        assert_eq!(rig.count("221010"), 1);
        assert!(rig.count("ATSH7DF") >= 1);

        let mut obd = Obd2::new(&mut rig.elm);
        assert!(obd.read_live(0x0D).is_some());
    }
}

#[test]
fn reading_single_values_and_failures() {
    let _lang = language(Language::English);
    let mut rig = DemoRig::new(600.0);
    assert!(rig.connected);

    let oil = CustomPid {
        name: "Oil".into(),
        request: "221002".into(),
        header: "7E0".into(),
        equation: "SIGNED16(INT16(A:B))/10".into(),
        key: USER_KEY_BASE + 5,
        min_value: -20.0,
        max_value: 150.0,
        unit: "\u{b0}C".into(),
        ..CustomPid::default()
    };
    let value = read_custom(&mut rig.elm, &oil).expect("oil temperature reads");
    assert_eq!(value.key, USER_KEY_BASE + 5);
    assert_eq!(value.category, PidCategory::Custom);
    assert_eq!(value.display_max, 150.0);
    assert_eq!(value.unit, "\u{b0}C");
    assert_eq!(value.ecu_source, "7E8");
    assert!(!value.ecu.is_empty());

    // Without a header: functional OBD addressing.
    let rpm = CustomPid {
        name: "RPM".into(),
        request: "010C".into(),
        equation: "((A*256)+B)/4".into(),
        ..CustomPid::default()
    };
    let value = read_custom(&mut rig.elm, &rpm).expect("rpm reads");
    assert!(value.value > 600.0);
    assert!(value.ecu_source.is_empty());

    {
        let mut reader = CustomPidReader::new(&mut rig.elm);
        // An identifier the engine does not know: the refusal comes through.
        let unknown = CustomPid {
            request: "22FFFF".into(),
            ..oil.clone()
        };
        let error = reader.read(&unknown).unwrap_err();
        assert!(error.contains("0x31"), "{error}");

        // A control unit that is not there.
        let nobody = CustomPid {
            header: "7A0".into(),
            ..oil.clone()
        };
        assert!(!reader.read(&nobody).unwrap_err().is_empty());

        // The answer is shorter than the equation needs.
        let short_answer = CustomPid {
            equation: "Z".into(),
            ..oil.clone()
        };
        let error = reader.read(&short_answer).unwrap_err();
        assert!(error.contains("byte Z"), "{error}");

        // A broken equation is reported without asking the car.
        let broken = CustomPid {
            equation: "A+".into(),
            ..oil.clone()
        };
        rig.sent.lock().unwrap().clear();
        assert!(!reader.read(&broken).unwrap_err().is_empty());
        assert!(rig.sent.lock().unwrap().is_empty());

        // A 29-bit header on an 11-bit bus is refused by the adapter check.
        let wide = CustomPid {
            header: "18DA10F1".into(),
            ..oil.clone()
        };
        assert!(!reader.read(&wide).unwrap_err().is_empty());

        // And afterwards the oil temperature still reads.
        assert!(reader.read(&oil).is_ok());
        reader.restore_default();
    }
    set_language(Language::German);

    let mut obd = Obd2::new(&mut rig.elm);
    assert!(obd.read_live(0x0C).is_some());
}

#[test]
fn kona_electric_recorded_multi_frame_answers_decode_like_obdb() {
    let _lang = german();
    for headers in [false, true] {
        let mut car = ScriptCar::new();
        assert!(car.connected);
        if headers {
            assert!(car.elm.set_headers(true));
        }
        car.set_frames("7E4|220101", &KONA_0101);
        car.set_frames("7E4|220105", &KONA_0105);
        car.clear_sent();

        let (v, failures) = read_pack(&mut car.elm, "hyundai-kona-niro-ev");
        assert!(failures.is_empty(), "{failures:?}");

        let a = |name: &str, expected: f64| {
            assert!(
                approx(v[name], expected),
                "{name}: {} vs {expected}",
                v[name]
            );
        };
        a("State of charge (BMS)", 36.5);
        a("State of charge (display)", 37.5);
        a("State of health", 99.2);
        a("Minimum cell deterioration", 4.9);
        a("Battery current (positive = discharge)", 0.0);
        a("Battery voltage", 354.5);
        a("Battery power (positive = discharge)", 0.0);
        a("Available charge power", 0.0);
        a("Maximum cell voltage", 3.6);
        assert_eq!(v["Cell with the highest voltage"], 2.0);
        a("Minimum cell voltage", 3.6);
        assert_eq!(v["Cell with the lowest voltage"], 1.0);
        a("Cell voltage spread", 0.0);
        assert_eq!(v["Battery temperature maximum"], 9.0);
        assert_eq!(v["Battery temperature minimum"], 9.0);
        assert_eq!(v["Battery module 1 temperature"], 8.0);
        assert_eq!(v["Battery module 3 temperature"], 9.0);
        assert_eq!(v["Battery inlet temperature"], 7.0);
        assert_eq!(v["Battery heater temperature"], 7.0);
        a("12 V battery voltage", 12.6);
        assert_eq!(v["Charging (1 = yes)"], 1.0);
        assert_eq!(v["Fast charging plug connected (1 = yes)"], 0.0);
        assert_eq!(v["Main relay closed (1 = yes)"], 0.0);
        a("Total charge (current)", 5614.8);
        a("Total discharge (current)", 5557.3);
        a("Total energy charged", 2108.1);
        a("Total energy discharged", 2031.1);
        a("Operating time", 8061459.0 / 3600.0);
        assert_eq!(v["Isolation resistance"], 3000.0);

        // 33 values, two requests, one addressing of the BMS.
        assert_eq!(car.count("220101"), 1);
        assert_eq!(car.count("220105"), 1);
        assert_eq!(car.count("ATSH7E4"), 1);
        assert_eq!(car.count("ATCRA7EC"), 1);
        assert_eq!(car.count("ATSH7DF"), 1);
    }
}

#[test]
fn kona_electric_negative_current_and_power() {
    let _lang = german();
    // The same answer with 0xFF9C (-10.0 A) in K and L: charging. The first
    // frame holds 62 01 01 and A to C, the next ones seven bytes each, so K
    // and L open the third frame.
    let mut frames: Vec<String> = KONA_0101.iter().map(|s| s.to_string()).collect();
    frames[2].replace_range(5..9, "FF9C");
    let car = ScriptCar::new();
    assert!(car.connected);
    car.set_frames("7E4|220101", &frames);
    car.set_frames("7E4|220105", &KONA_0105);
    let mut car = car;
    let (v, _) = read_pack(&mut car.elm, "hyundai-kona-niro-ev");
    assert!(approx(v["Battery current (positive = discharge)"], -10.0));
    assert!(approx(
        v["Battery power (positive = discharge)"],
        -10.0 * 354.5 / 1000.0
    ));
}

#[test]
fn nissan_leaf_and_renault_zoe_recorded_answers_decode_like_obdb() {
    let _lang = german();
    {
        let mut car = ScriptCar::new();
        assert!(car.connected);
        car.set_frames("79B|2101", &LEAF_2101);
        car.set_frames("79B|2161", &LEAF_2161);
        car.clear_sent();
        let (v, _) = read_pack(&mut car.elm, "nissan-leaf-ze0-aze0");
        assert!(approx(v["State of charge (battery controller)"], 76.0629));
        assert!(approx(v["Battery capacity"], 52.9216));
        assert!(approx(v["Battery voltage"], 385.43));
        assert!(approx(v["State of health"], 73.22));
        // The battery controller answers on 7BB, not 79B + 8.
        assert_eq!(car.count("ATSH79B"), 1);
        assert_eq!(car.count("ATCRA7BB"), 1);
        assert_eq!(car.count("ATFCSH79B"), 1);
        // Group 04 was not recorded: NO DATA, the temperatures are missing.
        assert!(!v.contains_key("Battery temperature sensor 1"));
    }
    {
        let mut car = ScriptCar::new();
        assert!(car.connected);
        car.set_frames("7E4|223203", &ZOE_3203);
        car.set_frames("7E4|223204", &ZOE_3204);
        let (v, _) = read_pack(&mut car.elm, "renault-zoe-ph1");
        assert!(approx(v["Battery voltage"], 337.0));
        assert!(approx(v["Battery current"], -133.5));
    }
}

// ---------------------------------------------------------------------------
// Packs
// ---------------------------------------------------------------------------

#[test]
fn built_in_packs_load_and_every_equation_parses() {
    let _lang = german();
    let names = pack_names();
    assert!(names.len() >= 5);
    assert_eq!(names[0], "caruspex-demo");
    assert_eq!(names.iter().collect::<BTreeSet<_>>().len(), names.len());
    for id in [
        "hyundai-kona-niro-ev",
        "hyundai-ioniq-28",
        "renault-zoe-ph1",
        "nissan-leaf-ze0-aze0",
    ] {
        assert!(names.contains(&id));
        assert_eq!(pack_category(id), PidCategory::HybridEv);
    }
    assert_eq!(pack_category("caruspex-demo"), PidCategory::Custom);
    assert!(pack_title("nope").is_empty());
    assert!(pack_pids("nope").is_empty());

    for lang in [Language::German, Language::English] {
        set_language(lang);
        let all = built_in_packs();
        assert!(all.len() >= 90);
        let mut keys = BTreeSet::new();
        let mut counted = 0;
        for n in &names {
            assert!(!pack_title(n).is_empty());
            let pack = pack_pids(n);
            assert!(!pack.is_empty());
            counted += pack.len();
            let mut seen = BTreeSet::new();
            for p in &pack {
                assert!(seen.insert(p.name.clone()), "{n}: {} twice", p.name);
            }
        }
        assert_eq!(counted, all.len());
        for p in all {
            let what = format!("{}: {} = {}", p.pack, p.name, p.equation);
            if parse_monitor_request(&p.request).is_some() {
                // Listened for: a DBC signal, no addressing.
                assert!(parse_can_signal(&p.equation).is_ok(), "{what}");
                assert!(p.header.is_empty(), "{what}");
                assert!(p.response.is_empty(), "{what}");
            } else {
                let eq = TorqueEquation::new(&p.equation);
                assert!(eq.ok(), "{what}");
                assert!(eq.bytes_needed() <= 80, "{what}");
                let request = parse_request_hex(&p.request).expect(&what);
                assert!(request.len() >= 2, "{what}");
                assert!(eq.evaluate(&[0u8; 80]).is_ok(), "{what}");
            }
            assert!(keys.insert(p.key), "{what}");
            assert!(is_custom_key(p.key));
            assert!(p.key >= PACK_KEY_BASE);
            assert!(p.key < USER_KEY_BASE);
            assert!(!p.name.is_empty(), "{what}");
            assert!(!p.short_name.is_empty(), "{what}");
            assert!(!p.source.is_empty(), "{what}");
            assert!(p.min_value < p.max_value, "{what}");
            assert_eq!(p.category, pack_category(&p.pack), "{what}");
            for text in [&p.name, &p.short_name, &p.unit, &p.source] {
                assert!(!text.contains('\u{2014}'), "{what}"); // em dash
                assert!(!text.contains('\u{2013}'), "{what}"); // en dash
            }
        }
    }
    set_language(Language::German);
    assert_eq!(pack_pids("hyundai-kona-niro-ev")[0].name, "Ladestand (BMS)");
    set_language(Language::English);
    assert_eq!(
        pack_pids("hyundai-kona-niro-ev")[0].name,
        "State of charge (BMS)"
    );
}

#[test]
fn the_custom_category_has_a_name_and_a_place() {
    let _lang = language(Language::English);
    assert!(pid_categories().contains(&PidCategory::Custom));
    assert_eq!(
        pid_category_name(PidCategory::Custom),
        "Manufacturer and custom values"
    );
    set_language(Language::German);
    assert_eq!(
        pid_category_name(PidCategory::Custom),
        "Hersteller und eigene Werte"
    );
}
