//! The built-in demo vehicle, talked to with raw ELM327 commands.
//!
//! The C++ tests drive the real Elm327 driver and the OBD services on top;
//! here the same facts are checked on the wire: request text in, answer text
//! out. See the report for the originals to re-add once those crates exist.

use caruspex_demo::{DemoAdapter, DemoClock, DemoOptions, DemoTransport};
use caruspex_link::Transport;

fn approx(actual: f64, expected: f64, margin: f64) -> bool {
    (actual - expected).abs() <= margin
}

/// A demo car with a hand-moved clock and no answer delay.
struct Rig {
    clock: DemoClock,
    demo: DemoTransport,
}

impl Rig {
    fn with(start: f64, adapter: DemoAdapter, gearbox: bool) -> Rig {
        let clock = DemoClock::new(start);
        let mut options = DemoOptions::with_clock(&clock);
        options.adapter = adapter;
        options.transmission_ecu = gearbox;
        let mut demo = DemoTransport::new(options);
        demo.flush_input(); // power-on greeting
        Rig { clock, demo }
    }

    fn new(start: f64) -> Rig {
        Rig::with(start, DemoAdapter::CloneV15, true)
    }

    /// Like a connected driver: no echo, no linefeeds, protocol 6.
    fn connected(start: f64) -> Rig {
        let mut rig = Rig::new(start);
        rig.setup();
        rig
    }

    fn setup(&mut self) {
        assert_eq!(self.ask("ATE0"), "ATE0\rOK\r\r>");
        assert_eq!(self.ask("ATL0"), "OK\r\r>");
        assert_eq!(self.ask("ATSP6"), "OK\r\r>");
    }

    fn ask(&mut self, command: &str) -> String {
        self.demo.write(format!("{command}\r").as_bytes());
        let mut out = Vec::new();
        for _ in 0..20 {
            if out.contains(&b'>') {
                break;
            }
            out.extend(self.demo.read(50));
        }
        String::from_utf8(out).unwrap()
    }

    /// The messages of one answer (spaces on, headers off): one per control
    /// unit, multi-frame answers reassembled from their numbered lines.
    fn messages(&mut self, command: &str) -> Vec<Vec<u8>> {
        let text = self.ask(command);
        parse_messages(&text)
    }

    fn live(&mut self, pid: u8) -> Vec<u8> {
        let messages = self.messages(&format!("01{pid:02X}"));
        let first = messages
            .first()
            .unwrap_or_else(|| panic!("no answer to PID {pid:02X}"));
        assert_eq!(first[0], 0x41, "PID {pid:02X}");
        assert_eq!(first[1], pid);
        first[2..].to_vec()
    }

    fn rpm(&mut self) -> f64 {
        let d = self.live(0x0C);
        (d[0] as f64 * 256.0 + d[1] as f64) / 4.0
    }

    fn speed(&mut self) -> f64 {
        self.live(0x0D)[0] as f64
    }

    fn coolant(&mut self) -> f64 {
        self.live(0x05)[0] as f64 - 40.0
    }

    fn volts(&mut self) -> f64 {
        let d = self.live(0x42);
        (d[0] as f64 * 256.0 + d[1] as f64) / 1000.0
    }

    fn u16_pid(&mut self, pid: u8) -> f64 {
        let d = self.live(pid);
        d[0] as f64 * 256.0 + d[1] as f64
    }
}

fn bytes_of(text: &str) -> Vec<u8> {
    text.split_whitespace()
        .map(|t| u8::from_str_radix(t, 16).unwrap())
        .collect()
}

/// Reads lines like "41 00 BE 3F A8 13", or "014" followed by "0: 49 02 01 ..."
/// for multi-frame answers. Stops at the prompt.
fn parse_messages(text: &str) -> Vec<Vec<u8>> {
    let mut messages: Vec<Vec<u8>> = Vec::new();
    let mut expected: Vec<usize> = Vec::new();
    for line in text.split('\r') {
        let line = line.trim().trim_end_matches('>').trim();
        if line.is_empty() || line == "SEARCHING..." || line == "NO DATA" {
            continue;
        }
        if line.len() == 3 && !line.contains(' ') && !line.contains(':') {
            messages.push(Vec::new());
            expected.push(usize::from_str_radix(line, 16).unwrap());
            continue;
        }
        if let Some((_, data)) = line.split_once(':') {
            messages.last_mut().unwrap().extend(bytes_of(data));
            continue;
        }
        messages.push(bytes_of(line));
        expected.push(0);
    }
    for (message, length) in messages.iter_mut().zip(expected) {
        if length > 0 {
            message.truncate(length);
        }
    }
    messages
}

/// The North American check digit, computed independently of the demo.
fn vin_check_digit(vin: &str) -> char {
    let value = |c: char| -> i32 {
        if c.is_ascii_digit() {
            return c as i32 - '0' as i32;
        }
        let letters = "ABCDEFGHJKLMNPRSTUVWXYZ";
        let values = [
            1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 7, 9, 2, 3, 4, 5, 6, 7, 8, 9,
        ];
        values[letters.find(c).unwrap()]
    };
    let weights = [8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2];
    let sum: i32 = vin.chars().zip(weights).map(|(c, w)| value(c) * w).sum();
    let rest = sum % 11;
    if rest == 10 {
        'X'
    } else {
        char::from(b'0' + rest as u8)
    }
}

/// Decodes the DTC bytes of a mode 03/07/0A answer into "P0171" style codes.
fn dtc_list(message: &[u8]) -> Vec<String> {
    let count = message[1] as usize;
    let mut codes = Vec::new();
    for i in 0..count {
        let a = message[2 + 2 * i];
        let b = message[3 + 2 * i];
        let letter = ['P', 'C', 'B', 'U'][(a >> 6) as usize];
        codes.push(format!(
            "{letter}{:X}{:X}{:X}{:X}",
            (a >> 4) & 3,
            a & 0xF,
            b >> 4,
            b & 0xF
        ));
    }
    codes
}

/// Codes from the engine (7E8) only: the first message of the answer.
fn codes(rig: &mut Rig, command: &str) -> Vec<String> {
    let messages = rig.messages(command);
    dtc_list(&messages[0])
}

fn not_ready(readiness: &[u8]) -> u32 {
    // Byte C: supported monitors, byte D: incomplete ones.
    (readiness[2] & readiness[3]).count_ones()
}

#[test]
fn the_demo_adapter_talks_like_an_elm327() {
    let mut rig = Rig::new(30.0);

    assert_eq!(rig.ask("ATZ"), "ATZ\r\r\rELM327 v1.5\r\r>");
    assert_eq!(rig.ask("ATI"), "ATI\rELM327 v1.5\r\r>");
    assert_eq!(rig.ask("ATE0"), "ATE0\rOK\r\r>"); // still echoed
    assert_eq!(rig.ask("ATI"), "ELM327 v1.5\r\r>");
    assert_eq!(rig.ask("AT@1"), "OBDII to RS232 Interpreter\r\r>");
    assert_eq!(rig.ask("ATXYZ"), "?\r\r>");
    assert_eq!(rig.ask("HELLO"), "?\r\r>");
    assert_eq!(rig.ask("ATDP"), "AUTO\r\r>");

    // Automatic search on the first request, then the protocol stays.
    let first = rig.ask("0100");
    assert!(first.starts_with("SEARCHING...\r41 00 "));
    assert!(first[15..].contains("\r41 00 ")); // the gearbox answers too
    assert_eq!(rig.ask("ATDPN"), "A6\r\r>");
    assert_eq!(rig.ask("ATDP"), "AUTO, ISO 15765-4 (CAN 11/500)\r\r>");
    assert!(rig.ask("0100").starts_with("41 00 "));
    assert_eq!(rig.ask("0108"), "NO DATA\r\r>"); // not supported by anyone
    assert_eq!(rig.ask(""), "NO DATA\r\r>"); // a bare carriage return repeats 0108
    let volts = rig.ask("ATRV"); // engine running: charging voltage
    assert!(volts.starts_with("14."));
    assert_eq!(volts.find("V\r\r>"), Some(4));

    // Spaces off, headers on: CAN identifier, PCI byte and padding.
    assert_eq!(rig.ask("ATS0"), "OK\r\r>");
    assert_eq!(rig.ask("ATH1"), "OK\r\r>");
    let headed = rig.ask("0100");
    assert!(headed.starts_with("7E806410"));
    assert!(headed.contains("\r7E906410"));
    assert_eq!(rig.ask("ATD1"), "OK\r\r>");
    assert!(rig.ask("0100").starts_with("7E8806410"));
    assert_eq!(rig.ask("ATCRA7E9"), "OK\r\r>");
    assert!(rig.ask("0100").starts_with("7E9806410"));
    assert_eq!(rig.ask("ATAR"), "OK\r\r>");

    // A physical request reaches only one control unit, and only it may
    // refuse a service.
    assert_eq!(rig.ask("ATD0"), "OK\r\r>");
    assert_eq!(rig.ask("ATSH7E1"), "OK\r\r>");
    assert_eq!(rig.ask("0902"), "NO DATA\r\r>"); // the gearbox has no VIN
    // The gearbox speaks UDS 22 but keeps no VIN in F190: out of range.
    assert!(rig.ask("22F190").starts_with("7E9037F2231"));
    assert!(rig.ask("2101").starts_with("7E9037F2111"));
    assert_eq!(rig.ask("ATSH7DF"), "OK\r\r>");

    // A wrong fixed protocol fails like on a real car.
    assert_eq!(rig.ask("ATSP5"), "OK\r\r>");
    assert_eq!(rig.ask("0100"), "BUS INIT: ERROR\r\r>");
    assert_eq!(rig.ask("ATSP8"), "OK\r\r>");
    assert_eq!(rig.ask("0100"), "CAN ERROR\r\r>");
    assert_eq!(rig.ask("ATSP6"), "OK\r\r>");
    assert_eq!(rig.ask("ATDPN"), "6\r\r>");
    assert!(rig.ask("0100").starts_with("7E806410"));

    // Monitoring a silent bus shows nothing until a character stops it.
    rig.demo.write(b"ATMA\r");
    assert!(rig.demo.read(20).is_empty());
    assert_eq!(rig.ask("X"), "STOPPED\r\r>");
}

#[test]
fn the_name_and_the_vin_are_the_demo_ones() {
    let rig = Rig::new(0.0);
    assert!(
        ["Demo-Fahrzeug (eingebaut)", "Demo vehicle (built-in)"]
            .contains(&rig.demo.name().as_str())
    );
    assert_eq!(DemoTransport::vin(), "YCXDE15C7MC048217");
}

#[test]
fn other_adapters_report_their_own_identity() {
    let mut rig = Rig::with(10.0, DemoAdapter::GenuineV14b, true);
    assert_eq!(rig.ask("ATI"), "ATI\rELM327 v1.4b\r\r>");
    assert_eq!(rig.ask("STI"), "STI\r?\r\r>");

    let mut stn = Rig::with(10.0, DemoAdapter::ObdLinkStn, true);
    assert_eq!(stn.ask("STI"), "STI\rSTN1130 v5.6.19\r\r>");
    assert_eq!(stn.ask("STDI"), "STDI\rOBDLink SX r4.2\r\r>");
    assert_eq!(stn.ask("AT@1"), "AT@1\rSCANTOOL.NET LLC\r\r>");

    // The OBDLink EX: batched commands as in the example of the manual
    // (FRPM 8.15), the MS-CAN preset, but no single wire CAN.
    let mut ex = Rig::with(10.0, DemoAdapter::ObdLinkEx, true);
    ex.setup();
    assert_eq!(ex.ask("STBC1"), "OK\r\r>");
    assert_eq!(
        ex.ask("STP33|ATH1|ATSH7E0|0100"),
        "OK\r|OK\r|OK\r|7E8 06 41 00 BE 3F B8 13 AA \r\r>"
    );
    assert_eq!(ex.ask("STPR"), "33\r\r>");
    assert_eq!(ex.ask("STP53"), "OK\r\r>");
    assert_eq!(ex.ask("STPRS"), "ISO 15765, 11-bit Tx, 125kbps, DLC=8\r\r>");
    assert_eq!(ex.ask("STP63"), "?\r\r>");
    // An error other than NO DATA ends the batch.
    assert_eq!(ex.ask("ATSH7DF|STXYZ|0100"), "OK\r|?\r\r>");

    // The clone with "OK to all" accepts what it does not know; v1.4 level
    // clones refuse commands of v2.0.
    let mut ok_all = Rig::with(10.0, DemoAdapter::CloneOkToAll, true);
    assert_eq!(ok_all.ask("ATXYZ"), "ATXYZ\rOK\r\r>");
    let mut v21 = Rig::with(10.0, DemoAdapter::CloneV21, true);
    assert_eq!(v21.ask("ATI"), "ATI\rELM327 v2.1\r\r>");
    assert_eq!(v21.ask("ATAMC"), "ATAMC\r?\r\r>");
}

#[test]
fn the_car_connects_and_identifies_itself() {
    // Raw version of "Elm327 and Obd2 connect to the demo car".
    let mut rig = Rig::new(300.0);
    assert_eq!(rig.ask("ATZ"), "ATZ\r\r\rELM327 v1.5\r\r>");
    rig.setup();
    assert_eq!(rig.ask("ATI"), "ELM327 v1.5\r\r>");
    assert_eq!(rig.ask("ATDP"), "ISO 15765-4 (CAN 11/500)\r\r>");

    let messages = rig.messages("0902");
    assert_eq!(messages.len(), 1);
    let vin = String::from_utf8(messages[0][3..].to_vec()).unwrap();
    assert_eq!(vin, DemoTransport::vin());
    assert_eq!(vin.len(), 17);
    assert!(!vin.contains(['I', 'O', 'Q']));
    assert_eq!(vin.chars().nth(8).unwrap(), vin_check_digit(&vin));

    // The support chain: ranges 00, 20, 40, 60, 80, A0 of both units, merged.
    let mut pids = std::collections::BTreeSet::new();
    for base in [0x00u8, 0x20, 0x40, 0x60, 0x80, 0xA0] {
        for message in rig.messages(&format!("01{base:02X}")) {
            let mask = u32::from_be_bytes([message[2], message[3], message[4], message[5]]);
            for bit in 0..32 {
                if mask & (1 << (31 - bit)) != 0 {
                    pids.insert(base as i32 + bit + 1);
                }
            }
        }
    }
    for pid in [
        0x01, 0x05, 0x0C, 0x0D, 0x10, 0x14, 0x15, 0x2F, 0x42, 0x5C, 0x5E,
    ] {
        assert!(pids.contains(&pid), "PID {pid:02X}");
    }
    assert!(!pids.contains(&0x0A));
    // Every supported PID really answers, and nothing else does.
    for pid in 0x01..=0x7Fi32 {
        if pid % 0x20 == 0 {
            continue;
        }
        let answers = rig.messages(&format!("01{pid:02X}"));
        let answered = answers
            .iter()
            .any(|m| m.len() >= 2 && m[0] == 0x41 && m[1] as i32 == pid);
        assert_eq!(answered, pids.contains(&pid), "PID {pid:02X}");
    }

    // Identification: calibration ID and ECU name of the engine.
    let calibration = rig.messages("0904");
    let engine = &calibration[0];
    assert_eq!(&engine[3..15], b"CX5EM21A0417");
    let name = rig.messages("090A");
    assert_eq!(&name[0][3..21], b"ECM\0-EngineControl");
    let cvn = rig.messages("0906");
    assert_eq!(cvn[0], vec![0x49, 0x06, 0x01, 0x5E, 0x71, 0x0C, 0x93]);
}

#[test]
fn without_the_gearbox_the_identification_is_unambiguous() {
    let mut rig = Rig::with(300.0, DemoAdapter::CloneV15, false);
    rig.setup();
    let calibration = rig.messages("0904");
    assert_eq!(calibration.len(), 1);
    assert_eq!(&calibration[0][3..15], b"CX5EM21A0417");
    assert_eq!(calibration[0][15..], [0, 0, 0, 0]);
    let name = rig.messages("090A");
    assert_eq!(name.len(), 1);
    assert_eq!(&name[0][3..21], b"ECM\0-EngineControl");
    // Nobody answers a physical request to 7E1.
    assert_eq!(rig.ask("ATSH7E1"), "OK\r\r>");
    assert_eq!(rig.ask("0904"), "NO DATA\r\r>");
}

#[test]
fn automatic_protocol_search_finds_the_demo_car() {
    // Raw version of the connect with protocol 0 and the CAN-first order.
    let mut rig = Rig::new(100.0);
    assert_eq!(rig.ask("ATE0"), "ATE0\rOK\r\r>");
    assert_eq!(rig.ask("ATSP0"), "OK\r\r>");
    assert_eq!(rig.ask("ATDP"), "AUTO\r\r>");
    assert!(rig.ask("0100").starts_with("SEARCHING...\r41 00 "));
    assert_eq!(rig.ask("ATDP"), "AUTO, ISO 15765-4 (CAN 11/500)\r\r>");

    // Trying the protocols one by one, CAN first, as the driver does.
    let mut other = Rig::new(100.0);
    assert_eq!(other.ask("ATE0"), "ATE0\rOK\r\r>");
    assert_eq!(other.ask("ATTP6"), "OK\r\r>");
    assert!(other.ask("0100").starts_with("41 00 "));
    assert_eq!(other.ask("ATTP3"), "OK\r\r>");
    assert_eq!(other.ask("0100"), "BUS INIT: ...ERROR\r\r>");
}

#[test]
fn demo_faults_read_clear_and_come_back() {
    let mut rig = Rig::connected(300.0);

    assert_eq!(codes(&mut rig, "03"), ["P0171", "P0420"]); // stored
    assert_eq!(codes(&mut rig, "07"), ["P0300"]); // pending: intermittent
    assert_eq!(codes(&mut rig, "0A"), ["P0171", "P0420"]); // permanent

    let status = rig.messages("0101");
    assert_eq!(status[0][2] & 0x80, 0x80); // MIL on
    assert_eq!(status[0][2] & 0x7F, 2); // two stored faults
    assert_eq!(not_ready(&status[0][2..]), 2); // EVAP and EGR/VVT
    assert!(rig.u16_pid(0x31) > 1800.0);

    // Freeze frame 0 belongs to the first fault.
    let freeze = rig.messages("020200");
    assert_eq!(freeze[0], vec![0x42, 0x02, 0x00, 0x01, 0x71]);
    assert_eq!(rig.ask("0204"), rig.ask("0204")); // asking twice changes nothing

    assert_eq!(rig.ask("04"), "44 \r44 \r\r>");

    // Stored and pending are gone; permanent codes stay until the monitors
    // pass again, exactly what mode 0A is for.
    assert_eq!(codes(&mut rig, "03"), Vec::<String>::new());
    assert_eq!(codes(&mut rig, "07"), Vec::<String>::new());
    assert_eq!(codes(&mut rig, "0A"), ["P0171", "P0420"]);
    let status = rig.messages("0101");
    assert_eq!(status[0][2], 0x00); // MIL off, no faults
    assert_eq!(not_ready(&status[0][2..]), 5);
    assert_eq!(rig.u16_pid(0x31), 0.0);
    assert_eq!(rig.u16_pid(0x4E), 0.0);
    assert_eq!(rig.live(0x30), vec![0]);
    // No freeze frame any more: only the "which fault" question answers 00 00.
    assert_eq!(
        rig.messages("020200")[0],
        vec![0x42, 0x02, 0x00, 0x00, 0x00]
    );
    assert_eq!(rig.ask("020500"), "NO DATA\r\r>");

    // After a while of driving the causes are still there.
    rig.clock.advance(700.0);
    assert_eq!(codes(&mut rig, "03"), ["P0171"]);
    assert_eq!(codes(&mut rig, "07"), ["P0300", "P0171", "P0420"]);
    let status = rig.messages("0101");
    assert_eq!(status[0][2], 0x81); // MIL on, one fault
    assert_eq!(not_ready(&status[0][2..]), 1); // EVAP never completes in the demo
    assert!(rig.u16_pid(0x31) > 5.0);
    assert!(rig.u16_pid(0x4E) >= 11.0);
    let freeze = rig.messages("020200");
    assert_eq!(&freeze[0][..3], [0x42, 0x02, 0x00]);
    assert_eq!(&freeze[0][3..], [0x03, 0x00]); // P0300
}

#[test]
fn clearing_over_uds_comes_back_the_same_way() {
    let mut rig = Rig::connected(300.0);
    assert_eq!(rig.ask("ATSH7E0"), "OK\r\r>");
    // Engine fault memory: stored codes carry 0xA8, pending ones 0x24.
    let before = rig.messages("1902FF");
    assert_eq!(before[0][..3], [0x59, 0x02, 0xFF]);
    assert_eq!(before[0].len(), 3 + 4 * 3);
    assert_eq!(rig.ask("14FFFFFF"), "54 \r\r>");
    assert_eq!(rig.messages("1902FF")[0], vec![0x59, 0x02, 0xFF]);
    // The OBD view of the same fault memory is cleared as well.
    assert_eq!(rig.ask("ATSH7DF"), "OK\r\r>");
    assert_eq!(codes(&mut rig, "03"), Vec::<String>::new());
}

#[test]
fn demo_live_values_follow_the_injected_clock() {
    let mut rig = Rig::connected(1.0); // ignition on, engine not yet started

    assert_eq!(rig.rpm(), 0.0);
    assert_eq!(rig.speed(), 0.0);
    assert!(approx(rig.volts(), 12.37, 0.1));
    assert_eq!(rig.ask("ATRV"), "12.6V\r\r>");
    assert!(approx(rig.coolant(), 12.0, 1.0));

    rig.clock.set(20.0); // idling, cold
    let cold_coolant = rig.coolant();
    assert!(rig.rpm() > 900.0);
    assert_eq!(rig.speed(), 0.0);
    assert!(rig.volts() > 13.0);
    assert_eq!(rig.live(0x03)[0], 0x01); // open loop, cold

    // Driving starts 50 s after ignition on, the cycle lasts 120 s.
    // 70 s into a cycle the car cruises at 100 km/h in sixth gear.
    rig.clock.set(50.0 + 3.0 * 120.0 + 70.0);
    assert!(rig.coolant() > cold_coolant + 30.0);
    let speed = rig.speed();
    let rpm = rig.rpm();
    assert!(approx(speed, 100.0, 2.0));
    let gear = rig.live(0xA4);
    assert_eq!(gear.len(), 4);
    assert_eq!(gear[1] >> 4, 6);
    let ratio = (((gear[2] as u32) << 8) | gear[3] as u32) as f64 / 1000.0;
    assert!(approx(ratio, 0.82, 1e-9));
    // rpm from speed, gear ratio, final drive 3.2 and 1.94 m tyre circumference.
    assert!(approx(rpm, speed / 3.6 / 1.94 * 60.0 * ratio * 3.2, 40.0));
    assert_eq!(rig.live(0x03)[0], 0x02); // closed loop
    let ltft = rig.live(0x07)[0] as f64 * 100.0 / 128.0 - 100.0;
    assert!(approx(ltft, 17.2, 0.8));
    assert!(rig.u16_pid(0x10) / 100.0 > 5.0);
    assert!(rig.u16_pid(0x5E) / 20.0 > 2.0);

    // Several PIDs in one request, as CAN allows.
    let multi = rig.messages("010C0D05");
    // 0C a b 0D a 05 a: eight bytes with the service byte, so two frames.
    // Without headers the padding of the last frame is dropped.
    assert_eq!(multi.len(), 1);
    let m = &multi[0];
    assert_eq!(m.len(), 8);
    assert_eq!(m[0], 0x41);
    assert_eq!(m[1], 0x0C);
    assert_eq!(m[4], 0x0D);
    assert_eq!(m[6], 0x05);

    // The O2 sensor switches.
    let (mut low, mut high) = (1.0f64, 0.0f64);
    for _ in 0..20 {
        rig.clock.advance(0.05);
        let v = rig.live(0x14)[0] as f64 / 200.0;
        low = low.min(v);
        high = high.max(v);
    }
    assert!(low < 0.2);
    assert!(high > 0.7);

    // Braking: fuel cut.
    rig.clock.set(50.0 + 3.0 * 120.0 + 90.0);
    assert_eq!(rig.live(0x03)[0], 0x04);
    assert_eq!(rig.u16_pid(0x5E), 0.0);

    // Odometer and fuel level over a longer drive.
    // PID A6 is read raw: the odometer is newer than the PID table.
    let odometer = |rig: &mut Rig| {
        let d = rig.live(0xA6);
        assert_eq!(d.len(), 4);
        (((d[0] as f64 * 256.0 + d[1] as f64) * 256.0 + d[2] as f64) * 256.0 + d[3] as f64) / 10.0
    };
    let odometer_before = odometer(&mut rig);
    let fuel_before = rig.live(0x2F)[0];
    let run_before = rig.u16_pid(0x1F);
    assert!(odometer_before > 48213.0);
    rig.clock.advance(1200.0);
    let odometer_after = odometer(&mut rig);
    assert!(approx(odometer_after - odometer_before, 18.3, 0.5)); // 54.8 km/h
    assert!(rig.live(0x2F)[0] < fuel_before);
    assert!(approx(rig.u16_pid(0x1F), run_before + 1200.0, 1.0));
}

#[test]
fn demo_answers_are_deterministic_with_an_injected_clock() {
    let mut a = Rig::connected(777.7);
    let mut b = Rig::connected(777.7);
    for command in ["010C", "010D", "0114", "0110", "015E", "01A6", "06A4"] {
        assert_eq!(a.ask(command), b.ask(command), "{command}");
    }
    // And the answer depends on the clock, not on real time.
    let first = a.ask("010C");
    assert_eq!(first, a.ask("010C"));
    a.clock.advance(0.3);
    assert_ne!(first, a.ask("010C"));
}

#[test]
fn demo_multi_frame_answers_with_and_without_headers() {
    let mut rig = Rig::connected(300.0);
    assert_eq!(rig.ask("ATS0"), "OK\r\r>");

    // Headers off: total length, then numbered lines.
    let plain = rig.ask("0902");
    let lines: Vec<&str> = plain
        .split('\r')
        .filter(|l| !l.is_empty() && *l != ">")
        .collect();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "014");
    assert!(lines[1].starts_with("0:490201"));
    assert!(lines[2].starts_with("1:"));

    // Headers on: identifier, PCI bytes and the reassembly is up to us.
    assert_eq!(rig.ask("ATH1"), "OK\r\r>");
    let headed = rig.ask("0902");
    let lines: Vec<&str> = headed
        .split('\r')
        .filter(|l| !l.is_empty() && *l != ">")
        .collect();
    assert_eq!(lines.len(), 3);
    let mut payload: Vec<u8> = Vec::new();
    let mut length = 0usize;
    for (i, line) in lines.iter().enumerate() {
        assert!(line.starts_with("7E8"));
        let frame: Vec<u8> = (3..line.len())
            .step_by(2)
            .map(|p| u8::from_str_radix(&line[p..p + 2], 16).unwrap())
            .collect();
        assert_eq!(frame.len(), 8);
        if i == 0 {
            assert_eq!(frame[0], 0x10); // first frame
            length = frame[1] as usize;
            payload.extend_from_slice(&frame[2..]);
        } else {
            assert_eq!(frame[0] as usize, 0x20 + i); // consecutive frames
            payload.extend_from_slice(&frame[1..]);
        }
    }
    assert_eq!(length, 20);
    payload.truncate(length);
    assert_eq!(payload[0], 0x49);
    assert_eq!(payload[1], 0x02);
    assert_eq!(&payload[3..], DemoTransport::vin().as_bytes());

    // Both control units answer the calibration request, three frames each.
    let both = rig.ask("0904");
    let lines: Vec<&str> = both
        .split('\r')
        .filter(|l| !l.is_empty() && *l != ">")
        .collect();
    assert_eq!(lines.len(), 6);
    assert_eq!(lines.iter().filter(|l| l.starts_with("7E9")).count(), 3);
    assert_eq!(rig.ask("ATH0"), "OK\r\r>");
}

#[test]
fn demo_mode_06_reports_misfire_counters_per_cylinder() {
    let mut rig = Rig::connected(600.0);

    let mask = rig.messages("0600");
    assert_eq!(mask[0], vec![0x46, 0x00, 0xC0, 0x00, 0x00, 0x01]);
    let misfire_mask = rig.messages("06A0");
    assert_eq!(misfire_mask[0], vec![0x46, 0xA0, 0xF8, 0x00, 0x00, 0x00]);

    let counters = |rig: &mut Rig, command: &str| -> Vec<i32> {
        let messages = rig.messages(command);
        let m = &messages[0];
        assert_eq!(m[0], 0x46);
        let m = &m[1..];
        assert!(m.len() >= 18);
        assert_eq!(m[1], 0x0B); // EWMA over the last ten driving cycles
        assert_eq!(m[2], 0x24); // counts
        assert_eq!(m[10], 0x0C); // this driving cycle
        vec![
            ((m[3] as i32) << 8) | m[4] as i32,
            ((m[12] as i32) << 8) | m[13] as i32,
        ]
    };
    let cylinder1 = counters(&mut rig, "06A2");
    let cylinder3 = counters(&mut rig, "06A4");
    assert!(cylinder3[0] > cylinder1[0]);
    assert!(cylinder3[1] > cylinder1[1]);

    // The catalyst test fails, which is what P0420 is about.
    let catalyst = rig.messages("0621");
    let c = &catalyst[0][1..];
    let value = ((c[3] as i32) << 8) | c[4] as i32;
    let maximum = ((c[7] as i32) << 8) | c[8] as i32;
    assert!(value > maximum);
}

#[test]
fn demo_uds_units_answer_physical_requests() {
    let mut rig = Rig::connected(300.0);

    // Tester present, with and without the positive answer.
    assert_eq!(rig.ask("ATSH760"), "OK\r\r>");
    assert_eq!(rig.ask("3E00"), "7E 00 \r\r>");
    assert_eq!(rig.ask("3E80"), "NO DATA\r\r>");
    assert_eq!(rig.ask("3E01"), "7F 3E 12 \r\r>");

    // ABS: its fault read first says "response pending", then answers with
    // C0035 (failure type 1C, status AF).
    assert_eq!(rig.ask("190208"), "7F 19 78 \r59 02 FF 40 35 1C AF \r\r>");
    // Clearing empties that unit only.
    assert_eq!(rig.ask("14FFFFFF"), "54 \r\r>");
    assert_eq!(rig.ask("190208"), "7F 19 78 \r59 02 FF \r\r>");
    // Climate keeps U0100 (stored, status 28).
    assert_eq!(rig.ask("ATSH744"), "OK\r\r>");
    assert_eq!(rig.ask("190208"), "59 02 FF C1 00 00 28 \r\r>");
    // Unknown services are refused by name, wrong lengths too.
    assert_eq!(rig.ask("2701"), "7F 27 11 \r\r>");
    assert_eq!(rig.ask("14FFFF"), "7F 14 13 \r\r>");
}

#[test]
fn demo_engine_maker_values_and_identification_over_uds() {
    let mut rig = Rig::connected(300.0);
    assert_eq!(rig.ask("ATSH7E0"), "OK\r\r>");

    // Identification: a string identifier.
    let vin = rig.messages("22F190");
    assert_eq!(&vin[0][..3], [0x62, 0xF1, 0x90]);
    assert_eq!(&vin[0][3..], DemoTransport::vin().as_bytes());
    assert_eq!(rig.ask("22F187"), rig.ask("22F187"));
    assert_eq!(rig.ask("22FFFF"), "7F 22 31 \r\r>");
    assert_eq!(rig.ask("22F1"), "7F 22 13 \r\r>");

    // Maker values: engine running, so the oil level reads 66 +- 2.
    let level = rig.messages("221001")[0].clone();
    assert!(approx(level[3] as f64, 66.0, 2.0));
    let status = rig.messages("221004")[0].clone();
    assert_eq!(status, vec![0x62, 0x10, 0x04, 0x01]);
    let starts = rig.messages("221003")[0].clone();
    assert_eq!(starts, vec![0x62, 0x10, 0x03, 0x00, 0x00, 0x07, 0x2B]);
    // The 16 byte operating block needs several frames.
    let block = rig.messages("221010")[0].clone();
    assert_eq!(block.len(), 3 + 16);
    let seconds = u32::from_be_bytes([block[3], block[4], block[5], block[6]]);
    assert!(seconds > 2_456_000);

    // Engine stopped (ignition on only): the dipstick reads higher at rest.
    rig.clock.set(1.0);
    assert_eq!(rig.messages("221004")[0], vec![0x62, 0x10, 0x04, 0x00]);
    assert_eq!(rig.messages("221001")[0][3], 74);
}

#[test]
fn cluster_and_abs_keep_the_odometer_of_pid_a6() {
    let mut rig = Rig::connected(300.0);
    // PID A6 of the engine, in tenths of a km.
    let a6 = rig.live(0xA6);
    let engine = f64::from(u32::from_be_bytes([a6[0], a6[1], a6[2], a6[3]])) / 10.0;
    assert!(engine > 48_213.0, "{engine}");

    // Instrument cluster: whole km, 32 bit.
    assert_eq!(rig.ask("ATSH720"), "OK\r\r>");
    let cluster = rig.messages("221011")[0].clone();
    assert_eq!(&cluster[..3], [0x62, 0x10, 0x11]);
    let km = f64::from(u32::from_be_bytes([
        cluster[3], cluster[4], cluster[5], cluster[6],
    ]));
    assert_eq!(km, engine.floor());

    // ABS: steps of ten km, 16 bit, so it lags at most ten km behind.
    assert_eq!(rig.ask("ATSH760"), "OK\r\r>");
    let abs = rig.messages("221011")[0].clone();
    assert_eq!(abs.len(), 5);
    let km = f64::from(u16::from_be_bytes([abs[3], abs[4]])) * 10.0;
    assert!(engine - km >= 0.0 && engine - km < 10.0, "{km} {engine}");

    // Only those two units know it; the others refuse the identifier.
    assert_eq!(rig.ask("ATSH737"), "OK\r\r>");
    assert_eq!(rig.ask("221011"), "7F 22 31 \r\r>");
}
