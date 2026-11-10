//! Adapter test: what the connected ELM327 really is and what it really can do.
//!
//! Nearly every cheap adapter says "ELM327" when asked, and many claim a
//! version that their firmware does not implement. The probe therefore does
//! not trust the version string. It asks a set of commands whose existence
//! depends on the firmware version and checks whether each one is answered
//! the way the original chip answers it. From that it derives the command set
//! the adapter behaves like and compares it with what it claims.
//!
//! Everything sent is either read-only or sets a value that `Elm327::connect`
//! has already set to the same value. Nothing writes to the adapter's
//! non-volatile memory (no ATPP ... SV/ON/OFF, no AT@3, no STSBR).
//!
//! The verdict is worded carefully: from the outside a good clone can look
//! exactly like the original, and the probe says so instead of pretending to
//! know.

use std::time::Instant;

use caruspex_i18n::{Language, Text, language, translate_core_owned};

use crate::elm327::{Elm327, ElmResult, ElmStatus, HeaderFormat, decode_messages};

/// A text built at runtime in both languages. `get()` picks the active one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BilingualText {
    pub de: String,
    pub en: String,
}

impl BilingualText {
    pub fn new(de: impl Into<String>, en: impl Into<String>) -> Self {
        Self {
            de: de.into(),
            en: en.into(),
        }
    }

    /// German or English; any other language looks the English text up in
    /// the catalog and falls back to it.
    pub fn get(&self) -> String {
        match language() {
            Language::German => self.de.clone(),
            Language::English => self.en.clone(),
            other => translate_core_owned(other, &self.en),
        }
    }
}

/// One version-dependent command and how the adapter answered it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandCheck {
    /// As sent, e.g. "ATCSM1".
    pub command: String,
    /// ELM327 version level, see [`version_text`].
    pub introduced_in: i32,
    /// Answered like the original chip.
    pub supported: bool,
    /// First response line, as received.
    pub response: String,
    /// What the command is for.
    pub purpose: BilingualText,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdapterReport {
    /// Answered ATI at all.
    pub reachable: bool,

    /// ATI, e.g. "ELM327 v1.5".
    pub version_text: String,
    /// AT@1
    pub description: String,
    /// Parsed from ATI, 0 if not parseable.
    pub claimed_level: i32,
    /// False for v1.5 and other versions never released.
    pub claimed_version_exists: bool,

    /// STN chip (OBDLink and others).
    pub is_stn: bool,
    /// STI
    pub stn_id: String,
    /// STDI
    pub stn_device: String,

    pub has_voltage: bool,
    /// ATRV, volts.
    pub voltage: f64,

    /// Round trip of "0100", averaged over a few tries, in milliseconds.
    /// Only measured when Elm327 has a connection to a control unit.
    pub round_trip_samples: i32,
    pub round_trip_ms: f64,
    /// The same with the expected number of answers appended ("0100 1").
    pub response_count_hint: bool,
    pub round_trip_with_hint_ms: f64,

    /// Answered "OK" to a command that does not exist.
    pub accepts_unknown_commands: bool,
    pub checks: Vec<CommandCheck>,
    /// Highest command set fully answered.
    pub behaves_like_level: i32,

    /// What was observed, plain sentences.
    pub facts: Vec<BilingualText>,
    /// Settings this adapter allows for more speed.
    pub speed_tips: Vec<BilingualText>,
    /// Hedged conclusion.
    pub verdict: BilingualText,
}

impl Default for AdapterReport {
    fn default() -> Self {
        Self {
            reachable: false,
            version_text: String::new(),
            description: String::new(),
            claimed_level: 0,
            claimed_version_exists: true,
            is_stn: false,
            stn_id: String::new(),
            stn_device: String::new(),
            has_voltage: false,
            voltage: 0.0,
            round_trip_samples: 0,
            round_trip_ms: 0.0,
            response_count_hint: false,
            round_trip_with_hint_ms: 0.0,
            accepts_unknown_commands: false,
            checks: Vec::new(),
            behaves_like_level: 0,
            facts: Vec::new(),
            speed_tips: Vec::new(),
            verdict: BilingualText::default(),
        }
    }
}

/// What a correct answer to a check looks like.
#[derive(Clone, Copy)]
enum Expect {
    /// "OK"
    Ok,
    /// A table of "nn:vv" entries (ATPPS).
    Table,
    /// "ON" or "OFF" (ATIGN).
    OnOff,
    /// One hex byte (ATAMC).
    HexByte,
}

struct CheckSpec {
    command: &'static str,
    level: i32,
    expect: Expect,
    purpose: Text,
}

// Version-dependent commands, oldest first. Every one is read-only or sets
// the value that Elm327::connect (or the power-on default) has already set,
// so running them changes nothing. Versions follow the ELM327 data sheet.
const CHECKS: [CheckSpec; 10] = [
    CheckSpec {
        command: "ATPPS",
        level: 110,
        expect: Expect::Table,
        purpose: Text::new(
            "Übersicht der programmierbaren Parameter (nur lesen)",
            "Summary of the programmable parameters (read only)",
        ),
    },
    CheckSpec {
        command: "ATKW1",
        level: 120,
        expect: Expect::Ok,
        purpose: Text::new(
            "Schlüsselbytes beim K-Line-Aufbau prüfen (Standardwert)",
            "Check key words on K-Line init (default value)",
        ),
    },
    CheckSpec {
        command: "ATS0",
        level: 130,
        expect: Expect::Ok,
        purpose: Text::new(
            "Leerzeichen in Antworten weglassen (schon eingestellt)",
            "Leave out spaces in responses (already set)",
        ),
    },
    CheckSpec {
        command: "ATD0",
        level: 130,
        expect: Expect::Ok,
        purpose: Text::new(
            "CAN-Datenlänge nicht anzeigen (Standardwert)",
            "Do not show the CAN data length (default)",
        ),
    },
    CheckSpec {
        command: "ATFE",
        level: 131,
        expect: Expect::Ok,
        purpose: Text::new("Gemerkte Ereignisse vergessen", "Forget events"),
    },
    CheckSpec {
        command: "ATCSM1",
        level: 140,
        expect: Expect::Ok,
        purpose: Text::new(
            "Stilles Mithören auf dem CAN-Bus (Standardwert)",
            "Silent CAN monitoring (default)",
        ),
    },
    CheckSpec {
        command: "ATIGN",
        level: 140,
        expect: Expect::OnOff,
        purpose: Text::new("Zündungseingang lesen", "Read the ignition input"),
    },
    CheckSpec {
        command: "ATCRA",
        level: 142,
        expect: Expect::Ok,
        purpose: Text::new(
            "CAN-Empfangsfilter zurücksetzen (Standardwert)",
            "Reset the CAN receive filter (default)",
        ),
    },
    CheckSpec {
        command: "ATJHF1",
        level: 142,
        expect: Expect::Ok,
        purpose: Text::new(
            "J1939-Kopfzeilen formatieren (Standardwert)",
            "Format J1939 headers (default)",
        ),
    },
    CheckSpec {
        command: "ATAMC",
        level: 200,
        expect: Expect::HexByte,
        purpose: Text::new(
            "Zähler der Busaktivität anzeigen",
            "Show the activity monitor count",
        ),
    },
];

/// The highest level the checks above can tell apart.
const HIGHEST_TESTED: i32 = 200;

/// Versions ELM Electronics actually released, as far as Caruspex knows.
const RELEASED: [i32; 12] = [100, 110, 120, 121, 130, 131, 140, 142, 200, 210, 220, 230];

fn first_line(result: &ElmResult) -> String {
    result.lines.first().cloned().unwrap_or_default()
}

fn upper(s: &str) -> String {
    s.to_ascii_uppercase()
}

fn answered_as_expected(expect: Expect, line: &str) -> bool {
    let up = upper(line);
    match expect {
        Expect::Ok => up == "OK",
        Expect::Table => up.contains(':'),
        Expect::OnOff => up == "ON" || up == "OFF",
        Expect::HexByte => {
            !up.is_empty() && up.len() <= 2 && up.chars().all(|c| c.is_ascii_hexdigit())
        }
    }
}

/// "14.2" for English, "14,2" for German.
fn number(value: f64, decimals: usize) -> BilingualText {
    let en = format!("{value:.decimals$}");
    let de = en.replace('.', ",");
    BilingualText { de, en }
}

/// The leading number of a text like `strtod` reads it ("14.1V" -> 14.1).
fn leading_number(text: &str) -> f64 {
    let t = text.trim_start();
    let mut end = 0;
    let mut seen_dot = false;
    for (i, c) in t.char_indices() {
        let sign = i == 0 && (c == '+' || c == '-');
        if c.is_ascii_digit() || sign || (c == '.' && !seen_dot) {
            seen_dot |= c == '.';
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    t[..end].parse().unwrap_or(0.0)
}

/// Average round trip of a request, counting only answers that carry data.
fn measure_round_trip(elm: &mut Elm327, command: &str, tries: i32) -> (f64, i32) {
    let mut total = 0.0;
    let mut good = 0;
    for _ in 0..tries {
        let start = Instant::now();
        let raw = elm.send(command, 3000);
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        if raw.status == ElmStatus::AdapterError {
            continue;
        }
        if !decode_messages(&raw.lines, 0x01, HeaderFormat::None).ok() {
            continue;
        }
        total += elapsed;
        good += 1;
    }
    (
        if good > 0 {
            total / f64::from(good)
        } else {
            0.0
        },
        good,
    )
}

/// "v1.4b" for 142, "v2.1" for 210. Levels are major * 100 + minor * 10 +
/// letter (a = 1, b = 2).
pub fn version_text(level: i32) -> String {
    if level <= 0 {
        return "?".to_string();
    }
    let major = level / 100;
    let minor = (level / 10) % 10;
    let letter = level % 10;
    let mut text = format!("v{major}.{minor}");
    if letter > 0 {
        text.push((b'a' + letter as u8 - 1) as char);
    }
    text
}

/// Parses "ELM327 v1.4b" and similar into a level; 0 if no version is found.
pub fn parse_version_level(text: &str) -> i32 {
    let t = text.as_bytes();
    let mut i = 0;
    while i + 2 < t.len() {
        let at_start = i == 0 || t[i - 1] == b'v' || t[i - 1] == b'V' || t[i - 1] == b' ';
        if !at_start || !t[i].is_ascii_digit() || t[i + 1] != b'.' || !t[i + 2].is_ascii_digit() {
            i += 1;
            continue;
        }
        let mut level = i32::from(t[i] - b'0') * 100 + i32::from(t[i + 2] - b'0') * 10;
        if i + 3 < t.len() {
            let letter = t[i + 3].to_ascii_lowercase();
            if (b'a'..=b'i').contains(&letter) {
                level += i32::from(letter - b'a') + 1;
            }
        }
        return level;
    }
    0
}

/// Runs the test over an open Elm327. Best called right after connect, before
/// header or filter settings are changed: the probe resets the CAN receive
/// filter (ATCRA) as part of the test.
pub fn probe_adapter(elm: &mut Elm327) -> AdapterReport {
    let mut report = AdapterReport::default();
    fn fact(report: &mut AdapterReport, de: String, en: String) {
        report.facts.push(BilingualText { de, en });
    }

    // --- Identity -----------------------------------------------------------
    let id = elm.send("ATI", 2000);
    if id.status == ElmStatus::AdapterError || id.lines.is_empty() {
        report.verdict = BilingualText::new(
            "Der Adapter antwortet nicht. Ein Test ist so nicht möglich.",
            "The adapter does not answer, so it cannot be tested.",
        );
        return report;
    }
    report.reachable = true;
    report.version_text = first_line(&id);
    report.claimed_level = parse_version_level(&report.version_text);
    fact(
        &mut report,
        format!("ATI meldet: {}", id.lines[0]),
        format!("ATI reports: {}", id.lines[0]),
    );

    let description = first_line(&elm.send("AT@1", 1500));
    if !description.is_empty() && description != "?" {
        fact(
            &mut report,
            format!("AT@1 meldet: {description}"),
            format!("AT@1 reports: {description}"),
        );
        report.description = description;
    }

    if report.claimed_level > 0 {
        report.claimed_version_exists = RELEASED.contains(&report.claimed_level);
        let v = version_text(report.claimed_level);
        if report.claimed_level == 150 {
            fact(
                &mut report,
                "Die Version v1.5 hat ELM Electronics nie veröffentlicht. Sie steht fast nur auf Nachbauten."
                    .into(),
                "ELM Electronics never released a v1.5. It appears almost only on clones.".into(),
            );
        } else if !report.claimed_version_exists {
            fact(
                &mut report,
                format!(
                    "Die gemeldete Version {v} ist unter den Caruspex bekannten Versionen des Originals nicht dabei."
                ),
                format!(
                    "The reported version {v} is not among the original versions known to Caruspex."
                ),
            );
        }
    }

    // --- A command that does not exist ---------------------------------------
    // The original chip answers "?". Some clones say "OK" to anything, and
    // then every "OK" below proves nothing.
    let bogus = upper(&first_line(&elm.send("ATQX", 1500)));
    report.accepts_unknown_commands = bogus == "OK";
    if report.accepts_unknown_commands {
        fact(
            &mut report,
            "Auf den erfundenen Befehl ATQX antwortet der Adapter mit OK statt mit ?.".into(),
            "The adapter answers the made-up command ATQX with OK instead of ?.".into(),
        );
    }

    // --- STN chip? -----------------------------------------------------------
    let sti = first_line(&elm.send("STI", 1500));
    if upper(&sti).starts_with("STN") {
        report.is_stn = true;
        fact(
            &mut report,
            format!("STI meldet: {sti}"),
            format!("STI reports: {sti}"),
        );
        report.stn_id = sti;
        let device = first_line(&elm.send("STDI", 1500));
        if !device.is_empty() && device != "?" {
            fact(
                &mut report,
                format!("STDI meldet: {device}"),
                format!("STDI reports: {device}"),
            );
            report.stn_device = device;
        }
    }

    // --- Voltage -------------------------------------------------------------
    let rv = first_line(&elm.send("ATRV", 1500));
    if rv.as_bytes().first().is_some_and(|c| c.is_ascii_digit()) {
        report.voltage = leading_number(&rv);
        report.has_voltage = true;
        let volts = number(report.voltage, 1);
        fact(
            &mut report,
            format!("Spannung an der OBD-Buchse: {} V", volts.de),
            format!("Voltage at the OBD socket: {} V", volts.en),
        );
    }

    // --- Version-dependent commands ------------------------------------------
    for spec in &CHECKS {
        let result = elm.send(spec.command, 1500);
        let response = first_line(&result);
        report.checks.push(CommandCheck {
            command: spec.command.to_string(),
            introduced_in: spec.level,
            supported: result.status != ElmStatus::AdapterError
                && answered_as_expected(spec.expect, &response),
            response,
            purpose: BilingualText::new(spec.purpose.de, spec.purpose.en),
        });
    }

    // Highest level up to which every check passed. Version 1.0 commands are
    // what Elm327::connect itself relies on, so they count as present.
    report.behaves_like_level = 100;
    let first_missing = report
        .checks
        .iter()
        .find(|c| !c.supported)
        .map(|c| c.introduced_in)
        .unwrap_or(0);
    for check in &report.checks {
        if first_missing != 0 && check.introduced_in >= first_missing {
            break;
        }
        report.behaves_like_level = report.behaves_like_level.max(check.introduced_in);
    }

    let missing: Vec<String> = report
        .checks
        .iter()
        .filter(|c| !c.supported)
        .map(|c| format!("{} ({})", c.command, version_text(c.introduced_in)))
        .collect();
    let missing = missing.join(", ");
    let behaves = version_text(report.behaves_like_level);
    if missing.is_empty() {
        fact(
            &mut report,
            "Alle geprüften Befehle bis v2.0 werden wie beim Original beantwortet.".into(),
            "Every tested command up to v2.0 is answered like the original.".into(),
        );
    } else {
        fact(
            &mut report,
            format!("Befehlssatz wie {behaves}. Nicht wie beim Original beantwortet: {missing}."),
            format!("Command set like {behaves}. Not answered like the original: {missing}."),
        );
    }

    // --- Round trip ----------------------------------------------------------
    if elm.connected() {
        let (ms, good) = measure_round_trip(elm, "0100", 5);
        report.round_trip_ms = ms;
        report.round_trip_samples = good;
        if good > 0 {
            let ms = number(report.round_trip_ms, 0);
            fact(
                &mut report,
                format!(
                    "Antwortzeit auf 0100: im Schnitt {} ms ({} Versuche)",
                    ms.de, good
                ),
                format!(
                    "Round trip of 0100: {} ms on average ({} tries)",
                    ms.en, good
                ),
            );
            // The number of expected answers after the request (v1.3 and
            // later) lets the adapter stop waiting after the first answer.
            let (hinted_ms, hinted) = measure_round_trip(elm, "01001", 3);
            report.round_trip_with_hint_ms = hinted_ms;
            report.response_count_hint = hinted > 0;
            if report.response_count_hint {
                let hms = number(report.round_trip_with_hint_ms, 0);
                fact(
                    &mut report,
                    format!("Mit Antwortzahl (0100 1): im Schnitt {} ms", hms.de),
                    format!("With an answer count (0100 1): {} ms on average", hms.en),
                );
            }
        }
    }

    // --- Settings for more speed ---------------------------------------------
    if report.response_count_hint && report.round_trip_with_hint_ms + 2.0 < report.round_trip_ms {
        let a = number(report.round_trip_with_hint_ms, 0);
        let b = number(report.round_trip_ms, 0);
        report.speed_tips.push(BilingualText::new(
            format!(
                "Die Zahl der erwarteten Antworten an die Anfrage hängen (etwa 0100 1). Dann wartet der Adapter nicht auf weitere Steuergeräte: hier {} statt {} ms.",
                a.de, b.de
            ),
            format!(
                "Append the number of expected answers to a request (such as 0100 1). The adapter then stops waiting for further control units: {} instead of {} ms here.",
                a.en, b.en
            ),
        ));
    }
    if report.behaves_like_level >= 120 && !report.accepts_unknown_commands {
        report.speed_tips.push(BilingualText::new(
            "Die schärfere adaptive Zeitsteuerung ATAT2 ist vorhanden und verkürzt die Wartezeit weiter. Bei fehlenden Antworten zurück auf ATAT1.",
            "The more aggressive adaptive timing ATAT2 is available and shortens the waiting further. Go back to ATAT1 if answers go missing.",
        ));
    }
    if report.is_stn {
        report.speed_tips.push(BilingualText::new(
            "STN-Chip: Mit STBR lässt sich die serielle Geschwindigkeit für diese Sitzung erhöhen, bei USB- und seriellen Verbindungen bis in den Megabit-Bereich. Bluetooth und WLAN bestimmen ihr Tempo selbst. Dauerhaft gespeichert wird dabei nichts (das täte erst STSBR). Caruspex stellt das nicht selbst um.",
            "STN chip: STBR raises the serial speed for this session, into the megabit range on USB and serial connections. Bluetooth and Wi-Fi set their own speed. Nothing is stored permanently (STSBR would do that). Caruspex does not change it itself.",
        ));
        report.speed_tips.push(BilingualText::new(
            "STN-Chips verarbeiten Anfragen deutlich schneller als ELM327-Nachbauten und puffern mehr. Kürzere Abfrageintervalle sind hier unbedenklich.",
            "STN chips process requests much faster than ELM327 clones and buffer more. Shorter polling intervals are fine here.",
        ));
    }

    // --- Verdict -------------------------------------------------------------
    let claimed = version_text(report.claimed_level);
    let comparable = report.claimed_level.min(HIGHEST_TESTED);
    let vt = report.version_text.clone();
    report.verdict = if report.accepts_unknown_commands {
        BilingualText::new(
            format!(
                "Der Adapter nennt sich {vt}, bestätigt aber auch Befehle, die es nicht gibt. Welche Funktionen er wirklich beherrscht, lässt sich deshalb nicht verlässlich prüfen. Das ist typisch für einfache Nachbauten."
            ),
            format!(
                "The adapter calls itself {vt} but also confirms commands that do not exist. Which functions it really has therefore cannot be tested reliably. This is typical of simple clones."
            ),
        )
    } else if report.is_stn {
        let name = if report.stn_device.is_empty() {
            report.stn_id.clone()
        } else {
            report.stn_device.clone()
        };
        BilingualText::new(
            format!(
                "STN-Chip ({name}): kein ELM327 und auch kein Nachbau, sondern ein eigener, kompatibler Chip von ScanTool.net. Er beantwortet die ELM327-Befehle wie {behaves}."
            ),
            format!(
                "STN chip ({name}): not an ELM327 and not a clone either, but an independent compatible chip by ScanTool.net. It answers the ELM327 commands like {behaves}."
            ),
        )
    } else if report.claimed_level == 0 {
        BilingualText::new(
            format!(
                "Der Adapter nennt keine erkennbare ELM327-Version. Er beantwortet die Befehle wie ein ELM327 {behaves}."
            ),
            format!(
                "The adapter reports no recognizable ELM327 version. It answers the commands like an ELM327 {behaves}."
            ),
        )
    } else if report.claimed_level == 150 {
        BilingualText::new(
            format!(
                "Wahrscheinlich ein Nachbau: Er meldet v1.5, eine Version, die es vom Original nie gab, und verhält sich wie ein ELM327 {behaves}."
            ),
            format!(
                "Probably a clone: it reports v1.5, a version the original never had, and behaves like an ELM327 {behaves}."
            ),
        )
    } else if report.behaves_like_level < comparable {
        let from = version_text(first_missing);
        BilingualText::new(
            format!(
                "Wahrscheinlich ein Nachbau, der sich als {claimed} ausgibt, sich aber wie {behaves} verhält: Befehle ab {from} fehlen oder werden anders beantwortet."
            ),
            format!(
                "Probably a clone that claims {claimed} but behaves like {behaves}: commands from {from} on are missing or answered differently."
            ),
        )
    } else if report.behaves_like_level > report.claimed_level {
        BilingualText::new(
            format!(
                "Der Adapter beherrscht mehr, als seine Angabe {claimed} erwarten lässt (wie {behaves}). Beim Original wäre das nicht so; vermutlich eine abweichende Firmware."
            ),
            format!(
                "The adapter can do more than its stated {claimed} suggests (like {behaves}). The original would not; most likely a different firmware."
            ),
        )
    } else {
        let beyond = report.claimed_level > HIGHEST_TESTED;
        BilingualText::new(
            format!(
                "Das Verhalten passt zu einem echten ELM327 {claimed}{}. Ein gut gemachter Nachbau lässt sich von außen allerdings nicht sicher davon unterscheiden.",
                if beyond {
                    " (geprüft wurden Befehle bis v2.0)"
                } else {
                    ""
                }
            ),
            format!(
                "The behaviour matches a genuine ELM327 {claimed}{}. A well-made clone cannot be told apart from the outside with certainty, though.",
                if beyond {
                    " (commands up to v2.0 were tested)"
                } else {
                    ""
                }
            ),
        )
    };
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elm327_version_strings() {
        assert_eq!(parse_version_level("ELM327 v1.5"), 150);
        assert_eq!(parse_version_level("ELM327 v1.4b"), 142);
        assert_eq!(parse_version_level("ELM327 v2.1"), 210);
        assert_eq!(parse_version_level("ELM327 1.3a"), 131);
        assert_eq!(parse_version_level("OBDII"), 0);
        assert_eq!(version_text(142), "v1.4b");
        assert_eq!(version_text(200), "v2.0");
        assert_eq!(version_text(0), "?");
    }

    #[test]
    fn numbers_in_both_languages() {
        assert_eq!(number(14.06, 1), BilingualText::new("14,1", "14.1"));
        assert_eq!(number(12.4, 0).en, "12");
        assert!((leading_number("14.1V") - 14.1).abs() < 1e-9);
        assert_eq!(leading_number("V"), 0.0);
    }

    #[test]
    fn answers_are_judged_like_the_original() {
        assert!(answered_as_expected(Expect::Ok, "ok"));
        assert!(answered_as_expected(Expect::Table, "00:FF F  01:FF F"));
        assert!(answered_as_expected(Expect::OnOff, "OFF"));
        assert!(answered_as_expected(Expect::HexByte, "0A"));
        assert!(!answered_as_expected(Expect::HexByte, "?"));
        assert!(!answered_as_expected(Expect::HexByte, "ABC"));
    }
}
