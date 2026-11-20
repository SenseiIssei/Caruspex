// The first test of a KKL cable on the car (pro page "Erster Test am Auto"),
// built for the Audi A2 but fine on any VAG with a K-line:
//
// - the cable test with the cable on USB only: chip and latency timer, port,
//   baud rate, echo, and the bit times of a 5-baud address on the break line;
// - the first test on the car: echo, then the engine (01) woken with the
//   timing measured and retried with another strategy where that can help,
//   identification, fault memory, measuring blocks 001 to 003, then 17, 03,
//   15, 46 and 09 with identification and fault memory.
//
// Everything is read only: nothing is cleared, no output test, no basic
// setting, no adaptation. The port is the one the user picked and is only
// opened when a test starts. "sim:a2" and "sim:a2:<fault>" run against the
// simulated A2 of caruspex-demo.
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use caruspex_demo::a2::{A2Fault, A2Line};
use caruspex_i18n::l;
use caruspex_link::kkl_cable::CableFacts;
use caruspex_link::kline_timed::{
    BIT_LIMIT_US, BIT_TOLERANCE_US, EchoProbe, EchoState, WakeFindings, analyze, judge,
    slow_echo_warning, warnings,
};
use caruspex_link::{
    PortInfo, RawKLine, SerialLine, TimedKLine, WakeStrategy, WakeTrace, WakeVerdict,
    enumerate_ports,
};
use caruspex_protocols::kwp1281::ecu_name;
use caruspex_protocols::{Ecu, KLineLog, MeasuringValue, VagKLine};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::Worker;

type TestLine = TimedKLine<Box<dyn RawKLine>>;

/// The bytes of the echo probe; a sleeping control unit only listens for the
/// 5-baud address and ignores them.
const PROBE: [u8; 3] = [0x55, 0xAA, 0x0F];
/// The line rests after the probe before the wake-up (ISO 9141 W5 >= 300 ms).
const REST_AFTER_PROBE_MS: u64 = 1000;
/// VAG units listen for a new address about two and a half seconds after a
/// failed one.
const REST_AFTER_FAIL_MS: u64 = 2600;
const ENGINE_ATTEMPTS: usize = 3;
/// The units after the engine, in the order of the test.
const OTHER_UNITS: [u8; 5] = [0x17, 0x03, 0x15, 0x46, 0x09];
const GROUPS: [u8; 3] = [1, 2, 3];

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KklCableRow {
    pub device: String,
    pub description: String,
    pub vid: u16,
    pub pid: u16,
    pub chip: String,
    /// "ftdi", "ch340", "prolific", "cp210x", "other", "none".
    pub family: String,
    pub family_name: String,
    pub latency_ms: Option<u32>,
    pub chip_advice: String,
    pub latency_advice: String,
    pub simulated: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KklEchoRow {
    pub sent: String,
    pub received: String,
    /// "clean", "none", "broken".
    pub state: String,
    pub round_trip_ms: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KklPhaseRow {
    pub one: bool,
    pub bits: u8,
    pub measured_ms: f64,
    pub target_ms: f64,
    pub call_ms: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KklWakeRow {
    pub address: u8,
    pub unit: String,
    pub attempt: usize,
    /// "standard" or "compensated".
    pub strategy: String,
    pub baud: u32,
    pub phases: Vec<KklPhaseRow>,
    pub worst_error_ms: f64,
    pub address_ms: f64,
    pub on_call_ms: f64,
    pub off_call_ms: f64,
    pub address_echo: usize,
    pub first_byte: String,
    pub first_byte_after_ms: Option<f64>,
    pub key_bytes: String,
    pub reply_after_ms: Option<f64>,
    pub reply_echo: Option<bool>,
    pub reply_round_trip_ms: Option<f64>,
    pub verdict: String,
    pub headline: String,
    pub error: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KklVerdictRow {
    pub kind: String,
    pub ok: bool,
    pub headline: String,
    pub advice: String,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KklTestStep {
    pub id: String,
    pub title: String,
    /// "running", "ok", "warn", "fail", "skip".
    pub state: String,
    pub summary: String,
    pub lines: Vec<String>,
    pub log: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KklTestReport {
    /// "cable" or "car".
    pub kind: String,
    pub version: String,
    pub platform: String,
    pub date: String,
    pub device: String,
    pub cable: KklCableRow,
    pub echo: Option<KklEchoRow>,
    pub wakes: Vec<KklWakeRow>,
    pub steps: Vec<KklTestStep>,
    pub verdict: KklVerdictRow,
    pub cancelled: bool,
}

fn fill(text: &str, values: &[&dyn std::fmt::Display]) -> String {
    crate::texts::args(text, values)
}

fn ms(us: u64) -> f64 {
    (us as f64 / 100.0).round() / 10.0
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The facts about the chosen port, from the port list (nothing is opened).
fn cable_row(device: &str) -> KklCableRow {
    let (port, latency, simulated) = match A2Fault::from_device(device) {
        Some(fault) => (
            PortInfo {
                device: device.to_string(),
                description: format!("FTDI FT232R USB UART ({})", l("simuliert", "simulated")),
                vid: 0x0403,
                pid: 0x6001,
                chip_guess: "FTDI FT232R".into(),
                likely_adapter: true,
                ..PortInfo::default()
            },
            Some(fault.latency_ms()),
            true,
        ),
        None => {
            let port = enumerate_ports()
                .into_iter()
                .find(|p| p.device == device)
                .unwrap_or_else(|| PortInfo {
                    device: device.to_string(),
                    description: device.to_string(),
                    ..PortInfo::default()
                });
            let latency = CableFacts::of_port(&port).latency_ms;
            (port, latency, false)
        }
    };
    let facts = CableFacts {
        family: caruspex_link::ChipFamily::from_ids(port.vid, port.pid),
        latency_ms: latency,
    };
    KklCableRow {
        device: port.device.clone(),
        description: port.description.clone(),
        vid: port.vid,
        pid: port.pid,
        chip: port.chip_guess.clone(),
        family: facts.family.key().into(),
        family_name: facts.family.name().into(),
        latency_ms: facts.latency_ms,
        chip_advice: facts.family.advice().into(),
        latency_advice: facts.latency_advice(),
        simulated,
    }
}

fn open_line(device: &str) -> Result<Box<dyn RawKLine>, String> {
    if let Some(fault) = A2Fault::from_device(device) {
        return Ok(Box::new(A2Line::new(fault)));
    }
    if device.is_empty() {
        return Err(l(
            "Kein Anschluss gewählt. Das KKL-Kabel anstecken und auf Suchen drücken.",
            "No port chosen. Plug in the KKL cable and press Search.",
        )
        .to_string());
    }
    if device.contains(':') && !device.starts_with('/') && !device.starts_with(r"\\") {
        return Err(l(
            "Der erste Test braucht ein KKL-Kabel an einem seriellen Port.",
            "The first test needs a KKL cable on a serial port.",
        )
        .to_string());
    }
    Ok(Box::new(SerialLine::open(device, 9600)?))
}

fn echo_row(probe: &EchoProbe) -> KklEchoRow {
    KklEchoRow {
        sent: hex_bytes(&probe.sent),
        received: hex_bytes(&probe.received),
        state: match probe.state() {
            EchoState::Clean => "clean",
            EchoState::None => "none",
            EchoState::Broken => "broken",
        }
        .into(),
        round_trip_ms: probe.worst_round_trip_us().map(ms),
    }
}

fn strategy_label(key: &str) -> &'static str {
    if key == "compensated" {
        l("Break-Aufrufe vorgezogen", "break calls started early")
    } else {
        l("normal getaktet", "standard timing")
    }
}

fn strategy_name(strategy: WakeStrategy) -> &'static str {
    strategy_label(strategy.key())
}

fn wake_row(
    ecu: Ecu,
    attempt: usize,
    trace: &WakeTrace,
    f: &WakeFindings,
    verdict: WakeVerdict,
    error: &str,
) -> KklWakeRow {
    KklWakeRow {
        address: ecu.address(),
        unit: ecu_name(ecu).to_string(),
        attempt,
        strategy: trace.strategy.key().into(),
        baud: trace.baud,
        phases: f
            .phases
            .iter()
            .map(|p| KklPhaseRow {
                one: p.one,
                bits: p.bits,
                measured_ms: ms(p.measured_us),
                target_ms: ms(p.target_us),
                call_ms: ms(p.call_us),
            })
            .collect(),
        worst_error_ms: ms(f.worst_edge_error_us),
        address_ms: ms(trace.address_us),
        on_call_ms: ms(f.mean_on_call_us),
        off_call_ms: ms(f.mean_off_call_us),
        address_echo: f.address_echo,
        first_byte: f.first_byte.map(|b| format!("{b:02X}")).unwrap_or_default(),
        first_byte_after_ms: f.first_byte_after_us.map(ms),
        key_bytes: f
            .key_bytes
            .map(|(a, b)| hex_bytes(&[a, b]))
            .unwrap_or_default(),
        reply_after_ms: f.reply_after_us.map(ms),
        reply_echo: f.reply_echo,
        reply_round_trip_ms: f.reply_round_trip_us.map(ms),
        verdict: verdict.key().into(),
        headline: verdict.headline().into(),
        error: error.to_string(),
    }
}

/// The measurements of one wake-up as lines for the step.
fn wake_lines(row: &KklWakeRow) -> Vec<String> {
    let mut lines = vec![fill(
        l("Versuch %1: %2, %3 Baud", "Attempt %1: %2, %3 baud"),
        &[&row.attempt, &strategy_label(&row.strategy), &row.baud],
    )];
    lines.push(fill(
        l(
            "Bits der Adresse: größte Abweichung %1 ms (erlaubt 10 ms), ganze Adresse %2 ms (Soll 2000 ms)",
            "Address bits: largest deviation %1 ms (10 ms allowed), whole address %2 ms (target 2000 ms)",
        ),
        &[&row.worst_error_ms, &row.address_ms],
    ));
    let phases: Vec<String> = row
        .phases
        .iter()
        .map(|p| {
            format!(
                "{} {}x200={} ms",
                if p.one { "1" } else { "0" },
                p.bits,
                p.measured_ms
            )
        })
        .collect();
    lines.push(phases.join(", "));
    lines.push(fill(
        l(
            "Break-Aufruf dauert: an %1 ms, aus %2 ms",
            "Break call takes: on %1 ms, off %2 ms",
        ),
        &[&row.on_call_ms, &row.off_call_ms],
    ));
    if let Some(after) = row.first_byte_after_ms {
        lines.push(fill(
            l(
                "Erstes Byte %1 nach %2 ms, Schlüsselbytes %3",
                "First byte %1 after %2 ms, key bytes %3",
            ),
            &[
                &row.first_byte,
                &after,
                &if row.key_bytes.is_empty() {
                    "-".to_string()
                } else {
                    row.key_bytes.clone()
                },
            ],
        ));
    } else {
        lines.push(
            l(
                "Nach der Adresse kam kein Byte.",
                "No byte came after the address.",
            )
            .to_string(),
        );
    }
    if let Some(reply) = row.reply_after_ms {
        let echo = row
            .reply_round_trip_ms
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into());
        lines.push(fill(
            l(
                "Antwort auf die Schlüsselbytes nach %1 ms, Echo nach %2 ms",
                "Reply to the key bytes after %1 ms, echo after %2 ms",
            ),
            &[&reply, &echo],
        ));
    }
    if !row.error.is_empty() {
        lines.push(row.error.replace('\n', " "));
    }
    lines
}

fn measured_line(group: u8, values: &[MeasuringValue]) -> String {
    let fields: Vec<String> = values
        .iter()
        .filter(|m| m.valid)
        .map(|m| {
            let value = if m.value.fract() == 0.0 {
                format!("{}", m.value)
            } else {
                format!("{:.2}", m.value)
            };
            format!("{} {} {}", m.label, value, m.unit)
                .trim_end()
                .to_string()
        })
        .collect();
    format!("{group:03}: {}", fields.join(" | "))
}

/// The step texts that come up again and again.
fn step(id: &str, title: &str) -> KklTestStep {
    KklTestStep {
        id: id.into(),
        title: title.into(),
        state: "running".into(),
        ..KklTestStep::default()
    }
}

fn verdict_row(verdict: WakeVerdict) -> KklVerdictRow {
    KklVerdictRow {
        kind: verdict.key().into(),
        ok: verdict.is_ok(),
        headline: verdict.headline().into(),
        advice: verdict.advice().into(),
        warnings: Vec::new(),
    }
}

fn faults_lines(faults: &[caruspex_protocols::Kwp1281Fault]) -> (String, Vec<String>) {
    let summary = if faults.is_empty() {
        l(
            "Kein Eintrag im Fehlerspeicher",
            "No entries in the fault memory",
        )
        .to_string()
    } else if faults.len() == 1 {
        l("1 Eintrag im Fehlerspeicher", "1 entry in the fault memory").to_string()
    } else {
        fill(
            l(
                "%1 Einträge im Fehlerspeicher",
                "%1 entries in the fault memory",
            ),
            &[&faults.len()],
        )
    };
    let lines = faults
        .iter()
        .map(|f| {
            format!(
                "{:05} ({}): {}",
                f.code,
                if f.sporadic {
                    l("sporadisch", "intermittent")
                } else {
                    l("gespeichert", "stored")
                },
                f.text
            )
        })
        .collect();
    (summary, lines)
}

impl Worker {
    fn kkl_test_step(&self, step: &KklTestStep) {
        self.emit("kkl_test_step", json!({ "step": step }));
    }

    fn kkl_test_finish(&mut self, report: KklTestReport) {
        self.busy(false);
        self.emit("kkl_test_finished", json!({ "report": report }));
    }

    fn new_report(&self, kind: &str, device: &str) -> KklTestReport {
        KklTestReport {
            kind: kind.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            platform: std::env::consts::OS.into(),
            date: jiff::Zoned::now().strftime("%Y-%m-%d").to_string(),
            device: device.into(),
            cable: cable_row(device),
            ..KklTestReport::default()
        }
    }

    /// The cable test with the cable on USB only.
    pub(super) fn kkl_cable_check(&mut self, device: &str) {
        self.close_kkl();
        self.cancel.store(false, Ordering::SeqCst);
        self.busy(true);
        let mut report = self.new_report("cable", device);
        let latency = report.cable.latency_ms;
        let mut warn: Vec<String> = Vec::new();
        if !report.cable.latency_advice.is_empty() && report.cable.latency_ms.is_some() {
            warn.push(report.cable.latency_advice.clone());
        }

        let mut port_step = step(
            "port",
            l(
                "Port öffnen und Baudrate wechseln",
                "Open the port and switch the baud rate",
            ),
        );
        self.kkl_test_step(&port_step);
        let line = match open_line(device) {
            Ok(line) => line,
            Err(error) => {
                port_step.state = "fail".into();
                port_step.summary = error.clone();
                self.kkl_test_step(&port_step);
                report.steps.push(port_step);
                report.verdict = verdict_row(WakeVerdict::PortProblem);
                report.verdict.warnings.push(error);
                return self.kkl_test_finish(report);
            }
        };
        let mut stream: TestLine = TimedKLine::new(line, 9600);
        let switched = stream.line_mut().set_baud(10400) && stream.line_mut().set_baud(9600);
        port_step.state = if switched { "ok" } else { "fail" }.into();
        port_step.summary = if switched {
            l(
                "Port offen, 9600 und 10400 Baud lassen sich setzen",
                "Port open, 9600 and 10400 baud can be set",
            )
        } else {
            l(
                "Die Baudrate lässt sich nicht wechseln",
                "The baud rate cannot be switched",
            )
        }
        .into();
        self.kkl_test_step(&port_step);
        report.steps.push(port_step);
        if !switched {
            report.verdict = verdict_row(WakeVerdict::PortProblem);
            return self.kkl_test_finish(report);
        }

        let mut echo_step = step("echo", l("Echo der K-Line", "Echo of the K-line"));
        self.kkl_test_step(&echo_step);
        let probe = stream.echo_probe(&PROBE);
        let echo = echo_row(&probe);
        echo_step.lines.push(fill(
            l("gesendet %1, zurück %2", "sent %1, back %2"),
            &[
                &echo.sent,
                &if echo.received.is_empty() {
                    "-".to_string()
                } else {
                    echo.received.clone()
                },
            ],
        ));
        let verdict = match probe.state() {
            EchoState::Clean => {
                echo_step.state = "ok".into();
                echo_step.summary = WakeVerdict::CableOnCar.headline().into();
                Some(WakeVerdict::CableOnCar)
            }
            EchoState::Broken => {
                echo_step.state = "fail".into();
                echo_step.summary = WakeVerdict::EchoBroken.headline().into();
                Some(WakeVerdict::EchoBroken)
            }
            EchoState::None => {
                echo_step.state = "ok".into();
                echo_step.summary = l(
                    "Kein Echo, so soll es ohne Auto sein",
                    "No echo, as it should be without the car",
                )
                .into();
                None
            }
        };
        warn.extend(probe.worst_round_trip_us().and_then(slow_echo_warning));
        report.echo = Some(echo);
        self.kkl_test_step(&echo_step);
        report.steps.push(echo_step);
        if let Some(verdict) = verdict {
            report.verdict = verdict_row(verdict);
            report.verdict.warnings = warn;
            return self.kkl_test_finish(report);
        }

        // Only with nothing on the line: the bits of a real address on the
        // break line, measured the way the first test sends them.
        let mut break_step = step(
            "break",
            l("Break-Leitung und Bitzeiten", "Break line and bit times"),
        );
        self.kkl_test_step(&break_step);
        stream.pause_ms(REST_AFTER_PROBE_MS);
        let trace = stream.clock_address(0x01);
        let findings = analyze(&trace);
        let verdict = if trace.break_failed {
            WakeVerdict::PortProblem
        } else if findings.worst_edge_error_us > BIT_LIMIT_US
            || findings.address_error_us.unsigned_abs() > 2 * BIT_LIMIT_US
        {
            WakeVerdict::TimingUnstable
        } else {
            WakeVerdict::CableReady
        };
        let row = wake_row(Ecu::Engine, 1, &trace, &findings, verdict, "");
        break_step.lines = wake_lines(&row);
        // Without the car nothing can answer, so the line about it goes.
        break_step.lines.retain(|line| {
            line != l(
                "Nach der Adresse kam kein Byte.",
                "No byte came after the address.",
            )
        });
        break_step.state = match verdict {
            WakeVerdict::CableReady if findings.worst_edge_error_us > BIT_TOLERANCE_US => "warn",
            WakeVerdict::CableReady => "ok",
            _ => "fail",
        }
        .into();
        break_step.summary = verdict.headline().into();
        self.kkl_test_step(&break_step);
        report.steps.push(break_step);
        report.wakes.push(row);
        warn.extend(warnings(&findings, latency));
        report.verdict = verdict_row(verdict);
        report.verdict.warnings = warn;
        self.kkl_test_finish(report);
    }

    /// Wakes `ecu`, up to `attempts` times with the strategy adapted to
    /// what the last attempt measured.
    fn kkl_test_wake(
        &mut self,
        vag: &mut VagKLine<TestLine>,
        ecu: Ecu,
        attempts: usize,
        latency: Option<u32>,
        rows: &mut Vec<KklWakeRow>,
    ) -> (WakeVerdict, Vec<String>) {
        let mut lines = Vec::new();
        let mut verdict = WakeVerdict::NoAnswer;
        let mut tried_10400 = false;
        for attempt in 1..=attempts {
            if self.cancelled() {
                break;
            }
            let result = vag.connect(ecu);
            let trace = vag.stream_mut().take_trace().unwrap_or_default();
            let findings = analyze(&trace);
            verdict = judge(&trace, &findings, result.is_ok(), latency);
            let row = wake_row(
                ecu,
                attempt,
                &trace,
                &findings,
                verdict,
                result.as_ref().err().map(String::as_str).unwrap_or(""),
            );
            lines.extend(wake_lines(&row));
            if verdict == WakeVerdict::Connected {
                lines.extend(warnings(&findings, latency));
            }
            rows.push(row);
            if verdict == WakeVerdict::Connected || attempt == attempts {
                break;
            }
            let stream = vag.stream_mut();
            match verdict {
                WakeVerdict::TimingUnstable => stream.set_strategy(WakeStrategy::Compensated {
                    on_lead_us: findings.mean_on_call_us,
                    off_lead_us: findings.mean_off_call_us,
                }),
                WakeVerdict::WrongSync if !tried_10400 => {
                    tried_10400 = true;
                    stream.set_baud_after_init(10400);
                }
                // Worth the same once more: the unit may just not have been
                // ready.
                WakeVerdict::NoAnswer
                | WakeVerdict::KeyBytesMissing
                | WakeVerdict::ReplyRejected
                | WakeVerdict::ProtocolError => {}
                // Nothing in software helps here.
                _ => break,
            }
            self.log(fill(
                l(
                    "%1: neuer Versuch in 2,6 s mit %2",
                    "%1: next attempt in 2.6 s with %2",
                ),
                &[&ecu_name(ecu), &strategy_name(stream.strategy())],
            ));
            stream.pause_ms(REST_AFTER_FAIL_MS);
        }
        (verdict, lines)
    }

    /// The guided first test on the car, read only.
    pub(super) fn kkl_first_test(&mut self, device: &str) {
        self.close_kkl();
        self.cancel.store(false, Ordering::SeqCst);
        self.busy(true);
        let mut report = self.new_report("car", device);
        let latency = report.cable.latency_ms;
        let collected: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let take_log = |c: &Arc<Mutex<Vec<String>>>| -> Vec<String> {
            c.lock()
                .map(|mut v| std::mem::take(&mut *v))
                .unwrap_or_default()
        };
        let mut warn: Vec<String> = Vec::new();
        if report.cable.latency_ms.is_some() && !report.cable.latency_advice.is_empty() {
            warn.push(report.cable.latency_advice.clone());
        }

        // The plan, so the page can show every step from the start.
        let mut plan: Vec<KklTestStep> = vec![
            step("cable", l("Kabel und Echo", "Cable and echo")),
            step(
                "wake",
                l("Motor wecken (Adresse 01)", "Wake the engine (address 01)"),
            ),
            step("ident", l("Kennung lesen", "Read the identification")),
            step("faults", l("Fehlerspeicher lesen", "Read the fault memory")),
            step(
                "groups",
                l("Messwertblöcke 001 bis 003", "Measuring blocks 001 to 003"),
            ),
        ];
        for address in OTHER_UNITS {
            plan.push(step(
                &format!("unit-{address:02x}"),
                ecu_name(Ecu::from_address(address)),
            ));
        }
        for s in plan.iter_mut() {
            s.state = "pending".into();
            self.kkl_test_step(s);
        }
        let mut next = 0usize;
        // Marks the step `index` as running and hands it out.
        macro_rules! begin {
            () => {{
                let mut s = plan[next].clone();
                s.state = "running".into();
                self.kkl_test_step(&s);
                s
            }};
        }
        macro_rules! end {
            ($s:expr) => {{
                let mut s = $s;
                s.log.extend(take_log(&collected));
                self.kkl_test_step(&s);
                plan[next] = s;
                next += 1;
            }};
        }
        macro_rules! finish {
            ($verdict:expr) => {{
                for s in plan.iter_mut().skip(next) {
                    s.state = "skip".into();
                    s.summary = l("nicht gelaufen", "not run").into();
                    self.kkl_test_step(s);
                }
                report.steps = plan;
                report.cancelled = self.cancelled();
                report.verdict = verdict_row($verdict);
                report.verdict.warnings = warn;
                return self.kkl_test_finish(report);
            }};
        }

        // 1. Cable and echo.
        let mut s = begin!();
        s.lines.push(format!(
            "{} ({}), {}",
            report.cable.description, report.cable.family_name, report.cable.chip_advice
        ));
        if let Some(latency) = report.cable.latency_ms {
            s.lines.push(fill(
                l(
                    "Latenz-Timer des Treibers: %1 ms",
                    "Latency timer of the driver: %1 ms",
                ),
                &[&latency],
            ));
        }
        let line = match open_line(device) {
            Ok(line) => line,
            Err(error) => {
                s.state = "fail".into();
                s.summary = error.clone();
                end!(s);
                warn.push(error);
                finish!(WakeVerdict::PortProblem);
            }
        };
        let mut stream: TestLine = TimedKLine::new(line, 9600);
        let probe = stream.echo_probe(&PROBE);
        let echo = echo_row(&probe);
        s.lines.push(fill(
            l("gesendet %1, zurück %2", "sent %1, back %2"),
            &[
                &echo.sent,
                &if echo.received.is_empty() {
                    "-".to_string()
                } else {
                    echo.received.clone()
                },
            ],
        ));
        let echo_state = probe.state();
        warn.extend(probe.worst_round_trip_us().and_then(slow_echo_warning));
        if let Some(rtt) = echo.round_trip_ms {
            s.lines
                .push(fill(l("Echo nach %1 ms", "Echo after %1 ms"), &[&rtt]));
        }
        report.echo = Some(echo);
        match echo_state {
            EchoState::Clean => {
                s.state = "ok".into();
                s.summary = l(
                    "Das Kabel hört sich selbst, die K-Line hat Strom",
                    "The cable hears itself, the K-line has power",
                )
                .into();
                end!(s);
            }
            EchoState::None | EchoState::Broken => {
                let verdict = if echo_state == EchoState::None {
                    WakeVerdict::NoEcho
                } else {
                    WakeVerdict::EchoBroken
                };
                s.state = "fail".into();
                s.summary = verdict.headline().into();
                end!(s);
                finish!(verdict);
            }
        }
        stream.pause_ms(REST_AFTER_PROBE_MS);

        // 2. The engine.
        let sink = self.sink.clone();
        let into = collected.clone();
        let log: KLineLog = Arc::new(move |m: &str| {
            if let Ok(mut v) = into.lock() {
                v.push(m.to_string());
            }
            sink.emit("log_line", json!({ "line": m }));
        });
        let mut vag = VagKLine::new(stream, Some(log));
        let mut s = begin!();
        let (engine, lines) = self.kkl_test_wake(
            &mut vag,
            Ecu::Engine,
            ENGINE_ATTEMPTS,
            latency,
            &mut report.wakes,
        );
        s.lines = lines;
        s.summary = engine.headline().into();
        s.state = if engine == WakeVerdict::Connected {
            "ok"
        } else {
            "fail"
        }
        .into();
        end!(s);
        if self.cancelled() || engine.stops_the_test() {
            finish!(engine);
        }

        if engine == WakeVerdict::Connected {
            // 3. Identification.
            let mut s = begin!();
            s.lines = vag
                .identification()
                .iter()
                .map(|l| l.trim().to_string())
                .collect();
            s.summary = vag.protocol_name().into();
            s.state = "ok".into();
            end!(s);

            // 4. Fault memory.
            let mut s = begin!();
            match vag.read_faults() {
                Ok(faults) => {
                    (s.summary, s.lines) = faults_lines(&faults);
                    s.state = "ok".into();
                }
                Err(error) => {
                    s.summary = error;
                    s.state = "fail".into();
                }
            }
            end!(s);

            // 5. Measuring blocks.
            let mut s = begin!();
            let mut read = 0;
            for group in GROUPS {
                if self.cancelled() {
                    break;
                }
                match vag.read_group(group) {
                    Ok(values) => {
                        read += 1;
                        s.lines.push(measured_line(group, &values));
                    }
                    Err(error) => s.lines.push(format!("{group:03}: {error}")),
                }
            }
            s.summary = fill(
                l("%1 von 3 Blöcken gelesen", "%1 of 3 blocks read"),
                &[&read],
            );
            s.state = match read {
                3 => "ok",
                0 => "fail",
                _ => "warn",
            }
            .into();
            // Signing off belongs to the engine's log.
            vag.disconnect();
            end!(s);
        } else {
            // Without the engine its three steps cannot run; the other units
            // may still answer.
            for _ in 0..3 {
                let mut s = plan[next].clone();
                s.state = "skip".into();
                s.summary = l("nicht gelaufen", "not run").into();
                end!(s);
            }
            vag.stream_mut().set_strategy(WakeStrategy::Standard);
            vag.stream_mut().set_baud_after_init(9600);
        }

        // 6. The other units: identification and fault memory.
        let mut answered = usize::from(engine == WakeVerdict::Connected);
        for address in OTHER_UNITS {
            if self.cancelled() {
                break;
            }
            let ecu = Ecu::from_address(address);
            let mut s = begin!();
            let (verdict, lines) = self.kkl_test_wake(&mut vag, ecu, 1, latency, &mut report.wakes);
            if verdict == WakeVerdict::Connected {
                answered += 1;
                s.lines = vag
                    .identification()
                    .iter()
                    .map(|l| l.trim().to_string())
                    .collect();
                match vag.read_faults() {
                    Ok(faults) => {
                        let (summary, fault_lines) = faults_lines(&faults);
                        s.summary = format!("{}, {summary}", vag.protocol_name());
                        s.lines.extend(fault_lines);
                        s.state = "ok".into();
                    }
                    Err(error) => {
                        s.summary = error;
                        s.state = "fail".into();
                    }
                }
                vag.disconnect();
                // The timing in the log only, the lines show what was read.
                s.log.extend(lines);
            } else {
                s.lines = lines;
                if verdict == WakeVerdict::NoAnswer {
                    s.state = "warn".into();
                    s.summary = l(
                        "meldet sich nicht, vielleicht nicht verbaut",
                        "does not answer, maybe not fitted",
                    )
                    .into();
                } else {
                    s.state = "fail".into();
                    s.summary = verdict.headline().into();
                }
            }
            end!(s);
            if verdict.stops_the_test() {
                finish!(verdict);
            }
        }
        drop(vag);

        let total = 1 + OTHER_UNITS.len();
        let mut verdict = verdict_row(engine);
        if engine == WakeVerdict::Connected {
            verdict.headline = l(
                "Die K-Line läuft: der Motor hat sich gemeldet",
                "The K-line works: the engine answered",
            )
            .into();
            verdict.advice = fill(
                l(
                    "Erster Test fertig: %1 von %2 Steuergeräten haben geantwortet. Den Bericht zu speichern und uns zu schicken hilft, die K-Line auf weiteren Autos zu prüfen.",
                    "First test done: %1 of %2 control units answered. Saving the report and sending it to us helps to check the K-line on more cars.",
                ),
                &[&answered, &total],
            );
        }
        for s in plan.iter_mut().skip(next) {
            s.state = "skip".into();
            s.summary = l("nicht gelaufen", "not run").into();
            self.kkl_test_step(s);
        }
        report.steps = plan;
        report.cancelled = self.cancelled();
        verdict.warnings = warn;
        report.verdict = verdict;
        self.kkl_test_finish(report);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use serde_json::Value;

    use super::*;
    use crate::settings::AppSettings;
    use crate::worker::{Job, Sink};

    #[derive(Default)]
    struct Events(Mutex<Vec<(String, Value)>>);

    impl Sink for Events {
        fn emit(&self, event: &str, payload: Value) {
            self.0.lock().unwrap().push((event.to_string(), payload));
        }
    }

    fn run(job: Job) -> (KklTestReport, Vec<String>) {
        let sink = Arc::new(Events::default());
        let mut worker = Worker::new(sink.clone(), Arc::new(AtomicBool::new(false)));
        worker.apply_settings(AppSettings::default());
        worker.handle(job);
        let events = sink.0.lock().unwrap();
        let finished: Vec<&Value> = events
            .iter()
            .filter(|(e, _)| e == "kkl_test_finished")
            .map(|(_, p)| p)
            .collect();
        assert_eq!(finished.len(), 1);
        let report: KklTestReport = serde_json::from_value(finished[0]["report"].clone()).unwrap();
        let log = events
            .iter()
            .filter(|(e, _)| e == "log_line")
            .filter_map(|(_, p)| p["line"].as_str().map(str::to_string))
            .collect();
        (report, log)
    }

    fn first_test(fault: A2Fault) -> (KklTestReport, Vec<String>) {
        run(Job::KklFirstTest(fault.device()))
    }

    fn state<'a>(report: &'a KklTestReport, id: &str) -> &'a str {
        &report.steps.iter().find(|s| s.id == id).unwrap().state
    }

    #[test]
    fn the_first_test_reads_the_whole_a2_and_writes_nothing() {
        let (report, log) = first_test(A2Fault::None);
        assert_eq!(report.verdict.kind, "connected", "{report:#?}");
        assert!(report.verdict.ok);
        for id in ["cable", "wake", "ident", "faults", "groups"] {
            assert_eq!(state(&report, id), "ok", "{id}: {report:#?}");
        }
        let ident = report.steps.iter().find(|s| s.id == "ident").unwrap();
        assert!(ident.lines[0].starts_with("036906034AM"));
        let faults = report.steps.iter().find(|s| s.id == "faults").unwrap();
        assert_eq!(faults.lines.len(), 1);
        assert!(faults.lines[0].starts_with("16684"), "{faults:?}");
        let groups = report.steps.iter().find(|s| s.id == "groups").unwrap();
        assert_eq!(groups.lines.len(), 3);
        assert!(groups.lines[0].starts_with("001:") && groups.lines[0].contains("800"));
        for id in ["unit-17", "unit-03", "unit-15", "unit-46"] {
            assert_eq!(state(&report, id), "ok", "{id}");
        }
        // 09 is not fitted in the simulated car.
        assert_eq!(state(&report, "unit-09"), "warn");
        let cluster = report.steps.iter().find(|s| s.id == "unit-17").unwrap();
        assert!(cluster.lines.iter().any(|l| l == "WAUZZZ8Z01N000417"));
        // Every unit got one attempt with the standard timing.
        assert_eq!(report.wakes.len(), 6);
        assert!(report.wakes.iter().all(|w| w.strategy == "standard"));
        assert!(report.wakes[0].worst_error_ms < 5.0);
        assert!(
            report.verdict.advice.contains("5"),
            "{}",
            report.verdict.advice
        );
        // Read only: no clearing (05), output test (04), basic settings (28)
        // or adaptation (2A, 2B, 2C) went out.
        for title in ["05", "04", "28", "2A", "2B", "2C"] {
            assert!(
                !log.iter().any(|l| l.contains(&format!("-> Block {title}"))),
                "{title}"
            );
        }
        assert!(!report.cancelled);
    }

    #[test]
    fn a_slow_break_line_is_retried_with_earlier_calls() {
        let (report, _) = first_test(A2Fault::SlowBreak);
        assert_eq!(report.wakes[0].verdict, "timing-unstable");
        assert!(report.wakes[0].worst_error_ms > 30.0);
        assert!(report.wakes[0].off_call_ms > 30.0);
        assert_eq!(report.wakes[1].strategy, "compensated");
        assert_eq!(report.wakes[1].verdict, "connected");
        // With the calls started early every stretch is on time, the stop
        // bit included, and the sync byte comes about 40 ms after it.
        let fixed = &report.wakes[1];
        assert!(fixed.worst_error_ms < 2.0, "{fixed:?}");
        assert!(
            fixed
                .phases
                .iter()
                .all(|p| (p.measured_ms - p.target_ms).abs() < 2.0),
            "{fixed:?}"
        );
        assert!(fixed.first_byte_after_ms.unwrap() > 30.0, "{fixed:?}");
        assert_eq!(report.verdict.kind, "connected");
        // The units after the engine keep the strategy that worked.
        assert!(
            report.wakes[2..]
                .iter()
                .all(|w| w.strategy == "compensated")
        );
    }

    #[test]
    fn a_slow_chip_makes_the_reply_to_the_key_bytes_late() {
        let (report, _) = first_test(A2Fault::SlowRead);
        assert_eq!(report.verdict.kind, "reply-too-late", "{report:#?}");
        assert_eq!(report.cable.latency_ms, Some(40));
        // Nothing in software helps, so the engine got one attempt.
        assert_eq!(report.wakes.iter().filter(|w| w.address == 1).count(), 1);
        assert!(report.verdict.warnings.iter().any(|w| w.contains("40 ms")));
        // The latency hint and the slow echo of the probe.
        assert_eq!(
            report.verdict.warnings.len(),
            2,
            "{:?}",
            report.verdict.warnings
        );
        // The other units would only fail the same way.
        assert_eq!(state(&report, "ident"), "skip");
        assert_eq!(state(&report, "unit-17"), "skip");
    }

    #[test]
    fn a_silent_car_is_a_question_of_the_ignition() {
        let (report, _) = first_test(A2Fault::NoAnswer);
        assert_eq!(report.verdict.kind, "no-answer");
        assert_eq!(state(&report, "cable"), "ok");
        assert_eq!(state(&report, "wake"), "fail");
        assert_eq!(
            report.wakes.iter().filter(|w| w.address == 1).count(),
            ENGINE_ATTEMPTS
        );
        assert_eq!(state(&report, "unit-17"), "warn");
    }

    #[test]
    fn a_rejected_key_reply_is_told_apart_from_a_late_one() {
        let (report, _) = first_test(A2Fault::KeyInversion);
        assert_eq!(report.verdict.kind, "reply-rejected");
        let w = &report.wakes[0];
        assert_eq!(w.key_bytes, "01 8A");
        assert_eq!(w.reply_echo, Some(true));
        assert!(w.reply_after_ms.unwrap() < 50.0);
    }

    #[test]
    fn echo_faults_stop_the_test_at_the_cable() {
        let (report, _) = first_test(A2Fault::BrokenEcho);
        assert_eq!(report.verdict.kind, "echo-broken");
        assert_eq!(state(&report, "cable"), "fail");
        assert_eq!(state(&report, "wake"), "skip");
        assert!(report.wakes.is_empty());

        let (report, _) = first_test(A2Fault::NoPower);
        assert_eq!(report.verdict.kind, "no-echo");
        assert_eq!(report.echo.unwrap().state, "none");
    }

    #[test]
    fn the_cable_test_has_its_own_verdicts() {
        let check = |fault: A2Fault| run(Job::KklCableCheck(fault.device())).0;
        let ready = check(A2Fault::NoPower);
        assert_eq!(ready.verdict.kind, "cable-ready", "{ready:#?}");
        assert_eq!(ready.cable.family, "ftdi");
        assert_eq!(ready.wakes.len(), 1);
        assert_eq!(ready.wakes[0].phases.len(), 4);

        assert_eq!(check(A2Fault::None).verdict.kind, "cable-on-car");
        assert_eq!(check(A2Fault::BrokenEcho).verdict.kind, "echo-broken");

        let (none, _) = run(Job::KklCableCheck(String::new()));
        assert_eq!(none.verdict.kind, "port-problem");
        assert!(!none.verdict.warnings.is_empty());
    }
}
