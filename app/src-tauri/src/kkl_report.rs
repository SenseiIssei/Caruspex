// The report of the KKL first test as two files for the maintainers: the
// JSON the worker sent and a plain text version, with an English frame like
// the issue text of "Share recording". They go into test-reports in the
// app's config folder, nothing is sent anywhere. The VIN and the immobiliser
// id that some instrument clusters put into their identification are masked
// unless the user ticked to include them.
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::worker::kkl_test::{KklTestReport, KklWakeRow};

/// The folder below the config folder.
pub const REPORT_FOLDER: &str = "test-reports";
/// Characters kept of a VIN: maker, model and model year, as the test
/// report form asks.
const VIN_KEEP: usize = 11;
/// Characters kept of an immobiliser id (VWZ7Z0..., AUZ7Z0...).
const IMMO_KEEP: usize = 7;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SavedReport {
    pub folder: String,
    pub json: String,
    pub text: String,
}

fn vin_char(c: char) -> bool {
    (c.is_ascii_uppercase() && !matches!(c, 'I' | 'O' | 'Q')) || c.is_ascii_digit()
}

fn masks(run: &str) -> Option<usize> {
    let digits = run.chars().filter(char::is_ascii_digit).count();
    let letters = run.chars().filter(char::is_ascii_uppercase).count();
    if run.len() == 17 && run.chars().all(vin_char) && digits > 0 && letters > 0 {
        return Some(VIN_KEEP);
    }
    // VAG immobiliser ids: 14 characters, the maker's letters, Z, then
    // mostly digits.
    let b = run.as_bytes();
    if run.len() == 14
        && b[0].is_ascii_uppercase()
        && b[1].is_ascii_uppercase()
        && b[2] == b'Z'
        && digits >= 6
    {
        return Some(IMMO_KEEP);
    }
    None
}

/// The text with every VIN and immobiliser id shortened.
pub fn mask_private(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        match masks(run) {
            Some(keep) => {
                out.push_str(&run[..keep]);
                out.push_str(&"*".repeat(run.len() - keep));
            }
            None => out.push_str(run),
        }
        run.clear();
    };
    for c in text.chars() {
        if c.is_ascii_uppercase() || c.is_ascii_digit() {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

fn mask_value(value: &mut Value) {
    match value {
        Value::String(s) => *s = mask_private(s),
        Value::Array(items) => items.iter_mut().for_each(mask_value),
        Value::Object(map) => map.values_mut().for_each(mask_value),
        _ => {}
    }
}

fn opt(value: Option<f64>) -> String {
    value
        .map(|v| format!("{v} ms"))
        .unwrap_or_else(|| "-".into())
}

fn wake_text(w: &KklWakeRow, out: &mut Vec<String>) {
    out.push(format!(
        "Unit {:02X} ({}), attempt {}, {} timing, {} baud: {}",
        w.address, w.unit, w.attempt, w.strategy, w.baud, w.verdict
    ));
    let phases: Vec<String> = w
        .phases
        .iter()
        .map(|p| {
            format!(
                "{} for {} bit(s): {} ms of {} ms (call {} ms)",
                if p.one { "high" } else { "low" },
                p.bits,
                p.measured_ms,
                p.target_ms,
                p.call_ms
            )
        })
        .collect();
    for p in phases {
        out.push(format!("    {p}"));
    }
    out.push(format!(
        "    largest edge error {} ms, address {} ms, break call on {} ms / off {} ms",
        w.worst_error_ms, w.address_ms, w.on_call_ms, w.off_call_ms
    ));
    out.push(format!(
        "    address echo bytes {}, first byte {} after {}, key bytes {}",
        w.address_echo,
        if w.first_byte.is_empty() {
            "-"
        } else {
            &w.first_byte
        },
        opt(w.first_byte_after_ms),
        if w.key_bytes.is_empty() {
            "-"
        } else {
            &w.key_bytes
        }
    ));
    if w.reply_after_ms.is_some() {
        out.push(format!(
            "    reply after {}, echo {} after {}",
            opt(w.reply_after_ms),
            match w.reply_echo {
                Some(true) => "matched",
                Some(false) => "differed",
                None => "missing",
            },
            opt(w.reply_round_trip_ms)
        ));
    }
    if !w.error.is_empty() {
        out.push(format!("    error: {}", w.error.replace('\n', " ")));
    }
}

/// The plain text version.
pub fn render_text(report: &KklTestReport, context: &[String]) -> String {
    let mut out: Vec<String> = Vec::new();
    let title = if report.kind == "cable" {
        "Caruspex KKL cable test (cable on USB only)"
    } else {
        "Caruspex KKL first test on the car (read only)"
    };
    out.push(title.into());
    out.push("=".repeat(title.len()));
    out.push(format!(
        "Caruspex {} on {}, {}",
        report.version, report.platform, report.date
    ));
    let c = &report.cable;
    out.push(format!(
        "Port {}: {}, USB {:04X}:{:04X}, chip {} ({}), latency timer {}{}",
        c.device,
        c.description,
        c.vid,
        c.pid,
        if c.chip.is_empty() { "?" } else { &c.chip },
        c.family,
        c.latency_ms
            .map(|v| format!("{v} ms"))
            .unwrap_or_else(|| "unknown".into()),
        if c.simulated { ", SIMULATED" } else { "" }
    ));
    if let Some(echo) = &report.echo {
        out.push(format!(
            "Echo: sent {}, back {} ({}), round trip {}",
            echo.sent,
            if echo.received.is_empty() {
                "-"
            } else {
                &echo.received
            },
            echo.state,
            opt(echo.round_trip_ms)
        ));
    }
    for line in context {
        out.push(line.clone());
    }
    if report.cancelled {
        out.push("The run was cancelled.".into());
    }
    out.push(String::new());
    out.push(format!(
        "Result: {} ({})",
        report.verdict.headline, report.verdict.kind
    ));
    if !report.verdict.advice.is_empty() {
        out.push(format!("  {}", report.verdict.advice));
    }
    for w in &report.verdict.warnings {
        out.push(format!("  ! {w}"));
    }

    out.push(String::new());
    out.push("Steps".into());
    out.push("-----".into());
    for s in &report.steps {
        out.push(format!(
            "[{:<4}] {}: {}",
            s.state.to_uppercase(),
            s.title,
            s.summary
        ));
        for line in &s.lines {
            out.push(format!("       {line}"));
        }
    }

    if !report.wakes.is_empty() {
        out.push(String::new());
        out.push("Wake-up timing".into());
        out.push("--------------".into());
        for w in &report.wakes {
            wake_text(w, &mut out);
        }
    }

    let logged: Vec<_> = report.steps.iter().filter(|s| !s.log.is_empty()).collect();
    if !logged.is_empty() {
        out.push(String::new());
        out.push("Raw log".into());
        out.push("-------".into());
        for s in logged {
            out.push(format!("[{}]", s.title));
            for line in &s.log {
                out.push(format!("  {line}"));
            }
        }
    }
    out.push(String::new());
    out.join("\n")
}

/// The next free pair of names: kkl-car-2026-11-20-1.json and .txt.
fn free_name(folder: &Path, kind: &str, date: &str) -> PathBuf {
    let mut n = 1;
    loop {
        let base = folder.join(format!("kkl-{kind}-{date}-{n}"));
        if !base.with_extension("json").exists() && !base.with_extension("txt").exists() {
            return base;
        }
        n += 1;
    }
}

/// Writes both files and says where they are.
pub fn save(
    config_dir: &Path,
    mut report: Value,
    context: &[String],
    include_vin: bool,
) -> Result<SavedReport, String> {
    let mut context = context.to_vec();
    if !include_vin {
        mask_value(&mut report);
        context = context.iter().map(|c| mask_private(c)).collect();
    }
    let parsed: KklTestReport =
        serde_json::from_value(report.clone()).map_err(|e| e.to_string())?;
    if let Value::Object(map) = &mut report {
        map.insert("context".into(), Value::from(context.clone()));
    }
    let folder = config_dir.join(REPORT_FOLDER);
    std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let kind = if parsed.kind == "cable" {
        "cable"
    } else {
        "car"
    };
    let date = if parsed.date.is_empty() {
        "report".to_string()
    } else {
        parsed
            .date
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '-')
            .collect()
    };
    let base = free_name(&folder, kind, &date);
    let json_path = base.with_extension("json");
    let text_path = base.with_extension("txt");
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    std::fs::write(&json_path, json).map_err(|e| format!("{}: {e}", json_path.display()))?;
    std::fs::write(&text_path, render_text(&parsed, &context))
        .map_err(|e| format!("{}: {e}", text_path.display()))?;
    Ok(SavedReport {
        folder: folder.display().to_string(),
        json: json_path.display().to_string(),
        text: text_path.display().to_string(),
    })
}

/// Only files in the report folder may be shown in the file manager.
pub fn inside_report_folder(config_dir: &Path, path: &Path) -> bool {
    let folder = config_dir.join(REPORT_FOLDER);
    match (folder.canonicalize(), path.canonicalize()) {
        (Ok(folder), Ok(path)) => path.starts_with(folder),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vin_and_immobiliser_id_are_masked() {
        let text = "WAUZZZ8Z01N000417 | AUZ7Z0A0000417 | 8Z0920900F  KOMBI";
        let masked = mask_private(text);
        assert_eq!(
            masked,
            "WAUZZZ8Z01N****** | AUZ7Z0A******* | 8Z0920900F  KOMBI"
        );
        // Part numbers and ordinary words stay.
        assert_eq!(
            mask_private("036906034AM MARELLI 4LV"),
            "036906034AM MARELLI 4LV"
        );
    }

    #[test]
    fn the_report_is_written_twice_and_masked() {
        let dir = std::env::temp_dir().join(format!("caruspex-kkl-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let report = KklTestReport {
            kind: "car".into(),
            version: "0.0.0".into(),
            platform: "test".into(),
            date: "2026-11-20".into(),
            device: "sim:a2".into(),
            steps: vec![crate::worker::kkl_test::KklTestStep {
                id: "unit-17".into(),
                title: "17".into(),
                state: "ok".into(),
                lines: vec!["WAUZZZ8Z01N000417".into()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let value = serde_json::to_value(&report).unwrap();
        let saved = save(&dir, value.clone(), &["Car: Audi A2".into()], false).unwrap();
        let text = std::fs::read_to_string(&saved.text).unwrap();
        assert!(text.contains("WAUZZZ8Z01N******"), "{text}");
        assert!(text.contains("Car: Audi A2"));
        let json = std::fs::read_to_string(&saved.json).unwrap();
        assert!(!json.contains("000417"));
        assert!(saved.json.ends_with("kkl-car-2026-11-20-1.json"));
        // A second save does not overwrite the first; with the box ticked
        // the VIN stays.
        let again = save(&dir, value, &[], true).unwrap();
        assert!(again.json.ends_with("kkl-car-2026-11-20-2.json"));
        assert!(
            std::fs::read_to_string(&again.text)
                .unwrap()
                .contains("WAUZZZ8Z01N000417")
        );
        assert!(inside_report_folder(&dir, Path::new(&again.text)));
        assert!(!inside_report_folder(&dir, &dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
